//! Independent gate-network MC6800 transition engine.

use crate::{
    gates, Motorola6800Error, Motorola6800State, StepTrace, CC_C, CC_H, CC_I, CC_N, CC_V, CC_Z,
};
use logic_gates::gates::{not_gate, or_gate, xor_gate};

const CC_RESERVED: u8 = 0xc0;

#[derive(Clone, Copy)]
enum Mode {
    Immediate,
    Direct,
    Indexed,
    Extended,
}

pub(crate) fn execute(
    before: &Motorola6800State,
    trace: &StepTrace,
) -> Result<Motorola6800State, Motorola6800Error> {
    let mut transition = Transition {
        state: before.clone(),
    };
    transition.state.pc = gates::add_16(before.pc, u16::from(trace.width), 0).0;
    transition.execute_opcode(trace.raw[0], &trace.raw)?;
    Ok(transition.state)
}

struct Transition {
    state: Motorola6800State,
}

impl Transition {
    fn byte(&self, address: u16) -> u8 {
        self.state.memory[usize::from(address)]
    }

    fn set_byte(&mut self, address: u16, value: u8) {
        self.state.memory[usize::from(address)] = value;
    }

    fn word(&self, address: u16) -> u16 {
        u16::from_be_bytes([
            self.byte(address),
            self.byte(gates::add_16(address, 1, 0).0),
        ])
    }

    fn set_word(&mut self, address: u16, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.set_byte(address, high);
        self.set_byte(gates::add_16(address, 1, 0).0, low);
    }

    fn flag(&self, flag: u8) -> u8 {
        u8::from(self.state.cc & flag != 0)
    }

    fn set_flag(&mut self, flag: u8, signal: u8) {
        let retained = gates::bit_and_8(self.state.cc, gates::bit_not_8(flag));
        let selected = gates::select_8(0, flag, signal);
        self.state.cc = gates::bit_or_8(gates::bit_or_8(retained, selected), CC_RESERVED);
    }

    fn set_nz8(&mut self, value: u8) {
        self.set_flag(CC_N, gates::negative_8(value));
        self.set_flag(CC_Z, gates::is_zero_8(value));
    }

    fn set_nz16(&mut self, value: u16) {
        self.set_flag(CC_N, gates::negative_16(value));
        self.set_flag(CC_Z, gates::is_zero_16(value));
    }

    fn logic8(&mut self, value: u8) {
        self.set_nz8(value);
        self.set_flag(CC_V, 0);
    }

    fn add8(&mut self, left: u8, right: u8, carry: u8) -> u8 {
        let (result, carry_out, overflow, half_carry) = gates::add_8(left, right, carry);
        self.set_nz8(result);
        self.set_flag(CC_C, carry_out);
        self.set_flag(CC_H, half_carry);
        self.set_flag(CC_V, overflow);
        result
    }

    fn sub8(&mut self, left: u8, right: u8, borrow: u8) -> u8 {
        let (result, borrow_out, overflow) = gates::sub_8(left, right, borrow);
        self.set_nz8(result);
        self.set_flag(CC_C, borrow_out);
        self.set_flag(CC_V, overflow);
        result
    }

    fn compare16(&mut self, left: u16, right: u16) {
        let (result, _, overflow) = gates::sub_16(left, right);
        self.set_nz16(result);
        self.set_flag(CC_V, overflow);
    }

    fn push8(&mut self, value: u8) {
        self.set_byte(self.state.sp, value);
        self.state.sp = gates::add_16(self.state.sp, u16::MAX, 0).0;
    }

    fn pull8(&mut self) -> u8 {
        self.state.sp = gates::add_16(self.state.sp, 1, 0).0;
        self.byte(self.state.sp)
    }

    fn push16(&mut self, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.push8(low);
        self.push8(high);
    }

    fn pull16(&mut self) -> u16 {
        let high = self.pull8();
        let low = self.pull8();
        u16::from_be_bytes([high, low])
    }

    fn push_frame(&mut self, pc: u16) {
        self.push16(pc);
        self.push16(self.state.x);
        self.push8(self.state.a);
        self.push8(self.state.b);
        self.push8(self.state.cc);
    }

    fn effective(&self, mode: Mode, raw: &[u8]) -> u16 {
        match mode {
            Mode::Direct => u16::from(raw[1]),
            Mode::Indexed => gates::add_16(self.state.x, u16::from(raw[1]), 0).0,
            Mode::Extended => u16::from_be_bytes([raw[1], raw[2]]),
            Mode::Immediate => 0,
        }
    }

    fn operand8(&self, mode: Mode, raw: &[u8]) -> u8 {
        match mode {
            Mode::Immediate => raw[1],
            _ => self.byte(self.effective(mode, raw)),
        }
    }

    fn operand16(&self, mode: Mode, raw: &[u8]) -> u16 {
        match mode {
            Mode::Immediate => u16::from_be_bytes([raw[1], raw[2]]),
            _ => self.word(self.effective(mode, raw)),
        }
    }

    fn execute_opcode(&mut self, opcode: u8, raw: &[u8]) -> Result<(), Motorola6800Error> {
        match opcode {
            0x01 => {}
            0x06 => self.state.cc = gates::bit_or_8(self.state.a, CC_RESERVED),
            0x07 => self.state.a = self.state.cc,
            0x08 => {
                self.state.x = gates::add_16(self.state.x, 1, 0).0;
                self.set_flag(CC_Z, gates::is_zero_16(self.state.x));
            }
            0x09 => {
                self.state.x = gates::add_16(self.state.x, u16::MAX, 0).0;
                self.set_flag(CC_Z, gates::is_zero_16(self.state.x));
            }
            0x0a => self.set_flag(CC_V, 0),
            0x0b => self.set_flag(CC_V, 1),
            0x0c => self.set_flag(CC_C, 0),
            0x0d => self.set_flag(CC_C, 1),
            0x0e => self.set_flag(CC_I, 0),
            0x0f => self.set_flag(CC_I, 1),
            0x10 => self.state.a = self.sub8(self.state.a, self.state.b, 0),
            0x11 => {
                self.sub8(self.state.a, self.state.b, 0);
            }
            0x16 => {
                self.state.b = self.state.a;
                self.logic8(self.state.b);
            }
            0x17 => {
                self.state.a = self.state.b;
                self.logic8(self.state.a);
            }
            0x19 => self.decimal_adjust(),
            0x1b => self.state.a = self.add8(self.state.a, self.state.b, 0),
            0x20..=0x2f => self.branch(opcode, raw[1]),
            0x30 => self.state.x = gates::add_16(self.state.sp, 1, 0).0,
            0x31 => self.state.sp = gates::add_16(self.state.sp, 1, 0).0,
            0x32 => self.state.a = self.pull8(),
            0x33 => self.state.b = self.pull8(),
            0x34 => self.state.sp = gates::add_16(self.state.sp, u16::MAX, 0).0,
            0x35 => self.state.sp = gates::add_16(self.state.x, u16::MAX, 0).0,
            0x36 => self.push8(self.state.a),
            0x37 => self.push8(self.state.b),
            0x39 => self.state.pc = self.pull16(),
            0x3b => {
                self.state.cc = gates::bit_or_8(self.pull8(), CC_RESERVED);
                self.state.b = self.pull8();
                self.state.a = self.pull8();
                self.state.x = self.pull16();
                self.state.pc = self.pull16();
            }
            0x3e => {
                self.push_frame(self.state.pc);
                self.state.halted = true;
            }
            0x3f => {
                self.push_frame(self.state.pc);
                self.set_flag(CC_I, 1);
                self.state.pc = self.word(0xfffa);
            }
            0x40..=0x5f => self.accumulator_unary(opcode),
            0x60..=0x7f => self.memory_unary(opcode, raw),
            0x80..=0xbf => self.group_a(opcode, raw),
            0xc0..=0xff => self.group_b(opcode, raw),
            _ => {
                return Err(Motorola6800Error::UnknownOpcode {
                    opcode,
                    pc: self.state.pc.wrapping_sub(raw.len() as u16),
                });
            }
        }
        Ok(())
    }

    fn decimal_adjust(&mut self) {
        let original = self.state.a;
        let low = gates::bit_and_8(original, 0x0f);
        let low_adjust = or_gate(gates::greater_than_8(low, 9), self.flag(CC_H));
        let high_adjust = or_gate(gates::greater_than_8(original, 0x99), self.flag(CC_C));
        let correction = gates::bit_or_8(
            gates::select_8(0, 0x06, low_adjust),
            gates::select_8(0, 0x60, high_adjust),
        );
        let (result, carry, _, _) = gates::add_8(original, correction, 0);
        self.state.a = result;
        self.set_nz8(result);
        self.set_flag(CC_C, or_gate(carry, high_adjust));
    }

    fn branch(&mut self, opcode: u8, displacement: u8) {
        let sign = gates::negative_8(displacement);
        let extension = gates::select_16(
            u16::from(displacement),
            0xff00 | u16::from(displacement),
            sign,
        );
        let target = gates::add_16(self.state.pc, extension, 0).0;
        self.state.pc = gates::select_16(
            self.state.pc,
            target,
            gates::branch_selected(opcode, self.state.cc),
        );
    }

    fn unary_value(&mut self, operation: u8, value: u8) -> u8 {
        match operation {
            0x0 => {
                let (result, _, overflow) = gates::sub_8(0, value, 0);
                self.set_nz8(result);
                self.set_flag(CC_V, overflow);
                self.set_flag(CC_C, not_gate(gates::is_zero_8(value)));
                result
            }
            0x3 => {
                let result = gates::bit_not_8(value);
                self.logic8(result);
                self.set_flag(CC_C, 1);
                result
            }
            0x4 => {
                let carry = gates::u8_to_bits(value)[0];
                let result = gates::shift_right_8(value, 0);
                self.set_flag(CC_C, carry);
                self.set_flag(CC_N, 0);
                self.set_flag(CC_Z, gates::is_zero_8(result));
                self.set_flag(CC_V, carry);
                result
            }
            0x6 => {
                let carry_in = self.flag(CC_C);
                let carry_out = gates::u8_to_bits(value)[0];
                let result = gates::shift_right_8(value, carry_in);
                self.set_flag(CC_C, carry_out);
                self.set_nz8(result);
                self.set_flag(CC_V, xor_gate(gates::negative_8(result), carry_out));
                result
            }
            0x7 => {
                let carry = gates::u8_to_bits(value)[0];
                let result = gates::shift_right_8(value, gates::negative_8(value));
                self.set_flag(CC_C, carry);
                self.set_nz8(result);
                self.set_flag(CC_V, xor_gate(gates::negative_8(result), carry));
                result
            }
            0x8 => {
                let carry = gates::negative_8(value);
                let result = gates::shift_left_8(value, 0);
                self.set_flag(CC_C, carry);
                self.set_nz8(result);
                self.set_flag(CC_V, xor_gate(gates::negative_8(result), carry));
                result
            }
            0x9 => {
                let carry_in = self.flag(CC_C);
                let carry_out = gates::negative_8(value);
                let result = gates::shift_left_8(value, carry_in);
                self.set_flag(CC_C, carry_out);
                self.set_nz8(result);
                self.set_flag(CC_V, xor_gate(gates::negative_8(result), carry_out));
                result
            }
            0xa => {
                let result = gates::sub_8(value, 1, 0).0;
                self.set_nz8(result);
                self.set_flag(CC_V, u8::from(value == 0x80));
                result
            }
            0xc => {
                let result = gates::add_8(value, 1, 0).0;
                self.set_nz8(result);
                self.set_flag(CC_V, u8::from(value == 0x7f));
                result
            }
            0xd => {
                self.set_nz8(value);
                self.set_flag(CC_V, 0);
                self.set_flag(CC_C, 0);
                value
            }
            0xf => {
                self.set_flag(CC_N, 0);
                self.set_flag(CC_Z, 1);
                self.set_flag(CC_V, 0);
                self.set_flag(CC_C, 0);
                0
            }
            _ => value,
        }
    }

    fn accumulator_unary(&mut self, opcode: u8) {
        let operation = gates::bit_and_8(opcode, 0x0f);
        let use_b = gates::u8_to_bits(opcode)[4];
        let value = gates::select_8(self.state.a, self.state.b, use_b);
        let result = self.unary_value(operation, value);
        if operation != 0x0d {
            self.state.a = gates::select_8(result, self.state.a, use_b);
            self.state.b = gates::select_8(self.state.b, result, use_b);
        }
    }

    fn memory_unary(&mut self, opcode: u8, raw: &[u8]) {
        let operation = gates::bit_and_8(opcode, 0x0f);
        let mode = if opcode < 0x70 {
            Mode::Indexed
        } else {
            Mode::Extended
        };
        let address = self.effective(mode, raw);
        if operation == 0x0e {
            self.state.pc = address;
            return;
        }
        let value = self.byte(address);
        let result = self.unary_value(operation, value);
        if operation != 0x0d {
            self.set_byte(address, result);
        }
    }

    fn group_mode(opcode: u8) -> Mode {
        match opcode >> 4 {
            0x8 | 0xc => Mode::Immediate,
            0x9 | 0xd => Mode::Direct,
            0xa | 0xe => Mode::Indexed,
            _ => Mode::Extended,
        }
    }

    fn group_a(&mut self, opcode: u8, raw: &[u8]) {
        let mode = Self::group_mode(opcode);
        match gates::bit_and_8(opcode, 0x0f) {
            0x0 => self.state.a = self.sub8(self.state.a, self.operand8(mode, raw), 0),
            0x1 => {
                self.sub8(self.state.a, self.operand8(mode, raw), 0);
            }
            0x2 => {
                self.state.a = self.sub8(self.state.a, self.operand8(mode, raw), self.flag(CC_C))
            }
            0x4 => {
                self.state.a = gates::bit_and_8(self.state.a, self.operand8(mode, raw));
                self.logic8(self.state.a);
            }
            0x5 => self.logic8(gates::bit_and_8(self.state.a, self.operand8(mode, raw))),
            0x6 => {
                self.state.a = self.operand8(mode, raw);
                self.logic8(self.state.a);
            }
            0x7 => {
                let address = self.effective(mode, raw);
                self.set_byte(address, self.state.a);
                self.logic8(self.state.a);
            }
            0x8 => {
                self.state.a = gates::bit_xor_8(self.state.a, self.operand8(mode, raw));
                self.logic8(self.state.a);
            }
            0x9 => {
                self.state.a = self.add8(self.state.a, self.operand8(mode, raw), self.flag(CC_C))
            }
            0xa => {
                self.state.a = gates::bit_or_8(self.state.a, self.operand8(mode, raw));
                self.logic8(self.state.a);
            }
            0xb => self.state.a = self.add8(self.state.a, self.operand8(mode, raw), 0),
            0xc => self.compare16(self.state.x, self.operand16(mode, raw)),
            0xd => {
                let target = if opcode == 0x8d {
                    let displacement = raw[1];
                    let sign = gates::negative_8(displacement);
                    let extension = gates::select_16(
                        u16::from(displacement),
                        0xff00 | u16::from(displacement),
                        sign,
                    );
                    gates::add_16(self.state.pc, extension, 0).0
                } else {
                    self.effective(mode, raw)
                };
                self.push16(self.state.pc);
                self.state.pc = target;
            }
            0xe => {
                self.state.sp = self.operand16(mode, raw);
                self.set_nz16(self.state.sp);
                self.set_flag(CC_V, 0);
            }
            0xf => {
                let address = self.effective(mode, raw);
                self.set_word(address, self.state.sp);
                self.set_nz16(self.state.sp);
                self.set_flag(CC_V, 0);
            }
            _ => {}
        }
    }

    fn group_b(&mut self, opcode: u8, raw: &[u8]) {
        let mode = Self::group_mode(opcode);
        match gates::bit_and_8(opcode, 0x0f) {
            0x0 => self.state.b = self.sub8(self.state.b, self.operand8(mode, raw), 0),
            0x1 => {
                self.sub8(self.state.b, self.operand8(mode, raw), 0);
            }
            0x2 => {
                self.state.b = self.sub8(self.state.b, self.operand8(mode, raw), self.flag(CC_C))
            }
            0x4 => {
                self.state.b = gates::bit_and_8(self.state.b, self.operand8(mode, raw));
                self.logic8(self.state.b);
            }
            0x5 => self.logic8(gates::bit_and_8(self.state.b, self.operand8(mode, raw))),
            0x6 => {
                self.state.b = self.operand8(mode, raw);
                self.logic8(self.state.b);
            }
            0x7 => {
                let address = self.effective(mode, raw);
                self.set_byte(address, self.state.b);
                self.logic8(self.state.b);
            }
            0x8 => {
                self.state.b = gates::bit_xor_8(self.state.b, self.operand8(mode, raw));
                self.logic8(self.state.b);
            }
            0x9 => {
                self.state.b = self.add8(self.state.b, self.operand8(mode, raw), self.flag(CC_C))
            }
            0xa => {
                self.state.b = gates::bit_or_8(self.state.b, self.operand8(mode, raw));
                self.logic8(self.state.b);
            }
            0xb => self.state.b = self.add8(self.state.b, self.operand8(mode, raw), 0),
            0xe => {
                self.state.x = self.operand16(mode, raw);
                self.set_nz16(self.state.x);
                self.set_flag(CC_V, 0);
            }
            0xf => {
                let address = self.effective(mode, raw);
                self.set_word(address, self.state.x);
                self.set_nz16(self.state.x);
                self.set_flag(CC_V, 0);
            }
            _ => {}
        }
    }
}
