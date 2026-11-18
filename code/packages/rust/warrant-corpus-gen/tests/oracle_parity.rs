//! The S3a acceptance test: the Rust lowering and the independent oracle agree.
//!
//! `python/warrant-oracle` is a second reader of `.warrant` files that shares no code with
//! this crate (VER04r section 10, [D3]).  It lowers the same documents to the same IR with
//! its own typer.  `tests/fixtures/dump_oracle_lowering.py` prints the oracle's lowering
//! (plus worst-path costs) as canonical JSON, and this test renders the *Rust* lowering of
//! the same document with [`view::oracle_view`] and demands the two be **byte-identical**.
//!
//! Documents compared: ct-compare (the S3 acceptance document), gf256 (arithmetic, 35
//! models, `max_ir_cost 32768`), and two synthetic documents that between them write every
//! `ir.core` construct a v0 / v1 document can.
//!
//! The negative controls at the bottom prove the comparison can fail: each takes the correct
//! Rust lowering, mis-lowers one thing on purpose, and checks that parity is lost.

mod support;

use coding_adventures_warrant_corpus_gen::ir::{Cmp, Ir};
use coding_adventures_warrant_corpus_gen::lower::{lower_document, Lowered, SuiteKind};
use coding_adventures_warrant_corpus_gen::view::oracle_view;
use support::*;

const CT_FIXTURE: &str = include_str!("fixtures/ct-compare.oracle-lowering.json");
const GF_FIXTURE: &str = include_str!("fixtures/gf256.oracle-lowering.json");
const V0_FIXTURE: &str = include_str!("fixtures/core_forms_v0.oracle-lowering.json");
const V1_FIXTURE: &str = include_str!("fixtures/core_forms_v1.oracle-lowering.json");

fn lowered(src: &str) -> Lowered {
    with_file(src, |f| lower_document(f).expect("lowers"))
}

fn view_text(l: &Lowered, limit: u64) -> String {
    oracle_view(l, limit)
        .expect("view")
        .render()
        .expect("render")
}

#[test]
fn ct_compare_matches_the_oracle_byte_for_byte() {
    let text = view_text(&lowered(CT_COMPARE), 2_000);
    assert_eq!(text, CT_FIXTURE);
}

#[test]
fn gf256_matches_the_oracle_byte_for_byte() {
    let text = view_text(&lowered(GF256), 32_768);
    assert_eq!(text, GF_FIXTURE);
}

#[test]
fn the_v0_synthetic_document_matches_the_oracle_byte_for_byte() {
    let text = view_text(&lowered(CORE_V0), 2_000);
    assert_eq!(text, V0_FIXTURE);
}

#[test]
fn the_v1_synthetic_document_matches_the_oracle_byte_for_byte() {
    let text = view_text(&lowered(CORE_V1), 2_000);
    assert_eq!(text, V1_FIXTURE);
}

#[test]
fn the_fixtures_are_canonical_json_with_one_final_newline_and_no_cr() {
    for (name, f) in [
        ("ct", CT_FIXTURE),
        ("gf", GF_FIXTURE),
        ("v0", V0_FIXTURE),
        ("v1", V1_FIXTURE),
    ] {
        assert!(!f.contains('\r'), "{name}");
        assert!(f.ends_with("}\n") && !f.ends_with("\n\n"), "{name}");
        assert!(f.is_ascii(), "{name}");
        assert!(
            f.contains("\"schema\": \"warrant-lowering-parity-v0\""),
            "{name}"
        );
    }
}

// ---------------------------------------------------------------------------
// Negative controls: a deliberately mis-lowered tree must break parity
// ---------------------------------------------------------------------------

/// The mutable children of a node, in order.
fn kids_mut(ir: &mut Ir) -> Vec<&mut Ir> {
    match ir {
        Ir::Bool(_) | Ir::Int { .. } | Ir::Bytes(_) | Ir::Var(_) | Ir::Err(_) => vec![],
        Ir::Subject { args, .. } | Ir::Model { args, .. } | Ir::Fixed { args, .. } => {
            args.iter_mut().collect()
        }
        Ir::Len(a) | Ir::Ok(a) | Ir::Not(a) | Ir::IsErr(a) | Ir::Neg(_, a) => vec![a],
        Ir::Cmp(_, a, b)
        | Ir::Arith(_, _, a, b)
        | Ir::Logic(_, a, b)
        | Ir::Concat(a, b)
        | Ir::Take(a, b)
        | Ir::Drop(a, b) => vec![a, b],
        Ir::If(c, t, e) => vec![c, t, e],
    }
}

/// Apply `f` to the first node (pre-order) for which it returns true.
fn mutate_first(ir: &mut Ir, f: &mut dyn FnMut(&mut Ir) -> bool) -> bool {
    if f(ir) {
        return true;
    }
    kids_mut(ir).into_iter().any(|k| mutate_first(k, f))
}

/// Apply `f` across every model body, suite admit/body, in order, until it fires once.
fn mutate_somewhere(l: &mut Lowered, f: &mut dyn FnMut(&mut Ir) -> bool) -> bool {
    for m in &mut l.models {
        if mutate_first(&mut m.body, f) {
            return true;
        }
    }
    for s in &mut l.suites {
        if mutate_first(&mut s.admit, f) || mutate_first(&mut s.body, f) {
            return true;
        }
    }
    false
}

fn assert_parity_breaks(
    mut l: Lowered,
    fixture: &str,
    limit: u64,
    mutation: &mut dyn FnMut(&mut Lowered) -> bool,
    what: &str,
) {
    assert_eq!(
        view_text(&l, limit),
        fixture,
        "the unmutated lowering must match first"
    );
    assert!(mutation(&mut l), "mutation `{what}` did not apply");
    assert_ne!(
        view_text(&l, limit),
        fixture,
        "parity survived a mis-lowering: {what}"
    );
}

#[test]
fn control_eq_lowered_as_ne_breaks_parity() {
    assert_parity_breaks(
        lowered(CT_COMPARE),
        CT_FIXTURE,
        2_000,
        &mut |l| {
            mutate_somewhere(l, &mut |n| match n {
                Ir::Cmp(c @ Cmp::Eq, ..) => {
                    *c = Cmp::Ne;
                    true
                }
                _ => false,
            })
        },
        "same: a == b lowered as ne",
    );
}

#[test]
fn control_swapped_operands_of_a_subtraction_break_parity() {
    // `a - 2u8` lowered as `2u8 - a`: the classic operand-order bug.
    assert_parity_breaks(
        lowered(GF256),
        GF_FIXTURE,
        32_768,
        &mut |l| {
            mutate_somewhere(l, &mut |n| match n {
                Ir::Arith(coding_adventures_warrant_corpus_gen::ir::Arith::Sub, _, a, b) => {
                    std::mem::swap(a, b);
                    true
                }
                _ => false,
            })
        },
        "a - b lowered as b - a",
    );
}

#[test]
fn control_a_dropped_refinement_breaks_parity() {
    assert_parity_breaks(
        lowered(CT_COMPARE),
        CT_FIXTURE,
        2_000,
        &mut |l| {
            let s = l
                .suites
                .iter_mut()
                .find(|s| s.claim == "select_picks")
                .unwrap();
            s.admit = Ir::Bool(true);
            true
        },
        "select_picks lost its refinement",
    );
}

#[test]
fn control_swapped_subject_arguments_break_parity() {
    assert_parity_breaks(
        lowered(CT_COMPARE),
        CT_FIXTURE,
        2_000,
        &mut |l| {
            mutate_somewhere(l, &mut |n| match n {
                Ir::Subject { op, args, .. } if op == "select_bytes" => {
                    args.swap(0, 1);
                    true
                }
                _ => false,
            })
        },
        "select_bytes(a, b, c) lowered as select_bytes(b, a, c)",
    );
}

#[test]
fn control_an_and_chain_folded_right_instead_of_left_breaks_parity() {
    // fold3 has three refinements; left fold is and(and(r1,r2),r3), a right fold is
    // and(r1,and(r2,r3)).
    assert_parity_breaks(
        lowered(CORE_V0),
        V0_FIXTURE,
        2_000,
        &mut |l| {
            let s = l.suites.iter_mut().find(|s| s.claim == "fold3").unwrap();
            let Ir::Logic(op, outer_left, r3) = s.admit.clone() else {
                return false;
            };
            let Ir::Logic(_, r1, r2) = *outer_left else {
                return false;
            };
            s.admit = Ir::Logic(op, r1, Box::new(Ir::Logic(op, r2, r3)));
            true
        },
        "three refinements folded right",
    );
}

#[test]
fn control_a_reference_claim_with_its_sides_swapped_breaks_parity() {
    assert_parity_breaks(
        lowered(CORE_V0),
        V0_FIXTURE,
        2_000,
        &mut |l| {
            let s = l
                .suites
                .iter_mut()
                .find(|s| s.claim == "ref_right")
                .unwrap();
            match &mut s.kind {
                SuiteKind::Reference {
                    subject, expected, ..
                } => {
                    std::mem::swap(subject, expected);
                    true
                }
                SuiteKind::Predicate => false,
            }
        },
        "subject and expected exchanged",
    );
}

#[test]
fn control_reversed_model_order_breaks_parity() {
    assert_parity_breaks(
        lowered(CT_COMPARE),
        CT_FIXTURE,
        2_000,
        &mut |l| {
            l.models.reverse();
            true
        },
        "model_order reversed",
    );
}

#[test]
fn control_a_wrong_cost_limit_that_changes_nothing_still_matches_but_a_tighter_one_fails_closed() {
    // The limit is not part of the view's text unless it is exceeded: a looser limit gives
    // the same bytes, a limit under the largest expression is refused, not silently wrong.
    let l = lowered(CT_COMPARE);
    assert_eq!(view_text(&l, 65_536), CT_FIXTURE);
    assert!(oracle_view(&l, 8).is_err());
}

#[test]
fn control_a_single_flipped_byte_in_the_fixture_is_detected_by_the_comparison() {
    let l = lowered(CT_COMPARE);
    let text = view_text(&l, 2_000);
    let flipped = CT_FIXTURE.replacen("\"eq\"", "\"ne\"", 1);
    assert_ne!(text, flipped);
    let one_digit = CT_FIXTURE.replacen("\"ir_cost\": 17", "\"ir_cost\": 16", 1);
    assert_ne!(text, one_digit);
}

#[test]
fn control_a_mis_lowered_op_refinement_breaks_parity() {
    // `h`'s refinements are and(a <= 100, b <= a); lower the second as `b < a` instead.
    assert_parity_breaks(
        lowered(CORE_V0),
        V0_FIXTURE,
        2_000,
        &mut |l| {
            let h = l.ops.iter_mut().find(|o| o.name == "h").unwrap();
            let admit = h.admit.as_mut().unwrap();
            mutate_first(admit, &mut |n| match n {
                Ir::Cmp(c @ Cmp::Le, a, _) if matches!(&**a, Ir::Var(v) if v == "b") => {
                    *c = Cmp::Lt;
                    true
                }
                _ => false,
            })
        },
        "op h: b <= a lowered as b < a",
    );
}

#[test]
fn control_a_dropped_op_refinement_breaks_parity() {
    assert_parity_breaks(
        lowered(GF256),
        GF_FIXTURE,
        32_768,
        &mut |l| {
            let p = l.ops.iter_mut().find(|o| o.name == "power").unwrap();
            p.admit = None;
            true
        },
        "gf256 power lost its refinement",
    );
}
