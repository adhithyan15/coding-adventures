package computeunit

import (
	"strings"
	"testing"

	pee "github.com/adhithyan15/coding-adventures/code/packages/go/parallel-execution-engine"
)

// =========================================================================
// Architecture tests
// =========================================================================

func TestArchitectureString(t *testing.T) {
	tests := []struct {
		arch Architecture
		want string
	}{
		{ArchNvidiaSM, "nvidia_sm"},
		{ArchAMDCU, "amd_cu"},
		{ArchGoogleMXU, "google_mxu"},
		{ArchIntelXeCore, "intel_xe_core"},
		{ArchAppleANECore, "apple_ane_core"},
	}
	for _, tt := range tests {
		if got := tt.arch.String(); got != tt.want {
			t.Errorf("Architecture(%d).String() = %q, want %q", tt.arch, got, tt.want)
		}
	}
}

func TestArchitectureStringUnknown(t *testing.T) {
	unknown := Architecture(999)
	got := unknown.String()
	if got == "" {
		t.Error("Unknown architecture should produce a non-empty string")
	}
}

// =========================================================================
// WarpState tests
// =========================================================================

func TestWarpStateString(t *testing.T) {
	tests := []struct {
		state WarpState
		want  string
	}{
		{WarpStateReady, "ready"},
		{WarpStateRunning, "running"},
		{WarpStateStalledMemory, "stalled_memory"},
		{WarpStateStalledBarrier, "stalled_barrier"},
		{WarpStateStalledDependency, "stalled_dependency"},
		{WarpStateCompleted, "completed"},
	}
	for _, tt := range tests {
		if got := tt.state.String(); got != tt.want {
			t.Errorf("WarpState(%d).String() = %q, want %q", tt.state, got, tt.want)
		}
	}
}

// =========================================================================
// SchedulingPolicy tests
// =========================================================================

func TestSchedulingPolicyString(t *testing.T) {
	tests := []struct {
		policy SchedulingPolicy
		want   string
	}{
		{ScheduleRoundRobin, "round_robin"},
		{ScheduleGreedy, "greedy"},
		{ScheduleOldestFirst, "oldest_first"},
		{ScheduleGTO, "gto"},
		{ScheduleLRR, "lrr"},
	}
	for _, tt := range tests {
		if got := tt.policy.String(); got != tt.want {
			t.Errorf("SchedulingPolicy(%d).String() = %q, want %q", tt.policy, got, tt.want)
		}
	}
}

// =========================================================================
// WorkItem tests
// =========================================================================

func TestNewWorkItem(t *testing.T) {
	w := NewWorkItem(42)
	if w.WorkID != 42 {
		t.Errorf("WorkID = %d, want 42", w.WorkID)
	}
	if w.ThreadCount != 32 {
		t.Errorf("ThreadCount = %d, want 32", w.ThreadCount)
	}
	if w.RegistersPerThread != 32 {
		t.Errorf("RegistersPerThread = %d, want 32", w.RegistersPerThread)
	}
	if w.PerThreadData == nil {
		t.Error("PerThreadData should be initialized")
	}
}

// =========================================================================
// ComputeUnitTrace tests
// =========================================================================

func TestComputeUnitTraceFormat(t *testing.T) {
	trace := ComputeUnitTrace{
		Cycle:             5,
		UnitName:          "SM",
		Arch:              ArchNvidiaSM,
		SchedulerAction:   "issued warp 3",
		ActiveWarps:       36,
		TotalWarps:        48,
		EngineTraces:      make(map[int]pee.EngineTrace),
		SharedMemoryUsed:  49152,
		SharedMemoryTotal: 98304,
		RegisterFileUsed:  32768,
		RegisterFileTotal: 65536,
		Occupancy:         0.75,
	}

	formatted := trace.Format()
	if formatted == "" {
		t.Error("Format() should produce non-empty string")
	}
	if !strings.Contains(formatted, "75.0%") {
		t.Errorf("Format() should contain occupancy percentage, got: %s", formatted)
	}
	if !strings.Contains(formatted, "SM") {
		t.Errorf("Format() should contain unit name, got: %s", formatted)
	}
	if !strings.Contains(formatted, "issued warp 3") {
		t.Errorf("Format() should contain scheduler action, got: %s", formatted)
	}
}

// =========================================================================
// SharedMemory tests
// =========================================================================

func TestSharedMemoryReadWrite(t *testing.T) {
	sm := NewSharedMemory(1024)

	err := sm.Write(0, 3.14)
	if err != nil {
		t.Fatalf("Write failed: %v", err)
	}

	val, err := sm.Read(0)
	if err != nil {
		t.Fatalf("Read failed: %v", err)
	}

	// Float32 precision
	if diff := val - 3.14; diff > 0.01 || diff < -0.01 {
		t.Errorf("Read() = %f, want ~3.14", val)
	}
}

func TestSharedMemoryOutOfRange(t *testing.T) {
	sm := NewSharedMemory(64)

	_, err := sm.Read(64)
	if err == nil {
		t.Error("Read at end of memory should fail")
	}

	err = sm.Write(-1, 1.0)
	if err == nil {
		t.Error("Write at negative address should fail")
	}
}

func TestSharedMemoryBankConflicts(t *testing.T) {
	sm := NewSharedMemory(1024)

	// addr 0:   bank = (0/4) % 32 = 0
	// addr 128: bank = (128/4) % 32 = 0  -- CONFLICT!
	// addr 4:   bank = (4/4) % 32 = 1
	// addr 12:  bank = (12/4) % 32 = 3
	conflicts := sm.CheckBankConflicts([]int{0, 4, 128, 12})
	if len(conflicts) != 1 {
		t.Fatalf("Expected 1 conflict group, got %d", len(conflicts))
	}
	if len(conflicts[0]) != 2 {
		t.Errorf("Expected 2 threads in conflict, got %d", len(conflicts[0]))
	}
}

func TestSharedMemoryNoConflicts(t *testing.T) {
	sm := NewSharedMemory(1024)

	conflicts := sm.CheckBankConflicts([]int{0, 4, 8, 12})
	if len(conflicts) != 0 {
		t.Errorf("Expected no conflicts, got %d", len(conflicts))
	}
}

func TestSharedMemoryReset(t *testing.T) {
	sm := NewSharedMemory(128)

	_ = sm.Write(0, 42.0)
	sm.Reset()

	if sm.TotalAccesses() != 0 {
		t.Errorf("After reset, TotalAccesses = %d, want 0", sm.TotalAccesses())
	}
	val, _ := sm.Read(0)
	if val != 0.0 {
		t.Errorf("After reset, Read(0) = %f, want 0.0", val)
	}
}

func TestSharedMemoryAccessTracking(t *testing.T) {
	sm := NewSharedMemory(1024)

	_ = sm.Write(0, 1.0)
	_ = sm.Write(4, 2.0)
	_, _ = sm.Read(0)

	if sm.TotalAccesses() != 3 {
		t.Errorf("TotalAccesses = %d, want 3", sm.TotalAccesses())
	}
}

// =========================================================================
// ResourceError tests
// =========================================================================

func TestResourceError(t *testing.T) {
	err := &ResourceError{Message: "not enough registers"}
	if err.Error() != "not enough registers" {
		t.Errorf("Error() = %q, want 'not enough registers'", err.Error())
	}
}
