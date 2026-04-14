package nibparser

import "testing"

func TestParseNibProgram(t *testing.T) {
	ast, err := ParseNib("fn main() { let x: u4 = 5; }")
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if ast == nil || ast.RuleName != "program" {
		t.Fatalf("expected program root, got %#v", ast)
	}
}
