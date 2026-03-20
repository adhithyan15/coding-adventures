package computeruntime

// Pipeline -- compiled kernels, descriptor sets, shader modules.
//
// # What is a Pipeline?
//
// A pipeline is a compiled kernel ready to execute. In Vulkan terms, it
// packages three things together:
//
//  1. ShaderModule -- the compiled program (instructions)
//  2. PipelineLayout -- what data the kernel expects (descriptor set layout)
//  3. Pipeline -- the combined, ready-to-dispatch object
//
// Think of it like a function call:
//   - ShaderModule = the function body (code)
//   - DescriptorSetLayout = the function signature (parameter types)
//   - DescriptorSet = the actual arguments (concrete buffers)
//   - Pipeline = the compiled function ready to call

import (
	"fmt"

	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// =========================================================================
// ShaderModule -- compiled program
// =========================================================================

// nextShaderModuleID is the global counter for shader module IDs.
var nextShaderModuleID int

// ShaderModule is a compiled program ready to be used in a pipeline.
//
// # GPU vs Dataflow
//
// For GPU-style devices (NVIDIA, AMD, Intel), the code is a list of
// instructions from our GenericISA (gpu-core package).
//
// For dataflow-style devices (TPU, ANE), the code is an operation
// descriptor -- just the operation name and parameters.
type ShaderModule struct {
	id         int
	code       []gpucore.Instruction
	operation  string
	entryPoint string
	localSize  [3]int
}

// ShaderModuleOptions holds optional parameters for NewShaderModule.
type ShaderModuleOptions struct {
	Code       []gpucore.Instruction
	Operation  string
	EntryPoint string
	LocalSize  [3]int
}

// NewShaderModule creates a new shader module.
func NewShaderModule(opts ShaderModuleOptions) *ShaderModule {
	id := nextShaderModuleID
	nextShaderModuleID++

	entryPoint := opts.EntryPoint
	if entryPoint == "" {
		entryPoint = "main"
	}

	localSize := opts.LocalSize
	if localSize == [3]int{0, 0, 0} {
		localSize = [3]int{32, 1, 1}
	}

	return &ShaderModule{
		id:         id,
		code:       opts.Code,
		operation:  opts.Operation,
		entryPoint: entryPoint,
		localSize:  localSize,
	}
}

// ModuleID returns the unique identifier.
func (s *ShaderModule) ModuleID() int { return s.id }

// Code returns the GPU-style instruction list. Nil for dataflow.
func (s *ShaderModule) Code() []gpucore.Instruction { return s.code }

// Operation returns the dataflow-style operation name. Empty for GPU.
func (s *ShaderModule) Operation() string { return s.operation }

// EntryPoint returns the entry point name (typically "main").
func (s *ShaderModule) EntryPoint() string { return s.entryPoint }

// LocalSize returns the workgroup dimensions declared in the shader.
func (s *ShaderModule) LocalSize() [3]int { return s.localSize }

// IsGPUStyle returns true if this is a GPU-style shader (has instruction code).
func (s *ShaderModule) IsGPUStyle() bool { return s.code != nil }

// IsDataflowStyle returns true if this is a dataflow-style shader (has operation name).
func (s *ShaderModule) IsDataflowStyle() bool { return s.operation != "" }

// =========================================================================
// DescriptorSetLayout -- describes the shape of data bindings
// =========================================================================

// nextDescriptorSetLayoutID is the global counter for descriptor set layout IDs.
var nextDescriptorSetLayoutID int

// DescriptorSetLayout describes what data a kernel expects.
//
// A layout is like a function signature -- it says "this kernel takes
// 3 storage buffers." It does not say WHICH buffers, just how many
// and what type.
type DescriptorSetLayout struct {
	id       int
	bindings []DescriptorBinding
}

// NewDescriptorSetLayout creates a new descriptor set layout.
func NewDescriptorSetLayout(bindings []DescriptorBinding) *DescriptorSetLayout {
	id := nextDescriptorSetLayoutID
	nextDescriptorSetLayoutID++
	bindingsCopy := make([]DescriptorBinding, len(bindings))
	copy(bindingsCopy, bindings)
	return &DescriptorSetLayout{
		id:       id,
		bindings: bindingsCopy,
	}
}

// LayoutID returns the unique identifier.
func (l *DescriptorSetLayout) LayoutID() int { return l.id }

// Bindings returns the binding slots in this layout.
func (l *DescriptorSetLayout) Bindings() []DescriptorBinding {
	result := make([]DescriptorBinding, len(l.bindings))
	copy(result, l.bindings)
	return result
}

// =========================================================================
// PipelineLayout -- shader + descriptor layout + push constants
// =========================================================================

// nextPipelineLayoutID is the global counter for pipeline layout IDs.
var nextPipelineLayoutID int

// PipelineLayout describes the complete interface of a pipeline.
//
// Combines:
//   - Descriptor set layouts (what buffers the kernel reads/writes)
//   - Push constant size (small inline data like alpha in SAXPY)
type PipelineLayout struct {
	id               int
	setLayouts       []*DescriptorSetLayout
	pushConstantSize int
}

// NewPipelineLayout creates a new pipeline layout.
func NewPipelineLayout(setLayouts []*DescriptorSetLayout, pushConstantSize int) *PipelineLayout {
	id := nextPipelineLayoutID
	nextPipelineLayoutID++
	layoutsCopy := make([]*DescriptorSetLayout, len(setLayouts))
	copy(layoutsCopy, setLayouts)
	return &PipelineLayout{
		id:               id,
		setLayouts:       layoutsCopy,
		pushConstantSize: pushConstantSize,
	}
}

// LayoutID returns the unique identifier.
func (l *PipelineLayout) LayoutID() int { return l.id }

// SetLayouts returns the descriptor set layouts used by this pipeline.
func (l *PipelineLayout) SetLayouts() []*DescriptorSetLayout { return l.setLayouts }

// PushConstantSize returns the maximum bytes for push constants.
func (l *PipelineLayout) PushConstantSize() int { return l.pushConstantSize }

// =========================================================================
// Pipeline -- compiled, ready to dispatch
// =========================================================================

// nextPipelineID is the global counter for pipeline IDs.
var nextPipelineID int

// Pipeline is a compiled kernel bound to a pipeline layout.
//
// Once created, bind it in a command buffer:
//
//	cb.CmdBindPipeline(pipeline)
//	cb.CmdDispatch(gridX, gridY, gridZ)
type Pipeline struct {
	id     int
	shader *ShaderModule
	layout *PipelineLayout
}

// NewPipeline creates a new pipeline.
func NewPipeline(shader *ShaderModule, layout *PipelineLayout) *Pipeline {
	id := nextPipelineID
	nextPipelineID++
	return &Pipeline{
		id:     id,
		shader: shader,
		layout: layout,
	}
}

// PipelineID returns the unique identifier.
func (p *Pipeline) PipelineID() int { return p.id }

// Shader returns the compiled shader module.
func (p *Pipeline) Shader() *ShaderModule { return p.shader }

// Layout returns the pipeline layout.
func (p *Pipeline) Layout() *PipelineLayout { return p.layout }

// WorkgroupSize returns the local workgroup dimensions from the shader.
func (p *Pipeline) WorkgroupSize() [3]int { return p.shader.LocalSize() }

// =========================================================================
// DescriptorSet -- concrete buffer bindings
// =========================================================================

// nextDescriptorSetID is the global counter for descriptor set IDs.
var nextDescriptorSetID int

// DescriptorSet holds concrete buffer assignments for a descriptor set layout.
//
// # Layout vs Set
//
// Layout says: "binding 0 is a storage buffer"
// Set says:    "binding 0 is buf_x (address 0x1000, 4096 bytes)"
//
// You create a set from a layout, then Write() buffers into it.
type DescriptorSet struct {
	id       int
	layout   *DescriptorSetLayout
	bindings map[int]*Buffer
}

// NewDescriptorSet creates a new descriptor set from a layout.
func NewDescriptorSet(layout *DescriptorSetLayout) *DescriptorSet {
	id := nextDescriptorSetID
	nextDescriptorSetID++
	return &DescriptorSet{
		id:       id,
		layout:   layout,
		bindings: make(map[int]*Buffer),
	}
}

// SetID returns the unique identifier.
func (ds *DescriptorSet) SetID() int { return ds.id }

// Layout returns the layout this set was created from.
func (ds *DescriptorSet) Layout() *DescriptorSetLayout { return ds.layout }

// Bindings returns a copy of the current buffer bindings.
func (ds *DescriptorSet) Bindings() map[int]*Buffer {
	result := make(map[int]*Buffer, len(ds.bindings))
	for k, v := range ds.bindings {
		result[k] = v
	}
	return result
}

// Write binds a buffer to a slot.
//
// Returns an error if binding does not exist in layout or buffer is freed.
func (ds *DescriptorSet) Write(binding int, buffer *Buffer) error {
	validBindings := make(map[int]bool)
	for _, b := range ds.layout.bindings {
		validBindings[b.Binding] = true
	}
	if !validBindings[binding] {
		return fmt.Errorf("binding %d not in layout", binding)
	}
	if buffer.Freed {
		return fmt.Errorf("cannot bind freed buffer %d to binding %d", buffer.BufferID, binding)
	}
	ds.bindings[binding] = buffer
	return nil
}

// GetBuffer returns the buffer at a binding slot, or nil if not bound.
func (ds *DescriptorSet) GetBuffer(binding int) *Buffer {
	return ds.bindings[binding]
}
