//! ISADecoder -- the interface between the Core and any instruction set.
//!
//! # Why a Trait?
//!
//! The Core knows how to move instructions through a pipeline, predict
//! branches, detect hazards, and access caches. But it does NOT know what
//! any instruction means. That is the ISA decoder's job.
//!
//! This separation mirrors real CPU design:
//!   - ARM defines the decoder semantics (what ADD, LDR, BEQ mean)
//!   - Apple/Qualcomm build the pipeline and caches
//!   - The decoder plugs into the pipeline via a well-defined interface
//!
//! The `ISADecoder` trait is that well-defined interface. Any ISA
//! (ARM, RISC-V, x86, or a custom teaching ISA) can implement it and
//! immediately run on any Core configuration.
//!
//! # The Three Methods
//!
//! The decoder has exactly three responsibilities:
//!
//!  1. `decode`: turn raw instruction bits into a structured PipelineToken
//!  2. `execute`: perform the actual computation (ALU, branch resolution)
//!  3. `instruction_size`: how many bytes per instruction
//!
//! These map directly to the ID and EX stages of the pipeline.

use cpu_pipeline::PipelineToken;

use crate::register_file::RegisterFile;

/// Protocol that any instruction set architecture (ISA) must implement
/// to plug into a Core.
///
/// ```text
///   IF stage:  fetch raw bits from memory
///   ID stage:  decoder.decode(raw, token) -> fills in decoded fields
///   EX stage:  decoder.execute(token, reg_file) -> computes ALU result
///   MEM stage: core handles cache access
///   WB stage:  core handles register writeback
/// ```
pub trait ISADecoder {
    /// Turns raw instruction bits into a structured PipelineToken.
    ///
    /// The decoder fills in:
    ///   - opcode (string name like "ADD", "LDR", "BEQ")
    ///   - rs1, rs2 (source register numbers, -1 if unused)
    ///   - rd (destination register number, -1 if unused)
    ///   - immediate (sign-extended immediate value)
    ///   - Control signals: reg_write, mem_read, mem_write, is_branch, is_halt
    fn decode(&self, raw_instruction: i64, token: PipelineToken) -> PipelineToken;

    /// Performs the ALU operation for a decoded instruction.
    ///
    /// Fills in:
    ///   - alu_result (computed value, or effective address for loads/stores)
    ///   - branch_taken (was the branch actually taken?)
    ///   - branch_target (where does the branch go?)
    ///   - write_data (final value to write to Rd, if reg_write is true)
    fn execute(&self, token: PipelineToken, reg_file: &RegisterFile) -> PipelineToken;

    /// Returns the size of one instruction in bytes.
    ///
    /// Determines how much the PC advances after each fetch:
    ///   - ARM (A64): 4 bytes (fixed-width 32-bit instructions)
    ///   - RISC-V:    4 bytes (base ISA) or 2 bytes (compressed)
    ///   - x86:       variable (1-15 bytes)
    fn instruction_size(&self) -> i64;
}

// =========================================================================
// MockDecoder -- a simple decoder for testing the Core
// =========================================================================

/// A minimal ISA decoder for testing purposes.
///
/// Supports a handful of instructions encoded in a simple format:
///
/// ```text
///   Bits 31-24: opcode (0=NOP, 1=ADD, 2=LOAD, 3=STORE, 4=BRANCH, 5=HALT,
///                        6=ADDI, 7=SUB)
///   Bits 23-20: Rd  (destination register)
///   Bits 19-16: Rs1 (first source register)
///   Bits 15-12: Rs2 (second source register)
///   Bits 11-0:  immediate (12-bit, sign-extended)
/// ```
///
/// # Instruction Reference
///
/// ```text
///   NOP    (0x00): Do nothing. Occupies a pipeline slot but has no effect.
///   ADD    (0x01): Rd = Rs1 + Rs2
///   LOAD   (0x02): Rd = Memory[Rs1 + imm]  (word load)
///   STORE  (0x03): Memory[Rs1 + imm] = Rs2  (word store)
///   BRANCH (0x04): If Rs1 == Rs2, PC = PC + imm (conditional branch)
///   HALT   (0x05): Stop execution.
///   ADDI   (0x06): Rd = Rs1 + imm
///   SUB    (0x07): Rd = Rs1 - Rs2
/// ```
pub struct MockDecoder;

impl MockDecoder {
    /// Creates a new MockDecoder.
    pub fn new() -> Self {
        MockDecoder
    }
}

impl Default for MockDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ISADecoder for MockDecoder {
    fn instruction_size(&self) -> i64 {
        4
    }

    /// Extracts fields from a raw 32-bit instruction and fills in the token.
    ///
    /// ```text
    ///   31      24 23    20 19    16 15    12 11           0
    ///  +----------+--------+--------+--------+--------------+
    ///  |  opcode  |   Rd   |  Rs1   |  Rs2   |  immediate   |
    ///  +----------+--------+--------+--------+--------------+
    /// ```
    fn decode(&self, raw: i64, mut token: PipelineToken) -> PipelineToken {
        // Extract fields using bit masking and shifting.
        let opcode = (raw >> 24) & 0xFF;
        let rd = (raw >> 20) & 0x0F;
        let rs1 = (raw >> 16) & 0x0F;
        let rs2 = (raw >> 12) & 0x0F;
        let mut imm = raw & 0xFFF;

        // Sign-extend the 12-bit immediate to a full i64.
        // If bit 11 is set, the value is negative.
        if imm & 0x800 != 0 {
            imm |= !0xFFF; // sign-extend by filling upper bits with 1s
        }

        match opcode {
            0x00 => {
                // NOP
                token.opcode = "NOP".to_string();
                token.rd = -1;
                token.rs1 = -1;
                token.rs2 = -1;
            }
            0x01 => {
                // ADD Rd, Rs1, Rs2
                token.opcode = "ADD".to_string();
                token.rd = rd;
                token.rs1 = rs1;
                token.rs2 = rs2;
                token.reg_write = true;
            }
            0x02 => {
                // LOAD Rd, [Rs1 + imm]
                token.opcode = "LOAD".to_string();
                token.rd = rd;
                token.rs1 = rs1;
                token.rs2 = -1;
                token.immediate = imm;
                token.reg_write = true;
                token.mem_read = true;
            }
            0x03 => {
                // STORE [Rs1 + imm], Rs2
                token.opcode = "STORE".to_string();
                token.rd = -1;
                token.rs1 = rs1;
                token.rs2 = rs2;
                token.immediate = imm;
                token.mem_write = true;
            }
            0x04 => {
                // BRANCH Rs1, Rs2, imm
                token.opcode = "BRANCH".to_string();
                token.rd = -1;
                token.rs1 = rs1;
                token.rs2 = rs2;
                token.immediate = imm;
                token.is_branch = true;
            }
            0x05 => {
                // HALT
                token.opcode = "HALT".to_string();
                token.rd = -1;
                token.rs1 = -1;
                token.rs2 = -1;
                token.is_halt = true;
            }
            0x06 => {
                // ADDI Rd, Rs1, imm
                token.opcode = "ADDI".to_string();
                token.rd = rd;
                token.rs1 = rs1;
                token.rs2 = -1;
                token.immediate = imm;
                token.reg_write = true;
            }
            0x07 => {
                // SUB Rd, Rs1, Rs2
                token.opcode = "SUB".to_string();
                token.rd = rd;
                token.rs1 = rs1;
                token.rs2 = rs2;
                token.reg_write = true;
            }
            _ => {
                // Unknown opcode -- treat as NOP.
                token.opcode = "NOP".to_string();
                token.rd = -1;
                token.rs1 = -1;
                token.rs2 = -1;
            }
        }

        token
    }

    /// Performs the ALU operation for a decoded instruction.
    ///
    /// Reads register values, computes the result, and fills in
    /// alu_result, branch_taken, branch_target, and write_data.
    fn execute(&self, mut token: PipelineToken, reg_file: &RegisterFile) -> PipelineToken {
        // Read source register values.
        let rs1_val = if token.rs1 >= 0 {
            reg_file.read(token.rs1)
        } else {
            0
        };
        let rs2_val = if token.rs2 >= 0 {
            reg_file.read(token.rs2)
        } else {
            0
        };

        match token.opcode.as_str() {
            "ADD" => {
                token.alu_result = rs1_val + rs2_val;
                token.write_data = token.alu_result;
            }
            "SUB" => {
                token.alu_result = rs1_val - rs2_val;
                token.write_data = token.alu_result;
            }
            "ADDI" => {
                token.alu_result = rs1_val + token.immediate;
                token.write_data = token.alu_result;
            }
            "LOAD" => {
                // Effective address = Rs1 + immediate.
                // Actual memory read happens in the MEM stage (handled by Core).
                token.alu_result = rs1_val + token.immediate;
            }
            "STORE" => {
                // Effective address = Rs1 + immediate.
                // Data to store comes from Rs2.
                token.alu_result = rs1_val + token.immediate;
                token.write_data = rs2_val;
            }
            "BRANCH" => {
                // Branch condition: Rs1 == Rs2
                // Branch target: PC + (immediate * instruction_size)
                let taken = rs1_val == rs2_val;
                token.branch_taken = taken;
                let target = token.pc + (token.immediate * 4);
                token.branch_target = target;
                if taken {
                    token.alu_result = target;
                } else {
                    token.alu_result = token.pc + 4;
                }
            }
            "NOP" | "HALT" => {
                // No computation needed.
            }
            _ => {
                // Unknown opcode -- no computation.
            }
        }

        token
    }
}

// =========================================================================
// MockInstruction -- helpers for encoding mock instructions
// =========================================================================

/// Returns the raw encoding for a NOP instruction.
pub fn encode_nop() -> i64 {
    0x00 << 24
}

/// Returns the raw encoding for ADD Rd, Rs1, Rs2.
pub fn encode_add(rd: i64, rs1: i64, rs2: i64) -> i64 {
    (0x01 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12)
}

/// Returns the raw encoding for SUB Rd, Rs1, Rs2.
pub fn encode_sub(rd: i64, rs1: i64, rs2: i64) -> i64 {
    (0x07 << 24) | (rd << 20) | (rs1 << 16) | (rs2 << 12)
}

/// Returns the raw encoding for ADDI Rd, Rs1, imm.
pub fn encode_addi(rd: i64, rs1: i64, imm: i64) -> i64 {
    (0x06 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xFFF)
}

/// Returns the raw encoding for LOAD Rd, [Rs1 + imm].
pub fn encode_load(rd: i64, rs1: i64, imm: i64) -> i64 {
    (0x02 << 24) | (rd << 20) | (rs1 << 16) | (imm & 0xFFF)
}

/// Returns the raw encoding for STORE [Rs1 + imm], Rs2.
pub fn encode_store(rs1: i64, rs2: i64, imm: i64) -> i64 {
    (0x03 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xFFF)
}

/// Returns the raw encoding for BRANCH Rs1, Rs2, imm.
/// The branch is taken if Rs1 == Rs2, jumping to PC + imm*4.
pub fn encode_branch(rs1: i64, rs2: i64, imm: i64) -> i64 {
    (0x04 << 24) | (rs1 << 16) | (rs2 << 12) | (imm & 0xFFF)
}

/// Returns the raw encoding for a HALT instruction.
pub fn encode_halt() -> i64 {
    0x05 << 24
}

/// Converts a sequence of raw instruction i64s into a byte slice
/// suitable for `LoadProgram`.
///
/// Each instruction is encoded as 4 bytes in little-endian order.
pub fn encode_program(instructions: &[i64]) -> Vec<u8> {
    let mut result = vec![0u8; instructions.len() * 4];
    for (i, &instr) in instructions.iter().enumerate() {
        let offset = i * 4;
        result[offset] = (instr & 0xFF) as u8;
        result[offset + 1] = ((instr >> 8) & 0xFF) as u8;
        result[offset + 2] = ((instr >> 16) & 0xFF) as u8;
        result[offset + 3] = ((instr >> 24) & 0xFF) as u8;
    }
    result
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_nop() {
        let dec = MockDecoder::new();
        let tok = dec.decode(encode_nop(), PipelineToken::new());
        assert_eq!(tok.opcode, "NOP");
        assert_eq!(tok.rd, -1);
        assert_eq!(tok.rs1, -1);
        assert_eq!(tok.rs2, -1);
    }

    #[test]
    fn test_decode_add() {
        let dec = MockDecoder::new();
        let tok = dec.decode(encode_add(1, 2, 3), PipelineToken::new());
        assert_eq!(tok.opcode, "ADD");
        assert_eq!(tok.rd, 1);
        assert_eq!(tok.rs1, 2);
        assert_eq!(tok.rs2, 3);
        assert!(tok.reg_write);
    }

    #[test]
    fn test_decode_sub() {
        let dec = MockDecoder::new();
        let tok = dec.decode(encode_sub(4, 5, 6), PipelineToken::new());
        assert_eq!(tok.opcode, "SUB");
        assert_eq!(tok.rd, 4);
        assert_eq!(tok.rs1, 5);
        assert_eq!(tok.rs2, 6);
        assert!(tok.reg_write);
    }

    #[test]
    fn test_decode_addi() {
        let dec = MockDecoder::new();
        let tok = dec.decode(encode_addi(1, 0, 42), PipelineToken::new());
        assert_eq!(tok.opcode, "ADDI");
        assert_eq!(tok.rd, 1);
        assert_eq!(tok.rs1, 0);
        assert_eq!(tok.rs2, -1);
        assert_eq!(tok.immediate, 42);
        assert!(tok.reg_write);
    }

    #[test]
    fn test_decode_load() {
        let dec = MockDecoder::new();
        let tok = dec.decode(encode_load(3, 1, 8), PipelineToken::new());
        assert_eq!(tok.opcode, "LOAD");
        assert_eq!(tok.rd, 3);
        assert_eq!(tok.rs1, 1);
        assert!(tok.reg_write);
        assert!(tok.mem_read);
        assert_eq!(tok.immediate, 8);
    }

    #[test]
    fn test_decode_store() {
        let dec = MockDecoder::new();
        let tok = dec.decode(encode_store(1, 2, 4), PipelineToken::new());
        assert_eq!(tok.opcode, "STORE");
        assert_eq!(tok.rd, -1);
        assert_eq!(tok.rs1, 1);
        assert_eq!(tok.rs2, 2);
        assert!(tok.mem_write);
    }

    #[test]
    fn test_decode_branch() {
        let dec = MockDecoder::new();
        let tok = dec.decode(encode_branch(1, 2, 3), PipelineToken::new());
        assert_eq!(tok.opcode, "BRANCH");
        assert!(tok.is_branch);
        assert_eq!(tok.rs1, 1);
        assert_eq!(tok.rs2, 2);
        assert_eq!(tok.immediate, 3);
    }

    #[test]
    fn test_decode_halt() {
        let dec = MockDecoder::new();
        let tok = dec.decode(encode_halt(), PipelineToken::new());
        assert_eq!(tok.opcode, "HALT");
        assert!(tok.is_halt);
    }

    #[test]
    fn test_decode_unknown_opcode() {
        let dec = MockDecoder::new();
        let tok = dec.decode(0xFF << 24, PipelineToken::new());
        assert_eq!(tok.opcode, "NOP");
    }

    #[test]
    fn test_execute_add() {
        let dec = MockDecoder::new();
        let mut rf = RegisterFile::new(None);
        rf.write(2, 10);
        rf.write(3, 20);
        let tok = dec.decode(encode_add(1, 2, 3), PipelineToken::new());
        let result = dec.execute(tok, &rf);
        assert_eq!(result.alu_result, 30);
        assert_eq!(result.write_data, 30);
    }

    #[test]
    fn test_execute_sub() {
        let dec = MockDecoder::new();
        let mut rf = RegisterFile::new(None);
        rf.write(5, 100);
        rf.write(6, 30);
        let tok = dec.decode(encode_sub(4, 5, 6), PipelineToken::new());
        let result = dec.execute(tok, &rf);
        assert_eq!(result.alu_result, 70);
    }

    #[test]
    fn test_execute_addi() {
        let dec = MockDecoder::new();
        let rf = RegisterFile::new(None);
        let tok = dec.decode(encode_addi(1, 0, 42), PipelineToken::new());
        let result = dec.execute(tok, &rf);
        assert_eq!(result.alu_result, 42);
        assert_eq!(result.write_data, 42);
    }

    #[test]
    fn test_execute_branch_taken() {
        let dec = MockDecoder::new();
        let mut rf = RegisterFile::new(None);
        rf.write(1, 10);
        rf.write(2, 10); // Rs1 == Rs2, branch taken
        let mut tok = dec.decode(encode_branch(1, 2, 3), PipelineToken::new());
        tok.pc = 100;
        let result = dec.execute(tok, &rf);
        assert!(result.branch_taken);
        assert_eq!(result.branch_target, 100 + 3 * 4);
    }

    #[test]
    fn test_execute_branch_not_taken() {
        let dec = MockDecoder::new();
        let mut rf = RegisterFile::new(None);
        rf.write(1, 10);
        rf.write(2, 20); // Rs1 != Rs2, branch not taken
        let mut tok = dec.decode(encode_branch(1, 2, 3), PipelineToken::new());
        tok.pc = 100;
        let result = dec.execute(tok, &rf);
        assert!(!result.branch_taken);
    }

    #[test]
    fn test_sign_extend_negative_immediate() {
        let dec = MockDecoder::new();
        // Encode ADDI with negative immediate (-1 = 0xFFF in 12-bit)
        let raw = (0x06i64 << 24) | (1 << 20) | (0 << 16) | 0xFFF;
        let tok = dec.decode(raw, PipelineToken::new());
        assert_eq!(tok.immediate, -1);
    }

    #[test]
    fn test_encode_program() {
        let program = encode_program(&[encode_addi(1, 0, 42), encode_halt()]);
        assert_eq!(program.len(), 8);
    }

    #[test]
    fn test_mock_decoder_default() {
        let _dec = MockDecoder::default();
        // Should not panic.
    }

    #[test]
    fn test_instruction_size() {
        let dec = MockDecoder::new();
        assert_eq!(dec.instruction_size(), 4);
    }
}
