//! Checked functional AArch64 simulator for Spec 07v.

pub mod encoding;

use std::fmt;

/// Exact architectural memory size.
pub const MEMORY_SIZE: usize = 65_536;
/// Physical GPR slots, X0-X30 and the hardwired XZR slot.
pub const REGISTER_COUNT: usize = 32;
/// Architectural register encoding used for XZR/WZR.
pub const XZR: usize = 31;

/// Complete architectural and installation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AArch64State {
    pub registers: [u64; REGISTER_COUNT],
    pub sp: u64,
    pub pc: u64,
    pub nzcv: u8,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u64,
    pub loaded_len: usize,
}

/// Typed lifecycle, fetch, decode, and data failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AArch64Error {
    InvalidState(String),
    InvalidRegister { index: usize },
    MisalignedProgram { origin: u64 },
    ProgramOutOfRange { origin: u64, length: usize },
    MisalignedFetch { pc: u64 },
    FetchOutsideProgram { pc: u64 },
    MisalignedData { address: u64, width: usize },
    DataOutOfRange { address: u64, width: usize },
    UnknownInstruction { raw: u32, pc: u64 },
    Halted,
    StepLimitExceeded { max_steps: usize },
}

impl fmt::Display for AArch64Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AArch64Error {}

fn bits(raw: u32, hi: u32, lo: u32) -> u32 {
    (raw >> lo) & (u32::MAX >> (31 - (hi - lo)))
}

fn width_mask(sf: u32) -> u64 {
    if sf == 0 {
        u32::MAX.into()
    } else {
        u64::MAX
    }
}

fn read_gpr(state: &AArch64State, index: usize, sf: u32) -> u64 {
    if index == XZR {
        0
    } else {
        state.registers[index] & width_mask(sf)
    }
}

fn write_gpr(state: &mut AArch64State, index: usize, value: u64, sf: u32) {
    if index != XZR {
        state.registers[index] = value & width_mask(sf);
    }
}

fn sign_extend(value: u64, width: u32) -> i64 {
    ((value << (64 - width)) as i64) >> (64 - width)
}

fn condition_holds(condition: u32, nzcv: u8) -> bool {
    let n = nzcv & 8 != 0;
    let z = nzcv & 4 != 0;
    let c = nzcv & 2 != 0;
    let v = nzcv & 1 != 0;
    let mut result = match condition >> 1 {
        0 => z,
        1 => c,
        2 => n,
        3 => v,
        4 => c && !z,
        5 => n == v,
        6 => n == v && !z,
        _ => true,
    };
    if condition & 1 != 0 && condition != 0x0f {
        result = !result;
    }
    result
}

fn arithmetic_flags(a: u64, b: u64, sf: u32, subtract: bool) -> (u64, u8) {
    let width = if sf == 0 { 32 } else { 64 };
    let mask = width_mask(sf);
    let a = a & mask;
    let b = b & mask;
    let result = if subtract {
        a.wrapping_sub(b)
    } else {
        a.wrapping_add(b)
    } & mask;
    let n = u8::from(result >> (width - 1) != 0);
    let z = u8::from(result == 0);
    let c = if subtract {
        u8::from(a >= b)
    } else if width == 64 {
        u8::from((a as u128 + b as u128) >> 64 != 0)
    } else {
        u8::from(a + b > u64::from(u32::MAX))
    };
    let sign = 1_u64 << (width - 1);
    let v = if subtract {
        u8::from(((a ^ b) & (a ^ result) & sign) != 0)
    } else {
        u8::from(((!(a ^ b)) & (a ^ result) & sign) != 0)
    };
    (result, (n << 3) | (z << 2) | (c << 1) | v)
}

fn logical_flags(value: u64, sf: u32) -> u8 {
    let width = if sf == 0 { 32 } else { 64 };
    (u8::from(value >> (width - 1) != 0) << 3) | (u8::from(value == 0) << 2)
}

fn apply_shift(value: u64, kind: u32, amount: u32, sf: u32) -> u64 {
    let width = if sf == 0 { 32 } else { 64 };
    let mask = width_mask(sf);
    let value = value & mask;
    let amount = amount % width;
    if amount == 0 {
        return value;
    }
    match kind {
        0 => value.wrapping_shl(amount) & mask,
        1 => value >> amount,
        2 => ((sign_extend(value, width) >> amount) as u64) & mask,
        _ if sf == 0 => u64::from((value as u32).rotate_right(amount)),
        _ => value.rotate_right(amount),
    }
}

fn decode_bitmask(n: u32, immr: u32, imms: u32) -> Option<u64> {
    let len = if n == 1 {
        6
    } else {
        let combined = (!imms) & 0x3f;
        31_u32.checked_sub(combined.leading_zeros())?
    };
    if len == 0 {
        return None;
    }
    let element_width = 1_u32 << len;
    let s = imms & (element_width - 1);
    if s == element_width - 1 {
        return None;
    }
    let rotation = immr & (element_width - 1);
    let element_mask = if element_width == 64 {
        u64::MAX
    } else {
        (1_u64 << element_width) - 1
    };
    let ones = if s == 63 {
        u64::MAX
    } else {
        (1_u64 << (s + 1)) - 1
    };
    let element = ((ones >> rotation) | (ones << ((element_width - rotation) % element_width)))
        & element_mask;
    let mut result = 0;
    let mut position = 0;
    while position < 64 {
        result |= element << position;
        position += element_width;
    }
    Some(result)
}

/// One complete instruction-boundary transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AArch64StepTrace {
    pub pc_before: u64,
    pub pc_after: u64,
    pub raw: u32,
    pub mnemonic: &'static str,
    pub state_before: AArch64State,
    pub state_after: AArch64State,
}

/// Successful bounded execution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AArch64ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<AArch64StepTrace>,
    pub final_state: AArch64State,
}

/// Exact checked AArch64 functional machine.
#[derive(Debug, Clone)]
pub struct AArch64Simulator {
    state: AArch64State,
}

impl Default for AArch64Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl AArch64Simulator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: AArch64State {
                registers: [0; REGISTER_COUNT],
                sp: 0,
                pc: 0,
                nzcv: 0,
                memory: vec![0; MEMORY_SIZE],
                halted: false,
                loaded_origin: 0,
                loaded_len: 0,
            },
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    #[must_use]
    pub fn get_state(&self) -> AArch64State {
        self.state.clone()
    }

    pub fn restore(&mut self, snapshot: &AArch64State) -> Result<(), AArch64Error> {
        Self::validate_state(snapshot)?;
        self.state = snapshot.clone();
        Ok(())
    }

    fn validate_state(state: &AArch64State) -> Result<(), AArch64Error> {
        if state.memory.len() != MEMORY_SIZE {
            return Err(AArch64Error::InvalidState(
                "memory must contain exactly 65,536 bytes".into(),
            ));
        }
        if state.registers[XZR] != 0 {
            return Err(AArch64Error::InvalidState("XZR must be zero".into()));
        }
        if state.nzcv > 0x0f {
            return Err(AArch64Error::InvalidState("NZCV must fit four bits".into()));
        }
        if state.pc & 3 != 0 {
            return Err(AArch64Error::InvalidState("PC must be word aligned".into()));
        }
        let end = state
            .loaded_origin
            .checked_add(state.loaded_len as u64)
            .ok_or_else(|| AArch64Error::InvalidState("installed range overflow".into()))?;
        if state.loaded_origin & 3 != 0 || end > MEMORY_SIZE as u64 {
            return Err(AArch64Error::InvalidState(
                "installed range is invalid".into(),
            ));
        }
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), AArch64Error> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u64) -> Result<(), AArch64Error> {
        if origin & 3 != 0 {
            return Err(AArch64Error::MisalignedProgram { origin });
        }
        let end =
            origin
                .checked_add(program.len() as u64)
                .ok_or(AArch64Error::ProgramOutOfRange {
                    origin,
                    length: program.len(),
                })?;
        if end > MEMORY_SIZE as u64 {
            return Err(AArch64Error::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        }
        self.reset();
        self.state.memory[origin as usize..end as usize].copy_from_slice(program);
        self.state.pc = origin;
        self.state.loaded_origin = origin;
        self.state.loaded_len = program.len();
        Ok(())
    }

    pub fn read_register(&self, index: usize) -> Result<u64, AArch64Error> {
        self.state
            .registers
            .get(index)
            .copied()
            .ok_or(AArch64Error::InvalidRegister { index })
    }

    pub fn write_register(&mut self, index: usize, value: u64) -> Result<(), AArch64Error> {
        let register = self
            .state
            .registers
            .get_mut(index)
            .ok_or(AArch64Error::InvalidRegister { index })?;
        if index != XZR {
            *register = value;
        }
        Ok(())
    }

    #[must_use]
    pub fn stack_pointer(&self) -> u64 {
        self.state.sp
    }

    pub fn set_stack_pointer(&mut self, value: u64) {
        self.state.sp = value;
    }

    #[must_use]
    pub fn nzcv(&self) -> u8 {
        self.state.nzcv
    }

    pub fn set_nzcv(&mut self, value: u8) -> Result<(), AArch64Error> {
        if value > 0x0f {
            return Err(AArch64Error::InvalidState("NZCV must fit four bits".into()));
        }
        self.state.nzcv = value;
        Ok(())
    }

    pub fn read_byte(&self, address: u64) -> Result<u8, AArch64Error> {
        self.state
            .memory
            .get(address as usize)
            .copied()
            .ok_or(AArch64Error::DataOutOfRange { address, width: 1 })
    }

    pub fn write_byte(&mut self, address: u64, value: u8) -> Result<(), AArch64Error> {
        let byte = self
            .state
            .memory
            .get_mut(address as usize)
            .ok_or(AArch64Error::DataOutOfRange { address, width: 1 })?;
        *byte = value;
        Ok(())
    }

    fn fetch(&self) -> Result<u32, AArch64Error> {
        let pc = self.state.pc;
        if pc & 3 != 0 {
            return Err(AArch64Error::MisalignedFetch { pc });
        }
        let installed_end = self.state.loaded_origin + self.state.loaded_len as u64;
        if pc < self.state.loaded_origin || pc.checked_add(4).is_none_or(|end| end > installed_end)
        {
            return Err(AArch64Error::FetchOutsideProgram { pc });
        }
        let start = pc as usize;
        Ok(u32::from_be_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("checked fetch"),
        ))
    }

    fn data_read(state: &AArch64State, address: u64, width: usize) -> Result<u64, AArch64Error> {
        if !address.is_multiple_of(width as u64) {
            return Err(AArch64Error::MisalignedData { address, width });
        }
        let end = address
            .checked_add(width as u64)
            .ok_or(AArch64Error::DataOutOfRange { address, width })?;
        if end > MEMORY_SIZE as u64 {
            return Err(AArch64Error::DataOutOfRange { address, width });
        }
        let mut value = 0;
        for byte in &state.memory[address as usize..end as usize] {
            value = (value << 8) | u64::from(*byte);
        }
        Ok(value)
    }

    fn data_write(
        state: &mut AArch64State,
        address: u64,
        width: usize,
        mut value: u64,
    ) -> Result<(), AArch64Error> {
        if !address.is_multiple_of(width as u64) {
            return Err(AArch64Error::MisalignedData { address, width });
        }
        let end = address
            .checked_add(width as u64)
            .ok_or(AArch64Error::DataOutOfRange { address, width })?;
        if end > MEMORY_SIZE as u64 {
            return Err(AArch64Error::DataOutOfRange { address, width });
        }
        for byte in state.memory[address as usize..end as usize]
            .iter_mut()
            .rev()
        {
            *byte = value as u8;
            value >>= 8;
        }
        Ok(())
    }

    /// Fetch, strictly decode, and atomically execute one instruction.
    pub fn step_checked(&mut self) -> Result<AArch64StepTrace, AArch64Error> {
        if self.state.halted {
            return Err(AArch64Error::Halted);
        }
        let before = self.get_state();
        let raw = self.fetch()?;
        let pc = before.pc;
        let mut next = before.clone();
        next.pc = pc.wrapping_add(4);
        let sf = bits(raw, 31, 31);
        let mnemonic;

        if raw == 0 {
            next.pc = pc;
            next.halted = true;
            mnemonic = "halt";
        } else if raw == 0xd503_201f {
            mnemonic = "nop";
        } else if bits(raw, 30, 26) == 0b00101 {
            let offset = sign_extend(u64::from(bits(raw, 25, 0)), 26).wrapping_mul(4);
            if bits(raw, 31, 31) != 0 {
                next.registers[30] = pc.wrapping_add(4);
                mnemonic = "bl";
            } else {
                mnemonic = "b";
            }
            next.pc = pc.wrapping_add_signed(offset);
        } else if bits(raw, 31, 24) == 0b0101_0100 && bits(raw, 4, 4) == 0 {
            if bits(raw, 3, 0) == 0x0f {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            let offset = sign_extend(u64::from(bits(raw, 23, 5)), 19).wrapping_mul(4);
            if condition_holds(bits(raw, 3, 0), next.nzcv) {
                next.pc = pc.wrapping_add_signed(offset);
            }
            mnemonic = "b.cond";
        } else if bits(raw, 30, 25) == 0b011010 {
            let value = read_gpr(&before, bits(raw, 4, 0) as usize, sf);
            let nonzero = bits(raw, 24, 24) != 0;
            if (value != 0) == nonzero {
                let offset = sign_extend(u64::from(bits(raw, 23, 5)), 19).wrapping_mul(4);
                next.pc = pc.wrapping_add_signed(offset);
            }
            mnemonic = if nonzero { "cbnz" } else { "cbz" };
        } else if bits(raw, 30, 25) == 0b011011 {
            let value = read_gpr(&before, bits(raw, 4, 0) as usize, 1);
            let bit_number = (bits(raw, 31, 31) << 5) | bits(raw, 23, 19);
            let nonzero = bits(raw, 24, 24) != 0;
            if (((value >> bit_number) & 1) != 0) == nonzero {
                let offset = sign_extend(u64::from(bits(raw, 18, 5)), 14).wrapping_mul(4);
                next.pc = pc.wrapping_add_signed(offset);
            }
            mnemonic = if nonzero { "tbnz" } else { "tbz" };
        } else if bits(raw, 31, 24) == 0xd6
            && bits(raw, 20, 16) == 0x1f
            && bits(raw, 15, 10) == 0
            && bits(raw, 4, 0) == 0
        {
            let target = read_gpr(&before, bits(raw, 9, 5) as usize, 1);
            if target & 3 != 0 {
                return Err(AArch64Error::MisalignedFetch { pc: target });
            }
            match bits(raw, 23, 21) {
                0 => {
                    next.pc = target;
                    mnemonic = "br";
                }
                1 => {
                    next.registers[30] = pc.wrapping_add(4);
                    next.pc = target;
                    mnemonic = "blr";
                }
                2 => {
                    next.pc = target;
                    mnemonic = "ret";
                }
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            }
        } else if bits(raw, 28, 23) == 0b100000 {
            let subtract = bits(raw, 30, 30) != 0;
            let set_flags = bits(raw, 29, 29) != 0;
            let immediate = u64::from(bits(raw, 21, 10)) << (12 * bits(raw, 22, 22));
            let rn = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let result = if set_flags {
                let (result, flags) = arithmetic_flags(rn, immediate, sf, subtract);
                next.nzcv = flags;
                result
            } else if subtract {
                rn.wrapping_sub(immediate) & width_mask(sf)
            } else {
                rn.wrapping_add(immediate) & width_mask(sf)
            };
            write_gpr(&mut next, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match (subtract, set_flags) {
                (false, false) => "add",
                (false, true) => "adds",
                (true, false) => "sub",
                (true, true) => "subs",
            };
        } else if bits(raw, 28, 23) == 0b100101 {
            let opcode = bits(raw, 30, 29);
            let halfword = bits(raw, 22, 21);
            if sf == 0 && halfword > 1 {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            let shift = halfword * 16;
            let immediate = u64::from(bits(raw, 20, 5)) << shift;
            let rd = bits(raw, 4, 0) as usize;
            let result = match opcode {
                0 => !immediate,
                2 => immediate,
                3 => {
                    let mask = 0xffff_u64 << shift;
                    (read_gpr(&before, rd, sf) & !mask) | immediate
                }
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            };
            write_gpr(&mut next, rd, result, sf);
            mnemonic = match opcode {
                0 => "movn",
                2 => "movz",
                _ => "movk",
            };
        } else if bits(raw, 28, 23) == 0b010010 {
            let n = bits(raw, 22, 22);
            if sf == 0 && n != 0 {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            let immediate = decode_bitmask(n, bits(raw, 21, 16), bits(raw, 15, 10))
                .ok_or(AArch64Error::UnknownInstruction { raw, pc })?
                & width_mask(sf);
            let rn = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let opcode = bits(raw, 30, 29);
            let result = match opcode {
                0 | 3 => rn & immediate,
                1 => rn | immediate,
                _ => rn ^ immediate,
            };
            if opcode == 3 {
                next.nzcv = logical_flags(result, sf);
            }
            write_gpr(&mut next, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = ["and", "orr", "eor", "ands"][opcode as usize];
        } else if bits(raw, 29, 27) == 0b111 && bits(raw, 26, 26) == 0 && bits(raw, 25, 24) == 0b01
        {
            let size = bits(raw, 31, 30);
            let opcode = bits(raw, 23, 22);
            let width = 1_usize << size;
            let rn = bits(raw, 9, 5) as usize;
            let rt = bits(raw, 4, 0) as usize;
            let base = if rn == XZR {
                before.sp
            } else {
                read_gpr(&before, rn, 1)
            };
            let address = base.wrapping_add(u64::from(bits(raw, 21, 10)) * width as u64);
            match (size, opcode) {
                (_, 0) => {
                    let value = read_gpr(&before, rt, u32::from(size == 3));
                    Self::data_write(&mut next, address, width, value)?;
                    mnemonic = ["strb", "strh", "str32", "str"][size as usize];
                }
                (_, 1) => {
                    let value = Self::data_read(&before, address, width)?;
                    write_gpr(&mut next, rt, value, u32::from(size == 3));
                    mnemonic = ["ldrb", "ldrh", "ldr32", "ldr"][size as usize];
                }
                (0..=2, 2) => {
                    let value = sign_extend(
                        Self::data_read(&before, address, width)?,
                        (width * 8) as u32,
                    ) as u64;
                    write_gpr(&mut next, rt, value, 1);
                    mnemonic = ["ldrsb", "ldrsh", "ldrsw"][size as usize];
                }
                (0..=1, 3) => {
                    let value = sign_extend(
                        Self::data_read(&before, address, width)?,
                        (width * 8) as u32,
                    ) as u64;
                    write_gpr(&mut next, rt, value, 0);
                    mnemonic = ["ldrsb32", "ldrsh32"][size as usize];
                }
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            }
        } else if bits(raw, 28, 24) == 0b01010 {
            let shift = bits(raw, 23, 22);
            let amount = bits(raw, 15, 10);
            if sf == 0 && amount >= 32 {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            let opcode = bits(raw, 30, 29);
            let invert = bits(raw, 21, 21) != 0;
            let rn = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let mut rm = apply_shift(
                read_gpr(&before, bits(raw, 20, 16) as usize, sf),
                shift,
                amount,
                sf,
            );
            if invert {
                rm = !rm & width_mask(sf);
            }
            let result = match opcode {
                0 | 3 => rn & rm,
                1 => rn | rm,
                _ => rn ^ rm,
            };
            if opcode == 3 {
                next.nzcv = logical_flags(result, sf);
            }
            write_gpr(&mut next, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match (opcode, invert) {
                (0, false) => "and",
                (0, true) => "bic",
                (1, false) => "orr",
                (1, true) => "orn",
                (2, false) => "eor",
                (2, true) => "eon",
                (3, false) => "ands",
                _ => "bics",
            };
        } else if bits(raw, 28, 24) == 0b01011 && bits(raw, 21, 21) == 0 {
            let shift = bits(raw, 23, 22);
            let amount = bits(raw, 15, 10);
            if shift == 3 || (sf == 0 && amount >= 32) {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            let subtract = bits(raw, 30, 30) != 0;
            let set_flags = bits(raw, 29, 29) != 0;
            let rn = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let rm = apply_shift(
                read_gpr(&before, bits(raw, 20, 16) as usize, sf),
                shift,
                amount,
                sf,
            );
            let result = if set_flags {
                let (result, flags) = arithmetic_flags(rn, rm, sf, subtract);
                next.nzcv = flags;
                result
            } else if subtract {
                rn.wrapping_sub(rm) & width_mask(sf)
            } else {
                rn.wrapping_add(rm) & width_mask(sf)
            };
            write_gpr(&mut next, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match (subtract, set_flags) {
                (false, false) => "add",
                (false, true) => "adds",
                (true, false) => "sub",
                (true, true) => "subs",
            };
        } else if bits(raw, 30, 30) == 0 && bits(raw, 28, 21) == 0b1101_0110 {
            if bits(raw, 29, 29) != 0 {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            let width = if sf == 0 { 32 } else { 64 };
            let rn = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let rm = read_gpr(&before, bits(raw, 20, 16) as usize, sf);
            let result = match bits(raw, 15, 10) {
                2 => rn.checked_div(rm).unwrap_or(0),
                3 => {
                    if rm == 0 {
                        0
                    } else {
                        let a = sign_extend(rn, width);
                        let b = sign_extend(rm, width);
                        a.checked_div(b).unwrap_or(a) as u64
                    }
                }
                8 => apply_shift(rn, 0, (rm % width as u64) as u32, sf),
                9 => apply_shift(rn, 1, (rm % width as u64) as u32, sf),
                10 => apply_shift(rn, 2, (rm % width as u64) as u32, sf),
                11 => apply_shift(rn, 3, (rm % width as u64) as u32, sf),
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            };
            write_gpr(&mut next, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match bits(raw, 15, 10) {
                2 => "udiv",
                3 => "sdiv",
                8 => "lslv",
                9 => "lsrv",
                10 => "asrv",
                _ => "rorv",
            };
        } else if bits(raw, 30, 30) == 1
            && bits(raw, 29, 29) == 0
            && bits(raw, 28, 21) == 0b1101_0110
            && bits(raw, 20, 16) == 0
        {
            let width = if sf == 0 { 32 } else { 64 };
            let value = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let result = match bits(raw, 15, 10) {
                0 => value.reverse_bits() >> (64 - width),
                1 => {
                    let mut result = 0;
                    for lane in 0..(width / 16) {
                        let part = ((value >> (lane * 16)) & 0xffff) as u16;
                        result |= u64::from(part.swap_bytes()) << (lane * 16);
                    }
                    result
                }
                2 => {
                    if sf == 0 {
                        u64::from((value as u32).swap_bytes())
                    } else {
                        value.swap_bytes()
                    }
                }
                3 if sf == 1 => {
                    u64::from((value as u32).swap_bytes())
                        | (u64::from(((value >> 32) as u32).swap_bytes()) << 32)
                }
                4 => {
                    if sf == 0 {
                        u64::from((value as u32).leading_zeros())
                    } else {
                        u64::from(value.leading_zeros())
                    }
                }
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            };
            write_gpr(&mut next, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match bits(raw, 15, 10) {
                0 => "rbit",
                1 => "rev16",
                2 => "rev",
                3 => "rev32",
                _ => "clz",
            };
        } else if bits(raw, 28, 24) == 0b11011 && bits(raw, 30, 29) == 0 {
            let opcode = bits(raw, 23, 21);
            let rn = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let rm = read_gpr(&before, bits(raw, 20, 16) as usize, sf);
            let result = match opcode {
                0 => {
                    let accumulator = read_gpr(&before, bits(raw, 14, 10) as usize, sf);
                    if bits(raw, 15, 15) == 0 {
                        accumulator.wrapping_add(rn.wrapping_mul(rm))
                    } else {
                        accumulator.wrapping_sub(rn.wrapping_mul(rm))
                    }
                }
                1 if sf == 1 && bits(raw, 15, 10) == 0 => {
                    (((rn as i64 as i128) * (rm as i64 as i128)) >> 64) as u64
                }
                2 if sf == 1 && bits(raw, 15, 10) == 0 => {
                    ((u128::from(rn) * u128::from(rm)) >> 64) as u64
                }
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            };
            write_gpr(&mut next, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match opcode {
                0 if bits(raw, 15, 15) == 0 => "madd",
                0 => "msub",
                1 => "smulh",
                _ => "umulh",
            };
        } else if bits(raw, 28, 21) == 0b1101_0100 && bits(raw, 29, 29) == 0 {
            let operation = bits(raw, 30, 30);
            let operation2 = bits(raw, 11, 10);
            let valid = matches!((operation, operation2), (0, 0 | 1) | (1, 0 | 1));
            if !valid {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            let rn = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let rm = read_gpr(&before, bits(raw, 20, 16) as usize, sf);
            let result = if condition_holds(bits(raw, 15, 12), before.nzcv) {
                rn
            } else {
                match (operation, operation2) {
                    (0, 0) => rm,
                    (0, 1) => rm.wrapping_add(1),
                    (1, 0) => !rm,
                    _ => rm.wrapping_neg(),
                }
            };
            write_gpr(&mut next, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match (operation, operation2) {
                (0, 0) => "csel",
                (0, 1) => "csinc",
                (1, 0) => "csinv",
                _ => "csneg",
            };
        } else if bits(raw, 31, 21) == 0b110_1010_0000 && bits(raw, 4, 0) == 1 {
            mnemonic = "svc";
        } else {
            return Err(AArch64Error::UnknownInstruction { raw, pc });
        }

        next.registers[XZR] = 0;
        self.state = next;
        Ok(AArch64StepTrace {
            pc_before: before.pc,
            pc_after: self.state.pc,
            raw,
            mnemonic,
            state_before: before,
            state_after: self.get_state(),
        })
    }

    pub fn run_checked(
        &mut self,
        max_steps: usize,
    ) -> Result<AArch64ExecutionResult, AArch64Error> {
        let before = self.get_state();
        let mut traces = Vec::new();
        if self.state.halted {
            return Ok(AArch64ExecutionResult {
                halted: true,
                steps: 0,
                traces,
                final_state: before,
            });
        }
        for _ in 0..max_steps {
            match self.step_checked() {
                Ok(trace) => traces.push(trace),
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
            if self.state.halted {
                return Ok(AArch64ExecutionResult {
                    halted: true,
                    steps: traces.len(),
                    traces,
                    final_state: self.get_state(),
                });
            }
        }
        self.state = before;
        Err(AArch64Error::StepLimitExceeded { max_steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding as enc;

    #[test]
    fn checked_bootstrap_lifecycle_is_exact() {
        let mut cpu = AArch64Simulator::new();
        assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
        assert_eq!(cpu.read_register(XZR), Ok(0));
        cpu.write_register(XZR, u64::MAX).unwrap();
        assert_eq!(cpu.read_register(XZR), Ok(0));
        cpu.load_checked(&0_u32.to_be_bytes()).unwrap();
        let result = cpu.run_checked(1).unwrap();
        assert!(result.halted);
        assert_eq!(result.steps, 1);
        assert_eq!(result.traces[0].mnemonic, "halt");
    }

    #[test]
    fn faults_and_bounded_runs_are_atomic() {
        let mut cpu = AArch64Simulator::new();
        cpu.load_checked(&0xffff_ffff_u32.to_be_bytes()).unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.step_checked(),
            Err(AArch64Error::UnknownInstruction {
                raw: 0xffff_ffff,
                pc: 0,
            })
        );
        assert_eq!(cpu.get_state(), before);
        cpu.load_checked(&0xd503_201f_u32.to_be_bytes()).unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.run_checked(1),
            Err(AArch64Error::StepLimitExceeded { max_steps: 1 })
        );
        assert_eq!(cpu.get_state(), before);
    }

    #[test]
    fn immediate_register_and_flag_families_execute() {
        let words = [
            enc::move_wide(1, 2, 0, 0xffff, 0),
            enc::move_wide(1, 3, 1, 0xabcd, 0),
            enc::add_sub_immediate(1, 0, 0, 1, 0, 0, 1),
            enc::add_sub_register(1, 1, 1, (0, 0), 0, 1, XZR as u32),
            enc::logical_register(1, 1, (0, 0), 0, 0, XZR as u32, 2),
            enc::halt(),
        ];
        let mut cpu = AArch64Simulator::new();
        cpu.load_checked(&enc::program(&words)).unwrap();
        let result = cpu.run_checked(words.len()).unwrap();
        assert_eq!(result.steps, words.len());
        assert_eq!(cpu.read_register(0), Ok(0x0000_0000_abcd_ffff));
        assert_eq!(cpu.read_register(1), Ok(0x0000_0000_abce_0000));
        assert_eq!(cpu.read_register(2), Ok(0x0000_0000_abcd_ffff));
        assert_eq!(cpu.nzcv(), 0b0010);
    }

    #[test]
    fn load_store_and_sign_extension_are_checked() {
        let words = [
            enc::move_wide(1, 2, 0, 0x100, 0),
            enc::move_wide(1, 2, 0, 0xff80, 1),
            enc::load_store_unsigned(1, 0, 0, 0, 0, 1),
            enc::load_store_unsigned(1, 0, 2, 0, 0, 2),
            enc::halt(),
        ];
        let mut cpu = AArch64Simulator::new();
        cpu.load_checked(&enc::program(&words)).unwrap();
        cpu.run_checked(words.len()).unwrap();
        assert_eq!(cpu.read_register(2), Ok(0xffff_ffff_ffff_ff80));
        assert_eq!(cpu.read_byte(0x100), Ok(0xff));
        assert_eq!(cpu.read_byte(0x101), Ok(0x80));

        cpu.load_checked(&enc::program(&[enc::load_store_unsigned(3, 0, 1, 0, 0, 1)]))
            .unwrap();
        cpu.write_register(0, MEMORY_SIZE as u64 - 4).unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.step_checked(),
            Err(AArch64Error::MisalignedData {
                address: MEMORY_SIZE as u64 - 4,
                width: 8,
            })
        );
        assert_eq!(cpu.get_state(), before);
    }

    #[test]
    fn all_branch_shapes_choose_exact_targets() {
        let words = [
            enc::compare_branch(1, false, 2, 0),
            u32::MAX,
            enc::test_branch(0, false, 2, 1),
            u32::MAX,
            enc::branch_conditional(2, 0),
            u32::MAX,
            enc::branch(true, 2),
            u32::MAX,
            enc::halt(),
        ];
        let mut cpu = AArch64Simulator::new();
        cpu.load_checked(&enc::program(&words)).unwrap();
        cpu.set_nzcv(0b0100).unwrap();
        let result = cpu.run_checked(5).unwrap();
        assert_eq!(result.steps, 5);
        assert_eq!(cpu.read_register(30), Ok(28));
    }

    #[test]
    fn divide_bit_and_multiply_select_families_execute() {
        let words = [
            enc::move_wide(1, 2, 0, 21, 0),
            enc::move_wide(1, 2, 0, 3, 1),
            enc::data_two_source(1, 1, 2, 0, 2),
            enc::data_two_source(1, 1, 8, 2, 3),
            enc::data_one_source(1, 4, 3, 4),
            enc::multiply_add(1, 0, 1, false, XZR as u32, 0, 5),
            enc::conditional_select(1, false, 5, 0, true, 2, 6),
            enc::svc(7),
            enc::halt(),
        ];
        let mut cpu = AArch64Simulator::new();
        cpu.load_checked(&enc::program(&words)).unwrap();
        cpu.run_checked(words.len()).unwrap();
        assert_eq!(cpu.read_register(2), Ok(7));
        assert_eq!(cpu.read_register(3), Ok(56));
        assert_eq!(cpu.read_register(4), Ok(58));
        assert_eq!(cpu.read_register(5), Ok(63));
        assert_eq!(cpu.read_register(6), Ok(64));
    }
}
