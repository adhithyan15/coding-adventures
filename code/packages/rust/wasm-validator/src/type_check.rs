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

/// Read the single raw (non-LEB128) lane-index immediate byte every
/// `extract_lane`/`replace_lane` SIMD opcode carries, advancing `offset`
/// past it. Shared by every lane-immediate `SimdOpKind` arm in the `0xFD`
/// match below (SIMD widen PR37) -- only the truncation check is common
/// here; each caller still applies its OWN shape-specific range check
/// (0-15 for `i8x16`, 0-7 for `i16x8`, 0-3 for `i32x4`/`f32x4`, 0-1 for
/// `i64x2`/`f64x2`) immediately after, since the valid range depends on
/// which vector shape the immediate belongs to.
fn read_lane_index(code: &[u8], offset: &mut usize, func_idx: usize, op_name: &str) -> Result<u8, ValidationError> {
    if *offset >= code.len() {
        return Err(ValidationError::Other(format!("function #{func_idx}: truncated {op_name} lane index")));
    }
    let lane_idx = code[*offset];
    *offset += 1;
    Ok(lane_idx)
}

/// Read and validate `i8x16.shuffle`'s 16-byte raw (non-LEB128)
/// lane-index immediate, advancing `offset` past all 16 bytes -- SIMD
/// widen PR38 (task #229-231), the direct extension of
/// [`read_lane_index`]'s single-byte pattern to this instruction's 16
/// bytes at once. The KEY difference from every `extract_lane`/
/// `replace_lane` range check above: `shuffle` indexes into the
/// COMBINED 32-lane array of its TWO v128 operands (lanes 0-15 from the
/// first, 16-31 from the second), so every one of the 16 bytes here must
/// be `0..=31`, not `0..=15` (`i8x16`'s own single-operand lane count).
///
/// This is a hard VALIDATION-TIME rejection, not merely a runtime
/// concern: a module with even ONE byte out of range is invalid and
/// rejected right here, before it can ever execute -- matching the WASM
/// spec's own requirement that an out-of-range `laneidx` makes a module
/// invalid. This is also this crate's half of the security property this
/// PR's own review scrutinizes most closely: because EVERY one of the 16
/// bytes is checked (not just the first or the last), `wasm-execution`'s
/// own gather (see that crate's `SimdOpKind::Shuffle`... well, its
/// `sub_opcode == 0x0D` dispatch arm in `register_simd`, since `Shuffle`
/// is intercepted before the generic `SimdOpKind` lookup there) can
/// never be reached with an out-of-range index for any module that
/// passed this check.
fn read_shuffle_lane_indices(code: &[u8], offset: &mut usize, func_idx: usize) -> Result<[u8; 16], ValidationError> {
    let mut indices = [0u8; 16];
    for (i, slot) in indices.iter_mut().enumerate() {
        let lane_idx = read_lane_index(code, offset, func_idx, "i8x16.shuffle")?;
        if lane_idx > 31 {
            return Err(ValidationError::Other(format!(
                "function #{func_idx}: i8x16.shuffle lane index at position {i} is {lane_idx}, but must be in 0..=31 (indexes into the combined 32-lane space of both v128 operands)"
            )));
        }
        *slot = lane_idx;
    }
    Ok(indices)
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
        // v128 (SIMD) and funcref/externref (WASM17) single-value
        // blocktypes -- a real, previously-undetected gap: both fell
        // through to the type-index branch below instead, where their
        // raw byte read as signed LEB128 (`0x7B`→-5, `0x70`→-16, `0x6F`
        // →-17) produced a bogus negative "type index" that always
        // failed with `TypeIndexOutOfBounds`. Confirmed via the real,
        // pinned-commit `simd_const.wast` corpus (`(block (result v128)
        // ...)`) -- see `wasm-execution`'s matching fix in
        // `decode_function_body`'s "blocktype" operand decoder.
        0x7B => Ok((vec![], vec![ValueType::V128], 1)),
        0x70 => Ok((vec![], vec![ValueType::Funcref], 1)),
        0x6F => Ok((vec![], vec![ValueType::Externref], 1)),
        // `exnref` (`0x69`, W24): the same real gap the three cases above
        // already fixed once for v128/funcref/externref, now hit by a
        // single-value `(block (result exnref) ...)` blocktype -- the real
        // corpus's own shape (`throw_ref.wast`'s `(block $h (result
        // exnref) ...)`). Security review (W24): this crate originally
        // special-cased `0xE9` here (this repo's ORIGINAL, incorrect
        // `ValueType::Exnref` byte -- the value's two's-complement-mod-256
        // representation, not its SLEB128 encoding). That was a real,
        // attacker-reachable bug: `0xE9`'s LEB128 continuation bit is SET
        // (`>= 0x80`), so it's indistinguishable from the leading byte of a
        // genuine multi-byte type-index encoding -- any module declaring
        // 234+ types could trigger a silent blocktype misparse. Fixed at
        // the source (`wasm-types::ValueType::Exnref::byte_tag`/`encode`,
        // now `0x69` -- the correct SLEB128 single-byte encoding of
        // `-0x17`, continuation bit clear, so it can only ever be a
        // complete standalone value, never a type-index prefix). See
        // `code/specs/W24-wasm-exceptions-exnref-catch-ref.md`.
        0x69 => Ok((vec![], vec![ValueType::Exnref], 1)),
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
    /// Combined imported + module-defined memory COUNT, same index-space
    /// convention as `func_types`/`global_types`/`table_count`. Added for
    /// W18 (task #92/#111): every memarg-carrying load/store, plus
    /// `memory.size`/`memory.grow`/`memory.init`/`memory.copy`/
    /// `memory.fill`, can now decode a REAL, non-zero memory index (see
    /// `wasm-execution`'s own multi-memory memarg support) that must be
    /// bounds-checked against the actual memory count -- `has_memory`
    /// alone ("is there at least one") is no longer sufficient once an
    /// instruction can reference memory N specifically.
    memory_count: u32,
    /// Each memory's `is64`-ness (memory64 proposal, W25), same combined
    /// imports-first-then-declared index-space/ordering convention as
    /// `table_element_types` below -- a load/store/`memory.size`/
    /// `memory.grow` must type-check its address/result against the
    /// TARGET memory's own `is64`, not unconditionally assume `i32` (see
    /// `code/specs/W25-wasm-memory64-first-slice.md`).
    memory_is64: Vec<bool>,
    /// Combined imported + module-defined table COUNT (WASM17), same
    /// index-space convention as `func_types`/`global_types` -- unlike
    /// `has_memory` (a plain bool, since every memory op hardcodes memory
    /// index 0 and ignores its reserved-byte immediate), `table.get`/
    /// `table.set` decode a REAL `tableidx` immediate that must be
    /// bounds-checked against the actual table count, not just "is there
    /// at least one".
    table_count: u32,
    /// Each table's raw element-type byte (`0x70` funcref / `0x6F`
    /// externref), same index-space/ordering convention as `table_count`
    /// (task #96): `table.get $t3 ...`/`table.set $t3 ...` must type-check
    /// against table `$t3`'s OWN declared element type, not unconditionally
    /// assume funcref -- WASM 1.0's single implicit table is always
    /// funcref, but a module with more than one table (multi-table) can
    /// mix funcref and externref tables freely.
    table_element_types: Vec<u8>,
    /// Combined imported + module-defined TAG types (W21 — the exceptions
    /// proposal), same index-space convention as `func_types` above:
    /// imports first, then module-defined (`module.tags: Vec<u32>`, type
    /// indices only) in declaration order. Each entry is the tag's
    /// underlying function signature (`crate::validate` already rejected
    /// any module where that signature has non-empty `results`, so this
    /// type-checker doesn't need to re-check that here — only use
    /// `.params` for `throw`'s pop rule).
    tag_types: Vec<FuncType>,
}

fn build_module_context(module: &WasmModule) -> Result<ModuleContext<'_>, ValidationError> {
    use wasm_types::ImportTypeInfo;

    let mut func_types = Vec::new();
    let mut global_types = Vec::new();
    let mut tag_types = Vec::new();
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
            ImportTypeInfo::Tag(type_idx) => {
                let ty = module
                    .types
                    .get(*type_idx as usize)
                    .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("import {}.{} (tag) references type index {type_idx}, but only {} types exist", imp.module_name, imp.name, module.types.len())))?;
                tag_types.push(ty.clone());
            }
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
    for &type_idx in &module.tags {
        let ty = module
            .types
            .get(type_idx as usize)
            .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("tag references type index {type_idx}, but only {} types exist", module.types.len())))?;
        tag_types.push(ty.clone());
    }

    let has_memory = !module.memories.is_empty() || module.imports.iter().any(|i| matches!(i.type_info, ImportTypeInfo::Memory(_)));
    let imported_memory_count = module.imports.iter().filter(|i| matches!(i.type_info, ImportTypeInfo::Memory(_))).count();
    let memory_count = (imported_memory_count + module.memories.len()) as u32;
    let mut memory_is64: Vec<bool> = module
        .imports
        .iter()
        .filter_map(|i| match &i.type_info {
            ImportTypeInfo::Memory(mt) => Some(mt.is64),
            _ => None,
        })
        .collect();
    memory_is64.extend(module.memories.iter().map(|m| m.is64));
    let mut table_element_types: Vec<u8> = module
        .imports
        .iter()
        .filter_map(|i| match &i.type_info {
            ImportTypeInfo::Table(tt) => Some(tt.element_type),
            _ => None,
        })
        .collect();
    table_element_types.extend(module.tables.iter().map(|t| t.element_type));
    let table_count = table_element_types.len() as u32;

    Ok(ModuleContext {
        module,
        func_types,
        global_types,
        has_memory,
        memory_count,
        memory_is64,
        table_count,
        table_element_types,
        tag_types,
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
                    0x08 => {
                        // `memory.init` (task #95): pops the same [dest,
                        // src, length] shape as memory.copy, but its
                        // `data_idx` immediate must reference a REAL data
                        // segment (unlike memory.copy/fill's discarded
                        // memory-index bytes) -- an out-of-bounds index is
                        // a real validation error, not deferred to a
                        // runtime trap, matching every other indexed
                        // immediate this type-checker validates (func/
                        // table/global/local indices).
                        if !ctx.has_memory {
                            err!("memory.init requires a declared memory");
                        }
                        let (data_idx, data_size) = decode_idx(code, offset)?;
                        let (memory, mem_size) = decode_idx(code, offset + data_size)?;
                        offset += data_size + mem_size;
                        // W18 (task #92/#111): `memory`'s LEB128 is now
                        // decoded for real by `wasm-execution` (task
                        // #109) instead of assumed MVP-only -- bounds-
                        // check it against the real memory count instead
                        // of hard-rejecting any nonzero value.
                        if memory >= ctx.memory_count {
                            err!("memory.init references memory index {memory}, but only {} memories exist", ctx.memory_count);
                        }
                        if data_idx as usize >= ctx.module.data.len() {
                            err!("memory.init references out-of-bounds data segment index {data_idx}");
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // length
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // source
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // destination
                    }
                    0x09 => {
                        // `data.drop` (task #95): no stack operands, no
                        // memory requirement at all (a module with zero
                        // memories can still declare and drop a passive
                        // data segment it never gets to `memory.init`
                        // from) -- just the same out-of-bounds data-
                        // segment-index check as `memory.init` above.
                        let (data_idx, data_size) = decode_idx(code, offset)?;
                        offset += data_size;
                        if data_idx as usize >= ctx.module.data.len() {
                            err!("data.drop references out-of-bounds data segment index {data_idx}");
                        }
                    }
                    0x0A => {
                        if !ctx.has_memory {
                            err!("memory.copy requires a declared memory");
                        }
                        let (dst_memory, dst_size) = decode_idx(code, offset)?;
                        let (src_memory, src_size) = decode_idx(code, offset + dst_size)?;
                        offset += dst_size + src_size;
                        // W18 (task #92/#111): both memidx LEB128s are now
                        // decoded for real by `wasm-execution` (task #109)
                        // instead of assumed MVP-only -- bounds-check each
                        // against the real memory count instead of hard-
                        // rejecting any nonzero value.
                        if dst_memory >= ctx.memory_count {
                            err!("memory.copy references destination memory index {dst_memory}, but only {} memories exist", ctx.memory_count);
                        }
                        if src_memory >= ctx.memory_count {
                            err!("memory.copy references source memory index {src_memory}, but only {} memories exist", ctx.memory_count);
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
                        // W18 (task #92/#111): see memory.init/memory.copy above.
                        if memory >= ctx.memory_count {
                            err!("memory.fill references memory index {memory}, but only {} memories exist", ctx.memory_count);
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // length
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // byte value
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // destination
                    }
                    0x0F => {
                        // `table.grow` (task #98): pops `[init, delta]`
                        // (init: the REFERENCED table's own element type,
                        // delta: i32), pushes i32 (old size, or -1 on
                        // failure -- never a validation-time error, growth
                        // failure is a normal runtime return value). Same
                        // per-table element-type lookup as `table.get`/
                        // `table.set` above (task #96) -- a table.grow on
                        // a funcref table takes a funcref init value, not
                        // whatever the FIRST declared table happens to be.
                        let (table_idx, size) = decode_idx(code, offset)?;
                        offset += size;
                        if table_idx >= ctx.table_count {
                            err!("table.grow references table index {table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        let elem_type = match ctx.table_element_types[table_idx as usize] {
                            0x6F => ValueType::Externref,
                            _ => ValueType::Funcref,
                        };
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // delta
                        pop_expect(&mut stack, frame!(), elem_type)?; // init value
                        push_val(&mut stack, ValueType::I32);
                    }
                    0x10 => {
                        // `table.size` (task #98): no stack operands,
                        // pushes the table's size as i32. Only the
                        // index-bounds check applies -- element type is
                        // irrelevant to a size query.
                        let (table_idx, size) = decode_idx(code, offset)?;
                        offset += size;
                        if table_idx >= ctx.table_count {
                            err!("table.size references table index {table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        push_val(&mut stack, ValueType::I32);
                    }
                    0x11 => {
                        // `table.fill` (task #98): pops `[dest, value,
                        // len]` (dest/len: i32, value: the REFERENCED
                        // table's own element type), no push. Same
                        // element-type lookup as 0x0F above.
                        let (table_idx, size) = decode_idx(code, offset)?;
                        offset += size;
                        if table_idx >= ctx.table_count {
                            err!("table.fill references table index {table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        let elem_type = match ctx.table_element_types[table_idx as usize] {
                            0x6F => ValueType::Externref,
                            _ => ValueType::Funcref,
                        };
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // length
                        pop_expect(&mut stack, frame!(), elem_type)?; // value
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // destination
                    }
                    0x0C => {
                        // `table.init` (task #97): pops `[dest, src, len]`
                        // (all i32), no push. Binary immediate order is
                        // `elemidx` THEN `tableidx` (opposite of the text
                        // form's `$table $elem` order -- confirmed against
                        // the real testsuite encoding). Both indices are
                        // hard validation errors on out-of-bounds, same
                        // discipline as `memory.init`'s data_idx check
                        // above (task #95).
                        let (elem_idx, elem_size) = decode_idx(code, offset)?;
                        let (table_idx, table_size) = decode_idx(code, offset + elem_size)?;
                        offset += elem_size + table_size;
                        if elem_idx as usize >= ctx.module.elements.len() {
                            err!("table.init references out-of-bounds element segment index {elem_idx}");
                        }
                        if table_idx >= ctx.table_count {
                            err!("table.init references table index {table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // length
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // source
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // destination
                    }
                    0x0D => {
                        // `elem.drop` (task #97): no stack operands, no
                        // table requirement at all (a module with zero
                        // tables can still declare and drop a passive elem
                        // segment it never `table.init`s from) -- mirrors
                        // `data.drop` above exactly.
                        let (elem_idx, size) = decode_idx(code, offset)?;
                        offset += size;
                        if elem_idx as usize >= ctx.module.elements.len() {
                            err!("elem.drop references out-of-bounds element segment index {elem_idx}");
                        }
                    }
                    0x0E => {
                        // `table.copy` (task #97): pops `[dest, src, len]`
                        // (all i32), no push. Text and binary immediate
                        // orders MATCH here (dst-then-src both times,
                        // unlike table.init above). Both table indices are
                        // bounds-checked independently -- a self-copy
                        // (dst == src) is valid and checked at runtime, not
                        // rejected here.
                        let (dst_table_idx, dst_size) = decode_idx(code, offset)?;
                        let (src_table_idx, src_size) = decode_idx(code, offset + dst_size)?;
                        offset += dst_size + src_size;
                        if dst_table_idx >= ctx.table_count {
                            err!("table.copy references destination table index {dst_table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        if src_table_idx >= ctx.table_count {
                            err!("table.copy references source table index {src_table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // length
                        pop_expect(&mut stack, frame!(), ValueType::I32)?; // source
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
                        // ref.i31 (W20; this crate previously called it
                        // i31.new): pops I32, pushes i31ref.
                        pop_expect(&mut stack, frame!(), ValueType::I32)?;
                        stack.push(StackType::Unknown);
                    }
                    0x1D => {
                        // i31.get_s: pops i31ref, pushes I32.
                        pop_val(&mut stack, frame!())?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    0x1E => {
                        // i31.get_u (W20, new): pops i31ref, pushes I32 —
                        // identical type-rule shape to i31.get_s (0x1D);
                        // the unsigned-vs-signed distinction is purely a
                        // runtime concern.
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
            0x08 => {
                // `throw` (W21 — exceptions proposal): pops the tag's
                // declared param types (an out-of-bounds tag index is a
                // hard validation error, "unknown tag" -- matches
                // `throw.wast`'s own `(assert_invalid (module (func (throw
                // 0))) "unknown tag 0")` case, though `grade_assert_invalid`
                // only checks THAT the module is rejected, never this exact
                // message text), then marks the rest of the current block
                // unreachable -- same shape `unreachable`/`br`/`return`
                // already use: control never falls through past a `throw`.
                let (tag_idx, size) = decode_idx(code, offset)?;
                offset += size;
                let tag_type = ctx.tag_types.get(tag_idx as usize).ok_or_else(|| {
                    ValidationError::Other(format!("function #{func_idx}: unknown tag {tag_idx}"))
                })?;
                pop_expect_many(&mut stack, frame!(), &tag_type.params)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x0A => {
                // `throw_ref` (W24 — exceptions proposal, fourth slice):
                // pops a real `exnref` (produced by a `catch_ref`/
                // `catch_all_ref` clause) and re-raises the exception it
                // names. Real spec type: `[t* (ref null exn)] -> [t2*]` —
                // same "pop one operand, then the rest of this block is
                // unreachable" shape `throw`/`unreachable`/`br`/`return`
                // already use above, since control never falls through
                // past it either. `throw_ref.wast`'s own two
                // `assert_invalid` cases (`(func (throw_ref))`, `(func
                // (block (throw_ref)))`, both "type mismatch") are exactly
                // this: an empty stack has nothing to pop.
                pop_expect(&mut stack, frame!(), ValueType::Exnref)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x1F => {
                // `try_table` (W21 — exceptions proposal): decodes exactly
                // like `block` (0x02) -- same blocktype immediate, same
                // `push_ctrl(..., FrameKind::Block, ...)` -- PLUS a
                // catch-clause list this repo deliberately never matches at
                // runtime (see `code/specs/
                // W21-wasm-exceptions-tag-throw-slice.md`'s "What actually
                // is separable" section for why: an uncaught exception
                // propagating straight through a `try_table` exactly like
                // it would through a plain `block` is the real spec's own
                // defined behavior for "no catch clause matched", and this
                // slice's `try_table` never looks for a match). The catch
                // clauses still get real, if narrow, validation here: each
                // tag index (for `catch`/`catch_ref`) must be in bounds,
                // and each label index must resolve to a real enclosing
                // block (same `resolve_label_target` every branch
                // instruction already uses) -- both are genuine
                // out-of-bounds hazards a hostile module could otherwise
                // use to reach an unvalidated index later, even though no
                // vendored corpus file currently exercises either failure
                // mode. Catch-target type-arity matching for PLAIN `catch`/
                // `catch_all` (the tag's params vs. the label's own
                // declared types) is still NOT checked -- unchanged W21/W22
                // scope reduction, deliberately left alone here (no
                // regression risk to already-passing `catch`/`catch_all`
                // directives). `catch_ref`/`catch_all_ref` (W24) DO get a
                // real arity/type check below, since it's exactly what
                // distinguishes a legitimately-typed target from an
                // invalid one now that they push a genuine `exnref` (see
                // `code/specs/W24-wasm-exceptions-exnref-catch-ref.md`,
                // and `try_table.wast`'s own `catch_ref`/`catch_all_ref`
                // `assert_invalid` cases, e.g. `(tag) (func (try_table
                // (catch_ref 0 0)))` "type mismatch": the target label
                // expects no values, but `catch_ref` would push `exnref`).
                let (params, results, bt_size) = decode_blocktype(ctx.module, code, offset)?;
                offset += bt_size;
                let (catch_count, cc_size) = decode_idx(code, offset)?;
                offset += cc_size;
                for _ in 0..catch_count {
                    let clause_kind = *code.get(offset).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: truncated try_table catch clause")))?;
                    offset += 1;
                    match clause_kind {
                        0x00 | 0x01 => {
                            // catch / catch_ref: tag idx, then label idx.
                            let (tag_idx, tsz) = decode_idx(code, offset)?;
                            offset += tsz;
                            let tag_params = ctx.tag_types.get(tag_idx as usize).map(|t| t.params.clone()).ok_or_else(|| {
                                ValidationError::Other(format!("function #{func_idx}: try_table catch clause references unknown tag {tag_idx}"))
                            })?;
                            let (label_idx, lsz) = decode_idx(code, offset)?;
                            offset += lsz;
                            let target = resolve_label_target(control_stack.len(), label_idx).ok_or_else(|| {
                                ValidationError::Other(format!("function #{func_idx}: try_table catch clause label target {label_idx} out of range"))
                            })?;
                            if clause_kind == 0x01 {
                                let mut expected = tag_params;
                                expected.push(ValueType::Exnref);
                                if label_types(&control_stack[target]) != expected.as_slice() {
                                    err!("try_table catch_ref clause: target label type does not match tag params + exnref");
                                }
                            }
                        }
                        0x02 | 0x03 => {
                            // catch_all / catch_all_ref: label idx only.
                            let (label_idx, lsz) = decode_idx(code, offset)?;
                            offset += lsz;
                            let target = resolve_label_target(control_stack.len(), label_idx).ok_or_else(|| {
                                ValidationError::Other(format!("function #{func_idx}: try_table catch_all clause label target {label_idx} out of range"))
                            })?;
                            if clause_kind == 0x03 && label_types(&control_stack[target]) != [ValueType::Exnref] {
                                err!("try_table catch_all_ref clause: target label type does not match [exnref]");
                            }
                        }
                        other => err!("try_table: unknown catch clause kind {other}"),
                    }
                }
                push_ctrl(&mut stack, &mut control_stack, FrameKind::Block, params, results)?;
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
                let (table_idx, sz2) = decode_idx(code, offset + sz1)?;
                offset += sz1 + sz2;
                // Task #107: `table_idx` used to be decoded and discarded
                // -- every `call_indirect` ran against table 0 regardless
                // of what it actually named. Same bounds-check shape
                // `table.grow`/`table.size`/`table.fill` (task #98) and
                // `table.init`/`table.copy` (task #97) already use.
                if table_idx >= ctx.table_count {
                    err!("call_indirect references table index {table_idx}, but only {} tables exist", ctx.table_count);
                }
                let callee_type = ctx
                    .module
                    .types
                    .get(type_idx as usize)
                    .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("function #{func_idx}: call_indirect references type index {type_idx}, but only {} types exist", ctx.module.types.len())))?;
                pop_expect(&mut stack, frame!(), ValueType::I32)?; // table index
                pop_expect_many(&mut stack, frame!(), &callee_type.params)?;
                push_vals(&mut stack, &callee_type.results);
            }
            0x12 => {
                // return_call (WASM16): same immediate as `call`, but
                // nothing runs after a tail call -- the callee's results
                // become the CURRENT FUNCTION's own results directly, so
                // they must match its declared result types exactly (not
                // merely be pushable for further use), and everything
                // textually after this is dead code, the same handling
                // `return` (0x0F) already has.
                let (callee, size) = decode_idx(code, offset)?;
                offset += size;
                let callee_type = ctx
                    .func_types
                    .get(callee as usize)
                    .ok_or_else(|| ValidationError::FuncIndexOutOfBounds(format!("function #{func_idx}: return_call references function index {callee}, but only {} functions exist", ctx.func_types.len())))?;
                let function_results = &control_stack
                    .first()
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: return_call with no open block")))?
                    .end_types;
                if callee_type.results != *function_results {
                    err!("return_call to function #{callee} returning {:?}, but the current function returns {function_results:?}", callee_type.results);
                }
                pop_expect_many(&mut stack, frame!(), &callee_type.params)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x13 => {
                // return_call_indirect (WASM16): same immediates as
                // `call_indirect` (typeidx, tableidx), same tail-call
                // result-type-must-match-exactly + dead-code-after rule
                // as `return_call` above.
                let (type_idx, sz1) = decode_idx(code, offset)?;
                let (table_idx, sz2) = decode_idx(code, offset + sz1)?;
                offset += sz1 + sz2;
                // Task #107: same bounds-check as call_indirect (0x11) above.
                if table_idx >= ctx.table_count {
                    err!("return_call_indirect references table index {table_idx}, but only {} tables exist", ctx.table_count);
                }
                let callee_type = ctx
                    .module
                    .types
                    .get(type_idx as usize)
                    .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("function #{func_idx}: return_call_indirect references type index {type_idx}, but only {} types exist", ctx.module.types.len())))?;
                let function_results = &control_stack
                    .first()
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: return_call_indirect with no open block")))?
                    .end_types;
                if callee_type.results != *function_results {
                    err!("return_call_indirect to type #{type_idx} returning {:?}, but the current function returns {function_results:?}", callee_type.results);
                }
                pop_expect(&mut stack, frame!(), ValueType::I32)?; // table index
                pop_expect_many(&mut stack, frame!(), &callee_type.params)?;
                mark_unreachable(&mut stack, frame_mut!());
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
                // table.get <tableidx> (WASM17, generalized task #96):
                // pops an i32 index, pushes the REFERENCED table's OWN
                // element type -- funcref or externref, whichever `$t`
                // was actually declared as (multi-table lets a module mix
                // both). Unlike `has_memory` (a plain bool, since memory
                // ops hardcode index 0), this decodes a REAL tableidx that
                // must be bounds-checked, same pattern as `call`'s funcidx
                // check above.
                let (table_idx, size) = decode_idx(code, offset)?;
                offset += size;
                if table_idx >= ctx.table_count {
                    err!("table.get references table index {table_idx}, but only {} tables exist", ctx.table_count);
                }
                let elem_type = match ctx.table_element_types[table_idx as usize] {
                    0x6F => ValueType::Externref,
                    _ => ValueType::Funcref,
                };
                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                push_val(&mut stack, elem_type);
            }
            0x26 => {
                // table.set <tableidx> (WASM17, generalized task #96):
                // pops a value of the REFERENCED table's own element type
                // and an i32 index, no push.
                let (table_idx, size) = decode_idx(code, offset)?;
                offset += size;
                if table_idx >= ctx.table_count {
                    err!("table.set references table index {table_idx}, but only {} tables exist", ctx.table_count);
                }
                let elem_type = match ctx.table_element_types[table_idx as usize] {
                    0x6F => ValueType::Externref,
                    _ => ValueType::Funcref,
                };
                pop_expect(&mut stack, frame!(), elem_type)?;
                pop_expect(&mut stack, frame!(), ValueType::I32)?;
            }

            // ── Memory ───────────────────────────────────────────────────────
            0x28..=0x3E => {
                if !ctx.has_memory {
                    err!("memory instruction used, but module declares no memory");
                }
                let info = get_opcode(byte).expect("0x28..=0x3E are all real memory opcodes");
                // W18 (task #92/#111): the align byte's top bit (0x40) is
                // the real multi-memory flag -- when set, a memidx
                // LEB128 trails the offset. Mirrors `wasm-execution`'s
                // own decode exactly (`MULTI_MEMORY_FLAG`).
                const MULTI_MEMORY_FLAG: u32 = 0x40;
                let (raw_align, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad memarg align: {e}")))?;
                // W25 (memory64): the memarg `offset` immediate is `u64`
                // unconditionally in the real spec's binary grammar
                // (verified live against `https://webassembly.github.io/
                // spec/core/binary/instructions.html` -- see `code/specs/
                // W25-wasm-memory64-first-slice.md`), not just for a
                // 64-bit memory; widened from the previous `u32`-typed
                // read purely for decode correctness -- this arm never
                // actually uses the decoded VALUE, only its byte length,
                // so this change is a no-op for every already-passing
                // file.
                let (_mem_offset, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad memarg offset: {e}")))?;
                let raw_align = raw_align as u32;
                let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                let align = raw_align & !MULTI_MEMORY_FLAG;
                offset += sz1 + sz2;
                let memidx = if has_memidx {
                    let (memidx, sz3) = decode_idx(code, offset)?;
                    offset += sz3;
                    memidx
                } else {
                    0
                };
                if memidx >= ctx.memory_count {
                    err!("{} references memory index {memidx}, but only {} memories exist", info.name, ctx.memory_count);
                }

                let (value_type, max_align) = memory_op_shape(info.name)?;
                let max_align = max_align_for(max_align);
                if align > max_align {
                    err!("{}: alignment 2^{align} exceeds the natural alignment 2^{max_align}", info.name);
                }
                // W25 (memory64): the address operand is `I64`, not
                // `I32`, when the TARGET memory (`memidx`, just
                // bounds-checked above) is `is64`.
                let addr_type = if ctx.memory_is64.get(memidx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                if info.stack_push == 1 {
                    pop_expect(&mut stack, frame!(), addr_type)?; // address
                    push_val(&mut stack, value_type);
                } else {
                    pop_expect(&mut stack, frame!(), value_type)?; // stored value (top)
                    pop_expect(&mut stack, frame!(), addr_type)?; // address
                }
            }
            0x3F => {
                // memory.size -- WASM17 already made this byte a real
                // memidx at execution time; W18 (task #92/#111) closes
                // the matching validation-time gap: bounds-check it here
                // too, instead of treating it as a discarded "reserved" byte.
                let (memidx, size) = decode_idx(code, offset)?;
                offset += size;
                if !ctx.has_memory {
                    err!("memory.size used, but module declares no memory");
                }
                if memidx >= ctx.memory_count {
                    err!("memory.size references memory index {memidx}, but only {} memories exist", ctx.memory_count);
                }
                // W25 (memory64): result type is `I64` for an `is64`
                // memory (see `code/specs/
                // W25-wasm-memory64-first-slice.md`).
                let result_type = if ctx.memory_is64.get(memidx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                push_val(&mut stack, result_type);
            }
            0x40 => {
                // memory.grow -- same real-memidx bounds-check as memory.size above.
                let (memidx, size) = decode_idx(code, offset)?;
                offset += size;
                if !ctx.has_memory {
                    err!("memory.grow used, but module declares no memory");
                }
                if memidx >= ctx.memory_count {
                    err!("memory.grow references memory index {memidx}, but only {} memories exist", ctx.memory_count);
                }
                // W25 (memory64): delta/result types are `I64` for an
                // `is64` memory, same shape as `memory.size` above.
                let grow_type = if ctx.memory_is64.get(memidx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                pop_expect(&mut stack, frame!(), grow_type)?;
                push_val(&mut stack, grow_type);
            }

            // ── `0xFE`-prefixed atomic memory operations (threads
            // proposal, WASM18): sub-opcode is a single RAW byte (not
            // LEB128), same shape 0xFB/0xFC's own sub-opcode read
            // already uses. `wasm_opcodes::ATOMIC_OPS` is the one shared
            // name/value-type/width table this crate, `wasm-execution`,
            // and `wasm-wast-parser` all key off. See `code/specs/
            // W09-wasm-atomics-plain.md`.
            0xFE => {
                let sub = *code.get(offset).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: truncated 0xFE opcode")))?;
                offset += 1;
                let atomic_op = wasm_opcodes::get_atomic_op(sub)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: unknown atomic sub-opcode {sub:#04x}")))?;

                if atomic_op.kind == wasm_opcodes::AtomicOpKind::Fence {
                    // atomic.fence: no immediate, no memory requirement
                    // (meaningful even with zero declared memories,
                    // though no real module would hit that), no stack
                    // effect at all.
                } else {
                    let (align, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad atomic memarg align: {e}")))?;
                    let (_mem_offset, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad atomic memarg offset: {e}")))?;
                    offset += sz1 + sz2;

                    if !ctx.has_memory {
                        err!("{} used, but module declares no memory", atomic_op.name);
                    }
                    // NOTE: this repo does NOT require the memory to be
                    // declared `shared` for atomic ops to validate --
                    // confirmed against the real, pinned-commit
                    // `proposals/threads/atomic.wast` testsuite file
                    // itself, whose own `;; unshared memory is OK` module
                    // exercises every atomic instruction in this file
                    // against a plain, non-`shared` `(memory 1 1)` and
                    // expects it to validate. `code/specs/
                    // W09-wasm-atomics-plain.md`'s prose claims otherwise
                    // (an early-draft-spec assumption that turned out not
                    // to match the pinned commit's actual corpus) --
                    // `MemoryType::shared` is still parsed and tracked
                    // for real, just not enforced as a validation gate.
                    // Atomic accesses must be naturally aligned EXACTLY --
                    // stricter than plain loads/stores, which only reject
                    // align > natural (an upper bound, not equality; see
                    // the `0x28..=0x3E` arm below).
                    let required_align = max_align_for(atomic_op.natural_align);
                    if align as u32 != required_align {
                        err!("{}: alignment 2^{align} must equal the natural alignment 2^{required_align} exactly", atomic_op.name);
                    }

                    match atomic_op.kind {
                        wasm_opcodes::AtomicOpKind::Load => {
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), ValueType::I32)?; // address
                            push_val(&mut stack, value_type);
                        }
                        wasm_opcodes::AtomicOpKind::Store => {
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), value_type)?; // value
                            pop_expect(&mut stack, frame!(), ValueType::I32)?; // address
                        }
                        wasm_opcodes::AtomicOpKind::Rmw => {
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), value_type)?; // operand
                            pop_expect(&mut stack, frame!(), ValueType::I32)?; // address
                            push_val(&mut stack, value_type); // old value
                        }
                        wasm_opcodes::AtomicOpKind::Cmpxchg => {
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), value_type)?; // replacement
                            pop_expect(&mut stack, frame!(), value_type)?; // expected
                            pop_expect(&mut stack, frame!(), ValueType::I32)?; // address
                            push_val(&mut stack, value_type); // old value
                        }
                        wasm_opcodes::AtomicOpKind::Notify => {
                            // memory.atomic.notify: pop (addr: i32, count:
                            // i32), push i32 (how many woken -- always 0
                            // with one native thread, see AtomicOpKind::
                            // Notify's own doc comment).
                            pop_expect(&mut stack, frame!(), ValueType::I32)?; // count
                            pop_expect(&mut stack, frame!(), ValueType::I32)?; // address
                            push_val(&mut stack, ValueType::I32);
                        }
                        wasm_opcodes::AtomicOpKind::Wait => {
                            // memory.atomic.wait32/wait64: pop (addr:
                            // i32, expected: value_type, timeout: i64),
                            // push i32 (result code).
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), ValueType::I64)?; // timeout
                            pop_expect(&mut stack, frame!(), value_type)?; // expected
                            pop_expect(&mut stack, frame!(), ValueType::I32)?; // address
                            push_val(&mut stack, ValueType::I32);
                        }
                        wasm_opcodes::AtomicOpKind::Fence => unreachable!("handled in the branch above"),
                    }
                }
            }

            // ── SIMD (v128) first slice, SIMD PR1b-2 -- see code/specs/
            // W13-wasm-simd-v128-first-slice.md ────────────────────────
            //
            // Same two-byte-prefix shape as `0xFE` atomics above, but the
            // sub-opcode is a LEB128 `u32` (not a raw byte) -- see
            // `wasm_opcodes::SimdOpInfo::sub_opcode`'s own doc comment for
            // why (`i32x4.add`'s real sub-opcode, 174, needs the 2-byte
            // LEB128 continuation encoding). `v128.const`'s own 16-byte
            // literal is decoded here only to advance `offset` past it
            // correctly -- its actual byte VALUES don't affect the type
            // stack, only its presence (pushing one `V128`) does.
            0xFD => {
                let (sub, size) = decode_unsigned(code, offset)
                    .map_err(|e| ValidationError::Other(format!("bad 0xFD sub-opcode: {e}")))?;
                offset += size;
                let simd_op = wasm_opcodes::get_simd_op(sub as u32)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: unknown SIMD sub-opcode {sub:#04x}")))?;
                match simd_op.kind {
                    wasm_opcodes::SimdOpKind::Const => {
                        if offset + 16 > code.len() {
                            return Err(ValidationError::Other(format!("function #{func_idx}: truncated v128.const literal")));
                        }
                        offset += 16;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::Shuffle => {
                        // i8x16.shuffle (SIMD widen PR38, task #229-231):
                        // pops two V128 operands (the BINARY shape shared
                        // with `Swizzle`/`Add`/etc. below), plus reads AND
                        // validates its 16-byte raw lane-index immediate
                        // via `read_shuffle_lane_indices` -- see that
                        // function's own doc comment for the full
                        // security reasoning (every one of the 16 bytes
                        // must be `0..=31`, checked here at VALIDATION
                        // time, so `wasm-execution`'s gather can never
                        // see an out-of-range index for a module that
                        // passes this check). The immediate is read
                        // before the two V128 pops below purely to match
                        // the binary encoding's own byte order (the
                        // immediate comes right after the sub-opcode,
                        // before any further bytes) -- the type checker's
                        // stack effect doesn't depend on this ordering.
                        read_shuffle_lane_indices(code, &mut offset, func_idx)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::Splat
                    | wasm_opcodes::SimdOpKind::SplatI8x16
                    | wasm_opcodes::SimdOpKind::SplatI16x8 => {
                        // i8x16.splat/i16x8.splat (SIMD widen PR16): same
                        // "pop I32, push V128" shape as i32x4.splat --
                        // only the low bits of the popped i32 matter at
                        // runtime, invisible to the type checker.
                        pop_expect(&mut stack, frame!(), ValueType::I32)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::SplatI64x2 => {
                        // i64x2.splat (SIMD widen PR16): the FIRST splat
                        // that pops I64 instead of I32.
                        pop_expect(&mut stack, frame!(), ValueType::I64)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::SplatF32x4 => {
                        // f32x4.splat (SIMD widen PR17): the FIRST
                        // floating-point-typed SIMD op in this crate's
                        // type rules -- pop F32, push V128.
                        pop_expect(&mut stack, frame!(), ValueType::F32)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::SplatF64x2 => {
                        // f64x2.splat (SIMD widen PR17): pop F64, push
                        // V128. Same shape as SplatF32x4.
                        pop_expect(&mut stack, frame!(), ValueType::F64)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::Add
                    | wasm_opcodes::SimdOpKind::Sub
                    | wasm_opcodes::SimdOpKind::Mul
                    | wasm_opcodes::SimdOpKind::MinS
                    | wasm_opcodes::SimdOpKind::MinU
                    | wasm_opcodes::SimdOpKind::MaxS
                    | wasm_opcodes::SimdOpKind::MaxU
                    | wasm_opcodes::SimdOpKind::DotI16x8S
                    | wasm_opcodes::SimdOpKind::RelaxedDotI8x16I7x16S
                    | wasm_opcodes::SimdOpKind::ExtmulLowI16x8S
                    | wasm_opcodes::SimdOpKind::ExtmulHighI16x8S
                    | wasm_opcodes::SimdOpKind::ExtmulLowI16x8U
                    | wasm_opcodes::SimdOpKind::ExtmulHighI16x8U
                    | wasm_opcodes::SimdOpKind::AddI8x16
                    | wasm_opcodes::SimdOpKind::SubI8x16
                    | wasm_opcodes::SimdOpKind::AddI16x8
                    | wasm_opcodes::SimdOpKind::SubI16x8
                    | wasm_opcodes::SimdOpKind::MulI16x8
                    | wasm_opcodes::SimdOpKind::MinSI8x16
                    | wasm_opcodes::SimdOpKind::MinUI8x16
                    | wasm_opcodes::SimdOpKind::MaxSI8x16
                    | wasm_opcodes::SimdOpKind::MaxUI8x16
                    | wasm_opcodes::SimdOpKind::AvgrUI8x16
                    | wasm_opcodes::SimdOpKind::MinSI16x8
                    | wasm_opcodes::SimdOpKind::MinUI16x8
                    | wasm_opcodes::SimdOpKind::MaxSI16x8
                    | wasm_opcodes::SimdOpKind::MaxUI16x8
                    | wasm_opcodes::SimdOpKind::AvgrUI16x8
                    | wasm_opcodes::SimdOpKind::ExtmulLowI8x16S
                    | wasm_opcodes::SimdOpKind::ExtmulHighI8x16S
                    | wasm_opcodes::SimdOpKind::ExtmulLowI8x16U
                    | wasm_opcodes::SimdOpKind::ExtmulHighI8x16U
                    | wasm_opcodes::SimdOpKind::And
                    | wasm_opcodes::SimdOpKind::AndNot
                    | wasm_opcodes::SimdOpKind::Or
                    | wasm_opcodes::SimdOpKind::Xor
                    | wasm_opcodes::SimdOpKind::AddI64x2
                    | wasm_opcodes::SimdOpKind::SubI64x2
                    | wasm_opcodes::SimdOpKind::MulI64x2
                    | wasm_opcodes::SimdOpKind::Swizzle
                    | wasm_opcodes::SimdOpKind::RelaxedSwizzle
                    | wasm_opcodes::SimdOpKind::MulF32x4
                    | wasm_opcodes::SimdOpKind::MinF32x4
                    | wasm_opcodes::SimdOpKind::MaxF32x4
                    | wasm_opcodes::SimdOpKind::PminF32x4
                    | wasm_opcodes::SimdOpKind::PmaxF32x4
                    | wasm_opcodes::SimdOpKind::AddF32x4
                    | wasm_opcodes::SimdOpKind::SubF32x4
                    | wasm_opcodes::SimdOpKind::DivF32x4
                    | wasm_opcodes::SimdOpKind::AddF64x2
                    | wasm_opcodes::SimdOpKind::SubF64x2
                    | wasm_opcodes::SimdOpKind::MulF64x2
                    | wasm_opcodes::SimdOpKind::DivF64x2
                    | wasm_opcodes::SimdOpKind::ExtmulLowI64x2S
                    | wasm_opcodes::SimdOpKind::ExtmulHighI64x2S
                    | wasm_opcodes::SimdOpKind::ExtmulLowI64x2U
                    | wasm_opcodes::SimdOpKind::ExtmulHighI64x2U
                    | wasm_opcodes::SimdOpKind::Q15mulrSatI16x8S
                    | wasm_opcodes::SimdOpKind::RelaxedQ15mulrI16x8S
                    | wasm_opcodes::SimdOpKind::NarrowI16x8S
                    | wasm_opcodes::SimdOpKind::NarrowI16x8U
                    | wasm_opcodes::SimdOpKind::NarrowI32x4S
                    | wasm_opcodes::SimdOpKind::NarrowI32x4U
                    | wasm_opcodes::SimdOpKind::AddSatI8x16S
                    | wasm_opcodes::SimdOpKind::AddSatI8x16U
                    | wasm_opcodes::SimdOpKind::SubSatI8x16S
                    | wasm_opcodes::SimdOpKind::SubSatI8x16U
                    | wasm_opcodes::SimdOpKind::AddSatI16x8S
                    | wasm_opcodes::SimdOpKind::AddSatI16x8U
                    | wasm_opcodes::SimdOpKind::SubSatI16x8S
                    | wasm_opcodes::SimdOpKind::SubSatI16x8U
                    | wasm_opcodes::SimdOpKind::MinF64x2
                    | wasm_opcodes::SimdOpKind::MaxF64x2
                    | wasm_opcodes::SimdOpKind::PminF64x2
                    | wasm_opcodes::SimdOpKind::PmaxF64x2
                    | wasm_opcodes::SimdOpKind::RelaxedMinF32x4
                    | wasm_opcodes::SimdOpKind::RelaxedMaxF32x4
                    | wasm_opcodes::SimdOpKind::RelaxedMinF64x2
                    | wasm_opcodes::SimdOpKind::RelaxedMaxF64x2 => {
                        // `dot_i16x8_s`/`extmul_low`/`high_i16x8_s`/`_u`/
                        // `i8x16.add`/`sub`/`i16x8.add`/`sub`/`mul`/
                        // `i8x16.min_s`/`min_u`/`max_s`/`max_u`/`avgr_u`/
                        // `i16x8.min_s`/`min_u`/`max_s`/`max_u`/`avgr_u`/
                        // `extmul_low`/`high_i8x16_s`/`_u`/`and`/`andnot`/
                        // `or`/`xor`/`i8x16.swizzle` (SIMD widen PR18) all
                        // read/write a narrower-than-`v128` (or, for the
                        // bitwise ops, lane-width-agnostic; or, for
                        // `swizzle`, an index-vector-driven permutation)
                        // shape internally, but AT THE TYPE LEVEL they're
                        // the same pop-two-push-one `v128` shape as
                        // `Add`/`Sub`/`Mul` above -- the type-checker
                        // only sees `v128`, never the narrower lane
                        // interpretation. `f32x4.mul`/`f32x4.min` (SIMD
                        // widen PR19) join too: their NaN/signed-zero
                        // runtime subtlety (see wasm-opcodes'
                        // `SimdOpKind::MinF32x4` doc comment) is entirely
                        // invisible here -- still just two V128 pops, one
                        // V128 push. `f32x4.add`/`sub`/`div` (SIMD widen
                        // PR29) join too, same reasoning: ordinary IEEE-754
                        // arithmetic (including `div`'s TOTAL behavior on
                        // a zero divisor -- no trap) is entirely a runtime
                        // concern, invisible to the type checker.
                        // `f64x2.add`/`sub`/`mul`/`div` (SIMD widen PR31)
                        // join too, direct 2-lane mirrors of
                        // `f32x4.add`/`sub`/`div` above plus `mul` on the
                        // same shape -- still just two V128 pops, one
                        // V128 push.
                        // `i64x2.extmul_low`/`high_i32x4_s`/
                        // `_u` (SIMD widen PR21) complete the third and
                        // final "extmul" rung -- same reasoning: the
                        // `i32x4` -> `i64x2` widening is entirely a
                        // runtime concern, invisible to the type checker.
                        // `i8x16.narrow_i16x8_s`/`_u`/`i16x8.narrow_i32x4_s`/
                        // `_u` (SIMD widen PR27) join too: the "narrow"
                        // family is genuinely BINARY (two v128 operands,
                        // unlike the UNARY "extend" family in PR26 above),
                        // and its saturating-then-narrowing runtime
                        // behavior is, same as every other kind in this
                        // arm, entirely invisible to the type checker --
                        // still just two V128 pops, one V128 push.
                        // `i8x16.add_sat_s`/`_u`/`.sub_sat_s`/`_u`/
                        // `i16x8.add_sat_s`/`_u`/`.sub_sat_s`/`_u` (SIMD
                        // widen PR33) join too: same BINARY pop-two-push-
                        // one-V128 shape as `NarrowI16x8S/U` above -- the
                        // compute-in-a-wider-type-then-clamp saturation
                        // arithmetic is entirely a runtime concern,
                        // invisible to the type checker.
                        // `f32x4.max`/`pmin`/`pmax` (SIMD widen PR34) join
                        // too: `f32x4.max`'s NaN-canonicalizing/signed-zero
                        // runtime subtlety (mirroring `f32x4.min` above) and
                        // `pmin`/`pmax`'s DELIBERATELY SIMPLER `<`-based
                        // conditional-select semantics (see wasm-opcodes'
                        // `SimdOpKind::PminF32x4`/`PmaxF32x4` doc comments
                        // for the exact first-operand-wins-on-NaN behavior)
                        // are both entirely runtime concerns -- at the type
                        // level all three are still just two V128 pops, one
                        // V128 push, same as `f32x4.min`/`mul` beside them.
                        // `f64x2.min`/`max`/`pmin`/`pmax` (SIMD widen PR35)
                        // join too, direct 2-lane mirrors of `f32x4.min`/
                        // `max`/`pmin`/`pmax` -- same NaN-canonicalizing vs.
                        // `<`-based-select runtime distinction (see
                        // wasm-opcodes' `SimdOpKind::MinF64x2`/`PminF64x2`
                        // doc comments), entirely invisible here -- still
                        // just two V128 pops, one V128 push.
                        // `i8x16.relaxed_swizzle` (relaxed SIMD epic PR1 --
                        // see code/specs/W19-wasm-relaxed-simd-first-slice.
                        // md) joins too: same `(v128, v128) -> v128` shape
                        // as `Swizzle` above, its only difference being
                        // that its out-of-range index behavior is spec-
                        // sanctioned as implementation-defined at RUNTIME
                        // -- entirely invisible here, still just two V128
                        // pops, one V128 push.
                        // `i16x8.relaxed_q15mulr_s` (relaxed SIMD epic PR2)
                        // joins too: same `(v128, v128) -> v128` shape as
                        // `Q15mulrSatI16x8S` above, its only difference
                        // being that the single `MIN, MIN` overflow lane's
                        // saturate-vs-wrap choice is spec-sanctioned as
                        // implementation-defined at RUNTIME -- entirely
                        // invisible here, still just two V128 pops, one
                        // V128 push.
                        // `f32x4.relaxed_min`/`relaxed_max`,
                        // `f64x2.relaxed_min`/`relaxed_max` (relaxed SIMD
                        // epic PR3) join too: same `(v128, v128) -> v128`
                        // shape as `PminF32x4`/`PmaxF32x4`/`PminF64x2`/
                        // `PmaxF64x2` above, whose bodies they reuse
                        // verbatim -- the NaN/signed-zero handling choice
                        // is, same as every other kind in this arm,
                        // entirely a runtime concern, invisible here.
                        // `i16x8.relaxed_dot_i8x16_i7x16_s` (relaxed SIMD
                        // epic PR6) joins too: same `(v128, v128) -> v128`
                        // shape as `DotI16x8S` above, at the narrower
                        // `i8x16` input width -- the "signed * signed"
                        // semantic choice for its `i7x16`-named operand
                        // (see wasm-opcodes' `SimdOpKind::
                        // RelaxedDotI8x16I7x16S` doc comment) is entirely
                        // a runtime concern, invisible here.
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::Eq
                    | wasm_opcodes::SimdOpKind::Ne
                    | wasm_opcodes::SimdOpKind::LtS
                    | wasm_opcodes::SimdOpKind::LtU
                    | wasm_opcodes::SimdOpKind::GtS
                    | wasm_opcodes::SimdOpKind::GtU
                    | wasm_opcodes::SimdOpKind::LeS
                    | wasm_opcodes::SimdOpKind::LeU
                    | wasm_opcodes::SimdOpKind::GeS
                    | wasm_opcodes::SimdOpKind::GeU
                    | wasm_opcodes::SimdOpKind::EqI16x8
                    | wasm_opcodes::SimdOpKind::NeI16x8
                    | wasm_opcodes::SimdOpKind::LtSI16x8
                    | wasm_opcodes::SimdOpKind::LtUI16x8
                    | wasm_opcodes::SimdOpKind::GtSI16x8
                    | wasm_opcodes::SimdOpKind::GtUI16x8
                    | wasm_opcodes::SimdOpKind::LeSI16x8
                    | wasm_opcodes::SimdOpKind::LeUI16x8
                    | wasm_opcodes::SimdOpKind::GeSI16x8
                    | wasm_opcodes::SimdOpKind::GeUI16x8
                    | wasm_opcodes::SimdOpKind::EqI8x16
                    | wasm_opcodes::SimdOpKind::NeI8x16
                    | wasm_opcodes::SimdOpKind::LtSI8x16
                    | wasm_opcodes::SimdOpKind::LtUI8x16
                    | wasm_opcodes::SimdOpKind::GtSI8x16
                    | wasm_opcodes::SimdOpKind::GtUI8x16
                    | wasm_opcodes::SimdOpKind::LeSI8x16
                    | wasm_opcodes::SimdOpKind::LeUI8x16
                    | wasm_opcodes::SimdOpKind::GeSI8x16
                    | wasm_opcodes::SimdOpKind::GeUI8x16
                    | wasm_opcodes::SimdOpKind::EqI64x2
                    | wasm_opcodes::SimdOpKind::NeI64x2
                    | wasm_opcodes::SimdOpKind::LtSI64x2
                    | wasm_opcodes::SimdOpKind::GtSI64x2
                    | wasm_opcodes::SimdOpKind::LeSI64x2
                    | wasm_opcodes::SimdOpKind::GeSI64x2
                    | wasm_opcodes::SimdOpKind::EqF32x4
                    | wasm_opcodes::SimdOpKind::NeF32x4
                    | wasm_opcodes::SimdOpKind::LtF32x4
                    | wasm_opcodes::SimdOpKind::GtF32x4
                    | wasm_opcodes::SimdOpKind::LeF32x4
                    | wasm_opcodes::SimdOpKind::GeF32x4
                    | wasm_opcodes::SimdOpKind::EqF64x2
                    | wasm_opcodes::SimdOpKind::NeF64x2
                    | wasm_opcodes::SimdOpKind::LtF64x2
                    | wasm_opcodes::SimdOpKind::GtF64x2
                    | wasm_opcodes::SimdOpKind::LeF64x2
                    | wasm_opcodes::SimdOpKind::GeF64x2 => {
                        // WASM's SIMD comparison convention: the RESULT is
                        // still a v128 (a per-lane boolean mask), not a
                        // plain i32 -- see `SimdOpKind::Eq`'s own doc
                        // comment in wasm-opcodes. Same convention for
                        // i16x8's and i8x16's own comparison families.
                        // `f32x4.eq`/`ne`/`lt`/`gt`/`le`/`ge` (SIMD widen
                        // PR30) join too: same pop-two-push-one V128 shape
                        // -- the IEEE-754 float-comparison and NaN-handling
                        // semantics are entirely a runtime concern (see
                        // `wasm-execution`), invisible to the type checker.
                        // `f64x2.eq`/`ne`/`lt`/`gt`/`le`/`ge` (SIMD widen
                        // PR32) join too, a direct 2-lane mirror of the
                        // `f32x4` comparison family just above -- still
                        // just two V128 pops, one V128 push.
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::Neg
                    | wasm_opcodes::SimdOpKind::Abs
                    | wasm_opcodes::SimdOpKind::ExtaddPairwiseI16x8S
                    | wasm_opcodes::SimdOpKind::ExtaddPairwiseI16x8U
                    | wasm_opcodes::SimdOpKind::NegI8x16
                    | wasm_opcodes::SimdOpKind::NegI16x8
                    | wasm_opcodes::SimdOpKind::AbsI8x16
                    | wasm_opcodes::SimdOpKind::PopcntI8x16
                    | wasm_opcodes::SimdOpKind::AbsI16x8
                    | wasm_opcodes::SimdOpKind::ExtaddPairwiseI8x16S
                    | wasm_opcodes::SimdOpKind::ExtaddPairwiseI8x16U
                    | wasm_opcodes::SimdOpKind::Not
                    | wasm_opcodes::SimdOpKind::AbsI64x2
                    | wasm_opcodes::SimdOpKind::NegI64x2
                    | wasm_opcodes::SimdOpKind::AbsF32x4
                    | wasm_opcodes::SimdOpKind::NegF32x4
                    | wasm_opcodes::SimdOpKind::SqrtF32x4
                    | wasm_opcodes::SimdOpKind::NegF64x2
                    | wasm_opcodes::SimdOpKind::SqrtF64x2
                    | wasm_opcodes::SimdOpKind::TruncSatF32x4S
                    | wasm_opcodes::SimdOpKind::TruncSatF32x4U
                    | wasm_opcodes::SimdOpKind::ConvertI32x4S
                    | wasm_opcodes::SimdOpKind::ConvertI32x4U
                    | wasm_opcodes::SimdOpKind::TruncSatF64x2SZero
                    | wasm_opcodes::SimdOpKind::TruncSatF64x2UZero
                    | wasm_opcodes::SimdOpKind::ExtendLowI8x16S
                    | wasm_opcodes::SimdOpKind::ExtendHighI8x16S
                    | wasm_opcodes::SimdOpKind::ExtendLowI8x16U
                    | wasm_opcodes::SimdOpKind::ExtendHighI8x16U
                    | wasm_opcodes::SimdOpKind::ExtendLowI16x8S
                    | wasm_opcodes::SimdOpKind::ExtendHighI16x8S
                    | wasm_opcodes::SimdOpKind::ExtendLowI16x8U
                    | wasm_opcodes::SimdOpKind::ExtendHighI16x8U
                    | wasm_opcodes::SimdOpKind::ExtendLowI32x4S
                    | wasm_opcodes::SimdOpKind::ExtendHighI32x4S
                    | wasm_opcodes::SimdOpKind::ExtendLowI32x4U
                    | wasm_opcodes::SimdOpKind::ExtendHighI32x4U
                    | wasm_opcodes::SimdOpKind::DemoteF64x2Zero
                    | wasm_opcodes::SimdOpKind::PromoteLowF32x4
                    | wasm_opcodes::SimdOpKind::ConvertLowI32x4S
                    | wasm_opcodes::SimdOpKind::ConvertLowI32x4U
                    | wasm_opcodes::SimdOpKind::AbsF64x2
                    | wasm_opcodes::SimdOpKind::CeilF32x4
                    | wasm_opcodes::SimdOpKind::FloorF32x4
                    | wasm_opcodes::SimdOpKind::TruncF32x4
                    | wasm_opcodes::SimdOpKind::NearestF32x4
                    | wasm_opcodes::SimdOpKind::CeilF64x2
                    | wasm_opcodes::SimdOpKind::FloorF64x2
                    | wasm_opcodes::SimdOpKind::TruncF64x2
                    | wasm_opcodes::SimdOpKind::NearestF64x2 => {
                        // UNARY, unlike every kind in the two arms above.
                        // `extadd_pairwise_i16x8_s`/`_u`/`i8x16.neg`/
                        // `i16x8.neg`/`i8x16.abs`/`popcnt`/`i16x8.abs`/
                        // `extadd_pairwise_i8x16_s`/`_u`/`v128.not` read
                        // their operand as a narrower lane width (or,
                        // for `not`, lane-width-agnostic bytes)
                        // internally, but are still just pop-one-`v128`-
                        // push-one-`v128` at the type level, same as
                        // `Neg`/`Abs`. `f32x4.abs` (SIMD widen PR19)
                        // joins too -- a pure bit operation, no new
                        // type-checker machinery needed. `f32x4.neg`/
                        // `sqrt` (SIMD widen PR29) join too, same
                        // reasoning -- their sign-flip/IEEE-754-sqrt
                        // runtime behavior is entirely invisible here,
                        // still just pop-one-`V128`-push-one-`V128`.
                        // `TruncSatF32x4S`/
                        // `_U`/`ConvertI32x4S`/`_U` (SIMD widen PR20) join
                        // too: even though these change the LANE TYPE
                        // (f32 lanes <-> i32 lanes), WASM's type system
                        // doesn't distinguish "i32-lane v128" from
                        // "f32-lane v128" -- both are just the opaque
                        // `V128` type here, same pop-one-push-one shape.
                        // `TruncSatF64x2SZero`/`_UZero` (SIMD widen PR25)
                        // join too, same reasoning: even though the
                        // runtime reads only 2 f64 lanes and writes 4 i32
                        // lanes (2 real + 2 zero-filled), the type
                        // checker still sees plain pop-one-`V128`-push-
                        // one-`V128`. `ExtendLow/HighI8x16S/_U`/
                        // `ExtendLow/HighI16x8S/_U` (SIMD widen PR26) join
                        // too: the "extend" family reads a narrower lane
                        // width and writes a wider one, but -- same
                        // reasoning as `ExtaddPairwiseI16x8S`/`_U` above --
                        // the type checker only ever sees the opaque
                        // `V128` type, never the narrower interpretation.
                        // `DemoteF64x2Zero`/`PromoteLowF32x4`/
                        // `ConvertLowI32x4S`/`_U` (SIMD widen PR28, the
                        // final PR of the 16-opcode `extend`/`narrow`/
                        // `promote`/`demote`/`convert_low` set) join too:
                        // all UNARY, and even though they cross both lane
                        // COUNT (4<->2) and lane TYPE (int/float, f32/f64)
                        // boundaries, the type checker still only ever
                        // sees the opaque `V128` type on both sides --
                        // the zero-fill (`DemoteF64x2Zero`) vs.
                        // lane-dropping (the other three) distinction is,
                        // like every other lane-shape detail above,
                        // entirely a runtime concern. `f64x2.neg`/`sqrt`
                        // (SIMD widen PR31) join too, direct 2-lane
                        // mirrors of `f32x4.neg`/`f32x4.sqrt` above --
                        // same pop-one-`V128`-push-one-`V128` shape.
                        // `f64x2.abs` (SIMD widen PR35) joins too, a direct
                        // 2-lane mirror of `f32x4.abs` above -- a pure bit
                        // operation, no new type-checker machinery needed,
                        // same pop-one-`V128`-push-one-`V128` shape.
                        // `ExtendLow/HighI32x4S/_U` (SIMD widen PR36) join
                        // too, the third and final rung of the "extend"
                        // family one lane width up from `ExtendLow/
                        // HighI16x8S/_U` -- same reasoning, the narrower
                        // (`i32`) source lane width and wider (`i64`)
                        // result lane width are both invisible to the type
                        // checker, still just pop-one-`V128`-push-one-
                        // `V128`.
                        // `f32x4.ceil`/`floor`/`trunc`/`nearest` and
                        // `f64x2.ceil`/`floor`/`trunc`/`nearest` (SIMD
                        // widen PR39) join too: all 8 UNARY, same
                        // pop-one-`V128`-push-one-`V128` shape as
                        // `AbsF32x4`/`AbsF64x2` above -- the per-lane
                        // IEEE-754 rounding-mode selection (including
                        // `nearest`'s ties-to-even vs. Rust's native
                        // away-from-zero `round()`, see
                        // `SimdOpKind::NearestF32x4`'s own doc comment in
                        // wasm-opcodes) is entirely a runtime concern,
                        // invisible here.
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::Bitselect
                    | wasm_opcodes::SimdOpKind::RelaxedLaneselectI8x16
                    | wasm_opcodes::SimdOpKind::RelaxedLaneselectI16x8
                    | wasm_opcodes::SimdOpKind::RelaxedLaneselectI32x4
                    | wasm_opcodes::SimdOpKind::RelaxedLaneselectI64x2
                    | wasm_opcodes::SimdOpKind::RelaxedMaddF32x4
                    | wasm_opcodes::SimdOpKind::RelaxedNmaddF32x4
                    | wasm_opcodes::SimdOpKind::RelaxedMaddF64x2
                    | wasm_opcodes::SimdOpKind::RelaxedNmaddF64x2
                    | wasm_opcodes::SimdOpKind::RelaxedDotI8x16I7x16AddS => {
                        // The first TERNARY SIMD op in this crate: pops
                        // THREE v128s, pushes one. See
                        // `SimdOpKind::Bitselect`'s own doc comment in
                        // wasm-opcodes for the runtime semantics -- at
                        // the type level it's just three V128 pops.
                        // `i8x16/i16x8/i32x4/i64x2.relaxed_laneselect`
                        // (relaxed-SIMD epic PR4 -- see `code/specs/
                        // W19-wasm-relaxed-simd-first-slice.md`) join
                        // too: same TERNARY `(v128, v128, v128) -> v128`
                        // shape as `Bitselect`, whose body they reuse
                        // verbatim at the runtime level -- the
                        // implementation-defined-vs-bitselect distinction
                        // the relaxed-simd spec draws (see
                        // `SimdOpKind::RelaxedLaneselectI8x16`'s own doc
                        // comment in wasm-opcodes) is entirely a runtime
                        // concern, invisible here.
                        // `f32x4.relaxed_madd`/`relaxed_nmadd`,
                        // `f64x2.relaxed_madd`/`relaxed_nmadd`
                        // (relaxed-SIMD epic PR5) join too: same TERNARY
                        // `(v128, v128, v128) -> v128` shape -- the fact
                        // that this family's runtime body is genuine
                        // per-lane floating-point fused-multiply-add
                        // rather than a bitwise blend (see
                        // `SimdOpKind::RelaxedMaddF32x4`'s own doc
                        // comment in wasm-opcodes) is, like every other
                        // numeric distinction in this match, entirely a
                        // runtime concern -- still just three V128 pops,
                        // one V128 push at this level.
                        // `i32x4.relaxed_dot_i8x16_i7x16_add_s` (relaxed-
                        // SIMD epic PR6) joins too: same TERNARY `(v128,
                        // v128, v128) -> v128` shape, but the FIRST
                        // ternary op in this crate whose third operand is
                        // a genuine numeric accumulator rather than a
                        // bitwise mask or a second fused-arithmetic input
                        // (see `SimdOpKind::RelaxedDotI8x16I7x16AddS`'s own
                        // doc comment in wasm-opcodes) -- entirely a
                        // runtime concern, invisible here: still just
                        // three V128 pops, one V128 push.
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::ExtractLane => {
                        // Lane-index immediate: a single raw byte, not a
                        // LEB128 value -- see wasm-execution's decoder for
                        // the same convention. (SIMD widen PR37 retrofit:
                        // the lane index's VALUE, not just its presence,
                        // is now checked here too -- see this match's own
                        // header comment above for why `i32x4.extract_lane`
                        // 4 must be a validation-time rejection, not a
                        // runtime-only one.)
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "i32x4.extract_lane")?;
                        if lane_idx >= 4 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: i32x4.extract_lane lane index {lane_idx} out of range (must be 0-3)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    wasm_opcodes::SimdOpKind::ExtractLaneI8x16S | wasm_opcodes::SimdOpKind::ExtractLaneI8x16U => {
                        // i8x16.extract_lane_s/_u (SIMD widen PR18): same
                        // shape as `ExtractLane` above -- a single raw
                        // lane-index byte immediate, pop one V128, push
                        // one I32. The valid 0-15 lane RANGE (vs
                        // `ExtractLane`'s 0-3) is now checked HERE (SIMD
                        // widen PR37 retrofit -- previously only the
                        // immediate's presence was checked, not its
                        // value, leaving out-of-range lane indices to
                        // reach `wasm-execution`'s runtime bounds check
                        // instead of being rejected at validation time,
                        // as the WASM spec requires). The sign-/zero-
                        // extend split remains a runtime-only concern,
                        // still invisible here.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "i8x16.extract_lane_s/u")?;
                        if lane_idx >= 16 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: i8x16.extract_lane_s/u lane index {lane_idx} out of range (must be 0-15)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    wasm_opcodes::SimdOpKind::ReplaceLaneI8x16 => {
                        // i8x16.replace_lane (SIMD widen PR18): a
                        // GENUINELY NEW shape (see its own `SimdOpKind`
                        // doc comment in wasm-opcodes) -- combines
                        // `ExtractLane*`'s lane-index immediate with the
                        // shift family's mixed-type binary pop order
                        // (`(ixNxM.shl (v128 $a) (i32 $amount))` pushes
                        // the v128 first, the scalar second, so the
                        // scalar is on TOP of stack and popped FIRST):
                        // pop I32 (the replacement value), then pop V128
                        // (the base operand), push V128. Lane-index VALUE
                        // now bounds-checked here too (SIMD widen PR37
                        // retrofit, same reasoning as `ExtractLaneI8x16S/U`
                        // above).
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "i8x16.replace_lane")?;
                        if lane_idx >= 16 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: i8x16.replace_lane lane index {lane_idx} out of range (must be 0-15)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::ExtractLaneI16x8S | wasm_opcodes::SimdOpKind::ExtractLaneI16x8U => {
                        // i16x8.extract_lane_s/_u (SIMD widen PR37):
                        // direct 8-lane mirror of `ExtractLaneI8x16S/U`
                        // above, one lane width up -- valid range 0-7.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "i16x8.extract_lane_s/u")?;
                        if lane_idx >= 8 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: i16x8.extract_lane_s/u lane index {lane_idx} out of range (must be 0-7)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    wasm_opcodes::SimdOpKind::ReplaceLaneI16x8 => {
                        // i16x8.replace_lane (SIMD widen PR37): direct
                        // 8-lane mirror of `ReplaceLaneI8x16` above --
                        // valid range 0-7.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "i16x8.replace_lane")?;
                        if lane_idx >= 8 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: i16x8.replace_lane lane index {lane_idx} out of range (must be 0-7)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::ReplaceLaneI32x4 => {
                        // i32x4.replace_lane (SIMD widen PR37): the
                        // `i32x4` counterpart to `ExtractLane` above --
                        // valid range 0-3, pop I32 then V128, push V128.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "i32x4.replace_lane")?;
                        if lane_idx >= 4 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: i32x4.replace_lane lane index {lane_idx} out of range (must be 0-3)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I32)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::ExtractLaneI64x2 => {
                        // i64x2.extract_lane (SIMD widen PR37): valid
                        // range 0-1 -- this table's narrowest lane-index
                        // range. Pops V128, pushes I64 (not I32 -- the
                        // first `extract_lane` family member whose
                        // result is a native 64-bit type, needing no
                        // widening).
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "i64x2.extract_lane")?;
                        if lane_idx >= 2 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: i64x2.extract_lane lane index {lane_idx} out of range (must be 0-1)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::I64);
                    }
                    wasm_opcodes::SimdOpKind::ReplaceLaneI64x2 => {
                        // i64x2.replace_lane (SIMD widen PR37): valid
                        // range 0-1. Pops I64 (not I32 -- the first
                        // `replace_lane` member with a 64-bit scalar
                        // operand) then V128, pushes V128.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "i64x2.replace_lane")?;
                        if lane_idx >= 2 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: i64x2.replace_lane lane index {lane_idx} out of range (must be 0-1)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::I64)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::ExtractLaneF32x4 => {
                        // f32x4.extract_lane (SIMD widen PR37): valid
                        // range 0-3. Pops V128, pushes F32 -- the first
                        // `extract_lane` family member whose result is
                        // floating-point.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "f32x4.extract_lane")?;
                        if lane_idx >= 4 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: f32x4.extract_lane lane index {lane_idx} out of range (must be 0-3)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::F32);
                    }
                    wasm_opcodes::SimdOpKind::ReplaceLaneF32x4 => {
                        // f32x4.replace_lane (SIMD widen PR37): valid
                        // range 0-3. Pops F32 (the first `replace_lane`
                        // member with a floating-point scalar operand)
                        // then V128, pushes V128.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "f32x4.replace_lane")?;
                        if lane_idx >= 4 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: f32x4.replace_lane lane index {lane_idx} out of range (must be 0-3)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::F32)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::ExtractLaneF64x2 => {
                        // f64x2.extract_lane (SIMD widen PR37): valid
                        // range 0-1. Pops V128, pushes F64.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "f64x2.extract_lane")?;
                        if lane_idx >= 2 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: f64x2.extract_lane lane index {lane_idx} out of range (must be 0-1)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::F64);
                    }
                    wasm_opcodes::SimdOpKind::ReplaceLaneF64x2 => {
                        // f64x2.replace_lane (SIMD widen PR37): valid
                        // range 0-1. Pops F64 then V128, pushes V128 --
                        // the LAST member of the extract_lane/
                        // replace_lane family across all six SIMD vector
                        // shapes, closing out validation-time bounds
                        // checking for the whole family.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "f64x2.replace_lane")?;
                        if lane_idx >= 2 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: f64x2.replace_lane lane index {lane_idx} out of range (must be 0-1)"
                            )));
                        }
                        pop_expect(&mut stack, frame!(), ValueType::F64)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::AnyTrue
                    | wasm_opcodes::SimdOpKind::AllTrueI8x16
                    | wasm_opcodes::SimdOpKind::AllTrueI16x8
                    | wasm_opcodes::SimdOpKind::AllTrueI32x4
                    | wasm_opcodes::SimdOpKind::AllTrueI64x2
                    | wasm_opcodes::SimdOpKind::BitmaskI8x16
                    | wasm_opcodes::SimdOpKind::BitmaskI16x8
                    | wasm_opcodes::SimdOpKind::BitmaskI32x4
                    | wasm_opcodes::SimdOpKind::BitmaskI64x2 => {
                        // Same v128-in/i32-out shape as `ExtractLane`
                        // above, but with NO lane-index immediate (these
                        // reduce over ALL lanes, not select one) -- pops
                        // one v128, pushes one i32.
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    wasm_opcodes::SimdOpKind::ShlI8x16
                    | wasm_opcodes::SimdOpKind::ShrSI8x16
                    | wasm_opcodes::SimdOpKind::ShrUI8x16
                    | wasm_opcodes::SimdOpKind::ShlI16x8
                    | wasm_opcodes::SimdOpKind::ShrSI16x8
                    | wasm_opcodes::SimdOpKind::ShrUI16x8
                    | wasm_opcodes::SimdOpKind::ShlI32x4
                    | wasm_opcodes::SimdOpKind::ShrSI32x4
                    | wasm_opcodes::SimdOpKind::ShrUI32x4
                    | wasm_opcodes::SimdOpKind::ShlI64x2
                    | wasm_opcodes::SimdOpKind::ShrSI64x2
                    | wasm_opcodes::SimdOpKind::ShrUI64x2 => {
                        // The FIRST mixed-type binary SIMD op family:
                        // `(ixNxM.shl (v128 $a) (i32 $amount))` pushes
                        // the v128 first, the i32 shift amount second --
                        // so the i32 is on TOP of stack and must be
                        // popped FIRST, then the v128, matching
                        // wasm-execution's own pop order.
                        pop_expect(&mut stack, frame!(), ValueType::I32)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::Load
                    | wasm_opcodes::SimdOpKind::Store
                    | wasm_opcodes::SimdOpKind::Load8Splat
                    | wasm_opcodes::SimdOpKind::Load16Splat
                    | wasm_opcodes::SimdOpKind::Load32Splat
                    | wasm_opcodes::SimdOpKind::Load64Splat
                    | wasm_opcodes::SimdOpKind::Load32Zero
                    | wasm_opcodes::SimdOpKind::Load64Zero
                    | wasm_opcodes::SimdOpKind::Load8x8S
                    | wasm_opcodes::SimdOpKind::Load8x8U
                    | wasm_opcodes::SimdOpKind::Load16x4S
                    | wasm_opcodes::SimdOpKind::Load16x4U
                    | wasm_opcodes::SimdOpKind::Load32x2S
                    | wasm_opcodes::SimdOpKind::Load32x2U => {
                        // v128.load/v128.store (SIMD widen PR15), the
                        // v128.loadN_splat family (SIMD PR40), the
                        // v128.loadN_zero family (SIMD PR41), plus the
                        // v128.load_extend family (SIMD PR42): a standard
                        // `memarg` immediate (align, offset[, memidx]) --
                        // decoded exactly like every scalar `iNN.load`/
                        // `iNN.store` (mirrors the `0x28..=0x3E` arm's own
                        // `MULTI_MEMORY_FLAG` handling) so a stray
                        // multi-memory encoding still consumes the right
                        // number of bytes and doesn't desync the rest of
                        // the function body. Unlike the scalar arm, this
                        // first slice's EXECUTOR unconditionally targets
                        // memory 0 (see wasm-execution's own scope note,
                        // which the load_extend/load_splat/load_zero
                        // families inherit unchanged) -- so an explicit
                        // non-zero memidx must be REJECTED here, not
                        // merely bounds-checked against `ctx.memory_
                        // count`. Bounds-checking alone would let a
                        // module targeting a real, in-bounds memory 1
                        // validate successfully and then silently
                        // read/write memory 0 at execution time instead
                        // -- fail closed until multi-memory v128.load/
                        // store is actually implemented (security review
                        // finding, task #162-164).
                        if !ctx.has_memory {
                            err!("v128.load/v128.store used, but module declares no memory");
                        }
                        const MULTI_MEMORY_FLAG: u32 = 0x40;
                        let (raw_align, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let (_mem_offset, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        offset += sz1 + sz2;
                        if has_memidx {
                            let (memidx, sz3) = decode_idx(code, offset)?;
                            offset += sz3;
                            if memidx != 0 {
                                err!("v128.load/v128.store: multi-memory (memory index {memidx}) is not yet supported -- only memory 0");
                            }
                        }
                        match simd_op.kind {
                            wasm_opcodes::SimdOpKind::Load
                            | wasm_opcodes::SimdOpKind::Load8Splat
                            | wasm_opcodes::SimdOpKind::Load16Splat
                            | wasm_opcodes::SimdOpKind::Load32Splat
                            | wasm_opcodes::SimdOpKind::Load64Splat
                            | wasm_opcodes::SimdOpKind::Load32Zero
                            | wasm_opcodes::SimdOpKind::Load64Zero
                            | wasm_opcodes::SimdOpKind::Load8x8S
                            | wasm_opcodes::SimdOpKind::Load8x8U
                            | wasm_opcodes::SimdOpKind::Load16x4S
                            | wasm_opcodes::SimdOpKind::Load16x4U
                            | wasm_opcodes::SimdOpKind::Load32x2S
                            | wasm_opcodes::SimdOpKind::Load32x2U => {
                                // v128.load and the whole loadN_splat/
                                // loadN_zero/load_extend families share
                                // the identical type signature: pop one
                                // i32 base address, push one v128 result
                                // -- whether the non-loaded lanes repeat
                                // the loaded value ("splat"), get zeroed
                                // ("zero"), or every loaded lane gets
                                // independently sign/zero-extended
                                // ("load_extend") changes nothing at the
                                // TYPE level, only at execution time.
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store => {
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                            }
                            _ => unreachable!("only Load/Store/Load8Splat/Load16Splat/Load32Splat/Load64Splat/Load32Zero/Load64Zero/Load8x8S/Load8x8U/Load16x4S/Load16x4U/Load32x2S/Load32x2U reach this arm"),
                        }
                    }
                    wasm_opcodes::SimdOpKind::Load8Lane | wasm_opcodes::SimdOpKind::Store8Lane => {
                        // v128.load8_lane / v128.store8_lane (SIMD PR44)
                        // -- the lane-load/store family's first bite, and
                        // a GENUINELY NEW instruction shape: unlike the
                        // arm just above (memarg only) and unlike
                        // `ExtractLane`/`ReplaceLaneI8x16` above (lane
                        // index only), this carries BOTH a memarg (align,
                        // offset[, memidx]) AND a lane-index byte,
                        // verified against BinarySIMD.md's own encoding
                        // order: "m:memarg, i:ImmLaneIdx16" -- memarg
                        // FIRST, lane index SECOND. Matches the pinned
                        // `simd_load8_lane.wast`/`simd_store8_lane.wast`
                        // corpus's own text-form immediate order
                        // (`(v128.load8_lane offset=4 4 ...)` -- `offset=`
                        // before the bare lane number).
                        if !ctx.has_memory {
                            err!("v128.load8_lane/v128.store8_lane used, but module declares no memory");
                        }
                        const MULTI_MEMORY_FLAG: u32 = 0x40;
                        let (raw_align, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let (_mem_offset, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        offset += sz1 + sz2;
                        if has_memidx {
                            let (memidx, sz3) = decode_idx(code, offset)?;
                            offset += sz3;
                            if memidx != 0 {
                                err!("v128.load8_lane/v128.store8_lane: multi-memory (memory index {memidx}) is not yet supported -- only memory 0");
                            }
                        }
                        // Lane-index immediate: a single raw byte (not
                        // LEB128), same `read_lane_index` helper
                        // `ExtractLane`/`ReplaceLane` above use -- and,
                        // per this PR's own lesson (PR37's retrofit: the
                        // validator must reject an out-of-range VALUE,
                        // not merely check the immediate's presence), a
                        // REAL 0-15 bounds check here, matching `i8x16`'s
                        // 16-lane width (both `load8_lane`/`store8_lane`
                        // index into a single 16-byte v128, one byte per
                        // lane, same as `i8x16.extract_lane_s/u` above).
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "v128.load8_lane/v128.store8_lane")?;
                        if lane_idx >= 16 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: v128.load8_lane/v128.store8_lane lane index {lane_idx} out of range (must be 0-15)"
                            )));
                        }
                        match simd_op.kind {
                            wasm_opcodes::SimdOpKind::Load8Lane => {
                                // pop the existing v128 (pushed LAST in
                                // source order, so on top of stack,
                                // popped FIRST -- its other 15 lanes are
                                // preserved at runtime, invisible at the
                                // type level), pop the i32 address, push
                                // the updated v128.
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store8Lane => {
                                // pop the v128 to read the lane from, pop
                                // the i32 address, no result -- same
                                // pop-order and no-push shape as `Store`
                                // above.
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                            }
                            _ => unreachable!("only Load8Lane/Store8Lane reach this arm"),
                        }
                    }
                    wasm_opcodes::SimdOpKind::Load16Lane | wasm_opcodes::SimdOpKind::Store16Lane => {
                        // v128.load16_lane / v128.store16_lane (SIMD
                        // PR45) -- the lane-load/store family's SECOND
                        // bite, one width up from the arm just above.
                        // Same GENUINELY NEW instruction shape (memarg
                        // AND lane-index byte together) -- verified
                        // against BinarySIMD.md's own encoding order:
                        // "m:memarg, i:ImmLaneIdx8" -- memarg FIRST, lane
                        // index SECOND. Matches the pinned
                        // `simd_load16_lane.wast`/`simd_store16_lane.
                        // wast` corpus's own text-form immediate order
                        // (`(v128.load16_lane offset=4 4 ...)` --
                        // `offset=` before the bare lane number).
                        if !ctx.has_memory {
                            err!("v128.load16_lane/v128.store16_lane used, but module declares no memory");
                        }
                        const MULTI_MEMORY_FLAG: u32 = 0x40;
                        let (raw_align, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let (_mem_offset, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        offset += sz1 + sz2;
                        if has_memidx {
                            let (memidx, sz3) = decode_idx(code, offset)?;
                            offset += sz3;
                            if memidx != 0 {
                                err!("v128.load16_lane/v128.store16_lane: multi-memory (memory index {memidx}) is not yet supported -- only memory 0");
                            }
                        }
                        // Lane-index immediate: a single raw byte (not
                        // LEB128), same `read_lane_index` helper the
                        // `Load8Lane`/`Store8Lane` arm above uses -- but
                        // a REAL 0-7 bounds check here, NOT the 0-15
                        // bound that arm uses: an `i16x8` v128 holds 8
                        // lanes (2 bytes each), not `i8x16`'s 16 (1 byte
                        // each), so reusing the wider bound would
                        // silently accept an invalid lane index 8-15 --
                        // the exact class of bug this PR's own doc
                        // comment (see `SimdOpKind::Load16Lane`'s own
                        // comment in `wasm-opcodes`) warns against.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "v128.load16_lane/v128.store16_lane")?;
                        if lane_idx >= 8 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: v128.load16_lane/v128.store16_lane lane index {lane_idx} out of range (must be 0-7)"
                            )));
                        }
                        match simd_op.kind {
                            wasm_opcodes::SimdOpKind::Load16Lane => {
                                // pop the existing v128 (pushed LAST in
                                // source order, so on top of stack,
                                // popped FIRST -- its other 7 lanes are
                                // preserved at runtime, invisible at the
                                // type level), pop the i32 address, push
                                // the updated v128.
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store16Lane => {
                                // pop the v128 to read the lane from, pop
                                // the i32 address, no result -- same
                                // pop-order and no-push shape as
                                // `Store8Lane` above.
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                            }
                            _ => unreachable!("only Load16Lane/Store16Lane reach this arm"),
                        }
                    }
                    wasm_opcodes::SimdOpKind::Load32Lane | wasm_opcodes::SimdOpKind::Store32Lane => {
                        // v128.load32_lane / v128.store32_lane (SIMD
                        // PR46) -- the lane-load/store family's THIRD
                        // bite, one width up from the arm just above.
                        // Same GENUINELY NEW instruction shape (memarg
                        // AND lane-index byte together) -- verified
                        // against BinarySIMD.md's own encoding order:
                        // "m:memarg, i:ImmLaneIdx4" -- memarg FIRST, lane
                        // index SECOND. Matches the pinned
                        // `simd_load32_lane.wast`/`simd_store32_lane.
                        // wast` corpus's own text-form immediate order
                        // (`(v128.load32_lane offset=4 4 ...)` --
                        // `offset=` before the bare lane number).
                        if !ctx.has_memory {
                            err!("v128.load32_lane/v128.store32_lane used, but module declares no memory");
                        }
                        const MULTI_MEMORY_FLAG: u32 = 0x40;
                        let (raw_align, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let (_mem_offset, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        offset += sz1 + sz2;
                        if has_memidx {
                            let (memidx, sz3) = decode_idx(code, offset)?;
                            offset += sz3;
                            if memidx != 0 {
                                err!("v128.load32_lane/v128.store32_lane: multi-memory (memory index {memidx}) is not yet supported -- only memory 0");
                            }
                        }
                        // Lane-index immediate: a single raw byte (not
                        // LEB128), same `read_lane_index` helper the
                        // `Load16Lane`/`Store16Lane` arm above uses -- but
                        // a REAL 0-3 bounds check here, NOT the 0-7
                        // bound that arm uses: an `i32x4` v128 holds 4
                        // lanes (4 bytes each), not `i16x8`'s 8 (2 bytes
                        // each), so reusing the wider bound would
                        // silently accept an invalid lane index 4-7 --
                        // the exact class of bug this PR's own doc
                        // comment (see `SimdOpKind::Load32Lane`'s own
                        // comment in `wasm-opcodes`) warns against.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "v128.load32_lane/v128.store32_lane")?;
                        if lane_idx >= 4 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: v128.load32_lane/v128.store32_lane lane index {lane_idx} out of range (must be 0-3)"
                            )));
                        }
                        match simd_op.kind {
                            wasm_opcodes::SimdOpKind::Load32Lane => {
                                // pop the existing v128 (pushed LAST in
                                // source order, so on top of stack,
                                // popped FIRST -- its other 3 lanes are
                                // preserved at runtime, invisible at the
                                // type level), pop the i32 address, push
                                // the updated v128.
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store32Lane => {
                                // pop the v128 to read the lane from, pop
                                // the i32 address, no result -- same
                                // pop-order and no-push shape as
                                // `Store16Lane` above.
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                            }
                            _ => unreachable!("only Load32Lane/Store32Lane reach this arm"),
                        }
                    }
                    wasm_opcodes::SimdOpKind::Load64Lane | wasm_opcodes::SimdOpKind::Store64Lane => {
                        // v128.load64_lane / v128.store64_lane (SIMD
                        // PR47) -- the lane-load/store family's FOURTH
                        // and FINAL bite, one width up from the arm just
                        // above. Same GENUINELY NEW instruction shape
                        // (memarg AND lane-index byte together) --
                        // verified against BinarySIMD.md's own encoding
                        // order: "m:memarg, i:ImmLaneIdx2" -- memarg
                        // FIRST, lane index SECOND. Matches the pinned
                        // `simd_load64_lane.wast`/`simd_store64_lane.
                        // wast` corpus's own text-form immediate order
                        // (`(v128.load64_lane offset=8 1 (addr) (x))` --
                        // `offset=` before the bare lane number).
                        if !ctx.has_memory {
                            err!("v128.load64_lane/v128.store64_lane used, but module declares no memory");
                        }
                        const MULTI_MEMORY_FLAG: u32 = 0x40;
                        let (raw_align, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let (_mem_offset, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        offset += sz1 + sz2;
                        if has_memidx {
                            let (memidx, sz3) = decode_idx(code, offset)?;
                            offset += sz3;
                            if memidx != 0 {
                                err!("v128.load64_lane/v128.store64_lane: multi-memory (memory index {memidx}) is not yet supported -- only memory 0");
                            }
                        }
                        // Lane-index immediate: a single raw byte (not
                        // LEB128), same `read_lane_index` helper the
                        // `Load32Lane`/`Store32Lane` arm above uses -- but
                        // a REAL 0-1 bounds check here, NOT the 0-3
                        // bound that arm uses: an `i64x2` v128 holds only
                        // 2 lanes (8 bytes each), not `i32x4`'s 4 (4
                        // bytes each), so reusing the wider bound would
                        // silently accept an invalid lane index 2-3 --
                        // the exact class of bug this PR's own doc
                        // comment (see `SimdOpKind::Load64Lane`'s own
                        // comment in `wasm-opcodes`) warns against.
                        let lane_idx = read_lane_index(code, &mut offset, func_idx, "v128.load64_lane/v128.store64_lane")?;
                        if lane_idx >= 2 {
                            return Err(ValidationError::Other(format!(
                                "function #{func_idx}: v128.load64_lane/v128.store64_lane lane index {lane_idx} out of range (must be 0-1)"
                            )));
                        }
                        match simd_op.kind {
                            wasm_opcodes::SimdOpKind::Load64Lane => {
                                // pop the existing v128 (pushed LAST in
                                // source order, so on top of stack,
                                // popped FIRST -- its other lane is
                                // preserved at runtime, invisible at the
                                // type level), pop the i32 address, push
                                // the updated v128.
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store64Lane => {
                                // pop the v128 to read the lane from, pop
                                // the i32 address, no result -- same
                                // pop-order and no-push shape as
                                // `Store32Lane` above.
                                pop_expect(&mut stack, frame!(), ValueType::V128)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32)?;
                            }
                            _ => unreachable!("only Load64Lane/Store64Lane reach this arm"),
                        }
                    }
                }
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
/// pops an address (`I32` for a 32-bit memory, `I64` for a 64-bit one --
/// W25, memory64 proposal -- the caller derives the right one from the
/// TARGET memory's own `is64`); the caller distinguishes load-family
/// (address in, `value_type` out) from store-family (address +
/// `value_type` in, nothing out) via `info.stack_push`.
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
