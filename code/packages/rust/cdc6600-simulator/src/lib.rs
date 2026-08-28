//! Functional CDC 6600 simulator for the repository's specified behavioral subset.
//!
//! The model uses 60-bit words, 18-bit address/index registers, four 15-bit
//! parcels per word, and checked 15/30-bit instruction execution.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MASK_60: u64 = (1_u64 << 60) - 1;
pub const MASK_18: u32 = (1_u32 << 18) - 1;
pub const SIGN_60: u64 = 1_u64 << 59;
pub const MEMORY_WORDS: usize = 4_096;
pub const MEMORY_PARCELS: u32 = (MEMORY_WORDS as u32) * 4;
pub const PARCEL_MASK: u16 = 0x7fff;
pub const HALT: [u8; 2] = [0, 0];

pub const F_TXB: u8 = 1;
pub const F_TBX: u8 = 2;
pub const F_TAX: u8 = 3;
pub const F_TXA: u8 = 4;
pub const F_IXPB: u8 = 5;
pub const F_IXMB: u8 = 6;
pub const F_IXXP: u8 = 7;
pub const F_IXXM: u8 = 8;
pub const F_BXND: u8 = 9;
pub const F_BXOR: u8 = 10;
pub const F_BXXR: u8 = 11;
pub const F_BXMR: u8 = 12;
pub const F_LSHL: u8 = 13;
pub const F_LSHR: u8 = 14;
pub const F_IBBP: u8 = 15;
pub const F_IBBM: u8 = 16;
pub const F_IAAP: u8 = 17;
pub const F_IAAM: u8 = 18;
pub const F_CMPEQ: u8 = 19;
pub const F_CMPLT: u8 = 20;
pub const F_CMPGT: u8 = 21;
pub const F_IXMUL: u8 = 22;

pub const F_LDXI: u8 = 32;
pub const F_LDBI: u8 = 33;
pub const F_LDAI: u8 = 34;
pub const F_LDX: u8 = 35;
pub const F_STX: u8 = 36;
pub const F_LDB: u8 = 37;
pub const F_STB: u8 = 38;
pub const F_JEQ: u8 = 40;
pub const F_JNE: u8 = 41;
pub const F_JXZ: u8 = 42;
pub const F_JXN: u8 = 43;
pub const F_JMP: u8 = 44;
pub const F_JSR: u8 = 45;
pub const F_RET: u8 = 46;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cdc6600State {
    pub p: u32,
    pub x: [u64; 8],
    pub a: [u32; 8],
    pub b: [u32; 8],
    pub memory: Vec<u64>,
    pub halted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub instruction: u32,
    pub parcels: u8,
    pub mnemonic: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: Cdc6600State,
    pub traces: Vec<StepTrace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Cdc6600Error {
    InvalidProgramLength { bytes: usize },
    NonCanonicalParcel { index: usize, value: u16 },
    ProgramTooLarge { parcels: usize, capacity: usize },
    ProgramCounterOutOfRange { target: u32 },
    MissingLongParcel { pc: u32 },
    MemoryAddressOutOfRange { address: u32 },
    InvalidRegister { bank: char, index: usize },
    UnknownShortOpcode { opcode: u8, pc: u32 },
    UnknownLongOpcode { opcode: u8, pc: u32 },
    MaxStepsExceeded { max_steps: usize },
}

impl Display for Cdc6600Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProgramLength { bytes } => {
                write!(
                    f,
                    "CDC 6600 program length {bytes} is not an even parcel transport"
                )
            }
            Self::NonCanonicalParcel { index, value } => {
                write!(
                    f,
                    "parcel {index} has non-canonical 16-bit value {value:#06x}"
                )
            }
            Self::ProgramTooLarge { parcels, capacity } => {
                write!(
                    f,
                    "program has {parcels} parcels but memory holds {capacity}"
                )
            }
            Self::ProgramCounterOutOfRange { target } => {
                write!(f, "parcel address {target} is outside 0..{MEMORY_PARCELS}")
            }
            Self::MissingLongParcel { pc } => {
                write!(f, "long instruction at parcel {pc} has no second parcel")
            }
            Self::MemoryAddressOutOfRange { address } => {
                write!(f, "word address {address} is outside 0..{MEMORY_WORDS}")
            }
            Self::InvalidRegister { bank, index } => {
                write!(f, "register {bank}{index} is outside {bank}0..{bank}7")
            }
            Self::UnknownShortOpcode { opcode, pc } => {
                write!(f, "unknown short opcode {opcode:#04o} at parcel {pc}")
            }
            Self::UnknownLongOpcode { opcode, pc } => {
                write!(f, "unknown long opcode {opcode:#04o} at parcel {pc}")
            }
            Self::MaxStepsExceeded { max_steps } => {
                write!(f, "maximum execution step count {max_steps} exceeded")
            }
        }
    }
}

impl Error for Cdc6600Error {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cdc6600Simulator {
    p: u32,
    x: [u64; 8],
    a: [u32; 8],
    b: [u32; 8],
    memory: Vec<u64>,
    halted: bool,
}

impl Default for Cdc6600Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Cdc6600Simulator {
    pub fn new() -> Self {
        Self {
            p: 0,
            x: [0; 8],
            a: [0; 8],
            b: [0; 8],
            memory: vec![0; MEMORY_WORDS],
            halted: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load(&mut self, program: &[u8]) -> Result<usize, Cdc6600Error> {
        if !program.len().is_multiple_of(2) {
            return Err(Cdc6600Error::InvalidProgramLength {
                bytes: program.len(),
            });
        }
        let parcel_count = program.len() / 2;
        if parcel_count > MEMORY_PARCELS as usize {
            return Err(Cdc6600Error::ProgramTooLarge {
                parcels: parcel_count,
                capacity: MEMORY_PARCELS as usize,
            });
        }
        let parcels = program
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
            .map(|(index, bytes)| {
                let value = u16::from_be_bytes([bytes[0], bytes[1]]);
                if value > PARCEL_MASK {
                    Err(Cdc6600Error::NonCanonicalParcel { index, value })
                } else {
                    Ok(value)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.load_parcels(&parcels)?;
        Ok(parcels.len())
    }

    pub fn load_parcels(&mut self, parcels: &[u16]) -> Result<(), Cdc6600Error> {
        if parcels.len() > MEMORY_PARCELS as usize {
            return Err(Cdc6600Error::ProgramTooLarge {
                parcels: parcels.len(),
                capacity: MEMORY_PARCELS as usize,
            });
        }
        for (index, value) in parcels.iter().copied().enumerate() {
            if value > PARCEL_MASK {
                return Err(Cdc6600Error::NonCanonicalParcel { index, value });
            }
        }

        let mut memory = vec![0_u64; MEMORY_WORDS];
        for (index, value) in parcels.iter().copied().enumerate() {
            let word = index / 4;
            let shift = (3 - index % 4) * 15;
            memory[word] |= u64::from(value) << shift;
        }
        *self = Self {
            memory,
            ..Self::new()
        };
        Ok(())
    }

    pub fn state(&self) -> Cdc6600State {
        Cdc6600State {
            p: self.p,
            x: self.x,
            a: self.a,
            b: self.b,
            memory: self.memory.clone(),
            halted: self.halted,
        }
    }

    /// Return an immutable snapshot of the complete architectural state.
    pub fn get_state(&self) -> Cdc6600State {
        self.state()
    }

    pub fn read_word(&self, address: usize) -> Result<u64, Cdc6600Error> {
        self.memory
            .get(address)
            .copied()
            .ok_or(Cdc6600Error::MemoryAddressOutOfRange {
                address: address as u32,
            })
    }

    pub fn write_word(&mut self, address: usize, value: u64) -> Result<(), Cdc6600Error> {
        let destination =
            self.memory
                .get_mut(address)
                .ok_or(Cdc6600Error::MemoryAddressOutOfRange {
                    address: address as u32,
                })?;
        *destination = value & MASK_60;
        Ok(())
    }

    pub fn set_program_counter(&mut self, p: u32) -> Result<(), Cdc6600Error> {
        Self::validate_parcel_address(p)?;
        self.p = p;
        Ok(())
    }

    pub fn write_x(&mut self, index: usize, value: u64) -> Result<(), Cdc6600Error> {
        let register = self
            .x
            .get_mut(index)
            .ok_or(Cdc6600Error::InvalidRegister { bank: 'X', index })?;
        *register = value & MASK_60;
        Ok(())
    }

    pub fn write_a(&mut self, index: usize, value: u32) -> Result<(), Cdc6600Error> {
        let register = self
            .a
            .get_mut(index)
            .ok_or(Cdc6600Error::InvalidRegister { bank: 'A', index })?;
        *register = value & MASK_18;
        Ok(())
    }

    pub fn write_b(&mut self, index: usize, value: u32) -> Result<(), Cdc6600Error> {
        let register = self
            .b
            .get_mut(index)
            .ok_or(Cdc6600Error::InvalidRegister { bank: 'B', index })?;
        if index != 0 {
            *register = value & MASK_18;
        }
        self.b[0] = 0;
        Ok(())
    }

    pub fn step(&mut self) -> Result<StepTrace, Cdc6600Error> {
        let pc_before = self.p;
        if self.halted {
            return Ok(StepTrace {
                pc_before,
                pc_after: pc_before,
                instruction: 0,
                parcels: 1,
                mnemonic: "HALT".to_string(),
                description: "HALT (already halted)".to_string(),
            });
        }

        let first = self.read_parcel(pc_before)?;
        if first == 0 {
            self.halted = true;
            return Ok(StepTrace {
                pc_before,
                pc_after: pc_before,
                instruction: 0,
                parcels: 1,
                mnemonic: "HALT".to_string(),
                description: format!("HALT @ parcel {pc_before:#06x}"),
            });
        }

        let opcode = ((first >> 9) & 0x3f) as u8;
        let i = usize::from((first >> 6) & 7);
        let j = usize::from((first >> 3) & 7);
        let k = usize::from(first & 7);

        let (instruction, parcels, mnemonic, pc_after) = if opcode >= 32 {
            let second_address = pc_before
                .checked_add(1)
                .filter(|address| *address < MEMORY_PARCELS)
                .ok_or(Cdc6600Error::MissingLongParcel { pc: pc_before })?;
            let second = self.read_parcel(second_address)?;
            let constant = (u32::from(first & 7) << 15) | u32::from(second);
            let next = pc_before + 2;
            let (mnemonic, pc_after) = self.exec_long(opcode, i, j, constant, next)?;
            (
                (u32::from(first) << 15) | u32::from(second),
                2,
                mnemonic,
                pc_after,
            )
        } else {
            let next = pc_before + 1;
            Self::validate_parcel_address(next)?;
            let mnemonic = self.exec_short(opcode, i, j, k, next)?;
            (u32::from(first), 1, mnemonic, next)
        };

        Ok(StepTrace {
            pc_before,
            pc_after,
            instruction,
            parcels,
            description: format!("{mnemonic} @ parcel {pc_before:#06x}"),
            mnemonic,
        })
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Cdc6600Error> {
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted {
                break;
            }
            traces.push(self.step()?);
        }
        if !self.halted {
            return Err(Cdc6600Error::MaxStepsExceeded { max_steps });
        }
        Ok(ExecutionResult {
            halted: true,
            steps: traces.len(),
            final_state: self.state(),
            traces,
        })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, Cdc6600Error> {
        self.load(program)?;
        self.run(max_steps)
    }

    fn read_parcel(&self, p: u32) -> Result<u16, Cdc6600Error> {
        Self::validate_parcel_address(p)?;
        let word = self.memory[(p / 4) as usize];
        let shift = (3 - p % 4) * 15;
        Ok(((word >> shift) & u64::from(PARCEL_MASK)) as u16)
    }

    fn validate_parcel_address(target: u32) -> Result<(), Cdc6600Error> {
        if target >= MEMORY_PARCELS {
            Err(Cdc6600Error::ProgramCounterOutOfRange { target })
        } else {
            Ok(())
        }
    }

    fn memory_address(base: u32, constant: u32) -> Result<usize, Cdc6600Error> {
        let address = base.wrapping_add(constant) & MASK_18;
        if address >= MEMORY_WORDS as u32 {
            Err(Cdc6600Error::MemoryAddressOutOfRange { address })
        } else {
            Ok(address as usize)
        }
    }

    fn exec_short(
        &mut self,
        opcode: u8,
        i: usize,
        j: usize,
        k: usize,
        next: u32,
    ) -> Result<String, Cdc6600Error> {
        let mut x = self.x;
        let mut a = self.a;
        let mut b = self.b;
        let xj = self.x[j];
        let xk = self.x[k];
        let aj = self.a[j];
        let bj = self.b[j];
        let bk = self.b[k];

        let mnemonic = match opcode {
            F_TXB => {
                x[i] = u64::from(bj);
                format!("TXB X{i},B{j}")
            }
            F_TBX => {
                if i != 0 {
                    b[i] = (xj as u32) & MASK_18;
                }
                format!("TBX B{i},X{j}")
            }
            F_TAX => {
                x[i] = u64::from(aj);
                format!("TAX X{i},A{j}")
            }
            F_TXA => {
                a[i] = (xj as u32) & MASK_18;
                format!("TXA A{i},X{j}")
            }
            F_IXPB => {
                x[i] = xj.wrapping_add(u64::from(bk)) & MASK_60;
                format!("IXPB X{i},X{j},B{k}")
            }
            F_IXMB => {
                x[i] = xj.wrapping_sub(u64::from(bk)) & MASK_60;
                format!("IXMB X{i},X{j},B{k}")
            }
            F_IXXP => {
                x[i] = xj.wrapping_add(xk) & MASK_60;
                format!("IXXP X{i},X{j},X{k}")
            }
            F_IXXM => {
                x[i] = xj.wrapping_sub(xk) & MASK_60;
                format!("IXXM X{i},X{j},X{k}")
            }
            F_BXND => {
                x[i] = xj & xk;
                format!("BXND X{i},X{j},X{k}")
            }
            F_BXOR => {
                x[i] = xj | xk;
                format!("BXOR X{i},X{j},X{k}")
            }
            F_BXXR => {
                x[i] = xj ^ xk;
                format!("BXXR X{i},X{j},X{k}")
            }
            F_BXMR => {
                x[i] = !xj & MASK_60;
                format!("BXMR X{i},X{j}")
            }
            F_LSHL => {
                x[i] = xj.wrapping_shl(bk & 63) & MASK_60;
                format!("LSHL X{i},X{j},B{k}")
            }
            F_LSHR => {
                x[i] = xj >> (bk & 63);
                format!("LSHR X{i},X{j},B{k}")
            }
            F_IBBP => {
                if i != 0 {
                    b[i] = bj.wrapping_add(bk) & MASK_18;
                }
                format!("IBBP B{i},B{j},B{k}")
            }
            F_IBBM => {
                if i != 0 {
                    b[i] = bj.wrapping_sub(bk) & MASK_18;
                }
                format!("IBBM B{i},B{j},B{k}")
            }
            F_IAAP => {
                a[i] = aj.wrapping_add(bk) & MASK_18;
                format!("IAAP A{i},A{j},B{k}")
            }
            F_IAAM => {
                a[i] = aj.wrapping_sub(bk) & MASK_18;
                format!("IAAM A{i},A{j},B{k}")
            }
            F_CMPEQ => {
                if i != 0 {
                    b[i] = u32::from(xj == xk);
                }
                format!("CMPEQ B{i},X{j},X{k}")
            }
            F_CMPLT => {
                if i != 0 {
                    b[i] = u32::from(signed_60(xj) < signed_60(xk));
                }
                format!("CMPLT B{i},X{j},X{k}")
            }
            F_CMPGT => {
                if i != 0 {
                    b[i] = u32::from(signed_60(xj) > signed_60(xk));
                }
                format!("CMPGT B{i},X{j},X{k}")
            }
            F_IXMUL => {
                x[i] = ((u128::from(xj) * u128::from(xk)) & u128::from(MASK_60)) as u64;
                format!("IXMUL X{i},X{j},X{k}")
            }
            _ => {
                return Err(Cdc6600Error::UnknownShortOpcode { opcode, pc: self.p });
            }
        };

        b[0] = 0;
        self.x = x;
        self.a = a;
        self.b = b;
        self.p = next;
        Ok(mnemonic)
    }

    fn exec_long(
        &mut self,
        opcode: u8,
        i: usize,
        j: usize,
        constant: u32,
        next: u32,
    ) -> Result<(String, u32), Cdc6600Error> {
        let mut x = self.x;
        let mut a = self.a;
        let mut b = self.b;
        let mut memory_update = None;
        let mut final_p = next;

        let mnemonic = match opcode {
            F_LDXI => {
                Self::validate_parcel_address(next)?;
                x[i] = u64::from(constant & MASK_18);
                format!("LDXI X{i},{constant}")
            }
            F_LDBI => {
                Self::validate_parcel_address(next)?;
                if i != 0 {
                    b[i] = constant & MASK_18;
                }
                format!("LDBI B{i},{constant}")
            }
            F_LDAI => {
                Self::validate_parcel_address(next)?;
                a[i] = constant & MASK_18;
                format!("LDAI A{i},{constant}")
            }
            F_LDX => {
                Self::validate_parcel_address(next)?;
                let address = Self::memory_address(self.a[j], constant)?;
                x[i] = self.memory[address];
                format!("LDX X{i},A{j}+{constant}")
            }
            F_STX => {
                Self::validate_parcel_address(next)?;
                let address = Self::memory_address(self.a[i], constant)?;
                memory_update = Some((address, self.x[j]));
                format!("STX A{i}+{constant},X{j}")
            }
            F_LDB => {
                Self::validate_parcel_address(next)?;
                let address = Self::memory_address(self.a[j], constant)?;
                if i != 0 {
                    b[i] = (self.memory[address] as u32) & MASK_18;
                }
                format!("LDB B{i},A{j}+{constant}")
            }
            F_STB => {
                Self::validate_parcel_address(next)?;
                let address = Self::memory_address(self.a[i], constant)?;
                memory_update = Some((address, u64::from(self.b[j])));
                format!("STB A{i}+{constant},B{j}")
            }
            F_JEQ => {
                if self.b[j] == 0 {
                    Self::validate_parcel_address(constant)?;
                    final_p = constant;
                } else {
                    Self::validate_parcel_address(next)?;
                }
                format!("JEQ B{j}==0,{constant}")
            }
            F_JNE => {
                if self.b[j] != 0 {
                    Self::validate_parcel_address(constant)?;
                    final_p = constant;
                } else {
                    Self::validate_parcel_address(next)?;
                }
                format!("JNE B{j}!=0,{constant}")
            }
            F_JXZ => {
                if self.x[j] == 0 {
                    Self::validate_parcel_address(constant)?;
                    final_p = constant;
                } else {
                    Self::validate_parcel_address(next)?;
                }
                format!("JXZ X{j}==0,{constant}")
            }
            F_JXN => {
                if self.x[j] != 0 {
                    Self::validate_parcel_address(constant)?;
                    final_p = constant;
                } else {
                    Self::validate_parcel_address(next)?;
                }
                format!("JXN X{j}!=0,{constant}")
            }
            F_JMP => {
                Self::validate_parcel_address(constant)?;
                final_p = constant;
                format!("JMP {constant}")
            }
            F_JSR => {
                Self::validate_parcel_address(next)?;
                Self::validate_parcel_address(constant)?;
                b[7] = next & MASK_18;
                final_p = constant;
                format!("JSR B7={next},P={constant}")
            }
            F_RET => {
                let target = self.b[j] & MASK_18;
                Self::validate_parcel_address(target)?;
                final_p = target;
                format!("RET P=B{j}")
            }
            _ => {
                return Err(Cdc6600Error::UnknownLongOpcode { opcode, pc: self.p });
            }
        };

        b[0] = 0;
        self.x = x;
        self.a = a;
        self.b = b;
        if let Some((address, value)) = memory_update {
            self.memory[address] = value & MASK_60;
        }
        self.p = final_p;
        Ok((mnemonic, final_p))
    }
}

pub fn signed_60(value: u64) -> i64 {
    let value = value & MASK_60;
    if value & SIGN_60 == 0 {
        value as i64
    } else {
        (value as i64) - (1_i64 << 60)
    }
}

pub fn short_instr(opcode: u8, i: u8, j: u8, k: u8) -> [u8; 2] {
    let value = (u16::from(opcode & 0x3f) << 9)
        | (u16::from(i & 7) << 6)
        | (u16::from(j & 7) << 3)
        | u16::from(k & 7);
    value.to_be_bytes()
}

pub fn long_instr(opcode: u8, i: u8, j: u8, constant: u32) -> [u8; 4] {
    let constant = constant & MASK_18;
    let first = (u16::from(opcode & 0x3f) << 9)
        | (u16::from(i & 7) << 6)
        | (u16::from(j & 7) << 3)
        | ((constant >> 15) as u16 & 7);
    let second = constant as u16 & PARCEL_MASK;
    let first = first.to_be_bytes();
    let second = second.to_be_bytes();
    [first[0], first[1], second[0], second[1]]
}
