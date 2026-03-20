package computeruntime

import "testing"

// =========================================================================
// ValidationLayer basic tests
// =========================================================================

func TestValidationLayerCreation(t *testing.T) {
	vl := NewValidationLayer()
	if len(vl.Warnings()) != 0 {
		t.Errorf("new VL should have 0 warnings, got %d", len(vl.Warnings()))
	}
	if len(vl.Errors()) != 0 {
		t.Errorf("new VL should have 0 errors, got %d", len(vl.Errors()))
	}
}

func TestValidationLayerClear(t *testing.T) {
	vl := NewValidationLayer()
	vl.RecordWrite(0)
	vl.ValidateReadAfterWrite(0)
	if len(vl.Warnings()) == 0 {
		t.Error("should have warnings")
	}
	vl.Clear()
	if len(vl.Warnings()) != 0 {
		t.Errorf("after Clear, should have 0 warnings, got %d", len(vl.Warnings()))
	}
}

// =========================================================================
// Command buffer validation
// =========================================================================

func TestValidateBeginValid(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	if err := vl.ValidateBegin(cb); err != nil {
		t.Errorf("ValidateBegin should pass: %v", err)
	}
}

func TestValidateBeginInvalid(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	_ = cb.Begin() // now in RECORDING
	if err := vl.ValidateBegin(cb); err == nil {
		t.Error("ValidateBegin in RECORDING state should fail")
	}
}

func TestValidateEndValid(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	_ = cb.Begin()
	if err := vl.ValidateEnd(cb); err != nil {
		t.Errorf("ValidateEnd should pass: %v", err)
	}
}

func TestValidateEndInvalid(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	if err := vl.ValidateEnd(cb); err == nil {
		t.Error("ValidateEnd in INITIAL state should fail")
	}
}

func TestValidateSubmitValid(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	_ = cb.Begin()
	_ = cb.End()
	if err := vl.ValidateSubmit(cb); err != nil {
		t.Errorf("ValidateSubmit should pass: %v", err)
	}
}

func TestValidateSubmitInvalid(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	if err := vl.ValidateSubmit(cb); err == nil {
		t.Error("ValidateSubmit in INITIAL state should fail")
	}
}

// =========================================================================
// Dispatch validation
// =========================================================================

func TestValidateDispatchNoPipeline(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	_ = cb.Begin()
	if err := vl.ValidateDispatch(cb, 1, 1, 1); err == nil {
		t.Error("ValidateDispatch without pipeline should fail")
	}
}

func TestValidateDispatchInvalidDimensions(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	_ = cb.Begin()
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)
	_ = cb.CmdBindPipeline(pipeline)

	if err := vl.ValidateDispatch(cb, 0, 1, 1); err == nil {
		t.Error("ValidateDispatch with groupX=0 should fail")
	}
	if err := vl.ValidateDispatch(cb, 1, -1, 1); err == nil {
		t.Error("ValidateDispatch with groupY=-1 should fail")
	}
}

func TestValidateDispatchValid(t *testing.T) {
	vl := NewValidationLayer()
	cb := NewCommandBuffer()
	_ = cb.Begin()
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	layout := NewPipelineLayout(nil, 0)
	pipeline := NewPipeline(shader, layout)
	_ = cb.CmdBindPipeline(pipeline)

	if err := vl.ValidateDispatch(cb, 4, 1, 1); err != nil {
		t.Errorf("ValidateDispatch should pass: %v", err)
	}
}

// =========================================================================
// Memory validation
// =========================================================================

func TestValidateMapValid(t *testing.T) {
	vl := NewValidationLayer()
	buf := &Buffer{
		BufferID: 0,
		Size:     256,
		MemType:  MemoryTypeHostVisible,
	}
	if err := vl.ValidateMap(buf); err != nil {
		t.Errorf("ValidateMap should pass: %v", err)
	}
}

func TestValidateMapFreed(t *testing.T) {
	vl := NewValidationLayer()
	buf := &Buffer{BufferID: 0, MemType: MemoryTypeHostVisible, Freed: true}
	if err := vl.ValidateMap(buf); err == nil {
		t.Error("ValidateMap on freed buffer should fail")
	}
}

func TestValidateMapAlreadyMapped(t *testing.T) {
	vl := NewValidationLayer()
	buf := &Buffer{BufferID: 0, MemType: MemoryTypeHostVisible, Mapped: true}
	if err := vl.ValidateMap(buf); err == nil {
		t.Error("ValidateMap on already mapped buffer should fail")
	}
}

func TestValidateMapDeviceLocalOnly(t *testing.T) {
	vl := NewValidationLayer()
	buf := &Buffer{BufferID: 0, MemType: MemoryTypeDeviceLocal}
	if err := vl.ValidateMap(buf); err == nil {
		t.Error("ValidateMap on DEVICE_LOCAL-only buffer should fail")
	}
}

func TestValidateBufferUsage(t *testing.T) {
	vl := NewValidationLayer()
	buf := &Buffer{BufferID: 0, Usage: BufferUsageStorage}
	if err := vl.ValidateBufferUsage(buf, BufferUsageStorage); err != nil {
		t.Errorf("should pass: %v", err)
	}
	if err := vl.ValidateBufferUsage(buf, BufferUsageTransferSrc); err == nil {
		t.Error("should fail for missing usage flag")
	}
}

func TestValidateBufferNotFreed(t *testing.T) {
	vl := NewValidationLayer()
	buf := &Buffer{BufferID: 0}
	if err := vl.ValidateBufferNotFreed(buf); err != nil {
		t.Errorf("should pass for non-freed buffer: %v", err)
	}
	buf.Freed = true
	if err := vl.ValidateBufferNotFreed(buf); err == nil {
		t.Error("should fail for freed buffer")
	}
}

// =========================================================================
// Barrier validation
// =========================================================================

func TestBarrierTracking(t *testing.T) {
	vl := NewValidationLayer()

	// Write without barrier should produce warning
	vl.RecordWrite(0)
	vl.ValidateReadAfterWrite(0)
	if len(vl.Warnings()) != 1 {
		t.Errorf("expected 1 warning, got %d", len(vl.Warnings()))
	}

	// After barrier, no warning
	vl.Clear()
	vl.RecordWrite(0)
	vl.RecordBarrier(nil) // global barrier
	vl.ValidateReadAfterWrite(0)
	if len(vl.Warnings()) != 0 {
		t.Errorf("expected 0 warnings after barrier, got %d", len(vl.Warnings()))
	}
}

func TestBarrierTrackingSpecificBuffers(t *testing.T) {
	vl := NewValidationLayer()

	vl.RecordWrite(0)
	vl.RecordWrite(1)
	vl.RecordBarrier(map[int]bool{0: true}) // only buffer 0

	vl.ValidateReadAfterWrite(0)
	if len(vl.Warnings()) != 0 {
		t.Errorf("buffer 0 should be protected, got %d warnings", len(vl.Warnings()))
	}

	vl.ValidateReadAfterWrite(1)
	if len(vl.Warnings()) != 1 {
		t.Errorf("buffer 1 should NOT be protected, got %d warnings", len(vl.Warnings()))
	}
}

func TestReadAfterWriteNoWrite(t *testing.T) {
	vl := NewValidationLayer()
	vl.ValidateReadAfterWrite(42)
	if len(vl.Warnings()) != 0 {
		t.Error("no warning expected when buffer was never written")
	}
}

// =========================================================================
// Descriptor set validation
// =========================================================================

func TestValidateDescriptorSetValid(t *testing.T) {
	vl := NewValidationLayer()

	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)
	buf := &Buffer{BufferID: 0, Size: 256}
	_ = ds.Write(0, buf)

	plLayout := NewPipelineLayout([]*DescriptorSetLayout{dsLayout}, 0)
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	pipeline := NewPipeline(shader, plLayout)

	if err := vl.ValidateDescriptorSet(ds, pipeline); err != nil {
		t.Errorf("ValidateDescriptorSet should pass: %v", err)
	}
}

func TestValidateDescriptorSetMissingBinding(t *testing.T) {
	vl := NewValidationLayer()

	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)
	// Don't write anything to binding 0

	plLayout := NewPipelineLayout([]*DescriptorSetLayout{dsLayout}, 0)
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	pipeline := NewPipeline(shader, plLayout)

	_ = vl.ValidateDescriptorSet(ds, pipeline)
	if len(vl.Warnings()) == 0 {
		t.Error("should warn about missing binding")
	}
}

func TestValidateDescriptorSetFreedBuffer(t *testing.T) {
	vl := NewValidationLayer()

	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)
	buf := &Buffer{BufferID: 0, Size: 256}
	_ = ds.Write(0, buf)
	buf.Freed = true // free after binding

	plLayout := NewPipelineLayout([]*DescriptorSetLayout{dsLayout}, 0)
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	pipeline := NewPipeline(shader, plLayout)

	if err := vl.ValidateDescriptorSet(ds, pipeline); err == nil {
		t.Error("should error on freed buffer in descriptor set")
	}
}

func TestValidateDescriptorSetNoLayouts(t *testing.T) {
	vl := NewValidationLayer()

	dsLayout := NewDescriptorSetLayout(nil)
	ds := NewDescriptorSet(dsLayout)

	plLayout := NewPipelineLayout(nil, 0) // no set layouts
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	pipeline := NewPipeline(shader, plLayout)

	if err := vl.ValidateDescriptorSet(ds, pipeline); err != nil {
		t.Errorf("should pass when no layouts: %v", err)
	}
}

// =========================================================================
// ValidationError tests
// =========================================================================

func TestValidationError(t *testing.T) {
	err := NewValidationError("test error")
	if err.Error() != "test error" {
		t.Errorf("Error() = %q, want %q", err.Error(), "test error")
	}
}
