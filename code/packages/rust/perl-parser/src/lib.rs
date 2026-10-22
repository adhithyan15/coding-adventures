//! Grammar-driven Perl-language parser.
//!
//! Like `perl-lexer`, this crate owns compiled grammars, not parsing code:
//! the generic [`GrammarParser`] runs over a [`ParserGrammar`] generated from
//! `code/grammars/perl/` by `grammar-tools generate-rust-compiled-grammars
//! perl`. [`release_parser_grammar`] returns the grammar of exactly one
//! inventory release (LANG84); [`parse_perl`] keeps using the unversioned
//! LANG81 execution grammar (`perl.grammar`).

use coding_adventures_perl_lexer::tokenize_perl;
use grammar_tools::parser_grammar::ParserGrammar;
use parser::grammar_parser::{GrammarASTNode, GrammarParser};

mod _grammar;

/// Every release in `code/grammars/perl/releases.csv` as `(release,
/// grammar_stage)`, pending rows included, in canonical inventory order. A
/// release here but not in [`SUPPORTED_RELEASES`] is known but has no pair
/// yet (stage `none`); a release in neither is unknown.
pub use _grammar::INVENTORY;

/// Every release with a compiled parser grammar, in inventory order.
pub use _grammar::SUPPORTED_RELEASES;

/// The deduplicated module (`g_<12 hex>`) that holds a release's parser
/// table, or `None` for a release without one.
pub use _grammar::grammar_class;

/// The parser's rule-depth cap. `GrammarParser`'s default is `usize::MAX`,
/// so every Perl parse sets this explicitly (LANG84 C5). The bounded grammar
/// nests one `unary` per `-`, so a deep `- - - … 1` reaches this cap and is
/// refused with an error rather than overflowing the stack.
pub const MAX_RULE_DEPTH: usize = 96;

/// The compiled parser grammar of exactly one inventory release, or `None`
/// when the release is pending or unknown. There is no fallback.
pub fn release_parser_grammar(release: &str) -> Option<ParserGrammar> {
    _grammar::parser_grammar(release)
}

/// The unversioned LANG81 parser grammar (`perl.grammar`).
pub fn unversioned_parser_grammar() -> ParserGrammar {
    _grammar::unversioned_parser_grammar()
}

/// Parse the accepted Perl subset with the unversioned LANG81 grammar,
/// rejecting trailing or malformed syntax.
pub fn parse_perl(source: &str) -> Result<GrammarASTNode, String> {
    let tokens = tokenize_perl(source)?;
    GrammarParser::new(tokens, unversioned_parser_grammar())
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
    use std::collections::{HashMap, HashSet};
    use std::path::Path;

    /// `code/grammars/perl/releases.csv` read by header NAME, one map per
    /// row from column to cell. Since LANG84 phase 2b the inventory has the
    /// common header plus `perlhist_line` and `lineage`, so no test may read
    /// a column by position or match a row by its ending (lesson: "read the
    /// inventory header before grouping release statuses"). Every row must
    /// have exactly the header's field count, so a shifted row cannot pass as
    /// another column.
    fn inventory_rows() -> Vec<HashMap<String, String>> {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let text = std::fs::read_to_string(directory.join("releases.csv")).unwrap();
        let mut lines = text.lines().filter(|line| !line.starts_with('#'));
        let header: Vec<&str> = lines.next().expect("inventory header").split(',').collect();
        lines
            .map(|line| {
                let fields: Vec<&str> = line.split(',').collect();
                assert_eq!(
                    fields.len(),
                    header.len(),
                    "malformed Perl release row: {line}"
                );
                header
                    .iter()
                    .zip(fields)
                    .map(|(column, cell)| (column.to_string(), cell.to_string()))
                    .collect()
            })
            .collect()
    }

    /// A LANG82 `partial` row as LANG84 phase 2b migrated it: status
    /// `partial`, measured stage `P`, covering the `pl-decimal-print` subset.
    fn assert_partial_row(release: &str) {
        let rows = inventory_rows();
        let row = rows
            .iter()
            .find(|row| row["release"] == release)
            .unwrap_or_else(|| panic!("{release}: no inventory row"));
        assert_eq!(row["grammar_status"], "partial", "{release}");
        assert_eq!(row["grammar_stage"], "P", "{release}");
        assert_eq!(row["grammar_subset"], "pl-decimal-print", "{release}");
    }

    #[test]
    fn inventory_rows_use_the_lang84_header() {
        let rows = inventory_rows();
        let columns: HashSet<&str> = rows[0].keys().map(String::as_str).collect();
        for column in [
            "release",
            "date",
            "channel",
            "source_ref",
            "source_sha",
            "predecessor",
            "grammar_status",
            "grammar_stage",
            "grammar_subset",
            "iir_status",
            "backend_status",
            "profile",
            "perlhist_line",
            "lineage",
        ] {
            assert!(columns.contains(column), "missing column {column}");
        }
        assert_eq!(columns.len(), 14);
        // Every compiled release is a migrated partial row, and only those.
        for row in &rows {
            let compiled = SUPPORTED_RELEASES.contains(&row["release"].as_str());
            assert_eq!(compiled, row["grammar_stage"] == "P", "{}", row["release"]);
        }
        for release in SUPPORTED_RELEASES {
            assert_partial_row(release);
        }
    }

    #[test]
    fn every_inventoried_pair_is_valid_and_parses_a_small_program() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let mut checked = 0;
        let mut releases = HashSet::new();
        let rows = inventory_rows();
        for row in &rows {
            let (release, status) = (&row["release"].as_str(), &row["grammar_status"].as_str());
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
                "{release}: {status}"
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
                    | "5.38.0"
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
                    | "5.38.0"
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
                    | "5.38.0"
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
        assert_partial_row("5.004_55");
        assert!(directory.join("perl5.004_55.tokens").is_file());
        assert!(directory.join("perl5.004_55.grammar").is_file());
    }

    #[test]
    fn perl_5_004_56_and_57_have_distinct_partial_pairs() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        for release in ["5.004_56", "5.004_57"] {
            assert_partial_row(release);
            assert!(directory.join(format!("perl{release}.tokens")).is_file());
            assert!(directory.join(format!("perl{release}.grammar")).is_file());
        }
    }

    #[test]
    fn perl_5_004_58_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_58");
        assert!(directory.join("perl5.004_58.tokens").is_file());
        assert!(directory.join("perl5.004_58.grammar").is_file());
    }

    #[test]
    fn perl_5_004_59_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_59");
        assert!(directory.join("perl5.004_59.tokens").is_file());
        assert!(directory.join("perl5.004_59.grammar").is_file());
    }

    #[test]
    fn perl_5_004_60_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_60");
        assert!(directory.join("perl5.004_60.tokens").is_file());
        assert!(directory.join("perl5.004_60.grammar").is_file());
    }

    #[test]
    fn perl_5_004_61_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_61");
        assert!(directory.join("perl5.004_61.tokens").is_file());
        assert!(directory.join("perl5.004_61.grammar").is_file());
    }

    #[test]
    fn perl_5_004_62_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_62");
        assert!(directory.join("perl5.004_62.tokens").is_file());
        assert!(directory.join("perl5.004_62.grammar").is_file());
    }

    #[test]
    fn perl_5_004_63_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_63");
        assert!(directory.join("perl5.004_63.tokens").is_file());
        assert!(directory.join("perl5.004_63.grammar").is_file());
    }

    #[test]
    fn perl_5_004_64_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_64");
        assert!(directory.join("perl5.004_64.tokens").is_file());
        assert!(directory.join("perl5.004_64.grammar").is_file());
    }

    #[test]
    fn perl_5_004_65_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_65");
        assert!(directory.join("perl5.004_65.tokens").is_file());
        assert!(directory.join("perl5.004_65.grammar").is_file());
    }

    #[test]
    fn perl_5_004_66_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_66");
        assert!(directory.join("perl5.004_66.tokens").is_file());
        assert!(directory.join("perl5.004_66.grammar").is_file());
    }

    #[test]
    fn perl_5_004_67_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_67");
        assert!(directory.join("perl5.004_67.tokens").is_file());
        assert!(directory.join("perl5.004_67.grammar").is_file());
    }

    #[test]
    fn perl_5_004_68_has_its_own_partial_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_68");
        assert!(directory.join("perl5.004_68.tokens").is_file());
        assert!(directory.join("perl5.004_68.grammar").is_file());
    }

    #[test]
    fn perl_5_004_69_has_its_own_partial_pair_without_constant_hooks() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.004_69");
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
        assert_partial_row("5.004_70");
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
        assert_partial_row("5.004_71");
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
        assert_partial_row("5.004_72");
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

    /// LANG84 phase 4b: the default release's own pair, grounded in the
    /// `v5.38.0` tag's `perly.y`, `toke.c`, `keywords.h` and `parser.h`.
    #[test]
    fn perl_5_38_0_has_its_own_partial_pair_grounded_in_its_tag() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        assert_partial_row("5.38.0");
        let token_text = std::fs::read_to_string(directory.join("perl5.38.0.tokens")).unwrap();
        let grammar_text = std::fs::read_to_string(directory.join("perl5.38.0.grammar")).unwrap();
        // The headers cite the tag and the exact blobs the rules come from.
        for text in [&token_text, &grammar_text] {
            for evidence in [
                "https://github.com/Perl/perl5/tree/v5.38.0",
                "f47267bec5921b10a8cc16545c9fdf11972c5f2d",
                "c0c09092c5e9305af5638535effebf18ea62cfe3",
                "1f81b3614c9a186468098cb15216c1dfe7ab3f2c",
                "feae6f6dce7ac861ac1ca207c097e51209a2191b",
                "46d6a19c7b1e8638c26ceaa8b30dfa3e75b8f62c",
                "PARTIAL",
            ] {
                assert!(text.contains(evidence), "5.38.0 pair lacks {evidence}");
            }
        }
        // The compiled tables are the ones the frontend uses by default.
        let token_grammar = coding_adventures_perl_lexer::release_token_grammar("5.38.0").unwrap();
        let parser_grammar = release_parser_grammar("5.38.0").unwrap();
        let parses = |source: &str| {
            GrammarLexer::new(source, &token_grammar)
                .tokenize()
                .is_ok_and(|tokens| {
                    GrammarParser::new(tokens, parser_grammar.clone())
                        .with_max_depth(MAX_RULE_DEPTH)
                        .parse()
                        .is_ok()
                })
        };
        // CR is whitespace: yylex falls through to the space case unless
        // PERL_STRICT_CR is defined, which a default build does not do.
        for source in [
            "print(1);\r\nprint(2);\r\n",
            "print\r(1\r+\r2);",
            "print (1);",
            "print(1- -2);",
            "print((1 + 2) * -3);",
        ] {
            assert!(parses(source), "5.38.0: supported form rejected: {source:?}");
        }
        for source in [
            // scan_num: a leading 0 is octal (and 8/9 are illegal there),
            // `_` separators, fractions, exponents and radix prefixes are
            // all real 5.38.0 syntax outside this subset.
            "print(012);",
            "print(08);",
            "print(1_000);",
            "print(1.5);",
            "print(1e3);",
            "print(0x1F);",
            "print(0b101);",
            "print(0o17);",
            // FF and VT are whitespace upstream; this subset excludes them.
            "print(1)\x0c;",
            "print(1)\x0b;",
            // print without parentheses, a list, a filehandle, other ops.
            "print 1;",
            "print(1, 2);",
            "print(STDOUT 1);",
            "print(+1);",
            "print(2 ** 3);",
            "print(7 / 2);",
            "print(7 % 2);",
            "printf(1);",
            "say(1);",
            // Comments, POD and the decrement token.
            "print(1); # one",
            "=pod\n\n=cut\nprint(1);",
            "print(1);\n__END__\n",
            "print(1--2);",
            "print(--1);",
            "use v5.38; print(1);",
            "BEGIN { print(1); }",
        ] {
            assert!(!parses(source), "5.38.0: unsupported form accepted: {source:?}");
        }
        // toke.c scan_num: tokenbuf[256] minus six reserved bytes.
        assert!(parses(&format!("print({});", "9".repeat(250))));
        assert!(!parses(&format!("print({});", "9".repeat(251))));
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

    /// Zero the one field deduplication may disagree on: a release shares the
    /// FIRST release's module in its class, so its line numbers belong there.
    fn without_line_numbers(mut grammar: ParserGrammar) -> ParserGrammar {
        for rule in &mut grammar.rules {
            rule.line_number = 0;
        }
        grammar
    }

    /// The LANG84 regeneration check for the compiled parser tables: the
    /// compiled release list is the inventory's non-pending rows, and each
    /// compiled grammar equals its own `.grammar` file parsed fresh.
    #[test]
    fn compiled_tables_match_every_inventoried_pair() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl");
        let rows = inventory_rows();
        // INVENTORY is every row as (release, grammar_stage), read from the
        // migrated inventory's own `grammar_stage` column.
        let all: Vec<(&str, &str)> = rows
            .iter()
            .map(|row| (row["release"].as_str(), row["grammar_stage"].as_str()))
            .collect();
        let compiled: Vec<&str> = rows
            .iter()
            .filter(|row| row["grammar_status"] != "pending")
            .map(|row| row["release"].as_str())
            .collect();
        assert_eq!(INVENTORY, all.as_slice());
        assert_eq!(SUPPORTED_RELEASES, compiled.as_slice());
        assert_eq!(
            SUPPORTED_RELEASES,
            coding_adventures_perl_lexer::SUPPORTED_RELEASES,
            "lexer and parser tables must cover the same releases"
        );
        for release in SUPPORTED_RELEASES {
            let text =
                std::fs::read_to_string(directory.join(format!("perl{release}.grammar"))).unwrap();
            assert_eq!(
                without_line_numbers(release_parser_grammar(release).unwrap()),
                without_line_numbers(parse_parser_grammar(&text).unwrap()),
                "{release}: compiled table is stale"
            );
        }
        for (release, _) in INVENTORY {
            assert_eq!(
                grammar_class(release).is_some(),
                SUPPORTED_RELEASES.contains(release)
            );
            assert_eq!(
                release_parser_grammar(release).is_some(),
                SUPPORTED_RELEASES.contains(release),
                "{release}: a pending release must have no table"
            );
        }
        let text = std::fs::read_to_string(directory.join("perl.grammar")).unwrap();
        assert_eq!(
            without_line_numbers(unversioned_parser_grammar()),
            without_line_numbers(parse_parser_grammar(&text).unwrap())
        );
    }

    /// Every compiled release pair, driven through the compiled tables rather
    /// than files read from disk, accepts the shared subset probe and refuses
    /// a malformed one.
    #[test]
    fn every_compiled_release_parses_the_shared_probe() {
        for release in SUPPORTED_RELEASES {
            let tokens = coding_adventures_perl_lexer::release_token_grammar(release).unwrap();
            let grammar = release_parser_grammar(release).unwrap();
            for (source, accepted) in [("print(1+2*3); print(-4);", true), ("print(", false)] {
                let tokens = GrammarLexer::new(source, &tokens).tokenize().unwrap();
                let parsed = GrammarParser::new(tokens, grammar.clone())
                    .with_max_depth(MAX_RULE_DEPTH)
                    .parse();
                assert_eq!(parsed.is_ok(), accepted, "{release}: {source}");
            }
        }
    }
}
