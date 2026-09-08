//! Exact checked Spec 07a RV32I architectural machine.
//!
//! The older [`crate::simulator::RiscVSimulator`] remains the compiler-facing
//! host-ABI harness. This module is the normative, restorable 64 KiB machine.

use crate::csr::{
    CAUSE_ECALL_M_MODE, CSR_MCAUSE, CSR_MEPC, CSR_MSCRATCH, CSR_MSTATUS, CSR_MTVEC, MIE,
};
use crate::decode::{self, DecodeResult};

/// Exact architectural memory size.
pub const RV32I_MEMORY_SIZE: usize = 65_536;

/// Complete architectural and installation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rv32IState {
    /// General-purpose registers x0-x31. x0 must remain zero.
    pub registers: [u32; 32],
    /// Address of the next instruction.
    pub pc: u32,
    /// Machine status CSR (0x300).
    pub csr_mstatus: u32,
    /// Machine trap-vector CSR (0x305).
    pub csr_mtvec: u32,
    /// Machine scratch CSR (0x340).
    pub csr_mscratch: u32,
    /// Machine exception-PC CSR (0x341).
    pub csr_mepc: u32,
    /// Machine cause CSR (0x342).
    pub csr_mcause: u32,
    /// Exact 64 KiB little-endian byte-addressed memory.
    pub memory: Vec<u8>,
    /// Whether a zero-vector ECALL has committed.
    pub halted: bool,
    /// First installed program byte.
    pub loaded_origin: u32,
    /// Number of installed program bytes.
    pub loaded_len: usize,
}

/// Typed lifecycle and architectural failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rv32IError {
    Halted,
    InvalidRegister { index: usize },
    InvalidState(String),
    MisalignedProgram { origin: u32 },
    ProgramOutOfRange { origin: u32, length: usize },
    MisalignedFetch { pc: u32 },
    FetchOutsideProgram { pc: u32 },
    MisalignedData { address: u32, width: usize },
    DataOutOfRange { address: u32, width: usize },
    UnknownInstruction { raw: u32, pc: u32 },
    UnsupportedCsr { address: u32, pc: u32 },
    StepLimitExceeded { max_steps: usize },
}

impl std::fmt::Display for Rv32IError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidRegister { index } => write!(f, "register {index} is outside x0-x31"),
            Self::InvalidState(message) => f.write_str(message),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin {origin:#010x} is not word-aligned")
            }
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes at {origin:#010x} exceeds 64 KiB memory"
            ),
            Self::MisalignedFetch { pc } => write!(f, "misaligned fetch at {pc:#010x}"),
            Self::FetchOutsideProgram { pc } => {
                write!(
                    f,
                    "instruction at {pc:#010x} crosses installed program bounds"
                )
            }
            Self::MisalignedData { address, width } => {
                write!(f, "{width}-byte access at {address:#010x} is misaligned")
            }
            Self::DataOutOfRange { address, width } => write!(
                f,
                "{width}-byte access at {address:#010x} exceeds 64 KiB memory"
            ),
            Self::UnknownInstruction { raw, pc } => {
                write!(f, "unknown instruction {raw:#010x} at {pc:#010x}")
            }
            Self::UnsupportedCsr { address, pc } => {
                write!(f, "unsupported CSR {address:#05x} at {pc:#010x}")
            }
            Self::StepLimitExceeded { max_steps } => {
                write!(f, "step limit {max_steps} exceeded")
            }
        }
    }
}

impl std::error::Error for Rv32IError {}

/// One committed architectural transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rv32IStepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: u32,
    pub mnemonic: String,
    pub state_before: Rv32IState,
    pub state_after: Rv32IState,
}

/// Successful bounded execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rv32IExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<Rv32IStepTrace>,
    pub final_state: Rv32IState,
}

/// Exact checked RV32I functional simulator.
#[derive(Debug, Clone)]
pub struct Rv32ISimulator {
    state: Rv32IState,
}

impl Default for Rv32ISimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Rv32ISimulator {
    /// Construct the reset state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Rv32IState {
                registers: [0; 32],
                pc: 0,
                csr_mstatus: 0,
                csr_mtvec: 0,
                csr_mscratch: 0,
                csr_mepc: 0,
                csr_mcause: 0,
                memory: vec![0; RV32I_MEMORY_SIZE],
                halted: false,
                loaded_origin: 0,
                loaded_len: 0,
            },
        }
    }

    /// Restore the reset state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Clone the complete state.
    #[must_use]
    pub fn get_state(&self) -> Rv32IState {
        self.state.clone()
    }

    /// Atomically restore a validated snapshot.
    pub fn restore(&mut self, state: &Rv32IState) -> Result<(), Rv32IError> {
        validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    /// Reset and install a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), Rv32IError> {
        self.load_at_checked(program, 0)
    }

    /// Reset and install a program at a checked word-aligned address.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), Rv32IError> {
        if origin & 3 != 0 {
            return Err(Rv32IError::MisalignedProgram { origin });
        }
        let start = origin as usize;
        let end = start
            .checked_add(program.len())
            .filter(|end| *end <= RV32I_MEMORY_SIZE)
            .ok_or(Rv32IError::ProgramOutOfRange {
                origin,
                length: program.len(),
            })?;
        let mut next = Self::new();
        next.state.memory[start..end].copy_from_slice(program);
        next.state.pc = origin;
        next.state.loaded_origin = origin;
        next.state.loaded_len = program.len();
        *self = next;
        Ok(())
    }

    /// Read one register.
    pub fn read_register(&self, index: usize) -> Result<u32, Rv32IError> {
        self.state
            .registers
            .get(index)
            .copied()
            .ok_or(Rv32IError::InvalidRegister { index })
    }

    /// Write one register. Writes to x0 are discarded.
    pub fn write_register(&mut self, index: usize, value: u32) -> Result<(), Rv32IError> {
        let register = self
            .state
            .registers
            .get_mut(index)
            .ok_or(Rv32IError::InvalidRegister { index })?;
        if index != 0 {
            *register = value;
        }
        Ok(())
    }

    /// Read one checked byte.
    pub fn read_byte(&self, address: u32) -> Result<u8, Rv32IError> {
        self.state
            .memory
            .get(address as usize)
            .copied()
            .ok_or(Rv32IError::DataOutOfRange { address, width: 1 })
    }

    /// Write one checked byte.
    pub fn write_byte(&mut self, address: u32, value: u8) -> Result<(), Rv32IError> {
        let byte = self
            .state
            .memory
            .get_mut(address as usize)
            .ok_or(Rv32IError::DataOutOfRange { address, width: 1 })?;
        *byte = value;
        Ok(())
    }

    /// Execute one transition atomically.
    pub fn step_checked(&mut self) -> Result<Rv32IStepTrace, Rv32IError> {
        if self.state.halted {
            return Err(Rv32IError::Halted);
        }
        let before = self.state.clone();
        let pc = before.pc;
        if pc & 3 != 0 {
            return Err(Rv32IError::MisalignedFetch { pc });
        }
        check_fetch(&before, pc)?;
        let raw = read_u32(&before, pc, false)?;
        let decoded = decode::decode(raw, pc as i32);
        validate_decoded(&decoded, pc)?;
        let mut next = before.clone();
        execute(&mut next, &decoded, pc)?;
        next.registers[0] = 0;
        let trace = Rv32IStepTrace {
            pc_before: pc,
            pc_after: next.pc,
            raw,
            mnemonic: decoded.mnemonic,
            state_before: before,
            state_after: next.clone(),
        };
        self.state = next;
        Ok(trace)
    }

    /// Execute until halt and roll back the whole run on any failure.
    pub fn run_checked(&mut self, max_steps: usize) -> Result<Rv32IExecutionResult, Rv32IError> {
        let before = self.state.clone();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.state.halted {
                return Ok(Rv32IExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.state.clone(),
                });
            }
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
        }
        self.state = before;
        Err(Rv32IError::StepLimitExceeded { max_steps })
    }
}

fn validate_state(state: &Rv32IState) -> Result<(), Rv32IError> {
    if state.memory.len() != RV32I_MEMORY_SIZE {
        return Err(Rv32IError::InvalidState(
            "memory must contain exactly 65,536 bytes".into(),
        ));
    }
    if state.registers[0] != 0 {
        return Err(Rv32IError::InvalidState("x0 must remain zero".into()));
    }
    if state.pc & 3 != 0 || state.loaded_origin & 3 != 0 {
        return Err(Rv32IError::InvalidState(
            "PC and loaded origin must be word-aligned".into(),
        ));
    }
    let start = state.loaded_origin as usize;
    if start
        .checked_add(state.loaded_len)
        .is_none_or(|end| end > RV32I_MEMORY_SIZE)
    {
        return Err(Rv32IError::InvalidState(
            "installed program exceeds memory".into(),
        ));
    }
    Ok(())
}

fn check_fetch(state: &Rv32IState, pc: u32) -> Result<(), Rv32IError> {
    let start = state.loaded_origin as usize;
    let end = start + state.loaded_len;
    let address = pc as usize;
    if address < start || address.checked_add(4).is_none_or(|last| last > end) {
        Err(Rv32IError::FetchOutsideProgram { pc })
    } else {
        Ok(())
    }
}

fn validate_decoded(decoded: &DecodeResult, pc: u32) -> Result<(), Rv32IError> {
    let mnemonic = decoded.mnemonic.as_str();
    let known = matches!(
        mnemonic,
        "addi"
            | "slti"
            | "sltiu"
            | "xori"
            | "ori"
            | "andi"
            | "slli"
            | "srli"
            | "srai"
            | "add"
            | "sub"
            | "sll"
            | "slt"
            | "sltu"
            | "xor"
            | "srl"
            | "sra"
            | "or"
            | "and"
            | "lb"
            | "lh"
            | "lw"
            | "lbu"
            | "lhu"
            | "sb"
            | "sh"
            | "sw"
            | "beq"
            | "bne"
            | "blt"
            | "bge"
            | "bltu"
            | "bgeu"
            | "jal"
            | "jalr"
            | "lui"
            | "auipc"
            | "ecall"
            | "mret"
            | "csrrw"
            | "csrrs"
            | "csrrc"
    );
    if !known || !valid_rv32i_encoding(decoded.raw) {
        return Err(Rv32IError::UnknownInstruction {
            raw: decoded.raw,
            pc,
        });
    }
    Ok(())
}

fn valid_rv32i_encoding(raw: u32) -> bool {
    let opcode = raw & 0x7f;
    let funct3 = (raw >> 12) & 7;
    let funct7 = raw >> 25;
    match opcode {
        0x13 => match funct3 {
            0 | 2 | 3 | 4 | 6 | 7 => true,
            1 => funct7 == 0,
            5 => matches!(funct7, 0 | 0x20),
            _ => false,
        },
        0x33 => match funct3 {
            0 => matches!(funct7, 0 | 0x20),
            5 => matches!(funct7, 0 | 0x20),
            1 | 2 | 3 | 4 | 6 | 7 => funct7 == 0,
            _ => false,
        },
        0x03 => matches!(funct3, 0 | 1 | 2 | 4 | 5),
        0x23 => matches!(funct3, 0..=2),
        0x63 => matches!(funct3, 0 | 1 | 4 | 5 | 6 | 7),
        0x67 => funct3 == 0,
        0x6f | 0x17 | 0x37 => true,
        0x73 => matches!(raw, 0x0000_0073 | 0x3020_0073) || matches!(funct3, 1..=3),
        _ => false,
    }
}

fn field(decoded: &DecodeResult, name: &str) -> u32 {
    decoded.fields.get(name).copied().unwrap_or_default() as u32
}

fn write_rd(state: &mut Rv32IState, rd: u32, value: u32) {
    if rd != 0 {
        state.registers[rd as usize] = value;
    }
}

fn execute(state: &mut Rv32IState, decoded: &DecodeResult, pc: u32) -> Result<(), Rv32IError> {
    let rd = field(decoded, "rd");
    let rs1 = field(decoded, "rs1") as usize;
    let rs2 = field(decoded, "rs2") as usize;
    let imm = field(decoded, "imm");
    let a = state.registers[rs1];
    let b = state.registers[rs2];
    state.pc = pc.wrapping_add(4);
    match decoded.mnemonic.as_str() {
        "addi" => write_rd(state, rd, a.wrapping_add(imm)),
        "slti" => write_rd(state, rd, u32::from((a as i32) < (imm as i32))),
        "sltiu" => write_rd(state, rd, u32::from(a < imm)),
        "xori" => write_rd(state, rd, a ^ imm),
        "ori" => write_rd(state, rd, a | imm),
        "andi" => write_rd(state, rd, a & imm),
        "slli" => write_rd(state, rd, a << (imm & 31)),
        "srli" => write_rd(state, rd, a >> (imm & 31)),
        "srai" => write_rd(state, rd, ((a as i32) >> (imm & 31)) as u32),
        "add" => write_rd(state, rd, a.wrapping_add(b)),
        "sub" => write_rd(state, rd, a.wrapping_sub(b)),
        "sll" => write_rd(state, rd, a << (b & 31)),
        "slt" => write_rd(state, rd, u32::from((a as i32) < (b as i32))),
        "sltu" => write_rd(state, rd, u32::from(a < b)),
        "xor" => write_rd(state, rd, a ^ b),
        "srl" => write_rd(state, rd, a >> (b & 31)),
        "sra" => write_rd(state, rd, ((a as i32) >> (b & 31)) as u32),
        "or" => write_rd(state, rd, a | b),
        "and" => write_rd(state, rd, a & b),
        "lb" => {
            let value = read_u8(state, a.wrapping_add(imm))?;
            write_rd(state, rd, (value as i8) as i32 as u32);
        }
        "lbu" => write_rd(state, rd, u32::from(read_u8(state, a.wrapping_add(imm))?)),
        "lh" => {
            let value = read_u16(state, a.wrapping_add(imm))?;
            write_rd(state, rd, (value as i16) as i32 as u32);
        }
        "lhu" => write_rd(state, rd, u32::from(read_u16(state, a.wrapping_add(imm))?)),
        "lw" => write_rd(state, rd, read_u32(state, a.wrapping_add(imm), true)?),
        "sb" => write_u8(state, a.wrapping_add(imm), b as u8)?,
        "sh" => write_u16(state, a.wrapping_add(imm), b as u16)?,
        "sw" => write_u32(state, a.wrapping_add(imm), b)?,
        "beq" if a == b => state.pc = pc.wrapping_add(imm),
        "bne" if a != b => state.pc = pc.wrapping_add(imm),
        "blt" if (a as i32) < (b as i32) => state.pc = pc.wrapping_add(imm),
        "bge" if (a as i32) >= (b as i32) => state.pc = pc.wrapping_add(imm),
        "bltu" if a < b => state.pc = pc.wrapping_add(imm),
        "bgeu" if a >= b => state.pc = pc.wrapping_add(imm),
        "beq" | "bne" | "blt" | "bge" | "bltu" | "bgeu" => {}
        "jal" => {
            write_rd(state, rd, pc.wrapping_add(4));
            state.pc = pc.wrapping_add(imm);
        }
        "jalr" => {
            let target = a.wrapping_add(imm) & !1;
            write_rd(state, rd, pc.wrapping_add(4));
            state.pc = target;
        }
        "lui" => write_rd(state, rd, imm << 12),
        "auipc" => write_rd(state, rd, pc.wrapping_add(imm << 12)),
        "ecall" => {
            if state.csr_mtvec == 0 {
                state.pc = pc;
                state.halted = true;
            } else {
                state.csr_mepc = pc;
                state.csr_mcause = CAUSE_ECALL_M_MODE;
                state.csr_mstatus &= !MIE;
                state.pc = state.csr_mtvec;
            }
        }
        "mret" => {
            state.csr_mstatus |= MIE;
            state.pc = state.csr_mepc;
        }
        "csrrw" | "csrrs" | "csrrc" => {
            let address = field(decoded, "csr");
            let old = read_csr(state, address, pc)?;
            let source = state.registers[rs1];
            let value = match decoded.mnemonic.as_str() {
                "csrrw" => source,
                "csrrs" => old | source,
                "csrrc" => old & !source,
                _ => unreachable!(),
            };
            write_csr(state, address, value, pc)?;
            write_rd(state, rd, old);
        }
        _ => unreachable!("validated mnemonic"),
    }
    Ok(())
}

fn read_csr(state: &Rv32IState, address: u32, pc: u32) -> Result<u32, Rv32IError> {
    match address {
        CSR_MSTATUS => Ok(state.csr_mstatus),
        CSR_MTVEC => Ok(state.csr_mtvec),
        CSR_MSCRATCH => Ok(state.csr_mscratch),
        CSR_MEPC => Ok(state.csr_mepc),
        CSR_MCAUSE => Ok(state.csr_mcause),
        _ => Err(Rv32IError::UnsupportedCsr { address, pc }),
    }
}

fn write_csr(state: &mut Rv32IState, address: u32, value: u32, pc: u32) -> Result<(), Rv32IError> {
    match address {
        CSR_MSTATUS => state.csr_mstatus = value,
        CSR_MTVEC => state.csr_mtvec = value,
        CSR_MSCRATCH => state.csr_mscratch = value,
        CSR_MEPC => state.csr_mepc = value,
        CSR_MCAUSE => state.csr_mcause = value,
        _ => return Err(Rv32IError::UnsupportedCsr { address, pc }),
    }
    Ok(())
}

fn checked_range(address: u32, width: usize, alignment: usize) -> Result<usize, Rv32IError> {
    if !(address as usize).is_multiple_of(alignment) {
        return Err(Rv32IError::MisalignedData { address, width });
    }
    let start = address as usize;
    if start
        .checked_add(width)
        .is_none_or(|end| end > RV32I_MEMORY_SIZE)
    {
        return Err(Rv32IError::DataOutOfRange { address, width });
    }
    Ok(start)
}

fn read_u8(state: &Rv32IState, address: u32) -> Result<u8, Rv32IError> {
    let start = checked_range(address, 1, 1)?;
    Ok(state.memory[start])
}

fn read_u16(state: &Rv32IState, address: u32) -> Result<u16, Rv32IError> {
    let start = checked_range(address, 2, 2)?;
    Ok(u16::from_le_bytes([
        state.memory[start],
        state.memory[start + 1],
    ]))
}

fn read_u32(state: &Rv32IState, address: u32, data: bool) -> Result<u32, Rv32IError> {
    let start = if data {
        checked_range(address, 4, 4)?
    } else {
        address as usize
    };
    Ok(u32::from_le_bytes([
        state.memory[start],
        state.memory[start + 1],
        state.memory[start + 2],
        state.memory[start + 3],
    ]))
}

fn write_u8(state: &mut Rv32IState, address: u32, value: u8) -> Result<(), Rv32IError> {
    let start = checked_range(address, 1, 1)?;
    state.memory[start] = value;
    Ok(())
}

fn write_u16(state: &mut Rv32IState, address: u32, value: u16) -> Result<(), Rv32IError> {
    let start = checked_range(address, 2, 2)?;
    state.memory[start..start + 2].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u32(state: &mut Rv32IState, address: u32, value: u32) -> Result<(), Rv32IError> {
    let start = checked_range(address, 4, 4)?;
    state.memory[start..start + 4].copy_from_slice(&value.to_le_bytes());
    Ok(())
}
