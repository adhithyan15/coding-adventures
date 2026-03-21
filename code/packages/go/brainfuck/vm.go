package brainfuck

// ==========================================================================
// Brainfuck VM Factory — Convenience Functions for Running Programs
// ==========================================================================
//
// This file provides two things:
//
//  1. BrainfuckResult — a struct that captures everything about a completed
//     Brainfuck execution: the output text, final tape state, data pointer
//     position, execution traces, and step count.
//
//  2. Convenience functions:
//     - CreateBrainfuckVM(inputData) — creates a ready-to-use BrainfuckVM.
//     - ExecuteBrainfuck(source, inputData) — the one-call "just run it"
//       function that translates and executes in a single step.
//
// ==========================================================================
// Usage Examples
// ==========================================================================
//
// Quick execution (most common):
//
//	result := ExecuteBrainfuck("+++.", "")
//	fmt.Println(result.Output)  // "\x03"
//
// Step-by-step (for debugging):
//
//	code := Translate("+++.")
//	bvm := CreateBrainfuckVM("")
//	traces := bvm.Execute(code)
//	// Inspect traces, bvm.Tape, bvm.DP, etc.

import (
	"strings"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// BrainfuckResult holds everything about a completed Brainfuck execution.
//
// This is the Go equivalent of Python's @dataclass BrainfuckResult. It
// bundles the program's output with enough state to inspect, debug, and
// test the execution.
//
// Fields:
//
//   - Output: The program's output as a single string (concatenation of
//     all "." commands). For example, "Hello World!\n".
//
//   - Tape: The final state of all 30,000 cells. Useful for testing
//     programs that compute values without printing them.
//
//   - DP: The final data pointer position. After ">>>", DP would be 3.
//
//   - Traces: A slice of VMTrace entries, one per executed instruction.
//     Useful for step-by-step debugging and visualization.
//
//   - Steps: Total number of instructions executed. Equal to len(Traces).
type BrainfuckResult struct {
	Output string
	Tape   []byte
	DP     int
	Traces []vm.VMTrace
	Steps  int
}

// CreateBrainfuckVM creates a BrainfuckVM initialized and ready for
// execution.
//
// This is a thin wrapper around NewBrainfuckVM that exists for API
// consistency with the Python implementation's create_brainfuck_vm().
//
// Parameters:
//
//   - inputData: Input to feed to "," commands. Each byte of the string
//     is one input byte. Empty string means all "," commands produce 0
//     (EOF behavior).
//
// Example:
//
//	bvm := CreateBrainfuckVM("Hello")
//	code := Translate(",.")
//	bvm.Execute(code)
//	// bvm.Output is ["H"]
func CreateBrainfuckVM(inputData string) *BrainfuckVM {
	return NewBrainfuckVM(inputData)
}

// ExecuteBrainfuck translates and executes a Brainfuck program in one call.
//
// This is the convenience function for quick execution. It handles the
// full pipeline: source → translate → create VM → execute → result.
//
// Parameters:
//
//   - source: The Brainfuck source code. Non-command characters are
//     ignored (treated as comments).
//
//   - inputData: Input bytes for "," commands. Empty string means EOF.
//
// Returns a BrainfuckResult with the output, final tape state, and
// execution traces.
//
// Panics if brackets are mismatched (translation error) or if the data
// pointer moves past the tape boundaries (runtime error).
//
// Examples:
//
// Simple addition (2 + 5 = 7):
//
//	result := ExecuteBrainfuck("++>+++++[<+>-]", "")
//	// result.Tape[0] == 7
//
// Hello character (ASCII 72 = 'H'):
//
//	result := ExecuteBrainfuck("+++++++++[>++++++++<-]>.", "")
//	// result.Output == "H"
func ExecuteBrainfuck(source string, inputData string) BrainfuckResult {
	// Step 1: Translate source to bytecode.
	code := Translate(source)

	// Step 2: Create a VM with the specified input.
	bvm := CreateBrainfuckVM(inputData)

	// Step 3: Execute the program.
	traces := bvm.Execute(code)

	// Step 4: Package the results.
	// Join all output strings (each "." produces one character).
	output := strings.Join(bvm.Output, "")

	// Copy the tape so the caller gets an independent snapshot.
	tapeCopy := make([]byte, len(bvm.Tape))
	copy(tapeCopy, bvm.Tape)

	return BrainfuckResult{
		Output: output,
		Tape:   tapeCopy,
		DP:     bvm.DP,
		Traces: traces,
		Steps:  len(traces),
	}
}
