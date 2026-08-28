//! Gate-level DEC PDP-11 simulator for the complete Spec 07o subset.

use arithmetic::adders::{ripple_carry_adder, ripple_carry_adder_with_carry};
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use logic_gates::sequential::{register, FlipFlopState};
pub use pdp11_simulator::*;

const ZERO_8: [u8; 8] = [0; 8];
const ZERO_16: [u8; 16] = [0; 16];
const ONE_16: [u8; 16] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const TWO_16: [u8; 16] = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// Exact persistent D flip-flop count.
pub const FLIP_FLOP_COUNT: usize = MEMORY_BYTES * 8 + 8 * 16 + 16 + 1;

/// Stable educational gate topology estimate.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 80_000;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BitRegister<const WIDTH: usize> {
    state: [FlipFlopState; WIDTH],
}

impl<const WIDTH: usize> BitRegister<WIDTH> {
    fn new(initial: &[u8; WIDTH]) -> Self {
        let mut value = Self {
            state: std::array::from_fn(|_| FlipFlopState::default()),
        };
        value.write(initial);
        value
    }

    fn read(&self) -> [u8; WIDTH] {
        let mut state = self.state.clone();
        register(&[0; WIDTH], 0, &mut state)
            .try_into()
            .expect("a fixed-width register preserves width")
    }

    fn write(&mut self, data: &[u8; WIDTH]) {
        register(data, 0, &mut self.state);
        register(data, 1, &mut self.state);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EffectiveAddress {
    bits: [u8; 16],
    register: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SingleOperation {
    Swab,
    Clr(bool),
    Com(bool),
    Inc(bool),
    Dec(bool),
    Neg(bool),
    Adc(bool),
    Sbc(bool),
    Tst(bool),
    Ror(bool),
    Rol(bool),
    Asr(bool),
    Asl(bool),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DoubleOperation {
    Mov(bool),
    Cmp(bool),
    Bit(bool),
    Bic(bool),
    Bis(bool),
    Add,
    Sub,
}

/// PDP-11 whose complete persistent state and architectural datapaths are gates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pdp11GateLevel {
    r: [BitRegister<16>; 8],
    psw: BitRegister<16>,
    memory: Vec<BitRegister<8>>,
    halted: BitRegister<1>,
}

impl Default for Pdp11GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdp11GateLevel {
    /// Construct the documented reset state.
    pub fn new() -> Self {
        Self {
            r: std::array::from_fn(|index| {
                let initial = if index == SP {
                    INITIAL_SP
                } else if index == PC {
                    LOAD_ADDRESS
                } else {
                    0
                };
                BitRegister::new(&bits_from_u16::<16>(initial))
            }),
            psw: BitRegister::new(&ZERO_16),
            memory: (0..MEMORY_BYTES)
                .map(|_| BitRegister::new(&ZERO_8))
                .collect(),
            halted: BitRegister::new(&[0]),
        }
    }

    /// Reset every architectural element through flip-flop writes.
    pub fn reset(&mut self) {
        for (index, register) in self.r.iter_mut().enumerate() {
            let initial = if index == SP {
                INITIAL_SP
            } else if index == PC {
                LOAD_ADDRESS
            } else {
                0
            };
            register.write(&bits_from_u16::<16>(initial));
        }
        self.psw.write(&ZERO_16);
        for byte in &mut self.memory {
            byte.write(&ZERO_8);
        }
        self.halted.write(&[0]);
    }

    /// Validate and atomically load program bytes at 0x1000.
    pub fn load(&mut self, program: &[u8]) -> Result<usize, Pdp11Error> {
        let capacity = MEMORY_BYTES - usize::from(LOAD_ADDRESS);
        if program.len() > capacity {
            return Err(Pdp11Error::ProgramTooLarge {
                bytes: program.len(),
                capacity,
            });
        }
        let mut replacement = Self::new();
        for (offset, byte) in program.iter().copied().enumerate() {
            replacement.memory[usize::from(LOAD_ADDRESS) + offset]
                .write(&bits_from_u16::<8>(u16::from(byte)));
        }
        *self = replacement;
        Ok(program.len())
    }

    /// Return a complete immutable owned snapshot.
    pub fn state(&self) -> Pdp11State {
        Pdp11State {
            r: std::array::from_fn(|index| bits_to_u16(&self.r[index].read())),
            psw: bits_to_u16(&self.psw.read()),
            halted: self.halted.read()[0] == 1,
            memory: self
                .memory
                .iter()
                .map(|byte| bits_to_u16(&byte.read()) as u8)
                .collect(),
        }
    }

    /// Alias matching the SIM00 lifecycle vocabulary.
    pub fn get_state(&self) -> Pdp11State {
        self.state()
    }

    /// Read one boundary byte.
    pub fn read_byte(&self, address: u16) -> u8 {
        bits_to_u16(&self.memory[usize::from(address)].read()) as u8
    }

    /// Clock one boundary byte into memory.
    pub fn write_byte(&mut self, address: u16, value: u8) {
        self.memory[usize::from(address)].write(&bits_from_u16::<8>(u16::from(value)));
    }

    /// Read one aligned little-endian word.
    pub fn read_word(&self, address: u16) -> Result<u16, Pdp11Error> {
        Ok(bits_to_u16(&self.read_word_bits(address)?))
    }

    /// Clock one aligned little-endian boundary word into memory.
    pub fn write_word(&mut self, address: u16, value: u16) -> Result<(), Pdp11Error> {
        self.write_word_bits(address, &bits_from_u16::<16>(value))
    }

    /// Clock a boundary value into R0..R7.
    pub fn write_register(&mut self, index: usize, value: u16) -> Result<(), Pdp11Error> {
        let destination = self
            .r
            .get_mut(index)
            .ok_or(Pdp11Error::InvalidRegister { index })?;
        destination.write(&bits_from_u16::<16>(value));
        Ok(())
    }

    /// Clock a boundary value into the PSW.
    pub fn write_psw(&mut self, value: u16) {
        self.psw.write(&bits_from_u16::<16>(value));
    }

    /// Execute one transactional gate-level clock.
    pub fn step(&mut self) -> Result<StepTrace, Pdp11Error> {
        let pc_before = bits_to_u16(&self.r[PC].read());
        if self.halted.read()[0] == 1 {
            return Ok(StepTrace {
                pc_before,
                pc_after: pc_before,
                mnemonic: "HALT".to_owned(),
                description: "HALT (already halted)".to_owned(),
            });
        }
        let checkpoint = self.clone();
        match self.execute_one(pc_before) {
            Ok(mnemonic) => {
                let pc_after = bits_to_u16(&self.r[PC].read());
                Ok(StepTrace {
                    pc_before,
                    pc_after,
                    description: format!("{mnemonic} @ 0x{pc_before:04X}"),
                    mnemonic,
                })
            }
            Err(error) => {
                *self = checkpoint;
                Err(error)
            }
        }
    }

    /// Run until HALT under a mandatory caller bound.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Pdp11Error> {
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted.read()[0] == 1 {
                break;
            }
            traces.push(self.step()?);
        }
        if self.halted.read()[0] == 0 {
            return Err(Pdp11Error::MaxStepsExceeded { max_steps });
        }
        Ok(ExecutionResult {
            halted: true,
            steps: traces.len(),
            final_state: self.state(),
            traces,
        })
    }

    /// Reset, load, and run one program.
    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, Pdp11Error> {
        self.load(program)?;
        self.run(max_steps)
    }

    /// Exact persistent DFF count.
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    /// Stable educational gate estimate.
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }

    fn read_word_bits(&self, address: u16) -> Result<[u8; 16], Pdp11Error> {
        if address & 1 != 0 {
            return Err(Pdp11Error::OddWordAddress {
                address,
                write: false,
            });
        }
        let index = usize::from(address);
        let low = self.memory[index].read();
        let high = self.memory[index + 1].read();
        let mut result = ZERO_16;
        result[..8].copy_from_slice(&low);
        result[8..].copy_from_slice(&high);
        Ok(result)
    }

    fn write_word_bits(&mut self, address: u16, value: &[u8; 16]) -> Result<(), Pdp11Error> {
        if address & 1 != 0 {
            return Err(Pdp11Error::OddWordAddress {
                address,
                write: true,
            });
        }
        let index = usize::from(address);
        self.memory[index].write(&value[..8].try_into().expect("word has low byte"));
        self.memory[index + 1].write(&value[8..].try_into().expect("word has high byte"));
        Ok(())
    }

    fn fetch_word(&mut self) -> Result<[u8; 16], Pdp11Error> {
        let pc = self.r[PC].read();
        let word = self.read_word_bits(bits_to_u16(&pc))?;
        self.r[PC].write(&add_bits(&pc, &TWO_16));
        Ok(word)
    }

    fn effective_address(
        &mut self,
        mode: u8,
        register: usize,
        word: bool,
    ) -> Result<EffectiveAddress, Pdp11Error> {
        if mode == 0 {
            return Ok(EffectiveAddress {
                bits: bits_from_u16::<16>(register as u16),
                register: true,
            });
        }
        let step = if word || register >= SP {
            TWO_16
        } else {
            ONE_16
        };
        let register_bits = self.r[register].read();
        let address = match mode {
            1 => register_bits,
            2 => {
                self.r[register].write(&add_bits(&register_bits, &step));
                register_bits
            }
            3 => {
                self.r[register].write(&add_bits(&register_bits, &TWO_16));
                self.read_word_bits(bits_to_u16(&register_bits))?
            }
            4 => {
                let decremented = sub_bits(&register_bits, &step);
                self.r[register].write(&decremented);
                decremented
            }
            5 => {
                let decremented = sub_bits(&register_bits, &TWO_16);
                self.r[register].write(&decremented);
                self.read_word_bits(bits_to_u16(&decremented))?
            }
            6 => {
                let displacement = self.fetch_word()?;
                add_bits(&self.r[register].read(), &displacement)
            }
            7 => {
                let displacement = self.fetch_word()?;
                let pointer = add_bits(&self.r[register].read(), &displacement);
                self.read_word_bits(bits_to_u16(&pointer))?
            }
            _ => unreachable!("three mode bits are always in 0..=7"),
        };
        Ok(EffectiveAddress {
            bits: address,
            register: false,
        })
    }

    fn read_operand(&self, address: EffectiveAddress, word: bool) -> Result<[u8; 16], Pdp11Error> {
        if address.register {
            let value = self.r[usize::from(bits_to_u16(&address.bits))].read();
            Ok(if word { value } else { clear_high_byte(&value) })
        } else if word {
            self.read_word_bits(bits_to_u16(&address.bits))
        } else {
            let byte = self.memory[usize::from(bits_to_u16(&address.bits))].read();
            Ok(zero_extend_8(&byte))
        }
    }

    fn write_operand(
        &mut self,
        address: EffectiveAddress,
        value: &[u8; 16],
        word: bool,
    ) -> Result<(), Pdp11Error> {
        if address.register {
            let index = usize::from(bits_to_u16(&address.bits));
            if word {
                self.r[index].write(value);
            } else {
                let mut combined = self.r[index].read();
                combined[..8].copy_from_slice(&value[..8]);
                self.r[index].write(&combined);
            }
            Ok(())
        } else if word {
            self.write_word_bits(bits_to_u16(&address.bits), value)
        } else {
            let byte: [u8; 8] = value[..8].try_into().expect("word has low byte");
            self.memory[usize::from(bits_to_u16(&address.bits))].write(&byte);
            Ok(())
        }
    }

    fn set_nzvc(&mut self, n: u8, z: u8, v: u8, c: u8) {
        let mut psw = self.psw.read();
        psw[0] = c;
        psw[1] = v;
        psw[2] = z;
        psw[3] = n;
        self.psw.write(&psw);
    }

    fn flag(&self, bit: usize) -> u8 {
        self.psw.read()[bit]
    }

    fn execute_one(&mut self, pc_before: u16) -> Result<String, Pdp11Error> {
        let instruction = self.fetch_word()?;
        let instruction_value = bits_to_u16(&instruction);
        if equal_constant(&instruction, 0) == 1 {
            self.halted.write(&[1]);
            return Ok("HALT".to_owned());
        }
        if equal_constant(&instruction, 0x00a0) == 1 {
            return Ok("NOP".to_owned());
        }
        if equal_constant(&instruction, 0x0002) == 1 {
            let sp = self.r[SP].read();
            let new_pc = self.read_word_bits(bits_to_u16(&sp))?;
            let next_sp = add_bits(&sp, &TWO_16);
            let new_psw = self.read_word_bits(bits_to_u16(&next_sp))?;
            self.r[SP].write(&add_bits(&next_sp, &TWO_16));
            self.r[PC].write(&new_pc);
            self.psw.write(&new_psw);
            return Ok("RTI".to_owned());
        }
        if matches_constant(&instruction, 0x0080, 0xfff8) == 1 {
            let register = usize::from(bits_to_u16(&instruction[..3]));
            let old_pc = self.r[register].read();
            let sp = self.r[SP].read();
            let restored = self.read_word_bits(bits_to_u16(&sp))?;
            self.r[PC].write(&old_pc);
            self.r[register].write(&restored);
            self.r[SP].write(&add_bits(&sp, &TWO_16));
            return Ok("RTS".to_owned());
        }
        if matches_constant(&instruction, 0x7e00, 0xfe00) == 1 {
            let register = usize::from(bits_to_u16(&instruction[6..9]));
            let offset = zero_extend_slice(&instruction[..6]);
            let decremented = sub_bits(&self.r[register].read(), &ONE_16);
            self.r[register].write(&decremented);
            if zero_bit(&decremented) == 0 {
                let displacement = add_bits(&offset, &offset);
                self.r[PC].write(&sub_bits(&self.r[PC].read(), &displacement));
            }
            return Ok("SOB".to_owned());
        }

        if let Some((mnemonic, taken)) = self.branch(&instruction[8..]) {
            if taken == 1 {
                let offset = sign_extend_8(
                    &instruction[..8]
                        .try_into()
                        .expect("instruction has low byte"),
                );
                let displacement = add_bits(&offset, &offset);
                self.r[PC].write(&add_bits(&self.r[PC].read(), &displacement));
            }
            return Ok(mnemonic.to_owned());
        }

        if matches_constant(&instruction, 0x0040, 0xffc0) == 1 {
            let mode = bits_to_u16(&instruction[3..6]) as u8;
            if zero_bit(&instruction[3..6]) == 1 {
                return Err(Pdp11Error::IllegalAddressingMode {
                    instruction: "JMP",
                    mode,
                });
            }
            let register = usize::from(bits_to_u16(&instruction[..3]));
            let address = self.effective_address(mode, register, true)?;
            self.r[PC].write(&address.bits);
            return Ok("JMP".to_owned());
        }

        if matches_constant(&instruction, 0x0800, 0xfe00) == 1 {
            let link = usize::from(bits_to_u16(&instruction[6..9]));
            let mode = bits_to_u16(&instruction[3..6]) as u8;
            if zero_bit(&instruction[3..6]) == 1 {
                return Err(Pdp11Error::IllegalAddressingMode {
                    instruction: "JSR",
                    mode,
                });
            }
            let register = usize::from(bits_to_u16(&instruction[..3]));
            let destination = self.effective_address(mode, register, true)?;
            let old_link = self.r[link].read();
            let return_address = self.r[PC].read();
            let new_sp = sub_bits(&self.r[SP].read(), &TWO_16);
            self.write_word_bits(bits_to_u16(&new_sp), &old_link)?;
            self.r[SP].write(&new_sp);
            self.r[link].write(&return_address);
            self.r[PC].write(&destination.bits);
            return Ok("JSR".to_owned());
        }

        let mode = bits_to_u16(&instruction[3..6]) as u8;
        let register = usize::from(bits_to_u16(&instruction[..3]));
        if let Some(operation) = decode_single(&instruction) {
            return self.execute_single(operation, mode, register);
        }
        if let Some(operation) = decode_double(&instruction) {
            let source_mode = bits_to_u16(&instruction[9..12]) as u8;
            let source_register = usize::from(bits_to_u16(&instruction[6..9]));
            return self.execute_double(operation, source_mode, source_register, mode, register);
        }
        Err(Pdp11Error::UnknownOpcode {
            opcode: instruction_value,
            pc: pc_before,
        })
    }

    fn branch(&self, opcode: &[u8]) -> Option<(&'static str, u8)> {
        let n = self.flag(3);
        let z = self.flag(2);
        let v = self.flag(1);
        let c = self.flag(0);
        let nv = xor_gate(n, v);
        Some(if equal_slice_constant(opcode, 0x01) == 1 {
            ("BR", 1)
        } else if equal_slice_constant(opcode, 0x02) == 1 {
            ("BNE", not_gate(z))
        } else if equal_slice_constant(opcode, 0x03) == 1 {
            ("BEQ", z)
        } else if equal_slice_constant(opcode, 0x04) == 1 {
            ("BGE", not_gate(nv))
        } else if equal_slice_constant(opcode, 0x05) == 1 {
            ("BLT", nv)
        } else if equal_slice_constant(opcode, 0x06) == 1 {
            ("BGT", and_gate(not_gate(z), not_gate(nv)))
        } else if equal_slice_constant(opcode, 0x07) == 1 {
            ("BLE", or_gate(z, nv))
        } else if equal_slice_constant(opcode, 0x80) == 1 {
            ("BPL", not_gate(n))
        } else if equal_slice_constant(opcode, 0x81) == 1 {
            ("BMI", n)
        } else if equal_slice_constant(opcode, 0x82) == 1 {
            ("BHI", and_gate(not_gate(c), not_gate(z)))
        } else if equal_slice_constant(opcode, 0x83) == 1 {
            ("BLOS", or_gate(c, z))
        } else if equal_slice_constant(opcode, 0x84) == 1 {
            ("BVC", not_gate(v))
        } else if equal_slice_constant(opcode, 0x85) == 1 {
            ("BVS", v)
        } else if equal_slice_constant(opcode, 0x86) == 1 {
            ("BCC", not_gate(c))
        } else if equal_slice_constant(opcode, 0x87) == 1 {
            ("BCS", c)
        } else {
            return None;
        })
    }

    fn execute_single(
        &mut self,
        operation: SingleOperation,
        mode: u8,
        register: usize,
    ) -> Result<String, Pdp11Error> {
        if operation == SingleOperation::Swab {
            let address = self.effective_address(mode, register, true)?;
            let source = self.read_operand(address, true)?;
            let mut result = ZERO_16;
            result[..8].copy_from_slice(&source[8..]);
            result[8..].copy_from_slice(&source[..8]);
            self.write_operand(address, &result, true)?;
            self.set_nzvc(result[7], zero_bit(&result[..8]), 0, 0);
            return Ok("SWAB".to_owned());
        }
        let word = single_width(operation);
        let address = self.effective_address(mode, register, word)?;
        let source = self.read_operand(address, word)?;
        let old_c = self.flag(0);
        let (mnemonic, result, write, flags) = match operation {
            SingleOperation::Clr(_) => (sized_name("CLR", word), ZERO_16, true, [0, 1, 0, 0]),
            SingleOperation::Com(_) => {
                let result = sized_not(&source, word);
                (
                    sized_name("COM", word),
                    result,
                    true,
                    logic_flags(&result, word, 1),
                )
            }
            SingleOperation::Inc(_) => {
                let result = sized_add(&source, &ONE_16, word);
                let mut flags = add_flags(&source, &ONE_16, word);
                flags[3] = old_c;
                (sized_name("INC", word), result, true, flags)
            }
            SingleOperation::Dec(_) => {
                let result = sized_sub(&source, &ONE_16, word);
                let mut flags = sub_flags(&source, &ONE_16, word);
                flags[3] = old_c;
                (sized_name("DEC", word), result, true, flags)
            }
            SingleOperation::Neg(_) => {
                let result = sized_sub(&ZERO_16, &source, word);
                let msb = if word { 15 } else { 7 };
                let overflow = equal_sized_constant(&source, 1_u16 << msb, word);
                (
                    sized_name("NEG", word),
                    result,
                    true,
                    [
                        result[msb],
                        zero_sized(&result, word),
                        overflow,
                        not_gate(zero_sized(&result, word)),
                    ],
                )
            }
            SingleOperation::Adc(_) => {
                let carry = bool_vector(old_c);
                let result = sized_add(&source, &carry, word);
                (
                    sized_name("ADC", word),
                    result,
                    true,
                    add_flags(&source, &carry, word),
                )
            }
            SingleOperation::Sbc(_) => {
                let carry = bool_vector(old_c);
                let result = sized_sub(&source, &carry, word);
                (
                    sized_name("SBC", word),
                    result,
                    true,
                    sub_flags(&source, &carry, word),
                )
            }
            SingleOperation::Tst(_) => (
                sized_name("TST", word),
                source,
                false,
                logic_flags(&source, word, 0),
            ),
            SingleOperation::Asr(_) => {
                let result = shift_right_arithmetic(&source, word);
                let carry = source[0];
                let n = result[if word { 15 } else { 7 }];
                (
                    sized_name("ASR", word),
                    result,
                    true,
                    [n, zero_sized(&result, word), xor_gate(n, carry), carry],
                )
            }
            SingleOperation::Asl(_) => {
                let result = shift_left(&source, old_c, word, false);
                let carry = source[if word { 15 } else { 7 }];
                let n = result[if word { 15 } else { 7 }];
                (
                    sized_name("ASL", word),
                    result,
                    true,
                    [n, zero_sized(&result, word), xor_gate(n, carry), carry],
                )
            }
            SingleOperation::Ror(_) => {
                let result = rotate_right(&source, old_c, word);
                let carry = source[0];
                let n = result[if word { 15 } else { 7 }];
                (
                    sized_name("ROR", word),
                    result,
                    true,
                    [n, zero_sized(&result, word), xor_gate(n, carry), carry],
                )
            }
            SingleOperation::Rol(_) => {
                let result = shift_left(&source, old_c, word, true);
                let carry = source[if word { 15 } else { 7 }];
                let n = result[if word { 15 } else { 7 }];
                (
                    sized_name("ROL", word),
                    result,
                    true,
                    [n, zero_sized(&result, word), xor_gate(n, carry), carry],
                )
            }
            SingleOperation::Swab => unreachable!(),
        };
        if write {
            self.write_operand(address, &result, word)?;
        }
        self.set_nzvc(flags[0], flags[1], flags[2], flags[3]);
        Ok(mnemonic)
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_double(
        &mut self,
        operation: DoubleOperation,
        source_mode: u8,
        source_register: usize,
        destination_mode: u8,
        destination_register: usize,
    ) -> Result<String, Pdp11Error> {
        let word = double_width(operation);
        let source_address = self.effective_address(source_mode, source_register, word)?;
        let source = self.read_operand(source_address, word)?;
        let destination_address =
            self.effective_address(destination_mode, destination_register, word)?;
        let destination = self.read_operand(destination_address, word)?;
        let old_c = self.flag(0);

        let mnemonic = match operation {
            DoubleOperation::Mov(_) => {
                if !word && destination_address.register {
                    self.r[usize::from(bits_to_u16(&destination_address.bits))].write(
                        &sign_extend_8(&source[..8].try_into().expect("word has byte")),
                    );
                } else {
                    self.write_operand(destination_address, &source, word)?;
                }
                let flags = logic_flags(&source, word, old_c);
                self.set_nzvc(flags[0], flags[1], 0, old_c);
                sized_name("MOV", word)
            }
            DoubleOperation::Cmp(_) => {
                let flags = sub_flags(&source, &destination, word);
                self.set_nzvc(flags[0], flags[1], flags[2], flags[3]);
                sized_name("CMP", word)
            }
            DoubleOperation::Bit(_) => {
                let result = sized_binary(&source, &destination, word, and_gate);
                let flags = logic_flags(&result, word, old_c);
                self.set_nzvc(flags[0], flags[1], 0, old_c);
                sized_name("BIT", word)
            }
            DoubleOperation::Bic(_) => {
                let inverted = sized_not(&source, word);
                let result = sized_binary(&destination, &inverted, word, and_gate);
                self.write_operand(destination_address, &result, word)?;
                let flags = logic_flags(&result, word, old_c);
                self.set_nzvc(flags[0], flags[1], 0, old_c);
                sized_name("BIC", word)
            }
            DoubleOperation::Bis(_) => {
                let result = sized_binary(&destination, &source, word, or_gate);
                self.write_operand(destination_address, &result, word)?;
                let flags = logic_flags(&result, word, old_c);
                self.set_nzvc(flags[0], flags[1], 0, old_c);
                sized_name("BIS", word)
            }
            DoubleOperation::Add => {
                let result = add_bits(&destination, &source);
                let flags = add_flags(&destination, &source, true);
                self.write_operand(destination_address, &result, true)?;
                self.set_nzvc(flags[0], flags[1], flags[2], flags[3]);
                "ADD".to_owned()
            }
            DoubleOperation::Sub => {
                let result = sub_bits(&destination, &source);
                let flags = sub_flags(&destination, &source, true);
                self.write_operand(destination_address, &result, true)?;
                self.set_nzvc(flags[0], flags[1], flags[2], flags[3]);
                "SUB".to_owned()
            }
        };
        Ok(mnemonic)
    }
}

fn decode_single(instruction: &[u8; 16]) -> Option<SingleOperation> {
    Some(if matches_constant(instruction, 0x00c0, 0xffc0) == 1 {
        SingleOperation::Swab
    } else if matches_constant(instruction, 0x0a00, 0xffc0) == 1 {
        SingleOperation::Clr(true)
    } else if matches_constant(instruction, 0x8a00, 0xffc0) == 1 {
        SingleOperation::Clr(false)
    } else if matches_constant(instruction, 0x0a40, 0xffc0) == 1 {
        SingleOperation::Com(true)
    } else if matches_constant(instruction, 0x8a40, 0xffc0) == 1 {
        SingleOperation::Com(false)
    } else if matches_constant(instruction, 0x0a80, 0xffc0) == 1 {
        SingleOperation::Inc(true)
    } else if matches_constant(instruction, 0x8a80, 0xffc0) == 1 {
        SingleOperation::Inc(false)
    } else if matches_constant(instruction, 0x0ac0, 0xffc0) == 1 {
        SingleOperation::Dec(true)
    } else if matches_constant(instruction, 0x8ac0, 0xffc0) == 1 {
        SingleOperation::Dec(false)
    } else if matches_constant(instruction, 0x0b00, 0xffc0) == 1 {
        SingleOperation::Neg(true)
    } else if matches_constant(instruction, 0x8b00, 0xffc0) == 1 {
        SingleOperation::Neg(false)
    } else if matches_constant(instruction, 0x0b40, 0xffc0) == 1 {
        SingleOperation::Adc(true)
    } else if matches_constant(instruction, 0x8b40, 0xffc0) == 1 {
        SingleOperation::Adc(false)
    } else if matches_constant(instruction, 0x0b80, 0xffc0) == 1 {
        SingleOperation::Sbc(true)
    } else if matches_constant(instruction, 0x8b80, 0xffc0) == 1 {
        SingleOperation::Sbc(false)
    } else if matches_constant(instruction, 0x0bc0, 0xffc0) == 1 {
        SingleOperation::Tst(true)
    } else if matches_constant(instruction, 0x8bc0, 0xffc0) == 1 {
        SingleOperation::Tst(false)
    } else if matches_constant(instruction, 0x0c00, 0xffc0) == 1 {
        SingleOperation::Ror(true)
    } else if matches_constant(instruction, 0x8c00, 0xffc0) == 1 {
        SingleOperation::Ror(false)
    } else if matches_constant(instruction, 0x0c40, 0xffc0) == 1 {
        SingleOperation::Rol(true)
    } else if matches_constant(instruction, 0x8c40, 0xffc0) == 1 {
        SingleOperation::Rol(false)
    } else if matches_constant(instruction, 0x0c80, 0xffc0) == 1 {
        SingleOperation::Asr(true)
    } else if matches_constant(instruction, 0x8c80, 0xffc0) == 1 {
        SingleOperation::Asr(false)
    } else if matches_constant(instruction, 0x0cc0, 0xffc0) == 1 {
        SingleOperation::Asl(true)
    } else if matches_constant(instruction, 0x8cc0, 0xffc0) == 1 {
        SingleOperation::Asl(false)
    } else {
        return None;
    })
}

fn decode_double(instruction: &[u8; 16]) -> Option<DoubleOperation> {
    Some(if matches_constant(instruction, 0x1000, 0xf000) == 1 {
        DoubleOperation::Mov(true)
    } else if matches_constant(instruction, 0x9000, 0xf000) == 1 {
        DoubleOperation::Mov(false)
    } else if matches_constant(instruction, 0x2000, 0xf000) == 1 {
        DoubleOperation::Cmp(true)
    } else if matches_constant(instruction, 0xa000, 0xf000) == 1 {
        DoubleOperation::Cmp(false)
    } else if matches_constant(instruction, 0x3000, 0xf000) == 1 {
        DoubleOperation::Bit(true)
    } else if matches_constant(instruction, 0xb000, 0xf000) == 1 {
        DoubleOperation::Bit(false)
    } else if matches_constant(instruction, 0x4000, 0xf000) == 1 {
        DoubleOperation::Bic(true)
    } else if matches_constant(instruction, 0xc000, 0xf000) == 1 {
        DoubleOperation::Bic(false)
    } else if matches_constant(instruction, 0x5000, 0xf000) == 1 {
        DoubleOperation::Bis(true)
    } else if matches_constant(instruction, 0xd000, 0xf000) == 1 {
        DoubleOperation::Bis(false)
    } else if matches_constant(instruction, 0x6000, 0xf000) == 1 {
        DoubleOperation::Add
    } else if matches_constant(instruction, 0xe000, 0xf000) == 1 {
        DoubleOperation::Sub
    } else {
        return None;
    })
}

fn single_width(operation: SingleOperation) -> bool {
    match operation {
        SingleOperation::Swab => true,
        SingleOperation::Clr(word)
        | SingleOperation::Com(word)
        | SingleOperation::Inc(word)
        | SingleOperation::Dec(word)
        | SingleOperation::Neg(word)
        | SingleOperation::Adc(word)
        | SingleOperation::Sbc(word)
        | SingleOperation::Tst(word)
        | SingleOperation::Ror(word)
        | SingleOperation::Rol(word)
        | SingleOperation::Asr(word)
        | SingleOperation::Asl(word) => word,
    }
}

fn double_width(operation: DoubleOperation) -> bool {
    match operation {
        DoubleOperation::Mov(word)
        | DoubleOperation::Cmp(word)
        | DoubleOperation::Bit(word)
        | DoubleOperation::Bic(word)
        | DoubleOperation::Bis(word) => word,
        DoubleOperation::Add | DoubleOperation::Sub => true,
    }
}

fn equal_constant(bits: &[u8; 16], value: u16) -> u8 {
    matches_constant(bits, value, u16::MAX)
}

fn matches_constant(bits: &[u8; 16], value: u16, mask: u16) -> u8 {
    bits.iter().enumerate().fold(1, |equal, (bit, input)| {
        if mask & (1 << bit) == 0 {
            equal
        } else {
            let expected = ((value >> bit) & 1) as u8;
            and_gate(equal, not_gate(xor_gate(*input, expected)))
        }
    })
}

fn equal_slice_constant(bits: &[u8], value: u16) -> u8 {
    bits.iter().enumerate().fold(1, |equal, (bit, input)| {
        and_gate(
            equal,
            not_gate(xor_gate(*input, ((value >> bit) & 1) as u8)),
        )
    })
}

fn add_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder(a, b)
        .sum
        .try_into()
        .expect("ripple addition preserves width")
}

fn sub_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder_with_carry(a, &b.map(not_gate), 1)
        .sum
        .try_into()
        .expect("ripple subtraction preserves width")
}

fn clear_high_byte(value: &[u8; 16]) -> [u8; 16] {
    let mut result = *value;
    result[8..].fill(0);
    result
}

fn zero_extend_8(value: &[u8; 8]) -> [u8; 16] {
    let mut result = ZERO_16;
    result[..8].copy_from_slice(value);
    result
}

fn sign_extend_8(value: &[u8; 8]) -> [u8; 16] {
    let mut result = zero_extend_8(value);
    result[8..].fill(value[7]);
    result
}

fn zero_extend_slice(value: &[u8]) -> [u8; 16] {
    let mut result = ZERO_16;
    result[..value.len()].copy_from_slice(value);
    result
}

fn zero_bit(value: &[u8]) -> u8 {
    not_gate(value.iter().copied().fold(0, or_gate))
}

fn zero_sized(value: &[u8; 16], word: bool) -> u8 {
    zero_bit(if word { &value[..] } else { &value[..8] })
}

fn sized_add(a: &[u8; 16], b: &[u8; 16], word: bool) -> [u8; 16] {
    if word {
        add_bits(a, b)
    } else {
        let low = add_bits(
            &a[..8].try_into().expect("word has byte"),
            &b[..8].try_into().expect("word has byte"),
        );
        zero_extend_8(&low)
    }
}

fn sized_sub(a: &[u8; 16], b: &[u8; 16], word: bool) -> [u8; 16] {
    if word {
        sub_bits(a, b)
    } else {
        let low = sub_bits(
            &a[..8].try_into().expect("word has byte"),
            &b[..8].try_into().expect("word has byte"),
        );
        zero_extend_8(&low)
    }
}

fn sized_not(value: &[u8; 16], word: bool) -> [u8; 16] {
    let mut result = ZERO_16;
    let width = if word { 16 } else { 8 };
    for bit in 0..width {
        result[bit] = not_gate(value[bit]);
    }
    result
}

fn sized_binary(a: &[u8; 16], b: &[u8; 16], word: bool, gate: fn(u8, u8) -> u8) -> [u8; 16] {
    let mut result = ZERO_16;
    let width = if word { 16 } else { 8 };
    for bit in 0..width {
        result[bit] = gate(a[bit], b[bit]);
    }
    result
}

fn bool_vector(value: u8) -> [u8; 16] {
    let mut result = ZERO_16;
    result[0] = value;
    result
}

fn logic_flags(result: &[u8; 16], word: bool, carry: u8) -> [u8; 4] {
    [
        result[if word { 15 } else { 7 }],
        zero_sized(result, word),
        0,
        carry,
    ]
}

fn add_flags(a: &[u8; 16], b: &[u8; 16], word: bool) -> [u8; 4] {
    if word {
        let sum = ripple_carry_adder(a, b);
        let result: [u8; 16] = sum.sum.try_into().expect("adder preserves width");
        let overflow = and_gate(
            not_gate(xor_gate(a[15], b[15])),
            xor_gate(a[15], result[15]),
        );
        [result[15], zero_bit(&result), overflow, sum.carry_out]
    } else {
        let left: [u8; 8] = a[..8].try_into().expect("word has byte");
        let right: [u8; 8] = b[..8].try_into().expect("word has byte");
        let sum = ripple_carry_adder(&left, &right);
        let result: [u8; 8] = sum.sum.try_into().expect("adder preserves width");
        let overflow = and_gate(
            not_gate(xor_gate(left[7], right[7])),
            xor_gate(left[7], result[7]),
        );
        [result[7], zero_bit(&result), overflow, sum.carry_out]
    }
}

fn sub_flags(a: &[u8; 16], b: &[u8; 16], word: bool) -> [u8; 4] {
    let result = sized_sub(a, b, word);
    let msb = if word { 15 } else { 7 };
    let overflow = and_gate(xor_gate(a[msb], b[msb]), xor_gate(a[msb], result[msb]));
    let borrow = unsigned_less(
        if word { &a[..] } else { &a[..8] },
        if word { &b[..] } else { &b[..8] },
    );
    [result[msb], zero_sized(&result, word), overflow, borrow]
}

fn unsigned_less(a: &[u8], b: &[u8]) -> u8 {
    a.iter()
        .zip(b)
        .rev()
        .fold((0, 1), |(less, equal), (left, right)| {
            let here = and_gate(equal, and_gate(not_gate(*left), *right));
            (
                or_gate(less, here),
                and_gate(equal, not_gate(xor_gate(*left, *right))),
            )
        })
        .0
}

fn equal_sized_constant(value: &[u8; 16], expected: u16, word: bool) -> u8 {
    let width = if word { 16 } else { 8 };
    value[..width]
        .iter()
        .enumerate()
        .fold(1, |equal, (bit, input)| {
            and_gate(
                equal,
                not_gate(xor_gate(*input, ((expected >> bit) & 1) as u8)),
            )
        })
}

fn shift_right_arithmetic(value: &[u8; 16], word: bool) -> [u8; 16] {
    let width = if word { 16 } else { 8 };
    let mut result = ZERO_16;
    result[..width - 1].copy_from_slice(&value[1..width]);
    result[width - 1] = value[width - 1];
    result
}

fn shift_left(value: &[u8; 16], carry_in: u8, word: bool, rotate: bool) -> [u8; 16] {
    let width = if word { 16 } else { 8 };
    let mut result = ZERO_16;
    result[0] = and_gate(carry_in, u8::from(rotate));
    result[1..width].copy_from_slice(&value[..width - 1]);
    result
}

fn rotate_right(value: &[u8; 16], carry_in: u8, word: bool) -> [u8; 16] {
    let width = if word { 16 } else { 8 };
    let mut result = ZERO_16;
    result[..width - 1].copy_from_slice(&value[1..width]);
    result[width - 1] = carry_in;
    result
}

fn sized_name(name: &str, word: bool) -> String {
    if word {
        name.to_owned()
    } else {
        format!("{name}B")
    }
}

fn bits_from_u16<const WIDTH: usize>(value: u16) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_to_u16(bits: &[u8]) -> u16 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | u16::from(*input) << bit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_decode_and_arithmetic_primitives_are_exact() {
        let instruction = bits_from_u16::<16>(0x8cc0);
        assert_eq!(
            decode_single(&instruction),
            Some(SingleOperation::Asl(false))
        );
        assert_eq!(matches_constant(&instruction, 0x8cc0, 0xffc0), 1);
        assert_eq!(bits_to_u16(&add_bits(&[1; 16], &ONE_16)), 0);
        assert_eq!(bits_to_u16(&sub_bits(&ZERO_16, &ONE_16)), 0xffff);
        assert_eq!(unsigned_less(&ZERO_16, &ONE_16), 1);
    }
}
