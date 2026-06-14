//! [`VMProfiler`] — runtime type observation for InterpreterIR instructions.
//!
//! The profiler watches each value-producing instruction execute and fills in
//! the feedback slot on the `IIRInstr` object:
//!
//! - `observed_slot`   — V8 Ignition-style state machine (LANG17)
//! - `observed_type`   — legacy two-state view kept in sync
//! - `observation_count` — legacy integer counter
//!
//! # When profiling fires
//!
//! The profiler only records observations for instructions whose `type_hint`
//! is `"any"` — statically typed instructions already know their type and
//! don't need profiling.  Consequences:
//!
//! - A **fully-typed** program pays **zero** profiler overhead.
//! - A **partially-typed** program only profiles the `"any"` slots.
//! - A **fully-untyped** program observes every value-producing instruction.
//!
//! # Pluggable type mapping
//!
//! The default type mapper is [`default_type_mapper`], which follows the same
//! rules as the Python vm-core's `default_type_mapper`.  Language frontends
//! that need different type-name semantics can supply a custom closure via
//! [`VMProfiler::with_mapper`].
//!
//! # Example
//!
//! ```
//! use vm_core::profiler::VMProfiler;
//! use vm_core::value::Value;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//!
//! let mut profiler = VMProfiler::new();
//! let mut instr = IIRInstr::new("add", Some("v0".into()),
//!     vec![Operand::Var("a".into())], "any");
//!
//! profiler.observe(&mut instr, &Value::Int(42));
//! assert_eq!(instr.observation_count, 1);
//! assert_eq!(instr.observed_type.as_deref(), Some("u8"));
//! ```

use interpreter_ir::instr::IIRInstr;
use crate::value::Value;

/// Map a `Value` to the IIR type string the profiler should record.
///
/// This is the default type mapper, matching the Python vm-core semantics.
/// Language frontends can override this with their own closure via
/// [`VMProfiler::with_mapper`].
pub fn default_type_mapper(value: &Value) -> &'static str {
    value.iir_type_name()
}

/// Observes instruction results and annotates `IIRInstr` feedback slots.
///
/// Always-on: runs inline in the dispatch loop with constant overhead
/// (one string comparison, one mapper call, one state-machine advance).
///
/// Only instructions with `type_hint == "any"` are profiled.
pub struct VMProfiler {
    total_observations: u64,
    mapper: Box<dyn Fn(&Value) -> String + Send + Sync>,
}

impl VMProfiler {
    /// Create a new profiler with the default type mapper.
    pub fn new() -> Self {
        VMProfiler {
            total_observations: 0,
            mapper: Box::new(|v| default_type_mapper(v).to_string()),
        }
    }

    /// Create a profiler with a custom value-to-type mapper.
    ///
    /// The mapper closure maps a runtime `Value` to an IIR type string.
    /// It should return `"any"` for values whose type cannot be determined.
    pub fn with_mapper<F>(mapper: F) -> Self
    where
        F: Fn(&Value) -> String + Send + Sync + 'static,
    {
        VMProfiler {
            total_observations: 0,
            mapper: Box::new(mapper),
        }
    }

    /// Record the runtime type of `result` on `instr`.
    ///
    /// Called by the dispatch loop after each instruction that produces a value
    /// (`instr.dest.is_some()`).  Advances the V8-style state machine on
    /// `instr.observed_slot` and keeps the legacy `observed_type` /
    /// `observation_count` views in sync.
    ///
    /// A no-op when `instr.type_hint != "any"` — typed instructions are
    /// skipped so fully-typed programs pay zero profiling cost.
    pub fn observe(&mut self, instr: &mut IIRInstr, result: &Value) {
        if instr.type_hint != "any" {
            return;
        }
        let rt = (self.mapper)(result);
        instr.record_observation(&rt);
        self.total_observations += 1;
    }

    /// Total number of profiling events recorded this session.
    pub fn total_observations(&self) -> u64 {
        self.total_observations
    }
}

impl Default for VMProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for VMProfiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VMProfiler")
            .field("total_observations", &self.total_observations)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::instr::Operand;

    fn any_instr() -> IIRInstr {
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into())], "any")
    }

    fn typed_instr() -> IIRInstr {
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into())], "u8")
    }

    #[test]
    fn observe_any_instr_updates_slot() {
        let mut profiler = VMProfiler::new();
        let mut instr = any_instr();
        profiler.observe(&mut instr, &Value::Int(42));
        assert_eq!(instr.observation_count, 1);
        assert_eq!(instr.observed_type.as_deref(), Some("u8"));
        assert_eq!(profiler.total_observations(), 1);
    }

    #[test]
    fn typed_instr_skipped_by_profiler() {
        let mut profiler = VMProfiler::new();
        let mut instr = typed_instr();
        profiler.observe(&mut instr, &Value::Int(42));
        // Typed instructions are never profiled.
        assert_eq!(instr.observation_count, 0);
        assert_eq!(profiler.total_observations(), 0);
    }

    #[test]
    fn polymorphic_after_two_types() {
        let mut profiler = VMProfiler::new();
        let mut instr = any_instr();
        profiler.observe(&mut instr, &Value::Int(10));   // u8
        profiler.observe(&mut instr, &Value::Int(300));  // u16
        assert!(instr.is_polymorphic());
    }

    #[test]
    fn custom_mapper_used() {
        let mut profiler = VMProfiler::with_mapper(|_v| "custom_type".to_string());
        let mut instr = any_instr();
        profiler.observe(&mut instr, &Value::Int(1));
        assert_eq!(instr.observed_type.as_deref(), Some("custom_type"));
    }
}
