// compiler_test.go — Tests for the Starlark AST-to-bytecode compiler.
//
// These tests verify that Starlark source code is correctly compiled into
// bytecode instructions. Each test compiles a source string and checks
// that the resulting CodeObject has the expected instructions, constants,
// and names.
//
// The tests are organized by language feature, progressing from simple
// to complex:
//   1. Basic assignment and arithmetic
//   2. Comparison and boolean operators
//   3. Control flow (if/else, for)
//   4. Functions
//   5. Collections (list, dict, tuple)
//   6. Advanced features (load, lambda, etc.)
//
package starlarkcompiler

import (
	"testing"

	starlarkparser "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-parser"
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// ════════════════════════════════════════════════════════════════════════
// TEST HELPERS
// ════════════════════════════════════════════════════════════════════════

// compile is a test helper that compiles Starlark source code and fails
// the test if compilation produces an error.
func compile(t *testing.T, source string) vm.CodeObject {
	t.Helper()
	code, err := CompileStarlark(source)
	if err != nil {
		t.Fatalf("compile error: %v", err)
	}
	return code
}

// assertOpcode checks that the instruction at the given index has the expected opcode.
func assertOpcode(t *testing.T, code vm.CodeObject, index int, expected vm.OpCode) {
	t.Helper()
	if index >= len(code.Instructions) {
		t.Fatalf("instruction index %d out of range (only %d instructions)", index, len(code.Instructions))
	}
	actual := code.Instructions[index].Opcode
	if actual != expected {
		expectedName := OpcodeName[expected]
		actualName := OpcodeName[actual]
		t.Errorf("instruction[%d]: expected %s (0x%02x), got %s (0x%02x)\nDisassembly:\n%s",
			index, expectedName, int(expected), actualName, int(actual), Disassemble(code))
	}
}

// assertOperand checks that the instruction at the given index has the expected operand.
func assertOperand(t *testing.T, code vm.CodeObject, index int, expected interface{}) {
	t.Helper()
	if index >= len(code.Instructions) {
		t.Fatalf("instruction index %d out of range (only %d instructions)", index, len(code.Instructions))
	}
	actual := code.Instructions[index].Operand
	if actual != expected {
		t.Errorf("instruction[%d] operand: expected %v, got %v\nDisassembly:\n%s",
			index, expected, actual, Disassemble(code))
	}
}

// assertConstant checks that the constants pool contains the expected value at the given index.
func assertConstant(t *testing.T, code vm.CodeObject, index int, expected interface{}) {
	t.Helper()
	if index >= len(code.Constants) {
		t.Fatalf("constant index %d out of range (only %d constants)", index, len(code.Constants))
	}
	actual := code.Constants[index]
	if actual != expected {
		t.Errorf("constants[%d]: expected %v (%T), got %v (%T)", index, expected, expected, actual, actual)
	}
}

// assertName checks that the names table contains the expected value at the given index.
func assertName(t *testing.T, code vm.CodeObject, index int, expected string) {
	t.Helper()
	if index >= len(code.Names) {
		t.Fatalf("name index %d out of range (only %d names)", index, len(code.Names))
	}
	if code.Names[index] != expected {
		t.Errorf("names[%d]: expected %q, got %q", index, expected, code.Names[index])
	}
}

// hasOpcodeInRange checks if any instruction in the given range has the specified opcode.
func hasOpcodeInRange(code vm.CodeObject, start, end int, opcode vm.OpCode) bool {
	for i := start; i < end && i < len(code.Instructions); i++ {
		if code.Instructions[i].Opcode == opcode {
			return true
		}
	}
	return false
}

// ════════════════════════════════════════════════════════════════════════
// BASIC ASSIGNMENT TESTS
// ════════════════════════════════════════════════════════════════════════

func TestSimpleAssignment(t *testing.T) {
	// x = 42
	// Expected: LOAD_CONST 0, STORE_NAME 0, HALT
	code := compile(t, "x = 42\n")

	assertOpcode(t, code, 0, OpLoadConst)
	assertOperand(t, code, 0, 0)
	assertOpcode(t, code, 1, OpStoreName)
	assertOperand(t, code, 1, 0)
	assertOpcode(t, code, 2, OpHalt)

	assertConstant(t, code, 0, 42)
	assertName(t, code, 0, "x")
}

func TestStringAssignment(t *testing.T) {
	// name = "hello"
	code := compile(t, `name = "hello"`+"\n")

	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpStoreName)
	assertOpcode(t, code, 2, OpHalt)

	assertConstant(t, code, 0, "hello")
	assertName(t, code, 0, "name")
}

func TestMultipleAssignments(t *testing.T) {
	// x = 1
	// y = 2
	code := compile(t, "x = 1\ny = 2\n")

	// x = 1: LOAD_CONST 0, STORE_NAME 0
	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpStoreName)
	// y = 2: LOAD_CONST 1, STORE_NAME 1
	assertOpcode(t, code, 2, OpLoadConst)
	assertOpcode(t, code, 3, OpStoreName)
	// HALT
	assertOpcode(t, code, 4, OpHalt)

	assertConstant(t, code, 0, 1)
	assertConstant(t, code, 1, 2)
	assertName(t, code, 0, "x")
	assertName(t, code, 1, "y")
}

// ════════════════════════════════════════════════════════════════════════
// ARITHMETIC TESTS
// ════════════════════════════════════════════════════════════════════════

func TestAddition(t *testing.T) {
	// x = 1 + 2
	// Expected: LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT
	code := compile(t, "x = 1 + 2\n")

	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpLoadConst)
	assertOpcode(t, code, 2, OpAdd)
	assertOpcode(t, code, 3, OpStoreName)
	assertOpcode(t, code, 4, OpHalt)

	assertConstant(t, code, 0, 1)
	assertConstant(t, code, 1, 2)
}

func TestSubtraction(t *testing.T) {
	code := compile(t, "x = 10 - 3\n")
	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpLoadConst)
	assertOpcode(t, code, 2, OpSub)
}

func TestMultiplication(t *testing.T) {
	code := compile(t, "x = 3 * 4\n")
	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpLoadConst)
	assertOpcode(t, code, 2, OpMul)
}

func TestDivision(t *testing.T) {
	code := compile(t, "x = 10 / 3\n")
	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpLoadConst)
	assertOpcode(t, code, 2, OpDiv)
}

func TestFloorDivision(t *testing.T) {
	code := compile(t, "x = 7 // 2\n")
	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpLoadConst)
	assertOpcode(t, code, 2, OpFloorDiv)
}

func TestModulo(t *testing.T) {
	code := compile(t, "x = 7 % 3\n")
	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpLoadConst)
	assertOpcode(t, code, 2, OpMod)
}

func TestExponentiation(t *testing.T) {
	code := compile(t, "x = 2 ** 10\n")
	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpLoadConst)
	assertOpcode(t, code, 2, OpPower)
}

func TestChainedArithmetic(t *testing.T) {
	// x = 1 + 2 + 3
	// Expected: LOAD 1, LOAD 2, ADD, LOAD 3, ADD, STORE_NAME, HALT
	code := compile(t, "x = 1 + 2 + 3\n")

	assertOpcode(t, code, 0, OpLoadConst) // 1
	assertOpcode(t, code, 1, OpLoadConst) // 2
	assertOpcode(t, code, 2, OpAdd)       // 1 + 2
	assertOpcode(t, code, 3, OpLoadConst) // 3
	assertOpcode(t, code, 4, OpAdd)       // (1+2) + 3
	assertOpcode(t, code, 5, OpStoreName) // x = ...
}

// ════════════════════════════════════════════════════════════════════════
// UNARY OPERATOR TESTS
// ════════════════════════════════════════════════════════════════════════

func TestUnaryNegation(t *testing.T) {
	code := compile(t, "x = -5\n")

	// Should have LOAD_CONST 5, NEGATE, STORE_NAME
	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpNegate {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected NEGATE opcode in output\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestBitwiseNot(t *testing.T) {
	code := compile(t, "x = ~0\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBitNot {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected BIT_NOT opcode in output\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestLogicalNot(t *testing.T) {
	code := compile(t, "x = not True\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpNot {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected NOT opcode in output\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// BOOLEAN LITERAL TESTS
// ════════════════════════════════════════════════════════════════════════

func TestBooleanTrue(t *testing.T) {
	code := compile(t, "x = True\n")
	assertOpcode(t, code, 0, OpLoadTrue)
	assertOpcode(t, code, 1, OpStoreName)
}

func TestBooleanFalse(t *testing.T) {
	code := compile(t, "x = False\n")
	assertOpcode(t, code, 0, OpLoadFalse)
	assertOpcode(t, code, 1, OpStoreName)
}

func TestNone(t *testing.T) {
	code := compile(t, "x = None\n")
	assertOpcode(t, code, 0, OpLoadNone)
	assertOpcode(t, code, 1, OpStoreName)
}

// ════════════════════════════════════════════════════════════════════════
// COMPARISON TESTS
// ════════════════════════════════════════════════════════════════════════

func TestComparisonEqual(t *testing.T) {
	code := compile(t, "x = 1 == 2\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCmpEq {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected CMP_EQ opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestComparisonNotEqual(t *testing.T) {
	code := compile(t, "x = 1 != 2\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCmpNe {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected CMP_NE opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestComparisonLessThan(t *testing.T) {
	code := compile(t, "x = 1 < 2\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCmpLt {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected CMP_LT opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestComparisonGreaterThan(t *testing.T) {
	code := compile(t, "x = 1 > 2\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCmpGt {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected CMP_GT opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestComparisonLessEqual(t *testing.T) {
	code := compile(t, "x = 1 <= 2\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCmpLe {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected CMP_LE opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestComparisonGreaterEqual(t *testing.T) {
	code := compile(t, "x = 1 >= 2\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCmpGe {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected CMP_GE opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestComparisonIn(t *testing.T) {
	code := compile(t, "x = 1 in [1, 2]\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCmpIn {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected CMP_IN opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestComparisonNotIn(t *testing.T) {
	code := compile(t, "x = 3 not in [1, 2]\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCmpNotIn {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected CMP_NOT_IN opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// BOOLEAN OPERATOR TESTS (SHORT-CIRCUIT)
// ════════════════════════════════════════════════════════════════════════

func TestBooleanOr(t *testing.T) {
	// x = a or b
	// Should use JUMP_IF_TRUE_OR_POP for short-circuit
	code := compile(t, "x = a or b\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpJumpIfTrueOrPop {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected JUMP_IF_TRUE_OR_POP opcode for 'or'\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestBooleanAnd(t *testing.T) {
	// x = a and b
	// Should use JUMP_IF_FALSE_OR_POP for short-circuit
	code := compile(t, "x = a and b\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpJumpIfFalseOrPop {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected JUMP_IF_FALSE_OR_POP opcode for 'and'\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// IF/ELSE TESTS
// ════════════════════════════════════════════════════════════════════════

func TestIfStatement(t *testing.T) {
	source := "if True:\n    x = 1\n"
	code := compile(t, source)

	// Should have: LOAD_TRUE, JUMP_IF_FALSE, ..., HALT
	assertOpcode(t, code, 0, OpLoadTrue)
	assertOpcode(t, code, 1, OpJumpIfFalse)

	// The body should contain LOAD_CONST + STORE_NAME
	found := hasOpcodeInRange(code, 2, len(code.Instructions)-1, OpStoreName)
	if !found {
		t.Errorf("expected STORE_NAME in if body\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestIfElseStatement(t *testing.T) {
	source := "if True:\n    x = 1\nelse:\n    x = 2\n"
	code := compile(t, source)

	// Should have JUMP_IF_FALSE and JUMP (to skip else)
	foundJIF := false
	foundJMP := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpJumpIfFalse {
			foundJIF = true
		}
		if instr.Opcode == OpJump {
			foundJMP = true
		}
	}
	if !foundJIF {
		t.Errorf("expected JUMP_IF_FALSE opcode\nDisassembly:\n%s", Disassemble(code))
	}
	if !foundJMP {
		t.Errorf("expected JUMP opcode for else branch\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// FOR LOOP TESTS
// ════════════════════════════════════════════════════════════════════════

func TestForLoop(t *testing.T) {
	source := "for x in [1, 2, 3]:\n    pass\n"
	code := compile(t, source)

	// Should have GET_ITER, FOR_ITER, JUMP
	foundGetIter := false
	foundForIter := false
	foundJump := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpGetIter {
			foundGetIter = true
		}
		if instr.Opcode == OpForIter {
			foundForIter = true
		}
		if instr.Opcode == OpJump {
			foundJump = true
		}
	}
	if !foundGetIter {
		t.Errorf("expected GET_ITER opcode\nDisassembly:\n%s", Disassemble(code))
	}
	if !foundForIter {
		t.Errorf("expected FOR_ITER opcode\nDisassembly:\n%s", Disassemble(code))
	}
	if !foundJump {
		t.Errorf("expected JUMP opcode for loop back\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// FUNCTION DEFINITION TESTS
// ════════════════════════════════════════════════════════════════════════

func TestSimpleFunction(t *testing.T) {
	source := "def f():\n    return 1\n"
	code := compile(t, source)

	// Should have MAKE_FUNCTION and STORE_NAME
	foundMakeFunc := false
	foundStoreName := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpMakeFunction {
			foundMakeFunc = true
		}
		if instr.Opcode == OpStoreName {
			foundStoreName = true
		}
	}
	if !foundMakeFunc {
		t.Errorf("expected MAKE_FUNCTION opcode\nDisassembly:\n%s", Disassemble(code))
	}
	if !foundStoreName {
		t.Errorf("expected STORE_NAME opcode\nDisassembly:\n%s", Disassemble(code))
	}

	// The function name should be in the names table
	assertName(t, code, 0, "f")
}

func TestFunctionWithParams(t *testing.T) {
	source := "def add(a, b):\n    return a + b\n"
	code := compile(t, source)

	foundMakeFunc := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpMakeFunction {
			foundMakeFunc = true
		}
	}
	if !foundMakeFunc {
		t.Errorf("expected MAKE_FUNCTION opcode\nDisassembly:\n%s", Disassemble(code))
	}

	// Check that the nested CodeObject exists in constants
	if len(code.Constants) == 0 {
		t.Fatal("expected at least one constant (the function info)")
	}

	// The constant should be a map containing "code", "params", "default_count"
	funcInfo, ok := code.Constants[0].(map[string]interface{})
	if !ok {
		t.Fatalf("expected function info map, got %T", code.Constants[0])
	}

	params, ok := funcInfo["params"].([]string)
	if !ok {
		t.Fatalf("expected params []string, got %T", funcInfo["params"])
	}
	if len(params) != 2 || params[0] != "a" || params[1] != "b" {
		t.Errorf("expected params [a, b], got %v", params)
	}

	// The nested CodeObject should contain ADD and RETURN_VALUE
	bodyCode, ok := funcInfo["code"].(vm.CodeObject)
	if !ok {
		t.Fatalf("expected CodeObject, got %T", funcInfo["code"])
	}

	foundAdd := false
	foundReturn := false
	for _, instr := range bodyCode.Instructions {
		if instr.Opcode == OpAdd {
			foundAdd = true
		}
		if instr.Opcode == OpReturnValue {
			foundReturn = true
		}
	}
	if !foundAdd {
		t.Errorf("expected ADD in function body\nBody disassembly:\n%s", Disassemble(bodyCode))
	}
	if !foundReturn {
		t.Errorf("expected RETURN_VALUE in function body\nBody disassembly:\n%s", Disassemble(bodyCode))
	}
}

// ════════════════════════════════════════════════════════════════════════
// FUNCTION CALL TESTS
// ════════════════════════════════════════════════════════════════════════

func TestFunctionCallNoArgs(t *testing.T) {
	code := compile(t, "f()\n")

	// f() is an expression statement -> compile, then POP
	foundCall := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCallFunction {
			foundCall = true
			if instr.Operand != 0 {
				t.Errorf("expected 0 args, got %v", instr.Operand)
			}
		}
	}
	if !foundCall {
		t.Errorf("expected CALL_FUNCTION opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestFunctionCallWithArgs(t *testing.T) {
	code := compile(t, "f(1, 2)\n")

	foundCall := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCallFunction {
			foundCall = true
			if instr.Operand != 2 {
				t.Errorf("expected 2 args, got %v", instr.Operand)
			}
		}
	}
	if !foundCall {
		t.Errorf("expected CALL_FUNCTION opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestFunctionCallWithKWArgs(t *testing.T) {
	code := compile(t, "f(x=1, y=2)\n")

	foundCallKW := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpCallFunctionKW {
			foundCallKW = true
		}
	}
	if !foundCallKW {
		t.Errorf("expected CALL_FUNCTION_KW opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// COLLECTION TESTS
// ════════════════════════════════════════════════════════════════════════

func TestEmptyList(t *testing.T) {
	code := compile(t, "x = []\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBuildList {
			found = true
			if instr.Operand != 0 {
				t.Errorf("expected BUILD_LIST 0, got BUILD_LIST %v", instr.Operand)
			}
		}
	}
	if !found {
		t.Errorf("expected BUILD_LIST opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestListLiteral(t *testing.T) {
	code := compile(t, "x = [1, 2, 3]\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBuildList {
			found = true
			if instr.Operand != 3 {
				t.Errorf("expected BUILD_LIST 3, got BUILD_LIST %v", instr.Operand)
			}
		}
	}
	if !found {
		t.Errorf("expected BUILD_LIST opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestEmptyDict(t *testing.T) {
	code := compile(t, "x = {}\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBuildDict {
			found = true
			if instr.Operand != 0 {
				t.Errorf("expected BUILD_DICT 0, got BUILD_DICT %v", instr.Operand)
			}
		}
	}
	if !found {
		t.Errorf("expected BUILD_DICT opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestDictLiteral(t *testing.T) {
	code := compile(t, `x = {"a": 1, "b": 2}`+"\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBuildDict {
			found = true
			if instr.Operand != 2 {
				t.Errorf("expected BUILD_DICT 2, got BUILD_DICT %v", instr.Operand)
			}
		}
	}
	if !found {
		t.Errorf("expected BUILD_DICT opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestEmptyTuple(t *testing.T) {
	code := compile(t, "x = ()\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBuildTuple {
			found = true
			if instr.Operand != 0 {
				t.Errorf("expected BUILD_TUPLE 0, got BUILD_TUPLE %v", instr.Operand)
			}
		}
	}
	if !found {
		t.Errorf("expected BUILD_TUPLE opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestTupleLiteral(t *testing.T) {
	code := compile(t, "x = (1, 2)\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBuildTuple {
			found = true
		}
	}
	if !found {
		t.Errorf("expected BUILD_TUPLE opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// ATTRIBUTE ACCESS AND SUBSCRIPT TESTS
// ════════════════════════════════════════════════════════════════════════

func TestAttributeAccess(t *testing.T) {
	code := compile(t, "x = obj.attr\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpLoadAttr {
			found = true
		}
	}
	if !found {
		t.Errorf("expected LOAD_ATTR opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestSubscript(t *testing.T) {
	code := compile(t, "x = lst[0]\n")

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpLoadSubscript {
			found = true
		}
	}
	if !found {
		t.Errorf("expected LOAD_SUBSCRIPT opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// LOAD STATEMENT TESTS
// ════════════════════════════════════════════════════════════════════════

func TestLoadStatement(t *testing.T) {
	code := compile(t, `load("module.star", "symbol")`+"\n")

	foundLoadModule := false
	foundImportFrom := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpLoadModule {
			foundLoadModule = true
		}
		if instr.Opcode == OpImportFrom {
			foundImportFrom = true
		}
	}
	if !foundLoadModule {
		t.Errorf("expected LOAD_MODULE opcode\nDisassembly:\n%s", Disassemble(code))
	}
	if !foundImportFrom {
		t.Errorf("expected IMPORT_FROM opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// PASS, BREAK, CONTINUE TESTS
// ════════════════════════════════════════════════════════════════════════

func TestPassStatement(t *testing.T) {
	code := compile(t, "pass\n")
	// pass is a no-op, so the only instruction should be HALT
	assertOpcode(t, code, 0, OpHalt)
}

func TestBreakStatement(t *testing.T) {
	source := "for x in [1]:\n    break\n"
	code := compile(t, source)

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBreak {
			found = true
		}
	}
	if !found {
		t.Errorf("expected BREAK opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestContinueStatement(t *testing.T) {
	source := "for x in [1]:\n    continue\n"
	code := compile(t, source)

	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpContinue {
			found = true
		}
	}
	if !found {
		t.Errorf("expected CONTINUE opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// RETURN STATEMENT TESTS
// ════════════════════════════════════════════════════════════════════════

func TestReturnNone(t *testing.T) {
	source := "def f():\n    return\n"
	code := compile(t, source)

	// The function body should have LOAD_NONE, RETURN_VALUE
	funcInfo, ok := code.Constants[0].(map[string]interface{})
	if !ok {
		t.Fatalf("expected function info map, got %T", code.Constants[0])
	}
	bodyCode := funcInfo["code"].(vm.CodeObject)

	// First return should be LOAD_NONE + RETURN_VALUE (from the explicit return)
	assertOpcode(t, bodyCode, 0, OpLoadNone)
	assertOpcode(t, bodyCode, 1, OpReturnValue)
}

func TestReturnValue(t *testing.T) {
	source := "def f():\n    return 42\n"
	code := compile(t, source)

	funcInfo := code.Constants[0].(map[string]interface{})
	bodyCode := funcInfo["code"].(vm.CodeObject)

	// Should have LOAD_CONST (42), RETURN_VALUE
	assertOpcode(t, bodyCode, 0, OpLoadConst)
	assertOpcode(t, bodyCode, 1, OpReturnValue)
}

// ════════════════════════════════════════════════════════════════════════
// BITWISE OPERATOR TESTS
// ════════════════════════════════════════════════════════════════════════

func TestBitwiseAnd(t *testing.T) {
	code := compile(t, "x = 5 & 3\n")
	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBitAnd {
			found = true
		}
	}
	if !found {
		t.Errorf("expected BIT_AND opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestBitwiseOr(t *testing.T) {
	code := compile(t, "x = 5 | 3\n")
	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBitOr {
			found = true
		}
	}
	if !found {
		t.Errorf("expected BIT_OR opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestBitwiseXor(t *testing.T) {
	code := compile(t, "x = 5 ^ 3\n")
	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpBitXor {
			found = true
		}
	}
	if !found {
		t.Errorf("expected BIT_XOR opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestLeftShift(t *testing.T) {
	code := compile(t, "x = 1 << 3\n")
	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpLShift {
			found = true
		}
	}
	if !found {
		t.Errorf("expected LEFT_SHIFT opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

func TestRightShift(t *testing.T) {
	code := compile(t, "x = 8 >> 2\n")
	found := false
	for _, instr := range code.Instructions {
		if instr.Opcode == OpRShift {
			found = true
		}
	}
	if !found {
		t.Errorf("expected RIGHT_SHIFT opcode\nDisassembly:\n%s", Disassemble(code))
	}
}

// ════════════════════════════════════════════════════════════════════════
// EXPRESSION STATEMENT TEST
// ════════════════════════════════════════════════════════════════════════

func TestExpressionStatement(t *testing.T) {
	// A bare expression should be compiled and then popped
	code := compile(t, "42\n")

	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpPop)
	assertOpcode(t, code, 2, OpHalt)
}

// ════════════════════════════════════════════════════════════════════════
// VARIABLE REFERENCE TEST
// ════════════════════════════════════════════════════════════════════════

func TestVariableReference(t *testing.T) {
	code := compile(t, "x = 1\ny = x\n")

	// x = 1: LOAD_CONST 0, STORE_NAME 0
	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpStoreName)
	// y = x: LOAD_NAME 0, STORE_NAME 1
	assertOpcode(t, code, 2, OpLoadName)
	assertOperand(t, code, 2, 0) // x is names[0]
	assertOpcode(t, code, 3, OpStoreName)
	assertOperand(t, code, 3, 1) // y is names[1]
}

// ════════════════════════════════════════════════════════════════════════
// END-TO-END COMPILATION TEST
// ════════════════════════════════════════════════════════════════════════

func TestCompileStarlarkEndToEnd(t *testing.T) {
	// Test the full CompileStarlark API
	source := "x = 1 + 2\n"
	code, err := CompileStarlark(source)
	if err != nil {
		t.Fatalf("CompileStarlark error: %v", err)
	}

	if len(code.Instructions) < 4 {
		t.Fatalf("expected at least 4 instructions, got %d\nDisassembly:\n%s",
			len(code.Instructions), Disassemble(code))
	}

	assertOpcode(t, code, 0, OpLoadConst)
	assertOpcode(t, code, 1, OpLoadConst)
	assertOpcode(t, code, 2, OpAdd)
	assertOpcode(t, code, 3, OpStoreName)
	assertOpcode(t, code, 4, OpHalt)

	assertConstant(t, code, 0, 1)
	assertConstant(t, code, 1, 2)
	assertName(t, code, 0, "x")
}

func TestCompileStarlarkParseError(t *testing.T) {
	// Invalid syntax should return an error
	_, err := CompileStarlark("def\n")
	if err == nil {
		t.Error("expected parse error for invalid syntax, got nil")
	}
}

// ════════════════════════════════════════════════════════════════════════
// OPCODE NAMES TEST
// ════════════════════════════════════════════════════════════════════════

func TestOpcodeNamesCoverage(t *testing.T) {
	// Ensure all opcodes have human-readable names
	opcodes := []vm.OpCode{
		OpLoadConst, OpPop, OpDup, OpLoadNone, OpLoadTrue, OpLoadFalse,
		OpStoreName, OpLoadName, OpStoreLocal, OpLoadLocal,
		OpStoreClosure, OpLoadClosure,
		OpAdd, OpSub, OpMul, OpDiv, OpFloorDiv, OpMod, OpPower,
		OpNegate, OpBitAnd, OpBitOr, OpBitXor, OpBitNot, OpLShift, OpRShift,
		OpCmpEq, OpCmpLt, OpCmpGt, OpCmpNe, OpCmpLe, OpCmpGe, OpCmpIn, OpCmpNotIn,
		OpNot,
		OpJump, OpJumpIfFalse, OpJumpIfTrue, OpJumpIfFalseOrPop, OpJumpIfTrueOrPop,
		OpBreak, OpContinue,
		OpMakeFunction, OpCallFunction, OpCallFunctionKW, OpReturnValue,
		OpBuildList, OpBuildDict, OpBuildTuple, OpListAppend, OpDictSet,
		OpLoadSubscript, OpStoreSubscript, OpLoadAttr, OpStoreAttr, OpLoadSlice,
		OpGetIter, OpForIter, OpUnpackSequence,
		OpLoadModule, OpImportFrom,
		OpPrintValue,
		OpHalt,
	}

	for _, op := range opcodes {
		name, ok := OpcodeName[op]
		if !ok || name == "" {
			t.Errorf("opcode 0x%02x has no name in OpcodeName map", int(op))
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// DISASSEMBLE TEST
// ════════════════════════════════════════════════════════════════════════

func TestDisassemble(t *testing.T) {
	code := compile(t, "x = 42\n")
	output := Disassemble(code)

	if output == "" {
		t.Error("expected non-empty disassembly output")
	}

	// Should contain the opcode names
	if !containsString(output, "LOAD_CONST") {
		t.Errorf("disassembly should contain LOAD_CONST:\n%s", output)
	}
	if !containsString(output, "STORE_NAME") {
		t.Errorf("disassembly should contain STORE_NAME:\n%s", output)
	}
	if !containsString(output, "HALT") {
		t.Errorf("disassembly should contain HALT:\n%s", output)
	}
}

func containsString(haystack, needle string) bool {
	return len(haystack) >= len(needle) &&
		(haystack == needle || len(haystack) > 0 && findSubstring(haystack, needle))
}

func findSubstring(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// ════════════════════════════════════════════════════════════════════════
// STRING LITERAL TESTS
// ════════════════════════════════════════════════════════════════════════

func TestParseStringLiteral(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		// Strings with quotes still present (prefixed strings not stripped by lexer)
		{`"hello"`, "hello"},
		{`'world'`, "world"},
		{`"a\nb"`, "a\nb"},
		{`"a\\b"`, "a\\b"},
		{`"a\"b"`, `a"b`},
		{`r"a\nb"`, `a\nb`},
		// Strings already stripped by the lexer (no prefix, bare content)
		{"hello", "hello"},
		{"", ""},
	}
	for _, tt := range tests {
		result := parseStringLiteral(tt.input)
		if result != tt.expected {
			t.Errorf("parseStringLiteral(%q) = %q, want %q", tt.input, result, tt.expected)
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// COMPILE AST TEST
// ════════════════════════════════════════════════════════════════════════

func TestCompileAST(t *testing.T) {
	starlarkparser.ParseStarlark("x = 1\n")
	// Just test that CompileAST doesn't panic
	ast, err := starlarkparser.ParseStarlark("x = 1\n")
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	code := CompileAST(ast)
	if len(code.Instructions) < 3 {
		t.Errorf("expected at least 3 instructions, got %d", len(code.Instructions))
	}
}
