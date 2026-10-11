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
                (release.starts_with(|character: char| character.is_ascii_digit())
                    || matches!(*release, "p54rc1" | "p54rc2"))
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
                    | "5.003_21"
                    | "5.003_22"
                    | "5.003_23"
                    | "5.003_24"
                    | "5.003_25"
                    | "5.003_26"
                    | "5.003_27"
                    | "5.003_28"
                    | "5.003_90"
                    | "5.003_91"
                    | "5.003_92"
                    | "5.003_93"
                    | "5.003_94"
                    | "5.003_95"
                    | "5.003_96"
                    | "5.003_97"
                    | "5.003_97a"
                    | "5.003_97b"
                    | "5.003_97c"
                    | "5.003_97d"
                    | "5.003_97e"
                    | "5.003_97f"
                    | "5.003_97g"
                    | "5.003_97h"
                    | "5.003_97i"
                    | "5.003_97j"
                    | "5.003_98"
                    | "5.003_99"
                    | "5.003_99a"
                    | "p54rc1"
                    | "p54rc2"
                    | "5.004"
                    | "5.004_01-t2"
                    | "5.004_01"
                    | "5.004_01_01"
                    | "5.004_01_02"
                    | "5.004_01_03"
                    | "5.004_02"
                    | "5.004_02_01"
                    | "5.004_03-t2"
                    | "5.004_03"
                    | "5.004_04-t1"
                    | "5.004_04-t2"
                    | "5.004_04-t3"
                    | "5.004_04-t4"
                    | "5.004_04"
                    | "5.004_04-m1"
                    | "5.004_04-m2"
                    | "5.004_04-m3"
                    | "5.004_04-m4"
                    | "5.004_05-MT5"
                    | "5.004_05-MT6"
                    | "5.004_05-MT7"
                    | "5.004_05-MT8"
                    | "5.004_05-MT9"
                    | "5.004_05"
                    | "5.004_50"
                    | "5.004_51"
                    | "5.004_52"
                    | "5.004_53"
                    | "5.004_54"
                    | "5.004_55"
                    | "5.004_56"
                    | "5.004_57"
                    | "5.004_58"
                    | "5.004_59"
                    | "5.004_60"
                    | "5.004_61"
                    | "5.004_62"
                    | "5.004_63"
                    | "5.004_64"
                    | "5.004_65"
                    | "5.004_66"
                    | "5.004_67"
                    | "5.004_68"
                    | "5.004_69"
                    | "5.004_70"
                    | "5.004_71"
                    | "5.004_72"
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
            if matches!(
                *release,
                "5.003_26"
                    | "5.003_27"
                    | "5.003_28"
                    | "5.003_90"
                    | "5.003_91"
                    | "5.003_92"
                    | "5.003_93"
                    | "5.003_94"
                    | "5.003_95"
                    | "5.003_96"
                    | "5.003_97"
                    | "5.003_97a"
                    | "5.003_97b"
                    | "5.003_97c"
                    | "5.003_97d"
                    | "5.003_97e"
                    | "5.003_97f"
                    | "5.003_97g"
                    | "5.003_97h"
                    | "5.003_97i"
                    | "5.003_97j"
                    | "5.003_98"
                    | "5.003_99"
                    | "5.003_99a"
                    | "p54rc1"
                    | "p54rc2"
                    | "5.004"
                    | "5.004_01-t2"
                    | "5.004_01"
                    | "5.004_01_01"
                    | "5.004_01_02"
                    | "5.004_01_03"
                    | "5.004_02"
                    | "5.004_02_01"
                    | "5.004_03-t2"
                    | "5.004_03"
                    | "5.004_04-t1"
                    | "5.004_04-t2"
                    | "5.004_04-t3"
                    | "5.004_04-t4"
                    | "5.004_04"
                    | "5.004_04-m1"
                    | "5.004_04-m2"
                    | "5.004_04-m3"
                    | "5.004_04-m4"
                    | "5.004_05-MT5"
                    | "5.004_05-MT6"
                    | "5.004_05-MT7"
                    | "5.004_05-MT8"
                    | "5.004_05-MT9"
                    | "5.004_05"
                    | "5.004_50"
                    | "5.004_51"
                    | "5.004_52"
                    | "5.004_53"
                    | "5.004_54"
                    | "5.004_55"
                    | "5.004_56"
                    | "5.004_57"
                    | "5.004_58"
                    | "5.004_59"
                    | "5.004_60"
                    | "5.004_61"
                    | "5.004_62"
                    | "5.004_63"
                    | "5.004_64"
                    | "5.004_65"
                    | "5.004_66"
                    | "5.004_67"
                    | "5.004_68"
                    | "5.004_69"
                    | "5.004_70"
                    | "5.004_71"
            ) {
                let carriage_return = GrammarLexer::new("print(1);\r", &token_grammar).tokenize();
                assert!(
                    carriage_return.map_or(true, |tokens| {
                        GrammarParser::new(tokens, parser_grammar.clone())
                            .with_max_depth(MAX_RULE_DEPTH)
                            .parse()
                            .is_err()
                    }),
                    "{release}: carriage return accepted"
                );
            }
            if matches!(
                *release,
                "5.003_96"
                    | "5.003_97"
                    | "5.003_97a"
                    | "5.003_97b"
                    | "5.003_97c"
                    | "5.003_97d"
                    | "5.003_97e"
                    | "5.003_97f"
                    | "5.003_97g"
                    | "5.003_97h"
                    | "5.003_97i"
                    | "5.003_97j"
                    | "5.003_98"
                    | "5.003_99"
                    | "5.003_99a"
                    | "p54rc1"
                    | "p54rc2"
                    | "5.004"
                    | "5.004_01-t2"
                    | "5.004_01"
                    | "5.004_01_01"
                    | "5.004_01_02"
                    | "5.004_01_03"
                    | "5.004_02"
                    | "5.004_02_01"
                    | "5.004_03-t2"
                    | "5.004_03"
                    | "5.004_04-t1"
                    | "5.004_04-t2"
                    | "5.004_04-t3"
                    | "5.004_04-t4"
                    | "5.004_04"
                    | "5.004_04-m1"
                    | "5.004_04-m2"
                    | "5.004_04-m3"
                    | "5.004_04-m4"
                    | "5.004_05-MT5"
                    | "5.004_05-MT6"
                    | "5.004_05-MT7"
                    | "5.004_05-MT8"
                    | "5.004_05-MT9"
                    | "5.004_05"
                    | "5.004_50"
                    | "5.004_51"
                    | "5.004_52"
                    | "5.004_53"
                    | "5.004_54"
                    | "5.004_55"
                    | "5.004_56"
                    | "5.004_57"
                    | "5.004_58"
                    | "5.004_59"
                    | "5.004_60"
                    | "5.004_61"
                    | "5.004_62"
                    | "5.004_63"
                    | "5.004_64"
                    | "5.004_65"
                    | "5.004_66"
                    | "5.004_67"
                    | "5.004_68"
                    | "5.004_69"
                    | "5.004_70"
                    | "5.004_71"
                    | "5.004_72"
            ) {
                let unknown = GrammarLexer::new("print(1);\u{0001}", &token_grammar).tokenize();
                assert!(
                    unknown.map_or(true, |tokens| {
                        GrammarParser::new(tokens, parser_grammar.clone())
                            .with_max_depth(MAX_RULE_DEPTH)
                            .parse()
                            .is_err()
                    }),
                    "{release}: unrecognized character accepted"
                );
            }
            if matches!(
                *release,
                "5.003_97i"
                    | "5.003_97j"
                    | "5.003_98"
                    | "5.003_99"
                    | "5.003_99a"
                    | "p54rc1"
                    | "p54rc2"
                    | "5.004"
                    | "5.004_01-t2"
                    | "5.004_01"
                    | "5.004_01_01"
                    | "5.004_01_02"
                    | "5.004_01_03"
                    | "5.004_02"
                    | "5.004_02_01"
                    | "5.004_03-t2"
                    | "5.004_03"
                    | "5.004_04-t1"
                    | "5.004_04-t2"
                    | "5.004_04-t3"
                    | "5.004_04-t4"
                    | "5.004_04"
                    | "5.004_04-m1"
                    | "5.004_04-m2"
                    | "5.004_04-m3"
                    | "5.004_04-m4"
                    | "5.004_05-MT5"
                    | "5.004_05-MT6"
                    | "5.004_05-MT7"
                    | "5.004_05-MT8"
                    | "5.004_05-MT9"
                    | "5.004_05"
                    | "5.004_50"
                    | "5.004_51"
                    | "5.004_52"
                    | "5.004_53"
                    | "5.004_54"
                    | "5.004_55"
                    | "5.004_56"
                    | "5.004_57"
                    | "5.004_58"
                    | "5.004_59"
                    | "5.004_60"
                    | "5.004_61"
                    | "5.004_62"
                    | "5.004_63"
                    | "5.004_64"
                    | "5.004_65"
                    | "5.004_66"
                    | "5.004_67"
                    | "5.004_68"
                    | "5.004_69"
                    | "5.004_70"
                    | "5.004_71"
                    | "5.004_72"
            ) {
                let accepted = format!("print({});", "9".repeat(250));
                let tokens = GrammarLexer::new(&accepted, &token_grammar)
                    .tokenize()
                    .unwrap();
                assert!(
                    GrammarParser::new(tokens, parser_grammar.clone())
                        .with_max_depth(MAX_RULE_DEPTH)
                        .parse()
                        .is_ok(),
                    "{release}: 250-digit decimal rejected"
                );
                let too_long = format!("print({});", "9".repeat(251));
                let rejected = GrammarLexer::new(&too_long, &token_grammar)
                    .tokenize()
                    .map_or(true, |tokens| {
                        GrammarParser::new(tokens, parser_grammar.clone())
                            .with_max_depth(MAX_RULE_DEPTH)
                            .parse()
                            .is_err()
                    });
                assert!(rejected, "{release}: 251-digit decimal accepted");
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
    fn perl_5_004_55_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_55,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_55.tokens").is_file());
        assert!(directory.join("perl5.004_55.grammar").is_file());
    }

    #[test]
    fn perl_5_004_56_and_57_have_distinct_partial_pairs() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        for release in ["5.004_56", "5.004_57"] {
            assert!(
                inventory
                    .lines()
                    .any(|row| row.starts_with(&format!("{release},")) && row.ends_with(",partial")),
                "{release}: expected partial inventory row"
            );
            assert!(directory.join(format!("perl{release}.tokens")).is_file());
            assert!(directory.join(format!("perl{release}.grammar")).is_file());
        }
    }

    #[test]
    fn perl_5_004_58_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_58,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_58.tokens").is_file());
        assert!(directory.join("perl5.004_58.grammar").is_file());
    }

    #[test]
    fn perl_5_004_59_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_59,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_59.tokens").is_file());
        assert!(directory.join("perl5.004_59.grammar").is_file());
    }

    #[test]
    fn perl_5_004_60_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_60,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_60.tokens").is_file());
        assert!(directory.join("perl5.004_60.grammar").is_file());
    }

    #[test]
    fn perl_5_004_61_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_61,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_61.tokens").is_file());
        assert!(directory.join("perl5.004_61.grammar").is_file());
    }

    #[test]
    fn perl_5_004_62_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_62,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_62.tokens").is_file());
        assert!(directory.join("perl5.004_62.grammar").is_file());
    }

    #[test]
    fn perl_5_004_63_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_63,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_63.tokens").is_file());
        assert!(directory.join("perl5.004_63.grammar").is_file());
    }

    #[test]
    fn perl_5_004_64_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_64,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_64.tokens").is_file());
        assert!(directory.join("perl5.004_64.grammar").is_file());
    }

    #[test]
    fn perl_5_004_65_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_65,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_65.tokens").is_file());
        assert!(directory.join("perl5.004_65.grammar").is_file());
    }

    #[test]
    fn perl_5_004_66_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_66,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_66.tokens").is_file());
        assert!(directory.join("perl5.004_66.grammar").is_file());
    }

    #[test]
    fn perl_5_004_67_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_67,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_67.tokens").is_file());
        assert!(directory.join("perl5.004_67.grammar").is_file());
    }

    #[test]
    fn perl_5_004_68_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_68,") && row.ends_with(",partial")));
        assert!(directory.join("perl5.004_68.tokens").is_file());
        assert!(directory.join("perl5.004_68.grammar").is_file());
    }

    #[test]
    fn perl_5_004_69_has_its_own_partial_pair_without_constant_hooks() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_69,") && row.ends_with(",partial")));
        let token_text = std::fs::read_to_string(directory.join("perl5.004_69.tokens")).unwrap();
        let grammar_text = std::fs::read_to_string(directory.join("perl5.004_69.grammar")).unwrap();
        let token_grammar = parse_token_grammar(&token_text).unwrap();
        let parser_grammar = parse_parser_grammar(&grammar_text).unwrap();
        // This release adds hints-controlled constant hooks. The partial
        // standalone grammar cannot install or invoke those hooks.
        for source in [
            "use overload; print(1);",
            "BEGIN { print(1); } print(2);",
            "$^H = 4096; print(1);",
        ] {
            let rejected =
                GrammarLexer::new(source, &token_grammar)
                    .tokenize()
                    .map_or(true, |tokens| {
                        GrammarParser::new(tokens, parser_grammar.clone())
                            .with_max_depth(MAX_RULE_DEPTH)
                            .parse()
                            .is_err()
                    });
            assert!(
                rejected,
                "5.004_69: constant-hook context accepted: {source}"
            );
        }
    }

    #[test]
    fn perl_5_004_70_has_its_own_partial_pair_without_hooks_or_overrides() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_70,") && row.ends_with(",partial")));
        let token_text = std::fs::read_to_string(directory.join("perl5.004_70.tokens")).unwrap();
        let grammar_text = std::fs::read_to_string(directory.join("perl5.004_70.grammar")).unwrap();
        let token_grammar = parse_token_grammar(&token_text).unwrap();
        let parser_grammar = parse_parser_grammar(&grammar_text).unwrap();
        // The archive changes keyword override resolution and constant-hook
        // internals. A standalone partial grammar must not accept the contexts
        // that install these hooks or shadow the builtin print keyword.
        for source in [
            "use overload; print(1);",
            "BEGIN { print(1); } print(2);",
            "$^H = 4096; print(1);",
            "sub print { 1; } print(2);",
            "package CORE::GLOBAL; sub print { 1; } print(2);",
            "use subs 'print'; print(1);",
            "our $x; print(1);",
            "print(m/a/t);",
        ] {
            let rejected =
                GrammarLexer::new(source, &token_grammar)
                    .tokenize()
                    .map_or(true, |tokens| {
                        GrammarParser::new(tokens, parser_grammar.clone())
                            .with_max_depth(MAX_RULE_DEPTH)
                            .parse()
                            .is_err()
                    });
            assert!(rejected, "5.004_70: unsupported context accepted: {source}");
        }
    }

    #[test]
    fn perl_5_004_71_has_its_own_partial_pair_without_hooks_or_overrides() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_71,") && row.ends_with(",partial")));
        let token_text = std::fs::read_to_string(directory.join("perl5.004_71.tokens")).unwrap();
        let grammar_text = std::fs::read_to_string(directory.join("perl5.004_71.grammar")).unwrap();
        let token_grammar = parse_token_grammar(&token_text).unwrap();
        let parser_grammar = parse_parser_grammar(&grammar_text).unwrap();
        // This archive changes builtin-shadowing diagnostics, overload-stack
        // naming, and regex hooks. The standalone subset excludes every
        // context that installs hooks or replaces the builtin print keyword.
        for source in [
            "use overload; print(1);",
            "BEGIN { print(1); } print(2);",
            "$^H = 4096; print(1);",
            "sub print { 1; } print(2);",
            "package CORE::GLOBAL; sub print { 1; } print(2);",
            "use subs 'print'; print(1);",
            "our $x; print(1);",
            "print(m/a/);",
            "print(m/a/t);",
        ] {
            let rejected =
                GrammarLexer::new(source, &token_grammar)
                    .tokenize()
                    .map_or(true, |tokens| {
                        GrammarParser::new(tokens, parser_grammar.clone())
                            .with_max_depth(MAX_RULE_DEPTH)
                            .parse()
                            .is_err()
                    });
            assert!(rejected, "5.004_71: unsupported context accepted: {source}");
        }
    }

    #[test]
    fn perl_5_004_72_has_its_own_partial_pair_without_hooks_or_overrides() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let inventory = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        assert!(inventory
            .lines()
            .any(|row| row.starts_with("5.004_72,") && row.ends_with(",partial")));
        let token_text = std::fs::read_to_string(directory.join("perl5.004_72.tokens")).unwrap();
        let grammar_text = std::fs::read_to_string(directory.join("perl5.004_72.grammar")).unwrap();
        let token_grammar = parse_token_grammar(&token_text).unwrap();
        let parser_grammar = parse_parser_grammar(&grammar_text).unwrap();
        // TMP_CRLF_PATCH makes CR whitespace in this own-release tokenizer.
        // It cannot join integer tokens or admit string/heredoc contexts.
        for source in [
            "\rprint(1);\r",
            "print\r(1);",
            "print(1\r+\r2);",
            "print(1);\rprint(2);",
            "print(1);\r\nprint(2);",
            "print(1);\n\rprint(2);",
            "print(1-\r-2);",
        ] {
            let tokens = GrammarLexer::new(source, &token_grammar)
                .tokenize()
                .unwrap();
            assert!(
                GrammarParser::new(tokens, parser_grammar.clone())
                    .with_max_depth(MAX_RULE_DEPTH)
                    .parse()
                    .is_ok(),
                "5.004_72: supported CR whitespace rejected: {source:?}"
            );
        }
        for source in [
            "print(1\r2);",
            "pri\rnt(1);",
            "print(1--\r2);",
            "print(1);\u{0001}",
            "print(\"a\r\nb\");",
            "print(<<END);\r\na\r\nEND\r\n",
            "\ruse overload;\rprint(1);",
            "\rBEGIN { print(1); }\r",
            "use overload; print(1);",
            "BEGIN { print(1); } print(2);",
            "$^H = 4096; print(1);",
            "sub print { 1; } print(2);",
            "package CORE::GLOBAL; sub print { 1; } print(2);",
            "use subs 'print'; print(1);",
            "our $x; print(1);",
            "print(m/a/);",
            "print(m/a/t);",
        ] {
            let rejected =
                GrammarLexer::new(source, &token_grammar)
                    .tokenize()
                    .map_or(true, |tokens| {
                        GrammarParser::new(tokens, parser_grammar.clone())
                            .with_max_depth(MAX_RULE_DEPTH)
                            .parse()
                            .is_err()
                    });
            assert!(rejected, "5.004_72: unsupported context accepted: {source}");
        }
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
