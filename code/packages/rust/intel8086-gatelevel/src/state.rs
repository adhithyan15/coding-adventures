//! D-flip-flop-backed persistent storage for the complete 8086 machine.

use logic_gates::sequential::{register, FlipFlopState};

const MEMORY_BYTES: usize = 1 << 20;

#[derive(Debug, Clone)]
pub(crate) struct StateRegister {
    state: Vec<FlipFlopState>,
}

impl StateRegister {
    pub(crate) fn new(width: usize) -> Self {
        Self {
            state: vec![FlipFlopState::default(); width],
        }
    }

    pub(crate) fn write(&mut self, value: u16) {
        let bits: Vec<u8> = (0..self.state.len())
            .map(|bit| ((value >> bit) & 1) as u8)
            .collect();
        register(&bits, 0, &mut self.state);
        register(&bits, 1, &mut self.state);
    }

    #[cfg(test)]
    pub(crate) fn read(&self) -> u16 {
        self.state
            .iter()
            .enumerate()
            .fold(0, |value, (bit, state)| {
                value | (u16::from(state.slave_q) << bit)
            })
    }
}

#[derive(Clone)]
pub(crate) struct DffMemory {
    state: Vec<FlipFlopState>,
    // Cached Q-bus values make full-state hashing O(bytes), while `state`
    // remains the persistent implementation and every write clocks both DFF
    // phases before refreshing this output-wire cache.
    q_cache: Box<[u8; MEMORY_BYTES]>,
}

impl DffMemory {
    pub(crate) const BYTE_LEN: usize = MEMORY_BYTES;
    pub(crate) const DFF_COUNT: usize = Self::BYTE_LEN * 8;

    pub(crate) fn new() -> Self {
        let q_cache = vec![0u8; Self::BYTE_LEN]
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| unreachable!("fixed 8086 memory size"));
        Self {
            state: vec![FlipFlopState::default(); Self::DFF_COUNT],
            q_cache,
        }
    }

    pub(crate) fn read(&self, address: usize) -> u8 {
        self.q_cache[address]
    }

    pub(crate) fn write(&mut self, address: usize, value: u8) {
        let start = address * 8;
        let bits: Vec<u8> = (0..8).map(|bit| (value >> bit) & 1).collect();
        register(&bits, 0, &mut self.state[start..start + 8]);
        register(&bits, 1, &mut self.state[start..start + 8]);
        self.q_cache[address] = value;
    }

    pub(crate) fn copy_from_slice(&mut self, origin: usize, bytes: &[u8]) {
        for (offset, byte) in bytes.iter().copied().enumerate() {
            self.write(origin + offset, byte);
        }
    }

    pub(crate) fn snapshot(&self) -> Box<[u8]> {
        self.q_cache.to_vec().into_boxed_slice()
    }
}
