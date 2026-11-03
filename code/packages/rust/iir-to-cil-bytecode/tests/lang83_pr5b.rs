//! LANG83 PR5b on the CLR (textual IL): a string holds one char per byte.
//!
//! | edge        | before PR5b                                  | PR5b                                             |
//! |-------------|----------------------------------------------|--------------------------------------------------|
//! | a literal   | printable ASCII only (`ldstr "…"`)            | any bytes; non-ASCII as `ldstr bytearray (b 00 …)` |
//! | `print_str` | `Console.Write(string)` (console encoding)    | `__iir_write_bytes(string)`: one byte per char    |
//! | `putchar`   | `Console.Write(char)` (`0xFF` → `C3 BF`)       | the byte as a one-char string, same writer        |
//! | `input_str` | `ReadLine` in the console's input encoding    | the launcher sets `Console.InputEncoding` Latin-1 |
//!
//! There is no CoreCLR on the development hosts, so these tests pin the IL,
//! and run a stack-depth model over the synthetic writer: CoreCLR rejects IL
//! whose stack shape differs between the edges into a label even when `ilasm`
//! assembles it. CI's CLR columns execute it.

use std::collections::HashMap;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_cil_bytecode::{emit_il, IIRClrConfig};

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn il_of(body: Vec<IIRInstr>) -> Result<String, String> {
    let mut m = IIRModule::new("Main", "test");
    m.functions.push(IIRFunction::new("main", vec![], "i64", body));
    m.entry_point = Some("main".into());
    emit_il(&m, &IIRClrConfig::default()).map_err(|e| e.to_string())
}

fn printing(literal: Operand) -> Vec<IIRInstr> {
    vec![
        IIRInstr::new("str_const", Some("s".into()), vec![literal], "str"),
        IIRInstr::new("print_str", None, vec![v("s")], "void"),
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![v("z")], "i64"),
    ]
}

#[test]
fn any_literal_is_one_char_per_byte() {
    let il = il_of(printing(Operand::Str("é".into()))).expect("a non-ASCII literal emits");
    assert!(il.contains("ldstr bytearray (C3 00 A9 00)"), "{il}");
    let il = il_of(printing(Operand::Bytes(vec![0x00, 0x41, 0xFF]))).expect("a raw-byte literal emits");
    assert!(il.contains("ldstr bytearray (00 00 41 00 FF 00)"), "{il}");
    // Printable ASCII keeps the quoted form, so no earlier IL changes.
    let il = il_of(printing(Operand::Str("HI\n".into()))).unwrap();
    assert!(il.contains(r#"ldstr "HI\n""#), "{il}");
}

/// The lines of one method, from its `.method` line to its closing brace.
fn method_lines(il: &str, name: &str) -> Vec<String> {
    let at = il.find(&format!(" {name}(")).unwrap_or_else(|| panic!("no method {name}"));
    let start = il[..at].rfind(".method").unwrap();
    il[start..]
        .lines()
        .skip(1)
        .map(str::trim)
        .take_while(|l| *l != "}")
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

fn call_effect(line: &str) -> (usize, usize) {
    let args = line.rsplit_once('(').unwrap().1.trim_end_matches(')');
    let n_args = if args.trim().is_empty() { 0 } else { args.split(',').count() };
    let instance = line.contains(" instance ");
    let ret = line.split_whitespace().nth(if instance { 2 } else { 1 }).unwrap();
    (n_args + usize::from(instance), usize::from(ret != "void"))
}

/// `(pops, pushes)` of each opcode the writer uses.
fn effect(line: &str) -> (usize, usize) {
    match line.split_whitespace().next().unwrap() {
        "ldarg" | "ldloc" | "ldc.i4" | "ldsfld" => (0, 1),
        "stloc" | "stsfld" | "brtrue" => (1, 0),
        "newarr" | "conv.u1" => (1, 1),
        "add" => (2, 1),
        "blt" => (2, 0),
        "stelem.i1" => (3, 0),
        "br" | "ret" => (0, 0),
        "call" | "callvirt" => call_effect(line),
        other => panic!("stack model does not know {other:?} in {line:?}"),
    }
}

/// Walk every path: labels and branches with an empty stack, no underflow,
/// `ret` (void) with an empty stack. Returns the deepest stack seen.
fn assert_stack_shape(code: &[String]) -> usize {
    let labels: HashMap<&str, usize> =
        code.iter().enumerate().filter_map(|(i, l)| l.strip_suffix(':').map(|n| (n, i))).collect();
    let mut depth_at: Vec<Option<usize>> = vec![None; code.len()];
    let mut work = vec![(0usize, 0usize)];
    let mut deepest = 0;
    while let Some((pc, depth)) = work.pop() {
        assert!(pc < code.len(), "control falls off the end");
        if let Some(d) = depth_at[pc] {
            assert_eq!(d, depth, "inconsistent depth at {:?}", code[pc]);
            continue;
        }
        depth_at[pc] = Some(depth);
        let line = &code[pc];
        if line.ends_with(':') {
            assert_eq!(depth, 0, "label {line} reached with a non-empty stack");
            work.push((pc + 1, 0));
            continue;
        }
        let (pops, pushes) = effect(line);
        assert!(depth >= pops, "stack underflow at {line:?}");
        let after = depth - pops + pushes;
        deepest = deepest.max(after);
        match line.split_whitespace().next().unwrap() {
            "ret" => assert_eq!(after, 0, "a void method returns with an empty stack"),
            op @ ("br" | "blt" | "brtrue") => {
                assert_eq!(after, 0, "branch {line:?} must leave the stack empty");
                let target = line.split_whitespace().nth(1).unwrap();
                work.push((labels[target], 0));
                if op != "br" {
                    work.push((pc + 1, 0));
                }
            }
            _ => work.push((pc + 1, after)),
        }
    }
    deepest
}

#[test]
fn print_str_and_putchar_go_through_one_byte_writer_with_a_sound_stack() {
    let mut body = printing(Operand::Bytes(vec![0xA9]));
    body.insert(2, IIRInstr::new("const", Some("c".into()), vec![Operand::Int(0xFF)], "i64"));
    body.insert(3, IIRInstr::new("call_builtin", None, vec![v("putchar"), v("c")], "void"));
    let il = il_of(body).unwrap();
    let entry = method_lines(&il, "MccarthyEntry");
    let writes = entry.iter().filter(|l| l.starts_with("call void ") && l.ends_with("Program::__iir_write_bytes(string)")).count();
    assert_eq!(writes, 2, "print_str and putchar both write bytes: {entry:#?}");
    assert!(!il.contains("System.Console::Write(char)") && !il.contains("System.Console::Write(string)"), "{il}");
    assert_eq!(il.matches(".field private static class [System.Runtime]System.IO.Stream __iir_stdout").count(), 1);
    let writer = method_lines(&il, "__iir_write_bytes");
    let code: Vec<String> = writer.into_iter().filter(|l| !l.starts_with('.')).collect();
    let deepest = assert_stack_shape(&code);
    assert!(deepest <= 8, "within .maxstack 8 (deepest {deepest})");
    // The writer flushes Console.Out before writing, so the stream stays ordered.
    assert!(code[1].ends_with("TextWriter::Flush()"), "{code:#?}");
    // A module with no string output carries no writer, field or input setup.
    let il = il_of(vec![
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![v("z")], "i64"),
    ])
    .unwrap();
    assert!(!il.contains("__iir_write_bytes") && !il.contains("__iir_stdout"), "{il}");
    assert!(!il.contains("set_InputEncoding"), "{il}");
}

#[test]
fn input_str_reads_latin1() {
    let il = il_of(vec![
        IIRInstr::new("call_builtin", Some("s".into()), vec![v("input_str")], "str"),
        IIRInstr::new("print_str", None, vec![v("s")], "void"),
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![v("z")], "i64"),
    ])
    .unwrap();
    let launcher = il.split(".entrypoint").nth(1).expect("a launcher");
    let set = launcher.find("System.Console::set_InputEncoding").expect("Latin-1 input");
    assert!(launcher[..set].contains("System.Text.Encoding::get_Latin1()"), "{launcher}");
    assert!(set < launcher.find("MccarthyEntry()").unwrap(), "set before the entry runs");
}

#[test]
fn a_function_may_not_be_named_iir_write_bytes() {
    let mut m = IIRModule::new("Main", "test");
    m.functions.push(IIRFunction::new("main", vec![], "i64", printing(Operand::Str("x".into()))));
    m.functions.push(IIRFunction::new("__iir_write_bytes", vec![], "i64", vec![
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![v("z")], "i64"),
    ]));
    m.entry_point = Some("main".into());
    let err = emit_il(&m, &IIRClrConfig::default()).unwrap_err().to_string();
    assert!(err.contains("__iir_write_bytes"), "{err}");
}
