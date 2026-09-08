//! Independent AArch64 decode and repository-gate execution engine.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xnor_gate, xor_gate};

use aarch64_simulator::{AArch64Error, AArch64State, MEMORY_SIZE, XZR};

use crate::state::{bits64, word64};

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

fn sign_extend(value: u64, width: u32) -> u64 {
    ((value << (64 - width)) as i64 >> (64 - width)) as u64
}

fn gate_boolean(left: u64, right: u64, gate: fn(u8, u8) -> u8) -> u64 {
    let left = bits64(left);
    let right = bits64(right);
    word64(&std::array::from_fn(|bit| gate(left[bit], right[bit])))
}

fn gate_add_with_carry(left: u64, right: u64, carry_in: u8) -> (u64, u8) {
    let output = ripple_carry_adder_with_carry(&bits64(left), &bits64(right), carry_in);
    let sum: [u8; 64] = output.sum.try_into().expect("64-bit ripple result");
    (word64(&sum), output.carry_out)
}

fn gate_add(left: u64, right: u64) -> u64 {
    gate_add_with_carry(left, right, 0).0
}

fn gate_subtract_full(left: u64, right: u64) -> (u64, u8, u8) {
    let left_bits = bits64(left);
    let right_bits = bits64(right);
    let inverted: [u8; 64] = std::array::from_fn(|bit| not_gate(right_bits[bit]));
    let output = ripple_carry_adder_with_carry(&left_bits, &inverted, 1);
    let difference: [u8; 64] = output.sum.try_into().expect("64-bit ripple result");
    let overflow = and_gate(
        xor_gate(left_bits[63], right_bits[63]),
        xor_gate(left_bits[63], difference[63]),
    );
    (word64(&difference), output.carry_out, overflow)
}

fn gate_equal(left: u64, right: u64, width: usize) -> bool {
    let left = bits64(left);
    let right = bits64(right);
    left[..width]
        .iter()
        .zip(&right[..width])
        .map(|(&a, &b)| xnor_gate(a, b))
        .fold(1, and_gate)
        != 0
}

fn arithmetic_flags(a: u64, b: u64, sf: u32, subtract: bool) -> (u64, u8) {
    let width = if sf == 0 { 32 } else { 64 };
    let mask = width_mask(sf);
    let a = a & mask;
    let b = b & mask;
    let (result, carry, overflow) = if subtract {
        let (result, carry, overflow) = gate_subtract_full(a, b);
        (result & mask, carry, overflow)
    } else {
        let (result, carry64) = gate_add_with_carry(a, b, 0);
        let carry = if sf == 0 {
            u8::from(gate_add(a, b) > u64::from(u32::MAX))
        } else {
            carry64
        };
        let sign = width - 1;
        let a_bits = bits64(a);
        let b_bits = bits64(b);
        let result_bits = bits64(result);
        let overflow = and_gate(
            xnor_gate(a_bits[sign], b_bits[sign]),
            xor_gate(a_bits[sign], result_bits[sign]),
        );
        (result & mask, carry, overflow)
    };
    let result_bits = bits64(result);
    let n = result_bits[width - 1];
    let z = u8::from(gate_equal(result, 0, width));
    (result, (n << 3) | (z << 2) | (carry << 1) | overflow)
}

fn condition_holds(condition: u32, nzcv: u8) -> bool {
    let n = (nzcv >> 3) & 1;
    let z = (nzcv >> 2) & 1;
    let c = (nzcv >> 1) & 1;
    let v = nzcv & 1;
    let base = match condition >> 1 {
        0 => z,
        1 => c,
        2 => n,
        3 => v,
        4 => and_gate(c, not_gate(z)),
        5 => xnor_gate(n, v),
        6 => and_gate(xnor_gate(n, v), not_gate(z)),
        _ => 1,
    };
    let invert = u8::from(condition & 1 != 0 && condition != 0x0f);
    xor_gate(base, invert) != 0
}

fn logical_flags(value: u64, sf: u32) -> u8 {
    let width = if sf == 0 { 32 } else { 64 };
    (bits64(value)[width - 1] << 3) | (u8::from(gate_equal(value, 0, width)) << 2)
}

#[derive(Debug, Clone, Copy)]
enum ShiftKind {
    Left,
    Right,
    ArithmeticRight,
}

fn gate_barrel(value: u64, kind: ShiftKind, amount: u32, width: usize) -> u64 {
    let selects = bits64(u64::from(amount));
    let mut current = bits64(value)[..width].to_vec();
    for (stage, select) in selects[..width.ilog2() as usize]
        .iter()
        .copied()
        .enumerate()
    {
        let distance = 1_usize << stage;
        let shifted: Vec<u8> = (0..width)
            .map(|bit| match kind {
                ShiftKind::Left => bit
                    .checked_sub(distance)
                    .map_or(0, |source| current[source]),
                ShiftKind::Right => current.get(bit + distance).copied().unwrap_or(0),
                ShiftKind::ArithmeticRight => current
                    .get(bit + distance)
                    .copied()
                    .unwrap_or(current[width - 1]),
            })
            .collect();
        current = current
            .iter()
            .zip(shifted)
            .map(|(&old, shifted)| mux2(old, shifted, select))
            .collect();
    }
    current
        .iter()
        .enumerate()
        .fold(0, |word, (bit, q)| word | (u64::from(*q) << bit))
}

fn apply_shift(value: u64, kind: u32, amount: u32, sf: u32) -> u64 {
    let width = if sf == 0 { 32 } else { 64 };
    let amount = amount % width as u32;
    match kind {
        0 => gate_barrel(value, ShiftKind::Left, amount, width),
        1 => gate_barrel(value, ShiftKind::Right, amount, width),
        2 => gate_barrel(value, ShiftKind::ArithmeticRight, amount, width),
        _ if amount == 0 => value & width_mask(sf),
        _ => {
            or_gate_word(
                gate_barrel(value, ShiftKind::Right, amount, width),
                gate_barrel(value, ShiftKind::Left, width as u32 - amount, width),
            ) & width_mask(sf)
        }
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
    let element = if rotation == 0 {
        ones
    } else {
        or_gate_word(
            gate_barrel(ones, ShiftKind::Right, rotation, element_width as usize),
            gate_barrel(
                ones,
                ShiftKind::Left,
                element_width - rotation,
                element_width as usize,
            ),
        )
    } & element_mask;
    let mut result = 0;
    let mut position = 0;
    while position < 64 {
        result = or_gate_word(result, element << position);
        position += element_width;
    }
    Some(result)
}

fn gate_negate(value: u64) -> u64 {
    gate_add(gate_boolean(value, u64::MAX, xor_gate), 1)
}

fn bits128(value: u128) -> [u8; 128] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn word128(bits: &[u8; 128]) -> u128 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u128::from(*q) << bit))
}

fn gate_negate_wide(value: u128) -> u128 {
    let bits = bits128(value);
    let inverted: [u8; 128] = std::array::from_fn(|bit| not_gate(bits[bit]));
    let one: [u8; 128] = std::array::from_fn(|bit| u8::from(bit == 0));
    let output = ripple_carry_adder_with_carry(&inverted, &one, 0);
    let result: [u8; 128] = output.sum.try_into().expect("128-bit ripple result");
    word128(&result)
}

fn gate_mul_wide(left: u64, right: u64) -> u128 {
    let left = bits64(left);
    let right = bits64(right);
    let mut accumulator = [0; 128];
    for (shift, select) in right.iter().copied().enumerate() {
        let addend: [u8; 128] = std::array::from_fn(|bit| {
            bit.checked_sub(shift)
                .filter(|source| *source < 64)
                .map_or(0, |source| and_gate(left[source], select))
        });
        let output = ripple_carry_adder_with_carry(&accumulator, &addend, 0);
        accumulator = output.sum.try_into().expect("128-bit multiply stage");
    }
    word128(&accumulator)
}

fn gate_mul_signed(left: u64, right: u64) -> u128 {
    let left_negative = bits64(left)[63] != 0;
    let right_negative = bits64(right)[63] != 0;
    let left_magnitude = if left_negative {
        gate_negate(left)
    } else {
        left
    };
    let right_magnitude = if right_negative {
        gate_negate(right)
    } else {
        right
    };
    let product = gate_mul_wide(left_magnitude, right_magnitude);
    if left_negative ^ right_negative {
        gate_negate_wide(product)
    } else {
        product
    }
}

fn gate_div_unsigned(dividend: u64, divisor: u64, width: usize) -> u64 {
    if gate_equal(divisor, 0, width) {
        return 0;
    }
    let dividend_bits = bits64(dividend);
    let divisor_bits = bits64(divisor);
    let mut remainder = vec![0; width + 1];
    let mut quotient = vec![0; width];
    let mut extended_divisor = divisor_bits[..width].to_vec();
    extended_divisor.push(0);
    for bit in (0..width).rev() {
        for position in (1..=width).rev() {
            remainder[position] = remainder[position - 1];
        }
        remainder[0] = dividend_bits[bit];
        let inverted: Vec<_> = extended_divisor.iter().copied().map(not_gate).collect();
        let trial = ripple_carry_adder_with_carry(&remainder, &inverted, 1);
        let select = trial.carry_out;
        remainder = remainder
            .iter()
            .zip(trial.sum)
            .map(|(&old, difference)| mux2(old, difference, select))
            .collect();
        quotient[bit] = select;
    }
    quotient
        .iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

fn gate_div_signed(dividend: u64, divisor: u64, width: usize) -> u64 {
    if gate_equal(divisor, 0, width) {
        return 0;
    }
    let mask = if width == 64 {
        u64::MAX
    } else {
        u64::from(u32::MAX)
    };
    let negative_dividend = bits64(dividend)[width - 1] != 0;
    let negative_divisor = bits64(divisor)[width - 1] != 0;
    let magnitude = |value: u64, negative| {
        if negative {
            gate_negate(value) & mask
        } else {
            value & mask
        }
    };
    let quotient = gate_div_unsigned(
        magnitude(dividend, negative_dividend),
        magnitude(divisor, negative_divisor),
        width,
    );
    if negative_dividend ^ negative_divisor {
        gate_negate(quotient) & mask
    } else {
        quotient
    }
}

fn gate_reverse(value: u64, operation: u32, width: usize) -> Option<u64> {
    let input = bits64(value);
    let output: [u8; 64] = match operation {
        0 => std::array::from_fn(|bit| {
            if bit < width {
                input[width - bit - 1]
            } else {
                0
            }
        }),
        1 => std::array::from_fn(|bit| {
            if bit < width {
                input[(bit / 16) * 16 + ((bit + 8) % 16)]
            } else {
                0
            }
        }),
        2 => std::array::from_fn(|bit| {
            if bit < width {
                input[width - 8 - (bit / 8) * 8 + bit % 8]
            } else {
                0
            }
        }),
        3 if width == 64 => std::array::from_fn(|bit| {
            let lane = (bit / 32) * 32;
            input[lane + 24 - (bit % 32 / 8) * 8 + bit % 8]
        }),
        4 => {
            let mut seen = 0;
            let mut count = 0;
            for bit in (0..width).rev() {
                count = gate_add(
                    count,
                    u64::from(and_gate(not_gate(seen), not_gate(input[bit]))),
                );
                seen = or_gate(seen, input[bit]);
            }
            return Some(count);
        }
        _ => return None,
    };
    Some(word64(&output))
}

pub(crate) struct WorkingCpu {
    pub(crate) state: AArch64State,
}

impl WorkingCpu {
    fn data_read(&self, address: u64, width: usize) -> Result<u64, AArch64Error> {
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
        for byte in &self.state.memory[address as usize..end as usize] {
            value = or_gate_word(gate_barrel(value, ShiftKind::Left, 8, 64), u64::from(*byte));
        }
        Ok(value)
    }

    fn data_write(&mut self, address: u64, width: usize, value: u64) -> Result<(), AArch64Error> {
        if !address.is_multiple_of(width as u64) {
            return Err(AArch64Error::MisalignedData { address, width });
        }
        let end = address
            .checked_add(width as u64)
            .ok_or(AArch64Error::DataOutOfRange { address, width })?;
        if end > MEMORY_SIZE as u64 {
            return Err(AArch64Error::DataOutOfRange { address, width });
        }
        for (offset, byte) in self.state.memory[address as usize..end as usize]
            .iter_mut()
            .enumerate()
        {
            let shift = (width - offset - 1) * 8;
            *byte = gate_barrel(value, ShiftKind::Right, shift as u32, 64) as u8;
        }
        Ok(())
    }

    pub(crate) fn step_inner(&mut self) -> Result<(u32, &'static str), AArch64Error> {
        if self.state.halted {
            return Err(AArch64Error::Halted);
        }
        let pc = self.state.pc;
        if pc & 3 != 0 {
            return Err(AArch64Error::MisalignedFetch { pc });
        }
        let end = self.state.loaded_origin + self.state.loaded_len as u64;
        if pc < self.state.loaded_origin || pc.checked_add(4).is_none_or(|next| next > end) {
            return Err(AArch64Error::FetchOutsideProgram { pc });
        }
        let start = pc as usize;
        let raw = u32::from_be_bytes(
            self.state.memory[start..start + 4]
                .try_into()
                .expect("checked fetch"),
        );
        let before = self.state.clone();
        let sf = bits(raw, 31, 31);
        self.state.pc = gate_add(pc, 4);
        let mnemonic;

        if raw == 0 {
            self.state.pc = pc;
            self.state.halted = true;
            mnemonic = "halt";
        } else if raw == 0xd503_201f {
            mnemonic = "nop";
        } else if bits(raw, 30, 26) == 0b00101 {
            let encoded = sign_extend(u64::from(bits(raw, 25, 0)), 26);
            let offset = gate_add(gate_add(encoded, encoded), gate_add(encoded, encoded));
            if bits(raw, 31, 31) != 0 {
                self.state.registers[30] = gate_add(pc, 4);
                mnemonic = "bl";
            } else {
                mnemonic = "b";
            }
            self.state.pc = gate_add(pc, offset);
        } else if bits(raw, 31, 24) == 0b0101_0100 && bits(raw, 4, 4) == 0 {
            if bits(raw, 3, 0) == 0x0f {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            if condition_holds(bits(raw, 3, 0), self.state.nzcv) {
                let encoded = sign_extend(u64::from(bits(raw, 23, 5)), 19);
                self.state.pc = gate_add(pc, gate_add(encoded, encoded));
                self.state.pc = gate_add(self.state.pc, gate_add(encoded, encoded));
            }
            mnemonic = "b.cond";
        } else if bits(raw, 30, 25) == 0b011010 {
            let value = read_gpr(&before, bits(raw, 4, 0) as usize, sf);
            let nonzero = bits(raw, 24, 24) != 0;
            if gate_equal(value, 0, if sf == 0 { 32 } else { 64 }) != nonzero {
                let encoded = sign_extend(u64::from(bits(raw, 23, 5)), 19);
                self.state.pc = gate_add(
                    pc,
                    gate_add(gate_add(encoded, encoded), gate_add(encoded, encoded)),
                );
            }
            mnemonic = if nonzero { "cbnz" } else { "cbz" };
        } else if bits(raw, 30, 25) == 0b011011 {
            let value = read_gpr(&before, bits(raw, 4, 0) as usize, 1);
            let bit_number = (bits(raw, 31, 31) << 5) | bits(raw, 23, 19);
            let nonzero = bits64(value)[bit_number as usize] != 0;
            if nonzero == (bits(raw, 24, 24) != 0) {
                let encoded = sign_extend(u64::from(bits(raw, 18, 5)), 14);
                self.state.pc = gate_add(
                    pc,
                    gate_add(gate_add(encoded, encoded), gate_add(encoded, encoded)),
                );
            }
            mnemonic = if bits(raw, 24, 24) != 0 {
                "tbnz"
            } else {
                "tbz"
            };
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
                    self.state.pc = target;
                    mnemonic = "br";
                }
                1 => {
                    self.state.registers[30] = gate_add(pc, 4);
                    self.state.pc = target;
                    mnemonic = "blr";
                }
                2 => {
                    self.state.pc = target;
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
                self.state.nzcv = flags;
                result
            } else if subtract {
                gate_subtract_full(rn, immediate).0 & width_mask(sf)
            } else {
                gate_add(rn, immediate) & width_mask(sf)
            };
            write_gpr(&mut self.state, bits(raw, 4, 0) as usize, result, sf);
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
                0 => gate_boolean(immediate, u64::MAX, xor_gate),
                2 => immediate,
                3 => {
                    let mask = 0xffff_u64 << shift;
                    or_gate_word(
                        gate_boolean(read_gpr(&before, rd, sf), mask, and_not_gate),
                        immediate,
                    )
                }
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            };
            write_gpr(&mut self.state, rd, result, sf);
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
                0 | 3 => gate_boolean(rn, immediate, and_gate),
                1 => gate_boolean(rn, immediate, or_gate),
                _ => gate_boolean(rn, immediate, xor_gate),
            };
            if opcode == 3 {
                self.state.nzcv = logical_flags(result, sf);
            }
            write_gpr(&mut self.state, bits(raw, 4, 0) as usize, result, sf);
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
            let scaled = gate_barrel(u64::from(bits(raw, 21, 10)), ShiftKind::Left, size, 64);
            let address = gate_add(base, scaled);
            match (size, opcode) {
                (_, 0) => {
                    let value = read_gpr(&before, rt, u32::from(size == 3));
                    self.data_write(address, width, value)?;
                    mnemonic = ["strb", "strh", "str32", "str"][size as usize];
                }
                (_, 1) => {
                    let value = self.data_read(address, width)?;
                    write_gpr(&mut self.state, rt, value, u32::from(size == 3));
                    mnemonic = ["ldrb", "ldrh", "ldr32", "ldr"][size as usize];
                }
                (0..=2, 2) => {
                    let value = sign_extend(self.data_read(address, width)?, (width * 8) as u32);
                    write_gpr(&mut self.state, rt, value, 1);
                    mnemonic = ["ldrsb", "ldrsh", "ldrsw"][size as usize];
                }
                (0..=1, 3) => {
                    let value = sign_extend(self.data_read(address, width)?, (width * 8) as u32);
                    write_gpr(&mut self.state, rt, value, 0);
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
                rm = gate_boolean(rm, u64::MAX, xor_gate) & width_mask(sf);
            }
            let result = match opcode {
                0 | 3 => gate_boolean(rn, rm, and_gate),
                1 => gate_boolean(rn, rm, or_gate),
                _ => gate_boolean(rn, rm, xor_gate),
            };
            if opcode == 3 {
                self.state.nzcv = logical_flags(result, sf);
            }
            write_gpr(&mut self.state, bits(raw, 4, 0) as usize, result, sf);
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
                self.state.nzcv = flags;
                result
            } else if subtract {
                gate_subtract_full(rn, rm).0 & width_mask(sf)
            } else {
                gate_add(rn, rm) & width_mask(sf)
            };
            write_gpr(&mut self.state, bits(raw, 4, 0) as usize, result, sf);
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
            let operation = bits(raw, 15, 10);
            let result = match operation {
                2 => gate_div_unsigned(rn, rm, width),
                3 => gate_div_signed(rn, rm, width),
                8 => apply_shift(
                    rn,
                    0,
                    gate_boolean(rm, width as u64 - 1, and_gate) as u32,
                    sf,
                ),
                9 => apply_shift(
                    rn,
                    1,
                    gate_boolean(rm, width as u64 - 1, and_gate) as u32,
                    sf,
                ),
                10 => apply_shift(
                    rn,
                    2,
                    gate_boolean(rm, width as u64 - 1, and_gate) as u32,
                    sf,
                ),
                11 => apply_shift(
                    rn,
                    3,
                    gate_boolean(rm, width as u64 - 1, and_gate) as u32,
                    sf,
                ),
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            };
            write_gpr(&mut self.state, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match operation {
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
            let operation = bits(raw, 15, 10);
            let result = gate_reverse(value, operation, width)
                .ok_or(AArch64Error::UnknownInstruction { raw, pc })?;
            write_gpr(&mut self.state, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match operation {
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
                    let product = gate_mul_wide(rn, rm) as u64;
                    if bits(raw, 15, 15) == 0 {
                        gate_add(accumulator, product)
                    } else {
                        gate_subtract_full(accumulator, product).0
                    }
                }
                1 if sf == 1 && bits(raw, 15, 10) == 0 => (gate_mul_signed(rn, rm) >> 64) as u64,
                2 if sf == 1 && bits(raw, 15, 10) == 0 => (gate_mul_wide(rn, rm) >> 64) as u64,
                _ => return Err(AArch64Error::UnknownInstruction { raw, pc }),
            };
            write_gpr(&mut self.state, bits(raw, 4, 0) as usize, result, sf);
            mnemonic = match opcode {
                0 if bits(raw, 15, 15) == 0 => "madd",
                0 => "msub",
                1 => "smulh",
                _ => "umulh",
            };
        } else if bits(raw, 28, 21) == 0b1101_0100 && bits(raw, 29, 29) == 0 {
            let operation = bits(raw, 30, 30);
            let operation2 = bits(raw, 11, 10);
            if !matches!((operation, operation2), (0, 0 | 1) | (1, 0 | 1)) {
                return Err(AArch64Error::UnknownInstruction { raw, pc });
            }
            let rn = read_gpr(&before, bits(raw, 9, 5) as usize, sf);
            let rm = read_gpr(&before, bits(raw, 20, 16) as usize, sf);
            let result = if condition_holds(bits(raw, 15, 12), before.nzcv) {
                rn
            } else {
                match (operation, operation2) {
                    (0, 0) => rm,
                    (0, 1) => gate_add(rm, 1),
                    (1, 0) => gate_boolean(rm, u64::MAX, xor_gate),
                    _ => gate_negate(rm),
                }
            };
            write_gpr(&mut self.state, bits(raw, 4, 0) as usize, result, sf);
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

        self.state.registers[XZR] = 0;
        debug_assert_eq!(self.state.memory.len(), MEMORY_SIZE);
        Ok((raw, mnemonic))
    }
}

fn and_not_gate(left: u8, right: u8) -> u8 {
    and_gate(left, not_gate(right))
}

fn or_gate_word(left: u64, right: u64) -> u64 {
    gate_boolean(left, right, or_gate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_add_subtract_and_flags_cover_width_edges() {
        assert_eq!(gate_add(u64::MAX, 1), 0);
        assert_eq!(gate_subtract_full(0, 1).0, u64::MAX);
        assert_eq!(arithmetic_flags(u64::MAX, 1, 1, false), (0, 0b0110));
        assert_eq!(
            arithmetic_flags(0, 1, 0, true),
            (u64::from(u32::MAX), 0b1000)
        );
    }
}
