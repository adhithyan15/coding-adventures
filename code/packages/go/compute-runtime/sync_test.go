package computeruntime

import "testing"

// =========================================================================
// Fence tests
// =========================================================================

func TestFenceCreation(t *testing.T) {
	f := NewFence(false)
	if f.Signaled() {
		t.Error("new unsignaled fence should not be signaled")
	}
	if f.WaitCycles() != 0 {
		t.Errorf("WaitCycles = %d, want 0", f.WaitCycles())
	}
}

func TestFenceCreationSignaled(t *testing.T) {
	f := NewFence(true)
	if !f.Signaled() {
		t.Error("fence created with signaled=true should be signaled")
	}
}

func TestFenceSignalAndWait(t *testing.T) {
	f := NewFence(false)
	if f.Wait(nil) {
		t.Error("unsignaled fence Wait should return false")
	}
	f.Signal()
	if !f.Signaled() {
		t.Error("fence should be signaled after Signal()")
	}
	if !f.Wait(nil) {
		t.Error("signaled fence Wait should return true")
	}
}

func TestFenceReset(t *testing.T) {
	f := NewFence(true)
	f.Reset()
	if f.Signaled() {
		t.Error("fence should not be signaled after Reset()")
	}
	if f.WaitCycles() != 0 {
		t.Errorf("WaitCycles should be 0 after Reset, got %d", f.WaitCycles())
	}
}

func TestFenceUniqueIDs(t *testing.T) {
	f1 := NewFence(false)
	f2 := NewFence(false)
	if f1.FenceID() == f2.FenceID() {
		t.Errorf("fences should have unique IDs: %d == %d", f1.FenceID(), f2.FenceID())
	}
}

// =========================================================================
// Semaphore tests
// =========================================================================

func TestSemaphoreCreation(t *testing.T) {
	s := NewSemaphore()
	if s.Signaled() {
		t.Error("new semaphore should not be signaled")
	}
}

func TestSemaphoreSignalAndReset(t *testing.T) {
	s := NewSemaphore()
	s.Signal()
	if !s.Signaled() {
		t.Error("semaphore should be signaled after Signal()")
	}
	s.Reset()
	if s.Signaled() {
		t.Error("semaphore should not be signaled after Reset()")
	}
}

func TestSemaphoreUniqueIDs(t *testing.T) {
	s1 := NewSemaphore()
	s2 := NewSemaphore()
	if s1.SemaphoreID() == s2.SemaphoreID() {
		t.Errorf("semaphores should have unique IDs: %d == %d", s1.SemaphoreID(), s2.SemaphoreID())
	}
}

// =========================================================================
// Event tests
// =========================================================================

func TestEventCreation(t *testing.T) {
	e := NewEvent()
	if e.Signaled() {
		t.Error("new event should not be signaled")
	}
	if e.Status() {
		t.Error("new event Status() should be false")
	}
}

func TestEventSetAndReset(t *testing.T) {
	e := NewEvent()
	e.Set()
	if !e.Signaled() {
		t.Error("event should be signaled after Set()")
	}
	if !e.Status() {
		t.Error("event Status() should be true after Set()")
	}
	e.Reset()
	if e.Signaled() {
		t.Error("event should not be signaled after Reset()")
	}
}

func TestEventUniqueIDs(t *testing.T) {
	e1 := NewEvent()
	e2 := NewEvent()
	if e1.EventID() == e2.EventID() {
		t.Errorf("events should have unique IDs: %d == %d", e1.EventID(), e2.EventID())
	}
}
