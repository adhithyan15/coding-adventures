package vendorapisimulators

// OpenGL Compute Simulator -- the legacy global state machine.
//
// # What is OpenGL?
//
// OpenGL is the oldest surviving GPU API (1992). Compute shaders were bolted
// on in OpenGL 4.3 (2012), long after the core API was designed around
// graphics rendering. This heritage shows: OpenGL uses a global state machine
// model where you bind things to "current" state and then issue commands that
// operate on whatever is currently bound.
//
// # The State Machine Model
//
// Unlike Vulkan (explicit objects) or Metal (scoped encoders), OpenGL
// maintains global state:
//
//	glUseProgram(prog)           // Sets "current program" globally
//	glBindBufferBase(0, buf_a)   // Sets "buffer at binding 0" globally
//	glDispatchCompute(4, 1, 1)   // Uses WHATEVER is currently bound
//
// This is simple for small programs but error-prone for large ones -- you
// must always remember what is bound.
//
// # Integer Handles
//
// OpenGL uses integer handles (GLuint) for everything. You never get a
// typed object -- just a number:
//
//	shader := gl.CreateShader(GL_COMPUTE_SHADER)    // Returns 1
//	program := gl.CreateProgram()                   // Returns 2
//	buffers := gl.GenBuffers(2)                     // Returns [3, 4]
//
// These integers are essentially IDs in internal lookup tables.

import (
	"fmt"

	cr "github.com/adhithyan15/coding-adventures/code/packages/go/compute-runtime"
	gpucore "github.com/adhithyan15/coding-adventures/code/packages/go/gpu-core"
)

// Suppress unused import for gpucore.
var _ = gpucore.Instruction{}

// =========================================================================
// OpenGL constants -- module-level, just like real OpenGL
// =========================================================================

const (
	// Shader types
	GL_COMPUTE_SHADER = 0x91B9

	// Buffer targets
	GL_SHADER_STORAGE_BUFFER = 0x90D2
	GL_ARRAY_BUFFER          = 0x8892
	GL_UNIFORM_BUFFER        = 0x8A11

	// Buffer usage hints
	GL_STATIC_DRAW  = 0x88E4
	GL_DYNAMIC_DRAW = 0x88E8
	GL_STREAM_DRAW  = 0x88E0

	// Map access bits
	GL_MAP_READ_BIT  = 0x0001
	GL_MAP_WRITE_BIT = 0x0002

	// Memory barrier bits
	GL_SHADER_STORAGE_BARRIER_BIT = 0x00002000
	GL_BUFFER_UPDATE_BARRIER_BIT  = 0x00000200
	GL_ALL_BARRIER_BITS           = 0xFFFFFFFF

	// Sync object results
	GL_ALREADY_SIGNALED    = 0x911A
	GL_CONDITION_SATISFIED = 0x911C
	GL_TIMEOUT_EXPIRED     = 0x911B
	GL_WAIT_FAILED         = 0x911D

	// Sync flags
	GL_SYNC_FLUSH_COMMANDS_BIT    = 0x00000001
	GL_SYNC_GPU_COMMANDS_COMPLETE = 0x9117
)

// =========================================================================
// GLContext -- the main OpenGL state machine
// =========================================================================

// GLContext is an OpenGL context -- a global state machine for GPU programming.
//
// # The State Machine
//
// GLContext maintains global state that commands operate on:
//
//   - currentProgram:  Which program is currently active (UseProgram)
//   - boundBuffers:    Which buffers are bound to which targets/indices
//   - programs:        Map of GL handle -> Layer 5 Pipeline
//   - shaders:         Map of GL handle -> shader source + code
//   - buffers:         Map of GL handle -> Layer 5 Buffer
//   - nextID:          Counter for generating unique GL handles
type GLContext struct {
	*BaseVendorSimulator

	// Global state
	currentProgram int // 0 means none
	boundBuffers   map[[2]int]int // (target, index) -> GL handle
	targetBuffers  map[int]int    // target -> GL handle (for non-indexed ops)

	// Internal lookup tables
	shaders  map[int]map[string]interface{}
	programs map[int]map[string]interface{}
	buffers  map[int]*cr.Buffer
	syncs    map[int]*cr.Fence
	uniforms map[[2]interface{}]interface{}

	// GL handle counter
	nextID int
}

// NewGLContext creates a new OpenGL context.
func NewGLContext() (*GLContext, error) {
	base, err := InitBase(nil, "")
	if err != nil {
		return nil, fmt.Errorf("failed to initialize OpenGL context: %w", err)
	}
	return &GLContext{
		BaseVendorSimulator: base,
		boundBuffers:        make(map[[2]int]int),
		targetBuffers:       make(map[int]int),
		shaders:             make(map[int]map[string]interface{}),
		programs:            make(map[int]map[string]interface{}),
		buffers:             make(map[int]*cr.Buffer),
		syncs:               make(map[int]*cr.Fence),
		uniforms:            make(map[[2]interface{}]interface{}),
		nextID:              1,
	}, nil
}

func (gl *GLContext) genID() int {
	handle := gl.nextID
	gl.nextID++
	return handle
}

// =================================================================
// Shader management
// =================================================================

// CreateShader creates a shader object (glCreateShader).
func (gl *GLContext) CreateShader(shaderType int) (int, error) {
	if shaderType != GL_COMPUTE_SHADER {
		return 0, fmt.Errorf(
			"only GL_COMPUTE_SHADER (0x%04X) is supported, got 0x%04X",
			GL_COMPUTE_SHADER, shaderType,
		)
	}
	handle := gl.genID()
	gl.shaders[handle] = map[string]interface{}{
		"source":   "",
		"code":     nil,
		"compiled": false,
		"type":     shaderType,
	}
	return handle, nil
}

// ShaderSource sets the source code for a shader (glShaderSource).
func (gl *GLContext) ShaderSource(shader int, source string) error {
	if _, ok := gl.shaders[shader]; !ok {
		return fmt.Errorf("invalid shader handle %d", shader)
	}
	gl.shaders[shader]["source"] = source
	return nil
}

// CompileShader compiles a shader (glCompileShader).
func (gl *GLContext) CompileShader(shader int) error {
	if _, ok := gl.shaders[shader]; !ok {
		return fmt.Errorf("invalid shader handle %d", shader)
	}
	gl.shaders[shader]["compiled"] = true
	return nil
}

// DeleteShader deletes a shader object (glDeleteShader).
func (gl *GLContext) DeleteShader(shader int) {
	delete(gl.shaders, shader)
}

// =================================================================
// Program management
// =================================================================

// CreateProgram creates a program object (glCreateProgram).
func (gl *GLContext) CreateProgram() int {
	handle := gl.genID()
	gl.programs[handle] = map[string]interface{}{
		"pipeline":      nil,
		"shaders":       []int{},
		"linked":        false,
		"shader_module": nil,
	}
	return handle
}

// AttachShader attaches a shader to a program (glAttachShader).
func (gl *GLContext) AttachShader(program, shader int) error {
	if _, ok := gl.programs[program]; !ok {
		return fmt.Errorf("invalid program handle %d", program)
	}
	if _, ok := gl.shaders[shader]; !ok {
		return fmt.Errorf("invalid shader handle %d", shader)
	}
	shaders := gl.programs[program]["shaders"].([]int)
	gl.programs[program]["shaders"] = append(shaders, shader)
	return nil
}

// LinkProgram links a program (glLinkProgram).
func (gl *GLContext) LinkProgram(program int) error {
	prog, ok := gl.programs[program]
	if !ok {
		return fmt.Errorf("invalid program handle %d", program)
	}
	shaders := prog["shaders"].([]int)
	if len(shaders) == 0 {
		return fmt.Errorf("program %d has no attached shaders", program)
	}

	// Get shader code from the first compute shader
	shaderHandle := shaders[0]
	shaderInfo := gl.shaders[shaderHandle]
	var code []gpucore.Instruction
	if c, ok := shaderInfo["code"].([]gpucore.Instruction); ok {
		code = c
	}

	// Create Layer 5 pipeline
	device := gl.LogicalDevice
	shader := device.CreateShaderModule(cr.ShaderModuleOptions{Code: code})
	dsLayout := device.CreateDescriptorSetLayout(nil)
	plLayout := device.CreatePipelineLayout([]*cr.DescriptorSetLayout{dsLayout}, 0)
	pipeline := device.CreateComputePipeline(shader, plLayout)

	prog["pipeline"] = pipeline
	prog["shader_module"] = shader
	prog["linked"] = true
	return nil
}

// UseProgram sets the active program (glUseProgram).
func (gl *GLContext) UseProgram(program int) error {
	if program == 0 {
		gl.currentProgram = 0
		return nil
	}
	prog, ok := gl.programs[program]
	if !ok {
		return fmt.Errorf("invalid program handle %d", program)
	}
	if !prog["linked"].(bool) {
		return fmt.Errorf("program %d is not linked", program)
	}
	gl.currentProgram = program
	return nil
}

// DeleteProgram deletes a program object (glDeleteProgram).
func (gl *GLContext) DeleteProgram(program int) {
	if gl.currentProgram == program {
		gl.currentProgram = 0
	}
	delete(gl.programs, program)
}

// =================================================================
// Buffer management
// =================================================================

// GenBuffers generates buffer objects (glGenBuffers).
func (gl *GLContext) GenBuffers(count int) []int {
	handles := make([]int, count)
	for i := 0; i < count; i++ {
		handle := gl.genID()
		gl.buffers[handle] = nil
		handles[i] = handle
	}
	return handles
}

// DeleteBuffers deletes buffer objects (glDeleteBuffers).
func (gl *GLContext) DeleteBuffers(bufferHandles []int) {
	for _, handle := range bufferHandles {
		if buf, ok := gl.buffers[handle]; ok && buf != nil {
			if !buf.Freed {
				gl.MemoryManager.Free(buf)
			}
		}
		delete(gl.buffers, handle)
		// Remove from any bindings
		for k, v := range gl.boundBuffers {
			if v == handle {
				delete(gl.boundBuffers, k)
			}
		}
		for k, v := range gl.targetBuffers {
			if v == handle {
				delete(gl.targetBuffers, k)
			}
		}
	}
}

// BindBuffer binds a buffer to a target (glBindBuffer).
func (gl *GLContext) BindBuffer(target, buffer int) error {
	if buffer == 0 {
		delete(gl.targetBuffers, target)
		return nil
	}
	if _, ok := gl.buffers[buffer]; !ok {
		return fmt.Errorf("invalid buffer handle %d", buffer)
	}
	gl.targetBuffers[target] = buffer
	return nil
}

// BufferData allocates and optionally fills a buffer (glBufferData).
func (gl *GLContext) BufferData(target, size int, data []byte, usage int) error {
	handle, ok := gl.targetBuffers[target]
	if !ok {
		return fmt.Errorf("no buffer bound to target 0x%04X", target)
	}

	// Free old allocation if exists
	if gl.buffers[handle] != nil {
		oldBuf := gl.buffers[handle]
		if !oldBuf.Freed {
			gl.MemoryManager.Free(oldBuf)
		}
	}

	// Allocate new buffer via Layer 5
	buf, err := gl.MemoryManager.Allocate(size, DefaultMemType(), DefaultUsage())
	if err != nil {
		return err
	}
	gl.buffers[handle] = buf

	// Upload initial data if provided
	if data != nil {
		mapped, err := gl.MemoryManager.Map(buf)
		if err != nil {
			return err
		}
		copyLen := size
		if copyLen > len(data) {
			copyLen = len(data)
		}
		if err := mapped.Write(0, data[:copyLen]); err != nil {
			return err
		}
		if err := gl.MemoryManager.Unmap(buf); err != nil {
			return err
		}
	}
	return nil
}

// BufferSubData updates a portion of a buffer (glBufferSubData).
func (gl *GLContext) BufferSubData(target, offset int, data []byte) error {
	handle, ok := gl.targetBuffers[target]
	if !ok {
		return fmt.Errorf("no buffer bound to target 0x%04X", target)
	}
	buf := gl.buffers[handle]
	if buf == nil {
		return fmt.Errorf("buffer %d has no data store", handle)
	}
	mapped, err := gl.MemoryManager.Map(buf)
	if err != nil {
		return err
	}
	if err := mapped.Write(offset, data); err != nil {
		return err
	}
	return gl.MemoryManager.Unmap(buf)
}

// BindBufferBase binds a buffer to an indexed binding point (glBindBufferBase).
func (gl *GLContext) BindBufferBase(target, index, buffer int) error {
	if _, ok := gl.buffers[buffer]; !ok {
		return fmt.Errorf("invalid buffer handle %d", buffer)
	}
	gl.boundBuffers[[2]int{target, index}] = buffer
	return nil
}

// MapBufferRange maps a buffer region for CPU access (glMapBufferRange).
func (gl *GLContext) MapBufferRange(target, offset, length, access int) ([]byte, error) {
	handle, ok := gl.targetBuffers[target]
	if !ok {
		return nil, fmt.Errorf("no buffer bound to target 0x%04X", target)
	}
	buf := gl.buffers[handle]
	if buf == nil {
		return nil, fmt.Errorf("buffer %d has no data store", handle)
	}

	if err := gl.MemoryManager.Invalidate(buf, 0, 0); err != nil {
		return nil, err
	}
	data := gl.MemoryManager.GetBufferData(buf.BufferID)
	end := offset + length
	if end > len(data) {
		end = len(data)
	}
	result := make([]byte, end-offset)
	copy(result, data[offset:end])
	return result, nil
}

// UnmapBuffer unmaps a buffer (glUnmapBuffer).
func (gl *GLContext) UnmapBuffer(target int) bool {
	return true
}

// =================================================================
// Compute dispatch
// =================================================================

// DispatchCompute dispatches compute work groups (glDispatchCompute).
//
// Uses whatever program and SSBO bindings are currently active.
func (gl *GLContext) DispatchCompute(numGroupsX, numGroupsY, numGroupsZ int) error {
	if gl.currentProgram == 0 {
		return fmt.Errorf("no program is currently active (call UseProgram first)")
	}

	prog := gl.programs[gl.currentProgram]
	device := gl.LogicalDevice

	// Get shader code from the program
	var shaderCode []gpucore.Instruction
	shaders := prog["shaders"].([]int)
	if len(shaders) > 0 {
		shaderHandle := shaders[0]
		if info, ok := gl.shaders[shaderHandle]; ok {
			if c, ok := info["code"].([]gpucore.Instruction); ok {
				shaderCode = c
			}
		}
	}

	// Find all SSBO bindings
	ssboBindings := map[int]*cr.Buffer{}
	for key, handle := range gl.boundBuffers {
		target := key[0]
		index := key[1]
		if target == GL_SHADER_STORAGE_BUFFER {
			if buf, ok := gl.buffers[handle]; ok && buf != nil {
				ssboBindings[index] = buf
			}
		}
	}

	// Create shader module
	shader := device.CreateShaderModule(cr.ShaderModuleOptions{Code: shaderCode})

	// Create descriptor set with SSBO bindings
	sortedSSBOIndices := sortedIntKeys(ssboBindings)
	bindings := make([]cr.DescriptorBinding, len(sortedSSBOIndices))
	for j, i := range sortedSSBOIndices {
		bindings[j] = cr.DescriptorBinding{Binding: i, Type: "storage", Count: 1}
	}
	dsLayout := device.CreateDescriptorSetLayout(bindings)
	plLayout := device.CreatePipelineLayout([]*cr.DescriptorSetLayout{dsLayout}, 0)
	pipeline := device.CreateComputePipeline(shader, plLayout)

	ds := device.CreateDescriptorSet(dsLayout)
	for _, i := range sortedSSBOIndices {
		if err := ds.Write(i, ssboBindings[i]); err != nil {
			return err
		}
	}

	// Record and submit
	_, err := gl.CreateAndSubmitCB(func(cb *cr.CommandBuffer) error {
		if err := cb.CmdBindPipeline(pipeline); err != nil {
			return err
		}
		if err := cb.CmdBindDescriptorSet(ds); err != nil {
			return err
		}
		return cb.CmdDispatch(numGroupsX, numGroupsY, numGroupsZ)
	}, nil)
	return err
}

// =================================================================
// Synchronization
// =================================================================

// MemoryBarrier inserts a memory barrier (glMemoryBarrier).
// In synchronous simulation, this is a no-op.
func (gl *GLContext) MemoryBarrier(barriers int) {}

// FenceSync creates a sync object (glFenceSync).
func (gl *GLContext) FenceSync() int {
	handle := gl.genID()
	fence := gl.LogicalDevice.CreateFence(true)
	gl.syncs[handle] = fence
	return handle
}

// ClientWaitSync waits for a sync object (glClientWaitSync).
func (gl *GLContext) ClientWaitSync(sync, flags, timeout int) int {
	fence, ok := gl.syncs[sync]
	if !ok {
		return GL_WAIT_FAILED
	}
	if fence.Signaled() {
		return GL_ALREADY_SIGNALED
	}
	result := fence.Wait(&timeout)
	if result {
		return GL_CONDITION_SATISFIED
	}
	return GL_TIMEOUT_EXPIRED
}

// DeleteSync deletes a sync object (glDeleteSync).
func (gl *GLContext) DeleteSync(sync int) {
	delete(gl.syncs, sync)
}

// Finish blocks until all GL commands complete (glFinish).
func (gl *GLContext) Finish() {
	gl.LogicalDevice.WaitIdle()
}

// =================================================================
// Uniforms (push constants)
// =================================================================

// GetUniformLocation gets the location of a uniform variable (glGetUniformLocation).
func (gl *GLContext) GetUniformLocation(program int, name string) (int, error) {
	if _, ok := gl.programs[program]; !ok {
		return 0, fmt.Errorf("invalid program handle %d", program)
	}
	// Return a deterministic location based on the name
	h := 0
	for _, c := range name {
		h = h*31 + int(c)
	}
	return h & 0x7FFFFFFF, nil
}

// Uniform1f sets a float uniform (glUniform1f).
func (gl *GLContext) Uniform1f(location int, value float64) {
	if gl.currentProgram != 0 {
		gl.uniforms[[2]interface{}{gl.currentProgram, location}] = value
	}
}

// Uniform1i sets an integer uniform (glUniform1i).
func (gl *GLContext) Uniform1i(location int, value int) {
	if gl.currentProgram != 0 {
		gl.uniforms[[2]interface{}{gl.currentProgram, location}] = value
	}
}

// sortedIntKeys returns the sorted keys of a map[int]*cr.Buffer.
func sortedIntKeys(m map[int]*cr.Buffer) []int {
	keys := make([]int, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	for i := 1; i < len(keys); i++ {
		for j := i; j > 0 && keys[j-1] > keys[j]; j-- {
			keys[j-1], keys[j] = keys[j], keys[j-1]
		}
	}
	return keys
}
