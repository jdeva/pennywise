//! Parser for the constrained ledger format we write ourselves.
//!
//! We only trust the format we produced via `TransactionService::format_transaction`:
//!
//! ```ledger
//! 2026-04-30 Payee
//!     ; Id: <uuid>
//!     Debit:Account  $12.34
//!         ; User: alice
//!     Credit:Account  -$12.34
//!         ; User: alice
//! ```
//!
//! Transactions without an `Id:` tag (legacy hand-written entries) are skipped
//! so their positions in the file are preserved verbatim. Callers that rewrite
//! the file use `find_entry_block` to locate a specific tx's byte range and
//! splice around it.

use uuid::Uuid;

/// A parsed transaction block. `start_byte` and `end_byte` mark the slice in
/// the original file contents that corresponds to this entry — from the start
/// of the `YYYY-MM-DD Payee` header line through the last posting/metadata
/// line (trailing newline excluded).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEntry {
    pub id: Option<Uuid>,
    pub date: String,
    pub payee: String,
    pub postings: Vec<ParsedPosting>,
    pub posted_by: Option<String>,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPosting {
    pub account: String,
    pub amount: String,
}

fn is_header_line(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 10
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[7] == b'-'
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit()
}

fn parse_tag<'a>(line: &'a str, tag: &str) -> Option<&'a str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(';') {
        return None;
    }
    let after_semi = trimmed[1..].trim_start();
    let prefix = format!("{}:", tag);
    after_semi.strip_prefix(&prefix).map(|v| v.trim())
}

/// Split a posting line like `"    Expenses:Food  $12.34"` into (account, amount).
/// Whitespace ≥ 2 spaces separates account from amount; comments start with `;`.
fn parse_posting_line(line: &str) -> Option<ParsedPosting> {
    let no_comment = match line.find(';') {
        Some(i) => &line[..i],
        None => line,
    };
    let trimmed = no_comment.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Find the split between account and amount — 2+ spaces OR tab.
    let bytes = trimmed.as_bytes();
    let mut split_at: Option<usize> = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\t' {
            split_at = Some(i);
            break;
        }
        if bytes[i] == b' ' && i + 1 < bytes.len() && bytes[i + 1] == b' ' {
            split_at = Some(i);
            break;
        }
        i += 1;
    }
    match split_at {
        Some(idx) => {
            let account = trimmed[..idx].trim().to_string();
            let amount = trimmed[idx..].trim().to_string();
            if account.is_empty() {
                None
            } else {
                Some(ParsedPosting { account, amount })
            }
        }
        None => {
            // No amount — rare but ledger allows it. Accept the whole line as account.
            Some(ParsedPosting {
                account: trimmed.to_string(),
                amount: String::new(),
            })
        }
    }
}

fn parse_header(line: &str) -> Option<(String, String)> {
    if !is_header_line(line) {
        return None;
    }
    let date = line[..10].to_string();
    let payee = line[10..].trim().to_string();
    Some((date, payee))
}

/// Parse all transaction entries in a ledger file's contents.
/// Comment lines (starting with `;`) and include directives are skipped
/// between entries. Blank lines end the current entry.
pub fn parse_entries(contents: &str) -> Vec<ParsedEntry> {
    let mut entries = Vec::new();
    let mut line_starts: Vec<usize> = vec![0];
    for (i, b) in contents.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }

    let lines: Vec<&str> = contents.split('\n').collect();
    let mut idx = 0;
    while idx < lines.len() {
        let line = lines[idx];
        if let Some((date, payee)) = parse_header(line) {
            let start_byte = line_starts[idx];
            let mut id: Option<Uuid> = None;
            let mut posted_by: Option<String> = None;
            let mut postings: Vec<ParsedPosting> = Vec::new();
            // End-of-entry is the last line before a blank line / next header /
            // EOF. Track the index of the last line that belongs to this entry.
            let mut last_line_idx = idx;
            let mut j = idx + 1;
            while j < lines.len() {
                let next = lines[j];
                if next.trim().is_empty() {
                    break;
                }
                if is_header_line(next) {
                    break;
                }
                // Tag on its own line (" ; Id: ...") or trailing metadata
                if let Some(v) = parse_tag(next, "Id") {
                    if id.is_none() {
                        id = Uuid::parse_str(v).ok();
                    }
                    last_line_idx = j;
                    j += 1;
                    continue;
                }
                if let Some(v) = parse_tag(next, "User") {
                    if posted_by.is_none() {
                        posted_by = Some(v.to_string());
                    }
                    last_line_idx = j;
                    j += 1;
                    continue;
                }
                // Pure comment line — skip but still part of entry block
                if next.trim_start().starts_with(';') {
                    last_line_idx = j;
                    j += 1;
                    continue;
                }
                if let Some(p) = parse_posting_line(next) {
                    postings.push(p);
                    last_line_idx = j;
                }
                j += 1;
            }
            // end_byte = start of (last_line_idx + 1) line, minus the newline
            let end_byte = if last_line_idx + 1 < line_starts.len() {
                line_starts[last_line_idx + 1].saturating_sub(1)
            } else {
                contents.len()
            };
            entries.push(ParsedEntry {
                id,
                date,
                payee,
                postings,
                posted_by,
                start_byte,
                end_byte,
            });
            idx = j;
        } else {
            idx += 1;
        }
    }
    entries
}

/// Find the byte range of the entry with the given id in `contents`.
/// Returns (start, end) — end is exclusive. The returned range excludes the
/// trailing newline so the rewriter can decide how to stitch surrounding lines.
pub fn find_entry_block(contents: &str, id: &Uuid) -> Option<(usize, usize)> {
    parse_entries(contents)
        .into_iter()
        .find(|e| e.id == Some(*id))
        .map(|e| (e.start_byte, e.end_byte))
}

/// Remove the entry block with `id` and return the new file contents.
/// Also trims the preceding blank-line separator if one exists, so we don't
/// leave orphan blank lines between entries.
pub fn remove_entry(contents: &str, id: &Uuid) -> Option<String> {
    let (start, end) = find_entry_block(contents, id)?;
    // Extend end forward to include the trailing newline if present.
    let mut end_inclusive = end;
    if end_inclusive < contents.len() && contents.as_bytes()[end_inclusive] == b'\n' {
        end_inclusive += 1;
    }
    // Trim one preceding blank-line separator if the line before start is blank.
    let mut real_start = start;
    if real_start >= 1 && contents.as_bytes()[real_start - 1] == b'\n' {
        // Look back for an additional blank line (i.e. "\n\n" before start).
        if real_start >= 2 && contents.as_bytes()[real_start - 2] == b'\n' {
            real_start -= 1;
        }
    }
    let mut out = String::with_capacity(contents.len());
    out.push_str(&contents[..real_start]);
    out.push_str(&contents[end_inclusive..]);
    Some(out)
}

/// Replace the entry block with `id` with `replacement` (which should be
/// the freshly-formatted transaction text, no trailing newline).
pub fn replace_entry(contents: &str, id: &Uuid, replacement: &str) -> Option<String> {
    let (start, end) = find_entry_block(contents, id)?;
    let mut out = String::with_capacity(contents.len() + replacement.len());
    out.push_str(&contents[..start]);
    out.push_str(replacement);
    out.push_str(&contents[end..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "; Period: 2026-Q2\n; Workspace ID: abc\n\n2026-04-01 Coffee\n    ; Id: 11111111-1111-1111-1111-111111111111\n    Expenses:Food:Coffee  $4.50\n    ; User: alice\n    Assets:Bank:Revolut  -$4.50\n    ; User: alice\n\n2026-04-15 Rent\n    ; Id: 22222222-2222-2222-2222-222222222222\n    Expenses:Rent  $1400.00\n    ; User: bob\n    Assets:Bank:Revolut  -$1400.00\n    ; User: bob\n";

    #[test]
    fn parses_two_entries_with_ids() {
        let entries = parse_entries(SAMPLE);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].date, "2026-04-01");
        assert_eq!(entries[0].payee, "Coffee");
        assert_eq!(entries[0].posted_by.as_deref(), Some("alice"));
        assert_eq!(entries[0].postings.len(), 2);
        assert_eq!(entries[0].postings[0].account, "Expenses:Food:Coffee");
        assert_eq!(entries[0].postings[0].amount, "$4.50");
        assert_eq!(entries[0].postings[1].account, "Assets:Bank:Revolut");
        assert_eq!(entries[0].postings[1].amount, "-$4.50");
        assert!(entries[0].id.is_some());
        assert!(entries[1].id.is_some());
        assert_ne!(entries[0].id, entries[1].id);
    }

    #[test]
    fn find_entry_block_returns_range_covering_entry() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let (start, end) = find_entry_block(SAMPLE, &id).unwrap();
        let slice = &SAMPLE[start..end];
        assert!(slice.starts_with("2026-04-01 Coffee"));
        assert!(slice.contains("Id: 11111111-1111-1111-1111-111111111111"));
        assert!(slice.contains("Assets:Bank:Revolut  -$4.50"));
        assert!(slice.contains("; User: alice"));
        assert!(!slice.contains("2026-04-15 Rent"));
    }

    #[test]
    fn remove_entry_drops_the_block_and_blank_separator() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let result = remove_entry(SAMPLE, &id).unwrap();
        assert!(!result.contains("11111111-1111-1111-1111-111111111111"));
        assert!(!result.contains("2026-04-01 Coffee"));
        assert!(result.contains("2026-04-15 Rent"));
        // No run of 3+ consecutive newlines left behind.
        assert!(!result.contains("\n\n\n"));
    }

    #[test]
    fn remove_entry_for_last_entry() {
        let id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
        let result = remove_entry(SAMPLE, &id).unwrap();
        assert!(!result.contains("2026-04-15 Rent"));
        assert!(result.contains("2026-04-01 Coffee"));
    }

    #[test]
    fn remove_entry_returns_none_for_missing_id() {
        let id = Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap();
        assert!(remove_entry(SAMPLE, &id).is_none());
    }

    #[test]
    fn replace_entry_swaps_block_in_place() {
        let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let new_block = "2026-04-02 Coffee v2\n    ; Id: 11111111-1111-1111-1111-111111111111\n    Expenses:Food:Coffee  $5.00\n    ; User: alice\n    Assets:Bank:Revolut  -$5.00\n    ; User: alice";
        let result = replace_entry(SAMPLE, &id, new_block).unwrap();
        assert!(result.contains("Coffee v2"));
        assert!(result.contains("$5.00"));
        assert!(!result.contains("Coffee\n"));
        assert!(result.contains("2026-04-15 Rent"));
    }

    #[test]
    fn entries_without_id_tag_still_parse_but_have_none_id() {
        let raw = "2026-04-01 Legacy\n    Expenses:Food  $1.00\n    Assets:Bank  -$1.00\n";
        let entries = parse_entries(raw);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].id.is_none());
        assert_eq!(entries[0].postings.len(), 2);
    }

    #[test]
    fn find_entry_block_ignores_no_id_entries() {
        let raw = "2026-04-01 Legacy\n    Expenses:Food  $1.00\n    Assets:Bank  -$1.00\n";
        let some_id = Uuid::new_v4();
        assert!(find_entry_block(raw, &some_id).is_none());
    }
}
