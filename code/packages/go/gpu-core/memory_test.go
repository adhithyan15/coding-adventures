package gpucore

import (
	"testing"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
)

// =========================================================================
// Constructor tests
// =========================================================================

// TestNewLocalMemory verifies basic creation.
func TestNewLocalMemory(t *testing.T) {
	mem, err := NewLocalMemory(4096)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if mem.Size != 4096 {
		t.Errorf("expected size 4096, got %d", mem.Size)
	}
}

// TestNewLocalMemoryInvalidSize verifies error for invalid sizes.
func TestNewLocalMemoryInvalidSize(t *testing.T) {
	_, err := NewLocalMemory(0)
	if err == nil {
		t.Error("expected error for size 0")
	}
	_, err = NewLocalMemory(-1)
	if err == nil {
		t.Error("expected error for size -1")
	}
}

// =========================================================================
// Raw byte access tests
// =========================================================================

// TestReadWriteByte verifies single byte read/write.
func TestReadWriteByte(t *testing.T) {
	mem, _ := NewLocalMemory(256)

	if err := mem.WriteByte(0, 0x42); err != nil {
		t.Fatalf("WriteByte error: %v", err)
	}
	got, err := mem.ReadByte(0)
	if err != nil {
		t.Fatalf("ReadByte error: %v", err)
	}
	if got != 0x42 {
		t.Errorf("expected 0x42, got 0x%02X", got)
	}
}

// TestReadWriteBytes verifies multi-byte read/write.
func TestReadWriteBytes(t *testing.T) {
	mem, _ := NewLocalMemory(256)

	data := []byte{0xDE, 0xAD, 0xBE, 0xEF}
	if err := mem.WriteBytes(10, data); err != nil {
		t.Fatalf("WriteBytes error: %v", err)
	}
	got, err := mem.ReadBytes(10, 4)
	if err != nil {
		t.Fatalf("ReadBytes error: %v", err)
	}
	for i, b := range got {
		if b != data[i] {
			t.Errorf("byte %d: expected 0x%02X, got 0x%02X", i, data[i], b)
		}
	}
}

// TestByteOutOfBounds verifies bounds checking for byte operations.
func TestByteOutOfBounds(t *testing.T) {
	mem, _ := NewLocalMemory(16)

	// Read past end
	_, err := mem.ReadByte(16)
	if err == nil {
		t.Error("expected error for read at address 16 (size=16)")
	}

	// Write past end
	if err := mem.WriteByte(16, 0); err == nil {
		t.Error("expected error for write at address 16 (size=16)")
	}

	// Negative address
	_, err = mem.ReadByte(-1)
	if err == nil {
		t.Error("expected error for negative address")
	}

	// Multi-byte past end
	_, err = mem.ReadBytes(14, 4)
	if err == nil {
		t.Error("expected error for read spanning past end")
	}

	// Multi-byte write past end
	if err := mem.WriteBytes(14, []byte{1, 2, 3, 4}); err == nil {
		t.Error("expected error for write spanning past end")
	}
}

// =========================================================================
// Float access tests
// =========================================================================

// TestStoreLoadFloat verifies round-trip float storage in FP32.
func TestStoreLoadFloat(t *testing.T) {
	mem, _ := NewLocalMemory(4096)

	testValues := []struct {
		addr  int
		value float64
	}{
		{0, 1.0},
		{4, 3.14},
		{8, -2.71},
		{12, 0.0},
		{100, 42.0},
	}

	for _, tv := range testValues {
		if err := mem.StoreGoFloat(tv.addr, tv.value, fp.FP32); err != nil {
			t.Fatalf("StoreGoFloat(%d, %g) error: %v", tv.addr, tv.value, err)
		}
		got, err := mem.LoadFloatAsGo(tv.addr, fp.FP32)
		if err != nil {
			t.Fatalf("LoadFloatAsGo(%d) error: %v", tv.addr, err)
		}
		diff := got - tv.value
		if diff < 0 {
			diff = -diff
		}
		if diff > 0.001 {
			t.Errorf("addr %d: stored %g, loaded %g", tv.addr, tv.value, got)
		}
	}
}

// TestStoreLoadFloatBits verifies raw FloatBits storage.
func TestStoreLoadFloatBits(t *testing.T) {
	mem, _ := NewLocalMemory(4096)

	bits := fp.FloatToBits(2.5, fp.FP32)
	if err := mem.StoreFloat(0, bits); err != nil {
		t.Fatalf("StoreFloat error: %v", err)
	}
	got, err := mem.LoadFloat(0, fp.FP32)
	if err != nil {
		t.Fatalf("LoadFloat error: %v", err)
	}
	gotF := fp.BitsToFloat(got)
	if gotF != 2.5 {
		t.Errorf("expected 2.5, got %g", gotF)
	}
}

// TestFloatOutOfBounds verifies bounds checking for float operations.
func TestFloatOutOfBounds(t *testing.T) {
	mem, _ := NewLocalMemory(8)

	// Try to store at address that would overflow
	err := mem.StoreGoFloat(6, 1.0, fp.FP32)
	if err == nil {
		t.Error("expected error for FP32 store at addr 6 in 8-byte memory")
	}

	_, err = mem.LoadFloat(6, fp.FP32)
	if err == nil {
		t.Error("expected error for FP32 load at addr 6 in 8-byte memory")
	}

	_, err = mem.LoadFloatAsGo(6, fp.FP32)
	if err == nil {
		t.Error("expected error for LoadFloatAsGo at addr 6 in 8-byte memory")
	}
}

// =========================================================================
// Dump and String tests
// =========================================================================

// TestDump verifies the memory dump function.
func TestMemoryDump(t *testing.T) {
	mem, _ := NewLocalMemory(256)
	_ = mem.WriteByte(0, 0xAA)
	_ = mem.WriteByte(1, 0xBB)

	dump := mem.Dump(0, 4)
	if len(dump) != 4 {
		t.Errorf("expected 4 bytes, got %d", len(dump))
	}
	if dump[0] != 0xAA || dump[1] != 0xBB {
		t.Errorf("expected [AA, BB, ...], got [%02X, %02X, ...]", dump[0], dump[1])
	}
}

// TestDumpPastEnd verifies dump is clamped to memory size.
func TestDumpPastEnd(t *testing.T) {
	mem, _ := NewLocalMemory(8)
	dump := mem.Dump(4, 100)
	if len(dump) != 4 {
		t.Errorf("expected 4 bytes (clamped), got %d", len(dump))
	}
}

// TestMemoryString verifies the string representation.
func TestMemoryString(t *testing.T) {
	mem, _ := NewLocalMemory(256)
	s := mem.String()
	if s == "" {
		t.Error("expected non-empty string")
	}
	t.Logf("Memory string: %s", s)

	_ = mem.WriteByte(0, 0xFF)
	s = mem.String()
	t.Logf("Memory string (with data): %s", s)
}

// =========================================================================
// FP16 format test
// =========================================================================

// TestStoreLoadFP16 verifies float storage in FP16 format.
func TestStoreLoadFP16(t *testing.T) {
	mem, _ := NewLocalMemory(256)

	// FP16 uses 2 bytes per value
	if err := mem.StoreGoFloat(0, 1.0, fp.FP16); err != nil {
		t.Fatalf("StoreGoFloat FP16 error: %v", err)
	}
	got, err := mem.LoadFloatAsGo(0, fp.FP16)
	if err != nil {
		t.Fatalf("LoadFloatAsGo FP16 error: %v", err)
	}
	if got != 1.0 {
		t.Errorf("expected 1.0, got %g", got)
	}
}
