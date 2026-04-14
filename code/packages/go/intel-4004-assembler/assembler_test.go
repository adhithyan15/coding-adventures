package intel4004assembler

import "testing"

func TestAssembleSimpleProgram(t *testing.T) {
	binary, err := Assemble("ORG 0x000\n_start:\n    LDM 5\n    XCH R2\n    HLT\n")
	if err != nil {
		t.Fatalf("assemble failed: %v", err)
	}
	if len(binary) != 3 {
		t.Fatalf("expected 3 bytes, got %d", len(binary))
	}
}
