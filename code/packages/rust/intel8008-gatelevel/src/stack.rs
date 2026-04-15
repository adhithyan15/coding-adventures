//! 8-level push-down stack for the Intel 8008.
//!
//! # Hardware model
//!
//! The 8008 stack is 8 × 14-bit registers in a circular push-down arrangement.
//! This is fundamentally different from the software-managed stack pointer used
//! by most modern CPUs.
//!
//! ```text
//! Stack organization:
//!   slot[0] = Program Counter (always the live PC)
//!   slot[1] = saved return address for current call frame
//!   slot[2] = saved return address for outer call frame
//!   ...
//!   slot[7] = oldest saved return address
//! ```
//!
//! **CALL operation (push):**
//! The stack rotates DOWN: slot[7] ← slot[6] ← ... ← slot[1] ← slot[0].
//! Then the jump target is loaded into slot[0] (the new PC).
//! The old slot[0] value (return address after the CALL instruction) is now
//! preserved in slot[1].
//!
//! **RETURN operation (pop):**
//! The stack rotates UP: slot[0] ← slot[1] ← slot[2] ← ... ← slot[6] ← slot[7].
//! slot[7] is zeroed. The value that was in slot[1] is now in slot[0] (the PC),
//! restoring execution to the return address.
//!
//! **Overflow (> 7 nested calls):**
//! The oldest return address in slot[7] is silently overwritten. Programs
//! must ensure at most 7 levels of nesting.
//!
//! # Gate count
//!
//! Each of the 8 × 14-bit registers uses 14 flip-flops = 14 × 6 gates = 84 gates.
//! Total for all 8 slots: 8 × 84 = 672 gates.
//! Compare: the 4004's 3-level × 12-bit stack = 3 × 12 × 6 = 216 gates.
//! The 8008's stack requires 3× more gates than the 4004's.
//!
//! # Implementation note
//!
//! Each slot is stored as a `Vec<FlipFlopState>` of 14 elements (LSB-first).
//! The `register(data, clock, state)` function from `logic_gates` simulates
//! the two-phase D flip-flop clock cycle.

use logic_gates::sequential::{register, FlipFlopState};

/// 8-level push-down stack storing 14-bit return addresses.
///
/// Slot 0 is always the current program counter. Slots 1-7 hold saved
/// return addresses in LIFO order.
pub struct PushDownStack {
    /// 8 slots of 14 flip-flops each (LSB-first).
    slots: Vec<Vec<FlipFlopState>>,
    depth: usize,
}

impl PushDownStack {
    /// Create a new stack with all slots zeroed and depth 0.
    pub fn new() -> Self {
        let slots: Vec<Vec<FlipFlopState>> = (0..8)
            .map(|_| (0..14).map(|_| FlipFlopState::default()).collect())
            .collect();
        PushDownStack { slots, depth: 0 }
    }

    /// Read the current PC (always slot 0).
    pub fn pc(&self) -> u16 {
        Self::slot_to_u16(&self.slots[0])
    }

    /// Write a new value to the PC (slot 0).
    pub fn set_pc(&mut self, value: u16) {
        Self::u16_to_slot(value & 0x3FFF, &mut self.slots[0]);
    }

    /// How many CALL frames are currently active (0-7).
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// CALL — save current PC, rotate stack down, load jump target.
    ///
    /// After this operation:
    /// - slot[0] = target (new PC)
    /// - slot[1] = old slot[0] (the saved return address)
    /// - slot[2..7] each hold the value from one position up
    pub fn push_and_jump(&mut self, target: u16) {
        // Rotate: slot[7] ← slot[6] ← ... ← slot[1] ← slot[0]
        // (copy each slot's bit states up the stack)
        for i in (1..8).rev() {
            let prev = slot_to_bits(&self.slots[i - 1]);
            write_bits_to_slot(&prev, &mut self.slots[i]);
        }
        // Load target into PC slot
        Self::u16_to_slot(target & 0x3FFF, &mut self.slots[0]);
        if self.depth < 7 {
            self.depth += 1;
        }
    }

    /// RETURN — rotate stack up, restoring the saved return address.
    ///
    /// After this operation:
    /// - slot[0] = old slot[1] (saved return address becomes new PC)
    /// - slot[7] = zeroed
    pub fn pop_return(&mut self) {
        // Rotate: slot[0] ← slot[1] ← ... ← slot[6] ← slot[7]
        for i in 0..7 {
            let next = slot_to_bits(&self.slots[i + 1]);
            write_bits_to_slot(&next, &mut self.slots[i]);
        }
        // Clear the now-orphaned oldest slot
        let zeros = vec![0u8; 14];
        write_bits_to_slot(&zeros, &mut self.slots[7]);
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    // -------------------------------------------------------------------------
    // Helpers for converting between u16 and 14-bit flip-flop slots
    // -------------------------------------------------------------------------

    fn slot_to_u16(slot: &[FlipFlopState]) -> u16 {
        slot.iter().enumerate().fold(0u16, |acc, (i, s)| {
            acc | ((s.slave_q as u16) << i)
        })
    }

    fn u16_to_slot(value: u16, slot: &mut Vec<FlipFlopState>) {
        let bits: Vec<u8> = (0..14).map(|i| ((value >> i) & 1) as u8).collect();
        register(&bits, 0, slot);
        register(&bits, 1, slot);
    }
}

/// Read a slot's current bit values (from slave_q outputs).
fn slot_to_bits(slot: &[FlipFlopState]) -> Vec<u8> {
    slot.iter().map(|s| s.slave_q).collect()
}

/// Write a bit vector into a slot via two-phase clock.
fn write_bits_to_slot(bits: &[u8], slot: &mut Vec<FlipFlopState>) {
    register(bits, 0, slot);
    register(bits, 1, slot);
}

impl Default for PushDownStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_push_pop() {
        let mut stack = PushDownStack::new();
        stack.set_pc(5); // simulate PC after a CALL instruction
        assert_eq!(stack.pc(), 5);
        assert_eq!(stack.depth(), 0);

        // CALL to 0x100
        stack.push_and_jump(0x100);
        assert_eq!(stack.pc(), 0x100);
        assert_eq!(stack.depth(), 1);

        // RETURN
        stack.pop_return();
        assert_eq!(stack.pc(), 5);
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_stack_nesting() {
        let mut stack = PushDownStack::new();
        stack.set_pc(0x10); // return address after main CALL
        stack.push_and_jump(0x100); // call f1
        stack.push_and_jump(0x200); // call f2 from f1
        assert_eq!(stack.depth(), 2);
        assert_eq!(stack.pc(), 0x200);
        stack.pop_return(); // return from f2
        assert_eq!(stack.pc(), 0x100);
        stack.pop_return();
        assert_eq!(stack.pc(), 0x10);
        assert_eq!(stack.depth(), 0);
    }
}
