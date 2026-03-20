package computeruntime

import (
	"testing"

	devicesimulator "github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// helper creates a CommandQueue backed by an NvidiaGPU for testing.
func newTestQueue() (*CommandQueue, *MemoryManager, *RuntimeStats) {
	gpu := devicesimulator.NewNvidiaGPU(nil, 2)
	stats := &RuntimeStats{}
	props := MemoryProperties{
		Heaps: []MemoryHeap{
			{Size: 16 * 1024 * 1024, Flags: MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent},
		},
		IsUnified: true,
	}
	mm := NewMemoryManager(gpu, props, stats)
	q := NewCommandQueue(QueueTypeCompute, 0, gpu, mm, stats)
	return q, mm, stats
}

// =========================================================================
// Basic submission tests
// =========================================================================

func TestSubmitEmptyCommandBuffer(t *testing.T) {
	q, _, stats := newTestQueue()

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.End()

	traces, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit failed: %v", err)
	}
	if len(traces) == 0 {
		t.Error("expected at least one trace")
	}
	if stats.TotalSubmissions != 1 {
		t.Errorf("TotalSubmissions = %d, want 1", stats.TotalSubmissions)
	}
	if stats.TotalCommandBuffers != 1 {
		t.Errorf("TotalCommandBuffers = %d, want 1", stats.TotalCommandBuffers)
	}
	if cb.State() != CommandBufferStateComplete {
		t.Errorf("CB state = %v, want %v", cb.State(), CommandBufferStateComplete)
	}
}

func TestSubmitNotRecorded(t *testing.T) {
	q, _, _ := newTestQueue()

	cb := NewCommandBuffer()
	// CB is in INITIAL state, not RECORDED
	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err == nil {
		t.Error("Submit with INITIAL CB should return error")
	}
}

func TestSubmitWithDispatch(t *testing.T) {
	q, _, stats := newTestQueue()

	shader := NewShaderModule(ShaderModuleOptions{
		Code:      []gpucore.Instruction{gpucore.Halt()},
		LocalSize: [3]int{1, 1, 1},
	})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdBindPipeline(pipeline)
	_ = cb.CmdDispatch(1, 1, 1)
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with dispatch failed: %v", err)
	}
	if stats.TotalDispatches != 1 {
		t.Errorf("TotalDispatches = %d, want 1", stats.TotalDispatches)
	}
	if q.TotalCycles() == 0 {
		t.Error("TotalCycles should be > 0 after dispatch")
	}
}

func TestSubmitWithDataflowDispatch(t *testing.T) {
	gpu := devicesimulator.NewGoogleTPU(nil, 2)
	stats := &RuntimeStats{}
	props := MemoryProperties{
		Heaps: []MemoryHeap{
			{Size: 16 * 1024 * 1024, Flags: MemoryTypeDeviceLocal | MemoryTypeHostVisible},
		},
	}
	mm := NewMemoryManager(gpu, props, stats)
	q := NewCommandQueue(QueueTypeCompute, 0, gpu, mm, stats)

	shader := NewShaderModule(ShaderModuleOptions{
		Operation: "matmul",
	})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdBindPipeline(pipeline)
	_ = cb.CmdDispatch(1, 1, 1)
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with dataflow dispatch failed: %v", err)
	}
}

// =========================================================================
// Fence and semaphore tests
// =========================================================================

func TestSubmitWithFence(t *testing.T) {
	q, _, _ := newTestQueue()
	fence := NewFence(false)

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, &SubmitOptions{Fence: fence})
	if err != nil {
		t.Fatalf("Submit with fence failed: %v", err)
	}
	if !fence.Signaled() {
		t.Error("fence should be signaled after submit")
	}
}

func TestSubmitWithSemaphores(t *testing.T) {
	q, _, stats := newTestQueue()

	signalSem := NewSemaphore()

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, &SubmitOptions{
		SignalSemaphores: []*Semaphore{signalSem},
	})
	if err != nil {
		t.Fatalf("Submit with signal semaphore failed: %v", err)
	}
	if !signalSem.Signaled() {
		t.Error("signal semaphore should be signaled")
	}
	if stats.TotalSemaphoreSignals != 1 {
		t.Errorf("TotalSemaphoreSignals = %d, want 1", stats.TotalSemaphoreSignals)
	}
}

func TestSubmitWithWaitSemaphore(t *testing.T) {
	q, _, _ := newTestQueue()

	sem := NewSemaphore()
	sem.Signal() // pre-signal it

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, &SubmitOptions{
		WaitSemaphores: []*Semaphore{sem},
	})
	if err != nil {
		t.Fatalf("Submit with wait semaphore failed: %v", err)
	}
	// Semaphore should be consumed (reset) after wait
	if sem.Signaled() {
		t.Error("wait semaphore should be consumed (reset) after submit")
	}
}

func TestSubmitWithUnsignaledWaitSemaphore(t *testing.T) {
	q, _, _ := newTestQueue()

	sem := NewSemaphore() // NOT signaled

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, &SubmitOptions{
		WaitSemaphores: []*Semaphore{sem},
	})
	if err == nil {
		t.Error("Submit with unsignaled wait semaphore should return error")
	}
}

// =========================================================================
// Transfer command tests
// =========================================================================

func TestSubmitCopyBuffer(t *testing.T) {
	q, mm, stats := newTestQueue()

	memType := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent
	src, _ := mm.Allocate(256, memType, BufferUsageStorage|BufferUsageTransferSrc)
	dst, _ := mm.Allocate(256, memType, BufferUsageStorage|BufferUsageTransferDst)

	// Write data to source
	srcData := mm.GetBufferData(src.BufferID)
	for i := range srcData {
		srcData[i] = byte(i % 256)
	}
	_, _ = mm.SyncBufferToDevice(src)

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdCopyBuffer(src, dst, 256, 0, 0)
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with copy failed: %v", err)
	}
	if stats.TotalTransfers != 1 {
		t.Errorf("TotalTransfers = %d, want 1", stats.TotalTransfers)
	}
}

func TestSubmitFillBuffer(t *testing.T) {
	q, mm, stats := newTestQueue()
	memType := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent
	buf, _ := mm.Allocate(256, memType, BufferUsageStorage)

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdFillBuffer(buf, 0xAA, 0, 256)
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with fill failed: %v", err)
	}
	if stats.TotalTransfers != 1 {
		t.Errorf("TotalTransfers = %d, want 1", stats.TotalTransfers)
	}

	// Verify fill
	data := mm.GetBufferData(buf.BufferID)
	for i := 0; i < 256; i++ {
		if data[i] != 0xAA {
			t.Errorf("byte %d: got %d, want %d", i, data[i], 0xAA)
			break
		}
	}
}

func TestSubmitUpdateBuffer(t *testing.T) {
	q, mm, stats := newTestQueue()
	memType := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent
	buf, _ := mm.Allocate(256, memType, BufferUsageStorage)

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdUpdateBuffer(buf, 10, []byte{1, 2, 3, 4})
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with update failed: %v", err)
	}
	if stats.TotalTransfers != 1 {
		t.Errorf("TotalTransfers = %d, want 1", stats.TotalTransfers)
	}
}

// =========================================================================
// Barrier tests
// =========================================================================

func TestSubmitPipelineBarrier(t *testing.T) {
	q, _, stats := newTestQueue()

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdPipelineBarrier(PipelineBarrierDesc{
		SrcStage: PipelineStageCompute,
		DstStage: PipelineStageCompute,
	})
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with barrier failed: %v", err)
	}
	if stats.TotalBarriers != 1 {
		t.Errorf("TotalBarriers = %d, want 1", stats.TotalBarriers)
	}
}

// =========================================================================
// Queue properties tests
// =========================================================================

func TestQueueProperties(t *testing.T) {
	q, _, _ := newTestQueue()
	if q.QueueType() != QueueTypeCompute {
		t.Errorf("QueueType = %v, want %v", q.QueueType(), QueueTypeCompute)
	}
	if q.QueueIndex() != 0 {
		t.Errorf("QueueIndex = %d, want 0", q.QueueIndex())
	}
}

func TestQueueWaitIdle(t *testing.T) {
	q, _, _ := newTestQueue()
	// Should not panic
	q.WaitIdle()
}

// =========================================================================
// Multiple CBs tests
// =========================================================================

func TestSubmitMultipleCommandBuffers(t *testing.T) {
	q, _, stats := newTestQueue()

	cb1 := NewCommandBuffer()
	_ = cb1.Begin()
	_ = cb1.End()

	cb2 := NewCommandBuffer()
	_ = cb2.Begin()
	_ = cb2.End()

	_, err := q.Submit([]*CommandBuffer{cb1, cb2}, nil)
	if err != nil {
		t.Fatalf("Submit multiple CBs failed: %v", err)
	}
	if stats.TotalCommandBuffers != 2 {
		t.Errorf("TotalCommandBuffers = %d, want 2", stats.TotalCommandBuffers)
	}
}

// =========================================================================
// toInt helper tests
// =========================================================================

func TestToInt(t *testing.T) {
	if toInt(42) != 42 {
		t.Error("toInt(42) should be 42")
	}
	if toInt(3.14) != 3 {
		t.Error("toInt(3.14) should be 3")
	}
	if toInt(int64(99)) != 99 {
		t.Error("toInt(int64(99)) should be 99")
	}
	if toInt("unknown") != 0 {
		t.Error("toInt(string) should be 0")
	}
}
