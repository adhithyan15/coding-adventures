package clibuilder

import (
	"testing"
)

// =========================================================================
// Token classifier tests
// =========================================================================
//
// These tests verify that the TokenClassifier correctly identifies all nine
// token types, including the longest-match-first disambiguation.

// buildTestFlags creates a slice of flag definitions for use in tests.
func buildTestFlags() []map[string]any {
	return []map[string]any{
		{"id": "long-listing", "short": "l", "long": "long-listing", "description": "long listing", "type": "boolean"},
		{"id": "all", "short": "a", "long": "all", "description": "show all", "type": "boolean"},
		{"id": "human-readable", "short": "h", "long": "human-readable", "description": "human readable", "type": "boolean"},
		{"id": "output", "short": "o", "long": "output", "description": "output file", "type": "string", "value_name": "FILE"},
		{"id": "count", "short": "c", "long": "count", "description": "count lines", "type": "integer"},
		// SDL flags for longest-match-first tests
		{"id": "classpath", "single_dash_long": "classpath", "description": "classpath", "type": "string"},
		{"id": "cp", "single_dash_long": "cp", "description": "cp short", "type": "string"},
		{"id": "verbose", "short": "v", "long": "verbose", "description": "verbose", "type": "boolean"},
	}
}

func TestTokenClassifier_EndOfFlags(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("--")
	if ev.Kind != TokenEndOfFlags {
		t.Errorf("expected end_of_flags, got %q", ev.Kind)
	}
}

func TestTokenClassifier_SingleDash_IsPositional(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-")
	if ev.Kind != TokenPositional {
		t.Errorf("expected positional for '-', got %q", ev.Kind)
	}
}

func TestTokenClassifier_LongFlag(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("--long-listing")
	if ev.Kind != TokenLongFlag {
		t.Errorf("expected long_flag, got %q", ev.Kind)
	}
	if ev.Name != "long-listing" {
		t.Errorf("expected name 'long-listing', got %q", ev.Name)
	}
}

func TestTokenClassifier_LongFlagWithValue_Equals(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("--output=result.txt")
	if ev.Kind != TokenLongFlagWithValue {
		t.Errorf("expected long_flag_with_value, got %q", ev.Kind)
	}
	if ev.Name != "output" {
		t.Errorf("expected name 'output', got %q", ev.Name)
	}
	if ev.Value != "result.txt" {
		t.Errorf("expected value 'result.txt', got %q", ev.Value)
	}
}

func TestTokenClassifier_LongFlagWithValue_EmptyValue(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	// --output= (empty value after =)
	ev := tc.Classify("--output=")
	if ev.Kind != TokenLongFlagWithValue {
		t.Errorf("expected long_flag_with_value, got %q", ev.Kind)
	}
	if ev.Value != "" {
		t.Errorf("expected empty value, got %q", ev.Value)
	}
}

func TestTokenClassifier_UnknownLongFlag(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("--unknown-flag")
	if ev.Kind != TokenUnknownFlag {
		t.Errorf("expected unknown_flag, got %q", ev.Kind)
	}
}

func TestTokenClassifier_SingleDashLong_ExactMatch(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-classpath")
	if ev.Kind != TokenSingleDashLong {
		t.Errorf("expected single_dash_long, got %q", ev.Kind)
	}
	if ev.Name != "classpath" {
		t.Errorf("expected name 'classpath', got %q", ev.Name)
	}
}

func TestTokenClassifier_SingleDashLong_LongestMatchFirst(t *testing.T) {
	// "-classpath" should match SINGLE_DASH_LONG("classpath"), not
	// stacked flags starting with 'c' then 'l' etc.
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-classpath")
	if ev.Kind != TokenSingleDashLong {
		t.Errorf("longest match: expected single_dash_long, got %q (raw=%q)", ev.Kind, ev.Raw)
	}
	if ev.Name != "classpath" {
		t.Errorf("expected 'classpath', got %q", ev.Name)
	}
}

func TestTokenClassifier_ShortFlag_Boolean(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-l")
	if ev.Kind != TokenShortFlag {
		t.Errorf("expected short_flag, got %q", ev.Kind)
	}
	if ev.Name != "l" {
		t.Errorf("expected name 'l', got %q", ev.Name)
	}
}

func TestTokenClassifier_ShortFlag_NonBoolean_NoValue(t *testing.T) {
	// -o with no inline value → SHORT_FLAG (value is next token)
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-o")
	if ev.Kind != TokenShortFlag {
		t.Errorf("expected short_flag, got %q", ev.Kind)
	}
	if ev.Name != "o" {
		t.Errorf("expected name 'o', got %q", ev.Name)
	}
}

func TestTokenClassifier_ShortFlagWithValue_InlineValue(t *testing.T) {
	// -oresult.txt — 'o' is a non-boolean flag, remainder is the value
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-oresult.txt")
	if ev.Kind != TokenShortFlagWithValue {
		t.Errorf("expected short_flag_with_value, got %q", ev.Kind)
	}
	if ev.Name != "o" {
		t.Errorf("expected name 'o', got %q", ev.Name)
	}
	if ev.Value != "result.txt" {
		t.Errorf("expected value 'result.txt', got %q", ev.Value)
	}
}

func TestTokenClassifier_StackedFlags_AllBoolean(t *testing.T) {
	// -lah — l, a, h are all boolean flags
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-lah")
	if ev.Kind != TokenStackedFlags {
		t.Errorf("expected stacked_flags, got %q", ev.Kind)
	}
	if len(ev.Chars) != 3 {
		t.Errorf("expected 3 chars, got %d: %v", len(ev.Chars), ev.Chars)
	}
}

func TestTokenClassifier_StackedFlags_LastNonBoolean(t *testing.T) {
	// -lo — l is boolean, o is non-boolean (last, no inline value)
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-lo")
	if ev.Kind != TokenStackedFlags {
		t.Errorf("expected stacked_flags, got %q", ev.Kind)
	}
	if len(ev.Chars) != 2 {
		t.Errorf("expected 2 chars, got %d: %v", len(ev.Chars), ev.Chars)
	}
}

func TestTokenClassifier_StackedFlags_UnknownCharacter(t *testing.T) {
	// -lXh — X is unknown → UNKNOWN_FLAG
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-lXh")
	if ev.Kind != TokenUnknownFlag {
		t.Errorf("expected unknown_flag for '-lXh', got %q", ev.Kind)
	}
}

func TestTokenClassifier_StackedFlags_NonBooleanNotLast(t *testing.T) {
	// -ol — o is non-boolean but not last → invalid stack → UNKNOWN_FLAG
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("-ol")
	if ev.Kind != TokenUnknownFlag {
		t.Errorf("expected unknown_flag for non-boolean in non-last position, got %q", ev.Kind)
	}
}

func TestTokenClassifier_Positional_BareWord(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("filename.txt")
	if ev.Kind != TokenPositional {
		t.Errorf("expected positional, got %q", ev.Kind)
	}
	if ev.Name != "filename.txt" {
		t.Errorf("expected name 'filename.txt', got %q", ev.Name)
	}
}

func TestTokenClassifier_Positional_NumberLike(t *testing.T) {
	// Numbers starting with digits are positional
	tc := NewTokenClassifier(buildTestFlags())
	ev := tc.Classify("42")
	if ev.Kind != TokenPositional {
		t.Errorf("expected positional for '42', got %q", ev.Kind)
	}
}

func TestTokenClassifier_LookupMethods(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())

	if def := tc.LookupByLong("output"); def == nil {
		t.Error("expected to find 'output' by long name")
	}
	if def := tc.LookupByShort("l"); def == nil {
		t.Error("expected to find 'l' by short name")
	}
	if def := tc.LookupBySDL("classpath"); def == nil {
		t.Error("expected to find 'classpath' by SDL name")
	}
	if def := tc.LookupByLong("nonexistent"); def != nil {
		t.Error("expected nil for unknown long flag")
	}
}

func TestTokenClassifier_KnownNames(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	longs := tc.KnownLongNames()
	if len(longs) == 0 {
		t.Error("expected non-empty long names")
	}
	shorts := tc.KnownShortNames()
	if len(shorts) == 0 {
		t.Error("expected non-empty short names")
	}
}

func TestTokenClassifier_ClassifyTraditional_NotSubcommand(t *testing.T) {
	// Flags: x=boolean, v=boolean, f=non-boolean
	flags := []map[string]any{
		{"id": "extract", "short": "x", "description": "extract", "type": "boolean"},
		{"id": "verbose", "short": "v", "description": "verbose", "type": "boolean"},
		{"id": "file", "short": "f", "description": "file", "type": "string"},
	}
	tc := NewTokenClassifier(flags)
	known := map[string]bool{}
	ev := tc.ClassifyTraditional("xvf", known)
	if ev.Kind != TokenStackedFlags {
		t.Errorf("expected stacked_flags for 'xvf', got %q", ev.Kind)
	}
}

func TestTokenClassifier_ClassifyTraditional_KnownSubcommand(t *testing.T) {
	tc := NewTokenClassifier(buildTestFlags())
	known := map[string]bool{"add": true}
	ev := tc.ClassifyTraditional("add", known)
	// "add" is a known subcommand → Classify("add") → positional
	if ev.Kind != TokenPositional {
		t.Errorf("expected positional for known subcommand 'add', got %q", ev.Kind)
	}
}
