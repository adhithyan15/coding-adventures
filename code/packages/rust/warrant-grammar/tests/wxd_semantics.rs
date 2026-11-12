//! VER04k sections 2.2, 2.6, 2.7, 2.12 and 2.15 ("Evaluator differential"):
//! the meaning *and the cost* of the seven D1 forms in the ground evaluator
//! (`codegen::Evaluator`), held against native Rust.
//!
//! ```text
//!   form             value oracle (native Rust)            work        data (bytes)
//!   take(xs, n)      xs.iter().take(clamp(n))              nodes + L   L (Bytes) or
//!                                                                      sum (32 + dsize(e)) (List)
//!   drop(xs, n)      xs.iter().skip(clamp(n))              nodes + L   ditto
//!   slice(xs, s, n)  xs.iter().skip(clamp(s)).take(..)     nodes + L   ditto
//!   replicate(x, n)  iter::repeat(x).take(max(0, n))       nodes + L   L x (32 + dsize(x))
//!   range(n)         (0..max(0, n))                        nodes + L   L x 32
//!   to_list(b)       b.iter()                              nodes + L   L x 32
//!   to_bytes(xs)     xs.iter() as u8                       nodes + L   L x 1
//! ```
//!
//! where `clamp(n) = max(0, min(n, len(xs)))`, `dsize` is the data size of
//! VER04k section 2.7 (a scalar 0, `Bytes` its length, a container
//! `sum (32 + dsize(c))`), and `nodes` is the existing one-unit-per-AST-node
//! charge (the call node and its argument nodes).
//!
//! Also pinned, per the S0 errata (VER04k section 9.1): the `dsize` charge of
//! every constructing node and none for pass-through nodes (E4), the deep-copy
//! charge (E5), `((x))` = 3 units and the 64-level depth limit through groups
//! and model calls (E1, E3), both caps (E6), and `replicate` of a 64 KiB binder
//! on either side of the data budget (1,023 copies fit, 1,024 do not).
//!
//! Not here, by design: the quadratic-accumulator *work* boundary of section
//! 2.15 needs `fold` (WX-D2), and the admission meter of E14 belongs to the
//! generated code (S2b); the ground evaluator does not evaluate admission
//! predicates.  Every result is checked against the oracle over **every list of
//! length 0..=4 on a 3-symbol alphabet** and the same strings as `Bytes`, with
//! the counts `-2 -1 0 1 2 len-1 len len+1 2^70`; the boundary sizes 1,000,
//! 65,536 and 65,537 elements; and pinned (work, data) for each form.
//!
//! The evaluator mutants of section 2.12 that D1 can express -- `take` without
//! the negative clamp (Python's `xs[:n]`), and `range(n)` including `n` -- are
//! written out below as alternative oracles, and the same sweep must **fail**
//! against each of them, so the sweep is known to be able to fail.
//!
//! Finally the v1-only short-circuit of `and`, `or` and `=>` (decided for
//! WX-D1: VER04 section 4 says short-circuit, the generated code always did, and
//! the ground evaluator now agrees in v1) and the unchanged eager v0 behaviour.

use std::collections::HashMap;

use coding_adventures_warrant_grammar::codegen::{
    CodegenErrorKind, CodegenLimits, DiagnosticCode as CgCode, EvalValue, Evaluator,
    DATA_BUDGET_BYTES, MAX_SEQ_ELEMENTS,
};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, Declaration, Expression, ParserLimits, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

/// 2^70: far past every host integer a lane might narrow a count to.
const HUGE: i128 = 1 << 70;
/// The way a document writes 2^70 (an integer literal is at most a u64).
const HUGE_TEXT: &str = "4294967296 * 4294967296 * 64";
/// The three symbols of the alphabet.
const ALPHABET: [u8; 3] = [0, 7, 255];

type Env = HashMap<&'static str, EvalValue>;

fn with_doc<R>(src: &str, f: impl FnOnce(&WarrantFile) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default())
        .unwrap_or_else(|e| panic!("parse: {e}\n{src}"));
    f(&file)
}

/// A document whose examples are exactly `bodies` (parsed, never checked:
/// the evaluator is exercised on its own, as `tests/wx_semantics.rs` does).
fn doc_of(header: &str, bodies: &[&str]) -> String {
    let mut s = format!("warrant sweep{header}\n");
    for (i, b) in bodies.iter().enumerate() {
        s.push_str(&format!("  example e{i}:\n    {b}\n"));
    }
    s
}

fn bodies<'a>(file: &'a WarrantFile<'a>) -> Vec<&'a Expression<'a>> {
    file.declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Example(e) => Some(&e.body),
            _ => None,
        })
        .collect()
}

/// Every string of length 0..=4 over [`ALPHABET`]: 1 + 3 + 9 + 27 + 81 = 121.
fn all_strings() -> Vec<Vec<u8>> {
    let mut out = vec![Vec::new()];
    let mut layer = vec![Vec::new()];
    for _ in 0..4 {
        let mut next = Vec::new();
        for s in &layer {
            for &a in &ALPHABET {
                let mut t: Vec<u8> = s.clone();
                t.push(a);
                next.push(t);
            }
        }
        out.extend(next.iter().cloned());
        layer = next;
    }
    assert_eq!(out.len(), 121);
    out
}

/// The counts of section 2.15 for a sequence of length `len`.
fn counts(len: usize) -> Vec<i128> {
    let l = len as i128;
    let mut c = vec![-2, -1, 0, 1, 2, l - 1, l, l + 1, HUGE];
    c.sort_unstable();
    c.dedup();
    c
}

fn u8_list(s: &[u8]) -> EvalValue {
    EvalValue::List(s.iter().map(|&x| EvalValue::U8(x)).collect())
}

/// `clamp(n) = max(0, min(n, len))`, on the exact value.
fn clamp(n: i128, len: usize) -> usize {
    if n <= 0 {
        0
    } else if n >= len as i128 {
        len
    } else {
        n as usize
    }
}

/// What the evaluator answered, reduced to the comparable outcome.
#[derive(Debug, PartialEq)]
enum Outcome {
    Value(EvalValue),
    /// W0703 with the exhaustion kind.
    Exhausted(CodegenErrorKind),
}

fn run<'a>(ev: &mut Evaluator<'a>, e: &'a Expression<'a>, env: &Env) -> Outcome {
    ev.reset_work();
    match ev.eval(e, env) {
        Ok(v) => Outcome::Value(v),
        Err(err) if err.code == CgCode::W0703 => Outcome::Exhausted(err.kind),
        Err(err) => panic!("unexpected {err}"),
    }
}

// ---------------------------------------------------------------------------
// The oracle and its mutants
// ---------------------------------------------------------------------------

/// A value oracle for the sequence forms over `u8` strings.  Each form takes
/// the input string and the counts, and returns the expected *elements* (the
/// caller wraps them as a list or as bytes).
struct Oracle {
    take: fn(&[u8], i128) -> Vec<u8>,
    drop: fn(&[u8], i128) -> Vec<u8>,
    range: fn(i128) -> Vec<u64>,
}

/// The specification: native iterators over the clamped count.
const SPEC: Oracle = Oracle {
    take: |xs, n| xs.iter().take(clamp(n, xs.len())).copied().collect(),
    drop: |xs, n| xs.iter().skip(clamp(n, xs.len())).copied().collect(),
    range: |n| (0..clamp(n, usize::MAX) as u64).collect(),
};

/// Mutant: `take` without the negative clamp, i.e. Python's `xs[:n]`, where a
/// negative `n` counts from the end (the lane hazard of section 2.13).
const TAKE_NO_CLAMP: Oracle = Oracle {
    take: |xs, n| {
        let len = xs.len() as i128;
        let end = if n < 0 { (len + n).max(0) } else { n.min(len) };
        xs[..end as usize].to_vec()
    },
    ..SPEC
};

/// Mutant: `range(n)` that includes `n` (Ruby/Elixir/Kotlin `0..n`).
const RANGE_INCLUSIVE: Oracle = Oracle {
    range: |n| {
        if n < 0 {
            Vec::new()
        } else {
            (0..=n.min(1 << 20) as u64).collect()
        }
    },
    ..SPEC
};

/// Run the full small-domain sweep against `oracle` and return how many cases
/// disagree.  The specification oracle must give 0, each mutant more than 0.
fn sweep(oracle: &Oracle) -> usize {
    let exprs = [
        "take(xs, n)",
        "drop(xs, n)",
        "slice(xs, s, n)",
        "take(b, n)",
        "drop(b, n)",
        "slice(b, s, n)",
        "to_list(b)",
        "to_bytes(xs)",
        "range(n)",
        "replicate(x, n)",
    ];
    let src = doc_of(" v1", &exprs);
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let mut mismatches = 0;
        let mut check = |got: Outcome, want: Outcome| {
            if got != want {
                mismatches += 1;
            }
        };
        let list = |v: Vec<u8>| Outcome::Value(u8_list(&v));
        let bytes = |v: Vec<u8>| Outcome::Value(EvalValue::Bytes(v));
        for s in all_strings() {
            let len = s.len();
            for n in counts(len) {
                let env: Env = HashMap::from([
                    ("xs", u8_list(&s)),
                    ("b", EvalValue::Bytes(s.clone())),
                    ("n", EvalValue::Int(n)),
                ]);
                check(run(&mut ev, e[0], &env), list((oracle.take)(&s, n)));
                check(run(&mut ev, e[1], &env), list((oracle.drop)(&s, n)));
                check(run(&mut ev, e[3], &env), bytes((oracle.take)(&s, n)));
                check(run(&mut ev, e[4], &env), bytes((oracle.drop)(&s, n)));
                for start in counts(len) {
                    let mut env = env.clone();
                    env.insert("s", EvalValue::Int(start));
                    let want = (oracle.take)(&(oracle.drop)(&s, start), n);
                    check(run(&mut ev, e[2], &env), list(want.clone()));
                    check(run(&mut ev, e[5], &env), bytes(want));
                }
            }
            let env: Env = HashMap::from([("xs", u8_list(&s)), ("b", EvalValue::Bytes(s.clone()))]);
            check(run(&mut ev, e[6], &env), list(s.clone()));
            check(run(&mut ev, e[7], &env), bytes(s.clone()));
        }
        for n in [-2, -1, 0, 1, 2, 3, 4] {
            let env: Env = HashMap::from([("n", EvalValue::Int(n))]);
            let want = (oracle.range)(n).into_iter().map(EvalValue::Nat).collect();
            check(
                run(&mut ev, e[8], &env),
                Outcome::Value(EvalValue::List(want)),
            );
            for &x in &ALPHABET {
                let env: Env = HashMap::from([("n", EvalValue::Int(n)), ("x", EvalValue::U8(x))]);
                let want = std::iter::repeat_n(x, clamp(n, usize::MAX)).collect();
                check(run(&mut ev, e[9], &env), list(want));
            }
        }
        mismatches
    })
}

#[test]
fn every_form_agrees_with_native_rust_on_every_small_input() {
    assert_eq!(sweep(&SPEC), 0);
}

#[test]
fn the_sweep_kills_take_without_the_clamp_and_range_including_n() {
    assert!(sweep(&TAKE_NO_CLAMP) > 0, "take-without-clamp survived");
    assert!(sweep(&RANGE_INCLUSIVE) > 0, "range-including-n survived");
}

// ---------------------------------------------------------------------------
// Huge counts, the element cap and the boundary sizes
// ---------------------------------------------------------------------------

#[test]
fn a_huge_count_clamps_for_take_drop_and_slice_and_exhausts_for_replicate_and_range() {
    let src = doc_of(
        " v1",
        &[
            &format!("take(xs, {HUGE_TEXT})"),
            &format!("drop(xs, {HUGE_TEXT})"),
            &format!("slice(xs, 1, {HUGE_TEXT})"),
            &format!("slice(xs, {HUGE_TEXT}, 1)"),
            &format!("replicate(0u8, {HUGE_TEXT})"),
            &format!("range({HUGE_TEXT})"),
            &format!("range(-{HUGE_TEXT})"),
            &format!("replicate(0u8, -{HUGE_TEXT})"),
        ],
    );
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let env: Env = HashMap::from([("xs", u8_list(&[1, 2, 3]))]);
        assert_eq!(
            run(&mut ev, e[0], &env),
            Outcome::Value(u8_list(&[1, 2, 3]))
        );
        assert_eq!(run(&mut ev, e[1], &env), Outcome::Value(u8_list(&[])));
        assert_eq!(run(&mut ev, e[2], &env), Outcome::Value(u8_list(&[2, 3])));
        assert_eq!(run(&mut ev, e[3], &env), Outcome::Value(u8_list(&[])));
        for i in [4, 5] {
            assert_eq!(
                run(&mut ev, e[i], &env),
                Outcome::Exhausted(CodegenErrorKind::SequenceTooLong),
                "example {i}"
            );
        }
        assert_eq!(
            run(&mut ev, e[6], &env),
            Outcome::Value(EvalValue::List(vec![]))
        );
        assert_eq!(
            run(&mut ev, e[7], &env),
            Outcome::Value(EvalValue::List(vec![]))
        );
    });
}

#[test]
fn sequences_of_1000_and_65536_elements_work_and_65537_is_refused() {
    assert_eq!(MAX_SEQ_ELEMENTS, 65_536);
    let src = doc_of(
        " v1",
        &[
            "range(n)",
            "replicate(7u8, n)",
            "to_list(b)",
            "to_bytes(xs)",
            "take(xs, n - 1)",
            "drop(b, 1)",
        ],
    );
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        for size in [1000usize, 65_536] {
            let raw: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
            let env: Env = HashMap::from([
                ("n", EvalValue::Int(size as i128)),
                ("b", EvalValue::Bytes(raw.clone())),
                ("xs", u8_list(&raw)),
            ]);
            let want_range: Vec<EvalValue> = (0..size as u64).map(EvalValue::Nat).collect();
            assert_eq!(
                run(&mut ev, e[0], &env),
                Outcome::Value(EvalValue::List(want_range))
            );
            assert_eq!(
                run(&mut ev, e[1], &env),
                Outcome::Value(u8_list(&vec![7; size]))
            );
            assert_eq!(run(&mut ev, e[2], &env), Outcome::Value(u8_list(&raw)));
            assert_eq!(
                run(&mut ev, e[3], &env),
                Outcome::Value(EvalValue::Bytes(raw.clone()))
            );
            assert_eq!(
                run(&mut ev, e[4], &env),
                Outcome::Value(u8_list(&raw[..size - 1]))
            );
            assert_eq!(
                run(&mut ev, e[5], &env),
                Outcome::Value(EvalValue::Bytes(raw[1..].to_vec()))
            );
        }
        let env: Env = HashMap::from([("n", EvalValue::Int(65_537))]);
        for i in [0, 1] {
            assert_eq!(
                run(&mut ev, e[i], &env),
                Outcome::Exhausted(CodegenErrorKind::SequenceTooLong)
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Pinned work and data
// ---------------------------------------------------------------------------

/// `(expression, env, work, data)`: the existing one unit per AST node, plus
/// `L` work and the section 2.7 data reservation for the form.
#[test]
fn work_and_data_are_pinned_per_form() {
    let xs = u8_list(&[1, 2, 3, 4, 5]);
    let b = EvalValue::Bytes(vec![1, 2, 3, 4, 5]);
    let nested = EvalValue::List(vec![
        EvalValue::Bytes(vec![1, 2]),
        EvalValue::Bytes(vec![3]),
        EvalValue::Bytes(vec![]),
    ]);
    // (body, work, data); nodes are counted in the comment.
    let cases: &[(&str, usize, u64)] = &[
        // call + xs + literal = 3 nodes, L = 2: 2 x 32 bytes.
        ("take(xs, 2)", 3 + 2, 2 * 32),
        ("take(b, 2)", 3 + 2, 2),
        ("drop(xs, 2)", 3 + 3, 3 * 32),
        ("drop(b, -1)", 4 + 5, 5),
        // call + xs + 2 literals = 4 nodes, L = 2.
        ("slice(xs, 1, 2)", 4 + 2, 2 * 32),
        ("slice(b, 4, 9)", 4 + 1, 1),
        ("replicate(9u8, 4)", 3 + 4, 4 * 32),
        ("replicate(9u8, -4)", 4, 0),
        ("range(6)", 2 + 6, 6 * 32),
        ("range(0)", 2, 0),
        ("to_list(b)", 2 + 5, 5 * 32),
        ("to_bytes(xs)", 2 + 5, 5),
        // Non-scalar elements are copied: each carries its payload
        // (VER04k section 9.1, S0 E5: sum of (32 + dsize(e)) over the copies).
        ("take(nested, 2)", 3 + 2, (32 + 2) + (32 + 1)),
        ("drop(nested, 1)", 3 + 2, (32 + 1) + 32),
        // b"0102" is a 2-byte literal (2 bytes of data), copied 3 times.
        ("replicate(b\"0102\", 3)", 3 + 3, 2 + 3 * (32 + 2)),
        ("replicate(xs, 2)", 3 + 2, 2 * (32 + 5 * 32)),
    ];
    let texts: Vec<&str> = cases.iter().map(|c| c.0).collect();
    let src = doc_of(" v1", &texts);
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let env: Env = HashMap::from([("xs", xs), ("b", b), ("nested", nested)]);
        for (i, (text, work, data)) in cases.iter().enumerate() {
            ev.reset_work();
            ev.eval(e[i], &env)
                .unwrap_or_else(|err| panic!("{text}: {err}"));
            assert_eq!(ev.work_used(), *work, "work of {text}");
            assert_eq!(ev.data_used(), *data, "data of {text}");
        }
    });
}

#[test]
fn every_constructing_node_charges_its_dsize_in_v1_and_pass_through_nodes_charge_nothing() {
    // VER04k section 2.7 / S0 E4: in a v1 evaluation every node charges
    // `dsize` of its value, except the pass-through nodes (references,
    // projections, `if`, `match`, groups, model calls, `unwrap_or`).
    //   dsize: scalar 0; Bytes len; List/tuple/fields sum(32 + dsize);
    //          Some(x)/Ok(x) 32 + dsize(x); and (E20) a constructor or an
    //          error value `E.C` also the bytes of the two names it stores.
    // b = 1,000 bytes, o = Some(3 bytes), t = (5 bytes, 1u8).
    let cases: &[(&str, u64)] = &[
        ("[1u8, 2u8]", 2 * 32),
        ("b\"0102\"", 2),
        ("concat(b\"01\", b\"0203\")", 1 + 2 + 3),
        ("concat([1u8], [2u8])", 32 + 32 + 2 * 32),
        // literals 1 + 1, the two lists 33 + 33, concat copies both: 66.
        ("concat([b\"01\"], [b\"02\"])", (1 + 33) + (1 + 33) + 66),
        // inner lists 64 + 32, the outer (32 + 64) + (32 + 32).
        ("[[1u8, 2u8], [3u8]]", 64 + 32 + (32 + 64) + (32 + 32)),
        ("[b\"0102\", b\"03\"]", 2 + 1 + (32 + 2) + (32 + 1)),
        ("(1, 2)", 2 * 32),
        ("(b, 1)", (32 + 1000) + 32),
        // E20: record `P` stores its type and constructor names, `P` and `P`.
        ("P(1u8, b\"0102\")", 2 + (1 + 1) + (32 + (32 + 2))),
        // A nullary constructor is its names: `Sh` + `Dot`.
        ("Dot", 2 + 3),
        ("Box2(1u8)", (2 + 4) + 32),
        ("[Dot, Dot]", 5 + 5 + 2 * (32 + 5)),
        // An error value is its names, `Er` + `BAD`; `Err(..)` charges them
        // again, as `Some(x)` charges `x` again (over-charging on purpose).
        ("Er.BAD", 5),
        ("Err(Er.BAD)", 5 + 5),
        ("be_bytes(5u32)", 4),
        ("le_bytes(5u16)", 2),
        ("Some(b)", 32 + 1000),
        ("Ok(b\"01\")", 1 + (32 + 1)),
        ("at([b\"0102\"], 0)", 2 + (32 + 2) + (32 + 2)),
        ("at([1u8], 5)", 32),
        ("fixed<2>(b\"0102\")", 2 + (32 + 2)),
        ("fixed<3>(b\"0102\")", 2),
        ("checked_add(1u8, 2u8)", 32),
        ("try<U8>(300)", 0),
        ("take(b, 3)", 3),
        ("take([b\"0102\", b\"03\"], 1)", 70 + (32 + 2)),
        ("replicate(b\"0102\", 2)", 2 + 2 * (32 + 2)),
        // Pass-through and scalar nodes: nothing.
        ("b", 0),
        ("(b)", 0),
        ("((b))", 0),
        ("if true then b else b", 0),
        ("id(b)", 0),
        ("unwrap_or(o, b)", 0),
        ("t.0", 0),
        ("(match o with None -> b | Some(v) -> v)", 0),
        ("len(b)", 0),
        ("b == b", 0),
    ];
    let texts: Vec<&str> = cases.iter().map(|c| c.0).collect();
    let mut src = doc_of(" v1", &texts);
    src.push_str("  record P(a: U8, b: Bytes)\n  model id(x: Bytes) -> Bytes = x\n");
    src.push_str("  variant Sh = Dot | Box2(w: U8)\n  error Er = BAD\n");
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let env: Env = HashMap::from([
            ("b", EvalValue::Bytes(vec![7; 1000])),
            (
                "o",
                EvalValue::Option(Some(Box::new(EvalValue::Bytes(vec![1, 2, 3])))),
            ),
            (
                "t",
                EvalValue::Tuple(vec![EvalValue::Bytes(vec![0; 5]), EvalValue::U8(1)]),
            ),
        ]);
        for (i, (text, data)) in cases.iter().enumerate() {
            ev.reset_work();
            ev.eval(e[i], &env)
                .unwrap_or_else(|err| panic!("{text}: {err}"));
            assert_eq!(ev.data_used(), *data, "v1 data of {text}");
        }
        // S0 E3: a group is a node: `((b))` is 3 work units.
        let groups = texts.iter().position(|t| *t == "((b))").unwrap();
        ev.reset_work();
        ev.eval(e[groups], &env).unwrap();
        assert_eq!(ev.work_used(), 3);
    });
    // v0 has no data meter at all.
    let v0 = [
        "[1u8, 2u8]",
        "b\"0102\"",
        "concat(b\"01\", b\"0203\")",
        "Some(b\"01\")",
        "at([1u8], 0)",
        "concat([1u8], [2u8])",
    ];
    with_doc(&doc_of("", &v0), |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        for (i, text) in v0.iter().enumerate() {
            ev.reset_work();
            ev.eval(e[i], &HashMap::new())
                .unwrap_or_else(|err| panic!("{text}: {err}"));
            assert_eq!(ev.data_used(), 0, "v0 data of {text}");
        }
    });
}

#[test]
fn replicate_of_a_64_kib_binder_fits_1023_copies_and_refuses_1024() {
    // VER04k section 2.7: 1,023 x 65,568 = 67,076,064 <= 2^26, and
    // 1,024 x 65,568 = 67,141,632 > 2^26.
    let src = doc_of(" v1", &["replicate(b, 1023)", "replicate(b, 1024)"]);
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let env: Env = HashMap::from([("b", EvalValue::Bytes(vec![0xa5; 65_536]))]);
        ev.reset_work();
        let v = ev.eval(e[0], &env).expect("1,023 copies fit");
        assert!(matches!(&v, EvalValue::List(xs) if xs.len() == 1023));
        drop(v);
        assert_eq!(ev.data_used(), 67_076_064);
        assert_eq!(ev.work_used(), 3 + 1023);
        assert_eq!(
            run(&mut ev, e[1], &env),
            Outcome::Exhausted(CodegenErrorKind::DataBudgetExceeded)
        );
    });
}

#[test]
fn depth_64_is_accepted_and_65_refused_through_groups_and_through_model_calls() {
    // S0 E1/E3: the 64 limit is nested node evaluation; a group is a level,
    // and a model body is one level below its call.
    let deep_limits = ParserLimits {
        max_depth: 128,
        ..ParserLimits::default()
    };
    let parse_eval = |src: &str| {
        let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
        let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
        let file = parse_document(&layout, &doc, deep_limits).expect("parse");
        let e = bodies(&file);
        let mut ev = Evaluator::new(&file, CodegenLimits::default());
        ev.reset_work();
        ev.eval(e[0], &HashMap::new())
            .map_err(|err| (err.code, err.kind))
    };
    // Groups: the parser's own nesting limit stops one declaration well
    // before 64 parentheses, so the groups are spread over a chain of 31
    // models, each body `(m<i+1>(a))`: the group and the call are a level
    // each.  `m1(1u8)` is level 1, m_i's group level 2i and its call 2i + 1;
    // the last body `((a))` puts `a` at level 2 x 31 + 2 = 64, and `(((a)))`
    // at 65.
    let grouped = |leaf: &str| {
        let mut s = String::from("warrant d v1\n  example e:\n    m1(1u8)\n");
        for i in 1..31 {
            s.push_str(&format!("  model m{i}(a: U8) -> U8 = (m{}(a))\n", i + 1));
        }
        s.push_str(&format!("  model m31(a: U8) -> U8 = {leaf}\n"));
        s
    };
    assert_eq!(parse_eval(&grouped("((a))")), Ok(EvalValue::U8(1)));
    assert_eq!(
        parse_eval(&grouped("(((a)))")),
        Err((CgCode::W0703, CodegenErrorKind::RecursionDepthExceeded))
    );
    // A chain m1 -> m2 -> ... -> mN, mN(a) = a: the call `m1(1u8)` is level 1
    // and mN's body `a` is level N + 1.
    let chain = |n: usize| {
        let mut s = String::from("warrant d v1\n  example e:\n    m1(1u8)\n");
        for i in 1..n {
            s.push_str(&format!("  model m{i}(a: U8) -> U8 = m{}(a)\n", i + 1));
        }
        s.push_str(&format!("  model m{n}(a: U8) -> U8 = a\n"));
        s
    };
    assert_eq!(parse_eval(&chain(63)), Ok(EvalValue::U8(1)));
    assert_eq!(
        parse_eval(&chain(64)),
        Err((CgCode::W0703, CodegenErrorKind::RecursionDepthExceeded))
    );
}

#[test]
fn the_leaves_cap_bounds_tuples_and_the_element_cap_bounds_sequences() {
    // S0 E6.  A tuple `(b, 1)` has 1 + len(b) + 1 leaves; 65,536 are allowed.
    let src = doc_of(" v1", &["(b, 1)", "to_list(b)", "to_bytes(xs)"]);
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let tuple_env = |n: usize| -> Env { HashMap::from([("b", EvalValue::Bytes(vec![1; n]))]) };
        assert!(matches!(
            run(&mut ev, e[0], &tuple_env(65_534)),
            Outcome::Value(_)
        ));
        assert_eq!(
            run(&mut ev, e[0], &tuple_env(65_535)),
            Outcome::Exhausted(CodegenErrorKind::WorkBudgetExceeded)
        );
        // The element cap: 65,537-element Bytes and List operands are refused
        // by the forms that would copy them.
        let env: Env = HashMap::from([
            ("b", EvalValue::Bytes(vec![1; 65_537])),
            ("xs", u8_list(&vec![1; 65_537])),
        ]);
        for i in [1, 2] {
            assert_eq!(
                run(&mut ev, e[i], &env),
                Outcome::Exhausted(CodegenErrorKind::SequenceTooLong)
            );
        }
    });
}

/// `replicate(b, 300)` of a 64 KiB `b`: 300 x (32 + 65,536) bytes.
const ONE_COPY_RUN: u64 = 300 * (32 + 65_536);
// Three such runs fit the budget (59,011,200 <= 67,108,864); four do not.
const _: () =
    assert!(3 * ONE_COPY_RUN <= DATA_BUDGET_BYTES && 4 * ONE_COPY_RUN > DATA_BUDGET_BYTES);

#[test]
fn the_data_budget_is_cumulative_per_instance_and_reset_between_instances() {
    assert_eq!(DATA_BUDGET_BYTES, 64 * 1024 * 1024);
    let three = "len(replicate(b, 300)) + len(replicate(b, 300)) + len(replicate(b, 300))";
    let four = format!("{three} + len(replicate(b, 300))");
    let src = doc_of(
        " v1",
        &[
            three,
            &four,
            "replicate(b, 65536)",
            "range(3)",
            "replicate(xs, 2048)",
        ],
    );
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        // A 64 KiB payload: each copy costs 32 + 65,536 bytes.
        let big = vec![0x5a; 65_536];
        let env: Env = HashMap::from([("b", EvalValue::Bytes(big)), ("xs", u8_list(&[0; 4096]))]);
        let one = ONE_COPY_RUN;
        assert_eq!(
            run(&mut ev, e[0], &env),
            Outcome::Value(EvalValue::Int(900))
        );
        assert_eq!(ev.data_used(), 3 * one);
        // ... a fourth in the same instance does not.
        assert_eq!(
            run(&mut ev, e[1], &env),
            Outcome::Exhausted(CodegenErrorKind::DataBudgetExceeded)
        );
        // A single request far over the budget is refused before allocating
        // (the allocation probe in tests/wxd_alloc_probe.rs pins "before").
        for i in [2, 4] {
            assert_eq!(
                run(&mut ev, e[i], &env),
                Outcome::Exhausted(CodegenErrorKind::DataBudgetExceeded),
                "example {i}"
            );
        }
        // reset_work starts a new instance: the data meter is back at zero.
        ev.reset_work();
        assert_eq!(ev.data_used(), 0);
        assert_eq!(
            run(&mut ev, e[3], &env),
            Outcome::Value(EvalValue::List(vec![
                EvalValue::Nat(0),
                EvalValue::Nat(1),
                EvalValue::Nat(2),
            ]))
        );
        assert_eq!(ev.data_used(), 3 * 32);
    });
}

#[test]
fn the_work_charge_comes_after_the_caps_so_a_capped_request_charges_only_its_nodes() {
    // range(65537): the element cap fires first, so the meter holds only the
    // nodes evaluated so far (the call and its literal), never 65,537 more.
    let src = doc_of(" v1", &["range(65537)", "replicate(1u8, 65536)"]);
    with_doc(&src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        assert_eq!(
            run(&mut ev, e[0], &HashMap::new()),
            Outcome::Exhausted(CodegenErrorKind::SequenceTooLong)
        );
        assert_eq!(ev.work_used(), 2);
        assert_eq!(ev.data_used(), 0);
        // With a work budget too small for L, the data was reserved first.
        let mut small = Evaluator::new(
            file,
            CodegenLimits {
                max_work: 1000,
                ..CodegenLimits::default()
            },
        );
        small.reset_work();
        let err = small.eval(e[1], &HashMap::new()).unwrap_err();
        assert_eq!(
            (err.code, err.kind),
            (CgCode::W0703, CodegenErrorKind::WorkBudgetExceeded)
        );
        assert_eq!(small.data_used(), 65_536 * 32);
    });
}

// ---------------------------------------------------------------------------
// Short-circuit: v1 only
// ---------------------------------------------------------------------------

#[test]
fn and_or_and_implies_short_circuit_in_v1() {
    let blow = format!("(replicate(0u8, {HUGE_TEXT}) == [])");
    let bodies_v1 = [
        format!("false and {blow}"),
        format!("true or {blow}"),
        format!("false => {blow}"),
        format!("true and {blow}"),
        format!("false or {blow}"),
        format!("true => {blow}"),
        "false and (1 + 2 == 3)".to_string(),
        "true and (1 + 2 == 3)".to_string(),
    ];
    let texts: Vec<&str> = bodies_v1.iter().map(String::as_str).collect();
    with_doc(&doc_of(" v1", &texts), |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let env = HashMap::new();
        // The deciding left operand: the right one is neither evaluated nor
        // charged (operator node + literal = 2 units).
        for (i, want) in [(0, false), (1, true), (2, true)] {
            assert_eq!(
                run(&mut ev, e[i], &env),
                Outcome::Value(EvalValue::Bool(want)),
                "{}",
                texts[i]
            );
            assert_eq!(ev.work_used(), 2, "{}", texts[i]);
        }
        // A non-deciding left operand evaluates the right one.
        for i in [3, 4, 5] {
            assert_eq!(
                run(&mut ev, e[i], &env),
                Outcome::Exhausted(CodegenErrorKind::SequenceTooLong),
                "{}",
                texts[i]
            );
        }
        assert_eq!(
            run(&mut ev, e[6], &env),
            Outcome::Value(EvalValue::Bool(false))
        );
        assert_eq!(ev.work_used(), 2);
        assert_eq!(
            run(&mut ev, e[7], &env),
            Outcome::Value(EvalValue::Bool(true))
        );
        // and + true + group + == + (+ 1 2) + 3 = 8 nodes.
        assert_eq!(ev.work_used(), 8);
    });
}

#[test]
fn v0_still_evaluates_both_operands_eagerly() {
    // An Int overflow on the right is still reached in v0 (W0702), and the
    // charge still covers both sides.
    let overflow = "(18446744073709551615 * 18446744073709551615 * 18446744073709551615 == 0)";
    let texts = [
        format!("false and {overflow}"),
        format!("true or {overflow}"),
        format!("false => {overflow}"),
        "false and (1 + 2 == 3)".to_string(),
        "true or (1 + 2 == 3)".to_string(),
        "false => (1 + 2 == 3)".to_string(),
    ];
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    with_doc(&doc_of("", &refs), |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let env = HashMap::new();
        for (i, text) in refs.iter().enumerate().take(3) {
            ev.reset_work();
            let err = ev.eval(e[i], &env).expect_err(text);
            assert_eq!(err.code, CgCode::W0702, "{text}");
        }
        for (i, want) in [(3, false), (4, true), (5, true)] {
            ev.reset_work();
            assert_eq!(ev.eval(e[i], &env).unwrap(), EvalValue::Bool(want));
            assert_eq!(ev.work_used(), 8, "{}", refs[i]);
        }
    });
    // The same overflow in v1 is never reached behind a deciding operand.
    with_doc(&doc_of(" v1", &refs[..3]), |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        for (i, want) in [(0, false), (1, true), (2, true)] {
            ev.reset_work();
            assert_eq!(
                ev.eval(e[i], &HashMap::new()).unwrap(),
                EvalValue::Bool(want)
            );
        }
    });
}

// ---------------------------------------------------------------------------
// An applied name is never a binder (security review)
// ---------------------------------------------------------------------------
//
// The checker resolves `f(..)` without looking at the binders in scope, so
// the evaluator must not either: `take(take, 0)` with a parameter `take` is
// the built-in.  In v1 the checker now also refuses such a parameter (W0203);
// the evaluator is held to the checker's reading anyway, because it can be
// handed a document that was never checked.  In v0, where a parameter may
// still spell a built-in, `len(len)` was typed `Nat` by the checker and
// evaluated to the `Bytes` parameter; it is now the `Nat` the checker typed.

#[test]
fn an_applied_name_is_the_built_in_or_model_not_the_binder_of_that_name() {
    let src = "warrant shadow v1\n  model exp(take: List<U8>) -> List<U8> = take(take, 0)\n  model r(range: Bool) -> List<Nat> = range(2)\n  model t(take: List<U8>) -> List<U8> = take\n  example e0:\n    exp([1u8, 2u8])\n  example e1:\n    r(true)\n  example e2:\n    t([3u8])\n";
    with_doc(src, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let env: Env = HashMap::new();
        assert_eq!(
            ev.eval(e[0], &env).expect("exp"),
            EvalValue::List(vec![]),
            "take(take, 0) is the built-in"
        );
        assert_eq!(
            ev.eval(e[1], &env).expect("r"),
            EvalValue::List(vec![EvalValue::Nat(0), EvalValue::Nat(1)]),
            "range(2) is the built-in"
        );
        // A bare name is still the binder.
        assert_eq!(
            ev.eval(e[2], &env).expect("t"),
            EvalValue::List(vec![EvalValue::U8(3)])
        );
    });
    let v0 = "warrant shadow\n  model f(len: Bytes) -> Nat = len(len)\n  example e0:\n    f(b\"0102\")\n";
    with_doc(v0, |file| {
        let e = bodies(file);
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        assert_eq!(
            ev.eval(e[0], &HashMap::new()).expect("f"),
            EvalValue::Nat(2),
            "len(len) is the built-in the checker typed"
        );
    });
}
