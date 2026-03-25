package veriloglexer

import (
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// Simple `define
// ---------------------------------------------------------------------------

func TestDefineSimple(t *testing.T) {
	source := "`define WIDTH 8\nwire [`WIDTH-1:0] data;"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "8") {
		t.Errorf("Expected `WIDTH to expand to 8, got: %s", result)
	}
	if strings.Contains(result, "`WIDTH") {
		t.Errorf("Expected `WIDTH to be replaced, got: %s", result)
	}
}

func TestDefineEmptyBody(t *testing.T) {
	source := "`define SYNTHESIS\n`ifdef SYNTHESIS\nwire a;\n`endif"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "wire a;") {
		t.Errorf("Expected wire a; to be included, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// Parameterized `define
// ---------------------------------------------------------------------------
//
// Parameterized macros take arguments, like function calls:
//
//   `define MAX(a, b) ((a) > (b) ? (a) : (b))
//   `MAX(x, y) → ((x) > (y) ? (x) : (y))

func TestDefineWithParams(t *testing.T) {
	source := "`define MAX(a, b) ((a) > (b) ? (a) : (b))\nassign z = `MAX(x, y);"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "((x) > (y) ? (x) : (y))") {
		t.Errorf("Expected parameterized macro expansion, got: %s", result)
	}
}

func TestDefineWithNestedParens(t *testing.T) {
	source := "`define ADD(a, b) (a + b)\nassign z = `ADD((x+1), y);"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "((x+1) + y)") {
		t.Errorf("Expected nested paren handling, got: %s", result)
	}
}

func TestParameterizedMacroWithoutArgs(t *testing.T) {
	// A parameterized macro referenced without args should be kept as-is
	source := "`define FOO(a) (a+1)\n`FOO"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "`FOO") {
		t.Errorf("Expected `FOO to remain when called without args, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// `undef
// ---------------------------------------------------------------------------

func TestUndef(t *testing.T) {
	source := "`define WIDTH 8\n`undef WIDTH\nwire [`WIDTH-1:0] data;"
	result := VerilogPreprocess(source)

	// After undef, `WIDTH should remain unexpanded
	if !strings.Contains(result, "`WIDTH") {
		t.Errorf("Expected `WIDTH to remain after undef, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// `ifdef / `endif
// ---------------------------------------------------------------------------

func TestIfdefDefined(t *testing.T) {
	source := "`define USE_CACHE\n`ifdef USE_CACHE\nwire cache;\n`endif"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "wire cache;") {
		t.Errorf("Expected wire cache; to be included (defined), got: %s", result)
	}
}

func TestIfdefNotDefined(t *testing.T) {
	source := "`ifdef USE_CACHE\nwire cache;\n`endif"
	result := VerilogPreprocess(source)

	if strings.Contains(result, "wire cache;") {
		t.Errorf("Expected wire cache; to be excluded (not defined), got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// `ifndef
// ---------------------------------------------------------------------------

func TestIfndefNotDefined(t *testing.T) {
	source := "`ifndef USE_CACHE\nwire no_cache;\n`endif"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "wire no_cache;") {
		t.Errorf("Expected wire no_cache; to be included (not defined), got: %s", result)
	}
}

func TestIfndefDefined(t *testing.T) {
	source := "`define USE_CACHE\n`ifndef USE_CACHE\nwire no_cache;\n`endif"
	result := VerilogPreprocess(source)

	if strings.Contains(result, "wire no_cache;") {
		t.Errorf("Expected wire no_cache; to be excluded (defined), got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// `ifdef / `else / `endif
// ---------------------------------------------------------------------------

func TestIfdefElse(t *testing.T) {
	source := "`define USE_CACHE\n`ifdef USE_CACHE\nwire cache;\n`else\nwire no_cache;\n`endif"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "wire cache;") {
		t.Errorf("Expected wire cache; to be included, got: %s", result)
	}
	if strings.Contains(result, "wire no_cache;") {
		t.Errorf("Expected wire no_cache; to be excluded, got: %s", result)
	}
}

func TestIfdefElseNotDefined(t *testing.T) {
	source := "`ifdef USE_CACHE\nwire cache;\n`else\nwire no_cache;\n`endif"
	result := VerilogPreprocess(source)

	if strings.Contains(result, "wire cache;") {
		t.Errorf("Expected wire cache; to be excluded, got: %s", result)
	}
	if !strings.Contains(result, "wire no_cache;") {
		t.Errorf("Expected wire no_cache; to be included, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// Nested Conditionals
// ---------------------------------------------------------------------------
//
// Nested conditionals must maintain their own condition stack entries.
// The inner conditional is only evaluated if the outer one is active.
//
//   `ifdef A           ← active if A defined
//     `ifdef B         ← active if A AND B defined
//       wire ab;       ← included only if both defined
//     `endif
//   `endif

func TestNestedConditionals(t *testing.T) {
	source := "`define A\n`define B\n`ifdef A\n`ifdef B\nwire ab;\n`endif\n`endif"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "wire ab;") {
		t.Errorf("Expected wire ab; when both A and B defined, got: %s", result)
	}
}

func TestNestedConditionalsInnerFalse(t *testing.T) {
	source := "`define A\n`ifdef A\n`ifdef B\nwire ab;\n`endif\n`endif"
	result := VerilogPreprocess(source)

	if strings.Contains(result, "wire ab;") {
		t.Errorf("Expected wire ab; excluded when B not defined, got: %s", result)
	}
}

func TestNestedConditionalsOuterFalse(t *testing.T) {
	source := "`define B\n`ifdef A\n`ifdef B\nwire ab;\n`endif\n`endif"
	result := VerilogPreprocess(source)

	if strings.Contains(result, "wire ab;") {
		t.Errorf("Expected wire ab; excluded when A not defined, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// `include (stubbed)
// ---------------------------------------------------------------------------

func TestIncludeStubbed(t *testing.T) {
	source := "`include \"types.v\""
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "/* `include \"types.v\" — not resolved */") {
		t.Errorf("Expected stubbed include comment, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// `timescale (stripped)
// ---------------------------------------------------------------------------

func TestTimescaleStripped(t *testing.T) {
	source := "`timescale 1ns/1ps\nwire a;"
	result := VerilogPreprocess(source)

	if strings.Contains(result, "timescale") {
		t.Errorf("Expected timescale to be stripped, got: %s", result)
	}
	if !strings.Contains(result, "wire a;") {
		t.Errorf("Expected wire a; to remain, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// Line Number Preservation
// ---------------------------------------------------------------------------
//
// When directives are processed, they are replaced with empty lines to
// preserve line numbers for error reporting.

func TestLineNumberPreservation(t *testing.T) {
	source := "`define A 1\n`ifdef A\nwire a;\n`endif\nwire b;"
	result := VerilogPreprocess(source)

	lines := strings.Split(result, "\n")
	if len(lines) != 5 {
		t.Fatalf("Expected 5 lines (preserved), got %d: %v", len(lines), lines)
	}
	// Lines 1, 2, 4 should be empty (directive lines)
	if lines[0] != "" {
		t.Errorf("Line 1 should be empty (define), got %q", lines[0])
	}
	if lines[1] != "" {
		t.Errorf("Line 2 should be empty (ifdef), got %q", lines[1])
	}
	if lines[2] != "wire a;" {
		t.Errorf("Line 3 should be 'wire a;', got %q", lines[2])
	}
	if lines[3] != "" {
		t.Errorf("Line 4 should be empty (endif), got %q", lines[3])
	}
	if lines[4] != "wire b;" {
		t.Errorf("Line 5 should be 'wire b;', got %q", lines[4])
	}
}

// ---------------------------------------------------------------------------
// Predefined Macros
// ---------------------------------------------------------------------------

func TestPredefinedMacros(t *testing.T) {
	source := "`ifdef SIMULATION\nwire sim;\n`endif"
	result := VerilogPreprocessWithDefines(source, map[string]string{"SIMULATION": ""})

	if !strings.Contains(result, "wire sim;") {
		t.Errorf("Expected wire sim; with predefined SIMULATION, got: %s", result)
	}
}

func TestPredefinedMacroValue(t *testing.T) {
	source := "wire [`WIDTH-1:0] data;"
	result := VerilogPreprocessWithDefines(source, map[string]string{"WIDTH": "16"})

	if !strings.Contains(result, "16") {
		t.Errorf("Expected `WIDTH to expand to 16, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// Undefined Macro References
// ---------------------------------------------------------------------------

func TestUndefinedMacroKept(t *testing.T) {
	source := "wire `UNDEFINED data;"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "`UNDEFINED") {
		t.Errorf("Expected undefined macro to remain, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// Multiple Macros on One Line
// ---------------------------------------------------------------------------

func TestMultipleMacrosOneLine(t *testing.T) {
	source := "`define A 1\n`define B 2\nassign x = `A + `B;"
	result := VerilogPreprocess(source)

	if !strings.Contains(result, "assign x = 1 + 2;") {
		t.Errorf("Expected both macros expanded, got: %s", result)
	}
}

// ---------------------------------------------------------------------------
// Empty Source
// ---------------------------------------------------------------------------

func TestEmptySource(t *testing.T) {
	result := VerilogPreprocess("")
	if result != "" {
		t.Errorf("Expected empty result for empty source, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Helper function tests
// ---------------------------------------------------------------------------

func TestAllTrue(t *testing.T) {
	if !allTrue([]bool{true, true, true}) {
		t.Error("Expected allTrue([true, true, true]) = true")
	}
	if allTrue([]bool{true, false, true}) {
		t.Error("Expected allTrue([true, false, true]) = false")
	}
	if !allTrue([]bool{}) {
		t.Error("Expected allTrue([]) = true")
	}
}

func TestSplitAndTrim(t *testing.T) {
	result := splitAndTrim("  a , b , c  ", ",")
	if len(result) != 3 || result[0] != "a" || result[1] != "b" || result[2] != "c" {
		t.Errorf("Expected [a, b, c], got %v", result)
	}
}

func TestExtractMacroArgs(t *testing.T) {
	text := "a, (b+c), d) rest"
	args, endPos := extractMacroArgs(text, 0)
	if args != "a, (b+c), d" {
		t.Errorf("Expected 'a, (b+c), d', got %q", args)
	}
	if endPos != 12 {
		t.Errorf("Expected endPos 12, got %d", endPos)
	}
}
