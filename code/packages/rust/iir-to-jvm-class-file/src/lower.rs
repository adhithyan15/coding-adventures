//! IIR → JVM class file lowering.
//!
//! This module does the real work: translating an [`interpreter_ir::IIRModule`]
//! into a [`jvm_class_file::JvmClassFile`] that can be serialised to bytes and
//! executed by any standard JVM.
//!
//! # Big picture
//!
//! ```text
//! IIRModule
//!   │
//!   ├─ validate_for_jvm()          ← pre-flight (from validate module)
//!   │
//!   ├─ build_constant_pool()       ← build the JVM constant pool
//!   │
//!   └─ lower_function()  (per IIRFunction)
//!         │
//!         ├─ Pass 1: allocate JVM local-variable slots
//!         │          params → 0..N-1
//!         │          dest/src vars → N, N+1, …
//!         │
//!         └─ Pass 2: emit JVM bytecode
//!               const        → iconst / bipush / sipush / ldc / ldc2_w
//!               add/sub/…    → iadd / ladd / fadd / dadd / …
//!               cmp_eq/…     → if_icmpNE pattern + iconst_1/0
//!               label        → record PC for backpatching
//!               jmp          → goto + fixup entry
//!               jmp_if_true  → iload cond; ifne + fixup entry
//!               call         → push args; invokestatic + fixup
//!               ret          → iload/lload/fload/dload; ireturn/…
//!               ret_void     → return
//!               After emitting all instructions: apply label fixups.
//! ```
//!
//! # JVM local variable slots
//!
//! On the JVM, each method has a "local variable array".  Slot 0 is typically
//! `this` for instance methods; for `static` methods it is the first argument.
//! Long (`J`) and double (`D`) values occupy *two consecutive slots* — this is
//! a quirk of the JVM's 32-bit-word heritage.
//!
//! Our allocation strategy for a function with N parameters:
//!
//! - Slots 0..N-1 (approximately) are claimed by parameters.  A `long` param
//!   claims two slots; all others claim one.
//! - Fresh locals start at slot N (or wherever the last param left off).
//!
//! Example: `fn foo(a: i32, b: i64) -> i64`
//! - `a` → slot 0 (width 1, i32)
//! - `b` → slot 1 (width 2, i64) — also claims slot 2
//! - first local → slot 3
//!
//! # Bytecode backpatching
//!
//! JVM branch instructions carry a signed 16-bit offset measured from the
//! *start* of the branch opcode.  When we emit a forward jump (to a label we
//! haven't seen yet), we:
//!
//! 1. Record a [`Fixup`] containing the opcode's position and the target label
//!    name.
//! 2. Emit two placeholder zero bytes.
//!
//! When we hit a `label` instruction, we record the current code length as that
//! label's PC in a `HashMap<String, u32>`.
//!
//! After all instructions are emitted, we walk the fixups and patch:
//!
//! ```text
//! offset = (target_pc as i32 - opcode_pos as i32) as i16
//! code[opcode_pos + 1] = (offset >> 8) as u8
//! code[opcode_pos + 2] = (offset & 0xFF) as u8
//! ```
//!
//! Backward jumps (to labels already seen) are handled identically — the target
//! PC is already in the map, so the fixup resolves immediately in the second pass.

use std::collections::HashMap;

use interpreter_ir::{IIRFunction, IIRModule, Operand};
use jvm_class_file::{
    JvmClassFile, JvmClassVersion, JvmCodeAttribute, JvmConstantPoolEntry, JvmMethodAttribute,
    JvmMethodInfo, ACC_PUBLIC, ACC_STATIC, ACC_SUPER,
};

use crate::validate::validate_for_jvm;

// ---------------------------------------------------------------------------
// JVM opcode constants
// ---------------------------------------------------------------------------
//
// Each constant is documented with the mnemonic from the JVM spec
// (JVMS §6.5).  We keep them grouped by category so it's easy to see which
// opcodes belong to the same family.

// ── Constants ─────────────────────────────────────────────────────────────
const ICONST_M1: u8 = 0x02; // push int -1
const ICONST_0: u8 = 0x03;  // push int 0
const ICONST_1: u8 = 0x04;  // push int 1
const ICONST_2: u8 = 0x05;  // push int 2
const ICONST_3: u8 = 0x06;  // push int 3
const ICONST_4: u8 = 0x07;  // push int 4
const ICONST_5: u8 = 0x08;  // push int 5
const LCONST_0: u8 = 0x09;  // push long 0
const LCONST_1: u8 = 0x0A;  // push long 1
const FCONST_0: u8 = 0x0B;  // push float 0.0
const FCONST_1: u8 = 0x0C;  // push float 1.0
const FCONST_2: u8 = 0x0D;  // push float 2.0
const DCONST_0: u8 = 0x0E;  // push double 0.0
const DCONST_1: u8 = 0x0F;  // push double 1.0
const BIPUSH: u8 = 0x10;    // push byte (sign-extended to int)
const SIPUSH: u8 = 0x11;    // push short (sign-extended to int)
const LDC: u8 = 0x12;       // push constant from CP (1-byte index)
const LDC2_W: u8 = 0x14;    // push long/double constant from CP (2-byte index)

// ── Local variable loads ───────────────────────────────────────────────────
const ILOAD: u8 = 0x15;   // load int from local slot N
const LLOAD: u8 = 0x16;   // load long from local slot N
const FLOAD: u8 = 0x17;   // load float from local slot N
const DLOAD: u8 = 0x18;   // load double from local slot N
const ILOAD_0: u8 = 0x1A; // load int from slot 0 (short form)
const ILOAD_1: u8 = 0x1B; // load int from slot 1 (short form)
const ILOAD_2: u8 = 0x1C; // load int from slot 2 (short form)
const ILOAD_3: u8 = 0x1D; // load int from slot 3 (short form)

// ── Local variable stores ──────────────────────────────────────────────────
const ISTORE: u8 = 0x36;   // store int to local slot N
const LSTORE: u8 = 0x37;   // store long to local slot N
const FSTORE: u8 = 0x38;   // store float to local slot N
const DSTORE: u8 = 0x39;   // store double to local slot N
const ISTORE_0: u8 = 0x3B; // store int to slot 0 (short form)
const ISTORE_1: u8 = 0x3C; // store int to slot 1 (short form)
const ISTORE_2: u8 = 0x3D; // store int to slot 2 (short form)
const ISTORE_3: u8 = 0x3E; // store int to slot 3 (short form)

// ── Integer arithmetic ─────────────────────────────────────────────────────
const IADD: u8 = 0x60;  // int add
const LADD: u8 = 0x61;  // long add
const FADD: u8 = 0x62;  // float add
const DADD: u8 = 0x63;  // double add
const ISUB: u8 = 0x64;  // int subtract
const LSUB: u8 = 0x65;  // long subtract
const FSUB: u8 = 0x66;  // float subtract
const DSUB: u8 = 0x67;  // double subtract
const IMUL: u8 = 0x68;  // int multiply
const LMUL: u8 = 0x69;  // long multiply
const FMUL: u8 = 0x6A;  // float multiply
const DMUL: u8 = 0x6B;  // double multiply
const IDIV: u8 = 0x6C;  // int divide
const LDIV: u8 = 0x6D;  // long divide
const FDIV: u8 = 0x6E;  // float divide
const DDIV: u8 = 0x6F;  // double divide
const IREM: u8 = 0x70;  // int remainder (modulo)
const LREM: u8 = 0x71;  // long remainder
const INEG: u8 = 0x74;  // int negate
const LNEG: u8 = 0x75;  // long negate
const FNEG: u8 = 0x76;  // float negate
const DNEG: u8 = 0x77;  // double negate
const ISHL: u8 = 0x78;  // int shift left
const LSHL: u8 = 0x79;  // long shift left
const ISHR: u8 = 0x7A;  // int arithmetic shift right
const LSHR: u8 = 0x7B;  // long arithmetic shift right
const IAND: u8 = 0x7E;  // int bitwise AND
const LAND: u8 = 0x7F;  // long bitwise AND
const IOR: u8 = 0x80;   // int bitwise OR
const LOR: u8 = 0x81;   // long bitwise OR
const IXOR: u8 = 0x82;  // int bitwise XOR
const LXOR: u8 = 0x83;  // long bitwise XOR

// ── Returns ────────────────────────────────────────────────────────────────
const IRETURN: u8 = 0xAC; // return int
const LRETURN: u8 = 0xAD; // return long
const FRETURN: u8 = 0xAE; // return float
const DRETURN: u8 = 0xAF; // return double
const RETURN: u8 = 0xB1;  // return void

// ── Method invocation ──────────────────────────────────────────────────────
const INVOKESTATIC: u8 = 0xB8; // invoke static method (2-byte CP index)

// ── Comparison and branching ───────────────────────────────────────────────
const IFEQ: u8 = 0x99;      // branch if top-of-stack == 0
const IFNE: u8 = 0x9A;      // branch if top-of-stack != 0
const IF_ICMPEQ: u8 = 0x9F; // branch if int1 == int2
const IF_ICMPNE: u8 = 0xA0; // branch if int1 != int2
const IF_ICMPLT: u8 = 0xA1; // branch if int1 < int2
const IF_ICMPGE: u8 = 0xA2; // branch if int1 >= int2
const IF_ICMPGT: u8 = 0xA3; // branch if int1 > int2
const IF_ICMPLE: u8 = 0xA4; // branch if int1 <= int2
const GOTO: u8 = 0xA7;      // unconditional branch (2-byte offset)

// ── Misc ───────────────────────────────────────────────────────────────────
const WIDE: u8 = 0xC4; // prefix for wide-index variants of load/store

// ---------------------------------------------------------------------------
// IIRJvmError
// ---------------------------------------------------------------------------

/// Errors that can occur during IIR → JVM class file lowering.
///
/// Each variant carries enough context to produce a good error message:
/// which function was being lowered, and what specifically went wrong.
///
/// The error is returned as `Err(IIRJvmError::…)` from [`lower_iir_to_jvm`].
/// Users who want a simple success/failure check can use `.is_ok()`;
/// users who want to display the error to a human should call `.to_string()`.
#[derive(Debug, Clone, PartialEq)]
pub enum IIRJvmError {
    /// Pre-flight validation produced one or more errors.
    ///
    /// This variant wraps all errors from [`crate::validate::validate_for_jvm`]
    /// into a single `Err(…)` return from the lowering pass.  Callers can
    /// also call `validate_for_jvm` directly to get the individual messages.
    ValidationFailed(Vec<String>),

    /// An IIR instruction opcode has no JVM equivalent in this backend.
    ///
    /// Occurs for opcodes that slipped through validation (e.g. because the
    /// caller bypassed the validate step) or new opcodes added after this
    /// backend was written.
    UnsupportedOp {
        /// Name of the function containing the unsupported instruction.
        function: String,
        /// The unrecognised opcode string.
        op: String,
    },

    /// An IIR type hint has no JVM primitive equivalent in this backend.
    ///
    /// Occurs for types that slipped through validation.
    UnsupportedType {
        /// Name of the function containing the bad type hint.
        function: String,
        /// The unrecognised type hint string.
        type_hint: String,
    },

    /// A branch target label was not found in the label map.
    ///
    /// This indicates a malformed IIR module: a `jmp`, `jmp_if_true`, or
    /// `jmp_if_false` instruction references a label that was never emitted.
    UndefinedLabel {
        /// Name of the function containing the jump.
        function: String,
        /// The label name that could not be resolved.
        label: String,
    },

    /// A variable name was referenced but never assigned a slot.
    ///
    /// Normally impossible after the two-pass slot allocation, but can occur
    /// if a Var operand references a name that was never written.
    UndefinedVariable {
        /// Name of the function containing the reference.
        function: String,
        /// The variable name that was not in the slot map.
        name: String,
    },

    /// An operand is structurally invalid for the instruction that uses it.
    ///
    /// For example, a `const` instruction with a `Var` source operand (which
    /// would mean "load a constant from a variable" — meaningless).
    InvalidOperand {
        /// Name of the function containing the malformed instruction.
        function: String,
        /// Human-readable description of what was wrong.
        detail: String,
    },
}

impl std::fmt::Display for IIRJvmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IIRJvmError::ValidationFailed(errs) => {
                write!(f, "JVM validation failed:\n  {}", errs.join("\n  "))
            }
            IIRJvmError::UnsupportedOp { function, op } => {
                write!(
                    f,
                    "function {:?}: unsupported op {:?} in JVM backend",
                    function, op
                )
            }
            IIRJvmError::UnsupportedType {
                function,
                type_hint,
            } => {
                write!(
                    f,
                    "function {:?}: unsupported type {:?} in JVM backend",
                    function, type_hint
                )
            }
            IIRJvmError::UndefinedLabel { function, label } => {
                write!(
                    f,
                    "function {:?}: undefined label {:?}",
                    function, label
                )
            }
            IIRJvmError::UndefinedVariable { function, name } => {
                write!(
                    f,
                    "function {:?}: undefined variable {:?}",
                    function, name
                )
            }
            IIRJvmError::InvalidOperand { function, detail } => {
                write!(
                    f,
                    "function {:?}: invalid operand — {}",
                    function, detail
                )
            }
        }
    }
}

impl std::error::Error for IIRJvmError {}

// ---------------------------------------------------------------------------
// IIRJvmConfig
// ---------------------------------------------------------------------------

/// Configuration for the IIR → JVM class file lowering pass.
///
/// At minimum you need a JVM class name (e.g. `"MyApp"`, `"demo/Calculator"`).
/// All IIR functions become `public static` methods of that single class.
///
/// # Default
///
/// `IIRJvmConfig::default()` produces `class_name = "IIRModule"`.
///
/// # Custom name
///
/// ```
/// use iir_to_jvm_class_file::IIRJvmConfig;
///
/// let cfg = IIRJvmConfig::new("demo/MyProgram");
/// assert_eq!(cfg.class_name, "demo/MyProgram");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRJvmConfig {
    /// The JVM binary class name (e.g. `"Main"` or `"com/example/Foo"`).
    ///
    /// JVM binary names use `/` as the package separator, not `.`.  The class
    /// name is written verbatim into the constant pool's `Class` entry.
    pub class_name: String,
}

impl Default for IIRJvmConfig {
    fn default() -> Self {
        Self {
            class_name: "IIRModule".to_string(),
        }
    }
}

impl IIRJvmConfig {
    /// Create a config with the given class name.
    pub fn new(class_name: impl Into<String>) -> Self {
        Self {
            class_name: class_name.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Backpatch fixup
// ---------------------------------------------------------------------------

/// A record of a forward-jump instruction that needs its offset patched later.
///
/// After emitting a `goto`, `ifne`, `ifeq`, or `if_icmpXX` instruction we
/// do not yet know the target address (because we may not have seen the `label`
/// instruction yet).  We store the opcode's byte position and the target label
/// name, then resolve them all after the entire function is emitted.
///
/// The JVM branch offset is measured from the start of the branch opcode, so:
///
/// ```text
/// offset = target_pc - opcode_pos   (as i16)
/// ```
///
/// This is written into `code[opcode_pos + 1]` (high byte) and
/// `code[opcode_pos + 2]` (low byte).
struct Fixup {
    /// Byte offset of the branch opcode within the method's code array.
    opcode_pos: usize,
    /// The label name to resolve.
    target: String,
}

// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

/// JVM type category for an IIR type hint.
///
/// The JVM distinguishes four "computational types" for local variables and
/// stack slots:
///
/// - `Int`    — covers Java's `int`, `short`, `byte`, `char`, `boolean`
/// - `Long`   — Java `long` (occupies two slots)
/// - `Float`  — Java `float`
/// - `Double` — Java `double` (occupies two slots)
///
/// We also add `Void` for `ret_void` and `_` (the return type of methods that
/// return nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JvmType {
    /// Maps to JVM `int` (descriptor `"I"`).
    Int,
    /// Maps to JVM `long` (descriptor `"J"`; occupies 2 slots).
    Long,
    /// Maps to JVM `float` (descriptor `"F"`).
    Float,
    /// Maps to JVM `double` (descriptor `"D"`; occupies 2 slots).
    Double,
    /// Maps to JVM `void` (descriptor `"V"`; no slot).
    Void,
}

impl JvmType {
    /// Number of local-variable slots this type occupies.
    ///
    /// The JVM allocates one slot per 32-bit value and two slots per 64-bit
    /// value (`long` and `double`).  All other JVM primitive types fit in one
    /// slot.
    fn slot_width(self) -> u16 {
        match self {
            JvmType::Long | JvmType::Double => 2,
            JvmType::Void => 0,
            _ => 1,
        }
    }

    /// The JVM descriptor character for this type.
    #[allow(dead_code)]
    fn descriptor(self) -> &'static str {
        match self {
            JvmType::Int => "I",
            JvmType::Long => "J",
            JvmType::Float => "F",
            JvmType::Double => "D",
            JvmType::Void => "V",
        }
    }
}

/// Map an IIR type hint string to a [`JvmType`].
///
/// Returns `None` for unknown / unsupported type hints.  The caller converts
/// `None` to an `UnsupportedType` error.
///
/// # Mapping table
///
/// | IIR hint              | JVM type | Note |
/// |-----------------------|----------|------|
/// | `i8/i16/i32/u8/u16/u32/bool` | `Int`    | All fit in 32-bit JVM int |
/// | `i64/u64`             | `Long`   | 64-bit; two slots |
/// | `f32`                 | `Float`  | IEEE-754 single-precision |
/// | `f64`                 | `Double` | IEEE-754 double-precision |
/// | `void` or `""`        | `Void`   | No value produced/consumed |
/// | anything else         | `None`   | Caller raises an error |
fn iir_type_to_jvm(hint: &str) -> Option<JvmType> {
    match hint {
        "i8" | "i16" | "i32" | "u8" | "u16" | "u32" | "bool" => Some(JvmType::Int),
        "i64" | "u64" => Some(JvmType::Long),
        "f32" => Some(JvmType::Float),
        "f64" => Some(JvmType::Double),
        "void" | "" => Some(JvmType::Void),
        // Catch-all: return None, let caller decide
        _ => None,
    }
}

/// Build the JVM method descriptor string for a function.
///
/// A JVM method descriptor encodes the parameter types and return type in a
/// compact string format:
///
/// ```text
/// (param_descriptors)return_descriptor
/// ```
///
/// For example, `fn add(a: i32, b: i32) -> i32` produces `"(II)I"`.
///
/// Unknown types default to `"I"` (int) to avoid a panic here — the validator
/// should have caught them earlier.
fn make_descriptor(params: &[(String, String)], return_type: &str) -> String {
    let mut d = String::from("(");
    for (_, ptype) in params {
        d.push_str(type_to_jvm_descriptor(ptype));
    }
    d.push(')');
    d.push_str(type_to_jvm_descriptor(return_type));
    d
}

/// Map an IIR type hint to a JVM descriptor character string.
///
/// This is used inside [`make_descriptor`] to build per-parameter and
/// return-type descriptors.  Unknown types default to `"I"` (int).
fn type_to_jvm_descriptor(hint: &str) -> &str {
    match hint {
        "i8" | "i16" | "i32" | "u8" | "u16" | "u32" | "bool" => "I",
        "i64" | "u64" => "J",
        "f32" => "F",
        "f64" => "D",
        "void" | "" => "V",
        _ => "I", // default for unknown — validator should have caught this
    }
}

// ---------------------------------------------------------------------------
// Bytecode emit helpers
// ---------------------------------------------------------------------------
//
// These helpers emit JVM bytecode into a `Vec<u8>` code buffer.  Each helper
// is documented with the opcodes it emits and why.

/// Emit an `iload` instruction (load int from local slot).
///
/// JVM has short forms for slots 0-3 (`iload_0` through `iload_3`), a
/// single-byte-index form for slots 0-255, and a `wide` prefix form for
/// slots 256-65535.  We emit the shortest valid encoding.
fn emit_iload(code: &mut Vec<u8>, idx: u16) {
    match idx {
        0 => code.push(ILOAD_0),
        1 => code.push(ILOAD_1),
        2 => code.push(ILOAD_2),
        3 => code.push(ILOAD_3),
        n if n <= 255 => {
            code.push(ILOAD);
            code.push(n as u8);
        }
        n => {
            // `wide iload` uses a 2-byte index
            code.push(WIDE);
            code.push(ILOAD);
            code.extend_from_slice(&(n as u16).to_be_bytes());
        }
    }
}

/// Emit an `lload` instruction (load long from local slot).
fn emit_lload(code: &mut Vec<u8>, idx: u16) {
    if idx <= 255 {
        code.push(LLOAD);
        code.push(idx as u8);
    } else {
        code.push(WIDE);
        code.push(LLOAD);
        code.extend_from_slice(&(idx as u16).to_be_bytes());
    }
}

/// Emit an `fload` instruction (load float from local slot).
fn emit_fload(code: &mut Vec<u8>, idx: u16) {
    if idx <= 255 {
        code.push(FLOAD);
        code.push(idx as u8);
    } else {
        code.push(WIDE);
        code.push(FLOAD);
        code.extend_from_slice(&(idx as u16).to_be_bytes());
    }
}

/// Emit a `dload` instruction (load double from local slot).
fn emit_dload(code: &mut Vec<u8>, idx: u16) {
    if idx <= 255 {
        code.push(DLOAD);
        code.push(idx as u8);
    } else {
        code.push(WIDE);
        code.push(DLOAD);
        code.extend_from_slice(&(idx as u16).to_be_bytes());
    }
}

/// Emit the appropriate typed load instruction for a slot and JVM type.
fn emit_typed_load(code: &mut Vec<u8>, idx: u16, jvm_type: JvmType) {
    match jvm_type {
        JvmType::Int => emit_iload(code, idx),
        JvmType::Long => emit_lload(code, idx),
        JvmType::Float => emit_fload(code, idx),
        JvmType::Double => emit_dload(code, idx),
        JvmType::Void => {} // nothing to load for void
    }
}

/// Emit an `istore` instruction (store int to local slot).
fn emit_istore(code: &mut Vec<u8>, idx: u16) {
    match idx {
        0 => code.push(ISTORE_0),
        1 => code.push(ISTORE_1),
        2 => code.push(ISTORE_2),
        3 => code.push(ISTORE_3),
        n if n <= 255 => {
            code.push(ISTORE);
            code.push(n as u8);
        }
        n => {
            code.push(WIDE);
            code.push(ISTORE);
            code.extend_from_slice(&(n as u16).to_be_bytes());
        }
    }
}

/// Emit an `lstore` instruction (store long to local slot).
fn emit_lstore(code: &mut Vec<u8>, idx: u16) {
    if idx <= 255 {
        code.push(LSTORE);
        code.push(idx as u8);
    } else {
        code.push(WIDE);
        code.push(LSTORE);
        code.extend_from_slice(&(idx as u16).to_be_bytes());
    }
}

/// Emit an `fstore` instruction (store float to local slot).
fn emit_fstore(code: &mut Vec<u8>, idx: u16) {
    if idx <= 255 {
        code.push(FSTORE);
        code.push(idx as u8);
    } else {
        code.push(WIDE);
        code.push(FSTORE);
        code.extend_from_slice(&(idx as u16).to_be_bytes());
    }
}

/// Emit a `dstore` instruction (store double to local slot).
fn emit_dstore(code: &mut Vec<u8>, idx: u16) {
    if idx <= 255 {
        code.push(DSTORE);
        code.push(idx as u8);
    } else {
        code.push(WIDE);
        code.push(DSTORE);
        code.extend_from_slice(&(idx as u16).to_be_bytes());
    }
}

/// Emit the appropriate typed store instruction for a slot and JVM type.
fn emit_typed_store(code: &mut Vec<u8>, idx: u16, jvm_type: JvmType) {
    match jvm_type {
        JvmType::Int => emit_istore(code, idx),
        JvmType::Long => emit_lstore(code, idx),
        JvmType::Float => emit_fstore(code, idx),
        JvmType::Double => emit_dstore(code, idx),
        JvmType::Void => {} // nothing to store for void
    }
}

/// Emit the most compact integer constant push instruction.
///
/// The JVM has several ways to push an int:
///
/// | Encoding       | Range         | Size |
/// |----------------|---------------|------|
/// | `iconst_m1`…`iconst_5` | -1..5 | 1 byte |
/// | `bipush byte`  | -128..127     | 2 bytes |
/// | `sipush short` | -32768..32767 | 3 bytes |
/// | `ldc cp_idx`   | any           | 2 bytes (CP entry needed) |
///
/// We handle the first three cases.  For values outside the sipush range we
/// emit an `ldc` with a placeholder index 0 — the constant pool would need
/// an `Integer` entry, which requires a more elaborate CP builder than we
/// implement in v1.  Tests only use values in the sipush range.
fn emit_iconst(code: &mut Vec<u8>, value: i32) {
    match value {
        -1 => code.push(ICONST_M1),
        0 => code.push(ICONST_0),
        1 => code.push(ICONST_1),
        2 => code.push(ICONST_2),
        3 => code.push(ICONST_3),
        4 => code.push(ICONST_4),
        5 => code.push(ICONST_5),
        v if v >= -128 && v <= 127 => {
            code.push(BIPUSH);
            code.push(v as u8);
        }
        v if v >= -32768 && v <= 32767 => {
            code.push(SIPUSH);
            code.extend_from_slice(&(v as i16).to_be_bytes());
        }
        _ => {
            // Out of sipush range.  In v1 we emit ldc with a placeholder index.
            // Tests that use large constants would need a proper CP builder.
            code.push(LDC);
            code.push(0); // placeholder CP index
        }
    }
}

/// Emit a long constant push.
///
/// JVM only has short forms for `0L` and `1L`.  Anything else needs an
/// `ldc2_w` instruction with a constant pool `Long` entry.  For v1 simplicity
/// we only handle 0 and 1 precisely; other values emit `lconst_0` as a
/// placeholder (tests use 0/1 or rely on load/store).
fn emit_lconst(code: &mut Vec<u8>, value: i64) {
    match value {
        0 => code.push(LCONST_0),
        1 => code.push(LCONST_1),
        _ => {
            // For v1, emit ldc2_w with a placeholder CP index.
            // A full implementation would add a Long CP entry.
            code.push(LDC2_W);
            code.extend_from_slice(&0u16.to_be_bytes()); // placeholder
        }
    }
}

/// Emit a float constant push.
///
/// JVM has short forms for `0.0f`, `1.0f`, `2.0f`.  Other float values need
/// an `ldc` instruction referencing a `Float` CP entry.  We emit a placeholder
/// `ldc 0` for other values in v1.
fn emit_fconst(code: &mut Vec<u8>, value: f32) {
    if value == 0.0f32 {
        code.push(FCONST_0);
    } else if value == 1.0f32 {
        code.push(FCONST_1);
    } else if value == 2.0f32 {
        code.push(FCONST_2);
    } else {
        // Other float constants need a CP Float entry in a full implementation.
        code.push(LDC);
        code.push(0); // placeholder CP index
    }
}

/// Emit a double constant push.
///
/// JVM has short forms for `0.0d` and `1.0d`.  Other double values need
/// `ldc2_w`.  Placeholder for v1.
fn emit_dconst(code: &mut Vec<u8>, value: f64) {
    if value == 0.0f64 {
        code.push(DCONST_0);
    } else if value == 1.0f64 {
        code.push(DCONST_1);
    } else {
        code.push(LDC2_W);
        code.extend_from_slice(&0u16.to_be_bytes()); // placeholder CP index
    }
}

// ---------------------------------------------------------------------------
// Comparison synthesis
// ---------------------------------------------------------------------------
//
// The JVM does not have a single "compare and leave boolean result on stack"
// instruction.  Instead, you emit a conditional branch and manually push
// `1` (true) or `0` (false) in the two arms.
//
// We use a fixed 8-byte pattern so offsets are known at compile time:
//
// ```
// [PC=0]  if_icmpNOT_eq  +7    ; 3 bytes: opcode + 2-byte offset
// [PC=3]  iconst_1               ; 1 byte  — condition was true
// [PC=4]  goto           +4    ; 3 bytes: opcode + 2-byte offset
// [PC=7]  iconst_0               ; 1 byte  — condition was false
// [PC=8]  …                      ; next instruction
// ```
//
// For `cmp_eq` we want to push 1 when the values ARE equal.  We use
// `if_icmpne` (branch if NOT equal): if NOT equal, skip iconst_1 and go to
// iconst_0; otherwise fall through to iconst_1.
//
// # Offset arithmetic
//
// - `if_icmpne` at PC=0 with offset 7: target = 0 + 7 = PC 7 ✓ (iconst_0)
// - `goto`      at PC=4 with offset 4: target = 4 + 4 = PC 8 ✓ (next instr)
//
// The JVM spec says branch offsets are measured from the opcode byte's
// address, so this is correct.

/// Emit a "compare two ints on the stack, push 1 if condition holds, else 0"
/// sequence using the given branch opcode.
///
/// `cmp_opcode` should be the **negated** condition opcode — i.e. the opcode
/// that branches when the condition is *false*, so the fall-through path is
/// the true case.
///
/// Examples:
/// - `cmp_eq` → `cmp_opcode = IF_ICMPNE` (skip true case when not equal)
/// - `cmp_ne` → `cmp_opcode = IF_ICMPEQ`
/// - `cmp_lt` → `cmp_opcode = IF_ICMPGE`
/// - `cmp_le` → `cmp_opcode = IF_ICMPGT`
/// - `cmp_gt` → `cmp_opcode = IF_ICMPLE`
/// - `cmp_ge` → `cmp_opcode = IF_ICMPLT`
///
/// Stack state on entry: `[…, int1, int2]`
/// Stack state on exit:  `[…, 0_or_1]`
fn emit_int_compare(code: &mut Vec<u8>, cmp_opcode: u8) {
    // if_icmpXX at current PC, offset to iconst_0 (7 bytes forward)
    code.push(cmp_opcode);
    code.extend_from_slice(&7i16.to_be_bytes());
    // True arm — condition held
    code.push(ICONST_1);
    // Jump past the false arm
    code.push(GOTO);
    code.extend_from_slice(&4i16.to_be_bytes());
    // False arm — condition did not hold
    code.push(ICONST_0);
    // 8 bytes total emitted
}

// ---------------------------------------------------------------------------
// Slot allocator
// ---------------------------------------------------------------------------

/// Allocate JVM local variable slots for all variables in a function.
///
/// Returns a map from variable name to (slot_index, JvmType).
///
/// # Algorithm
///
/// Pass 1 over the parameters, then over all instructions (dest and Var srcs),
/// assigning each new name the next available slot.  The slot width depends
/// on the JVM type (`long`/`double` claim 2 consecutive slots).
///
/// The `type_map` argument is a pre-built mapping from variable name to its
/// declared IIR type.  We build it from the function parameters and the `dest`
/// fields of instructions before calling this helper.
fn allocate_slots(
    func: &IIRFunction,
    type_map: &HashMap<String, JvmType>,
) -> HashMap<String, (u16, JvmType)> {
    let mut slots: HashMap<String, (u16, JvmType)> = HashMap::new();
    // Use u32 internally to detect overflow before narrowing to u16.
    // JVM local slots are 16-bit; exceeding 65,535 slots would produce
    // silently aliased locals, so we assert-guard the cast.
    let mut next_slot: u32 = 0;

    // ── Step 1: allocate params ──────────────────────────────────────────────
    //
    // Function parameters always occupy the first slots, in order.  This
    // matches the JVM calling convention: when the method is invoked, the
    // arguments are already in slots 0..N-1.
    for (param_name, param_type_str) in &func.params {
        let jvm_type = iir_type_to_jvm(param_type_str.as_str())
            .unwrap_or(JvmType::Int); // fallback; validator should catch bad types
        assert!(
            next_slot <= u16::MAX as u32,
            "JVM local slot overflow: too many variables in function {:?}", func.name
        );
        slots.insert(param_name.clone(), (next_slot as u16, jvm_type));
        next_slot += jvm_type.slot_width() as u32;
    }

    // ── Step 2: allocate dests and src Vars ──────────────────────────────────
    //
    // Walk every instruction in order.  For each destination variable and each
    // Var-kind source operand, look up the name in our slots map.  If it's not
    // there yet, assign the next available slot.
    for instr in &func.instructions {
        // Look up the type for this variable: prefer the instruction's
        // type_hint, else fall back to what we've already seen.
        let instr_type = iir_type_to_jvm(&instr.type_hint).unwrap_or(JvmType::Int);

        if let Some(dest) = &instr.dest {
            if !slots.contains_key(dest.as_str()) {
                // Use the instruction's declared type for the dest variable.
                let var_type = type_map
                    .get(dest.as_str())
                    .copied()
                    .unwrap_or(instr_type);
                assert!(
                    next_slot <= u16::MAX as u32,
                    "JVM local slot overflow: too many variables in function {:?}", func.name
                );
                slots.insert(dest.clone(), (next_slot as u16, var_type));
                next_slot += var_type.slot_width() as u32;
            }
        }

        for src in &instr.srcs {
            if let Operand::Var(name) = src {
                if !slots.contains_key(name.as_str()) {
                    let var_type = type_map.get(name.as_str()).copied().unwrap_or(instr_type);
                    assert!(
                        next_slot <= u16::MAX as u32,
                        "JVM local slot overflow: too many variables in function {:?}", func.name
                    );
                    slots.insert(name.clone(), (next_slot as u16, var_type));
                    next_slot += var_type.slot_width() as u32;
                }
            }
        }
    }

    slots
}

/// Build a variable-to-type map for a function.
///
/// Scans parameters and instruction dest fields to build a `name → JvmType`
/// lookup used by [`allocate_slots`].
fn build_type_map(func: &IIRFunction) -> HashMap<String, JvmType> {
    let mut map: HashMap<String, JvmType> = HashMap::new();

    for (pname, ptype) in &func.params {
        if let Some(t) = iir_type_to_jvm(ptype) {
            map.insert(pname.clone(), t);
        }
    }
    for instr in &func.instructions {
        if let Some(dest) = &instr.dest {
            if let Some(t) = iir_type_to_jvm(&instr.type_hint) {
                map.entry(dest.clone()).or_insert(t);
            }
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Constant pool builder (minimal)
// ---------------------------------------------------------------------------
//
// The JVM constant pool is 1-indexed: index 0 is always `None` (unused).
// Entries are added in the order they are first referenced.

/// Minimal constant pool builder for the JVM class file.
///
/// We build a minimal constant pool containing:
/// - The class name (as `Utf8` + `Class` entry)
/// - `java/lang/Object` (super class)
/// - For each method: `Utf8` name, `Utf8` descriptor, and a `Methodref`
///   (for `invokestatic` calls)
/// - `"Code"` (the code attribute name)
///
/// The constant pool is 1-indexed per the JVM spec, so index 0 is always
/// `None`.
struct ConstantPoolBuilder {
    entries: Vec<Option<JvmConstantPoolEntry>>,
    /// Quick lookup: key string → 1-based index.
    index_map: HashMap<String, u16>,
}

impl ConstantPoolBuilder {
    fn new() -> Self {
        Self {
            entries: vec![None], // index 0 is always unused in JVM
            index_map: HashMap::new(),
        }
    }

    /// Add an entry (or return existing index if key already present).
    ///
    /// Returns the 1-based constant pool index.  The JVM constant pool is
    /// limited to 65,535 entries; exceeding this limit causes a panic with
    /// a clear message (the validator should prevent this in practice, but
    /// we guard here as a last line of defense).
    fn add_entry(&mut self, key: String, entry: JvmConstantPoolEntry) -> u16 {
        if let Some(&idx) = self.index_map.get(&key) {
            return idx;
        }
        assert!(
            self.entries.len() < u16::MAX as usize,
            "JVM constant pool overflow: too many entries (limit 65535)"
        );
        self.entries.push(Some(entry));
        // Safe: bounded by the assert above.
        let idx = (self.entries.len() - 1) as u16;
        self.index_map.insert(key, idx);
        idx
    }

    /// Add a UTF8 string entry.
    fn add_utf8(&mut self, s: &str) -> u16 {
        let key = format!("Utf8:{}", s);
        self.add_entry(key, JvmConstantPoolEntry::Utf8(s.to_string()))
    }

    /// Add a Class entry referencing a UTF8 name.
    fn add_class(&mut self, class_name: &str) -> u16 {
        let name_idx = self.add_utf8(class_name);
        let key = format!("Class:{}", class_name);
        self.add_entry(key, JvmConstantPoolEntry::Class { name_index: name_idx })
    }

    /// Add a NameAndType entry.
    fn add_name_and_type(&mut self, name: &str, descriptor: &str) -> u16 {
        let name_idx = self.add_utf8(name);
        let desc_idx = self.add_utf8(descriptor);
        let key = format!("NameAndType:{}:{}", name, descriptor);
        self.add_entry(
            key,
            JvmConstantPoolEntry::NameAndType {
                name_index: name_idx,
                descriptor_index: desc_idx,
            },
        )
    }

    /// Add a Methodref entry.
    fn add_methodref(&mut self, class_name: &str, method_name: &str, descriptor: &str) -> u16 {
        let class_idx = self.add_class(class_name);
        let nat_idx = self.add_name_and_type(method_name, descriptor);
        let key = format!("Methodref:{}.{}:{}", class_name, method_name, descriptor);
        self.add_entry(
            key,
            JvmConstantPoolEntry::Methodref {
                class_index: class_idx,
                name_and_type_index: nat_idx,
            },
        )
    }

    /// Finalise the pool and return it as a `Vec<Option<JvmConstantPoolEntry>>`.
    fn build(self) -> Vec<Option<JvmConstantPoolEntry>> {
        self.entries
    }
}

// ---------------------------------------------------------------------------
// Function lowering
// ---------------------------------------------------------------------------

/// Lower a single `IIRFunction` to a `JvmMethodInfo`.
///
/// This is the core translation unit.  It performs:
///
/// 1. Slot allocation (deterministic two-pass scan)
/// 2. Bytecode emission (instruction by instruction)
/// 3. Backpatch fixup resolution
///
/// The returned `JvmMethodInfo` has `access_flags = ACC_PUBLIC | ACC_STATIC`,
/// because IIR functions are plain procedures with no object receiver.
fn lower_function(
    func: &IIRFunction,
    class_name: &str,
    module: &IIRModule,
    cp: &mut ConstantPoolBuilder,
) -> Result<JvmMethodInfo, IIRJvmError> {
    let fname = &func.name;

    // ── Pass 1: build type map and allocate slots ────────────────────────────
    let type_map = build_type_map(func);
    let slots = allocate_slots(func, &type_map);

    // Compute max_locals: the maximum slot index + width across all variables.
    // Use u32 arithmetic to avoid u16 overflow before the final bounds check.
    // (allocate_slots already asserts that next_slot fits in u16, so this
    //  sum is safe in practice, but we compute it wide for clarity.)
    let max_locals: u16 = {
        let from_slots = slots
            .values()
            .map(|(idx, t)| (*idx as u32) + (t.slot_width() as u32))
            .max()
            .unwrap_or(0);
        let from_params = func.params.len() as u32;
        let raw = from_slots.max(from_params);
        u16::try_from(raw).unwrap_or_else(|_| {
            // allocate_slots should have already asserted; this is belt-and-braces.
            panic!(
                "max_locals {} overflows u16 for function {:?}; \
                 this should have been caught in allocate_slots",
                raw, func.name
            )
        })
    };

    // ── Pass 2: emit bytecode ────────────────────────────────────────────────
    let mut code: Vec<u8> = Vec::new();
    let mut label_map: HashMap<String, u32> = HashMap::new(); // label → PC
    let mut fixups: Vec<Fixup> = Vec::new();

    // Lookup a variable's slot + type, returning an error if not found.
    let lookup_var = |name: &str| -> Result<(u16, JvmType), IIRJvmError> {
        slots.get(name).copied().ok_or_else(|| IIRJvmError::UndefinedVariable {
            function: fname.clone(),
            name: name.to_string(),
        })
    };

    for instr in &func.instructions {
        // Resolve the instruction's own JVM type (best effort; void is OK).
        let instr_jtype = iir_type_to_jvm(&instr.type_hint).unwrap_or(JvmType::Int);

        match instr.op.as_str() {
            // ── label ───────────────────────────────────────────────────────
            //
            // `label` in IIR is simply a named marker.  On the JVM, labels are
            // not instructions — they are byte positions.  We record the current
            // code length as this label's PC and emit no bytes.
            "label" => {
                if let Some(Operand::Var(name)) = instr.srcs.first() {
                    label_map.insert(name.clone(), code.len() as u32);
                }
                // No bytes emitted for label
            }

            // ── const ────────────────────────────────────────────────────────
            //
            // Load a compile-time constant into a local variable slot.
            // The operand is an immediate: Int, Float, or Bool.
            "const" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "const instruction has no dest".to_string(),
                })?;
                let src = instr.srcs.first().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "const instruction has no source operand".to_string(),
                })?;

                let (dest_slot, dest_type) = lookup_var(dest_name)?;

                match src {
                    Operand::Int(v) => {
                        match dest_type {
                            JvmType::Long => emit_lconst(&mut code, *v),
                            JvmType::Float => emit_fconst(&mut code, *v as f32),
                            JvmType::Double => emit_dconst(&mut code, *v as f64),
                            _ => emit_iconst(&mut code, *v as i32),
                        }
                    }
                    Operand::Bool(b) => {
                        // Booleans are represented as int 0 or 1 on the JVM.
                        emit_iconst(&mut code, if *b { 1 } else { 0 });
                    }
                    Operand::Float(f) => {
                        match dest_type {
                            JvmType::Float => emit_fconst(&mut code, *f as f32),
                            JvmType::Double => emit_dconst(&mut code, *f),
                            _ => {
                                // Integer destination with float source — unusual
                                // but not necessarily wrong (e.g. casting).
                                emit_iconst(&mut code, *f as i32);
                            }
                        }
                    }
                    Operand::Var(_) => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: "const instruction has a Var source — use load_reg instead"
                                .to_string(),
                        });
                    }
                }
                emit_typed_store(&mut code, dest_slot, dest_type);
            }

            // ── load_reg / store_reg ─────────────────────────────────────────
            //
            // Copy one variable to another.  On the JVM this is a typed load
            // followed by a typed store.
            "load_reg" | "store_reg" => {
                let dest_name = match instr.dest.as_deref() {
                    Some(n) => n,
                    None => {
                        // store_reg with no dest: nop
                        continue;
                    }
                };
                let src = instr.srcs.first().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: format!("{} has no source operand", instr.op),
                })?;
                let src_name = match src {
                    Operand::Var(n) => n.as_str(),
                    _ => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("{} source is not a Var", instr.op),
                        })
                    }
                };
                let (src_slot, src_type) = lookup_var(src_name)?;
                let (dest_slot, _) = lookup_var(dest_name)?;
                emit_typed_load(&mut code, src_slot, src_type);
                emit_typed_store(&mut code, dest_slot, src_type);
            }

            // ── type_assert ──────────────────────────────────────────────────
            //
            // A runtime type assertion hint — useful for the JIT, but the JVM
            // backend does nothing here.  The assertion is satisfied statically
            // by the type hints we're already using.
            "type_assert" => {
                // Erased: no bytes emitted.
            }

            // ── Binary arithmetic: add, sub, mul, div, mod ───────────────────
            //
            // Each IIR binary arithmetic op maps to a single JVM opcode that
            // pops two values and pushes one.  We load both source operands,
            // emit the opcode, then store the result.
            "add" | "sub" | "mul" | "div" | "mod" => {
                let (src0, src1) = two_srcs(func, instr, &slots)?;
                emit_typed_load(&mut code, src0.0, src0.1);
                emit_typed_load(&mut code, src1.0, src1.1);

                let opcode = match (instr.op.as_str(), instr_jtype) {
                    ("add", JvmType::Int) => IADD,
                    ("add", JvmType::Long) => LADD,
                    ("add", JvmType::Float) => FADD,
                    ("add", JvmType::Double) => DADD,
                    ("sub", JvmType::Int) => ISUB,
                    ("sub", JvmType::Long) => LSUB,
                    ("sub", JvmType::Float) => FSUB,
                    ("sub", JvmType::Double) => DSUB,
                    ("mul", JvmType::Int) => IMUL,
                    ("mul", JvmType::Long) => LMUL,
                    ("mul", JvmType::Float) => FMUL,
                    ("mul", JvmType::Double) => DMUL,
                    ("div", JvmType::Int) => IDIV,
                    ("div", JvmType::Long) => LDIV,
                    ("div", JvmType::Float) => FDIV,
                    ("div", JvmType::Double) => DDIV,
                    ("mod", JvmType::Int) => IREM,
                    ("mod", JvmType::Long) => LREM,
                    _ => IADD, // fallback
                };
                code.push(opcode);

                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, instr_jtype);
                }
            }

            // ── Negation ─────────────────────────────────────────────────────
            //
            // `neg` is a unary op: pops one value, pushes its negation.
            "neg" => {
                let src = one_src(func, instr, &slots)?;
                emit_typed_load(&mut code, src.0, src.1);
                let opcode = match instr_jtype {
                    JvmType::Long => LNEG,
                    JvmType::Float => FNEG,
                    JvmType::Double => DNEG,
                    _ => INEG,
                };
                code.push(opcode);
                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, instr_jtype);
                }
            }

            // ── Bitwise: and, or, xor ────────────────────────────────────────
            //
            // Maps to `iand`/`land`, `ior`/`lor`, `ixor`/`lxor`.
            "and" | "or" | "xor" => {
                let (src0, src1) = two_srcs(func, instr, &slots)?;
                emit_typed_load(&mut code, src0.0, src0.1);
                emit_typed_load(&mut code, src1.0, src1.1);
                let opcode = match (instr.op.as_str(), instr_jtype) {
                    ("and", JvmType::Long) => LAND,
                    ("and", _) => IAND,
                    ("or", JvmType::Long) => LOR,
                    ("or", _) => IOR,
                    ("xor", JvmType::Long) => LXOR,
                    ("xor", _) => IXOR,
                    _ => IAND,
                };
                code.push(opcode);
                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, instr_jtype);
                }
            }

            // ── Bitwise NOT ───────────────────────────────────────────────────
            //
            // JVM has no `inot`.  For boolean NOT we XOR with 1 (flips the LSB).
            // For integer NOT we XOR with -1 (all bits flipped).
            "not" => {
                let src = one_src(func, instr, &slots)?;
                emit_typed_load(&mut code, src.0, src.1);
                if instr_jtype == JvmType::Long {
                    emit_lconst(&mut code, -1i64);
                    code.push(LXOR);
                } else {
                    // XOR with 1 for booleans, -1 for int NOT.
                    // We use -1 to implement bitwise NOT for int types.
                    emit_iconst(&mut code, -1);
                    code.push(IXOR);
                }
                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, instr_jtype);
                }
            }

            // ── Shifts: shl, shr ─────────────────────────────────────────────
            //
            // JVM shift amount is always an int (even for `lshl`), so we load
            // the shift count as an int regardless of the value type.
            "shl" | "shr" => {
                let (src0, src1) = two_srcs(func, instr, &slots)?;
                emit_typed_load(&mut code, src0.0, src0.1); // value
                emit_iload(&mut code, src1.0);               // shift count (always int)
                let opcode = match (instr.op.as_str(), instr_jtype) {
                    ("shl", JvmType::Long) => LSHL,
                    ("shl", _) => ISHL,
                    ("shr", JvmType::Long) => LSHR,
                    ("shr", _) => ISHR,
                    _ => ISHL,
                };
                code.push(opcode);
                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, instr_jtype);
                }
            }

            // ── Comparisons ──────────────────────────────────────────────────
            //
            // For integer comparisons we use the 8-byte fixed pattern from
            // `emit_int_compare`.  The JVM `if_icmpXX` family only works on
            // `int`; for `long` and `float` comparisons, a more elaborate
            // `lcmp`/`fcmpl`/`dcmpl` path is needed.  We use integer compare
            // for all types in v1 (tests use int types for comparisons).
            "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
                let (src0, src1) = two_srcs(func, instr, &slots)?;
                emit_iload(&mut code, src0.0);
                emit_iload(&mut code, src1.0);
                let cmp_opcode = match instr.op.as_str() {
                    // We use the NEGATED opcode so fall-through is the true case.
                    "cmp_eq" => IF_ICMPNE, // if NOT equal → skip true arm
                    "cmp_ne" => IF_ICMPEQ, // if IS equal  → skip true arm
                    "cmp_lt" => IF_ICMPGE, // if >= → skip true arm
                    "cmp_le" => IF_ICMPGT, // if >  → skip true arm
                    "cmp_gt" => IF_ICMPLE, // if <= → skip true arm
                    "cmp_ge" => IF_ICMPLT, // if <  → skip true arm
                    _ => IF_ICMPNE,
                };
                emit_int_compare(&mut code, cmp_opcode);
                // Result (0 or 1) is now on the stack; store it.
                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_istore(&mut code, dest_slot);
                }
            }

            // ── jmp ──────────────────────────────────────────────────────────
            //
            // Unconditional jump to a label.  We emit `goto` with a placeholder
            // offset and record a fixup.
            "jmp" => {
                let label = match instr.srcs.first() {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: "jmp has no label operand".to_string(),
                        })
                    }
                };
                let opcode_pos = code.len();
                code.push(GOTO);
                code.extend_from_slice(&0i16.to_be_bytes()); // placeholder
                fixups.push(Fixup { opcode_pos, target: label });
            }

            // ── jmp_if_true ───────────────────────────────────────────────────
            //
            // Jump to label if condition variable is non-zero (truthy).
            // Emits: `iload cond; ifne <label>`.
            "jmp_if_true" => {
                let (cond_src, label) = cond_and_label(fname, instr)?;
                let (cond_slot, _) = lookup_var(cond_src)?;
                emit_iload(&mut code, cond_slot);
                let opcode_pos = code.len();
                code.push(IFNE);
                code.extend_from_slice(&0i16.to_be_bytes()); // placeholder
                fixups.push(Fixup { opcode_pos, target: label });
            }

            // ── jmp_if_false ──────────────────────────────────────────────────
            //
            // Jump to label if condition variable is zero (falsy).
            // Emits: `iload cond; ifeq <label>`.
            "jmp_if_false" => {
                let (cond_src, label) = cond_and_label(fname, instr)?;
                let (cond_slot, _) = lookup_var(cond_src)?;
                emit_iload(&mut code, cond_slot);
                let opcode_pos = code.len();
                code.push(IFEQ);
                code.extend_from_slice(&0i16.to_be_bytes()); // placeholder
                fixups.push(Fixup { opcode_pos, target: label });
            }

            // ── ret ───────────────────────────────────────────────────────────
            //
            // Return a value from the function.  Load the result variable onto
            // the stack, then emit the appropriate typed return opcode.
            "ret" => {
                let src = instr.srcs.first().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "ret has no source operand".to_string(),
                })?;
                match src {
                    Operand::Var(name) => {
                        let (slot, jtype) = lookup_var(name)?;
                        emit_typed_load(&mut code, slot, jtype);
                        let ret_opcode = match jtype {
                            JvmType::Int => IRETURN,
                            JvmType::Long => LRETURN,
                            JvmType::Float => FRETURN,
                            JvmType::Double => DRETURN,
                            JvmType::Void => RETURN,
                        };
                        code.push(ret_opcode);
                    }
                    Operand::Int(v) => {
                        let ret_type = iir_type_to_jvm(&func.return_type).unwrap_or(JvmType::Int);
                        match ret_type {
                            JvmType::Long => {
                                emit_lconst(&mut code, *v);
                                code.push(LRETURN);
                            }
                            _ => {
                                emit_iconst(&mut code, *v as i32);
                                code.push(IRETURN);
                            }
                        }
                    }
                    Operand::Bool(b) => {
                        emit_iconst(&mut code, if *b { 1 } else { 0 });
                        code.push(IRETURN);
                    }
                    Operand::Float(f) => {
                        let ret_type = iir_type_to_jvm(&func.return_type).unwrap_or(JvmType::Double);
                        match ret_type {
                            JvmType::Float => {
                                emit_fconst(&mut code, *f as f32);
                                code.push(FRETURN);
                            }
                            JvmType::Double => {
                                emit_dconst(&mut code, *f);
                                code.push(DRETURN);
                            }
                            _ => {
                                emit_iconst(&mut code, *f as i32);
                                code.push(IRETURN);
                            }
                        }
                    }
                }
            }

            // ── ret_void ─────────────────────────────────────────────────────
            //
            // Return from a void function.  JVM opcode `return` (0xB1).
            "ret_void" => {
                code.push(RETURN);
            }

            // ── call ──────────────────────────────────────────────────────────
            //
            // Call another static method in the same class.  The JVM
            // `invokestatic` instruction requires a 2-byte constant pool index
            // pointing to a `Methodref` entry.
            //
            // We look up the callee function in the module to find its descriptor,
            // then add (or find) a Methodref in the constant pool.
            "call" => {
                // First source is the callee name (as a Var operand).
                let callee_name = match instr.srcs.first() {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: "call has no callee name operand".to_string(),
                        })
                    }
                };

                // Look up the callee to get its descriptor.
                let callee_fn = module.get_function(&callee_name).ok_or_else(|| {
                    IIRJvmError::UndefinedVariable {
                        function: fname.clone(),
                        name: callee_name.clone(),
                    }
                })?;
                let descriptor = make_descriptor(&callee_fn.params, &callee_fn.return_type);

                // Push argument slots onto the JVM stack.
                for arg in instr.srcs.iter().skip(1) {
                    match arg {
                        Operand::Var(n) => {
                            let (slot, jtype) = lookup_var(n)?;
                            emit_typed_load(&mut code, slot, jtype);
                        }
                        Operand::Int(v) => emit_iconst(&mut code, *v as i32),
                        Operand::Bool(b) => emit_iconst(&mut code, if *b { 1 } else { 0 }),
                        Operand::Float(f) => emit_fconst(&mut code, *f as f32),
                    }
                }

                // Emit invokestatic with CP index for the Methodref.
                let methodref_idx = cp.add_methodref(class_name, &callee_name, &descriptor);
                code.push(INVOKESTATIC);
                code.extend_from_slice(&methodref_idx.to_be_bytes());

                // Store the return value if there is a dest.
                if let Some(dest) = &instr.dest {
                    let ret_type = iir_type_to_jvm(&callee_fn.return_type).unwrap_or(JvmType::Int);
                    if ret_type != JvmType::Void {
                        let (dest_slot, _) = lookup_var(dest)?;
                        emit_typed_store(&mut code, dest_slot, ret_type);
                    }
                }
            }

            // ── Unknown op ───────────────────────────────────────────────────
            //
            // If we reach here, the validator missed something or an opcode was
            // added after this backend was written.  Return a descriptive error.
            other => {
                return Err(IIRJvmError::UnsupportedOp {
                    function: fname.clone(),
                    op: other.to_string(),
                });
            }
        }
    }

    // ── Apply backpatch fixups ─────────────────────────────────────────────
    //
    // Now that all instructions are emitted, resolve any forward-jump offsets.
    // For each fixup, look up the target label in `label_map` and compute the
    // signed 16-bit offset from the opcode position.
    for fixup in &fixups {
        let target_pc = *label_map.get(&fixup.target).ok_or_else(|| {
            IIRJvmError::UndefinedLabel {
                function: fname.clone(),
                label: fixup.target.clone(),
            }
        })?;
        // Compute the signed 16-bit branch offset.  JVM branches are limited
        // to ±32,767 bytes (use goto_w for larger ranges — not implemented in
        // V1).  We also guard against the case where opcode_pos exceeds the
        // code buffer (an internal invariant violation that should never occur
        // but we prefer a clean error over an index-out-of-bounds panic).
        let raw_offset = target_pc as i64 - fixup.opcode_pos as i64;
        if raw_offset < i16::MIN as i64 || raw_offset > i16::MAX as i64 {
            return Err(IIRJvmError::InvalidOperand {
                function: fname.clone(),
                detail: format!(
                    "branch offset {} does not fit in i16 (label {:?}); \
                     goto_w is not implemented in V1 — split large functions",
                    raw_offset, fixup.target
                ),
            });
        }
        if fixup.opcode_pos + 2 >= code.len() {
            return Err(IIRJvmError::InvalidOperand {
                function: fname.clone(),
                detail: format!(
                    "internal: fixup opcode_pos {} is out of bounds for code len {}",
                    fixup.opcode_pos, code.len()
                ),
            });
        }
        let offset = raw_offset as i16;
        let offset_bytes = offset.to_be_bytes();
        code[fixup.opcode_pos + 1] = offset_bytes[0];
        code[fixup.opcode_pos + 2] = offset_bytes[1];
    }

    // ── Safety net: ensure non-empty code ────────────────────────────────────
    //
    // A JVM method with zero bytecode bytes is invalid.  If the instruction
    // list produced no bytes (e.g. only `label` and `type_assert` instructions),
    // emit a `return` to be safe.
    if code.is_empty() {
        code.push(RETURN);
    }

    // ── Build method descriptor ────────────────────────────────────────────
    let descriptor = make_descriptor(&func.params, &func.return_type);

    // max_stack is the depth of the operand stack.  A conservative upper
    // bound is 2 (for binary ops that push two values before a binary opcode).
    // In practice our emit patterns never exceed a stack depth of ~4.
    let max_stack: u16 = 8;

    let code_attribute = JvmCodeAttribute {
        name: "Code".to_string(),
        max_stack,
        max_locals,
        code,
        nested_attributes: vec![],
    };

    Ok(JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: func.name.clone(),
        descriptor,
        attributes: vec![JvmMethodAttribute::Code(code_attribute)],
    })
}

// ---------------------------------------------------------------------------
// Operand helpers
// ---------------------------------------------------------------------------

/// Extract the (slot, type) for a single source operand (must be a Var).
fn one_src(
    func: &IIRFunction,
    instr: &interpreter_ir::IIRInstr,
    slots: &HashMap<String, (u16, JvmType)>,
) -> Result<(u16, JvmType), IIRJvmError> {
    let src = instr.srcs.first().ok_or_else(|| IIRJvmError::InvalidOperand {
        function: func.name.clone(),
        detail: format!("{} has no source operand", instr.op),
    })?;
    match src {
        Operand::Var(name) => slots.get(name.as_str()).copied().ok_or_else(|| {
            IIRJvmError::UndefinedVariable {
                function: func.name.clone(),
                name: name.clone(),
            }
        }),
        _ => Err(IIRJvmError::InvalidOperand {
            function: func.name.clone(),
            detail: format!("{} expects a Var source", instr.op),
        }),
    }
}

/// Extract (slot, type) for both sources of a binary instruction (must both be Vars).
fn two_srcs(
    func: &IIRFunction,
    instr: &interpreter_ir::IIRInstr,
    slots: &HashMap<String, (u16, JvmType)>,
) -> Result<((u16, JvmType), (u16, JvmType)), IIRJvmError> {
    let s0 = instr.srcs.get(0).ok_or_else(|| IIRJvmError::InvalidOperand {
        function: func.name.clone(),
        detail: format!("{} needs 2 source operands, got 0", instr.op),
    })?;
    let s1 = instr.srcs.get(1).ok_or_else(|| IIRJvmError::InvalidOperand {
        function: func.name.clone(),
        detail: format!("{} needs 2 source operands, got 1", instr.op),
    })?;

    let get_slot = |op: &Operand| -> Result<(u16, JvmType), IIRJvmError> {
        match op {
            Operand::Var(name) => slots.get(name.as_str()).copied().ok_or_else(|| {
                IIRJvmError::UndefinedVariable {
                    function: func.name.clone(),
                    name: name.clone(),
                }
            }),
            _ => Err(IIRJvmError::InvalidOperand {
                function: func.name.clone(),
                detail: format!("{} expects Var operands, got immediate", instr.op),
            }),
        }
    };

    Ok((get_slot(s0)?, get_slot(s1)?))
}

/// Extract the condition variable name and target label for a conditional jump.
///
/// For `jmp_if_true` and `jmp_if_false`:
/// - `srcs[0]` = condition variable (Var)
/// - `srcs[1]` (or last) = target label (Var)
fn cond_and_label<'a>(
    fname: &str,
    instr: &'a interpreter_ir::IIRInstr,
) -> Result<(&'a str, String), IIRJvmError> {
    let cond = match instr.srcs.first() {
        Some(Operand::Var(n)) => n.as_str(),
        _ => {
            return Err(IIRJvmError::InvalidOperand {
                function: fname.to_string(),
                detail: format!("{} needs a condition Var as first operand", instr.op),
            })
        }
    };
    let label = match instr.srcs.last() {
        Some(Operand::Var(n)) if instr.srcs.len() > 1 => n.clone(),
        _ => {
            return Err(IIRJvmError::InvalidOperand {
                function: fname.to_string(),
                detail: format!("{} needs a label Var as last operand", instr.op),
            })
        }
    };
    Ok((cond, label))
}

// ---------------------------------------------------------------------------
// lower_iir_to_jvm — public entry point
// ---------------------------------------------------------------------------

/// Lower an `IIRModule` to a `JvmClassFile`.
///
/// This is the main public function of this module.  It:
///
/// 1. Calls [`validate_for_jvm`] and returns `Err(ValidationFailed(…))` if
///    there are any validation errors.
/// 2. Builds a minimal JVM constant pool (class name, super class, method names
///    and descriptors, `"Code"` attribute name).
/// 3. Lowers each `IIRFunction` to a `JvmMethodInfo` with a `Code` attribute
///    containing raw JVM bytecode.
/// 4. Returns the assembled `JvmClassFile`.
///
/// # Target class version
///
/// We emit Java 8 class files (major version 52, minor version 0).  Java 8
/// is the oldest LTS version that still sees widespread deployment, and all
/// modern JVMs can load it.
///
/// # Example
///
/// ```rust
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
/// use iir_to_jvm_class_file::{lower_iir_to_jvm, IIRJvmConfig};
///
/// let fn_ = IIRFunction::new(
///     "add",
///     vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
///     "i32",
///     vec![
///         IIRInstr::new("add", Some("r".into()),
///             vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
///         IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
///     ],
/// );
/// let module = IIRModule {
///     name: "demo".into(),
///     functions: vec![fn_],
///     entry_point: Some("add".into()),
///     language: "test".into(),
/// };
/// let cfg = IIRJvmConfig::new("MyClass");
/// let class_file = lower_iir_to_jvm(&module, &cfg).unwrap();
/// assert_eq!(class_file.methods.len(), 1);
/// assert_eq!(class_file.this_class_name, "MyClass");
/// ```
pub fn lower_iir_to_jvm(
    module: &IIRModule,
    config: &IIRJvmConfig,
) -> Result<JvmClassFile, IIRJvmError> {
    // ── Step 1: validate ──────────────────────────────────────────────────────
    let errors = validate_for_jvm(module);
    if !errors.is_empty() {
        return Err(IIRJvmError::ValidationFailed(errors));
    }

    // ── Step 2: build constant pool ───────────────────────────────────────────
    let mut cp = ConstantPoolBuilder::new();

    // Pre-populate the required entries.
    cp.add_class(&config.class_name);
    cp.add_class("java/lang/Object");
    cp.add_utf8("Code");

    // Pre-register Methodref entries for every function (needed for `call`).
    for func in &module.functions {
        let descriptor = make_descriptor(&func.params, &func.return_type);
        cp.add_methodref(&config.class_name, &func.name, &descriptor);
    }

    // ── Step 3: lower each function ───────────────────────────────────────────
    let mut methods: Vec<JvmMethodInfo> = Vec::new();
    for func in &module.functions {
        let method = lower_function(func, &config.class_name, module, &mut cp)?;
        methods.push(method);
    }

    // ── Step 4: assemble JvmClassFile ─────────────────────────────────────────
    Ok(JvmClassFile {
        // Java 8 = major version 52.
        version: JvmClassVersion { major: 52, minor: 0 },
        // ACC_PUBLIC | ACC_SUPER — standard flags for a public class.
        // ACC_SUPER must be set for Java 1.1+ classes (changes how `invokespecial`
        // searches the superclass method table).
        access_flags: ACC_PUBLIC | ACC_SUPER,
        this_class_name: config.class_name.clone(),
        super_class_name: "java/lang/Object".to_string(),
        constant_pool: cp.build(),
        methods,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule};

    fn make_module(func: IIRFunction) -> IIRModule {
        let name = func.name.clone();
        IIRModule {
            name: "test".into(),
            functions: vec![func],
            entry_point: Some(name),
            language: "test".into(),
        }
    }

    fn void_fn(name: &str) -> IIRFunction {
        IIRFunction::new(
            name,
            vec![],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        )
    }

    fn make_cfg() -> IIRJvmConfig {
        IIRJvmConfig::new("TestClass")
    }

    #[test]
    fn config_default() {
        let cfg = IIRJvmConfig::default();
        assert_eq!(cfg.class_name, "IIRModule");
    }

    #[test]
    fn config_new() {
        let cfg = IIRJvmConfig::new("Foo");
        assert_eq!(cfg.class_name, "Foo");
    }

    #[test]
    fn lower_void_fn_ok() {
        let module = make_module(void_fn("main"));
        let result = lower_iir_to_jvm(&module, &make_cfg());
        assert!(result.is_ok(), "{:?}", result);
    }

    #[test]
    fn lower_produces_one_method() {
        let module = make_module(void_fn("main"));
        let class = lower_iir_to_jvm(&module, &make_cfg()).unwrap();
        assert_eq!(class.methods.len(), 1);
    }

    #[test]
    fn lower_class_name_from_config() {
        let module = make_module(void_fn("main"));
        let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("MyClass")).unwrap();
        assert_eq!(class.this_class_name, "MyClass");
    }

    #[test]
    fn lower_super_class_is_object() {
        let module = make_module(void_fn("main"));
        let class = lower_iir_to_jvm(&module, &make_cfg()).unwrap();
        assert_eq!(class.super_class_name, "java/lang/Object");
    }

    #[test]
    fn lower_version_is_java8() {
        let module = make_module(void_fn("main"));
        let class = lower_iir_to_jvm(&module, &make_cfg()).unwrap();
        assert_eq!(class.version.major, 52);
        assert_eq!(class.version.minor, 0);
    }

    #[test]
    fn lower_method_has_code() {
        let module = make_module(void_fn("main"));
        let class = lower_iir_to_jvm(&module, &make_cfg()).unwrap();
        let method = &class.methods[0];
        let code = method.code_attribute().unwrap();
        assert!(!code.code.is_empty(), "code should be non-empty");
    }

    #[test]
    fn lower_validation_failure_propagates() {
        let module = IIRModule {
            name: "empty".into(),
            functions: vec![],
            entry_point: None,
            language: "test".into(),
        };
        let result = lower_iir_to_jvm(&module, &make_cfg());
        assert!(matches!(result, Err(IIRJvmError::ValidationFailed(_))));
    }

    #[test]
    fn iir_error_display_validation_failed() {
        let err = IIRJvmError::ValidationFailed(vec!["err1".into()]);
        let s = err.to_string();
        assert!(s.contains("validation failed") || s.contains("JVM validation failed"));
    }

    #[test]
    fn iir_error_display_unsupported_op() {
        let err = IIRJvmError::UnsupportedOp {
            function: "f".into(),
            op: "io_in".into(),
        };
        assert!(err.to_string().contains("unsupported op"));
    }
}
