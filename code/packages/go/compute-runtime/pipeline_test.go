package computeruntime

import (
	"testing"

	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// ShaderModule tests
// =========================================================================

func TestShaderModuleGPUStyle(t *testing.T) {
	code := []gpucore.Instruction{gpucore.Halt()}
	shader := NewShaderModule(ShaderModuleOptions{
		Code:      code,
		LocalSize: [3]int{256, 1, 1},
	})

	if !shader.IsGPUStyle() {
		t.Error("shader with code should be GPU style")
	}
	if shader.IsDataflowStyle() {
		t.Error("shader with code should not be dataflow style")
	}
	if shader.EntryPoint() != "main" {
		t.Errorf("EntryPoint = %q, want %q", shader.EntryPoint(), "main")
	}
	if shader.LocalSize() != [3]int{256, 1, 1} {
		t.Errorf("LocalSize = %v, want %v", shader.LocalSize(), [3]int{256, 1, 1})
	}
	if shader.Code() == nil {
		t.Error("Code should not be nil")
	}
}

func TestShaderModuleDataflowStyle(t *testing.T) {
	shader := NewShaderModule(ShaderModuleOptions{
		Operation: "matmul",
	})

	if shader.IsGPUStyle() {
		t.Error("shader with only operation should not be GPU style")
	}
	if !shader.IsDataflowStyle() {
		t.Error("shader with operation should be dataflow style")
	}
	if shader.Operation() != "matmul" {
		t.Errorf("Operation = %q, want %q", shader.Operation(), "matmul")
	}
}

func TestShaderModuleDefaults(t *testing.T) {
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	if shader.EntryPoint() != "main" {
		t.Errorf("default EntryPoint = %q, want %q", shader.EntryPoint(), "main")
	}
	if shader.LocalSize() != [3]int{32, 1, 1} {
		t.Errorf("default LocalSize = %v, want %v", shader.LocalSize(), [3]int{32, 1, 1})
	}
}

func TestShaderModuleUniqueIDs(t *testing.T) {
	s1 := NewShaderModule(ShaderModuleOptions{Operation: "a"})
	s2 := NewShaderModule(ShaderModuleOptions{Operation: "b"})
	if s1.ModuleID() == s2.ModuleID() {
		t.Errorf("shader modules should have unique IDs: %d == %d", s1.ModuleID(), s2.ModuleID())
	}
}

func TestShaderModuleCustomEntryPoint(t *testing.T) {
	shader := NewShaderModule(ShaderModuleOptions{
		Operation:  "test",
		EntryPoint: "compute_main",
	})
	if shader.EntryPoint() != "compute_main" {
		t.Errorf("EntryPoint = %q, want %q", shader.EntryPoint(), "compute_main")
	}
}

// =========================================================================
// DescriptorSetLayout tests
// =========================================================================

func TestDescriptorSetLayout(t *testing.T) {
	bindings := []DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
		{Binding: 1, Type: "uniform", Count: 1},
	}
	layout := NewDescriptorSetLayout(bindings)
	if len(layout.Bindings()) != 2 {
		t.Errorf("Bindings count = %d, want 2", len(layout.Bindings()))
	}
	if layout.Bindings()[0].Type != "storage" {
		t.Errorf("Bindings[0].Type = %q, want %q", layout.Bindings()[0].Type, "storage")
	}
}

func TestDescriptorSetLayoutUniqueIDs(t *testing.T) {
	l1 := NewDescriptorSetLayout(nil)
	l2 := NewDescriptorSetLayout(nil)
	if l1.LayoutID() == l2.LayoutID() {
		t.Errorf("layouts should have unique IDs")
	}
}

// =========================================================================
// PipelineLayout tests
// =========================================================================

func TestPipelineLayout(t *testing.T) {
	dsLayout := NewDescriptorSetLayout(nil)
	pl := NewPipelineLayout([]*DescriptorSetLayout{dsLayout}, 64)
	if len(pl.SetLayouts()) != 1 {
		t.Errorf("SetLayouts count = %d, want 1", len(pl.SetLayouts()))
	}
	if pl.PushConstantSize() != 64 {
		t.Errorf("PushConstantSize = %d, want 64", pl.PushConstantSize())
	}
}

func TestPipelineLayoutUniqueIDs(t *testing.T) {
	pl1 := NewPipelineLayout(nil, 0)
	pl2 := NewPipelineLayout(nil, 0)
	if pl1.LayoutID() == pl2.LayoutID() {
		t.Errorf("pipeline layouts should have unique IDs")
	}
}

// =========================================================================
// Pipeline tests
// =========================================================================

func TestPipeline(t *testing.T) {
	shader := NewShaderModule(ShaderModuleOptions{
		Operation: "test",
		LocalSize: [3]int{64, 1, 1},
	})
	dsLayout := NewDescriptorSetLayout(nil)
	plLayout := NewPipelineLayout([]*DescriptorSetLayout{dsLayout}, 0)
	pipeline := NewPipeline(shader, plLayout)

	if pipeline.Shader() != shader {
		t.Error("Shader() should return original shader")
	}
	if pipeline.Layout() != plLayout {
		t.Error("Layout() should return original layout")
	}
	if pipeline.WorkgroupSize() != [3]int{64, 1, 1} {
		t.Errorf("WorkgroupSize = %v, want %v", pipeline.WorkgroupSize(), [3]int{64, 1, 1})
	}
}

func TestPipelineUniqueIDs(t *testing.T) {
	shader := NewShaderModule(ShaderModuleOptions{Operation: "test"})
	layout := NewPipelineLayout(nil, 0)
	p1 := NewPipeline(shader, layout)
	p2 := NewPipeline(shader, layout)
	if p1.PipelineID() == p2.PipelineID() {
		t.Errorf("pipelines should have unique IDs")
	}
}

// =========================================================================
// DescriptorSet tests
// =========================================================================

func TestDescriptorSet(t *testing.T) {
	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
		{Binding: 1, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)

	if ds.Layout() != dsLayout {
		t.Error("Layout() should return original layout")
	}
	if len(ds.Bindings()) != 0 {
		t.Errorf("new descriptor set should have 0 bindings, got %d", len(ds.Bindings()))
	}
}

func TestDescriptorSetWrite(t *testing.T) {
	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)
	buf := &Buffer{BufferID: 42, Size: 256}

	if err := ds.Write(0, buf); err != nil {
		t.Fatalf("Write failed: %v", err)
	}
	if ds.GetBuffer(0) != buf {
		t.Error("GetBuffer(0) should return the written buffer")
	}
}

func TestDescriptorSetWriteInvalidBinding(t *testing.T) {
	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)
	buf := &Buffer{BufferID: 42, Size: 256}

	if err := ds.Write(5, buf); err == nil {
		t.Error("Write to invalid binding should return error")
	}
}

func TestDescriptorSetWriteFreedBuffer(t *testing.T) {
	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)
	buf := &Buffer{BufferID: 42, Size: 256, Freed: true}

	if err := ds.Write(0, buf); err == nil {
		t.Error("Write with freed buffer should return error")
	}
}

func TestDescriptorSetGetBufferNil(t *testing.T) {
	dsLayout := NewDescriptorSetLayout([]DescriptorBinding{
		{Binding: 0, Type: "storage", Count: 1},
	})
	ds := NewDescriptorSet(dsLayout)
	if ds.GetBuffer(0) != nil {
		t.Error("GetBuffer on unbound slot should return nil")
	}
}

func TestDescriptorSetUniqueIDs(t *testing.T) {
	dsLayout := NewDescriptorSetLayout(nil)
	ds1 := NewDescriptorSet(dsLayout)
	ds2 := NewDescriptorSet(dsLayout)
	if ds1.SetID() == ds2.SetID() {
		t.Errorf("descriptor sets should have unique IDs")
	}
}
