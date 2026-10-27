//! Stable-Q architectural storage clocked through repository D flip-flops.

use logic_gates::sequential::{register, FlipFlopState};
use std::collections::BTreeSet;

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

/// Packed stable-Q representation of the 64 KiB memory D flip-flops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DffMemory {
    q: Vec<u8>,
    // This is derived host metadata, not architectural storage. It keeps reset
    // proportional to changed bytes while every selected byte is still
    // clocked through the repository DFF primitive.
    nonzero_addresses: BTreeSet<usize>,
}

impl DffMemory {
    pub(crate) fn new(byte_len: usize) -> Self {
        Self {
            q: vec![0; byte_len],
            nonzero_addresses: BTreeSet::new(),
        }
    }

    pub(crate) fn bit_len(&self) -> usize {
        self.q.len() * 8
    }

    pub(crate) fn snapshot(&self) -> Vec<u8> {
        self.q.clone()
    }

    pub(crate) fn try_snapshot(&self) -> Result<Vec<u8>, ()> {
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(self.q.len()).map_err(|_| ())?;
        bytes.extend_from_slice(&self.q);
        Ok(bytes)
    }

    pub(crate) fn snapshot_into(&self, bytes: &mut Vec<u8>) {
        bytes.clear();
        bytes.extend_from_slice(&self.q);
    }

    pub(crate) fn restore_snapshot(&mut self, bytes: &[u8]) {
        for (address, value) in bytes.iter().copied().enumerate() {
            if self.q[address] != value && crate::gates::byte_changed(self.q[address], value) == 1 {
                self.write(address, value);
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        let selected: Vec<usize> = self.nonzero_addresses.iter().copied().collect();
        for address in selected {
            self.write(address, 0);
        }
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
        if crate::gates::is_zero_8(self.q[address]) == 1 {
            self.nonzero_addresses.remove(&address);
        } else {
            self.nonzero_addresses.insert(address);
        }
    }
}
