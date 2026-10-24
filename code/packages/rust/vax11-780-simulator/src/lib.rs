//! Checked DEC VAX-11/780 integer architectural simulator.
//!
//! The model implements the one-byte integer opcode core and the complete VAX
//! general-operand addressing grammar over a bounded 64 KiB educational arena.

pub mod encoding;

/// Exact size of the educational memory arena.
pub const MEMORY_SIZE: usize = 65_536;
/// Architectural stack pointer register.
pub const REGISTER_SP: usize = 14;
/// Architectural program counter register.
pub const REGISTER_PC: usize = 15;
/// Maximum number of traces retained by one bounded run.
pub const MAX_RETAINED_TRACES: usize = 65_536;

/// The architecturally visible low PSL condition bits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Psl {
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
}

/// Complete owned architectural and lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaxState {
    pub regs: [u32; 16],
    pub psl: Psl,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

/// Scalar state retained on each trace without duplicating the memory arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarState {
    pub regs: [u32; 16],
    pub psl: Psl,
    pub halted: bool,
}

/// A complete successful instruction transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: Vec<u8>,
    pub mnemonic: String,
    pub state_before: ScalarState,
    pub state_after: ScalarState,
}

/// Successful bounded execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: VaxState,
}

/// Typed fail-closed lifecycle and execution errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaxError {
    Halted,
    InvalidRegister {
        index: usize,
    },
    InvalidState(String),
    ProgramOutOfRange {
        origin: u32,
        length: usize,
        memory_size: usize,
    },
    FetchOutsideProgram {
        pc: u32,
    },
    TruncatedInstruction {
        pc: u32,
    },
    MemoryOutOfRange {
        address: u32,
        width: usize,
    },
    StackOutOfRange {
        sp: u32,
    },
    UnknownOpcode {
        opcode: u8,
        pc: u32,
    },
    UnsupportedTwoByteOpcode {
        prefix: u8,
        pc: u32,
    },
    ReservedAddressingMode {
        specifier: u8,
        pc: u32,
    },
    RecursiveIndexMode {
        pc: u32,
    },
    InvalidDestinationMode {
        specifier: u8,
        pc: u32,
    },
    DivideByZero {
        pc: u32,
    },
    StepLimitExceeded {
        max_steps: usize,
    },
    TraceLimitTooLarge {
        requested: usize,
        maximum: usize,
    },
}

impl std::fmt::Display for VaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidRegister { index } => write!(f, "register {index} is outside R0-R15"),
            Self::InvalidState(message) => f.write_str(message),
            Self::ProgramOutOfRange {
                origin,
                length,
                memory_size,
            } => write!(
                f,
                "program of {length} bytes at {origin:#010x} exceeds {memory_size}-byte memory"
            ),
            Self::FetchOutsideProgram { pc } => {
                write!(
                    f,
                    "instruction fetch at {pc:#010x} is outside the installed program"
                )
            }
            Self::TruncatedInstruction { pc } => {
                write!(
                    f,
                    "instruction at {pc:#010x} crosses the installed program boundary"
                )
            }
            Self::MemoryOutOfRange { address, width } => {
                write!(f, "{width}-byte access at {address:#010x} exceeds memory")
            }
            Self::StackOutOfRange { sp } => write!(f, "stack access at {sp:#010x} exceeds memory"),
            Self::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown opcode {opcode:#04x} at {pc:#010x}")
            }
            Self::UnsupportedTwoByteOpcode { prefix, pc } => write!(
                f,
                "unsupported two-byte opcode prefix {prefix:#04x} at {pc:#010x}"
            ),
            Self::ReservedAddressingMode { specifier, pc } => write!(
                f,
                "reserved addressing specifier {specifier:#04x} at {pc:#010x}"
            ),
            Self::RecursiveIndexMode { pc } => {
                write!(f, "recursive indexed addressing at {pc:#010x}")
            }
            Self::InvalidDestinationMode { specifier, pc } => write!(
                f,
                "specifier {specifier:#04x} is not writable at {pc:#010x}"
            ),
            Self::DivideByZero { pc } => write!(f, "division by zero at {pc:#010x}"),
            Self::StepLimitExceeded { max_steps } => {
                write!(f, "step limit {max_steps} exceeded")
            }
            Self::TraceLimitTooLarge { requested, maximum } => write!(
                f,
                "requested trace limit {requested} exceeds safe maximum {maximum}"
            ),
        }
    }
}

impl std::error::Error for VaxError {}

#[derive(Debug, Clone, Copy)]
enum Place {
    Register(usize),
    Memory(u32),
}

#[derive(Debug, Clone, Copy)]
enum Operand {
    Value { value: u32, specifier: u8 },
    Place { place: Place, specifier: u8 },
}

/// Functional DEC VAX-11/780 integer simulator.
#[derive(Debug, Clone)]
pub struct VaxSimulator {
    regs: [u32; 16],
    psl: Psl,
    memory: Vec<u8>,
    halted: bool,
    loaded_origin: u32,
    loaded_len: usize,
}

impl Default for VaxSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl VaxSimulator {
    /// Construct a reset machine.
    pub fn new() -> Self {
        Self {
            regs: [0; 16],
            psl: Psl::default(),
            memory: vec![0; MEMORY_SIZE],
            halted: false,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    /// Reset every architectural and lifecycle field.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Return a complete owned snapshot.
    pub fn get_state(&self) -> VaxState {
        VaxState {
            regs: self.regs,
            psl: self.psl,
            memory: self.memory.clone(),
            halted: self.halted,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    /// Atomically restore a validated complete snapshot.
    pub fn restore(&mut self, state: &VaxState) -> Result<(), VaxError> {
        validate_state(state)?;
        self.regs = state.regs;
        self.psl = state.psl;
        self.memory.clone_from(&state.memory);
        self.halted = state.halted;
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        Ok(())
    }

    /// Return the condition flags.
    pub const fn psl(&self) -> Psl {
        self.psl
    }

    /// Replace the condition flags.
    pub fn set_psl(&mut self, psl: Psl) {
        self.psl = psl;
    }

    /// Read a register after validating its index.
    pub fn read_register_checked(&self, index: usize) -> Result<u32, VaxError> {
        self.regs
            .get(index)
            .copied()
            .ok_or(VaxError::InvalidRegister { index })
    }

    /// Write a full register after validating its index.
    pub fn write_register_checked(&mut self, index: usize, value: u32) -> Result<(), VaxError> {
        let register = self
            .regs
            .get_mut(index)
            .ok_or(VaxError::InvalidRegister { index })?;
        *register = value;
        Ok(())
    }

    /// Write a one-, two-, or four-byte register slice.
    pub fn write_register_width_checked(
        &mut self,
        index: usize,
        width: usize,
        value: u32,
    ) -> Result<(), VaxError> {
        if !matches!(width, 1 | 2 | 4) {
            return Err(VaxError::InvalidState(format!(
                "invalid operand width {width}"
            )));
        }
        let previous = self.read_register_checked(index)?;
        self.write_register_checked(index, merge_width(previous, value, width))
    }

    /// Read one byte from memory.
    pub fn read_byte_checked(&self, address: u32) -> Result<u8, VaxError> {
        let start = checked_range(address, 1)?;
        Ok(self.memory[start])
    }

    /// Read a little-endian word from memory.
    pub fn read_word_checked(&self, address: u32) -> Result<u16, VaxError> {
        let start = checked_range(address, 2)?;
        Ok(u16::from_le_bytes([
            self.memory[start],
            self.memory[start + 1],
        ]))
    }

    /// Read a little-endian longword from memory.
    pub fn read_long_checked(&self, address: u32) -> Result<u32, VaxError> {
        let start = checked_range(address, 4)?;
        Ok(u32::from_le_bytes(
            self.memory[start..start + 4]
                .try_into()
                .expect("length checked"),
        ))
    }

    /// Write one byte to memory.
    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), VaxError> {
        let start = checked_range(address, 1)?;
        self.memory[start] = value;
        Ok(())
    }

    /// Write a little-endian word to memory.
    pub fn write_word_checked(&mut self, address: u32, value: u16) -> Result<(), VaxError> {
        let start = checked_range(address, 2)?;
        self.memory[start..start + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Write a little-endian longword to memory.
    pub fn write_long_checked(&mut self, address: u32, value: u32) -> Result<(), VaxError> {
        let start = checked_range(address, 4)?;
        self.memory[start..start + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// Reset and install a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), VaxError> {
        self.load_at_checked(program, 0)
    }

    /// Reset and atomically install a program at an exact origin.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), VaxError> {
        let start = usize::try_from(origin).map_err(|_| VaxError::ProgramOutOfRange {
            origin,
            length: program.len(),
            memory_size: MEMORY_SIZE,
        })?;
        let end = start
            .checked_add(program.len())
            .filter(|end| *end <= MEMORY_SIZE)
            .ok_or(VaxError::ProgramOutOfRange {
                origin,
                length: program.len(),
                memory_size: MEMORY_SIZE,
            })?;
        let mut replacement = Self::new();
        replacement.memory[start..end].copy_from_slice(program);
        replacement.regs[REGISTER_PC] = origin;
        replacement.loaded_origin = origin;
        replacement.loaded_len = program.len();
        *self = replacement;
        Ok(())
    }

    /// Execute exactly one instruction, rolling back on every error.
    pub fn step_checked(&mut self) -> Result<StepTrace, VaxError> {
        if self.halted {
            return Err(VaxError::Halted);
        }
        let checkpoint = self.clone();
        let pc_before = self.regs[REGISTER_PC];
        if !self.pc_is_loaded(pc_before) {
            return Err(VaxError::FetchOutsideProgram { pc: pc_before });
        }
        let state_before = self.scalar_state();
        let mut raw = Vec::new();
        match self.execute_one(pc_before, &mut raw) {
            Ok(mnemonic) => Ok(StepTrace {
                pc_before,
                pc_after: self.regs[REGISTER_PC],
                raw,
                mnemonic: mnemonic.to_owned(),
                state_before,
                state_after: self.scalar_state(),
            }),
            Err(error) => {
                *self = checkpoint;
                Err(error)
            }
        }
    }

    /// Execute the installed program transactionally with a bounded trace.
    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, VaxError> {
        if max_steps > MAX_RETAINED_TRACES {
            return Err(VaxError::TraceLimitTooLarge {
                requested: max_steps,
                maximum: MAX_RETAINED_TRACES,
            });
        }
        let checkpoint = self.clone();
        let mut traces = Vec::with_capacity(max_steps.min(1024));
        while !self.halted {
            if traces.len() == max_steps {
                *self = checkpoint;
                return Err(VaxError::StepLimitExceeded { max_steps });
            }
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    *self = checkpoint;
                    return Err(error);
                }
            }
        }
        Ok(ExecutionResult {
            halted: true,
            steps: traces.len(),
            traces,
            final_state: self.get_state(),
        })
    }

    /// Reset, load, and execute a program transactionally.
    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, VaxError> {
        let checkpoint = self.clone();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                *self = checkpoint;
                Err(error)
            }
        }
    }

    fn scalar_state(&self) -> ScalarState {
        ScalarState {
            regs: self.regs,
            psl: self.psl,
            halted: self.halted,
        }
    }

    fn pc_is_loaded(&self, pc: u32) -> bool {
        let Some(offset) = pc.checked_sub(self.loaded_origin) else {
            return false;
        };
        usize::try_from(offset).is_ok_and(|offset| offset < self.loaded_len)
    }

    fn fetch_byte(&mut self, start: u32, raw: &mut Vec<u8>) -> Result<u8, VaxError> {
        let pc = self.regs[REGISTER_PC];
        if !self.pc_is_loaded(pc) {
            return Err(VaxError::TruncatedInstruction { pc: start });
        }
        let byte = self.read_byte_checked(pc)?;
        self.regs[REGISTER_PC] = pc.wrapping_add(1);
        raw.push(byte);
        Ok(byte)
    }

    fn fetch_unsigned(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        width: usize,
    ) -> Result<u32, VaxError> {
        let mut bytes = [0_u8; 4];
        for byte in &mut bytes[..width] {
            *byte = self.fetch_byte(start, raw)?;
        }
        Ok(u32::from_le_bytes(bytes))
    }

    fn decode_operand(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        width: usize,
        indexed: bool,
    ) -> Result<Operand, VaxError> {
        let specifier = self.fetch_byte(start, raw)?;
        let mode = specifier >> 4;
        let register = usize::from(specifier & 0x0f);
        match mode {
            0..=3 => Ok(Operand::Value {
                value: u32::from(specifier),
                specifier,
            }),
            4 => {
                if indexed {
                    return Err(VaxError::RecursiveIndexMode { pc: start });
                }
                if register == REGISTER_PC {
                    return Err(VaxError::ReservedAddressingMode {
                        specifier,
                        pc: start,
                    });
                }
                let base = self.decode_operand(start, raw, width, true)?;
                let address = match base {
                    Operand::Place {
                        place: Place::Memory(address),
                        ..
                    } => address,
                    _ => {
                        return Err(VaxError::ReservedAddressingMode {
                            specifier,
                            pc: start,
                        });
                    }
                };
                Ok(Operand::Place {
                    place: Place::Memory(
                        address.wrapping_add(self.regs[register].wrapping_mul(width as u32)),
                    ),
                    specifier,
                })
            }
            5 => Ok(Operand::Place {
                place: Place::Register(register),
                specifier,
            }),
            6 => Ok(Operand::Place {
                place: Place::Memory(self.regs[register]),
                specifier,
            }),
            7 => {
                if register == REGISTER_PC {
                    return Err(VaxError::ReservedAddressingMode {
                        specifier,
                        pc: start,
                    });
                }
                self.regs[register] = self.regs[register].wrapping_sub(width as u32);
                Ok(Operand::Place {
                    place: Place::Memory(self.regs[register]),
                    specifier,
                })
            }
            8 if register == REGISTER_PC => Ok(Operand::Value {
                value: self.fetch_unsigned(start, raw, width)?,
                specifier,
            }),
            8 => {
                let address = self.regs[register];
                self.regs[register] = address.wrapping_add(width as u32);
                Ok(Operand::Place {
                    place: Place::Memory(address),
                    specifier,
                })
            }
            9 if register == REGISTER_PC => Ok(Operand::Place {
                place: Place::Memory(self.fetch_unsigned(start, raw, 4)?),
                specifier,
            }),
            9 => {
                let pointer_address = self.regs[register];
                self.regs[register] = pointer_address.wrapping_add(4);
                Ok(Operand::Place {
                    place: Place::Memory(self.read_long_checked(pointer_address)?),
                    specifier,
                })
            }
            10..=15 => {
                let displacement_width = match mode {
                    10 | 11 => 1,
                    12 | 13 => 2,
                    _ => 4,
                };
                let raw_displacement = self.fetch_unsigned(start, raw, displacement_width)?;
                let displacement = sign_extend(raw_displacement, displacement_width);
                let base = self.regs[register];
                let intermediate = base.wrapping_add(displacement);
                let address = if mode & 1 == 1 {
                    self.read_long_checked(intermediate)?
                } else {
                    intermediate
                };
                Ok(Operand::Place {
                    place: Place::Memory(address),
                    specifier,
                })
            }
            _ => unreachable!(),
        }
    }

    fn read_operand(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        width: usize,
    ) -> Result<u32, VaxError> {
        let operand = self.decode_operand(start, raw, width, false)?;
        self.read_decoded(operand, width)
    }

    fn decode_destination(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        width: usize,
    ) -> Result<Place, VaxError> {
        match self.decode_operand(start, raw, width, false)? {
            Operand::Place { place, .. } => Ok(place),
            Operand::Value { specifier, .. } => Err(VaxError::InvalidDestinationMode {
                specifier,
                pc: start,
            }),
        }
    }

    fn read_decoded(&self, operand: Operand, width: usize) -> Result<u32, VaxError> {
        match operand {
            Operand::Value { value, .. } => Ok(value & width_mask(width)),
            Operand::Place { place, .. } => self.read_place(place, width),
        }
    }

    fn read_place(&self, place: Place, width: usize) -> Result<u32, VaxError> {
        match (place, width) {
            (Place::Register(index), _) => Ok(self.regs[index] & width_mask(width)),
            (Place::Memory(address), 1) => Ok(u32::from(self.read_byte_checked(address)?)),
            (Place::Memory(address), 2) => Ok(u32::from(self.read_word_checked(address)?)),
            (Place::Memory(address), 4) => self.read_long_checked(address),
            _ => unreachable!("validated VAX operand width"),
        }
    }

    fn write_place(&mut self, place: Place, width: usize, value: u32) -> Result<(), VaxError> {
        match (place, width) {
            (Place::Register(index), _) => self.write_register_width_checked(index, width, value),
            (Place::Memory(address), 1) => self.write_byte_checked(address, value as u8),
            (Place::Memory(address), 2) => self.write_word_checked(address, value as u16),
            (Place::Memory(address), 4) => self.write_long_checked(address, value),
            _ => unreachable!("validated VAX operand width"),
        }
    }

    fn decode_address(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        width: usize,
    ) -> Result<u32, VaxError> {
        match self.decode_operand(start, raw, width, false)? {
            Operand::Place {
                place: Place::Memory(address),
                ..
            } => Ok(address),
            Operand::Place { specifier, .. } | Operand::Value { specifier, .. } => {
                Err(VaxError::InvalidDestinationMode {
                    specifier,
                    pc: start,
                })
            }
        }
    }

    fn push_long(&mut self, value: u32) -> Result<(), VaxError> {
        let sp = self.regs[REGISTER_SP].wrapping_sub(4);
        if checked_range(sp, 4).is_err() {
            return Err(VaxError::StackOutOfRange { sp });
        }
        self.write_long_checked(sp, value)?;
        self.regs[REGISTER_SP] = sp;
        Ok(())
    }

    fn pop_long(&mut self) -> Result<u32, VaxError> {
        let sp = self.regs[REGISTER_SP];
        let value = self
            .read_long_checked(sp)
            .map_err(|_| VaxError::StackOutOfRange { sp })?;
        self.regs[REGISTER_SP] = sp.wrapping_add(4);
        Ok(value)
    }

    fn set_nz(&mut self, value: u32, width: usize) {
        let masked = value & width_mask(width);
        self.psl.n = masked & sign_bit(width) != 0;
        self.psl.z = masked == 0;
    }

    fn execute_one(&mut self, start: u32, raw: &mut Vec<u8>) -> Result<&'static str, VaxError> {
        let opcode = self.fetch_byte(start, raw)?;
        if opcode >= 0xfc {
            return Err(VaxError::UnsupportedTwoByteOpcode {
                prefix: opcode,
                pc: start,
            });
        }
        match opcode {
            0x00 => {
                self.halted = true;
                Ok("HALT")
            }
            0x01 => Ok("NOP"),
            0x05 => {
                self.regs[REGISTER_PC] = self.pop_long()?;
                Ok("RSB")
            }
            0x10 => self.execute_branch(start, raw, 1, true, true, "BSBB"),
            0x11 => self.execute_branch(start, raw, 1, true, false, "BRB"),
            0x12..=0x1f if opcode != 0x16 && opcode != 0x17 => {
                let condition = self.branch_condition(opcode);
                self.execute_branch(start, raw, 1, condition, false, branch_name(opcode))
            }
            0x16 => {
                let address = self.decode_address(start, raw, 1)?;
                let return_pc = self.regs[REGISTER_PC];
                self.push_long(return_pc)?;
                self.regs[REGISTER_PC] = address;
                Ok("JSB")
            }
            0x17 => {
                self.regs[REGISTER_PC] = self.decode_address(start, raw, 1)?;
                Ok("JMP")
            }
            0x30 => self.execute_branch(start, raw, 2, true, true, "BSBW"),
            0x31 => self.execute_branch(start, raw, 2, true, false, "BRW"),
            0x3c => self.execute_movz(start, raw, 2, "MOVZWL"),
            0x3e => self.execute_address(start, raw, 2, false, "MOVAW"),
            0x3f => self.execute_address(start, raw, 2, true, "PUSHAW"),
            0x80..=0x8f => self.execute_arithmetic_family(start, raw, opcode, 1),
            0x90..=0x97 => self.execute_data_family(start, raw, opcode, 1),
            0x9a => self.execute_movz(start, raw, 1, "MOVZBL"),
            0x9b => self.execute_movz(start, raw, 2, "MOVZBW"),
            0x9e => self.execute_address(start, raw, 1, false, "MOVAB"),
            0x9f => self.execute_address(start, raw, 1, true, "PUSHAB"),
            0xa0..=0xaf => self.execute_arithmetic_family(start, raw, opcode, 2),
            0xb0..=0xb7 => self.execute_data_family(start, raw, opcode, 2),
            0xc0..=0xcf => self.execute_arithmetic_family(start, raw, opcode, 4),
            0xd0..=0xd7 => self.execute_data_family(start, raw, opcode, 4),
            0xdd => {
                let value = self.read_operand(start, raw, 4)?;
                self.push_long(value)?;
                Ok("PUSHL")
            }
            0xde => self.execute_address(start, raw, 4, false, "MOVAL"),
            0xdf => self.execute_address(start, raw, 4, true, "PUSHAL"),
            _ => Err(VaxError::UnknownOpcode { opcode, pc: start }),
        }
    }

    fn execute_branch(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        width: usize,
        condition: bool,
        subroutine: bool,
        name: &'static str,
    ) -> Result<&'static str, VaxError> {
        let displacement = sign_extend(self.fetch_unsigned(start, raw, width)?, width);
        let base = self.regs[REGISTER_PC];
        if subroutine {
            self.push_long(base)?;
        }
        if condition {
            self.regs[REGISTER_PC] = base.wrapping_add(displacement);
        }
        Ok(name)
    }

    fn branch_condition(&self, opcode: u8) -> bool {
        match opcode {
            0x12 => !self.psl.z,
            0x13 => self.psl.z,
            0x14 => !(self.psl.n || self.psl.z),
            0x15 => self.psl.n || self.psl.z,
            0x18 => !self.psl.n,
            0x19 => self.psl.n,
            0x1a => !(self.psl.c || self.psl.z),
            0x1b => self.psl.c || self.psl.z,
            0x1c => !self.psl.v,
            0x1d => self.psl.v,
            0x1e => !self.psl.c,
            0x1f => self.psl.c,
            _ => false,
        }
    }

    fn execute_arithmetic_family(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        opcode: u8,
        width: usize,
    ) -> Result<&'static str, VaxError> {
        let operation = opcode & 0x0f;
        if operation == 0x0f {
            return self.execute_case(start, raw, width);
        }
        if operation == 0x0e {
            let source = self.read_operand(start, raw, width)?;
            let destination = self.decode_destination(start, raw, width)?;
            let mask = width_mask(width);
            let result = (0_u32).wrapping_sub(source) & mask;
            self.write_place(destination, width, result)?;
            self.set_nz(result, width);
            self.psl.v = source == sign_bit(width);
            self.psl.c = result != 0;
            return Ok(width_name("MNEG", width));
        }
        let three_operand = operation & 1 == 1;
        let family = operation >> 1;
        let first = self.read_operand(start, raw, width)?;
        let (second, destination) = if three_operand {
            let second = self.read_operand(start, raw, width)?;
            let destination = self.decode_destination(start, raw, width)?;
            (second, destination)
        } else {
            let operand = self.decode_operand(start, raw, width, false)?;
            let destination = match operand {
                Operand::Place { place, .. } => place,
                Operand::Value { specifier, .. } => {
                    return Err(VaxError::InvalidDestinationMode {
                        specifier,
                        pc: start,
                    });
                }
            };
            (self.read_place(destination, width)?, destination)
        };
        let mask = width_mask(width);
        let (result, name) = match family {
            0 => {
                let result = second.wrapping_add(first) & mask;
                self.psl.c = u64::from(second & mask) + u64::from(first & mask) > u64::from(mask);
                self.psl.v = signed_add_overflow(second, first, result, width);
                (
                    result,
                    width_name(if three_operand { "ADD3" } else { "ADD2" }, width),
                )
            }
            1 => {
                let result = second.wrapping_sub(first) & mask;
                self.psl.c = second & mask < first & mask;
                self.psl.v = signed_sub_overflow(second, first, result, width);
                (
                    result,
                    width_name(if three_operand { "SUB3" } else { "SUB2" }, width),
                )
            }
            2 => {
                let result = first.wrapping_mul(second) & mask;
                self.psl.v = signed_product_overflow(first, second, width);
                self.psl.c = false;
                (
                    result,
                    width_name(if three_operand { "MUL3" } else { "MUL2" }, width),
                )
            }
            3 => {
                if first & mask == 0 {
                    return Err(VaxError::DivideByZero { pc: start });
                }
                let dividend = signed_value(second, width);
                let divisor = signed_value(first, width);
                let quotient = dividend.checked_div(divisor).unwrap_or(dividend);
                let min = -(1_i64 << (width * 8 - 1));
                let max = (1_i64 << (width * 8 - 1)) - 1;
                self.psl.v = quotient < min || quotient > max;
                self.psl.c = false;
                (
                    quotient as u32 & mask,
                    width_name(if three_operand { "DIV3" } else { "DIV2" }, width),
                )
            }
            4 => {
                self.psl.v = false;
                (
                    second | first,
                    width_name(if three_operand { "BIS3" } else { "BIS2" }, width),
                )
            }
            5 => {
                self.psl.v = false;
                (
                    second & !first,
                    width_name(if three_operand { "BIC3" } else { "BIC2" }, width),
                )
            }
            6 => {
                self.psl.v = false;
                (
                    second ^ first,
                    width_name(if three_operand { "XOR3" } else { "XOR2" }, width),
                )
            }
            _ => unreachable!(),
        };
        self.write_place(destination, width, result)?;
        self.set_nz(result, width);
        Ok(name)
    }

    fn execute_case(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        width: usize,
    ) -> Result<&'static str, VaxError> {
        let selector = self.read_operand(start, raw, width)?;
        let base = self.read_operand(start, raw, width)?;
        let limit = self.read_operand(start, raw, width)?;
        let index = selector.wrapping_sub(base) & width_mask(width);
        self.psl.n = index & sign_bit(width) != 0;
        self.psl.z = index == 0;
        self.psl.v = false;
        self.psl.c = index > limit;
        if index <= limit {
            let table = self.regs[REGISTER_PC];
            let entry = table.wrapping_add(index.wrapping_mul(2));
            let displacement = i32::from(self.read_word_checked(entry)? as i16) as u32;
            self.regs[REGISTER_PC] = table.wrapping_add(displacement);
        }
        Ok(width_name("CASE", width))
    }

    fn execute_data_family(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        opcode: u8,
        width: usize,
    ) -> Result<&'static str, VaxError> {
        let operation = opcode & 7;
        match operation {
            0 => {
                let value = self.read_operand(start, raw, width)?;
                let destination = self.decode_destination(start, raw, width)?;
                self.write_place(destination, width, value)?;
                self.set_nz(value, width);
                self.psl.v = false;
                Ok(width_name("MOV", width))
            }
            1 => {
                let first = self.read_operand(start, raw, width)?;
                let second = self.read_operand(start, raw, width)?;
                self.psl.n = signed_value(first, width) < signed_value(second, width);
                self.psl.z = first & width_mask(width) == second & width_mask(width);
                self.psl.v = false;
                self.psl.c = first & width_mask(width) < second & width_mask(width);
                Ok(width_name("CMP", width))
            }
            3 => {
                let mask = self.read_operand(start, raw, width)?;
                let source = self.read_operand(start, raw, width)?;
                self.set_nz(source & mask, width);
                self.psl.v = false;
                Ok(width_name("BIT", width))
            }
            2 => {
                let source = self.read_operand(start, raw, width)?;
                let destination = self.decode_destination(start, raw, width)?;
                let result = !source & width_mask(width);
                self.write_place(destination, width, result)?;
                self.set_nz(result, width);
                self.psl.v = false;
                Ok(width_name("MCOM", width))
            }
            4 => {
                let destination = self.decode_destination(start, raw, width)?;
                self.write_place(destination, width, 0)?;
                self.set_nz(0, width);
                self.psl.v = false;
                Ok(width_name("CLR", width))
            }
            5 => {
                let value = self.read_operand(start, raw, width)?;
                self.set_nz(value, width);
                self.psl.v = false;
                self.psl.c = false;
                Ok(width_name("TST", width))
            }
            6 | 7 => {
                let operand = self.decode_operand(start, raw, width, false)?;
                let destination = match operand {
                    Operand::Place { place, .. } => place,
                    Operand::Value { specifier, .. } => {
                        return Err(VaxError::InvalidDestinationMode {
                            specifier,
                            pc: start,
                        });
                    }
                };
                let old = self.read_place(destination, width)?;
                let result = if operation == 6 {
                    old.wrapping_add(1)
                } else {
                    old.wrapping_sub(1)
                } & width_mask(width);
                self.write_place(destination, width, result)?;
                self.set_nz(result, width);
                self.psl.v = if operation == 6 {
                    old & width_mask(width) == sign_bit(width) - 1
                } else {
                    old & width_mask(width) == sign_bit(width)
                };
                Ok(width_name(
                    if operation == 6 { "INC" } else { "DEC" },
                    width,
                ))
            }
            _ => unreachable!(),
        }
    }

    fn execute_movz(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        source_width: usize,
        name: &'static str,
    ) -> Result<&'static str, VaxError> {
        let source = self.read_operand(start, raw, source_width)?;
        let destination = self.decode_destination(start, raw, 4)?;
        self.write_place(destination, 4, source)?;
        self.set_nz(source, 4);
        self.psl.n = false;
        self.psl.v = false;
        Ok(name)
    }

    fn execute_address(
        &mut self,
        start: u32,
        raw: &mut Vec<u8>,
        source_width: usize,
        push: bool,
        name: &'static str,
    ) -> Result<&'static str, VaxError> {
        let address = self.decode_address(start, raw, source_width)?;
        if push {
            self.push_long(address)?;
        } else {
            let destination = self.decode_destination(start, raw, 4)?;
            self.write_place(destination, 4, address)?;
            self.set_nz(address, 4);
            self.psl.v = false;
        }
        Ok(name)
    }
}

fn validate_state(state: &VaxState) -> Result<(), VaxError> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(VaxError::InvalidState(format!(
            "memory has {} bytes; expected {MEMORY_SIZE}",
            state.memory.len()
        )));
    }
    let start = usize::try_from(state.loaded_origin)
        .map_err(|_| VaxError::InvalidState("loaded origin is outside memory".to_owned()))?;
    if start
        .checked_add(state.loaded_len)
        .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(VaxError::InvalidState(
            "installed program range is outside memory".to_owned(),
        ));
    }
    Ok(())
}

fn checked_range(address: u32, width: usize) -> Result<usize, VaxError> {
    let start =
        usize::try_from(address).map_err(|_| VaxError::MemoryOutOfRange { address, width })?;
    if start.checked_add(width).is_none_or(|end| end > MEMORY_SIZE) {
        return Err(VaxError::MemoryOutOfRange { address, width });
    }
    Ok(start)
}

const fn width_mask(width: usize) -> u32 {
    match width {
        1 => 0xff,
        2 => 0xffff,
        4 => u32::MAX,
        _ => unreachable!(),
    }
}

const fn sign_bit(width: usize) -> u32 {
    1_u32 << (width * 8 - 1)
}

const fn merge_width(previous: u32, value: u32, width: usize) -> u32 {
    let mask = width_mask(width);
    previous & !mask | value & mask
}

const fn sign_extend(value: u32, width: usize) -> u32 {
    let mask = width_mask(width);
    let value = value & mask;
    if value & sign_bit(width) != 0 {
        value | !mask
    } else {
        value
    }
}

const fn signed_value(value: u32, width: usize) -> i64 {
    sign_extend(value, width) as i32 as i64
}

const fn signed_add_overflow(lhs: u32, rhs: u32, result: u32, width: usize) -> bool {
    let sign = sign_bit(width);
    (lhs ^ result) & (rhs ^ result) & sign != 0
}

const fn signed_sub_overflow(lhs: u32, rhs: u32, result: u32, width: usize) -> bool {
    let sign = sign_bit(width);
    (lhs ^ rhs) & (lhs ^ result) & sign != 0
}

fn signed_product_overflow(lhs: u32, rhs: u32, width: usize) -> bool {
    let product = i128::from(signed_value(lhs, width)) * i128::from(signed_value(rhs, width));
    let min = -(1_i128 << (width * 8 - 1));
    let max = (1_i128 << (width * 8 - 1)) - 1;
    product < min || product > max
}

fn width_name(base: &str, width: usize) -> &'static str {
    match (base, width) {
        ("ADD2", 1) => "ADDB2",
        ("ADD2", 2) => "ADDW2",
        ("ADD2", 4) => "ADDL2",
        ("ADD3", 1) => "ADDB3",
        ("ADD3", 2) => "ADDW3",
        ("ADD3", 4) => "ADDL3",
        ("SUB2", 1) => "SUBB2",
        ("SUB2", 2) => "SUBW2",
        ("SUB2", 4) => "SUBL2",
        ("SUB3", 1) => "SUBB3",
        ("SUB3", 2) => "SUBW3",
        ("SUB3", 4) => "SUBL3",
        ("MUL2", 1) => "MULB2",
        ("MUL2", 2) => "MULW2",
        ("MUL2", 4) => "MULL2",
        ("MUL3", 1) => "MULB3",
        ("MUL3", 2) => "MULW3",
        ("MUL3", 4) => "MULL3",
        ("DIV2", 1) => "DIVB2",
        ("DIV2", 2) => "DIVW2",
        ("DIV2", 4) => "DIVL2",
        ("DIV3", 1) => "DIVB3",
        ("DIV3", 2) => "DIVW3",
        ("DIV3", 4) => "DIVL3",
        ("BIS2", 1) => "BISB2",
        ("BIS2", 2) => "BISW2",
        ("BIS2", 4) => "BISL2",
        ("BIS3", 1) => "BISB3",
        ("BIS3", 2) => "BISW3",
        ("BIS3", 4) => "BISL3",
        ("BIC2", 1) => "BICB2",
        ("BIC2", 2) => "BICW2",
        ("BIC2", 4) => "BICL2",
        ("BIC3", 1) => "BICB3",
        ("BIC3", 2) => "BICW3",
        ("BIC3", 4) => "BICL3",
        ("XOR2", 1) => "XORB2",
        ("XOR2", 2) => "XORW2",
        ("XOR2", 4) => "XORL2",
        ("XOR3", 1) => "XORB3",
        ("XOR3", 2) => "XORW3",
        ("XOR3", 4) => "XORL3",
        ("MNEG", 1) => "MNEGB",
        ("MNEG", 2) => "MNEGW",
        ("MNEG", 4) => "MNEGL",
        ("CASE", 1) => "CASEB",
        ("CASE", 2) => "CASEW",
        ("CASE", 4) => "CASEL",
        ("MOV", 1) => "MOVB",
        ("MOV", 2) => "MOVW",
        ("MOV", 4) => "MOVL",
        ("CMP", 1) => "CMPB",
        ("CMP", 2) => "CMPW",
        ("CMP", 4) => "CMPL",
        ("MCOM", 1) => "MCOMB",
        ("MCOM", 2) => "MCOMW",
        ("MCOM", 4) => "MCOML",
        ("BIT", 1) => "BITB",
        ("BIT", 2) => "BITW",
        ("BIT", 4) => "BITL",
        ("CLR", 1) => "CLRB",
        ("CLR", 2) => "CLRW",
        ("CLR", 4) => "CLRL",
        ("TST", 1) => "TSTB",
        ("TST", 2) => "TSTW",
        ("TST", 4) => "TSTL",
        ("INC", 1) => "INCB",
        ("INC", 2) => "INCW",
        ("INC", 4) => "INCL",
        ("DEC", 1) => "DECB",
        ("DEC", 2) => "DECW",
        ("DEC", 4) => "DECL",
        _ => "UNKNOWN",
    }
}

const fn branch_name(opcode: u8) -> &'static str {
    match opcode {
        0x12 => "BNEQ",
        0x13 => "BEQL",
        0x14 => "BGTR",
        0x15 => "BLEQ",
        0x18 => "BGEQ",
        0x19 => "BLSS",
        0x1a => "BGTRU",
        0x1b => "BLEQU",
        0x1c => "BVC",
        0x1d => "BVS",
        0x1e => "BGEQU",
        0x1f => "BLSSU",
        _ => "BRANCH",
    }
}
