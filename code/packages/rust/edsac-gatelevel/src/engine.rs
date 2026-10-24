//! Independent EDSAC order engine assembled from repository gate primitives.
//!
//! The `EdsacState` value here is a transient bundle of wires. Persistent
//! storage lives in `EdsacGateLevel`; a successful transition is clocked into
//! its DFF banks only after this combinational network settles.

use arithmetic::adders::full_adder;
use edsac_simulator::{
    EdsacError, EdsacState, Length, Opcode, StepTrace, EVEN_CELL_MASK, MEMORY_CELLS, ODD_CELL_MASK,
};
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

const SLICE_MASK: u16 = 0x7fff;

pub(crate) struct WorkingMachine {
    pub(crate) state: EdsacState,
}

fn bit(value: u128, index: usize) -> u8 {
    ((value >> index) & 1) as u8
}

fn gate_equal(value: u128, expected: u128, width: usize) -> u8 {
    (0..width).fold(1, |equal, index| {
        and_gate(
            equal,
            not_gate(xor_gate(bit(value, index), bit(expected, index))),
        )
    })
}

fn gate_mux(select: u8, when_high: u8, when_low: u8) -> u8 {
    or_gate(
        and_gate(select, when_high),
        and_gate(not_gate(select), when_low),
    )
}

fn wire_field(value: u128, first: usize, width: usize) -> u128 {
    (0..width).fold(0, |field, index| {
        field | (u128::from(bit(value, first + index)) << index)
    })
}

fn gate_store_read(memory: &[u32; MEMORY_CELLS], address: usize) -> u32 {
    let mut output = 0_u32;
    for (location, &word) in memory.iter().enumerate() {
        let selected = gate_equal(address as u128, location as u128, 10);
        for index in 0..18 {
            let driven = and_gate(selected, bit(word.into(), index));
            let combined = or_gate(bit(output.into(), index), driven);
            if combined == 1 {
                output |= 1 << index;
            }
        }
    }
    output
}

fn gate_store_write(memory: &mut [u32; MEMORY_CELLS], address: usize, value: u32) {
    for (location, word) in memory.iter_mut().enumerate() {
        let selected = gate_equal(address as u128, location as u128, 10);
        let width = if location.is_multiple_of(2) { 18 } else { 17 };
        let mut next = 0_u32;
        for index in 0..width {
            let q = gate_mux(
                selected,
                bit(value.into(), index),
                bit((*word).into(), index),
            );
            next |= u32::from(q) << index;
        }
        *word = next;
    }
}

fn gate_even_address(address: usize) -> usize {
    let mut even = 0_usize;
    for index in 0..10 {
        let wire = if index == 0 {
            and_gate(bit(address as u128, index), 0)
        } else {
            bit(address as u128, index)
        };
        even |= usize::from(wire) << index;
    }
    even
}

fn gate_odd_address(even: usize) -> usize {
    let mut odd = 0_usize;
    for index in 0..10 {
        let wire = if index == 0 {
            or_gate(bit(even as u128, index), 1)
        } else {
            bit(even as u128, index)
        };
        odd |= usize::from(wire) << index;
    }
    odd
}

fn gate_add(a: u128, b: u128, carry_in: u8, width: usize) -> (u128, u8) {
    let mut sum = 0_u128;
    let mut carry = carry_in;
    for index in 0..width {
        let (sum_bit, next_carry) = full_adder(bit(a, index), bit(b, index), carry);
        sum |= u128::from(sum_bit) << index;
        carry = next_carry;
    }
    (sum, carry)
}

fn gate_and(a: u128, b: u128, width: usize) -> u128 {
    (0..width).fold(0, |value, index| {
        value | (u128::from(and_gate(bit(a, index), bit(b, index))) << index)
    })
}

fn gate_not(value: u128, width: usize) -> u128 {
    (0..width).fold(0, |result, index| {
        result | (u128::from(not_gate(bit(value, index))) << index)
    })
}

fn gate_twos_complement(value: u128, width: usize) -> u128 {
    gate_add(gate_not(value, width), 1, 0, width).0
}

fn gate_multiply_15(a: u16, b: u16) -> u64 {
    let mut product = 0_u128;
    for multiplier_bit in 0..15 {
        let enabled = bit(u128::from(b), multiplier_bit);
        let partial = (0..15).fold(0_u128, |value, multiplicand_bit| {
            value
                | (u128::from(and_gate(bit(u128::from(a), multiplicand_bit), enabled))
                    << (multiplicand_bit + multiplier_bit))
        });
        product = gate_add(product, partial, 0, 30).0;
    }
    product as u64
}

fn decode_opcode(code: u8) -> Option<Opcode> {
    const OPCODES: [(u8, Opcode); 18] = [
        (3, Opcode::E),
        (4, Opcode::R),
        (5, Opcode::T),
        (6, Opcode::Y),
        (7, Opcode::U),
        (8, Opcode::I),
        (9, Opcode::O),
        (12, Opcode::S),
        (13, Opcode::Z),
        (17, Opcode::F),
        (21, Opcode::H),
        (22, Opcode::N),
        (25, Opcode::L),
        (26, Opcode::X),
        (27, Opcode::G),
        (28, Opcode::A),
        (30, Opcode::C),
        (31, Opcode::V),
    ];
    OPCODES.into_iter().find_map(|(pattern, opcode)| {
        (gate_equal(code.into(), pattern.into(), 5) == 1).then_some(opcode)
    })
}

fn requires_short(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::E | Opcode::Y | Opcode::I | Opcode::O | Opcode::F | Opcode::Z | Opcode::G
    )
}

fn name(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::E => "E",
        Opcode::R => "R",
        Opcode::T => "T",
        Opcode::Y => "Y",
        Opcode::U => "U",
        Opcode::I => "I",
        Opcode::O => "O",
        Opcode::S => "S",
        Opcode::Z => "Z",
        Opcode::F => "F",
        Opcode::H => "H",
        Opcode::N => "N",
        Opcode::L => "L",
        Opcode::X => "X",
        Opcode::G => "G",
        Opcode::A => "A",
        Opcode::C => "C",
        Opcode::V => "V",
    }
}

fn length_suffix(length: Length) -> char {
    match length {
        Length::Short => 'F',
        Length::Long => 'D',
    }
}

fn add_slice(a: u16, b: u16, carry: u8) -> (u16, u8) {
    let (sum, carry) = gate_add(a.into(), b.into(), carry, 15);
    (sum as u16, carry)
}

fn pack_operand(value: [u16; 3]) -> u128 {
    (u128::from(value[0]) << 20) | (u128::from(value[1]) << 5) | u128::from(value[2] >> 10)
}

fn unpack_operand(value: u128) -> [u16; 3] {
    [
        ((value >> 20) & 0x7fff) as u16,
        ((value >> 5) & 0x7fff) as u16,
        ((value & 0x1f) as u16) << 10,
    ]
}

fn pack_accumulator(value: [u16; 5]) -> u128 {
    (u128::from(value[0]) << 56)
        | (u128::from(value[1]) << 41)
        | (u128::from(value[2]) << 26)
        | (u128::from(value[3]) << 11)
        | u128::from(value[4] >> 4)
}

fn unpack_accumulator(value: u128) -> [u16; 5] {
    [
        ((value >> 56) & 0x7fff) as u16,
        ((value >> 41) & 0x7fff) as u16,
        ((value >> 26) & 0x7fff) as u16,
        ((value >> 11) & 0x7fff) as u16,
        ((value & 0x7ff) as u16) << 4,
    ]
}

fn replace_field(original: u32, replacement: u32, first: usize, width: usize) -> u32 {
    let mut result = original;
    for index in first..first + width {
        let selected = bit(replacement.into(), index - first);
        let kept = bit(original.into(), index);
        let output = or_gate(and_gate(kept, 0), and_gate(selected, 1));
        if output == 1 {
            result |= 1 << index;
        } else {
            result &= !(1 << index);
        }
    }
    result
}

impl WorkingMachine {
    pub(crate) fn step_inner(&mut self) -> Result<StepTrace, EdsacError> {
        if self.state.halted {
            return Err(EdsacError::Halted);
        }
        let pc_before = self.state.scr;
        let end = self
            .state
            .loaded_origin
            .saturating_add(self.state.loaded_words);
        if pc_before < self.state.loaded_origin || pc_before >= end {
            return Err(EdsacError::FetchOutsideProgram {
                scr: pc_before,
                origin: self.state.loaded_origin,
                words: self.state.loaded_words,
            });
        }

        let instruction = gate_store_read(&self.state.memory, pc_before) & ODD_CELL_MASK;
        let code = wire_field(instruction.into(), 12, 5) as u8;
        let opcode = decode_opcode(code).ok_or(EdsacError::UnknownOpcode {
            opcode: code,
            instruction,
        })?;
        let length = if bit(instruction.into(), 0) == 0 {
            Length::Short
        } else {
            Length::Long
        };
        if length == Length::Long && requires_short(opcode) {
            return Err(EdsacError::InvalidLength {
                opcode,
                instruction,
            });
        }
        let address = wire_field(instruction.into(), 1, 10) as usize;
        let operand = self.load_operand(address, length);
        let (next_scr, overflow) = gate_add(pc_before as u128, 1, 0, 10);

        self.state.order_tank = instruction;
        self.state.multiplicand = operand;
        self.state.multiplicand_length = length;
        self.state.scr = next_scr as usize;

        let mut description = format!("{} {address}{}", name(opcode), length_suffix(length));
        let mut io_code = None;
        let mut control_transferred = false;
        match opcode {
            Opcode::A => self.add_operand(),
            Opcode::S => {
                self.negate_operand();
                self.add_operand();
            }
            Opcode::H => self.state.multiplier = self.state.multiplicand,
            Opcode::V => self.multiply_add(),
            Opcode::N => {
                self.negate_operand();
                self.multiply_add();
            }
            Opcode::T | Opcode::U => {
                self.store_accumulator(address, length);
                if opcode == Opcode::T {
                    self.state.accumulator = [0; 5];
                }
            }
            Opcode::C => self.collate(),
            Opcode::R | Opcode::L => {
                let count = self.pulse_count(instruction);
                if opcode == Opcode::R {
                    self.shift_right(count);
                } else {
                    self.shift_left(count);
                }
                description = format!("{} {count}", name(opcode));
            }
            Opcode::E | Opcode::G => {
                let negative = bit(self.state.accumulator[0].into(), 14);
                let take = if opcode == Opcode::E {
                    not_gate(negative)
                } else {
                    negative
                };
                if take == 1 {
                    self.state.scr = address;
                    control_transferred = true;
                }
            }
            Opcode::I => {
                let code = *self
                    .state
                    .input
                    .get(self.state.input_cursor)
                    .ok_or(EdsacError::InputExhausted)?;
                self.state.input_cursor += 1;
                let current = gate_store_read(&self.state.memory, address);
                let next = replace_field(current, u32::from(code), 0, 5);
                gate_store_write(&mut self.state.memory, address, next);
                io_code = Some(code);
            }
            Opcode::O => {
                let word = gate_store_read(&self.state.memory, address);
                let code = wire_field(word.into(), 12, 5) as u8;
                self.state.output.push(code);
                self.state.last_output = code;
                io_code = Some(code);
            }
            Opcode::F => {
                let current = gate_store_read(&self.state.memory, address);
                let next = replace_field(current, u32::from(self.state.last_output), 12, 5);
                gate_store_write(&mut self.state.memory, address, next);
                io_code = Some(self.state.last_output);
            }
            Opcode::X => {}
            Opcode::Y => self.round(),
            Opcode::Z => self.state.halted = true,
        }

        if overflow == 1 && !self.state.halted && !control_transferred {
            return Err(EdsacError::SequenceControlOverflow);
        }

        Ok(StepTrace {
            pc_before,
            pc_after: self.state.scr,
            instruction,
            opcode,
            address,
            length,
            mnemonic: name(opcode).to_owned(),
            description,
            io_code,
        })
    }

    fn pulse_count(&self, instruction: u32) -> usize {
        let mut count = 1;
        for index in 0..17 {
            if gate_equal(bit(instruction.into(), index).into(), 1, 1) == 1 {
                break;
            }
            count += 1;
        }
        count
    }

    fn load_operand(&self, address: usize, length: Length) -> [u16; 3] {
        match length {
            Length::Short => {
                let value = gate_store_read(&self.state.memory, address) & ODD_CELL_MASK;
                let mut first = 0_u16;
                let mut second = 0_u16;
                for index in 0..15 {
                    first |= u16::from(bit(value.into(), index + 2)) << index;
                }
                for index in 0..2 {
                    second |= u16::from(bit(value.into(), index)) << (index + 13);
                }
                [first, second, 0]
            }
            Length::Long => {
                let even = gate_even_address(address);
                let junior = gate_store_read(&self.state.memory, even);
                let senior = gate_store_read(&self.state.memory, gate_odd_address(even));
                let mut operand = [0_u16; 3];
                for index in 0..15 {
                    operand[0] |= u16::from(bit(senior.into(), index + 2)) << index;
                }
                for index in 0..13 {
                    operand[1] |= u16::from(bit(junior.into(), index + 5)) << index;
                }
                for index in 0..2 {
                    operand[1] |= u16::from(bit(senior.into(), index)) << (index + 13);
                }
                for index in 0..5 {
                    operand[2] |= u16::from(bit(junior.into(), index)) << (index + 10);
                }
                operand
            }
        }
    }

    fn store_accumulator(&mut self, address: usize, length: Length) {
        let [a0, a1, a2, ..] = self.state.accumulator;
        let mut senior = 0_u32;
        for index in 0..15 {
            senior |= u32::from(bit(a0.into(), index)) << (index + 2);
        }
        for index in 0..2 {
            senior |= u32::from(bit(a1.into(), index + 13)) << index;
        }
        match length {
            Length::Short => gate_store_write(&mut self.state.memory, address, senior),
            Length::Long => {
                let even = gate_even_address(address);
                let mut junior = 0_u32;
                for index in 0..13 {
                    junior |= u32::from(bit(a1.into(), index)) << (index + 5);
                }
                for index in 0..5 {
                    junior |= u32::from(bit(a2.into(), index + 10)) << index;
                }
                gate_store_write(&mut self.state.memory, gate_odd_address(even), senior);
                gate_store_write(&mut self.state.memory, even, junior & EVEN_CELL_MASK);
            }
        }
    }

    fn add_operand(&mut self) {
        let mut carry = 0;
        for index in [2, 1, 0] {
            let (sum, next) = add_slice(
                self.state.accumulator[index],
                self.state.multiplicand[index],
                carry,
            );
            self.state.accumulator[index] = sum;
            carry = next;
        }
    }

    fn round(&mut self) {
        let (sum, mut carry) = add_slice(self.state.accumulator[2], 0x0200, 0);
        self.state.accumulator[2] = sum;
        for index in [1, 0] {
            let (sum, next) = add_slice(self.state.accumulator[index], 0, carry);
            self.state.accumulator[index] = sum;
            carry = next;
        }
    }

    fn collate(&mut self) {
        let mut carry = 0;
        for index in [2, 1, 0] {
            let selected = gate_and(
                self.state.multiplier[index].into(),
                self.state.multiplicand[index].into(),
                15,
            ) as u16;
            let (sum, next) = add_slice(self.state.accumulator[index], selected, carry);
            self.state.accumulator[index] = sum;
            carry = next;
        }
    }

    fn negate_operand(&mut self) {
        self.state.multiplicand = unpack_operand(gate_twos_complement(
            pack_operand(self.state.multiplicand),
            35,
        ));
    }

    fn multiply_add(&mut self) {
        let saved = self.state.accumulator;
        let mut negative = false;
        if bit(self.state.multiplicand[0].into(), 14) == 1 {
            self.negate_operand();
            negative = true;
        }
        let b = self.state.multiplicand;
        self.state.multiplicand = self.state.multiplier;
        if bit(self.state.multiplicand[0].into(), 14) == 1 {
            self.negate_operand();
            negative = !negative;
        }
        let r = self.state.multiplicand;

        let mut product = [0_u64; 5];
        let mut x = gate_multiply_15(b[2], r[2]);
        product[4] = x >> 14;
        x = gate_add(
            gate_multiply_15(b[1], r[2]).into(),
            gate_multiply_15(b[2], r[1]).into(),
            0,
            64,
        )
        .0 as u64;
        product[4] = gate_add(product[4].into(), ((x << 1) & 0x7ffc).into(), 0, 64).0 as u64;
        product[3] = gate_add((x >> 14).into(), (product[4] >> 15).into(), 0, 64).0 as u64;
        x = gate_add(
            gate_add(
                gate_multiply_15(b[0], r[2]).into(),
                gate_multiply_15(b[1], r[1]).into(),
                0,
                64,
            )
            .0,
            gate_multiply_15(b[2], r[0]).into(),
            0,
            64,
        )
        .0 as u64;
        product[3] = gate_add(product[3].into(), ((x << 1) & 0x7ffe).into(), 0, 64).0 as u64;
        product[2] = gate_add((x >> 14).into(), (product[3] >> 15).into(), 0, 64).0 as u64;
        x = gate_add(
            gate_multiply_15(b[0], r[1]).into(),
            gate_multiply_15(b[1], r[0]).into(),
            0,
            64,
        )
        .0 as u64;
        product[2] = gate_add(product[2].into(), ((x << 1) & 0x7ffe).into(), 0, 64).0 as u64;
        product[1] = gate_add((x >> 14).into(), (product[2] >> 15).into(), 0, 64).0 as u64;
        x = gate_multiply_15(b[0], r[0]);
        product[1] = gate_add(product[1].into(), ((x << 1) & 0x7fff).into(), 0, 64).0 as u64;
        product[0] = gate_add((x >> 14).into(), (product[1] >> 15).into(), 0, 64).0 as u64;
        self.state.accumulator = product.map(|value| (value as u16) & SLICE_MASK);

        if negative {
            self.negate_accumulator();
        }
        self.add_full_accumulator(saved);
    }

    fn negate_accumulator(&mut self) {
        self.state.accumulator = unpack_accumulator(gate_twos_complement(
            pack_accumulator(self.state.accumulator),
            71,
        ));
    }

    fn add_full_accumulator(&mut self, addend: [u16; 5]) {
        let mut carry = 0;
        for index in [4, 3, 2, 1, 0] {
            let width = if index == 4 { 11 } else { 15 };
            let shift = if index == 4 { 4 } else { 0 };
            let a = self.state.accumulator[index] >> shift;
            let b = addend[index] >> shift;
            let (sum, next) = gate_add(a.into(), b.into(), carry, width);
            self.state.accumulator[index] = (sum as u16) << shift;
            carry = next;
        }
    }

    fn shift_right(&mut self, count: usize) {
        for _ in 0..count {
            let old = pack_accumulator(self.state.accumulator);
            let mut shifted = u128::from(bit(old, 70)) << 70;
            for index in 0..70 {
                shifted |= u128::from(bit(old, index + 1)) << index;
            }
            self.state.accumulator = unpack_accumulator(shifted);
        }
    }

    fn shift_left(&mut self, count: usize) {
        for _ in 0..count {
            let old = pack_accumulator(self.state.accumulator);
            let mut shifted = 0_u128;
            for index in 1..71 {
                shifted |= u128::from(bit(old, index - 1)) << index;
            }
            self.state.accumulator = unpack_accumulator(shifted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_arithmetic_covers_decode_add_negate_and_multiply() {
        assert_eq!(decode_opcode(28), Some(Opcode::A));
        assert_eq!(decode_opcode(0), None);
        assert_eq!(gate_add(0x7fff, 1, 0, 15), (0, 1));
        assert_eq!(gate_twos_complement(1, 35), (1_u128 << 35) - 1);
        assert_eq!(gate_multiply_15(123, 456), 56_088);
    }

    #[test]
    fn gate_store_mux_and_shift_wires_are_independent_paths() {
        let mut memory = [0_u32; MEMORY_CELLS];
        memory[3] = 0x1_2345;
        assert_eq!(gate_store_read(&memory, 3), 0x1_2345);
        gate_store_write(&mut memory, 3, 0x1_abcd);
        assert_eq!(memory[3], 0x1_abcd);
        assert_eq!(memory[2], 0);
        assert_eq!(gate_even_address(3), 2);
        assert_eq!(gate_odd_address(2), 3);

        let mut machine = WorkingMachine {
            state: edsac_simulator::EdsacSimulator::new().state(),
        };
        machine.state.accumulator = [0x4001, 0, 0, 0, 0];
        machine.shift_right(1);
        assert_eq!(machine.state.accumulator[0], 0x6000);
        machine.shift_left(1);
        assert_eq!(machine.state.accumulator[0], 0x4001);
    }
}
