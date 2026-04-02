// Package gpucore implements a generic, pluggable GPU processing element simulator.
//
// # Why Protocols (Interfaces in Go)?
//
// Every GPU vendor (NVIDIA, AMD, Intel, ARM) and every accelerator type (GPU,
// TPU, NPU) has a processing element at its heart. They all do the same basic
// thing: compute floating-point operations. But the details differ:
//
//	NVIDIA CUDA Core:     FP32 ALU + 255 registers + PTX instructions
//	AMD Stream Processor: FP32 ALU + 256 VGPRs + GCN instructions
//	Intel Vector Engine:  SIMD8 ALU + GRF + Xe instructions
//	ARM Mali Exec Engine: FP32 ALU + register bank + Mali instructions
//	TPU Processing Element: MAC unit + weight register + accumulator
//	NPU MAC Unit:         MAC + activation function + buffer
//
// Instead of building separate simulators for each, we define two interfaces:
//
//  1. ProcessingElement -- the generic "any compute unit" interface
//  2. InstructionSet -- the pluggable "how to decode and execute instructions"
//
// Any vendor-specific implementation just needs to satisfy these interfaces.
// The core simulation infrastructure (registers, memory, tracing) is reused.
//
// # Go Interfaces vs Python Protocols
//
// In Python, a Protocol is structural (duck typing). In Go, interfaces are
// also structural -- if a type has the right methods, it satisfies the interface
// automatically without declaring it. This is the same concept:
//
//	type Flyable interface {
//	    Fly()
//	}
//
//	type Bird struct{}
//	func (b Bird) Fly() { fmt.Println("flap flap") }
//
//	type Airplane struct{}
//	func (a Airplane) Fly() { fmt.Println("zoom") }
//
//	// Both Bird and Airplane satisfy Flyable -- no explicit declaration needed!
package gpucore

// =========================================================================
// ExecuteResult -- what an instruction execution produces
// =========================================================================

// ExecuteResult is the outcome of executing a single instruction.
//
// This is what the InstructionSet's Execute() method returns. It tells the
// core what changed so the core can build a complete execution trace.
//
// Fields:
//   - Description: Human-readable summary, e.g. "R3 = R1 * R2 = 6.0"
//   - NextPCOffset: How to advance the program counter.
//     +1 for most instructions (next instruction).
//     Other values for branches/jumps.
//   - AbsoluteJump: If true, NextPCOffset is an absolute address,
//     not a relative offset.
//   - RegistersChanged: Map of register name -> new float value.
//   - MemoryChanged: Map of memory address -> new float value.
//   - Halted: True if this instruction stops execution.
type ExecuteResult struct {
	Description      string
	NextPCOffset     int
	AbsoluteJump     bool
	RegistersChanged map[string]float64
	MemoryChanged    map[int]float64
	Halted           bool
}

// NewExecuteResult creates an ExecuteResult with sensible defaults.
//
// By default, the PC advances by 1 (the next instruction), the core is not
// halted, and no registers or memory changed. This mirrors the Python
// dataclass defaults.
func NewExecuteResult(description string) ExecuteResult {
	result, _ := StartNew[ExecuteResult]("gpu-core.NewExecuteResult", ExecuteResult{},
		func(op *Operation[ExecuteResult], rf *ResultFactory[ExecuteResult]) *OperationResult[ExecuteResult] {
			return rf.Generate(true, false, ExecuteResult{
				Description:  description,
				NextPCOffset: 1,
			})
		}).GetResult()
	return result
}

// =========================================================================
// InstructionSet -- pluggable ISA (the key to vendor-agnosticism)
// =========================================================================

// InstructionSet is a pluggable instruction set that can be swapped to
// simulate any vendor's ISA.
//
// The GPUCore calls isa.Execute(instruction, registers, memory) for each
// instruction. The ISA implementation:
//  1. Reads the opcode to determine what operation to perform
//  2. Reads source registers and/or memory
//  3. Performs the computation (using FPAdd, FPMul, FMA, etc.)
//  4. Writes the result to the destination register and/or memory
//  5. Returns an ExecuteResult describing what happened
//
// To add support for a new vendor (e.g., NVIDIA PTX):
//
//	type PTXISA struct{}
//	func (p PTXISA) Name() string { return "PTX" }
//	func (p PTXISA) Execute(inst Instruction, regs *FPRegisterFile, mem *LocalMemory) ExecuteResult {
//	    switch inst.Op {
//	    case PTXOpADDF32: ...
//	    case PTXOpFMARNF32: ...
//	    }
//	}
//
//	core := NewGPUCore(WithISA(PTXISA{}))
type InstructionSet interface {
	// Name returns the ISA name, e.g. "Generic", "PTX", "GCN", "Xe", "Mali".
	Name() string

	// Execute decodes and executes a single instruction.
	Execute(inst Instruction, regs *FPRegisterFile, mem *LocalMemory) ExecuteResult
}

// =========================================================================
// ProcessingElement -- the most generic abstraction
// =========================================================================

// ProcessingElement is the interface for any compute unit in any accelerator.
//
// This is the most generic interface -- a GPU core, a TPU processing element,
// and an NPU MAC unit all satisfy this interface. It provides just enough
// structure for a higher-level component (like a warp scheduler or systolic
// array controller) to drive the PE.
//
// Why so minimal? Different accelerators have radically different execution models:
//   - GPUs: instruction-stream + register file (step = execute one instruction)
//   - TPUs: dataflow, no instructions (step = one MAC + pass data to neighbor)
//   - NPUs: scheduled MACs (step = one MAC from the scheduler's queue)
//
// This interface captures only what they ALL share: the ability to advance
// one cycle, check if done, and reset.
type ProcessingElement interface {
	// StepOne executes one cycle. Returns a trace of what happened.
	// Named StepOne to avoid conflict with GPUCore.Step which returns
	// a typed (GPUCoreTrace, error) pair for direct use.
	StepOne() (interface{}, error)

	// Halted returns true if this PE has finished execution.
	Halted() bool

	// Reset resets to initial state.
	Reset()
}
