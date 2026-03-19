package cpusimulator

import (
	"testing"
)

// === Test helpers ===

// makeTestSparseMemory creates a SparseMemory with two non-contiguous regions:
//   - RAM at 0x00000000, 4096 bytes (read/write)
//   - ROM at 0xFFFF0000, 256 bytes (read-only)
//
// This mimics a minimal embedded system where code lives in low RAM and
// hardware registers or bootloader code lives at the top of the address space.
func makeTestSparseMemory() *SparseMemory {
	return NewSparseMemory([]MemoryRegion{
		{Base: 0x00000000, Size: 4096, Name: "RAM"},
		{Base: 0xFFFF0000, Size: 256, Name: "ROM", ReadOnly: true},
	})
}

// === Construction tests ===

func TestNewSparseMemory_AllocatesRegions(t *testing.T) {
	mem := makeTestSparseMemory()
	if mem.RegionCount() != 2 {
		t.Fatalf("expected 2 regions, got %d", mem.RegionCount())
	}

	// Verify RAM region
	if mem.Regions[0].Name != "RAM" {
		t.Errorf("expected region 0 name 'RAM', got %q", mem.Regions[0].Name)
	}
	if mem.Regions[0].Base != 0x00000000 {
		t.Errorf("expected RAM base 0x00000000, got 0x%08X", mem.Regions[0].Base)
	}
	if mem.Regions[0].Size != 4096 {
		t.Errorf("expected RAM size 4096, got %d", mem.Regions[0].Size)
	}
	if len(mem.Regions[0].Data) != 4096 {
		t.Errorf("expected RAM data length 4096, got %d", len(mem.Regions[0].Data))
	}
	if mem.Regions[0].ReadOnly {
		t.Error("expected RAM to be read/write")
	}

	// Verify ROM region
	if mem.Regions[1].Name != "ROM" {
		t.Errorf("expected region 1 name 'ROM', got %q", mem.Regions[1].Name)
	}
	if !mem.Regions[1].ReadOnly {
		t.Error("expected ROM to be read-only")
	}
}

func TestNewSparseMemory_PrePopulatedData(t *testing.T) {
	// Pre-populate ROM with data (like a bootloader image)
	romData := make([]byte, 64)
	romData[0] = 0xAA
	romData[63] = 0xBB

	mem := NewSparseMemory([]MemoryRegion{
		{Base: 0x1000, Size: 64, Data: romData, Name: "ROM", ReadOnly: true},
	})

	if mem.ReadByte(0x1000) != 0xAA {
		t.Errorf("expected 0xAA at ROM start, got 0x%02X", mem.ReadByte(0x1000))
	}
	if mem.ReadByte(0x103F) != 0xBB {
		t.Errorf("expected 0xBB at ROM end, got 0x%02X", mem.ReadByte(0x103F))
	}
}

func TestNewSparseMemory_ZeroInitialized(t *testing.T) {
	mem := makeTestSparseMemory()
	// All bytes should start at zero
	for i := uint32(0); i < 16; i++ {
		if mem.ReadByte(i) != 0 {
			t.Errorf("expected 0 at address 0x%X, got 0x%02X", i, mem.ReadByte(i))
		}
	}
}

// === Byte read/write tests ===

func TestSparseMemory_ReadWriteByte(t *testing.T) {
	mem := makeTestSparseMemory()

	// Write and read back several bytes in RAM
	mem.WriteByte(0x0000, 0x42)
	mem.WriteByte(0x0001, 0xFF)
	mem.WriteByte(0x0FFF, 0x99) // last byte of RAM

	if got := mem.ReadByte(0x0000); got != 0x42 {
		t.Errorf("expected 0x42, got 0x%02X", got)
	}
	if got := mem.ReadByte(0x0001); got != 0xFF {
		t.Errorf("expected 0xFF, got 0x%02X", got)
	}
	if got := mem.ReadByte(0x0FFF); got != 0x99 {
		t.Errorf("expected 0x99, got 0x%02X", got)
	}
}

func TestSparseMemory_ReadOnlyRegion_WriteSilentlyIgnored(t *testing.T) {
	mem := makeTestSparseMemory()

	// ROM starts at 0xFFFF0000, all zeros
	if got := mem.ReadByte(0xFFFF0000); got != 0 {
		t.Errorf("ROM should start as 0, got 0x%02X", got)
	}

	// Writing to ROM should be silently ignored
	mem.WriteByte(0xFFFF0000, 0xDE)
	if got := mem.ReadByte(0xFFFF0000); got != 0 {
		t.Errorf("ROM should still be 0 after write, got 0x%02X", got)
	}
}

// === Word read/write tests ===

func TestSparseMemory_ReadWriteWord_LittleEndian(t *testing.T) {
	mem := makeTestSparseMemory()

	// Write 0xDEADBEEF and verify little-endian byte layout
	mem.WriteWord(0x0100, 0xDEADBEEF)

	// Check individual bytes (little-endian: least significant byte first)
	if got := mem.ReadByte(0x0100); got != 0xEF {
		t.Errorf("byte 0: expected 0xEF, got 0x%02X", got)
	}
	if got := mem.ReadByte(0x0101); got != 0xBE {
		t.Errorf("byte 1: expected 0xBE, got 0x%02X", got)
	}
	if got := mem.ReadByte(0x0102); got != 0xAD {
		t.Errorf("byte 2: expected 0xAD, got 0x%02X", got)
	}
	if got := mem.ReadByte(0x0103); got != 0xDE {
		t.Errorf("byte 3: expected 0xDE, got 0x%02X", got)
	}

	// Read back as word
	if got := mem.ReadWord(0x0100); got != 0xDEADBEEF {
		t.Errorf("expected 0xDEADBEEF, got 0x%08X", got)
	}
}

func TestSparseMemory_WriteWord_ReadOnly(t *testing.T) {
	mem := makeTestSparseMemory()

	// Attempting to write a word to ROM should be silently ignored
	mem.WriteWord(0xFFFF0000, 0x12345678)
	if got := mem.ReadWord(0xFFFF0000); got != 0x00000000 {
		t.Errorf("ROM word should be 0 after ignored write, got 0x%08X", got)
	}
}

func TestSparseMemory_WordRoundTrip(t *testing.T) {
	mem := makeTestSparseMemory()

	// Test various word values including edge cases
	testCases := []struct {
		addr uint32
		val  uint32
	}{
		{0x0000, 0x00000000},
		{0x0004, 0xFFFFFFFF},
		{0x0008, 0x00000001},
		{0x000C, 0x80000000},
		{0x0010, 0x7FFFFFFF},
		{0x0014, 0x01020304},
	}

	for _, tc := range testCases {
		mem.WriteWord(tc.addr, tc.val)
		got := mem.ReadWord(tc.addr)
		if got != tc.val {
			t.Errorf("at 0x%04X: wrote 0x%08X, read back 0x%08X", tc.addr, tc.val, got)
		}
	}
}

// === LoadBytes tests ===

func TestSparseMemory_LoadBytes(t *testing.T) {
	mem := makeTestSparseMemory()

	data := []byte{0x01, 0x02, 0x03, 0x04, 0x05}
	mem.LoadBytes(0x0200, data)

	for i, expected := range data {
		got := mem.ReadByte(uint32(0x0200 + i))
		if got != expected {
			t.Errorf("at offset %d: expected 0x%02X, got 0x%02X", i, expected, got)
		}
	}
}

func TestSparseMemory_LoadBytes_IntoReadOnlyRegion(t *testing.T) {
	mem := makeTestSparseMemory()

	// LoadBytes should bypass the ReadOnly check — this is for initialization
	data := []byte{0xAA, 0xBB, 0xCC, 0xDD}
	mem.LoadBytes(0xFFFF0000, data)

	// The data should have been loaded successfully
	if got := mem.ReadByte(0xFFFF0000); got != 0xAA {
		t.Errorf("expected 0xAA, got 0x%02X", got)
	}
	if got := mem.ReadByte(0xFFFF0003); got != 0xDD {
		t.Errorf("expected 0xDD, got 0x%02X", got)
	}

	// But subsequent writes via WriteByte should still be ignored
	mem.WriteByte(0xFFFF0000, 0x00)
	if got := mem.ReadByte(0xFFFF0000); got != 0xAA {
		t.Errorf("ROM should still be 0xAA after WriteByte, got 0x%02X", got)
	}
}

// === Dump tests ===

func TestSparseMemory_Dump(t *testing.T) {
	mem := makeTestSparseMemory()

	mem.WriteByte(0x0010, 0xAA)
	mem.WriteByte(0x0011, 0xBB)
	mem.WriteByte(0x0012, 0xCC)

	dumped := mem.Dump(0x0010, 3)
	if len(dumped) != 3 {
		t.Fatalf("expected dump length 3, got %d", len(dumped))
	}
	if dumped[0] != 0xAA || dumped[1] != 0xBB || dumped[2] != 0xCC {
		t.Errorf("unexpected dump: %v", dumped)
	}
}

func TestSparseMemory_Dump_IsCopy(t *testing.T) {
	mem := makeTestSparseMemory()
	mem.WriteByte(0x0000, 0xFF)

	dumped := mem.Dump(0x0000, 4)
	dumped[0] = 0x00 // modifying the copy

	// Original should be unchanged
	if got := mem.ReadByte(0x0000); got != 0xFF {
		t.Errorf("Dump returned a reference, not a copy: original changed to 0x%02X", got)
	}
}

// === Unmapped address tests ===

func TestSparseMemory_ReadByte_UnmappedPanics(t *testing.T) {
	mem := makeTestSparseMemory()

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on unmapped read, got none")
		}
	}()

	// Address 0x80000000 is between our RAM (ending at 0x1000) and ROM (starting at 0xFFFF0000)
	_ = mem.ReadByte(0x80000000)
}

func TestSparseMemory_WriteByte_UnmappedPanics(t *testing.T) {
	mem := makeTestSparseMemory()

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on unmapped write, got none")
		}
	}()

	mem.WriteByte(0x80000000, 0xFF)
}

func TestSparseMemory_ReadWord_UnmappedPanics(t *testing.T) {
	mem := makeTestSparseMemory()

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on unmapped word read, got none")
		}
	}()

	_ = mem.ReadWord(0x80000000)
}

func TestSparseMemory_WriteWord_UnmappedPanics(t *testing.T) {
	mem := makeTestSparseMemory()

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on unmapped word write, got none")
		}
	}()

	mem.WriteWord(0x80000000, 0xDEAD)
}

func TestSparseMemory_ReadWord_CrossesBoundaryPanics(t *testing.T) {
	mem := makeTestSparseMemory()

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic when word read crosses region boundary, got none")
		}
	}()

	// RAM ends at 0x1000. A 4-byte read at 0x0FFE would need bytes at 0x0FFE, 0x0FFF, 0x1000, 0x1001.
	// 0x1000 and 0x1001 are unmapped, so this should panic.
	_ = mem.ReadWord(0x0FFE)
}

// === Multiple non-contiguous region tests ===

func TestSparseMemory_MultipleRegions(t *testing.T) {
	mem := NewSparseMemory([]MemoryRegion{
		{Base: 0x00000000, Size: 1024, Name: "RAM"},
		{Base: 0x10000000, Size: 256, Name: "SRAM"},
		{Base: 0xFFFF0000, Size: 128, Name: "IO"},
	})

	// Write to each region and verify isolation
	mem.WriteByte(0x00000000, 0x11)
	mem.WriteByte(0x10000000, 0x22)
	mem.WriteByte(0xFFFF0000, 0x33)

	if got := mem.ReadByte(0x00000000); got != 0x11 {
		t.Errorf("RAM: expected 0x11, got 0x%02X", got)
	}
	if got := mem.ReadByte(0x10000000); got != 0x22 {
		t.Errorf("SRAM: expected 0x22, got 0x%02X", got)
	}
	if got := mem.ReadByte(0xFFFF0000); got != 0x33 {
		t.Errorf("IO: expected 0x33, got 0x%02X", got)
	}
}

// === High address region tests ===
//
// These tests verify that arithmetic with addresses near 0xFFFFFFFF
// works correctly (no integer overflow issues).

func TestSparseMemory_HighAddressRegion(t *testing.T) {
	mem := NewSparseMemory([]MemoryRegion{
		{Base: 0xFFFB0000, Size: 0x50000, Name: "HIGH_IO"},
	})

	// Write near the very top of the 32-bit address space
	mem.WriteByte(0xFFFB0000, 0x01)          // start of region
	mem.WriteByte(0xFFFFFFFE, 0xFE)          // near end
	mem.WriteWord(0xFFFFFFFC, 0xCAFEBABE)    // last 4 bytes of region

	if got := mem.ReadByte(0xFFFB0000); got != 0x01 {
		t.Errorf("expected 0x01, got 0x%02X", got)
	}
	if got := mem.ReadWord(0xFFFFFFFC); got != 0xCAFEBABE {
		t.Errorf("expected 0xCAFEBABE, got 0x%08X", got)
	}
}

// === LoadBytes as program loader ===

func TestSparseMemory_LoadProgram(t *testing.T) {
	mem := NewSparseMemory([]MemoryRegion{
		{Base: 0x00000000, Size: 0x10000, Name: "RAM"},
	})

	// Simulate loading a small RISC-V program (4 instructions)
	// Each instruction is 4 bytes, little-endian
	program := []byte{
		0x93, 0x00, 0xA0, 0x02, // addi x1, x0, 42
		0x13, 0x01, 0x30, 0x00, // addi x2, x0, 3
		0xB3, 0x01, 0x21, 0x00, // add x3, x2, x2 (actually x1+x2 depends on encoding)
		0x73, 0x00, 0x00, 0x00, // ecall
	}
	mem.LoadBytes(0x0000, program)

	// Verify first instruction word
	word0 := mem.ReadWord(0x0000)
	if word0 != 0x02A00093 {
		t.Errorf("expected instruction 0x02A00093, got 0x%08X", word0)
	}
}

// === findRegion edge cases ===

func TestSparseMemory_LoadBytes_UnmappedPanics(t *testing.T) {
	mem := makeTestSparseMemory()

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on LoadBytes to unmapped address, got none")
		}
	}()

	mem.LoadBytes(0x80000000, []byte{0x01, 0x02})
}

func TestSparseMemory_Dump_UnmappedPanics(t *testing.T) {
	mem := makeTestSparseMemory()

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on Dump from unmapped address, got none")
		}
	}()

	_ = mem.Dump(0x80000000, 4)
}

func TestSparseMemory_EmptyRegions(t *testing.T) {
	mem := NewSparseMemory([]MemoryRegion{})

	defer func() {
		r := recover()
		if r == nil {
			t.Fatal("expected panic on read from empty SparseMemory, got none")
		}
	}()

	_ = mem.ReadByte(0x0000)
}

func TestSparseMemory_RegionCount(t *testing.T) {
	mem := NewSparseMemory([]MemoryRegion{
		{Base: 0, Size: 16, Name: "A"},
		{Base: 0x1000, Size: 16, Name: "B"},
		{Base: 0x2000, Size: 16, Name: "C"},
	})
	if got := mem.RegionCount(); got != 3 {
		t.Errorf("expected 3 regions, got %d", got)
	}
}
