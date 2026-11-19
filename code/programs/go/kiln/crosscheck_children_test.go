package main

import (
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"sync"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// This file is D34: the in-test cross-check of every trace a CHILD process wrote.
//
// # The gap it closes
//
// The kiln-sandbox CI job sets KILN_TRACE_DIFF=1 for the `go test` process, and
// that makes trace.ParseOptions run BOTH readers (the grammar path and the old
// scanner) and demand that they agree.  But the real recordings are made by
// CHILD processes: `kiln record prepare` (its structure probes), `kiln record
// step`, the classify child and `collect`.  A test starts each with a hand-built
// environment, and that environment must not carry KILN_TRACE_DIFF: recordcmd.go
// seals the environment into facts and keys, so a debugging switch could leak
// into sealed data.  So every production parse of a real trace was grammar-only
// and never cross-checked, and the 14-day / 200-run soak (KILN04 section 4.9)
// saw almost none of them.
//
// # The fix
//
// The traces are files.  When a sandbox test's children are done, the TEST
// process, which does have the CI environment, reads every trace file under the
// record directory back and runs trace.ParseCrossChecked on it with the Limits,
// helper-root setting and working directory the child used.  Any disagreement
// is a test failure; so is an ALLOWED stricter-only divergence, because on a
// real trace the spec's rule is that there are none (a hit there would void that
// recording once the old scanner is gone).
//
// The check is a t.Cleanup registered right after the record directory is
// sealed.  Cleanups run last-in first-out, so it runs after the test body (the
// children are finished) and before the t.TempDir cleanup that deletes the
// traces (that one was registered when the test first asked for a temp dir).

// crossCheckEnabled says whether this test process is the cross-check one: the
// kiln-sandbox CI job sets KILN_TRACE_DIFF=1; a local run does not, and stays
// fast.
func crossCheckEnabled() bool { return os.Getenv("KILN_TRACE_DIFF") == "1" }

// crossCheckParse is the parse the check runs.  A variable so a unit test can
// stand in a mismatch, which no real trace is known to produce.
var crossCheckParse = trace.ParseCrossChecked

// crossCheckReport is what one directory's check found.
type crossCheckReport struct {
	Traces, Bytes, Mismatches, Allowed int
	// Problems are the failure lines (no path text; a trace is named by its
	// quoted base name), and Failed the paths to Diagnose.
	Problems []string
	Failed   []string
}

// traceParams are how a child parsed one trace.
type traceParams struct {
	cwd    string
	limits trace.Limits
}

// crossCheckedDirs remembers the directories already checked in this process, so
// a directory registered twice is counted once.
var (
	crossCheckedMu   sync.Mutex
	crossCheckedDirs = map[string]bool{}
)

// crossCheckChildTraces registers the D34 check for recDir.  Call it once the
// record directory is known and the temp dir exists, so that this cleanup is
// registered after the temp dir's and therefore runs before it.
func crossCheckChildTraces(t *testing.T, recDir string) {
	t.Helper()
	if !crossCheckEnabled() {
		return
	}
	t.Cleanup(func() {
		crossCheckedMu.Lock()
		done := crossCheckedDirs[recDir]
		crossCheckedDirs[recDir] = true
		crossCheckedMu.Unlock()
		if done {
			return
		}
		rep := crossCheckDir(recDir, crossCheckParse)
		// The census line.  assert_sandbox_tests.py reads it: it must say traces >= 1
		// in at least one passing test, so the check cannot go vacuous.
		t.Logf("PROBE: crosscheck traces=%d bytes=%d mismatches=%d allowed=%d", rep.Traces, rep.Bytes, rep.Mismatches, rep.Allowed)
		for _, p := range rep.Problems {
			t.Errorf("cross-check of a child's trace: %s", p)
		}
		for _, p := range rep.Failed {
			logTraceFile(t, p) // the D32 diagnostic: bounded and sanitised
		}
	})
}

// crossCheckDir checks every *.trace under recDir.  It never fails a test itself.
func crossCheckDir(recDir string, parse func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, int)) crossCheckReport {
	var rep crossCheckReport
	kilnExe, known := childParams(recDir)
	var paths []string
	filepath.WalkDir(recDir, func(p string, d fs.DirEntry, err error) error {
		if err == nil && d.Type().IsRegular() && strings.HasSuffix(p, ".trace") {
			paths = append(paths, p)
		}
		return nil
	})
	sort.Strings(paths)
	for _, p := range paths {
		name := strconv.Quote(filepath.Base(p))
		params, ok := known[p]
		if !ok { // a trace no child parse is known for: generic limits
			params = traceParams{cwd: "/", limits: record.ClassifyLimits()}
		}
		f, err := openTraceNoFollow(p)
		if err != nil {
			rep.Problems = append(rep.Problems, "cannot open trace "+name)
			continue
		}
		if st, err := f.Stat(); err == nil {
			rep.Bytes += int(st.Size())
		}
		_, _, m, allowed := parse(f, params.cwd, trace.Options{Limits: params.limits, HelperRoot: true, HelperExe: kilnExe})
		f.Close()
		rep.Traces++
		if m != nil {
			rep.Mismatches++
			rep.Problems = append(rep.Problems, fmt.Sprintf("trace %s: %s", name, m.Error()))
			rep.Failed = append(rep.Failed, p)
		}
		if allowed > 0 {
			rep.Allowed += allowed
			rep.Problems = append(rep.Problems, fmt.Sprintf("trace %s took %d allowed stricter-only divergence(s); on a real trace there must be none", name, allowed))
			if m == nil {
				rep.Failed = append(rep.Failed, p)
			}
		}
	}
	return rep
}

// childParams recovers, from the sealed `pre/` directory, how the children parsed
// each trace: the helper binary, and per trace the working directory and Limits
// (parseHelperTrace, parseStep, probeLimits and ClassifyLimits in
// internal/record).  When the directory cannot be verified (prepare did not
// finish) it returns a placeholder helper and no known traces, which makes the
// check only weaker, never wrong.
func childParams(recDir string) (kilnExe string, known map[string]traceParams) {
	known = map[string]traceParams{}
	pre, err := record.VerifyPre(recDir)
	if err != nil {
		return "/kiln", known
	}
	for _, action := range pre.Config.Actions() {
		for i := 0; i < 64; i++ {
			known[record.StepFile(recDir, action, i, "trace")] = traceParams{pre.Config.PackageDir(action), record.ClassifyLimits()}
		}
	}
	probes, _ := filepath.Glob(filepath.Join(recDir, record.ProbeDir, "*.trace"))
	for _, p := range probes {
		known[p] = traceParams{pre.Config.PackageDir(pre.Probes.Action), record.ProbeParseLimits()}
	}
	return pre.Config.KilnExe, known
}

// ---------------------------------------------------------------------------
// Unit tests.  They use POSIX paths and strace-shaped data, so they skip on
// Windows (the build job there runs kiln's tests; see the lesson on portable
// kiln tests).

func skipCrossCheckOnWindows(t *testing.T) {
	t.Helper()
	if runtime.GOOS == "windows" {
		t.Skip("POSIX paths and strace fixtures: the traces are Linux-shaped")
	}
}

func writeFakeTrace(t *testing.T, path, body string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(body), 0o600); err != nil {
		t.Fatal(err)
	}
}

// Garbage is refused by both readers with the same reason: they agree, so the
// check counts the trace and its bytes and reports nothing wrong.
func TestCrossCheckDirCountsAgreement(t *testing.T) {
	skipCrossCheckOnWindows(t)
	dir := t.TempDir()
	writeFakeTrace(t, filepath.Join(dir, record.ProbeDir, "a.trace"), "not a trace\n")
	writeFakeTrace(t, filepath.Join(dir, "traces", "b.trace"), "also not\n")
	writeFakeTrace(t, filepath.Join(dir, "traces", "b.json"), "{}")
	rep := crossCheckDir(dir, trace.ParseCrossChecked)
	if rep.Traces != 2 || rep.Bytes != len("not a trace\n")+len("also not\n") || rep.Mismatches != 0 || rep.Allowed != 0 || len(rep.Problems) != 0 {
		t.Errorf("report = %+v", rep)
	}
}

// A mismatch, and an allowed divergence, are both failures with no raw path.
func TestCrossCheckDirReportsMismatchAndAllowed(t *testing.T) {
	skipCrossCheckOnWindows(t)
	dir := t.TempDir()
	writeFakeTrace(t, filepath.Join(dir, "x.trace"), "x\n")
	writeFakeTrace(t, filepath.Join(dir, "y.trace"), "y\n")
	fake := func(r io.Reader, cwd string, o trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, int) {
		b, _ := io.ReadAll(r)
		if string(b) == "x\n" {
			return nil, nil, &trace.CrossCheckMismatch{Detail: "FAIL-OPEN: test"}, 0
		}
		return nil, nil, nil, 2
	}
	rep := crossCheckDir(dir, fake)
	if rep.Traces != 2 || rep.Mismatches != 1 || rep.Allowed != 2 || len(rep.Problems) != 2 || len(rep.Failed) != 2 {
		t.Fatalf("report = %+v", rep)
	}
	for _, p := range rep.Problems {
		if strings.Contains(p, dir) {
			t.Errorf("a problem line holds a path: %s", p)
		}
	}
}

// Without KILN_TRACE_DIFF=1 the registration does nothing: a local run reads no trace.
func TestCrossCheckChildTracesIsOffWithoutTheSwitch(t *testing.T) {
	t.Setenv("KILN_TRACE_DIFF", "")
	called := false
	old := crossCheckParse
	crossCheckParse = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, int) {
		called = true
		return nil, nil, nil, 0
	}
	defer func() { crossCheckParse = old }()
	dir := t.TempDir()
	writeFakeTrace(t, filepath.Join(dir, "x.trace"), "x\n")
	t.Run("register", func(t *testing.T) { crossCheckChildTraces(t, dir) })
	if called {
		t.Error("the cross-check ran without KILN_TRACE_DIFF=1")
	}
}

// With the switch on, registering the same directory twice checks it once.
func TestCrossCheckChildTracesCountsADirectoryOnce(t *testing.T) {
	skipCrossCheckOnWindows(t)
	t.Setenv("KILN_TRACE_DIFF", "1")
	calls := 0
	old := crossCheckParse
	crossCheckParse = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, int) {
		calls++
		return nil, nil, nil, 0
	}
	defer func() { crossCheckParse = old }()
	dir := t.TempDir()
	writeFakeTrace(t, filepath.Join(dir, "x.trace"), "x\n")
	t.Run("twice", func(t *testing.T) {
		crossCheckChildTraces(t, dir)
		crossCheckChildTraces(t, dir)
	})
	if calls != 1 {
		t.Errorf("parse calls = %d, want 1", calls)
	}
}
