//! Stable D-flip-flop storage for the complete MIPS R2000 gate machine.

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

pub(crate) fn clock_word(q: &mut [u8; 32], value: u32) {
    let input: Vec<u8> = (0..32).map(|bit| ((value >> bit) & 1) as u8).collect();
    let mut state: Vec<FlipFlopState> = q.iter().copied().map(stable_state).collect();
    register(&input, 0, &mut state);
    let output = register(&input, 1, &mut state);
    q.copy_from_slice(&output);
}

pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let input = [u8::from(value)];
    let mut state = [stable_state(*q)];
    register(&input, 0, &mut state);
    *q = register(&input, 1, &mut state)[0];
}

/// One persistent D flip-flop observed at stable instruction boundaries.
///
/// Keeping the Q bit inside this type prevents an adjacent host-language
/// compatibility field from silently becoming the architectural authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DffBit {
    q: u8,
}

impl DffBit {
    pub(crate) const fn new() -> Self {
        Self { q: 0 }
    }

    pub(crate) const fn q(&self) -> bool {
        self.q != 0
    }

    pub(crate) fn clock(&mut self, value: bool) {
        clock_bit(&mut self.q, value);
    }

    pub(crate) const fn dff_count(&self) -> usize {
        1
    }
}

/// Packed stable-Q representation of the 524,288 memory DFFs.
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
        for (offset, byte) in bytes.iter().copied().enumerate() {
            self.write(origin + offset, byte);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.q.fill(0);
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }

    pub(crate) fn restore_snapshot(&mut self, bytes: &[u8]) {
        self.q.copy_from_slice(bytes);
    }

    pub(crate) fn dff_count(&self) -> usize {
        self.q.len() * 8
    }
}

impl Deref for DffMemory {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.q
    }
}
