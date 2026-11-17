//! Limits and refusals at the seams between the S2 domain API, the planner and the
//! enumerator: the branches a well-formed adopter never reaches but a hostile or careless
//! document can (VER04r 14: every bound is checked before the first case, in checked
//! arithmetic).

mod support;

use coding_adventures_warrant_corpus_gen::analyse;
use coding_adventures_warrant_corpus_gen::domains::{
    claim_domains, describe, kind_name, map_codegen, resolve_claim, s162_items, BinderDomain,
    SuiteDomains,
};
use coding_adventures_warrant_corpus_gen::enumerate_document;
use coding_adventures_warrant_corpus_gen::error::{code, GenError};
use coding_adventures_warrant_corpus_gen::eval::Program;
use coding_adventures_warrant_corpus_gen::profile::{parse_profile, Profile};
use coding_adventures_warrant_corpus_gen::suite::SuitePlan;
use coding_adventures_warrant_corpus_gen::types::Ty;
use coding_adventures_warrant_grammar::codegen::{CodegenError, CodegenErrorKind, DiagnosticCode};
use coding_adventures_warrant_grammar::domain::{Domain, DomainEnv, ErrName};
use coding_adventures_warrant_grammar::fixed::FixedTy;
use support::*;

fn v1_with(extra_models: &str, claims: &str) -> String {
    let head = &ENUM_V1[..ENUM_V1.find("  claim ").unwrap()];
    let head = head.replace(
        "  error E = BAD\n",
        &format!("  error E = BAD\n{extra_models}"),
    );
    format!("{head}{claims}")
}

// ---------------------------------------------------------------------------
// domains
// ---------------------------------------------------------------------------

#[test]
fn a_domain_size_that_overflows_is_w0706_never_zero() {
    // 377^9 > 2^64: the descriptor of such a tuple has no size
    let huge = Domain::Tuple(vec![Domain::U64; 9]);
    let e = describe(&huge, 1).unwrap_err();
    assert_eq!(e.code, code::W0706);
    assert!(e.message.contains("never silently zero"));
}

#[test]
fn kind_names_spell_every_domain_kind() {
    let err = ErrName("E".into());
    let cases = [
        (Domain::Bool, "bool"),
        (Domain::U8, "u8_all"),
        (Domain::Fx(FixedTy::U8), "u8_all"),
        (Domain::U64, "u64_edge"),
        (Domain::Fx(FixedTy::U64), "u64_edge"),
        (Domain::Nat, "nat_0_64"),
        (Domain::NatCount, "nat_count"),
        (Domain::Bytes, "bytes_d1"),
        (Domain::BytesN(4), "bytes_n(4)"),
        (Domain::Fx(FixedTy::I16), "fx_full(I16)"),
        (Domain::Fx(FixedTy::U32), "fx_edge(U32)"),
        (Domain::FxEdge(FixedTy::U16), "fx_edge16(U16)"),
        (
            Domain::Tuple(vec![Domain::Bool, Domain::Nat]),
            "tuple(bool, nat_0_64)",
        ),
        (Domain::Opt(Box::new(Domain::Bool)), "option(bool)"),
        (Domain::List(Box::new(Domain::Bytes)), "list(bytes_d1)"),
        (
            Domain::Res(Box::new(Domain::Bytes), vec!["X".into()], err),
            "result(bytes_d1)",
        ),
        (
            Domain::Data {
                name: "P".into(),
                ctors: vec![("P".into(), vec![Domain::Bool])],
                record: true,
            },
            "record(P)",
        ),
        (
            Domain::Data {
                name: "V".into(),
                ctors: vec![("A".into(), vec![]), ("B".into(), vec![])],
                record: false,
            },
            "variant(V)",
        ),
    ];
    for (d, want) in cases {
        assert_eq!(kind_name(&d), want);
    }
}

#[test]
fn the_16_2_scan_walks_into_records_and_variants() {
    let rec = Domain::Data {
        name: "R".into(),
        ctors: vec![("R".into(), vec![Domain::Bool, Domain::NatCount])],
        record: true,
    };
    assert_eq!(s162_items(&rec), ["3"]);
    let var = Domain::Data {
        name: "V".into(),
        ctors: vec![
            ("A".into(), vec![Domain::FxEdge(FixedTy::I16)]),
            ("B".into(), vec![Domain::Fx(FixedTy::I32)]),
        ],
        record: false,
    };
    let mut items = s162_items(&var);
    items.sort_unstable();
    assert_eq!(items, ["4", "5"]);
    // a composite deeper than the scan's own guard is not followed (and not an error)
    let mut deep = Domain::NatCount;
    for _ in 0..80 {
        deep = Domain::Opt(Box::new(deep));
    }
    assert!(s162_items(&deep).is_empty());
}

#[test]
fn the_s2_resolver_error_codes_map_to_ours() {
    let err = |c| CodegenError {
        code: c,
        kind: CodegenErrorKind::EvaluationError("x".into()),
        span: (0, 0),
    };
    assert_eq!(
        map_codegen(&err(DiagnosticCode::W0706), "c").code,
        code::W0706
    );
    assert_eq!(
        map_codegen(&err(DiagnosticCode::W0707), "c").code,
        code::W0707
    );
    assert_eq!(
        map_codegen(&err(DiagnosticCode::W0709), "c").code,
        code::W0701
    );
    let e = map_codegen(&err(DiagnosticCode::W0706), "claim");
    assert_eq!(e.at, "claim");
    // our own sentence, never the resolver's text
    assert!(!e.message.contains("EvaluationError"));
}

#[test]
fn an_over_wide_bytes_binder_is_refused_before_the_resolver_sees_it() {
    let src = v1_with(
        "",
        "  claim wide:\n    forall a: Bytes<5000>, b: Bytes<5000>. op_cat(a, b) == op_cat(b, a)\n",
    );
    let e = with_file(&src, |f| {
        enumerate_document(f, &Profile::default()).unwrap_err()
    });
    // the lowering's type resolution refuses it first (a `type` error); the resolver's W0707
    // is reachable only through `map_codegen`, which the test above pins
    assert_eq!(e.code, code::TYPE, "{e}");
    assert!(e.message.contains("4096"), "{e}");
}

#[test]
fn resolution_refuses_a_missing_claim_too_many_binders_and_a_huge_product() {
    let src = v1_with(
        "",
        "  claim nine:\n    forall a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool, i: Bool. op_inc(1u8) == inc(1u8)\n  claim four:\n    forall a: Bytes, b: Bytes, c: Bytes, d: Bytes. op_cat(a, b) == op_cat(c, d)\n  claim one:\n    forall a: U8. op_inc(a) == inc(a)\n",
    );
    with_file(&src, |file| {
        let p = Profile::default();
        let an = analyse(file, &p).unwrap();
        let env = DomainEnv::new(file);
        let suite = |name: &str| {
            an.lowered
                .suites
                .iter()
                .find(|s| s.claim == name)
                .unwrap()
                .clone()
        };
        // not in the document
        let mut ghost = suite("one");
        ghost.claim = "ghost".into();
        let e = resolve_claim(file, &env, &ghost, p.budget).unwrap_err();
        assert_eq!((e.code, e.at.as_str()), (code::W0701, "ghost"));
        assert_eq!(
            claim_domains(file, &env, "ghost", p.budget)
                .unwrap_err()
                .code,
            code::W0701
        );
        // more than 8 binders
        let e = resolve_claim(file, &env, &suite("nine"), p.budget).unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        assert!(e.message.contains("8 binders"));
        // 347^4 = 1.4e10 ordinals: past 2^28, even when the budget allows it
        let e = resolve_claim(file, &env, &suite("four"), u64::MAX).unwrap_err();
        assert_eq!(e.code, code::LIMIT);
        assert!(e.message.contains("2^28"));
        // the lowered binders and the resolved ones must agree
        let mut skewed = suite("one");
        skewed.binders.push(("extra".into(), Ty::Bool));
        let e = resolve_claim(file, &env, &skewed, p.budget).unwrap_err();
        assert_eq!(e.code, code::W0701);
        assert!(e.message.contains("disagree"));
        // and the happy path resolves
        let ok = resolve_claim(file, &env, &suite("one"), p.budget).unwrap();
        assert_eq!((ok.binders.len(), ok.product), (1, 256));
    });
}

// ---------------------------------------------------------------------------
// the planner
// ---------------------------------------------------------------------------

#[test]
fn the_planner_checks_binders_chunk_size_and_the_product_it_is_handed() {
    with_file(ENUM_V1, |file| {
        let p = parse_profile(b"chunk_size 1024\n").unwrap();
        let an = analyse(file, &p).unwrap();
        let program = Program::new(&an.lowered.models);
        let env = DomainEnv::new(file);
        let i = an
            .lowered
            .suites
            .iter()
            .position(|s| s.claim == "add_wraps")
            .unwrap();
        let suite = &an.lowered.suites[i];
        let cost = an.suite_costs[i].total;
        let fresh = || resolve_claim(file, &env, suite, p.budget).unwrap();
        // happy
        let plan = SuitePlan::new(&program, suite, fresh(), cost, 1024).unwrap();
        assert_eq!(plan.chunks_total(), 64);
        assert_eq!(plan.product(), 65_536);
        // a chunk size outside 2^10..2^16
        for bad in [512u64, 3000, 1 << 17, 0] {
            let e = SuitePlan::new(&program, suite, fresh(), cost, bad)
                .err()
                .unwrap();
            assert_eq!(e.code, code::LIMIT, "{bad}");
        }
        // a product that is not the product of the sizes
        let mut skewed = fresh();
        skewed.product += 1;
        let e = SuitePlan::new(&program, suite, skewed, cost, 1024)
            .err()
            .unwrap();
        assert_eq!(e.code, code::W0706);
        // more than 8 binders handed to the planner directly
        let binder = |n: &str| BinderDomain::new(n, &Ty::Bool, Domain::Bool).unwrap();
        let nine = SuiteDomains {
            binders: (0..9).map(|i| binder(&format!("b{i}"))).collect(),
            product: 512,
        };
        let e = SuitePlan::new(&program, suite, nine, cost, 1024)
            .err()
            .unwrap();
        assert_eq!(e.code, code::LIMIT);
        assert!(e.message.contains("8 binders"));
        // a chunk beyond the last
        assert!(plan.run_chunk(64, None).is_err());
        assert!(plan.chunk_bounds(64).is_err());
        assert_eq!(plan.chunk_bounds(63).unwrap(), (63 * 1024, 65_536));
    });
}

#[test]
fn the_total_ir_work_of_a_full_run_is_bounded_before_the_first_case() {
    // A model chain costing about 8,200 nodes on 2^24 ordinals is 1.4e11 node evaluations
    // per suite; the full-run bound is 8.59e12, so the 63rd such suite is refused, with
    // not one case evaluated (the claims are never run).
    let mut models = String::from("  model c0(a: U8) -> U8 = a\n");
    for i in 1..=12 {
        models += &format!("  model c{i}(a: U8) -> U8 = c{p}(c{p}(a))\n", p = i - 1);
    }
    let mut claims = String::new();
    for k in 0..70 {
        claims += &format!("  claim w{k}:\n    forall a: U8, b: U8, c: U8. op_inc(a) == c12(b)\n");
    }
    let src = v1_with(&models, &claims);
    let p = parse_profile(b"max_ir_cost 65536\n").unwrap();
    let t = std::time::Instant::now();
    let e = with_file(&src, |f| enumerate_document(f, &p).unwrap_err());
    assert_eq!(e.code, code::LIMIT);
    assert!(e.message.contains("IR work"), "{e}");
    assert!(t.elapsed().as_secs() < 30);
}

#[test]
fn the_total_product_ordinals_of_a_full_run_are_bounded_at_2_to_the_32() {
    // 257 cheap suites of 2^24 ordinals each pass 2^32 on the 257th
    let mut claims = String::new();
    for k in 0..257 {
        claims += &format!("  claim w{k}:\n    forall a: U8, b: U8, c: U8. op_inc(a) == inc(b)\n");
    }
    let src = v1_with("", &claims);
    let e = with_file(&src, |f| {
        enumerate_document(f, &Profile::default()).unwrap_err()
    });
    assert_eq!(e.code, code::LIMIT);
    assert!(e.message.contains("2^32"), "{e}");
    assert_eq!(e.at, "w256");
}

#[test]
fn totals_over_a_small_document() {
    let e = with_file(ENUM_V0, |f| {
        enumerate_document(f, &Profile::default()).unwrap()
    });
    assert_eq!(e.cases(), e.suites.iter().map(|s| s.cases).sum::<u64>());
    assert_eq!(e.expected_calls(), e.cases());
    assert_eq!(
        e.discarded(),
        e.suites.iter().map(|s| s.discarded).sum::<u64>()
    );
    // the profile's claim of a slim_chunk is validated against the chunks that exist
    let err: GenError = {
        let p = parse_profile(b"slim_chunk small_add 9\n").unwrap();
        with_file(ENUM_V0, |f| enumerate_document(f, &p).unwrap_err())
    };
    assert_eq!(err.code, code::PROFILE);
}

// ---------------------------------------------------------------------------
// the evaluator at the documented bounds, on a default test thread (H1)
// ---------------------------------------------------------------------------

/// `nm` chained models `c_i(x) = c_{i-1}(x) + 1u8 (plus times)` and one claim that calls the
/// last: `nm` active model frames, each `plus + 2` IR levels deep.
fn chain_doc(nm: usize, plus: usize) -> String {
    let mut models = String::new();
    for i in 0..nm {
        let inner = if i == 0 {
            "x".to_string()
        } else {
            format!("c{}(x)", i - 1)
        };
        models += &format!(
            "  model c{i}(x: U8) -> U8 = {inner}{}\n",
            " + 1u8".repeat(plus)
        );
    }
    v1_with(
        &models,
        &format!(
            "  claim deep:\n    forall a: U8. op_inc(a) == c{}(a)\n",
            nm - 1
        ),
    )
}

#[test]
fn the_evaluator_runs_64_model_frames_of_deep_expressions_on_a_default_test_thread() {
    // This is the real bound of VER04r 14 / VER04 section 2: 64 active model frames, each
    // with a deep expression, evaluated per case in a DEBUG build on the harness's default
    // 2 MiB test thread.  A recursive evaluator overflowed the stack at 8 frames here.
    let ceiling = parse_profile(b"max_ir_cost 65536\n").unwrap();
    for (nm, plus, p) in [
        (64, 0, Profile::default()),
        (64, 40, ceiling.clone()),
        (64, 55, ceiling),
    ] {
        let src = chain_doc(nm, plus);
        let e = with_file(&src, |f| enumerate_document(f, &p).unwrap());
        let s = &e.suites[0];
        assert_eq!(s.cases, 256, "{nm} {plus}");
    }
    // one frame more is a limit error, not a crash
    let e = with_file(&chain_doc(65, 0), |f| {
        enumerate_document(f, &Profile::default()).unwrap_err()
    });
    assert_eq!(e.code, code::LIMIT);
    assert!(e.message.contains("64 frames"), "{e}");
}

#[test]
fn a_pure_subject_argument_that_leaves_the_exact_window_is_refused_at_generation() {
    // M1: with no fixed-width arithmetic the body used to be skipped, so this generated
    // cleanly and every lane then hit `ir-overflow`
    let src = v1_with(
        "  model huge(n: Nat) -> Int = n * 18446744073709551615 * 18446744073709551615\n",
        "  claim far:\n    forall a: Bytes. op_len(take(a, huge(len(a)))) == len(a)\n",
    );
    let e = with_file(&src, |f| {
        enumerate_document(f, &Profile::default()).unwrap_err()
    });
    assert_eq!(e.code, code::W0701);
    assert!(e.message.contains("2^127"), "{e}");
}

#[test]
fn a_pure_subject_argument_over_max_ir_value_is_refused_at_generation() {
    // m0(x) = x; m_k(x) = m_{k-1}(concat(x, x)): 14 doublings of an 8-byte input pass 65,536
    let mut models = String::from("  model m0(x: Bytes) -> Bytes = x\n");
    for k in 1..=14 {
        models += &format!(
            "  model m{k}(x: Bytes) -> Bytes = m{}(concat(x, x))\n",
            k - 1
        );
    }
    let src = v1_with(
        &models,
        "  claim grow:\n    forall a: Bytes. op_len(m14(a)) == len(a)\n",
    );
    let e = with_file(&src, |f| {
        enumerate_document(f, &Profile::default()).unwrap_err()
    });
    assert_eq!(e.code, code::W0701);
    assert!(e.message.contains("MAX_IR_VALUE"), "{e}");
}

#[test]
fn every_bound_is_charged_before_any_suite_is_planned() {
    // M3: suite 0 admits nothing (a planning refusal, after its admission table is built);
    // later suites blow the total IR-work bound.  The bound is charged for ALL suites first,
    // so the refusal is the limit, and no suite was planned.
    let mut models = String::from("  model c0(a: U8) -> U8 = a\n");
    for i in 1..=12 {
        models += &format!("  model c{i}(a: U8) -> U8 = c{p}(c{p}(a))\n", p = i - 1);
    }
    let mut claims =
        String::from("  claim none:\n    forall a: U8 | a > 255u8. op_inc(a) == inc(a)\n");
    for k in 0..70 {
        claims += &format!("  claim w{k}:\n    forall a: U8, b: U8, c: U8. op_inc(a) == c12(b)\n");
    }
    let p = parse_profile(b"max_ir_cost 65536\n").unwrap();
    let e = with_file(&v1_with(&models, &claims), |f| {
        enumerate_document(f, &p).unwrap_err()
    });
    assert_eq!(e.code, code::LIMIT, "{e}");
    assert!(e.message.contains("IR work"), "{e}");
}
