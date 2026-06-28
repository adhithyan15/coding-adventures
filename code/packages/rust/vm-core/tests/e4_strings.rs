//! LANG-FULL E4 (layer 1) — reference VM semantics for shared string ops.
//!
//! Direct IIR tests prove the representation-agnostic string surface before a
//! frontend lowers source-language strings into it.

use std::sync::{Arc, Mutex};

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use vm_core::core::VMCore;
use vm_core::value::Value;

fn ins(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(op, dest.map(|s| s.to_string()), srcs, ty)
}

fn run(instrs: Vec<IIRInstr>, ret_ty: &str) -> Result<Option<Value>, vm_core::errors::VMError> {
    let f = IIRFunction::new("main", vec![], ret_ty, instrs);
    let mut module = IIRModule::new("e4", "e4");
    module.add_or_replace(f);
    let mut vm = VMCore::new();
    vm.execute(&mut module, "main", &[])
}

#[test]
fn str_len_counts_bytes() {
    let result = run(
        vec![
            ins("str_const", Some("s"), vec![Operand::Str("A\nB".into())], "str"),
            ins("str_len", Some("n"), vec![Operand::Var("s".into())], "i64"),
            ins("ret", None, vec![Operand::Var("n".into())], "i64"),
        ],
        "i64",
    )
    .unwrap();
    assert_eq!(result, Some(Value::Int(3)));
}

#[test]
fn str_concat_and_eq_return_i64_bool() {
    let result = run(
        vec![
            ins("str_const", Some("a"), vec![Operand::Str("AB".into())], "str"),
            ins("str_const", Some("b"), vec![Operand::Str("CDE".into())], "str"),
            ins("str_concat", Some("cat"), vec![Operand::Var("a".into()), Operand::Var("b".into())], "str"),
            ins("str_const", Some("want"), vec![Operand::Str("ABCDE".into())], "str"),
            ins("str_eq", Some("ok"), vec![Operand::Var("cat".into()), Operand::Var("want".into())], "i64"),
            ins("ret", None, vec![Operand::Var("ok".into())], "i64"),
        ],
        "i64",
    )
    .unwrap();
    assert_eq!(result, Some(Value::Int(1)));
}

#[test]
fn str_cmp_returns_three_way_order() {
    let result = run(
        vec![
            ins("str_const", Some("a"), vec![Operand::Str("ALPHA".into())], "str"),
            ins("str_const", Some("b"), vec![Operand::Str("BETA".into())], "str"),
            ins("str_cmp", Some("lt"), vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
            ins("str_cmp", Some("eq"), vec![Operand::Var("a".into()), Operand::Var("a".into())], "i64"),
            ins("str_cmp", Some("gt"), vec![Operand::Var("b".into()), Operand::Var("a".into())], "i64"),
            ins("add", Some("sum"), vec![Operand::Var("lt".into()), Operand::Var("eq".into())], "i64"),
            ins("add", Some("sum2"), vec![Operand::Var("sum".into()), Operand::Var("gt".into())], "i64"),
            ins("ret", None, vec![Operand::Var("sum2".into())], "i64"),
        ],
        "i64",
    )
    .unwrap();
    assert_eq!(result, Some(Value::Int(0)));
}

#[test]
fn str_slice_produces_substring_value() {
    let result = run(
        vec![
            ins("str_const", Some("s"), vec![Operand::Str("ABCDE".into())], "str"),
            ins("const", Some("start"), vec![Operand::Int(1)], "i64"),
            ins("const", Some("end"), vec![Operand::Int(4)], "i64"),
            ins(
                "str_slice",
                Some("sub"),
                vec![
                    Operand::Var("s".into()),
                    Operand::Var("start".into()),
                    Operand::Var("end".into()),
                ],
                "str",
            ),
            ins("str_len", Some("n"), vec![Operand::Var("sub".into())], "i64"),
            ins("ret", None, vec![Operand::Var("n".into())], "i64"),
        ],
        "i64",
    )
    .unwrap();
    assert_eq!(result, Some(Value::Int(3)));
}

#[test]
fn str_slice_traps_out_of_bounds() {
    let err = run(
        vec![
            ins("str_const", Some("s"), vec![Operand::Str("ABC".into())], "str"),
            ins("const", Some("start"), vec![Operand::Int(2)], "i64"),
            ins("const", Some("end"), vec![Operand::Int(4)], "i64"),
            ins(
                "str_slice",
                Some("sub"),
                vec![
                    Operand::Var("s".into()),
                    Operand::Var("start".into()),
                    Operand::Var("end".into()),
                ],
                "str",
            ),
        ],
        "str",
    )
    .unwrap_err();
    assert!(err.to_string().contains("str_slice"));
}

#[test]
fn str_index_returns_unsigned_byte() {
    let result = run(
        vec![
            ins("str_const", Some("s"), vec![Operand::Str("ABC".into())], "str"),
            ins("const", Some("i"), vec![Operand::Int(1)], "i64"),
            ins("str_index", Some("b"), vec![Operand::Var("s".into()), Operand::Var("i".into())], "i64"),
            ins("ret", None, vec![Operand::Var("b".into())], "i64"),
        ],
        "i64",
    )
    .unwrap();
    assert_eq!(result, Some(Value::Int(b'B' as i64)));
}

#[test]
fn str_index_traps_out_of_bounds() {
    let err = run(
        vec![
            ins("str_const", Some("s"), vec![Operand::Str("ABC".into())], "str"),
            ins("const", Some("i"), vec![Operand::Int(3)], "i64"),
            ins("str_index", Some("b"), vec![Operand::Var("s".into()), Operand::Var("i".into())], "i64"),
            ins("ret", None, vec![Operand::Var("b".into())], "i64"),
        ],
        "i64",
    )
    .unwrap_err();
    assert!(err.to_string().contains("str_index"));
}

#[test]
fn print_str_routes_through_builtin_sink_without_newline() {
    let output = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&output);
    let mut vm = VMCore::new();
    vm.builtins_mut().register("print_str", move |args| {
        let s = args.first().and_then(Value::as_str).unwrap();
        captured.lock().unwrap().push_str(s);
        Ok(Value::Null)
    });

    let f = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            ins("str_const", Some("s"), vec![Operand::Str("HELLO".into())], "str"),
            ins("print_str", None, vec![Operand::Var("s".into())], "void"),
            ins("const", Some("ok"), vec![Operand::Int(7)], "i64"),
            ins("ret", None, vec![Operand::Var("ok".into())], "i64"),
        ],
    );
    let mut module = IIRModule::new("e4p", "e4p");
    module.add_or_replace(f);

    assert_eq!(vm.execute(&mut module, "main", &[]).unwrap(), Some(Value::Int(7)));
    assert_eq!(&*output.lock().unwrap(), "HELLO");
}
