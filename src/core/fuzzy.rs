//! High-Performance Fuzzy Search & String Ranking Engine
//!
//! Features:
//! - Subsequence Matching with Positional Bonus
//! - Word Boundary, CamelCase, and Path Separator bonuses
//! - Consecutive Character Match multiplier
//! - Extension match weighting
//! - Zero external dependencies, pure Rust O(N) scoring

/// Evaluates how closely `target` matches `query`.
/// Returns `Some(score)` if `query` is a subsequence of `target`, or `None` if it does not match.
/// Higher score indicates a better, more relevant match.
pub fn fuzzy_match(query: &str, target: &str) -> Option<i64> {
    let q = query.trim();
    if q.is_empty() {
        return Some(100);
    }

    let q_chars: Vec<char> = q.chars().collect();
    let t_chars: Vec<char> = target.chars().collect();

    if q_chars.len() > t_chars.len() {
        return None;
    }

    let mut score = 0i64;
    let mut q_idx = 0;
    let mut consecutive_matches = 0i64;
    let mut first_match_idx = None;

    for (t_idx, &t_char) in t_chars.iter().enumerate() {
        if q_idx >= q_chars.len() {
            break;
        }

        let q_char = q_chars[q_idx];
        let matched = if q_char.is_ascii() && t_char.is_ascii() {
            q_char.eq_ignore_ascii_case(&t_char)
        } else {
            q_char.to_lowercase().eq(t_char.to_lowercase())
        };

        if matched {
            if first_match_idx.is_none() {
                first_match_idx = Some(t_idx);
            }

            // Base match score
            let mut match_bonus = 10i64;

            // Bonus for exact case match
            if q_char == t_char {
                match_bonus += 5;
            }

            // Consecutive match bonus (rewards substrings)
            consecutive_matches += 1;
            match_bonus += consecutive_matches * 10;

            // Word boundary bonus (following space, dot, underscore, dash, slash, or camelCase)
            if t_idx == 0 {
                match_bonus += 30; // Beginning of string bonus
            } else {
                let prev_char = t_chars[t_idx - 1];
                if matches!(prev_char, ' ' | '.' | '_' | '-' | '/' | '\\') {
                    match_bonus += 25;
                } else if prev_char.is_lowercase() && t_char.is_uppercase() {
                    match_bonus += 20; // CamelCase bonus
                }
            }

            // Bonus for match in file extension
            if target[t_idx..].contains('.') {
                // Inside directory or stem
            } else {
                match_bonus += 15; // In file extension
            }

            score += match_bonus;
            q_idx += 1;
        } else {
            consecutive_matches = 0;
        }
    }

    if q_idx == q_chars.len() {
        // Penalty for gap between start and first match
        if let Some(first) = first_match_idx {
            score -= (first as i64) * 2;
        }
        // Penalty for extra length difference
        let len_diff = (t_chars.len() - q_chars.len()) as i64;
        score -= len_diff;

        Some(score)
    } else {
        None
    }
}

/// Helper to sort items by fuzzy score descending
pub fn sort_by_fuzzy_match<'a, T, F>(items: &mut Vec<T>, query: &str, mut extract_key: F)
where
    F: FnMut(&T) -> &'a str,
{
    if query.trim().is_empty() {
        return;
    }

    let mut scored: Vec<(T, i64)> = items
        .drain(..)
        .filter_map(|item| {
            let key = extract_key(&item);
            fuzzy_match(query, key).map(|score| (item, score))
        })
        .collect();

    scored.sort_by_key(|a| std::cmp::Reverse(a.1));
    items.extend(scored.into_iter().map(|(item, _)| item));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact_and_prefix() {
        assert!(fuzzy_match("doc", "document.pdf").is_some());
        assert!(fuzzy_match("pdf", "document.pdf").is_some());
        assert!(fuzzy_match("xyz", "document.pdf").is_none());

        let score_prefix = fuzzy_match("doc", "document.pdf").unwrap();
        let score_mid = fuzzy_match("men", "document.pdf").unwrap();
        assert!(score_prefix > score_mid);
    }

    #[test]
    fn test_fuzzy_match_word_boundary() {
        let score_word = fuzzy_match("photo", "my_vacation_photo.jpg").unwrap();
        assert!(score_word > 0);
    }

    #[test]
    fn test_sort_by_fuzzy_match() {
        let mut list = vec![
            "annual_report_2026.pdf",
            "report.docx",
            "weekly_report.xlsx",
            "random_notes.txt",
        ];
        sort_by_fuzzy_match(&mut list, "rep", |s| *s);
        assert_eq!(list[0], "report.docx");
    }
}
