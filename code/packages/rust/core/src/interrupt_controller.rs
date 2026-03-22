//! InterruptController -- routes interrupts to cores.
//!
//! # What are Interrupts?
//!
//! An interrupt is a signal that temporarily diverts the CPU from its current
//! work to handle an urgent event. Examples:
//!
//!   - Timer interrupt: "100ms have passed, let the OS scheduler run"
//!   - I/O interrupt: "keyboard key was pressed" or "network packet arrived"
//!   - Inter-processor interrupt (IPI): "Core 0 needs Core 1 to flush its TLB"
//!   - Software interrupt: "this program wants to make a system call"
//!
//! # How the Controller Works
//!
//! The interrupt controller is the traffic cop for interrupts:
//!
//!  1. An external device (or another core) raises an interrupt.
//!  2. The controller queues it and decides which core should handle it.
//!  3. On the next cycle, the controller signals the target core.
//!  4. The core acknowledges the interrupt and begins handling it.
//!
//! This implementation is a simplified shell -- it queues interrupts and
//! routes them, but does not model priorities or masking.

/// An interrupt waiting to be delivered.
#[derive(Debug, Clone)]
pub struct PendingInterrupt {
    /// Identifies the interrupt source (e.g., timer=0, keyboard=1).
    pub interrupt_id: usize,

    /// Which core should handle it.
    /// usize::MAX means "route to any available core".
    pub target_core: usize,
}

/// Records a core acknowledging an interrupt.
#[derive(Debug, Clone)]
pub struct AcknowledgedInterrupt {
    pub core_id: usize,
    pub interrupt_id: usize,
}

/// Manages interrupt routing in a multi-core system.
pub struct InterruptController {
    /// Queued interrupts waiting to be delivered.
    pending: Vec<PendingInterrupt>,

    /// Interrupts that have been acknowledged.
    acknowledged: Vec<AcknowledgedInterrupt>,

    /// Total number of cores in the system.
    num_cores: usize,
}

impl InterruptController {
    /// Creates an interrupt controller for the given number of cores.
    pub fn new(num_cores: usize) -> Self {
        Self {
            pending: Vec::new(),
            acknowledged: Vec::new(),
            num_cores,
        }
    }

    /// Queues an interrupt for delivery.
    ///
    /// If `target_core` is `usize::MAX`, the interrupt is routed to core 0
    /// (simplest routing policy).
    pub fn raise_interrupt(&mut self, interrupt_id: usize, target_core: usize) {
        let target = if target_core == usize::MAX || target_core >= self.num_cores {
            0
        } else {
            target_core
        };
        self.pending.push(PendingInterrupt {
            interrupt_id,
            target_core: target,
        });
    }

    /// Records that a core has begun handling an interrupt.
    ///
    /// Removes the first matching interrupt from the pending queue.
    pub fn acknowledge(&mut self, core_id: usize, interrupt_id: usize) {
        self.acknowledged.push(AcknowledgedInterrupt {
            core_id,
            interrupt_id,
        });

        // Remove first matching from pending.
        if let Some(pos) = self.pending.iter().position(|p| {
            p.interrupt_id == interrupt_id && p.target_core == core_id
        }) {
            self.pending.remove(pos);
        }
    }

    /// Returns all pending interrupts targeted at a specific core.
    pub fn pending_for_core(&self, core_id: usize) -> Vec<PendingInterrupt> {
        self.pending
            .iter()
            .filter(|p| p.target_core == core_id)
            .cloned()
            .collect()
    }

    /// Returns the total number of pending (unacknowledged) interrupts.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Returns the total number of acknowledged interrupts.
    pub fn acknowledged_count(&self) -> usize {
        self.acknowledged.len()
    }

    /// Clears all pending and acknowledged interrupts.
    pub fn reset(&mut self) {
        self.pending.clear();
        self.acknowledged.clear();
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_interrupt_controller() {
        let ic = InterruptController::new(4);
        assert_eq!(ic.pending_count(), 0);
        assert_eq!(ic.acknowledged_count(), 0);
    }

    #[test]
    fn test_raise_interrupt() {
        let mut ic = InterruptController::new(4);
        ic.raise_interrupt(0, 1);
        assert_eq!(ic.pending_count(), 1);
        let pending = ic.pending_for_core(1);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].interrupt_id, 0);
    }

    #[test]
    fn test_raise_interrupt_default_routing() {
        let mut ic = InterruptController::new(4);
        ic.raise_interrupt(0, usize::MAX);
        assert_eq!(ic.pending_count(), 1);
        let pending = ic.pending_for_core(0);
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_raise_interrupt_out_of_range_core() {
        let mut ic = InterruptController::new(4);
        ic.raise_interrupt(0, 100);
        let pending = ic.pending_for_core(0);
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_acknowledge_interrupt() {
        let mut ic = InterruptController::new(4);
        ic.raise_interrupt(0, 1);
        ic.acknowledge(1, 0);
        assert_eq!(ic.pending_count(), 0);
        assert_eq!(ic.acknowledged_count(), 1);
    }

    #[test]
    fn test_pending_for_core_filters() {
        let mut ic = InterruptController::new(4);
        ic.raise_interrupt(0, 0);
        ic.raise_interrupt(1, 1);
        ic.raise_interrupt(2, 0);
        let pending_core0 = ic.pending_for_core(0);
        assert_eq!(pending_core0.len(), 2);
        let pending_core1 = ic.pending_for_core(1);
        assert_eq!(pending_core1.len(), 1);
    }

    #[test]
    fn test_reset() {
        let mut ic = InterruptController::new(4);
        ic.raise_interrupt(0, 0);
        ic.acknowledge(0, 0);
        ic.raise_interrupt(1, 1);
        ic.reset();
        assert_eq!(ic.pending_count(), 0);
        assert_eq!(ic.acknowledged_count(), 0);
    }
}
