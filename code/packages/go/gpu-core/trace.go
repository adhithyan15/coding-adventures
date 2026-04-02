package gpucore

// Execution traces -- making every instruction's journey visible.
//
// # Why Traces?
//
// A key principle of this project is educational transparency: every operation
// should be observable. When a GPU core executes an instruction, the trace
// records exactly what happened:
//
//	Cycle 3 | PC=2 | FFMA R3, R0, R1, R2
//	-> R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
//	-> Registers changed: {R3: 7.0}
//	-> Next PC: 3
//
// This lets a student (or debugger) follow the execution step by step,
// understanding not just *what* the GPU did but *why* -- which registers were
// read, what computation was performed, and what state changed.
//
// # Trace vs Log
//
// A trace is more structured than a log message. Each field is typed and
// accessible programmatically, which enables:
//   - Automated testing (assert trace.RegistersChanged["R3"] == 7.0)
//   - Visualization tools (render execution as a timeline)
//   - Performance analysis (count cycles, track register usage)

import "fmt"

// GPUCoreTrace is a record of one instruction's execution on a GPU core.
//
// Every call to GPUCore.Step() returns one of these, providing full
// visibility into what the instruction did.
//
// Fields:
//   - Cycle: The clock cycle number (1-indexed).
//   - PC: The program counter BEFORE this instruction executed.
//   - Inst: The instruction that was executed.
//   - Description: Human-readable description of what happened.
//     Example: "R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0"
//   - RegistersChanged: Which registers changed and their new values.
//     Example: {"R3": 7.0}
//   - MemoryChanged: Which memory addresses changed and their new values.
//     Example: {0: 3.14, 4: 2.71}
//   - NextPC: The program counter AFTER this instruction.
//   - Halted: True if this instruction stopped execution.
type GPUCoreTrace struct {
	Cycle             int
	PC                int
	Inst              Instruction
	Description       string
	NextPC            int
	Halted            bool
	RegistersChanged  map[string]float64
	MemoryChanged     map[int]float64
}

// Format pretty-prints this trace record for educational display.
//
// Returns a multi-line string like:
//
//	[Cycle 3] PC=2: FFMA R3, R0, R1, R2
//	  -> R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
//	  -> Registers: {R3=7}
//	  -> Next PC: 3
func (t GPUCoreTrace) Format() string {
	result, _ := StartNew[string]("gpu-core.GPUCoreTrace.Format", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			lines := fmt.Sprintf("[Cycle %d] PC=%d: %s", t.Cycle, t.PC, t.Inst.String())
			lines += fmt.Sprintf("\n  -> %s", t.Description)

			if len(t.RegistersChanged) > 0 {
				regs := ""
				for k, v := range t.RegistersChanged {
					if regs != "" {
						regs += ", "
					}
					regs += fmt.Sprintf("%s=%g", k, v)
				}
				lines += fmt.Sprintf("\n  -> Registers: {%s}", regs)
			}

			if len(t.MemoryChanged) > 0 {
				mems := ""
				for k, v := range t.MemoryChanged {
					if mems != "" {
						mems += ", "
					}
					mems += fmt.Sprintf("0x%04X=%g", k, v)
				}
				lines += fmt.Sprintf("\n  -> Memory: {%s}", mems)
			}

			if t.Halted {
				lines += "\n  -> HALTED"
			} else {
				lines += fmt.Sprintf("\n  -> Next PC: %d", t.NextPC)
			}

			return rf.Generate(true, false, lines)
		}).GetResult()
	return result
}
