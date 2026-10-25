//! Stable-Q storage for the EDSAC's fixed-width architectural state.
//!
//! EDSAC's mercury store is asymmetric: every even location includes the
//! sandwich digit and is eighteen bits wide, while every odd location is only
//! seventeen bits. Keeping the Q bus packed by even/odd pair makes that
//! historical detail visible instead of pretending the machine had a modern
//! uniform word array.

use edsac_simulator::MEMORY_CELLS;
use logic_gates::sequential::{register, FlipFlopState};

const BITS_PER_PAIR: usize = 35;
pub(crate) const MEMORY_DFFS: usize = MEMORY_CELLS / 2 * BITS_PER_PAIR;

fn stable_state(q: u8) -> FlipFlopState {
    FlipFlopState {
        master_q: q,
        master_q_bar: q ^ 1,
        slave_q: q,
        slave_q_bar: q ^ 1,
    }
}

pub(crate) fn clock_bits(q: &mut [u8], value: u128) {
    let input = (0..q.len())
        .map(|bit| ((value >> bit) & 1) as u8)
        .collect::<Vec<_>>();
    let mut state = q.iter().copied().map(stable_state).collect::<Vec<_>>();
    register(&input, 0, &mut state);
    q.copy_from_slice(&register(&input, 1, &mut state));
}

pub(crate) fn bits_value(q: &[u8]) -> u128 {
    q.iter()
        .enumerate()
        .fold(0_u128, |value, (bit, q)| value | (u128::from(*q) << bit))
}

pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    clock_bits(std::slice::from_mut(q), u128::from(value));
}

/// One packed stable-Q bit for every physical short-store digit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DffStore {
    q: Vec<u8>,
}

impl DffStore {
    pub(crate) fn new() -> Self {
        Self {
            q: vec![0; MEMORY_DFFS],
        }
    }

    fn range(address: usize) -> Option<std::ops::Range<usize>> {
        if address >= MEMORY_CELLS {
            return None;
        }
        let pair = address / 2;
        let start = pair * BITS_PER_PAIR + if address.is_multiple_of(2) { 0 } else { 18 };
        let width = if address.is_multiple_of(2) { 18 } else { 17 };
        Some(start..start + width)
    }

    pub(crate) fn read(&self, address: usize) -> Option<u32> {
        Self::range(address).map(|range| bits_value(&self.q[range]) as u32)
    }

    pub(crate) fn write(&mut self, address: usize, value: u32) -> bool {
        let Some(range) = Self::range(address) else {
            return false;
        };
        clock_bits(&mut self.q[range], u128::from(value));
        true
    }

    pub(crate) fn clear(&mut self) {
        self.q.fill(0);
    }

    pub(crate) fn snapshot(&self) -> Box<[u32; MEMORY_CELLS]> {
        Box::new(std::array::from_fn(|address| {
            self.read(address).expect("snapshot address is in range")
        }))
    }
}

pub(crate) fn clock_accumulator(q: &mut [u8; 71], value: [u16; 5]) {
    for slice in 0..4 {
        clock_bits(
            &mut q[slice * 15..slice * 15 + 15],
            u128::from(value[slice]),
        );
    }
    clock_bits(&mut q[60..71], u128::from(value[4] >> 4));
}

pub(crate) fn accumulator_value(q: &[u8; 71]) -> [u16; 5] {
    let mut value = [0_u16; 5];
    for slice in 0..4 {
        value[slice] = bits_value(&q[slice * 15..slice * 15 + 15]) as u16;
    }
    value[4] = (bits_value(&q[60..71]) as u16) << 4;
    value
}

pub(crate) fn clock_operand(q: &mut [u8; 35], value: [u16; 3]) {
    clock_bits(&mut q[0..15], u128::from(value[0]));
    clock_bits(&mut q[15..30], u128::from(value[1]));
    clock_bits(&mut q[30..35], u128::from(value[2] >> 10));
}

pub(crate) fn operand_value(q: &[u8; 35]) -> [u16; 3] {
    [
        bits_value(&q[0..15]) as u16,
        bits_value(&q[15..30]) as u16,
        (bits_value(&q[30..35]) as u16) << 10,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asymmetric_store_and_unpadded_registers_round_trip() {
        let mut store = DffStore::new();
        assert!(store.write(0, 0x3_ffff));
        assert!(store.write(1, 0x1_ffff));
        assert_eq!(store.read(0), Some(0x3_ffff));
        assert_eq!(store.read(1), Some(0x1_ffff));
        assert_eq!(store.q.len(), 17_920);

        let mut accumulator = [0; 71];
        let tank = [1, 2, 3, 4, 0x7ff0];
        clock_accumulator(&mut accumulator, tank);
        assert_eq!(accumulator_value(&accumulator), tank);

        let mut operand = [0; 35];
        let tank = [1, 2, 0x7c00];
        clock_operand(&mut operand, tank);
        assert_eq!(operand_value(&operand), tank);
    }
}
