package processmanager

import "testing"

// =============================================================================
// AddProcess and Schedule Tests
// =============================================================================

func TestScheduleEmpty(t *testing.T) {
	sched := NewPriorityScheduler()
	_, ok := sched.Schedule()
	if ok {
		t.Error("Empty scheduler should return false")
	}
}

func TestScheduleSingleProcess(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)

	pid, ok := sched.Schedule()
	if !ok || pid != 1 {
		t.Errorf("Got (%d, %v), want (1, true)", pid, ok)
	}
}

func TestSchedulePicksHighestPriority(t *testing.T) {
	// Lower priority number = higher priority = runs first.
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.AddProcess(2, 5)
	sched.AddProcess(3, 39)

	pid, _ := sched.Schedule()
	if pid != 2 {
		t.Errorf("First scheduled = %d, want 2 (priority 5)", pid)
	}

	pid, _ = sched.Schedule()
	if pid != 1 {
		t.Errorf("Second scheduled = %d, want 1 (priority 20)", pid)
	}

	pid, _ = sched.Schedule()
	if pid != 3 {
		t.Errorf("Third scheduled = %d, want 3 (priority 39)", pid)
	}
}

func TestRoundRobinWithinPriority(t *testing.T) {
	// Same-priority processes should be FIFO.
	sched := NewPriorityScheduler()
	sched.AddProcess(10, 20)
	sched.AddProcess(11, 20)
	sched.AddProcess(12, 20)

	pid, _ := sched.Schedule()
	if pid != 10 {
		t.Errorf("First = %d, want 10", pid)
	}
	pid, _ = sched.Schedule()
	if pid != 11 {
		t.Errorf("Second = %d, want 11", pid)
	}
	pid, _ = sched.Schedule()
	if pid != 12 {
		t.Errorf("Third = %d, want 12", pid)
	}
}

func TestScheduleRemovesFromQueue(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.Schedule()

	_, ok := sched.Schedule()
	if ok {
		t.Error("Queue should be empty after scheduling the only process")
	}
}

func TestPriorityClampedLow(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, -5)
	if sched.GetPriority(1) != MinPriority {
		t.Errorf("Priority = %d, want %d", sched.GetPriority(1), MinPriority)
	}
}

func TestPriorityClampedHigh(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 100)
	if sched.GetPriority(1) != MaxPriority {
		t.Errorf("Priority = %d, want %d", sched.GetPriority(1), MaxPriority)
	}
}

// =============================================================================
// RemoveProcess Tests
// =============================================================================

func TestRemoveProcess(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.RemoveProcess(1)

	_, ok := sched.Schedule()
	if ok {
		t.Error("Removed process should not be schedulable")
	}
}

func TestRemoveNonexistent(t *testing.T) {
	sched := NewPriorityScheduler()
	// Should not panic.
	sched.RemoveProcess(999)
}

func TestRemoveResetsCurrentPID(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.Schedule()
	sched.RemoveProcess(1)

	if sched.CurrentPID() != -1 {
		t.Error("Current PID should be -1 after removing current process")
	}
}

func TestRemoveFromMiddle(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.AddProcess(2, 20)
	sched.AddProcess(3, 20)
	sched.RemoveProcess(2)

	pid, _ := sched.Schedule()
	if pid != 1 {
		t.Errorf("First = %d, want 1", pid)
	}
	pid, _ = sched.Schedule()
	if pid != 3 {
		t.Errorf("Second = %d, want 3", pid)
	}
}

// =============================================================================
// SetPriority Tests
// =============================================================================

func TestSetPriority(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.SetPriority(1, 5)

	if sched.GetPriority(1) != 5 {
		t.Errorf("Priority = %d, want 5", sched.GetPriority(1))
	}
}

func TestSetPriorityAffectsScheduling(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.AddProcess(2, 20)
	sched.SetPriority(1, 5) // Promote PID 1.

	pid, _ := sched.Schedule()
	if pid != 1 {
		t.Errorf("Promoted process should schedule first, got %d", pid)
	}
}

func TestSetSamePriority(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.SetPriority(1, 20) // No change.

	if sched.GetPriority(1) != 20 {
		t.Error("Priority should remain unchanged")
	}
}

func TestSetPriorityClamped(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.SetPriority(1, -10)

	if sched.GetPriority(1) != MinPriority {
		t.Errorf("Priority = %d, want %d", sched.GetPriority(1), MinPriority)
	}
}

func TestSetPriorityUnknownPID(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.SetPriority(99, 5)

	if sched.GetPriority(99) != 5 {
		t.Error("Should record priority for unknown PID")
	}
}

// =============================================================================
// GetPriority Tests
// =============================================================================

func TestGetPriorityKnown(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 10)
	if sched.GetPriority(1) != 10 {
		t.Error("Should return actual priority")
	}
}

func TestGetPriorityUnknown(t *testing.T) {
	sched := NewPriorityScheduler()
	if sched.GetPriority(999) != DefaultPriority {
		t.Error("Should return DefaultPriority for unknown PID")
	}
}

// =============================================================================
// CurrentPID Tests
// =============================================================================

func TestCurrentPIDInitial(t *testing.T) {
	sched := NewPriorityScheduler()
	if sched.CurrentPID() != -1 {
		t.Error("Initial CurrentPID should be -1")
	}
}

func TestCurrentPIDAfterSchedule(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(42, 20)
	sched.Schedule()

	if sched.CurrentPID() != 42 {
		t.Errorf("CurrentPID = %d, want 42", sched.CurrentPID())
	}
}

func TestCurrentPIDAfterEmptySchedule(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.Schedule()

	if sched.CurrentPID() != -1 {
		t.Error("CurrentPID should be -1 when nothing scheduled")
	}
}

// =============================================================================
// TimeQuantum Tests
// =============================================================================

func TestTimeQuantumHighest(t *testing.T) {
	sched := NewPriorityScheduler()
	if sched.GetTimeQuantum(0) != BaseQuantum {
		t.Errorf("Quantum at priority 0 = %d, want %d", sched.GetTimeQuantum(0), BaseQuantum)
	}
}

func TestTimeQuantumDefault(t *testing.T) {
	sched := NewPriorityScheduler()
	expected := BaseQuantum - (20 * QuantumPerPriority)
	if sched.GetTimeQuantum(20) != expected {
		t.Errorf("Quantum at priority 20 = %d, want %d", sched.GetTimeQuantum(20), expected)
	}
}

func TestTimeQuantumLowest(t *testing.T) {
	sched := NewPriorityScheduler()
	expected := BaseQuantum - (39 * QuantumPerPriority)
	if sched.GetTimeQuantum(39) != expected {
		t.Errorf("Quantum at priority 39 = %d, want %d", sched.GetTimeQuantum(39), expected)
	}
}

func TestTimeQuantumClamped(t *testing.T) {
	sched := NewPriorityScheduler()
	if sched.GetTimeQuantum(-5) != BaseQuantum {
		t.Error("Negative priority should clamp to 0")
	}
	expected := BaseQuantum - (39 * QuantumPerPriority)
	if sched.GetTimeQuantum(100) != expected {
		t.Error("Priority > 39 should clamp to 39")
	}
}

// =============================================================================
// IsEmpty Tests
// =============================================================================

func TestIsEmptyNew(t *testing.T) {
	sched := NewPriorityScheduler()
	if !sched.IsEmpty() {
		t.Error("New scheduler should be empty")
	}
}

func TestIsEmptyWithProcess(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	if sched.IsEmpty() {
		t.Error("Scheduler with processes should not be empty")
	}
}

func TestIsEmptyAfterSchedule(t *testing.T) {
	sched := NewPriorityScheduler()
	sched.AddProcess(1, 20)
	sched.Schedule()
	if !sched.IsEmpty() {
		t.Error("Scheduler should be empty after scheduling all processes")
	}
}

// =============================================================================
// Constants Tests
// =============================================================================

func TestConstants(t *testing.T) {
	if MinPriority != 0 {
		t.Error("MinPriority should be 0")
	}
	if MaxPriority != 39 {
		t.Error("MaxPriority should be 39")
	}
	if DefaultPriority != 20 {
		t.Error("DefaultPriority should be 20")
	}
	if BaseQuantum != 200 {
		t.Error("BaseQuantum should be 200")
	}
	if QuantumPerPriority != 4 {
		t.Error("QuantumPerPriority should be 4")
	}
}
