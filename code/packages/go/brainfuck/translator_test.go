package brainfuck

// ==========================================================================
// Translator Tests — Verifying Source-to-Bytecode Translation
// ==========================================================================
//
// These tests mirror the Python test_translator.py tests. They verify:
//
//  1. Basic translation: each BF character maps to one instruction.
//  2. Bracket matching: "[" and "]" are connected with correct operands.
//  3. Error handling: mismatched brackets cause panics.

import (
	"strings"
	"testing"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// =========================================================================
// Basic Translation Tests
// =========================================================================

// TestEmptyProgram verifies that an empty source produces only a HALT.
func TestEmptyProgram(t *testing.T) {
	code := Translate("")
	if len(code.Instructions) != 1 {
		t.Fatalf("Expected 1 instruction (HALT), got %d", len(code.Instructions))
	}
	if code.Instructions[0].Opcode != OpHalt {
		t.Errorf("Expected HALT, got opcode %d", code.Instructions[0].Opcode)
	}
}

// TestSingleRight verifies ">" translates to OpRight + HALT.
func TestSingleRight(t *testing.T) {
	code := Translate(">")
	assertOpcode(t, code.Instructions[0], OpRight)
	assertOpcode(t, code.Instructions[1], OpHalt)
}

// TestSingleLeft verifies "<" translates to OpLeft.
func TestSingleLeft(t *testing.T) {
	code := Translate("<")
	assertOpcode(t, code.Instructions[0], OpLeft)
}

// TestSingleInc verifies "+" translates to OpInc.
func TestSingleInc(t *testing.T) {
	code := Translate("+")
	assertOpcode(t, code.Instructions[0], OpInc)
}

// TestSingleDec verifies "-" translates to OpDec.
func TestSingleDec(t *testing.T) {
	code := Translate("-")
	assertOpcode(t, code.Instructions[0], OpDec)
}

// TestSingleOutput verifies "." translates to OpOutput.
func TestSingleOutput(t *testing.T) {
	code := Translate(".")
	assertOpcode(t, code.Instructions[0], OpOutput)
}

// TestSingleInput verifies "," translates to OpInput.
func TestSingleInput(t *testing.T) {
	code := Translate(",")
	assertOpcode(t, code.Instructions[0], OpInput)
}

// TestMultipleCommands verifies a sequence of commands.
func TestMultipleCommands(t *testing.T) {
	code := Translate("+++>.")
	expected := []vm.OpCode{OpInc, OpInc, OpInc, OpRight, OpOutput, OpHalt}
	if len(code.Instructions) != len(expected) {
		t.Fatalf("Expected %d instructions, got %d", len(expected), len(code.Instructions))
	}
	for i, exp := range expected {
		assertOpcode(t, code.Instructions[i], exp)
	}
}

// TestCommentsIgnored verifies that non-BF characters are skipped.
func TestCommentsIgnored(t *testing.T) {
	code := Translate("hello + world - !")
	expected := []vm.OpCode{OpInc, OpDec, OpHalt}
	if len(code.Instructions) != len(expected) {
		t.Fatalf("Expected %d instructions, got %d", len(expected), len(code.Instructions))
	}
	for i, exp := range expected {
		assertOpcode(t, code.Instructions[i], exp)
	}
}

// TestWhitespaceIgnored verifies that whitespace is treated as comments.
func TestWhitespaceIgnored(t *testing.T) {
	code := Translate("  +  +  +  ")
	expected := []vm.OpCode{OpInc, OpInc, OpInc, OpHalt}
	if len(code.Instructions) != len(expected) {
		t.Fatalf("Expected %d instructions, got %d", len(expected), len(code.Instructions))
	}
	for i, exp := range expected {
		assertOpcode(t, code.Instructions[i], exp)
	}
}

// TestEmptyConstantPool verifies that the constant pool is always empty.
func TestEmptyConstantPool(t *testing.T) {
	code := Translate("+++")
	if len(code.Constants) != 0 {
		t.Errorf("Expected empty constants, got %v", code.Constants)
	}
}

// TestEmptyNamePool verifies that the name pool is always empty.
func TestEmptyNamePool(t *testing.T) {
	code := Translate("+++")
	if len(code.Names) != 0 {
		t.Errorf("Expected empty names, got %v", code.Names)
	}
}

// =========================================================================
// Bracket Matching Tests
// =========================================================================

// TestSimpleLoop verifies "[>+<-]" bracket matching.
//
// Instructions: LOOP_START, RIGHT, INC, LEFT, DEC, LOOP_END, HALT
//
// LOOP_START at index 0 should jump to index 6 (past LOOP_END at 5).
// LOOP_END at index 5 should jump back to index 0.
func TestSimpleLoop(t *testing.T) {
	code := Translate("[>+<-]")
	if len(code.Instructions) != 7 {
		t.Fatalf("Expected 7 instructions, got %d", len(code.Instructions))
	}

	loopStart := code.Instructions[0]
	loopEnd := code.Instructions[5]

	assertOpcode(t, loopStart, OpLoopStart)
	if loopStart.Operand.(int) != 6 {
		t.Errorf("LOOP_START operand: expected 6, got %v", loopStart.Operand)
	}

	assertOpcode(t, loopEnd, OpLoopEnd)
	if loopEnd.Operand.(int) != 0 {
		t.Errorf("LOOP_END operand: expected 0, got %v", loopEnd.Operand)
	}
}

// TestNestedLoops verifies "++[>++[>+<-]<-]" — outer and inner brackets.
//
// Instruction layout:
//
//	0:INC 1:INC 2:LOOP_START(15) 3:RIGHT 4:INC 5:INC 6:LOOP_START(12)
//	7:RIGHT 8:INC 9:LEFT 10:DEC 11:LOOP_END(6) 12:LEFT 13:DEC
//	14:LOOP_END(2) 15:HALT
func TestNestedLoops(t *testing.T) {
	code := Translate("++[>++[>+<-]<-]")

	// Inner loop: "[" at 6 jumps to 12, "]" at 11 jumps back to 6.
	innerStart := code.Instructions[6]
	innerEnd := code.Instructions[11]
	if innerStart.Operand.(int) != 12 {
		t.Errorf("Inner LOOP_START operand: expected 12, got %v", innerStart.Operand)
	}
	if innerEnd.Operand.(int) != 6 {
		t.Errorf("Inner LOOP_END operand: expected 6, got %v", innerEnd.Operand)
	}

	// Outer loop: "[" at 2 jumps to 15, "]" at 14 jumps back to 2.
	outerStart := code.Instructions[2]
	outerEnd := code.Instructions[14]
	if outerStart.Operand.(int) != 15 {
		t.Errorf("Outer LOOP_START operand: expected 15, got %v", outerStart.Operand)
	}
	if outerEnd.Operand.(int) != 2 {
		t.Errorf("Outer LOOP_END operand: expected 2, got %v", outerEnd.Operand)
	}
}

// TestEmptyLoop verifies "[]" — an empty loop (infinite if cell != 0).
func TestEmptyLoop(t *testing.T) {
	code := Translate("[]")
	assertOpcode(t, code.Instructions[0], OpLoopStart)
	if code.Instructions[0].Operand.(int) != 2 {
		t.Errorf("Expected LOOP_START operand 2, got %v", code.Instructions[0].Operand)
	}
	assertOpcode(t, code.Instructions[1], OpLoopEnd)
	if code.Instructions[1].Operand.(int) != 0 {
		t.Errorf("Expected LOOP_END operand 0, got %v", code.Instructions[1].Operand)
	}
}

// TestAdjacentLoops verifies "[][]" — two loops side by side.
func TestAdjacentLoops(t *testing.T) {
	code := Translate("[][]")
	// First loop: [0]→LOOP_START(2), [1]→LOOP_END(0)
	if code.Instructions[0].Operand.(int) != 2 {
		t.Errorf("First LOOP_START operand: expected 2, got %v", code.Instructions[0].Operand)
	}
	if code.Instructions[1].Operand.(int) != 0 {
		t.Errorf("First LOOP_END operand: expected 0, got %v", code.Instructions[1].Operand)
	}
	// Second loop: [2]→LOOP_START(4), [3]→LOOP_END(2)
	if code.Instructions[2].Operand.(int) != 4 {
		t.Errorf("Second LOOP_START operand: expected 4, got %v", code.Instructions[2].Operand)
	}
	if code.Instructions[3].Operand.(int) != 2 {
		t.Errorf("Second LOOP_END operand: expected 2, got %v", code.Instructions[3].Operand)
	}
}

// =========================================================================
// Bracket Error Tests
// =========================================================================

// TestUnmatchedOpenBracket verifies that "[" alone panics.
func TestUnmatchedOpenBracket(t *testing.T) {
	assertPanics(t, func() { Translate("[") }, "Unmatched '['")
}

// TestUnmatchedCloseBracket verifies that "]" alone panics.
func TestUnmatchedCloseBracket(t *testing.T) {
	assertPanics(t, func() { Translate("]") }, "Unmatched ']'")
}

// TestExtraOpenBracket verifies that "[[]" panics.
func TestExtraOpenBracket(t *testing.T) {
	assertPanics(t, func() { Translate("[[]") }, "Unmatched '['")
}

// TestExtraCloseBracket verifies that "[]]" panics.
func TestExtraCloseBracket(t *testing.T) {
	assertPanics(t, func() { Translate("[]]") }, "Unmatched ']'")
}

// TestMultipleUnmatched verifies that "[[" reports 2 unclosed brackets.
func TestMultipleUnmatched(t *testing.T) {
	assertPanics(t, func() { Translate("[[") }, "2 unclosed")
}

// =========================================================================
// Test helpers
// =========================================================================

// assertOpcode checks that an instruction has the expected opcode.
func assertOpcode(t *testing.T, instr vm.Instruction, expected vm.OpCode) {
	t.Helper()
	if instr.Opcode != expected {
		t.Errorf("Expected opcode %d, got %d", expected, instr.Opcode)
	}
}

// assertPanics checks that fn panics with a message containing substr.
func assertPanics(t *testing.T, fn func(), substr string) {
	t.Helper()
	defer func() {
		r := recover()
		if r == nil {
			t.Errorf("Expected panic containing %q, but no panic occurred", substr)
			return
		}
		msg := ""
		switch v := r.(type) {
		case string:
			msg = v
		case error:
			msg = v.Error()
		default:
			msg = ""
		}
		if !strings.Contains(msg, substr) {
			t.Errorf("Expected panic containing %q, got %q", substr, msg)
		}
	}()
	fn()
}
