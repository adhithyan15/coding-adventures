package clibuilder

// =========================================================================
// v1.1 Feature Tests
// =========================================================================
//
// This file tests the four backwards-compatible features introduced in
// CLI Builder v1.1:
//
//   Feature 1: Count type — a flag that increments on each occurrence
//   Feature 2: Enum optional values (default_when_present)
//   Feature 3: Flag presence detection (ExplicitFlags)
//   Feature 4: int64 range validation
//
// Each feature has its own section with multiple test cases covering
// normal usage, edge cases, and error conditions.

import (
	"strings"
	"testing"
)

// =========================================================================
// Feature 1: Count Type
// =========================================================================
//
// Count flags work like boolean flags in that they consume no value token.
// The key difference: instead of being set to true, each occurrence
// increments an int64 counter.
//
//   -v       → 1
//   -vv      → 2
//   -vvv     → 3
//   --verbose --verbose → 2
//
// When absent, count flags default to int64(0).

// countSpec defines a CLI with a count-type flag for verbosity.
const countSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "mytool",
  "description": "A tool with count flags",
  "flags": [
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Increase verbosity level",
      "type": "count"
    },
    {
      "id": "quiet",
      "short": "q",
      "long": "quiet",
      "description": "Decrease verbosity level",
      "type": "count"
    },
    {
      "id": "all",
      "short": "a",
      "long": "all",
      "description": "Show all files",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "path",
      "display_name": "PATH",
      "description": "File or directory path",
      "type": "path",
      "required": false
    }
  ]
}`

func TestCountFlag_SingleOccurrence(t *testing.T) {
	// -v should set verbose to int64(1)
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-v"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["verbose"] != int64(1) {
		t.Errorf("expected verbose=1, got %v (%T)", r.Flags["verbose"], r.Flags["verbose"])
	}
}

func TestCountFlag_StackedTriple(t *testing.T) {
	// -vvv should set verbose to int64(3)
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-vvv"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["verbose"] != int64(3) {
		t.Errorf("expected verbose=3, got %v (%T)", r.Flags["verbose"], r.Flags["verbose"])
	}
}

func TestCountFlag_LongFormRepeated(t *testing.T) {
	// --verbose --verbose should set verbose to int64(2)
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "--verbose", "--verbose"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["verbose"] != int64(2) {
		t.Errorf("expected verbose=2, got %v (%T)", r.Flags["verbose"], r.Flags["verbose"])
	}
}

func TestCountFlag_Absent(t *testing.T) {
	// When absent, count flags default to int64(0)
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["verbose"] != int64(0) {
		t.Errorf("expected verbose=int64(0), got %v (%T)", r.Flags["verbose"], r.Flags["verbose"])
	}
}

func TestCountFlag_MixedWithBoolean(t *testing.T) {
	// -avv should set all=true and verbose=2
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-avv"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["all"] != true {
		t.Errorf("expected all=true, got %v", r.Flags["all"])
	}
	if r.Flags["verbose"] != int64(2) {
		t.Errorf("expected verbose=2, got %v (%T)", r.Flags["verbose"], r.Flags["verbose"])
	}
}

func TestCountFlag_MixedShortAndLong(t *testing.T) {
	// -v --verbose -v should set verbose to 3
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-v", "--verbose", "-v"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["verbose"] != int64(3) {
		t.Errorf("expected verbose=3, got %v (%T)", r.Flags["verbose"], r.Flags["verbose"])
	}
}

func TestCountFlag_MultipleCountFlags(t *testing.T) {
	// -vvqq should set verbose=2 and quiet=2
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-vvqq"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["verbose"] != int64(2) {
		t.Errorf("expected verbose=2, got %v", r.Flags["verbose"])
	}
	if r.Flags["quiet"] != int64(2) {
		t.Errorf("expected quiet=2, got %v", r.Flags["quiet"])
	}
}

func TestCountFlag_WithPositionalArg(t *testing.T) {
	// -vv /tmp should set verbose=2 and path="/tmp"
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-vv", "/tmp"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["verbose"] != int64(2) {
		t.Errorf("expected verbose=2, got %v", r.Flags["verbose"])
	}
	if r.Arguments["path"] != "/tmp" {
		t.Errorf("expected path=/tmp, got %v", r.Arguments["path"])
	}
}

// =========================================================================
// Feature 2: Enum Optional Values (default_when_present)
// =========================================================================
//
// When an enum flag has `default_when_present`, it can be used without a
// value. If the next token is a valid enum value, consume it; otherwise
// use default_when_present.
//
//   --color          → "always" (default_when_present)
//   --color=always   → "always" (explicit)
//   --color=never    → "never"  (explicit)
//   --color auto     → "auto"   (next token is valid enum)
//   --color somefile → "always" + somefile as positional

const dwpSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "grep",
  "description": "Search for patterns",
  "flags": [
    {
      "id": "color",
      "long": "color",
      "description": "Colorize output",
      "type": "enum",
      "enum_values": ["always", "never", "auto"],
      "default_when_present": "always"
    },
    {
      "id": "ignore-case",
      "short": "i",
      "long": "ignore-case",
      "description": "Case insensitive search",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "pattern",
      "display_name": "PATTERN",
      "description": "Search pattern",
      "type": "string",
      "required": false
    }
  ]
}`

func TestDWP_ExplicitValueEquals(t *testing.T) {
	// --color=never should use "never" (explicit value via =)
	p, err := NewParserFromBytes([]byte(dwpSpec), []string{"grep", "--color=never"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["color"] != "never" {
		t.Errorf("expected color=never, got %v", r.Flags["color"])
	}
}

func TestDWP_NoValueLastToken(t *testing.T) {
	// --color (last token) should use default_when_present = "always"
	p, err := NewParserFromBytes([]byte(dwpSpec), []string{"grep", "--color"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["color"] != "always" {
		t.Errorf("expected color=always (default_when_present), got %v", r.Flags["color"])
	}
}

func TestDWP_NextTokenIsValidEnum(t *testing.T) {
	// --color auto should consume "auto" as the value
	p, err := NewParserFromBytes([]byte(dwpSpec), []string{"grep", "--color", "auto"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["color"] != "auto" {
		t.Errorf("expected color=auto, got %v", r.Flags["color"])
	}
}

func TestDWP_NextTokenIsNotValidEnum(t *testing.T) {
	// --color somefile → use dwp "always", somefile is positional
	p, err := NewParserFromBytes([]byte(dwpSpec), []string{"grep", "--color", "somefile"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["color"] != "always" {
		t.Errorf("expected color=always (dwp fallback), got %v", r.Flags["color"])
	}
	if r.Arguments["pattern"] != "somefile" {
		t.Errorf("expected pattern=somefile, got %v", r.Arguments["pattern"])
	}
}

func TestDWP_NextTokenIsFlag(t *testing.T) {
	// --color -i → use dwp "always", -i is a separate flag
	p, err := NewParserFromBytes([]byte(dwpSpec), []string{"grep", "--color", "-i"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if r.Flags["color"] != "always" {
		t.Errorf("expected color=always (dwp, next is flag), got %v", r.Flags["color"])
	}
	if r.Flags["ignore-case"] != true {
		t.Errorf("expected ignore-case=true, got %v", r.Flags["ignore-case"])
	}
}

func TestDWP_InvalidEqualsValue(t *testing.T) {
	// --color=purple should produce an invalid_enum_value error
	p, err := NewParserFromBytes([]byte(dwpSpec), []string{"grep", "--color=purple"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected error for invalid enum value, got nil")
	}
	pe := err.(*ParseErrors)
	if pe.Errors[0].ErrorType != ErrInvalidEnumValue {
		t.Errorf("expected invalid_enum_value error, got %s", pe.Errors[0].ErrorType)
	}
}

func TestDWP_SpecValidation_InvalidDWPValue(t *testing.T) {
	// default_when_present must be a valid enum value
	badSpec := `{
	  "cli_builder_spec_version": "1.0",
	  "name": "test",
	  "description": "test",
	  "flags": [
	    {
	      "id": "color",
	      "long": "color",
	      "description": "Colorize",
	      "type": "enum",
	      "enum_values": ["always", "never"],
	      "default_when_present": "maybe"
	    }
	  ]
	}`
	_, err := NewParserFromBytes([]byte(badSpec), []string{"test"})
	if err == nil {
		t.Fatal("expected spec error for invalid default_when_present value")
	}
	if !strings.Contains(err.Error(), "default_when_present") {
		t.Errorf("expected error about default_when_present, got: %v", err)
	}
}

func TestDWP_SpecValidation_NonEnumType(t *testing.T) {
	// default_when_present is only valid for enum flags
	badSpec := `{
	  "cli_builder_spec_version": "1.0",
	  "name": "test",
	  "description": "test",
	  "flags": [
	    {
	      "id": "name",
	      "long": "name",
	      "description": "Name",
	      "type": "string",
	      "default_when_present": "default"
	    }
	  ]
	}`
	_, err := NewParserFromBytes([]byte(badSpec), []string{"test"})
	if err == nil {
		t.Fatal("expected spec error for non-enum default_when_present")
	}
	if !strings.Contains(err.Error(), "default_when_present") {
		t.Errorf("expected error about default_when_present, got: %v", err)
	}
}

// =========================================================================
// Feature 3: Flag Presence Detection (ExplicitFlags)
// =========================================================================
//
// ExplicitFlags tracks which flags were explicitly set by the user in argv.
// Flags filled with defaults do NOT appear in ExplicitFlags.

func TestExplicitFlags_BasicPresence(t *testing.T) {
	// -v should appear in ExplicitFlags; quiet should not
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-v"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if !containsString(r.ExplicitFlags, "verbose") {
		t.Errorf("expected ExplicitFlags to contain 'verbose', got %v", r.ExplicitFlags)
	}
	if containsString(r.ExplicitFlags, "quiet") {
		t.Errorf("expected ExplicitFlags NOT to contain 'quiet', got %v", r.ExplicitFlags)
	}
}

func TestExplicitFlags_CountMultipleAppearances(t *testing.T) {
	// -vvv should produce ["verbose", "verbose", "verbose"] in ExplicitFlags
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-vvv"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	count := 0
	for _, f := range r.ExplicitFlags {
		if f == "verbose" {
			count++
		}
	}
	if count != 3 {
		t.Errorf("expected 'verbose' to appear 3 times in ExplicitFlags, got %d (flags: %v)", count, r.ExplicitFlags)
	}
}

func TestExplicitFlags_BooleanFlag(t *testing.T) {
	// -a should appear in ExplicitFlags
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "-a"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if !containsString(r.ExplicitFlags, "all") {
		t.Errorf("expected ExplicitFlags to contain 'all', got %v", r.ExplicitFlags)
	}
}

func TestExplicitFlags_Empty(t *testing.T) {
	// No flags passed → ExplicitFlags should be empty/nil
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if len(r.ExplicitFlags) != 0 {
		t.Errorf("expected empty ExplicitFlags, got %v", r.ExplicitFlags)
	}
}

func TestExplicitFlags_ValueFlag(t *testing.T) {
	// Test that value-taking flags are also tracked
	spec := `{
	  "cli_builder_spec_version": "1.0",
	  "name": "test",
	  "description": "test tool",
	  "flags": [
	    {
	      "id": "output",
	      "short": "o",
	      "long": "output",
	      "description": "Output file",
	      "type": "string"
	    },
	    {
	      "id": "count",
	      "short": "c",
	      "long": "count",
	      "description": "Count",
	      "type": "integer"
	    }
	  ]
	}`
	p, err := NewParserFromBytes([]byte(spec), []string{"test", "-o", "file.txt"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if !containsString(r.ExplicitFlags, "output") {
		t.Errorf("expected ExplicitFlags to contain 'output', got %v", r.ExplicitFlags)
	}
	// count was not passed, should not be in ExplicitFlags
	if containsString(r.ExplicitFlags, "count") {
		t.Errorf("expected ExplicitFlags NOT to contain 'count', got %v", r.ExplicitFlags)
	}
}

func TestExplicitFlags_LongEqualsValue(t *testing.T) {
	// --output=file.txt should track output in ExplicitFlags
	spec := `{
	  "cli_builder_spec_version": "1.0",
	  "name": "test",
	  "description": "test tool",
	  "flags": [
	    {
	      "id": "output",
	      "short": "o",
	      "long": "output",
	      "description": "Output file",
	      "type": "string"
	    }
	  ]
	}`
	p, err := NewParserFromBytes([]byte(spec), []string{"test", "--output=file.txt"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if !containsString(r.ExplicitFlags, "output") {
		t.Errorf("expected ExplicitFlags to contain 'output', got %v", r.ExplicitFlags)
	}
}

func TestExplicitFlags_DWPFlag(t *testing.T) {
	// --color (with dwp) should appear in ExplicitFlags
	p, err := NewParserFromBytes([]byte(dwpSpec), []string{"grep", "--color"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("parse error: %v", err)
	}
	r := result.(*ParseResult)
	if !containsString(r.ExplicitFlags, "color") {
		t.Errorf("expected ExplicitFlags to contain 'color', got %v", r.ExplicitFlags)
	}
}

// =========================================================================
// Feature 4: int64 Range Validation
// =========================================================================
//
// Integer values outside [-2^63, 2^63-1] should produce an invalid_value
// error with a specific range message.

func TestInt64Range_MaxValue(t *testing.T) {
	// 9223372036854775807 (max int64) should parse fine
	val, err := coerceValue("9223372036854775807", "integer", nil)
	if err != nil {
		t.Fatalf("unexpected error for max int64: %v", err)
	}
	if val != int64(9223372036854775807) {
		t.Errorf("expected max int64, got %v", val)
	}
}

func TestInt64Range_MinValue(t *testing.T) {
	// -9223372036854775808 (min int64) should parse fine
	val, err := coerceValue("-9223372036854775808", "integer", nil)
	if err != nil {
		t.Fatalf("unexpected error for min int64: %v", err)
	}
	if val != int64(-9223372036854775808) {
		t.Errorf("expected min int64, got %v", val)
	}
}

func TestInt64Range_Overflow(t *testing.T) {
	// 9223372036854775808 (max int64 + 1) should produce a range error
	_, err := coerceValue("9223372036854775808", "integer", nil)
	if err == nil {
		t.Fatal("expected error for int64 overflow")
	}
	if !strings.Contains(err.Error(), "out of range") {
		t.Errorf("expected range error message, got: %v", err)
	}
}

func TestInt64Range_Underflow(t *testing.T) {
	// -9223372036854775809 (min int64 - 1) should produce a range error
	_, err := coerceValue("-9223372036854775809", "integer", nil)
	if err == nil {
		t.Fatal("expected error for int64 underflow")
	}
	if !strings.Contains(err.Error(), "out of range") {
		t.Errorf("expected range error message, got: %v", err)
	}
}

func TestInt64Range_VeryLargeNumber(t *testing.T) {
	// A very large number should produce a range error
	_, err := coerceValue("99999999999999999999999999", "integer", nil)
	if err == nil {
		t.Fatal("expected error for very large number")
	}
	if !strings.Contains(err.Error(), "out of range") {
		t.Errorf("expected range error message, got: %v", err)
	}
}

func TestInt64Range_NotANumber(t *testing.T) {
	// Non-numeric string should produce "not a valid integer", not range error
	_, err := coerceValue("abc", "integer", nil)
	if err == nil {
		t.Fatal("expected error for non-numeric string")
	}
	if !strings.Contains(err.Error(), "not a valid integer") {
		t.Errorf("expected 'not a valid integer' message, got: %v", err)
	}
}

func TestInt64Range_IntegrationWithParser(t *testing.T) {
	// Integration test: overflow in a full parse should produce invalid_value error
	spec := `{
	  "cli_builder_spec_version": "1.0",
	  "name": "test",
	  "description": "test tool",
	  "flags": [
	    {
	      "id": "count",
	      "short": "c",
	      "long": "count",
	      "description": "Number of items",
	      "type": "integer"
	    }
	  ]
	}`
	p, err := NewParserFromBytes([]byte(spec), []string{"test", "--count=99999999999999999999"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected parse error for integer overflow")
	}
	pe := err.(*ParseErrors)
	if pe.Errors[0].ErrorType != ErrInvalidValue {
		t.Errorf("expected invalid_value error, got %s: %s", pe.Errors[0].ErrorType, pe.Errors[0].Message)
	}
}

// =========================================================================
// Count type in help output
// =========================================================================

func TestCountFlag_HelpOutput(t *testing.T) {
	// Count flags should not show a value placeholder in help
	p, err := NewParserFromBytes([]byte(countSpec), []string{"mytool", "--help"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, _ := p.Parse()
	hr := result.(*HelpResult)
	// -v/--verbose should NOT have <COUNT> or <VALUE> suffix
	if strings.Contains(hr.Text, "<COUNT>") {
		t.Error("count flags should not show <COUNT> in help")
	}
}

func TestDWP_HelpOutput(t *testing.T) {
	// Enum flags with default_when_present should show [=VALUE]
	p, err := NewParserFromBytes([]byte(dwpSpec), []string{"grep", "--help"})
	if err != nil {
		t.Fatalf("spec error: %v", err)
	}
	result, _ := p.Parse()
	hr := result.(*HelpResult)
	if !strings.Contains(hr.Text, "[=") {
		t.Errorf("expected [=VALUE] for dwp flag in help, got:\n%s", hr.Text)
	}
}

// =========================================================================
// Count type coercion
// =========================================================================

func TestCoerce_Count(t *testing.T) {
	// Count can be coerced from a string (e.g., --count=3)
	val, err := coerceValue("5", "count", nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != int64(5) {
		t.Errorf("expected int64(5), got %v (%T)", val, val)
	}
}

func TestCoerce_Count_Invalid(t *testing.T) {
	_, err := coerceValue("abc", "count", nil)
	if err == nil {
		t.Fatal("expected error for invalid count value")
	}
}

// =========================================================================
// Count type interaction with isPresent
// =========================================================================

func TestCountFlag_IsPresent_Zero(t *testing.T) {
	// int64(0) should be considered not present (for constraint checks)
	if isPresent(int64(0)) {
		t.Error("expected int64(0) to be considered not present")
	}
}

func TestCountFlag_IsPresent_Positive(t *testing.T) {
	// int64(1) should be considered present
	if !isPresent(int64(1)) {
		t.Error("expected int64(1) to be considered present")
	}
}

// =========================================================================
// Helpers
// =========================================================================

func containsString(slice []string, s string) bool {
	for _, item := range slice {
		if item == s {
			return true
		}
	}
	return false
}
