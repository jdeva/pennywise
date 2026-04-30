/**
 * Parsers for `ledger-cli` balance/register text output.
 *
 * These formats are whitespace-column-based and surprisingly stable across
 * versions — but still brittle enough that callers should gracefully fall
 * back to the raw text if the parse yields nothing useful.
 */

export interface BalanceRow {
  account: string
  amount: number
  currency: string
  depth: number
}

export interface RegisterPosting {
  account: string
  amount: number
  currency: string
}

export interface RegisterEntry {
  date: string
  payee: string
  postings: RegisterPosting[]
}

/**
 * Strips a leading currency symbol and returns { amount, currency }.
 * Examples: "$ -1,234.56" → { amount: -1234.56, currency: "$" }
 */
function parseMoney(raw: string): { amount: number; currency: string } | null {
  const trimmed = raw.trim()
  if (!trimmed) return null
  const match = trimmed.match(/^([^\d\-+.]+)?\s*(-?[\d,]+(?:\.\d+)?)$/)
  if (!match) return null
  const currency = (match[1] ?? '').trim()
  const amount = parseFloat(match[2].replace(/,/g, ''))
  if (!Number.isFinite(amount)) return null
  return { amount, currency }
}

/**
 * Parses `ledger balance` output. Account indentation in the original output
 * encodes hierarchy — we reflect that via `depth`.
 *
 * Skips the final `--------` separator line and the all-accounts total below it.
 */
export function parseBalance(raw: string): BalanceRow[] {
  const rows: BalanceRow[] = []
  const lines = raw.split('\n')

  // Depth is derived from indent of the account-name column, which ledger pads
  // with 2 extra spaces per tree level. We find the shallowest indent among
  // non-leaf lines and treat that as depth 0, then compute depth in units of 2 spaces.
  let baseIndent: number | null = null

  // First pass: collect parsed rows with raw account indent.
  const parsed: Array<{ amount: number; currency: string; name: string; indent: number }> = []
  for (const line of lines) {
    if (!line.trim()) continue
    if (line.includes('---')) break

    // Typical line: "            $1574.76  Assets"     (depth 0)
    //               "            $1612.76    Bank:Revolut"  (depth 1, extra indent)
    // Capture: amountCell (anything up to a 2+ space gap) + the account (with indent preserved).
    const match = line.match(/^(\s*[^\s].*?)(\s{2,})([^\s].*)$/)
    if (!match) continue
    const money = parseMoney(match[1])
    if (!money) continue
    const accountCell = match[3]
    // The gap between amount cell end and account start — but the amount cell
    // itself is right-aligned and may be padded with leading spaces. What we
    // actually need is the indent within the account column region.
    // The gap before the account (match[2]) is the delimiter; any additional
    // spaces beyond the minimum-2 indicate tree nesting.
    const gapWidth = match[2].length
    const indent = gapWidth
    const name = accountCell.trim()
    if (baseIndent === null || indent < baseIndent) baseIndent = indent
    parsed.push({ amount: money.amount, currency: money.currency, name, indent })
  }

  const base = baseIndent ?? 0
  for (const p of parsed) {
    const depth = Math.max(0, Math.round((p.indent - base) / 2))
    rows.push({ account: p.name, amount: p.amount, currency: p.currency, depth })
  }

  return rows
}

/**
 * Parses `ledger register` output into transaction entries.
 *
 * Output shape — columns are fixed-width whitespace-padded:
 *   col 0-8:   date (YY-MMM-DD)
 *   col 10-31: payee (padded with spaces to col 32)
 *   col 32-:   account, then amount, then running total
 *
 *   26-Apr-30 Mate1 posted this     Expenses:Food:Coffee          $8.00        $8.00
 *                                   Assets:Bank:Revolut          $-8.00            0
 *
 * Posting lines (continuation) have the date+payee columns blank.
 *
 * We parse right-to-left for amounts (last money is running total; second-last
 * is the posting amount) and split date/payee/account using fixed-column logic
 * rather than counting 2+ space gaps (which breaks on multi-word payees).
 */
const ACCOUNT_COL = 32 // empirical — ledger pads payee to col 32 before account
const DATE_LEN = 9 // "YY-MMM-DD"

export function parseRegister(raw: string): RegisterEntry[] {
  const entries: RegisterEntry[] = []
  let current: RegisterEntry | null = null

  for (const rawLine of raw.split('\n')) {
    if (!rawLine.trim()) continue

    // Identify the account column. On header lines, col ≥ ACCOUNT_COL; on
    // continuation lines (no date/payee), the account just starts after leading
    // spaces and we can locate it by scanning for the first non-space.
    const isHeader = /^\d{2,4}-[A-Za-z0-9]{2,3}-\d{1,2}/.test(rawLine)

    // Extract the account + amounts region (everything from ACCOUNT_COL onward).
    const tail = rawLine.length >= ACCOUNT_COL ? rawLine.slice(ACCOUNT_COL) : rawLine.trimStart()

    // Split the tail on 2+ spaces: [account, amount, runningTotal?]
    const cells = tail.split(/\s{2,}/).map((s) => s.trim()).filter(Boolean)
    if (cells.length < 2) continue

    // Find the two money cells from the right. Rightmost = running total, next = amount.
    let amountIdx = -1
    for (let i = cells.length - 1; i >= 0; i--) {
      if (parseMoney(cells[i])) {
        if (amountIdx === -1) {
          amountIdx = i
        } else {
          amountIdx = i
          break
        }
      }
    }
    if (amountIdx === -1) continue
    const amountMoney = parseMoney(cells[amountIdx])
    if (!amountMoney) continue
    const account = cells.slice(0, amountIdx).join(' ').trim()
    if (!account) continue

    if (isHeader) {
      if (current) entries.push(current)
      const date = rawLine.slice(0, DATE_LEN)
      // Payee is from col (DATE_LEN + 1) up to ACCOUNT_COL, trimmed.
      const payeeRaw = rawLine.slice(DATE_LEN + 1, ACCOUNT_COL)
      const payee = payeeRaw.trim()
      current = {
        date,
        payee,
        postings: [{ account, amount: amountMoney.amount, currency: amountMoney.currency }],
      }
    } else if (current) {
      current.postings.push({ account, amount: amountMoney.amount, currency: amountMoney.currency })
    }
  }
  if (current) entries.push(current)
  return entries
}

/** Normalise any NaiveDate-ish string into ISO YYYY-MM-DD when possible. */
export function normaliseDate(s: string): string {
  // Common ledger format: `YY-MMM-DD`
  const m = s.match(/^(\d{2,4})-([A-Za-z]{3})-(\d{1,2})$/)
  if (m) {
    const months = ['jan','feb','mar','apr','may','jun','jul','aug','sep','oct','nov','dec']
    const month = months.indexOf(m[2].toLowerCase())
    if (month === -1) return s
    const year = m[1].length === 2 ? `20${m[1]}` : m[1]
    return `${year}-${String(month + 1).padStart(2, '0')}-${m[3].padStart(2, '0')}`
  }
  return s
}

/** Pretty-print an amount with currency sign in natural position. */
export function formatAmount(amount: number, currency: string): string {
  const sign = amount < 0 ? '-' : ''
  const abs = Math.abs(amount).toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
  // Symbol-first (e.g. $) vs suffix (e.g. USD) heuristic
  if (currency.length <= 1) return `${sign}${currency}${abs}`
  return `${sign}${abs} ${currency}`
}
