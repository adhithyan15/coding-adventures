package gpucore

import (
	"testing"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
)

// =========================================================================
// Constructor tests
// =========================================================================

// TestNewFPRegisterFileDefaults verifies the default constructor creates
// a register file with 32 zero-initialized FP32 registers.
func TestNewFPRegisterFileDefaults(t *testing.T) {
	rf, err := NewFPRegisterFile(32, fp.FP32)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if rf.NumRegisters != 32 {
		t.Errorf("expected 32 registers, got %d", rf.NumRegisters)
	}
	if rf.Fmt != fp.FP32 {
		t.Errorf("expected FP32 format")
	}

	// All registers should be zero.
	for i := 0; i < 32; i++ {
		val, err := rf.ReadFloat(i)
		if err != nil {
			t.Fatalf("ReadFloat(%d) error: %v", i, err)
		}
		if val != 0.0 {
			t.Errorf("R%d should be 0.0, got %g", i, val)
		}
	}
}

// TestNewFPRegisterFileInvalidSize verifies that creating a register file
// with out-of-range sizes returns an error.
func TestNewFPRegisterFileInvalidSize(t *testing.T) {
	tests := []int{0, -1, 257, 1000}
	for _, n := range tests {
		_, err := NewFPRegisterFile(n, fp.FP32)
		if err == nil {
			t.Errorf("expected error for num_registers=%d", n)
		}
	}
}

// TestNewFPRegisterFileValidSizes verifies boundary values work.
func TestNewFPRegisterFileValidSizes(t *testing.T) {
	for _, n := range []int{1, 256} {
		rf, err := NewFPRegisterFile(n, fp.FP32)
		if err != nil {
			t.Errorf("unexpected error for num_registers=%d: %v", n, err)
		}
		if rf.NumRegisters != n {
			t.Errorf("expected %d registers, got %d", n, rf.NumRegisters)
		}
	}
}

// =========================================================================
// Read/Write tests
// =========================================================================

// TestReadWriteFloat verifies round-trip write/read of float values.
func TestReadWriteFloat(t *testing.T) {
	rf, _ := NewFPRegisterFile(32, fp.FP32)

	testValues := []struct {
		index int
		value float64
	}{
		{0, 3.14},
		{1, -2.71},
		{31, 42.0},
		{15, 0.5},
	}

	for _, tv := range testValues {
		if err := rf.WriteFloat(tv.index, tv.value); err != nil {
			t.Fatalf("WriteFloat(%d, %g) error: %v", tv.index, tv.value, err)
		}
		got, err := rf.ReadFloat(tv.index)
		if err != nil {
			t.Fatalf("ReadFloat(%d) error: %v", tv.index, err)
		}
		// FP32 has limited precision, so we check within tolerance.
		diff := got - tv.value
		if diff < 0 {
			diff = -diff
		}
		if diff > 0.001 {
			t.Errorf("R%d: wrote %g, read %g", tv.index, tv.value, got)
		}
	}
}

// TestReadWriteFloatBits verifies raw FloatBits read/write.
func TestReadWriteFloatBits(t *testing.T) {
	rf, _ := NewFPRegisterFile(32, fp.FP32)

	bits := fp.FloatToBits(1.0, fp.FP32)
	if err := rf.Write(0, bits); err != nil {
		t.Fatalf("Write error: %v", err)
	}

	got, err := rf.Read(0)
	if err != nil {
		t.Fatalf("Read error: %v", err)
	}

	gotF := fp.BitsToFloat(got)
	if gotF != 1.0 {
		t.Errorf("expected 1.0, got %g", gotF)
	}
}

// TestReadOutOfBounds verifies that reading an out-of-range register
// returns an error.
func TestReadOutOfBounds(t *testing.T) {
	rf, _ := NewFPRegisterFile(8, fp.FP32)

	_, err := rf.Read(-1)
	if err == nil {
		t.Error("expected error for index -1")
	}

	_, err = rf.Read(8)
	if err == nil {
		t.Error("expected error for index 8 (max is 7)")
	}
}

// TestWriteOutOfBounds verifies that writing to an out-of-range register
// returns an error.
func TestWriteOutOfBounds(t *testing.T) {
	rf, _ := NewFPRegisterFile(8, fp.FP32)

	err := rf.Write(-1, fp.FloatToBits(1.0, fp.FP32))
	if err == nil {
		t.Error("expected error for index -1")
	}

	err = rf.Write(8, fp.FloatToBits(1.0, fp.FP32))
	if err == nil {
		t.Error("expected error for index 8")
	}
}

// TestReadFloatOutOfBounds verifies error handling for ReadFloat.
func TestReadFloatOutOfBounds(t *testing.T) {
	rf, _ := NewFPRegisterFile(8, fp.FP32)
	_, err := rf.ReadFloat(100)
	if err == nil {
		t.Error("expected error for index 100")
	}
}

// TestWriteFloatOutOfBounds verifies error handling for WriteFloat.
func TestWriteFloatOutOfBounds(t *testing.T) {
	rf, _ := NewFPRegisterFile(8, fp.FP32)
	err := rf.WriteFloat(100, 1.0)
	if err == nil {
		t.Error("expected error for index 100")
	}
}

// =========================================================================
// Dump tests
// =========================================================================

// TestDumpNonZero verifies that Dump() only includes non-zero registers.
func TestDumpNonZero(t *testing.T) {
	rf, _ := NewFPRegisterFile(32, fp.FP32)
	_ = rf.WriteFloat(0, 1.0)
	_ = rf.WriteFloat(5, 2.5)

	dump := rf.Dump()
	if len(dump) != 2 {
		t.Errorf("expected 2 non-zero registers, got %d", len(dump))
	}
	if dump["R0"] != 1.0 {
		t.Errorf("expected R0=1.0, got %g", dump["R0"])
	}
	if dump["R5"] != 2.5 {
		t.Errorf("expected R5=2.5, got %g", dump["R5"])
	}
}

// TestDumpAll verifies that DumpAll() includes all registers.
func TestDumpAll(t *testing.T) {
	rf, _ := NewFPRegisterFile(8, fp.FP32)
	_ = rf.WriteFloat(0, 1.0)

	dump := rf.DumpAll()
	if len(dump) != 8 {
		t.Errorf("expected 8 registers in DumpAll, got %d", len(dump))
	}
}

// TestDumpAllZero verifies Dump() returns empty for all-zero register file.
func TestDumpAllZero(t *testing.T) {
	rf, _ := NewFPRegisterFile(32, fp.FP32)
	dump := rf.Dump()
	if len(dump) != 0 {
		t.Errorf("expected empty dump, got %d entries", len(dump))
	}
}

// =========================================================================
// String tests
// =========================================================================

// TestStringAllZero verifies the string representation when all registers
// are zero.
func TestStringAllZero(t *testing.T) {
	rf, _ := NewFPRegisterFile(32, fp.FP32)
	s := rf.String()
	if s == "" {
		t.Error("expected non-empty string")
	}
	t.Logf("All-zero string: %s", s)
}

// TestStringNonZero verifies the string representation with some non-zero
// registers.
func TestStringNonZero(t *testing.T) {
	rf, _ := NewFPRegisterFile(32, fp.FP32)
	_ = rf.WriteFloat(0, 1.0)
	_ = rf.WriteFloat(3, 2.5)
	s := rf.String()
	if s == "" {
		t.Error("expected non-empty string")
	}
	t.Logf("Non-zero string: %s", s)
}
