//! LANG83 PR6c on the CLR textual (`emit_il`) path: the non-trapping IEEE
//! ops `f64_add`, `f64_sub`, `f64_mul`, `f64_div`, `f64_neg`, and
//! `f64_signbit`.
//!
//! No `dotnet`/`ilasm` on the development hosts, so these pin the emitted IL
//! (CI's CLR column assembles and runs it on real CoreCLR through
//! `lang-aot`'s conformance suite). CIL float arithmetic is IEEE by
//! specification — `div` throws only for an *integer* zero — so each op is a
//! single instruction with no guard; the sign bit is the raw-bits call
//! followed by a LOGICAL shift.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_cil_bytecode::{emit_il, validate_iir_for_clr, IIRClrConfig};

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

/// `main(a: f64, b: f64)`: `r <- op(a[, b])`, then return `ret` (an i64
/// constant, or the sign bit when `op` is `f64_signbit`).
fn module(op: &str, dest_ty: &str, params_ty: &str) -> IIRModule {
    let srcs = if matches!(op, "f64_neg" | "f64_signbit") { vec![v("a")] } else { vec![v("a"), v("b")] };
    let mut body = vec![IIRInstr::new(op, Some("r".into()), srcs, dest_ty)];
    if dest_ty == "i64" {
        body.push(IIRInstr::new("ret", None, vec![v("r")], "i64"));
    } else {
        body.push(IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"));
        body.push(IIRInstr::new("ret", None, vec![v("z")], "i64"));
    }
    let params = vec![("a".into(), params_ty.into()), ("b".into(), params_ty.into())];
    let mut m = IIRModule::new("Main", "test");
    m.functions.push(IIRFunction::new("main", params, "i64", body));
    m.entry_point = Some("main".into());
    m
}

/// The trimmed IL lines from the first `ldarg 0` up to (not including) the
/// first `stloc` — exactly the op's own sequence.
fn op_il(op: &str, dest_ty: &str) -> Vec<String> {
    let m = module(op, dest_ty, "f64");
    assert!(validate_iir_for_clr(&m).is_empty(), "{:?}", validate_iir_for_clr(&m));
    let il = emit_il(&m, &IIRClrConfig::new("Main")).expect("emit_il");
    let start = il.find("ldarg 0").unwrap_or_else(|| panic!("{op}: loads a\n{il}"));
    il[start..].lines().map(|l| l.trim().to_string()).take_while(|l| !l.starts_with("stloc")).collect()
}

#[test]
fn arithmetic_is_one_unguarded_cil_instruction() {
    for (op, inst) in [("f64_add", "add"), ("f64_sub", "sub"), ("f64_mul", "mul"), ("f64_div", "div")] {
        assert_eq!(op_il(op, "f64"), ["ldarg 0", "ldarg 1", inst], "{op}");
    }
    assert_eq!(op_il("f64_neg", "f64"), ["ldarg 0", "neg"]);
}

#[test]
fn signbit_reads_the_raw_bits_then_shifts_logically() {
    assert_eq!(
        op_il("f64_signbit", "i64"),
        [
            "ldarg 0",
            "call int64 [System.Runtime]System.BitConverter::DoubleToInt64Bits(float64)",
            "ldc.i4.s 63",
            "shr.un",
        ]
    );
}

/// Never a throw: no `ckfinite`, no trap sequence, no `Exception`.
#[test]
fn nothing_traps() {
    for op in ["f64_add", "f64_sub", "f64_mul", "f64_div", "f64_neg"] {
        let il = emit_il(&module(op, "f64", "f64"), &IIRClrConfig::new("Main")).unwrap();
        assert!(!il.contains("ckfinite") && !il.contains("throw"), "{op}:\n{il}");
    }
}

/// An integer operand, or a dest of the wrong type, is a compile error — not
/// IL that CoreCLR's verifier would reject.
#[test]
fn non_float_operands_are_refused() {
    for op in ["f64_add", "f64_neg", "f64_signbit"] {
        let dest_ty = if op == "f64_signbit" { "i64" } else { "f64" };
        assert!(emit_il(&module(op, dest_ty, "i64"), &IIRClrConfig::new("Main")).is_err(), "{op}: i64 sources");
    }
    assert!(emit_il(&module("f64_signbit", "f64", "f64"), &IIRClrConfig::new("Main")).is_err(), "signbit into a float");
    assert!(emit_il(&module("f64_add", "i64", "f64"), &IIRClrConfig::new("Main")).is_err(), "a sum into an integer");
}
