package record

import (
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// DerivedStateRoots names the writable host binds the recorder derives from
// stateAnchors, so the classifier (traced.Config.HostBinds) never mistakes
// them for fresh tmpfs (KILN04 D40).
func TestDerivedStateRoots(t *testing.T) {
	roots := &legacy.HostRoots{Roots: []legacy.HostRoot{
		{Path: "/usr", Kind: legacy.RootImage},
		{Path: "$HOME/.cargo/bin", Kind: legacy.RootTool, Tool: "rust"},
		{Path: "$HOME/.rustup", Kind: legacy.RootTool, Tool: "rust"}, // direct child: no anchor
		{Path: "$HOME/.cargo/registry", Kind: legacy.RootState, Ecosystem: "rust"},
	}}
	got := DerivedStateRoots("/home/runner", roots)
	want := []string{"/home/runner/.cargo/git", "/home/runner/.cargo/registry"}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("DerivedStateRoots = %q, want %q", got, want)
	}
	if got := DerivedStateRoots("/home/runner", &legacy.HostRoots{}); len(got) != 0 {
		t.Errorf("no roots gave %q", got)
	}
	if DerivedStateRoots("/home/runner", nil) != nil {
		t.Error("nil roots must give nil")
	}
}
