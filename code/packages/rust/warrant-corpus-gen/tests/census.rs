//! The VER04r 16.2 census (slice S3b): which claims of which adopters can the S3 generator
//! emit at `ir.core`, and why not.
//!
//! VER04r 16.3 item 11e settled for S3 that a binder descriptor of kind `nat_count`, a signed
//! `fx_edge`, a 16-bit `fx_edge` or a list of byte strings is *refused*, and asked S3b for the
//! census S10 needs to decide them.  S10c-2 describes the first three (`nat_shift`, `fx_edge`:
//! D26, D21, D25), so only a list of byte strings (`ir.data`) is still a 16.2 hit.  [`census_document`] reads every claim of a document
//! (the lowering's first refusal, the cost bounds, the S2 domain resolution and the 16.2 scan
//! of the resolved descriptors) and [`render`] prints it.
//!
//! The first test runs the census over **the eleven adopters that bind `gen_evidence`**
//! (`code/packages/rust/*/evidence.bindings`) and prints the report (`cargo test --test
//! census -- --nocapture`); the numbers it prints are quoted in the crate README.  It pins
//! only what must stay true, not the adopters' evolving claim counts: ct-compare and gf256
//! are the two documents the generator emits whole, and every adopter's census must at
//! least be computable and internally consistent.  The remaining tests pin the census logic
//! on small synthetic documents: the settled 16.2 items are described, the open one is reported.

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
fn the_adopters_that_lower_whole_are_the_ones_the_census_calls_emittable() {
    for (name, doc) in adopters() {
        let d = doc.unwrap();
        // The census does not run the evaluator: a document is "emittable" when every item lowers and
        // fits its bounds.  S10c-2 evaluates `ir.fixed`, so x25519 is emittable whole; wasm-leb128 joined
        // with its adopter PR (VER04r [D28] split its two `W0701` claims, so all 44 claims lower).  **S11b
        // lowers `ir.data`**, so arithmetic lowers whole (all 64 claims) and logic-gates lowers all 88
        // claims, which closes CV2-F10 (VER04r 16.14 [D53]; R60); `generate` still refuses both, naming
        // `ir.data`, until S11c-2 (R37), which removed the guard (`ir_data_adopters_lower_and_analyse_since_s11c_2`).
        if ["ct-compare", "gf256", "x25519", "wasm-leb128", "arithmetic"].contains(&name.as_str()) {
            assert!(d.emittable(), "{name}");
            continue;
        }
        // any other adopter is not emittable whole: fail closed (a `W0701` claim, a feature, ...)
        assert!(!d.emittable(), "{name}");
    }
    // logic-gates: every claim lowers; what keeps the document from being whole is its examples, whose shape
    // an `omit_example` line of the corpus profile excuses (the adopter PR's, VER04r 8.4), never a level
    let lg = census_of(
        ADOPTERS.iter().find(|a| a.0 == "logic-gates").unwrap().1,
        &ceiling(),
    )
    .unwrap();
    assert!(lg.claims.iter().all(|c| c.blocked.is_none()));
    assert!(!lg.other_blocks.is_empty());
    assert!(lg
        .other_blocks
        .iter()
        .all(|(_, _, e)| e.code == code::W0701 && !e.is_level()));
}

#[test]
fn ir_data_adopters_lower_and_analyse_since_s11c_2() {
    // R37: S11b lowered, typed and costed `ir.data` and kept `generate` fail-closed; S11c-2 evaluates it and
    // removes the guard.  Arithmetic (records, a payload-free variant) and ct-compare both analyse.
    for name in ["arithmetic", "ct-compare"] {
        let src = ADOPTERS.iter().find(|a| a.0 == name).unwrap().1;
        with_file(src, |f| {
            coding_adventures_warrant_corpus_gen::analyse(f, &ceiling())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        });
    }
}

// -- the pinned histograms (S10b) ---------------------------------------------------------------

/// Claims by first blocker, per adopter, with `max_ir_cost 65536`: **a claim is named by the
/// first construct, in source order, that cannot be lowered, and by `ir.fixed` only when nothing
/// else blocks it** (S10b: the built-ins lower, so a claim that also uses a constructor is named by
/// that level, as the oracle's census names it).
///
/// History.  Before S10b every claim that met a built-in first was `ir.fixed`: the README's old
/// totals were `ir.data` 154, `ir.fixed` 101, `inout` 9, `W0701` 2.  S10b: `ir.data` 79,
/// `ir.fixed` 99, `ir.tuple` 71, `W0701` 8, `inout` 9 and 89 emittable.  S10t-2 lowers `ir.tuple`:
/// `ir.data` 100, `ir.fixed` 130, `W0701` 8, `inout` 9, `16.2` 3 and 105 emittable.  **S10c-2
/// evaluates `ir.fixed` and describes the signed and 16-bit `fx_edge` and `nat_shift`**, so no claim
/// is blocked on `ir.fixed` or `16.2` any more.  Now: `ir.data` 100, `inout` 9, `W0701` 8 and 238
/// emittable (of 355).  **The wasm-leb128 adopter (VER04r [D28]) splits the two `W0701` claims of
/// wasm-leb128 into four `ok` ones**: 357 claims, `ir.data` 100, `inout` 9, `W0701` 6 and 242
/// emittable.  **VOS-039c adds seven oracle-preservation claims to the `-any` contract block**: the
/// `vos-sched-any` document is 15 claims, all `ir.data`, so 364 claims and `ir.data` 107.
///
/// **S11b lowers `ir.data`** (VER04r 16.14 [D46] to [D48]): the name `ir.data` is never a *lowering*
/// refusal any more (R28), so the 107 claims divide as follows (every pin below that moved, and why):
///
/// | adopter | before | now | why |
/// |---|---|---|---|
/// | arithmetic | `ir.data` 55, `ok` 8 | `ok` 63 | records, the payload-free variant and one `match` on it lower (R60: all 63 suites lower) |
/// | logic-gates | `ir.data` 3, `ok` 85 | `ok` 88 | `Option` is a real type; the three claims lower |
/// | hash-functions | `ir.data` 21, `ok` 29, `W0701` 6 | `err-value` 2, `ok` 48, `W0701` 6 | 19 of the 21 lower; the two that use an error value bound by a pattern are the feature `err-value` ([D55], R28) |
/// | vos-sched-any | `ir.data` 15 | `ok` 5, `W0701` 10 | what `ir.data` hid: ten claims break VER04r 4.3 or 4.4 (a claim that calls no op, an impure guard) |
/// | vos-sched-fp-bitmap | `ir.data` 13 | `ok` 11, `W0701` 2 | the same two rules |
///
/// Totals: 364 claims, `ok` 335, `W0701` 18, `err-value` 2, `inout` 9.  Nothing is "emittable" in the sense of
/// `generate` for the `ir.data` adopters (R37): the census counts what **lowers**.
const PINS: [(&str, &[(&str, usize)]); 11] = [
    ("arithmetic", &[("ok", 64)]), // VER04r D76: the 9-binder claim became two of 8
    ("ct-compare", &[("ok", 9)]),
    ("gf256", &[("ok", 38)]),
    (
        "hash-functions",
        &[("W0701", 6), ("err-value", 2), ("ok", 48)],
    ),
    ("logic-gates", &[("ok", 88)]),
    ("lzw", &[("ok", 21)]),
    ("vos-sched-any", &[("W0701", 10), ("ok", 5)]),
    ("vos-sched-fp-bitmap", &[("W0701", 2), ("ok", 11)]),
    ("wasm-leb128", &[("ok", 44)]),
    ("x25519", &[("ok", 8)]),
    ("zeroize", &[("inout", 9)]),
];

/// The oracle's census of the same documents, as of S11a: its **lowering** view, a suite named by the level
/// or feature that refused it at lowering (`LoweredSuite.unsupported` while `lowered` is false:
/// `err-value`), by `W0701` when it lowered with a lowering refusal (`LoweredSuite.refusal`, the impure
/// guard and the claim-calls-no-op rule, VER04r 4.3, 4.4), and `ok` for a suite that lowered, whatever
/// the evaluator then does with it.  (The oracle's *evaluation* view, which `emit.census` reports as
/// `suites_by_level`, still counts a unit that lowered and needs `ir.data` under `ir.data` until S11c-1
/// evaluates it, R37: arithmetic `ir.core` 8 and `ir.data` 55, logic-gates 85 and 3, hash-functions
/// `ir.core` 35, `err-value` 2 and `ir.data` 19; that is the table of `warrant-oracle/tests/test_lower.py`.)
///
/// The numbers come from running the oracle's `lower_document` over the eleven specifications.  S10c-1:
/// `ir.fixed` is evaluated, so no suite names it any more, and the six hash-functions claims that have
/// an impure guard *and* a built-in name the guard in both tools.  **S11a/S11b:** arithmetic is 63 `ok` and
/// logic-gates 88 (R60: from S11a the oracle's row is the generator's, *in the count of what lowers*; the
/// difference R13 (d) recorded is closed, [D53]), hash-functions' 21 `ir.data` claims are 19 `ok` and 2
/// `err-value` (R28), and the two vos-sched documents, which both tools used to count under `ir.data`,
/// show their `W0701` claims.
const ORACLE_PINS: [(&str, &[(&str, usize)]); 11] = [
    ("ct-compare", &[("ok", 9)]),
    ("gf256", &[("ok", 38)]),
    ("wasm-leb128", &[("ok", 44)]),
    ("hash-functions", &[("ok", 48), ("W0701", 6), ("err-value", 2)]),
    ("logic-gates", &[("ok", 88)]),
    ("lzw", &[("ok", 21)]),
    ("arithmetic", &[("ok", 64)]), // VER04r D76: the 9-binder claim became two of 8
    ("x25519", &[("ok", 8)]),
    ("zeroize", &[("ok", 9)]),
    ("vos-sched-any", &[("ok", 5), ("W0701", 10)]),
    ("vos-sched-fp-bitmap", &[("ok", 11), ("W0701", 2)]),
];

fn histogram(name: &str) -> std::collections::BTreeMap<String, usize> {
    let src = ADOPTERS.iter().find(|a| a.0 == name).unwrap().1;
    let d = census_of(src, &ceiling()).unwrap();
    let mut h = std::collections::BTreeMap::new();
    for c in &d.claims {
        *h.entry(c.reason_key()).or_insert(0) += 1;
    }
    h
}

fn pin(rows: &[(&str, usize)]) -> std::collections::BTreeMap<String, usize> {
    rows.iter().map(|(k, n)| (k.to_string(), *n)).collect()
}

#[test]
fn the_first_blocker_histogram_of_every_adopter_is_pinned() {
    for (name, rows) in PINS {
        assert_eq!(histogram(name), pin(rows), "{name}");
    }
    let total: usize = PINS
        .iter()
        .flat_map(|(_, r)| r.iter())
        .map(|(_, n)| n)
        .sum();
    assert_eq!(total, 365); // 364 before VER04r D76 split arithmetic's 9-binder claim in two
}

#[test]
fn the_generator_census_agrees_with_the_oracle_and_every_difference_has_a_named_cause() {
    // Exactly equal for the ten documents whose every claim both tools lower or refuse for the same
    // reason.  S10t-2 closed the tuple gap, CV2-F1 the guard-timing gap, S10c-2 the `ir.fixed` and
    // descriptor gaps (wasm-leb128, hash-functions, lzw and x25519 joined the list), and **S11b the
    // `ir.data` gap** (arithmetic, logic-gates and the two vos-sched documents joined it): CV2-F10 and
    // R13 (d)'s arithmetic difference are closed, both tools lower records, variants and `Option`.
    for name in [
        "arithmetic",
        "ct-compare",
        "gf256",
        "hash-functions",
        "logic-gates",
        "lzw",
        "vos-sched-any",
        "vos-sched-fp-bitmap",
        "wasm-leb128",
        "x25519",
    ] {
        let oracle = ORACLE_PINS.iter().find(|p| p.0 == name).unwrap().1;
        assert_eq!(histogram(name), pin(oracle), "{name}");
    }
    // The one that still differs, differs in one direction only: the generator never *lowers* a
    // claim the oracle refuses.
    let ok = |h: &std::collections::BTreeMap<String, usize>| h.get("ok").copied().unwrap_or(0);
    for (name, oracle) in &ORACLE_PINS {
        let g = histogram(name);
        assert!(ok(&g) <= ok(&pin(oracle)), "{name}");
    }
    // zeroize: `inout` parameters and `ensures` clauses are generator refusals (the oracle lowers
    // all 9 claims): a documented feature gap, not a level.
    let z = histogram("zeroize");
    assert_eq!((z["inout"], z.get("ok").copied().unwrap_or(0)), (9, 0));
    // hash-functions: the two `err-value` claims are [D55]'s error value that a pattern binds and uses
    assert_eq!(histogram("hash-functions")["err-value"], 2);
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
fn the_settled_16_2_items_are_described_and_only_the_list_of_byte_strings_is_reported() {
    // item 5 (S10c-2, D25): two U16 binders are 2^32 ordinals; the budget demotes one to an edge
    // set, which is now a described `fx_edge`, so the claim is emittable
    let d = one(&v1_doc(
        "",
        "  claim i5:\n    forall a: U16, b: U16. op_u16(a) == id16(b)\n",
    ));
    assert!(d.claims[0].s162.is_empty());
    assert!(d.claims[0].emittable());
    assert_eq!(d.claims[0].reason_key(), "ok");
    // item 4 (D21): a signed 64-bit leaf is an `fx_edge` of 623 values
    let d = one(&v1_doc(
        "  model idi(a: I64) -> I64 = a\n",
        "  claim i4:\n    forall a: I64. op_u64(1u64) == id64(1u64) and idi(a) == a\n",
    ));
    assert!(d.claims[0].s162.is_empty());
    assert!(d.claims[0].emittable());
    assert_eq!(
        d.claims[0].domain.as_ref().unwrap(),
        &(623, vec!["fx_edge(I64)".to_string()])
    );
    // item 3 (D26): a Nat that flows into a shift count is `nat_shift`, 73 values
    let d = one(&v1_doc(
        "",
        "  claim i3:\n    forall n: Nat, a: U8. op_inc(a) == inc(shl(a, n))\n",
    ));
    assert!(d.claims[0].s162.is_empty());
    assert_eq!(
        d.claims[0].domain.as_ref().unwrap(),
        &(73 * 256, vec!["nat_shift".to_string(), "u8_all".to_string()])
    );
    assert!(d.claims[0].emittable(), "shl lowers and evaluates since S10c-2");
    // ... and a parenthesised count selects it too (R10: parentheses do not matter)
    let d = one(&v1_doc(
        "",
        "  claim i3p:\n    forall n: Nat, a: U8. op_inc(a) == inc(shl(a, (n)))\n",
    ));
    assert_eq!(d.claims[0].domain.as_ref().unwrap().1[0], "nat_shift");
    // items 1/2: a list of byte strings (`ir.list` as well, R27) is the one item still undecided
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
    assert_eq!(d.claims[0].reason_key(), "ir.list");
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
        "  claim i12:\n    forall l: List<Bytes<2>>. op_inc(1u8) == inc(1u8)\n  claim ok_one:\n    forall a: U8. op_inc(a) == inc(a)\n",
    );
    let docs = vec![("synth".to_string(), census_of(&src, &Profile::default()))];
    let r = render(&docs, true);
    assert!(
        r.contains("synth (enum_forms_v1, v1): 2 claims, 1 emittable at ir.core"),
        "{r}"
    );
    assert!(r.contains("VER04r 16.2 item 1/2: 1 claims: synth/i12"), "{r}");
    assert!(r.contains("i12: blocked level [ir.list]"), "{r}");
    assert!(r.contains("ok_one: ok; P=256 [u8_all]"), "{r}");
    assert!(r.contains("claims by first blocker: ir.list 1, ok 1"), "{r}");
    // no hit: the report says so
    let clean = render(
        &[("ct".to_string(), census_of(CT_COMPARE, &Profile::default()))],
        false,
    );
    assert!(
        clean.contains("no claim touches an undecided item"),
        "{clean}"
    );
    assert!(clean.contains("document emittable"));
}
