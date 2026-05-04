//! [`VMFrame`] — per-call-frame state of the vm-core register interpreter.
//!
//! Each function call creates a fresh `VMFrame`.  The frame holds:
//!
//! - `fn_name`       — name of the function being executed (used to look up
//!                     the `IIRFunction` in the module on each dispatch tick)
//! - `ip`            — instruction pointer (index into the function's body)
//! - `registers`     — flat register file, one slot per named variable
//! - `name_to_reg`   — variable name → register index (built from params)
//! - `return_dest`   — register index in the **caller** frame where the return
//!                     value is stored (`None` for the root frame)
//!
//! # Why `fn_name` instead of `&IIRFunction`
//!
//! Storing a name string rather than a reference breaks the borrow conflict
//! that would otherwise arise in the dispatch loop: the loop needs
//! `&mut vm.frames` (to advance `ip`) AND `&mut vm.module` (to look up the
//! callee in `handle_call`) simultaneously.  Since they're both fields of
//! `VMCore`, holding live borrows of both through `vm` is rejected by the
//! borrow checker.  Using a name + module re-lookup on each tick is the
//! clean, Rust-idiomatic solution — the lookup is a linear scan over what
//! is typically a small slice (≤ 100 functions).
//!
//! # Example
//!
//! ```
//! use vm_core::frame::VMFrame;
//! use vm_core::value::Value;
//!
//! let mut frame = VMFrame::new("add", 8, None);
//! frame.assign("a", Value::Int(10));
//! frame.assign("b", Value::Int(20));
//! assert_eq!(frame.resolve("a"), Some(&Value::Int(10)));
//! ```

use std::collections::HashMap;
use crate::value::Value;

/// Per-call-frame state of the interpreter.
#[derive(Debug)]
pub struct VMFrame {
    /// Name of the `IIRFunction` being executed.
    pub fn_name: String,

    /// Instruction pointer — 0-based index into `fn_.instructions`.
    pub ip: usize,

    /// Flat register file: `registers[i]` holds the value of whichever
    /// variable maps to register index `i`.
    pub registers: Vec<Value>,

    /// Variable name → register index.
    ///
    /// Parameters occupy the first `params.len()` slots (assigned in order);
    /// subsequent assignments extend the map dynamically.
    pub name_to_reg: HashMap<String, usize>,

    /// Register index in the **caller** frame where the return value should
    /// be written when this frame's `ret` instruction fires.
    ///
    /// `None` for the root frame (no caller exists).
    pub return_dest: Option<usize>,
}

impl VMFrame {
    /// Create a blank frame for the named function with `register_count`
    /// pre-allocated slots, all initialised to `Value::Null`.
    pub fn new(fn_name: impl Into<String>, register_count: usize, return_dest: Option<usize>) -> Self {
        VMFrame {
            fn_name: fn_name.into(),
            ip: 0,
            registers: vec![Value::Null; register_count],
            name_to_reg: HashMap::new(),
            return_dest,
        }
    }

    /// Build a frame from a function's parameter list, mapping param names to
    /// the first N register slots in declaration order.
    pub fn for_function(
        fn_name: impl Into<String>,
        params: &[(String, String)],
        register_count: usize,
        return_dest: Option<usize>,
    ) -> Self {
        let mut frame = VMFrame::new(fn_name, register_count, return_dest);
        for (idx, (param_name, _)) in params.iter().enumerate() {
            frame.name_to_reg.insert(param_name.clone(), idx);
        }
        frame
    }

    // ------------------------------------------------------------------
    // Register access
    // ------------------------------------------------------------------

    /// Return the value of a named variable, or `None` if not in scope.
    pub fn resolve(&self, name: &str) -> Option<&Value> {
        let idx = *self.name_to_reg.get(name)?;
        self.registers.get(idx)
    }

    /// Maximum number of distinct named registers per frame.
    ///
    /// Guards against unbounded register-file growth from adversarial or
    /// malformed IIR that assigns to a unique variable name on every
    /// instruction.  Assignments beyond this limit are silently dropped (the
    /// variable resolves as `Null`) rather than allocating more heap.
    pub const MAX_FRAME_REGISTERS: usize = 65_536;

    /// Assign a value to a named variable, allocating a new register if needed.
    ///
    /// Returns silently if the frame register cap [`Self::MAX_FRAME_REGISTERS`]
    /// has been reached and `name` is a new variable.  This prevents unbounded
    /// heap growth from adversarial IIR programs that generate unique variable
    /// names in a loop.
    pub fn assign(&mut self, name: &str, value: Value) {
        let next_idx = self.name_to_reg.len();
        // Enforce register-file cap for new variables.
        if next_idx >= Self::MAX_FRAME_REGISTERS && !self.name_to_reg.contains_key(name) {
            return; // SECURITY: silently drop rather than grow without bound
        }
        let idx = *self.name_to_reg.entry(name.to_string()).or_insert(next_idx);
        // Grow register file if needed (shouldn't happen for well-formed IIR).
        if idx >= self.registers.len() {
            self.registers.resize(idx + 1, Value::Null);
        }
        self.registers[idx] = value;
    }

    /// Snapshot the current register values (used by the trace system).
    pub fn snapshot(&self) -> Vec<Value> {
        self.registers.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_frame_all_null() {
        let frame = VMFrame::new("main", 4, None);
        assert_eq!(frame.registers.len(), 4);
        assert!(frame.registers.iter().all(|v| *v == Value::Null));
    }

    #[test]
    fn for_function_maps_params() {
        let params = vec![
            ("a".to_string(), "u8".to_string()),
            ("b".to_string(), "u8".to_string()),
        ];
        let mut frame = VMFrame::for_function("add", &params, 8, None);
        frame.registers[0] = Value::Int(10);
        frame.registers[1] = Value::Int(20);
        assert_eq!(frame.resolve("a"), Some(&Value::Int(10)));
        assert_eq!(frame.resolve("b"), Some(&Value::Int(20)));
    }

    #[test]
    fn assign_creates_new_register() {
        let mut frame = VMFrame::new("f", 8, None);
        frame.assign("x", Value::Int(99));
        assert_eq!(frame.resolve("x"), Some(&Value::Int(99)));
    }

    #[test]
    fn assign_overwrites_existing() {
        let mut frame = VMFrame::new("f", 8, None);
        frame.assign("x", Value::Int(1));
        frame.assign("x", Value::Int(2));
        assert_eq!(frame.resolve("x"), Some(&Value::Int(2)));
    }

    #[test]
    fn resolve_undefined_returns_none() {
        let frame = VMFrame::new("f", 8, None);
        assert!(frame.resolve("missing").is_none());
    }
}
