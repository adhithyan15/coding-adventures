package core

// =========================================================================
// InterruptController -- routes interrupts to cores
// =========================================================================

// InterruptController manages interrupt routing in a multi-core system.
//
// # What are Interrupts?
//
// An interrupt is a signal that temporarily diverts the CPU from its current
// work to handle an urgent event. Examples:
//
//   - Timer interrupt: "100ms have passed, let the OS scheduler run"
//   - I/O interrupt: "keyboard key was pressed" or "network packet arrived"
//   - Inter-processor interrupt (IPI): "Core 0 needs Core 1 to flush its TLB"
//   - Software interrupt: "this program wants to make a system call"
//
// # How the Controller Works
//
// The interrupt controller is the traffic cop for interrupts:
//
//  1. An external device (or another core) raises an interrupt.
//  2. The controller queues it and decides which core should handle it.
//  3. On the next cycle, the controller signals the target core.
//  4. The core acknowledges the interrupt and begins handling it.
//
// In real hardware, interrupt controllers are sophisticated:
//   - ARM GIC (Generic Interrupt Controller): prioritized, masked, routable
//   - x86 APIC (Advanced Programmable Interrupt Controller): similar
//
// This implementation is a simplified shell -- it queues interrupts and
// routes them, but does not model priorities or masking.
//
// # Future Extensions
//
//   - Priority levels (higher-priority interrupts preempt lower ones)
//   - Interrupt masking (cores can temporarily ignore certain interrupts)
//   - Nested interrupts (handling one interrupt while another arrives)
//   - Interrupt affinity (bind certain interrupts to specific cores)
type InterruptController struct {
	// pending holds queued interrupts waiting to be delivered.
	pending []PendingInterrupt

	// acknowledged tracks which interrupts have been acknowledged.
	acknowledged []AcknowledgedInterrupt

	// numCores is the total number of cores in the system.
	numCores int
}

// PendingInterrupt represents an interrupt waiting to be delivered.
type PendingInterrupt struct {
	// InterruptID identifies the interrupt source (e.g., timer=0, keyboard=1).
	InterruptID int

	// TargetCore specifies which core should handle it.
	// -1 means "route to any available core" (the controller picks one).
	TargetCore int
}

// AcknowledgedInterrupt records a core acknowledging an interrupt.
type AcknowledgedInterrupt struct {
	CoreID      int
	InterruptID int
}

// NewInterruptController creates an interrupt controller for the given
// number of cores.
func NewInterruptController(numCores int) *InterruptController {
	return &InterruptController{
		numCores: numCores,
	}
}

// RaiseInterrupt queues an interrupt for delivery.
//
// If targetCore is -1, the interrupt will be routed to core 0 (simplest
// routing policy -- round-robin or load-based routing is a future extension).
func (ic *InterruptController) RaiseInterrupt(interruptID int, targetCore int) {
	if targetCore == -1 {
		targetCore = 0 // default: route to core 0
	}
	if targetCore >= ic.numCores {
		targetCore = 0
	}
	ic.pending = append(ic.pending, PendingInterrupt{
		InterruptID: interruptID,
		TargetCore:  targetCore,
	})
}

// Acknowledge records that a core has begun handling an interrupt.
//
// In real hardware, acknowledgment tells the interrupt controller that the
// core has received the signal and started executing the interrupt handler.
// The controller can then clear the pending flag and potentially deliver
// the next interrupt.
func (ic *InterruptController) Acknowledge(coreID int, interruptID int) {
	ic.acknowledged = append(ic.acknowledged, AcknowledgedInterrupt{
		CoreID:      coreID,
		InterruptID: interruptID,
	})

	// Remove from pending.
	remaining := ic.pending[:0]
	removed := false
	for _, p := range ic.pending {
		if !removed && p.InterruptID == interruptID && p.TargetCore == coreID {
			removed = true
			continue
		}
		remaining = append(remaining, p)
	}
	ic.pending = remaining
}

// PendingForCore returns all pending interrupts targeted at a specific core.
func (ic *InterruptController) PendingForCore(coreID int) []PendingInterrupt {
	var result []PendingInterrupt
	for _, p := range ic.pending {
		if p.TargetCore == coreID {
			result = append(result, p)
		}
	}
	return result
}

// PendingCount returns the total number of pending (unacknowledged) interrupts.
func (ic *InterruptController) PendingCount() int {
	return len(ic.pending)
}

// AcknowledgedCount returns the total number of acknowledged interrupts.
func (ic *InterruptController) AcknowledgedCount() int {
	return len(ic.acknowledged)
}

// Reset clears all pending and acknowledged interrupts.
func (ic *InterruptController) Reset() {
	ic.pending = nil
	ic.acknowledged = nil
}
