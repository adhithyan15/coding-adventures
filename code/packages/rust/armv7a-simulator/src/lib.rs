//! Complete functional ARMv7-A / Thumb-2 simulator for Spec 07x.
//!
//! The architecture intentionally uses a small, wrapping 64 KiB memory so the
//! instruction semantics remain inspectable. Program installation and fetches
//! are checked, while architectural data accesses wrap modulo 64 KiB.

pub mod encoding;

/// Architectural memory size.
pub const MEMORY_SIZE: usize = 65_536;
/// Stack pointer register.
pub const SP: usize = 13;
/// Link register.
pub const LR: usize = 14;
/// Program counter register.
pub const PC: usize = 15;

const CPSR_N: u32 = 1 << 31;
const CPSR_Z: u32 = 1 << 30;
const CPSR_C: u32 = 1 << 29;
const CPSR_V: u32 = 1 << 28;
const CPSR_T: u32 = 1 << 5;

/// Complete restorable architectural and installation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Armv7AState {
    /// R0-R15. R15 is kept equal to [`Self::pc`].
    pub registers: [u32; 16],
    /// Authoritative next fetch address.
    pub pc: u32,
    /// Current Program Status Register.
    pub cpsr: u32,
    /// Exact 64 KiB byte-addressed memory.
    pub memory: Vec<u8>,
    /// Whether the halt sentinel has committed.
    pub halted: bool,
    /// First installed program byte.
    pub loaded_origin: u32,
    /// Number of installed program bytes.
    pub loaded_len: usize,
}

/// Typed lifecycle and execution failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Armv7AError {
    Halted,
    InvalidRegister { index: usize },
    InvalidState(String),
    MisalignedProgram { origin: u32 },
    ProgramOutOfRange { origin: u32, length: usize },
    MisalignedFetch { pc: u32 },
    FetchOutsideProgram { pc: u32, width: usize },
    UnknownInstruction16 { raw: u16, pc: u32 },
    UnknownInstruction32 { first: u16, second: u16, pc: u32 },
    StepLimitExceeded { max_steps: usize },
}

impl std::fmt::Display for Armv7AError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidRegister { index } => write!(f, "register {index} is outside r0-r15"),
            Self::InvalidState(message) => f.write_str(message),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin {origin:#010x} is not halfword-aligned")
            }
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes at {origin:#010x} exceeds 64 KiB memory"
            ),
            Self::MisalignedFetch { pc } => write!(f, "misaligned Thumb fetch at {pc:#010x}"),
            Self::FetchOutsideProgram { pc, width } => write!(
                f,
                "{width}-byte instruction at {pc:#010x} crosses installed program bounds"
            ),
            Self::UnknownInstruction16 { raw, pc } => {
                write!(f, "unknown 16-bit instruction {raw:#06x} at {pc:#010x}")
            }
            Self::UnknownInstruction32 { first, second, pc } => write!(
                f,
                "unknown 32-bit instruction {first:#06x} {second:#06x} at {pc:#010x}"
            ),
            Self::StepLimitExceeded { max_steps } => {
                write!(f, "step limit {max_steps} exceeded")
            }
        }
    }
}

impl std::error::Error for Armv7AError {}

/// One committed architectural transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u32,
    pub pc_after: u32,
    pub raw: u32,
    pub width: usize,
    pub mnemonic: String,
    pub state_before: Armv7AState,
    pub state_after: Armv7AState,
}

/// Successful bounded execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: Armv7AState,
}

/// Exact functional ARMv7-A / Thumb-2 machine.
#[derive(Debug, Clone)]
pub struct Armv7ASimulator {
    state: Armv7AState,
}

impl Default for Armv7ASimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Armv7ASimulator {
    /// Construct the Spec 07x reset state.
    #[must_use]
    pub fn new() -> Self {
        let mut registers = [0; 16];
        registers[SP] = 0xfff8;
        Self {
            state: Armv7AState {
                registers,
                pc: 0,
                cpsr: CPSR_T,
                memory: vec![0; MEMORY_SIZE],
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
    pub fn get_state(&self) -> Armv7AState {
        self.state.clone()
    }

    /// Atomically restore a validated snapshot.
    pub fn restore(&mut self, state: &Armv7AState) -> Result<(), Armv7AError> {
        validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    /// Reset and load a program at address zero.
    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), Armv7AError> {
        self.load_at_checked(program, 0)
    }

    /// Reset and load a program at a checked halfword-aligned origin.
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), Armv7AError> {
        if origin & 1 != 0 {
            return Err(Armv7AError::MisalignedProgram { origin });
        }
        let start = origin as usize;
        let Some(end) = start.checked_add(program.len()) else {
            return Err(Armv7AError::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        };
        if end > MEMORY_SIZE {
            return Err(Armv7AError::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        }
        let mut next = Self::new();
        next.state.memory[start..end].copy_from_slice(program);
        next.state.pc = origin;
        next.state.registers[PC] = origin;
        next.state.loaded_origin = origin;
        next.state.loaded_len = program.len();
        *self = next;
        Ok(())
    }

    /// Read one register.
    pub fn read_register(&self, index: usize) -> Result<u32, Armv7AError> {
        self.state
            .registers
            .get(index)
            .copied()
            .ok_or(Armv7AError::InvalidRegister { index })
    }

    /// Write one register, keeping R15 and PC coherent.
    pub fn write_register(&mut self, index: usize, value: u32) -> Result<(), Armv7AError> {
        if index >= 16 {
            return Err(Armv7AError::InvalidRegister { index });
        }
        if index == PC {
            self.set_pc(value & !1);
        } else {
            self.state.registers[index] = value;
        }
        Ok(())
    }

    /// Read a wrapping byte from architectural memory.
    #[must_use]
    pub fn read_byte(&self, address: u32) -> u8 {
        self.state.memory[address as u16 as usize]
    }

    /// Write a wrapping byte to architectural memory.
    pub fn write_byte(&mut self, address: u32, value: u8) {
        self.state.memory[address as u16 as usize] = value;
    }

    /// Execute one atomic transition.
    pub fn step_checked(&mut self) -> Result<StepTrace, Armv7AError> {
        if self.state.halted {
            return Err(Armv7AError::Halted);
        }
        let before = self.state.clone();
        match self.step_inner() {
            Ok((raw, width, mnemonic)) => Ok(StepTrace {
                pc_before: before.pc,
                pc_after: self.state.pc,
                raw,
                width,
                mnemonic,
                state_before: before,
                state_after: self.state.clone(),
            }),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    /// Execute atomically until halt; roll back the whole run on any failure.
    pub fn run_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, Armv7AError> {
        let before = self.state.clone();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.state.halted {
                return Ok(ExecutionResult {
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
        Err(Armv7AError::StepLimitExceeded { max_steps })
    }

    fn step_inner(&mut self) -> Result<(u32, usize, String), Armv7AError> {
        let pc = self.state.pc;
        if pc & 1 != 0 {
            return Err(Armv7AError::MisalignedFetch { pc });
        }
        self.check_fetch(pc, 2)?;
        let first = self.read16(pc);
        self.set_pc(pc.wrapping_add(2));
        if first == 0 {
            self.state.halted = true;
            return Ok((0, 2, "hlt".into()));
        }
        let top5 = first >> 11;
        if matches!(top5, 0b11101..=0b11111) {
            self.check_fetch(pc, 4)?;
            let second = self.read16(pc.wrapping_add(2));
            self.set_pc(pc.wrapping_add(4));
            let mnemonic = self.execute32(first, second, pc)?;
            Ok((((first as u32) << 16) | second as u32, 4, mnemonic))
        } else {
            let mnemonic = self.execute16(first, pc)?;
            Ok((first as u32, 2, mnemonic))
        }
    }

    fn check_fetch(&self, pc: u32, width: usize) -> Result<(), Armv7AError> {
        let start = self.state.loaded_origin as usize;
        let end = start + self.state.loaded_len;
        let address = pc as usize;
        if address < start || address.checked_add(width).is_none_or(|last| last > end) {
            return Err(Armv7AError::FetchOutsideProgram { pc, width });
        }
        Ok(())
    }

    fn set_pc(&mut self, pc: u32) {
        self.state.pc = pc;
        self.state.registers[PC] = pc;
    }

    fn read_register_operand(&self, index: usize, instruction_pc: u32) -> u32 {
        if index == PC {
            instruction_pc.wrapping_add(4)
        } else {
            self.state.registers[index]
        }
    }

    fn write_register_operand(&mut self, index: usize, value: u32) {
        if index == PC {
            self.set_pc(value & !1);
        } else {
            self.state.registers[index] = value;
        }
    }

    fn read16(&self, address: u32) -> u16 {
        u16::from_le_bytes([
            self.read_byte(address),
            self.read_byte(address.wrapping_add(1)),
        ])
    }

    fn read32(&self, address: u32) -> u32 {
        u32::from_le_bytes([
            self.read_byte(address),
            self.read_byte(address.wrapping_add(1)),
            self.read_byte(address.wrapping_add(2)),
            self.read_byte(address.wrapping_add(3)),
        ])
    }

    fn write16(&mut self, address: u32, value: u16) {
        for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
            self.write_byte(address.wrapping_add(offset as u32), byte);
        }
    }

    fn write32(&mut self, address: u32, value: u32) {
        for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
            self.write_byte(address.wrapping_add(offset as u32), byte);
        }
    }

    fn flag(&self, mask: u32) -> bool {
        self.state.cpsr & mask != 0
    }

    fn set_flag(&mut self, mask: u32, value: bool) {
        if value {
            self.state.cpsr |= mask;
        } else {
            self.state.cpsr &= !mask;
        }
    }

    fn set_nz(&mut self, value: u32) {
        self.set_flag(CPSR_N, value >> 31 != 0);
        self.set_flag(CPSR_Z, value == 0);
    }

    fn add_flags(&mut self, left: u32, right: u32, carry: bool) -> u32 {
        let carry_value = u64::from(carry);
        let full = left as u64 + right as u64 + carry_value;
        let result = full as u32;
        self.set_nz(result);
        self.set_flag(CPSR_C, full > u32::MAX as u64);
        let signed = left as i32 as i64 + right as i32 as i64 + carry_value as i64;
        self.set_flag(CPSR_V, signed < i32::MIN as i64 || signed > i32::MAX as i64);
        result
    }

    fn sub_flags(&mut self, left: u32, right: u32, borrow: bool) -> u32 {
        let subtrahend = right as u64 + u64::from(borrow);
        let result = left.wrapping_sub(right).wrapping_sub(u32::from(borrow));
        self.set_nz(result);
        self.set_flag(CPSR_C, left as u64 >= subtrahend);
        let signed = left as i32 as i64 - right as i32 as i64 - i64::from(borrow);
        self.set_flag(CPSR_V, signed < i32::MIN as i64 || signed > i32::MAX as i64);
        result
    }

    fn condition(&self, condition: u8) -> bool {
        let n = self.flag(CPSR_N);
        let z = self.flag(CPSR_Z);
        let c = self.flag(CPSR_C);
        let v = self.flag(CPSR_V);
        match condition {
            0x0 => z,
            0x1 => !z,
            0x2 => c,
            0x3 => !c,
            0x4 => n,
            0x5 => !n,
            0x6 => v,
            0x7 => !v,
            0x8 => c && !z,
            0x9 => !c || z,
            0xa => n == v,
            0xb => n != v,
            0xc => !z && n == v,
            0xd => z || n != v,
            0xe => true,
            _ => false,
        }
    }

    fn execute16(&mut self, raw: u16, pc: u32) -> Result<String, Armv7AError> {
        match raw >> 13 {
            0b000 => return self.execute16_shift_add_sub(raw),
            0b001 => return Ok(self.execute16_immediate(raw)),
            _ => {}
        }
        if raw >> 10 == 0b010000 {
            return Ok(self.execute16_data(raw));
        }
        if raw >> 10 == 0b010001 {
            return Ok(self.execute16_special(raw, pc));
        }
        if raw >> 12 == 0b0101 {
            return Ok(self.execute16_memory_register(raw, pc));
        }
        if matches!(raw >> 12, 0b0110..=0b1000) {
            return Ok(self.execute16_memory_immediate(raw, pc));
        }
        if raw >> 12 == 0b1001 {
            return Ok(self.execute16_memory_sp(raw));
        }
        if raw >> 12 == 0b1010 {
            return Ok(self.execute16_adr(raw, pc));
        }
        if raw >> 12 == 0b1011 {
            return self.execute16_misc(raw, pc);
        }
        if raw >> 12 == 0b1100 {
            return self.execute16_multiple(raw);
        }
        if raw >> 12 == 0b1101 {
            let condition = ((raw >> 8) & 0xf) as u8;
            if condition >= 0xe {
                return Err(Armv7AError::UnknownInstruction16 { raw, pc });
            }
            if self.condition(condition) {
                let offset = (raw as u8 as i8 as i32 * 2) as u32;
                self.set_pc(self.state.pc.wrapping_add(offset));
            }
            return Ok(format!("b.cond{condition:x}"));
        }
        if raw >> 11 == 0b11100 {
            let offset = sign_extend((raw & 0x7ff) as u32, 11).wrapping_mul(2);
            self.set_pc(self.state.pc.wrapping_add(offset));
            return Ok("b".into());
        }
        Err(Armv7AError::UnknownInstruction16 { raw, pc })
    }

    fn execute16_shift_add_sub(&mut self, raw: u16) -> Result<String, Armv7AError> {
        let operation = (raw >> 11) & 3;
        if operation < 3 {
            let amount = ((raw >> 6) & 0x1f) as u32;
            let rm = ((raw >> 3) & 7) as usize;
            let rd = (raw & 7) as usize;
            let (result, carry) = shift_immediate(
                self.state.registers[rm],
                operation as u8,
                amount,
                self.flag(CPSR_C),
            );
            self.state.registers[rd] = result;
            self.set_nz(result);
            self.set_flag(CPSR_C, carry);
            return Ok(["lsls", "lsrs", "asrs"][operation as usize].into());
        }
        let op = (raw >> 9) & 3;
        let operand = ((raw >> 6) & 7) as usize;
        let rn = ((raw >> 3) & 7) as usize;
        let rd = (raw & 7) as usize;
        let left = self.state.registers[rn];
        let right = if op < 2 {
            self.state.registers[operand]
        } else {
            operand as u32
        };
        let result = if op & 1 == 0 {
            self.add_flags(left, right, false)
        } else {
            self.sub_flags(left, right, false)
        };
        self.state.registers[rd] = result;
        Ok(if op & 1 == 0 { "adds" } else { "subs" }.into())
    }

    fn execute16_immediate(&mut self, raw: u16) -> String {
        let operation = (raw >> 11) & 3;
        let register = ((raw >> 8) & 7) as usize;
        let immediate = (raw & 0xff) as u32;
        match operation {
            0 => {
                self.state.registers[register] = immediate;
                self.set_nz(immediate);
                "movs"
            }
            1 => {
                self.sub_flags(self.state.registers[register], immediate, false);
                "cmp"
            }
            2 => {
                self.state.registers[register] =
                    self.add_flags(self.state.registers[register], immediate, false);
                "adds"
            }
            _ => {
                self.state.registers[register] =
                    self.sub_flags(self.state.registers[register], immediate, false);
                "subs"
            }
        }
        .into()
    }

    fn execute16_data(&mut self, raw: u16) -> String {
        let operation = ((raw >> 6) & 0xf) as u8;
        let rm = ((raw >> 3) & 7) as usize;
        let rdn = (raw & 7) as usize;
        let left = self.state.registers[rdn];
        let right = self.state.registers[rm];
        let mnemonic = match operation {
            0x0 => {
                let result = left & right;
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "ands"
            }
            0x1 => {
                let result = left ^ right;
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "eors"
            }
            0x2..=0x4 | 0x7 => {
                let shift = match operation {
                    2 => 0,
                    3 => 1,
                    4 => 2,
                    _ => 3,
                };
                let (result, carry) = shift_register(left, shift, right & 0xff, self.flag(CPSR_C));
                self.state.registers[rdn] = result;
                self.set_nz(result);
                self.set_flag(CPSR_C, carry);
                ["lsls", "lsrs", "asrs", "rors"][shift as usize]
            }
            0x5 => {
                self.state.registers[rdn] = self.add_flags(left, right, self.flag(CPSR_C));
                "adcs"
            }
            0x6 => {
                self.state.registers[rdn] = self.sub_flags(left, right, !self.flag(CPSR_C));
                "sbcs"
            }
            0x8 => {
                self.set_nz(left & right);
                "tst"
            }
            0x9 => {
                self.state.registers[rdn] = self.sub_flags(0, left, false);
                "rsbs"
            }
            0xa => {
                self.sub_flags(left, right, false);
                "cmp"
            }
            0xb => {
                self.add_flags(left, right, false);
                "cmn"
            }
            0xc => {
                let result = left | right;
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "orrs"
            }
            0xd => {
                let result = left.wrapping_mul(right);
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "muls"
            }
            0xe => {
                let result = left & !right;
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "bics"
            }
            _ => {
                let result = !right;
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "mvns"
            }
        };
        mnemonic.into()
    }

    fn execute16_special(&mut self, raw: u16, pc: u32) -> String {
        let operation = (raw >> 8) & 3;
        let rm = ((raw >> 3) & 0xf) as usize;
        let rd = ((((raw >> 7) & 1) << 3) | (raw & 7)) as usize;
        if operation == 3 {
            let target = self.read_register_operand(rm, pc);
            if raw & (1 << 7) != 0 {
                self.state.registers[LR] = self.state.pc | 1;
            }
            self.set_pc(target & !1);
            return if raw & (1 << 7) != 0 { "blx" } else { "bx" }.into();
        }
        let left = self.read_register_operand(rd, pc);
        let right = self.read_register_operand(rm, pc);
        match operation {
            0 => {
                self.write_register_operand(rd, left.wrapping_add(right));
                "add"
            }
            1 => {
                self.sub_flags(left, right, false);
                "cmp"
            }
            _ => {
                self.write_register_operand(rd, right);
                "mov"
            }
        }
        .into()
    }

    fn execute16_memory_register(&mut self, raw: u16, _pc: u32) -> String {
        let operation = (raw >> 9) & 7;
        let rm = ((raw >> 6) & 7) as usize;
        let rn = ((raw >> 3) & 7) as usize;
        let rt = (raw & 7) as usize;
        let address = self.state.registers[rn].wrapping_add(self.state.registers[rm]);
        match operation {
            0 => self.write32(address, self.state.registers[rt]),
            1 => self.write16(address, self.state.registers[rt] as u16),
            2 => self.write_byte(address, self.state.registers[rt] as u8),
            3 => self.state.registers[rt] = self.read_byte(address) as i8 as i32 as u32,
            4 => self.state.registers[rt] = self.read32(address),
            5 => self.state.registers[rt] = self.read16(address) as u32,
            6 => self.state.registers[rt] = self.read_byte(address) as u32,
            _ => self.state.registers[rt] = self.read16(address) as i16 as i32 as u32,
        }
        [
            "str", "strh", "strb", "ldrsb", "ldr", "ldrh", "ldrb", "ldrsh",
        ][operation as usize]
            .into()
    }

    fn execute16_memory_immediate(&mut self, raw: u16, _pc: u32) -> String {
        let operation = raw >> 11;
        let immediate = ((raw >> 6) & 0x1f) as u32;
        let rn = ((raw >> 3) & 7) as usize;
        let rt = (raw & 7) as usize;
        let (scale, load, width, mnemonic) = match operation {
            0b01100 => (4, false, 4, "str"),
            0b01101 => (4, true, 4, "ldr"),
            0b01110 => (1, false, 1, "strb"),
            0b01111 => (1, true, 1, "ldrb"),
            0b10000 => (2, false, 2, "strh"),
            _ => (2, true, 2, "ldrh"),
        };
        let address = self.state.registers[rn].wrapping_add(immediate * scale);
        if load {
            self.state.registers[rt] = match width {
                1 => self.read_byte(address) as u32,
                2 => self.read16(address) as u32,
                _ => self.read32(address),
            };
        } else {
            match width {
                1 => self.write_byte(address, self.state.registers[rt] as u8),
                2 => self.write16(address, self.state.registers[rt] as u16),
                _ => self.write32(address, self.state.registers[rt]),
            }
        }
        mnemonic.into()
    }

    fn execute16_memory_sp(&mut self, raw: u16) -> String {
        let load = raw & (1 << 11) != 0;
        let rt = ((raw >> 8) & 7) as usize;
        let address = self.state.registers[SP].wrapping_add((raw as u8 as u32) * 4);
        if load {
            self.state.registers[rt] = self.read32(address);
            "ldr"
        } else {
            self.write32(address, self.state.registers[rt]);
            "str"
        }
        .into()
    }

    fn execute16_adr(&mut self, raw: u16, pc: u32) -> String {
        let rd = ((raw >> 8) & 7) as usize;
        let immediate = (raw as u8 as u32) * 4;
        let base = if raw & (1 << 11) == 0 {
            pc.wrapping_add(4) & !3
        } else {
            self.state.registers[SP]
        };
        self.state.registers[rd] = base.wrapping_add(immediate);
        if raw & (1 << 11) == 0 {
            "adr"
        } else {
            "add-sp"
        }
        .into()
    }

    fn execute16_misc(&mut self, raw: u16, pc: u32) -> Result<String, Armv7AError> {
        if raw & 0xff00 == 0xbf00 {
            return Ok("nop".into());
        }
        if raw >> 8 == 0xb0 {
            let offset = ((raw & 0x7f) as u32) * 4;
            if raw & 0x80 != 0 {
                self.state.registers[SP] = self.state.registers[SP].wrapping_sub(offset);
                return Ok("sub-sp".into());
            }
            self.state.registers[SP] = self.state.registers[SP].wrapping_add(offset);
            return Ok("add-sp".into());
        }
        if raw & 0xfe00 == 0xb400 {
            let mut registers: Vec<usize> = (0..8).filter(|bit| raw & (1 << bit) != 0).collect();
            if raw & 0x100 != 0 {
                registers.push(LR);
            }
            for register in registers.into_iter().rev() {
                self.state.registers[SP] = self.state.registers[SP].wrapping_sub(4);
                self.write32(self.state.registers[SP], self.state.registers[register]);
            }
            return Ok("push".into());
        }
        if raw & 0xfe00 == 0xbc00 {
            for register in 0..8 {
                if raw & (1 << register) != 0 {
                    self.state.registers[register] = self.read32(self.state.registers[SP]);
                    self.state.registers[SP] = self.state.registers[SP].wrapping_add(4);
                }
            }
            if raw & 0x100 != 0 {
                let target = self.read32(self.state.registers[SP]);
                self.state.registers[SP] = self.state.registers[SP].wrapping_add(4);
                self.set_pc(target & !1);
            }
            return Ok("pop".into());
        }
        Err(Armv7AError::UnknownInstruction16 { raw, pc })
    }

    fn execute16_multiple(&mut self, raw: u16) -> Result<String, Armv7AError> {
        let load = raw & (1 << 11) != 0;
        let rn = ((raw >> 8) & 7) as usize;
        let list = raw as u8;
        if list == 0 {
            return Err(Armv7AError::UnknownInstruction16 {
                raw,
                pc: self.state.pc.wrapping_sub(2),
            });
        }
        let mut address = self.state.registers[rn];
        for register in 0..8 {
            if list & (1 << register) != 0 {
                if load {
                    self.state.registers[register] = self.read32(address);
                } else {
                    self.write32(address, self.state.registers[register]);
                }
                address = address.wrapping_add(4);
            }
        }
        if !load || list & (1 << rn) == 0 {
            self.state.registers[rn] = address;
        }
        Ok(if load { "ldm" } else { "stm" }.into())
    }

    fn execute32(&mut self, first: u16, second: u16, pc: u32) -> Result<String, Armv7AError> {
        if first >> 11 == 0b11110 && second >> 14 == 0b11 && second & (1 << 12) != 0 {
            let s = ((first >> 10) & 1) as u32;
            let j1 = ((second >> 13) & 1) as u32;
            let j2 = ((second >> 11) & 1) as u32;
            let i1 = (!(j1 ^ s)) & 1;
            let i2 = (!(j2 ^ s)) & 1;
            let encoded = (s << 24)
                | (i1 << 23)
                | (i2 << 22)
                | (((first & 0x03ff) as u32) << 12)
                | (((second & 0x07ff) as u32) << 1);
            self.state.registers[LR] = self.state.pc | 1;
            self.set_pc(self.state.pc.wrapping_add(sign_extend(encoded, 25)));
            return Ok("bl".into());
        }
        if first >> 11 == 0b11110 {
            return self.execute32_immediate(first, second, pc);
        }
        if first >> 11 == 0b11111 {
            return self.execute32_memory(first, second, pc);
        }
        Err(Armv7AError::UnknownInstruction32 { first, second, pc })
    }

    fn execute32_immediate(
        &mut self,
        first: u16,
        second: u16,
        pc: u32,
    ) -> Result<String, Armv7AError> {
        let i = ((first >> 10) & 1) as u32;
        let operation = ((first >> 5) & 0xf) as u8;
        let set_flags = first & (1 << 4) != 0;
        let rn = (first & 0xf) as usize;
        let imm3 = ((second >> 12) & 7) as u32;
        let rd = ((second >> 8) & 0xf) as usize;
        let imm8 = (second & 0xff) as u32;
        let immediate12 = (i << 11) | (imm3 << 8) | imm8;
        let special = (first >> 4) & 0x3f;
        if special == 0b100100 || special == 0b101100 {
            let immediate = (((first & 0xf) as u32) << 12) | immediate12;
            if rd == PC {
                return Err(Armv7AError::UnknownInstruction32 { first, second, pc });
            }
            if special == 0b100100 {
                self.state.registers[rd] = immediate;
                return Ok("movw".into());
            }
            self.state.registers[rd] = (immediate << 16) | (self.state.registers[rd] & 0xffff);
            return Ok("movt".into());
        }
        let immediate = thumb_expand_immediate(immediate12);
        let left = self.read_register_operand(rn, pc);
        let (result, mnemonic) = match operation {
            0x0 => (left & immediate, "and.w"),
            0x2 if rn == PC => (immediate, "mov.w"),
            0x2 => (left | immediate, "orr.w"),
            0x4 => (left ^ immediate, "eor.w"),
            0x8 => (left.wrapping_add(immediate), "add.w"),
            0xa => (
                left.wrapping_add(immediate)
                    .wrapping_add(u32::from(self.flag(CPSR_C))),
                "adc.w",
            ),
            0xd => (left.wrapping_sub(immediate), "sub.w"),
            0xe => (immediate.wrapping_sub(left), "rsb.w"),
            _ => return Err(Armv7AError::UnknownInstruction32 { first, second, pc }),
        };
        if rd == PC {
            return Err(Armv7AError::UnknownInstruction32 { first, second, pc });
        }
        self.state.registers[rd] = result;
        if set_flags {
            match operation {
                0x8 => {
                    self.add_flags(left, immediate, false);
                }
                0xa => {
                    self.add_flags(left, immediate, self.flag(CPSR_C));
                }
                0xd => {
                    self.sub_flags(left, immediate, false);
                }
                0xe => {
                    self.sub_flags(immediate, left, false);
                }
                _ => self.set_nz(result),
            }
        }
        Ok(mnemonic.into())
    }

    fn execute32_memory(
        &mut self,
        first: u16,
        second: u16,
        pc: u32,
    ) -> Result<String, Armv7AError> {
        let size = (first >> 5) & 3;
        let load = first & (1 << 4) != 0;
        let rn = (first & 0xf) as usize;
        let rt = ((second >> 12) & 0xf) as usize;
        if size == 3 || rt == PC {
            return Err(Armv7AError::UnknownInstruction32 { first, second, pc });
        }
        let address = self
            .read_register_operand(rn, pc)
            .wrapping_add((second & 0x0fff) as u32);
        let mnemonic = if load {
            self.state.registers[rt] = match size {
                0 => self.read_byte(address) as u32,
                1 => self.read16(address) as u32,
                _ => self.read32(address),
            };
            ["ldrb.w", "ldrh.w", "ldr.w"][size as usize]
        } else {
            match size {
                0 => self.write_byte(address, self.state.registers[rt] as u8),
                1 => self.write16(address, self.state.registers[rt] as u16),
                _ => self.write32(address, self.state.registers[rt]),
            }
            ["strb.w", "strh.w", "str.w"][size as usize]
        };
        Ok(mnemonic.into())
    }
}

fn validate_state(state: &Armv7AState) -> Result<(), Armv7AError> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(Armv7AError::InvalidState(format!(
            "memory has {} bytes, expected {MEMORY_SIZE}",
            state.memory.len()
        )));
    }
    if state.pc & 1 != 0 {
        return Err(Armv7AError::InvalidState(format!(
            "PC {:#010x} is not halfword-aligned",
            state.pc
        )));
    }
    if state.registers[PC] != state.pc {
        return Err(Armv7AError::InvalidState(
            "R15 must equal the authoritative PC".into(),
        ));
    }
    if state.cpsr & CPSR_T == 0 {
        return Err(Armv7AError::InvalidState(
            "CPSR.T must remain set in the Thumb-only machine".into(),
        ));
    }
    let start = state.loaded_origin as usize;
    if start > MEMORY_SIZE
        || start
            .checked_add(state.loaded_len)
            .is_none_or(|end| end > MEMORY_SIZE)
    {
        return Err(Armv7AError::InvalidState(
            "installed program range exceeds memory".into(),
        ));
    }
    Ok(())
}

fn sign_extend(value: u32, bits: u32) -> u32 {
    let shift = 32 - bits;
    ((value << shift) as i32 >> shift) as u32
}

fn thumb_expand_immediate(immediate12: u32) -> u32 {
    let immediate8 = immediate12 & 0xff;
    if immediate12 >> 10 == 0 {
        match (immediate12 >> 8) & 3 {
            0 => immediate8,
            1 => (immediate8 << 16) | immediate8,
            2 => (immediate8 << 24) | (immediate8 << 8),
            _ => immediate8 * 0x0101_0101,
        }
    } else {
        let unrotated = 0x80 | (immediate12 & 0x7f);
        unrotated.rotate_right((immediate12 >> 7) & 0x1f)
    }
}

fn shift_immediate(value: u32, kind: u8, amount: u32, carry_in: bool) -> (u32, bool) {
    match kind {
        0 if amount == 0 => (value, carry_in),
        0 => (value << amount, value & (1 << (32 - amount)) != 0),
        1 => {
            let amount = if amount == 0 { 32 } else { amount };
            if amount == 32 {
                (0, value >> 31 != 0)
            } else {
                (value >> amount, value & (1 << (amount - 1)) != 0)
            }
        }
        _ => {
            let amount = if amount == 0 { 32 } else { amount };
            if amount == 32 {
                ((value as i32 >> 31) as u32, value >> 31 != 0)
            } else {
                (
                    (value as i32 >> amount) as u32,
                    value & (1 << (amount - 1)) != 0,
                )
            }
        }
    }
}

fn shift_register(value: u32, kind: u8, amount: u32, carry_in: bool) -> (u32, bool) {
    if amount == 0 {
        return (value, carry_in);
    }
    match kind {
        0 if amount < 32 => (value << amount, value & (1 << (32 - amount)) != 0),
        0 if amount == 32 => (0, value & 1 != 0),
        0 => (0, false),
        1 if amount < 32 => (value >> amount, value & (1 << (amount - 1)) != 0),
        1 if amount == 32 => (0, value >> 31 != 0),
        1 => (0, false),
        2 if amount < 32 => (
            (value as i32 >> amount) as u32,
            value & (1 << (amount - 1)) != 0,
        ),
        2 => ((value as i32 >> 31) as u32, value >> 31 != 0),
        _ => {
            let rotation = amount & 31;
            if rotation == 0 {
                (value, value >> 31 != 0)
            } else {
                let result = value.rotate_right(rotation);
                (result, result >> 31 != 0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_and_basic_program() {
        let mut cpu = Armv7ASimulator::new();
        assert_eq!(cpu.read_register(SP), Ok(0xfff8));
        let bytes = encoding::program(&[
            encoding::mov_imm8(0, 40),
            encoding::add_imm8(0, 2),
            encoding::halt(),
        ]);
        cpu.load_checked(&bytes).unwrap();
        let result = cpu.run_checked(4).unwrap();
        assert_eq!(result.steps, 3);
        assert_eq!(result.final_state.registers[0], 42);
        assert!(result.halted);
    }

    #[test]
    fn step_and_run_faults_are_atomic() {
        let mut cpu = Armv7ASimulator::new();
        cpu.load_checked(&0xffff_u16.to_le_bytes()).unwrap();
        let before = cpu.get_state();
        assert!(matches!(
            cpu.step_checked(),
            Err(Armv7AError::FetchOutsideProgram { width: 4, .. })
        ));
        assert_eq!(cpu.get_state(), before);

        let bytes = encoding::program(&[encoding::mov_imm8(0, 7), encoding::branch(-1)]);
        cpu.load_checked(&bytes).unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.run_checked(3),
            Err(Armv7AError::StepLimitExceeded { max_steps: 3 })
        );
        assert_eq!(cpu.get_state(), before);
    }

    #[test]
    fn wide_move_and_branch_link() {
        let mut cpu = Armv7ASimulator::new();
        let bytes = encoding::program(&[
            encoding::movw(0, 0x1234),
            encoding::movt(0, 0x5678),
            encoding::branch_link(2),
            encoding::halt(),
            encoding::mov_imm8(1, 42),
            encoding::data_register(0, 0, 0),
        ]);
        cpu.load_checked(&bytes).unwrap();
        cpu.step_checked().unwrap();
        cpu.step_checked().unwrap();
        assert_eq!(cpu.read_register(0), Ok(0x5678_1234));
        cpu.step_checked().unwrap();
        assert_eq!(cpu.read_register(LR), Ok(13));
        assert_eq!(cpu.get_state().pc, 14);
        cpu.step_checked().unwrap();
        assert_eq!(cpu.read_register(1), Ok(42));
    }
}
