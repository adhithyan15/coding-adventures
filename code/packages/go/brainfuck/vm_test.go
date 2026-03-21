package brainfuck

// ==========================================================================
// End-to-End Tests — Real Brainfuck Programs Translated and Executed
// ==========================================================================
//
// These tests mirror the Python test_e2e.py tests. Each test uses the
// ExecuteBrainfuck() convenience function, which handles the full pipeline:
// source → translate → create VM → execute → BrainfuckResult.
//
// This is the highest-level test suite: if these pass, the translator,
// handlers, and VM factory are all working together correctly.

import (
	"strings"
	"testing"
)

// =========================================================================
// Simple Programs
// =========================================================================

// TestE2EEmptyProgram verifies that an empty program produces no output
// and leaves the tape zeroed.
func TestE2EEmptyProgram(t *testing.T) {
	result := ExecuteBrainfuck("", "")
	if result.Output != "" {
		t.Errorf("Expected empty output, got %q", result.Output)
	}
	if result.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0, got %d", result.Tape[0])
	}
}

// TestE2ESingleInc verifies "+" sets cell 0 to 1.
func TestE2ESingleInc(t *testing.T) {
	result := ExecuteBrainfuck("+", "")
	if result.Tape[0] != 1 {
		t.Errorf("Expected Tape[0]=1, got %d", result.Tape[0])
	}
}

// TestE2EAddition verifies the classic addition pattern: 2 + 5 = 7.
//
// Put 2 in cell 0, 5 in cell 1. Loop: decrement cell 1, increment cell 0.
// Result: 7 in cell 0, 0 in cell 1.
func TestE2EAddition(t *testing.T) {
	result := ExecuteBrainfuck("++>+++++[<+>-]", "")
	if result.Tape[0] != 7 {
		t.Errorf("Expected Tape[0]=7, got %d", result.Tape[0])
	}
	if result.Tape[1] != 0 {
		t.Errorf("Expected Tape[1]=0, got %d", result.Tape[1])
	}
}

// TestE2EMoveValue verifies moving a value from cell 0 to cell 1.
//
// Set cell 0 to 5, then [>+<-] moves it to cell 1.
func TestE2EMoveValue(t *testing.T) {
	result := ExecuteBrainfuck("+++++[>+<-]", "")
	if result.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0, got %d", result.Tape[0])
	}
	if result.Tape[1] != 5 {
		t.Errorf("Expected Tape[1]=5, got %d", result.Tape[1])
	}
}

// TestE2ECellWrappingOverflow verifies 255 + 1 = 0 (byte wrapping).
func TestE2ECellWrappingOverflow(t *testing.T) {
	source := strings.Repeat("+", 256)
	result := ExecuteBrainfuck(source, "")
	if result.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0 (wrapped), got %d", result.Tape[0])
	}
}

// TestE2ECellWrappingUnderflow verifies 0 - 1 = 255 (byte wrapping).
func TestE2ECellWrappingUnderflow(t *testing.T) {
	result := ExecuteBrainfuck("-", "")
	if result.Tape[0] != 255 {
		t.Errorf("Expected Tape[0]=255 (wrapped), got %d", result.Tape[0])
	}
}

// TestE2ESkipEmptyLoop verifies [] is skipped when cell is 0.
func TestE2ESkipEmptyLoop(t *testing.T) {
	result := ExecuteBrainfuck("[]+++", "")
	if result.Tape[0] != 3 {
		t.Errorf("Expected Tape[0]=3, got %d", result.Tape[0])
	}
}

// =========================================================================
// Output Tests
// =========================================================================

// TestE2EOutputH verifies outputting 'H' (ASCII 72 = 9 * 8).
func TestE2EOutputH(t *testing.T) {
	result := ExecuteBrainfuck("+++++++++[>++++++++<-]>.", "")
	if result.Output != "H" {
		t.Errorf("Expected output 'H', got %q", result.Output)
	}
}

// TestE2EOutputMultipleChars verifies outputting 'AB' by setting cell
// to 65 ('A'), outputting, incrementing, and outputting again.
func TestE2EOutputMultipleChars(t *testing.T) {
	source := strings.Repeat("+", 65) + ".+."
	result := ExecuteBrainfuck(source, "")
	if result.Output != "AB" {
		t.Errorf("Expected output 'AB', got %q", result.Output)
	}
}

// TestE2EHelloWorld is the canonical Brainfuck test. If this works,
// everything works.
//
// Source: https://esolangs.org/wiki/Brainfuck
func TestE2EHelloWorld(t *testing.T) {
	helloWorld := "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]" +
		">>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."
	result := ExecuteBrainfuck(helloWorld, "")
	if result.Output != "Hello World!\n" {
		t.Errorf("Expected 'Hello World!\\n', got %q", result.Output)
	}
}

// =========================================================================
// Input Tests
// =========================================================================

// TestE2EEchoSingleChar verifies ",." reads and outputs one character.
func TestE2EEchoSingleChar(t *testing.T) {
	result := ExecuteBrainfuck(",.", "X")
	if result.Output != "X" {
		t.Errorf("Expected output 'X', got %q", result.Output)
	}
}

// TestE2ECatProgram verifies ",[.,]" — echo input until EOF.
//
// Reads a character. If nonzero, output it and read the next.
// On EOF (0), the loop exits.
func TestE2ECatProgram(t *testing.T) {
	result := ExecuteBrainfuck(",[.,]", "Hi")
	if result.Output != "Hi" {
		t.Errorf("Expected output 'Hi', got %q", result.Output)
	}
}

// TestE2EInputToCell verifies the cell holds the input byte value.
func TestE2EInputToCell(t *testing.T) {
	result := ExecuteBrainfuck(",", "A")
	if result.Tape[0] != 65 {
		t.Errorf("Expected Tape[0]=65, got %d", result.Tape[0])
	}
}

// TestE2EEOFIsZero verifies reading with no input gives 0.
func TestE2EEOFIsZero(t *testing.T) {
	result := ExecuteBrainfuck(",", "")
	if result.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0, got %d", result.Tape[0])
	}
}

// =========================================================================
// Nested Loop Tests
// =========================================================================

// TestE2ENestedMultiplication verifies 2 * 3 = 6 using nested loops.
//
// Algorithm:
//
//	cell[0] = 2, cell[1] = 3
//	Outer loop (cell 0): for each unit, copy cell[1] to cell[2] via cell[3].
//	Result: cell[2] = 6.
func TestE2ENestedMultiplication(t *testing.T) {
	source := "++>+++<[>[>+>+<<-]>>[<<+>>-]<<<-]"
	result := ExecuteBrainfuck(source, "")
	if result.Tape[2] != 6 {
		t.Errorf("Expected Tape[2]=6, got %d", result.Tape[2])
	}
}

// TestE2EDeeplyNested verifies "++[>++[>+<-]<-]" — nested decrement loops.
//
// Outer loop runs 2 times. Each time: cell[1] = 2, inner loop moves cell[1]
// to cell[2]. After 2 outer iterations: cell[2] = 2 + 2 = 4.
func TestE2EDeeplyNested(t *testing.T) {
	result := ExecuteBrainfuck("++[>++[>+<-]<-]", "")
	if result.Tape[2] != 4 {
		t.Errorf("Expected Tape[2]=4, got %d", result.Tape[2])
	}
	if result.Tape[1] != 0 {
		t.Errorf("Expected Tape[1]=0, got %d", result.Tape[1])
	}
	if result.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0, got %d", result.Tape[0])
	}
}

// =========================================================================
// BrainfuckResult Tests
// =========================================================================

// TestE2EResultFields verifies the BrainfuckResult struct fields have
// correct types (non-nil, correct lengths).
func TestE2EResultFields(t *testing.T) {
	result := ExecuteBrainfuck("+++.", "")
	if result.Output == "" && result.Tape == nil {
		t.Error("Result should have non-nil fields")
	}
	if len(result.Tape) != TapeSize {
		t.Errorf("Expected tape size %d, got %d", TapeSize, len(result.Tape))
	}
	if result.Steps <= 0 {
		t.Error("Expected positive step count")
	}
	if len(result.Traces) != result.Steps {
		t.Errorf("Expected len(Traces)==Steps, got %d vs %d", len(result.Traces), result.Steps)
	}
}

// TestE2EStepCount verifies the step count is correct.
// "+++" → 3 INCs + 1 HALT = 4 steps.
func TestE2EStepCount(t *testing.T) {
	result := ExecuteBrainfuck("+++", "")
	if result.Steps != 4 {
		t.Errorf("Expected 4 steps, got %d", result.Steps)
	}
}

// TestE2EFinalDP verifies the data pointer's final position.
// ">>>" → DP = 3.
func TestE2EFinalDP(t *testing.T) {
	result := ExecuteBrainfuck(">>>", "")
	if result.DP != 3 {
		t.Errorf("Expected DP=3, got %d", result.DP)
	}
}

// TestE2ETracesPopulated verifies that traces are generated.
// "+" → INC + HALT = 2 traces.
func TestE2ETracesPopulated(t *testing.T) {
	result := ExecuteBrainfuck("+", "")
	if len(result.Traces) != 2 {
		t.Errorf("Expected 2 traces, got %d", len(result.Traces))
	}
}

// =========================================================================
// Comment Tests
// =========================================================================

// TestE2ECommentsInCode verifies non-BF characters are ignored.
func TestE2ECommentsInCode(t *testing.T) {
	result := ExecuteBrainfuck("This is + a + program + .", "")
	if result.Tape[0] != 3 {
		t.Errorf("Expected Tape[0]=3, got %d", result.Tape[0])
	}
}

// TestE2ENumbersIgnored verifies digit characters are treated as comments.
func TestE2ENumbersIgnored(t *testing.T) {
	result := ExecuteBrainfuck("123+456", "")
	if result.Tape[0] != 1 {
		t.Errorf("Expected Tape[0]=1, got %d", result.Tape[0])
	}
}

// TestE2ENewlinesIgnored verifies newlines are treated as comments.
func TestE2ENewlinesIgnored(t *testing.T) {
	result := ExecuteBrainfuck("+\n+\n+", "")
	if result.Tape[0] != 3 {
		t.Errorf("Expected Tape[0]=3, got %d", result.Tape[0])
	}
}
