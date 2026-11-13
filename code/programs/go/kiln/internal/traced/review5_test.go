package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

func TestHostRootMayNotCoverFreshDirectories(t *testing.T) {
	good := Input{Package: "code/p/a", Traces: []*trace.Trace{step(stat("go.work"))}, Recipe: []string{"step"}}
	for name, path := range map[string]string{
		"equals home":     "/home/runner",
		"equals tmpdir":   "/tmp/kiln-1",
		"contains home":   "/home",
		"contains tmpdir": "/tmp",
	} {
		h := newHarness(t)
		h.cfg.HostRoots.Roots = append(h.cfg.HostRoots.Roots, legacy.HostRoot{Path: path, Kind: legacy.RootImage})
		if _, err := Classify(h.cfg, good); err == nil {
			t.Errorf("%s: no error", name)
		}
	}
}
