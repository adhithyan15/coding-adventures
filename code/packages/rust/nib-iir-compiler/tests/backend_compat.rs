//! Cross-backend compatibility: confirm Nib's IIR output passes the
//! validators of every IIR-to-* backend in the workspace.
//!
//! Before the typed-CIR fix, Nib emitted `call_builtin "+"` with
//! `type_hint: "any"`, which every IIR-to-* validator rejected:
//!
//! ```text
//! [wasm] 3 errors:
//!   UntypedInstruction: function "main", op "call_builtin" has type_hint "any"
//!   UnsupportedOp: function "main", op "call_builtin" …
//!   UntypedInstruction: function "main", op "ret" has type_hint "any"
//! ```
//!
//! After the fix, Nib emits typed CIR ops directly (`add`, `cmp_eq`,
//! …) with `type_hint: "i64"`, matching the pattern oct-iir-compiler
//! uses, and the validators accept the module.

use nib_iir_compiler::compile_source;

#[test]
fn nib_iir_accepted_by_iir_to_wasm() {
    let m = compile_source("fn main() -> u8 { return 30 + 12; }", "compat")
        .expect("Nib must compile to IIR");
    let errs = iir_to_wasm::validate::validate_for_wasm(&m);
    assert!(
        errs.is_empty(),
        "wasm validator should accept Nib's typed IIR; got errors: {errs:?}"
    );
}

#[test]
fn nib_iir_accepted_by_iir_to_jvm() {
    let m = compile_source("fn main() -> u8 { return 30 + 12; }", "compat")
        .expect("Nib must compile to IIR");
    let errs = iir_to_jvm_class_file::validate::validate_for_jvm(&m);
    assert!(
        errs.is_empty(),
        "jvm validator should accept Nib's typed IIR; got errors: {errs:?}"
    );
}

#[test]
fn nib_iir_accepted_by_iir_to_clr() {
    let m = compile_source("fn main() -> u8 { return 30 + 12; }", "compat")
        .expect("Nib must compile to IIR");
    let errs = iir_to_cil_bytecode::validate::validate_iir_for_clr(&m);
    assert!(
        errs.is_empty(),
        "clr validator should accept Nib's typed IIR; got errors: {errs:?}"
    );
}

#[test]
fn nib_iir_accepted_by_iir_to_beam() {
    let m = compile_source("fn main() -> u8 { return 30 + 12; }", "compat")
        .expect("Nib must compile to IIR");
    let errs = iir_to_beam::validate::validate_for_beam(&m);
    assert!(
        errs.is_empty(),
        "beam validator should accept Nib's typed IIR; got errors: {errs:?}"
    );
}

/// Comparison operator: `cmp_lt` must be emitted (not `call_builtin "<"`)
/// so all four backends accept the result-of-a-comparison shape.
#[test]
fn nib_iir_with_comparison_accepted_by_every_backend() {
    let src = "fn main() -> u8 { \
                 if 5 < 10 { return 1; } else { return 0; } \
               }";
    let m = compile_source(src, "compat").expect("Nib must compile");

    // Must contain `cmp_lt`, never `call_builtin "<"`.
    let ops: Vec<&str> = m.functions[0]
        .instructions
        .iter()
        .map(|i| i.op.as_str())
        .collect();
    assert!(ops.contains(&"cmp_lt"), "expected `cmp_lt` op; got {ops:?}");

    for (name, errs) in [
        ("wasm", iir_to_wasm::validate::validate_for_wasm(&m)),
        ("jvm", iir_to_jvm_class_file::validate::validate_for_jvm(&m)),
        (
            "clr",
            iir_to_cil_bytecode::validate::validate_iir_for_clr(&m),
        ),
        ("beam", iir_to_beam::validate::validate_for_beam(&m)),
    ] {
        assert!(
            errs.is_empty(),
            "[{name}] expected no validator errors; got {} error(s): {errs:?}",
            errs.len()
        );
    }
}
