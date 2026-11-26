package traced

import (
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
	"reflect"
	"testing"
)

// KILN04 D50 rule 2c: the parser now hands the classifier a successful
// read-only open with an EMPTY ReturnFDPath (no -y annotation).  openCall
// already treats a non-absolute return path as "walk the name, settle nothing
// from the kernel"; these tests pin that this is what happens, rather than
// assume it.

// bare is a successful read-only open with no annotation.
func bare(path string) trace.Event {
	return open(path, "O_RDONLY|O_CLOEXEC", "")
}

func TestBareOpenIsRecordedAgainstTheWalkedPath(t *testing.T) {
	for _, p := range []string{"code/p/a/README", "code/p/a/src/x.go", "code/p/b/b.txt"} {
		bareRec := newHarness(t).run(bare(p))
		annRec := newHarness(t).run(open(abs(p), "O_RDONLY|O_CLOEXEC", abs(p)))
		wantNoReasons(t, bareRec)
		if !reflect.DeepEqual(bareRec.Entries, annRec.Entries) || len(bareRec.Entries) == 0 {
			t.Errorf("%s: bare %q, annotated %q", p, bareRec.Entries, annRec.Entries)
		}
	}
}

// The read is under the strict stability rule: when the file the walk named is
// not what the post-run disk shows, the record is not trusted.  Here the walk
// found a regular file and the disk (the settle step) shows a symlink that was
// swapped in, exactly the case the annotation would have caught as
// fd-path-differs and the strict rule catches without it.
func TestBareOpenIsUnderTheStrictStabilityRule(t *testing.T) {
	p := "code/p/a/README"
	h := newHarness(t)
	rec := h.run(bare(p), symlink("elsewhere", p))
	if !hasUnstable(rec) && len(rec.Reasons) == 0 {
		t.Errorf("a bare read of a name the action then replaced was trusted: %+v", rec)
	}
}

// A final symlink is followed as a stat would follow it: the link and its
// target are both entries.
func TestBareOpenOfAFinalSymlinkIsHandledLikeStat(t *testing.T) {
	h := newHarness(t)
	rec := h.run(bare("code/p/a/lnk"))
	st := newHarness(t).run(stat("code/p/a/lnk"))
	wantNoReasons(t, rec)
	if !reflect.DeepEqual(rec.Entries, st.Entries) {
		t.Errorf("bare open entries %q, stat entries %q", rec.Entries, st.Entries)
	}
}

func TestBareOpenOfAProcMagicLinkIsStillRefused(t *testing.T) {
	for _, p := range []string{"/proc/self/cwd", "/proc/self/fd/3", "/proc/self/root", "/proc/self/cwd/../../cpuinfo", "/proc/1234/exe", "/dev/fd/3"} {
		rec := newHarness(t).run(bare(p))
		if !hasReason(rec, "trace-incomplete:proc-path") {
			t.Errorf("%s: reasons %q entries %q, want proc-path", p, rec.Reasons, rec.Entries)
		}
	}
}

func TestBareOpenUnderTheTmpfsIsContained(t *testing.T) {
	f := tTmp + "/scratch.txt"
	h := newHarness(t)
	rec := h.wc(open(f, "O_WRONLY|O_CREAT|O_EXCL", f), bare(f))
	if hasUnstable(rec) {
		t.Errorf("a bare read of the action's own tmpfs file was unstable: %q", rec.Reasons)
	}
}

// The census counters are summed over the steps from the parser's own counts,
// and a nil census costs nothing.
func TestCensusCountsTheParseRules(t *testing.T) {
	h := newHarness(t)
	c := NewCensus()
	s1, s2 := step(bare("code/p/a/README")), step()
	s1.RestartedSpawns, s1.BareOpens = 1, 1
	s2.RestartedSpawns, s2.BareOpens = 2, 4
	if _, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{s1, s2}, Recipe: []string{"step", "step"}, Census: c}); err != nil {
		t.Fatal(err)
	}
	if c.RestartedSpawns != 3 || c.BareOpens != 5 {
		t.Fatalf("census %d/%d, want 3/5", c.RestartedSpawns, c.BareOpens)
	}
	var nilCensus *Census
	nilCensus.noteParseRules(s1) // must not panic
}
