//! LANG83 PR5a — strings fail closed on WASM.
//!
//! Every string failure reaches the host as a registered kind through
//! `env.__trap`, then the module's `unreachable` stops it:
//!
//! | failure                                   | kind id | before PR5a           |
//! |-------------------------------------------|---------|-----------------------|
//! | `str_index` outside `0 ≤ i < len`         | 1 `Trap.Bounds`   | bare `unreachable` |
//! | `str_slice` outside `0 ≤ a ≤ b ≤ len`     | 1 `Trap.Bounds`   | bare `unreachable` |
//! | `str_concat` longer than the ceiling      | 5 `Trap.Overflow` | no check at all   |
//!
//! Each case runs in two forms: *opaque*, where the string and the indices
//! reach the op as parameters of a helper function (so nothing can fold), and
//! *literal*, where everything is a constant the backend may fold. The indices
//! include the values that only a full 64-bit check sees: `2³² + 1` wraps to
//! 1 under `i32.wrap_i64`, and `2³²` to 0.

mod common;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};

const BOUNDS: i32 = 1;
const OVERFLOW: i32 = 5;

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn module(functions: Vec<IIRFunction>, ceiling: Option<u32>) -> IIRModule {
    let mut m = IIRModule::new("pr5a", "test");
    m.functions = functions;
    m.str_max_bytes = ceiling;
    m
}

/// Run `main`; `Ok(value)` or `Err(the kind id env.__trap received)`.
fn run(m: &IIRModule) -> Result<i64, Option<i32>> {
    let wasm = lower_iir_to_wasm(m, &IIRWasmConfig::default()).expect("lowering");
    let bytes = encode_module(&wasm).expect("encoding");
    let (rt, slot) = common::runtime_with_trap_slot();
    match rt.load_and_run(&bytes, "main", &[]) {
        Ok(vals) => {
            assert_eq!(slot.get(), None, "returned after env.__trap");
            Ok(vals.first().copied().expect("main returns a value"))
        }
        Err(_) => Err(slot.get()),
    }
}

/// `main` passes "ABC" and the `args` to `f`, which applies `body` to its
/// parameters and returns its last dest (an `i64`).
fn opaque(params: &[(&str, &str)], args: &[IIRInstr], call_args: &[&str], body: Vec<IIRInstr>) -> Vec<IIRFunction> {
    let f = IIRFunction::new("f", params.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect(), "i64", body);
    let mut main = args.to_vec();
    let mut srcs = vec![v("f")];
    srcs.extend(call_args.iter().map(|a| v(a)));
    main.push(IIRInstr::new("call", Some("r".into()), srcs, "i64"));
    main.push(IIRInstr::new("ret", None, vec![v("r")], "i64"));
    vec![f, IIRFunction::new("main", vec![], "i64", main)]
}

fn int(d: &str, n: i64) -> IIRInstr {
    IIRInstr::new("const", Some(d.into()), vec![Operand::Int(n)], "i64")
}
fn text(d: &str, s: &str) -> IIRInstr {
    IIRInstr::new("str_const", Some(d.into()), vec![Operand::Str(s.into())], "str")
}

fn index_opaque(i: i64) -> IIRModule {
    module(
        opaque(
            &[("s", "str"), ("i", "i64")],
            &[text("abc", "ABC"), int("idx", i)],
            &["abc", "idx"],
            vec![
                IIRInstr::new("str_index", Some("b".into()), vec![v("s"), v("i")], "i64"),
                IIRInstr::new("ret", None, vec![v("b")], "i64"),
            ],
        ),
        None,
    )
}

fn index_literal(i: i64) -> IIRModule {
    module(
        vec![IIRFunction::new("main", vec![], "i64", vec![
            text("s", "ABC"),
            int("i", i),
            IIRInstr::new("str_index", Some("b".into()), vec![v("s"), v("i")], "i64"),
            IIRInstr::new("ret", None, vec![v("b")], "i64"),
        ])],
        None,
    )
}

fn slice_opaque(a: i64, b: i64) -> IIRModule {
    module(
        opaque(
            &[("s", "str"), ("a", "i64"), ("b", "i64")],
            &[text("abc", "ABC"), int("x", a), int("y", b)],
            &["abc", "x", "y"],
            vec![
                IIRInstr::new("str_slice", Some("t".into()), vec![v("s"), v("a"), v("b")], "str"),
                IIRInstr::new("str_len", Some("n".into()), vec![v("t")], "i64"),
                IIRInstr::new("ret", None, vec![v("n")], "i64"),
            ],
        ),
        None,
    )
}

fn slice_literal(a: i64, b: i64) -> IIRModule {
    module(
        vec![IIRFunction::new("main", vec![], "i64", vec![
            text("s", "ABC"),
            int("a", a),
            int("b", b),
            IIRInstr::new("str_slice", Some("t".into()), vec![v("s"), v("a"), v("b")], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![v("t")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ])],
        None,
    )
}

const OUT_OF_RANGE_INDICES: [i64; 6] = [-1, 3, i64::MAX, i64::MIN, (1 << 32) + 1, 1 << 32];
const OUT_OF_RANGE_SLICES: [(i64, i64); 6] =
    [(-1, 1), (2, 1), (0, 4), (1 << 32, (1 << 32) + 3), (0, (1 << 32) + 1), (i64::MIN, 0)];

#[test]
fn str_index_out_of_range_traps_bounds_in_both_forms() {
    for i in OUT_OF_RANGE_INDICES {
        assert_eq!(run(&index_opaque(i)), Err(Some(BOUNDS)), "opaque ABC[{i}]");
        assert_eq!(run(&index_literal(i)), Err(Some(BOUNDS)), "literal ABC[{i}]");
    }
    assert_eq!(run(&index_opaque(0)), Ok(65));
    assert_eq!(run(&index_opaque(2)), Ok(67));
    assert_eq!(run(&index_literal(2)), Ok(67));
}

/// Verification item 2 of the PR5a plan: WASM `str_slice` wraps its offsets
/// to `i32` only after the full `i64` check, so `[2³², 2³² + 3)` (which wraps
/// to `[0, 3)`) is out of range, not "ABC".
#[test]
fn str_slice_out_of_range_traps_bounds_in_both_forms() {
    for (a, b) in OUT_OF_RANGE_SLICES {
        assert_eq!(run(&slice_opaque(a, b)), Err(Some(BOUNDS)), "opaque ABC[{a}..{b})");
        assert_eq!(run(&slice_literal(a, b)), Err(Some(BOUNDS)), "literal ABC[{a}..{b})");
    }
    for (a, b, n) in [(0, 0, 0), (3, 3, 0), (0, 3, 3), (1, 2, 1)] {
        assert_eq!(run(&slice_opaque(a, b)), Ok(n), "opaque ABC[{a}..{b})");
        assert_eq!(run(&slice_literal(a, b)), Ok(n), "literal ABC[{a}..{b})");
    }
}

/// `s ++ s` with `s` 8 bytes (16), then `++ "x"` (17), under `ceiling`; opaque.
fn concat_opaque(ceiling: Option<u32>) -> IIRModule {
    module(
        opaque(
            &[("s", "str"), ("x", "str")],
            &[text("eight", "01234567"), text("one", "x")],
            &["eight", "one"],
            vec![
                IIRInstr::new("str_concat", Some("d".into()), vec![v("s"), v("s")], "str"),
                IIRInstr::new("str_len", Some("n".into()), vec![v("d")], "i64"),
                IIRInstr::new("str_concat", Some("e".into()), vec![v("d"), v("x")], "str"),
                IIRInstr::new("ret", None, vec![v("n")], "i64"),
            ],
        ),
        ceiling,
    )
}

/// The same with every operand a literal, so the backend may fold.
fn concat_literal(ceiling: Option<u32>) -> IIRModule {
    module(
        vec![IIRFunction::new("main", vec![], "i64", vec![
            text("s", "01234567"),
            text("x", "x"),
            IIRInstr::new("str_concat", Some("d".into()), vec![v("s"), v("s")], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![v("d")], "i64"),
            IIRInstr::new("str_concat", Some("e".into()), vec![v("d"), v("x")], "str"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ])],
        ceiling,
    )
}

#[test]
fn str_concat_past_the_ceiling_traps_overflow_in_both_forms() {
    assert_eq!(run(&concat_opaque(Some(16))), Err(Some(OVERFLOW)));
    assert_eq!(run(&concat_literal(Some(16))), Err(Some(OVERFLOW)));
    for ceiling in [Some(17), None] {
        assert_eq!(run(&concat_opaque(ceiling)), Ok(16), "{ceiling:?}");
        assert_eq!(run(&concat_literal(ceiling)), Ok(16), "{ceiling:?}");
    }
}

// ── #18800 item 1: the ceiling is checked at compile time ───────────────────

/// `main` binds one literal of `len` bytes and returns its length.
fn literal_of(len: usize, ceiling: Option<u32>) -> IIRModule {
    module(
        vec![IIRFunction::new("main", vec![], "i64", vec![
            text("s", &"z".repeat(len)),
            IIRInstr::new("str_len", Some("n".into()), vec![v("s")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ])],
        ceiling,
    )
}

/// A `str_const` longer than the module's ceiling is malformed IR, and so is
/// a ceiling above `STR_MAX_BYTES`. `lower_iir_to_wasm` refuses both before
/// it emits anything (#18800); before, it compiled the 17-byte literal and
/// the program returned 17 under a ceiling of 16.
#[test]
fn a_literal_past_the_module_ceiling_is_refused_at_compile_time() {
    let refusal = |m: &IIRModule| match lower_iir_to_wasm(m, &IIRWasmConfig::default()) {
        Err(iir_to_wasm::IIRWasmError::ValidationFailed(errors)) => errors,
        other => panic!("expected a validation failure, got {other:?}"),
    };
    let errors = refusal(&literal_of(17, Some(16)));
    assert!(errors.iter().any(|e| e.starts_with("StrTooLong")), "{errors:?}");
    let errors = refusal(&literal_of(0, Some(interpreter_ir::STR_MAX_BYTES + 1)));
    assert!(errors.iter().any(|e| e.starts_with("StrMaxBytes")), "{errors:?}");
    // The boundary: a literal exactly at the ceiling compiles and runs.
    assert_eq!(run(&literal_of(16, Some(16))), Ok(16));
    assert!(iir_to_wasm::validate_for_wasm(&literal_of(17, Some(16)))
        .iter()
        .any(|e| e.starts_with("StrTooLong")));
}

// ── #18800 item 4: no literal fold past the ceiling ─────────────────────────

/// `s` is an 8-byte literal; `d1 = s ++ s` (16 bytes), then `d{k+1} = dk ++
/// dk`, `doublings` times in all, and `main` returns `str_len` of the last.
fn doubling_chain(doublings: usize, ceiling: Option<u32>) -> IIRModule {
    let mut body = vec![text("d0", "01234567")];
    for k in 0..doublings {
        let (src, dst) = (format!("d{k}"), format!("d{}", k + 1));
        body.push(IIRInstr::new("str_concat", Some(dst), vec![v(&src), v(&src)], "str"));
    }
    body.push(IIRInstr::new("str_len", Some("n".into()), vec![v(&format!("d{doublings}"))], "i64"));
    body.push(IIRInstr::new("ret", None, vec![v("n")], "i64"));
    module(vec![IIRFunction::new("main", vec![], "i64", body)], ceiling)
}

/// The fold pre-pass used to join any two literals, with no ceiling: a chain
/// of `d = d ++ d` doubled in the *compiler*. Twenty doublings of 8 bytes put
/// about 16 MiB of joined literals into the data segment (forty would have
/// asked for 8 TiB), although the program traps `Trap.Overflow` at the second
/// concat. The pre-pass now stops at the ceiling, like twig-aot's fold: the
/// join past it is a run-time concat, and the run-time check traps.
#[test]
fn a_literal_join_past_the_ceiling_is_left_to_the_runtime_check() {
    let m = doubling_chain(20, Some(16));
    let bytes = encode_module(&lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering"))
        .expect("encoding");
    assert!(bytes.len() < 64 * 1024, "the module is {} bytes", bytes.len());
    assert_eq!(run(&m), Err(Some(OVERFLOW)));

    // The 17-byte join of the ceiling-16 case is not in the data segment:
    // it is built at run time, if at all (here it is not: the trap comes first).
    let bytes = encode_module(&lower_iir_to_wasm(&concat_literal(Some(16)), &IIRWasmConfig::default()).unwrap())
        .unwrap();
    let joined = b"0123456701234567x";
    assert!(!bytes.windows(joined.len()).any(|w| w == joined), "the 17-byte join was folded");
    // Under the registry ceiling the same join still folds into the data.
    let bytes = encode_module(&lower_iir_to_wasm(&concat_literal(None), &IIRWasmConfig::default()).unwrap())
        .unwrap();
    assert!(bytes.windows(joined.len()).any(|w| w == joined), "the fold fast path is gone");
    // And a chain within the ceiling still folds all the way (8 → 16 → 32).
    assert_eq!(run(&doubling_chain(2, None)), Ok(32));
}

// ── #18800 security review: a stale literal must not stand in for a write ──

/// `d = str_const "x"` with a non-`str` hint (validation accepts it), then
/// `d = <op>`, then `str_len d`. The fold table is keyed by name, so `d`'s
/// first literal must not answer for the second write.
fn rewritten(first_hint: &str, second: IIRInstr, ceiling: Option<u32>) -> IIRModule {
    module(
        vec![IIRFunction::new("main", vec![], "i64", vec![
            IIRInstr::new("str_const", Some("d".into()), vec![Operand::Str("x".into())], first_hint),
            text("a", "abcdefgh"),
            int("lo", 2),
            int("hi", 99),
            second,
            IIRInstr::new("str_len", Some("n".into()), vec![v("d")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ])],
        ceiling,
    )
}

/// The reviewer's reproducer: under a ceiling of 8, `d = a ++ a` (16 bytes) is
/// not folded, but the not-folded branch left `d`'s earlier literal "x" in the
/// table, so `str_len d` returned 1 and the concat's `Trap.Overflow` never
/// ran (before PR5a's follow-up the fold-site trap caught it). Now `d` is
/// run-time valued (it is written twice, whatever the hints) and the run-time
/// check traps. Same for a literal `str_slice` whose constant range is out of
/// bounds: it must trap `Trap.Bounds`, not read the stale literal.
#[test]
fn a_rewritten_string_variable_never_reads_its_stale_literal() {
    let concat = || IIRInstr::new("str_concat", Some("d".into()), vec![v("a"), v("a")], "str");
    let slice = || IIRInstr::new("str_slice", Some("d".into()), vec![v("a"), v("lo"), v("hi")], "str");
    for hint in ["bool", "void", "str"] {
        assert_eq!(run(&rewritten(hint, concat(), Some(8))), Err(Some(OVERFLOW)), "concat, first hint {hint}");
        assert_eq!(run(&rewritten(hint, concat(), None)), Ok(16), "concat fits, first hint {hint}");
        assert_eq!(run(&rewritten(hint, slice(), None)), Err(Some(BOUNDS)), "slice, first hint {hint}");
    }
}
