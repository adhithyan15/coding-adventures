//! The VER04r 16.2 census (slice S3b): which claims of which adopters can the S3 generator
//! emit at `ir.core`, and why not.
//!
//! VER04r 16.3 item 11e settles for S3 that a binder descriptor of kind `nat_count`, a signed
//! `fx_edge`, a 16-bit `fx_edge` or a list of byte strings is *refused*, and asks S3b for the
//! census S10 needs to decide them.  [`census_document`] reads every claim of a document
//! (the lowering's first refusal, the cost bounds, the S2 domain resolution and the 16.2 scan
//! of the resolved descriptors) and [`render`] prints it.
//!
//! The first test runs the census over **the eleven adopters that bind `gen_evidence`**
//! (`code/packages/rust/*/evidence.bindings`) and prints the report (`cargo test --test
//! census -- --nocapture`); the numbers it prints are quoted in the crate README.  It pins
//! only what must stay true, not the adopters' evolving claim counts: ct-compare and gf256
//! are the two documents the generator emits whole, and every adopter's census must at
//! least be computable and internally consistent.  The remaining tests pin the census logic
//! on small synthetic documents, including one hit for each of the five 16.2 items.

mod support;

use coding_adventures_warrant_corpus_gen::census::{census_document, render, DocCensus};
use coding_adventures_warrant_corpus_gen::error::{code, GenError};
use coding_adventures_warrant_corpus_gen::profile::{parse_profile, Profile};
use support::*;

/// The adopters of `gen_evidence`, by module name.
const ADOPTERS: [(&str, &str); 11] = [
    (
        "arithmetic",
        include_str!("../../../../specs/arithmetic.warrant"),
    ),
    (
        "ct-compare",
        include_str!("../../../../specs/ct-compare.warrant"),
    ),
    ("gf256", include_str!("../../../../specs/gf256.warrant")),
    (
        "hash-functions",
        include_str!("../../../../specs/hash-functions.warrant"),
    ),
    (
        "logic-gates",
        include_str!("../../../../specs/logic-gates.warrant"),
    ),
    ("lzw", include_str!("../../../../specs/lzw.warrant")),
    (
        "vos-sched-any",
        include_str!("../../../../specs/vos-sched-any.warrant"),
    ),
    (
        "vos-sched-fp-bitmap",
        include_str!("../../../../specs/vos-sched-fp-bitmap.warrant"),
    ),
    (
        "wasm-leb128",
        include_str!("../../../../specs/wasm-leb128.warrant"),
    ),
    ("x25519", include_str!("../../../../specs/x25519.warrant")),
    ("zeroize", include_str!("../../../../specs/zeroize.warrant")),
];

/// The census profile: the ceiling cost bound, so that no adopter is blocked by a cost the
/// document would simply declare (gf256 declares 32,768; lzw needs more than 2,000).
fn ceiling() -> Profile {
    parse_profile(b"max_ir_cost 65536\n").unwrap()
}

fn census_of(src: &str, p: &Profile) -> Result<DocCensus, GenError> {
    with_file(src, |f| census_document(f, p))
}

fn adopters() -> Vec<(String, Result<DocCensus, GenError>)> {
    ADOPTERS
        .iter()
        .map(|(name, src)| (name.to_string(), census_of(src, &ceiling())))
        .collect()
}

#[test]
fn the_adopter_census_is_computed_for_all_eleven_documents_and_printed() {
    let docs = adopters();
    assert_eq!(docs.len(), 11);
    let report = render(&docs, false);
    println!("{report}");
    let verbose = render(&docs, true);
    assert!(verbose.len() > report.len());
    // every adopter is a well-formed document and its census is internally consistent
    let mut claims = 0;
    for (name, doc) in &docs {
        let d = doc.as_ref().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(d.emittable_claims() <= d.claims.len(), "{name}");
        assert_eq!(
            d.emittable(),
            d.emittable_claims() == d.claims.len()
                && d.other_blocks.is_empty()
                && d.cost_error.is_none(),
            "{name}"
        );
        claims += d.claims.len();
    }
    assert!(claims > 300, "{claims}");
    assert!(report.contains("TOTAL: 11 documents"));
    // deterministic
    assert_eq!(report, render(&adopters(), false));
}

#[test]
fn ct_compare_and_gf256_are_emittable_whole_and_touch_none_of_16_2() {
    for (name, claims) in [("ct-compare", 9), ("gf256", 38)] {
        let src = ADOPTERS.iter().find(|a| a.0 == name).unwrap().1;
        let d = census_of(src, &ceiling()).unwrap();
        assert_eq!(d.claims.len(), claims, "{name}");
        assert!(d.emittable(), "{name}");
        assert!(d.other_blocks.is_empty(), "{name}");
        assert!(
            d.claims
                .iter()
                .all(|c| c.s162.is_empty() && c.reason_key() == "ok"),
            "{name}"
        );
    }
    // gf256 under the DEFAULT cost bound is blocked: it declares `max_ir_cost 32768`
    let gf = ADOPTERS.iter().find(|a| a.0 == "gf256").unwrap().1;
    let d = census_of(gf, &Profile::default()).unwrap();
    assert!(!d.emittable());
}

#[test]
fn the_unlowerable_adopters_are_refused_whole_with_the_level_named() {
    for (name, doc) in adopters() {
        let d = doc.unwrap();
        if name == "ct-compare" || name == "gf256" {
            continue;
        }
        // an adopter beyond ir.core is not emittable whole: fail closed (VER04r 13)
        assert!(!d.emittable(), "{name}");
    }
}

// -- the census logic on synthetic documents ------------------------------------------------

fn v1_doc(extra_models: &str, claims: &str) -> String {
    let head = &ENUM_V1[..ENUM_V1.find("  claim ").unwrap()];
    format!(
        "{}{claims}",
        head.replace(
            "  error E = BAD\n",
            &format!("  error E = BAD\n{extra_models}")
        )
    )
}

fn one(src: &str) -> DocCensus {
    census_of(src, &Profile::default()).unwrap()
}

#[test]
fn a_claim_within_ir_core_is_ok_with_its_product_and_descriptors() {
    let d = one(&v1_doc(
        "",
        "  claim fine:\n    forall a: U8, b: Bool. op_inc(a) == inc(a)\n",
    ));
    let c = &d.claims[0];
    assert!(c.emittable() && d.emittable());
    assert_eq!(
        c.domain.as_ref().unwrap(),
        &(512, vec!["u8_all".to_string(), "bool".to_string()])
    );
    assert_eq!(c.reason_key(), "ok");
}

#[test]
fn the_five_16_2_items_are_each_reported_by_name() {
    // item 5: two U16 binders are 2^32 ordinals; the budget demotes one to an edge set
    let d = one(&v1_doc(
        "",
        "  claim i5:\n    forall a: U16, b: U16. op_u16(a) == id16(b)\n",
    ));
    assert_eq!(d.claims[0].s162, ["5"]);
    assert!(!d.claims[0].emittable());
    assert_eq!(d.claims[0].reason_key(), "16.2");
    assert!(
        d.claims[0].blocked.is_none(),
        "it lowers fine: only the descriptor is undecided"
    );
    // item 4: a signed 64-bit leaf (the unsigned 64-bit leaf is fine)
    let d = one(&v1_doc(
        "  model idi(a: I64) -> I64 = a\n",
        "  claim i4:\n    forall a: I64. op_u64(1u64) == id64(1u64) and idi(a) == a\n",
    ));
    assert_eq!(d.claims[0].s162, ["4"]);
    // item 3: a Nat that flows into a shift count has the 73-value domain (and shl is ir.fixed)
    let d = one(&v1_doc(
        "",
        "  claim i3:\n    forall n: Nat, a: U8. op_inc(a) == inc(shl(a, n))\n",
    ));
    assert_eq!(d.claims[0].s162, ["3"]);
    assert_eq!(
        d.claims[0].reason_key(),
        "ir.fixed",
        "the level is the first blocker"
    );
    // items 1/2: a list of byte strings (ir.data as well)
    let d = one(&v1_doc(
        "",
        "  claim i12:\n    forall l: List<Bytes<2>>. op_inc(1u8) == inc(1u8)\n",
    ));
    assert_eq!(d.claims[0].s162, ["1/2"]);
    // a list of Nat is not a list of byte strings
    let d = one(&v1_doc(
        "",
        "  claim ln:\n    forall l: List<Nat>. op_inc(1u8) == inc(1u8)\n",
    ));
    assert!(d.claims[0].s162.is_empty());
    assert_eq!(d.claims[0].reason_key(), "ir.data");
    // a fine unsigned wide leaf and a v1 U64 are not on the list
    let d = one(&v1_doc(
        "",
        "  claim wide:\n    forall a: U32, b: U64. op_u32(a) == id32(a) and op_u64(b) == id64(b)\n",
    ));
    assert!(d.claims[0].s162.is_empty());
}

#[test]
fn each_blocker_is_reported_under_its_own_key() {
    let d = one(&v1_doc(
        "  model big(n: Nat) -> Nat = n\n",
        "  claim over_budget:\n    forall a: Bytes, b: Bytes, c: Bytes. op_cat(a, b) == op_cat(a, c)\n  claim no_call:\n    forall a: U8. a == a\n",
    ));
    // 347^3 > 2^24: the resolver's W0706, with the lowering fine
    let c = &d.claims[0];
    assert_eq!(c.domain.as_ref().unwrap_err().code, code::W0706);
    assert_eq!(c.reason_key(), "W0706");
    assert!(!c.emittable());
    // a claim that calls no operation is W0701 at lowering (S3a review L5)
    assert_eq!(d.claims[1].blocked.as_ref().unwrap().code, code::W0701);
    assert_eq!(d.claims[1].reason_key(), "W0701");
    assert!(!d.emittable());
    // the report names both, and an unresolvable domain prints no product
    let r = render(&[("blockers".to_string(), Ok(d))], true);
    assert!(r.contains("over_budget: blocked W0706"), "{r}");
    assert!(r.contains("P=?"), "{r}");
    assert!(r.contains("no_call: blocked W0701"), "{r}");
    assert!(
        r.contains("claims by first blocker: W0701 1, W0706 1"),
        "{r}"
    );
}

#[test]
fn a_model_chain_over_the_cost_bound_blocks_every_claim_and_is_reported_once() {
    let mut models = String::from("  model c0(a: U8) -> U8 = a\n");
    for i in 1..=12 {
        models += &format!("  model c{i}(a: U8) -> U8 = c{p}(c{p}(a))\n", p = i - 1);
    }
    let d = one(&v1_doc(&models, "  claim deep:\n    forall a: U8. op_inc(a) == c12(a)\n  claim shallow:\n    forall a: U8. op_inc(a) == inc(a)\n"));
    assert_eq!(d.cost_error.as_ref().unwrap().code, code::LIMIT);
    assert!(d
        .claims
        .iter()
        .all(|c| c.blocked.as_ref().is_some_and(|b| b.code == code::LIMIT)));
    assert!(!d.emittable());
    let r = render(&[("chain".to_string(), Ok(d.clone()))], false);
    assert!(r.contains("models: limit:"), "{r}");
    // the ceiling bound still stops the doubling chain (2^13 nodes < 65,536: it fits)
    let ok = census_of(
        &v1_doc(
            &models,
            "  claim deep:\n    forall a: U8. op_inc(a) == c12(a)\n",
        ),
        &ceiling(),
    )
    .unwrap();
    assert!(ok.cost_error.is_none());
    assert!(ok.emittable());
}

#[test]
fn a_malformed_document_is_an_error_not_a_census() {
    // an unbound name: the S2 checker refuses it
    let src = v1_doc(
        "",
        "  claim bad:\n    forall a: U8. op_inc(a) == nowhere(a)\n",
    );
    let e = census_of(&src, &Profile::default()).unwrap_err();
    assert_eq!(e.code, code::CHECK);
    let report = render(&[("bad".to_string(), Err(e))], false);
    assert!(report.contains("bad: not well formed"));
}

#[test]
fn the_report_groups_refused_items_and_lists_16_2_hits_by_claim() {
    let src = v1_doc(
        "",
        "  claim i5:\n    forall a: U16, b: U16. op_u16(a) == id16(b)\n  claim ok_one:\n    forall a: U8. op_inc(a) == inc(a)\n",
    );
    let docs = vec![("synth".to_string(), census_of(&src, &Profile::default()))];
    let r = render(&docs, true);
    assert!(
        r.contains("synth (enum_forms_v1, v1): 2 claims, 1 emittable at ir.core"),
        "{r}"
    );
    assert!(r.contains("VER04r 16.2 item 5: 1 claims: synth/i5"), "{r}");
    assert!(r.contains("i5: blocked 16.2; P="), "{r}");
    assert!(r.contains("ok_one: ok; P=256 [u8_all]"), "{r}");
    assert!(r.contains("claims by first blocker: 16.2: 1"), "{r}");
    // no hit: the report says so
    let clean = render(
        &[("ct".to_string(), census_of(CT_COMPARE, &Profile::default()))],
        false,
    );
    assert!(
        clean.contains("no claim touches any of the five items"),
        "{clean}"
    );
    assert!(clean.contains("document emittable"));
}
