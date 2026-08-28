//! Fixed-width persistent state backed by D flip-flops.

use logic_gates::sequential::{register, FlipFlopState};

use crate::bits::int_to_bits;

/// A fixed-width state word whose bits are all stored in D flip-flops.
#[derive(Clone)]
pub(crate) struct StateRegister {
    state: Vec<FlipFlopState>,
}

impl StateRegister {
    pub(crate) fn new(width: usize) -> Self {
        let mut state = vec![FlipFlopState::default(); width];
        let zeros = vec![0; width];
        register(&zeros, 0, &mut state);
        register(&zeros, 1, &mut state);
        Self { state }
    }

    pub(crate) fn read(&self) -> u16 {
        self.state
            .iter()
            .enumerate()
            .fold(0, |value, (bit, state)| {
                value | (u16::from(state.slave_q) << bit)
            })
    }

    pub(crate) fn write(&mut self, value: u16) {
        let bits = int_to_bits(value as u8, self.state.len());
        register(&bits, 0, &mut self.state);
        register(&bits, 1, &mut self.state);
    }

    pub(crate) fn reset(&mut self) {
        self.write(0);
    }
}

/// The complete 16 KiB memory array, with one DFF-backed byte per address.
#[derive(Clone)]
pub(crate) struct DffMemory {
    bytes: Vec<StateRegister>,
}

impl DffMemory {
    pub(crate) const BYTE_LEN: usize = 16_384;

    pub(crate) fn new() -> Self {
        Self {
            bytes: (0..Self::BYTE_LEN).map(|_| StateRegister::new(8)).collect(),
        }
    }

    pub(crate) fn read(&self, address: usize) -> u8 {
        self.bytes[address].read() as u8
    }

    pub(crate) fn write(&mut self, address: usize, value: u8) {
        self.bytes[address].write(u16::from(value));
    }

    pub(crate) fn copy_from_slice(&mut self, start: usize, bytes: &[u8]) {
        for (offset, byte) in bytes.iter().copied().enumerate() {
            self.write(start + offset, byte);
        }
    }

    pub(crate) fn snapshot(&self) -> Box<[u8; Self::BYTE_LEN]> {
        let mut result = Box::new([0; Self::BYTE_LEN]);
        for (address, byte) in result.iter_mut().enumerate() {
            *byte = self.read(address);
        }
        result
    }
}

impl Default for DffMemory {
    fn default() -> Self {
        Self::new()
    }
}
