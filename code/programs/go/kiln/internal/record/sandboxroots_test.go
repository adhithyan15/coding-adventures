package record

import (
	"strings"
	"testing"
)

// KILN04 D39 rule 1b lets the classifier treat an absent probe strictly inside
// the fresh HOME or TMPDIR as a constant.  That is sound only if everything the
// sandbox BINDS from the host beneath them is a path the classifier knows about
// as a declared root (internal/traced prepare: every host-roots.json root, of
// any kind, is excluded from "fresh").  Both sides come from the same
// host-roots.json: BuildSandboxConfig turns the image and tool roots into
// sandbox RoRoots, and traced reads the same entries as its roots.  This test
// pins the subset relation, so a future change that binds an extra host path
// under HOME (a new anchor, say) cannot silently reach the rule.
//
// The one deliberate exception is a state root the recorder DERIVES from an
// anchor (stateAnchors: ~/.cargo/registry and ~/.cargo/git).  The sandbox
// requires each to be absent or an empty directory (stateRootEmpty), so it is
// as empty as the tmpfs around it and an absent probe of it is still a constant.
func TestSandboxBindsUnderHomeAreDeclaredRoots(t *testing.T) {
	in := baseInput(runnerFS())
	cfg, err := BuildSandboxConfig(in)
	if err != nil {
		t.Fatal(err)
	}
	covers := func(root, p string) bool { return p == root || strings.HasPrefix(p, root+"/") }
	beneath := func(p string) bool {
		for _, d := range []string{in.Home, cfg.Tmpdir} {
			if strings.HasPrefix(p, d+"/") {
				return true
			}
		}
		return false
	}
	var declared []string
	for _, r := range in.Roots.Roots {
		p, err := expandRoot(r.Path, in.Home)
		if err != nil {
			t.Fatal(err)
		}
		declared = append(declared, p)
	}
	declaredCovers := func(p string) bool {
		for _, d := range declared {
			if covers(d, p) {
				return true
			}
		}
		return false
	}
	seen := 0
	for _, r := range cfg.RoRoots {
		if !beneath(r) {
			continue
		}
		seen++
		if !declaredCovers(r) {
			t.Errorf("the sandbox binds %s from the host beneath HOME/TMPDIR, but no declared host root covers it: "+
				"traced would treat absent probes there as fresh-tmpfs constants (KILN04 D39)", r)
		}
	}
	if seen == 0 {
		t.Fatal("the fixture binds nothing beneath HOME; the test pins nothing")
	}
	for _, s := range cfg.StateRoots {
		if !beneath(s) || declaredCovers(s) {
			continue
		}
		derived := false
		for a := range stateAnchors {
			derived = derived || covers(in.Home+"/"+a, s)
		}
		if !derived {
			t.Errorf("state root %s is bound beneath HOME/TMPDIR but is neither declared nor derived from a stateAnchors entry", s)
		}
	}
}
