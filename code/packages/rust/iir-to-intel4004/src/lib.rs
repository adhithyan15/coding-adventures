//! # iir-to-intel4004 — IIR → Intel 4004 machine code backend (v0.4.0).
//!
//! ## ⚠ DEPRECATED — use `intel4004-backend` instead
//!
//! As of Phase 4 of the historical-arch backend migration
//! ([`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md)),
//! this crate is deprecated.  Use:
//!
//! - **`intel4004-encoder`** — pure encoding tables (opcodes + `encode_*`).
//! - **`intel4004-backend`** — implements `jit_core::backend::Backend`
//!   over monomorphised CIR.
//!
//! `lang-aot --emit=intel4004` routes through the new pair as of
//! Phase 4; existing public API of this crate continues to work
//! for backward compatibility but emits deprecation warnings.
//!
//! ## Original module docs (still applicable to the lowering algorithm)
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u8>` of encoded
//! 8-bit Intel 4004 opcodes, suitable to drop into any 4004
//! simulator or to write out as a flat `.bin` for burning to a
//! 1702/2708 EPROM and plugging into a 4004 dev board.
//!
//! ## Why an Intel 4004 backend?
//!
//! The Intel 4004 (1971) was the **world's first commercial
//! microprocessor**.  Tiny ISA, 4-bit data, 12-bit ROM addresses,
//! single 4-bit accumulator, 16 4-bit registers organised as 8
//! register pairs, tiny ROM (4 KiB max) and RAM (640 bits max).
//!
//! In this codebase the 4004 is primarily a **Brainfuck fit** —
//! BF's minimal needs (single tape pointer, ±1 increment ops,
//! conditional jump on zero) actually do map cleanly to a 4004's
//! accumulator-and-loop programming model.
//!
//! Adding the 4004 gives us:
//!
//! 1. **Historical fidelity.**  The 4004 is where the entire
//!    modern-microprocessor lineage began.
//! 2. **A fourth architecture backend** alongside RV32I (A1),
//!    Intel 8008 (A2), and ARMv7 (A3).  The most constrained
//!    target in the lane by a wide margin.
//! 3. **Stress-tests the IIR's neutrality.**  If a 4-bit-data,
//!    4 KiB-ROM target can swallow the IIR's 64-bit `Operand::Int`
//!    shape (via truncation + range-rejection), every other
//!    backend can too.
//!
//! ## Scope of v0.1.0 (A4)
//!
//! This release is a **skeleton**: any IIR module lowers to a
//! single `JUN 0x000` (jump-unconditional to address 0) infinite-
//! loop halt sentinel, encoded as the two bytes `0x40 0x00`.  No
//! instruction selection yet; that arrives in A4+ and beyond.
//!
//! ## Why `Vec<u8>` output, not textual asm?
//!
//! - **Round-trips with the in-tree intel-4004-assembler and any
//!   4004 simulator** — both consume raw byte streams.
//! - **Deterministic test surface** — 4004 mnemonics have multiple
//!   historical spellings (Intel MCS-4 manual vs modern reverse-
//!   engineered docs).  Bytes are unambiguous.
//! - **Trivial output size** — 4004 instructions are 1 or 2
//!   bytes; emitting bytes directly skips a textual-assembly
//!   round-trip.
//!
//! ## Quick start
//!
//! ```
//! #![allow(deprecated)]
//! use interpreter_ir::IIRModule;
//! use iir_to_intel4004::{validate_for_intel4004, lower_iir_to_intel4004, IIRIntel4004Config};
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
//! assert!(validate_for_intel4004(&module).is_empty());
//!
//! let bytes = lower_iir_to_intel4004(&module, &IIRIntel4004Config::default())
//!     .expect("lowering should succeed");
//! // JUN 0x000 = 0x40 0x00 = the canonical 4004-ROM halt idiom.
//! assert_eq!(bytes, vec![0x40, 0x00]);
//! ```

use interpreter_ir::{IIRModule, Operand};
use std::collections::HashMap;
use std::fmt;

// ===========================================================================
// Intel 4004 opcode constants
// ===========================================================================
//
// The 4004 has no formal HLT.  The canonical "halt" idiom is `JUN
// 0x000` — an unconditional jump back to ROM address 0, which
// (when itself at address 0) loops forever, simulating halt.

/// Intel 4004 `LD r` opcode high nibble — `0xA0`.  Loads register
/// `r` (bottom nibble) into the accumulator.
///
/// Bit pattern: `1010 rrrr`.  Single-byte instruction — OR in the
/// 4-bit register selector (0..=15) to form the full opcode byte.
///
/// Example: `LD r3 = 0xA3 = 0b1010_0011`.
///
/// Does NOT modify the source register `r` — it's a copy, not a swap.
pub const LD_OPCODE: u8 = 0xA0;

/// Intel 4004 `XCH r` opcode high nibble — `0xB0`.  Exchanges the
/// accumulator with register `r`.
///
/// Bit pattern: `1011 rrrr`.  Single-byte instruction.  After
/// execution, `ACC` holds what was in `r`, and `r` holds what was
/// in `ACC`.
///
/// XCH is the only way on the 4004 to STORE the accumulator into
/// a register — there's no `ST r` instruction.  The exchange
/// destroys the previous contents of the destination register,
/// which is fine for fresh allocation but means we must be careful
/// not to overwrite a still-live value.
pub const XCH_OPCODE: u8 = 0xB0;

/// Intel 4004 `LDM n` opcode high nibble — `0xD0`.  Loads the 4-bit
/// immediate `n` (low nibble) into the accumulator.
///
/// Bit pattern: `1101 nnnn` (immediate-load family).  Single-byte
/// instruction — OR in the 4-bit immediate value (0..=15) to form
/// the full opcode byte.
///
/// Example:
/// ```text
/// LDM 0  = 0xD0     (0b1101_0000)
/// LDM 7  = 0xD7     (0b1101_0111)
/// LDM 15 = 0xDF     (0b1101_1111)
/// ```
///
/// The 4004's accumulator is exactly 4 bits wide, so `LDM` is the
/// only "load immediate" instruction needed — there's no wider
/// immediate idiom (no `LDM_16` etc.).  Values wider than 4 bits
/// must be built up via multiple `LDM`/arithmetic-op pairs in
/// future slices.
pub const LDM_OPCODE: u8 = 0xD0;

/// Canonical "halt" sentinel for the Intel 4004 — two bytes:
/// `0x40 0x00` (= `JUN 0x000`, jump-unconditional to ROM address 0).
///
/// Bit layout:
///
/// ```text
/// byte 1: 0100 0000 = 0x40   (JUN opcode + high nibble of 12-bit addr = 0)
/// byte 2: 0000 0000 = 0x00   (low byte of address = 0)
/// ```
///
/// JUN's encoding is `0100 aaaa aaaaaaaa` — the high 4 bits of
/// byte 1 are the JUN opcode (`0100`), the low 4 bits hold the
/// high 4 bits of the 12-bit address, and byte 2 holds the low 8
/// bits.
///
/// When emitted at ROM address 0 (where lowering starts), this
/// instruction's address is 0 and its target is also 0, so the CPU
/// infinitely re-executes it.  Every 4004 simulator and any
/// oscilloscope hooked to the program counter recognises this
/// pattern as "the chip is stuck at address 0".
///
/// ## Why JUN-self over NOP-cycle or unimplemented-opcode?
///
/// | Candidate | Pros | Cons |
/// |-----------|------|------|
/// | `JUN 0x000` (this) | Self-documenting; portable across all 4004 implementations | None for skeleton purposes |
/// | `NOP NOP NOP ...` (0x00 cycle) | Even simpler bytes | Doesn't halt — keeps running into whatever follows |
/// | Unimplemented opcode | Forces a trap | 4004 silicon executes most "unused" bit patterns as NOPs; not portable |
///
/// `JUN 0x000` wins on portability + clarity.
pub const HALT_LOOP: [u8; 2] = [0x40, 0x00];

// ===========================================================================
// IIRIntel4004Config
// ===========================================================================

/// Configuration for the IIR → Intel 4004 lowering pass.
///
/// Currently only the module name is configurable, reserved for
/// future symbol-table / ROM-image emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRIntel4004Config {
    /// Module name — reserved for future symbol-table / `.bin`
    /// header use.
    pub module_name: String,
}

impl IIRIntel4004Config {
    /// Build a config with a custom module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }
}

impl Default for IIRIntel4004Config {
    fn default() -> Self {
        Self {
            module_name: "iir_module".into(),
        }
    }
}

// ===========================================================================
// IIRIntel4004Error
// ===========================================================================

/// Errors that can occur during IIR → Intel 4004 lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRIntel4004Error {
    /// The module failed pre-flight validation.
    ValidationFailed(Vec<String>),
    /// An IIR opcode not yet supported by this backend.  v0.1.0
    /// doesn't lower any instructions, so a non-empty function
    /// body would surface this in a future version.
    UnsupportedOp { function: String, op: String },
    /// A type hint that does not map to any Intel 4004
    /// representation.
    UnsupportedType { function: String, type_hint: String },
    /// An operand has an unexpected shape.
    InvalidOperand { function: String, detail: String },
    /// A variable name was used (via `mov` or `ret`) before it was
    /// bound by `const` or `mov`.
    UndefinedVariable { function: String, name: String },
    /// The function tried to bind more locals than the 4004's 16
    /// 4-bit general-purpose registers can hold.  Stack spilling
    /// via the 4004's data-RAM (`SRC`/`WRM`/`RDM` family) lands in
    /// a future increment.
    OutOfRegisters { function: String, name: String },
}

impl fmt::Display for IIRIntel4004Error {
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
                write!(f, "out of 4004 registers (r0..r15) while binding {name:?} in function {function:?}; data-RAM spilling not yet supported")
            }
        }
    }
}

impl std::error::Error for IIRIntel4004Error {}

// ===========================================================================
// validate_for_intel4004
// ===========================================================================

/// Pre-flight validation for IIR → Intel 4004 lowering.
///
/// **v0.1.0 stub**: always returns an empty `Vec` — there are no
/// validation rules yet because no instructions are lowered.
/// Future versions will add rules as opcodes come online (see
/// `MULTILANG-ARCHITECTURE-BACKENDS.md` §A4).
///
/// Mirrors the shape of the other IIR backends'
/// `validate_for_{wasm,jvm,clr,beam,llvm,riscv,intel8008,armv7}`
/// so callers can switch backends without changing their pre-
/// flight logic.
pub fn validate_for_intel4004(_module: &IIRModule) -> Vec<String> {
    Vec::new()
}

// ===========================================================================
// lower_iir_to_intel4004
// ===========================================================================

/// Sentinel `env` value meaning "this var currently lives in the
/// accumulator (ACC)", not in any of the 16 GP registers.  Distinct
/// from the 4-bit register indices `0..=15`.
const ACC_MARKER: u8 = 16;

/// Supported instruction opcodes in v0.3.0 (A4++).
///
/// * `const dest, Int(n)` lowers to `LDM n` with `dest` taking
///   over the accumulator (evicting the previous ACC owner to a
///   real register via XCH if needed).
/// * `mov dest, src` lowers to `LD src_reg; XCH dest_reg` —
///   copies src's value into dest's freshly-allocated register.
/// * `ret <var>` stages var into ACC (eliding LD when var is
///   already there) and emits the JUN 0x000 halt sentinel.
/// * `ret_void` just emits the halt sentinel.
const SUPPORTED_OPS: &[&str] = &[
    "const", "mov", "ret", "ret_void",
];

/// Lower an [`IIRModule`] to a `Vec<u8>` of Intel 4004 opcode bytes.
///
/// **v0.2.0 scope** (A4+ — first real lowering):
///
/// | IIR op | 4004 lowering |
/// |--------|---------------|
/// | `const dest, Int(n)` (4-bit imm) | `LDM n` (`0xD0 \| n`) |
/// | `ret <var>` | `JUN 0x000` (halt sentinel — real RET in A4++) |
/// | `ret_void` | `JUN 0x000` |
///
/// ### Accumulator-only first slice
///
/// Every `const` loads into the accumulator.  Multi-register
/// allocation via the 4004's 8 register pairs (`r0r1..r14r15`)
/// arrives in A4++ alongside arithmetic.
///
/// ### Why `ret` → halt sentinel for now?
///
/// The 4004's real `RET` is `BBL` (Branch Back to Last; opcode
/// `1100 dddd`).  But `BBL` requires that a corresponding `JMS`
/// (Jump to SubRoutine, opcode `0101 aaaa aaaaaaaa`) have pushed
/// the return address onto the 4004's 3-deep internal call stack
/// first.  Without proper call/return discipline (which lands in
/// A4++), `BBL` from a fresh-start ROM would pop a garbage
/// address from the stack and jump to it — undefined behaviour
/// on most 4004 simulators.
///
/// `JUN 0x000` gives the simulator a clean, deterministic
/// stopping point in the meantime.
///
/// ### Empty-module contract
///
/// Preserves v0.1.0's behaviour for the trivial "fn main() {}"
/// case: any module with no functions emits the bare
/// `HALT_LOOP` so the simulator halts deterministically.  Once
/// at least one function is lowered, the halt sentinel terminates
/// the last function's instruction stream.
#[deprecated(
    since = "0.4.0",
    note = "use `intel4004_backend::compile` over CIR — see code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md"
)]
pub fn lower_iir_to_intel4004(
    module: &IIRModule,
    _cfg: &IIRIntel4004Config,
) -> Result<Vec<u8>, IIRIntel4004Error> {
    let errors = validate_for_intel4004(module);
    if !errors.is_empty() {
        return Err(IIRIntel4004Error::ValidationFailed(errors));
    }

    // Trivial empty-module contract — preserves v0.1.0 callable
    // behaviour for the canonical "fn main() {}" minimal case.
    if module.functions.is_empty() {
        return Ok(HALT_LOOP.to_vec());
    }

    let mut bytes = Vec::new();
    for f in &module.functions {
        // ── Per-function allocator state ──────────────────────────────
        //
        // The 4004's ALU is accumulator-anchored: every value flows
        // through the 4-bit ACC.  We maintain three pieces of state:
        //
        //   env: HashMap<String, u8>
        //     var name → physical location.  Values in `0..=15` are
        //     register indices for the 16 GP registers; `ACC_MARKER`
        //     (= 16) means "currently lives in ACC".
        //
        //   next_reg: usize
        //     Next free GP register index.  Bumps from 0 upward as
        //     we spill names from ACC into r0, r1, ...
        //
        //   acc_owner: Option<String>
        //     Which var (if any) currently owns ACC.  When a new const
        //     or LD-based op arrives that needs to clobber ACC, we
        //     first evict the current owner via XCH.
        //
        // The ACC-first model preserves v0.2.0's 3-byte shape for
        // `const v; ret v` (LDM v; JUN 0x000 — no XCH/LD needed when
        // v is the sole var).  Mirrors iir-to-intel8008 v0.3.0's
        // A-first pool ordering.
        let mut env: HashMap<String, u8> = HashMap::new();
        let mut next_reg: usize = 0;
        let mut acc_owner: Option<String> = None;

        for instr in &f.instructions {
            if !SUPPORTED_OPS.contains(&instr.op.as_str()) {
                return Err(IIRIntel4004Error::UnsupportedOp {
                    function: f.name.clone(),
                    op: instr.op.clone(),
                });
            }
            match instr.op.as_str() {
                // ── const dest, Int(n) → (XCH r_evict)? + LDM n ────────
                //
                // If ACC is currently owned by a different var, we
                // evict that var to its next-free real register via XCH
                // BEFORE the LDM (which would otherwise clobber ACC).
                // Then LDM n loads the new value into ACC, and dest
                // becomes the new ACC owner.
                "const" => {
                    let dest = require_dest(instr, "const", &f.name)?;
                    let n = encode_immediate_nibble(instr.srcs.first(), &f.name)?;
                    evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg, &f.name)?;
                    bytes.push(LDM_OPCODE | n);
                    env.insert(dest.to_string(), ACC_MARKER);
                    acc_owner = Some(dest.to_string());
                }

                // ── mov dest, src → LD src_reg; XCH dest_reg ───────────
                //
                // `mov` copies src's value into a freshly-allocated
                // dest register.  Lowering shape:
                //
                //   [optional] if src is in ACC: evict src to a real
                //     register first (so LD/XCH below have a stable
                //     source).
                //   LD r_src             ; ACC ← src's value
                //   alloc r_dest
                //   XCH r_dest           ; r_dest = src's value,
                //                        ; ACC = old r_dest (junk)
                "mov" => {
                    let dest = require_dest(instr, "mov", &f.name)?;
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel4004Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "mov srcs[0] must be Var".into(),
                        }),
                    };
                    // If src lives in ACC, evict so the LD below has
                    // a stable register source.
                    if matches!(env.get(&src_name), Some(&ACC_MARKER)) {
                        evict_acc(&mut bytes, &mut env, &mut acc_owner, &mut next_reg, &f.name)?;
                    }
                    let src_reg = lookup_register(&env, &src_name, &f.name)?;
                    debug_assert!(src_reg <= 15, "src_reg should be a real GP register after eviction");
                    bytes.push(LD_OPCODE | src_reg);
                    let r_dest = alloc_register(&mut next_reg, dest, &f.name)?;
                    bytes.push(XCH_OPCODE | r_dest);
                    env.insert(dest.to_string(), r_dest);
                    // After XCH, ACC holds the (junk) old r_dest contents.
                    acc_owner = None;
                }

                // ── ret <var>: stage var into ACC, then JUN 0x000 ──────
                //
                // If var is already the ACC owner, no LD is needed —
                // ACC already holds its value.  Otherwise emit
                // `LD r_var` to stage it.
                "ret" => {
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(s)) => s.clone(),
                        _ => return Err(IIRIntel4004Error::InvalidOperand {
                            function: f.name.clone(),
                            detail: "ret srcs[0] must be Var".into(),
                        }),
                    };
                    let src_reg = lookup_register(&env, &src_name, &f.name)?;
                    if src_reg != ACC_MARKER {
                        bytes.push(LD_OPCODE | src_reg);
                    }
                    bytes.extend_from_slice(&HALT_LOOP);
                }

                // ── ret_void: JUN 0x000 ────────────────────────────────
                "ret_void" => {
                    bytes.extend_from_slice(&HALT_LOOP);
                }

                _ => unreachable!("SUPPORTED_OPS guard above prevents this"),
            }
        }
    }

    // Defensive — if a function had no instructions at all, fall
    // back to HALT_LOOP so the output is still a valid halting
    // program.
    if bytes.is_empty() {
        bytes.extend_from_slice(&HALT_LOOP);
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
) -> Result<&'a str, IIRIntel4004Error> {
    instr.dest.as_deref().ok_or_else(|| IIRIntel4004Error::InvalidOperand {
        function: fn_name.to_string(),
        detail: format!("{op} requires a dest"),
    })
}

/// Evict the current ACC owner (if any) to its next-free GP register
/// via `XCH r_evict`.  Updates the `env` mapping and clears
/// `acc_owner`.  No-op when ACC is unowned.
///
/// Returns `OutOfRegisters` if all 16 GP registers are already
/// allocated.
fn evict_acc(
    bytes: &mut Vec<u8>,
    env: &mut HashMap<String, u8>,
    acc_owner: &mut Option<String>,
    next_reg: &mut usize,
    fn_name: &str,
) -> Result<(), IIRIntel4004Error> {
    if let Some(name) = acc_owner.take() {
        if *next_reg >= 16 {
            return Err(IIRIntel4004Error::OutOfRegisters {
                function: fn_name.to_string(),
                name,
            });
        }
        let r = *next_reg as u8;
        *next_reg += 1;
        bytes.push(XCH_OPCODE | r);
        env.insert(name, r);
    }
    Ok(())
}

/// Allocate a fresh GP register for `dest`.  Returns the 4-bit
/// register index, or `OutOfRegisters` if all 16 are taken.
fn alloc_register(
    next_reg: &mut usize,
    dest: &str,
    fn_name: &str,
) -> Result<u8, IIRIntel4004Error> {
    if *next_reg >= 16 {
        return Err(IIRIntel4004Error::OutOfRegisters {
            function: fn_name.to_string(),
            name: dest.to_string(),
        });
    }
    let r = *next_reg as u8;
    *next_reg += 1;
    Ok(r)
}

fn lookup_register(
    env: &HashMap<String, u8>,
    name: &str,
    fn_name: &str,
) -> Result<u8, IIRIntel4004Error> {
    env.get(name).copied().ok_or_else(|| IIRIntel4004Error::UndefinedVariable {
        function: fn_name.to_string(),
        name: name.to_string(),
    })
}

fn encode_immediate_nibble(
    op: Option<&Operand>,
    fn_name: &str,
) -> Result<u8, IIRIntel4004Error> {
    let n = match op {
        Some(Operand::Int(n)) => *n,
        Some(Operand::Bool(b)) => if *b { 1 } else { 0 },
        _ => return Err(IIRIntel4004Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: "const srcs[0] must be Int or Bool".into(),
        }),
    };
    if (0..=15).contains(&n) {
        Ok(n as u8)
    } else if (-8..0).contains(&n) {
        // Two's-complement reinterpretation: -1 → 0xF, -8 → 0x8.
        Ok((n & 0xF) as u8)
    } else {
        Err(IIRIntel4004Error::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "const {n} exceeds 4-bit nibble range ([-8, 15]); the \
                 4004's accumulator and LDM immediate are both 4 bits \
                 wide — wider values must be built up via multiple \
                 LDM/arithmetic-op pairs in A4++"
            ),
        })
    }
}
