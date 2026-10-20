//! LANG83 operation conformance — every standard operation, every backend.
//!
//! `code/specs/LANG83-iir-standard-operation-registry.md` makes every operation a
//! frontend may emit a first-class IIR op or standard builtin that **each backend
//! lowers itself** — or refuses at compile time. Each backend crate declares its
//! answer in an `op_support` table. This suite is where those declarations meet
//! reality:
//!
//! | Test | Claim it checks |
//! |------|-----------------|
//! | `every_registry_op_has_cases` | no registry op is without a conformance case |
//! | `every_lowered_op_runs_on_every_backend` | a `Lowered` declaration is true: the case compiles on every code-generation backend that declares it, and runs with the expected output on every executable column |
//! | `refused_ops_are_rejected_at_compile_time` | a `Refused` declaration is enforced: `check_module_for` *and* the backend's own compile entry point reject the case with a typed `UnsupportedOps` error |
//! | `print_i64_and_putchar_share_one_ordered_stream` | the LANG83 ordered-output contract for the one builtin pair no frontend emits together today |
//!
//! # The case corpus is IIR, not source
//!
//! Each [`Case`] builds its `IIRModule` directly, so an operation can be tested the
//! moment it enters the registry — before any frontend emits it — and so the
//! corpus says exactly which operations a case exercises: the registry operations
//! a case *uses* are computed from its module (`registry_entry` over every
//! instruction), never hand-listed, and a case runs on a backend only if the
//! backend lowers all of them.
//!
//! # Columns and skip policy
//!
//! | Column | What runs here | Skips when |
//! |--------|----------------|------------|
//! | WASM | compile + execute (in-process `wasm-runtime`) | never |
//! | VM | execute (`vm-core`) | never |
//! | JIT | execute (`jit-core`), and `main` must really be JIT-compiled | never (cases whose ops the JIT refuses are not JIT cases) |
//! | LLVM | compile + `clang` + execute | `clang` absent |
//! | native | compile + link + execute (Linux host) | not Linux |
//! | JVM, CLR, BEAM | compile to the artifact the column runs | never — execution of those columns stays in `lang_matrix` (JVM) or CI (CLR/BEAM) |
//!
//! The in-process columns follow `lang_matrix::proven_columns_do_not_silently_skip`:
//! they have no toolchain to be absent, so they never skip.
//!
//! # Outcomes
//!
//! [`Expect::StdoutThenTrap`] and [`RunResult::Trapped`]'s `stdout` exist so a
//! "print, then trap" case can assert both the output written before the trap and
//! the trap kind. In PR1a no backend lowers `trap` (it is a PR1b op), so the one
//! such case is exercised only by the refusal test; PR1b turns it into a running
//! case on every column.

use std::process::Command;
use std::sync::{Arc, Mutex};

use interpreter_ir::op_registry::{registry_entry, OpSupport, STANDARD_OPS};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lang_aot::{check_module_for, Backend, LangAotError};

mod common;

// ===========================================================================
// Expected outcomes and observed results
// ===========================================================================

/// What a case must do when run.
#[derive(Debug, Clone, Copy)]
enum Expect {
    /// Complete with this (trimmed) stdout.
    Stdout(&'static str),
    /// Write this (trimmed) stdout, then trap with this kind.
    StdoutThenTrap(&'static str, &'static str),
}

/// What a column observed.
#[derive(Debug)]
enum RunResult {
    /// The program returned normally.
    Completed { stdout: String },
    /// The program trapped. `stdout` is everything written before the trap;
    /// `kind` is the reported trap kind when the column can recover it.
    Trapped { stdout: String, kind: Option<String> },
}

fn assert_outcome(column: &str, case: &Case, result: RunResult) {
    match (case.expect, result) {
        (Expect::Stdout(want), RunResult::Completed { stdout }) => {
            assert_eq!(stdout.replace("\r\n", "\n"), want, "{column}: case `{}` stdout", case.name)
        }
        (Expect::StdoutThenTrap(want, kind), RunResult::Trapped { stdout, kind: got }) => {
            assert_eq!(stdout.replace("\r\n", "\n"), want, "{column}: case `{}` stdout before trap", case.name);
            if let Some(got) = got {
                assert_eq!(got, kind, "{column}: case `{}` trap kind", case.name);
            }
        }
        (expect, other) => panic!("{column}: case `{}` expected {expect:?}, got {other:?}", case.name),
    }
}

// ===========================================================================
// The case corpus
// ===========================================================================

/// One conformance case: a named IIR program and its expected outcome.
struct Case {
    name: &'static str,
    build: fn() -> IIRModule,
    expect: Expect,
}

impl Case {
    fn module(&self) -> IIRModule {
        (self.build)()
    }

    /// Every registry operation the case's module uses — computed, not listed.
    fn registry_ops(&self) -> Vec<&'static str> {
        let module = self.module();
        let mut ops: Vec<&'static str> = module
            .functions
            .iter()
            .flat_map(|f| f.instructions.iter())
            .filter_map(|i| registry_entry(i).map(|s| s.name))
            .collect();
        ops.sort_unstable();
        ops.dedup();
        ops
    }

    /// `true` if `backend` lowers every registry operation the case uses.
    fn lowered_on(&self, backend: Backend) -> bool {
        self.registry_ops().iter().all(|op| backend.op_support(op).is_lowered())
    }
}

// --- tiny IIR builders, so each case reads as a program ----------------------

fn var(name: &str) -> Operand {
    Operand::Var(name.into())
}
fn int(d: &str, n: i64) -> IIRInstr {
    IIRInstr::new("const", Some(d.into()), vec![Operand::Int(n)], "i64")
}
fn real(d: &str, x: f64) -> IIRInstr {
    IIRInstr::new("const", Some(d.into()), vec![Operand::Float(x)], "f64")
}
fn text(d: &str, s: &str) -> IIRInstr {
    IIRInstr::new("str_const", Some(d.into()), vec![Operand::Str(s.into())], "str")
}
fn op(name: &str, d: &str, srcs: &[&str], ty: &str) -> IIRInstr {
    IIRInstr::new(name, Some(d.into()), srcs.iter().map(|s| var(s)).collect(), ty)
}
fn builtin(name: &str, arg: &str) -> IIRInstr {
    IIRInstr::new("call_builtin", None, vec![var(name), var(arg)], "void")
}
fn print_i64(v: &str) -> IIRInstr {
    builtin("print_i64", v)
}
fn print_str(v: &str) -> IIRInstr {
    IIRInstr::new("print_str", None, vec![var(v)], "void")
}

/// `main() -> i64 { <body>; return 0 }` — exit code 0, observable via stdout.
fn program(mut body: Vec<IIRInstr>) -> IIRModule {
    body.push(int("__exit", 0));
    body.push(IIRInstr::new("ret", None, vec![var("__exit")], "i64"));
    let mut module = IIRModule::new("opconf", "iir");
    module.entry_point = Some("main".into());
    module.add_or_replace(IIRFunction::new("main", vec![], "i64", body));
    module
}

/// Two-operand integer op `name(a, b)`, printed.
fn int_binop(name: &str, a: i64, b: i64) -> IIRModule {
    program(vec![int("a", a), int("b", b), op(name, "r", &["a", "b"], "i64"), print_i64("r")])
}

/// Comparison `name(a, b)`, printed as 1/0 through a branch (the result is
/// never printed directly, so no backend's bool-to-integer convention matters).
///
/// The comparison carries the `i64` type hint, the shape ALGOL emits. The `bool`
/// hint Twig emits is NOT used here because the native pipeline miscompiles it
/// for negative operands: `jit_core::specialise` names the CIR op after the
/// *result* hint (`cmp_lt_bool`), so x86-64 compares the operands at `bool`
/// width and `-1 < 2` is false (Twig `(if (< (- 0 1) 2) 7 3)` exits 3 natively).
/// This suite found it; it is recorded as a separate follow-up rather than
/// fixed in the LANG83 contract slice.
fn compare(name: &str, a: i64, b: i64) -> IIRModule {
    program(vec![
        int("a", a),
        int("b", b),
        int("one", 1),
        int("zero", 0),
        op(name, "c", &["a", "b"], "i64"),
        IIRInstr::new("jmp_if_false", None, vec![var("c"), var("is_false")], "void"),
        print_i64("one"),
        IIRInstr::new("jmp", None, vec![var("done")], "void"),
        IIRInstr::new("label", None, vec![var("is_false")], "void"),
        print_i64("zero"),
        IIRInstr::new("label", None, vec![var("done")], "void"),
    ])
}

/// Unary `f64` op `name(x)`, truncated to an integer and printed.
fn f64_unary(name: &str, x: f64) -> IIRModule {
    program(vec![
        real("x", x),
        op(name, "y", &["x"], "f64"),
        op("real_to_int_trunc", "n", &["y"], "i64"),
        print_i64("n"),
    ])
}

/// The corpus. Values avoid every backend-dependent edge (no zero divisors, no
/// `MIN / -1`, results exactly representable), because those edges are what the
/// PR1b ops define — the legacy ops are only required to agree away from them.
fn cases() -> Vec<Case> {
    vec![
        // ── Output ──────────────────────────────────────────────────────────
        Case {
            name: "print_i64 writes the value and a newline",
            build: || program(vec![int("a", 42), int("b", -7), print_i64("a"), print_i64("b")]),
            expect: Expect::Stdout("42\n-7"),
        },
        Case {
            name: "putchar and print_i64 share one ordered stream",
            build: || {
                program(vec![
                    int("a", 65),
                    builtin("putchar", "a"),
                    int("n", 1),
                    print_i64("n"),
                    int("b", 66),
                    builtin("putchar", "b"),
                ])
            },
            expect: Expect::Stdout("A1\nB"),
        },
        Case {
            name: "print_str writes bytes without a newline",
            build: || program(vec![text("a", "he"), text("b", "llo"), print_str("a"), print_str("b")]),
            expect: Expect::Stdout("hello"),
        },
        Case {
            name: "write_i64 writes the value without a newline (PR1b)",
            build: || program(vec![int("a", 4), int("b", 2), builtin("write_i64", "a"), builtin("write_i64", "b")]),
            expect: Expect::Stdout("42"),
        },
        // ── Strings ─────────────────────────────────────────────────────────
        Case {
            name: "str_len counts bytes",
            build: || program(vec![text("s", "hello"), op("str_len", "n", &["s"], "i64"), print_i64("n")]),
            expect: Expect::Stdout("5"),
        },
        Case {
            name: "str_index reads one byte",
            build: || {
                program(vec![text("s", "HELLO"), int("i", 1), op("str_index", "c", &["s", "i"], "i64"), print_i64("c")])
            },
            expect: Expect::Stdout("69"),
        },
        Case {
            name: "str_slice takes [start, end)",
            build: || {
                program(vec![
                    text("s", "HELLO"),
                    int("a", 1),
                    int("b", 3),
                    op("str_slice", "t", &["s", "a", "b"], "str"),
                    print_str("t"),
                ])
            },
            expect: Expect::Stdout("EL"),
        },
        Case {
            name: "str_concat joins",
            build: || {
                program(vec![text("a", "AB"), text("b", "CD"), op("str_concat", "t", &["a", "b"], "str"), print_str("t")])
            },
            expect: Expect::Stdout("ABCD"),
        },
        Case {
            name: "str_eq compares bytes",
            build: || {
                program(vec![text("a", "xy"), text("b", "xy"), op("str_eq", "r", &["a", "b"], "i64"), print_i64("r")])
            },
            expect: Expect::Stdout("1"),
        },
        Case {
            name: "str_cmp orders lexically",
            build: || {
                program(vec![text("a", "a"), text("b", "b"), op("str_cmp", "r", &["a", "b"], "i64"), print_i64("r")])
            },
            expect: Expect::Stdout("-1"),
        },
        // ── Arithmetic ──────────────────────────────────────────────────────
        Case { name: "add", build: || int_binop("add", 40, 2), expect: Expect::Stdout("42") },
        Case { name: "sub", build: || int_binop("sub", -7, 2), expect: Expect::Stdout("-9") },
        Case { name: "mul", build: || int_binop("mul", 6, -7), expect: Expect::Stdout("-42") },
        Case { name: "div truncates", build: || int_binop("div", -7, 2), expect: Expect::Stdout("-3") },
        Case { name: "mod truncates", build: || int_binop("mod", -7, 2), expect: Expect::Stdout("-1") },
        Case {
            name: "neg",
            build: || program(vec![int("a", 5), op("neg", "r", &["a"], "i64"), print_i64("r")]),
            expect: Expect::Stdout("-5"),
        },
        Case {
            name: "i64_div_floor rounds toward -inf (PR1b)",
            build: || int_binop("i64_div_floor", -7, 2),
            expect: Expect::Stdout("-4"),
        },
        Case {
            name: "i64_mod_floor takes the divisor's sign (PR1b)",
            build: || int_binop("i64_mod_floor", -7, 2),
            expect: Expect::Stdout("1"),
        },
        // ── Comparison ──────────────────────────────────────────────────────
        Case { name: "cmp_eq", build: || compare("cmp_eq", 3, 3), expect: Expect::Stdout("1") },
        Case { name: "cmp_ne", build: || compare("cmp_ne", 3, 3), expect: Expect::Stdout("0") },
        Case { name: "cmp_lt", build: || compare("cmp_lt", -1, 2), expect: Expect::Stdout("1") },
        Case { name: "cmp_le", build: || compare("cmp_le", 2, 2), expect: Expect::Stdout("1") },
        Case { name: "cmp_gt", build: || compare("cmp_gt", -1, 2), expect: Expect::Stdout("0") },
        Case { name: "cmp_ge", build: || compare("cmp_ge", 1, 2), expect: Expect::Stdout("0") },
        // ── Float math (exact results only) ─────────────────────────────────
        Case { name: "f64_sqrt", build: || f64_unary("f64_sqrt", 16.0), expect: Expect::Stdout("4") },
        Case { name: "f64_sin", build: || f64_unary("f64_sin", 0.0), expect: Expect::Stdout("0") },
        Case { name: "f64_cos", build: || f64_unary("f64_cos", 0.0), expect: Expect::Stdout("1") },
        Case { name: "f64_ln", build: || f64_unary("f64_ln", 1.0), expect: Expect::Stdout("0") },
        Case { name: "f64_exp", build: || f64_unary("f64_exp", 0.0), expect: Expect::Stdout("1") },
        Case { name: "f64_atan", build: || f64_unary("f64_atan", 0.0), expect: Expect::Stdout("0") },
        Case { name: "f64_tan", build: || f64_unary("f64_tan", 0.0), expect: Expect::Stdout("0") },
        Case {
            name: "f64_pow",
            build: || {
                program(vec![
                    real("b", 2.0),
                    real("e", 10.0),
                    op("f64_pow", "y", &["b", "e"], "f64"),
                    op("real_to_int_trunc", "n", &["y"], "i64"),
                    print_i64("n"),
                ])
            },
            expect: Expect::Stdout("1024"),
        },
        // ── Conversions ─────────────────────────────────────────────────────
        Case {
            name: "int_to_real round-trips",
            build: || {
                program(vec![
                    int("n", -7),
                    op("int_to_real", "x", &["n"], "f64"),
                    op("real_to_int_floor", "m", &["x"], "i64"),
                    print_i64("m"),
                ])
            },
            expect: Expect::Stdout("-7"),
        },
        Case {
            name: "real_to_int_trunc rounds toward zero",
            build: || program(vec![real("x", -2.5), op("real_to_int_trunc", "n", &["x"], "i64"), print_i64("n")]),
            expect: Expect::Stdout("-2"),
        },
        Case {
            name: "real_to_int_floor rounds toward -inf",
            build: || program(vec![real("x", -2.5), op("real_to_int_floor", "n", &["x"], "i64"), print_i64("n")]),
            expect: Expect::Stdout("-3"),
        },
        // ── Errors ──────────────────────────────────────────────────────────
        Case {
            name: "trap flushes earlier output then reports its kind (PR1b)",
            build: || {
                program(vec![
                    text("s", "before"),
                    print_str("s"),
                    IIRInstr::new("trap", None, vec![Operand::Str("Trap.User".into())], "void"),
                ])
            },
            expect: Expect::StdoutThenTrap("before", "Trap.User"),
        },
    ]
}

// ===========================================================================
// Column runners
// ===========================================================================

/// The code-generation backends a case is compiled on.
const COMPILE_BACKENDS: &[Backend] =
    &[Backend::Llvm, Backend::Wasm, Backend::Jvm, Backend::Clr, Backend::Beam];

/// Compile `module` through `backend`'s lang-aot entry point; the artifact is
/// discarded (execution, where available, is a separate runner).
fn compile_on(backend: Backend, module: IIRModule) -> Result<(), LangAotError> {
    match backend {
        Backend::Llvm => lang_aot::compile_iir_to_llvm(module, "opconf", "x86_64-unknown-linux-gnu").map(drop),
        Backend::Wasm => lang_aot::compile_iir_to_wasm(module).map(drop),
        Backend::Jvm => lang_aot::compile_iir_to_jvm_class(module, "OpConf").map(drop),
        Backend::Clr => lang_aot::compile_iir_to_cil_text(module, "OpConf").map(drop),
        Backend::Beam => lang_aot::compile_iir_to_beam(module, "opconf").map(drop),
        Backend::X86_64 | Backend::Aarch64 => {
            let dir = tempfile::tempdir().expect("native: temp dir");
            native_compile(&module, &dir.path().join("prog"))
        }
        Backend::Vm | Backend::Jit => check_module_for(backend, &module).map_err(LangAotError::from),
    }
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
fn native_compile(module: &IIRModule, exe: &std::path::Path) -> Result<(), LangAotError> {
    lang_aot::compile_iir_to_host_executable(module, exe)
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn native_compile(module: &IIRModule, _exe: &std::path::Path) -> Result<(), LangAotError> {
    check_module_for(Backend::native_host(), module).map_err(LangAotError::from)
}

/// One shared, ordered output buffer (LANG83): every output builtin appends here.
type Sink = Arc<Mutex<Vec<u8>>>;

fn sink_text(sink: &Sink) -> String {
    String::from_utf8_lossy(&sink.lock().expect("output sink poisoned")).trim().to_string()
}

fn register_vm_output(vm: &mut vm_core::VMCore, sink: &Sink) {
    use vm_core::value::Value;
    let s = Arc::clone(sink);
    vm.builtins_mut().register("print_i64", move |args: &[Value]| {
        let n = args.first().and_then(Value::as_i64).unwrap_or(0);
        s.lock().expect("sink").extend_from_slice(format!("{n}\n").as_bytes());
        Ok(Value::Null)
    });
    let s = Arc::clone(sink);
    vm.builtins_mut().register("putchar", move |args: &[Value]| {
        let b = (args.first().and_then(Value::as_i64).unwrap_or(0) & 0xFF) as u8;
        s.lock().expect("sink").push(b);
        Ok(Value::Null)
    });
    let s = Arc::clone(sink);
    vm.builtins_mut().register("print_str", move |args: &[Value]| {
        let text = args.first().and_then(Value::as_str).unwrap_or("");
        s.lock().expect("sink").extend_from_slice(text.as_bytes());
        Ok(Value::Null)
    });
}

fn run_vm(module: IIRModule) -> RunResult {
    let mut module = module;
    let mut vm = vm_core::VMCore::new();
    let sink: Sink = Arc::default();
    register_vm_output(&mut vm, &sink);
    match vm.execute(&mut module, "main", &[]) {
        Ok(_) => RunResult::Completed { stdout: sink_text(&sink) },
        Err(_) => RunResult::Trapped { stdout: sink_text(&sink), kind: None },
    }
}

/// The JIT column. Only cases whose every op the JIT declares `Lowered` reach
/// here, so `main` must genuinely be JIT-compiled — asserting it is what stops
/// the interpreter fallback from passing as JIT support.
///
/// `main` is compiled explicitly with `JITCore::compile` rather than left to
/// `execute_with_jit`'s eager phase, because that phase only compiles
/// `FullyTyped` functions and `infer_type_status` does not count `"void"` as a
/// concrete type — so any function containing a `call_builtin … : void` (every
/// function that prints) is `PartiallyTyped` and would silently run on the
/// interpreter tier. (That is also true of the `lang_matrix` JIT column today;
/// LANG83 records it as a follow-up.)
fn run_jit(module: IIRModule, case: &Case) -> RunResult {
    use jit_core::core::JITCore;
    use jit_core::GenericCirJit;
    use vm_core::value::Value;
    let mut module = module;
    let sink: Sink = Arc::default();
    let mut vm = vm_core::VMCore::new();
    register_vm_output(&mut vm, &sink);
    let backend = GenericCirJit::new();
    let s = Arc::clone(&sink);
    backend.register_builtin("print_i64", move |args: &[Value]| {
        let n = args.first().and_then(Value::as_i64).unwrap_or(0);
        s.lock().expect("sink").extend_from_slice(format!("{n}\n").as_bytes());
        Value::Null
    });
    let s = Arc::clone(&sink);
    backend.register_builtin("putchar", move |args: &[Value]| {
        let b = (args.first().and_then(Value::as_i64).unwrap_or(0) & 0xFF) as u8;
        s.lock().expect("sink").push(b);
        Value::Null
    });
    // `run` cannot return an error, so a JIT-side trap lands in this slot.
    let error_slot = Arc::clone(&backend.error);
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let compiled = jit.compile(&mut vm, &module, "main");
    assert!(
        matches!(compiled, Ok(true)) && jit.is_compiled("main"),
        "Jit: case `{}` uses only JIT-Lowered ops, so `main` must be JIT-compiled ({compiled:?})",
        case.name
    );
    let result = jit.execute(&mut vm, &mut module, "main", &[]);
    let jit_error = error_slot.lock().expect("jit error slot").take();
    match (result, jit_error) {
        (Ok(_), None) => RunResult::Completed { stdout: sink_text(&sink) },
        _ => RunResult::Trapped { stdout: sink_text(&sink), kind: None },
    }
}

/// The body of a host import: arguments (and linear memory) in, results out.
type HostBody = Box<
    dyn Fn(
        &[wasm_execution::WasmValue],
        Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError>,
>;

/// A host import implemented by a closure, so the WASM host below stays a table.
struct HostFn {
    ty: wasm_types::FuncType,
    f: HostBody,
}

impl wasm_execution::HostFunction for HostFn {
    fn func_type(&self) -> &wasm_types::FuncType {
        &self.ty
    }
    fn call(
        &self,
        args: &[wasm_execution::WasmValue],
        memory: Option<&mut wasm_execution::LinearMemory>,
    ) -> Result<Vec<wasm_execution::WasmValue>, wasm_execution::TrapError> {
        (self.f)(args, memory)
    }
}

/// The WASM host: the output imports append to ONE ordered sink; the `f64`
/// math imports are libm through Rust's `f64` methods.
struct ConformanceHost {
    sink: Sink,
}

fn trap(msg: &str) -> wasm_execution::TrapError {
    wasm_execution::TrapError::new(msg)
}

fn arg_i64(args: &[wasm_execution::WasmValue], i: usize) -> Result<i64, wasm_execution::TrapError> {
    match args.get(i) {
        Some(wasm_execution::WasmValue::I64(v)) => Ok(*v),
        Some(wasm_execution::WasmValue::I32(v)) => Ok(i64::from(*v)),
        _ => Err(trap("expected an integer argument")),
    }
}

fn arg_f64(args: &[wasm_execution::WasmValue], i: usize) -> Result<f64, wasm_execution::TrapError> {
    match args.get(i) {
        Some(wasm_execution::WasmValue::F64(v)) => Ok(*v),
        _ => Err(trap("expected an f64 argument")),
    }
}

impl wasm_execution::HostInterface for ConformanceHost {
    fn resolve_function(&self, module: &str, name: &str) -> Option<Box<dyn wasm_execution::HostFunction>> {
        use wasm_execution::WasmValue;
        use wasm_types::{FuncType, ValueType::{F64, I32, I64}};
        if module != "env" {
            return None;
        }
        let sink = Arc::clone(&self.sink);
        let unary = |g: fn(f64) -> f64| -> Option<Box<dyn wasm_execution::HostFunction>> {
            Some(Box::new(HostFn {
                ty: FuncType { params: vec![F64], results: vec![F64] },
                f: Box::new(move |a, _| Ok(vec![WasmValue::F64(g(arg_f64(a, 0)?))])),
            }))
        };
        match name {
            "__print_i64" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![I64], results: vec![] },
                f: Box::new(move |a, _| {
                    let n = arg_i64(a, 0)?;
                    sink.lock().expect("sink").extend_from_slice(format!("{n}\n").as_bytes());
                    Ok(vec![])
                }),
            })),
            "putchar" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![I32], results: vec![] },
                f: Box::new(move |a, _| {
                    sink.lock().expect("sink").push((arg_i64(a, 0)? & 0xFF) as u8);
                    Ok(vec![])
                }),
            })),
            "__print_str" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![I32, I32], results: vec![] },
                f: Box::new(move |a, memory| {
                    let ptr = usize::try_from(arg_i64(a, 0)?).map_err(|_| trap("__print_str: bad ptr"))?;
                    let len = usize::try_from(arg_i64(a, 1)?).map_err(|_| trap("__print_str: bad len"))?;
                    let memory = memory.ok_or_else(|| trap("__print_str: no linear memory"))?;
                    let mut bytes = Vec::with_capacity(len);
                    for offset in 0..len {
                        bytes.push(memory.load_i32_8u(ptr + offset)? as u8);
                    }
                    sink.lock().expect("sink").extend_from_slice(&bytes);
                    Ok(vec![])
                }),
            })),
            "__sin" => unary(f64::sin),
            "__cos" => unary(f64::cos),
            "__ln" => unary(f64::ln),
            "__exp" => unary(f64::exp),
            "__atan" => unary(f64::atan),
            "__tan" => unary(f64::tan),
            "__pow" => Some(Box::new(HostFn {
                ty: FuncType { params: vec![F64, F64], results: vec![F64] },
                f: Box::new(|a, _| Ok(vec![WasmValue::F64(arg_f64(a, 0)?.powf(arg_f64(a, 1)?))])),
            })),
            _ => None,
        }
    }

    fn resolve_global(
        &self,
        _module: &str,
        _name: &str,
    ) -> Option<(wasm_types::GlobalType, std::rc::Rc<std::cell::RefCell<wasm_execution::GlobalStorage>>)> {
        None
    }

    fn resolve_memory(&self, _module: &str, _name: &str) -> Option<wasm_execution::LinearMemory> {
        None
    }

    fn resolve_table(&self, _module: &str, _name: &str) -> Option<wasm_execution::Table> {
        None
    }
}

fn run_wasm(module: IIRModule, case: &Case) -> RunResult {
    let bytes = lang_aot::compile_iir_to_wasm(module)
        .unwrap_or_else(|e| panic!("Wasm: case `{}` failed to compile: {e}", case.name));
    let sink: Sink = Arc::default();
    let rt = wasm_runtime::WasmRuntime::with_host(Box::new(ConformanceHost { sink: Arc::clone(&sink) }));
    match rt.load_and_run(&bytes, "main", &[]) {
        Ok(_) => RunResult::Completed { stdout: sink_text(&sink) },
        Err(_) => RunResult::Trapped { stdout: sink_text(&sink), kind: None },
    }
}

fn clang_ok() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

/// The LLVM column's print helpers (the harness C runtime LANG83 PR1b retires by
/// emitting self-contained definitions; until then it is linked like the
/// matrix's `PRINT_RUNTIME_C`).
const PRINT_RUNTIME_C: &str = "#include <stdio.h>\n#include <stdint.h>\n\
void __print_i64(int64_t x){printf(\"%lld\\n\",(long long)x);}\n\
void __print_str(const char* p,int64_t len){if(len>0){fwrite(p,1,(size_t)len,stdout);}}\n";

fn run_llvm(module: IIRModule, case: &Case) -> Option<RunResult> {
    if !clang_ok() {
        return None;
    }
    let triple = String::from_utf8(Command::new("clang").arg("-dumpmachine").output().ok()?.stdout)
        .ok()?
        .trim()
        .to_string();
    let ll = lang_aot::compile_iir_to_llvm(module, "opconf", &triple)
        .unwrap_or_else(|e| panic!("Llvm: case `{}` failed to compile: {e}", case.name));
    let dir = tempfile::tempdir().expect("llvm: temp dir");
    let ll_path = dir.path().join("prog.ll");
    std::fs::write(&ll_path, &ll).expect("llvm: write .ll");
    let exe = dir.path().join("prog");
    let mut cmd = Command::new("clang");
    cmd.arg("-x").arg("ir").arg(&ll_path);
    if ll.contains("@__twig_") || ll.contains("@__dyn_") {
        let rt = |n: &str| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime").join(n);
        cmd.arg("-x").arg("none").arg(rt("dynval_runtime.c")).args(common::gc_link_args()).arg(rt("twig_runtime.c"));
    }
    if ll.contains("@__print_") {
        let rt_path = dir.path().join("print_rt.c");
        std::fs::write(&rt_path, PRINT_RUNTIME_C).expect("llvm: write print runtime");
        cmd.arg("-x").arg("c").arg(&rt_path);
    }
    if cfg!(target_os = "linux") {
        cmd.arg("-x").arg("none").arg("-lm");
    }
    let built = cmd.arg("-o").arg(&exe).output().expect("llvm: spawn clang");
    assert!(
        built.status.success(),
        "Llvm: case `{}` did not link: {}",
        case.name,
        String::from_utf8_lossy(&built.stderr)
    );
    Some(run_process(Command::new(&exe)))
}

fn run_process(mut cmd: Command) -> RunResult {
    let out = cmd.output().expect("spawn compiled program");
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() {
        RunResult::Completed { stdout }
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let kind = stderr.split_whitespace().find(|w| w.contains("Trap.")).map(str::to_string);
        RunResult::Trapped { stdout, kind }
    }
}

#[cfg(target_os = "linux")]
fn run_native(module: IIRModule, case: &Case) -> Option<RunResult> {
    let dir = tempfile::tempdir().expect("native: temp dir");
    let exe = dir.path().join("prog");
    lang_aot::compile_iir_to_host_executable(&module, &exe)
        .unwrap_or_else(|e| panic!("NativeAot: case `{}` failed to compile: {e}", case.name));
    Some(run_process(Command::new(&exe)))
}

#[cfg(not(target_os = "linux"))]
fn run_native(_module: IIRModule, _case: &Case) -> Option<RunResult> {
    None
}

/// Run `case` on every executable column that lowers all of its ops; returns
/// the columns that ran.
fn execute_everywhere(case: &Case) -> Vec<&'static str> {
    let mut ran = Vec::new();
    if case.lowered_on(Backend::Wasm) {
        assert_outcome("Wasm", case, run_wasm(case.module(), case));
        ran.push("Wasm");
    }
    if case.lowered_on(Backend::Vm) {
        assert_outcome("Vm", case, run_vm(case.module()));
        ran.push("Vm");
    }
    if case.lowered_on(Backend::Jit) {
        assert_outcome("Jit", case, run_jit(case.module(), case));
        ran.push("Jit");
    }
    if case.lowered_on(Backend::Llvm) {
        if let Some(result) = run_llvm(case.module(), case) {
            assert_outcome("Llvm", case, result);
            ran.push("Llvm");
        }
    }
    if case.lowered_on(Backend::native_host()) {
        if let Some(result) = run_native(case.module(), case) {
            assert_outcome("NativeAot", case, result);
            ran.push("NativeAot");
        }
    }
    ran
}

// ===========================================================================
// The gates
// ===========================================================================

#[test]
fn every_registry_op_has_cases() {
    let corpus = cases();
    let missing: Vec<&str> = STANDARD_OPS
        .iter()
        .map(|s| s.name)
        .filter(|name| !corpus.iter().any(|c| c.registry_ops().contains(name)))
        .collect();
    assert!(missing.is_empty(), "registry ops with no conformance case: {missing:?}");
    // Case names are how failures are reported; keep them unambiguous.
    let mut names: Vec<&str> = corpus.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), corpus.len(), "duplicate case names");
}

#[test]
fn every_lowered_op_runs_on_every_backend() {
    let mut compiled = 0usize;
    let mut executed = 0usize;
    for case in cases() {
        // Compile on every code-generation backend that lowers all the case's ops.
        for &backend in COMPILE_BACKENDS.iter().chain(std::iter::once(&Backend::native_host())) {
            if case.lowered_on(backend) {
                if let Err(e) = compile_on(backend, case.module()) {
                    panic!("{backend}: case `{}` declares Lowered but failed to compile: {e}", case.name);
                }
                compiled += 1;
            }
        }
        // Execute on every executable column that lowers them.
        let ran = execute_everywhere(&case);
        // The in-process columns never skip a case their declarations accept.
        for (column, backend) in [("Wasm", Backend::Wasm), ("Vm", Backend::Vm)] {
            if case.lowered_on(backend) {
                assert!(ran.contains(&column), "{column} skipped case `{}`", case.name);
            }
        }
        executed += ran.len();
    }
    eprintln!("LANG83 conformance: {compiled} compilations, {executed} executions");
    assert!(compiled > 0 && executed > 0);
}

#[test]
fn refused_ops_are_rejected_at_compile_time() {
    let mut refusals = 0usize;
    for case in cases() {
        for backend in Backend::ALL {
            let refused: Vec<&str> = case
                .registry_ops()
                .into_iter()
                .filter(|op| matches!(backend.op_support(op), OpSupport::Refused(_)))
                .collect();
            if refused.is_empty() {
                continue;
            }
            // 1. The shared check names exactly the refused ops.
            let err = check_module_for(backend, &case.module())
                .expect_err("a module using a refused op must not pass check_module_for");
            let mut named: Vec<&str> = err.refusals.iter().map(|r| r.op).collect();
            named.sort_unstable();
            assert_eq!(named, refused, "{backend}: case `{}`", case.name);
            // 2. The backend's own compile entry point refuses it the same way,
            //    before any lowering pass. Only the host's native backend has a
            //    native entry point here; the other one is checked by step 1.
            let native = matches!(backend, Backend::X86_64 | Backend::Aarch64);
            if !native || backend == Backend::native_host() {
                match compile_on(backend, case.module()) {
                    Err(LangAotError::UnsupportedOps(e)) => assert_eq!(e.backend, backend),
                    other => panic!("{backend}: case `{}` must be refused with UnsupportedOps, got {other:?}", case.name),
                }
            }
            refusals += 1;
        }
    }
    // Every PR1b op is refused on all nine backends today.
    assert!(refusals >= 4 * Backend::ALL.len(), "only {refusals} refusals exercised");
}

#[test]
fn print_i64_and_putchar_share_one_ordered_stream() {
    let corpus = cases();
    let case = corpus
        .iter()
        .find(|c| c.registry_ops() == ["print_i64", "putchar"])
        .expect("the ordered-stream case must stay in the corpus");
    let ran = execute_everywhere(case);
    for column in ["Wasm", "Vm", "Jit"] {
        assert!(ran.contains(&column), "{column} must run the ordered-stream case");
    }
    eprintln!("LANG83 print_i64 + putchar ordering proven on {ran:?}");
}
