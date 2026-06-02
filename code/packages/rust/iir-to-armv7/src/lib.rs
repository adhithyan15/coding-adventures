//! # iir-to-armv7 — IIR → ARMv7 (A32) machine code backend (v0.1.0 skeleton).
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u32>` of encoded
//! 32-bit ARMv7-A (A32) instructions, suitable to drop into the
//! in-tree `arm-simulator` or to write out as a flat `.bin` for
//! `qemu-arm` / `objcopy`.
//!
//! ## Why an ARMv7 backend?
//!
//! ARMv7 (32-bit ARM, A32 encoding) is the **phone-class target** of
//! the LANG VM architecture-backend lane.  It covers Cortex-A7/A8/A9-
//! era SoCs and many embedded boards (early Raspberry Pi, BeagleBone,
//! Olimex A20-OLinuXino) — vastly more deployed silicon than any
//! single 8008 chip ever shipped, but architecturally a clean
//! fixed-width 32-bit RISC like RV32I.
//!
//! Adding ARMv7 as a backend gives us:
//!
//! 1. **A third architecture backend** alongside RV32I (A1) and
//!    Intel 8008 (A2).  The three sit at meaningfully different
//!    points in the design space:
//!    - RV32I: clean 32-bit RISC, load-store, no condition codes.
//!    - Intel 8008: irregular 8-bit accumulator CISC, 14-bit address
//!      bus — historical fidelity for Oct.
//!    - **ARMv7 (A32)**: 32-bit RISC with a `cond` field on EVERY
//!      instruction plus a barrel shifter on the second operand.
//!      Same word width as RV32I, fundamentally different ISA.
//! 2. **Foundation for native phone-OS targets.**  Once the AOT
//!    wiring lands (A3+++), the same LANG VM source program can
//!    cross-compile to ARMv7 Linux executables.
//! 3. **Round-trip with the in-tree `arm-simulator`.**  The
//!    `Vec<u32>` output drops directly into the simulator for
//!    in-process tests.
//!
//! ## Scope of v0.1.0 (A3)
//!
//! This release is a **skeleton**: any IIR module lowers to a single
//! `BKPT #0xFFFF` instruction (encoding `0xE12FFF7F`).  No instruction
//! selection yet; that arrives in A3+ (`const` + `bx lr`) and beyond.
//!
//! ## Why `Vec<u32>` output, not textual asm?
//!
//! - **Round-trips with `arm-simulator`** — its decoder consumes raw
//!   little-endian 32-bit words.
//! - **Deterministic test surface** — `assert_eq!(words[0], 0xE12FFF7F)`
//!   is unambiguous; ARM assembler syntax has GNU `as`, LLVM `clang`,
//!   and ARMASM divergence we don't want to entangle with.
//! - **Trivial encoding shape** — every A32 instruction is exactly 4
//!   bytes (in stark contrast to the 8008's 1/2/3 byte variability).
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::IIRModule;
//! use iir_to_armv7::{validate_for_armv7, lower_iir_to_armv7, IIRArmv7Config};
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
//! assert!(validate_for_armv7(&module).is_empty());
//!
//! let words = lower_iir_to_armv7(&module, &IIRArmv7Config::default())
//!     .expect("lowering should succeed");
//! // 0xE12FFF7F == ARMv7-A BKPT #0xFFFF.
//! assert_eq!(words, vec![0xE12F_FF7F]);
//! ```
//!
//! ## Pipeline position
//!
//! ```text
//! IIRModule
//!   → validate_for_armv7()      pre-flight, returns Vec<String>
//!   → lower_iir_to_armv7()      returns Vec<u32> of A32 words
//!   → (optional)
//!       • arm-simulator: in-process testing
//!       • write to .bin + qemu-arm
//!       • objcopy + linker for an ELF on a phone-class Linux board
//! ```

use interpreter_ir::{IIRModule, Operand};
use std::fmt;

// ===========================================================================
// ARMv7 (A32) opcode constants
// ===========================================================================
//
// Every A32 instruction is exactly 4 bytes, with a fixed encoding
// template `cond IIII OOOO ... ` where `cond` is the conditional-
// execution prefix (the 4-bit field bits 31..28 every A32 instruction
// carries) and `IIII OOOO` selects the instruction family.  Unlike
// RV32I and the 8008, ARMv7 has no "unconditional" sub-encoding for
// most ops — `cond = 0b1110 = 0xE` is the "always-execute" value used
// everywhere a conditional prefix isn't actively wanted.

/// ARMv7-A `BKPT #0xFFFF` opcode — `0xE12FFF7F`.  Triggers a
/// breakpoint exception; semantically "stop execution".
///
/// Bit layout (cond=AL):
///
/// ```text
/// 31..28  cond    = 0xE = 1110            (always — unconditional)
/// 27..20          = 0001 0010 = 0x12      (BKPT opcode family)
/// 19.. 8  imm12   = 0xFFF                 (top 12 bits of imm16)
///  7.. 4          = 0111 = 0x7            (BKPT opcode family)
///  3.. 0  imm4    = 0xF                   (bottom 4 bits of imm16)
/// ```
///
/// Concatenated: `1110 0001 0010 1111_1111_1111 0111 1111` =
/// `0xE12FFF7F`.
///
/// ## Why BKPT and not WFI or `b .`?
///
/// | Candidate | Pros | Cons |
/// |-----------|------|------|
/// | `BKPT #imm16` | Semantically "stop"; every ARM debugger / emulator recognises it | None for skeleton purposes |
/// | `WFI`         | True halt | Requires kernel/hypervisor privilege; illegal in userspace |
/// | `B .`         | Pure userspace, no traps | Burns CPU; harder to detect without a host timeout |
///
/// BKPT wins on simplicity + emulator round-trip.  The
/// `arm-simulator`'s decoder flags it as `bkpt` and stops single-
/// stepping.
pub const BKPT: u32 = 0xE12F_FF7F;

/// ARMv7-A `BX LR` opcode — `0xE12FFF1E`.  Branches to the address in
/// the link register (`r14`), exchanging instruction sets (which on
/// pure A32 code is a no-op — A32 → A32).  Semantically: "return from
/// this function" per the AAPCS calling convention.
///
/// Bit layout (cond=AL):
///
/// ```text
/// 31..28  cond  = 0xE = 1110            (always — unconditional)
/// 27..20        = 0001 0010 = 0x12      (BX opcode family)
/// 19.. 8        = 1111 1111 1111 = 0xFFF
///  7.. 4        = 0001 = 0x1            (BX opcode family)
///  3.. 0  Rm    = 1110 = 0xE            (Rm = lr = r14)
/// ```
///
/// Concatenated: `1110 0001 0010 1111_1111_1111 0001 1110` =
/// `0xE12FFF1E`.
///
/// CAREFUL: BX is `0xE12FFF1E`, NOT `0xE12FFF7F` (which is BKPT —
/// the bit-7 difference distinguishes "branch & exchange" from
/// "breakpoint").  Both share the same `12F_FF` family bits.
pub const BX_LR: u32 = 0xE12F_FF1E;

/// ARMv7-A `MOV Rd, #imm8` (data-processing immediate) base
/// encoding for `Rd = r0` — `0xE3A0_0000`.  OR in the 8-bit immediate
/// (bits 7..0) and the destination register (bits 15..12) to form the
/// full instruction word.
///
/// Bit layout (cond=AL, S=0, Rn=0):
///
/// ```text
/// 31..28  cond     = 0xE = 1110           (always — unconditional)
/// 27..25           = 001                   (data-processing immediate)
/// 24..21  opcode   = 1101                 (MOV)
/// 20      S        = 0                     (don't set flags)
/// 19..16  Rn       = 0000                  (unused for MOV)
/// 15..12  Rd       = (in this base, 0)    (target register)
/// 11.. 8  rotate   = 0000                  (no rotation on the imm)
///  7.. 0  imm8     = (in this base, 0)    (the 8-bit value)
/// ```
///
/// Concatenated for `MOV r0, #0`: `1110 0011 1010 0000 0000 0000 0000 0000`
/// = `0xE3A00000`.
///
/// For `MOV r0, #N`: OR in `N` (8 bits).
/// For `MOV Rd, #N`: OR in `(Rd << 12) | N`.
pub const MOV_IMM_R0_BASE: u32 = 0xE3A0_0000;

/// Encode an `ARMv7-A `MOV Rd, #imm8`` instruction.
///
/// `rd` must be in `[0, 15]` (4-bit ARM register selector).  `imm8`
/// is the immediate value, range `[0, 255]`.
///
/// Wider immediates (9-32 bits) require either a rotate (the 12-bit
/// immediate field encodes 8 value bits + 4 rotation bits, allowing
/// any rotated 8-bit value) or a `movw`/`movt` pair (ARMv7+).  Those
/// land in A3++ — v0.2.0's `const` only supports 8-bit values.
pub(crate) fn encode_mov_imm(rd: u8, imm8: u8) -> u32 {
    debug_assert!(rd <= 15, "rd out of 4-bit range: {rd}");
    MOV_IMM_R0_BASE | ((rd as u32) << 12) | (imm8 as u32)
}

// ===========================================================================
// IIRArmv7Config
// ===========================================================================

/// Configuration for the IIR → ARMv7 lowering pass.
///
/// Currently only the module name is configurable, reserved for future
/// symbol-table / ELF-section emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRArmv7Config {
    /// Module name — reserved for future symbol-table / `.bin` header use.
    pub module_name: String,
}

impl IIRArmv7Config {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRArmv7Config {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRArmv7Error
// ===========================================================================

/// Errors that can occur during IIR → ARMv7 lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRArmv7Error {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0
    /// doesn't lower any instructions, so a non-empty function body
    /// would surface this in a future version.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any ARMv7 representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
}

impl fmt::Display for IIRArmv7Error {
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
        }
    }
}

impl std::error::Error for IIRArmv7Error {}

// ===========================================================================
// validate_for_armv7
// ===========================================================================

/// Pre-flight validation for IIR → ARMv7 lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.  Future
/// versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A3).
///
/// Mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam,llvm,riscv,intel8008}` so callers
/// can switch backends without changing their pre-flight logic.
pub fn validate_for_armv7(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_armv7
// ===========================================================================

/// Register `r0` — the AAPCS first-argument / return-value register.
/// Every `const` in v0.2.0 (A3+) lowers to `mov r0, #imm` because
/// we don't yet have a multi-register allocator (A3++).
const REG_R0: u8 = 0;

/// Supported instruction opcodes in v0.2.0 (A3+).
///
/// * `const dest, Int(n)` lowers to `mov r0, #n` (every value goes
///   into `r0` in this slice).
/// * `ret <var>` and `ret_void` both lower to `bx lr` (the AAPCS
///   return convention — the value is already in `r0` by
///   construction).
const SUPPORTED_OPS: &[&str] = &[
    "const", "ret", "ret_void",
];

/// Lower an [`IIRModule`] to a `Vec<u32>` of ARMv7 (A32) opcode words.
///
/// **v0.2.0 scope** (A3+ — first real lowering):
///
/// | IIR op | A32 lowering |
/// |--------|--------------|
/// | `const dest, Int(n)` (8-bit imm) | `mov r0, #n` (`0xE3A0_00NN`) |
/// | `ret <var>` (int) | `bx lr` (`0xE12FFF1E`) — `var` is already in `r0` |
/// | `ret_void` | `bx lr` |
///
/// ### Accumulator-only first slice
///
/// Every `const` allocates to `r0` — the AAPCS return-value register.
/// A real linear allocator over `r0..r12` (and the v0.3.x ARM
/// equivalent of v0.3.0's RISC-V move) arrives in A3++.
///
/// ### Empty-module contract
///
/// Preserves v0.1.0's behaviour for the trivial "`fn main() {}`" case:
/// any module with no functions emits a single `BKPT #0xFFFF` so the
/// in-tree `arm-simulator` halts deterministically.  Once at least
/// one function is lowered, the BKPT is replaced by the function's
/// real instruction stream.
pub fn lower_iir_to_armv7(
    module: &IIRModule,
    _cfg: &IIRArmv7Config,
) -> Result<Vec<u32>, IIRArmv7Error> {
    let errors = validate_for_armv7(module);
    if !errors.is_empty() {
        return Err(IIRArmv7Error::ValidationFailed(errors));
    }

    // Trivial empty-module contract — preserves v0.1.0 callable behaviour
    // for the canonical "fn main() {}" minimal case.
    if module.functions.is_empty() {
        return Ok(vec![BKPT]);
    }

    let mut words = Vec::new();
    for f in &module.functions {
        for instr in &f.instructions {
            if !SUPPORTED_OPS.contains(&instr.op.as_str()) {
                return Err(IIRArmv7Error::UnsupportedOp {
                    function: f.name.clone(),
                    op: instr.op.clone(),
                });
            }
            match instr.op.as_str() {
                // ── const dest, Int(n) → MOV R0, #n ─────────────────
                //
                // The accumulator-only first slice: every const goes
                // into r0.  Multi-register allocation lands in A3++.
                "const" => {
                    let _dest = require_dest(instr, "const", &f.name)?;
                    let imm8 = encode_immediate_byte(instr.srcs.first(), &f.name)?;
                    words.push(encode_mov_imm(REG_R0, imm8));
                }

                // ── ret <var> → BX LR ──────────────────────────────
                //
                // The value is already in r0 (every const lowers
                // there in v0.2.0).  Per AAPCS, returning a value
                // means leaving it in r0 and branching to lr.  No
                // staging MOV needed in this slice.
                "ret" => {
                    // Validate that srcs[0] is a Var — front-end bugs
                    // surface as `InvalidOperand` rather than producing
                    // surprising silence.
                    match instr.srcs.first() {
                        Some(Operand::Var(_)) => {}
                        _ => return Err(IIRArmv7Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "ret srcs[0] must be Var".into(),
                        }),
                    };
                    words.push(BX_LR);
                }

                // ── ret_void → BX LR ───────────────────────────────
                "ret_void" => {
                    words.push(BX_LR);
                }

                _ => unreachable!("SUPPORTED_OPS guard above prevents this"),
            }
        }
    }

    // Defensive — if a function had no instructions at all, fall back
    // to BKPT so the output is still a valid halting program.
    if words.is_empty() {
        words.push(BKPT);
    }

    Ok(words)
}

// ---------------------------------------------------------------------------
// Per-instruction helpers
// ---------------------------------------------------------------------------

fn require_dest<'a>(
    instr: &'a interpreter_ir::IIRInstr,
    op: &str,
    fn_name: &str,
) -> Result<&'a str, IIRArmv7Error> {
    instr.dest.as_deref().ok_or_else(|| IIRArmv7Error::InvalidOperand {
        function: fn_name.to_string(),
        detail: format!("{op} requires a dest"),
    })
}

fn encode_immediate_byte(
    op: Option<&Operand>,
    fn_name: &str,
) -> Result<u8, IIRArmv7Error> {
    let n = match op {
        Some(Operand::Int(n)) => *n,
        Some(Operand::Bool(b)) => if *b { 1 } else { 0 },
        _ => return Err(IIRArmv7Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: "const srcs[0] must be Int or Bool".into(),
        }),
    };
    if (0..=255).contains(&n) {
        Ok(n as u8)
    } else if (-128..0).contains(&n) {
        Ok((n as i8) as u8)
    } else {
        Err(IIRArmv7Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "const {n} exceeds 8-bit byte range ([-128, 255]); A32's \
                 12-bit MOV immediate field supports rotated 8-bit values \
                 — wider raw immediates need a `movw`/`movt` pair, which \
                 lands in A3++"
            ),
        })
    }
}
