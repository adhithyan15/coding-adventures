//! `IIRInstr` — the single instruction of InterpreterIR.
//!
//! # Design
//!
//! Each instruction is a small record with:
//!
//! - `op`         — a mnemonic string identifying the operation (`"add"`, `"jmp"`, …)
//! - `dest`       — the SSA variable produced, or `None` for void ops
//! - `srcs`       — source operands: variable names or immediate literals
//! - `type_hint`  — the declared type (`"u8"`, `"bool"`, `"any"`, …)
//!
//! At runtime, extra fields are written by the `vm-core` profiler:
//!
//! - `observed_slot`      — a `SlotState` holding the V8 Ignition-style
//!                          feedback state machine (UNINIT → MONO → POLY → MEGA)
//! - `observed_type`      — the single observed type for `Monomorphic` slots,
//!                          `"polymorphic"` for POLY/MEGA, or `None` before any
//!                          observation
//! - `observation_count`  — total observations recorded
//!
//! A `deopt_anchor` marks the interpreter instruction index the JIT must revert
//! to if a type guard fails.
//!
//! # Example
//!
//! ```
//! use interpreter_ir::instr::{IIRInstr, Operand};
//!
//! // Statically typed (Tetrad)
//! let instr = IIRInstr::new("add", Some("v0".into()), vec![
//!     Operand::Var("a".into()),
//!     Operand::Var("b".into()),
//! ], "u8");
//!
//! // Dynamically typed (before profiling)
//! let dyn_instr = IIRInstr::new("add", Some("v0".into()), vec![
//!     Operand::Var("a".into()),
//!     Operand::Var("b".into()),
//! ], "any");
//! assert!(!dyn_instr.is_typed());
//! assert!(!dyn_instr.has_observation());
//! ```

use crate::opcodes::{is_concrete_type, POLYMORPHIC_TYPE};
use crate::slot_state::{SlotKind, SlotState};

// ---------------------------------------------------------------------------
// Operand
// ---------------------------------------------------------------------------

/// A source operand for an IIR instruction.
///
/// Operands are either a reference to a named variable (resolved at runtime
/// against the register file) or an immediate literal embedded directly in
/// the instruction.
///
/// Note: `bool` must be checked before `Int` at the source language level —
/// in Python `isinstance(True, int)` is True; in Rust the enum discriminant
/// makes this explicit.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// A reference to a named variable (looked up in the register file).
    Var(String),
    /// A signed integer immediate (covers all of u8/u16/u32/u64 via sign-extension).
    Int(i64),
    /// An IEEE 754 double-precision float immediate.
    Float(f64),
    /// A boolean immediate.
    Bool(bool),
}

impl Operand {
    /// Return the variable name if this is a `Var` operand, else `None`.
    pub fn as_var(&self) -> Option<&str> {
        match self {
            Operand::Var(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl std::fmt::Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Var(s)   => write!(f, "{s}"),
            Operand::Int(n)   => write!(f, "{n}"),
            Operand::Float(v) => write!(f, "{v}"),
            Operand::Bool(b)  => write!(f, "{b}"),
        }
    }
}

// ---------------------------------------------------------------------------
// IIRInstr
// ---------------------------------------------------------------------------

/// A single InterpreterIR instruction.
///
/// # Static fields
///
/// `op`, `dest`, `srcs`, `type_hint`, and `may_alloc` are set at compile time
/// by the language frontend and never changed after.
///
/// # Runtime fields
///
/// `observed_slot`, `observed_type`, `observation_count`, and `deopt_anchor`
/// are written by `vm-core` during execution.  They start `None`/`0` and
/// accumulate as the profiler samples the instruction.
#[derive(Debug, Clone)]
pub struct IIRInstr {
    // --- Static fields (set at compile time, never changed) ---

    /// Instruction mnemonic, e.g. `"add"`, `"jmp"`, `"ret"`.
    pub op: String,

    /// Name of the SSA variable produced, or `None` for void instructions
    /// (branches, stores, returns).
    pub dest: Option<String>,

    /// Source operands — each is either a variable name or an immediate.
    pub srcs: Vec<Operand>,

    /// Declared type of `dest`.  Use `"any"` for dynamically typed languages
    /// where the type is unknown at compile time.
    pub type_hint: String,

    /// LANG16 hint: this instruction may trigger a heap allocation.
    ///
    /// Set by language frontends for `alloc`, `box`, `safepoint`, or any
    /// `call` whose callee transitively allocates.  Defaults to `false`.
    pub may_alloc: bool,

    // --- Runtime profiling (populated by vm-core) ---

    /// V8 Ignition-style feedback slot (LANG17).
    ///
    /// `None` until the profiler samples this instruction for the first time;
    /// thereafter holds a `SlotState` that advances monotonically.
    pub observed_slot: Option<SlotState>,

    /// Legacy two-state type view kept in sync with `observed_slot`.
    ///
    /// `None` = not yet observed, `"u8"` etc. = monomorphic,
    /// `"polymorphic"` = polymorphic or megamorphic.
    pub observed_type: Option<String>,

    /// Number of times this instruction has been profiled by vm-core.
    pub observation_count: u32,

    /// Instruction index to resume interpreter at if a JIT type guard fails.
    ///
    /// Set by jit-core when emitting a type guard for this instruction.
    /// `None` means no guard has been emitted yet.
    pub deopt_anchor: Option<usize>,

    /// Inline-cache slot id for this instruction (LANG20 PR 7).
    ///
    /// `Some(slot)` for instructions that own an inline cache —
    /// `send`, `load_property`, `store_property`, and (eventually)
    /// the call_site / arith call sites that JITs warm.  `None`
    /// for instructions without ICs (arithmetic, control flow,
    /// `const`, …).
    ///
    /// Slot ids are assigned by the language frontend per
    /// IIRFunction (sequential 0, 1, 2, …); the runtime allocates
    /// IC storage at function-load time based on the highest
    /// assigned id.  See LANG20 §"Inline cache machinery".
    ///
    /// **Backward compatibility.**  IC-eligible instructions that
    /// leave this field `None` get a stack-allocated fresh IC per
    /// dispatch (the PR 6 behaviour).  This keeps hand-built
    /// IIRModules in tests working without re-emitting them with
    /// slot ids; new front-ends that want persistent ICs assign
    /// slots via [`IIRInstr::with_ic_slot`].
    pub ic_slot: Option<u32>,
}

impl IIRInstr {
    /// Create a new instruction with all profiling fields set to `None`/`0`.
    pub fn new(
        op: impl Into<String>,
        dest: Option<String>,
        srcs: Vec<Operand>,
        type_hint: impl Into<String>,
    ) -> Self {
        IIRInstr {
            op: op.into(),
            dest,
            srcs,
            type_hint: type_hint.into(),
            may_alloc: false,
            observed_slot: None,
            observed_type: None,
            observation_count: 0,
            deopt_anchor: None,
            ic_slot: None,
        }
    }

    /// Builder: assign an inline-cache slot id to this instruction.
    ///
    /// Used by language frontends that emit IC-owning opcodes
    /// (`send`, `load_property`, `store_property`).  The runtime
    /// then allocates persistent IC storage indexed by `slot`.
    ///
    /// Calling this on a non-IC-owning opcode is harmless — the
    /// slot is stored but the dispatcher only consults it for
    /// IC-eligible opcodes.
    pub fn with_ic_slot(mut self, slot: u32) -> Self {
        self.ic_slot = Some(slot);
        self
    }

    // ------------------------------------------------------------------
    // Static classification
    // ------------------------------------------------------------------

    /// Return `true` if this instruction has a concrete (non-dynamic) type
    /// hint — it is already typed at compile time.
    ///
    /// Concrete-type instructions are skipped by the profiler because their
    /// type is known; only `"any"` instructions are observed.
    pub fn is_typed(&self) -> bool {
        is_concrete_type(&self.type_hint)
    }

    /// Return `true` if the profiler has recorded at least one observation.
    pub fn has_observation(&self) -> bool {
        self.observation_count > 0
    }

    /// Return `true` if multiple types have been observed (do not specialise).
    pub fn is_polymorphic(&self) -> bool {
        self.observed_type.as_deref() == Some(POLYMORPHIC_TYPE)
    }

    /// The best available type: concrete hint first, then observed, then `"any"`.
    pub fn effective_type(&self) -> &str {
        if self.is_typed() {
            return &self.type_hint;
        }
        if let Some(t) = &self.observed_type {
            if t != POLYMORPHIC_TYPE {
                return t.as_str();
            }
        }
        "any"
    }

    // ------------------------------------------------------------------
    // Profiling
    // ------------------------------------------------------------------

    /// Update the observation slot with a new runtime type.
    ///
    /// Called by `vm-core`'s profiler after each execution of this instruction.
    /// Advances the V8 Ignition-style state machine on `observed_slot` and
    /// keeps the legacy `observed_type` / `observation_count` fields in sync.
    pub fn record_observation(&mut self, runtime_type: &str) {
        if self.observed_slot.is_none() {
            self.observed_slot = Some(SlotState::new());
        }
        let slot = self.observed_slot.as_mut().unwrap();
        slot.record(runtime_type);

        // Keep the legacy mirror fields in sync so existing callers that read
        // `observed_type` / `observation_count` keep working without
        // modification.
        self.observation_count = slot.count;
        match slot.kind {
            SlotKind::Monomorphic => {
                self.observed_type = slot.observations.first().cloned();
            }
            SlotKind::Polymorphic | SlotKind::Megamorphic => {
                self.observed_type = Some(POLYMORPHIC_TYPE.to_string());
            }
            SlotKind::Uninitialized => {
                // Unreachable: `record` always leaves the slot in one of the
                // three active kinds.  But Rust needs all arms covered.
            }
        }
    }
}

impl std::fmt::Display for IIRInstr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(dest) = &self.dest {
            write!(f, "{dest} = ")?;
        }
        write!(f, "{}(", self.op)?;
        for (i, src) in self.srcs.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{src}")?;
        }
        write!(f, ") : {}", self.type_hint)?;
        if self.observation_count > 0 {
            write!(
                f,
                " [obs={:?}×{}]",
                self.observed_type, self.observation_count
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_instr(type_hint: &str) -> IIRInstr {
        IIRInstr::new(
            "add",
            Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            type_hint,
        )
    }

    #[test]
    fn typed_instruction() {
        let instr = add_instr("u8");
        assert!(instr.is_typed());
        assert_eq!(instr.effective_type(), "u8");
    }

    #[test]
    fn untyped_instruction() {
        let instr = add_instr("any");
        assert!(!instr.is_typed());
        assert_eq!(instr.effective_type(), "any");
        assert!(!instr.has_observation());
    }

    #[test]
    fn record_observation_updates_legacy_fields() {
        let mut instr = add_instr("any");
        instr.record_observation("u8");
        assert_eq!(instr.observation_count, 1);
        assert_eq!(instr.observed_type.as_deref(), Some("u8"));
        assert!(!instr.is_polymorphic());
        assert_eq!(instr.effective_type(), "u8");
    }

    #[test]
    fn polymorphic_after_two_types() {
        let mut instr = add_instr("any");
        instr.record_observation("u8");
        instr.record_observation("u16");
        assert!(instr.is_polymorphic());
        assert_eq!(instr.effective_type(), "any");
    }

    #[test]
    fn operand_display() {
        assert_eq!(format!("{}", Operand::Var("x".into())), "x");
        assert_eq!(format!("{}", Operand::Int(42)), "42");
        assert_eq!(format!("{}", Operand::Bool(true)), "true");
    }
}
