//! Dispatch loop and standard opcode handlers for vm-core.
//!
//! # Borrow-checker architecture
//!
//! Rust requires that we hold only **one** mutable borrow of a value at a time.
//! The dispatch loop needs two things simultaneously:
//!
//! 1. A mutable reference to the execution state (frame stack, module functions,
//!    memory, counters).
//! 2. The opcode handler to call — which may itself mutate the execution state.
//!
//! The solution is to split the dispatch context into two structs:
//!
//! - [`DispatchCtx`] — all **mutable** execution state.
//! - Separate `&HashMap` parameters for read-only lookups (`extra_opcodes`,
//!   `jit_handlers`) that are passed to the dispatch loop directly.
//!
//! Handler functions take `(ctx: &mut DispatchCtx, instr: &IIRInstr)` — they
//! access the current frame through `ctx.frames.last_mut()`.  This is safe
//! because `frames` and the other fields are disjoint fields of `DispatchCtx`,
//! and handlers release each field borrow before borrowing another.
//!
//! # Opcode table
//!
//! Standard opcodes are dispatched via a `match` (the compiler emits a jump
//! table at `-O2`).  Language-specific opcodes go into the `extra_opcodes`
//! parameter, which shadows the standard table.
//!
//! # u8_wrap
//!
//! When `ctx.u8_wrap` is `true` (Tetrad 8-bit mode), every arithmetic result
//! is masked with `& 0xFF`.  The mask is applied inside each arithmetic handler,
//! not in the dispatch loop.

use std::collections::HashMap;
use interpreter_ir::instr::{IIRInstr, Operand};
use crate::errors::VMError;
use crate::frame::VMFrame;
use crate::value::Value;

// ---------------------------------------------------------------------------
// DispatchCtx — mutable execution state
// ---------------------------------------------------------------------------

/// All mutable state the dispatch loop needs in one place.
///
/// `extra_opcodes` and `jit_handlers` are intentionally **not** here — they
/// are passed as separate `&HashMap` parameters to avoid borrow conflicts when
/// calling handlers.
///
/// This struct is `pub` because it appears in the [`OpcodeHandler`] signature,
/// which language frontends pass to [`crate::core::VMCore::register_opcode`].
pub struct DispatchCtx<'a> {
    pub frames: &'a mut Vec<VMFrame>,
    pub module_fns: &'a mut Vec<interpreter_ir::function::IIRFunction>,
    pub memory: &'a mut HashMap<i64, Value>,
    pub builtins: &'a crate::builtins::BuiltinRegistry,
    pub u8_wrap: bool,
    pub max_frames: usize,
    /// Maximum number of distinct memory-address entries before `store_mem` /
    /// `io_out` return `VMError::Custom`.  Guards against unbounded HashMap
    /// growth from looping IIR programs.
    pub max_memory_entries: usize,
    /// Optional hard cap on total instructions dispatched.  `None` = unlimited
    /// (safe for trusted code); `Some(N)` returns `VMError::Custom` after N
    /// instructions (sandbox / untrusted-IIR mode).
    pub max_instructions: Option<u64>,
    pub fn_call_counts: &'a mut HashMap<String, u32>,
    pub metrics_instrs: &'a mut u64,
    pub metrics_jit_hits: &'a mut u64,
    // ── LANG17: branch + loop metrics ────────────────────────────────────────
    /// Per-function, per-instruction-index branch taken/not-taken counts.
    ///
    /// Keyed `branch_stats[fn_name][ip]`.  Only conditional branches
    /// (`jmp_if_true`, `jmp_if_false`) write here.
    pub branch_stats: &'a mut HashMap<String, HashMap<usize, crate::branch_stats::BranchStats>>,
    /// Per-function, per-source-ip back-edge (loop iteration) counts.
    ///
    /// Keyed `loop_back_edge_counts[fn_name][source_ip]`.  Incremented
    /// whenever a jump (conditional or unconditional) targets an earlier
    /// instruction index.
    pub loop_back_edge_counts: &'a mut HashMap<String, HashMap<usize, u64>>,
    /// Optional per-instruction trace accumulator (LANG17 execute_traced path).
    ///
    /// `None` on the normal `execute()` path — zero overhead.  `Some` on
    /// `execute_traced()`, where each instruction appends a `VMTrace`.
    pub tracer: Option<&'a mut Vec<crate::trace::VMTrace>>,
}

/// Signature for a language-specific opcode extension handler.
///
/// `jit_handlers` is passed separately so the handler can look up compiled
/// functions without borrowing through `DispatchCtx`.
pub type OpcodeHandler = Box<
    dyn Fn(
            &mut DispatchCtx<'_>,
            &HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,
            &IIRInstr,
        ) -> Result<Option<Value>, VMError>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// Operand resolution helpers
// ---------------------------------------------------------------------------

fn resolve_operand(frame: &VMFrame, operand: &Operand) -> Result<Value, VMError> {
    match operand {
        Operand::Var(name) => frame
            .resolve(name)
            .cloned()
            .ok_or_else(|| VMError::UndefinedVariable(name.clone())),
        Operand::Int(n)   => Ok(Value::Int(*n)),
        Operand::Float(f) => Ok(Value::Float(*f)),
        Operand::Bool(b)  => Ok(Value::Bool(*b)),
    }
}

fn resolve_src(frame: &VMFrame, srcs: &[Operand], idx: usize) -> Result<Value, VMError> {
    srcs.get(idx)
        .ok_or_else(|| VMError::Custom(format!("missing operand at index {idx}")))
        .and_then(|op| resolve_operand(frame, op))
}

// ---------------------------------------------------------------------------
// Arithmetic helpers
// ---------------------------------------------------------------------------

#[inline]
fn wrap_int(v: i64, u8_wrap: bool) -> Value {
    if u8_wrap { Value::Int(v & 0xFF) } else { Value::Int(v) }
}

fn int_srcs(
    frame: &VMFrame,
    srcs: &[Operand],
) -> Result<(i64, i64), VMError> {
    let a = resolve_src(frame, srcs, 0)?
        .as_i64()
        .ok_or_else(|| VMError::Custom("arithmetic on non-integer".into()))?;
    let b = resolve_src(frame, srcs, 1)?
        .as_i64()
        .ok_or_else(|| VMError::Custom("arithmetic on non-integer".into()))?;
    Ok((a, b))
}

// ---------------------------------------------------------------------------
// Standard opcode handlers
//
// Handler signature: fn(&mut DispatchCtx, &IIRInstr) -> Result<Option<Value>, VMError>
//
// Handlers access the current frame via `ctx.frames.last_mut()`.
// Handlers that call functions also use ctx.module_fns.
// These are DISJOINT fields of DispatchCtx — the borrow checker allows it.
// ---------------------------------------------------------------------------

fn handle_const(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let value = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("empty frame stack".into()))?;
        resolve_src(frame, &instr.srcs, 0)?
    };
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, value.clone());
    }
    Ok(Some(value))
}

macro_rules! binary_arith_handler {
    ($name:ident, $op:tt) => {
        fn $name(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
            let (a, b) = {
                let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
                int_srcs(frame, &instr.srcs)?
            };
            let result = wrap_int(a $op b, ctx.u8_wrap);
            if let Some(dest) = &instr.dest {
                ctx.frames.last_mut().unwrap().assign(dest, result.clone());
            }
            Ok(Some(result))
        }
    };
}

binary_arith_handler!(handle_add, +);
binary_arith_handler!(handle_sub, -);
binary_arith_handler!(handle_mul, *);

fn handle_div(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (a, b) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        int_srcs(frame, &instr.srcs)?
    };
    if b == 0 { return Err(VMError::DivisionByZero); }
    let result = wrap_int(a / b, ctx.u8_wrap);
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

fn handle_mod(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (a, b) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        int_srcs(frame, &instr.srcs)?
    };
    if b == 0 { return Err(VMError::DivisionByZero); }
    let result = wrap_int(a % b, ctx.u8_wrap);
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

fn handle_neg(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let a = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        resolve_src(frame, &instr.srcs, 0)?
            .as_i64()
            .ok_or_else(|| VMError::Custom("neg on non-integer".into()))?
    };
    let result = wrap_int(-a, ctx.u8_wrap);
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

// Bitwise -------------------------------------------------------------------

macro_rules! binary_bitwise_handler {
    ($name:ident, $op:tt) => {
        fn $name(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
            let (a, b) = {
                let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
                int_srcs(frame, &instr.srcs)?
            };
            let result = Value::Int(a $op b);
            if let Some(dest) = &instr.dest {
                ctx.frames.last_mut().unwrap().assign(dest, result.clone());
            }
            Ok(Some(result))
        }
    };
}

binary_bitwise_handler!(handle_and, &);
binary_bitwise_handler!(handle_or,  |);
binary_bitwise_handler!(handle_xor, ^);

fn handle_not(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let a = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        resolve_src(frame, &instr.srcs, 0)?
            .as_i64()
            .ok_or_else(|| VMError::Custom("not on non-integer".into()))?
    };
    let result = Value::Int(!a);
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

fn handle_shl(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (a, n) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        (resolve_src(frame, &instr.srcs, 0)?.as_i64().unwrap_or(0),
         resolve_src(frame, &instr.srcs, 1)?.as_i64().unwrap_or(0))
    };
    // Clamp to 0..63 to prevent a panic in debug mode.  Rust panics on
    // `i64 << n` when n >= 64, and `n as u32` wraps negative n to a huge value.
    let shift = n.clamp(0, 63) as u32;
    let result = Value::Int(a << shift);
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

fn handle_shr(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (a, n) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        (resolve_src(frame, &instr.srcs, 0)?.as_i64().unwrap_or(0),
         resolve_src(frame, &instr.srcs, 1)?.as_i64().unwrap_or(0))
    };
    // Same clamp as shl — prevents panic for out-of-range shift amounts.
    let shift = n.clamp(0, 63) as u32;
    let result = Value::Int(a >> shift);
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

// Comparisons ---------------------------------------------------------------

macro_rules! int_cmp_handler {
    ($name:ident, $op:tt) => {
        fn $name(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
            let (a, b) = {
                let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
                int_srcs(frame, &instr.srcs)?
            };
            let result = Value::Bool(a $op b);
            if let Some(dest) = &instr.dest {
                ctx.frames.last_mut().unwrap().assign(dest, result.clone());
            }
            Ok(Some(result))
        }
    };
}

fn handle_cmp_eq(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (a, b) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        (resolve_src(frame, &instr.srcs, 0)?, resolve_src(frame, &instr.srcs, 1)?)
    };
    let result = Value::Bool(a == b);
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

fn handle_cmp_ne(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (a, b) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        (resolve_src(frame, &instr.srcs, 0)?, resolve_src(frame, &instr.srcs, 1)?)
    };
    let result = Value::Bool(a != b);
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

int_cmp_handler!(handle_cmp_lt, <);
int_cmp_handler!(handle_cmp_le, <=);
int_cmp_handler!(handle_cmp_gt, >);
int_cmp_handler!(handle_cmp_ge, >=);

// Control flow --------------------------------------------------------------

fn handle_label(_ctx: &mut DispatchCtx, _instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    // Labels are no-ops at runtime; they exist only for branch resolution.
    Ok(None)
}

fn find_label_ip(
    fns: &[interpreter_ir::function::IIRFunction],
    fn_name: &str,
    label: &str,
) -> Result<usize, VMError> {
    let fn_ = fns.iter().find(|f| f.name == fn_name)
        .ok_or_else(|| VMError::Custom(format!("function {fn_name:?} not found")))?;
    fn_.label_index(label)
        .ok_or_else(|| VMError::UndefinedLabel {
            label: label.to_string(),
            function: fn_name.to_string(),
        })
}

fn handle_jmp(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (fn_name, label, source_ip) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let label = instr.srcs.first()
            .and_then(|s| s.as_var())
            .ok_or_else(|| VMError::Custom("jmp missing label".into()))?
            .to_string();
        // source_ip is frame.ip - 1 because the dispatch loop already advanced it.
        (frame.fn_name.clone(), label, frame.ip.saturating_sub(1))
    };
    let target = find_label_ip(ctx.module_fns, &fn_name, &label)?;
    // Back-edge detection: unconditional jump to an earlier instruction.
    if target < source_ip {
        ctx.loop_back_edge_counts
            .entry(fn_name)
            .or_default()
            .entry(source_ip)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }
    ctx.frames.last_mut().unwrap().ip = target;
    Ok(None)
}

fn handle_jmp_if_true(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (fn_name, cond, label, source_ip) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let cond = resolve_src(frame, &instr.srcs, 0)?;
        let label = instr.srcs.get(1)
            .and_then(|s| s.as_var())
            .ok_or_else(|| VMError::Custom("jmp_if_true missing label".into()))?
            .to_string();
        (frame.fn_name.clone(), cond, label, frame.ip.saturating_sub(1))
    };
    let taken = cond.is_truthy();
    // Record branch stats (always-on, O(1) per branch).
    ctx.branch_stats
        .entry(fn_name.clone())
        .or_default()
        .entry(source_ip)
        .or_default()
        .bump(taken);
    if taken {
        let target = find_label_ip(ctx.module_fns, &fn_name, &label)?;
        // Back-edge: taken conditional branch to earlier instruction = loop.
        if target < source_ip {
            ctx.loop_back_edge_counts
                .entry(fn_name)
                .or_default()
                .entry(source_ip)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
        ctx.frames.last_mut().unwrap().ip = target;
    }
    Ok(None)
}

fn handle_jmp_if_false(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (fn_name, cond, label, source_ip) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let cond = resolve_src(frame, &instr.srcs, 0)?;
        let label = instr.srcs.get(1)
            .and_then(|s| s.as_var())
            .ok_or_else(|| VMError::Custom("jmp_if_false missing label".into()))?
            .to_string();
        (frame.fn_name.clone(), cond, label, frame.ip.saturating_sub(1))
    };
    let taken = !cond.is_truthy();
    // Record branch stats.
    ctx.branch_stats
        .entry(fn_name.clone())
        .or_default()
        .entry(source_ip)
        .or_default()
        .bump(taken);
    if taken {
        let target = find_label_ip(ctx.module_fns, &fn_name, &label)?;
        // Back-edge: taken conditional branch to earlier instruction = loop.
        if target < source_ip {
            ctx.loop_back_edge_counts
                .entry(fn_name)
                .or_default()
                .entry(source_ip)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
        ctx.frames.last_mut().unwrap().ip = target;
    }
    Ok(None)
}

fn handle_ret(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (value, return_dest) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let value = if instr.srcs.is_empty() {
            Value::Null
        } else {
            resolve_src(frame, &instr.srcs, 0)?
        };
        (value, frame.return_dest)
    };
    ctx.frames.pop();
    if let (Some(dest_idx), Some(caller)) = (return_dest, ctx.frames.last_mut()) {
        if dest_idx >= caller.registers.len() {
            caller.registers.resize(dest_idx + 1, Value::Null);
        }
        caller.registers[dest_idx] = value.clone();
    }
    Ok(Some(value))
}

fn handle_ret_void(ctx: &mut DispatchCtx, _instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    ctx.frames.pop();
    Ok(Some(Value::Null))
}

// Memory --------------------------------------------------------------------

/// Maximum register index.  A negative or huge register index from a
/// user-supplied IIR program must be rejected — `(-1i64) as usize` wraps to
/// `usize::MAX`, making the subsequent `resize(idx + 1, …)` wrap back to 0
/// and then the index access panics.
const MAX_REGISTER_IDX: usize = 65_535;

fn handle_load_reg(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let value = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let raw = resolve_src(frame, &instr.srcs, 0)?.as_i64().unwrap_or(0);
        if raw < 0 || raw as usize > MAX_REGISTER_IDX {
            return Err(VMError::Custom(format!("invalid register index: {raw}")));
        }
        frame.registers.get(raw as usize).cloned().unwrap_or(Value::Null)
    };
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, value.clone());
    }
    Ok(Some(value))
}

fn handle_store_reg(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (idx, value) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let raw = resolve_src(frame, &instr.srcs, 0)?.as_i64().unwrap_or(0);
        if raw < 0 || raw as usize > MAX_REGISTER_IDX {
            return Err(VMError::Custom(format!("invalid register index: {raw}")));
        }
        let value = resolve_src(frame, &instr.srcs, 1)?;
        (raw as usize, value)
    };
    let frame = ctx.frames.last_mut().unwrap();
    if idx >= frame.registers.len() {
        frame.registers.resize(idx + 1, Value::Null);
    }
    frame.registers[idx] = value;
    Ok(None)
}

fn handle_load_mem(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let addr = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        resolve_src(frame, &instr.srcs, 0)?.as_i64().unwrap_or(0)
    };
    let value = ctx.memory.get(&addr).cloned().unwrap_or(Value::Int(0));
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, value.clone());
    }
    Ok(Some(value))
}

fn handle_store_mem(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (addr, value) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let addr = resolve_src(frame, &instr.srcs, 0)?.as_i64().unwrap_or(0);
        let value = resolve_src(frame, &instr.srcs, 1)?;
        (addr, value)
    };
    // Enforce the memory entry cap.  A loop calling store_mem with different
    // addresses can grow the HashMap without bound, eventually OOM-killing the
    // process.  Reject new addresses once the cap is reached.
    if !ctx.memory.contains_key(&addr) && ctx.memory.len() >= ctx.max_memory_entries {
        return Err(VMError::Custom(format!(
            "memory entry limit {} exceeded", ctx.max_memory_entries
        )));
    }
    ctx.memory.insert(addr, value);
    Ok(None)
}

// Function calls ------------------------------------------------------------

/// Handle a `call fn_name arg1 arg2 …` instruction.
///
/// JIT handlers in the `jit_handlers` map take priority over interpreted
/// execution.  The `jit_handlers` map is passed as a separate reference so
/// we can call the handler while also mutating `ctx`.
fn handle_call(
    ctx: &mut DispatchCtx,
    jit_handlers: &HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,
    instr: &IIRInstr,
) -> Result<Option<Value>, VMError> {
    // Step 1: Extract everything we need from the current frame BEFORE any other
    // borrows.  This releases the frame borrow so we can mutate ctx later.
    let (fn_name, args, dest_name) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let fn_name = instr.srcs.first()
            .and_then(|s| s.as_var())
            .ok_or_else(|| VMError::Custom("call missing function name".into()))?
            .to_string();
        let args: Vec<Value> = instr.srcs[1..].iter()
            .map(|s| resolve_operand(frame, s))
            .collect::<Result<_, _>>()?;
        let dest_name = instr.dest.clone();
        (fn_name, args, dest_name)
    }; // frame borrow released here

    // Step 2: JIT handler takes priority.
    if let Some(handler) = jit_handlers.get(&fn_name) {
        // jit_handlers borrow is a SEPARATE reference — no conflict with ctx.
        let result = handler(&args);
        *ctx.metrics_jit_hits += 1;
        if let Some(dest) = &dest_name {
            ctx.frames.last_mut().unwrap().assign(dest, result.clone());
        }
        return Ok(Some(result));
    }

    // Step 3: Interpreter path.
    let callee_idx = ctx.module_fns.iter().position(|f| f.name == fn_name)
        .ok_or_else(|| VMError::UnknownOpcode(format!("function {fn_name:?}")))?;

    if ctx.frames.len() >= ctx.max_frames {
        return Err(VMError::FrameOverflow { depth: ctx.max_frames, callee: fn_name.clone() });
    }

    // Step 4: Determine the return-value register slot in the CALLER frame.
    // We need to look up / allocate it in the caller frame before pushing
    // the callee frame.
    let ret_dest: Option<usize> = dest_name.as_ref().map(|dest_name| {
        let frame = ctx.frames.last_mut().unwrap();
        let next_idx = frame.name_to_reg.len();
        *frame.name_to_reg.entry(dest_name.clone()).or_insert(next_idx)
    });

    // Step 5: Build and push the callee frame.
    let callee_params = ctx.module_fns[callee_idx].params.clone();
    let callee_reg_count = ctx.module_fns[callee_idx].register_count;
    let mut callee_frame = VMFrame::for_function(
        &fn_name, &callee_params, callee_reg_count, ret_dest,
    );
    for (i, val) in args.into_iter().enumerate().take(callee_params.len()) {
        callee_frame.registers[i] = val;
    }

    // Use saturating_add to prevent wrap-around that would make a hot function
    // appear cold again (defeating JIT tier-promotion logic).
    ctx.module_fns[callee_idx].call_count =
        ctx.module_fns[callee_idx].call_count.saturating_add(1);
    let count = ctx.fn_call_counts.entry(fn_name).or_insert(0);
    *count = count.saturating_add(1);
    ctx.frames.push(callee_frame);

    Ok(None) // result stored when callee executes `ret`
}

fn handle_call_builtin(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (name, args) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let name = instr.srcs.first()
            .and_then(|s| s.as_var())
            .ok_or_else(|| VMError::Custom("call_builtin missing name".into()))?
            .to_string();
        let args: Vec<Value> = instr.srcs[1..].iter()
            .map(|s| resolve_operand(frame, s))
            .collect::<Result<_, _>>()?;
        (name, args)
    };
    // builtins is &BuiltinRegistry (immutable), ctx.frames is the only mutable borrow — fine.
    let result = ctx.builtins.call(&name, &args)?;
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

// I/O -----------------------------------------------------------------------

fn handle_io_in(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let port = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        resolve_src(frame, &instr.srcs, 0)?.as_i64().unwrap_or(0)
    };
    let value = ctx.memory.get(&port).cloned().unwrap_or(Value::Int(0));
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, value.clone());
    }
    Ok(Some(value))
}

fn handle_io_out(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (port, value) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let port = resolve_src(frame, &instr.srcs, 0)?.as_i64().unwrap_or(0);
        let value = resolve_src(frame, &instr.srcs, 1)?;
        (port, value)
    };
    // Apply the same memory-entry cap as store_mem.
    if !ctx.memory.contains_key(&port) && ctx.memory.len() >= ctx.max_memory_entries {
        return Err(VMError::Custom(format!(
            "memory entry limit {} exceeded", ctx.max_memory_entries
        )));
    }
    ctx.memory.insert(port, value);
    Ok(None)
}

// Coercions -----------------------------------------------------------------

fn handle_cast(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (value, target) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let value = resolve_src(frame, &instr.srcs, 0)?;
        let target = instr.srcs.get(1)
            .and_then(|s| s.as_var())
            .unwrap_or(instr.type_hint.as_str())
            .to_string();
        (value, target)
    };
    let result = match target.as_str() {
        "u8"       => Value::Int(value.as_i64().unwrap_or(0) & 0xFF),
        "u16"      => Value::Int(value.as_i64().unwrap_or(0) & 0xFFFF),
        "u32"      => Value::Int(value.as_i64().unwrap_or(0) & 0xFFFF_FFFF),
        "u64"|"i64" => Value::Int(value.as_i64().unwrap_or(0)),
        "bool"     => Value::Bool(value.is_truthy()),
        "str"      => Value::Str(value.to_string()),
        _          => value,
    };
    if let Some(dest) = &instr.dest {
        ctx.frames.last_mut().unwrap().assign(dest, result.clone());
    }
    Ok(Some(result))
}

fn handle_type_assert(ctx: &mut DispatchCtx, instr: &IIRInstr) -> Result<Option<Value>, VMError> {
    let (fn_name, value, expected) = {
        let frame = ctx.frames.last().ok_or_else(|| VMError::Custom("no frame".into()))?;
        let value = resolve_src(frame, &instr.srcs, 0)?;
        let expected = instr.srcs.get(1)
            .and_then(|s| s.as_var())
            .unwrap_or(instr.type_hint.as_str())
            .to_string();
        (frame.fn_name.clone(), value, expected)
    };
    let actual = value.iir_type_name();
    if actual != expected {
        Err(VMError::TypeError {
            expected,
            actual: actual.to_string(),
            context: fn_name,
        })
    } else {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Standard opcode dispatch table
// ---------------------------------------------------------------------------

type StdHandlerFn = fn(&mut DispatchCtx<'_>, &IIRInstr) -> Result<Option<Value>, VMError>;

pub(crate) fn lookup_standard(op: &str) -> Option<StdHandlerFn> {
    match op {
        "const"        => Some(handle_const),
        "add"          => Some(handle_add),
        "sub"          => Some(handle_sub),
        "mul"          => Some(handle_mul),
        "div"          => Some(handle_div),
        "mod"          => Some(handle_mod),
        "neg"          => Some(handle_neg),
        "and"          => Some(handle_and),
        "or"           => Some(handle_or),
        "xor"          => Some(handle_xor),
        "not"          => Some(handle_not),
        "shl"          => Some(handle_shl),
        "shr"          => Some(handle_shr),
        "cmp_eq"       => Some(handle_cmp_eq),
        "cmp_ne"       => Some(handle_cmp_ne),
        "cmp_lt"       => Some(handle_cmp_lt),
        "cmp_le"       => Some(handle_cmp_le),
        "cmp_gt"       => Some(handle_cmp_gt),
        "cmp_ge"       => Some(handle_cmp_ge),
        "label"        => Some(handle_label),
        "jmp"          => Some(handle_jmp),
        "jmp_if_true"  => Some(handle_jmp_if_true),
        "jmp_if_false" => Some(handle_jmp_if_false),
        "ret"          => Some(handle_ret),
        "ret_void"     => Some(handle_ret_void),
        "load_reg"     => Some(handle_load_reg),
        "store_reg"    => Some(handle_store_reg),
        "load_mem"     => Some(handle_load_mem),
        "store_mem"    => Some(handle_store_mem),
        "call_builtin" => Some(handle_call_builtin),
        "io_in"        => Some(handle_io_in),
        "io_out"       => Some(handle_io_out),
        "cast"         => Some(handle_cast),
        "type_assert"  => Some(handle_type_assert),
        // `call` is handled specially (needs jit_handlers) — see dispatch loop.
        _              => None,
    }
}

// ---------------------------------------------------------------------------
// Main dispatch loop
// ---------------------------------------------------------------------------

/// Run instructions until the frame stack is empty.
///
/// Returns the value produced by the last `ret` / `ret_void` in the root
/// frame, or `None` if the function body was empty.
pub(crate) fn run_dispatch_loop(
    ctx: &mut DispatchCtx<'_>,
    extra_opcodes: &HashMap<String, OpcodeHandler>,
    jit_handlers: &HashMap<String, Box<dyn Fn(&[Value]) -> Value + Send + Sync>>,
    profiler: &mut Option<crate::profiler::VMProfiler>,
) -> Result<Option<Value>, VMError> {
    let mut return_value: Option<Value> = None;

    loop {
        // Peek at the top frame without taking a long-lived borrow.
        let (fn_name, ip) = match ctx.frames.last() {
            Some(f) => (f.fn_name.clone(), f.ip),
            None    => break,
        };

        // Find the function in the module.
        let fn_idx = ctx.module_fns.iter().position(|f| f.name == fn_name)
            .ok_or_else(|| VMError::Custom(format!("function {fn_name:?} not in module")))?;

        // Check end-of-function.
        if ip >= ctx.module_fns[fn_idx].instructions.len() {
            ctx.frames.pop();
            continue;
        }

        // Clone the instruction to release the module borrow before dispatch.
        // The clone is O(small) — a few strings and a short Vec.
        let instr = ctx.module_fns[fn_idx].instructions[ip].clone();

        // Snapshot registers *before* dispatch (only on execute_traced path).
        let registers_before: Vec<Value> = if ctx.tracer.is_some() {
            ctx.frames.last()
                .map(|f| f.registers.clone())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let frame_depth_before = ctx.frames.len().saturating_sub(1);

        // Advance IP before dispatch so branch handlers can overwrite it.
        ctx.frames.last_mut().unwrap().ip += 1;

        // Dispatch — choose handler in priority order:
        //   1. language-specific extra opcode
        //   2. `call` (needs jit_handlers)
        //   3. standard opcode
        let result = if let Some(extra) = extra_opcodes.get(&instr.op) {
            // extra is borrowed from extra_opcodes (a separate &HashMap reference).
            // ctx is &mut DispatchCtx — no overlap.
            extra(ctx, jit_handlers, &instr)?
        } else if instr.op == "call" {
            // `call` is special: it needs jit_handlers as a separate parameter.
            handle_call(ctx, jit_handlers, &instr)?
        } else if let Some(std_handler) = lookup_standard(&instr.op) {
            std_handler(ctx, &instr)?
        } else {
            return Err(VMError::UnknownOpcode(instr.op.clone()));
        };

        *ctx.metrics_instrs += 1;

        // Enforce per-execution instruction budget (sandbox / untrusted-IIR mode).
        // An IIR program with an unconditional backward jump runs forever without
        // this guard.  `None` means unlimited (suitable for trusted, compiled code).
        if let Some(limit) = ctx.max_instructions {
            if *ctx.metrics_instrs >= limit {
                return Err(VMError::Custom(format!(
                    "execution step limit {limit} exceeded — possible infinite loop"
                )));
            }
        }

        // Profile — update the feedback slot on the original instruction.
        // We need to re-acquire &mut IIRInstr from the module.
        if let (Some(prof), Some(result_val)) = (profiler.as_mut(), result.as_ref()) {
            if instr.dest.is_some() && instr.type_hint == "any" {
                if let Some(fn_) = ctx.module_fns.iter_mut().find(|f| f.name == fn_name) {
                    if ip < fn_.instructions.len() {
                        prof.observe(&mut fn_.instructions[ip], result_val);
                    }
                }
            }
        }

        // Instruction trace (execute_traced path only).
        if let Some(tracer) = ctx.tracer.as_deref_mut() {
            let registers_after = ctx.frames.last()
                .map(|f| f.registers.clone())
                .unwrap_or_default();
            tracer.push(crate::trace::VMTrace {
                frame_depth: frame_depth_before,
                fn_name: fn_name.clone(),
                ip,
                instr: instr.clone(),
                registers_before,
                registers_after,
            });
        }

        // Track return values.
        if matches!(instr.op.as_str(), "ret" | "ret_void") {
            return_value = result;
        }
    }

    Ok(return_value)
}
