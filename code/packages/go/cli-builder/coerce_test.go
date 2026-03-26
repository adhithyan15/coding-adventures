package clibuilder

import (
	"testing"
)

// =========================================================================
// Coercion tests
// =========================================================================

func TestCoerce_String(t *testing.T) {
	val, err := coerceValue("hello", "string", nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != "hello" {
		t.Errorf("expected 'hello', got %v", val)
	}
}

func TestCoerce_String_Empty(t *testing.T) {
	_, err := coerceValue("", "string", nil)
	if err == nil {
		t.Fatal("expected error for empty string")
	}
}

func TestCoerce_Integer_Valid(t *testing.T) {
	val, err := coerceValue("42", "integer", nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != int64(42) {
		t.Errorf("expected int64(42), got %v (%T)", val, val)
	}
}

func TestCoerce_Integer_Negative(t *testing.T) {
	val, err := coerceValue("-7", "integer", nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != int64(-7) {
		t.Errorf("expected int64(-7), got %v", val)
	}
}

func TestCoerce_Integer_Invalid(t *testing.T) {
	_, err := coerceValue("abc", "integer", nil)
	if err == nil {
		t.Fatal("expected error for non-integer")
	}
}

func TestCoerce_Float_Valid(t *testing.T) {
	val, err := coerceValue("3.14", "float", nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	f, ok := val.(float64)
	if !ok {
		t.Fatalf("expected float64, got %T", val)
	}
	if f < 3.13 || f > 3.15 {
		t.Errorf("expected ~3.14, got %v", f)
	}
}

func TestCoerce_Float_Invalid(t *testing.T) {
	_, err := coerceValue("not-a-float", "float", nil)
	if err == nil {
		t.Fatal("expected error for non-float")
	}
}

func TestCoerce_Boolean_True(t *testing.T) {
	for _, raw := range []string{"true", "1", "yes"} {
		val, err := coerceValue(raw, "boolean", nil)
		if err != nil {
			t.Errorf("unexpected error for %q: %v", raw, err)
			continue
		}
		if val != true {
			t.Errorf("expected true for %q, got %v", raw, val)
		}
	}
}

func TestCoerce_Boolean_False(t *testing.T) {
	for _, raw := range []string{"false", "0", "no"} {
		val, err := coerceValue(raw, "boolean", nil)
		if err != nil {
			t.Errorf("unexpected error for %q: %v", raw, err)
			continue
		}
		if val != false {
			t.Errorf("expected false for %q, got %v", raw, val)
		}
	}
}

func TestCoerce_Boolean_Invalid(t *testing.T) {
	_, err := coerceValue("maybe", "boolean", nil)
	if err == nil {
		t.Fatal("expected error for invalid boolean")
	}
}

func TestCoerce_Path_Valid(t *testing.T) {
	val, err := coerceValue("/some/path/that/may/not/exist", "path", nil)
	if err != nil {
		t.Fatalf("unexpected error for path: %v", err)
	}
	if val != "/some/path/that/may/not/exist" {
		t.Errorf("expected path unchanged, got %v", val)
	}
}

func TestCoerce_Path_Empty(t *testing.T) {
	_, err := coerceValue("", "path", nil)
	if err == nil {
		t.Fatal("expected error for empty path")
	}
}

func TestCoerce_Enum_Valid(t *testing.T) {
	def := map[string]any{"enum_values": []any{"json", "csv", "table"}}
	val, err := coerceValue("json", "enum", def)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != "json" {
		t.Errorf("expected 'json', got %v", val)
	}
}

func TestCoerce_Enum_Invalid(t *testing.T) {
	def := map[string]any{"enum_values": []any{"json", "csv", "table"}}
	_, err := coerceValue("bork", "enum", def)
	if err == nil {
		t.Fatal("expected error for invalid enum value")
	}
}

func TestCoerce_Enum_CaseSensitive(t *testing.T) {
	def := map[string]any{"enum_values": []any{"JSON"}}
	_, err := coerceValue("json", "enum", def)
	if err == nil {
		t.Fatal("enum comparison should be case-sensitive: 'json' != 'JSON'")
	}
}

func TestCoerce_UnknownType_PassThrough(t *testing.T) {
	val, err := coerceValue("anything", "unknown-type", nil)
	if err != nil {
		t.Fatalf("unexpected error for unknown type: %v", err)
	}
	if val != "anything" {
		t.Errorf("expected pass-through value, got %v", val)
	}
}
