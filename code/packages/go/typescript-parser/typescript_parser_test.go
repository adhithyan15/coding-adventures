package typescriptparser

import (
	"testing"
)

func TestParseTypescript(t *testing.T) {
	source := "let x = 1 + 2;"
	program, err := ParseTypescript(source)
	if err != nil {
		t.Fatalf("Failed to parse TypeScript code: %v", err)
	}

	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}
