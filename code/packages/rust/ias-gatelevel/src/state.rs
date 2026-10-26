//! Stable-Q architectural storage clocked through repository D flip-flops.

use logic_gates::sequential::{register, FlipFlopState};

fn stable_state(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

pub(crate) fn clock_bits<const WIDTH: usize>(q: &mut [u8; WIDTH], value: u64) {
    let input: [u8; WIDTH] = std::array::from_fn(|bit| ((value >> bit) & 1) as u8);
    let mut state: [FlipFlopState; WIDTH] = std::array::from_fn(|bit| stable_state(q[bit]));
    register(&input, 0, &mut state);
    q.copy_from_slice(&register(&input, 1, &mut state));
}

pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let input = [u8::from(value)];
    let mut state = [stable_state(*q)];
    register(&input, 0, &mut state);
    *q = register(&input, 1, &mut state)[0];
}

pub(crate) fn read_bits(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

/// Packed stable-Q representation of the 163,840 main-memory DFFs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DffMemory40 {
    q: Vec<u64>,
}

impl DffMemory40 {
    pub(crate) fn new(words: usize) -> Self {
        Self { q: vec![0; words] }
    }

    pub(crate) fn snapshot(&self) -> Vec<u64> {
        self.q.clone()
    }

    pub(crate) fn restore_snapshot(&mut self, words: &[u64]) {
        for (address, value) in words.iter().copied().enumerate() {
            self.write(address, value);
        }
    }

    pub(crate) fn clear(&mut self) {
        for address in 0..self.q.len() {
            self.write(address, 0);
        }
    }

    pub(crate) fn read(&self, address: usize) -> u64 {
        self.q[address]
    }

    pub(crate) fn write(&mut self, address: usize, value: u64) {
        let old = self.q[address];
        let input: [u8; 40] = std::array::from_fn(|bit| ((value >> bit) & 1) as u8);
        let mut state: [FlipFlopState; 40] =
            std::array::from_fn(|bit| stable_state(((old >> bit) & 1) as u8));
        register(&input, 0, &mut state);
        self.q[address] = register(&input, 1, &mut state)
            .iter()
            .enumerate()
            .fold(0, |word, (bit, q)| word | (u64::from(*q) << bit));
    }
}
