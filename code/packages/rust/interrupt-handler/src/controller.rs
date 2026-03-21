//! Interrupt Controller — manages pending queue, masking, and dispatch.
//!
//! The interrupt controller is the central hub connecting hardware signals
//! to software handlers. It manages:
//!
//! 1. Pending queue: sorted list of raised interrupt numbers
//! 2. Mask register: 32-bit value blocking specific interrupts (0-31)
//! 3. Global enable flag: master switch for all interrupts
//! 4. Priority dispatch: lower number = higher priority

use crate::idt::InterruptDescriptorTable;
use crate::isr::ISRRegistry;

/// Manages the full interrupt lifecycle.
///
/// The lifecycle:
///   1. Device calls `raise_interrupt(number)`
///   2. Pipeline checks `has_pending()` between instructions
///   3. If pending: `next_pending()` returns highest-priority interrupt
///   4. CPU saves context, looks up IDT, dispatches ISR
///   5. After ISR: `acknowledge()` removes from pending
///   6. CPU restores context and resumes
pub struct InterruptController {
    /// The Interrupt Descriptor Table.
    pub idt: InterruptDescriptorTable,
    /// The ISR Registry mapping interrupt numbers to handlers.
    pub registry: ISRRegistry,
    /// Queue of pending interrupt numbers (sorted ascending).
    pending: Vec<usize>,
    /// 32-bit mask register: bit N = 1 means interrupt N is masked (0-31).
    pub mask_register: u32,
    /// Global interrupt enable flag.
    pub enabled: bool,
}

impl InterruptController {
    /// Create a controller with empty IDT, registry, and no pending.
    pub fn new() -> Self {
        Self {
            idt: InterruptDescriptorTable::new(),
            registry: ISRRegistry::new(),
            pending: Vec::new(),
            mask_register: 0,
            enabled: true,
        }
    }

    /// Add an interrupt to the pending queue.
    ///
    /// If already pending, it is not added again (no duplicates).
    /// The queue stays sorted ascending (lower = higher priority).
    pub fn raise_interrupt(&mut self, number: usize) {
        if self.pending.contains(&number) {
            return;
        }
        // Insert in sorted position
        let pos = self.pending.partition_point(|&n| n < number);
        self.pending.insert(pos, number);
    }

    /// Return `true` if any unmasked pending interrupts exist and enabled.
    pub fn has_pending(&self) -> bool {
        if !self.enabled {
            return false;
        }
        self.pending.iter().any(|&n| !self.is_masked(n))
    }

    /// Return highest-priority (lowest-numbered) unmasked pending interrupt.
    /// Returns `None` if none available or globally disabled.
    pub fn next_pending(&self) -> Option<usize> {
        if !self.enabled {
            return None;
        }
        self.pending.iter().copied().find(|&n| !self.is_masked(n))
    }

    /// Remove the given interrupt from the pending queue (EOI).
    pub fn acknowledge(&mut self, number: usize) {
        if let Some(pos) = self.pending.iter().position(|&n| n == number) {
            self.pending.remove(pos);
        }
    }

    /// Set or clear the mask for interrupt number (0-31 only).
    ///
    /// `masked = true` blocks; `masked = false` allows.
    /// Interrupts 32+ are not controlled by the mask register.
    pub fn set_mask(&mut self, number: usize, masked: bool) {
        if number > 31 {
            return; // only 0-31 are maskable
        }
        if masked {
            self.mask_register |= 1 << number;
        } else {
            self.mask_register &= !(1 << number);
        }
    }

    /// Return `true` if the interrupt is currently masked (blocked).
    ///
    /// Interrupts 32+ are never masked by the mask register.
    pub fn is_masked(&self, number: usize) -> bool {
        if number > 31 {
            return false;
        }
        (self.mask_register & (1 << number)) != 0
    }

    /// Set the global interrupt enable flag.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Clear the global interrupt enable flag.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Return the number of pending interrupts (masked and unmasked).
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Remove all pending interrupts.
    pub fn clear_all(&mut self) {
        self.pending.clear();
    }
}

impl Default for InterruptController {
    fn default() -> Self {
        Self::new()
    }
}
