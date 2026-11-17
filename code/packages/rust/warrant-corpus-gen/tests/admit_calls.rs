//! The owner decisions of 2026-11-17, second batch (VER04r 16.9): [D18] v0 rule 2 applies to
//! the evaluation of an example, and [D19] a subject call whose arguments are outside the
//! callee's `admit` is a generation error, in a claim (the first such admitted case is
//! named) and in an example.  Both are refusals: nothing is discarded silently, and the
//! emitted bytes of a document that passes are the bytes it had before.

mod support;

use coding_adventures_warrant_corpus_gen::{code, GenInput};
use support::*;

/// The operations every document here declares: `(name, declaration line)`.
const OPS: [(&str, &str); 4] = [
    ("op_add", "op op_add(a: U8, b: U8) -> U8"),
    ("op_lo", "op op_lo(a: U8 | a < 10u8, b: U8) -> U8"),
    ("op_two", "op op_two(a: U8 | a < 10u8, b: U8 | b > a) -> U8"),
    ("op_m", "op op_m(a: U8 | add8(a, 1u8) < 5u8, b: U8) -> U8"),
];

/// A document with `OPS`, two models, and the given version suffix and extra declarations.
fn doc(version: &str, extra: &str) -> String {
    doc_with(version, extra, &[])
}

/// As [`doc`], with more operations.
fn doc_with(version: &str, extra: &str, more: &[(&str, &str)]) -> String {
    let ops: Vec<(&str, &str)> = OPS.iter().chain(more.iter()).copied().collect();
    let mut s = format!("warrant synth{version}\n");
    s += "  model add8(a: U8, b: U8) -> U8 = a + b\n";
    s += "  model first(a: U8, b: U8) -> U8 = a\n";
    for (_, line) in &ops {
        s += &format!("  {line}\n");
    }
    s += "  assume platform:\n    none\n  assume environment:\n    none\n";
    for (op, _) in &ops {
        s += &format!("  labels {op}:\n    a secret\n    b secret\n");
    }
    for (op, _) in &ops {
        for p in ["a", "b"] {
            s += &format!("  input {op}.{p}:\n    hostile\n    ensures no_panic, terminates\n");
        }
    }
    for (op, _) in &ops {
        s += &format!("  bound {op}:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n");
    }
    s += extra;
    s
}

fn gen(src: &str) -> Result<coding_adventures_warrant_corpus_gen::CorpusV2, coding_adventures_warrant_corpus_gen::GenError> {
    gen_profile(src, "")
}

fn gen_profile(src: &str, profile: &str) -> Result<coding_adventures_warrant_corpus_gen::CorpusV2, coding_adventures_warrant_corpus_gen::GenError> {
    with_file(src, |f| {
        coding_adventures_warrant_corpus_gen::generate(&GenInput {
            file: f,
            spec_path: "s.warrant",
            spec_bytes: src.as_bytes(),
            profile_bytes: profile.as_bytes(),
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
fn d19_an_admit_that_reads_a_subject_dependent_argument_is_refused_statically() {
    let nested = "  claim n:\n    forall a: U8 | a < 10u8, b: U8 | b < 2u8. op_lo(op_add(a, b), b) == first(add8(a, b), b)\n";
    let e = gen(&doc(" v1", nested)).unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::W0701, "n"), "{e}");
    assert!(e.message.contains("depends on a subject result"), "{e}");
}

#[test]
fn d19_a_subject_dependent_argument_the_admit_does_not_read_is_accepted_and_the_rest_checked() {
    // op_lo's admit reads `a` only: `b` may be a subject result (the gf256 shape
    // `power(add(a, b), 2u64)`), and `a` is still checked on every admitted case.
    let ok = "  claim n:\n    forall a: U8 | a < 10u8, b: U8 | b < 2u8. op_lo(a, op_add(a, b)) == first(a, add8(a, b))\n";
    assert!(gen(&doc(" v1", ok)).is_ok());
    let weak = ok.replace("a < 10u8", "a < 12u8");
    let e = gen(&doc(" v1", &weak)).unwrap_err();
    assert!(e.message.contains("admitted case 20"), "{e}");
}

#[test]
fn d18_a_wrap_while_evaluating_a_callee_admit_is_refused_in_v0_and_not_recorded() {
    // op_m's admit is `add8(a, 1u8) < 5u8`: at a = 255 it wraps to 0 and admits.
    let claim = "  claim m:\n    forall a: U8 | a == 255u8, b: U8 | b < 2u8. op_m(a, b) == first(a, b)\n";
    let e = gen(&doc("", claim)).unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::W0701, "m"), "{e}");
    assert!(e.message.contains("wraps while evaluating the admit of op `op_m`"), "{e}");
    assert!(e.message.contains("admitted case 0"), "{e}");
    let c = gen(&doc(" v1", claim)).unwrap();
    // the wraps of the admit are not the document's: the suite row records none
    assert!(c.corpus_json.contains("\"wraps\": 0"), "{}", c.corpus_json);
    let ex = "  example e:\n    op_m(255u8, 4u8) == 255u8\n";
    let e = gen(&doc("", ex)).unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::W0701, "e"), "{e}");
    assert!(e.message.contains("wraps while evaluating the admit of op `op_m`"), "{e}");
    assert!(gen(&doc(" v1", ex)).is_ok());
    // no wrap, no refusal
    let calm = "  claim m:\n    forall a: U8 | a < 3u8, b: U8 | b < 2u8. op_m(a, b) == first(a, b)\n";
    assert!(gen(&doc("", calm)).is_ok());
}

#[test]
fn d19_two_offending_calls_in_one_case_name_the_leftmost() {
    // both calls are outside their admit in the first admitted case (a = 10, b = 0); the
    // oracle walks left to right and so does the generator
    let both = "  claim both:\n    forall a: U8 | a >= 10u8, b: U8 | b < 2u8. op_lo(a, b) == op_two(a, b)\n";
    let e = gen(&doc(" v1", both)).unwrap_err();
    assert!(e.message.contains("`op_lo`") && e.message.contains("admitted case 0"), "{e}");
    let swapped = both.replace("op_lo(a, b) == op_two(a, b)", "op_two(a, b) == op_lo(a, b)");
    let e = gen(&doc(" v1", &swapped)).unwrap_err();
    assert!(e.message.contains("`op_two`"), "{e}");
}

/// A balanced `op` tree of `n` copies of `leaf`.
fn bal(leaf: &str, n: usize, op: &str) -> String {
    if n == 1 {
        return leaf.to_string();
    }
    format!("({} {op} {})", bal(leaf, n / 2, op), bal(leaf, n - n / 2, op))
}

#[test]
fn d19_the_check_is_charged_to_the_ir_work_meter() {
    // 128 calls of an op whose admit costs ~500, over 2^28 product ordinals: ~1.8e13, past the
    // full-run bound of 8.6e12, while the recorded ir_cost (about 700) alone is far below it.
    let admit = bal("a < 10u8", 256, "and");
    let big = format!("op op_big(a: U8 | {admit}, b: U8) -> U8");
    let body = bal("op_big(a, b)", 128, "+");
    let claim = format!(
        "  claim dos:\n    forall a: U8 | a < 10u8, b: U8, c: U8, d: Bool, e: Bool, f: Bool, g: Bool. {body} == 0u8\n"
    );
    let e = gen_profile(&doc_with(" v1", &claim, &[("op_big", &big)]), "budget 268435456\n").unwrap_err();
    assert_eq!((e.code, e.at.as_str()), (code::LIMIT, "dos"), "{e}");
    assert!(e.message.contains("total IR work"), "{e}");
}
