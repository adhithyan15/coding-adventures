//! A lowered function must fit one vm-core frame (issue #18419).
//!
//! vm-core refuses, before it runs anything, a function whose frame would
//! need more than `VMFrame::MAX_FRAME_REGISTERS` (2¹⁶ = 65,536) registers
//! (`VMError::RegisterLimitExceeded`). The count it checks is
//!
//! ```text
//!   registers = params  +  distinct instruction dests that are not params
//! ```
//!
//! The default frontend budgets (64 KiB, 16 Ki tokens) cannot reach that
//! many, but the trusted maximum a caller may ask for (1 MiB, 256 Ki tokens)
//! can. Before #18419 such a program compiled and then failed at VM load,
//! far from its source. The compiler now refuses it itself, as a `Budget`
//! error naming the function and the statement that went over.
//!
//! The tests sit exactly on the boundary, on the VM column: a `main` of
//! 65,536 registers compiles **and runs on vm-core**; one more register is
//! refused by the compiler.
//!
//! ## How a program of exactly N registers is built
//!
//! `main` has no parameters, so its registers are its distinct dests. In the
//! integer subset every literal and every operator result is a fresh `pyN`
//! temporary, and `main` ends with one more for its `return 0`:
//!
//! ```text
//!   print(1+1+…+1)   with n ones   →  n consts + (n − 1) adds = 2n − 1
//!   print(1)                       →  1
//!   end of main (const 0; ret)     →  1
//! ```
//!
//! 257 lines of 128 ones are 257 × 255 = 65,535 registers; with the exit
//! register that is 65,536 exactly. A trailing `print(1)` makes 65,537.
//! [`registers`] counts the compiled module the way vm-core does, so the
//! arithmetic above is checked, not trusted.

use std::collections::HashSet;

use coding_adventures_python_parser::{MAX_TRUSTED_SOURCE_BYTES, MAX_TRUSTED_TOKENS};
use interpreter_ir::{IIRFunction, IIRModule};
use python_iir_compiler::{
    compile_source_at, run_module, FrontendBudgets, FrontendError, MAX_REGISTERS_PER_FUNCTION,
};
use vm_core::frame::VMFrame;

/// The ceiling under test, taken from vm-core rather than restated.
const LIMIT: usize = VMFrame::MAX_FRAME_REGISTERS;

/// The trusted maximum a caller may ask for.
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

/// A function's registers, counted as vm-core's
/// `VMFrame::check_register_limit` counts them: one per parameter by
/// position, plus one per distinct dest that does not name a parameter.
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

/// `print(1+1+…+1)` with `ones` ones: `2 × ones − 1` registers.
fn sum_line(ones: usize) -> String {
    format!("print({})\n", vec!["1"; ones].join("+"))
}

/// 257 lines that, with `main`'s exit register, need exactly 65,536.
fn at_the_limit() -> String {
    assert_eq!(257 * (2 * 128 - 1) + 1, LIMIT, "the construction in the module docs");
    sum_line(128).repeat(257)
}

/// The compiler's ceiling is vm-core's constant, not a copy that could drift.
#[test]
fn the_compiler_ceiling_is_vm_cores_frame_limit() {
    assert_eq!(MAX_REGISTERS_PER_FUNCTION, VMFrame::MAX_FRAME_REGISTERS);
    assert_eq!(MAX_REGISTERS_PER_FUNCTION, 65_536);
}

#[test]
fn a_main_of_exactly_the_frame_limit_compiles_and_runs_on_the_vm() {
    let source = at_the_limit();
    // The default budgets cannot reach the limit at all; only trusted ones can.
    assert!(matches!(
        compile_source_at(&source, None, "t", &FrontendBudgets::default()),
        Err(FrontendError::Budget { .. })
    ));
    let module = compile(&source).unwrap_or_else(|e| panic!("65,536 registers must compile: {e}"));
    let main = module.get_function("main").unwrap();
    assert_eq!(registers(main), LIMIT);
    // vm-core's own load-time check agrees, and the VM runs the whole thing.
    assert!(VMFrame::check_register_limit(main).is_ok());
    let ran = run_module(module).unwrap_or_else(|e| panic!("65,536 registers must run: {e}"));
    assert_eq!(ran.trap, None);
    assert_eq!(ran.stdout, "128\n".repeat(257));
}

#[test]
fn one_register_past_the_frame_limit_is_refused_by_the_compiler() {
    // Line 258 is the `print(1)` that leaves no room for `main`'s exit.
    let source = format!("{}print(1)\n", at_the_limit());
    let error = compile(&source).expect_err("65,537 registers must not compile");
    let FrontendError::Budget { release, which, limit, function, start } = &error else {
        panic!("expected a Budget error, got {error:?}");
    };
    assert_eq!(release, "3.13.16");
    assert_eq!(*which, "registers per function");
    assert_eq!(*limit, LIMIT);
    assert_eq!(function.as_deref(), Some("main"));
    assert_eq!(*start, Some((258, 1)));
    let text = error.to_string();
    assert!(text.contains("`main`") && text.contains("258:1") && text.contains("65536"), "{text}");
}

/// The same boundary where the register that does not fit belongs to a
/// statement in the middle of the file, not to the last one: the error
/// points at it, and nothing after it is lowered.
#[test]
fn the_error_points_at_the_statement_that_went_over() {
    // Lines 1–257 use 65,535 registers, all that the statements may have
    // (the last one is `main`'s exit). Line 258 is the first statement with
    // no room, so the error names it, and lines 259–261 are never lowered.
    let source = format!("{}print(1)\n{}", at_the_limit(), "print(1)\n".repeat(3));
    let error = compile(&source).unwrap_err();
    assert!(
        matches!(&error, FrontendError::Budget { start: Some((258, 1)), function: Some(f), .. } if f == "main"),
        "{error:?}"
    );
    // Several statements on one line: the position is the statement's own.
    let source = format!("{}print(2); print(1)\n", sum_line(128).repeat(256) + &sum_line(127));
    // 256 × 255 + 253 = 65,533; `print(2)` makes 65,534, `print(1)` 65,535,
    // and the exit 65,536: it fits. One more `print(3)` does not.
    assert!(compile(&source).is_ok());
    let over = source.trim_end().to_string() + "; print(3)\n";
    let error = compile(&over).unwrap_err();
    assert!(matches!(&error, FrontendError::Budget { start: Some((258, 21)), .. }), "{error:?}");
}

/// The register limit is per function, and the float helpers the compiler
/// adds are functions of their own: each is far below it, counted as
/// vm-core counts (its parameters included).
#[test]
fn every_function_of_a_float_module_fits_a_frame() {
    let module = compile("print(1.5)\nprint(-0.0)\n").unwrap();
    assert!(module.functions.len() > 1, "the float helpers are present");
    for function in &module.functions {
        assert!(VMFrame::check_register_limit(function).is_ok(), "{}", function.name);
        assert!(registers(function) <= LIMIT, "{}", function.name);
    }
}
