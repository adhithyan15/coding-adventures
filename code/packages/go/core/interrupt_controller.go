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
	result, _ := StartNew[*InterruptController]("core.NewInterruptController", nil,
		func(op *Operation[*InterruptController], rf *ResultFactory[*InterruptController]) *OperationResult[*InterruptController] {
			op.AddProperty("num_cores", numCores)
			return rf.Generate(true, false, &InterruptController{
				numCores: numCores,
			})
		}).GetResult()
	return result
}

// RaiseInterrupt queues an interrupt for delivery.
//
// If targetCore is -1, the interrupt will be routed to core 0 (simplest
// routing policy -- round-robin or load-based routing is a future extension).
func (ic *InterruptController) RaiseInterrupt(interruptID int, targetCore int) {
	_, _ = StartNew[struct{}]("core.InterruptController.RaiseInterrupt", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("interrupt_id", interruptID)
			op.AddProperty("target_core", targetCore)
			if targetCore == -1 {
				targetCore = 0
			}
			if targetCore >= ic.numCores {
				targetCore = 0
			}
			ic.pending = append(ic.pending, PendingInterrupt{
				InterruptID: interruptID,
				TargetCore:  targetCore,
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Acknowledge records that a core has begun handling an interrupt.
//
// In real hardware, acknowledgment tells the interrupt controller that the
// core has received the signal and started executing the interrupt handler.
// The controller can then clear the pending flag and potentially deliver
// the next interrupt.
func (ic *InterruptController) Acknowledge(coreID int, interruptID int) {
	_, _ = StartNew[struct{}]("core.InterruptController.Acknowledge", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("core_id", coreID)
			op.AddProperty("interrupt_id", interruptID)
			ic.acknowledged = append(ic.acknowledged, AcknowledgedInterrupt{
				CoreID:      coreID,
				InterruptID: interruptID,
			})

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
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// PendingForCore returns all pending interrupts targeted at a specific core.
func (ic *InterruptController) PendingForCore(coreID int) []PendingInterrupt {
	result, _ := StartNew[[]PendingInterrupt]("core.InterruptController.PendingForCore", nil,
		func(op *Operation[[]PendingInterrupt], rf *ResultFactory[[]PendingInterrupt]) *OperationResult[[]PendingInterrupt] {
			op.AddProperty("core_id", coreID)
			var pending []PendingInterrupt
			for _, p := range ic.pending {
				if p.TargetCore == coreID {
					pending = append(pending, p)
				}
			}
			return rf.Generate(true, false, pending)
		}).GetResult()
	return result
}

// PendingCount returns the total number of pending (unacknowledged) interrupts.
func (ic *InterruptController) PendingCount() int {
	result, _ := StartNew[int]("core.InterruptController.PendingCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(ic.pending))
		}).GetResult()
	return result
}

// AcknowledgedCount returns the total number of acknowledged interrupts.
func (ic *InterruptController) AcknowledgedCount() int {
	result, _ := StartNew[int]("core.InterruptController.AcknowledgedCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(ic.acknowledged))
		}).GetResult()
	return result
}

// Reset clears all pending and acknowledged interrupts.
func (ic *InterruptController) Reset() {
	_, _ = StartNew[struct{}]("core.InterruptController.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			ic.pending = nil
			ic.acknowledged = nil
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
