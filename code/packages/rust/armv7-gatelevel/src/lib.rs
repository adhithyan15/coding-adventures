//! Exact gate-level partner for the Spec 07b ARMv7 educational machine.
//!
//! Persistent architectural bits are D-flip-flops. Condition predicates,
//! decode equality, Boolean operations, rotate-immediate, arithmetic, and
//! effective-address paths are built from the repository's logic gates and
//! ripple-carry adder rather than host arithmetic.

use arithmetic::adders::ripple_carry_adder_with_carry;
use arm_simulator::functional::{
    ArmError, ArmState, Armv7Simulator, ExecutionResult, Flags, StepTrace, HALT_WORD, MEMORY_SIZE,
};
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xnor_gate, xor_gate};

mod state;
use state::{bits32, clock_bit, clock_word, word32, DffMemory};

/// 524,288 memory bits + 512 R0–R15 bits + 4 NZCV bits + 1 halt bit.
pub const FLIP_FLOP_COUNT: usize = MEMORY_SIZE * 8 + 16 * 32 + 4 + 1;

#[derive(Debug, Clone)]
pub struct Armv7GateLevel {
    memory: DffMemory,
    registers: [[u8; 32]; 16],
    flags: [u8; 4],
    halted: u8,
    loaded_origin: u32,
    loaded_len: usize,
}

impl Default for Armv7GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Armv7GateLevel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: DffMemory::new(MEMORY_SIZE),
            registers: [[0; 32]; 16],
            flags: [0; 4],
            halted: 0,
            loaded_origin: 0,
            loaded_len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.memory.clear();
        self.registers = [[0; 32]; 16];
        self.flags = [0; 4];
        self.halted = 0;
        self.loaded_origin = 0;
        self.loaded_len = 0;
    }

    #[must_use]
    pub fn get_state(&self) -> ArmState {
        let registers = std::array::from_fn(|index| word32(&self.registers[index]));
        ArmState {
            registers,
            pc: registers[15],
            flags: Flags {
                n: self.flags[0] != 0,
                z: self.flags[1] != 0,
                c: self.flags[2] != 0,
                v: self.flags[3] != 0,
            },
            memory: self.memory.snapshot(),
            halted: self.halted != 0,
            loaded_origin: self.loaded_origin,
            loaded_len: self.loaded_len,
        }
    }

    pub fn restore(&mut self, state: &ArmState) -> Result<(), ArmError> {
        let mut validator = Armv7Simulator::new();
        validator.restore(state)?;
        self.restore_validated(state);
        Ok(())
    }

    fn restore_validated(&mut self, state: &ArmState) {
        self.memory.restore_snapshot(&state.memory);
        for (q, value) in self.registers.iter_mut().zip(state.registers) {
            *q = bits32(value);
        }
        self.flags = [
            u8::from(state.flags.n),
            u8::from(state.flags.z),
            u8::from(state.flags.c),
            u8::from(state.flags.v),
        ];
        self.halted = u8::from(state.halted);
        self.loaded_origin = state.loaded_origin;
        self.loaded_len = state.loaded_len;
    }

    pub fn load_checked(&mut self, program: &[u8]) -> Result<(), ArmError> {
        self.load_at_checked(program, 0)
    }

    pub fn load_at_checked(&mut self, program: &[u8], origin: u32) -> Result<(), ArmError> {
        if origin & 3 != 0 {
            return Err(ArmError::MisalignedProgram { origin });
        }
        let start = origin as usize;
        if start >= MEMORY_SIZE {
            return Err(ArmError::ProgramOutOfRange {
                origin,
                length: program.len(),
            });
        }
        let end = start
            .checked_add(program.len())
            .filter(|end| *end <= MEMORY_SIZE)
            .ok_or(ArmError::ProgramOutOfRange {
                origin,
                length: program.len(),
            })?;
        self.reset();
        for (address, byte) in (start..end).zip(program.iter().copied()) {
            self.memory.write(address, byte);
        }
        self.loaded_origin = origin;
        self.loaded_len = program.len();
        self.write_register_unchecked(15, origin);
        Ok(())
    }

    pub fn load(&mut self, program: &[u8]) -> Result<(), ArmError> {
        self.load_checked(program)
    }

    pub fn read_register_checked(&self, index: usize) -> Result<u32, ArmError> {
        self.registers
            .get(index)
            .map(word32)
            .ok_or(ArmError::InvalidRegister { index })
    }

    pub fn write_register_checked(&mut self, index: usize, value: u32) -> Result<(), ArmError> {
        if index >= 16 {
            return Err(ArmError::InvalidRegister { index });
        }
        if index == 15 {
            require_alignment(value, 4)?;
            direct_range(value, 4)?;
        }
        self.write_register_unchecked(index, value);
        Ok(())
    }

    pub fn read_byte_checked(&self, address: u32) -> Result<u8, ArmError> {
        Ok(self.memory.read(direct_range(address, 1)?))
    }

    pub fn write_byte_checked(&mut self, address: u32, value: u8) -> Result<(), ArmError> {
        let index = direct_range(address, 1)?;
        self.memory.write(index, value);
        Ok(())
    }

    pub fn read_word_checked(&self, address: u32) -> Result<u32, ArmError> {
        require_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        Ok(u32::from_le_bytes(std::array::from_fn(|offset| {
            self.memory.read(start + offset)
        })))
    }

    pub fn write_word_checked(&mut self, address: u32, value: u32) -> Result<(), ArmError> {
        require_alignment(address, 4)?;
        let start = direct_range(address, 4)?;
        for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
            self.memory.write(start + offset, byte);
        }
        Ok(())
    }

    pub fn step_checked(&mut self) -> Result<StepTrace, ArmError> {
        if self.halted != 0 {
            return Err(ArmError::Halted);
        }
        self.validate_fetch()?;
        let before = self.get_state();
        let pc = before.pc;
        let raw = self.fetch_word();
        match self.execute_raw(raw, pc) {
            Ok((mnemonic, condition_passed)) => Ok(StepTrace {
                pc_before: pc,
                pc_after: self.pc(),
                raw,
                mnemonic: mnemonic.to_string(),
                condition_passed,
                state_before: before,
                state_after: self.get_state(),
            }),
            Err(error) => {
                self.restore_validated(&before);
                Err(error)
            }
        }
    }

    pub fn step(&mut self) -> Result<StepTrace, ArmError> {
        self.step_checked()
    }

    pub fn run_loaded_checked(&mut self, max_steps: usize) -> Result<ExecutionResult, ArmError> {
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
                    self.restore_validated(&before);
                    return Err(error);
                }
            }
        }
        self.restore_validated(&before);
        Err(ArmError::StepLimitExceeded { max_steps })
    }

    pub fn run_checked(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, ArmError> {
        let before = self.get_state();
        self.load_checked(program)?;
        match self.run_loaded_checked(max_steps) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.restore_validated(&before);
                Err(error)
            }
        }
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, ArmError> {
        self.run_checked(program, max_steps)
    }

    fn pc(&self) -> u32 {
        word32(&self.registers[15])
    }

    fn write_register_unchecked(&mut self, index: usize, value: u32) {
        clock_word(&mut self.registers[index], value);
    }

    fn set_flags(&mut self, n: u8, z: u8, c: u8, v: u8) {
        for (q, value) in self.flags.iter_mut().zip([n, z, c, v]) {
            clock_bit(q, value != 0);
        }
    }

    fn validate_fetch(&self) -> Result<(), ArmError> {
        let pc = self.pc();
        require_alignment(pc, 4)?;
        let end = self.loaded_origin.saturating_add(self.loaded_len as u32);
        if pc < self.loaded_origin || pc.checked_add(4).is_none_or(|next| next > end) {
            return Err(ArmError::TruncatedInstruction { pc });
        }
        Ok(())
    }

    fn fetch_word(&self) -> u32 {
        let start = self.pc() as usize;
        u32::from_le_bytes(std::array::from_fn(|offset| {
            self.memory.read(start + offset)
        }))
    }

    fn execute_raw(&mut self, raw: u32, pc: u32) -> Result<(&'static str, bool), ArmError> {
        if gate_equal(raw, HALT_WORD, 32) != 0 {
            clock_bit(&mut self.halted, true);
            return Ok(("HLT", true));
        }
        let condition = (raw >> 28) & 0xf;
        if gate_condition(condition, &self.flags) == 0 {
            self.write_register_unchecked(15, sequential_pc(pc)?);
            return Ok(("SKIP", false));
        }
        let class = (raw >> 25) & 7;
        let mnemonic = if gate_equal(class, 0, 3) != 0 || gate_equal(class, 1, 3) != 0 {
            self.execute_data_processing(raw, pc)?
        } else if gate_equal(class, 2, 3) != 0 || gate_equal(class, 3, 3) != 0 {
            self.execute_transfer(raw, pc)?
        } else if gate_equal(class, 5, 3) != 0 {
            self.execute_branch(raw, pc)?
        } else {
            return Err(ArmError::UnknownInstruction { raw, pc });
        };
        Ok((mnemonic, true))
    }

    fn execute_data_processing(&mut self, raw: u32, pc: u32) -> Result<&'static str, ArmError> {
        let immediate = bit(raw, 25) != 0;
        let opcode = (raw >> 21) & 0xf;
        let set_flags = bit(raw, 20) != 0 || gate_equal(opcode, 0xa, 4) != 0;
        let rn = ((raw >> 16) & 0xf) as usize;
        let rd = ((raw >> 12) & 0xf) as usize;
        let a = self.registers[rn];
        let (operand, shifter_carry) = self.decode_operand2(raw, pc, immediate)?;
        let old_v = self.flags[3];
        let (mnemonic, result, carry, overflow, write_result) = if gate_equal(opcode, 0x0, 4) != 0 {
            (
                "AND",
                gate_boolean(&a, &operand, and_gate),
                shifter_carry,
                old_v,
                true,
            )
        } else if gate_equal(opcode, 0x2, 4) != 0 {
            let (result, carry, overflow) = gate_subtract(&a, &operand);
            ("SUB", result, carry, overflow, true)
        } else if gate_equal(opcode, 0x4, 4) != 0 {
            let (result, carry, overflow) = gate_add(&a, &operand);
            ("ADD", result, carry, overflow, true)
        } else if gate_equal(opcode, 0xa, 4) != 0 {
            let (result, carry, overflow) = gate_subtract(&a, &operand);
            ("CMP", result, carry, overflow, false)
        } else if gate_equal(opcode, 0xc, 4) != 0 {
            (
                "ORR",
                gate_boolean(&a, &operand, or_gate),
                shifter_carry,
                old_v,
                true,
            )
        } else if gate_equal(opcode, 0xd, 4) != 0 {
            ("MOV", operand, shifter_carry, old_v, true)
        } else {
            return Err(ArmError::UnknownInstruction { raw, pc });
        };

        let value = word32(&result);
        let mut next = sequential_pc(pc)?;
        if write_result {
            if rd == 15 {
                require_alignment(value, 4)?;
                direct_range(value, 4)?;
                next = value;
            } else {
                self.write_register_unchecked(rd, value);
            }
        }
        if set_flags {
            self.set_flags(result[31], gate_zero(&result), carry, overflow);
        }
        self.write_register_unchecked(15, next);
        Ok(mnemonic)
    }

    fn decode_operand2(
        &self,
        raw: u32,
        pc: u32,
        immediate: bool,
    ) -> Result<([u8; 32], u8), ArmError> {
        let operand2 = raw & 0xfff;
        if immediate {
            let rotation = (operand2 >> 8) & 0xf;
            let result = gate_rotate_right(&bits32(operand2 & 0xff), rotation);
            let carry = mux2(self.flags[2], result[31], gate_nonzero(rotation, 4));
            Ok((result, carry))
        } else {
            if gate_nonzero(operand2 & 0xff0, 12) != 0 {
                return Err(ArmError::UnsupportedOperand2 { raw, pc });
            }
            Ok((self.registers[(operand2 & 0xf) as usize], self.flags[2]))
        }
    }

    fn execute_transfer(&mut self, raw: u32, pc: u32) -> Result<&'static str, ArmError> {
        let valid = and_gate(
            and_gate(not_gate(bit(raw, 25)), bit(raw, 24)),
            and_gate(not_gate(bit(raw, 22)), not_gate(bit(raw, 21))),
        );
        if valid == 0 {
            return Err(ArmError::UnsupportedAddressingMode { raw, pc });
        }
        let rn = ((raw >> 16) & 0xf) as usize;
        let rd = ((raw >> 12) & 0xf) as usize;
        let offset = bits32(raw & 0xfff);
        let base = self.registers[rn];
        let address = if bit(raw, 23) != 0 {
            word32(&gate_add(&base, &offset).0)
        } else {
            word32(&gate_subtract(&base, &offset).0)
        };
        if bit(raw, 20) != 0 {
            let value = self.read_word_checked(address)?;
            if rd == 15 {
                require_alignment(value, 4)?;
                direct_range(value, 4)?;
                self.write_register_unchecked(15, value);
            } else {
                self.write_register_unchecked(rd, value);
                self.write_register_unchecked(15, sequential_pc(pc)?);
            }
            Ok("LDR")
        } else {
            self.write_word_checked(address, word32(&self.registers[rd]))?;
            self.write_register_unchecked(15, sequential_pc(pc)?);
            Ok("STR")
        }
    }

    fn execute_branch(&mut self, raw: u32, pc: u32) -> Result<&'static str, ArmError> {
        let offset = (((raw & 0x00ff_ffff) << 8) as i32 >> 6) as u32;
        let pc_plus_eight = word32(&gate_add(&bits32(pc), &bits32(8)).0);
        let target = word32(&gate_add(&bits32(pc_plus_eight), &bits32(offset)).0);
        require_alignment(target, 4)?;
        direct_range(target, 4)?;
        if bit(raw, 24) != 0 {
            let link = word32(&gate_add(&bits32(pc), &bits32(4)).0);
            self.write_register_unchecked(14, link);
        }
        self.write_register_unchecked(15, target);
        Ok(if bit(raw, 24) != 0 { "BL" } else { "B" })
    }
}

fn bit(value: u32, index: usize) -> u8 {
    ((value >> index) & 1) as u8
}

fn gate_equal(value: u32, expected: u32, width: usize) -> u8 {
    (0..width).fold(1, |equal, index| {
        and_gate(equal, xnor_gate(bit(value, index), bit(expected, index)))
    })
}

fn gate_nonzero(value: u32, width: usize) -> u8 {
    (0..width).fold(0, |any, index| or_gate(any, bit(value, index)))
}

fn gate_condition(condition: u32, flags: &[u8; 4]) -> u8 {
    let [n, z, c, v] = *flags;
    let predicate = [
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
    predicate[condition as usize]
}

fn gate_boolean(a: &[u8; 32], b: &[u8; 32], gate: fn(u8, u8) -> u8) -> [u8; 32] {
    std::array::from_fn(|index| gate(a[index], b[index]))
}

fn gate_add(a: &[u8; 32], b: &[u8; 32]) -> ([u8; 32], u8, u8) {
    let result = ripple_carry_adder_with_carry(a, b, 0);
    let sum: [u8; 32] = result.sum.try_into().expect("32-bit adder result");
    let overflow = and_gate(not_gate(xor_gate(a[31], b[31])), xor_gate(a[31], sum[31]));
    (sum, result.carry_out, overflow)
}

fn gate_subtract(a: &[u8; 32], b: &[u8; 32]) -> ([u8; 32], u8, u8) {
    let inverted: [u8; 32] = std::array::from_fn(|index| not_gate(b[index]));
    let result = ripple_carry_adder_with_carry(a, &inverted, 1);
    let difference: [u8; 32] = result.sum.try_into().expect("32-bit subtractor result");
    let overflow = and_gate(xor_gate(a[31], b[31]), xor_gate(a[31], difference[31]));
    (difference, result.carry_out, overflow)
}

fn gate_zero(bits: &[u8; 32]) -> u8 {
    not_gate(bits.iter().copied().fold(0, or_gate))
}

fn gate_rotate_right(value: &[u8; 32], encoded_rotation: u32) -> [u8; 32] {
    let mut current = *value;
    for stage in 0..4 {
        let shift = 2usize << stage;
        let select = bit(encoded_rotation, stage);
        current = std::array::from_fn(|index| {
            mux2(current[index], current[(index + shift) % 32], select)
        });
    }
    current
}

fn require_alignment(address: u32, width: usize) -> Result<(), ArmError> {
    if !address.is_multiple_of(width as u32) {
        return Err(ArmError::MisalignedAccess { address, width });
    }
    Ok(())
}

fn direct_range(address: u32, width: usize) -> Result<usize, ArmError> {
    let start = address as usize;
    start
        .checked_add(width)
        .filter(|end| *end <= MEMORY_SIZE)
        .map(|_| start)
        .ok_or(ArmError::MemoryOutOfRange { address, width })
}

fn sequential_pc(pc: u32) -> Result<u32, ArmError> {
    let next = word32(&gate_add(&bits32(pc), &bits32(4)).0);
    direct_range(next, 4)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arm_simulator::functional::{
        assemble_words, encode_data_immediate, encode_data_register, encode_halt, Condition,
        DataOpcode,
    };

    #[test]
    fn exact_topology_and_gate_primitives() {
        assert_eq!(FLIP_FLOP_COUNT, 524_805);
        let (sum, carry, overflow) = gate_add(&bits32(u32::MAX), &bits32(1));
        assert_eq!(word32(&sum), 0);
        assert_eq!(carry, 1);
        assert_eq!(overflow, 0);
        assert_eq!(word32(&gate_rotate_right(&bits32(1), 1)), 0x4000_0000);
    }

    #[test]
    fn functional_program_executes_through_gate_path() {
        let words = [
            encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 7),
            encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 1, 0, 5),
            encode_data_register(Condition::Al, DataOpcode::Add, true, 0, 2, 1),
            encode_halt(),
        ];
        let result = Armv7GateLevel::new()
            .run_checked(&assemble_words(&words), 8)
            .unwrap();
        assert!(result.halted);
        assert_eq!(result.final_state.registers[2], 12);
    }
}
