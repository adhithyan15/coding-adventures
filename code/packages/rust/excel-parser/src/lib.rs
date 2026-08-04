//! Excel parser backed by compiled parser grammar and reference token normalization.

use coding_adventures_excel_lexer::tokenize_excel_formula;
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Recursion-depth cap for the Excel [`GrammarParser`] — see
/// [`GrammarParser::with_max_depth`] for why this guard exists at all (deep
/// recursion through `parse_rule` can overflow the *native* thread stack —
/// an uncatchable process abort — before this crate's own callers get a
/// chance to report anything). Before this constant was applied,
/// `create_excel_parser` never called `with_max_depth` at all, leaving
/// every caller (including this crate's own `parse_excel_formula`)
/// exposed to a native-stack-overflow DoS from adversarial deeply-nested
/// input (e.g. `=(((...1...)))`).
///
/// **Not the shared engine's bare default** (see `csharp-parser`'s own
/// identically-named constant for why a blind `DEFAULT_MAX_RULE_DEPTH`
/// (128) is unsafe-for-usability on a rich general-purpose grammar).
/// Measured directly instead (binary search over candidate
/// `with_max_depth` values against a fixed 5000-level adversarial
/// `=(((...1...)))` input — ordinary parenthesised grouping, via
/// `primary -> parenthesized_expression -> expression` — on a
/// default-~2MiB-stack worker thread in a debug build, no
/// `RUST_MIN_STACK` override or explicit `Builder::stack_size` present):
/// safe at **299**, crashes at **300**.
///
/// `MAX_RULE_DEPTH` is set to **200** — about 33% below that floor
/// (comparable margin to `derive-parser`'s own ~33%, `apl-parser`'s
/// ~26.5%, `j-parser`'s ~30%). Measured real-input headroom at `200`:
/// plain parenthesised nesting parses cleanly to at least 10 levels —
/// comfortably beyond ordinary hand-written nesting depth.
///
/// This is measured against only **one** of Excel's recursion shapes
/// (ordinary paren grouping) — a full audit would also cover nested
/// function calls (`SUM(SUM(SUM(...)))`) and nested parenthesised
/// reference expressions, the way `css-parser`/`toml-parser` measured
/// *every* shape in their own (much smaller) grammars. That fuller audit
/// is a tracked follow-up; this pass at minimum replaces an unmeasured,
/// silently-broken default with a properly-measured floor for the shape
/// most likely to bind.
const MAX_RULE_DEPTH: usize = 200;

fn previous_significant_token(tokens: &[Token], index: usize) -> Option<&Token> {
    let mut i = index;
    while i > 0 {
        i -= 1;
        if tokens[i].effective_type_name() != "SPACE" {
            return Some(&tokens[i]);
        }
    }
    None
}

fn next_significant_token(tokens: &[Token], index: usize) -> Option<&Token> {
    let mut i = index + 1;
    while i < tokens.len() {
        if tokens[i].effective_type_name() != "SPACE" {
            return Some(&tokens[i]);
        }
        i += 1;
    }
    None
}

fn normalize_excel_reference_tokens(tokens: Vec<Token>) -> Vec<Token> {
    let original = tokens.clone();
    tokens
        .into_iter()
        .enumerate()
        .map(|(index, token)| {
            let previous = previous_significant_token(&original, index);
            let next = next_significant_token(&original, index);
            let adjacent_to_colon = previous.map(|t| t.effective_type_name()) == Some("COLON")
                || next.map(|t| t.effective_type_name()) == Some("COLON");

            if token.effective_type_name() == "NAME" && adjacent_to_colon {
                return Token {
                    type_: TokenType::Name,
                    value: token.value,
                    line: token.line,
                    column: token.column,
                    type_name: Some("COLUMN_REF".to_string()),
                    flags: None, cv: None,
                };
            }

            if token.effective_type_name() == "NUMBER" && adjacent_to_colon {
                return Token {
                    type_: TokenType::Name,
                    value: token.value,
                    line: token.line,
                    column: token.column,
                    type_name: Some("ROW_REF".to_string()),
                    flags: None, cv: None,
                };
            }

            token
        })
        .collect()
}

pub fn create_excel_parser(source: &str) -> GrammarParser {
    let tokens = tokenize_excel_formula(source);
    let grammar = _grammar::parser_grammar();

    let mut parser = GrammarParser::new(tokens, grammar).with_max_depth(MAX_RULE_DEPTH);
    parser.add_pre_parse(Box::new(normalize_excel_reference_tokens));
    parser
}

pub fn parse_excel_formula(source: &str) -> GrammarASTNode {
    let mut parser = create_excel_parser(source);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Excel parse failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function_formula() {
        assert_eq!(parse_excel_formula("=SUM(A1:B2)").rule_name, "formula");
    }

    #[test]
    fn test_parse_column_range() {
        assert_eq!(parse_excel_formula("A:C").rule_name, "formula");
    }

    #[test]
    fn test_parse_row_range() {
        assert_eq!(parse_excel_formula("1:3").rule_name, "formula");
    }

    #[test]
    fn test_factory_exists() {
        let mut parser = create_excel_parser("A1");
        assert_eq!(parser.parse().expect("parse").rule_name, "formula");
    }

    // -------------------------------------------------------------------
    // Recursion-depth guard (DoS hardening) -- see MAX_RULE_DEPTH's own
    // doc comment for the measurement.
    // -------------------------------------------------------------------

    fn nested_paren_source(n: usize) -> String {
        format!("={}1{}", "(".repeat(n), ")".repeat(n))
    }

    /// Deeply-nested input must not overflow the native stack on a
    /// default-stack thread -- the whole point of the guard.
    #[test]
    fn test_deeply_nested_input_does_not_overflow_on_default_stack() {
        let src = nested_paren_source(5000);
        let handle = std::thread::spawn(move || {
            let mut parser = create_excel_parser(&src);
            let _ = parser.parse();
        });
        handle
            .join()
            .expect("MAX_RULE_DEPTH must trip BEFORE native overflow on the default stack");
    }

    /// Reasonable, hand-writable nesting stays well under the cap.
    #[test]
    fn test_reasonable_nesting_stays_under_the_cap() {
        let mut parser = create_excel_parser(&nested_paren_source(10));
        assert!(parser.parse().is_ok());
    }
}

