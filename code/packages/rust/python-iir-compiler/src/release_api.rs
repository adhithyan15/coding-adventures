//! # The LANG84 release path: `py-int-print`/`py-float-print` (or `py2-int-print`) for one exact CPython release
//!
//! ```text
//!   source ──► resolve_release ──► GrammarLexer(python<release>.tokens)
//!          ──► GrammarParser(python<release>.grammar) ──► GrammarASTNode
//!          ──► int_lower.rs (walks the tree in place; reads a ReleaseProfile)
//!          ──► IIRModule of LANG83 registry ops ──► any of the nine backends
//! ```
//!
//! The first three stages are generic and table-driven, in `python-parser`'s
//! release API. This module only resolves the release's profile, bounds a
//! caller-supplied tree, runs the walker and validates the result.
//!
//! LANG79's float pilot ([`crate::compile_source`]) is a separate path over
//! the legacy minor-line grammar `"3.12"`; nothing here calls it, and it
//! does not call this. `tests/iir_path.rs` keeps the two apart.

use std::sync::{Arc, Mutex};

use coding_adventures_python_parser::{parse_python_at, resolve_release, FrontendBudgets};
use interpreter_ir::{IIRFunction, IIRModule, Operand};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use vm_core::{errors::VMError, value::Value, VMCore};

use crate::float_repr::{self, MAX_STEPS_PER_FLOAT_PRINT};
use crate::frontend_error::FrontendError;
use crate::int_lower::{shown_rule, Lowering, MAX_IIR_INSTRUCTIONS};
use crate::profiles::{profile_for, ReleaseProfile};

/// The integer subset every Python 3 release lowers, the default release
/// (`DEFAULT_RELEASE`) included, as `code/grammars/python/subsets.csv` names
/// it. Each release's own subsets are its profile's
/// (`profiles::profile_for(release).subsets`); [`SUBSETS`] lists them all.
pub const SUBSET: &str = "py-int-print";

/// Every subset the release path lowers (LANG84 phase 7c slice 1 added
/// `py2-int-print`, slice 3 `py-float-print`).
pub const SUBSETS: &[&str] = &["py-int-print", "py2-int-print", "py-float-print"];

// Budgets for a caller-supplied tree (LANG84 C5). A tree that came from
// source is already bounded by the parser's token and depth budgets; one
// handed in directly is not, so `compile_cst` measures it first.
const MAX_AST_ITEMS: usize = 16_384;
const MAX_AST_DEPTH: usize = 256;
const MAX_AST_TEXT_BYTES: usize = 1024 * 1024;
const MAX_AST_FIELD_BYTES: usize = 64 * 1024;
/// Bytes of output the VM runner keeps before refusing to write more.
const MAX_OUTPUT_BYTES: usize = 1_000_000;

/// The fewest instructions `main` spends on one float `print` of its own:
/// the instruction that computes the value (at least a `const`), the `call`
/// of `__py_write_float`, and the newline's `const` and `putchar`.
const MIN_MAIN_INSTRUCTIONS_PER_FLOAT_PRINT: u64 = 4;

/// The VM step budget [`run_module`] gives a program.
///
/// `main` is straight-line (the subsets have no loops and no user
/// functions), so it executes each of its at most [`MAX_IIR_INSTRUCTIONS`]
/// instructions once. Until LANG83 PR7c-py that was the whole budget,
/// because a float print was one `call_builtin write_f64_py` step. Now each
/// float print also runs the module's own `__py_write_float`, up to
/// [`MAX_STEPS_PER_FLOAT_PRINT`] steps, and a print costs `main` at least
/// [`MIN_MAIN_INSTRUCTIONS_PER_FLOAT_PRINT`] instructions, so
///
/// ```text
///   steps ≤ instructions + prints × 1,024
///         ≤ MAX_IIR_INSTRUCTIONS × (1 + 1,024 / 4)       = 257,000,000
/// ```
///
/// covers the largest module the instruction limit admits, whatever its
/// prints. The flat 1,000,000 refused valid programs: the 2,048-line,
/// 64 KiB `print(-1.2345678901234567e-308)` program, which the default
/// frontend budgets accept, needs about 1.4 million steps. The budget still
/// bounds a hand-built module given to [`run_module`].
const MAX_VM_STEPS: u64 = MAX_IIR_INSTRUCTIONS as u64
    * (1 + MAX_STEPS_PER_FLOAT_PRINT / MIN_MAIN_INSTRUCTIONS_PER_FLOAT_PRINT);

/// Lex, parse and compile `source` against exactly one CPython release.
///
/// `release` is `None` for `DEFAULT_RELEASE` (`3.13.16`) or an exact
/// inventory ID such as `Some("3.0a1")`. An unknown ID, a row with no grammar
/// pair, or a row with no IIR profile is a typed error: nothing falls back to
/// a neighbouring release or to a legacy minor-line grammar (LANG84 C4).
pub fn compile_source_at(
    source: &str,
    release: Option<&str>,
    module_name: &str,
    budgets: &FrontendBudgets,
) -> Result<IIRModule, FrontendError> {
    let (release, profile) = release_and_profile(release)?;
    check_module_name(release, module_name)?;
    let parsed = parse_python_at(source, Some(release), budgets)?;
    finish(release, profile, module_name, &parsed.ast)
}

/// Compile a parse tree the caller already has, as if `release` produced it.
///
/// The tree is measured against the caller-supplied-tree budgets (items,
/// depth, text) before anything walks it, because unlike a tree from
/// [`compile_source_at`] nothing upstream bounded it.
pub fn compile_cst(
    ast: &GrammarASTNode,
    release: Option<&str>,
    module_name: &str,
) -> Result<IIRModule, FrontendError> {
    let (release, profile) = release_and_profile(release)?;
    check_module_name(release, module_name)?;
    check_ast_budget(release, ast)?;
    finish(release, profile, module_name, ast)
}

fn release_and_profile(release: Option<&str>) -> Result<(&'static str, &'static ReleaseProfile), FrontendError> {
    let release = resolve_release(release)?;
    let profile = profile_for(release).ok_or_else(|| FrontendError::ReleaseNotAvailable {
        release: release.to_string(),
        stage: format!("no IIR profile for any of {} (iir_status is -)", SUBSETS.join(", ")),
    })?;
    Ok((release, profile))
}

fn check_module_name(release: &str, module_name: &str) -> Result<(), FrontendError> {
    if module_name.len() > MAX_AST_FIELD_BYTES {
        return Err(FrontendError::Budget {
            release: release.to_string(),
            which: "module name bytes",
            limit: MAX_AST_FIELD_BYTES,
        });
    }
    Ok(())
}

/// Lower the tree into `main` and check the result is valid IIR.
fn finish(
    release: &'static str,
    profile: &ReleaseProfile,
    module_name: &str,
    ast: &GrammarASTNode,
) -> Result<IIRModule, FrontendError> {
    let instructions = Lowering::new(release, profile).lower_file(ast)?;
    let mut module = IIRModule::new(module_name, "python");
    module.entry_point = Some("main".into());
    // LANG84 "Selection API": the module names the one release whose grammar
    // produced it, so a backend, a cache key or a test can tell a 3.0a1
    // module from a 3.13.16 one even when their instructions are identical.
    module.release = Some(release.to_string());
    // LANG83 PR7c-py: `repr(float)` is this compiler's own code. The helpers
    // go only into a module whose `main` holds a float print, so every
    // `py-int-print` and `py2-int-print` module is exactly `main`, as before.
    // (A print after a certain trap is still lowered, dead, so a module that
    // traps first still carries them.)
    let prints_a_float = instructions.iter().any(|instruction| {
        instruction.op == "call"
            && instruction.srcs.first().and_then(Operand::as_var) == Some(float_repr::WRITE_FLOAT)
    });
    module.add_or_replace(IIRFunction::new("main", vec![], "i64", instructions));
    if prints_a_float {
        for helper in float_repr::float_repr_helpers() {
            module.add_or_replace(helper);
        }
    }
    let errors = module.validate();
    if !errors.is_empty() {
        // Unreachable for trees the walker accepted; reported, not panicked.
        return Err(FrontendError::Unsupported {
            release: release.to_string(),
            rule: shown_rule(&ast.rule_name),
            start: None,
            end: None,
            message: format!("the lowered module is not valid IIR: {}", errors.join("; ")),
        });
    }
    Ok(module)
}

/// Walk a caller-supplied tree once, iteratively, and refuse it if it is too
/// large, too deep or carries too much text. The item count is checked
/// before a node's children are queued, so a hostile tree costs at most the
/// budget's worth of work.
fn check_ast_budget(release: &str, root: &GrammarASTNode) -> Result<(), FrontendError> {
    let budget = |which: &'static str, limit: usize| FrontendError::Budget {
        release: release.to_string(),
        which,
        limit,
    };
    let mut pending = vec![(root, 1_usize)];
    let mut visited = 1_usize;
    let mut text_bytes = 0_usize;
    while let Some((node, depth)) = pending.pop() {
        count_text(&node.rule_name, &mut text_bytes).map_err(|(w, l)| budget(w, l))?;
        for child in &node.children {
            if visited == MAX_AST_ITEMS {
                return Err(budget("CST items", MAX_AST_ITEMS));
            }
            visited += 1;
            match child {
                ASTNodeOrToken::Node(inner) => {
                    if depth == MAX_AST_DEPTH {
                        return Err(budget("CST depth", MAX_AST_DEPTH));
                    }
                    pending.push((inner, depth + 1));
                }
                ASTNodeOrToken::Token(token) => {
                    count_text(&token.value, &mut text_bytes).map_err(|(w, l)| budget(w, l))?;
                    if let Some(name) = &token.type_name {
                        count_text(name, &mut text_bytes).map_err(|(w, l)| budget(w, l))?;
                    }
                    if let Some(cv) = &token.cv {
                        count_text(cv, &mut text_bytes).map_err(|(w, l)| budget(w, l))?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn count_text(field: &str, total: &mut usize) -> Result<(), (&'static str, usize)> {
    if field.len() > MAX_AST_FIELD_BYTES {
        return Err(("CST field bytes", MAX_AST_FIELD_BYTES));
    }
    match total.checked_add(field.len()) {
        Some(next) if next <= MAX_AST_TEXT_BYTES => {
            *total = next;
            Ok(())
        }
        _ => Err(("CST text bytes", MAX_AST_TEXT_BYTES)),
    }
}

// ---------------------------------------------------------------------------
// Running a module on the repository's own VM
// ---------------------------------------------------------------------------

/// What a program did on `vm-core`: its output, and the LANG83 trap kind if
/// it stopped on one (after that output, as CPython prints before raising).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutput {
    /// Everything `print` wrote, in order.
    pub stdout: String,
    /// `Some("Trap.DivZero")` or `Some("Trap.Overflow")` when it trapped.
    pub trap: Option<String>,
}

/// Why a run produced no [`RunOutput`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// The source did not compile.
    Frontend(FrontendError),
    /// The VM stopped for a reason other than a LANG83 trap (a budget).
    Vm(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Frontend(error) => write!(f, "{error}"),
            RunError::Vm(message) => write!(f, "VM error: {message}"),
        }
    }
}

impl std::error::Error for RunError {}

/// Compile `source` for `release` and run it on `vm-core`.
pub fn run_source_at(source: &str, release: Option<&str>) -> Result<RunOutput, RunError> {
    let module = compile_source_at(source, release, "python-script", &FrontendBudgets::default())
        .map_err(RunError::Frontend)?;
    run_module(module)
}

/// Run a compiled module on `vm-core`, capturing `print_i64` and `putchar`
/// into one ordered, bounded buffer (LANG83 "ordered output stream").
///
/// | builtin     | writes                                                    |
/// | ----------- | --------------------------------------------------------- |
/// | `print_i64` | an `int` and its newline                                  |
/// | `putchar`   | one byte: a newline, or a character of a float's `repr`   |
///
/// Since LANG83 PR7c-py the module writes every character of a float itself
/// (`__py_write_float`), so the runner registers no float writer.
pub fn run_module(mut module: IIRModule) -> Result<RunOutput, RunError> {
    let output = Arc::new(Mutex::new(Vec::<u8>::new()));
    let mut vm = VMCore::new();
    vm.max_instructions = Some(MAX_VM_STEPS);
    let sink = Arc::clone(&output);
    vm.builtins_mut().register("print_i64", move |args| {
        let [Value::Int(number)] = args else {
            return Err(VMError::Custom("print_i64 expects one integer".into()));
        };
        append(&sink, format!("{number}\n").as_bytes())
    });
    let sink = Arc::clone(&output);
    vm.builtins_mut().register("putchar", move |args| {
        let [Value::Int(byte)] = args else {
            return Err(VMError::Custom("putchar expects one integer".into()));
        };
        append(&sink, &[(*byte & 0xFF) as u8])
    });
    let outcome = vm.execute(&mut module, "main", &[]);
    let stdout = String::from_utf8_lossy(
        &output
            .lock()
            .map_err(|_| RunError::Vm("Python output lock poisoned".into()))?,
    )
    .into_owned();
    match outcome {
        Ok(_) => Ok(RunOutput { stdout, trap: None }),
        Err(VMError::Trap { kind }) => Ok(RunOutput {
            stdout,
            trap: Some(kind),
        }),
        Err(error) => Err(RunError::Vm(error.to_string())),
    }
}

/// Append to the bounded output buffer, all or nothing.
fn append(sink: &Mutex<Vec<u8>>, bytes: &[u8]) -> Result<Value, VMError> {
    let mut buffer = sink
        .lock()
        .map_err(|_| VMError::Custom("Python output lock poisoned".into()))?;
    if buffer
        .len()
        .checked_add(bytes.len())
        .is_none_or(|size| size > MAX_OUTPUT_BYTES)
    {
        return Err(VMError::Custom("Python output limit exceeded".into()));
    }
    buffer.extend_from_slice(bytes);
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    //! LANG83 PR7c-py: the module's own `repr(float)` against vm-core's
    //! PR6a formatter (the text `write_f64_py` wrote), what one print costs
    //! on the VM, and the runner's step budget against the largest program
    //! the limits accept.

    use super::*;
    use coding_adventures_python_parser::{MAX_TRUSTED_SOURCE_BYTES, MAX_TRUSTED_TOKENS};

    /// A Python 3 spelling of `x` that reads back as exactly `x`: Rust's
    /// shortest round-trip `{:e}` form (`1.5e0`, `5e-324`), a `-` (that is,
    /// `f64_neg`) for a negative value, and arithmetic for the specials
    /// (`py-float-print` refuses a literal that overflows to `inf`).
    fn spelling(x: f64) -> String {
        let magnitude = if x.is_nan() {
            "(1e308 * 10 - 1e308 * 10)".to_string()
        } else if x.is_infinite() {
            "(1e308 * 10)".to_string()
        } else {
            format!("{:e}", x.abs())
        };
        if x.is_sign_negative() {
            format!("-{magnitude}")
        } else {
            magnitude
        }
    }

    /// Every layout boundary of `repr`, the specials, and a deterministic
    /// sweep of bit patterns, each with both signs: 1,248 values.
    fn sweep() -> Vec<f64> {
        let mut values: Vec<f64> = vec![
            0.0, f64::INFINITY, f64::NAN, 1.0, 1.5, 2.5, 0.1, 0.5, 0.3, 123.456, 4.35, 100.0, 1e15, 1e16,
            9.999999999999998e15, 1.0000000000000002e16, 1234567890123456.8, 1e-4, 1e-5, 9.999e-5,
            1.2345e-4, 0.00012345678901234567, 1.5e-7, 1e22, 1e21, 1e100, 1e-100, 1e-9, 1e-10, 1e9, 1e10,
            1.7976931348623157e308, 5e-324, 1e-323, 2.2250738585072014e-308, 1.2345678901234567e-308,
            4503599627370496.0, 9007199254740992.0, 9007199254740993.0, 12345678901234567.0,
            0.30000000000000004, 1.0 / 3.0, 2.0 / 3.0, 1e-300, 1e300, 0.14285714285714285,
        ];
        // xorshift64 over raw bit patterns; non-finite and zero patterns
        // are skipped (the specials above cover them).
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        while values.len() < 624 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let x = f64::from_bits(state);
            if x.is_finite() && x != 0.0 {
                values.push(x.abs());
            }
        }
        values.iter().flat_map(|&x| [x, -x]).collect()
    }

    /// Steps one module takes on vm-core, its output discarded.
    fn steps(mut module: IIRModule) -> u64 {
        let mut vm = VMCore::new();
        vm.builtins_mut().register("putchar", |_| Ok(Value::Null));
        vm.builtins_mut().register("print_i64", |_| Ok(Value::Null));
        vm.execute(&mut module, "main", &[]).unwrap();
        vm.metrics_instrs()
    }

    fn compile(source: &str) -> IIRModule {
        compile_source_at(source, None, "t", &FrontendBudgets::default()).unwrap()
    }

    /// `print(x)` writes `repr(x)` byte for byte as vm-core's PR6a
    /// `python_repr` does, over the sweep. The programs are batched (one
    /// print per line, each line checked) to keep the run short. Each batch
    /// is also a valid Python 3 program, so where a host `python3` is
    /// installed it runs the same source and must print the same bytes.
    #[test]
    fn write_float_matches_the_pr6a_python_repr_byte_for_byte() {
        let host = std::process::Command::new("python3")
            .args(["-I", "-c", "pass"])
            .output()
            .is_ok_and(|out| out.status.success());
        if !host {
            eprintln!("SKIP host python3 half: no `python3` on PATH");
        }
        let values = sweep();
        assert_eq!(values.len(), 1_248);
        for batch in values.chunks(208) {
            let source: String = batch.iter().map(|&x| format!("print({})\n", spelling(x))).collect();
            let want: String = batch
                .iter()
                .map(|&x| format!("{}\n", vm_core::float_format::python_repr(x)))
                .collect();
            let ran = run_source_at(&source, None).unwrap();
            assert_eq!(ran.trap, None);
            for ((got, want), x) in ran.stdout.lines().zip(want.lines()).zip(batch) {
                assert_eq!(got, want, "print({})", spelling(*x));
            }
            assert_eq!(ran.stdout, want);
            if host {
                let out = std::process::Command::new("python3").args(["-I", "-c", &source]).output().unwrap();
                assert!(out.status.success());
                assert_eq!(String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"), want);
            }
        }
    }

    /// No float print costs more than [`MAX_STEPS_PER_FLOAT_PRINT`] steps,
    /// the whole `print(x)` program included, so the budget derived from it
    /// holds. The dearest are measured, not guessed.
    #[test]
    fn one_float_print_stays_within_its_step_bound() {
        let mut dearest = (0u64, 0.0f64);
        for x in sweep() {
            let cost = steps(compile(&format!("print({})\n", spelling(x))));
            assert!(cost <= MAX_STEPS_PER_FLOAT_PRINT, "print({}) took {cost} steps", spelling(x));
            if cost > dearest.0 {
                dearest = (cost, x);
            }
        }
        eprintln!("dearest float print: {} steps for {}", dearest.0, spelling(dearest.1));
        // The bound has headroom, but not so much that it means nothing.
        assert!(dearest.0 > MAX_STEPS_PER_FLOAT_PRINT / 2, "{dearest:?}");
        for source in ["print(1.5)\n", "print(-1.2345678901234567e-308)\n", "print(1e15)\n", "print(-1.2345678901234567e15)\n", "print(0.0)\n", "print(1e308 * 10 - 1e308 * 10)\n", "print(-0.00012345678901234567)\n"] {
            eprintln!("{source:?}: {} steps", steps(compile(source)));
        }
    }

    /// The step budget covers the largest program the limits accept with
    /// the dearest float print. A flat budget of `MAX_IIR_INSTRUCTIONS`
    /// (what the runner had while a print was one builtin step) refuses it.
    #[test]
    fn the_step_budget_covers_the_largest_float_printing_program() {
        let line = "print(-1.2345678901234567e-308)\n";
        let lines = |count: usize| line.repeat(count);
        // run_source_at's own limits: the default frontend budgets. Find the
        // largest repetition the compiler still accepts.
        let (mut fits, mut over) = (1usize, 2usize);
        while compile_source_at(&lines(over), None, "t", &FrontendBudgets::default()).is_ok() {
            fits = over;
            over *= 2;
        }
        while over - fits > 1 {
            let middle = fits + (over - fits) / 2;
            if compile_source_at(&lines(middle), None, "t", &FrontendBudgets::default()).is_ok() {
                fits = middle;
            } else {
                over = middle;
            }
        }
        let ran = run_source_at(&lines(fits), None).unwrap_or_else(|e| panic!("{fits} lines: {e}"));
        assert_eq!(ran.stdout, "-1.2345678901234567e-308\n".repeat(fits));
        let module = compile(&lines(fits));
        let needed = steps(module);
        assert!(needed > MAX_IIR_INSTRUCTIONS as u64, "{fits} lines took only {needed} steps");
        assert!(needed <= MAX_VM_STEPS);

        // The trusted maximum a caller may ask for: 1 MiB of source and
        // 256 Ki tokens, run through `run_module`. Here the binding limit is
        // vm-core's frame, which holds at most 65,536 registers (more
        // are dropped, and the program fails on an "undefined variable"
        // whatever its prints); each such line uses three, as it did before
        // PR7c-py, because every print's unused result shares `py_repr`.
        let trusted = FrontendBudgets {
            max_source_bytes: MAX_TRUSTED_SOURCE_BYTES,
            max_tokens: MAX_TRUSTED_TOKENS,
            ..FrontendBudgets::default()
        };
        let registers = |module: &IIRModule| {
            let main = module.get_function("main").unwrap();
            main.instructions.iter().filter_map(|i| i.dest.as_deref()).collect::<std::collections::HashSet<_>>().len()
        };
        let per_line = registers(&compile(&lines(2))) - registers(&compile(&lines(1)));
        assert_eq!(per_line, 3);
        let count = (vm_core::frame::VMFrame::MAX_FRAME_REGISTERS - 2) / per_line;
        let module = compile_source_at(&lines(count), None, "t", &trusted).unwrap();
        assert!(registers(&module) <= vm_core::frame::VMFrame::MAX_FRAME_REGISTERS);
        let ran = run_module(module).unwrap_or_else(|e| panic!("{count} lines: {e}"));
        assert_eq!(ran.stdout.lines().count(), count);
        eprintln!("default budgets: {fits} lines, {needed} steps; trusted: {count} lines");
    }
}
