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

  for (const line of lines) {
    if (!line.trim()) continue
    if (line.includes('---')) break

    // Account name starts where the 2+ space gap after the amount ends.
    // A typical line: "       $ -1,234.56  Assets:Checking"
    const match = line.match(/^(\s*[^\s].*?)\s{2,}([^\s].*)$/)
    if (!match) continue
    const money = parseMoney(match[1])
    if (!money) continue
    const accountRaw = match[2]
    const depth = accountRaw.match(/^\s*/)?.[0].length ?? 0
    rows.push({
      account: accountRaw.trim(),
      amount: money.amount,
      currency: money.currency,
      depth,
    })
  }

  return rows
}

/**
 * Parses `ledger register` output into transaction entries with their postings.
 *
 * Each entry starts with a line that has a date + payee; subsequent lines
 * lacking a date belong to that entry's postings.
 *
 *   26-Feb-01 Rent        Assets:Checking        $ -1000.00   $ -1000.00
 *                         Expenses:Rent           $ 1000.00            0
 */
export function parseRegister(raw: string): RegisterEntry[] {
  const entries: RegisterEntry[] = []
  const dateHead = /^(\d{2,4}[-\/][A-Za-z0-9]{2,3}[-\/]\d{1,2})\s+(.+)$/
  let current: RegisterEntry | null = null

  for (const line of raw.split('\n')) {
    if (!line.trim()) continue
    const headMatch = line.match(dateHead)
    const rest = headMatch ? headMatch[2] : line

    // Split the remainder by 2+ spaces; we expect [account, amount, running?]
    const cells = rest.split(/\s{2,}/).map((s) => s.trim()).filter(Boolean)
    if (cells.length < 2) continue

    const money = parseMoney(cells[cells.length - 2])
    if (!money) {
      // Trailing money might be missing — try last cell
      const fallback = parseMoney(cells[cells.length - 1])
      if (!fallback) continue
    }
    // Find the first cell that looks like money from the right
    let amountIdx = -1
    for (let i = cells.length - 1; i >= 0; i--) {
      if (parseMoney(cells[i])) {
        amountIdx = i
        break
      }
    }
    if (amountIdx === -1) continue
    // The amount is the *first* money from left, not the rightmost (that's running total).
    // We look left until we find a non-money cell — everything before is the account.
    let firstMoneyIdx = amountIdx
    for (let i = 0; i < amountIdx; i++) {
      if (parseMoney(cells[i])) {
        firstMoneyIdx = i
        break
      }
    }
    const account = cells.slice(0, firstMoneyIdx).join(' ')
    const amountMoney = parseMoney(cells[firstMoneyIdx])
    if (!account || !amountMoney) continue

    if (headMatch) {
      // Start of a new entry
      if (current) entries.push(current)
      current = {
        date: headMatch[1],
        payee: extractPayee(headMatch[2], account),
        postings: [{ account, amount: amountMoney.amount, currency: amountMoney.currency }],
      }
    } else if (current) {
      current.postings.push({ account, amount: amountMoney.amount, currency: amountMoney.currency })
    }
  }
  if (current) entries.push(current)
  return entries
}

/**
 * The first posting line includes the payee AND the account. Payee ends
 * where the account name starts; split on 2+ spaces and drop the trailing
 * account cell.
 */
function extractPayee(headRest: string, account: string): string {
  const idx = headRest.lastIndexOf(account)
  if (idx === -1) return headRest.split(/\s{2,}/)[0].trim()
  return headRest.slice(0, idx).trim()
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
