//! A lowered function must fit one vm-core frame (issue #18419).
//!
//! vm-core refuses, before it runs anything, a function whose frame would
//! need more than `VMFrame::MAX_FRAME_REGISTERS` (2¹⁶ = 65,536) registers,
//! counted as `params + distinct instruction dests that are not params`.
//! The default budgets (64 KiB, 16 Ki tokens) cannot reach that many; the
//! trusted maximum (1 MiB, 256 Ki tokens) can. Before #18419 such a program
//! compiled and then failed at VM load. The compiler now refuses it itself,
//! as a `Budget` naming `main` and the statement that went over.
//!
//! The tests sit on the boundary, on the VM column: 65,536 registers compile
//! **and run on vm-core**; one more is refused by the compiler.
//!
//! ## A program of exactly N registers
//!
//! `main` has no parameters, so its registers are its distinct dests. Every
//! literal and operator result is a fresh `rbN`, and `main` ends with one
//! more for its `ret 0`:
//!
//! ```text
//!   puts(1+1+…+1)   with n ones   →  n consts + (n − 1) adds = 2n − 1
//!   puts(1)                       →  1
//!   end of main (const 0; ret)    →  1
//! ```
//!
//! 257 lines of 128 ones are 257 × 255 = 65,535 registers, and with the exit
//! register 65,536 exactly. [`registers`] counts the compiled module as
//! vm-core does, so this arithmetic is checked, not trusted.

use std::collections::HashSet;

use coding_adventures_ruby_parser::{MAX_TRUSTED_SOURCE_BYTES, MAX_TRUSTED_TOKENS};
use interpreter_ir::{IIRFunction, IIRModule};
use ruby_iir_compiler::{
    compile_source_at, run_module, FrontendBudgets, FrontendError, DEFAULT_RELEASE,
    MAX_REGISTERS_PER_FUNCTION,
};
use vm_core::frame::VMFrame;

/// The ceiling under test, taken from vm-core rather than restated.
const LIMIT: usize = VMFrame::MAX_FRAME_REGISTERS;

fn trusted() -> FrontendBudgets {
    FrontendBudgets {
        max_source_bytes: MAX_TRUSTED_SOURCE_BYTES,
        max_tokens: MAX_TRUSTED_TOKENS,
        ..FrontendBudgets::default()
    }
}

fn compile(source: &str) -> Result<IIRModule, FrontendError> {
    compile_source_at(source, None, "t", &trusted())
}

/// A function's registers as vm-core's `VMFrame::check_register_limit`
/// counts them: one per parameter by position, plus one per distinct dest
/// that does not name a parameter.
fn registers(function: &IIRFunction) -> usize {
    let params: HashSet<&str> = function.params.iter().map(|(name, _)| name.as_str()).collect();
    let locals: HashSet<&str> = function
        .instructions
        .iter()
        .filter_map(|instruction| instruction.dest.as_deref())
        .filter(|name| !params.contains(name))
        .collect();
    function.params.len() + locals.len()
}

/// `puts(1+1+…+1)` with `ones` ones: `2 × ones − 1` registers.
fn sum_line(ones: usize) -> String {
    format!("puts({})\n", vec!["1"; ones].join("+"))
}

/// 257 lines that, with `main`'s exit register, need exactly 65,536.
fn at_the_limit() -> String {
    assert_eq!(257 * (2 * 128 - 1) + 1, LIMIT, "the construction in the module docs");
    sum_line(128).repeat(257)
}

#[test]
fn the_compiler_ceiling_is_vm_cores_frame_limit() {
    assert_eq!(MAX_REGISTERS_PER_FUNCTION, VMFrame::MAX_FRAME_REGISTERS);
    assert_eq!(MAX_REGISTERS_PER_FUNCTION, 65_536);
}

#[test]
fn a_main_of_exactly_the_frame_limit_compiles_and_runs_on_the_vm() {
    let source = at_the_limit();
    assert!(matches!(
        compile_source_at(&source, None, "t", &FrontendBudgets::default()),
        Err(FrontendError::Budget { .. })
    ));
    let module = compile(&source).unwrap_or_else(|e| panic!("65,536 registers must compile: {e}"));
    let main = module.get_function("main").unwrap();
    assert_eq!(registers(main), LIMIT);
    assert!(VMFrame::check_register_limit(main).is_ok());
    let ran = run_module(module).unwrap_or_else(|e| panic!("65,536 registers must run: {e}"));
    assert_eq!(ran.trap, None);
    assert_eq!(ran.stdout, "128\n".repeat(257));
}

#[test]
fn one_register_past_the_frame_limit_is_refused_by_the_compiler() {
    // Line 258 is the `puts(1)` that leaves no room for `main`'s exit; the
    // lines after it are never lowered.
    let source = format!("{}puts(1)\n{}", at_the_limit(), "puts(1)\n".repeat(3));
    let error = compile(&source).expect_err("65,537 registers must not compile");
    let FrontendError::Budget { release, which, limit, function, start } = &error else {
        panic!("expected a Budget error, got {error:?}");
    };
    assert_eq!(release, DEFAULT_RELEASE);
    assert_eq!(*which, "registers per function");
    assert_eq!(*limit, LIMIT);
    assert_eq!(function.as_deref(), Some("main"));
    assert_eq!(*start, Some((258, 1)));
    let text = error.to_string();
    assert!(text.contains("`main`") && text.contains("258:1") && text.contains("65536"), "{text}");
}

/// Several statements on one line: the position is the statement's own.
#[test]
fn the_error_points_at_the_statement_that_went_over() {
    // 256 × 255 + 253 = 65,533; `puts(2)` makes 65,534, `puts(1)` 65,535,
    // and the exit 65,536: it fits. One more `puts(3)` does not.
    let source = format!("{}puts(2); puts(1)\n", sum_line(128).repeat(256) + &sum_line(127));
    let module = compile(&source).unwrap();
    assert_eq!(registers(module.get_function("main").unwrap()), LIMIT);
    let over = source.trim_end().to_string() + "; puts(3)\n";
    let error = compile(&over).unwrap_err();
    assert!(
        matches!(&error, FrontendError::Budget { start: Some((258, 19)), function: Some(f), .. } if f == "main"),
        "{error:?}"
    );
}
