//! [`VMFrame`] — per-call-frame state of the vm-core register interpreter.
//!
//! Each function call creates a fresh `VMFrame`.  The frame holds:
//!
//! - `fn_name`       — name of the function being executed (used to look up
//!   the `IIRFunction` in the module on each dispatch tick)
//! - `ip`            — instruction pointer (index into the function's body)
//! - `registers`     — flat register file, one slot per named variable
//! - `name_to_reg`   — variable name → register index (built from params)
//! - `return_dest`   — register index in the **caller** frame where the return
//!   value is stored (`None` for the root frame)
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
//! frame.assign("a", Value::Int(10)).unwrap();
//! frame.assign("b", Value::Int(20)).unwrap();
//! assert_eq!(frame.resolve("a"), Some(&Value::Int(10)));
//! ```
//!
//! # The register limit
//!
//! A frame holds at most [`VMFrame::MAX_FRAME_REGISTERS`] (2¹⁶ = 65,536)
//! distinct named registers.  This is a resource bound, not an array size —
//! the register file is a growable `Vec` — and it is the same 2¹⁶ the rest of
//! the stack already uses: `load_reg`/`store_reg` accept indices
//! `0..=65_535`, and twig-vm's `MAX_REGISTERS_PER_FRAME` is `1 << 16`.
//!
//! Exceeding it is an error, never a silent drop:
//!
//! ```text
//!   VMCore::execute(module, …)
//!     └─ check_register_limit(f) for every f   ← before any instruction runs
//!          over the limit?  → Err(RegisterLimitExceeded { function, registers, limit })
//!   dispatch: frame.assign(dest, v)            ← run-time backstop
//!          new name past the limit? → the same Err
//! ```
//!
//! The load-time check is the one a program normally meets.  The run-time
//! check only fires for a register no instruction `dest` names — one a
//! language opcode handler builds on the fly.
//!
//! A function's `register_count` is only a pre-allocation hint and is not
//! trusted: [`VMFrame::new`] clamps it to the limit, so a module declaring
//! `register_count = usize::MAX` cannot make the VM allocate (and abort)
//! before the first instruction.

use std::collections::{HashMap, HashSet};
use interpreter_ir::IIRFunction;
use crate::errors::VMError;
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
    ///
    /// `register_count` is a hint from the frontend, so it is clamped to
    /// [`Self::MAX_FRAME_REGISTERS`]: a larger value can only be a frontend
    /// over-reporting (the load-time check has already refused any function
    /// that really uses more), and trusting it would let a hand-built module
    /// with `register_count = usize::MAX` abort the process on allocation.
    /// The file still grows on demand in [`Self::assign`].
    pub fn new(fn_name: impl Into<String>, register_count: usize, return_dest: Option<usize>) -> Self {
        VMFrame {
            fn_name: fn_name.into(),
            ip: 0,
            registers: vec![Value::Null; register_count.min(Self::MAX_FRAME_REGISTERS)],
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
        // A function always needs at least one register slot per parameter: the
        // dispatcher places the call arguments directly at indices `0..params.len()`.
        // A frontend may under-report `register_count` (e.g. a hoisted `LAMBDA` body
        // whose only register is its parameter reports `0`), so size the register
        // file to cover the parameters regardless — mirroring how `assign` grows the
        // file for under-reported *locals*. Without this a lambda call indexes past
        // the end of `registers` and panics.
        let mut frame = VMFrame::new(fn_name, register_count.max(params.len()), return_dest);
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

    /// Maximum number of distinct named registers per frame (2¹⁶).
    ///
    /// A resource bound: it caps one frame's register file, and so the
    /// memory a deep call stack can pin (`max_frames` frames × this many
    /// registers each).  It matches the index range `load_reg`/`store_reg`
    /// accept (`0..=65_535`) and twig-vm's `MAX_REGISTERS_PER_FRAME`.
    ///
    /// A function over the limit is refused before it runs (see
    /// [`Self::check_register_limit`]); a new register past the limit at run
    /// time is the same [`VMError::RegisterLimitExceeded`].  Neither path
    /// drops the write and carries on.
    pub const MAX_FRAME_REGISTERS: usize = 1 << 16;

    /// Refuse a function that needs more registers than a frame can hold.
    ///
    /// The registers a function's frame can ever name are its parameters plus
    /// the `dest` of its instructions — every handler that creates a register
    /// does so through [`Self::assign`] or the `call` return slot, both keyed
    /// by an instruction's `dest`.  So the distinct names in that set are an
    /// exact, static count of the frame's named registers.
    ///
    /// Counting them needs a hash set, but almost every function is far below
    /// the limit, so a cheap upper bound comes first:
    ///
    /// ```text
    ///   params + (instructions with a dest)  ≤ limit  →  Ok, no allocation
    ///   otherwise                                    →  count distinct names
    /// ```
    ///
    /// Only a function that really could be over the limit pays for the set.
    ///
    /// # Errors
    ///
    /// [`VMError::RegisterLimitExceeded`] naming the function, its distinct
    /// register count and the limit.
    pub fn check_register_limit(function: &IIRFunction) -> Result<(), VMError> {
        let dests = function.instructions.iter().filter(|i| i.dest.is_some()).count();
        if function.params.len().saturating_add(dests) <= Self::MAX_FRAME_REGISTERS {
            return Ok(());
        }
        let names: HashSet<&str> = function
            .params
            .iter()
            .map(|(name, _)| name.as_str())
            .chain(function.instructions.iter().filter_map(|i| i.dest.as_deref()))
            .collect();
        if names.len() > Self::MAX_FRAME_REGISTERS {
            return Err(VMError::RegisterLimitExceeded {
                function: function.name.clone(),
                registers: names.len(),
                limit: Self::MAX_FRAME_REGISTERS,
            });
        }
        Ok(())
    }

    /// Return the register index for `name`, allocating the next free index
    /// if `name` is new to this frame.
    ///
    /// Shared by [`Self::assign`] and the `call` handler (which reserves the
    /// caller's return slot before the callee runs), so both obey the limit.
    ///
    /// # Errors
    ///
    /// [`VMError::RegisterLimitExceeded`] if `name` is new and the frame
    /// already holds [`Self::MAX_FRAME_REGISTERS`] registers.  The frame is
    /// left unchanged.
    pub fn register_index(&mut self, name: &str) -> Result<usize, VMError> {
        if let Some(&idx) = self.name_to_reg.get(name) {
            return Ok(idx);
        }
        let next_idx = self.name_to_reg.len();
        if next_idx >= Self::MAX_FRAME_REGISTERS {
            return Err(VMError::RegisterLimitExceeded {
                function: self.fn_name.clone(),
                registers: next_idx + 1,
                limit: Self::MAX_FRAME_REGISTERS,
            });
        }
        self.name_to_reg.insert(name.to_string(), next_idx);
        Ok(next_idx)
    }

    /// Assign a value to a named variable, allocating a new register if needed.
    ///
    /// # Errors
    ///
    /// [`VMError::RegisterLimitExceeded`] if `name` would be the frame's
    /// 65,537th register.  This used to return silently, dropping the value,
    /// so the program failed later with a misleading `undefined variable`.
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), VMError> {
        let idx = self.register_index(name)?;
        // Grow the register file if the frontend under-reported
        // `register_count` (or it was clamped in `new`).
        if idx >= self.registers.len() {
            self.registers.resize(idx + 1, Value::Null);
        }
        self.registers[idx] = value;
        Ok(())
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

    /// A frontend may under-report `register_count` for a function whose only
    /// registers are its parameters — a hoisted McCarthy `LAMBDA` body like
    /// `(LAMBDA (X) X)` reports `register_count = 0`. The register file must still
    /// be sized to hold the parameters, or the dispatcher's direct
    /// `registers[i] = arg` writes index past the end and panics (McCarthy W15b).
    #[test]
    fn for_function_sizes_registers_to_cover_params_when_count_underreports() {
        let params = vec![("x".to_string(), "any".to_string())];
        let mut frame = VMFrame::for_function("lambda_0", &params, 0, None);
        assert!(
            frame.registers.len() >= params.len(),
            "register file must cover the parameters even when register_count under-reports",
        );
        // The dispatcher writes the argument directly by index — must not panic.
        frame.registers[0] = Value::Int(5);
        assert_eq!(frame.resolve("x"), Some(&Value::Int(5)));
    }

    #[test]
    fn assign_creates_new_register() {
        let mut frame = VMFrame::new("f", 8, None);
        frame.assign("x", Value::Int(99)).unwrap();
        assert_eq!(frame.resolve("x"), Some(&Value::Int(99)));
    }

    #[test]
    fn assign_overwrites_existing() {
        let mut frame = VMFrame::new("f", 8, None);
        frame.assign("x", Value::Int(1)).unwrap();
        frame.assign("x", Value::Int(2)).unwrap();
        assert_eq!(frame.resolve("x"), Some(&Value::Int(2)));
    }

    #[test]
    fn resolve_undefined_returns_none() {
        let frame = VMFrame::new("f", 8, None);
        assert!(frame.resolve("missing").is_none());
    }

    // ------------------------------------------------------------------
    // The register limit
    // ------------------------------------------------------------------

    use interpreter_ir::{IIRInstr, Operand};

    /// A function whose body is `n` `const` instructions, each into a fresh
    /// register `r0`, `r1`, …, then `ret_void`.
    fn function_with_registers(name: &str, n: usize) -> IIRFunction {
        let mut instructions: Vec<IIRInstr> = (0..n)
            .map(|i| IIRInstr::new("const", Some(format!("r{i}")), vec![Operand::Int(1)], "any"))
            .collect();
        instructions.push(IIRInstr::new("ret_void", None, vec![], "void"));
        IIRFunction::new(name, vec![], "void", instructions)
    }

    /// Fill a frame to exactly the limit, then ask for one more register.
    /// The 65,537th assignment is an error naming the function — it used to
    /// return silently and leave the name unbound.
    #[test]
    fn assign_past_the_limit_is_an_error_not_a_silent_drop() {
        let mut frame = VMFrame::new("big", 0, None);
        for i in 0..VMFrame::MAX_FRAME_REGISTERS {
            frame.assign(&format!("r{i}"), Value::Int(i as i64)).unwrap();
        }
        // Re-assigning an existing name at the limit is still fine.
        frame.assign("r0", Value::Int(-1)).unwrap();
        assert_eq!(frame.resolve("r0"), Some(&Value::Int(-1)));

        let err = frame.assign("one_too_many", Value::Int(7)).unwrap_err();
        match err {
            VMError::RegisterLimitExceeded { function, registers, limit } => {
                assert_eq!(function, "big");
                assert_eq!(registers, VMFrame::MAX_FRAME_REGISTERS + 1);
                assert_eq!(limit, VMFrame::MAX_FRAME_REGISTERS);
            }
            other => panic!("expected RegisterLimitExceeded, got {other:?}"),
        }
        // The failed assignment left the frame unchanged.
        assert_eq!(frame.name_to_reg.len(), VMFrame::MAX_FRAME_REGISTERS);
        assert!(frame.resolve("one_too_many").is_none());
    }

    #[test]
    fn register_index_is_stable_for_a_known_name() {
        let mut frame = VMFrame::new("f", 0, None);
        let a = frame.register_index("a").unwrap();
        let b = frame.register_index("b").unwrap();
        assert_eq!((a, b), (0, 1));
        assert_eq!(frame.register_index("a").unwrap(), 0);
    }

    /// `register_count` is an untrusted hint: a huge value is clamped rather
    /// than allocated (it would abort the process).
    #[test]
    fn new_clamps_an_absurd_register_count() {
        let frame = VMFrame::new("f", usize::MAX, None);
        assert_eq!(frame.registers.len(), VMFrame::MAX_FRAME_REGISTERS);
    }

    #[test]
    fn check_accepts_a_function_exactly_at_the_limit() {
        let f = function_with_registers("at_limit", VMFrame::MAX_FRAME_REGISTERS);
        assert!(VMFrame::check_register_limit(&f).is_ok());
    }

    #[test]
    fn check_refuses_a_function_one_past_the_limit() {
        let f = function_with_registers("over", VMFrame::MAX_FRAME_REGISTERS + 1);
        let err = VMFrame::check_register_limit(&f).unwrap_err();
        assert_eq!(
            err.to_string(),
            "function \"over\" needs 65537 registers, more than the 65536 a vm-core frame can hold"
        );
    }

    /// Parameters are registers too, and count toward the limit.
    #[test]
    fn check_counts_parameters() {
        let mut f = function_with_registers("p", VMFrame::MAX_FRAME_REGISTERS);
        f.params = vec![("extra".into(), "any".into())];
        assert!(matches!(
            VMFrame::check_register_limit(&f),
            Err(VMError::RegisterLimitExceeded { registers: 65_537, .. })
        ));
    }

    /// Many instructions writing the same few names is fine: the count is of
    /// distinct names, not of instructions (the slow path's hash set).
    #[test]
    fn check_counts_distinct_names_not_instructions() {
        let mut f = function_with_registers("loopy", 0);
        let body: Vec<IIRInstr> = (0..VMFrame::MAX_FRAME_REGISTERS * 2)
            .map(|i| IIRInstr::new("const", Some(format!("r{}", i % 3)), vec![Operand::Int(1)], "any"))
            .collect();
        f.instructions.splice(0..0, body);
        assert!(VMFrame::check_register_limit(&f).is_ok());
    }
}
