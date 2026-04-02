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
	result, _ := StartNew[*CommandBuffer]("compute-runtime.NewCommandBuffer", nil,
		func(op *Operation[*CommandBuffer], rf *ResultFactory[*CommandBuffer]) *OperationResult[*CommandBuffer] {
			id := nextCommandBufferID
			nextCommandBufferID++
			return rf.Generate(true, false, &CommandBuffer{
				id:    id,
				state: CommandBufferStateInitial,
			})
		}).GetResult()
	return result
}

// CommandBufferID returns the unique identifier.
func (cb *CommandBuffer) CommandBufferID() int {
	result, _ := StartNew[int]("compute-runtime.CommandBuffer.CommandBufferID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, cb.id)
		}).GetResult()
	return result
}

// State returns the current lifecycle state.
func (cb *CommandBuffer) State() CommandBufferState {
	result, _ := StartNew[CommandBufferState]("compute-runtime.CommandBuffer.State", 0,
		func(op *Operation[CommandBufferState], rf *ResultFactory[CommandBufferState]) *OperationResult[CommandBufferState] {
			return rf.Generate(true, false, cb.state)
		}).GetResult()
	return result
}

// Commands returns all recorded commands.
func (cb *CommandBuffer) Commands() []RecordedCommand {
	result, _ := StartNew[[]RecordedCommand]("compute-runtime.CommandBuffer.Commands", nil,
		func(op *Operation[[]RecordedCommand], rf *ResultFactory[[]RecordedCommand]) *OperationResult[[]RecordedCommand] {
			res := make([]RecordedCommand, len(cb.commands))
			copy(res, cb.commands)
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// BoundPipeline returns the currently bound pipeline (for validation).
func (cb *CommandBuffer) BoundPipeline() *Pipeline {
	result, _ := StartNew[*Pipeline]("compute-runtime.CommandBuffer.BoundPipeline", nil,
		func(op *Operation[*Pipeline], rf *ResultFactory[*Pipeline]) *OperationResult[*Pipeline] {
			return rf.Generate(true, false, cb.boundPipeline)
		}).GetResult()
	return result
}

// BoundDescriptorSet returns the currently bound descriptor set (for validation).
func (cb *CommandBuffer) BoundDescriptorSet() *DescriptorSet {
	result, _ := StartNew[*DescriptorSet]("compute-runtime.CommandBuffer.BoundDescriptorSet", nil,
		func(op *Operation[*DescriptorSet], rf *ResultFactory[*DescriptorSet]) *OperationResult[*DescriptorSet] {
			return rf.Generate(true, false, cb.boundDescriptorSet)
		}).GetResult()
	return result
}

// =================================================================
// Lifecycle
// =================================================================

// Begin starts recording commands.
//
// Transitions: INITIAL -> RECORDING, or COMPLETE -> RECORDING (reuse).
// Returns an error if not in INITIAL or COMPLETE state.
func (cb *CommandBuffer) Begin() error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.Begin", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if cb.state != CommandBufferStateInitial && cb.state != CommandBufferStateComplete {
				return rf.Fail(struct{}{}, fmt.Errorf(
					"cannot begin recording: state is %s (expected initial or complete)",
					cb.state,
				))
			}
			cb.state = CommandBufferStateRecording
			cb.commands = cb.commands[:0]
			cb.boundPipeline = nil
			cb.boundDescriptorSet = nil
			cb.pushConstants = nil
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// End finishes recording commands.
//
// Transitions: RECORDING -> RECORDED.
// Returns an error if not in RECORDING state.
func (cb *CommandBuffer) End() error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.End", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if cb.state != CommandBufferStateRecording {
				return rf.Fail(struct{}{}, fmt.Errorf(
					"cannot end recording: state is %s (expected recording)",
					cb.state,
				))
			}
			cb.state = CommandBufferStateRecorded
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Reset resets the command buffer to INITIAL state for reuse.
func (cb *CommandBuffer) Reset() {
	_, _ = StartNew[struct{}]("compute-runtime.CommandBuffer.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			cb.state = CommandBufferStateInitial
			cb.commands = cb.commands[:0]
			cb.boundPipeline = nil
			cb.boundDescriptorSet = nil
			cb.pushConstants = nil
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// MarkPending marks the command buffer as submitted (called by CommandQueue).
func (cb *CommandBuffer) MarkPending() {
	_, _ = StartNew[struct{}]("compute-runtime.CommandBuffer.MarkPending", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			cb.state = CommandBufferStatePending
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// MarkComplete marks the command buffer as finished (called by CommandQueue).
func (cb *CommandBuffer) MarkComplete() {
	_, _ = StartNew[struct{}]("compute-runtime.CommandBuffer.MarkComplete", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			cb.state = CommandBufferStateComplete
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
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
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdBindPipeline", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pipeline_id", pipeline.PipelineID())
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			cb.boundPipeline = pipeline
			cb.commands = append(cb.commands, RecordedCommand{
				Command: "bind_pipeline",
				Args:    map[string]interface{}{"pipeline_id": pipeline.PipelineID()},
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CmdBindDescriptorSet binds a descriptor set for subsequent dispatches.
//
// Must be called after CmdBindPipeline().
func (cb *CommandBuffer) CmdBindDescriptorSet(descriptorSet *DescriptorSet) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdBindDescriptorSet", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("set_id", descriptorSet.SetID())
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			cb.boundDescriptorSet = descriptorSet
			cb.commands = append(cb.commands, RecordedCommand{
				Command: "bind_descriptor_set",
				Args:    map[string]interface{}{"set_id": descriptorSet.SetID()},
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CmdPushConstants sets push constant data for the next dispatch.
//
// Push constants are small pieces of data (<=128 bytes) sent inline
// with the dispatch command.
func (cb *CommandBuffer) CmdPushConstants(offset int, data []byte) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdPushConstants", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("offset", offset)
			op.AddProperty("size", len(data))
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			cb.pushConstants = make([]byte, len(data))
			copy(cb.pushConstants, data)
			cb.commands = append(cb.commands, RecordedCommand{
				Command: "push_constants",
				Args:    map[string]interface{}{"offset": offset, "size": len(data)},
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
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
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdDispatch", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("groupX", groupX)
			op.AddProperty("groupY", groupY)
			op.AddProperty("groupZ", groupZ)
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if cb.boundPipeline == nil {
				return rf.Fail(struct{}{}, fmt.Errorf("cannot dispatch: no pipeline bound"))
			}
			cb.commands = append(cb.commands, RecordedCommand{
				Command: "dispatch",
				Args: map[string]interface{}{
					"group_x": groupX,
					"group_y": groupY,
					"group_z": groupZ,
				},
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CmdDispatchIndirect launches a compute kernel with grid dimensions from a GPU buffer.
//
// The buffer contains three uint32 values: (groupX, groupY, groupZ).
func (cb *CommandBuffer) CmdDispatchIndirect(buffer *Buffer, offset int) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdDispatchIndirect", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			op.AddProperty("offset", offset)
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			if cb.boundPipeline == nil {
				return rf.Fail(struct{}{}, fmt.Errorf("cannot dispatch: no pipeline bound"))
			}
			cb.commands = append(cb.commands, RecordedCommand{
				Command: "dispatch_indirect",
				Args: map[string]interface{}{
					"buffer_id": buffer.BufferID,
					"offset":    offset,
				},
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// =================================================================
// Transfer commands
// =================================================================

// CmdCopyBuffer copies data between device buffers.
func (cb *CommandBuffer) CmdCopyBuffer(src, dst *Buffer, size, srcOffset, dstOffset int) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdCopyBuffer", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("src_id", src.BufferID)
			op.AddProperty("dst_id", dst.BufferID)
			op.AddProperty("size", size)
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
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
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CmdFillBuffer fills a buffer with a constant byte value.
func (cb *CommandBuffer) CmdFillBuffer(buffer *Buffer, value, offset, size int) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdFillBuffer", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			op.AddProperty("value", value)
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
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
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CmdUpdateBuffer writes small data inline from CPU to device buffer.
//
// Limited to small updates (<= 65536 bytes).
func (cb *CommandBuffer) CmdUpdateBuffer(buffer *Buffer, offset int, data []byte) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdUpdateBuffer", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			op.AddProperty("offset", offset)
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
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
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// =================================================================
// Synchronization commands
// =================================================================

// CmdPipelineBarrier inserts an execution + memory barrier.
func (cb *CommandBuffer) CmdPipelineBarrier(barrier PipelineBarrierDesc) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdPipelineBarrier", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("src_stage", barrier.SrcStage.String())
			op.AddProperty("dst_stage", barrier.DstStage.String())
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
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
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CmdSetEvent signals an event from the GPU.
func (cb *CommandBuffer) CmdSetEvent(event *Event, stage PipelineStage) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdSetEvent", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("event_id", event.EventID())
			op.AddProperty("stage", stage.String())
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			cb.commands = append(cb.commands, RecordedCommand{
				Command: "set_event",
				Args: map[string]interface{}{
					"event_id": event.EventID(),
					"stage":    stage.String(),
				},
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CmdWaitEvent waits for an event before proceeding.
func (cb *CommandBuffer) CmdWaitEvent(event *Event, srcStage, dstStage PipelineStage) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdWaitEvent", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("event_id", event.EventID())
			op.AddProperty("src_stage", srcStage.String())
			op.AddProperty("dst_stage", dstStage.String())
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			cb.commands = append(cb.commands, RecordedCommand{
				Command: "wait_event",
				Args: map[string]interface{}{
					"event_id":  event.EventID(),
					"src_stage": srcStage.String(),
					"dst_stage": dstStage.String(),
				},
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// CmdResetEvent resets an event from the GPU side.
func (cb *CommandBuffer) CmdResetEvent(event *Event, stage PipelineStage) error {
	_, err := StartNew[struct{}]("compute-runtime.CommandBuffer.CmdResetEvent", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("event_id", event.EventID())
			op.AddProperty("stage", stage.String())
			if err := cb.requireRecording(); err != nil {
				return rf.Fail(struct{}{}, err)
			}
			cb.commands = append(cb.commands, RecordedCommand{
				Command: "reset_event",
				Args: map[string]interface{}{
					"event_id": event.EventID(),
					"stage":    stage.String(),
				},
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}
