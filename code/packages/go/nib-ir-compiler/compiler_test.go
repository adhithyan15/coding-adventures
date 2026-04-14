package nibircompiler

import (
	"testing"

	nibtypechecker "github.com/adhithyan15/coding-adventures/code/packages/go/nib-type-checker"
)

func TestCompileSourceProducesIR(t *testing.T) {
	typed := nibtypechecker.CheckSource("fn main() { let x: u4 = 5; }")
	if !typed.OK {
		t.Fatalf("type check failed: %#v", typed.Errors)
	}
	result := CompileNib(typed.TypedAST, ReleaseConfig())
	if result.Program == nil || len(result.Program.Instructions) == 0 {
		t.Fatal("expected instructions")
	}
}
