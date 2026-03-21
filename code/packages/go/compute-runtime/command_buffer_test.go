package computeruntime

import (
	"testing"
)

// =========================================================================
// Lifecycle tests
// =========================================================================

func TestCommandBufferCreation(t *testing.T) {
	cb := NewCommandBuffer()
	if cb.State() != CommandBufferStateInitial {
		t.Errorf("State = %v, want %v", cb.State(), CommandBufferStateInitial)
	}
	if len(cb.Commands()) != 0 {
		t.Errorf("Commands = %d, want 0", len(cb.Commands()))
	}
}

func TestCommandBufferBeginEnd(t *testing.T) {
	cb := NewCommandBuffer()

	if err := cb.Begin(); err != nil {
		t.Fatalf("Begin failed: %v", err)
	}
	if cb.State() != CommandBufferStateRecording {
		t.Errorf("State after Begin = %v, want %v", cb.State(), CommandBufferStateRecording)
	}

	if err := cb.End(); err != nil {
		t.Fatalf("End failed: %v", err)
	}
	if cb.State() != CommandBufferStateRecorded {
		t.Errorf("State after End = %v, want %v", cb.State(), CommandBufferStateRecorded)
	}
}

func TestCommandBufferBeginInvalidState(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()
	// Can't begin while recording
	if err := cb.Begin(); err == nil {
		t.Error("Begin during RECORDING should return error")
	}
}

func TestCommandBufferEndInvalidState(t *testing.T) {
	cb := NewCommandBuffer()
	// Can't end without begin
	if err := cb.End(); err == nil {
		t.Error("End in INITIAL state should return error")
	}
}

func TestCommandBufferReset(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.End()
	cb.Reset()
	if cb.State() != CommandBufferStateInitial {
		t.Errorf("State after Reset = %v, want %v", cb.State(), CommandBufferStateInitial)
	}
}

func TestCommandBufferReuse(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.End()
	cb.MarkPending()
	cb.MarkComplete()

	// Should be able to begin again from COMPLETE state
	if err := cb.Begin(); err != nil {
		t.Fatalf("Begin from COMPLETE failed: %v", err)
	}
}

func TestCommandBufferUniqueIDs(t *testing.T) {
	cb1 := NewCommandBuffer()
	cb2 := NewCommandBuffer()
	if cb1.CommandBufferID() == cb2.CommandBufferID() {
		t.Errorf("CBs should have unique IDs: %d == %d", cb1.CommandBufferID(), cb2.CommandBufferID())
	}
}

// =========================================================================
// Compute command tests
// =========================================================================

func TestCmdBindPipeline(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)

	if err := cb.CmdBindPipeline(pipeline); err != nil {
		t.Fatalf("CmdBindPipeline failed: %v", err)
	}
	if cb.BoundPipeline() != pipeline {
		t.Error("BoundPipeline should be the pipeline we bound")
	}
	if len(cb.Commands()) != 1 {
		t.Errorf("Commands = %d, want 1", len(cb.Commands()))
	}
}

func TestCmdBindPipelineNotRecording(t *testing.T) {
	cb := NewCommandBuffer()
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)

	if err := cb.CmdBindPipeline(pipeline); err == nil {
		t.Error("CmdBindPipeline outside RECORDING should return error")
	}
}

func TestCmdBindDescriptorSet(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)

	if err := cb.CmdBindDescriptorSet(ds); err != nil {
		t.Fatalf("CmdBindDescriptorSet failed: %v", err)
	}
	if cb.BoundDescriptorSet() != ds {
		t.Error("BoundDescriptorSet should match")
	}
}

func TestCmdPushConstants(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	data := []byte{0x42, 0x43}
	if err := cb.CmdPushConstants(0, data); err != nil {
		t.Fatalf("CmdPushConstants failed: %v", err)
	}
	if len(cb.Commands()) != 1 {
		t.Errorf("Commands = %d, want 1", len(cb.Commands()))
	}
}

func TestCmdDispatch(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)
	_ = cb.CmdBindPipeline(pipeline)

	if err := cb.CmdDispatch(4, 1, 1); err != nil {
		t.Fatalf("CmdDispatch failed: %v", err)
	}
}

func TestCmdDispatchNoPipeline(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	if err := cb.CmdDispatch(4, 1, 1); err == nil {
		t.Error("CmdDispatch without pipeline should return error")
	}
}

func TestCmdDispatchIndirect(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)
	_ = cb.CmdBindPipeline(pipeline)

	buf := &Buffer{BufferID: 0, Size: 12}
	if err := cb.CmdDispatchIndirect(buf, 0); err != nil {
		t.Fatalf("CmdDispatchIndirect failed: %v", err)
	}
}

func TestCmdDispatchIndirectNoPipeline(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	buf := &Buffer{BufferID: 0, Size: 12}
	if err := cb.CmdDispatchIndirect(buf, 0); err == nil {
		t.Error("CmdDispatchIndirect without pipeline should return error")
	}
}

// =========================================================================
// Transfer command tests
// =========================================================================

func TestCmdCopyBuffer(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	src := &Buffer{BufferID: 0, Size: 1024}
	dst := &Buffer{BufferID: 1, Size: 1024}

	if err := cb.CmdCopyBuffer(src, dst, 512, 0, 0); err != nil {
		t.Fatalf("CmdCopyBuffer failed: %v", err)
	}
	if len(cb.Commands()) != 1 {
		t.Errorf("Commands = %d, want 1", len(cb.Commands()))
	}
}

func TestCmdFillBuffer(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	buf := &Buffer{BufferID: 0, Size: 256}

	if err := cb.CmdFillBuffer(buf, 0, 0, 0); err != nil {
		t.Fatalf("CmdFillBuffer failed: %v", err)
	}
	cmd := cb.Commands()[0]
	if cmd.Args["size"].(int) != 256 {
		t.Errorf("fill size = %v, want 256", cmd.Args["size"])
	}
}

func TestCmdUpdateBuffer(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	buf := &Buffer{BufferID: 0, Size: 256}
	data := []byte{1, 2, 3, 4}

	if err := cb.CmdUpdateBuffer(buf, 0, data); err != nil {
		t.Fatalf("CmdUpdateBuffer failed: %v", err)
	}
}

// =========================================================================
// Synchronization command tests
// =========================================================================

func TestCmdPipelineBarrier(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	barrier := PipelineBarrierDesc{
		SrcStage: PipelineStageCompute,
		DstStage: PipelineStageCompute,
		MemoryBarriers: []MemoryBarrier{
			{SrcAccess: AccessFlagsShaderWrite, DstAccess: AccessFlagsShaderRead},
		},
	}

	if err := cb.CmdPipelineBarrier(barrier); err != nil {
		t.Fatalf("CmdPipelineBarrier failed: %v", err)
	}
}

func TestCmdSetEvent(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	event := NewEvent()
	if err := cb.CmdSetEvent(event, PipelineStageCompute); err != nil {
		t.Fatalf("CmdSetEvent failed: %v", err)
	}
}

func TestCmdWaitEvent(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	event := NewEvent()
	if err := cb.CmdWaitEvent(event, PipelineStageCompute, PipelineStageCompute); err != nil {
		t.Fatalf("CmdWaitEvent failed: %v", err)
	}
}

func TestCmdResetEvent(t *testing.T) {
	cb := NewCommandBuffer()
	_ = cb.Begin()

	event := NewEvent()
	if err := cb.CmdResetEvent(event, PipelineStageCompute); err != nil {
		t.Fatalf("CmdResetEvent failed: %v", err)
	}
}

func TestRecordCommandsNotRecording(t *testing.T) {
	cb := NewCommandBuffer()
	// State is INITIAL, not RECORDING
	event := NewEvent()
	if err := cb.CmdSetEvent(event, PipelineStageCompute); err == nil {
		t.Error("CmdSetEvent in INITIAL state should return error")
	}
	if err := cb.CmdWaitEvent(event, PipelineStageCompute, PipelineStageCompute); err == nil {
		t.Error("CmdWaitEvent in INITIAL state should return error")
	}
	if err := cb.CmdResetEvent(event, PipelineStageCompute); err == nil {
		t.Error("CmdResetEvent in INITIAL state should return error")
	}
	if err := cb.CmdPipelineBarrier(DefaultPipelineBarrier()); err == nil {
		t.Error("CmdPipelineBarrier in INITIAL state should return error")
	}
	if err := cb.CmdPushConstants(0, []byte{1}); err == nil {
		t.Error("CmdPushConstants in INITIAL state should return error")
	}
	buf := &Buffer{BufferID: 0, Size: 256}
	if err := cb.CmdCopyBuffer(buf, buf, 1, 0, 0); err == nil {
		t.Error("CmdCopyBuffer in INITIAL state should return error")
	}
	if err := cb.CmdFillBuffer(buf, 0, 0, 0); err == nil {
		t.Error("CmdFillBuffer in INITIAL state should return error")
	}
	if err := cb.CmdUpdateBuffer(buf, 0, []byte{1}); err == nil {
		t.Error("CmdUpdateBuffer in INITIAL state should return error")
	}
}
