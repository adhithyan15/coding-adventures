//! Gate-level CDC 6600 simulator for the complete Spec 07t subset.
//!
//! Persistent state is held in simulated master-slave D flip-flops. Decode,
//! arithmetic, compare, shift, multiply, address, and branch paths are composed
//! from the repository's digital-logic primitives.

use arithmetic::adders::{ripple_carry_adder, ripple_carry_adder_with_carry};
pub use cdc6600_simulator::*;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use logic_gates::sequential::{register, FlipFlopState};

const ZERO_18: [u8; 18] = [0; 18];
const ZERO_60: [u8; 60] = [0; 60];
const ONE_18: [u8; 18] = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const TWO_18: [u8; 18] = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// Exact number of persistent master-slave D flip-flops in the model.
pub const FLIP_FLOP_COUNT: usize = MEMORY_WORDS * 60 + 8 * 60 + 8 * 18 + 7 * 18 + 18 + 1;

/// Stable educational storage-plus-combinational gate estimate.
pub const ESTIMATED_GATE_COUNT: usize = FLIP_FLOP_COUNT * 6 + 40_000;

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
            .expect("a fixed-width register preserves its width")
    }

    fn write(&mut self, data: &[u8; WIDTH]) {
        register(data, 0, &mut self.state);
        register(data, 1, &mut self.state);
    }
}

/// CDC 6600 whose architectural state and data paths are digital logic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cdc6600GateLevel {
    p: BitRegister<18>,
    x: [BitRegister<60>; 8],
    a: [BitRegister<18>; 8],
    /// B0 is hardwired; these registers hold B1 through B7.
    b: [BitRegister<18>; 7],
    memory: Vec<BitRegister<60>>,
    halted: BitRegister<1>,
}

impl Default for Cdc6600GateLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl Cdc6600GateLevel {
    /// Construct a zeroed 4,096-word machine.
    pub fn new() -> Self {
        Self {
            p: BitRegister::new(&ZERO_18),
            x: std::array::from_fn(|_| BitRegister::new(&ZERO_60)),
            a: std::array::from_fn(|_| BitRegister::new(&ZERO_18)),
            b: std::array::from_fn(|_| BitRegister::new(&ZERO_18)),
            memory: (0..MEMORY_WORDS)
                .map(|_| BitRegister::new(&ZERO_60))
                .collect(),
            halted: BitRegister::new(&[0]),
        }
    }

    /// Restore every persistent element through flip-flop writes.
    pub fn reset(&mut self) {
        self.p.write(&ZERO_18);
        for register in &mut self.x {
            register.write(&ZERO_60);
        }
        for register in &mut self.a {
            register.write(&ZERO_18);
        }
        for register in &mut self.b {
            register.write(&ZERO_18);
        }
        for word in &mut self.memory {
            word.write(&ZERO_60);
        }
        self.halted.write(&[0]);
    }

    /// Validate and load canonical big-endian 15-bit parcel transport.
    pub fn load(&mut self, program: &[u8]) -> Result<usize, Cdc6600Error> {
        if !program.len().is_multiple_of(2) {
            return Err(Cdc6600Error::InvalidProgramLength {
                bytes: program.len(),
            });
        }
        let parcel_count = program.len() / 2;
        if parcel_count > MEMORY_PARCELS as usize {
            return Err(Cdc6600Error::ProgramTooLarge {
                parcels: parcel_count,
                capacity: MEMORY_PARCELS as usize,
            });
        }
        let parcels = program
            .chunks(2)
            .enumerate()
            .map(|(index, bytes)| {
                let value = u16::from_be_bytes([bytes[0], bytes[1]]);
                if value > PARCEL_MASK {
                    Err(Cdc6600Error::NonCanonicalParcel { index, value })
                } else {
                    Ok(value)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.load_parcels(&parcels)?;
        Ok(parcels.len())
    }

    /// Validate and load decoded 15-bit parcels atomically.
    pub fn load_parcels(&mut self, parcels: &[u16]) -> Result<(), Cdc6600Error> {
        if parcels.len() > MEMORY_PARCELS as usize {
            return Err(Cdc6600Error::ProgramTooLarge {
                parcels: parcels.len(),
                capacity: MEMORY_PARCELS as usize,
            });
        }
        for (index, value) in parcels.iter().copied().enumerate() {
            if value > PARCEL_MASK {
                return Err(Cdc6600Error::NonCanonicalParcel { index, value });
            }
        }

        let mut replacement = Self::new();
        for (index, value) in parcels.iter().copied().enumerate() {
            let word = index / 4;
            let offset = (3 - index % 4) * 15;
            let mut word_bits = replacement.memory[word].read();
            let parcel_bits = bits_from_u16::<15>(value);
            word_bits[offset..offset + 15].copy_from_slice(&parcel_bits);
            replacement.memory[word].write(&word_bits);
        }
        *self = replacement;
        Ok(())
    }

    /// Return an immutable owned snapshot of the complete machine state.
    pub fn state(&self) -> Cdc6600State {
        Cdc6600State {
            p: bits_to_u32(&self.p.read()),
            x: std::array::from_fn(|index| bits_to_u64(&self.x[index].read())),
            a: std::array::from_fn(|index| bits_to_u32(&self.a[index].read())),
            b: std::array::from_fn(|index| {
                if index == 0 {
                    0
                } else {
                    bits_to_u32(&self.b[index - 1].read())
                }
            }),
            memory: self
                .memory
                .iter()
                .map(|word| bits_to_u64(&word.read()))
                .collect(),
            halted: self.halted.read()[0] == 1,
        }
    }

    /// Alias for [`Self::state`] matching the SIM00 lifecycle vocabulary.
    pub fn get_state(&self) -> Cdc6600State {
        self.state()
    }

    /// Read one checked 60-bit memory word.
    pub fn read_word(&self, address: usize) -> Result<u64, Cdc6600Error> {
        self.memory
            .get(address)
            .map(|word| bits_to_u64(&word.read()))
            .ok_or(Cdc6600Error::MemoryAddressOutOfRange {
                address: address as u32,
            })
    }

    /// Clock one boundary value into a checked memory word.
    pub fn write_word(&mut self, address: usize, value: u64) -> Result<(), Cdc6600Error> {
        let word = self
            .memory
            .get_mut(address)
            .ok_or(Cdc6600Error::MemoryAddressOutOfRange {
                address: address as u32,
            })?;
        word.write(&bits_from_u64::<60>(value));
        Ok(())
    }

    /// Set P after validating its parcel address.
    pub fn set_program_counter(&mut self, p: u32) -> Result<(), Cdc6600Error> {
        validate_parcel_address(p)?;
        self.p.write(&bits_from_u32::<18>(p));
        Ok(())
    }

    /// Clock a masked boundary value into Xi.
    pub fn write_x(&mut self, index: usize, value: u64) -> Result<(), Cdc6600Error> {
        let destination = self
            .x
            .get_mut(index)
            .ok_or(Cdc6600Error::InvalidRegister { bank: 'X', index })?;
        destination.write(&bits_from_u64::<60>(value));
        Ok(())
    }

    /// Clock a masked boundary value into Ai.
    pub fn write_a(&mut self, index: usize, value: u32) -> Result<(), Cdc6600Error> {
        let destination = self
            .a
            .get_mut(index)
            .ok_or(Cdc6600Error::InvalidRegister { bank: 'A', index })?;
        destination.write(&bits_from_u32::<18>(value));
        Ok(())
    }

    /// Clock a masked boundary value into Bi; writes to B0 are ignored.
    pub fn write_b(&mut self, index: usize, value: u32) -> Result<(), Cdc6600Error> {
        if index >= 8 {
            return Err(Cdc6600Error::InvalidRegister { bank: 'B', index });
        }
        if index != 0 {
            self.b[index - 1].write(&bits_from_u32::<18>(value));
        }
        Ok(())
    }

    /// Execute one gate-level fetch/decode/execute clock.
    pub fn step(&mut self) -> Result<StepTrace, Cdc6600Error> {
        let pc_bits = self.p.read();
        let pc_before = bits_to_u32(&pc_bits);
        if self.halted.read()[0] == 1 {
            return Ok(StepTrace {
                pc_before,
                pc_after: pc_before,
                instruction: 0,
                parcels: 1,
                mnemonic: "HALT".to_owned(),
                description: "HALT (already halted)".to_owned(),
            });
        }

        let first = self.read_parcel_bits(pc_before)?;
        if zero_bit(&first) == 1 {
            self.halted.write(&[1]);
            return Ok(StepTrace {
                pc_before,
                pc_after: pc_before,
                instruction: 0,
                parcels: 1,
                mnemonic: "HALT".to_owned(),
                description: format!("HALT @ parcel {pc_before:#06x}"),
            });
        }

        let opcode_bits: [u8; 6] = first[9..15]
            .try_into()
            .expect("a parcel contains six opcode bits");
        let selects = decode_opcode(opcode_bits);
        let opcode = bits_to_u32(&opcode_bits) as u8;
        let i = bits_to_u32(&first[6..9]) as usize;
        let j = bits_to_u32(&first[3..6]) as usize;
        let k = bits_to_u32(&first[..3]) as usize;

        let (instruction, parcels, mnemonic, final_p) = if opcode_bits[5] == 1 {
            let second_address = pc_before
                .checked_add(1)
                .filter(|address| *address < MEMORY_PARCELS)
                .ok_or(Cdc6600Error::MissingLongParcel { pc: pc_before })?;
            let second = self.read_parcel_bits(second_address)?;
            let mut constant = [0; 18];
            constant[..15].copy_from_slice(&second);
            constant[15..].copy_from_slice(&first[..3]);
            let next = add_bits(&pc_bits, &TWO_18);
            let (mnemonic, final_p) =
                self.exec_long(&selects, opcode, i, j, &constant, &next, pc_before)?;
            let instruction =
                (u32::from(bits_to_u16(&first)) << 15) | u32::from(bits_to_u16(&second));
            (instruction, 2, mnemonic, final_p)
        } else {
            let next = add_bits(&pc_bits, &ONE_18);
            validate_parcel_address(bits_to_u32(&next))?;
            let mnemonic = self.exec_short(&selects, opcode, i, j, k, &next, pc_before)?;
            (u32::from(bits_to_u16(&first)), 1, mnemonic, next)
        };

        let pc_after = bits_to_u32(&final_p);
        Ok(StepTrace {
            pc_before,
            pc_after,
            instruction,
            parcels,
            description: format!("{mnemonic} @ parcel {pc_before:#06x}"),
            mnemonic,
        })
    }

    /// Run an already-loaded machine until HALT under a mandatory bound.
    pub fn run(&mut self, max_steps: usize) -> Result<ExecutionResult, Cdc6600Error> {
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted.read()[0] == 1 {
                break;
            }
            traces.push(self.step()?);
        }
        if self.halted.read()[0] == 0 {
            return Err(Cdc6600Error::MaxStepsExceeded { max_steps });
        }
        Ok(ExecutionResult {
            halted: true,
            steps: traces.len(),
            final_state: self.state(),
            traces,
        })
    }

    /// Reset, load, and execute a canonical parcel transport.
    pub fn execute(
        &mut self,
        program: &[u8],
        max_steps: usize,
    ) -> Result<ExecutionResult, Cdc6600Error> {
        self.load(program)?;
        self.run(max_steps)
    }

    /// Exact persistent flip-flop topology count.
    pub const fn flip_flop_count(&self) -> usize {
        FLIP_FLOP_COUNT
    }

    /// Stable educational storage-plus-combinational gate estimate.
    pub const fn gate_count(&self) -> usize {
        ESTIMATED_GATE_COUNT
    }

    fn read_parcel_bits(&self, p: u32) -> Result<[u8; 15], Cdc6600Error> {
        validate_parcel_address(p)?;
        let word = self.memory[(p / 4) as usize].read();
        let offset = (3 - p as usize % 4) * 15;
        Ok(word[offset..offset + 15]
            .try_into()
            .expect("a memory word contains four parcels"))
    }

    fn b_bits(&self, index: usize) -> [u8; 18] {
        if index == 0 {
            ZERO_18
        } else {
            self.b[index - 1].read()
        }
    }

    fn write_b_bits(&mut self, index: usize, value: &[u8; 18]) {
        if index != 0 {
            self.b[index - 1].write(value);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn exec_short(
        &mut self,
        selects: &[u8; 64],
        opcode: u8,
        i: usize,
        j: usize,
        k: usize,
        next: &[u8; 18],
        pc: u32,
    ) -> Result<String, Cdc6600Error> {
        if or_reduce(&selects[1..23]) == 0 {
            return Err(Cdc6600Error::UnknownShortOpcode { opcode, pc });
        }

        let xj = self.x[j].read();
        let xk = self.x[k].read();
        let aj = self.a[j].read();
        let bj = self.b_bits(j);
        let bk = self.b_bits(k);

        let mnemonic = if selects[F_TXB as usize] == 1 {
            self.x[i].write(&zero_extend::<18, 60>(&bj));
            format!("TXB X{i},B{j}")
        } else if selects[F_TBX as usize] == 1 {
            self.write_b_bits(i, &xj[..18].try_into().expect("X has 18 low bits"));
            format!("TBX B{i},X{j}")
        } else if selects[F_TAX as usize] == 1 {
            self.x[i].write(&zero_extend::<18, 60>(&aj));
            format!("TAX X{i},A{j}")
        } else if selects[F_TXA as usize] == 1 {
            self.a[i].write(&xj[..18].try_into().expect("X has 18 low bits"));
            format!("TXA A{i},X{j}")
        } else if selects[F_IXPB as usize] == 1 {
            self.x[i].write(&add_bits(&xj, &zero_extend::<18, 60>(&bk)));
            format!("IXPB X{i},X{j},B{k}")
        } else if selects[F_IXMB as usize] == 1 {
            self.x[i].write(&sub_bits(&xj, &zero_extend::<18, 60>(&bk)));
            format!("IXMB X{i},X{j},B{k}")
        } else if selects[F_IXXP as usize] == 1 {
            self.x[i].write(&add_bits(&xj, &xk));
            format!("IXXP X{i},X{j},X{k}")
        } else if selects[F_IXXM as usize] == 1 {
            self.x[i].write(&sub_bits(&xj, &xk));
            format!("IXXM X{i},X{j},X{k}")
        } else if selects[F_BXND as usize] == 1 {
            self.x[i].write(&binary_bits(&xj, &xk, and_gate));
            format!("BXND X{i},X{j},X{k}")
        } else if selects[F_BXOR as usize] == 1 {
            self.x[i].write(&binary_bits(&xj, &xk, or_gate));
            format!("BXOR X{i},X{j},X{k}")
        } else if selects[F_BXXR as usize] == 1 {
            self.x[i].write(&binary_bits(&xj, &xk, xor_gate));
            format!("BXXR X{i},X{j},X{k}")
        } else if selects[F_BXMR as usize] == 1 {
            self.x[i].write(&xj.map(not_gate));
            format!("BXMR X{i},X{j}")
        } else if selects[F_LSHL as usize] == 1 {
            self.x[i].write(&barrel_shift(&xj, &bk[..6], true));
            format!("LSHL X{i},X{j},B{k}")
        } else if selects[F_LSHR as usize] == 1 {
            self.x[i].write(&barrel_shift(&xj, &bk[..6], false));
            format!("LSHR X{i},X{j},B{k}")
        } else if selects[F_IBBP as usize] == 1 {
            self.write_b_bits(i, &add_bits(&bj, &bk));
            format!("IBBP B{i},B{j},B{k}")
        } else if selects[F_IBBM as usize] == 1 {
            self.write_b_bits(i, &sub_bits(&bj, &bk));
            format!("IBBM B{i},B{j},B{k}")
        } else if selects[F_IAAP as usize] == 1 {
            self.a[i].write(&add_bits(&aj, &bk));
            format!("IAAP A{i},A{j},B{k}")
        } else if selects[F_IAAM as usize] == 1 {
            self.a[i].write(&sub_bits(&aj, &bk));
            format!("IAAM A{i},A{j},B{k}")
        } else if selects[F_CMPEQ as usize] == 1 {
            self.write_b_bits(i, &bool_to_18(equal_bit(&xj, &xk)));
            format!("CMPEQ B{i},X{j},X{k}")
        } else if selects[F_CMPLT as usize] == 1 {
            self.write_b_bits(i, &bool_to_18(signed_less(&xj, &xk)));
            format!("CMPLT B{i},X{j},X{k}")
        } else if selects[F_CMPGT as usize] == 1 {
            self.write_b_bits(i, &bool_to_18(signed_less(&xk, &xj)));
            format!("CMPGT B{i},X{j},X{k}")
        } else {
            debug_assert_eq!(selects[F_IXMUL as usize], 1);
            self.x[i].write(&multiply_low(&xj, &xk));
            format!("IXMUL X{i},X{j},X{k}")
        };

        self.p.write(next);
        Ok(mnemonic)
    }

    #[allow(clippy::too_many_arguments)]
    fn exec_long(
        &mut self,
        selects: &[u8; 64],
        opcode: u8,
        i: usize,
        j: usize,
        constant: &[u8; 18],
        next: &[u8; 18],
        pc: u32,
    ) -> Result<(String, [u8; 18]), Cdc6600Error> {
        let known = or_gate(or_reduce(&selects[32..39]), or_reduce(&selects[40..47]));
        if known == 0 {
            return Err(Cdc6600Error::UnknownLongOpcode { opcode, pc });
        }

        let constant_value = bits_to_u32(constant);
        let next_value = bits_to_u32(next);
        let mut final_p = *next;
        let mut memory_write: Option<(usize, [u8; 60])> = None;

        let mnemonic = if selects[F_LDXI as usize] == 1 {
            validate_parcel_address(next_value)?;
            self.x[i].write(&zero_extend::<18, 60>(constant));
            format!("LDXI X{i},{constant_value}")
        } else if selects[F_LDBI as usize] == 1 {
            validate_parcel_address(next_value)?;
            self.write_b_bits(i, constant);
            format!("LDBI B{i},{constant_value}")
        } else if selects[F_LDAI as usize] == 1 {
            validate_parcel_address(next_value)?;
            self.a[i].write(constant);
            format!("LDAI A{i},{constant_value}")
        } else if selects[F_LDX as usize] == 1 {
            validate_parcel_address(next_value)?;
            let address = checked_memory_address(&self.a[j].read(), constant)?;
            self.x[i].write(&self.memory[address].read());
            format!("LDX X{i},A{j}+{constant_value}")
        } else if selects[F_STX as usize] == 1 {
            validate_parcel_address(next_value)?;
            let address = checked_memory_address(&self.a[i].read(), constant)?;
            memory_write = Some((address, self.x[j].read()));
            format!("STX A{i}+{constant_value},X{j}")
        } else if selects[F_LDB as usize] == 1 {
            validate_parcel_address(next_value)?;
            let address = checked_memory_address(&self.a[j].read(), constant)?;
            let word = self.memory[address].read();
            self.write_b_bits(i, &word[..18].try_into().expect("word has low bits"));
            format!("LDB B{i},A{j}+{constant_value}")
        } else if selects[F_STB as usize] == 1 {
            validate_parcel_address(next_value)?;
            let address = checked_memory_address(&self.a[i].read(), constant)?;
            memory_write = Some((address, zero_extend::<18, 60>(&self.b_bits(j))));
            format!("STB A{i}+{constant_value},B{j}")
        } else if selects[F_JEQ as usize] == 1 {
            let taken = zero_bit(&self.b_bits(j));
            final_p = branch_target(taken, next, constant)?;
            format!("JEQ B{j}==0,{constant_value}")
        } else if selects[F_JNE as usize] == 1 {
            let taken = not_gate(zero_bit(&self.b_bits(j)));
            final_p = branch_target(taken, next, constant)?;
            format!("JNE B{j}!=0,{constant_value}")
        } else if selects[F_JXZ as usize] == 1 {
            let taken = zero_bit(&self.x[j].read());
            final_p = branch_target(taken, next, constant)?;
            format!("JXZ X{j}==0,{constant_value}")
        } else if selects[F_JXN as usize] == 1 {
            let taken = not_gate(zero_bit(&self.x[j].read()));
            final_p = branch_target(taken, next, constant)?;
            format!("JXN X{j}!=0,{constant_value}")
        } else if selects[F_JMP as usize] == 1 {
            validate_parcel_address(constant_value)?;
            final_p = *constant;
            format!("JMP {constant_value}")
        } else if selects[F_JSR as usize] == 1 {
            validate_parcel_address(next_value)?;
            validate_parcel_address(constant_value)?;
            self.write_b_bits(7, next);
            final_p = *constant;
            format!("JSR B7={next_value},P={constant_value}")
        } else {
            debug_assert_eq!(selects[F_RET as usize], 1);
            let target = self.b_bits(j);
            validate_parcel_address(bits_to_u32(&target))?;
            final_p = target;
            format!("RET P=B{j}")
        };

        if let Some((address, value)) = memory_write {
            self.memory[address].write(&value);
        }
        self.p.write(&final_p);
        Ok((mnemonic, final_p))
    }
}

fn validate_parcel_address(target: u32) -> Result<(), Cdc6600Error> {
    if target >= MEMORY_PARCELS {
        Err(Cdc6600Error::ProgramCounterOutOfRange { target })
    } else {
        Ok(())
    }
}

fn checked_memory_address(base: &[u8; 18], constant: &[u8; 18]) -> Result<usize, Cdc6600Error> {
    let address = add_bits(base, constant);
    let value = bits_to_u32(&address);
    if value >= MEMORY_WORDS as u32 {
        Err(Cdc6600Error::MemoryAddressOutOfRange { address: value })
    } else {
        Ok(value as usize)
    }
}

fn branch_target(
    taken: u8,
    fallthrough: &[u8; 18],
    target: &[u8; 18],
) -> Result<[u8; 18], Cdc6600Error> {
    let selected = mux_bits(taken, fallthrough, target);
    validate_parcel_address(bits_to_u32(&selected))?;
    Ok(selected)
}

fn decode_opcode(bits: [u8; 6]) -> [u8; 64] {
    let inverted = bits.map(not_gate);
    std::array::from_fn(|opcode| {
        bits.iter().enumerate().fold(1, |selected, (bit, input)| {
            let wire = if opcode & (1 << bit) == 0 {
                inverted[bit]
            } else {
                *input
            };
            and_gate(selected, wire)
        })
    })
}

fn add_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder(a, b)
        .sum
        .try_into()
        .expect("a ripple-carry adder preserves width")
}

fn sub_bits<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> [u8; WIDTH] {
    ripple_carry_adder_with_carry(a, &b.map(not_gate), 1)
        .sum
        .try_into()
        .expect("a ripple-carry subtractor preserves width")
}

fn binary_bits<const WIDTH: usize>(
    a: &[u8; WIDTH],
    b: &[u8; WIDTH],
    gate: fn(u8, u8) -> u8,
) -> [u8; WIDTH] {
    std::array::from_fn(|bit| gate(a[bit], b[bit]))
}

fn mux(select: u8, zero: u8, one: u8) -> u8 {
    or_gate(and_gate(not_gate(select), zero), and_gate(select, one))
}

fn mux_bits<const WIDTH: usize>(select: u8, zero: &[u8; WIDTH], one: &[u8; WIDTH]) -> [u8; WIDTH] {
    std::array::from_fn(|bit| mux(select, zero[bit], one[bit]))
}

fn or_reduce(bits: &[u8]) -> u8 {
    bits.iter().copied().fold(0, or_gate)
}

fn zero_bit(bits: &[u8]) -> u8 {
    not_gate(or_reduce(bits))
}

fn equal_bit(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).fold(1, |equal, (left, right)| {
        and_gate(equal, not_gate(xor_gate(*left, *right)))
    })
}

fn unsigned_less(a: &[u8], b: &[u8]) -> u8 {
    a.iter()
        .zip(b)
        .rev()
        .fold((0, 1), |(less, equal), (left, right)| {
            let less_here = and_gate(equal, and_gate(not_gate(*left), *right));
            let next_less = or_gate(less, less_here);
            let next_equal = and_gate(equal, not_gate(xor_gate(*left, *right)));
            (next_less, next_equal)
        })
        .0
}

fn signed_less<const WIDTH: usize>(a: &[u8; WIDTH], b: &[u8; WIDTH]) -> u8 {
    let sign_difference = xor_gate(a[WIDTH - 1], b[WIDTH - 1]);
    mux(sign_difference, unsigned_less(a, b), a[WIDTH - 1])
}

fn barrel_shift(value: &[u8; 60], controls: &[u8], left: bool) -> [u8; 60] {
    let mut output = *value;
    for (stage, control) in controls.iter().copied().enumerate() {
        let distance = 1 << stage;
        let shifted = std::array::from_fn(|bit| {
            if left {
                bit.checked_sub(distance)
                    .map(|source| output[source])
                    .unwrap_or(0)
            } else {
                output.get(bit + distance).copied().unwrap_or(0)
            }
        });
        output = mux_bits(control, &output, &shifted);
    }
    output
}

fn multiply_low(a: &[u8; 60], b: &[u8; 60]) -> [u8; 60] {
    let mut product = ZERO_60;
    for (row, multiplier) in b.iter().copied().enumerate() {
        let partial = std::array::from_fn(|bit| {
            bit.checked_sub(row)
                .map(|source| and_gate(a[source], multiplier))
                .unwrap_or(0)
        });
        product = add_bits(&product, &partial);
    }
    product
}

fn bool_to_18(value: u8) -> [u8; 18] {
    let mut bits = ZERO_18;
    bits[0] = value;
    bits
}

fn zero_extend<const SOURCE: usize, const DESTINATION: usize>(
    source: &[u8; SOURCE],
) -> [u8; DESTINATION] {
    debug_assert!(SOURCE <= DESTINATION);
    let mut result = [0; DESTINATION];
    result[..SOURCE].copy_from_slice(source);
    result
}

fn bits_from_u64<const WIDTH: usize>(value: u64) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_from_u32<const WIDTH: usize>(value: u32) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_from_u16<const WIDTH: usize>(value: u16) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | u64::from(*input) << bit)
}

fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | u32::from(*input) << bit)
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
    fn decoder_is_one_hot_for_every_opcode() {
        for opcode in 0..64_u8 {
            let decoded = decode_opcode(bits_from_u16::<6>(u16::from(opcode)));
            assert_eq!(decoded.iter().sum::<u8>(), 1);
            assert_eq!(decoded[usize::from(opcode)], 1);
        }
    }

    #[test]
    fn register_and_gate_datapaths_hold_expected_bits() {
        let mut register = BitRegister::<18>::new(&ZERO_18);
        register.write(&bits_from_u32::<18>(0x2aaaa));
        assert_eq!(bits_to_u32(&register.read()), 0x2aaaa);
        assert_eq!(bits_to_u32(&add_bits(&[1; 18], &ONE_18)), 0);
        assert_eq!(bits_to_u32(&sub_bits(&ZERO_18, &ONE_18)), MASK_18);
        assert_eq!(signed_less(&bits_from_u64::<60>(MASK_60), &ZERO_60), 1);
        assert_eq!(
            bits_to_u64(&multiply_low(
                &bits_from_u64::<60>(7),
                &bits_from_u64::<60>(9)
            )),
            63
        );
    }
}
