//! A local's **declared** type and the type actually stored into it must
//! agree — otherwise the module is ill-typed and a conforming validator
//! rejects it outright.
//!
//! WASM is strictly typed: `local.set $x` demands exactly `$x`'s declared
//! type, no coercion. That makes the locals section a contract the emitter
//! has to honour on every store. For a long time nothing in this repo
//! checked it — `wasm-execution` is untyped at runtime, and the validator
//! did not look at instruction operands — so three separate places in
//! `lower.rs` broke the contract and nothing failed. `wasm-validator`'s
//! instruction-level type checker (WASM06 / W02 Phase 2) then rejected 42
//! cross-language matrix cells in one go.
//!
//! These tests pin the two shapes that can be built from hand-written IIR.
//! The third (the lisp predicate builtins `pair?`/`not`/`equal?` storing an
//! `i32` boolean into an `i64` slot) needs the WasmGC `$LispyPair` /
//! `i31ref` value model, and is covered end-to-end by the Twig symbol and
//! record cells in `lang-aot`'s cross-language matrix.
//!
//! Every test here runs the module through `WasmRuntime::load_and_run`,
//! which **validates** before executing. A test that only lowered and
//! encoded would pass on an ill-typed module and pin nothing.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_runtime::WasmRuntime;

/// Lower + encode a single-function module named `main` returning `ret_ty`.
fn module(ret_ty: &str, instrs: Vec<IIRInstr>) -> Vec<u8> {
    let f = IIRFunction::new("main", vec![], ret_ty, instrs);
    let m = IIRModule {
        name: "slot_widths".into(),
        functions: vec![f],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let wasm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).expect("lowering failed");
    encode_module(&wasm).expect("encoding failed")
}

/// Validate + run, returning `main`'s value. Panics with the validation
/// error if the module is ill-typed — which is the point of these tests.
fn run(bytes: &[u8]) -> i64 {
    WasmRuntime::new()
        .load_and_run(bytes, "main", &[])
        .expect("wasm validate/run failed")
        .first()
        .copied()
        .expect("main returns a value")
}

/// **Bug 1** — a comparison's `type_hint` names its *operands*, not its
/// destination.
///
/// `cmp_ge` over two `f64`s is the exact shape every Dartmouth BASIC
/// comparison has (REAL is BASIC's only numeric type). The comparison
/// itself must be `f64.ge`, but `f64.ge` — like every WASM comparison —
/// pushes an **`i32`** 0/1. Declaring the destination local `f64` from the
/// operand hint produced `local.set` of an i32 into an f64 local:
/// `TypeMismatch: expected F64, found I32`.
#[test]
fn an_f64_comparison_destination_is_a_bool_local_not_an_f64_one() {
    for (a, b, expected) in [(3.0_f64, 2.0_f64, 1_i64), (2.0, 3.0, 0)] {
        let bytes = module(
            "bool",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Float(a)], "f64"),
                IIRInstr::new("const", Some("b".into()), vec![Operand::Float(b)], "f64"),
                // type_hint "f64" selects `f64.ge`; the dest is a bool.
                IIRInstr::new(
                    "cmp_ge",
                    Some("c".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())],
                    "f64",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
            ],
        );
        assert_eq!(run(&bytes), expected, "f64 cmp_ge {a} >= {b}");
    }
}

/// The same rule for `f32` operands, the other float width.
#[test]
fn an_f32_comparison_destination_is_a_bool_local_too() {
    let bytes = module(
        "bool",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Float(7.5)], "f32"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Float(7.5)], "f32"),
            IIRInstr::new(
                "cmp_eq",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "f32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
        ],
    );
    assert_eq!(run(&bytes), 1);
}

/// An `i64`-declared comparison destination stays `i64`.
///
/// This is the case the cmp lowering has always handled correctly, by
/// widening its `i32` result with `i64.extend_i32_u`. It is deliberately
/// *not* rewritten to a bool: `concretize_scalar_any_for_wasm` retypes a
/// scalar `any` to `i64`, and the Brainfuck / tagged-Twig value model then
/// consumes the guard with `i64.eqz`. Pinned here so a future "simplify
/// this to always emit bool" cannot quietly break that model.
#[test]
fn an_i64_comparison_destination_stays_i64() {
    let bytes = module(
        "i64",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(9)], "i64"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(4)], "i64"),
            IIRInstr::new(
                "cmp_gt",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i64"),
        ],
    );
    assert_eq!(run(&bytes), 1);
}

/// **Bug 2** — `slot_is_i64` must agree with `hint_to_value_type`.
///
/// Narrow *unsigned* types ride the i64 register model (LANG-FULL E2):
/// `hint_to_value_type("u8")` is `I64`, so a `u8` local is declared `i64`.
/// `slot_is_i64` instead matched the hint spellings `"i64" | "u64"` and so
/// answered "no" for `u8` — and the `and`/`or`/`xor` arm, which asks it
/// whether to wrap its `i64` result back down, duly emitted `i32.wrap_i64`
/// before storing into an `i64` local: `TypeMismatch: expected I64, found
/// I32`. These are Nib's `12 & 10`, `12 | 3`, `6 ^ 5` matrix cells.
#[test]
fn narrow_unsigned_bitwise_results_are_stored_at_the_declared_i64_width() {
    for ty in ["u4", "u8", "u16", "u32"] {
        for (op, a, b, expected) in [
            ("and", 12_i64, 10_i64, 8_i64),
            ("or", 12, 3, 15),
            ("xor", 6, 5, 3),
        ] {
            let bytes = module(
                ty,
                vec![
                    IIRInstr::new("const", Some("a".into()), vec![Operand::Int(a)], ty),
                    IIRInstr::new("const", Some("b".into()), vec![Operand::Int(b)], ty),
                    IIRInstr::new(
                        op,
                        Some("c".into()),
                        vec![Operand::Var("a".into()), Operand::Var("b".into())],
                        ty,
                    ),
                    IIRInstr::new("ret", None, vec![Operand::Var("c".into())], ty),
                ],
            );
            assert_eq!(run(&bytes), expected, "{ty}: {a} {op} {b}");
        }
    }
}

/// Signed `i32`-model widths are untouched by the `slot_is_i64` change:
/// `hint_to_value_type("i32")` is `I32`, so no widening is involved and the
/// bitwise ops stay on the i32 opcodes.
#[test]
fn i32_model_bitwise_results_stay_i32() {
    let bytes = module(
        "i32",
        vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(12)], "i32"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(10)], "i32"),
            IIRInstr::new(
                "and",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
        ],
    );
    assert_eq!(run(&bytes), 8);
}
