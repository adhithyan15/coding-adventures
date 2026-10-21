//! LANG83 PR4 on the CLR textual (`emit_il`) path: the overflow-checked
//! `i64_add_ovf`, `i64_sub_ovf`, `i64_mul_ovf` and `i64_neg_ovf`.
//!
//! No `dotnet`/`ilasm` on the development hosts, so these pin the emitted IL
//! and — because CoreCLR rejects IL that `ilasm` happily assembles when the
//! evaluation stack is inconsistent at a branch or label (lesson from Perl
//! 4a) — they also run a small **stack-depth model** over every emitted line:
//! the depth may never go negative, every conditional branch must leave the
//! stack empty, and every label must be reached with an empty stack. CI's CLR
//! column assembles and runs the IL on real CoreCLR.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_cil_bytecode::{emit_il, lower_iir_to_cil, validate_iir_for_clr, IIRClrConfig};

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn module(body: Vec<IIRInstr>, params: Vec<(String, String)>) -> IIRModule {
    let mut m = IIRModule::new("Main", "test");
    m.functions.push(IIRFunction::new("main", params, "i64", body));
    m.entry_point = Some("main".into());
    m
}

fn i64_params() -> Vec<(String, String)> {
    vec![("a".into(), "i64".into()), ("b".into(), "i64".into())]
}

/// The trimmed IL lines of the method that holds `op`, from its first
/// instruction to its `ret`.
fn checked_il(op: &str) -> Vec<String> {
    let srcs = if op == "i64_neg_ovf" { vec![v("a")] } else { vec![v("a"), v("b")] };
    let m = module(
        vec![IIRInstr::new(op, Some("q".into()), srcs, "i64"), IIRInstr::new("ret", None, vec![v("q")], "i64")],
        i64_params(),
    );
    assert!(validate_iir_for_clr(&m).is_empty(), "{:?}", validate_iir_for_clr(&m));
    let il = emit_il(&m, &IIRClrConfig::new("Main")).expect("emit_il");
    let start = il.find("ldarg 0").expect("loads a");
    il[start..].lines().map(|l| l.trim().to_string()).take_while(|l| !l.starts_with("ret")).collect()
}

const TRAP: [&str; 5] = [
    "call class [System.Runtime]System.IO.TextWriter [System.Console]System.Console::get_Out()",
    "callvirt instance void [System.Runtime]System.IO.TextWriter::Flush()",
    "ldstr \"Trap.Overflow\"",
    "newobj instance void [System.Runtime]System.Exception::.ctor(string)",
    "throw",
];

/// Indices where a full `Trap.Overflow` sequence starts.
fn overflow_traps(lines: &[String]) -> Vec<usize> {
    (0..lines.len().saturating_sub(TRAP.len() - 1))
        .filter(|&i| (0..TRAP.len()).all(|k| lines[i + k] == TRAP[k]))
        .collect()
}

/// Stack-depth model of the IL subset these sequences use. Panics on an
/// unknown line, so a new instruction cannot slip past the model.
fn assert_stack_shape(lines: &[String]) {
    let mut depth: i32 = 0;
    let mut reachable = true;
    for l in lines {
        if l.ends_with(':') {
            assert!(!reachable || depth == 0, "label {l} reached with depth {depth} in {lines:#?}");
            depth = 0;
            reachable = true;
            continue;
        }
        assert!(reachable, "unreachable instruction {l:?} after throw/br in {lines:#?}");
        let op = l.split_whitespace().next().unwrap_or("");
        let (pop, push) = match op {
            "ldarg" | "ldloc" | "ldc.i8" | "ldc.i4.0" | "ldstr" => (0, 1),
            "starg" | "stloc" => (1, 0),
            "add" | "sub" | "mul" | "div" | "xor" | "and" => (2, 1),
            "neg" | "conv.i4" | "conv.i8" => (1, 1),
            "bge" | "bne.un" | "beq" => (2, 0),
            "call" if l == TRAP[0] => (0, 1),
            "callvirt" if l == TRAP[1] => (1, 0),
            "newobj" if l == TRAP[3] => (1, 1),
            "throw" => (1, 0),
            _ => panic!("stack model does not know {l:?}"),
        };
        assert!(depth >= pop, "stack underflow at {l:?} in {lines:#?}");
        depth = depth - pop + push;
        if matches!(op, "bge" | "bne.un" | "beq") {
            assert_eq!(depth, 0, "branch {l:?} must leave the stack empty");
            let target = l.split_whitespace().nth(1).unwrap();
            assert_eq!(lines.iter().filter(|d| **d == format!("{target}:")).count(), 1, "label {target} defined once");
        }
        if op == "throw" {
            assert_eq!(depth, 0, "throw leaves nothing behind");
            reachable = false;
        }
    }
    // The lines stop just before `ret`: only the return value is left.
    assert_eq!(depth, 1, "only the return value remains for ret: {lines:#?}");
}

#[test]
fn add_and_sub_ovf_branch_around_one_trap_then_wrap() {
    for (op, arith) in [("i64_add_ovf", "add"), ("i64_sub_ovf", "sub")] {
        let lines = checked_il(op);
        let traps = overflow_traps(&lines);
        assert_eq!(traps.len(), 1, "{op}: {lines:#?}");
        let t = traps[0];
        assert_eq!(&lines[t - 3..t - 1], ["and", "ldc.i8 0"], "{op}: sign of the combined test");
        assert!(lines[t - 1].starts_with("bge "), "{op}: bge over the trap");
        // After the label: ld a; ld b; add|sub; st dest — never `add.ovf`.
        let tail = &lines[t + TRAP.len()..];
        assert_eq!(tail[1..4], ["ldarg 0", "ldarg 1", arith], "{op}: {tail:#?}");
        assert!(lines.iter().all(|l| !l.contains(".ovf")), "{op}: no OverflowException path");
        assert_stack_shape(&lines);
    }
}

#[test]
fn neg_ovf_compares_with_min() {
    let lines = checked_il("i64_neg_ovf");
    let traps = overflow_traps(&lines);
    assert_eq!(traps.len(), 1);
    assert_eq!(lines[traps[0] - 2], format!("ldc.i8 {}", i64::MIN));
    assert!(lines[traps[0] - 1].starts_with("bne.un "));
    assert!(lines.windows(2).any(|w| w == ["ldarg 0", "neg"]));
    assert_stack_shape(&lines);
}

#[test]
fn mul_ovf_guards_minus_one_times_min_before_dividing_back() {
    let lines = checked_il("i64_mul_ovf");
    let traps = overflow_traps(&lines);
    assert_eq!(traps.len(), 2, "{lines:#?}");
    // Guard 1 tests a == -1 then b == MIN, before any divide.
    let div = lines.iter().position(|l| l == "div").expect("div in the check");
    assert!(traps[0] < div && div < traps[1], "the -1 × MIN trap precedes the divide-back check");
    assert!(lines[..traps[0]].contains(&"ldc.i8 -1".to_string()));
    assert!(lines[..traps[0]].contains(&format!("ldc.i8 {}", i64::MIN)));
    // Guard 2: a == 0 skips, (a*b)/a == b skips.
    assert_eq!(lines.iter().filter(|l| l.starts_with("beq ")).count(), 2);
    let tail = &lines[traps[1] + TRAP.len()..];
    assert_eq!(tail[1..4], ["ldarg 0", "ldarg 1", "mul"], "{tail:#?}");
    assert_stack_shape(&lines);
}

#[test]
fn checked_ops_need_an_int64_dest_and_the_binary_path_refuses_them() {
    let narrow = module(
        vec![
            IIRInstr::new("i64_add_ovf", Some("q".into()), vec![v("a"), v("b")], "i32"),
            IIRInstr::new("ret", None, vec![v("q")], "i32"),
        ],
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
    );
    assert!(emit_il(&narrow, &IIRClrConfig::new("Main")).is_err(), "defined on i64 only");
    // The 32-bit simulator path (`lower_iir_to_cil`) does not lower PR4 ops;
    // it must refuse them with a typed error, never miscompile.
    let m = module(
        vec![
            IIRInstr::new("i64_mul_ovf", Some("q".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
        i64_params(),
    );
    assert!(lower_iir_to_cil(&m, &IIRClrConfig::new("Main")).is_err());
}
