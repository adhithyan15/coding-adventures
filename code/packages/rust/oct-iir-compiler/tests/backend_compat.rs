//! Oct → IIR-to-* backend acceptance tests.
//!
//! Proves Oct's emitted IIR is accepted by every AOT backend
//! (wasm / jvm / clr / beam).  Without these tests we could regress
//! Oct's IIR shape (e.g. emit an op no backend knows) without anyone
//! noticing until users try to AOT-compile.
//!
//! Pattern mirrors `twig-ir-compiler/tests/backend_compat.rs`.

use oct_iir_compiler::compile_source;

/// Helper: run every backend's validator on a module.  Panics on any
/// rejection, attributing the failure to the named backend.
fn assert_accepted_by_every_backend(m: &interpreter_ir::IIRModule, label: &str) {
    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(m)),
        ("jvm",  iir_to_jvm_class_file::validate::validate_for_jvm(m)),
        ("clr",  iir_to_cil_bytecode::validate::validate_iir_for_clr(m)),
        ("beam", iir_to_beam::validate::validate_for_beam(m)),
    ] {
        assert!(errs.is_empty(),
            "[{name}] validator rejected Oct {label}; got {} error(s): {errs:?}",
            errs.len());
    }
}

// ---------------------------------------------------------------------------
// Group 1: minimal shapes
// ---------------------------------------------------------------------------

/// The smallest Oct program: empty `fn main()`.
#[test]
fn oct_empty_main_accepted_by_every_backend() {
    let m = compile_source("fn main() { }", "compat")
        .expect("Oct must compile");
    assert_accepted_by_every_backend(&m, "empty main");
}

/// `let` binding + ret — exercises `const`, `mov`, `ret_void` on main
/// plus a function that returns a value.
#[test]
fn oct_return_constant_accepted_by_every_backend() {
    let src = "fn answer() -> u8 { return 42; } fn main() { }";
    let m = compile_source(src, "compat").expect("Oct must compile");
    assert_accepted_by_every_backend(&m, "`fn answer() -> u8 { return 42; }`");
}

// ---------------------------------------------------------------------------
// Group 2: arithmetic
// ---------------------------------------------------------------------------

/// Binary `+` on u8 args → typed `add`.
#[test]
fn oct_typed_add_accepted_by_every_backend() {
    let src = "fn add_two() -> u8 { let x: u8 = 30; let y: u8 = 12; return x + y; } \
               fn main() { }";
    let m = compile_source(src, "compat").expect("Oct must compile");
    assert_accepted_by_every_backend(&m, "u8 `x + y`");
}

/// `-` operator (subtraction).
#[test]
fn oct_typed_sub_accepted_by_every_backend() {
    let src = "fn sub_two() -> u8 { let x: u8 = 50; let y: u8 = 8; return x - y; } \
               fn main() { }";
    let m = compile_source(src, "compat").expect("Oct must compile");
    assert_accepted_by_every_backend(&m, "u8 `x - y`");
}

// ---------------------------------------------------------------------------
// Group 3: comparisons
// ---------------------------------------------------------------------------

/// `==` comparison + return.
#[test]
fn oct_typed_eq_accepted_by_every_backend() {
    let src = "fn eq_check() -> bool { let x: u8 = 5; return x == 5; } \
               fn main() { }";
    let m = compile_source(src, "compat").expect("Oct must compile");
    assert_accepted_by_every_backend(&m, "u8 `x == 5`");
}

/// `<` (less than).
#[test]
fn oct_typed_lt_accepted_by_every_backend() {
    let src = "fn lt_check() -> bool { let x: u8 = 3; return x < 10; } \
               fn main() { }";
    let m = compile_source(src, "compat").expect("Oct must compile");
    assert_accepted_by_every_backend(&m, "u8 `x < 10`");
}

// ---------------------------------------------------------------------------
// Group 4: control flow
// ---------------------------------------------------------------------------

/// `if`/`else` — exercises `jmp_if_false`, `mov`, `jmp`, `label`.
#[test]
fn oct_if_else_accepted_by_every_backend() {
    let src = "fn pick() -> u8 { \
                   let x: u8 = 0; \
                   if x == 0 { x = 1; } else { x = 2; } \
                   return x; \
               } \
               fn main() { }";
    let m = compile_source(src, "compat").expect("Oct must compile");
    assert_accepted_by_every_backend(&m, "if/else `if x == 0 { x = 1 } else { x = 2 }`");
}

/// `while` loop — exercises backward jmp + cmp_lt loop header.
#[test]
fn oct_while_loop_accepted_by_every_backend() {
    let src = "fn count_to_ten() -> u8 { \
                   let n: u8 = 0; \
                   while n < 10 { n = n + 1; } \
                   return n; \
               } \
               fn main() { }";
    let m = compile_source(src, "compat").expect("Oct must compile");
    assert_accepted_by_every_backend(&m, "while `n < 10` loop");
}

// ---------------------------------------------------------------------------
// Group 5: function-status invariant
// ---------------------------------------------------------------------------

/// Every Oct function in the module must be `FullyTyped` post-OCT03 —
/// without that, JITCore's threshold-zero compile path doesn't fire
/// and the IIR-to-* backends emit untyped fallbacks (or reject).
#[test]
fn oct_every_function_is_fully_typed() {
    let src = "fn a() -> u8 { return 1; } \
               fn b() -> u8 { let x: u8 = a(); return x + 1; } \
               fn main() { }";
    let m = compile_source(src, "compat").expect("Oct must compile");
    for f in &m.functions {
        assert_eq!(
            f.type_status,
            interpreter_ir::function::FunctionTypeStatus::FullyTyped,
            "Oct fn {:?} should be FullyTyped; got: {:?}",
            f.name, f.type_status,
        );
    }
}
