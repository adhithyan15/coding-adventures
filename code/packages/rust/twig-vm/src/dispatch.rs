//! # `dispatch` — the real interpreter dispatch loop for `twig-vm`.
//!
//! Originally LANG20 PR 4 (tree-walking dispatcher); extended in
//! PR 5 to cover closures, top-level value defines, and quoted
//! symbols; extended in PR 6 to cover the method-dispatch
//! opcodes `send`, `load_property`, `store_property`; extended in
//! PR 7 with **persistent inline-cache slots** so a hot
//! `load_property` site shares one IC instance across all its
//! activations (the V8-style IC machinery the JIT will speculate
//! on).  Runs an entire `IIRModule` end to end and returns a
//! `LispyValue`.
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
//! ### IC slot lookup (PR 7)
//!
//! When the IIR compiler assigns `IIRInstr::ic_slot = Some(slot)`
//! to an IC-owning instruction, the dispatcher routes through
//! the **persistent [`ICTable`]** — a per-function vector of
//! [`InlineCache<LispyICEntry>`] indexed by slot id.  Two
//! activations of the same function hit the same IC instance,
//! so a `load_property` that observes a class on call N can
//! benefit on call N+1 (the V8-style fast-path the JIT
//! eventually compiles against).
//!
//! When `ic_slot` is `None` (the PR 6 default), the dispatcher
//! stack-allocates a fresh IC per dispatch — backwards-
//! compatible with hand-built IIRModules in tests that don't
//! assign slots.  New IC-owning frontends (Ruby, JS) emit slot
//! ids; existing tests keep working.
//!
//! For Lispy specifically the binding's `send_message` etc. don't
//! consult the IC (they error immediately), so the IC table
//! mostly verifies plumbing.  The cache fills the moment a Ruby-
//! or JS-binding's `send_message` calls
//! `InlineCache::record(...)`.
//!
//! Out of scope (later PRs):
//!
//! - **Profiler that reads IC observations** — PR 8 wires the
//!   `vm-core` profiler that dumps `.ldp` artefacts (LANG22) from
//!   IC contents.
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

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand, SlotState};
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

/// Maximum inline-cache slots any single function can declare
/// (LANG20 PR 7).
///
/// `IIRInstr::ic_slot: Option<u32>` permits values up to
/// `u32::MAX`; `ICTable::get_or_alloc` calls
/// `Vec::resize_with(slot + 1, …)` on first access, so a
/// hand-built or malformed IIR with `ic_slot = Some(u32::MAX -
/// 1)` would attempt a multi-hundred-GB allocation and OOM-abort
/// the process.  Capping at 2¹⁶ matches the
/// [`MAX_REGISTERS_PER_FRAME`] convention — a real-world Twig
/// function won't approach it (most have <10 IC sites; the
/// largest realistic case is <100).  Found by PR 7 security
/// review.
pub const MAX_IC_SLOTS_PER_FUNCTION: u32 = 1 << 16;

/// Maximum number of distinct functions an [`ICTable`] will
/// hold (LANG20 PR 7).
///
/// Bounds the outer HashMap's growth so a hand-built module
/// declaring millions of functions can't unboundedly grow the
/// per-process IC storage.  Matches the
/// [`MAX_IC_SLOTS_PER_FUNCTION`] cap; combined the two limit
/// total IC storage to ~2³² entries × ~64 bytes = ~256 GB
/// which still permits every reasonable program but stops
/// adversarial inputs at the boundary.  Found by PR 7
/// security review.
pub const MAX_IC_FUNCTIONS: usize = 1 << 16;

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
///
/// `pub(crate)` so the [`crate::debug::FrameView`] wrapper can borrow
/// it.  All callers outside `dispatch` go through `FrameView`'s narrow
/// API rather than touching the registers HashMap directly.
#[derive(Debug)]
pub(crate) struct Frame {
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

    /// All register names live in the frame.  Used by the debug bridge.
    pub(crate) fn register_names(&self) -> Vec<String> {
        self.registers.keys().cloned().collect()
    }

    /// Debug-printable rendering of a register's current value.
    ///
    /// Returns `None` if the register is not bound.  Used by the debug
    /// hook to honour DAP's `variables` request.
    pub(crate) fn debug_print(&self, name: &str) -> Option<String> {
        self.registers.get(name).map(|v| format!("{v:?}"))
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
// IC table (PR 7)
// ---------------------------------------------------------------------------

/// Persistent inline-cache storage indexed by `(function_name,
/// ic_slot)`.
///
/// One [`InlineCache<LispyICEntry>`] per IC-owning instruction
/// (`send`, `load_property`, `store_property`).  Two activations
/// of the same function hit the **same IC instance**, so a hot
/// site that observed a class on call N benefits on call N+1
/// (the V8-style fast path the JIT eventually compiles
/// against).
///
/// **Storage shape.**  `HashMap<String, Vec<InlineCache<...>>>`.
/// The outer key is the function name (the IIR carries names,
/// not numeric ids); the inner Vec is dense — slot N occupies
/// index N.  Functions without IC-owning instructions never get
/// an entry.
///
/// **Lifetime.**  Per-run today (one [`ICTable`] per call to
/// [`run`]); PR 8+ moves it to per-VM state when `LangVM` lands,
/// so the cache survives across multiple `run` calls.  The
/// public API stays the same.
///
/// **Why string-keyed instead of function-id-indexed?**  The IIR
/// already keys functions by name everywhere (`UnknownFunction`,
/// `module.functions.find`); using the same key avoids
/// introducing a separate identifier scheme.  When register
/// allocation refactors functions to numeric ids in a future PR,
/// this field migrates with them.
#[derive(Debug, Default)]
pub struct ICTable {
    by_function: HashMap<String, Vec<InlineCache<LispyICEntry>>>,
}

impl ICTable {
    /// Construct an empty IC table.
    pub fn new() -> Self {
        ICTable {
            by_function: HashMap::new(),
        }
    }

    /// Get a `&mut` reference to the IC at `(fn_name, slot)`,
    /// growing the per-function vector if needed.
    ///
    /// First access to a slot allocates a fresh
    /// [`InlineCache::new`] (state Uninit, zero entries, zero
    /// counters).  Subsequent accesses return the existing
    /// instance.  This is the API the dispatcher's IC-owning
    /// opcode handlers use on every dispatch.
    ///
    /// Errors if `slot >= MAX_IC_SLOTS_PER_FUNCTION` or the
    /// table already holds `MAX_IC_FUNCTIONS` distinct
    /// functions and `fn_name` is new.  Both caps are defensive
    /// against hand-built malformed IIR; well-formed input
    /// never approaches them.
    pub fn get_or_alloc(
        &mut self,
        fn_name: &str,
        slot: u32,
    ) -> Result<&mut InlineCache<LispyICEntry>, RunError> {
        if slot >= MAX_IC_SLOTS_PER_FUNCTION {
            return Err(RunError::MalformedInstruction(format!(
                "ic_slot {slot} exceeds MAX_IC_SLOTS_PER_FUNCTION ({MAX_IC_SLOTS_PER_FUNCTION})"
            )));
        }
        // Reject new function entries beyond MAX_IC_FUNCTIONS.
        // `entry().or_default()` would silently grow past the
        // cap; check membership first (cheap when the function
        // is already present, the common case).
        if !self.by_function.contains_key(fn_name)
            && self.by_function.len() >= MAX_IC_FUNCTIONS
        {
            return Err(RunError::MalformedInstruction(format!(
                "ICTable function count exceeds MAX_IC_FUNCTIONS ({MAX_IC_FUNCTIONS})"
            )));
        }
        let entry = self.by_function.entry(fn_name.to_string()).or_default();
        let slot_idx = slot as usize;
        if entry.len() <= slot_idx {
            entry.resize_with(slot_idx + 1, InlineCache::new);
        }
        Ok(&mut entry[slot_idx])
    }

    /// Read-only lookup.  Returns `None` if the slot has never
    /// been accessed (either the function has no IC entries or
    /// the slot is beyond the highest slot ever requested).
    /// Tests use this to verify the table mechanics.
    pub fn get(&self, fn_name: &str, slot: u32) -> Option<&InlineCache<LispyICEntry>> {
        self.by_function
            .get(fn_name)
            .and_then(|v| v.get(slot as usize))
    }

    /// Number of slots currently allocated for `fn_name`.
    /// Returns 0 for functions that have no IC entries.  Tests
    /// use this to verify dense allocation.
    pub fn slot_count(&self, fn_name: &str) -> usize {
        self.by_function
            .get(fn_name)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Total IC slots across all functions.  Mostly for testing
    /// + future profile-artefact dumps.
    pub fn total_slots(&self) -> usize {
        self.by_function.values().map(|v| v.len()).sum()
    }
}

// ---------------------------------------------------------------------------
// Profile table (PR 8)
// ---------------------------------------------------------------------------

/// Maximum number of distinct functions the profiler tracks.
///
/// Mirrors [`MAX_IC_FUNCTIONS`] — same defensive cap shape.
/// Hand-built malformed IIR with millions of unique function
/// names can't unboundedly grow the per-process profile state.
pub const MAX_PROFILED_FUNCTIONS: usize = 1 << 16;

/// Maximum number of `(function_name, instr_index)` slots the
/// profiler tracks across all functions.
///
/// `MAX_PROFILED_FUNCTIONS × max_instructions_per_function` would
/// be `2³² ≈ 4 billion` entries in the absolute worst case —
/// roughly 150 GB of HashMap.  In practice the per-run
/// instruction budget (`MAX_INSTRUCTIONS_PER_RUN = 2²⁰`) bounds a
/// single run's growth, but `ProfileTable` is designed to be
/// reused across multiple `run_with_profile` calls (the future
/// per-VM state pattern).  This cap ensures even a long-lived
/// table reused across many adversarial-IIR runs can't grow
/// without bound.
///
/// 2²⁰ = ~1M slots is plenty for realistic Twig programs (a
/// 1000-function program with 100 dest-producing instructions
/// per function uses 100K slots — 10% of the cap).  Found by
/// PR 8 security review (Medium #3).
pub const MAX_PROFILED_INSTRUCTION_SLOTS: usize = 1 << 20;

/// V8 Ignition-style profile data collected during dispatch.
///
/// The `IIRModule` is borrowed `&` by the dispatcher — multiple
/// frames see the same module on the recursion stack — so the
/// profiler can't mutate `IIRInstr::observed_slot` directly.
/// Instead, observations live in a side-table here, keyed by
/// `(function_name, instr_index)`.  Per-function call counts
/// live in their own map.
///
/// **Lifetime.**  Per-run today (one [`ProfileTable`] per call
/// to [`run`]); future PRs that introduce a long-lived `LangVM`
/// will move it there so the cache survives across runs.  The
/// public API stays the same.
///
/// **What feeds in.**
///
/// - On dispatch entry: increment `call_count` for `func.name`.
/// - On every instruction with a `dest` that produced a value:
///   classify the value via `LispyBinding::class_of` and call
///   `SlotState::record(class_str)`.
/// - On every IC consult: the binding's `note_hit` /
///   `note_miss` already runs through the persistent IC table
///   (PR 7); the profile table observes the *result* of those
///   consults via the same `class_of` pass on the dest.
///
/// The IC table tracks per-call-site cache hit/miss; this
/// table tracks per-instruction *result-type* observations and
/// per-function *warmth*.  Together they answer the V8-style
/// "is this site monomorphic, polymorphic, or megamorphic, and
/// is the function hot enough to JIT?" question — the data
/// feed for LANG22's `.ldp` profile artefact format.
///
/// **What's NOT here in PR 8.**  The `.ldp` binary serialiser
/// (LANG22 PR 11d) and the JIT promotion threshold (LANG22
/// PR 11f) consume this table.  PR 8 just collects.
#[derive(Debug, Default)]
pub struct ProfileTable {
    /// Function name → call count.  Incremented once per call to
    /// `dispatch()` for that function.  `u64` so even very long
    /// production runs don't overflow.
    call_counts: HashMap<String, u64>,
    /// `(function_name, instr_index)` → SlotState.  Sparse:
    /// only instructions that produced an observable result
    /// have entries.
    instruction_slots: HashMap<(String, usize), SlotState>,
}

impl ProfileTable {
    /// Construct an empty profile table.
    pub fn new() -> Self {
        ProfileTable {
            call_counts: HashMap::new(),
            instruction_slots: HashMap::new(),
        }
    }

    /// Increment the per-function call count.  Returns the new
    /// value so JIT-promotion-threshold checks can branch on it
    /// inline (future PR).
    ///
    /// Errors if `fn_name` is new and the table already holds
    /// [`MAX_PROFILED_FUNCTIONS`] distinct names.  Re-incrementing
    /// existing functions always succeeds.
    pub fn note_call(&mut self, fn_name: &str) -> Result<u64, RunError> {
        if !self.call_counts.contains_key(fn_name)
            && self.call_counts.len() >= MAX_PROFILED_FUNCTIONS
        {
            return Err(RunError::MalformedInstruction(format!(
                "ProfileTable function count exceeds MAX_PROFILED_FUNCTIONS ({MAX_PROFILED_FUNCTIONS})"
            )));
        }
        let entry = self
            .call_counts
            .entry(fn_name.to_string())
            .and_modify(|n| *n = n.saturating_add(1))
            .or_insert(1);
        Ok(*entry)
    }

    /// Record a runtime observation for `(fn_name, instr_index)`.
    ///
    /// Advances the V8-style state machine on the slot:
    /// Uninitialized → Monomorphic → Polymorphic → Megamorphic.
    /// `class_str` is the language-defined type tag (here
    /// always one of "int", "nil", "bool", "symbol", "cons",
    /// "closure" — the LispyClass kinds).
    ///
    /// Two caps are enforced:
    ///
    /// - If `fn_name` is new and the table already holds
    ///   [`MAX_PROFILED_FUNCTIONS`] distinct names → reject.
    /// - If the `(fn_name, instr_index)` slot is new and the
    ///   table already holds [`MAX_PROFILED_INSTRUCTION_SLOTS`]
    ///   slots → reject.  The second cap is the load-bearing
    ///   one for `ProfileTable`s reused across many runs.
    pub fn note_observation(
        &mut self,
        fn_name: &str,
        instr_index: usize,
        class_str: &str,
    ) -> Result<(), RunError> {
        if !self.call_counts.contains_key(fn_name)
            && self.call_counts.len() >= MAX_PROFILED_FUNCTIONS
        {
            return Err(RunError::MalformedInstruction(format!(
                "ProfileTable observation for new function exceeds MAX_PROFILED_FUNCTIONS ({MAX_PROFILED_FUNCTIONS})"
            )));
        }
        let key = (fn_name.to_string(), instr_index);
        if !self.instruction_slots.contains_key(&key)
            && self.instruction_slots.len() >= MAX_PROFILED_INSTRUCTION_SLOTS
        {
            return Err(RunError::MalformedInstruction(format!(
                "ProfileTable instruction-slot count exceeds MAX_PROFILED_INSTRUCTION_SLOTS ({MAX_PROFILED_INSTRUCTION_SLOTS})"
            )));
        }
        let slot = self.instruction_slots.entry(key).or_insert_with(SlotState::new);
        slot.record(class_str);
        Ok(())
    }

    /// Read-only access to a function's call count.  Returns 0
    /// for never-called functions.
    pub fn call_count(&self, fn_name: &str) -> u64 {
        self.call_counts.get(fn_name).copied().unwrap_or(0)
    }

    /// Read-only access to a single instruction's observation
    /// slot.  Returns `None` if the instruction has never produced
    /// an observation (e.g. it's a control-flow opcode, or
    /// hasn't executed yet).
    pub fn observed_slot(
        &self,
        fn_name: &str,
        instr_index: usize,
    ) -> Option<&SlotState> {
        self.instruction_slots
            .get(&(fn_name.to_string(), instr_index))
    }

    /// Number of distinct functions tracked.  Mostly for tests.
    pub fn function_count(&self) -> usize {
        self.call_counts.len()
    }

    /// Number of per-instruction slots tracked.  Mostly for tests.
    pub fn instruction_slot_count(&self) -> usize {
        self.instruction_slots.len()
    }
}

/// Map a [`LispyValue`] to the canonical class-name string used
/// by the profiler's [`SlotState::record`] and the future
/// `.ldp` serialiser.  `None` for values whose class can't be
/// determined (shouldn't happen in well-formed dispatch — every
/// `LispyValue` has a class).
fn lispy_class_str(value: LispyValue) -> Option<&'static str> {
    use lispy_runtime::LispyClass;
    match LispyBinding::class_of(value)? {
        LispyClass::Int => Some("int"),
        LispyClass::Nil => Some("nil"),
        LispyClass::Bool => Some("bool"),
        LispyClass::Symbol => Some("symbol"),
        LispyClass::Cons => Some("cons"),
        LispyClass::Closure => Some("closure"),
    }
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
    let mut ic_table = ICTable::new();
    let mut profile = ProfileTable::new();
    run_with_profile(module, &mut globals, &mut ic_table, &mut profile)
}

/// Run an `IIRModule` against a caller-supplied globals table.
///
/// Equivalent to [`run_with_profile`] with fresh IC + profile
/// tables.  Retained as a public entry point for backward
/// compatibility (PR 5 callers); new callers that want IC or
/// profile inspection should use [`run_with_profile`].
pub fn run_with_globals(
    module: &IIRModule,
    globals: &mut Globals,
) -> Result<LispyValue, RunError> {
    let mut ic_table = ICTable::new();
    let mut profile = ProfileTable::new();
    run_with_profile(module, globals, &mut ic_table, &mut profile)
}

/// Run an `IIRModule` against caller-supplied globals + IC table.
///
/// Equivalent to [`run_with_profile`] with a fresh profile table.
/// Retained as a public entry point for PR 7 callers that don't
/// need profile observations.
pub fn run_with_state(
    module: &IIRModule,
    globals: &mut Globals,
    ic_table: &mut ICTable,
) -> Result<LispyValue, RunError> {
    let mut profile = ProfileTable::new();
    run_with_profile(module, globals, ic_table, &mut profile)
}

/// Run an `IIRModule` against caller-supplied globals, IC table,
/// and profile table.
///
/// The profile table accumulates per-function call counts and
/// per-instruction type observations across the run.  Tests pass
/// an external `ProfileTable` to inspect observations after the
/// run; the JIT promotion threshold (LANG22 PR 11f) reads this
/// to decide which functions are hot enough to specialise.
///
/// This is the most-flexible entry point.  All three tables
/// persist for the duration of the call — pass the same tables
/// to a subsequent `run_with_profile` to accumulate across
/// multiple runs (the future per-VM state pattern).
pub fn run_with_profile(
    module: &IIRModule,
    globals: &mut Globals,
    ic_table: &mut ICTable,
    profile: &mut ProfileTable,
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
    let mut debug: Option<&mut dyn crate::debug::DebugHooks> = None;
    dispatch(module, entry, &[], 0, &mut budget, globals, ic_table, profile, &mut debug)
}

/// Run a module under a debug hook.
///
/// The hook is invoked at every safepoint (between IIR instructions at
/// every recursion depth).  Production callers wire this up to a
/// [`crate::debug_server::DebugServer`] for DAP support; tests can
/// supply any [`crate::debug::DebugHooks`] impl.
///
/// `globals`, `ic_table`, `profile` follow the same lifetime contract as
/// [`run_with_profile`] — fresh each call unless the caller is
/// accumulating state across runs.
pub fn run_with_debug(
    module: &IIRModule,
    globals: &mut Globals,
    ic_table: &mut ICTable,
    profile: &mut ProfileTable,
    debug: &mut dyn crate::debug::DebugHooks,
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
    let mut debug_opt: Option<&mut dyn crate::debug::DebugHooks> = Some(debug);
    dispatch(module, entry, &[], 0, &mut budget, globals, ic_table, profile, &mut debug_opt)
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
    ic_table: &mut ICTable,
    profile: &mut ProfileTable,
    debug: &mut Option<&mut dyn crate::debug::DebugHooks>,
) -> Result<LispyValue, RunError> {
    if depth > MAX_DISPATCH_DEPTH {
        return Err(RunError::DepthExceeded);
    }

    // PR 8: increment per-function call count once per
    // dispatch entry.  This is the JIT-promotion-threshold
    // signal — `LANG20 §"Tier interaction matrix"` says
    // Untyped functions promote at call_count > 100.
    profile.note_call(&func.name)?;

    let mut frame = Frame::new(func, args)?;
    let labels = build_label_index(func)?;

    let mut pc = 0;
    while pc < func.instructions.len() {
        // Debug safepoint — between every instruction.  The hook may
        // process incoming wire commands, detect breakpoints, and
        // block waiting for a resume.  Cost when no debugger is
        // attached: one `Option::is_some` branch per instruction.
        if let Some(d) = debug.as_deref_mut() {
            let view = crate::debug::FrameView::new(&frame);
            d.before_instruction(&func.name, depth, pc, &view);
        }

        budget.tick()?;
        let instr = &func.instructions[pc];
        // Capture before the match — `pc` may be updated to an
        // arbitrary label index by jmp / jmp_if_false, so we
        // need the just-executed instruction's index recorded
        // up-front for the post-match profile-recording site.
        let instr_pc = pc;

        match instr.op.as_str() {
            "const" => {
                exec_const(instr, &mut frame)?;
                pc += 1;
            }
            "call_builtin" => {
                exec_call_builtin(module, instr, &mut frame, depth, budget, globals, ic_table, profile, debug)?;
                pc += 1;
            }
            "call" => {
                exec_call(module, instr, &mut frame, depth, budget, globals, ic_table, profile, debug)?;
                pc += 1;
            }
            "send" => {
                exec_send(instr, &mut frame, ic_table, &func.name)?;
                pc += 1;
            }
            "load_property" => {
                exec_load_property(instr, &mut frame, ic_table, &func.name)?;
                pc += 1;
            }
            "store_property" => {
                exec_store_property(instr, &mut frame, ic_table, &func.name)?;
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

        // PR 8: profile the just-completed instruction.
        //
        // We observe the *result* of every dest-producing opcode —
        // that's the "what type does this expression evaluate
        // to" signal V8-style speculation needs.  Control-flow
        // opcodes (jmp/label/ret) and side-effecting ones
        // (store_property) have no dest; the `if let Some(dest)`
        // guard short-circuits and nothing is recorded.
        //
        // **Why the just-completed instruction is `instr_pc` here?**
        // The match arms above either incremented `pc` by 1
        // (most cases) or set `pc` to a label index (jmp /
        // jmp_if_false).  But the only opcodes that produce a
        // dest are the `pc += 1` ones; those are the only paths
        // that reach this branch.  `instr_pc` captured before
        // the match holds the just-executed instruction's index
        // unambiguously, so the recording site doesn't need to
        // reason about post-match `pc` semantics.  Cleanup
        // recommended by PR 8 security review (Low #2).
        if let Some(dest) = &instr.dest {
            if let Some(value) = frame.get(dest) {
                if let Some(class_str) = lispy_class_str(value) {
                    profile.note_observation(&func.name, instr_pc, class_str)?;
                }
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
    ic_table: &mut ICTable,
    profile: &mut ProfileTable,
    debug: &mut Option<&mut dyn crate::debug::DebugHooks>,
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
            return exec_apply_closure(module, instr, frame, depth, budget, globals, ic_table, profile, debug);
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
    ic_table: &mut ICTable,
    profile: &mut ProfileTable,
    debug: &mut Option<&mut dyn crate::debug::DebugHooks>,
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

    let result = dispatch(module, callee, &call_args, depth + 1, budget, globals, ic_table, profile, debug)?;
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
    ic_table: &mut ICTable,
    profile: &mut ProfileTable,
    debug: &mut Option<&mut dyn crate::debug::DebugHooks>,
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
        dispatch(module, callee, &all_args, depth + 1, budget, globals, ic_table, profile, debug)?
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
fn exec_send(
    instr: &IIRInstr,
    frame: &mut Frame,
    ic_table: &mut ICTable,
    fn_name: &str,
) -> Result<(), RunError> {
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

    // PR 7: route IC through the persistent table when a slot is
    // assigned; fall back to a stack-allocated fresh IC otherwise.
    // TODO(LANG20 PR 8): `DispatchCx::new_for_test` is the only
    // constructor today; replace with the production constructor
    // when the type grows real fields.
    let mut local_ic = InlineCache::<LispyICEntry>::new();
    let mut cx = DispatchCx::<LispyBinding>::new_for_test();
    let result = match instr.ic_slot {
        Some(slot) => {
            let ic = ic_table.get_or_alloc(fn_name, slot)?;
            LispyBinding::send_message(receiver, selector, &args, ic, &mut cx)
        }
        None => {
            LispyBinding::send_message(receiver, selector, &args, &mut local_ic, &mut cx)
        }
    }
    .map_err(RunError::Runtime)?;

    if let Some(d) = &instr.dest {
        frame.set(d.clone(), result)?;
    }
    Ok(())
}

/// Handle the `load_property obj, key` IIR opcode.
///
/// `srcs[0]` is the object; `srcs[1]` is the symbol-id register
/// holding the property key.  Routes through the persistent IC
/// table when `ic_slot` is assigned (PR 7); falls back to a
/// stack-allocated fresh IC otherwise (PR 6 backward compat).
/// For Lispy returns `RuntimeError::NoSuchProperty`.
fn exec_load_property(
    instr: &IIRInstr,
    frame: &mut Frame,
    ic_table: &mut ICTable,
    fn_name: &str,
) -> Result<(), RunError> {
    if instr.srcs.len() != 2 {
        return Err(RunError::MalformedInstruction(format!(
            "load_property expects 2 srcs (obj, key), got {}",
            instr.srcs.len()
        )));
    }
    let obj = operand_to_value(&instr.srcs[0], &|n| frame.get(n))
        .map_err(RunError::OperandConversion)?;
    let key = read_symbol_arg(&instr.srcs[1], frame, "load_property", 1)?;

    let mut local_ic = InlineCache::<LispyICEntry>::new();
    let result = match instr.ic_slot {
        Some(slot) => {
            let ic = ic_table.get_or_alloc(fn_name, slot)?;
            LispyBinding::load_property(obj, key, ic)
        }
        None => LispyBinding::load_property(obj, key, &mut local_ic),
    }
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
/// emits this as a side-effecting instruction).  Routes through
/// the persistent IC table when `ic_slot` is assigned.
fn exec_store_property(
    instr: &IIRInstr,
    frame: &mut Frame,
    ic_table: &mut ICTable,
    fn_name: &str,
) -> Result<(), RunError> {
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

    let mut local_ic = InlineCache::<LispyICEntry>::new();
    match instr.ic_slot {
        Some(slot) => {
            let ic = ic_table.get_or_alloc(fn_name, slot)?;
            LispyBinding::store_property(obj, key, value, ic)
        }
        None => LispyBinding::store_property(obj, key, value, &mut local_ic),
    }
    .map_err(RunError::Runtime)?;
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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
            param_refinements: Vec::new(),
            return_refinement: None,
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

    // ─────────────────────────────────────────────────────────────────
    // PR 7: persistent IC slot machinery
    // ─────────────────────────────────────────────────────────────────
    //
    // The PR 6 path stack-allocated a fresh IC per dispatch.  PR 7
    // adds the persistent table indexed by `IIRInstr::ic_slot` so a
    // hot site shares one IC instance across activations — the V8-
    // style fast path the JIT eventually compiles against.  Tests
    // verify:
    //
    //   - ICTable mechanics in isolation (alloc, get, slot count)
    //   - The dispatcher routes through the table when ic_slot is
    //     Some(slot)
    //   - Backward compat: ic_slot=None falls through to a fresh
    //     stack IC (PR 6 behaviour) and the table stays empty
    //   - The same instruction's IC is reused across two activations
    //     of the same function (the hot-site invariant)

    // ── ICTable in isolation ─────────────────────────────────────────

    #[test]
    fn ic_table_starts_empty() {
        let t = ICTable::new();
        assert_eq!(t.total_slots(), 0);
        assert_eq!(t.slot_count("anything"), 0);
        assert!(t.get("anything", 0).is_none());
    }

    #[test]
    fn ic_table_allocates_on_first_access() {
        let mut t = ICTable::new();
        t.get_or_alloc("f", 0).unwrap();
        assert_eq!(t.slot_count("f"), 1);
        assert_eq!(t.total_slots(), 1);
        assert!(t.get("f", 0).is_some());
    }

    #[test]
    fn ic_table_is_dense_per_function() {
        // Accessing slot 5 directly grows the function's vec to
        // 6 entries (0..=5); the gap entries are pre-allocated
        // empty ICs.  Tests the resize-with logic.
        let mut t = ICTable::new();
        t.get_or_alloc("f", 5).unwrap();
        assert_eq!(t.slot_count("f"), 6);
        for slot in 0..6 {
            assert!(
                t.get("f", slot).is_some(),
                "slot {slot} should be allocated by gap-filling",
            );
        }
    }

    #[test]
    fn ic_table_separate_functions_dont_share() {
        let mut t = ICTable::new();
        t.get_or_alloc("f", 0).unwrap();
        t.get_or_alloc("g", 0).unwrap();
        assert_eq!(t.slot_count("f"), 1);
        assert_eq!(t.slot_count("g"), 1);
        // Mutating one IC must not affect the other.
        t.get_or_alloc("f", 0).unwrap().note_hit();
        assert_eq!(t.get("f", 0).unwrap().hit_count(), 1);
        assert_eq!(t.get("g", 0).unwrap().hit_count(), 0);
    }

    #[test]
    fn ic_table_repeated_access_returns_same_instance() {
        // Same IC across multiple `get_or_alloc` calls — verify
        // by mutating once and reading the count back.
        let mut t = ICTable::new();
        t.get_or_alloc("f", 0).unwrap().note_hit();
        t.get_or_alloc("f", 0).unwrap().note_hit();
        t.get_or_alloc("f", 0).unwrap().note_hit();
        assert_eq!(t.get("f", 0).unwrap().hit_count(), 3);
    }

    #[test]
    fn ic_table_rejects_slot_at_max() {
        // PR 7 security review (HIGH #1): ic_slot >= MAX_IC_SLOTS_PER_FUNCTION
        // is rejected instead of attempting a multi-GB
        // resize_with.  Test exactly the boundary.
        let mut t = ICTable::new();
        let err = t.get_or_alloc("f", MAX_IC_SLOTS_PER_FUNCTION).unwrap_err();
        assert!(matches!(
            err,
            RunError::MalformedInstruction(s) if s.contains("MAX_IC_SLOTS_PER_FUNCTION")
        ));
        // The cap is exclusive — slot MAX-1 still works.
        t.get_or_alloc("f", MAX_IC_SLOTS_PER_FUNCTION - 1).unwrap();
    }

    #[test]
    fn ic_table_rejects_too_many_functions() {
        // PR 7 security review (MEDIUM #2): adding the
        // MAX_IC_FUNCTIONS+1th distinct function name fails.
        // We don't actually allocate 65k entries (slow); just
        // synthesize the invariant by populating up to the cap.
        let mut t = ICTable::new();
        for i in 0..MAX_IC_FUNCTIONS {
            t.get_or_alloc(&format!("f{i}"), 0).unwrap();
        }
        assert_eq!(t.by_function.len(), MAX_IC_FUNCTIONS);
        // The next distinct name is rejected.
        let err = t.get_or_alloc("overflow", 0).unwrap_err();
        assert!(matches!(
            err,
            RunError::MalformedInstruction(s) if s.contains("MAX_IC_FUNCTIONS")
        ));
        // But re-accessing an existing function still works (cap
        // is on distinct names, not on accesses).
        t.get_or_alloc("f0", 1).unwrap();
    }

    // ── Dispatcher routes through the table ──────────────────────────

    #[test]
    fn send_with_ic_slot_uses_table() {
        // send instr with ic_slot=Some(0).  After running, the
        // table should have a slot allocated for the current
        // function.  For Lispy the binding errors before
        // touching the IC, so hit/miss counts stay 0 — but the
        // slot existence proves the table was consulted.
        let instrs = vec![
            IIRInstr::new("const", Some("recv".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new("const", Some("sel".into()), vec![Operand::Var("m".into())], "any"),
            IIRInstr::new(
                "send",
                Some("r".into()),
                vec![Operand::Var("recv".into()), Operand::Var("sel".into())],
                "any",
            )
            .with_ic_slot(0),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let module = module_with_main(instrs, 4);
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let err = run_with_state(&module, &mut globals, &mut ic_table).unwrap_err();
        // The send still surfaces NoSuchMethod for Lispy.
        assert!(matches!(
            err,
            RunError::Runtime(RuntimeError::NoSuchMethod { .. })
        ));
        // Slot 0 was allocated on the `main` function.
        assert_eq!(ic_table.slot_count("main"), 1);
        assert!(ic_table.get("main", 0).is_some());
    }

    #[test]
    fn load_property_without_ic_slot_skips_table() {
        // Backward compat: ic_slot=None falls through to a
        // stack-allocated IC.  The persistent table stays empty.
        let instrs = vec![
            IIRInstr::new("const", Some("obj".into()), vec![Operand::Int(0)], "any"),
            IIRInstr::new("const", Some("k".into()), vec![Operand::Var("x".into())], "any"),
            IIRInstr::new(
                "load_property",
                Some("r".into()),
                vec![Operand::Var("obj".into()), Operand::Var("k".into())],
                "any",
            ), // no with_ic_slot — defaults to None
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let module = module_with_main(instrs, 4);
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let _ = run_with_state(&module, &mut globals, &mut ic_table).unwrap_err();
        assert_eq!(ic_table.total_slots(), 0);
    }

    #[test]
    fn store_property_with_ic_slot_uses_table() {
        let instrs = vec![
            IIRInstr::new("const", Some("obj".into()), vec![Operand::Int(0)], "any"),
            IIRInstr::new("const", Some("k".into()), vec![Operand::Var("x".into())], "any"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "any"),
            IIRInstr::new(
                "store_property",
                None,
                vec![
                    Operand::Var("obj".into()),
                    Operand::Var("k".into()),
                    Operand::Var("v".into()),
                ],
                "void",
            )
            .with_ic_slot(2), // arbitrary slot id
            IIRInstr::new("ret", None, vec![Operand::Int(0)], "any"),
        ];
        let module = module_with_main(instrs, 8);
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let _ = run_with_state(&module, &mut globals, &mut ic_table).unwrap_err();
        // Slot 2 access pre-fills 0, 1, 2.
        assert_eq!(ic_table.slot_count("main"), 3);
    }

    #[test]
    fn ic_persists_across_two_calls_to_same_function() {
        // Build a function `do-it` containing a single
        // `load_property` with ic_slot=0.  Call it twice from
        // `main`.  The IC for slot 0 in `do-it` should be the
        // SAME instance both times — verifiable via hit count
        // accumulation through note_hit() in the table itself
        // (we manually note_hit between calls to simulate what
        // a real binding's send_message would do).
        let do_it_instrs = vec![
            IIRInstr::new("const", Some("k".into()), vec![Operand::Var("x".into())], "any"),
            IIRInstr::new(
                "load_property",
                Some("r".into()),
                vec![Operand::Var("o".into()), Operand::Var("k".into())],
                "any",
            )
            .with_ic_slot(0),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "any"),
        ];
        let do_it = F {
            name: "do-it".into(),
            params: vec![("o".into(), "any".into())],
            return_type: "any".into(),
            register_count: 4,
            instructions: do_it_instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
            param_refinements: Vec::new(),
            return_refinement: None,
        };
        let main_instrs = vec![
            IIRInstr::new("const", Some("o1".into()), vec![Operand::Int(1)], "any"),
            IIRInstr::new("call", Some("_unused".into()), vec![
                Operand::Var("do-it".into()), Operand::Var("o1".into()),
            ], "any"),
            IIRInstr::new("ret", None, vec![Operand::Int(0)], "any"),
        ];
        let main = F {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
            register_count: 4,
            instructions: main_instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
            param_refinements: Vec::new(),
            return_refinement: None,
        };
        let module = IIRModule {
            name: "test".into(),
            functions: vec![do_it, main],
            entry_point: Some("main".into()),
            language: "twig".into(),
        };

        // First call.  load_property errors with NoSuchProperty
        // (since Lispy doesn't have properties), but the IC slot
        // for "do-it" gets allocated.
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let err = run_with_state(&module, &mut globals, &mut ic_table).unwrap_err();
        assert!(matches!(
            err,
            RunError::Runtime(RuntimeError::NoSuchProperty { .. })
        ));
        assert_eq!(ic_table.slot_count("do-it"), 1);

        // Manually note a hit on the slot to simulate what a
        // real binding's load_property would do on a cache hit.
        ic_table.get_or_alloc("do-it", 0).unwrap().note_hit();
        assert_eq!(ic_table.get("do-it", 0).unwrap().hit_count(), 1);

        // Second call (re-using the same ic_table).  The same
        // IC instance is consulted — its hit count is still 1
        // even though the dispatcher's IC access for this call
        // didn't note a hit (Lispy errors before that path).
        // The persistence is the test: the previous note_hit
        // wasn't lost.
        let _ = run_with_state(&module, &mut globals, &mut ic_table);
        assert_eq!(
            ic_table.get("do-it", 0).unwrap().hit_count(),
            1,
            "IC instance must persist across calls (the V8-style hot-site invariant)",
        );

        // Slot count unchanged — second call doesn't grow the table.
        assert_eq!(ic_table.slot_count("do-it"), 1);
    }

    // ── ic_slot field round-trip on IIRInstr ─────────────────────────

    #[test]
    fn ic_slot_default_none() {
        let i = IIRInstr::new("send", None, vec![], "any");
        assert_eq!(i.ic_slot, None);
    }

    #[test]
    fn ic_slot_builder_sets_field() {
        let i = IIRInstr::new("send", None, vec![], "any").with_ic_slot(42);
        assert_eq!(i.ic_slot, Some(42));
    }

    // ─────────────────────────────────────────────────────────────────
    // PR 8: profiler (per-function call_count + per-instr SlotState)
    // ─────────────────────────────────────────────────────────────────

    use interpreter_ir::SlotKind;

    // ── ProfileTable mechanics ──────────────────────────────────────

    #[test]
    fn profile_table_starts_empty() {
        let p = ProfileTable::new();
        assert_eq!(p.function_count(), 0);
        assert_eq!(p.instruction_slot_count(), 0);
        assert_eq!(p.call_count("anything"), 0);
        assert!(p.observed_slot("anything", 0).is_none());
    }

    #[test]
    fn profile_table_note_call_increments() {
        let mut p = ProfileTable::new();
        assert_eq!(p.note_call("f").unwrap(), 1);
        assert_eq!(p.note_call("f").unwrap(), 2);
        assert_eq!(p.note_call("f").unwrap(), 3);
        assert_eq!(p.call_count("f"), 3);
        assert_eq!(p.call_count("g"), 0);
    }

    #[test]
    fn profile_table_note_observation_advances_slot() {
        let mut p = ProfileTable::new();
        // First observation: slot becomes monomorphic on "int".
        p.note_observation("f", 0, "int").unwrap();
        let slot = p.observed_slot("f", 0).unwrap();
        assert_eq!(slot.kind, SlotKind::Monomorphic);
        assert_eq!(slot.count, 1);

        // Same type again: still mono, count goes up.
        p.note_observation("f", 0, "int").unwrap();
        let slot = p.observed_slot("f", 0).unwrap();
        assert_eq!(slot.kind, SlotKind::Monomorphic);
        assert_eq!(slot.count, 2);

        // Distinct second type: poly.
        p.note_observation("f", 0, "bool").unwrap();
        let slot = p.observed_slot("f", 0).unwrap();
        assert_eq!(slot.kind, SlotKind::Polymorphic);
    }

    #[test]
    fn profile_table_separate_keys_dont_share() {
        let mut p = ProfileTable::new();
        p.note_observation("f", 0, "int").unwrap();
        p.note_observation("f", 1, "bool").unwrap();
        p.note_observation("g", 0, "cons").unwrap();
        assert_eq!(p.observed_slot("f", 0).unwrap().observations, vec!["int"]);
        assert_eq!(p.observed_slot("f", 1).unwrap().observations, vec!["bool"]);
        assert_eq!(p.observed_slot("g", 0).unwrap().observations, vec!["cons"]);
        assert_eq!(p.instruction_slot_count(), 3);
    }

    #[test]
    fn profile_table_rejects_too_many_functions() {
        let mut p = ProfileTable::new();
        for i in 0..MAX_PROFILED_FUNCTIONS {
            p.note_call(&format!("f{i}")).unwrap();
        }
        assert_eq!(p.function_count(), MAX_PROFILED_FUNCTIONS);
        let err = p.note_call("overflow").unwrap_err();
        assert!(matches!(
            err,
            RunError::MalformedInstruction(s) if s.contains("MAX_PROFILED_FUNCTIONS")
        ));
        // Existing functions still increment.
        p.note_call("f0").unwrap();
    }

    #[test]
    fn profile_table_rejects_too_many_instruction_slots() {
        // PR 8 security review (Medium #3): cap on the per-instr
        // slot map.  This prevents long-lived ProfileTables
        // reused across many runs from growing without bound.
        let mut p = ProfileTable::new();
        // Use a single function name to hit the slot cap quickly.
        p.note_call("f").unwrap();
        for slot in 0..MAX_PROFILED_INSTRUCTION_SLOTS {
            p.note_observation("f", slot, "int").unwrap();
        }
        assert_eq!(p.instruction_slot_count(), MAX_PROFILED_INSTRUCTION_SLOTS);
        // Next distinct slot is rejected.
        let err = p.note_observation("f", MAX_PROFILED_INSTRUCTION_SLOTS, "int").unwrap_err();
        assert!(matches!(
            err,
            RunError::MalformedInstruction(s) if s.contains("MAX_PROFILED_INSTRUCTION_SLOTS")
        ));
        // Re-observing an existing slot still works (cap is on
        // distinct slots, not on observations).
        p.note_observation("f", 0, "bool").unwrap();
    }

    // ── Dispatcher records observations + call counts ────────────────

    #[test]
    fn dispatch_increments_call_count_for_main() {
        let module = compile_source("(+ 1 2)", "test").unwrap();
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let mut profile = ProfileTable::new();
        run_with_profile(&module, &mut globals, &mut ic_table, &mut profile).unwrap();
        // `main` is called exactly once per `run_with_profile`.
        assert_eq!(profile.call_count("main"), 1);
    }

    #[test]
    fn dispatch_call_counts_grow_with_recursion() {
        // (fact 5) recurses 6 times: fact(5), fact(4), fact(3),
        // fact(2), fact(1), fact(0).
        let src = "
            (define (fact n)
              (if (= n 0) 1 (* n (fact (- n 1)))))
            (fact 5)
        ";
        let module = compile_source(src, "test").unwrap();
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let mut profile = ProfileTable::new();
        run_with_profile(&module, &mut globals, &mut ic_table, &mut profile).unwrap();
        assert_eq!(profile.call_count("main"), 1);
        assert_eq!(profile.call_count("fact"), 6);
    }

    #[test]
    fn dispatch_records_int_observations_for_arithmetic() {
        // (+ 1 2) — main has these instructions:
        //   const _n1 = 1
        //   const _n2 = 2
        //   call_builtin "+" _n1 _n2 -> _r3
        //   ret _r3
        // After one run, the call_builtin instr should have a
        // monomorphic int observation.
        let module = compile_source("(+ 1 2)", "test").unwrap();
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let mut profile = ProfileTable::new();
        run_with_profile(&module, &mut globals, &mut ic_table, &mut profile).unwrap();

        // Find a call_builtin instr with dest in main.  All three
        // dest-producing instructions should have observations.
        let main = module.functions.iter().find(|f| f.name == "main").unwrap();
        let mut int_observations = 0;
        for (i, instr) in main.instructions.iter().enumerate() {
            if instr.dest.is_some() {
                if let Some(slot) = profile.observed_slot("main", i) {
                    if slot.observations.iter().any(|s| s == "int") {
                        int_observations += 1;
                    }
                }
            }
        }
        assert!(
            int_observations >= 3,
            "expected ≥3 int observations (2 const + 1 add result), got {int_observations}",
        );
    }

    #[test]
    fn dispatch_records_polymorphic_observation_across_recursive_calls() {
        // fact(5) returns int every iteration, but the (if (= n 0) 1 ...)
        // branches to either `1` (int) or `(* n ...)` (also int) — so
        // the `_move` instructions inside `if` lowering should
        // observe int monomorphically.  This test mostly verifies
        // that the same slot accumulates observations across
        // recursive calls.
        let src = "
            (define (fact n)
              (if (= n 0) 1 (* n (fact (- n 1)))))
            (fact 5)
        ";
        let module = compile_source(src, "test").unwrap();
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let mut profile = ProfileTable::new();
        run_with_profile(&module, &mut globals, &mut ic_table, &mut profile).unwrap();

        // Find any instr in fact that produced multiple
        // observations.  The recursive call_builtin "*" should
        // have run once per non-base-case iteration (5 times).
        let fact = module.functions.iter().find(|f| f.name == "fact").unwrap();
        let max_observation_count = (0..fact.instructions.len())
            .filter_map(|i| profile.observed_slot("fact", i))
            .map(|slot| slot.count)
            .max()
            .unwrap_or(0);
        assert!(
            max_observation_count >= 5,
            "expected ≥5 observations on the most-hit instr, got {max_observation_count}",
        );
    }

    #[test]
    fn dispatch_does_not_observe_control_flow_opcodes() {
        // jmp / label / ret have no dest — no observation should
        // be recorded for those positions.
        let module = compile_source("(if (< 1 2) 100 200)", "test").unwrap();
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let mut profile = ProfileTable::new();
        run_with_profile(&module, &mut globals, &mut ic_table, &mut profile).unwrap();

        let main = module.functions.iter().find(|f| f.name == "main").unwrap();
        for (i, instr) in main.instructions.iter().enumerate() {
            // Control-flow ops have no dest; the profiler must
            // not have recorded an observation at this index.
            if matches!(instr.op.as_str(), "jmp" | "jmp_if_false" | "label" | "ret") {
                assert!(
                    profile.observed_slot("main", i).is_none(),
                    "{} at instr {i} should have no observation",
                    instr.op,
                );
            }
        }
    }

    // ── run/run_with_globals/run_with_state still work ──────────────

    #[test]
    fn run_creates_internal_profile_table_and_discards_it() {
        // Backward compat: callers that don't care about the
        // profile use `run` and get a freshly-allocated profile
        // that's dropped on return.
        assert_eq!(run(&compile_source("(+ 1 2)", "test").unwrap()).unwrap().as_int(), Some(3));
    }

    #[test]
    fn run_with_state_creates_internal_profile_table() {
        let module = compile_source("(+ 1 2)", "test").unwrap();
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        run_with_state(&module, &mut globals, &mut ic_table).unwrap();
        // Caller didn't ask for profile — IC table also stays empty
        // since no IC-owning instructions ran.
        assert_eq!(ic_table.total_slots(), 0);
    }

    #[test]
    fn profile_persists_across_two_calls_to_run_with_profile() {
        // Same `&mut ProfileTable` reused for two runs — call
        // counts and observations accumulate.  This is the
        // future per-VM state pattern (LANG22 §"Migration path").
        let module = compile_source("(+ 1 2)", "test").unwrap();
        let mut globals = Globals::new();
        let mut ic_table = ICTable::new();
        let mut profile = ProfileTable::new();
        run_with_profile(&module, &mut globals, &mut ic_table, &mut profile).unwrap();
        run_with_profile(&module, &mut globals, &mut ic_table, &mut profile).unwrap();
        run_with_profile(&module, &mut globals, &mut ic_table, &mut profile).unwrap();
        assert_eq!(profile.call_count("main"), 3);
    }
}
