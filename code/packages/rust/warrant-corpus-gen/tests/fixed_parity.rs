//! The S10b acceptance test: the Rust lowering of every `ir.fixed` built-in and the independent
//! oracle agree, byte for byte (VER04r section 13, slice S10b).
//!
//! `python/warrant-oracle` committed, in slice S10a, a synthetic document that uses all 35
//! built-ins of Appendix A.1 at every width the spec allows (`fixed_forms_v1.warrant`) and its
//! *lowering dump* (`fixed_forms_v1.oracle-lowering.json`, schema `warrant-lowering-parity-v1`:
//! the lowered IR of every op `admit`, model, suite and example, each unit's computed
//! `requires`, and the worst-path `ir_cost`).  This test lowers the same document with this
//! crate and renders it with [`view::oracle_view_v1`]; the two texts must be **identical**.
//!
//! The files are read from the oracle's directory (no copy), so an edit to either one of them
//! reruns this test (the build tool's repository-input row `rust/warrant-corpus-gen`).  The
//! oracle's own BUILD `--check`s the dump against its `dump-lowering` subcommand, so the chain
//! is: oracle source -> committed dump == this crate's lowering.
//!
//! The negative controls at the bottom show the comparison can fail.

mod support;

use coding_adventures_warrant_corpus_gen::ir::Ir;
use coding_adventures_warrant_corpus_gen::lower::{lower_document, lower_document_with_fixed};
use coding_adventures_warrant_corpus_gen::view::oracle_view_v1;
use coding_adventures_warrant_corpus_gen::{code, GenError};
use support::*;

const WARRANT: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-forms-v1/fixed_forms_v1.warrant"
);
const DUMP: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/fixed-forms-v1/fixed_forms_v1.oracle-lowering.json"
);

/// The bound the oracle's `dump-lowering` costs under by default (`MAX_IR_COST`).
const LIMIT: u64 = 2_000;

fn view_text(src: &str) -> String {
    with_file(src, |f| {
        let l = lower_document_with_fixed(f).expect("lowers");
        oracle_view_v1(&l, LIMIT)
            .expect("view")
            .render()
            .expect("render")
    })
}

/// The first line where two texts differ, with a little context, for a readable failure.
fn first_difference(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}: rust `{x}` vs oracle `{y}`", i + 1);
        }
    }
    format!(
        "lengths differ: {} vs {} lines",
        a.lines().count(),
        b.lines().count()
    )
}

#[test]
fn the_fixed_forms_document_matches_the_oracle_dump_byte_for_byte() {
    let text = view_text(WARRANT);
    assert!(text == DUMP, "{}", first_difference(&text, DUMP));
}

#[test]
fn the_dump_is_canonical_json_of_the_v1_schema() {
    assert!(!DUMP.contains('\r'));
    assert!(DUMP.ends_with("}\n") && !DUMP.ends_with("\n\n"));
    assert!(DUMP.is_ascii());
    assert!(DUMP.contains("\"schema\": \"warrant-lowering-parity-v1\""));
}

#[test]
fn generate_stays_a_fail_closed_stub_for_the_fixed_document() {
    // Lowering for the dump succeeds, but the ordinary path refuses the document as a whole,
    // naming the level, exactly as before S10b.
    let e: GenError = with_file(WARRANT, |f| lower_document(f).expect_err("refused"));
    assert_eq!(e.code, code::LEVEL, "{e}");
    assert_eq!(e.level, Some("ir.fixed"), "{e}");
}

// ---- negative controls ---------------------------------------------------------------------

#[test]
fn a_mis_lowered_built_in_loses_parity() {
    let mutated = WARRANT.replacen("bit_and(a, b)", "bit_or(a, b)", 1);
    assert_ne!(mutated, WARRANT);
    assert_ne!(view_text(&mutated), DUMP);
}

#[test]
fn a_dropped_requires_loses_parity() {
    let text = view_text(WARRANT);
    let broken = text.replacen("\"ir.fixed\"", "\"ir.core\"", 1);
    assert_ne!(broken, DUMP);
}

#[test]
fn the_fixed_nodes_really_are_in_the_lowered_document() {
    let mut fixed = 0;
    with_file(WARRANT, |f| {
        let l = lower_document_with_fixed(f).unwrap();
        for m in &l.models {
            m.body
                .walk(&mut |n| fixed += usize::from(matches!(n, Ir::Fixed { .. })));
        }
    });
    assert!(fixed > 400, "{fixed}");
}
