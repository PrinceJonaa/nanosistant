//! Rule evaluator — deterministic, zero-LLM rule execution.
//!
//! Evaluates queries against a set of [`Rule`]s using keyword matching
//! (Option A) and executes the matched formula.  The arithmetic sub-evaluator
//! uses a hand-rolled shunting-yard / recursive-descent parser — no external
//! crates, no I/O.

use std::collections::HashMap;
use crate::rules::{Rule, RuleFormula};

// ═══════════════════════════════════════
// RuleEvaluator
// ═══════════════════════════════════════

/// Evaluates queries against a loaded set of [`Rule`]s.
pub struct RuleEvaluator {
    rules: Vec<Rule>,
}

impl RuleEvaluator {
    /// Create an evaluator from a list of (approved) rules.
    #[must_use]
    pub fn new(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    /// Try to evaluate a query against loaded rules.
    ///
    /// Returns `Some((response, confidence))` for the highest-confidence rule
    /// that fires.  Returns `None` if no rule matches.
    #[must_use]
    pub fn evaluate(&self, query: &str) -> Option<(String, f64)> {
        let candidates = self.keyword_match(query);
        let mut best: Option<(String, f64)> = None;

        for rule in candidates {
            if !rule.approved {
                continue;
            }
            let score = self.keyword_score(rule, query);
            let effective_confidence = rule.confidence * score;
            if let Some(response) = self.evaluate_formula(&rule.formula, query, score) {
                match &best {
                    Some((_, prev_conf)) if *prev_conf >= effective_confidence => {}
                    _ => best = Some((response, effective_confidence)),
                }
            }
        }

        best
    }

    /// Return all rules whose trigger keywords appear in the query.
    fn keyword_match<'a>(&'a self, query: &str) -> Vec<&'a Rule> {
        let q = query.to_lowercase();
        self.rules
            .iter()
            .filter(|r| {
                r.trigger_keywords
                    .iter()
                    .any(|kw| q.contains(kw.to_lowercase().as_str()))
            })
            .collect()
    }

    /// Score how many trigger keywords from the rule match the query.
    ///
    /// Score = matching_count / total_keywords, clamped to [0.0, 1.0].
    fn keyword_score(&self, rule: &Rule, query: &str) -> f64 {
        if rule.trigger_keywords.is_empty() {
            return 0.0;
        }
        let q = query.to_lowercase();
        let matching = rule
            .trigger_keywords
            .iter()
            .filter(|kw| q.contains(kw.to_lowercase().as_str()))
            .count();
        (matching as f64 / rule.trigger_keywords.len() as f64).clamp(0.0, 1.0)
    }

    /// Execute a [`RuleFormula`] against the query.
    fn evaluate_formula(&self, formula: &RuleFormula, query: &str, score: f64) -> Option<String> {
        match formula {
            RuleFormula::Static { response } => Some(response.clone()),

            RuleFormula::Arithmetic { expr, variables } => {
                // Build a variable map from names → extracted numbers
                let mut vars: HashMap<String, f64> = HashMap::new();
                if let Some(num) = self.extract_number(query) {
                    for var in variables {
                        vars.insert(var.clone(), num);
                    }
                }
                self.eval_arithmetic(expr, &vars)
                    .map(|n| format!("{n:.3}"))
            }

            RuleFormula::Lookup { table } => {
                let q = query.to_lowercase();
                table
                    .iter()
                    .find(|(k, _)| q.contains(k.to_lowercase().as_str()))
                    .map(|(_, v)| v.clone())
            }

            RuleFormula::WeightedScore { weights } => {
                let q = query.to_lowercase();
                let total_weight: f64 = weights.values().sum();
                if total_weight == 0.0 {
                    return None;
                }
                let matched_weight: f64 = weights
                    .iter()
                    .filter(|(k, _)| q.contains(k.to_lowercase().as_str()))
                    .map(|(_, w)| w)
                    .sum();
                Some(format!("{:.3}", matched_weight / total_weight))
            }

            RuleFormula::Classification { thresholds } => {
                // Use `score` (keyword match ratio) as the classification input
                let mut sorted = thresholds.clone();
                sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                let label = sorted
                    .iter()
                    .rev()
                    .find(|(threshold, _)| score >= *threshold)
                    .map(|(_, label)| label.clone())
                    .unwrap_or_else(|| "unclassified".to_string());
                Some(label)
            }

            RuleFormula::Template { template, slots: _ } => {
                // Fill numeric slot with the first extracted number
                let mut result = template.clone();
                if let Some(num) = self.extract_number(query) {
                    // Extract slot names first (collecting owned Strings to avoid borrow)
                    let slot_names: Vec<String> = result
                        .split('{')
                        .skip(1)
                        .filter_map(|s| s.split('}').next())
                        .map(|s| s.to_string())
                        .collect();
                    for slot in slot_names {
                        result = result.replace(&format!("{{{slot}}}"), &format!("{num:.3}"));
                    }
                }
                Some(result)
            }
        }
    }

    /// Evaluate a simple arithmetic expression with named variable substitution.
    ///
    /// Supports: `+`, `-`, `*`, `/`, `^` (power), unary `-`, parentheses.
    /// Variable names from `vars` are substituted before parsing.
    ///
    /// Returns `None` on parse error or division-by-zero.
    pub fn eval_arithmetic(&self, expr: &str, vars: &HashMap<String, f64>) -> Option<f64> {
        // Substitute variables
        let mut expr = expr.to_string();
        for (name, val) in vars {
            expr = expr.replace(name.as_str(), &val.to_string());
        }
        eval_expr(expr.trim())
    }

    /// Extract the first floating-point number from a query string.
    pub fn extract_number(&self, query: &str) -> Option<f64> {
        // Walk through words and try to parse each as f64
        for word in query.split_whitespace() {
            // Strip trailing punctuation
            let clean: String = word.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
            if !clean.is_empty() {
                if let Ok(n) = clean.parse::<f64>() {
                    return Some(n);
                }
            }
        }
        None
    }
}

// ═══════════════════════════════════════
// Arithmetic parser (shunting-yard + eval)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Op(char),
    LParen,
    RParen,
}

fn tokenize(expr: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            let n: f64 = s.parse().ok()?;
            tokens.push(Token::Num(n));
            continue;
        }
        match c {
            '+' | '-' | '*' | '/' | '^' => {
                tokens.push(Token::Op(c));
                i += 1;
            }
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            _ => return None, // unknown character
        }
    }
    Some(tokens)
}

fn precedence(op: char) -> i32 {
    match op {
        '+' | '-' => 1,
        '*' | '/' => 2,
        '^' => 3,
        _ => 0,
    }
}

fn right_assoc(op: char) -> bool {
    op == '^'
}

/// Evaluate a flat token stream using shunting-yard → RPN → stack eval.
fn eval_tokens(tokens: Vec<Token>) -> Option<f64> {
    // Shunting-yard: convert infix to RPN, handling unary minus
    let mut output: Vec<f64> = Vec::new();
    let mut ops: Vec<char> = Vec::new();
    let mut prev_was_value = false;

    for tok in tokens {
        match tok {
            Token::Num(n) => {
                output.push(n);
                prev_was_value = true;
            }
            Token::Op(op) => {
                // Detect unary minus: if previous token was not a value/rparen
                if op == '-' && !prev_was_value {
                    // Push sentinel for unary minus as a pseudo-operator
                    // We handle it by pushing 0 and binary minus
                    output.push(0.0);
                }
                while let Some(&top) = ops.last() {
                    if top == '(' {
                        break;
                    }
                    if precedence(top) > precedence(op)
                        || (precedence(top) == precedence(op) && !right_assoc(op))
                    {
                        ops.pop();
                        apply_op(top, &mut output)?;
                    } else {
                        break;
                    }
                }
                ops.push(op);
                prev_was_value = false;
            }
            Token::LParen => {
                ops.push('(');
                prev_was_value = false;
            }
            Token::RParen => {
                while let Some(&top) = ops.last() {
                    if top == '(' { break; }
                    ops.pop();
                    apply_op(top, &mut output)?;
                }
                if ops.last() == Some(&'(') {
                    ops.pop();
                } else {
                    return None; // mismatched parens
                }
                prev_was_value = true;
            }
        }
    }
    while let Some(op) = ops.pop() {
        if op == '(' { return None; }
        apply_op(op, &mut output)?;
    }
    if output.len() == 1 {
        Some(output[0])
    } else {
        None
    }
}

fn apply_op(op: char, stack: &mut Vec<f64>) -> Option<()> {
    let b = stack.pop()?;
    let a = stack.pop()?;
    let result = match op {
        '+' => a + b,
        '-' => a - b,
        '*' => a * b,
        '/' => {
            if b == 0.0 { return None; }
            a / b
        }
        '^' => a.powf(b),
        _ => return None,
    };
    stack.push(result);
    Some(())
}

/// Top-level expression evaluator.
fn eval_expr(expr: &str) -> Option<f64> {
    let tokens = tokenize(expr)?;
    eval_tokens(tokens)
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Rule, RuleFormula, RuleExample};

    // ── arithmetic eval ──────────────────────────────────────────────────────

    fn evaluator_empty() -> RuleEvaluator {
        RuleEvaluator::new(vec![])
    }

    #[test]
    fn arithmetic_addition() {
        let ev = evaluator_empty();
        let vars = HashMap::new();
        assert_eq!(ev.eval_arithmetic("2 + 3", &vars), Some(5.0));
    }

    #[test]
    fn arithmetic_subtraction() {
        let ev = evaluator_empty();
        let vars = HashMap::new();
        assert_eq!(ev.eval_arithmetic("10 - 4", &vars), Some(6.0));
    }

    #[test]
    fn arithmetic_multiplication() {
        let ev = evaluator_empty();
        let vars = HashMap::new();
        assert_eq!(ev.eval_arithmetic("3 * 7", &vars), Some(21.0));
    }

    #[test]
    fn arithmetic_division() {
        let ev = evaluator_empty();
        let vars = HashMap::new();
        let result = ev.eval_arithmetic("10 / 4", &vars).unwrap();
        assert!((result - 2.5).abs() < 1e-10);
    }

    #[test]
    fn arithmetic_power() {
        let ev = evaluator_empty();
        let vars = HashMap::new();
        assert_eq!(ev.eval_arithmetic("2 ^ 8", &vars), Some(256.0));
    }

    #[test]
    fn arithmetic_parentheses() {
        let ev = evaluator_empty();
        let vars = HashMap::new();
        let result = ev.eval_arithmetic("(2 + 3) * 4", &vars).unwrap();
        assert!((result - 20.0).abs() < 1e-10);
    }

    #[test]
    fn arithmetic_variable_substitution() {
        let ev = evaluator_empty();
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), 120.0);
        // BPM → bar duration: 60 / bpm * 4
        let result = ev.eval_arithmetic("60 / x * 4", &vars).unwrap();
        assert!((result - 2.0).abs() < 1e-10);
    }

    #[test]
    fn arithmetic_complex_expression() {
        let ev = evaluator_empty();
        let vars = HashMap::new();
        // (3 + 4) * 2 ^ 2 = 7 * 4 = 28
        let result = ev.eval_arithmetic("(3 + 4) * 2 ^ 2", &vars).unwrap();
        assert!((result - 28.0).abs() < 1e-10);
    }

    #[test]
    fn arithmetic_division_by_zero_returns_none() {
        let ev = evaluator_empty();
        let vars = HashMap::new();
        assert_eq!(ev.eval_arithmetic("5 / 0", &vars), None);
    }

    // ── number extraction ─────────────────────────────────────────────────────

    #[test]
    fn extracts_integer_from_query() {
        let ev = evaluator_empty();
        assert_eq!(ev.extract_number("what is 120 bpm"), Some(120.0));
    }

    #[test]
    fn extracts_float_from_query() {
        let ev = evaluator_empty();
        assert_eq!(ev.extract_number("bpm is 140.5"), Some(140.5));
    }

    #[test]
    fn no_number_returns_none() {
        let ev = evaluator_empty();
        assert_eq!(ev.extract_number("what is the scale"), None);
    }

    // ── keyword matching ──────────────────────────────────────────────────────

    fn bpm_rule() -> Rule {
        Rule {
            id: "bpm-bar".to_string(),
            description: "BPM to bar duration".to_string(),
            trigger_keywords: vec!["bpm".to_string(), "bar".to_string()],
            semantic_hint: None,
            formula: RuleFormula::Arithmetic {
                expr: "60 / x * 4".to_string(),
                variables: vec!["x".to_string()],
            },
            confidence: 0.95,
            examples: vec![],
            proposed_by: "dreamer".to_string(),
            approved: true,
        }
    }

    fn static_rule() -> Rule {
        Rule {
            id: "hello".to_string(),
            description: "Say hello".to_string(),
            trigger_keywords: vec!["hello".to_string(), "hi".to_string()],
            semantic_hint: None,
            formula: RuleFormula::Static { response: "Hello, operator!".to_string() },
            confidence: 0.80,
            examples: vec![],
            proposed_by: "operator".to_string(),
            approved: true,
        }
    }

    #[test]
    fn keyword_match_fires_on_partial_keyword() {
        let ev = RuleEvaluator::new(vec![bpm_rule()]);
        let result = ev.evaluate("what is the bar at 120 bpm");
        assert!(result.is_some());
    }

    #[test]
    fn keyword_match_misses_unrelated_query() {
        let ev = RuleEvaluator::new(vec![bpm_rule()]);
        let result = ev.evaluate("calculate the distance to the moon");
        assert!(result.is_none());
    }

    #[test]
    fn static_rule_returns_fixed_response() {
        let ev = RuleEvaluator::new(vec![static_rule()]);
        let (resp, _conf) = ev.evaluate("hello there").unwrap();
        assert_eq!(resp, "Hello, operator!");
    }

    #[test]
    fn unapproved_rule_does_not_fire() {
        let mut rule = bpm_rule();
        rule.approved = false;
        let ev = RuleEvaluator::new(vec![rule]);
        assert!(ev.evaluate("what is 120 bpm bar duration").is_none());
    }

    #[test]
    fn arithmetic_rule_evaluates_correctly() {
        let ev = RuleEvaluator::new(vec![bpm_rule()]);
        let (resp, _conf) = ev.evaluate("what is a bar at 120 bpm").unwrap();
        // 60 / 120 * 4 = 2.0
        assert!(resp.starts_with("2.000"), "got: {resp}");
    }

    #[test]
    fn lookup_rule_evaluates() {
        let mut table = HashMap::new();
        table.insert("C major".to_string(), "C D E F G A B".to_string());
        let rule = Rule {
            id: "key-lookup".to_string(),
            description: "Key lookup".to_string(),
            trigger_keywords: vec!["key".to_string(), "major".to_string()],
            semantic_hint: None,
            formula: RuleFormula::Lookup { table },
            confidence: 0.8,
            examples: vec![],
            proposed_by: "operator".to_string(),
            approved: true,
        };
        let ev = RuleEvaluator::new(vec![rule]);
        let (resp, _) = ev.evaluate("what notes are in C major key").unwrap();
        assert_eq!(resp, "C D E F G A B");
    }

    #[test]
    fn weighted_score_rule_evaluates() {
        let mut weights = HashMap::new();
        weights.insert("good".to_string(), 3.0);
        weights.insert("bad".to_string(), 1.0);
        let rule = Rule {
            id: "sentiment".to_string(),
            description: "Sentiment score".to_string(),
            trigger_keywords: vec!["good".to_string(), "bad".to_string()],
            semantic_hint: None,
            formula: RuleFormula::WeightedScore { weights },
            confidence: 0.7,
            examples: vec![],
            proposed_by: "operator".to_string(),
            approved: true,
        };
        let ev = RuleEvaluator::new(vec![rule]);
        // "this is good" matches "good" (3.0) / total (4.0) = 0.75
        let (resp, _) = ev.evaluate("this is good sentiment").unwrap();
        let score: f64 = resp.parse().unwrap();
        assert!((score - 0.75).abs() < 0.01);
    }

    #[test]
    fn template_rule_substitutes_number() {
        let rule = Rule {
            id: "bpm-template".to_string(),
            description: "Template BPM".to_string(),
            trigger_keywords: vec!["bpm".to_string()],
            semantic_hint: None,
            formula: RuleFormula::Template {
                template: "BPM value: {n}".to_string(),
                slots: vec!["n".to_string()],
            },
            confidence: 0.8,
            examples: vec![],
            proposed_by: "operator".to_string(),
            approved: true,
        };
        let ev = RuleEvaluator::new(vec![rule]);
        let (resp, _) = ev.evaluate("the bpm is 140").unwrap();
        assert!(resp.contains("140.000"), "got: {resp}");
    }

    #[test]
    fn highest_confidence_rule_wins() {
        let low = Rule {
            id: "low".to_string(),
            description: "Low confidence".to_string(),
            trigger_keywords: vec!["bpm".to_string()],
            semantic_hint: None,
            formula: RuleFormula::Static { response: "low response".to_string() },
            confidence: 0.4,
            examples: vec![],
            proposed_by: "operator".to_string(),
            approved: true,
        };
        let high = Rule {
            id: "high".to_string(),
            description: "High confidence".to_string(),
            trigger_keywords: vec!["bpm".to_string()],
            semantic_hint: None,
            formula: RuleFormula::Static { response: "high response".to_string() },
            confidence: 0.9,
            examples: vec![],
            proposed_by: "operator".to_string(),
            approved: true,
        };
        let ev = RuleEvaluator::new(vec![low, high]);
        let (resp, _) = ev.evaluate("what is 120 bpm").unwrap();
        assert_eq!(resp, "high response");
    }

    #[test]
    fn keyword_score_all_matching() {
        let ev = evaluator_empty();
        let rule = bpm_rule(); // keywords: ["bpm", "bar"]
        let score = ev.keyword_score(&rule, "bpm and bar");
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn keyword_score_partial_matching() {
        let ev = evaluator_empty();
        let rule = bpm_rule(); // keywords: ["bpm", "bar"]
        let score = ev.keyword_score(&rule, "only bpm here");
        assert!((score - 0.5).abs() < 1e-10);
    }
}
