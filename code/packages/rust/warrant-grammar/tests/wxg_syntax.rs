//! VER04k section 4.4 and 4.11 (syntax): the `inout` parameter mode and the
//! `ensures` declaration of WX-G.
//!
//! Each case is authored with its own expected verdict.  The version gating
//! lives in the hand-written parser (the grammar-driven corpus runner cannot
//! express W0101, see `wx_syntax.rs`), so these tests drive the real parser.

use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

/// The parser's verdict on `src`: `None` when it parses, else the stable code.
fn code(src: &str) -> Option<&'static str> {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    parse_document(&layout, &doc, ParserLimits::default())
        .err()
        .map(|e| e.code())
}

fn accepts(src: &str) {
    assert_eq!(code(src), None, "{src}");
}

fn rejects(src: &str, want: &str) {
    assert_eq!(code(src), Some(want), "{src}");
}

fn op_doc(header: &str, param_type: &str) -> String {
    format!("warrant t{header}\n  op f(x: inout {param_type}) -> U8\n")
}

#[test]
fn inout_precedes_every_admissible_type() {
    for ty in [
        "Bool", "U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64", "Nat", "Bytes", "Bytes<4>",
        "List<U8>", "List<Bytes>", "Rec",
    ] {
        accepts(&op_doc(" v1", ty));
    }
    // More than one inout parameter, mixed with ordinary ones and a refinement.
    accepts("warrant t v1\n  op f(a: U8, b: inout Bytes, c: inout Nat | c > 1) -> (Nat, Bytes, Nat)\n");
}

#[test]
fn inout_alone_is_still_a_type_alias_named_inout() {
    // VER04k section 4.4: a mode only when immediately followed by a type.
    accepts("warrant t v1\n  model inout = U8\n  op f(x: inout) -> U8\n");
    accepts("warrant t v1\n  model inout = U8\n  op f(x: inout, y: U8) -> U8\n");
    accepts("warrant t v1\n  model inout = U8\n  op f(x: inout | x > 1u8) -> U8\n");
}

#[test]
fn inout_is_w0101_in_a_v0_document_and_accepted_under_v1() {
    let text = |header: &str| format!("warrant t{header}\n  op zeroize(buf: inout Bytes) -> Bytes\n");
    rejects(&text(""), "W0101");
    accepts(&text(" v1"));
}

#[test]
fn inout_is_not_a_model_parameter_or_a_quantifier_binder() {
    rejects("warrant t v1\n  model m(x: inout Bytes) -> Bytes = x\n", "W0001");
    rejects(
        "warrant t v1\n  op f(x: Bytes) -> Bool\n  claim c:\n    forall b: inout Bytes. f(b) == f(b)\n",
        "W0001",
    );
}

#[test]
fn ensures_declaration_with_one_and_thirty_two_rows() {
    accepts("warrant t v1\n  op f(x: Bytes) -> Bytes\n  ensures f:\n    len(result) == len(x)\n");
    let rows: String = (0..32).map(|_| "    len(result) == len(x)\n").collect();
    accepts(&format!("warrant t v1\n  op f(x: Bytes) -> Bytes\n  ensures f:\n{rows}"));
    // The 32-row cap is the checker's (W0602); the parser accepts 33.
    let rows: String = (0..33).map(|_| "    len(result) == len(x)\n").collect();
    accepts(&format!("warrant t v1\n  op f(x: Bytes) -> Bytes\n  ensures f:\n{rows}"));
}

#[test]
fn ensures_at_declaration_level_is_w0101_in_a_v0_document() {
    let text = |header: &str| {
        format!("warrant t{header}\n  op f(x: Bytes) -> Bytes\n  ensures f:\n    len(result) == len(x)\n")
    };
    rejects(&text(""), "W0101");
    accepts(&text(" v1"));
}

#[test]
fn malformed_ensures_blocks_are_syntax_errors() {
    // No rows.
    rejects("warrant t v1\n  op f(x: Bytes) -> Bytes\n  ensures f:\n", "W0001");
    // No colon.
    rejects("warrant t v1\n  op f(x: Bytes) -> Bytes\n  ensures f\n    true\n", "W0001");
    // No operation name.
    rejects("warrant t v1\n  ensures:\n    true\n", "W0001");
}

#[test]
fn the_input_block_ensures_row_keeps_its_meaning() {
    // `ensures` as the last row of an `input` block is unchanged (VER04 section 2).
    accepts(
        "warrant t\n  op f(x: Bytes) -> Bool\n  input f.x:\n    hostile\n    ensures no_panic\n",
    );
}
