package rombios

// === BIOS Firmware Generator ===
//
// The BIOS (Basic Input/Output System) is the very first code that runs
// when the simulated computer powers on. It lives in ROM at 0xFFFF0000
// and performs four critical tasks:
//
//   1. Probe memory  -- discover how much RAM is available
//   2. Initialize IDT -- set up interrupt handlers so the system can
//                        respond to hardware events and exceptions
//   3. Write HardwareInfo -- leave a "status report" at 0x00001000
//                            for the bootloader to read
//   4. Jump to bootloader -- transfer control to 0x00010000
//
// The firmware is generated as raw RISC-V machine code using encoding
// helpers from the riscv-simulator package. This avoids a dependency on
// the assembler (which would create a circular dependency) and provides
// educational value: the reader sees exactly how high-level operations
// translate to individual RISC-V instructions.
//
// === Analogy ===
//
// The BIOS is like a building manager who arrives first thing in the
// morning. They turn on the lights, check all the rooms, verify the
// heating works, write a status report and leave it on the front desk,
// then unlock the front door for the tenants (the OS).

import (
	"fmt"

	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

// === Well-known addresses ===
//
// These constants define the memory layout that the BIOS firmware uses.
// They form a contract between the BIOS, bootloader, and kernel.
const (
	// IDTBase is where the Interrupt Descriptor Table starts.
	// Entry 0 is at address 0x00000000, entry 1 at 0x00000008, etc.
	IDTBase uint32 = 0x00000000

	// IDTEntryCount is the number of IDT entries (matching x86 convention).
	IDTEntryCount = 256

	// IDTEntrySize is the size of each IDT entry in bytes.
	// Each entry holds an ISR address (4 bytes) + flags (4 bytes).
	IDTEntrySize = 8

	// ISRStubBase is where the ISR stub routines live in memory.
	// These are minimal placeholders that the kernel will overwrite.
	ISRStubBase uint32 = 0x00000800

	// DefaultFaultHandler is the address of the default fault handler.
	// It is an infinite loop (JAL x0, 0) that spins forever, making
	// unexpected interrupts detectable by a debugger.
	DefaultFaultHandler uint32 = 0x00000800

	// TimerISR is the address of the timer interrupt handler stub.
	TimerISR uint32 = 0x00000808

	// KeyboardISR is the address of the keyboard interrupt handler stub.
	KeyboardISR uint32 = 0x00000810

	// SyscallISR is the address of the system call handler stub.
	SyscallISR uint32 = 0x00000818

	// ProbeStart is the starting address for the memory probe.
	// We skip low memory (0x00000000-0x000FFFFF) because it contains
	// the IDT, HardwareInfo, and ISR stubs.
	ProbeStart uint32 = 0x00100000

	// ProbeStep is the increment between probe addresses (1 MB).
	ProbeStep uint32 = 0x00100000

	// ProbeLimit is the upper bound for probing. We must not probe
	// into the framebuffer or ROM region.
	ProbeLimit uint32 = 0xFFFB0000

	// DefaultBootloaderEntry is where BIOS jumps after initialization.
	DefaultBootloaderEntry uint32 = 0x00010000

	// DefaultFramebufferBase is the default framebuffer address.
	DefaultFramebufferBase uint32 = 0xFFFB0000
)

// BIOSConfig controls what the BIOS firmware will do during initialization.
// These values determine the contents of the HardwareInfo struct and the
// final jump target.
type BIOSConfig struct {
	// MemorySize is the RAM size to report. If 0, the BIOS will probe
	// memory at boot time by writing and reading test patterns.
	// For testing, set this to a known value to skip the probe.
	MemorySize int

	// DisplayColumns is the text display width (default: 80).
	DisplayColumns int

	// DisplayRows is the text display height (default: 25).
	DisplayRows int

	// FramebufferBase is the framebuffer start address.
	// Default: 0xFFFB0000.
	FramebufferBase uint32

	// BootloaderEntry is the address to jump to after initialization.
	// Default: 0x00010000.
	BootloaderEntry uint32
}

// DefaultBIOSConfig returns a sensible default configuration:
//
//	MemorySize: 0 (probe), DisplayColumns: 80, DisplayRows: 25,
//	FramebufferBase: 0xFFFB0000, BootloaderEntry: 0x00010000.
func DefaultBIOSConfig() BIOSConfig {
	return BIOSConfig{
		MemorySize:      0,
		DisplayColumns:  80,
		DisplayRows:     25,
		FramebufferBase: DefaultFramebufferBase,
		BootloaderEntry: DefaultBootloaderEntry,
	}
}

// AnnotatedInstruction pairs a machine code instruction with its
// human-readable assembly and a comment explaining its purpose.
//
// This is invaluable for debugging and education -- a reader can see
// the raw machine code, its assembly equivalent, and what it does in
// the boot sequence, all in one place.
//
// Example:
//
//	AnnotatedInstruction{
//	    Address:     0xFFFF0000,
//	    MachineCode: 0x001002B7,
//	    Assembly:    "lui x5, 0x00100",
//	    Comment:     "Step 1: Load probe start address (1 MB) into x5",
//	}
type AnnotatedInstruction struct {
	Address     uint32 // Memory address where this instruction lives
	MachineCode uint32 // Raw 32-bit RISC-V instruction
	Assembly    string // Human-readable assembly (e.g., "lui x5, 0x100")
	Comment     string // What this instruction does in the boot sequence
}

// BIOSFirmware generates BIOS firmware as RISC-V machine code.
//
// Rather than writing assembly by hand, the firmware is constructed
// programmatically. Each instruction is emitted using encoding helpers
// that produce the correct 32-bit machine code.
type BIOSFirmware struct {
	Config BIOSConfig
}

// NewBIOSFirmware creates a firmware generator with the given config.
func NewBIOSFirmware(config BIOSConfig) *BIOSFirmware {
	return &BIOSFirmware{Config: config}
}

// Generate returns the BIOS firmware as raw RISC-V machine code bytes.
// The returned byte slice can be loaded directly into a ROM.
func (b *BIOSFirmware) Generate() []byte {
	annotated := b.GenerateWithComments()
	instructions := make([]uint32, len(annotated))
	for i, a := range annotated {
		instructions[i] = a.MachineCode
	}
	return riscv.Assemble(instructions)
}

// GenerateWithComments returns the firmware as annotated instructions.
// Each instruction includes its address, machine code, assembly text,
// and a human-readable comment.
func (b *BIOSFirmware) GenerateWithComments() []AnnotatedInstruction {
	var instructions []AnnotatedInstruction
	address := DefaultROMBase

	// Helper to emit an instruction and advance the address counter.
	emit := func(code uint32, asm, comment string) {
		instructions = append(instructions, AnnotatedInstruction{
			Address:     address,
			MachineCode: code,
			Assembly:    asm,
			Comment:     comment,
		})
		address += 4
	}

	// ═══════════════════════════════════════════════════════════════
	// Step 1: Memory Probe
	// ═══════════════════════════════════════════════════════════════
	//
	// Discover how much RAM is installed by writing a test pattern
	// (0xDEADBEEF) to progressively higher addresses. If the value
	// reads back correctly, that memory exists. If not, we've found
	// the boundary.
	//
	// Registers used:
	//   x5  = current test address
	//   x6  = test pattern (0xDEADBEEF)
	//   x7  = value read back
	//   x8  = memory size result
	//   x9  = probe limit (0xFFFB0000)
	//   x10 = probe step (0x00100000 = 1 MB)

	if b.Config.MemorySize > 0 {
		// Skip the probe -- use the configured value directly.
		// This is useful for testing where we know the RAM size.
		upper := uint32(b.Config.MemorySize) >> 12
		lower := uint32(b.Config.MemorySize) & 0xFFF

		emit(riscv.EncodeLui(8, int(upper)),
			fmt.Sprintf("lui x8, 0x%05X", upper),
			fmt.Sprintf("Step 1: Load configured memory size upper bits (%d bytes)", b.Config.MemorySize))

		if lower != 0 {
			emit(riscv.EncodeAddi(8, 8, int(lower)),
				fmt.Sprintf("addi x8, x8, 0x%03X", lower),
				"Step 1: Add lower 12 bits of memory size")
		}
	} else {
		// Memory probe: write test pattern and read back.
		emit(riscv.EncodeLui(5, int(ProbeStart>>12)),
			fmt.Sprintf("lui x5, 0x%05X", ProbeStart>>12),
			"Step 1: x5 = 0x00100000 (probe start at 1 MB)")

		// Load test pattern 0xDEADBEEF into x6.
		// LUI loads upper 20 bits: 0xDEADB << 12 = 0xDEADB000
		// But 0xEEF sign-extends to -0x111 when treated as 12-bit signed,
		// and LUI needs to compensate. The trick: if bit 11 of the lower
		// part is set, add 1 to the upper part.
		//
		// 0xDEADBEEF = (0xDEADC << 12) + (-0x111)
		//   because 0xBEEF = 0xB000 + 0xEEF
		//   and 0xEEF as signed 12-bit = -0x111 (since 0xEEF > 0x7FF)
		//   so we need LUI 0xDEADC, ADDI -0x111
		//
		// Actually let's compute correctly:
		//   0xDEADBEEF: upper20 = 0xDEADB, lower12 = 0xEEF
		//   0xEEF = 3823, which has bit 11 set (> 2047)
		//   So signed interpretation = 0xEEF - 0x1000 = -273 = -0x111
		//   Compensated upper = 0xDEADB + 1 = 0xDEADC
		emit(riscv.EncodeLui(6, 0xDEADC),
			"lui x6, 0xDEADC",
			"Step 1: x6 upper = 0xDEADC000 (compensated for sign extension)")

		emit(riscv.EncodeAddi(6, 6, signExtend12(0xEEF)),
			fmt.Sprintf("addi x6, x6, %d", signExtend12(0xEEF)),
			"Step 1: x6 = 0xDEADBEEF (test pattern)")

		emit(riscv.EncodeLui(9, int(ProbeLimit>>12)),
			fmt.Sprintf("lui x9, 0x%05X", ProbeLimit>>12),
			"Step 1: x9 = 0xFFFB0000 (probe limit)")

		emit(riscv.EncodeLui(10, int(ProbeStep>>12)),
			fmt.Sprintf("lui x10, 0x%05X", ProbeStep>>12),
			"Step 1: x10 = 0x00100000 (1 MB probe step)")

		// probe_loop:
		//   sw x6, 0(x5)   -- write test pattern
		//   lw x7, 0(x5)   -- read it back
		//   bne x6, x7, +12 -- if mismatch, skip to probe_done
		//   add x5, x5, x10 -- advance by 1 MB
		//   blt x5, x9, -16 -- loop back if below limit

		emit(riscv.EncodeSw(6, 5, 0),
			"sw x6, 0(x5)",
			"Step 1: Write test pattern to [x5]")

		emit(riscv.EncodeLw(7, 5, 0),
			"lw x7, 0(x5)",
			"Step 1: Read it back into x7")

		emit(riscv.EncodeBne(6, 7, 12),
			"bne x6, x7, +12",
			"Step 1: If mismatch, memory ends here (skip to probe_done)")

		emit(riscv.EncodeAdd(5, 5, 10),
			"add x5, x5, x10",
			"Step 1: Advance probe address by 1 MB")

		emit(riscv.EncodeBlt(5, 9, -16),
			"blt x5, x9, -16",
			"Step 1: Loop back if below probe limit")

		// probe_done: x5 holds the detected memory size
		emit(riscv.EncodeAdd(8, 5, 0),
			"add x8, x5, x0",
			"Step 1: x8 = detected memory size (copy from x5)")
	}

	// ═══════════════════════════════════════════════════════════════
	// Step 2: IDT Initialization
	// ═══════════════════════════════════════════════════════════════
	//
	// Write ISR stub routines at 0x00000800, then populate the IDT
	// with 256 entries. Each entry is 8 bytes: ISR address + flags.
	//
	// ISR stubs:
	//   0x800: default_fault_handler: jal x0, 0 (infinite loop)
	//   0x808: timer_isr: mret (return from interrupt)
	//   0x810: keyboard_isr: mret
	//   0x818: syscall_isr: mret

	// First, write ISR stub code to memory at 0x00000800.
	// We use x11 as a base register and x12 for instruction words.

	// Load ISR stub base address into x11
	emit(riscv.EncodeLui(11, int(ISRStubBase>>12)),
		fmt.Sprintf("lui x11, 0x%05X", ISRStubBase>>12),
		"Step 2a: x11 = 0x00000800 (ISR stub base)")

	// If ISRStubBase lower 12 bits are non-zero, add them
	if ISRStubBase&0xFFF != 0 {
		emit(riscv.EncodeAddi(11, 11, int(ISRStubBase&0xFFF)),
			fmt.Sprintf("addi x11, x11, %d", ISRStubBase&0xFFF),
			"Step 2a: Add lower bits of ISR stub base")
	}

	// Write default_fault_handler at 0x800: jal x0, 0 (infinite loop)
	// JAL x0, 0 encodes to 0x0000006F
	faultInstr := riscv.EncodeJal(0, 0)
	emit(encodeLiUpper(12, faultInstr),
		fmt.Sprintf("lui x12, 0x%05X", faultInstr>>12),
		"Step 2a: Load fault handler instruction (jal x0, 0) upper bits")
	if faultInstr&0xFFF != 0 {
		emit(riscv.EncodeAddi(12, 12, signExtend12(int(faultInstr&0xFFF))),
			fmt.Sprintf("addi x12, x12, %d", signExtend12(int(faultInstr&0xFFF))),
			"Step 2a: Load fault handler instruction lower bits")
	}
	emit(riscv.EncodeSw(12, 11, 0),
		"sw x12, 0(x11)",
		"Step 2a: Store fault handler (jal x0, 0) at 0x800")

	// Write NOP at 0x804 (second word of fault handler area)
	emit(riscv.EncodeSw(0, 11, 4),
		"sw x0, 4(x11)",
		"Step 2a: Store NOP at 0x804 (padding)")

	// Write timer_isr at 0x808: mret
	mretInstr := riscv.EncodeMret()
	emit(encodeLiUpper(12, mretInstr),
		fmt.Sprintf("lui x12, 0x%05X", mretInstr>>12),
		"Step 2a: Load mret instruction upper bits")
	if mretInstr&0xFFF != 0 {
		emit(riscv.EncodeAddi(12, 12, signExtend12(int(mretInstr&0xFFF))),
			fmt.Sprintf("addi x12, x12, %d", signExtend12(int(mretInstr&0xFFF))),
			"Step 2a: Load mret instruction lower bits")
	}
	emit(riscv.EncodeSw(12, 11, 8),
		"sw x12, 8(x11)",
		"Step 2a: Store timer_isr (mret) at 0x808")

	// Write keyboard_isr at 0x810: mret
	emit(riscv.EncodeSw(12, 11, 16),
		"sw x12, 16(x11)",
		"Step 2a: Store keyboard_isr (mret) at 0x810")

	// Write syscall_isr at 0x818: mret
	emit(riscv.EncodeSw(12, 11, 24),
		"sw x12, 24(x11)",
		"Step 2a: Store syscall_isr (mret) at 0x818")

	// Step 2b: Write IDT entries
	//
	// For each of 256 entries, store the ISR address at IDT_BASE + entry*8
	// and flags at IDT_BASE + entry*8 + 4.
	//
	// We use a loop with:
	//   x13 = current IDT entry address
	//   x14 = default handler address (0x800)
	//   x15 = entry counter
	//   x16 = IDT end address (256 * 8 = 2048 = 0x800)
	//   x17 = flags word (0x00000001 = present)
	//   x18 = timer ISR address (0x808)
	//   x19 = keyboard ISR address (0x810)
	//   x20 = syscall ISR address (0x818)
	//   x21 = timer entry offset (32 * 8 = 256 = 0x100)
	//   x22 = keyboard entry offset (33 * 8 = 264 = 0x108)
	//   x23 = syscall entry offset (128 * 8 = 1024 = 0x400)

	// x13 = IDT base (0x00000000) -- just use x0 + offset
	emit(riscv.EncodeAddi(13, 0, 0),
		"addi x13, x0, 0",
		"Step 2b: x13 = 0 (IDT base address)")

	// x14 = default handler address (0x800)
	emit(riscv.EncodeAddi(14, 0, 0x800-0x1000),
		fmt.Sprintf("addi x14, x0, %d", signExtend12(0x800)),
		"Step 2b: x14 = 0x800 (default fault handler) -- via sign extension trick")

	// Actually, 0x800 = 2048, which doesn't fit in 12-bit signed (-2048 to 2047).
	// We need LUI + ADDI or just use the fact that 0x800 is exactly -2048 in signed.
	// Wait: 0x800 as signed 12-bit = -2048. Let's use LUI for clarity.

	// Let me reconsider: ADDI has 12-bit signed immediate range [-2048, 2047].
	// 0x800 = 2048, which is just out of range. So we need:

	// Actually 0x800 sign-extended from 12 bits: 0x800 = 0b1000_0000_0000
	// That IS -2048 in signed 12-bit! And -2048 + 0 = -2048 = 0xFFFFF800 (negative).
	// That won't work. We need to load 0x800 = 2048 properly.

	// Strategy: lui x14, 1 gives x14 = 0x1000, then addi x14, x14, -0x800
	// 0x1000 - 0x800 = 0x800. Yes!

	// Let me redo the x14 loading:
	// Remove the last instruction and redo.

	// Actually, let me remove the incorrect instruction and redo the IDT section.
	// Since I already emitted a wrong one, let me just rebuild the instructions list.
	// This is getting complex. Let me use a cleaner approach.

	// I'll pop the last bad instruction and redo.
	instructions = instructions[:len(instructions)-1]
	address -= 4

	// Load 0x800 into x14: LUI x14, 1 => x14 = 0x1000; ADDI x14, x14, -2048
	emit(riscv.EncodeLui(14, 1),
		"lui x14, 0x00001",
		"Step 2b: x14 = 0x1000 (will subtract to get 0x800)")
	emit(riscv.EncodeAddi(14, 14, -2048),
		"addi x14, x14, -2048",
		"Step 2b: x14 = 0x800 (default fault handler address)")

	// x16 = IDT end (256 * 8 = 2048 = 0x800)
	emit(riscv.EncodeLui(16, 1),
		"lui x16, 0x00001",
		"Step 2b: x16 = 0x1000 (will subtract to get 0x800)")
	emit(riscv.EncodeAddi(16, 16, -2048),
		"addi x16, x16, -2048",
		"Step 2b: x16 = 0x800 (IDT end = 256 entries * 8 bytes)")

	// x17 = flags (1 = present)
	emit(riscv.EncodeAddi(17, 0, 1),
		"addi x17, x0, 1",
		"Step 2b: x17 = 1 (IDT flags: present)")

	// Load special ISR addresses
	// x18 = 0x808 (timer ISR)
	emit(riscv.EncodeLui(18, 1),
		"lui x18, 0x00001",
		"Step 2b: x18 = 0x1000")
	emit(riscv.EncodeAddi(18, 18, -2040),
		"addi x18, x18, -2040",
		"Step 2b: x18 = 0x808 (timer ISR address)")

	// x19 = 0x810 (keyboard ISR)
	emit(riscv.EncodeLui(19, 1),
		"lui x19, 0x00001",
		"Step 2b: x19 = 0x1000")
	emit(riscv.EncodeAddi(19, 19, -2032),
		"addi x19, x19, -2032",
		"Step 2b: x19 = 0x810 (keyboard ISR address)")

	// x20 = 0x818 (syscall ISR)
	emit(riscv.EncodeLui(20, 1),
		"lui x20, 0x00001",
		"Step 2b: x20 = 0x1000")
	emit(riscv.EncodeAddi(20, 20, -2024),
		"addi x20, x20, -2024",
		"Step 2b: x20 = 0x818 (syscall ISR address)")

	// Load special entry offsets for comparison
	// Entry 32 offset = 32*8 = 256 = 0x100
	emit(riscv.EncodeAddi(21, 0, 256),
		"addi x21, x0, 256",
		"Step 2b: x21 = 256 (entry 32 offset: timer)")

	// Entry 33 offset = 33*8 = 264 = 0x108
	emit(riscv.EncodeAddi(22, 0, 264),
		"addi x22, x0, 264",
		"Step 2b: x22 = 264 (entry 33 offset: keyboard)")

	// Entry 128 offset = 128*8 = 1024 = 0x400
	emit(riscv.EncodeAddi(23, 0, 1024),
		"addi x23, x0, 1024",
		"Step 2b: x23 = 1024 (entry 128 offset: syscall)")

	// IDT loop:
	//   Check if x13 == x21 (timer entry offset)
	//   Check if x13 == x22 (keyboard entry offset)
	//   Check if x13 == x23 (syscall entry offset)
	//   Default: store x14 (default handler)
	//   Store flags at +4
	//   Advance x13 by 8
	//   If x13 < x16, loop

	// idt_loop:
	loopStart := address

	// Check for timer entry
	emit(riscv.EncodeBeq(13, 21, 20),
		"beq x13, x21, +20",
		"Step 2b: If at timer entry (32), jump to store timer ISR")

	// Check for keyboard entry
	emit(riscv.EncodeBeq(13, 22, 24),
		"beq x13, x22, +24",
		"Step 2b: If at keyboard entry (33), jump to store keyboard ISR")

	// Check for syscall entry
	emit(riscv.EncodeBeq(13, 23, 28),
		"beq x13, x23, +28",
		"Step 2b: If at syscall entry (128), jump to store syscall ISR")

	// Default: store default handler
	emit(riscv.EncodeSw(14, 13, 0),
		"sw x14, 0(x13)",
		"Step 2b: Store default handler address at IDT[x13]")

	// Jump past the special stores
	emit(riscv.EncodeJal(0, 24),
		"jal x0, +24",
		"Step 2b: Skip special ISR stores")

	// timer_store: (offset +20 from beq)
	emit(riscv.EncodeSw(18, 13, 0),
		"sw x18, 0(x13)",
		"Step 2b: Store timer ISR address at IDT[32]")
	emit(riscv.EncodeJal(0, 16),
		"jal x0, +16",
		"Step 2b: Skip to flags store")

	// keyboard_store: (offset +24 from beq)
	emit(riscv.EncodeSw(19, 13, 0),
		"sw x19, 0(x13)",
		"Step 2b: Store keyboard ISR address at IDT[33]")
	emit(riscv.EncodeJal(0, 8),
		"jal x0, +8",
		"Step 2b: Skip to flags store")

	// syscall_store: (offset +28 from beq)
	emit(riscv.EncodeSw(20, 13, 0),
		"sw x20, 0(x13)",
		"Step 2b: Store syscall ISR address at IDT[128]")

	// flags_store: (common path)
	emit(riscv.EncodeSw(17, 13, 4),
		"sw x17, 4(x13)",
		"Step 2b: Store flags (present) at IDT[x13]+4")

	// Advance to next entry
	emit(riscv.EncodeAddi(13, 13, 8),
		"addi x13, x13, 8",
		"Step 2b: Advance to next IDT entry (x13 += 8)")

	// Loop condition
	loopOffset := int(loopStart - address)
	emit(riscv.EncodeBlt(13, 16, loopOffset),
		fmt.Sprintf("blt x13, x16, %d", loopOffset),
		"Step 2b: Loop back if more entries remain")

	// ═══════════════════════════════════════════════════════════════
	// Step 3: Write HardwareInfo
	// ═══════════════════════════════════════════════════════════════
	//
	// Populate the HardwareInfo struct at 0x00001000 with the values
	// discovered and configured during initialization.

	emit(riscv.EncodeLui(5, int(HardwareInfoAddress>>12)),
		fmt.Sprintf("lui x5, 0x%05X", HardwareInfoAddress>>12),
		"Step 3: x5 = 0x00001000 (HardwareInfo base)")

	// MemorySize (offset 0) -- stored in x8 from Step 1
	emit(riscv.EncodeSw(8, 5, 0),
		"sw x8, 0(x5)",
		"Step 3: HardwareInfo.MemorySize = x8")

	// DisplayColumns (offset 4)
	emit(riscv.EncodeAddi(6, 0, b.Config.DisplayColumns),
		fmt.Sprintf("addi x6, x0, %d", b.Config.DisplayColumns),
		fmt.Sprintf("Step 3: x6 = %d (display columns)", b.Config.DisplayColumns))
	emit(riscv.EncodeSw(6, 5, 4),
		"sw x6, 4(x5)",
		"Step 3: HardwareInfo.DisplayColumns")

	// DisplayRows (offset 8)
	emit(riscv.EncodeAddi(6, 0, b.Config.DisplayRows),
		fmt.Sprintf("addi x6, x0, %d", b.Config.DisplayRows),
		fmt.Sprintf("Step 3: x6 = %d (display rows)", b.Config.DisplayRows))
	emit(riscv.EncodeSw(6, 5, 8),
		"sw x6, 8(x5)",
		"Step 3: HardwareInfo.DisplayRows")

	// FramebufferBase (offset 12)
	fbUpper := b.Config.FramebufferBase >> 12
	fbLower := b.Config.FramebufferBase & 0xFFF
	emit(riscv.EncodeLui(6, int(fbUpper)),
		fmt.Sprintf("lui x6, 0x%05X", fbUpper),
		fmt.Sprintf("Step 3: x6 upper = 0x%05X000", fbUpper))
	if fbLower != 0 {
		emit(riscv.EncodeAddi(6, 6, signExtend12(int(fbLower))),
			fmt.Sprintf("addi x6, x6, %d", signExtend12(int(fbLower))),
			"Step 3: x6 lower bits for FramebufferBase")
	}
	emit(riscv.EncodeSw(6, 5, 12),
		"sw x6, 12(x5)",
		fmt.Sprintf("Step 3: HardwareInfo.FramebufferBase = 0x%08X", b.Config.FramebufferBase))

	// IDTBase (offset 16) -- always 0
	emit(riscv.EncodeSw(0, 5, 16),
		"sw x0, 16(x5)",
		"Step 3: HardwareInfo.IDTBase = 0x00000000")

	// IDTEntries (offset 20) -- 256
	emit(riscv.EncodeAddi(6, 0, 256),
		"addi x6, x0, 256",
		"Step 3: x6 = 256 (IDT entry count)")
	emit(riscv.EncodeSw(6, 5, 20),
		"sw x6, 20(x5)",
		"Step 3: HardwareInfo.IDTEntries = 256")

	// BootloaderEntry (offset 24)
	blUpper := b.Config.BootloaderEntry >> 12
	blLower := b.Config.BootloaderEntry & 0xFFF
	emit(riscv.EncodeLui(6, int(blUpper)),
		fmt.Sprintf("lui x6, 0x%05X", blUpper),
		fmt.Sprintf("Step 3: x6 = bootloader entry upper (0x%05X000)", blUpper))
	if blLower != 0 {
		emit(riscv.EncodeAddi(6, 6, signExtend12(int(blLower))),
			fmt.Sprintf("addi x6, x6, %d", signExtend12(int(blLower))),
			"Step 3: x6 lower bits for BootloaderEntry")
	}
	emit(riscv.EncodeSw(6, 5, 24),
		"sw x6, 24(x5)",
		fmt.Sprintf("Step 3: HardwareInfo.BootloaderEntry = 0x%08X", b.Config.BootloaderEntry))

	// ═══════════════════════════════════════════════════════════════
	// Step 4: Jump to Bootloader
	// ═══════════════════════════════════════════════════════════════
	//
	// Transfer control to the bootloader at the configured entry point.
	// Using JALR with x0 as the link register means this is a one-way
	// jump -- the BIOS does not expect to get control back.

	emit(riscv.EncodeLui(6, int(b.Config.BootloaderEntry>>12)),
		fmt.Sprintf("lui x6, 0x%05X", b.Config.BootloaderEntry>>12),
		"Step 4: x6 = bootloader entry upper bits")
	if b.Config.BootloaderEntry&0xFFF != 0 {
		emit(riscv.EncodeAddi(6, 6, signExtend12(int(b.Config.BootloaderEntry&0xFFF))),
			fmt.Sprintf("addi x6, x6, %d", signExtend12(int(b.Config.BootloaderEntry&0xFFF))),
			"Step 4: Add lower bits of bootloader entry")
	}
	emit(riscv.EncodeJalr(0, 6, 0),
		"jalr x0, x6, 0",
		fmt.Sprintf("Step 4: Jump to bootloader at 0x%08X (no return)", b.Config.BootloaderEntry))

	return instructions
}

// signExtend12 sign-extends a 12-bit value to a full int.
// RISC-V ADDI treats its immediate as a signed 12-bit value,
// so values >= 0x800 are negative.
func signExtend12(val int) int {
	val = val & 0xFFF
	if val >= 0x800 {
		return val - 0x1000
	}
	return val
}

// encodeLiUpper returns LUI instruction for loading the upper 20 bits
// of a 32-bit value into a register. If the lower 12 bits have bit 11
// set, the upper value is incremented by 1 to compensate for sign
// extension in the subsequent ADDI.
func encodeLiUpper(rd int, value uint32) uint32 {
	upper := value >> 12
	if value&0x800 != 0 {
		upper++
	}
	return riscv.EncodeLui(rd, int(upper))
}
