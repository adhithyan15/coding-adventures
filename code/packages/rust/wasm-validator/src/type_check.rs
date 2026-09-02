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

use std::rc::Rc;

use wasm_leb128::{decode_signed, decode_unsigned, decode_unsigned_bounded};
use wasm_opcodes::get_opcode;
use wasm_types::{
    ArrayType, CanonicalGroup, FieldType, FuncType, FunctionBody, GlobalType, StorageType, StructType, TypeKind, ValueType, WasmModule, nominal_subtype_chain,
};

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

/// WASM reference-type subtyping (W11 addendum): is a value of type
/// `actual` a legal stand-in wherever `expected` is declared?
///
/// Exact equality is always a subtype of itself; beyond that, this crate
/// only implements the ONE direction its function-references slice needs:
/// a nullable reference to a SPECIFIC concrete function type
/// (`ValueType::ConcreteFuncRef`, `(ref null $t)`) is a subtype of the
/// general `funcref` (every concrete function type's nullable ref is a
/// "some kind of funcref"), but never the reverse -- a plain `funcref`
/// carries no static guarantee about WHICH function type it names, so it
/// cannot stand in for a specific one. This is exactly the real corpus's
/// own `return_call.wast`/`return_call_indirect.wast` "Result subtyping"
/// tests: one direction `assert_return`s successfully, the mirror-image
/// direction `assert_invalid`s with a type mismatch.
///
/// W32 first slice (`code/specs/W32-wasm-non-null-concrete-reference-types.md`)
/// adds the four **bottom reference types** -- `NullFuncref`,
/// `NullExternref`, `NullExnref`, `NullRef` -- each a strict subtype of
/// every nullable type in its own hierarchy (func/extern/exn/any). That
/// lattice is owned by `wasm_types::ValueType::is_bottom_subtype_of` (kept
/// there, not duplicated here, so `wasm-types` stays the single source of
/// truth for the type system's own shape); this function just asks it.
///
/// Non-null concrete refs (`(ref $t)`, no `null` keyword) and full
/// structural subtyping for `call_indirect`/`ref.cast` against concrete
/// function/struct types are explicitly NOT part of this slice -- see the
/// spec's "Explicitly out of scope" / "genuinely open-ended" notes.
/// Whether `vt` is a WASM `numtype` or `vectype` -- the ONLY categories
/// the untyped `select` (`0x1B`) instruction accepts, per the real
/// reference-types proposal spec text (a reference-typed operand pair
/// requires the explicit `(result t)`-annotated `select`, `0x1C`, which
/// this crate does not implement -- see that opcode's own doc comment).
/// `false` for every reference type (`Anyref`/`I31ref`/`StructRef`/
/// `ConcreteFuncRef`/their W32-second-slice non-null counterparts/
/// `Funcref`/`Externref`/`Exnref`/every W32-first-slice bottom type).
fn is_numeric_or_vector(vt: ValueType) -> bool {
    matches!(vt, ValueType::I32 | ValueType::I64 | ValueType::F32 | ValueType::F64 | ValueType::V128)
}

/// A `&WasmModule` bundled with its own canonicalized type-group forms
/// (W34 third slice, `code/specs/W34-wasm-gc-canonical-type-equivalence.md`),
/// computed exactly ONCE per [`type_check_module`] call -- right after
/// [`check_type_subtyping`] confirms the module's `sub`/`rec` reference
/// ordering is well-founded, the same precondition `wasm_types::
/// canonicalize_types`'s own termination argument depends on -- and
/// threaded everywhere `module: &WasmModule` used to be threaded through
/// this file's instruction-level type-checking machinery.
///
/// `Copy` (both fields are plain references/slices) so every existing call
/// site that used to forward a bare `module`/`ctx.module` value keeps
/// compiling completely unchanged once this type replaces `&WasmModule` in
/// each of those functions' signatures -- no call site needed to grow a
/// new argument. `Deref<Target = WasmModule>` for the identical reason:
/// every existing `module.<field or method>` access (the overwhelming
/// majority of this file's own usages, e.g. `module.type_subtyping_at(..)`,
/// `module.types.len()`) also keeps compiling unchanged -- only the
/// handful of call sites that need real canonical-equivalence data reach
/// for `.canonically_equivalent(..)` directly.
///
/// This is intentionally a lightweight, per-call-cheap value: it carries a
/// borrowed slice, never a fresh computation, so passing it through 150+
/// existing call sites costs nothing beyond what passing `&WasmModule`
/// already cost (two words instead of one). See this crate's own security
/// review for this slice for why per-call-site cost, not just correctness,
/// was checked here.
#[derive(Clone, Copy)]
struct TypeContext<'a> {
    module: &'a WasmModule,
    canonical_types: &'a [Option<(Rc<CanonicalGroup>, u32)>],
}

impl<'a> std::ops::Deref for TypeContext<'a> {
    type Target = WasmModule;
    fn deref(&self) -> &WasmModule {
        self.module
    }
}

impl<'a> TypeContext<'a> {
    /// Whether flat type-section indices `sub_idx`/`super_idx` are related
    /// by the real GC-proposal rule (W34): nominal (nested `sub` chain) OR
    /// canonically equivalent at any hop, per [`nominal_subtype_chain`]'s
    /// own doc comment ("nominal modulo canonicalization"). This is the
    /// SAME shared, security-reviewed walk `wasm-execution`'s runtime
    /// dispatch uses (via its own `canonical_types` field), so the two can
    /// never drift apart.
    fn nominal_or_canonical_subtype(&self, sub_idx: u32, super_idx: u32) -> bool {
        nominal_subtype_chain(&self.module.type_subtyping, self.canonical_types, sub_idx, super_idx)
    }
}

/// `actual` flows where `expected` is required. `module` supplies the W33
/// first-slice nominal `sub`-chain (`code/specs/
/// W33-wasm-gc-recursive-type-subtyping.md`) AND, since W34's third slice
/// (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`), real canonical
/// type-group equivalence, needed for every `ConcreteFuncRef`/
/// `NonNullConcreteFuncRef`/`StructRef`/`NonNullStructRef`/`ArrayRef`/
/// `NonNullArrayRef` arm below -- every OTHER arm here predates W33 and
/// never looks at `module` at all, so a `WasmModule` that never populated
/// `type_subtyping` (see that field's own doc comment) behaves exactly as
/// before.
///
/// **W34 third slice**: the `StructRef`/`NonNullStructRef`/`ArrayRef`/
/// `NonNullArrayRef` arms are NEW -- this function had ZERO arms for any
/// of the four struct/array reference variants before this slice (a real,
/// previously-open gap the W34 spec's own research flagged: those variants
/// are already index-parametrized exactly like `ConcreteFuncRef`, and
/// `TypeSubtyping`/`nominal_subtype_chain` were already kind-agnostic --
/// only this function's own arm list was incomplete). Added here using the
/// exact same `nominal_or_canonical_subtype` termination the func arms now
/// also use, generalized rather than duplicated.
fn is_assignable(actual: ValueType, expected: ValueType, module: TypeContext) -> bool {
    actual == expected
        || matches!((actual, expected), (ValueType::ConcreteFuncRef(_), ValueType::Funcref))
        || actual.is_bottom_subtype_of(&expected)
        // W32 second slice: `NonNullStructRef(i) <: StructRef(i) <: Anyref`,
        // `NonNullConcreteFuncRef(i) <: ConcreteFuncRef(i) <: Funcref` --
        // see `ValueType::is_non_null_subtype_of`'s own doc comment for why
        // both hops of each chain are direct rules, not composed ones.
        || actual.is_non_null_subtype_of(&expected)
        // W33 first slice / W34 third slice: a reference to a DECLARED
        // NOMINAL SUBTYPE, or a CANONICALLY EQUIVALENT type, flows wherever
        // a reference to the expected type is required -- `(ref $t2)`/`(ref
        // null $t2)` is assignable to a `(ref $t1)`/`(ref null $t1)` slot
        // whenever `$t2 <: $t1` per the module's own `sub` chain OR `$t2`/
        // `$t1` are canonically the same type despite having no declared
        // relationship at all (`type-subtyping.wast`'s "Subsumption"
        // section combined with `type-rec.wast`'s "Static/Dynamic matching"
        // sections -- see `code/specs/
        // W34-wasm-gc-canonical-type-equivalence.md`'s own worked example).
        || matches!((actual, expected), (ValueType::ConcreteFuncRef(i), ValueType::ConcreteFuncRef(j)) if module.nominal_or_canonical_subtype(i, j))
        || matches!((actual, expected), (ValueType::NonNullConcreteFuncRef(i), ValueType::NonNullConcreteFuncRef(j)) if module.nominal_or_canonical_subtype(i, j))
        || matches!((actual, expected), (ValueType::NonNullConcreteFuncRef(i), ValueType::ConcreteFuncRef(j)) if module.nominal_or_canonical_subtype(i, j))
        // W34 third slice: the struct/array analogues of the three func
        // arms above -- see this function's own doc comment for why these
        // didn't exist at all before this slice.
        || matches!((actual, expected), (ValueType::StructRef(i), ValueType::StructRef(j)) if module.nominal_or_canonical_subtype(i, j))
        || matches!((actual, expected), (ValueType::NonNullStructRef(i), ValueType::NonNullStructRef(j)) if module.nominal_or_canonical_subtype(i, j))
        || matches!((actual, expected), (ValueType::NonNullStructRef(i), ValueType::StructRef(j)) if module.nominal_or_canonical_subtype(i, j))
        || matches!((actual, expected), (ValueType::ArrayRef(i), ValueType::ArrayRef(j)) if module.nominal_or_canonical_subtype(i, j))
        || matches!((actual, expected), (ValueType::NonNullArrayRef(i), ValueType::NonNullArrayRef(j)) if module.nominal_or_canonical_subtype(i, j))
        || matches!((actual, expected), (ValueType::NonNullArrayRef(i), ValueType::ArrayRef(j)) if module.nominal_or_canonical_subtype(i, j))
        // W37 (`code/specs/W37-wasm-gc-reftype-tables.md`): the two new
        // abstract hierarchy tops' own subtyping edges. The real GC
        // proposal's own hierarchy is `struct <: eq <: any` (and
        // symmetrically `array <: eq <: any`, `i31 <: eq <: any` --
        // `I31ref`'s own missing `<: Anyref`/`<: Eqref` edges are a
        // separate, PRE-EXISTING gap this spec's own research flagged but
        // did not fix, since no table declaration in this spec's corpus
        // cluster needs it -- see `code/specs/
        // W37-wasm-gc-reftype-tables.md`'s design section 1). Each hop is
        // listed directly, matching every other chain in this function's
        // own "no transitive closure" convention (`NonNullStructRef(_) <:
        // StructRef(_) <: Anyref` above is the same shape one hierarchy
        // over): a concrete struct reference (nullable OR non-null) is
        // assignable to the new abstract STRUCT top (not just the
        // pre-existing `Anyref` arm), and `structref <: eqref <: anyref`.
        //
        // Corpus-verified need: `ref_cast.wast`'s own `(table 20 (ref null
        // struct))` + `(table.set ... (struct.new_default $t))` requires
        // exactly the `NonNullStructRef(_), StructRefAny` arm below for
        // that module to type-check at all. The remaining arms complete
        // the hierarchy per the real spec's own subtyping rules but are
        // not yet exercised by any vendored corpus fixture -- included for
        // correctness (a `structref`/`eqref`-typed table, global, or
        // struct field must type-check per the real rules regardless of
        // which specific fixture happens to probe it first), not because
        // today's corpus demands each one individually.
        || matches!((actual, expected), (ValueType::StructRef(_), ValueType::StructRefAny))
        || matches!((actual, expected), (ValueType::NonNullStructRef(_), ValueType::StructRefAny))
        || matches!((actual, expected), (ValueType::StructRef(_), ValueType::Eqref))
        || matches!((actual, expected), (ValueType::NonNullStructRef(_), ValueType::Eqref))
        || matches!((actual, expected), (ValueType::Eqref, ValueType::Anyref))
        || matches!((actual, expected), (ValueType::StructRefAny, ValueType::Eqref))
        || matches!((actual, expected), (ValueType::StructRefAny, ValueType::Anyref))
}

/// Require an already-popped [`StackType`] to be assignable to `expected`
/// (an `Unknown` actual or expected always matches -- see [`pop_val`]; a
/// `Known` actual must satisfy [`is_assignable`], not bare equality, so a
/// concrete function-type ref can flow wherever `funcref` is expected).
/// Factored out of [`pop_expect`] so a caller that already HAS a
/// `StackType` in hand (`br_table`'s multi-target check below: the SAME
/// small, already-popped operand values must be checked against several
/// DIFFERENT targets) can reuse this exact assignability logic without
/// re-popping from -- or cloning -- the real stack for every target.
fn check_stacktype_assignable(actual: StackType, expected: ValueType, module: TypeContext<'_>) -> Result<(), ValidationError> {
    match actual {
        StackType::Unknown => Ok(()),
        StackType::Known(actual) if is_assignable(actual, expected, module) => Ok(()),
        StackType::Known(actual) => Err(ValidationError::Other(format!(
            "TypeMismatch: expected {expected:?}, found {actual:?}"
        ))),
    }
}

/// Pop one value and require it to be assignable to `expected`.
fn pop_expect(stack: &mut Vec<StackType>, frame: &ControlFrame, expected: ValueType, module: TypeContext<'_>) -> Result<(), ValidationError> {
    let actual = pop_val(stack, frame)?;
    check_stacktype_assignable(actual, expected, module)
}

/// Pop and verify a whole type list, in reverse (the last-listed type is
/// on top of the stack -- e.g. `store`'s `[I32, T]` pops `T` first).
fn pop_expect_many(stack: &mut Vec<StackType>, frame: &ControlFrame, expected: &[ValueType], module: TypeContext<'_>) -> Result<(), ValidationError> {
    for &t in expected.iter().rev() {
        pop_expect(stack, frame, t, module)?;
    }
    Ok(())
}

/// Whether `callee_results` -- a tail call's callee's own declared result
/// types -- are a legal stand-in for `function_results`, the CURRENT
/// function's own declared result types (`return_call`/
/// `return_call_indirect`'s own special-cased check, W11 addendum;
/// see [`is_assignable`]). Arity must match exactly (WASM has no
/// result-count subtyping); each pairwise result must be assignable in
/// the same direction `pop_expect` already checks.
fn results_assignable(callee_results: &[ValueType], function_results: &[ValueType], module: TypeContext<'_>) -> bool {
    callee_results.len() == function_results.len()
        && callee_results.iter().zip(function_results.iter()).all(|(&a, &b)| is_assignable(a, b, module))
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
    module: TypeContext<'_>,
) -> Result<(), ValidationError> {
    let outer = control_stack.last().cloned();
    // The enclosing frame is the "current" one for the purposes of popping
    // these params off of it (they live in the enclosing scope until this
    // call moves them into the new one).
    if let Some(outer_frame) = &outer {
        pop_expect_many(stack, outer_frame, &start_types, module)?;
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
            module,
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
fn pop_ctrl(stack: &mut Vec<StackType>, control_stack: &mut Vec<ControlFrame>, module: TypeContext<'_>) -> Result<ControlFrame, ValidationError> {
    let frame = control_stack
        .last()
        .cloned()
        .ok_or_else(|| ValidationError::Other("unexpected `end`/`else`: no open block".to_string()))?;
    pop_expect_many(stack, &frame, &frame.end_types, module)?;
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
fn decode_blocktype(module: TypeContext<'_>, code: &[u8], offset: usize) -> Result<(Vec<ValueType>, Vec<ValueType>, usize), ValidationError> {
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
        // The four W32-first-slice bottom reference types (`code/specs/
        // W32-wasm-non-null-concrete-reference-types.md`): same explicit-
        // arm treatment as `exnref` (`0x69`) directly above, for the same
        // reason -- each byte is a plausible real type-section index, and
        // each one's LEB128 continuation bit is independently verified
        // clear (see `ValueType::NullFuncref`'s own doc comment), so it's
        // safe to special-case rather than falling into the type-index
        // branch below.
        0x73 => Ok((vec![], vec![ValueType::NullFuncref], 1)),
        0x72 => Ok((vec![], vec![ValueType::NullExternref], 1)),
        0x74 => Ok((vec![], vec![ValueType::NullExnref], 1)),
        0x71 => Ok((vec![], vec![ValueType::NullRef], 1)),
        // `(ref null $t)` / `(ref $t)` as a single-value blocktype result
        // (W32 second slice: real corpus regression found, not a
        // hypothetical -- `ref.wast`'s own `block-result-invalid`/
        // `loop-result-invalid` cases, `(block (result (ref 1)) ...)`,
        // started silently structurally validating instead of being
        // rejected once this slice's `(ref $t)` TEXT parsing made this
        // shape reachable). Same real gap `exnref`'s `0x69` fix and the
        // four bottom types above already closed for THEIR bytes: `0x63`/
        // `0x64` are ALSO plausible real type-section indices when
        // (mis)decoded as a bare signed LEB128 byte by the generic
        // fallback below, and BOTH carry a trailing LEB128 type index
        // that fallback does not know to skip -- on a module with 100+
        // declared types this would silently desynchronize the rest of
        // the instruction stream instead of erroring.
        //
        // Resolves to `ConcreteFuncRef`/`NonNullConcreteFuncRef`
        // specifically, never `StructRef`/`NonNullStructRef` -- same
        // reasoning `ref.null`'s own `0xD0` handler documents: this
        // crate's `wasm-wast-parser` has no struct-type TEXT-format
        // declarations at all, so no real `.wast` source can put a
        // struct-type index here. The index IS bounds-checked (unlike
        // `ref.null`'s permissive out-of-range-falls-back-to-Unknown
        // convention) because a blocktype has no `Unknown` fallback to
        // fall back TO -- it must resolve to a real, concrete result
        // type list or the block cannot be type-checked at all, so an
        // out-of-range index here is unconditionally a real error
        // (exactly the "unknown type" `ref.wast` itself expects).
        0x63 => {
            let (idx, size) = decode_idx(code, offset + 1)?;
            if idx as usize >= module.types.len() {
                return Err(ValidationError::TypeIndexOutOfBounds(format!(
                    "blocktype references type index {idx}, but only {} types exist",
                    module.types.len()
                )));
            }
            Ok((vec![], vec![ValueType::ConcreteFuncRef(idx)], 1 + size))
        }
        0x64 => {
            let (idx, size) = decode_idx(code, offset + 1)?;
            if idx as usize >= module.types.len() {
                return Err(ValidationError::TypeIndexOutOfBounds(format!(
                    "blocktype references type index {idx}, but only {} types exist",
                    module.types.len()
                )));
            }
            Ok((vec![], vec![ValueType::NonNullConcreteFuncRef(idx)], 1 + size))
        }
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

/// Decode one plain `valtype` immediate byte -- e.g. one element of typed
/// `select`'s (`0x1C`) `vec(valtype)` immediate. Shares `decode_blocktype`'s
/// byte-tag table above for every single-byte numtype/vectype/reftype, and
/// its bounds-checked `0x63`/`0x64` concrete-reference handling, but is NOT
/// the same decode: a standalone `valtype` has no "lone byte OR a real
/// type-section index" ambiguity the way a *blocktype* does (that shorthand
/// only exists for blocktypes -- see `decode_blocktype`'s own doc comment),
/// so any byte this table doesn't recognize is unconditionally a malformed
/// immediate here, never "maybe a type index" the way `decode_blocktype`'s
/// catch-all arm treats it.
fn decode_valtype(module: TypeContext<'_>, code: &[u8], offset: usize) -> Result<(ValueType, usize), ValidationError> {
    let byte = *code.get(offset).ok_or_else(|| ValidationError::Other("truncated valtype immediate".to_string()))?;
    match byte {
        0x7F => Ok((ValueType::I32, 1)),
        0x7E => Ok((ValueType::I64, 1)),
        0x7D => Ok((ValueType::F32, 1)),
        0x7C => Ok((ValueType::F64, 1)),
        0x7B => Ok((ValueType::V128, 1)),
        0x70 => Ok((ValueType::Funcref, 1)),
        0x6F => Ok((ValueType::Externref, 1)),
        0x6E => Ok((ValueType::Anyref, 1)),
        0x6C => Ok((ValueType::I31ref, 1)),
        0x69 => Ok((ValueType::Exnref, 1)),
        0x73 => Ok((ValueType::NullFuncref, 1)),
        0x72 => Ok((ValueType::NullExternref, 1)),
        0x74 => Ok((ValueType::NullExnref, 1)),
        0x71 => Ok((ValueType::NullRef, 1)),
        0x63 => {
            let (idx, size) = decode_idx(code, offset + 1)?;
            if idx as usize >= module.types.len() {
                return Err(ValidationError::TypeIndexOutOfBounds(format!(
                    "select result type references type index {idx}, but only {} types exist",
                    module.types.len()
                )));
            }
            Ok((ValueType::ConcreteFuncRef(idx), 1 + size))
        }
        0x64 => {
            let (idx, size) = decode_idx(code, offset + 1)?;
            if idx as usize >= module.types.len() {
                return Err(ValidationError::TypeIndexOutOfBounds(format!(
                    "select result type references type index {idx}, but only {} types exist",
                    module.types.len()
                )));
            }
            Ok((ValueType::NonNullConcreteFuncRef(idx), 1 + size))
        }
        other => Err(ValidationError::Other(format!("invalid valtype byte 0x{other:02X}"))),
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

/// Ceiling on `array.new_fixed`'s literal element-count immediate this
/// validator will actually iterate over (W33 fourth slice) -- see that
/// opcode's own match arm for why an unbounded loop over an
/// attacker-controlled count is a real algorithmic DoS, not a hypothetical
/// one, even though no single iteration allocates memory.
const MAX_ARRAY_NEW_FIXED_COUNT: u32 = 1_000_000;

/// The declared field count of the WasmGC struct type at type-section index
/// `type_idx` -- how many values `struct.new` pops. Resolved via
/// `WasmModule::struct_type_at` (W33 fourth slice: `type_kinds`-aware first,
/// falling back to the legacy `types.len() + k` offset convention -- see
/// that method's own doc comment) rather than re-deriving the offset
/// directly here, since a TEXT-format module can now interleave struct
/// declarations among func ones in a way the legacy formula alone cannot
/// represent.
fn struct_field_count(module: TypeContext<'_>, type_idx: u32) -> Result<usize, ValidationError> {
    let st = module
        .struct_type_at(type_idx)
        .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("struct.new references struct type index {type_idx}, but no such struct type exists")))?;
    Ok(st.fields.len())
}

/// The array-type analogue of [`struct_field_count`]: the element
/// [`StorageType`]/mutability of the WasmGC array type at flat type-section
/// index `type_idx` (W33 fourth slice) -- resolved the same
/// `type_kinds`-aware-first way via [`WasmModule::array_type_at`].
///
/// Reaches through `module.module` (the wrapped `&'a WasmModule` field)
/// rather than `module.array_type_at(..)` directly (which WOULD also
/// compile, via [`TypeContext`]'s `Deref`, but would tie the returned
/// `&FieldType`'s lifetime to this function's own ephemeral internal
/// auto-ref of the by-value `module: TypeContext<'_>` parameter -- one
/// step shorter than the real `'a` this reference actually lives for, a
/// real, easy-to-miss lifetime trap for any `Deref`-based wrapper handing
/// back a borrowed reference, not merely a style preference). Every OTHER
/// function in this file that reaches through `module.<method>` only ever
/// consumes the result immediately (an owned/`Copy` value, or a reference
/// used and dropped within the SAME function body), so this trap doesn't
/// apply to them -- see [`struct_field`] just below for the identical fix.
fn array_element_field(module: TypeContext<'_>, type_idx: u32) -> Result<&FieldType, ValidationError> {
    module
        .module
        .array_type_at(type_idx)
        .map(|at| &at.element)
        .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("references array type index {type_idx}, but no such array type exists")))
}

/// One field of the WasmGC struct type at flat type-section index
/// `type_idx` (W33 fourth slice) -- `struct.set`'s own bounds AND
/// mutability check both need the real [`FieldType`], not just the field
/// count [`struct_field_count`] returns. See [`array_element_field`]'s own
/// doc comment for why this reaches through `module.module` explicitly.
fn struct_field(module: TypeContext<'_>, type_idx: u32, field_idx: u32) -> Result<&FieldType, ValidationError> {
    let st = module
        .module
        .struct_type_at(type_idx)
        .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("references struct type index {type_idx}, but no such struct type exists")))?;
    st.fields.get(field_idx as usize).ok_or_else(|| {
        ValidationError::TypeIndexOutOfBounds(format!(
            "struct.set references field index {field_idx} of type {type_idx}, but only {} fields exist",
            st.fields.len()
        ))
    })
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
fn type_check_numeric(stack: &mut Vec<StackType>, frame: &ControlFrame, name: &str, stack_pop: u8, module: TypeContext<'_>) -> Result<(), ValidationError> {
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
        pop_expect(stack, frame, operand_type, module)?;
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
    /// W34 third slice: `module` is now a [`TypeContext`], bundling the raw
    /// `&'a WasmModule` together with this module's own canonicalized
    /// type-group forms (computed once in [`type_check_module`], right
    /// after [`check_type_subtyping`] confirms the `sub`/`rec` ordering is
    /// well-founded) -- see that type's own doc comment for why every
    /// existing `ctx.module.<field/method>` access below still compiles
    /// unchanged.
    module: TypeContext<'a>,
    /// Combined imported + module-defined function types, indexed by the
    /// combined function index space (imports first, matching every other
    /// index space in the binary format).
    func_types: Vec<FuncType>,
    /// Each function's OWN declared type-SECTION index, same combined
    /// imports-first-then-declared index space as `func_types` (parallel
    /// array, same length, built in lockstep). Needed for `ref.func`'s
    /// real spec typing rule (W32 second slice: `code/specs/
    /// W32-wasm-non-null-concrete-reference-types.md`): `ref.func $f :
    /// [] -> [(ref $t)]` where `$t` is `$f`'s own function-type index
    /// (verified against WebAssembly/function-references's own
    /// `Overview.md`) -- `func_types[i]` alone gives the RESOLVED
    /// `FuncType` (params/results), which isn't enough to build a
    /// `ValueType::NonNullConcreteFuncRef(idx)` naming the type-SECTION
    /// index specifically (two different functions can declare
    /// byte-identical signatures at two different type-section indices,
    /// and the real spec's `ref.func` result must name the ACTUAL index
    /// the function declared, not merely an equal-looking one -- this
    /// repo's own scope explicitly excludes structural type-equivalence,
    /// see the spec's "explicitly out of scope" section).
    func_type_indices: Vec<u32>,
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
    /// Each table's fully-resolved element type -- `Funcref`/`Externref`
    /// for an ordinary table, or the table's own concrete
    /// `ConcreteFuncRef`/`NonNullConcreteFuncRef` when the source declared
    /// one via `(table $t (ref null $t) ...)` (function-references
    /// proposal; see `WasmModule::table_concrete_element_types`'s own doc
    /// comment) -- same index-space/ordering convention as `table_count`
    /// (task #96): `table.get $t3 ...`/`table.set $t3 ...` must type-check
    /// against table `$t3`'s OWN declared element type, not unconditionally
    /// assume funcref -- WASM 1.0's single implicit table is always
    /// funcref, but a module with more than one table (multi-table) can
    /// mix funcref and externref tables freely, and a function-references
    /// table narrows funcref further still. This used to be just the raw
    /// `u8` tag (`0x70`/`0x6F`) with a `0x6F => Externref, _ => Funcref`
    /// match at every use site, which silently discarded any concrete
    /// type and is exactly what let `br_table.wast`'s own `meet-funcref-*`/
    /// `meet-multi-ref` tests regress (`table.get` on a `(ref null $t)`
    /// table pushed generic `Funcref`, then a `br_table` label requiring
    /// the narrower `$t` failed with a spurious `TypeMismatch`).
    table_element_types: Vec<ValueType>,
    /// Each table's `is64`-ness (table64 proposal, W26), same combined
    /// imports-first-then-declared index-space/ordering convention as
    /// `table_element_types`/`memory_is64` -- `table.get`/`table.set`/
    /// `table.grow`/`table.size`/`table.fill`/`table.init`/`table.copy`/
    /// `call_indirect`/`return_call_indirect` must type-check their
    /// index/dest/src/len/delta operands (and pushed results) against the
    /// TARGET table's own `is64`, not unconditionally assume `i32` (see
    /// `code/specs/W26-wasm-table64-first-slice.md`'s follow-up "real
    /// table64 operations" scope).
    table_is64: Vec<bool>,
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

fn build_module_context<'a>(module: &'a WasmModule, canonical_types: &'a [Option<(Rc<CanonicalGroup>, u32)>]) -> Result<ModuleContext<'a>, ValidationError> {
    use wasm_types::ImportTypeInfo;

    let mut func_types = Vec::new();
    let mut func_type_indices = Vec::new();
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
                func_type_indices.push(*type_idx);
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
        func_type_indices.push(type_idx);
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
    // Import tables can only ever be generic funcref/externref in this
    // crate's text format (no concrete-typed table IMPORT syntax exists --
    // see `WasmModule::table_concrete_element_types`'s own doc comment),
    // so each one resolves from its raw byte tag alone.
    let byte_to_generic_reftype = |b: u8| if b == wasm_types::EXTERNREF { ValueType::Externref } else { ValueType::Funcref };
    let mut table_element_types: Vec<ValueType> = module
        .imports
        .iter()
        .filter_map(|i| match &i.type_info {
            ImportTypeInfo::Table(tt) => Some(byte_to_generic_reftype(tt.element_type)),
            _ => None,
        })
        .collect();
    // Module-defined tables: prefer the real concrete type
    // (`table_concrete_element_types[i]`) when the source declared one,
    // falling back to the generic byte tag otherwise -- exactly the
    // `Some`/`None` split `table_concrete_element_types`'s own doc comment
    // describes. `.get(i)` (not indexing) because that vec is allowed to
    // be shorter than `module.tables`, same convention `type_kinds`/
    // `type_subtyping` already established elsewhere in this module.
    table_element_types.extend(module.tables.iter().enumerate().map(|(i, t)| {
        module
            .table_concrete_element_types
            .get(i)
            .copied()
            .flatten()
            .unwrap_or_else(|| byte_to_generic_reftype(t.element_type))
    }));
    let table_count = table_element_types.len() as u32;
    let mut table_is64: Vec<bool> = module
        .imports
        .iter()
        .filter_map(|i| match &i.type_info {
            ImportTypeInfo::Table(tt) => Some(tt.is64),
            _ => None,
        })
        .collect();
    table_is64.extend(module.tables.iter().map(|t| t.is64));

    Ok(ModuleContext {
        module: TypeContext { module, canonical_types },
        func_types,
        func_type_indices,
        global_types,
        has_memory,
        memory_count,
        memory_is64,
        table_count,
        table_element_types,
        table_is64,
        tag_types,
    })
}

/// Type-check every function body in `module`. The first ill-typed
/// function (by index) determines the error.
/// Validates every declared `(sub [final] $parent (func ...))` relationship
/// in `module.types`/`module.type_subtyping` (W33 first slice: `code/specs/
/// W33-wasm-gc-recursive-type-subtyping.md`) -- WITHIN this one module's own
/// type section, by absolute type-section index (never cross-module; the
/// canonical type-group equivalence that would need is item (3b), this
/// slice's own explicitly out-of-scope piece).
///
/// Two things reject a declared `sub $parent`:
/// - **`$parent` is final** (`type-subtyping.wast`'s own "Finality
///   violation" section, lines 780-811): a type with no `sub` clause at
///   all, or an explicit `(sub final ...)`, forecloses further subtyping,
///   checked here rather than merely documented.
/// - **The declared child/parent pair doesn't satisfy the GC proposal's
///   real structural function-subtyping rule** (`func_is_structural_
///   subtype`'s own doc comment) -- invariant arity, contravariant params,
///   covariant results (`type-subtyping.wast` lines 944-949's arity-
///   mismatch case, the pure-func member of its "Invalid subtyping
///   definitions" section — every OTHER case there targets a struct/array
///   body, which stays unparseable and therefore unreachable here, see
///   `wasm-wast-parser`'s own struct/array doc comments).
///
/// **W34 third slice** (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`):
/// struct/array composite-type-kind invariance and field-list
/// width/depth/variance rules ARE now checked here too, dispatching on each
/// side's real [`TypeKind`] (`Func`/`Struct`/`Array`) instead of always
/// reading `module.types[i]` as the real body. Before this slice, a
/// struct/array-kind flat index's `module.types[i]` slot is an unused,
/// never-populated dummy `FuncType { params: vec![], results: vec![] }`
/// (see [`TypeKind`]'s own doc comment) -- so a declared `(sub $parent
/// (struct ...))`/`(sub $parent (array ...))` relationship used to be
/// checked against TWO EMPTY func signatures (trivially "compatible": zero
/// arity, vacuously contravariant/covariant) instead of the real
/// field/element lists, a real, previously-open correctness gap the W34
/// spec's own research flagged (pre-dating W34 entirely -- a W33-first-
/// slice-era gap struct/array TEXT-format parsing, shipped in W33's fourth
/// slice, made newly REACHABLE without fixing). See
/// [`struct_is_structural_subtype`]/[`array_is_structural_subtype`] for the
/// real width/depth/variance rules now applied. A declared `sub`
/// relationship between two DIFFERENT composite-type kinds (e.g. a struct
/// declaring a func parent) is always rejected -- the GC proposal has no
/// such cross-kind subtyping relation at all.
fn check_type_subtyping(module: &WasmModule, canonical_types: &[Option<(Rc<CanonicalGroup>, u32)>]) -> Result<(), ValidationError> {
    // W34 third slice -- CORRECTION found by re-verifying against the real
    // corpus, not by assumption (this campaign's own discipline): an
    // EARLIER version of this function's own doc comment claimed composite-
    // type structural-subtype variance (this function's own job) is
    // "orthogonal to canonical equivalence," and ran this whole check
    // BEFORE `canonical_types` existed, passing an EMPTY table into every
    // `is_assignable` call reachable from here. That claim is WRONG:
    // `type-subtyping.wast`'s own "Static matching of recursive types"
    // module (a plain `module` directive, no `assert_invalid` -- MUST
    // validate) declares `$f1`/`$f2` as two SEPARATE, differently-indexed,
    // multi-member `rec` groups with NO shared `sub` relationship at all,
    // yet structurally IDENTICAL shapes -- canonically the SAME type. A
    // later struct's declared `sub $s2 (struct (field (ref $f1) ...))`
    // relationship requires `(ref $f1) <: (ref $s2)`'s own first field
    // `(ref $f2)` to hold via CANONICAL equivalence (`$f1 == $f2`, no
    // nominal chain exists between them) -- exactly the case an empty
    // canonical table cannot satisfy. `canonical_types` is threaded in as
    // a real parameter now (computed by the caller right after `check_
    // type_subtyping_is_acyclic` -- hoisted OUT of this function, since its
    // own ordering guarantee is what canonicalization's termination
    // argument depends on -- succeeds), so this function's own struct/array
    // field-covariance checks (via `func_is_structural_subtype`/
    // `field_is_structural_subtype`) get the SAME real canonical data
    // `is_assignable`'s other callers do.
    let tc = TypeContext { module, canonical_types };
    for i in 0..module.types.len() as u32 {
        let Some(parent_idx) = module.type_subtyping_at(i).supertype else {
            continue;
        };
        if parent_idx as usize >= module.types.len() {
            return Err(ValidationError::TypeIndexOutOfBounds(format!(
                "type #{i} declares supertype index {parent_idx}, but only {} types exist",
                module.types.len()
            )));
        }
        if module.type_subtyping_at(parent_idx).is_final {
            return Err(ValidationError::Other(format!("sub type: type #{i} declares #{parent_idx} as its supertype, but #{parent_idx} is final")));
        }
        let child_kind = module.type_kinds.get(i as usize).copied().unwrap_or(TypeKind::Func);
        let parent_kind = module.type_kinds.get(parent_idx as usize).copied().unwrap_or(TypeKind::Func);
        let ok = match (child_kind, parent_kind) {
            (TypeKind::Func, TypeKind::Func) => func_is_structural_subtype(&module.types[i as usize], &module.types[parent_idx as usize], tc),
            (TypeKind::Struct(ck), TypeKind::Struct(pk)) => match (module.struct_types.get(ck as usize), module.struct_types.get(pk as usize)) {
                (Some(c), Some(p)) => struct_is_structural_subtype(c, p, tc),
                _ => false,
            },
            (TypeKind::Array(ck), TypeKind::Array(pk)) => match (module.array_types.get(ck as usize), module.array_types.get(pk as usize)) {
                (Some(c), Some(p)) => array_is_structural_subtype(c, p, tc),
                _ => false,
            },
            // A struct declaring a func/array parent (or any other
            // cross-kind mix) has no legal subtyping relation at all --
            // the real GC proposal never relates different composite-type
            // kinds this way.
            _ => false,
        };
        if !ok {
            return Err(ValidationError::Other(format!(
                "sub type: type #{i} is not a valid structural subtype of its declared supertype #{parent_idx} (arity/field-count must match kind-appropriate rules: func -- contravariant params, covariant results; struct -- width+per-field covariant/invariant; array -- element covariant/invariant)"
            )));
        }
    }
    Ok(())
}

/// Rejects a cyclic `sub` chain anywhere in `module.types` (security
/// review finding, W33 first slice -- see `check_type_subtyping`'s own
/// doc comment for why this matters). Each type has AT MOST one declared
/// `sub` parent, so the whole `supertype` relation forms a "functional
/// graph" (out-degree <= 1 everywhere) -- a real, spec-legal such graph is
/// a forest of chains all eventually reaching a type with `supertype:
/// None`; a cycle means at least one chain never terminates.
///
/// Uses the standard three-color (white/gray/black) traversal, giving
/// O(number of types) total work: every type is colored exactly once,
/// and a follow-up traversal starting from an already-BLACK type stops
/// immediately rather than re-walking a chain already proven acyclic.
/// This keeps the check itself cheap even for a module with many
/// separate chains -- deliberately NOT the O(chain depth) per-query cost
/// `WasmModule::func_type_is_nominal_subtype` has (that one is a
/// different, already-addressed finding -- see its own doc comment for
/// the fixed hop bound).
fn check_type_subtyping_is_acyclic(module: &WasmModule) -> Result<(), ValidationError> {
    const WHITE: u8 = 0; // not yet visited
    const GRAY: u8 = 1; // on the current traversal's path
    const BLACK: u8 = 2; // fully processed, already proven acyclic

    let n = module.types.len();
    let mut color = vec![WHITE; n];
    for start in 0..n {
        if color[start] != WHITE {
            continue;
        }
        let mut path = Vec::new();
        let mut cur = start;
        loop {
            match color[cur] {
                WHITE => {
                    color[cur] = GRAY;
                    path.push(cur);
                }
                GRAY => {
                    return Err(ValidationError::Other(format!("sub type: type #{cur}'s declared supertype chain is cyclic")));
                }
                _ => break, // BLACK: already proven acyclic from here on
            }
            match module.type_subtyping_at(cur as u32).supertype {
                Some(parent) if (parent as usize) < n => cur = parent as usize,
                // No parent, or an out-of-range one (a different error
                // the per-type loop below reports precisely) -- either
                // way, this chain terminates here.
                _ => break,
            }
        }
        for node in path {
            color[node] = BLACK;
        }
    }
    Ok(())
}

/// The GC proposal's real function-type structural subtyping rule --
/// **invariant arity, contravariant params, covariant results** -- NOT
/// function-references' narrower "invariant for now" rule (`code/specs/
/// W33-wasm-gc-recursive-type-subtyping.md`'s own "Why this needs GC, not
/// function-references" section verified this against the real
/// `WebAssembly/function-references` `Overview.md` text; not re-litigated
/// here).
///
/// `type-subtyping.wast` lines 28-31 demonstrate the real rule directly:
/// `$f1 (func (param (ref $s')) (result anyref))`, its declared sub `$f2
/// (func (param (ref $s)) (result (ref any)))` -- `$s <: $s'` is FALSE
/// ($s' is $s's own sub), yet accepting the WIDER `(ref $s)` param in the
/// subtype is correct because params are contravariant; the result
/// position (`(ref any) <: anyref`) is covariant.
fn func_is_structural_subtype(child: &FuncType, parent: &FuncType, module: TypeContext<'_>) -> bool {
    child.params.len() == parent.params.len()
        && child.results.len() == parent.results.len()
        // Contravariant: the PARENT's param must be assignable to the
        // CHILD's param slot (the child may accept a WIDER param type).
        && child.params.iter().zip(parent.params.iter()).all(|(&c, &p)| is_assignable(p, c, module))
        // Covariant: the CHILD's result must be assignable to the
        // PARENT's result slot (the child may return something NARROWER).
        && child.results.iter().zip(parent.results.iter()).all(|(&c, &p)| is_assignable(c, p, module))
}

/// **W34 third slice**: one struct/array FIELD's own structural subtyping
/// rule, per the real GC proposal's `##### Composite Types` text --
/// mutability must match exactly (a field can't gain OR lose mutability
/// through a `sub` relationship), and:
/// - **mutable fields are INVARIANT**: the storage type must match
///   EXACTLY (a mutable field can be both read and written, so neither a
///   wider-on-read nor a wider-on-write relaxation is sound).
/// - **immutable fields are COVARIANT**: the child's storage type need
///   only be [`is_assignable`] to the parent's (an immutable field is
///   read-only, so a NARROWER child value type is a sound stand-in
///   wherever the parent's wider type is expected).
///
/// Packed storage (`i8`/`i16`) has no further covariance of its own within
/// this crate's `StorageType` (there's no narrower-than-`i8` packed width
/// to be covariant WITH) -- two packed fields are compatible only when
/// their packed width matches exactly, mutable or not, which falls out of
/// this same rule for the mutable case and (since `StorageType::I8 ==
/// StorageType::I8` is the only way two packed variants satisfy
/// `is_assignable`, which never relates `I8`/`I16` to anything else) for
/// the immutable case too.
fn field_is_structural_subtype(child: &FieldType, parent: &FieldType, module: TypeContext<'_>) -> bool {
    if child.mutable != parent.mutable {
        return false;
    }
    if child.mutable {
        child.storage == parent.storage
    } else {
        match (child.storage, parent.storage) {
            (StorageType::Val(c), StorageType::Val(p)) => is_assignable(c, p, module),
            (StorageType::I8, StorageType::I8) | (StorageType::I16, StorageType::I16) => true,
            _ => false,
        }
    }
}

/// **W34 third slice**: the GC proposal's real struct structural subtyping
/// rule -- **width subtyping** (the child may declare MORE fields than the
/// parent; every extra trailing field is simply not visible through the
/// parent's own type) plus, for every field position the parent DOES
/// declare, [`field_is_structural_subtype`]'s per-field variance rule, in
/// declaration order (struct fields are positional, not named, at the
/// binary/structural level -- `type-subtyping.wast`'s own struct/array
/// "Invalid subtyping definitions" cases this closes are exactly these
/// two rules: a child with FEWER fields than its declared parent, and a
/// mutable field whose storage type merely widens instead of matching
/// exactly).
fn struct_is_structural_subtype(child: &StructType, parent: &StructType, module: TypeContext<'_>) -> bool {
    child.fields.len() >= parent.fields.len() && parent.fields.iter().zip(child.fields.iter()).all(|(p, c)| field_is_structural_subtype(c, p, module))
}

/// **W34 third slice**: the GC proposal's real array structural subtyping
/// rule -- an array type is, structurally, a single [`FieldType`] (see
/// [`ArrayType`]'s own doc comment for why this crate reuses `FieldType`
/// directly rather than a bespoke element-type shape), so array subtyping
/// is exactly [`field_is_structural_subtype`] applied once to the two
/// arrays' own `element` fields -- no width dimension exists for an array
/// (it has exactly one field position, always).
fn array_is_structural_subtype(child: &ArrayType, parent: &ArrayType, module: TypeContext<'_>) -> bool {
    field_is_structural_subtype(&child.element, &parent.element, module)
}

// ──────────────────────────────────────────────────────────────────────────────
// Const-expression type-checking (global initializers, element/data-segment
// offsets) -- a real, pre-existing, previously-unfilled gap
// ──────────────────────────────────────────────────────────────────────────────
//
// This crate had NO const-expr type-checker at all before this section:
// `crate::validate`'s "Check 4c" bounds-checks a global's OWN declared
// `ConcreteFuncRef`/`NonNullConcreteFuncRef` type INDEX, but nothing ever
// compared the VALUE an `init_expr`/`offset_expr` actually produces against
// that declared type. Surfaced by `code/specs/
// W33-wasm-gc-recursive-type-subtyping.md`'s "A newly-discovered, THIRD
// gap" addendum while tracing two honest reclassifications in
// `type-rec.wast`/`type-subtyping.wast` -- confirmed independently here
// (not just trusted from that doc): grepping this crate for any production
// read of `globals[..].init_expr`/`elements[..].offset_expr`/
// `data[..].offset_expr` outside test fixtures found none before this
// section existed. This predates W33 entirely and would affect even a
// plain MVP `(global i32 (i64.const 0))` mismatch (declared `i32`,
// initialized with an `i64` constant) -- confirmed via the real corpus:
// `global.wast` alone has 16 `assert_invalid` directives probing exactly
// this class of gap (type mismatch, an illegal non-const opcode, a
// forward/self/out-of-range `global.get`, and a `global.get` onto a
// mutable global), every one of which structurally validated fine before
// this section existed (graded `NotYetSupported`, not `Fail` -- see
// `wasm-conformance`'s own `grade_assert_invalid` doc comment for why).

/// Determine the static result type of a WASM constant expression --
/// the same opcode set `wasm_execution::evaluate_const_expr` interprets
/// at runtime (see that function's own doc comment for the authoritative
/// allowed-opcode list), computing a TYPE instead of executing arithmetic.
///
/// `global_limit` bounds which `global.get` indices are legal here, per
/// the real spec's two distinct visibility rules for constant expressions:
/// - `Some(n)`: this expression IS a global's own initializer, at
///   combined (imports-first) index `n` -- only indices strictly less
///   than `n` are "prior" (forward references, including a global
///   referencing itself, are invalid: the global section initializes in
///   order, so a later/self index names a global that doesn't exist yet).
/// - `None`: this expression is an element- or data-segment offset. Both
///   sections come after the ENTIRE global section in a module's binary
///   layout, so by the time either runs every declared global (import or
///   module-defined) already exists -- eligibility is bounds alone, via
///   `ctx.global_types.len()`.
///
/// A referenced global must also be IMMUTABLE regardless of context
/// (real spec rule, verified directly against `global.wast`'s own
/// `(global (import "test" "global-mut-i32") (mut i32)) (global i32
/// (global.get 0))` `assert_invalid "constant expression required"`
/// case) -- a mutable global's value isn't a compile-time constant.
///
/// Returns `StackType::Unknown` for the one heap-type/struct-index shape
/// the rest of this crate's function-body checker ALSO doesn't fully
/// model yet (`ref.null`'s `0x63`-tagged concrete index at or past
/// `module.types.len()` -- the identical permissive fallback that
/// handler's own doc comment explains, reused here verbatim), so this
/// never introduces a false reject for a shape the rest of the crate
/// already treats as "not fully typed" -- only for shapes it can
/// concretely resolve to a real, wrong type.
fn const_expr_type(expr: &[u8], ctx: &ModuleContext, global_limit: Option<u32>) -> Result<StackType, ValidationError> {
    let mut stack: Vec<StackType> = Vec::new();
    let mut offset = 0usize;

    while offset < expr.len() {
        let opcode = expr[offset];
        offset += 1;

        match opcode {
            // i32.const
            0x41 => {
                let (_, consumed) =
                    decode_signed(expr, offset).map_err(|e| ValidationError::Other(format!("i32.const: {e}")))?;
                offset += consumed;
                stack.push(StackType::Known(ValueType::I32));
            }
            // i64.const
            0x42 => {
                let (_, consumed) =
                    decode_signed(expr, offset).map_err(|e| ValidationError::Other(format!("i64.const: {e}")))?;
                offset += consumed;
                stack.push(StackType::Known(ValueType::I64));
            }
            // f32.const -- 4 raw (non-LEB128) bytes.
            0x43 => {
                if offset + 4 > expr.len() {
                    return Err(ValidationError::Other("f32.const: not enough bytes in constant expression".to_string()));
                }
                offset += 4;
                stack.push(StackType::Known(ValueType::F32));
            }
            // f64.const -- 8 raw bytes.
            0x44 => {
                if offset + 8 > expr.len() {
                    return Err(ValidationError::Other("f64.const: not enough bytes in constant expression".to_string()));
                }
                offset += 8;
                stack.push(StackType::Known(ValueType::F64));
            }
            // global.get -- see this function's own doc comment for the
            // `global_limit`/immutability rules.
            0x23 => {
                let (idx, consumed) = decode_unsigned_bounded(expr, offset, 32)
                    .map_err(|e| ValidationError::Other(format!("global.get: {e}")))?;
                offset += consumed;
                let idx = idx as u32;
                let limit = global_limit.unwrap_or(ctx.global_types.len() as u32);
                if idx >= limit {
                    return Err(ValidationError::Other(format!(
                        "unknown global: constant expression references global index {idx}, but only {limit} prior global(s) are visible here"
                    )));
                }
                let gt = &ctx.global_types[idx as usize];
                if gt.mutable {
                    return Err(ValidationError::Other(format!(
                        "constant expression required: global.get references global index {idx}, which is mutable"
                    )));
                }
                stack.push(StackType::Known(gt.value_type));
            }
            // Extended-const proposal: i32.add/i32.sub/i32.mul -- pop two
            // i32 operands, push one i32 result.
            0x6A..=0x6C => {
                let b = pop_const(&mut stack)?;
                let a = pop_const(&mut stack)?;
                check_const_operand(a, ValueType::I32, ctx.module)?;
                check_const_operand(b, ValueType::I32, ctx.module)?;
                stack.push(StackType::Known(ValueType::I32));
            }
            // Extended-const proposal: i64.add/i64.sub/i64.mul -- same
            // pop-two-push-one shape as the i32 trio just above.
            0x7C..=0x7E => {
                let b = pop_const(&mut stack)?;
                let a = pop_const(&mut stack)?;
                check_const_operand(a, ValueType::I64, ctx.module)?;
                check_const_operand(b, ValueType::I64, ctx.module)?;
                stack.push(StackType::Known(ValueType::I64));
            }
            // v128.const (SIMD, 0xFD-prefixed): sub-opcode is a LEB128 u32,
            // must be 0x0C (the only SIMD sub-opcode legal in a constant
            // expression), followed by 16 RAW lane bytes.
            0xFD => {
                let (sub_opcode, consumed) = decode_unsigned_bounded(expr, offset, 32)
                    .map_err(|e| ValidationError::Other(format!("v128.const: {e}")))?;
                offset += consumed;
                if sub_opcode != 0x0C {
                    return Err(ValidationError::Other(format!(
                        "illegal SIMD sub-opcode 0x{sub_opcode:02X} in constant expression"
                    )));
                }
                if offset + 16 > expr.len() {
                    return Err(ValidationError::Other("v128.const: not enough bytes in constant expression".to_string()));
                }
                offset += 16;
                stack.push(StackType::Known(ValueType::V128));
            }
            // WasmGC prefix (0xFB): `ref.i31` (sub-opcode 0x1C) is the one
            // GC instruction the real spec allows in a constant expression
            // for THIS repo's pre-W33-fourth-slice scope. W33 fourth slice
            // adds the real GC proposal's OTHER constant-legal instructions:
            // `struct.new`/`struct.new_default`/`array.new`/`array.new_
            // default`/`array.new_fixed` -- confirmed directly against the
            // real vendored corpus, not assumed: `struct.wast`'s own
            // `(global (ref $vec) (struct.new $vec ...))`/`(global (ref
            // $vec) (struct.new_default $vec))` and `array.wast`'s
            // `(global (ref $vec) (array.new $vec ...))`/`(global (ref
            // $vec) (array.new_fixed $vec 2 ...))` are all real, VALID
            // (non-`assert_invalid`) module-level globals. `array.new_data`/
            // `array.new_elem` are deliberately excluded (`array.wast`'s own
            // `assert_invalid "constant expression required"` cases probe
            // exactly this) -- moot for now anyway since `wasm-wast-parser`
            // doesn't parse either instruction at all yet (see that crate's
            // own `encode_gc_struct_array_instr` doc comment), so a global
            // using one fails to PARSE long before reaching this checker.
            0xFB => {
                let sub = *expr
                    .get(offset)
                    .ok_or_else(|| ValidationError::Other("truncated WasmGC opcode in constant expression".to_string()))?;
                offset += 1;
                match sub {
                    0x1C => {
                        let v = pop_const(&mut stack)?;
                        check_const_operand(v, ValueType::I32, ctx.module)?;
                        stack.push(StackType::Known(ValueType::I31ref));
                    }
                    0x00 | 0x01 => {
                        // struct.new / struct.new_default <type_idx>.
                        let (type_idx, size) = decode_unsigned_bounded(expr, offset, 32)
                            .map_err(|e| ValidationError::Other(format!("struct.new: {e}")))?;
                        offset += size;
                        let type_idx = type_idx as u32;
                        let st = ctx
                            .module
                            .struct_type_at(type_idx)
                            .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("struct.new references struct type index {type_idx}, but no such struct type exists")))?;
                        if sub == 0x00 {
                            // struct.new: pop one const operand per declared
                            // field, in REVERSE (last field on top).
                            for field in st.fields.iter().rev() {
                                let v = pop_const(&mut stack)?;
                                check_const_operand(v, field.storage.widened_type(), ctx.module)?;
                            }
                        }
                        stack.push(StackType::Known(ValueType::NonNullStructRef(type_idx)));
                    }
                    0x06 | 0x07 => {
                        // array.new / array.new_default <type_idx>: pop
                        // [elem_value?, i32 length].
                        let (type_idx, size) = decode_unsigned_bounded(expr, offset, 32)
                            .map_err(|e| ValidationError::Other(format!("array.new: {e}")))?;
                        offset += size;
                        let type_idx = type_idx as u32;
                        let elem_ty = array_element_field(ctx.module, type_idx)?.storage.widened_type();
                        let len = pop_const(&mut stack)?;
                        check_const_operand(len, ValueType::I32, ctx.module)?;
                        if sub == 0x06 {
                            let v = pop_const(&mut stack)?;
                            check_const_operand(v, elem_ty, ctx.module)?;
                        }
                        stack.push(StackType::Known(ValueType::NonNullArrayRef(type_idx)));
                    }
                    0x08 => {
                        // array.new_fixed <type_idx> <count>: pop `count`
                        // const operands. Same `MAX_ARRAY_NEW_FIXED_COUNT`
                        // DoS guard as the function-body checker's own
                        // identical arm -- see that arm's doc comment.
                        let (type_idx, sz1) = decode_unsigned_bounded(expr, offset, 32)
                            .map_err(|e| ValidationError::Other(format!("array.new_fixed: {e}")))?;
                        offset += sz1;
                        let type_idx = type_idx as u32;
                        let (count, sz2) = decode_unsigned_bounded(expr, offset, 32)
                            .map_err(|e| ValidationError::Other(format!("array.new_fixed count: {e}")))?;
                        offset += sz2;
                        let elem_ty = array_element_field(ctx.module, type_idx)?.storage.widened_type();
                        if count > MAX_ARRAY_NEW_FIXED_COUNT as u64 {
                            return Err(ValidationError::Other(format!(
                                "array.new_fixed count {count} exceeds the maximum of {MAX_ARRAY_NEW_FIXED_COUNT}"
                            )));
                        }
                        for _ in 0..count {
                            let v = pop_const(&mut stack)?;
                            check_const_operand(v, elem_ty, ctx.module)?;
                        }
                        stack.push(StackType::Known(ValueType::NonNullArrayRef(type_idx)));
                    }
                    _ => {
                        return Err(ValidationError::Other(format!(
                            "illegal WasmGC sub-opcode 0x{sub:02X} in constant expression"
                        )));
                    }
                }
            }
            // `ref.null <heap_type>` -- same heap-type byte -> `ValueType`
            // mapping the function-body checker's own `0xD0` handler uses
            // (see that handler's doc comment for the derivation of every
            // byte value below, including why an out-of-range `0x63`
            // concrete index falls back to `Unknown` rather than erroring).
            0xD0 => {
                let heap_type = *expr
                    .get(offset)
                    .ok_or_else(|| ValidationError::Other("truncated ref.null heap-type immediate in constant expression".to_string()))?;
                offset += 1;
                let result = match heap_type {
                    0x70 => StackType::Known(ValueType::Funcref),
                    0x6F => StackType::Known(ValueType::Externref),
                    0x0F => StackType::Known(ValueType::Anyref),
                    0x73 => StackType::Known(ValueType::NullFuncref),
                    0x72 => StackType::Known(ValueType::NullExternref),
                    0x74 => StackType::Known(ValueType::NullExnref),
                    0x71 => StackType::Known(ValueType::NullRef),
                    0x63 => {
                        let (idx, size) = decode_idx(expr, offset)?;
                        offset += size;
                        if (idx as usize) < ctx.module.types.len() {
                            StackType::Known(ValueType::ConcreteFuncRef(idx))
                        } else {
                            StackType::Unknown
                        }
                    }
                    _ => StackType::Unknown,
                };
                stack.push(result);
            }
            // `ref.func <funcidx>` -- a real, spec-legal constant
            // instruction (function-references proposal). Real result
            // type: `(ref $t)` where `$t` is the referenced function's OWN
            // declared type-section index -- see `ctx.func_type_indices`'s
            // own doc comment (the identical rule the function-body
            // checker's `0xD2` handler already implements). Bounds-checked
            // with `decode_unsigned_bounded(.., 32)`, not the plain
            // `decode_idx` the `0xD0` arm above uses -- security review
            // finding (`wasm-execution::evaluate_const_expr`'s own `0xD2`
            // arm doc comment) established that a raw `u64` LEB128 decode
            // narrowed with `as u32` SILENTLY TRUNCATES a huge index into a
            // small in-range one instead of being rejected; using the
            // bounded decoder here closes the identical class of bug in
            // this new code rather than reintroducing it.
            0xD2 => {
                let (idx, consumed) = decode_unsigned_bounded(expr, offset, 32)
                    .map_err(|e| ValidationError::Other(format!("ref.func: {e}")))?;
                offset += consumed;
                let idx = idx as usize;
                match ctx.func_type_indices.get(idx) {
                    Some(&type_idx) => stack.push(StackType::Known(ValueType::NonNullConcreteFuncRef(type_idx))),
                    None => {
                        return Err(ValidationError::FuncIndexOutOfBounds(format!(
                            "ref.func: constant expression references function index {idx}, but only {} functions exist",
                            ctx.func_type_indices.len()
                        )));
                    }
                }
            }
            // end -- the expression must leave EXACTLY one value on the
            // stack (an empty stack is `(;empty instruction sequence;)`,
            // real spec `assert_invalid "type mismatch"`; more than one
            // remaining value is `global.wast`'s own `(global i32
            // (i32.const 0) (i32.const 0))`/`(global i32 (global.get 0)
            // (global.get 0))` shape, ALSO a real `"type mismatch"` --
            // both verified directly against that file's own corpus
            // cases, not assumed).
            0x0B => {
                let result = pop_const(&mut stack)?;
                if !stack.is_empty() {
                    return Err(ValidationError::Other(
                        "type mismatch: constant expression leaves more than one value on the stack".to_string(),
                    ));
                }
                return Ok(result);
            }
            _ => {
                return Err(ValidationError::Other(format!(
                    "illegal opcode 0x{opcode:02X} in constant expression"
                )));
            }
        }
    }

    Err(ValidationError::Other("constant expression missing end opcode".to_string()))
}

/// Pop one value from a constant expression's abstract type stack, or a
/// real underflow error (an empty/too-short constant expression, e.g.
/// `(global f32 (f32.neg (f32.const 0)))`'s illegal-opcode case never
/// reaches this, but a hand-crafted or fuzzed `init_expr` -- untrusted
/// module bytecode -- reasonably could).
fn pop_const(stack: &mut Vec<StackType>) -> Result<StackType, ValidationError> {
    stack
        .pop()
        .ok_or_else(|| ValidationError::Other("constant expression: stack underflow".to_string()))
}

/// Require a constant-expression operand to be assignable to `expected`
/// (an `Unknown` actual always matches, mirroring `pop_expect`'s identical
/// dead-code-polymorphism rule for ordinary instructions).
fn check_const_operand(actual: StackType, expected: ValueType, module: TypeContext<'_>) -> Result<(), ValidationError> {
    match actual {
        StackType::Unknown => Ok(()),
        StackType::Known(t) if is_assignable(t, expected, module) => Ok(()),
        StackType::Known(t) => Err(ValidationError::Other(format!(
            "type mismatch: constant expression expected {expected:?}, found {t:?}"
        ))),
    }
}

/// Check a fully-evaluated constant expression's static result (`actual`)
/// against what its context requires (`expected`), reusing the exact same
/// `is_assignable` lattice -- including the W32/W33 non-null/bottom/
/// nominal-subtype rules -- every other check in this crate already uses,
/// per this section's own design goal: a const-expr type-checker that
/// respects real subtyping (`(global (ref $t) (ref.func $f))` needs the
/// real subtype check, not bare equality), not a narrower one-off rule.
fn check_const_expr_result(actual: StackType, expected: ValueType, module: TypeContext<'_>, what: &str) -> Result<(), ValidationError> {
    match actual {
        StackType::Unknown => Ok(()),
        StackType::Known(t) if is_assignable(t, expected, module) => Ok(()),
        StackType::Known(t) => Err(ValidationError::Other(format!("{what}: type mismatch, expected {expected:?}, found {t:?}"))),
    }
}

/// Type-check every module-level constant expression -- global
/// initializers, and active element-/data-segment offset expressions --
/// against their declared/required type (see this section's own module
/// doc comment for the gap this closes).
///
/// Runs AFTER `crate::validate`'s own Check 8/Check 9 (data/element
/// segment memory/table-index bounds): by the time this runs, every
/// ACTIVE segment's `memory_index`/`table_index` is already known
/// in-bounds, so indexing `ctx.memory_is64`/`ctx.table_is64` by them is
/// safe -- still guarded defensively (`.get(..).unwrap_or(false)`) rather
/// than a bare index, since this function's own correctness should not
/// depend on exactly which checks ran before it in `crate::validate`.
///
/// A PASSIVE segment (bulk-memory/bulk-table proposals) has no offset
/// expression at all (`offset_expr` is always empty for one, per
/// `Element`/`DataSegment`'s own doc comments) and is never applied at
/// instantiation time -- skipped here, matching every other check in this
/// crate that special-cases passive segments the same way.
fn check_const_exprs(ctx: &ModuleContext) -> Result<(), ValidationError> {
    let module = ctx.module;
    let imported_global_count = (ctx.global_types.len() - module.globals.len()) as u32;

    for (i, g) in module.globals.iter().enumerate() {
        let abs_idx = imported_global_count + i as u32;
        let result = const_expr_type(&g.init_expr, ctx, Some(abs_idx))?;
        check_const_expr_result(result, g.global_type.value_type, module, &format!("global #{abs_idx} initializer"))?;
    }

    for (i, elem) in module.elements.iter().enumerate() {
        if elem.is_passive {
            continue;
        }
        let is64 = ctx.table_is64.get(elem.table_index as usize).copied().unwrap_or(false);
        let expected = if is64 { ValueType::I64 } else { ValueType::I32 };
        let result = const_expr_type(&elem.offset_expr, ctx, None)?;
        check_const_expr_result(result, expected, module, &format!("element segment #{i} offset"))?;
    }

    for (i, seg) in module.data.iter().enumerate() {
        if seg.is_passive {
            continue;
        }
        let is64 = ctx.memory_is64.get(seg.memory_index as usize).copied().unwrap_or(false);
        let expected = if is64 { ValueType::I64 } else { ValueType::I32 };
        let result = const_expr_type(&seg.offset_expr, ctx, None)?;
        check_const_expr_result(result, expected, module, &format!("data segment #{i} offset"))?;
    }

    Ok(())
}

/// Type-checks every function body in `module`, returning this module's own
/// canonicalized type-group forms (W34 third slice: `code/specs/
/// W34-wasm-gc-canonical-type-equivalence.md`) as a side product on success,
/// so `crate::validate` (the sole caller) can cache them on `ValidatedModule`
/// WITHOUT computing them a second time -- see the inline comment below for
/// why right here, immediately after `check_type_subtyping`, is the correct
/// place to compute them (and the only place this crate computes them at
/// all: every instruction-level check below reaches them via `ModuleContext`/
/// `TypeContext`, never by calling `canonicalize_types` again).
#[allow(clippy::type_complexity)] // Vec<Option<(Rc<CanonicalGroup>, u32)>> mirrors wasm_types::canonicalize_types's own return shape verbatim; a type alias would only hide the connection.
pub(crate) fn type_check_module(module: &WasmModule) -> Result<Vec<Option<(Rc<CanonicalGroup>, u32)>>, ValidationError> {
    // Security review finding (W33 first slice): a cyclic `sub` chain must
    // be rejected before ANYTHING downstream leans on the `sub` graph
    // being well-founded -- see `check_type_subtyping_is_acyclic`'s own
    // doc comment. Hoisted out of `check_type_subtyping` itself (W34 third
    // slice) precisely BECAUSE `canonicalize_types` right below also
    // depends on this exact guarantee, and needs to run BEFORE `check_
    // type_subtyping`'s own struct/array field-covariance checks now (see
    // that function's own doc comment for why: those checks turned out to
    // need real canonical data too, not just the nominal `sub` chain).
    check_type_subtyping_is_acyclic(module)?;
    // W34 third slice: computed exactly ONCE per module, right here, now
    // that acyclicity is confirmed -- `wasm_types::canonicalize_types`'s
    // own doc comment explains why that ordering guarantee matters even
    // though canonicalization itself never recurses. Threaded into EVERY
    // downstream check that needs it: `check_type_subtyping`'s own struct/
    // array field-covariance checks just below, AND every instruction-level
    // check further down via `ModuleContext`/`TypeContext` (see those
    // types' own doc comments) -- computed once here, never recomputed at
    // a per-instruction call site, so `is_assignable` and friends only ever
    // pay `canonical_types_equivalent`'s own comparison cost, never
    // canonicalization's. Security review (W34 third slice): that
    // comparison cost is NOT unconditionally O(1) -- `wasm_types::
    // canonicalize_types`'s own interning makes it O(1) for the common,
    // actually-reachable case (two groups this SAME call produced), via an
    // `Rc::ptr_eq` fast path, but still falls back to a real structural
    // walk (bounded by `CanonicalCost`'s own caps, per group) whenever it
    // doesn't -- see that function's own doc comment and CHANGELOG entry
    // for the real DoS this closes and how. Returned to `crate::validate`
    // so IT doesn't need to compute this a second time for `ValidatedModule`'s
    // own cache.
    let canonical_types = wasm_types::canonicalize_types(module);
    check_type_subtyping(module, &canonical_types)?;
    let ctx = build_module_context(module, &canonical_types)?;
    check_const_exprs(&ctx)?;
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
    Ok(canonical_types)
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
                // Real corpus bug (`binary-leb128.wast`'s "i64_trunc_sat_
                // f64_u with 6 bytes" `assert_malformed` case, W-addendum
                // 2026-09-01 pass): the sub-opcode is a `u32` LEB128 per
                // spec, same as every other index-shaped immediate --
                // `decode_unsigned` (bounded only to the native 64-bit/
                // 10-byte budget) let a 6-byte encoding of the small value
                // `7` through uncaught. `decode_unsigned_bounded(.., 32)`
                // enforces the real `ceil(32/7) = 5`-byte cap (overlong)
                // and the "unused top bits must be zero" out-of-range rule
                // -- exactly `read_u32leb`'s own precedent in
                // `wasm-module-parser`.
                let (sub, size) = decode_unsigned_bounded(code, offset, 32)
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
                        pop_expect(&mut stack, frame!(), input, ctx.module)?;
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
                        //
                        // Real corpus bug (`binary.wast`'s "memory.init
                        // requires a data count section", W-addendum
                        // 2026-09-01 pass): the spec requires a data count
                        // section (§12) whenever `memory.init` appears
                        // ANYWHERE in the code section, independent of
                        // whether the data segment it names actually
                        // exists -- checked FIRST, before the out-of-
                        // bounds checks below, so it fires even for a
                        // `data_idx` that happens to be in-bounds.
                        if ctx.module.missing_data_count_section {
                            err!("memory.init requires a data count section");
                        }
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
                        // W30 (memory64 bulk ops): `dest`'s width depends on
                        // the TARGET memory's own `is64` -- `src`/`len`
                        // (positions within the data segment) always stay
                        // `i32`, since a passive data segment isn't itself
                        // address-typed. Mirrors `table.init`'s identical
                        // "only the target-side index widens" rule (W26)
                        // and is confirmed against the real `bulk64.wast`/
                        // `memory_init64.wast` corpus: `(memory.init 0
                        // (i64.const 0) (i32.const 1) (i32.const 2))` pops
                        // `dest` as `i64` for an `is64` memory 0, but
                        // `src`/`len` stay `i32` even there.
                        let dest_type = if ctx.memory_is64.get(memory as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // length (segment-side, always i32)
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // source (segment-side, always i32)
                        pop_expect(&mut stack, frame!(), dest_type, ctx.module)?; // destination
                    }
                    0x09 => {
                        // `data.drop` (task #95): no stack operands, no
                        // memory requirement at all (a module with zero
                        // memories can still declare and drop a passive
                        // data segment it never gets to `memory.init`
                        // from) -- just the same out-of-bounds data-
                        // segment-index check as `memory.init` above.
                        //
                        // Same "data count section required" gate as
                        // `memory.init` above (`binary.wast`'s "data.drop
                        // requires a data count section" case), checked
                        // first for the same reason.
                        if ctx.module.missing_data_count_section {
                            err!("data.drop requires a data count section");
                        }
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
                        // W30 (memory64 bulk ops): `dest`'s width follows
                        // the DESTINATION memory's own `is64`, `src`'s
                        // follows the SOURCE memory's own `is64` --
                        // independently, mirroring `table.copy`'s identical
                        // mixed-index-width rule (W26 / `table_copy_
                        // mixed.wast`). `len`'s width is `i64` ONLY when
                        // BOTH memories are `is64` -- otherwise `i32`, even
                        // when exactly one side is `is64` -- same "the
                        // smaller of the two index types governs a shared
                        // length/count operand" rule `table.copy`'s own
                        // comment already documents for the combined
                        // memory64/table64 proposal. Confirmed against the
                        // real `memory_copy64.wast` corpus's own
                        // `assert_invalid` cases (an `is64` memory's
                        // `memory.copy` rejects a plain `i32` operand for
                        // dest/src/len as "type mismatch").
                        let dst_is64 = ctx.memory_is64.get(dst_memory as usize).copied().unwrap_or(false);
                        let src_is64 = ctx.memory_is64.get(src_memory as usize).copied().unwrap_or(false);
                        let dst_type = if dst_is64 { ValueType::I64 } else { ValueType::I32 };
                        let src_type = if src_is64 { ValueType::I64 } else { ValueType::I32 };
                        let len_type = if dst_is64 && src_is64 { ValueType::I64 } else { ValueType::I32 };
                        pop_expect(&mut stack, frame!(), len_type, ctx.module)?; // length
                        pop_expect(&mut stack, frame!(), src_type, ctx.module)?; // source
                        pop_expect(&mut stack, frame!(), dst_type, ctx.module)?; // destination
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
                        // W30 (memory64 bulk ops): `dest`/`len` are `i64`
                        // for an `is64` memory (`value`, the fill byte,
                        // stays `i32` regardless -- only its low 8 bits are
                        // ever used, same as the is32 case). Mirrors
                        // `table.fill`'s identical is64-dependent
                        // dest/len-only widening (W26).
                        let is64 = ctx.memory_is64.get(memory as usize).copied().unwrap_or(false);
                        let idx_type = if is64 { ValueType::I64 } else { ValueType::I32 };
                        pop_expect(&mut stack, frame!(), idx_type, ctx.module)?; // length
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // byte value
                        pop_expect(&mut stack, frame!(), idx_type, ctx.module)?; // destination
                    }
                    0x0F => {
                        // `table.grow` (task #98): pops `[init, delta]`
                        // (init: the REFERENCED table's own element type,
                        // delta: i32/i64 per that table's own `is64` --
                        // W26), pushes i32/i64 (old size, or -1 on
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
                        let elem_type = ctx.table_element_types[table_idx as usize];
                        let idx_type = if ctx.table_is64.get(table_idx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                        pop_expect(&mut stack, frame!(), idx_type, ctx.module)?; // delta
                        pop_expect(&mut stack, frame!(), elem_type, ctx.module)?; // init value
                        push_val(&mut stack, idx_type);
                    }
                    0x10 => {
                        // `table.size` (task #98): no stack operands,
                        // pushes the table's size -- i64 for an `is64`
                        // table (W26), i32 otherwise. Only the
                        // index-bounds check applies -- element type is
                        // irrelevant to a size query.
                        let (table_idx, size) = decode_idx(code, offset)?;
                        offset += size;
                        if table_idx >= ctx.table_count {
                            err!("table.size references table index {table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        let idx_type = if ctx.table_is64.get(table_idx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                        push_val(&mut stack, idx_type);
                    }
                    0x11 => {
                        // `table.fill` (task #98): pops `[dest, value,
                        // len]` (dest/len: i32/i64 per that table's own
                        // `is64` -- W26, value: the REFERENCED table's own
                        // element type), no push. Same element-type lookup
                        // as 0x0F above.
                        let (table_idx, size) = decode_idx(code, offset)?;
                        offset += size;
                        if table_idx >= ctx.table_count {
                            err!("table.fill references table index {table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        let elem_type = ctx.table_element_types[table_idx as usize];
                        let idx_type = if ctx.table_is64.get(table_idx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                        pop_expect(&mut stack, frame!(), idx_type, ctx.module)?; // length
                        pop_expect(&mut stack, frame!(), elem_type, ctx.module)?; // value
                        pop_expect(&mut stack, frame!(), idx_type, ctx.module)?; // destination
                    }
                    0x0C => {
                        // `table.init` (task #97): pops `[dest, src, len]`,
                        // no push. Binary immediate order is `elemidx`
                        // THEN `tableidx` (opposite of the text form's
                        // `$table $elem` order -- confirmed against the
                        // real testsuite encoding). Both indices are hard
                        // validation errors on out-of-bounds, same
                        // discipline as `memory.init`'s data_idx check
                        // above (task #95).
                        //
                        // W26 (table64): `dest`'s width depends on the
                        // TARGET table's own `is64` -- `src`/`len`
                        // (positions within the element segment) always
                        // stay `i32`, since a passive element segment
                        // isn't itself address-typed (verified against the
                        // real `table_init64.wast` corpus: an is64
                        // table's `table.init` still takes plain i32
                        // `src`/`len` operands, only `dest` widens).
                        let (elem_idx, elem_size) = decode_idx(code, offset)?;
                        let (table_idx, table_size) = decode_idx(code, offset + elem_size)?;
                        offset += elem_size + table_size;
                        if elem_idx as usize >= ctx.module.elements.len() {
                            err!("table.init references out-of-bounds element segment index {elem_idx}");
                        }
                        if table_idx >= ctx.table_count {
                            err!("table.init references table index {table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        let dest_type = if ctx.table_is64.get(table_idx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // length (segment-side, always i32)
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // source (segment-side, always i32)
                        pop_expect(&mut stack, frame!(), dest_type, ctx.module)?; // destination
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
                        // `table.copy` (task #97): pops `[dest, src, len]`,
                        // no push. Text and binary immediate orders MATCH
                        // here (dst-then-src both times, unlike table.init
                        // above). Both table indices are bounds-checked
                        // independently -- a self-copy (dst == src) is
                        // valid and checked at runtime, not rejected here.
                        //
                        // W26 (table64) / `table_copy_mixed.wast`: `dest`'s
                        // width follows the DESTINATION table's own
                        // `is64`, `src`'s follows the SOURCE table's own
                        // `is64` -- independently, since a mixed
                        // is64/is32 copy is legal (`table_copy_mixed.
                        // wast`'s `test_64to32`/`test_32to64` cases both
                        // validate). `len`'s width is `i64` ONLY when
                        // BOTH tables are `is64` -- otherwise `i32`, even
                        // when exactly one side is `is64` (confirmed
                        // against that same corpus file's real, valid
                        // `test_64to32`/`test_32to64` cases, which both
                        // use a plain `i32` `len` despite one table being
                        // `is64`, and its `bad_size_arg` `assert_invalid`
                        // case, which types `len` to `i64` in that same
                        // mixed scenario and is correctly rejected). Same
                        // "the smaller of the two index types governs a
                        // shared length/count operand" rule the combined
                        // memory64/table64 proposal defines for mixed
                        // memory.copy too.
                        let (dst_table_idx, dst_size) = decode_idx(code, offset)?;
                        let (src_table_idx, src_size) = decode_idx(code, offset + dst_size)?;
                        offset += dst_size + src_size;
                        if dst_table_idx >= ctx.table_count {
                            err!("table.copy references destination table index {dst_table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        if src_table_idx >= ctx.table_count {
                            err!("table.copy references source table index {src_table_idx}, but only {} tables exist", ctx.table_count);
                        }
                        let dst_is64 = ctx.table_is64.get(dst_table_idx as usize).copied().unwrap_or(false);
                        let src_is64 = ctx.table_is64.get(src_table_idx as usize).copied().unwrap_or(false);
                        let dst_type = if dst_is64 { ValueType::I64 } else { ValueType::I32 };
                        let src_type = if src_is64 { ValueType::I64 } else { ValueType::I32 };
                        let len_type = if dst_is64 && src_is64 { ValueType::I64 } else { ValueType::I32 };
                        pop_expect(&mut stack, frame!(), len_type, ctx.module)?; // length
                        pop_expect(&mut stack, frame!(), src_type, ctx.module)?; // source
                        pop_expect(&mut stack, frame!(), dst_type, ctx.module)?; // destination
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
                    0x01 => {
                        // struct.new_default <type_idx> (W33 fourth slice):
                        // pops NOTHING (every field gets its type's zero
                        // value), pushes one structref -- the only
                        // difference from struct.new's stack effect.
                        let (type_idx, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad struct.new_default type index: {e}")))?;
                        offset += size;
                        // Still a real bounds check on `type_idx` (matching
                        // struct.new's own behavior) even though the field
                        // count itself isn't used for a pop loop here.
                        struct_field_count(ctx.module, type_idx as u32)?;
                        stack.push(StackType::Unknown);
                    }
                    0x02 | 0x03 | 0x05 => {
                        // struct.get / struct.get_s / struct.get_u
                        // <type_idx> <field_idx> (W33 fourth slice adds the
                        // two packed-field variants, sharing struct.get's
                        // existing stack-effect shape: pop structref, push
                        // the field's -- possibly sign/zero-extended --
                        // value). See `encode_gc_struct_array_instr`'s own
                        // doc comment (wasm-wast-parser) for why 0x05 is
                        // struct.get_u in THIS repo's numbering, not the
                        // real GC spec's.
                        let (type_idx, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad struct.get type index: {e}")))?;
                        let (field_idx, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad struct.get field index: {e}")))?;
                        offset += sz1 + sz2;
                        struct_field(ctx.module, type_idx as u32, field_idx as u32)?; // real bounds check
                        pop_val(&mut stack, frame!())?;
                        stack.push(StackType::Unknown);
                    }
                    0x04 => {
                        // struct.set <type_idx> <field_idx>: pops the new
                        // value and the structref. W33 fourth slice: also a
                        // real mutability check -- `struct.wast`'s own
                        // "struct.set-immutable" `assert_invalid` case
                        // requires this to be a validation error, not
                        // silently accepted (the field itself is still
                        // real, storage-backed data at runtime; immutability
                        // is purely a static, validator-enforced property,
                        // same division of responsibility as every other
                        // WASM "invalid" rule this crate checks statically).
                        let (type_idx, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad struct.set type index: {e}")))?;
                        let (field_idx, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad struct.set field index: {e}")))?;
                        offset += sz1 + sz2;
                        if !struct_field(ctx.module, type_idx as u32, field_idx as u32)?.mutable {
                            return Err(ValidationError::Other(format!("struct.set: immutable field {field_idx} of type {type_idx}")));
                        }
                        pop_val(&mut stack, frame!())?;
                        pop_val(&mut stack, frame!())?;
                    }
                    0x06 | 0x07 => {
                        // array.new / array.new_default <type_idx> (W33
                        // fourth slice): array.new pops [elem_value, i32
                        // length]; array.new_default pops only [i32
                        // length] (the element gets its type's zero value).
                        // Both push one arrayref.
                        let (type_idx, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad array.new type index: {e}")))?;
                        offset += size;
                        array_element_field(ctx.module, type_idx as u32)?; // real bounds check
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // length
                        if sub == 0x06 {
                            pop_val(&mut stack, frame!())?; // elem value
                        }
                        stack.push(StackType::Unknown);
                    }
                    0x08 => {
                        // array.new_fixed <type_idx> <count> (W33 fourth
                        // slice): pops exactly `count` element values
                        // (declared as a literal immediate, not derived
                        // from the operand stack), pushes one arrayref.
                        // `count` is bounded by `MAX_ARRAY_NEW_FIXED_COUNT`
                        // before the pop loop runs -- a malformed module
                        // could otherwise claim a `u32::MAX` count and
                        // force this validator into a multi-billion-
                        // iteration loop (a real algorithmic DoS even
                        // though each iteration is O(1) and allocates
                        // nothing -- the same class of guard `wasm-module-
                        // parser`'s `MAX_PREALLOC` exists for, just for
                        // iteration count here instead of allocation size).
                        let (type_idx, sz1) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad array.new_fixed type index: {e}")))?;
                        let (count, sz2) = decode_unsigned(code, offset + sz1).map_err(|e| ValidationError::Other(format!("bad array.new_fixed count: {e}")))?;
                        offset += sz1 + sz2;
                        array_element_field(ctx.module, type_idx as u32)?; // real bounds check
                        if count > MAX_ARRAY_NEW_FIXED_COUNT as u64 {
                            return Err(ValidationError::Other(format!(
                                "array.new_fixed count {count} exceeds the maximum of {MAX_ARRAY_NEW_FIXED_COUNT}"
                            )));
                        }
                        for _ in 0..count {
                            pop_val(&mut stack, frame!())?;
                        }
                        stack.push(StackType::Unknown);
                    }
                    0x0B..=0x0D => {
                        // array.get / array.get_s / array.get_u <type_idx>
                        // (W33 fourth slice): pops [arrayref, i32 index],
                        // pushes the element's -- possibly sign/zero-
                        // extended -- value.
                        let (type_idx, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad array.get type index: {e}")))?;
                        offset += size;
                        array_element_field(ctx.module, type_idx as u32)?; // real bounds check
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // index
                        pop_val(&mut stack, frame!())?; // arrayref
                        stack.push(StackType::Unknown);
                    }
                    0x0E => {
                        // array.set <type_idx>: pops [arrayref, i32 index,
                        // value], pushes nothing. W33 fourth slice: also a
                        // real mutability check -- `array.wast`'s own
                        // "array.set-immutable" `assert_invalid` case, the
                        // array-hierarchy mirror of struct.set's own check
                        // just above.
                        let (type_idx, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad array.set type index: {e}")))?;
                        offset += size;
                        if !array_element_field(ctx.module, type_idx as u32)?.mutable {
                            return Err(ValidationError::Other(format!("array.set: immutable array element (type {type_idx})")));
                        }
                        pop_val(&mut stack, frame!())?; // value
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // index
                        pop_val(&mut stack, frame!())?; // arrayref
                    }
                    0x0F => {
                        // array.len (W33 fourth slice): NO type immediate
                        // (see `encode_array_len`'s own doc comment in
                        // wasm-wast-parser) -- pops one arrayref, pushes I32.
                        pop_val(&mut stack, frame!())?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    0x14 | 0x15 => {
                        // ref.test / ref.test null <heap_type>: pops a ref,
                        // pushes an I32 boolean.
                        let (_, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad ref.test heap type: {e}")))?;
                        offset += size;
                        pop_val(&mut stack, frame!())?;
                        push_val(&mut stack, ValueType::I32);
                    }
                    0x16 | 0x17 => {
                        // ref.cast / ref.cast null <heap_type> (W33 second
                        // slice, item 4): pops a ref, pushes a ref back --
                        // real dynamic-type checking happens at runtime
                        // (`wasm-execution`'s handler traps "cast failure"
                        // on a genuine mismatch), so this static pass only
                        // needs to keep the abstract stack's byte layout
                        // and height accurate, same as every other GC op
                        // here. MUST consume the heap-type immediate's LEB
                        // bytes (previously fell into the `_ => {}` no-
                        // immediate default below, which would silently
                        // desync `offset` from every REAL instruction
                        // after it in the same function body -- confirmed
                        // via this slice's own `type-subtyping.wast`
                        // diagnostic trace, not assumed).
                        let (_, size) = decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad ref.cast heap type: {e}")))?;
                        offset += size;
                        pop_val(&mut stack, frame!())?;
                        stack.push(StackType::Unknown);
                    }
                    0x1C => {
                        // ref.i31 (W20; this crate previously called it
                        // i31.new): pops I32, pushes i31ref.
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
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
                // `Unknown`.
                //
                // W11 addendum: a concrete `$t` reference is now ALSO a
                // real static type, not a blanket `Unknown` fallback --
                // `wasm-wast-parser`'s `parse_ref_null_heap_type` emits it
                // as the tag byte `0x63` followed by an unsigned LEB128
                // type-section index (the SAME 2-byte shape `ValueType::
                // ConcreteFuncRef`'s own `.encode()` uses in a value-type
                // position; see that variant's doc comment for why this
                // repo's own `wasm-wast-parser`/`wasm-execution`/
                // `wasm-validator` agree on it without needing to match
                // the real spec's sign-disambiguated binary heap-type
                // encoding). Any OTHER heap-type byte still falls back to
                // `Unknown` -- full subtyping remains outside this
                // validator phase for every other GC reference type.
                //
                // The SAME `0x63` tag byte is also `StructRef`'s own
                // 2-byte encoding, whose index lives in a DIFFERENT,
                // offset space (`types.len() + k` -- see `StructRef`'s own
                // doc comment and `struct_field_count`'s identical
                // convention). This crate's own `wasm-wast-parser` has no
                // text-format struct-type declarations at all (so no
                // TEXT-format `ref.null $StructType` can reach this code
                // today), but a `0x63`-tagged index at or past
                // `ctx.module.types.len()` is exactly what a real one
                // WOULD look like -- so only an index strictly BELOW that
                // bound is treated as a genuine `ConcreteFuncRef` here; an
                // index at or past it falls back to the same permissive
                // `Unknown` every other not-yet-modeled case already gets,
                // rather than being rejected as out-of-bounds (a security
                // review round confirmed hard-erroring there would be a
                // real regression the moment struct-type text declarations
                // exist, not just a hypothetical one).
                let heap_type = *code.get(offset).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: truncated ref.null heap-type immediate")))?;
                offset += 1;
                match heap_type {
                    0x70 => push_val(&mut stack, ValueType::Funcref),
                    0x6F => push_val(&mut stack, ValueType::Externref),
                    0x0F => push_val(&mut stack, ValueType::Anyref),
                    // The four BOTTOM heap types (W32 first slice:
                    // `code/specs/W32-wasm-non-null-concrete-reference-
                    // types.md`; real corpus vendoring pass --
                    // `ref_null.wast`'s own `(ref.null nofunc)`/
                    // `(ref.null noextern)`/`(ref.null noexn)`/
                    // `(ref.null none)`) -- `wasm-wast-parser::module::
                    // parse_ref_null_heap_type` now emits the REAL,
                    // independently-verified GC/function-references
                    // proposal tag bytes for these (matching `ValueType::
                    // Null{Funcref,Externref,Exnref,Ref}::byte_tag()`), so
                    // this handler pushes the genuine bottom-type static
                    // type instead of falling back to `Unknown` -- this is
                    // what lets `is_assignable`'s bottom-type lattice
                    // actually fire on a `ref.null nofunc` result flowing
                    // into a `funcref`-typed slot.
                    0x73 => push_val(&mut stack, ValueType::NullFuncref),
                    0x72 => push_val(&mut stack, ValueType::NullExternref),
                    0x74 => push_val(&mut stack, ValueType::NullExnref),
                    0x71 => push_val(&mut stack, ValueType::NullRef),
                    0x63 => {
                        let (idx, size) = decode_idx(code, offset)?;
                        offset += size;
                        if (idx as usize) < ctx.module.types.len() {
                            push_val(&mut stack, ValueType::ConcreteFuncRef(idx));
                        } else {
                            stack.push(StackType::Unknown);
                        }
                    }
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
                // ref.func <funcidx>: pushes a reference to a function by
                // index. Bounds-checked the same way `call`'s type rule
                // above checks its own funcidx.
                //
                // W32 second slice: the real spec's typing rule is
                // `ref.func $f : [] -> [(ref $t)]` where `$t` is `$f`'s OWN
                // function-type index (WebAssembly/function-references's
                // `Overview.md`, verified directly -- see
                // `ValueType::NonNullConcreteFuncRef`'s own doc comment),
                // NOT the pre-W32-second-slice placeholder of pushing bare
                // `Funcref` for every `ref.func` regardless of which
                // function it names. `NonNullConcreteFuncRef(idx) <:
                // ConcreteFuncRef(idx) <: Funcref` (both direct rules, see
                // `is_assignable`) means every PRE-EXISTING corpus use of
                // `ref.func` where a plain `funcref`-typed slot was
                // expected keeps validating exactly as before -- this is a
                // strictly MORE PRECISE static type, not a behavior change
                // for anything that only ever checked assignability.
                let (callee, size) = decode_idx(code, offset)?;
                offset += size;
                match ctx.func_type_indices.get(callee as usize) {
                    Some(&type_idx) => push_val(&mut stack, ValueType::NonNullConcreteFuncRef(type_idx)),
                    None => {
                        return Err(ValidationError::FuncIndexOutOfBounds(format!(
                            "function #{func_idx}: ref.func references function index {callee}, but only {} functions exist",
                            ctx.func_types.len()
                        )));
                    }
                }
            }

            // ── Control ──────────────────────────────────────────────────────
            0x00 => mark_unreachable(&mut stack, frame_mut!()), // unreachable
            0x01 => {}                                          // nop
            0x02 => {
                // block
                let (params, results, size) = decode_blocktype(ctx.module, code, offset)?;
                offset += size;
                push_ctrl(&mut stack, &mut control_stack, FrameKind::Block, params, results, ctx.module)?;
            }
            0x03 => {
                // loop
                let (params, results, size) = decode_blocktype(ctx.module, code, offset)?;
                offset += size;
                push_ctrl(&mut stack, &mut control_stack, FrameKind::Loop, params, results, ctx.module)?;
            }
            0x04 => {
                // if
                let (params, results, size) = decode_blocktype(ctx.module, code, offset)?;
                offset += size;
                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // condition
                push_ctrl(&mut stack, &mut control_stack, FrameKind::If, params, results, ctx.module)?;
            }
            0x05 => {
                // else
                let closed = pop_ctrl(&mut stack, &mut control_stack, ctx.module)?;
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
                pop_expect_many(&mut stack, frame!(), &tag_type.params, ctx.module)?;
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
                pop_expect(&mut stack, frame!(), ValueType::Exnref, ctx.module)?;
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
                push_ctrl(&mut stack, &mut control_stack, FrameKind::Block, params, results, ctx.module)?;
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
                let closed = pop_ctrl(&mut stack, &mut control_stack, ctx.module)?;
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
                pop_expect_many(&mut stack, frame!(), &types, ctx.module)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x0D => {
                // br_if
                let (depth, size) = decode_idx(code, offset)?;
                offset += size;
                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // condition
                let target = resolve_label_target(control_stack.len(), depth)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: br_if target {depth} out of range")))?;
                let types = label_types(&control_stack[target]).to_vec();
                pop_expect_many(&mut stack, frame!(), &types, ctx.module)?;
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

                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // index
                let default_target = resolve_label_target(control_stack.len(), default_label)
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: br_table default target {default_label} out of range")))?;
                let default_types = label_types(&control_stack[default_target]).to_vec();
                // Every target (each of `labels`, plus the default) must
                // independently accept the SAME original operand value(s)
                // -- WASM's br_table typing rule is a "meet" over all
                // targets, not a left-to-right chain, so source ORDER must
                // not affect validity. This used to check target `i`
                // against whatever `push_vals` had just put back from
                // target `i-1` (narrowing the value's apparent type on
                // each pass), which broke as soon as an EARLIER-checked
                // target needed a WIDER type than a LATER one -- e.g. a
                // concrete `(ref $t)`-typed value is genuinely assignable
                // to both a `(ref null $t)` target AND a generic `(ref
                // null func)` target, but the old chain rejected it
                // whenever the generic target was checked (and thus
                // widened away) before the concrete one. Real corpus
                // regression this closes: `br_table.wast`'s own
                // `meet-funcref-1`/`meet-multi-ref` etc., which deliberately
                // list their targets in every possible order specifically
                // to catch an order-dependent implementation like this.
                //
                // The default target's OWN arity worth of values is
                // popped from the real stack exactly ONCE, into a small
                // `operands` vec (size = arity, not stack depth) -- every
                // OTHER target is then checked against this SAME fixed
                // snapshot via `check_stacktype_assignable` alone, never
                // touching (or cloning) the real stack again. This
                // matters for more than tidiness: a `br_table` can list
                // an attacker-controlled number of targets, and the
                // operand stack can independently be attacker-driven
                // deep -- cloning the WHOLE stack once per target (an
                // earlier version of this fix did exactly that) is
                // O(target_count * stack_depth), quadratic in a single
                // instruction's own two independently-controllable
                // dimensions. Checking a fixed arity-sized snapshot
                // instead keeps this O(target_count * arity), the same
                // asymptotic cost the old (order-DEPENDENT, but not
                // DoS-prone) implementation already had.
                let mut operands = Vec::with_capacity(default_types.len());
                for _ in 0..default_types.len() {
                    operands.push(pop_val(&mut stack, frame!())?);
                }
                operands.reverse(); // popped top-to-bottom; restore left-to-right to match `default_types`' own order
                for &label in &labels {
                    let target = resolve_label_target(control_stack.len(), label)
                        .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: br_table target {label} out of range")))?;
                    let types = label_types(&control_stack[target]).to_vec();
                    if types.len() != default_types.len() {
                        err!("br_table targets have mismatched arities ({} vs default's {})", types.len(), default_types.len());
                    }
                    for (&actual, &expected) in operands.iter().zip(types.iter()) {
                        check_stacktype_assignable(actual, expected, ctx.module)?;
                    }
                }
                for (&actual, &expected) in operands.iter().zip(default_types.iter()) {
                    check_stacktype_assignable(actual, expected, ctx.module)?;
                }
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x0F => {
                // return
                let results = control_stack
                    .first()
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: return with no open block")))?
                    .end_types
                    .clone();
                pop_expect_many(&mut stack, frame!(), &results, ctx.module)?;
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
                pop_expect_many(&mut stack, frame!(), &callee_type.params, ctx.module)?;
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
                // W37 security review finding: `call_indirect`/
                // `return_call_indirect` must target a FUNCREF-family
                // table only -- `wasm-execution`'s own dispatch handler
                // (`0x11`'s doc comment) explicitly relies on this as a
                // safety invariant ("`wasm-validator` already guarantees
                // `call_indirect` only ever targets an actual funcref-typed
                // table"), resolving a table slot's raw `u32` payload via
                // `resolve_function_ref_for_dispatch` -- which treats that
                // `u32` as a FUNCTION INDEX unconditionally. Before this
                // check existed, a table declared with any non-funcref
                // reftype (previously only `externref` was reachable here;
                // W37's own generalization of table-declaration parsing
                // newly makes `eqref`/`anyref`/`i31ref`/`structref`/a
                // concrete struct or array type reachable too) could stash
                // an opaque GC-heap handle in a table slot and then
                // `call_indirect` on it -- if that handle's raw `u32` value
                // happened to coincide with a valid function index, this
                // would dispatch an ARBITRARY function using a value that
                // was never a function reference (a real, if bounded --
                // `resolve_function_ref_for_dispatch` still bounds-checks
                // against `func_types`, so this is a type-confusion/logic
                // bug, not memory-unsafety -- validator-soundness bypass).
                // `table.get`/`table.set`/`table.fill`/`table.copy` already
                // consult `ctx.table_element_types` for the identical
                // reason; this instruction never did.
                let elem_type = ctx.table_element_types[table_idx as usize];
                if !matches!(elem_type, ValueType::Funcref | ValueType::ConcreteFuncRef(_) | ValueType::NonNullConcreteFuncRef(_)) {
                    err!("call_indirect requires a funcref-family table, but table {table_idx} has element type {elem_type:?}");
                }
                let callee_type = ctx
                    .module
                    .types
                    .get(type_idx as usize)
                    .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("function #{func_idx}: call_indirect references type index {type_idx}, but only {} types exist", ctx.module.types.len())))?;
                // W26 (table64): the table-index operand is i64 for an
                // `is64` table, i32 otherwise.
                let idx_type = if ctx.table_is64.get(table_idx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                pop_expect(&mut stack, frame!(), idx_type, ctx.module)?; // table index
                pop_expect_many(&mut stack, frame!(), &callee_type.params, ctx.module)?;
                push_vals(&mut stack, &callee_type.results);
            }
            0x12 => {
                // return_call (WASM16): same immediate as `call`, but
                // nothing runs after a tail call -- the callee's results
                // become the CURRENT FUNCTION's own results directly, so
                // they must be ASSIGNABLE to its declared result types (not
                // merely be pushable for further use; W11 addendum: a
                // nullable ref to a specific concrete function type is a
                // legal stand-in for the callee's declared `funcref` slot,
                // not bare equality -- see `results_assignable`), and
                // everything textually after this is dead code, the same
                // handling `return` (0x0F) already has.
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
                if !results_assignable(&callee_type.results, function_results, ctx.module) {
                    err!("return_call to function #{callee} returning {:?}, but the current function returns {function_results:?}", callee_type.results);
                }
                pop_expect_many(&mut stack, frame!(), &callee_type.params, ctx.module)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x13 => {
                // return_call_indirect (WASM16): same immediates as
                // `call_indirect` (typeidx, tableidx), same tail-call
                // result-type-must-be-assignable + dead-code-after rule
                // as `return_call` above.
                let (type_idx, sz1) = decode_idx(code, offset)?;
                let (table_idx, sz2) = decode_idx(code, offset + sz1)?;
                offset += sz1 + sz2;
                // Task #107: same bounds-check as call_indirect (0x11) above.
                if table_idx >= ctx.table_count {
                    err!("return_call_indirect references table index {table_idx}, but only {} tables exist", ctx.table_count);
                }
                // W37 security review finding: same funcref-family-only
                // requirement as `call_indirect` (0x11) above -- see that
                // arm's own doc comment for the full writeup.
                let elem_type = ctx.table_element_types[table_idx as usize];
                if !matches!(elem_type, ValueType::Funcref | ValueType::ConcreteFuncRef(_) | ValueType::NonNullConcreteFuncRef(_)) {
                    err!("return_call_indirect requires a funcref-family table, but table {table_idx} has element type {elem_type:?}");
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
                if !results_assignable(&callee_type.results, function_results, ctx.module) {
                    err!("return_call_indirect to type #{type_idx} returning {:?}, but the current function returns {function_results:?}", callee_type.results);
                }
                // W26 (table64): same is64-dependent table-index operand
                // width as `call_indirect` (0x11) above.
                let idx_type = if ctx.table_is64.get(table_idx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                pop_expect(&mut stack, frame!(), idx_type, ctx.module)?; // table index
                pop_expect_many(&mut stack, frame!(), &callee_type.params, ctx.module)?;
                mark_unreachable(&mut stack, frame_mut!());
            }
            0x14 => {
                // call_ref $t (function-references proposal, W32 second
                // slice): `[t1* (ref null $t)] -> [t2*]`, traps on null --
                // independently verified against WebAssembly/function-
                // references's own `Overview.md`, NOT restricted to a
                // non-null-only operand the way this repo's own W32 spec
                // document first assumed before this slice checked (see
                // `ValueType::NonNullConcreteFuncRef`'s own doc comment).
                // The ref operand is popped LAST (it's on TOP of the
                // stack, per the type rule's own `t1* (ref null $t)`
                // ordering), same shape `call_indirect`'s own table-index
                // operand already uses.
                let (type_idx, size) = decode_idx(code, offset)?;
                offset += size;
                let callee_type = ctx
                    .module
                    .types
                    .get(type_idx as usize)
                    .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("function #{func_idx}: call_ref references type index {type_idx}, but only {} types exist", ctx.module.types.len())))?;
                pop_expect(&mut stack, frame!(), ValueType::ConcreteFuncRef(type_idx), ctx.module)?; // the ref operand
                pop_expect_many(&mut stack, frame!(), &callee_type.params, ctx.module)?;
                push_vals(&mut stack, &callee_type.results);
            }
            0x15 => {
                // return_call_ref (function-references proposal, W32
                // second slice): same immediate/operand shape as
                // `call_ref` (0x14) above, same tail-call result-type-
                // must-be-assignable + dead-code-after rule `return_call`/
                // `return_call_indirect` already use.
                let (type_idx, size) = decode_idx(code, offset)?;
                offset += size;
                let callee_type = ctx
                    .module
                    .types
                    .get(type_idx as usize)
                    .ok_or_else(|| ValidationError::TypeIndexOutOfBounds(format!("function #{func_idx}: return_call_ref references type index {type_idx}, but only {} types exist", ctx.module.types.len())))?;
                let function_results = &control_stack
                    .first()
                    .ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: return_call_ref with no open block")))?
                    .end_types;
                if !results_assignable(&callee_type.results, function_results, ctx.module) {
                    err!("return_call_ref to type #{type_idx} returning {:?}, but the current function returns {function_results:?}", callee_type.results);
                }
                pop_expect(&mut stack, frame!(), ValueType::ConcreteFuncRef(type_idx), ctx.module)?; // the ref operand
                pop_expect_many(&mut stack, frame!(), &callee_type.params, ctx.module)?;
                mark_unreachable(&mut stack, frame_mut!());
            }

            // ── Parametric ───────────────────────────────────────────────────
            0x1A => {
                pop_val(&mut stack, frame!())?; // drop: any type
            }
            0x1B => {
                // select (untyped -- MVP form, no `(result t)` immediate;
                // the reference-types proposal's TYPED select, opcode
                // `0x1C` with an explicit `vec(valtype)` immediate, is a
                // separate, unrelated capability gap this crate's
                // `wasm-wast-parser` does not yet parse at all -- see
                // `select.wast`'s own real corpus census, which fails at
                // the FIRST module using it).
                //
                // W32 second slice: real corpus regression found (not a
                // pre-existing test this slice broke) -- `select.wast`'s
                // own `type-ref-implicit`/`type-funcref-implicit`/
                // `type-externref-implicit` `assert_invalid` cases require
                // rejecting a REFERENCE-typed operand pair here (the real
                // spec restricts the UNTYPED form to `numtype`/`vectype`
                // operands only -- `(ref $t)`/`funcref`/`externref` must
                // use the explicit `(result t)` form instead, which this
                // crate doesn't support, so the untyped form must reject
                // them outright rather than silently accept). Before this
                // slice, `(ref $t)`/`(param $r (ref $t))` was entirely
                // unparseable, so these three cases passed only via a
                // lucky parse failure, never by this check actually
                // running.
                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // condition
                let t2 = pop_val(&mut stack, frame!())?;
                let t1 = pop_val(&mut stack, frame!())?;
                let result = match (t1, t2) {
                    (StackType::Unknown, StackType::Unknown) => StackType::Unknown,
                    (StackType::Unknown, k @ StackType::Known(_)) | (k @ StackType::Known(_), StackType::Unknown) => k,
                    (StackType::Known(a), StackType::Known(b)) if a == b => {
                        if !is_numeric_or_vector(a) {
                            err!("untyped select operand {a:?} is a reference type -- the explicit '(result t)' form is required for reference-typed select operands");
                        }
                        StackType::Known(a)
                    }
                    (StackType::Known(a), StackType::Known(b)) => err!("select operands have different types ({a:?} vs {b:?})"),
                };
                stack.push(result);
            }
            0x1C => {
                // Typed select (reference-types proposal, W37): an
                // explicit `vec(valtype)` immediate replaces `0x1B`'s
                // "infer the type from the operands" rule -- required
                // whenever the untyped form's restriction to numeric/
                // vector operands (see `0x1B`'s own arm above) doesn't
                // apply, e.g. selecting between two reference values.
                //
                // Real validation rule (`select.wast`'s own `arity-0`/
                // `arity-2` `assert_invalid` cases): the immediate's
                // `vec(valtype)` must have EXACTLY one entry. The binary
                // format's `vec(valtype)` shape can technically encode
                // any count -- `select` still only ever selects and
                // produces ONE value, so 0 or 2+ is a validation error
                // ("invalid result arity", matching the real spec
                // interpreter's own wording), not a parse error --
                // `wasm-wast-parser` deliberately parses every count
                // permissively and leaves this arity check to the
                // validator, the same "parse permissively, validate
                // strictly" split this crate's own blocktype/signature
                // parsing already uses elsewhere.
                let (count, mut size) =
                    decode_unsigned(code, offset).map_err(|e| ValidationError::Other(format!("bad select result-type count: {e}")))?;
                // `count.min(4096)` caps the up-front allocation the same
                // way `br_table`'s own label-vec decode above does --
                // an attacker-controlled LEB128 count can claim billions
                // of entries, but each entry still costs at least one
                // real byte of `code` to decode, so the loop below can
                // never run more iterations than `code` has bytes left,
                // regardless of what `count` claims.
                let mut result_types: Vec<ValueType> = Vec::with_capacity(count.min(4096) as usize);
                for _ in 0..count {
                    let (ty, sz) = decode_valtype(ctx.module, code, offset + size)?;
                    result_types.push(ty);
                    size += sz;
                }
                offset += size;
                if result_types.len() != 1 {
                    err!("select: invalid result arity ({} types, expected exactly 1)", result_types.len());
                }
                let t = result_types[0];
                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // condition
                pop_expect(&mut stack, frame!(), t, ctx.module)?; // 2nd operand
                pop_expect(&mut stack, frame!(), t, ctx.module)?; // 1st operand
                push_val(&mut stack, t);
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
                pop_expect(&mut stack, frame!(), ty, ctx.module)?;
            }
            0x22 => {
                // local.tee
                let (idx, size) = decode_idx(code, offset)?;
                offset += size;
                let ty = *locals.get(idx as usize).ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: local.tee index {idx} out of bounds ({} locals)", locals.len())))?;
                pop_expect(&mut stack, frame!(), ty, ctx.module)?;
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
                pop_expect(&mut stack, frame!(), gt.value_type, ctx.module)?;
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
                let elem_type = ctx.table_element_types[table_idx as usize];
                // W26 (table64): the index operand is i64 for an `is64`
                // table, i32 otherwise.
                let idx_type = if ctx.table_is64.get(table_idx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                pop_expect(&mut stack, frame!(), idx_type, ctx.module)?;
                push_val(&mut stack, elem_type);
            }
            0x26 => {
                // table.set <tableidx> (WASM17, generalized task #96):
                // pops a value of the REFERENCED table's own element type
                // and an index (i64 for an `is64` table, i32 otherwise --
                // W26), no push.
                let (table_idx, size) = decode_idx(code, offset)?;
                offset += size;
                if table_idx >= ctx.table_count {
                    err!("table.set references table index {table_idx}, but only {} tables exist", ctx.table_count);
                }
                let elem_type = ctx.table_element_types[table_idx as usize];
                let idx_type = if ctx.table_is64.get(table_idx as usize).copied().unwrap_or(false) { ValueType::I64 } else { ValueType::I32 };
                pop_expect(&mut stack, frame!(), elem_type, ctx.module)?;
                pop_expect(&mut stack, frame!(), idx_type, ctx.module)?;
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
                // Real corpus bug (`binary-leb128.wast`'s memarg align/
                // offset overlong and out-of-range `assert_malformed`
                // cases, W-addendum 2026-09-01 pass): `align` is a `u32`
                // per spec, unconditionally -- `decode_unsigned` (bounded
                // only to the native 64-bit/10-byte budget) let a 6+-byte
                // or high-bit-set encoding of a small value through
                // uncaught.
                let (raw_align, sz1) = decode_unsigned_bounded(code, offset, 32).map_err(|e| ValidationError::Other(format!("bad memarg align: {e}")))?;
                let raw_align = raw_align as u32;
                let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                let align = raw_align & !MULTI_MEMORY_FLAG;
                // `offset`'s own width is genuinely context-dependent --
                // NOT unconditionally `u32` the way `align` is. This
                // SUPERSEDES the previous W25 comment here, which claimed
                // (citing a LIVE fetch of the current spec page) that
                // `offset` is `u64` "unconditionally": that's the spec's
                // CURRENT (post-memory64-merge) text, but the pinned
                // testsuite commit this repo's `wasm-conformance` corpus
                // is actually graded against
                // (`28864811cf03bdbf880733786148feaba339582d`) disagrees
                // in BOTH directions, confirmed by two real corpus files:
                // - `binary-leb128.wast` asserts a 6-byte offset encoding
                //   of the value `2`, on a plain 32-bit memory, IS
                //   malformed ("integer representation too long") -- only
                //   holds under a 5-byte/32-bit budget.
                // - `binary_leb128_64.wast` asserts a 10-byte offset
                //   encoding of `2^64 - 1`, on a memory64 (`is64`) memory,
                //   is perfectly valid (a plain, non-`assert_malformed`
                //   `module`), and that `2^64` (one bit further) IS
                //   malformed ("integer too large") -- only holds under a
                //   FULL 64-bit budget.
                // So: narrow to 32 bits only when we can already prove
                // (without having decoded `offset` yet) that a 32-bit
                // budget is correct -- i.e. no explicit multi-memory
                // `memidx` follows (so the target is implicitly memory 0)
                // AND memory 0 isn't `is64`. The multi-memory case can't
                // be resolved this way at all: `memidx` isn't decoded
                // until AFTER `offset` in this format (see below), so
                // which memory `offset` even addresses is still unknown
                // here -- fall back to the full native 64-bit budget
                // (`decode_unsigned_bounded(.., 64)` is byte-for-byte
                // `decode_unsigned`) rather than guess, exactly preserving
                // this crate's previous, un-regressed behavior for that
                // combination (no corpus file currently exercises a
                // multi-memory memarg with an offset value that a 32-bit
                // budget would have rejected anyway, so this is a pure
                // no-op there, not a knowingly-loose carve-out).
                let implicit_memory_0_is64 = !has_memidx && ctx.memory_is64.first().copied().unwrap_or(false);
                let offset_bits: u32 = if has_memidx || implicit_memory_0_is64 { 64 } else { 32 };
                let (_mem_offset, sz2) = decode_unsigned_bounded(code, offset + sz1, offset_bits).map_err(|e| ValidationError::Other(format!("bad memarg offset: {e}")))?;
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
                    pop_expect(&mut stack, frame!(), addr_type, ctx.module)?; // address
                    push_val(&mut stack, value_type);
                } else {
                    pop_expect(&mut stack, frame!(), value_type, ctx.module)?; // stored value (top)
                    pop_expect(&mut stack, frame!(), addr_type, ctx.module)?; // address
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
                pop_expect(&mut stack, frame!(), grow_type, ctx.module)?;
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
                    // Same align/offset width fix as the plain-memory
                    // `0x28..=0x3E` memarg read above -- `align` is always
                    // `u32`-bounded; `offset` widens to 64 bits only for
                    // an `is64` memory 0 (atomics have no multi-memory
                    // `memidx` immediate at all, so memory 0 is always the
                    // target -- no ambiguous case to fall back from here).
                    let (align, sz1) = decode_unsigned_bounded(code, offset, 32).map_err(|e| ValidationError::Other(format!("bad atomic memarg align: {e}")))?;
                    let offset_bits: u32 = if ctx.memory_is64.first().copied().unwrap_or(false) { 64 } else { 32 };
                    let (_mem_offset, sz2) = decode_unsigned_bounded(code, offset + sz1, offset_bits).map_err(|e| ValidationError::Other(format!("bad atomic memarg offset: {e}")))?;
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
                            pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // address
                            push_val(&mut stack, value_type);
                        }
                        wasm_opcodes::AtomicOpKind::Store => {
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), value_type, ctx.module)?; // value
                            pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // address
                        }
                        wasm_opcodes::AtomicOpKind::Rmw => {
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), value_type, ctx.module)?; // operand
                            pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // address
                            push_val(&mut stack, value_type); // old value
                        }
                        wasm_opcodes::AtomicOpKind::Cmpxchg => {
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), value_type, ctx.module)?; // replacement
                            pop_expect(&mut stack, frame!(), value_type, ctx.module)?; // expected
                            pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // address
                            push_val(&mut stack, value_type); // old value
                        }
                        wasm_opcodes::AtomicOpKind::Notify => {
                            // memory.atomic.notify: pop (addr: i32, count:
                            // i32), push i32 (how many woken -- always 0
                            // with one native thread, see AtomicOpKind::
                            // Notify's own doc comment).
                            pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // count
                            pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // address
                            push_val(&mut stack, ValueType::I32);
                        }
                        wasm_opcodes::AtomicOpKind::Wait => {
                            // memory.atomic.wait32/wait64: pop (addr:
                            // i32, expected: value_type, timeout: i64),
                            // push i32 (result code).
                            let value_type = atomic_op.value_type.ok_or_else(|| ValidationError::Other(format!("function #{func_idx}: atomic op {} has no value type", atomic_op.name)))?;
                            pop_expect(&mut stack, frame!(), ValueType::I64, ctx.module)?; // timeout
                            pop_expect(&mut stack, frame!(), value_type, ctx.module)?; // expected
                            pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?; // address
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
                // Same u32-bounded fix as the `0xFC` sub-opcode read above
                // -- identical shape (a LEB128 `u32` sub-opcode), same
                // overlong/out-of-range gap when left on the unbounded
                // native decoder.
                let (sub, size) = decode_unsigned_bounded(code, offset, 32)
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::Splat
                    | wasm_opcodes::SimdOpKind::SplatI8x16
                    | wasm_opcodes::SimdOpKind::SplatI16x8 => {
                        // i8x16.splat/i16x8.splat (SIMD widen PR16): same
                        // "pop I32, push V128" shape as i32x4.splat --
                        // only the low bits of the popped i32 matter at
                        // runtime, invisible to the type checker.
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::SplatI64x2 => {
                        // i64x2.splat (SIMD widen PR16): the FIRST splat
                        // that pops I64 instead of I32.
                        pop_expect(&mut stack, frame!(), ValueType::I64, ctx.module)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::SplatF32x4 => {
                        // f32x4.splat (SIMD widen PR17): the FIRST
                        // floating-point-typed SIMD op in this crate's
                        // type rules -- pop F32, push V128.
                        pop_expect(&mut stack, frame!(), ValueType::F32, ctx.module)?;
                        push_val(&mut stack, ValueType::V128);
                    }
                    wasm_opcodes::SimdOpKind::SplatF64x2 => {
                        // f64x2.splat (SIMD widen PR17): pop F64, push
                        // V128. Same shape as SplatF32x4.
                        pop_expect(&mut stack, frame!(), ValueType::F64, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::I64, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::F32, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::F64, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                        pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
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
                        // Same u32-bounded fix (W-addendum 2026-09-01
                        // pass) as the plain-memory `0x28..=0x3E` memarg
                        // read above, applied here and at every other
                        // `v128`-prefixed memarg site below -- identical
                        // field shapes, same overlong/out-of-range gap on
                        // the unbounded native decoder.
                        let (raw_align, sz1) = decode_unsigned_bounded(code, offset, 32).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        // `offset` width: same context-dependent rule as
                        // the plain-memory `0x28..=0x3E` arm -- 32 bits
                        // only when memory 0 (the only target v128.load/
                        // v128.store currently allows -- see the explicit
                        // non-zero-`memidx` rejection below) is known,
                        // right now, not to be `is64`; the ambiguous
                        // explicit-`memidx` case (rejected below anyway
                        // once decoded, but not yet known at this point)
                        // falls back to the full native 64-bit budget
                        // rather than guess.
                        let implicit_memory_0_is64 = !has_memidx && ctx.memory_is64.first().copied().unwrap_or(false);
                        let offset_bits: u32 = if has_memidx || implicit_memory_0_is64 { 64 } else { 32 };
                        let (_mem_offset, sz2) = decode_unsigned_bounded(code, offset + sz1, offset_bits).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
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
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store => {
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
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
                        let (raw_align, sz1) = decode_unsigned_bounded(code, offset, 32).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        // `offset` width: same context-dependent rule as
                        // the plain-memory `0x28..=0x3E` arm -- 32 bits
                        // only when memory 0 (the only target v128.load/
                        // v128.store currently allows -- see the explicit
                        // non-zero-`memidx` rejection below) is known,
                        // right now, not to be `is64`; the ambiguous
                        // explicit-`memidx` case (rejected below anyway
                        // once decoded, but not yet known at this point)
                        // falls back to the full native 64-bit budget
                        // rather than guess.
                        let implicit_memory_0_is64 = !has_memidx && ctx.memory_is64.first().copied().unwrap_or(false);
                        let offset_bits: u32 = if has_memidx || implicit_memory_0_is64 { 64 } else { 32 };
                        let (_mem_offset, sz2) = decode_unsigned_bounded(code, offset + sz1, offset_bits).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
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
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store8Lane => {
                                // pop the v128 to read the lane from, pop
                                // the i32 address, no result -- same
                                // pop-order and no-push shape as `Store`
                                // above.
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
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
                        let (raw_align, sz1) = decode_unsigned_bounded(code, offset, 32).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        // `offset` width: same context-dependent rule as
                        // the plain-memory `0x28..=0x3E` arm -- 32 bits
                        // only when memory 0 (the only target v128.load/
                        // v128.store currently allows -- see the explicit
                        // non-zero-`memidx` rejection below) is known,
                        // right now, not to be `is64`; the ambiguous
                        // explicit-`memidx` case (rejected below anyway
                        // once decoded, but not yet known at this point)
                        // falls back to the full native 64-bit budget
                        // rather than guess.
                        let implicit_memory_0_is64 = !has_memidx && ctx.memory_is64.first().copied().unwrap_or(false);
                        let offset_bits: u32 = if has_memidx || implicit_memory_0_is64 { 64 } else { 32 };
                        let (_mem_offset, sz2) = decode_unsigned_bounded(code, offset + sz1, offset_bits).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
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
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store16Lane => {
                                // pop the v128 to read the lane from, pop
                                // the i32 address, no result -- same
                                // pop-order and no-push shape as
                                // `Store8Lane` above.
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
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
                        let (raw_align, sz1) = decode_unsigned_bounded(code, offset, 32).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        // `offset` width: same context-dependent rule as
                        // the plain-memory `0x28..=0x3E` arm -- 32 bits
                        // only when memory 0 (the only target v128.load/
                        // v128.store currently allows -- see the explicit
                        // non-zero-`memidx` rejection below) is known,
                        // right now, not to be `is64`; the ambiguous
                        // explicit-`memidx` case (rejected below anyway
                        // once decoded, but not yet known at this point)
                        // falls back to the full native 64-bit budget
                        // rather than guess.
                        let implicit_memory_0_is64 = !has_memidx && ctx.memory_is64.first().copied().unwrap_or(false);
                        let offset_bits: u32 = if has_memidx || implicit_memory_0_is64 { 64 } else { 32 };
                        let (_mem_offset, sz2) = decode_unsigned_bounded(code, offset + sz1, offset_bits).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
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
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store32Lane => {
                                // pop the v128 to read the lane from, pop
                                // the i32 address, no result -- same
                                // pop-order and no-push shape as
                                // `Store16Lane` above.
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
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
                        let (raw_align, sz1) = decode_unsigned_bounded(code, offset, 32).map_err(|e| ValidationError::Other(format!("bad v128 memarg align: {e}")))?;
                        let raw_align = raw_align as u32;
                        let has_memidx = raw_align & MULTI_MEMORY_FLAG != 0;
                        // `offset` width: same context-dependent rule as
                        // the plain-memory `0x28..=0x3E` arm -- 32 bits
                        // only when memory 0 (the only target v128.load/
                        // v128.store currently allows -- see the explicit
                        // non-zero-`memidx` rejection below) is known,
                        // right now, not to be `is64`; the ambiguous
                        // explicit-`memidx` case (rejected below anyway
                        // once decoded, but not yet known at this point)
                        // falls back to the full native 64-bit budget
                        // rather than guess.
                        let implicit_memory_0_is64 = !has_memidx && ctx.memory_is64.first().copied().unwrap_or(false);
                        let offset_bits: u32 = if has_memidx || implicit_memory_0_is64 { 64 } else { 32 };
                        let (_mem_offset, sz2) = decode_unsigned_bounded(code, offset + sz1, offset_bits).map_err(|e| ValidationError::Other(format!("bad v128 memarg offset: {e}")))?;
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
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
                                push_val(&mut stack, ValueType::V128);
                            }
                            wasm_opcodes::SimdOpKind::Store64Lane => {
                                // pop the v128 to read the lane from, pop
                                // the i32 address, no result -- same
                                // pop-order and no-push shape as
                                // `Store32Lane` above.
                                pop_expect(&mut stack, frame!(), ValueType::V128, ctx.module)?;
                                pop_expect(&mut stack, frame!(), ValueType::I32, ctx.module)?;
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
                pop_expect(&mut stack, frame!(), input, ctx.module)?;
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
                    type_check_numeric(&mut stack, frame!(), info.name, info.stack_pop, ctx.module)?;
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
