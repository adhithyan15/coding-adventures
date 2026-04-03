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
	result, _ := StartNew[*ShaderModule]("compute-runtime.NewShaderModule", nil,
		func(op *Operation[*ShaderModule], rf *ResultFactory[*ShaderModule]) *OperationResult[*ShaderModule] {
			op.AddProperty("operation", opts.Operation)
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

			return rf.Generate(true, false, &ShaderModule{
				id:         id,
				code:       opts.Code,
				operation:  opts.Operation,
				entryPoint: entryPoint,
				localSize:  localSize,
			})
		}).GetResult()
	return result
}

// ModuleID returns the unique identifier.
func (s *ShaderModule) ModuleID() int {
	result, _ := StartNew[int]("compute-runtime.ShaderModule.ModuleID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, s.id)
		}).GetResult()
	return result
}

// Code returns the GPU-style instruction list. Nil for dataflow.
func (s *ShaderModule) Code() []gpucore.Instruction {
	result, _ := StartNew[[]gpucore.Instruction]("compute-runtime.ShaderModule.Code", nil,
		func(op *Operation[[]gpucore.Instruction], rf *ResultFactory[[]gpucore.Instruction]) *OperationResult[[]gpucore.Instruction] {
			return rf.Generate(true, false, s.code)
		}).GetResult()
	return result
}

// Operation returns the dataflow-style operation name. Empty for GPU.
func (s *ShaderModule) Operation() string {
	result, _ := StartNew[string]("compute-runtime.ShaderModule.Operation", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, s.operation)
		}).GetResult()
	return result
}

// EntryPoint returns the entry point name (typically "main").
func (s *ShaderModule) EntryPoint() string {
	result, _ := StartNew[string]("compute-runtime.ShaderModule.EntryPoint", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, s.entryPoint)
		}).GetResult()
	return result
}

// LocalSize returns the workgroup dimensions declared in the shader.
func (s *ShaderModule) LocalSize() [3]int {
	result, _ := StartNew[[3]int]("compute-runtime.ShaderModule.LocalSize", [3]int{},
		func(op *Operation[[3]int], rf *ResultFactory[[3]int]) *OperationResult[[3]int] {
			return rf.Generate(true, false, s.localSize)
		}).GetResult()
	return result
}

// IsGPUStyle returns true if this is a GPU-style shader (has instruction code).
func (s *ShaderModule) IsGPUStyle() bool {
	result, _ := StartNew[bool]("compute-runtime.ShaderModule.IsGPUStyle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, s.code != nil)
		}).GetResult()
	return result
}

// IsDataflowStyle returns true if this is a dataflow-style shader (has operation name).
func (s *ShaderModule) IsDataflowStyle() bool {
	result, _ := StartNew[bool]("compute-runtime.ShaderModule.IsDataflowStyle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, s.operation != "")
		}).GetResult()
	return result
}

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
	result, _ := StartNew[*DescriptorSetLayout]("compute-runtime.NewDescriptorSetLayout", nil,
		func(op *Operation[*DescriptorSetLayout], rf *ResultFactory[*DescriptorSetLayout]) *OperationResult[*DescriptorSetLayout] {
			op.AddProperty("num_bindings", len(bindings))
			id := nextDescriptorSetLayoutID
			nextDescriptorSetLayoutID++
			bindingsCopy := make([]DescriptorBinding, len(bindings))
			copy(bindingsCopy, bindings)
			return rf.Generate(true, false, &DescriptorSetLayout{
				id:       id,
				bindings: bindingsCopy,
			})
		}).GetResult()
	return result
}

// LayoutID returns the unique identifier.
func (l *DescriptorSetLayout) LayoutID() int {
	result, _ := StartNew[int]("compute-runtime.DescriptorSetLayout.LayoutID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, l.id)
		}).GetResult()
	return result
}

// Bindings returns the binding slots in this layout.
func (l *DescriptorSetLayout) Bindings() []DescriptorBinding {
	result, _ := StartNew[[]DescriptorBinding]("compute-runtime.DescriptorSetLayout.Bindings", nil,
		func(op *Operation[[]DescriptorBinding], rf *ResultFactory[[]DescriptorBinding]) *OperationResult[[]DescriptorBinding] {
			res := make([]DescriptorBinding, len(l.bindings))
			copy(res, l.bindings)
			return rf.Generate(true, false, res)
		}).GetResult()
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
	result, _ := StartNew[*PipelineLayout]("compute-runtime.NewPipelineLayout", nil,
		func(op *Operation[*PipelineLayout], rf *ResultFactory[*PipelineLayout]) *OperationResult[*PipelineLayout] {
			op.AddProperty("push_constant_size", pushConstantSize)
			id := nextPipelineLayoutID
			nextPipelineLayoutID++
			layoutsCopy := make([]*DescriptorSetLayout, len(setLayouts))
			copy(layoutsCopy, setLayouts)
			return rf.Generate(true, false, &PipelineLayout{
				id:               id,
				setLayouts:       layoutsCopy,
				pushConstantSize: pushConstantSize,
			})
		}).GetResult()
	return result
}

// LayoutID returns the unique identifier.
func (l *PipelineLayout) LayoutID() int {
	result, _ := StartNew[int]("compute-runtime.PipelineLayout.LayoutID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, l.id)
		}).GetResult()
	return result
}

// SetLayouts returns the descriptor set layouts used by this pipeline.
func (l *PipelineLayout) SetLayouts() []*DescriptorSetLayout {
	result, _ := StartNew[[]*DescriptorSetLayout]("compute-runtime.PipelineLayout.SetLayouts", nil,
		func(op *Operation[[]*DescriptorSetLayout], rf *ResultFactory[[]*DescriptorSetLayout]) *OperationResult[[]*DescriptorSetLayout] {
			return rf.Generate(true, false, l.setLayouts)
		}).GetResult()
	return result
}

// PushConstantSize returns the maximum bytes for push constants.
func (l *PipelineLayout) PushConstantSize() int {
	result, _ := StartNew[int]("compute-runtime.PipelineLayout.PushConstantSize", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, l.pushConstantSize)
		}).GetResult()
	return result
}

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
	result, _ := StartNew[*Pipeline]("compute-runtime.NewPipeline", nil,
		func(op *Operation[*Pipeline], rf *ResultFactory[*Pipeline]) *OperationResult[*Pipeline] {
			id := nextPipelineID
			nextPipelineID++
			return rf.Generate(true, false, &Pipeline{
				id:     id,
				shader: shader,
				layout: layout,
			})
		}).GetResult()
	return result
}

// PipelineID returns the unique identifier.
func (p *Pipeline) PipelineID() int {
	result, _ := StartNew[int]("compute-runtime.Pipeline.PipelineID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, p.id)
		}).GetResult()
	return result
}

// Shader returns the compiled shader module.
func (p *Pipeline) Shader() *ShaderModule {
	result, _ := StartNew[*ShaderModule]("compute-runtime.Pipeline.Shader", nil,
		func(op *Operation[*ShaderModule], rf *ResultFactory[*ShaderModule]) *OperationResult[*ShaderModule] {
			return rf.Generate(true, false, p.shader)
		}).GetResult()
	return result
}

// Layout returns the pipeline layout.
func (p *Pipeline) Layout() *PipelineLayout {
	result, _ := StartNew[*PipelineLayout]("compute-runtime.Pipeline.Layout", nil,
		func(op *Operation[*PipelineLayout], rf *ResultFactory[*PipelineLayout]) *OperationResult[*PipelineLayout] {
			return rf.Generate(true, false, p.layout)
		}).GetResult()
	return result
}

// WorkgroupSize returns the local workgroup dimensions from the shader.
func (p *Pipeline) WorkgroupSize() [3]int {
	result, _ := StartNew[[3]int]("compute-runtime.Pipeline.WorkgroupSize", [3]int{},
		func(op *Operation[[3]int], rf *ResultFactory[[3]int]) *OperationResult[[3]int] {
			return rf.Generate(true, false, p.shader.LocalSize())
		}).GetResult()
	return result
}

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
	result, _ := StartNew[*DescriptorSet]("compute-runtime.NewDescriptorSet", nil,
		func(op *Operation[*DescriptorSet], rf *ResultFactory[*DescriptorSet]) *OperationResult[*DescriptorSet] {
			id := nextDescriptorSetID
			nextDescriptorSetID++
			return rf.Generate(true, false, &DescriptorSet{
				id:       id,
				layout:   layout,
				bindings: make(map[int]*Buffer),
			})
		}).GetResult()
	return result
}

// SetID returns the unique identifier.
func (ds *DescriptorSet) SetID() int {
	result, _ := StartNew[int]("compute-runtime.DescriptorSet.SetID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, ds.id)
		}).GetResult()
	return result
}

// Layout returns the layout this set was created from.
func (ds *DescriptorSet) Layout() *DescriptorSetLayout {
	result, _ := StartNew[*DescriptorSetLayout]("compute-runtime.DescriptorSet.Layout", nil,
		func(op *Operation[*DescriptorSetLayout], rf *ResultFactory[*DescriptorSetLayout]) *OperationResult[*DescriptorSetLayout] {
			return rf.Generate(true, false, ds.layout)
		}).GetResult()
	return result
}

// Bindings returns a copy of the current buffer bindings.
func (ds *DescriptorSet) Bindings() map[int]*Buffer {
	result, _ := StartNew[map[int]*Buffer]("compute-runtime.DescriptorSet.Bindings", nil,
		func(op *Operation[map[int]*Buffer], rf *ResultFactory[map[int]*Buffer]) *OperationResult[map[int]*Buffer] {
			res := make(map[int]*Buffer, len(ds.bindings))
			for k, v := range ds.bindings {
				res[k] = v
			}
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Write binds a buffer to a slot.
//
// Returns an error if binding does not exist in layout or buffer is freed.
func (ds *DescriptorSet) Write(binding int, buffer *Buffer) error {
	_, err := StartNew[struct{}]("compute-runtime.DescriptorSet.Write", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("binding", binding)
			op.AddProperty("buffer_id", buffer.BufferID)
			validBindings := make(map[int]bool)
			for _, b := range ds.layout.bindings {
				validBindings[b.Binding] = true
			}
			if !validBindings[binding] {
				return rf.Fail(struct{}{}, fmt.Errorf("binding %d not in layout", binding))
			}
			if buffer.Freed {
				return rf.Fail(struct{}{}, fmt.Errorf("cannot bind freed buffer %d to binding %d", buffer.BufferID, binding))
			}
			ds.bindings[binding] = buffer
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// GetBuffer returns the buffer at a binding slot, or nil if not bound.
func (ds *DescriptorSet) GetBuffer(binding int) *Buffer {
	result, _ := StartNew[*Buffer]("compute-runtime.DescriptorSet.GetBuffer", nil,
		func(op *Operation[*Buffer], rf *ResultFactory[*Buffer]) *OperationResult[*Buffer] {
			op.AddProperty("binding", binding)
			return rf.Generate(true, false, ds.bindings[binding])
		}).GetResult()
	return result
}
