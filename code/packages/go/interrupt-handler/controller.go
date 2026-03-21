package interrupthandler

import "sort"

// =========================================================================
// Interrupt Controller
// =========================================================================

// InterruptController manages the full interrupt lifecycle: pending queue,
// masking, enable/disable, and dispatching to ISRs. It is the central hub
// that connects hardware signals to software handlers.
//
// The lifecycle of an interrupt:
//
//  1. A device (timer, keyboard) or instruction (ecall) calls RaiseInterrupt.
//  2. The interrupt is added to the Pending queue.
//  3. The pipeline checks HasPending() between instructions.
//  4. If pending: NextPending() returns the highest-priority interrupt.
//  5. The CPU saves context (SaveContext), looks up the IDT, and dispatches
//     the ISR via the Registry.
//  6. After the ISR returns, the CPU calls Acknowledge() to remove the
//     interrupt from the pending queue.
//  7. The CPU calls RestoreContext() and resumes the interrupted code.
//
// Masking: The mask register is a 32-bit value where each bit controls
// whether the corresponding interrupt number (0-31) is blocked. Interrupts
// 32+ are always unmasked in our simplified model (unless globally disabled).
//
//	Mask Register: 0b...0000_0000_0000_0100
//	                                    ^
//	                        Interrupt 2 is masked (blocked)
//
// Global enable: When Enabled is false, NO interrupts are dispatched.
// This is used during interrupt handling itself to prevent nested
// interrupts (our simplified model does not support nesting).
type InterruptController struct {
	IDT          *InterruptDescriptorTable
	Registry     *ISRRegistry
	Pending      []int  // Queue of pending interrupt numbers (sorted ascending)
	MaskRegister uint32 // Bitmask: bit N = 1 means interrupt N is masked
	Enabled      bool   // Global interrupt enable flag
}

// NewInterruptController creates a controller with a fresh IDT, empty
// registry, no pending interrupts, no masks, and interrupts enabled.
func NewInterruptController() *InterruptController {
	return &InterruptController{
		IDT:          NewIDT(),
		Registry:     NewISRRegistry(),
		Pending:      nil,
		MaskRegister: 0,
		Enabled:      true,
	}
}

// RaiseInterrupt adds an interrupt to the pending queue. If the interrupt
// is already pending, it is not added again (no duplicates). The pending
// queue is kept sorted by interrupt number (ascending = higher priority
// first).
//
// This is called by hardware devices:
//
//	Timer chip every N cycles:  controller.RaiseInterrupt(32)
//	Keyboard on keystroke:      controller.RaiseInterrupt(33)
//	ecall instruction:          controller.RaiseInterrupt(128)
func (ic *InterruptController) RaiseInterrupt(number int) {
	// Check for duplicates: an interrupt that is already pending should
	// not be added again. In real hardware, this is handled by the
	// interrupt controller's "pending" bit register.
	for _, n := range ic.Pending {
		if n == number {
			return
		}
	}
	ic.Pending = append(ic.Pending, number)
	sort.Ints(ic.Pending)
}

// HasPending returns true if there are any unmasked pending interrupts AND
// the global enable flag is set. The pipeline calls this between
// instructions to decide whether to take an interrupt.
//
// The check considers:
//  1. Global enable flag must be true
//  2. At least one pending interrupt must not be masked
func (ic *InterruptController) HasPending() bool {
	if !ic.Enabled {
		return false
	}
	for _, n := range ic.Pending {
		if !ic.IsMasked(n) {
			return true
		}
	}
	return false
}

// NextPending returns the highest-priority (lowest-numbered) unmasked
// pending interrupt. Returns -1 if no unmasked interrupts are pending
// or if interrupts are globally disabled.
//
// Priority rule: lower interrupt number = higher priority.
//
//	Multiple pending: [5, 32, 33, 128]
//	NextPending() returns 5 (CPU exception = highest priority)
func (ic *InterruptController) NextPending() int {
	if !ic.Enabled {
		return -1
	}
	for _, n := range ic.Pending {
		if !ic.IsMasked(n) {
			return n
		}
	}
	return -1
}

// Acknowledge removes the given interrupt from the pending queue. This is
// called after the ISR completes, signaling that the interrupt has been
// handled.
//
// In real hardware, the CPU sends an EOI (End of Interrupt) signal to the
// interrupt controller. In our simulation, Acknowledge() serves the same
// purpose.
func (ic *InterruptController) Acknowledge(number int) {
	for i, n := range ic.Pending {
		if n == number {
			ic.Pending = append(ic.Pending[:i], ic.Pending[i+1:]...)
			return
		}
	}
}

// SetMask sets or clears the mask for a specific interrupt number (0-31).
// Masked interrupts remain in the pending queue but are not dispatched.
//
// This is how the kernel temporarily blocks interrupts during critical
// sections (e.g., while modifying the process table). Interrupts 32+
// are not controlled by the mask register in our simplified model.
//
//	masked=true:  block the interrupt (set bit)
//	masked=false: allow the interrupt (clear bit)
func (ic *InterruptController) SetMask(number int, masked bool) {
	if number < 0 || number > 31 {
		return // Only interrupts 0-31 are maskable
	}
	if masked {
		ic.MaskRegister |= 1 << uint(number)
	} else {
		ic.MaskRegister &^= 1 << uint(number)
	}
}

// IsMasked returns true if the given interrupt number is currently masked
// (blocked). Interrupts 32+ are never masked by the mask register (they
// can only be blocked by the global enable flag).
func (ic *InterruptController) IsMasked(number int) bool {
	if number < 0 || number > 31 {
		return false // Interrupts 32+ are always unmasked
	}
	return ic.MaskRegister&(1<<uint(number)) != 0
}

// Enable sets the global interrupt enable flag. After calling Enable(),
// pending unmasked interrupts can be dispatched. This is called after an
// ISR returns (as part of the mret instruction) to re-enable interrupts.
func (ic *InterruptController) Enable() {
	ic.Enabled = true
}

// Disable clears the global interrupt enable flag. No interrupts will be
// dispatched until Enable() is called. This is used:
//
//  1. Automatically when entering an ISR (prevent nesting)
//  2. Manually by the kernel during critical sections
func (ic *InterruptController) Disable() {
	ic.Enabled = false
}

// PendingCount returns the number of interrupts currently in the pending
// queue (both masked and unmasked).
func (ic *InterruptController) PendingCount() int {
	return len(ic.Pending)
}

// ClearAll removes all pending interrupts from the queue.
func (ic *InterruptController) ClearAll() {
	ic.Pending = nil
}
