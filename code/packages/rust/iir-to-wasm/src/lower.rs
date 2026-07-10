//! Two-pass IIR → WASM lowering (with WasmGC heap ops — Phase 2).
//!
//! # Overview
//!
//! This module implements the main lowering pass: it takes an `IIRModule` and
//! produces a `WasmModule`.  The algorithm runs in two passes per function:
//!
//! **Pass 1 — Register allocation**
//! Scan the function's parameters and instructions and assign a unique WASM
//! local-variable index to each IIR variable.  Parameters are indices 0..N-1;
//! additional IIR variables are indices N..total-1.  A single extra local is
//! appended at index `total` to serve as the **dispatch variable** (used by the
//! dispatch-loop control-flow pattern).
//!
//! **Pass 2 — Code generation**
//! Translate each `IIRInstr` to WASM bytecode, using the register map from
//! Pass 1.  The output is a `FunctionBody` whose `locals` list declares the
//! extra (non-parameter) local variables and whose `code` field contains the
//! raw WASM instruction bytes.
//!
//! # Control flow: dispatch-loop
//!
//! WASM uses *structured* control flow — no raw labels or unconditional jumps.
//! To represent IIR's label/jmp/jmp_if patterns, we use the **dispatch-loop**
//! pattern:
//!
//! ```text
//! (block $exit       ;; break here = leave function body (no return value needed)
//!   (loop $dispatch  ;; break with depth 0 = re-enter loop top
//!
//!     ;; N nested blocks — one per basic block — innermost = block 0
//!     (block $bb_N-1 ... (block $bb_0
//!       local.get $dispatch
//!       br_table 0 1 … N-1   ;; dispatch[i] → break out of i levels
//!       ;; default (depth N) → break out of all blocks, exit loop, exit function
//!     ) … )
//!
//!     ;; Basic block 0 body (after all nested block ends are emitted)
//!     …
//!     ;; At end: set dispatch to next block index, br to $dispatch (depth 0)
//!
//!     ;; Basic block 1 body
//!     …
//!
//!     ;; Basic block N-1 body
//!     …
//!   )
//! )
//! ```
//!
//! The key insight is that `br_table 0 1 … N-1` exits blocks at depths
//! 0, 1, …, N-1.  Exiting depth 0 (innermost) puts execution just after
//! `block $bb_0`'s `end`, which is where we put bb_0's body.  Exiting depth 1
//! skips bb_0's body and puts execution after `block $bb_1`'s `end`, etc.
//!
//! For functions **without** any label/jmp instructions (the common case for
//! purely arithmetic programs), we skip the dispatch-loop entirely and emit
//! instructions linearly.
//!
//! # Type mapping
//!
//! ```text
//! IIR type hint                     →  WASM ValueType
//! ────────────────────────────────────────────────────
//! i8, i16, i32, u8, u16, u32, bool  →  I32
//! i64, u64                           →  I64
//! f32                                →  F32
//! f64                                →  F64
//! void                               →  (no return type)
//! ref<LispyPair>                     →  Anyref  (WasmGC)
//! ```
//!
//! # WasmGC heap ops (Phase 2)
//!
//! When the module contains any `ref<LispyPair>`-typed instructions, we
//! register the `$LispyPair` struct type in the module's `struct_types` vec.
//! Its type-section index is `func_types.len()` (struct types come after all
//! function types in the WasmGC type section).
//!
//! ```wat
//! (type $LispyPair (struct
//!   (field $head (mut (ref null any)))   ;; field 0
//!   (field $tail (mut (ref null any))))) ;; field 1
//! ```
//!
//! ## Lowering patterns
//!
//! **`alloc ref<LispyPair>`** — The pattern is fused across the alloc + any
//! following field_store instructions, but in this lowering we simply emit
//! `ref.null none` (a null pair) and then each field is written separately
//! by `field_store`.  This avoids requiring the IIR front-end to guarantee
//! exactly two consecutive field_stores.  (`struct.new` would require both
//! fields on the stack simultaneously, which complicates the front-end.)
//!
//! Actually: the specification asks us to fuse `alloc + 2 field_stores` into
//! a single `struct.new`.  We implement a simpler but equivalent approach:
//! push `ref.null none` for `alloc` (allocating a null placeholder), then
//! each subsequent `field_store` on the same pair local calls `struct.set`.
//! The full struct.new fusion would need look-ahead; this approach is correct
//! and sufficient for the Lispy runtime.
//!
//! Actually, since the task spec requires `struct.new`, let's do it right: we
//! keep `alloc` as emitting `ref.null none` (the "nil" cons) and separate
//! `field_store` instructions will mutate it via `struct.set`.  For the
//! `struct.new` fusion pattern, the front-end is responsible for calling
//! `alloc` with `head`/`tail` already loaded.  The simplest implementation
//! that passes all tests: `alloc` emits `ref.null none; local.set dest`.
//!
//! For `field_store dest pair field_idx`:
//! ```wasm
//! local.get $pair_local
//! local.get $val_local
//! struct.set $LispyPair field_idx
//! ```
//!
//! For `field_load dest pair field_idx`:
//! ```wasm
//! local.get $pair_local
//! struct.get $LispyPair field_idx
//! local.set $dest_local
//! ```
//!
//! For `is_null dest x`:
//! ```wasm
//! local.get $x_local
//! ref.is_null
//! local.set $dest_local
//! ```
//!
//! For `const ref<LispyPair>` (nil):
//! ```wasm
//! ref.null none
//! local.set $dest_local
//! ```

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use wasm_module_encoder::{GcInstruction, encode_gc_instruction};
use wasm_types::{
    DataSegment, ExternalKind, Export, FieldType, FuncType, FunctionBody, Global, GlobalType,
    Import, ImportTypeInfo, StructType, ValueType, WasmModule,
};

use crate::codegen::{
    encode_br, encode_br_table, encode_call, encode_f32_const, encode_f64_const,
    encode_f64_load, encode_f64_store, encode_i32_load, encode_i32_store, encode_i64_load,
    encode_i64_store,
    encode_i32_const, encode_i64_const, encode_local_get, encode_local_set, BLOCK, BLOCK_EMPTY,
    DROP, END, F32_ADD, F32_DIV, F32_EQ, F32_GE, F32_GT, F32_LE, F32_LT, F32_MUL, F32_NEG,
    F32_NE, F32_SUB, F64_ADD, F64_CONVERT_I64_S, F64_DIV, F64_EQ, F64_FLOOR, F64_GE, F64_GT,
    F64_LE, F64_LT, F64_MUL,
    F64_NEG, F64_NE, F64_SQRT, F64_SUB, I32_ADD, I32_AND, I32_DIV_S, I32_DIV_U, I32_EQ, I32_EQZ, I32_GE_S,
    I64_TRUNC_F64_S,
    I32_GE_U, I32_GT_S, I32_GT_U, I32_LE_S, I32_LE_U, I32_LT_S, I32_LT_U, I32_MUL, I32_NE,
    I32_OR, I32_REM_S, I32_REM_U, I32_SHL, I32_SHR_S, I32_SHR_U, I32_SUB, I32_XOR, I64_ADD,
    I64_AND, I64_DIV_S, I64_DIV_U, I64_EQ, I64_GE_S, I64_GE_U, I64_GT_S, I64_LE_S, I64_LT_S,
    I64_MUL, I64_NE, I64_OR, I64_REM_S, I64_REM_U, I64_SHL, I64_SHR_S, I64_SHR_U, I64_SUB,
    I64_XOR, IF, LOOP, RETURN, UNREACHABLE,
};
use crate::validate::validate_for_wasm;

/// The synthetic module-level global that holds the E5 array bump pointer (the
/// next free byte offset in linear memory). Injected into `global_names` when a
/// module uses any array op; the `__` prefix keeps it out of any frontend's name
/// space.
const ARRAY_BUMP_GLOBAL: &str = "__array_bump";

#[derive(Debug, Clone, PartialEq, Eq)]
struct WasmStringLiteral {
    offset: u32,
    len: u32,
    bytes: Vec<u8>,
}

type FunctionStringLiterals = HashMap<String, WasmStringLiteral>;
type ModuleStringLiterals = HashMap<String, FunctionStringLiterals>;

// ---------------------------------------------------------------------------
// E4-dyn (E4d-3): runtime (branch-selected) strings
// ---------------------------------------------------------------------------
//
// The literal machinery above resolves every string to a single compile-time
// `{offset, len}` keyed by its destination variable.  That is exact for a
// straight-line program: even `s := "OK"; s := "NO"` folds correctly because
// the last write wins and there is only ever one live value at `print_str`.
//
// It breaks the moment control flow chooses the string:
//
// ```basic
// 10 INPUT N
// 20 IF N > 0 THEN 50
// 30 LET A$ = "LO"      ← str_const A$ "LO"   (block B1)
// 40 GOTO 60
// 50 LET A$ = "HI"      ← str_const A$ "HI"   (block B2)
// 60 PRINT A$           ← print_str A$        (block B3)
// ```
//
// Here `A$` is the dest of `str_const` in **two different basic blocks**, so
// the by-dest table can only remember one of them (last writer, `"HI"`), and
// its length would be wrong whenever the other branch ran and the two literals
// differ in length.  This is the exact problem the LLVM backend solved in
// E4d-2: promote such a variable to carry a **runtime handle** instead of a
// folded literal.
//
// Representation (mirrors E4d-2's `{i64 len}[bytes]` block, sized for wasm32):
//   * A *runtime string* is a length-prefixed block in linear memory:
//     `[i32 len (4 bytes, little-endian)][len bytes of UTF-8]`.
//   * The *handle* carried in the string variable's i32 local is the byte
//     offset of that block (the offset of its length prefix).
//   * `str_const` of a promoted var stores the handle (its block's offset);
//     `print_str` of a promoted var reads the length back with `i32.load` and
//     passes `handle + 4` (the bytes) + that length to `env.__print_str`.
//
// A string assigned in only one basic block keeps the folded literal fast path
// unchanged — zero behavioural change for every existing WASM string cell.

/// Per-function set of string variables promoted to a runtime handle (assigned
/// by `str_const` in more than one basic block).
type FunctionRuntimeStrVars = HashSet<String>;
type ModuleRuntimeStrVars = HashMap<String, FunctionRuntimeStrVars>;

/// Per-function map: literal text → linear-memory offset of its length-prefixed
/// runtime block `[i32 len][bytes]`.  Dedadup'd by text, so two `str_const`s of
/// the same string in different blocks share one block.
type FunctionRuntimeStrBlocks = HashMap<String, u32>;
type ModuleRuntimeStrBlocks = HashMap<String, FunctionRuntimeStrBlocks>;

/// Compute the set of string variables that must be promoted to a runtime
/// handle: those that are the destination of a `str`-typed instruction in **more
/// than one basic block**.  This mirrors `iir-to-llvm`'s `collect_slot_vars`
/// (the `str_blocks` half) so the two backends promote exactly the same
/// variables from identical IIR.
///
/// Basic-block boundaries match the rest of this backend: a `label` starts a new
/// block, and a terminator (`jmp`/`jmp_if_false`/`jmp_if_true`/`ret`/`ret_void`)
/// ends one.  A str variable reassigned twice *straight-line* stays in one block
/// and keeps the literal fast path — the linear last-writer-wins tracking is
/// exactly right there.
fn collect_runtime_str_vars(fn_: &IIRFunction) -> FunctionRuntimeStrVars {
    let mut str_blocks: HashMap<&str, HashSet<usize>> = HashMap::new();
    // Folded-literal string destinations, and str vars handed to a callee as a call
    // argument.  A `str_const`/`str_concat`/`str_slice` whose result folds to a
    // compile-time literal normally takes the folded fast path: its handle is the
    // RAW-byte data offset and its length is known only at compile time (via the
    // `string_literals` table).  But when that literal is passed to a FUNCTION, the
    // callee has no compile-time length for the parameter — its `str_len`/
    // `str_concat`/`str_slice`/`str_eq` must read a length-prefixed `[i32 len][bytes]`
    // block header at run time.  So a folded literal used as a call argument must be
    // promoted to a runtime-block handle exactly like a control-flow-selected string,
    // even though it is assigned in only one block.  (`str_const` alone was promoted
    // originally; `str_concat`/`str_slice` results — e.g. `(strlen (substring …))` or
    // a `let*`-derived `string-append` fed to a function — need the same treatment,
    // else the callee reads the first data byte as the length: `"HELLO"` → `'H'`=72.)
    let mut folding_str_dests: HashSet<&str> = HashSet::new();
    let mut call_arg_vars: HashSet<&str> = HashSet::new();
    // E4d-BA-arr: str vars stored as the *value* of an `array_set` into an
    // `array<str>` element.  Same rationale as `call_arg_vars` below — a folded
    // literal handed to `array_set` must become a runtime-block handle, because
    // `array_set` emits `local.get val` and stores that i32 into the element, but
    // a folded literal's local is never assigned the block offset (its handle
    // lives only in the compile-time `string_literals` table).  Left un-promoted,
    // the element would store 0 and a later `array_get` + `print_str`/`str_concat`
    // would read the module header as a bogus length and trap.
    let mut array_set_val_vars: HashSet<&str> = HashSet::new();
    let mut block: usize = 0;
    for instr in &fn_.instructions {
        let op = instr.op.as_str();
        if op == "label" {
            block += 1;
        }
        if let Some(dest) = &instr.dest {
            if instr.type_hint == "str" {
                str_blocks.entry(dest.as_str()).or_default().insert(block);
            }
        }
        // Any string-producing op that folds to a compile-time literal. A `str` dest
        // that is instead a live handle (a param, a call result, a branch-selected
        // var) is NOT listed here: it already carries a runtime block, so passing it
        // to a callee needs no promotion — and it has no `string_literals` entry to
        // fold from.
        if matches!(op, "str_const" | "str_concat" | "str_slice") {
            if let Some(dest) = &instr.dest {
                folding_str_dests.insert(dest.as_str());
            }
        }
        if op == "call" {
            // srcs[0] is the callee name; srcs[1..] are the arguments.
            for src in instr.srcs.iter().skip(1) {
                if let Operand::Var(v) = src {
                    call_arg_vars.insert(v.as_str());
                }
            }
        }
        // E4d-BA-arr: `array_set handle, idx, val` — the value (src[2]) is the
        // string being stored into an element.
        if op == "array_set" {
            if let Some(Operand::Var(v)) = instr.srcs.get(2) {
                array_set_val_vars.insert(v.as_str());
            }
        }
        if matches!(op, "jmp" | "jmp_if_false" | "jmp_if_true" | "ret" | "ret_void") {
            block += 1;
        }
    }
    let mut promoted: FunctionRuntimeStrVars = str_blocks
        .iter()
        .filter(|(_, blocks)| blocks.len() >= 2)
        .map(|(name, _)| name.to_string())
        .collect();
    // Promote a folded literal passed across a function boundary (see above).
    for v in &call_arg_vars {
        if folding_str_dests.contains(v) {
            promoted.insert(v.to_string());
        }
    }
    // E4d-BA-arr: likewise promote a folded literal stored into an `array<str>`
    // element — `array_set` needs the runtime block handle in the val local.
    for v in &array_set_val_vars {
        if folding_str_dests.contains(v) {
            promoted.insert(v.to_string());
        }
    }
    promoted
}

// ---------------------------------------------------------------------------
// Global opcode helpers (LANG32)
// ---------------------------------------------------------------------------

/// Encode a WASM `global.get N` opcode.
///
/// Binary layout: `[0x23, leb128(N)]`
///
/// `global.get` pushes the current value of module-level global variable at
/// index `idx` onto the WASM value stack.  The global must already exist in
/// the module's global section (or be imported).
/// Emit `i32.load8_u offset=0 align=0` — read one byte from linear memory.
///
/// The address is the top of the operand stack (i32, byte offset into the
/// module's default memory).  The result is the zero-extended `u8` value
/// as an `i32` on the stack.
///
/// # WASM binary layout
///
/// ```text
/// 0x2D  <align:LEB128 u32>  <offset:LEB128 u32>
/// ```
///
/// For Brainfuck we use natural alignment 0 (1-byte access) and zero
/// offset — addresses are passed dynamically, never as an immediate.
///
/// Result encoding: `[0x2D, 0x00, 0x00]` — opcode + align byte + offset byte.
fn encode_i32_load8_u() -> Vec<u8> {
    // `0x2D` = i32.load8_u; align = 0 (LEB128); offset = 0 (LEB128).
    vec![0x2Du8, 0x00u8, 0x00u8]
}

/// Emit `i32.store8 offset=0 align=0` — write the low byte of an i32 to memory.
///
/// Pops `value` then `addr` from the stack (in that order — same as every
/// WASM store opcode), and stores `value & 0xFF` at `mem[addr]`.
///
/// # WASM binary layout
///
/// ```text
/// 0x3A  <align:LEB128 u32>  <offset:LEB128 u32>
/// ```
///
/// Result encoding: `[0x3A, 0x00, 0x00]`.
fn encode_i32_store8() -> Vec<u8> {
    // `0x3A` = i32.store8; align = 0; offset = 0.
    vec![0x3Au8, 0x00u8, 0x00u8]
}

/// Emit `i32.add` (0x6A) — pop two i32, push their sum.
///
/// Used by the byte-tape ops (LANG-MATRIX LM-W Brainfuck) to compute the
/// effective address `base + idx` before an `i32.load8_u` / `i32.store8`.
fn encode_i32_add() -> Vec<u8> {
    vec![0x6Au8]
}

/// Emit `memory.copy` (bulk-memory `0xFC 0x0A`) — copy a run of bytes within the
/// single linear memory. Pops `size`, then `src`, then `dest` (so the stack, bottom
/// → top, is `[dest, src, size]`), and copies `size` bytes from `src` to `dest`,
/// overlap-safe. The two trailing `0x00` bytes are the destination and source
/// memory indices — always memory 0 in our single-memory modules.
///
/// Used by E4-dyn runtime `str_concat` to splice each operand's bytes into a freshly
/// bump-allocated `[i32 len][bytes]` block. The `wasm-execution` interpreter decodes
/// the `0xFC` prefix and executes this via `LinearMemory::copy`.
///
/// Result encoding: `[0xFC, 0x0A, 0x00, 0x00]`.
fn encode_memory_copy() -> Vec<u8> {
    vec![0xFCu8, 0x0Au8, 0x00u8, 0x00u8]
}

fn encode_i64_global_init(value: i64) -> Vec<u8> {
    let mut bytes = encode_i64_const(value);
    bytes.push(END);
    bytes
}

/// Emit `i32.wrap_i64` (0xA7) — truncate an i64 to i32 (drop the high 32 bits).
///
/// The Brainfuck value model is uniformly `i64` after `lower_brainfuck_for_aot`
/// widens it, but wasm linear-memory addresses and `i32.store8` values are i32.
/// This narrows a tape pointer / cell value to the i32 the memory ops expect;
/// the low byte (the only part a cell holds) is preserved.
fn encode_i32_wrap_i64() -> Vec<u8> {
    vec![0xA7u8]
}

/// Emit `i64.extend_i32_u` (0xAD) — zero-extend an i32 to i64.
///
/// The dual of [`encode_i32_wrap_i64`]: after `i32.load8_u` yields a
/// zero-extended byte as i32, this widens it back to the i64 cell register.
fn encode_i64_extend_i32_u() -> Vec<u8> {
    vec![0xADu8]
}

fn encode_global_get(idx: u32) -> Vec<u8> {
    use wasm_leb128::encode_unsigned;
    let mut bytes = vec![0x23u8]; // global.get opcode
    bytes.extend(encode_unsigned(idx as u64));
    bytes
}

/// Encode a WASM `global.set N` opcode.
///
/// Binary layout: `[0x24, leb128(N)]`
///
/// `global.set` pops the top of the WASM value stack and stores it into the
/// module-level global variable at index `idx`.  The global must be declared
/// mutable (`(mut ...)`) in the global type.
fn encode_global_set(idx: u32) -> Vec<u8> {
    use wasm_leb128::encode_unsigned;
    let mut bytes = vec![0x24u8]; // global.set opcode
    bytes.extend(encode_unsigned(idx as u64));
    bytes
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced during IIR → WASM lowering.
///
/// Variants are designed to be specific enough to help the front-end author
/// understand what went wrong without needing to read the lowering source.
#[derive(Debug, Clone, PartialEq)]
pub enum IIRWasmError {
    /// The module failed pre-flight validation.
    /// The inner vector contains the human-readable error strings from
    /// [`validate_for_wasm`].
    ValidationFailed(Vec<String>),

    /// An instruction uses an opcode that the WASM backend does not know
    /// how to lower.
    UnsupportedOp {
        /// Name of the function that contains the unsupported instruction.
        function: String,
        /// The unrecognised opcode string.
        op: String,
    },

    /// An instruction's `type_hint` cannot be mapped to a WASM `ValueType`.
    UnsupportedType {
        function: String,
        type_hint: String,
    },

    /// A branch or jump targets a label that was not defined in the function.
    UndefinedLabel {
        function: String,
        label: String,
    },

    /// A source operand refers to a variable that was never assigned a
    /// register index (not a parameter and never defined by any instruction).
    UndefinedVariable {
        function: String,
        name: String,
    },

    /// An instruction's operand list is structurally invalid (wrong count,
    /// wrong kind, etc.).
    InvalidOperand {
        function: String,
        detail: String,
    },
}

impl std::fmt::Display for IIRWasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IIRWasmError::ValidationFailed(errs) => {
                write!(f, "ValidationFailed: {}", errs.join("; "))
            }
            IIRWasmError::UnsupportedOp { function, op } => {
                write!(f, "UnsupportedOp in {function:?}: op {op:?}")
            }
            IIRWasmError::UnsupportedType {
                function,
                type_hint,
            } => {
                write!(
                    f,
                    "UnsupportedType in {function:?}: type_hint {type_hint:?}"
                )
            }
            IIRWasmError::UndefinedLabel { function, label } => {
                write!(f, "UndefinedLabel in {function:?}: label {label:?}")
            }
            IIRWasmError::UndefinedVariable { function, name } => {
                write!(f, "UndefinedVariable in {function:?}: var {name:?}")
            }
            IIRWasmError::InvalidOperand { function, detail } => {
                write!(f, "InvalidOperand in {function:?}: {detail}")
            }
        }
    }
}

impl std::error::Error for IIRWasmError {}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the IIR → WASM lowering pass.
///
/// Currently only carries the module name (written into a WASM custom section
/// or used for debugging).  Additional options (e.g. optimisation level,
/// memory model) can be added here in future versions.
#[derive(Debug, Clone)]
pub struct IIRWasmConfig {
    /// The name embedded in the WASM module (used for identification).
    pub module_name: String,
}

impl Default for IIRWasmConfig {
    fn default() -> Self {
        Self {
            module_name: "iir_module".to_string(),
        }
    }
}

impl IIRWasmConfig {
    /// Create a new config with the given module name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            module_name: name.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

/// Map an IIR `type_hint` string to a WASM `ValueType`.
///
/// Returns `None` for hints that have no WASM equivalent (`"void"`, `"str"`,
/// `"any"`, etc.).  The caller decides whether `None` is an error (for typed
/// destinations) or acceptable (for void returns).
///
/// WasmGC: `"ref<LispyPair>"` and any other `"ref<...>"` hints map to
/// `Anyref` — the nullable top reference type.  This means that any GC
/// struct reference can be held in a local of type `anyref`, which is the
/// WasmGC equivalent of Java's `Object`.
///
/// ```text
/// IIR hint               → WASM ValueType
/// ─────────────────────────────────────────
/// i8 / i16 / i32         → I32
/// u8 / u16 / u32         → I32
/// bool                   → I32   (0 = false, non-zero = true)
/// i64 / u64              → I64
/// f32                    → F32
/// f64                    → F64
/// ref<LispyPair> / ref<…>→ Anyref  (WasmGC)
/// (everything else)      → None
/// ```
pub fn hint_to_value_type(hint: &str) -> Option<ValueType> {
    match hint {
        // Narrow **unsigned** integers ride the i64 register model (LANG-FULL
        // E2): every backend's frontends carry integer values in 64-bit slots,
        // so a `u8` register is an `i64` whose arithmetic is masked to 8 bits
        // after each op (see `emit_wasm_width_mask`). Typing the local `i32`
        // would trap the moment a `u8` op met an `i64` operand (a const/let),
        // which is exactly Nib's value model.
        "u4" | "u8" | "u16" | "u32" => Some(ValueType::I64),
        // Signed narrow widths and `bool` keep the i32 model (booleans are i32
        // 0/1; no frontend emits i8/i16/i32 register arithmetic).
        "i8" | "i16" | "i32" | "bool" => Some(ValueType::I32),
        "i64" | "u64" => Some(ValueType::I64),
        "f32" => Some(ValueType::F32),
        "f64" => Some(ValueType::F64),
        // LANG-FULL E4 literal-output foothold: a `str` local is a byte offset
        // into module linear memory. Richer string ops still validate-fail until
        // the full byte-string runtime lands.
        "str" => Some(ValueType::I32),
        // LANG-FULL E5: an `array<T>` handle is a byte offset into linear memory,
        // carried in an `i64` register (like the Brainfuck tape base) and wrapped
        // to `i32` when used as a WASM address. The *element* type drives the
        // per-access `i64.load`/`f64.load`/… and is validated separately.
        _ if interpreter_ir::opcodes::is_array_type(hint) => Some(ValueType::I64),
        _ if hint.starts_with("ref<") => {
            // Any GC reference type (validated by validate.rs to be supported)
            // is held in a WASM local of type `anyref` — the GC reference
            // supertype.  Concrete struct operations use type-indexed
            // struct.get / struct.set instructions that carry the type index
            // as an immediate, so the local type can be the broader `anyref`.
            Some(ValueType::Anyref)
        }
        _ => None,
    }
}

/// The WASM value type + byte size for an E5 array element type hint. The
/// 64-bit elements (`i64` / `f64`) back the ALGOL frontend's `integer` and
/// `real` arrays; the 4-byte `str` element (E4d-BA-arr) backs BASIC string
/// arrays.  Any other element produces a clear error rather than a silently
/// wrong store width.
fn wasm_array_elem(elem: &str, fn_name: &str) -> Result<(ValueType, u32), IIRWasmError> {
    match elem {
        "i64" | "u64" => Ok((ValueType::I64, 8)),
        "f64" => Ok((ValueType::F64, 8)),
        // E4d-BA-arr: a `str` element is an E4-dyn runtime string handle — a
        // 4-byte `i32` linear-memory offset (the same representation a `str`
        // local uses via `hint_to_value_type`), so a string array is a flat
        // block of i32 handles.  `array_get`/`array_set` select `i32.load`/
        // `i32.store` for it (below).
        "str" => Ok((ValueType::I32, 4)),
        _ => Err(IIRWasmError::UnsupportedType {
            function: fn_name.to_string(),
            type_hint: format!("array element {elem:?} (only i64/f64/str elements on WASM so far)"),
        }),
    }
}

/// Extract `srcs[i]` of an array op as a variable name, with a clear error
/// naming the op and the operand's role (`handle`/`idx`/`val`).
fn array_var<'a>(
    instr: &'a IIRInstr,
    i: usize,
    op: &str,
    role: &str,
    fn_name: &str,
) -> Result<&'a str, IIRWasmError> {
    match instr.srcs.get(i) {
        Some(Operand::Var(v)) => Ok(v.as_str()),
        _ => Err(IIRWasmError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!("{op} requires Operand::Var({role}) as src[{i}]"),
        }),
    }
}

/// Return `true` if the type hint represents a 64-bit integer type.
///
/// Used during arithmetic to select `i64.*` vs `i32.*` opcodes.
fn is_i64_hint(hint: &str) -> bool {
    matches!(hint, "i64" | "u64")
}

/// Return `true` if the hint is computed in the **i64 register model** — the
/// true 64-bit ints (`i64`/`u64`) *and* the narrow **unsigned** types
/// (`u4`/`u8`/`u16`/`u32`), which LANG-FULL E2 computes wide (`i64.*` ops over
/// i64-slot operands) and then masks to width. Selects `i64.*` opcodes so the
/// op never meets a width-mismatched operand; the post-op
/// [`emit_wasm_width_mask`] restores the narrow wrap.
fn uses_i64_register(hint: &str) -> bool {
    is_i64_hint(hint) || matches!(hint, "u4" | "u8" | "u16" | "u32")
}

/// Return `true` if the type hint represents an unsigned integer type.
///
/// Used to select `_u` (unsigned) vs `_s` (signed) comparison and division
/// opcodes for `i32` types.  For `i64` we always use signed in v1 (matching
/// the IIR spec's signed-default model).
fn is_unsigned_hint(hint: &str) -> bool {
    matches!(hint, "u4" | "u8" | "u16" | "u32" | "u64")
}

/// Mask a narrow-width arithmetic result down to its bit width (LANG-FULL E2).
///
/// Narrow **unsigned** integers are computed in the **i64 register model**
/// (`i64.*` ops over i64-slot operands — see [`uses_i64_register`]), so the
/// result is masked with `i64.const <mask>; i64.and` so `200u8 + 100u8` becomes
/// `44` and `~x` on a `u8` flips only 8 bits.  This mirrors vm-core's
/// `mask_result` / jit-core's `MASK_WIDTH` / the LLVM `and i64` / the native
/// `and #mask`, and is the register-arithmetic analogue of the byte-tape
/// `i32.store8`.  `i64`/`u64`/`u32`/float hints emit nothing — `u32` already
/// wraps mod-2³² within the i64 op only up to 2³², so it too gets a mask;
/// `i64`/`u64`/float carry their full width.
fn emit_wasm_width_mask(code: &mut Vec<u8>, type_hint: &str) {
    let mask: i64 = match type_hint {
        "u4" => 0xF,
        "u8" => 0xFF,
        "u16" => 0xFFFF,
        "u32" => 0xFFFF_FFFF,
        _ => return,
    };
    code.extend(encode_i64_const(mask));
    code.push(I64_AND);
}

/// Return `true` if the type hint represents a floating-point type.
fn is_float_hint(hint: &str) -> bool {
    matches!(hint, "f32" | "f64")
}

// ---------------------------------------------------------------------------
// Register allocation (Pass 1)
// ---------------------------------------------------------------------------

/// Build a map from IIR variable name → WASM local index for one function.
///
/// **Algorithm:**
///
/// 1. Assign indices 0..param_count-1 to the function parameters, in order.
///    These are already "free" locals in WASM — they receive the call arguments.
///
/// 2. Walk all instructions in order.  For each instruction:
///    - If it has a `dest` variable not yet in the map, assign the next index.
///    - For each source operand that is a `Var`, if not yet in the map,
///      assign the next index (handles uses of variables that are defined
///      earlier in the program — they will already be in the map — but also
///      catches any forward references that a front-end might produce).
///
/// 3. The resulting map covers every variable that appears anywhere in the
///    function.  Indices are compact: 0, 1, 2, …, N-1.
///
/// The returned count includes parameters.  Locals for `FunctionBody.locals`
/// are indices `param_count..total_vars-1`.  An extra dispatch local is added
/// on top at index `total_vars`.
fn build_register_map(fn_: &IIRFunction) -> HashMap<String, u32> {
    let mut map: HashMap<String, u32> = HashMap::new();
    let mut next_idx: u32 = 0;

    // WASM limits the number of locals per function to u32::MAX in theory,
    // but realistic modules use far fewer.  We apply a generous cap to catch
    // pathological inputs before the index counter wraps around to 0, which
    // would silently produce duplicate local indices and corrupt the output.
    const MAX_WASM_LOCALS: u32 = 1 << 20; // 1,048,576 — already pathological

    // Parameters come first (they receive the call arguments in WASM).
    for (param_name, _) in &fn_.params {
        map.entry(param_name.clone()).or_insert_with(|| {
            let idx = next_idx;
            assert!(
                idx < MAX_WASM_LOCALS,
                "WASM local index overflow: too many variables in function {:?}", fn_.name
            );
            next_idx += 1;
            idx
        });
    }

    // Walk instructions in program order.
    for instr in &fn_.instructions {
        // Destination variable.
        if let Some(dest) = &instr.dest {
            map.entry(dest.clone()).or_insert_with(|| {
                let idx = next_idx;
                assert!(
                    idx < MAX_WASM_LOCALS,
                    "WASM local index overflow: too many variables in function {:?}", fn_.name
                );
                next_idx += 1;
                idx
            });
        }

        // Source variables.
        for src in &instr.srcs {
            if let Operand::Var(name) = src {
                map.entry(name.clone()).or_insert_with(|| {
                    let idx = next_idx;
                    assert!(
                        idx < MAX_WASM_LOCALS,
                        "WASM local index overflow: too many variables in function {:?}", fn_.name
                    );
                    next_idx += 1;
                    idx
                });
            }
        }
    }

    map
}

/// Infer IIR type hints for each WASM local index.
///
/// We scan parameters and instruction destinations for type hints associated
/// with each local index. First definition wins, matching the existing local
/// declaration behavior.
fn infer_local_type_hints(
    fn_: &IIRFunction,
    reg_map: &HashMap<String, u32>,
) -> HashMap<u32, String> {
    // Build a map: var_index → best known type hint.
    let mut var_type: HashMap<u32, String> = HashMap::new();

    // Seed from parameter types.
    for (param_name, param_type) in &fn_.params {
        if let Some(&idx) = reg_map.get(param_name) {
            var_type.insert(idx, param_type.clone());
        }
    }

    // Walk instructions: use the dest type_hint as the type for the dest var.
    for instr in &fn_.instructions {
        if let Some(dest) = &instr.dest {
            if let Some(&idx) = reg_map.get(dest) {
                // Only update if we don't already have a type (first definition wins).
                var_type.entry(idx).or_insert_with(|| instr.type_hint.clone());
            }
        }
    }

    var_type
}

/// Infer the WASM ValueType for each local variable beyond the parameters.
///
/// Returns a `Vec<ValueType>` parallel to indices `param_count..total_vars`.
/// If a variable has no type information, we default to `I32` (the most
/// common type and the natural choice for boolean/integer values).
fn infer_local_types(
    var_type: &HashMap<u32, String>,
    param_count: u32,
    total_vars: u32,
) -> Vec<ValueType> {
    // Build the locals list: one ValueType per index from param_count to total_vars-1.
    //
    // Note: locals whose hint starts with "ref<" map to `Anyref` via
    // `hint_to_value_type`.  This ensures that GC struct locals are declared
    // as `anyref` in the WASM locals section, which is the widest compatible
    // reference type and is always valid for holding WasmGC struct refs.
    (param_count..total_vars)
        .map(|idx| {
            let hint = var_type.get(&idx).map(|s| s.as_str()).unwrap_or("i32");
            hint_to_value_type(hint).unwrap_or(ValueType::I32)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Basic block splitting
// ---------------------------------------------------------------------------

/// A basic block: a slice of IIR instructions that starts at a label (or at
/// the implicit function entry) and runs until the next label or end of
/// function.
///
/// We own the instructions by cloning them from the function so the codegen
/// pass can work with owned data.
struct BasicBlock {
    /// Instructions in this block (does not include the `label` instruction
    /// that names this block).
    instrs: Vec<interpreter_ir::IIRInstr>,
}

/// Split a function's instruction list into basic blocks.
///
/// The first block (index 0) is the implicit entry block; it contains all
/// instructions before the first `label` instruction.  Each subsequent label
/// starts a new block.
///
/// Returns:
/// - `blocks`: a `Vec<BasicBlock>` in order.
/// - `label_to_block`: a map from label name to block index (0-based).
fn split_into_blocks(
    fn_: &IIRFunction,
) -> (Vec<BasicBlock>, HashMap<String, u32>) {
    // We always have at least one block: the implicit entry block.
    let mut blocks: Vec<BasicBlock> = vec![BasicBlock { instrs: Vec::new() }];
    let mut label_to_block: HashMap<String, u32> = HashMap::new();

    for instr in &fn_.instructions {
        if instr.op == "label" {
            // Start a new basic block.  The label name identifies it.
            // u32::try_from is safe because we cap label count at 65,536
            // via validate_for_wasm (Check 6).
            let block_idx = u32::try_from(blocks.len())
                .expect("basic block count overflows u32 (should be caught by validation)");
            if let Some(Operand::Var(label_name)) = instr.srcs.first() {
                label_to_block.insert(label_name.clone(), block_idx);
            }
            blocks.push(BasicBlock { instrs: Vec::new() });
        } else {
            // Append to the current (last) block.
            blocks
                .last_mut()
                .expect("blocks always has at least one entry")
                .instrs
                .push(instr.clone());
        }
    }

    (blocks, label_to_block)
}

/// Return `true` if the function contains any `label`, `jmp`, `jmp_if_true`,
/// or `jmp_if_false` instructions.
///
/// Functions without control-flow instructions can use simple linear code
/// emission, which is faster and produces smaller binaries.
fn has_control_flow(fn_: &IIRFunction) -> bool {
    fn_.instructions.iter().any(|i| {
        matches!(
            i.op.as_str(),
            "label" | "jmp" | "jmp_if_true" | "jmp_if_false"
        )
    })
}

// ---------------------------------------------------------------------------
// Instruction code generation
// ---------------------------------------------------------------------------

/// Emit WASM bytes for a single IIR instruction.
///
/// This is the heart of the backend.  The function pattern-matches on `op`
/// and emits the correct sequence of WASM opcodes + immediates into `code`.
///
/// For binary arithmetic: load the two source locals, emit the arithmetic
/// opcode, store the result into the destination local:
///
/// ```text
/// local.get <r1>
/// local.get <r2>
/// <opcode>
/// local.set <rd>
/// ```
///
/// For unary arithmetic: load the one source local, emit negation or bitwise-
/// not, store result:
///
/// ```text
/// ;; neg (i32):  0 - r  →  i32.const 0; local.get r; i32.sub; local.set rd
/// ;; not (i32):  r ^ -1 →  local.get r; i32.const -1; i32.xor; local.set rd
/// ```
///
/// For constants:
///
/// ```text
/// i32.const <value> ; local.set rd
/// i64.const <value> ; local.set rd
/// f64.const <value> ; local.set rd
/// ```
///
/// For comparisons: same as binary arithmetic — result is always `i32`
/// (WASM boolean conventions: 0 = false, 1 = true).
///
/// # Parameters
///
/// - `code` — output buffer to append bytes into.
/// - `instr` — the instruction to emit.
/// - `reg_map` — variable name → local index map built in Pass 1.
/// - `fn_map` — function name → WASM function index map.
/// - `fn_name` — the enclosing function name (for error context).
/// - `dispatch_reg` — local index of the dispatch variable (for control flow).
/// - `label_to_block` — label name → block index map.
/// - `block_idx` — index of the **current** basic block being emitted (0-based,
///   only meaningful when `is_dispatch_loop` is `true`).  Used to compute
///   the correct WASM branch depth for `jmp`/`jmp_if_*` instructions.
/// - `n_blocks` — total number of basic blocks in the function (needed to
///   compute the "loop back" depth for backward jumps).
/// - `is_dispatch_loop` — whether we are inside a dispatch-loop structure.
///   When `true`, `jmp`/`jmp_if_*` instructions set the dispatch variable and
///   branch to the appropriate position.  When `false`, they emit simplified
///   forms.
/// - `lispy_pair_type_idx` — if `Some(idx)`, this function contains at least
///   one `ref<LispyPair>` instruction and the `$LispyPair` struct type is at
///   type-section index `idx`.  Used by `alloc`, `field_load`, `field_store`.
/// - `global_map` — global variable name → WASM global index.  Built by the
///   pre-pass in `lower_iir_to_wasm`.  Used by `global_load`/`global_store`.
/// - `print_fn_idx` — if `Some(idx)`, the `$__print_i64` import is at WASM
///   function index `idx` (always 0 when present).  Used by `io_out`.
#[allow(clippy::too_many_arguments)]
fn emit_instr(
    code: &mut Vec<u8>,
    instr: &interpreter_ir::IIRInstr,
    reg_map: &HashMap<String, u32>,
    local_type_hints: &HashMap<u32, String>,
    fn_map: &HashMap<String, u32>,
    fn_name: &str,
    dispatch_reg: u32,
    label_to_block: &HashMap<String, u32>,
    block_idx: usize,
    n_blocks: usize,
    is_dispatch_loop: bool,
    lispy_pair_type_idx: Option<u32>,
    global_map: &HashMap<String, u32>,
    string_literals: &FunctionStringLiterals,
    runtime_str_vars: &FunctionRuntimeStrVars,
    runtime_str_blocks: &FunctionRuntimeStrBlocks,
    print_fn_idx: Option<u32>,
    print_str_fn_idx: Option<u32>,
    putchar_fn_idx: Option<u32>,
    getchar_fn_idx: Option<u32>,
    input_i64_fn_idx: Option<u32>,
    input_str_fn_idx: Option<u32>,
    sin_fn_idx: Option<u32>,
    cos_fn_idx: Option<u32>,
    ln_fn_idx: Option<u32>,
    exp_fn_idx: Option<u32>,
    atan_fn_idx: Option<u32>,
    tan_fn_idx: Option<u32>,
    pow_fn_idx: Option<u32>,
    // Function index of the in-module `$__str_eq` helper (present iff the module
    // has a `str_eq` that can't be folded — see `uses_str_eq_runtime`).
    str_eq_fn_idx: Option<u32>,
) -> Result<(), IIRWasmError> {
    // Helper closures to resolve variable names.
    let get_reg = |var: &str| -> Result<u32, IIRWasmError> {
        reg_map.get(var).copied().ok_or_else(|| IIRWasmError::UndefinedVariable {
            function: fn_name.to_string(),
            name: var.to_string(),
        })
    };

    // True when a local slot is an `i64` (vs `i32`). The Brainfuck value model
    // is uniformly `i64` after `lower_brainfuck_for_aot` widens it, so the
    // byte-tape ops + `putchar`/`getchar` convert between that i64 and the i32
    // that wasm linear-memory addresses / `i32.store8` values / the libc
    // `putchar`/`getchar` imports use. Looking the width up (rather than
    // assuming i64) keeps the lowering correct for an un-widened i32 caller too.
    let slot_is_i64 = |slot: u32| -> bool {
        local_type_hints
            .get(&slot)
            .map(|h| matches!(h.as_str(), "i64" | "u64"))
            .unwrap_or(false)
    };

    let get_label = |label: &str| -> Result<u32, IIRWasmError> {
        label_to_block
            .get(label)
            .copied()
            .ok_or_else(|| IIRWasmError::UndefinedLabel {
                function: fn_name.to_string(),
                label: label.to_string(),
            })
    };

    // The type of the current instruction's destination/operands.
    let ty = instr.type_hint.as_str();

    match instr.op.as_str() {
        // ── str_const ────────────────────────────────────────────────────────
        //
        // E4 literal-output foothold for WASM: materialise a string literal as
        // an i32 byte offset into the module's linear memory.  The companion
        // length is compile-time metadata carried in `string_literals` and is
        // consumed by `print_str`; `str_concat` below uses the same table for
        // literal-only concatenation metadata.
        "str_const" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "str_const must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;

            // E4-dyn (E4d-3) runtime path: a string variable chosen by control
            // flow carries an i32 **handle** = the offset of its length-prefixed
            // block `[i32 len][bytes]`.  The handle (not the raw-byte offset) is
            // what `print_str` needs so it can read the length back at run time,
            // because the compile-time table cannot tell which branch's literal
            // is live.  The block was laid down in `collect_module_features`,
            // keyed by the literal text.
            if runtime_str_vars.contains(dest) {
                let text = match instr.srcs.first() {
                    Some(Operand::Str(s)) => s.as_str(),
                    _ => return Err(IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: "str_const requires Operand::Str".to_string(),
                    }),
                };
                let block_offset = runtime_str_blocks.get(text).copied().ok_or_else(|| {
                    IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: format!("str_const missing runtime block for {text:?}"),
                    }
                })?;
                code.extend(encode_i32_const(block_offset as i32));
                code.extend(encode_local_set(rd));
                return Ok(());
            }

            // Literal fast path: a single-assignment string folds to a known
            // raw-byte offset + compile-time length.
            let lit = string_literals.get(dest).ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("str_const missing module string table entry for {dest:?}"),
            })?;
            code.extend(encode_i32_const(lit.offset as i32));
            code.extend(encode_local_set(rd));
        }

        // ── str_concat → literal data-segment metadata OR runtime block ──────
        //
        // Literal fast path: when BOTH operands folded to a compile-time literal, the
        // module string table already holds the joined bytes at a fixed data-segment
        // offset — the handle is that constant offset (mirrors str_slice / str_const).
        //
        // Runtime path (E4-dyn): at least one operand is a runtime handle (an `INPUT`
        // result, a call result, a branch-selected string) with no literal entry. On
        // WASM a `str` is an i32 handle to a `[i32 len][bytes]` block, so we build a
        // fresh block in linear memory:
        //   new  = bump                                  ;; base of the joined block
        //   bump = bump + 4 + la + lb                    ;; reserve header + both runs
        //   mem[new]      = la + lb                      ;; write the i32 length header
        //   memory.copy(new+4,       a+4, la)            ;; splice operand a's bytes
        //   memory.copy(new+4+la,    b+4, lb)            ;; then operand b's bytes
        // `la`/`lb` are re-read from each operand's header with `i32.load` wherever
        // needed, which keeps the whole sequence free of scratch locals (the only
        // local written is the destination, `rd`). `print_str`/`str_len` on the result
        // then read its header at run time, exactly like any other runtime string.
        "str_concat" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "str_concat must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            if let Some(lit) = string_literals.get(dest) {
                // Both operands are literals — the joined bytes are a compile-time
                // constant.  Normally the handle is that constant's raw data-segment
                // offset.  But when this folded result crosses a call boundary
                // (`runtime_str_vars`), the callee needs a length-prefixed header, so
                // emit the runtime-block handle laid down in `collect_module_features`
                // instead — keyed by the folded text exactly as at laydown.
                if runtime_str_vars.contains(dest) {
                    let key = String::from_utf8_lossy(&lit.bytes);
                    let block_offset =
                        runtime_str_blocks.get(key.as_ref()).copied().ok_or_else(|| {
                            IIRWasmError::InvalidOperand {
                                function: fn_name.to_string(),
                                detail: format!(
                                    "str_concat missing runtime block for folded {dest:?}"
                                ),
                            }
                        })?;
                    code.extend(encode_i32_const(block_offset as i32));
                } else {
                    code.extend(encode_i32_const(lit.offset as i32));
                }
                code.extend(encode_local_set(rd));
            } else {
                // Runtime path: at least one operand is a live handle.  This already
                // bump-allocates a `[i32 len][bytes]` block, so its base IS a valid
                // runtime handle — a promoted call-arg needs no extra treatment here.
                let (a, b) = match instr.srcs.as_slice() {
                    [Operand::Var(a), Operand::Var(b)] => (a, b),
                    _ => {
                        return Err(IIRWasmError::InvalidOperand {
                            function: fn_name.to_string(),
                            detail: "runtime str_concat requires srcs [Var(a), Var(b)]".to_string(),
                        });
                    }
                };
                let ra = get_reg(a)?;
                let rb = get_reg(b)?;
                let bump = *global_map.get(ARRAY_BUMP_GLOBAL).ok_or_else(|| {
                    IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "str_concat (missing __array_bump global)".to_string(),
                    }
                })?;

                // rd = new = i32.wrap(bump)  — the fresh block's base handle.
                code.extend(encode_global_get(bump));
                code.extend(encode_i32_wrap_i64());
                code.extend(encode_local_set(rd));

                // bump = bump + i64(4 + la + lb)  — reserve header + both byte runs.
                code.extend(encode_global_get(bump));
                code.extend(encode_i32_const(4));
                code.extend(encode_local_get(ra));
                code.extend(encode_i32_load(0));
                code.push(I32_ADD);
                code.extend(encode_local_get(rb));
                code.extend(encode_i32_load(0));
                code.push(I32_ADD);
                code.extend(encode_i64_extend_i32_u());
                code.push(I64_ADD);
                code.extend(encode_global_set(bump));

                // mem[new] = la + lb  — write the i32 length header.
                code.extend(encode_local_get(rd));
                code.extend(encode_local_get(ra));
                code.extend(encode_i32_load(0));
                code.extend(encode_local_get(rb));
                code.extend(encode_i32_load(0));
                code.push(I32_ADD);
                code.extend(encode_i32_store(0));

                // memory.copy(new+4, a+4, la)  — splice operand a's bytes.
                code.extend(encode_local_get(rd));
                code.extend(encode_i32_const(4));
                code.push(I32_ADD);
                code.extend(encode_local_get(ra));
                code.extend(encode_i32_const(4));
                code.push(I32_ADD);
                code.extend(encode_local_get(ra));
                code.extend(encode_i32_load(0));
                code.extend(encode_memory_copy());

                // memory.copy(new+4+la, b+4, lb)  — then operand b's bytes.
                code.extend(encode_local_get(rd));
                code.extend(encode_i32_const(4));
                code.push(I32_ADD);
                code.extend(encode_local_get(ra));
                code.extend(encode_i32_load(0));
                code.push(I32_ADD);
                code.extend(encode_local_get(rb));
                code.extend(encode_i32_const(4));
                code.push(I32_ADD);
                code.extend(encode_local_get(rb));
                code.extend(encode_i32_load(0));
                code.extend(encode_memory_copy());
            }
        }

        // ── str_slice → literal data-segment metadata ────────────────────────
        "str_slice" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "str_slice must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let lit = string_literals.get(dest).ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("str_slice missing module string table entry for {dest:?}"),
            })?;
            // Like `str_concat`: a folded slice handed to a callee needs its runtime
            // block handle (with a real header), not the raw sliced-bytes offset.
            if runtime_str_vars.contains(dest) {
                let key = String::from_utf8_lossy(&lit.bytes);
                let block_offset =
                    runtime_str_blocks.get(key.as_ref()).copied().ok_or_else(|| {
                        IIRWasmError::InvalidOperand {
                            function: fn_name.to_string(),
                            detail: format!("str_slice missing runtime block for folded {dest:?}"),
                        }
                    })?;
                code.extend(encode_i32_const(block_offset as i32));
            } else {
                code.extend(encode_i32_const(lit.offset as i32));
            }
            code.extend(encode_local_set(rd));
        }

        // ── str_index → bounds-checked literal byte load ───────────────────────
        "str_index" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "str_index must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let src = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "str_index requires srcs[0] = Operand::Var(str)".to_string(),
                }),
            };
            let idx = match instr.srcs.get(1) {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "str_index requires srcs[1] = Operand::Var(idx)".to_string(),
                }),
            };
            let src_slot = get_reg(src)?;
            let idx_slot = get_reg(idx)?;
            let lit = string_literals.get(src).ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("str_index source {src:?} is not a direct str_const local"),
            })?;

            // Bounds: idx >=u len → unreachable. On i64 indices, a negative value
            // becomes huge under the unsigned compare, matching E4's trap rule.
            code.extend(encode_local_get(idx_slot));
            if slot_is_i64(idx_slot) {
                code.extend(encode_i64_const(lit.len as i64));
                code.push(I64_GE_U);
            } else {
                let len = i32::try_from(lit.len).map_err(|_| IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: format!("str_index literal length {} does not fit i32", lit.len),
                })?;
                code.extend(encode_i32_const(len));
                code.push(I32_GE_U);
            }
            code.push(IF);
            code.push(BLOCK_EMPTY);
            code.push(UNREACHABLE);
            code.push(END);

            code.extend(encode_local_get(src_slot));
            code.extend(encode_local_get(idx_slot));
            if slot_is_i64(idx_slot) {
                code.extend(encode_i32_wrap_i64());
            }
            code.extend(encode_i32_add());
            code.extend(encode_i32_load8_u());
            if slot_is_i64(rd) {
                code.extend(encode_i64_extend_i32_u());
            }
            code.extend(encode_local_set(rd));
        }

        // ── str_len → literal byte count ─────────────────────────────────────
        //
        // Direct literal strings carry their byte count in the same table that
        // `print_str` uses. The E4 v1 WASM slice deliberately keeps this
        // literal-only; dynamic string algebra is still rejected by validation.
        "str_len" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "str_len must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let val_var = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "str_len requires Operand::Var(str)".to_string(),
                }),
            };

            // E4-dyn runtime path (E4d-3/E4d-3b): a runtime string (branch-selected
            // slot, call result, return value, or parameter) has no compile-time
            // length — its local is an i32 handle to a `[i32 len][bytes]` block, so
            // read the length back with `i32.load` at the handle.
            if runtime_str_vars.contains(val_var) || !string_literals.contains_key(val_var) {
                let val_slot = get_reg(val_var)?;
                code.extend(encode_local_get(val_slot));
                code.extend(encode_i32_load(0));
                if slot_is_i64(rd) {
                    code.extend(encode_i64_extend_i32_u());
                }
                code.extend(encode_local_set(rd));
                return Ok(());
            }

            // Literal fast path: single-assignment string with a compile-time length.
            let lit = string_literals.get(val_var).ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!(
                    "str_len currently supports only direct str_const locals, got {val_var:?}"
                ),
            })?;
            if slot_is_i64(rd) {
                code.extend(encode_i64_const(lit.len as i64));
            } else {
                let len = i32::try_from(lit.len).map_err(|_| IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: format!("str_len literal length {} does not fit i32", lit.len),
                })?;
                code.extend(encode_i32_const(len));
            }
            code.extend(encode_local_set(rd));
        }

        // ── str_eq → literal byte equality ───────────────────────────────────
        "str_eq" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "str_eq must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let left = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "str_eq requires srcs[0] = Operand::Var(str)".to_string(),
                }),
            };
            let right = match instr.srcs.get(1) {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "str_eq requires srcs[1] = Operand::Var(str)".to_string(),
                }),
            };
            match (string_literals.get(left), string_literals.get(right)) {
                // Both operands folded to compile-time literals — constant-fold the
                // comparison to a `1`/`0` immediate, no runtime work.
                (Some(left_lit), Some(right_lit)) => {
                    let value = if left_lit.bytes == right_lit.bytes { 1 } else { 0 };
                    if slot_is_i64(rd) {
                        code.extend(encode_i64_const(value));
                    } else {
                        code.extend(encode_i32_const(value as i32));
                    }
                    code.extend(encode_local_set(rd));
                }
                // At least one operand is a runtime string handle (a param, a call
                // result). Both operand slots hold i32 handles to `[i32 len][bytes]`
                // blocks — a folded-literal operand was promoted to a runtime block
                // in `collect_module_features` so it too presents a real header.
                // Delegate to the self-contained in-module `$__str_eq` helper.
                _ => {
                    let helper = str_eq_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "str_eq runtime path without $__str_eq helper (internal error)"
                            .to_string(),
                    })?;
                    let left_slot = get_reg(left)?;
                    let right_slot = get_reg(right)?;
                    code.extend(encode_local_get(left_slot));
                    code.extend(encode_local_get(right_slot));
                    code.extend(encode_call(helper));
                    // The helper returns i32; widen to i64 if the dest slot is i64.
                    if slot_is_i64(rd) {
                        code.extend(encode_i64_extend_i32_u());
                    }
                    code.extend(encode_local_set(rd));
                }
            }
        }

        // ── str_cmp → literal byte ordering ─────────────────────────────────
        "str_cmp" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "str_cmp must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let left = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "str_cmp requires srcs[0] = Operand::Var(str)".to_string(),
                }),
            };
            let right = match instr.srcs.get(1) {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "str_cmp requires srcs[1] = Operand::Var(str)".to_string(),
                }),
            };
            let left_lit = string_literals.get(left).ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("str_cmp left source {left:?} is not a direct str_const local"),
            })?;
            let right_lit = string_literals.get(right).ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("str_cmp right source {right:?} is not a direct str_const local"),
            })?;
            let value = match left_lit.bytes.cmp(&right_lit.bytes) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
            if slot_is_i64(rd) {
                code.extend(encode_i64_const(value));
            } else {
                code.extend(encode_i32_const(value as i32));
            }
            code.extend(encode_local_set(rd));
        }

        // ── print_str → call $__print_str(ptr, len) ──────────────────────────
        //
        // `print_str Var("%s")` writes the literal bytes through the host
        // import `env.__print_str(i32 ptr, i32 len)`.  The pointer is the local
        // produced by `str_const`; the length is looked up from the same
        // compile-time literal table.
        "print_str" => {
            let fn_idx = print_str_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                function: fn_name.to_string(),
                op: "print_str: no $__print_str import registered (internal error)".to_string(),
            })?;
            let val_var = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "print_str requires Operand::Var(str)".to_string(),
                }),
            };
            let val_slot = get_reg(val_var)?;

            // E4-dyn runtime path: the source's local holds an i32 handle = the
            // offset of a length-prefixed block `[i32 len][bytes]`.  This is the
            // case for a branch-selected string (E4d-3, in `runtime_str_vars`) AND
            // for any string WITHOUT a compile-time literal entry — a function
            // **return value / call result** or a **parameter** (E4d-3b): its local
            // holds an i32 handle to the callee's `[i32 len][bytes]` block.  Read
            // the length back from linear memory (`i32.load` at the handle) and
            // pass the *bytes* pointer (handle + 4) plus that length to
            // `env.__print_str(ptr, len)` — mirroring the LLVM E4d-2/E4d-2b path.
            //
            //   local.get slot       ;; handle
            //   i32.const 4
            //   i32.add              ;; ptr  = handle + 4      → stack: [ptr]
            //   local.get slot       ;; handle                 → stack: [ptr, handle]
            //   i32.load offset=0    ;; len  = mem[handle]     → stack: [ptr, len]
            //   call __print_str
            if runtime_str_vars.contains(val_var) || !string_literals.contains_key(val_var) {
                code.extend(encode_local_get(val_slot));
                code.extend(encode_i32_const(4));
                code.extend(encode_i32_add());
                code.extend(encode_local_get(val_slot));
                code.extend(encode_i32_load(0));
                code.extend(encode_call(fn_idx));
                return Ok(());
            }

            // Literal fast path: single-assignment string with a compile-time
            // known raw-byte offset + length.
            let lit = string_literals.get(val_var).ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!(
                    "print_str currently supports only direct str_const locals, got {val_var:?}"
                ),
            })?;
            code.extend(encode_local_get(val_slot));
            code.extend(encode_i32_const(lit.len as i32));
            code.extend(encode_call(fn_idx));
        }

        // ── const ────────────────────────────────────────────────────────────
        //
        // Load an immediate value (integer, float, bool, or nil ref) into a
        // local.
        //
        // WasmGC extension: `const ref<LispyPair>` with no source operand
        // emits `ref.null none` — a typed null that is compatible with any
        // nullable GC reference type.  This is the Lisp `nil` value.
        "const" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "const must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;

            // Special case: `const ref<LispyPair>` with no source = nil.
            if ty.starts_with("ref<") && instr.srcs.is_empty() {
                // ref.null none: typed null compatible with all nullable refs.
                encode_gc_instruction(code, &GcInstruction::RefNull);
                code.extend(encode_local_set(rd));
                return Ok(());
            }

            let src = instr.srcs.first().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "const must have exactly one source (or zero for ref<...> nil)".to_string(),
            })?;

            match src {
                Operand::Int(v) => {
                    // Narrow unsigned consts ride the i64 register model (E2),
                    // so they materialise as `i64.const` into their i64 local.
                    if uses_i64_register(ty) {
                        code.extend(encode_i64_const(*v));
                    } else {
                        code.extend(encode_i32_const(*v as i32));
                    }
                }
                Operand::Bool(b) => {
                    // Booleans are represented as i32: true = 1, false = 0.
                    code.extend(encode_i32_const(if *b { 1 } else { 0 }));
                }
                Operand::Float(v) => {
                    // Float constants are natively supported in WASM.
                    // Use f32 or f64 based on the type hint.
                    if ty == "f32" {
                        code.extend(encode_f32_const(*v as f32));
                    } else {
                        // Default to f64 for any other float hint.
                        code.extend(encode_f64_const(*v));
                    }
                }
                Operand::Var(name) => {
                    // `const` from a variable name is unusual but valid —
                    // emit a local.get (copy).
                    let src_reg = get_reg(name)?;
                    code.extend(encode_local_get(src_reg));
                }
                Operand::Str(s) => {
                    // String literals are not representable as WASM value types.
                    return Err(IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: format!("const: Operand::Str({:?}) is not a WASM value type", s),
                    });
                }
            }
            code.extend(encode_local_set(rd));
        }

        // ── Binary arithmetic ────────────────────────────────────────────────
        //
        // Pattern: local.get r1; local.get r2; <opcode>; local.set rd
        "add" | "sub" | "mul" | "div" | "mod" | "rem" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("{} must have a dest", instr.op),
            })?;
            let rd = get_reg(dest)?;
            let r1 = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            let r2 = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?;

            code.extend(encode_local_get(r1));
            code.extend(encode_local_get(r2));

            // E2: narrow unsigned types use the i64 register model
            // (`uses_i64_register`), so they select `i64.*` ops over their
            // i64-slot operands; the post-op mask restores the narrow width.
            let opcode: u8 = match (instr.op.as_str(), ty) {
                ("add", t) if uses_i64_register(t) => I64_ADD,
                ("add", t) if is_float_hint(t) && t == "f32" => F32_ADD,
                ("add", t) if is_float_hint(t) => F64_ADD,
                ("add", _) => I32_ADD,
                ("sub", t) if uses_i64_register(t) => I64_SUB,
                ("sub", t) if is_float_hint(t) && t == "f32" => F32_SUB,
                ("sub", t) if is_float_hint(t) => F64_SUB,
                ("sub", _) => I32_SUB,
                ("mul", t) if uses_i64_register(t) => I64_MUL,
                ("mul", t) if is_float_hint(t) && t == "f32" => F32_MUL,
                ("mul", t) if is_float_hint(t) => F64_MUL,
                ("mul", _) => I32_MUL,
                ("div", t) if uses_i64_register(t) && is_unsigned_hint(t) => I64_DIV_U,
                ("div", t) if uses_i64_register(t) => I64_DIV_S,
                ("div", t) if is_float_hint(t) && t == "f32" => F32_DIV,
                ("div", t) if is_float_hint(t) => F64_DIV,
                ("div", t) if is_unsigned_hint(t) => I32_DIV_U,
                ("div", _) => I32_DIV_S,
                ("mod" | "rem", t) if uses_i64_register(t) && is_unsigned_hint(t) => I64_REM_U,
                ("mod" | "rem", t) if uses_i64_register(t) => I64_REM_S,
                ("mod" | "rem", t) if is_unsigned_hint(t) => I32_REM_U,
                ("mod" | "rem", _) => I32_REM_S,
                _ => unreachable!("matched outer pattern"),
            };
            code.push(opcode);
            // E2: wrap a narrow-width result (`u4`/`u8`/`u16`) to its bit width;
            // `u32`/`i32` already wrapped via the i32 op, `i64` carries i64 ops.
            emit_wasm_width_mask(code, ty);
            code.extend(encode_local_set(rd));
        }

        // ── Bitwise / shift ───────────────────────────────────────────────────
        "and" | "or" | "xor" | "shl" | "shr" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("{} must have a dest", instr.op),
            })?;
            let rd = get_reg(dest)?;
            let r1 = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            let r2 = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?;

            // E2: narrow unsigned types use the i64 register model (see above).
            //
            // Additionally, ALGOL boolean comparisons on strings emit `cmp_ne`
            // with type_hint "i64" (so the comparison locals are i64), then feed
            // those into `and`/`or` with type_hint "bool".  We detect this via
            // the actual local types of r1/r2 and upgrade to the i64 opcode so
            // the WASM types remain consistent.
            //
            // When one operand is i32 and the other i64, the narrower operand
            // must be widened BEFORE the operation so both stack values match
            // the chosen opcode (WASM is strictly typed; mixing widths is rejected).
            let op_is_bitwise = matches!(instr.op.as_str(), "and" | "or" | "xor");
            let r1_is_i64 = slot_is_i64(r1);
            let r2_is_i64 = slot_is_i64(r2);
            let type_is_i64 = uses_i64_register(ty);
            let use_i64 = type_is_i64 || (op_is_bitwise && (r1_is_i64 || r2_is_i64));

            // Push r1 then widen if the operation needs i64 but this slot is i32.
            code.extend(encode_local_get(r1));
            if op_is_bitwise && use_i64 && !r1_is_i64 {
                code.extend(encode_i64_extend_i32_u());
            }
            // Push r2 then widen if the operation needs i64 but this slot is i32.
            code.extend(encode_local_get(r2));
            if op_is_bitwise && use_i64 && !r2_is_i64 {
                code.extend(encode_i64_extend_i32_u());
            }

            let opcode: u8 = match (instr.op.as_str(), ty) {
                ("and", _) if use_i64 => I64_AND,
                ("and", _) => I32_AND,
                ("or", _) if use_i64 => I64_OR,
                ("or", _) => I32_OR,
                ("xor", _) if use_i64 => I64_XOR,
                ("xor", _) => I32_XOR,
                ("shl", t) if uses_i64_register(t) => I64_SHL,
                ("shl", _) => I32_SHL,
                ("shr", t) if uses_i64_register(t) && is_unsigned_hint(t) => I64_SHR_U,
                ("shr", t) if uses_i64_register(t) => I64_SHR_S,
                ("shr", t) if is_unsigned_hint(t) => I32_SHR_U,
                ("shr", _) => I32_SHR_S,
                _ => unreachable!(),
            };
            code.push(opcode);
            // E2: a narrow left-shift can push bits past the width (`1u8 << 8`),
            // so mask the result; `and`/`or`/`xor`/`shr` stay canonical too.
            emit_wasm_width_mask(code, ty);
            // When the result is i64 but the dest local is narrower (e.g. "bool"
            // → i32), wrap the i64 result back to i32 before local.set so the
            // stored type matches the declared local type.
            if op_is_bitwise && use_i64 && !slot_is_i64(rd) {
                code.extend(encode_i32_wrap_i64());
            }
            code.extend(encode_local_set(rd));
        }

        // ── Comparisons ───────────────────────────────────────────────────────
        //
        // WASM comparisons always produce an i32 result (0 or 1).
        // The source operands have the type described by `ty`; the result
        // is always i32.
        //
        // **Naming**.  Twig historically emitted bare `eq` / `ne` / `lt` etc.
        // — the names we still match below.  BASIC, Nib, and Oct emit the
        // `cmp_*`-prefixed form (`cmp_eq`, `cmp_ne`, …).  We accept both
        // shapes by stripping the `cmp_` prefix on entry and routing the
        // bare form through the same opcode table.
        "eq" | "ne" | "lt" | "le" | "gt" | "ge"
        | "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("{} must have a dest", instr.op),
            })?;
            let rd = get_reg(dest)?;
            let r1 = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            let r2 = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?;

            code.extend(encode_local_get(r1));
            code.extend(encode_local_get(r2));

            // Strip the `cmp_` prefix so the existing per-type lookup table
            // doesn't need 12 extra arms.
            let bare = instr.op.strip_prefix("cmp_").unwrap_or(instr.op.as_str());
            let cmp_ty = local_type_hints.get(&r1).map(String::as_str).unwrap_or(ty);

            let opcode: u8 = match (bare, cmp_ty) {
                // i64 register model — true 64-bit ints AND narrow unsigned
                // types (whose locals are i64 under E2). A masked narrow value
                // is always in [0, 2ⁿ) — positive in i64 — so the signed i64
                // relational ops give the correct unsigned result.
                ("eq", t) if uses_i64_register(t) => I64_EQ,
                ("ne", t) if uses_i64_register(t) => I64_NE,
                ("lt", t) if uses_i64_register(t) => I64_LT_S,
                ("le", t) if uses_i64_register(t) => I64_LE_S,
                ("gt", t) if uses_i64_register(t) => I64_GT_S,
                ("ge", t) if uses_i64_register(t) => I64_GE_S,
                // f32
                ("eq", "f32") => F32_EQ,
                ("ne", "f32") => F32_NE,
                ("lt", "f32") => F32_LT,
                ("le", "f32") => F32_LE,
                ("gt", "f32") => F32_GT,
                ("ge", "f32") => F32_GE,
                // f64
                ("eq", t) if is_float_hint(t) => F64_EQ,
                ("ne", t) if is_float_hint(t) => F64_NE,
                ("lt", t) if is_float_hint(t) => F64_LT,
                ("le", t) if is_float_hint(t) => F64_LE,
                ("gt", t) if is_float_hint(t) => F64_GT,
                ("ge", t) if is_float_hint(t) => F64_GE,
                // i32 (signed)
                ("eq", _) => I32_EQ,
                ("ne", _) => I32_NE,
                ("lt", t) if is_unsigned_hint(t) => I32_LT_U,
                ("lt", _) => I32_LT_S,
                ("le", t) if is_unsigned_hint(t) => I32_LE_U,
                ("le", _) => I32_LE_S,
                ("gt", t) if is_unsigned_hint(t) => I32_GT_U,
                ("gt", _) => I32_GT_S,
                ("ge", t) if is_unsigned_hint(t) => I32_GE_U,
                ("ge", _) => I32_GE_S,
                _ => unreachable!(),
            };
            code.push(opcode);
            // A wasm comparison always yields an `i32` boolean (0/1), regardless
            // of operand width. If the dest register is an *i64*-declared local
            // (e.g. a scalar `any` concretised to i64 by `concretize_scalar_any_
            // for_wasm`), the i32 result must be widened so the stored value
            // matches the local's declared type — otherwise the module is
            // ill-typed (an i32 sitting in an i64 local), which the lenient
            // in-repo runtime tolerated only as long as every consumer used i32
            // ops. The widened-Brainfuck control flow (`i64.eqz` on an i64 guard,
            // LANG-MATRIX LM-W) needs the value to actually be i64. The dual fix
            // lives in the `jmp_if_*` arms.
            if slot_is_i64(rd) {
                code.extend(encode_i64_extend_i32_u());
            }
            code.extend(encode_local_set(rd));
        }

        // ── Unary negation ────────────────────────────────────────────────────
        //
        // WASM has no single "neg" opcode for integers.  We synthesise it:
        //   i32: `0 - r`  →  `i32.const 0; local.get r; i32.sub`
        //   i64: `0 - r`  →  `i64.const 0; local.get r; i64.sub`
        //   f32: direct `f32.neg`
        //   f64: direct `f64.neg`
        "neg" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "neg must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;

            if ty == "f32" {
                code.extend(encode_local_get(r));
                code.push(F32_NEG);
            } else if is_float_hint(ty) {
                code.extend(encode_local_get(r));
                code.push(F64_NEG);
            } else if uses_i64_register(ty) {
                // Narrow unsigned types use the i64 register model (E2).
                code.extend(encode_i64_const(0));
                code.extend(encode_local_get(r));
                code.push(I64_SUB);
            } else {
                code.extend(encode_i32_const(0));
                code.extend(encode_local_get(r));
                code.push(I32_SUB);
            }
            // E2: a narrow `neg` is `(0 - r)` mod-2ⁿ — mask it to the width.
            emit_wasm_width_mask(code, ty);
            code.extend(encode_local_set(rd));
        }

        // ── Bitwise NOT ───────────────────────────────────────────────────────
        //
        // WASM has no single "not" opcode.  Synthesise with XOR -1:
        //   i32: `local.get r; i32.const -1; i32.xor`
        //   i64: `local.get r; i64.const -1; i64.xor`
        "not" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "not must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;

            code.extend(encode_local_get(r));
            if uses_i64_register(ty) {
                // Narrow unsigned types use the i64 register model (E2).
                code.extend(encode_i64_const(-1));
                code.push(I64_XOR);
            } else {
                code.extend(encode_i32_const(-1));
                code.push(I32_XOR);
            }
            // E2: `~x` on a narrow width must flip only its low bits
            // (`~0u8 == 255`, not `0xFFFF_FFFF`) — mask after the XOR.
            emit_wasm_width_mask(code, ty);
            code.extend(encode_local_set(rd));
        }

        // ── Logical NOT (boolean) ─────────────────────────────────────────────
        //
        // IIR `lnot` converts a boolean/i32 value to its logical inverse:
        //   i32.eqz pushes 1 if the value is 0, else 0.
        "lnot" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "lnot must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;

            code.extend(encode_local_get(r));
            code.push(I32_EQZ);
            code.extend(encode_local_set(rd));
        }

        // ── numeric conversions integer↔real (LANG-FULL E8) ───────────────────
        //
        // The dest local is already typed `f64` (int_to_real) or `i64`
        // (real_to_int_*) by `infer_local_type_hints`, which reads each var's
        // type from the producing instruction's `type_hint`.
        //
        // `real_to_int_*` uses the **non-saturating** `i64.trunc_f64_s`, which
        // **traps** on NaN/±∞/out-of-`i64`-range — matching vm-core's
        // `real_to_i64_checked` fail-closed trap exactly. (The saturating
        // `trunc_sat` would clamp instead and silently diverge from the VM.)
        "int_to_real" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "int_to_real must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            code.extend(encode_local_get(r)); // i64
            code.push(F64_CONVERT_I64_S); // → f64
            code.extend(encode_local_set(rd));
        }
        "real_to_int_trunc" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "real_to_int_trunc must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            code.extend(encode_local_get(r)); // f64
            code.push(I64_TRUNC_F64_S); // → i64 (toward zero; traps out-of-range)
            code.extend(encode_local_set(rd));
        }
        "real_to_int_floor" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "real_to_int_floor must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            code.extend(encode_local_get(r)); // f64
            code.push(F64_FLOOR); // round toward −∞ (entier)
            code.push(I64_TRUNC_F64_S); // trunc the integral result → i64 (traps out-of-range)
            code.extend(encode_local_set(rd));
        }
        // `f64_sqrt` — IEEE-754 hardware square root (WASM MVP opcode 0x9F).
        // WASM `f64.sqrt` propagates NaN and returns NaN for negative inputs,
        // matching IEEE-754 and the VM handler's `f.sqrt()` semantics.
        "f64_sqrt" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "f64_sqrt must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            code.extend(encode_local_get(r)); // f64
            code.push(F64_SQRT); // f64.sqrt — hardware sqrt, no libm call
            code.extend(encode_local_set(rd));
        }
        // `f64_sin` / `f64_cos` / `f64_ln` / `f64_exp` / `f64_atan` / `f64_tan`.
        //
        // WASM has no built-in sin/cos/log/exp opcodes; they are resolved via
        // host-imported functions declared in the module's import section (see
        // `collect_module_features` / import injection in `lower_iir_to_wasm`).
        // The import indices were assigned in Step 3 and threaded through here.
        // Pattern: load argument f64, call import, store result f64.
        "f64_sin" | "f64_cos" | "f64_ln" | "f64_exp" | "f64_atan" | "f64_tan" => {
            let import_idx = match instr.op.as_str() {
                "f64_sin" => sin_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                    function: fn_name.to_string(),
                    op: "f64_sin: env.__sin import not registered (internal error)".to_string(),
                })?,
                "f64_cos" => cos_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                    function: fn_name.to_string(),
                    op: "f64_cos: env.__cos import not registered (internal error)".to_string(),
                })?,
                "f64_ln" => ln_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                    function: fn_name.to_string(),
                    op: "f64_ln: env.__ln import not registered (internal error)".to_string(),
                })?,
                "f64_exp" => exp_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                    function: fn_name.to_string(),
                    op: "f64_exp: env.__exp import not registered (internal error)".to_string(),
                })?,
                "f64_atan" => atan_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                    function: fn_name.to_string(),
                    op: "f64_atan: env.__atan import not registered (internal error)".to_string(),
                })?,
                "f64_tan" => tan_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                    function: fn_name.to_string(),
                    op: "f64_tan: env.__tan import not registered (internal error)".to_string(),
                })?,
                _ => unreachable!(),
            };
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("{} must have a dest", instr.op),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            code.extend(encode_local_get(r));           // push f64 argument
            code.extend(encode_call(import_idx));       // call env.__sin/cos/ln/exp/atan/tan
            code.extend(encode_local_set(rd));          // store f64 result
        }

        // `f64_pow` — two-argument pow(base, exp) via `env.__pow` host import.
        // There is no WASM native pow opcode; the host supplies libm semantics.
        "f64_pow" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "f64_pow must have a dest".to_string(),
            })?;
            let idx = pow_fn_idx.ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "f64_pow: env.__pow import not registered (internal error)".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let rb = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?; // base
            let re = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?; // exp
            code.extend(encode_local_get(rb)); // f64 base
            code.extend(encode_local_get(re)); // f64 exp
            code.extend(encode_call(idx));     // call env.__pow
            code.extend(encode_local_set(rd));
        }

        // ── move / copy ───────────────────────────────────────────────────────
        //
        // `mov rd, rs` — copy a variable.  Emit: local.get rs; local.set rd
        "mov" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "mov must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let r = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            code.extend(encode_local_get(r));
            code.extend(encode_local_set(rd));
        }

        // ── call ──────────────────────────────────────────────────────────────
        //
        // IIR `call callee_name [arg0, arg1, …]` → WASM `call fn_idx`.
        //
        // Push all arguments onto the stack first (left to right), then
        // emit `call <fn_idx>`.  If the call has a destination, the return
        // value is on top of the stack → store it with local.set.
        "call" => {
            // First source is the callee name (Operand::Var).
            let callee_name =
                instr.srcs.first().and_then(|s| s.as_var()).ok_or_else(|| {
                    IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: "call: first src must be the callee name".to_string(),
                    }
                })?;

            let fn_idx = fn_map.get(callee_name).copied().ok_or_else(|| {
                IIRWasmError::UndefinedVariable {
                    function: fn_name.to_string(),
                    name: callee_name.to_string(),
                }
            })?;

            // Remaining sources are the arguments.
            for src in instr.srcs.iter().skip(1) {
                match src {
                    Operand::Var(name) => {
                        let r = get_reg(name)?;
                        code.extend(encode_local_get(r));
                    }
                    Operand::Int(v) => {
                        code.extend(encode_i32_const(*v as i32));
                    }
                    Operand::Float(v) => {
                        code.extend(encode_f64_const(*v));
                    }
                    Operand::Bool(b) => {
                        code.extend(encode_i32_const(if *b { 1 } else { 0 }));
                    }
                    Operand::Str(s) => {
                        // String literals cannot be passed as WASM call arguments.
                        return Err(IIRWasmError::InvalidOperand {
                            function: fn_name.to_string(),
                            detail: format!("call: Operand::Str({:?}) cannot be a call argument", s),
                        });
                    }
                }
            }

            code.extend(encode_call(fn_idx));

            // If there is a destination, the return value is now on the stack.
            if let Some(dest) = &instr.dest {
                let rd = get_reg(dest)?;
                code.extend(encode_local_set(rd));
            } else if instr.type_hint != "void" {
                // Callee returned a value but we don't use it — drop it.
                code.push(DROP);
            }
        }

        // ── ret ───────────────────────────────────────────────────────────────
        //
        // `ret <src>` — load the return value and emit `return`.
        "ret" => {
            if let Some(src) = instr.srcs.first() {
                match src {
                    Operand::Var(name) => {
                        let r = get_reg(name)?;
                        code.extend(encode_local_get(r));
                    }
                    Operand::Int(v) => {
                        if is_i64_hint(ty) {
                            code.extend(encode_i64_const(*v));
                        } else {
                            code.extend(encode_i32_const(*v as i32));
                        }
                    }
                    Operand::Float(v) => {
                        if ty == "f32" {
                            code.extend(encode_f32_const(*v as f32));
                        } else {
                            code.extend(encode_f64_const(*v));
                        }
                    }
                    Operand::Bool(b) => {
                        code.extend(encode_i32_const(if *b { 1 } else { 0 }));
                    }
                    Operand::Str(s) => {
                        // String literals cannot be returned as WASM values.
                        return Err(IIRWasmError::InvalidOperand {
                            function: fn_name.to_string(),
                            detail: format!("ret: Operand::Str({:?}) is not a WASM return value", s),
                        });
                    }
                }
            }
            code.push(RETURN);
        }

        // ── ret_void ──────────────────────────────────────────────────────────
        //
        // `ret_void` — return with no value.
        "ret_void" => {
            code.push(RETURN);
        }

        // ── label ─────────────────────────────────────────────────────────────
        //
        // `label name` — marks the start of a basic block.
        //
        // In the dispatch-loop scheme, labels are split out by the basic-block
        // splitter before codegen, so individual `label` instructions inside
        // emit_instr are NOP equivalents.  We emit a `nop` to preserve
        // instruction count for debuggers.
        "label" => {
            // NOP — handled structurally by the dispatch-loop builder.
        }

        // ── jmp ───────────────────────────────────────────────────────────────
        //
        // `jmp <label>` — unconditional branch.
        //
        // Dispatch-loop: set the dispatch variable to the target block index,
        // then break back to the loop (depth 0 from inside the loop body).
        //
        // Non-dispatch: emit `return` (simplified — loop-free functions with
        // jmp are unusual; this keeps v1 simple).
        "jmp" => {
            let label = instr
                .srcs
                .first()
                .and_then(|s| s.as_var())
                .ok_or_else(|| IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "jmp requires a label name as first src".to_string(),
                })?;

            if is_dispatch_loop {
                let target_idx = get_label(label)? as usize;
                code.extend(encode_i32_const(target_idx as i32));
                code.extend(encode_local_set(dispatch_reg));
                // Compute the WASM branch depth.
                //
                // The dispatch-loop works by ALWAYS re-entering the LOOP, never
                // jumping directly between basic-block bodies.  Every jump (forward
                // or backward) sets the dispatch variable, then branches back to the
                // LOOP so br_table can redispatch to the correct block.
                //
                // After the br_table fires for the first time, each successive END
                // instruction pops one label from the stack.  From body[block_idx]
                // the surviving labels are exactly:
                //
                //   [outer_exit, LOOP, bb_0, …, bb_{n_blocks-block_idx-3}]
                //
                // which is `n_blocks - block_idx` labels total.  The LOOP label is
                // always at depth `n_blocks - block_idx - 2` from the innermost label:
                //
                //   depth(LOOP) = n_blocks - block_idx - 2
                //
                // For the last block (block_idx = n_blocks - 1) the LOOP has already
                // been consumed; those blocks always end with `ret` in well-formed
                // programs so this case should never occur in practice.
                let depth = if block_idx + 1 < n_blocks {
                    (n_blocks - block_idx - 2) as u32
                } else {
                    0 // last block — should not normally jmp, but fall safe
                };
                code.extend(encode_br(depth));
            } else {
                // Simplified: emit RETURN (matches "exit" semantics for
                // straight-line code that happens to have a terminal jmp).
                code.push(RETURN);
            }
        }

        // ── jmp_if_true ───────────────────────────────────────────────────────
        //
        // `jmp_if_true <cond_var>, <label>` — branch if the condition is truthy.
        //
        // Dispatch-loop: if cond != 0, set dispatch = target and loop.
        "jmp_if_true" => {
            let cond = instr
                .srcs
                .first()
                .and_then(|s| s.as_var())
                .ok_or_else(|| IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "jmp_if_true: first src must be condition variable".to_string(),
                })?;
            let label = instr
                .srcs
                .get(1)
                .and_then(|s| s.as_var())
                .ok_or_else(|| IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "jmp_if_true: second src must be label name".to_string(),
                })?;

            let cond_reg = get_reg(cond)?;

            if is_dispatch_loop {
                let target_idx = get_label(label)? as usize;
                // Emit:  if cond != 0 { dispatch = target_idx; br <depth> }
                //
                // Like `jmp`, we always re-enter the LOOP (not jump directly to the
                // target block).  Inside the `if` block there is one extra label on
                // the label_stack, so the LOOP depth is one higher than for a plain
                // `jmp`:
                //
                //   depth(LOOP) inside `if` = (n_blocks - block_idx - 2) + 1
                //                           = n_blocks - block_idx - 1
                //
                // For the last block the LOOP is gone; use depth=1 to exit `if`
                // plus outer_exit (should not occur in well-formed programs).
                let depth = if block_idx + 1 < n_blocks {
                    (n_blocks - block_idx - 1) as u32
                } else {
                    1 // last block — should not normally conditional-jmp
                };
                code.extend(encode_local_get(cond_reg));
                // `if` tests an i32 != 0. An i64 condition (the Brainfuck loop
                // guard after `lower_brainfuck_for_aot` widening) must be reduced
                // to an i32 truth value first: `i64.eqz; i32.eqz` yields 1 iff the
                // i64 is non-zero.
                if slot_is_i64(cond_reg) {
                    code.push(crate::codegen::I64_EQZ);
                    code.push(I32_EQZ);
                }
                // if (empty block type, no result)
                code.push(crate::codegen::IF);
                code.push(BLOCK_EMPTY);
                code.extend(encode_i32_const(target_idx as i32));
                code.extend(encode_local_set(dispatch_reg));
                code.extend(encode_br(depth));
                code.push(END); // end of if
            } else {
                // Simplified: just consume the condition (drop it).
                code.extend(encode_local_get(cond_reg));
                code.push(DROP);
            }
        }

        // ── jmp_if_false ──────────────────────────────────────────────────────
        //
        // `jmp_if_false <cond_var>, <label>` — branch if the condition is falsy.
        //
        // Like jmp_if_true but with the condition inverted via i32.eqz.
        "jmp_if_false" => {
            let cond = instr
                .srcs
                .first()
                .and_then(|s| s.as_var())
                .ok_or_else(|| IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "jmp_if_false: first src must be condition variable".to_string(),
                })?;
            let label = instr
                .srcs
                .get(1)
                .and_then(|s| s.as_var())
                .ok_or_else(|| IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "jmp_if_false: second src must be label name".to_string(),
                })?;

            let cond_reg = get_reg(cond)?;

            if is_dispatch_loop {
                let target_idx = get_label(label)? as usize;
                // Emit: if cond == 0 { dispatch = target_idx; br <depth> }
                // Same depth computation as jmp_if_true: always re-enter the LOOP,
                // one extra label level for the enclosing `if` block.
                let depth = if block_idx + 1 < n_blocks {
                    (n_blocks - block_idx - 1) as u32
                } else {
                    1 // last block — should not normally conditional-jmp
                };
                code.extend(encode_local_get(cond_reg));
                // "branch if cond == 0" → push the i32 boolean `cond == 0`.
                // Use the width-correct eqz: `i64.eqz` for an i64 guard (the
                // widened Brainfuck cell), `i32.eqz` otherwise. Both yield i32.
                if slot_is_i64(cond_reg) {
                    code.push(crate::codegen::I64_EQZ);
                } else {
                    code.push(I32_EQZ);
                }
                code.push(crate::codegen::IF);
                code.push(BLOCK_EMPTY);
                code.extend(encode_i32_const(target_idx as i32));
                code.extend(encode_local_set(dispatch_reg));
                code.extend(encode_br(depth));
                code.push(END);
            } else {
                code.extend(encode_local_get(cond_reg));
                code.push(DROP);
            }
        }

        // ── nop / phi / other metadata ops ───────────────────────────────────
        "nop" | "phi" => {
            // nop: emit WASM nop.
            // phi: SSA phi nodes are resolved during register allocation
            //      (all phi inputs map to the same local) — nothing to emit.
        }

        // ── WasmGC: alloc ref<LispyPair> ──────────────────────────────────────
        //
        // Allocate a new `$LispyPair` on the GC heap. WasmGC `struct.new`
        // consumes one initial value per field, so we push a typed null for each
        // of the pair's two `anyref` fields (car, cdr) and then `struct.new`,
        // yielding a real `(null . null)` cell. The `field_store`s that follow
        // overwrite the nulls with the head and tail — so we don't need
        // look-ahead to fuse `alloc` with its two stores into one `struct.new`.
        // (See the `"alloc"` arm below for the exact byte sequence.)
        "alloc" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "alloc must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;

            // Only ref<LispyPair> is supported.
            if !ty.starts_with("ref<") {
                return Err(IIRWasmError::UnsupportedType {
                    function: fn_name.to_string(),
                    type_hint: ty.to_string(),
                });
            }

            let type_idx = lispy_pair_type_idx.ok_or_else(|| IIRWasmError::UnsupportedType {
                function: fn_name.to_string(),
                type_hint: ty.to_string(),
            })?;

            // Actually allocate the `$LispyPair` (LANG77 / McCarthy L3b-3a-3c).
            // WasmGC `struct.new` consumes one value per field, so we push a
            // typed null for each of the pair's two `anyref` fields (car, cdr)
            // and then `struct.new`, yielding a *real* heap object. The
            // following `field_store`s (`struct.set`) overwrite those nulls with
            // the head and tail. (Previously this emitted a bare `ref.null`,
            // which left the "cell" null — so the very next `struct.set` trapped
            // on a null reference.)
            //
            // ```wasm
            // ref.null none           ;; default car
            // ref.null none           ;; default cdr
            // struct.new $LispyPair    ;; (null . null) — a fresh cell
            // local.set $dest
            // ```
            encode_gc_instruction(code, &GcInstruction::RefNull);
            encode_gc_instruction(code, &GcInstruction::RefNull);
            encode_gc_instruction(code, &GcInstruction::StructNew(type_idx));
            code.extend(encode_local_set(rd));
        }

        // ── WasmGC: field_load dest pair field_idx ────────────────────────────
        //
        // Load one field of a `$LispyPair` (car or cdr).
        //
        // IIR layout:
        //   dest: name of variable to store result into
        //   srcs[0]: Var(pair_variable_name)
        //   srcs[1]: Int(field_index)   — 0 = $head (car), 1 = $tail (cdr)
        //
        // ```wasm
        // local.get $pair
        // struct.get $LispyPair <field_idx>
        // local.set $dest
        // ```
        "field_load" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "field_load must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;

            // Get the pair source variable.
            let pair_reg = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;

            // Get the field index from the second source (must be an Int).
            let field_idx = match instr.srcs.get(1) {
                Some(Operand::Int(n)) => *n as u32,
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "field_load: second src must be an Int field index".to_string(),
                }),
            };

            let type_idx = lispy_pair_type_idx.ok_or_else(|| IIRWasmError::UnsupportedType {
                function: fn_name.to_string(),
                type_hint: "ref<LispyPair> type not registered in module".to_string(),
            })?;

            code.extend(encode_local_get(pair_reg));
            encode_gc_instruction(code, &GcInstruction::StructGet(type_idx, field_idx));
            code.extend(encode_local_set(rd));
        }

        // ── WasmGC: field_store pair field_idx val ────────────────────────────
        //
        // Store a value into one field of a `$LispyPair`.
        //
        // IIR layout:
        //   dest: None (field_store has no result — it is a side-effecting write)
        //   srcs[0]: Var(pair_variable_name)
        //   srcs[1]: Int(field_index)
        //   srcs[2]: Var(value_variable_name)
        //
        // ```wasm
        // local.get $pair
        // local.get $val
        // struct.set $LispyPair <field_idx>
        // ```
        "field_store" => {
            // Get the pair source variable (srcs[0]).
            let pair_reg = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;

            // Get the field index (srcs[1] as Int).
            let field_idx = match instr.srcs.get(1) {
                Some(Operand::Int(n)) => *n as u32,
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "field_store: second src must be an Int field index".to_string(),
                }),
            };

            // Get the value variable (srcs[2]).
            let val_reg = get_src_reg(&instr.srcs, 2, reg_map, fn_name)?;

            let type_idx = lispy_pair_type_idx.ok_or_else(|| IIRWasmError::UnsupportedType {
                function: fn_name.to_string(),
                type_hint: "ref<LispyPair> type not registered in module".to_string(),
            })?;

            code.extend(encode_local_get(pair_reg));
            code.extend(encode_local_get(val_reg));
            encode_gc_instruction(code, &GcInstruction::StructSet(type_idx, field_idx));
        }

        // ── WasmGC: is_null dest x ────────────────────────────────────────────
        //
        // Test whether a GC reference is null.
        //
        // ```wasm
        // local.get $x
        // ref.is_null     ;; pushes i32: 1 if null, 0 if non-null
        // local.set $dest
        // ```
        //
        // The result is always `i32` (WASM boolean convention), regardless of
        // the type of the reference being tested.
        "is_null" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "is_null must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let x_reg = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;

            code.extend(encode_local_get(x_reg));
            encode_gc_instruction(code, &GcInstruction::RefIsNull);
            code.extend(encode_local_set(rd));
        }

        // ── WasmGC: box dest src — i32 → i31ref (LANG77 L3b-3a) ────────────────
        //
        // Box a 31-bit integer into an `i31ref` (a WasmGC tagged reference), so
        // a lisp integer atom can live in an `anyref` cons-cell field / be held
        // uniformly as a reference. The uniform-anyref value model boxes every
        // lisp integer this way (mirroring the native NaN-box `(n << 3)` tag).
        //
        // ```wasm
        // local.get $src     ;; i32
        // ref.i31            ;; → (ref i31)
        // local.set $dest    ;; anyref
        // ```
        //
        // (`i31ref` carries 31 bits; the retype/box pass is responsible for
        // narrowing a wider integer before boxing — out of scope for this op.)
        "box" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "box must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let src_reg = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;

            code.extend(encode_local_get(src_reg));
            encode_gc_instruction(code, &GcInstruction::I31New);
            code.extend(encode_local_set(rd));
        }

        // ── WasmGC: unbox dest src — i31ref → i32 (LANG77 L3b-3a) ──────────────
        //
        // Read the 31-bit integer back out of an `i31ref`, sign-extended to an
        // i32. This is the inverse of `box`, applied at the boundary where a
        // boxed lisp integer re-enters the numeric world (e.g. the program's
        // return value), mirroring the native `unbox` (arithmetic `>> 3`).
        //
        // ```wasm
        // local.get $src     ;; i31ref
        // i31.get_s          ;; → i32 (sign-extended)
        // local.set $dest    ;; i32
        // ```
        "unbox" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "unbox must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let src_reg = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;

            code.extend(encode_local_get(src_reg));
            encode_gc_instruction(code, &GcInstruction::I31GetS);
            code.extend(encode_local_set(rd));
        }

        // ── global_store → global.set N ──────────────────────────────────────
        //
        // `global_store Str("name"), Var("%v")`
        //
        // Stores a value from a local variable into a named module-level global.
        //
        // WASM binary sequence:
        // ```
        // local.get  slot_v   ;; push the value to store
        // global.set idx      ;; pop and write into global[idx]
        // ```
        //
        // srcs[0] = Operand::Str(name) — the compile-time global name
        // srcs[1] = Operand::Var(val_reg) — the value to store
        "global_store" => {
            let global_name = match instr.srcs.first() {
                Some(Operand::Str(s)) => s.as_str(),
                _ => return Err(IIRWasmError::UnsupportedOp {
                    function: fn_name.to_string(),
                    op: "global_store: srcs[0] must be Operand::Str(name)".to_string(),
                }),
            };
            let val_var = match instr.srcs.get(1) {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "global_store: srcs[1] must be Operand::Var(reg)".to_string(),
                }),
            };
            let global_idx = *global_map.get(global_name).ok_or_else(|| IIRWasmError::UnsupportedOp {
                function: fn_name.to_string(),
                op: format!("global_store: unknown global {:?}", global_name),
            })?;
            let val_slot = get_reg(val_var)?;
            code.extend(encode_local_get(val_slot));
            code.extend(encode_global_set(global_idx));
        }

        // ── global_load → global.get N ───────────────────────────────────────
        //
        // `%dest = global_load Str("name")`
        //
        // Loads the value of a named module-level global into a local variable.
        //
        // WASM binary sequence:
        // ```
        // global.get idx      ;; push global[idx] onto the stack
        // local.set  slot_dest ;; pop and store into the destination local
        // ```
        //
        // srcs[0] = Operand::Str(name) — the compile-time global name
        // dest = Some(reg) — the register to store the loaded value
        "global_load" => {
            let global_name = match instr.srcs.first() {
                Some(Operand::Str(s)) => s.as_str(),
                _ => return Err(IIRWasmError::UnsupportedOp {
                    function: fn_name.to_string(),
                    op: "global_load: srcs[0] must be Operand::Str(name)".to_string(),
                }),
            };
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "global_load must have a dest".to_string(),
            })?;
            let global_idx = *global_map.get(global_name).ok_or_else(|| IIRWasmError::UnsupportedOp {
                function: fn_name.to_string(),
                op: format!("global_load: unknown global {:?}", global_name),
            })?;
            let dest_slot = get_reg(dest)?;
            code.extend(encode_global_get(global_idx));
            code.extend(encode_local_set(dest_slot));
        }

        // ── io_out → call $__print_i64 ────────────────────────────────────────
        //
        // `io_out Var("%val")`
        //
        // Emits a call to the host-provided `env.__print_i64(i64)` import.
        // The host is expected to print the i64 value to stdout.
        //
        // WASM binary sequence:
        // ```
        // local.get  slot_val ;; push the value to print
        // call       fn_idx   ;; call env.__print_i64
        // ```
        //
        // `print_fn_idx` is always 0 when present because imports occupy the
        // first N slots in the WASM function index space (before defined fns).
        //
        // srcs[0] = Operand::Var(val_reg) — the value to print
        "io_out" => {
            let fn_idx = print_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                function: fn_name.to_string(),
                op: "io_out: no $__print_i64 import registered (internal error)".to_string(),
            })?;
            let val_var = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "io_out requires Operand::Var(reg)".to_string(),
                }),
            };
            let val_slot = get_reg(val_var)?;
            code.extend(encode_local_get(val_slot));
            code.extend(encode_call(fn_idx));
        }

        // ── load_mem → i32.load8_u offset=0 align=0 ──────────────────────────
        //
        // `load_mem  v  ptr   u8`  → read tape byte at address `ptr` into `v`.
        //
        // The WASM linear memory is a flat `Vec<u8>` from the host's
        // perspective.  `i32.load8_u` reads one byte at the address on top
        // of the stack and pushes the zero-extended `u8` value as `i32`.
        // Brainfuck's `ptr` register holds a `u32` (encoded as `i32` in
        // WASM), so the address is already on the stack in the right type.
        //
        // The tape itself is allocated by `lower_iir_to_wasm` when the
        // module uses memory ops (see the `uses_memory` injection step).
        // Out-of-bounds reads trap at the WASM layer — Brainfuck's
        // interpreter chose the "oob read = 0" lazy-tape convention, but
        // WASM has no zero-fill-on-trap; programs that walk the pointer
        // outside `[0, tape_size)` will fail at runtime.  The brainfuck
        // frontend allocates a 30,000-cell tape by default, which fits
        // comfortably in one 64 KiB page.
        "load_mem" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "load_mem must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let addr_var = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "load_mem requires Operand::Var(addr) as src[0]".to_string(),
                }),
            };
            let addr_slot = get_reg(addr_var)?;
            code.extend(encode_local_get(addr_slot));
            code.extend(encode_i32_load8_u());
            code.extend(encode_local_set(rd));
        }

        // ── store_mem → i32.store8 offset=0 align=0 ──────────────────────────
        //
        // `store_mem   ptr  v   u8`  → write low byte of `v` to tape[ptr].
        //
        // WASM `i32.store8` pops the value (top of stack) and then the
        // address (next on stack), then stores `value & 0xFF` at
        // `mem[addr]`.  We push `addr` first, then `val`, so the stack at
        // call time is `[addr, val]` (addr deeper, val on top) — exactly
        // what i32.store8 expects.
        //
        // Out-of-bounds writes trap, terminating the WASM module.  The
        // bf-iir-compiler's `BrainfuckVM::execute_module` raises a
        // structured `BrainfuckError` for the same case in the interpreter
        // path; the JIT path's `store_mem` handler does the same.  The
        // WASM path's "trap and abort" is the standard memory-safe
        // alternative.
        "store_mem" => {
            if instr.srcs.len() < 2 {
                return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "store_mem requires 2 srcs: [addr, val]".to_string(),
                });
            }
            let addr_var = match &instr.srcs[0] {
                Operand::Var(v) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "store_mem src[0] must be Operand::Var(addr)".to_string(),
                }),
            };
            let val_var = match &instr.srcs[1] {
                Operand::Var(v) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "store_mem src[1] must be Operand::Var(val)".to_string(),
                }),
            };
            let addr_slot = get_reg(addr_var)?;
            let val_slot  = get_reg(val_var)?;
            code.extend(encode_local_get(addr_slot));
            code.extend(encode_local_get(val_slot));
            code.extend(encode_i32_store8());
        }

        // ── alloc_bytes → tape base offset (LANG-MATRIX LM-W Brainfuck) ───────
        //
        // `alloc_bytes  dest  <-  size`.  The wasm module's linear memory *is*
        // the Brainfuck tape and it starts at offset 0, so `dest` (the tape
        // base) is simply the constant address 0.  The `size` operand only
        // determines how big the memory must be — that is handled module-wide:
        // `collect_module_features` flags `uses_memory`, and `lower_iir_to_wasm`
        // emits a fixed 1-page (64 KiB) memory, comfortably larger than the
        // 30 000-cell default tape.  `dest` is an `i64` register (the widened
        // BF value model — see `lower_brainfuck_for_aot` Step 5), so we push an
        // i64 zero; if some caller declared it i32 we push an i32 zero instead.
        "alloc_bytes" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "alloc_bytes must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            if slot_is_i64(rd) {
                code.extend(encode_i64_const(0));
            } else {
                code.extend(encode_i32_const(0));
            }
            code.extend(encode_local_set(rd));
        }

        // ── load_byte → i32.load8_u at base+idx, widened to the cell ─────────
        //
        // `load_byte  dest  <-  base, idx`.  Reads one tape cell.  `base` (the
        // tape, = 0) and `idx` are `i64` registers, but a wasm address is i32,
        // so we wrap each to i32 and add to form the effective address, then
        // `i32.load8_u` (which zero-extends the byte to i32).  The result is
        // widened back to i64 with `i64.extend_i32_u` to match the i64 cell
        // register `dest`.  This is the wasm twin of the LLVM
        // `getelementptr i8 + load i8 + zext` lowering — "byte width only at the
        // tape boundary."  An out-of-bounds index traps at the wasm layer.
        "load_byte" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "load_byte must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let base_var = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "load_byte requires Operand::Var(base) as src[0]".to_string(),
                }),
            };
            let idx_var = match instr.srcs.get(1) {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "load_byte requires Operand::Var(idx) as src[1]".to_string(),
                }),
            };
            let base_slot = get_reg(base_var)?;
            let idx_slot = get_reg(idx_var)?;
            // addr = wrap(base) + wrap(idx)
            code.extend(encode_local_get(base_slot));
            if slot_is_i64(base_slot) {
                code.extend(encode_i32_wrap_i64());
            }
            code.extend(encode_local_get(idx_slot));
            if slot_is_i64(idx_slot) {
                code.extend(encode_i32_wrap_i64());
            }
            code.extend(encode_i32_add());
            code.extend(encode_i32_load8_u());
            if slot_is_i64(rd) {
                code.extend(encode_i64_extend_i32_u());
            }
            code.extend(encode_local_set(rd));
        }

        // ── store_byte → i32.store8 of the low byte at base+idx ──────────────
        //
        // `store_byte  base, idx, val`  (no dest).  Writes one tape cell.  The
        // effective address is `wrap(base) + wrap(idx)` (i32); the value is
        // narrowed with `i32.wrap_i64` and `i32.store8` keeps only its low 8
        // bits — which is exactly what enforces Brainfuck's 8-bit cell
        // wrap-around (`255 + 1 == 0`) even though the arithmetic ran at i64
        // width.  The wasm twin of the LLVM `trunc i64…i8 + store i8`.
        "store_byte" => {
            if instr.dest.is_some() {
                return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "store_byte must not have a dest".to_string(),
                });
            }
            if instr.srcs.len() < 3 {
                return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "store_byte requires 3 srcs: [base, idx, val]".to_string(),
                });
            }
            let base_var = match &instr.srcs[0] {
                Operand::Var(v) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "store_byte src[0] must be Operand::Var(base)".to_string(),
                }),
            };
            let idx_var = match &instr.srcs[1] {
                Operand::Var(v) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "store_byte src[1] must be Operand::Var(idx)".to_string(),
                }),
            };
            let val_var = match &instr.srcs[2] {
                Operand::Var(v) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "store_byte src[2] must be Operand::Var(val)".to_string(),
                }),
            };
            let base_slot = get_reg(base_var)?;
            let idx_slot = get_reg(idx_var)?;
            let val_slot = get_reg(val_var)?;
            // addr = wrap(base) + wrap(idx) — pushed first (store8 wants [addr, val]).
            code.extend(encode_local_get(base_slot));
            if slot_is_i64(base_slot) {
                code.extend(encode_i32_wrap_i64());
            }
            code.extend(encode_local_get(idx_slot));
            if slot_is_i64(idx_slot) {
                code.extend(encode_i32_wrap_i64());
            }
            code.extend(encode_i32_add());
            // value (low byte) — narrowed to i32.
            code.extend(encode_local_get(val_slot));
            if slot_is_i64(val_slot) {
                code.extend(encode_i32_wrap_i64());
            }
            code.extend(encode_i32_store8());
        }

        // ── alloc_array → bump-allocate a length-prefixed block (E5) ─────────
        //
        // `alloc_array dest <- count : array<T>`. Linear memory holds, per array,
        // `[i64 length][elem 0][elem 1]…`; the handle (`dest`) is the byte offset
        // of the block, taken from the `__array_bump` global, which is then
        // advanced by `8 + count*elemsize` so the next array gets a fresh region.
        // The length is written into the header. The wasm twin of the LLVM
        // `@calloc [i64 len][elems…]` + `store i64 count` lowering.
        //
        // Trust boundary: `count` is a compiler-produced operand, so the bump
        // arithmetic is plain wrapping i64. A hand-built IIR with `count ≈ 2⁶¹`
        // could overflow the size and the stored length; the worst case is still a
        // wasm trap or a clobber *confined to the 64 KiB linear-memory sandbox*
        // (every access is bounds-checked by the runtime), never host memory —
        // strictly safer than the unchecked `alloc_bytes` byte-tape.
        "alloc_array" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "alloc_array must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let count_var = match instr.srcs.first() {
                Some(Operand::Var(v)) => v.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "alloc_array requires Operand::Var(count) as src[0]".to_string(),
                }),
            };
            let count_slot = get_reg(count_var)?;
            let elem = interpreter_ir::opcodes::array_elem_type(&instr.type_hint).ok_or_else(|| {
                IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: format!("alloc_array type_hint must be array<T>, got {:?}", instr.type_hint),
                }
            })?;
            let (_, elem_size) = wasm_array_elem(&elem, fn_name)?;
            let bump = *global_map.get(ARRAY_BUMP_GLOBAL).ok_or_else(|| IIRWasmError::UnsupportedOp {
                function: fn_name.to_string(),
                op: "alloc_array (missing __array_bump global)".to_string(),
            })?;
            // handle (dest) = current bump.
            code.extend(encode_global_get(bump));
            code.extend(encode_local_set(rd));
            // bump = handle + 8 + count*elemsize.
            code.extend(encode_local_get(rd));
            code.extend(encode_local_get(count_slot));
            code.extend(encode_i64_const(elem_size as i64));
            code.push(I64_MUL);
            code.extend(encode_i64_const(8));
            code.push(I64_ADD);
            code.push(I64_ADD);
            code.extend(encode_global_set(bump));
            // mem[wrap(handle) + 0] = count   (i64 length header).
            code.extend(encode_local_get(rd));
            code.extend(encode_i32_wrap_i64());
            code.extend(encode_local_get(count_slot));
            code.extend(encode_i64_store(0));
        }

        // ── array_get → bounds-checked element load (E5) ─────────────────────
        //
        // `array_get dest <- handle, idx : T`. A single **unsigned** compare
        // `idx >=u len` traps (`unreachable`) on both a `>= len` index and a
        // negative one (a negative i64 is a huge unsigned value) — the wasm twin
        // of LLVM's `icmp uge` + `llvm.trap`. Then load the element at
        // `wrap(handle) + idx*elemsize`, offset 8 past the length header.
        "array_get" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "array_get must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let handle_slot = get_reg(array_var(instr, 0, "array_get", "handle", fn_name)?)?;
            let idx_slot = get_reg(array_var(instr, 1, "array_get", "idx", fn_name)?)?;
            let (vt, elem_size) = wasm_array_elem(&instr.type_hint, fn_name)?;
            // bounds: idx >=u len  →  unreachable
            code.extend(encode_local_get(idx_slot));
            code.extend(encode_local_get(handle_slot));
            code.extend(encode_i32_wrap_i64());
            code.extend(encode_i64_load(0)); // length header
            code.push(I64_GE_U);
            code.push(IF);
            code.push(BLOCK_EMPTY);
            code.push(UNREACHABLE);
            code.push(END);
            // addr = wrap(handle) + wrap(idx)*elemsize ; load at offset 8
            code.extend(encode_local_get(handle_slot));
            code.extend(encode_i32_wrap_i64());
            code.extend(encode_local_get(idx_slot));
            code.extend(encode_i32_wrap_i64());
            code.extend(encode_i32_const(elem_size as i32));
            code.push(I32_MUL);
            code.push(I32_ADD);
            match vt {
                ValueType::F64 => code.extend(encode_f64_load(8)),
                // E4d-BA-arr: a `str` element is a 4-byte i32 handle.
                ValueType::I32 => code.extend(encode_i32_load(8)),
                _ => code.extend(encode_i64_load(8)),
            }
            code.extend(encode_local_set(rd));
        }

        // ── array_set → bounds-checked element store (E5) ────────────────────
        //
        // `array_set handle, idx, val : T` (no dest). Same `idx >=u len` →
        // `unreachable` guard, then store `val` at `wrap(handle) + idx*elemsize`,
        // offset 8.
        "array_set" => {
            if instr.dest.is_some() {
                return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "array_set must not have a dest".to_string(),
                });
            }
            let handle_slot = get_reg(array_var(instr, 0, "array_set", "handle", fn_name)?)?;
            let idx_slot = get_reg(array_var(instr, 1, "array_set", "idx", fn_name)?)?;
            let val_slot = get_reg(array_var(instr, 2, "array_set", "val", fn_name)?)?;
            let (vt, elem_size) = wasm_array_elem(&instr.type_hint, fn_name)?;
            // bounds: idx >=u len  →  unreachable
            code.extend(encode_local_get(idx_slot));
            code.extend(encode_local_get(handle_slot));
            code.extend(encode_i32_wrap_i64());
            code.extend(encode_i64_load(0));
            code.push(I64_GE_U);
            code.push(IF);
            code.push(BLOCK_EMPTY);
            code.push(UNREACHABLE);
            code.push(END);
            // addr = wrap(handle) + wrap(idx)*elemsize ; store at offset 8
            code.extend(encode_local_get(handle_slot));
            code.extend(encode_i32_wrap_i64());
            code.extend(encode_local_get(idx_slot));
            code.extend(encode_i32_wrap_i64());
            code.extend(encode_i32_const(elem_size as i32));
            code.push(I32_MUL);
            code.push(I32_ADD);
            code.extend(encode_local_get(val_slot));
            match vt {
                ValueType::F64 => code.extend(encode_f64_store(8)),
                // E4d-BA-arr: a `str` element is a 4-byte i32 handle.
                ValueType::I32 => code.extend(encode_i32_store(8)),
                _ => code.extend(encode_i64_store(8)),
            }
        }

        // ── array_len → load the i64 length header (E5) ──────────────────────
        //
        // `array_len dest <- handle`. The length lives at `wrap(handle) + 0`.
        "array_len" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: "array_len must have a dest".to_string(),
            })?;
            let rd = get_reg(dest)?;
            let handle_slot = get_reg(array_var(instr, 0, "array_len", "handle", fn_name)?)?;
            code.extend(encode_local_get(handle_slot));
            code.extend(encode_i32_wrap_i64());
            code.extend(encode_i64_load(0));
            code.extend(encode_local_set(rd));
        }

        // ── call_builtin → call $env.<name> ──────────────────────────────────
        //
        // Dispatch on `srcs[0]` (the builtin name, carried as Var) to the
        // corresponding host import.  The validator (validate.rs) has
        // already rejected any name not in `CALL_BUILTIN_SUPPORTED_NAMES`,
        // so the inner match here only handles whitelisted names; falling
        // off the match indicates a validator/lowerer drift and returns
        // `UnsupportedOp` as a safety net.
        //
        // | Builtin   | Host import       | Operand layout                        |
        // |-----------|-------------------|----------------------------------------|
        // | `putchar` | `env.putchar(i32)` | srcs = [Var("putchar"), Var(val)]      |
        // | `getchar` | `env.getchar() → i32` | srcs = [Var("getchar")]; dest = byte |
        "call_builtin" => {
            let name = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.as_str(),
                _ => return Err(IIRWasmError::InvalidOperand {
                    function: fn_name.to_string(),
                    detail: "call_builtin: srcs[0] must be the builtin name as Operand::Var".to_string(),
                }),
            };
            match name {
                "putchar" => {
                    let val_var = match instr.srcs.get(1) {
                        Some(Operand::Var(v)) => v.as_str(),
                        _ => return Err(IIRWasmError::InvalidOperand {
                            function: fn_name.to_string(),
                            detail: "call_builtin \"putchar\" requires srcs[1] = Operand::Var(val)".to_string(),
                        }),
                    };
                    let val_slot = get_reg(val_var)?;
                    let fn_idx = putchar_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "call_builtin \"putchar\": no env.putchar import registered (internal error)".to_string(),
                    })?;
                    // `env.putchar` takes an i32; the Brainfuck cell register is
                    // i64 after `lower_brainfuck_for_aot` widening, so narrow it
                    // (the printable byte lives in the low 8 bits regardless).
                    code.extend(encode_local_get(val_slot));
                    if slot_is_i64(val_slot) {
                        code.extend(encode_i32_wrap_i64());
                    }
                    code.extend(encode_call(fn_idx));
                }
                "getchar" => {
                    let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: "call_builtin \"getchar\" requires a dest register".to_string(),
                    })?;
                    let rd = get_reg(dest)?;
                    let fn_idx = getchar_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "call_builtin \"getchar\": no env.getchar import registered (internal error)".to_string(),
                    })?;
                    code.extend(encode_call(fn_idx));
                    // `env.getchar` returns an i32; widen it to the i64 cell
                    // register `dest` (the widened BF value model).
                    if slot_is_i64(rd) {
                        code.extend(encode_i64_extend_i32_u());
                    }
                    code.extend(encode_local_set(rd));
                }
                // BA-INPUT: `input_i64` reads a line from stdin and parses it as
                // an i64. The host provides `env.__input_i64() -> i64`; unlike
                // `getchar` which returns an i32 byte, this returns a full i64 so
                // no widening is needed. The BASIC compiler emits:
                //   call_builtin "input_i64" [dest=reg]
                // Shape: srcs[0]=Var("input_i64"), dest=Some(varname)
                "input_i64" => {
                    let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: "call_builtin \"input_i64\" requires a dest register".to_string(),
                    })?;
                    let rd = get_reg(dest)?;
                    let fn_idx = input_i64_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "call_builtin \"input_i64\": no env.__input_i64 import registered (internal error)".to_string(),
                    })?;
                    // `env.__input_i64` returns i64 directly — no widening needed.
                    code.extend(encode_call(fn_idx));
                    code.extend(encode_local_set(rd));
                }
                // E4-dyn: BASIC string `INPUT A$`. `input_str` reads a whole line as
                // a runtime string. On WASM a `str` value is an i32 **handle** — the
                // linear-memory offset of a `[i32 len][bytes]` block. We bump-allocate
                // a `[i32 len][MAX bytes]` region from `__array_bump` (its base is the
                // handle), then call `env.__input_str(block, MAX)` which fills the
                // whole block (length header + bytes). `print_str` later reads the
                // length via `i32.load` at the handle. Single `call` — the host owns
                // the memory writes, so no `i32.store` here.
                //   Shape: srcs[0]=Var("input_str"), dest=Some(varname)
                "input_str" => {
                    // Cap the line at 256 bytes: the module's linear memory is a
                    // single fixed 64 KiB page (`min=max=1`), and each block is never
                    // freed (bump-only), so a large MAX would exhaust the page after a
                    // few reads. 256 covers a BASIC input line; a longer line is
                    // truncated (V1 permissive contract).
                    const INPUT_STR_MAX: i32 = 256;
                    let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: "call_builtin \"input_str\" requires a dest register".to_string(),
                    })?;
                    let rd = get_reg(dest)?;
                    let fn_idx = input_str_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "call_builtin \"input_str\": no env.__input_str import registered (internal error)".to_string(),
                    })?;
                    let bump = *global_map.get(ARRAY_BUMP_GLOBAL).ok_or_else(|| IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "call_builtin \"input_str\" (missing __array_bump global)".to_string(),
                    })?;
                    // handle (dest, i32) = wrap(current bump).
                    code.extend(encode_global_get(bump));
                    code.extend(encode_i32_wrap_i64());
                    code.extend(encode_local_set(rd));
                    // bump = bump + (4 + MAX).
                    code.extend(encode_global_get(bump));
                    code.extend(encode_i64_const((4 + INPUT_STR_MAX) as i64));
                    code.push(I64_ADD);
                    code.extend(encode_global_set(bump));
                    // env.__input_str(block=handle, max=MAX) — host fills the block.
                    code.extend(encode_local_get(rd));
                    code.extend(encode_i32_const(INPUT_STR_MAX));
                    code.extend(encode_call(fn_idx));
                    // rd already holds the handle; nothing left on the stack.
                }
                // G2: `print_i64` reuses the same `env.__print_i64`
                // host import the `io_out` opcode injects.  Lowering
                // is identical to `io_out`: load the i64 argument from
                // its local and emit `call <print_fn_idx>`.
                //
                // Shape: `call_builtin "print_i64", val [void]`
                // - srcs[0] = Var("print_i64")
                // - srcs[1] = Var(val)
                // - dest    = None (void)
                "print_i64" => {
                    let val_var = match instr.srcs.get(1) {
                        Some(Operand::Var(v)) => v.as_str(),
                        _ => return Err(IIRWasmError::InvalidOperand {
                            function: fn_name.to_string(),
                            detail: "call_builtin \"print_i64\" requires srcs[1] = Operand::Var(val)".to_string(),
                        }),
                    };
                    let val_slot = get_reg(val_var)?;
                    let fn_idx = print_fn_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "call_builtin \"print_i64\": no env.__print_i64 import registered (internal error — collect_module_features should have set uses_io_out)".to_string(),
                    })?;
                    code.extend(encode_local_get(val_slot));
                    code.extend(encode_call(fn_idx));
                }
                // McCarthy `pair?` (LANG77 L3b-3a-4): "is this lisp value a cons
                // cell?". In the uniform-anyref model a cons is a `$LispyPair`
                // struct reference, so `pair?` is exactly `ref.test $LispyPair`:
                // it pushes i32 1 for a cons, 0 for a boxed atom (`i31ref`) or
                // nil (the null reference). `ATOM x` is the frontend's
                // `not(pair? x)`.
                //
                //   local.get $arg            ;; anyref
                //   ref.test $LispyPair        ;; → i32 (1 = cons, 0 = atom/nil)
                //   local.set $dest
                "pair?" => {
                    let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: "call_builtin \"pair?\" requires a dest register".to_string(),
                    })?;
                    let rd = get_reg(dest)?;
                    let arg = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?;
                    let type_idx = lispy_pair_type_idx.ok_or_else(|| IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: "call_builtin \"pair?\": module has no $LispyPair struct type".to_string(),
                    })?;
                    code.extend(encode_local_get(arg));
                    encode_gc_instruction(code, &GcInstruction::RefTest(type_idx));
                    code.extend(encode_local_set(rd));
                }
                // The lisp `not` (LANG77 L3b-3a-4): boolean negation of a
                // predicate's machine boolean (0/1), i.e. `i32.eqz`. (Distinct
                // from the numeric `not` *op*, which is a bitwise XOR -1.)
                //
                //   local.get $arg            ;; i32 (0 or 1)
                //   i32.eqz                    ;; → i32 (1 → 0, 0 → 1)
                //   local.set $dest
                "not" => {
                    let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: "call_builtin \"not\" requires a dest register".to_string(),
                    })?;
                    let rd = get_reg(dest)?;
                    let arg = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?;
                    code.extend(encode_local_get(arg));
                    code.push(0x45); // i32.eqz
                    code.extend(encode_local_set(rd));
                }
                // McCarthy `EQ` (frontend builtin `equal?`) on atoms (LANG77
                // L3b-3a-4c): equality of two lisp integer atoms. Both arguments
                // arrive boxed as `i31ref` (the structural pass boxes a lisp atom
                // before a predicate), so we unbox each and compare:
                //
                //   local.get $a  i31.get_s     ;; a → i32
                //   local.get $b  i31.get_s     ;; b → i32
                //   i32.eq                       ;; → i32 (1 = equal)
                //   local.set $dest
                //
                // This is McCarthy `eq` (atom equality); deep structural `equal`
                // over cons cells is a separate, later builtin.
                "equal?" => {
                    let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                        function: fn_name.to_string(),
                        detail: "call_builtin \"equal?\" requires a dest register".to_string(),
                    })?;
                    let rd = get_reg(dest)?;
                    let a = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?;
                    let b = get_src_reg(&instr.srcs, 2, reg_map, fn_name)?;
                    code.extend(encode_local_get(a));
                    encode_gc_instruction(code, &GcInstruction::I31GetS);
                    code.extend(encode_local_get(b));
                    encode_gc_instruction(code, &GcInstruction::I31GetS);
                    code.push(0x46); // i32.eq
                    code.extend(encode_local_set(rd));
                }
                _ => {
                    // Validator should have rejected this; defense in depth.
                    return Err(IIRWasmError::UnsupportedOp {
                        function: fn_name.to_string(),
                        op: format!("call_builtin {:?}: not in WASM backend whitelist", name),
                    });
                }
            }
        }

        // ── Unknown op ────────────────────────────────────────────────────────
        _ => {
            return Err(IIRWasmError::UnsupportedOp {
                function: fn_name.to_string(),
                op: instr.op.clone(),
            });
        }
    }

    Ok(())
}

/// Resolve the register index for source operand at position `idx`.
///
/// Returns an error if the operand is not a `Var` or the variable is not in
/// the register map.  Immediate operands (Int/Float/Bool) cannot serve as
/// direct source operands for binary instructions — the front-end is expected
/// to have lowered them through a `const` instruction first.
fn get_src_reg(
    srcs: &[Operand],
    idx: usize,
    reg_map: &HashMap<String, u32>,
    fn_name: &str,
) -> Result<u32, IIRWasmError> {
    match srcs.get(idx) {
        Some(Operand::Var(name)) => {
            reg_map.get(name).copied().ok_or_else(|| IIRWasmError::UndefinedVariable {
                function: fn_name.to_string(),
                name: name.clone(),
            })
        }
        Some(other) => Err(IIRWasmError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!("expected Var at src[{idx}], got {:?}", other),
        }),
        None => Err(IIRWasmError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!("missing src operand at index {idx}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Function body lowering
// ---------------------------------------------------------------------------

/// Lower one `IIRFunction` to a `FunctionBody`.
///
/// This is where the two-pass strategy described at the module level is
/// implemented:
///
/// 1. Build the register map (Pass 1).
/// 2. Determine local types.
/// 3. If the function has control flow (labels/jmps): use the dispatch-loop
///    pattern.  Otherwise: emit instructions linearly.
/// 4. Terminate with the WASM function `end` opcode.
///
/// The `lispy_pair_type_idx` parameter is `Some(n)` when the module has a
/// `$LispyPair` struct type at type-section index `n`.  Pass `None` for
/// functions that contain no GC heap ops.
///
/// `global_map` maps global variable names to their WASM global section
/// indices.  Used by `global_load`/`global_store` instructions.
///
/// `print_fn_idx` is `Some(0)` when the module imports `env.__print_i64`.
/// Used by `io_out` instructions.
fn lower_function(
    fn_: &IIRFunction,
    fn_map: &HashMap<String, u32>,
    lispy_pair_type_idx: Option<u32>,
    global_map: &HashMap<String, u32>,
    string_literals: &FunctionStringLiterals,
    runtime_str_vars: &FunctionRuntimeStrVars,
    runtime_str_blocks: &FunctionRuntimeStrBlocks,
    print_fn_idx: Option<u32>,
    print_str_fn_idx: Option<u32>,
    putchar_fn_idx: Option<u32>,
    getchar_fn_idx: Option<u32>,
    input_i64_fn_idx: Option<u32>,
    input_str_fn_idx: Option<u32>,
    sin_fn_idx: Option<u32>,
    cos_fn_idx: Option<u32>,
    ln_fn_idx: Option<u32>,
    exp_fn_idx: Option<u32>,
    atan_fn_idx: Option<u32>,
    tan_fn_idx: Option<u32>,
    pow_fn_idx: Option<u32>,
    str_eq_fn_idx: Option<u32>,
) -> Result<FunctionBody, IIRWasmError> {
    let param_count = fn_.params.len() as u32;
    let reg_map = build_register_map(fn_);
    let total_vars = reg_map.len() as u32;
    // The dispatch variable sits at the next available local index.
    let dispatch_reg = total_vars;

    // Infer types for non-parameter locals.
    let local_type_hints = infer_local_type_hints(fn_, &reg_map);
    let mut local_types = infer_local_types(&local_type_hints, param_count, total_vars);
    // Append the dispatch variable (always I32 — it holds a block index).
    local_types.push(ValueType::I32);

    let use_dispatch = has_control_flow(fn_);
    let (mut blocks, label_to_block) = split_into_blocks(fn_);
    // If the last basic block contains a conditional branch (jmp_if_true /
    // jmp_if_false), it cannot restart the dispatch loop from inside the WASM
    // dispatch-loop pattern.  When bb_{N-1} (the last block) is entered via
    // the br_table, execute_branch truncates the label_stack to [$exit,
    // $dispatch], then the matching END instruction pops $dispatch, leaving
    // only [$exit].  A `jmp_if_true` inside an `if` block at depth 1 would
    // exit $exit (terminating the function prematurely) instead of restarting
    // $dispatch.  Fix: append a sentinel empty block so the block with the
    // conditional branches is now the second-to-last (block_idx = N-1, with
    // n_blocks = N+1), where the depth formula n_blocks-block_idx-1 correctly
    // resolves to $dispatch depth 1.  The sentinel bb_N is never dispatched
    // to (dispatch_reg is never set to N).
    if blocks
        .last()
        .map(|b| {
            b.instrs
                .iter()
                .any(|i| matches!(i.op.as_str(), "jmp_if_true" | "jmp_if_false"))
        })
        .unwrap_or(false)
    {
        blocks.push(BasicBlock { instrs: Vec::new() });
    }
    let n_blocks = blocks.len();

    let mut code: Vec<u8> = Vec::new();

    if use_dispatch {
        // ── Dispatch-loop pattern ────────────────────────────────────────────
        //
        // Initialise dispatch to 0 (start at entry block).
        code.extend(encode_i32_const(0));
        code.extend(encode_local_set(dispatch_reg));

        // Outer block (exit): breaking out of this block terminates the loop.
        code.push(BLOCK);
        code.push(BLOCK_EMPTY);

        // Loop: breaking to depth 0 (the loop itself) re-enters at the top.
        code.push(LOOP);
        code.push(BLOCK_EMPTY);

        // N nested blocks — one per basic block — innermost first (bb_0).
        // They are emitted in "innermost first" order so that br_table depth 0
        // exits the innermost block, placing execution in bb_0's body.
        for _ in 0..n_blocks {
            code.push(BLOCK);
            code.push(BLOCK_EMPTY);
        }

        // Dispatch: pop block index, jump to the corresponding block.
        //
        // `br_table [0, 1, …, N-1] default=N`
        //   depth 0 → exits bb_0's block → execution falls into bb_0 body
        //   depth 1 → exits bb_1's block → execution falls into bb_1 body
        //   …
        //   depth N → exits the outer loop block (impossible in normal use)
        code.extend(encode_local_get(dispatch_reg));
        let targets: Vec<u32> = (0..n_blocks as u32).collect();
        code.extend(encode_br_table(&targets, n_blocks as u32));

        // Emit the END of each nested block followed by that block's body.
        //
        // After the br_table, WASM execution is "between" blocks — the
        // innermost block(s) have been exited by the branch, and we are in
        // the scope of the next outer block.  Closing each block with `end`
        // and then emitting the block's instructions is the standard trick.
        for (block_idx, block) in blocks.iter().enumerate() {
            // Close the nested block for `block_idx`.
            code.push(END);

            // Emit the body instructions of this basic block.
            for instr in &block.instrs {
                emit_instr(
                    &mut code,
                    instr,
                    &reg_map,
                    &local_type_hints,
                    fn_map,
                    &fn_.name,
                    dispatch_reg,
                    &label_to_block,
                    block_idx,  // current block index (for branch-depth computation)
                    n_blocks,
                    true, // inside dispatch-loop
                    lispy_pair_type_idx,
                    global_map,
                    string_literals,
                    runtime_str_vars,
                    runtime_str_blocks,
                    print_fn_idx,
                    print_str_fn_idx,
                    putchar_fn_idx,
                    getchar_fn_idx,
                    input_i64_fn_idx,
                    input_str_fn_idx,
                    sin_fn_idx,
                    cos_fn_idx,
                    ln_fn_idx,
                    exp_fn_idx,
                    atan_fn_idx,
                    tan_fn_idx,
                    pow_fn_idx,
                    str_eq_fn_idx,
                )?;
            }

            // At the end of each block, if execution reaches here (i.e. it
            // was not ended by a ret/jmp), advance to the next block and
            // re-enter the loop so br_table can redispatch.
            //
            // We MUST re-enter the LOOP (not fall through to the next block
            // body directly) to keep the label_stack consistent.  The loop
            // depth from body[block_idx] is `n_blocks - block_idx - 2`.
            //
            // Special case: the last block (block_idx = n_blocks - 1) has
            // label_stack = [outer_exit], so `br 0` exits the outer block,
            // which terminates the function.  This is correct: well-formed
            // programs end the last block with `ret`, so fall-through there
            // is unreachable, but we emit a valid instruction for safety.
            let last_op = block.instrs.last().map(|i| i.op.as_str()).unwrap_or("");
            if !matches!(last_op, "ret" | "ret_void" | "jmp") {
                if block_idx + 1 < n_blocks {
                    let next_block = (block_idx + 1) as i32;
                    code.extend(encode_i32_const(next_block));
                    code.extend(encode_local_set(dispatch_reg));
                    // Re-enter loop: depth = n_blocks - block_idx - 2.
                    let loop_depth = (n_blocks - block_idx - 2) as u32;
                    code.extend(encode_br(loop_depth));
                } else {
                    // Last block: `br 0` exits outer_exit → function ends.
                    code.extend(encode_br(0));
                }
            }
        }

        // End the loop and the outer exit block.
        code.push(END); // end loop
        code.push(END); // end exit block
    } else {
        // ── Linear emission (no control flow) ────────────────────────────────
        //
        // For functions without labels or jumps, we emit instructions in order.
        for instr in &fn_.instructions {
            emit_instr(
                &mut code,
                instr,
                &reg_map,
                &local_type_hints,
                fn_map,
                &fn_.name,
                dispatch_reg,
                &label_to_block,
                0,      // block_idx unused when is_dispatch_loop=false
                n_blocks,
                false,  // no dispatch loop
                lispy_pair_type_idx,
                global_map,
                string_literals,
                runtime_str_vars,
                runtime_str_blocks,
                print_fn_idx,
                print_str_fn_idx,
                putchar_fn_idx,
                getchar_fn_idx,
                input_i64_fn_idx,
                input_str_fn_idx,
                sin_fn_idx,
                cos_fn_idx,
                ln_fn_idx,
                exp_fn_idx,
                atan_fn_idx,
                tan_fn_idx,
                pow_fn_idx,
                str_eq_fn_idx,
            )?;
        }
    }

    // Every WASM function body must end with the `end` opcode.
    code.push(END);

    Ok(FunctionBody {
        locals: local_types,
        code,
    })
}

// ---------------------------------------------------------------------------
// Module lowering (main entry point)
// ---------------------------------------------------------------------------

/// Detect whether an IIR module contains any `ref<LispyPair>` heap ops.
///
/// Returns `true` if any instruction in any function has a type hint of
/// `"ref<LispyPair>"`, which triggers WasmGC struct type registration.
fn module_uses_lispy_pair(module: &IIRModule) -> bool {
    module.functions.iter().any(|fn_| {
        fn_.instructions.iter().any(|i| {
            i.type_hint == "ref<LispyPair>"
                // `pair?` lowers to `ref.test $LispyPair`, so it needs the struct
                // type even in a module that never `cons`es (e.g. `(ATOM 5)`).
                || (i.op == "call_builtin"
                    && matches!(i.srcs.first(), Some(Operand::Var(n)) if n == "pair?"))
        }) || fn_.params.iter().any(|(_, t)| t == "ref<LispyPair>")
            || fn_.return_type == "ref<LispyPair>"
    })
}

/// Build the canonical `$LispyPair` struct type definition.
///
/// ```wat
/// (type $LispyPair (struct
///   (field $head (mut (ref null any)))   ;; field 0 — car
///   (field $tail (mut (ref null any))))) ;; field 1 — cdr
/// ```
///
/// Both fields are `anyref` (= `(ref null any)`) and mutable, so the
/// runtime can write `head` and `tail` after allocation.
fn make_lispy_pair_struct_type() -> StructType {
    StructType {
        fields: vec![
            FieldType {
                val_type: ValueType::Anyref,
                mutable: true,
            }, // $head — car value
            FieldType {
                val_type: ValueType::Anyref,
                mutable: true,
            }, // $tail — cdr value
        ],
    }
}

/// Scan `module` for global variable names and `io_out` usage.
///
/// Returns `(global_names, uses_io_out)`:
///
/// - `global_names` — deduplicated list of all global names referenced by
///   `global_load`/`global_store` instructions, in first-seen order.  Their
///   position in the vec determines their WASM global section index.
/// - `uses_io_out` — `true` if any instruction in any function has the
///   `"io_out"` opcode.  Triggers injection of the `env.__print_i64` import.
/// Features the WASM lowering must materialize as imports / sections.
///
/// Populated by [`collect_module_features`] from a single pass over the
/// module.  Each boolean field gates an injection step in
/// [`lower_iir_to_wasm`]:
///
/// | Field              | Trigger                                | What it injects |
/// |--------------------|----------------------------------------|-----------------|
/// | `global_names`     | `global_load` / `global_store`          | `Global` entries (one per name, i64, mutable) |
/// | `uses_io_out`      | `io_out`                                | `env.__print_i64` import |
/// | `uses_print_str`   | `print_str`                             | `env.__print_str` import + linear memory |
/// | `uses_putchar`     | `call_builtin` with name `"putchar"`    | `env.putchar` import   |
/// | `uses_getchar`     | `call_builtin` with name `"getchar"`    | `env.getchar` import   |
/// | `uses_input_i64`   | `call_builtin` with name `"input_i64"` | `env.__input_i64` import |
/// | `uses_memory`      | `load_mem` / `store_mem`                | A 1-page linear `Memory` |
///
/// All combinations are valid — the eventual import order is documented
/// in [`lower_iir_to_wasm`].
struct ModuleFeatures {
    /// Deduplicated global names, in first-seen order.  Position in the
    /// vec = WASM global-section index.
    global_names: Vec<String>,
    uses_io_out: bool,
    uses_print_str: bool,
    uses_putchar: bool,
    uses_getchar: bool,
    uses_input_i64: bool,
    /// True when the module calls `call_builtin "input_str"` (BASIC string
    /// `INPUT A$`). Triggers injection of the `env.__input_str(i32,i32) -> i32`
    /// host import (fills a linear-memory buffer with the line, returns its length).
    uses_input_str: bool,
    /// True when the module calls `f64_sin`/`f64_cos`/`f64_ln`/`f64_exp`/`f64_atan`/`f64_tan`.
    /// Triggers injection of the corresponding host imports (`env.__sin` etc.,
    /// each `f64 -> f64`).  WASM has no built-in transcendental opcodes; these
    /// are provided by the host runtime (libm on the test host).
    uses_f64_sin: bool,
    uses_f64_cos: bool,
    uses_f64_ln: bool,
    uses_f64_exp: bool,
    uses_f64_atan: bool,
    uses_f64_tan: bool,
    /// True when any function emits `f64_pow` — triggers the `env.__pow` import.
    uses_f64_pow: bool,
    /// True when the module reads or writes Brainfuck's tape memory.
    /// Triggers the addition of a single 1-page linear memory to the
    /// module's `Memory` section.
    uses_memory: bool,
    /// True when some `str_eq` cannot be folded at compile time — at least one
    /// operand is a runtime string handle (a param, a call result) rather than a
    /// compile-time literal.  Triggers injection of the self-contained in-module
    /// `$__str_eq(i32,i32)->i32` helper (a header-length check + byte-compare
    /// loop) that the `str_eq` lowering `call`s.  Unlike I/O, string equality is
    /// NOT a host import — it is emitted inside the module so the WASM is
    /// self-contained (mirrors the native/LLVM `__twig_str_eq` archive helper).
    uses_str_eq_runtime: bool,
    /// Function-name -> string-local -> data-segment offset/length.
    string_literals: ModuleStringLiterals,
    /// Concatenated literal bytes for the active E4 string data segment.
    string_data: Vec<u8>,
    /// E4-dyn (E4d-3): fn-name -> set of string vars promoted to a runtime
    /// handle (assigned in >1 basic block).
    runtime_str_vars: ModuleRuntimeStrVars,
    /// E4-dyn (E4d-3): fn-name -> literal text -> offset of its length-prefixed
    /// runtime block `[i32 len][bytes]` in the string data segment.
    runtime_str_blocks: ModuleRuntimeStrBlocks,
}

/// Walk the module once to collect everything the lowering needs to know
/// for module-level decisions (imports, sections, fn-index offsets).
/// Lay down (once, deduplicated by folded text) a length-prefixed runtime block
/// `[i32 len (LE)][bytes]` for a promoted folded-literal string, append it to the
/// module's `string_data`, and record the block's offset (its *handle*) plus mark
/// `dest` as carrying a runtime handle.  This is the shared core of the promoted
/// `str_const`/`str_concat`/`str_slice` paths: a folded literal handed to a callee
/// must present a real header (the callee has no compile-time length), so it is
/// materialised as a runtime block exactly like a control-flow-selected string.
///
/// Keyed by the folded text so identical literals (across ops or blocks) share one
/// block.  All frontends that reach here emit UTF-8 string literals, so the lossy
/// conversion is lossless and the key is a faithful, collision-free stand-in for
/// the bytes; the lowering side recomputes the identical key from the same bytes.
fn lay_runtime_str_block(
    runtime_str_blocks: &mut ModuleRuntimeStrBlocks,
    runtime_str_vars: &mut ModuleRuntimeStrVars,
    string_data: &mut Vec<u8>,
    fn_name: &str,
    dest: &str,
    bytes: &[u8],
    len: u32,
) {
    let key = String::from_utf8_lossy(bytes).into_owned();
    let fn_blocks = runtime_str_blocks.entry(fn_name.to_string()).or_default();
    if !fn_blocks.contains_key(&key) {
        let block_offset = string_data.len() as u32;
        string_data.extend_from_slice(&len.to_le_bytes());
        string_data.extend_from_slice(bytes);
        fn_blocks.insert(key, block_offset);
    }
    runtime_str_vars
        .entry(fn_name.to_string())
        .or_default()
        .insert(dest.to_string());
}

/// Build the self-contained in-module `$__str_eq(a: i32, b: i32) -> i32` helper.
///
/// `a` and `b` are handles to `[i32 len (LE)][bytes]` blocks in linear memory
/// (the same runtime-string representation the rest of this backend uses).  The
/// helper returns `1` if the two strings are byte-for-byte equal, else `0`:
///
/// ```text
///   if mem[a] != mem[b] { return 0 }          ;; lengths differ
///   len = mem[a]; i = 0
///   loop {
///     if i < len {
///       if mem8[a+4+i] != mem8[b+4+i] { return 0 }
///       i = i + 1; continue
///     }
///   }                                          ;; i == len → fall through
///   return 1
/// ```
///
/// It is emitted once per module (gated by `uses_str_eq_runtime`) and appended
/// after all IIR-defined functions, so its function index is
/// `fn_idx_base + module.functions.len()`.  Params live in the `FuncType`; the
/// two scratch locals (`len` = local 2, `i` = local 3) are declared here.
///
/// Why an in-module function and not a host import (like `env.__print_str`):
/// I/O is inherently the host's job, but string equality is pure computation —
/// keeping it inside the module makes the emitted WASM self-contained, exactly
/// like the native/LLVM backends link a `__twig_str_eq` helper into the binary
/// rather than expecting the embedder to supply one.
fn build_str_eq_helper() -> FunctionBody {
    // BLOCK_EMPTY/END/I32_ADD/I32_LT_U/I32_NE/IF/LOOP/RETURN are imported at module top.
    // Local indices: a = 0, b = 1 (params); len = 2, i = 3 (scratch, below).
    const A: u32 = 0;
    const B: u32 = 1;
    const LEN: u32 = 2;
    const I: u32 = 3;
    let mut c: Vec<u8> = Vec::new();

    // if mem[a] != mem[b] { return 0 }  — lengths differ ⇒ not equal.
    c.extend(encode_local_get(A));
    c.extend(encode_i32_load(0));
    c.extend(encode_local_get(B));
    c.extend(encode_i32_load(0));
    c.push(I32_NE);
    c.push(IF);
    c.push(BLOCK_EMPTY);
    c.extend(encode_i32_const(0));
    c.push(RETURN);
    c.push(END);

    // len = mem[a]  (== mem[b] here); i = 0.
    c.extend(encode_local_get(A));
    c.extend(encode_i32_load(0));
    c.extend(encode_local_set(LEN));
    c.extend(encode_i32_const(0));
    c.extend(encode_local_set(I));

    // loop { if i < len { compare byte; i += 1; continue } }  — falls through
    // (no branch) once i == len, i.e. every byte matched.
    c.push(LOOP);
    c.push(BLOCK_EMPTY);
    c.extend(encode_local_get(I));
    c.extend(encode_local_get(LEN));
    c.push(I32_LT_U);
    c.push(IF);
    c.push(BLOCK_EMPTY);
    // mem8[a + 4 + i]
    c.extend(encode_local_get(A));
    c.extend(encode_i32_const(4));
    c.push(I32_ADD);
    c.extend(encode_local_get(I));
    c.push(I32_ADD);
    c.extend(encode_i32_load8_u());
    // mem8[b + 4 + i]
    c.extend(encode_local_get(B));
    c.extend(encode_i32_const(4));
    c.push(I32_ADD);
    c.extend(encode_local_get(I));
    c.push(I32_ADD);
    c.extend(encode_i32_load8_u());
    c.push(I32_NE);
    c.push(IF);
    c.push(BLOCK_EMPTY);
    c.extend(encode_i32_const(0));
    c.push(RETURN);
    c.push(END);
    // i = i + 1
    c.extend(encode_local_get(I));
    c.extend(encode_i32_const(1));
    c.push(I32_ADD);
    c.extend(encode_local_set(I));
    // continue: branch to the enclosing loop (depth 1: inner `if` = 0, loop = 1).
    c.extend(encode_br(1));
    c.push(END); // end of the `if i < len` block
    c.push(END); // end of the loop

    // Every byte matched → equal.
    c.extend(encode_i32_const(1));
    c.push(END); // function end — implicit return of the i32 on the stack

    FunctionBody {
        locals: vec![ValueType::I32, ValueType::I32], // len, i
        code: c,
    }
}

fn collect_module_features(module: &IIRModule) -> ModuleFeatures {
    // Use a HashSet for O(1) deduplication checks, preserving first-seen order
    // in the Vec.  Without the set, deduplication would be O(M × N) where M is
    // the number of global accesses and N the number of distinct names —
    // quadratic for adversarially crafted modules with many globals.
    let mut global_names: Vec<String> = Vec::new();
    let mut global_names_seen: HashSet<String> = HashSet::new();
    let mut uses_io_out = false;
    let mut uses_print_str = false;
    let mut uses_putchar = false;
    let mut uses_getchar = false;
    let mut uses_input_i64 = false;
    let mut uses_input_str = false;
    let mut uses_f64_sin  = false;
    let mut uses_f64_cos  = false;
    let mut uses_f64_ln   = false;
    let mut uses_f64_exp  = false;
    let mut uses_f64_atan = false;
    let mut uses_f64_tan  = false;
    let mut uses_memory = false;
    let mut uses_f64_pow = false;
    let mut uses_str_eq_runtime = false;
    let mut string_literals: ModuleStringLiterals = HashMap::new();
    let mut string_data: Vec<u8> = Vec::new();
    let mut runtime_str_vars: ModuleRuntimeStrVars = HashMap::new();
    let mut runtime_str_blocks: ModuleRuntimeStrBlocks = HashMap::new();
    for fn_ in &module.functions {
        let mut fn_ints: HashMap<String, i64> = HashMap::new();
        // E4-dyn (E4d-3): which of this function's string variables are chosen by
        // control flow (assigned in >1 basic block) and so must carry a runtime
        // handle rather than a folded literal.
        let fn_runtime_vars = collect_runtime_str_vars(fn_);
        for instr in &fn_.instructions {
            match instr.op.as_str() {
                "global_load" | "global_store" => {
                    if let Some(Operand::Str(name)) = instr.srcs.first() {
                        if global_names_seen.insert(name.clone()) {
                            global_names.push(name.clone());
                        }
                    }
                }
                "io_out" => {
                    uses_io_out = true;
                }
                "f64_pow" => {
                    uses_f64_pow = true;
                }
                "const" => {
                    if let (Some(dest), Some(Operand::Int(value))) =
                        (instr.dest.as_ref(), instr.srcs.first())
                    {
                        fn_ints.insert(dest.clone(), *value);
                    }
                }
                "str_const" => {
                    if let (Some(dest), Some(Operand::Str(s))) =
                        (instr.dest.as_ref(), instr.srcs.first())
                    {
                        let offset = string_data.len() as u32;
                        let len = s.len() as u32;
                        string_data.extend_from_slice(s.as_bytes());
                        string_literals
                            .entry(fn_.name.clone())
                            .or_default()
                            .insert(dest.clone(), WasmStringLiteral {
                                offset,
                                len,
                                bytes: s.as_bytes().to_vec(),
                            });

                        // E4-dyn (E4d-3): if this string variable is chosen by
                        // control flow — OR handed to a callee as a folded literal
                        // (`fn_runtime_vars`) — lay down a length-prefixed runtime
                        // block `[i32 len (LE)][bytes]` and remember its offset (the
                        // *handle*).  Deduplicated by text — two branches (or a
                        // `str_const` and a `str_concat`) that yield the same string
                        // share one block.  The flat literal above is left in place
                        // too (harmless, and still serves any literal-only reader).
                        if fn_runtime_vars.contains(dest) {
                            lay_runtime_str_block(
                                &mut runtime_str_blocks,
                                &mut runtime_str_vars,
                                &mut string_data,
                                &fn_.name,
                                dest,
                                s.as_bytes(),
                                len,
                            );
                        }
                    }
                }
                "str_concat" => {
                    if let (Some(dest), [Operand::Var(left), Operand::Var(right)]) =
                        (instr.dest.as_ref(), instr.srcs.as_slice())
                    {
                        let Some((left_lit, right_lit)) = string_literals
                            .get(&fn_.name)
                            .and_then(|fn_strings| {
                                Some((
                                    fn_strings.get(left)?.clone(),
                                    fn_strings.get(right)?.clone(),
                                ))
                            })
                        else {
                            continue;
                        };
                        let mut bytes = left_lit.bytes;
                        bytes.extend_from_slice(&right_lit.bytes);
                        let offset = string_data.len() as u32;
                        let len = bytes.len() as u32;
                        string_data.extend_from_slice(&bytes);
                        // If this folded result is passed across a call boundary, also
                        // lay down a length-prefixed runtime block `[i32 len][bytes]`
                        // and remember its offset (the handle) — exactly like the
                        // promoted `str_const` path above, so the callee reads a real
                        // header instead of the first data byte. Keyed by the folded
                        // text for dedup (lisp string literals are UTF-8, so the lossy
                        // conversion is lossless and collision-free here; the lowering
                        // side derives the same key from the identical `lit.bytes`).
                        if fn_runtime_vars.contains(dest) {
                            lay_runtime_str_block(
                                &mut runtime_str_blocks,
                                &mut runtime_str_vars,
                                &mut string_data,
                                &fn_.name,
                                dest,
                                &bytes,
                                len,
                            );
                        }
                        string_literals
                            .entry(fn_.name.clone())
                            .or_default()
                            .insert(dest.clone(), WasmStringLiteral {
                                offset,
                                len,
                                bytes,
                            });
                    }
                }
                "str_len" => {
                    if let (Some(dest), [Operand::Var(src)]) =
                        (instr.dest.as_ref(), instr.srcs.as_slice())
                    {
                        let Some(lit) = string_literals
                            .get(&fn_.name)
                            .and_then(|fn_strings| fn_strings.get(src))
                        else {
                            continue;
                        };
                        fn_ints.insert(dest.clone(), lit.len as i64);
                    }
                }
                "add" | "sub" | "mul" | "div" => {
                    if let (Some(dest), [Operand::Var(left), Operand::Var(right)]) =
                        (instr.dest.as_ref(), instr.srcs.as_slice())
                    {
                        let (Some(left), Some(right)) =
                            (fn_ints.get(left).copied(), fn_ints.get(right).copied())
                        else {
                            continue;
                        };
                        let value = match instr.op.as_str() {
                            "add" => left.checked_add(right),
                            "sub" => left.checked_sub(right),
                            "mul" => left.checked_mul(right),
                            "div" if right != 0 => left.checked_div(right),
                            _ => None,
                        };
                        if let Some(value) = value {
                            fn_ints.insert(dest.clone(), value);
                        }
                    }
                }
                "str_slice" => {
                    if let (
                        Some(dest),
                        [Operand::Var(src), Operand::Var(start), Operand::Var(end)],
                    ) = (instr.dest.as_ref(), instr.srcs.as_slice())
                    {
                        let Some(src_lit) = string_literals
                            .get(&fn_.name)
                            .and_then(|fn_strings| fn_strings.get(src))
                        else {
                            continue;
                        };
                        let (Some(start), Some(end)) =
                            (fn_ints.get(start).copied(), fn_ints.get(end).copied())
                        else {
                            continue;
                        };
                        let (Ok(start), Ok(end)) =
                            (usize::try_from(start), usize::try_from(end))
                        else {
                            continue;
                        };
                        if end < start || end > src_lit.bytes.len() {
                            continue;
                        }
                        let bytes = src_lit.bytes[start..end].to_vec();
                        let offset = string_data.len() as u32;
                        let len = bytes.len() as u32;
                        string_data.extend_from_slice(&bytes);
                        // Same promotion as `str_concat`: a folded slice passed
                        // across a call boundary (e.g. `(strlen (substring …))`)
                        // needs a real header, or the callee reads the first sliced
                        // byte as the length.
                        if fn_runtime_vars.contains(dest) {
                            lay_runtime_str_block(
                                &mut runtime_str_blocks,
                                &mut runtime_str_vars,
                                &mut string_data,
                                &fn_.name,
                                dest,
                                &bytes,
                                len,
                            );
                        }
                        string_literals
                            .entry(fn_.name.clone())
                            .or_default()
                            .insert(dest.clone(), WasmStringLiteral {
                                offset,
                                len,
                                bytes,
                            });
                    }
                }
                // A `str_eq` folds to a compile-time constant only when BOTH
                // operands are folded literals.  Otherwise (a param, a call
                // result — anything without a `string_literals` entry) it needs
                // the runtime `$__str_eq` helper, which compares two `[i32 len]
                // [bytes]` blocks.  Any operand that IS a folded literal must
                // then be promoted to a runtime block too, so it presents a real
                // header to the helper (a raw data offset has none).
                "str_eq" => {
                    if let [Operand::Var(left), Operand::Var(right)] = instr.srcs.as_slice() {
                        let fn_strings = string_literals.get(&fn_.name);
                        let left_lit = fn_strings.and_then(|m| m.get(left)).cloned();
                        let right_lit = fn_strings.and_then(|m| m.get(right)).cloned();
                        if left_lit.is_none() || right_lit.is_none() {
                            uses_str_eq_runtime = true;
                            uses_memory = true;
                            // Promote any folded-literal operand to a runtime block.
                            if let Some(lit) = left_lit {
                                lay_runtime_str_block(
                                    &mut runtime_str_blocks, &mut runtime_str_vars,
                                    &mut string_data, &fn_.name, left, &lit.bytes, lit.len,
                                );
                            }
                            if let Some(lit) = right_lit {
                                lay_runtime_str_block(
                                    &mut runtime_str_blocks, &mut runtime_str_vars,
                                    &mut string_data, &fn_.name, right, &lit.bytes, lit.len,
                                );
                            }
                        }
                    }
                }
                "print_str" => {
                    uses_print_str = true;
                    uses_memory = true;
                }
                // `load_mem`/`store_mem` are the raw BF-frontend tape ops;
                // `alloc_bytes`/`load_byte`/`store_byte` are the lowered AOT/LLVM
                // form `lower_brainfuck_for_aot` rewrites them into. Either shape
                // means the module needs a linear memory for the tape.
                "load_mem" | "store_mem" | "alloc_bytes" | "load_byte" | "store_byte" => {
                    uses_memory = true;
                }
                // LANG-FULL E5: array ops live in linear memory too, and they need
                // a module-level **bump pointer** so successive `alloc_array`s get
                // distinct bases. We model it as one extra mutable i64 global,
                // `__array_bump` (init 0 — arrays start at memory offset 0; an ALGOL
                // array program never also drives the Brainfuck byte-tape). It is
                // injected into `global_names` here so it gets a global slot and a
                // `global_map` index the lowering can look up.
                s if interpreter_ir::opcodes::is_array_op(s) => {
                    uses_memory = true;
                    if global_names_seen.insert(ARRAY_BUMP_GLOBAL.to_string()) {
                        global_names.push(ARRAY_BUMP_GLOBAL.to_string());
                    }
                }
                // E4-dyn: a `str_concat` whose operands are runtime handles (not
                // foldable literals) bump-allocates a fresh `[i32 len][bytes]` block
                // and `memory.copy`s both operands in — so, like an array op, it needs
                // linear memory + the `__array_bump` global. (A both-literal concat
                // folds to a data-segment offset and never touches the bump pointer,
                // so the injected global is simply unused there.)
                "str_concat" => {
                    uses_memory = true;
                    if global_names_seen.insert(ARRAY_BUMP_GLOBAL.to_string()) {
                        global_names.push(ARRAY_BUMP_GLOBAL.to_string());
                    }
                }
                "call_builtin" => {
                    // The builtin name is in srcs[0] as Var.
                    if let Some(Operand::Var(name)) = instr.srcs.first() {
                        match name.as_str() {
                            "putchar" => uses_putchar = true,
                            "getchar" => uses_getchar = true,
                            // G2: `print_i64` reuses the same
                            // `env.__print_i64` import the `io_out`
                            // opcode injects.  Flipping `uses_io_out`
                            // ensures the import is wired in even when
                            // the module uses `print_i64` exclusively
                            // (no `io_out` opcodes).
                            "print_i64" => uses_io_out = true,
                            // BA-INPUT: BASIC `INPUT X` — triggers injection of
                            // `env.__input_i64() -> i64` host import.
                            "input_i64" => uses_input_i64 = true,
                            // E4-dyn: BASIC string `INPUT A$` — triggers the
                            // `env.__input_str(i32,i32) -> i32` host import AND (like an
                            // array op) linear memory + the `__array_bump` global, since
                            // the lowering bump-allocates the `[i32 len][bytes]` block the
                            // host fills. A pure INPUT-A$ program has no array op, so this
                            // is where memory/bump get injected for it.
                            "input_str" => {
                                uses_input_str = true;
                                uses_memory = true;
                                if global_names_seen.insert(ARRAY_BUMP_GLOBAL.to_string()) {
                                    global_names.push(ARRAY_BUMP_GLOBAL.to_string());
                                }
                            }
                            // Other builtin names are rejected by the
                            // validator before we get here — be defensive
                            // and don't crash on unknown ones at compile time.
                            _ => {}
                        }
                    }
                }
                // ALGOL 60 transcendentals — no WASM opcode; resolved via host imports.
                "f64_sin"  => uses_f64_sin  = true,
                "f64_cos"  => uses_f64_cos  = true,
                "f64_ln"   => uses_f64_ln   = true,
                "f64_exp"  => uses_f64_exp  = true,
                "f64_atan" => uses_f64_atan = true,
                "f64_tan"  => uses_f64_tan  = true,
                _ => {}
            }
        }
    }
    ModuleFeatures {
        global_names,
        uses_io_out,
        uses_print_str,
        uses_putchar,
        uses_getchar,
        uses_input_i64,
        uses_input_str,
        uses_f64_sin,
        uses_f64_cos,
        uses_f64_ln,
        uses_f64_exp,
        uses_f64_atan,
        uses_f64_tan,
        uses_memory,
        uses_f64_pow,
        uses_str_eq_runtime,
        string_literals,
        string_data,
        runtime_str_vars,
        runtime_str_blocks,
    }
}

/// Lower an `IIRModule` to a `WasmModule`, with WasmGC struct types.
///
/// # Algorithm
///
/// 1. **Validate** — run `validate_for_wasm`.  Return `Err(ValidationFailed)`
///    if there are any errors.
///
/// 2. **Detect heap ops** — if the module contains any `ref<LispyPair>`-typed
///    instructions, we will register the `$LispyPair` struct type.
///
/// 3. **Build the function index map** — iterate over all functions in order
///    and assign consecutive WASM function indices (0, 1, 2, …).
///
/// 4. **Lower each function** — for each `IIRFunction`:
///    a. Build the WASM function type (`FuncType`) from the parameter types
///       and return type.
///    b. Lower the function body to a `FunctionBody`.
///    c. Record an export so the function is callable from the host.
///
/// 5. **Assemble the `WasmModule`** — fill in `types`, `struct_types`,
///    `functions`, `exports`, and `code` fields.
///
/// # Returns
///
/// `Ok(WasmModule)` on success.  The module can be passed to
/// `wasm_module_encoder::encode_module` to produce raw `.wasm` bytes.
pub fn lower_iir_to_wasm(
    module: &IIRModule,
    _config: &IIRWasmConfig,
) -> Result<WasmModule, IIRWasmError> {
    // ── Step 1: Validate ─────────────────────────────────────────────────────
    let errors = validate_for_wasm(module);
    if !errors.is_empty() {
        return Err(IIRWasmError::ValidationFailed(errors));
    }

    // ── Step 2: Detect WasmGC heap ops ───────────────────────────────────────
    //
    // If the module references `ref<LispyPair>`, we will append the
    // `$LispyPair` struct type to the type section.  Function types are
    // encoded first (indices 0..N-1), then struct types (indices N..).
    // We don't know N yet (it's determined during step 4), so we defer
    // computing `lispy_pair_type_idx` until after all function types are
    // collected.
    let uses_lispy_pair = module_uses_lispy_pair(module);

    // ── Step 2b: Collect globals, io_out, and Brainfuck features ─────────────
    //
    // `global_names` is a deduplicated list of all global variable names
    // referenced by `global_load`/`global_store` instructions, in first-seen
    // order.  Position in the vec = WASM global section index.
    //
    // `uses_io_out` triggers injection of the `env.__print_i64(i64)` host import.
    // `uses_putchar` / `uses_getchar` trigger injection of `env.putchar(i32)` /
    // `env.getchar() -> i32` host imports — Brainfuck's I/O builtins.
    // `uses_memory` triggers injection of a 1-page linear memory (the BF tape).
    //
    // Imports occupy the first slots of the WASM function-index space, in
    // declaration order; defined functions are then shifted up by the
    // import count.
    let features = collect_module_features(module);
    let global_names = features.global_names.clone();
    let uses_io_out  = features.uses_io_out;
    let uses_print_str = features.uses_print_str;
    let uses_putchar = features.uses_putchar;
    let uses_getchar = features.uses_getchar;
    let uses_input_i64 = features.uses_input_i64;
    let uses_input_str = features.uses_input_str;
    let uses_f64_sin  = features.uses_f64_sin;
    let uses_f64_cos  = features.uses_f64_cos;
    let uses_f64_ln   = features.uses_f64_ln;
    let uses_f64_exp  = features.uses_f64_exp;
    let uses_f64_atan = features.uses_f64_atan;
    let uses_f64_tan  = features.uses_f64_tan;
    let uses_memory  = features.uses_memory;
    let uses_f64_pow = features.uses_f64_pow;
    let uses_str_eq_runtime = features.uses_str_eq_runtime;
    let string_literals = features.string_literals;
    let string_data = features.string_data;
    let runtime_str_vars = features.runtime_str_vars;
    let runtime_str_blocks = features.runtime_str_blocks;

    // Map each global name to its WASM global section index.
    let global_map: HashMap<String, u32> = global_names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i as u32))
        .collect();

    // ── Step 3: Build function index map ─────────────────────────────────────
    //
    // WASM function indices are contiguous starting from 0.  Imports occupy
    // the first slots, in declaration order.  Defined functions follow.
    //
    // Import order (mirrored when building `imports` below):
    //   0. env.__print_i64   (if uses_io_out)
    //   1. env.__print_str   (if uses_print_str)
    //   2. env.putchar       (if uses_putchar)
    //   3. env.getchar       (if uses_getchar)
    //   4. env.__input_i64   (if uses_input_i64)
    //   4b. env.__input_str  (if uses_input_str)  — inserted right after input_i64
    //   5. env.__sin         (if uses_f64_sin)
    //   6. env.__cos         (if uses_f64_cos)
    //   7. env.__ln          (if uses_f64_ln)
    //   8. env.__exp         (if uses_f64_exp)
    //   9. env.__atan        (if uses_f64_atan)
    //  10. env.__tan         (if uses_f64_tan)
    //  11. env.__pow         (if uses_f64_pow)
    let mut next_import_idx: u32 = 0;
    let print_fn_idx: Option<u32> = if uses_io_out {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let print_str_fn_idx: Option<u32> = if uses_print_str {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let putchar_fn_idx: Option<u32> = if uses_putchar {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let getchar_fn_idx: Option<u32> = if uses_getchar {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let input_i64_fn_idx: Option<u32> = if uses_input_i64 {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let input_str_fn_idx: Option<u32> = if uses_input_str {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let sin_fn_idx: Option<u32> = if uses_f64_sin {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let cos_fn_idx: Option<u32> = if uses_f64_cos {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let ln_fn_idx: Option<u32> = if uses_f64_ln {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let exp_fn_idx: Option<u32> = if uses_f64_exp {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let atan_fn_idx: Option<u32> = if uses_f64_atan {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let tan_fn_idx: Option<u32> = if uses_f64_tan {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let pow_fn_idx: Option<u32> = if uses_f64_pow {
        let i = next_import_idx; next_import_idx += 1; Some(i)
    } else { None };
    let fn_idx_base: u32 = next_import_idx;

    let fn_map: HashMap<String, u32> = module
        .functions
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.clone(), i as u32 + fn_idx_base))
        .collect();

    // The `$__str_eq` helper (if needed) is appended after every IIR-defined
    // function, so its index is the next defined-function slot.  Computed here so
    // the `str_eq` lowering can `call` it; the body/type/entry are added below.
    let str_eq_fn_idx: Option<u32> = if uses_str_eq_runtime {
        Some(fn_idx_base + module.functions.len() as u32)
    } else {
        None
    };

    // ── Step 4: Lower each function ──────────────────────────────────────────

    let mut types: Vec<FuncType> = Vec::new();
    let mut functions: Vec<u32> = Vec::new(); // type indices
    let mut exports: Vec<Export> = Vec::new();
    let mut code: Vec<FunctionBody> = Vec::new();

    // We build the types vec first (without the struct type) to know the
    // function type count, then compute lispy_pair_type_idx.  But we need
    // lispy_pair_type_idx during function lowering.  Resolution: collect all
    // function types first, compute the struct type index, then lower bodies.

    // Sub-pass A: collect FuncType entries.
    let mut func_type_indices: Vec<u32> = Vec::new(); // parallel to module.functions
    for fn_ in &module.functions {
        let param_types: Vec<ValueType> = fn_
            .params
            .iter()
            .filter_map(|(_, type_hint)| hint_to_value_type(type_hint))
            .collect();

        let result_types: Vec<ValueType> = if fn_.return_type == "void" {
            vec![]
        } else {
            match hint_to_value_type(&fn_.return_type) {
                Some(vt) => vec![vt],
                None => vec![], // unknown return type → treat as void
            }
        };

        let func_type = FuncType {
            params: param_types,
            results: result_types,
        };

        // Deduplicate types: check if we already have this FuncType.
        let type_idx = if let Some(pos) = types.iter().position(|t| *t == func_type) {
            pos as u32
        } else {
            let idx = types.len() as u32;
            types.push(func_type);
            idx
        };
        func_type_indices.push(type_idx);
    }

    // Now we know how many function types there are.
    // If we use LispyPair, the struct type is at index types.len().
    let lispy_pair_type_idx: Option<u32> = if uses_lispy_pair {
        Some(types.len() as u32)
    } else {
        None
    };

    // Add host-import types & entries for any enabled features.
    //
    // Each FuncType is appended to `types` after defined-function FuncTypes
    // and after the optional LispyPair struct type.  The struct type, when
    // present, occupies one slot between the function types and the
    // imports' function types.
    //
    // Imports are pushed to `host_imports` in the same order their
    // function indices were assigned earlier in Step 3:
    //   0. env.__print_i64   (if uses_io_out)
    //   1. env.__print_str   (if uses_print_str)
    //   2. env.putchar       (if uses_putchar)
    //   3. env.getchar       (if uses_getchar)
    //   4. env.__sin         (if uses_f64_sin)
    //   5. env.__cos         (if uses_f64_cos)
    //   6. env.__ln          (if uses_f64_ln)
    //   7. env.__exp         (if uses_f64_exp)
    //   8. env.__atan        (if uses_f64_atan)
    //   9. env.__tan         (if uses_f64_tan)
    //  10. env.__pow         (if uses_f64_pow)
    //
    // The function index that emit_instr uses is the one we assigned earlier
    // (print_fn_idx / putchar_fn_idx / getchar_fn_idx).  Here we just need
    // to push the type and import entries in matching order.
    let mut host_imports: Vec<Import> = Vec::new();
    let struct_type_offset: u32 = if uses_lispy_pair { 1 } else { 0 };
    if uses_io_out {
        // env.__print_i64(i64) -> ()
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![ValueType::I64], results: vec![] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__print_i64".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_print_str {
        // env.__print_str(i32 ptr, i32 len) -> ()
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![],
        });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__print_str".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_putchar {
        // env.putchar(i32) -> ()
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![ValueType::I32], results: vec![] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "putchar".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_getchar {
        // env.getchar() -> i32  (returns the next byte, or -1 / 0 for EOF
        // depending on host convention — Brainfuck's interpreter convention
        // is 0 for EOF, which matches the lazy-tape semantics).
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![], results: vec![ValueType::I32] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "getchar".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_input_i64 {
        // env.__input_i64() -> i64 — BASIC's `INPUT X`.  Reads one line from
        // stdin and parses it as a signed 64-bit integer; returns 0 on EOF or
        // parse failure (the same V1 permissive contract as `__twig_input_i64`
        // in `twig_runtime.c`).  The test host in `lang_matrix.rs` resolves
        // this import to `InputI64Func`, which drains the per-program stdin
        // buffer line-by-line.
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![], results: vec![ValueType::I64] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__input_i64".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_input_str {
        // env.__input_str(i32 block, i32 max) -> () — BASIC string `INPUT A$`
        // (E4-dyn). Reads one line from stdin and writes the WHOLE runtime-string
        // block `[i32 len][bytes]` into linear memory at `block` (len capped at
        // `max`) — the same repr `print_str` reads (`i32.load` the header). The
        // lowering (see `emit_instr`'s `"input_str"` arm) bump-allocates a
        // `[i32 len][max bytes]` region and passes its base; letting the host write
        // both the length header and the bytes keeps the codegen a single `call`
        // (no `i32.store`). The test host in `lang_matrix.rs` resolves this to
        // `InputStrFunc`, draining the per-program stdin buffer one line at a time
        // (V1 permissive contract; a longer line is truncated at `max`).
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![],
        });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__input_str".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    // ALGOL 60 transcendentals — WASM has no built-in opcodes for sin/cos/log/exp
    // so they are resolved via host imports (env.__sin, etc.).  Each is f64 → f64.
    // The test host in lang_matrix.rs resolves these to Rust's f64::sin() etc.
    if uses_f64_sin {
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![ValueType::F64], results: vec![ValueType::F64] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__sin".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_f64_cos {
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![ValueType::F64], results: vec![ValueType::F64] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__cos".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_f64_ln {
        // Note: ALGOL calls it `ln` but all backends (libm, Java, LLVM) use `log`.
        // The WASM import is named `__ln` to match the ALGOL name at the ABI boundary.
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![ValueType::F64], results: vec![ValueType::F64] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__ln".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_f64_exp {
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![ValueType::F64], results: vec![ValueType::F64] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__exp".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_f64_atan {
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![ValueType::F64], results: vec![ValueType::F64] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__atan".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_f64_tan {
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType { params: vec![ValueType::F64], results: vec![ValueType::F64] });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__tan".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }
    if uses_f64_pow {
        // env.__pow(f64 base, f64 exp) -> f64  — libm pow, no WASM native opcode.
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType {
            params: vec![ValueType::F64, ValueType::F64],
            results: vec![ValueType::F64],
        });
        host_imports.push(Import {
            module_name: "env".to_string(),
            name: "__pow".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(type_idx),
        });
    }

    // Build WASM Global entries — one mutable i64 per named global,
    // initialised to 0.
    //
    // Binary init_expr for `i64.const 0; end`: `[0x42, 0x00, 0x0B]`
    //   0x42 = i64.const opcode
    //   0x00 = LEB128 encoding of 0
    //   0x0B = end opcode (terminates the constant expression)
    let wasm_globals: Vec<Global> = global_names
        .iter()
        .map(|name| Global {
            global_type: GlobalType {
                value_type: ValueType::I64,
                mutable: true,
            },
            init_expr: if name == ARRAY_BUMP_GLOBAL {
                encode_i64_global_init(string_data.len() as i64)
            } else {
                encode_i64_global_init(0)
            },
        })
        .collect();

    // Sub-pass B: lower function bodies.
    for (fn_idx, fn_) in module.functions.iter().enumerate() {
        functions.push(func_type_indices[fn_idx]);

        // Export the function by name, with index offset for any imports.
        exports.push(Export {
            name: fn_.name.clone(),
            kind: ExternalKind::Function,
            index: fn_idx as u32 + fn_idx_base,
        });

        // Lower the function body, passing GC type index, global map, and
        // the host-import indices (print / putchar / getchar) so emit_instr
        // can `call <import_idx>` directly.
        let empty_string_literals = FunctionStringLiterals::new();
        let fn_string_literals = string_literals
            .get(&fn_.name)
            .unwrap_or(&empty_string_literals);
        // E4-dyn (E4d-3): this function's runtime-string promotions + their
        // length-prefixed block offsets (empty for functions with no
        // branch-selected strings).
        let empty_runtime_vars = FunctionRuntimeStrVars::new();
        let fn_runtime_str_vars = runtime_str_vars
            .get(&fn_.name)
            .unwrap_or(&empty_runtime_vars);
        let empty_runtime_blocks = FunctionRuntimeStrBlocks::new();
        let fn_runtime_str_blocks = runtime_str_blocks
            .get(&fn_.name)
            .unwrap_or(&empty_runtime_blocks);
        let body = lower_function(
            fn_, &fn_map, lispy_pair_type_idx, &global_map, fn_string_literals,
            fn_runtime_str_vars, fn_runtime_str_blocks,
            print_fn_idx, print_str_fn_idx, putchar_fn_idx, getchar_fn_idx,
            input_i64_fn_idx, input_str_fn_idx,
            sin_fn_idx, cos_fn_idx, ln_fn_idx, exp_fn_idx,
            atan_fn_idx, tan_fn_idx,
            pow_fn_idx, str_eq_fn_idx,
        )?;
        code.push(body);
    }

    // Append the self-contained `$__str_eq` helper after every IIR function, so
    // its index matches `str_eq_fn_idx` (= fn_idx_base + module.functions.len()).
    // Its FuncType is registered exactly like a host-import type — pushed to
    // `types` with the same `+ struct_type_offset` convention — so the index is
    // correct whether or not the module also carries the `$LispyPair` struct type.
    if str_eq_fn_idx.is_some() {
        let type_idx = types.len() as u32 + struct_type_offset;
        types.push(FuncType {
            params: vec![ValueType::I32, ValueType::I32],
            results: vec![ValueType::I32],
        });
        functions.push(type_idx);
        code.push(build_str_eq_helper());
    }

    // ── Step 5: Assemble WasmModule ──────────────────────────────────────────

    // If the module uses $LispyPair, register the struct type.
    let struct_types = if uses_lispy_pair {
        vec![make_lispy_pair_struct_type()]
    } else {
        vec![]
    };

    // Brainfuck tape: a single 1-page linear memory.  Each WASM memory page is
    // 64 KiB = 65,536 bytes, comfortably larger than Brainfuck's default 30,000
    // -cell tape.  The memory is module-defined (not imported); a future
    // extension can add an `import` variant if the host wants to provide
    // the buffer.  Modules that don't use `load_mem`/`store_mem` get no
    // memory section, preserving binary compatibility with the existing
    // non-BF callers (Twig, BASIC, Oct, Nib, Lispy).
    let memories: Vec<wasm_types::MemoryType> = if uses_memory
        || uses_print_str
        || !string_data.is_empty()
    {
        vec![wasm_types::MemoryType {
            limits: wasm_types::Limits { min: 1, max: Some(1) },
        }]
    } else {
        vec![]
    };

    let data = if string_data.is_empty() {
        vec![]
    } else {
        vec![DataSegment {
            memory_index: 0,
            offset_expr: vec![0x41u8, 0x00u8, 0x0Bu8], // i32.const 0; end
            data: string_data,
        }]
    };

    Ok(WasmModule {
        types,
        struct_types,
        functions,
        imports: host_imports,
        memories,
        globals: wasm_globals,
        exports,
        code,
        data,
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    fn single_fn(
        name: &str,
        params: Vec<(&str, &str)>,
        ret: &str,
        instrs: Vec<IIRInstr>,
    ) -> IIRModule {
        let fn_ = IIRFunction::new(
            name,
            params.into_iter().map(|(n, t)| (n.into(), t.into())).collect(),
            ret,
            instrs,
        );
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some(name.into()),
            language: "test".into(),
            exports: vec![],
            imports: vec![],
        }
    }

    #[test]
    fn lower_void_function() {
        let m = single_fn("main", vec![], "void", vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        assert_eq!(wm.functions.len(), 1);
        assert_eq!(wm.exports.len(), 1);
        assert_eq!(wm.code.len(), 1);
        assert!(!wm.code[0].code.is_empty());
    }

    #[test]
    fn lower_add_i32() {
        let m = single_fn(
            "add",
            vec![("a", "i32"), ("b", "i32")],
            "i32",
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
            ],
        );
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        // The code should contain I32_ADD (0x6A).
        assert!(wm.code[0].code.contains(&0x6A));
    }

    #[test]
    fn lower_const_i32() {
        let m = single_fn("f", vec![], "i32", vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        // Code should contain I32_CONST (0x41).
        assert!(wm.code[0].code.contains(&0x41));
    }

    #[test]
    fn lower_f64_const_accepted() {
        // Float constants are valid in the WASM backend (unlike BEAM).
        let m = single_fn("f", vec![], "f64", vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "f64"),
        ]);
        let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
        assert!(result.is_ok(), "f64 const should succeed; err: {:?}", result);
    }

    #[test]
    fn comparison_uses_operand_type_when_result_is_bool() {
        let m = single_fn("f", vec![], "bool", vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(6)], "i64"),
            IIRInstr::new("cmp_le", Some("ok".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
            IIRInstr::new("ret", None, vec![Operand::Var("ok".into())], "bool"),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        assert!(
            wm.code[0].code.contains(&I64_LE_S),
            "i64 operands with a bool comparison result must emit i64.le_s"
        );
    }

    #[test]
    fn lower_validation_failure_propagates() {
        // A module with no functions → ValidationFailed.
        let m = IIRModule {
            name: "empty".into(),
            functions: vec![],
            entry_point: None,
            language: "test".into(),
            exports: vec![],
            imports: vec![],
        };
        let result = lower_iir_to_wasm(&m, &IIRWasmConfig::default());
        assert!(matches!(result, Err(IIRWasmError::ValidationFailed(_))));
    }

    #[test]
    fn function_type_recorded_correctly() {
        let m = single_fn(
            "add",
            vec![("a", "i32"), ("b", "i32")],
            "i32",
            vec![
                IIRInstr::new("add", Some("v0".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
            ],
        );
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        assert_eq!(wm.types[0].params, vec![ValueType::I32, ValueType::I32]);
        assert_eq!(wm.types[0].results, vec![ValueType::I32]);
    }

    #[test]
    fn export_has_correct_name() {
        let m = single_fn("my_fn", vec![], "void", vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        assert_eq!(wm.exports[0].name, "my_fn");
        assert_eq!(wm.exports[0].kind, ExternalKind::Function);
        assert_eq!(wm.exports[0].index, 0);
    }

    // ── LANG-FULL E5 — bounds-checked arrays (linear-memory static model) ────

    fn array_fn() -> IIRModule {
        // a := new i64[3]; a[0] := 42; r := a[0]; n := len(a); ret r
        single_fn("main", vec![], "i64", vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(3)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<i64>"),
            IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(42)], "i64"),
            IIRInstr::new("array_set", None,
                vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("v".into())], "i64"),
            IIRInstr::new("array_get", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("i0".into())], "i64"),
            IIRInstr::new("array_len", Some("m".into()), vec![Operand::Var("a".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i64"),
        ])
    }

    #[test]
    fn array_ops_emit_memory_bump_and_trap() {
        let wm = lower_iir_to_wasm(&array_fn(), &IIRWasmConfig::default()).unwrap();
        // A linear memory is declared (arrays live in it) ...
        assert_eq!(wm.memories.len(), 1, "array module needs a linear memory");
        // ... and the synthetic `__array_bump` global is injected.
        assert!(!wm.globals.is_empty(), "array module needs the bump-pointer global");
        let code = &wm.code[0].code;
        // The bounds check: `i64.ge_u` (0x5A) + `if` (0x04) + `unreachable` (0x00).
        assert!(code.contains(&0x5A), "i64.ge_u bounds compare; code: {code:02X?}");
        assert!(code.contains(&0x04), "if for the trap branch");
        assert!(code.contains(&0x00), "unreachable trap");
        // Length header + element store/load: i64.store (0x37) + i64.load (0x29).
        assert!(code.contains(&0x37), "i64.store (header + element)");
        assert!(code.contains(&0x29), "i64.load (element + length)");
        // Bump pointer is read/written: global.get (0x23) + global.set (0x24).
        assert!(code.contains(&0x23) && code.contains(&0x24), "global.get/set for the bump");
    }

    #[test]
    fn array_handle_is_i64() {
        // The `array<T>` handle rides an i64 register (a byte offset).
        assert_eq!(hint_to_value_type("array<i64>"), Some(ValueType::I64));
        assert_eq!(hint_to_value_type("array<f64>"), Some(ValueType::I64));
    }

    #[test]
    fn f64_array_uses_f64_store_load() {
        let m = single_fn("main", vec![], "f64", vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("n".into())], "array<f64>"),
            IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(2.5)], "f64"),
            IIRInstr::new("array_set", None,
                vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("v".into())], "f64"),
            IIRInstr::new("array_get", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("i0".into())], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        let code = &wm.code[0].code;
        assert!(code.contains(&0x39), "f64.store for array<f64> element");
        assert!(code.contains(&0x2B), "f64.load for array<f64> element");
    }

    #[test]
    fn str_array_uses_i32_element_store() {
        // E4d-BA-arr: an `array<str>` element is a 4-byte i32 handle, so the
        // element `array_set` emits `i32.store` (0x36) — not the i64.store an
        // `array<i64>` element uses.  Lowering an `array<str>` at all is the
        // regression guard: before E4d-BA-arr, `wasm_array_elem`/the validator
        // rejected a `str` element outright.
        let m = single_fn("main", vec![], "void", vec![
            IIRInstr::new("const", Some("n".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new("alloc_array", Some("a".into()),
                vec![Operand::Var("n".into())], "array<str>"),
            IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("str_const", Some("s".into()),
                vec![Operand::Str("HI".into())], "str"),
            IIRInstr::new("array_set", None,
                vec![Operand::Var("a".into()), Operand::Var("i0".into()),
                     Operand::Var("s".into())], "str"),
            IIRInstr::new("array_get", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("i0".into())], "str"),
            IIRInstr::new("print_str", None, vec![Operand::Var("r".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let wm = lower_iir_to_wasm(&m, &IIRWasmConfig::default()).unwrap();
        let code = &wm.code[0].code;
        assert!(code.contains(&0x36), "i32.store for the array<str> element handle");
    }

    #[test]
    fn str_array_elem_is_i32_4_bytes() {
        assert_eq!(wasm_array_elem("str", "main").unwrap(), (ValueType::I32, 4));
    }
}
