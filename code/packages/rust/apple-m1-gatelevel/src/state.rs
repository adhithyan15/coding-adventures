use logic_gates::sequential::{register, FlipFlopState};

fn stable_state(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

pub(crate) fn word64(bits: &[u8; 64]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

pub(crate) fn word128(bits: &[u8; 128]) -> u128 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u128::from(*q) << bit))
}

fn clock_slice(q: &mut [u8], input: &[u8]) {
    let mut state: Vec<_> = q.iter().copied().map(stable_state).collect();
    register(input, 0, &mut state);
    q.copy_from_slice(&register(input, 1, &mut state));
}

pub(crate) fn clock_word64(q: &mut [u8; 64], value: u64) {
    let input: [u8; 64] = std::array::from_fn(|bit| ((value >> bit) & 1) as u8);
    clock_slice(q, &input);
}

pub(crate) fn clock_word128(q: &mut [u8; 128], value: u128) {
    let input: [u8; 128] = std::array::from_fn(|bit| ((value >> bit) & 1) as u8);
    clock_slice(q, &input);
}

pub(crate) fn clock_bits(q: &mut [u8], value: u8) {
    let input: Vec<_> = (0..q.len()).map(|bit| (value >> bit) & 1).collect();
    clock_slice(q, &input);
}

pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    clock_bits(std::slice::from_mut(q), u8::from(value));
}

#[derive(Debug, Clone)]
pub(crate) struct DffMemory {
    q: Vec<u8>,
}

impl DffMemory {
    pub(crate) fn new() -> Self {
        Self { q: vec![0; 65_536] }
    }

    pub(crate) fn read(&self, address: u64) -> Option<u8> {
        self.q.get(address as usize).copied()
    }

    pub(crate) fn write(&mut self, address: u64, value: u8) -> bool {
        let Some(old) = self.q.get(address as usize).copied() else {
            return false;
        };
        let input: Vec<_> = (0..8).map(|bit| (value >> bit) & 1).collect();
        let mut bits: Vec<_> = (0..8).map(|bit| stable_state((old >> bit) & 1)).collect();
        register(&input, 0, &mut bits);
        self.q[address as usize] = register(&input, 1, &mut bits)
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));
        true
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }

    pub(crate) fn restore(&mut self, bytes: &[u8]) {
        self.q.copy_from_slice(bytes);
    }
}
