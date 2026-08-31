//! DFF-backed Alpha integer register file and PC pair.

use crate::state::clock_word;

const ZERO_REGISTER: usize = 31;

#[derive(Clone, Debug)]
pub(crate) struct RegisterFile64 {
    regs: [[u8; 64]; 32],
    pc: [u8; 64],
    npc: [u8; 64],
}

impl RegisterFile64 {
    pub(crate) fn new() -> Self {
        let mut rf = Self {
            regs: [[0; 64]; 32],
            pc: [0; 64],
            npc: [0; 64],
        };
        clock_word(&mut rf.npc, 4);
        rf
    }

    pub(crate) fn read(&self, index: usize) -> u64 {
        if index == ZERO_REGISTER {
            0
        } else {
            bits_to_u64(&self.regs[index])
        }
    }

    pub(crate) fn write(&mut self, index: usize, value: u64) {
        if index != ZERO_REGISTER {
            clock_word(&mut self.regs[index], value);
        }
    }

    pub(crate) fn pc(&self) -> u64 {
        bits_to_u64(&self.pc)
    }

    pub(crate) fn npc(&self) -> u64 {
        bits_to_u64(&self.npc)
    }

    pub(crate) fn write_pc(&mut self, value: u64) {
        clock_word(&mut self.pc, value);
    }

    pub(crate) fn write_npc(&mut self, value: u64) {
        clock_word(&mut self.npc, value);
    }

    pub(crate) fn snapshot(&self) -> [u64; 32] {
        std::array::from_fn(|index| self.read(index))
    }

    pub(crate) fn restore(&mut self, regs: &[u64; 32], pc: u64, npc: u64) {
        for (index, value) in regs.iter().copied().enumerate() {
            if index == ZERO_REGISTER {
                clock_word(&mut self.regs[index], 0);
            } else {
                clock_word(&mut self.regs[index], value);
            }
        }
        self.write_pc(pc);
        self.write_npc(npc);
    }
}

fn bits_to_u64(bits: &[u8; 64]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}
