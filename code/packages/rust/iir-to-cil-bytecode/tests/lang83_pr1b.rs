//! LANG83 PR1b on the CLR textual (`emit_il`) path: `i64_div_floor`,
//! `i64_mod_floor`, `trap`, `write_i64`.
//!
//! No `dotnet`/`ilasm` on the development hosts, so these pin the emitted IL
//! line by line (CI's CLR column assembles and runs it on real CoreCLR). Every
//! branch label the sequences use must be defined exactly once.

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

fn il_of(m: &IIRModule) -> String {
    assert!(validate_iir_for_clr(m).is_empty(), "{:?}", validate_iir_for_clr(m));
    emit_il(m, &IIRClrConfig::new("Main")).expect("emit_il")
}

fn floor_il(op: &str) -> Vec<String> {
    let il = il_of(&module(
        vec![
            IIRInstr::new(op, Some("q".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
    ));
    // The body of the entry method (the one holding the op), trimmed lines.
    let start = il.find("ldarg 1").expect("loads b");
    il[start..].lines().map(|l| l.trim().to_string()).take_while(|l| !l.starts_with("ret")).collect()
}

const TRAP: [&str; 4] = [
    "call class [System.Runtime]System.IO.TextWriter [System.Console]System.Console::get_Out()",
    "callvirt instance void [System.Runtime]System.IO.TextWriter::Flush()",
    "newobj instance void [System.Runtime]System.Exception::.ctor(string)",
    "throw",
];

/// Indices where a full trap sequence for `kind` starts.
fn traps(lines: &[String], kind: &str) -> Vec<usize> {
    (0..lines.len().saturating_sub(4))
        .filter(|&i| {
            lines[i] == TRAP[0]
                && lines[i + 1] == TRAP[1]
                && lines[i + 2] == format!("ldstr \"{kind}\"")
                && lines[i + 3] == TRAP[2]
                && lines[i + 4] == TRAP[3]
        })
        .collect()
}

/// Every label used by a branch is defined exactly once.
fn assert_labels_resolve(lines: &[String]) {
    for l in lines {
        for br in ["br ", "bne.un ", "brtrue ", "brfalse "] {
            if let Some(target) = l.strip_prefix(br) {
                let defs = lines.iter().filter(|d| **d == format!("{target}:")).count();
                assert_eq!(defs, 1, "label {target} must be defined once in {lines:#?}");
            }
        }
    }
}

#[test]
fn i64_div_floor_guards_both_edges_before_div() {
    let lines = floor_il("i64_div_floor");
    let zero = traps(&lines, "Trap.DivZero");
    let ovf = traps(&lines, "Trap.Overflow");
    assert_eq!((zero.len(), ovf.len()), (1, 1), "{lines:#?}");
    let div = lines.iter().position(|l| l == "div").expect("div");
    assert!(zero[0] < ovf[0] && ovf[0] < div, "both edges are tested before the divide");
    assert!(lines.contains(&format!("ldc.i8 {}", i64::MIN)));
    // Floor fix-up: subtract the 0/1 adjustment widened to int64.
    let tail: Vec<&str> = lines[div..].iter().map(String::as_str).collect();
    assert!(tail.ends_with(&["and", "conv.i8", "sub", "stloc V_0"]) || tail.ends_with(&["and", "conv.i8", "sub", "starg 0"])
        || tail.windows(3).any(|w| w == ["and", "conv.i8", "sub"]), "{tail:#?}");
    assert_labels_resolve(&lines);
}

#[test]
fn i64_mod_floor_returns_zero_for_minus_one_and_adds_the_divisor() {
    let lines = floor_il("i64_mod_floor");
    assert_eq!(traps(&lines, "Trap.DivZero").len(), 1);
    assert!(traps(&lines, "Trap.Overflow").is_empty(), "MIN mod -1 is 0");
    let rem = lines.iter().position(|l| l == "rem").expect("rem");
    let zero_store = lines.iter().position(|l| l == "ldc.i8 0").expect("zero literal");
    assert!(lines.windows(4).any(|w| w[0] == "conv.i8" && w[1] == "ldarg 1" && w[2] == "mul" && w[3] == "add"),
        "r + adj*b: {lines:#?}");
    assert!(zero_store < rem);
    assert_labels_resolve(&lines);
}

#[test]
fn trap_flushes_console_out_then_throws_the_kind() {
    let il = il_of(&module(
        vec![
            IIRInstr::new("trap", None, vec![Operand::Str("Trap.User".into())], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![v("z")], "i64"),
        ],
        vec![],
    ));
    let lines: Vec<String> = il.lines().map(|l| l.trim().to_string()).collect();
    assert_eq!(traps(&lines, "Trap.User").len(), 1, "{il}");
}

#[test]
fn write_i64_calls_console_write_int64() {
    let il = il_of(&module(
        vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(-5)], "i64"),
            IIRInstr::new("call_builtin", None, vec![v("write_i64"), v("n")], "void"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ],
        vec![],
    ));
    assert!(il.contains("call void [System.Console]System.Console::Write(int64)"), "{il}");
    assert!(!il.contains("WriteLine(int64)"), "no newline");
}

#[test]
fn unregistered_trap_kind_is_refused() {
    let m = module(vec![IIRInstr::new("trap", None, vec![Operand::Str("Nope".into())], "void")], vec![]);
    assert!(emit_il(&m, &IIRClrConfig::new("Main")).is_err());
}

/// The binary `lower_iir_to_cil` path (the 32-bit `clr-simulator`) does not
/// lower the PR1b ops; it must refuse them with a typed error, never
/// miscompile. (`op_support` describes the textual path, LANG83 §2.)
#[test]
fn binary_artifact_path_refuses_the_pr1b_ops() {
    let m = module(
        vec![
            IIRInstr::new("i64_div_floor", Some("q".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
    );
    assert!(lower_iir_to_cil(&m, &IIRClrConfig::new("Main")).is_err());
}


/// Security-review note (LANG83 PR1b): the floor sequences' internal labels
/// used to be the fixed `'__lang83_<n>'`, so a user label of that exact
/// spelling collided with one (a duplicate label ilasm rejects — or, for a
/// branch to an *undefined* user label `__lang83_1`, a jump into the middle of
/// the guard sequence). The internal prefix now grows until no name the
/// function mentions starts with it.
#[test]
fn user_label_spelled_like_an_internal_floor_label_cannot_collide() {
    let il = il_of(&module(
        vec![
            IIRInstr::new("jmp", None, vec![v("__lang83_1")], "void"),
            IIRInstr::new("label", None, vec![v("__lang83_1")], "void"),
            IIRInstr::new("i64_div_floor", Some("q".into()), vec![v("a"), v("b")], "i64"),
            IIRInstr::new("ret", None, vec![v("q")], "i64"),
        ],
        vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
    ));
    let lines: Vec<String> = il.lines().map(|l| l.trim().to_string()).collect();
    // The user's label is defined exactly once — by the user.
    assert_eq!(lines.iter().filter(|l| *l == "'__lang83_1':").count(), 1, "{il}");
    // The internal labels moved to a longer prefix the user never mentioned.
    assert!(lines.iter().any(|l| l.starts_with("'___lang83_")), "{il}");
    assert_labels_resolve(&lines);
}
