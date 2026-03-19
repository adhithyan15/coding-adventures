package rubyparser

import (
	"testing"
)

func TestParseRuby(t *testing.T) {
	source := "x = 1 + 2 * 3"
	program, err := ParseRuby(source)
	if err != nil {
		t.Fatalf("Failed to parse Ruby code: %v", err)
	}

	if len(program.Statements) != 1 {
		t.Fatalf("Expected 1 statement, got %d", len(program.Statements))
	}
}
