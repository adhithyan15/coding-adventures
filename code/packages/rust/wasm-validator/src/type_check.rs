//! # Instruction-level type checking (WASM06 / W02 Phase 2)
//!
//! Everything module-structural validation ([`crate::validate`]) can't catch:
//! whether every instruction *sequence* is actually well-typed. This is an
//! **abstract interpretation** of each function body — the same tree-walk a
//! real execution would do, except the operand stack holds *types*
//! (`I32`, `F64`, ...) instead of concrete values.
//!
//! ```text
//! Concrete execution (wasm-execution):        Abstract execution (this module):
//!   i32.const 3   -> stack: [3]                  i32.const 3   -> stack: [I32]
//!   i32.const 5   -> stack: [3, 5]                i32.const 5   -> stack: [I32, I32]
//!   i32.add       -> stack: [8]                   i32.add       -> stack: [I32]
//! ```
//!
//! ## The control frame stack
//!
//! WASM's branches all target a lexically enclosing `block`/`loop`/`if` --
//! there is no arbitrary jump. This module tracks one [`ControlFrame`] per
//! open scope, each recording the value-stack height at entry (so a branch
//! can never reach below its own scope) and the two type lists a branch
//! needs depending on which way it targets this frame:
//!
//! - `start_types` -- what a branch to a **`loop`'s START** needs (re-entry
//!   re-consumes its params).
//! - `end_types` -- what a branch to a **`block`/`if`'s END** needs (also
//!   what a normal fall-through, or the function's own `end`, needs).
//!
//! ## Dead code and the `Unknown` type
//!
//! After `br`/`br_if` unconditionally taken/`return`/`unreachable`, the
//! *rest of that block* is unreachable -- but it's still bytes that must be
//! walked. WASM permits *any* stack shape in dead code, so once a frame is
//! marked unreachable, every pop from it returns [`StackType::Unknown`]
//! (compatible with anything) instead of type-checking or underflowing,
//! regardless of what's really on the stack. This is a deliberately
//! stronger rule than the literal `len(stack) <= frame.stack_height`
//! wording in `W02-wasm-validator.md`'s own pseudocode (which would still
//! strictly type-check any real values sitting above the frame's floor at
//! the moment reachability was lost) -- that reading rejects the spec's
//! *own* worked example (`f32.const 3.14` then `i64.add` inside dead code),
//! since the popped `f32.const` really is on the stack and really doesn't
//! match `i64.add`'s expected `I64`. The rule implemented here (`Unknown`
//! unconditionally while a frame is unreachable) is what real engines
//! implement and is the reading that makes that example type-check. See
//! `pop_val` below.

use wasm_leb128::{decode_signed, decode_unsigned};
use wasm_opcodes::get_opcode;
use wasm_types::{FuncType, FunctionBody, GlobalType, ValueType, WasmModule};

use crate::ValidationError;

// ──────────────────────────────────────────────────────────────────────────────
// Abstract value stack
// ──────────────────────────────────────────────────────────────────────────────

/// One entry on the abstract value stack: either a concrete type, or
/// [`Unknown`](StackType::Unknown) -- the polymorphic placeholder used
/// inside dead code (see the module doc comment).
#[derive(Debug, Clone, Copy, PartialEq)]
enum StackType {
    Known(ValueType),
    Unknown,
}

// ──────────────────────────────────────────────────────────────────────────────
// Control frame stack
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum FrameKind {
    Block,
    Loop,
    If,
}

/// One entry on the control frame stack -- one open `block`/`loop`/`if`
/// scope, or (at index 0) the function body's own implicit outer scope.
#[derive(Debug, Clone)]
struct ControlFrame {
    kind: FrameKind,
    /// What a branch to this frame's START needs (a `loop`'s declared
    /// params -- re-entering the loop re-consumes them).
    start_types: Vec<ValueType>,
    /// What a branch to this frame's END needs, and what must be on the
    /// stack when this frame's own `end` is reached.
    end_types: Vec<ValueType>,
    /// The value-stack height when this frame was entered (after popping
    /// its params from the enclosing stack, before re-pushing them as the
    /// frame's own initial content -- see `push_ctrl`). A pop can never
    /// reach below this height while the frame is reachable.
    stack_height: usize,
    /// Set once an unconditional branch/return/`unreachable` instruction
    /// executes within this frame. See the module doc comment.
    unreachable: bool,
    /// Whether this `If` frame has already seen its `else`. Determines
    /// whether a second `else` is an error, and whether the "no else
    /// implies start_types == end_types" rule applies at `end`.
    saw_else: bool,
}

/// What a branch (`br`/`br_if`/`br_table`) targeting `frame` needs on the
/// stack -- `start_types` for a loop (branch re-enters its start), `end_types`
/// for a block/if (branch exits at its end). See the module doc comment.
fn label_types(frame: &ControlFrame) -> &[ValueType] {
    if frame.kind == FrameKind::Loop {
        &frame.start_types
    } else {
        &frame.end_types
    }
}

/// Resolve a `br`/`br_if`/`br_table` `depth` immediate (an attacker-
/// controlled `u32` read straight from the bytecode) to an index into
/// `control_stack`, or `None` if it's out of range. Uses `checked_add`
/// before the `checked_sub`, not a plain `1 + depth as usize`, so this
/// can't silently overflow on a 32-bit `usize` target when `depth` is
/// `u32::MAX`.
fn resolve_label_target(control_stack_len: usize, depth: u32) -> Option<usize> {
    control_stack_len.checked_sub((depth as usize).checked_add(1)?)
}

// ──────────────────────────────────────────────────────────────────────────────
// Stack primitives
// ──────────────────────────────────────────────────────────────────────────────

/// Pop one value from `stack`, honoring `frame`'s dead-code polymorphism.
///
/// While `frame.unreachable`, every pop returns `Unknown` -- if a real
/// value happens to be above `frame.stack_height` it's discarded (keeping
/// the stack from growing unboundedly across a long dead region), but its
/// type is never compared against anything. Once reachable again, popping
/// at or below `frame.stack_height` is a genuine `StackUnderflow`.
fn pop_val(stack: &mut Vec<StackType>, frame: &ControlFrame) -> Result<StackType, ValidationError> {
    if frame.unreachable {
        if stack.len() > frame.stack_height {
            stack.pop();
        }
        return Ok(StackType::Unknown);
    }
    if stack.len() <= frame.stack_height {
        return Err(ValidationError::Other("StackUnderflow: not enough operands".to_string()));
    }
    Ok(stack.pop().unwrap())
}

/// Pop one value and require it to match `expected` (an `Unknown` actual
/// or expected always matches -- see [`pop_val`]).
fn pop_expect(stack: &mut Vec<StackType>, frame: &ControlFrame, expected: ValueType) -> Result<(), ValidationError> {
    match pop_val(stack, frame)? {
        StackType::Unknown => Ok(()),
        StackType::Known(actual) if actual == expected => Ok(()),
        StackType::Known(actual) => Err(ValidationError::Other(format!(
            "TypeMismatch: expected {expected:?}, found {actual:?}"
        ))),
    }
}

/// Pop and verify a whole type list, in reverse (the last-listed type is
/// on top of the stack -- e.g. `store`'s `[I32, T]` pops `T` first).
fn pop_expect_many(stack: &mut Vec<StackType>, frame: &ControlFrame, expected: &[ValueType]) -> Result<(), ValidationError> {
    for &t in expected.iter().rev() {
        pop_expect(stack, frame, t)?;
    }
    Ok(())
}

fn push_val(stack: &mut Vec<StackType>, t: ValueType) {
    stack.push(StackType::Known(t));
}

fn push_vals(stack: &mut Vec<StackType>, ts: &[ValueType]) {
    for &t in ts {
        push_val(stack, t);
    }
}

/// Enter dead code: truncate to this (innermost) frame's own floor and
/// mark it unreachable. Called by `unreachable`, `br`, and `return`.
fn mark_unreachable(stack: &mut Vec<StackType>, frame: &mut ControlFrame) {
    stack.truncate(frame.stack_height);
    frame.unreachable = true;
}

/// Open a new control frame: pop its params off the enclosing stack
/// (verifying they're really there), record the height *below* them, then
/// push them back as the new frame's own initial content. This is the
/// same pop-then-repush shape the WASM spec's own reference validation
/// algorithm uses -- `stack_height` ends up excluding the frame's own
/// params, so a branch back out to this exact height (e.g. an
/// unconditional branch inside the frame) correctly wipes them too.
fn push_ctrl(
    stack: &mut Vec<StackType>,
    control_stack: &mut Vec<ControlFrame>,
    kind: FrameKind,
    start_types: Vec<ValueType>,
    end_types: Vec<ValueType>,
) -> Result<(), ValidationError> {
    let outer = control_stack.last().cloned();
    // The enclosing frame is the "current" one for the purposes of popping
    // these params off of it (they live in the enclosing scope until this
    // call moves them into the new one).
    if let Some(outer_frame) = &outer {
        pop_expect_many(stack, outer_frame, &start_types)?;
    } else {
        pop_expect_many(
            stack,
            &ControlFrame {
                kind: FrameKind::Block,
                start_types: vec![],
                end_types: vec![],
                stack_height: 0,
                unreachable: false,
                saw_else: false,
            },
            &start_types,
        )?;
    }
    let stack_height = stack.len();
    control_stack.push(ControlFrame {
        kind,
        start_types: start_types.clone(),
        end_types,
        stack_height,
        unreachable: false,
        saw_else: false,
    });
    push_vals(stack, &start_types);
    Ok(())
}

/// Close the innermost control frame: verify its `end_types` are on top of
/// the stack (dead-code-tolerant), require nothing extra is left over
/// (skipped while unreachable -- dead code may leave any shape, so it's
/// simply flushed down to the frame's own floor instead), then pop it.
fn pop_ctrl(stack: &mut Vec<StackType>, control_stack: &mut Vec<ControlFrame>) -> Result<ControlFrame, ValidationError> {
    let frame = control_stack
        .last()
        .cloned()
        .ok_or_else(|| ValidationError::Other("unexpected `end`/`else`: no open block".to_string()))?;
    pop_expect_many(stack, &frame, &frame.end_types)?;
    if frame.unreachable {
        stack.truncate(frame.stack_height);
    } else if stack.len() != frame.stack_height {
        return Err(ValidationError::Other(format!(
            "{} extra value(s) left on the stack at block end",
            stack.len() - frame.stack_height
        )));
    }
    control_stack.pop();
    Ok(frame)
}

// ──────────────────────────────────────────────────────────────────────────────
// Blocktype resolution (WASM06/WASM04's multi-value encoding)
// ──────────────────────────────────────────────────────────────────────────────

/// Decode a `block`/`loop`/`if` header's blocktype immediate at `code[offset]`,
/// returning `(params, results, bytes_consumed)`. Mirrors `wasm-execution`'s
/// `block_arity` (single byte for empty/one-result, else a real type-section
/// index -- see that function's own doc comment for the `ctx.types`-not-
/// `ctx.func_types` history this crate has no analogous bug to repeat, since
/// it's never given anything BUT the real type section).
fn decode_blocktype(module: &WasmModule, code: &[u8], offset: usize) -> Result<(Vec<ValueType>, Vec<ValueType>, usize), ValidationError> {
    let byte = *code
        .get(offset)
        .ok_or_else(|| ValidationError::Other("truncated blocktype immediate".to_string()))?;
    match byte {
        0x40 => Ok((vec![], vec![], 1)),
        0x7F => Ok((vec![], vec![ValueType::I32], 1)),
        0x7E => Ok((vec![], vec![ValueType::I64], 1)),
        0x7D => Ok((vec![], vec![ValueType::F32], 1)),
        0x7C => Ok((vec![], vec![ValueType::F64], 1)),
        _ => {
            let (idx, size) = decode_signed(code, offset).map_err(|e| ValidationError::Other(format!("bad blocktype immediate: {e}")))?;
            let ty = module
                .types
                .get(idx as usize)
                .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("blocktype references type index {idx}, but only {} types exist", module.types.len())))?;
            Ok((ty.params.clone(), ty.results.clone(), size))
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Immediate decoding helpers (index-only; values are never needed for typing)
// ──────────────────────────────────────────────────────────────────────────────

fn decode_idx(code: &[u8], offset: usize) -> Result<(u32, usize), ValidationError> {
    let (v, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad index immediate: {e}")))?;
    Ok((v as u32, size))
}

/// The alignment (in `log2` bytes) a memory instruction with `access_bytes`-
/// wide natural alignment permits at most -- part of W02 §2.6's memarg rule.
fn max_align_for(access_bytes: u32) -> u32 {
    access_bytes.trailing_zeros()
}

/// The declared field count of the WasmGC struct type at type-section index
/// `type_idx` -- how many values `struct.new` pops. Struct types live after
/// the function types in the combined type index space: struct type `k` is
/// at type-section index `module.types.len() + k` (see `WasmModule::struct_types`'s
/// own doc comment).
fn struct_field_count(module: &WasmModule, type_idx: u32) -> Result<usize, ValidationError> {
    let struct_idx = (type_idx as usize)
        .checked_sub(module.types.len())
        .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("struct.new type index {type_idx} is a function type, not a struct type")))?;
    let st = module
        .struct_types
        .get(struct_idx)
        .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("struct.new references struct type index {type_idx}, but only {} struct types exist", module.struct_types.len())))?;
    Ok(st.fields.len())
}

// ──────────────────────────────────────────────────────────────────────────────
// Numeric instruction type rule (covers ~130 opcodes via one generic rule)
// ──────────────────────────────────────────────────────────────────────────────

/// True for a numeric instruction whose result is always `I32` regardless
/// of its operand type -- `eqz` and every comparison (`eq`/`ne`/`lt_*`/
/// `gt_*`/`le_*`/`ge_*`/`lt`/`gt`/`le`/`ge`). Matches W02 §2.6's numeric
/// table: "comparison instructions always produce I32... regardless of
/// whether they compare integers or floats."
fn is_boolean_result_numeric(suffix: &str) -> bool {
    matches!(
        suffix,
        "eqz" | "eq" | "ne" | "lt_s" | "lt_u" | "lt" | "gt_s" | "gt_u" | "gt" | "le_s" | "le_u" | "le" | "ge_s" | "ge_u" | "ge"
    )
}

/// Apply a numeric (`numeric_i32`/`numeric_i64`/`numeric_f32`/`numeric_f64`)
/// instruction's type rule.
///
/// The operand type is always derivable from the instruction's own name
/// prefix (`i32.`/`i64.`/`f32.`/`f64.`); `stack_pop`/`stack_push` (0 for a
/// `*.const`, 1 for unary/`eqz`, 2 for binary/comparison) already come from
/// `wasm-opcodes`' metadata, so this one rule covers the whole family
/// instead of ~130 individually hand-listed opcodes.
fn type_check_numeric(stack: &mut Vec<StackType>, frame: &ControlFrame, name: &str, stack_pop: u8) -> Result<(), ValidationError> {
    let (prefix, suffix) = name.split_once('.').ok_or_else(|| ValidationError::Other(format!("malformed numeric opcode name {name:?}")))?;
    let operand_type = match prefix {
        "i32" => ValueType::I32,
        "i64" => ValueType::I64,
        "f32" => ValueType::F32,
        "f64" => ValueType::F64,
        _ => return Err(ValidationError::Other(format!("unrecognized numeric opcode {name:?}"))),
    };
    if stack_pop == 0 {
        // *.const
        push_val(stack, operand_type);
        return Ok(());
    }
    for _ in 0..stack_pop {
        pop_expect(stack, frame, operand_type)?;
    }
    let result_type = if is_boolean_result_numeric(suffix) { ValueType::I32 } else { operand_type };
    push_val(stack, result_type);
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Conversion instruction type rule (W02 §2.6 "Conversion Instructions")
// ──────────────────────────────────────────────────────────────────────────────

/// `(input, output)` for every conversion opcode, including the "still
/// single-byte" sign-extension proposal (`i32.extend8_s` etc., WASM03) --
/// hand-listed against W02 §2.6's own table rather than derived, since
/// unlike the numeric family the input/output types aren't a regular
/// function of the name alone (e.g. `i32.wrap_i64` takes `I64`, not `I32`).
fn conversion_types(name: &str) -> Option<(ValueType, ValueType)> {
    use ValueType::*;
    Some(match name {
        "i32.wrap_i64" => (I64, I32),
        "i32.trunc_f32_s" | "i32.trunc_f32_u" | "i32.trunc_sat_f32_s" | "i32.trunc_sat_f32_u" => (F32, I32),
        "i32.trunc_f64_s" | "i32.trunc_f64_u" | "i32.trunc_sat_f64_s" | "i32.trunc_sat_f64_u" => (F64, I32),
        "i64.extend_i32_s" | "i64.extend_i32_u" => (I32, I64),
        "i64.trunc_f32_s" | "i64.trunc_f32_u" | "i64.trunc_sat_f32_s" | "i64.trunc_sat_f32_u" => (F32, I64),
        "i64.trunc_f64_s" | "i64.trunc_f64_u" | "i64.trunc_sat_f64_s" | "i64.trunc_sat_f64_u" => (F64, I64),
        "f32.convert_i32_s" | "f32.convert_i32_u" => (I32, F32),
        "f32.convert_i64_s" | "f32.convert_i64_u" => (I64, F32),
        "f32.demote_f64" => (F64, F32),
        "f64.convert_i32_s" | "f64.convert_i32_u" => (I32, F64),
        "f64.convert_i64_s" | "f64.convert_i64_u" => (I64, F64),
        "f64.promote_f32" => (F32, F64),
        "i32.reinterpret_f32" => (F32, I32),
        "i64.reinterpret_f64" => (F64, I64),
        "f32.reinterpret_i32" => (I32, F32),
        "f64.reinterpret_i64" => (I64, F64),
        "i32.extend8_s" | "i32.extend16_s" => (I32, I32),
        "i64.extend8_s" | "i64.extend16_s" | "i64.extend32_s" => (I64, I64),
        _ => return None,
    })
}

// ──────────────────────────────────────────────────────────────────────────────
// Per-function type checking
// ──────────────────────────────────────────────────────────────────────────────

/// Everything needed to resolve an instruction's type rule, computed once
/// per module (not per function) and threaded through.
struct ModuleContext<'a> {
    module: &'a WasmModule,
    /// Combined imported + module-defined function types, indexed by the
    /// combined function index space (imports first, matching every other
    /// index space in the binary format).
    func_types: Vec<FuncType>,
    /// Combined imported + module-defined global types, same index-space
    /// convention as `func_types`.
    global_types: Vec<GlobalType>,
    has_memory: bool,
    /// Combined imported + module-defined table COUNT (WASM17), same
    /// index-space convention as `func_types`/`global_types` -- unlike
    /// `has_memory` (a plain bool, since every memory op hardcodes memory
    /// index 0 and ignores its reserved-byte immediate), `table.get`/
    /// `table.set` decode a REAL `tableidx` immediate that must be
    /// bounds-checked against the actual table count, not just "is there
    /// at least one".
    table_count: u32,
}

fn build_module_context(module: &WasmModule) -> Result<ModuleContext<'_>, ValidationError> {
    use wasm_types::ImportTypeInfo;

    let mut func_types = Vec::new();
    let mut global_types = Vec::new();
    for imp in &module.imports {
        match &imp.type_info {
            ImportTypeInfo::Function(type_idx) => {
                let ty = module
                    .types
                    .get(*type_idx as usize)
                    .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("import {}.{} references type index {type_idx}, but only {} types exist", imp.module_name, imp.name, module.types.len())))?;
                func_types.push(ty.clone());
            }
            ImportTypeInfo::Global(gt) => global_types.push(gt.clone()),
            _ => {}
        }
    }
    for &type_idx in &module.functions {
        let ty = module
            .types
            .get(type_idx as usize)
            .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("function references type index {type_idx}, but only {} types exist", module.types.len())))?;
        func_types.push(ty.clone());
    }
    for g in &module.globals {
        global_types.push(g.global_type.clone());
    }

    let has_memory = !module.memories.is_empty() || module.imports.iter().any(|i| matches!(i.type_info, ImportTypeInfo::Memory(_)));
    let imported_table_count = module.imports.iter().filter(|i| matches!(i.type_info, ImportTypeInfo::Table(_))).count() as u32;
    let table_count = imported_table_count + module.tables.len() as u32;

    Ok(ModuleContext {
        module,
        func_types,
        global_types,
        has_memory,
        table_count,
    })
}

/// Type-check every function body in `module`. The first ill-typed
/// function (by index) determines the error.
pub(crate) fn type_check_module(module: &WasmModule) -> Result<(), ValidationError> {
    let ctx = build_module_context(module)?;
    let imported_function_count = ctx.func_types.len() - module.functions.len();

    for (i, &type_idx) in module.functions.iter().enumerate() {
        let func_idx = imported_function_count + i;
        let func_type = &ctx.func_types[func_idx];
        let body = module
            .code
            .get(i)
            .ok_or_else(|| ValidationError::Other(format!("function #{func_idx} (type {type_idx}) has no matching code entry")))?;
        type_check_function(&ctx, func_idx, func_type, body)?;
    }
    Ok(())
}

fn type_check_function(ctx: &ModuleContext, func_idx: usize, func_type: &FuncType, body: &FunctionBody) -> Result<(), ValidationError> {
    let mut locals = func_type.params.clone();
    locals.extend(body.locals.iter().copied());

    let mut stack: Vec<StackType> = Vec::new();
    // The function body is itself an implicit outer frame; its "kind" is
    // irrelevant (nothing ever branches to depth == number of enclosing
    // blocks except `return`, which reads end_types directly off frame 0,
    // not via `label_types`), so `Block` is a harmless placeholder.
    let mut control_stack: Vec<ControlFrame> = vec![ControlFrame {
        kind: FrameKind::Block,
        start_types: func_type.params.clone(),
        end_types: func_type.results.clone(),
        stack_height: 0,
        unreachable: false,
        saw_else: false,
    }];

    let code = &body.code;
    let mut offset = 0usize;

    while offset < code.len() {
        let byte = code[offset];
        offset += 1;

        macro_rules! err {
            ($($arg:tt)*) => {
                return Err(ValidationError::Other(format!("function #{func_idx}: {}", format!($($arg)*))))
            };
        }
        // Security: defense in depth alongside the `0x0B` handler's own
        // guard above -- if `control_stack` were ever unexpectedly empty
        // here (e.g. from a future bug reintroducing the premature-`end`
        // hole), this returns a clean `ValidationError` instead of
        // panicking. A validator panicking on adversarial bytecode is
        // itself a DoS: the one thing this code must never do is crash on
        // malformed input, only reject it.
        macro_rules! frame {
            () => {
                match control_stack.last() {
                    Some(f) => f,
                    None => return Err(ValidationError::Other(format!("function #{func_idx}: no open block (control stack unexpectedly empty)"))),
                }
            };
        }
        macro_rules! frame_mut {
            () => {
                match control_stack.last_mut() {
                    Some(f) => f,
                    None => return Err(ValidationError::Other(format!("function #{func_idx}: no open block (control stack unexpectedly empty)"))),
                }
            };
        }

        match byte {
            // ── `0xFC`-prefixed saturating conversions and bulk memory ──
            0xFC => {
                let (sub, size) = decode_unsigned(code, offset)
                    .map_err(|e| ValidationError::Other(format!("bad 0xFC sub-opcode: {e}")))?;
                offset += size;
                match sub {
                    0x00..=0x07 => {
                        let name = [
                            "i32.trunc_sat_f32_s",
                            "i32.trunc_sat_f32_u",
                            "i32.trunc_sat_f64_s",
                            "i32.trunc_sat_f64_u",
                            "i64.trunc_sat_f32_s",
                            "i64.trunc_sat_f32_u",
                            "i64.trunc_sat_f64_s",
                            "i64.trunc_sat_f64_u",
                        ][sub as usize];
                        let (input, output) = conversion_types(name)
                            .expect("trunc_sat names are all in conversion_types");
                        pop_expect(&mut stack, frame!(), input)?;
                        push_val(&mut stack, output);
                    }
                    0x0A => {
                        if !ctx.has_memory {
                            err!("memory.copy requires a declared memory");
                        }
                        let (dst_memory, dst_size) = decode_idx(code, offset)?;
                        let (src_memory, src_size) = decode_idx(code, offset + dst_size)?;
                        offset += dst_size + src_size;
                        if dst_memory != 0 || src_memory != 0 {
                            err!("memory.copy references unsupported nonzero memory index");
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // length
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // source
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // destination
                    }
                    0x0B => {
                        if !ctx.has_memory {
                            err!("memory.fill requires a declared memory");
                        }
                        let (memory, memory_size) = decode_idx(code, offset)?;
                        offset += memory_size;
                        if memory != 0 {
                            err!("memory.fill references unsupported nonzero memory index");
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // length
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // byte value
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // destination
                    }
                    other => err!("unsupported 0xFC sub-opcode {other:#x}"),
                }
            }

            // ── `0xFB`-prefixed WasmGC opcodes -- out of W02 Phase 2's MVP-only
            // scope, but this repo already has real modules that use this
            // crate's own small, closed set of them (struct/i31/ref.test), so
            // they're decoded enough to stay byte-in-sync AND keep the
            // abstract stack's HEIGHT accurate (an MVP instruction later in
            // the same function must still see the right slot), using
            // `Unknown` for whatever a GC op pushes since real reference-type
            // subtyping isn't implemented here. Mirrors wasm-execution's own
            // `decode_function_body` sub-opcode table for byte layout. ──────
            0xFB => {
                let sub = *code.get(offset).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: truncated 0xFB opcode")))?;
                offset += 1;
                match sub {
                    0x00 => {
                        // struct.new <type_idx>: pops one value per declared
                        // field, pushes one structref.
                        let (type_idx, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad struct.new type index: {e}")))?;
                        offset += size;
                        for _ in 0..struct_field_count(ctx.module, type_idx as u32)? {
                            pop_val(&mut stack, frame!())?;
                        }
                        stack.push(StackType::Unknown);
                    }
                    0x02 => {
                        // struct.get <type_idx> <field_idx>: pops structref,
                        // pushes the field's value.
                        let (_, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad struct.get type index: {e}")))?;
                        let (_, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad struct.get field index: {e}")))?;
                        offset += sz1 + sz2;
                        pop_val(&mut stack, frame!())?;
                        stack.push(StackType::Unknown);
                    }
                    0x04 => {
                        // struct.set <type_idx> <field_idx>: pops the new
                        // value and the structref.
                        let (_, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad struct.set type index: {e}")))?;
                        let (_, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad struct.set field index: {e}")))?;
                        offset += sz1 + sz2;
                        pop_val(&mut stack, frame!())?;
                        pop_val(&mut stack, frame!())?;
                    }
                    0x14 | 0x15 => {
                        // ref.test / ref.test null <heap_type>: pops a ref,
                        // pushes an I32 boolean.
                        let (_, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad ref.test heap type: {e}")))?;
                        offset += size;
                        pop_val(&mut stack, frame!())?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    0x1C => {
                        // i31.new: pops I32, pushes i31ref.
                        pop_expect(&mut stack, frame!(), ValueType::I32)?;
                        stack.push(StackType::Unknown);
                    }
                    0x1D => {
                        // i31.get_s: pops i31ref, pushes I32.
                        pop_val(&mut stack, frame!())?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    _ => {} // unknown sub-opcode: no immediates, no stack effect
                }
            }
            0xD0 => {
                // ref.null <heap_type byte>: pushes a null reference.
                // WASM17 upgrade: push the REAL static type for the two
                // heap types this repo's `wasm-wast-parser` actually emits
                // (`func` = 0x70, `extern` = 0x6F, and the pre-existing bare
                // `ref.null` convention `none` = 0x0F, which stays Anyref)
                // instead of the previously-unconditional `Unknown` -- this
                // is what lets `select`/`global.set`/etc.'s existing
                // type-mismatch checks catch a funcref-vs-externref mixup,
                // which they couldn't when both looked like the same
                // `Unknown`. Any other heap-type byte (a concrete `$t`
                // reference, out of this repo's scope) still falls back to
                // `Unknown` -- full subtyping remains outside this
                // validator phase, same as every other GC reference type.
                let heap_type = *code.get(offset).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: truncated ref.null heap-type immediate")))?;
                offset += 1;
                match heap_type {
                    0x70 => push_val(&mut stack, ValueType::Funcref),
                    0x6F => push_val(&mut stack, ValueType::Externref),
                    0x0F => push_val(&mut stack, ValueType::Anyref),
                    _ => stack.push(StackType::Unknown),
                }
            }
            0xD1 => {
                // ref.is_null: accepts any reference and produces an i32.
                // Reference subtyping remains outside this validator phase,
                // so ref-producing instructions use Unknown on the stack.
                pop_val(&mut stack, frame!())?;
                push_val(&mut stack, ValueType::I32);
            }
            0xD2 => {
                // ref.func <funcidx> (WASM17): pushes a non-null funcref
                // referring to a function by index. Bounds-checked the same
                // way `call`'s type rule above checks its own funcidx.
                let (callee, size) = decode_idx(code, offset)?;
                offset += size;
                if ctx.func_types.get(callee as usize).is_none() {
                    return Err(ValidationError::FuncIndexOutOfBounds(format!(
                        "function #{func_idx}: ref.func references function index {callee}, but only {} functions exist",
                        ctx.func_types.len()
                    )));
                }
                push_val(&mut stack, ValueType::Funcref);
            }

            // ── Control ──────────────────────────────────────────────────────
            0x00 => mark_unreachable(&mut stack, frame_mut!()), // unreachable
            0x01 => {}                                          // nop
            0x02 => {
                // block
                let (params, results, size) = decode_blocktype(ctx.module, code, offset)?;
                offset += size;
                push_ctrl(&mut stack, &mut control_stack, FrameKind::Block, params, results)?;
            }
            0x03 => {
                // loop
                let (params, results, size) = decode_blocktype(ctx.module, code, offset)?;
                offset += size;
                push_ctrl(&mut stack, &mut control_stack, FrameKind::Loop, params, results)?;
            }
            0x04 => {
                // if
                let (params, results, size) = decode_blocktype(ctx.module, code, offset)?;
                offset += size;
                pop_expect(&mut stack, frame!(), ValueType::I32)?; // condition
                push_ctrl(&mut stack, &mut control_stack, FrameKind::If, params, results)?;
            }
            0x05 => {
                // else
                let closed = pop_ctrl(&mut stack, &mut control_stack)?;
                if closed.kind != FrameKind::If || closed.saw_else {
                    err!("unexpected `else` (not inside an `if`, or `if` already had one)");
                }
                // NOT `push_ctrl`: that pops `start_types` off the ENCLOSING
                // scope again, but they were already consumed once when the
                // original `if` opened -- the else-branch reuses the exact
                // same params, it doesn't need the enclosing code to supply
                // a second copy. `pop_ctrl` just above already verified the
                // stack is back down to `closed.stack_height`, so this only
                // needs to re-push the same start_types on top of that.
                let stack_height = closed.stack_height;
                let start_types = closed.start_types;
                push_vals(&mut stack, &start_types);
                control_stack.push(ControlFrame {
                    kind: FrameKind::If,
                    start_types,
                    end_types: closed.end_types,
                    stack_height,
                    unreachable: false,
                    saw_else: true,
                });
            }
            0x0B => {
                // end
                //
                // Security: `control_stack` always starts with exactly one
                // entry (the function body's own implicit outer block),
                // meant to be closed by exactly one matching `end` -- the
                // LAST byte of a well-formed body. Without this guard, a
                // crafted body could close that outer frame early (e.g. a
                // 2-byte `[0x0B, X]` body for any func with empty declared
                // results) and empty `control_stack` while bytes remain,
                // making every later opcode handler's `frame!()`/
                // `frame_mut!()` -- and `return`'s own `control_stack[0]`
                // read -- panic instead of cleanly rejecting the module.
                if control_stack.len() == 1 && offset != code.len() {
                    err!(
                        "unexpected `end`: closes the function's own implicit outer block before the end of the function body ({} trailing byte(s))",
                        code.len() - offset
                    );
                }
                let closed = pop_ctrl(&mut stack, &mut control_stack)?;
                if closed.kind == FrameKind::If && !closed.saw_else && closed.start_types != closed.end_types {
                    err!("`if` without a matching `else` must have identical param and result types");
                }
                push_vals(&mut stack, &closed.end_types);
            }
            0x0C => {
                // br
                let (depth, size) = decode_idx(code, offset)?;
                offset += size;
                let target = resolve_label_target(control_stack.len(), depth)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: br target {depth} out of range")))?;
                let types = label_types(&control_stack[target]).to_vec();
                pop_expect_many(&mut stack, frame!(), &types)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x0D => {
                // br_if
                let (depth, size) = decode_idx(code, offset)?;
                offset += size;
                pop_expect(&mut stack, frame!(), ValueType::I32)?; // condition
                let target = resolve_label_target(control_stack.len(), depth)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: br_if target {depth} out of range")))?;
                let types = label_types(&control_stack[target]).to_vec();
                pop_expect_many(&mut stack, frame!(), &types)?;
                push_vals(&mut stack, &types); // not taken: stack preserved
            }
            0x0E => {
                // br_table: vec(labelidx) + default labelidx
                let (count, mut size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad br_table count: {e}")))?;
                let mut labels = Vec::with_capacity(count.min(4096) as usize);
                for _ in 0..count {
                    let (label, sz) = decode_idx(code, offset + size)?;
                    labels.push(label);
                    size += sz;
                }
                let (default_label, sz) = decode_idx(code, offset + size)?;
                size += sz;
                offset += size;

                pop_expect(&mut stack, frame!(), ValueType::I32)?; // index
                let default_target = resolve_label_target(control_stack.len(), default_label)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: br_table default target {default_label} out of range")))?;
                let default_types = label_types(&control_stack[default_target]).to_vec();
                for &label in &labels {
                    let target = resolve_label_target(control_stack.len(), label)
                        .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: br_table target {label} out of range")))?;
                    let types = label_types(&control_stack[target]).to_vec();
                    if types.len() != default_types.len() {
                        err!("br_table targets have mismatched arities ({} vs default's {})", types.len(), default_types.len());
                    }
                    // Verify without permanently consuming (every target
                    // must type-check against the SAME current stack).
                    pop_expect_many(&mut stack, frame!(), &types)?;
                    push_vals(&mut stack, &types);
                }
                pop_expect_many(&mut stack, frame!(), &default_types)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x0F => {
                // return
                let results = control_stack
                    .first()
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: return with no open block")))?
                    .end_types
                    .clone();
                pop_expect_many(&mut stack, frame!(), &results)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x10 => {
                // call
                let (callee, size) = decode_idx(code, offset)?;
                offset += size;
                let callee_type = ctx
                    .func_types
                    .get(callee as usize)
                    .ok_or_else(|| ValidationError::FuncIndexOutOfBounds(format!("function #{func_idx}: call references function index {callee}, but only {} functions exist", ctx.func_types.len())))?;
                pop_expect_many(&mut stack, frame!(), &callee_type.params)?;
                push_vals(&mut stack, &callee_type.results);
            }
            0x11 => {
                // call_indirect: typeidx, tableidx
                let (type_idx, sz1) = decode_idx(code, offset)?;
                let (_table_idx, sz2) = decode_idx(code, offset + sz1)?;
                offset += sz1 + sz2;
                let callee_type = ctx
                    .module
                    .types
                    .get(type_idx as usize)
                    .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("function #{func_idx}: call_indirect references type index {type_idx}, but only {} types exist", ctx.module.types.len())))?;
                pop_expect(&mut stack, frame!(), ValueType::I32)?; // table index
                pop_expect_many(&mut stack, frame!(), &callee_type.params)?;
                push_vals(&mut stack, &callee_type.results);
            }

            // ── Parametric ───────────────────────────────────────────────────
            0x1A => {
                pop_val(&mut stack, frame!())?; // drop: any type
            }
            0x1B => {
                // select
                pop_expect(&mut stack, frame!(), ValueType::I32)?; // condition
                let t2 = pop_val(&mut stack, frame!())?;
                let t1 = pop_val(&mut stack, frame!())?;
                let result = match (t1, t2) {
                    (StackType::Unknown, StackType::Unknown) => StackType::Unknown,
                    (StackType::Unknown, k @ StackType::Known(_)) | (k @ StackType::Known(_), StackType::Unknown) => k,
                    (StackType::Known(a), StackType::Known(b)) if a == b => StackType::Known(a),
                    (StackType::Known(a), StackType::Known(b)) => err!("select operands have different types ({a:?} vs {b:?})"),
                };
                stack.push(result);
            }

            // ── Variable ─────────────────────────────────────────────────────
            0x20 => {
                // local.get
                let (idx, size) = decode_idx(code, offset)?;
                offset += size;
                let ty = *locals.get(idx as usize).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: local.get index {idx} out of bounds ({} locals)", locals.len())))?;
                push_val(&mut stack, ty);
            }
            0x21 => {
                // local.set
                let (idx, size) = decode_idx(code, offset)?;
                offset += size;
                let ty = *locals.get(idx as usize).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: local.set index {idx} out of bounds ({} locals)", locals.len())))?;
                pop_expect(&mut stack, frame!(), ty)?;
            }
            0x22 => {
                // local.tee
                let (idx, size) = decode_idx(code, offset)?;
                offset += size;
                let ty = *locals.get(idx as usize).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: local.tee index {idx} out of bounds ({} locals)", locals.len())))?;
                pop_expect(&mut stack, frame!(), ty)?;
                push_val(&mut stack, ty);
            }
            0x23 => {
                // global.get
                let (idx, size) = decode_idx(code, offset)?;
                offset += size;
                let gt = ctx
                    .global_types
                    .get(idx as usize)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: global.get index {idx} out of bounds ({} globals)", ctx.global_types.len())))?
                    .clone();
                push_val(&mut stack, gt.value_type);
            }
            0x24 => {
                // global.set
                let (idx, size) = decode_idx(code, offset)?;
                offset += size;
                let gt = ctx
                    .global_types
                    .get(idx as usize)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: global.set index {idx} out of bounds ({} globals)", ctx.global_types.len())))?
                    .clone();
                if !gt.mutable {
                    err!("global.set on immutable global {idx}");
                }
                pop_expect(&mut stack, frame!(), gt.value_type)?;
            }
            0x25 => {
                // table.get <tableidx> (WASM17): pops an i32 index, pushes
                // a funcref -- WASM 1.0's single table is always funcref
                // (see `code/specs/W08-wasm-funcref-externref.md`'s
                // "explicitly out of scope" section for why non-funcref
                // tables aren't modeled here). Unlike `has_memory` (a plain
                // bool, since memory ops hardcode index 0), this decodes a
                // REAL tableidx that must be bounds-checked, same pattern
                // as `call`'s funcidx check above.
                let (table_idx, size) = decode_idx(code, offset)?;
                offset += size;
                if table_idx >= ctx.table_count {
                    err!("table.get references table index {table_idx}, but only {} tables exist", ctx.table_count);
                }
                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                push_val(&mut stack, ValueType::Funcref);
            }
            0x26 => {
                // table.set <tableidx> (WASM17): pops a funcref and an i32
                // index, no push.
                let (table_idx, size) = decode_idx(code, offset)?;
                offset += size;
                if table_idx >= ctx.table_count {
                    err!("table.set references table index {table_idx}, but only {} tables exist", ctx.table_count);
                }
                pop_expect(&mut stack, frame!(), ValueType::Funcref)?;
                pop_expect(&mut stack, frame!(), ValueType::I32)?;
            }

            // ── Memory ───────────────────────────────────────────────────────
            0x28..=0x3E => {
                if !ctx.has_memory {
                    err!("memory instruction used, but module declares no memory");
                }
                let info = get_opcode(byte).expect("0x28..=0x3E are all real memory opcodes");
                let (align, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad memarg align: {e}")))?;
                let (_mem_offset, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad memarg offset: {e}")))?;
                offset += sz1 + sz2;

                let (value_type, max_align) = memory_op_shape(info.name)?;
                let max_align = max_align_for(max_align);
                if align as u32 > max_align {
                    err!("{}: alignment 2^{align} exceeds the natural alignment 2^{max_align}", info.name);
                }
                if info.stack_push == 1 {
                    pop_expect(&mut stack, frame!(), ValueType::I32)?; // address
                    push_val(&mut stack, value_type);
                } else {
                    pop_expect(&mut stack, frame!(), value_type)?; // stored value (top)
                    pop_expect(&mut stack, frame!(), ValueType::I32)?; // address
                }
            }
            0x3F => {
                // memory.size
                let (_reserved, size) = decode_idx(code, offset)?;
                offset += size;
                if !ctx.has_memory {
                    err!("memory.size used, but module declares no memory");
                }
                push_val(&mut stack, ValueType::I32);
            }
            0x40 => {
                // memory.grow
                let (_reserved, size) = decode_idx(code, offset)?;
                offset += size;
                if !ctx.has_memory {
                    err!("memory.grow used, but module declares no memory");
                }
                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                push_val(&mut stack, ValueType::I32);
            }

            // ── Conversion (incl. sign-extension, WASM03) ───────────────────
            0xA7..=0xC4 => {
                let info = get_opcode(byte).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: unknown conversion opcode {byte:#x}")))?;
                let (input, output) = conversion_types(info.name).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: no type rule for conversion opcode {}", info.name)))?;
                pop_expect(&mut stack, frame!(), input)?;
                push_val(&mut stack, output);
            }

            _ => {
                let info = get_opcode(byte).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: unknown opcode {byte:#x}")))?;
                if matches!(info.category, "numeric_i32" | "numeric_i64" | "numeric_f32" | "numeric_f64") {
                    // The only numeric opcodes with an immediate are the
                    // four `*.const` forms -- every arithmetic/comparison
                    // opcode has `immediates: &[]`. Skipping the const
                    // value's own bytes here is essential: leaving them
                    // unconsumed desyncs every opcode read after it (a
                    // constant's trailing immediate byte gets misread as
                    // the NEXT instruction's opcode byte).
                    match info.immediates {
                        [] => {}
                        ["i32"] => {
                            let (_, size) = decode_signed(code, offset).map_err(|e| ValidationError::Other(format!("bad i32.const immediate: {e}")))?;
                            offset += size;
                        }
                        ["i64"] => {
                            let (_, size) = decode_signed(code, offset).map_err(|e| ValidationError::Other(format!("bad i64.const immediate: {e}")))?;
                            offset += size;
                        }
                        ["f32"] => offset += 4,
                        ["f64"] => offset += 8,
                        _ => err!("unexpected immediates {:?} on numeric opcode {}", info.immediates, info.name),
                    }
                    type_check_numeric(&mut stack, frame!(), info.name, info.stack_pop)?;
                } else {
                    err!("no type rule implemented for opcode {} (category {})", info.name, info.category);
                }
            }
        }
    }

    if !control_stack.is_empty() {
        return Err(ValidationError::Other(format!("function #{func_idx}: body ended with {} unclosed block(s)", control_stack.len())));
    }
    Ok(())
}

/// `(value_type, natural_access_size_in_bytes)` for a memory-family opcode
/// name -- W02 §2.6's memory instruction table. Every one of these opcodes
/// pops an `I32` address; the caller distinguishes load-family (address
/// in, `value_type` out) from store-family (address + `value_type` in,
/// nothing out) via `info.stack_push`.
fn memory_op_shape(name: &str) -> Result<(ValueType, u32), ValidationError> {
    use ValueType::*;
    Ok(match name {
        "i32.load" => (I32, 4),
        "i64.load" => (I64, 8),
        "f32.load" => (F32, 4),
        "f64.load" => (F64, 8),
        "i32.load8_s" | "i32.load8_u" => (I32, 1),
        "i32.load16_s" | "i32.load16_u" => (I32, 2),
        "i64.load8_s" | "i64.load8_u" => (I64, 1),
        "i64.load16_s" | "i64.load16_u" => (I64, 2),
        "i64.load32_s" | "i64.load32_u" => (I64, 4),
        "i32.store" => (I32, 4),
        "i64.store" => (I64, 8),
        "f32.store" => (F32, 4),
        "f64.store" => (F64, 8),
        "i32.store8" => (I32, 1),
        "i32.store16" => (I32, 2),
        "i64.store8" => (I64, 1),
        "i64.store16" => (I64, 2),
        "i64.store32" => (I64, 4),
        other => return Err(ValidationError::Other(format!("no type rule for memory opcode {other:?}"))),
    })
}
