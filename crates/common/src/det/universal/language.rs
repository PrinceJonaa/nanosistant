//! Language primitives, pattern matching, formal grammar, and symbol analysis — pure deterministic functions.

// ═══════════════════════════════════════
// String Primitives
// ═══════════════════════════════════════

/// Count Unicode scalar values (characters) in `s`.
#[must_use]
pub fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Count whitespace-delimited words in `s`.
#[must_use]
pub fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Count sentences by splitting on `.`, `!`, and `?`.
///
/// Consecutive or trailing terminators do not produce empty sentences.
#[must_use]
pub fn sentence_count(s: &str) -> usize {
    s.split(|c| c == '.' || c == '!' || c == '?')
        .filter(|seg| !seg.trim().is_empty())
        .count()
}

/// Estimate the number of syllables in a single word using a vowel-cluster heuristic.
///
/// Counts contiguous runs of vowel characters (a, e, i, o, u — case-insensitive),
/// then subtracts one for each silent trailing `e`. Minimum result is 1.
#[must_use]
pub fn syllable_estimate(word: &str) -> usize {
    if word.is_empty() {
        return 0;
    }
    let lower: Vec<char> = word.to_lowercase().chars().collect();
    let vowels = "aeiou";

    // Count vowel clusters.
    let mut count = 0usize;
    let mut in_cluster = false;
    for &c in &lower {
        if vowels.contains(c) {
            if !in_cluster {
                count += 1;
                in_cluster = true;
            }
        } else {
            in_cluster = false;
        }
    }

    // Subtract for silent trailing 'e': fires when the word ends in
    // <non-syllabic-consonant> + 'e'.  Consonants 'l' and 'r' are excluded
    // because they form a syllable of their own in endings like "-le", "-re".
    if count > 1 {
        let n = lower.len();
        if n >= 2 {
            let last  = lower[n - 1];
            let penul = lower[n - 2];
            let penul_is_vowel      = "aeiou".contains(penul);
            let penul_is_syllabic   = penul == 'l' || penul == 'r';
            if last == 'e' && !penul_is_vowel && !penul_is_syllabic {
                count -= 1;
            }
        }
    }

    count.max(1)
}

/// Mean character length of whitespace-delimited words in `s`.
///
/// Returns `0.0` for an empty string.
#[must_use]
pub fn avg_word_length(s: &str) -> f64 {
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.is_empty() {
        return 0.0;
    }
    let total: usize = words.iter().map(|w| w.chars().count()).sum();
    total as f64 / words.len() as f64
}

// ═══════════════════════════════════════
// Pattern Matching
// ═══════════════════════════════════════

/// Return `true` if `s` reads the same forwards and backwards, ignoring case
/// and non-alphanumeric characters (Unicode-aware).
#[must_use]
pub fn is_palindrome(s: &str) -> bool {
    let filtered: Vec<char> = s
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect();
    let reversed: Vec<char> = filtered.iter().copied().rev().collect();
    filtered == reversed
}

/// Return the longest common prefix shared by strings `a` and `b`.
#[must_use]
pub fn longest_common_prefix(a: &str, b: &str) -> String {
    a.chars()
        .zip(b.chars())
        .take_while(|(x, y)| x == y)
        .map(|(c, _)| c)
        .collect()
}

/// Compute the Levenshtein (edit) distance between `a` and `b` using
/// the Wagner–Fischer dynamic-programming algorithm.
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    // dp[i][j] = edit distance between a[..i] and b[..j].
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }

    for i in 1..=m {
        for j in 1..=n {
            if a_chars[i - 1] == b_chars[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            } else {
                dp[i][j] = 1 + dp[i - 1][j - 1]   // substitution
                    .min(dp[i - 1][j])               // deletion
                    .min(dp[i][j - 1]);              // insertion
            }
        }
    }
    dp[m][n]
}

/// Normalised similarity: `1.0 - edit_distance(a, b) / max(len_a, len_b)`.
///
/// Returns `1.0` when both strings are empty, `0.0` when completely different.
#[must_use]
pub fn similarity_ratio(a: &str, b: &str) -> f64 {
    let len_a = a.chars().count();
    let len_b = b.chars().count();
    let max_len = len_a.max(len_b);
    if max_len == 0 {
        return 1.0;
    }
    let dist = edit_distance(a, b);
    1.0 - dist as f64 / max_len as f64
}

/// Return `true` if `pattern` occurs as a substring of `text` (naive search).
#[must_use]
pub fn contains_pattern(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    let t_chars: Vec<char> = text.chars().collect();
    let p_chars: Vec<char> = pattern.chars().collect();
    let (tl, pl) = (t_chars.len(), p_chars.len());
    if pl > tl { return false; }
    (0..=(tl - pl)).any(|i| t_chars[i..i + pl] == p_chars[..])
}

// ═══════════════════════════════════════
// Formal Grammar / Tokenization
// ═══════════════════════════════════════

/// Split `s` into lowercase tokens, stripping whitespace and punctuation.
///
/// A token is a maximal run of alphabetic or numeric characters.
#[must_use]
pub fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                current.push(lc);
            }
        } else if !current.is_empty() {
            tokens.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Generate all contiguous n-grams from a token slice.
///
/// Returns an empty `Vec` when `n == 0` or `n > tokens.len()`.
#[must_use]
pub fn ngrams(tokens: &[String], n: usize) -> Vec<Vec<String>> {
    if n == 0 || n > tokens.len() {
        return Vec::new();
    }
    (0..=(tokens.len() - n))
        .map(|i| tokens[i..i + n].to_vec())
        .collect()
}

/// Count occurrences of each token and return pairs sorted by count descending,
/// then alphabetically ascending for ties.
#[must_use]
pub fn token_frequency(tokens: &[String]) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for tok in tokens {
        if let Some(entry) = counts.iter_mut().find(|(t, _)| t == tok) {
            entry.1 += 1;
        } else {
            counts.push((tok.clone(), 1));
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    counts
}

/// Type-token ratio (TTR): unique token count divided by total token count.
///
/// Returns `0.0` for an empty slice.
#[must_use]
pub fn type_token_ratio(tokens: &[String]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }
    let mut unique: Vec<&String> = tokens.iter().collect();
    unique.dedup(); // sort first to make dedup meaningful
    // Manual unique count without sorting (preserve original semantics).
    let mut seen: Vec<&String> = Vec::new();
    for t in tokens {
        if !seen.contains(&t) {
            seen.push(t);
        }
    }
    seen.len() as f64 / tokens.len() as f64
}

// ═══════════════════════════════════════
// Encoding / Symbol
// ═══════════════════════════════════════

/// Return `true` if `s` is a valid ASCII identifier: starts with a letter or
/// underscore, followed by letters, digits, or underscores only.
#[must_use]
pub fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Convert a `CamelCase` or `PascalCase` string to `snake_case`.
///
/// A new word boundary is detected before each uppercase letter that follows
/// a lowercase letter or digit, or before an uppercase letter that is followed
/// by a lowercase letter (handles `XMLParser` → `xml_parser`).
#[must_use]
pub fn camel_to_snake(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_lower = i > 0 && chars[i - 1].is_lowercase();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            if i > 0 && (prev_lower || (next_lower && prev_upper)) {
                result.push('_');
            }
            for lc in c.to_lowercase() {
                result.push(lc);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert a `snake_case` string to `camelCase` (lower camel case).
///
/// Leading underscores are stripped; segments separated by `_` are title-cased.
#[must_use]
pub fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalise_next = false;
    let mut first_char = true;
    for c in s.chars() {
        if c == '_' {
            capitalise_next = true;
        } else if first_char {
            for lc in c.to_lowercase() {
                result.push(lc);
            }
            first_char = false;
            capitalise_next = false;
        } else if capitalise_next {
            for uc in c.to_uppercase() {
                result.push(uc);
            }
            capitalise_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Produce a URL-friendly slug: lowercase, runs of non-alphanumeric characters
/// replaced by a single `-`, with leading/trailing `-` stripped.
#[must_use]
pub fn slugify(s: &str) -> String {
    let mut result = String::new();
    let mut prev_dash = true; // suppress leading dash
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            for lc in c.to_lowercase() {
                result.push(lc);
            }
            prev_dash = false;
        } else if !prev_dash {
            result.push('-');
            prev_dash = true;
        }
    }
    // Strip trailing dash.
    if result.ends_with('-') {
        result.pop();
    }
    result
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── String primitives ────────────────

    #[test] fn test_char_count() {
        assert_eq!(char_count("hello"), 5);
        assert_eq!(char_count("héllo"), 5); // é = one scalar
        assert_eq!(char_count(""), 0);
    }

    #[test] fn test_word_and_sentence_count() {
        assert_eq!(word_count("The quick brown fox"), 4);
        assert_eq!(word_count(""), 0);
        assert_eq!(sentence_count("Hello! How are you? Fine."), 3);
        assert_eq!(sentence_count("No terminator"), 1);
        assert_eq!(sentence_count(""), 0);
    }

    #[test] fn test_syllable_estimate() {
        assert_eq!(syllable_estimate("cat"),   1);
        assert_eq!(syllable_estimate("table"), 2); // ta-ble (trailing e removed from 2 clusters)
        assert_eq!(syllable_estimate("beautiful"), 3); // beau-ti-ful
        assert_eq!(syllable_estimate(""),      0);
        assert_eq!(syllable_estimate("the"),   1); // one cluster 'e', silent-e rule keeps min 1
    }

    #[test] fn test_avg_word_length() {
        // "hi" (2) + "bye" (3) = 5 / 2 = 2.5
        assert!((avg_word_length("hi bye") - 2.5).abs() < 1e-9);
        assert_eq!(avg_word_length(""), 0.0);
    }

    // ── Pattern matching ─────────────────

    #[test] fn test_is_palindrome() {
        assert!( is_palindrome("racecar"));
        assert!( is_palindrome("A man a plan a canal Panama"));
        assert!( is_palindrome(""));
        assert!(!is_palindrome("hello"));
        assert!( is_palindrome("Was it a car or a cat I saw"));
    }

    #[test] fn test_longest_common_prefix() {
        assert_eq!(longest_common_prefix("flower", "flow"),    "flow");
        assert_eq!(longest_common_prefix("dog",    "racecar"), "");
        assert_eq!(longest_common_prefix("abc",    "abc"),     "abc");
        assert_eq!(longest_common_prefix("",       "abc"),     "");
    }

    #[test] fn test_edit_distance() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("",       "abc"),     3);
        assert_eq!(edit_distance("abc",    "abc"),     0);
        assert_eq!(edit_distance("a",      ""),        1);
    }

    #[test] fn test_similarity_ratio() {
        assert!((similarity_ratio("abc", "abc") - 1.0).abs() < 1e-9);
        assert!((similarity_ratio("",    ""   ) - 1.0).abs() < 1e-9);
        let r = similarity_ratio("kitten", "sitting");
        // edit_distance = 3, max_len = 7 → 1 - 3/7 ≈ 0.571
        assert!((r - (1.0 - 3.0 / 7.0)).abs() < 1e-9);
    }

    #[test] fn test_contains_pattern() {
        assert!( contains_pattern("hello world", "world"));
        assert!( contains_pattern("hello world", ""));
        assert!(!contains_pattern("hello world", "xyz"));
        assert!( contains_pattern("abcdef", "cde"));
    }

    // ── Formal grammar / tokenization ────

    #[test] fn test_tokenize() {
        assert_eq!(tokenize("Hello, World!"), vec!["hello", "world"]);
        assert_eq!(tokenize("one two  three"), vec!["one", "two", "three"]);
        assert_eq!(tokenize(""), Vec::<String>::new());
    }

    #[test] fn test_ngrams() {
        let toks: Vec<String> = vec!["a","b","c","d"].iter().map(|s| s.to_string()).collect();
        let bi = ngrams(&toks, 2);
        assert_eq!(bi.len(), 3);
        assert_eq!(bi[0], vec!["a", "b"]);
        assert_eq!(bi[2], vec!["c", "d"]);
        assert!(ngrams(&toks, 0).is_empty());
        assert!(ngrams(&toks, 5).is_empty());
    }

    #[test] fn test_token_frequency() {
        let toks: Vec<String> = vec!["a","b","a","c","b","a"].iter().map(|s| s.to_string()).collect();
        let freq = token_frequency(&toks);
        assert_eq!(freq[0], ("a".to_string(), 3));
        assert_eq!(freq[1], ("b".to_string(), 2));
        assert_eq!(freq[2], ("c".to_string(), 1));
    }

    #[test] fn test_type_token_ratio() {
        let all_same: Vec<String> = vec!["a","a","a"].iter().map(|s| s.to_string()).collect();
        assert!((type_token_ratio(&all_same) - 1.0 / 3.0).abs() < 1e-9);
        let all_diff: Vec<String> = vec!["a","b","c"].iter().map(|s| s.to_string()).collect();
        assert!((type_token_ratio(&all_diff) - 1.0).abs() < 1e-9);
        assert_eq!(type_token_ratio(&[]), 0.0);
    }

    // ── Encoding / symbol ────────────────

    #[test] fn test_is_valid_identifier() {
        assert!( is_valid_identifier("hello"));
        assert!( is_valid_identifier("_private"));
        assert!( is_valid_identifier("foo_bar_2"));
        assert!(!is_valid_identifier("2bad"));
        assert!(!is_valid_identifier("has space"));
        assert!(!is_valid_identifier(""));
    }

    #[test] fn test_camel_to_snake() {
        assert_eq!(camel_to_snake("CamelCase"),   "camel_case");
        assert_eq!(camel_to_snake("myVariable"),  "my_variable");
        assert_eq!(camel_to_snake("XMLParser"),   "xml_parser");
        assert_eq!(camel_to_snake("simple"),      "simple");
        assert_eq!(camel_to_snake(""),            "");
    }

    #[test] fn test_snake_to_camel() {
        assert_eq!(snake_to_camel("snake_case"),       "snakeCase");
        assert_eq!(snake_to_camel("my_variable_name"), "myVariableName");
        assert_eq!(snake_to_camel("simple"),           "simple");
        assert_eq!(snake_to_camel(""),                 "");
    }

    #[test] fn test_slugify() {
        assert_eq!(slugify("Hello World"),        "hello-world");
        assert_eq!(slugify("  leading spaces"),   "leading-spaces");
        assert_eq!(slugify("special!@#chars"),    "special-chars");
        assert_eq!(slugify("already-slug"),       "already-slug");
        assert_eq!(slugify(""),                   "");
    }
}
