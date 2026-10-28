//! LANG83 floats on real WASM: the PR6c IEEE ops (`f64_add` … `f64_neg`,
//! `f64_signbit`) and the PR7b digit ops (`f64_class`, `f64_digits`,
//! `f64_exp10`) that identify their results.
//!
//! Lower → encode → run on the in-process `wasm-runtime`. The digits are
//! computed inside the module by the in-module helpers of
//! `src/lang83_f64.rs`; the module imports nothing for them. Every expected
//! digit below is host `python3`'s `repr`, split at the digits.
//!
//! This file began as PR6a's `write_f64_py` / `write_f64_js` tests. LANG83
//! PR7e removed both writers (a compiler lays a float out itself); the PR6a
//! table now runs through the compilers' helpers in `lang-aot`'s
//! `op_conformance`, on this column among others.

use std::cell::RefCell;
use std::rc::Rc;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_wasm::{encode_module, lower_iir_to_wasm, IIRWasmConfig};
use wasm_execution::{GlobalStorage, HostFunction, HostInterface, LinearMemory, Table, TrapError, WasmValue};
use wasm_types::{FuncType, GlobalType, ValueType};

struct Putchar(Rc<RefCell<Vec<u8>>>, FuncType);

impl HostFunction for Putchar {
    fn func_type(&self) -> &FuncType {
        &self.1
    }
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        match args.first() {
            Some(WasmValue::I32(c)) => self.0.borrow_mut().push(*c as u8),
            _ => return Err(TrapError::new("putchar: expected an i32")),
        }
        Ok(vec![])
    }
}

/// A host with `env.putchar`, `env.__write_i64` and `env.__print_str`, all
/// writing into one buffer.
struct Host(Rc<RefCell<Vec<u8>>>);

struct WriteI64(Rc<RefCell<Vec<u8>>>, FuncType);

impl HostFunction for WriteI64 {
    fn func_type(&self) -> &FuncType {
        &self.1
    }
    fn call(&self, args: &[WasmValue], _memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        match args.first() {
            Some(WasmValue::I64(n)) => self.0.borrow_mut().extend_from_slice(n.to_string().as_bytes()),
            _ => return Err(TrapError::new("__write_i64: expected an i64")),
        }
        Ok(vec![])
    }
}

struct PrintStr(Rc<RefCell<Vec<u8>>>, FuncType);

impl HostFunction for PrintStr {
    fn func_type(&self) -> &FuncType {
        &self.1
    }
    fn call(&self, args: &[WasmValue], memory: Option<&mut LinearMemory>) -> Result<Vec<WasmValue>, TrapError> {
        let (Some(WasmValue::I32(p)), Some(WasmValue::I32(n)), Some(mem)) = (args.first(), args.get(1), memory) else {
            return Err(TrapError::new("__print_str: bad call"));
        };
        for i in 0..*n as usize {
            let b = mem.load_i32_8u(*p as usize + i)? as u8;
            self.0.borrow_mut().push(b);
        }
        Ok(vec![])
    }
}

impl HostInterface for Host {
    fn resolve_function(&self, module: &str, name: &str) -> Option<Box<dyn HostFunction>> {
        match (module, name) {
            ("env", "putchar") => Some(Box::new(Putchar(
                Rc::clone(&self.0),
                FuncType { params: vec![ValueType::I32], results: vec![] },
            ))),
            ("env", "__write_i64") => Some(Box::new(WriteI64(
                Rc::clone(&self.0),
                FuncType { params: vec![ValueType::I64], results: vec![] },
            ))),
            ("env", "__print_str") => Some(Box::new(PrintStr(
                Rc::clone(&self.0),
                FuncType { params: vec![ValueType::I32, ValueType::I32], results: vec![] },
            ))),
            _ => None,
        }
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
        name: "lang83_f64".into(),
        functions: vec![IIRFunction::new("main", vec![], "i64", body)],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
        release: None,
    }
}

fn run(m: &IIRModule) -> String {
    run_with_result(m).0
}

/// Run `main`; its stdout and its return values.
fn run_with_result(m: &IIRModule) -> (String, Vec<i64>) {
    let wasm = lower_iir_to_wasm(m, &IIRWasmConfig::default()).expect("lowering failed");
    let bytes = encode_module(&wasm).expect("encoding failed");
    let out = Rc::new(RefCell::new(Vec::new()));
    let rt = wasm_runtime::WasmRuntime::with_host(Box::new(Host(Rc::clone(&out))));
    let result = rt.load_and_run(&bytes, "main", &[]).expect("no float op traps");
    let bytes = out.borrow().clone();
    (String::from_utf8(bytes).expect("ASCII output"), result)
}

/// The digit core's scratch region sits after the static data: a string
/// literal printed before and after the digit search over the widest values
/// (the largest finite double, the smallest subnormal) is untouched by its
/// big integers.
#[test]
fn scratch_region_does_not_overlap_string_data() {
    let body = vec![
        IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("before|".into())], "str"),
        IIRInstr::new("print_str", None, vec![v("s")], "void"),
        IIRInstr::new("const", Some("x".into()), vec![Operand::Float(1.7976931348623157e308)], "f64"),
        IIRInstr::new("f64_digits", Some("d".into()), vec![v("x")], "i64"),
        IIRInstr::new("call_builtin", None, vec![v("write_i64"), v("d")], "void"),
        IIRInstr::new("const", Some("y".into()), vec![Operand::Float(5e-324)], "f64"),
        IIRInstr::new("f64_exp10", Some("e".into()), vec![v("y")], "i64"),
        IIRInstr::new("call_builtin", None, vec![v("write_i64"), v("e")], "void"),
        IIRInstr::new("print_str", None, vec![v("s")], "void"),
        IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
        IIRInstr::new("ret", None, vec![v("z")], "i64"),
    ];
    assert_eq!(run(&module(body)), "before|17976931348623157-324before|");
}

/// LANG83 PR7e: `write_f64_py` / `write_f64_js` (the in-module
/// `$__iir_write_f64`) are gone, so a module that still calls one is
/// refused at lowering rather than given an ill-formed call.
#[test]
fn retired_float_writers_are_refused() {
    for name in ["write_f64_py", "write_f64_js"] {
        let body = vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Float(0.5)], "f64"),
            IIRInstr::new("call_builtin", None, vec![v(name), v("x")], "void"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![v("z")], "i64"),
        ];
        assert!(lower_iir_to_wasm(&module(body), &IIRWasmConfig::default()).is_err(), "{name}");
    }
}

// ── LANG83 PR6c: IEEE arithmetic and the sign bit ──────────────────────────

/// For each `(op, operands)`: compute on float constants, print the
/// result's `f64_class`, `f64_digits` and `f64_exp10` with `write_i64`, then
/// its `f64_signbit` as the digit `'0' + bit`, a space between, `\n` after
/// each row. (PR6a printed the result with `write_f64_js`, gone since PR7e;
/// class, digits, exponent and sign identify every value just as well.)
fn ieee(cases: &[(&str, &[f64])]) -> IIRModule {
    let mut body = vec![
        IIRInstr::new("const", Some("nl".into()), vec![Operand::Int(10)], "i64"),
        IIRInstr::new("const", Some("sp".into()), vec![Operand::Int(32)], "i64"),
        IIRInstr::new("const", Some("zero".into()), vec![Operand::Int(48)], "i64"),
    ];
    for (i, (op, xs)) in cases.iter().enumerate() {
        let srcs: Vec<Operand> = xs
            .iter()
            .enumerate()
            .map(|(k, x)| {
                let name = format!("c{i}_{k}");
                body.push(IIRInstr::new("const", Some(name.clone()), vec![Operand::Float(*x)], "f64"));
                v(&name)
            })
            .collect();
        let (r, s, d) = (format!("r{i}"), format!("s{i}"), format!("d{i}"));
        body.push(IIRInstr::new(*op, Some(r.clone()), srcs, "f64"));
        for query in ["f64_class", "f64_digits", "f64_exp10"] {
            let q = format!("{query}{i}");
            body.push(IIRInstr::new(query, Some(q.clone()), vec![v(&r)], "i64"));
            body.push(IIRInstr::new("call_builtin", None, vec![v("write_i64"), v(&q)], "void"));
            body.push(IIRInstr::new("call_builtin", None, vec![v("putchar"), v("sp")], "void"));
        }
        body.push(IIRInstr::new("f64_signbit", Some(s.clone()), vec![v(&r)], "i64"));
        body.push(IIRInstr::new("add", Some(d.clone()), vec![v(&s), v("zero")], "i64"));
        body.push(IIRInstr::new("call_builtin", None, vec![v("putchar"), v(&d)], "void"));
        body.push(IIRInstr::new("call_builtin", None, vec![v("putchar"), v("nl")], "void"));
    }
    body.push(IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"));
    body.push(IIRInstr::new("ret", None, vec![v("z")], "i64"));
    module(body)
}

/// The PR6c edge table on the in-process WASM runtime: nothing traps.
#[test]
fn pr6c_ieee_ops_follow_the_edge_table() {
    let inf = f64::INFINITY;
    let out = run(&ieee(&[
        ("f64_div", &[1.0, 0.0]),
        ("f64_div", &[-1.0, 0.0]),
        ("f64_mul", &[-1.0, 0.0]),
        ("f64_mul", &[1e308, 10.0]),
        ("f64_add", &[0.1, 0.2]),
        ("f64_sub", &[-0.0, 0.0]),
        ("f64_neg", &[0.0]),
        ("f64_neg", &[-inf]),
    ]));
    // `class digits exp10 sign`: Infinity, -Infinity, -0, Infinity,
    // 0.30000000000000004, -0, -0, Infinity.
    assert_eq!(
        out,
        "2 0 0 0\n2 0 0 1\n0 0 0 1\n2 0 0 0\n1 30000000000000004 -1 0\n0 0 0 1\n0 0 0 1\n2 0 0 0\n"
    );
    // NaN: class 3; its sign bit is unspecified (0 or 1).
    let nan = run(&ieee(&[("f64_div", &[0.0, 0.0]), ("f64_sub", &[inf, inf])]));
    for line in nan.lines() {
        assert!(line == "3 0 0 0" || line == "3 0 0 1", "{nan:?}");
    }
}

/// `f64_signbit` is ONE logical shift of the reinterpreted bits, so its
/// result is an i64 0/1 — never -1 from an arithmetic shift.
#[test]
fn pr6c_signbit_is_reinterpret_then_logical_shift() {
    // i64.reinterpret_f64 (0xBD), i64.const 63 (0x42 0x3F), i64.shr_u (0x88),
    // counted over every body; the digit core's own helpers are subtracted
    // out by lowering the same module without the op.
    let count = |cases: &[(&str, &[f64])]| {
        let wasm = lower_iir_to_wasm(&ieee(cases), &IIRWasmConfig::default()).unwrap();
        wasm.code.iter().map(|b| b.code.windows(4).filter(|w| *w == [0xBD, 0x42, 0x3F, 0x88]).count()).sum::<usize>()
    };
    assert_eq!(count(&[("f64_neg", &[1.0]), ("f64_neg", &[2.0])]) - count(&[("f64_neg", &[1.0])]), 1);
}

// ── LANG83 PR7b: f64_class / f64_digits / f64_exp10 ─────────────────────────

/// A module with three exported functions `f64_class(x)`, `f64_digits(x)`,
/// `f64_exp10(x)`, each returning its op's result for an `f64` parameter.
fn digit_queries() -> IIRModule {
    let query = |op: &str| {
        IIRFunction::new(
            op,
            vec![("x".to_string(), "f64".to_string())],
            "i64",
            vec![
                IIRInstr::new(op, Some("r".into()), vec![v("x")], "i64"),
                IIRInstr::new("ret", None, vec![v("r")], "i64"),
            ],
        )
    };
    IIRModule {
        name: "lang83_pr7b".into(),
        functions: vec![query("f64_class"), query("f64_digits"), query("f64_exp10")],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
        release: None,
    }
}

/// The three ops on the real in-process runtime, against host `python3`'s
/// `repr` split at the digits. The module prints nothing, so it imports no
/// `env.putchar`: the digit core writes nothing.
#[test]
fn pr7b_digit_queries_match_python_repr() {
    let wasm = lower_iir_to_wasm(&digit_queries(), &IIRWasmConfig::default()).expect("lowering failed");
    assert!(wasm.imports.iter().all(|i| i.name != "putchar"), "no output, so no putchar import");
    let bytes = encode_module(&wasm).expect("encoding failed");
    let rt = wasm_runtime::WasmRuntime::with_host(Box::new(Host(Rc::new(RefCell::new(Vec::new())))));
    let module = rt.load(&bytes).expect("load");
    let validated = rt.validate(&module).expect("validate");
    let mut instance = rt.instantiate(&validated).expect("instantiate");
    // (value, class, digits, exp10) — the digits are host python3's repr.
    let rows: &[(f64, i64, i64, i64)] = &[
        (1.5, 1, 15, 0),
        (-1.5, 1, 15, 0),
        (100.0, 1, 1, 2),
        (0.1, 1, 1, -1),
        (123.456, 1, 123456, 2),
        (1e21, 1, 1, 21),
        (1e-7, 1, 1, -7),
        (5e-324, 1, 5, -324),
        (2.2250738585072014e-308, 1, 22250738585072014, -308),
        (1.7976931348623157e308, 1, 17976931348623157, 308),
        (2.9802322387695312e-08, 1, 29802322387695312, -8),
        (1072012167405248.2, 1, 10720121674052482, 15),
        (0.0, 0, 0, 0),
        (-0.0, 0, 0, 0),
        (f64::INFINITY, 2, 0, 0),
        (f64::NEG_INFINITY, 2, 0, 0),
        (f64::NAN, 3, 0, 0),
    ];
    for &(x, class, digits, exp10) in rows {
        let mut call = |op: &str| match rt.call_typed(&mut instance, op, &[WasmValue::F64(x)]).expect(op).as_slice() {
            [WasmValue::I64(n)] => *n,
            other => panic!("{op}({x:e}) returned {other:?}"),
        };
        assert_eq!(call("f64_class"), class, "f64_class({x:e})");
        assert_eq!(call("f64_digits"), digits, "f64_digits({x:e})");
        assert_eq!(call("f64_exp10"), exp10, "f64_exp10({x:e})");
    }
}

/// A module that asks for both the digits and the exponent of several values
/// carries ONE digit core (main + its ten helpers, nothing duplicated), and
/// imports nothing for it.
#[test]
fn pr7b_digit_core_is_emitted_once() {
    let body = vec![
        IIRInstr::new("const", Some("x".into()), vec![Operand::Float(123.456)], "f64"),
        IIRInstr::new("f64_digits", Some("d".into()), vec![v("x")], "i64"),
        IIRInstr::new("const", Some("y".into()), vec![Operand::Float(1e21)], "f64"),
        IIRInstr::new("f64_exp10", Some("e".into()), vec![v("y")], "i64"),
        IIRInstr::new("f64_digits", Some("d2".into()), vec![v("y")], "i64"),
        IIRInstr::new("add", Some("t".into()), vec![v("d"), v("e")], "i64"),
        IIRInstr::new("add", Some("u".into()), vec![v("t"), v("d2")], "i64"),
        IIRInstr::new("ret", None, vec![v("u")], "i64"),
    ];
    let m = module(body);
    let wasm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
    assert_eq!(wasm.code.len(), 1 + 10);
    assert!(wasm.imports.is_empty(), "{:?}", wasm.imports);
    let (out, result) = run_with_result(&m);
    assert_eq!(out, "");
    assert_eq!(result, vec![123_456 + 21 + 1]);
}
