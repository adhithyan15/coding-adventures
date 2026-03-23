package javascriptparser

import (
	"testing"
)

func TestParseJavascript(t *testing.T) {
	source := "let x = 1 + 2;"
	program, err := ParseJavascript(source)
	if err != nil {
		t.Fatalf("Failed to parse JavaScript code: %v", err)
	}

	if program.RuleName != "program" {
		t.Fatalf("Expected program rule at root, got %s", program.RuleName)
	}
}
