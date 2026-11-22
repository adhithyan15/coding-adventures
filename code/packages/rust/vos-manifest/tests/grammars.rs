//! The two grammar pairs under `code/grammars` pass the repository's own validators,
//! and the engine behaves the way the grammars' comments say it does.  The comments
//! make claims about engine facts (first match wins, a skip entry that matches nothing
//! keeps leading blanks, a literal matches by token value); each claim is probed here,
//! so a change in the engine that would silently change the meaning of a grammar
//! fails a test instead.

use std::collections::HashSet;

use grammar_tools::cross_validator::cross_validate;
use grammar_tools::parser_grammar::{parse_parser_grammar, validate_parser_grammar};
use grammar_tools::token_grammar::{parse_token_grammar, validate_token_grammar};
use lexer::grammar_lexer::GrammarLexer;
use parser::grammar_parser::GrammarParser;
use vos_manifest::grammar::*;

fn issues(tokens: &str, grammar: &str) -> Vec<String> {
    let t = parse_token_grammar(tokens).expect("tokens parse");
    let g = parse_parser_grammar(grammar).expect("grammar parses");
    let mut out = validate_token_grammar(&t);
    let names: HashSet<String> = t.definitions.iter().map(|d| d.name.clone()).collect();
    out.extend(validate_parser_grammar(&g, Some(&names)));
    out.extend(cross_validate(&t, &g));
    out
}

#[test]
fn the_contract_pair_has_no_validation_issue() {
    assert_eq!(
        issues(CONTRACT_TOKENS_SRC, CONTRACT_GRAMMAR_SRC),
        Vec::<String>::new()
    );
}

#[test]
fn the_document_pair_has_exactly_one_intended_warning() {
    // FORGED exists to be refused: no rule of the parser grammar may mention it, so
    // that a line carrying a bad marker is a token the grammar has no place for.
    let got = issues(DOCUMENT_TOKENS_SRC, DOCUMENT_GRAMMAR_SRC);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].contains("'FORGED'") && got[0].contains("never used"),
        "{got:?}"
    );
}

fn kinds(src: &str, tokens: &grammar_tools::token_grammar::TokenGrammar) -> Vec<(String, String)> {
    GrammarLexer::new(src, tokens)
        .tokenize()
        .expect("lexes")
        .iter()
        .map(|t| (token_kind(t), t.value.clone()))
        .filter(|(k, _)| k != "EOF")
        .collect()
}

#[test]
fn the_contract_lexer_skips_single_spaces_and_refuses_everything_else() {
    let g = contract_tokens();
    let toks = kinds("ceiling dispatch  steps 160\n", g);
    let kinds_only: Vec<&str> = toks.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(kinds_only, ["NAME", "NAME", "NAME", "DEC", "NEWLINE"]);
    for bad in ["a\tb\n", "a\r\n", "a;\n", "a\u{e9}\n", "a=1\n"] {
        assert!(
            GrammarLexer::new(bad, g).tokenize().is_err(),
            "{bad:?} must not lex"
        );
    }
}

#[test]
fn a_dec_is_zero_or_has_no_leading_zero_and_at_most_twenty_digits() {
    let g = contract_tokens();
    let values = |s: &str| -> Vec<String> {
        kinds(s, g)
            .into_iter()
            .filter(|(k, _)| k == "DEC")
            .map(|(_, v)| v)
            .collect()
    };
    assert_eq!(values("0"), ["0"]);
    // `0123` is two DEC tokens, which no rule of the parser grammar accepts.
    assert_eq!(values("0123"), ["0", "123"]);
    // Twenty digits are one token; the twenty-first starts another.
    assert_eq!(values("12345678901234567890"), ["12345678901234567890"]);
    assert_eq!(
        values("123456789012345678901"),
        ["12345678901234567890", "1"]
    );
}

#[test]
fn a_grammar_literal_matches_a_name_token_by_value() {
    // `name` is a NAME token whose text is `name`; a package may itself be called `name`.
    let tokens = GrammarLexer::new(
        "vos-contract 1\nname name\niface name\nend\n",
        contract_tokens(),
    )
    .tokenize()
    .unwrap();
    let mut p = GrammarParser::new(tokens, contract_grammar().clone());
    assert!(p.parse().is_ok());
}

#[test]
fn the_contract_parser_refuses_trailing_rows_but_tolerates_a_blank_line() {
    // Two `end` rows: refused.  A blank line after `end`: ACCEPTED by the grammar (an
    // empty line is a newline token), which is why the canonical re-emit comparison exists.
    let parse = |s: &str| {
        let tokens = GrammarLexer::new(s, contract_tokens()).tokenize().unwrap();
        GrammarParser::new(tokens, contract_grammar().clone())
            .parse()
            .is_ok()
    };
    assert!(!parse("vos-contract 1\nname a\niface b\nend\nend\n"));
    assert!(parse("vos-contract 1\nname a\niface b\nend\n\n"));
    assert!(!parse("vos-contract 1\nname a\niface b\nend"));
}

#[test]
fn the_document_lexer_keeps_every_byte_and_the_first_matching_definition_wins() {
    let g = document_tokens();
    let h = "0".repeat(64);
    let exact = format!("  -- vos02:contract-begin sha256={h}\n");
    let toks = kinds(&exact, g);
    assert_eq!(toks[0].0, "CONTRACT_BEGIN");
    assert_eq!(
        toks[0].1,
        exact.trim_end_matches('\n'),
        "leading blanks are kept"
    );
    // The same text indented four spaces, extended, or buried after code is FORGED.
    for bad in [
        format!("    -- vos02:contract-begin sha256={h}\n"),
        format!("  -- vos02:contract-begin sha256={h} extra\n"),
        "  model x  -- vos02:impl-end\n".to_string(),
        "  example e: s == \"-- vos02:impl-end\"\n".to_string(),
        "  -- vos02:nonsense\n".to_string(),
        "  -- vos02:impl-begin x\n".to_string(),
    ] {
        let toks = kinds(&bad, g);
        // Not a lone exact marker: either a FORGED token, or an exact marker followed
        // by stray text (a LINE), which the parser grammar has no place for.
        let lone_marker = toks.len() == 2
            && toks[0].0 != "FORGED"
            && toks[0].0 != "LINE"
            && toks[1].0 == "NEWLINE";
        assert!(!lone_marker, "{bad:?} -> {toks:?}");
        assert!(
            toks.iter().any(|(k, _)| k == "FORGED" || k == "LINE"),
            "{bad:?} -> {toks:?}"
        );
    }
    // A plain line that merely resembles one is a LINE.
    assert_eq!(kinds("  -- a vos02 remark\n", g)[0].0, "LINE");
    assert_eq!(kinds("\n", g)[0].0, "NEWLINE");
}

#[test]
fn the_tests_that_read_the_contract_package_have_a_cargo_edge_to_it() {
    // VOS-D058: the build tool selects tests by Cargo dependency edges.  These suites read
    // the real vos-sched-contract files, so an edit to a .part or contract.vos must select
    // them; the edge is a dev-dependency.  Removing it silently blinds CI.
    let manifest = include_str!("../Cargo.toml");
    let dev = manifest
        .split("[dev-dependencies]")
        .nth(1)
        .expect("a dev-dependencies table");
    assert!(dev.contains("vos-sched-contract = { path = \"../vos-sched-contract\" }"));
}
