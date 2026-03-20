package computeruntime

// CommandBuffer -- recorded sequence of GPU commands.
//
// # The Record-Then-Submit Model
//
// Instead of calling GPU operations one at a time (like CUDA), Vulkan records
// commands into a buffer and submits the whole buffer at once:
//
//	// CUDA style (implicit, one at a time):
//	cudaMemcpy(dst, src, size)     // executes immediately
//	kernel<<<grid, block>>>(args)  // executes immediately
//
//	// Vulkan style (explicit, batched):
//	cb.Begin()                     // start recording
//	cb.CmdCopyBuffer(...)          // just records -- does not execute
//	cb.CmdDispatch(...)            // just records -- does not execute
//	cb.End()                       // stop recording
//	queue.Submit([cb])             // NOW everything executes
//
// # Why Batch?
//
//  1. Driver optimization -- the driver sees all commands at once and can
//     reorder, merge, or eliminate redundancies.
//  2. Reuse -- submit the same CB multiple times without re-recording.
//  3. Multi-threaded recording -- different CPU threads record different CBs.
//  4. Validation -- check the entire sequence for errors before any GPU work.

import "fmt"

// nextCommandBufferID is the global counter for command buffer IDs.
var nextCommandBufferID int

// CommandBuffer is a recorded sequence of GPU commands.
//
// # Command Types
//
// Compute commands:
//
//	CmdBindPipeline       -- select which kernel to run
//	CmdBindDescriptorSet  -- bind memory to kernel parameters
//	CmdPushConstants      -- small inline data (<=128 bytes)
//	CmdDispatch           -- launch kernel with grid dimensions
//	CmdDispatchIndirect   -- read grid dimensions from a GPU buffer
//
// Transfer commands:
//
//	CmdCopyBuffer         -- device-to-device memory copy
//	CmdFillBuffer         -- fill buffer with a constant value
//	CmdUpdateBuffer       -- write small data inline (CPU -> GPU)
//
// Synchronization commands:
//
//	CmdPipelineBarrier    -- execution + memory ordering
//	CmdSetEvent           -- signal an event from GPU
//	CmdWaitEvent          -- wait for event before proceeding
//	CmdResetEvent         -- reset event from GPU
type CommandBuffer struct {
	id       int
	state    CommandBufferState
	commands []RecordedCommand

	// Currently bound state (for validation)
	boundPipeline      *Pipeline
	boundDescriptorSet *DescriptorSet
	pushConstants      []byte
}

// NewCommandBuffer creates a new command buffer in INITIAL state.
func NewCommandBuffer() *CommandBuffer {
	id := nextCommandBufferID
	nextCommandBufferID++
	return &CommandBuffer{
		id:    id,
		state: CommandBufferStateInitial,
	}
}

// CommandBufferID returns the unique identifier.
func (cb *CommandBuffer) CommandBufferID() int {
	return cb.id
}

// State returns the current lifecycle state.
func (cb *CommandBuffer) State() CommandBufferState {
	return cb.state
}

// Commands returns all recorded commands.
func (cb *CommandBuffer) Commands() []RecordedCommand {
	result := make([]RecordedCommand, len(cb.commands))
	copy(result, cb.commands)
	return result
}

// BoundPipeline returns the currently bound pipeline (for validation).
func (cb *CommandBuffer) BoundPipeline() *Pipeline {
	return cb.boundPipeline
}

// BoundDescriptorSet returns the currently bound descriptor set (for validation).
func (cb *CommandBuffer) BoundDescriptorSet() *DescriptorSet {
	return cb.boundDescriptorSet
}

// =================================================================
// Lifecycle
// =================================================================

// Begin starts recording commands.
//
// Transitions: INITIAL -> RECORDING, or COMPLETE -> RECORDING (reuse).
// Returns an error if not in INITIAL or COMPLETE state.
func (cb *CommandBuffer) Begin() error {
	if cb.state != CommandBufferStateInitial && cb.state != CommandBufferStateComplete {
		return fmt.Errorf(
			"cannot begin recording: state is %s (expected initial or complete)",
			cb.state,
		)
	}
	cb.state = CommandBufferStateRecording
	cb.commands = cb.commands[:0]
	cb.boundPipeline = nil
	cb.boundDescriptorSet = nil
	cb.pushConstants = nil
	return nil
}

// End finishes recording commands.
//
// Transitions: RECORDING -> RECORDED.
// Returns an error if not in RECORDING state.
func (cb *CommandBuffer) End() error {
	if cb.state != CommandBufferStateRecording {
		return fmt.Errorf(
			"cannot end recording: state is %s (expected recording)",
			cb.state,
		)
	}
	cb.state = CommandBufferStateRecorded
	return nil
}

// Reset resets the command buffer to INITIAL state for reuse.
func (cb *CommandBuffer) Reset() {
	cb.state = CommandBufferStateInitial
	cb.commands = cb.commands[:0]
	cb.boundPipeline = nil
	cb.boundDescriptorSet = nil
	cb.pushConstants = nil
}

// MarkPending marks the command buffer as submitted (called by CommandQueue).
func (cb *CommandBuffer) MarkPending() {
	cb.state = CommandBufferStatePending
}

// MarkComplete marks the command buffer as finished (called by CommandQueue).
func (cb *CommandBuffer) MarkComplete() {
	cb.state = CommandBufferStateComplete
}

// requireRecording ensures we are in RECORDING state.
func (cb *CommandBuffer) requireRecording() error {
	if cb.state != CommandBufferStateRecording {
		return fmt.Errorf(
			"cannot record command: state is %s (expected recording)",
			cb.state,
		)
	}
	return nil
}

// =================================================================
// Compute commands
// =================================================================

// CmdBindPipeline binds a compute pipeline for subsequent dispatches.
//
// Must be called before CmdDispatch().
func (cb *CommandBuffer) CmdBindPipeline(pipeline *Pipeline) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	cb.boundPipeline = pipeline
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "bind_pipeline",
		Args:    map[string]interface{}{"pipeline_id": pipeline.PipelineID()},
	})
	return nil
}

// CmdBindDescriptorSet binds a descriptor set for subsequent dispatches.
//
// Must be called after CmdBindPipeline().
func (cb *CommandBuffer) CmdBindDescriptorSet(descriptorSet *DescriptorSet) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	cb.boundDescriptorSet = descriptorSet
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "bind_descriptor_set",
		Args:    map[string]interface{}{"set_id": descriptorSet.SetID()},
	})
	return nil
}

// CmdPushConstants sets push constant data for the next dispatch.
//
// Push constants are small pieces of data (<=128 bytes) sent inline
// with the dispatch command.
func (cb *CommandBuffer) CmdPushConstants(offset int, data []byte) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	cb.pushConstants = make([]byte, len(data))
	copy(cb.pushConstants, data)
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "push_constants",
		Args:    map[string]interface{}{"offset": offset, "size": len(data)},
	})
	return nil
}

// CmdDispatch launches a compute kernel.
//
// # Dispatch Dimensions
//
// The dispatch creates a 3D grid of workgroups:
//
//	Total threads = (groupX * groupY * groupZ) * (localX * localY * localZ)
//
// Returns an error if no pipeline is bound.
func (cb *CommandBuffer) CmdDispatch(groupX, groupY, groupZ int) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	if cb.boundPipeline == nil {
		return fmt.Errorf("cannot dispatch: no pipeline bound")
	}
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "dispatch",
		Args: map[string]interface{}{
			"group_x": groupX,
			"group_y": groupY,
			"group_z": groupZ,
		},
	})
	return nil
}

// CmdDispatchIndirect launches a compute kernel with grid dimensions from a GPU buffer.
//
// The buffer contains three uint32 values: (groupX, groupY, groupZ).
func (cb *CommandBuffer) CmdDispatchIndirect(buffer *Buffer, offset int) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	if cb.boundPipeline == nil {
		return fmt.Errorf("cannot dispatch: no pipeline bound")
	}
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "dispatch_indirect",
		Args: map[string]interface{}{
			"buffer_id": buffer.BufferID,
			"offset":    offset,
		},
	})
	return nil
}

// =================================================================
// Transfer commands
// =================================================================

// CmdCopyBuffer copies data between device buffers.
func (cb *CommandBuffer) CmdCopyBuffer(src, dst *Buffer, size, srcOffset, dstOffset int) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "copy_buffer",
		Args: map[string]interface{}{
			"src_id":     src.BufferID,
			"dst_id":     dst.BufferID,
			"size":       size,
			"src_offset": srcOffset,
			"dst_offset": dstOffset,
		},
	})
	return nil
}

// CmdFillBuffer fills a buffer with a constant byte value.
func (cb *CommandBuffer) CmdFillBuffer(buffer *Buffer, value, offset, size int) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	actualSize := size
	if actualSize <= 0 {
		actualSize = buffer.Size
	}
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "fill_buffer",
		Args: map[string]interface{}{
			"buffer_id": buffer.BufferID,
			"value":     value,
			"offset":    offset,
			"size":      actualSize,
		},
	})
	return nil
}

// CmdUpdateBuffer writes small data inline from CPU to device buffer.
//
// Limited to small updates (<= 65536 bytes).
func (cb *CommandBuffer) CmdUpdateBuffer(buffer *Buffer, offset int, data []byte) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	dataCopy := make([]byte, len(data))
	copy(dataCopy, data)
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "update_buffer",
		Args: map[string]interface{}{
			"buffer_id": buffer.BufferID,
			"offset":    offset,
			"data":      dataCopy,
		},
	})
	return nil
}

// =================================================================
// Synchronization commands
// =================================================================

// CmdPipelineBarrier inserts an execution + memory barrier.
func (cb *CommandBuffer) CmdPipelineBarrier(barrier PipelineBarrierDesc) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "pipeline_barrier",
		Args: map[string]interface{}{
			"src_stage":            barrier.SrcStage.String(),
			"dst_stage":            barrier.DstStage.String(),
			"memory_barrier_count": len(barrier.MemoryBarriers),
			"buffer_barrier_count": len(barrier.BufferBarriers),
		},
	})
	return nil
}

// CmdSetEvent signals an event from the GPU.
func (cb *CommandBuffer) CmdSetEvent(event *Event, stage PipelineStage) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "set_event",
		Args: map[string]interface{}{
			"event_id": event.EventID(),
			"stage":    stage.String(),
		},
	})
	return nil
}

// CmdWaitEvent waits for an event before proceeding.
func (cb *CommandBuffer) CmdWaitEvent(event *Event, srcStage, dstStage PipelineStage) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "wait_event",
		Args: map[string]interface{}{
			"event_id":  event.EventID(),
			"src_stage": srcStage.String(),
			"dst_stage": dstStage.String(),
		},
	})
	return nil
}

// CmdResetEvent resets an event from the GPU side.
func (cb *CommandBuffer) CmdResetEvent(event *Event, stage PipelineStage) error {
	if err := cb.requireRecording(); err != nil {
		return err
	}
	cb.commands = append(cb.commands, RecordedCommand{
		Command: "reset_event",
		Args: map[string]interface{}{
			"event_id": event.EventID(),
			"stage":    stage.String(),
		},
	})
	return nil
}
