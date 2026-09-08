//! PowerPC 601 integer simulator for Layer 07u.

pub mod encoding;

pub const MEMORY_SIZE: usize = 65_536;
pub const HALT_WORD: u32 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerPcState {
    pub cia: u32,
    pub gpr: [u32; 32],
    pub lr: u32,
    pub ctr: u32,
    pub xer: u32,
    pub cr: u32,
    pub memory: Vec<u8>,
    pub halted: bool,
    pub loaded_origin: u32,
    pub loaded_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerPcError {
    Halted,
    InvalidRegister { index: usize },
    InvalidState(String),
    MisalignedProgram { origin: u32 },
    ProgramOutOfRange { origin: u32, length: usize },
    TruncatedInstruction { cia: u32 },
    MemoryOutOfRange { address: u32, width: usize },
    MisalignedAccess { address: u32, width: usize },
    UnknownInstruction { raw: u32, cia: u32 },
    DivisionByZero { signed: bool },
    StepLimitExceeded { max_steps: usize },
}

impl std::fmt::Display for PowerPcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Halted => f.write_str("CPU is halted"),
            Self::InvalidRegister { index } => write!(f, "register {index} is outside r0-r31"),
            Self::InvalidState(message) => f.write_str(message),
            Self::MisalignedProgram { origin } => {
                write!(f, "program origin {origin:#010x} is not word-aligned")
            }
            Self::ProgramOutOfRange { origin, length } => write!(
                f,
                "program of {length} bytes at {origin:#010x} exceeds 64 KiB memory"
            ),
            Self::TruncatedInstruction { cia } => {
                write!(
                    f,
                    "instruction at {cia:#010x} crosses installed program bounds"
                )
            }
            Self::MemoryOutOfRange { address, width } => {
                write!(f, "{width}-byte access at {address:#010x} exceeds memory")
            }
            Self::MisalignedAccess { address, width } => {
                write!(f, "misaligned {width}-byte access at {address:#010x}")
            }
            Self::UnknownInstruction { raw, cia } => {
                write!(f, "unknown instruction {raw:#010x} at {cia:#010x}")
            }
            Self::DivisionByZero { signed } => write!(
                f,
                "{} division by zero",
                if *signed { "signed" } else { "unsigned" }
            ),
            Self::StepLimitExceeded { max_steps } => {
                write!(f, "step limit {max_steps} exceeded")
            }
        }
    }
}

impl std::error::Error for PowerPcError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub cia_before: u32,
    pub cia_after: u32,
    pub raw: u32,
    pub mnemonic: String,
    pub state_before: PowerPcState,
    pub state_after: PowerPcState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub halted: bool,
    pub steps: usize,
    pub traces: Vec<StepTrace>,
    pub final_state: PowerPcState,
}

#[derive(Debug, Clone)]
pub struct PowerPc601Simulator {
    state: PowerPcState,
}

impl Default for PowerPc601Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerPc601Simulator {
    pub fn new() -> Self {
        Self {
            state: PowerPcState {
                cia: 0,
                gpr: [0; 32],
                lr: 0,
                ctr: 0,
                xer: 0,
                cr: 0,
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

    pub fn get_state(&self) -> PowerPcState {
        self.state.clone()
    }

    pub fn restore(&mut self, state: &PowerPcState) -> Result<(), PowerPcError> {
        validate_state(state)?;
        self.state = state.clone();
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), PowerPcError> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), PowerPcError> {
        if origin & 3 != 0 {
            return Err(PowerPcError::MisalignedProgram { origin });
        }
        let start = origin as usize;
        let end = start
            .checked_add(program.len())
            .filter(|end| *end <= MEMORY_SIZE)
            .ok_or(PowerPcError::ProgramOutOfRange {
                origin,
                length: program.len(),
            })?;
        self.reset();
        self.state.memory[start..end].copy_from_slice(program);
        self.state.cia = origin;
        self.state.loaded_origin = origin;
        self.state.loaded_len = program.len();
        Ok(())
    }

    pub fn load(&mut self, program: &[u8]) -> Result<(), PowerPcError> {
        self.load_checked(program)
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u32, PowerPcError> {
        self.state
            .gpr
            .get(index)
            .copied()
            .ok_or(PowerPcError::InvalidRegister { index })
    }

    pub fn write_register_checked(&mut self, index: usize, value: u32) -> Result<(), PowerPcError> {
        let register = self
            .state
            .gpr
            .get_mut(index)
            .ok_or(PowerPcError::InvalidRegister { index })?;
        *register = value;
        Ok(())
    }

    pub fn read_byte_checked(&self, address: u32) -> Result<u8, PowerPcError> {
        Ok(self.state.memory[direct_range(address, 1)?])
    }

    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), PowerPcError> {
        let index = direct_range(address, 1)?;
        self.state.memory[index] = value;
        Ok(())
    }

    pub fn read_half_checked(&self, address: u32) -> Result<u16, PowerPcError> {
        direct_alignment(address, 2)?;
        let start = direct_range(address, 2)?;
        Ok(u16::from_be_bytes(
            self.state.memory[start..start + 2]
                .try_into()
                .expect("validated width"),
        ))
    }

    pub fn write_half_checked(&mut self, address: u32, value: u16) -> Result<(), PowerPcError> {
        direct_alignment(address, 2)?;
        let start = direct_range(address, 2)?;
        self.state.memory[start..start + 2].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    pub fn read_word_checked(&self, address: u32) -> Result<u32, PowerPcError> {
        direct_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        Ok(u32::from_be_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("validated width"),
        ))
    }

    pub fn write_word_checked(&mut self, address: u32, value: u32) -> Result<(), PowerPcError> {
        direct_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        self.state.memory[start..start + 4].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    pub fn step_checked(&mut self) -> Result<StepTrace, PowerPcError> {
        if self.state.halted {
            return Err(PowerPcError::Halted);
        }
        self.validate_fetch()?;
        let before = self.get_state();
        let cia = before.cia;
        let raw = self.fetch_word();
        let outcome = self.execute_raw(raw, cia);
        match outcome {
            Ok(mnemonic) => Ok(StepTrace {
                cia_before: cia,
                cia_after: self.state.cia,
                raw,
                mnemonic: mnemonic.to_string(),
                state_before: before,
                state_after: self.get_state(),
            }),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    pub fn step(&mut self) -> Result<StepTrace, PowerPcError> {
        self.step_checked()
    }

    pub fn run_loaded_checked(
        &mut self,
        max_steps: usize,
    ) -> Result<ExecutionResult, PowerPcError> {
        let before = self.get_state();
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            match self.step_checked() {
                Ok(trace) => {
                    let halted = trace.state_after.halted;
                    traces.push(trace);
                    if halted {
                        return Ok(ExecutionResult {
                            halted: true,
                            steps: traces.len(),
                            traces,
                            final_state: self.get_state(),
                        });
                    }
                }
                Err(error) => {
                    self.state = before;
                    return Err(error);
                }
            }
        }
        self.state = before;
        Err(PowerPcError::StepLimitExceeded { max_steps })
    }

    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, PowerPcError> {
        let before = self.get_state();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.state = before;
                Err(error)
            }
        }
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, PowerPcError> {
        self.run_checked(program, max_steps)
    }

    fn validate_fetch(&self) -> Result<(), PowerPcError> {
        let cia = self.state.cia;
        if cia & 3 != 0 {
            return Err(PowerPcError::MisalignedAccess {
                address: cia,
                width: 4,
            });
        }
        let start = self.state.loaded_origin;
        let end = start + self.state.loaded_len as u32;
        if cia < start || cia.checked_add(4).is_none_or(|next| next > end) {
            return Err(PowerPcError::TruncatedInstruction { cia });
        }
        Ok(())
    }

    fn fetch_word(&self) -> u32 {
        let start = self.state.cia as usize;
        u32::from_be_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("validated fetch"),
        )
    }

    fn execute_raw(&mut self, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
        if raw == HALT_WORD {
            self.state.halted = true;
            return Ok("HALT");
        }
        let opcode = raw >> 26;
        let rd = ((raw >> 21) & 0x1f) as usize;
        let ra = ((raw >> 16) & 0x1f) as usize;
        let immediate = raw as u16;
        let simm = immediate as i16 as i32 as u32;
        let next = cia.wrapping_add(4);
        match opcode {
            8 => {
                let (value, no_borrow) = simm.overflowing_sub(self.state.gpr[ra]);
                self.state.gpr[rd] = value;
                self.set_ca(!no_borrow);
                self.state.cia = next;
                Ok("SUBFIC")
            }
            10 | 11 => {
                let field = ((raw >> 23) & 7) as usize;
                let left = self.state.gpr[ra];
                let relation = if opcode == 11 {
                    (left as i32).cmp(&(simm as i32))
                } else {
                    left.cmp(&u32::from(immediate))
                };
                self.set_cr_field(field, relation);
                self.state.cia = next;
                Ok(if opcode == 11 { "CMPI" } else { "CMPLI" })
            }
            14 => {
                let base = if ra == 0 { 0 } else { self.state.gpr[ra] };
                self.state.gpr[rd] = base.wrapping_add(simm);
                self.state.cia = next;
                Ok("ADDI")
            }
            15 => {
                let base = if ra == 0 { 0 } else { self.state.gpr[ra] };
                self.state.gpr[rd] = base.wrapping_add(simm.wrapping_shl(16));
                self.state.cia = next;
                Ok("ADDIS")
            }
            16 => self.execute_bc(raw, cia),
            18 => self.execute_b(raw, cia),
            19 => self.execute_bx(raw, cia),
            24..=26 | 28..=29 => {
                let source = self.state.gpr[rd];
                let operand = match opcode {
                    25 | 29 => u32::from(immediate) << 16,
                    _ => u32::from(immediate),
                };
                let value = match opcode {
                    24 | 25 => source | operand,
                    26 => source ^ operand,
                    28 | 29 => source & operand,
                    _ => unreachable!(),
                };
                self.state.gpr[ra] = value;
                if matches!(opcode, 28 | 29) {
                    self.update_cr0(value);
                }
                self.state.cia = next;
                Ok(match opcode {
                    24 => "ORI",
                    25 => "ORIS",
                    26 => "XORI",
                    28 => "ANDI.",
                    29 => "ANDIS.",
                    _ => unreachable!(),
                })
            }
            31 => self.execute_x31(raw, cia),
            32..=44 => self.execute_memory(raw, cia, opcode, rd, ra, simm),
            _ => Err(PowerPcError::UnknownInstruction { raw, cia }),
        }
    }

    fn execute_memory(
        &mut self,
        raw: u32,
        cia: u32,
        opcode: u32,
        reg: usize,
        ra: usize,
        displacement: u32,
    ) -> Result<&'static str, PowerPcError> {
        let base = if ra == 0 { 0 } else { self.state.gpr[ra] };
        let address = base.wrapping_add(displacement);
        let update = matches!(opcode, 33 | 35 | 37 | 39 | 41);
        let mnemonic = match opcode {
            32 | 33 => {
                self.state.gpr[reg] = self.read_word_checked(address)?;
                if opcode == 33 {
                    "LWZU"
                } else {
                    "LWZ"
                }
            }
            34 | 35 => {
                self.state.gpr[reg] = u32::from(self.read_byte_checked(address)?);
                if opcode == 35 {
                    "LBZU"
                } else {
                    "LBZ"
                }
            }
            36 | 37 => {
                self.write_word_checked(address, self.state.gpr[reg])?;
                if opcode == 37 {
                    "STWU"
                } else {
                    "STW"
                }
            }
            38 | 39 => {
                self.write_byte_checked(address, self.state.gpr[reg] as u8)?;
                if opcode == 39 {
                    "STBU"
                } else {
                    "STB"
                }
            }
            40 | 41 => {
                self.state.gpr[reg] = u32::from(self.read_half_checked(address)?);
                if opcode == 41 {
                    "LHZU"
                } else {
                    "LHZ"
                }
            }
            42 => {
                self.state.gpr[reg] = self.read_half_checked(address)? as i16 as i32 as u32;
                "LHA"
            }
            44 => {
                self.write_half_checked(address, self.state.gpr[reg] as u16)?;
                "STH"
            }
            _ => return Err(PowerPcError::UnknownInstruction { raw, cia }),
        };
        if update {
            self.state.gpr[ra] = address;
        }
        self.state.cia = cia.wrapping_add(4);
        Ok(mnemonic)
    }

    fn execute_b(&mut self, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
        let offset = (((raw & 0x03ff_fffc) as i32) << 6) >> 6;
        if raw & 1 != 0 {
            self.state.lr = cia.wrapping_add(4);
        }
        self.state.cia = if raw & 2 != 0 {
            offset as u32
        } else {
            cia.wrapping_add(offset as u32)
        };
        Ok(if raw & 1 != 0 { "BL" } else { "B" })
    }

    fn execute_bc(&mut self, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
        let bo = (raw >> 21) & 31;
        let bi = (raw >> 16) & 31;
        let offset = (((raw & 0x0000_fffc) as i32) << 16) >> 16;
        let taken = self.eval_branch(bo, bi);
        if raw & 1 != 0 {
            self.state.lr = cia.wrapping_add(4);
        }
        self.state.cia = if taken {
            if raw & 2 != 0 {
                offset as u32
            } else {
                cia.wrapping_add(offset as u32)
            }
        } else {
            cia.wrapping_add(4)
        };
        Ok("BC")
    }

    fn execute_bx(&mut self, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
        let bo = (raw >> 21) & 31;
        let bi = (raw >> 16) & 31;
        let xo = (raw >> 1) & 0x3ff;
        let target = match xo {
            16 => self.state.lr & !3,
            528 => self.state.ctr & !3,
            _ => return Err(PowerPcError::UnknownInstruction { raw, cia }),
        };
        let taken = self.eval_branch(bo, bi);
        if raw & 1 != 0 {
            self.state.lr = cia.wrapping_add(4);
        }
        self.state.cia = if taken { target } else { cia.wrapping_add(4) };
        Ok(if xo == 16 { "BCLR" } else { "BCCTR" })
    }

    fn execute_x31(&mut self, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
        let rs = ((raw >> 21) & 31) as usize;
        let ra = ((raw >> 16) & 31) as usize;
        let rb = ((raw >> 11) & 31) as usize;
        let xo10 = (raw >> 1) & 0x3ff;
        let xo9 = xo10 & 0x1ff;
        let next = cia.wrapping_add(4);
        let mut result = None;
        let mut destination = rs;
        let mnemonic = match xo10 {
            0 | 32 => {
                let field = ((raw >> 23) & 7) as usize;
                let relation = if xo10 == 0 {
                    (self.state.gpr[ra] as i32).cmp(&(self.state.gpr[rb] as i32))
                } else {
                    self.state.gpr[ra].cmp(&self.state.gpr[rb])
                };
                self.set_cr_field(field, relation);
                if xo10 == 0 {
                    "CMP"
                } else {
                    "CMPL"
                }
            }
            19 => {
                result = Some(self.state.cr);
                "MFCR"
            }
            24 | 26 | 28 | 124 | 316 | 444 | 476 | 536 | 792 | 824 => {
                destination = ra;
                let source = self.state.gpr[rs];
                let value = match xo10 {
                    24 => {
                        let n = self.state.gpr[rb] & 63;
                        if n >= 32 {
                            0
                        } else {
                            source << n
                        }
                    }
                    26 => source.leading_zeros(),
                    28 => source & self.state.gpr[rb],
                    124 => !(source | self.state.gpr[rb]),
                    316 => source ^ self.state.gpr[rb],
                    444 => source | self.state.gpr[rb],
                    476 => !(source & self.state.gpr[rb]),
                    536 => {
                        let n = self.state.gpr[rb] & 63;
                        if n >= 32 {
                            0
                        } else {
                            source >> n
                        }
                    }
                    792 | 824 => {
                        let n = if xo10 == 824 {
                            rb as u32
                        } else {
                            (self.state.gpr[rb] & 63).min(31)
                        };
                        let shifted_out = n != 0 && source & ((1_u32 << n) - 1) != 0;
                        self.set_ca((source as i32) < 0 && shifted_out);
                        ((source as i32) >> n) as u32
                    }
                    _ => unreachable!(),
                };
                result = Some(value);
                if raw & 1 != 0 {
                    self.update_cr0(value);
                }
                "LOGIC"
            }
            144 => {
                let mask = ((raw >> 12) & 0xff) as u8;
                for field in 0..8 {
                    if mask & (0x80 >> field) != 0 {
                        let shift = 28 - field * 4;
                        self.state.cr = (self.state.cr & !(0xf << shift))
                            | (self.state.gpr[rs] & (0xf << shift));
                    }
                }
                "MTCRF"
            }
            339 => {
                let spr = decode_spr(raw);
                result = Some(match spr {
                    1 => self.state.xer,
                    8 => self.state.lr,
                    9 => self.state.ctr,
                    _ => 0,
                });
                "MFSPR"
            }
            467 => {
                match decode_spr(raw) {
                    1 => self.state.xer = self.state.gpr[rs],
                    8 => self.state.lr = self.state.gpr[rs],
                    9 => self.state.ctr = self.state.gpr[rs],
                    _ => {}
                }
                "MTSPR"
            }
            _ => match xo9 {
                10 | 40 | 104 | 138 | 235 | 266 | 459 | 491 => {
                    let a = self.state.gpr[ra];
                    let b = self.state.gpr[rb];
                    let value = match xo9 {
                        10 => {
                            let (v, carry) = a.overflowing_add(b);
                            self.set_ca(carry);
                            v
                        }
                        40 => b.wrapping_sub(a),
                        104 => a.wrapping_neg(),
                        138 => {
                            let carry_in = u32::from(self.state.xer & (1 << 29) != 0);
                            let full = u64::from(a) + u64::from(b) + u64::from(carry_in);
                            self.set_ca(full > u64::from(u32::MAX));
                            full as u32
                        }
                        235 => (a as i32).wrapping_mul(b as i32) as u32,
                        266 => a.wrapping_add(b),
                        459 => {
                            if b == 0 {
                                return Err(PowerPcError::DivisionByZero { signed: false });
                            }
                            a / b
                        }
                        491 => {
                            if b == 0 {
                                return Err(PowerPcError::DivisionByZero { signed: true });
                            }
                            (a as i32).wrapping_div(b as i32) as u32
                        }
                        _ => unreachable!(),
                    };
                    result = Some(value);
                    "ARITH"
                }
                _ => return Err(PowerPcError::UnknownInstruction { raw, cia }),
            },
        };
        if let Some(value) = result {
            self.state.gpr[destination] = value;
        }
        self.state.cia = next;
        Ok(mnemonic)
    }

    fn eval_branch(&mut self, bo: u32, bi: u32) -> bool {
        let ctr_ok = if bo & 0x10 == 0 {
            self.state.ctr = self.state.ctr.wrapping_sub(1);
            (self.state.ctr != 0) ^ (bo & 8 != 0)
        } else {
            true
        };
        let condition_ok = if bo & 4 != 0 {
            true
        } else {
            let cr_bit = self.state.cr & (1 << (31 - bi)) != 0;
            cr_bit == (bo & 2 != 0)
        };
        ctr_ok && condition_ok
    }

    fn set_ca(&mut self, value: bool) {
        self.state.xer = (self.state.xer & !(1 << 29)) | if value { 1 << 29 } else { 0 };
    }

    fn set_cr_field(&mut self, field: usize, relation: std::cmp::Ordering) {
        let nibble = match relation {
            std::cmp::Ordering::Less => 8,
            std::cmp::Ordering::Greater => 4,
            std::cmp::Ordering::Equal => 2,
        } | u32::from(self.state.xer & (1 << 31) != 0);
        let shift = 28 - field * 4;
        self.state.cr = (self.state.cr & !(0xf << shift)) | (nibble << shift);
    }

    fn update_cr0(&mut self, value: u32) {
        self.set_cr_field(0, (value as i32).cmp(&0));
    }
}

fn decode_spr(raw: u32) -> u32 {
    let encoded = (raw >> 11) & 0x3ff;
    ((encoded & 0x1f) << 5) | (encoded >> 5)
}

fn validate_state(state: &PowerPcState) -> Result<(), PowerPcError> {
    if state.memory.len() != MEMORY_SIZE {
        return Err(PowerPcError::InvalidState(format!(
            "memory must contain exactly {MEMORY_SIZE} bytes"
        )));
    }
    if state.cia as usize >= MEMORY_SIZE || state.cia & 3 != 0 {
        return Err(PowerPcError::InvalidState(
            "CIA must be aligned inside memory".into(),
        ));
    }
    if state.loaded_origin & 3 != 0
        || state.loaded_origin as usize >= MEMORY_SIZE
        || state.loaded_origin as usize + state.loaded_len > MEMORY_SIZE
    {
        return Err(PowerPcError::InvalidState(
            "installed program range is invalid".into(),
        ));
    }
    Ok(())
}

fn direct_alignment(address: u32, width: usize) -> Result<(), PowerPcError> {
    if !address.is_multiple_of(width as u32) {
        return Err(PowerPcError::MisalignedAccess { address, width });
    }
    Ok(())
}

fn direct_range(address: u32, width: usize) -> Result<usize, PowerPcError> {
    let start = address as usize;
    start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .map(|_| start)
        .ok_or(PowerPcError::MemoryOutOfRange { address, width })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{assemble, b_form, d_form, halt, i_form, x_form, xfx_form, xo_form};

    #[test]
    fn big_endian_addi_ori_and_halt() {
        let program = assemble(&[d_form(14, 1, 0, 42), d_form(24, 1, 1, 0x100), halt()]);
        let mut cpu = PowerPc601Simulator::new();
        let result = cpu.run_checked(&program, 4).unwrap();
        assert!(result.halted);
        assert_eq!(result.final_state.gpr[1], 0x12a);
        assert_eq!(result.traces[0].raw, 0x3820_002a);
    }

    #[test]
    fn restore_and_faults_are_atomic() {
        let mut cpu = PowerPc601Simulator::new();
        let before = cpu.get_state();
        let mut invalid = before.clone();
        invalid.memory.pop();
        assert!(cpu.restore(&invalid).is_err());
        assert_eq!(cpu.get_state(), before);
        cpu.load_checked(&assemble(&[0x0400_0000])).unwrap();
        let before = cpu.get_state();
        assert!(cpu.step_checked().is_err());
        assert_eq!(cpu.get_state(), before);
    }

    #[test]
    fn checked_direct_access_is_big_endian() {
        let mut cpu = PowerPc601Simulator::new();
        cpu.write_word_checked(4, 0x0102_0304).unwrap();
        assert_eq!(cpu.read_word_checked(4).unwrap(), 0x0102_0304);
        assert_eq!(cpu.read_byte_checked(4).unwrap(), 1);
        assert!(cpu.read_word_checked(2).is_err());
    }

    #[test]
    fn immediate_compare_memory_and_branch_families() {
        let program = assemble(&[
            d_form(14, 1, 0, 0x100),
            d_form(15, 2, 0, 0x1234),
            d_form(36, 2, 1, 0),
            d_form(32, 3, 1, 0),
            d_form(11, 0, 3, 0),
            b_form(16, 18, 1, 8, false, false),
            d_form(14, 4, 0, 99),
            i_form(18, 8, false, true),
            d_form(14, 5, 0, 77),
            halt(),
        ]);
        let mut cpu = PowerPc601Simulator::new();
        let result = cpu.run_checked(&program, 16).unwrap();
        assert_eq!(result.final_state.gpr[2], 0x1234_0000);
        assert_eq!(result.final_state.gpr[3], 0x1234_0000);
        assert_eq!(result.final_state.gpr[4], 0);
        assert_eq!(result.final_state.gpr[5], 0);
        assert_eq!(result.final_state.lr, 32);
        assert_eq!(
            &result.final_state.memory[0x100..0x104],
            &[0x12, 0x34, 0, 0]
        );
    }

    #[test]
    fn xform_arithmetic_logic_and_spr_families() {
        let program = assemble(&[
            d_form(14, 1, 0, -7),
            d_form(14, 2, 0, 3),
            xo_form(31, 3, 1, 2, false, 266, false),
            xo_form(31, 4, 1, 2, false, 235, false),
            x_form(31, 3, 5, 4, 444, true),
            xfx_form(31, 5, 8, 467),
            xfx_form(31, 6, 8, 339),
            halt(),
        ]);
        let mut cpu = PowerPc601Simulator::new();
        let result = cpu.run_checked(&program, 12).unwrap();
        assert_eq!(result.final_state.gpr[3], (-4_i32) as u32);
        assert_eq!(result.final_state.gpr[4], (-21_i32) as u32);
        assert_eq!(result.final_state.gpr[5], (-1_i32) as u32);
        assert_eq!(result.final_state.lr, u32::MAX);
        assert_eq!(result.final_state.gpr[6], u32::MAX);
        assert_eq!(result.final_state.cr >> 28, 8);
    }

    #[test]
    fn architectural_faults_are_transactional() {
        let program = assemble(&[d_form(14, 1, 0, 1), d_form(32, 2, 1, 0), halt()]);
        let mut cpu = PowerPc601Simulator::new();
        cpu.load_checked(&program).unwrap();
        cpu.step_checked().unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.step_checked(),
            Err(PowerPcError::MisalignedAccess {
                address: 1,
                width: 4
            })
        );
        assert_eq!(cpu.get_state(), before);

        let divide = assemble(&[xo_form(31, 3, 1, 2, false, 459, false)]);
        cpu.load_checked(&divide).unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.step_checked(),
            Err(PowerPcError::DivisionByZero { signed: false })
        );
        assert_eq!(cpu.get_state(), before);
    }
}
