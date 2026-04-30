//! # `dispatch` — the real interpreter dispatch loop for `twig-vm`.
//!
//! Originally LANG20 PR 4 (tree-walking dispatcher); extended in
//! PR 5 to cover closures, top-level value defines, and quoted
//! symbols; extended in PR 6 to cover the method-dispatch
//! opcodes `send`, `load_property`, `store_property`.  Runs an
//! entire `IIRModule` end to end and returns a `LispyValue`.
//!
//! ## Scope
//!
//! Through PR 6 the dispatcher covers the IIR subset emitted by
//! every Lispy frontend plus the method-dispatch opcodes Ruby /
//! JavaScript frontends will eventually need.  The supported
//! opcodes:
//!
//! | Opcode             | What it does                                      |
//! |--------------------|---------------------------------------------------|
//! | `const Int/Bool`   | bind register ← `Int` / `Bool` immediate          |
//! | `const Var(s)`     | bind register ← `LispyValue::symbol(intern(s))` (PR 5: string-via-symbol convention) |
//! | `call_builtin`     | look up Lispy builtin OR special-case             |
//! | `call`             | resolve callee in module, recurse into dispatcher |
//! | `jmp`              | unconditional branch to label                     |
//! | `jmp_if_false`     | branch if cond register is `false` or `nil`       |
//! | `label`            | no-op marker (jump target)                        |
//! | `ret`              | return value to caller                            |
//! | `send`             | (PR 6) dispatch a method via `LangBinding::send_message` |
//! | `load_property`    | (PR 6) read a property via `LangBinding::load_property` |
//! | `store_property`   | (PR 6) write a property via `LangBinding::store_property` |
//!
//! ### Special-cased `call_builtin` names (PR 5)
//!
//! Three builtin names need access to per-VM state (globals
//! table, IIRModule, dispatcher recursion) and so are handled
//! inline in `exec_call_builtin` rather than as context-free
//! `BuiltinFn` pointers:
//!
//! | Name            | Behaviour                                          |
//! |-----------------|----------------------------------------------------|
//! | `apply_closure` | Extract `(fn_name, captures)` from the closure handle.  If the closure flag says builtin, dispatch via `LispyBinding::resolve_builtin`.  Else look `fn_name` up in `module.functions` and recurse into `dispatch` with `captures ++ args`. |
//! | `global_set`    | Write `globals[name] = value` (name and value both supplied as srcs). |
//! | `global_get`    | Read `globals[name]`; error if unset.              |
//!
//! All other `call_builtin` names route through
//! `LispyBinding::resolve_builtin` exactly as before.
//!
//! ### Method dispatch opcodes (PR 6)
//!
//! The three opcodes added in PR 6 (`send`, `load_property`,
//! `store_property`) all share the same shape: extract the
//! receiver/object from `srcs[0]`, extract a `SymbolId` from
//! `srcs[1]` (the selector / property key, lowered through the
//! string-as-symbol convention), allocate a per-instruction
//! [`InlineCache<LispyICEntry>`], and call the corresponding
//! `LangBinding` trait method.
//!
//! For Lispy specifically the binding methods correctly return
//! `RuntimeError::NoSuchMethod` / `NoSuchProperty` — Lispy doesn't
//! have method dispatch, and a Twig program that emits these
//! opcodes is using a feature the language doesn't have.  PR 6's
//! value is **wiring the opcodes through the trait machinery**
//! so a future Ruby- or JS-binding can implement them and have
//! them dispatched by exactly the same dispatcher.  Tests
//! hand-build IIRModules using these opcodes since
//! `twig-ir-compiler` doesn't emit them yet (no `(send obj msg
//! ...)` form in Twig source).
//!
//! The IC parameter is allocated **fresh per dispatch** in PR 6
//! (no caching across calls); PR 7 lands the IC machinery that
//! makes the cache persistent + populated by the binding's
//! `send_message` / `load_property` / `store_property`
//! implementations.  Until then, the binding's IC handling is a
//! no-op for Lispy and the cache fills nothing.
//!
//! Out of scope (later PRs):
//!
//! - **Persistent IC slots** — PR 7 adds the per-call-site IC
//!   table indexed by `IIRInstr::ic_slot` (LANG20 §"IIR additions").
//! - **JIT promotion, deopt** — PR 8+.
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
use lang_runtime_core::{
    DispatchCx, InlineCache, LangBinding, RuntimeError, SymbolId,
};
use lispy_runtime::{intern, name_of, LispyBinding, LispyICEntry, LispyValue};

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
/// Returned by [`run`] and [`crate::TwigVM::run`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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

    /// `global_get <name>` referenced a name that has never been
    /// the target of a `global_set`.  In Twig source this is a
    /// "use before define" — a forward reference to a top-level
    /// `(define x ...)` from inside a function whose body runs
    /// before the define.
    UndefinedGlobal(String),

    /// `apply_closure` was called on a value that isn't a closure
    /// (heap-allocated with `class_or_kind == CLASS_CLOSURE`).
    /// User-visible "<value> is not callable" surface.
    NotCallable(String),
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
            RunError::UndefinedGlobal(s) => write!(f, "undefined global: {s:?}"),
            RunError::NotCallable(s) => write!(f, "not callable: {s}"),
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

    /// Insert or update `name → value`.
    ///
    /// Errors if the frame already holds [`MAX_REGISTERS_PER_FRAME`]
    /// distinct names and `name` is new.  Hand-built malformed IIR
    /// could otherwise grow the per-frame `HashMap` unboundedly with
    /// fresh names (one heap allocation per name); the per-run
    /// instruction budget partially limits this but not strongly
    /// enough to prevent multi-gigabyte allocation.  Found by
    /// PR 5 security review (#10).
    fn set(&mut self, name: String, value: LispyValue) -> Result<(), RunError> {
        if !self.registers.contains_key(&name)
            && self.registers.len() >= MAX_REGISTERS_PER_FRAME
        {
            return Err(RunError::MalformedInstruction(format!(
                "frame register count exceeds MAX_REGISTERS_PER_FRAME ({MAX_REGISTERS_PER_FRAME})"
            )));
        }
        self.registers.insert(name, value);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------

/// Top-level value-define table.  One per [`run`] invocation;
/// shared across all `Frame`s within that run via a `&mut`
/// reference threaded through `dispatch`.
///
/// Twig's `(define x value)` form lowers to
/// `call_builtin "global_set" name value`; references to top-level
/// values lower to `call_builtin "global_get" name`.  This struct
/// is the storage backing both.
///
/// Keyed by [`SymbolId`] rather than `String` because the IR
/// compiler interns names through `lispy-runtime::intern` already
/// (the dispatcher's `const Var(text)` handler creates a symbol),
/// so SymbolId comparisons stay O(1) integer-equality.
///
/// **Lifetime note.**  Globals are per-run, not per-process.
/// Calling `TwigVM::run` twice gives two independent global
/// tables.  When PR 6+ adds `LangVM`-level state, globals will
/// move there and persist across runs — at that point this
/// struct moves to a shared location.  The dispatcher API stays
/// the same.
#[derive(Debug, Default)]
pub struct Globals {
    map: HashMap<SymbolId, LispyValue>,
}

impl Globals {
    /// Construct an empty globals table.
    pub fn new() -> Self {
        Globals { map: HashMap::new() }
    }

    /// Read a global by interned name.  Returns `None` if the name
    /// has never been the target of a `set`.
    pub fn get(&self, name: SymbolId) -> Option<LispyValue> {
        self.map.get(&name).copied()
    }

    /// Write `value` to the global named `name`.  Overwrites any
    /// prior value.
    pub fn set(&mut self, name: SymbolId, value: LispyValue) {
        self.map.insert(name, value);
    }

    /// Number of globals currently set.  Mostly for testing.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// `true` if no globals are set.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
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
    let mut globals = Globals::new();
    run_with_globals(module, &mut globals)
}

/// Run an `IIRModule` against a caller-supplied globals table.
///
/// Used by tests that want to inspect the table after the run, or
/// by future per-VM state (PR 6+) that wants globals to persist
/// across multiple `run` calls.  Most callers should use [`run`].
pub fn run_with_globals(
    module: &IIRModule,
    globals: &mut Globals,
) -> Result<LispyValue, RunError> {
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
    dispatch(module, entry, &[], 0, &mut budget, globals)
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
    globals: &mut Globals,
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
                exec_call_builtin(module, instr, &mut frame, depth, budget, globals)?;
                pc += 1;
            }
            "call" => {
                exec_call(module, instr, &mut frame, depth, budget, globals)?;
                pc += 1;
            }
            "send" => {
                exec_send(instr, &mut frame)?;
                pc += 1;
            }
            "load_property" => {
                exec_load_property(instr, &mut frame)?;
                pc += 1;
            }
            "store_property" => {
                exec_store_property(instr, &mut frame)?;
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
        Operand::Var(text) => {
            // PR 5: `const _s1 = "literal"` — the IR compiler emits
            // these for the string-shaped arguments to
            // `make_closure` / `make_builtin_closure` /
            // `make_symbol` / `global_set` / `global_get`.  We
            // intern the text and store the result as a symbol;
            // downstream builtins read the symbol's name back via
            // the intern table when they need the original string.
            //
            // The "string-as-symbol" convention loses the user-
            // facing distinction between strings and symbols.
            // Intentional at PR 5: Lispy's surface syntax has
            // only symbols (no string literal form).  When a
            // proper string value lands in a future PR, this arm
            // changes; the IR compiler already emits the right
            // operand shape.
            // Detect intern-table exhaustion eagerly — `intern`
            // returns `SymbolId::NONE` when the table is full, and
            // letting NONE flow through would surface as a confusing
            // `MalformedInstruction("...unknown fn_name id...")`
            // much later in `exec_apply_closure`.  Found by PR 5
            // security review (#8).
            let id = intern(text);
            if id == SymbolId::NONE {
                return Err(RunError::Runtime(RuntimeError::TypeError(format!(
                    "intern table exhausted: cannot intern {text:?}"
                ))));
            }
            LispyValue::symbol(id)
        }
    };
    frame.set(dest.clone(), value)?;
    Ok(())
}

fn exec_call_builtin(
    module: &IIRModule,
    instr: &IIRInstr,
    frame: &mut Frame,
    depth: usize,
    budget: &mut ExecutionBudget,
    globals: &mut Globals,
) -> Result<(), RunError> {
    let name = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.as_str(),
        Some(_) => return Err(RunError::MalformedInstruction(
            "call_builtin srcs[0] must be Var(name)".into(),
        )),
        None => return Err(RunError::MalformedInstruction(
            "call_builtin requires srcs[0]".into(),
        )),
    };

    // ── Special-cased builtins (PR 5) ────────────────────────────
    //
    // These names need access to per-VM state (globals table,
    // module reference, dispatcher recursion) and so are handled
    // inline here rather than as context-free `BuiltinFn`
    // pointers.  Everything else falls through to the normal
    // resolve_builtin path.
    match name {
        "global_set" => return exec_global_set(instr, frame, globals),
        "global_get" => return exec_global_get(instr, frame, globals),
        "apply_closure" => {
            return exec_apply_closure(module, instr, frame, depth, budget, globals);
        }
        _ => {}
    }

    // ── Normal builtin path ──────────────────────────────────────
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
        frame.set(d.clone(), result)?;
    }
    Ok(())
}

fn exec_call(
    module: &IIRModule,
    instr: &IIRInstr,
    frame: &mut Frame,
    depth: usize,
    budget: &mut ExecutionBudget,
    globals: &mut Globals,
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

    let result = dispatch(module, callee, &call_args, depth + 1, budget, globals)?;
    if let Some(d) = &instr.dest {
        frame.set(d.clone(), result)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// PR 5: special-cased builtins (need module / globals / recursion)
// ---------------------------------------------------------------------------

/// Handle `call_builtin "global_set" name value`.
///
/// `srcs[1]` is the global's name (must resolve to a symbol value
/// — the IR compiler emits a `const`-via-symbol for it); `srcs[2]`
/// is the value to store.  No dest (return value is discarded;
/// the IR compiler emits these for top-level value-defines).
fn exec_global_set(
    instr: &IIRInstr,
    frame: &mut Frame,
    globals: &mut Globals,
) -> Result<(), RunError> {
    if instr.srcs.len() != 3 {
        return Err(RunError::MalformedInstruction(format!(
            "global_set expects 3 srcs (name, name_arg, value), got {}",
            instr.srcs.len()
        )));
    }
    let frame_ref = &*frame;
    let name_v = operand_to_value(&instr.srcs[1], &|n| frame_ref.get(n))
        .map_err(RunError::OperandConversion)?;
    let value = operand_to_value(&instr.srcs[2], &|n| frame_ref.get(n))
        .map_err(RunError::OperandConversion)?;
    let name_id = name_v.as_symbol().ok_or_else(|| {
        RuntimeError::TypeError(format!("global_set: expected symbol name, got {name_v}"))
    }).map_err(RunError::Runtime)?;
    globals.set(name_id, value);
    Ok(())
}

/// Handle `call_builtin "global_get" name`.
///
/// `srcs[1]` is the global's name (symbol).  Returns the stored
/// value; errors with `UndefinedGlobal` if the name has never been
/// the target of a `global_set`.
fn exec_global_get(
    instr: &IIRInstr,
    frame: &mut Frame,
    globals: &Globals,
) -> Result<(), RunError> {
    if instr.srcs.len() != 2 {
        return Err(RunError::MalformedInstruction(format!(
            "global_get expects 2 srcs (\"global_get\", name_arg), got {}",
            instr.srcs.len()
        )));
    }
    let frame_ref = &*frame;
    let name_v = operand_to_value(&instr.srcs[1], &|n| frame_ref.get(n))
        .map_err(RunError::OperandConversion)?;
    let name_id = name_v.as_symbol().ok_or_else(|| {
        RuntimeError::TypeError(format!("global_get: expected symbol name, got {name_v}"))
    }).map_err(RunError::Runtime)?;
    let value = globals.get(name_id).ok_or_else(|| {
        let s = name_of(name_id).unwrap_or_else(|| format!("<symbol {}>", name_id.0));
        RunError::UndefinedGlobal(s)
    })?;
    if let Some(d) = &instr.dest {
        frame.set(d.clone(), value)?;
    }
    Ok(())
}

/// Handle `call_builtin "apply_closure" handle arg0 arg1 ...`.
///
/// `srcs[1]` is the closure handle (must be a heap value with
/// `class_or_kind == CLASS_CLOSURE`).  Remaining srcs are the
/// user-supplied arguments.
///
/// **For builtin closures** (`CLOSURE_FLAG_BUILTIN`): look the
/// closure's `fn_name` up via `LispyBinding::resolve_builtin` and
/// call the resolved fn pointer with the user args.  Captures
/// are guaranteed empty by construction.
///
/// **For user-fn closures**: look `fn_name` up in
/// `module.functions`, recurse into `dispatch` with
/// `captures ++ args` as the parameter list (Twig closures
/// receive captures as the first parameters of the underlying
/// function — see `twig-ir-compiler` §"Anonymous lambda").
fn exec_apply_closure(
    module: &IIRModule,
    instr: &IIRInstr,
    frame: &mut Frame,
    depth: usize,
    budget: &mut ExecutionBudget,
    globals: &mut Globals,
) -> Result<(), RunError> {
    if instr.srcs.len() < 2 {
        return Err(RunError::MalformedInstruction(format!(
            "apply_closure expects at least 2 srcs (\"apply_closure\", handle), got {}",
            instr.srcs.len()
        )));
    }

    // Resolve the handle to a LispyValue and confirm it's a closure.
    let frame_ref = &*frame;
    let handle = operand_to_value(&instr.srcs[1], &|n| frame_ref.get(n))
        .map_err(RunError::OperandConversion)?;
    // SAFETY: closure values come from `make_closure` /
    // `make_builtin_closure` which go through alloc_*_closure;
    // those return properly-tagged heap pointers that live forever
    // (PR 2 leak).  `as_closure` walks the header; safe to call.
    let closure = unsafe { lispy_runtime::as_closure(handle) }.ok_or_else(|| {
        RunError::NotCallable(format!("apply_closure: {handle} is not a closure"))
    })?;
    let fn_name_id = closure.fn_name;
    let captures = closure.captures.clone();
    let is_builtin = closure.is_builtin();

    // Resolve user args.
    let mut user_args: Vec<LispyValue> = Vec::with_capacity(instr.srcs.len() - 2);
    for src in &instr.srcs[2..] {
        let v = operand_to_value(src, &|n| frame_ref.get(n))
            .map_err(RunError::OperandConversion)?;
        user_args.push(v);
    }

    let result = if is_builtin {
        // Defense-in-depth: a well-formed `make_builtin_closure`
        // produces no captures, but we explicitly assert this so a
        // malformed Closure (`flags = CLOSURE_FLAG_BUILTIN` AND
        // non-empty captures via `Closure` field-level access) can't
        // silently drop captures.  Found by PR 5 security review (#2).
        debug_assert!(
            captures.is_empty(),
            "builtin closure must have no captures (got {})",
            captures.len(),
        );
        // Builtin closure: dispatch via resolve_builtin.
        let name_str = name_of(fn_name_id).ok_or_else(|| {
            RunError::MalformedInstruction(format!(
                "apply_closure: builtin closure has unknown fn_name id {}",
                fn_name_id.0
            ))
        })?;
        let builtin = <LispyBinding as lang_runtime_core::LangBinding>::resolve_builtin(&name_str)
            .ok_or_else(|| RunError::UnknownBuiltin(name_str.clone()))?;
        builtin(&user_args).map_err(RunError::Runtime)?
    } else {
        // User-fn closure: prepend captures, look up function,
        // recurse into dispatch.
        let mut all_args = captures;
        all_args.extend(user_args);
        let name_str = name_of(fn_name_id).ok_or_else(|| {
            RunError::MalformedInstruction(format!(
                "apply_closure: closure has unknown fn_name id {}",
                fn_name_id.0
            ))
        })?;
        let callee = module
            .functions
            .iter()
            .find(|f| f.name == name_str)
            .ok_or_else(|| RunError::UnknownFunction(name_str.clone()))?;
        dispatch(module, callee, &all_args, depth + 1, budget, globals)?
    };

    if let Some(d) = &instr.dest {
        frame.set(d.clone(), result)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// PR 6: method-dispatch opcodes (send / load_property / store_property)
// ---------------------------------------------------------------------------
//
// These three opcodes route through the corresponding `LangBinding`
// trait method.  For Lispy, the trait methods correctly return
// `NoSuchMethod` / `NoSuchProperty` (Lispy has no method dispatch);
// the value of PR 6 is wiring the dispatcher path so a future
// Ruby-binding or JS-binding can implement these and immediately
// have them dispatched without further dispatcher changes.
//
// The IC parameter is allocated **fresh per dispatch** in PR 6.
// PR 7 (IC machinery) introduces a per-call-site IC table indexed
// by `IIRInstr::ic_slot` — at that point this allocation moves to
// table lookup but the trait calls stay identical.

/// Helper: extract the SymbolId stored in a register (the
/// dispatcher's string-as-symbol convention from PR 5).  Used by
/// every method-dispatch opcode for the selector / key argument.
fn read_symbol_arg(
    operand: &Operand,
    frame: &Frame,
    op_for_msg: &str,
    arg_idx_for_msg: usize,
) -> Result<SymbolId, RunError> {
    let v = operand_to_value(operand, &|n| frame.get(n))
        .map_err(RunError::OperandConversion)?;
    v.as_symbol().ok_or_else(|| {
        RunError::Runtime(RuntimeError::TypeError(format!(
            "{op_for_msg}: srcs[{arg_idx_for_msg}] expected symbol selector/key, got {v}"
        )))
    })
}

/// Handle the `send recv, selector, args...` IIR opcode (LANG20
/// §"IIR additions").
///
/// `srcs[0]` is the receiver; `srcs[1]` is the symbol-id register
/// holding the method selector; `srcs[2..]` are the user-supplied
/// arguments.  Allocates a fresh per-instruction `InlineCache` (PR
/// 7 makes it persistent) and calls
/// `LispyBinding::send_message`.  For Lispy this returns
/// `RuntimeError::NoSuchMethod`, which is the correct behaviour
/// for a language without method dispatch.
fn exec_send(instr: &IIRInstr, frame: &mut Frame) -> Result<(), RunError> {
    if instr.srcs.len() < 2 {
        return Err(RunError::MalformedInstruction(format!(
            "send expects at least 2 srcs (recv, selector), got {}",
            instr.srcs.len()
        )));
    }
    // DoS guard: cap the args allocation up-front.  Hand-built
    // malformed IIR could declare an instruction with millions of
    // srcs and OOM us via `Vec::with_capacity`.  Found by PR 6
    // security review.  The cap is defensive — well-formed IIR
    // never approaches it (function arities are bounded by source
    // syntax; a Twig function can't have 65k arguments).
    if instr.srcs.len() > MAX_REGISTERS_PER_FRAME {
        return Err(RunError::MalformedInstruction(format!(
            "send: srcs.len()={} exceeds MAX_REGISTERS_PER_FRAME ({MAX_REGISTERS_PER_FRAME})",
            instr.srcs.len()
        )));
    }
    let receiver = operand_to_value(&instr.srcs[0], &|n| frame.get(n))
        .map_err(RunError::OperandConversion)?;
    let selector = read_symbol_arg(&instr.srcs[1], frame, "send", 1)?;

    let mut args: Vec<LispyValue> = Vec::with_capacity(instr.srcs.len() - 2);
    for src in &instr.srcs[2..] {
        let v = operand_to_value(src, &|n| frame.get(n))
            .map_err(RunError::OperandConversion)?;
        args.push(v);
    }

    // Stack-allocated IC (LispyICEntry: Copy + small array of
    // entries — no heap touch).  TODO(LANG20 PR 4-or-later):
    // `DispatchCx::new_for_test` is the only constructor today;
    // replace with the production constructor when the type grows
    // real fields.
    let mut ic: InlineCache<LispyICEntry> = InlineCache::new();
    let mut cx = DispatchCx::<LispyBinding>::new_for_test();
    let result = LispyBinding::send_message(receiver, selector, &args, &mut ic, &mut cx)
        .map_err(RunError::Runtime)?;

    if let Some(d) = &instr.dest {
        frame.set(d.clone(), result)?;
    }
    Ok(())
}

/// Handle the `load_property obj, key` IIR opcode.
///
/// `srcs[0]` is the object; `srcs[1]` is the symbol-id register
/// holding the property key.  Allocates a fresh per-instruction
/// `InlineCache` and calls `LispyBinding::load_property`.  For
/// Lispy this returns `RuntimeError::NoSuchProperty`.
fn exec_load_property(instr: &IIRInstr, frame: &mut Frame) -> Result<(), RunError> {
    if instr.srcs.len() != 2 {
        return Err(RunError::MalformedInstruction(format!(
            "load_property expects 2 srcs (obj, key), got {}",
            instr.srcs.len()
        )));
    }
    let obj = operand_to_value(&instr.srcs[0], &|n| frame.get(n))
        .map_err(RunError::OperandConversion)?;
    let key = read_symbol_arg(&instr.srcs[1], frame, "load_property", 1)?;

    let mut ic: InlineCache<LispyICEntry> = InlineCache::new();
    let result = LispyBinding::load_property(obj, key, &mut ic)
        .map_err(RunError::Runtime)?;

    if let Some(d) = &instr.dest {
        frame.set(d.clone(), result)?;
    }
    Ok(())
}

/// Handle the `store_property obj, key, value` IIR opcode.
///
/// `srcs[0]` is the object; `srcs[1]` is the symbol-id register
/// holding the property key; `srcs[2]` is the value to write.
/// No dest (`store_property` returns void; the IIR compiler
/// emits this as a side-effecting instruction).
fn exec_store_property(instr: &IIRInstr, frame: &mut Frame) -> Result<(), RunError> {
    if instr.srcs.len() != 3 {
        return Err(RunError::MalformedInstruction(format!(
            "store_property expects 3 srcs (obj, key, value), got {}",
            instr.srcs.len()
        )));
    }
    let obj = operand_to_value(&instr.srcs[0], &|n| frame.get(n))
        .map_err(RunError::OperandConversion)?;
    let key = read_symbol_arg(&instr.srcs[1], frame, "store_property", 1)?;
    let value = operand_to_value(&instr.srcs[2], &|n| frame.get(n))
        .map_err(RunError::OperandConversion)?;

    let mut ic: InlineCache<LispyICEntry> = InlineCache::new();
    LispyBinding::store_property(obj, key, value, &mut ic).map_err(RunError::Runtime)?;
    // store_property has no dest — the result is the side-effect.
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

    // ─────────────────────────────────────────────────────────────────
    // PR 5: closures, top-level value defines, quoted symbols
    // ─────────────────────────────────────────────────────────────────

    // ── Quoted symbols ──────────────────────────────────────────────

    #[test]
    fn quoted_symbol_round_trips() {
        // 'foo evaluates to a symbol value with name "foo".
        let v = run_source("'foo").unwrap();
        let sym_id = v.as_symbol().expect("expected symbol value");
        let name = lispy_runtime::name_of(sym_id).unwrap();
        assert_eq!(name, "foo");
    }

    #[test]
    fn symbol_p_recognises_quoted_symbol() {
        // (symbol? 'bar) is #t.
        let v = run_source("(symbol? 'bar)").unwrap();
        assert_eq!(v, LispyValue::TRUE);
    }

    // ── Anonymous lambdas + apply ───────────────────────────────────

    #[test]
    fn anonymous_lambda_no_capture() {
        // ((lambda (x) (* x x)) 5)  → 25
        assert_eq!(
            run_source("((lambda (x) (* x x)) 5)").unwrap().as_int(),
            Some(25),
        );
    }

    #[test]
    fn anonymous_lambda_two_params() {
        // ((lambda (x y) (+ x y)) 3 4)  → 7
        assert_eq!(
            run_source("((lambda (x y) (+ x y)) 3 4)").unwrap().as_int(),
            Some(7),
        );
    }

    #[test]
    fn lambda_captures_enclosing_let() {
        // (let ((x 10)) ((lambda (y) (+ x y)) 5))  → 15
        let src = "(let ((x 10)) ((lambda (y) (+ x y)) 5))";
        assert_eq!(run_source(src).unwrap().as_int(), Some(15));
    }

    #[test]
    fn lambda_captures_multiple_values() {
        // Captures both x and y.
        let src = "(let ((x 1) (y 2)) ((lambda (z) (+ x (+ y z))) 4))";
        assert_eq!(run_source(src).unwrap().as_int(), Some(7));
    }

    #[test]
    fn nested_lambdas() {
        // Curried add: ((lambda (x) (lambda (y) (+ x y))) 3) returns
        // a closure that adds 3 to its argument.  Apply that to 4.
        let src = "(((lambda (x) (lambda (y) (+ x y))) 3) 4)";
        assert_eq!(run_source(src).unwrap().as_int(), Some(7));
    }

    // ── Higher-order via builtin closure ────────────────────────────

    #[test]
    fn higher_order_passing_user_fn() {
        // (define (apply-it f x y) (f x y))
        // (apply-it (lambda (a b) (* a b)) 6 7)  → 42
        let src = "
            (define (apply-it f x y) (f x y))
            (apply-it (lambda (a b) (* a b)) 6 7)
        ";
        assert_eq!(run_source(src).unwrap().as_int(), Some(42));
    }

    #[test]
    fn higher_order_passing_builtin() {
        // Pass `+` itself as a value.  Twig wraps it in
        // `make_builtin_closure`; apply_closure routes through
        // resolve_builtin.
        let src = "
            (define (apply-it f x y) (f x y))
            (apply-it + 2 3)
        ";
        assert_eq!(run_source(src).unwrap().as_int(), Some(5));
    }

    // ── Top-level value defines ─────────────────────────────────────

    #[test]
    fn top_level_value_define_then_use() {
        // (define x 42) x  → 42
        assert_eq!(run_source("(define x 42) x").unwrap().as_int(), Some(42));
    }

    #[test]
    fn top_level_value_define_used_in_function() {
        // (define base 100)
        // (define (bump n) (+ base n))
        // (bump 5)  → 105
        let src = "
            (define base 100)
            (define (bump n) (+ base n))
            (bump 5)
        ";
        assert_eq!(run_source(src).unwrap().as_int(), Some(105));
    }

    #[test]
    fn top_level_value_define_overwrites() {
        // Last write wins.
        assert_eq!(
            run_source("(define x 1) (define x 99) x").unwrap().as_int(),
            Some(99),
        );
    }

    // ── Closure-returning functions ─────────────────────────────────

    #[test]
    fn function_returning_closure() {
        // Make-adder pattern.  Tests that a closure returned from
        // a function still has working captures after the maker
        // returns (Box::leak ensures correctness).
        let src = "
            (define (make-adder x) (lambda (y) (+ x y)))
            ((make-adder 10) 5)
        ";
        assert_eq!(run_source(src).unwrap().as_int(), Some(15));
    }

    // ── Error paths ─────────────────────────────────────────────────

    #[test]
    fn apply_closure_on_non_closure_errors() {
        // Hand-craft a module that calls apply_closure on an int.
        // The IR compiler can't generate this directly (it always
        // wraps in make_closure first), but we test the dispatcher
        // refuses the malformed input.
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        let main = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: 4,
            instructions: vec![
                IIRInstr::new(
                    "const",
                    Some("x".into()),
                    vec![Operand::Int(7)],
                    "any",
                ),
                IIRInstr::new(
                    "call_builtin",
                    Some("r".into()),
                    vec![
                        Operand::Var("apply_closure".into()),
                        Operand::Var("x".into()),
                    ],
                    "any",
                ),
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
        assert!(matches!(err, RunError::NotCallable(_)));
    }

    #[test]
    fn global_get_undefined_errors() {
        // Hand-craft a module that calls global_get on a name
        // that was never set.  The IR compiler doesn't emit this
        // directly (compile_var_ref errors at compile time on
        // unbound names), but the dispatcher must refuse.
        use interpreter_ir::function::{FunctionTypeStatus, IIRFunction};
        let main = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: 4,
            instructions: vec![
                IIRInstr::new(
                    "const",
                    Some("name".into()),
                    vec![Operand::Var("ghost".into())],
                    "any",
                ),
                IIRInstr::new(
                    "call_builtin",
                    Some("r".into()),
                    vec![
                        Operand::Var("global_get".into()),
                        Operand::Var("name".into()),
                    ],
                    "any",
                ),
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
        match err {
            RunError::UndefinedGlobal(n) => assert_eq!(n, "ghost"),
            other => panic!("expected UndefinedGlobal, got {other:?}"),
        }
    }

    // ── Globals struct sanity ───────────────────────────────────────

    #[test]
    fn globals_set_and_get_round_trip() {
        let mut g = Globals::new();
        assert!(g.is_empty());
        let id = lispy_runtime::intern("foo");
        g.set(id, LispyValue::int(7));
        assert_eq!(g.len(), 1);
        assert_eq!(g.get(id), Some(LispyValue::int(7)));
        assert_eq!(g.get(lispy_runtime::intern("bar")), None);
    }

    #[test]
    fn run_with_globals_threads_the_table() {
        // The Twig frontend rejects unbound name references at
        // compile time, so a program that touches a pre-seeded
        // global isn't expressible from source.  Instead we verify
        // that `run_with_globals` accepts a non-empty seed table,
        // runs a program that doesn't interfere with it, and
        // leaves the table intact afterwards (so a future
        // multi-run TwigVM can reuse it).
        let mut g = Globals::new();
        g.set(lispy_runtime::intern("seed"), LispyValue::int(99));
        let module = compile_source("(+ 1 2)", "test").unwrap();
        let v = run_with_globals(&module, &mut g).unwrap();
        assert_eq!(v.as_int(), Some(3));
        // Pre-seeded value is still there after the run.
        assert_eq!(g.get(lispy_runtime::intern("seed")), Some(LispyValue::int(99)));
    }

    #[test]
    fn defines_then_run_with_seeded_globals_writes_more() {
        // (define x 5) (+ x 10) → 15.  Verifies that global_set
        // works when the dispatcher receives a non-empty initial
        // globals table, and that the new entry coexists with
        // pre-seeded ones.
        let mut g = Globals::new();
        g.set(lispy_runtime::intern("seed"), LispyValue::int(99));
        let module = compile_source("(define x 5) (+ x 10)", "test").unwrap();
        let v = run_with_globals(&module, &mut g).unwrap();
        assert_eq!(v.as_int(), Some(15));
        // Both the seed and the program-defined x are present.
        assert_eq!(g.get(lispy_runtime::intern("seed")), Some(LispyValue::int(99)));
        assert_eq!(g.get(lispy_runtime::intern("x")), Some(LispyValue::int(5)));
    }

    // ─────────────────────────────────────────────────────────────────
    // PR 6: send / load_property / store_property opcodes
    // ─────────────────────────────────────────────────────────────────
    //
    // `twig-ir-compiler` doesn't yet emit these opcodes — Twig source
    // syntax has no `(send obj msg ...)` form.  These tests therefore
    // hand-build a minimal IIRModule using each opcode and run it
    // through the dispatcher.  For Lispy, the binding methods return
    // `NoSuchMethod` / `NoSuchProperty`, which is the correct
    // behaviour for a language without method dispatch — PR 6's value
    // is wiring the OPCODES through the trait machinery so a future
    // Ruby/JS binding gets dispatch for free.

    use interpreter_ir::function::{FunctionTypeStatus, IIRFunction as F};

    /// Build a minimal module with `main` containing `instrs` plus a
    /// trailing `(ret nil)`.  Convenient for the hand-built opcode
    /// tests below.
    fn module_with_main(instrs: Vec<IIRInstr>, register_count: usize) -> IIRModule {
        let main = F {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
            register_count,
            instructions: instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        };
        IIRModule {
            name: "test".into(),
            functions: vec![main],
            entry_point: Some("main".into()),
            language: "twig".into(),
        }
    }

    // ── send ────────────────────────────────────────────────────────

    #[test]
    fn send_on_lispy_value_returns_no_such_method() {
        // Hand-build:
        //   r0 = const 42                    (receiver)
        //   sel = const "any-method"         (interned as symbol)
        //   r1 = send r0 sel                 (no extra args)
        //   ret r1
        let instrs = vec![
            IIRInstr::new("const", Some("r0".into()), vec![Operand::Int(42)], "any"),
            IIRInstr::new("const", Some("sel".into()), vec![Operand::Var("any-method".into())], "any"),
            IIRInstr::new(
                "send",
                Some("r1".into()),
                vec![Operand::Var("r0".into()), Operand::Var("sel".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r1".into())], "any"),
        ];
        let module = module_with_main(instrs, 8);
        let err = run(&module).unwrap_err();
        // Lispy correctly refuses send on any value — the runtime
        // surface for a language without method dispatch.
        match err {
            RunError::Runtime(RuntimeError::NoSuchMethod { selector }) => {
                assert_eq!(
                    lispy_runtime::name_of(selector).as_deref(),
                    Some("any-method"),
                );
            }
            other => panic!("expected Runtime(NoSuchMethod), got {other:?}"),
        }
    }

    #[test]
    fn send_with_args_routes_through_binding() {
        // send recv sel arg1 arg2 — verifies the args slice is
        // forwarded.  Lispy still rejects, but the rejection
        // includes the named selector — which means args were
        // correctly assembled and the binding's send_message was
        // invoked.
        let instrs = vec![
            IIRInstr::new("const", Some("recv".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new("const", Some("sel".into()), vec![Operand::Var("plus".into())], "any"),
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(2)], "any"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(3)], "any"),
            IIRInstr::new(
                "send",
                Some("r".into()),
                vec![
                    Operand::Var("recv".into()),
                    Operand::Var("sel".into()),
                    Operand::Var("a".into()),
                    Operand::Var("b".into()),
                ],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let module = module_with_main(instrs, 8);
        let err = run(&module).unwrap_err();
        assert!(matches!(
            err,
            RunError::Runtime(RuntimeError::NoSuchMethod { .. })
        ));
    }

    #[test]
    fn send_missing_selector_errors_malformed() {
        // send with only 1 src (just the receiver) — selector is missing.
        let instrs = vec![
            IIRInstr::new("const", Some("recv".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new(
                "send",
                Some("r".into()),
                vec![Operand::Var("recv".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let module = module_with_main(instrs, 4);
        let err = run(&module).unwrap_err();
        assert!(matches!(err, RunError::MalformedInstruction(s) if s.contains("send")));
    }

    #[test]
    fn send_non_symbol_selector_errors_typed() {
        // Selector must be a symbol; passing an int is a type error.
        let instrs = vec![
            IIRInstr::new("const", Some("recv".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new("const", Some("sel".into()), vec![Operand::Int(7)], "any"),
            IIRInstr::new(
                "send",
                Some("r".into()),
                vec![Operand::Var("recv".into()), Operand::Var("sel".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let module = module_with_main(instrs, 4);
        let err = run(&module).unwrap_err();
        match err {
            RunError::Runtime(RuntimeError::TypeError(msg)) => {
                assert!(msg.contains("symbol"));
            }
            other => panic!("expected Runtime(TypeError(symbol)), got {other:?}"),
        }
    }

    // ── load_property ───────────────────────────────────────────────

    #[test]
    fn load_property_on_lispy_value_returns_no_such_property() {
        let instrs = vec![
            IIRInstr::new("const", Some("obj".into()), vec![Operand::Int(42)], "any"),
            IIRInstr::new("const", Some("k".into()), vec![Operand::Var("name".into())], "any"),
            IIRInstr::new(
                "load_property",
                Some("r".into()),
                vec![Operand::Var("obj".into()), Operand::Var("k".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let module = module_with_main(instrs, 4);
        let err = run(&module).unwrap_err();
        match err {
            RunError::Runtime(RuntimeError::NoSuchProperty { key }) => {
                assert_eq!(lispy_runtime::name_of(key).as_deref(), Some("name"));
            }
            other => panic!("expected Runtime(NoSuchProperty), got {other:?}"),
        }
    }

    #[test]
    fn load_property_wrong_arity_errors() {
        let instrs = vec![
            IIRInstr::new("const", Some("obj".into()), vec![Operand::Int(0)], "any"),
            // Missing key.
            IIRInstr::new(
                "load_property",
                Some("r".into()),
                vec![Operand::Var("obj".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let module = module_with_main(instrs, 4);
        let err = run(&module).unwrap_err();
        assert!(matches!(
            err,
            RunError::MalformedInstruction(s) if s.contains("load_property")
        ));
    }

    // ── store_property ──────────────────────────────────────────────

    #[test]
    fn store_property_on_lispy_value_returns_no_such_property() {
        let instrs = vec![
            IIRInstr::new("const", Some("obj".into()), vec![Operand::Int(42)], "any"),
            IIRInstr::new("const", Some("k".into()), vec![Operand::Var("count".into())], "any"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "any"),
            IIRInstr::new(
                "store_property",
                None, // no dest — store_property is side-effecting
                vec![
                    Operand::Var("obj".into()),
                    Operand::Var("k".into()),
                    Operand::Var("v".into()),
                ],
                "void",
            ),
            // store_property has no result; return nil after.
            IIRInstr::new(
                "call_builtin",
                Some("nil_v".into()),
                vec![Operand::Var("make_nil".into())],
                "any",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("nil_v".into())], "any"),
        ];
        let module = module_with_main(instrs, 8);
        let err = run(&module).unwrap_err();
        match err {
            RunError::Runtime(RuntimeError::NoSuchProperty { key }) => {
                assert_eq!(lispy_runtime::name_of(key).as_deref(), Some("count"));
            }
            other => panic!("expected Runtime(NoSuchProperty), got {other:?}"),
        }
    }

    #[test]
    fn store_property_wrong_arity_errors() {
        // 2 srcs (obj, key) — missing value.
        let instrs = vec![
            IIRInstr::new("const", Some("obj".into()), vec![Operand::Int(0)], "any"),
            IIRInstr::new("const", Some("k".into()), vec![Operand::Var("x".into())], "any"),
            IIRInstr::new(
                "store_property",
                None,
                vec![Operand::Var("obj".into()), Operand::Var("k".into())],
                "void",
            ),
            IIRInstr::new("ret", None, vec![Operand::Int(0)], "any"),
        ];
        let module = module_with_main(instrs, 4);
        let err = run(&module).unwrap_err();
        assert!(matches!(
            err,
            RunError::MalformedInstruction(s) if s.contains("store_property")
        ));
    }

    // ── Selector / key validation shared between send / load / store ─

    #[test]
    fn send_with_too_many_srcs_caps_at_max_registers() {
        // DoS guard: a hand-built `send` with srcs.len() >
        // MAX_REGISTERS_PER_FRAME should be refused before
        // allocating the args Vec.  Found by PR 6 security review.
        let mut srcs: Vec<Operand> = Vec::with_capacity(MAX_REGISTERS_PER_FRAME + 2);
        srcs.push(Operand::Var("recv".into()));
        srcs.push(Operand::Var("sel".into()));
        for _ in 0..MAX_REGISTERS_PER_FRAME {
            srcs.push(Operand::Int(0));
        }
        // The instruction itself is too large to embed in a
        // realistic test program, so we skip the const-prelude
        // and exercise the bounds check directly via a malformed
        // module — the dispatcher should reject before reading
        // srcs[0].
        let instrs = vec![IIRInstr::new("send", Some("r".into()), srcs, "any")];
        let module = module_with_main(instrs, 4);
        let err = run(&module).unwrap_err();
        assert!(matches!(
            err,
            RunError::MalformedInstruction(s) if s.contains("MAX_REGISTERS_PER_FRAME")
        ));
    }

    #[test]
    fn store_property_non_symbol_key_errors_typed() {
        let instrs = vec![
            IIRInstr::new("const", Some("obj".into()), vec![Operand::Int(0)], "any"),
            // Key is an int, not a symbol.
            IIRInstr::new("const", Some("k".into()), vec![Operand::Int(7)], "any"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(99)], "any"),
            IIRInstr::new(
                "store_property",
                None,
                vec![
                    Operand::Var("obj".into()),
                    Operand::Var("k".into()),
                    Operand::Var("v".into()),
                ],
                "void",
            ),
            IIRInstr::new("ret", None, vec![Operand::Int(0)], "any"),
        ];
        let module = module_with_main(instrs, 4);
        let err = run(&module).unwrap_err();
        match err {
            RunError::Runtime(RuntimeError::TypeError(msg)) => {
                assert!(msg.contains("store_property"));
                assert!(msg.contains("symbol"));
            }
            other => panic!("expected Runtime(TypeError(...symbol...)), got {other:?}"),
        }
    }
}
