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

use interpreter_ir::opcodes::{array_elem_type, is_array_type};
use interpreter_ir::{IIRFunction, IIRModule, Operand};
use jvm_class_file::{
    JvmClassFile, JvmClassVersion, JvmCodeAttribute, JvmConstantPoolEntry, JvmFieldInfo,
    JvmMethodAttribute, JvmMethodInfo, ACC_PUBLIC, ACC_STATIC, ACC_SUPER,
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
const LDC_W: u8 = 0x13;     // push int/float constant from CP (2-byte index)
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

// ── E8: numeric conversions (int ⇄ real) ────────────────────────────────────
//
// The JVM has dedicated single-byte opcodes for every primitive→primitive
// numeric widening/narrowing.  We use four of them:
//
//   * `i2d` / `l2d` — widen an int / long to a double (exact for all int
//     values; the IIR `int_to_real` op).
//   * `d2i` / `d2l` — narrow a double to an int / long, **truncating toward
//     zero** (drops the fraction).  This is exactly the IIR
//     `real_to_int_trunc` semantics.
//
// `real_to_int_floor` (round toward −∞, ALGOL `entier`) has no single opcode:
// we first call `java/lang/Math.floor(D)D` (which rounds toward −∞, returning
// a double) and *then* `d2l`/`d2i` to land in the integer model.
//
// ⚠️ Trap divergence (documented — diverges from
// `lang-full-e8-numeric-conversions.md` §7's uniform-trap recommendation,
// recorded in that spec's footnote ²): the VM / LLVM / WASM backends
// *trap* on NaN / ±∞ / out-of-i64-range inputs to `real_to_int_*`.  The JVM's
// `d2i`/`d2l` instead **saturate** (NaN→0, +∞→MAX, −∞→MIN) and never throw.
// For every *finite, in-range* value — which is all the `entier`/coercion
// use case ever produces — the two agree bit-for-bit, so the matrix cells
// (which exercise only such values) match.  The divergence is confined to
// pathological inputs the matrix never feeds; emitting a JVM range-check +
// `athrow` would require from-scratch exception bytecode with no reusable
// precedent in this backend, so we take the documented-divergence path.
const I2D: u8 = 0x87;  // int → double (widen, exact)
const L2D: u8 = 0x8A;  // long → double (widen)
const D2I: u8 = 0x8E;  // double → int (truncate toward zero, saturating)
const D2L: u8 = 0x8F;  // double → long (truncate toward zero, saturating)

// ── Returns ────────────────────────────────────────────────────────────────
const IRETURN: u8 = 0xAC; // return int
const LRETURN: u8 = 0xAD; // return long
const FRETURN: u8 = 0xAE; // return float
const DRETURN: u8 = 0xAF; // return double
const RETURN: u8 = 0xB1;  // return void

// ── Method invocation ──────────────────────────────────────────────────────
const INVOKESTATIC: u8 = 0xB8;   // invoke static method (2-byte CP index)
const INVOKEVIRTUAL: u8 = 0xB6;  // invoke instance method (2-byte CP index)
const CHECKCAST: u8 = 0xC0;      // checkcast (2-byte CP class index)
const INSTANCEOF: u8 = 0xC1;     // instanceof (2-byte CP class index) → push 0/1

// ── Field access ────────────────────────────────────────────────────────────
const GETSTATIC: u8 = 0xB2; // get value of static field (2-byte CP index)
const PUTSTATIC: u8 = 0xB3; // set value of static field (2-byte CP index)

// JVM byte-array access opcodes — used by Brainfuck `load_mem` / `store_mem`.
// BALOAD pops [arrayref, index] and pushes the byte at that index, sign-extended
// to an int.  BASTORE pops [arrayref, index, value] and stores `value & 0xFF`
// at `arrayref[index]`.  These match Brainfuck's u8 tape semantics exactly,
// modulo the sign-extension on load (we mask with `& 0xFF` after BALOAD).
const BALOAD: u8 = 0x33;
const BASTORE: u8 = 0x54;

/// Host class name for Brainfuck's I/O builtins and tape storage.
///
/// The host (Java runtime / launcher) must provide a class with this binary
/// name (slash-separated) containing:
///
///   * `public static byte[] __tape` — the BF tape (typically 30,000 bytes)
///   * `public static void putchar(int)` — write one byte to stdout
///   * `public static int  getchar()`    — read one byte from stdin, or `-1` / `0`
///                                          on EOF (BF's interpreter convention is `0`)
///
/// Picking a fixed host class keeps the BF-compiled class self-contained:
/// no `<clinit>` required on the BF side, and no per-program tape size baked
/// into the bytecode — the host can dial that knob without recompiling.
const BF_RUNTIME_CLASS: &str = "env/BFRuntime";

/// Host class name for BASIC's `PRINT` builtin (and future BASIC I/O).
///
/// The host (Java runtime / launcher) must provide a class with this binary
/// name (slash-separated) containing:
///
///   * `public static void println(long)` — print a 64-bit integer value
///     followed by a newline to stdout.
///
/// We pick a dedicated class instead of overloading [`BF_RUNTIME_CLASS`]
/// because BASIC's I/O model (line/value oriented, mostly numeric) differs
/// from Brainfuck's (byte-stream oriented).  Keeping them separate lets a
/// JVM launcher provide just one, or stub them independently.
///
/// This mirrors the wasm backend's `env.__print_i64` host import (see
/// `iir-to-wasm` v0.8.0, gap G2): both let BASIC's `PRINT` reach real
/// backend bytecode by deferring the actual write to the host.
const BASIC_RUNTIME_CLASS: &str = "env/BasicRuntime";

// ── Comparison and branching ───────────────────────────────────────────────
const IFEQ: u8 = 0x99;      // branch if TOS int == 0
const IFNE: u8 = 0x9A;      // branch if TOS int != 0
const IFLT: u8 = 0x9B;      // branch if TOS int < 0  (used after lcmp)
const IFGE: u8 = 0x9C;      // branch if TOS int >= 0 (used after lcmp)
const IFGT: u8 = 0x9D;      // branch if TOS int > 0  (used after lcmp)
const IFLE: u8 = 0x9E;      // branch if TOS int <= 0 (used after lcmp)
const IF_ICMPEQ: u8 = 0x9F; // branch if int1 == int2
const IF_ICMPNE: u8 = 0xA0; // branch if int1 != int2
const IF_ICMPLT: u8 = 0xA1; // branch if int1 < int2
const IF_ICMPGE: u8 = 0xA2; // branch if int1 >= int2
const IF_ICMPGT: u8 = 0xA3; // branch if int1 > int2
const IF_ICMPLE: u8 = 0xA4; // branch if int1 <= int2
const GOTO: u8 = 0xA7;      // unconditional branch (2-byte offset)

// ── Misc ───────────────────────────────────────────────────────────────────
const WIDE: u8 = 0xC4; // prefix for wide-index variants of load/store

// ── Heap / reference ops (Phase 2 — Object[] cons cells) ──────────────────
//
// These opcodes implement `cons` (alloc + field_store×2), `car`/`cdr`
// (field_load), `is_null`, and `nil` (const ref<LispyPair>) using plain
// `Object[]` arrays.  No Java class definitions are required — the JVM GC
// manages array lifetimes natively.
//
// See `jvm_class_file` for the full documentation of each opcode.

const ACONST_NULL: u8 = 0x01; // push null reference
const ALOAD: u8 = 0x19;        // aload  <index>  — load reference from local
const ASTORE: u8 = 0x3A;       // astore <index>  — store reference to local
const AALOAD: u8 = 0x32;       // aaload           — array[index] → reference
const AASTORE: u8 = 0x53;      // aastore          — array[index] ← reference
const ANEWARRAY: u8 = 0xBD;    // anewarray <cp>  — allocate reference array
const IFNULL: u8 = 0xC6;       // ifnull  <off>   — branch if top is null
const DUP: u8 = 0x59;          // dup              — duplicate TOS (any cat-1)
#[allow(dead_code)]
const SWAP: u8 = 0x5F;         // swap             — swap top two cat-1 values (future use)

// ===========================================================================
// LANG36 closure opcodes — long[] dispatch table
// ===========================================================================
//
// Closures are represented as `long[]` arrays:
//   closure[0] = function dispatch index (u32 as long)
//   closure[1..] = captured values (all cast to long)
//
// `alloc_closure` builds the array; `call_closure` passes it + an args
// `long[]` to a generated `__callClosure(long[], long[]) → long` method.

/// `newarray` (0xBC) — allocate a primitive-typed array.
///
/// Operand: one-byte type code.  For `long[]`, use T_LONG = 0x0B.
///
/// Stack: `count (int)` → `arrayref`
const NEWARRAY: u8 = 0xBC;

/// Type code for `newarray T_LONG` — produces a `long[]`.
const T_LONG: u8 = 0x0B;

// Primitive-array type codes for `newarray` (JVMS Table 6.5.newarray-A), used by
// the E5 array primitive (`alloc_array`) to pick the element width.
const T_INT: u8 = 0x0A; //  int[]
const T_FLOAT: u8 = 0x06; // float[]
const T_DOUBLE: u8 = 0x07; // double[]

/// `laload` (0x2F) — load a `long` element from a `long[]`.
///
/// Stack: `arrayref, index (int)` → `long`
const LALOAD: u8 = 0x2F;

/// `lastore` (0x50) — store a `long` element into a `long[]`.
///
/// Stack: `arrayref, index (int), value (long)` → (empty)
const LASTORE: u8 = 0x50;

// The remaining typed array load/store opcodes + `arraylength`, used by the E5
// array ops (`array_get`/`array_set`/`array_len`). Each `*aload`/`*astore`
// performs the JVM's **native bounds check** (a negative or `>= length` index
// throws `ArrayIndexOutOfBoundsException`), which is exactly E5's trap semantics.
const IALOAD: u8 = 0x2E; //  int[]   element load
const IASTORE: u8 = 0x4F; //  int[]   element store
const FALOAD: u8 = 0x30; //  float[]  element load
const FASTORE: u8 = 0x51; //  float[]  element store
const DALOAD: u8 = 0x31; //  double[] element load
const DASTORE: u8 = 0x52; //  double[] element store
const ARRAYLENGTH: u8 = 0xBE; // arrayref → length (int)

/// `lcmp` (0x94) — compare two longs.
///
/// Stack: `long1, long2` → `int` (-1, 0, or 1)
///
/// Used in `__callClosure` to dispatch on the function index:
///   `lload fn_idx_slot; ldc2_w target_idx; lcmp; ifeq case_N`
const LCMP: u8 = 0x94;
const DCMPL: u8 = 0x97; // compare two doubles → int -1/0/1 (NaN → -1)
const DCMPG: u8 = 0x98; // compare two doubles → int -1/0/1 (NaN → +1)

/// `l2i` (0x88) — narrow long to int (truncates to low 32 bits).
///
/// Stack: `long` → `int`
///
/// Used to convert a long-typed closure result back to i32 when the
/// `call_closure` destination has an i32/bool type hint.
const L2I: u8 = 0x88;

/// `i2l` (0x85) — widen int to long.
///
/// Stack: `int` → `long`
///
/// Used to box i32/bool captured values into the `long[]` closure array.
const I2L: u8 = 0x85;

fn checked_jvm_string_literal(ctx: &str, s: &str) -> Result<(), IIRJvmError> {
    for ch in s.chars() {
        match ch {
            '\n' | '\r' | '\t' => {}
            c if c.is_ascii_graphic() || c == ' ' => {}
            c => {
                return Err(IIRJvmError::InvalidOperand {
                    function: ctx.to_string(),
                    detail: format!(
                        "str_const literal contains unsupported non-printable/non-ASCII character U+{:04X}",
                        c as u32
                    ),
                })
            }
        }
    }
    Ok(())
}

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
/// - `Ref`    — Java object reference (`Object[]` for LispyPair cons cells)
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
    /// Maps to a JVM reference type (descriptor `"Ljava/lang/Object;"`).
    ///
    /// Phase 2 uses this for `ref<LispyPair>` — cons cells stored as
    /// `Object[]` arrays.  Reference locals use `aload`/`astore` instead of
    /// `iload`/`istore`, and they occupy exactly one local slot (unlike `long`
    /// and `double` which take two).
    Ref,
}

impl JvmType {
    /// Number of local-variable slots this type occupies.
    ///
    /// The JVM allocates one slot per 32-bit value and two slots per 64-bit
    /// value (`long` and `double`).  All other JVM primitive types fit in one
    /// slot, including object references.
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
            JvmType::Ref => "Ljava/lang/Object;",
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
        // Narrow integer widths and `bool` use the JVM `int` model (LANG-FULL
        // E2). A scalar program reaches this backend through
        // `lang_aot::concretize_scalar_any_for_jvm`, which narrows `i64`→`i32`
        // (the in-repo jvm-simulator is 32-bit and the entry must `ireturn`), so
        // a narrow-unsigned op already meets `i32` operands — no `long` register
        // model is needed. (The long model was tried in v0.13.0 and reverted: it
        // left narrow ops `long` while concretize made the consts/return `int`,
        // producing unverifiable bytecode — `istore` consts feeding an `lmul`,
        // `lreturn` from an `int`-returning method.) The narrow-width WRAP is
        // restored by masking the int result (see `emit_jvm_width_mask`). `u4`
        // (Nib's nibble) is recognised so it gets an int slot.
        "u4" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "bool" => Some(JvmType::Int),
        "i64" | "u64" => Some(JvmType::Long),
        "f32" => Some(JvmType::Float),
        "f64" => Some(JvmType::Double),
        "void" | "" => Some(JvmType::Void),
        // Phase 2: LispyPair cons cells are represented as Object[] references.
        // Any variable holding a pair (or nil) gets a Ref slot, which uses
        // aload/astore rather than iload/istore.
        "ref<LispyPair>" => Some(JvmType::Ref),
        // McCarthy W3b: a boxed lisp value (`ref<any>`) is a `java.lang.Object`
        // — an `Integer` for an atom, an `Object[]` for a cons cell. Uses
        // aload/astore like any other reference.
        "ref<any>" => Some(JvmType::Ref),
        // LANG36: A closure is a `long[]` array reference.
        // Variables holding closures use aload/astore (Ref = reference type).
        "closure" => Some(JvmType::Ref),
        // LANG-FULL E4: first string foothold. A `str` value is a JVM reference
        // local so `str_const` can materialise a `java.lang.String` and
        // `print_str` can pass it to `PrintStream.print(String)`. Richer byte
        // string ops remain unsupported by the validator.
        "str" => Some(JvmType::Ref),
        // LANG-FULL E5: an `array<T>` handle is a JVM primitive-array reference
        // (`int[]`/`long[]`/`double[]`/…). Like every reference it occupies one
        // local slot and uses aload/astore; the *element* opcode (iaload/laload/
        // daload/…) is chosen per access from `T`. The element type must itself
        // map to a JVM type (no nested or reference-element arrays yet).
        h if is_array_type(h) => {
            let elem = array_elem_type(h)?;
            match iir_type_to_jvm(&elem)? {
                JvmType::Int | JvmType::Long | JvmType::Float | JvmType::Double => Some(JvmType::Ref),
                // E4d-BA-arr: a supported *reference* element (`array<str>` →
                // `String[]`) is also a Ref-slot handle; its elements load/store
                // with `aaload`/`aastore` and it allocates with `anewarray`.
                JvmType::Ref if jvm_ref_array_element_class(&elem).is_some() => Some(JvmType::Ref),
                JvmType::Void | JvmType::Ref => None,
            }
        }
        // Catch-all: return None, let caller decide
        _ => None,
    }
}

/// For a **reference**-element array (E4d-BA-arr), the JVM class used by
/// `anewarray` and loaded/stored with `aaload`/`aastore`.  `array<str>` →
/// `String[]`; every other element is a primitive handled by
/// [`array_element_opcodes`] instead, so this returns `None` for them.
fn jvm_ref_array_element_class(elem_hint: &str) -> Option<&'static str> {
    match elem_hint {
        "str" => Some("java/lang/String"),
        _ => None,
    }
}

/// The `newarray` type code + typed load/store opcodes for an array whose
/// **element** maps to `elem` ([`JvmType`]). Returns `None` for element types
/// that can't be a primitive-array element here (`Void`, `Ref` — nested arrays
/// are a future phase). The three opcodes are `(newarray atype, *aload, *astore)`.
fn array_element_opcodes(elem: JvmType) -> Option<(u8, u8, u8)> {
    match elem {
        JvmType::Int => Some((T_INT, IALOAD, IASTORE)),
        JvmType::Long => Some((T_LONG, LALOAD, LASTORE)),
        JvmType::Float => Some((T_FLOAT, FALOAD, FASTORE)),
        JvmType::Double => Some((T_DOUBLE, DALOAD, DASTORE)),
        JvmType::Void | JvmType::Ref => None,
    }
}

/// Extract `srcs[i]` of an array op as a variable name, with a clear error
/// naming the op and the operand's role (`handle`/`idx`/`val`).
fn array_var_operand(
    instr: &interpreter_ir::IIRInstr,
    i: usize,
    op: &str,
    role: &str,
    fname: &str,
) -> Result<String, IIRJvmError> {
    match instr.srcs.get(i) {
        Some(Operand::Var(s)) => Ok(s.clone()),
        _ => Err(IIRJvmError::InvalidOperand {
            function: fname.to_string(),
            detail: format!("{op} requires Operand::Var({role}) as src[{i}]"),
        }),
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
        // Narrow integer widths and `bool` use the JVM `int` model (E2) — see
        // `iir_type_to_jvm` — so their descriptor is `I`.
        "u4" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "bool" => "I",
        "i64" | "u64" => "J",
        "f32" => "F",
        "f64" => "D",
        "void" | "" => "V",
        // Phase 2: LispyPair cons cells are Object[] references.
        // The JVM method descriptor for a reference parameter/return is
        // "Ljava/lang/Object;" (the erasure of the actual Object[] type).
        "ref<LispyPair>" => "Ljava/lang/Object;",
        // McCarthy W3b: a boxed lisp value (`ref<any>`) erases to Object.
        "ref<any>" => "Ljava/lang/Object;",
        // LANG-FULL E4: string literals flow as java.lang.String.
        "str" => "Ljava/lang/String;",
        // LANG36: A closure is a `long[]` — descriptor is "[J".
        "closure" => "[J",
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

/// Emit an `aload` instruction (load object reference from local slot).
///
/// Used for `ref<LispyPair>` variables (cons cells represented as `Object[]`).
/// Like `iload`, the JVM has short-form `aload_0`..`aload_3` opcodes (0x2A–0x2D),
/// a 1-byte-index form `aload N` for slots ≤255, and a `wide aload` form for
/// larger indices.  We emit the 1-byte form for all indices ≤255 (short forms
/// for 0–3 are a minor optimisation we skip for code simplicity).
fn emit_aload(code: &mut Vec<u8>, idx: u16) {
    if idx <= 255 {
        code.push(ALOAD);
        code.push(idx as u8);
    } else {
        // wide aload for slot indices 256–65535.
        code.push(WIDE);
        code.push(ALOAD);
        code.extend_from_slice(&idx.to_be_bytes());
    }
}

/// Emit the appropriate typed load instruction for a slot and JVM type.
fn emit_typed_load(code: &mut Vec<u8>, idx: u16, jvm_type: JvmType) {
    match jvm_type {
        JvmType::Int => emit_iload(code, idx),
        JvmType::Long => emit_lload(code, idx),
        JvmType::Float => emit_fload(code, idx),
        JvmType::Double => emit_dload(code, idx),
        JvmType::Ref => emit_aload(code, idx),
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

/// Emit an `astore` instruction (store object reference to local slot).
///
/// Counterpart to [`emit_aload`] — stores an `Object[]` cons-cell reference
/// from the operand stack into the named local variable slot.
fn emit_astore(code: &mut Vec<u8>, idx: u16) {
    if idx <= 255 {
        code.push(ASTORE);
        code.push(idx as u8);
    } else {
        code.push(WIDE);
        code.push(ASTORE);
        code.extend_from_slice(&idx.to_be_bytes());
    }
}

/// Emit the appropriate typed store instruction for a slot and JVM type.
fn emit_typed_store(code: &mut Vec<u8>, idx: u16, jvm_type: JvmType) {
    match jvm_type {
        JvmType::Int => emit_istore(code, idx),
        JvmType::Long => emit_lstore(code, idx),
        JvmType::Float => emit_fstore(code, idx),
        JvmType::Double => emit_dstore(code, idx),
        JvmType::Ref => emit_astore(code, idx),
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
            // Out of sipush range: this path requires a constant-pool entry, so
            // callers with an `int` literal beyond ±32767 MUST use
            // `emit_iconst_cp` instead. Reaching here would emit an invalid `ldc`
            // (placeholder index 0 → a JVM `constantTag` crash), so we refuse:
            // a 0-byte no-op leaves the stack short, which the verifier/test
            // catches loudly rather than corrupting a class. (McCarthy W5a fixed
            // every user-constant call site to route large values through the CP.)
            debug_assert!(
                false,
                "emit_iconst called with out-of-sipush-range value {value}; \
                 use emit_iconst_cp (constant-pool ldc) instead"
            );
        }
    }
}

/// Push an `int` constant, using the **constant pool** (`ldc`/`ldc_w`) for values
/// outside the `bipush`/`sipush` range. The constant-pool-aware companion of
/// [`emit_iconst`]: every call site that emits a *user-controlled* integer
/// literal (a `const`, a `mov`/`ret` immediate, a `call` argument) must use this,
/// since an interned symbol id or any literal ≥ 2¹⁵ would otherwise hit the
/// (now-`debug_assert`-guarded) invalid-`ldc` path. Structural indices (field
/// numbers, slot counts, arg counts) stay on [`emit_iconst`] — they are always
/// small (McCarthy W5a).
fn emit_iconst_cp(code: &mut Vec<u8>, cp: &mut ConstantPoolBuilder, value: i32) {
    if (-32768..=32767).contains(&value) {
        emit_iconst(code, value);
        return;
    }
    let idx = cp.add_integer(value);
    if idx <= 0xFF {
        code.push(LDC);
        code.push(idx as u8);
    } else {
        code.push(LDC_W);
        code.extend_from_slice(&idx.to_be_bytes());
    }
}

fn emit_ldc_index(code: &mut Vec<u8>, idx: u16) {
    if idx <= 0xFF {
        code.push(LDC);
        code.push(idx as u8);
    } else {
        code.push(LDC_W);
        code.extend_from_slice(&idx.to_be_bytes());
    }
}

/// Mask the narrow-width `int` on top of the operand stack to its bit width
/// (LANG-FULL E2).
///
/// JVM `int` arithmetic (`iadd`/`imul`/…) wraps mod-2³², so `u32`/`i32` are
/// already correct.  The smaller widths (`u4`/`u8`/`u16`) need an explicit
/// `iconst/sipush/ldc <mask>; iand` after the op so `200u8 + 100u8` becomes
/// `44` and `~0u8` is `255` — mirroring vm-core's `mask_result`, jit-core's
/// `MASK_WIDTH`, the wasm `i64.and`, and the byte-tape `baload`+mask precedent.
/// We deliberately use a positive mask + `iand` rather than `i2b`/`i2s` (which
/// sign-extend, giving a *signed* byte — wrong for the unsigned narrow types the
/// LANG-FULL frontends use).  `i64`/`u64`/floats emit nothing.
///
/// The mask must match the **value model** of the op it follows: a scalar
/// (exit-code) program is concretized to `i32`, so its narrow op runs on the
/// `int` model and the mask is an `int` `iand`. But a **printing** program (Oct's
/// `out`, Dartmouth BASIC's `PRINT`) keeps the `i64`/`long` model — there the op
/// is e.g. `ladd`/`lxor` and the result on the stack is a `long`, so an `int`
/// `iand` over it is unverifiable (operand-type mismatch). For the long model we
/// therefore push the mask as a `long` (int mask + `i2l`; the masks are positive,
/// so the widening zero-extends) and use `land`. `jtype` is the op's `instr_jtype`,
/// so the mask is always operand-consistent with the value it narrows. (This is the
/// principled version of the int-only mask reverted in v0.13.0 — keyed on the op's
/// actual model, not assumed.) `i64`/`u64`/floats emit nothing.
fn emit_jvm_width_mask(
    code: &mut Vec<u8>,
    cp: &mut ConstantPoolBuilder,
    type_hint: &str,
    jtype: JvmType,
) {
    let mask: i32 = match type_hint {
        "u4" => 0xF,
        "u8" => 0xFF,
        "u16" => 0xFFFF,
        _ => return,
    };
    emit_iconst_cp(code, cp, mask);
    match jtype {
        // long model (printing programs): widen the mask to a long, then `land`.
        JvmType::Long => {
            code.push(I2L);
            code.push(LAND);
        }
        // int model (concretized scalar programs): plain `iand`.
        _ => code.push(IAND),
    }
}

/// Emit a long constant push.
///
/// JVM only has short forms for `0L` and `1L`.  Anything else needs an
/// Emit a long (i64) constant push onto the JVM operand stack.
///
/// JVM has dedicated opcodes for `0L` (`lconst_0`) and `1L` (`lconst_1`).
/// For other values that fit in a short int we synthesise the long via an int
/// push followed by `i2l` (int-to-long widening):
///
/// ```text
/// iconst_N    — push int N          (2 ≤ N ≤ 5: 1 byte)
/// bipush  N   — push byte  N        (-128 ≤ N ≤ 127: 2 bytes)
/// sipush  N   — push short N        (-32768 ≤ N ≤ 32767: 3 bytes)
/// i2l         — widen int → long    (1 byte)
/// ```
///
/// Full long constants (larger than i16 range) would require a `ldc2_w` with
/// a proper `Long` constant-pool entry; the constant pool builder (`ConstantPool`)
/// does support `add_long`, so callers needing that path should use it directly.
/// For the arithmetic programs in this VM (Twig fib, etc.) values never exceed
/// i16 range, so this covers all practical cases.
fn emit_lconst(code: &mut Vec<u8>, value: i64) {
    match value {
        // JVM has dedicated 1-byte long constants for 0 and 1.
        0 => code.push(LCONST_0),
        1 => code.push(LCONST_1),
        // iconst_2 … iconst_5 + i2l (2 bytes total)
        2 => { code.push(ICONST_2); code.push(I2L); }
        3 => { code.push(ICONST_3); code.push(I2L); }
        4 => { code.push(ICONST_4); code.push(I2L); }
        5 => { code.push(ICONST_5); code.push(I2L); }
        // iconst_m1 + i2l for -1
        -1 => { code.push(ICONST_M1); code.push(I2L); }
        // bipush (byte-range) + i2l
        v if v >= -128 && v <= 127 => {
            code.push(BIPUSH);
            code.push(v as i8 as u8);
            code.push(I2L);
        }
        // sipush (short-range) + i2l
        v if v >= i16::MIN as i64 && v <= i16::MAX as i64 => {
            code.push(SIPUSH);
            code.extend_from_slice(&(v as i16).to_be_bytes());
            code.push(I2L);
        }
        _ => {
            // Values outside i16 range but within i32 range: push as int + i2l.
            // This avoids a CP Long entry for the i32-range values.
            if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
                code.push(LDC);
                // Placeholder — callers that need CP must use emit_lconst_cp instead.
                // This arm should not be reached for JvmType::Long const lowering.
                code.extend_from_slice(&0xFFFFu16.to_be_bytes());
            } else {
                code.push(LDC2_W);
                code.extend_from_slice(&0xFFFFu16.to_be_bytes());
            }
        }
    }
}

/// Emit a long constant push, adding a `CONSTANT_Long` pool entry for values
/// outside the i16 range that `emit_lconst` cannot handle inline.
///
/// JVM has 1-byte short forms for `0L` (`lconst_0`) and `1L` (`lconst_1`).
/// For small values (−128 to 32767) we reuse the existing `bipush`/`sipush`/
/// `iconst_*` + `i2l` tricks from `emit_lconst`. Larger values require an
/// `ldc2_w <cp_index>` pointing at a `CONSTANT_Long` pool entry — the Long form
/// of what `emit_iconst_cp` does for `int`.
///
/// This is the correct counterpart to `emit_dconst_cp` / `emit_iconst_cp`.
/// Call this instead of `emit_lconst` whenever the constant can be an arbitrary
/// i64 (e.g. the `"const"` IIR lowering path for `JvmType::Long` destinations).
fn emit_lconst_cp(code: &mut Vec<u8>, cp: &mut ConstantPoolBuilder, value: i64) {
    match value {
        0 => code.push(LCONST_0),
        1 => code.push(LCONST_1),
        2 => { code.push(ICONST_2); code.push(I2L); }
        3 => { code.push(ICONST_3); code.push(I2L); }
        4 => { code.push(ICONST_4); code.push(I2L); }
        5 => { code.push(ICONST_5); code.push(I2L); }
        -1 => { code.push(ICONST_M1); code.push(I2L); }
        v if v >= -128 && v <= 127 => {
            code.push(BIPUSH);
            code.push(v as i8 as u8);
            code.push(I2L);
        }
        v if v >= i16::MIN as i64 && v <= i16::MAX as i64 => {
            code.push(SIPUSH);
            code.extend_from_slice(&(v as i16).to_be_bytes());
            code.push(I2L);
        }
        _ => {
            let idx = cp.add_long(value);
            code.push(LDC2_W);
            code.extend_from_slice(&idx.to_be_bytes());
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

/// Emit a double constant push, adding a `CONSTANT_Double` pool entry when one
/// is needed (LANG-FULL E3).
///
/// JVM has 1-byte short forms for `0.0d` (`dconst_0`) and `1.0d` (`dconst_1`).
/// Any *other* double must be loaded with `ldc2_w <cp_index>`, where the index
/// points at a `CONSTANT_Double` pool entry. The old `emit_dconst` left a
/// placeholder index `#0` (the unused phantom slot) — valid-looking bytecode
/// that the verifier rejects, so an ALGOL `real` literal like `2.5` produced an
/// unloadable class. `cp.add_double` interns the value (reserving the two pool
/// slots a `Double` occupies) and we emit the real index.
fn emit_dconst_cp(code: &mut Vec<u8>, cp: &mut ConstantPoolBuilder, value: f64) {
    if value == 0.0f64 && value.is_sign_positive() {
        code.push(DCONST_0);
    } else if value == 1.0f64 {
        code.push(DCONST_1);
    } else {
        let idx = cp.add_double(value);
        code.push(LDC2_W);
        code.extend_from_slice(&idx.to_be_bytes());
    }
}

/// Emit a "compare two doubles on the stack, push 1 if the condition holds,
/// else 0" sequence — the `double` counterpart of [`emit_long_compare`]
/// (LANG-FULL E3). The JVM has no `if_dcmpXX`; the idiom is a `dcmpl`/`dcmpg`
/// (which leaves an `int` -1/0/1 on the stack) followed by a unary `ifXX`
/// branch over that result, with the same negated-opcode table the long path
/// uses.
///
/// `dcmp_opcode` is `DCMPL` (NaN → -1) or `DCMPG` (NaN → +1); `branch_opcode`
/// is the negated unary branch (e.g. `IFNE` for `cmp_eq`). Stack on entry:
/// `[…, double1, double2]`; on exit: `[…, 0_or_1]`.
fn emit_double_compare(code: &mut Vec<u8>, dcmp_opcode: u8, branch_opcode: u8) {
    code.push(dcmp_opcode); // 1 byte: compare doubles, push -1/0/1
    // ifXX at current PC, offset to iconst_0 (7 bytes forward)
    code.push(branch_opcode);
    code.extend_from_slice(&7i16.to_be_bytes());
    // True arm — condition held
    code.push(ICONST_1);
    // Jump past the false arm
    code.push(GOTO);
    code.extend_from_slice(&4i16.to_be_bytes());
    // False arm
    code.push(ICONST_0);
    // 9 bytes total (1 for dcmp + 8 for the branch pattern)
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
/// Extract a `call_builtin`'s dest register name, or a descriptive error
/// (McCarthy W4 predicate lowering).
fn builtin_dest<'a>(
    instr: &'a interpreter_ir::IIRInstr,
    fname: &str,
    name: &str,
) -> Result<&'a str, IIRJvmError> {
    instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
        function: fname.to_string(),
        detail: format!("call_builtin {name:?} requires a dest register"),
    })
}

/// Extract a `call_builtin`'s `srcs[idx]` as a variable name (the builtin name
/// is `srcs[0]`, so arguments start at `idx == 1`).
fn builtin_arg(
    instr: &interpreter_ir::IIRInstr,
    fname: &str,
    name: &str,
    idx: usize,
) -> Result<String, IIRJvmError> {
    match instr.srcs.get(idx) {
        Some(Operand::Var(s)) => Ok(s.clone()),
        _ => Err(IIRJvmError::InvalidOperand {
            function: fname.to_string(),
            detail: format!("call_builtin {name:?} requires srcs[{idx}] = Operand::Var"),
        }),
    }
}

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

/// Emit a "compare two longs on the stack (already loaded), push 1 if
/// condition holds, else 0" sequence.
///
/// This is the long counterpart of `emit_int_compare`.  JVM does not have
/// `if_lcmpXX` instructions; instead the idiom is:
///
/// ```text
/// lcmp          ; compare top two longs → int (-1, 0, +1)
/// <ifXX> +7     ; negated condition: skip iconst_1 when condition is FALSE
/// iconst_1      ; condition was true
/// goto   +4
/// iconst_0      ; condition was false
/// ```
///
/// `cmp_opcode` is the **negated** unary branch instruction (operates on the
/// `lcmp` int result):
///
/// | IIR op   | negated opcode (skip true when false) |
/// |----------|---------------------------------------|
/// | `cmp_eq` | `IFNE` (0x9A)  — skip when result ≠ 0 |
/// | `cmp_ne` | `IFEQ` (0x99)  — skip when result = 0 |
/// | `cmp_lt` | `IFGE` (0x9C)  — skip when result ≥ 0 |
/// | `cmp_le` | `IFGT` (0x9D)  — skip when result > 0 |
/// | `cmp_gt` | `IFLE` (0x9E)  — skip when result ≤ 0 |
/// | `cmp_ge` | `IFLT` (0x9B)  — skip when result < 0 |
///
/// Stack state on entry: `[…, long1, long2]`
/// Stack state on exit:  `[…, 0_or_1]`
fn emit_long_compare(code: &mut Vec<u8>, cmp_opcode: u8) {
    code.push(LCMP); // 1 byte: compare longs, push -1/0/1
    // ifXX at current PC, offset to iconst_0 (7 bytes forward)
    code.push(cmp_opcode);
    code.extend_from_slice(&7i16.to_be_bytes());
    // True arm — condition held
    code.push(ICONST_1);
    // Jump past the false arm
    code.push(GOTO);
    code.extend_from_slice(&4i16.to_be_bytes());
    // False arm
    code.push(ICONST_0);
    // 9 bytes total (1 for lcmp + 8 for the branch pattern)
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
///
/// # LANG36 special cases
///
/// `alloc_closure` destinations are always `JvmType::Ref` (they hold a
/// `long[]` reference, regardless of the `"closure"` type_hint string).
///
/// `call_closure` destinations are always `JvmType::Long` — the generated
/// `__callClosure(long[], long[])` dispatch method always returns `long`,
/// so the receiving slot must be two-slot-wide even when the type_hint is
/// the generic `"any"` string.
/// A comparison op produces a 0/1 boolean `int`, whatever its operand width.
/// Its dest slot must therefore be `int` on the JVM (see [`build_type_map`]).
fn is_comparison_op(op: &str) -> bool {
    matches!(
        op,
        "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge"
    )
}

/// A narrow unsigned width (`u4`/`u8`/`u16`) — one that rides the JVM `int`
/// model by default and is brought back into range by [`emit_jvm_width_mask`].
fn is_narrow_width(hint: &str) -> bool {
    matches!(hint, "u4" | "u8" | "u16")
}

/// True when `instr` is a narrow-width arithmetic / bitwise / unary op whose
/// operands ride the `long` value model.
///
/// Two value models reach this backend (see [`iir_type_to_jvm`]): an exit-code
/// scalar program is concretized to `i32` (operands are `int`), but a **printing**
/// program (Oct's `out`, Dartmouth BASIC's `PRINT`) keeps the `i64`/`long` model so
/// its value can be passed to `print_i64`. Oct's only integer type is `u8`, so a
/// printing Oct program emits a narrow-hinted `add`/`~`/… over `long` operands. By
/// default the narrow hint maps the op to the `int` model (`iadd`), but the operands
/// are loaded as `long` — an unverifiable mix. Such an op must therefore stay on the
/// `long` model (`ladd`/`lxor`/…), with the narrow hint driving only the post-op width
/// mask (`emit_jvm_width_mask` then emits `i2l; land`). Operand types come from
/// `type_map`; a const/def always precedes its use, so they are already recorded.
fn narrow_op_over_long(
    instr: &interpreter_ir::IIRInstr,
    type_map: &HashMap<String, JvmType>,
) -> bool {
    if !is_narrow_width(&instr.type_hint) {
        return false;
    }
    if !matches!(
        instr.op.as_str(),
        "add" | "sub" | "mul" | "div" | "mod" | "and" | "or" | "xor" | "not" | "neg"
    ) {
        return false;
    }
    instr.srcs.iter().any(|s| {
        matches!(s, Operand::Var(v) if type_map.get(v) == Some(&JvmType::Long))
    })
}

fn build_type_map(func: &IIRFunction) -> HashMap<String, JvmType> {
    let mut map: HashMap<String, JvmType> = HashMap::new();

    for (pname, ptype) in &func.params {
        if let Some(t) = iir_type_to_jvm(ptype) {
            map.insert(pname.clone(), t);
        }
    }
    for instr in &func.instructions {
        if let Some(dest) = &instr.dest {
            // LANG36: Override type for closure opcodes so slot allocation is
            // always correct, regardless of the type_hint string.
            let t = if instr.op == "alloc_closure" {
                // Closure handle = long[] reference
                JvmType::Ref
            } else if instr.op == "call_closure" {
                // __callClosure always returns long
                JvmType::Long
            } else if is_comparison_op(&instr.op) {
                // A comparison ALWAYS produces a 0/1 `int` result (it is stored
                // with a bare `istore`), regardless of its `type_hint` — which
                // carries the *operand* width, not the result width. Typing the
                // dest by the hint (e.g. `i64` → `Long`) for a comparison over
                // `long` operands gives the slot a `Long` type, so a later
                // `jmp_if_false` reads it with `lload` while the comparison wrote
                // it with `istore` → the verifier rejects "uninitialized register
                // pair" (BA-JVM-1: BASIC's `IF`/`FOR` over its i64 value model,
                // which — unlike the concretized-to-i32 scalar path — keeps the
                // operands `long`). Force the bool result to `Int`.
                JvmType::Int
            } else if narrow_op_over_long(instr, &map) {
                // A narrow op over `long` operands (a printing program) keeps its
                // result on the `long` model; the narrow hint only drives the mask.
                JvmType::Long
            } else {
                iir_type_to_jvm(&instr.type_hint).unwrap_or(JvmType::Int)
            };
            map.entry(dest.clone()).or_insert(t);
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

    /// Add a `CONSTANT_String` entry for an `ldc` string literal.
    fn add_string(&mut self, s: &str) -> u16 {
        let string_idx = self.add_utf8(s);
        let key = format!("String:{}", s);
        self.add_entry(key, JvmConstantPoolEntry::String { string_index: string_idx })
    }

    /// Add a `CONSTANT_Integer` entry (deduplicated) and return its 1-based
    /// index, for an `int` literal too large for `bipush`/`sipush` and so loaded
    /// with `ldc`/`ldc_w` (McCarthy W5a — e.g. an interned symbol id ≥ 2²⁹).
    fn add_integer(&mut self, value: i32) -> u16 {
        let key = format!("Integer:{}", value);
        self.add_entry(key, JvmConstantPoolEntry::Integer(value))
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

    /// Add a Fieldref entry (used for `getstatic`/`putstatic` field access).
    ///
    /// This is the constant pool counterpart of `add_methodref`, but for field
    /// references.  The JVM encodes field access via `JvmConstantPoolEntry::Fieldref`
    /// which points to a Class entry and a NameAndType entry just like Methodref.
    ///
    /// # Example (io_out uses this for `java/lang/System.out`)
    ///
    /// ```text
    /// Fieldref { class = "java/lang/System", name = "out", desc = "Ljava/io/PrintStream;" }
    /// ```
    fn add_fieldref(&mut self, class_name: &str, field_name: &str, descriptor: &str) -> u16 {
        let class_idx = self.add_class(class_name);
        let nat_idx = self.add_name_and_type(field_name, descriptor);
        let key = format!("Fieldref:{}.{}:{}", class_name, field_name, descriptor);
        self.add_entry(
            key,
            JvmConstantPoolEntry::Fieldref {
                class_index: class_idx,
                name_and_type_index: nat_idx,
            },
        )
    }

    /// Add a Long constant entry.
    ///
    /// Per the JVM spec §4.4.5, Long constants occupy **two** consecutive constant
    /// pool slots: the Long entry at index N, and an unusable "phantom" `None` at
    /// index N+1.  The phantom is never referenced; it is only here to keep the
    /// indices consistent with what the serialiser writes into the file.
    ///
    /// Returns the 1-based index of the Long entry (not the phantom).
    fn add_long(&mut self, value: i64) -> u16 {
        let key = format!("Long:{}", value);
        if let Some(&idx) = self.index_map.get(&key) {
            return idx;
        }
        // We need two free slots.
        assert!(
            self.entries.len() + 1 < u16::MAX as usize,
            "JVM constant pool overflow: too many entries (limit 65535)"
        );
        self.entries.push(Some(JvmConstantPoolEntry::Long(value)));
        let idx = (self.entries.len() - 1) as u16;
        self.index_map.insert(key, idx);
        // Push the phantom slot (index N+1 is unusable per JVM spec §4.4.5).
        self.entries.push(None);
        idx
    }

    /// Add a `CONSTANT_Double` entry (deduplicated by exact bit pattern) and
    /// return its 1-based index, for an `f64` literal loaded with `ldc2_w`
    /// (LANG-FULL E3 — ALGOL `real` constants). Like `Long`, a `Double`
    /// occupies **two** pool slots, so a phantom `None` follows it.
    fn add_double(&mut self, value: f64) -> u16 {
        // Key on the raw bits so `-0.0`/`0.0` and any NaN payloads dedup
        // exactly and we never rely on `f64: Eq` (which doesn't hold).
        let key = format!("Double:{:016X}", value.to_bits());
        if let Some(&idx) = self.index_map.get(&key) {
            return idx;
        }
        // We need two free slots.
        assert!(
            self.entries.len() + 1 < u16::MAX as usize,
            "JVM constant pool overflow: too many entries (limit 65535)"
        );
        self.entries.push(Some(JvmConstantPoolEntry::Double(value)));
        let idx = (self.entries.len() - 1) as u16;
        self.index_map.insert(key, idx);
        // Push the phantom slot (index N+1 is unusable per JVM spec §4.4.5).
        self.entries.push(None);
        idx
    }

    /// Finalise the pool and return it as a `Vec<Option<JvmConstantPoolEntry>>`.
    fn build(self) -> Vec<Option<JvmConstantPoolEntry>> {
        self.entries
    }
}

// ---------------------------------------------------------------------------
// LANG36 — Closure dispatch table
// ---------------------------------------------------------------------------
//
// A JVM closure is a `long[]` array where:
//   [0] = dispatch index (which function to call)
//   [1..n] = captured values (as longs)
//
// A generated `__callClosure(long[] closure, long[] args) -> long` method
// acts as the dynamic dispatch point.  It reads closure[0] and branches
// using lcmp + ifne chains to the appropriate static function call.

/// One entry in the closure dispatch table.
///
/// Each entry corresponds to one function that can be allocated as a closure
/// (i.e. appears as `srcs[0]` in any `alloc_closure` instruction).
#[derive(Debug, Clone)]
struct ClosureDispatchEntry {
    /// The IIR function name (e.g. `"__lambda_0"`).
    fn_name: String,
    /// Stable integer index assigned to this function (0-based, alphabetical).
    dispatch_idx: usize,
    /// Number of captured values for this closure (= `srcs[1..]` length in
    /// the `alloc_closure` instruction).
    n_captures: usize,
    /// Full parameter list of the target function (name + type_hint pairs).
    ///
    /// Used to reconstruct the call signature inside `__callClosure`.
    fn_params: Vec<(String, String)>,
    /// Return type of the target function.
    fn_return_type: String,
}

/// Pre-pass: collect all closure-eligible functions from the module.
///
/// Scans every function's instructions for `alloc_closure` opcodes and
/// collects the target function names (`srcs[0]` = `Operand::Str(fn_name)`).
///
/// Returns a `HashMap<fn_name → ClosureDispatchEntry>` sorted by index.
/// Indices are assigned alphabetically (deterministic ordering → byte-identical
/// class files from identical modules).
fn collect_closure_dispatch(module: &IIRModule) -> HashMap<String, ClosureDispatchEntry> {
    // Step 1: gather (fn_name, n_captures) from every alloc_closure instruction.
    //
    // If the same function is allocated with different capture counts in different
    // places, we record the FIRST occurrence.  A later lowering-time check
    // (captures + args ≠ func.params.len()) will catch inconsistencies.
    let mut name_to_captures: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();

    for func in &module.functions {
        for instr in &func.instructions {
            if instr.op == "alloc_closure" {
                if let Some(Operand::Str(fn_name)) = instr.srcs.first() {
                    let n_caps = instr.srcs.len().saturating_sub(1); // skip Str(fn_name)
                    name_to_captures
                        .entry(fn_name.clone())
                        .or_insert(n_caps);
                }
            }
        }
    }

    // Step 2: assign indices (BTreeMap guarantees alphabetical order).
    let mut dispatch: HashMap<String, ClosureDispatchEntry> = HashMap::new();
    for (idx, (fn_name, n_captures)) in name_to_captures.into_iter().enumerate() {
        // Look up the function's parameter list and return type.
        let (fn_params, fn_return_type) = module
            .get_function(&fn_name)
            .map(|f| (f.params.clone(), f.return_type.clone()))
            .unwrap_or_else(|| (vec![], "i64".to_string()));

        dispatch.insert(
            fn_name.clone(),
            ClosureDispatchEntry {
                fn_name,
                dispatch_idx: idx,
                n_captures,
                fn_params,
                fn_return_type,
            },
        );
    }
    dispatch
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
    closure_dispatch: &HashMap<String, ClosureDispatchEntry>,
    globals: &HashMap<String, String>,
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

    // Pre-register java/lang/Object class in the constant pool.
    // Required for `anewarray` (cons cell allocation).
    let object_class_idx = cp.add_class("java/lang/Object");

    // Index-based loop so we can look ahead for the `alloc + field_store ×2`
    // cons-cell pattern (see the `alloc` arm below).
    let instrs = &func.instructions;
    let mut i = 0usize;
    while i < instrs.len() {
        let instr = &instrs[i];
        // Resolve the instruction's own JVM type (best effort; void is OK).
        // A narrow op over `long` operands stays on the `long` model (matching the
        // `Long` slot `build_type_map` gave its dest), so the opcode (`ladd`/`lxor`/…)
        // and the post-op mask (`i2l; land`) are operand-consistent.
        let instr_jtype = if narrow_op_over_long(instr, &type_map) {
            JvmType::Long
        } else {
            iir_type_to_jvm(&instr.type_hint).unwrap_or(JvmType::Int)
        };

        match instr.op.as_str() {
            // ── LANG-FULL E4: string literal output foothold ────────────────
            //
            // Dartmouth BASIC `PRINT "..."` lowers to `str_const` +
            // `print_str`; literal Twig `string-length`, `string=?`, `string<?` /
            // `string>?`, and `string-append` lower to `str_len`, `str_eq`,
            // `str_cmp`, and `str_concat`.
            // Use Java's native `String` only for this literal foothold; richer
            // byte-oriented string algebra remains rejected by the validator
            // until the JVM representation owns those semantics explicitly.
            "str_const" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "str_const instruction has no dest".to_string(),
                })?;
                let literal = match instr.srcs.first() {
                    Some(Operand::Str(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_const expects a string literal, got {other:?}"),
                        })
                    }
                };
                checked_jvm_string_literal(fname, literal)?;
                let (dest_slot, dest_type) = lookup_var(dest_name)?;
                if dest_type != JvmType::Ref {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: instr.type_hint.clone(),
                    });
                }
                let idx = cp.add_string(literal);
                emit_ldc_index(&mut code, idx);
                emit_astore(&mut code, dest_slot);
            }

            "str_concat" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "str_concat instruction has no dest".to_string(),
                })?;
                let left = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_concat expects left string variable, got {other:?}"),
                        })
                    }
                };
                let right = match instr.srcs.get(1) {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_concat expects right string variable, got {other:?}"),
                        })
                    }
                };
                let (left_slot, left_type) = lookup_var(left)?;
                let (right_slot, right_type) = lookup_var(right)?;
                let (dest_slot, dest_type) = lookup_var(dest_name)?;
                if left_type != JvmType::Ref || right_type != JvmType::Ref || dest_type != JvmType::Ref {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: "str".to_string(),
                    });
                }
                emit_aload(&mut code, left_slot);
                emit_aload(&mut code, right_slot);
                let concat_ref = cp.add_methodref(
                    "java/lang/String",
                    "concat",
                    "(Ljava/lang/String;)Ljava/lang/String;",
                );
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&concat_ref.to_be_bytes());
                emit_astore(&mut code, dest_slot);
            }

            "str_slice" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "str_slice instruction has no dest".to_string(),
                })?;
                let src = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_slice expects a string variable, got {other:?}"),
                        })
                    }
                };
                let start = match instr.srcs.get(1) {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_slice expects a start variable, got {other:?}"),
                        })
                    }
                };
                let end = match instr.srcs.get(2) {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_slice expects an end variable, got {other:?}"),
                        })
                    }
                };
                let (src_slot, src_type) = lookup_var(src)?;
                let (start_slot, start_type) = lookup_var(start)?;
                let (end_slot, end_type) = lookup_var(end)?;
                let (dest_slot, dest_type) = lookup_var(dest_name)?;
                if src_type != JvmType::Ref
                    || dest_type != JvmType::Ref
                    || (start_type != JvmType::Int && start_type != JvmType::Long)
                    || (end_type != JvmType::Int && end_type != JvmType::Long)
                {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: "str_slice".to_string(),
                    });
                }
                emit_aload(&mut code, src_slot);
                emit_typed_load(&mut code, start_slot, start_type);
                if start_type == JvmType::Long {
                    code.push(L2I);
                }
                emit_typed_load(&mut code, end_slot, end_type);
                if end_type == JvmType::Long {
                    code.push(L2I);
                }
                let substring_ref =
                    cp.add_methodref("java/lang/String", "substring", "(II)Ljava/lang/String;");
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&substring_ref.to_be_bytes());
                emit_astore(&mut code, dest_slot);
            }

            "str_len" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "str_len instruction has no dest".to_string(),
                })?;
                let src = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_len expects a string variable, got {other:?}"),
                        })
                    }
                };
                let (src_slot, src_type) = lookup_var(src)?;
                if src_type != JvmType::Ref {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: "str".to_string(),
                    });
                }
                let (dest_slot, dest_type) = lookup_var(dest_name)?;
                if dest_type != JvmType::Int && dest_type != JvmType::Long {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: instr.type_hint.clone(),
                    });
                }
                emit_aload(&mut code, src_slot);
                let length_ref = cp.add_methodref("java/lang/String", "length", "()I");
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&length_ref.to_be_bytes());
                if dest_type == JvmType::Long {
                    code.push(I2L);
                }
                emit_typed_store(&mut code, dest_slot, dest_type);
            }

            "str_index" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "str_index instruction has no dest".to_string(),
                })?;
                let src = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_index expects a string variable, got {other:?}"),
                        })
                    }
                };
                let idx = match instr.srcs.get(1) {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_index expects an index variable, got {other:?}"),
                        })
                    }
                };
                let (src_slot, src_type) = lookup_var(src)?;
                let (idx_slot, idx_type) = lookup_var(idx)?;
                if src_type != JvmType::Ref || (idx_type != JvmType::Int && idx_type != JvmType::Long) {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: "str_index".to_string(),
                    });
                }
                let (dest_slot, dest_type) = lookup_var(dest_name)?;
                if dest_type != JvmType::Int && dest_type != JvmType::Long {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: instr.type_hint.clone(),
                    });
                }
                emit_aload(&mut code, src_slot);
                emit_typed_load(&mut code, idx_slot, idx_type);
                if idx_type == JvmType::Long {
                    code.push(L2I);
                }
                let char_at_ref = cp.add_methodref("java/lang/String", "charAt", "(I)C");
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&char_at_ref.to_be_bytes());
                if dest_type == JvmType::Long {
                    code.push(I2L);
                }
                emit_typed_store(&mut code, dest_slot, dest_type);
            }

            "str_eq" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "str_eq instruction has no dest".to_string(),
                })?;
                let left = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_eq expects left string variable, got {other:?}"),
                        })
                    }
                };
                let right = match instr.srcs.get(1) {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_eq expects right string variable, got {other:?}"),
                        })
                    }
                };
                let (left_slot, left_type) = lookup_var(left)?;
                let (right_slot, right_type) = lookup_var(right)?;
                if left_type != JvmType::Ref || right_type != JvmType::Ref {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: "str".to_string(),
                    });
                }
                let (dest_slot, dest_type) = lookup_var(dest_name)?;
                if dest_type != JvmType::Int && dest_type != JvmType::Long {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: instr.type_hint.clone(),
                    });
                }
                emit_aload(&mut code, left_slot);
                emit_aload(&mut code, right_slot);
                let equals_ref =
                    cp.add_methodref("java/lang/String", "equals", "(Ljava/lang/Object;)Z");
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&equals_ref.to_be_bytes());
                if dest_type == JvmType::Long {
                    code.push(I2L);
                }
                emit_typed_store(&mut code, dest_slot, dest_type);
            }

            "str_cmp" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "str_cmp instruction has no dest".to_string(),
                })?;
                let left = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_cmp expects left string variable, got {other:?}"),
                        })
                    }
                };
                let right = match instr.srcs.get(1) {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("str_cmp expects right string variable, got {other:?}"),
                        })
                    }
                };
                let (left_slot, left_type) = lookup_var(left)?;
                let (right_slot, right_type) = lookup_var(right)?;
                if left_type != JvmType::Ref || right_type != JvmType::Ref {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: "str".to_string(),
                    });
                }
                let (dest_slot, dest_type) = lookup_var(dest_name)?;
                if dest_type != JvmType::Int && dest_type != JvmType::Long {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: instr.type_hint.clone(),
                    });
                }
                emit_aload(&mut code, left_slot);
                emit_aload(&mut code, right_slot);
                let compare_ref =
                    cp.add_methodref("java/lang/String", "compareTo", "(Ljava/lang/String;)I");
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&compare_ref.to_be_bytes());
                let signum_ref = cp.add_methodref("java/lang/Integer", "signum", "(I)I");
                code.push(INVOKESTATIC);
                code.extend_from_slice(&signum_ref.to_be_bytes());
                if dest_type == JvmType::Long {
                    code.push(I2L);
                }
                emit_typed_store(&mut code, dest_slot, dest_type);
            }

            "print_str" => {
                if instr.dest.is_some() {
                    return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "print_str is a side-effecting op and must not have a dest".to_string(),
                    });
                }
                let src = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s,
                    other => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("print_str expects a string variable, got {other:?}"),
                        })
                    }
                };
                let (src_slot, src_type) = lookup_var(src)?;
                if src_type != JvmType::Ref {
                    return Err(IIRJvmError::UnsupportedType {
                        function: fname.clone(),
                        type_hint: instr.type_hint.clone(),
                    });
                }
                let out_ref = cp.add_fieldref(
                    "java/lang/System",
                    "out",
                    "Ljava/io/PrintStream;",
                );
                code.push(GETSTATIC);
                code.extend_from_slice(&out_ref.to_be_bytes());
                emit_aload(&mut code, src_slot);
                let print_ref = cp.add_methodref(
                    "java/io/PrintStream",
                    "print",
                    "(Ljava/lang/String;)V",
                );
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&print_ref.to_be_bytes());
            }

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
            // The operand is an immediate: Int, Float, Bool, or (for nil) any
            // value when type_hint is `"ref<LispyPair>"`.
            //
            // # `nil` constant (`const ref<LispyPair>`)
            //
            // The Lispy nil value is represented as `null` on the JVM.
            // When `type_hint == "ref<LispyPair>"` we emit `aconst_null` + `astore`
            // regardless of the source operand (the frontend typically passes
            // Int(0) to signal nil, but we ignore the value).
            "const" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "const instruction has no dest".to_string(),
                })?;

                let (dest_slot, dest_type) = lookup_var(dest_name)?;

                // Special case: const ref<LispyPair> = nil → aconst_null
                if dest_type == JvmType::Ref {
                    code.push(ACONST_NULL);
                    emit_astore(&mut code, dest_slot);
                    i += 1;
                    continue;
                }

                let src = instr.srcs.first().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "const instruction has no source operand".to_string(),
                })?;

                match src {
                    Operand::Int(v) => {
                        match dest_type {
                            JvmType::Long => emit_lconst_cp(&mut code, cp, *v),
                            JvmType::Float => emit_fconst(&mut code, *v as f32),
                            JvmType::Double => emit_dconst_cp(&mut code, cp, *v as f64),
                            _ => emit_iconst_cp(&mut code, cp, *v as i32),
                        }
                    }
                    Operand::Bool(b) => {
                        // Booleans are represented as int 0 or 1 on the JVM.
                        emit_iconst(&mut code, if *b { 1 } else { 0 });
                    }
                    Operand::Float(f) => {
                        match dest_type {
                            JvmType::Float => emit_fconst(&mut code, *f as f32),
                            JvmType::Double => emit_dconst_cp(&mut code, cp, *f),
                            _ => {
                                // Integer destination with float source — unusual
                                // but not necessarily wrong (e.g. casting).
                                emit_iconst_cp(&mut code, cp, *f as i32);
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
                    // LANG32: Str is a compile-time string literal (global variable name).
                    // The JVM backend doesn't yet support string-value constants; skip.
                    Operand::Str(_) => {
                        i += 1;
                        continue;
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
                        i += 1;
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
                // E2: wrap a narrow (u4/u8/u16) result; u32/i32 already wrap via
                // the i32 op, i64 via the long op.
                emit_jvm_width_mask(&mut code, cp, &instr.type_hint, instr_jtype);

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
                // E2: a narrow `neg` is `(0 - r)` mod-2ⁿ — mask it to the width.
                emit_jvm_width_mask(&mut code, cp, &instr.type_hint, instr_jtype);
                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, instr_jtype);
                }
            }

            // ── E8: int_to_real ─────────────────────────────────────────────
            //
            // Widen an integer to a double.  We load the source with *its own*
            // jtype (an integer is sometimes modelled as JVM `int`, sometimes
            // as `long` — see the dual-value-model note) and pick `i2d` vs
            // `l2d` to match, then store into the (double) destination slot.
            "int_to_real" => {
                let src = one_src(func, instr, &slots)?;
                emit_typed_load(&mut code, src.0, src.1);
                code.push(match src.1 {
                    JvmType::Long => L2D,
                    _ => I2D,
                });
                if let Some(dest) = &instr.dest {
                    let (dest_slot, dest_jtype) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, dest_jtype);
                }
            }

            // ── E8: real_to_int_trunc ────────────────────────────────────────
            //
            // Narrow a double to an integer, truncating toward zero.  `d2i` /
            // `d2l` *are* the truncate-toward-zero opcodes, so this is a single
            // instruction; we pick the width from the destination slot's jtype.
            "real_to_int_trunc" => {
                let src = one_src(func, instr, &slots)?;
                emit_typed_load(&mut code, src.0, src.1);
                if let Some(dest) = &instr.dest {
                    let (dest_slot, dest_jtype) = lookup_var(dest)?;
                    code.push(match dest_jtype {
                        JvmType::Long => D2L,
                        _ => D2I,
                    });
                    emit_typed_store(&mut code, dest_slot, dest_jtype);
                }
            }

            // ── E8: real_to_int_floor (ALGOL `entier`) ──────────────────────
            //
            // Round toward −∞, then land in the integer model.  There is no
            // single "floor-to-int" opcode, so we call `Math.floor(D)D` (rounds
            // toward −∞, still a double) and *then* `d2l`/`d2i` (which now only
            // drops a `.0` fraction, so the truncate direction no longer
            // matters).  For 2.7 → floor → 2.0 → 2; for −2.7 → floor → −3.0 →
            // −3 (vs −2 for a bare truncate), which is the `entier` contract.
            "real_to_int_floor" => {
                let src = one_src(func, instr, &slots)?;
                emit_typed_load(&mut code, src.0, src.1);
                let mref = cp.add_methodref("java/lang/Math", "floor", "(D)D");
                code.push(INVOKESTATIC);
                code.extend_from_slice(&mref.to_be_bytes());
                if let Some(dest) = &instr.dest {
                    let (dest_slot, dest_jtype) = lookup_var(dest)?;
                    code.push(match dest_jtype {
                        JvmType::Long => D2L,
                        _ => D2I,
                    });
                    emit_typed_store(&mut code, dest_slot, dest_jtype);
                }
            }
            // ── AL8 sqrt + transcendentals: java/lang/Math calls ─────────────
            //
            // `java/lang/Math.sqrt(D)D` is an intrinsic in every modern JVM —
            // HotSpot lowers it directly to `sqrtsd` on x86_64 with no JNI
            // overhead.  NaN propagates and negative inputs return NaN, matching
            // IEEE-754 and the VM handler's `f64::sqrt()` contract.
            //
            // `sin`/`cos`/`exp` map directly to the Java method names; ALGOL's
            // `ln` maps to `java/lang/Math.log(D)D` (Java's natural log).
            "f64_sqrt" | "f64_sin" | "f64_cos" | "f64_ln" | "f64_exp"
            | "f64_atan" | "f64_tan" => {
                let java_method = match instr.op.as_str() {
                    "f64_sqrt" => "sqrt",
                    "f64_sin"  => "sin",
                    "f64_cos"  => "cos",
                    "f64_ln"   => "log",
                    "f64_exp"  => "exp",
                    "f64_atan" => "atan",
                    "f64_tan"  => "tan",
                    _ => unreachable!(),
                };
                let src = one_src(func, instr, &slots)?;
                emit_typed_load(&mut code, src.0, src.1);
                let mref = cp.add_methodref("java/lang/Math", java_method, "(D)D");
                code.push(INVOKESTATIC);
                code.extend_from_slice(&mref.to_be_bytes());
                if let Some(dest) = &instr.dest {
                    let (dest_slot, dest_jtype) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, dest_jtype);
                }
            }
            // ── BA-pow: two-argument pow(base, exp) via java/lang/Math.pow ───
            //
            // `Math.pow(DD)D` handles all IEEE-754 edge cases (NaN, ±inf,
            // negative bases with fractional exponents → NaN) matching every
            // other backend and the VM handler's `f64::powf` contract.
            "f64_pow" => {
                let (src0, src1) = two_srcs(func, instr, &slots)?;
                emit_typed_load(&mut code, src0.0, src0.1); // base
                emit_typed_load(&mut code, src1.0, src1.1); // exp
                let mref = cp.add_methodref("java/lang/Math", "pow", "(DD)D");
                code.push(INVOKESTATIC);
                code.extend_from_slice(&mref.to_be_bytes());
                if let Some(dest) = &instr.dest {
                    let (dest_slot, dest_jtype) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, dest_jtype);
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
                // E2: keep a narrow bitwise result canonical for its width.
                emit_jvm_width_mask(&mut code, cp, &instr.type_hint, instr_jtype);
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
                // E2: `~x` on a narrow width must flip only its low bits
                // (`~0u8 == 255`, not `-1`) — mask after the XOR.
                emit_jvm_width_mask(&mut code, cp, &instr.type_hint, instr_jtype);
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
                // E2: a narrow left-shift can push bits past the width
                // (`1u8 << 8`), so mask the result.
                emit_jvm_width_mask(&mut code, cp, &instr.type_hint, instr_jtype);
                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_typed_store(&mut code, dest_slot, instr_jtype);
                }
            }

            // ── Comparisons ──────────────────────────────────────────────────
            //
            // For integer (`int`) operands we use the 8-byte `emit_int_compare`
            // pattern with `if_icmpXX` (two-int branch instructions).
            //
            // For `long` operands (type "i64") we use the 9-byte
            // `emit_long_compare` pattern: `lload` both values, `lcmp` to
            // get an int result, then a unary `ifXX` branch.
            //
            // The comparison result is always stored as an int (0 or 1) in the
            // destination slot, regardless of operand type.
            "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
                let (src0, src1) = two_srcs(func, instr, &slots)?;

                if src0.1 == JvmType::Double {
                    // Double comparison (LANG-FULL E3): dload both, `dcmpl`/`dcmpg`
                    // to an int -1/0/1, then the same unary `ifXX` branch the long
                    // path uses. Without this branch a `real` comparison fell into
                    // the `else` int path, which `iload`ed a two-slot double as a
                    // single int and used `if_icmpne` — the verifier rejected the
                    // class (empty output). `dcmpg` is chosen for `>`/`>=` so a
                    // NaN operand makes them false (NaN → +1); `dcmpl` for the
                    // rest (NaN → -1) — matching javac's convention.
                    emit_typed_load(&mut code, src0.0, JvmType::Double);
                    emit_typed_load(&mut code, src1.0, JvmType::Double);
                    let (dcmp, branch) = match instr.op.as_str() {
                        "cmp_eq" => (DCMPL, IFNE), // skip true when result ≠ 0
                        "cmp_ne" => (DCMPL, IFEQ), // skip true when result = 0
                        "cmp_lt" => (DCMPL, IFGE), // skip true when result ≥ 0
                        "cmp_le" => (DCMPL, IFGT), // skip true when result > 0
                        "cmp_gt" => (DCMPG, IFLE), // skip true when result ≤ 0
                        "cmp_ge" => (DCMPG, IFLT), // skip true when result < 0
                        _ => (DCMPL, IFNE),
                    };
                    emit_double_compare(&mut code, dcmp, branch);
                } else if src0.1 == JvmType::Long {
                    // Long comparison: lload both, lcmp, then unary ifXX.
                    emit_typed_load(&mut code, src0.0, JvmType::Long);
                    emit_typed_load(&mut code, src1.0, JvmType::Long);
                    let cmp_opcode = match instr.op.as_str() {
                        // Negated unary branch opcodes (ifXX operates on lcmp int result).
                        "cmp_eq" => IFNE, // skip true when result ≠ 0 (not equal)
                        "cmp_ne" => IFEQ, // skip true when result = 0 (equal)
                        "cmp_lt" => IFGE, // skip true when result ≥ 0 (not less)
                        "cmp_le" => IFGT, // skip true when result > 0 (greater)
                        "cmp_gt" => IFLE, // skip true when result ≤ 0 (not greater)
                        "cmp_ge" => IFLT, // skip true when result < 0 (less)
                        _ => IFNE,
                    };
                    emit_long_compare(&mut code, cmp_opcode);
                } else {
                    // Int comparison: iload both, if_icmpXX.
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
                }
                // Result (0 or 1) is now on the stack; store it.
                if let Some(dest) = &instr.dest {
                    let (dest_slot, _) = lookup_var(dest)?;
                    emit_istore(&mut code, dest_slot);
                }
            }

            // ── mov (copy) ────────────────────────────────────────────────────
            //
            // `mov rd, rs` — copy a value from one variable to another.
            // This is the IIR encoding of the Twig `_move` builtin, emitted by
            // the compiler for if-expression arm unification and function-call
            // result forwarding.
            //
            // JVM sequence:
            //   <typed-load rs>    ← push rs onto the JVM operand stack
            //   <typed-store rd>   ← pop and store into rd's local slot
            //
            // We use the source variable's type to pick the right load/store
            // opcodes (iload/lload/fload/dload and their store mirrors).
            "mov" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "mov must have a dest".to_string(),
                })?;
                let (dest_slot, dest_type) = lookup_var(dest_name)?;

                match instr.srcs.first() {
                    Some(Operand::Var(src_name)) => {
                        let (src_slot, src_type) = lookup_var(src_name)?;
                        emit_typed_load(&mut code, src_slot, src_type);
                        // The source and dest slots can differ in width — e.g. a
                        // bool/int comparison result mov'd into a `long`
                        // accumulator (Oct's short-circuit `&&`/`||` over its i64
                        // value model, which — unlike the concretized-to-i32
                        // scalar path — keeps values `long`). Storing with the
                        // *source* type into a wider dest slot leaves the slot's
                        // second half uninitialized, which a later `lload` trips
                        // (`VerifyError: uninitialized register pair`). Bridge
                        // int↔long so the store matches the dest slot's width.
                        match (src_type, dest_type) {
                            (JvmType::Int, JvmType::Long) => code.push(I2L),
                            (JvmType::Long, JvmType::Int) => code.push(L2I),
                            _ => {}
                        }
                        emit_typed_store(&mut code, dest_slot, dest_type);
                    }
                    Some(Operand::Int(v)) => {
                        // Constant mov — unusual but valid; emit iconst, widening
                        // to long if the dest slot is `long` (same width rule).
                        emit_iconst_cp(&mut code, cp, *v as i32);
                        if dest_type == JvmType::Long {
                            code.push(I2L);
                        }
                        emit_typed_store(&mut code, dest_slot, dest_type);
                    }
                    Some(Operand::Bool(b)) => {
                        emit_iconst(&mut code, if *b { 1 } else { 0 });
                        if dest_type == JvmType::Long {
                            code.push(I2L);
                        }
                        emit_typed_store(&mut code, dest_slot, dest_type);
                    }
                    _ => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: "mov: first src must be Var, Int, or Bool".to_string(),
                        });
                    }
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
                let (cond_slot, cond_ty) = lookup_var(cond_src)?;
                // `ifne` tests an int != 0. An i64 condition (the widened
                // Brainfuck loop guard, LANG-MATRIX LM-J) must first be reduced
                // to an int: `lload; lconst_0; lcmp` pushes -1/0/+1, which `ifne`
                // then branches on. `iload`ing a long would read only one of its
                // two slots — a verify error.
                if cond_ty == JvmType::Long {
                    emit_lload(&mut code, cond_slot);
                    code.push(LCONST_0);
                    code.push(LCMP);
                } else {
                    emit_iload(&mut code, cond_slot);
                }
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
                let (cond_slot, cond_ty) = lookup_var(cond_src)?;
                // "branch if zero" — same width handling as `jmp_if_true`: an
                // i64 guard is reduced via `lload; lconst_0; lcmp` before `ifeq`.
                if cond_ty == JvmType::Long {
                    emit_lload(&mut code, cond_slot);
                    code.push(LCONST_0);
                    code.push(LCMP);
                } else {
                    emit_iload(&mut code, cond_slot);
                }
                let opcode_pos = code.len();
                code.push(IFEQ);
                code.extend_from_slice(&0i16.to_be_bytes()); // placeholder
                fixups.push(Fixup { opcode_pos, target: label });
            }

            // ── ret ───────────────────────────────────────────────────────────
            //
            // Return a value from the function.  Load the result variable onto
            // the stack, then emit the appropriate typed return opcode.
            //
            // For `ref<LispyPair>` return types we use `areturn` (0xB0) — the
            // JVM object-reference return instruction.
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
                            // `areturn` (0xB0) — return an object reference.
                            // Used when a function returns a LispyPair ref.
                            JvmType::Ref => 0xB0,
                        };
                        code.push(ret_opcode);
                    }
                    Operand::Int(v) => {
                        let ret_type = iir_type_to_jvm(&func.return_type).unwrap_or(JvmType::Int);
                        match ret_type {
                            JvmType::Long => {
                                emit_lconst_cp(&mut code, cp, *v);
                                code.push(LRETURN);
                            }
                            _ => {
                                emit_iconst_cp(&mut code, cp, *v as i32);
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
                                emit_dconst_cp(&mut code, cp, *f);
                                code.push(DRETURN);
                            }
                            _ => {
                                emit_iconst_cp(&mut code, cp, *f as i32);
                                code.push(IRETURN);
                            }
                        }
                    }
                    // LANG32: Str is a compile-time string literal (global variable name).
                    // Returning a string from a function is not supported in V1.
                    Operand::Str(_) => {
                        return Err(IIRJvmError::UnsupportedOp {
                            function: fname.clone(),
                            op: "ret with Str operand — string return values not yet supported".into(),
                        });
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
            // ── load_mem (Brainfuck) ─────────────────────────────────────────
            //
            // `load_mem  v  ptr  u8`  → read tape[ptr] into `v`.
            //
            // The tape is `env/BFRuntime.__tape : [B`, a static byte array
            // provided by the host class.  JVM `baload` pops [array, index]
            // and pushes the byte at that index sign-extended to an int.
            // We mask with `& 0xFF` to match BF's unsigned u8 semantics
            // (cells are u8 — the high bits of the int must be zero).
            //
            // Emitted sequence (5 bytes + 1 = 6):
            //
            //   GETSTATIC env/BFRuntime.__tape : [B    ; push array ref
            //   ILOAD     <ptr_slot>                    ; push index
            //   BALOAD                                  ; pop[arr,idx], push byte
            //   SIPUSH    0x00FF                        ; push mask (2 bytes inline)
            //   IAND                                    ; mask off sign-extended bits
            //   ISTORE    <dest_slot>                   ; store as u8 value
            //
            // We use SIPUSH + IAND rather than i2b because `i2b` re-sign-
            // extends, which is exactly the opposite of what we want.
            "load_mem" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| {
                    IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "load_mem must have a dest".to_string(),
                    }
                })?;
                let addr_name = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "load_mem src[0] must be Operand::Var(addr)".to_string(),
                    }),
                };
                let (addr_slot, _) = lookup_var(&addr_name)?;
                let (dest_slot, _) = lookup_var(dest_name)?;

                let tape_fieldref = cp.add_fieldref(BF_RUNTIME_CLASS, "__tape", "[B");
                code.push(GETSTATIC);
                code.extend_from_slice(&tape_fieldref.to_be_bytes());
                emit_iload(&mut code, addr_slot);
                code.push(BALOAD);
                // Mask sign-extended byte back into u8 (0..=255 int).
                code.push(SIPUSH);
                code.extend_from_slice(&0x00FFi16.to_be_bytes());
                code.push(IAND);
                emit_istore(&mut code, dest_slot);
            }

            // ── store_mem (Brainfuck) ────────────────────────────────────────
            //
            // `store_mem  ptr  v  u8`  → write low byte of v into tape[ptr].
            //
            // JVM `bastore` pops [array, index, value] and stores
            // `value & 0xFF` at `array[index]`.  Truncation matches BF's u8
            // tape — overflow / sign of `v` is irrelevant.
            //
            // Emitted sequence (4 bytes + 1 = 5):
            //
            //   GETSTATIC env/BFRuntime.__tape : [B    ; push array ref
            //   ILOAD     <ptr_slot>                    ; push index
            //   ILOAD     <val_slot>                    ; push value
            //   BASTORE                                 ; pop[arr,idx,val]
            "store_mem" => {
                if instr.srcs.len() < 2 {
                    return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "store_mem requires 2 srcs: [addr, val]".to_string(),
                    });
                }
                let addr_name = match &instr.srcs[0] {
                    Operand::Var(s) => s.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "store_mem src[0] must be Operand::Var(addr)".to_string(),
                    }),
                };
                let val_name = match &instr.srcs[1] {
                    Operand::Var(s) => s.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "store_mem src[1] must be Operand::Var(val)".to_string(),
                    }),
                };
                let (addr_slot, _) = lookup_var(&addr_name)?;
                let (val_slot, _)  = lookup_var(&val_name)?;

                let tape_fieldref = cp.add_fieldref(BF_RUNTIME_CLASS, "__tape", "[B");
                code.push(GETSTATIC);
                code.extend_from_slice(&tape_fieldref.to_be_bytes());
                emit_iload(&mut code, addr_slot);
                emit_iload(&mut code, val_slot);
                code.push(BASTORE);
            }

            // ── alloc_bytes (LANG-MATRIX LM-J Brainfuck) ─────────────────────
            //
            // `alloc_bytes  dest  <-  size`.  The JVM tape is the host class's
            // pre-allocated static field `env/BFRuntime.__tape : [B`, so there
            // is nothing to allocate at runtime — this is a no-op.  `dest` (the
            // BF tape base, `__bf_tape`) is therefore never materialised: the
            // `load_byte`/`store_byte` ops below `getstatic` the tape directly
            // and ignore the base operand (it is always 0 in this pipeline).
            // This mirrors the LLVM/WASM lowering's "tape at a fixed base," just
            // with the base implicit in the static field rather than a pointer.
            "alloc_bytes" => {
                // Intentionally emits no bytecode.
            }

            // ── load_byte (LANG-MATRIX LM-J Brainfuck) ───────────────────────
            //
            // `load_byte  dest  <-  base, idx`.  Read one tape cell, unsigned.
            // The lowered form of the BF `load_mem` above: same `getstatic
            // __tape; <idx>; baload; & 0xFF` shape, but the operands may be
            // `i64` (the widened BF value model) rather than `i32` — so we
            // narrow an `i64` index to `int` with `l2i` for `baload`, and widen
            // the masked `int` cell back to `i64` with `i2l` for an `i64` dest.
            // The base operand is the static tape, so it is ignored.
            "load_byte" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| {
                    IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "load_byte must have a dest".to_string(),
                    }
                })?;
                let idx_name = match instr.srcs.get(1) {
                    Some(Operand::Var(s)) => s.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "load_byte requires Operand::Var(idx) as src[1]".to_string(),
                    }),
                };
                let (idx_slot, idx_ty) = lookup_var(&idx_name)?;
                let (dest_slot, dest_ty) = lookup_var(dest_name)?;

                let tape_fieldref = cp.add_fieldref(BF_RUNTIME_CLASS, "__tape", "[B");
                code.push(GETSTATIC);
                code.extend_from_slice(&tape_fieldref.to_be_bytes());
                emit_typed_load(&mut code, idx_slot, idx_ty);
                if idx_ty == JvmType::Long {
                    code.push(L2I); // baload needs an int index
                }
                code.push(BALOAD);
                // Mask the sign-extended byte back into an unsigned 0..=255 int.
                code.push(SIPUSH);
                code.extend_from_slice(&0x00FFi16.to_be_bytes());
                code.push(IAND);
                if dest_ty == JvmType::Long {
                    code.push(I2L); // widen the cell to the i64 dest register
                }
                emit_typed_store(&mut code, dest_slot, dest_ty);
            }

            // ── store_byte (LANG-MATRIX LM-J Brainfuck) ──────────────────────
            //
            // `store_byte  base, idx, val`  (no dest).  Write the low byte of
            // `val` into `tape[idx]`.  The lowered form of `store_mem`; `bastore`
            // stores `val & 0xFF` (so BF's 8-bit cell wrap-around is free).  An
            // `i64` index / value is narrowed with `l2i` before the array op.
            // The base operand is the static tape, so it is ignored.
            "store_byte" => {
                if instr.dest.is_some() {
                    return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "store_byte must not have a dest".to_string(),
                    });
                }
                if instr.srcs.len() < 3 {
                    return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "store_byte requires 3 srcs: [base, idx, val]".to_string(),
                    });
                }
                let idx_name = match &instr.srcs[1] {
                    Operand::Var(s) => s.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "store_byte src[1] must be Operand::Var(idx)".to_string(),
                    }),
                };
                let val_name = match &instr.srcs[2] {
                    Operand::Var(s) => s.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "store_byte src[2] must be Operand::Var(val)".to_string(),
                    }),
                };
                let (idx_slot, idx_ty) = lookup_var(&idx_name)?;
                let (val_slot, val_ty) = lookup_var(&val_name)?;

                let tape_fieldref = cp.add_fieldref(BF_RUNTIME_CLASS, "__tape", "[B");
                code.push(GETSTATIC);
                code.extend_from_slice(&tape_fieldref.to_be_bytes());
                emit_typed_load(&mut code, idx_slot, idx_ty);
                if idx_ty == JvmType::Long {
                    code.push(L2I);
                }
                emit_typed_load(&mut code, val_slot, val_ty);
                if val_ty == JvmType::Long {
                    code.push(L2I);
                }
                code.push(BASTORE);
            }

            // ── alloc_array (LANG-FULL E5) ───────────────────────────────────
            //
            // `alloc_array  dest  <-  count`  (type_hint `array<T>`).  Allocate a
            // fresh JVM primitive array `new T[count]` and bind `dest` to its
            // reference.  `newarray` takes an **int** count and a one-byte element
            // type code (T_INT/T_LONG/T_DOUBLE/…), so an `i64` count is narrowed
            // with `l2i` first.  JVM arrays are zero-initialised, matching the
            // reference VM's default-init.  `dest` is a `Ref` local → `astore`.
            //
            //   <count>; [l2i]; newarray <atype>; astore dest
            "alloc_array" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| {
                    IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "alloc_array must have a dest".to_string(),
                    }
                })?;
                let count_name = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "alloc_array requires Operand::Var(count) as src[0]".to_string(),
                    }),
                };
                let elem_hint = array_elem_type(&instr.type_hint).ok_or_else(|| {
                    IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: format!("alloc_array type_hint must be array<T>, got {:?}", instr.type_hint),
                    }
                })?;
                let (count_slot, count_ty) = lookup_var(&count_name)?;
                let (dest_slot, _dest_ty) = lookup_var(dest_name)?;
                emit_typed_load(&mut code, count_slot, count_ty);
                if count_ty == JvmType::Long {
                    code.push(L2I); // newarray/anewarray count is an int
                }
                if let Some(class) = jvm_ref_array_element_class(&elem_hint) {
                    // E4d-BA-arr: reference-element array (`array<str>` → `String[]`)
                    // allocates with `anewarray <class cp index>` (not `newarray`).
                    let cidx = cp.add_class(class);
                    code.push(ANEWARRAY);
                    code.extend_from_slice(&cidx.to_be_bytes());
                } else {
                    let (atype, _, _) = iir_type_to_jvm(&elem_hint).and_then(array_element_opcodes)
                        .ok_or_else(|| IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("alloc_array element type {elem_hint:?} is not a supported JVM array element"),
                        })?;
                    code.push(NEWARRAY);
                    code.push(atype);
                }
                emit_astore(&mut code, dest_slot);
            }

            // ── array_get (LANG-FULL E5) ─────────────────────────────────────
            //
            // `array_get  dest  <-  handle, idx`  (type_hint = element `T`).  Read
            // `handle[idx]`.  The handle is a `Ref` local (`aload`); the index is
            // narrowed to `int` if it arrived as `i64`; the typed `*aload` does the
            // **native bounds check** (OOB → `ArrayIndexOutOfBoundsException`, i.e.
            // E5's trap).  `dest` shares `T` with the load, so no conversion.
            //
            //   aload handle; <idx>; [l2i]; <Taload>; store dest
            "array_get" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| {
                    IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "array_get must have a dest".to_string(),
                    }
                })?;
                let handle_name = array_var_operand(instr, 0, "array_get", "handle", &fname)?;
                let idx_name = array_var_operand(instr, 1, "array_get", "idx", &fname)?;
                // E4d-BA-arr: a `str` element loads with `aaload` (reference element);
                // primitive elements use the typed `*aload` from `array_element_opcodes`.
                let aload_op = if jvm_ref_array_element_class(&instr.type_hint).is_some() {
                    AALOAD
                } else {
                    iir_type_to_jvm(&instr.type_hint)
                        .and_then(array_element_opcodes)
                        .map(|(_, aload_op, _)| aload_op)
                        .ok_or_else(|| IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("array_get element type {:?} is not a supported JVM array element", instr.type_hint),
                        })?
                };
                let (h_slot, _) = lookup_var(&handle_name)?;
                let (i_slot, i_ty) = lookup_var(&idx_name)?;
                let (dest_slot, dest_ty) = lookup_var(dest_name)?;
                emit_aload(&mut code, h_slot);
                emit_typed_load(&mut code, i_slot, i_ty);
                if i_ty == JvmType::Long {
                    code.push(L2I);
                }
                code.push(aload_op);
                emit_typed_store(&mut code, dest_slot, dest_ty);
            }

            // ── array_set (LANG-FULL E5) ─────────────────────────────────────
            //
            // `array_set  handle, idx, val`  (type_hint = element `T`, no dest).
            // Write `handle[idx] = val`.  The typed `*astore` bounds-checks the
            // index natively (OOB → AIOOBE).
            //
            //   aload handle; <idx>; [l2i]; <val>; <Tastore>
            "array_set" => {
                if instr.dest.is_some() {
                    return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "array_set must not have a dest".to_string(),
                    });
                }
                let handle_name = array_var_operand(instr, 0, "array_set", "handle", &fname)?;
                let idx_name = array_var_operand(instr, 1, "array_set", "idx", &fname)?;
                let val_name = array_var_operand(instr, 2, "array_set", "val", &fname)?;
                // E4d-BA-arr: a `str` element stores with `aastore` (reference element);
                // primitive elements use the typed `*astore` from `array_element_opcodes`.
                let astore_op = if jvm_ref_array_element_class(&instr.type_hint).is_some() {
                    AASTORE
                } else {
                    iir_type_to_jvm(&instr.type_hint)
                        .and_then(array_element_opcodes)
                        .map(|(_, _, astore_op)| astore_op)
                        .ok_or_else(|| IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: format!("array_set element type {:?} is not a supported JVM array element", instr.type_hint),
                        })?
                };
                let (h_slot, _) = lookup_var(&handle_name)?;
                let (i_slot, i_ty) = lookup_var(&idx_name)?;
                let (v_slot, v_ty) = lookup_var(&val_name)?;
                emit_aload(&mut code, h_slot);
                emit_typed_load(&mut code, i_slot, i_ty);
                if i_ty == JvmType::Long {
                    code.push(L2I);
                }
                emit_typed_load(&mut code, v_slot, v_ty);
                code.push(astore_op);
            }

            // ── array_len (LANG-FULL E5) ─────────────────────────────────────
            //
            // `array_len  dest  <-  handle`.  `arraylength` yields an `int`; widen
            // to `long` with `i2l` when the dest is `i64`.
            //
            //   aload handle; arraylength; [i2l]; store dest
            "array_len" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| {
                    IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "array_len must have a dest".to_string(),
                    }
                })?;
                let handle_name = array_var_operand(instr, 0, "array_len", "handle", &fname)?;
                let (h_slot, _) = lookup_var(&handle_name)?;
                let (dest_slot, dest_ty) = lookup_var(dest_name)?;
                emit_aload(&mut code, h_slot);
                code.push(ARRAYLENGTH);
                if dest_ty == JvmType::Long {
                    code.push(I2L);
                }
                emit_typed_store(&mut code, dest_slot, dest_ty);
            }

            // ── call_builtin (Brainfuck putchar / getchar) ───────────────────
            //
            // The validator (validate.rs) enforces that `srcs[0]` is in
            // [`CALL_BUILTIN_SUPPORTED_NAMES`], so the inner match here only
            // handles whitelisted names.  Falling off the match indicates a
            // validator/lowerer drift and returns `UnsupportedOp` as a safety
            // net.
            //
            // | Builtin   | Operand layout                          | Bytecode emitted |
            // |-----------|------------------------------------------|-------------------|
            // | `putchar`   | srcs = [Var("putchar"), Var(val)]; no dest    | [l]load val [l2i]; invokestatic env/BFRuntime.putchar(I)V |
            // | `getchar`   | srcs = [Var("getchar")]; dest = byte slot     | invokestatic env/BFRuntime.getchar()I; istore dest |
            // | `print_i64` | srcs = [Var("print_i64"), Var(val:i64)]; no dest | lload val; invokestatic env/BasicRuntime.println(J)V |
            "call_builtin" => {
                let builtin_name = match instr.srcs.first() {
                    Some(Operand::Var(s)) => s.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "call_builtin: srcs[0] must be the builtin name as Operand::Var".to_string(),
                    }),
                };
                match builtin_name.as_str() {
                    "putchar" => {
                        // putchar takes one i32 arg and returns void.
                        //
                        // In the narrow i32 value model (expression programs, Brainfuck)
                        // the char value is an `Int` slot → load with `iload`.
                        //
                        // In the wide i64 value model (BASIC `INPUT` + `PRINT`, where
                        // `input_i64` forces i64 — see BA-JVM-INPUT in `lang-aot`) the
                        // char value is a `Long` slot → load with `lload` then narrow
                        // with `l2i` before handing it to `putchar(I)V`.
                        let val_name = match instr.srcs.get(1) {
                            Some(Operand::Var(s)) => s.clone(),
                            _ => return Err(IIRJvmError::InvalidOperand {
                                function: fname.clone(),
                                detail: "call_builtin \"putchar\" requires srcs[1] = Operand::Var(val)".to_string(),
                            }),
                        };
                        let (val_slot, val_type) = lookup_var(&val_name)?;
                        if val_type == JvmType::Long {
                            emit_lload(&mut code, val_slot);
                            code.push(L2I);
                        } else {
                            emit_iload(&mut code, val_slot);
                        }
                        let mref = cp.add_methodref(BF_RUNTIME_CLASS, "putchar", "(I)V");
                        code.push(INVOKESTATIC);
                        code.extend_from_slice(&mref.to_be_bytes());
                    }
                    "getchar" => {
                        // getchar takes no args, returns i32 (the byte, or -1/0
                        // for EOF — host convention).
                        let dest_name = instr.dest.as_deref().ok_or_else(|| {
                            IIRJvmError::InvalidOperand {
                                function: fname.clone(),
                                detail: "call_builtin \"getchar\" requires a dest register".to_string(),
                            }
                        })?;
                        let (dest_slot, _) = lookup_var(dest_name)?;
                        let mref = cp.add_methodref(BF_RUNTIME_CLASS, "getchar", "()I");
                        code.push(INVOKESTATIC);
                        code.extend_from_slice(&mref.to_be_bytes());
                        emit_istore(&mut code, dest_slot);
                    }
                    "print_i64" => {
                        // print_i64 takes one i64 (`long` on the JVM) arg and
                        // returns void.  This is BASIC's `PRINT` lowered: the
                        // value is loaded with `lload` (long load) and then
                        // we invokestatic the host's `println(J)V` method on
                        // [`BASIC_RUNTIME_CLASS`].  The host is responsible
                        // for the actual write — we just hand the long to it.
                        //
                        // Mirrors the wasm backend (iir-to-wasm v0.8.0) which
                        // routes the same builtin to `env.__print_i64`.
                        let val_name = match instr.srcs.get(1) {
                            Some(Operand::Var(s)) => s.clone(),
                            _ => return Err(IIRJvmError::InvalidOperand {
                                function: fname.clone(),
                                detail: "call_builtin \"print_i64\" requires srcs[1] = Operand::Var(val:i64)".to_string(),
                            }),
                        };
                        let (val_slot, _) = lookup_var(&val_name)?;
                        emit_lload(&mut code, val_slot);
                        let mref = cp.add_methodref(BASIC_RUNTIME_CLASS, "println", "(J)V");
                        code.push(INVOKESTATIC);
                        code.extend_from_slice(&mref.to_be_bytes());
                    }
                    "input_i64" => {
                        // input_i64 takes no arguments and returns one i64 (`long`).
                        // This is BASIC's `INPUT X` lowered: we call the host's
                        // `readLong()J` static method, which reads one line from stdin
                        // and parses it as a `long` (0 on EOF / parse failure — the
                        // same V1 permissive contract as `__twig_input_i64` in C).
                        // The result sits on the JVM stack as a long; `lstore` moves
                        // it into `dest`.
                        let dest_name = builtin_dest(instr, fname, "input_i64")?;
                        let (dest_slot, _) = lookup_var(dest_name)?;
                        let mref = cp.add_methodref(BASIC_RUNTIME_CLASS, "readLong", "()J");
                        code.push(INVOKESTATIC);
                        code.extend_from_slice(&mref.to_be_bytes());
                        emit_lstore(&mut code, dest_slot);
                    }
                    "input_str" => {
                        // BASIC string `INPUT A$` (E4-dyn): read a whole line *as
                        // the string value itself* — no numeric parse, unlike
                        // `input_i64`. The host `BasicRuntime.readLine()` returns a
                        // `java.lang.String`, which is exactly how a `str` value is
                        // carried on the JVM (`iir_type_to_jvm("str") = Ref`), so the
                        // returned reference `astore`s straight into the `str`-typed
                        // dest slot. `PRINT A$` then consumes it via the shared E4
                        // string path. Like `input_i64` this assumes input is present
                        // (V1 permissive contract).
                        let dest_name = builtin_dest(instr, fname, "input_str")?;
                        let (dest_slot, _) = lookup_var(dest_name)?;
                        let mref = cp.add_methodref(
                            BASIC_RUNTIME_CLASS, "readLine", "()Ljava/lang/String;");
                        code.push(INVOKESTATIC);
                        code.extend_from_slice(&mref.to_be_bytes());
                        emit_astore(&mut code, dest_slot);
                    }
                    // ── McCarthy W4: the lisp predicates (F3–F5). ──
                    //
                    // The structural pass emits these as the *same* backend-agnostic
                    // `call_builtin`s the wasm path uses (where they lower to
                    // `ref.test`/`i32.eqz`/`i31.get_s`+`i32.eq`). On the JVM the
                    // uniform-`Object` model makes them:
                    //   pair?   → `instanceof Object[]`   (a cons is an Object[])
                    //   not     → logical not of a 0/1 bool
                    //   equal?  → unbox both Integers and `if_icmpeq`
                    "pair?" => {
                        // Is the (boxed) lisp value a cons cell? A cons is an
                        // `Object[]`; an atom is an `Integer`; nil is `null`.
                        let dest_name = builtin_dest(instr, fname, "pair?")?;
                        let arg = builtin_arg(instr, fname, "pair?", 1)?;
                        let (dest_slot, _) = lookup_var(dest_name)?;
                        let (arg_slot, _) = lookup_var(&arg)?;
                        emit_aload(&mut code, arg_slot);
                        let cidx = cp.add_class("[Ljava/lang/Object;");
                        code.push(INSTANCEOF);
                        code.extend_from_slice(&cidx.to_be_bytes());
                        emit_istore(&mut code, dest_slot);
                    }
                    "not" => {
                        // Logical not of a 0/1 machine boolean: `arg ^ 1`.
                        let dest_name = builtin_dest(instr, fname, "not")?;
                        let arg = builtin_arg(instr, fname, "not", 1)?;
                        let (dest_slot, _) = lookup_var(dest_name)?;
                        let (arg_slot, _) = lookup_var(&arg)?;
                        emit_iload(&mut code, arg_slot);
                        code.push(ICONST_1);
                        code.push(IXOR);
                        emit_istore(&mut code, dest_slot);
                    }
                    "equal?" => {
                        // `EQ` on atoms: unbox both `Integer`s and compare. The
                        // structural pass guarantees both args are boxed atoms
                        // (symbols interned to ints, integers as ints), so the
                        // identity test reduces to integer equality.
                        let dest_name = builtin_dest(instr, fname, "equal?")?;
                        let a = builtin_arg(instr, fname, "equal?", 1)?;
                        let b = builtin_arg(instr, fname, "equal?", 2)?;
                        let (dest_slot, _) = lookup_var(dest_name)?;
                        let (a_slot, _) = lookup_var(&a)?;
                        let (b_slot, _) = lookup_var(&b)?;
                        let int_cidx = cp.add_class("java/lang/Integer");
                        let intval = cp.add_methodref("java/lang/Integer", "intValue", "()I");
                        // unbox a
                        emit_aload(&mut code, a_slot);
                        code.push(CHECKCAST);
                        code.extend_from_slice(&int_cidx.to_be_bytes());
                        code.push(INVOKEVIRTUAL);
                        code.extend_from_slice(&intval.to_be_bytes());
                        // unbox b
                        emit_aload(&mut code, b_slot);
                        code.push(CHECKCAST);
                        code.extend_from_slice(&int_cidx.to_be_bytes());
                        code.push(INVOKEVIRTUAL);
                        code.extend_from_slice(&intval.to_be_bytes());
                        // a == b ? 1 : 0  (IF_ICMPNE skips the true arm when a≠b)
                        emit_int_compare(&mut code, IF_ICMPNE);
                        emit_istore(&mut code, dest_slot);
                    }
                    _ => {
                        // Validator should have rejected this; defense in depth.
                        return Err(IIRJvmError::UnsupportedOp {
                            function: fname.clone(),
                            op: format!("call_builtin {:?}: not in JVM backend whitelist", builtin_name),
                        });
                    }
                }
            }

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
                        Operand::Int(v) => emit_iconst_cp(&mut code, cp, *v as i32),
                        Operand::Bool(b) => emit_iconst(&mut code, if *b { 1 } else { 0 }),
                        Operand::Float(f) => emit_fconst(&mut code, *f as f32),
                        // LANG32: Str is a compile-time string literal — not
                        // a passable argument in V1; return an error.
                        Operand::Str(_) => {
                            return Err(IIRJvmError::UnsupportedOp {
                                function: fname.clone(),
                                op: "call with Str argument — string args not yet supported".into(),
                            });
                        }
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

            // ── alloc_closure (LANG36) ───────────────────────────────────────
            //
            // Build a `long[]` array representing a closure:
            //   closure[0] = dispatch index (which function to call)
            //   closure[1..n] = captured values cast to long
            //
            // srcs[0] = Operand::Str(fn_name)  — callee name (not a variable)
            // srcs[1..] = Var(cap_i)           — captured variables
            //
            // JVM sequence emitted (n = n_captures):
            //
            //   iconst_{n+1}              ← array length = 1 (idx slot) + n (caps)
            //   newarray T_LONG           ← long[] closure_arr = new long[n+1]
            //   dup
            //   iconst_0                  ← index 0
            //   <push dispatch_idx as long>
            //   lastore                   ← closure_arr[0] = dispatch_idx
            //   dup
            //   iconst_1                  ← index 1
            //   <load cap0, widen to long if i32>
            //   lastore                   ← closure_arr[1] = cap0
            //   …
            //   astore dest_slot          ← dest = closure_arr
            "alloc_closure" => {
                let dest_name = instr
                    .dest
                    .as_deref()
                    .ok_or_else(|| IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "alloc_closure has no dest".to_string(),
                    })?;
                let fn_name = match instr.srcs.first() {
                    Some(Operand::Str(n)) => n.clone(),
                    _ => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: "alloc_closure srcs[0] must be Operand::Str(fn_name)"
                                .to_string(),
                        })
                    }
                };

                // Look up the dispatch index for this function.
                let entry = closure_dispatch.get(&fn_name).ok_or_else(|| {
                    IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: format!(
                            "alloc_closure: function {:?} not in closure dispatch table \
                             (this is an internal error — the pre-pass should have collected it)",
                            fn_name
                        ),
                    }
                })?;
                let dispatch_idx = entry.dispatch_idx;

                // Collect captured variable names (srcs[1..]).
                let captures: Vec<&str> = instr
                    .srcs
                    .iter()
                    .skip(1)
                    .map(|s| match s {
                        Operand::Var(n) => n.as_str(),
                        _ => "",
                    })
                    .collect();
                let n_captures = captures.len();

                // Validate: captures + call_args == func.params.len().
                let total_params = entry.fn_params.len();
                let n_call_args = total_params.saturating_sub(n_captures);
                if n_captures + n_call_args != total_params {
                    return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: format!(
                            "alloc_closure {:?}: capture count ({}) + expected call args ({}) \
                             ≠ function arity ({})",
                            fn_name, n_captures, n_call_args, total_params
                        ),
                    });
                }

                let (dest_slot, _) = lookup_var(dest_name)?;

                // ── Emit array allocation ────────────────────────────────────
                // iconst_{n+1} — array length
                emit_iconst(&mut code, (n_captures + 1) as i32);
                // newarray T_LONG — allocate long[]
                code.push(NEWARRAY);
                code.push(T_LONG);

                // ── Store dispatch index at closure[0] ───────────────────────
                code.push(DUP);
                code.push(ICONST_0); // index 0
                // Push dispatch_idx as a long constant.
                match dispatch_idx {
                    0 => code.push(LCONST_0),
                    1 => code.push(LCONST_1),
                    n => {
                        // Large dispatch index: use ldc2_w with a real Long CP entry.
                        let cp_idx = cp.add_long(n as i64);
                        code.push(LDC2_W);
                        code.extend_from_slice(&cp_idx.to_be_bytes());
                    }
                }
                code.push(LASTORE); // closure_arr[0] = dispatch_idx

                // ── Store each capture at closure[1..n] ──────────────────────
                for (cap_i, cap_name) in captures.iter().enumerate() {
                    code.push(DUP);
                    emit_iconst(&mut code, (cap_i + 1) as i32); // slot index
                    let (cap_slot, cap_type) = lookup_var(cap_name)?;
                    match cap_type {
                        JvmType::Int => {
                            // i32 → widen to long via i2l.
                            emit_iload(&mut code, cap_slot);
                            code.push(I2L);
                        }
                        JvmType::Long => {
                            emit_lload(&mut code, cap_slot);
                        }
                        other => {
                            // f32/f64/Ref captures are not supported in v1.
                            // Float captures should have been caught by the validator
                            // (Check 2.5); we guard here as a belt-and-braces measure.
                            return Err(IIRJvmError::InvalidOperand {
                                function: fname.clone(),
                                detail: format!(
                                    "alloc_closure {:?}: capture {:?} has unsupported \
                                     JVM type {:?}; only i32/i64 captures are supported in v1",
                                    fn_name, cap_name, other
                                ),
                            });
                        }
                    }
                    code.push(LASTORE); // closure_arr[cap_i + 1] = cap_value
                }

                // ── Store the completed closure array ────────────────────────
                emit_astore(&mut code, dest_slot);
            }

            // ── call_closure (LANG36) ────────────────────────────────────────
            //
            // Build a `long[]` args array and call `__callClosure(long[], long[])`.
            //
            // srcs[0] = Var(handle)   — the closure (long[]) reference
            // srcs[1..] = Var(arg_i)  — call-time arguments
            //
            // JVM sequence:
            //
            //   aload handle_slot        ← push closure handle (long[])
            //   iconst_{n_args}          ← args array size
            //   newarray T_LONG          ← long[] args_arr = new long[n_args]
            //   dup
            //   iconst_0
            //   <load arg0, widen if i32>
            //   lastore                  ← args_arr[0] = arg0
            //   …
            //   invokestatic ClassName.__callClosure([J[J)J
            //   lstore dest_slot         ← dest = result (always a long)
            "call_closure" => {
                let dest_name = instr
                    .dest
                    .as_deref()
                    .ok_or_else(|| IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "call_closure has no dest".to_string(),
                    })?;
                let handle_name = match instr.srcs.first() {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => {
                        return Err(IIRJvmError::InvalidOperand {
                            function: fname.clone(),
                            detail: "call_closure srcs[0] must be Var(handle)".to_string(),
                        })
                    }
                };

                let (handle_slot, _) = lookup_var(&handle_name)?;
                let (dest_slot, _) = lookup_var(dest_name)?;

                // Collect call-time argument variables (srcs[1..]).
                let args: Vec<&str> = instr
                    .srcs
                    .iter()
                    .skip(1)
                    .map(|s| match s {
                        Operand::Var(n) => n.as_str(),
                        _ => "",
                    })
                    .collect();
                let n_args = args.len();

                // ── Push closure handle ──────────────────────────────────────
                emit_aload(&mut code, handle_slot);

                // ── Build args array ─────────────────────────────────────────
                emit_iconst(&mut code, n_args as i32);
                code.push(NEWARRAY);
                code.push(T_LONG);

                for (arg_i, arg_name) in args.iter().enumerate() {
                    code.push(DUP);
                    emit_iconst(&mut code, arg_i as i32);
                    let (arg_slot, arg_type) = lookup_var(arg_name)?;
                    match arg_type {
                        JvmType::Int => {
                            emit_iload(&mut code, arg_slot);
                            code.push(I2L);
                        }
                        JvmType::Long => {
                            emit_lload(&mut code, arg_slot);
                        }
                        other => {
                            return Err(IIRJvmError::InvalidOperand {
                                function: fname.clone(),
                                detail: format!(
                                    "call_closure: arg {:?} has unsupported type {:?}; \
                                     only i32/i64 args are supported in v1",
                                    arg_name, other
                                ),
                            });
                        }
                    }
                    code.push(LASTORE); // args_arr[arg_i] = arg_value
                }

                // ── invokestatic ClassName.__callClosure([J[J)J ──────────────
                //
                // The dispatch method takes (long[], long[]) and returns long.
                // Descriptor: "([J[J)J".
                let dispatch_ref =
                    cp.add_methodref(class_name, "__callClosure", "([J[J)J");
                code.push(INVOKESTATIC);
                code.extend_from_slice(&dispatch_ref.to_be_bytes());

                // ── Store the long result ────────────────────────────────────
                //
                // __callClosure always returns long.  The dest slot was
                // allocated as Long (JvmType::Long) by build_type_map.
                emit_lstore(&mut code, dest_slot);
            }

            // ── alloc ref<LispyPair> — Object[] cons cell allocation ─────────
            //
            // A `cons` cell in Lispy is a 2-element `Object[]` array:
            //   index 0 = head (car)
            //   index 1 = tail (cdr)
            //
            // When we see `alloc ref<LispyPair>` followed immediately by two
            // `field_store` instructions that write fields 0 and 1, we
            // pattern-match the whole triple and emit the full cons-cell
            // construction sequence in one shot, then advance the index past
            // all three instructions.
            //
            // If the two `field_store` instructions are not present (e.g. the
            // pair is constructed incrementally or only one field is written),
            // we fall back to emitting *just* the array allocation and storing
            // the uninitialised reference in the dest slot.
            //
            // Pattern: alloc + field_store[0] + field_store[1]
            //
            // JVM sequence emitted:
            //
            //   iconst_2                  ← array length = 2
            //   anewarray java/lang/Object ← allocate Object[]
            //   dup                        ← keep ref for second aastore
            //   iconst_0                   ← field index 0 (head)
            //   aload   <head_slot>        ← value to store
            //   aastore                    ← array[0] = head
            //   dup                        ← keep ref for astore at end
            //   iconst_1                   ← field index 1 (tail)
            //   aload   <tail_slot>        ← value to store
            //   aastore                    ← array[1] = tail
            //   astore  <dest_slot>        ← store the pair into local
            "alloc" if instr.type_hint == "ref<LispyPair>" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "alloc ref<LispyPair> has no dest".to_string(),
                })?;
                let (dest_slot, _) = lookup_var(dest_name)?;

                // Look ahead: do the next two instructions form a
                // field_store[0] + field_store[1] pair?
                //
                // Expected layout of field_store:
                //   op        = "field_store"
                //   srcs[0]   = Var(dest_name)      ← the array ref
                //   srcs[1]   = Int(field_index)     ← 0 or 1
                //   srcs[2]   = Var(value_name)      ← value to store
                //   type_hint = "ref<LispyPair>"
                let next1 = instrs.get(i + 1);
                let next2 = instrs.get(i + 2);

                let cons_pattern = match (next1, next2) {
                    (Some(fs1), Some(fs2))
                        if fs1.op == "field_store"
                            && fs2.op == "field_store"
                            && fs1.srcs.get(0) == Some(&Operand::Var(dest_name.to_string()))
                            && fs2.srcs.get(0) == Some(&Operand::Var(dest_name.to_string()))
                            && fs1.srcs.get(1) == Some(&Operand::Int(0))
                            && fs2.srcs.get(1) == Some(&Operand::Int(1)) =>
                    {
                        // Extract head and tail variable names.
                        let head = match fs1.srcs.get(2) {
                            Some(Operand::Var(n)) => n.clone(),
                            _ => return Err(IIRJvmError::InvalidOperand {
                                function: fname.clone(),
                                detail: "field_store[0] value is not a Var".to_string(),
                            }),
                        };
                        let tail = match fs2.srcs.get(2) {
                            Some(Operand::Var(n)) => n.clone(),
                            _ => return Err(IIRJvmError::InvalidOperand {
                                function: fname.clone(),
                                detail: "field_store[1] value is not a Var".to_string(),
                            }),
                        };
                        Some((head, tail))
                    }
                    _ => None,
                };

                if let Some((head_name, tail_name)) = cons_pattern {
                    // Full cons cell construction: alloc + field_store[0] + field_store[1]
                    let (head_slot, _) = lookup_var(&head_name)?;
                    let (tail_slot, _) = lookup_var(&tail_name)?;

                    // iconst_2 — array length
                    code.push(ICONST_2);
                    // anewarray java/lang/Object — element type CP index
                    code.push(ANEWARRAY);
                    code.extend_from_slice(&object_class_idx.to_be_bytes());
                    // dup — keep ref on stack for first aastore
                    code.push(DUP);
                    // iconst_0 — field 0 (head)
                    code.push(ICONST_0);
                    // aload head
                    emit_aload(&mut code, head_slot);
                    // aastore — array[0] = head
                    code.push(AASTORE);
                    // dup — keep ref on stack for second aastore
                    code.push(DUP);
                    // iconst_1 — field 1 (tail)
                    code.push(ICONST_1);
                    // aload tail
                    emit_aload(&mut code, tail_slot);
                    // aastore — array[1] = tail
                    code.push(AASTORE);
                    // astore dest — save the completed pair
                    emit_astore(&mut code, dest_slot);

                    // Skip the two field_store instructions we consumed.
                    i += 3;
                    continue;
                } else {
                    // No immediate field_stores: just allocate an uninitialised
                    // Object[2] and store the reference.  The caller is expected
                    // to use separate `field_store` instructions to fill the fields.
                    code.push(ICONST_2);
                    code.push(ANEWARRAY);
                    code.extend_from_slice(&object_class_idx.to_be_bytes());
                    emit_astore(&mut code, dest_slot);
                    i += 1;
                    continue;
                }
            }

            // ── alloc (other type) ────────────────────────────────────────────
            //
            // Any `alloc` with a type other than `ref<LispyPair>` falls through
            // to the unsupported-op error.  The validator should have caught
            // `ref<other>` types already (they fail Check 4), but we guard here
            // as a belt-and-braces measure.
            "alloc" => {
                return Err(IIRJvmError::UnsupportedOp {
                    function: fname.clone(),
                    op: format!("alloc (type_hint = {:?})", instr.type_hint),
                });
            }

            // ── field_store (bare — not consumed by alloc lookahead) ──────────
            //
            // A `field_store` that was NOT immediately preceded by a matching
            // `alloc ref<LispyPair>` — for example, updating a single field of
            // a pair that was previously allocated.
            //
            // IIR layout:
            //   srcs[0] = Var(array_ref)   ← the Object[] to write into
            //   srcs[1] = Int(field_index) ← 0 = car, 1 = cdr
            //   srcs[2] = Var(value)       ← value to store
            //
            // JVM sequence:
            //   aload  array_ref
            //   iconst_N                   ← field index
            //   aload  value
            //   aastore
            "field_store" => {
                let arr_name = match instr.srcs.get(0) {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "field_store srcs[0] must be a Var (array ref)".to_string(),
                    }),
                };
                let field_idx = match instr.srcs.get(1) {
                    Some(Operand::Int(n)) => *n as i32,
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "field_store srcs[1] must be an Int (field index)".to_string(),
                    }),
                };
                let val_name = match instr.srcs.get(2) {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "field_store srcs[2] must be a Var (value)".to_string(),
                    }),
                };

                let (arr_slot, _) = lookup_var(&arr_name)?;
                let (val_slot, _) = lookup_var(&val_name)?;

                emit_aload(&mut code, arr_slot);
                emit_iconst(&mut code, field_idx);
                emit_aload(&mut code, val_slot);
                code.push(AASTORE);
            }

            // ── field_load (car/cdr) ─────────────────────────────────────────
            //
            // Load one element from a cons-cell array into a destination slot.
            //
            // IIR layout:
            //   srcs[0] = Var(array_ref)   ← the Object[] to read from
            //   srcs[1] = Int(field_index) ← 0 = car, 1 = cdr
            //   dest    = result variable
            //
            // JVM sequence:
            //   aload  array_ref
            //   iconst_N                   ← field index (0 or 1)
            //   aaload                     ← push array[index]
            //   astore dest
            "field_load" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "field_load has no dest".to_string(),
                })?;
                let arr_name = match instr.srcs.get(0) {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "field_load srcs[0] must be a Var (array ref)".to_string(),
                    }),
                };
                let field_idx = match instr.srcs.get(1) {
                    Some(Operand::Int(n)) => *n as i32,
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "field_load srcs[1] must be an Int (field index)".to_string(),
                    }),
                };

                let (dest_slot, _) = lookup_var(dest_name)?;
                let (arr_slot, _) = lookup_var(&arr_name)?;

                emit_aload(&mut code, arr_slot);
                emit_iconst(&mut code, field_idx);
                code.push(AALOAD);
                emit_astore(&mut code, dest_slot);
            }

            // ── is_null ───────────────────────────────────────────────────────
            //
            // Test whether a reference is `null` (i.e. the Lispy `nil`).
            // Produces an int result: 1 = null/nil, 0 = non-null.
            //
            // IIR layout:
            //   srcs[0] = Var(ref_var)   ← the reference to test
            //   dest    = result (bool / i32)
            //
            // JVM pattern (8 bytes of code):
            //
            //   [PC+0]  iconst_1          ← "assume true" default
            //   [PC+1]  aload  ref_var    ← push the reference (1–3 bytes)
            //   [PC+?]  ifnull  +3        ← if null: skip iconst_0, fall through with 1
            //   [PC+?]  iconst_0          ← overwrite: not null → result = 0
            //   [PC+?]  istore dest
            //
            // Because `aload` is variable-width (1 or 3 bytes for wide), we
            // compute the `ifnull` offset dynamically instead of using a fixed
            // offset.
            //
            // A cleaner alternative uses `swap`:
            //   iconst_1
            //   aload ref_var
            //   ifnull +3       (3 bytes: opcode + 2-byte offset)
            //   iconst_0        (1 byte)
            //   istore dest
            //
            // With `aload_N` short forms (slot ≤3, 1 byte), the ifnull offset
            // is +3 (skip `iconst_0`).  But we use the 2-byte `aload N` form
            // throughout for simplicity, so:
            //
            //   iconst_1                  1 byte
            //   aload  N  (2 bytes)       skip to [PC+3] relative to ifnull
            //   ifnull +3  (3 bytes)      targets PC+6 (iconst_0 if NOT null)
            //     — wait, we want: if null → keep iconst_1; if not null → replace
            //
            // Let me re-derive.  Goal: push 1 if ref==null, else push 0.
            //
            //   Stack snapshot before we start:     [...]
            //   After iconst_1:                     [..., 1]
            //   After aload ref:                    [..., 1, ref]
            //   ifnull +3  → if ref==null branch to PC_after_iconst0
            //     (else fall through to iconst_0)
            //   [fall-through] iconst_0:            [..., 0]  (replaces 1 — no! stack now has [...,1,0])
            //
            // This doesn't work directly because iconst_0 would ADD to the stack.
            // We need `swap` or a different shape.
            //
            // Correct pattern with swap:
            //
            //   aload  ref_var        ← push ref
            //   iconst_1              ← push 1 (placeholder "true")
            //   swap                  ← swap: now [ref, 1] → [..., ref, 1] becomes [..., 1, ref]
            //   ifnull +3             ← consume ref; if null, jump to [PC+3]
            //   iconst_0              ← NOT null: replace 1 on stack with 0
            //   istore dest           ← store result
            //   ← jump target lands here if null (1 already on stack)
            //   istore dest
            //
            // But then we have two istore instructions!  Cleaner:
            //
            //   aload  ref_var        ← push ref                   1+1 = 2 bytes
            //   ifnull +5             ← if null, goto [not-null-end+5]: 3 bytes
            //   iconst_0              ← not null → result = 0       1 byte
            //   goto +3               ← jump past iconst_1:         3 bytes
            //   iconst_1              ← null → result = 1           1 byte
            //   istore dest           ← store result
            //
            // Offset from `ifnull` opcode:
            //   ifnull is at position P.
            //   iconst_0 at P+3 (immediately after ifnull's 2-byte operand).
            //   goto at P+4.
            //   iconst_1 at P+7.
            //   istore at P+8.
            //   `ifnull` offset = target - P = (P+7) - P = 7.  ✓ (lands on iconst_1)
            //   `goto`  offset  = (P+8) - (P+4) = 4.           ✓ (lands on istore)
            //
            // This is 8 bytes of code (before istore), analogous to emit_int_compare.
            // ── box — wrap an i32 atom as a `java.lang.Integer` (McCarthy W3b) ──
            //
            // The managed value model is uniform-reference: an atom stored in a
            // cons cell (`Object[]`) or passed where a lisp value is expected is
            // boxed. The wasm backend lowers `box` to `ref.i31`; the JVM lowers it
            // to `Integer.valueOf(I)` — the same shared IIR op, a per-backend
            // boxing. Bytecode:  iload src ; invokestatic Integer.valueOf ; astore dest.
            "box" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "box has no dest".to_string(),
                })?;
                let src_name = match instr.srcs.first() {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "box srcs[0] must be a Var".to_string(),
                    }),
                };
                let (dest_slot, _) = lookup_var(dest_name)?;
                let (src_slot, _) = lookup_var(&src_name)?;
                emit_iload(&mut code, src_slot);
                let mref = cp.add_methodref(
                    "java/lang/Integer",
                    "valueOf",
                    "(I)Ljava/lang/Integer;",
                );
                code.push(INVOKESTATIC);
                code.extend_from_slice(&mref.to_be_bytes());
                emit_astore(&mut code, dest_slot);
            }

            // ── unbox — unwrap a `java.lang.Integer` reference to its i32 value ──
            //
            // The dual of `box`: `checkcast Integer ; Integer.intValue()`. Used at
            // the entry/return boundary (the wasm backend lowers `unbox` to
            // `i31.get_s`). Bytecode:  aload src ; checkcast Integer ; invokevirtual
            // Integer.intValue ; istore dest.
            "unbox" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "unbox has no dest".to_string(),
                })?;
                let src_name = match instr.srcs.first() {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "unbox srcs[0] must be a Var".to_string(),
                    }),
                };
                let (dest_slot, _) = lookup_var(dest_name)?;
                let (src_slot, _) = lookup_var(&src_name)?;
                emit_aload(&mut code, src_slot);
                let cidx = cp.add_class("java/lang/Integer");
                code.push(CHECKCAST);
                code.extend_from_slice(&cidx.to_be_bytes());
                let mref = cp.add_methodref("java/lang/Integer", "intValue", "()I");
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&mref.to_be_bytes());
                emit_istore(&mut code, dest_slot);
            }

            "is_null" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "is_null has no dest".to_string(),
                })?;
                let ref_name = match instr.srcs.get(0) {
                    Some(Operand::Var(n)) => n.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "is_null srcs[0] must be a Var".to_string(),
                    }),
                };

                let (dest_slot, _) = lookup_var(dest_name)?;
                let (ref_slot, _) = lookup_var(&ref_name)?;

                // aload ref_var — push the reference to test (2 bytes)
                emit_aload(&mut code, ref_slot);
                // ifnull +7 — if null, jump to iconst_1 (3 bytes; offset = 7)
                code.push(IFNULL);
                code.extend_from_slice(&7i16.to_be_bytes());
                // not-null arm: iconst_0 (1 byte)
                code.push(ICONST_0);
                // goto +4 — jump past iconst_1 to istore (3 bytes; offset = 4)
                code.push(GOTO);
                code.extend_from_slice(&4i16.to_be_bytes());
                // null arm: iconst_1 (1 byte) — ifnull branch lands here
                code.push(ICONST_1);
                // istore dest — store the result
                emit_istore(&mut code, dest_slot);
            }

            // ── global_load → getstatic <this>.G_N:J ; lstore (LANG-FULL E6) ─
            //
            // A module global is a `public static long G_N` field of this class
            // (collected in `globals`). `getstatic` pushes its value, `lstore`
            // writes it into the dest's long slot.
            "global_load" => {
                let dest_name = instr.dest.as_deref().ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: "global_load has no dest".to_string(),
                })?;
                let gname = match instr.srcs.first() {
                    Some(Operand::Str(s)) => s,
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "global_load expects a string global name at srcs[0]".to_string(),
                    }),
                };
                let field_name = globals.get(gname).ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: format!("global_load: global {gname:?} was not collected (internal error)"),
                })?;
                let (dest_slot, dest_type) = lookup_var(dest_name)?;
                let fref = cp.add_fieldref(class_name, field_name, "J");
                code.push(GETSTATIC);
                code.extend_from_slice(&fref.to_be_bytes());
                // `getstatic J` pushes a long.  The field is always 64-bit, but the
                // dest local may be a narrower `int` (an `integer` program
                // concretised to i32) — narrow the long with `l2i` before `istore`,
                // the mirror of the `i2l` widen on `global_store`.
                if dest_type != JvmType::Long {
                    code.push(L2I);
                }
                emit_typed_store(&mut code, dest_slot, dest_type);
            }

            // ── global_store → lload ; putstatic <this>.G_N:J (LANG-FULL E6) ──
            "global_store" => {
                let gname = match instr.srcs.first() {
                    Some(Operand::Str(s)) => s,
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "global_store expects a string global name at srcs[0]".to_string(),
                    }),
                };
                let val_src = match instr.srcs.get(1) {
                    Some(Operand::Var(v)) => v.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "global_store expects a Var value at srcs[1]".to_string(),
                    }),
                };
                let field_name = globals.get(gname).ok_or_else(|| IIRJvmError::InvalidOperand {
                    function: fname.clone(),
                    detail: format!("global_store: global {gname:?} was not collected (internal error)"),
                })?;
                let (val_slot, val_type) = lookup_var(&val_src)?;
                emit_typed_load(&mut code, val_slot, val_type);
                // The field is `J` (long); widen an i32 value to long first.
                if val_type != JvmType::Long {
                    code.push(I2L);
                }
                let fref = cp.add_fieldref(class_name, field_name, "J");
                code.push(PUTSTATIC);
                code.extend_from_slice(&fref.to_be_bytes());
            }

            // ── io_out → System.out.println(long) ───────────────────────────
            //
            // `io_out %val` prints an i64 value to stdout.  JVM steps:
            //   1. getstatic java/lang/System.out : Ljava/io/PrintStream;
            //   2. lload <slot_of_%val>           (push the long onto the operand stack)
            //   3. invokevirtual java/io/PrintStream.println(J)V
            //
            // Constant pool entries needed:
            //   - Class("java/io/PrintStream")
            //   - Fieldref("java/lang/System", "out", "Ljava/io/PrintStream;")
            //   - Methodref("java/io/PrintStream", "println", "(J)V")
            //
            // We add these to the constant pool builder on demand.
            "io_out" => {
                let val_src = match instr.srcs.first() {
                    Some(Operand::Var(v)) => v.clone(),
                    _ => return Err(IIRJvmError::InvalidOperand {
                        function: fname.clone(),
                        detail: "io_out requires a Var operand".to_string(),
                    }),
                };
                let (val_slot, val_type) = lookup_var(&val_src)?;

                // Step 1: getstatic java/lang/System.out : Ljava/io/PrintStream;
                let system_out_ref = cp.add_fieldref(
                    "java/lang/System",
                    "out",
                    "Ljava/io/PrintStream;",
                );
                code.push(GETSTATIC);
                code.extend_from_slice(&system_out_ref.to_be_bytes());

                // Step 2: load the value variable onto the operand stack.
                // We use emit_typed_load so that i32 values use iload and
                // i64 values use lload — matching the method descriptor below.
                emit_typed_load(&mut code, val_slot, val_type);

                // Step 3: invokevirtual java/io/PrintStream.println(J)V
                // The descriptor "(J)V" means "takes one long, returns void".
                // If the IIR variable is i32 we still call the (J)V overload
                // (the JVM will silently use the wider type); a production
                // backend would pick the matching overload from the type map.
                let println_ref = cp.add_methodref(
                    "java/io/PrintStream",
                    "println",
                    "(J)V",
                );
                code.push(INVOKEVIRTUAL);
                code.extend_from_slice(&println_ref.to_be_bytes());
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
        i += 1;
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
// generate_call_closure_dispatch — synthesize the __callClosure method
// ---------------------------------------------------------------------------

/// Generate the `__callClosure(long[], long[]) → long` dispatch method.
///
/// The method is a static function that:
/// 1. Reads `closure[0]` (the dispatch index).
/// 2. Compares it with each expected index using `lcmp` + `ifne`.
/// 3. On a match, loads the captures from `closure[1..]` and the call-time
///    args from `args[0..]`, narrows them to the target parameter types if
///    needed, calls the static function, widens the result to `long` if
///    needed, and returns.
/// 4. After all cases: returns `0L` (unreachable default).
///
/// # Method signature
///
/// `static long __callClosure(long[] closure, long[] args)`
/// Descriptor: `([J[J)J`
///
/// # Parameters
/// - slot 0 = `closure` (`long[]` — one reference slot)
/// - slot 1 = `args`    (`long[]` — one reference slot)
fn generate_call_closure_dispatch(
    class_name: &str,
    dispatch_table: &[&ClosureDispatchEntry], // sorted by dispatch_idx
    cp: &mut ConstantPoolBuilder,
) -> JvmMethodInfo {
    let mut code: Vec<u8> = Vec::new();

    // The CLOSURE parameter is in slot 0, ARGS parameter is in slot 1.
    // Both are reference types (long[] → one slot each), so max_locals = 2.
    let max_locals: u16 = 2;

    for entry in dispatch_table {
        // ── Emit dispatch check ──────────────────────────────────────────────
        //
        // Pattern:
        //   aload 0           ← push closure array
        //   iconst_0          ← index 0 (dispatch slot)
        //   laload            ← push closure[0] as long
        //   <push expected dispatch_idx as long>
        //   lcmp              ← → int: 0 if equal, ±1 otherwise
        //   ifne +<body_size> ← if NOT equal, skip the body
        //
        // We emit the `ifne` with a placeholder offset, then emit the body,
        // then fix the offset to point just past `lreturn`.

        // Load closure[0]
        code.push(ALOAD);
        code.push(0u8); // aload 0 = closure array
        code.push(ICONST_0); // index 0
        code.push(LALOAD); // push closure[0] as long

        // Push expected dispatch index as long.
        match entry.dispatch_idx {
            0 => code.push(LCONST_0),
            1 => code.push(LCONST_1),
            n => {
                let cp_idx = cp.add_long(n as i64);
                code.push(LDC2_W);
                code.extend_from_slice(&cp_idx.to_be_bytes());
            }
        }

        // lcmp: compare the two longs.
        code.push(LCMP);

        // ifne <skip_body>: if LCMP result != 0 (not equal), skip this body.
        // We patch the offset after emitting the body.
        let ifne_pos = code.len();
        code.push(IFNE);
        code.extend_from_slice(&0i16.to_be_bytes()); // placeholder

        // ── Emit call body ───────────────────────────────────────────────────
        //
        // Push captures: closure[1..n_captures], narrowing long → int if needed.
        for cap_i in 0..entry.n_captures {
            let (_cap_name, cap_type_str) = entry.fn_params.get(cap_i)
                .map(|(n, t)| (n.as_str(), t.as_str()))
                .unwrap_or(("", "i64"));
            let cap_jtype = iir_type_to_jvm(cap_type_str).unwrap_or(JvmType::Long);

            code.push(ALOAD);
            code.push(0u8); // closure array
            emit_iconst(&mut code, (cap_i + 1) as i32); // closure[cap_i + 1]
            code.push(LALOAD); // push as long

            if cap_jtype == JvmType::Int {
                // Parameter expects int — narrow the long.
                code.push(L2I);
            }
            // Otherwise it's already a long — leave on stack.
        }

        // Push call-time args: args[0..n_args], narrowing if needed.
        let n_call_args = entry.fn_params.len().saturating_sub(entry.n_captures);
        for arg_i in 0..n_call_args {
            let param_idx = entry.n_captures + arg_i;
            let (_arg_name, arg_type_str) = entry.fn_params.get(param_idx)
                .map(|(n, t)| (n.as_str(), t.as_str()))
                .unwrap_or(("", "i64"));
            let arg_jtype = iir_type_to_jvm(arg_type_str).unwrap_or(JvmType::Long);

            code.push(ALOAD);
            code.push(1u8); // args array
            emit_iconst(&mut code, arg_i as i32); // args[arg_i]
            code.push(LALOAD); // push as long

            if arg_jtype == JvmType::Int {
                code.push(L2I);
            }
        }

        // Invoke the target function.
        let fn_desc = make_descriptor(&entry.fn_params, &entry.fn_return_type);
        let fn_ref = cp.add_methodref(class_name, &entry.fn_name, &fn_desc);
        code.push(INVOKESTATIC);
        code.extend_from_slice(&fn_ref.to_be_bytes());

        // Widen the result to long if the function returns int.
        let ret_jtype =
            iir_type_to_jvm(&entry.fn_return_type).unwrap_or(JvmType::Long);
        if ret_jtype == JvmType::Int {
            code.push(I2L);
        }

        code.push(LRETURN);

        // ── Patch the ifne offset ────────────────────────────────────────────
        //
        // The offset in `ifne` is measured from the opcode's own position.
        // We want it to jump to the instruction AFTER `lreturn`.
        //
        // JVM branch offsets are signed 16-bit, so a dispatch body must be
        // < 32,767 bytes.  For any reasonable number of closure targets this
        // is never an issue; we guard explicitly so a malformed or adversarially
        // crafted module produces a clear error rather than silent truncation.
        let body_end = code.len();
        let raw_offset = (body_end as i64) - (ifne_pos as i64);
        if raw_offset < i16::MIN as i64 || raw_offset > i16::MAX as i64 {
            // This is an internal limit of the JVM's 16-bit branch offsets.
            // In practice it would require tens of thousands of closure entries
            // to trigger, but we guard it explicitly for correctness.
            panic!(
                "__callClosure dispatch body too large for JVM i16 branch offset \
                 ({} bytes, max {}); split the closure dispatch table or reduce \
                 the number of closure-eligible functions",
                raw_offset, i16::MAX
            );
        }
        let offset = raw_offset as i16;
        let ob = offset.to_be_bytes();
        code[ifne_pos + 1] = ob[0];
        code[ifne_pos + 2] = ob[1];
    }

    // ── Default (unreachable): return 0L ─────────────────────────────────────
    code.push(LCONST_0);
    code.push(LRETURN);

    // Descriptor: ([J[J)J — takes two long[], returns long.
    let descriptor = "([J[J)J".to_string();
    cp.add_utf8(&descriptor);
    cp.add_utf8("__callClosure");

    // Generous max_stack — the worst case is:
    //   closure[0] comparison: 2 longs = 4 stack words
    //   body: long[] ref + int index + long value per arg = 3 per arg
    // We set 16 to be safe for any reasonable function arity.
    let max_stack: u16 = 16;

    let code_attribute = JvmCodeAttribute {
        name: "Code".to_string(),
        max_stack,
        max_locals,
        code,
        nested_attributes: vec![],
    };

    JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: "__callClosure".to_string(),
        descriptor,
        attributes: vec![JvmMethodAttribute::Code(code_attribute)],
    }
}

// ---------------------------------------------------------------------------
// serialize_jvm_class_file — convert JvmClassFile to bytes
// ---------------------------------------------------------------------------
//
// The JVM class file format (JVMS §4.1):
//
//   magic                 (u32 = 0xCAFEBABE)
//   minor_version, major_version (u16 each)
//   constant_pool_count   (u16 = len of constant_pool vec, including index 0)
//   constant_pool[1..count-1] (variable-length entries)
//   access_flags          (u16)
//   this_class            (u16 CP index of Class entry)
//   super_class           (u16 CP index of Class entry)
//   interfaces_count      (u16 = 0)
//   fields_count          (u16 = 0)
//   methods_count         (u16)
//   methods[0..count-1]   (method_info records)
//   attributes_count      (u16 = 0)

/// Serialise a `JvmClassFile` to a raw `.class` file byte vector.
///
/// Used by the real-JVM integration test (see `test_backend.rs`).  The
/// output should be accepted by any Java 8+ JVM.
pub fn serialize_jvm_class_file(class: &JvmClassFile) -> Vec<u8> {
    let mut out = Vec::new();

    // Magic, version.
    out.extend_from_slice(&0xCAFE_BABEu32.to_be_bytes());
    out.extend_from_slice(&class.version.minor.to_be_bytes());
    out.extend_from_slice(&class.version.major.to_be_bytes());

    // Constant pool count (the vec includes the phantom at index 0, so
    // cp.len() == the JVM constant_pool_count field).
    let cp_count = class.constant_pool.len() as u16;
    out.extend_from_slice(&cp_count.to_be_bytes());

    // Emit CP entries; skip the phantom at index 0 and any None placeholders
    // (which appear after Long/Double entries — those don't emit any bytes).
    for entry in class.constant_pool.iter().skip(1) {
        match entry {
            None => {} // phantom slot — no bytes
            Some(e) => serialize_cp_entry(&mut out, e),
        }
    }

    // Access flags, this_class, super_class.
    out.extend_from_slice(&class.access_flags.to_be_bytes());
    let this_idx = find_class_cp_index(&class.constant_pool, &class.this_class_name)
        .unwrap_or(0);
    let super_idx = find_class_cp_index(&class.constant_pool, &class.super_class_name)
        .unwrap_or(0);
    out.extend_from_slice(&this_idx.to_be_bytes());
    out.extend_from_slice(&super_idx.to_be_bytes());

    // Interfaces: none.
    out.extend_from_slice(&0u16.to_be_bytes()); // interfaces_count

    // Fields (LANG-FULL E6 — module static globals). Each `field_info` is
    // access_flags, name_index, descriptor_index, attributes_count=0. The name
    // and descriptor Utf8 entries are already in the CP (added by the
    // `add_fieldref` calls during lowering).
    out.extend_from_slice(&(class.fields.len() as u16).to_be_bytes());
    for field in &class.fields {
        out.extend_from_slice(&field.access_flags.to_be_bytes());
        out.extend_from_slice(&find_utf8_cp_index(&class.constant_pool, &field.name).to_be_bytes());
        out.extend_from_slice(&find_utf8_cp_index(&class.constant_pool, &field.descriptor).to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // attributes_count
    }

    // Methods.
    out.extend_from_slice(&(class.methods.len() as u16).to_be_bytes());
    for method in &class.methods {
        serialize_method(&mut out, method, &class.constant_pool);
    }

    // Class-level attributes: none.
    out.extend_from_slice(&0u16.to_be_bytes());

    out
}

/// Find the CP index of the Class entry whose name matches `class_name`.
///
/// We first find the Utf8 entry for the name, then find the Class entry that
/// references it.  Returns `None` if not found (should never happen for
/// class files produced by this lowering pass).
fn find_class_cp_index(
    cp: &[Option<JvmConstantPoolEntry>],
    class_name: &str,
) -> Option<u16> {
    // Step 1: find Utf8 entry index.
    let utf8_idx = cp.iter().enumerate().find_map(|(i, e)| {
        if let Some(JvmConstantPoolEntry::Utf8(s)) = e {
            if s == class_name {
                Some(i as u16)
            } else {
                None
            }
        } else {
            None
        }
    })?;

    // Step 2: find Class entry that references it.
    cp.iter().enumerate().find_map(|(i, e)| {
        if let Some(JvmConstantPoolEntry::Class { name_index }) = e {
            if *name_index == utf8_idx {
                Some(i as u16)
            } else {
                None
            }
        } else {
            None
        }
    })
}

/// Find the CP index of a Utf8 entry that matches `s`.
fn find_utf8_cp_index(cp: &[Option<JvmConstantPoolEntry>], s: &str) -> u16 {
    cp.iter().enumerate().find_map(|(i, e)| {
        if let Some(JvmConstantPoolEntry::Utf8(v)) = e {
            if v == s {
                Some(i as u16)
            } else {
                None
            }
        } else {
            None
        }
    })
    .unwrap_or(0)
}

/// Serialize one `JvmConstantPoolEntry` to bytes.
fn serialize_cp_entry(out: &mut Vec<u8>, entry: &JvmConstantPoolEntry) {
    match entry {
        JvmConstantPoolEntry::Utf8(s) => {
            out.push(1); // CONSTANT_Utf8
            let b = s.as_bytes();
            // JVMS §4.4.7: the length field is u16 — guard against truncation.
            // In practice JVM class-name and method-name strings are always
            // well under 65,535 bytes; the check catches adversarially long
            // strings passed through an untrusted IIRModule.
            assert!(
                b.len() <= u16::MAX as usize,
                "JVM Utf8 constant pool entry exceeds 65535 bytes (actual {} bytes): {:?}",
                b.len(), s
            );
            out.extend_from_slice(&(b.len() as u16).to_be_bytes());
            out.extend_from_slice(b);
        }
        JvmConstantPoolEntry::Integer(v) => {
            out.push(3); // CONSTANT_Integer
            out.extend_from_slice(&v.to_be_bytes());
        }
        JvmConstantPoolEntry::Long(v) => {
            out.push(5); // CONSTANT_Long
            out.extend_from_slice(&v.to_be_bytes());
            // Note: the phantom None at N+1 is handled by the caller skipping None.
        }
        JvmConstantPoolEntry::Double(v) => {
            out.push(6); // CONSTANT_Double
            out.extend_from_slice(&v.to_bits().to_be_bytes());
        }
        JvmConstantPoolEntry::Class { name_index } => {
            out.push(7); // CONSTANT_Class
            out.extend_from_slice(&name_index.to_be_bytes());
        }
        JvmConstantPoolEntry::String { string_index } => {
            out.push(8); // CONSTANT_String
            out.extend_from_slice(&string_index.to_be_bytes());
        }
        JvmConstantPoolEntry::Fieldref { class_index, name_and_type_index } => {
            out.push(9); // CONSTANT_Fieldref
            out.extend_from_slice(&class_index.to_be_bytes());
            out.extend_from_slice(&name_and_type_index.to_be_bytes());
        }
        JvmConstantPoolEntry::Methodref { class_index, name_and_type_index } => {
            out.push(10); // CONSTANT_Methodref
            out.extend_from_slice(&class_index.to_be_bytes());
            out.extend_from_slice(&name_and_type_index.to_be_bytes());
        }
        JvmConstantPoolEntry::NameAndType { name_index, descriptor_index } => {
            out.push(12); // CONSTANT_NameAndType
            out.extend_from_slice(&name_index.to_be_bytes());
            out.extend_from_slice(&descriptor_index.to_be_bytes());
        }
    }
}

/// Serialize one `JvmMethodInfo` to bytes.
fn serialize_method(
    out: &mut Vec<u8>,
    method: &JvmMethodInfo,
    cp: &[Option<JvmConstantPoolEntry>],
) {
    out.extend_from_slice(&method.access_flags.to_be_bytes());
    out.extend_from_slice(&find_utf8_cp_index(cp, &method.name).to_be_bytes());
    out.extend_from_slice(&find_utf8_cp_index(cp, &method.descriptor).to_be_bytes());
    out.extend_from_slice(&(method.attributes.len() as u16).to_be_bytes());
    for attr in &method.attributes {
        serialize_method_attribute(out, attr, cp);
    }
}

/// Serialize one `JvmMethodAttribute` to bytes.
fn serialize_method_attribute(
    out: &mut Vec<u8>,
    attr: &JvmMethodAttribute,
    cp: &[Option<JvmConstantPoolEntry>],
) {
    match attr {
        JvmMethodAttribute::Code(code_attr) => {
            let name_idx = find_utf8_cp_index(cp, "Code");
            out.extend_from_slice(&name_idx.to_be_bytes());

            // Build the Code attribute body.
            let mut body = Vec::new();
            body.extend_from_slice(&code_attr.max_stack.to_be_bytes());
            body.extend_from_slice(&code_attr.max_locals.to_be_bytes());
            body.extend_from_slice(&(code_attr.code.len() as u32).to_be_bytes());
            body.extend_from_slice(&code_attr.code);
            body.extend_from_slice(&0u16.to_be_bytes()); // exception_table_length
            body.extend_from_slice(&0u16.to_be_bytes()); // nested_attributes_count

            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(&body);
        }
        JvmMethodAttribute::Raw(raw) => {
            let name_idx = find_utf8_cp_index(cp, &raw.name);
            out.extend_from_slice(&name_idx.to_be_bytes());
            out.extend_from_slice(&(raw.info.len() as u32).to_be_bytes());
            out.extend_from_slice(&raw.info);
        }
    }
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
/// We emit Java 5 class files (major version 49, minor version 0).
///
/// **Why version 49?**  The Java 7+ verifier (used for class file versions
/// ≥ 51) requires a `StackMapTable` attribute in every method that contains
/// branches.  Generating a correct StackMapTable requires a dataflow pass
/// over the bytecode, which is non-trivial to implement in v1 of this lowerer.
///
/// Class file version 49 (Java 5) uses the older type-inferencing verifier,
/// which does not require `StackMapTable`.  All modern JVMs — including Java
/// 21 — support loading class files as far back as version 45.3, and use the
/// old verifier for versions ≤ 49.  The code we emit is semantically correct
/// for any JVM version; only the verification path differs.
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
///     exports: vec![],
///     imports: vec![],
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

    // ── Step 1b: closure pre-pass ─────────────────────────────────────────────
    //
    // Collect every function referenced as a closure target (`alloc_closure`
    // srcs[0] = Str(fn_name)) and build a dispatch table sorted alphabetically
    // for deterministic index assignment.  The resulting map is threaded
    // through the rest of lowering so that `alloc_closure` and `call_closure`
    // instructions can emit the right bytecode without re-scanning the module.
    let closure_dispatch = collect_closure_dispatch(module);

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

    // If the module uses closures, pre-register the `__callClosure` Methodref
    // so that `call_closure` instructions can reference it by CP index.
    // The descriptor is `([J[J)J` — two long[] args, returns long.
    if !closure_dispatch.is_empty() {
        cp.add_methodref(&config.class_name, "__callClosure", "([J[J)J");
    }

    // ── Step 2b: collect module globals (LANG-FULL E6 layer 1) ────────────────
    //
    // Every distinct name read/written by `global_load`/`global_store` becomes a
    // `public static long G_N` field of this class (first-seen order). The map
    // (global name → JVM field name) is threaded into lowering so a
    // `global_load`/`global_store` emits `getstatic`/`putstatic` of the right
    // `Fieldref`. Field name is index-based (`G_0`, `G_1`, …) so an arbitrary
    // source identifier can never form an invalid or colliding JVM field name.
    let (globals, global_fields) = collect_global_fields(module);

    // ── Step 3: lower each function ───────────────────────────────────────────
    let mut methods: Vec<JvmMethodInfo> = Vec::new();
    for func in &module.functions {
        let method = lower_function(func, &config.class_name, module, &mut cp, &closure_dispatch, &globals)?;
        methods.push(method);
    }

    // ── Step 3b: generate __callClosure dispatch method (if needed) ───────────
    //
    // We generate this AFTER lowering the user functions so that all CP entries
    // for those functions (names, descriptors, Methodrefs) are already present.
    // The dispatch method just calls into those already-registered entries.
    if !closure_dispatch.is_empty() {
        // Sort by dispatch index for deterministic bytecode.
        let mut sorted: Vec<&ClosureDispatchEntry> =
            closure_dispatch.values().collect();
        sorted.sort_by_key(|e| e.dispatch_idx);

        let dispatch_method =
            generate_call_closure_dispatch(&config.class_name, &sorted, &mut cp);
        methods.push(dispatch_method);
    }

    // ── Step 4: assemble JvmClassFile ─────────────────────────────────────────
    Ok(JvmClassFile {
        // Java 5 = major version 49.  See doc-comment above for why we stay at
        // version 49 rather than targeting Java 8 (52): version 49 avoids the
        // mandatory StackMapTable attribute required by the Java 7+ verifier.
        version: JvmClassVersion { major: 49, minor: 0 },
        // ACC_PUBLIC | ACC_SUPER — standard flags for a public class.
        // ACC_SUPER must be set for Java 1.1+ classes (changes how `invokespecial`
        // searches the superclass method table).
        access_flags: ACC_PUBLIC | ACC_SUPER,
        this_class_name: config.class_name.clone(),
        super_class_name: "java/lang/Object".to_string(),
        constant_pool: cp.build(),
        fields: global_fields,
        methods,
    })
}

/// Collect every distinct module-global name (read or written) into
/// `(name → "G_N", [JvmFieldInfo])`, numbered in first-seen order across all
/// functions (LANG-FULL E6 layer 1). Each global is a `public static long`
/// field; the field name is index-based so an arbitrary source identifier can
/// never form an invalid or colliding JVM field name.
fn collect_global_fields(module: &IIRModule) -> (HashMap<String, String>, Vec<JvmFieldInfo>) {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut fields: Vec<JvmFieldInfo> = Vec::new();
    for f in &module.functions {
        for i in &f.instructions {
            if i.op == "global_load" || i.op == "global_store" {
                if let Some(Operand::Str(name)) = i.srcs.first() {
                    if !map.contains_key(name) {
                        let field_name = format!("G_{}", fields.len());
                        map.insert(name.clone(), field_name.clone());
                        fields.push(JvmFieldInfo {
                            access_flags: ACC_PUBLIC | ACC_STATIC,
                            name: field_name,
                            descriptor: "J".to_string(), // long
                        });
                    }
                }
            }
        }
    }
    (map, fields)
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
            exports: vec![],
            imports: vec![],
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
    fn narrow_op_over_long_operands_stays_long() {
        // LANG-FULL O2 / JVM long model. A *printing* program (Oct `out`) keeps the
        // i64 model, so an Oct `200 + 100` (u8 hint) has `long` operands. The `add`
        // must compute on the long model (`ladd` + a long mask), so its dest is typed
        // `Long` — NOT the `Int` the bare u8 hint would give. An `iadd` over `long`
        // operands would be unverifiable. This is what makes `200u8 + 100u8 = 44`
        // (and `~0u8 = 255`) run on the JVM for a printing program.
        let f = IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(200)], "i64"),
                IIRInstr::new("const", Some("b".into()), vec![Operand::Int(100)], "i64"),
                IIRInstr::new(
                    "add",
                    Some("c".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())],
                    "u8",
                ),
                IIRInstr::new("ret_void", None, vec![], "void"),
            ],
        );
        let tm = build_type_map(&f);
        assert_eq!(
            tm.get("c"),
            Some(&JvmType::Long),
            "u8 add over long operands must stay Long; got {:?}",
            tm.get("c")
        );
    }

    #[test]
    fn narrow_op_over_int_operands_stays_int() {
        // The concretized (exit-code) path: operands are `i32`, so the narrow op uses
        // the int model + an int mask, unchanged by the long-model fix.
        let f = IIRFunction::new(
            "main",
            vec![],
            "i32",
            vec![
                IIRInstr::new("const", Some("a".into()), vec![Operand::Int(200)], "i32"),
                IIRInstr::new("const", Some("b".into()), vec![Operand::Int(100)], "i32"),
                IIRInstr::new(
                    "add",
                    Some("c".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())],
                    "u8",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
            ],
        );
        let tm = build_type_map(&f);
        assert_eq!(
            tm.get("c"),
            Some(&JvmType::Int),
            "u8 add over int operands stays Int; got {:?}",
            tm.get("c")
        );
    }

    #[test]
    fn lower_class_name_from_config() {
        let module = make_module(void_fn("main"));
        let class = lower_iir_to_jvm(&module, &IIRJvmConfig::new("MyClass")).unwrap();
        assert_eq!(class.this_class_name, "MyClass");
    }

    // ── LANG-FULL E5 — array opcode lowering ────────────────────────────────

    /// Extract the raw Code bytes of a single-method lowered class.
    fn code_bytes(module: &IIRModule) -> Vec<u8> {
        let class = lower_iir_to_jvm(module, &make_cfg()).expect("lowers");
        let method = &class.methods[0];
        for attr in &method.attributes {
            if let JvmMethodAttribute::Code(c) = attr {
                return c.code.clone();
            }
        }
        panic!("method has no Code attribute");
    }

    #[test]
    fn array_handle_maps_to_ref() {
        // An `array<T>` handle occupies one local slot and uses aload/astore.
        assert_eq!(iir_type_to_jvm("array<i32>"), Some(JvmType::Ref));
        assert_eq!(iir_type_to_jvm("array<i64>"), Some(JvmType::Ref));
        assert_eq!(iir_type_to_jvm("array<f64>"), Some(JvmType::Ref));
        // E4d-BA-arr: a supported reference element (`array<str>` → `String[]`) now
        // maps to a Ref handle; an unsupported ref element still returns None.
        assert_eq!(iir_type_to_jvm("array<str>"), Some(JvmType::Ref));
        assert_eq!(iir_type_to_jvm("array<ref<LispyPair>>"), None);
    }

    #[test]
    fn array_element_opcodes_by_type() {
        assert_eq!(array_element_opcodes(JvmType::Int), Some((T_INT, IALOAD, IASTORE)));
        assert_eq!(array_element_opcodes(JvmType::Long), Some((T_LONG, LALOAD, LASTORE)));
        assert_eq!(array_element_opcodes(JvmType::Double), Some((T_DOUBLE, DALOAD, DASTORE)));
        assert_eq!(array_element_opcodes(JvmType::Ref), None);
    }

    /// `int[]` alloc/set/get/len lower to `newarray T_INT` + `iastore`/`iaload`
    /// + `arraylength`. The JVM bounds-checks each `*aload`/`*astore` natively.
    #[test]
    fn int_array_emits_native_array_opcodes() {
        // a := new int[3]; a[0] := 7; r := a[0]; n := len(a); ret r
        let f = IIRFunction::new(
            "main",
            vec![],
            "i32",
            vec![
                IIRInstr::new("const", Some("c3".into()), vec![Operand::Int(3)], "i32"),
                IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("c3".into())], "array<i32>"),
                IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i32"),
                IIRInstr::new("const", Some("v7".into()), vec![Operand::Int(7)], "i32"),
                IIRInstr::new("array_set", None,
                    vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("v7".into())], "i32"),
                IIRInstr::new("array_get", Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("i0".into())], "i32"),
                IIRInstr::new("array_len", Some("n".into()), vec![Operand::Var("a".into())], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
            ],
        );
        let code = code_bytes(&make_module(f));
        assert!(code.contains(&NEWARRAY) && code.contains(&T_INT), "newarray int[] expected");
        assert!(code.contains(&IASTORE), "iastore (array_set) expected");
        assert!(code.contains(&IALOAD), "iaload (array_get) expected");
        assert!(code.contains(&ARRAYLENGTH), "arraylength (array_len) expected");
    }

    /// E4d-BA-arr: `String[]` (BASIC `DIM A$(n)`) uses `anewarray java/lang/String`
    /// + `aastore`/`aaload` — reference-element ops, since a str value is a native
    /// `java.lang.String` (not a primitive). The JVM bounds-checks each access.
    #[test]
    fn string_array_emits_reference_array_opcodes() {
        let f = IIRFunction::new(
            "main",
            vec![],
            "i32",
            vec![
                IIRInstr::new("const", Some("c2".into()), vec![Operand::Int(2)], "i32"),
                IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("c2".into())], "array<str>"),
                IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i32"),
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("HI".into())], "str"),
                IIRInstr::new("array_set", None,
                    vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("s".into())], "str"),
                IIRInstr::new("array_get", Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("i0".into())], "str"),
                IIRInstr::new("array_len", Some("n".into()), vec![Operand::Var("a".into())], "i32"),
                IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i32"),
            ],
        );
        let code = code_bytes(&make_module(f));
        assert!(code.contains(&ANEWARRAY), "anewarray String[] expected");
        assert!(code.contains(&AASTORE), "aastore (array_set) expected");
        assert!(code.contains(&AALOAD), "aaload (array_get) expected");
    }

    /// `double[]` uses `newarray T_DOUBLE` + `dastore`/`daload`.
    #[test]
    fn double_array_emits_double_opcodes() {
        let f = IIRFunction::new(
            "main",
            vec![],
            "f64",
            vec![
                IIRInstr::new("const", Some("c2".into()), vec![Operand::Int(2)], "i32"),
                IIRInstr::new("alloc_array", Some("a".into()), vec![Operand::Var("c2".into())], "array<f64>"),
                IIRInstr::new("const", Some("i0".into()), vec![Operand::Int(0)], "i32"),
                IIRInstr::new("const", Some("v".into()), vec![Operand::Float(2.5)], "f64"),
                IIRInstr::new("array_set", None,
                    vec![Operand::Var("a".into()), Operand::Var("i0".into()), Operand::Var("v".into())], "f64"),
                IIRInstr::new("array_get", Some("r".into()),
                    vec![Operand::Var("a".into()), Operand::Var("i0".into())], "f64"),
                IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "f64"),
            ],
        );
        let code = code_bytes(&make_module(f));
        assert!(code.contains(&T_DOUBLE), "newarray double[] expected");
        assert!(code.contains(&DASTORE), "dastore expected");
        assert!(code.contains(&DALOAD), "daload expected");
    }

    #[test]
    fn lower_super_class_is_object() {
        let module = make_module(void_fn("main"));
        let class = lower_iir_to_jvm(&module, &make_cfg()).unwrap();
        assert_eq!(class.super_class_name, "java/lang/Object");
    }

    #[test]
    fn lower_version_is_java5() {
        // We emit version 49 (Java 5) to avoid the mandatory StackMapTable
        // required by the Java 7+ verifier.  See the doc-comment on lower_iir_to_jvm.
        let module = make_module(void_fn("main"));
        let class = lower_iir_to_jvm(&module, &make_cfg()).unwrap();
        assert_eq!(class.version.major, 49);
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
            exports: vec![],
            imports: vec![],
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
