package traced

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// The differential fuzzer's reshaped / dotdot oracles (model_test.go, bursts 22
// to 27) only fail when the classifier lets something through.  They would also
// stay silent if a burst quietly picked up an UNRELATED reason, because the
// fuzzer returns early on any record that has one.  This test runs three of the
// bursts in a fixed order, alone, through the model and the classifier, and
// pins the record's reasons to exactly the taint's, so a later change that
// makes a burst trip on something else fails here instead of disarming the
// oracles.

// runModelOps runs ops against a fresh model, in order, as ONE step by one
// process, and classifies the result with a census attached.
func runModelOps(t *testing.T, ops func(m *model) []trace.Event) (*RawRecord, *Census) {
	t.Helper()
	m, tree := newModel(rootChoices[0](""), homeChoices[0], tmpChoices[0])
	evs := ops(m)
	sh := execve("/usr/bin/sh", "sh", "-c", "step")
	sh.Pid, sh.Line, sh.StartLine, sh.Cwd = 1, 1, 1, m.root
	tr := &trace.Trace{
		RootPid: 1,
		Procs:   map[int]*trace.Process{1: {Pid: 1, Exit: &trace.Exit{Pid: 1}}},
		Exits:   []trace.Exit{{Pid: 1}},
		Events:  []trace.Event{sh},
	}
	for i, e := range evs {
		e.Line, e.StartLine = i+2, i+2
		tr.Events = append(tr.Events, e)
	}
	h := newHarness(t)
	h.cfg.Root, h.cfg.Home, h.cfg.Tmpdir = m.root, m.home, m.tmp
	h.cfg.ScratchDirs = modelScratch
	h.cfg.Packages = NewPackageIndex([]string{"code/p/a", "code/p/b"})
	h.cfg.Tree = tree
	h.cfg.FS = modelFS{m}
	census := NewCensus()
	rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: []*trace.Trace{tr}, Recipe: []string{"step"}, StepNetNS: true, Census: census})
	if err != nil {
		t.Fatal(err)
	}
	return rec, census
}

func wantExactReasons(t *testing.T, rec *RawRecord, want ...string) {
	t.Helper()
	if len(rec.Reasons) != len(want) {
		t.Fatalf("reasons %q, want exactly %q", rec.Reasons, want)
	}
	for i := range want {
		if rec.Reasons[i] != want[i] {
			t.Fatalf("reasons %q, want exactly %q", rec.Reasons, want)
		}
	}
}

func TestReshapedBurstsHaveExactlyTheTaintsReasons(t *testing.T) {
	cat := func(parts ...[]trace.Event) []trace.Event {
		var all []trace.Event
		for _, p := range parts {
			all = append(all, p...)
		}
		return all
	}
	// The host-reads settleReshaped raises, nothing else: the absences were
	// allowed by D43 and the move turns them back.
	t.Run("burst 22: an own directory is renamed", func(t *testing.T) {
		rec, _ := runModelOps(t, func(m *model) []trace.Event {
			return cat(
				m.doStat(1, "/package.json", true),
				m.doMkdirLike(1, "mkdir", m.tmp+"/o", ""),
				m.doRenameLink(1, "rename", m.tmp+"/o", m.tmp+"/u.o", false),
				m.doStat(1, "/package.json", true),
				m.doStat(1, "/opt/x", true))
		})
		wantExactReasons(t, rec, "host-read:/opt/x", "host-read:/package.json")
	})
	t.Run("burst 25: two own directories are exchanged", func(t *testing.T) {
		rec, _ := runModelOps(t, func(m *model) []trace.Event {
			return cat(
				m.doMkdirLike(1, "mkdir", m.tmp+"/o", ""),
				m.doMkdirLike(1, "mkdir", m.tmp+"/u.o", ""),
				m.doStat(1, "/package.json", true),
				m.doRenameLink(1, "rename", m.tmp+"/o", m.tmp+"/u.o", true),
				m.doStat(1, "/var/x", true))
		})
		wantExactReasons(t, rec, "host-read:/package.json", "host-read:/var/x")
	})
	// And the same absences with NO move are constants: the control that says
	// the two records above differ only by the taint.
	t.Run("no move: the absences are constants", func(t *testing.T) {
		rec, _ := runModelOps(t, func(m *model) []trace.Event {
			return cat(
				m.doStat(1, "/package.json", true),
				m.doMkdirLike(1, "mkdir", m.tmp+"/o", ""),
				m.doStat(1, "/opt/x", true))
		})
		wantExactReasons(t, rec)
	})
	// Burst 27 with the cwd directory NOT made again: the text walk stops at the
	// renamed-away cwd directory and a ".." follows.
	t.Run("burst 27 variant: a renamed-away cwd, failed probe through dotdot", func(t *testing.T) {
		rec, census := runModelOps(t, func(m *model) []trace.Event {
			return cat(
				m.doMkdirLike(1, "mkdir", m.tmp+"/o", ""),
				m.doChdir(1, m.tmp+"/o"),
				m.doRenameLink(1, "rename", m.tmp+"/o", m.tmp+"/u.o", false),
				m.doStat(1, "gone/../x", true))
		})
		// The reasons are the taint's: the sealed-scratch absence of the cwd
		// directory turned back into a host-read, and unstable-path.  The
		// rename of a directory also trips the strict stability rule
		// (final-changed), which alone would raise unstable-path, so the REASON
		// cannot tell the dotdot refusal from it: the census sub-code does.
		wantExactReasons(t, rec, "host-read:$TMPDIR/o", rUnstable)
		want := map[string]string{
			"host-read":                                    "$TMPDIR/o",
			"trace-incomplete:unstable-path:dotdot":        "$TMPDIR/o/gone/../x",
			"trace-incomplete:unstable-path:final-changed": "$TMPDIR/o",
		}
		got := censusKinds(census)
		if len(got) != len(want) {
			t.Fatalf("census %v, want exactly %v", got, want)
		}
		for kind, p := range want {
			if len(got[kind]) != 1 || got[kind][0] != p {
				t.Fatalf("census %v, want exactly %v", got, want)
			}
		}
	})
}
