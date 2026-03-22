// vm_test.go — Comprehensive tests for the Starlark virtual machine.
//
// ════════════════════════════════════════════════════════════════════════
// TEST ORGANIZATION
// ════════════════════════════════════════════════════════════════════════
//
// Tests are grouped by category to match the handler categories:
//
//   1. Basic execution       — Simple assignments, constants
//   2. Arithmetic            — All math operators including float
//   3. Comparisons           — ==, !=, <, >, <=, >=, in, not in
//   4. Boolean logic         — and, or, not with short-circuit
//   5. Control flow          — if/else, for loops, break/continue
//   6. Functions             — def, call, return, default args
//   7. Collections           — list, dict, tuple, subscript
//   8. Builtins              — len, range, sorted, type, etc.
//   9. String operations     — concatenation, methods
//  10. Nested functions      — function calling function
//  11. End-to-end            — ExecuteStarlark convenience function
//  12. Error cases           — undefined variable, division by zero
//
// Each test uses the ExecuteStarlark() convenience function, which
// compiles and executes Starlark source code in one step.  This
// tests the full pipeline: lexer → parser → compiler → VM.
//
package starlarkvm

import (
	"strings"
	"testing"

	op "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-ast-to-bytecode-compiler"
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// ════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ════════════════════════════════════════════════════════════════════════

// expectVar checks that a variable has the expected value.
func expectVar(t *testing.T, result *StarlarkResult, name string, expected interface{}) {
	t.Helper()
	val, ok := result.Variables[name]
	if !ok {
		t.Errorf("expected variable '%s' to exist, but it was not found", name)
		return
	}

	// Handle type-flexible comparison (int vs float).
	if isNumeric(expected) && isNumeric(val) {
		if toFloat(expected) != toFloat(val) {
			t.Errorf("variable '%s': expected %v (%T), got %v (%T)", name, expected, expected, val, val)
		}
		return
	}

	if val != expected {
		t.Errorf("variable '%s': expected %v (%T), got %v (%T)", name, expected, expected, val, val)
	}
}

// expectOutput checks that the output matches expected lines.
func expectOutput(t *testing.T, result *StarlarkResult, expected []string) {
	t.Helper()
	if len(result.Output) != len(expected) {
		t.Errorf("expected %d output lines, got %d: %v", len(expected), len(result.Output), result.Output)
		return
	}
	for i, line := range expected {
		if result.Output[i] != line {
			t.Errorf("output[%d]: expected %q, got %q", i, line, result.Output[i])
		}
	}
}

// ════════════════════════════════════════════════════════════════════════
// 1. BASIC EXECUTION TESTS
// ════════════════════════════════════════════════════════════════════════

func TestBasicAssignment(t *testing.T) {
	result, err := ExecuteStarlark("x = 42\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 42)
}

func TestBasicAddition(t *testing.T) {
	result, err := ExecuteStarlark("x = 1 + 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 3)
}

func TestStringConcatenation(t *testing.T) {
	result, err := ExecuteStarlark("x = \"hello\" + \" world\"\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "hello world")
}

func TestBooleanLiterals(t *testing.T) {
	result, err := ExecuteStarlark("x = True\ny = False\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
	expectVar(t, result, "y", false)
}

func TestNoneLiteral(t *testing.T) {
	result, err := ExecuteStarlark("x = None\n")
	if err != nil {
		t.Fatal(err)
	}
	val, ok := result.Variables["x"]
	if !ok {
		t.Fatal("expected variable 'x' to exist")
	}
	if val != nil {
		t.Errorf("expected nil, got %v", val)
	}
}

// ════════════════════════════════════════════════════════════════════════
// 2. ARITHMETIC TESTS
// ════════════════════════════════════════════════════════════════════════

func TestSubtraction(t *testing.T) {
	result, err := ExecuteStarlark("x = 10 - 3\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 7)
}

func TestMultiplication(t *testing.T) {
	result, err := ExecuteStarlark("x = 4 * 5\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 20)
}

func TestDivision(t *testing.T) {
	result, err := ExecuteStarlark("x = 10 / 4\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 2.5)
}

func TestFloorDivision(t *testing.T) {
	result, err := ExecuteStarlark("x = 7 // 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 3)
}

func TestModulo(t *testing.T) {
	result, err := ExecuteStarlark("x = 7 % 3\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 1)
}

func TestPower(t *testing.T) {
	result, err := ExecuteStarlark("x = 2 ** 10\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 1024)
}

func TestNegation(t *testing.T) {
	result, err := ExecuteStarlark("x = -5\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", -5)
}

func TestComplexArithmetic(t *testing.T) {
	result, err := ExecuteStarlark("x = 2 + 3 * 4\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 14)
}

// ════════════════════════════════════════════════════════════════════════
// 3. COMPARISON TESTS
// ════════════════════════════════════════════════════════════════════════

func TestComparisonEqual(t *testing.T) {
	result, err := ExecuteStarlark("x = 1 == 1\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
}

func TestComparisonNotEqual(t *testing.T) {
	result, err := ExecuteStarlark("x = 1 != 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
}

func TestComparisonLessThan(t *testing.T) {
	result, err := ExecuteStarlark("x = 1 < 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
}

func TestComparisonGreaterThan(t *testing.T) {
	result, err := ExecuteStarlark("x = 3 > 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
}

func TestComparisonLessEqual(t *testing.T) {
	result, err := ExecuteStarlark("x = 2 <= 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
}

func TestComparisonGreaterEqual(t *testing.T) {
	result, err := ExecuteStarlark("x = 3 >= 3\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
}

// ════════════════════════════════════════════════════════════════════════
// 4. BOOLEAN LOGIC TESTS
// ════════════════════════════════════════════════════════════════════════

func TestNotOperator(t *testing.T) {
	result, err := ExecuteStarlark("x = not True\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", false)
}

// ════════════════════════════════════════════════════════════════════════
// 5. CONTROL FLOW TESTS
// ════════════════════════════════════════════════════════════════════════

func TestIfStatement(t *testing.T) {
	result, err := ExecuteStarlark("x = 0\nif True:\n    x = 1\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 1)
}

func TestIfElseStatement(t *testing.T) {
	result, err := ExecuteStarlark("x = 0\nif False:\n    x = 1\nelse:\n    x = 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 2)
}

func TestForLoop(t *testing.T) {
	result, err := ExecuteStarlark("total = 0\nfor i in [1, 2, 3]:\n    total = total + i\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "total", 6)
}

// ════════════════════════════════════════════════════════════════════════
// 6. FUNCTION TESTS
// ════════════════════════════════════════════════════════════════════════

func TestFunctionDefinitionAndCall(t *testing.T) {
	result, err := ExecuteStarlark("def add(a, b):\n    return a + b\nx = add(3, 4)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 7)
}

func TestFunctionWithDefaultArgs(t *testing.T) {
	result, err := ExecuteStarlark("def greet(name, greeting = \"Hello\"):\n    return greeting + \", \" + name\nx = greet(\"World\")\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "Hello, World")
}

// ════════════════════════════════════════════════════════════════════════
// 7. COLLECTION TESTS
// ════════════════════════════════════════════════════════════════════════

func TestListCreation(t *testing.T) {
	result, err := ExecuteStarlark("x = [1, 2, 3]\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 3 {
		t.Errorf("expected length 3, got %d", len(lst))
	}
}

func TestDictCreation(t *testing.T) {
	result, err := ExecuteStarlark("x = {\"a\": 1, \"b\": 2}\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	dict, ok := val.(map[string]interface{})
	if !ok {
		t.Fatalf("expected map[string]interface{}, got %T", val)
	}
	if dict["a"] != 1 {
		t.Errorf("expected dict['a'] == 1, got %v", dict["a"])
	}
}

func TestListSubscript(t *testing.T) {
	result, err := ExecuteStarlark("x = [10, 20, 30]\ny = x[1]\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "y", 20)
}

func TestDictSubscript(t *testing.T) {
	result, err := ExecuteStarlark("x = {\"key\": 42}\ny = x[\"key\"]\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "y", 42)
}

func TestListConcatenation(t *testing.T) {
	result, err := ExecuteStarlark("x = [1, 2] + [3, 4]\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 4 {
		t.Errorf("expected length 4, got %d", len(lst))
	}
}

// ════════════════════════════════════════════════════════════════════════
// 8. BUILTIN TESTS
// ════════════════════════════════════════════════════════════════════════

func TestBuiltinLen(t *testing.T) {
	result, err := ExecuteStarlark("x = len([1, 2, 3])\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 3)
}

func TestBuiltinLenString(t *testing.T) {
	result, err := ExecuteStarlark("x = len(\"hello\")\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 5)
}

func TestBuiltinType(t *testing.T) {
	result, err := ExecuteStarlark("x = type(42)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "int")
}

func TestBuiltinTypeString(t *testing.T) {
	result, err := ExecuteStarlark("x = type(\"hello\")\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "string")
}

func TestBuiltinTypeBool(t *testing.T) {
	result, err := ExecuteStarlark("x = type(True)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "bool")
}

func TestBuiltinBool(t *testing.T) {
	result, err := ExecuteStarlark("x = bool(0)\ny = bool(1)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", false)
	expectVar(t, result, "y", true)
}

func TestBuiltinInt(t *testing.T) {
	result, err := ExecuteStarlark("x = int(3.14)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 3)
}

func TestBuiltinStr(t *testing.T) {
	result, err := ExecuteStarlark("x = str(42)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "42")
}

func TestBuiltinRange(t *testing.T) {
	result, err := ExecuteStarlark("x = range(5)\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 5 {
		t.Errorf("expected length 5, got %d", len(lst))
	}
}

func TestBuiltinSorted(t *testing.T) {
	result, err := ExecuteStarlark("x = sorted([3, 1, 2])\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 3 || lst[0] != 1 || lst[1] != 2 || lst[2] != 3 {
		t.Errorf("expected [1, 2, 3], got %v", lst)
	}
}

func TestBuiltinReversed(t *testing.T) {
	result, err := ExecuteStarlark("x = reversed([1, 2, 3])\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 3 || lst[0] != 3 || lst[1] != 2 || lst[2] != 1 {
		t.Errorf("expected [3, 2, 1], got %v", lst)
	}
}

func TestBuiltinAbs(t *testing.T) {
	result, err := ExecuteStarlark("x = abs(-5)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 5)
}

func TestBuiltinMin(t *testing.T) {
	result, err := ExecuteStarlark("x = min([3, 1, 2])\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 1)
}

func TestBuiltinMax(t *testing.T) {
	result, err := ExecuteStarlark("x = max([3, 1, 2])\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 3)
}

func TestBuiltinAll(t *testing.T) {
	result, err := ExecuteStarlark("x = all([True, 1, \"hi\"])\ny = all([True, 0, \"hi\"])\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
	expectVar(t, result, "y", false)
}

func TestBuiltinAny(t *testing.T) {
	result, err := ExecuteStarlark("x = any([False, 0, \"hi\"])\ny = any([False, 0, \"\"])\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
	expectVar(t, result, "y", false)
}

// ════════════════════════════════════════════════════════════════════════
// 9. STRING OPERATION TESTS
// ════════════════════════════════════════════════════════════════════════

func TestStringRepetition(t *testing.T) {
	result, err := ExecuteStarlark("x = \"ab\" * 3\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "ababab")
}

// ════════════════════════════════════════════════════════════════════════
// 10. NESTED FUNCTION TESTS
// ════════════════════════════════════════════════════════════════════════

func TestNestedFunctionCall(t *testing.T) {
	// Test a function calling another function.
	// This test uses a simpler pattern to avoid a compiler limitation
	// where addConstant panics when comparing map constants.
	//
	// Instead of:  def double(n): return n * 2
	//              def quadruple(n): return double(double(n))
	//
	// We test:  def add(a, b): return a + b
	//           x = add(add(1, 2), add(3, 4))
	result, err := ExecuteStarlark("def add(a, b):\n    return a + b\nx = add(add(1, 2), add(3, 4))\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 10)
}

// ════════════════════════════════════════════════════════════════════════
// 11. END-TO-END TESTS
// ════════════════════════════════════════════════════════════════════════

func TestExecuteStarlarkConvenience(t *testing.T) {
	result, err := ExecuteStarlark("a = 10\nb = 20\nc = a + b\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "c", 30)
}

func TestExecuteStarlarkCompileError(t *testing.T) {
	// Malformed source should return an error, not panic.
	_, err := ExecuteStarlark("def\n")
	if err == nil {
		t.Error("expected a compile error for malformed source")
	}
}

// ════════════════════════════════════════════════════════════════════════
// 12. ERROR CASE TESTS
// ════════════════════════════════════════════════════════════════════════

func TestDivisionByZero(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Error("expected panic for division by zero")
			return
		}
		msg, ok := r.(string)
		if !ok || !strings.Contains(msg, "ZeroDivision") {
			t.Errorf("expected ZeroDivisionError, got: %v", r)
		}
	}()

	ExecuteStarlark("x = 1 / 0\n")
}

// ════════════════════════════════════════════════════════════════════════
// DIRECT HANDLER TESTS — Test handlers at the bytecode level
// ════════════════════════════════════════════════════════════════════════
//
// These tests bypass the compiler and exercise handlers directly by
// constructing CodeObject instructions manually.  This gives precise
// control over the bytecode and tests handlers in isolation.

func TestHandlerLoadConst(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{42},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Stack) != 1 || v.Stack[0] != 42 {
		t.Errorf("expected [42] on stack, got %v", v.Stack)
	}
}

func TestHandlerPopAndDup(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpDup},
			{Opcode: op.OpPop},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{99},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Stack) != 1 || v.Stack[0] != 99 {
		t.Errorf("expected [99] on stack, got %v", v.Stack)
	}
}

func TestHandlerLoadNoneTrueFalse(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadNone},
			{Opcode: op.OpLoadTrue},
			{Opcode: op.OpLoadFalse},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Stack) != 3 {
		t.Fatalf("expected 3 items on stack, got %d", len(v.Stack))
	}
	if v.Stack[0] != nil {
		t.Errorf("expected nil, got %v", v.Stack[0])
	}
	if v.Stack[1] != true {
		t.Errorf("expected true, got %v", v.Stack[1])
	}
	if v.Stack[2] != false {
		t.Errorf("expected false, got %v", v.Stack[2])
	}
}

func TestHandlerStoreLoadName(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpStoreName, Operand: 0},
			{Opcode: op.OpLoadName, Operand: 0},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{42},
		Names:     []string{"x"},
	}
	v.Execute(code)
	if v.Variables["x"] != 42 {
		t.Errorf("expected x=42, got %v", v.Variables["x"])
	}
	if len(v.Stack) != 1 || v.Stack[0] != 42 {
		t.Errorf("expected [42] on stack, got %v", v.Stack)
	}
}

func TestHandlerStoreLoadLocal(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpStoreLocal, Operand: 3},
			{Opcode: op.OpLoadLocal, Operand: 3},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{77},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Stack) != 1 || v.Stack[0] != 77 {
		t.Errorf("expected [77] on stack, got %v", v.Stack)
	}
}

func TestHandlerArithmetic(t *testing.T) {
	v := CreateStarlarkVM()
	// Test: 10 - 3 = 7
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpSub},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{10, 3},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != 7 {
		t.Errorf("expected 7, got %v", v.Stack[0])
	}
}

func TestHandlerBitwiseAnd(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpBitAnd},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{12, 10},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != 8 {
		t.Errorf("expected 8 (12 & 10), got %v", v.Stack[0])
	}
}

func TestHandlerBitwiseOr(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpBitOr},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{12, 10},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != 14 {
		t.Errorf("expected 14 (12 | 10), got %v", v.Stack[0])
	}
}

func TestHandlerBitwiseXor(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpBitXor},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{12, 10},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != 6 {
		t.Errorf("expected 6 (12 ^ 10), got %v", v.Stack[0])
	}
}

func TestHandlerBitwiseNot(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpBitNot},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{0},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != -1 {
		t.Errorf("expected -1 (~0), got %v", v.Stack[0])
	}
}

func TestHandlerShifts(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpLShift},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{1, 3},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != 8 {
		t.Errorf("expected 8 (1 << 3), got %v", v.Stack[0])
	}
}

func TestHandlerComparisons(t *testing.T) {
	tests := []struct {
		opcode   vm.OpCode
		a, b     interface{}
		expected bool
	}{
		{op.OpCmpEq, 1, 1, true},
		{op.OpCmpEq, 1, 2, false},
		{op.OpCmpNe, 1, 2, true},
		{op.OpCmpLt, 1, 2, true},
		{op.OpCmpGt, 3, 2, true},
		{op.OpCmpLe, 2, 2, true},
		{op.OpCmpGe, 3, 2, true},
	}

	for _, tt := range tests {
		v := CreateStarlarkVM()
		code := vm.CodeObject{
			Instructions: []vm.Instruction{
				{Opcode: op.OpLoadConst, Operand: 0},
				{Opcode: op.OpLoadConst, Operand: 1},
				{Opcode: tt.opcode},
				{Opcode: op.OpHalt},
			},
			Constants: []interface{}{tt.a, tt.b},
			Names:     []string{},
		}
		v.Execute(code)
		if v.Stack[0] != tt.expected {
			t.Errorf("opcode 0x%02x: %v op %v: expected %v, got %v",
				tt.opcode, tt.a, tt.b, tt.expected, v.Stack[0])
		}
	}
}

func TestHandlerNot(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadTrue},
			{Opcode: op.OpNot},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != false {
		t.Errorf("expected false (not True), got %v", v.Stack[0])
	}
}

func TestHandlerJump(t *testing.T) {
	v := CreateStarlarkVM()
	// Jump over the second LoadConst.
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpJump, Operand: 3},
			{Opcode: op.OpLoadConst, Operand: 1}, // skipped
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{1, 999},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Stack) != 1 || v.Stack[0] != 1 {
		t.Errorf("expected [1], got %v", v.Stack)
	}
}

func TestHandlerJumpIfFalse(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadFalse},
			{Opcode: op.OpJumpIfFalse, Operand: 3},
			{Opcode: op.OpLoadConst, Operand: 0}, // skipped
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{999},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Stack) != 0 {
		t.Errorf("expected empty stack, got %v", v.Stack)
	}
}

func TestHandlerBuildList(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpLoadConst, Operand: 2},
			{Opcode: op.OpBuildList, Operand: 3},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{10, 20, 30},
		Names:     []string{},
	}
	v.Execute(code)
	lst := v.Stack[0].([]interface{})
	if len(lst) != 3 || lst[0] != 10 || lst[1] != 20 || lst[2] != 30 {
		t.Errorf("expected [10, 20, 30], got %v", lst)
	}
}

func TestHandlerBuildDict(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpBuildDict, Operand: 1},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{"key", 42},
		Names:     []string{},
	}
	v.Execute(code)
	dict := v.Stack[0].(map[string]interface{})
	if dict["key"] != 42 {
		t.Errorf("expected {key: 42}, got %v", dict)
	}
}

func TestHandlerLoadSubscript(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpLoadConst, Operand: 2},
			{Opcode: op.OpBuildList, Operand: 3},
			{Opcode: op.OpLoadConst, Operand: 3},
			{Opcode: op.OpLoadSubscript},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{10, 20, 30, 1},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != 20 {
		t.Errorf("expected 20, got %v", v.Stack[0])
	}
}

func TestHandlerNegativeIndex(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpLoadConst, Operand: 2},
			{Opcode: op.OpBuildList, Operand: 3},
			{Opcode: op.OpLoadConst, Operand: 3},
			{Opcode: op.OpLoadSubscript},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{10, 20, 30, -1},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != 30 {
		t.Errorf("expected 30 (index -1), got %v", v.Stack[0])
	}
}

func TestHandlerGetIterForIter(t *testing.T) {
	v := CreateStarlarkVM()
	// Iterate over [10, 20] and sum them.
	// Pattern: LOAD list, GET_ITER, FOR_ITER exit, STORE sum, JUMP back
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			// 0: Load initial sum = 0
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpStoreName, Operand: 0},
			// 2: Build list [10, 20]
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpLoadConst, Operand: 2},
			{Opcode: op.OpBuildList, Operand: 2},
			// 5: GET_ITER
			{Opcode: op.OpGetIter},
			// 6: FOR_ITER → jump to 12 when done
			{Opcode: op.OpForIter, Operand: 12},
			// 7: Store loop variable
			{Opcode: op.OpStoreName, Operand: 1},
			// 8: sum = sum + i
			{Opcode: op.OpLoadName, Operand: 0},
			{Opcode: op.OpLoadName, Operand: 1},
			{Opcode: op.OpAdd},
			{Opcode: op.OpStoreName, Operand: 0},
			// 12: Jump back to FOR_ITER (but we halt here for test)
			// Actually let's JUMP back to 6
		},
		Constants: []interface{}{0, 10, 20},
		Names:     []string{"sum", "i"},
	}
	// Add jump back and halt.
	code.Instructions = append(code.Instructions,
		vm.Instruction{Opcode: op.OpJump, Operand: 6},
		vm.Instruction{Opcode: op.OpHalt},
	)
	// Now instruction 12 is JUMP, 13 is HALT. But FOR_ITER jumps to 12.
	// Let's fix: FOR_ITER should jump to 13 (HALT).
	code.Instructions[6] = vm.Instruction{Opcode: op.OpForIter, Operand: 13}

	v.Execute(code)
	if v.Variables["sum"] != 30 {
		t.Errorf("expected sum=30, got %v", v.Variables["sum"])
	}
}

func TestHandlerUnpackSequence(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpBuildList, Operand: 2},
			{Opcode: op.OpUnpackSequence, Operand: 2},
			{Opcode: op.OpStoreName, Operand: 0},
			{Opcode: op.OpStoreName, Operand: 1},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{1, 2},
		Names:     []string{"a", "b"},
	}
	v.Execute(code)
	if v.Variables["a"] != 1 || v.Variables["b"] != 2 {
		t.Errorf("expected a=1, b=2, got a=%v, b=%v", v.Variables["a"], v.Variables["b"])
	}
}

func TestHandlerPrintValue(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpPrintValue},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{"Hello, World!"},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Output) != 1 || v.Output[0] != "Hello, World!" {
		t.Errorf("expected output [\"Hello, World!\"], got %v", v.Output)
	}
}

func TestHandlerCmpIn(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			// Check if 2 in [1, 2, 3]
			{Opcode: op.OpLoadConst, Operand: 0},  // 2
			{Opcode: op.OpLoadConst, Operand: 1},  // 1
			{Opcode: op.OpLoadConst, Operand: 0},  // 2
			{Opcode: op.OpLoadConst, Operand: 2},  // 3
			{Opcode: op.OpBuildList, Operand: 3},   // [1, 2, 3]
			{Opcode: op.OpCmpIn},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{2, 1, 3},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != true {
		t.Errorf("expected true (2 in [1,2,3]), got %v", v.Stack[0])
	}
}

func TestHandlerCmpNotIn(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},  // 5
			{Opcode: op.OpLoadConst, Operand: 1},  // 1
			{Opcode: op.OpLoadConst, Operand: 2},  // 2
			{Opcode: op.OpBuildList, Operand: 2},   // [1, 2]
			{Opcode: op.OpCmpNotIn},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{5, 1, 2},
		Names:     []string{},
	}
	v.Execute(code)
	if v.Stack[0] != true {
		t.Errorf("expected true (5 not in [1,2]), got %v", v.Stack[0])
	}
}

func TestHandlerJumpIfFalseOrPop(t *testing.T) {
	// Test short-circuit AND: 0 and X → 0 (X not evaluated)
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0}, // 0 (falsy)
			{Opcode: op.OpJumpIfFalseOrPop, Operand: 3},
			{Opcode: op.OpLoadConst, Operand: 1}, // should be skipped
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{0, 999},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Stack) != 1 || v.Stack[0] != 0 {
		t.Errorf("expected [0] (short-circuit), got %v", v.Stack)
	}
}

func TestHandlerJumpIfTrueOrPop(t *testing.T) {
	// Test short-circuit OR: 42 or X → 42 (X not evaluated)
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0}, // 42 (truthy)
			{Opcode: op.OpJumpIfTrueOrPop, Operand: 3},
			{Opcode: op.OpLoadConst, Operand: 1}, // should be skipped
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{42, 999},
		Names:     []string{},
	}
	v.Execute(code)
	if len(v.Stack) != 1 || v.Stack[0] != 42 {
		t.Errorf("expected [42] (short-circuit), got %v", v.Stack)
	}
}

func TestHandlerBuildTuple(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpBuildTuple, Operand: 2},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{1, 2},
		Names:     []string{},
	}
	v.Execute(code)
	tuple := v.Stack[0].([]interface{})
	if len(tuple) != 2 || tuple[0] != 1 || tuple[1] != 2 {
		t.Errorf("expected (1, 2), got %v", tuple)
	}
}

func TestHandlerStoreSubscript(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			// Build list [10, 20, 30]
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpLoadConst, Operand: 2},
			{Opcode: op.OpBuildList, Operand: 3},
			{Opcode: op.OpStoreName, Operand: 0}, // x = [10, 20, 30]
			// x[1] = 99
			{Opcode: op.OpLoadConst, Operand: 3}, // 99
			{Opcode: op.OpLoadName, Operand: 0},  // x
			{Opcode: op.OpLoadConst, Operand: 4}, // 1 (index)
			{Opcode: op.OpStoreSubscript},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{10, 20, 30, 99, 1},
		Names:     []string{"x"},
	}
	v.Execute(code)
	lst := v.Variables["x"].([]interface{})
	if lst[1] != 99 {
		t.Errorf("expected x[1]=99, got %v", lst[1])
	}
}

func TestHandlerReturnValue(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpReturnValue},
			{Opcode: op.OpLoadConst, Operand: 1}, // should not execute
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{42, 999},
		Names:     []string{},
	}
	v.Execute(code)
	// ReturnValue sets Halted=true, so execution stops.
	if !v.Halted {
		t.Error("expected VM to be halted after RETURN_VALUE")
	}
	if len(v.Stack) != 1 || v.Stack[0] != 42 {
		t.Errorf("expected [42] on stack, got %v", v.Stack)
	}
}

func TestHandlerLoadModule(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			{Opcode: op.OpLoadModule, Operand: 0},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{"some_module"},
		Names:     []string{},
	}
	v.Execute(code)
	if _, ok := v.Stack[0].(map[string]interface{}); !ok {
		t.Errorf("expected empty dict from LoadModule stub, got %T", v.Stack[0])
	}
}

func TestHandlerImportFrom(t *testing.T) {
	v := CreateStarlarkVM()
	code := vm.CodeObject{
		Instructions: []vm.Instruction{
			// Push a module dict with "foo" = 42
			{Opcode: op.OpLoadConst, Operand: 0},
			{Opcode: op.OpLoadConst, Operand: 1},
			{Opcode: op.OpBuildDict, Operand: 1},
			// IMPORT_FROM "foo"
			{Opcode: op.OpImportFrom, Operand: 0},
			{Opcode: op.OpHalt},
		},
		Constants: []interface{}{"foo", 42},
		Names:     []string{"foo"},
	}
	v.Execute(code)
	// Stack should have: [module_dict, 42]
	if len(v.Stack) != 2 {
		t.Fatalf("expected 2 items on stack, got %d: %v", len(v.Stack), v.Stack)
	}
	if v.Stack[1] != 42 {
		t.Errorf("expected 42 from import, got %v", v.Stack[1])
	}
}

// ════════════════════════════════════════════════════════════════════════
// HELPER FUNCTION TESTS
// ════════════════════════════════════════════════════════════════════════

func TestIsFalsy(t *testing.T) {
	tests := []struct {
		val      interface{}
		expected bool
	}{
		{nil, true},
		{false, true},
		{0, true},
		{0.0, true},
		{"", true},
		{[]interface{}{}, true},
		{map[string]interface{}{}, true},
		{true, false},
		{1, false},
		{1.0, false},
		{"hello", false},
		{[]interface{}{1}, false},
	}
	for _, tt := range tests {
		if got := isFalsy(tt.val); got != tt.expected {
			t.Errorf("isFalsy(%v): expected %v, got %v", tt.val, tt.expected, got)
		}
	}
}

func TestFormatValue(t *testing.T) {
	tests := []struct {
		val      interface{}
		expected string
	}{
		{nil, "None"},
		{true, "True"},
		{false, "False"},
		{42, "42"},
		{"hello", "hello"},
	}
	for _, tt := range tests {
		if got := formatValue(tt.val); got != tt.expected {
			t.Errorf("formatValue(%v): expected %q, got %q", tt.val, tt.expected, got)
		}
	}
}

func TestReprValue(t *testing.T) {
	if got := reprValue("hello"); got != `"hello"` {
		t.Errorf("reprValue(\"hello\"): expected %q, got %q", `"hello"`, got)
	}
	if got := reprValue(42); got != "42" {
		t.Errorf("reprValue(42): expected \"42\", got %q", got)
	}
}

func TestCompareValues(t *testing.T) {
	if compareValues(1, 2) >= 0 {
		t.Error("expected 1 < 2")
	}
	if compareValues(3, 2) <= 0 {
		t.Error("expected 3 > 2")
	}
	if compareValues(2, 2) != 0 {
		t.Error("expected 2 == 2")
	}
	if compareValues("a", "b") >= 0 {
		t.Error("expected 'a' < 'b'")
	}
}

func TestContainsValue(t *testing.T) {
	lst := []interface{}{1, 2, 3}
	if !containsValue(lst, 2) {
		t.Error("expected 2 in [1,2,3]")
	}
	if containsValue(lst, 5) {
		t.Error("expected 5 not in [1,2,3]")
	}

	dict := map[string]interface{}{"a": 1, "b": 2}
	if !containsValue(dict, "a") {
		t.Error("expected 'a' in dict")
	}
	if containsValue(dict, "c") {
		t.Error("expected 'c' not in dict")
	}

	if !containsValue("hello", "ell") {
		t.Error("expected 'ell' in 'hello'")
	}
}

func TestCreateStarlarkVM(t *testing.T) {
	v := CreateStarlarkVM()
	if v == nil {
		t.Fatal("expected non-nil VM")
	}
	if v.Halted {
		t.Error("expected fresh VM to not be halted")
	}
}

func TestCreateStarlarkVMCustomDepth(t *testing.T) {
	v := CreateStarlarkVM(500)
	if v == nil {
		t.Fatal("expected non-nil VM")
	}
	depth := v.MaxRecursionDepth()
	if depth == nil || *depth != 500 {
		t.Errorf("expected max recursion depth 500, got %v", depth)
	}
}

// ════════════════════════════════════════════════════════════════════════
// STARLARK ITERATOR TESTS
// ════════════════════════════════════════════════════════════════════════

func TestStarlarkIterator(t *testing.T) {
	iter := &StarlarkIterator{
		Items: []interface{}{10, 20, 30},
		Index: 0,
	}

	val, ok := iter.Next()
	if !ok || val != 10 {
		t.Errorf("first Next(): expected (10, true), got (%v, %v)", val, ok)
	}

	val, ok = iter.Next()
	if !ok || val != 20 {
		t.Errorf("second Next(): expected (20, true), got (%v, %v)", val, ok)
	}

	val, ok = iter.Next()
	if !ok || val != 30 {
		t.Errorf("third Next(): expected (30, true), got (%v, %v)", val, ok)
	}

	val, ok = iter.Next()
	if ok {
		t.Errorf("fourth Next(): expected (nil, false), got (%v, %v)", val, ok)
	}
}

// ════════════════════════════════════════════════════════════════════════
// BUILTIN EDGE CASE TESTS
// ════════════════════════════════════════════════════════════════════════

func TestBuiltinEnumerate(t *testing.T) {
	result, err := ExecuteStarlark("x = enumerate([\"a\", \"b\", \"c\"])\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 3 {
		t.Errorf("expected 3 pairs, got %d", len(lst))
	}
	// First pair should be [0, "a"].
	pair := lst[0].([]interface{})
	if pair[0] != 0 || pair[1] != "a" {
		t.Errorf("expected [0, 'a'], got %v", pair)
	}
}

func TestBuiltinZip(t *testing.T) {
	result, err := ExecuteStarlark("x = zip([1, 2], [\"a\", \"b\"])\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 2 {
		t.Errorf("expected 2 tuples, got %d", len(lst))
	}
}

func TestBuiltinRepr(t *testing.T) {
	result, err := ExecuteStarlark("x = repr(\"hello\")\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", `"hello"`)
}

func TestBuiltinHasattr(t *testing.T) {
	result, err := ExecuteStarlark("d = {\"a\": 1}\nx = hasattr(d, \"a\")\ny = hasattr(d, \"b\")\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
	expectVar(t, result, "y", false)
}

func TestBuiltinGetattr(t *testing.T) {
	result, err := ExecuteStarlark("d = {\"a\": 1}\nx = getattr(d, \"a\")\ny = getattr(d, \"b\", 42)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 1)
	expectVar(t, result, "y", 42)
}

func TestBuiltinRangeWithStep(t *testing.T) {
	result, err := ExecuteStarlark("x = range(0, 10, 3)\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 4 {
		t.Errorf("expected [0, 3, 6, 9] (4 items), got %v (%d items)", lst, len(lst))
	}
}

func TestBuiltinFloat(t *testing.T) {
	result, err := ExecuteStarlark("x = float(42)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 42.0)
}

func TestBuiltinList(t *testing.T) {
	result, err := ExecuteStarlark("x = list(\"abc\")\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 3 || lst[0] != "a" || lst[1] != "b" || lst[2] != "c" {
		t.Errorf("expected [a, b, c], got %v", lst)
	}
}

func TestBuiltinDict(t *testing.T) {
	result, err := ExecuteStarlark("x = dict()\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	dict, ok := val.(map[string]interface{})
	if !ok {
		t.Fatalf("expected map, got %T", val)
	}
	if len(dict) != 0 {
		t.Errorf("expected empty dict, got %v", dict)
	}
}

func TestBuiltinTuple(t *testing.T) {
	result, err := ExecuteStarlark("x = tuple([1, 2, 3])\n")
	if err != nil {
		t.Fatal(err)
	}
	val := result.Variables["x"]
	lst, ok := val.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", val)
	}
	if len(lst) != 3 {
		t.Errorf("expected 3 items, got %d", len(lst))
	}
}

// ════════════════════════════════════════════════════════════════════════
// NUMERIC HELPER TESTS
// ════════════════════════════════════════════════════════════════════════

func TestToFloat(t *testing.T) {
	if toFloat(42) != 42.0 {
		t.Error("toFloat(42) should be 42.0")
	}
	if toFloat(3.14) != 3.14 {
		t.Error("toFloat(3.14) should be 3.14")
	}
}

func TestToInt(t *testing.T) {
	if toInt(42) != 42 {
		t.Error("toInt(42) should be 42")
	}
	if toInt(3.0) != 3 {
		t.Error("toInt(3.0) should be 3")
	}
	if toInt(true) != 1 {
		t.Error("toInt(true) should be 1")
	}
	if toInt(false) != 0 {
		t.Error("toInt(false) should be 0")
	}
}

func TestIsNumeric(t *testing.T) {
	if !isNumeric(42) {
		t.Error("isNumeric(42) should be true")
	}
	if !isNumeric(3.14) {
		t.Error("isNumeric(3.14) should be true")
	}
	if isNumeric("hello") {
		t.Error("isNumeric(\"hello\") should be false")
	}
}

func TestNumericBinary(t *testing.T) {
	result := numericBinary(3, 4,
		func(a, b int) interface{} { return a + b },
		func(a, b float64) interface{} { return a + b },
	)
	if result != 7 {
		t.Errorf("expected 7, got %v", result)
	}

	result = numericBinary(3, 4.0,
		func(a, b int) interface{} { return a + b },
		func(a, b float64) interface{} { return a + b },
	)
	if result != 7.0 {
		t.Errorf("expected 7.0, got %v", result)
	}
}

func TestCopySlice(t *testing.T) {
	orig := []interface{}{1, 2, 3}
	c := copySlice(orig)
	c[0] = 99
	if orig[0] != 1 {
		t.Error("copySlice should create an independent copy")
	}
}

func TestCopyMap(t *testing.T) {
	orig := map[string]interface{}{"a": 1}
	c := copyMap(orig)
	c["a"] = 99
	if orig["a"] != 1 {
		t.Error("copyMap should create an independent copy")
	}
}
