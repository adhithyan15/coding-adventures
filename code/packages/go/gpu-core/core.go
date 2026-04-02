package gpucore

// GPUCore -- the generic, pluggable accelerator processing element.
//
// # What is a GPU Core?
//
// A GPU core is the smallest independently programmable compute unit on a GPU.
// It's like a tiny, simplified CPU that does one thing well: floating-point math.
//
//	CPU Core (complex):                    GPU Core (simple):
//	+------------------------+             +----------------------+
//	| Branch predictor       |             |                      |
//	| Out-of-order engine    |             | In-order execution   |
//	| Large cache hierarchy  |             | Small register file  |
//	| Integer + FP ALUs      |             | FP ALU only          |
//	| Complex decoder        |             | Simple fetch-execute  |
//	| Speculative execution  |             | No speculation       |
//	+------------------------+             +----------------------+
//
// A single GPU core is MUCH simpler than a CPU core. GPUs achieve performance
// not through per-core complexity, but through massive parallelism: thousands
// of these simple cores running in parallel.
//
// # How This Core is Pluggable
//
// The GPUCore takes an InstructionSet as a constructor option. This ISA
// object handles all the vendor-specific decode and execute logic:
//
//	// Generic educational ISA (this package)
//	core := NewGPUCore()
//
//	// NVIDIA PTX (future package)
//	core := NewGPUCore(WithISA(PTXISA{}), WithNumRegisters(255))
//
//	// AMD GCN (future package)
//	core := NewGPUCore(WithISA(GCNISA{}), WithNumRegisters(256))
//
// The core itself (fetch loop, registers, memory, tracing) stays the same.
// Only the ISA changes.
//
// # Execution Model
//
// The GPU core uses a simple fetch-execute loop (no separate decode stage):
//
//	+------------------------------------------+
//	|              GPU Core                     |
//	|                                          |
//	|  +---------+    +------------------+     |
//	|  | Program |---->   Fetch          |     |
//	|  | Memory  |    |   instruction    |     |
//	|  +---------+    |   at PC          |     |
//	|                 +--------+---------+     |
//	|                          |               |
//	|                 +--------v---------+     |
//	|  +-----------+  |   ISA.Execute()  |     |
//	|  | Register  |<-|   (pluggable!)   |---->| Trace
//	|  | File      |->|                  |     |
//	|  +-----------+  +--------+---------+     |
//	|                          |               |
//	|  +-----------+  +--------v---------+     |
//	|  |  Local   |<- |  Update PC       |     |
//	|  |  Memory  |   +------------------+     |
//	|  +-----------+                           |
//	+------------------------------------------+
//
// Each Step():
//  1. Fetch: read instruction at program[PC]
//  2. Execute: call isa.Execute(instruction, registers, memory)
//  3. Update PC: advance based on ExecuteResult (branch or +1)
//  4. Return trace: GPUCoreTrace with full execution details

import (
	"fmt"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/fp-arithmetic"
)

// GPUCore is a generic GPU processing element with a pluggable instruction set.
//
// This is the central type of the package. It simulates a single processing
// element -- one CUDA core, one AMD stream processor, one Intel vector engine,
// or one ARM Mali execution engine -- depending on which InstructionSet you
// plug in.
type GPUCore struct {
	// ISA is the instruction set being used.
	ISA InstructionSet

	// Fmt is the floating-point format for registers.
	Fmt fp.FloatFormat

	// Registers is the floating-point register file.
	Registers *FPRegisterFile

	// Memory is the local scratchpad memory.
	Memory *LocalMemory

	// PC is the program counter (index into the loaded program).
	PC int

	// Cycle is the current clock cycle count.
	Cycle int

	// halted is true if the core has executed a HALT instruction.
	halted bool

	// program is the loaded instruction sequence.
	program []Instruction
}

// =========================================================================
// Functional options pattern -- idiomatic Go configuration
// =========================================================================
//
// Instead of having a constructor with many parameters (some with defaults),
// Go uses functional options. Each option is a function that modifies the
// core's configuration:
//
//	core := NewGPUCore()                          // all defaults
//	core := NewGPUCore(WithISA(myISA))            // custom ISA
//	core := NewGPUCore(WithNumRegisters(256))     // 256 registers
//	core := NewGPUCore(WithISA(ptx), WithMemorySize(8192))  // combo

// Option is a function that configures a GPUCore during construction.
type Option func(*gpuCoreConfig)

// gpuCoreConfig holds the configuration for a new GPUCore.
type gpuCoreConfig struct {
	isa          InstructionSet
	fmt          fp.FloatFormat
	numRegisters int
	memorySize   int
}

// WithISA sets the instruction set for the GPU core.
func WithISA(isa InstructionSet) Option {
	result, _ := StartNew[Option]("gpu-core.WithISA", nil,
		func(op *Operation[Option], rf *ResultFactory[Option]) *OperationResult[Option] {
			opt := func(c *gpuCoreConfig) {
				c.isa = isa
			}
			return rf.Generate(true, false, opt)
		}).GetResult()
	return result
}

// WithFormat sets the floating-point format for registers.
func WithFormat(format fp.FloatFormat) Option {
	result, _ := StartNew[Option]("gpu-core.WithFormat", nil,
		func(op *Operation[Option], rf *ResultFactory[Option]) *OperationResult[Option] {
			opt := func(c *gpuCoreConfig) {
				c.fmt = format
			}
			return rf.Generate(true, false, opt)
		}).GetResult()
	return result
}

// WithNumRegisters sets the number of floating-point registers.
func WithNumRegisters(n int) Option {
	result, _ := StartNew[Option]("gpu-core.WithNumRegisters", nil,
		func(op *Operation[Option], rf *ResultFactory[Option]) *OperationResult[Option] {
			opt := func(c *gpuCoreConfig) {
				c.numRegisters = n
			}
			return rf.Generate(true, false, opt)
		}).GetResult()
	return result
}

// WithMemorySize sets the local memory size in bytes.
func WithMemorySize(size int) Option {
	result, _ := StartNew[Option]("gpu-core.WithMemorySize", nil,
		func(op *Operation[Option], rf *ResultFactory[Option]) *OperationResult[Option] {
			opt := func(c *gpuCoreConfig) {
				c.memorySize = size
			}
			return rf.Generate(true, false, opt)
		}).GetResult()
	return result
}

// NewGPUCore creates a new GPU core with the given options.
//
// Default configuration:
//   - ISA: GenericISA
//   - Format: FP32
//   - Registers: 32
//   - Memory: 4096 bytes (4 KB)
//
// Example:
//
//	core := NewGPUCore()
//	core.LoadProgram([]Instruction{
//	    Limm(0, 3.0),
//	    Limm(1, 4.0),
//	    Fmul(2, 0, 1),
//	    Halt(),
//	})
//	traces := core.Run(10000)
//	val, _ := core.Registers.ReadFloat(2)
//	// val == 12.0
func NewGPUCore(opts ...Option) *GPUCore {
	result, _ := StartNew[*GPUCore]("gpu-core.NewGPUCore", nil,
		func(op *Operation[*GPUCore], rf *ResultFactory[*GPUCore]) *OperationResult[*GPUCore] {
			// Apply defaults.
			cfg := &gpuCoreConfig{
				isa:          GenericISA{},
				fmt:          fp.FP32,
				numRegisters: 32,
				memorySize:   4096,
			}

			// Apply user options.
			for _, opt := range opts {
				opt(cfg)
			}

			// Create subsystems. These constructors can't fail with valid defaults,
			// so we panic on error (indicates a programming bug in the options).
			regs, err := NewFPRegisterFile(cfg.numRegisters, cfg.fmt)
			if err != nil {
				panic(fmt.Sprintf("gpucore: invalid register config: %v", err))
			}
			mem, err := NewLocalMemory(cfg.memorySize)
			if err != nil {
				panic(fmt.Sprintf("gpucore: invalid memory config: %v", err))
			}

			return rf.Generate(true, false, &GPUCore{
				ISA:       cfg.isa,
				Fmt:       cfg.fmt,
				Registers: regs,
				Memory:    mem,
			})
		}).PanicOnUnexpected().GetResult()
	return result
}

// IsHalted returns true if the core has executed a HALT instruction.
func (c *GPUCore) IsHalted() bool {
	result, _ := StartNew[bool]("gpu-core.IsHalted", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, c.halted)
		}).GetResult()
	return result
}

// Halted implements the ProcessingElement interface.
func (c *GPUCore) Halted() bool {
	result, _ := StartNew[bool]("gpu-core.Halted", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, c.halted)
		}).GetResult()
	return result
}

// StepOne implements the ProcessingElement interface.
// It wraps Step() to return an interface{} for generic PE usage.
func (c *GPUCore) StepOne() (interface{}, error) {
	return StartNew[interface{}]("gpu-core.StepOne", nil,
		func(op *Operation[interface{}], rf *ResultFactory[interface{}]) *OperationResult[interface{}] {
			trace, err := c.Step()
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, interface{}(trace))
		}).GetResult()
}

// LoadProgram loads a program (list of instructions) into the core.
//
// This replaces any previously loaded program and resets the PC to 0,
// but does NOT reset registers or memory. Call Reset() for a full reset.
func (c *GPUCore) LoadProgram(program []Instruction) {
	_, _ = StartNew[struct{}]("gpu-core.LoadProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			c.program = make([]Instruction, len(program))
			copy(c.program, program)
			c.PC = 0
			c.halted = false
			c.Cycle = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Step executes one instruction and returns a trace of what happened.
//
// This is the core fetch-execute loop:
//  1. Check if halted or PC out of range
//  2. Fetch instruction at PC
//  3. Call ISA.Execute() to perform the operation
//  4. Update PC based on the result
//  5. Build and return a trace record
//
// Returns a GPUCoreTrace and an error. The error is non-nil if the core
// is halted or the PC is out of range.
func (c *GPUCore) Step() (GPUCoreTrace, error) {
	return StartNew[GPUCoreTrace]("gpu-core.Step", GPUCoreTrace{},
		func(op *Operation[GPUCoreTrace], rf *ResultFactory[GPUCoreTrace]) *OperationResult[GPUCoreTrace] {
			if c.halted {
				return rf.Fail(GPUCoreTrace{}, fmt.Errorf("cannot step: core is halted"))
			}

			if c.PC < 0 || c.PC >= len(c.program) {
				return rf.Fail(GPUCoreTrace{}, fmt.Errorf(
					"PC=%d out of program range [0, %d)", c.PC, len(c.program),
				))
			}

			// Fetch the instruction at the current program counter.
			instruction := c.program[c.PC]
			currentPC := c.PC
			c.Cycle++

			// Execute -- delegated to the pluggable ISA.
			result := c.ISA.Execute(instruction, c.Registers, c.Memory)

			// Update the program counter based on the execution result.
			var nextPC int
			if result.Halted {
				c.halted = true
				nextPC = currentPC // PC doesn't advance on halt
			} else if result.AbsoluteJump {
				nextPC = result.NextPCOffset
			} else {
				nextPC = currentPC + result.NextPCOffset
			}
			c.PC = nextPC

			// Build the execution trace.
			regsChanged := result.RegistersChanged
			if regsChanged == nil {
				regsChanged = make(map[string]float64)
			}
			memChanged := result.MemoryChanged
			if memChanged == nil {
				memChanged = make(map[int]float64)
			}

			return rf.Generate(true, false, GPUCoreTrace{
				Cycle:            c.Cycle,
				PC:               currentPC,
				Inst:             instruction,
				Description:      result.Description,
				NextPC:           nextPC,
				Halted:           result.Halted,
				RegistersChanged: regsChanged,
				MemoryChanged:    memChanged,
			})
		}).GetResult()
}

// Run executes the program until HALT or maxSteps reached.
//
// This repeatedly calls Step() until the core halts or the step limit is
// reached (preventing infinite loops from hanging).
//
// Returns a list of GPUCoreTrace records, one per instruction executed.
// Returns an error if maxSteps is exceeded (likely an infinite loop).
func (c *GPUCore) Run(maxSteps int) ([]GPUCoreTrace, error) {
	return StartNew[[]GPUCoreTrace]("gpu-core.Run", nil,
		func(op *Operation[[]GPUCoreTrace], rf *ResultFactory[[]GPUCoreTrace]) *OperationResult[[]GPUCoreTrace] {
			var traces []GPUCoreTrace
			steps := 0

			for !c.halted && steps < maxSteps {
				trace, err := c.Step()
				if err != nil {
					return rf.Fail(traces, err)
				}
				traces = append(traces, trace)
				steps++
			}

			if !c.halted && steps >= maxSteps {
				return rf.Fail(traces, fmt.Errorf(
					"execution limit reached (%d steps). Possible infinite loop. Last PC=%d",
					maxSteps, c.PC,
				))
			}

			return rf.Generate(true, false, traces)
		}).GetResult()
}

// Reset resets the core to its initial state.
//
// Clears registers, memory, PC, and cycle count. The loaded program
// is preserved -- call LoadProgram() to change it.
func (c *GPUCore) Reset() {
	_, _ = StartNew[struct{}]("gpu-core.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			regs, _ := NewFPRegisterFile(c.Registers.NumRegisters, c.Fmt)
			mem, _ := NewLocalMemory(c.Memory.Size)
			c.Registers = regs
			c.Memory = mem
			c.PC = 0
			c.Cycle = 0
			c.halted = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// String returns a human-readable representation of the core.
func (c *GPUCore) String() string {
	result, _ := StartNew[string]("gpu-core.String", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			status := fmt.Sprintf("running at PC=%d", c.PC)
			if c.halted {
				status = "halted"
			}
			return rf.Generate(true, false, fmt.Sprintf(
				"GPUCore(isa=%s, regs=%d, fmt=%s, %s)",
				c.ISA.Name(), c.Registers.NumRegisters, c.Fmt.Name, status,
			))
		}).GetResult()
	return result
}
