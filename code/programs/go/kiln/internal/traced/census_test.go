package traced

import (
	"fmt"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// runCensus classifies one step with a census attached.
func (h *harness) runCensus(events ...trace.Event) (*RawRecord, *Census) {
	h.t.Helper()
	st := step(events...)
	h.settle([]*trace.Trace{st})
	c := NewCensus()
	rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{st}, Recipe: []string{"step"}, Census: c})
	if err != nil {
		h.t.Fatal(err)
	}
	return rec, c
}

func censusCount(c *Census, kind, path string) int64 {
	for _, e := range c.Entries() {
		if e.Kind == kind && e.Path == path {
			return e.Count
		}
	}
	return 0
}

// More than 16 host reads: the record keeps 16, the census keeps every one.
func TestCensusSeesPastTheCap(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	var evs []trace.Event
	for i := 0; i < 40; i++ {
		evs = append(evs, openAbs(fmt.Sprintf("/var/x/f%02d", i)))
	}
	rec, c := h.runCensus(evs...)
	n := 0
	for _, r := range rec.Reasons {
		if strings.HasPrefix(r, "host-read:") {
			n++
		}
	}
	if n != maxReasonsPerKind {
		t.Fatalf("record keeps %d host-read reasons, want %d", n, maxReasonsPerKind)
	}
	got := 0
	for _, e := range c.Entries() {
		if e.Kind == "host-read" {
			got++
		}
	}
	if got < 40 {
		t.Fatalf("census holds %d host-read paths, want at least 40", got)
	}
	if censusCount(c, "host-read", "/var/x/f39") != 1 {
		t.Fatalf("the last path is missing: %v", c.Entries())
	}
}

// proc-path has no path in its reason text; the census has it.
func TestCensusCapturesProcPath(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	rec, c := h.runCensus(openAbs("/proc/1234/cmdline"))
	wantReason(t, rec, "trace-incomplete:proc-path")
	if censusCount(c, "trace-incomplete:proc-path", "/proc/1234/cmdline") != 1 {
		t.Fatalf("proc path missing: %v", c.Entries())
	}
	for _, r := range rec.Reasons {
		if strings.Contains(r, "1234") {
			t.Fatalf("the record's reason carries the path: %q", r)
		}
	}
}

// A repeated pair is one entry with a count; $HOME is redacted as in a reason.
func TestCensusCountsAndRedacts(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	_, c := h.runCensus(openAbs("/home/runner/.cache/z"), openAbs("/home/runner/.cache/z"))
	if n := censusCount(c, "host-read", "$HOME/.cache/z"); n != 2 {
		t.Fatalf("count %d: %v", n, c.Entries())
	}
}

func TestCensusBound(t *testing.T) {
	c := NewCensus()
	for i := 0; i < MaxCensusPerKind+100; i++ {
		c.Note("host-read", fmt.Sprintf("/p/%d", i))
	}
	if len(c.Entries()) != MaxCensusPerKind {
		t.Fatalf("held %d", len(c.Entries()))
	}
	if c.Overflow["host-read"] != 100 || c.Total != int64(MaxCensusPerKind+100) {
		t.Fatalf("overflow %v total %d", c.Overflow, c.Total)
	}
	// A pair already held still counts past the bound.
	c.Note("host-read", "/p/0")
	if censusCount(c, "host-read", "/p/0") != 2 || c.Overflow["host-read"] != 100 {
		t.Fatalf("held pair not counted: %d, overflow %v", censusCount(c, "host-read", "/p/0"), c.Overflow)
	}
	// The overall bound holds across many kinds.
	m := NewCensus()
	for k := 0; k < 24; k++ {
		for i := 0; i < MaxCensusPerKind; i++ {
			m.Note(fmt.Sprintf("kind%d", k), fmt.Sprintf("/p/%d", i))
		}
	}
	if len(m.Entries()) != MaxCensusEntries {
		t.Fatalf("overall bound: held %d", len(m.Entries()))
	}
	// A nil census is a no-op.
	var nc *Census
	nc.Note("k", "p")
	if nc.Entries() != nil {
		t.Fatal("nil census has entries")
	}
}

func TestCensusSanitisesHostilePaths(t *testing.T) {
	c := NewCensus()
	c.Note("host-read", "/a\nb::c‮evil")
	c.Note("host-read", "/"+strings.Repeat("x", 5000))
	for _, e := range c.Entries() {
		if strings.ContainsAny(e.Path, "\n‮") {
			t.Fatalf("unprintable path survived: %q", e.Path)
		}
		if len(e.Path) > maxReasonText+3 {
			t.Fatalf("path of %d bytes", len(e.Path))
		}
	}
	if censusCount(c, "host-read", "/a?b::c?evil") != 1 {
		t.Fatalf("%v", c.Entries())
	}
}

// A flood of one kind must not erase another's entries; overflow is per kind.
func TestCensusFloodDoesNotStarveOtherKinds(t *testing.T) {
	c := NewCensus()
	for i := 0; i < 5000; i++ {
		c.Note("host-read", fmt.Sprintf("/f/%d", i))
	}
	c.Note("trace-incomplete:proc-path", "/proc/7/cmdline")
	if censusCount(c, "trace-incomplete:proc-path", "/proc/7/cmdline") != 1 {
		t.Fatal("the late kind was starved")
	}
	if c.Overflow["host-read"] != int64(5000-MaxCensusPerKind) || c.Overflow["trace-incomplete:proc-path"] != 0 {
		t.Fatalf("overflow %v", c.Overflow)
	}
}

// A path the classifier already sanitised is not sanitised again: a cut
// multi-byte rune must keep its three dots, not grow five.
func TestCensusDoesNotSanitiseTwice(t *testing.T) {
	long := "/" + strings.Repeat("\u00e9", 150)
	h := newHarness(t)
	h.noSettle = true
	_, c := h.runCensus(openAbs("/var/" + long))
	want := sanitize("/var/" + long)
	if censusCount(c, "host-read", want) != 1 {
		t.Fatalf("path was altered: %v", c.Entries())
	}
}
