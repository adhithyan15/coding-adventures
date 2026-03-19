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

	if len(program.Statements) != 1 {
		t.Fatalf("Expected 1 statement, got %d", len(program.Statements))
	}
}
