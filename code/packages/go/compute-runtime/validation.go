package computeruntime

// ValidationLayer -- catches GPU programming errors early.
//
// # What is a Validation Layer?
//
// In Vulkan, validation layers are optional middleware that check every API
// call for errors. They are enabled during development and disabled in
// production (for performance). Common errors they catch:
//
//   - Dispatching without binding a pipeline
//   - Using a freed buffer in a descriptor set
//   - Missing a barrier between write and read
//   - Mapping a DEVICE_LOCAL-only buffer
//   - Exceeding device limits
//
// Our validation layer checks every operation and is always enabled
// (since we are a simulator, not a production runtime).

import "fmt"

// =========================================================================
// ValidationError -- a validation check failure
// =========================================================================

// ValidationError represents a GPU programming mistake -- something that
// would cause undefined behavior or crashes on real hardware.
type ValidationError struct {
	Message string
}

// Error implements the error interface.
func (e *ValidationError) Error() string {
	return e.Message
}

// NewValidationError creates a new ValidationError.
func NewValidationError(msg string) *ValidationError {
	result, _ := StartNew[*ValidationError]("compute-runtime.NewValidationError", nil,
		func(op *Operation[*ValidationError], rf *ResultFactory[*ValidationError]) *OperationResult[*ValidationError] {
			return rf.Generate(true, false, &ValidationError{Message: msg})
		}).GetResult()
	return result
}

// =========================================================================
// ValidationLayer -- validates runtime operations
// =========================================================================

// ValidationLayer validates runtime operations and raises clear error messages.
//
// # What It Checks
//
//  1. Command buffer state transitions (cannot record without Begin())
//  2. Pipeline/descriptor binding (cannot dispatch without binding both)
//  3. Memory type compatibility (cannot map DEVICE_LOCAL)
//  4. Buffer usage flags (cannot use STORAGE buffer as TRANSFER_SRC)
//  5. Freed resource detection (cannot use freed buffers)
//  6. Barrier correctness (warn on write->read without barrier)
type ValidationLayer struct {
	warnings         []string
	errors           []string
	writtenBuffers   map[int]bool
	barrieredBuffers map[int]bool
}

// NewValidationLayer creates a new validation layer.
func NewValidationLayer() *ValidationLayer {
	result, _ := StartNew[*ValidationLayer]("compute-runtime.NewValidationLayer", nil,
		func(op *Operation[*ValidationLayer], rf *ResultFactory[*ValidationLayer]) *OperationResult[*ValidationLayer] {
			return rf.Generate(true, false, &ValidationLayer{
				writtenBuffers:   make(map[int]bool),
				barrieredBuffers: make(map[int]bool),
			})
		}).GetResult()
	return result
}

// Warnings returns all validation warnings issued so far.
func (vl *ValidationLayer) Warnings() []string {
	result, _ := StartNew[[]string]("compute-runtime.ValidationLayer.Warnings", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			res := make([]string, len(vl.warnings))
			copy(res, vl.warnings)
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Errors returns all validation errors issued so far.
func (vl *ValidationLayer) Errors() []string {
	result, _ := StartNew[[]string]("compute-runtime.ValidationLayer.Errors", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			res := make([]string, len(vl.errors))
			copy(res, vl.errors)
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Clear clears all warnings and errors.
func (vl *ValidationLayer) Clear() {
	_, _ = StartNew[struct{}]("compute-runtime.ValidationLayer.Clear", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			vl.warnings = vl.warnings[:0]
			vl.errors = vl.errors[:0]
			vl.writtenBuffers = make(map[int]bool)
			vl.barrieredBuffers = make(map[int]bool)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Command buffer validation ---

// ValidateBegin validates that Begin() is allowed on the command buffer.
func (vl *ValidationLayer) ValidateBegin(cb *CommandBuffer) error {
	_, err := StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateBegin", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("cb_id", cb.CommandBufferID())
			if cb.State() != CommandBufferStateInitial && cb.State() != CommandBufferStateComplete {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf(
						"Cannot begin CB#%d: state is %s (expected initial or complete)",
						cb.CommandBufferID(), cb.State(),
					),
				})
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// ValidateEnd validates that End() is allowed on the command buffer.
func (vl *ValidationLayer) ValidateEnd(cb *CommandBuffer) error {
	_, err := StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateEnd", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("cb_id", cb.CommandBufferID())
			if cb.State() != CommandBufferStateRecording {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf(
						"Cannot end CB#%d: state is %s (expected recording)",
						cb.CommandBufferID(), cb.State(),
					),
				})
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// ValidateSubmit validates that a CB can be submitted.
func (vl *ValidationLayer) ValidateSubmit(cb *CommandBuffer) error {
	_, err := StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateSubmit", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("cb_id", cb.CommandBufferID())
			if cb.State() != CommandBufferStateRecorded {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf(
						"Cannot submit CB#%d: state is %s (expected recorded)",
						cb.CommandBufferID(), cb.State(),
					),
				})
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// --- Dispatch validation ---

// ValidateDispatch validates a dispatch command.
func (vl *ValidationLayer) ValidateDispatch(
	cb *CommandBuffer,
	groupX, groupY, groupZ int,
) error {
	_, err := StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateDispatch", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("groupX", groupX)
			op.AddProperty("groupY", groupY)
			op.AddProperty("groupZ", groupZ)
			if cb.BoundPipeline() == nil {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf(
						"Cannot dispatch in CB#%d: no pipeline bound (call CmdBindPipeline first)",
						cb.CommandBufferID(),
					),
				})
			}
			if groupX <= 0 || groupY <= 0 || groupZ <= 0 {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf(
						"Dispatch dimensions must be positive: (%d, %d, %d)",
						groupX, groupY, groupZ,
					),
				})
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// --- Memory validation ---

// ValidateMap validates that a buffer can be mapped.
func (vl *ValidationLayer) ValidateMap(buffer *Buffer) error {
	_, err := StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateMap", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			if buffer.Freed {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf("Cannot map freed buffer %d", buffer.BufferID),
				})
			}
			if buffer.Mapped {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf("Buffer %d is already mapped", buffer.BufferID),
				})
			}
			if !buffer.MemType.Has(MemoryTypeHostVisible) {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf(
						"Cannot map buffer %d: not HOST_VISIBLE (type=%s). Use a staging buffer for DEVICE_LOCAL memory.",
						buffer.BufferID, buffer.MemType,
					),
				})
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// ValidateBufferUsage validates that a buffer has the required usage flags.
func (vl *ValidationLayer) ValidateBufferUsage(buffer *Buffer, requiredUsage BufferUsage) error {
	_, err := StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateBufferUsage", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			if !buffer.Usage.Has(requiredUsage) {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf(
						"Buffer %d lacks required usage %d (has %d)",
						buffer.BufferID, requiredUsage, buffer.Usage,
					),
				})
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// ValidateBufferNotFreed validates that a buffer is not freed.
func (vl *ValidationLayer) ValidateBufferNotFreed(buffer *Buffer) error {
	_, err := StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateBufferNotFreed", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", buffer.BufferID)
			if buffer.Freed {
				return rf.Fail(struct{}{}, &ValidationError{
					Message: fmt.Sprintf("Buffer %d has been freed", buffer.BufferID),
				})
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// --- Barrier validation ---

// RecordWrite records that a buffer was written to (for barrier checking).
func (vl *ValidationLayer) RecordWrite(bufferID int) {
	_, _ = StartNew[struct{}]("compute-runtime.ValidationLayer.RecordWrite", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", bufferID)
			vl.writtenBuffers[bufferID] = true
			delete(vl.barrieredBuffers, bufferID)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RecordBarrier records that a barrier was placed.
// If bufferIDs is nil, it is treated as a global barrier covering all written buffers.
func (vl *ValidationLayer) RecordBarrier(bufferIDs map[int]bool) {
	_, _ = StartNew[struct{}]("compute-runtime.ValidationLayer.RecordBarrier", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if bufferIDs == nil {
				for id := range vl.writtenBuffers {
					vl.barrieredBuffers[id] = true
				}
			} else {
				for id := range bufferIDs {
					vl.barrieredBuffers[id] = true
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ValidateReadAfterWrite warns if reading a buffer that was written without a barrier.
func (vl *ValidationLayer) ValidateReadAfterWrite(bufferID int) {
	_, _ = StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateReadAfterWrite", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("buffer_id", bufferID)
			if vl.writtenBuffers[bufferID] && !vl.barrieredBuffers[bufferID] {
				vl.warnings = append(vl.warnings, fmt.Sprintf(
					"Reading buffer %d after write without barrier. Insert CmdPipelineBarrier() between write and read.",
					bufferID,
				))
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// --- Descriptor set validation ---

// ValidateDescriptorSet validates that a descriptor set is compatible with a pipeline.
func (vl *ValidationLayer) ValidateDescriptorSet(
	descriptorSet *DescriptorSet,
	pipeline *Pipeline,
) error {
	_, err := StartNew[struct{}]("compute-runtime.ValidationLayer.ValidateDescriptorSet", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("set_id", descriptorSet.SetID())
			layout := pipeline.Layout()
			if len(layout.SetLayouts()) == 0 {
				return rf.Generate(true, false, struct{}{})
			}

			expectedLayout := layout.SetLayouts()[0]
			for _, bindingDef := range expectedLayout.Bindings() {
				buf := descriptorSet.GetBuffer(bindingDef.Binding)
				if buf == nil {
					vl.warnings = append(vl.warnings, fmt.Sprintf(
						"Binding %d not set in descriptor set %d",
						bindingDef.Binding, descriptorSet.SetID(),
					))
				} else if buf.Freed {
					return rf.Fail(struct{}{}, &ValidationError{
						Message: fmt.Sprintf(
							"Binding %d uses freed buffer %d",
							bindingDef.Binding, buf.BufferID,
						),
					})
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}
