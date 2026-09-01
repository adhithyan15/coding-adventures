//! Stable D-flip-flop-backed storage for the complete 68000 machine.
//!
//! A master/slave D flip-flop has four internal latch outputs, but at the
//! instruction boundary both latches are stable and their complete state is
//! determined by Q: `(q, !q, q, !q)`. Memory therefore stores one packed Q bit
//! per architectural DFF and reconstructs the transient latch state whenever a
//! byte is clocked. This is lossless at the simulator's observable clock
//! boundary and avoids expanding 16 MiB of memory into a 512 MiB host object.

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

/// A small bank of explicitly modelled master/slave D flip-flops.
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

    pub(crate) fn write(&mut self, value: u32) {
        let bits: Vec<u8> = (0..self.state.len())
            .map(|bit| ((value >> bit) & 1) as u8)
            .collect();
        register(&bits, 0, &mut self.state);
        register(&bits, 1, &mut self.state);
    }

    #[cfg(test)]
    pub(crate) fn read(&self) -> u32 {
        self.state
            .iter()
            .enumerate()
            .fold(0, |value, (bit, state)| {
                value | (u32::from(state.slave_q) << bit)
            })
    }
}

/// Packed stable-Q representation of the 134,217,728 memory DFFs.
#[derive(Clone)]
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
        for (offset, byte) in bytes.iter().copied().enumerate() {
            self.write(origin + offset, byte);
        }
    }

    pub(crate) fn restore_snapshot(&mut self, bytes: &[u8]) {
        self.q.copy_from_slice(bytes);
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }
}

// Preserve the crate's original public byte-indexing surface. Simulator-owned
// writes use `write`, while direct mutation remains a legacy test/inspection
// escape hatch representing an externally forced stable Q bus.
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
