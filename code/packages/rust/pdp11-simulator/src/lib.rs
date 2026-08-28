//! Functional DEC PDP-11 simulator for the complete Spec 07o subset.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MEMORY_BYTES: usize = 65_536;
pub const LOAD_ADDRESS: u16 = 0x1000;
pub const INITIAL_SP: u16 = 0xf000;
pub const SP: usize = 6;
pub const PC: usize = 7;
pub const HALT: [u8; 2] = [0, 0];

pub const PSW_N: u16 = 0b1000;
pub const PSW_Z: u16 = 0b0100;
pub const PSW_V: u16 = 0b0010;
pub const PSW_C: u16 = 0b0001;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pdp11State {
    pub r: [u16; 8],
    pub psw: u16,
    pub halted: bool,
    pub memory: Vec<u8>,
}

impl Pdp11State {
    pub const fn n(&self) -> bool {
        self.psw & PSW_N != 0
    }

    pub const fn z(&self) -> bool {
        self.psw & PSW_Z != 0
    }

    pub const fn v(&self) -> bool {
        self.psw & PSW_V != 0
    }

    pub const fn c(&self) -> bool {
        self.psw & PSW_C != 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub mnemonic: String,
    pub description: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub final_state: Pdp11State,
    pub traces: Vec<StepTrace>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pdp11Error {
    ProgramTooLarge { bytes: usize, capacity: usize },
    InvalidRegister { index: usize },
    OddWordAddress { address: u16, write: bool },
    IllegalAddressingMode { instruction: &'static str, mode: u8 },
    UnknownOpcode { opcode: u16, pc: u16 },
    MaxStepsExceeded { max_steps: usize },
}

impl Display for Pdp11Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProgramTooLarge { bytes, capacity } => {
                write!(
                    f,
                    "program has {bytes} bytes but load region holds {capacity}"
                )
            }
            Self::InvalidRegister { index } => write!(f, "register R{index} is outside R0..R7"),
            Self::OddWordAddress { address, write } => write!(
                f,
                "odd address word {} at {address:#06x}",
                if *write { "write" } else { "read" }
            ),
            Self::IllegalAddressingMode { instruction, mode } => {
                write!(f, "{instruction} does not permit addressing mode {mode}")
            }
            Self::UnknownOpcode { opcode, pc } => {
                write!(f, "unknown PDP-11 opcode {opcode:#06x} at PC={pc:#06x}")
            }
            Self::MaxStepsExceeded { max_steps } => {
                write!(f, "maximum execution step count {max_steps} exceeded")
            }
        }
    }
}

impl Error for Pdp11Error {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EffectiveAddress {
    value: u16,
    register: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SingleOperation {
    Swab,
    Clr(bool),
    Com(bool),
    Inc(bool),
    Dec(bool),
    Neg(bool),
    Adc(bool),
    Sbc(bool),
    Tst(bool),
    Ror(bool),
    Rol(bool),
    Asr(bool),
    Asl(bool),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DoubleOperation {
    Mov(bool),
    Cmp(bool),
    Bit(bool),
    Bic(bool),
    Bis(bool),
    Add,
    Sub,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pdp11Simulator {
    r: [u16; 8],
    psw: u16,
    memory: Vec<u8>,
    halted: bool,
}

impl Default for Pdp11Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdp11Simulator {
    pub fn new() -> Self {
        let mut r = [0; 8];
        r[SP] = INITIAL_SP;
        r[PC] = LOAD_ADDRESS;
        Self {
            r,
            psw: 0,
            memory: vec![0; MEMORY_BYTES],
            halted: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load(&mut self, program: &[u8]) -> Result<usize, Pdp11Error> {
        let capacity = MEMORY_BYTES - usize::from(LOAD_ADDRESS);
        if program.len() > capacity {
            return Err(Pdp11Error::ProgramTooLarge {
                bytes: program.len(),
                capacity,
            });
        }
        let mut replacement = Self::new();
        let start = usize::from(LOAD_ADDRESS);
        replacement.memory[start..start + program.len()].copy_from_slice(program);
        *self = replacement;
        Ok(program.len())
    }

    pub fn state(&self) -> Pdp11State {
        Pdp11State {
            r: self.r,
            psw: self.psw,
            halted: self.halted,
            memory: self.memory.clone(),
        }
    }

    pub fn get_state(&self) -> Pdp11State {
        self.state()
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        self.memory[usize::from(address)] = value;
    }

    pub fn read_word(&self, address: u16) -> Result<u16, Pdp11Error> {
        if address & 1 != 0 {
            return Err(Pdp11Error::OddWordAddress {
                address,
                write: false,
            });
        }
        let index = usize::from(address);
        Ok(u16::from_le_bytes([
            self.memory[index],
            self.memory[index + 1],
        ]))
    }

    pub fn write_word(&mut self, address: u16, value: u16) -> Result<(), Pdp11Error> {
        if address & 1 != 0 {
            return Err(Pdp11Error::OddWordAddress {
                address,
                write: true,
            });
        }
        let index = usize::from(address);
        self.memory[index..index + 2].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn write_register(&mut self, index: usize, value: u16) -> Result<(), Pdp11Error> {
        let register = self
            .r
            .get_mut(index)
            .ok_or(Pdp11Error::InvalidRegister { index })?;
        *register = value;
        Ok(())
    }

    pub fn write_psw(&mut self, value: u16) {
        self.psw = value;
    }

    pub fn step(&mut self) -> Result<StepTrace, Pdp11Error> {
        let pc_before = self.r[PC];
        if self.halted {
            return Ok(StepTrace {
                pc_before,
                pc_after: pc_before,
                mnemonic: "HALT".to_owned(),
                description: "HALT (already halted)".to_owned(),
            });
        }

        let checkpoint = self.clone();
        match self.execute_one(pc_before) {
            Ok(mnemonic) => Ok(StepTrace {
                pc_before,
                pc_after: self.r[PC],
                description: format!("{mnemonic} @ 0x{pc_before:04X}"),
                mnemonic,
            }),
            Err(error) => {
                *self = checkpoint;
                Err(error)
            }
        }
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Pdp11Error> {
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted {
                break;
            }
            traces.push(self.step()?);
        }
        if !self.halted {
            return Err(Pdp11Error::MaxStepsExceeded { max_steps });
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
    ) -> Result<ExecutionResult, Pdp11Error> {
        self.load(program)?;
        self.run(max_steps)
    }

    fn fetch_word(&mut self) -> Result<u16, Pdp11Error> {
        let word = self.read_word(self.r[PC])?;
        self.r[PC] = self.r[PC].wrapping_add(2);
        Ok(word)
    }

    fn effective_address(
        &mut self,
        mode: u8,
        register: usize,
        word: bool,
    ) -> Result<EffectiveAddress, Pdp11Error> {
        if mode == 0 {
            return Ok(EffectiveAddress {
                value: register as u16,
                register: true,
            });
        }
        let step = if word || register >= SP { 2 } else { 1 };
        let address = match mode {
            1 => self.r[register],
            2 => {
                let address = self.r[register];
                self.r[register] = self.r[register].wrapping_add(step);
                address
            }
            3 => {
                let pointer = self.r[register];
                self.r[register] = self.r[register].wrapping_add(2);
                self.read_word(pointer)?
            }
            4 => {
                self.r[register] = self.r[register].wrapping_sub(step);
                self.r[register]
            }
            5 => {
                self.r[register] = self.r[register].wrapping_sub(2);
                self.read_word(self.r[register])?
            }
            6 => {
                let displacement = self.fetch_word()?;
                self.r[register].wrapping_add(displacement)
            }
            7 => {
                let displacement = self.fetch_word()?;
                self.read_word(self.r[register].wrapping_add(displacement))?
            }
            _ => unreachable!("three mode bits are always in 0..=7"),
        };
        Ok(EffectiveAddress {
            value: address,
            register: false,
        })
    }

    fn read_operand(&self, address: EffectiveAddress, word: bool) -> Result<u16, Pdp11Error> {
        if address.register {
            Ok(if word {
                self.r[usize::from(address.value)]
            } else {
                self.r[usize::from(address.value)] & 0xff
            })
        } else if word {
            self.read_word(address.value)
        } else {
            Ok(u16::from(self.read_byte(address.value)))
        }
    }

    fn write_operand(
        &mut self,
        address: EffectiveAddress,
        value: u16,
        word: bool,
    ) -> Result<(), Pdp11Error> {
        if address.register {
            let register = &mut self.r[usize::from(address.value)];
            if word {
                *register = value;
            } else {
                *register = (*register & 0xff00) | (value & 0xff);
            }
            Ok(())
        } else if word {
            self.write_word(address.value, value)
        } else {
            self.write_byte(address.value, value as u8);
            Ok(())
        }
    }

    fn set_nzvc(&mut self, n: bool, z: bool, v: bool, c: bool) {
        self.psw = (self.psw & 0xfff0) | pack_psw(n, z, v, c);
    }

    fn flag(&self, mask: u16) -> bool {
        self.psw & mask != 0
    }

    fn execute_one(&mut self, pc_before: u16) -> Result<String, Pdp11Error> {
        let instruction = self.fetch_word()?;
        if instruction == 0 {
            self.halted = true;
            return Ok("HALT".to_owned());
        }
        if instruction == 0x00a0 {
            return Ok("NOP".to_owned());
        }
        if instruction == 0x0002 {
            let new_pc = self.read_word(self.r[SP])?;
            self.r[SP] = self.r[SP].wrapping_add(2);
            let new_psw = self.read_word(self.r[SP])?;
            self.r[SP] = self.r[SP].wrapping_add(2);
            self.r[PC] = new_pc;
            self.psw = new_psw;
            return Ok("RTI".to_owned());
        }
        if instruction & 0xfff8 == 0x0080 {
            let register = usize::from(instruction & 7);
            let new_pc = self.r[register];
            let restored = self.read_word(self.r[SP])?;
            self.r[PC] = new_pc;
            self.r[register] = restored;
            self.r[SP] = self.r[SP].wrapping_add(2);
            return Ok("RTS".to_owned());
        }
        if instruction & 0xfe00 == 0x7e00 {
            let register = usize::from((instruction >> 6) & 7);
            let offset = instruction & 0x3f;
            self.r[register] = self.r[register].wrapping_sub(1);
            if self.r[register] != 0 {
                self.r[PC] = self.r[PC].wrapping_sub(offset.wrapping_mul(2));
            }
            return Ok("SOB".to_owned());
        }

        let branch_opcode = (instruction >> 8) as u8;
        if let Some((mnemonic, taken)) = self.branch(branch_opcode) {
            if taken {
                let offset = i16::from(instruction as u8 as i8);
                self.r[PC] = self.r[PC].wrapping_add_signed(offset.wrapping_mul(2));
            }
            return Ok(mnemonic.to_owned());
        }

        if instruction & 0xffc0 == 0x0040 {
            let mode = ((instruction >> 3) & 7) as u8;
            if mode == 0 {
                return Err(Pdp11Error::IllegalAddressingMode {
                    instruction: "JMP",
                    mode,
                });
            }
            let register = usize::from(instruction & 7);
            let address = self.effective_address(mode, register, true)?;
            self.r[PC] = address.value;
            return Ok("JMP".to_owned());
        }

        if instruction & 0xfe00 == 0x0800 {
            let link = usize::from((instruction >> 6) & 7);
            let mode = ((instruction >> 3) & 7) as u8;
            if mode == 0 {
                return Err(Pdp11Error::IllegalAddressingMode {
                    instruction: "JSR",
                    mode,
                });
            }
            let register = usize::from(instruction & 7);
            let destination = self.effective_address(mode, register, true)?;
            let old_link = self.r[link];
            let return_address = self.r[PC];
            self.r[SP] = self.r[SP].wrapping_sub(2);
            self.write_word(self.r[SP], old_link)?;
            self.r[link] = return_address;
            self.r[PC] = destination.value;
            return Ok("JSR".to_owned());
        }

        let mode = ((instruction >> 3) & 7) as u8;
        let register = usize::from(instruction & 7);
        if let Some(operation) = decode_single((instruction >> 6) & 0x03ff) {
            return self.execute_single(operation, mode, register);
        }

        if let Some(operation) = decode_double(instruction >> 12) {
            let source = (instruction >> 6) & 0x3f;
            let source_mode = ((source >> 3) & 7) as u8;
            let source_register = usize::from(source & 7);
            return self.execute_double(operation, source_mode, source_register, mode, register);
        }

        Err(Pdp11Error::UnknownOpcode {
            opcode: instruction,
            pc: pc_before,
        })
    }

    fn branch(&self, opcode: u8) -> Option<(&'static str, bool)> {
        let n = self.flag(PSW_N);
        let z = self.flag(PSW_Z);
        let v = self.flag(PSW_V);
        let c = self.flag(PSW_C);
        Some(match opcode {
            0x01 => ("BR", true),
            0x02 => ("BNE", !z),
            0x03 => ("BEQ", z),
            0x04 => ("BGE", !(n ^ v)),
            0x05 => ("BLT", n ^ v),
            0x06 => ("BGT", !z && !(n ^ v)),
            0x07 => ("BLE", z || (n ^ v)),
            0x80 => ("BPL", !n),
            0x81 => ("BMI", n),
            0x82 => ("BHI", !c && !z),
            0x83 => ("BLOS", c || z),
            0x84 => ("BVC", !v),
            0x85 => ("BVS", v),
            0x86 => ("BCC", !c),
            0x87 => ("BCS", c),
            _ => return None,
        })
    }

    fn execute_single(
        &mut self,
        operation: SingleOperation,
        mode: u8,
        register: usize,
    ) -> Result<String, Pdp11Error> {
        if operation == SingleOperation::Swab {
            let address = self.effective_address(mode, register, true)?;
            let source = self.read_operand(address, true)?;
            let result = source.rotate_left(8);
            self.write_operand(address, result, true)?;
            let low = result as u8;
            self.set_nzvc(low & 0x80 != 0, low == 0, false, false);
            return Ok("SWAB".to_owned());
        }

        let word = match operation {
            SingleOperation::Clr(word)
            | SingleOperation::Com(word)
            | SingleOperation::Inc(word)
            | SingleOperation::Dec(word)
            | SingleOperation::Neg(word)
            | SingleOperation::Adc(word)
            | SingleOperation::Sbc(word)
            | SingleOperation::Tst(word)
            | SingleOperation::Ror(word)
            | SingleOperation::Rol(word)
            | SingleOperation::Asr(word)
            | SingleOperation::Asl(word) => word,
            SingleOperation::Swab => unreachable!(),
        };
        let address = self.effective_address(mode, register, word)?;
        let source = self.read_operand(address, word)?;
        let mask = size_mask(word);
        let msb = size_msb(word);

        let (mnemonic, result, write, flags) = match operation {
            SingleOperation::Clr(_) => (
                sized_name("CLR", word),
                0,
                true,
                (false, true, false, false),
            ),
            SingleOperation::Com(_) => {
                let result = !source & mask;
                (
                    sized_name("COM", word),
                    result,
                    true,
                    logic_flags(result, word, true),
                )
            }
            SingleOperation::Inc(_) => {
                let result = source.wrapping_add(1) & mask;
                let (n, z, v, _) = add_flags(source, 1, word);
                (
                    sized_name("INC", word),
                    result,
                    true,
                    (n, z, v, self.flag(PSW_C)),
                )
            }
            SingleOperation::Dec(_) => {
                let result = source.wrapping_sub(1) & mask;
                let (n, z, v, _) = sub_flags(source, 1, word);
                (
                    sized_name("DEC", word),
                    result,
                    true,
                    (n, z, v, self.flag(PSW_C)),
                )
            }
            SingleOperation::Neg(_) => {
                let result = source.wrapping_neg() & mask;
                (
                    sized_name("NEG", word),
                    result,
                    true,
                    (result & msb != 0, result == 0, source == msb, result != 0),
                )
            }
            SingleOperation::Adc(_) => {
                let carry = u16::from(self.flag(PSW_C));
                let result = source.wrapping_add(carry) & mask;
                (
                    sized_name("ADC", word),
                    result,
                    true,
                    add_flags(source, carry, word),
                )
            }
            SingleOperation::Sbc(_) => {
                let carry = u16::from(self.flag(PSW_C));
                let result = source.wrapping_sub(carry) & mask;
                (
                    sized_name("SBC", word),
                    result,
                    true,
                    sub_flags(source, carry, word),
                )
            }
            SingleOperation::Tst(_) => (
                sized_name("TST", word),
                source,
                false,
                logic_flags(source, word, false),
            ),
            SingleOperation::Asr(_) => {
                let carry = source & 1 != 0;
                let result = ((source >> 1) | (source & msb)) & mask;
                let n = result & msb != 0;
                (
                    sized_name("ASR", word),
                    result,
                    true,
                    (n, result == 0, n ^ carry, carry),
                )
            }
            SingleOperation::Asl(_) => {
                let carry = source & msb != 0;
                let result = source.wrapping_shl(1) & mask;
                let n = result & msb != 0;
                (
                    sized_name("ASL", word),
                    result,
                    true,
                    (n, result == 0, n ^ carry, carry),
                )
            }
            SingleOperation::Ror(_) => {
                let carry = source & 1 != 0;
                let result = ((source >> 1)
                    | (u16::from(self.flag(PSW_C)) << if word { 15 } else { 7 }))
                    & mask;
                let n = result & msb != 0;
                (
                    sized_name("ROR", word),
                    result,
                    true,
                    (n, result == 0, n ^ carry, carry),
                )
            }
            SingleOperation::Rol(_) => {
                let carry = source & msb != 0;
                let result = (source.wrapping_shl(1) | u16::from(self.flag(PSW_C))) & mask;
                let n = result & msb != 0;
                (
                    sized_name("ROL", word),
                    result,
                    true,
                    (n, result == 0, n ^ carry, carry),
                )
            }
            SingleOperation::Swab => unreachable!(),
        };
        if write {
            self.write_operand(address, result, word)?;
        }
        self.set_nzvc(flags.0, flags.1, flags.2, flags.3);
        Ok(mnemonic)
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_double(
        &mut self,
        operation: DoubleOperation,
        source_mode: u8,
        source_register: usize,
        destination_mode: u8,
        destination_register: usize,
    ) -> Result<String, Pdp11Error> {
        let word = match operation {
            DoubleOperation::Mov(word)
            | DoubleOperation::Cmp(word)
            | DoubleOperation::Bit(word)
            | DoubleOperation::Bic(word)
            | DoubleOperation::Bis(word) => word,
            DoubleOperation::Add | DoubleOperation::Sub => true,
        };
        let source_address = self.effective_address(source_mode, source_register, word)?;
        let source = self.read_operand(source_address, word)?;
        let destination_address =
            self.effective_address(destination_mode, destination_register, word)?;
        let destination = self.read_operand(destination_address, word)?;
        let mask = size_mask(word);

        let mnemonic = match operation {
            DoubleOperation::Mov(_) => {
                if !word && destination_address.register {
                    let signed = i16::from(source as u8 as i8) as u16;
                    self.r[usize::from(destination_address.value)] = signed;
                } else {
                    self.write_operand(destination_address, source, word)?;
                }
                let (n, z, _, _) = logic_flags(source, word, false);
                self.set_nzvc(n, z, false, self.flag(PSW_C));
                sized_name("MOV", word)
            }
            DoubleOperation::Cmp(_) => {
                let flags = sub_flags(source, destination, word);
                self.set_nzvc(flags.0, flags.1, flags.2, flags.3);
                sized_name("CMP", word)
            }
            DoubleOperation::Bit(_) => {
                let result = source & destination;
                let (n, z, _, _) = logic_flags(result, word, false);
                self.set_nzvc(n, z, false, self.flag(PSW_C));
                sized_name("BIT", word)
            }
            DoubleOperation::Bic(_) => {
                let result = destination & (!source & mask);
                self.write_operand(destination_address, result, word)?;
                let (n, z, _, _) = logic_flags(result, word, false);
                self.set_nzvc(n, z, false, self.flag(PSW_C));
                sized_name("BIC", word)
            }
            DoubleOperation::Bis(_) => {
                let result = (destination | source) & mask;
                self.write_operand(destination_address, result, word)?;
                let (n, z, _, _) = logic_flags(result, word, false);
                self.set_nzvc(n, z, false, self.flag(PSW_C));
                sized_name("BIS", word)
            }
            DoubleOperation::Add => {
                let result = destination.wrapping_add(source);
                self.write_operand(destination_address, result, true)?;
                let flags = add_flags(destination, source, true);
                self.set_nzvc(flags.0, flags.1, flags.2, flags.3);
                "ADD".to_owned()
            }
            DoubleOperation::Sub => {
                let result = destination.wrapping_sub(source);
                self.write_operand(destination_address, result, true)?;
                let flags = sub_flags(destination, source, true);
                self.set_nzvc(flags.0, flags.1, flags.2, flags.3);
                "SUB".to_owned()
            }
        };
        Ok(mnemonic)
    }
}

fn decode_single(opcode: u16) -> Option<SingleOperation> {
    Some(match opcode {
        0x003 => SingleOperation::Swab,
        0x028 => SingleOperation::Clr(true),
        0x228 => SingleOperation::Clr(false),
        0x029 => SingleOperation::Com(true),
        0x229 => SingleOperation::Com(false),
        0x02a => SingleOperation::Inc(true),
        0x22a => SingleOperation::Inc(false),
        0x02b => SingleOperation::Dec(true),
        0x22b => SingleOperation::Dec(false),
        0x02c => SingleOperation::Neg(true),
        0x22c => SingleOperation::Neg(false),
        0x02d => SingleOperation::Adc(true),
        0x22d => SingleOperation::Adc(false),
        0x02e => SingleOperation::Sbc(true),
        0x22e => SingleOperation::Sbc(false),
        0x02f => SingleOperation::Tst(true),
        0x22f => SingleOperation::Tst(false),
        0x030 => SingleOperation::Ror(true),
        0x230 => SingleOperation::Ror(false),
        0x031 => SingleOperation::Rol(true),
        0x231 => SingleOperation::Rol(false),
        0x032 => SingleOperation::Asr(true),
        0x232 => SingleOperation::Asr(false),
        0x033 => SingleOperation::Asl(true),
        0x233 => SingleOperation::Asl(false),
        _ => return None,
    })
}

fn decode_double(opcode: u16) -> Option<DoubleOperation> {
    Some(match opcode {
        0x1 => DoubleOperation::Mov(true),
        0x9 => DoubleOperation::Mov(false),
        0x2 => DoubleOperation::Cmp(true),
        0xa => DoubleOperation::Cmp(false),
        0x3 => DoubleOperation::Bit(true),
        0xb => DoubleOperation::Bit(false),
        0x4 => DoubleOperation::Bic(true),
        0xc => DoubleOperation::Bic(false),
        0x5 => DoubleOperation::Bis(true),
        0xd => DoubleOperation::Bis(false),
        0x6 => DoubleOperation::Add,
        0xe => DoubleOperation::Sub,
        _ => return None,
    })
}

fn size_mask(word: bool) -> u16 {
    if word {
        0xffff
    } else {
        0x00ff
    }
}

fn size_msb(word: bool) -> u16 {
    if word {
        0x8000
    } else {
        0x0080
    }
}

fn sized_name(name: &str, word: bool) -> String {
    if word {
        name.to_owned()
    } else {
        format!("{name}B")
    }
}

fn pack_psw(n: bool, z: bool, v: bool, c: bool) -> u16 {
    (u16::from(n) << 3) | (u16::from(z) << 2) | (u16::from(v) << 1) | u16::from(c)
}

fn logic_flags(result: u16, word: bool, carry: bool) -> (bool, bool, bool, bool) {
    let result = result & size_mask(word);
    (result & size_msb(word) != 0, result == 0, false, carry)
}

fn add_flags(a: u16, b: u16, word: bool) -> (bool, bool, bool, bool) {
    let mask = u32::from(size_mask(word));
    let msb = size_msb(word);
    let raw = u32::from(a & size_mask(word)) + u32::from(b & size_mask(word));
    let result = raw as u16 & size_mask(word);
    (
        result & msb != 0,
        result == 0,
        (!(a ^ b) & (a ^ result) & msb) != 0,
        raw > mask,
    )
}

fn sub_flags(a: u16, b: u16, word: bool) -> (bool, bool, bool, bool) {
    let mask = size_mask(word);
    let msb = size_msb(word);
    let a = a & mask;
    let b = b & mask;
    let result = a.wrapping_sub(b) & mask;
    (
        result & msb != 0,
        result == 0,
        ((a ^ b) & (a ^ result) & msb) != 0,
        a < b,
    )
}

pub const fn operand(mode: u8, register: u8) -> u16 {
    (((mode & 7) as u16) << 3) | (register & 7) as u16
}

pub const fn word(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

pub const fn double_instruction(
    base: u16,
    source_mode: u8,
    source_register: u8,
    destination_mode: u8,
    destination_register: u8,
) -> [u8; 2] {
    word(
        base | (operand(source_mode, source_register) << 6)
            | operand(destination_mode, destination_register),
    )
}

pub const fn single_instruction(base: u16, mode: u8, register: u8) -> [u8; 2] {
    word(base | operand(mode, register))
}

pub const fn branch_instruction(opcode: u8, offset: i8) -> [u8; 2] {
    word((opcode as u16) << 8 | offset as u8 as u16)
}

pub const fn jsr_instruction(link: u8, mode: u8, register: u8) -> [u8; 2] {
    word(0x0800 | (((link & 7) as u16) << 6) | operand(mode, register))
}

pub const fn rts_instruction(register: u8) -> [u8; 2] {
    word(0x0080 | (register & 7) as u16)
}

pub const fn sob_instruction(register: u8, offset: u8) -> [u8; 2] {
    word(0x7e00 | (((register & 7) as u16) << 6) | (offset & 0x3f) as u16)
}
