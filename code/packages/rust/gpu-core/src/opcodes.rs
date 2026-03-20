//! Opcodes and Instructions -- the vocabulary of GPU core programs.
//!
//! # What is an Opcode?
//!
//! An opcode (operation code) is a number or name that tells the processor what
//! to do. It's like a verb in a sentence:
//!
//! ```text
//! English:  "Add the first two numbers and store in the third"
//! Assembly: FADD R2, R0, R1
//! ```
//!
//! The opcode is FADD. The registers R0, R1, R2 are the operands.
//!
//! # Instruction Representation
//!
//! Real GPU hardware represents instructions as binary words (32 or 64 bits of
//! 1s and 0s packed together). But at this layer -- the processing element
//! simulator -- we use a structured Rust enum and struct instead:
//!
//! ```text
//! Binary (real hardware): 01001000_00000010_00000000_00000001
//! Our representation:     Instruction { opcode: Opcode::Fadd, rd: 2, rs1: 0, rs2: 1 }
//! ```
//!
//! Why? Because binary encoding is the job of the *assembler* layer above us.
//! The processing element receives already-decoded instructions from the
//! instruction cache. We're simulating what happens *after* decode.
//!
//! # The Instruction Set
//!
//! Our GenericISA has 16 opcodes organized into four categories:
//!
//! ```text
//! Arithmetic:  Fadd, Fsub, Fmul, Ffma, Fneg, Fabs  (6 opcodes)
//! Memory:      Load, Store                            (2 opcodes)
//! Data move:   Mov, Limm                              (2 opcodes)
//! Control:     Beq, Blt, Bne, Jmp, Nop, Halt          (6 opcodes)
//! ```
//!
//! This is deliberately minimal. Real ISAs have hundreds of opcodes, but these
//! 16 are enough to write any floating-point program (they're Turing-complete
//! when combined with branches and memory).
//!
//! # Helper Constructors
//!
//! Writing programs as raw `Instruction { ... }` calls is verbose. The helper
//! functions (`fadd`, `fmul`, `ffma`, `load`, `store`, `limm`, `halt`, etc.)
//! make programs readable:
//!
//! ```text
//! // Without helpers (verbose):
//! let program = vec![
//!     Instruction { opcode: Opcode::Limm, rd: 0, immediate: 2.0, ..Default::default() },
//!     Instruction { opcode: Opcode::Limm, rd: 1, immediate: 3.0, ..Default::default() },
//!     Instruction { opcode: Opcode::Fmul, rd: 2, rs1: 0, rs2: 1, ..Default::default() },
//!     Instruction { opcode: Opcode::Halt, ..Default::default() },
//! ];
//!
//! // With helpers (clean):
//! let program = vec![
//!     limm(0, 2.0),
//!     limm(1, 3.0),
//!     fmul(2, 0, 1),
//!     halt(),
//! ];
//! ```

use std::fmt;

/// The set of operations a GPU core can perform.
///
/// Organized by category:
///
/// **Floating-point arithmetic** (uses `fp-arithmetic` crate):
/// - `Fadd` -- add two registers
/// - `Fsub` -- subtract two registers
/// - `Fmul` -- multiply two registers
/// - `Ffma` -- fused multiply-add (three source registers)
/// - `Fneg` -- negate a register
/// - `Fabs` -- absolute value of a register
///
/// **Memory operations:**
/// - `Load` -- load float from memory into register
/// - `Store` -- store register value to memory
///
/// **Data movement:**
/// - `Mov` -- copy one register to another
/// - `Limm` -- load an immediate (literal) float value
///
/// **Control flow:**
/// - `Beq` -- branch if equal
/// - `Blt` -- branch if less than
/// - `Bne` -- branch if not equal
/// - `Jmp` -- unconditional jump
/// - `Nop` -- no operation
/// - `Halt` -- stop execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    // Arithmetic
    Fadd,
    Fsub,
    Fmul,
    Ffma,
    Fneg,
    Fabs,

    // Memory
    Load,
    Store,

    // Data movement
    Mov,
    Limm,

    // Control flow
    Beq,
    Blt,
    Bne,
    Jmp,
    Nop,
    Halt,
}

impl Opcode {
    /// Return the assembly mnemonic for this opcode.
    ///
    /// This matches the Python implementation's string values: "fadd", "fsub", etc.
    pub fn mnemonic(&self) -> &'static str {
        match self {
            Opcode::Fadd => "FADD",
            Opcode::Fsub => "FSUB",
            Opcode::Fmul => "FMUL",
            Opcode::Ffma => "FFMA",
            Opcode::Fneg => "FNEG",
            Opcode::Fabs => "FABS",
            Opcode::Load => "LOAD",
            Opcode::Store => "STORE",
            Opcode::Mov => "MOV",
            Opcode::Limm => "LIMM",
            Opcode::Beq => "BEQ",
            Opcode::Blt => "BLT",
            Opcode::Bne => "BNE",
            Opcode::Jmp => "JMP",
            Opcode::Nop => "NOP",
            Opcode::Halt => "HALT",
        }
    }
}

/// A single GPU core instruction.
///
/// This is a structured representation of an instruction, not a binary
/// encoding. It contains all the information needed to execute the
/// instruction: the opcode and up to four operands.
///
/// # Fields
///
/// - `opcode`: What operation to perform (see [`Opcode`] enum).
/// - `rd`: Destination register index (0-255).
/// - `rs1`: First source register index (0-255).
/// - `rs2`: Second source register index (0-255).
/// - `rs3`: Third source register (used only by FFMA).
/// - `immediate`: A literal float value (used by LIMM, branch offsets,
///   memory offsets). For branches, this is the number of instructions to
///   skip (positive = forward, negative = back).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub rd: usize,
    pub rs1: usize,
    pub rs2: usize,
    pub rs3: usize,
    pub immediate: f64,
}

impl Default for Instruction {
    fn default() -> Self {
        Self {
            opcode: Opcode::Nop,
            rd: 0,
            rs1: 0,
            rs2: 0,
            rs3: 0,
            immediate: 0.0,
        }
    }
}

impl fmt::Display for Instruction {
    /// Pretty-print the instruction in assembly-like syntax.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.opcode {
            Opcode::Fadd | Opcode::Fsub | Opcode::Fmul => {
                write!(
                    f,
                    "{} R{}, R{}, R{}",
                    self.opcode.mnemonic(),
                    self.rd,
                    self.rs1,
                    self.rs2
                )
            }
            Opcode::Ffma => {
                write!(
                    f,
                    "{} R{}, R{}, R{}, R{}",
                    self.opcode.mnemonic(),
                    self.rd,
                    self.rs1,
                    self.rs2,
                    self.rs3
                )
            }
            Opcode::Fneg | Opcode::Fabs => {
                write!(
                    f,
                    "{} R{}, R{}",
                    self.opcode.mnemonic(),
                    self.rd,
                    self.rs1
                )
            }
            Opcode::Load => {
                write!(
                    f,
                    "{} R{}, [R{}+{}]",
                    self.opcode.mnemonic(),
                    self.rd,
                    self.rs1,
                    self.immediate
                )
            }
            Opcode::Store => {
                write!(
                    f,
                    "{} [R{}+{}], R{}",
                    self.opcode.mnemonic(),
                    self.rs1,
                    self.immediate,
                    self.rs2
                )
            }
            Opcode::Mov => {
                write!(
                    f,
                    "{} R{}, R{}",
                    self.opcode.mnemonic(),
                    self.rd,
                    self.rs1
                )
            }
            Opcode::Limm => {
                write!(
                    f,
                    "{} R{}, {}",
                    self.opcode.mnemonic(),
                    self.rd,
                    self.immediate
                )
            }
            Opcode::Beq | Opcode::Blt | Opcode::Bne => {
                let sign = if self.immediate >= 0.0 { "+" } else { "" };
                write!(
                    f,
                    "{} R{}, R{}, {}{}",
                    self.opcode.mnemonic(),
                    self.rs1,
                    self.rs2,
                    sign,
                    self.immediate as i64
                )
            }
            Opcode::Jmp => {
                write!(f, "{} {}", self.opcode.mnemonic(), self.immediate as i64)
            }
            Opcode::Nop => write!(f, "NOP"),
            Opcode::Halt => write!(f, "HALT"),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper constructors -- make programs readable
// ---------------------------------------------------------------------------

/// FADD Rd, Rs1, Rs2 -- floating-point addition: Rd = Rs1 + Rs2.
pub fn fadd(rd: usize, rs1: usize, rs2: usize) -> Instruction {
    Instruction {
        opcode: Opcode::Fadd,
        rd,
        rs1,
        rs2,
        ..Default::default()
    }
}

/// FSUB Rd, Rs1, Rs2 -- floating-point subtraction: Rd = Rs1 - Rs2.
pub fn fsub(rd: usize, rs1: usize, rs2: usize) -> Instruction {
    Instruction {
        opcode: Opcode::Fsub,
        rd,
        rs1,
        rs2,
        ..Default::default()
    }
}

/// FMUL Rd, Rs1, Rs2 -- floating-point multiplication: Rd = Rs1 * Rs2.
pub fn fmul(rd: usize, rs1: usize, rs2: usize) -> Instruction {
    Instruction {
        opcode: Opcode::Fmul,
        rd,
        rs1,
        rs2,
        ..Default::default()
    }
}

/// FFMA Rd, Rs1, Rs2, Rs3 -- fused multiply-add: Rd = Rs1 * Rs2 + Rs3.
pub fn ffma(rd: usize, rs1: usize, rs2: usize, rs3: usize) -> Instruction {
    Instruction {
        opcode: Opcode::Ffma,
        rd,
        rs1,
        rs2,
        rs3,
        ..Default::default()
    }
}

/// FNEG Rd, Rs1 -- negate: Rd = -Rs1.
pub fn fneg(rd: usize, rs1: usize) -> Instruction {
    Instruction {
        opcode: Opcode::Fneg,
        rd,
        rs1,
        ..Default::default()
    }
}

/// FABS Rd, Rs1 -- absolute value: Rd = |Rs1|.
pub fn fabs(rd: usize, rs1: usize) -> Instruction {
    Instruction {
        opcode: Opcode::Fabs,
        rd,
        rs1,
        ..Default::default()
    }
}

/// LOAD Rd, [Rs1+offset] -- load float from memory into register.
pub fn load(rd: usize, rs1: usize, offset: f64) -> Instruction {
    Instruction {
        opcode: Opcode::Load,
        rd,
        rs1,
        immediate: offset,
        ..Default::default()
    }
}

/// STORE [Rs1+offset], Rs2 -- store register value to memory.
pub fn store(rs1: usize, rs2: usize, offset: f64) -> Instruction {
    Instruction {
        opcode: Opcode::Store,
        rs1,
        rs2,
        immediate: offset,
        ..Default::default()
    }
}

/// MOV Rd, Rs1 -- copy register: Rd = Rs1.
pub fn mov(rd: usize, rs1: usize) -> Instruction {
    Instruction {
        opcode: Opcode::Mov,
        rd,
        rs1,
        ..Default::default()
    }
}

/// LIMM Rd, value -- load immediate float: Rd = value.
pub fn limm(rd: usize, value: f64) -> Instruction {
    Instruction {
        opcode: Opcode::Limm,
        rd,
        immediate: value,
        ..Default::default()
    }
}

/// BEQ Rs1, Rs2, offset -- branch if equal.
pub fn beq(rs1: usize, rs2: usize, offset: i64) -> Instruction {
    Instruction {
        opcode: Opcode::Beq,
        rs1,
        rs2,
        immediate: offset as f64,
        ..Default::default()
    }
}

/// BLT Rs1, Rs2, offset -- branch if less than.
pub fn blt(rs1: usize, rs2: usize, offset: i64) -> Instruction {
    Instruction {
        opcode: Opcode::Blt,
        rs1,
        rs2,
        immediate: offset as f64,
        ..Default::default()
    }
}

/// BNE Rs1, Rs2, offset -- branch if not equal.
pub fn bne(rs1: usize, rs2: usize, offset: i64) -> Instruction {
    Instruction {
        opcode: Opcode::Bne,
        rs1,
        rs2,
        immediate: offset as f64,
        ..Default::default()
    }
}

/// JMP target -- unconditional jump to absolute address.
pub fn jmp(target: i64) -> Instruction {
    Instruction {
        opcode: Opcode::Jmp,
        immediate: target as f64,
        ..Default::default()
    }
}

/// NOP -- no operation, advance program counter.
pub fn nop() -> Instruction {
    Instruction {
        opcode: Opcode::Nop,
        ..Default::default()
    }
}

/// HALT -- stop execution.
pub fn halt() -> Instruction {
    Instruction {
        opcode: Opcode::Halt,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_mnemonics() {
        assert_eq!(Opcode::Fadd.mnemonic(), "FADD");
        assert_eq!(Opcode::Halt.mnemonic(), "HALT");
        assert_eq!(Opcode::Limm.mnemonic(), "LIMM");
        assert_eq!(Opcode::Ffma.mnemonic(), "FFMA");
    }

    #[test]
    fn test_instruction_display() {
        assert_eq!(format!("{}", fadd(2, 0, 1)), "FADD R2, R0, R1");
        assert_eq!(format!("{}", fsub(3, 1, 2)), "FSUB R3, R1, R2");
        assert_eq!(format!("{}", fmul(4, 2, 3)), "FMUL R4, R2, R3");
        assert_eq!(format!("{}", ffma(5, 1, 2, 3)), "FFMA R5, R1, R2, R3");
        assert_eq!(format!("{}", fneg(1, 0)), "FNEG R1, R0");
        assert_eq!(format!("{}", fabs(1, 0)), "FABS R1, R0");
        assert_eq!(format!("{}", load(0, 1, 4.0)), "LOAD R0, [R1+4]");
        assert_eq!(format!("{}", store(1, 2, 0.0)), "STORE [R1+0], R2");
        assert_eq!(format!("{}", mov(1, 0)), "MOV R1, R0");
        assert_eq!(format!("{}", limm(0, 3.14)), "LIMM R0, 3.14");
        assert_eq!(format!("{}", beq(0, 1, 3)), "BEQ R0, R1, +3");
        assert_eq!(format!("{}", blt(0, 1, -2)), "BLT R0, R1, -2");
        assert_eq!(format!("{}", bne(0, 1, 5)), "BNE R0, R1, +5");
        assert_eq!(format!("{}", jmp(10)), "JMP 10");
        assert_eq!(format!("{}", nop()), "NOP");
        assert_eq!(format!("{}", halt()), "HALT");
    }

    #[test]
    fn test_helper_constructors() {
        let inst = fadd(2, 0, 1);
        assert_eq!(inst.opcode, Opcode::Fadd);
        assert_eq!(inst.rd, 2);
        assert_eq!(inst.rs1, 0);
        assert_eq!(inst.rs2, 1);

        let inst = ffma(5, 1, 2, 3);
        assert_eq!(inst.opcode, Opcode::Ffma);
        assert_eq!(inst.rs3, 3);

        let inst = limm(0, 42.0);
        assert_eq!(inst.opcode, Opcode::Limm);
        assert_eq!(inst.immediate, 42.0);

        let inst = halt();
        assert_eq!(inst.opcode, Opcode::Halt);
    }

    #[test]
    fn test_instruction_default() {
        let inst = Instruction::default();
        assert_eq!(inst.opcode, Opcode::Nop);
        assert_eq!(inst.rd, 0);
        assert_eq!(inst.rs1, 0);
        assert_eq!(inst.rs2, 0);
        assert_eq!(inst.rs3, 0);
        assert_eq!(inst.immediate, 0.0);
    }

    #[test]
    fn test_opcode_equality() {
        assert_eq!(Opcode::Fadd, Opcode::Fadd);
        assert_ne!(Opcode::Fadd, Opcode::Fsub);
    }

    #[test]
    fn test_instruction_clone() {
        let inst = fadd(2, 0, 1);
        let cloned = inst;
        assert_eq!(inst, cloned);
    }
}
