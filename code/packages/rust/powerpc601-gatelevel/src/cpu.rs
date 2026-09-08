//! Exact DFF-backed PowerPC 601 lifecycle scaffold.

use crate::alu;
use crate::register_file::RegisterFile32;
use crate::state::{clock_bit, DffMemory};
use powerpc601_simulator::{
    ExecutionResult, PowerPc601Simulator, PowerPcError, PowerPcState, StepTrace, HALT_WORD,
    MEMORY_SIZE,
};

/// Exact persistent topology: 524,288 memory bits, 1,024 GPR bits, 160 bits
/// for LR/CTR/XER/CR/CIA, and one halt bit.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 32 * 32 + 5 * 32 + 1;

#[derive(Clone, Debug)]
pub struct PowerPc601GateSimulator {
    rf: RegisterFile32,
    memory: DffMemory,
    halt_q: u8,
    loaded_origin: u32,
    loaded_len: usize,
}

impl Default for PowerPc601GateSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerPc601GateSimulator {
    pub fn new() -> Self {
        Self {
            rf: RegisterFile32::new(),
            memory: DffMemory::new(MEMORY_SIZE),
            halt_q: 0,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.rf = RegisterFile32::new();
        self.memory.clear();
        clock_bit(&mut self.halt_q, false);
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    pub fn get_state(&self) -> PowerPcState {
        PowerPcState {
            cia: self.rf.cia(),
            gpr: self.rf.snapshot(),
            lr: self.rf.lr(),
            ctr: self.rf.ctr(),
            xer: self.rf.xer(),
            cr: self.rf.cr(),
            memory: self.memory.snapshot(),
            halted: self.halt_q != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    pub fn restore(&mut self, state: &PowerPcState) -> Result<(), PowerPcError> {
        let mut validator = PowerPc601Simulator::new();
        validator.restore(state)?;
        self.rf.restore(
            &state.gpr, state.lr, state.ctr, state.xer, state.cr, state.cia,
        );
        self.memory.restore_snapshot(&state.memory);
        clock_bit(&mut self.halt_q, state.halted);
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
        Ok(())
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), PowerPcError> {
        self.load_at_checked(program, 0)
    }
    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), PowerPcError> {
        let mut validator = PowerPc601Simulator::new();
        validator.load_at_checked(program, origin)?;
        self.restore(&validator.get_state())
    }
    pub fn load(&mut self, program: &[u8]) -> Result<(), PowerPcError> {
        self.load_checked(program)
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u32, PowerPcError> {
        if index >= 32 {
            return Err(PowerPcError::InvalidRegister { index });
        }
        Ok(self.rf.read(index))
    }

    pub fn write_register_checked(&mut self, index: usize, value: u32) -> Result<(), PowerPcError> {
        if index >= 32 {
            return Err(PowerPcError::InvalidRegister { index });
        }
        self.rf.write(index, value);
        Ok(())
    }

    pub fn read_byte_checked(&self, address: u32) -> Result<u8, PowerPcError> {
        let index = direct_range(address, 1)?;
        Ok(self.memory[index])
    }

    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), PowerPcError> {
        let index = direct_range(address, 1)?;
        self.memory.write(index, value);
        Ok(())
    }

    pub fn read_half_checked(&self, address: u32) -> Result<u16, PowerPcError> {
        direct_alignment(address, 2)?;
        let start = direct_range(address, 2)?;
        Ok(u16::from_be_bytes([
            self.memory[start],
            self.memory[start + 1],
        ]))
    }

    pub fn write_half_checked(&mut self, address: u32, value: u16) -> Result<(), PowerPcError> {
        direct_alignment(address, 2)?;
        let start = direct_range(address, 2)?;
        for (offset, byte) in value.to_be_bytes().into_iter().enumerate() {
            self.memory.write(start + offset, byte);
        }
        Ok(())
    }

    pub fn read_word_checked(&self, address: u32) -> Result<u32, PowerPcError> {
        direct_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        Ok(u32::from_be_bytes(std::array::from_fn(|offset| {
            self.memory[start + offset]
        })))
    }

    pub fn write_word_checked(&mut self, address: u32, value: u32) -> Result<(), PowerPcError> {
        direct_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        for (offset, byte) in value.to_be_bytes().into_iter().enumerate() {
            self.memory.write(start + offset, byte);
        }
        Ok(())
    }

    pub fn step_checked(&mut self) -> Result<StepTrace, PowerPcError> {
        let before = self.get_state();
        if before.halted {
            return Err(PowerPcError::Halted);
        }
        validate_fetch(&before)?;
        let cia = before.cia;
        let raw = u32::from_be_bytes(
            before.memory[cia as usize..cia as usize + 4]
                .try_into()
                .expect("validated fetch"),
        );
        let mut after = before.clone();
        let mnemonic = execute_raw(&mut after, raw, cia)?;
        self.commit_transition(&before, &after);
        Ok(StepTrace {
            cia_before: cia,
            cia_after: after.cia,
            raw,
            mnemonic: mnemonic.to_string(),
            state_before: before,
            state_after: after,
        })
    }

    fn commit_transition(&mut self, before: &PowerPcState, after: &PowerPcState) {
        self.rf.restore(
            &after.gpr, after.lr, after.ctr, after.xer, after.cr, after.cia,
        );
        for (address, (&old, &new)) in before.memory.iter().zip(&after.memory).enumerate() {
            if old != new {
                self.memory.write(address, new);
            }
        }
        clock_bit(&mut self.halt_q, after.halted);
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
                    self.restore(&before).expect("previous state is valid");
                    return Err(error);
                }
            }
        }
        self.restore(&before).expect("previous state is valid");
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
                self.restore(&before).expect("previous state is valid");
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
}

fn validate_fetch(state: &PowerPcState) -> Result<(), PowerPcError> {
    if state.cia & 3 != 0 {
        return Err(PowerPcError::MisalignedAccess {
            address: state.cia,
            width: 4,
        });
    }
    let end = state.loaded_origin + state.loaded_len as u32;
    if state.cia < state.loaded_origin || state.cia.checked_add(4).is_none_or(|next| next > end) {
        return Err(PowerPcError::TruncatedInstruction { cia: state.cia });
    }
    Ok(())
}

fn execute_raw(state: &mut PowerPcState, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
    if raw == HALT_WORD {
        state.halted = true;
        return Ok("HALT");
    }
    let opcode = raw >> 26;
    let rd = ((raw >> 21) & 31) as usize;
    let ra = ((raw >> 16) & 31) as usize;
    let immediate = raw as u16;
    let simm = immediate as i16 as i32 as u32;
    let next = cia.wrapping_add(4);
    match opcode {
        8 => {
            let outcome = alu::sub(simm, state.gpr[ra]);
            state.gpr[rd] = outcome.result;
            set_ca(state, outcome.carry != 0);
            state.cia = next;
            Ok("SUBFIC")
        }
        10 | 11 => {
            let field = ((raw >> 23) & 7) as usize;
            let relation = if opcode == 11 {
                alu::compare_signed(state.gpr[ra], simm)
            } else {
                alu::compare_unsigned(state.gpr[ra], u32::from(immediate))
            };
            set_cr_field(state, field, relation);
            state.cia = next;
            Ok(if opcode == 11 { "CMPI" } else { "CMPLI" })
        }
        14 | 15 => {
            let base = if ra == 0 { 0 } else { state.gpr[ra] };
            let operand = if opcode == 15 {
                alu::shift_left(simm, 16).result
            } else {
                simm
            };
            state.gpr[rd] = alu::add(base, operand, 0).result;
            state.cia = next;
            Ok(if opcode == 14 { "ADDI" } else { "ADDIS" })
        }
        16 => execute_bc(state, raw, cia),
        18 => execute_b(state, raw, cia),
        19 => execute_bx(state, raw, cia),
        24..=26 | 28..=29 => {
            let source = state.gpr[rd];
            let operand = if matches!(opcode, 25 | 29) {
                alu::shift_left(u32::from(immediate), 16).result
            } else {
                u32::from(immediate)
            };
            let value = match opcode {
                24 | 25 => alu::or(source, operand).result,
                26 => alu::xor(source, operand).result,
                28 | 29 => alu::and(source, operand).result,
                _ => unreachable!(),
            };
            state.gpr[ra] = value;
            if matches!(opcode, 28 | 29) {
                update_cr0(state, value);
            }
            state.cia = next;
            Ok(match opcode {
                24 => "ORI",
                25 => "ORIS",
                26 => "XORI",
                28 => "ANDI.",
                29 => "ANDIS.",
                _ => unreachable!(),
            })
        }
        31 => execute_x31(state, raw, cia),
        32..=44 => execute_memory(state, raw, cia, opcode, rd, ra, simm),
        _ => Err(PowerPcError::UnknownInstruction { raw, cia }),
    }
}

fn execute_memory(
    state: &mut PowerPcState,
    raw: u32,
    cia: u32,
    opcode: u32,
    reg: usize,
    ra: usize,
    displacement: u32,
) -> Result<&'static str, PowerPcError> {
    let base = if ra == 0 { 0 } else { state.gpr[ra] };
    let address = alu::add(base, displacement, 0).result;
    let update = matches!(opcode, 33 | 35 | 37 | 39 | 41);
    let mnemonic = match opcode {
        32 | 33 => {
            state.gpr[reg] = state_read_word(state, address)?;
            if opcode == 33 {
                "LWZU"
            } else {
                "LWZ"
            }
        }
        34 | 35 => {
            state.gpr[reg] = u32::from(state_read_byte(state, address)?);
            if opcode == 35 {
                "LBZU"
            } else {
                "LBZ"
            }
        }
        36 | 37 => {
            state_write_word(state, address, state.gpr[reg])?;
            if opcode == 37 {
                "STWU"
            } else {
                "STW"
            }
        }
        38 | 39 => {
            state_write_byte(state, address, state.gpr[reg] as u8)?;
            if opcode == 39 {
                "STBU"
            } else {
                "STB"
            }
        }
        40 | 41 => {
            state.gpr[reg] = u32::from(state_read_half(state, address)?);
            if opcode == 41 {
                "LHZU"
            } else {
                "LHZ"
            }
        }
        42 => {
            state.gpr[reg] = state_read_half(state, address)? as i16 as i32 as u32;
            "LHA"
        }
        44 => {
            state_write_half(state, address, state.gpr[reg] as u16)?;
            "STH"
        }
        _ => return Err(PowerPcError::UnknownInstruction { raw, cia }),
    };
    if update {
        state.gpr[ra] = address;
    }
    state.cia = cia.wrapping_add(4);
    Ok(mnemonic)
}

fn execute_b(state: &mut PowerPcState, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
    let offset = (((raw & 0x03ff_fffc) as i32) << 6) >> 6;
    if raw & 1 != 0 {
        state.lr = alu::add(cia, 4, 0).result;
    }
    state.cia = if raw & 2 != 0 {
        offset as u32
    } else {
        alu::add(cia, offset as u32, 0).result
    };
    Ok(if raw & 1 != 0 { "BL" } else { "B" })
}

fn execute_bc(state: &mut PowerPcState, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
    let bo = (raw >> 21) & 31;
    let bi = (raw >> 16) & 31;
    let offset = (((raw & 0x0000_fffc) as i32) << 16) >> 16;
    let taken = eval_branch(state, bo, bi);
    if raw & 1 != 0 {
        state.lr = alu::add(cia, 4, 0).result;
    }
    state.cia = if taken {
        if raw & 2 != 0 {
            offset as u32
        } else {
            alu::add(cia, offset as u32, 0).result
        }
    } else {
        alu::add(cia, 4, 0).result
    };
    Ok("BC")
}

fn execute_bx(state: &mut PowerPcState, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
    let bo = (raw >> 21) & 31;
    let bi = (raw >> 16) & 31;
    let xo = (raw >> 1) & 0x3ff;
    let target = match xo {
        16 => alu::and(state.lr, 0xffff_fffc).result,
        528 => alu::and(state.ctr, 0xffff_fffc).result,
        _ => return Err(PowerPcError::UnknownInstruction { raw, cia }),
    };
    let taken = eval_branch(state, bo, bi);
    if raw & 1 != 0 {
        state.lr = alu::add(cia, 4, 0).result;
    }
    state.cia = if taken {
        target
    } else {
        alu::add(cia, 4, 0).result
    };
    Ok(if xo == 16 { "BCLR" } else { "BCCTR" })
}

fn execute_x31(state: &mut PowerPcState, raw: u32, cia: u32) -> Result<&'static str, PowerPcError> {
    let rs = ((raw >> 21) & 31) as usize;
    let ra = ((raw >> 16) & 31) as usize;
    let rb = ((raw >> 11) & 31) as usize;
    let xo10 = (raw >> 1) & 0x3ff;
    let xo9 = xo10 & 0x1ff;
    let next = alu::add(cia, 4, 0).result;
    let mut result = None;
    let mut destination = rs;
    let mnemonic = match xo10 {
        0 | 32 => {
            let field = ((raw >> 23) & 7) as usize;
            let relation = if xo10 == 0 {
                alu::compare_signed(state.gpr[ra], state.gpr[rb])
            } else {
                alu::compare_unsigned(state.gpr[ra], state.gpr[rb])
            };
            set_cr_field(state, field, relation);
            if xo10 == 0 {
                "CMP"
            } else {
                "CMPL"
            }
        }
        19 => {
            result = Some(state.cr);
            "MFCR"
        }
        24 | 26 | 28 | 124 | 316 | 444 | 476 | 536 | 792 | 824 => {
            destination = ra;
            let source = state.gpr[rs];
            let value = match xo10 {
                24 => alu::shift_left(source, state.gpr[rb] & 63).result,
                26 => alu::count_leading_zeros(source),
                28 => alu::and(source, state.gpr[rb]).result,
                124 => alu::nor(source, state.gpr[rb]).result,
                316 => alu::xor(source, state.gpr[rb]).result,
                444 => alu::or(source, state.gpr[rb]).result,
                476 => alu::nand(source, state.gpr[rb]).result,
                536 => alu::shift_right(source, state.gpr[rb] & 63).result,
                792 | 824 => {
                    let n = if xo10 == 824 {
                        rb as u32
                    } else {
                        (state.gpr[rb] & 63).min(31)
                    };
                    let shifted_out = n != 0
                        && alu::and(source, alu::sub(alu::shift_left(1, n).result, 1).result)
                            .result
                            != 0;
                    set_ca(state, (source as i32) < 0 && shifted_out);
                    alu::shift_right_arithmetic(source, n).result
                }
                _ => unreachable!(),
            };
            result = Some(value);
            if raw & 1 != 0 {
                update_cr0(state, value);
            }
            "LOGIC"
        }
        144 => {
            let mask = ((raw >> 12) & 0xff) as u8;
            for field in 0..8 {
                if mask & (0x80 >> field) != 0 {
                    let shift = 28 - field * 4;
                    let field_mask = 0xf_u32 << shift;
                    state.cr = alu::or(
                        alu::and(state.cr, !field_mask).result,
                        alu::and(state.gpr[rs], field_mask).result,
                    )
                    .result;
                }
            }
            "MTCRF"
        }
        339 => {
            result = Some(match decode_spr(raw) {
                1 => state.xer,
                8 => state.lr,
                9 => state.ctr,
                _ => 0,
            });
            "MFSPR"
        }
        467 => {
            match decode_spr(raw) {
                1 => state.xer = state.gpr[rs],
                8 => state.lr = state.gpr[rs],
                9 => state.ctr = state.gpr[rs],
                _ => {}
            }
            "MTSPR"
        }
        _ => match xo9 {
            10 | 40 | 104 | 138 | 235 | 266 | 459 | 491 => {
                let a = state.gpr[ra];
                let b = state.gpr[rb];
                let value = match xo9 {
                    10 => {
                        let outcome = alu::add(a, b, 0);
                        set_ca(state, outcome.carry != 0);
                        outcome.result
                    }
                    40 => alu::sub(b, a).result,
                    104 => alu::sub(0, a).result,
                    138 => {
                        let carry_in = u8::from(state.xer & (1 << 29) != 0);
                        let outcome = alu::add(a, b, carry_in);
                        set_ca(state, outcome.carry != 0);
                        outcome.result
                    }
                    235 => alu::multiply_low(a, b),
                    266 => alu::add(a, b, 0).result,
                    459 => alu::divide_unsigned(a, b)
                        .ok_or(PowerPcError::DivisionByZero { signed: false })?,
                    491 => alu::divide_signed(a, b)
                        .ok_or(PowerPcError::DivisionByZero { signed: true })?,
                    _ => unreachable!(),
                };
                result = Some(value);
                "ARITH"
            }
            _ => return Err(PowerPcError::UnknownInstruction { raw, cia }),
        },
    };
    if let Some(value) = result {
        state.gpr[destination] = value;
    }
    state.cia = next;
    Ok(mnemonic)
}

fn eval_branch(state: &mut PowerPcState, bo: u32, bi: u32) -> bool {
    let ctr_ok = if bo & 0x10 == 0 {
        state.ctr = alu::sub(state.ctr, 1).result;
        (state.ctr != 0) ^ (bo & 8 != 0)
    } else {
        true
    };
    let condition_ok = if bo & 4 != 0 {
        true
    } else {
        let cr_bit = state.cr & (1 << (31 - bi)) != 0;
        cr_bit == (bo & 2 != 0)
    };
    ctr_ok && condition_ok
}

fn set_ca(state: &mut PowerPcState, value: bool) {
    state.xer = alu::or(
        alu::and(state.xer, !(1 << 29)).result,
        if value { 1 << 29 } else { 0 },
    )
    .result;
}

fn set_cr_field(state: &mut PowerPcState, field: usize, relation: std::cmp::Ordering) {
    let nibble = match relation {
        std::cmp::Ordering::Less => 8,
        std::cmp::Ordering::Greater => 4,
        std::cmp::Ordering::Equal => 2,
    } | u32::from(state.xer & (1 << 31) != 0);
    let shift = 28 - field * 4;
    let mask = 0xf_u32 << shift;
    state.cr = alu::or(
        alu::and(state.cr, !mask).result,
        alu::shift_left(nibble, shift as u32).result,
    )
    .result;
}

fn update_cr0(state: &mut PowerPcState, value: u32) {
    set_cr_field(state, 0, alu::compare_signed(value, 0));
}

fn decode_spr(raw: u32) -> u32 {
    let encoded = (raw >> 11) & 0x3ff;
    ((encoded & 0x1f) << 5) | (encoded >> 5)
}

fn state_read_byte(state: &PowerPcState, address: u32) -> Result<u8, PowerPcError> {
    Ok(state.memory[direct_range(address, 1)?])
}

fn state_write_byte(state: &mut PowerPcState, address: u32, value: u8) -> Result<(), PowerPcError> {
    let index = direct_range(address, 1)?;
    state.memory[index] = value;
    Ok(())
}

fn state_read_half(state: &PowerPcState, address: u32) -> Result<u16, PowerPcError> {
    direct_alignment(address, 2)?;
    let start = direct_range(address, 2)?;
    Ok(u16::from_be_bytes(
        state.memory[start..start + 2]
            .try_into()
            .expect("validated width"),
    ))
}

fn state_write_half(
    state: &mut PowerPcState,
    address: u32,
    value: u16,
) -> Result<(), PowerPcError> {
    direct_alignment(address, 2)?;
    let start = direct_range(address, 2)?;
    state.memory[start..start + 2].copy_from_slice(&value.to_be_bytes());
    Ok(())
}

fn state_read_word(state: &PowerPcState, address: u32) -> Result<u32, PowerPcError> {
    direct_alignment(address, 4)?;
    let start = direct_range(address, 4)?;
    Ok(u32::from_be_bytes(
        state.memory[start..start + 4]
            .try_into()
            .expect("validated width"),
    ))
}

fn state_write_word(
    state: &mut PowerPcState,
    address: u32,
    value: u32,
) -> Result<(), PowerPcError> {
    direct_alignment(address, 4)?;
    let start = direct_range(address, 4)?;
    state.memory[start..start + 4].copy_from_slice(&value.to_be_bytes());
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
