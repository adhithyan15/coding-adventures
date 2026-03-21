package computeruntime

// CommandQueue -- FIFO submission of command buffers to a device.
//
// # How Submission Works
//
// When you submit command buffers to a queue, the runtime processes them
// sequentially, executing each recorded command against the Layer 6 device:
//
//	queue.Submit([cb1, cb2], fence)
//	    |
//	    +-- Execute cb1's commands:
//	    |   +-- bind_pipeline -> set current pipeline
//	    |   +-- bind_descriptor_set -> set current descriptors
//	    |   +-- dispatch(4, 1, 1) -> device.LaunchKernel() + device.Run()
//	    |   +-- pipeline_barrier -> (ensure completion, log trace)
//	    |
//	    +-- Execute cb2's commands:
//	    |   +-- copy_buffer -> device.memcpy
//	    |
//	    +-- Signal semaphores (if any)
//	    +-- Signal fence (if any)
//
// # Multiple Queues
//
// A device can have multiple queues. Queues of different types (compute,
// transfer) can execute in parallel.

import (
	"encoding/binary"
	"fmt"

	devicesimulator "github.com/adhithyan15/coding-adventures/code/packages/go/device-simulator"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// CommandQueue is a FIFO queue that submits command buffers to a device.
//
// Properties:
//   - Commands within a CB execute sequentially
//   - CBs within a submission execute sequentially
//   - Multiple submissions execute sequentially (FIFO)
//   - Multiple QUEUES can execute in parallel
type CommandQueue struct {
	queueType     QueueType
	queueIndex    int
	device        devicesimulator.AcceleratorDevice
	memoryManager *MemoryManager
	stats         *RuntimeStats
	totalCycles   int

	// Execution state
	currentPipeline      *Pipeline
	currentDescriptorSet *DescriptorSet
	currentPushConstants []byte
}

// NewCommandQueue creates a new command queue.
func NewCommandQueue(
	queueType QueueType,
	queueIndex int,
	device devicesimulator.AcceleratorDevice,
	memoryManager *MemoryManager,
	stats *RuntimeStats,
) *CommandQueue {
	return &CommandQueue{
		queueType:     queueType,
		queueIndex:    queueIndex,
		device:        device,
		memoryManager: memoryManager,
		stats:         stats,
	}
}

// QueueType returns what kind of work this queue handles.
func (q *CommandQueue) QueueType() QueueType {
	return q.queueType
}

// QueueIndex returns the index within queues of the same type.
func (q *CommandQueue) QueueIndex() int {
	return q.queueIndex
}

// TotalCycles returns the total device cycles consumed by this queue.
func (q *CommandQueue) TotalCycles() int {
	return q.totalCycles
}

// SubmitOptions holds optional parameters for Submit.
type SubmitOptions struct {
	WaitSemaphores   []*Semaphore
	SignalSemaphores []*Semaphore
	Fence            *Fence
}

// Submit submits command buffers for execution.
//
// # Submission Flow
//
//  1. Wait for all wait semaphores to be signaled
//  2. Execute each command buffer sequentially
//  3. Signal all signal semaphores
//  4. Signal the fence (if provided)
//
// Returns an error if any CB is not in RECORDED state or a wait semaphore
// is not signaled.
func (q *CommandQueue) Submit(
	commandBuffers []*CommandBuffer,
	opts *SubmitOptions,
) ([]RuntimeTrace, error) {
	var traces []RuntimeTrace

	if opts == nil {
		opts = &SubmitOptions{}
	}

	// Validate CB states
	for _, cb := range commandBuffers {
		if cb.State() != CommandBufferStateRecorded {
			return nil, fmt.Errorf(
				"CB#%d is in state %s, expected recorded",
				cb.CommandBufferID(), cb.State(),
			)
		}
	}

	// Wait on semaphores
	for _, sem := range opts.WaitSemaphores {
		if !sem.Signaled() {
			return nil, fmt.Errorf(
				"semaphore %d is not signaled -- cannot proceed (possible deadlock)",
				sem.SemaphoreID(),
			)
		}
		qt := q.queueType
		semID := sem.SemaphoreID()
		traces = append(traces, RuntimeTrace{
			TimestampCycles: q.totalCycles,
			EventType:       RuntimeEventSemaphoreWait,
			Description:     fmt.Sprintf("Wait on semaphore S%d", semID),
			QueueType:       &qt,
			SemaphoreID:     &semID,
		})
		sem.Reset()
	}

	// Log submission
	q.stats.TotalSubmissions++
	q.stats.TotalCommandBuffers += len(commandBuffers)

	cbIDs := make([]int, len(commandBuffers))
	for i, cb := range commandBuffers {
		cbIDs[i] = cb.CommandBufferID()
	}

	qt := q.queueType
	traces = append(traces, RuntimeTrace{
		TimestampCycles: q.totalCycles,
		EventType:       RuntimeEventSubmit,
		Description:     fmt.Sprintf("Submit CB %v to %s queue", cbIDs, q.queueType),
		QueueType:       &qt,
	})

	// Execute each command buffer
	for _, cb := range commandBuffers {
		cb.MarkPending()
		cbTraces, err := q.executeCommandBuffer(cb)
		if err != nil {
			return traces, err
		}
		traces = append(traces, cbTraces...)
		cb.MarkComplete()
	}

	// Signal semaphores
	for _, sem := range opts.SignalSemaphores {
		sem.Signal()
		q.stats.TotalSemaphoreSignals++
		semID := sem.SemaphoreID()
		traces = append(traces, RuntimeTrace{
			TimestampCycles: q.totalCycles,
			EventType:       RuntimeEventSemaphoreSignal,
			Description:     fmt.Sprintf("Signal semaphore S%d", semID),
			QueueType:       &qt,
			SemaphoreID:     &semID,
		})
	}

	// Signal fence
	if opts.Fence != nil {
		opts.Fence.Signal()
		fenceID := opts.Fence.FenceID()
		traces = append(traces, RuntimeTrace{
			TimestampCycles: q.totalCycles,
			EventType:       RuntimeEventFenceSignal,
			Description:     fmt.Sprintf("Signal fence F%d", fenceID),
			QueueType:       &qt,
			FenceID:         &fenceID,
		})
	}

	// Update stats
	q.stats.TotalDeviceCycles = q.totalCycles
	q.stats.UpdateUtilization()
	q.stats.Traces = append(q.stats.Traces, traces...)

	return traces, nil
}

// WaitIdle blocks until this queue has no pending work.
//
// In our synchronous simulation, submit() always runs to completion,
// so this is a no-op.
func (q *CommandQueue) WaitIdle() {
	// No-op in synchronous simulation
}

// executeCommandBuffer executes all commands in a command buffer.
func (q *CommandQueue) executeCommandBuffer(cb *CommandBuffer) ([]RuntimeTrace, error) {
	var traces []RuntimeTrace

	// Replay the CB's bind state
	q.currentPipeline = cb.BoundPipeline()
	q.currentDescriptorSet = cb.BoundDescriptorSet()

	qt := q.queueType
	cbID := cb.CommandBufferID()
	traces = append(traces, RuntimeTrace{
		TimestampCycles: q.totalCycles,
		EventType:       RuntimeEventBeginExecution,
		Description:     fmt.Sprintf("Begin CB#%d", cbID),
		QueueType:       &qt,
		CommandBufferID: &cbID,
	})

	for _, cmd := range cb.Commands() {
		cmdTraces, err := q.executeCommand(cmd)
		if err != nil {
			return traces, err
		}
		traces = append(traces, cmdTraces...)
	}

	traces = append(traces, RuntimeTrace{
		TimestampCycles: q.totalCycles,
		EventType:       RuntimeEventEndExecution,
		Description:     fmt.Sprintf("End CB#%d", cbID),
		QueueType:       &qt,
		CommandBufferID: &cbID,
	})

	return traces, nil
}

// executeCommand executes a single recorded command against the device.
func (q *CommandQueue) executeCommand(cmd RecordedCommand) ([]RuntimeTrace, error) {
	switch cmd.Command {
	case "bind_pipeline":
		return nil, nil
	case "bind_descriptor_set":
		return nil, nil
	case "push_constants":
		return nil, nil
	case "dispatch":
		return q.execDispatch(cmd.Args)
	case "dispatch_indirect":
		return q.execDispatchIndirect(cmd.Args)
	case "copy_buffer":
		return q.execCopyBuffer(cmd.Args)
	case "fill_buffer":
		return q.execFillBuffer(cmd.Args)
	case "update_buffer":
		return q.execUpdateBuffer(cmd.Args)
	case "pipeline_barrier":
		return q.execPipelineBarrier(cmd.Args)
	case "set_event":
		return nil, nil
	case "wait_event":
		return nil, nil
	case "reset_event":
		return nil, nil
	default:
		return nil, fmt.Errorf("unknown command: %s", cmd.Command)
	}
}

// =================================================================
// Command executors
// =================================================================

func (q *CommandQueue) execDispatch(args map[string]interface{}) ([]RuntimeTrace, error) {
	groupX := toInt(args["group_x"])
	groupY := toInt(args["group_y"])
	groupZ := toInt(args["group_z"])

	pipeline := q.currentPipeline
	if pipeline == nil {
		return nil, fmt.Errorf("no pipeline bound for dispatch")
	}

	shader := pipeline.Shader()

	var kernel devicesimulator.KernelDescriptor
	if shader.IsGPUStyle() {
		kernel = devicesimulator.KernelDescriptor{
			Name:               fmt.Sprintf("dispatch_%dx%dx%d", groupX, groupY, groupZ),
			Program:            shader.Code(),
			GridDim:            [3]int{groupX, groupY, groupZ},
			BlockDim:           [3]int{shader.LocalSize()[0], shader.LocalSize()[1], shader.LocalSize()[2]},
			RegistersPerThread: 32,
		}
	} else {
		kernel = devicesimulator.KernelDescriptor{
			Name:       fmt.Sprintf("op_%s", shader.Operation()),
			Operation:  shader.Operation(),
			InputData:  [][]float64{{1.0}},
			WeightData: [][]float64{{1.0}},
		}
	}

	q.device.LaunchKernel(kernel)
	deviceTraces := q.device.Run(10000)
	cycles := len(deviceTraces)
	q.totalCycles += cycles

	q.stats.TotalDispatches++

	qt := q.queueType
	dtInterfaces := make([]interface{}, len(deviceTraces))
	for i, dt := range deviceTraces {
		dtInterfaces[i] = dt
	}

	return []RuntimeTrace{{
		TimestampCycles: q.totalCycles,
		EventType:       RuntimeEventEndExecution,
		Description: fmt.Sprintf(
			"Dispatch (%d,%d,%d) completed in %d cycles",
			groupX, groupY, groupZ, cycles,
		),
		QueueType:    &qt,
		DeviceTraces: dtInterfaces,
	}}, nil
}

func (q *CommandQueue) execDispatchIndirect(args map[string]interface{}) ([]RuntimeTrace, error) {
	bufferID := toInt(args["buffer_id"])
	offset := toInt(args["offset"])

	data := q.memoryManager.GetBufferData(bufferID)
	if len(data) < offset+12 {
		return nil, fmt.Errorf("buffer too small for indirect dispatch")
	}

	groupX := int(binary.LittleEndian.Uint32(data[offset : offset+4]))
	groupY := int(binary.LittleEndian.Uint32(data[offset+4 : offset+8]))
	groupZ := int(binary.LittleEndian.Uint32(data[offset+8 : offset+12]))

	return q.execDispatch(map[string]interface{}{
		"group_x": groupX,
		"group_y": groupY,
		"group_z": groupZ,
	})
}

func (q *CommandQueue) execCopyBuffer(args map[string]interface{}) ([]RuntimeTrace, error) {
	srcID := toInt(args["src_id"])
	dstID := toInt(args["dst_id"])
	size := toInt(args["size"])
	srcOffset := toInt(args["src_offset"])
	dstOffset := toInt(args["dst_offset"])

	srcData := q.memoryManager.GetBufferData(srcID)
	dstData := q.memoryManager.GetBufferData(dstID)

	copy(dstData[dstOffset:dstOffset+size], srcData[srcOffset:srcOffset+size])

	srcBuf, err := q.memoryManager.GetBuffer(srcID)
	if err != nil {
		return nil, err
	}
	dstBuf, err := q.memoryManager.GetBuffer(dstID)
	if err != nil {
		return nil, err
	}

	dataBytes, readCycles, err := q.device.MemcpyDeviceToHost(srcBuf.DeviceAddress+srcOffset, size)
	if err != nil {
		return nil, err
	}
	writeCycles, err := q.device.MemcpyHostToDevice(dstBuf.DeviceAddress+dstOffset, dataBytes)
	if err != nil {
		return nil, err
	}

	cycles := readCycles + writeCycles
	q.totalCycles += cycles
	q.stats.TotalTransfers++

	qt := q.queueType
	return []RuntimeTrace{{
		TimestampCycles: q.totalCycles,
		EventType:       RuntimeEventMemoryTransfer,
		Description: fmt.Sprintf(
			"Copy %d bytes: buf#%d -> buf#%d (%d cycles)",
			size, srcID, dstID, cycles,
		),
		QueueType: &qt,
	}}, nil
}

func (q *CommandQueue) execFillBuffer(args map[string]interface{}) ([]RuntimeTrace, error) {
	bufferID := toInt(args["buffer_id"])
	value := toInt(args["value"])
	offset := toInt(args["offset"])
	size := toInt(args["size"])

	bufData := q.memoryManager.GetBufferData(bufferID)
	fillByte := byte(value & 0xFF)
	for i := offset; i < offset+size; i++ {
		bufData[i] = fillByte
	}

	buf, err := q.memoryManager.GetBuffer(bufferID)
	if err != nil {
		return nil, err
	}
	fillBytes := make([]byte, size)
	for i := range fillBytes {
		fillBytes[i] = fillByte
	}
	_, err = q.device.MemcpyHostToDevice(buf.DeviceAddress+offset, fillBytes)
	if err != nil {
		return nil, err
	}

	q.stats.TotalTransfers++
	return nil, nil
}

func (q *CommandQueue) execUpdateBuffer(args map[string]interface{}) ([]RuntimeTrace, error) {
	bufferID := toInt(args["buffer_id"])
	offset := toInt(args["offset"])
	data, _ := args["data"].([]byte)

	bufData := q.memoryManager.GetBufferData(bufferID)
	copy(bufData[offset:offset+len(data)], data)

	buf, err := q.memoryManager.GetBuffer(bufferID)
	if err != nil {
		return nil, err
	}
	_, err = q.device.MemcpyHostToDevice(buf.DeviceAddress+offset, data)
	if err != nil {
		return nil, err
	}

	q.stats.TotalTransfers++
	return nil, nil
}

func (q *CommandQueue) execPipelineBarrier(args map[string]interface{}) ([]RuntimeTrace, error) {
	q.stats.TotalBarriers++

	srcStage, _ := args["src_stage"].(string)
	dstStage, _ := args["dst_stage"].(string)

	qt := q.queueType
	return []RuntimeTrace{{
		TimestampCycles: q.totalCycles,
		EventType:       RuntimeEventBarrier,
		Description:     fmt.Sprintf("Barrier: %s -> %s", srcStage, dstStage),
		QueueType:       &qt,
	}}, nil
}

// toInt converts an interface{} to int, handling both int and float64 from maps.
func toInt(v interface{}) int {
	switch val := v.(type) {
	case int:
		return val
	case float64:
		return int(val)
	case int64:
		return int(val)
	default:
		return 0
	}
}

// Ensure gpucore is used (the import is needed for KernelDescriptor.Program field type).
var _ []gpucore.Instruction
