package computeruntime

import (
	"encoding/binary"
	"testing"

	devicesimulator "github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// Indirect dispatch tests
// =========================================================================

func TestSubmitDispatchIndirect(t *testing.T) {
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

	// Allocate a buffer to hold indirect dispatch params (3 uint32s = 12 bytes)
	memType := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent
	indirectBuf, _ := mm.Allocate(12, memType, BufferUsageIndirect)

	// Write dispatch dimensions: (1, 1, 1)
	data := mm.GetBufferData(indirectBuf.BufferID)
	binary.LittleEndian.PutUint32(data[0:4], 1)
	binary.LittleEndian.PutUint32(data[4:8], 1)
	binary.LittleEndian.PutUint32(data[8:12], 1)

	shader := NewShaderModule(ShaderModuleOptions{
		Code:      []gpucore.Instruction{gpucore.Halt()},
		LocalSize: [3]int{1, 1, 1},
	})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdBindPipeline(pipeline)
	_ = cb.CmdDispatchIndirect(indirectBuf, 0)
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with indirect dispatch failed: %v", err)
	}
	if stats.TotalDispatches != 1 {
		t.Errorf("TotalDispatches = %d, want 1", stats.TotalDispatches)
	}
}

// =========================================================================
// RuntimeEventType String coverage
// =========================================================================

func TestAllRuntimeEventTypeStrings(t *testing.T) {
	tests := []struct {
		et   RuntimeEventType
		want string
	}{
		{RuntimeEventSubmit, "SUBMIT"},
		{RuntimeEventBeginExecution, "BEGIN_EXECUTION"},
		{RuntimeEventEndExecution, "END_EXECUTION"},
		{RuntimeEventFenceSignal, "FENCE_SIGNAL"},
		{RuntimeEventFenceWait, "FENCE_WAIT"},
		{RuntimeEventSemaphoreSignal, "SEMAPHORE_SIGNAL"},
		{RuntimeEventSemaphoreWait, "SEMAPHORE_WAIT"},
		{RuntimeEventBarrier, "BARRIER"},
		{RuntimeEventMemoryAlloc, "MEMORY_ALLOC"},
		{RuntimeEventMemoryFree, "MEMORY_FREE"},
		{RuntimeEventMemoryMap, "MEMORY_MAP"},
		{RuntimeEventMemoryTransfer, "MEMORY_TRANSFER"},
		{RuntimeEventType(99), "UNKNOWN"},
	}
	for _, tt := range tests {
		got := tt.et.String()
		if got != tt.want {
			t.Errorf("RuntimeEventType(%d).String() = %q, want %q", tt.et, got, tt.want)
		}
	}
}

// =========================================================================
// Full pipeline: dispatch + barrier + dispatch
// =========================================================================

func TestDispatchBarrierDispatch(t *testing.T) {
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
	_ = cb.CmdPipelineBarrier(PipelineBarrierDesc{
		SrcStage: PipelineStageCompute,
		DstStage: PipelineStageCompute,
	})
	_ = cb.CmdDispatch(1, 1, 1)
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit failed: %v", err)
	}
	if stats.TotalDispatches != 2 {
		t.Errorf("TotalDispatches = %d, want 2", stats.TotalDispatches)
	}
	if stats.TotalBarriers != 1 {
		t.Errorf("TotalBarriers = %d, want 1", stats.TotalBarriers)
	}
}

// =========================================================================
// Set/Wait/Reset events in command queue
// =========================================================================

func TestSubmitWithEvents(t *testing.T) {
	q, _, _ := newTestQueue()

	event := NewEvent()

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdSetEvent(event, PipelineStageCompute)
	_ = cb.CmdWaitEvent(event, PipelineStageCompute, PipelineStageCompute)
	_ = cb.CmdResetEvent(event, PipelineStageCompute)
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with events failed: %v", err)
	}
}

// =========================================================================
// Bind commands in command queue execution (no-ops but need coverage)
// =========================================================================

func TestSubmitWithBindCommands(t *testing.T) {
	q, _, _ := newTestQueue()

	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)

	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)

	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.CmdBindPipeline(pipeline)
	_ = cb.CmdBindDescriptorSet(ds)
	_ = cb.CmdPushConstants(0, []byte{1, 2, 3, 4})
	_ = cb.End()

	_, err := q.Submit([]*CommandBuffer{cb}, nil)
	if err != nil {
		t.Fatalf("Submit with bind commands failed: %v", err)
	}
}

// =========================================================================
// MemoryType String edge cases
// =========================================================================

func TestMemoryTypeStringAll(t *testing.T) {
	all := MemoryTypeDeviceLocal | MemoryTypeHostVisible | MemoryTypeHostCoherent | MemoryTypeHostCached
	got := all.String()
	if got != "DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT | HOST_CACHED" {
		t.Errorf("all flags String() = %q", got)
	}
}

// =========================================================================
// Semaphore chained submissions
// =========================================================================

func TestChainedSubmissionsViaSemaphore(t *testing.T) {
	q, _, _ := newTestQueue()

	sem := NewSemaphore()

	cb1 := NewCommandBuffer()
	_ = cb1.Begin()
	_ = cb1.End()

	// First submit: signal the semaphore
	_, err := q.Submit([]*CommandBuffer{cb1}, &SubmitOptions{
		SignalSemaphores: []*Semaphore{sem},
	})
	if err != nil {
		t.Fatalf("first submit failed: %v", err)
	}
	if !sem.Signaled() {
		t.Error("semaphore should be signaled after first submit")
	}

	// Second submit: wait on the semaphore
	cb2 := NewCommandBuffer()
	_ = cb2.Begin()
	_ = cb2.End()

	_, err = q.Submit([]*CommandBuffer{cb2}, &SubmitOptions{
		WaitSemaphores: []*Semaphore{sem},
	})
	if err != nil {
		t.Fatalf("second submit failed: %v", err)
	}
	if sem.Signaled() {
		t.Error("semaphore should be consumed after wait")
	}
}

// =========================================================================
// Fence Wait with timeout
// =========================================================================

func TestFenceWaitWithTimeout(t *testing.T) {
	f := NewFence(false)
	timeout := 100
	if f.Wait(&timeout) {
		t.Error("unsignaled fence should not pass wait even with timeout")
	}
	// Wait doesn't track cycles in our synchronous implementation
	if f.WaitCycles() != 0 {
		t.Errorf("WaitCycles = %d, want 0", f.WaitCycles())
	}

	f.Signal()
	if !f.Wait(&timeout) {
		t.Error("signaled fence should pass wait with timeout")
	}
}

// =========================================================================
// DescriptorSet Bindings returns a copy
// =========================================================================

func TestDescriptorSetBindingsCopy(t *testing.T) {
	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)
	buf := &Buffer{BufferID: 0, Size: 64}
	_ = ds.Write(0, buf)

	bindings := ds.Bindings()
	if len(bindings) != 1 {
		t.Errorf("Bindings len = %d, want 1", len(bindings))
	}
	// Modify the returned map -- should not affect the original
	delete(bindings, 0)
	if ds.GetBuffer(0) != buf {
		t.Error("modifying returned Bindings should not affect the descriptor set")
	}
}
