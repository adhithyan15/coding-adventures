package main

import (
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
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

// crossCheckReport is what one directory's check found.  Accepted counts the
// traces the parse ACCEPTED (no refusal): a check whose parameters drifted (a
// wrong helper executable or working directory) has both readers refuse every
// trace identically, which is "agreement" and would read as mismatches=0, so
// the census line carries accepted=K and CI requires it to be at least 1.
type crossCheckReport struct {
	Traces, Accepted, Bytes, Mismatches, Allowed int
	// Problems are the failure lines (no path text; a trace is named by its
	// quoted base name), and Failed the paths to Diagnose.
	Problems []string
	Failed   []string
}

// traceParams are how a child parsed one trace.  MustAccept, when set, says why
// the child's own verdict was "parsed": the test's parse must then accept it too.
type traceParams struct {
	cwd        string
	limits     trace.Limits
	mustAccept string
}

// childInfo is what the sealed `pre/` says about the children's parses.
type childInfo struct {
	kilnExe  string
	known    map[string]traceParams
	verified bool // pre/ verified: every trace must be one of known
}

// crossCheckedDirs remembers the directories already checked in this process, so
// a directory registered twice is counted once.
var (
	crossCheckedMu   sync.Mutex
	crossCheckedDirs = map[string]bool{}
)

// crossCheckChildTraces registers the D34 check for recDir.  kilnExe is the kiln
// binary the children run (the helper executable of their traces); it is used
// when `pre/` cannot be verified, e.g. because prepare refused a probe trace,
// which is exactly the case the check must still see.
//
// Call it BEFORE `prepare` runs, once the temp dir exists: cleanups run
// last-in first-out, so the check then runs after the children are finished and
// before the temp dir (registered earlier) is deleted.
//
// The parse behind it keeps one process-global counter of allowed divergences
// (trace.AllowedDivergences), so the check, and the tests that call it, must not
// run in parallel: no t.Parallel.
func crossCheckChildTraces(t *testing.T, recDir, kilnExe string) {
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
		rep := crossCheckDir(recDir, kilnExe, crossCheckParse)
		// The census line.  assert_sandbox_tests.py reads it: traces >= 1 and
		// accepted >= 1 in total over the passing tests, so the check cannot go
		// vacuous.  Every field is a count: no text can ride on the line.
		t.Logf("PROBE: crosscheck traces=%d accepted=%d bytes=%d mismatches=%d allowed=%d", rep.Traces, rep.Accepted, rep.Bytes, rep.Mismatches, rep.Allowed)
		for _, p := range rep.Problems {
			t.Errorf("cross-check of a child's trace: %s", p)
		}
		for _, p := range rep.Failed {
			logTraceFile(t, p) // the D32 diagnostic: bounded and sanitised
		}
	})
}

// crossCheckDir checks every *.trace under recDir.  It never fails a test itself.
func crossCheckDir(recDir, kilnExe string, parse func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, int)) crossCheckReport {
	var rep crossCheckReport
	info := childParams(recDir, kilnExe)
	var paths []string
	err := filepath.WalkDir(recDir, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			if p == recDir && errors.Is(err, fs.ErrNotExist) {
				return filepath.SkipAll // the test stopped before the directory existed
			}
			rep.Problems = append(rep.Problems, fmt.Sprintf("cannot walk the record directory at %q", filepath.Base(p)))
			return nil
		}
		if d.Type().IsRegular() && strings.HasSuffix(p, ".trace") {
			paths = append(paths, p)
		}
		return nil
	})
	if err != nil {
		rep.Problems = append(rep.Problems, "cannot walk the record directory")
	}
	sort.Strings(paths)
	for _, p := range paths {
		name := strconv.Quote(filepath.Base(p))
		params, ok := info.known[p]
		if !ok {
			if info.verified {
				rep.Problems = append(rep.Problems, "trace "+name+" is not one the verified pre/ accounts for (an unknown action or step)")
			}
			// Generic parameters, so it is still read by both readers.
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
		// ParseCrossChecked reads the process-global allowed-divergence counter
		// (trace.AllowedDivergences) before and after: never call it from parallel
		// tests (no t.Parallel here), or one trace would be charged another's hits.
		_, perr, m, allowed := parse(f, params.cwd, trace.Options{Limits: params.limits, HelperRoot: true, HelperExe: info.kilnExe})
		f.Close()
		rep.Traces++
		if m == nil && perr == nil {
			rep.Accepted++
		}
		if m != nil {
			rep.Mismatches++
			rep.Problems = append(rep.Problems, fmt.Sprintf("trace %s: %s", name, m.Error()))
			rep.Failed = append(rep.Failed, p)
		} else if perr != nil && params.mustAccept != "" {
			// Both readers refuse alike, but the child ACCEPTED this trace: the
			// check's parameters differ from the child's, or the parse is not
			// deterministic.  Either way the census would be meaningless.
			rep.Problems = append(rep.Problems, fmt.Sprintf("trace %s was refused here (%s) although %s", name, errorReason(perr), params.mustAccept))
			rep.Failed = append(rep.Failed, p)
		}
		if allowed > 0 {
			rep.Allowed += allowed
			rep.Problems = append(rep.Problems, fmt.Sprintf("trace %s took %d allowed stricter-only divergence(s); on a real trace there must be none", name, allowed))
			if m == nil && !(perr != nil && params.mustAccept != "") {
				rep.Failed = append(rep.Failed, p)
			}
		}
	}
	return rep
}

// errorReason is a refusal's reason (no path text: a trace.Error carries a
// reason and a line), or a fixed word for any other error.
func errorReason(err error) string {
	var te *trace.Error
	if errors.As(err, &te) {
		return fmt.Sprintf("%s, line %d", te.Reason, te.Line)
	}
	return "an error"
}

var stepTraceName = regexp.MustCompile(`^[0-9]+\.trace$`)

// childParams recovers, from the sealed `pre/` directory, how the children parsed
// each trace: the helper binary, and per trace the working directory, the Limits
// (parseHelperTrace, parseStep, probeLimits and ClassifyLimits in
// internal/record) and, where a child's own verdict is on disk, that the trace
// was accepted:
//
//   - a probe run listed in the sealed probes.json parsed (prepare refuses
//     otherwise);
//   - a step trace of an action whose record.json says parse_ok parsed.
//
// The steps are found from the directory listing, not a fixed range.  When `pre/`
// cannot be verified (prepare did not finish) the helper is the test's own kiln
// binary and nothing is known: the check is then only weaker, never wrong.
func childParams(recDir, fallbackExe string) childInfo {
	exe := fallbackExe
	if r, err := filepath.EvalSymlinks(exe); err == nil {
		exe = r
	}
	info := childInfo{kilnExe: exe, known: map[string]traceParams{}}
	pre, err := record.VerifyPre(recDir)
	if err != nil {
		return info
	}
	info.kilnExe, info.verified = pre.Config.KilnExe, true
	for _, action := range pre.Config.Actions() {
		why := ""
		if data, err := os.ReadFile(record.RecordFile(recDir, action)); err == nil {
			if rec, err := record.DecodeClassified(data); err == nil && rec.ParseOK {
				why = "record.json says every step trace of the action parsed"
			}
		}
		entries, _ := os.ReadDir(record.StepDir(recDir, action))
		for _, e := range entries {
			if stepTraceName.MatchString(e.Name()) {
				info.known[filepath.Join(record.StepDir(recDir, action), e.Name())] = traceParams{pre.Config.PackageDir(action), record.ClassifyLimits(), why}
			}
		}
	}
	probes, _ := filepath.Glob(filepath.Join(recDir, record.ProbeDir, "*.trace"))
	for _, p := range probes {
		info.known[p] = traceParams{cwd: pre.Config.PackageDir(pre.Probes.Action), limits: record.ProbeParseLimits()}
	}
	for _, r := range pre.Probes.Runs {
		p := filepath.Join(recDir, record.ProbeDir, r.Name+".trace")
		if tp, ok := info.known[p]; ok {
			tp.mustAccept = "prepare succeeded, so its probe run parsed"
			info.known[p] = tp
		}
	}
	return info
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
	rep := crossCheckDir(dir, "/kiln", trace.ParseCrossChecked)
	if rep.Traces != 2 || rep.Accepted != 0 || rep.Bytes != len("not a trace\n")+len("also not\n") || rep.Mismatches != 0 || rep.Allowed != 0 || len(rep.Problems) != 0 {
		t.Errorf("report = %+v", rep)
	}
}

// Accepted counts the traces the parse accepted, and a refusal is not one; this
// is what keeps a drifted check from reading as a clean one.
func TestCrossCheckDirCountsAcceptance(t *testing.T) {
	skipCrossCheckOnWindows(t)
	dir := t.TempDir()
	writeFakeTrace(t, filepath.Join(dir, "x.trace"), "x\n")
	writeFakeTrace(t, filepath.Join(dir, "y.trace"), "y\n")
	fake := func(r io.Reader, cwd string, o trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, int) {
		b, _ := io.ReadAll(r)
		if string(b) == "x\n" {
			return &trace.Trace{}, nil, nil, 0
		}
		return nil, &trace.Error{Reason: "unparsable-line", Line: 1}, nil, 0
	}
	rep := crossCheckDir(dir, "/kiln", fake)
	if rep.Traces != 2 || rep.Accepted != 1 || len(rep.Problems) != 0 {
		t.Errorf("report = %+v", rep)
	}
}

// A walk error is a problem, not silence.
func TestCrossCheckDirReportsAWalkError(t *testing.T) {
	skipCrossCheckOnWindows(t)
	if os.Geteuid() == 0 {
		t.Skip("root reads any directory")
	}
	dir := t.TempDir()
	locked := filepath.Join(dir, "locked")
	writeFakeTrace(t, filepath.Join(locked, "x.trace"), "x\n")
	if err := os.Chmod(locked, 0); err != nil {
		t.Fatal(err)
	}
	defer os.Chmod(locked, 0o700)
	rep := crossCheckDir(dir, "/kiln", trace.ParseCrossChecked)
	if len(rep.Problems) == 0 {
		t.Errorf("a directory that cannot be read was not reported: %+v", rep)
	}
}

// A missing record directory (the test stopped first) is not a walk error.
func TestCrossCheckDirOfAMissingDirectoryIsEmpty(t *testing.T) {
	rep := crossCheckDir(filepath.Join(t.TempDir(), "absent"), "/kiln", trace.ParseCrossChecked)
	if rep.Traces != 0 || len(rep.Problems) != 0 {
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
	rep := crossCheckDir(dir, "/kiln", fake)
	if rep.Traces != 2 || rep.Accepted != 1 || rep.Mismatches != 1 || rep.Allowed != 2 || len(rep.Problems) != 2 || len(rep.Failed) != 2 {
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
	t.Run("register", func(t *testing.T) { crossCheckChildTraces(t, dir, "/kiln") })
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
		crossCheckChildTraces(t, dir, "/kiln")
		crossCheckChildTraces(t, dir, "/kiln")
	})
	if calls != 1 {
		t.Errorf("parse calls = %d, want 1", calls)
	}
}
