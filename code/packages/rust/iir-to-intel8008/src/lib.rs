//! # iir-to-intel8008 — IIR → Intel 8008 machine code backend (v0.4.0).
//!
//! ## ⚠ DEPRECATED — use `intel8008-backend` instead
//!
//! As of Phase 6 of the historical-arch backend migration, this
//! crate is deprecated.  Use `intel8008-encoder` for byte
//! encoding and `intel8008-backend` for CIR lowering via the
//! `Backend` trait.  `intel8008-backend` v0.1.0 is a minimal-
//! viable port (just `const_*` + `ret_*`); the full op set this
//! crate had can be ported in future increments.
//!
//! ## Original module docs (still applicable)
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u8>` of encoded
//! 8-bit Intel 8008 opcodes, suitable to drop into the in-tree
//! [`intel8008-simulator`] or to write out as a flat `.bin` for an
//! external emulator.
//!
//! ## Why an Intel 8008 backend?
//!
//! The Intel 8008 (1972) is the first commercial 8-bit microprocessor.
//! In this codebase it's **Oct's native target** — the Oct front-end
//! produces IIR specifically intended to round-trip through an 8008.
//!
//! Adding the 8008 as a backend gives us:
//!
//! 1. **Historical fidelity.**  Oct programs can run on the actual ISA
//!    they were designed for.
//! 2. **A second architecture backend** alongside RISC-V (A1).  The two
//!    architectures sit at opposite ends of the design space: RV32I is
//!    a clean modern load-store RISC; the 8008 is an irregular
//!    accumulator-based CISC with 8-bit registers and 14-bit
//!    addressing.  Each exposes a different set of constraints in the
//!    backend interface.
//! 3. **A foundation for A4 (Intel 4004)** — the 4004 is even more
//!    constrained, but shares much of the historical-microprocessor
//!    backend shape that A2 establishes.
//!
//! ## Scope of v0.1.0 (A2)
//!
//! This release is a **skeleton**: any IIR module lowers to a single
//! `HLT` instruction (opcode `0x76`).  No instruction selection yet;
//! that arrives in A2+ (basic arithmetic / MOV / register allocation)
//! and beyond.
//!
//! ## Why `Vec<u8>` output, not textual asm?
//!
//! - **Round-trips with `intel8008-simulator`** — its
//!   [`Simulator::run`] method consumes raw `&[u8]` instruction streams.
//! - **Deterministic test surface** — `assert_eq!(bytes, vec![0x76])`
//!   is unambiguous; assembler syntax for the 8008 has Intel-mnemonics
//!   vs MCS-8 historical-mnemonics divergence.
//! - **Trivial output size** — Intel 8008 instructions are 1, 2, or 3
//!   bytes; emitting bytes directly skips a textual-assembly round-trip
//!   that contributes nothing.
//!
//! ## Quick start
//!
//! ```
//! #![allow(deprecated)]
//! use interpreter_ir::IIRModule;
//! use iir_to_intel8008::{validate_for_intel8008, lower_iir_to_intel8008, IIRIntel8008Config};
//!
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![],
//!     entry_point: None,
//!     language: "demo".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! assert!(validate_for_intel8008(&module).is_empty());
//!
//! let bytes = lower_iir_to_intel8008(&module, &IIRIntel8008Config::default())
//!     .expect("lowering should succeed");
//! // 0x76 == Intel 8008 HLT.
//! assert_eq!(bytes, vec![0x76]);
//! ```
//!
//! ## Pipeline position
//!
//! ```text
//! IIRModule
//!   → validate_for_intel8008()    pre-flight, returns Vec<String>
//!   → lower_iir_to_intel8008()    returns Vec<u8> of 8008 opcodes
//!   → (optional)
//!       • intel8008_simulator::Simulator::run for in-process testing
//!       • write to .bin + external emulator
//!       • burn to a 1702 EPROM (Oct's intended deployment path)
//! ```

use interpreter_ir::{IIRModule, Operand};
use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// Intel 8008 opcode constants
// ===========================================================================
//
// The 8008's instruction encoding is irregular by modern standards: the
// top 2 bits group instructions into four families (immediate-byte ops,
// register-register MOV, ALU on register, and condition/jump/call), and
// the lower 6 bits select within the family.  HLT lives in the
// register-register MOV family but encodes "MOV M,M" — semantically a
// self-move on the memory-pointer pseudo-register — which the silicon
// implements as a halt.

/// Intel 8008 `HLT` opcode — `0x76`.  Halts the CPU.
pub const HLT: u8 = 0x76;

/// Intel 8008 `MVI A, imm8` first byte — `0x3E`.  Loads the next byte
/// into the accumulator register.
///
/// Bit pattern: `00 111 110` (immediate-load family `00 rrr 110`, where
/// `rrr = 111 = A`).  Two-byte instruction: this opcode plus the literal
/// immediate byte.
pub const MVI_A: u8 = 0x3E;

/// Intel 8008 conditional `JFC addr` (jump if flag-carry clear) first
/// byte — `0x40`.  Three-byte instruction (`JFC` + low + high).
///
/// Bit pattern: `01 000 000` (jump family `01 ccc T 00`, where
/// `ccc = 000 = carry flag` and `T = 0 = jump if flag clear`).
/// After a `CMP r` (which computes `A - r`), carry is SET iff
/// `A < r` (i.e. a borrow happened).  So `JFC` is the "skip if
/// `A >= r`" half — used by the `cmp_lt` capture to default-out of
/// the "set-true" path.
pub const JFC: u8 = 0x40;

/// Intel 8008 conditional `JTC addr` (jump if flag-carry set) first
/// byte — `0x44`.  Three-byte instruction.
///
/// Bit pattern: `01 000 100` (carry flag, `T = 1 = jump if flag set`).
/// `JTC` is the "skip if `A < r`" half — used by `cmp_gte` to default
/// to `0` and only set `1` when the carry isn't set after CMP.
///
/// CAREFUL: `0x44` is NOT the unconditional `JMP` (which is `0x7C`).
/// The encoding cheat-sheet in `iir-to-intel8008.md` lists every
/// group-01 jump/call opcode side-by-side so this confusion doesn't
/// recur — same hazard the v0.3.4 commit message flagged for
/// `JMP ↔ JFC` and `CAL ↔ CFZ`.
pub const JTC: u8 = 0x44;

/// Intel 8008 conditional `JFS addr` (jump if sign clear, "positive
/// or zero") first byte — `0x50`.  Three-byte instruction.
///
/// Bit pattern: `01 010 000` (sign flag, T = 0).  After `CMP r`,
/// sign is SET when the high bit of `A - r` is 1 — useful only for
/// signed integer ordering, which doesn't yet land in IIR.  Pinned
/// here so the spec's encoding cheat-sheet stays in sync with the
/// public surface.
pub const JFS: u8 = 0x50;

/// Intel 8008 conditional `JTS addr` (jump if sign set, "negative")
/// first byte — `0x54`.  Three-byte instruction.  Bit pattern:
/// `01 010 100`.  Sibling of `JFS`; both pinned for forward
/// compatibility with signed-arithmetic lowerings.
pub const JTS: u8 = 0x54;

/// Intel 8008 conditional `JFP addr` (jump if parity clear, "odd")
/// first byte — `0x58`.  Three-byte instruction.  Bit pattern:
/// `01 011 000`.  Parity flag is rarely used by high-level IIR
/// constructs; pinned for completeness and round-trip fidelity with
/// the simulator's decoder.
pub const JFP: u8 = 0x58;

/// Intel 8008 conditional `JTP addr` (jump if parity set, "even")
/// first byte — `0x5C`.  Three-byte instruction.  Bit pattern:
/// `01 011 100`.  Sibling of `JFP`.
pub const JTP: u8 = 0x5C;

/// Intel 8008 conditional `JFZ addr` (jump if flag-zero clear) first
/// byte — `0x48`.  Three-byte instruction (`JFZ` + low addr + high addr).
///
/// Bit pattern: `01 001 000` (jump family `01 ccc T 00`, where
/// `ccc = 001 = zero flag` and `T = 0 = jump if flag is *clear*`).
/// "Flag clear" for the zero flag means the last arithmetic/logical
/// op produced a NON-zero result.  So `JFZ` is the "jump if true"
/// half of a boolean test.
pub const JFZ: u8 = 0x48;

/// Intel 8008 conditional `JTZ addr` (jump if flag-zero set) first
/// byte — `0x4C`.  Three-byte instruction.
///
/// Bit pattern: `01 001 100` (`ccc = 001 = zero`, `T = 1 = jump if
/// flag is *set*`).  "Flag set" means the last op produced a zero
/// result, so `JTZ` is the "jump if false" half.
pub const JTZ: u8 = 0x4C;

/// Intel 8008 unconditional `RET` opcode — `0x07`.  Pops the
/// top of the CPU's internal 7-deep return-address stack and jumps
/// there.  Single-byte instruction.
///
/// Bit pattern: `00 000 111`.  Lives in the immediate-byte family
/// (`00 xxx 110/111`) but with the low 3 bits = `111` instead of
/// `110` — distinct from `MVI rrr, n` which has the trailing `110`.
///
/// CAREFUL: `0x07` is NOT to be confused with `RFC` (return if flag-
/// carry clear, `0x03`) or its conditional siblings.  This is the
/// unconditional variant — always returns.
pub const RET: u8 = 0x07;

/// Intel 8008 unconditional `CAL addr` first byte — `0x7E`.  Pushes
/// the address of the NEXT instruction (= PC + 3, the byte after the
/// 3-byte CAL) onto the CPU's internal return-address stack, then
/// jumps to `addr`.  Three-byte instruction (CAL + low byte + high byte).
///
/// Bit pattern: `01 111 110`.
///
/// CRITICAL: `CAL` is `0x7E`, NOT `0x46`.  `0x46` is `CFZ` (call if
/// flag-zero clear) — a *conditional* call that the silicon will
/// silently take or skip based on whatever zero-flag state happened
/// to be live.  The same family of confusion as `JMP ↔ JFC` flagged
/// in v0.3.4 and `JTC ↔ JMP` flagged in v0.3.8.  Pin to `0x7E` so
/// the simulator's disassembler correctly reports "CAL" and the
/// silicon executes the call unconditionally.
pub const CAL: u8 = 0x7E;

/// Intel 8008 unconditional `JMP addr` first byte — `0x7C`.  Followed
/// by two address bytes (low byte then high byte) — the 8008 has a
/// 14-bit address space so only the bottom 6 bits of the high byte are
/// significant (top 2 bits ignored by the silicon).
///
/// Bit pattern: `01 111 100`.  In the group-01 family the disambiguation
/// is via `ddd` (bits 5-3): `ddd = 111` is the unconditional variant;
/// `ddd ≤ 011` selects one of the four conditional jump opcodes (JFC,
/// JFZ, JFS, JFP at `ddd = 000…011`; JTC, JTZ, JTS, JTP at the matching
/// `01 ccc 100` with T-bit set in `sss`).  Three-byte instruction total.
///
/// CAREFUL: it is NOT `0x44` — `0x44` is JFC (jump if flag-carry clear),
/// a *conditional* jump that the silicon will silently take in unexpected
/// states.  Pin to `0x7C` so the simulator's disassembler correctly
/// reports "JMP" and the silicon takes the branch unconditionally.
pub const JMP: u8 = 0x7C;

// ===========================================================================
// Intel 8008 register encoding
// ===========================================================================
//
// The 8008's 3-bit register field uses these codes:
//
//   A = 7   (111)  accumulator
//   B = 0   (000)
//   C = 1   (001)
//   D = 2   (010)
//   E = 3   (011)
//   H = 4   (100)
//   L = 5   (101)
//   M = 6   (110)  memory pseudo-register (not allocated to)
//
// MVI rrr, n   = 00 rrr 110             = `(rrr << 3) | 0x06`  + imm byte
// MOV ddd, sss = 01 ddd sss             = `(ddd << 3) | sss | 0x40`
//
// The MVI/MOV opcode generators below assert their inputs fit in 3 bits
// — anything outside [0, 7] would corrupt the surrounding bit-fields.

/// Linear-allocator pool ordered to keep the trivial `const v; ret v`
/// case at one MVI byte: A is handed out first, so `ret v` finds the
/// value already in A and skips the redundant `MOV A, X` round-trip.
const REGISTER_POOL: [u8; 7] = [7, 0, 1, 2, 3, 4, 5]; // A, B, C, D, E, H, L

/// The accumulator register (`A = 0b111 = 7`).  `ret <var>` moves the
/// return value into A before halting; `ret_void` doesn't touch it.
const REG_A: u8 = 7;

/// Encode an `MVI rrr, imm8` first byte.
///
/// `rrr` must be in `[0, 7]` (any of the 7 GP registers + the M
/// pseudo-register; in practice we only ever pass GP-register indices).
fn encode_mvi(rrr: u8) -> u8 {
    debug_assert!(rrr <= 7, "register index out of 3-bit range: {rrr}");
    (rrr << 3) | 0x06
}

/// Encode a `MOV ddd, sss` opcode (single byte, family `01 ddd sss`).
fn encode_mov(ddd: u8, sss: u8) -> u8 {
    debug_assert!(ddd <= 7, "ddd out of 3-bit range: {ddd}");
    debug_assert!(sss <= 7, "sss out of 3-bit range: {sss}");
    0x40 | (ddd << 3) | sss
}

/// Encode an accumulator-target ALU opcode in family `10 ooo sss`.
///
/// The 3-bit `ooo` selects the operation (`000` = ADD, `010` = SUB,
/// `111` = CMP, etc.).  The 3-bit `sss` selects the right-hand source
/// register — the left-hand source AND destination are always the
/// accumulator `A`, which is the 8008's defining accumulator-based ISA
/// shape.
fn encode_alu(ooo: u8, sss: u8) -> u8 {
    debug_assert!(ooo <= 7, "ooo out of 3-bit range: {ooo}");
    debug_assert!(sss <= 7, "sss out of 3-bit range: {sss}");
    0x80 | (ooo << 3) | sss
}

/// ALU operation codes (`ooo` field in `10 ooo sss`).
const ALU_ADD: u8 = 0b000; // ADD r
const ALU_ADC: u8 = 0b001; // ACA r (add with carry-in)
const ALU_SUB: u8 = 0b010; // SUB r
const ALU_SBB: u8 = 0b011; // SCA r (sub with borrow-in)
const ALU_AND: u8 = 0b100; // ANA r (logical AND)
const ALU_XOR: u8 = 0b101; // XRA r (logical XOR)
const ALU_OR:  u8 = 0b110; // ORA r (logical OR)
const ALU_CMP: u8 = 0b111; // CMP r — A - r, sets flags, DISCARDS result.
                           // Z=1 iff A == r.  Wired in v0.3.6 with a
                           // flag-to-bool capture sequence using JFZ.

// ===========================================================================
// IIRIntel8008Config
// ===========================================================================

/// Configuration for the IIR → Intel 8008 lowering pass.
///
/// Currently only the module name is configurable, reserved for future
/// symbol-table / ROM-image emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRIntel8008Config {
    /// Module name — reserved for future symbol-table / `.bin` header use.
    pub module_name: String,
}

impl IIRIntel8008Config {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRIntel8008Config {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRIntel8008Error
// ===========================================================================

/// Errors that can occur during IIR → Intel 8008 lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRIntel8008Error {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0 doesn't
    /// lower any instructions, so a non-empty function body would
    /// surface this in a future version.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any Intel 8008 representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
    /// A variable name was used (via `mov` or `ret`) before it was bound
    /// by `const` or `mov`.
    UndefinedVariable { function: String, name: String },
    /// The function tried to bind more locals than the 8008's 7
    /// general-purpose registers (A/B/C/D/E/H/L) can hold.  Stack
    /// spilling lands in a future increment (A2++.5 or later).
    OutOfRegisters { function: String, name: String },
    /// A `jmp` referenced a label name that wasn't defined by a
    /// `label` op anywhere in the same function.  Cross-function
    /// jumps aren't supported — labels are per-function in v0.3.4.
    UndefinedLabel { function: String, label: String },
    /// A function emitted more than the 8008's 14-bit address space
    /// (16384 bytes) — `jmp` targets cannot be encoded.  Practical
    /// programs are tens to hundreds of bytes so this is a guard
    /// against runaway expansion, not a real workload concern.
    AddressOutOfRange { function: String, address: usize },
    /// A `call` referenced a function name that wasn't defined
    /// anywhere in the module.  Cross-module calls aren't yet
    /// supported — that lands with `lang-aot --target=intel8008` in
    /// A2+++.
    UndefinedFunction { caller: String, callee: String },
}

impl fmt::Display for IIRIntel8008Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationFailed(errs) => {
                write!(f, "validation failed:\n  {}", errs.join("\n  "))
            }
            Self::UnsupportedOp { function, op } => {
                write!(f, "unsupported op in function {function:?}: {op}")
            }
            Self::UnsupportedType { function, type_hint } => {
                write!(f, "unsupported type in function {function:?}: {type_hint}")
            }
            Self::InvalidOperand { function, detail } => {
                write!(f, "invalid operand in function {function:?}: {detail}")
            }
            Self::UndefinedVariable { function, name } => {
                write!(f, "undefined variable {name:?} in function {function:?}")
            }
            Self::OutOfRegisters { function, name } => {
                write!(f, "out of 8008 registers (A/B/C/D/E/H/L) while binding {name:?} in function {function:?}; stack spilling not yet supported")
            }
            Self::UndefinedLabel { function, label } => {
                write!(f, "undefined label {label:?} referenced by jmp in function {function:?}")
            }
            Self::AddressOutOfRange { function, address } => {
                write!(f, "function {function:?} grew to address 0x{address:04x}, exceeding the 8008's 14-bit (16384-byte) address space")
            }
            Self::UndefinedFunction { caller, callee } => {
                write!(f, "undefined function {callee:?} called from {caller:?}")
            }
        }
    }
}

impl std::error::Error for IIRIntel8008Error {}

// ===========================================================================
// validate_for_intel8008
// ===========================================================================

/// Pre-flight validation for IIR → Intel 8008 lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.  Future
/// versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A2).
///
/// Mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam,llvm,riscv}` so callers can switch
/// backends without changing their pre-flight logic.
pub fn validate_for_intel8008(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_intel8008
// ===========================================================================

/// Lower an [`IIRModule`] to a `Vec<u8>` of Intel 8008 opcode bytes.
///
/// **v0.1.0 scope**: emits a single `HLT` regardless of the input.
/// This is the smallest "valid Intel 8008 program" we can produce —
/// enough to load into the simulator, step once, and confirm
/// [`Simulator::halted`] returns `true`.  Real lowering arrives in
/// v0.2.0+ (A2+).
/// Supported instruction opcodes in v0.3.0 (A2++).
///
/// `const` lowers to `MVI rrr, n` with `rrr` allocated from the
/// `REGISTER_POOL`.  `mov` lowers to `MOV ddd, sss`.  `ret` moves the
/// value into `A` (if not already there) and emits `HLT`; `ret_void`
/// just emits `HLT`.  Anything else is `UnsupportedOp`.
const SUPPORTED_OPS: &[&str] = &[
    // A2 / A2+ / A2++
    "const", "mov", "ret", "ret_void",
    // A2++.5 — accumulator ALU
    "add", "sub",
    // A2++.5.5 — bitwise accumulator ALU
    "and", "or", "xor",
    // A2++.5.5 second slice — carry/borrow chained ALU
    "adc", "sbb",
    // A2++.5.5 third slice — labels + unconditional jump.
    "label", "jmp",
    // A2++.5.5 fourth slice — boolean-conditional jumps (just the
    // zero-flag pair JFZ/JTZ; the remaining 6 flag opcodes defer to
    // v0.3.7).
    "jmp_if_true", "jmp_if_false",
    // A2++.5.5 fifth slice — equality comparison with flag-to-bool capture.
    "cmp",
    // A2++.5.5 sixth slice — inequality + ordering comparisons.  All
    // share the same "CMP + capture sequence" skeleton as `cmp`,
    // differing only in (a) which conditional jump opcode skips the
    // "set true" path, and (b) whether the operands are swapped
    // before staging.
    "cmp_ne", "cmp_lt", "cmp_gt",
    // A2++.5.5 seventh slice — closed-end ordering (a >= b, a <= b).
    // Both slot into the same shared `emit_cmp_capture` helper via
    // `JTC` (jump if carry SET) — the natural complement of `JFC` —
    // with `cmp_lte` reusing `cmp_gte` via the same operand-swap
    // trick that v0.3.7 used for `cmp_gt`.
    "cmp_gte", "cmp_lte",
    // A2++.5.5 eighth slice — real function calls + returns via the
    // 8008's internal 7-deep return-address stack.  `call dest, fn`
    // emits CAL + 14-bit address + captures the return value from A
    // into dest_reg.  `ret`/`ret_void` switch from HLT to RET for
    // non-entry-point functions (entry-point keeps HLT — calling RET
    // there would pop a garbage address from an empty stack).
    "call",
];

#[deprecated(
    since = "0.4.0",
    note = "use `intel8008_backend::compile` over CIR — see code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md"
)]
pub fn lower_iir_to_intel8008(
    module: &IIRModule,
    _cfg: &IIRIntel8008Config,
) -> Result<Vec<u8>, IIRIntel8008Error> {
    let errors = validate_for_intel8008(module);
    if !errors.is_empty() {
        return Err(IIRIntel8008Error::ValidationFailed(errors));
    }

    // Trivial empty-module contract — preserves v0.1.0 callable behaviour
    // for the canonical "fn main() {}" minimal case.
    if module.functions.is_empty() {
        return Ok(vec![HLT]);
    }

    // ── Module-level call-site resolution state (v0.3.9) ────────────────
    //
    // Each function's start byte offset is recorded as we walk
    // `module.functions` in source order.  When a `call <fn_name>` is
    // emitted, we record (slot, fn_name) into `pending_calls`; after
    // every function has been emitted, a final pass backpatches each
    // pending CAL with the 14-bit absolute address of its target.
    //
    // This mirrors `iir-to-riscv`'s A1++.5.5 module-level jal
    // resolution: cross-function references are the simplest motivation
    // for separating local-jump backpatching (per-function) from
    // call-site resolution (module-level).
    let mut function_addrs: HashMap<String, usize> = HashMap::new();
    let mut pending_calls: Vec<(usize, String, String /* caller */)> = Vec::new();
    // Entry-point name (if any) drives the HLT-vs-RET decision in `ret`/
    // `ret_void` lowering — the entry-point can't safely return via RET
    // (it would pop a garbage address from an empty return stack).
    let entry_point = module.entry_point.as_deref();

    let mut bytes = Vec::new();
    for f in &module.functions {
        // Record this function's start address before emitting its body.
        function_addrs.insert(f.name.clone(), bytes.len());
        let is_entry = Some(f.name.as_str()) == entry_point;
        // ── Per-function allocator state ──────────────────────────────
        //
        // IIR var name → its assigned 3-bit register index.  Sequentially
        // hands out registers from REGISTER_POOL starting with A — this
        // keeps the trivial `const v; ret v` case at the same 3-byte
        // shape as v0.2.0 (no redundant `MOV A, X` round-trip).
        let mut env: HashMap<String, u8> = HashMap::new();
        let mut next_reg: usize = 0;

        // ── Per-function label-resolution state (v0.3.4) ──────────────
        //
        // The 8008 has no PC-relative addressing — every `jmp` carries
        // a full 14-bit absolute target.  We emit jumps in pass 1 with
        // placeholder zero bytes at the address slots, recording
        // `(slot_byte_offset, target_label)` in `pending_jmps`.  After
        // the function's instruction list is walked, pass 2 looks up
        // each pending jmp's label in `labels` and backpatches the two
        // address bytes (low then high; only the bottom 6 bits of the
        // high byte are significant — the 8008's address bus is 14
        // bits wide).
        //
        // A `jmp` to a label defined LATER in the function (forward
        // jump) is the whole point of the two-pass approach — pass 1
        // can't know the address yet.
        //
        // Labels are scoped per-function.  A `jmp` to a label that
        // isn't defined in the same function surfaces as
        // `UndefinedLabel`; cross-function jumps (which would also
        // need module-level call-site resolution like A1++.5.5)
        // aren't supported in v0.3.4.
        //
        // Byte offsets here are *relative to the start of the module*
        // (not the start of the function), because that's the actual
        // physical address each instruction will reside at when the
        // module's `Vec<u8>` is loaded at address 0 in the 8008's
        // memory map.  For now we assume modules load at address 0;
        // wider relocation lands with the AOT wiring in A2+++.
        let mut labels: HashMap<String, usize> = HashMap::new();
        let mut pending_jmps: Vec<(usize, String)> = Vec::new();

        for instr in &f.instructions {
            if !SUPPORTED_OPS.contains(&instr.op.as_str()) {
                return Err(IIRIntel8008Error::UnsupportedOp {
                    function: f.name.clone(),
                    op: instr.op.clone(),
                });
            }
            match instr.op.as_str() {
                // ── const dest, Int(n) → MVI dest_reg, n ────────────────
                "const" => {
                    let dest = require_dest(instr, "const", &f.name)?;
                    let byte = encode_immediate_byte(instr.srcs.first(), &f.name)?;
                    let rrr = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                    bytes.push(encode_mvi(rrr));
                    bytes.push(byte);
                }

                // ── mov dest, src → MOV ddd, sss ────────────────────────
                //
                // If the source and dest happen to be the same register
                // (unlikely under SSA but possible if upstream re-binds a
                // name), we emit no byte — the move is a no-op.
                "mov" => {
                    let dest = require_dest(instr, "mov", &f.name)?;
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "mov srcs[0] must be Var".into(),
                        }),
                    };
                    let sss = lookup_register(&env, &src_name, &f.name)?;
                    let ddd = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                    if ddd != sss {
                        bytes.push(encode_mov(ddd, sss));
                    }
                }

                // ── ret <var>: stage value in A, then RET (or HLT for entry) ─
                //
                // The 8008's return-value calling convention puts the
                // value in `A`.  We stage `var` into A (eliding the
                // MOV if already there) and then:
                //   - emit `HLT` for the module's entry-point function
                //     (RET would underflow the empty return stack).
                //   - emit `RET` (`0x07`) for any other function.
                "ret" => {
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "ret srcs[0] must be Var".into(),
                        }),
                    };
                    let sss = lookup_register(&env, &src_name, &f.name)?;
                    if sss != REG_A {
                        bytes.push(encode_mov(REG_A, sss));
                    }
                    bytes.push(if is_entry { HLT } else { RET });
                }

                // ── ret_void: HLT for the entry-point, RET otherwise ────
                "ret_void" => {
                    bytes.push(if is_entry { HLT } else { RET });
                }

                // ── add / adc / sub / sbb / and / or / xor → MOV A,a; OP b_reg; MOV dest,A
                //
                // All seven accumulator-target ALU ops share an identical
                // shape, differing only in the 3-bit `ooo` selector:
                //
                //   ADD = 0b000   AND = 0b100
                //   ADC = 0b001   XOR = 0b101
                //   SUB = 0b010   OR  = 0b110
                //   SBB = 0b011
                //
                // `cmp` (ooo = 0b111) is intentionally NOT here — it sets
                // flags without producing a register result, so its
                // lowering needs an extra capture sequence and lands with
                // the branch ops in v0.3.4.
                //
                // The 8008's ALU is *always* accumulator-anchored: left
                // source AND destination are A; only the right source is
                // a variable register.  Optional MOVs at the ends collapse
                // when a_reg or dest_reg already coincide with A.
                //
                // ADC / SBB read the carry flag bit set by a PRIOR ALU op
                // — they don't change shape here, but front-ends should
                // ensure no carry-clobbering op runs between the
                // producer (e.g. an ADD that overflowed) and the
                // consumer.  This crate doesn't reorder instructions; it
                // emits them in source order, so the contract is the
                // front-end's to uphold.
                "add" | "adc" | "sub" | "sbb" | "and" | "or" | "xor" => {
                    let dest = require_dest(instr, &instr.op, &f.name)?;
                    let a_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} srcs[0] must be Var", instr.op),
                        }),
                    };
                    let b_name = match instr.srcs.get(1) {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} srcs[1] must be Var", instr.op),
                        }),
                    };
                    let a_reg = lookup_register(&env, &a_name, &f.name)?;
                    let b_reg = lookup_register(&env, &b_name, &f.name)?;
                    // Stage a into A if it isn't already there.
                    if a_reg != REG_A {
                        bytes.push(encode_mov(REG_A, a_reg));
                    }
                    // Execute the ALU op (result lands in A).  Dispatch
                    // on op-name → 3-bit `ooo` selector.  The `_` arm is
                    // unreachable because the outer match arm above is the
                    // only path here and it covers exactly these five
                    // strings.
                    let ooo = match instr.op.as_str() {
                        "add" => ALU_ADD,
                        "adc" => ALU_ADC,
                        "sub" => ALU_SUB,
                        "sbb" => ALU_SBB,
                        "and" => ALU_AND,
                        "or"  => ALU_OR,
                        "xor" => ALU_XOR,
                        _ => unreachable!("outer arm restricts to these 7"),
                    };
                    bytes.push(encode_alu(ooo, b_reg));
                    // Capture the result into dest_reg unless dest_reg is A.
                    let dest_reg = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                    if dest_reg != REG_A {
                        bytes.push(encode_mov(dest_reg, REG_A));
                    }
                }

                // ── label "<name>": record the current byte offset ──────
                //
                // Zero bytes emitted.  `label` is purely a marker so a
                // subsequent (or preceding, via pass-2 backpatching)
                // `jmp` can resolve to a concrete 14-bit address.
                //
                // A duplicate label name overwrites the prior position —
                // a front-end bug we could catch with a validator pass,
                // but for v0.3.4 the latest definition wins (mirrors
                // iir-to-riscv's v0.3.1 semantics).
                // ── cmp dest, a, b → boolean equality test ────────────
                //
                // IIR-level `cmp dest, a, b` produces a boolean
                // (`dest = (a == b) ? 1 : 0`).  The 8008's CMP
                // (family `10 111 sss` = `0xB8 | sss`) computes
                // `A - r`, sets the zero flag (`Z = 1 iff A == r`),
                // and *discards* the difference.  So we need a
                // flag-to-register capture sequence after CMP.
                //
                // Lowering shape (8 bytes when a is already in A,
                // 9 when it isn't):
                //
                //   [optional]  MOV A, a_reg           ; stage a
                //   CMP b_reg     (0xB8 | b_reg)       ; sets Z
                //   MVI dest, 0                        ; default false
                //   JFZ <fallthrough>                  ; Z=0 (a != b) → skip overwrite
                //   MVI dest, 1                        ; Z=1 (a == b) → set true
                //   <-- fallthrough lives here -->
                //
                // The JFZ's target is computed inline (NOT via the
                // two-pass `pending_jmps` mechanism) because:
                //   - The target is always a fixed +4-byte forward
                //     offset from the JFZ instruction itself, so it
                //     can be resolved at emit time.
                //   - This keeps the capture sequence fully
                //     self-contained — no synthetic label names
                //     leaking into the user-visible `labels` map.
                //
                // Note: only EQUALITY (`a == b`) is supported in
                // v0.3.6.  Less-than / greater-than need the sign +
                // carry flags and land with the other 6 conditional
                // jump opcodes in v0.3.7.
                // ── cmp / cmp_ne / cmp_lt / cmp_gt ──────────────────────
                //
                // All four boolean comparisons share the same CMP +
                // flag-to-bool capture skeleton.  They differ only in
                // two parameters:
                //
                //   1. Whether to swap operands before staging.
                //      `cmp_gt a, b` is just `cmp_lt b, a` — both
                //      compute "carry set after CMP" → `a < b` (for
                //      cmp_lt, after staging a; for cmp_gt, after
                //      staging b so the CMP becomes `b - a` and
                //      carry-set means `b < a` ⇔ `a > b`).
                //
                //   2. Which conditional jump opcode SKIPS the
                //      "set-true" overwrite:
                //        cmp     → JFZ (skip when Z clear / a != b)
                //        cmp_ne  → JTZ (skip when Z set   / a == b)
                //        cmp_lt  → JFC (skip when C clear / a >= b)
                //        cmp_gt  → JFC (skip when C clear / b >= a)
                //
                // Both axes feed a single helper below so any future
                // tweak to the capture shape (e.g. shrinking via a
                // different idiom) only needs one site change.
                "cmp" | "cmp_ne" | "cmp_lt" | "cmp_gt" | "cmp_gte" | "cmp_lte" => {
                    let dest = require_dest(instr, &instr.op, &f.name)?;
                    let a_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} srcs[0] must be Var", instr.op),
                        }),
                    };
                    let b_name = match instr.srcs.get(1) {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} srcs[1] must be Var", instr.op),
                        }),
                    };
                    let a_reg = lookup_register(&env, &a_name, &f.name)?;
                    let b_reg = lookup_register(&env, &b_name, &f.name)?;
                    // Decide skip-jump opcode AND whether to swap.
                    let (skip_op, swap) = match instr.op.as_str() {
                        "cmp"     => (JFZ, false),
                        "cmp_ne"  => (JTZ, false),
                        "cmp_lt"  => (JFC, false),
                        "cmp_gt"  => (JFC, true),
                        // a >= b iff NOT (a < b) iff carry CLEAR after CMP b.
                        // Skip-when-true (false) means skip when carry SET → JTC.
                        "cmp_gte" => (JTC, false),
                        // a <= b iff b >= a — same skip opcode as cmp_gte,
                        // operands swapped (so CMP a runs after staging b).
                        "cmp_lte" => (JTC, true),
                        _ => unreachable!("outer arm restricts to these 6"),
                    };
                    let (left, right) = if swap { (b_reg, a_reg) } else { (a_reg, b_reg) };
                    // Allocate dest_reg up front so the helper can
                    // emit MVI dest, {0,1} sequences.
                    let dest_reg = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                    emit_cmp_capture(&mut bytes, left, right, dest_reg, skip_op, &f.name)?;
                }

                "label" => {
                    let name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "label requires srcs[0] = Operand::Var(name)".into(),
                        }),
                    };
                    labels.insert(name, bytes.len());
                }

                // ── jmp "<name>": unconditional 3-byte jump (`JMP addr`)
                //
                // Pass 1: emit `0x7C` followed by two zero bytes as
                // placeholders for the 14-bit absolute target.  Record
                // the offset of the low-address byte so pass 2 can
                // backpatch in `(addr & 0xFF, (addr >> 8) & 0x3F)`.
                //
                // CRITICAL: `JMP` is `0x7C`, NOT `0x44`.  `0x44` is JFC
                // (jump if flag-carry clear) — a *conditional* jump.
                // In the 8008's group-01 family the unconditional
                // variant has `ddd = 111`, encoded as `01 111 100`.
                "jmp" => {
                    let target = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "jmp requires srcs[0] = Operand::Var(target_label)".into(),
                        }),
                    };
                    bytes.push(JMP);
                    let slot = bytes.len();
                    bytes.push(0); // low-address placeholder
                    bytes.push(0); // high-address placeholder
                    pending_jmps.push((slot, target));
                }

                // ── call dest, "<fn_name>" → CAL + capture A into dest ──
                //
                // Operand layout: srcs = [Var(fn_name)], optional dest.
                //
                // Pass 1: emit `0x7E 0x00 0x00` and record
                // (slot, fn_name, caller) into the module-level
                // `pending_calls` for the final backpatching pass.
                //
                // Argument passing isn't yet supported — calls in v0.3.9
                // are zero-arg, single-return-value (via A).  Mirrors
                // iir-to-riscv's A1++.5.5 staging where args came in a
                // later A1++.5.5.5.
                //
                // CRITICAL: CAL is `0x7E`, NOT `0x46`.  `0x46` is `CFZ`
                // (call if flag-zero clear) — a conditional call.  Same
                // family of confusion as `JMP ↔ JFC`.
                "call" => {
                    let fn_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "call requires srcs[0] = Operand::Var(fn_name)".into(),
                        }),
                    };
                    bytes.push(CAL);
                    let slot = bytes.len();
                    bytes.push(0); // low-address placeholder
                    bytes.push(0); // high-address placeholder
                    pending_calls.push((slot, fn_name, f.name.clone()));
                    // If the IIR site binds a dest, capture the return
                    // value from A into dest_reg.  A bare `call` without
                    // a dest discards the return value (a "void call").
                    if let Some(dest) = instr.dest.as_deref() {
                        let dest_reg = alloc_register(&mut next_reg, dest, &mut env, &f.name)?;
                        if dest_reg != REG_A {
                            bytes.push(encode_mov(dest_reg, REG_A));
                        }
                    }
                }

                // ── jmp_if_true / jmp_if_false ─────────────────────────
                //
                // Operand layout: srcs = [Var(cond), Var(target_label)].
                //
                // The 8008 has no "branch on register" — every
                // conditional branch reads ONE of the four CPU flags
                // (carry, zero, sign, parity) set by the LAST
                // arithmetic/logical op.  To branch on a boolean
                // register's value we have to provoke a flag from it:
                //
                //   MOV A, cond_reg   ; load cond into A (skipped if
                //                     ; cond already in A — MOV is
                //                     ; flag-non-affecting on 8008)
                //   ANA A             ; A := A & A (no change), sets
                //                     ; Z flag from A's value
                //   JFZ target        ; if Z==0 (A was non-zero, i.e.
                //                     ; cond was true)  → jump
                //   JTZ target        ; if Z==1 (A was zero, i.e.
                //                     ; cond was false) → jump
                //
                // ANA A (`0xA7`) is the canonical 8008 "TEST A"
                // idiom — same role as `test eax, eax` on x86.
                //
                // The MOV is elided when cond_reg is already A,
                // which is common when this branch immediately
                // follows the cond's producer (e.g. an `add` that
                // landed in A).
                //
                // Choice of JFZ for "true" / JTZ for "false":
                //   Z=1 iff A==0 iff cond is the boolean "false".
                //   So we jump-if-false on JTZ ("zero flag SET")
                //   and jump-if-true  on JFZ ("zero flag CLEAR").
                "jmp_if_true" | "jmp_if_false" => {
                    let cond_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} requires srcs[0] = Operand::Var(cond)", instr.op),
                        }),
                    };
                    let target = match instr.srcs.get(1) {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel8008Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: format!("{} requires srcs[1] = Operand::Var(target_label)", instr.op),
                        }),
                    };
                    let cond_reg = lookup_register(&env, &cond_name, &f.name)?;
                    // Stage cond into A if not already there.
                    if cond_reg != REG_A {
                        bytes.push(encode_mov(REG_A, cond_reg));
                    }
                    // ANA A — sets Z flag from A's current value (TEST idiom).
                    bytes.push(encode_alu(ALU_AND, REG_A));
                    // The branch opcode encodes the polarity.
                    let branch_opcode = if instr.op == "jmp_if_true" {
                        JFZ // jump if Z clear (cond was non-zero / true)
                    } else {
                        JTZ // jump if Z set (cond was zero / false)
                    };
                    bytes.push(branch_opcode);
                    let slot = bytes.len();
                    bytes.push(0); // low-address placeholder
                    bytes.push(0); // high-address placeholder
                    pending_jmps.push((slot, target));
                }

                _ => unreachable!("SUPPORTED_OPS guard above prevents this"),
            }
        }

        // ── Pass 2: backpatch pending jmps ────────────────────────────
        //
        // Each pending entry is `(low_byte_slot, label_name)`.  Look up
        // the label's recorded byte position, range-check against the
        // 14-bit address space (16384 bytes), then write the low byte
        // and the bottom 6 bits of the high byte at `slot` and
        // `slot + 1` respectively.
        //
        // The top 2 bits of the high address byte are written as zero —
        // the 8008's address bus is 14 bits wide, so the silicon
        // ignores them, but emitting clean zeros makes the byte stream
        // unambiguously disassemble back to the same `JMP addr` even
        // when downstream tools sign-extend or print all 8 bits.
        for (slot, label) in pending_jmps {
            let target = *labels.get(&label).ok_or_else(|| {
                IIRIntel8008Error::UndefinedLabel {
                    function: f.name.clone(),
                    label: label.clone(),
                }
            })?;
            if target >= 1 << 14 {
                return Err(IIRIntel8008Error::AddressOutOfRange {
                    function: f.name.clone(),
                    address: target,
                });
            }
            bytes[slot]     = (target & 0xFF) as u8;
            bytes[slot + 1] = ((target >> 8) & 0x3F) as u8;
        }
    }

    // ── Module-level call-backpatching pass (v0.3.9) ──────────────
    //
    // All functions have been emitted, so `function_addrs` has every
    // valid call target.  Walk `pending_calls` and write the 14-bit
    // absolute address of each callee into the placeholder slots.
    for (slot, callee, caller) in &pending_calls {
        let target = *function_addrs.get(callee).ok_or_else(|| {
            IIRIntel8008Error::UndefinedFunction {
                caller: caller.clone(),
                callee: callee.clone(),
            }
        })?;
        if target >= 1 << 14 {
            return Err(IIRIntel8008Error::AddressOutOfRange {
                function: caller.clone(),
                address: target,
            });
        }
        bytes[*slot]     = (target & 0xFF) as u8;
        bytes[*slot + 1] = ((target >> 8) & 0x3F) as u8;
    }

    if bytes.is_empty() {
        bytes.push(HLT);
    }

    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Per-instruction helpers
// ---------------------------------------------------------------------------

fn require_dest<'a>(
    instr: &'a interpreter_ir::IIRInstr,
    op: &str,
    fn_name: &str,
) -> Result<&'a str, IIRIntel8008Error> {
    instr.dest.as_deref().ok_or_else(|| IIRIntel8008Error::InvalidOperand {
        function: fn_name.to_string(),
        detail: format!("{op} requires a dest"),
    })
}

fn encode_immediate_byte(
    op: Option<&Operand>,
    fn_name: &str,
) -> Result<u8, IIRIntel8008Error> {
    let n = match op {
        Some(Operand::Int(n)) => *n,
        Some(Operand::Bool(b)) => if *b { 1 } else { 0 },
        _ => return Err(IIRIntel8008Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: "const srcs[0] must be Int or Bool".into(),
        }),
    };
    if (0..=255).contains(&n) {
        Ok(n as u8)
    } else if (-128..0).contains(&n) {
        Ok((n as i8) as u8)
    } else {
        Err(IIRIntel8008Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "const {n} exceeds 8-bit byte range ([-128, 255]); 8008 \
                 has no wide-immediate idiom — split into multiple MVIs \
                 in A2++"
            ),
        })
    }
}

fn alloc_register(
    next_reg: &mut usize,
    dest: &str,
    env: &mut HashMap<String, u8>,
    fn_name: &str,
) -> Result<u8, IIRIntel8008Error> {
    if *next_reg >= REGISTER_POOL.len() {
        return Err(IIRIntel8008Error::OutOfRegisters {
            function: fn_name.to_string(),
            name: dest.to_string(),
        });
    }
    let rrr = REGISTER_POOL[*next_reg];
    *next_reg += 1;
    env.insert(dest.to_string(), rrr);
    Ok(rrr)
}

fn lookup_register(
    env: &HashMap<String, u8>,
    name: &str,
    fn_name: &str,
) -> Result<u8, IIRIntel8008Error> {
    env.get(name).copied().ok_or_else(|| IIRIntel8008Error::UndefinedVariable {
        function: fn_name.to_string(),
        name: name.to_string(),
    })
}

/// Emit the shared CMP + flag-to-bool capture sequence used by
/// `cmp` / `cmp_ne` / `cmp_lt` / `cmp_gt`.
///
/// Stages `left_reg` into `A` if needed (MOV is flag-non-affecting on
/// 8008, so the flag the CMP sets afterwards isn't polluted), runs
/// `CMP right_reg` to populate Z and C, then emits the 7-byte capture:
///
/// ```text
/// MVI dest_reg, 0
/// <skip_op> <fallthrough>     ; 3-byte conditional jump
/// MVI dest_reg, 1
/// <fallthrough>
/// ```
///
/// `skip_op` is the conditional jump that should be TAKEN when the
/// comparison result is the BOOLEAN `false`.  For equality (`cmp`)
/// that's `JFZ` (Z clear = not equal).  For ordering (`cmp_lt`/
/// `cmp_gt`) that's `JFC` (carry clear = not less).  For inequality
/// (`cmp_ne`) that's `JTZ` (Z set = equal).
///
/// The forward target is computed inline (`bytes.len() + 4`) — the
/// same self-contained approach used by v0.3.6's `cmp` so no
/// synthetic labels leak into the user-visible namespace.  The
/// 14-bit `AddressOutOfRange` guard still runs.
fn emit_cmp_capture(
    bytes: &mut Vec<u8>,
    left_reg: u8,
    right_reg: u8,
    dest_reg: u8,
    skip_op: u8,
    fn_name: &str,
) -> Result<(), IIRIntel8008Error> {
    // Stage left into A.
    if left_reg != REG_A {
        bytes.push(encode_mov(REG_A, left_reg));
    }
    // CMP right_reg — sets Z and C flags.
    bytes.push(encode_alu(ALU_CMP, right_reg));
    // Capture: default to false.
    bytes.push(encode_mvi(dest_reg));
    bytes.push(0);
    // Conditional skip jump (its semantics determine which 8008 flag
    // we observe and which polarity makes the boolean false).
    bytes.push(skip_op);
    let target = bytes.len() + 4;
    if target >= 1 << 14 {
        return Err(IIRIntel8008Error::AddressOutOfRange {
            function: fn_name.to_string(),
            address: target,
        });
    }
    bytes.push((target & 0xFF) as u8);
    bytes.push(((target >> 8) & 0x3F) as u8);
    // Set true (executed only when the skip jump WASN'T taken).
    bytes.push(encode_mvi(dest_reg));
    bytes.push(1);
    Ok(())
}
