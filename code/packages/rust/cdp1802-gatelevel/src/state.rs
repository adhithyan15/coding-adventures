//! Stable-Q storage built from repository D flip-flops.

use logic_gates::sequential::{register, FlipFlopState};

fn stable(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

pub(crate) fn clock_bits(q: &mut [u8], value: u64) {
    let input: Vec<u8> = (0..q.len()).map(|bit| ((value >> bit) & 1) as u8).collect();
    let mut state: Vec<FlipFlopState> = q.iter().copied().map(stable).collect();
    register(&input, 0, &mut state);
    q.copy_from_slice(&register(&input, 1, &mut state));
}

pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let mut state = [stable(*q)];
    register(&[u8::from(value)], 0, &mut state);
    *q = register(&[u8::from(value)], 1, &mut state)[0];
}

pub(crate) fn read_bits(q: &[u8]) -> u64 {
    q.iter()
        .enumerate()
        .fold(0, |value, (bit, wire)| value | (u64::from(*wire) << bit))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StateRegister {
    q: Vec<u8>,
}

impl StateRegister {
    pub(crate) fn new(width: usize) -> Self {
        Self { q: vec![0; width] }
    }

    pub(crate) fn read(&self) -> u64 {
        read_bits(&self.q)
    }

    pub(crate) fn clock(&mut self, value: u64) {
        clock_bits(&mut self.q, value);
    }

    pub(crate) fn dff_count(&self) -> usize {
        self.q.len()
    }
}

/// Packed byte view of 524,288 authoritative memory Q outputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DffMemory {
    q: Vec<u8>,
}

impl DffMemory {
    pub(crate) fn new(bytes: usize) -> Self {
        Self { q: vec![0; bytes] }
    }

    pub(crate) fn read(&self, address: usize) -> u8 {
        self.q[address]
    }

    pub(crate) fn write(&mut self, address: usize, value: u8) {
        let old = self.q[address];
        let input: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
        let mut states: [FlipFlopState; 8] = std::array::from_fn(|bit| stable((old >> bit) & 1));
        register(&input, 0, &mut states);
        self.q[address] = register(&input, 1, &mut states)
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }

    pub(crate) fn try_snapshot(&self) -> Result<Vec<u8>, ()> {
        let mut result = Vec::new();
        result.try_reserve_exact(self.q.len()).map_err(|_| ())?;
        result.extend_from_slice(&self.q);
        Ok(result)
    }

    pub(crate) fn restore(&mut self, bytes: &[u8]) {
        for (address, value) in bytes.iter().copied().enumerate() {
            if self.q[address] != value {
                self.write(address, value);
            }
        }
    }

    pub(crate) fn dff_count(&self) -> usize {
        self.q.len() * 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_memory_clock_stable_q() {
        let mut register = StateRegister::new(16);
        register.clock(0xa55a);
        assert_eq!(register.read(), 0xa55a);
        assert_eq!(register.dff_count(), 16);

        let mut memory = DffMemory::new(4);
        memory.write(2, 0xa5);
        assert_eq!(memory.read(2), 0xa5);
        memory.restore(&[0; 4]);
        assert_eq!(memory.snapshot(), vec![0; 4]);
    }
}
