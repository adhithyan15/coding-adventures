//! Stable-Q architectural storage clocked through repository D flip-flops.

use logic_gates::sequential::{d_flip_flop, FlipFlopState};
use pa_risc11_simulator::{PaRisc11Error, PaRisc11State, MEMORY_SIZE};

fn stable(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

pub(crate) fn clock_bits<const WIDTH: usize>(q: &mut [u8; WIDTH], value: u64) {
    for (bit, stored) in q.iter_mut().enumerate() {
        let input = ((value >> bit) & 1) as u8;
        let mut state = stable(*stored);
        d_flip_flop(input, 0, &mut state);
        *stored = d_flip_flop(input, 1, &mut state).0;
    }
}

pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let input = u8::from(value);
    let mut state = stable(*q);
    d_flip_flop(input, 0, &mut state);
    *q = d_flip_flop(input, 1, &mut state).0;
}

fn read(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

#[derive(Clone, Debug)]
pub(crate) struct DffMemory {
    q: Vec<u8>,
}

impl DffMemory {
    fn new() -> Self {
        Self {
            q: vec![0; MEMORY_SIZE * 8],
        }
    }

    pub(crate) fn bit_len(&self) -> usize {
        self.q.len()
    }

    pub(crate) fn read(&self, address: usize) -> u8 {
        (0..8).fold(0, |byte, bit| byte | (self.q[address * 8 + bit] << bit))
    }

    pub(crate) fn write(&mut self, address: usize, value: u8) {
        let start = address * 8;
        for bit in 0..8 {
            let input = (value >> bit) & 1;
            let mut state = stable(self.q[start + bit]);
            d_flip_flop(input, 0, &mut state);
            self.q[start + bit] = d_flip_flop(input, 1, &mut state).0;
        }
    }

    fn project(&self) -> Result<Vec<u8>, PaRisc11Error> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(MEMORY_SIZE)
            .map_err(|_| PaRisc11Error::AllocationFailed)?;
        bytes.extend((0..MEMORY_SIZE).map(|address| self.read(address)));
        Ok(bytes)
    }

    fn apply(&mut self, bytes: &[u8]) {
        for (address, value) in bytes.iter().copied().enumerate() {
            if self.read(address) != value {
                self.write(address, value);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PhysicalState {
    pub(crate) memory: DffMemory,
    gpr: [[u8; 32]; 32],
    pc: [u8; 32],
    next_pc: [u8; 32],
    carry_borrow: [u8; 8],
    nullify_next: u8,
    in_delay_slot: u8,
    halted: u8,
    loaded_origin: [u8; 32],
    loaded_len: [u8; 17],
}

impl PhysicalState {
    pub(crate) fn new() -> Self {
        let mut state = Self {
            memory: DffMemory::new(),
            gpr: [[0; 32]; 32],
            pc: [0; 32],
            next_pc: [0; 32],
            carry_borrow: [0; 8],
            nullify_next: 0,
            in_delay_slot: 0,
            halted: 0,
            loaded_origin: [0; 32],
            loaded_len: [0; 17],
        };
        state.apply(&PaRisc11State::default());
        state
    }

    pub(crate) fn project(&self) -> Result<PaRisc11State, PaRisc11Error> {
        Ok(PaRisc11State {
            gpr: std::array::from_fn(|register| read(&self.gpr[register]) as u32),
            carry_borrow: self.carry_borrow.map(|q| q == 1),
            pc: read(&self.pc) as u32,
            next_pc: read(&self.next_pc) as u32,
            nullify_next: self.nullify_next == 1,
            in_delay_slot: self.in_delay_slot == 1,
            memory: self.memory.project()?,
            halted: self.halted == 1,
            loaded_origin: read(&self.loaded_origin) as u32,
            loaded_len: read(&self.loaded_len) as usize,
        })
    }

    pub(crate) fn apply(&mut self, value: &PaRisc11State) {
        self.memory.apply(&value.memory);
        for (register, q) in self.gpr.iter_mut().enumerate() {
            clock_bits(q, u64::from(value.gpr[register]));
        }
        clock_bits(&mut self.pc, u64::from(value.pc));
        clock_bits(&mut self.next_pc, u64::from(value.next_pc));
        for (q, value) in self.carry_borrow.iter_mut().zip(value.carry_borrow) {
            clock_bit(q, value);
        }
        clock_bit(&mut self.nullify_next, value.nullify_next);
        clock_bit(&mut self.in_delay_slot, value.in_delay_slot);
        clock_bit(&mut self.halted, value.halted);
        clock_bits(&mut self.loaded_origin, u64::from(value.loaded_origin));
        clock_bits(&mut self.loaded_len, value.loaded_len as u64);
    }

    pub(crate) fn inventory(&self) -> PhysicalStorageInventory {
        PhysicalStorageInventory {
            memory: self.memory.bit_len(),
            general_registers: self.gpr.len() * self.gpr[0].len(),
            instruction_queue: self.pc.len() + self.next_pc.len(),
            carry_borrow: self.carry_borrow.len(),
            nullify: usize::from(self.nullify_next <= 1),
            delay: usize::from(self.in_delay_slot <= 1),
            halt: usize::from(self.halted <= 1),
            loaded_origin: self.loaded_origin.len(),
            loaded_len: self.loaded_len.len(),
        }
    }

    #[cfg(test)]
    pub(crate) fn corrupt_halt_q(&mut self, halted: bool) {
        clock_bit(&mut self.halted, halted);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalStorageInventory {
    pub memory: usize,
    pub general_registers: usize,
    pub instruction_queue: usize,
    pub carry_borrow: usize,
    pub nullify: usize,
    pub delay: usize,
    pub halt: usize,
    pub loaded_origin: usize,
    pub loaded_len: usize,
}

impl PhysicalStorageInventory {
    #[must_use]
    pub fn total(self) -> usize {
        self.memory
            + self.general_registers
            + self.instruction_queue
            + self.carry_borrow
            + self.nullify
            + self.delay
            + self.halt
            + self.loaded_origin
            + self.loaded_len
    }
}
