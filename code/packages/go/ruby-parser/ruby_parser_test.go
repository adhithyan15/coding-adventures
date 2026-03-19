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

	if program.RuleName != "program" {
		t.Fatalf("Expected program rule mapped, got %s", program.RuleName)
	}
}
