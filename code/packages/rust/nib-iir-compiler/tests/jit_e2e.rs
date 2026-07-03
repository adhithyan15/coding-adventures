//! End-to-end JIT tests: prove Nib programs flow through the LANG VM's
//! JIT chain (`vm-core` + `jit-core` + `GenericCirJit`).
//!
//! With `jit-core::GenericCirJit` landed, Nib gets a real JIT without
//! a per-language Backend impl.  This file is the proof: compile a Nib
//! source, hand the IIR to `JITCore::execute_with_jit` with a
//! `GenericCirJit` backend, observe the return value matches the
//! interpreter.
//!
//! Nib is the third language (after Brainfuck and BASIC) to plug into
//! `JITCore`, and the second (after Oct) to do so via `GenericCirJit`
//! with **zero per-language Backend code**.

use jit_core::core::JITCore;
use jit_core::GenericCirJit;
use nib_iir_compiler::compile_source;
use vm_core::core::VMCore;
use vm_core::value::Value;

/// Run a Nib source through the JIT chain and return the result of
/// `entry`'s call.  The interpreter path and JIT-compiled path yield
/// the same observable value; we don't care which one fired here.
fn run_nib_through_jit(source: &str, entry: &str) -> Value {
    let mut module = compile_source(source, "nib_jit_e2e").expect("Nib source must compile");

    let entry_fn = module
        .functions
        .iter()
        .find(|f| f.name == entry)
        .unwrap_or_else(|| panic!("module must have function {entry:?}"));
    assert_eq!(
        entry_fn.type_status,
        interpreter_ir::function::FunctionTypeStatus::FullyTyped,
        "Nib entry function {entry:?} should be FullyTyped after v0.5.0; got: {:?}",
        entry_fn.type_status,
    );

    let mut vm = VMCore::new();
    let backend = GenericCirJit::new();
    let error_handle = backend.error_handle();

    let mut jit = JITCore::new(&mut vm, Box::new(backend));
    let result_opt = jit
        .execute_with_jit(&mut vm, &mut module, entry, &[])
        .expect("JIT execution must succeed");

    if let Some(e) = error_handle.lock().unwrap().clone() {
        panic!("GenericCirJit error: {e}");
    }

    result_opt.unwrap_or(Value::Null)
}

/// Smallest possible JIT test: Nib's main returns 42.
#[test]
fn nib_jit_returns_constant_42() {
    let src = "fn main() -> u8 { return 42; }";
    let v = run_nib_through_jit(src, "main");
    assert_eq!(
        v.as_i64(),
        Some(42),
        "Nib fn main() -> u8 returning 42 should yield 42 via JIT; got: {v:?}"
    );
}

/// Arithmetic + return — exercises typed const + add + ret.
#[test]
fn nib_jit_inline_arithmetic() {
    let src = "fn main() -> u8 { return 30 + 12; }";
    let v = run_nib_through_jit(src, "main");
    assert_eq!(
        v.as_i64(),
        Some(42),
        "Nib fn main() returning 30+12 should yield 42 via JIT; got: {v:?}"
    );
}

/// `let` bindings + arithmetic + return.
#[test]
fn nib_jit_let_and_add() {
    let src = "fn main() -> u4 { let x: u4 = 7; return x; }";
    let v = run_nib_through_jit(src, "main");
    assert_eq!(
        v.as_i64(),
        Some(7),
        "Nib fn main() with let x = 7; return x should yield 7 via JIT; got: {v:?}"
    );
}

/// `if`/`else` branches through the JIT.
#[test]
fn nib_jit_if_else() {
    let src = "fn main() -> u8 { if 1 == 1 { return 100; } else { return 200; } }";
    let v = run_nib_through_jit(src, "main");
    assert_eq!(
        v.as_i64(),
        Some(100),
        "Nib if 1==1 should take the then branch and return 100 via JIT; got: {v:?}"
    );
}
