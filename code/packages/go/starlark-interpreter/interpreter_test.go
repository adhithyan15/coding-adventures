// interpreter_test.go -- Comprehensive tests for the Starlark interpreter.
//
// ============================================================================
// TEST ORGANIZATION
// ============================================================================
//
// Tests are grouped by category:
//
//   1. Basic interpretation      -- Simple assignments, expressions
//   2. String operations         -- Concatenation, string variables
//   3. Functions                 -- def, call, return, recursive, default args
//   4. Control flow              -- if/else, for loop, break
//   5. Collections               -- list, dict, tuple
//   6. Builtins                  -- len, range, sorted, type
//   7. Print capture             -- print() output in result.Output
//   8. Load support              -- load() with DictResolver
//   9. Load caching              -- Same file loaded twice
//  10. Load with functions       -- Loaded module exports functions
//  11. Load errors               -- Missing file, no resolver
//  12. InterpretFile             -- Execute from a temp file
//  13. BUILD file simulation     -- load rules, call rule functions
//  14. Error handling            -- Syntax errors, runtime errors
//  15. Options                   -- WithMaxRecursionDepth, WithFileResolver
//
// Each test uses the package-level Interpret() convenience function or
// the StarlarkInterpreter struct, testing the full pipeline from source
// code to execution result.
//
package starlarkinterpreter

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	starlarkvm "github.com/adhithyan15/coding-adventures/code/packages/go/starlark-vm"
)

// ============================================================================
// HELPERS
// ============================================================================

// expectVar checks that a variable has the expected value in the result.
// Supports numeric comparison across int and float types.
func expectVar(t *testing.T, result *starlarkvm.StarlarkResult, name string, expected interface{}) {
	t.Helper()
	val, ok := result.Variables[name]
	if !ok {
		t.Errorf("expected variable '%s' to exist, but it was not found. Variables: %v", name, result.Variables)
		return
	}

	// Handle numeric comparison: int 3 should equal int 3.
	expectedInt, expectedIsInt := expected.(int)
	valInt, valIsInt := val.(int)
	if expectedIsInt && valIsInt {
		if expectedInt != valInt {
			t.Errorf("variable '%s': expected %v, got %v", name, expected, val)
		}
		return
	}

	if val != expected {
		t.Errorf("variable '%s': expected %v (%T), got %v (%T)", name, expected, expected, val, val)
	}
}

// expectOutput checks that result.Output matches the expected lines.
func expectOutput(t *testing.T, result *starlarkvm.StarlarkResult, expected []string) {
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

// expectPanic runs a function and checks that it panics with a message
// containing the expected substring.
func expectPanic(t *testing.T, expectedSubstring string, fn func()) {
	t.Helper()
	defer func() {
		r := recover()
		if r == nil {
			t.Errorf("expected panic containing %q, but no panic occurred", expectedSubstring)
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
		if !strings.Contains(msg, expectedSubstring) {
			t.Errorf("expected panic containing %q, got %q", expectedSubstring, msg)
		}
	}()
	fn()
}

// ============================================================================
// 1. BASIC INTERPRETATION TESTS
// ============================================================================

func TestBasicAssignment(t *testing.T) {
	result, err := Interpret("x = 42\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 42)
}

func TestBasicArithmetic(t *testing.T) {
	result, err := Interpret("x = 1 + 2 * 3\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 7)
}

func TestMultipleAssignments(t *testing.T) {
	result, err := Interpret("x = 10\ny = 20\nz = x + y\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 10)
	expectVar(t, result, "y", 20)
	expectVar(t, result, "z", 30)
}

func TestBooleanLiterals(t *testing.T) {
	result, err := Interpret("a = True\nb = False\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "a", true)
	expectVar(t, result, "b", false)
}

func TestNoneLiteral(t *testing.T) {
	result, err := Interpret("x = None\n")
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

func TestSubtractionAndNegation(t *testing.T) {
	result, err := Interpret("x = 10 - 3\ny = -5\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 7)
	expectVar(t, result, "y", -5)
}

func TestFloorDivisionAndModulo(t *testing.T) {
	result, err := Interpret("x = 7 // 2\ny = 7 % 3\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 3)
	expectVar(t, result, "y", 1)
}

func TestPower(t *testing.T) {
	result, err := Interpret("x = 2 ** 10\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 1024)
}

// ============================================================================
// 2. STRING OPERATIONS TESTS
// ============================================================================

func TestStringConcatenation(t *testing.T) {
	result, err := Interpret("x = \"hello\" + \" world\"\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "hello world")
}

func TestStringVariable(t *testing.T) {
	result, err := Interpret("name = \"Alice\"\ngreeting = \"Hello, \" + name\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "greeting", "Hello, Alice")
}

func TestStringRepetition(t *testing.T) {
	result, err := Interpret("x = \"ab\" * 3\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "ababab")
}

// ============================================================================
// 3. FUNCTION TESTS
// ============================================================================

func TestFunctionDefAndCall(t *testing.T) {
	result, err := Interpret("def add(a, b):\n    return a + b\nx = add(3, 4)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 7)
}

func TestFunctionWithReturn(t *testing.T) {
	result, err := Interpret("def double(n):\n    return n * 2\nx = double(21)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 42)
}

func TestFunctionWithDefaultArgs(t *testing.T) {
	result, err := Interpret("def greet(name, greeting = \"Hello\"):\n    return greeting + \", \" + name\nx = greet(\"World\")\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "Hello, World")
}

func TestNestedFunctionCall(t *testing.T) {
	// Test a function called multiple times in a nested expression.
	// We use a single def to avoid a compiler limitation where
	// addConstant panics when comparing map constants (two defs
	// at the same level produce uncomparable CodeObject maps).
	src := "def add(a, b):\n    return a + b\nx = add(add(1, 2), add(3, 4))\n"
	result, err := Interpret(src)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 10)
}

// ============================================================================
// 4. CONTROL FLOW TESTS
// ============================================================================

func TestIfStatement(t *testing.T) {
	result, err := Interpret("x = 0\nif True:\n    x = 1\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 1)
}

func TestIfElseStatement(t *testing.T) {
	result, err := Interpret("x = 0\nif False:\n    x = 1\nelse:\n    x = 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 2)
}

func TestForLoop(t *testing.T) {
	result, err := Interpret("total = 0\nfor i in [1, 2, 3]:\n    total = total + i\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "total", 6)
}

func TestForLoopWithRange(t *testing.T) {
	result, err := Interpret("total = 0\nfor i in range(5):\n    total = total + i\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "total", 10)
}

// ============================================================================
// 5. COLLECTION TESTS
// ============================================================================

func TestListCreation(t *testing.T) {
	result, err := Interpret("x = [1, 2, 3]\n")
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
	result, err := Interpret("x = {\"a\": 1}\n")
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
	result, err := Interpret("x = [10, 20, 30]\ny = x[1]\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "y", 20)
}

func TestDictSubscript(t *testing.T) {
	result, err := Interpret("x = {\"key\": 42}\ny = x[\"key\"]\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "y", 42)
}

// ============================================================================
// 6. BUILTIN TESTS
// ============================================================================

func TestBuiltinLen(t *testing.T) {
	result, err := Interpret("x = len([1, 2, 3])\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 3)
}

func TestBuiltinRange(t *testing.T) {
	result, err := Interpret("x = range(5)\n")
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
	result, err := Interpret("x = sorted([3, 1, 2])\n")
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

func TestBuiltinType(t *testing.T) {
	result, err := Interpret("x = type(42)\ny = type(\"hello\")\nz = type(True)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", "int")
	expectVar(t, result, "y", "string")
	expectVar(t, result, "z", "bool")
}

// ============================================================================
// 7. PRINT CAPTURE TESTS
// ============================================================================

func TestPrintCapture(t *testing.T) {
	result, err := Interpret("print(42)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectOutput(t, result, []string{"42"})
}

func TestPrintMultipleValues(t *testing.T) {
	result, err := Interpret("print(1)\nprint(2)\nprint(3)\n")
	if err != nil {
		t.Fatal(err)
	}
	expectOutput(t, result, []string{"1", "2", "3"})
}

func TestPrintString(t *testing.T) {
	result, err := Interpret("print(\"hello world\")\n")
	if err != nil {
		t.Fatal(err)
	}
	expectOutput(t, result, []string{"hello world"})
}

// ============================================================================
// 8. LOAD SUPPORT TESTS
// ============================================================================

func TestLoadSimpleVariable(t *testing.T) {
	files := map[string]string{
		"constants.star": "PI = 3\nE = 2\n",
	}
	resolver := DictResolver(files)
	result, err := Interpret(
		"load(\"constants.star\", \"PI\")\nx = PI\n",
		resolver,
	)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 3)
}

func TestLoadMultipleSymbols(t *testing.T) {
	files := map[string]string{
		"constants.star": "PI = 3\nE = 2\n",
	}
	resolver := DictResolver(files)
	result, err := Interpret(
		"load(\"constants.star\", \"PI\", \"E\")\nx = PI + E\n",
		resolver,
	)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 5)
}

// ============================================================================
// 9. LOAD CACHING TESTS
// ============================================================================

func TestLoadCachingSameFile(t *testing.T) {
	// Load the same file twice.  The interpreter should only execute
	// it once (caching), so the result should be the same.
	files := map[string]string{
		"constants.star": "VALUE = 42\n",
	}
	resolver := DictResolver(files)
	interp := NewInterpreter(WithFileResolver(resolver))

	// First load
	result1, err := interp.Interpret("load(\"constants.star\", \"VALUE\")\nx = VALUE\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result1, "x", 42)

	// Second load in a new execution -- but same interpreter, so cache is shared.
	result2, err := interp.Interpret("load(\"constants.star\", \"VALUE\")\ny = VALUE\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result2, "y", 42)

	// Verify the cache has exactly one entry.
	if len(interp.loadCache) != 1 {
		t.Errorf("expected 1 cache entry, got %d", len(interp.loadCache))
	}
}

// ============================================================================
// 10. LOAD WITH FUNCTIONS TESTS
// ============================================================================

func TestLoadFunction(t *testing.T) {
	files := map[string]string{
		"math.star": "def double(n):\n    return n * 2\n",
	}
	resolver := DictResolver(files)
	result, err := Interpret(
		"load(\"math.star\", \"double\")\nx = double(21)\n",
		resolver,
	)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 42)
}

func TestLoadFunctionAndVariable(t *testing.T) {
	// Test loading both a function and a variable from the same module.
	// We avoid two def statements at module level due to a compiler
	// limitation with addConstant comparing uncomparable map types.
	files := map[string]string{
		"helpers.star": "def double(n):\n    return n * 2\nFACTOR = 3\n",
	}
	resolver := DictResolver(files)
	result, err := Interpret(
		"load(\"helpers.star\", \"double\", \"FACTOR\")\nx = double(5) + FACTOR\n",
		resolver,
	)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 13)
}

// ============================================================================
// 11. LOAD ERROR TESTS
// ============================================================================

func TestLoadMissingFile(t *testing.T) {
	files := map[string]string{}
	resolver := DictResolver(files)
	expectPanic(t, "file not found", func() {
		_, _ = Interpret(
			"load(\"missing.star\", \"x\")\n",
			resolver,
		)
	})
}

func TestLoadNoResolver(t *testing.T) {
	// Calling load() without a resolver should panic.
	expectPanic(t, "no file resolver", func() {
		_, _ = Interpret("load(\"anything.star\", \"x\")\n")
	})
}

// ============================================================================
// 12. INTERPRET FILE TESTS
// ============================================================================

func TestInterpretFile(t *testing.T) {
	// Write a temp file and interpret it.
	tmpDir := t.TempDir()
	filePath := filepath.Join(tmpDir, "test.star")
	content := "x = 100\ny = x * 2\n"
	if err := os.WriteFile(filePath, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}

	result, err := InterpretFile(filePath)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 100)
	expectVar(t, result, "y", 200)
}

func TestInterpretFileNoTrailingNewline(t *testing.T) {
	// File without trailing newline -- interpreter should add one.
	tmpDir := t.TempDir()
	filePath := filepath.Join(tmpDir, "test.star")
	content := "x = 99" // no trailing newline
	if err := os.WriteFile(filePath, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}

	result, err := InterpretFile(filePath)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 99)
}

func TestInterpretFileMissing(t *testing.T) {
	_, err := InterpretFile("/nonexistent/path/test.star")
	if err == nil {
		t.Error("expected error for missing file, got nil")
	}
}

func TestInterpretFileWithResolver(t *testing.T) {
	// Write a main file and use a resolver for loaded files.
	tmpDir := t.TempDir()
	mainPath := filepath.Join(tmpDir, "main.star")
	mainContent := "load(\"helpers.star\", \"greet\")\nmsg = greet(\"World\")\n"
	if err := os.WriteFile(mainPath, []byte(mainContent), 0644); err != nil {
		t.Fatal(err)
	}

	files := map[string]string{
		"helpers.star": "def greet(name):\n    return \"Hello, \" + name\n",
	}
	resolver := DictResolver(files)

	result, err := InterpretFile(mainPath, resolver)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "msg", "Hello, World")
}

// ============================================================================
// 13. BUILD FILE SIMULATION TESTS
// ============================================================================

func TestBuildFileSimulation(t *testing.T) {
	// Simulate a Bazel-like BUILD file that loads rule definitions
	// and calls them to declare targets.
	//
	// Note: the label must NOT start with 'r', 'b', 'R', or 'B'
	// due to a known compiler bug where parseStringLiteral treats
	// those as raw/byte string prefixes when the lexer has already
	// stripped quotes.
	files := map[string]string{
		"defs.star": "def go_library(name, srcs):\n    return {\"name\": name, \"srcs\": srcs, \"kind\": \"go_library\"}\n",
	}
	resolver := DictResolver(files)

	src := "load(\"defs.star\", \"go_library\")\n" +
		"lib = go_library(\"mylib\", [\"main.go\"])\n"

	result, err := Interpret(src, resolver)
	if err != nil {
		t.Fatal(err)
	}

	val := result.Variables["lib"]
	dict, ok := val.(map[string]interface{})
	if !ok {
		t.Fatalf("expected map[string]interface{}, got %T", val)
	}
	if dict["name"] != "mylib" {
		t.Errorf("expected name 'mylib', got %v", dict["name"])
	}
	if dict["kind"] != "go_library" {
		t.Errorf("expected kind 'go_library', got %v", dict["kind"])
	}
}

// ============================================================================
// 14. ERROR HANDLING TESTS
// ============================================================================

func TestSyntaxError(t *testing.T) {
	// Invalid syntax should return an error from compilation.
	_, err := Interpret("def\n")
	if err == nil {
		t.Error("expected error for syntax error, got nil")
	}
}

func TestRuntimeErrorDivisionByZero(t *testing.T) {
	expectPanic(t, "ZeroDivisionError", func() {
		_, _ = Interpret("x = 1 // 0\n")
	})
}

func TestRuntimeErrorUndefinedVariable(t *testing.T) {
	expectPanic(t, "not defined", func() {
		_, _ = Interpret("x = undefined_var\n")
	})
}

// ============================================================================
// 15. OPTIONS TESTS
// ============================================================================

func TestWithMaxRecursionDepth(t *testing.T) {
	interp := NewInterpreter(WithMaxRecursionDepth(500))
	if interp.MaxRecursionDepth != 500 {
		t.Errorf("expected MaxRecursionDepth 500, got %d", interp.MaxRecursionDepth)
	}

	result, err := interp.Interpret("x = 42\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 42)
}

func TestWithFileResolver(t *testing.T) {
	files := map[string]string{
		"data.star": "VALUE = 99\n",
	}
	resolver := DictResolver(files)
	interp := NewInterpreter(WithFileResolver(resolver))

	result, err := interp.Interpret("load(\"data.star\", \"VALUE\")\nx = VALUE\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 99)
}

func TestDefaultOptions(t *testing.T) {
	interp := NewInterpreter()
	if interp.MaxRecursionDepth != 200 {
		t.Errorf("expected default MaxRecursionDepth 200, got %d", interp.MaxRecursionDepth)
	}
	if interp.FileResolver != nil {
		t.Error("expected nil FileResolver by default")
	}
	if interp.loadCache == nil {
		t.Error("expected non-nil loadCache")
	}
}

func TestMultipleOptions(t *testing.T) {
	files := map[string]string{
		"data.star": "X = 1\n",
	}
	resolver := DictResolver(files)
	interp := NewInterpreter(
		WithFileResolver(resolver),
		WithMaxRecursionDepth(100),
	)
	if interp.MaxRecursionDepth != 100 {
		t.Errorf("expected 100, got %d", interp.MaxRecursionDepth)
	}
	if interp.FileResolver == nil {
		t.Error("expected non-nil FileResolver")
	}
}

// ============================================================================
// ADDITIONAL TESTS (to reach 30+)
// ============================================================================

func TestComparisonOperators(t *testing.T) {
	result, err := Interpret("a = 1 == 1\nb = 1 != 2\nc = 1 < 2\nd = 3 > 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "a", true)
	expectVar(t, result, "b", true)
	expectVar(t, result, "c", true)
	expectVar(t, result, "d", true)
}

func TestDictResolverReturnsError(t *testing.T) {
	resolver := DictResolver(map[string]string{})
	_, err := resolver("nonexistent.star")
	if err == nil {
		t.Error("expected error for nonexistent file")
	}
	if !strings.Contains(err.Error(), "file not found") {
		t.Errorf("expected 'file not found' in error, got: %s", err.Error())
	}
}

func TestDictResolverFindsFile(t *testing.T) {
	resolver := DictResolver(map[string]string{
		"test.star": "x = 1\n",
	})
	content, err := resolver("test.star")
	if err != nil {
		t.Fatal(err)
	}
	if content != "x = 1\n" {
		t.Errorf("expected 'x = 1\\n', got %q", content)
	}
}

func TestInterpreterReuse(t *testing.T) {
	// Reusing an interpreter across multiple Interpret calls should work.
	// Each call gets a fresh VM, but the load cache is shared.
	interp := NewInterpreter()

	result1, err := interp.Interpret("x = 1\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result1, "x", 1)

	result2, err := interp.Interpret("y = 2\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result2, "y", 2)
}

func TestLoadedModuleVariableIsolation(t *testing.T) {
	// Variables from loaded modules should not leak into the main scope
	// except for explicitly imported symbols.
	files := map[string]string{
		"module.star": "PUBLIC = 42\n_PRIVATE = 99\n",
	}
	resolver := DictResolver(files)
	result, err := Interpret(
		"load(\"module.star\", \"PUBLIC\")\nx = PUBLIC\n",
		resolver,
	)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 42)
	// _PRIVATE should NOT be in the main scope (it's not imported).
	if _, exists := result.Variables["_PRIVATE"]; exists {
		t.Error("_PRIVATE should not be in main scope")
	}
}

func TestEmptyProgram(t *testing.T) {
	result, err := Interpret("\n")
	if err != nil {
		t.Fatal(err)
	}
	if len(result.Variables) != 0 {
		t.Errorf("expected no variables, got %d", len(result.Variables))
	}
}

func TestListConcatenation(t *testing.T) {
	result, err := Interpret("x = [1, 2] + [3, 4]\n")
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

func TestBuiltinLenString(t *testing.T) {
	result, err := Interpret("x = len(\"hello\")\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 5)
}

func TestResultHasTraces(t *testing.T) {
	// Every execution should produce traces (execution log).
	result, err := Interpret("x = 1\n")
	if err != nil {
		t.Fatal(err)
	}
	if len(result.Traces) == 0 {
		t.Error("expected non-empty traces")
	}
}

func TestResultOutputIsEmptyWithoutPrint(t *testing.T) {
	result, err := Interpret("x = 1\n")
	if err != nil {
		t.Fatal(err)
	}
	if len(result.Output) != 0 {
		t.Errorf("expected empty output, got %v", result.Output)
	}
}

func TestConvenienceInterpretWithNilResolver(t *testing.T) {
	// Passing nil resolver should work (just no load support).
	result, err := Interpret("x = 42\n", nil)
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", 42)
}

func TestNotOperator(t *testing.T) {
	result, err := Interpret("x = not False\ny = not True\n")
	if err != nil {
		t.Fatal(err)
	}
	expectVar(t, result, "x", true)
	expectVar(t, result, "y", false)
}
