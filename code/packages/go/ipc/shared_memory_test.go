package ipc

import (
	"bytes"
	"testing"
)

// ========================================================================
// Attach / Detach
// ========================================================================

func TestSharedMemoryAttach(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	if !shm.Attach(1) {
		t.Error("first attach should succeed")
	}
	if !shm.IsAttached(1) {
		t.Error("PID 1 should be attached")
	}
}

func TestSharedMemoryAttachDuplicate(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	shm.Attach(1)
	if shm.Attach(1) {
		t.Error("duplicate attach should return false")
	}
}

func TestSharedMemoryDetach(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	shm.Attach(1)
	if !shm.Detach(1) {
		t.Error("detach should succeed")
	}
	if shm.IsAttached(1) {
		t.Error("PID 1 should not be attached after detach")
	}
}

func TestSharedMemoryDetachNotAttached(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	if shm.Detach(99) {
		t.Error("detaching non-attached PID should return false")
	}
}

func TestSharedMemoryMultiplePIDs(t *testing.T) {
	shm := NewSharedMemoryRegion("buffer", 4096, 1)
	shm.Attach(1)
	shm.Attach(2)
	shm.Attach(3)
	if shm.AttachedCount() != 3 {
		t.Errorf("expected 3, got %d", shm.AttachedCount())
	}
}

func TestSharedMemoryDetachReducesCount(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	shm.Attach(1)
	shm.Attach(2)
	shm.Detach(1)
	if shm.AttachedCount() != 1 {
		t.Errorf("expected 1, got %d", shm.AttachedCount())
	}
}

// ========================================================================
// Read / Write
// ========================================================================

func TestSharedMemoryWriteAndRead(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	n, err := shm.WriteAt(0, []byte("hello"))
	if err != nil || n != 5 {
		t.Fatalf("write failed: n=%d, err=%v", n, err)
	}
	data, err := shm.ReadAt(0, 5)
	if err != nil || !bytes.Equal(data, []byte("hello")) {
		t.Errorf("expected 'hello', got %q (err=%v)", data, err)
	}
}

func TestSharedMemoryWriteAtOffset(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	shm.WriteAt(100, []byte("data"))
	data, _ := shm.ReadAt(100, 4)
	if !bytes.Equal(data, []byte("data")) {
		t.Errorf("expected 'data', got %q", data)
	}
	// Bytes before the write are zero
	zeros, _ := shm.ReadAt(0, 4)
	if !bytes.Equal(zeros, make([]byte, 4)) {
		t.Error("bytes before write should be zero")
	}
}

func TestSharedMemoryOverwrite(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	shm.WriteAt(0, []byte("old"))
	shm.WriteAt(0, []byte("new"))
	data, _ := shm.ReadAt(0, 3)
	if !bytes.Equal(data, []byte("new")) {
		t.Errorf("expected 'new', got %q", data)
	}
}

func TestSharedMemoryZeroInitialized(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 16, 1)
	data, _ := shm.ReadAt(0, 16)
	if !bytes.Equal(data, make([]byte, 16)) {
		t.Error("fresh region should be zero-initialized")
	}
}

func TestSharedMemoryMultiProcessVisibility(t *testing.T) {
	shm := NewSharedMemoryRegion("cache", 4096, 1)
	shm.Attach(1)
	shm.Attach(2)

	shm.WriteAt(0, []byte("shared data from process 1"))
	data, _ := shm.ReadAt(0, 26)
	if !bytes.Equal(data, []byte("shared data from process 1")) {
		t.Errorf("expected 'shared data from process 1', got %q", data)
	}
}

// ========================================================================
// Bounds checking
// ========================================================================

func TestSharedMemoryReadBeyondSize(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 16, 1)
	_, err := shm.ReadAt(10, 10)
	if err == nil {
		t.Error("should error on read beyond bounds")
	}
}

func TestSharedMemoryWriteBeyondSize(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 16, 1)
	_, err := shm.WriteAt(10, []byte("0123456789"))
	if err == nil {
		t.Error("should error on write beyond bounds")
	}
}

func TestSharedMemoryNegativeReadOffset(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 16, 1)
	_, err := shm.ReadAt(-1, 4)
	if err == nil {
		t.Error("should error on negative offset")
	}
}

func TestSharedMemoryNegativeWriteOffset(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 16, 1)
	_, err := shm.WriteAt(-1, []byte("data"))
	if err == nil {
		t.Error("should error on negative offset")
	}
}

func TestSharedMemoryReadExactlyAtBoundary(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 8, 1)
	shm.WriteAt(0, []byte("12345678"))
	data, err := shm.ReadAt(0, 8)
	if err != nil || !bytes.Equal(data, []byte("12345678")) {
		t.Error("reading exactly at boundary should work")
	}
}

func TestSharedMemoryWriteExactlyAtBoundary(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 8, 1)
	n, err := shm.WriteAt(0, []byte("12345678"))
	if err != nil || n != 8 {
		t.Error("writing exactly at boundary should work")
	}
}

// ========================================================================
// Properties
// ========================================================================

func TestSharedMemoryName(t *testing.T) {
	shm := NewSharedMemoryRegion("my_region", 1024, 42)
	if shm.Name() != "my_region" {
		t.Errorf("expected 'my_region', got %q", shm.Name())
	}
}

func TestSharedMemorySize(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 2048, 1)
	if shm.Size() != 2048 {
		t.Errorf("expected 2048, got %d", shm.Size())
	}
}

func TestSharedMemoryOwnerPID(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 42)
	if shm.OwnerPID() != 42 {
		t.Errorf("expected 42, got %d", shm.OwnerPID())
	}
}

func TestSharedMemoryAttachedCountEmpty(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	if shm.AttachedCount() != 0 {
		t.Errorf("expected 0, got %d", shm.AttachedCount())
	}
}

func TestSharedMemoryIsAttachedFalse(t *testing.T) {
	shm := NewSharedMemoryRegion("test", 1024, 1)
	if shm.IsAttached(99) {
		t.Error("PID 99 should not be attached")
	}
}
