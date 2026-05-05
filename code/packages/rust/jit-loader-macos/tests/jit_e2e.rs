//! End-to-end JIT integration tests.
//!
//! These exercise the full in-process JIT pipeline:
//!
//! ```text
//! IIRModule
//!     │  aot-core::specialise → typed CIR
//!     │  aarch64-backend::compile_function → ARM64 bytes
//!     ▼
//! CodePage::new(bytes)        ← jit-loader-macos installs the bytes
//!     │
//!     ▼
//! transmute → extern "C" fn   ← cast to a typed function pointer
//!     │
//!     ▼
//! Rust calls the JIT'd function in-process
//! ```
//!
//! Equivalent to what `twig-vm`'s JIT path does at runtime when a hot
//! function is detected — but driven from a test harness so we can
//! assert the result without spinning up the full VM.

#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use aarch64_backend::AArch64Backend;
use aot_core::infer::infer_types;
use aot_core::specialise::aot_specialise;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use jit_core::backend::{Backend, FunctionContext};
use jit_loader_macos::CodePage;

/// Compile a single IIRFunction through the AOT-style pipeline and
/// install the result via the JIT loader.  Returns a `CodePage` that
/// the caller can transmute to the appropriate `extern "C"` fn type.
fn jit_function(f: &IIRFunction) -> CodePage {
    let inferred = infer_types(f);
    let cir = aot_specialise(f, Some(&inferred));
    let ctx = FunctionContext {
        name: &f.name, params: &f.params, return_type: &f.return_type,
    };
    let bytes = AArch64Backend.compile_function(&ctx, &cir)
        .expect("backend produced bytes");
    CodePage::new(&bytes).expect("JIT install")
}

#[test]
fn jit_returns_const() {
    // fn() -> u64 { 42 }
    let f = IIRFunction::new(
        "k", vec![], "u64",
        vec![
            IIRInstr::new("const", Some("v0".into()),
                          vec![Operand::Int(42)], "u64"),
            IIRInstr::new("ret", None,
                          vec![Operand::Var("v0".into())], "u64"),
        ],
    );
    let page = jit_function(&f);
    let func: extern "C" fn() -> u64 = unsafe { page.as_function() };
    assert_eq!(func(), 42);
}

#[test]
fn jit_adds_two_args() {
    // fn(a: u64, b: u64) -> u64 { a + b }
    let f = IIRFunction::new(
        "add", vec![("a".into(), "u64".into()), ("b".into(), "u64".into())], "u64",
        vec![
            // The IR-compiler emits `+` as call_builtin; aot-core lowers
            // it to add_<ty> when both args are typed.
            IIRInstr::new("call_builtin", Some("v0".into()),
                          vec![
                              Operand::Var("+".into()),
                              Operand::Var("a".into()),
                              Operand::Var("b".into()),
                          ], "any"),
            IIRInstr::new("ret", None,
                          vec![Operand::Var("v0".into())], "u64"),
        ],
    );
    let page = jit_function(&f);
    let func: extern "C" fn(u64, u64) -> u64 = unsafe { page.as_function() };
    assert_eq!(func(7, 35), 42);
    assert_eq!(func(100, 200), 300);
    assert_eq!(func(1, 1), 2);
}

#[test]
fn jit_compares() {
    // fn(a: u64, b: u64) -> u64 { if a < b { 1 } else { 0 } }
    let f = IIRFunction::new(
        "lt", vec![("a".into(), "u64".into()), ("b".into(), "u64".into())], "u64",
        vec![
            IIRInstr::new("call_builtin", Some("v0".into()),
                          vec![
                              Operand::Var("<".into()),
                              Operand::Var("a".into()),
                              Operand::Var("b".into()),
                          ], "any"),
            IIRInstr::new("ret", None,
                          vec![Operand::Var("v0".into())], "u64"),
        ],
    );
    let page = jit_function(&f);
    let func: extern "C" fn(u64, u64) -> u64 = unsafe { page.as_function() };
    assert_eq!(func(1, 5), 1);
    assert_eq!(func(5, 5), 0);
    assert_eq!(func(9, 5), 0);
}

#[test]
fn jit_nib_source_through_loader() {
    // Compile a Nib source program all the way to executable code.
    let src = "fn add3(a: u4, b: u4) -> u4 { return a + b; }";
    let module = nib_iir_compiler::compile_source(src, "jit_demo")
        .expect("nib → IIR");
    // Find the function and JIT it.
    let f = module.functions.iter().find(|f| f.name == "add3")
        .expect("add3 in module");
    let page = jit_function(f);
    let func: extern "C" fn(u64, u64) -> u64 = unsafe { page.as_function() };
    assert_eq!(func(3, 4), 7);
    assert_eq!(func(1, 9), 10);
}

#[test]
fn many_jit_functions_coexist() {
    // Install several distinct functions; each must continue working
    // even after the others have been installed.
    let f1 = IIRFunction::new(
        "k1", vec![], "u64",
        vec![
            IIRInstr::new("const", Some("v0".into()),
                          vec![Operand::Int(1)], "u64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u64"),
        ],
    );
    let f2 = IIRFunction::new(
        "k2", vec![], "u64",
        vec![
            IIRInstr::new("const", Some("v0".into()),
                          vec![Operand::Int(2)], "u64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u64"),
        ],
    );
    let p1 = jit_function(&f1);
    let p2 = jit_function(&f2);
    let g1: extern "C" fn() -> u64 = unsafe { p1.as_function() };
    let g2: extern "C" fn() -> u64 = unsafe { p2.as_function() };
    assert_eq!(g1(), 1);
    assert_eq!(g2(), 2);
    // Re-call after both installed:
    assert_eq!(g1(), 1);
    assert_eq!(g2(), 2);
}

/// Module-level reference: load multiple functions from one IIRModule.
#[test]
fn module_with_multiple_functions() {
    let _m = IIRModule::new("multi", "demo");
    // We don't actually use the module here — just verify the type
    // imports compose.  Real per-fn JIT iterates module.functions.
}
