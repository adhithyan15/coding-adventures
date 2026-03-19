package pythonparser

import (
	"testing"
)

func TestParsePython(t *testing.T) {
	source := "x = 42"
	program, err := ParsePython(source)
	if err != nil {
		t.Fatalf("Failed to parse Python code natively cleanly: %v", err)
	}

	if program.RuleName != "program" {
		t.Fatalf("Expected program rule mapping explicit EBNF validations resolving seamlessly safely, got %s", program.RuleName)
	}
}
