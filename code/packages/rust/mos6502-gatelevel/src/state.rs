//! D flip-flop-backed persistent state for the complete MOS 6502 machine.

use logic_gates::sequential::{register, FlipFlopState};

#[derive(Debug, Clone)]
pub(crate) struct StateRegister {
    state: Vec<FlipFlopState>,
}

impl StateRegister {
    pub(crate) fn new(width: usize) -> Self {
        Self {
            state: vec![FlipFlopState::default(); width],
        }
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
        let bits: Vec<u8> = (0..self.state.len())
            .map(|bit| ((value >> bit) & 1) as u8)
            .collect();
        register(&bits, 0, &mut self.state);
        register(&bits, 1, &mut self.state);
    }
}

#[derive(Clone)]
pub(crate) struct DffMemory {
    state: Vec<FlipFlopState>,
}

impl DffMemory {
    pub(crate) const BYTE_LEN: usize = 65_536;
    pub(crate) const DFF_COUNT: usize = Self::BYTE_LEN * 8;

    pub(crate) fn new() -> Self {
        Self {
            state: vec![FlipFlopState::default(); Self::DFF_COUNT],
        }
    }

    pub(crate) fn read(&self, address: usize) -> u8 {
        let start = address * 8;
        self.state[start..start + 8]
            .iter()
            .enumerate()
            .fold(0, |value, (bit, state)| value | (state.slave_q << bit))
    }

    pub(crate) fn write(&mut self, address: usize, value: u8) {
        let start = address * 8;
        let bits: Vec<u8> = (0..8).map(|bit| (value >> bit) & 1).collect();
        register(&bits, 0, &mut self.state[start..start + 8]);
        register(&bits, 1, &mut self.state[start..start + 8]);
    }

    pub(crate) fn copy_from_slice(&mut self, bytes: &[u8]) {
        for (address, byte) in bytes.iter().copied().enumerate() {
            self.write(address, byte);
        }
    }

    pub(crate) fn snapshot(&self) -> Box<[u8]> {
        (0..Self::BYTE_LEN)
            .map(|address| self.read(address))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}
