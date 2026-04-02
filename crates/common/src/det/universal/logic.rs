//! Boolean algebra and propositional logic — pure deterministic functions.

// ═══════════════════════════════════════
// Basic Logic Gates
// ═══════════════════════════════════════

#[must_use]
pub fn logical_and(a: bool, b: bool) -> bool { a && b }

#[must_use]
pub fn logical_or(a: bool, b: bool) -> bool { a || b }

#[must_use]
pub fn logical_xor(a: bool, b: bool) -> bool { a ^ b }

/// a → b  (material implication)
#[must_use]
pub fn logical_implies(a: bool, b: bool) -> bool { !a || b }

#[must_use]
pub fn logical_nand(a: bool, b: bool) -> bool { !(a && b) }

#[must_use]
pub fn logical_nor(a: bool, b: bool) -> bool { !(a || b) }

// ═══════════════════════════════════════
// Truth Table
// ═══════════════════════════════════════

/// Evaluate a boolean expression string over all 2^n variable combinations.
///
/// Supported operators (in decreasing precedence): `!`, `&&`, `||`, `->`, `<->`.
/// Variable names must match entries in `vars` exactly.
/// Returns rows in natural binary counting order (all-false first).
///
/// # Examples
/// ```
/// use nstn_common::det::logic::truth_table;
/// let rows = truth_table("a && b", &["a", "b"]);
/// assert_eq!(rows.len(), 4);
/// assert_eq!(rows[3], vec![true, true, true]); // last col = result
/// ```
#[must_use]
pub fn truth_table(expr: &str, vars: &[&str]) -> Vec<Vec<bool>> {
    let n = vars.len();
    let combos = 1usize << n;
    let mut result = Vec::with_capacity(combos);
    for i in 0..combos {
        let mut assignment = Vec::with_capacity(n);
        for bit in (0..n).rev() {
            assignment.push((i >> bit) & 1 == 1);
        }
        let val = eval_expr(expr, vars, &assignment);
        let mut row = assignment;
        row.push(val);
        result.push(row);
    }
    result
}

/// Tokenise and evaluate a simple propositional logic expression.
fn eval_expr(expr: &str, vars: &[&str], values: &[bool]) -> bool {
    let tokens = tokenize(expr.trim());
    let mut pos = 0;
    parse_biconditional(&tokens, &mut pos, vars, values)
}

fn parse_biconditional(tokens: &[String], pos: &mut usize, vars: &[&str], values: &[bool]) -> bool {
    let mut left = parse_implication(tokens, pos, vars, values);
    while *pos < tokens.len() && tokens[*pos] == "<->" {
        *pos += 1;
        let right = parse_implication(tokens, pos, vars, values);
        left = left == right;
    }
    left
}

fn parse_implication(tokens: &[String], pos: &mut usize, vars: &[&str], values: &[bool]) -> bool {
    let left = parse_or(tokens, pos, vars, values);
    if *pos < tokens.len() && tokens[*pos] == "->" {
        *pos += 1;
        let right = parse_implication(tokens, pos, vars, values); // right-associative
        logical_implies(left, right)
    } else {
        left
    }
}

fn parse_or(tokens: &[String], pos: &mut usize, vars: &[&str], values: &[bool]) -> bool {
    let mut left = parse_and(tokens, pos, vars, values);
    while *pos < tokens.len() && tokens[*pos] == "||" {
        *pos += 1;
        let right = parse_and(tokens, pos, vars, values);
        left = left || right;
    }
    left
}

fn parse_and(tokens: &[String], pos: &mut usize, vars: &[&str], values: &[bool]) -> bool {
    let mut left = parse_not(tokens, pos, vars, values);
    while *pos < tokens.len() && tokens[*pos] == "&&" {
        *pos += 1;
        let right = parse_not(tokens, pos, vars, values);
        left = left && right;
    }
    left
}

fn parse_not(tokens: &[String], pos: &mut usize, vars: &[&str], values: &[bool]) -> bool {
    if *pos < tokens.len() && tokens[*pos] == "!" {
        *pos += 1;
        return !parse_not(tokens, pos, vars, values);
    }
    parse_atom(tokens, pos, vars, values)
}

fn parse_atom(tokens: &[String], pos: &mut usize, vars: &[&str], values: &[bool]) -> bool {
    if *pos >= tokens.len() { return false; }
    let tok = &tokens[*pos];
    if tok == "(" {
        *pos += 1;
        let val = parse_biconditional(tokens, pos, vars, values);
        if *pos < tokens.len() && tokens[*pos] == ")" { *pos += 1; }
        val
    } else if tok == "true" {
        *pos += 1;
        true
    } else if tok == "false" {
        *pos += 1;
        false
    } else {
        let result = vars.iter().zip(values.iter())
            .find(|(&v, _)| v == tok)
            .map_or(false, |(_, &b)| b);
        *pos += 1;
        result
    }
}

fn tokenize(expr: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' => { i += 1; }
            '(' | ')' => { tokens.push(chars[i].to_string()); i += 1; }
            '!' => { tokens.push("!".to_string()); i += 1; }
            '&' if i + 1 < chars.len() && chars[i+1] == '&' => {
                tokens.push("&&".to_string()); i += 2;
            }
            '|' if i + 1 < chars.len() && chars[i+1] == '|' => {
                tokens.push("||".to_string()); i += 2;
            }
            '-' if i + 1 < chars.len() && chars[i+1] == '>' => {
                tokens.push("->".to_string()); i += 2;
            }
            '<' if i + 2 < chars.len() && chars[i+1] == '-' && chars[i+2] == '>' => {
                tokens.push("<->".to_string()); i += 3;
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut name = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    name.push(chars[i]); i += 1;
                }
                tokens.push(name);
            }
            _ => { i += 1; }
        }
    }
    tokens
}

// ═══════════════════════════════════════
// Set Operations (on sorted i64 slices)
// ═══════════════════════════════════════

#[must_use]
pub fn set_union(a: &[i64], b: &[i64]) -> Vec<i64> {
    let mut result = a.to_vec();
    for &x in b {
        if !result.contains(&x) { result.push(x); }
    }
    result.sort_unstable();
    result
}

#[must_use]
pub fn set_intersection(a: &[i64], b: &[i64]) -> Vec<i64> {
    let mut result: Vec<i64> = a.iter().filter(|x| b.contains(x)).copied().collect();
    result.sort_unstable();
    result
}

#[must_use]
pub fn set_difference(a: &[i64], b: &[i64]) -> Vec<i64> {
    let mut result: Vec<i64> = a.iter().filter(|x| !b.contains(x)).copied().collect();
    result.sort_unstable();
    result
}

/// Returns true if every element of `a` is in `b`.
#[must_use]
pub fn set_is_subset(a: &[i64], b: &[i64]) -> bool {
    a.iter().all(|x| b.contains(x))
}

// ═══════════════════════════════════════
// Combinatorics
// ═══════════════════════════════════════

/// Product of all set sizes — number of tuples in the Cartesian product.
#[must_use]
pub fn cartesian_product_count(set_sizes: &[usize]) -> usize {
    set_sizes.iter().product()
}

/// Size of the power set of a set with n elements: 2^n.
#[must_use]
pub fn powerset_size(n: usize) -> usize { 1 << n }

/// Binomial coefficient C(n, k).
#[must_use]
pub fn combinations(n: u64, k: u64) -> u64 {
    if k > n { return 0; }
    let k = k.min(n - k);
    let mut result = 1u64;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

/// Partial permutation P(n, k) = n! / (n-k)!
#[must_use]
pub fn permutations(n: u64, k: u64) -> u64 {
    if k > n { return 0; }
    (n - k + 1..=n).product()
}

/// Factorial n! Returns None on overflow.
#[must_use]
pub fn factorial(n: u64) -> Option<u64> {
    if n > 20 { return None; }
    Some((1..=n).product())
}

// ═══════════════════════════════════════
// Tests
// ═══════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_logical_and() {
        assert!( logical_and(true, true));
        assert!(!logical_and(true, false));
        assert!(!logical_and(false, false));
    }

    #[test] fn test_logical_or() {
        assert!( logical_or(true, false));
        assert!(!logical_or(false, false));
    }

    #[test] fn test_logical_xor() {
        assert!( logical_xor(true, false));
        assert!(!logical_xor(true, true));
        assert!(!logical_xor(false, false));
    }

    #[test] fn test_logical_implies() {
        assert!( logical_implies(false, false)); // F→F = T
        assert!( logical_implies(false, true));  // F→T = T
        assert!( logical_implies(true,  true));  // T→T = T
        assert!(!logical_implies(true,  false)); // T→F = F
    }

    #[test] fn test_logical_nand() {
        assert!(!logical_nand(true, true));
        assert!( logical_nand(true, false));
        assert!( logical_nand(false, false));
    }

    #[test] fn test_logical_nor() {
        assert!( logical_nor(false, false));
        assert!(!logical_nor(true, false));
        assert!(!logical_nor(true, true));
    }

    #[test] fn truth_table_and() {
        let rows = truth_table("a && b", &["a", "b"]);
        assert_eq!(rows.len(), 4);
        // FF=F, FT=F, TF=F, TT=T
        assert!(!rows[0][2]);
        assert!(!rows[1][2]);
        assert!(!rows[2][2]);
        assert!( rows[3][2]);
    }

    #[test] fn truth_table_xor() {
        let rows = truth_table("a && !b || !a && b", &["a", "b"]);
        // XOR via formula
        assert!(!rows[0][2]); // F xor F = F
        assert!( rows[1][2]); // F xor T = T
        assert!( rows[2][2]); // T xor F = T
        assert!(!rows[3][2]); // T xor T = F
    }

    #[test] fn test_set_union() {
        assert_eq!(set_union(&[1,2,3], &[2,3,4]), vec![1,2,3,4]);
        assert_eq!(set_union(&[], &[1]), vec![1]);
    }

    #[test] fn test_set_intersection() {
        assert_eq!(set_intersection(&[1,2,3], &[2,3,4]), vec![2,3]);
        assert_eq!(set_intersection(&[1], &[2]), Vec::<i64>::new());
    }

    #[test] fn test_set_difference() {
        assert_eq!(set_difference(&[1,2,3], &[2,3]), vec![1]);
        assert_eq!(set_difference(&[1,2], &[3,4]), vec![1,2]);
    }

    #[test] fn test_set_is_subset() {
        assert!( set_is_subset(&[1,2], &[1,2,3]));
        assert!(!set_is_subset(&[1,4], &[1,2,3]));
        assert!( set_is_subset(&[], &[1,2,3]));
    }

    #[test] fn test_cartesian_product_count() {
        assert_eq!(cartesian_product_count(&[2, 3, 4]), 24);
        assert_eq!(cartesian_product_count(&[]), 1);
    }

    #[test] fn test_powerset_size() {
        assert_eq!(powerset_size(0), 1);
        assert_eq!(powerset_size(3), 8);
        assert_eq!(powerset_size(10), 1024);
    }

    #[test] fn test_combinations() {
        assert_eq!(combinations(5, 2), 10);
        assert_eq!(combinations(10, 3), 120);
        assert_eq!(combinations(5, 0), 1);
        assert_eq!(combinations(5, 6), 0);
    }

    #[test] fn test_permutations() {
        assert_eq!(permutations(5, 2), 20);
        assert_eq!(permutations(5, 0), 1);
        assert_eq!(permutations(3, 3), 6);
        assert_eq!(permutations(2, 5), 0);
    }

    #[test] fn test_factorial() {
        assert_eq!(factorial(0), Some(1));
        assert_eq!(factorial(5), Some(120));
        assert_eq!(factorial(20), Some(2_432_902_008_176_640_000));
        assert_eq!(factorial(21), None);
    }
}
