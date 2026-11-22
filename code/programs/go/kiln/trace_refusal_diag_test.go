package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// This file is D32's in-test diagnostic for the kiln-sandbox CI job.
//
// The incident: TestSandboxRecordStep failed with "structure probe: probe walk:
// the trace does not parse: trace-incomplete:unparsable-line (trace: /tmp/.../
// walk.trace)" and the trace was deleted with t.TempDir before anyone could look
// at it.  The children that run `kiln record prepare` have hand-built
// environments without KILN_TRACE_DIFF, so the parse inside them was
// grammar-only, never cross-checked against the old scanner.
//
// So the TEST process, which does have the environment of the CI job, reads the
// trace back at the moment the child fails and asks trace.Diagnose to run both
// readers on it.  Nothing is uploaded as an artifact: traces hold paths, the
// repository is public, and KILN04 says no artifact is uploaded.  The reasons
// and line numbers are always logged; the window of trace lines only when
// KILN_TRACE_DIFF=1, the same policy as the cross-check's own window.

const (
	traceRefusalMarker = "the trace does not parse"
	traceMarkerOpen    = "(trace: "
	// maxDiagnoseRead bounds how much of the trace the test reads back.
	maxDiagnoseRead = 8 << 20
)

// traceRefusalPath finds the trace path in a refusal message: the text between
// the LAST "(trace: " and the ")" that closes it (the last, because strace's own
// output, quoted earlier in the same message, can hold the same words).  It
// returns "" unless the message says the trace does not parse, the path is
// absolute and clean, and it lies under root (the test's temp dir).
func traceRefusalPath(stderr, root string) string {
	if !strings.Contains(stderr, traceRefusalMarker) {
		return ""
	}
	k := strings.LastIndex(stderr, traceMarkerOpen)
	if k < 0 {
		return ""
	}
	rest := stderr[k+len(traceMarkerOpen):]
	end := strings.IndexByte(rest, ')')
	if end < 0 {
		return ""
	}
	p := rest[:end]
	if !filepath.IsAbs(p) || filepath.Clean(p) != p || root == "" {
		return ""
	}
	rel, err := filepath.Rel(filepath.Clean(root), p)
	if err != nil || rel == "." || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return ""
	}
	return p
}

// logTraceRefusal is called with a failed child's stderr.  When the child
// refused a trace, it logs trace.Diagnose's report of that trace; otherwise it
// does nothing.  It never fails the test itself: the caller reports the failure.
func logTraceRefusal(t testing.TB, stderr string) {
	t.Helper()
	root := os.TempDir()
	if r, err := filepath.EvalSymlinks(root); err == nil {
		root = r
	}
	p := traceRefusalPath(stderr, root)
	if p == "" {
		// The message may name the unresolved temp dir (a symlinked /tmp).
		p = traceRefusalPath(stderr, os.TempDir())
	}
	if p == "" {
		return
	}
	logTraceFile(t, p)
}

// logTraceFile reads the trace at p back and logs trace.Diagnose's report of it.
// It opens the file ONCE, with O_NOFOLLOW and O_NONBLOCK where the platform has
// them (openTraceNoFollow), and checks the handle with Stat: a trace is a regular
// file, never a link or a FIFO.  At most maxDiagnoseRead bytes are read.  Only
// the file's base name is printed (quoted: a name is text a pull request chose),
// and no error text, since an *os.PathError carries the whole path.
func logTraceFile(t testing.TB, p string) {
	t.Helper()
	name := filepath.Base(p)
	f, err := openTraceNoFollow(p)
	if err != nil {
		t.Logf("trace diagnostic: cannot open the trace %q", name)
		return
	}
	defer f.Close()
	if st, err := f.Stat(); err != nil || !st.Mode().IsRegular() {
		t.Logf("trace diagnostic: %q is not a regular file", name)
		return
	}
	data, err := io.ReadAll(io.LimitReader(f, maxDiagnoseRead+1))
	if err != nil {
		t.Logf("trace diagnostic: cannot read the trace %q", name)
		return
	}
	note := ""
	if len(data) > maxDiagnoseRead {
		data = data[:maxDiagnoseRead]
		note = " (read truncated at 8 MiB)"
	}
	// The traces of `prepare`'s probes and of `step` are both helper traces.
	t.Logf("trace diagnostic for %q%s (KILN_TRACE_DIFF=%q):\n%s", name, note, os.Getenv("KILN_TRACE_DIFF"), trace.Diagnose(data, true))
}

// logStepTraces diagnoses every trace file of one action's steps
// (record.StepFile).  Step and classify-child refusals do not reach stderr: they
// land in the sidecar's parse_reasons (or surface as an unexpected unknown_writes
// or a void in `collect`), so a sandbox test that sees one calls this before it
// fails.  Nothing is uploaded; it only logs.
func logStepTraces(t testing.TB, recDir, action string) {
	t.Helper()
	for i := 0; i < 64; i++ { // a recipe has few steps; the bound keeps a bad dir cheap
		p := record.StepFile(recDir, action, i, "trace")
		if _, err := os.Lstat(p); err != nil {
			if i == 0 {
				t.Logf("trace diagnostic: action %q has no step trace", action)
			}
			return
		}
		logTraceFile(t, p)
	}
}

func TestTraceRefusalPath(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("the fixtures are POSIX absolute paths (/tmp/...), which filepath.IsAbs refuses on Windows; strace traces are Linux-only")
	}
	root := filepath.Join(string(filepath.Separator), "tmp", "t1")
	in := func(rel string) string { return filepath.Join(root, rel) }
	msg := func(p string) string {
		return "kiln: structure probe: probe walk: the trace does not parse: trace-incomplete:unparsable-line (line 7) (trace: " + p + ")"
	}
	cases := []struct {
		name, stderr, want string
	}{
		{"plain", msg(in("record/probe/walk.trace")), in("record/probe/walk.trace")},
		{"strace said it first", `x (strace said: "(trace: /etc/passwd)"; trace: ` + in("w.trace") + ")", ""},
		{"last marker wins", "(trace: /etc/passwd) " + msg(in("w.trace")), in("w.trace")},
		{"not a refusal", "probe walk: a process outlived strace (trace: " + in("w.trace") + ")", ""},
		{"relative", msg("record/w.trace"), ""},
		{"outside the temp dir", msg("/etc/passwd"), ""},
		{"dot dot", msg(in("../../etc/passwd")), ""},
		{"unclean", msg(root + "/a/../b"), ""},
		{"the root itself", msg(root), ""},
		{"no close", "the trace does not parse (trace: " + in("w.trace"), ""},
		{"no marker", "the trace does not parse", ""},
	}
	for _, c := range cases {
		if got := traceRefusalPath(c.stderr, root); got != c.want {
			t.Errorf("%s: got %q, want %q", c.name, got, c.want)
		}
	}
	if traceRefusalPath(msg(in("w.trace")), "") != "" {
		t.Error("an empty root must accept nothing")
	}
}

func TestLogTraceRefusalReadsAndDiagnoses(t *testing.T) {
	dir := t.TempDir()
	p := filepath.Join(dir, "walk.trace")
	if err := os.WriteFile(p, []byte("1  execve(\"/bin/true\", [\"true\"], []) = 0\n1  wait4(-1, <unfinished ...>) = ?\n"), 0o600); err != nil {
		t.Fatal(err)
	}
	// logTraceRefusal logs through t; a recording TB shows what it wrote.
	rec := &recordingTB{TB: t}
	logTraceRefusal(rec, "probe walk: the trace does not parse: trace-incomplete:unparsable-line (line 2) (trace: "+p+")")
	if got := strings.Join(rec.logs, "\n"); !strings.Contains(got, "old scanner: refused") || !strings.Contains(got, "grammar reader: refused") {
		t.Errorf("no diagnosis logged (is the temp dir under os.TempDir?): %q", got)
	}
	rec = &recordingTB{TB: t}
	logTraceRefusal(rec, "something else failed")
	if len(rec.logs) != 0 {
		t.Errorf("logged for an unrelated failure: %q", rec.logs)
	}
}

// recordingTB keeps what a helper logs.
type recordingTB struct {
	testing.TB
	logs []string
}

func (r *recordingTB) Logf(format string, args ...any) {
	r.logs = append(r.logs, fmt.Sprintf(format, args...))
}
