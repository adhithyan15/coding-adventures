package blockram

import (
	"testing"
)

// =========================================================================
// SinglePortRAM Tests
// =========================================================================

func TestSinglePortRAM_WriteAndRead(t *testing.T) {
	ram := NewSinglePortRAM(256, 8, ReadFirst)

	data := []int{1, 1, 0, 0, 1, 0, 1, 0}
	zeros := []int{0, 0, 0, 0, 0, 0, 0, 0}

	// Write to address 0: clock LOW then HIGH (rising edge triggers write)
	ram.Tick(0, 0, data, 1) // no-op (clock low)
	ram.Tick(1, 0, data, 1) // rising edge: write

	// Read from address 0: clock LOW then HIGH
	ram.Tick(0, 0, zeros, 0)
	out := ram.Tick(1, 0, zeros, 0) // rising edge: read
	assertSliceEqual(t, "read after write", out, data)
}

func TestSinglePortRAM_ReadFirst(t *testing.T) {
	ram := NewSinglePortRAM(4, 4, ReadFirst)

	// Write initial data
	ram.Tick(0, 0, []int{1, 0, 1, 0}, 1)
	ram.Tick(1, 0, []int{1, 0, 1, 0}, 1) // write [1,0,1,0] to addr 0

	// Overwrite with new data — ReadFirst should return OLD value
	ram.Tick(0, 0, []int{0, 1, 0, 1}, 1)
	out := ram.Tick(1, 0, []int{0, 1, 0, 1}, 1)
	assertSliceEqual(t, "ReadFirst returns old", out, []int{1, 0, 1, 0})
}

func TestSinglePortRAM_WriteFirst(t *testing.T) {
	ram := NewSinglePortRAM(4, 4, WriteFirst)

	// Write initial data
	ram.Tick(0, 0, []int{1, 0, 1, 0}, 1)
	ram.Tick(1, 0, []int{1, 0, 1, 0}, 1)

	// Overwrite — WriteFirst should return NEW value
	ram.Tick(0, 0, []int{0, 1, 0, 1}, 1)
	out := ram.Tick(1, 0, []int{0, 1, 0, 1}, 1)
	assertSliceEqual(t, "WriteFirst returns new", out, []int{0, 1, 0, 1})
}

func TestSinglePortRAM_NoChange(t *testing.T) {
	ram := NewSinglePortRAM(4, 4, NoChange)

	// Read address 0 first (all zeros)
	ram.Tick(0, 0, []int{0, 0, 0, 0}, 0)
	out := ram.Tick(1, 0, []int{0, 0, 0, 0}, 0)
	assertSliceEqual(t, "initial read", out, []int{0, 0, 0, 0})

	// Write — NoChange should return previous read value
	ram.Tick(0, 0, []int{1, 1, 1, 1}, 1)
	out = ram.Tick(1, 0, []int{1, 1, 1, 1}, 1)
	assertSliceEqual(t, "NoChange returns prev", out, []int{0, 0, 0, 0})
}

func TestSinglePortRAM_MultipleAddresses(t *testing.T) {
	ram := NewSinglePortRAM(4, 4, ReadFirst)

	// Write to different addresses
	for addr := 0; addr < 4; addr++ {
		data := make([]int, 4)
		data[addr] = 1
		ram.Tick(0, addr, data, 1)
		ram.Tick(1, addr, data, 1)
	}

	// Read back each address
	for addr := 0; addr < 4; addr++ {
		zeros := make([]int, 4)
		ram.Tick(0, addr, zeros, 0)
		out := ram.Tick(1, addr, zeros, 0)
		expected := make([]int, 4)
		expected[addr] = 1
		assertSliceEqual(t, "multi addr read", out, expected)
	}
}

func TestSinglePortRAM_NoRisingEdge(t *testing.T) {
	ram := NewSinglePortRAM(4, 4, ReadFirst)

	// Without a rising edge, output should be the last read (all zeros)
	out := ram.Tick(0, 0, []int{1, 1, 1, 1}, 1) // no rising edge
	assertSliceEqual(t, "no rising edge", out, []int{0, 0, 0, 0})
}

func TestSinglePortRAM_Dump(t *testing.T) {
	ram := NewSinglePortRAM(2, 4, ReadFirst)

	ram.Tick(0, 0, []int{1, 0, 1, 0}, 1)
	ram.Tick(1, 0, []int{1, 0, 1, 0}, 1)

	dump := ram.Dump()
	if len(dump) != 2 {
		t.Fatalf("Dump length = %d, want 2", len(dump))
	}
	assertSliceEqual(t, "Dump[0]", dump[0], []int{1, 0, 1, 0})
	assertSliceEqual(t, "Dump[1]", dump[1], []int{0, 0, 0, 0})
}

func TestSinglePortRAM_Properties(t *testing.T) {
	ram := NewSinglePortRAM(256, 8, ReadFirst)
	if ram.Depth() != 256 {
		t.Errorf("Depth() = %d, want 256", ram.Depth())
	}
	if ram.Width() != 8 {
		t.Errorf("Width() = %d, want 8", ram.Width())
	}
}

func TestSinglePortRAM_Invalid(t *testing.T) {
	assertPanics(t, "depth=0", func() { NewSinglePortRAM(0, 8, ReadFirst) })
	assertPanics(t, "width=0", func() { NewSinglePortRAM(4, 0, ReadFirst) })

	ram := NewSinglePortRAM(4, 4, ReadFirst)
	assertPanics(t, "addr out of range", func() { ram.Tick(1, 4, []int{0, 0, 0, 0}, 0) })
	assertPanics(t, "addr negative", func() { ram.Tick(1, -1, []int{0, 0, 0, 0}, 0) })
	assertPanics(t, "data wrong length", func() { ram.Tick(1, 0, []int{0, 0}, 0) })
	assertPanics(t, "bad clock", func() { ram.Tick(2, 0, []int{0, 0, 0, 0}, 0) })
	assertPanics(t, "bad we", func() { ram.Tick(1, 0, []int{0, 0, 0, 0}, 2) })
	assertPanics(t, "bad data bit", func() { ram.Tick(1, 0, []int{0, 2, 0, 0}, 0) })
}

// =========================================================================
// DualPortRAM Tests
// =========================================================================

func TestDualPortRAM_IndependentPorts(t *testing.T) {
	ram := NewDualPortRAM(8, 4, ReadFirst, ReadFirst)

	zeros := []int{0, 0, 0, 0}
	dataA := []int{1, 0, 1, 0}
	dataB := []int{0, 1, 0, 1}

	// Write to addr 0 via port A, addr 1 via port B (simultaneously)
	ram.Tick(0, 0, dataA, 1, 1, dataB, 1)
	ram.Tick(1, 0, dataA, 1, 1, dataB, 1) // rising edge

	// Read back via opposite ports
	ram.Tick(0, 1, zeros, 0, 0, zeros, 0)
	outA, outB, err := ram.Tick(1, 1, zeros, 0, 0, zeros, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertSliceEqual(t, "Port A reads addr 1", outA, dataB)
	assertSliceEqual(t, "Port B reads addr 0", outB, dataA)
}

func TestDualPortRAM_WriteCollision(t *testing.T) {
	ram := NewDualPortRAM(4, 4, ReadFirst, ReadFirst)

	data := []int{1, 1, 1, 1}

	// Both ports write to same address → collision
	ram.Tick(0, 0, data, 1, 0, data, 1)
	_, _, err := ram.Tick(1, 0, data, 1, 0, data, 1)

	if err == nil {
		t.Fatal("expected WriteCollisionError, got nil")
	}
	wcErr, ok := err.(*WriteCollisionError)
	if !ok {
		t.Fatalf("expected *WriteCollisionError, got %T", err)
	}
	if wcErr.Address != 0 {
		t.Errorf("WriteCollisionError.Address = %d, want 0", wcErr.Address)
	}
}

func TestDualPortRAM_ReadReadSameAddress(t *testing.T) {
	ram := NewDualPortRAM(4, 4, ReadFirst, ReadFirst)

	data := []int{1, 0, 1, 0}
	zeros := []int{0, 0, 0, 0}

	// Write data to addr 0
	ram.Tick(0, 0, data, 1, 0, zeros, 0)
	ram.Tick(1, 0, data, 1, 0, zeros, 0)

	// Both ports read same address — should work fine
	ram.Tick(0, 0, zeros, 0, 0, zeros, 0)
	outA, outB, err := ram.Tick(1, 0, zeros, 0, 0, zeros, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertSliceEqual(t, "Port A", outA, data)
	assertSliceEqual(t, "Port B", outB, data)
}

func TestDualPortRAM_WriteReadDifferentAddress(t *testing.T) {
	ram := NewDualPortRAM(4, 4, ReadFirst, ReadFirst)

	dataA := []int{1, 1, 0, 0}
	zeros := []int{0, 0, 0, 0}

	// Port A writes to addr 0, Port B reads addr 1 (all zeros)
	ram.Tick(0, 0, dataA, 1, 1, zeros, 0)
	outA, outB, err := ram.Tick(1, 0, dataA, 1, 1, zeros, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Port A: ReadFirst → returns old value at addr 0 (zeros)
	assertSliceEqual(t, "Port A write", outA, zeros)
	// Port B: reads addr 1 (zeros)
	assertSliceEqual(t, "Port B read", outB, zeros)
}

func TestDualPortRAM_NoRisingEdge(t *testing.T) {
	ram := NewDualPortRAM(4, 4, ReadFirst, ReadFirst)
	zeros := []int{0, 0, 0, 0}

	// No rising edge → returns last read (zeros)
	outA, outB, err := ram.Tick(0, 0, zeros, 0, 0, zeros, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertSliceEqual(t, "no edge A", outA, zeros)
	assertSliceEqual(t, "no edge B", outB, zeros)
}

func TestDualPortRAM_Properties(t *testing.T) {
	ram := NewDualPortRAM(16, 8, ReadFirst, ReadFirst)
	if ram.Depth() != 16 {
		t.Errorf("Depth() = %d, want 16", ram.Depth())
	}
	if ram.Width() != 8 {
		t.Errorf("Width() = %d, want 8", ram.Width())
	}
}

func TestDualPortRAM_Invalid(t *testing.T) {
	assertPanics(t, "depth=0", func() { NewDualPortRAM(0, 4, ReadFirst, ReadFirst) })
	assertPanics(t, "width=0", func() { NewDualPortRAM(4, 0, ReadFirst, ReadFirst) })

	ram := NewDualPortRAM(4, 4, ReadFirst, ReadFirst)
	zeros := []int{0, 0, 0, 0}
	assertPanics(t, "bad clock", func() { ram.Tick(2, 0, zeros, 0, 0, zeros, 0) })
	assertPanics(t, "bad weA", func() { ram.Tick(1, 0, zeros, 2, 0, zeros, 0) })
	assertPanics(t, "bad weB", func() { ram.Tick(1, 0, zeros, 0, 0, zeros, 2) })
	assertPanics(t, "addrA out of range", func() { ram.Tick(1, 4, zeros, 0, 0, zeros, 0) })
	assertPanics(t, "addrB out of range", func() { ram.Tick(1, 0, zeros, 0, 4, zeros, 0) })
	assertPanics(t, "dataA wrong len", func() { ram.Tick(1, 0, []int{0}, 0, 0, zeros, 0) })
	assertPanics(t, "dataB wrong len", func() { ram.Tick(1, 0, zeros, 0, 0, []int{0}, 0) })
}

func TestDualPortRAM_WriteFirst(t *testing.T) {
	ram := NewDualPortRAM(4, 4, WriteFirst, WriteFirst)
	zeros := []int{0, 0, 0, 0}
	data := []int{1, 0, 1, 0}

	// Write via port A
	ram.Tick(0, 0, data, 1, 0, zeros, 0)
	outA, _, err := ram.Tick(1, 0, data, 1, 0, zeros, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// WriteFirst: returns new value
	assertSliceEqual(t, "WriteFirst A", outA, data)
}

func TestDualPortRAM_NoChange(t *testing.T) {
	ram := NewDualPortRAM(4, 4, NoChange, NoChange)
	zeros := []int{0, 0, 0, 0}
	data := []int{1, 1, 1, 1}

	// Write via port A — NoChange returns previous read (zeros)
	ram.Tick(0, 0, data, 1, 0, zeros, 0)
	outA, _, err := ram.Tick(1, 0, data, 1, 0, zeros, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	assertSliceEqual(t, "NoChange A", outA, zeros)
}

func TestWriteCollisionError_Message(t *testing.T) {
	err := &WriteCollisionError{Address: 42}
	expected := "blockram: write collision: both ports writing to address 42"
	if err.Error() != expected {
		t.Errorf("Error() = %q, want %q", err.Error(), expected)
	}
}
