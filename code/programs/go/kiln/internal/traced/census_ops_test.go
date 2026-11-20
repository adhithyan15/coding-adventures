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
		"proc ops":     {stat("/proc/1235/status"), getdents("/proc/1236/fd")},
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

// opsOfKind is the census's op-class counts for one kind.
func opsOfKind(c *Census, kind string) map[string]int64 { return c.Ops[kind] }

// proc-path is split by op class at its raise sites: an open, a stat and a
// listing of /proc paths land in three different classes.  Census only.
func TestCensusSplitsProcPathByOpClass(t *testing.T) {
	h := newHarness(t)
	h.noSettle = true
	rec, c := h.runCensus(
		openAbs("/proc/1234/cmdline"),
		stat("/proc/1235/status"),
		getdents("/proc/1236/fd"),
	)
	wantReason(t, rec, "trace-incomplete:proc-path")
	got := opsOfKind(c, "trace-incomplete:proc-path")
	var total int64
	for class, n := range got {
		if !IsOpClass(class) {
			t.Errorf("class %q is outside OpClasses", class)
		}
		total += n
	}
	if total < 3 || got["open"] < 1 || got["stat"] < 1 || got["listing"] < 1 {
		t.Errorf("proc-path ops = %v, want an open, a stat and a listing", got)
	}
}

// unstable-path is split by op class too, per sub-code, including the sub-codes
// raised after the walk (checkStability), whose class was remembered while the
// call was being observed.  Every occurrence has a class, and the counts add up
// to the occurrences the census holds for the kind.
func TestCensusSplitsUnstablePathByOpClass(t *testing.T) {
	h := newHarness(t)
	h.fs.link(abs("code/p/a/final-link"), "../b")
	_, c := h.runCensus(
		symlink("../b", "code/p/a/tmp-link"),
		rename("code/p/a/tmp-link", "code/p/a/final-link"),
		stat("code/p/a/final-link/b.txt"),
	)
	seen := 0
	for _, e := range c.Entries() {
		if !strings.HasPrefix(e.Kind, "trace-incomplete:unstable-path:") {
			continue
		}
		seen++
		var sum int64
		for class, n := range opsOfKind(c, e.Kind) {
			if !IsOpClass(class) {
				t.Errorf("%s: class %q is outside OpClasses", e.Kind, class)
			}
			sum += n
		}
		if sum < e.Count {
			t.Errorf("%s: ops sum %d < %d occurrences: %v", e.Kind, sum, e.Count, c.Ops[e.Kind])
		}
	}
	if seen == 0 {
		t.Fatalf("no unstable-path sub-code: %v", c.Entries())
	}
	// The path of a stat, after the walk: the stat's class, not the last event's.
	found := false
	for kind, m := range c.Ops {
		if strings.HasPrefix(kind, "trace-incomplete:unstable-path:") && m["stat"] > 0 {
			found = true
		}
	}
	if !found {
		t.Errorf("no unstable-path occurrence was classed as a stat: %v", c.Ops)
	}
}

// The remembered class exists only with a Census, keeps the FIRST look's class
// for a path, and a path with none is "other".
func TestCensusLookupOpsOnlyWithACensus(t *testing.T) {
	c := &classifier{}
	c.noteLookupOp("/p")
	if c.lookupOps != nil {
		t.Errorf("lookupOps filled without a Census: %v", c.lookupOps)
	}
	c.census = NewCensus()
	c.obsPresent, c.opKind = true, "stat"
	c.noteLookupOp("/p")
	c.opKind = "open"
	c.noteLookupOp("/p")
	if got := c.lookupOpOf("/p"); got != "stat" {
		t.Errorf("lookupOpOf(/p) = %q, want the first look's class stat", got)
	}
	if got := c.lookupOpOf("/nowhere"); got != "other" {
		t.Errorf("lookupOpOf(unknown) = %q", got)
	}
}
