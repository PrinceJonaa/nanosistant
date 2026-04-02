//! Text processing deterministic functions.
//! Readability, keyword extraction, formatting, and NLP primitives.

use serde::Serialize;
use std::collections::HashMap;

/// Flesch-Kincaid reading ease score.
#[must_use]
pub fn flesch_reading_ease(text: &str) -> f64 {
    let words = word_count(text);
    let sentences = sentence_count(text);
    let syllables = syllable_count_text(text);
    if words == 0 || sentences == 0 { return 0.0; }
    let score = 206.835
        - 1.015 * (words as f64 / sentences as f64)
        - 84.6 * (syllables as f64 / words as f64);
    (score * 10.0).round() / 10.0
}

/// Reading ease label.
#[must_use]
pub fn reading_ease_label(score: f64) -> &'static str {
    if score >= 90.0       { "Very Easy (5th grade)" }
    else if score >= 80.0  { "Easy" }
    else if score >= 70.0  { "Fairly Easy" }
    else if score >= 60.0  { "Standard (8th-9th grade)" }
    else if score >= 50.0  { "Fairly Difficult" }
    else if score >= 30.0  { "Difficult (college)" }
    else                   { "Very Difficult (professional)" }
}

/// Word count.
#[must_use]
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Sentence count (rough: ends with .?!).
#[must_use]
pub fn sentence_count(text: &str) -> usize {
    let count = text.chars().filter(|&c| matches!(c, '.' | '?' | '!')).count();
    count.max(1)
}

/// Syllable count for an entire text.
#[must_use]
pub fn syllable_count_text(text: &str) -> usize {
    text.split_whitespace()
        .map(|w| syllable_count_word(w.to_lowercase().as_str()))
        .sum()
}

fn syllable_count_word(word: &str) -> usize {
    let clean: String = word.chars().filter(|c| c.is_alphabetic()).collect();
    if clean.is_empty() { return 0; }
    let mut count = 0;
    let mut prev_vowel = false;
    for ch in clean.chars() {
        let is_vowel = matches!(ch, 'a'|'e'|'i'|'o'|'u'|'y');
        if is_vowel && !prev_vowel { count += 1; }
        prev_vowel = is_vowel;
    }
    if clean.ends_with('e') && count > 1 { count -= 1; }
    count.max(1)
}

/// Estimated reading time in minutes (250 words/min average).
#[must_use]
pub fn reading_time_minutes(text: &str) -> f64 {
    let words = word_count(text);
    (words as f64 / 250.0 * 10.0).round() / 10.0
}

/// Character frequency map (lowercase alphabetic only).
#[must_use]
pub fn char_frequency(text: &str) -> Vec<(char, usize)> {
    let mut map: HashMap<char, usize> = HashMap::new();
    for ch in text.chars().filter(|c| c.is_alphabetic()) {
        *map.entry(ch.to_ascii_lowercase()).or_insert(0) += 1;
    }
    let mut result: Vec<(char, usize)> = map.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result
}

/// Extract top N keyword candidates (TF-IDF style, stopword-filtered).
#[must_use]
pub fn extract_keywords(text: &str, top_n: usize) -> Vec<(String, usize)> {
    let stopwords = stopword_set();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for word in text.split_whitespace() {
        let clean: String = word.chars()
            .filter(|c| c.is_alphanumeric()).collect();
        let lower = clean.to_lowercase();
        if lower.len() >= 3 && !stopwords.contains(lower.as_str()) {
            *counts.entry(lower).or_insert(0) += 1;
        }
    }
    let mut result: Vec<(String, usize)> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result.truncate(top_n);
    result
}

/// Keyword density: occurrences of a keyword / total words.
#[must_use]
pub fn keyword_density(text: &str, keyword: &str) -> f64 {
    let total = word_count(text);
    if total == 0 { return 0.0; }
    let lower_text = text.to_lowercase();
    let lower_kw = keyword.to_lowercase();
    let count = lower_text.split_whitespace()
        .filter(|w| {
            let clean: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
            clean == lower_kw
        }).count();
    (count as f64 / total as f64 * 100.0 * 100.0).round() / 100.0
}

/// URL-safe slug from a string.
#[must_use]
pub fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for ch in text.to_lowercase().chars() {
        if ch.is_alphanumeric() { slug.push(ch); }
        else if ch == ' ' || ch == '-' || ch == '_' {
            if !slug.ends_with('-') { slug.push('-'); }
        }
    }
    slug.trim_matches('-').to_string()
}

/// Truncate text to max_chars, adding ellipsis if needed.
#[must_use]
pub fn truncate(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars { return text.to_string(); }
    // Try to break at a word boundary
    let slice = &text[..max_chars.min(text.len())];
    if let Some(pos) = slice.rfind(|c: char| c.is_whitespace()) {
        format!("{}…", &text[..pos])
    } else {
        format!("{}…", slice)
    }
}

/// Count unique words in text.
#[must_use]
pub fn unique_word_count(text: &str) -> usize {
    use std::collections::HashSet;
    text.split_whitespace()
        .map(|w| w.to_lowercase())
        .collect::<HashSet<_>>()
        .len()
}

/// Lexical diversity: unique_words / total_words.
#[must_use]
pub fn lexical_diversity(text: &str) -> f64 {
    let total = word_count(text);
    if total == 0 { return 0.0; }
    let unique = unique_word_count(text);
    (unique as f64 / total as f64 * 1000.0).round() / 1000.0
}

/// Detect language direction hint from character sets.
#[must_use]
pub fn text_direction_hint(text: &str) -> &'static str {
    let rtl_count = text.chars().filter(|&c| {
        matches!(c as u32, 0x0600..=0x06FF | 0x0590..=0x05FF | 0xFB50..=0xFDFF)
    }).count();
    let total_alpha = text.chars().filter(|c| c.is_alphabetic()).count();
    if total_alpha > 0 && rtl_count > total_alpha / 2 { "rtl" } else { "ltr" }
}

/// Basic sentiment signals: positive/negative word ratio.
/// Not a real sentiment model — keyword heuristic only.
#[derive(Debug, Serialize)]
pub struct SentimentHint {
    pub positive_count: usize,
    pub negative_count: usize,
    pub signal: String,
}

#[must_use]
pub fn sentiment_hint(text: &str) -> SentimentHint {
    let positive = ["good","great","excellent","amazing","awesome","love","best",
                    "perfect","beautiful","wonderful","fantastic","happy","positive",
                    "win","winning","growth","success","strong","brilliant","joy"];
    let negative = ["bad","terrible","awful","horrible","hate","worst","ugly",
                    "poor","negative","fail","failure","weak","broken","wrong",
                    "loss","losing","crash","problem","issue","error","sad","angry"];
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let pos = words.iter().filter(|w| positive.contains(&w.as_ref())).count();
    let neg = words.iter().filter(|w| negative.contains(&w.as_ref())).count();
    let signal = if pos > neg * 2 { "positive" }
                 else if neg > pos * 2 { "negative" }
                 else { "neutral" };
    SentimentHint {
        positive_count: pos,
        negative_count: neg,
        signal: signal.to_string(),
    }
}

fn stopword_set() -> &'static std::collections::HashSet<&'static str> {
    use std::sync::OnceLock;
    static SET: OnceLock<std::collections::HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        ["a","an","the","and","or","but","in","on","at","to","for","of",
         "with","by","from","is","was","are","were","be","been","being",
         "have","has","had","do","does","did","will","would","could","should",
         "may","might","shall","can","this","that","these","those","it","its",
         "he","she","they","we","you","i","my","your","our","their","his","her",
         "not","no","so","if","as","up","out","about","into","than","then",
         "more","most","also","just","even","still","only","any","all","both"]
            .iter().cloned().collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn word_count_basic() {
        assert_eq!(word_count("hello world how are you"), 5);
    }
    #[test] fn flesch_score_simple() {
        let text = "The cat sat on the mat. It was a big cat.";
        let score = flesch_reading_ease(text);
        assert!(score > 50.0, "score: {score}");
    }
    #[test] fn slugify_works() {
        assert_eq!(slugify("Hello World! How are you?"), "hello-world-how-are-you");
    }
    #[test] fn truncate_works() {
        let t = truncate("Hello wonderful world of Rust", 15);
        assert!(t.ends_with('…'));
        assert!(t.len() < 20);
    }
    #[test] fn extract_keywords_returns() {
        let text = "rust programming language fast performance systems rust";
        let kw = extract_keywords(text, 3);
        assert!(!kw.is_empty());
        assert_eq!(kw[0].0, "rust");
    }
    #[test] fn keyword_density_works() {
        let d = keyword_density("rust is great rust is fast", "rust");
        assert!((d - 33.33).abs() < 1.0);
    }
    #[test] fn sentiment_hint_positive() {
        let s = sentiment_hint("this is amazing great and wonderful");
        assert_eq!(s.signal, "positive");
    }
    #[test] fn lexical_diversity_unique() {
        let d = lexical_diversity("a b c d e");
        assert!((d - 1.0).abs() < 0.001);
    }
}
