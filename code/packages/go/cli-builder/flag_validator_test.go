package clibuilder

import (
	"testing"
)

// =========================================================================
// Flag validator tests
// =========================================================================

func makeFlag(id, short, long, flagType string, extra map[string]any) map[string]any {
	f := map[string]any{
		"id":          id,
		"description": id + " flag",
		"type":        flagType,
	}
	if short != "" {
		f["short"] = short
	}
	if long != "" {
		f["long"] = long
	}
	for k, v := range extra {
		f[k] = v
	}
	return f
}

func TestFlagValidator_NoErrors(t *testing.T) {
	flags := []map[string]any{
		makeFlag("verbose", "v", "verbose", "boolean", nil),
		makeFlag("output", "o", "output", "string", nil),
	}
	fv := NewFlagValidator(flags, nil)
	errs := fv.Validate(map[string]any{
		"verbose": true,
		"output":  "out.txt",
	})
	if len(errs) != 0 {
		t.Errorf("expected no errors, got: %v", errs)
	}
}

func TestFlagValidator_ConflictingFlags(t *testing.T) {
	flags := []map[string]any{
		makeFlag("enable-escapes", "e", "enable-escapes", "boolean", map[string]any{
			"conflicts_with": []any{"disable-escapes"},
		}),
		makeFlag("disable-escapes", "E", "disable-escapes", "boolean", map[string]any{
			"conflicts_with": []any{"enable-escapes"},
		}),
	}
	fv := NewFlagValidator(flags, nil)
	errs := fv.Validate(map[string]any{
		"enable-escapes":  true,
		"disable-escapes": true,
	})
	found := false
	for _, e := range errs {
		if e.ErrorType == ErrConflictingFlags {
			found = true
		}
	}
	if !found {
		t.Errorf("expected conflicting_flags error, got: %v", errs)
	}
}

func TestFlagValidator_MissingDependency(t *testing.T) {
	// human-readable requires long-listing
	flags := []map[string]any{
		makeFlag("long-listing", "l", "long-listing", "boolean", nil),
		makeFlag("human-readable", "h", "human-readable", "boolean", map[string]any{
			"requires": []any{"long-listing"},
		}),
	}
	fv := NewFlagValidator(flags, nil)
	errs := fv.Validate(map[string]any{
		"human-readable": true,
		"long-listing":   false,
	})
	found := false
	for _, e := range errs {
		if e.ErrorType == ErrMissingDependencyFlag {
			found = true
		}
	}
	if !found {
		t.Errorf("expected missing_dependency_flag error, got: %v", errs)
	}
}

func TestFlagValidator_RequiredFlag_Present(t *testing.T) {
	flags := []map[string]any{
		makeFlag("message", "m", "message", "string", map[string]any{"required": true}),
	}
	fv := NewFlagValidator(flags, nil)
	errs := fv.Validate(map[string]any{"message": "my commit"})
	if len(errs) != 0 {
		t.Errorf("expected no errors when required flag is present, got: %v", errs)
	}
}

func TestFlagValidator_RequiredFlag_Absent(t *testing.T) {
	flags := []map[string]any{
		makeFlag("message", "m", "message", "string", map[string]any{"required": true}),
	}
	fv := NewFlagValidator(flags, nil)
	errs := fv.Validate(map[string]any{"message": nil})
	found := false
	for _, e := range errs {
		if e.ErrorType == ErrMissingRequiredFlag {
			found = true
		}
	}
	if !found {
		t.Errorf("expected missing_required_flag error, got: %v", errs)
	}
}

func TestFlagValidator_RequiredUnless_Exempted(t *testing.T) {
	// "file" is required unless "stdin" is present
	flags := []map[string]any{
		makeFlag("file", "f", "file", "string", map[string]any{
			"required":        true,
			"required_unless": []any{"stdin"},
		}),
		makeFlag("stdin", "", "stdin", "boolean", nil),
	}
	fv := NewFlagValidator(flags, nil)
	// stdin is present and true → file should not be required
	errs := fv.Validate(map[string]any{
		"file":  nil,
		"stdin": true,
	})
	for _, e := range errs {
		if e.ErrorType == ErrMissingRequiredFlag {
			t.Errorf("should be exempted by required_unless, but got: %v", e)
		}
	}
}

func TestFlagValidator_ExclusiveGroup_Violation(t *testing.T) {
	flags := []map[string]any{
		makeFlag("extended-regexp", "E", "extended-regexp", "boolean", nil),
		makeFlag("fixed-strings", "F", "fixed-strings", "boolean", nil),
		makeFlag("perl-regexp", "P", "perl-regexp", "boolean", nil),
	}
	groups := []map[string]any{
		{
			"id":       "regexp-engine",
			"flag_ids": []any{"extended-regexp", "fixed-strings", "perl-regexp"},
			"required": false,
		},
	}
	fv := NewFlagValidator(flags, groups)
	errs := fv.Validate(map[string]any{
		"extended-regexp": true,
		"fixed-strings":   true,
		"perl-regexp":     false,
	})
	found := false
	for _, e := range errs {
		if e.ErrorType == ErrExclusiveGroupViolation {
			found = true
		}
	}
	if !found {
		t.Errorf("expected exclusive_group_violation error, got: %v", errs)
	}
}

func TestFlagValidator_ExclusiveGroup_Required_Missing(t *testing.T) {
	flags := []map[string]any{
		makeFlag("extended-regexp", "E", "extended-regexp", "boolean", nil),
		makeFlag("fixed-strings", "F", "fixed-strings", "boolean", nil),
	}
	groups := []map[string]any{
		{
			"id":       "regexp-engine",
			"flag_ids": []any{"extended-regexp", "fixed-strings"},
			"required": true,
		},
	}
	fv := NewFlagValidator(flags, groups)
	// Neither flag is present
	errs := fv.Validate(map[string]any{
		"extended-regexp": false,
		"fixed-strings":   false,
	})
	found := false
	for _, e := range errs {
		if e.ErrorType == ErrMissingExclusiveGroup {
			found = true
		}
	}
	if !found {
		t.Errorf("expected missing_exclusive_group error, got: %v", errs)
	}
}

func TestFlagValidator_ExclusiveGroup_Required_OnePresent(t *testing.T) {
	flags := []map[string]any{
		makeFlag("extended-regexp", "E", "extended-regexp", "boolean", nil),
		makeFlag("fixed-strings", "F", "fixed-strings", "boolean", nil),
	}
	groups := []map[string]any{
		{
			"id":       "regexp-engine",
			"flag_ids": []any{"extended-regexp", "fixed-strings"},
			"required": true,
		},
	}
	fv := NewFlagValidator(flags, groups)
	errs := fv.Validate(map[string]any{
		"extended-regexp": true,
		"fixed-strings":   false,
	})
	// Should be no error — exactly one is present
	for _, e := range errs {
		if e.ErrorType == ErrMissingExclusiveGroup || e.ErrorType == ErrExclusiveGroupViolation {
			t.Errorf("unexpected error: %v", e)
		}
	}
}

func TestFlagLabel(t *testing.T) {
	tests := []struct {
		def      map[string]any
		expected string
	}{
		{map[string]any{"id": "verbose", "short": "v", "long": "verbose"}, "-v/--verbose"},
		{map[string]any{"id": "output", "long": "output"}, "--output"},
		{map[string]any{"id": "q", "short": "q"}, "-q"},
		{nil, "(unknown)"},
	}
	for _, tt := range tests {
		got := flagLabel(tt.def)
		if got != tt.expected {
			t.Errorf("flagLabel(%v): expected %q, got %q", tt.def, tt.expected, got)
		}
	}
}
