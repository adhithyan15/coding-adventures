package interrupthandler

// =========================================================================
// Interrupt Frame (Saved CPU Context)
// =========================================================================

// InterruptFrame holds all CPU state needed to resume execution after an
// interrupt has been handled. When an interrupt fires, the CPU pushes this
// frame onto the kernel stack before jumping to the ISR. When the ISR
// returns, the CPU pops the frame and resumes the interrupted code.
//
// Layout on the kernel stack (136 bytes total):
//
//	+---------------------------------------+ <- old sp
//	| PC (return address)         | 4 bytes |
//	| MStatus register            | 4 bytes |
//	| MCause register             | 4 bytes |
//	| x1  (ra - return address)   | 4 bytes |
//	| x2  (sp - stack pointer)    | 4 bytes |
//	| x3  (gp - global pointer)   | 4 bytes |
//	| ...                         |         |
//	| x31 (t6)                    | 4 bytes |
//	+---------------------------------------+ <- new sp
//	  Total: 3 + 31 = 34 words = 136 bytes
//
// Why save ALL 32 registers? The ISR is arbitrary code -- it might use any
// register. If we only saved "some" registers, the ISR could corrupt the
// interrupted program's state. Saving everything is safe and simple.
//
// Real CPUs optimize this: the ISR saves only the registers it actually
// uses (callee-saved convention). We save all 32 for correctness.
type InterruptFrame struct {
	PC        uint32     // Saved program counter (where to resume)
	Registers [32]uint32 // All 32 RISC-V general-purpose registers (x0-x31)
	MStatus   uint32     // Machine status register
	MCause    uint32     // What caused the interrupt (interrupt number)
}

// SaveContext creates an InterruptFrame from the current CPU state.
// This is called at the beginning of interrupt handling, before the ISR
// runs. The mcause field records which interrupt triggered the save.
//
// Parameters:
//   - registers: all 32 general-purpose registers (x0-x31)
//   - pc: the program counter (address of the next instruction to execute
//     after the interrupt is handled)
//   - mstatus: the machine status register
//   - mcause: the interrupt number that caused this context save
//
// Returns a complete InterruptFrame that can later be passed to
// RestoreContext to resume the interrupted code.
func SaveContext(registers [32]uint32, pc uint32, mstatus uint32, mcause uint32) InterruptFrame {
	return InterruptFrame{
		PC:        pc,
		Registers: registers,
		MStatus:   mstatus,
		MCause:    mcause,
	}
}

// RestoreContext extracts CPU state from an InterruptFrame. This is called
// after the ISR completes, to resume the interrupted code exactly where it
// left off.
//
// Returns:
//   - registers: all 32 general-purpose registers
//   - pc: the program counter to resume at
//   - mstatus: the machine status register
func RestoreContext(frame InterruptFrame) (registers [32]uint32, pc uint32, mstatus uint32) {
	return frame.Registers, frame.PC, frame.MStatus
}
