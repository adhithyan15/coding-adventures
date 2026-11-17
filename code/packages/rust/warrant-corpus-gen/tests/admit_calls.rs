//! The owner decisions of 2026-11-17, second batch (VER04r 16.9): [D18] v0 rule 2 applies to
//! the evaluation of an example, and [D19] a subject call whose arguments are outside the
//! callee's `admit` is a generation error, in a claim (the first such admitted case is
//! named) and in an example.  Both are refusals: nothing is discarded silently, and the
//! emitted bytes of a document that passes are the bytes it had before.

mod support;

use coding_adventures_warrant_corpus_gen::{code, GenInput};
use support::*;

/// A document with `op_add(a: U8, b: U8)` (no admit) and `op_lo(a: U8 | a < 10u8, b: U8)`,
/// one model, and the given version suffix and extra declarations.
fn doc(version: &str, extra: &str) -> String {
    let mut s = format!("warrant synth{version}\n");
    s += "  model add8(a: U8, b: U8) -> U8 = a + b\n";
    s += "  model first(a: U8, b: U8) -> U8 = a\n";
    s += "  op op_add(a: U8, b: U8) -> U8\n";
    s += "  op op_lo(a: U8 | a < 10u8, b: U8) -> U8\n";
    s += "  assume platform:\n    none\n  assume environment:\n    none\n";
    for op in ["op_add", "op_lo"] {
        s += &format!("  labels {op}:\n    a secret\n    b secret\n");
    }
    for op in ["op_add", "op_lo"] {
        for p in ["a", "b"] {
            s += &format!("  input {op}.{p}:\n    hostile\n    ensures no_panic, terminates\n");
        }
    }
    for op in ["op_add", "op_lo"] {
        s += &format!("  bound {op}:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n");
    }
    s += extra;
    s
}

fn gen(src: &str) -> Result<coding_adventures_warrant_corpus_gen::CorpusV2, coding_adventures_warrant_corpus_gen::GenError> {
    with_file(src, |f| {
        coding_adventures_warrant_corpus_gen::generate(&GenInput {
            file: f,
            spec_path: "s.warrant",
            spec_bytes: src.as_bytes(),
            profile_bytes: b"",
            binding_faults: None,
        })
    })
}

const WRAPPING_EXAMPLE: &str = "  example wrap:\n    op_add(255u8 + 2u8, 0u8) == 1u8\n";
const PLAIN_CLAIM: &str = "  claim c:\n    forall a: U8 | a < 2u8, b: U8 | b < 2u8. op_add(a, b) == add8(a, b)\n";

#[test]
fn d18_an_example_that_wraps_fails_in_a_v0_document() {
    let e = gen(&doc("", &format!("{PLAIN_CLAIM}{WRAPPING_EXAMPLE}"))).unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::W0701, "wrap"), "{e}");
    assert!(e.message.contains("wraps"), "{e}");
}

#[test]
fn d18_the_same_example_is_accepted_in_a_v1_document() {
    let c = gen(&doc(" v1", &format!("{PLAIN_CLAIM}{WRAPPING_EXAMPLE}"))).unwrap();
    assert!(c.corpus_json.contains("warrant/synth/example/wrap"));
    // 255u8 + 2u8 wraps to 1u8, and that is the argument the corpus carries
    assert!(c.cases_tfv.contains("Ka I1 Kb I0 Kexpect I1"), "{}", c.cases_tfv);
}

#[test]
fn d18_a_v0_example_that_does_not_wrap_is_accepted() {
    let ok = "  example fine:\n    op_add(1u8 + 2u8, 0u8) == 3u8\n";
    assert!(gen(&doc("", &format!("{PLAIN_CLAIM}{ok}"))).is_ok());
}

#[test]
fn d19_a_claim_with_a_weaker_refinement_than_the_ops_admit_is_refused() {
    let weak = "  claim weak:\n    forall a: U8 | a < 20u8, b: U8 | b < 2u8. op_lo(a, b) == first(a, b)\n";
    let e = gen(&doc(" v1", weak)).unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::W0701, "weak"), "{e}");
    assert!(e.message.contains("`op_lo`"), "{e}");
    // the first offender in product order: a = 10, b = 0 is admitted case 20 (a < 20, b < 2)
    assert!(e.message.contains("admitted case 20"), "{e}");
}

#[test]
fn d19_the_same_claim_with_a_matching_refinement_is_accepted() {
    let ok = "  claim ok:\n    forall a: U8 | a < 10u8, b: U8 | b < 2u8. op_lo(a, b) == first(a, b)\n";
    let c = gen(&doc(" v1", ok)).unwrap();
    assert_eq!(c.chunk_files.len(), 1);
}

#[test]
fn d19_an_unrefined_binder_is_refused_even_when_the_hole_is_one_value() {
    // a: U8 reaches 255; the very first offending case is a = 10, b = 0: admitted case 2560
    let no_refinement = "  claim open:\n    forall a: U8, b: U8. op_lo(a, b) == first(a, b)\n";
    let e = gen(&doc(" v1", no_refinement)).unwrap_err();
    assert!(e.message.contains("admitted case 2560"), "{e}");
}

#[test]
fn d19_an_example_outside_the_ops_admit_is_refused() {
    let bad = "  example out:\n    op_lo(10u8, 0u8) == 10u8\n";
    let e = gen(&doc(" v1", bad)).unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::W0701, "out"), "{e}");
    assert!(e.message.contains("`op_lo`"), "{e}");
    let ok = "  example in_admit:\n    op_lo(9u8, 0u8) == 9u8\n";
    assert!(gen(&doc(" v1", ok)).is_ok());
}

#[test]
fn d19_a_call_under_an_untaken_branch_is_not_checked_a_taken_one_is() {
    // The call sits in the `then` of a pure guard: only the cases that take it are checked.
    let guarded = "  claim g:\n    forall a: U8, b: U8. (if a < 10u8 then op_lo(a, b) else op_add(a, b)) == (if a < 10u8 then first(a, b) else add8(a, b))\n";
    // the else branch wraps: it is a v1 document, which is fine
    assert!(gen(&doc(" v1", guarded)).is_ok());
    let bad_guard = guarded.replace("a < 10u8 then op_lo", "a < 11u8 then op_lo");
    let e = gen(&doc(" v1", &bad_guard)).unwrap_err();
    assert!(e.message.contains("outside the op's admit"), "{e}");
}

#[test]
fn d19_an_argument_that_depends_on_a_subject_result_is_refused_statically() {
    let nested = "  claim n:\n    forall a: U8 | a < 10u8, b: U8 | b < 2u8. op_lo(op_add(a, b), b) == first(add8(a, b), b)\n";
    let e = gen(&doc(" v1", nested)).unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::W0701, "n"), "{e}");
    assert!(e.message.contains("depends on a subject result"), "{e}");
}
