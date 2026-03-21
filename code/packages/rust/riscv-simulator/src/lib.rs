//! # RISC-V RV32I Simulator — a clean, modern instruction set.
//!
//! ## What is RISC-V?
//!
//! RISC-V (pronounced "risk-five") is an open-source instruction set architecture (ISA).
//! Unlike the highly complex x86 architecture (CISC), RISC-V is built on the philosophy
//! of a Reduced Instruction Set Computer -- the idea that a CPU should have a small number
//! of simple instructions rather than many complex ones.
//!
//! ## Register conventions
//!
//! RISC-V has 32 registers, each 32 bits wide. The most important quirk is:
//!
//! ```text
//!     x0 = always 0 (hardwired -- writes are ignored, reads always return 0)
//! ```
//!
//! Because x0 is always 0, it enables clever pseudo-instructions without
//! dedicated opcodes:
//!
//! ```text
//!     addi x1, x0, 42   =>   x1 = 0 + 42 = 42   (effectively "load 42 into x1")
//! ```
//!
//! ## Supported instructions
//!
//! This simulator implements a subset of RV32I:
//!
//! | Instruction | Type   | Encoding                                          |
//! |-------------|--------|---------------------------------------------------|
//! | `addi`      | I-type | `[imm[11:0] | rs1 | funct3 | rd | 0010011]`      |
//! | `add`       | R-type | `[funct7 | rs2 | rs1 | funct3 | rd | 0110011]`   |
//! | `sub`       | R-type | `[0100000 | rs2 | rs1 | funct3 | rd | 0110011]`  |
//! | `ecall`     | System | `[000...0 | 1110011]`                             |
//!
//! ## Architecture bridge
//!
//! This simulator bridges the gap between binary-encoded bits and the generic
//! fetch-decode-execute cycle of our CPU base. It implements the two CPU traits:
//!
//! - [`RiscVDecoder`] implements `InstructionDecoder` -- translates bits to fields
//! - [`RiscVExecutor`] implements `InstructionExecutor` -- applies operations

use std::collections::HashMap;
use cpu_simulator::{
    CPU, DecodeResult, ExecuteResult, InstructionDecoder, InstructionExecutor, Memory,
    PipelineTrace, RegisterFile,
};

// ===========================================================================
// Opcode constants
// ===========================================================================

/// RISC-V opcodes are always in the lower 7 bits of the instruction word: bits [6:0].
///
/// ```text
///     31                                      7  6     0
///     +--------------------------------------+----------+
///     |       (instruction-specific fields)   |  opcode  |
///     +--------------------------------------+----------+
/// ```

/// I-type arithmetic with immediate (e.g., `addi x1, x0, 42`).
const OPCODE_OP_IMM: u32 = 0b0010011;

/// R-type register-to-register arithmetic (e.g., `add x3, x1, x2`).
const OPCODE_OP: u32 = 0b0110011;

/// System instructions (e.g., `ecall` to halt or invoke the OS).
const OPCODE_SYSTEM: u32 = 0b1110011;

// ===========================================================================
// Decoder
// ===========================================================================

/// Translates raw 32-bit instruction words into structured decode results.
///
/// The decoder's job is pure pattern matching -- it extracts bit fields
/// from the instruction word and names them. It doesn't modify any state.
pub struct RiscVDecoder;

impl InstructionDecoder for RiscVDecoder {
    /// Determine the instruction type by looking at the opcode bits.
    ///
    /// The opcode is always in bits [6:0]. Based on this 7-bit field,
    /// we dispatch to the appropriate format-specific decoder.
    fn decode(&self, raw: u32, _pc: usize) -> DecodeResult {
        let opcode = raw & 0x7F;
        match opcode {
            OPCODE_OP_IMM => self.decode_i_type(raw, "addi"),
            OPCODE_OP => self.decode_r_type(raw),
            OPCODE_SYSTEM => DecodeResult {
                mnemonic: "ecall".to_string(),
                fields: HashMap::from([("opcode".to_string(), opcode as i32)]),
                raw_instruction: raw,
            },
            _ => DecodeResult {
                mnemonic: format!("UNKNOWN(0x{:02x})", opcode),
                fields: HashMap::from([("opcode".to_string(), opcode as i32)]),
                raw_instruction: raw,
            },
        }
    }
}

impl RiscVDecoder {
    /// Decode an R-type (Register-to-Register) instruction.
    ///
    /// R-type format (32 bits total):
    ///
    /// ```text
    ///     31      25  24  20  19  15  14  12  11   7  6    0
    ///     +---------+------+------+-------+------+--------+
    ///     | funct7  | rs2  | rs1  | funct3|  rd  | opcode |
    ///     | 7 bits  |5 bits|5 bits|3 bits |5 bits| 7 bits |
    ///     +---------+------+------+-------+------+--------+
    /// ```
    ///
    /// The `funct7` and `funct3` fields together determine which operation
    /// to perform. For example:
    ///
    /// - `funct7=0, funct3=0` => `add`
    /// - `funct7=0x20, funct3=0` => `sub`
    fn decode_r_type(&self, raw: u32) -> DecodeResult {
        let rd = ((raw >> 7) & 0x1F) as i32;
        let funct3 = ((raw >> 12) & 0x7) as i32;
        let rs1 = ((raw >> 15) & 0x1F) as i32;
        let rs2 = ((raw >> 20) & 0x1F) as i32;
        let funct7 = ((raw >> 25) & 0x7F) as i32;

        let mnemonic = match (funct3, funct7) {
            (0, 0) => "add".to_string(),
            (0, 0x20) => "sub".to_string(),
            _ => format!("r_op(f3={},f7={})", funct3, funct7),
        };

        DecodeResult {
            mnemonic,
            fields: HashMap::from([
                ("rd".to_string(), rd),
                ("rs1".to_string(), rs1),
                ("rs2".to_string(), rs2),
                ("funct3".to_string(), funct3),
                ("funct7".to_string(), funct7),
            ]),
            raw_instruction: raw,
        }
    }

    /// Decode an I-type (Immediate) instruction.
    ///
    /// I-type format (32 bits total):
    ///
    /// ```text
    ///     31         20  19  15  14  12  11   7  6    0
    ///     +------------+------+-------+------+--------+
    ///     | imm[11:0]  | rs1  | funct3|  rd  | opcode |
    ///     | 12 bits    |5 bits|3 bits |5 bits| 7 bits |
    ///     +------------+------+-------+------+--------+
    /// ```
    ///
    /// The 12-bit immediate is sign-extended to 32 bits. This means if
    /// bit 11 is set (the value is negative in two's complement), we
    /// extend the sign bit across the upper 20 bits.
    ///
    /// Example of sign extension:
    ///
    /// ```text
    ///     12-bit: 1111_1111_1011  (= -5 in 12-bit two's complement)
    ///     32-bit: 1111_1111_1111_1111_1111_1111_1111_1011  (= -5 as i32)
    /// ```
    fn decode_i_type(&self, raw: u32, default_mnemonic: &str) -> DecodeResult {
        let rd = ((raw >> 7) & 0x1F) as i32;
        let funct3 = ((raw >> 12) & 0x7) as i32;
        let rs1 = ((raw >> 15) & 0x1F) as i32;
        let mut imm = ((raw >> 20) & 0xFFF) as i32;

        // Sign-extend the 12-bit immediate to 32 bits.
        // The MSB of a 12-bit number is bit 11 (0x800).
        // If it's set, the value is negative, so we subtract 2^12 (0x1000)
        // to convert from unsigned 12-bit to signed 32-bit.
        if imm & 0x800 != 0 {
            imm -= 0x1000;
        }

        DecodeResult {
            mnemonic: default_mnemonic.to_string(),
            fields: HashMap::from([
                ("rd".to_string(), rd),
                ("rs1".to_string(), rs1),
                ("imm".to_string(), imm),
                ("funct3".to_string(), funct3),
            ]),
            raw_instruction: raw,
        }
    }
}

// ===========================================================================
// Executor
// ===========================================================================

/// Applies decoded RISC-V instructions to registers and memory.
///
/// The executor is where state changes happen. Given a decoded instruction,
/// it reads source registers, computes the result, and writes back to the
/// destination register.
pub struct RiscVExecutor;

impl InstructionExecutor for RiscVExecutor {
    fn execute(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        _memory: &mut Memory,
        pc: usize,
    ) -> ExecuteResult {
        match decoded.mnemonic.as_str() {
            "addi" => self.exec_addi(decoded, registers, pc),
            "add" => self.exec_add(decoded, registers, pc),
            "sub" => self.exec_sub(decoded, registers, pc),
            "ecall" => {
                // ecall halts our simple CPU.
                ExecuteResult {
                    description: "System call (halt)".to_string(),
                    registers_changed: HashMap::new(),
                    memory_changed: HashMap::new(),
                    next_pc: pc,
                    halted: true,
                }
            }
            other => ExecuteResult {
                description: format!("Unknown instruction: {}", other),
                registers_changed: HashMap::new(),
                memory_changed: HashMap::new(),
                next_pc: pc + 4,
                halted: false,
            },
        }
    }
}

impl RiscVExecutor {
    /// Execute `addi rd, rs1, imm` -- add immediate to register.
    ///
    /// ```text
    ///     rd = rs1 + sign_extended(imm)
    /// ```
    ///
    /// Special case: if rd is x0, the write is silently discarded
    /// (x0 is hardwired to zero in RISC-V).
    fn exec_addi(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        pc: usize,
    ) -> ExecuteResult {
        let rd = decoded.fields["rd"] as usize;
        let rs1 = decoded.fields["rs1"] as usize;
        let imm = decoded.fields["imm"];

        let rs1_val = registers.read(rs1) as i32;
        let result = (rs1_val.wrapping_add(imm)) as u32;

        let mut changes = HashMap::new();
        if rd != 0 {
            registers.write(rd, result);
            changes.insert(format!("x{}", rd), result);
        }

        ExecuteResult {
            description: format!(
                "x{} = x{}({}) + {} = {}",
                rd, rs1, rs1_val, imm, result as i32
            ),
            registers_changed: changes,
            memory_changed: HashMap::new(),
            next_pc: pc + 4,
            halted: false,
        }
    }

    /// Execute `add rd, rs1, rs2` -- add two registers.
    ///
    /// ```text
    ///     rd = rs1 + rs2
    /// ```
    fn exec_add(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        pc: usize,
    ) -> ExecuteResult {
        let rd = decoded.fields["rd"] as usize;
        let rs1 = decoded.fields["rs1"] as usize;
        let rs2 = decoded.fields["rs2"] as usize;

        let rs1_val = registers.read(rs1) as i32;
        let rs2_val = registers.read(rs2) as i32;
        let result = (rs1_val.wrapping_add(rs2_val)) as u32;

        let mut changes = HashMap::new();
        // x0 is hardwired to zero -- intercept writes here.
        if rd != 0 {
            registers.write(rd, result);
            changes.insert(format!("x{}", rd), result);
        }

        ExecuteResult {
            description: format!(
                "x{} = x{}({}) + x{}({}) = {}",
                rd, rs1, rs1_val, rs2, rs2_val, result as i32
            ),
            registers_changed: changes,
            memory_changed: HashMap::new(),
            next_pc: pc + 4,
            halted: false,
        }
    }

    /// Execute `sub rd, rs1, rs2` -- subtract two registers.
    ///
    /// ```text
    ///     rd = rs1 - rs2
    /// ```
    fn exec_sub(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        pc: usize,
    ) -> ExecuteResult {
        let rd = decoded.fields["rd"] as usize;
        let rs1 = decoded.fields["rs1"] as usize;
        let rs2 = decoded.fields["rs2"] as usize;

        let rs1_val = registers.read(rs1) as i32;
        let rs2_val = registers.read(rs2) as i32;
        let result = (rs1_val.wrapping_sub(rs2_val)) as u32;

        let mut changes = HashMap::new();
        if rd != 0 {
            registers.write(rd, result);
            changes.insert(format!("x{}", rd), result);
        }

        ExecuteResult {
            description: format!(
                "x{} = x{}({}) - x{}({}) = {}",
                rd, rs1, rs1_val, rs2, rs2_val, result as i32
            ),
            registers_changed: changes,
            memory_changed: HashMap::new(),
            next_pc: pc + 4,
            halted: false,
        }
    }
}

// ===========================================================================
// Simulator
// ===========================================================================

/// The full RISC-V simulation environment.
///
/// Wraps a generic CPU with RISC-V-specific decoder and executor,
/// providing a clean interface for loading and running programs.
pub struct RiscVSimulator {
    pub cpu: CPU,
}

impl RiscVSimulator {
    /// Create a new RISC-V simulator with the given memory size.
    ///
    /// Sets up:
    /// - 32 registers (x0-x31), each 32 bits wide
    /// - Memory of the specified size
    /// - RISC-V decoder and executor
    pub fn new(memory_size: usize) -> Self {
        RiscVSimulator {
            cpu: CPU::new(
                Box::new(RiscVDecoder),
                Box::new(RiscVExecutor),
                32, // 32 registers (x0-x31)
                32, // 32-bit register width
                memory_size,
            ),
        }
    }

    /// Load and run a program to completion.
    ///
    /// Returns the full pipeline trace -- one entry per instruction executed.
    pub fn run(&mut self, program: &[u8]) -> Vec<PipelineTrace> {
        self.cpu.load_program(program, 0);
        self.cpu.run(10000)
    }

    /// Execute a single instruction and return its pipeline trace.
    pub fn step(&mut self) -> PipelineTrace {
        self.cpu.step()
    }
}

// ===========================================================================
// Encoding helpers
// ===========================================================================
// These functions create machine code bytes for testing. In a real system,
// an assembler would do this. Here, we encode instructions directly so
// our tests don't depend on an assembler implementation.

/// Encode `addi rd, rs1, imm` as a 32-bit instruction word.
///
/// I-type encoding:
/// ```text
///     [imm[11:0] | rs1 | funct3=000 | rd | opcode=0010011]
/// ```
pub fn encode_addi(rd: usize, rs1: usize, imm: i32) -> u32 {
    let imm_bits = (imm & 0xFFF) as u32;
    (imm_bits << 20) | ((rs1 as u32) << 15) | (0 << 12) | ((rd as u32) << 7) | OPCODE_OP_IMM
}

/// Encode `add rd, rs1, rs2` as a 32-bit instruction word.
///
/// R-type encoding with funct7=0000000:
/// ```text
///     [0000000 | rs2 | rs1 | funct3=000 | rd | opcode=0110011]
/// ```
pub fn encode_add(rd: usize, rs1: usize, rs2: usize) -> u32 {
    (0 << 25)
        | ((rs2 as u32) << 20)
        | ((rs1 as u32) << 15)
        | (0 << 12)
        | ((rd as u32) << 7)
        | OPCODE_OP
}

/// Encode `sub rd, rs1, rs2` as a 32-bit instruction word.
///
/// R-type encoding with funct7=0100000:
/// ```text
///     [0100000 | rs2 | rs1 | funct3=000 | rd | opcode=0110011]
/// ```
pub fn encode_sub(rd: usize, rs1: usize, rs2: usize) -> u32 {
    (0x20 << 25)
        | ((rs2 as u32) << 20)
        | ((rs1 as u32) << 15)
        | (0 << 12)
        | ((rd as u32) << 7)
        | OPCODE_OP
}

/// Encode `ecall` as a 32-bit instruction word.
///
/// System encoding: all zeros except the opcode field.
pub fn encode_ecall() -> u32 {
    OPCODE_SYSTEM
}

/// Assemble a sequence of 32-bit instruction words into little-endian bytes.
///
/// RISC-V uses little-endian byte ordering, meaning the least significant
/// byte of each instruction comes first in memory:
///
/// ```text
///     Instruction: 0x12345678
///     Memory:      [0x78, 0x56, 0x34, 0x12]
///                   ^^^^                ^^^^
///                   LSB                 MSB
/// ```
pub fn assemble(instructions: &[u32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(instructions.len() * 4);
    for &inst in instructions {
        result.push((inst & 0xFF) as u8);
        result.push(((inst >> 8) & 0xFF) as u8);
        result.push(((inst >> 16) & 0xFF) as u8);
        result.push(((inst >> 24) & 0xFF) as u8);
    }
    result
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test a complete program: x1=1, x2=2, x3=x1+x2=3, x4=x3-x1=2.
    #[test]
    fn simulator_runs_full_program() {
        let mut sim = RiscVSimulator::new(65536);
        let program = assemble(&[
            encode_addi(1, 0, 1),  // x1 = 0 + 1 = 1
            encode_addi(2, 0, 2),  // x2 = 0 + 2 = 2
            encode_add(3, 1, 2),   // x3 = x1 + x2 = 3
            encode_sub(4, 3, 1),   // x4 = x3 - x1 = 2
            encode_ecall(),        // halt
        ]);

        let traces = sim.run(&program);
        assert_eq!(traces.len(), 5);

        assert_eq!(sim.cpu.registers.read(1), 1);
        assert_eq!(sim.cpu.registers.read(3), 3);
        assert_eq!(sim.cpu.registers.read(4), 2);
    }

    /// x0 must be hardwired to zero -- writes should be silently discarded.
    #[test]
    fn register_zero_is_hardwired() {
        let mut sim = RiscVSimulator::new(1024);
        let program = assemble(&[
            encode_addi(0, 0, 42), // attempt to write 42 to x0
            encode_ecall(),
        ]);
        sim.run(&program);
        assert_eq!(
            sim.cpu.registers.read(0),
            0,
            "x0 must always read as zero"
        );
    }

    /// Negative immediate values must be correctly sign-extended.
    #[test]
    fn negative_immediate_decoding() {
        let mut sim = RiscVSimulator::new(1024);
        let program = assemble(&[
            encode_addi(1, 0, -5), // x1 = 0 + (-5) = -5
            encode_ecall(),
        ]);
        sim.run(&program);
        let val = sim.cpu.registers.read(1) as i32;
        assert_eq!(val, -5, "Negative immediate should decode correctly");
    }

    /// Unknown instructions should produce a safe fallback description.
    #[test]
    fn unknown_instruction_handled_gracefully() {
        let mut sim = RiscVSimulator::new(1024);
        let program = assemble(&[
            0xFFFFFFFF, // invalid instruction
            encode_ecall(),
        ]);
        let traces = sim.run(&program);
        assert!(
            !traces[0].execute.description.is_empty(),
            "Unknown instruction should have a fallback description"
        );
    }

    /// Verify encoding helpers produce correct bit patterns.
    #[test]
    fn encoding_round_trip() {
        let decoder = RiscVDecoder;

        // addi x1, x0, 42
        let raw = encode_addi(1, 0, 42);
        let decoded = decoder.decode(raw, 0);
        assert_eq!(decoded.mnemonic, "addi");
        assert_eq!(decoded.fields["rd"], 1);
        assert_eq!(decoded.fields["rs1"], 0);
        assert_eq!(decoded.fields["imm"], 42);

        // add x3, x1, x2
        let raw = encode_add(3, 1, 2);
        let decoded = decoder.decode(raw, 0);
        assert_eq!(decoded.mnemonic, "add");
        assert_eq!(decoded.fields["rd"], 3);

        // sub x4, x3, x1
        let raw = encode_sub(4, 3, 1);
        let decoded = decoder.decode(raw, 0);
        assert_eq!(decoded.mnemonic, "sub");
        assert_eq!(decoded.fields["rd"], 4);
    }

    /// Verify step() can be called individually.
    #[test]
    fn step_returns_single_trace() {
        let mut sim = RiscVSimulator::new(1024);
        let program = assemble(&[
            encode_addi(1, 0, 10),
            encode_ecall(),
        ]);
        sim.cpu.load_program(&program, 0);

        let trace = sim.step();
        assert_eq!(trace.decode.mnemonic, "addi");
        assert_eq!(sim.cpu.registers.read(1), 10);
    }
}
