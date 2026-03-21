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
	return &ValidationError{Message: msg}
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
	return &ValidationLayer{
		writtenBuffers:   make(map[int]bool),
		barrieredBuffers: make(map[int]bool),
	}
}

// Warnings returns all validation warnings issued so far.
func (vl *ValidationLayer) Warnings() []string {
	result := make([]string, len(vl.warnings))
	copy(result, vl.warnings)
	return result
}

// Errors returns all validation errors issued so far.
func (vl *ValidationLayer) Errors() []string {
	result := make([]string, len(vl.errors))
	copy(result, vl.errors)
	return result
}

// Clear clears all warnings and errors.
func (vl *ValidationLayer) Clear() {
	vl.warnings = vl.warnings[:0]
	vl.errors = vl.errors[:0]
	vl.writtenBuffers = make(map[int]bool)
	vl.barrieredBuffers = make(map[int]bool)
}

// --- Command buffer validation ---

// ValidateBegin validates that Begin() is allowed on the command buffer.
func (vl *ValidationLayer) ValidateBegin(cb *CommandBuffer) error {
	if cb.State() != CommandBufferStateInitial && cb.State() != CommandBufferStateComplete {
		return &ValidationError{
			Message: fmt.Sprintf(
				"Cannot begin CB#%d: state is %s (expected initial or complete)",
				cb.CommandBufferID(), cb.State(),
			),
		}
	}
	return nil
}

// ValidateEnd validates that End() is allowed on the command buffer.
func (vl *ValidationLayer) ValidateEnd(cb *CommandBuffer) error {
	if cb.State() != CommandBufferStateRecording {
		return &ValidationError{
			Message: fmt.Sprintf(
				"Cannot end CB#%d: state is %s (expected recording)",
				cb.CommandBufferID(), cb.State(),
			),
		}
	}
	return nil
}

// ValidateSubmit validates that a CB can be submitted.
func (vl *ValidationLayer) ValidateSubmit(cb *CommandBuffer) error {
	if cb.State() != CommandBufferStateRecorded {
		return &ValidationError{
			Message: fmt.Sprintf(
				"Cannot submit CB#%d: state is %s (expected recorded)",
				cb.CommandBufferID(), cb.State(),
			),
		}
	}
	return nil
}

// --- Dispatch validation ---

// ValidateDispatch validates a dispatch command.
func (vl *ValidationLayer) ValidateDispatch(
	cb *CommandBuffer,
	groupX, groupY, groupZ int,
) error {
	if cb.BoundPipeline() == nil {
		return &ValidationError{
			Message: fmt.Sprintf(
				"Cannot dispatch in CB#%d: no pipeline bound (call CmdBindPipeline first)",
				cb.CommandBufferID(),
			),
		}
	}
	if groupX <= 0 || groupY <= 0 || groupZ <= 0 {
		return &ValidationError{
			Message: fmt.Sprintf(
				"Dispatch dimensions must be positive: (%d, %d, %d)",
				groupX, groupY, groupZ,
			),
		}
	}
	return nil
}

// --- Memory validation ---

// ValidateMap validates that a buffer can be mapped.
func (vl *ValidationLayer) ValidateMap(buffer *Buffer) error {
	if buffer.Freed {
		return &ValidationError{
			Message: fmt.Sprintf("Cannot map freed buffer %d", buffer.BufferID),
		}
	}
	if buffer.Mapped {
		return &ValidationError{
			Message: fmt.Sprintf("Buffer %d is already mapped", buffer.BufferID),
		}
	}
	if !buffer.MemType.Has(MemoryTypeHostVisible) {
		return &ValidationError{
			Message: fmt.Sprintf(
				"Cannot map buffer %d: not HOST_VISIBLE (type=%s). Use a staging buffer for DEVICE_LOCAL memory.",
				buffer.BufferID, buffer.MemType,
			),
		}
	}
	return nil
}

// ValidateBufferUsage validates that a buffer has the required usage flags.
func (vl *ValidationLayer) ValidateBufferUsage(buffer *Buffer, requiredUsage BufferUsage) error {
	if !buffer.Usage.Has(requiredUsage) {
		return &ValidationError{
			Message: fmt.Sprintf(
				"Buffer %d lacks required usage %d (has %d)",
				buffer.BufferID, requiredUsage, buffer.Usage,
			),
		}
	}
	return nil
}

// ValidateBufferNotFreed validates that a buffer is not freed.
func (vl *ValidationLayer) ValidateBufferNotFreed(buffer *Buffer) error {
	if buffer.Freed {
		return &ValidationError{
			Message: fmt.Sprintf("Buffer %d has been freed", buffer.BufferID),
		}
	}
	return nil
}

// --- Barrier validation ---

// RecordWrite records that a buffer was written to (for barrier checking).
func (vl *ValidationLayer) RecordWrite(bufferID int) {
	vl.writtenBuffers[bufferID] = true
	delete(vl.barrieredBuffers, bufferID)
}

// RecordBarrier records that a barrier was placed.
// If bufferIDs is nil, it is treated as a global barrier covering all written buffers.
func (vl *ValidationLayer) RecordBarrier(bufferIDs map[int]bool) {
	if bufferIDs == nil {
		for id := range vl.writtenBuffers {
			vl.barrieredBuffers[id] = true
		}
	} else {
		for id := range bufferIDs {
			vl.barrieredBuffers[id] = true
		}
	}
}

// ValidateReadAfterWrite warns if reading a buffer that was written without a barrier.
func (vl *ValidationLayer) ValidateReadAfterWrite(bufferID int) {
	if vl.writtenBuffers[bufferID] && !vl.barrieredBuffers[bufferID] {
		vl.warnings = append(vl.warnings, fmt.Sprintf(
			"Reading buffer %d after write without barrier. Insert CmdPipelineBarrier() between write and read.",
			bufferID,
		))
	}
}

// --- Descriptor set validation ---

// ValidateDescriptorSet validates that a descriptor set is compatible with a pipeline.
func (vl *ValidationLayer) ValidateDescriptorSet(
	descriptorSet *DescriptorSet,
	pipeline *Pipeline,
) error {
	layout := pipeline.Layout()
	if len(layout.SetLayouts()) == 0 {
		return nil
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
			return &ValidationError{
				Message: fmt.Sprintf(
					"Binding %d uses freed buffer %d",
					bindingDef.Binding, buf.BufferID,
				),
			}
		}
	}
	return nil
}
