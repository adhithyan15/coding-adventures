//! End-to-end JIT tests: prove Oct programs flow through the LANG VM's
//! JIT chain (`vm-core` + `jit-core` + `GenericCirJit`).
//!
//! With `jit-core::GenericCirJit` landed, Oct gets a real JIT without
//! a per-language Backend impl.  This file is the proof: compile an Oct
//! source, hand the IIR to `JITCore::execute_with_jit` with a
//! `GenericCirJit` backend, and observe the program ran by either
//! - calling a function that returns a value and checking the return,
//! - or asserting the JIT's step counter ticked (proving a backward
//!   jump fired = the bytecode interpreter ran).

use std::sync::Mutex;

use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use oct_iir_compiler::compile_source;
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Helper: run an Oct source through the JIT chain and return the
/// result of `entry`'s call (interpreter or JIT-compiled — both paths
/// yield the same observable value).
fn run_oct_through_jit(source: &str, entry: &str) -> Value {
    let mut module = compile_source(source, "oct_jit_e2e")
        .expect("Oct source must compile");

    let entry_fn = module.functions.iter()
        .find(|f| f.name == entry)
        .unwrap_or_else(|| panic!("module must have function {entry:?}"));
    // Oct's IIR is statically typed, so the entry function should be
    // FullyTyped after the OCT03 fix.  Without that, JITCore's
    // threshold-zero compile path doesn't fire and the JIT never runs.
    assert_eq!(
        entry_fn.type_status,
        interpreter_ir::function::FunctionTypeStatus::FullyTyped,
        "Oct entry function {entry:?} should be FullyTyped; got: {:?}",
        entry_fn.type_status,
    );

    let mut vm = VMCore::new();
    let backend = GenericCirJit::new();
    let error_handle = backend.error_handle();
    let steps_handle = backend.steps_handle();

    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let result_opt = jit.execute_with_jit(&mut vm, &mut module, entry, &[])
        .expect("JIT execution must succeed");

    if let Some(e) = error_handle.lock().unwrap().clone() {
        panic!("GenericCirJit error: {e}");
    }
    // Expose the step counter via a side-channel so individual tests
    // can assert "loop body ran" without piping more state through.
    *LAST_STEPS.lock().unwrap() = *steps_handle.lock().unwrap();

    result_opt.unwrap_or(Value::Null)
}

/// Side-channel for individual tests to read the JIT's step counter
/// after the last `run_oct_through_jit` call.
static LAST_STEPS: Mutex<u64> = Mutex::new(0);

fn last_step_count() -> u64 {
    *LAST_STEPS.lock().unwrap()
}

/// Smallest possible JIT test: Oct's `answer` returns 42.
#[test]
fn oct_jit_returns_constant_42() {
    let src = "fn answer() -> u8 { return 42; } fn main() { }";
    let v = run_oct_through_jit(src, "answer");
    assert_eq!(v.as_i64(), Some(42),
        "Oct fn answer() -> u8 returning 42 should yield 42 via JIT; got: {v:?}");
}

/// Arithmetic + return — exercises `const_i64`, `add_i64`, `ret_i64`.
#[test]
fn oct_jit_arithmetic_and_return() {
    let src = "fn sum() -> u8 { let x: u8 = 30; let y: u8 = 12; return x + y; } \
               fn main() { }";
    let v = run_oct_through_jit(src, "sum");
    assert_eq!(v.as_i64(), Some(42),
        "Oct fn sum() -> u8 should compute 30+12 = 42 via JIT; got: {v:?}");
}

/// If/else through the JIT — exercises `cmp_eq_i64` + `jmp_if_false`
/// + `mov` + `jmp` + `label`.
#[test]
fn oct_jit_if_else() {
    let src = "fn pick() -> u8 { \
                   let x: u8 = 0; \
                   if x == 0 { x = 1; } else { x = 2; } \
                   return x; \
               } \
               fn main() { }";
    let v = run_oct_through_jit(src, "pick");
    assert_eq!(v.as_i64(), Some(1),
        "Oct if x==0 then x=1 else x=2 should produce x=1 via JIT; got: {v:?}");
}

/// `static` global, cross-function, RUN-VERIFIED (LANG-FULL O3).
///
/// `counter` is a module-level `static`.  `run` writes it (40), then calls
/// `bump` — a *different* function — twice; each `bump` reads-modifies-writes
/// the same global.  `run` finally reads it back.  If `static` lowered to a
/// per-function register (the pre-O3 behaviour, where a name is just a local
/// slot), `bump`'s mutations would be invisible to `run` and the result would
/// be 40, not 42.  Getting 42 proves the value lives in ONE shared module
/// global that survives across calls and is visible to every function — the
/// whole point of O3.  (The `= 99` initialiser is overwritten by `run`; the
/// initialiser path itself is proven by the `lang_matrix.rs` `out`-checked
/// program, which runs `main` end-to-end.)
#[test]
fn oct_jit_static_global_shared_across_functions() {
    let src = "static counter: u8 = 99; \
               fn bump() { counter = counter + 1; } \
               fn run() -> u8 { counter = 40; bump(); bump(); return counter; } \
               fn main() { }";
    let v = run_oct_through_jit(src, "run");
    assert_eq!(v.as_i64(), Some(42),
        "Oct static `counter` shared across run()/bump() should accumulate to \
         42 (40 + two bumps); a per-function register would give 40. Got: {v:?}");
}

/// While loop through the JIT — exercises backward `jmp` (the JIT's
/// bytecode loop) or the interpreter path's label/jmp dispatch.
/// Either way, the result must be 10 (the loop ran to completion).
///
/// We don't assert the step counter ticked because `JITCore` may
/// execute the function through the interpreter on the first call
/// (the compiled bytecode only fires on the *next* dispatch when the
/// cache entry exists).  Both paths yield the same observable value.
#[test]
fn oct_jit_while_loop() {
    let src = "fn count() -> u8 { \
                   let n: u8 = 0; \
                   while n < 10 { n = n + 1; } \
                   return n; \
               } \
               fn main() { }";
    let v = run_oct_through_jit(src, "count");
    assert_eq!(v.as_i64(), Some(10),
        "Oct while n<10 should iterate 10 times and return 10; got: {v:?}");
    // step counter exposed in case future tests want to assert; not
    // checked here (see doc-comment).
    let _ = last_step_count();
}
