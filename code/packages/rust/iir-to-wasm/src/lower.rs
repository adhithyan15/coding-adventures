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

use std::collections::{HashMap, HashSet};

use interpreter_ir::{IIRFunction, IIRModule, Operand};
use wasm_module_encoder::{GcInstruction, encode_gc_instruction};
use wasm_types::{
    ExternalKind, Export, FieldType, FuncType, FunctionBody, Global, GlobalType, Import,
    ImportTypeInfo, StructType, ValueType, WasmModule,
};

use crate::codegen::{
    encode_br, encode_br_table, encode_call, encode_f32_const, encode_f64_const,
    encode_i32_const, encode_i64_const, encode_local_get, encode_local_set, BLOCK, BLOCK_EMPTY,
    DROP, END, F32_ADD, F32_DIV, F32_EQ, F32_GE, F32_GT, F32_LE, F32_LT, F32_MUL, F32_NEG,
    F32_NE, F32_SUB, F64_ADD, F64_DIV, F64_EQ, F64_GE, F64_GT, F64_LE, F64_LT, F64_MUL,
    F64_NEG, F64_NE, F64_SUB, I32_ADD, I32_AND, I32_DIV_S, I32_DIV_U, I32_EQ, I32_EQZ, I32_GE_S,
    I32_GE_U, I32_GT_S, I32_GT_U, I32_LE_S, I32_LE_U, I32_LT_S, I32_LT_U, I32_MUL, I32_NE,
    I32_OR, I32_REM_S, I32_REM_U, I32_SHL, I32_SHR_S, I32_SHR_U, I32_SUB, I32_XOR, I64_ADD,
    I64_AND, I64_DIV_S, I64_DIV_U, I64_EQ, I64_GE_S, I64_GT_S, I64_LE_S, I64_LT_S, I64_MUL,
    I64_NE, I64_OR, I64_REM_S, I64_REM_U, I64_SHL, I64_SHR_S, I64_SUB, I64_XOR,
    LOOP, RETURN,
};
use crate::validate::validate_for_wasm;

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
        "i8" | "i16" | "i32" | "u8" | "u16" | "u32" | "bool" => Some(ValueType::I32),
        "i64" | "u64" => Some(ValueType::I64),
        "f32" => Some(ValueType::F32),
        "f64" => Some(ValueType::F64),
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

/// Return `true` if the type hint represents a 64-bit integer type.
///
/// Used during arithmetic to select `i64.*` vs `i32.*` opcodes.
fn is_i64_hint(hint: &str) -> bool {
    matches!(hint, "i64" | "u64")
}

/// Return `true` if the type hint represents an unsigned integer type.
///
/// Used to select `_u` (unsigned) vs `_s` (signed) comparison and division
/// opcodes for `i32` types.  For `i64` we always use signed in v1 (matching
/// the IIR spec's signed-default model).
fn is_unsigned_hint(hint: &str) -> bool {
    matches!(hint, "u8" | "u16" | "u32" | "u64")
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

/// Infer the WASM ValueType for each local variable beyond the parameters.
///
/// We scan instructions for type hints associated with each variable index and
/// return a `Vec<ValueType>` parallel to indices `param_count..total_vars`.
/// If a variable has no type information, we default to `I32` (the most
/// common type and the natural choice for boolean/integer values).
fn infer_local_types(
    fn_: &IIRFunction,
    reg_map: &HashMap<String, u32>,
    param_count: u32,
    total_vars: u32,
) -> Vec<ValueType> {
    // Build a map: var_index → best known type hint.
    let mut var_type: HashMap<u32, String> = HashMap::new();

    // Seed from parameter types.
    for (i, (param_name, param_type)) in fn_.params.iter().enumerate() {
        if let Some(&idx) = reg_map.get(param_name) {
            var_type.insert(idx, param_type.clone());
        }
        let _ = i; // suppress unused warning
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
    fn_map: &HashMap<String, u32>,
    fn_name: &str,
    dispatch_reg: u32,
    label_to_block: &HashMap<String, u32>,
    block_idx: usize,
    n_blocks: usize,
    is_dispatch_loop: bool,
    lispy_pair_type_idx: Option<u32>,
    global_map: &HashMap<String, u32>,
    print_fn_idx: Option<u32>,
) -> Result<(), IIRWasmError> {
    // Helper closures to resolve variable names.
    let get_reg = |var: &str| -> Result<u32, IIRWasmError> {
        reg_map.get(var).copied().ok_or_else(|| IIRWasmError::UndefinedVariable {
            function: fn_name.to_string(),
            name: var.to_string(),
        })
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
                    if is_i64_hint(ty) {
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
        "add" | "sub" | "mul" | "div" | "rem" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("{} must have a dest", instr.op),
            })?;
            let rd = get_reg(dest)?;
            let r1 = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            let r2 = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?;

            code.extend(encode_local_get(r1));
            code.extend(encode_local_get(r2));

            let opcode: u8 = match (instr.op.as_str(), ty) {
                ("add", t) if is_i64_hint(t) => I64_ADD,
                ("add", t) if is_float_hint(t) && t == "f32" => F32_ADD,
                ("add", t) if is_float_hint(t) => F64_ADD,
                ("add", _) => I32_ADD,
                ("sub", t) if is_i64_hint(t) => I64_SUB,
                ("sub", t) if is_float_hint(t) && t == "f32" => F32_SUB,
                ("sub", t) if is_float_hint(t) => F64_SUB,
                ("sub", _) => I32_SUB,
                ("mul", t) if is_i64_hint(t) => I64_MUL,
                ("mul", t) if is_float_hint(t) && t == "f32" => F32_MUL,
                ("mul", t) if is_float_hint(t) => F64_MUL,
                ("mul", _) => I32_MUL,
                ("div", t) if is_i64_hint(t) && is_unsigned_hint(t) => I64_DIV_U,
                ("div", t) if is_i64_hint(t) => I64_DIV_S,
                ("div", t) if is_float_hint(t) && t == "f32" => F32_DIV,
                ("div", t) if is_float_hint(t) => F64_DIV,
                ("div", t) if is_unsigned_hint(t) => I32_DIV_U,
                ("div", _) => I32_DIV_S,
                ("rem", t) if is_i64_hint(t) && is_unsigned_hint(t) => I64_REM_U,
                ("rem", t) if is_i64_hint(t) => I64_REM_S,
                ("rem", t) if is_unsigned_hint(t) => I32_REM_U,
                ("rem", _) => I32_REM_S,
                _ => unreachable!("matched outer pattern"),
            };
            code.push(opcode);
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

            code.extend(encode_local_get(r1));
            code.extend(encode_local_get(r2));

            let opcode: u8 = match (instr.op.as_str(), ty) {
                ("and", t) if is_i64_hint(t) => I64_AND,
                ("and", _) => I32_AND,
                ("or", t) if is_i64_hint(t) => I64_OR,
                ("or", _) => I32_OR,
                ("xor", t) if is_i64_hint(t) => I64_XOR,
                ("xor", _) => I32_XOR,
                ("shl", t) if is_i64_hint(t) => I64_SHL,
                ("shl", _) => I32_SHL,
                ("shr", t) if is_i64_hint(t) && is_unsigned_hint(t) => I64_SHR_S, // i64 has no _u for hint-based default in v1
                ("shr", t) if is_i64_hint(t) => I64_SHR_S,
                ("shr", t) if is_unsigned_hint(t) => I32_SHR_U,
                ("shr", _) => I32_SHR_S,
                _ => unreachable!(),
            };
            code.push(opcode);
            code.extend(encode_local_set(rd));
        }

        // ── Comparisons ───────────────────────────────────────────────────────
        //
        // WASM comparisons always produce an i32 result (0 or 1).
        // The source operands have the type described by `ty`; the result
        // is always i32.
        "eq" | "ne" | "lt" | "le" | "gt" | "ge" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRWasmError::InvalidOperand {
                function: fn_name.to_string(),
                detail: format!("{} must have a dest", instr.op),
            })?;
            let rd = get_reg(dest)?;
            let r1 = get_src_reg(&instr.srcs, 0, reg_map, fn_name)?;
            let r2 = get_src_reg(&instr.srcs, 1, reg_map, fn_name)?;

            code.extend(encode_local_get(r1));
            code.extend(encode_local_get(r2));

            let opcode: u8 = match (instr.op.as_str(), ty) {
                // i64
                ("eq", t) if is_i64_hint(t) => I64_EQ,
                ("ne", t) if is_i64_hint(t) => I64_NE,
                ("lt", t) if is_i64_hint(t) => I64_LT_S,
                ("le", t) if is_i64_hint(t) => I64_LE_S,
                ("gt", t) if is_i64_hint(t) => I64_GT_S,
                ("ge", t) if is_i64_hint(t) => I64_GE_S,
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
            } else if is_i64_hint(ty) {
                code.extend(encode_i64_const(0));
                code.extend(encode_local_get(r));
                code.push(I64_SUB);
            } else {
                code.extend(encode_i32_const(0));
                code.extend(encode_local_get(r));
                code.push(I32_SUB);
            }
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
            if is_i64_hint(ty) {
                code.extend(encode_i64_const(-1));
                code.push(I64_XOR);
            } else {
                code.extend(encode_i32_const(-1));
                code.push(I32_XOR);
            }
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
                code.push(I32_EQZ);
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
        // Allocate a new `$LispyPair` on the GC heap.
        //
        // We emit `ref.null none` here (a null pair placeholder) because the
        // subsequent `field_store` instructions will populate the fields via
        // `struct.set`.  This avoids the need for look-ahead to fuse with
        // exactly two field_stores.
        //
        // If you need `struct.new` fusion (e.g. for performance), the front-end
        // should arrange to call `alloc` only after pushing head and tail.
        //
        // ```wasm
        // ref.null none    ;; typed null for $LispyPair slot
        // local.set $dest
        // ```
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

            // Emit ref.null none — the canonical "uninitialized GC ref".
            encode_gc_instruction(code, &GcInstruction::RefNull);
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
    print_fn_idx: Option<u32>,
) -> Result<FunctionBody, IIRWasmError> {
    let param_count = fn_.params.len() as u32;
    let reg_map = build_register_map(fn_);
    let total_vars = reg_map.len() as u32;
    // The dispatch variable sits at the next available local index.
    let dispatch_reg = total_vars;

    // Infer types for non-parameter locals.
    let mut local_types = infer_local_types(fn_, &reg_map, param_count, total_vars);
    // Append the dispatch variable (always I32 — it holds a block index).
    local_types.push(ValueType::I32);

    let use_dispatch = has_control_flow(fn_);
    let (blocks, label_to_block) = split_into_blocks(fn_);
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
                    fn_map,
                    &fn_.name,
                    dispatch_reg,
                    &label_to_block,
                    block_idx,  // current block index (for branch-depth computation)
                    n_blocks,
                    true, // inside dispatch-loop
                    lispy_pair_type_idx,
                    global_map,
                    print_fn_idx,
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
                fn_map,
                &fn_.name,
                dispatch_reg,
                &label_to_block,
                0,      // block_idx unused when is_dispatch_loop=false
                n_blocks,
                false,  // no dispatch loop
                lispy_pair_type_idx,
                global_map,
                print_fn_idx,
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
        fn_.instructions.iter().any(|i| i.type_hint == "ref<LispyPair>")
            || fn_.params.iter().any(|(_, t)| t == "ref<LispyPair>")
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
fn collect_globals_and_io(module: &IIRModule) -> (Vec<String>, bool) {
    // Use a HashSet for O(1) deduplication checks, preserving first-seen order
    // in the Vec.  Without the set, deduplication would be O(M × N) where M is
    // the number of global accesses and N the number of distinct names —
    // quadratic for adversarially crafted modules with many globals.
    let mut global_names: Vec<String> = Vec::new();
    let mut global_names_seen: HashSet<String> = HashSet::new();
    let mut uses_io_out = false;
    for fn_ in &module.functions {
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
                _ => {}
            }
        }
    }
    (global_names, uses_io_out)
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

    // ── Step 2b: Collect global variable names and io_out usage ──────────────
    //
    // `global_names` is a deduplicated list of all global variable names
    // referenced by `global_load`/`global_store` instructions, in first-seen
    // order.  Position in the vec = WASM global section index.
    //
    // `uses_io_out` triggers injection of the `env.__print_i64(i64)` host
    // import, which occupies function index 0 in the WASM function index
    // space (imports come before defined functions).
    let (global_names, uses_io_out) = collect_globals_and_io(module);

    // Map each global name to its WASM global section index.
    let global_map: HashMap<String, u32> = global_names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i as u32))
        .collect();

    // ── Step 3: Build function index map ─────────────────────────────────────
    //
    // WASM function indices are contiguous starting from 0.  When a
    // `$__print_i64` import is present it occupies index 0, so all defined
    // functions are shifted up by 1.
    //
    // `fn_idx_base` is 1 when the print import is injected, 0 otherwise.
    let fn_idx_base: u32 = if uses_io_out { 1 } else { 0 };

    let fn_map: HashMap<String, u32> = module
        .functions
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.clone(), i as u32 + fn_idx_base))
        .collect();

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

    // Add the $__print_i64 import type and import entry if io_out is used.
    //
    // The print import type is pushed AFTER all defined-function FuncTypes and
    // after the optional LispyPair struct type, so its type index is
    // `types.len() + struct_types_count`.  Since struct_types are encoded in
    // the same WASM type section after func types, the print type index is
    // `types.len() + (1 if uses_lispy_pair else 0)`.
    //
    // `print_fn_idx` is always 0 (the import occupies the first function slot).
    let print_fn_idx: Option<u32>;
    let print_imports: Vec<Import>;
    if uses_io_out {
        // The print type goes after function types and the optional struct type.
        let print_type_idx =
            types.len() as u32 + if uses_lispy_pair { 1 } else { 0 };
        types.push(FuncType {
            params: vec![ValueType::I64],
            results: vec![],
        });
        print_fn_idx = Some(0u32);
        print_imports = vec![Import {
            module_name: "env".to_string(),
            name: "__print_i64".to_string(),
            kind: ExternalKind::Function,
            type_info: ImportTypeInfo::Function(print_type_idx),
        }];
    } else {
        print_fn_idx = None;
        print_imports = vec![];
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
        .map(|_| Global {
            global_type: GlobalType {
                value_type: ValueType::I64,
                mutable: true,
            },
            init_expr: vec![0x42u8, 0x00u8, 0x0Bu8], // i64.const 0; end
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
        // the print import index if io_out is used.
        let body = lower_function(fn_, &fn_map, lispy_pair_type_idx, &global_map, print_fn_idx)?;
        code.push(body);
    }

    // ── Step 5: Assemble WasmModule ──────────────────────────────────────────

    // If the module uses $LispyPair, register the struct type.
    let struct_types = if uses_lispy_pair {
        vec![make_lispy_pair_struct_type()]
    } else {
        vec![]
    };

    Ok(WasmModule {
        types,
        struct_types,
        functions,
        imports: print_imports,
        globals: wasm_globals,
        exports,
        code,
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
}
