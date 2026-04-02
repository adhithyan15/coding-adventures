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
	result, _ := StartNew[*CPU]("cpu-simulator.NewCPU", nil,
		func(op *Operation[*CPU], rf *ResultFactory[*CPU]) *OperationResult[*CPU] {
			op.AddProperty("num_registers", numRegisters)
			op.AddProperty("bit_width", bitWidth)
			op.AddProperty("memory_size", memorySize)
			return rf.Generate(true, false, &CPU{
				Registers: NewRegisterFile(numRegisters, bitWidth),
				Memory:    NewMemory(memorySize),
				PC:        0,
				Halted:    false,
				Cycle:     0,
				Decoder:   decoder,
				Executor:  executor,
			})
		}).GetResult()
	return result
}

// State dumps all important tracking information.
func (cpu *CPU) State() CPUState {
	result, _ := StartNew[CPUState]("cpu-simulator.CPU.State", CPUState{},
		func(op *Operation[CPUState], rf *ResultFactory[CPUState]) *OperationResult[CPUState] {
			return rf.Generate(true, false, CPUState{
				PC:        cpu.PC,
				Registers: cpu.Registers.Dump(),
				Halted:    cpu.Halted,
				Cycle:     cpu.Cycle,
			})
		}).GetResult()
	return result
}

// LoadProgram primes the memory address space with instructions.
func (cpu *CPU) LoadProgram(program []byte, startAddress int) {
	_, _ = StartNew[struct{}]("cpu-simulator.CPU.LoadProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("start_address", startAddress)
			op.AddProperty("program_size", len(program))
			cpu.Memory.LoadBytes(startAddress, program)
			cpu.PC = startAddress
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Step performs exact ONE cycle.
func (cpu *CPU) Step() PipelineTrace {
	result, _ := StartNew[PipelineTrace]("cpu-simulator.CPU.Step", PipelineTrace{},
		func(op *Operation[PipelineTrace], rf *ResultFactory[PipelineTrace]) *OperationResult[PipelineTrace] {
			op.AddProperty("cycle", cpu.Cycle)
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
			return rf.Generate(true, false, trace)
		}).GetResult()
	return result
}

// Run executes instructions until a halt state or the max instructions limit logic avoids infinite loops.
func (cpu *CPU) Run(maxSteps int) []PipelineTrace {
	result, _ := StartNew[[]PipelineTrace]("cpu-simulator.CPU.Run", nil,
		func(op *Operation[[]PipelineTrace], rf *ResultFactory[[]PipelineTrace]) *OperationResult[[]PipelineTrace] {
			op.AddProperty("max_steps", maxSteps)
			var traces []PipelineTrace
			for i := 0; i < maxSteps; i++ {
				if cpu.Halted {
					break
				}
				traces = append(traces, cpu.Step())
			}
			return rf.Generate(true, false, traces)
		}).GetResult()
	return result
}
