//! Packed stable-Q storage backed by the repository D-flip-flop primitive.

use logic_gates::sequential::{register, FlipFlopState};

fn stable_state(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

pub(crate) fn bits32(value: u32) -> [u8; 32] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

pub(crate) fn word32(bits: &[u8; 32]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u32::from(*q) << bit))
}

pub(crate) fn clock_word(q: &mut [u8; 32], value: u32) {
    let input = bits32(value);
    let mut state: Vec<_> = q.iter().copied().map(stable_state).collect();
    register(&input, 0, &mut state);
    q.copy_from_slice(&register(&input, 1, &mut state));
}

pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let input = [u8::from(value)];
    let mut state = [stable_state(*q)];
    register(&input, 0, &mut state);
    *q = register(&input, 1, &mut state)[0];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DffMemory {
    q: Vec<u8>,
}

impl DffMemory {
    pub(crate) fn new() -> Self {
        Self { q: vec![0; 65_536] }
    }

    pub(crate) fn read(&self, address: u32) -> u8 {
        self.q[address as u16 as usize]
    }

    pub(crate) fn write(&mut self, address: u32, value: u8) {
        let index = address as u16 as usize;
        let old = self.q[index];
        let input: Vec<_> = (0..8).map(|bit| (value >> bit) & 1).collect();
        let mut state: Vec<_> = (0..8).map(|bit| stable_state((old >> bit) & 1)).collect();
        register(&input, 0, &mut state);
        let output = register(&input, 1, &mut state);
        self.q[index] = output
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));
    }

    pub(crate) fn clear(&mut self) {
        for address in 0..self.q.len() {
            if self.q[address] != 0 {
                self.write(address as u32, 0);
            }
        }
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }

    pub(crate) fn restore_snapshot(&mut self, bytes: &[u8]) {
        self.q.copy_from_slice(bytes);
    }
}
