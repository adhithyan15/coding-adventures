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

/// Packed stable-Q representation of the main-storage D flip-flops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DffMemory {
    q: Vec<u8>,
}

impl DffMemory {
    pub(crate) fn new(byte_len: usize) -> Self {
        Self::try_new(byte_len).expect("architectural memory allocation failed")
    }

    pub(crate) fn try_new(byte_len: usize) -> Result<Self, ()> {
        let mut q = Vec::new();
        q.try_reserve_exact(byte_len).map_err(|_| ())?;
        q.resize(byte_len, 0);
        Ok(Self { q })
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }

    pub(crate) fn try_snapshot(&self) -> Result<Vec<u8>, ()> {
        let mut snapshot = Vec::new();
        snapshot.try_reserve_exact(self.q.len()).map_err(|_| ())?;
        snapshot.extend_from_slice(&self.q);
        Ok(snapshot)
    }

    pub(crate) fn restore_snapshot(&mut self, bytes: &[u8]) {
        self.q.copy_from_slice(bytes);
    }

    pub(crate) fn clear(&mut self) {
        self.q.fill(0);
    }

    pub(crate) fn read(&self, address: usize) -> u8 {
        self.q[address]
    }

    pub(crate) fn write(&mut self, address: usize, value: u8) {
        let old = self.q[address];
        let input: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
        let mut state: [FlipFlopState; 8] =
            std::array::from_fn(|bit| stable_state((old >> bit) & 1));
        register(&input, 0, &mut state);
        self.q[address] = register(&input, 1, &mut state)
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));
    }
}
