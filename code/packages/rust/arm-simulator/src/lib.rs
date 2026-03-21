//! # ARM Simulator — the architecture that powers your phone.
//!
//! ## What is ARM?
//!
//! ARM (originally Acorn RISC Machine) powers your phone, your tablet, and
//! probably your laptop. It was designed in 1985 with a focus on power efficiency.
//!
//! Unlike RISC-V's strict, zero-magic layout, ARM features several unique design
//! quirks. Most notably: conditionally executed instructions. Every instruction
//! has a 4-bit condition code field (bits [31:28]). The CPU checks the condition
//! flags before executing -- if the condition isn't met, the instruction becomes
//! a NOP (no operation).
//!
//! ## Register conventions
//!
//! ARM has 16 registers, each 32 bits wide:
//!
//! ```text
//!     R0-R12   General purpose
//!     R13      SP (Stack Pointer)
//!     R14      LR (Link Register -- return address)
//!     R15      PC (Program Counter -- visible to assembly writers!)
//! ```
//!
//! ## Supported instructions
//!
//! This simulator implements a subset of ARM data processing:
//!
//! | Instruction | Opcode  | Description                         |
//! |-------------|---------|-------------------------------------|
//! | `MOV Rd, #imm` | 0b1101 | Move immediate to register       |
//! | `ADD Rd, Rn, Rm` | 0b0100 | Add two registers              |
//! | `SUB Rd, Rn, Rm` | 0b0010 | Subtract two registers         |
//! | `HLT`       | 0xFFFFFFFF | Halt (custom testing instruction) |
//!
//! ## ARM's rotate trick for immediates
//!
//! ARM encodes immediate values with only 12 bits, but splits them into
//! an 8-bit value and a 4-bit rotation amount. The rotation is multiplied
//! by 2, giving a circular right shift:
//!
//! ```text
//!     immediate = rotate_right(imm8, rotate * 2)
//! ```
//!
//! This clever trick lets ARM represent many useful constants (like powers
//! of 2 and common masks) that wouldn't fit in a plain 12-bit field.

use std::collections::HashMap;
use cpu_simulator::{
    CPU, DecodeResult, ExecuteResult, InstructionDecoder, InstructionExecutor, Memory,
    PipelineTrace, RegisterFile,
};

// ===========================================================================
// Constants
// ===========================================================================

/// Condition code: Always execute (the default for most instructions).
const COND_AL: u32 = 0b1110;

/// Data processing opcodes (bits [24:21] of the instruction word).
const OPCODE_MOV: u32 = 0b1101;
const OPCODE_ADD: u32 = 0b0100;
const OPCODE_SUB: u32 = 0b0010;

/// A sentinel value representing the HLT (halt) instruction.
/// This isn't a real ARM instruction -- it's a convention for our simulator.
const HLT_INSTRUCTION: u32 = 0xFFFFFFFF;

// ===========================================================================
// Decoder
// ===========================================================================

/// Parses 32-bit ARM data processing instructions.
///
/// ARM instruction format (data processing):
///
/// ```text
///     31  28  27 26  25  24  21  20  19  16  15  12  11       0
///     +------+-----+---+------+---+------+------+--------------+
///     | cond | 00  | I | opcode| S |  Rn  |  Rd  |  operand2   |
///     +------+-----+---+------+---+------+------+--------------+
/// ```
pub struct ARMDecoder;

impl InstructionDecoder for ARMDecoder {
    fn decode(&self, raw: u32, _pc: usize) -> DecodeResult {
        // Check for our custom HLT sentinel first.
        if raw == HLT_INSTRUCTION {
            return DecodeResult {
                mnemonic: "hlt".to_string(),
                fields: HashMap::new(),
                raw_instruction: raw,
            };
        }
        self.decode_data_processing(raw)
    }
}

impl ARMDecoder {
    /// Decode a data processing instruction by extracting all bit fields.
    fn decode_data_processing(&self, raw: u32) -> DecodeResult {
        let cond = ((raw >> 28) & 0xF) as i32;
        let i_bit = ((raw >> 25) & 0x1) as i32;
        let opcode = ((raw >> 21) & 0xF) as u32;
        let s_bit = ((raw >> 20) & 0x1) as i32;
        let rn = ((raw >> 16) & 0xF) as i32;
        let rd = ((raw >> 12) & 0xF) as i32;
        let operand2 = (raw & 0xFFF) as u32;

        let mnemonic = match opcode {
            OPCODE_MOV => "mov".to_string(),
            OPCODE_ADD => "add".to_string(),
            OPCODE_SUB => "sub".to_string(),
            _ => format!("dp_op({:04b})", opcode),
        };

        let mut fields = HashMap::from([
            ("cond".to_string(), cond),
            ("i_bit".to_string(), i_bit),
            ("opcode".to_string(), opcode as i32),
            ("s_bit".to_string(), s_bit),
            ("rn".to_string(), rn),
            ("rd".to_string(), rd),
        ]);

        if i_bit == 1 {
            // Immediate operand with rotation.
            //
            // ARM uses a clever bit-saving trick: the 12-bit operand2 field
            // is split into a 4-bit rotation amount and an 8-bit immediate.
            // The effective value is: rotate_right(imm8, rotate * 2).
            //
            // This allows encoding values like 0xFF000000 (imm8=0xFF, rotate=4)
            // that wouldn't fit in a plain 12-bit immediate.
            let rotate = (operand2 >> 8) & 0xF;
            let imm8 = operand2 & 0xFF;
            let shift = rotate * 2;

            let imm_value = if shift > 0 {
                // Circular right rotation (ROR).
                (imm8 >> shift) | (imm8 << (32 - shift))
            } else {
                imm8
            };
            fields.insert("imm".to_string(), imm_value as i32);
        } else {
            // Register operand: Rm is in the lowest 4 bits of operand2.
            let rm = (operand2 & 0xF) as i32;
            fields.insert("rm".to_string(), rm);
        }

        DecodeResult {
            mnemonic,
            fields,
            raw_instruction: raw,
        }
    }
}

// ===========================================================================
// Executor
// ===========================================================================

/// Applies decoded ARM instructions to registers and memory.
pub struct ARMExecutor;

impl InstructionExecutor for ARMExecutor {
    fn execute(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        _memory: &mut Memory,
        pc: usize,
    ) -> ExecuteResult {
        match decoded.mnemonic.as_str() {
            "mov" => self.exec_mov(decoded, registers, pc),
            "add" => self.exec_add(decoded, registers, pc),
            "sub" => self.exec_sub(decoded, registers, pc),
            "hlt" => ExecuteResult {
                description: "Halt".to_string(),
                registers_changed: HashMap::new(),
                memory_changed: HashMap::new(),
                next_pc: pc,
                halted: true,
            },
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

impl ARMExecutor {
    /// Execute `MOV Rd, #imm` -- move immediate value into a register.
    fn exec_mov(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        pc: usize,
    ) -> ExecuteResult {
        let rd = decoded.fields["rd"] as usize;
        let imm = decoded.fields["imm"] as u32;

        registers.write(rd, imm);

        ExecuteResult {
            description: format!("R{} = {}", rd, imm),
            registers_changed: HashMap::from([(format!("R{}", rd), imm)]),
            memory_changed: HashMap::new(),
            next_pc: pc + 4,
            halted: false,
        }
    }

    /// Execute `ADD Rd, Rn, Rm` -- add two register values.
    fn exec_add(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        pc: usize,
    ) -> ExecuteResult {
        let rd = decoded.fields["rd"] as usize;
        let rn = decoded.fields["rn"] as usize;
        let rm = decoded.fields["rm"] as usize;

        let rn_val = registers.read(rn);
        let rm_val = registers.read(rm);
        let result = rn_val.wrapping_add(rm_val);

        registers.write(rd, result);

        ExecuteResult {
            description: format!(
                "R{} = R{}({}) + R{}({}) = {}",
                rd, rn, rn_val, rm, rm_val, result
            ),
            registers_changed: HashMap::from([(format!("R{}", rd), result)]),
            memory_changed: HashMap::new(),
            next_pc: pc + 4,
            halted: false,
        }
    }

    /// Execute `SUB Rd, Rn, Rm` -- subtract two register values.
    fn exec_sub(
        &self,
        decoded: &DecodeResult,
        registers: &mut RegisterFile,
        pc: usize,
    ) -> ExecuteResult {
        let rd = decoded.fields["rd"] as usize;
        let rn = decoded.fields["rn"] as usize;
        let rm = decoded.fields["rm"] as usize;

        let rn_val = registers.read(rn);
        let rm_val = registers.read(rm);
        let result = rn_val.wrapping_sub(rm_val);

        registers.write(rd, result);

        ExecuteResult {
            description: format!(
                "R{} = R{}({}) - R{}({}) = {}",
                rd, rn, rn_val, rm, rm_val, result
            ),
            registers_changed: HashMap::from([(format!("R{}", rd), result)]),
            memory_changed: HashMap::new(),
            next_pc: pc + 4,
            halted: false,
        }
    }
}

// ===========================================================================
// Simulator
// ===========================================================================

/// The full ARM simulation environment.
pub struct ARMSimulator {
    pub cpu: CPU,
}

impl ARMSimulator {
    /// Create a new ARM simulator with 16 registers (R0-R15), each 32 bits wide.
    pub fn new(memory_size: usize) -> Self {
        ARMSimulator {
            cpu: CPU::new(
                Box::new(ARMDecoder),
                Box::new(ARMExecutor),
                16, // 16 registers (R0-R15)
                32, // 32-bit register width
                memory_size,
            ),
        }
    }

    /// Load and run a program to completion (or 10,000 step limit).
    pub fn run(&mut self, program: &[u8]) -> Vec<PipelineTrace> {
        self.cpu.load_program(program, 0);
        self.cpu.run(10000)
    }

    /// Execute a single instruction.
    pub fn step(&mut self) -> PipelineTrace {
        self.cpu.step()
    }
}

// ===========================================================================
// Encoding helpers
// ===========================================================================

/// Encode `MOV Rd, #imm` with condition=AL, I-bit=1.
pub fn encode_mov_imm(rd: usize, imm: u32) -> u32 {
    let cond = COND_AL;
    let i_bit = 1u32;
    let opcode = OPCODE_MOV;
    let s_bit = 0u32;
    let rn = 0u32;
    let imm8 = imm & 0xFF;
    let rotate = 0u32;

    (cond << 28)
        | (0b00 << 26)
        | (i_bit << 25)
        | (opcode << 21)
        | (s_bit << 20)
        | (rn << 16)
        | ((rd as u32) << 12)
        | (rotate << 8)
        | imm8
}

/// Encode `ADD Rd, Rn, Rm` with condition=AL, register mode.
pub fn encode_add(rd: usize, rn: usize, rm: usize) -> u32 {
    (COND_AL << 28)
        | (0b00 << 26)
        | (0 << 25)
        | (OPCODE_ADD << 21)
        | (0 << 20)
        | ((rn as u32) << 16)
        | ((rd as u32) << 12)
        | (rm as u32)
}

/// Encode `SUB Rd, Rn, Rm` with condition=AL, register mode.
pub fn encode_sub(rd: usize, rn: usize, rm: usize) -> u32 {
    (COND_AL << 28)
        | (0b00 << 26)
        | (0 << 25)
        | (OPCODE_SUB << 21)
        | (0 << 20)
        | ((rn as u32) << 16)
        | ((rd as u32) << 12)
        | (rm as u32)
}

/// Encode the HLT sentinel.
pub fn encode_hlt() -> u32 {
    HLT_INSTRUCTION
}

/// Assemble instruction words to little-endian bytes.
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

    /// Test a complete ARM program: MOV R0, #1; MOV R1, #2; ADD R2, R0, R1; SUB R3, R2, R0; HLT.
    #[test]
    fn arm_simulator_full_program() {
        let mut sim = ARMSimulator::new(65536);
        let program = assemble(&[
            encode_mov_imm(0, 1),
            encode_mov_imm(1, 2),
            encode_add(2, 0, 1),
            encode_sub(3, 2, 0),
            encode_hlt(),
        ]);

        let traces = sim.run(&program);
        assert_eq!(traces.len(), 5);

        assert_eq!(sim.cpu.registers.read(0), 1);
        assert_eq!(sim.cpu.registers.read(1), 2);
        assert_eq!(sim.cpu.registers.read(2), 3);
        assert_eq!(sim.cpu.registers.read(3), 2);
    }

    /// Test the ARM rotate decode logic.
    ///
    /// Creating an instruction with rotate=1, imm8=1 means:
    ///   effective_value = rotate_right(1, 1*2) = rotate_right(1, 2) = 0x40000000
    #[test]
    fn arm_rotate_decode() {
        let mut sim = ARMSimulator::new(1024);

        // Manually encode: cond=AL, I=1, opcode=MOV, rd=1, rotate=1, imm8=1
        let raw = (COND_AL << 28) | (1 << 25) | (OPCODE_MOV << 21) | (1 << 12) | (1 << 8) | 1;
        let program = assemble(&[raw, encode_hlt()]);

        sim.run(&program);
        let val = sim.cpu.registers.read(1);
        assert_eq!(val, 0x40000000, "Rotate right by 2 of 1 should be 0x40000000");
    }

    /// Unknown opcodes should produce a non-empty mnemonic, not crash.
    #[test]
    fn unknown_opcode_handled() {
        let mut sim = ARMSimulator::new(1024);
        let program = assemble(&[(COND_AL << 28) | (0xF << 21), encode_hlt()]);
        let traces = sim.run(&program);
        assert!(
            !traces[0].decode.mnemonic.is_empty(),
            "Unknown instruction should have a non-empty mnemonic"
        );
    }
}
