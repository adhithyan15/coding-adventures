//! DFF-backed x86-64 register and flag storage.

use crate::state::{clock_bit, clock_word};

#[derive(Clone, Debug)]
pub(crate) struct RegisterFile64 {
    gpr: [[u8; 64]; 16],
    rip: [u8; 64],
    flags: [u8; 5],
}

impl RegisterFile64 {
    pub(crate) fn new() -> Self {
        let mut file = Self {
            gpr: [[0; 64]; 16],
            rip: [0; 64],
            flags: [0; 5],
        };
        file.write(4, 0xfff8);
        file
    }

    pub(crate) fn read(&self, index: usize) -> u64 {
        bits_to_u64(&self.gpr[index])
    }

    pub(crate) fn write(&mut self, index: usize, value: u64) {
        clock_word(&mut self.gpr[index], value);
    }

    pub(crate) fn rip(&self) -> u64 {
        bits_to_u64(&self.rip)
    }

    pub(crate) fn flags(&self) -> u64 {
        u64::from(self.flags[0])
            | (u64::from(self.flags[1]) << 2)
            | (u64::from(self.flags[2]) << 6)
            | (u64::from(self.flags[3]) << 7)
            | (u64::from(self.flags[4]) << 11)
    }

    pub(crate) fn snapshot(&self) -> [u64; 16] {
        std::array::from_fn(|index| self.read(index))
    }

    pub(crate) fn restore(&mut self, gpr: &[u64; 16], rip: u64, flags: u64) {
        for (index, value) in gpr.iter().copied().enumerate() {
            self.write(index, value);
        }
        clock_word(&mut self.rip, rip);
        for (q, bit) in self.flags.iter_mut().zip([0, 2, 6, 7, 11]) {
            clock_bit(q, flags & (1 << bit) != 0);
        }
    }
}

fn bits_to_u64(bits: &[u8; 64]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}
