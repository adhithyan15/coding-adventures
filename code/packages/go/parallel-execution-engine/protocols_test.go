package parallelexecutionengine

// Tests for the protocols: ExecutionModel, EngineTrace, DivergenceInfo,
// DataflowInfo, and the ParallelExecutionEngine interface.

import (
	"strings"
	"testing"
)

// =========================================================================
// ExecutionModel tests
// =========================================================================

// TestExecutionModelString verifies that each execution model has a correct
// string representation. These strings appear in traces and logs, so they
// must be stable and human-readable.
func TestExecutionModelString(t *testing.T) {
	tests := []struct {
		model ExecutionModel
		want  string
	}{
		{SIMT, "SIMT"},
		{SIMD, "SIMD"},
		{Systolic, "SYSTOLIC"},
		{ScheduledMAC, "SCHEDULED_MAC"},
		{VLIW, "VLIW"},
	}

	for _, tt := range tests {
		got := tt.model.String()
		if got != tt.want {
			t.Errorf("ExecutionModel(%d).String() = %q, want %q", int(tt.model), got, tt.want)
		}
	}
}

// TestExecutionModelUnknown verifies that an out-of-range execution model
// returns a descriptive fallback string.
func TestExecutionModelUnknown(t *testing.T) {
	got := ExecutionModel(999).String()
	if !strings.Contains(got, "UNKNOWN") {
		t.Errorf("unknown ExecutionModel.String() = %q, expected to contain 'UNKNOWN'", got)
	}
}

// =========================================================================
// DivergenceInfo tests
// =========================================================================

// TestDivergenceInfoCreation verifies that DivergenceInfo can be created
// and its fields accessed correctly.
func TestDivergenceInfoCreation(t *testing.T) {
	di := DivergenceInfo{
		ActiveMaskBefore: []bool{true, true, true, true},
		ActiveMaskAfter:  []bool{true, true, false, false},
		ReconvergencePC:  10,
		DivergenceDepth:  1,
	}

	if len(di.ActiveMaskBefore) != 4 {
		t.Errorf("ActiveMaskBefore length = %d, want 4", len(di.ActiveMaskBefore))
	}
	if di.ReconvergencePC != 10 {
		t.Errorf("ReconvergencePC = %d, want 10", di.ReconvergencePC)
	}
	if di.DivergenceDepth != 1 {
		t.Errorf("DivergenceDepth = %d, want 1", di.DivergenceDepth)
	}
}

// =========================================================================
// DataflowInfo tests
// =========================================================================

// TestDataflowInfoCreation verifies that DataflowInfo can be created
// with PE states and data positions.
func TestDataflowInfoCreation(t *testing.T) {
	df := DataflowInfo{
		PEStates: [][]string{
			{"acc=0", "acc=1.5"},
			{"acc=2.0", "acc=3.0"},
		},
		DataPositions: map[string][2]int{
			"input_0": {0, 1},
		},
	}

	if len(df.PEStates) != 2 {
		t.Errorf("PEStates rows = %d, want 2", len(df.PEStates))
	}
	if pos, ok := df.DataPositions["input_0"]; !ok || pos[0] != 0 || pos[1] != 1 {
		t.Errorf("DataPositions['input_0'] = %v, want [0,1]", pos)
	}
}

// =========================================================================
// EngineTrace tests
// =========================================================================

// TestEngineTraceFormat verifies that EngineTrace.Format() produces a
// human-readable multi-line string with cycle, engine, utilization,
// and per-unit details.
func TestEngineTraceFormat(t *testing.T) {
	trace := EngineTrace{
		Cycle:       3,
		EngineName:  "WarpEngine",
		Model:       SIMT,
		Description: "FADD R2, R0, R1 -- 3/4 threads active",
		UnitTraces: map[int]string{
			0: "R2 = 1.0 + 2.0 = 3.0",
			1: "R2 = 3.0 + 4.0 = 7.0",
			2: "(masked -- diverged)",
			3: "R2 = 5.0 + 6.0 = 11.0",
		},
		ActiveMask:  []bool{true, true, false, true},
		ActiveCount: 3,
		TotalCount:  4,
		Utilization: 0.75,
	}

	formatted := trace.Format()

	// Verify key components are present in the output.
	if !strings.Contains(formatted, "Cycle 3") {
		t.Error("Format() missing cycle number")
	}
	if !strings.Contains(formatted, "WarpEngine") {
		t.Error("Format() missing engine name")
	}
	if !strings.Contains(formatted, "SIMT") {
		t.Error("Format() missing execution model")
	}
	if !strings.Contains(formatted, "75.0%") {
		t.Error("Format() missing utilization percentage")
	}
	if !strings.Contains(formatted, "3/4 active") {
		t.Error("Format() missing active count")
	}
	if !strings.Contains(formatted, "Unit 0") {
		t.Error("Format() missing unit traces")
	}
}

// TestEngineTraceFormatWithDivergence verifies that divergence info
// appears in the formatted trace.
func TestEngineTraceFormatWithDivergence(t *testing.T) {
	trace := EngineTrace{
		Cycle:       1,
		EngineName:  "WarpEngine",
		Model:       SIMT,
		Description: "test",
		UnitTraces:  map[int]string{},
		ActiveMask:  []bool{true},
		ActiveCount: 1,
		TotalCount:  1,
		Utilization: 1.0,
		Divergence: &DivergenceInfo{
			ActiveMaskBefore: []bool{true, true},
			ActiveMaskAfter:  []bool{true, false},
			ReconvergencePC:  5,
			DivergenceDepth:  1,
		},
	}

	formatted := trace.Format()
	if !strings.Contains(formatted, "Divergence") {
		t.Error("Format() should include divergence info when present")
	}
	if !strings.Contains(formatted, "depth=1") {
		t.Error("Format() should include divergence depth")
	}
}

// TestEngineTraceZeroUtilization verifies correct formatting at 0% utilization.
func TestEngineTraceZeroUtilization(t *testing.T) {
	trace := EngineTrace{
		Cycle:       1,
		EngineName:  "TestEngine",
		Model:       SIMD,
		Description: "idle",
		UnitTraces:  map[int]string{},
		ActiveMask:  []bool{false, false},
		ActiveCount: 0,
		TotalCount:  2,
		Utilization: 0.0,
	}

	formatted := trace.Format()
	if !strings.Contains(formatted, "0.0%") {
		t.Error("Format() should show 0.0% for zero utilization")
	}
}

// TestEngineTraceFullUtilization verifies correct formatting at 100%.
func TestEngineTraceFullUtilization(t *testing.T) {
	trace := EngineTrace{
		Cycle:       1,
		EngineName:  "TestEngine",
		Model:       SIMD,
		Description: "full",
		UnitTraces:  map[int]string{0: "ok", 1: "ok"},
		ActiveMask:  []bool{true, true},
		ActiveCount: 2,
		TotalCount:  2,
		Utilization: 1.0,
	}

	formatted := trace.Format()
	if !strings.Contains(formatted, "100.0%") {
		t.Error("Format() should show 100.0% for full utilization")
	}
}
