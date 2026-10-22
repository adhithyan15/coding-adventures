//! # The LANG84 release path: `py-int-print` for one exact CPython release
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
use interpreter_ir::{IIRFunction, IIRModule};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use vm_core::{errors::VMError, value::Value, VMCore};

use crate::frontend_error::FrontendError;
use crate::int_lower::{shown_rule, Lowering, MAX_IIR_INSTRUCTIONS};
use crate::profiles::{profile_for, ReleaseProfile};

/// The subset the release path lowers, as `code/grammars/python/subsets.csv`
/// names it.
pub const SUBSET: &str = "py-int-print";

// Budgets for a caller-supplied tree (LANG84 C5). A tree that came from
// source is already bounded by the parser's token and depth budgets; one
// handed in directly is not, so `compile_cst` measures it first.
const MAX_AST_ITEMS: usize = 16_384;
const MAX_AST_DEPTH: usize = 256;
const MAX_AST_TEXT_BYTES: usize = 1024 * 1024;
const MAX_AST_FIELD_BYTES: usize = 64 * 1024;
/// Bytes of output the VM runner keeps before refusing to write more.
const MAX_OUTPUT_BYTES: usize = 1_000_000;

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
        stage: format!("no IIR profile for {SUBSET} (iir_status is -)"),
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
    module.add_or_replace(IIRFunction::new("main", vec![], "i64", instructions));
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
pub fn run_module(mut module: IIRModule) -> Result<RunOutput, RunError> {
    let output = Arc::new(Mutex::new(Vec::<u8>::new()));
    let mut vm = VMCore::new();
    // The program is straight-line, so it executes at most as many
    // instructions as it holds.
    vm.max_instructions = Some(MAX_IIR_INSTRUCTIONS as u64);
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
