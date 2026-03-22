//! Hardware call stack -- 3 levels of 12-bit return addresses.
//!
//! # The 4004's stack
//!
//! The Intel 4004 has a 3-level hardware call stack. This is NOT a
//! software stack in RAM -- it's three physical 12-bit registers plus
//! a 2-bit circular pointer, all built from D flip-flops.
//!
//! Why only 3 levels? The 4004 was designed for calculators, which had
//! simple call structures. Three levels of subroutine nesting was enough
//! for the Busicom 141-PF calculator's firmware.
//!
//! # Silent overflow
//!
//! When you push a 4th address, the stack wraps silently -- the oldest
//! return address is overwritten. There is no stack overflow exception.
//! This matches the real hardware behavior. The 4004's designers saved
//! transistors by not including overflow detection.

use logic_gates::sequential::{register, FlipFlopState};

use crate::bits::{bits_to_int, int_to_bits};

/// 3-level x 12-bit hardware call stack.
///
/// Built from 3 x 12 = 36 D flip-flops for storage, plus a 2-bit
/// pointer that wraps modulo 3.
pub struct HardwareStack {
    /// Three 12-bit register states.
    levels: Vec<Vec<FlipFlopState>>,
    /// Current stack pointer (0, 1, or 2).
    pointer: usize,
}

impl HardwareStack {
    /// Initialize stack with 3 empty slots and pointer at 0.
    pub fn new() -> Self {
        let mut levels = Vec::with_capacity(3);
        for _ in 0..3 {
            let mut state: Vec<FlipFlopState> =
                (0..12).map(|_| FlipFlopState::default()).collect();
            register(&[0; 12], 0, &mut state);
            register(&[0; 12], 1, &mut state);
            levels.push(state);
        }
        Self {
            levels,
            pointer: 0,
        }
    }

    /// Push a return address. Wraps silently on overflow.
    ///
    /// In real hardware: the pointer selects which of the 3 registers
    /// to write, then the pointer increments mod 3.
    pub fn push(&mut self, address: u16) {
        let bits = int_to_bits(address & 0xFFF, 12);
        register(&bits, 0, &mut self.levels[self.pointer]);
        register(&bits, 1, &mut self.levels[self.pointer]);
        self.pointer = (self.pointer + 1) % 3;
    }

    /// Pop and return the top address.
    ///
    /// Decrements pointer mod 3, then reads that register.
    pub fn pop(&mut self) -> u16 {
        self.pointer = (self.pointer + 3 - 1) % 3;
        let mut state = self.levels[self.pointer].clone();
        let output = register(&[0; 12], 0, &mut state);
        bits_to_int(&output)
    }

    /// Reset all stack levels to 0 and pointer to 0.
    pub fn reset(&mut self) {
        for i in 0..3 {
            register(&[0; 12], 0, &mut self.levels[i]);
            register(&[0; 12], 1, &mut self.levels[i]);
        }
        self.pointer = 0;
    }

    /// Current pointer position (not true depth, since we wrap).
    pub fn depth(&self) -> usize {
        self.pointer
    }

    /// Read all stack level values (for inspection only).
    pub fn read_levels(&self) -> Vec<u16> {
        let mut values = Vec::with_capacity(3);
        for i in 0..3 {
            let mut state = self.levels[i].clone();
            let output = register(&[0; 12], 0, &mut state);
            values.push(bits_to_int(&output));
        }
        values
    }

    /// 3 x 12-bit registers (216 gates) + pointer logic (~10 gates).
    pub fn gate_count(&self) -> usize {
        226
    }
}

impl Default for HardwareStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let mut stack = HardwareStack::new();
        stack.push(0x123);
        assert_eq!(stack.pop(), 0x123);
    }

    #[test]
    fn test_push_pop_multiple() {
        let mut stack = HardwareStack::new();
        stack.push(0x100);
        stack.push(0x200);
        stack.push(0x300);
        assert_eq!(stack.pop(), 0x300);
        assert_eq!(stack.pop(), 0x200);
        assert_eq!(stack.pop(), 0x100);
    }

    #[test]
    fn test_stack_wraps() {
        let mut stack = HardwareStack::new();
        stack.push(0x100);
        stack.push(0x200);
        stack.push(0x300);
        stack.push(0x400); // Overwrites level 0 (0x100)
        assert_eq!(stack.pop(), 0x400);
        assert_eq!(stack.pop(), 0x300);
        assert_eq!(stack.pop(), 0x200);
    }

    #[test]
    fn test_stack_reset() {
        let mut stack = HardwareStack::new();
        stack.push(0x123);
        stack.reset();
        assert_eq!(stack.depth(), 0);
    }
}
