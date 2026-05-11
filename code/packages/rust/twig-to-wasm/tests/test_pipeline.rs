//! Integration tests for the `twig-to-wasm` end-to-end pipeline.
//!
//! These tests verify that:
//!
//! 1. Well-formed Twig programs that use only numeric operations compile to
//!    valid WASM binaries (non-empty bytes starting with `b"\x00asm"`).
//! 2. Ill-formed Twig programs (syntax errors, unbound names) produce
//!    `TwigToWasmError::CompileError`.
//! 3. Programs that use operations the WASM backend cannot handle (e.g.
//!    `make_nil` for empty programs, closures) produce
//!    `TwigToWasmError::WasmError`.
//! 4. The error types implement `Display` and `std::error::Error`.
//!
//! ## Why "only numeric programs compile to WASM"?
//!
//! Twig is a dynamically-typed Lisp.  At the IIR level every operation is a
//! `call_builtin` — `call_builtin "+", a, b` for addition, `call_builtin
//! "make_nil"` for the nil value, etc.  The `pre_lower_builtins` pass
//! converts the *arithmetic* builtins (`+`, `-`, `*`, `/`, `=`, `<`, `>`,
//! `<=`, `>=`, `not`, `_move`) to typed IIR ops (`add`, `sub`, …, `mov`)
//! that the WASM backend can handle.
//!
//! Builtins outside that table — `make_nil`, `cons`, `make_closure`,
//! `apply_closure`, `global_get` — remain as `call_builtin` instructions.
//! The WASM backend's validator rejects `call_builtin`, so programs that
//! emit those at the IIR level produce a `WasmError`.
//!
//! The practical consequence: Twig programs consisting entirely of
//! *numeric* top-level function definitions and call sites compile cleanly.
//! Programs that use runtime dispatch, closures, or the nil/cons heap all
//! fail with `WasmError`.

use twig_to_wasm::{compile_twig_to_wasm, TwigToWasmError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compile source and assert it succeeds, returning the WASM bytes.
fn compile_ok(source: &str) -> Vec<u8> {
    compile_twig_to_wasm(source, "test_module")
        .unwrap_or_else(|e| panic!("expected Ok but got Err: {e}\nsource: {source}"))
}

/// Compile source and assert it fails, returning the error.
fn compile_err(source: &str) -> TwigToWasmError {
    compile_twig_to_wasm(source, "test_module")
        .expect_err("expected Err but compilation succeeded")
}

// ---------------------------------------------------------------------------
// ── Group 1: Successful compilations ─────────────────────────────────────
//
// These programs consist entirely of arithmetic operations that the
// builtin-lowering pass can convert to typed IIR ops.
// ---------------------------------------------------------------------------

// Test 1.1 — the simplest possible program
#[test]
fn simple_addition_compiles() {
    let bytes = compile_ok("(+ 1 2)");
    assert!(!bytes.is_empty(), "should produce non-empty WASM binary");
    assert!(
        bytes.starts_with(b"\x00asm"),
        "WASM binary must start with \\x00asm magic; got {:?}",
        &bytes[..4.min(bytes.len())]
    );
}

// Test 1.2 — subtraction
#[test]
fn subtraction_compiles() {
    let bytes = compile_ok("(define (sub a b) (- a b)) (sub 10 3)");
    assert!(bytes.starts_with(b"\x00asm"));
    assert!(!bytes.is_empty());
}

// Test 1.3 — multiplication
#[test]
fn multiplication_compiles() {
    let bytes = compile_ok("(define (mul a b) (* a b)) (mul 4 5)");
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.4 — division
#[test]
fn division_compiles() {
    let bytes = compile_ok("(define (div a b) (/ a b)) (div 10 2)");
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.5 — equality comparison
#[test]
fn equality_comparison_compiles() {
    let bytes = compile_ok("(define (eq? a b) (= a b)) (eq? 5 5)");
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.6 — less-than comparison
#[test]
fn less_than_comparison_compiles() {
    let bytes = compile_ok("(define (lt? a b) (< a b)) (lt? 3 7)");
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.7 — greater-than comparison
#[test]
fn greater_than_comparison_compiles() {
    let bytes = compile_ok("(define (gt? a b) (> a b)) (gt? 10 5)");
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.8 — nested arithmetic
#[test]
fn nested_arithmetic_compiles() {
    // (a + b) * (c - d) — all numeric, all lowerable.
    let bytes = compile_ok(
        "(define (expr a b c d) (* (+ a b) (- c d))) (expr 2 3 10 4)"
    );
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.9 — factorial (recursive, numeric only)
#[test]
fn factorial_compiles() {
    let bytes = compile_ok(
        "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)"
    );
    assert!(bytes.starts_with(b"\x00asm"));
    // Should be a non-trivial binary with more than just the WASM header.
    assert!(bytes.len() > 8);
}

// Test 1.10 — Fibonacci (recursive, numeric only)
#[test]
fn fibonacci_compiles() {
    let bytes = compile_ok(
        "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (fib 10)"
    );
    assert!(bytes.starts_with(b"\x00asm"));
    assert!(bytes.len() > 8);
}

// Test 1.11 — multiple functions
#[test]
fn multiple_functions_compile() {
    let bytes = compile_ok(
        "(define (square x) (* x x))\
         (define (cube x) (* x (square x)))\
         (cube 3)"
    );
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.12 — if expression with comparison
#[test]
fn if_with_comparison_compiles() {
    let bytes = compile_ok(
        "(define (max2 a b) (if (> a b) a b)) (max2 7 4)"
    );
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.13 — deeply nested arithmetic expressions
#[test]
fn deeply_nested_arithmetic_compiles() {
    // ((a + b) * (c + d)) - ((e + f) * (g + h))
    let src = "(define (f a b c d e g h i) \
                 (- (* (+ a b) (+ c d)) (* (+ e g) (+ h i)))) \
               (f 1 2 3 4 5 6 7 8)";
    let bytes = compile_ok(src);
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.14 — mutual recursion (numeric only)
#[test]
fn mutually_recursive_functions_compile() {
    let bytes = compile_ok(
        "(define (my-even n) (if (= n 0) 1 (my-odd (- n 1))))\
         (define (my-odd n) (if (= n 0) 0 (my-even (- n 1))))\
         (my-even 4)"
    );
    assert!(bytes.starts_with(b"\x00asm"));
}

// Test 1.15 — output is substantially more than 8 bytes (WASM binary has
//             more content than just the magic + version header).
#[test]
fn output_is_non_trivial_wasm_binary() {
    // A non-trivial program should produce a binary with sections:
    // type, function, export, code, etc.
    let bytes = compile_ok("(define (add a b) (+ a b)) (add 10 20)");
    // Rough lower bound: magic(4) + version(4) + at least one section
    assert!(
        bytes.len() > 12,
        "expected a multi-section WASM binary; got {} bytes",
        bytes.len()
    );
}

// ---------------------------------------------------------------------------
// ── Group 2: Error cases — compile errors (stage 1) ─────────────────────
// ---------------------------------------------------------------------------

// Test 2.1 — syntax error
#[test]
fn syntax_error_produces_compile_error() {
    let err = compile_err("(bad syntax (((");
    assert!(
        matches!(err, TwigToWasmError::CompileError(_)),
        "syntax error should produce CompileError; got: {err}"
    );
}

// Test 2.2 — unbound name
#[test]
fn unbound_name_produces_compile_error() {
    // `undefined_name` is not a builtin or a defined function.
    let err = compile_err("undefined_name");
    assert!(
        matches!(err, TwigToWasmError::CompileError(_)),
        "unbound name should produce CompileError; got: {err}"
    );
}

// Test 2.3 — lambda capturing unbound name
#[test]
fn lambda_capturing_unbound_name_is_compile_error() {
    let err = compile_err("(define (f) (lambda (x) (+ x z)))");
    assert!(
        matches!(err, TwigToWasmError::CompileError(_)),
        "unbound capture should produce CompileError; got: {err}"
    );
}

// Test 2.4 — unbalanced parentheses (parse error)
#[test]
fn unbalanced_parens_produce_compile_error() {
    let err = compile_err("(define (f x) (* x x)");
    assert!(
        matches!(err, TwigToWasmError::CompileError(_)),
        "unbalanced parens should produce CompileError; got: {err}"
    );
}

// ---------------------------------------------------------------------------
// ── Group 3: Error cases — WASM backend errors (stage 4) ─────────────────
//
// Programs that compile to valid IIR but contain operations the WASM backend
// cannot handle.
// ---------------------------------------------------------------------------

// Test 3.1 — empty program emits make_nil which WASM cannot handle
#[test]
fn empty_program_is_wasm_error() {
    // An empty Twig program compiles to: `call_builtin make_nil` + `ret`.
    // `make_nil` survives builtin-lowering (not in the arithmetic table),
    // so the WASM validator sees a `call_builtin` instruction and rejects it.
    let err = compile_err("");
    assert!(
        matches!(err, TwigToWasmError::WasmError(_)),
        "empty program should produce WasmError (call_builtin make_nil); got: {err}"
    );
}

// Test 3.2 — nil literal produces call_builtin which WASM cannot lower
#[test]
fn nil_literal_is_wasm_error() {
    let err = compile_err("nil");
    assert!(
        matches!(err, TwigToWasmError::WasmError(_)),
        "nil literal should produce WasmError; got: {err}"
    );
}

// Test 3.3 — boolean constant: either compiles or is a WASM error, never panics
//
// In Twig, `#t` compiles to `const #t : bool`.  The WASM backend may or may
// not handle `bool` constants depending on implementation.  This test only
// verifies the pipeline is wired and doesn't crash.
#[test]
fn boolean_literal_does_not_panic() {
    let _ = compile_twig_to_wasm("#t", "test_module");
    // No assertion — just ensure no panic.
}

// ---------------------------------------------------------------------------
// ── Group 4: Error type properties ──────────────────────────────────────
// ---------------------------------------------------------------------------

// Test 4.1 — TwigToWasmError implements Display
#[test]
fn error_display_compile_error() {
    let err = compile_err("undefined_name");
    let s = format!("{err}");
    // The display must be non-empty and mention something useful.
    assert!(!s.is_empty(), "Display output should not be empty");
}

// Test 4.2 — TwigToWasmError implements std::error::Error
#[test]
fn error_implements_std_error() {
    let err = compile_err("undefined_name");
    let _: &dyn std::error::Error = &err;
}

// Test 4.3 — error source chain works
#[test]
fn error_source_chain() {
    use std::error::Error;
    let err = compile_err("undefined_name");
    // source() should return Some (the wrapped inner error).
    assert!(err.source().is_some(), "error chain should have a source");
}

// ---------------------------------------------------------------------------
// ── Group 5: WASM binary structure checks ───────────────────────────────
// ---------------------------------------------------------------------------

// Test 5.1 — magic bytes are correct
#[test]
fn wasm_magic_bytes_are_correct() {
    let bytes = compile_ok("(define (add a b) (+ a b)) (add 1 2)");
    assert_eq!(&bytes[..4], b"\x00asm", "first 4 bytes should be WASM magic");
}

// Test 5.2 — WASM version field is 1
#[test]
fn wasm_version_is_1() {
    // After the 4-byte magic, WASM 1.0 has version [0x01, 0x00, 0x00, 0x00].
    let bytes = compile_ok("(define (add a b) (+ a b)) (add 1 2)");
    assert_eq!(
        &bytes[4..8],
        &[0x01, 0x00, 0x00, 0x00],
        "bytes 4..8 should be WASM version 1"
    );
}

// Test 5.3 — factorial output is deterministic (same input → same output)
#[test]
fn compilation_is_deterministic() {
    let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)";
    let a = compile_ok(src);
    let b = compile_ok(src);
    assert_eq!(a, b, "compilation should be deterministic");
}

// Test 5.4 — different programs produce different binaries
#[test]
fn different_programs_produce_different_binaries() {
    let add = compile_ok("(define (f a b) (+ a b)) (f 1 2)");
    let sub = compile_ok("(define (f a b) (- a b)) (f 1 2)");
    // It is very unlikely (effectively impossible) that add and sub produce
    // identical binaries.
    assert_ne!(add, sub, "different programs should produce different binaries");
}

// Test 5.5 — exported function name is embedded in the binary
//
// WASM 1.0 does not embed the module name in the binary itself (the module
// name is metadata for the embedding host, not stored in the `.wasm` bytes).
// However, the WASM export section does embed each function's name as a
// length-prefixed UTF-8 string.  We verify that the function name "add"
// appears somewhere in the binary.
#[test]
fn function_name_embedded_in_binary() {
    // The IIR lowering pass exports every function by its IIR name, so "add"
    // must appear in the WASM export section as raw bytes.
    let bytes = compile_twig_to_wasm(
        "(define (add a b) (+ a b)) (add 1 2)",
        "myapp",
    ).unwrap();
    let name_bytes = b"add";
    let found = bytes.windows(name_bytes.len()).any(|w| w == name_bytes);
    assert!(found, "function name 'add' should appear in the WASM export section");
}
