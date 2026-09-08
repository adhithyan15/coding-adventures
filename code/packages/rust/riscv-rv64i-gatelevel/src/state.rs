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

pub(crate) fn bits64(value: u64) -> [u8; 64] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

pub(crate) fn word64(bits: &[u8; 64]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))
}

pub(crate) fn clock_word(q: &mut [u8; 64], value: u64) {
    let input = bits64(value);
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

    pub(crate) fn read(&self, address: u64) -> Option<u8> {
        self.q.get(address as usize).copied()
    }

    pub(crate) fn write(&mut self, address: u64, value: u8) -> bool {
        let Some(old) = self.q.get(address as usize).copied() else {
            return false;
        };
        let input: Vec<_> = (0..8).map(|bit| (value >> bit) & 1).collect();
        let mut state: Vec<_> = (0..8).map(|bit| stable_state((old >> bit) & 1)).collect();
        register(&input, 0, &mut state);
        let output = register(&input, 1, &mut state);
        self.q[address as usize] = output
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));
        true
    }

    pub(crate) fn clear(&mut self) {
        for address in 0..self.q.len() {
            if self.q[address] != 0 {
                let wrote = self.write(address as u64, 0);
                debug_assert!(wrote);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clocks_words_bits_and_checked_memory() {
        let mut word = [0; 64];
        clock_word(&mut word, 0x0123_4567_89ab_cdef);
        assert_eq!(word64(&word), 0x0123_4567_89ab_cdef);

        let mut bit = 0;
        clock_bit(&mut bit, true);
        assert_eq!(bit, 1);

        let mut memory = DffMemory::new();
        assert!(memory.write(0xffff, 0xa5));
        assert_eq!(memory.read(0xffff), Some(0xa5));
        assert!(!memory.write(0x1_0000, 1));
        assert_eq!(memory.read(0x1_0000), None);
        memory.clear();
        assert_eq!(memory.read(0xffff), Some(0));
    }
}
