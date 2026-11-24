package traced

import (
	"bytes"
	"sort"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// The exec-head case of the DIFFERENTIAL FUZZER (KILN04 D48 step 5).
//
// The model (model_test.go) knows the truth of every exec it makes: whether it
// succeeded and which bytes the FIRST program had at that instant, which is what
// the supervisor would have pinned.  From that this file writes, for each step,
// the `.exechead` sidecar a healthy supervisor would have left (one line per exec
// event, in trace order: `confirmed` with the true head for a successful exec,
// `failed` for the rest, `outside` for the step's own shell) and checks three
// things against the classifier:
//
//  1. HONEST.  Classified with that sidecar, the action is judged by the same
//     oracles as the plain record (judgeRecord).  This is the case that matters:
//     an action that execs a file under the tmpfs is always refused without a
//     sidecar, so only with one does it ever reach the read-set oracle.  A
//     believed head makes the classifier walk the loader chain and observe the
//     interpreters; if it forgot one, a lookup the model made is uncovered and the
//     oracle fails.  The classifier must also not refuse the honest sidecar with
//     any `exechead-*` sub-code.
//
//  2. SLIPPED.  The sidecar is perturbed the way a real recording can go wrong
//     (a line missing or repeated, a non-leader line, a status that is not
//     `confirmed`, a path that differs, a kernel off the table, two lines
//     swapped) and EVERY exec the plain classifier refused as `under-tmp` must
//     still be refused, now under an `exechead-*` code: a slip must never turn
//     into a believed head.  (The perturbation is applied to every pid, so the
//     set of refused paths cannot shrink for a pid the perturbation skipped.)
//
// Both runs use the model's own steps with the init pid set (the pairing needs
// a helper trace's anchor).  The data bytes choose the perturbation, so a seed
// reproduces.

// ehStats counts what the exec-head case really exercised, so a change to the
// model that silently stopped producing execs under the tmpfs cannot leave the
// case vacuous (TestDifferentialExecHeadIsNotVacuous).
type ehCounters struct {
	withExec       int // steps' worth of actions with at least one successful exec
	honestClean    int // honest sidecar, no reason: the read-set oracle ran on an action with an exec under the tmpfs
	slipped        [13]int
	slippedRefused int // perturbed runs in which the plain classifier had an under-tmp refusal to preserve
	// perRefused[k] counts the runs of perturbation k that had an under-tmp
	// refusal to preserve (so applied AND refused, not merely applied).
	perRefused [13]int
}

// ehStats counts the random generator's cases; ehBias the biased generator's
// (exechead_biased_test.go).
var ehStats, ehBias ehCounters

// TestDifferentialExecHeadIsNotVacuous runs the seeded differential and demands
// that the exec-head case reached its three situations: an honest sidecar that
// the read-set oracle judged, and every perturbation kind with an under-tmp
// refusal to preserve.
func TestDifferentialExecHeadIsNotVacuous(t *testing.T) {
	if testing.Short() {
		t.Skip("the seeded differential runs 400 seeds under -short, too few to reach every situation")
	}
	ehStats = ehCounters{}
	TestDifferentialSeeded(t)
	t.Logf("exec-head differential: %+v", ehStats)
	// Most random actions have some other reason, so few honest ones are clean
	// (3 of 4000 seeds at the time of writing): at least one must be.
	if ehStats.withExec < 50 || ehStats.honestClean < 1 || ehStats.slippedRefused < 50 {
		t.Errorf("the exec-head case is (nearly) vacuous: %+v", ehStats)
	}
	for k := 1; k < 9; k++ {
		if ehStats.slipped[k] == 0 {
			t.Errorf("perturbation %d was never applied", k)
		}
	}
}

// execHeadLines builds the honest sidecar of step i, or nil when the model and
// the trace do not describe the same execs (nothing to say about it).
func execHeadLines(m *model, i int, st *trace.Trace) ([]exechead.Line, bool, bool) {
	queue := map[int][]execRec{}
	for _, r := range m.stepExecs[i] {
		queue[r.pid] = append(queue[r.pid], r)
	}
	var lines []exechead.Line
	any := false
	for k := range st.Events {
		ev := &st.Events[k]
		if !execEvent(ev) {
			continue
		}
		if k == 0 && ev.Pid == 1 && ev.Path == "/usr/bin/sh" {
			lines = append(lines, exechead.Line{Tid: 1, Tgid: 1, Syscall: exechead.SyscallExecve, Path: ev.Path, Status: exechead.StatusOutside})
			continue
		}
		q := queue[ev.Pid]
		if len(q) == 0 || q[0].path != ev.Path || q[0].ok != ev.Result.OK {
			return nil, false, false
		}
		r := q[0]
		queue[ev.Pid] = q[1:]
		l := exechead.Line{Tid: ev.Pid, Tgid: ev.Pid, Syscall: exechead.SyscallExecve, Path: ev.Path, Status: exechead.StatusFailed}
		switch {
		case r.ok && bytes.HasPrefix(r.head, []byte("#!")):
			// A script: /proc/<tgid>/exe becomes the interpreter, never the pinned
			// file, so a real supervisor writes `mismatch` (or `gone`), never
			// `confirmed`.  The classifier refuses it as exechead-mismatch.
			l.Status = exechead.StatusMismatch
			any = true
		case r.ok:
			l.Status, l.Dev, l.Ino, l.Head = exechead.StatusConfirmed, 1, 1, r.head
			any = true
		}
		lines = append(lines, l)
	}
	for _, q := range queue {
		if len(q) != 0 {
			return nil, false, false
		}
	}
	return lines, any, true
}

func renumber(lines []exechead.Line) []exechead.Line {
	for i := range lines {
		lines[i].Seq = i
	}
	return lines
}

// perturbExecHead applies perturbation kind to a copy of lines (and the header's
// kernel).  It says whether the kind applied to EVERY pid that has a line, which
// is what lets the caller demand that no refusal shrinks.
func perturbExecHead(kind int, kernel string, lines []exechead.Line) ([]exechead.Line, string, bool) {
	out := append([]exechead.Line(nil), lines...)
	first := map[int]int{} // tgid -> index of its first line
	byPid := map[int][]int{}
	for i, l := range out {
		if _, ok := first[l.Tgid]; !ok {
			first[l.Tgid] = i
		}
		byPid[l.Tgid] = append(byPid[l.Tgid], i)
	}
	switch kind {
	case 1: // a missing line, for every pid
		skip := map[int]bool{}
		for _, i := range first {
			skip[i] = true
		}
		var kept []exechead.Line
		for i, l := range out {
			if !skip[i] {
				kept = append(kept, l)
			}
		}
		return renumber(kept), kernel, true
	case 2: // a repeated line, for every pid
		dup := map[int]bool{}
		for _, i := range first {
			dup[i] = true
		}
		var more []exechead.Line
		for i, l := range out {
			more = append(more, l)
			if dup[i] {
				more = append(more, l)
			}
		}
		return renumber(more), kernel, true
	case 3: // a non-leader line, for every pid
		for _, i := range first {
			out[i].Tid = out[i].Tgid + 1000
		}
		return out, kernel, true
	case 4: // no evidence: every confirmed line is `gone`
		for i := range out {
			if out[i].Status == exechead.StatusConfirmed {
				out[i].Status, out[i].Head, out[i].Dev, out[i].Ino = exechead.StatusGone, nil, 0, 0
			}
		}
		return out, kernel, true
	case 5: // a path that is not the trace's
		for i := range out {
			if out[i].Status == exechead.StatusConfirmed {
				out[i].Path += "x"
			}
		}
		return out, kernel, true
	case 6: // a kernel that is not on the table
		return out, "6.8.0-generic", true
	case 8: // a status that contradicts the result: a failed exec's line says `confirmed`
		// Applied when at least one pid has a failed exec to contradict.
		contradicted := false
		for _, idx := range byPid {
			hit := false
			for _, k := range idx {
				if out[k].Status == exechead.StatusFailed {
					out[k].Status, out[k].Dev, out[k].Ino, out[k].Head = exechead.StatusConfirmed, 1, 1, []byte("\x7fELF")
					hit = true
				}
			}
			if !hit {
				// No failed exec in this pid to contradict: refuse it another way
				// (a non-leader line), so the oracle's "no refusal may disappear"
				// still holds for every pid.
				out[idx[0]].Tid = out[idx[0]].Tgid + 1000
			} else {
				contradicted = true
			}
		}
		return out, kernel, contradicted
	case 9: // the wrong syscall: every line says execveat (the trace has execve)
		for i := range out {
			out[i].Syscall = exechead.SyscallExecveat
		}
		return out, kernel, true
	case 10, 12: // applied by the caller (it changes the trace, not the sidecar)
		return out, kernel, true
	case 11: // a confirmed line whose head is a script: exe never becomes one
		for i := range out {
			if out[i].Status == exechead.StatusConfirmed {
				out[i].Head = []byte("#!/usr/bin/sh\n")
			}
		}
		return out, kernel, true
	case 7: // an ordinal slip: every pid's lines shifted by one place (cyclically)
		// Every line moves, so every exec is paired with its NEIGHBOUR's line.  It
		// applies only when no two cyclically adjacent lines of a pid have the same
		// path: a slip onto a line with the very same path is not detectable (and
		// is the forged-sidecar boundary, which the nonce and the digest guard, not
		// the pairing).  A swap of two OTHER lines is not a slip for the exec under
		// test either, which is why the whole sequence is rotated.
		all := true
		for _, idx := range byPid {
			if len(idx) < 2 {
				all = false
				continue
			}
			for i := range idx {
				if out[idx[i]].Path == out[idx[(i+1)%len(idx)]].Path {
					all = false
				}
			}
		}
		if !all {
			return out, kernel, false
		}
		for _, idx := range byPid {
			old := make([]exechead.Line, len(idx))
			for i, k := range idx {
				old[i] = lines[k]
			}
			for i, k := range idx {
				n := old[(i+1)%len(idx)]
				n.Tid, n.Tgid = lines[k].Tid, lines[k].Tgid
				out[k] = n
			}
		}
		return out, kernel, true
	}
	return out, kernel, false
}

func execUnreadablePaths(c *Census, subPrefix string) map[string]bool {
	out := map[string]bool{}
	for k, paths := range censusKinds(c) {
		if strings.HasPrefix(k, "trace-incomplete:exec-unreadable:"+subPrefix) {
			for _, p := range paths {
				out[p] = true
			}
		}
	}
	return out
}

func checkExecHeads(t *testing.T, m *model, tree *memTree, h *harness, steps []*trace.Trace, recipe []string, data []byte, plain *Census, biased bool) {
	t.Helper()
	if len(m.stepExecs) != len(steps) {
		return
	}
	traces := make([]*trace.Trace, len(steps))
	lines := make([][]exechead.Line, len(steps))
	anyOK := false
	for i, st := range steps {
		cp := *st
		cp.InitPid = 1
		traces[i] = &cp
		ls, any, ok := execHeadLines(m, i, st)
		if !ok {
			return
		}
		lines[i] = renumber(ls)
		anyOK = anyOK || any
	}
	if !anyOK {
		return // no successful exec: nothing a head could change
	}
	st := &ehStats
	if biased {
		st = &ehBias
	}
	st.withExec++
	files := func(kind int) ([]*exechead.File, bool) {
		fs := make([]*exechead.File, len(steps))
		applied := true
		for i := range steps {
			ls, kernel, all := lines[i], ehKernel, true
			if kind != 0 {
				ls, kernel, all = perturbExecHead(kind, ehKernel, lines[i])
			}
			applied = applied && all
			fs[i] = &exechead.File{Header: exechead.Header{V: exechead.Version, Kernel: kernel}, Lines: renumber(append([]exechead.Line(nil), ls...))}
		}
		return fs, applied
	}
	classify := func(fs []*exechead.File, trs ...[]*trace.Trace) (*RawRecord, *Census, bool) {
		c := NewCensus()
		use := traces
		if len(trs) > 0 {
			use = trs[0]
		}
		rec, err := Classify(h.cfg, Input{Package: "code/p/a", Traces: use, Recipe: recipe, StepNetNS: !m.noNetNS, Census: c, ExecHeads: fs})
		return rec, c, err == nil
	}

	// 1. Honest.
	honest, _ := files(0)
	rec, c, ok := classify(honest)
	if !ok {
		return // the action moved or removed the checkout's own directories
	}
	for k := range censusKinds(c) {
		if strings.HasPrefix(k, "trace-incomplete:exec-unreadable:exechead-") && k != "trace-incomplete:exec-unreadable:exechead-mismatch" {
			t.Fatalf("an honest sidecar was refused as %s (census %v)\nroot %s home %s tmp %s", k, censusKinds(c), m.root, m.home, m.tmp)
		}
	}
	if biased && len(rec.Reasons) != 0 {
		t.Fatalf("a biased case has no refusal but the exec under the tmpfs, and the honest sidecar answers it, yet the record has reasons %q (census %v)\nlines %+v", rec.Reasons, censusKinds(c), lines)
	}
	if len(rec.Reasons) == 0 {
		st.honestClean++ // reached the read-set oracle: the case no plain record can reach
	}
	judgeRecord(t, m, tree, rec, c)

	// 2. Slipped.  The random generator applies one perturbation chosen by the
	// data; the biased one applies every kind to every case.
	want := execUnreadablePaths(plain, "under-tmp")
	kinds := []int{1 + int(data[len(data)/2])%8}
	if biased {
		// 9: every line says execveat; 10: the init pid is unknown (the trace is
		// not a helper trace); 11: a confirmed line whose head is a `#!` script.
		kinds = []int{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12}
	}
	for _, kind := range kinds {
		slipped, applied := files(kind)
		if !applied {
			continue
		}
		if kind != 12 {
			st.slipped[kind]++
		}
		var rec2 *RawRecord
		var c2 *Census
		var ok bool
		if kind == 10 || kind == 12 {
			alt := make([]*trace.Trace, len(traces))
			whole := true // kind 12: every pid with an exec has a failed one to blank
			for i, tr := range traces {
				cp := *tr
				cp.Events = append([]trace.Event(nil), tr.Events...)
				if kind == 10 {
					cp.InitPid = 0
				} else {
					blanked := map[int]bool{}
					seen := map[int]bool{}
					for k := range cp.Events {
						ev := &cp.Events[k]
						if !execEvent(ev) {
							continue
						}
						seen[ev.Pid] = true
						if !ev.Result.OK && !blanked[ev.Pid] {
							// strace's `= ?`: the exec never reported a result.
							ev.Result.Errno = "?"
							blanked[ev.Pid] = true
						}
					}
					for pid := range seen {
						whole = whole && blanked[pid]
					}
				}
				alt[i] = &cp
			}
			if !whole {
				continue
			}
			if kind == 12 {
				st.slipped[kind]++
			}
			rec2, c2, ok = classify(slipped, alt)
		} else {
			rec2, c2, ok = classify(slipped)
		}
		if !ok {
			continue
		}
		got := execUnreadablePaths(c2, "")
		if len(want) > 0 {
			st.slippedRefused++
			st.perRefused[kind]++
		}
		var missing []string
		for p := range want {
			if !got[p] {
				missing = append(missing, p)
			}
		}
		if len(missing) > 0 {
			sort.Strings(missing)
			t.Fatalf("perturbation %d of the exec-head sidecar made a refused exec believed: %q are no longer refused\nreasons %q census %v\nlines %+v",
				kind, missing, rec2.Reasons, censusKinds(c2), lines)
		}
	}
}
