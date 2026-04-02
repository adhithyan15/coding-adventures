package brainfuck

// ==========================================================================
// Brainfuck Opcode Handlers — The Semantics of Each Command
// ==========================================================================
//
// This file defines the BrainfuckVM struct and its execution logic. Unlike
// the Python implementation (which dynamically attaches tape/dp attributes
// to a GenericVM), Go requires a concrete struct with typed fields.
//
// The BrainfuckVM wraps the virtual-machine package's types (CodeObject,
// Instruction, VMTrace) and implements its own execution loop with a
// switch statement — one case per Brainfuck opcode.
//
// ==========================================================================
// Brainfuck's State Model
// ==========================================================================
//
// Brainfuck doesn't use a stack, variables, or locals. Instead it has:
//
//   - Tape: A fixed array of 30,000 byte cells, all initialized to 0.
//     This is the program's entire memory. Each cell holds a value 0–255.
//
//   - DP (Data Pointer): An index into the tape, starting at 0. The ">"
//     and "<" commands move this pointer right and left.
//
//   - InputBuffer / InputPos: Simulates stdin. The "," command reads one
//     byte from here. When exhausted (EOF), reads return 0.
//
//   - PC (Program Counter): Which instruction we're executing next.
//
//   - Output: Accumulated output characters from "." commands.
//
// ==========================================================================
// Cell Wrapping
// ==========================================================================
//
// Brainfuck cells are unsigned bytes: values 0–255. Incrementing 255 wraps
// to 0; decrementing 0 wraps to 255. This is modular arithmetic:
//
//	cell = (cell + 1) % 256   // INC
//	cell = (cell - 1 + 256) % 256   // DEC
//
// Note: In Go, the % operator can return negative values for negative
// operands (unlike Python). So for DEC, we add 256 before taking the
// modulus to ensure correct wrapping: (0 - 1 + 256) % 256 == 255.
//
// ==========================================================================
// Error Handling
// ==========================================================================
//
// Following the virtual-machine package's convention, runtime errors cause
// panics. Moving the data pointer past either end of the tape is an error
// (some implementations wrap; we error because silent wrapping hides bugs).

import (
	"fmt"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// TapeSize is the number of cells on the Brainfuck tape.
//
// The original Brainfuck specification uses 30,000 cells. Some
// implementations use more (or dynamically grow), but 30,000 is
// the classic size that matches the original language definition.
const TapeSize = 30000

// BrainfuckVM holds all state needed to execute a Brainfuck program.
//
// It combines execution state (PC, Halted, Output) with Brainfuck-specific
// state (Tape, DP, InputBuffer, InputPos). This is the Go equivalent of
// Python's approach of dynamically attaching attributes to a GenericVM.
//
// Fields:
//
//   - Tape: The 30,000-cell memory array. Each cell is a byte (0–255).
//   - DP: Data pointer — index of the "current" cell.
//   - PC: Program counter — index of the next instruction to execute.
//   - Halted: Set to true when HALT is reached.
//   - Output: Characters produced by "." commands, collected in order.
//   - InputBuffer: The input string (simulates stdin).
//   - InputPos: Current read position in InputBuffer.
type BrainfuckVM struct {
	Tape        []byte
	DP          int
	PC          int
	Halted      bool
	Output      []string
	InputBuffer string
	InputPos    int
}

// NewBrainfuckVM creates a BrainfuckVM with all state initialized.
//
// The tape is zeroed, the data pointer starts at cell 0, and the input
// buffer is set to the provided string (empty string means all ","
// commands will read EOF / 0).
func NewBrainfuckVM(inputData string) *BrainfuckVM {
	result, _ := StartNew[*BrainfuckVM]("brainfuck.NewBrainfuckVM", nil,
		func(op *Operation[*BrainfuckVM], rf *ResultFactory[*BrainfuckVM]) *OperationResult[*BrainfuckVM] {
			op.AddProperty("inputDataLen", len(inputData))
			tape := make([]byte, TapeSize)
			return rf.Generate(true, false, &BrainfuckVM{
				Tape:        tape,
				DP:          0,
				PC:          0,
				Halted:      false,
				Output:      []string{},
				InputBuffer: inputData,
				InputPos:    0,
			})
		}).GetResult()
	return result
}

// Execute runs a complete Brainfuck program (CodeObject) and returns
// a trace of every instruction executed.
//
// Execution continues until either:
//   - A HALT instruction is reached (vm.Halted becomes true), or
//   - The program counter moves past the end of the instruction list.
//
// Each step produces a VMTrace entry recording the PC, instruction,
// and any output. This is useful for debugging and visualization.
func (bvm *BrainfuckVM) Execute(code vm.CodeObject) []vm.VMTrace {
	result, _ := StartNew[[]vm.VMTrace]("brainfuck.Execute", nil,
		func(_ *Operation[[]vm.VMTrace], rf *ResultFactory[[]vm.VMTrace]) *OperationResult[[]vm.VMTrace] {
			var traces []vm.VMTrace
			for !bvm.Halted && bvm.PC < len(code.Instructions) {
				trace := bvm.step(code)
				traces = append(traces, trace)
			}
			return rf.Generate(true, false, traces)
		}).PanicOnUnexpected().GetResult()
	return result
}

// Step executes a single instruction and returns a VMTrace describing
// what happened.
//
// This is the heart of the Brainfuck interpreter. It implements a
// fetch-decode-execute cycle:
//
//  1. **Fetch**: Read the instruction at bvm.PC.
//  2. **Decode**: Switch on the opcode to determine which handler to run.
//  3. **Execute**: Perform the opcode's action (move pointer, modify cell,
//     do I/O, or branch).
//  4. **Trace**: Record what happened for debugging.
//
// Each opcode handler is documented inline below.
func (bvm *BrainfuckVM) Step(code vm.CodeObject) vm.VMTrace {
	result, _ := StartNew[vm.VMTrace]("brainfuck.Step", vm.VMTrace{},
		func(_ *Operation[vm.VMTrace], rf *ResultFactory[vm.VMTrace]) *OperationResult[vm.VMTrace] {
			return rf.Generate(true, false, bvm.step(code))
		}).PanicOnUnexpected().GetResult()
	return result
}

// step is the internal (non-instrumented) implementation of Step.
// Called by Execute to avoid nested Operation instrumentation.
func (bvm *BrainfuckVM) step(code vm.CodeObject) vm.VMTrace {
	instr := code.Instructions[bvm.PC]
	pcBefore := bvm.PC
	var outputVal *string
	desc := ""

	switch instr.Opcode {

	// -----------------------------------------------------------------
	// ">" — Move data pointer right
	// -----------------------------------------------------------------
	// The data pointer advances to the next cell. If we're already at
	// the last cell (index TapeSize-1), this is an error — the program
	// is trying to access memory beyond the tape.
	case OpRight:
		bvm.DP++
		if bvm.DP >= TapeSize {
			panic(fmt.Sprintf(
				"BrainfuckError: Data pointer moved past end of tape (position %d). "+
					"The tape has %d cells (indices 0–%d).",
				bvm.DP, TapeSize, TapeSize-1,
			))
		}
		bvm.PC++
		desc = fmt.Sprintf("Move data pointer right to cell %d", bvm.DP)

	// -----------------------------------------------------------------
	// "<" — Move data pointer left
	// -----------------------------------------------------------------
	// The data pointer moves back to the previous cell. If we're at
	// cell 0, this is an error — there's nothing to the left.
	case OpLeft:
		bvm.DP--
		if bvm.DP < 0 {
			panic(
				"BrainfuckError: Data pointer moved before start of tape (position -1). " +
					"The tape starts at index 0.",
			)
		}
		bvm.PC++
		desc = fmt.Sprintf("Move data pointer left to cell %d", bvm.DP)

	// -----------------------------------------------------------------
	// "+" — Increment the current cell
	// -----------------------------------------------------------------
	// The byte at the data pointer increases by 1. If the cell is 255,
	// it wraps around to 0 (unsigned byte arithmetic).
	//
	// Truth table for edge cases:
	//   cell=0   → cell=1
	//   cell=254 → cell=255
	//   cell=255 → cell=0   (wrap!)
	case OpInc:
		bvm.Tape[bvm.DP] = byte((int(bvm.Tape[bvm.DP]) + 1) % 256)
		bvm.PC++
		desc = fmt.Sprintf("Increment cell %d to %d", bvm.DP, bvm.Tape[bvm.DP])

	// -----------------------------------------------------------------
	// "-" — Decrement the current cell
	// -----------------------------------------------------------------
	// The byte at the data pointer decreases by 1. If the cell is 0,
	// it wraps around to 255 (unsigned byte arithmetic).
	//
	// Note: Go's % operator can produce negative results, so we add 256
	// before taking the modulus: (0 - 1 + 256) % 256 == 255.
	//
	// Truth table for edge cases:
	//   cell=1   → cell=0
	//   cell=0   → cell=255  (wrap!)
	//   cell=255 → cell=254
	case OpDec:
		bvm.Tape[bvm.DP] = byte((int(bvm.Tape[bvm.DP]) - 1 + 256) % 256)
		bvm.PC++
		desc = fmt.Sprintf("Decrement cell %d to %d", bvm.DP, bvm.Tape[bvm.DP])

	// -----------------------------------------------------------------
	// "." — Output the current cell as an ASCII character
	// -----------------------------------------------------------------
	// Converts the byte value to a character and appends it to the
	// output buffer. For example, cell value 72 outputs 'H'.
	//
	// The output is also recorded in the VMTrace so that debugging
	// tools can show exactly which instruction produced which output.
	case OpOutput:
		ch := string(rune(bvm.Tape[bvm.DP]))
		bvm.Output = append(bvm.Output, ch)
		outputVal = &ch
		bvm.PC++
		desc = fmt.Sprintf("Output cell %d as '%s' (ASCII %d)", bvm.DP, ch, bvm.Tape[bvm.DP])

	// -----------------------------------------------------------------
	// "," — Read one byte of input into the current cell
	// -----------------------------------------------------------------
	// Reads the next byte from the input buffer. If the input is
	// exhausted (EOF), the cell is set to 0.
	//
	// Different Brainfuck implementations handle EOF differently:
	//   - Set cell to 0 (our choice — clean and predictable)
	//   - Set cell to 255 (-1 in unsigned)
	//   - Leave cell unchanged
	//
	// We chose 0 because it makes ",[.,]" (cat program) work naturally:
	// the loop exits when EOF produces a 0 cell.
	case OpInput:
		if bvm.InputPos < len(bvm.InputBuffer) {
			bvm.Tape[bvm.DP] = bvm.InputBuffer[bvm.InputPos]
			bvm.InputPos++
		} else {
			// EOF: set cell to 0
			bvm.Tape[bvm.DP] = 0
		}
		bvm.PC++
		desc = fmt.Sprintf("Read input into cell %d (value %d)", bvm.DP, bvm.Tape[bvm.DP])

	// -----------------------------------------------------------------
	// "[" — Loop start (conditional forward jump)
	// -----------------------------------------------------------------
	// This implements the "while" condition:
	//
	//   if tape[dp] == 0:
	//       jump to instruction after matching "]"
	//   else:
	//       continue to next instruction (enter loop body)
	//
	// The operand holds the target address: one past the matching "]".
	// If the cell is already zero, we skip the entire loop. If nonzero,
	// we fall through into the loop body.
	case OpLoopStart:
		if bvm.Tape[bvm.DP] == 0 {
			// Cell is zero — skip the loop entirely.
			target := instr.Operand.(int)
			bvm.PC = target
			desc = fmt.Sprintf("Cell %d is 0, skip loop to instruction %d", bvm.DP, target)
		} else {
			// Cell is nonzero — enter the loop body.
			bvm.PC++
			desc = fmt.Sprintf("Cell %d is %d (nonzero), enter loop", bvm.DP, bvm.Tape[bvm.DP])
		}

	// -----------------------------------------------------------------
	// "]" — Loop end (conditional backward jump)
	// -----------------------------------------------------------------
	// This implements the loop-back:
	//
	//   if tape[dp] != 0:
	//       jump back to matching "["
	//   else:
	//       continue to next instruction (exit loop)
	//
	// The operand holds the index of the matching "[". Together with
	// LOOP_START, this creates:
	//
	//   while tape[dp] != 0:
	//       <loop body>
	case OpLoopEnd:
		if bvm.Tape[bvm.DP] != 0 {
			// Cell is nonzero — loop again.
			target := instr.Operand.(int)
			bvm.PC = target
			desc = fmt.Sprintf("Cell %d is %d (nonzero), loop back to instruction %d", bvm.DP, bvm.Tape[bvm.DP], target)
		} else {
			// Cell is zero — exit the loop.
			bvm.PC++
			desc = fmt.Sprintf("Cell %d is 0, exit loop", bvm.DP)
		}

	// -----------------------------------------------------------------
	// HALT — Stop execution
	// -----------------------------------------------------------------
	// Every translated program ends with HALT. This sets the Halted flag,
	// which causes the Execute loop to stop.
	case OpHalt:
		bvm.Halted = true
		desc = "Halt execution"

	default:
		panic(fmt.Sprintf("BrainfuckError: Unknown opcode %d", instr.Opcode))
	}

	return vm.VMTrace{
		PC:          pcBefore,
		Instruction: instr,
		StackBefore: []interface{}{},
		StackAfter:  []interface{}{},
		Variables:   map[string]interface{}{},
		Output:      outputVal,
		Description: desc,
	}
}
