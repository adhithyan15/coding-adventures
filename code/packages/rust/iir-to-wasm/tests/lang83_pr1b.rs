//! LANG83 PR1b on real WASM: `i64_div_floor`, `i64_mod_floor`, `trap`,
//! `write_i64`.
//!
//! Lower → encode → run on the in-process `wasm-runtime` with a host that
//! implements the two new imports:
//!
//! | Import | Host behaviour here |
//! |--------|---------------------|
//! | `env.__write_i64(i64)` | append the decimal text to the one output buffer |
//! | `env.putchar(i32)` | append the byte to the same buffer (ordering proof) |
//! | `env.__trap(i32)` | record the kind id, return — the module's own `unreachable` must still stop it |
//!
//! The last row is deliberate: a host that *returns* from `__trap` must not be
//! able to resume the program, so these tests prove the emitted `unreachable`.

use std::cell::RefCell;
use std::rc::Rc;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, validate_for_wasm, IIRWasmConfig};
use wasm_execution::{GlobalStorage, HostFunction, HostInterface, LinearMemory, Table, TrapError, WasmValue};
use wasm_types::{FuncType, GlobalType, ValueType};

#[derive(Default)]
struct State {
    out: Vec<u8>,
    trap_kind: Option<i32>,
}

struct Host(Rc<RefCell<State>>);

struct Import {
    ty: FuncType,
    state: Rc<RefCell<State>>,
    name: &'static str,
}

impl HostFunction for Import {
    fn func_type(&self) -> &FuncType {
        &self.ty
    }
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        let mut st = self.state.borrow_mut();
        match (self.name, args.first()) {
            ("__write_i64", Some(WasmValue::I64(n))) => st.out.extend_from_slice(n.to_string().as_bytes()),
            ("putchar", Some(WasmValue::I32(c))) => st.out.push(*c as u8),
            ("__trap", Some(WasmValue::I32(k))) => st.trap_kind = Some(*k),
            _ => return Err(TrapError::new("unexpected host call")),
        }
        Ok(vec![])
    }
}

impl HostInterface for Host {
    fn resolve_function(&self, module: &str, name: &str) -> Option<Box<dyn HostFunction>> {
        let (name, param) = match (module, name) {
            ("env", "__write_i64") => ("__write_i64", ValueType::I64),
            ("env", "putchar") => ("putchar", ValueType::I32),
            ("env", "__trap") => ("__trap", ValueType::I32),
            _ => return None,
        };
        Some(Box::new(Import { ty: FuncType { params: vec![param], results: vec![] }, state: Rc::clone(&self.0), name }))
    }
    fn resolve_global(&self, _: &str, _: &str) -> Option<(GlobalType, Rc<RefCell<GlobalStorage>>)> {
        None
    }
    fn resolve_memory(&self, _: &str, _: &str) -> Option<LinearMemory> {
        None
    }
    fn resolve_table(&self, _: &str, _: &str) -> Option<Table> {
        None
    }
}

fn v(n: &str) -> Operand {
    Operand::Var(n.into())
}

fn module(body: Vec<IIRInstr>) -> IIRModule {
    IIRModule {
        name: "lang83".into(),
        functions: vec![IIRFunction::new("main", vec![], "i64", body)],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

/// Run `main`; returns (stdout, trap kind id if `__trap` was called, completed?).
fn run(m: &IIRModule) -> (String, Option<i32>, bool) {
    let wasm = lower_iir_to_wasm(m, &IIRWasmConfig::default()).expect("lowering failed");
    let bytes = encode_module(&wasm).expect("encoding failed");
    let state = Rc::new(RefCell::new(State::default()));
    let rt = wasm_runtime::WasmRuntime::with_host(Box::new(Host(Rc::clone(&state))));
    let completed = rt.load_and_run(&bytes, "main", &[]).is_ok();
    let st = state.borrow();
    (String::from_utf8_lossy(&st.out).into_owned(), st.trap_kind, completed)
}

/// `write_i64(op(a, b)); putchar(' ')` for every pair, then return 0.
fn floor_program(op: &str, pairs: &[(i64, i64)]) -> IIRModule {
    let mut body = vec![IIRInstr::new("const", Some("sp".into()), vec![Operand::Int(32)], "i64")];
    for (i, (a, b)) in pairs.iter().enumerate() {
        let (x, y, r) = (format!("x{i}"), format!("y{i}"), format!("r{i}"));
        body.push(IIRInstr::new("const", Some(x.clone()), vec![Operand::Int(*a)], "i64"));
        body.push(IIRInstr::new("const", Some(y.clone()), vec![Operand::Int(*b)], "i64"));
        body.push(IIRInstr::new(op, Some(r.clone()), vec![v(&x), v(&y)], "i64"));
        body.push(IIRInstr::new("call_builtin", None, vec![v("write_i64"), v(&r)], "void"));
        body.push(IIRInstr::new("call_builtin", None, vec![v("putchar"), v("sp")], "void"));
    }
    body.push(IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"));
    body.push(IIRInstr::new("ret", None, vec![v("z")], "i64"));
    module(body)
}

#[test]
fn floor_div_follows_the_sign_table_on_wasm() {
    let (out, trap, done) = run(&floor_program("i64_div_floor", &[(7, 2), (-7, 2), (7, -2), (-7, -2), (i64::MIN, 1)]));
    assert!(done && trap.is_none());
    assert_eq!(out, "3 -4 -4 3 -9223372036854775808 ");
}

#[test]
fn floor_mod_follows_the_sign_table_on_wasm() {
    let (out, trap, done) = run(&floor_program("i64_mod_floor", &[(7, 2), (-7, 2), (7, -2), (-7, -2), (i64::MIN, -1), (i64::MIN, 1)]));
    assert!(done && trap.is_none());
    assert_eq!(out, "1 1 -1 -1 0 0 ");
}

#[test]
fn floor_ops_trap_with_their_kind_after_earlier_output() {
    // Trap.DivZero = 3, Trap.Overflow = 5 (exception_kind's stable ids).
    let (out, trap, done) = run(&floor_program("i64_div_floor", &[(-7, 2), (1, 0), (9, 9)]));
    assert_eq!((out.as_str(), trap, done), ("-4 ", Some(3), false));
    let (out, trap, done) = run(&floor_program("i64_div_floor", &[(5, 2), (i64::MIN, -1)]));
    assert_eq!((out.as_str(), trap, done), ("2 ", Some(5), false));
    let (out, trap, done) = run(&floor_program("i64_mod_floor", &[(5, 3), (5, 0)]));
    assert_eq!((out.as_str(), trap, done), ("2 ", Some(3), false));
}

#[test]
fn trap_stops_the_program_even_when_the_host_returns() {
    let m = module(vec![
        IIRInstr::new("const", Some("n".into()), vec![Operand::Int(-12)], "i64"),
        IIRInstr::new("call_builtin", None, vec![v("write_i64"), v("n")], "void"),
        IIRInstr::new("trap", None, vec![Operand::Str("Trap.User".into())], "void"),
        IIRInstr::new("call_builtin", None, vec![v("write_i64"), v("n")], "void"),
        IIRInstr::new("ret", None, vec![v("n")], "i64"),
    ]);
    let (out, trap, done) = run(&m);
    assert_eq!((out.as_str(), trap, done), ("-12", Some(6), false), "unreachable must follow env.__trap");
}

#[test]
fn trap_with_an_unregistered_kind_is_rejected_by_the_validator() {
    let m = module(vec![IIRInstr::new("trap", None, vec![Operand::Str("Nope".into())], "void")]);
    assert!(validate_for_wasm(&m).iter().any(|e| e.starts_with("InvalidTrap")), "{:?}", validate_for_wasm(&m));
}

#[test]
fn imports_are_appended_after_the_older_ones() {
    // A module using only the new ops imports exactly the two new functions.
    let wasm = lower_iir_to_wasm(&floor_program("i64_div_floor", &[(1, 1)]), &IIRWasmConfig::default()).unwrap();
    let names: Vec<&str> = wasm.imports.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["putchar", "__write_i64", "__trap"]);
}
