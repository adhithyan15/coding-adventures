//! Independent mixed-width Thumb-2 decode and execution engine.

use arithmetic::adders::ripple_carry_adder_with_carry;
use armv7a_simulator::{Armv7AError, Armv7AState, LR, PC, SP};
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xnor_gate, xor_gate};

use crate::state::{bits32, word32};

const CPSR_N: u32 = 1 << 31;
const CPSR_Z: u32 = 1 << 30;
const CPSR_C: u32 = 1 << 29;
const CPSR_V: u32 = 1 << 28;

pub(crate) struct WorkingCpu {
    pub(crate) state: Armv7AState,
}

impl WorkingCpu {
    fn read_byte(&self, address: u32) -> u8 {
        self.state.memory[address as u16 as usize]
    }

    fn write_byte(&mut self, address: u32, value: u8) {
        self.state.memory[address as u16 as usize] = value;
    }

    pub(crate) fn step_inner(&mut self) -> Result<(u32, usize, String), Armv7AError> {
        let pc = self.state.pc;
        if pc & 1 != 0 {
            return Err(Armv7AError::MisalignedFetch { pc });
        }
        self.check_fetch(pc, 2)?;
        let first = self.read16(pc);
        self.set_pc(gate_add(pc, 2));
        if first == 0 {
            self.state.halted = true;
            return Ok((0, 2, "hlt".into()));
        }
        let top5 = first >> 11;
        let wide = or_gate(
            or_gate(
                u8::from(gate_equal(top5 as u32, 0b11101, 5)),
                u8::from(gate_equal(top5 as u32, 0b11110, 5)),
            ),
            u8::from(gate_equal(top5 as u32, 0b11111, 5)),
        ) != 0;
        if wide {
            self.check_fetch(pc, 4)?;
            let second = self.read16(gate_add(pc, 2));
            self.set_pc(gate_add(pc, 4));
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
            gate_add(instruction_pc, 4)
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
            self.read_byte(gate_add(address, 1)),
        ])
    }

    fn read32(&self, address: u32) -> u32 {
        u32::from_le_bytes([
            self.read_byte(address),
            self.read_byte(gate_add(address, 1)),
            self.read_byte(gate_add(address, 2)),
            self.read_byte(gate_add(address, 3)),
        ])
    }

    fn write16(&mut self, address: u32, value: u16) {
        for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
            self.write_byte(gate_add(address, offset as u32), byte);
        }
    }

    fn write32(&mut self, address: u32, value: u32) {
        for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
            self.write_byte(gate_add(address, offset as u32), byte);
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
        let bits = bits32(value);
        self.set_flag(CPSR_N, bits[31] != 0);
        self.set_flag(CPSR_Z, not_gate(bits.iter().copied().fold(0, or_gate)) != 0);
    }

    fn add_flags(&mut self, left: u32, right: u32, carry: bool) -> u32 {
        let (result, carry_out, overflow) = gate_add_carry(left, right, carry);
        self.set_nz(result);
        self.set_flag(CPSR_C, carry_out);
        self.set_flag(CPSR_V, overflow);
        result
    }

    fn sub_flags(&mut self, left: u32, right: u32, borrow: bool) -> u32 {
        let (result, carry_out, overflow) = gate_subtract_borrow(left, right, borrow);
        self.set_nz(result);
        self.set_flag(CPSR_C, carry_out);
        self.set_flag(CPSR_V, overflow);
        result
    }

    fn condition(&self, condition: u8) -> bool {
        let n = u8::from(self.flag(CPSR_N));
        let z = u8::from(self.flag(CPSR_Z));
        let c = u8::from(self.flag(CPSR_C));
        let v = u8::from(self.flag(CPSR_V));
        let predicates = [
            z,
            not_gate(z),
            c,
            not_gate(c),
            n,
            not_gate(n),
            v,
            not_gate(v),
            and_gate(c, not_gate(z)),
            or_gate(not_gate(c), z),
            xnor_gate(n, v),
            xor_gate(n, v),
            and_gate(not_gate(z), xnor_gate(n, v)),
            or_gate(z, xor_gate(n, v)),
            1,
            0,
        ];
        predicates[condition as usize] != 0
    }

    fn execute16(&mut self, raw: u16, pc: u32) -> Result<String, Armv7AError> {
        if gate_equal((raw >> 13) as u32, 0b000, 3) {
            return self.execute16_shift_add_sub(raw);
        }
        if gate_equal((raw >> 13) as u32, 0b001, 3) {
            return Ok(self.execute16_immediate(raw));
        }
        if gate_equal((raw >> 10) as u32, 0b010000, 6) {
            return Ok(self.execute16_data(raw));
        }
        if gate_equal((raw >> 10) as u32, 0b010001, 6) {
            return Ok(self.execute16_special(raw, pc));
        }
        if gate_equal((raw >> 12) as u32, 0b0101, 4) {
            return Ok(self.execute16_memory_register(raw, pc));
        }
        let immediate_memory = [0b0110, 0b0111, 0b1000]
            .into_iter()
            .any(|prefix| gate_equal((raw >> 12) as u32, prefix, 4));
        if immediate_memory {
            return Ok(self.execute16_memory_immediate(raw, pc));
        }
        if gate_equal((raw >> 12) as u32, 0b1001, 4) {
            return Ok(self.execute16_memory_sp(raw));
        }
        if gate_equal((raw >> 12) as u32, 0b1010, 4) {
            return Ok(self.execute16_adr(raw, pc));
        }
        if gate_equal((raw >> 12) as u32, 0b1011, 4) {
            return self.execute16_misc(raw, pc);
        }
        if gate_equal((raw >> 12) as u32, 0b1100, 4) {
            return self.execute16_multiple(raw);
        }
        if gate_equal((raw >> 12) as u32, 0b1101, 4) {
            let condition = ((raw >> 8) & 0xf) as u8;
            if condition >= 0xe {
                return Err(Armv7AError::UnknownInstruction16 { raw, pc });
            }
            if self.condition(condition) {
                let offset = gate_shift_left_one(sign_extend((raw & 0xff) as u32, 8));
                self.set_pc(gate_add(self.state.pc, offset));
            }
            return Ok(format!("b.cond{condition:x}"));
        }
        if gate_equal((raw >> 11) as u32, 0b11100, 5) {
            let offset = gate_shift_left_one(sign_extend((raw & 0x7ff) as u32, 11));
            self.set_pc(gate_add(self.state.pc, offset));
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
                let result = gate_boolean(left, right, and_gate);
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "ands"
            }
            0x1 => {
                let result = gate_boolean(left, right, xor_gate);
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
                self.set_nz(gate_boolean(left, right, and_gate));
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
                let result = gate_boolean(left, right, or_gate);
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "orrs"
            }
            0xd => {
                let result = gate_multiply(left, right);
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "muls"
            }
            0xe => {
                let result = gate_boolean(left, gate_not(right), and_gate);
                self.state.registers[rdn] = result;
                self.set_nz(result);
                "bics"
            }
            _ => {
                let result = gate_not(right);
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
                self.write_register_operand(rd, gate_add(left, right));
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
        let address = gate_add(self.state.registers[rn], self.state.registers[rm]);
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
        let address = gate_add(self.state.registers[rn], gate_scale(immediate, scale));
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
        let address = gate_add(self.state.registers[SP], gate_scale(raw as u8 as u32, 4));
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
        let immediate = gate_scale(raw as u8 as u32, 4);
        let base = if raw & (1 << 11) == 0 {
            gate_add(pc, 4) & !3
        } else {
            self.state.registers[SP]
        };
        self.state.registers[rd] = gate_add(base, immediate);
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
            let offset = gate_scale((raw & 0x7f) as u32, 4);
            if raw & 0x80 != 0 {
                self.state.registers[SP] = gate_subtract(self.state.registers[SP], offset);
                return Ok("sub-sp".into());
            }
            self.state.registers[SP] = gate_add(self.state.registers[SP], offset);
            return Ok("add-sp".into());
        }
        if raw & 0xfe00 == 0xb400 {
            let mut registers: Vec<usize> = (0..8).filter(|bit| raw & (1 << bit) != 0).collect();
            if raw & 0x100 != 0 {
                registers.push(LR);
            }
            for register in registers.into_iter().rev() {
                self.state.registers[SP] = gate_subtract(self.state.registers[SP], 4);
                self.write32(self.state.registers[SP], self.state.registers[register]);
            }
            return Ok("push".into());
        }
        if raw & 0xfe00 == 0xbc00 {
            for register in 0..8 {
                if raw & (1 << register) != 0 {
                    self.state.registers[register] = self.read32(self.state.registers[SP]);
                    self.state.registers[SP] = gate_add(self.state.registers[SP], 4);
                }
            }
            if raw & 0x100 != 0 {
                let target = self.read32(self.state.registers[SP]);
                self.state.registers[SP] = gate_add(self.state.registers[SP], 4);
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
                pc: gate_subtract(self.state.pc, 2),
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
                address = gate_add(address, 4);
            }
        }
        if !load || list & (1 << rn) == 0 {
            self.state.registers[rn] = address;
        }
        Ok(if load { "ldm" } else { "stm" }.into())
    }

    fn execute32(&mut self, first: u16, second: u16, pc: u32) -> Result<String, Armv7AError> {
        let prefix = first >> 11;
        if gate_equal(prefix as u32, 0b11110, 5)
            && gate_equal((second >> 14) as u32, 0b11, 2)
            && second & (1 << 12) != 0
        {
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
            self.set_pc(gate_add(self.state.pc, sign_extend(encoded, 25)));
            return Ok("bl".into());
        }
        if gate_equal(prefix as u32, 0b11110, 5) {
            return self.execute32_immediate(first, second, pc);
        }
        if gate_equal(prefix as u32, 0b11111, 5) {
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
            0x0 => (gate_boolean(left, immediate, and_gate), "and.w"),
            0x2 if rn == PC => (immediate, "mov.w"),
            0x2 => (gate_boolean(left, immediate, or_gate), "orr.w"),
            0x4 => (gate_boolean(left, immediate, xor_gate), "eor.w"),
            0x8 => (gate_add(left, immediate), "add.w"),
            0xa => (
                gate_add_carry(left, immediate, self.flag(CPSR_C)).0,
                "adc.w",
            ),
            0xd => (gate_subtract(left, immediate), "sub.w"),
            0xe => (gate_subtract(immediate, left), "rsb.w"),
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
        let address = gate_add(self.read_register_operand(rn, pc), (second & 0x0fff) as u32);
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
fn gate_equal(value: u32, expected: u32, width: usize) -> bool {
    (0..width).fold(1, |equal, bit| {
        and_gate(
            equal,
            xnor_gate(((value >> bit) & 1) as u8, ((expected >> bit) & 1) as u8),
        )
    }) != 0
}

fn gate_add_carry(left: u32, right: u32, carry: bool) -> (u32, bool, bool) {
    let left_bits = bits32(left);
    let right_bits = bits32(right);
    let output = ripple_carry_adder_with_carry(&left_bits, &right_bits, u8::from(carry));
    let sum: [u8; 32] = output.sum.try_into().expect("32-bit ripple result");
    let overflow = and_gate(
        not_gate(xor_gate(left_bits[31], right_bits[31])),
        xor_gate(left_bits[31], sum[31]),
    ) != 0;
    (word32(&sum), output.carry_out != 0, overflow)
}

fn gate_add(left: u32, right: u32) -> u32 {
    gate_add_carry(left, right, false).0
}

fn gate_subtract_borrow(left: u32, right: u32, borrow: bool) -> (u32, bool, bool) {
    let left_bits = bits32(left);
    let right_bits = bits32(right);
    let inverted: [u8; 32] = std::array::from_fn(|bit| not_gate(right_bits[bit]));
    let output = ripple_carry_adder_with_carry(&left_bits, &inverted, u8::from(!borrow));
    let difference: [u8; 32] = output.sum.try_into().expect("32-bit ripple result");
    let overflow = and_gate(
        xor_gate(left_bits[31], right_bits[31]),
        xor_gate(left_bits[31], difference[31]),
    ) != 0;
    (word32(&difference), output.carry_out != 0, overflow)
}

fn gate_subtract(left: u32, right: u32) -> u32 {
    gate_subtract_borrow(left, right, false).0
}

fn gate_boolean(left: u32, right: u32, gate: fn(u8, u8) -> u8) -> u32 {
    let left_bits = bits32(left);
    let right_bits = bits32(right);
    word32(&std::array::from_fn(|bit| {
        gate(left_bits[bit], right_bits[bit])
    }))
}

fn gate_not(value: u32) -> u32 {
    let input = bits32(value);
    word32(&std::array::from_fn(|bit| not_gate(input[bit])))
}

fn gate_shift_left_one(value: u32) -> u32 {
    let input = bits32(value);
    word32(&std::array::from_fn(|bit| {
        if bit == 0 {
            0
        } else {
            input[bit - 1]
        }
    }))
}

fn gate_scale(value: u32, scale: u32) -> u32 {
    match scale {
        1 => value,
        2 => gate_shift_left_one(value),
        4 => gate_shift_left_one(gate_shift_left_one(value)),
        _ => unreachable!("Thumb memory scale is one, two, or four"),
    }
}

fn gate_multiply(left: u32, right: u32) -> u32 {
    let right_bits = bits32(right);
    let mut product = 0;
    let mut multiplicand = left;
    for select in right_bits {
        let multiplicand_bits = bits32(multiplicand);
        let candidate = word32(&std::array::from_fn(|bit| {
            and_gate(multiplicand_bits[bit], select)
        }));
        product = gate_add(product, candidate);
        multiplicand = gate_shift_left_one(multiplicand);
    }
    product
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
        gate_barrel(unrotated, 3, (immediate12 >> 7) & 0x1f)
    }
}

fn gate_barrel(value: u32, kind: u8, amount: u32) -> u32 {
    let selects = bits32(amount);
    let mut current = bits32(value);
    for (stage, select) in selects[..5].iter().copied().enumerate() {
        let distance = 1usize << stage;
        let shifted: [u8; 32] = std::array::from_fn(|bit| match kind {
            0 => bit
                .checked_sub(distance)
                .map_or(0, |source| current[source]),
            1 => current.get(bit + distance).copied().unwrap_or(0),
            2 => current.get(bit + distance).copied().unwrap_or(current[31]),
            _ => current[(bit + distance) % 32],
        });
        current = std::array::from_fn(|bit| mux2(current[bit], shifted[bit], select));
    }
    word32(&current)
}

fn shift_immediate(value: u32, kind: u8, amount: u32, carry_in: bool) -> (u32, bool) {
    match kind {
        0 if amount == 0 => (value, carry_in),
        0 => (
            gate_barrel(value, 0, amount),
            bits32(value)[(32 - amount) as usize] != 0,
        ),
        1 => {
            let amount = if amount == 0 { 32 } else { amount };
            if amount == 32 {
                (0, bits32(value)[31] != 0)
            } else {
                (
                    gate_barrel(value, 1, amount),
                    bits32(value)[(amount - 1) as usize] != 0,
                )
            }
        }
        _ => {
            let amount = if amount == 0 { 32 } else { amount };
            if amount == 32 {
                (
                    if bits32(value)[31] == 0 { 0 } else { u32::MAX },
                    bits32(value)[31] != 0,
                )
            } else {
                (
                    gate_barrel(value, 2, amount),
                    bits32(value)[(amount - 1) as usize] != 0,
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
        0 if amount < 32 => (
            gate_barrel(value, 0, amount),
            bits32(value)[(32 - amount) as usize] != 0,
        ),
        0 if amount == 32 => (0, bits32(value)[0] != 0),
        0 => (0, false),
        1 if amount < 32 => (
            gate_barrel(value, 1, amount),
            bits32(value)[(amount - 1) as usize] != 0,
        ),
        1 if amount == 32 => (0, bits32(value)[31] != 0),
        1 => (0, false),
        2 if amount < 32 => (
            gate_barrel(value, 2, amount),
            bits32(value)[(amount - 1) as usize] != 0,
        ),
        2 => (
            if bits32(value)[31] == 0 { 0 } else { u32::MAX },
            bits32(value)[31] != 0,
        ),
        _ => {
            let rotation = amount & 31;
            if rotation == 0 {
                (value, bits32(value)[31] != 0)
            } else {
                let result = gate_barrel(value, 3, rotation);
                (result, bits32(result)[31] != 0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_arithmetic_boolean_shift_and_multiply_edges() {
        assert_eq!(gate_add_carry(u32::MAX, 1, false), (0, true, false));
        assert_eq!(
            gate_add_carry(i32::MAX as u32, 1, false),
            (0x8000_0000, false, true)
        );
        assert_eq!(gate_subtract_borrow(0, 1, false), (u32::MAX, false, false));
        assert_eq!(gate_boolean(0xf0f0, 0x0ff0, and_gate), 0x00f0);
        assert_eq!(gate_not(0), u32::MAX);
        assert_eq!(gate_barrel(1, 0, 31), 0x8000_0000);
        assert_eq!(gate_barrel(1, 3, 1), 0x8000_0000);
        assert_eq!(gate_multiply(0xffff_ffff, 3), 0xffff_fffd);
    }
}
