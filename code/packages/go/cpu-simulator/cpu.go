// Package cpusimulator provides the central CPU operations block.
package cpusimulator

// InstructionDecoder must be implemented by an ISA (like ARM or RISC-V).
type InstructionDecoder interface {
	Decode(rawInstruction uint32, pc int) DecodeResult
}

// InstructionExecutor must be implemented by an ISA to modify state.
type InstructionExecutor interface {
	Execute(decoded DecodeResult, registers *RegisterFile, memory *Memory, pc int) ExecuteResult
}

// CPUState is a point-in-time snapshot.
type CPUState struct {
	PC        int
	Registers map[string]uint32
	Halted    bool
	Cycle     int
}

// CPU drives the fetch-decode-execute cycle using a generic ISA decoder/executor.
type CPU struct {
	Registers *RegisterFile
	Memory    *Memory
	PC        int
	Halted    bool
	Cycle     int
	Decoder   InstructionDecoder
	Executor  InstructionExecutor
}

// NewCPU creates the simulated computing core.
func NewCPU(decoder InstructionDecoder, executor InstructionExecutor, numRegisters, bitWidth, memorySize int) *CPU {
	return &CPU{
		Registers: NewRegisterFile(numRegisters, bitWidth),
		Memory:    NewMemory(memorySize),
		PC:        0,
		Halted:    false,
		Cycle:     0,
		Decoder:   decoder,
		Executor:  executor,
	}
}

// State dumps all important tracking information.
func (cpu *CPU) State() CPUState {
	return CPUState{
		PC:        cpu.PC,
		Registers: cpu.Registers.Dump(),
		Halted:    cpu.Halted,
		Cycle:     cpu.Cycle,
	}
}

// LoadProgram primes the memory address space with instructions.
func (cpu *CPU) LoadProgram(program []byte, startAddress int) {
	cpu.Memory.LoadBytes(startAddress, program)
	cpu.PC = startAddress
}

// Step performs exact ONE cycle.
func (cpu *CPU) Step() PipelineTrace {
	if cpu.Halted {
		panic("CPU has halted — no more instructions to execute")
	}

	// 1: FETCH
	rawInstruction := cpu.Memory.ReadWord(cpu.PC)
	fetchResult := FetchResult{PC: cpu.PC, RawInstruction: rawInstruction}

	// 2: DECODE
	decodeResult := cpu.Decoder.Decode(rawInstruction, cpu.PC)

	// 3: EXECUTE
	executeResult := cpu.Executor.Execute(decodeResult, cpu.Registers, cpu.Memory, cpu.PC)

	cpu.PC = executeResult.NextPC
	cpu.Halted = executeResult.Halted

	trace := PipelineTrace{
		Cycle:            cpu.Cycle,
		Fetch:            fetchResult,
		Decode:           decodeResult,
		Execute:          executeResult,
		RegisterSnapshot: cpu.Registers.Dump(),
	}

	cpu.Cycle++
	return trace
}

// Run executes instructions until a halt state or the max instructions limit logic avoids infinite loops.
func (cpu *CPU) Run(maxSteps int) []PipelineTrace {
	var traces []PipelineTrace
	for i := 0; i < maxSteps; i++ {
		if cpu.Halted {
			break
		}
		traces = append(traces, cpu.Step())
	}
	return traces
}
