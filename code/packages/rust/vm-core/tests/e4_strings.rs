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

fn run_with_ceiling(
    instrs: Vec<IIRInstr>,
    ret_ty: &str,
    ceiling: Option<u32>,
) -> Result<Option<Value>, vm_core::errors::VMError> {
    let f = IIRFunction::new("main", vec![], ret_ty, instrs);
    let mut module = IIRModule::new("e4", "e4");
    module.str_max_bytes = ceiling;
    module.add_or_replace(f);
    let mut vm = VMCore::new();
    vm.execute(&mut module, "main", &[])
}

fn assert_trap(err: vm_core::errors::VMError, kind: &str) {
    assert!(
        matches!(&err, vm_core::errors::VMError::Trap { kind: k } if k == kind),
        "expected VMError::Trap {{ kind: {kind:?} }}, got {err:?}"
    );
}

fn assert_bounds_trap(err: vm_core::errors::VMError) {
    assert_trap(err, "Trap.Bounds");
}

/// `str_index s, i` on "ABC" with `i` from a `const`.
fn index_abc(i: i64) -> Vec<IIRInstr> {
    vec![
        ins("str_const", Some("s"), vec![Operand::Str("ABC".into())], "str"),
        ins("const", Some("i"), vec![Operand::Int(i)], "i64"),
        ins("str_index", Some("b"), vec![Operand::Var("s".into()), Operand::Var("i".into())], "i64"),
        ins("ret", None, vec![Operand::Var("b".into())], "i64"),
    ]
}

/// `str_slice s, a, b` on "ABC", returning the slice's length.
fn slice_abc(a: i64, b: i64) -> Vec<IIRInstr> {
    vec![
        ins("str_const", Some("s"), vec![Operand::Str("ABC".into())], "str"),
        ins("const", Some("a"), vec![Operand::Int(a)], "i64"),
        ins("const", Some("b"), vec![Operand::Int(b)], "i64"),
        ins(
            "str_slice",
            Some("t"),
            vec![Operand::Var("s".into()), Operand::Var("a".into()), Operand::Var("b".into())],
            "str",
        ),
        ins("str_len", Some("n"), vec![Operand::Var("t".into())], "i64"),
        ins("ret", None, vec![Operand::Var("n".into())], "i64"),
    ]
}

/// LANG83 PR5a: every out-of-range index is `Trap.Bounds`, including the
/// values that only a full 64-bit check sees (`2³² + 1` narrows to 1).
#[test]
fn str_index_out_of_range_is_trap_bounds_for_every_edge() {
    for i in [-1, 3, i64::MAX, i64::MIN, (1 << 32) + 1] {
        assert_bounds_trap(run(index_abc(i), "i64").unwrap_err());
    }
    assert_eq!(run(index_abc(0), "i64").unwrap(), Some(Value::Int(65)));
    assert_eq!(run(index_abc(2), "i64").unwrap(), Some(Value::Int(67)));
}

#[test]
fn str_slice_out_of_range_is_trap_bounds_for_every_edge() {
    for (a, b) in [(-1, 2), (2, 1), (0, 4), (1 << 32, (1 << 32) + 3), (i64::MIN, 0), (0, i64::MAX)] {
        assert_bounds_trap(run(slice_abc(a, b), "i64").unwrap_err());
    }
    for (a, b, n) in [(0, 0, 0), (3, 3, 0), (0, 3, 3), (1, 2, 1)] {
        assert_eq!(run(slice_abc(a, b), "i64").unwrap(), Some(Value::Int(n)), "[{a}, {b})");
    }
}

/// `str_concat` of two 8-byte strings, then of that and one more byte, under
/// a module ceiling of 16.
fn concat_to_17() -> Vec<IIRInstr> {
    vec![
        ins("str_const", Some("a"), vec![Operand::Str("01234567".into())], "str"),
        ins("str_concat", Some("b"), vec![Operand::Var("a".into()), Operand::Var("a".into())], "str"),
        ins("str_len", Some("n"), vec![Operand::Var("b".into())], "i64"),
        ins("str_const", Some("x"), vec![Operand::Str("x".into())], "str"),
        ins("str_concat", Some("c"), vec![Operand::Var("b".into()), Operand::Var("x".into())], "str"),
        ins("ret", None, vec![Operand::Var("n".into())], "i64"),
    ]
}

/// LANG83 PR5a (owner decision Q2): past the module's string ceiling,
/// `str_concat` raises `Trap.Overflow`; exactly at the ceiling it succeeds.
#[test]
fn str_concat_past_the_module_ceiling_is_trap_overflow() {
    assert_trap(run_with_ceiling(concat_to_17(), "i64", Some(16)).unwrap_err(), "Trap.Overflow");
    // A ceiling of 17 admits the same program.
    assert_eq!(run_with_ceiling(concat_to_17(), "i64", Some(17)).unwrap(), Some(Value::Int(16)));
    // So does the registry ceiling (no `str_max_bytes`).
    assert_eq!(run_with_ceiling(concat_to_17(), "i64", None).unwrap(), Some(Value::Int(16)));
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
    // LANG83 PR5a: a registered trap, not an untyped `VMError::Custom`.
    assert_bounds_trap(err);
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
    assert_bounds_trap(err);
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
