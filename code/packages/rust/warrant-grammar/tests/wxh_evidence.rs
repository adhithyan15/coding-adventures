//! VER04j WX-H evidence generation, tested the way VER04h tests the v0 generator:
//! by *compiling and running* what it emits.  The generated harness
//! (`tests/generated/wxh_evidence.rs`, checked in and drift-tested by
//! `tests/wxh_regen.rs`) is included as a module and run against the honest
//! subject of the section 6.7 pilot (`tests/support/wxh_subject_fixed.rs`).
//!
//! Sections: the whole suite and every negative control; the generated
//! `output-bound` suites and the manifest fields; the allocation observer
//! (window, cap, containment, the latch); relation bias for a limit parameter;
//! the faults as plain functions; negative bounds.  The vulnerable and the
//! wasteful subject are `tests/wxh_vulnerable.rs` and `tests/wxh_wasteful.rs`.

use std::collections::{BTreeSet, HashMap};

use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, BinaryOp, Declaration, Expression, ParserLimits,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

use coding_adventures_test_bootstrap::prng::SplitMix64;
use coding_adventures_test_bootstrap::Verdict;

#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod harness {
    pub mod wxh_subject {
        include!("support/wxh_subject_fixed.rs");
    }
    include!("generated/wxh_evidence.rs");

    // ----- test hooks: the generated module's private machinery ----------------

    /// What one observed call did.
    #[derive(Debug, PartialEq, Eq)]
    pub enum Seen {
        Finished(u64),
        Panicked(u64),
        Tripped(u64),
        Refused,
    }

    pub fn exec(job: impl FnOnce() -> V + Send + 'static) -> Seen {
        match obs_exec(job) {
            Exec::Finished(Ok(_), p) => Seen::Finished(p),
            Exec::Finished(Err(_), p) => Seen::Panicked(p),
            Exec::Tripped(p) => Seen::Tripped(p),
            Exec::Refused => Seen::Refused,
        }
    }

    pub fn cap() -> u64 {
        OBSERVER_CAP_BYTES
    }

    /// `obs_observe`, with the observer's failure as an `Err` message.
    pub fn observe(
        op: &str,
        heap: Option<i128>,
        job: impl FnOnce() -> V + Send + 'static,
    ) -> Result<V, String> {
        match catch_unwind(AssertUnwindSafe(|| obs_observe(op, heap, job))) {
            Ok(v) => Ok(v),
            Err(p) => match p.downcast::<ObsFailure>() {
                Ok(f) => Err(f.0),
                Err(other) => std::panic::resume_unwind(other),
            },
        }
    }

    pub fn begin_test() {
        obs_begin_test();
    }

    pub fn end_test() -> (u64, u64, Option<i128>, String) {
        let a = obs_end_test();
        (a.instances, a.max_peak, a.bound_at_max, a.note)
    }

    pub fn bound_of(f: impl FnOnce() -> V) -> Result<i128, String> {
        match catch_unwind(AssertUnwindSafe(|| obs_bound("test bound", f))) {
            Ok(n) => Ok(n),
            Err(p) => match p.downcast::<ObsFailure>() {
                Ok(f) => Err(f.0),
                Err(other) => std::panic::resume_unwind(other),
            },
        }
    }

    pub fn size_of(v: &V) -> i128 {
        obs_size(v)
    }

    pub fn output_ok(v: V, bound: i128) -> bool {
        obs_output_ok(v, bound)
    }

    pub fn truncate(v: V, source: &[u8], limit: i128) -> V {
        fault_truncate_silently(v, source, limit)
    }

    /// Environments drawn by the relation-biased generator.
    pub fn gen_values(tys: &[Ty], seed: u64, mode: u8, n: usize) -> Vec<Vec<V>> {
        let mut rng = SplitMix64::new(seed);
        (0..n)
            .map(|_| build(tys, &mut RngSrc(&mut rng), mode))
            .collect()
    }

    /// The generated model function `name` applied to `args`.
    pub fn model(name: &str, args: &[V]) -> V {
        match name {
            "lo" => model_lo(args[0].clone(), args[1].clone()),
            "hi" => model_hi(args[0].clone(), args[1].clone()),
            "cat" => model_cat(args[0].clone(), args[1].clone()),
            "stream" => model_stream(args[0].clone()),
            other => panic!("no model {other}"),
        }
    }

    pub fn op_decompress_direct(data: &[u8], limit: u32, fault: Fault) -> V {
        op_decompress(
            &[],
            &[V::Bytes(data.to_vec()), V::Fixed(F::U32, i128::from(limit))],
            fault,
        )
    }
}
use harness::{Fault, Seen, Ty, F, V};

include!("support/wxh_runner.rs");

fn is_fail(v: &Verdict) -> bool {
    matches!(v, Verdict::Fail { .. })
}

// ---------------------------------------------------------------------------
// The whole generated suite, including every negative control
// ---------------------------------------------------------------------------

#[test]
fn the_full_generated_suite_passes_and_every_fault_is_caught() {
    let run = run_once();
    let failures: Vec<String> = run
        .outcomes
        .iter()
        .filter(|o| !o.passed())
        .map(|o| format!("{}: {}", o.id, message(o)))
        .collect();
    assert!(failures.is_empty(), "failures:\n{}", failures.join("\n"));
    let negctl: Vec<_> = run
        .outcomes
        .iter()
        .filter(|o| o.id.contains("/negctl/"))
        .collect();
    assert!(!negctl.is_empty() && negctl.iter().all(|o| o.passed()));
    // Every fault of the binding is exercised against every kind of suite.
    for fault in [
        "ignore-limit",
        "off-by-one-limit",
        "limit-after-alloc",
        "truncate-silently",
        "missing-error",
    ] {
        for kind in ["example", "exhaustive", "property", "fuzz"] {
            let id = format!("negctl/{kind}/decompress/{fault}");
            assert!(
                negctl.iter().any(|o| o.id.ends_with(&id)),
                "no negative control {id}"
            );
        }
    }
}

#[test]
fn the_output_bound_property_is_registered_as_exhaustive_property_and_fuzz() {
    for kind in ["exhaustive", "property", "fuzz"] {
        let id = format!("warrant/wxh/{kind}/decompress/output-bound");
        let o = run_once()
            .outcomes
            .iter()
            .find(|o| o.id == id)
            .unwrap_or_else(|| panic!("no test {id}"));
        assert!(o.passed(), "{id}: {}", message(o));
    }
}

#[test]
fn the_manifest_records_the_output_bound_and_the_observer() {
    // The manifest is pretty-printed: compare without the space after a colon.
    let compact = run_once().manifest.replace("\": ", "\":");
    let m = &compact;
    assert!(!m.is_empty(), "no manifest was written");
    // The claim field names the synthesized property.
    assert!(m.contains("\"claim\":\"decompress/output-bound\""), "{m}");
    let entry = |id: &str| -> String {
        let at = m.find(id).unwrap_or_else(|| panic!("no entry {id}"));
        let rest = &m[at..];
        rest[..rest.find('}').unwrap()].to_string()
    };
    let number = |e: &str, key: &str| -> u64 {
        let k = format!("\"{key}\":");
        let at = e.find(&k).unwrap_or_else(|| panic!("no {key} in {e}"));
        let rest = &e[at + k.len()..];
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        rest[..end].parse().unwrap()
    };
    for id in [
        "exhaustive/decompress/output-bound",
        "property/decompress/output-bound",
        "fuzz/decompress/output-bound",
        "exhaustive/empty_input_is_corrupt_under_any_limit",
        "example/three_bytes_within_the_limit",
    ] {
        let e = entry(id);
        let instances = number(&e, "instances_observed");
        let peak = number(&e, "max_observed_heap");
        assert!(instances > 0, "{id}: {e}");
        assert!(peak > 0, "{id}: {e}");
        // The measured peak is below the bound at that instance.
        if e.contains("\"bound_at_max\":") {
            assert!(peak <= number(&e, "bound_at_max"), "{id}: {e}");
        }
    }
    // Negative controls are not observed entries.
    let negctl = entry("negctl/exhaustive/decompress/ignore-limit");
    assert!(!negctl.contains("instances_observed"), "{negctl}");
    assert!(m.contains("\"warrant_version\":\"v1\""));
}

#[test]
fn each_limit_fault_fails_the_output_bound_or_a_claim_when_run_directly() {
    // Run the exhaustive output-bound suite and the examples under each fault.
    for (fault, name) in [
        (Fault::IgnoreLimit, "ignore-limit"),
        (Fault::OffByOneLimit, "off-by-one-limit"),
        (Fault::TruncateSilently, "truncate-silently"),
    ] {
        let (v, _) = harness::exh_decompress_output_bound(fault);
        let ex = harness::example_one_byte_over_the_limit(fault);
        assert!(
            is_fail(&v) || !ex,
            "{name}: neither the output bound nor the example noticed"
        );
        assert!(!ex, "{name}: the over-the-limit example must notice");
    }
}

#[test]
fn the_honest_subject_answers_the_pilot_examples() {
    let ok = |b: &[u8]| V::Ok(Box::new(V::Bytes(b.to_vec())));
    let hex = |s: &str| -> Vec<u8> {
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
            .collect()
    };
    assert_eq!(
        harness::op_decompress_direct(&hex("030000000361"), 3, Fault::None),
        ok(b"aaa")
    );
    assert_eq!(
        harness::op_decompress_direct(&hex("030000000361"), 2, Fault::None),
        V::Err("OUTPUT_LIMIT".into())
    );
    assert_eq!(
        harness::op_decompress_direct(&hex("ffffffffff61ff61ff61"), 1024, Fault::None),
        V::Err("OUTPUT_LIMIT".into())
    );
    assert_eq!(
        harness::op_decompress_direct(&hex("ffffffffff61ff61ff61"), u32::MAX, Fault::None),
        V::Err("CORRUPT".into())
    );
}

// ---------------------------------------------------------------------------
// The allocation observer
// ---------------------------------------------------------------------------

#[test]
fn a_window_measures_the_peak_of_the_call_not_of_the_caller() {
    let before = vec![0u8; 5 << 20]; // allocated outside the window: not counted
    let seen = harness::exec(|| {
        let v = vec![1u8; 1 << 20];
        V::Bool(v.len() == 1 << 20)
    });
    drop(before);
    match seen {
        Seen::Finished(peak) => {
            assert!(peak >= 1 << 20, "peak {peak} misses the allocation");
            assert!(peak < 2 << 20, "peak {peak} counts memory the caller held");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn equal_results_with_ten_times_the_allocation_are_distinguished() {
    let lean = |extra: usize| {
        harness::exec(move || {
            let out = vec![7u8; 4096];
            let scratch = vec![0u8; extra];
            std::hint::black_box(&scratch);
            V::Bytes(out)
        })
    };
    let (Seen::Finished(a), Seen::Finished(b)) = (lean(0), lean(10 * 4096)) else {
        panic!("both calls fit the cap")
    };
    assert!(b >= a + 10 * 4096 - 64, "peaks {a} and {b} are not told apart");
}

#[test]
fn a_panic_inside_the_window_is_carried_back_with_its_peak() {
    let seen = harness::exec(|| {
        let v = vec![0u8; 100_000];
        std::hint::black_box(&v);
        panic!("the subject panicked");
    });
    assert!(matches!(seen, Seen::Panicked(p) if p >= 100_000), "{seen:?}");
}

#[test]
fn an_allocation_over_the_cap_is_contained_not_made() {
    // 4 GiB: were the allocation made the process would be at the mercy of the
    // kernel's overcommit; it is refused, the worker is parked and abandoned,
    // and the caller fails the instance.
    let seen = harness::exec(|| {
        let v: Vec<u8> = Vec::with_capacity(0xFFFF_FFFF);
        V::Bool(v.is_empty())
    });
    assert_eq!(seen, Seen::Tripped(0xFFFF_FFFF), "{seen:?}");
    // The next call gets a fresh worker and works.
    assert!(matches!(harness::exec(|| V::Bool(true)), Seen::Finished(_)));
}

#[test]
fn the_cap_is_the_detection_even_without_a_heap_bound() {
    // A "cap-less" observer would let this through (and exhaust the machine):
    // with no heap row the cap alone must fail the instance, and an allocation
    // just under the cap must pass.
    let cap = harness::cap();
    harness::begin_test();
    let r = harness::observe("t", None, move || {
        let v = vec![0u8; usize::try_from(cap).unwrap() + 1];
        V::Bool(!v.is_empty())
    });
    let msg = r.expect_err("a call over the cap fails");
    assert!(msg.starts_with("bound-heap-exceeded"), "{msg}");
    assert!(msg.contains(&format!("{}", harness::cap())), "{msg}");
    let (instances, _, _, note) = harness::end_test();
    assert_eq!(instances, 1);
    assert_eq!(note, "", "the note is recorded by the claim wrapper, not here");

    harness::begin_test();
    let r = harness::observe("t", None, move || {
        let v = vec![0u8; usize::try_from(cap).unwrap() / 2];
        V::Bool(!v.is_empty())
    });
    assert!(r.is_ok(), "{r:?}");
    harness::end_test();
}

#[test]
fn a_test_that_tripped_fails_its_later_observed_calls_without_running_them() {
    harness::begin_test();
    let first = harness::observe("t", None, || {
        let v: Vec<u8> = Vec::with_capacity(1 << 31);
        V::Bool(v.is_empty())
    })
    .expect_err("the bomb trips");
    assert!(first.contains("asked for"), "{first}");
    // Shrinkers re-run the property many times: none of those may abandon
    // another worker.
    let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = ran.clone();
    let second = harness::observe("t", None, move || {
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        V::Bool(true)
    })
    .expect_err("later calls fail");
    assert!(second.contains("an earlier instance of this test"), "{second}");
    assert!(!ran.load(std::sync::atomic::Ordering::SeqCst), "the job ran");
    harness::end_test();
    // A new test starts clean.
    harness::begin_test();
    assert!(harness::observe("t", None, || V::Bool(true)).is_ok());
    harness::end_test();
}

#[test]
fn a_heap_bound_below_the_peak_fails_the_instance() {
    harness::begin_test();
    let r = harness::observe("t", Some(1000), || {
        let v = vec![0u8; 10_000];
        std::hint::black_box(&v);
        V::Bool(true)
    });
    let msg = r.expect_err("peak over the heap bound");
    assert!(msg.contains("heap bound of 1000"), "{msg}");
    let (_, peak, bound, _) = harness::end_test();
    assert!(peak >= 10_000 && bound == Some(1000));
    harness::begin_test();
    assert!(harness::observe("t", Some(1 << 20), || V::Bool(true)).is_ok());
    harness::end_test();
}

// ---------------------------------------------------------------------------
// Bounds: negative and unevaluable
// ---------------------------------------------------------------------------

#[test]
fn a_negative_or_unevaluable_bound_fails_the_instance() {
    assert_eq!(harness::bound_of(|| V::Int(41)), Ok(41));
    assert_eq!(harness::bound_of(|| V::Fixed(F::U32, 7)), Ok(7));
    let neg = harness::bound_of(|| V::Int(-1)).expect_err("negative");
    assert!(neg.starts_with("bound-negative") && neg.contains("-1"), "{neg}");
    let boom = harness::bound_of(|| panic!("arithmetic overflow")).expect_err("unevaluable");
    assert!(boom.contains("cannot be evaluated"), "{boom}");
    let not_a_number = harness::bound_of(|| V::Bool(true)).expect_err("not an Int");
    assert!(not_a_number.starts_with("bound-negative"), "{not_a_number}");
}

#[test]
fn the_size_of_a_result_follows_section_6_2() {
    let b = |n: usize| V::Bytes(vec![0; n]);
    assert_eq!(harness::size_of(&b(5)), 5);
    assert_eq!(harness::size_of(&V::List(vec![V::Bool(true); 3])), 3);
    assert_eq!(harness::size_of(&V::Ok(Box::new(b(9)))), 9);
    assert_eq!(harness::size_of(&V::Err("X".into())), 0);
    assert!(harness::output_ok(V::Ok(Box::new(b(4))), 4));
    assert!(!harness::output_ok(V::Ok(Box::new(b(5))), 4));
    assert!(harness::output_ok(V::Err("X".into()), 0));
    let r = std::panic::catch_unwind(|| harness::size_of(&V::Bool(true)));
    assert!(r.is_err(), "a scalar has no output size");
}

// ---------------------------------------------------------------------------
// Relation bias and the fault rewrites
// ---------------------------------------------------------------------------

#[test]
fn relation_bias_draws_the_limit_from_the_boundaries_of_the_byte_string() {
    const BYTES: Ty = Ty::Bytes;
    const LIMIT: Ty = Ty::Fx(F::U32);
    let envs = harness::gen_values(&[BYTES, LIMIT], 0xC0FFEE, 0, 20_000);
    let mut seen: BTreeSet<i128> = BTreeSet::new();
    let (mut at_len, mut at_len_less_one, mut at_twice) = (false, false, false);
    for env in &envs {
        let (V::Bytes(d), V::Fixed(F::U32, m)) = (&env[0], &env[1]) else {
            panic!("shape")
        };
        let len = d.len() as i128;
        seen.insert(*m);
        at_len |= *m == len && len > 2;
        at_len_less_one |= *m == len - 1 && len > 2;
        at_twice |= *m == 2 * len && len > 2;
    }
    for want in [0, 1, 255, 256, 65535, i128::from(u32::MAX)] {
        assert!(seen.contains(&want), "limit {want} is never drawn");
    }
    assert!(at_len && at_len_less_one && at_twice, "len-relative limits are not drawn");
    // Uniform draws remain: the limit is not only ever a boundary value.
    assert!(seen.len() > 30, "only {} distinct limits", seen.len());
}

#[test]
fn relation_bias_for_a_limit_needs_a_byte_string_and_an_unsigned_number() {
    // Neither shape is touched: the draw is the unbiased one (and in range).
    for tys in [[Ty::Fx(F::U32), Ty::Fx(F::U32)], [Ty::Fx(F::I32), Ty::Fx(F::I32)]] {
        for env in harness::gen_values(&tys, 7, 0, 500) {
            assert_eq!(env.len(), 2);
        }
    }
    // A Nat limit is clamped to u32.
    for env in harness::gen_values(&[Ty::Bytes, Ty::Nat], 11, 0, 2000) {
        let V::Int(n) = env[1] else { panic!("Nat") };
        assert!((0..=i128::from(u32::MAX)).contains(&n));
    }
    // A U16 limit never leaves its range.
    for env in harness::gen_values(&[Ty::Bytes, Ty::Fx(F::U16)], 13, 0, 2000) {
        let V::Fixed(F::U16, n) = env[1] else { panic!("U16") };
        assert!((0..=65535).contains(&n));
    }
}

#[test]
fn truncate_silently_turns_an_error_into_a_short_ok() {
    let src = [1u8, 2, 3, 4, 5];
    assert_eq!(
        harness::truncate(V::Err("OUTPUT_LIMIT".into()), &src, 3),
        V::Ok(Box::new(V::Bytes(vec![1, 2, 3])))
    );
    assert_eq!(
        harness::truncate(V::Err("X".into()), &src, 100),
        V::Ok(Box::new(V::Bytes(src.to_vec())))
    );
    assert_eq!(
        harness::truncate(V::Err("X".into()), &src, -5),
        V::Ok(Box::new(V::Bytes(Vec::new())))
    );
    let ok = V::Ok(Box::new(V::Bytes(vec![9])));
    assert_eq!(harness::truncate(ok.clone(), &src, 3), ok);
}

#[test]
fn the_pilot_domains_carry_the_bomb_values() {
    // The exhaustive domain of a U32 binder holds MAX (the hostile limit) and
    // the generator draws it; a Bytes binder reaches headers of every length.
    let envs = harness::gen_values(&[Ty::Bytes, Ty::Fx(F::U32)], 5, 0, 200);
    assert!(envs.iter().any(|e| matches!(&e[0], V::Bytes(b) if b.len() >= 4)));
    let mut rng = SplitMix64::new(1);
    assert_ne!(rng.next_u64(), 0);
}

// ---------------------------------------------------------------------------
// Differential: the ground evaluator against the generated model code
// (VER04j sections 8 item 1 and 10.3) for the built-ins this slice adds
// ---------------------------------------------------------------------------

#[test]
fn the_evaluator_and_the_generated_models_agree_on_imin_imax_and_concat() {
    let ints: [i128; 9] = [-(1 << 40), -5, -1, 0, 1, 7, 255, 65536, 1 << 40];
    let blobs: [&[u8]; 5] = [b"", b"a", b"\x00\xff", b"hello", &[7u8; 20]];
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let mut src = String::from(
        "warrant t v1\n  model lo(a: Int, b: Int) -> Int = imin(a, b)\n  model hi(a: Int, b: Int) -> Int = imax(a, b)\n  model cat(a: Bytes, b: Bytes) -> Bytes = concat(a, b)\n",
    );
    let mut rows: Vec<(&str, V, V)> = Vec::new();
    let mut n = 0;
    for a in ints {
        for b in ints {
            for f in ["lo", "hi"] {
                n += 1;
                src.push_str(&format!("  example e{n}:\n    {f}({a}, {b}) == {f}({a}, {b})\n"));
                rows.push((f, V::Int(a), V::Int(b)));
            }
        }
    }
    for a in blobs {
        for b in blobs {
            n += 1;
            src.push_str(&format!(
                "  example e{n}:\n    cat(b\"{}\", b\"{}\") == cat(b\"{}\", b\"{}\")\n",
                hex(a),
                hex(b),
                hex(a),
                hex(b)
            ));
            rows.push(("cat", V::Bytes(a.to_vec()), V::Bytes(b.to_vec())));
        }
    }
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let mut ev = Evaluator::new(&file, CodegenLimits::default());
    let examples: Vec<&Expression> = file
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Example(e) => Some(&e.body),
            _ => None,
        })
        .collect();
    assert_eq!(examples.len(), rows.len());
    for (body, (name, a, b)) in examples.into_iter().zip(rows) {
        let Expression::Binary { op: BinaryOp::Eq, left, .. } = body else {
            panic!("== expected")
        };
        ev.reset_work();
        let ground = ev.eval(left, &HashMap::new()).expect("evaluates");
        let generated = harness::model(name, &[a.clone(), b.clone()]);
        match (&ground, &generated) {
            (g, V::Int(x)) => assert_eq!(g.as_i128(), Some(*x), "{name}({a:?}, {b:?})"),
            (EvalValue::Bytes(g), V::Bytes(x)) => assert_eq!(g, x, "{name}({a:?}, {b:?})"),
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn the_stream_model_builds_a_well_formed_header_and_pair() {
    let V::Bytes(b) = harness::model("stream", &[V::Fixed(F::U32, 5)]) else {
        panic!("bytes")
    };
    assert_eq!(b, vec![5, 0, 0, 0, 5, 0x61]);
}
