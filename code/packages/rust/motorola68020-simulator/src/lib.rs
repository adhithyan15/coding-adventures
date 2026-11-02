//! Checked bounded Motorola MC68020 functional simulator (Spec 07at).
//!
//! The public API owns MC68020 state. A private translation bridge reuses the
//! checked Layer 07n MC68000 implementation for the explicitly claimed
//! inherited register-only subset; no MC68000 type leaks through this API.

use std::fmt;

/// Bounded storage installed by this first MC68020 cell (16 MiB).
pub const MEMORY_SIZE: usize = 16 * 1024 * 1024;
/// Default program origin.
pub const LOAD_ADDRESS: u32 = 0x1000;
/// Reset user stack pointer.
pub const INITIAL_SP: u32 = 0x00f000;

const SR_S: u16 = 0x2000;
const SR_M: u16 = 0x1000;
const CCR_X: u16 = 0x10;
const CCR_N: u16 = 0x08;
const CCR_Z: u16 = 0x04;
const CCR_V: u16 = 0x02;
const CCR_C: u16 = 0x01;

/// Complete owned architectural state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mc68020State {
    pub d: [u32; 8],
    pub a: [u32; 8],
    pub pc: u32,
    pub sr: u16,
    pub usp: u32,
    pub isp: u32,
    pub msp: u32,
    pub halted: bool,
    pub memory: Vec<u8>,
    pub installed_origin: u32,
    pub installed_bytes: u32,
}

impl Mc68020State {
    pub fn x(&self) -> bool {
        self.sr & CCR_X != 0
    }
    pub fn n(&self) -> bool {
        self.sr & CCR_N != 0
    }
    pub fn z(&self) -> bool {
        self.sr & CCR_Z != 0
    }
    pub fn v(&self) -> bool {
        self.sr & CCR_V != 0
    }
    pub fn c(&self) -> bool {
        self.sr & CCR_C != 0
    }
}

/// One complete successful instruction transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: Vec<u8>,
    pub width: u32,
    pub mnemonic: String,
    pub effect: String,
    pub state_before: Mc68020State,
    pub state_after: Mc68020State,
}

/// A bounded run result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: Mc68020State,
    pub traces: Vec<StepTrace>,
}

/// Typed fail-closed simulator error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mc68020Error {
    EmptyProgram,
    MisalignedProgram {
        origin: u32,
    },
    ProgramOutOfRange {
        origin: u32,
        size: usize,
    },
    InvalidState(String),
    InvalidRegister {
        bank: char,
        index: usize,
    },
    Halted,
    FetchOutsideProgram {
        pc: u32,
    },
    MisalignedFetch {
        pc: u32,
    },
    TruncatedInstruction {
        pc: u32,
        needed: usize,
        available: usize,
    },
    UnknownOpcode {
        opcode: u16,
    },
    UnsupportedOpcode {
        opcode: u16,
        reason: &'static str,
    },
    InvalidEffectiveAddress {
        mode: u8,
        register: u8,
        reason: &'static str,
    },
    StorageOutOfRange {
        address: u32,
        size: usize,
    },
    Trap {
        condition: u8,
        pc: u32,
    },
    MaxStepsZero,
    MaxStepsExceeded {
        max_steps: usize,
    },
    Inherited(String),
}

impl fmt::Display for Mc68020Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => f.write_str("program is empty"),
            Self::MisalignedProgram { origin } => write!(f, "program origin {origin:#010x} is odd"),
            Self::ProgramOutOfRange { origin, size } => write!(
                f,
                "program of {size} bytes at {origin:#010x} exceeds storage"
            ),
            Self::InvalidState(message) => write!(f, "invalid state: {message}"),
            Self::InvalidRegister { bank, index } => write!(f, "invalid {bank}{index} register"),
            Self::Halted => f.write_str("CPU is halted"),
            Self::FetchOutsideProgram { pc } => {
                write!(f, "fetch at {pc:#010x} is outside the installed program")
            }
            Self::MisalignedFetch { pc } => write!(f, "instruction fetch at {pc:#010x} is odd"),
            Self::TruncatedInstruction {
                pc,
                needed,
                available,
            } => write!(
                f,
                "instruction at {pc:#010x} needs {needed} bytes but only {available} remain"
            ),
            Self::UnknownOpcode { opcode } => write!(f, "unknown opcode {opcode:#06x}"),
            Self::UnsupportedOpcode { opcode, reason } => {
                write!(f, "unsupported opcode {opcode:#06x}: {reason}")
            }
            Self::InvalidEffectiveAddress {
                mode,
                register,
                reason,
            } => write!(f, "invalid effective address {mode}/{register}: {reason}"),
            Self::StorageOutOfRange { address, size } => {
                write!(f, "{size}-byte access at {address:#010x} exceeds storage")
            }
            Self::Trap { condition, pc } => {
                write!(f, "TRAPcc condition {condition:#x} is true at {pc:#010x}")
            }
            Self::MaxStepsZero => f.write_str("max_steps must be greater than zero"),
            Self::MaxStepsExceeded { max_steps } => {
                write!(f, "program did not halt within {max_steps} steps")
            }
            Self::Inherited(message) => write!(f, "inherited MC68000 execution failed: {message}"),
        }
    }
}

impl std::error::Error for Mc68020Error {}

/// Checked MC68020 machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mc68020Simulator {
    state: Mc68020State,
}

#[derive(Debug, Clone)]
struct Cursor {
    start: u32,
    next: u32,
    raw: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
enum Destination {
    D(usize),
    A(usize),
    Memory(u32),
}

impl Default for Mc68020Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Mc68020Simulator {
    /// Construct a reset problem-state machine.
    pub fn new() -> Self {
        let mut a = [0; 8];
        a[7] = INITIAL_SP;
        Self {
            state: Mc68020State {
                d: [0; 8],
                a,
                pc: LOAD_ADDRESS,
                sr: 0,
                usp: INITIAL_SP,
                isp: INITIAL_SP,
                msp: INITIAL_SP,
                halted: false,
                memory: vec![0; MEMORY_SIZE],
                installed_origin: 0,
                installed_bytes: 0,
            },
        }
    }

    /// Return a complete owned snapshot.
    pub fn state(&self) -> Mc68020State {
        self.state.clone()
    }

    /// Atomically restore a validated complete snapshot.
    pub fn restore(&mut self, state: &Mc68020State) -> Result<(), Mc68020Error> {
        Self::validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    fn validate_state(state: &Mc68020State) -> Result<(), Mc68020Error> {
        if state.memory.len() != MEMORY_SIZE {
            return Err(Mc68020Error::InvalidState(format!(
                "memory has {} bytes, expected {MEMORY_SIZE}",
                state.memory.len()
            )));
        }
        let start = state.installed_origin as usize;
        let end = start.checked_add(state.installed_bytes as usize);
        if start > MEMORY_SIZE || end.is_none_or(|end| end > MEMORY_SIZE) {
            return Err(Mc68020Error::InvalidState(
                "installed program range exceeds storage".into(),
            ));
        }
        let bank = if state.sr & SR_S == 0 {
            state.usp
        } else if state.sr & SR_M == 0 {
            state.isp
        } else {
            state.msp
        };
        if state.a[7] != bank {
            return Err(Mc68020Error::InvalidState(
                "A7 does not match the SR-selected stack bank".into(),
            ));
        }
        Ok(())
    }

    /// Clear all state and storage.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Atomically reset and install exact big-endian bytes.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), Mc68020Error> {
        if program.is_empty() {
            return Err(Mc68020Error::EmptyProgram);
        }
        if origin & 1 != 0 {
            return Err(Mc68020Error::MisalignedProgram { origin });
        }
        let end = (origin as usize).checked_add(program.len());
        if origin as usize > MEMORY_SIZE || end.is_none_or(|end| end > MEMORY_SIZE) {
            return Err(Mc68020Error::ProgramOutOfRange {
                origin,
                size: program.len(),
            });
        }
        let mut next = Self::new();
        next.state.memory[origin as usize..end.expect("range checked")].copy_from_slice(program);
        next.state.pc = origin;
        next.state.installed_origin = origin;
        next.state.installed_bytes = program.len() as u32;
        *self = next;
        Ok(())
    }

    /// Read a data register.
    pub fn data_register(&self, index: usize) -> Result<u32, Mc68020Error> {
        self.state
            .d
            .get(index)
            .copied()
            .ok_or(Mc68020Error::InvalidRegister { bank: 'D', index })
    }

    /// Atomically write a data register.
    pub fn set_data_register(&mut self, index: usize, value: u32) -> Result<(), Mc68020Error> {
        let slot = self
            .state
            .d
            .get_mut(index)
            .ok_or(Mc68020Error::InvalidRegister { bank: 'D', index })?;
        *slot = value;
        Ok(())
    }

    /// Read an address register.
    pub fn address_register(&self, index: usize) -> Result<u32, Mc68020Error> {
        self.state
            .a
            .get(index)
            .copied()
            .ok_or(Mc68020Error::InvalidRegister { bank: 'A', index })
    }

    /// Atomically write an address register and synchronize A7's selected bank.
    pub fn set_address_register(&mut self, index: usize, value: u32) -> Result<(), Mc68020Error> {
        if index >= 8 {
            return Err(Mc68020Error::InvalidRegister { bank: 'A', index });
        }
        self.set_a(index, value);
        Ok(())
    }

    /// Read a checked byte.
    pub fn read_byte(&self, address: u32) -> Result<u8, Mc68020Error> {
        self.check_data_range(address, 1)?;
        Ok(self.state.memory[address as usize])
    }

    /// Atomically write a checked byte.
    pub fn write_byte(&mut self, address: u32, value: u8) -> Result<(), Mc68020Error> {
        self.check_data_range(address, 1)?;
        self.state.memory[address as usize] = value;
        Ok(())
    }

    /// Read a checked big-endian word.
    pub fn read_word(&self, address: u32) -> Result<u16, Mc68020Error> {
        self.check_data_range(address, 2)?;
        let i = address as usize;
        Ok(u16::from_be_bytes([
            self.state.memory[i],
            self.state.memory[i + 1],
        ]))
    }

    /// Atomically write a checked big-endian word.
    pub fn write_word(&mut self, address: u32, value: u16) -> Result<(), Mc68020Error> {
        self.check_data_range(address, 2)?;
        let i = address as usize;
        self.state.memory[i..i + 2].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Read a checked big-endian longword.
    pub fn read_long(&self, address: u32) -> Result<u32, Mc68020Error> {
        self.check_data_range(address, 4)?;
        let i = address as usize;
        Ok(u32::from_be_bytes(
            self.state.memory[i..i + 4].try_into().expect("four bytes"),
        ))
    }

    /// Atomically write a checked big-endian longword.
    pub fn write_long(&mut self, address: u32, value: u32) -> Result<(), Mc68020Error> {
        self.check_data_range(address, 4)?;
        let i = address as usize;
        self.state.memory[i..i + 4].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    fn check_data_range(&self, address: u32, size: usize) -> Result<(), Mc68020Error> {
        let end = (address as usize).checked_add(size);
        if address as usize >= MEMORY_SIZE || end.is_none_or(|end| end > MEMORY_SIZE) {
            Err(Mc68020Error::StorageOutOfRange { address, size })
        } else {
            Ok(())
        }
    }

    fn set_a(&mut self, index: usize, value: u32) {
        self.state.a[index] = value;
        if index == 7 {
            if self.state.sr & SR_S == 0 {
                self.state.usp = value;
            } else if self.state.sr & SR_M == 0 {
                self.state.isp = value;
            } else {
                self.state.msp = value;
            }
        }
    }

    /// Execute one instruction atomically.
    pub fn step(&mut self) -> Result<StepTrace, Mc68020Error> {
        if self.state.halted {
            return Err(Mc68020Error::Halted);
        }
        if self.state.pc & 1 != 0 {
            return Err(Mc68020Error::MisalignedFetch { pc: self.state.pc });
        }
        let before = self.state.clone();
        let pc_before = self.state.pc;
        let mut candidate = self.clone();
        let (raw, mnemonic, effect) = candidate.step_inner()?;
        let after = candidate.state.clone();
        let trace = StepTrace {
            pc_before,
            pc_after: after.pc,
            width: raw.len() as u32,
            raw,
            mnemonic,
            effect,
            state_before: before,
            state_after: after,
        };
        *self = candidate;
        Ok(trace)
    }

    /// Run an installed program transactionally until halt.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Mc68020Error> {
        if max_steps == 0 {
            return Err(Mc68020Error::MaxStepsZero);
        }
        let entry = self.clone();
        let mut traces = Vec::new();
        for step in 0..max_steps {
            match self.step() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    *self = entry;
                    return Err(error);
                }
            }
            if self.state.halted {
                return Ok(ExecutionResult {
                    halted: true,
                    steps: step + 1,
                    final_state: self.state(),
                    traces,
                });
            }
        }
        *self = entry;
        Err(Mc68020Error::MaxStepsExceeded { max_steps })
    }

    /// Load and run transactionally.
    pub fn execute(
        &mut self,
        program: &[u8],
        origin: u32,
        max_steps: usize,
    ) -> Result<ExecutionResult, Mc68020Error> {
        let entry = self.clone();
        self.load(program, origin)?;
        match self.run(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                *self = entry;
                Err(error)
            }
        }
    }

    fn step_inner(&mut self) -> Result<(Vec<u8>, String, String), Mc68020Error> {
        let pc = self.state.pc;
        let mut cursor = Cursor {
            start: pc,
            next: pc,
            raw: Vec::new(),
        };
        let opcode = self.fetch_word(&mut cursor)?;
        let (mnemonic, effect) = if opcode == 0x4e4f {
            self.state.pc = cursor.next;
            self.state.halted = true;
            ("TRAP #15".into(), "halt".into())
        } else if Self::is_privileged_or_unmodeled_trap(opcode) {
            return Err(Mc68020Error::UnsupportedOpcode {
                opcode,
                reason:
                    "privileged or exception-frame operation is outside this problem-state cell",
            });
        } else if opcode == 0x4e74 {
            self.execute_rtd(&mut cursor)?
        } else if opcode & 0xfff8 == 0x4808 {
            self.execute_link_long(opcode, &mut cursor)?
        } else if opcode & 0xfff8 == 0x49c0 {
            self.execute_extb(opcode, &cursor)
        } else if opcode & 0xf0f8 == 0x50f8 && matches!(opcode & 7, 2..=4) {
            self.execute_trapcc(opcode, &mut cursor)?
        } else if opcode >> 12 == 6 {
            self.execute_branch(opcode, &mut cursor)?
        } else if opcode >> 12 == 2 {
            self.execute_move_long(opcode, &mut cursor)?
        } else if Self::is_inherited(opcode) {
            self.execute_inherited(pc)?
        } else {
            return Err(Mc68020Error::UnknownOpcode { opcode });
        };
        Ok((cursor.raw, mnemonic, effect))
    }

    fn require_fetch(&self, address: u32, size: usize) -> Result<(), Mc68020Error> {
        let start = self.state.installed_origin;
        let end = start.saturating_add(self.state.installed_bytes);
        if address < start || address >= end {
            return Err(Mc68020Error::FetchOutsideProgram { pc: address });
        }
        let available = end.saturating_sub(address) as usize;
        if size > available
            || (address as usize)
                .checked_add(size)
                .is_none_or(|end| end > MEMORY_SIZE)
        {
            return Err(Mc68020Error::TruncatedInstruction {
                pc: self.state.pc,
                needed: size,
                available,
            });
        }
        Ok(())
    }

    fn fetch_word(&self, cursor: &mut Cursor) -> Result<u16, Mc68020Error> {
        let program_start = self.state.installed_origin;
        let program_end = program_start + self.state.installed_bytes;
        if cursor.next < program_start || cursor.next >= program_end {
            if cursor.next == cursor.start {
                return Err(Mc68020Error::FetchOutsideProgram { pc: cursor.next });
            }
            return Err(Mc68020Error::TruncatedInstruction {
                pc: cursor.start,
                needed: cursor.next.wrapping_sub(cursor.start) as usize + 2,
                available: program_end.saturating_sub(cursor.start) as usize,
            });
        }
        self.require_fetch(cursor.next, 2)?;
        if cursor.next & 1 != 0 {
            return Err(Mc68020Error::MisalignedFetch { pc: cursor.next });
        }
        let i = cursor.next as usize;
        cursor.next = cursor.next.wrapping_add(2);
        let bytes = [self.state.memory[i], self.state.memory[i + 1]];
        cursor.raw.extend_from_slice(&bytes);
        Ok(u16::from_be_bytes(bytes))
    }

    fn fetch_long(&self, cursor: &mut Cursor) -> Result<u32, Mc68020Error> {
        let hi = self.fetch_word(cursor)? as u32;
        let lo = self.fetch_word(cursor)? as u32;
        Ok((hi << 16) | lo)
    }

    fn execute_move_long(
        &mut self,
        opcode: u16,
        cursor: &mut Cursor,
    ) -> Result<(String, String), Mc68020Error> {
        let src_mode = ((opcode >> 3) & 7) as u8;
        let src_reg = (opcode & 7) as u8;
        let dst_mode = ((opcode >> 6) & 7) as u8;
        let dst_reg = ((opcode >> 9) & 7) as u8;
        let value = self.read_ea_long(src_mode, src_reg, cursor)?;
        let destination = self.resolve_destination(dst_mode, dst_reg, cursor)?;
        match destination {
            Destination::D(register) => {
                self.state.d[register] = value;
                self.set_move_flags(value);
            }
            Destination::A(register) => self.set_a(register, value),
            Destination::Memory(address) => {
                self.write_long(address, value)?;
                self.set_move_flags(value);
            }
        }
        self.state.pc = cursor.next;
        let mnemonic = if matches!(destination, Destination::A(_)) {
            "MOVEA.L"
        } else {
            "MOVE.L"
        };
        Ok((mnemonic.into(), format!("{value:#010x}")))
    }

    fn read_ea_long(
        &mut self,
        mode: u8,
        register: u8,
        cursor: &mut Cursor,
    ) -> Result<u32, Mc68020Error> {
        let r = register as usize;
        match mode {
            0 => Ok(self.state.d[r]),
            1 => Ok(self.state.a[r]),
            2 => self.read_long(self.state.a[r]),
            3 => {
                let address = self.state.a[r];
                let value = self.read_long(address)?;
                self.set_a(r, address.wrapping_add(4));
                Ok(value)
            }
            4 => {
                let address = self.state.a[r].wrapping_sub(4);
                let value = self.read_long(address)?;
                self.set_a(r, address);
                Ok(value)
            }
            5 => {
                let displacement = self.fetch_word(cursor)? as i16 as i32 as u32;
                self.read_long(self.state.a[r].wrapping_add(displacement))
            }
            6 => {
                let address = self.indexed_address(self.state.a[r], cursor)?;
                self.read_long(address)
            }
            7 => match register {
                0 => {
                    let address = self.fetch_word(cursor)? as i16 as i32 as u32;
                    self.read_long(address)
                }
                1 => {
                    let address = self.fetch_long(cursor)?;
                    self.read_long(address)
                }
                2 => {
                    let base = cursor.next;
                    let displacement = self.fetch_word(cursor)? as i16 as i32 as u32;
                    self.read_long(base.wrapping_add(displacement))
                }
                3 => {
                    let base = cursor.next;
                    let address = self.indexed_address(base, cursor)?;
                    self.read_long(address)
                }
                4 => self.fetch_long(cursor),
                _ => Err(self.invalid_ea(mode, register, "reserved source mode")),
            },
            _ => Err(self.invalid_ea(mode, register, "invalid source mode")),
        }
    }

    fn resolve_destination(
        &mut self,
        mode: u8,
        register: u8,
        cursor: &mut Cursor,
    ) -> Result<Destination, Mc68020Error> {
        let r = register as usize;
        match mode {
            0 => Ok(Destination::D(r)),
            1 => Ok(Destination::A(r)),
            2 => Ok(Destination::Memory(self.state.a[r])),
            3 => {
                let address = self.state.a[r];
                self.check_data_range(address, 4)?;
                self.set_a(r, address.wrapping_add(4));
                Ok(Destination::Memory(address))
            }
            4 => {
                let address = self.state.a[r].wrapping_sub(4);
                self.check_data_range(address, 4)?;
                self.set_a(r, address);
                Ok(Destination::Memory(address))
            }
            5 => {
                let d = self.fetch_word(cursor)? as i16 as i32 as u32;
                Ok(Destination::Memory(self.state.a[r].wrapping_add(d)))
            }
            6 => Ok(Destination::Memory(
                self.indexed_address(self.state.a[r], cursor)?,
            )),
            7 => match register {
                0 => Ok(Destination::Memory(
                    self.fetch_word(cursor)? as i16 as i32 as u32
                )),
                1 => Ok(Destination::Memory(self.fetch_long(cursor)?)),
                _ => Err(self.invalid_ea(mode, register, "destination is not alterable")),
            },
            _ => Err(self.invalid_ea(mode, register, "invalid destination mode")),
        }
    }

    fn invalid_ea(&self, mode: u8, register: u8, reason: &'static str) -> Mc68020Error {
        Mc68020Error::InvalidEffectiveAddress {
            mode,
            register,
            reason,
        }
    }

    fn indexed_address(&self, base: u32, cursor: &mut Cursor) -> Result<u32, Mc68020Error> {
        let extension = self.fetch_word(cursor)?;
        if extension & 0x0100 == 0 {
            let index = self.index_value(extension);
            let displacement = extension as u8 as i8 as i32 as u32;
            return Ok(base.wrapping_add(index).wrapping_add(displacement));
        }
        if extension & 0x0008 != 0 {
            return Err(self.invalid_ea(6, 0, "reserved full-extension bit 3 is set"));
        }
        let base_suppressed = extension & 0x0080 != 0;
        let index_suppressed = extension & 0x0040 != 0;
        let bd_size = (extension >> 4) & 3;
        let iis = extension & 7;
        if bd_size == 0 {
            return Err(self.invalid_ea(6, 0, "reserved base-displacement size"));
        }
        if iis == 4 || (index_suppressed && iis >= 4) {
            return Err(self.invalid_ea(6, 0, "reserved I/IS encoding"));
        }
        let base_displacement = match bd_size {
            1 => 0,
            2 => self.fetch_word(cursor)? as i16 as i32 as u32,
            3 => self.fetch_long(cursor)?,
            _ => unreachable!(),
        };
        let outer = match iis {
            2 | 6 => self.fetch_word(cursor)? as i16 as i32 as u32,
            3 | 7 => self.fetch_long(cursor)?,
            _ => 0,
        };
        let outer_active = matches!(iis, 2 | 3 | 6 | 7);
        if base_suppressed && index_suppressed && bd_size == 1 && !outer_active {
            return Err(self.invalid_ea(6, 0, "full extension suppresses every address term"));
        }
        let base_term = if base_suppressed { 0 } else { base };
        let index_term = if index_suppressed {
            0
        } else {
            self.index_value(extension)
        };
        let inner = base_term.wrapping_add(base_displacement);
        match iis {
            0 => Ok(inner.wrapping_add(index_term)),
            1..=3 => {
                let pointer = self.read_long(inner.wrapping_add(index_term))?;
                Ok(pointer.wrapping_add(outer))
            }
            5..=7 => {
                let pointer = self.read_long(inner)?;
                Ok(pointer.wrapping_add(index_term).wrapping_add(outer))
            }
            _ => Err(self.invalid_ea(6, 0, "reserved I/IS encoding")),
        }
    }

    fn index_value(&self, extension: u16) -> u32 {
        let register = ((extension >> 12) & 7) as usize;
        let raw = if extension & 0x8000 != 0 {
            self.state.a[register]
        } else {
            self.state.d[register]
        };
        let value = if extension & 0x0800 != 0 {
            raw
        } else {
            raw as u16 as i16 as i32 as u32
        };
        value.wrapping_mul(1u32 << ((extension >> 9) & 3))
    }

    fn set_move_flags(&mut self, value: u32) {
        self.state.sr &= !(CCR_N | CCR_Z | CCR_V | CCR_C);
        if value == 0 {
            self.state.sr |= CCR_Z;
        }
        if value & 0x8000_0000 != 0 {
            self.state.sr |= CCR_N;
        }
    }

    fn execute_branch(
        &mut self,
        opcode: u16,
        cursor: &mut Cursor,
    ) -> Result<(String, String), Mc68020Error> {
        let condition = ((opcode >> 8) & 15) as u8;
        let low = opcode as u8;
        let base = cursor.next;
        let displacement = match low {
            0 => self.fetch_word(cursor)? as i16 as i32 as u32,
            0xff => self.fetch_long(cursor)?,
            value => value as i8 as i32 as u32,
        };
        let taken = condition <= 1 || self.condition_true(condition);
        if condition == 1 {
            let return_pc = cursor.next;
            let sp = self.state.a[7].wrapping_sub(4);
            self.write_long(sp, return_pc)?;
            self.set_a(7, sp);
        }
        self.state.pc = if taken {
            base.wrapping_add(displacement)
        } else {
            cursor.next
        };
        let name = [
            "BRA", "BSR", "BHI", "BLS", "BCC", "BCS", "BNE", "BEQ", "BVC", "BVS", "BPL", "BMI",
            "BGE", "BLT", "BGT", "BLE",
        ][condition as usize];
        Ok((
            name.into(),
            format!(
                "{} to {:#010x}",
                if taken { "taken" } else { "not taken" },
                self.state.pc
            ),
        ))
    }

    fn condition_true(&self, condition: u8) -> bool {
        let n = self.state.sr & CCR_N != 0;
        let z = self.state.sr & CCR_Z != 0;
        let v = self.state.sr & CCR_V != 0;
        let c = self.state.sr & CCR_C != 0;
        match condition {
            0 => true,
            1 => false,
            2 => !c && !z,
            3 => c || z,
            4 => !c,
            5 => c,
            6 => !z,
            7 => z,
            8 => !v,
            9 => v,
            10 => !n,
            11 => n,
            12 => n == v,
            13 => n != v,
            14 => !z && n == v,
            15 => z || n != v,
            _ => false,
        }
    }

    fn execute_rtd(&mut self, cursor: &mut Cursor) -> Result<(String, String), Mc68020Error> {
        let displacement = self.fetch_word(cursor)? as i16 as i32 as u32;
        let sp = self.state.a[7];
        let target = self.read_long(sp)?;
        self.set_a(7, sp.wrapping_add(4).wrapping_add(displacement));
        self.state.pc = target;
        Ok(("RTD".into(), format!("return {target:#010x}")))
    }

    fn execute_link_long(
        &mut self,
        opcode: u16,
        cursor: &mut Cursor,
    ) -> Result<(String, String), Mc68020Error> {
        let register = (opcode & 7) as usize;
        let displacement = self.fetch_long(cursor)?;
        let sp = self.state.a[7].wrapping_sub(4);
        self.set_a(7, sp);
        let saved = self.state.a[register];
        self.write_long(sp, saved)?;
        self.set_a(register, sp);
        let adjusted = self.state.a[7].wrapping_add(displacement);
        self.set_a(7, adjusted);
        self.state.pc = cursor.next;
        Ok((
            "LINK.L".into(),
            format!("A{register}, {:#010x}", displacement),
        ))
    }

    fn execute_extb(&mut self, opcode: u16, cursor: &Cursor) -> (String, String) {
        let register = (opcode & 7) as usize;
        let value = self.state.d[register] as u8 as i8 as i32 as u32;
        self.state.d[register] = value;
        self.set_move_flags(value);
        self.state.pc = cursor.next;
        ("EXTB.L".into(), format!("D{register} = {value:#010x}"))
    }

    fn execute_trapcc(
        &mut self,
        opcode: u16,
        cursor: &mut Cursor,
    ) -> Result<(String, String), Mc68020Error> {
        let condition = ((opcode >> 8) & 15) as u8;
        let form = opcode & 7;
        match form {
            2 => {
                self.fetch_word(cursor)?;
            }
            3 => {
                self.fetch_long(cursor)?;
            }
            4 => {}
            _ => unreachable!(),
        }
        if self.condition_true(condition) {
            return Err(Mc68020Error::Trap {
                condition,
                pc: cursor.start,
            });
        }
        self.state.pc = cursor.next;
        Ok(("TRAPCC".into(), "condition false".into()))
    }

    fn is_inherited(opcode: u16) -> bool {
        opcode == 0x4e71
            || opcode & 0xf100 == 0x7000
            || opcode & 0xfff8 == 0x4840
            || opcode & 0xfff8 == 0x4880
            || opcode & 0xfff8 == 0x48c0
            || (opcode >> 12 == 5 && (opcode >> 6) & 3 != 3 && (opcode >> 3) & 7 == 0)
    }

    fn is_privileged_or_unmodeled_trap(opcode: u16) -> bool {
        matches!(opcode, 0x4e70 | 0x4e72 | 0x4e73 | 0x4e7a | 0x4e7b)
            || (opcode & 0xfff0 == 0x4e40 && opcode != 0x4e4f)
            || matches!(opcode, 0x007c | 0x027c | 0x0a7c)
    }

    fn execute_inherited(&mut self, pc: u32) -> Result<(String, String), Mc68020Error> {
        use m68k_simulator::{M68kSimulator, M68kState};
        let mut inherited = M68kSimulator::architectural();
        inherited
            .restore(&M68kState {
                d: self.state.d,
                a: self.state.a,
                pc,
                sr: self.state.sr,
                halted: false,
                memory: self.state.memory.clone(),
            })
            .map_err(|error| Mc68020Error::Inherited(error.to_string()))?;
        let trace = inherited
            .step_checked()
            .map_err(|error| Mc68020Error::Inherited(error.to_string()))?;
        let state = inherited.get_state();
        self.state.d = state.d;
        self.state.a = state.a;
        self.state.pc = state.pc;
        self.state.sr = state.sr;
        self.state.halted = state.halted;
        self.state.memory = state.memory;
        self.sync_active_stack();
        Ok((
            trace.mnemonic,
            "checked MC68000 inherited transition".into(),
        ))
    }

    fn sync_active_stack(&mut self) {
        let value = self.state.a[7];
        if self.state.sr & SR_S == 0 {
            self.state.usp = value;
        } else if self.state.sr & SR_M == 0 {
            self.state.isp = value;
        } else {
            self.state.msp = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_condition_selectors_match_the_architectural_truth_table() {
        let mut cpu = Mc68020Simulator::new();
        for sr in 0u16..16 {
            cpu.state.sr = sr;
            let n = sr & CCR_N != 0;
            let z = sr & CCR_Z != 0;
            let v = sr & CCR_V != 0;
            let c = sr & CCR_C != 0;
            let expected = [
                true,
                false,
                !c && !z,
                c || z,
                !c,
                c,
                !z,
                z,
                !v,
                v,
                !n,
                n,
                n == v,
                n != v,
                !z && n == v,
                z || n != v,
            ];
            for (condition, expected) in expected.into_iter().enumerate() {
                assert_eq!(
                    cpu.condition_true(condition as u8),
                    expected,
                    "SR={sr:#x} cc={condition}"
                );
            }
        }
    }
}
