//! Grammar-driven Perl-language parser for LANG81's bounded print pilot.

use coding_adventures_perl_lexer::tokenize_perl;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

// The tiny pilot grammar has bounded source size and uses a guarded parser.
const MAX_RULE_DEPTH: usize = 96;

/// Parse the accepted Perl subset, rejecting trailing or malformed syntax.
pub fn parse_perl(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = tokenize_perl(source)?;
    GrammarParser::new(tokens, _grammar::parser_grammar())
        .with_max_depth(MAX_RULE_DEPTH)
        .parse()
        .map_err(|error| format!("Perl parse failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammar_tools::{
        cross_validator::cross_validate, parser_grammar::parse_parser_grammar,
        token_grammar::parse_token_grammar,
    };
    use lexer::grammar_lexer::GrammarLexer;
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn every_inventoried_pair_is_valid_and_parses_a_small_program() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        let mut checked = 0;
        let mut releases = HashSet::new();
        for row in inventory
            .lines()
            .filter(|row| !row.starts_with('#'))
            .skip(1)
        {
            let fields: Vec<_> = row.split(',').collect();
            let [release, _, _, status] = fields.as_slice() else {
                panic!("malformed Perl release row: {row}");
            };
            assert!(
                release.starts_with(|character: char| character.is_ascii_digit())
                    && release
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric()
                            || "._-".contains(character))
                    && !release.contains(".."),
                "unsafe Perl release ID: {release}"
            );
            assert!(
                releases.insert(*release),
                "duplicate Perl release ID: {release}"
            );
            assert!(
                matches!(*status, "pending" | "partial" | "complete"),
                "{row}"
            );
            if *status == "pending" {
                continue;
            }
            let token_text =
                std::fs::read_to_string(directory.join(format!("perl{release}.tokens"))).unwrap();
            let grammar_text =
                std::fs::read_to_string(directory.join(format!("perl{release}.grammar"))).unwrap();
            let token_grammar = parse_token_grammar(&token_text).unwrap();
            let parser_grammar = parse_parser_grammar(&grammar_text).unwrap();
            let issues = cross_validate(&token_grammar, &parser_grammar);
            assert!(
                issues.iter().all(|issue| !issue.starts_with("Error:")),
                "{release}: {issues:?}"
            );
            for source in [
                "print(0);",
                "print(10);",
                "print(1);",
                "print(1+2*3);",
                "print(-4);",
            ] {
                let tokens = GrammarLexer::new(source, &token_grammar)
                    .tokenize()
                    .unwrap();
                assert!(
                    GrammarParser::new(tokens, parser_grammar.clone())
                        .with_max_depth(MAX_RULE_DEPTH)
                        .parse()
                        .is_ok(),
                    "{release}: {source}"
                );
            }
            let tokens = GrammarLexer::new("print(", &token_grammar)
                .tokenize()
                .unwrap();
            assert!(
                GrammarParser::new(tokens, parser_grammar.clone())
                    .with_max_depth(MAX_RULE_DEPTH)
                    .parse()
                    .is_err(),
                "{release}: malformed print accepted"
            );
            let has_decrement = token_grammar
                .definitions
                .iter()
                .any(|definition| definition.name == "DECREMENT");
            assert!(has_decrement, "{release}: missing decrement token");
            let adjacent = GrammarLexer::new("print(1--2);", &token_grammar)
                .tokenize()
                .unwrap();
            assert!(
                GrammarParser::new(adjacent, parser_grammar.clone())
                    .with_max_depth(MAX_RULE_DEPTH)
                    .parse()
                    .is_err(),
                "{release}: decrement parsed as two minus operators"
            );
            let spaced = GrammarLexer::new("print(1- -2);", &token_grammar)
                .tokenize()
                .unwrap();
            assert!(
                GrammarParser::new(spaced, parser_grammar.clone())
                    .with_max_depth(MAX_RULE_DEPTH)
                    .parse()
                    .is_ok(),
                "{release}: spaced minus operators should parse"
            );
            if matches!(
                *release,
                "1.000"
                    | "1.0.15"
                    | "1.0_16"
                    | "2.000"
                    | "2.001"
                    | "3.000"
                    | "3.044"
                    | "4.000"
                    | "4.036"
                    | "5.000"
                    | "5.001"
                    | "5.001n"
                    | "5.002"
                    | "5.002_01"
                    | "5.003"
                    | "5.003_01"
                    | "5.003_02"
                    | "5.003_03"
                    | "5.003_04"
                    | "5.003_05"
                    | "5.003_06"
                    | "5.003_07"
                    | "5.003_08"
                    | "5.003_09"
                    | "5.003_10"
                    | "5.003_11"
                    | "5.003_12"
                    | "5.003_13"
                    | "5.003_14"
                    | "5.003_15"
                    | "5.003_16"
                    | "5.003_17"
                    | "5.003_18"
                    | "5.003_19"
                    | "5.003_20"
                    | "5.38.2"
                    | "5.44.0"
                    | "5.45.3"
            ) {
                for source in ["print(08);", "print(09);", "print(012);"] {
                    let tokens = GrammarLexer::new(source, &token_grammar)
                        .tokenize()
                        .unwrap();
                    assert!(
                        GrammarParser::new(tokens, parser_grammar.clone())
                            .with_max_depth(MAX_RULE_DEPTH)
                            .parse()
                            .is_err(),
                        "{release}: leading-zero literal accepted: {source}"
                    );
                }
            }
            checked += 1;
        }
        assert!(checked >= 13, "release pair coverage unexpectedly shrank");
    }

    #[test]
    fn parses_print_expression_with_precedence() {
        let tree = parse_perl("print(7 - 2 * 3);").unwrap();
        assert_eq!(tree.rule_name, "program");
    }

    #[test]
    fn rejects_octal_shaped_source_before_lowering() {
        for source in ["print(08);", "print(09);", "print(010);"] {
            assert!(parse_perl(source).is_err(), "{source}");
        }
        for source in ["print(0);", "print(10);"] {
            assert!(parse_perl(source).is_ok(), "{source}");
        }
    }

    #[test]
    fn rejects_unsupported_and_trailing_tokens() {
        for source in ["print(1); x", "say(1);", "print(1 / 2);", "print(1, 2);"] {
            assert!(parse_perl(source).is_err(), "{source}");
        }
    }

    #[test]
    fn deep_parentheses_fail_without_stack_overflow() {
        let source = format!("print({}1{});", "(".repeat(1000), ")".repeat(1000));
        assert!(parse_perl(&source).is_err());
    }
}
