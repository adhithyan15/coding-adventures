//! DFF-backed PowerPC 601 integer and special-register file.

use crate::state::clock_word;

#[derive(Clone, Debug)]
pub(crate) struct RegisterFile32 {
    gpr: [[u8; 32]; 32],
    lr: [u8; 32],
    ctr: [u8; 32],
    xer: [u8; 32],
    cr: [u8; 32],
    cia: [u8; 32],
}

impl RegisterFile32 {
    pub(crate) fn new() -> Self {
        Self {
            gpr: [[0; 32]; 32],
            lr: [0; 32],
            ctr: [0; 32],
            xer: [0; 32],
            cr: [0; 32],
            cia: [0; 32],
        }
    }
    pub(crate) fn read(&self, index: usize) -> u32 {
        bits_to_u32(&self.gpr[index])
    }
    pub(crate) fn write(&mut self, index: usize, value: u32) {
        clock_word(&mut self.gpr[index], value);
    }
    pub(crate) fn snapshot(&self) -> [u32; 32] {
        std::array::from_fn(|index| self.read(index))
    }
    pub(crate) fn cia(&self) -> u32 {
        bits_to_u32(&self.cia)
    }
    pub(crate) fn lr(&self) -> u32 {
        bits_to_u32(&self.lr)
    }
    pub(crate) fn ctr(&self) -> u32 {
        bits_to_u32(&self.ctr)
    }
    pub(crate) fn xer(&self) -> u32 {
        bits_to_u32(&self.xer)
    }
    pub(crate) fn cr(&self) -> u32 {
        bits_to_u32(&self.cr)
    }
    pub(crate) fn restore(
        &mut self,
        gpr: &[u32; 32],
        lr: u32,
        ctr: u32,
        xer: u32,
        cr: u32,
        cia: u32,
    ) {
        for (index, value) in gpr.iter().copied().enumerate() {
            self.write(index, value);
        }
        clock_word(&mut self.lr, lr);
        clock_word(&mut self.ctr, ctr);
        clock_word(&mut self.xer, xer);
        clock_word(&mut self.cr, cr);
        clock_word(&mut self.cia, cia);
    }
}

fn bits_to_u32(bits: &[u8; 32]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u32::from(*q) << bit))
}
