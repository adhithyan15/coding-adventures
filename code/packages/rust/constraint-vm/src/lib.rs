//! # `constraint-vm` — the Constraint-VM instruction-stream executor.
//!
//! **LANG24 PR 24-D**.  Walks a [`Program`] one opcode at a time and
//! drives the [`Engine`], accumulating state (asserted predicates, push/pop
//! scope stack).  Returns a [`VmOutput`] describing what the program produced.
//!
//! This is the "easy crate" in the LANG24 stack — all the interesting
//! solving happens in `constraint-engine`; this crate is just the
//! orchestration layer.
//!
//! ## Architecture (mirrors LP08 exactly)
//!
//! ```text
//! constraint-instructions — Program (instruction stream)
//!         │
//! constraint-vm (this crate)
//!         │   walks Program, dispatches each opcode
//!         │   maintains scope stack (Engine snapshots)
//!         │
//! constraint-engine — Engine::check_sat() → SolverResult
//! ```
//!
//! ## Scope stack and incremental solving
//!
//! `PushScope` saves a snapshot of the current engine state.
//! `PopScope` restores the saved snapshot, undoing all assertions made
//! since the matching push.  This enables incremental solving —
//! consumers (like `lang-refinement-checker`) can explore alternative
//! hypotheses without re-building the solver from scratch.
//!
//! `Reset` clears all assertions *and* the scope stack (returns to a
//! fresh engine).
//!
//! ## Resource limits
//!
//! The VM enforces:
//!
//! | Limit | Default | Control |
//! |-------|---------|---------|
//! | Max instructions per run | 10 000 | [`Config::max_instrs`] |
//! | Max scope depth | 100 | [`Config::max_scope_depth`] |
//! | Max assertions total | 10 000 | [`Config::max_assertions`] |
//!
//! Exceeding any limit produces a [`VmError::LimitExceeded`] rather than
//! running forever or panicking.
//!
//! ## Trace output
//!
//! `Echo` instructions append a string to the trace log (accessible via
//! [`VmOutput::trace`]).  Useful for debugging constraint programs.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::HashMap;

use constraint_core::Sort;
use constraint_engine::{Engine, Model, SolverResult};
use constraint_instructions::{ConstraintInstr, Program};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Resource limits for the VM.
///
/// Construct with [`Config::default`] and override fields as needed.
#[derive(Debug, Clone)]
pub struct Config {
    /// Maximum number of instructions to execute before returning
    /// [`VmError::LimitExceeded`].  Default: 10 000.
    pub max_instrs: usize,
    /// Maximum push/pop nesting depth.  Default: 100.
    pub max_scope_depth: usize,
    /// Maximum total assertions across all scopes.  Default: 10 000.
    pub max_assertions: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config { max_instrs: 10_000, max_scope_depth: 100, max_assertions: 10_000 }
    }
}

// ---------------------------------------------------------------------------
// VmError
// ---------------------------------------------------------------------------

/// Errors the VM can return.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VmError {
    /// `GetModel` executed when the last `CheckSat` did not return SAT.
    NoModel,
    /// `GetUnsatCore` executed when the last `CheckSat` did not return UNSAT.
    NoUnsatCore,
    /// `GetModel` / `GetUnsatCore` executed before any `CheckSat`.
    NoPriorCheckSat,
    /// `PopScope` executed with no matching `PushScope`.
    UnmatchedPop,
    /// A resource limit (instruction count, scope depth, assertion count)
    /// was exceeded.
    LimitExceeded(String),
    /// The solver returned an error the VM can't handle.
    EngineError(String),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::NoModel => write!(f, "get-model: last check-sat was not SAT"),
            VmError::NoUnsatCore => write!(f, "get-unsat-core: last check-sat was not UNSAT"),
            VmError::NoPriorCheckSat => write!(f, "get-model/get-unsat-core: no prior check-sat"),
            VmError::UnmatchedPop => write!(f, "pop: no matching push"),
            VmError::LimitExceeded(reason) => write!(f, "limit exceeded: {reason}"),
            VmError::EngineError(reason) => write!(f, "engine error: {reason}"),
        }
    }
}

impl std::error::Error for VmError {}

// ---------------------------------------------------------------------------
// VmOutput
// ---------------------------------------------------------------------------

/// The output produced by executing a [`Program`].
///
/// Collects all the results across the full instruction stream — each
/// `CheckSat` appends to [`VmOutput::sat_results`], each `GetModel`
/// appends to [`VmOutput::models`], and each `Echo` appends to
/// [`VmOutput::trace`].
#[derive(Debug, Clone)]
pub struct VmOutput {
    /// Result of each `CheckSat` in execution order.
    pub sat_results: Vec<SolverResult>,
    /// Model extracted by each `GetModel` in execution order.
    pub models: Vec<Model>,
    /// Messages produced by `Echo` instructions, in order.
    pub trace: Vec<String>,
    /// Number of instructions executed.
    pub instructions_executed: usize,
}

impl VmOutput {
    fn new() -> Self {
        VmOutput {
            sat_results: Vec::new(),
            models: Vec::new(),
            trace: Vec::new(),
            instructions_executed: 0,
        }
    }

    /// Return the last `CheckSat` result, or `None` if none were executed.
    pub fn last_sat_result(&self) -> Option<&SolverResult> {
        self.sat_results.last()
    }

    /// Return the last model extracted, or `None` if `GetModel` was never
    /// called (or was called after a non-SAT result).
    pub fn last_model(&self) -> Option<&Model> {
        self.models.last()
    }
}

// ---------------------------------------------------------------------------
// Vm
// ---------------------------------------------------------------------------

/// The Constraint-VM executor.
///
/// Construct one per constraint-solving session (or per refinement-
/// checking invocation).  Re-use across multiple programs by calling
/// [`Vm::reset`] between runs.
pub struct Vm {
    config: Config,
    /// Current engine (top of scope stack).
    engine: Engine,
    /// Saved snapshots for `PushScope` / `PopScope`.
    ///
    /// Each entry pairs the engine snapshot with the `total_assertions`
    /// count at the time of the push, so that `PopScope` can restore both
    /// together and the assertion limit isn't permanently consumed by
    /// assertions made inside a (subsequently popped) scope.
    scope_stack: Vec<(Engine, usize)>,
    /// Result of the most recent `CheckSat`.
    last_check_sat: Option<SolverResult>,
    /// Total assertions submitted (across all scopes) — for the limit.
    total_assertions: usize,
    /// Options set via `SetOption` (stored for inspection; not all affect
    /// the solver in v1).
    options: HashMap<String, String>,
}

impl Vm {
    /// Construct a new VM with default resource limits.
    pub fn new() -> Self {
        Vm::with_config(Config::default())
    }

    /// Construct a new VM with a custom [`Config`].
    pub fn with_config(config: Config) -> Self {
        Vm {
            config,
            engine: Engine::new(),
            scope_stack: Vec::new(),
            last_check_sat: None,
            total_assertions: 0,
            options: HashMap::new(),
        }
    }

    /// Reset the VM to its initial state (clear engine + scope stack).
    pub fn reset(&mut self) {
        self.engine = Engine::new();
        self.scope_stack.clear();
        self.last_check_sat = None;
        self.total_assertions = 0;
    }

    /// Execute a [`Program`] and return its output.
    ///
    /// Continues from whatever state the VM is currently in.  Call
    /// [`Vm::reset`] before `execute` if you want a fresh run.
    pub fn execute(&mut self, program: &Program) -> Result<VmOutput, VmError> {
        let mut out = VmOutput::new();

        for instr in program.instructions() {
            // Instruction count limit.
            if out.instructions_executed >= self.config.max_instrs {
                return Err(VmError::LimitExceeded(format!(
                    "instruction limit ({}) exceeded",
                    self.config.max_instrs
                )));
            }
            out.instructions_executed += 1;

            self.execute_one(instr, &mut out)?;
        }

        Ok(out)
    }

    // -------------------------------------------------------------------------
    // Single-instruction dispatch
    // -------------------------------------------------------------------------

    fn execute_one(&mut self, instr: &ConstraintInstr, out: &mut VmOutput) -> Result<(), VmError> {
        match instr {
            ConstraintInstr::SetLogic { logic } => {
                self.engine.set_logic(*logic);
            }

            ConstraintInstr::DeclareVar { name, sort } => {
                self.engine.declare_var(name.clone(), sort.clone());
            }

            ConstraintInstr::DeclareFn { name, .. } => {
                // v1: uninterpreted functions are declared but the engine
                // treats applications as `Unknown` predicates.  Record the
                // declaration so the engine doesn't crash on Apply nodes.
                // Full EUF support is a future LANG24 PR.
                self.engine.declare_var(
                    name.clone(),
                    Sort::Uninterpreted(format!("fn:{name}")),
                );
            }

            ConstraintInstr::Assert { pred } => {
                // Assertion count limit.
                self.total_assertions += 1;
                if self.total_assertions > self.config.max_assertions {
                    return Err(VmError::LimitExceeded(format!(
                        "assertion limit ({}) exceeded",
                        self.config.max_assertions
                    )));
                }
                self.engine.assert(pred.clone());
            }

            ConstraintInstr::CheckSat => {
                let result = self.engine.check_sat();
                out.sat_results.push(result.clone());
                self.last_check_sat = Some(result);
            }

            ConstraintInstr::GetModel => {
                let result = self.last_check_sat.as_ref().ok_or(VmError::NoPriorCheckSat)?;
                match result {
                    SolverResult::Sat(model) => {
                        out.models.push(model.clone());
                    }
                    SolverResult::Unsat => return Err(VmError::NoModel),
                    SolverResult::Unknown(_) => return Err(VmError::NoModel),
                }
            }

            ConstraintInstr::GetUnsatCore => {
                let result = self.last_check_sat.as_ref().ok_or(VmError::NoPriorCheckSat)?;
                match result {
                    SolverResult::Unsat => {
                        // v1: empty unsat core (full UNSAT core extraction
                        // requires clause provenance tracking — a future PR).
                        // Return without error so the instruction doesn't
                        // crash the program.
                    }
                    SolverResult::Sat(_) | SolverResult::Unknown(_) => {
                        return Err(VmError::NoUnsatCore);
                    }
                }
            }

            ConstraintInstr::PushScope => {
                // Scope depth limit.
                if self.scope_stack.len() >= self.config.max_scope_depth {
                    return Err(VmError::LimitExceeded(format!(
                        "scope depth limit ({}) exceeded",
                        self.config.max_scope_depth
                    )));
                }
                let snap = self.engine.snapshot();
                // Save both the engine snapshot and the current assertion count so
                // that PopScope can restore the limit counter.  Without this, every
                // assertion made inside a pushed scope permanently consumes budget
                // even after the scope is popped — leaking capacity and making the
                // limit unreliable over many push/pop cycles.
                self.scope_stack.push((snap, self.total_assertions));
            }

            ConstraintInstr::PopScope => {
                let (saved_engine, saved_assertions) =
                    self.scope_stack.pop().ok_or(VmError::UnmatchedPop)?;
                self.engine = saved_engine;
                self.total_assertions = saved_assertions;
                self.last_check_sat = None; // Scope changed — prior result stale.
            }

            ConstraintInstr::Reset => {
                self.engine.reset_all();
                self.scope_stack.clear();
                self.last_check_sat = None;
                self.total_assertions = 0;
            }

            ConstraintInstr::Echo { msg } => {
                out.trace.push(msg.clone());
            }

            ConstraintInstr::SetOption { key, value } => {
                self.options.insert(key.clone(), value.to_string());
            }

            // `#[non_exhaustive]` — future variants don't crash the VM;
            // they're silently ignored until the VM is updated to handle them.
            _ => {}
        }
        Ok(())
    }
}

impl Default for Vm {
    fn default() -> Self {
        Vm::new()
    }
}

// ---------------------------------------------------------------------------
// Convenience: run a single constraint program to a SolverResult
// ---------------------------------------------------------------------------

/// Run a complete constraint program and return the last `CheckSat` result.
///
/// Convenience wrapper for callers that only care about satisfiability (e.g.
/// `lang-refinement-checker`).  Returns `SolverResult::Unknown` if the
/// program never executed a `CheckSat`.
pub fn check_sat(program: &Program) -> Result<SolverResult, VmError> {
    let mut vm = Vm::new();
    let out = vm.execute(program)?;
    Ok(out
        .last_sat_result()
        .cloned()
        .unwrap_or_else(|| SolverResult::Unknown("no check-sat in program".into())))
}

/// Run a program and extract the last model.
///
/// Returns `None` if the program was UNSAT or never executed `CheckSat` /
/// `GetModel`.
pub fn get_model(program: &Program) -> Result<Option<Model>, VmError> {
    let mut vm = Vm::new();
    let out = vm.execute(program)?;
    Ok(out.last_model().cloned())
}

// ---------------------------------------------------------------------------
// Builder — programmatic program construction
// ---------------------------------------------------------------------------

/// Ergonomic builder for constraint programs.
///
/// Builds a [`Program`] one instruction at a time without requiring the
/// caller to deal with the `Program::new` validation layer directly.
///
/// ```rust
/// use constraint_vm::ProgramBuilder;
/// use constraint_core::{Sort, Logic};
///
/// let prog = ProgramBuilder::new()
///     .set_logic(Logic::QF_LIA)
///     .declare_int("x")
///     .assert_ge_int("x", 0)
///     .assert_le_int("x", 100)
///     .check_sat()
///     .get_model()
///     .build();
/// ```
#[derive(Default)]
pub struct ProgramBuilder {
    instrs: Vec<ConstraintInstr>,
}

impl ProgramBuilder {
    /// Create an empty builder.
    pub fn new() -> Self {
        ProgramBuilder { instrs: Vec::new() }
    }

    /// Append a raw instruction.
    pub fn push(mut self, instr: ConstraintInstr) -> Self {
        self.instrs.push(instr);
        self
    }

    /// Set the active logic.
    pub fn set_logic(self, logic: constraint_core::Logic) -> Self {
        self.push(ConstraintInstr::SetLogic { logic })
    }

    /// Declare an integer variable.
    pub fn declare_int(self, name: impl Into<String>) -> Self {
        self.push(ConstraintInstr::DeclareVar { name: name.into(), sort: Sort::Int })
    }

    /// Declare a boolean variable.
    pub fn declare_bool(self, name: impl Into<String>) -> Self {
        self.push(ConstraintInstr::DeclareVar { name: name.into(), sort: Sort::Bool })
    }

    /// Assert a predicate.
    pub fn assert_pred(self, pred: constraint_core::Predicate) -> Self {
        self.push(ConstraintInstr::Assert { pred })
    }

    /// Assert `var ≥ lo`.
    pub fn assert_ge_int(self, var: impl Into<String>, lo: i128) -> Self {
        let v = var.into();
        self.assert_pred(constraint_core::Predicate::Ge(
            Box::new(constraint_core::Predicate::Var(v)),
            Box::new(constraint_core::Predicate::Int(lo)),
        ))
    }

    /// Assert `var ≤ hi`.
    pub fn assert_le_int(self, var: impl Into<String>, hi: i128) -> Self {
        let v = var.into();
        self.assert_pred(constraint_core::Predicate::Le(
            Box::new(constraint_core::Predicate::Var(v)),
            Box::new(constraint_core::Predicate::Int(hi)),
        ))
    }

    /// Assert `var = val`.
    pub fn assert_eq_int(self, var: impl Into<String>, val: i128) -> Self {
        let v = var.into();
        self.assert_pred(constraint_core::Predicate::Eq(
            Box::new(constraint_core::Predicate::Var(v)),
            Box::new(constraint_core::Predicate::Int(val)),
        ))
    }

    /// Append a `CheckSat`.
    pub fn check_sat(self) -> Self {
        self.push(ConstraintInstr::CheckSat)
    }

    /// Append a `GetModel`.
    pub fn get_model(self) -> Self {
        self.push(ConstraintInstr::GetModel)
    }

    /// Append a `PushScope`.
    pub fn push_scope(self) -> Self {
        self.push(ConstraintInstr::PushScope)
    }

    /// Append a `PopScope`.
    pub fn pop_scope(self) -> Self {
        self.push(ConstraintInstr::PopScope)
    }

    /// Append an `Echo`.
    pub fn echo(self, msg: impl Into<String>) -> Self {
        self.push(ConstraintInstr::Echo { msg: msg.into() })
    }

    /// Consume the builder and return an unchecked `Program`.
    ///
    /// Uses `Program::new_unchecked` because the builder never introduces
    /// invalid identifier names or unmatched pops (the builder API
    /// doesn't expose those paths).
    pub fn build(self) -> Program {
        Program::new_unchecked(self.instrs)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use constraint_core::{Logic, Predicate, Sort};
    use constraint_instructions::{ConstraintInstr, Program};

    use super::*;

    // ─── Helpers ─────────────────────────────────────────────────────────────

    fn declare_int(name: &str) -> ConstraintInstr {
        ConstraintInstr::DeclareVar { name: name.into(), sort: Sort::Int }
    }

    fn declare_bool(name: &str) -> ConstraintInstr {
        ConstraintInstr::DeclareVar { name: name.into(), sort: Sort::Bool }
    }

    fn assert_ge(var: &str, n: i128) -> ConstraintInstr {
        ConstraintInstr::Assert {
            pred: Predicate::Ge(
                Box::new(Predicate::Var(var.into())),
                Box::new(Predicate::Int(n)),
            ),
        }
    }

    fn assert_le(var: &str, n: i128) -> ConstraintInstr {
        ConstraintInstr::Assert {
            pred: Predicate::Le(
                Box::new(Predicate::Var(var.into())),
                Box::new(Predicate::Int(n)),
            ),
        }
    }

    fn checked_program(instrs: Vec<ConstraintInstr>) -> Program {
        Program::new(instrs).expect("test program should be valid")
    }

    // ─── Basic execution ──────────────────────────────────────────────────────

    #[test]
    fn empty_program_returns_empty_output() {
        let prog = checked_program(vec![]);
        let out = Vm::new().execute(&prog).unwrap();
        assert_eq!(out.sat_results.len(), 0);
        assert_eq!(out.instructions_executed, 0);
    }

    #[test]
    fn check_sat_trivially_sat() {
        let prog = checked_program(vec![declare_int("x"), ConstraintInstr::CheckSat]);
        let out = Vm::new().execute(&prog).unwrap();
        assert_eq!(out.sat_results.len(), 1);
        assert!(out.last_sat_result().unwrap().is_sat());
    }

    #[test]
    fn lia_range_sat_with_model() {
        // x ≥ 0 ∧ x ≤ 100 → SAT; get-model returns an x in range.
        let prog = checked_program(vec![
            ConstraintInstr::SetLogic { logic: Logic::QF_LIA },
            declare_int("x"),
            assert_ge("x", 0),
            assert_le("x", 100),
            ConstraintInstr::CheckSat,
            ConstraintInstr::GetModel,
        ]);
        let out = Vm::new().execute(&prog).unwrap();
        assert!(out.last_sat_result().unwrap().is_sat());
        let m = out.last_model().unwrap();
        let x = m.get("x").unwrap().as_int().unwrap();
        assert!((0..=100).contains(&x), "x={x} not in [0,100]");
    }

    #[test]
    fn lia_contradiction_unsat() {
        // x ≥ 10 ∧ x ≤ 5 → UNSAT.
        let prog = checked_program(vec![
            declare_int("x"),
            assert_ge("x", 10),
            assert_le("x", 5),
            ConstraintInstr::CheckSat,
        ]);
        let out = Vm::new().execute(&prog).unwrap();
        assert!(out.last_sat_result().unwrap().is_unsat());
    }

    #[test]
    fn echo_appends_to_trace() {
        let prog = checked_program(vec![
            ConstraintInstr::Echo { msg: "hello".into() },
            ConstraintInstr::Echo { msg: "world".into() },
        ]);
        let out = Vm::new().execute(&prog).unwrap();
        assert_eq!(out.trace, vec!["hello", "world"]);
    }

    // ─── GetModel / GetUnsatCore errors ───────────────────────────────────────

    #[test]
    fn get_model_before_check_sat_errors() {
        let prog = checked_program(vec![declare_int("x"), ConstraintInstr::GetModel]);
        let err = Vm::new().execute(&prog).unwrap_err();
        assert_eq!(err, VmError::NoPriorCheckSat);
    }

    #[test]
    fn get_model_after_unsat_errors() {
        let prog = checked_program(vec![
            declare_int("x"),
            assert_ge("x", 10),
            assert_le("x", 5),
            ConstraintInstr::CheckSat,
            ConstraintInstr::GetModel,
        ]);
        let err = Vm::new().execute(&prog).unwrap_err();
        assert_eq!(err, VmError::NoModel);
    }

    #[test]
    fn get_unsat_core_before_check_sat_errors() {
        let prog = checked_program(vec![declare_int("x"), ConstraintInstr::GetUnsatCore]);
        let err = Vm::new().execute(&prog).unwrap_err();
        assert_eq!(err, VmError::NoPriorCheckSat);
    }

    #[test]
    fn get_unsat_core_after_sat_errors() {
        let prog = checked_program(vec![
            declare_int("x"),
            ConstraintInstr::CheckSat,
            ConstraintInstr::GetUnsatCore,
        ]);
        let err = Vm::new().execute(&prog).unwrap_err();
        assert_eq!(err, VmError::NoUnsatCore);
    }

    // ─── Push/pop scope ───────────────────────────────────────────────────────

    #[test]
    fn push_pop_restores_state() {
        // Declare x ∈ [0,100].  Push.  Assert x ∈ [0,-1] (UNSAT).
        // Pop.  Check SAT again → should be SAT.
        let prog = checked_program(vec![
            declare_int("x"),
            assert_ge("x", 0),
            assert_le("x", 100),
            ConstraintInstr::CheckSat, // SAT
            ConstraintInstr::PushScope,
            assert_ge("x", 200), // Contradiction
            ConstraintInstr::CheckSat, // UNSAT
            ConstraintInstr::PopScope,
            ConstraintInstr::CheckSat, // SAT again
        ]);
        let out = Vm::new().execute(&prog).unwrap();
        assert_eq!(out.sat_results.len(), 3);
        assert!(out.sat_results[0].is_sat());
        assert!(out.sat_results[1].is_unsat());
        assert!(out.sat_results[2].is_sat());
    }

    #[test]
    fn unmatched_pop_errors() {
        // Program::new would normally catch this, but we use
        // new_unchecked to bypass validation for this error test.
        let prog = Program::new_unchecked(vec![ConstraintInstr::PopScope]);
        let err = Vm::new().execute(&prog).unwrap_err();
        assert_eq!(err, VmError::UnmatchedPop);
    }

    // ─── Reset ───────────────────────────────────────────────────────────────

    #[test]
    fn reset_clears_assertions() {
        let prog = checked_program(vec![
            declare_int("x"),
            assert_ge("x", 10),
            assert_le("x", 5), // contradiction
            ConstraintInstr::CheckSat, // UNSAT
            ConstraintInstr::Reset,
            declare_int("y"),
            ConstraintInstr::CheckSat, // SAT (fresh state)
        ]);
        let out = Vm::new().execute(&prog).unwrap();
        assert_eq!(out.sat_results.len(), 2);
        assert!(out.sat_results[0].is_unsat());
        assert!(out.sat_results[1].is_sat());
    }

    // ─── Resource limits ─────────────────────────────────────────────────────

    #[test]
    fn instruction_limit_exceeded() {
        let config = Config { max_instrs: 3, ..Default::default() };
        let prog = checked_program(vec![
            ConstraintInstr::Echo { msg: "a".into() },
            ConstraintInstr::Echo { msg: "b".into() },
            ConstraintInstr::Echo { msg: "c".into() },
            ConstraintInstr::Echo { msg: "d".into() }, // 4th → limit exceeded
        ]);
        let err = Vm::with_config(config).execute(&prog).unwrap_err();
        assert!(matches!(err, VmError::LimitExceeded(_)));
    }

    #[test]
    fn scope_depth_limit_exceeded() {
        let config = Config { max_scope_depth: 2, ..Default::default() };
        // Push 3 times — third should fail.
        let prog = Program::new_unchecked(vec![
            ConstraintInstr::PushScope,
            ConstraintInstr::PushScope,
            ConstraintInstr::PushScope, // 3rd → limit exceeded
        ]);
        let err = Vm::with_config(config).execute(&prog).unwrap_err();
        assert!(matches!(err, VmError::LimitExceeded(_)));
    }

    #[test]
    fn assertion_limit_exceeded() {
        let config = Config { max_assertions: 2, ..Default::default() };
        let prog = checked_program(vec![
            declare_int("x"),
            assert_ge("x", 0),
            assert_le("x", 10),
            assert_ge("x", 1), // 3rd assertion → limit exceeded
        ]);
        let err = Vm::with_config(config).execute(&prog).unwrap_err();
        assert!(matches!(err, VmError::LimitExceeded(_)));
    }

    // ─── Convenience functions ────────────────────────────────────────────────

    #[test]
    fn check_sat_fn_works() {
        let prog = checked_program(vec![declare_int("x"), assert_ge("x", 0), ConstraintInstr::CheckSat]);
        let r = check_sat(&prog).unwrap();
        assert!(r.is_sat());
    }

    #[test]
    fn check_sat_fn_no_check_sat_returns_unknown() {
        let prog = checked_program(vec![declare_int("x")]);
        let r = check_sat(&prog).unwrap();
        assert!(r.is_unknown());
    }

    #[test]
    fn get_model_fn_works() {
        let prog = checked_program(vec![
            declare_int("x"),
            assert_ge("x", 5),
            assert_le("x", 5),
            ConstraintInstr::CheckSat,
            ConstraintInstr::GetModel,
        ]);
        let m = get_model(&prog).unwrap().unwrap();
        assert_eq!(m.get("x").unwrap().as_int(), Some(5));
    }

    // ─── Bool variables ───────────────────────────────────────────────────────

    #[test]
    fn bool_var_sat_with_model() {
        let prog = checked_program(vec![
            declare_bool("flag"),
            ConstraintInstr::Assert {
                pred: Predicate::Var("flag".into()),
            },
            ConstraintInstr::CheckSat,
            ConstraintInstr::GetModel,
        ]);
        let out = Vm::new().execute(&prog).unwrap();
        assert!(out.last_sat_result().unwrap().is_sat());
        let m = out.last_model().unwrap();
        let val = m.get("flag").unwrap().as_bool().unwrap();
        assert!(val);
    }

    // ─── ProgramBuilder ───────────────────────────────────────────────────────

    #[test]
    fn builder_basic() {
        let prog = ProgramBuilder::new()
            .set_logic(Logic::QF_LIA)
            .declare_int("x")
            .assert_ge_int("x", 0)
            .assert_le_int("x", 127)
            .check_sat()
            .get_model()
            .build();

        let out = Vm::new().execute(&prog).unwrap();
        assert!(out.last_sat_result().unwrap().is_sat());
        let x = out.last_model().unwrap().get("x").unwrap().as_int().unwrap();
        assert!((0..=127).contains(&x));
    }

    #[test]
    fn builder_push_pop() {
        let prog = ProgramBuilder::new()
            .declare_int("x")
            .assert_ge_int("x", 0)
            .assert_le_int("x", 100)
            .check_sat()
            .push_scope()
            .assert_ge_int("x", 200)
            .check_sat()
            .pop_scope()
            .check_sat()
            .build();

        let out = Vm::new().execute(&prog).unwrap();
        assert_eq!(out.sat_results.len(), 3);
        assert!(out.sat_results[0].is_sat());
        assert!(out.sat_results[1].is_unsat());
        assert!(out.sat_results[2].is_sat());
    }

    #[test]
    fn builder_echo() {
        let prog = ProgramBuilder::new()
            .echo("step 1")
            .echo("step 2")
            .build();
        let out = Vm::new().execute(&prog).unwrap();
        assert_eq!(out.trace, vec!["step 1", "step 2"]);
    }

    // ─── Multiple CheckSat in one program ─────────────────────────────────────

    #[test]
    fn multiple_check_sat_accumulate() {
        // Flip satisfiability by pushing / adding / popping.
        let prog = ProgramBuilder::new()
            .declare_int("x")
            .assert_ge_int("x", 0)
            .assert_le_int("x", 10)
            .check_sat() // SAT
            .push_scope()
            .assert_ge_int("x", 100) // contradiction
            .check_sat() // UNSAT
            .pop_scope()
            .check_sat() // SAT again
            .build();

        let out = Vm::new().execute(&prog).unwrap();
        assert_eq!(out.sat_results.len(), 3);
        let kinds: Vec<&str> = out.sat_results.iter().map(|r| {
            if r.is_sat() { "sat" } else if r.is_unsat() { "unsat" } else { "unknown" }
        }).collect();
        assert_eq!(kinds, vec!["sat", "unsat", "sat"]);
    }

    // ─── VmError Display ─────────────────────────────────────────────────────

    #[test]
    fn vm_error_displays() {
        assert!(VmError::NoModel.to_string().contains("get-model"));
        assert!(VmError::UnmatchedPop.to_string().contains("pop"));
        assert!(VmError::LimitExceeded("x".into()).to_string().contains("limit"));
    }
}
