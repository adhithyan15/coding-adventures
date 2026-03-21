package bootloader

import (
	"testing"

	riscv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

// =========================================================================
// Code Generation Tests
// =========================================================================

// TestGenerateProducesValidBytes verifies that Generate() returns a byte
// slice whose length is a multiple of 4 (each RISC-V instruction is 4 bytes).
func TestGenerateProducesValidBytes(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 1024
	bl := NewBootloader(config)

	code := bl.Generate()
	if len(code) == 0 {
		t.Fatal("Generate() returned empty byte slice")
	}
	if len(code)%4 != 0 {
		t.Fatalf("Generate() length %d is not a multiple of 4", len(code))
	}
}

// TestGenerateIsDeterministic verifies that calling Generate() twice with
// the same config produces identical output.
func TestGenerateIsDeterministic(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 2048
	bl := NewBootloader(config)

	code1 := bl.Generate()
	code2 := bl.Generate()

	if len(code1) != len(code2) {
		t.Fatalf("Two Generate() calls produced different lengths: %d vs %d", len(code1), len(code2))
	}
	for i := range code1 {
		if code1[i] != code2[i] {
			t.Fatalf("Byte mismatch at offset %d: 0x%02X vs 0x%02X", i, code1[i], code2[i])
		}
	}
}

// TestAnnotatedOutput verifies that GenerateWithComments() produces entries
// with non-empty Assembly and Comment fields.
func TestAnnotatedOutput(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 512
	bl := NewBootloader(config)

	annotated := bl.GenerateWithComments()
	if len(annotated) == 0 {
		t.Fatal("GenerateWithComments() returned no instructions")
	}

	for i, a := range annotated {
		if a.Assembly == "" {
			t.Errorf("Instruction %d at 0x%08X has empty Assembly", i, a.Address)
		}
		if a.Comment == "" {
			t.Errorf("Instruction %d at 0x%08X has empty Comment", i, a.Address)
		}
		if a.MachineCode == 0 && a.Assembly != "addi t0, x0, 0" &&
			a.Assembly != "addi t1, x0, 0" && a.Assembly != "addi t2, x0, 0" &&
			a.Assembly != "addi sp, x0, 0" && a.Assembly != "addi x0, x0, 0" {
			// Machine code 0 is only valid for NOP-like instructions
		}
	}
}

// TestInstructionCount verifies InstructionCount returns a positive value.
func TestInstructionCount(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 4096
	bl := NewBootloader(config)

	count := bl.InstructionCount()
	if count <= 0 {
		t.Fatalf("InstructionCount() returned %d, expected positive", count)
	}
}

// TestEstimateCycles verifies cycle estimation scales with kernel size.
func TestEstimateCycles(t *testing.T) {
	config := DefaultBootloaderConfig()

	config.KernelSize = 1024
	bl1 := NewBootloader(config)
	cycles1 := bl1.EstimateCycles()

	config.KernelSize = 4096
	bl2 := NewBootloader(config)
	cycles2 := bl2.EstimateCycles()

	if cycles2 <= cycles1 {
		t.Fatalf("4KB kernel (%d cycles) should take more than 1KB (%d cycles)",
			cycles2, cycles1)
	}
}

// TestAddressesStartAtEntry verifies the first instruction address matches
// the configured entry address.
func TestAddressesStartAtEntry(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 256
	bl := NewBootloader(config)

	annotated := bl.GenerateWithComments()
	if annotated[0].Address != config.EntryAddress {
		t.Fatalf("First instruction at 0x%08X, expected 0x%08X",
			annotated[0].Address, config.EntryAddress)
	}
}

// TestAddressesAreSequential verifies instructions are at consecutive
// 4-byte-aligned addresses.
func TestAddressesAreSequential(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 512
	bl := NewBootloader(config)

	annotated := bl.GenerateWithComments()
	for i := 1; i < len(annotated); i++ {
		expected := annotated[i-1].Address + 4
		if annotated[i].Address != expected {
			t.Fatalf("Instruction %d at 0x%08X, expected 0x%08X",
				i, annotated[i].Address, expected)
		}
	}
}

// =========================================================================
// Execution Tests (on simulated CPU)
// =========================================================================

// TestCopyVerification populates a disk image with known kernel bytes, runs
// the bootloader on a RISC-V simulator, and verifies kernel bytes appear
// at the kernel load address.
func TestCopyVerification(t *testing.T) {
	kernelData := make([]byte, 64)
	for i := range kernelData {
		kernelData[i] = byte(i + 1) // Non-zero pattern
	}

	config := DefaultBootloaderConfig()
	config.KernelSize = uint32(len(kernelData))
	bl := NewBootloader(config)

	// Create a RISC-V simulator with enough memory.
	// We need memory for: bootloader code, kernel area, disk area, boot protocol.
	sim := riscv.NewRiscVSimulator(0x11000000) // big enough for disk mapped region

	// Write boot protocol with valid magic at 0x00001000.
	writeWord(sim, BootProtocolAddress, BootProtocolMagic)

	// Write kernel data to disk mapped region.
	diskKernelAddr := DiskMemoryMapBase + config.KernelDiskOffset
	for i, b := range kernelData {
		sim.CPU.Memory.WriteByte(int(diskKernelAddr)+i, b)
	}

	// Load bootloader code at entry address.
	bootCode := bl.Generate()
	for i, b := range bootCode {
		sim.CPU.Memory.WriteByte(int(config.EntryAddress)+i, b)
	}

	// Set PC to bootloader entry and run.
	sim.CPU.PC = int(config.EntryAddress)
	sim.CPU.Run(10000)

	// Verify kernel bytes were copied to the load address.
	for i, expected := range kernelData {
		actual := sim.CPU.Memory.ReadByte(int(config.KernelLoadAddress) + i)
		if actual != expected {
			t.Fatalf("Kernel byte %d: got 0x%02X, expected 0x%02X", i, actual, expected)
		}
	}
}

// TestStackPointerSet verifies the stack pointer is set correctly after boot.
func TestStackPointerSet(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 8 // Minimal kernel

	bl := NewBootloader(config)
	sim := riscv.NewRiscVSimulator(0x11000000)

	// Set up boot protocol.
	writeWord(sim, BootProtocolAddress, BootProtocolMagic)

	// Write a halt instruction at kernel entry so the simulator stops.
	// ecall with mtvec=0 causes halt.
	haltInstr := riscv.EncodeEcall()
	writeWord(sim, config.KernelLoadAddress, haltInstr)

	// Also write halt to disk so it gets copied.
	diskKernelAddr := DiskMemoryMapBase + config.KernelDiskOffset
	writeWord(sim, diskKernelAddr, haltInstr)
	writeWord(sim, diskKernelAddr+4, haltInstr)

	// Load and run bootloader.
	bootCode := bl.Generate()
	for i, b := range bootCode {
		sim.CPU.Memory.WriteByte(int(config.EntryAddress)+i, b)
	}
	sim.CPU.PC = int(config.EntryAddress)
	sim.CPU.Run(10000)

	// Check stack pointer (x2).
	sp := uint32(sim.CPU.Registers.Read(2))
	if sp != config.StackBase {
		t.Fatalf("Stack pointer = 0x%08X, expected 0x%08X", sp, config.StackBase)
	}
}

// TestPCAfterJump verifies the CPU's PC is at the kernel entry point after
// the bootloader's final JALR.
func TestPCAfterJump(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 4

	bl := NewBootloader(config)
	sim := riscv.NewRiscVSimulator(0x11000000)

	writeWord(sim, BootProtocolAddress, BootProtocolMagic)

	// Put ecall (halt) at kernel entry.
	haltInstr := riscv.EncodeEcall()
	writeWord(sim, config.KernelLoadAddress, haltInstr)
	writeWord(sim, DiskMemoryMapBase+config.KernelDiskOffset, haltInstr)

	bootCode := bl.Generate()
	for i, b := range bootCode {
		sim.CPU.Memory.WriteByte(int(config.EntryAddress)+i, b)
	}
	sim.CPU.PC = int(config.EntryAddress)
	sim.CPU.Run(10000)

	// The CPU should have halted at the kernel entry address.
	pc := uint32(sim.CPU.PC)
	if pc != config.KernelLoadAddress {
		t.Fatalf("PC = 0x%08X, expected 0x%08X (kernel entry)", pc, config.KernelLoadAddress)
	}
}

// TestMagicValidationHalt verifies the bootloader halts when the magic
// number is wrong.
func TestMagicValidationHalt(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 64
	bl := NewBootloader(config)

	sim := riscv.NewRiscVSimulator(0x11000000)

	// Write WRONG magic to boot protocol.
	writeWord(sim, BootProtocolAddress, 0xDEADBEEF)

	bootCode := bl.Generate()
	for i, b := range bootCode {
		sim.CPU.Memory.WriteByte(int(config.EntryAddress)+i, b)
	}
	sim.CPU.PC = int(config.EntryAddress)
	sim.CPU.Run(10000)

	// The CPU should NOT have reached the kernel entry.
	pc := uint32(sim.CPU.PC)
	if pc == config.KernelLoadAddress {
		t.Fatal("Bootloader should NOT jump to kernel with wrong magic")
	}

	// The bootloader should be stuck in the halt loop.
	// The halt instruction is a JAL x0, 0 which loops forever.
	// After 10000 cycles, the PC should be at the halt label.
	// Verify the kernel data was NOT copied.
	for i := 0; i < 64; i++ {
		val := sim.CPU.Memory.ReadByte(int(config.KernelLoadAddress) + i)
		if val != 0 {
			t.Fatalf("Kernel memory at offset %d should be 0 (no copy), got 0x%02X", i, val)
		}
	}
}

// TestVariableKernelSizes tests copying kernels of different sizes.
func TestVariableKernelSizes(t *testing.T) {
	sizes := []int{4, 64, 256, 1024, 4096}

	for _, size := range sizes {
		t.Run(string(rune(size)), func(t *testing.T) {
			kernelData := make([]byte, size)
			for i := range kernelData {
				kernelData[i] = byte((i * 7) & 0xFF) // Varied pattern
			}

			config := DefaultBootloaderConfig()
			config.KernelSize = uint32(size)
			bl := NewBootloader(config)

			sim := riscv.NewRiscVSimulator(0x11000000)
			writeWord(sim, BootProtocolAddress, BootProtocolMagic)

			diskKernelAddr := DiskMemoryMapBase + config.KernelDiskOffset
			for i, b := range kernelData {
				sim.CPU.Memory.WriteByte(int(diskKernelAddr)+i, b)
			}

			bootCode := bl.Generate()
			for i, b := range bootCode {
				sim.CPU.Memory.WriteByte(int(config.EntryAddress)+i, b)
			}

			// Put halt at kernel entry.
			haltBytes := riscv.Assemble([]uint32{riscv.EncodeEcall()})
			for i, b := range haltBytes {
				sim.CPU.Memory.WriteByte(int(config.KernelLoadAddress)+i, b)
			}

			sim.CPU.PC = int(config.EntryAddress)
			sim.CPU.Run(100000)

			// Verify all bytes were copied.
			for i, expected := range kernelData {
				actual := sim.CPU.Memory.ReadByte(int(config.KernelLoadAddress) + i)
				if actual != expected {
					t.Fatalf("Size %d: byte %d: got 0x%02X, expected 0x%02X",
						size, i, actual, expected)
				}
			}
		})
	}
}

// TestZeroKernelSize verifies the bootloader handles zero-size kernel
// gracefully (skips the copy loop).
func TestZeroKernelSize(t *testing.T) {
	config := DefaultBootloaderConfig()
	config.KernelSize = 0
	bl := NewBootloader(config)

	sim := riscv.NewRiscVSimulator(0x11000000)
	writeWord(sim, BootProtocolAddress, BootProtocolMagic)

	// Put halt at kernel entry.
	writeWord(sim, config.KernelLoadAddress, riscv.EncodeEcall())

	bootCode := bl.Generate()
	for i, b := range bootCode {
		sim.CPU.Memory.WriteByte(int(config.EntryAddress)+i, b)
	}

	sim.CPU.PC = int(config.EntryAddress)
	sim.CPU.Run(10000)

	// Should still reach kernel entry and halt.
	pc := uint32(sim.CPU.PC)
	if pc != config.KernelLoadAddress {
		t.Fatalf("PC = 0x%08X, expected 0x%08X", pc, config.KernelLoadAddress)
	}
}

// =========================================================================
// Disk Image Tests
// =========================================================================

// TestNewDiskImage verifies creation and initial state.
func TestNewDiskImage(t *testing.T) {
	disk := NewDiskImage(1024)
	if disk.Size() != 1024 {
		t.Fatalf("Disk size = %d, expected 1024", disk.Size())
	}
	// All bytes should be zero initially.
	for i := 0; i < 1024; i++ {
		if disk.ReadByteAt(i) != 0 {
			t.Fatalf("Byte %d should be 0 initially", i)
		}
	}
}

// TestLoadKernel verifies kernel bytes appear at the correct offset.
func TestLoadKernel(t *testing.T) {
	disk := NewDiskImage(DefaultDiskSize)
	kernel := []byte{0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE}
	disk.LoadKernel(kernel)

	for i, expected := range kernel {
		actual := disk.ReadByteAt(DiskKernelOffset + i)
		if actual != expected {
			t.Fatalf("Kernel byte %d: got 0x%02X, expected 0x%02X", i, actual, expected)
		}
	}
}

// TestLoadUserProgram verifies bytes appear at a custom offset.
func TestLoadUserProgram(t *testing.T) {
	disk := NewDiskImage(DefaultDiskSize)
	program := []byte{0x01, 0x02, 0x03, 0x04}
	disk.LoadUserProgram(program, DiskUserProgramBase)

	for i, expected := range program {
		actual := disk.ReadByteAt(DiskUserProgramBase + i)
		if actual != expected {
			t.Fatalf("Program byte %d: got 0x%02X, expected 0x%02X", i, actual, expected)
		}
	}
}

// TestDiskReadWord verifies little-endian word reading.
func TestDiskReadWord(t *testing.T) {
	disk := NewDiskImage(1024)
	// Write 0xDEADBEEF in little-endian at offset 0.
	disk.data[0] = 0xEF
	disk.data[1] = 0xBE
	disk.data[2] = 0xAD
	disk.data[3] = 0xDE

	word := disk.ReadWord(0)
	if word != 0xDEADBEEF {
		t.Fatalf("ReadWord(0) = 0x%08X, expected 0xDEADBEEF", word)
	}
}

// TestDiskData verifies Data() returns the full underlying slice.
func TestDiskData(t *testing.T) {
	disk := NewDiskImage(256)
	data := disk.Data()
	if len(data) != 256 {
		t.Fatalf("Data() length = %d, expected 256", len(data))
	}
	// Modify through Data() and verify via ReadByte.
	data[10] = 0x42
	if disk.ReadByteAt(10) != 0x42 {
		t.Fatal("Data() should return the backing slice")
	}
}

// TestDiskReadOutOfBounds verifies out-of-bounds reads return 0.
func TestDiskReadOutOfBounds(t *testing.T) {
	disk := NewDiskImage(16)
	if disk.ReadByteAt(100) != 0 {
		t.Fatal("Out-of-bounds ReadByte should return 0")
	}
	if disk.ReadWord(100) != 0 {
		t.Fatal("Out-of-bounds ReadWord should return 0")
	}
	if disk.ReadByteAt(-1) != 0 {
		t.Fatal("Negative ReadByte should return 0")
	}
}

// =========================================================================
// DefaultBootloaderConfig Tests
// =========================================================================

func TestDefaultBootloaderConfig(t *testing.T) {
	config := DefaultBootloaderConfig()
	if config.EntryAddress != DefaultEntryAddress {
		t.Errorf("EntryAddress = 0x%08X, expected 0x%08X", config.EntryAddress, DefaultEntryAddress)
	}
	if config.KernelDiskOffset != DefaultKernelDiskOffset {
		t.Errorf("KernelDiskOffset = 0x%08X, expected 0x%08X", config.KernelDiskOffset, DefaultKernelDiskOffset)
	}
	if config.KernelLoadAddress != DefaultKernelLoadAddress {
		t.Errorf("KernelLoadAddress = 0x%08X, expected 0x%08X", config.KernelLoadAddress, DefaultKernelLoadAddress)
	}
	if config.StackBase != DefaultStackBase {
		t.Errorf("StackBase = 0x%08X, expected 0x%08X", config.StackBase, DefaultStackBase)
	}
}

// =========================================================================
// Helpers
// =========================================================================

// writeWord writes a 32-bit little-endian word to the simulator's memory.
func writeWord(sim *riscv.RiscVSimulator, address uint32, value uint32) {
	sim.CPU.Memory.WriteByte(int(address), byte(value&0xFF))
	sim.CPU.Memory.WriteByte(int(address)+1, byte((value>>8)&0xFF))
	sim.CPU.Memory.WriteByte(int(address)+2, byte((value>>16)&0xFF))
	sim.CPU.Memory.WriteByte(int(address)+3, byte((value>>24)&0xFF))
}
