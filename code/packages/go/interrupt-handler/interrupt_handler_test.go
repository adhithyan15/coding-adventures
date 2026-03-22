package interrupthandler

import (
	"testing"
)

// =========================================================================
// IDT Tests
// =========================================================================

// TestNewIDT verifies that a freshly created IDT has all 256 entries
// marked as not present. This is the ground state before the BIOS or
// kernel installs any handlers.
func TestNewIDT(t *testing.T) {
	idt := NewIDT()
	for i := 0; i < 256; i++ {
		entry := idt.GetEntry(i)
		if entry.Present {
			t.Errorf("entry %d should not be present in a new IDT", i)
		}
		if entry.ISRAddress != 0 {
			t.Errorf("entry %d ISRAddress should be 0, got %d", i, entry.ISRAddress)
		}
		if entry.PrivilegeLevel != 0 {
			t.Errorf("entry %d PrivilegeLevel should be 0, got %d", i, entry.PrivilegeLevel)
		}
	}
}

// TestIDTSetGetEntry verifies that SetEntry and GetEntry roundtrip
// correctly for a typical interrupt (timer at entry 32).
func TestIDTSetGetEntry(t *testing.T) {
	idt := NewIDT()
	entry := IDTEntry{
		ISRAddress:     0x00020100,
		Present:        true,
		PrivilegeLevel: 0,
	}
	idt.SetEntry(IntTimer, entry)

	got := idt.GetEntry(IntTimer)
	if got.ISRAddress != 0x00020100 {
		t.Errorf("ISRAddress: want 0x00020100, got 0x%08X", got.ISRAddress)
	}
	if !got.Present {
		t.Error("Present: want true, got false")
	}
	if got.PrivilegeLevel != 0 {
		t.Errorf("PrivilegeLevel: want 0, got %d", got.PrivilegeLevel)
	}
}

// TestIDTBoundaryEntries verifies entry 0 and entry 255 to catch any
// off-by-one errors in the array indexing.
func TestIDTBoundaryEntries(t *testing.T) {
	idt := NewIDT()

	// Entry 0: division by zero
	idt.SetEntry(0, IDTEntry{ISRAddress: 0x1000, Present: true, PrivilegeLevel: 0})
	// Entry 255: last possible entry
	idt.SetEntry(255, IDTEntry{ISRAddress: 0xFF00, Present: true, PrivilegeLevel: 1})

	got0 := idt.GetEntry(0)
	if got0.ISRAddress != 0x1000 || !got0.Present {
		t.Errorf("entry 0: got %+v", got0)
	}

	got255 := idt.GetEntry(255)
	if got255.ISRAddress != 0xFF00 || !got255.Present || got255.PrivilegeLevel != 1 {
		t.Errorf("entry 255: got %+v", got255)
	}
}

// TestIDTOverwrite verifies that setting the same entry twice overwrites
// the first value with the second.
func TestIDTOverwrite(t *testing.T) {
	idt := NewIDT()
	idt.SetEntry(IntTimer, IDTEntry{ISRAddress: 0x1000, Present: true})
	idt.SetEntry(IntTimer, IDTEntry{ISRAddress: 0x2000, Present: true})

	got := idt.GetEntry(IntTimer)
	if got.ISRAddress != 0x2000 {
		t.Errorf("overwrite: want 0x2000, got 0x%08X", got.ISRAddress)
	}
}

// =========================================================================
// IDT Serialization Tests
// =========================================================================

// TestIDTWriteToMemory verifies that writing the IDT to a byte slice
// produces the correct binary layout at the expected offsets.
func TestIDTWriteToMemory(t *testing.T) {
	idt := NewIDT()
	idt.SetEntry(0, IDTEntry{ISRAddress: 0x00001000, Present: true, PrivilegeLevel: 0})
	idt.SetEntry(IntTimer, IDTEntry{ISRAddress: 0x00020100, Present: true, PrivilegeLevel: 0})
	idt.SetEntry(IntSyscall, IDTEntry{ISRAddress: 0xDEADBEEF, Present: true, PrivilegeLevel: 1})

	memory := make([]byte, IDTSize+100)
	idt.WriteToMemory(memory, 0)

	// Entry 0 at offset 0: address 0x00001000 little-endian
	if memory[0] != 0x00 || memory[1] != 0x10 || memory[2] != 0x00 || memory[3] != 0x00 {
		t.Errorf("entry 0 address bytes: %02X %02X %02X %02X", memory[0], memory[1], memory[2], memory[3])
	}
	if memory[4] != 0x01 { // present
		t.Errorf("entry 0 present: want 0x01, got 0x%02X", memory[4])
	}

	// Entry 32 (timer) at offset 32*8 = 256
	off := IntTimer * IDTEntrySize
	if memory[off] != 0x00 || memory[off+1] != 0x01 || memory[off+2] != 0x02 || memory[off+3] != 0x00 {
		t.Errorf("entry 32 address bytes: %02X %02X %02X %02X",
			memory[off], memory[off+1], memory[off+2], memory[off+3])
	}

	// Entry 128 (syscall) at offset 128*8 = 1024
	off = IntSyscall * IDTEntrySize
	if memory[off] != 0xEF || memory[off+1] != 0xBE || memory[off+2] != 0xAD || memory[off+3] != 0xDE {
		t.Errorf("entry 128 address bytes: %02X %02X %02X %02X",
			memory[off], memory[off+1], memory[off+2], memory[off+3])
	}
	if memory[off+5] != 0x01 { // privilege level
		t.Errorf("entry 128 privilege: want 0x01, got 0x%02X", memory[off+5])
	}
}

// TestIDTLoadFromMemory verifies that loading known bytes into an IDT
// produces the correct entries.
func TestIDTLoadFromMemory(t *testing.T) {
	memory := make([]byte, IDTSize)

	// Write entry 5 manually: address 0xCAFEBABE, present, privilege 0
	off := 5 * IDTEntrySize
	memory[off] = 0xBE
	memory[off+1] = 0xBA
	memory[off+2] = 0xFE
	memory[off+3] = 0xCA
	memory[off+4] = 0x01 // present
	memory[off+5] = 0x00 // privilege

	idt := NewIDT()
	idt.LoadFromMemory(memory, 0)

	got := idt.GetEntry(5)
	if got.ISRAddress != 0xCAFEBABE {
		t.Errorf("ISRAddress: want 0xCAFEBABE, got 0x%08X", got.ISRAddress)
	}
	if !got.Present {
		t.Error("Present: want true, got false")
	}
}

// TestIDTRoundtrip verifies that writing an IDT to memory and loading it
// back produces identical entries.
func TestIDTRoundtrip(t *testing.T) {
	original := NewIDT()
	original.SetEntry(0, IDTEntry{ISRAddress: 0x1000, Present: true, PrivilegeLevel: 0})
	original.SetEntry(IntTimer, IDTEntry{ISRAddress: 0x20100, Present: true, PrivilegeLevel: 0})
	original.SetEntry(IntSyscall, IDTEntry{ISRAddress: 0xDEAD, Present: true, PrivilegeLevel: 1})
	original.SetEntry(255, IDTEntry{ISRAddress: 0xFFFF, Present: true, PrivilegeLevel: 2})

	memory := make([]byte, IDTSize)
	original.WriteToMemory(memory, 0)

	loaded := NewIDT()
	loaded.LoadFromMemory(memory, 0)

	for i := 0; i < 256; i++ {
		orig := original.GetEntry(i)
		got := loaded.GetEntry(i)
		if orig.ISRAddress != got.ISRAddress || orig.Present != got.Present || orig.PrivilegeLevel != got.PrivilegeLevel {
			t.Errorf("entry %d mismatch: orig=%+v, got=%+v", i, orig, got)
		}
	}
}

// TestIDTEndianness verifies that ISR addresses are stored in little-endian
// format (RISC-V convention). The address 0x04030201 should appear as bytes
// 01 02 03 04.
func TestIDTEndianness(t *testing.T) {
	idt := NewIDT()
	idt.SetEntry(0, IDTEntry{ISRAddress: 0x04030201, Present: true})

	memory := make([]byte, IDTSize)
	idt.WriteToMemory(memory, 0)

	// Little-endian: least significant byte first
	if memory[0] != 0x01 || memory[1] != 0x02 || memory[2] != 0x03 || memory[3] != 0x04 {
		t.Errorf("endianness: want 01 02 03 04, got %02X %02X %02X %02X",
			memory[0], memory[1], memory[2], memory[3])
	}
}

// =========================================================================
// ISR Registry Tests
// =========================================================================

// TestISRRegisterAndDispatch verifies that a registered handler is called
// exactly once when dispatched.
func TestISRRegisterAndDispatch(t *testing.T) {
	registry := NewISRRegistry()
	callCount := 0

	registry.Register(IntTimer, func(frame *InterruptFrame, kernel interface{}) {
		callCount++
	})

	frame := &InterruptFrame{MCause: IntTimer}
	registry.Dispatch(IntTimer, frame, nil)

	if callCount != 1 {
		t.Errorf("handler call count: want 1, got %d", callCount)
	}
}

// TestISRHandlerReceivesFrame verifies that the dispatched handler receives
// the correct InterruptFrame.
func TestISRHandlerReceivesFrame(t *testing.T) {
	registry := NewISRRegistry()
	var receivedFrame *InterruptFrame

	registry.Register(IntTimer, func(frame *InterruptFrame, kernel interface{}) {
		receivedFrame = frame
	})

	frame := &InterruptFrame{
		PC:      0x1000,
		MCause:  IntTimer,
		MStatus: 0x1800,
	}
	frame.Registers[1] = 0xAAAA // x1 = ra
	frame.Registers[2] = 0xBBBB // x2 = sp

	registry.Dispatch(IntTimer, frame, nil)

	if receivedFrame == nil {
		t.Fatal("handler was not called")
	}
	if receivedFrame.PC != 0x1000 {
		t.Errorf("PC: want 0x1000, got 0x%08X", receivedFrame.PC)
	}
	if receivedFrame.Registers[1] != 0xAAAA {
		t.Errorf("x1: want 0xAAAA, got 0x%08X", receivedFrame.Registers[1])
	}
}

// TestISRHasHandler verifies the HasHandler query.
func TestISRHasHandler(t *testing.T) {
	registry := NewISRRegistry()
	registry.Register(IntTimer, func(frame *InterruptFrame, kernel interface{}) {})

	if !registry.HasHandler(IntTimer) {
		t.Error("HasHandler(32): want true, got false")
	}
	if registry.HasHandler(IntKeyboard) {
		t.Error("HasHandler(33): want false, got true")
	}
}

// TestISROverwrite verifies that registering a new handler for the same
// interrupt number replaces the old one.
func TestISROverwrite(t *testing.T) {
	registry := NewISRRegistry()
	firstCalled := false
	secondCalled := false

	registry.Register(IntTimer, func(frame *InterruptFrame, kernel interface{}) {
		firstCalled = true
	})
	registry.Register(IntTimer, func(frame *InterruptFrame, kernel interface{}) {
		secondCalled = true
	})

	registry.Dispatch(IntTimer, &InterruptFrame{}, nil)

	if firstCalled {
		t.Error("first handler should not have been called after overwrite")
	}
	if !secondCalled {
		t.Error("second handler should have been called")
	}
}

// TestISRDispatchPanicsOnMissing verifies that dispatching to an
// unregistered interrupt number panics (double fault).
func TestISRDispatchPanicsOnMissing(t *testing.T) {
	registry := NewISRRegistry()

	defer func() {
		if r := recover(); r == nil {
			t.Error("expected panic when dispatching unregistered interrupt")
		}
	}()

	registry.Dispatch(IntTimer, &InterruptFrame{}, nil)
}

// =========================================================================
// Interrupt Controller Tests
// =========================================================================

// TestControllerRaiseInterrupt verifies that raising an interrupt adds it
// to the pending queue.
func TestControllerRaiseInterrupt(t *testing.T) {
	ic := NewInterruptController()
	ic.RaiseInterrupt(IntTimer)

	if ic.PendingCount() != 1 {
		t.Errorf("PendingCount: want 1, got %d", ic.PendingCount())
	}
}

// TestControllerHasPending verifies the HasPending check.
func TestControllerHasPending(t *testing.T) {
	ic := NewInterruptController()

	if ic.HasPending() {
		t.Error("HasPending should be false with empty queue")
	}

	ic.RaiseInterrupt(IntTimer)
	if !ic.HasPending() {
		t.Error("HasPending should be true after raising interrupt")
	}
}

// TestControllerNextPending verifies priority ordering: lower number =
// higher priority, so NextPending returns the lowest-numbered interrupt.
func TestControllerNextPending(t *testing.T) {
	ic := NewInterruptController()
	ic.RaiseInterrupt(IntKeyboard) // 33
	ic.RaiseInterrupt(IntTimer)    // 32

	next := ic.NextPending()
	if next != IntTimer {
		t.Errorf("NextPending: want %d (timer), got %d", IntTimer, next)
	}
}

// TestControllerAcknowledge verifies that acknowledging an interrupt
// removes it from the pending queue.
func TestControllerAcknowledge(t *testing.T) {
	ic := NewInterruptController()
	ic.RaiseInterrupt(IntTimer)
	ic.Acknowledge(IntTimer)

	if ic.PendingCount() != 0 {
		t.Errorf("PendingCount after acknowledge: want 0, got %d", ic.PendingCount())
	}
}

// TestControllerNoDuplicates verifies that raising the same interrupt
// twice does not create duplicate entries in the pending queue.
func TestControllerNoDuplicates(t *testing.T) {
	ic := NewInterruptController()
	ic.RaiseInterrupt(IntTimer)
	ic.RaiseInterrupt(IntTimer)

	if ic.PendingCount() != 1 {
		t.Errorf("PendingCount: want 1, got %d", ic.PendingCount())
	}
}

// TestControllerMask verifies that a masked interrupt is pending but not
// dispatched (HasPending returns false because no unmasked interrupts exist).
func TestControllerMask(t *testing.T) {
	ic := NewInterruptController()
	ic.SetMask(IntInvalidOpcode, true) // mask interrupt 5
	ic.RaiseInterrupt(IntInvalidOpcode)

	// The interrupt IS in the queue...
	if ic.PendingCount() != 1 {
		t.Errorf("PendingCount: want 1, got %d", ic.PendingCount())
	}
	// ...but HasPending returns false because it's masked
	if ic.HasPending() {
		t.Error("HasPending should be false when only masked interrupts are pending")
	}
	// NextPending should return -1
	if ic.NextPending() != -1 {
		t.Errorf("NextPending: want -1, got %d", ic.NextPending())
	}
}

// TestControllerUnmask verifies that unmasking a previously masked
// interrupt makes it dispatchable.
func TestControllerUnmask(t *testing.T) {
	ic := NewInterruptController()
	ic.SetMask(IntInvalidOpcode, true)
	ic.RaiseInterrupt(IntInvalidOpcode)

	// Masked: not dispatchable
	if ic.HasPending() {
		t.Error("should not have pending while masked")
	}

	// Unmask: now dispatchable
	ic.SetMask(IntInvalidOpcode, false)
	if !ic.HasPending() {
		t.Error("should have pending after unmask")
	}
}

// TestControllerIsMasked verifies the IsMasked query and that interrupts
// 32+ are never masked by the mask register.
func TestControllerIsMasked(t *testing.T) {
	ic := NewInterruptController()

	// Initially no masks
	if ic.IsMasked(5) {
		t.Error("interrupt 5 should not be masked initially")
	}

	ic.SetMask(5, true)
	if !ic.IsMasked(5) {
		t.Error("interrupt 5 should be masked after SetMask(5, true)")
	}

	// Interrupts 32+ are never masked by the mask register
	if ic.IsMasked(IntTimer) {
		t.Error("interrupt 32 should never be masked by mask register")
	}
}

// TestControllerGlobalDisable verifies that disabling interrupts prevents
// dispatch even when interrupts are pending and unmasked.
func TestControllerGlobalDisable(t *testing.T) {
	ic := NewInterruptController()
	ic.Disable()
	ic.RaiseInterrupt(IntTimer)

	if ic.HasPending() {
		t.Error("HasPending should be false when globally disabled")
	}
	if ic.NextPending() != -1 {
		t.Errorf("NextPending: want -1 when disabled, got %d", ic.NextPending())
	}
}

// TestControllerGlobalEnable verifies that re-enabling interrupts allows
// pending interrupts to be dispatched.
func TestControllerGlobalEnable(t *testing.T) {
	ic := NewInterruptController()
	ic.Disable()
	ic.RaiseInterrupt(IntTimer)
	ic.Enable()

	if !ic.HasPending() {
		t.Error("HasPending should be true after re-enabling")
	}
}

// TestControllerClearAll verifies that ClearAll removes all pending
// interrupts.
func TestControllerClearAll(t *testing.T) {
	ic := NewInterruptController()
	ic.RaiseInterrupt(IntTimer)
	ic.RaiseInterrupt(IntKeyboard)
	ic.RaiseInterrupt(IntSyscall)
	ic.ClearAll()

	if ic.PendingCount() != 0 {
		t.Errorf("PendingCount after ClearAll: want 0, got %d", ic.PendingCount())
	}
}

// =========================================================================
// Context Save/Restore Tests
// =========================================================================

// TestContextRoundtrip verifies that saving and restoring context produces
// identical register values, PC, and MStatus.
func TestContextRoundtrip(t *testing.T) {
	var regs [32]uint32
	for i := 0; i < 32; i++ {
		regs[i] = uint32(i * 100)
	}
	pc := uint32(0x00080000)
	mstatus := uint32(0x00001800)
	mcause := uint32(IntTimer)

	frame := SaveContext(regs, pc, mstatus, mcause)
	gotRegs, gotPC, gotMStatus := RestoreContext(frame)

	if gotPC != pc {
		t.Errorf("PC: want 0x%08X, got 0x%08X", pc, gotPC)
	}
	if gotMStatus != mstatus {
		t.Errorf("MStatus: want 0x%08X, got 0x%08X", mstatus, gotMStatus)
	}
	for i := 0; i < 32; i++ {
		if gotRegs[i] != regs[i] {
			t.Errorf("register x%d: want %d, got %d", i, regs[i], gotRegs[i])
		}
	}
}

// TestContextAllRegisters verifies that all 32 registers are saved and
// restored with distinct values.
func TestContextAllRegisters(t *testing.T) {
	var regs [32]uint32
	for i := 0; i < 32; i++ {
		regs[i] = uint32(0xDEAD0000 + i)
	}

	frame := SaveContext(regs, 0, 0, 0)
	gotRegs, _, _ := RestoreContext(frame)

	for i := 0; i < 32; i++ {
		want := uint32(0xDEAD0000 + i)
		if gotRegs[i] != want {
			t.Errorf("register x%d: want 0x%08X, got 0x%08X", i, want, gotRegs[i])
		}
	}
}

// TestContextMCause verifies that MCause is stored in the frame.
func TestContextMCause(t *testing.T) {
	frame := SaveContext([32]uint32{}, 0, 0, IntTimer)
	if frame.MCause != IntTimer {
		t.Errorf("MCause: want %d, got %d", IntTimer, frame.MCause)
	}
}

// =========================================================================
// Priority Tests
// =========================================================================

// TestPriorityMultiplePending verifies that multiple pending interrupts
// are dispatched in priority order (lowest number first).
func TestPriorityMultiplePending(t *testing.T) {
	ic := NewInterruptController()
	ic.RaiseInterrupt(IntSyscall)  // 128
	ic.RaiseInterrupt(IntKeyboard) // 33
	ic.RaiseInterrupt(IntInvalidOpcode) // 5
	ic.RaiseInterrupt(IntTimer)    // 32

	// Expected dispatch order: 5, 32, 33, 128
	expected := []int{IntInvalidOpcode, IntTimer, IntKeyboard, IntSyscall}
	for _, want := range expected {
		got := ic.NextPending()
		if got != want {
			t.Errorf("NextPending: want %d, got %d", want, got)
		}
		ic.Acknowledge(got)
	}

	if ic.PendingCount() != 0 {
		t.Errorf("PendingCount after all acknowledged: want 0, got %d", ic.PendingCount())
	}
}

// TestPriorityAcknowledgeAndNext verifies that acknowledging the highest-
// priority interrupt reveals the next one.
func TestPriorityAcknowledgeAndNext(t *testing.T) {
	ic := NewInterruptController()
	ic.RaiseInterrupt(IntInvalidOpcode) // 5
	ic.RaiseInterrupt(IntTimer)         // 32

	if ic.NextPending() != IntInvalidOpcode {
		t.Errorf("first NextPending: want %d, got %d", IntInvalidOpcode, ic.NextPending())
	}

	ic.Acknowledge(IntInvalidOpcode)

	if ic.NextPending() != IntTimer {
		t.Errorf("second NextPending: want %d, got %d", IntTimer, ic.NextPending())
	}
}

// =========================================================================
// Full Lifecycle Test
// =========================================================================

// TestFullLifecycle simulates the complete interrupt lifecycle:
// raise -> mask check -> context save -> IDT lookup -> ISR dispatch ->
// acknowledge -> context restore -> verify.
func TestFullLifecycle(t *testing.T) {
	ic := NewInterruptController()

	// Step 1: Install a timer ISR in the IDT and registry
	ic.IDT.SetEntry(IntTimer, IDTEntry{
		ISRAddress:     0x00020100,
		Present:        true,
		PrivilegeLevel: 0,
	})

	handlerCalled := false
	var handlerFrame *InterruptFrame

	ic.Registry.Register(IntTimer, func(frame *InterruptFrame, kernel interface{}) {
		handlerCalled = true
		handlerFrame = frame
	})

	// Step 2: Set up CPU state (simulating a running program)
	var cpuRegs [32]uint32
	cpuRegs[1] = 0x10000  // ra
	cpuRegs[2] = 0x7FFF0  // sp
	cpuRegs[10] = 42       // a0 (syscall argument)
	cpuPC := uint32(0x80000)
	cpuMStatus := uint32(0x1800)

	// Step 3: Timer fires
	ic.RaiseInterrupt(IntTimer)

	// Step 4: Pipeline checks for pending interrupts
	if !ic.HasPending() {
		t.Fatal("expected pending interrupt after raise")
	}

	// Step 5: Get the next pending interrupt
	intNum := ic.NextPending()
	if intNum != IntTimer {
		t.Fatalf("NextPending: want %d, got %d", IntTimer, intNum)
	}

	// Step 6: Save context
	frame := SaveContext(cpuRegs, cpuPC, cpuMStatus, uint32(intNum))

	// Step 7: Disable interrupts (prevent nesting)
	ic.Disable()

	// Step 8: Look up IDT entry
	idtEntry := ic.IDT.GetEntry(intNum)
	if !idtEntry.Present {
		t.Fatal("IDT entry should be present")
	}
	if idtEntry.ISRAddress != 0x00020100 {
		t.Fatalf("ISR address: want 0x00020100, got 0x%08X", idtEntry.ISRAddress)
	}

	// Step 9: Dispatch ISR
	ic.Registry.Dispatch(intNum, &frame, nil)

	if !handlerCalled {
		t.Fatal("ISR handler was not called")
	}
	if handlerFrame.MCause != uint32(IntTimer) {
		t.Errorf("MCause in handler: want %d, got %d", IntTimer, handlerFrame.MCause)
	}

	// Step 10: Acknowledge interrupt
	ic.Acknowledge(intNum)
	if ic.PendingCount() != 0 {
		t.Errorf("PendingCount after acknowledge: want 0, got %d", ic.PendingCount())
	}

	// Step 11: Restore context
	restoredRegs, restoredPC, restoredMStatus := RestoreContext(frame)

	// Step 12: Re-enable interrupts
	ic.Enable()

	// Step 13: Verify restored state matches original
	if restoredPC != cpuPC {
		t.Errorf("restored PC: want 0x%08X, got 0x%08X", cpuPC, restoredPC)
	}
	if restoredMStatus != cpuMStatus {
		t.Errorf("restored MStatus: want 0x%08X, got 0x%08X", cpuMStatus, restoredMStatus)
	}
	if restoredRegs[1] != 0x10000 {
		t.Errorf("restored ra: want 0x10000, got 0x%08X", restoredRegs[1])
	}
	if restoredRegs[2] != 0x7FFF0 {
		t.Errorf("restored sp: want 0x7FFF0, got 0x%08X", restoredRegs[2])
	}
	if restoredRegs[10] != 42 {
		t.Errorf("restored a0: want 42, got %d", restoredRegs[10])
	}
}

// TestNextPendingWhenEmpty verifies that NextPending returns -1 when no
// interrupts are pending.
func TestNextPendingWhenEmpty(t *testing.T) {
	ic := NewInterruptController()
	if ic.NextPending() != -1 {
		t.Errorf("NextPending on empty: want -1, got %d", ic.NextPending())
	}
}

// TestMaskHighInterruptIsIgnored verifies that SetMask for interrupts 32+
// is a no-op (they are always unmasked in our simplified model).
func TestMaskHighInterruptIsIgnored(t *testing.T) {
	ic := NewInterruptController()
	ic.SetMask(IntTimer, true) // 32 is out of mask register range
	ic.RaiseInterrupt(IntTimer)

	// Timer should still be dispatchable since mask register only covers 0-31
	if !ic.HasPending() {
		t.Error("interrupt 32+ should not be affected by mask register")
	}
}
