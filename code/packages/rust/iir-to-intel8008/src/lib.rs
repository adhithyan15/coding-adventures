//! # iir-to-intel8008 — IIR → Intel 8008 machine code backend (v0.1.0 skeleton).
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
// ALU_CMP = 0b111 lives in the v0.3.4 slice — `cmp` produces a
// boolean dest at the IIR level but the 8008 CMP only sets flags
// (discards the difference), so the lowering needs an extra
// flag-to-register capture sequence (typically a conditional load
// or a paired conditional jump).  That's wired alongside the
// branch ops, not here.

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
    // A2++.5.5 third slice — labels + unconditional jump (`cmp` and
    // conditional jumps deferred to v0.3.5; they need flag-to-bool
    // capture).
    "label", "jmp",
];

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

    let mut bytes = Vec::new();
    for f in &module.functions {
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

                // ── ret <var>: stage value in A, then HLT ───────────────
                //
                // If `var`'s register is already A, the MOV is omitted.
                // Real RET via CALL/stack lands in A2++.5 — until then,
                // HLT is the universal stopping primitive.
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
                    bytes.push(HLT);
                }

                // ── ret_void: just HLT ──────────────────────────────────
                "ret_void" => {
                    bytes.push(HLT);
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
