package interrupthandler

// =========================================================================
// ISR (Interrupt Service Routine) Registry
// =========================================================================

// ISRHandler is the function signature for interrupt service routines.
// When an interrupt fires and the CPU has saved context, the controller
// calls the registered ISRHandler for that interrupt number.
//
// Parameters:
//   - frame: pointer to the saved CPU state. The handler may inspect
//     (and in some cases modify) the frame -- for example, a syscall
//     handler writes the return value into the frame's register a0.
//   - kernel: an opaque handle to kernel facilities. The handler uses
//     this to access the process table, I/O buffers, etc. We use
//     interface{} here because the interrupt handler package does not
//     depend on the kernel package -- that would create a circular
//     dependency. The kernel registers concrete handlers that know
//     how to cast this to the right type.
type ISRHandler func(frame *InterruptFrame, kernel interface{})

// ISRRegistry maps interrupt numbers to Go handler functions. This is the
// "software side" of interrupt handling: the IDT maps interrupt numbers to
// memory addresses (for the hardware simulation), while the ISRRegistry
// maps them to actual Go functions (for the emulation).
//
// Why both? In a real CPU, the IDT entry's ISR address points to machine
// code in memory. In our emulator, we need to map that same interrupt
// number to a Go function. The IDT is for the simulated hardware path;
// the registry is for the emulation shortcut.
type ISRRegistry struct {
	handlers map[int]ISRHandler
}

// NewISRRegistry creates an empty ISR registry with no handlers registered.
func NewISRRegistry() *ISRRegistry {
	return &ISRRegistry{
		handlers: make(map[int]ISRHandler),
	}
}

// Register installs a handler for the given interrupt number. If a handler
// was previously registered for this number, it is silently overwritten.
// This matches real OS behavior: the kernel can replace interrupt handlers
// at runtime (e.g., during boot, the BIOS installs default handlers, then
// the kernel replaces them with its own).
func (r *ISRRegistry) Register(interruptNumber int, handler ISRHandler) {
	r.handlers[interruptNumber] = handler
}

// Dispatch calls the registered handler for the given interrupt number.
// It panics if no handler is registered -- this represents a "double fault"
// condition where an interrupt fired but nobody installed a handler for it.
// In a real OS, this would typically trigger a kernel panic / blue screen.
func (r *ISRRegistry) Dispatch(interruptNumber int, frame *InterruptFrame, kernel interface{}) {
	handler, ok := r.handlers[interruptNumber]
	if !ok {
		panic("no ISR handler registered for interrupt")
	}
	handler(frame, kernel)
}

// HasHandler returns true if a handler is registered for the given
// interrupt number. The interrupt controller checks this before dispatching
// to provide better error messages.
func (r *ISRRegistry) HasHandler(interruptNumber int) bool {
	_, ok := r.handlers[interruptNumber]
	return ok
}
