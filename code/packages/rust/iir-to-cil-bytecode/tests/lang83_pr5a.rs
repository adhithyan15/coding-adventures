//! LANG83 PR5a on the CLR textual (`emit_il`) path: string failures are
//! registered traps, and every bound is checked on the full `int64` value
//! before it is narrowed to the `int32` that `System.String` takes.
//!
//! Before PR5a `str_index s, i` was `ld s; ld i; conv.i4; get_Chars`, so
//! `i = 2^32 + 1` read `s[1]`, an out-of-range index threw
//! `IndexOutOfRangeException` (no flush, no kind), and `str_concat` had no
//! ceiling.
//!
//! There is no `dotnet` here, so, like `lang83_pr6a.rs`, these tests check the
//! emitted IL two ways: its shape (the guard, the trap kind, the order of the
//! narrowing) and a **stack-depth model** over every control-flow path of the
//! entry method. CoreCLR rejects a method whose stack is not empty at a label,
//! or whose depth differs between two paths into it, even when `ilasm`
//! accepts it. CI runs the cases on a real CoreCLR through `lang-aot`.

use std::collections::HashMap;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_cil_bytecode::{emit_il, IIRClrConfig};

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

/// `main(params…) -> i64 { body }`, emitted as IL.
fn il(params: &[(&str, &str)], body: Vec<IIRInstr>, ceiling: Option<u32>) -> String {
    let mut m = IIRModule::new("Main", "test");
    m.functions.push(IIRFunction::new(
        "main",
        params.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect(),
        "i64",
        body,
    ));
    m.entry_point = Some("main".into());
    m.str_max_bytes = ceiling;
    emit_il(&m, &IIRClrConfig::default()).expect("emits IL")
}

/// The instruction lines of the entry method (`MccarthyEntry`): everything
/// after its `.method … {` line up to its closing `}`, minus directives.
fn entry_code(il: &str) -> Vec<String> {
    let at = il.find("MccarthyEntry(").expect("entry method");
    let start = il[..at].rfind(".method").unwrap();
    il[start..]
        .lines()
        .skip(1)
        .map(str::trim)
        .take_while(|l| *l != "}")
        .filter(|l| !l.is_empty() && !l.starts_with('.') && !l.starts_with("//"))
        .map(str::to_string)
        .collect()
}

fn call_effect(line: &str) -> (usize, usize) {
    let op = line.split_whitespace().next().unwrap();
    let args = line.rsplit_once('(').unwrap().1.trim_end_matches(')');
    let n_args = if args.trim().is_empty() { 0 } else { args.split(',').count() };
    let instance = line.contains(" instance ");
    match op {
        "newobj" => (n_args, 1),
        _ => {
            let ret = line.split_whitespace().nth(if instance { 2 } else { 1 }).unwrap();
            (n_args + usize::from(instance), usize::from(ret != "void"))
        }
    }
}

const BRANCHES: &[&str] = &["beq", "bne.un", "blt", "bge", "bgt", "ble", "blt.un", "bge.un", "bgt.un", "ble.un", "br"];

fn effect(line: &str) -> (usize, usize) {
    let op = line.split_whitespace().next().unwrap();
    match op {
        "ldarg" | "ldloc" | "ldstr" | "ldc.i4" | "ldc.i8" => (0, 1),
        "stloc" | "pop" => (1, 0),
        "add" | "sub" => (2, 1),
        "conv.i8" | "conv.i4" => (1, 1),
        "beq" | "bne.un" | "blt" | "bge" | "bgt" | "ble" | "blt.un" | "bge.un" | "bgt.un" | "ble.un" => (2, 0),
        "br" | "ret" => (0, 0),
        "throw" => (1, 0),
        "call" | "callvirt" | "newobj" => call_effect(line),
        _ => panic!("stack model does not know {line:?}"),
    }
}

/// Walk every path of the entry method: each label is reached with an empty
/// stack, no instruction underflows, branches leave the stack empty, `ret`
/// leaves one value, and `throw` ends its path.
fn assert_stack_shape(code: &[String]) {
    let labels: HashMap<&str, usize> =
        code.iter().enumerate().filter_map(|(i, l)| l.strip_suffix(':').map(|n| (n, i))).collect();
    let mut depth_at: Vec<Option<usize>> = vec![None; code.len()];
    let mut work = vec![(0usize, 0usize)];
    while let Some((pc, depth)) = work.pop() {
        assert!(pc < code.len(), "control falls off the end");
        match depth_at[pc] {
            Some(d) => {
                assert_eq!(d, depth, "inconsistent depth at {:?}", code[pc]);
                continue;
            }
            None => depth_at[pc] = Some(depth),
        }
        let line = &code[pc];
        if line.ends_with(':') {
            assert_eq!(depth, 0, "label {line} reached with a non-empty stack");
            work.push((pc + 1, depth));
            continue;
        }
        let (pops, pushes) = effect(line);
        assert!(depth >= pops, "stack underflow at {line:?}");
        let after = depth - pops + pushes;
        let op = line.split_whitespace().next().unwrap();
        match op {
            "ret" => assert_eq!(after, 1, "`ret` leaves the int64 result"),
            "throw" => {}
            _ if BRANCHES.contains(&op) => {
                assert_eq!(after, 0, "branch {line:?} must leave the stack empty");
                let target = line.split_whitespace().nth(1).unwrap();
                work.push((*labels.get(target).unwrap_or_else(|| panic!("undefined label {target}")), 0));
                if op != "br" {
                    work.push((pc + 1, after));
                }
            }
            _ => work.push((pc + 1, after)),
        }
    }
}

/// The index of the first line that starts with `prefix`.
fn first(code: &[String], prefix: &str) -> usize {
    code.iter().position(|l| l.starts_with(prefix)).unwrap_or_else(|| panic!("no {prefix:?} in {code:#?}"))
}

/// Every `ldstr "Trap.…"` (the kind of a trap sequence), in order.
fn trap_kinds(code: &[String]) -> Vec<String> {
    code.iter()
        .filter_map(|l| l.strip_prefix("ldstr \"Trap.").map(|k| format!("Trap.{}", k.trim_end_matches('"'))))
        .collect()
}

#[test]
fn str_index_checks_the_int64_index_unsigned_before_narrowing() {
    let code = entry_code(&il(
        &[("s", "str"), ("i", "i64")],
        vec![
            IIRInstr::new("str_index", Some("b".into()), vec![v("s"), v("i")], "i64"),
            IIRInstr::new("ret", None, vec![v("b")], "i64"),
        ],
        None,
    ));
    assert_stack_shape(&code);
    assert_eq!(trap_kinds(&code), ["Trap.Bounds"]);
    let guard = first(&code, "blt.un ");
    assert!(guard < first(&code, "conv.i4"), "the guard comes before the narrowing: {code:#?}");
    assert!(first(&code, "conv.i4") < first(&code, "callvirt instance char"), "{code:#?}");
}

#[test]
fn str_slice_checks_both_offsets_before_narrowing() {
    let code = entry_code(&il(
        &[("s", "str"), ("a", "i64"), ("b", "i64")],
        vec![
            IIRInstr::new("str_slice", Some("t".into()), vec![v("s"), v("a"), v("b")], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![v("t")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ],
        None,
    ));
    assert_stack_shape(&code);
    assert_eq!(trap_kinds(&code), ["Trap.Bounds"]);
    let guards: Vec<usize> = code.iter().enumerate().filter(|(_, l)| l.starts_with("bgt.un ")).map(|(i, _)| i).collect();
    assert_eq!(guards.len(), 2, "end >u len, then start >u end: {code:#?}");
    assert!(guards[1] < first(&code, "conv.i4"), "{code:#?}");
    assert!(first(&code, "conv.i4") < first(&code, "callvirt instance string [System.Runtime]System.String::Substring"));
}

#[test]
fn str_concat_checks_the_module_ceiling() {
    for (ceiling, want) in [(Some(16), 16i64), (None, i64::from(interpreter_ir::STR_MAX_BYTES))] {
        let code = entry_code(&il(
            &[("a", "str"), ("b", "str")],
            vec![
                IIRInstr::new("str_concat", Some("c".into()), vec![v("a"), v("b")], "str"),
                IIRInstr::new("str_len", Some("n".into()), vec![v("c")], "i64"),
                IIRInstr::new("ret", None, vec![v("n")], "i64"),
            ],
            ceiling,
        ));
        assert_stack_shape(&code);
        assert_eq!(trap_kinds(&code), ["Trap.Overflow"]);
        let limit = first(&code, &format!("ldc.i8 {want}"));
        assert_eq!(code[limit + 1].split_whitespace().next(), Some("ble.un"), "{code:#?}");
        assert!(limit < first(&code, "call string [System.Runtime]System.String::Concat"), "{code:#?}");
    }
}

/// #18800 item 1: every path to the CLR refuses a `str_const` longer than the
/// module's ceiling, and a ceiling above `STR_MAX_BYTES`, before it emits
/// anything: the binary path (`lower_iir_to_cil`, through
/// `validate_iir_for_clr`), the real-CoreCLR text path (`emit_il`) and the
/// typed-scalar path (`lower_typed_scalars_to_cil`). Before, the text path
/// (the one the CoreCLR column runs) compiled the 17-byte literal, and the
/// other two, which have no `str_const` lowering, refused it only as an
/// unsupported operation.
#[test]
fn a_literal_past_the_module_ceiling_is_refused_on_every_clr_path() {
    use iir_to_cil_bytecode::{lower_iir_to_cil, lower_typed_scalars_to_cil, validate_iir_for_clr, IIRClrError};
    let module = |len: usize, ceiling: Option<u32>| {
        let mut m = IIRModule::new("Main", "test");
        m.functions.push(IIRFunction::new("main", vec![], "i64", vec![
            IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("z".repeat(len))], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![v("s")], "i64"),
            IIRInstr::new("ret", None, vec![v("n")], "i64"),
        ]));
        m.str_max_bytes = ceiling;
        m
    };
    let config = IIRClrConfig::default();
    for (m, kind) in [(module(17, Some(16)), "StrTooLong"), (module(0, Some(interpreter_ir::STR_MAX_BYTES + 1)), "StrMaxBytes")] {
        let refusals = [
            ("lower_iir_to_cil", lower_iir_to_cil(&m, &config).err()),
            ("emit_il", emit_il(&m, &config).err()),
            ("lower_typed_scalars_to_cil", lower_typed_scalars_to_cil(&m, &config).err()),
        ];
        for (path, refusal) in refusals {
            match refusal {
                Some(IIRClrError::ValidationFailed(errors)) => {
                    assert!(errors.iter().any(|e| e.starts_with(kind)), "{path}: {errors:?}");
                }
                other => panic!("{path}: expected a {kind} validation failure, got {other:?}"),
            }
        }
        assert!(validate_iir_for_clr(&m).iter().any(|e| e.starts_with(kind)));
    }
    // At the ceiling the string-capable path still compiles.
    emit_il(&module(16, Some(16)), &config).expect("text path");
}
