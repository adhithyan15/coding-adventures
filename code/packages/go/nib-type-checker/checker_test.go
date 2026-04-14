package nibtypechecker

import "testing"

func TestCheckSourceAcceptsSimpleProgram(t *testing.T) {
	result := CheckSource("fn main() { let x: u4 = 5; }")
	if !result.OK {
		t.Fatalf("expected type check success, got %#v", result.Errors)
	}
}

func TestCheckSourceRejectsBadAssignment(t *testing.T) {
	result := CheckSource("fn main() { let x: bool = 1 +% 2; }")
	if result.OK {
		t.Fatal("expected type check failure")
	}
}
