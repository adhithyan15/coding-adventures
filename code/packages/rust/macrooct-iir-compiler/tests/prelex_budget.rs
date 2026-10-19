//! Pre-lex budgets: every source byte MacroOct hands to its lexer is charged
//! against the translation unit's bounds *before* the lexer runs.
//!
//! The engine's own `tokens_produced` and `bytes_per_file` checks only see
//! tokens a dialect has already produced, and the lexer materialises a whole
//! file at a time. Before these budgets, an 8 MB file of `;` cost about 2 GB
//! of memory before being rejected, and a 17 MB primary file over the 16 MB
//! per-file limit compiled. Each test below pins one route by which source
//! bytes reach the lexer: the primary file, an include, nested includes that
//! each fit alone but not together, and an unused member of the in-memory
//! bundle.

use coding_adventures_macrooct_iir_compiler::{
    compile_source, compile_source_with_includes, preprocess_source, MacroOctError,
};
use coding_adventures_source_preprocessor::{Bounds, MemoryFs};

fn preprocess_error(source: &str, extra: &[(&str, String)], bounds: Bounds) -> String {
    let mut fs = MemoryFs::new();
    for (name, text) in extra {
        fs.insert(*name, text.clone());
    }
    let file = fs.insert("<main>", source);
    match preprocess_source(source, file, &mut fs, bounds) {
        Err(error) => error.to_string(),
        Ok(_) => panic!("{source:?} must be rejected"),
    }
}

#[test]
fn primary_source_byte_budget_is_checked_before_lexing() {
    let error = preprocess_error(
        "@define X 7\n",
        &[],
        Bounds { bytes_per_file: 4, ..Bounds::default() },
    );
    assert!(error.contains("pre-lex byte budget"), "{error}");
}

#[test]
fn dense_primary_tokens_are_bounded_before_lexing() {
    // Five `;` would be five tokens; a five-token budget admits four bytes.
    let error = preprocess_error(
        ";;;;;",
        &[],
        Bounds { tokens_produced: 5, ..Bounds::default() },
    );
    assert!(error.contains("pre-lex byte budget"), "{error}");
}

#[test]
fn dense_included_tokens_are_bounded_before_lexing() {
    let error = preprocess_error(
        "@include \"x\"\n",
        &[("x", ";".repeat(20))],
        Bounds { tokens_produced: 20, ..Bounds::default() },
    );
    assert!(error.contains("pre-lex"), "{error}");
}

#[test]
fn nested_dense_includes_share_one_prelex_token_budget() {
    // Each file fits the per-file cap (79 bytes) alone; together the primary,
    // `a` and `b` lex 13 + 39 + 39 = 91 bytes, over the unit's 79.
    let error = preprocess_error(
        "@include \"a\"\n",
        &[
            ("a", format!("@include \"b\"\n{}", ";".repeat(26))),
            ("b", ";".repeat(39)),
        ],
        Bounds { tokens_produced: 80, ..Bounds::default() },
    );
    assert!(error.contains("aggregate pre-lex"), "{error}");
}

#[test]
fn oversized_primary_is_rejected_by_the_string_entry_point() {
    // Whitespace lexes to almost no tokens, so only a byte check stops it.
    let source = " ".repeat(Bounds::default().bytes_per_file as usize + 1);
    let error = compile_source(&source, "m").unwrap_err();
    assert!(matches!(error, MacroOctError::Preprocess(_)), "{error}");
    assert!(error.to_string().contains("byte budget"), "{error}");
}

#[test]
fn unused_oversized_include_is_rejected_before_memory_fs_insertion() {
    let oversized = "x".repeat(Bounds::default().bytes_per_file as usize + 1);
    let error = compile_source_with_includes(
        "fn main() { out(1, 42); }\n",
        "m",
        &[("unused.macrooct", &oversized)],
    )
    .unwrap_err();
    assert!(matches!(error, MacroOctError::Preprocess(_)), "{error}");
    assert!(error.to_string().contains("byte budget"), "{error}");
}

#[test]
fn programs_within_budget_still_compile_through_includes() {
    let module = compile_source_with_includes(
        "@include \"helper.macrooct\"\nfn main() { out(1, helper()); }\n",
        "m",
        &[("helper.macrooct", "fn helper() -> u8 { return 42; }")],
    )
    .expect("a small bundle stays within every budget");
    assert_eq!(module.functions.len(), 2);
}
