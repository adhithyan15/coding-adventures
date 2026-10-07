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
    use std::path::Path;

    #[test]
    fn every_inventoried_pair_is_valid_and_parses_a_small_program() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        let mut checked = 0;
        for row in inventory
            .lines()
            .filter(|row| !row.starts_with('#'))
            .skip(1)
        {
            let fields: Vec<_> = row.split(',').collect();
            let [release, _, _, status] = fields.as_slice() else {
                panic!("malformed Perl release row: {row}");
            };
            if *status == "pending" {
                continue;
            }
            assert!(matches!(*status, "partial" | "complete"), "{row}");
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
            for source in ["print(1);", "print(1+2*3);", "print(-4);"] {
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
                GrammarParser::new(tokens, parser_grammar)
                    .with_max_depth(MAX_RULE_DEPTH)
                    .parse()
                    .is_err(),
                "{release}: malformed print accepted"
            );
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
