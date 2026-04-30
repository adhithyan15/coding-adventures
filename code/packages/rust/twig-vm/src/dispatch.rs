//! # `dispatch` — the real interpreter dispatch loop for `twig-vm`.
//!
//! This module is **PR 4 of LANG20**.  It replaces the
//! 1-instruction `evaluate_call_builtin` helper with a complete
//! tree-walking dispatcher that runs an entire `IIRModule` end to
//! end and returns a `LispyValue`.
//!
//! ## Scope
//!
//! PR 4 covers the IIR subset emitted by `twig-ir-compiler` for
//! programs **without closures or top-level value defines**.  That
//! is exactly the set:
//!
//! | Opcode          | What it does                                      |
//! |-----------------|---------------------------------------------------|
//! | `const`         | bind register ← `Int` / `Bool` immediate          |
//! | `call_builtin`  | look up Lispy builtin, materialise args, dispatch |
//! | `call`          | resolve callee in module, recurse into dispatcher |
//! | `jmp`           | unconditional branch to label                     |
//! | `jmp_if_false`  | branch if cond register is `false` or `nil`       |
//! | `label`         | no-op marker (jump target)                        |
//! | `ret`           | return value to caller                            |
//!
//! Out of scope (later PRs):
//!
//! - `make_closure`, `apply_closure`, `make_builtin_closure` —
//!   need closure heap layout and indirect dispatch (PR 5+)
//! - `global_set`, `global_get` — need a per-process global table
//! - `make_symbol` — needs a `Symbol` `LispyValue` constructor
//!   from a runtime-string operand (lispy-runtime's `intern`
//!   accepts `&str`, but the IR routes the name through a
//!   `const` instruction which we'd need to track)
//! - `send`, `load`, `store` — need IC machinery from PR 6+
//!
//! Programs using any of these emit an opcode this dispatcher
//! refuses with a clear `UnsupportedOpcode` error, rather than
//! silently producing wrong answers.
//!
//! ## Recursion model
//!
//! Each `call` opcode recurses into a fresh `Frame`, using the
//! Rust call stack as the activation stack.  This is the same
//! tactic Python's `eval` uses internally; it caps Twig recursion
//! at the host's stack depth.  We additionally guard with
//! [`MAX_DISPATCH_DEPTH`] so adversarial input can't crash the
//! process before the OS kicks in.
//!
//! When PR 6+ adds JIT-style on-stack replacement we'll replace
//! this with an explicit frame stack inside `LangVM`.  The public
//! API (`run`) stays compatible.
//!
//! ## Frame layout
//!
//! `Frame::registers` is a `HashMap<String, LispyValue>` —
//! string-keyed by SSA name as emitted by the IIR compiler.  This
//! is **not** how `vm-core` will represent frames in production —
//! that's a `Vec<LispyValue>` indexed by register-id.  But the
//! IIR carries names today, and rewriting to ids is a separate
//! commit; this PR's contract is to make Twig programs *run*,
//! not to land the final register-allocator.

use std::collections::HashMap;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lang_runtime_core::RuntimeError;
use lispy_runtime::{LispyBinding, LispyValue};

use crate::operand::operand_to_value;

// ---------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------

/// Maximum number of nested `call` opcodes the dispatcher will
/// follow before refusing to recurse further.
///
/// At PR 4 each Twig call uses the host Rust stack frame, so this
/// indirectly caps stack usage.  The cap is generous enough for
/// `(fact 200)` to succeed but small enough that adversarial
/// input can't crash the process — modern host stacks tolerate
/// roughly 10⁴ frames before SIGSEGV.
pub const MAX_DISPATCH_DEPTH: usize = 256;

/// Maximum number of instructions any single dispatcher invocation
/// will execute before refusing.  Catches infinite loops in
/// hand-built malformed IIR (the parser/compiler can't produce
/// these for well-formed Twig source, but the VM has no way to
/// know that).
///
/// 2²⁰ ≈ one million instructions per top-level run is plenty for
/// any sensible Twig program — `(fact 1000)` executes ~10⁴
/// instructions.
pub const MAX_INSTRUCTIONS_PER_RUN: u64 = 1 << 20;

/// Maximum register-file size the dispatcher will pre-allocate
/// for a single frame.
///
/// `IIRFunction::register_count` is computed by the IR compiler,
/// but the dispatcher must not trust it — a hand-built module
/// with `register_count = usize::MAX` would allocate a
/// `HashMap` with that capacity and abort the process before
/// any instruction tick fires.  Clamping at 2¹⁶ matches what a
/// real-world Twig function ever uses (factorial uses ~12
/// registers; the largest test case here uses ~30) and bounds
/// the up-front allocation.
pub const MAX_REGISTERS_PER_FRAME: usize = 1 << 16;

// ---------------------------------------------------------------------------
// RunError
// ---------------------------------------------------------------------------

/// Errors the dispatcher can surface.
///
/// Distinct from [`crate::EvaluateError`] — this is the
/// full-program error type, returned by [`run`] and
/// [`crate::TwigVM::run`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// The module's `entry_point` doesn't name any function in
    /// `module.functions`.  Should never happen for compiler-
    /// generated modules — `twig-ir-compiler` always synthesises
    /// `main`.
    NoEntryPoint(String),

    /// The dispatcher hit an opcode it doesn't implement.  See the
    /// "Scope" section of the module docs for the supported set.
    UnsupportedOpcode(String),

    /// An instruction was structurally malformed — wrong number of
    /// operands, missing dest, etc.  This indicates a bug in the
    /// frontend, not in user code.
    MalformedInstruction(String),

    /// `Operand::Var(name)` referenced a register that hasn't been
    /// written.  In well-formed IIR every read is dominated by a
    /// write, so this is also a frontend bug.
    UnknownRegister(String),

    /// `call <name>` referenced a function that doesn't exist in
    /// the module.  Frontend bug or hand-built broken IIR.
    UnknownFunction(String),

    /// `jmp <label>` referenced a label that wasn't emitted by any
    /// `label` instruction in the same function.
    UnknownLabel(String),

    /// `call_builtin <name>` named a builtin that
    /// [`LispyBinding::resolve_builtin`] doesn't know.
    UnknownBuiltin(String),

    /// A `call_builtin` argument couldn't be converted to a
    /// `LispyValue`.
    OperandConversion(RuntimeError),

    /// A builtin or callee returned a runtime error.
    Runtime(RuntimeError),

    /// Recursion hit [`MAX_DISPATCH_DEPTH`] — refused.
    DepthExceeded,

    /// Total instruction count hit
    /// [`MAX_INSTRUCTIONS_PER_RUN`] — refused.
    InstructionLimitExceeded,

    /// Function arity mismatch on `call`.
    ArityMismatch {
        /// Number of parameters declared by the callee function.
        expected: usize,
        /// Number of arguments actually supplied at the call site.
        got: usize,
        /// Name of the callee function (the `srcs[0]` of the
        /// failing `call` instruction).
        callee: String,
    },

    /// Reached the end of a function body without ever executing a
    /// `ret`.  Frontend bug — `twig-ir-compiler` always emits a
    /// trailing `ret`.
    FellOffEnd(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::NoEntryPoint(s) => write!(f, "no entry point: {s:?}"),
            RunError::UnsupportedOpcode(s) => write!(f, "unsupported opcode: {s:?}"),
            RunError::MalformedInstruction(s) => write!(f, "malformed instruction: {s}"),
            RunError::UnknownRegister(s) => write!(f, "unknown register: {s:?}"),
            RunError::UnknownFunction(s) => write!(f, "unknown function: {s:?}"),
            RunError::UnknownLabel(s) => write!(f, "unknown label: {s:?}"),
            RunError::UnknownBuiltin(s) => write!(f, "unknown builtin: {s:?}"),
            RunError::OperandConversion(e) => write!(f, "operand conversion: {e}"),
            RunError::Runtime(e) => write!(f, "runtime error: {e}"),
            RunError::DepthExceeded => write!(f, "max dispatch depth ({MAX_DISPATCH_DEPTH}) exceeded"),
            RunError::InstructionLimitExceeded => {
                write!(f, "max instructions per run ({MAX_INSTRUCTIONS_PER_RUN}) exceeded")
            }
            RunError::ArityMismatch { expected, got, callee } => {
                write!(f, "arity mismatch on call to {callee:?}: expected {expected}, got {got}")
            }
            RunError::FellOffEnd(name) => write!(f, "function {name:?} fell off end without ret"),
        }
    }
}

impl std::error::Error for RunError {}

// ---------------------------------------------------------------------------
// Frame
// ---------------------------------------------------------------------------

/// Per-call-activation register file.  One `Frame` per `call`
/// opcode, plus one for the entry-point function.
///
/// **GC-rooting note** (relevant for PR 5+ when a real
/// collector lands): values stored in `registers` are roots that
/// must be traced.  Today the runtime uses `Box::leak` so heap
/// pointers never get freed — that lets us hold raw `LispyValue`
/// copies in this `HashMap` safely.  When the leak is replaced
/// with a real collector, this struct's storage becomes a root
/// set and the dispatcher will need to expose it to the GC's
/// `trace_value` hook.
#[derive(Debug)]
struct Frame {
    /// SSA name → current value.  Names are unique within a
    /// function (the IIR is in SSA form), so a flat HashMap is
    /// adequate.
    registers: HashMap<String, LispyValue>,
}

impl Frame {
    fn new(func: &IIRFunction, args: &[LispyValue]) -> Result<Self, RunError> {
        if args.len() != func.params.len() {
            return Err(RunError::ArityMismatch {
                expected: func.params.len(),
                got: args.len(),
                callee: func.name.clone(),
            });
        }
        // Cap the up-front HashMap capacity at a sane bound.
        // `register_count` comes from the IR compiler and is not
        // user-controlled in practice, but a hand-built module
        // could ship `register_count = usize::MAX`, which would
        // abort the process at allocation time.  See the
        // `MAX_REGISTERS_PER_FRAME` doc-comment for the rationale.
        let cap = func
            .register_count
            .max(args.len())
            .min(MAX_REGISTERS_PER_FRAME);
        let mut registers = HashMap::with_capacity(cap);
        for ((name, _ty), val) in func.params.iter().zip(args.iter()) {
            registers.insert(name.clone(), *val);
        }
        Ok(Frame { registers })
    }

    fn get(&self, name: &str) -> Option<LispyValue> {
        self.registers.get(name).copied()
    }

    fn set(&mut self, name: String, value: LispyValue) {
        self.registers.insert(name, value);
    }
}

// ---------------------------------------------------------------------------
// Label index
// ---------------------------------------------------------------------------

/// Map from `label` instruction's name → index of that instruction.
///
/// Built once per function on entry; jumps then resolve in O(1).
///
/// Errors on duplicate label names — `twig-ir-compiler` always
/// generates fresh names via `fresh_label`, so duplicates are
/// only possible from hand-built malformed IIR.  Failing fast
/// here prevents the silent "last write wins" behaviour where
/// a `jmp L` could quietly redirect to the wrong target.
fn build_label_index(func: &IIRFunction) -> Result<HashMap<String, usize>, RunError> {
    let mut idx = HashMap::new();
    for (i, instr) in func.instructions.iter().enumerate() {
        if instr.op == "label" {
            let name = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(RunError::MalformedInstruction(format!(
                    "label at instr {i} of {:?} missing Var operand", func.name
                ))),
            };
            if idx.insert(name.clone(), i).is_some() {
                return Err(RunError::MalformedInstruction(format!(
                    "duplicate label {name:?} in function {:?}", func.name
                )));
            }
        }
    }
    Ok(idx)
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

/// Run an `IIRModule` end-to-end and return the value produced by
/// its entry-point function.
///
/// This is the main entry point for [`crate::TwigVM::run`].  Most
/// callers should go through that facade rather than this free
/// function — the facade exists to grow per-VM state in later PRs.
///
/// # Errors
///
/// See [`RunError`] for the variant table.  In short: any
/// structural problem with the IR, any unsupported opcode, any
/// runtime trap raised by a builtin, and any resource limit
/// (depth, instruction count) all surface as a `RunError`.
pub fn run(module: &IIRModule) -> Result<LispyValue, RunError> {
    let entry_name = module
        .entry_point
        .as_deref()
        .ok_or_else(|| RunError::NoEntryPoint("module.entry_point is None".into()))?;
    let entry = module
        .functions
        .iter()
        .find(|f| f.name == entry_name)
        .ok_or_else(|| RunError::NoEntryPoint(entry_name.to_string()))?;

    let mut budget = ExecutionBudget::new();
    dispatch(module, entry, &[], 0, &mut budget)
}

// Per-run instruction counter — enforces
// [`MAX_INSTRUCTIONS_PER_RUN`].
struct ExecutionBudget {
    instructions: u64,
}

impl ExecutionBudget {
    fn new() -> Self {
        ExecutionBudget { instructions: 0 }
    }

    fn tick(&mut self) -> Result<(), RunError> {
        self.instructions += 1;
        if self.instructions > MAX_INSTRUCTIONS_PER_RUN {
            return Err(RunError::InstructionLimitExceeded);
        }
        Ok(())
    }
}

/// Execute `func` with `args` against the surrounding `module`.
///
/// `depth` is the current call-stack depth; the guard at the top
/// rejects depths beyond [`MAX_DISPATCH_DEPTH`].  Recursive calls
/// pass `depth + 1`.
fn dispatch(
    module: &IIRModule,
    func: &IIRFunction,
    args: &[LispyValue],
    depth: usize,
    budget: &mut ExecutionBudget,
) -> Result<LispyValue, RunError> {
    if depth > MAX_DISPATCH_DEPTH {
        return Err(RunError::DepthExceeded);
    }

    let mut frame = Frame::new(func, args)?;
    let labels = build_label_index(func)?;

    let mut pc = 0;
    while pc < func.instructions.len() {
        budget.tick()?;
        let instr = &func.instructions[pc];

        match instr.op.as_str() {
            "const" => {
                exec_const(instr, &mut frame)?;
                pc += 1;
            }
            "call_builtin" => {
                exec_call_builtin(instr, &mut frame)?;
                pc += 1;
            }
            "call" => {
                exec_call(module, instr, &mut frame, depth, budget)?;
                pc += 1;
            }
            "jmp" => {
                pc = exec_jmp(instr, &labels)?;
            }
            "jmp_if_false" => {
                pc = exec_jmp_if_false(instr, &frame, &labels, pc)?;
            }
            "label" => {
                pc += 1;
            }
            "ret" => {
                return exec_ret(instr, &frame);
            }
            other => {
                return Err(RunError::UnsupportedOpcode(other.to_string()));
            }
        }
    }

    Err(RunError::FellOffEnd(func.name.clone()))
}

// ---------------------------------------------------------------------------
// Per-opcode handlers
// ---------------------------------------------------------------------------

fn exec_const(instr: &IIRInstr, frame: &mut Frame) -> Result<(), RunError> {
    let dest = instr.dest.as_ref().ok_or_else(|| {
        RunError::MalformedInstruction("const requires dest".into())
    })?;
    let src = instr.srcs.first().ok_or_else(|| {
        RunError::MalformedInstruction("const requires srcs[0]".into())
    })?;
    let value = match src {
        Operand::Int(n) => {
            // Same range check as operand_to_value — keep behaviour
            // consistent across the two conversion sites.
            const MAX: i64 = (1 << 60) - 1;
            const MIN: i64 = -(1 << 60);
            if !(MIN..=MAX).contains(n) {
                return Err(RunError::OperandConversion(RuntimeError::TypeError(
                    format!("integer literal {n} outside Lispy's tagged-int range"),
                )));
            }
            LispyValue::int(*n)
        }
        Operand::Bool(b) => LispyValue::bool(*b),
        Operand::Float(_) => {
            return Err(RunError::OperandConversion(RuntimeError::TypeError(
                "Lispy doesn't have flonums yet".into(),
            )));
        }
        Operand::Var(_text) => {
            // The IR compiler emits `const _s1 = "literal"` (with
            // a Var operand carrying the string text) only when
            // wiring up `make_closure` / `make_symbol` /
            // `global_set` / `global_get` — all of which are out
            // of scope for PR 4.  If we see this, it means the
            // user wrote a Twig program that needs closures,
            // globals, or quoted symbols; the frontend emitted
            // valid IR for those features, but the dispatcher
            // can't run them yet.
            //
            // We surface this as UnsupportedOpcode (paramaterised
            // with a hint) so callers see a clear "not yet" rather
            // than a crash.
            return Err(RunError::UnsupportedOpcode(
                "const with string operand (closures / globals / symbols — PR 5+)".into(),
            ));
        }
    };
    frame.set(dest.clone(), value);
    Ok(())
}

fn exec_call_builtin(instr: &IIRInstr, frame: &mut Frame) -> Result<(), RunError> {
    let name = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.as_str(),
        Some(_) => return Err(RunError::MalformedInstruction(
            "call_builtin srcs[0] must be Var(name)".into(),
        )),
        None => return Err(RunError::MalformedInstruction(
            "call_builtin requires srcs[0]".into(),
        )),
    };

    let builtin = <LispyBinding as lang_runtime_core::LangBinding>::resolve_builtin(name)
        .ok_or_else(|| RunError::UnknownBuiltin(name.to_string()))?;

    let mut call_args: Vec<LispyValue> = Vec::with_capacity(instr.srcs.len().saturating_sub(1));
    for src in &instr.srcs[1..] {
        // Read-only borrow of frame for the lookup callback —
        // dropped before we touch frame.set below.
        let frame_ref = &*frame;
        let v = operand_to_value(src, &|n| frame_ref.get(n))
            .map_err(RunError::OperandConversion)?;
        call_args.push(v);
    }

    let result = builtin(&call_args).map_err(RunError::Runtime)?;
    if let Some(d) = &instr.dest {
        frame.set(d.clone(), result);
    }
    Ok(())
}

fn exec_call(
    module: &IIRModule,
    instr: &IIRInstr,
    frame: &mut Frame,
    depth: usize,
    budget: &mut ExecutionBudget,
) -> Result<(), RunError> {
    let callee_name = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.as_str(),
        Some(_) => return Err(RunError::MalformedInstruction(
            "call srcs[0] must be Var(name)".into(),
        )),
        None => return Err(RunError::MalformedInstruction(
            "call requires srcs[0]".into(),
        )),
    };

    let callee = module
        .functions
        .iter()
        .find(|f| f.name == callee_name)
        .ok_or_else(|| RunError::UnknownFunction(callee_name.to_string()))?;

    let mut call_args: Vec<LispyValue> = Vec::with_capacity(instr.srcs.len().saturating_sub(1));
    for src in &instr.srcs[1..] {
        let frame_ref = &*frame;
        let v = operand_to_value(src, &|n| frame_ref.get(n))
            .map_err(RunError::OperandConversion)?;
        call_args.push(v);
    }

    let result = dispatch(module, callee, &call_args, depth + 1, budget)?;
    if let Some(d) = &instr.dest {
        frame.set(d.clone(), result);
    }
    Ok(())
}

fn exec_jmp(instr: &IIRInstr, labels: &HashMap<String, usize>) -> Result<usize, RunError> {
    let label = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.as_str(),
        _ => return Err(RunError::MalformedInstruction(
            "jmp requires Var(label) in srcs[0]".into(),
        )),
    };
    labels
        .get(label)
        .copied()
        .ok_or_else(|| RunError::UnknownLabel(label.to_string()))
}

fn exec_jmp_if_false(
    instr: &IIRInstr,
    frame: &Frame,
    labels: &HashMap<String, usize>,
    pc: usize,
) -> Result<usize, RunError> {
    // srcs[0] = condition register, srcs[1] = label
    let cond_name = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.as_str(),
        _ => return Err(RunError::MalformedInstruction(
            "jmp_if_false requires Var(cond) in srcs[0]".into(),
        )),
    };
    let label = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s.as_str(),
        _ => return Err(RunError::MalformedInstruction(
            "jmp_if_false requires Var(label) in srcs[1]".into(),
        )),
    };
    let cond = frame
        .get(cond_name)
        .ok_or_else(|| RunError::UnknownRegister(cond_name.to_string()))?;
    // Scheme semantics: only #f and nil branch.  Everything else
    // (including 0, empty string, empty list head, etc.) is truthy.
    if cond.is_truthy() {
        Ok(pc + 1)
    } else {
        labels
            .get(label)
            .copied()
            .ok_or_else(|| RunError::UnknownLabel(label.to_string()))
    }
}

fn exec_ret(instr: &IIRInstr, frame: &Frame) -> Result<LispyValue, RunError> {
    let src = instr.srcs.first().ok_or_else(|| {
        RunError::MalformedInstruction("ret requires srcs[0]".into())
    })?;
    operand_to_value(src, &|n| frame.get(n)).map_err(RunError::OperandConversion)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use twig_ir_compiler::compile_source;

    fn run_source(src: &str) -> Result<LispyValue, RunError> {
        let module = compile_source(src, "test").expect("compilation failed");
        run(&module)
    }

    // ── Arithmetic and comparisons ──────────────────────────────────

    #[test]
    fn addition_returns_sum() {
        assert_eq!(run_source("(+ 1 2)").unwrap().as_int(), Some(3));
    }

    #[test]
    fn subtraction_returns_diff() {
        assert_eq!(run_source("(- 10 3)").unwrap().as_int(), Some(7));
    }

    #[test]
    fn multiplication_returns_product() {
        assert_eq!(run_source("(* 6 7)").unwrap().as_int(), Some(42));
    }

    #[test]
    fn nested_arithmetic() {
        // (+ (* 2 3) (- 10 4)) → 6 + 6 = 12
        assert_eq!(run_source("(+ (* 2 3) (- 10 4))").unwrap().as_int(), Some(12));
    }

    #[test]
    fn comparison_lt_true() {
        assert_eq!(run_source("(< 1 2)").unwrap(), LispyValue::TRUE);
    }

    #[test]
    fn comparison_lt_false() {
        assert_eq!(run_source("(< 5 5)").unwrap(), LispyValue::FALSE);
    }

    #[test]
    fn equality_true() {
        assert_eq!(run_source("(= 7 7)").unwrap(), LispyValue::TRUE);
    }

    // ── if ──────────────────────────────────────────────────────────

    #[test]
    fn if_true_branch_taken() {
        assert_eq!(
            run_source("(if (< 1 2) 100 200)").unwrap().as_int(),
            Some(100),
        );
    }

    #[test]
    fn if_false_branch_taken() {
        assert_eq!(
            run_source("(if (< 5 2) 100 200)").unwrap().as_int(),
            Some(200),
        );
    }

    #[test]
    fn if_with_bool_literal_condition() {
        assert_eq!(run_source("(if #t 1 2)").unwrap().as_int(), Some(1));
        assert_eq!(run_source("(if #f 1 2)").unwrap().as_int(), Some(2));
    }

    // ── let ─────────────────────────────────────────────────────────

    #[test]
    fn let_single_binding() {
        assert_eq!(run_source("(let ((x 5)) (* x x))").unwrap().as_int(), Some(25));
    }

    #[test]
    fn let_multiple_bindings() {
        assert_eq!(
            run_source("(let ((x 1) (y 2)) (+ x y))").unwrap().as_int(),
            Some(3),
        );
    }

    #[test]
    fn let_body_uses_let_bound_names_only() {
        // `let` is parallel-bind: y on the RHS does NOT see x's
        // freshly-bound value.  Twig source:
        //   (let ((x 10) (y x)) y)
        // would error at compile time because `x` isn't yet in
        // outer scope.  Instead, test that nested lets work as
        // expected.
        let src = "(let ((x 1)) (let ((y (+ x 1))) (+ x y)))";
        assert_eq!(run_source(src).unwrap().as_int(), Some(3));
    }

    // ── begin ───────────────────────────────────────────────────────

    #[test]
    fn begin_returns_last_value() {
        assert_eq!(run_source("(begin 1 2 3)").unwrap().as_int(), Some(3));
    }

    // ── User-defined functions ──────────────────────────────────────

    #[test]
    fn define_and_call_simple() {
        let src = "(define (square x) (* x x)) (square 7)";
        assert_eq!(run_source(src).unwrap().as_int(), Some(49));
    }

    #[test]
    fn define_two_args() {
        let src = "(define (add3 a b c) (+ (+ a b) c)) (add3 1 2 3)";
        assert_eq!(run_source(src).unwrap().as_int(), Some(6));
    }

    #[test]
    fn factorial_recursion() {
        let src = "
            (define (fact n)
              (if (= n 0) 1 (* n (fact (- n 1)))))
            (fact 5)
        ";
        assert_eq!(run_source(src).unwrap().as_int(), Some(120));
    }

    #[test]
    fn fibonacci_recursion() {
        let src = "
            (define (fib n)
              (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))
            (fib 10)
        ";
        assert_eq!(run_source(src).unwrap().as_int(), Some(55));
    }

    #[test]
    fn mutual_recursion() {
        // even? and odd? — classic mutual recursion test.
        let src = "
            (define (is_even n)
              (if (= n 0) #t (is_odd (- n 1))))
            (define (is_odd n)
              (if (= n 0) #f (is_even (- n 1))))
            (is_even 10)
        ";
        assert_eq!(run_source(src).unwrap(), LispyValue::TRUE);
    }

    // ── Cons / car / cdr through the dispatcher ─────────────────────

    #[test]
    fn cons_car_returns_first() {
        assert_eq!(run_source("(car (cons 1 2))").unwrap().as_int(), Some(1));
    }

    #[test]
    fn cons_cdr_returns_second() {
        assert_eq!(run_source("(cdr (cons 1 2))").unwrap().as_int(), Some(2));
    }

    #[test]
    fn pair_p_is_true_for_cons_cell() {
        assert_eq!(run_source("(pair? (cons 1 2))").unwrap(), LispyValue::TRUE);
    }

    // ── Bool / nil handling in conditionals ─────────────────────────

    #[test]
    fn zero_is_truthy_in_scheme() {
        // Crucial: in Scheme, 0 is truthy (only #f and nil are false).
        assert_eq!(run_source("(if 0 1 2)").unwrap().as_int(), Some(1));
    }

    // ── Error paths ─────────────────────────────────────────────────

    #[test]
    fn no_entry_point_errors_out() {
        let mut module = compile_source("(+ 1 2)", "test").unwrap();
        module.entry_point = None;
        let err = run(&module).unwrap_err();
        assert!(matches!(err, RunError::NoEntryPoint(_)));
    }

    #[test]
    fn entry_point_referencing_missing_fn_errors() {
        let mut module = compile_source("(+ 1 2)", "test").unwrap();
        module.entry_point = Some("does_not_exist".into());
        let err = run(&module).unwrap_err();
        match err {
            RunError::NoEntryPoint(s) => assert_eq!(s, "does_not_exist"),
            other => panic!("expected NoEntryPoint, got {other:?}"),
        }
    }

    #[test]
    fn unknown_opcode_surfaces_unsupported() {
        // Hand-craft a module that uses an opcode we don't support.
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        let main = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: 4,
            instructions: vec![
                IIRInstr::new("not_a_real_opcode", None, vec![], "any"),
            ],
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        };
        let module = IIRModule {
            name: "test".into(),
            functions: vec![main],
            entry_point: Some("main".into()),
            language: "twig".into(),
        };
        let err = run(&module).unwrap_err();
        assert!(matches!(err, RunError::UnsupportedOpcode(s) if s == "not_a_real_opcode"));
    }

    #[test]
    fn fall_off_end_errors() {
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        // Function with no instructions at all → falls off end immediately.
        let main = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: 0,
            instructions: vec![],
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        };
        let module = IIRModule {
            name: "test".into(),
            functions: vec![main],
            entry_point: Some("main".into()),
            language: "twig".into(),
        };
        let err = run(&module).unwrap_err();
        assert!(matches!(err, RunError::FellOffEnd(s) if s == "main"));
    }

    #[test]
    fn division_by_zero_surfaces_runtime() {
        let err = run_source("(/ 7 0)").unwrap_err();
        assert!(matches!(err, RunError::Runtime(RuntimeError::TypeError(_))));
    }

    #[test]
    fn unknown_function_in_call_errors() {
        // Compiler enforces this at compile time, but if a hand-
        // built module names a missing callee we surface
        // UnknownFunction.
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        let main = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: 4,
            instructions: vec![
                IIRInstr::new("call", Some("r".into()), vec![Operand::Var("ghost".into())], "any"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
            ],
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        };
        let module = IIRModule {
            name: "test".into(),
            functions: vec![main],
            entry_point: Some("main".into()),
            language: "twig".into(),
        };
        let err = run(&module).unwrap_err();
        assert!(matches!(err, RunError::UnknownFunction(s) if s == "ghost"));
    }

    #[test]
    fn deep_recursion_surfaces_depth_exceeded() {
        // Self-recursion that never terminates — should hit the
        // depth cap rather than blowing the host stack.  We don't
        // assert the exact depth (the limit is generous), only
        // that we get the right error variant.
        let src = "
            (define (loop n) (loop (+ n 1)))
            (loop 0)
        ";
        let err = run_source(src).unwrap_err();
        // Depth or instruction limit — both are acceptable
        // termination signals for this infinite-recursion test.
        assert!(
            matches!(err, RunError::DepthExceeded | RunError::InstructionLimitExceeded),
            "expected DepthExceeded or InstructionLimitExceeded, got {err:?}",
        );
    }

    // ── Internal helpers ────────────────────────────────────────────

    #[test]
    fn frame_arity_check_fires_on_mismatch() {
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        let f = IIRFunction {
            name: "f".into(),
            params: vec![("x".into(), "any".into())],
            return_type: "any".into(),
            register_count: 1,
            instructions: vec![],
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        };
        let err = Frame::new(&f, &[]).unwrap_err();
        match err {
            RunError::ArityMismatch { expected, got, callee } => {
                assert_eq!(expected, 1);
                assert_eq!(got, 0);
                assert_eq!(callee, "f");
            }
            other => panic!("expected ArityMismatch, got {other:?}"),
        }
    }

    #[test]
    fn build_label_index_rejects_duplicate_labels() {
        // Hand-build a function with two `label "L"` instructions.
        // The IR compiler never emits this (fresh_label generates
        // unique names) but the dispatcher must refuse it rather
        // than silently letting the second occurrence shadow the
        // first.
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        let f = IIRFunction {
            name: "f".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: 1,
            instructions: vec![
                IIRInstr::new("label", None, vec![Operand::Var("L".into())], "void"),
                IIRInstr::new("label", None, vec![Operand::Var("L".into())], "void"),
            ],
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        };
        let err = build_label_index(&f).unwrap_err();
        match err {
            RunError::MalformedInstruction(s) => assert!(s.contains("duplicate label")),
            other => panic!("expected MalformedInstruction, got {other:?}"),
        }
    }

    #[test]
    fn frame_caps_register_count() {
        // Hand-build a function claiming usize::MAX registers.
        // Without the cap, `HashMap::with_capacity(usize::MAX)`
        // aborts the process; with it, allocation succeeds and
        // the dispatcher can refuse on a more graceful boundary.
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        let f = IIRFunction {
            name: "huge".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: usize::MAX,
            instructions: vec![],
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        };
        // Should succeed (clamped to MAX_REGISTERS_PER_FRAME).
        let frame = Frame::new(&f, &[]).expect("frame creation must not abort");
        // Capacity is implementation-defined to be ≥ requested,
        // but should not be near usize::MAX.
        assert!(
            frame.registers.capacity() <= MAX_REGISTERS_PER_FRAME * 2,
            "capacity {} should be near MAX_REGISTERS_PER_FRAME ({MAX_REGISTERS_PER_FRAME})",
            frame.registers.capacity(),
        );
    }

    #[test]
    fn build_label_index_finds_emitted_labels() {
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        let f = IIRFunction {
            name: "f".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: 1,
            instructions: vec![
                IIRInstr::new("label", None, vec![Operand::Var("L1".into())], "void"),
                IIRInstr::new("label", None, vec![Operand::Var("L2".into())], "void"),
            ],
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        };
        let idx = build_label_index(&f).unwrap();
        assert_eq!(idx.get("L1"), Some(&0));
        assert_eq!(idx.get("L2"), Some(&1));
    }

    #[test]
    fn execution_budget_enforces_cap() {
        let mut b = ExecutionBudget::new();
        // Burn through the budget — increment to one past the limit.
        for _ in 0..MAX_INSTRUCTIONS_PER_RUN {
            b.tick().expect("budget shouldn't overflow yet");
        }
        // The MAX_INSTRUCTIONS_PER_RUN+1th tick should fail.
        let err = b.tick().unwrap_err();
        assert!(matches!(err, RunError::InstructionLimitExceeded));
    }
}
