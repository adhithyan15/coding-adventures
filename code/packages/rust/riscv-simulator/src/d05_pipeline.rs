//! Bounded RV32I teaching-program adapter for the reusable D05 Core pipeline.
//!
//! This is an opt-in five-stage bridge for one reviewed instruction subset.
//! It uses the repository decoder, the Core's real cycle callbacks, Core RAM,
//! and operand-indexed forwarding. It does not make this simulator a general
//! RV32I pipeline implementation.

use cpu_pipeline::{
    ForwardedOperand, OperandSlot, PipelineSnapshot, PipelineToken,
};
use sim_core::{
    Core, ISADecoder, MemoryImageError, MemorySegment, RegisterFile, RegisterFileConfig,
    simple_config,
};

use crate::decode;

/// One initial little-endian word in a bounded D05 program image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D05InitialWord {
    /// Starting byte address. The adapter requires natural word alignment.
    pub address: usize,
    /// The 32-bit word value.
    pub value: u32,
}

/// Why a program could not be run through the bounded D05 adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum D05PipelineError {
    /// Instructions must be a nonempty sequence of complete 32-bit words.
    InvalidProgramLength { length: usize },
    /// A word is outside the five-instruction subset or is not exact ECALL.
    UnsupportedInstruction { pc: usize, raw: u32, mnemonic: String },
    /// Input RAM must be nonempty and contain the complete program.
    InvalidRamSize { ram_bytes: usize, program_bytes: usize },
    /// Initial words must be naturally aligned.
    MisalignedInitialWord { address: usize },
    /// An initial data range may not overwrite the loaded program.
    ProgramDataOverlap { address: usize },
    /// The supplied initial image did not fit or had an ambiguous overlap.
    MemoryImage(MemoryImageError),
    /// The pipeline Core could not be constructed.
    CoreConstruction(String),
    /// ECALL did not retire within the caller's cycle budget.
    CycleLimitExceeded { max_cycles: usize },
}

impl std::fmt::Display for D05PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProgramLength { length } => {
                write!(f, "program length {length} is not a nonzero multiple of four")
            }
            Self::UnsupportedInstruction { pc, raw, mnemonic } => write!(
                f,
                "unsupported D05 instruction {mnemonic} ({raw:#010x}) at PC {pc}"
            ),
            Self::InvalidRamSize {
                ram_bytes,
                program_bytes,
            } => write!(
                f,
                "RAM size {ram_bytes} cannot hold program of {program_bytes} bytes"
            ),
            Self::MisalignedInitialWord { address } => {
                write!(f, "initial word address {address} is not aligned")
            }
            Self::ProgramDataOverlap { address } => {
                write!(f, "initial data at {address} overlaps the program")
            }
            Self::MemoryImage(error) => error.fmt(f),
            Self::CoreConstruction(error) => write!(f, "cannot construct Core: {error}"),
            Self::CycleLimitExceeded { max_cycles } => {
                write!(f, "ECALL did not retire within {max_cycles} cycles")
            }
        }
    }
}

impl std::error::Error for D05PipelineError {}

impl From<MemoryImageError> for D05PipelineError {
    fn from(error: MemoryImageError) -> Self {
        Self::MemoryImage(error)
    }
}

/// The observations produced by one real `Core::step()` call.
#[derive(Debug, Clone)]
pub struct D05CycleObservation {
    /// Pipeline snapshot after this Core cycle.
    pub snapshot: PipelineSnapshot,
    /// Named operand values delivered to EX in this cycle.
    pub forwarded_operands: Vec<ForwardedOperand>,
}

/// Final state and actual-cycle observations from the D05 Core pipeline.
#[derive(Debug, Clone)]
pub struct D05PipelineResult {
    /// One observation for each Core cycle, including the ECALL retirement.
    pub cycles: Vec<D05CycleObservation>,
    /// Program counters observed in WB, in retirement order.
    pub retired_pcs: Vec<u32>,
    /// Core register values x0-x31 after halt.
    pub registers: [u32; 32],
    /// Entire bounded Core RAM after halt.
    pub memory: Vec<u8>,
    /// Number of cycles held for a data hazard.
    pub stall_count: usize,
    /// Number of cycles redirected after a branch.
    pub flush_count: usize,
    /// Actual Core cycle count at halt.
    pub total_cycles: usize,
}

/// Runs a bounded program subset through the existing five-stage Core.
///
/// Supported instructions are `addi`, `sw`, `lw`, `beq`, and the exact ECALL
/// encoding `0x00000073`. The caller supplies flat RAM size and initial words;
/// the program is loaded at byte zero and words are initialized after it.
/// Execution advances through real Core ticks and fails if ECALL does not
/// retire within `max_cycles`.
pub fn run_d05_pipeline(
    program: &[u8],
    ram_bytes: usize,
    initial_words: &[D05InitialWord],
    max_cycles: usize,
) -> Result<D05PipelineResult, D05PipelineError> {
    validate_program(program)?;
    if ram_bytes == 0 || program.len() > ram_bytes {
        return Err(D05PipelineError::InvalidRamSize {
            ram_bytes,
            program_bytes: program.len(),
        });
    }
    for word in initial_words {
        if word.address % 4 != 0 {
            return Err(D05PipelineError::MisalignedInitialWord {
                address: word.address,
            });
        }
        if word.address < program.len() {
            return Err(D05PipelineError::ProgramDataOverlap {
                address: word.address,
            });
        }
    }

    let mut config = simple_config();
    config.memory_size = ram_bytes;
    config.register_file = Some(RegisterFileConfig {
        count: 32,
        width: 32,
        zero_register: true,
    });
    let mut core = Core::new_with_operand_forwarding(config, Box::new(D05RiscVDecoder))
        .map_err(D05PipelineError::CoreConstruction)?;
    core.load_program(program, 0);

    let word_bytes: Vec<_> = initial_words
        .iter()
        .map(|word| word.value.to_le_bytes())
        .collect();
    let segments: Vec<_> = initial_words
        .iter()
        .zip(&word_bytes)
        .map(|(word, bytes)| MemorySegment {
            address: word.address,
            bytes,
        })
        .collect();
    core.initialize_memory_segments(&segments)?;

    let mut cycles = Vec::new();
    let mut retired_pcs = Vec::new();
    for _ in 0..max_cycles {
        let snapshot = core.step();
        if let Some(writeback) = snapshot.stages.get("WB") {
            if !writeback.is_bubble {
                retired_pcs.push(writeback.pc as u32);
            }
        }
        cycles.push(D05CycleObservation {
            snapshot,
            forwarded_operands: core.pipeline().last_forwarded_operands().to_vec(),
        });
        if core.is_halted() {
            let mut registers = [0; 32];
            for (index, register) in registers.iter_mut().enumerate() {
                *register = core.read_register(index as i64) as u32;
            }
            let stats = core.stats();
            let memory = core.read_memory_bytes(0, ram_bytes)?;
            return Ok(D05PipelineResult {
                cycles,
                retired_pcs,
                registers,
                memory,
                stall_count: stats.stall_count.max(0) as usize,
                flush_count: stats.flush_count.max(0) as usize,
                total_cycles: core.cycle() as usize,
            });
        }
    }
    Err(D05PipelineError::CycleLimitExceeded { max_cycles })
}

fn validate_program(program: &[u8]) -> Result<(), D05PipelineError> {
    if program.is_empty() || !program.len().is_multiple_of(4) {
        return Err(D05PipelineError::InvalidProgramLength {
            length: program.len(),
        });
    }
    for (index, bytes) in program.chunks_exact(4).enumerate() {
        let raw = u32::from_le_bytes(bytes.try_into().expect("four-byte instruction"));
        let decoded = decode::decode(raw, (index * 4) as i32);
        let supported = matches!(decoded.mnemonic.as_str(), "addi" | "sw" | "lw" | "beq")
            || (decoded.mnemonic == "ecall" && raw == 0x0000_0073);
        if !supported {
            return Err(D05PipelineError::UnsupportedInstruction {
                pc: index * 4,
                raw,
                mnemonic: decoded.mnemonic,
            });
        }
    }
    Ok(())
}

struct D05RiscVDecoder;

impl ISADecoder for D05RiscVDecoder {
    fn decode(&self, raw_instruction: i64, mut token: PipelineToken) -> PipelineToken {
        let raw = raw_instruction as u32;
        let decoded = decode::decode(raw, token.pc as i32);
        token.raw_instruction = raw_instruction;
        token.opcode = decoded.mnemonic.clone();
        token.rd = field(&decoded.fields, "rd", -1) as i64;
        token.rs1 = field(&decoded.fields, "rs1", -1) as i64;
        token.rs2 = field(&decoded.fields, "rs2", -1) as i64;
        token.immediate = field(&decoded.fields, "imm", 0) as i64;
        match decoded.mnemonic.as_str() {
            "addi" => token.reg_write = true,
            "lw" => {
                token.reg_write = true;
                token.mem_read = true;
            }
            "sw" => token.mem_write = true,
            "beq" => token.is_branch = true,
            "ecall" => token.is_halt = raw == 0x0000_0073,
            _ => {}
        }
        token
    }

    fn execute(&self, token: PipelineToken, reg_file: &RegisterFile) -> PipelineToken {
        self.execute_forwarded(token, reg_file, &[])
    }

    fn execute_with_forwarding(
        &self,
        token: PipelineToken,
        reg_file: &RegisterFile,
        forwarded: &[ForwardedOperand],
    ) -> PipelineToken {
        self.execute_forwarded(token, reg_file, forwarded)
    }

    fn instruction_size(&self) -> i64 {
        4
    }
}

impl D05RiscVDecoder {
    fn execute_forwarded(
        &self,
        mut token: PipelineToken,
        reg_file: &RegisterFile,
        forwarded: &[ForwardedOperand],
    ) -> PipelineToken {
        let rs1 = source_value(token.rs1, OperandSlot::Rs1, reg_file, forwarded);
        let rs2 = source_value(token.rs2, OperandSlot::Rs2, reg_file, forwarded);
        match token.opcode.as_str() {
            "addi" => {
                token.write_data = rs1.wrapping_add(token.immediate);
                token.alu_result = token.write_data;
            }
            "lw" | "sw" => {
                token.alu_result = rs1.wrapping_add(token.immediate);
                if token.opcode == "sw" {
                    token.write_data = rs2;
                }
            }
            "beq" => {
                token.branch_taken = rs1 == rs2;
                token.branch_target = token.pc.wrapping_add(token.immediate);
                token.alu_result = if token.branch_taken {
                    token.branch_target
                } else {
                    token.pc.wrapping_add(4)
                };
            }
            _ => {}
        }
        token
    }
}

fn field(fields: &std::collections::HashMap<String, i32>, name: &str, default: i32) -> i32 {
    fields.get(name).copied().unwrap_or(default)
}

fn source_value(
    register: i64,
    slot: OperandSlot,
    reg_file: &RegisterFile,
    forwarded: &[ForwardedOperand],
) -> i64 {
    if register < 0 {
        return 0;
    }
    forwarded
        .iter()
        .find(|value| value.operand == slot && value.register == register)
        .map_or_else(|| reg_file.read(register), |value| value.value)
}
