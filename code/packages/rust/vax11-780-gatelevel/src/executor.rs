//! Independent VAX integer transition engine built from gate networks.

use crate::gates;
use vax11_780_simulator::{
    ScalarState, StepTrace, VaxError, VaxState, MEMORY_SIZE, REGISTER_PC, REGISTER_SP,
};

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

pub(crate) fn execute(before: &VaxState) -> Result<(VaxState, StepTrace), VaxError> {
    if before.halted {
        return Err(VaxError::Halted);
    }
    let mut machine = GateMachine {
        state: before.clone(),
    };
    let pc_before = machine.state.regs[REGISTER_PC];
    if !machine.pc_is_loaded(pc_before) {
        return Err(VaxError::FetchOutsideProgram { pc: pc_before });
    }
    let state_before = scalar(before);
    let mut raw = Vec::new();
    let mnemonic = machine.execute_one(pc_before, &mut raw)?;
    let state_after = scalar(&machine.state);
    let trace = StepTrace {
        pc_before,
        pc_after: machine.state.regs[REGISTER_PC],
        raw,
        mnemonic: mnemonic.to_owned(),
        state_before,
        state_after,
    };
    Ok((machine.state, trace))
}

fn scalar(state: &VaxState) -> ScalarState {
    ScalarState {
        regs: state.regs,
        psl: state.psl,
        halted: state.halted,
    }
}

struct GateMachine {
    state: VaxState,
}

impl GateMachine {
    fn pc_is_loaded(&self, pc: u32) -> bool {
        let Some(offset) = pc.checked_sub(self.state.loaded_origin) else {
            return false;
        };
        usize::try_from(offset).is_ok_and(|offset| offset < self.state.loaded_len)
    }

    fn fetch_byte(&mut self, start: u32, raw: &mut Vec<u8>) -> Result<u8, VaxError> {
        let pc = self.state.regs[REGISTER_PC];
        if !self.pc_is_loaded(pc) {
            return Err(VaxError::TruncatedInstruction { pc: start });
        }
        let byte = self.read_byte(pc)?;
        self.state.regs[REGISTER_PC] = gates::add(pc, 1, 4).value;
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
        let fetched_specifier = self.fetch_byte(start, raw)?;
        let specifier = gates::decode_byte_value(fetched_specifier).ok_or_else(|| {
            VaxError::InvalidState("gate operand decoder was not one-hot".to_owned())
        })?;
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
                let base_specifier = match base {
                    Operand::Value { specifier, .. } | Operand::Place { specifier, .. } => {
                        specifier
                    }
                };
                let base_mode = base_specifier >> 4;
                let base_register = usize::from(base_specifier & 0x0f);
                if base_register == register && matches!(base_mode, 7..=9) {
                    return Err(VaxError::ReservedAddressingMode {
                        specifier: base_specifier,
                        pc: start,
                    });
                }
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
                let scaled = gates::multiply(self.state.regs[register], width as u32, 4).value;
                Ok(Operand::Place {
                    place: Place::Memory(gates::add(address, scaled, 4).value),
                    specifier,
                })
            }
            5 if register == REGISTER_PC => Err(VaxError::ReservedAddressingMode {
                specifier,
                pc: start,
            }),
            5 => Ok(Operand::Place {
                place: Place::Register(register),
                specifier,
            }),
            6 if register == REGISTER_PC => Err(VaxError::ReservedAddressingMode {
                specifier,
                pc: start,
            }),
            6 => Ok(Operand::Place {
                place: Place::Memory(self.state.regs[register]),
                specifier,
            }),
            7 => {
                if register == REGISTER_PC {
                    return Err(VaxError::ReservedAddressingMode {
                        specifier,
                        pc: start,
                    });
                }
                self.state.regs[register] =
                    gates::subtract(self.state.regs[register], width as u32, 4).value;
                Ok(Operand::Place {
                    place: Place::Memory(self.state.regs[register]),
                    specifier,
                })
            }
            8 if register == REGISTER_PC => Ok(Operand::Value {
                value: self.fetch_unsigned(start, raw, width)?,
                specifier,
            }),
            8 => {
                let address = self.state.regs[register];
                self.state.regs[register] = gates::add(address, width as u32, 4).value;
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
                let pointer_address = self.state.regs[register];
                self.state.regs[register] = gates::add(pointer_address, 4, 4).value;
                Ok(Operand::Place {
                    place: Place::Memory(self.read_long(pointer_address)?),
                    specifier,
                })
            }
            10..=15 => {
                let displacement_width = match mode {
                    10 | 11 => 1,
                    12 | 13 => 2,
                    _ => 4,
                };
                let displacement = sign_extend(
                    self.fetch_unsigned(start, raw, displacement_width)?,
                    displacement_width,
                );
                let intermediate = gates::add(self.state.regs[register], displacement, 4).value;
                let address = if mode & 1 == 1 {
                    self.read_long(intermediate)?
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
            Operand::Value { value, .. } => Ok(value & gates::width_mask(width)),
            Operand::Place { place, .. } => self.read_place(place, width),
        }
    }

    fn read_place(&self, place: Place, width: usize) -> Result<u32, VaxError> {
        match (place, width) {
            (Place::Register(index), _) => Ok(self.state.regs[index] & gates::width_mask(width)),
            (Place::Memory(address), 1) => Ok(u32::from(self.read_byte(address)?)),
            (Place::Memory(address), 2) => Ok(u32::from(self.read_word(address)?)),
            (Place::Memory(address), 4) => self.read_long(address),
            _ => unreachable!("validated VAX operand width"),
        }
    }

    fn write_place(&mut self, place: Place, width: usize, value: u32) -> Result<(), VaxError> {
        match (place, width) {
            (Place::Register(index), _) => {
                let mask = gates::width_mask(width);
                self.state.regs[index] =
                    gates::bit_or(gates::bit_and(self.state.regs[index], !mask, 4), value, 4);
                Ok(())
            }
            (Place::Memory(address), 1) => self.write_bytes(address, &value.to_le_bytes()[..1]),
            (Place::Memory(address), 2) => self.write_bytes(address, &value.to_le_bytes()[..2]),
            (Place::Memory(address), 4) => self.write_bytes(address, &value.to_le_bytes()),
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
        let sp = gates::subtract(self.state.regs[REGISTER_SP], 4, 4).value;
        if checked_range(sp, 4).is_err() {
            return Err(VaxError::StackOutOfRange { sp });
        }
        self.write_bytes(sp, &value.to_le_bytes())?;
        self.state.regs[REGISTER_SP] = sp;
        Ok(())
    }

    fn pop_long(&mut self) -> Result<u32, VaxError> {
        let sp = self.state.regs[REGISTER_SP];
        let value = self
            .read_long(sp)
            .map_err(|_| VaxError::StackOutOfRange { sp })?;
        self.state.regs[REGISTER_SP] = gates::add(sp, 4, 4).value;
        Ok(value)
    }

    fn set_nz(&mut self, value: u32, width: usize) {
        self.state.psl.n = gates::is_negative(value, width);
        self.state.psl.z = gates::is_zero(value, width);
    }

    fn execute_one(&mut self, start: u32, raw: &mut Vec<u8>) -> Result<&'static str, VaxError> {
        let fetched_opcode = self.fetch_byte(start, raw)?;
        let opcode = gates::decode_byte_value(fetched_opcode).ok_or_else(|| {
            VaxError::InvalidState("gate opcode decoder was not one-hot".to_owned())
        })?;
        if opcode >= 0xfc {
            return Err(VaxError::UnsupportedTwoByteOpcode {
                prefix: opcode,
                pc: start,
            });
        }
        match opcode {
            0x00 => {
                self.state.halted = true;
                Ok("HALT")
            }
            0x01 => Ok("NOP"),
            0x05 => {
                self.state.regs[REGISTER_PC] = self.pop_long()?;
                Ok("RSB")
            }
            0x10 => self.execute_branch(start, raw, 1, true, true, "BSBB"),
            0x11 => self.execute_branch(start, raw, 1, true, false, "BRB"),
            0x12..=0x1f if opcode != 0x16 && opcode != 0x17 => {
                let condition = gates::branch_condition(
                    opcode,
                    self.state.psl.n,
                    self.state.psl.z,
                    self.state.psl.v,
                    self.state.psl.c,
                );
                self.execute_branch(start, raw, 1, condition, false, branch_name(opcode))
            }
            0x16 => {
                let address = self.decode_address(start, raw, 1)?;
                let return_pc = self.state.regs[REGISTER_PC];
                self.push_long(return_pc)?;
                self.state.regs[REGISTER_PC] = address;
                Ok("JSB")
            }
            0x17 => {
                self.state.regs[REGISTER_PC] = self.decode_address(start, raw, 1)?;
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
                self.set_nz(value, 4);
                self.state.psl.v = false;
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
        let base = self.state.regs[REGISTER_PC];
        if subroutine {
            self.push_long(base)?;
        }
        self.state.regs[REGISTER_PC] =
            gates::select_u32(base, gates::add(base, displacement, 4).value, condition);
        Ok(name)
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
            let result = gates::negate(source, width);
            self.write_place(destination, width, result.value)?;
            self.set_nz(result.value, width);
            self.state.psl.v = result.overflow;
            self.state.psl.c = !gates::is_zero(result.value, width);
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
        let (result, name) = match family {
            0 => {
                let result = gates::add(second, first, width);
                self.state.psl.c = result.carry_or_borrow;
                self.state.psl.v = result.overflow;
                (
                    result.value,
                    width_name(if three_operand { "ADD3" } else { "ADD2" }, width),
                )
            }
            1 => {
                let result = gates::subtract(second, first, width);
                self.state.psl.c = result.carry_or_borrow;
                self.state.psl.v = result.overflow;
                (
                    result.value,
                    width_name(if three_operand { "SUB3" } else { "SUB2" }, width),
                )
            }
            2 => {
                let result = gates::multiply(first, second, width);
                self.state.psl.v = result.overflow;
                self.state.psl.c = false;
                (
                    result.value,
                    width_name(if three_operand { "MUL3" } else { "MUL2" }, width),
                )
            }
            3 => {
                let result = gates::divide(second, first, width)
                    .ok_or(VaxError::DivideByZero { pc: start })?;
                self.state.psl.v = result.overflow;
                self.state.psl.c = false;
                (
                    result.value,
                    width_name(if three_operand { "DIV3" } else { "DIV2" }, width),
                )
            }
            4 => {
                self.state.psl.v = false;
                (
                    gates::bit_or(second, first, width),
                    width_name(if three_operand { "BIS3" } else { "BIS2" }, width),
                )
            }
            5 => {
                self.state.psl.v = false;
                (
                    gates::bit_and(second, gates::bit_not(first, width), width),
                    width_name(if three_operand { "BIC3" } else { "BIC2" }, width),
                )
            }
            6 => {
                self.state.psl.v = false;
                (
                    gates::bit_xor(second, first, width),
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
        let index = gates::subtract(selector, base, width).value;
        self.state.psl.n = signed_value(index, width) < signed_value(limit, width);
        self.state.psl.z = gates::is_zero(gates::subtract(index, limit, width).value, width);
        self.state.psl.v = false;
        self.state.psl.c = gates::unsigned_less(index, limit, width);
        let table = self.state.regs[REGISTER_PC];
        if !gates::unsigned_less(limit, index, width) {
            let scaled = gates::multiply(index, 2, 4).value;
            let entry = gates::add(table, scaled, 4).value;
            let displacement = sign_extend(u32::from(self.read_word(entry)?), 2);
            self.state.regs[REGISTER_PC] = gates::add(table, displacement, 4).value;
        } else {
            let scaled = gates::multiply(limit, 2, 4).value;
            self.state.regs[REGISTER_PC] =
                gates::add(table, gates::add(2, scaled, 4).value, 4).value;
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
                self.state.psl.v = false;
                Ok(width_name("MOV", width))
            }
            1 => {
                let first = self.read_operand(start, raw, width)?;
                let second = self.read_operand(start, raw, width)?;
                self.state.psl.n = signed_value(first, width) < signed_value(second, width);
                self.state.psl.z =
                    gates::is_zero(gates::subtract(first, second, width).value, width);
                self.state.psl.v = false;
                self.state.psl.c = gates::unsigned_less(first, second, width);
                Ok(width_name("CMP", width))
            }
            2 => {
                let source = self.read_operand(start, raw, width)?;
                let destination = self.decode_destination(start, raw, width)?;
                let result = gates::bit_not(source, width);
                self.write_place(destination, width, result)?;
                self.set_nz(result, width);
                self.state.psl.v = false;
                Ok(width_name("MCOM", width))
            }
            3 => {
                let mask = self.read_operand(start, raw, width)?;
                let source = self.read_operand(start, raw, width)?;
                self.set_nz(gates::bit_and(source, mask, width), width);
                self.state.psl.v = false;
                Ok(width_name("BIT", width))
            }
            4 => {
                let destination = self.decode_destination(start, raw, width)?;
                self.write_place(destination, width, 0)?;
                self.set_nz(0, width);
                self.state.psl.v = false;
                Ok(width_name("CLR", width))
            }
            5 => {
                let value = self.read_operand(start, raw, width)?;
                self.set_nz(value, width);
                self.state.psl.v = false;
                self.state.psl.c = false;
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
                let arithmetic = if operation == 6 {
                    gates::add(old, 1, width)
                } else {
                    gates::subtract(old, 1, width)
                };
                self.write_place(destination, width, arithmetic.value)?;
                self.set_nz(arithmetic.value, width);
                self.state.psl.v = arithmetic.overflow;
                self.state.psl.c = arithmetic.carry_or_borrow;
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
        self.state.psl.n = false;
        self.state.psl.v = false;
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
            self.set_nz(address, 4);
            self.state.psl.v = false;
        } else {
            let destination = self.decode_destination(start, raw, 4)?;
            self.write_place(destination, 4, address)?;
            self.set_nz(address, 4);
            self.state.psl.v = false;
        }
        Ok(name)
    }

    fn read_byte(&self, address: u32) -> Result<u8, VaxError> {
        Ok(self.state.memory[checked_range(address, 1)?])
    }

    fn read_word(&self, address: u32) -> Result<u16, VaxError> {
        let start = checked_range(address, 2)?;
        Ok(u16::from_le_bytes([
            self.state.memory[start],
            self.state.memory[start + 1],
        ]))
    }

    fn read_long(&self, address: u32) -> Result<u32, VaxError> {
        let start = checked_range(address, 4)?;
        Ok(u32::from_le_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("range length is checked"),
        ))
    }

    fn write_bytes(&mut self, address: u32, bytes: &[u8]) -> Result<(), VaxError> {
        let start = checked_range(address, bytes.len())?;
        self.state.memory[start..start + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
}

fn checked_range(address: u32, width: usize) -> Result<usize, VaxError> {
    let start =
        usize::try_from(address).map_err(|_| VaxError::MemoryOutOfRange { address, width })?;
    start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .map(|_| start)
        .ok_or(VaxError::MemoryOutOfRange { address, width })
}

fn sign_extend(value: u32, width: usize) -> u32 {
    let mask = gates::width_mask(width);
    let value = value & mask;
    if gates::is_negative(value, width) {
        value | !mask
    } else {
        value
    }
}

fn signed_value(value: u32, width: usize) -> i64 {
    i64::from(sign_extend(value, width) as i32)
}

fn branch_name(opcode: u8) -> &'static str {
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
        _ => unreachable!(),
    }
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
        _ => unreachable!("known VAX instruction family and width"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vax11_780_simulator::{encoding, VaxSimulator};

    fn loaded_state(program: &[u8]) -> VaxState {
        let mut cpu = VaxSimulator::new();
        cpu.load_checked(program).unwrap();
        cpu.get_state()
    }

    #[test]
    fn independent_engine_rejects_invalid_lifecycle_and_decode_states() {
        let mut halted = loaded_state(&[encoding::HALT]);
        halted.halted = true;
        assert_eq!(execute(&halted), Err(VaxError::Halted));

        let mut outside = loaded_state(&[encoding::HALT]);
        outside.regs[REGISTER_PC] = 2;
        assert_eq!(
            execute(&outside),
            Err(VaxError::FetchOutsideProgram { pc: 2 })
        );
        let mut origin_cpu = VaxSimulator::new();
        origin_cpu.load_at_checked(&[encoding::HALT], 4).unwrap();
        let mut before_origin = origin_cpu.get_state();
        before_origin.regs[REGISTER_PC] = 3;
        assert_eq!(
            execute(&before_origin),
            Err(VaxError::FetchOutsideProgram { pc: 3 })
        );

        let cases = [
            (vec![0xd0], VaxError::TruncatedInstruction { pc: 0 }),
            (
                vec![0xfc],
                VaxError::UnsupportedTwoByteOpcode {
                    prefix: 0xfc,
                    pc: 0,
                },
            ),
            (vec![0x02], VaxError::UnknownOpcode { opcode: 2, pc: 0 }),
            (
                vec![0xd0, 0x40, 0x40, 0x60, 0x50],
                VaxError::RecursiveIndexMode { pc: 0 },
            ),
            (
                vec![0xd0, 0x40, 0x70, 0x50],
                VaxError::ReservedAddressingMode {
                    specifier: 0x70,
                    pc: 0,
                },
            ),
            (
                vec![0xd0, 0x40, 0x01, 0x50],
                VaxError::ReservedAddressingMode {
                    specifier: 0x40,
                    pc: 0,
                },
            ),
            (
                vec![0xd0, 0x4f, 0x60, 0x50],
                VaxError::ReservedAddressingMode {
                    specifier: 0x4f,
                    pc: 0,
                },
            ),
            (
                vec![0xd0, 0x5f, 0x50],
                VaxError::ReservedAddressingMode {
                    specifier: 0x5f,
                    pc: 0,
                },
            ),
            (
                vec![0xd0, 0x6f, 0x50],
                VaxError::ReservedAddressingMode {
                    specifier: 0x6f,
                    pc: 0,
                },
            ),
            (
                vec![0xd0, 0x7f, 0x50],
                VaxError::ReservedAddressingMode {
                    specifier: 0x7f,
                    pc: 0,
                },
            ),
            (
                vec![0x17, 0x50],
                VaxError::InvalidDestinationMode {
                    specifier: 0x50,
                    pc: 0,
                },
            ),
            (
                vec![0xc0, 1, 2],
                VaxError::InvalidDestinationMode {
                    specifier: 2,
                    pc: 0,
                },
            ),
            (
                vec![0xd0, 1, 2],
                VaxError::InvalidDestinationMode {
                    specifier: 2,
                    pc: 0,
                },
            ),
            (
                vec![0xd6, 1],
                VaxError::InvalidDestinationMode {
                    specifier: 1,
                    pc: 0,
                },
            ),
        ];
        for (program, expected) in cases {
            assert_eq!(execute(&loaded_state(&program)), Err(expected));
        }

        let mut stack = loaded_state(&[0x10, 0]);
        stack.regs[REGISTER_SP] = 0;
        assert_eq!(
            execute(&stack),
            Err(VaxError::StackOutOfRange { sp: u32::MAX - 3 })
        );

        let case_default = encoding::assemble(&[
            encoding::instruction(
                0x8f,
                &[
                    encoding::literal(5),
                    encoding::literal(0),
                    encoding::literal(2),
                ],
            ),
            vec![0; 6],
            vec![encoding::HALT],
        ]);
        assert_eq!(
            execute(&loaded_state(&case_default)).unwrap().1.mnemonic,
            "CASEB"
        );
        assert!(std::panic::catch_unwind(|| branch_name(0)).is_err());
        assert!(std::panic::catch_unwind(|| width_name("INVALID", 1)).is_err());
        let invalid_width = GateMachine {
            state: loaded_state(&[encoding::HALT]),
        };
        assert!(
            std::panic::catch_unwind(|| invalid_width.read_place(Place::Memory(0), 3)).is_err()
        );
        let mut invalid_write_width = invalid_width;
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            invalid_write_width.write_place(Place::Memory(0), 3, 0)
        }))
        .is_err());
    }

    #[test]
    fn independent_engine_covers_subroutine_and_memory_read_widths() {
        let mut bsbb = loaded_state(&[0x10, 0, encoding::HALT]);
        bsbb.regs[REGISTER_SP] = 0x400;
        assert_eq!(execute(&bsbb).unwrap().1.mnemonic, "BSBB");

        for (opcode, address) in [(0x90_u8, 0x200_u32), (0xb0, 0x202)] {
            let program = encoding::assemble(&[
                encoding::instruction(
                    opcode,
                    &[encoding::absolute(address), encoding::register(0)],
                ),
                vec![encoding::HALT],
            ]);
            let mut state = loaded_state(&program);
            state.memory[address as usize] = 0x34;
            state.memory[address as usize + 1] = 0x12;
            assert_eq!(
                execute(&state).unwrap().0.regs[0] & 0xffff,
                if opcode == 0x90 { 0x34 } else { 0x1234 }
            );
        }
    }
}
