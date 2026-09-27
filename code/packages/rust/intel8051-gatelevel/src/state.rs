//! Stable D-flip-flop-backed storage for the complete 8051 machine.

use logic_gates::sequential::{register, FlipFlopState};
use std::ops::{Deref, DerefMut};

fn stable_state(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

/// A small explicitly modelled master/slave D-flip-flop register.
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

    pub(crate) fn write(&mut self, value: u16) {
        let bits: Vec<u8> = (0..self.state.len())
            .map(|bit| ((value >> bit) & 1) as u8)
            .collect();
        register(&bits, 0, &mut self.state);
        register(&bits, 1, &mut self.state);
    }

    #[cfg(test)]
    pub(crate) fn read(&self) -> u16 {
        self.state
            .iter()
            .enumerate()
            .fold(0, |value, (bit, state)| {
                value | (u16::from(state.slave_q) << bit)
            })
    }
}

/// Packed stable-Q representation of a byte-addressed DFF bank.
///
/// At an instruction boundary a master/slave DFF's full stable state is
/// determined by Q. Writes reconstruct that state and clock both phases through
/// the repository's sequential-gate primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DffMemory {
    q: Vec<u8>,
}

impl DffMemory {
    pub(crate) fn new(byte_len: usize) -> Self {
        Self {
            q: vec![0; byte_len],
        }
    }

    pub(crate) fn read(&self, address: usize) -> u8 {
        self.q[address]
    }

    pub(crate) fn write(&mut self, address: usize, value: u8) {
        let old = self.q[address];
        let bits: Vec<u8> = (0..8).map(|bit| (value >> bit) & 1).collect();
        let mut state: Vec<FlipFlopState> =
            (0..8).map(|bit| stable_state((old >> bit) & 1)).collect();
        register(&bits, 0, &mut state);
        let output = register(&bits, 1, &mut state);
        self.q[address] = output
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));
    }

    pub(crate) fn copy_from_slice(&mut self, origin: usize, bytes: &[u8]) {
        for (offset, value) in bytes.iter().copied().enumerate() {
            self.write(origin + offset, value);
        }
    }

    pub(crate) fn restore_snapshot(&mut self, bytes: &[u8]) {
        self.q.copy_from_slice(bytes);
    }

    pub(crate) fn clear(&mut self) {
        self.q.fill(0);
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }
}

// Preserve the original public byte-indexing surface for inspection and legacy
// tests. Simulator-owned writes always use `write`; direct mutation represents
// an externally forced stable Q bus.
impl Deref for DffMemory {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.q
    }
}

impl DerefMut for DffMemory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.q
    }
}
