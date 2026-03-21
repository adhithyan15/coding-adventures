package computeruntime

import (
	"testing"

	devicesimulator "github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator"
)

// helper creates a MemoryManager backed by an NvidiaGPU for testing.
func newTestMemoryManager() (*MemoryManager, *RuntimeStats) {
	gpu := devicesimulator.NewNvidiaGPU(nil, 2)
	stats := &RuntimeStats{}
	props := MemoryProperties{
		Heaps: []MemoryHeap{
			{Size: 16 * 1024 * 1024, Flags: MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent},
		},
		IsUnified: true,
	}
	mm := NewMemoryManager(gpu, props, stats)
	return mm, stats
}

// =========================================================================
// Allocation tests
// =========================================================================

func TestAllocateSuccess(t *testing.T) {
	mm, stats := newTestMemoryManager()
	buf, err := mm.Allocate(1024, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	if err != nil {
		t.Fatalf("Allocate failed: %v", err)
	}
	if buf.Size != 1024 {
		t.Errorf("Size = %d, want 1024", buf.Size)
	}
	if buf.Freed {
		t.Error("new buffer should not be freed")
	}
	if buf.Mapped {
		t.Error("new buffer should not be mapped")
	}
	if stats.TotalAllocations != 1 {
		t.Errorf("TotalAllocations = %d, want 1", stats.TotalAllocations)
	}
	if stats.TotalAllocatedBytes != 1024 {
		t.Errorf("TotalAllocatedBytes = %d, want 1024", stats.TotalAllocatedBytes)
	}
	if mm.AllocatedBufferCount() != 1 {
		t.Errorf("AllocatedBufferCount = %d, want 1", mm.AllocatedBufferCount())
	}
}

func TestAllocateZeroSize(t *testing.T) {
	mm, _ := newTestMemoryManager()
	_, err := mm.Allocate(0, MemoryTypeDeviceLocal, BufferUsageStorage)
	if err == nil {
		t.Error("Allocate(0) should return error")
	}
}

func TestAllocateNegativeSize(t *testing.T) {
	mm, _ := newTestMemoryManager()
	_, err := mm.Allocate(-1, MemoryTypeDeviceLocal, BufferUsageStorage)
	if err == nil {
		t.Error("Allocate(-1) should return error")
	}
}

func TestPeakAllocatedBytes(t *testing.T) {
	mm, stats := newTestMemoryManager()
	memType := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent

	buf1, _ := mm.Allocate(1000, memType, BufferUsageStorage)
	buf2, _ := mm.Allocate(2000, memType, BufferUsageStorage)
	if stats.PeakAllocatedBytes != 3000 {
		t.Errorf("PeakAllocatedBytes = %d, want 3000", stats.PeakAllocatedBytes)
	}

	_ = mm.Free(buf1)
	_ = mm.Free(buf2)
	// Peak should remain at 3000 even after freeing
	if stats.PeakAllocatedBytes != 3000 {
		t.Errorf("PeakAllocatedBytes after free = %d, want 3000", stats.PeakAllocatedBytes)
	}
	if mm.CurrentAllocatedBytes() != 0 {
		t.Errorf("CurrentAllocatedBytes = %d, want 0", mm.CurrentAllocatedBytes())
	}
}

// =========================================================================
// Free tests
// =========================================================================

func TestFreeSuccess(t *testing.T) {
	mm, stats := newTestMemoryManager()
	buf, _ := mm.Allocate(1024, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	err := mm.Free(buf)
	if err != nil {
		t.Fatalf("Free failed: %v", err)
	}
	if !buf.Freed {
		t.Error("buffer should be freed")
	}
	if stats.TotalFrees != 1 {
		t.Errorf("TotalFrees = %d, want 1", stats.TotalFrees)
	}
}

func TestFreeAlreadyFreed(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(1024, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	_ = mm.Free(buf)
	err := mm.Free(buf)
	if err == nil {
		t.Error("double free should return error")
	}
}

func TestFreeMappedBuffer(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(1024, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	_, _ = mm.Map(buf)
	err := mm.Free(buf)
	if err == nil {
		t.Error("freeing a mapped buffer should return error")
	}
}

func TestFreeUnknownBuffer(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf := &Buffer{BufferID: 9999}
	err := mm.Free(buf)
	if err == nil {
		t.Error("freeing an unknown buffer should return error")
	}
}

// =========================================================================
// Map/Unmap tests
// =========================================================================

func TestMapSuccess(t *testing.T) {
	mm, stats := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	mapped, err := mm.Map(buf)
	if err != nil {
		t.Fatalf("Map failed: %v", err)
	}
	if !buf.Mapped {
		t.Error("buffer should be mapped")
	}
	if mapped.Size() != 256 {
		t.Errorf("mapped Size = %d, want 256", mapped.Size())
	}
	if stats.TotalMaps != 1 {
		t.Errorf("TotalMaps = %d, want 1", stats.TotalMaps)
	}
}

func TestMapDeviceLocalOnly(t *testing.T) {
	gpu := devicesimulator.NewNvidiaGPU(nil, 2)
	stats := &RuntimeStats{}
	props := MemoryProperties{
		Heaps: []MemoryHeap{
			{Size: 16 * 1024 * 1024, Flags: MemoryTypeDeviceLocal},
		},
	}
	mm := NewMemoryManager(gpu, props, stats)
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal, BufferUsageStorage)
	_, err := mm.Map(buf)
	if err == nil {
		t.Error("mapping DEVICE_LOCAL-only buffer should return error")
	}
}

func TestMapFreedBuffer(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	_ = mm.Free(buf)
	_, err := mm.Map(buf)
	if err == nil {
		t.Error("mapping freed buffer should return error")
	}
}

func TestMapAlreadyMapped(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	_, _ = mm.Map(buf)
	_, err := mm.Map(buf)
	if err == nil {
		t.Error("double mapping should return error")
	}
}

func TestUnmapSuccess(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible|MemoryTypeHostCoherent, BufferUsageStorage)
	_, _ = mm.Map(buf)
	err := mm.Unmap(buf)
	if err != nil {
		t.Fatalf("Unmap failed: %v", err)
	}
	if buf.Mapped {
		t.Error("buffer should not be mapped after Unmap")
	}
}

func TestUnmapNotMapped(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	err := mm.Unmap(buf)
	if err == nil {
		t.Error("unmapping an unmapped buffer should return error")
	}
}

// =========================================================================
// MappedMemory read/write tests
// =========================================================================

func TestMappedMemoryReadWrite(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	mapped, _ := mm.Map(buf)

	// Write data
	data := []byte{1, 2, 3, 4}
	err := mapped.Write(0, data)
	if err != nil {
		t.Fatalf("Write failed: %v", err)
	}
	if !mapped.Dirty() {
		t.Error("mapped memory should be dirty after write")
	}

	// Read it back
	readBack, err := mapped.Read(0, 4)
	if err != nil {
		t.Fatalf("Read failed: %v", err)
	}
	for i, b := range readBack {
		if b != data[i] {
			t.Errorf("byte %d: got %d, want %d", i, b, data[i])
		}
	}

	// Read the full buffer
	full := mapped.GetData()
	if len(full) != 256 {
		t.Errorf("GetData len = %d, want 256", len(full))
	}
}

func TestMappedMemoryReadOutOfBounds(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(16, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	mapped, _ := mm.Map(buf)
	_, err := mapped.Read(10, 10)
	if err == nil {
		t.Error("reading out of bounds should return error")
	}
}

func TestMappedMemoryWriteOutOfBounds(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(16, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	mapped, _ := mm.Map(buf)
	err := mapped.Write(10, make([]byte, 10))
	if err == nil {
		t.Error("writing out of bounds should return error")
	}
}

func TestMappedMemoryBuffer(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(16, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	mapped, _ := mm.Map(buf)
	if mapped.Buffer() != buf {
		t.Error("MappedMemory.Buffer() should return the original buffer")
	}
}

// =========================================================================
// GetBuffer tests
// =========================================================================

func TestGetBuffer(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(1024, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	got, err := mm.GetBuffer(buf.BufferID)
	if err != nil {
		t.Fatalf("GetBuffer failed: %v", err)
	}
	if got != buf {
		t.Error("GetBuffer should return same buffer")
	}
}

func TestGetBufferNotFound(t *testing.T) {
	mm, _ := newTestMemoryManager()
	_, err := mm.GetBuffer(9999)
	if err == nil {
		t.Error("GetBuffer for unknown ID should return error")
	}
}

// =========================================================================
// Flush and Invalidate tests
// =========================================================================

func TestFlushSuccess(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	err := mm.Flush(buf, 0, 0)
	if err != nil {
		t.Fatalf("Flush failed: %v", err)
	}
}

func TestFlushFreedBuffer(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	_ = mm.Free(buf)
	err := mm.Flush(buf, 0, 0)
	if err == nil {
		t.Error("flushing freed buffer should return error")
	}
}

func TestInvalidateSuccess(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	err := mm.Invalidate(buf, 0, 0)
	if err != nil {
		t.Fatalf("Invalidate failed: %v", err)
	}
}

func TestInvalidateFreedBuffer(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	_ = mm.Free(buf)
	err := mm.Invalidate(buf, 0, 0)
	if err == nil {
		t.Error("invalidating freed buffer should return error")
	}
}

// =========================================================================
// Sync buffer tests
// =========================================================================

func TestSyncBufferToDevice(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	cycles, err := mm.SyncBufferToDevice(buf)
	if err != nil {
		t.Fatalf("SyncBufferToDevice failed: %v", err)
	}
	if cycles < 0 {
		t.Errorf("cycles should be >= 0, got %d", cycles)
	}
}

func TestSyncBufferFromDevice(t *testing.T) {
	mm, _ := newTestMemoryManager()
	buf, _ := mm.Allocate(256, MemoryTypeDeviceLocal|MemoryTypeHostVisible, BufferUsageStorage)
	cycles, err := mm.SyncBufferFromDevice(buf)
	if err != nil {
		t.Fatalf("SyncBufferFromDevice failed: %v", err)
	}
	if cycles < 0 {
		t.Errorf("cycles should be >= 0, got %d", cycles)
	}
}

func TestMemoryProperties(t *testing.T) {
	mm, _ := newTestMemoryManager()
	props := mm.MemoryProperties()
	if len(props.Heaps) != 1 {
		t.Errorf("expected 1 heap, got %d", len(props.Heaps))
	}
	if !props.IsUnified {
		t.Error("expected unified memory")
	}
}

func TestTraceLogging(t *testing.T) {
	mm, stats := newTestMemoryManager()
	memType := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent
	buf, _ := mm.Allocate(1024, memType, BufferUsageStorage)
	_, _ = mm.Map(buf)
	_ = mm.Unmap(buf)
	_ = mm.Free(buf)

	// Should have traces for: alloc, map, free
	if len(stats.Traces) < 3 {
		t.Errorf("expected at least 3 traces, got %d", len(stats.Traces))
	}
}
