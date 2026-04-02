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
package bootloader

import (
	"fmt"

	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

const (
	DefaultEntryAddress      uint32 = 0x00010000
	DefaultKernelDiskOffset  uint32 = 0x00080000
	DefaultKernelLoadAddress uint32 = 0x00020000
	DefaultStackBase         uint32 = 0x0006FFF0
	DiskMemoryMapBase        uint32 = 0x10000000
	BootProtocolAddress      uint32 = 0x00001000
	BootProtocolMagic        uint32 = 0xB007CAFE
)

// BootloaderConfig holds the addresses and sizes that the bootloader uses.
type BootloaderConfig struct {
	EntryAddress      uint32
	KernelDiskOffset  uint32
	KernelLoadAddress uint32
	KernelSize        uint32
	StackBase         uint32
}

// DefaultBootloaderConfig returns a configuration with conventional addresses.
func DefaultBootloaderConfig() BootloaderConfig {
	result, _ := StartNew[BootloaderConfig]("bootloader.DefaultBootloaderConfig", BootloaderConfig{},
		func(op *Operation[BootloaderConfig], rf *ResultFactory[BootloaderConfig]) *OperationResult[BootloaderConfig] {
			cfg := BootloaderConfig{
				EntryAddress:      DefaultEntryAddress,
				KernelDiskOffset:  DefaultKernelDiskOffset,
				KernelLoadAddress: DefaultKernelLoadAddress,
				KernelSize:        0,
				StackBase:         DefaultStackBase,
			}
			return rf.Generate(true, false, cfg)
		}).GetResult()
	return result
}

// AnnotatedInstruction pairs a 32-bit RISC-V instruction with its assembly
// mnemonic and a comment explaining its role in the boot sequence.
type AnnotatedInstruction struct {
	Address     uint32
	MachineCode uint32
	Assembly    string
	Comment     string
}

// Bootloader generates RISC-V machine code that loads the kernel from disk
// into RAM and transfers control to it.
type Bootloader struct {
	Config BootloaderConfig
}

// NewBootloader creates a bootloader with the given configuration.
func NewBootloader(config BootloaderConfig) *Bootloader {
	result, _ := StartNew[*Bootloader]("bootloader.NewBootloader", nil,
		func(op *Operation[*Bootloader], rf *ResultFactory[*Bootloader]) *OperationResult[*Bootloader] {
			return rf.Generate(true, false, &Bootloader{Config: config})
		}).GetResult()
	return result
}

// Generate produces the bootloader as a byte slice of RISC-V machine code.
func (b *Bootloader) Generate() []byte {
	result, _ := StartNew[[]byte]("bootloader.Bootloader.Generate", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			annotated := b.GenerateWithComments()
			instructions := make([]uint32, len(annotated))
			for i, a := range annotated {
				instructions[i] = a.MachineCode
			}
			return rf.Generate(true, false, riscv.Assemble(instructions))
		}).GetResult()
	return result
}

// GenerateWithComments produces annotated instructions for debugging and
// educational display.
func (b *Bootloader) GenerateWithComments() []AnnotatedInstruction {
	result, _ := StartNew[[]AnnotatedInstruction]("bootloader.Bootloader.GenerateWithComments", nil,
		func(op *Operation[[]AnnotatedInstruction], rf *ResultFactory[[]AnnotatedInstruction]) *OperationResult[[]AnnotatedInstruction] {
			return rf.Generate(true, false, b.generateWithCommentsInternal())
		}).GetResult()
	return result
}

// generateWithCommentsInternal is the internal implementation.
func (b *Bootloader) generateWithCommentsInternal() []AnnotatedInstruction {
	var instructions []AnnotatedInstruction
	address := b.Config.EntryAddress

	emit := func(code uint32, asm, comment string) {
		instructions = append(instructions, AnnotatedInstruction{
			Address:     address,
			MachineCode: code,
			Assembly:    asm,
			Comment:     comment,
		})
		address += 4
	}

	emit(riscv.EncodeLui(5, 1),
		"lui t0, 0x00001",
		"Phase 1: t0 = 0x00001000 (boot protocol address)")

	emit(riscv.EncodeLw(6, 5, 0),
		"lw t1, 0(t0)",
		"Phase 1: t1 = memory[0x00001000] (magic number)")

	emit(riscv.EncodeLui(7, 0xB007D),
		"lui t2, 0xB007D",
		"Phase 1: t2 upper = 0xB007D000 (compensated for sign extension)")

	emit(riscv.EncodeAddi(7, 7, signExtend12(0xAFE)),
		fmt.Sprintf("addi t2, t2, %d", signExtend12(0xAFE)),
		"Phase 1: t2 = 0xB007CAFE (expected magic)")

	haltBranchIndex := len(instructions)
	emit(riscv.EncodeBne(6, 7, 0),
		"bne t1, t2, halt",
		"Phase 1: If magic wrong, halt (infinite loop)")

	source := DiskMemoryMapBase + b.Config.KernelDiskOffset
	emitLoadImmediate(&instructions, &address, 5, source,
		"Phase 2: t0 = source (disk mapped kernel location)")

	emitLoadImmediate(&instructions, &address, 6, b.Config.KernelLoadAddress,
		"Phase 2: t1 = destination (kernel load address)")

	emitLoadImmediate(&instructions, &address, 7, b.Config.KernelSize,
		"Phase 2: t2 = bytes remaining (kernel size)")

	emit(riscv.EncodeBeq(7, 0, 24),
		"beq t2, x0, +24",
		"Phase 3: Skip copy if kernel size is 0")

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

	loopOffset := int(copyLoopAddr) - int(address)
	emit(riscv.EncodeBne(7, 0, loopOffset),
		fmt.Sprintf("bne t2, x0, %d", loopOffset),
		"Phase 3: Loop if bytes remain")

	emitLoadImmediate(&instructions, &address, 2, b.Config.StackBase,
		"Phase 4: sp = stack base")

	emitLoadImmediate(&instructions, &address, 5, b.Config.KernelLoadAddress,
		"Phase 4: t0 = kernel entry address")

	emit(riscv.EncodeJalr(0, 5, 0),
		"jalr x0, t0, 0",
		fmt.Sprintf("Phase 4: Jump to kernel at 0x%08X (no return)", b.Config.KernelLoadAddress))

	haltAddr := address
	emit(riscv.EncodeJal(0, 0),
		"jal x0, 0",
		"Halt: Infinite loop (bad boot protocol magic)")

	branchPC := instructions[haltBranchIndex].Address
	haltOffset := int(haltAddr) - int(branchPC)
	instructions[haltBranchIndex].MachineCode = riscv.EncodeBne(6, 7, haltOffset)
	instructions[haltBranchIndex].Assembly = fmt.Sprintf("bne t1, t2, +%d", haltOffset)

	return instructions
}

// InstructionCount returns the number of instructions in the bootloader.
func (b *Bootloader) InstructionCount() int {
	result, _ := StartNew[int]("bootloader.Bootloader.InstructionCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(b.GenerateWithComments()))
		}).GetResult()
	return result
}

// EstimateCycles returns an estimate of total cycles to copy the kernel.
func (b *Bootloader) EstimateCycles() int {
	result, _ := StartNew[int]("bootloader.Bootloader.EstimateCycles", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			iterations := int(b.Config.KernelSize) / 4
			return rf.Generate(true, false, iterations*6+20)
		}).GetResult()
	return result
}

func signExtend12(val int) int {
	val = val & 0xFFF
	if val >= 0x800 {
		return val - 0x1000
	}
	return val
}

func emitLoadImmediate(instructions *[]AnnotatedInstruction, address *uint32, rd int, value uint32, comment string) {
	upper := value >> 12
	lower := value & 0xFFF

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
		signedLower := signExtend12(int(lower))
		*instructions = append(*instructions, AnnotatedInstruction{
			Address:     *address,
			MachineCode: riscv.EncodeAddi(rd, 0, signedLower),
			Assembly:    fmt.Sprintf("addi %s, x0, %d", regName, signedLower),
			Comment:     comment,
		})
		*address += 4
	} else {
		*instructions = append(*instructions, AnnotatedInstruction{
			Address:     *address,
			MachineCode: riscv.EncodeAddi(rd, 0, 0),
			Assembly:    fmt.Sprintf("addi %s, x0, 0", regName),
			Comment:     comment + " (value = 0)",
		})
		*address += 4
	}
}
