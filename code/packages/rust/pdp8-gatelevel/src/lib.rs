//! Educational gate-level DEC PDP-8/E base-processor simulator.
//!
//! Persistent architectural bits are clocked D flip-flops. Architectural
//! arithmetic, Boolean logic, predicates, decode, page selection, and data
//! movement flow through repository digital-logic primitives.

use arithmetic::adders::ripple_carry_adder;
use logic_gates::combinational::mux2;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use logic_gates::sequential::{register, FlipFlopState};

pub use pdp8_simulator::{
    encode_iot, encode_memory_reference, ExecutionResult, IotEvent, Opcode, Pdp8Error,
    Pdp8Simulator, Pdp8State, StepTrace, HLT, MEMORY_WORDS, NOP, WORD_MASK,
};

const WIDTH: usize = 12;
const AUTO_INDEX_START: u16 = 0o0010;
const AUTO_INDEX_END: u16 = 0o0017;
const ZERO_12: [u8; WIDTH] = [0; WIDTH];
const ONE_12: [u8; WIDTH] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// Exact count of simulated persistent state bits.
pub const FLIP_FLOP_COUNT: usize = MEMORY_WORDS * WIDTH + 4 * WIDTH + 5;

/// Stable educational storage-plus-combinational gate estimate.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 24_000;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BitRegister<const BITS: usize> {
    state: [FlipFlopState; BITS],
}

impl<const BITS: usize> BitRegister<BITS> {
    fn new(initial: &[u8; BITS]) -> Self {
        let mut value = Self {
            state: std::array::from_fn(|_| FlipFlopState::default()),
        };
        value.write(initial);
        value
    }

    fn read(&self) -> [u8; BITS] {
        let mut state = self.state.clone();
        register(&[0; BITS], 0, &mut state)
            .try_into()
            .expect("a fixed-width register preserves its width")
    }

    fn write(&mut self, data: &[u8; BITS]) {
        register(data, 0, &mut self.state);
        register(data, 1, &mut self.state);
    }
}

/// PDP-8/E state and datapaths expressed through digital-logic primitives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pdp8GateLevel {
    memory: Vec<BitRegister<WIDTH>>,
    ac: BitRegister<WIDTH>,
    pc: BitRegister<WIDTH>,
    mq: BitRegister<WIDTH>,
    switch_register: BitRegister<WIDTH>,
    link: BitRegister<1>,
    interrupt_enable: BitRegister<1>,
    interrupt_pending: BitRegister<1>,
    interrupt_delay: BitRegister<1>,
    halted: BitRegister<1>,
    loaded_origin: u16,
    loaded_words: usize,
}

impl Default for Pdp8GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdp8GateLevel {
    /// Construct a clear processor and zeroed 4K-word core.
    pub fn new() -> Self {
        Self {
            memory: (0..MEMORY_WORDS)
                .map(|_| BitRegister::new(&ZERO_12))
                .collect(),
            ac: BitRegister::new(&ZERO_12),
            pc: BitRegister::new(&ZERO_12),
            mq: BitRegister::new(&ZERO_12),
            switch_register: BitRegister::new(&ZERO_12),
            link: BitRegister::new(&[0]),
            interrupt_enable: BitRegister::new(&[0]),
            interrupt_pending: BitRegister::new(&[0]),
            interrupt_delay: BitRegister::new(&[0]),
            halted: BitRegister::new(&[0]),
            loaded_origin: 0,
            loaded_words: 0,
        }
    }

    /// Clock zero through every architectural state element.
    pub fn reset(&mut self) {
        for word in &mut self.memory {
            word.write(&ZERO_12);
        }
        self.ac.write(&ZERO_12);
        self.pc.write(&ZERO_12);
        self.mq.write(&ZERO_12);
        self.switch_register.write(&ZERO_12);
        self.link.write(&[0]);
        self.interrupt_enable.write(&[0]);
        self.interrupt_pending.write(&[0]);
        self.interrupt_delay.write(&[0]);
        self.halted.write(&[0]);
        self.loaded_origin = 0;
        self.loaded_words = 0;
    }

    /// Load strict little-endian containers carrying 12-bit words.
    pub fn load(&mut self, program: &[u8], origin: usize) -> Result<usize, Pdp8Error> {
        if !program.len().is_multiple_of(2) {
            return Err(Pdp8Error::OddProgramLength {
                bytes: program.len(),
            });
        }
        validate_program_extent(program.len() / 2, origin)?;
        let words = program
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>();
        self.load_words(&words, origin)
    }

    /// Replace the processor with a clean state and clock in program words.
    pub fn load_words(&mut self, program: &[u16], origin: usize) -> Result<usize, Pdp8Error> {
        validate_program(program, origin)?;
        self.reset();
        for (offset, word) in program.iter().copied().enumerate() {
            self.memory[origin + offset].write(&u16_to_bits(word));
        }
        self.pc.write(&u16_to_bits(origin as u16));
        self.loaded_origin = origin as u16;
        self.loaded_words = program.len();
        Ok(program.len())
    }

    /// Restore a complete snapshot after functional-contract validation.
    pub fn restore(&mut self, state: &Pdp8State) -> Result<(), Pdp8Error> {
        let mut validator = Pdp8Simulator::new();
        validator.restore(state)?;
        for (destination, word) in self.memory.iter_mut().zip(state.memory.iter().copied()) {
            destination.write(&u16_to_bits(word));
        }
        self.ac.write(&u16_to_bits(state.ac));
        self.pc.write(&u16_to_bits(state.pc));
        self.mq.write(&u16_to_bits(state.mq));
        self.switch_register
            .write(&u16_to_bits(state.switch_register));
        self.link.write(&[u8::from(state.link)]);
        self.interrupt_enable
            .write(&[u8::from(state.interrupt_enable)]);
        self.interrupt_pending
            .write(&[u8::from(state.interrupt_pending)]);
        self.interrupt_delay.write(&[state.interrupt_delay]);
        self.halted.write(&[u8::from(state.halted)]);
        self.loaded_origin = state.loaded_origin;
        self.loaded_words = state.loaded_words;
        Ok(())
    }

    /// Read every D flip-flop into an owned architectural snapshot.
    pub fn state(&self) -> Pdp8State {
        let memory: Box<[u16; MEMORY_WORDS]> = self
            .memory
            .iter()
            .map(|word| bits_to_u16(&word.read()))
            .collect::<Vec<_>>()
            .into_boxed_slice()
            .try_into()
            .expect("the gate memory always has exactly 4096 words");
        Pdp8State {
            memory,
            ac: bits_to_u16(&self.ac.read()),
            link: self.link.read()[0] == 1,
            pc: bits_to_u16(&self.pc.read()),
            mq: bits_to_u16(&self.mq.read()),
            switch_register: bits_to_u16(&self.switch_register.read()),
            interrupt_enable: self.interrupt_enable.read()[0] == 1,
            interrupt_pending: self.interrupt_pending.read()[0] == 1,
            interrupt_delay: self.interrupt_delay.read()[0],
            halted: self.halted.read()[0] == 1,
            loaded_origin: self.loaded_origin,
            loaded_words: self.loaded_words,
        }
    }

    pub fn get_state(&self) -> Pdp8State {
        self.state()
    }

    pub fn read_memory(&self, address: usize) -> Result<u16, Pdp8Error> {
        self.memory
            .get(address)
            .map(|word| bits_to_u16(&word.read()))
            .ok_or(Pdp8Error::InvalidAddress { address })
    }

    pub fn write_memory(&mut self, address: usize, word: u16) -> Result<(), Pdp8Error> {
        validate_word(word, address)?;
        self.memory
            .get_mut(address)
            .ok_or(Pdp8Error::InvalidAddress { address })?
            .write(&u16_to_bits(word));
        Ok(())
    }

    pub fn set_accumulator(&mut self, value: u16) -> Result<(), Pdp8Error> {
        validate_word(value, 0)?;
        self.ac.write(&u16_to_bits(value));
        Ok(())
    }

    pub fn set_link(&mut self, value: bool) {
        self.link.write(&[u8::from(value)]);
    }

    pub fn set_program_counter(&mut self, value: u16) -> Result<(), Pdp8Error> {
        validate_word(value, 0)?;
        self.pc.write(&u16_to_bits(value));
        Ok(())
    }

    pub fn set_multiplier_quotient(&mut self, value: u16) -> Result<(), Pdp8Error> {
        validate_word(value, 0)?;
        self.mq.write(&u16_to_bits(value));
        Ok(())
    }

    pub fn set_switch_register(&mut self, value: u16) -> Result<(), Pdp8Error> {
        validate_word(value, 0)?;
        self.switch_register.write(&u16_to_bits(value));
        Ok(())
    }

    pub fn set_interrupt_request(&mut self, pending: bool) {
        self.interrupt_pending.write(&[u8::from(pending)]);
    }

    /// Execute one fail-closed gate clock cycle.
    pub fn step(&mut self) -> Result<StepTrace, Pdp8Error> {
        if self.halted.read()[0] == 1 {
            return Err(Pdp8Error::Halted);
        }
        let interrupt_selected = and_gate(
            and_gate(
                self.interrupt_enable.read()[0],
                self.interrupt_pending.read()[0],
            ),
            not_gate(self.interrupt_delay.read()[0]),
        );
        if interrupt_selected == 1 {
            return Ok(self.enter_interrupt());
        }

        let pc_before_bits = self.pc.read();
        let pc_before = bits_to_u16(&pc_before_bits);
        let instruction_bits = self.memory[usize::from(pc_before)].read();
        let instruction = bits_to_u16(&instruction_bits);
        validate_instruction(&instruction_bits, instruction)?;
        let delay_before = self.interrupt_delay.read()[0];

        self.pc.write(&add_bits(&pc_before_bits, &ONE_12).0);
        let selects = decode_three([
            instruction_bits[9],
            instruction_bits[10],
            instruction_bits[11],
        ]);
        let (mnemonic, effective_address, iot) = if selects[..6].contains(&1) {
            let (address_bits, address) = self.resolve_effective_address(&instruction_bits);
            let mnemonic = self.execute_memory_reference(selects, address_bits, address);
            (mnemonic, Some(address), None)
        } else if selects[6] == 1 {
            let (mnemonic, event) = self.execute_iot(instruction);
            (mnemonic, None, event)
        } else {
            (self.execute_operate(&instruction_bits), None, None)
        };

        let current_is_ion = selects[6] == 1 && instruction == encode_iot(0, 1);
        if delay_before == 1 && !current_is_ion {
            self.interrupt_delay.write(&[0]);
        }
        let pc_after = bits_to_u16(&self.pc.read());
        Ok(StepTrace {
            pc_before,
            pc_after,
            instruction: Some(instruction),
            effective_address,
            description: format!("{mnemonic} @ {pc_before:04o}"),
            mnemonic,
            iot,
        })
    }

    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Pdp8Error> {
        let mut traces = Vec::new();
        while self.halted.read()[0] == 0 {
            if traces.len() == max_steps {
                return Err(Pdp8Error::MaxStepsExceeded { max_steps });
            }
            traces.push(self.step()?);
        }
        Ok(ExecutionResult {
            halted: true,
            steps: traces.len(),
            final_state: self.state(),
            traces,
        })
    }

    pub fn execute(
        &mut self,
        program: &[u8],
        origin: usize,
        max_steps: usize,
    ) -> Result<ExecutionResult, Pdp8Error> {
        self.load(program, origin)?;
        self.run(max_steps)
    }

    /// Exact persistent D-flip-flop count.
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    /// Stable educational gate-equivalent estimate.
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }

    fn enter_interrupt(&mut self) -> StepTrace {
        let pc_bits = self.pc.read();
        let pc_before = bits_to_u16(&pc_bits);
        self.memory[0].write(&pc_bits);
        self.pc.write(&ONE_12);
        self.interrupt_enable.write(&[0]);
        self.interrupt_pending.write(&[0]);
        StepTrace {
            pc_before,
            pc_after: 1,
            instruction: None,
            effective_address: Some(0),
            mnemonic: "INTERRUPT".to_owned(),
            description: format!("INTERRUPT: M[0000] <- {pc_before:04o}; PC <- 0001"),
            iot: None,
        }
    }

    fn resolve_effective_address(&mut self, instruction: &[u8; WIDTH]) -> ([u8; WIDTH], u16) {
        let pc = self.pc.read();
        let mut address = std::array::from_fn(|bit| {
            if bit < 7 {
                instruction[bit]
            } else {
                mux2(0, pc[bit], instruction[7])
            }
        });
        if instruction[8] == 1 {
            let pointer_address = bits_to_u16(&address);
            if (AUTO_INDEX_START..=AUTO_INDEX_END).contains(&pointer_address) {
                let incremented =
                    add_bits(&self.memory[usize::from(pointer_address)].read(), &ONE_12).0;
                self.memory[usize::from(pointer_address)].write(&incremented);
            }
            address = self.memory[usize::from(pointer_address)].read();
        }
        let numeric = bits_to_u16(&address);
        (address, numeric)
    }

    fn execute_memory_reference(
        &mut self,
        selects: [u8; 8],
        address_bits: [u8; WIDTH],
        address: u16,
    ) -> String {
        let index = usize::from(address);
        let operand = self.memory[index].read();
        let mnemonic = if selects[0] == 1 {
            self.ac.write(&and_bits(&self.ac.read(), &operand));
            "AND"
        } else if selects[1] == 1 {
            let (sum, carry) = add_bits(&self.ac.read(), &operand);
            self.ac.write(&sum);
            self.link.write(&[xor_gate(self.link.read()[0], carry)]);
            "TAD"
        } else if selects[2] == 1 {
            let (incremented, _) = add_bits(&operand, &ONE_12);
            self.memory[index].write(&incremented);
            if is_zero(&incremented) == 1 {
                self.increment_pc();
            }
            "ISZ"
        } else if selects[3] == 1 {
            self.memory[index].write(&self.ac.read());
            self.ac.write(&ZERO_12);
            "DCA"
        } else if selects[4] == 1 {
            self.memory[index].write(&self.pc.read());
            self.pc.write(&add_bits(&address_bits, &ONE_12).0);
            "JMS"
        } else {
            debug_assert_eq!(selects[5], 1);
            self.pc.write(&address_bits);
            "JMP"
        };
        format!("{mnemonic} {address:04o}")
    }

    fn execute_iot(&mut self, instruction: u16) -> (String, Option<IotEvent>) {
        let device = ((instruction >> 3) & 0o77) as u8;
        let pulses = (instruction & 0o7) as u8;
        if device != 0 {
            return (
                format!("IOT {device:02o},{pulses:o}"),
                Some(IotEvent { device, pulses }),
            );
        }
        let mnemonic = match pulses {
            0 => {
                if self.interrupt_enable.read()[0] == 1 {
                    self.increment_pc();
                }
                self.interrupt_enable.write(&[0]);
                self.interrupt_delay.write(&[0]);
                "SKON"
            }
            1 => {
                self.interrupt_enable.write(&[1]);
                // One bit represents the one complete post-ION instruction.
                self.interrupt_delay.write(&[1]);
                "ION"
            }
            2 => {
                self.interrupt_enable.write(&[0]);
                self.interrupt_delay.write(&[0]);
                "IOF"
            }
            3 => {
                if self.interrupt_pending.read()[0] == 1 {
                    self.increment_pc();
                }
                "SRQ"
            }
            7 => {
                self.interrupt_enable.write(&[0]);
                self.interrupt_pending.write(&[0]);
                self.interrupt_delay.write(&[0]);
                "CAF"
            }
            _ => unreachable!("unsupported CPU IOTs are rejected before clocking"),
        };
        (mnemonic.to_owned(), None)
    }

    fn execute_operate(&mut self, instruction: &[u8; WIDTH]) -> String {
        if instruction[8] == 0 {
            self.execute_group_one(instruction)
        } else {
            self.execute_group_two(instruction)
        }
    }

    fn execute_group_one(&mut self, instruction: &[u8; WIDTH]) -> String {
        let mut operations = Vec::new();
        if instruction[7] == 1 {
            self.ac.write(&ZERO_12);
            operations.push("CLA");
        }
        if instruction[6] == 1 {
            self.link.write(&[0]);
            operations.push("CLL");
        }
        if instruction[5] == 1 {
            self.ac.write(&self.ac.read().map(not_gate));
            operations.push("CMA");
        }
        if instruction[4] == 1 {
            self.link.write(&[not_gate(self.link.read()[0])]);
            operations.push("CML");
        }
        if instruction[0] == 1 {
            let mut combined = [0; 13];
            combined[..12].copy_from_slice(&self.ac.read());
            combined[12] = self.link.read()[0];
            let one = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            let incremented: [u8; 13] = ripple_carry_adder(&combined, &one)
                .sum
                .try_into()
                .expect("a 13-bit adder preserves width");
            self.ac.write(&incremented[..12].try_into().unwrap());
            self.link.write(&[incremented[12]]);
            operations.push("IAC");
        }

        let rotate_right = instruction[3] == 1;
        let rotate_left = instruction[2] == 1;
        let twice_or_swap = instruction[1] == 1;
        if rotate_right || rotate_left {
            let count = if twice_or_swap { 2 } else { 1 };
            for _ in 0..count {
                let ac = self.ac.read();
                let link = self.link.read()[0];
                let mut next = [0; WIDTH];
                if rotate_right {
                    next[..11].copy_from_slice(&ac[1..]);
                    next[11] = link;
                    self.link.write(&[ac[0]]);
                } else {
                    next[1..].copy_from_slice(&ac[..11]);
                    next[0] = link;
                    self.link.write(&[ac[11]]);
                }
                self.ac.write(&next);
            }
            operations.push(match (rotate_right, count) {
                (true, 1) => "RAR",
                (true, _) => "RTR",
                (false, 1) => "RAL",
                (false, _) => "RTL",
            });
        } else if twice_or_swap {
            let ac = self.ac.read();
            let mut swapped = [0; WIDTH];
            swapped[..6].copy_from_slice(&ac[6..]);
            swapped[6..].copy_from_slice(&ac[..6]);
            self.ac.write(&swapped);
            operations.push("BSW");
        }
        join_operations(&operations)
    }

    fn execute_group_two(&mut self, instruction: &[u8; WIDTH]) -> String {
        let ac = self.ac.read();
        let selected = or_gate(
            or_gate(
                and_gate(instruction[6], ac[11]),
                and_gate(instruction[5], is_zero(&ac)),
            ),
            and_gate(instruction[4], self.link.read()[0]),
        );
        let skip = xor_gate(selected, instruction[3]);
        let mut operations = Vec::new();
        if instruction[6] == 1 {
            operations.push("SMA");
        }
        if instruction[5] == 1 {
            operations.push("SZA");
        }
        if instruction[4] == 1 {
            operations.push("SNL");
        }
        if instruction[3] == 1 {
            operations.push("RSS");
        }
        if skip == 1 {
            self.increment_pc();
        }
        if instruction[7] == 1 {
            self.ac.write(&ZERO_12);
            operations.push("CLA");
        }
        if instruction[2] == 1 {
            self.ac
                .write(&or_bits(&self.ac.read(), &self.switch_register.read()));
            operations.push("OSR");
        }
        if instruction[1] == 1 {
            self.halted.write(&[1]);
            operations.push("HLT");
        }
        join_operations(&operations)
    }

    fn increment_pc(&mut self) {
        self.pc.write(&add_bits(&self.pc.read(), &ONE_12).0);
    }
}

fn validate_program(program: &[u16], origin: usize) -> Result<(), Pdp8Error> {
    validate_program_extent(program.len(), origin)?;
    for (index, word) in program.iter().copied().enumerate() {
        validate_word(word, index)?;
    }
    Ok(())
}

fn validate_program_extent(words: usize, origin: usize) -> Result<(), Pdp8Error> {
    if origin >= MEMORY_WORDS {
        return Err(Pdp8Error::InvalidOrigin { origin });
    }
    let capacity = MEMORY_WORDS - origin;
    if words > capacity {
        return Err(Pdp8Error::ProgramTooLarge { words, capacity });
    }
    Ok(())
}

fn validate_word(word: u16, index: usize) -> Result<(), Pdp8Error> {
    if word > WORD_MASK {
        Err(Pdp8Error::InvalidWord { word, index })
    } else {
        Ok(())
    }
}

fn validate_instruction(bits: &[u8; WIDTH], instruction: u16) -> Result<(), Pdp8Error> {
    let selects = decode_three([bits[9], bits[10], bits[11]]);
    if selects[7] == 1 {
        if bits[8] == 0 && and_gate(bits[3], bits[2]) == 1 {
            return Err(Pdp8Error::InvalidOperate { instruction });
        }
        if bits[8] == 1 && bits[0] == 1 {
            return Err(Pdp8Error::UnsupportedEae { instruction });
        }
    }
    if selects[6] == 1 {
        let device = (instruction >> 3) & 0o77;
        let pulses = instruction & 0o7;
        if device == 0 && (4..=6).contains(&pulses) {
            return Err(Pdp8Error::UnsupportedCpuIot { instruction });
        }
    }
    Ok(())
}

fn decode_three(bits: [u8; 3]) -> [u8; 8] {
    let inverted = bits.map(not_gate);
    std::array::from_fn(|value| {
        let b0 = if value & 1 == 0 { inverted[0] } else { bits[0] };
        let b1 = if value & 2 == 0 { inverted[1] } else { bits[1] };
        let b2 = if value & 4 == 0 { inverted[2] } else { bits[2] };
        and_gate(and_gate(b0, b1), b2)
    })
}

fn add_bits(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> ([u8; WIDTH], u8) {
    let result = ripple_carry_adder(a, b);
    (
        result
            .sum
            .try_into()
            .expect("a ripple-carry adder preserves width"),
        result.carry_out,
    )
}

fn and_bits(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    std::array::from_fn(|bit| and_gate(a[bit], b[bit]))
}

fn or_bits(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    std::array::from_fn(|bit| or_gate(a[bit], b[bit]))
}

fn is_zero(bits: &[u8; WIDTH]) -> u8 {
    not_gate(bits.iter().copied().fold(0, or_gate))
}

fn u16_to_bits(value: u16) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_to_u16(bits: &[u8]) -> u16 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | u16::from(*input) << bit)
}

fn join_operations(operations: &[&str]) -> String {
    if operations.is_empty() {
        "NOP".to_owned()
    } else {
        operations.join(" ")
    }
}
