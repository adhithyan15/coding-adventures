package traced

import (
	"bytes"
	"encoding/json"
	"fmt"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// censusKinds lists the kinds the census holds, with the paths under each.
func censusKinds(c *Census) map[string][]string {
	m := map[string][]string{}
	for _, e := range c.Entries() {
		m[e.Kind] = append(m[e.Kind], e.Path)
	}
	return m
}

// opsOf is the census's op-class counts for host-read.
func opsOf(c *Census) map[string]int64 { return c.Ops["host-read"] }

// host-read is split by the kind of look that raised it: a stat that found
// something, a probe that found nothing, an open, a listing.  Census only: the
// record's reasons are the same four host-read reasons they always were.
func TestCensusSplitsHostReadByOpClass(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	rec, c := h.runCensus(
		stat("/var/x/stat-me"),
		fail(stat("/var/x/absent"), "ENOENT"),
		openAbs("/var/x/open-me"),
		getdents("/var/x/list-me"),
	)
	want := map[string]int64{"stat": 1, "absent-probe": 1, "open": 1, "listing": 1}
	got := opsOf(c)
	for k, v := range want {
		if got[k] != v {
			t.Errorf("ops = %v, want %v", got, want)
			break
		}
	}
	var total int64
	for _, v := range got {
		total += v
	}
	if total != 4 {
		t.Errorf("ops sum %d, want 4: %v", total, got)
	}
	n := 0
	for _, r := range rec.Reasons {
		if strings.HasPrefix(r, "host-read:") {
			n++
		}
	}
	if n != 4 {
		t.Errorf("record holds %d host-read reasons, want 4", n)
	}
	// A class outside the closed list is dropped, and a nil census is a no-op.
	c.noteOp("host-read", "chmod-and-so-on")
	if _, ok := opsOf(c)["chmod-and-so-on"]; ok {
		t.Error("an op class outside OpClasses was kept")
	}
	var nc *Census
	nc.noteOp("host-read", "open")
}

// unstable-path and exec-unreadable carry no path in the record; the census is
// told WHICH rule fired (a fixed sub-code) and the redacted path, and the
// record's reason string is unchanged.
func TestCensusSubCodesForUnstableAndExecUnreadable(t *testing.T) {
	h := newHarness(t)
	h.fs.link(abs("code/p/a/final-link"), "../b")
	rec, c := h.runCensus(
		symlink("../b", "code/p/a/tmp-link"),
		rename("code/p/a/tmp-link", "code/p/a/final-link"),
		stat("code/p/a/final-link/b.txt"),
	)
	wantReason(t, rec, "trace-incomplete:unstable-path")
	subs := 0
	for kind, paths := range censusKinds(c) {
		if rest, ok := strings.CutPrefix(kind, "trace-incomplete:unstable-path:"); ok {
			subs++
			if rest == "" || strings.ContainsAny(rest, "/ ") || len(paths) == 0 || paths[0] == "" {
				t.Errorf("kind %q paths %q", kind, paths)
			}
		}
		if kind == "trace-incomplete:unstable-path" {
			t.Errorf("the plain kind was fed to the census: %v", paths)
		}
	}
	if subs == 0 {
		t.Fatalf("no unstable-path sub-code in the census: %v", c.Entries())
	}
	for _, r := range rec.Reasons {
		if strings.HasPrefix(r, "trace-incomplete:unstable-path") && r != "trace-incomplete:unstable-path" {
			t.Errorf("the record's reason changed: %q", r)
		}
	}

	h = newHarness(t)
	h.noSettle = true
	for _, tc := range []struct {
		ev         trace.Event
		kind, path string
	}{
		{execve("/usr/bin/ghost", "g"), "trace-incomplete:exec-unreadable:host-disk", "/usr/bin/ghost"},
		{execve("/proc/self/exe", "self"), "trace-incomplete:exec-unreadable:virtual-exe", "/proc/self/exe"},
		{execve("/home/runner/tool", "t"), "trace-incomplete:exec-unreadable:under-tmp", "$HOME/tool"},
	} {
		rec, c := h.runCensus(tc.ev)
		wantReason(t, rec, "trace-incomplete:exec-unreadable")
		if censusCount(c, tc.kind, tc.path) != 1 {
			t.Errorf("%s: want %q at %q, census holds %v", tc.ev.Path, tc.kind, tc.path, c.Entries())
		}
	}
}

// THE guard of the census contract: a Census is write-only.  The very same
// events classified with and without one give byte-identical records, across
// every raise site the census observes.
func TestCensusDoesNotChangeTheRecord(t *testing.T) {
	scenarios := map[string][]trace.Event{
		"host reads":   {stat("/var/x/a"), fail(stat("/var/x/b"), "ENOENT"), openAbs("/var/x/c"), getdents("/var/x/d")},
		"proc":         {openAbs("/proc/1234/cmdline")},
		"exec ghost":   {execve("/usr/bin/ghost", "g")},
		"exec virtual": {execve("/proc/self/exe", "self")},
		"exec tmp":     {execve("/home/runner/tool", "t")},
		"unstable": {
			symlink("../b", "code/p/a/tmp-link"),
			rename("code/p/a/tmp-link", "code/p/a/final-link"),
			stat("code/p/a/final-link/b.txt"),
		},
		"flood": func() []trace.Event {
			var evs []trace.Event
			for i := 0; i < 40; i++ {
				evs = append(evs, openAbs(fmt.Sprintf("/var/x/f%02d", i)))
			}
			return evs
		}(),
	}
	for name, evs := range scenarios {
		h := newHarness(t)
		h.noSettle = true
		h.fs.link(abs("code/p/a/final-link"), "../b")
		with, _ := h.runCensus(evs...)
		h2 := newHarness(t)
		h2.noSettle = true
		h2.fs.link(abs("code/p/a/final-link"), "../b")
		without := h2.run(evs...)
		a, err1 := json.Marshal(with)
		b, err2 := json.Marshal(without)
		if err1 != nil || err2 != nil || !bytes.Equal(a, b) {
			t.Errorf("%s: the record changed with a Census attached:\nwith    %s\nwithout %s", name, a, b)
		}
	}
}
