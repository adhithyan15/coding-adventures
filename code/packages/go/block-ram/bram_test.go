package blockram

import (
	"testing"
)

// =========================================================================
// ConfigurableBRAM Tests
// =========================================================================

func TestConfigurableBRAM_NewAndProperties(t *testing.T) {
	bram := NewConfigurableBRAM(1024, 8)
	if bram.Depth() != 128 {
		t.Errorf("Depth() = %d, want 128", bram.Depth())
	}
	if bram.Width() != 8 {
		t.Errorf("Width() = %d, want 8", bram.Width())
	}
	if bram.TotalBits() != 1024 {
		t.Errorf("TotalBits() = %d, want 1024", bram.TotalBits())
	}
}

func TestConfigurableBRAM_WriteAndReadPortA(t *testing.T) {
	bram := NewConfigurableBRAM(256, 8)

	data := []int{1, 0, 1, 0, 0, 1, 0, 1}
	zeros := []int{0, 0, 0, 0, 0, 0, 0, 0}

	// Write via port A
	bram.TickA(0, 0, data, 1)
	bram.TickA(1, 0, data, 1) // rising edge

	// Read via port A
	bram.TickA(0, 0, zeros, 0)
	out := bram.TickA(1, 0, zeros, 0)
	assertSliceEqual(t, "TickA read", out, data)
}

func TestConfigurableBRAM_WriteAndReadPortB(t *testing.T) {
	bram := NewConfigurableBRAM(256, 8)

	data := []int{0, 1, 0, 1, 1, 0, 1, 0}
	zeros := []int{0, 0, 0, 0, 0, 0, 0, 0}

	// Write via port B
	bram.TickB(0, 0, data, 1)
	bram.TickB(1, 0, data, 1) // rising edge

	// Read via port B
	bram.TickB(0, 0, zeros, 0)
	out := bram.TickB(1, 0, zeros, 0)
	assertSliceEqual(t, "TickB read", out, data)
}

func TestConfigurableBRAM_Reconfigure(t *testing.T) {
	bram := NewConfigurableBRAM(1024, 8)
	if bram.Depth() != 128 || bram.Width() != 8 {
		t.Errorf("Initial config: depth=%d width=%d, want 128/8", bram.Depth(), bram.Width())
	}

	// Write some data
	data := []int{1, 1, 1, 1, 1, 1, 1, 1}
	bram.TickA(0, 0, data, 1)
	bram.TickA(1, 0, data, 1)

	// Reconfigure to 16-bit width
	bram.Reconfigure(16)
	if bram.Depth() != 64 || bram.Width() != 16 {
		t.Errorf("After reconfigure: depth=%d width=%d, want 64/16", bram.Depth(), bram.Width())
	}

	// Old data should be cleared
	zeros := make([]int, 16)
	bram.TickA(0, 0, zeros, 0)
	out := bram.TickA(1, 0, zeros, 0)
	assertSliceEqual(t, "after reconfigure read", out, zeros)
}

func TestConfigurableBRAM_ReconfigureWidths(t *testing.T) {
	bram := NewConfigurableBRAM(1024, 8)

	// Reconfigure to 1-bit wide
	bram.Reconfigure(1)
	if bram.Depth() != 1024 || bram.Width() != 1 {
		t.Errorf("1-bit: depth=%d width=%d, want 1024/1", bram.Depth(), bram.Width())
	}

	// Reconfigure to 32-bit wide
	bram.Reconfigure(32)
	if bram.Depth() != 32 || bram.Width() != 32 {
		t.Errorf("32-bit: depth=%d width=%d, want 32/32", bram.Depth(), bram.Width())
	}
}

func TestConfigurableBRAM_Invalid(t *testing.T) {
	assertPanics(t, "totalBits=0", func() { NewConfigurableBRAM(0, 8) })
	assertPanics(t, "width=0", func() { NewConfigurableBRAM(1024, 0) })
	assertPanics(t, "width doesn't divide", func() { NewConfigurableBRAM(1024, 3) })

	bram := NewConfigurableBRAM(1024, 8)
	assertPanics(t, "Reconfigure width=0", func() { bram.Reconfigure(0) })
	assertPanics(t, "Reconfigure doesn't divide", func() { bram.Reconfigure(3) })
}

func TestConfigurableBRAM_MultipleAddresses(t *testing.T) {
	bram := NewConfigurableBRAM(64, 4)
	// depth = 16

	// Write to several addresses via port A
	for addr := 0; addr < 4; addr++ {
		data := make([]int, 4)
		data[addr] = 1
		bram.TickA(0, addr, data, 1)
		bram.TickA(1, addr, data, 1)
	}

	// Read back
	for addr := 0; addr < 4; addr++ {
		zeros := []int{0, 0, 0, 0}
		bram.TickA(0, addr, zeros, 0)
		out := bram.TickA(1, addr, zeros, 0)
		expected := make([]int, 4)
		expected[addr] = 1
		assertSliceEqual(t, "multi addr", out, expected)
	}
}
