//! End-to-end JIT tests that exercise the REAL `BasicCirJit` backend
//! (not `NullBackend`).
//!
//! `tests/jit_smoke.rs` proves the IIR flows through the JIT chain even
//! with a no-op backend.  This file is the **real-JIT** counterpart:
//! we hand `JITCore` a `BasicCirJit` instance and assert that the
//! compiled bytecode produces the same observable output as the
//! interpreter path.
//!
//! ## What gets exercised
//!
//! - `BasicCirJit::compile` translates BASIC's specialised CIR
//!   (`const_i64`, `add_i64`, `cmp_*_i64`, `jmp`, `jmp_if_false`,
//!   `call_builtin "print_i64"`, `ret_void`) into a packed bytecode.
//! - `BasicCirJit::run` interprets that bytecode in a tight loop and
//!   pushes printed integers into the shared `Arc<Mutex<Vec<i64>>>`
//!   output buffer.
//! - `JITCore` invokes `compile` once (BASIC's main is FullyTyped, so
//!   the threshold-zero path fires before the first interpreted call)
//!   and routes subsequent dispatches through the compiled handler.
//!
//! ## Why a separate file?
//!
//! Keeping the real-JIT tests separate from the NullBackend smoke tests
//! lets us flip between the two backends without recompiling tests, and
//! makes it easy to compare bytecode-JIT-only output (this file) with
//! interpreter-only output (jit_smoke.rs).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use dartmouth_basic_iir_compiler::{compile_source, BasicCirJit};
use jit_core::core::JITCore;
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Run a BASIC source through the JIT chain backed by `BasicCirJit` and
/// return everything the program PRINTed.  Each `PRINT n` becomes one
/// entry in the returned vec.
fn run_with_real_jit(source: &str) -> String {
    let mut module = compile_source(source, "real_jit_demo")
        .expect("BASIC source must compile");

    // Sanity: the main function must be FullyTyped, otherwise jit-core
    // won't fire its threshold-zero compile path.
    let main = module.functions.iter()
        .find(|f| f.name == "main")
        .expect("BASIC module must have a `main` function");
    assert_eq!(
        main.type_status,
        interpreter_ir::function::FunctionTypeStatus::FullyTyped,
        "BASIC's main must be FullyTyped for the JIT threshold-zero \
         compile path to fire; got: {:?}", main.type_status,
    );

    let mut vm = VMCore::new();

    // The shared output buffer that BasicCirJit's PRINT_I64 opcode
    // pushes into.  We also register a matching `print_i64` builtin
    // on the VM so the *interpreter* fallback (used for any
    // FullyTyped → PartiallyTyped → Untyped tier drops, and for ops
    // BasicCirJit's compile() rejects) sees the same output sink.
    let output: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let input: Arc<Mutex<VecDeque<i64>>> = Arc::new(Mutex::new(VecDeque::new()));
    let steps: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    let error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // BA2: BASIC `PRINT` now renders characters through the universal `putchar`
    // builtin (digits via the synthetic recursive `__basic_print_int` helper,
    // separator spaces, and the line-ending newline) rather than the old
    // line-buffered `print_i64`. `BasicCirJit` only compiles the `print_i64`
    // opcode + the straight-line ops; it refuses (returns `None` →
    // interpreter-fallback) on the helper's `call`/`call_builtin putchar`, so
    // the byte stream is produced by the VM interpreter through this builtin.
    let chars: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    {
        let chars = Arc::clone(&chars);
        vm.builtins_mut().register("putchar", move |args| {
            let b = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            chars.lock().unwrap().push(b as u8);
            Ok(Value::Null)
        });
    }
    // The legacy `print_i64` sink stays registered (and is shared with
    // `BasicCirJit` below) so any path that still emits it has a home; BA2
    // BASIC no longer emits it, so it simply stays empty.
    {
        let output = Arc::clone(&output);
        vm.builtins_mut().register("print_i64", move |args| {
            let n = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
            output.lock().unwrap().push(n);
            Ok(Value::Null)
        });
    }
    {
        let input = Arc::clone(&input);
        vm.builtins_mut().register("input_i64", move |_args| {
            let v = input.lock().unwrap().pop_front().unwrap_or(0);
            Ok(Value::Int(v))
        });
    }

    // The real JIT backend, sharing buffers with the interpreter
    // builtins above.
    let backend = BasicCirJit::new(
        Arc::clone(&output),
        Arc::clone(&input),
        Arc::clone(&steps),
        Arc::clone(&error),
        None,
        None,
    );
    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    jit.execute_with_jit(&mut vm, &mut module, "main", &[])
        .expect("JIT execution must succeed");

    // Surface any JIT-side error so test failures are diagnostic.
    if let Some(msg) = error.lock().unwrap().clone() {
        panic!("BasicCirJit reported runtime error: {msg}");
    }

    // Bind the cloned bytes to a local first so the `MutexGuard` temporary is
    // dropped here (before `chars` goes out of scope at the closing brace) —
    // a tail `chars.lock().unwrap().clone()` expression holds the guard until
    // the block ends, which older rustc rejects (E0597).
    let bytes = chars.lock().unwrap().clone();
    String::from_utf8(bytes).expect("BASIC PRINT output must be valid UTF-8")
}

/// Smallest possible PRINT test through the real JIT.
#[test]
fn real_jit_basic_print_42() {
    let got = run_with_real_jit("10 PRINT 42\n20 END\n");
    assert_eq!(got, "42\n",
        "BasicCirJit should print `42` + newline, got {got:?}");
}

/// LET + arithmetic + PRINT through the real JIT.
#[test]
fn real_jit_basic_let_arithmetic_print() {
    let src = "10 LET A = 30\n\
               20 LET B = 12\n\
               30 PRINT A + B\n\
               40 END\n";
    let got = run_with_real_jit(src);
    assert_eq!(got, "42\n",
        "BasicCirJit should print `42` from 30 + 12, got {got:?}");
}

/// FOR / NEXT through the real JIT — exercises `jmp` / `jmp_if_false` /
/// `add_i64` / `cmp_le_i64` in the bytecode interpreter.
#[test]
fn real_jit_basic_for_loop_prints_1_2_3() {
    let src = "10 FOR I = 1 TO 3\n\
               20 PRINT I\n\
               30 NEXT I\n\
               40 END\n";
    let got = run_with_real_jit(src);
    assert_eq!(got, "1\n2\n3\n",
        "BasicCirJit should print 1,2,3 (each on its own line) from FOR I = 1 TO 3, got {got:?}");
}

/// IF / GOTO through the real JIT — exercises `cmp_gt_i64` +
/// `jmp_if_true` + forward jmp.
#[test]
fn real_jit_basic_if_then_goto() {
    let src = "10 LET A = 7\n\
               20 IF A > 5 THEN 100\n\
               30 PRINT 0\n\
               40 END\n\
               100 PRINT A\n\
               110 END\n";
    let got = run_with_real_jit(src);
    assert_eq!(got, "7\n",
        "BasicCirJit should print `7` from IF A > 5 THEN 100, got {got:?}");
}

/// Multiplication via PRINT.  Exercises `mul_i64`.
#[test]
fn real_jit_basic_multiplication() {
    let src = "10 LET A = 6\n\
               20 LET B = 7\n\
               30 PRINT A * B\n\
               40 END\n";
    let got = run_with_real_jit(src);
    assert_eq!(got, "42\n",
        "BasicCirJit should print `42` from 6 * 7, got {got:?}");
}

/// Two-iteration FOR with arithmetic in the body — proves the
/// backward jump and step counter work over multiple iterations.
#[test]
fn real_jit_basic_for_loop_accumulator() {
    let src = "10 LET S = 0\n\
               20 FOR I = 1 TO 5\n\
               30 LET S = S + I\n\
               40 NEXT I\n\
               50 PRINT S\n\
               60 END\n";
    let got = run_with_real_jit(src);
    // 1 + 2 + 3 + 4 + 5 = 15
    assert_eq!(got, "15\n",
        "BasicCirJit should compute 1+2+3+4+5 = 15, got {got:?}");
}
