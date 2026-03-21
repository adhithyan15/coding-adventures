// Package bootloader generates RISC-V machine code for the second stage of
// the boot sequence: loading the OS kernel from disk into RAM.
//
// === What is a bootloader? ===
//
// A bootloader is a small program that runs after the BIOS (S01) finishes
// hardware initialization. Its job is deceptively simple but critical:
//
//   1. Read the boot protocol left by the BIOS at 0x00001000
//   2. Validate the magic number (0xB007CAFE) to ensure BIOS ran correctly
//   3. Copy the kernel binary from the "disk" region into kernel RAM
//   4. Set the stack pointer for the kernel
//   5. Jump to the kernel entry point (0x00020000)
//
// Once the bootloader transfers control to the kernel, its code is never
// executed again. It served as a delivery vehicle -- nothing more.
//
// === Analogy ===
//
// The bootloader is like a delivery person. The BIOS unlocked the front
// door and turned on the lights (initialized hardware, set up the IDT).
// Now the bootloader carries the OS into the building, sets it up in its
// office, hands it the keys, and leaves.
//
// === Memory Map ===
//
//	0x00001000: Boot protocol (written by BIOS, read by bootloader)
//	0x00010000: Bootloader code (this package generates these bytes)
//	0x00020000: Kernel code (bootloader copies kernel here)
//	0x0006FFF0: Kernel stack pointer (bootloader sets SP here)
//	0x10000000: Disk image base (memory-mapped disk region)
//	0x10080000: Kernel on disk (disk base + kernel offset)
package bootloader

import (
	"fmt"

	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

// =========================================================================
// Well-known addresses and constants
// =========================================================================

const (
	// DefaultEntryAddress is where the bootloader code lives in memory.
	// The BIOS jumps here after completing POST and writing the boot protocol.
	DefaultEntryAddress uint32 = 0x00010000

	// DefaultKernelDiskOffset is where the kernel binary begins within the
	// disk image. This is a conventional offset (512 KB into the disk) that
	// gives room for boot metadata.
	DefaultKernelDiskOffset uint32 = 0x00080000

	// DefaultKernelLoadAddress is where the kernel binary is copied to in RAM.
	// After the bootloader finishes, kernel code starts executing here.
	DefaultKernelLoadAddress uint32 = 0x00020000

	// DefaultStackBase is the initial stack pointer for the kernel.
	// The stack grows downward from this address.
	DefaultStackBase uint32 = 0x0006FFF0

	// DiskMemoryMapBase is where the disk image is memory-mapped in the
	// CPU's address space. The bootloader reads from DiskMemoryMapBase +
	// KernelDiskOffset to access the kernel bytes on "disk."
	DiskMemoryMapBase uint32 = 0x10000000

	// BootProtocolAddress is where the BIOS writes the boot protocol struct.
	// The bootloader reads hardware info from this location.
	BootProtocolAddress uint32 = 0x00001000

	// BootProtocolMagic is the expected magic number in the boot protocol.
	// "BOOT CAFE" -- a sanity check that the BIOS ran correctly.
	BootProtocolMagic uint32 = 0xB007CAFE
)

// =========================================================================
// BootloaderConfig -- configurable addresses for the bootloader
// =========================================================================

// BootloaderConfig holds the addresses and sizes that the bootloader uses.
// These can be customized for testing, but the defaults match the system's
// well-known memory layout.
type BootloaderConfig struct {
	// EntryAddress is where the bootloader code lives (default: 0x00010000).
	EntryAddress uint32

	// KernelDiskOffset is where the kernel starts in the disk image
	// (default: 0x00080000).
	KernelDiskOffset uint32

	// KernelLoadAddress is where to copy the kernel in RAM
	// (default: 0x00020000).
	KernelLoadAddress uint32

	// KernelSize is the size of the kernel binary in bytes.
	// Must be a multiple of 4 (word-aligned) for the copy loop.
	KernelSize uint32

	// StackBase is the initial stack pointer (default: 0x0006FFF0).
	StackBase uint32
}

// DefaultBootloaderConfig returns a configuration with conventional addresses.
// KernelSize defaults to 0 -- the caller must set it based on the actual
// kernel binary size.
func DefaultBootloaderConfig() BootloaderConfig {
	return BootloaderConfig{
		EntryAddress:      DefaultEntryAddress,
		KernelDiskOffset:  DefaultKernelDiskOffset,
		KernelLoadAddress: DefaultKernelLoadAddress,
		KernelSize:        0,
		StackBase:         DefaultStackBase,
	}
}

// =========================================================================
// AnnotatedInstruction -- machine code with human-readable explanation
// =========================================================================

// AnnotatedInstruction pairs a 32-bit RISC-V instruction with its assembly
// mnemonic and a comment explaining its role in the boot sequence.
//
// This is the primary debugging and educational output of the bootloader.
// A reader can trace through the annotated instructions to understand
// exactly what happens during the boot process.
type AnnotatedInstruction struct {
	// Address is the memory location of this instruction.
	Address uint32

	// MachineCode is the raw 32-bit RISC-V instruction word.
	MachineCode uint32

	// Assembly is the human-readable assembly mnemonic (e.g., "lw t3, 0(t0)").
	Assembly string

	// Comment explains what this instruction does in the boot sequence.
	Comment string
}

// =========================================================================
// Bootloader -- the code generator
// =========================================================================

// Bootloader generates RISC-V machine code that loads the kernel from disk
// into RAM and transfers control to it.
type Bootloader struct {
	Config BootloaderConfig
}

// NewBootloader creates a bootloader with the given configuration.
func NewBootloader(config BootloaderConfig) *Bootloader {
	return &Bootloader{Config: config}
}

// Generate produces the bootloader as a byte slice of RISC-V machine code.
// This is the binary that gets loaded at BootloaderConfig.EntryAddress.
func (b *Bootloader) Generate() []byte {
	annotated := b.GenerateWithComments()
	instructions := make([]uint32, len(annotated))
	for i, a := range annotated {
		instructions[i] = a.MachineCode
	}
	return riscv.Assemble(instructions)
}

// GenerateWithComments produces annotated instructions for debugging
// and educational display. Each instruction includes its address,
// machine code, assembly text, and a human-readable comment.
//
// The bootloader executes in four phases:
//
//	Phase 1: Validate the boot protocol magic number
//	Phase 2: Read boot parameters (kernel location, size, etc.)
//	Phase 3: Copy kernel from disk to RAM (word-by-word loop)
//	Phase 4: Set stack pointer and jump to kernel entry
func (b *Bootloader) GenerateWithComments() []AnnotatedInstruction {
	var instructions []AnnotatedInstruction
	address := b.Config.EntryAddress

	// Helper: emit an instruction and advance the address.
	emit := func(code uint32, asm, comment string) {
		instructions = append(instructions, AnnotatedInstruction{
			Address:     address,
			MachineCode: code,
			Assembly:    asm,
			Comment:     comment,
		})
		address += 4
	}

	// =====================================================================
	// Phase 1: Validate Boot Protocol
	// =====================================================================
	//
	// Load the boot protocol address (0x00001000) and read the magic number.
	// If the magic does not match 0xB007CAFE, the bootloader halts in an
	// infinite loop rather than proceeding with potentially corrupt data.
	//
	// Registers used:
	//   t0 (x5) = boot protocol address
	//   t1 (x6) = magic number read from memory
	//   t2 (x7) = expected magic number

	// Load boot protocol address into t0.
	// 0x00001000 = LUI 1 << 12, so LUI t0, 1
	emit(riscv.EncodeLui(5, 1),
		"lui t0, 0x00001",
		"Phase 1: t0 = 0x00001000 (boot protocol address)")

	// Read magic number from boot protocol.
	emit(riscv.EncodeLw(6, 5, 0),
		"lw t1, 0(t0)",
		"Phase 1: t1 = memory[0x00001000] (magic number)")

	// Load expected magic 0xB007CAFE into t2.
	// 0xB007CAFE: upper20 = 0xB007C, lower12 = 0xAFE
	// 0xAFE = 2814, bit 11 is set (> 2047), so signed = 0xAFE - 0x1000 = -1282
	// Compensated upper = 0xB007C + 1 = 0xB007D
	emit(riscv.EncodeLui(7, 0xB007D),
		"lui t2, 0xB007D",
		"Phase 1: t2 upper = 0xB007D000 (compensated for sign extension)")

	emit(riscv.EncodeAddi(7, 7, signExtend12(0xAFE)),
		fmt.Sprintf("addi t2, t2, %d", signExtend12(0xAFE)),
		"Phase 1: t2 = 0xB007CAFE (expected magic)")

	// Compare magic: if mismatch, jump to halt (Phase 4 end + a few instructions)
	// We'll use a forward branch. The halt is at the end of the generated code.
	// For now, emit a placeholder BNE that jumps forward to a halt loop.
	// The halt is: jal x0, 0 (infinite loop). We calculate the offset later.
	haltBranchIndex := len(instructions)
	emit(riscv.EncodeBne(6, 7, 0), // placeholder offset, will patch
		"bne t1, t2, halt",
		"Phase 1: If magic wrong, halt (infinite loop)")

	// =====================================================================
	// Phase 2: Read Boot Parameters
	// =====================================================================
	//
	// The boot protocol at 0x00001000 contains kernel location info.
	// We use hardcoded values from the config rather than reading them from
	// the boot protocol, since our simplified system pre-configures these.
	// This matches the pragmatic approach used elsewhere in the simulation.
	//
	// Registers:
	//   t0 (x5) = source address (disk mapped region + kernel offset)
	//   t1 (x6) = destination address (kernel load address)
	//   t2 (x7) = bytes remaining (kernel size)

	// Source: DiskMemoryMapBase + KernelDiskOffset
	source := DiskMemoryMapBase + b.Config.KernelDiskOffset
	emitLoadImmediate(&instructions, &address, 5, source,
		"Phase 2: t0 = source (disk mapped kernel location)")

	// Destination: KernelLoadAddress
	emitLoadImmediate(&instructions, &address, 6, b.Config.KernelLoadAddress,
		"Phase 2: t1 = destination (kernel load address)")

	// Bytes remaining: KernelSize
	emitLoadImmediate(&instructions, &address, 7, b.Config.KernelSize,
		"Phase 2: t2 = bytes remaining (kernel size)")

	// =====================================================================
	// Phase 3: Copy Kernel (word-by-word loop)
	// =====================================================================
	//
	// The copy loop transfers 4 bytes per iteration from the disk region
	// to kernel RAM. For a 4 KB kernel, that is 1024 iterations.
	//
	// Registers:
	//   t0 (x5) = source pointer (advances by 4 each iteration)
	//   t1 (x6) = destination pointer (advances by 4 each iteration)
	//   t2 (x7) = bytes remaining (decrements by 4 each iteration)
	//   t3 (x28) = temporary for the data word being copied

	// If kernel size is 0, skip the copy loop entirely.
	// beq t2, x0, skip_copy (offset = 6 instructions * 4 = 24 bytes)
	emit(riscv.EncodeBeq(7, 0, 24),
		"beq t2, x0, +24",
		"Phase 3: Skip copy if kernel size is 0")

	// copy_loop:
	copyLoopAddr := address

	emit(riscv.EncodeLw(28, 5, 0),
		"lw t3, 0(t0)",
		"Phase 3: Load 4 bytes from disk [t0]")

	emit(riscv.EncodeSw(28, 6, 0),
		"sw t3, 0(t1)",
		"Phase 3: Store 4 bytes to kernel RAM [t1]")

	emit(riscv.EncodeAddi(5, 5, 4),
		"addi t0, t0, 4",
		"Phase 3: Advance source pointer by 4")

	emit(riscv.EncodeAddi(6, 6, 4),
		"addi t1, t1, 4",
		"Phase 3: Advance destination pointer by 4")

	emit(riscv.EncodeAddi(7, 7, -4),
		"addi t2, t2, -4",
		"Phase 3: Decrement bytes remaining by 4")

	// Loop back to copy_loop if bytes remain.
	loopOffset := int(copyLoopAddr) - int(address)
	emit(riscv.EncodeBne(7, 0, loopOffset),
		fmt.Sprintf("bne t2, x0, %d", loopOffset),
		"Phase 3: Loop if bytes remain")

	// =====================================================================
	// Phase 4: Set Stack and Jump to Kernel
	// =====================================================================
	//
	// Set the stack pointer (sp/x2) and jump to the kernel entry point.
	// The JALR with rd=x0 means this is a one-way jump -- the bootloader
	// does not expect to get control back.

	// Load stack base into sp (x2).
	emitLoadImmediate(&instructions, &address, 2, b.Config.StackBase,
		"Phase 4: sp = stack base")

	// Load kernel entry address into t0.
	emitLoadImmediate(&instructions, &address, 5, b.Config.KernelLoadAddress,
		"Phase 4: t0 = kernel entry address")

	// Jump to kernel. jalr x0, t0, 0 -- no return.
	emit(riscv.EncodeJalr(0, 5, 0),
		"jalr x0, t0, 0",
		fmt.Sprintf("Phase 4: Jump to kernel at 0x%08X (no return)", b.Config.KernelLoadAddress))

	// halt: infinite loop for error cases (bad magic number).
	haltAddr := address
	emit(riscv.EncodeJal(0, 0),
		"jal x0, 0",
		"Halt: Infinite loop (bad boot protocol magic)")

	// =====================================================================
	// Patch the halt branch offset
	// =====================================================================
	//
	// Now that we know where the halt label is, go back and fix the BNE
	// instruction that branches to halt on magic mismatch.
	branchPC := instructions[haltBranchIndex].Address
	haltOffset := int(haltAddr) - int(branchPC)
	instructions[haltBranchIndex].MachineCode = riscv.EncodeBne(6, 7, haltOffset)
	instructions[haltBranchIndex].Assembly = fmt.Sprintf("bne t1, t2, +%d", haltOffset)

	return instructions
}

// InstructionCount returns the number of instructions in the bootloader.
func (b *Bootloader) InstructionCount() int {
	return len(b.GenerateWithComments())
}

// EstimateCycles returns an estimate of total cycles to copy the kernel,
// based on the copy loop iteration count. Each iteration is 6 instructions
// plus setup and teardown.
func (b *Bootloader) EstimateCycles() int {
	iterations := int(b.Config.KernelSize) / 4
	// 6 instructions per copy iteration + ~20 for setup/teardown
	return iterations*6 + 20
}

// =========================================================================
// Helpers
// =========================================================================

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

// emitLoadImmediate emits one or two instructions to load a 32-bit constant
// into a register. Uses LUI + ADDI for values that need both upper and lower
// bits, or just LUI/ADDI when one suffices.
//
// This handles the RISC-V sign-extension quirk: when the lower 12 bits have
// bit 11 set, the LUI value must be incremented by 1 to compensate for the
// negative sign extension of ADDI.
func emitLoadImmediate(instructions *[]AnnotatedInstruction, address *uint32, rd int, value uint32, comment string) {
	upper := value >> 12
	lower := value & 0xFFF

	// If lower 12 bits have bit 11 set, ADDI will sign-extend them as negative.
	// Compensate by adding 1 to the upper value.
	if lower >= 0x800 {
		upper++
	}

	regNames := map[int]string{
		2: "sp", 5: "t0", 6: "t1", 7: "t2",
	}
	regName := regNames[rd]
	if regName == "" {
		regName = fmt.Sprintf("x%d", rd)
	}

	if upper != 0 {
		*instructions = append(*instructions, AnnotatedInstruction{
			Address:     *address,
			MachineCode: riscv.EncodeLui(rd, int(upper)),
			Assembly:    fmt.Sprintf("lui %s, 0x%05X", regName, upper),
			Comment:     comment + fmt.Sprintf(" (upper: 0x%05X000)", upper),
		})
		*address += 4

		if lower != 0 {
			signedLower := signExtend12(int(lower))
			*instructions = append(*instructions, AnnotatedInstruction{
				Address:     *address,
				MachineCode: riscv.EncodeAddi(rd, rd, signedLower),
				Assembly:    fmt.Sprintf("addi %s, %s, %d", regName, regName, signedLower),
				Comment:     comment + fmt.Sprintf(" (lower: %d)", signedLower),
			})
			*address += 4
		}
	} else if lower != 0 {
		// Value fits in 12 bits -- just use ADDI from x0.
		signedLower := signExtend12(int(lower))
		*instructions = append(*instructions, AnnotatedInstruction{
			Address:     *address,
			MachineCode: riscv.EncodeAddi(rd, 0, signedLower),
			Assembly:    fmt.Sprintf("addi %s, x0, %d", regName, signedLower),
			Comment:     comment,
		})
		*address += 4
	} else {
		// Value is 0 -- load zero.
		*instructions = append(*instructions, AnnotatedInstruction{
			Address:     *address,
			MachineCode: riscv.EncodeAddi(rd, 0, 0),
			Assembly:    fmt.Sprintf("addi %s, x0, 0", regName),
			Comment:     comment + " (value = 0)",
		})
		*address += 4
	}
}
