package brainfuck

// ==========================================================================
// Handler Tests — Verifying Individual Opcode Behavior
// ==========================================================================
//
// These tests mirror the Python test_handlers.py tests. Each test creates
// a minimal CodeObject with specific instructions, executes it on a
// BrainfuckVM, and verifies the resulting state.
//
// The execCode helper creates a VM, appends HALT to the given instructions,
// wraps them in a CodeObject, and runs them.

import (
	"strings"
	"testing"

	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// execCode creates a BrainfuckVM, executes the given instructions (plus
// an implicit HALT), and returns the VM for state inspection.
func execCode(instructions []vm.Instruction, inputData string) *BrainfuckVM {
	// Append HALT so the VM knows when to stop.
	instrs := make([]vm.Instruction, len(instructions))
	copy(instrs, instructions)
	instrs = append(instrs, vm.Instruction{Opcode: OpHalt, Operand: nil})

	code := vm.CodeObject{
		Instructions: instrs,
		Constants:    []interface{}{},
		Names:        []string{},
	}
	bvm := CreateBrainfuckVM(inputData)
	bvm.Execute(code)
	return bvm
}

// =========================================================================
// Pointer Movement Tests (> and <)
// =========================================================================

// TestRightMovesPointer verifies ">" increments the data pointer.
func TestRightMovesPointer(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpRight, Operand: nil},
	}, "")
	if bvm.DP != 1 {
		t.Errorf("Expected DP=1, got %d", bvm.DP)
	}
}

// TestLeftMovesPointer verifies ">" then "<" returns to position 0.
func TestLeftMovesPointer(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpRight, Operand: nil},
		{Opcode: OpLeft, Operand: nil},
	}, "")
	if bvm.DP != 0 {
		t.Errorf("Expected DP=0, got %d", bvm.DP)
	}
}

// TestMultipleRights verifies 10 ">" commands move DP to 10.
func TestMultipleRights(t *testing.T) {
	instrs := make([]vm.Instruction, 10)
	for i := range instrs {
		instrs[i] = vm.Instruction{Opcode: OpRight, Operand: nil}
	}
	bvm := execCode(instrs, "")
	if bvm.DP != 10 {
		t.Errorf("Expected DP=10, got %d", bvm.DP)
	}
}

// TestLeftAtZeroPanics verifies "<" at position 0 causes a panic.
func TestLeftAtZeroPanics(t *testing.T) {
	assertPanicsHandler(t, func() {
		execCode([]vm.Instruction{
			{Opcode: OpLeft, Operand: nil},
		}, "")
	}, "before start")
}

// TestRightPastTapePanics verifies moving past the tape end causes a panic.
func TestRightPastTapePanics(t *testing.T) {
	instrs := make([]vm.Instruction, TapeSize)
	for i := range instrs {
		instrs[i] = vm.Instruction{Opcode: OpRight, Operand: nil}
	}
	assertPanicsHandler(t, func() {
		execCode(instrs, "")
	}, "past end")
}

// =========================================================================
// Cell Modification Tests (+ and -)
// =========================================================================

// TestIncIncrementsCell verifies "+" sets cell 0 to 1.
func TestIncIncrementsCell(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpInc, Operand: nil},
	}, "")
	if bvm.Tape[0] != 1 {
		t.Errorf("Expected Tape[0]=1, got %d", bvm.Tape[0])
	}
}

// TestMultipleIncs verifies 5 "+" commands set cell 0 to 5.
func TestMultipleIncs(t *testing.T) {
	instrs := make([]vm.Instruction, 5)
	for i := range instrs {
		instrs[i] = vm.Instruction{Opcode: OpInc, Operand: nil}
	}
	bvm := execCode(instrs, "")
	if bvm.Tape[0] != 5 {
		t.Errorf("Expected Tape[0]=5, got %d", bvm.Tape[0])
	}
}

// TestDecDecrementsCell verifies "++-" gives cell value 1.
func TestDecDecrementsCell(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpInc, Operand: nil},
		{Opcode: OpInc, Operand: nil},
		{Opcode: OpDec, Operand: nil},
	}, "")
	if bvm.Tape[0] != 1 {
		t.Errorf("Expected Tape[0]=1, got %d", bvm.Tape[0])
	}
}

// TestIncWrapsAt255 verifies 256 increments wrap back to 0.
func TestIncWrapsAt255(t *testing.T) {
	instrs := make([]vm.Instruction, 256)
	for i := range instrs {
		instrs[i] = vm.Instruction{Opcode: OpInc, Operand: nil}
	}
	bvm := execCode(instrs, "")
	if bvm.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0 (wrapped), got %d", bvm.Tape[0])
	}
}

// TestDecWrapsAt0 verifies decrementing 0 gives 255.
func TestDecWrapsAt0(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpDec, Operand: nil},
	}, "")
	if bvm.Tape[0] != 255 {
		t.Errorf("Expected Tape[0]=255 (wrapped), got %d", bvm.Tape[0])
	}
}

// TestIncDifferentCell verifies incrementing a cell other than cell 0.
func TestIncDifferentCell(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpRight, Operand: nil},
		{Opcode: OpInc, Operand: nil},
		{Opcode: OpInc, Operand: nil},
	}, "")
	if bvm.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0, got %d", bvm.Tape[0])
	}
	if bvm.Tape[1] != 2 {
		t.Errorf("Expected Tape[1]=2, got %d", bvm.Tape[1])
	}
}

// =========================================================================
// Output Tests (.)
// =========================================================================

// TestOutputASCII verifies "." outputs the cell value as ASCII.
// Sets cell to 65 ('A') with 65 increments, then outputs.
func TestOutputASCII(t *testing.T) {
	instrs := make([]vm.Instruction, 65)
	for i := range instrs {
		instrs[i] = vm.Instruction{Opcode: OpInc, Operand: nil}
	}
	instrs = append(instrs, vm.Instruction{Opcode: OpOutput, Operand: nil})
	bvm := execCode(instrs, "")
	if len(bvm.Output) != 1 || bvm.Output[0] != "A" {
		t.Errorf("Expected output ['A'], got %v", bvm.Output)
	}
}

// TestOutputZero verifies outputting cell value 0 produces a null character.
func TestOutputZero(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpOutput, Operand: nil},
	}, "")
	if len(bvm.Output) != 1 || bvm.Output[0] != "\x00" {
		t.Errorf("Expected output ['\\x00'], got %v", bvm.Output)
	}
}

// TestMultipleOutputs verifies multiple "." commands accumulate output.
func TestMultipleOutputs(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpInc, Operand: nil},
		{Opcode: OpOutput, Operand: nil},
		{Opcode: OpInc, Operand: nil},
		{Opcode: OpOutput, Operand: nil},
	}, "")
	if len(bvm.Output) != 2 {
		t.Errorf("Expected 2 outputs, got %d", len(bvm.Output))
	}
}

// =========================================================================
// Input Tests (,)
// =========================================================================

// TestReadOneByte verifies "," reads the first byte of input.
func TestReadOneByte(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpInput, Operand: nil},
	}, "A")
	if bvm.Tape[0] != 65 {
		t.Errorf("Expected Tape[0]=65 (ord 'A'), got %d", bvm.Tape[0])
	}
}

// TestReadMultipleBytes verifies sequential "," reads consume input in order.
func TestReadMultipleBytes(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpInput, Operand: nil},
		{Opcode: OpRight, Operand: nil},
		{Opcode: OpInput, Operand: nil},
	}, "AB")
	if bvm.Tape[0] != 65 {
		t.Errorf("Expected Tape[0]=65, got %d", bvm.Tape[0])
	}
	if bvm.Tape[1] != 66 {
		t.Errorf("Expected Tape[1]=66, got %d", bvm.Tape[1])
	}
}

// TestEOFGivesZero verifies reading with no input gives 0.
func TestEOFGivesZero(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpInput, Operand: nil},
	}, "")
	if bvm.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0 (EOF), got %d", bvm.Tape[0])
	}
}

// TestEOFAfterInput verifies that reading past the end of input gives 0.
func TestEOFAfterInput(t *testing.T) {
	bvm := execCode([]vm.Instruction{
		{Opcode: OpInput, Operand: nil},
		{Opcode: OpRight, Operand: nil},
		{Opcode: OpInput, Operand: nil},
	}, "X")
	if bvm.Tape[0] != byte('X') {
		t.Errorf("Expected Tape[0]=%d, got %d", 'X', bvm.Tape[0])
	}
	if bvm.Tape[1] != 0 {
		t.Errorf("Expected Tape[1]=0 (EOF), got %d", bvm.Tape[1])
	}
}

// =========================================================================
// Control Flow Tests ([ and ])
// =========================================================================

// TestSkipLoopWhenZero verifies that "[..]" is skipped when cell is 0.
func TestSkipLoopWhenZero(t *testing.T) {
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: OpLoopStart, Operand: 3}, // skip to index 3
			{Opcode: OpInc, Operand: nil},      // should be skipped
			{Opcode: OpLoopEnd, Operand: 0},    // should be skipped
			{Opcode: OpHalt, Operand: nil},
		},
		Constants: []interface{}{},
		Names:     []string{},
	}
	bvm := CreateBrainfuckVM("")
	bvm.Execute(code)
	if bvm.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0 (INC was skipped), got %d", bvm.Tape[0])
	}
}

// TestEnterLoopWhenNonzero verifies the loop body executes when cell != 0.
func TestEnterLoopWhenNonzero(t *testing.T) {
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: OpInc, Operand: nil},       // cell = 1
			{Opcode: OpLoopStart, Operand: 5},   // cell != 0, enter loop
			{Opcode: OpDec, Operand: nil},        // cell = 0
			{Opcode: OpRight, Operand: nil},      // dp = 1
			{Opcode: OpLoopEnd, Operand: 1},      // cell[1] == 0, exit
			{Opcode: OpHalt, Operand: nil},
		},
		Constants: []interface{}{},
		Names:     []string{},
	}
	bvm := CreateBrainfuckVM("")
	bvm.Execute(code)
	if bvm.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0, got %d", bvm.Tape[0])
	}
	if bvm.DP != 1 {
		t.Errorf("Expected DP=1, got %d", bvm.DP)
	}
}

// TestLoopRepeats verifies that a loop repeats until the cell becomes 0.
// Sets cell to 3, then uses [>+<-] to move the value to cell 1.
func TestLoopRepeats(t *testing.T) {
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: OpInc, Operand: nil},       // cell[0] = 1
			{Opcode: OpInc, Operand: nil},       // cell[0] = 2
			{Opcode: OpInc, Operand: nil},       // cell[0] = 3
			{Opcode: OpLoopStart, Operand: 8},   // [
			{Opcode: OpRight, Operand: nil},     // dp = 1
			{Opcode: OpInc, Operand: nil},       // cell[1]++
			{Opcode: OpLeft, Operand: nil},      // dp = 0
			{Opcode: OpDec, Operand: nil},       // cell[0]--
			{Opcode: OpLoopEnd, Operand: 3},     // ]
			{Opcode: OpHalt, Operand: nil},
		},
		Constants: []interface{}{},
		Names:     []string{},
	}
	bvm := CreateBrainfuckVM("")
	bvm.Execute(code)
	if bvm.Tape[0] != 0 {
		t.Errorf("Expected Tape[0]=0, got %d", bvm.Tape[0])
	}
	if bvm.Tape[1] != 3 {
		t.Errorf("Expected Tape[1]=3, got %d", bvm.Tape[1])
	}
}

// =========================================================================
// VM State Tests
// =========================================================================

// TestTapeSize verifies the tape has TapeSize cells.
func TestTapeSize(t *testing.T) {
	bvm := CreateBrainfuckVM("")
	if len(bvm.Tape) != TapeSize {
		t.Errorf("Expected tape size %d, got %d", TapeSize, len(bvm.Tape))
	}
}

// TestTapeInitializedToZero verifies all cells start at 0.
func TestTapeInitializedToZero(t *testing.T) {
	bvm := CreateBrainfuckVM("")
	for i, v := range bvm.Tape {
		if v != 0 {
			t.Errorf("Expected Tape[%d]=0, got %d", i, v)
			break
		}
	}
}

// TestDPStartsAtZero verifies the data pointer starts at 0.
func TestDPStartsAtZero(t *testing.T) {
	bvm := CreateBrainfuckVM("")
	if bvm.DP != 0 {
		t.Errorf("Expected DP=0, got %d", bvm.DP)
	}
}

// TestInputBufferSet verifies the input buffer and position are initialized.
func TestInputBufferSet(t *testing.T) {
	bvm := CreateBrainfuckVM("hello")
	if bvm.InputBuffer != "hello" {
		t.Errorf("Expected InputBuffer='hello', got %q", bvm.InputBuffer)
	}
	if bvm.InputPos != 0 {
		t.Errorf("Expected InputPos=0, got %d", bvm.InputPos)
	}
}

// =========================================================================
// Test helper
// =========================================================================

// assertPanicsHandler checks that fn panics with a message containing substr.
func assertPanicsHandler(t *testing.T, fn func(), substr string) {
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
		}
		if !strings.Contains(msg, substr) {
			t.Errorf("Expected panic containing %q, got %q", substr, msg)
		}
	}()
	fn()
}
