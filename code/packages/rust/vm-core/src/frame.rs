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
//! # Parameters are positional
//!
//! A function's parameters occupy register slots `0..params.len()` by
//! *position* — the dispatcher writes argument `i` straight into
//! `registers[i]` — whatever their names.  So a function with parameters
//! `(a, a)` uses two slots even though it has one distinct parameter name,
//! and the first register a body instruction creates is slot 2, not slot 1:
//!
//! ```text
//!   params (a, a)      slot 0  slot 1   ← both arguments, by position
//!   name_to_reg        a → 1            ← the later duplicate wins the name
//!   next new register  slot 2           ← `next_reg`, never `name_to_reg.len()`
//! ```
//!
//! Counting registers therefore means `params.len()` plus the distinct
//! `dest` names that are not parameters — never just distinct names, which
//! under-counts a function with duplicate parameter names.
//!
//! # The `register_count` hint
//!
//! A function's `register_count` is only a pre-allocation hint and is not
//! trusted.  [`VMFrame::for_function`] pre-allocates at most what the
//! function could ever use — `params.len() + instructions.len()`, and never
//! more than the limit — so neither `register_count = usize::MAX` (which
//! would abort on allocation) nor a tiny recursive function claiming 65,536
//! registers can make each frame allocate far more than it needs.
//!
//! # The register-slot budget (all live frames together)
//!
//! The per-frame limit bounds one frame; it does not bound a program.  A
//! three-instruction recursive function
//!
//! ```text
//!   store_reg 65535, 1     ← grow this frame's file to 65,536 slots
//!   call r = f             ← recurse
//!   ret r
//! ```
//!
//! used to grow *every* frame to 65,536 × 24-byte `Value`s (1.5 MiB) and
//! pin ~768 MiB across 512 frames before `FrameOverflow` ended it.  So the
//! frames on one call stack also share a **budget** of register slots
//! (`VMCore::max_register_slots`, default 2²⁰), checked whenever a register
//! file grows.  The accounting is a running sum carried down the stack:
//!
//! ```text
//!   frame        slots_below              its own slots (capacity)
//!   ─────        ───────────              ────────────────────────
//!   root         0                        c₀
//!   callee 1     c₀                       c₁
//!   callee 2     c₀ + c₁                  c₂        ← top: the only frame
//!                                                      the VM grows
//!   a frame may grow while   slots_below + new capacity ≤ budget
//! ```
//!
//! It works because the VM only ever grows the **top** frame (a caller
//! cannot run while its callee does, and `call` reserves the caller's return
//! slot *before* pushing the callee), so the frames beneath the top are
//! frozen and their sum, recorded once at push time, stays exact.  Popping a
//! frame needs no bookkeeping at all: its share leaves with it.
//!
//! The sum is of `Vec` **capacities**, not lengths: capacity is what is
//! actually allocated, and growth here doubles (amortised O(1) appends) but
//! never past what the budget allows.  So the register files of one
//! `execute` occupy at most `budget × size_of::<Value>()` bytes — 24 MiB at
//! the default.  Exceeding it is [`VMError::RegisterBudgetExceeded`].
//!
//! Host code that reaches into `pub registers` directly (an opcode
//! extension, say) is trusted and outside the accounting; it should grow a
//! file only through [`VMFrame::store_index`] or [`VMFrame::assign`].

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
    ///
    /// Private, with a read-only view in [`Self::name_to_reg`]: inserting a
    /// name directly would skip `next_reg`, so the next new register would be
    /// handed the same slot and the two names would silently alias.  New
    /// names go through [`Self::register_index`] / [`Self::assign`].
    name_to_reg: HashMap<String, usize>,

    /// Register index in the **caller** frame where the return value should
    /// be written when this frame's `ret` instruction fires.
    ///
    /// `None` for the root frame (no caller exists).
    pub return_dest: Option<usize>,

    /// The index the next new register name gets.
    ///
    /// Starts at `params.len()`: parameters own slots `0..params.len()` by
    /// position, even when two share a name, so `name_to_reg.len()` would
    /// hand out a slot that still holds an argument.  It is also the number
    /// of registers in use, which is what the frame limit bounds.
    next_reg: usize,

    /// Register slots (`Vec` capacities) held by every frame **beneath**
    /// this one on the call stack, recorded when this frame was pushed.
    ///
    /// The frames beneath cannot grow while this one is live (see the module
    /// docs), so the number stays exact without any bookkeeping on pop.
    slots_below: usize,

    /// The VM-wide register-slot budget this frame was pushed under
    /// (`VMCore::max_register_slots`); `usize::MAX` for a standalone frame
    /// built with [`Self::new`] / [`Self::for_function`].
    slot_budget: usize,
}

impl VMFrame {
    /// Create a blank frame (no parameters) for the named function with
    /// `register_count` pre-allocated slots, all initialised to `Value::Null`.
    ///
    /// `register_count` is clamped to [`Self::MAX_FRAME_REGISTERS`], so an
    /// absurd value cannot abort the process on allocation.  The file still
    /// grows on demand in [`Self::assign`].
    ///
    /// A frame built here is *standalone*: it is not part of a VM call
    /// stack, so only the per-frame limit applies to it, not the VM-wide
    /// register-slot budget (see the module docs).
    pub fn new(fn_name: impl Into<String>, register_count: usize, return_dest: Option<usize>) -> Self {
        VMFrame {
            fn_name: fn_name.into(),
            ip: 0,
            registers: vec![Value::Null; register_count.min(Self::MAX_FRAME_REGISTERS)],
            name_to_reg: HashMap::new(),
            return_dest,
            next_reg: 0,
            slots_below: 0,
            slot_budget: usize::MAX,
        }
    }

    /// Build a standalone frame for a call to `function` — the same as the
    /// VM's own `for_call` with nothing beneath it and no VM-wide budget:
    /// parameter `i` owns slot `i`, and the untrusted `register_count` hint
    /// pre-allocates at least `params` and at most
    /// `min(params + instructions, MAX_FRAME_REGISTERS)` slots.
    ///
    /// # Errors
    ///
    /// [`VMError::RegisterLimitExceeded`] if the function has more parameters
    /// than a frame has registers.
    pub fn for_function(function: &IIRFunction, return_dest: Option<usize>) -> Result<Self, VMError> {
        Self::for_call(function, return_dest, 0, usize::MAX)
    }

    /// Build the frame for a call to `function` on a VM call stack: parameter
    /// `i` owns register slot `i` (the caller then writes argument `i` into
    /// `registers[i]`).
    ///
    /// `slots_below` is what the frames beneath already hold (the caller's
    /// [`Self::stack_slots`], or 0 for the root frame) and `slot_budget` the
    /// VM-wide register-slot budget; together they cap this frame at
    /// `slot_budget - slots_below` slots (and never more than
    /// [`Self::MAX_FRAME_REGISTERS`]).
    ///
    /// The register file is pre-allocated to the frontend's `register_count`
    /// hint, bounded on both sides:
    ///
    /// ```text
    ///   at most   params + instructions   (all the function could ever name)
    ///   at most   MAX_FRAME_REGISTERS, and what the budget leaves
    ///   at least  params                  (argument slots are written by index)
    /// ```
    ///
    /// The lower bound matters because a frontend may under-report — a hoisted
    /// McCarthy `LAMBDA` body whose only register is its parameter reports
    /// `0` — and the dispatcher's `registers[i] = arg` would otherwise index
    /// past the end and panic.  The upper bounds stop an over-reported hint
    /// from pre-allocating memory the function can never use, on every call.
    ///
    /// # Errors
    ///
    /// - [`VMError::RegisterLimitExceeded`] if the function has more
    ///   parameters than a frame has registers.  [`VMCore::execute`](crate::core::VMCore::execute)
    ///   refuses such a module before it runs; this is the backstop that keeps
    ///   every argument write in bounds (a function added to the module while
    ///   it runs never passed that check).
    /// - [`VMError::RegisterBudgetExceeded`] if the budget has no room left
    ///   for even the parameters.
    pub(crate) fn for_call(
        function: &IIRFunction,
        return_dest: Option<usize>,
        slots_below: usize,
        slot_budget: usize,
    ) -> Result<Self, VMError> {
        let n_params = function.params.len();
        if n_params > Self::MAX_FRAME_REGISTERS {
            return Err(VMError::RegisterLimitExceeded {
                function: function.name.clone(),
                registers: n_params,
                limit: Self::MAX_FRAME_REGISTERS,
            });
        }
        let slot_limit = Self::slot_limit_for(slots_below, slot_budget);
        if n_params > slot_limit {
            return Err(VMError::RegisterBudgetExceeded {
                function: function.name.clone(),
                slots: slots_below.saturating_add(n_params),
                budget: slot_budget,
            });
        }
        let could_ever_use = n_params.saturating_add(function.instructions.len());
        let prealloc = function
            .register_count
            .min(could_ever_use)
            .min(slot_limit)
            .max(n_params);
        let mut frame = VMFrame::new(function.name.clone(), prealloc, return_dest);
        frame.slots_below = slots_below;
        frame.slot_budget = slot_budget;
        for (idx, (param_name, _)) in function.params.iter().enumerate() {
            frame.name_to_reg.insert(param_name.clone(), idx);
        }
        frame.next_reg = n_params;
        Ok(frame)
    }

    // ------------------------------------------------------------------
    // Register access
    // ------------------------------------------------------------------

    /// Return the value of a named variable, or `None` if not in scope.
    pub fn resolve(&self, name: &str) -> Option<&Value> {
        let idx = *self.name_to_reg.get(name)?;
        self.registers.get(idx)
    }

    /// Read-only view of the variable name → register index map.
    ///
    /// There is deliberately no mutable access: a name inserted behind the
    /// frame's back would not advance its next-free-slot counter, and the
    /// next new register would alias it.  Use [`Self::assign`] or
    /// [`Self::register_index`] to create a name.
    pub fn name_to_reg(&self) -> &HashMap<String, usize> {
        &self.name_to_reg
    }

    /// Register slots held by this frame **and every frame beneath it** —
    /// the running sum the VM-wide budget is checked against.
    ///
    /// For the top frame of a VM call stack this is the whole stack's
    /// register-file footprint, in `Value` slots (`Vec` capacity, i.e. what
    /// is allocated, not just what is in use).  A callee pushed on top of
    /// this frame starts its own count from here.
    pub fn stack_slots(&self) -> usize {
        self.slots_below.saturating_add(self.registers.capacity())
    }

    /// Maximum number of distinct named registers per frame (2¹⁶).
    ///
    /// A resource bound: it caps one frame's register file.  It matches the
    /// index range `load_reg`/`store_reg` accept (`0..=65_535`) and twig-vm's
    /// `MAX_REGISTERS_PER_FRAME`.  The memory a whole call stack can pin is
    /// bounded separately, by the VM-wide register-slot budget (see the
    /// module docs).
    ///
    /// A function over the limit is refused before it runs (see
    /// [`Self::check_register_limit`]); a new register past the limit at run
    /// time is the same [`VMError::RegisterLimitExceeded`].  Neither path
    /// drops the write and carries on.
    pub const MAX_FRAME_REGISTERS: usize = 1 << 16;

    /// The most slots a frame may hold given what lies beneath it:
    ///
    /// ```text
    ///   min( MAX_FRAME_REGISTERS ,  slot_budget − slots_below )
    /// ```
    ///
    /// (saturating: a full budget leaves 0, not a wrapped huge number).
    fn slot_limit_for(slots_below: usize, slot_budget: usize) -> usize {
        Self::MAX_FRAME_REGISTERS.min(slot_budget.saturating_sub(slots_below))
    }

    /// Grow the register file so that it holds at least `len` slots.
    ///
    /// The one place a frame's register file grows, so the one place the
    /// limits are enforced:
    ///
    /// ```text
    ///   len ≤ registers.len()              → nothing to do
    ///   len > MAX_FRAME_REGISTERS          → RegisterLimitExceeded
    ///   len > budget − slots_below         → RegisterBudgetExceeded
    ///   otherwise                          → grow, capacity clamped to the limit
    /// ```
    ///
    /// Capacity grows by doubling, as `Vec` itself would (so a run of
    /// one-at-a-time `assign`s stays amortised O(1)), but is clamped to the
    /// slot limit — a plain `resize` could double past it, and capacity is
    /// what the budget counts.  On error the frame is unchanged.
    fn grow_to(&mut self, len: usize) -> Result<(), VMError> {
        if len <= self.registers.len() {
            return Ok(());
        }
        if len > Self::MAX_FRAME_REGISTERS {
            return Err(VMError::RegisterLimitExceeded {
                function: self.fn_name.clone(),
                registers: len,
                limit: Self::MAX_FRAME_REGISTERS,
            });
        }
        let slot_limit = Self::slot_limit_for(self.slots_below, self.slot_budget);
        if len > slot_limit {
            return Err(VMError::RegisterBudgetExceeded {
                function: self.fn_name.clone(),
                slots: self.slots_below.saturating_add(len),
                budget: self.slot_budget,
            });
        }
        let capacity = self.registers.capacity();
        if len > capacity {
            // `len ≤ slot_limit` was checked above, so the clamp is well formed.
            let target = capacity.saturating_mul(2).clamp(len, slot_limit);
            self.registers.reserve_exact(target - self.registers.len());
        }
        self.registers.resize(len, Value::Null);
        Ok(())
    }

    /// Refuse a function that needs more registers than a frame can hold.
    ///
    /// A frame's registers are:
    ///
    /// - one slot per **parameter, by position** — `params.len()` of them,
    ///   even if two parameters share a name (see the module docs); plus
    /// - one slot per distinct instruction `dest` that is **not** a
    ///   parameter name (a `dest` naming a parameter reuses its slot).
    ///
    /// Every handler that creates a register does so through
    /// [`Self::assign`] or the `call` return slot, both keyed by an
    /// instruction's `dest`, so this is an exact, static count — the same
    /// number the frame's `next_reg` reaches at run time.  Counting distinct
    /// names alone would under-count duplicate parameters and let a function
    /// whose argument slots do not fit pass.
    ///
    /// Counting needs hash sets, but almost every function is far below the
    /// limit, so a cheap upper bound comes first:
    ///
    /// ```text
    ///   params + (instructions with a dest)  ≤ limit  →  Ok, no allocation
    ///   otherwise                                    →  count exactly
    /// ```
    ///
    /// # Errors
    ///
    /// [`VMError::RegisterLimitExceeded`] naming the function, its register
    /// count and the limit.
    pub fn check_register_limit(function: &IIRFunction) -> Result<(), VMError> {
        let n_params = function.params.len();
        let dests = function.instructions.iter().filter(|i| i.dest.is_some()).count();
        if n_params.saturating_add(dests) <= Self::MAX_FRAME_REGISTERS {
            return Ok(());
        }
        let param_names: HashSet<&str> =
            function.params.iter().map(|(name, _)| name.as_str()).collect();
        let locals: HashSet<&str> = function
            .instructions
            .iter()
            .filter_map(|i| i.dest.as_deref())
            .filter(|name| !param_names.contains(name))
            .collect();
        let registers = n_params.saturating_add(locals.len());
        if registers > Self::MAX_FRAME_REGISTERS {
            return Err(VMError::RegisterLimitExceeded {
                function: function.name.clone(),
                registers,
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
    /// A new name's slot is allocated **now**, not when it is first written:
    /// the `call` handler relies on that, because the caller must not grow
    /// after its callee has been pushed on top of it (the budget accounting
    /// assumes only the top frame grows).
    ///
    /// # Errors
    ///
    /// [`VMError::RegisterLimitExceeded`] if `name` is new and the frame
    /// already holds [`Self::MAX_FRAME_REGISTERS`] registers, or
    /// [`VMError::RegisterBudgetExceeded`] if the VM-wide budget has no room
    /// for its slot.  Either way the frame is left unchanged.
    pub fn register_index(&mut self, name: &str) -> Result<usize, VMError> {
        if let Some(&idx) = self.name_to_reg.get(name) {
            return Ok(idx);
        }
        // `next_reg`, not `name_to_reg.len()`: with duplicate parameter names
        // the map is shorter than the parameter slots, and its length would
        // be a slot that still holds an argument.
        let next_idx = self.next_reg;
        if next_idx >= Self::MAX_FRAME_REGISTERS {
            return Err(VMError::RegisterLimitExceeded {
                function: self.fn_name.clone(),
                registers: next_idx + 1,
                limit: Self::MAX_FRAME_REGISTERS,
            });
        }
        // Grow first, then record the name: a refused growth leaves no
        // half-registered name behind.
        self.grow_to(next_idx + 1)?;
        self.name_to_reg.insert(name.to_string(), next_idx);
        self.next_reg = next_idx + 1;
        Ok(next_idx)
    }

    /// Assign a value to a named variable, allocating a new register if needed.
    ///
    /// # Errors
    ///
    /// [`VMError::RegisterLimitExceeded`] if `name` would be the frame's
    /// 65,537th register.  This used to return silently, dropping the value,
    /// so the program failed later with a misleading `undefined variable`.
    /// [`VMError::RegisterBudgetExceeded`] if the call stack's register files
    /// have used up the VM-wide budget.
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), VMError> {
        // `register_index` has already grown the file to cover a new slot.
        let idx = self.register_index(name)?;
        self.registers[idx] = value;
        Ok(())
    }

    /// Write `value` into register slot `idx` by **index** (the `store_reg`
    /// view of the register file), growing the file if needed.
    ///
    /// The by-index and by-name views share one file and one set of limits:
    /// slot `idx` may be a named register's slot, and growing the file to
    /// reach it is charged to the budget exactly like a new name.
    ///
    /// # Errors
    ///
    /// [`VMError::RegisterLimitExceeded`] if `idx` is past the last slot a
    /// frame can have; [`VMError::RegisterBudgetExceeded`] if reaching it
    /// would overrun the VM-wide budget.  The frame is unchanged on error.
    pub fn store_index(&mut self, idx: usize, value: Value) -> Result<(), VMError> {
        self.grow_to(idx.saturating_add(1))?;
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

    /// A function with the given parameter names, a `register_count` hint
    /// and an empty body (just `ret_void`).
    fn fn_with_params(names: &[&str], register_count: usize) -> IIRFunction {
        let params = names.iter().map(|n| (n.to_string(), "any".to_string())).collect();
        let mut f = IIRFunction::new("f", params, "void", vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        f.register_count = register_count;
        f
    }

    #[test]
    fn for_function_maps_params() {
        let f = fn_with_params(&["a", "b"], 8);
        let mut frame = VMFrame::for_function(&f, None).unwrap();
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
        let f = fn_with_params(&["x"], 0);
        let mut frame = VMFrame::for_function(&f, None).unwrap();
        assert!(
            frame.registers.len() >= f.params.len(),
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

    /// Duplicate parameter names still own one positional slot each, so the
    /// first new register goes *after* them.  Handing out
    /// `name_to_reg.len()` (= 1 here) would overwrite the second argument.
    #[test]
    fn a_new_register_never_reuses_a_duplicate_parameter_slot() {
        let f = fn_with_params(&["a", "a"], 0);
        let mut frame = VMFrame::for_function(&f, None).unwrap();
        frame.registers[0] = Value::Int(10);
        frame.registers[1] = Value::Int(20);
        frame.assign("x", Value::Int(99)).unwrap();
        assert_eq!(frame.name_to_reg["x"], 2);
        assert_eq!(frame.registers[0], Value::Int(10));
        assert_eq!(frame.registers[1], Value::Int(20), "argument 2 must survive");
        assert_eq!(frame.resolve("x"), Some(&Value::Int(99)));
    }

    /// More parameters than a frame has registers — all with one name, so
    /// only one *distinct* name — is a typed error, not a frame too small for
    /// its argument writes (which used to panic with index out of bounds).
    #[test]
    fn for_function_refuses_more_positional_params_than_the_limit() {
        let names = vec!["a"; VMFrame::MAX_FRAME_REGISTERS + 1];
        let f = fn_with_params(&names, 0);
        match VMFrame::for_function(&f, None) {
            Err(VMError::RegisterLimitExceeded { registers, limit, .. }) => {
                assert_eq!(registers, VMFrame::MAX_FRAME_REGISTERS + 1);
                assert_eq!(limit, VMFrame::MAX_FRAME_REGISTERS);
            }
            other => panic!("expected RegisterLimitExceeded, got {other:?}"),
        }
    }

    /// Exactly the limit of parameters fits, with a slot for each argument.
    #[test]
    fn for_function_accepts_params_exactly_at_the_limit() {
        let names = vec!["a"; VMFrame::MAX_FRAME_REGISTERS];
        let frame = VMFrame::for_function(&fn_with_params(&names, 0), None).unwrap();
        assert_eq!(frame.registers.len(), VMFrame::MAX_FRAME_REGISTERS);
    }

    /// An over-reported `register_count` cannot pre-allocate more than the
    /// function could ever name (`params + instructions`).
    #[test]
    fn for_function_bounds_the_hint_by_what_the_function_can_use() {
        let f = fn_with_params(&["n"], VMFrame::MAX_FRAME_REGISTERS);
        let frame = VMFrame::for_function(&f, None).unwrap();
        // 1 parameter + 1 instruction (`ret_void`).
        assert_eq!(frame.registers.len(), 2);
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

    /// Parameters count by position: 65,537 parameters that all share one
    /// name are 65,537 registers, not one.
    #[test]
    fn check_counts_duplicate_parameters_by_position() {
        let names = vec!["a"; VMFrame::MAX_FRAME_REGISTERS + 1];
        let f = fn_with_params(&names, 0);
        assert!(matches!(
            VMFrame::check_register_limit(&f),
            Err(VMError::RegisterLimitExceeded { registers: 65_537, .. })
        ));
    }

    /// A `dest` that names a parameter reuses the parameter's slot, so it is
    /// not counted twice.
    #[test]
    fn check_does_not_double_count_a_dest_that_is_a_parameter() {
        let mut f = function_with_registers("p", VMFrame::MAX_FRAME_REGISTERS);
        f.params = vec![("r0".into(), "any".into())];
        // 65,536 dests r0..r65535, but r0 is the parameter: 1 + 65,535 locals
        // = 65,536 (the upper bound, 65,537, sends this down the exact path).
        assert!(VMFrame::check_register_limit(&f).is_ok());
        f.params.push(("extra".into(), "any".into()));
        // 2 params + 65,535 locals = 65,537.
        assert!(matches!(
            VMFrame::check_register_limit(&f),
            Err(VMError::RegisterLimitExceeded { registers: 65_537, .. })
        ));
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

    // ------------------------------------------------------------------
    // The register-slot budget
    // ------------------------------------------------------------------

    /// A parameterless frame pushed under `budget` with `below` slots
    /// already held by the frames beneath it.
    fn budgeted(below: usize, budget: usize) -> VMFrame {
        VMFrame::for_call(&fn_with_params(&[], 0), None, below, budget).unwrap()
    }

    /// `store_index` grows the file to reach a slot, within the frame's share
    /// of the budget; one slot more is the typed error and leaves the frame
    /// exactly as it was.
    #[test]
    fn store_index_grows_within_the_budget_and_refuses_past_it() {
        let mut frame = budgeted(60, 100); // 40 slots left for this frame
        frame.store_index(39, Value::Int(7)).unwrap();
        assert_eq!(frame.registers[39], Value::Int(7));
        assert_eq!(frame.stack_slots(), 100);

        let before = frame.registers.clone();
        match frame.store_index(40, Value::Int(8)).unwrap_err() {
            VMError::RegisterBudgetExceeded { function, slots, budget } => {
                assert_eq!(function, "f");
                assert_eq!(slots, 101);
                assert_eq!(budget, 100);
            }
            other => panic!("expected RegisterBudgetExceeded, got {other:?}"),
        }
        assert_eq!(frame.registers, before);
        assert_eq!(frame.stack_slots(), 100);
    }

    /// Named registers are charged the same way, and a refused name is not
    /// half-registered: it does not resolve and does not consume a slot
    /// number.
    #[test]
    fn a_refused_name_leaves_the_frame_unchanged() {
        let mut frame = budgeted(0, 3);
        for name in ["a", "b", "c"] {
            frame.assign(name, Value::Int(1)).unwrap();
        }
        assert!(matches!(
            frame.assign("d", Value::Int(1)),
            Err(VMError::RegisterBudgetExceeded { slots: 4, budget: 3, .. })
        ));
        assert!(frame.resolve("d").is_none());
        assert_eq!(frame.name_to_reg().len(), 3);
        assert_eq!(frame.register_index("c").unwrap(), 2, "known names still resolve");
    }

    /// Capacity, not length, is what the budget counts, so growth must not
    /// let `Vec`'s doubling overshoot: one-at-a-time assignment up to the
    /// limit ends with capacity exactly at the limit, never above it.
    #[test]
    fn amortised_growth_never_allocates_past_the_frame_share() {
        let mut frame = budgeted(0, 1_000);
        for i in 0..1_000 {
            frame.assign(&format!("r{i}"), Value::Int(i)).unwrap();
            assert!(frame.registers.capacity() <= 1_000, "capacity overshot at {i}");
        }
        assert_eq!(frame.stack_slots(), 1_000);
        // Doubling still happens below the limit (amortised O(1) appends).
        let mut small = budgeted(0, 1_000);
        small.assign("x", Value::Null).unwrap();
        small.assign("y", Value::Null).unwrap();
        small.assign("z", Value::Null).unwrap();
        assert_eq!(small.registers.capacity(), 4);
    }

    /// `for_call` refuses a frame whose parameters alone do not fit in what
    /// the budget leaves, and caps an over-reported hint at that share.
    #[test]
    fn for_call_respects_the_remaining_budget() {
        let f = fn_with_params(&["a", "b", "c"], 64);
        assert!(matches!(
            VMFrame::for_call(&f, None, 98, 100),
            Err(VMError::RegisterBudgetExceeded { slots: 101, budget: 100, .. })
        ));
        let frame = VMFrame::for_call(&f, None, 96, 100).unwrap();
        assert_eq!(frame.registers.len(), 4, "hint 64 capped by params + instructions");
        assert_eq!(frame.stack_slots(), 100);
    }

    /// The per-frame limit still applies when the budget is larger, so an
    /// index past the last slot a frame can have is `RegisterLimitExceeded`.
    #[test]
    fn store_index_past_the_frame_limit_is_the_frame_limit_error() {
        let mut frame = VMFrame::new("f", 0, None);
        assert!(matches!(
            frame.store_index(VMFrame::MAX_FRAME_REGISTERS, Value::Null),
            Err(VMError::RegisterLimitExceeded { registers: 65_537, limit: 65_536, .. })
        ));
        assert!(frame.registers.is_empty());
    }

    /// A standalone frame has no budget beyond the per-frame limit.
    #[test]
    fn a_standalone_frame_is_bounded_only_by_the_frame_limit() {
        let mut frame = VMFrame::new("f", 0, None);
        frame.store_index(VMFrame::MAX_FRAME_REGISTERS - 1, Value::Int(1)).unwrap();
        assert_eq!(frame.stack_slots(), VMFrame::MAX_FRAME_REGISTERS);
    }

    /// The read-only name map view shows the parameters by position.
    #[test]
    fn name_to_reg_view_lists_parameters() {
        let frame = VMFrame::for_function(&fn_with_params(&["a", "b"], 0), None).unwrap();
        assert_eq!(frame.name_to_reg().get("a"), Some(&0));
        assert_eq!(frame.name_to_reg().get("b"), Some(&1));
        assert_eq!(frame.name_to_reg().len(), 2);
    }
}
