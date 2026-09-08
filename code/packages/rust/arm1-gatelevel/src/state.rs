//! Stable D-flip-flop storage for the ARM1 gate-level machine.
//!
//! At an instruction boundary the complete stable state of a master/slave D
//! flip-flop is determined by Q: `(q, !q, q, !q)`. The 64 MiB memory and the
//! register file therefore keep one packed Q bit per architectural DFF and
//! reconstruct the internal latch state whenever a value is clocked. This is
//! lossless at the simulator's observable boundary and avoids a multi-gigabyte
//! host representation.

use logic_gates::sequential::{register, FlipFlopState};
use std::ops::Deref;

fn stable_state(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

/// Clock a 32-bit word into packed stable-Q storage.
pub(crate) fn clock_word(q: &mut [u8; 32], value: u32) {
    let input: Vec<u8> = (0..32).map(|bit| ((value >> bit) & 1) as u8).collect();
    let mut state: Vec<FlipFlopState> = q.iter().copied().map(stable_state).collect();
    register(&input, 0, &mut state);
    let output = register(&input, 1, &mut state);
    q.copy_from_slice(&output);
}

/// Clock one bit into packed stable-Q storage.
pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let input = [u8::from(value)];
    let mut state = [stable_state(*q)];
    register(&input, 0, &mut state);
    *q = register(&input, 1, &mut state)[0];
}

/// Packed stable-Q representation of a byte-addressed DFF bank.
#[derive(Clone, Debug, PartialEq, Eq)]
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
        let input: Vec<u8> = (0..8).map(|bit| (value >> bit) & 1).collect();
        let mut state: Vec<FlipFlopState> =
            (0..8).map(|bit| stable_state((old >> bit) & 1)).collect();
        register(&input, 0, &mut state);
        let output = register(&input, 1, &mut state);
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

    pub(crate) fn clear(&mut self) {
        self.q.fill(0);
    }

    pub(crate) fn restore_snapshot(&mut self, bytes: &[u8]) {
        self.q.copy_from_slice(bytes);
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }
}

impl Deref for DffMemory {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.q
    }
}
