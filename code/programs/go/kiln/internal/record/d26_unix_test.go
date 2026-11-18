//go:build unix

package record

// D26: S3's start-time gate is boot-relative, and the sealed side of it is
// prepare's OWN start time, read before prepare starts anything.

import (
	"errors"
	"testing"
)

var errNoSelf = errors.New("no self stat here")

// The ordering that makes the sealed value safe. Prepare starts sandboxes of
// its own (the structure probe and the toolchain probes), and a process one of
// those leaves behind started AFTER prepare did. If the start time were read
// after them, such a leftover would look older than prepare and S3 would skip
// it. The hook fails the test when either probe has already run.
func TestPrepareReadsItsOwnStartBeforeStartingAnyProbe(t *testing.T) {
	p := newPrep(t)
	p.withProbeBin("xtool")
	p.commit()
	calls := 0
	o := p.options()
	o.SelfStartTicks = func() (int64, error) {
		calls++
		if len(p.probeIn) != 0 || len(p.probeRuns) != 0 {
			t.Errorf("the start time was read after %d structure and %d toolchain probes", len(p.probeIn), len(p.probeRuns))
		}
		return 777, nil
	}
	t.Cleanup(p.writable)
	res, err := Prepare(o)
	if err != nil {
		t.Fatal(err)
	}
	// It did run the probes (otherwise the check above proved nothing) and read
	// the start exactly once, and sealed what it read.
	if len(p.probeIn) == 0 || len(p.probeRuns) == 0 {
		t.Fatalf("the probes did not run: %d structure, %d toolchain", len(p.probeIn), len(p.probeRuns))
	}
	if calls != 1 || res.Binding.PreparedTicks != 777 {
		t.Fatalf("calls %d, sealed %d", calls, res.Binding.PreparedTicks)
	}
}

// Collect's sanity check: its own start is never before prepare's on one boot.
func TestCollectSanityChecksThePreparedTicksAgainstItsOwnStart(t *testing.T) {
	for name, tc := range map[string]struct {
		self func(sealed int64) (int64, error)
		want func(sealed int64) int64
	}{
		"later start: the sealed bound is used": {
			self: func(s int64) (int64, error) { return s + 5000, nil },
			want: func(s int64) int64 { return s - SurvivorSlackTicks },
		},
		"the same tick: the sealed bound is used": {
			self: func(s int64) (int64, error) { return s, nil },
			want: func(s int64) int64 { return s - SurvivorSlackTicks },
		},
		"earlier start (reboot or another clock source): every process counts": {
			self: func(s int64) (int64, error) { return s - 1, nil },
			want: func(int64) int64 { return 0 },
		},
		"unreadable: every process counts": {
			self: func(int64) (int64, error) { return 0, errNoSelf },
			want: func(int64) int64 { return 0 },
		},
	} {
		f := newCfix(t)
		f.good()
		sealed := f.pre.Binding.PreparedTicks
		var got SurvivorQuery
		o := f.opts()
		o.SelfStartTicks = func() (int64, error) { return tc.self(sealed) }
		o.Survivors = func(q SurvivorQuery) ([]string, error) { got = q; return nil, nil }
		if _, err := Collect(o); err != nil {
			t.Fatalf("%s: %v", name, err)
		}
		if want := tc.want(sealed); got.SinceTicks != want {
			t.Errorf("%s: SinceTicks %d, want %d (sealed %d)", name, got.SinceTicks, want, sealed)
		}
	}
}
