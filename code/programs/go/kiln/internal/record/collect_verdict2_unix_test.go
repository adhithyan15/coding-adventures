//go:build unix

package record

import (
	"os"
	"testing"
)

// An action whose steps vanished but whose verdict is left: its record counts a
// step nobody can find, and the counter now runs ahead of the sidecars, so the
// whole shard's order is in doubt.
func TestCollectAVerdictWithNoStepVoidsTheShard(t *testing.T) {
	f := newCfix(t)
	f.good()
	os.Remove(StepFile(f.p.dir, ca, 0, "json"))
	os.Remove(StepFile(f.p.dir, ca, 0, "trace"))
	_, sh := f.collect()
	requireUnknown(t, sh, ca, "counts 1 started steps, the sidecars 0")
	requireUnknown(t, sh, cb, "the counter reached 4 and 3 sidecars")
}
