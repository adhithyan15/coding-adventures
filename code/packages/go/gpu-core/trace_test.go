package gpucore

import (
	"strings"
	"testing"
)

// =========================================================================
// GPUCoreTrace tests
// =========================================================================

// TestTraceFormat verifies the pretty-print format for a typical trace.
func TestTraceFormat(t *testing.T) {
	trace := GPUCoreTrace{
		Cycle:       3,
		PC:          2,
		Inst:        Fmul(2, 0, 1),
		Description: "R2 = R0 * R1 = 3 * 4 = 12",
		NextPC:      3,
		Halted:      false,
		RegistersChanged: map[string]float64{"R2": 12.0},
		MemoryChanged:    map[int]float64{},
	}

	output := trace.Format()
	t.Logf("Trace format:\n%s", output)

	// Verify key components are present
	if !strings.Contains(output, "Cycle 3") {
		t.Error("expected 'Cycle 3' in output")
	}
	if !strings.Contains(output, "PC=2") {
		t.Error("expected 'PC=2' in output")
	}
	if !strings.Contains(output, "FMUL R2, R0, R1") {
		t.Error("expected 'FMUL R2, R0, R1' in output")
	}
	if !strings.Contains(output, "R2 = R0 * R1") {
		t.Error("expected description in output")
	}
	if !strings.Contains(output, "Next PC: 3") {
		t.Error("expected 'Next PC: 3' in output")
	}
	if !strings.Contains(output, "Registers:") {
		t.Error("expected 'Registers:' in output")
	}
}

// TestTraceFormatHalted verifies that halted traces show "HALTED".
func TestTraceFormatHalted(t *testing.T) {
	trace := GPUCoreTrace{
		Cycle:            1,
		PC:               0,
		Inst:             Halt(),
		Description:      "Halted",
		NextPC:           0,
		Halted:           true,
		RegistersChanged: map[string]float64{},
		MemoryChanged:    map[int]float64{},
	}

	output := trace.Format()
	if !strings.Contains(output, "HALTED") {
		t.Error("expected 'HALTED' in output")
	}
	// Should NOT contain "Next PC" when halted
	if strings.Contains(output, "Next PC:") {
		t.Error("should not contain 'Next PC:' when halted")
	}
}

// TestTraceFormatWithMemory verifies that memory changes are shown.
func TestTraceFormatWithMemory(t *testing.T) {
	trace := GPUCoreTrace{
		Cycle:            2,
		PC:               1,
		Inst:             Store(0, 1, 0),
		Description:      "Mem[0] = R1 = 3.14",
		NextPC:           2,
		Halted:           false,
		RegistersChanged: map[string]float64{},
		MemoryChanged:    map[int]float64{0: 3.14},
	}

	output := trace.Format()
	if !strings.Contains(output, "Memory:") {
		t.Error("expected 'Memory:' in output")
	}
}

// TestTraceFormatNoChanges verifies output when nothing changed.
func TestTraceFormatNoChanges(t *testing.T) {
	trace := GPUCoreTrace{
		Cycle:            1,
		PC:               0,
		Inst:             Nop(),
		Description:      "No operation",
		NextPC:           1,
		Halted:           false,
		RegistersChanged: map[string]float64{},
		MemoryChanged:    map[int]float64{},
	}

	output := trace.Format()
	if strings.Contains(output, "Registers:") {
		t.Error("should not contain 'Registers:' when none changed")
	}
	if strings.Contains(output, "Memory:") {
		t.Error("should not contain 'Memory:' when none changed")
	}
}
