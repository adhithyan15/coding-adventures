//go:build unix

package record

// The classification child (classify_child.go), exercised for real: the test
// binary itself is re-executed as `kiln record classify-action` (the standard
// helper-process pattern), so what the supervisor sees is a genuine process
// that panics, hangs, is killed, or runs out of memory, and what the record says
// about it.

import (
	"fmt"
	"os"
	"os/exec"
	"reflect"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// TestClassifyChildHelper is not a test: it is the child process. It does
// nothing unless KILN_CLASSIFY_HELPER=1, and then runs ClassifyChild with the
// fault named by KILN_HELPER_FAULT.
func TestClassifyChildHelper(t *testing.T) {
	if os.Getenv("KILN_CLASSIFY_HELPER") != "1" {
		t.Skip("the child process of the classification tests")
	}
	args := os.Args
	for i, a := range args {
		if a == "--" {
			args = args[i+1:]
			break
		}
	}
	// record classify-action --dir D --action A --step N --data-limit B
	step, _ := strconv.Atoi(args[7])
	limit, _ := strconv.ParseInt(args[9], 10, 64)
	var h ChildHooks
	switch os.Getenv("KILN_HELPER_FAULT") {
	case "panic":
		h.Classify = func(traced.Config, traced.Input) (*traced.RawRecord, error) { panic("boom") }
	case "error":
		h.Classify = func(traced.Config, traced.Input) (*traced.RawRecord, error) { return nil, fmt.Errorf("bad config") }
	case "hang":
		h.Classify = func(traced.Config, traced.Input) (*traced.RawRecord, error) {
			time.Sleep(time.Hour)
			return nil, nil
		}
	case "exit3":
		h.Classify = func(traced.Config, traced.Input) (*traced.RawRecord, error) { os.Exit(3); return nil, nil }
	case "kill":
		h.Classify = func(traced.Config, traced.Input) (*traced.RawRecord, error) {
			syscall.Kill(os.Getpid(), syscall.SIGKILL)
			time.Sleep(time.Hour)
			return nil, nil
		}
	case "alloc":
		h.Classify = func(traced.Config, traced.Input) (*traced.RawRecord, error) {
			b := make([]byte, 3<<30)
			for i := 0; i < len(b); i += 4096 {
				b[i] = 1
			}
			return nil, fmt.Errorf("allocated %d", len(b))
		}
	}
	if err := ClassifyChild(args[3], args[5], step, limit, h); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	os.Exit(0)
}

// testChildCmd makes the supervisor's ChildCmd run the helper process with a
// fault (and extra NAME=value entries).
func testChildCmd(fault string, extra ...string) func(args, env []string) *exec.Cmd {
	return func(args, env []string) *exec.Cmd {
		cmd := exec.Command(os.Args[0], append([]string{"-test.run=^TestClassifyChildHelper$", "--"}, args...)...)
		cmd.Env = append(append(append([]string{}, env...), "KILN_CLASSIFY_HELPER=1", "KILN_HELPER_FAULT="+fault), extra...)
		return cmd
	}
}

// --- limits -----------------------------------------------------------------

func TestClassifyLimitsAreTheChosenOnesNotTheParsersDefaults(t *testing.T) {
	l := ClassifyLimits()
	def := trace.DefaultLimits()
	if l.MaxTotalBytes != DefaultTraceCap || l.MaxTotalBytes <= def.MaxTotalBytes || l.MaxEvents <= def.MaxEvents || l.MaxEvents != ClassifyMaxEvents {
		t.Fatalf("%+v vs the parser's %+v", l, def)
	}
	d := ClassifyDeps{Limits: trace.Limits{MaxTotalBytes: 7}}.withDefaults()
	if d.Limits.MaxTotalBytes != 7 || d.Limits.MaxEvents != ClassifyMaxEvents || d.Limits.MaxLineBytes != l.MaxLineBytes {
		t.Fatalf("%+v", d.Limits)
	}
}

// The regression of the review (M-1): a step's trace bigger than the parser's
// own default (64 MiB, and over a million records) is one the supervisor accepts
// (its cap is 1 GiB), and the epilogue must classify it, not call it unparseable.
func TestEpilogueClassifiesATraceBeyondTheParsersDefaultLimits(t *testing.T) {
	if testing.Short() {
		t.Skip("writes and parses a 70 MiB trace")
	}
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	var body []string
	one := `stat("absent-file", 0x1) = -1 ENOENT (No such file or directory)`
	n := (70 << 20) / (len(one) + 4)
	body = make([]string, n)
	for i := range body {
		body[i] = one
	}
	f.give("z", actZ, line, 0, cannedOpts{}, body...)
	st, err := os.Stat(f.aux + "/z.trace")
	if err != nil || st.Size() <= trace.DefaultLimits().MaxTotalBytes || n <= trace.DefaultLimits().MaxEvents {
		t.Fatalf("the fixture is not over the parser's defaults: %v %d events", err, n)
	}
	res := f.must(actZ, 0, 1, line)
	if res.ExitCode != 0 || res.Record != RecordWritten {
		t.Fatalf("%d %q %q", res.ExitCode, res.Record, res.RecordWhy)
	}
	rec := f.readRecord(actZ)
	if !rec.ParseOK || rec.Record == nil {
		t.Fatalf("a trace the supervisor accepted was not classified: %q", rec.ParseReasons)
	}
}

// And the other side: the limit is real. A trace one byte over the byte cap the
// classifier is given is refused by the parser (parse_ok false, a reason naming
// it), one at the cap is not.
func TestEpilogueTraceJustOverTheLimitIsAParseFailure(t *testing.T) {
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	f.must(actZ, 0, 1, line)
	size := f.sidecar(actZ, 0).TraceBytes
	at := ClassifyDeps{Limits: trace.Limits{MaxTotalBytes: size}}
	rec, err := classifyAction(at, f.pre, actZ, 0, f.sidecar(actZ, 0))
	if err != nil || !rec.ParseOK {
		t.Fatalf("at the limit: %v %q", err, rec.ParseReasons)
	}
	over := ClassifyDeps{Limits: trace.Limits{MaxTotalBytes: size - 1}}
	rec, err = classifyAction(over, f.pre, actZ, 0, f.sidecar(actZ, 0))
	if err != nil || rec.ParseOK || rec.Record != nil || !strings.Contains(strings.Join(rec.ParseReasons, "|"), "trace-incomplete") {
		t.Fatalf("one byte over the limit: %v %+v", err, rec)
	}
}

// --- the child --------------------------------------------------------------

func TestChildFailuresAreRecordFailedAndNeverChangeTheStatus(t *testing.T) {
	line := playLine("", "z", 7)
	cases := []struct {
		name, fault, want string
		timeout           time.Duration
	}{
		{"a panic", "panic", "panic: boom", 0},
		{"an error", "error", "bad config", 0},
		{"a crash", "exit3", "exit status 3", 0},
		{"a kill", "kill", "signal: killed", 0},
		{"a hang past the deadline", "hang", "timeout", 500 * time.Millisecond},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
			f.give("z", actZ, line, 7, cannedOpts{})
			o := f.opts(actZ, 0, 1, line)
			o.ChildCmd = testChildCmd(c.fault)
			if c.timeout > 0 {
				o.ClassifyTimeout = c.timeout
			}
			start := time.Now()
			res, err := RunStep(o)
			if err != nil {
				t.Fatal(err)
			}
			if res.ExitCode != 7 || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, c.want) {
				t.Fatalf("exit %d record %q why %q (want %q)", res.ExitCode, res.Record, res.RecordWhy, c.want)
			}
			if time.Since(start) > 60*time.Second {
				t.Fatal("the supervisor waited for the child")
			}
			if exists(RecordFile(f.p.dir, actZ)) {
				t.Fatal("a record.json beside the record.failed")
			}
			why, err := os.ReadFile(RecordFailedFile(f.p.dir, actZ))
			if err != nil || !strings.Contains(string(why), c.want) {
				t.Fatalf("%q %v", why, err)
			}
			if sc := f.sidecar(actZ, 0); sc.RootStatus != 7 || !sc.Clean {
				t.Fatalf("%+v", sc)
			}
		})
	}
}

// A child that does not start at all (a binary that is not a program) is a
// record.failed too.
func TestChildThatCannotBeStartedIsRecordFailed(t *testing.T) {
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	o := f.opts(actZ, 0, 1, line)
	o.ChildCmd = func(args, env []string) *exec.Cmd { return exec.Command(f.aux + "/no-such-program") }
	res, err := RunStep(o)
	if err != nil || res.ExitCode != 0 || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, "could not be started") {
		t.Fatalf("%v %d %q %q", err, res.ExitCode, res.Record, res.RecordWhy)
	}
}

// A child that exits 0 and leaves nothing, or something that is not this
// action's record, is not believed.
func TestParentDoesNotTrustACleanExitWithoutARecord(t *testing.T) {
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	o := f.opts(actZ, 0, 1, line)
	o.ChildCmd = func(args, env []string) *exec.Cmd { return exec.Command("/usr/bin/true") }
	res, err := RunStep(o)
	if err != nil || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, "left no record.json") {
		t.Fatalf("%v %q %q", err, res.Record, res.RecordWhy)
	}
	// A record.json that does not decode.
	f2 := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f2.give("z", actZ, line, 0, cannedOpts{})
	o = f2.opts(actZ, 0, 1, line)
	o.ChildCmd = func(args, env []string) *exec.Cmd {
		return exec.Command("/bin/sh", "-c", `printf '{' > "$1"`, "sh", RecordFile(f2.p.dir, actZ))
	}
	if res, err = RunStep(o); err != nil || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, "does not decode") {
		t.Fatalf("%v %q %q", err, res.Record, res.RecordWhy)
	}
	// A valid record of the right shape under another sealed directory.
	f3 := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f3.give("z", actZ, line, 0, cannedOpts{})
	other := ClassifiedRecord{Schema: ClassifiedSchema, Action: actZ, BindingID: strings.Repeat("a", 64), SealedSHA256: strings.Repeat("b", 64), ParseReasons: []string{"x"}}
	data, _ := EncodeClassified(other)
	writeBytes(t, f3.aux+"/other.json", data, 0o644)
	o = f3.opts(actZ, 0, 1, line)
	o.ChildCmd = func(args, env []string) *exec.Cmd {
		return exec.Command("/bin/sh", "-c", `cp "$1" "$2"`, "sh", f3.aux+"/other.json", RecordFile(f3.p.dir, actZ))
	}
	if res, err = RunStep(o); err != nil || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, "another action or another sealed directory") {
		t.Fatalf("%v %q %q", err, res.Record, res.RecordWhy)
	}
}

// The child is given paths and numbers and re-checks them: a step number out of
// range, an action the plan lacks, a missing sidecar are all refusals.
func TestClassifyChildRefusesWhatItIsNotGiven(t *testing.T) {
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, ChildHooks{}); err == nil || !strings.Contains(err.Error(), "sidecar") {
		t.Fatalf("no sidecar yet: %v", err)
	}
	f.must(actZ, 0, 1, line)
	for name, c := range map[string]struct {
		dir, action string
		step        int
		want        string
	}{
		"an unknown action": {f.p.dir, "code/packages/x/ghost", 0, "not an action"},
		"a step too high":   {f.p.dir, actZ, 1, "out of range"},
		"a negative step":   {f.p.dir, actZ, -1, "out of range"},
		"no pre":            {f.aux, actZ, 0, "pre/"},
		"a relative dir":    {"record", actZ, 0, "absolute"},
	} {
		if err := ClassifyChild(c.dir, c.action, c.step, 0, ChildHooks{}); err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: %v", name, err)
		}
	}
	// The real record was written by the step already: a second one is refused.
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, ChildHooks{}); err == nil || !strings.Contains(err.Error(), "classified-twice") {
		t.Fatalf("%v", err)
	}
}

// writeAtomic is whole or absent, and never replaces.
func TestWriteAtomic(t *testing.T) {
	d := canonTemp(t)
	p := d + "/record.json"
	if err := writeAtomic(p, []byte("one")); err != nil {
		t.Fatal(err)
	}
	if err := writeAtomic(p, []byte("two")); err == nil || !strings.Contains(err.Error(), "exists") {
		t.Fatalf("replaced: %v", err)
	}
	if got, _ := os.ReadFile(p); string(got) != "one" {
		t.Fatalf("%q", got)
	}
	entries, _ := os.ReadDir(d)
	if len(entries) != 1 {
		t.Fatalf("a temporary file is left: %v", entries)
	}
	if err := writeAtomic(d+"/no/such/dir/x", []byte("x")); err == nil {
		t.Fatal("wrote into a missing directory")
	}
}

// The child's work, run in this process, gives the same record the child
// process wrote (the statements the child process runs are counted here), over
// the whole G7 table, and its parse and classify seams are the real ones.
func TestClassifyChildInProcessAgreesWithTheChildProcess(t *testing.T) {
	cases := []struct {
		status int
		opts   cannedOpts
	}{
		{0, cannedOpts{}},
		{3, cannedOpts{}},
		{125, cannedOpts{}},
		{3, cannedOpts{rootExit: 0, hasRootExit: true}},
		{0, cannedOpts{rootSHA: strings.Repeat("a", 64)}},
		{0, cannedOpts{initSHA: strings.Repeat("b", 64)}},
		{0, cannedOpts{noRootArgv: true}},
		{0, cannedOpts{noInitExec: true}},
		{125, cannedOpts{noStepExec: true}},
		{125, cannedOpts{noInit: true}},
		{3, cannedOpts{noStepExec: true}},
	}
	for i, c := range cases {
		line := playLine("", "z", c.status)
		f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
		f.give("z", actZ, line, c.status, c.opts)
		f.must(actZ, 0, 1, line)
		want, _ := os.ReadFile(RecordFile(f.p.dir, actZ))
		if err := os.Remove(RecordFile(f.p.dir, actZ)); err != nil {
			t.Fatal(err)
		}
		if err := ClassifyChild(f.p.dir, actZ, 0, 0, ChildHooks{}); err != nil {
			t.Fatalf("case %d: %v", i, err)
		}
		got, _ := os.ReadFile(RecordFile(f.p.dir, actZ))
		if string(got) != string(want) {
			t.Fatalf("case %d: in-process\n%s\nchild process\n%s", i, got, want)
		}
	}
}

// ClassifyChild's seams: a classifier that errors or panics is an error of the
// child (and the in-process panic is recovered), and a disk view can be
// replaced.
func TestClassifyChildHooks(t *testing.T) {
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	f.must(actZ, 0, 1, line)
	os.Remove(RecordFile(f.p.dir, actZ))
	boom := ChildHooks{Classify: func(traced.Config, traced.Input) (*traced.RawRecord, error) { panic("boom") }}
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, boom); err == nil || !strings.Contains(err.Error(), "panic: boom") {
		t.Fatalf("%v", err)
	}
	bad := ChildHooks{Classify: func(traced.Config, traced.Input) (*traced.RawRecord, error) { return nil, fmt.Errorf("bad") }}
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, bad); err == nil || !strings.Contains(err.Error(), "classify: bad") {
		t.Fatalf("%v", err)
	}
	if exists(RecordFile(f.p.dir, actZ)) {
		t.Fatal("a failed child wrote a record")
	}
	var asked bool
	spy := ChildHooks{DiskFS: spyFS{LiveFS{}, &asked}}
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, spy); err != nil || !asked {
		t.Fatalf("%v asked=%v", err, asked)
	}
}

type spyFS struct {
	LiveFS
	asked *bool
}

func (s spyFS) Lstat(p string) (traced.FileInfo, error) {
	*s.asked = true
	return s.LiveFS.Lstat(p)
}

// --- round 2 of the review ---------------------------------------------------

func TestSupervisorBoundsWhatItReadsBackFromTheChild(t *testing.T) {
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	o := f.opts(actZ, 0, 1, line)
	o.MaxRecordBytes = 10
	res, err := RunStep(o)
	if err != nil || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, "record-too-large") {
		t.Fatalf("%v %q %q", err, res.Record, res.RecordWhy)
	}
	// A single verdict: the oversize record.json was removed.
	if exists(RecordFile(f.p.dir, actZ)) || !exists(RecordFailedFile(f.p.dir, actZ)) {
		t.Fatal("record.json stands beside record.failed")
	}
	// A record.json that is not a regular file (a link to somewhere) is refused.
	g := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	g.give("z", actZ, line, 0, cannedOpts{})
	o = g.opts(actZ, 0, 1, line)
	o.ChildCmd = func(args, env []string) *exec.Cmd {
		return exec.Command("/bin/sh", "-c", `ln -s /etc/hostname "$1"`, "sh", RecordFile(g.p.dir, actZ))
	}
	if res, err = RunStep(o); err != nil || res.Record != RecordFailed || !strings.Contains(res.RecordWhy, "left no record.json") {
		t.Fatalf("%v %q %q", err, res.Record, res.RecordWhy)
	}
	if exists(RecordFile(g.p.dir, actZ)) {
		t.Fatal("the link was left")
	}
}

// A child that links a valid record and is then killed leaves one verdict.
func TestKilledChildAfterLinkingARecordLeavesOnlyRecordFailed(t *testing.T) {
	line := playLine("", "z", 4)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 4, cannedOpts{})
	o := f.opts(actZ, 0, 1, line)
	o.ChildCmd = func(args, env []string) *exec.Cmd {
		inner := testChildCmd("")(args, env)
		cmd := exec.Command("/bin/sh", append([]string{"-c", `"$@"; kill -9 $$`, "sh"}, inner.Args...)...)
		cmd.Env = inner.Env
		return cmd
	}
	res, err := RunStep(o)
	if err != nil || res.ExitCode != 4 || res.Record != RecordFailed {
		t.Fatalf("%v %d %q %q", err, res.ExitCode, res.Record, res.RecordWhy)
	}
	if exists(RecordFile(f.p.dir, actZ)) || !exists(RecordFailedFile(f.p.dir, actZ)) {
		t.Fatal("not a single verdict")
	}
}

// A raw record whose lists reached the classifier's cap is dropped by the child.
func TestChildDropsARecordWhoseListsHitTheCap(t *testing.T) {
	line := playLine("", "z", 0)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 0, cannedOpts{})
	f.must(actZ, 0, 1, line)
	os.Remove(RecordFile(f.p.dir, actZ))
	h := ChildHooks{Classify: func(cfg traced.Config, in traced.Input) (*traced.RawRecord, error) {
		return &traced.RawRecord{Package: actZ, Writes: make([]string, ClassifyMaxListed+1)}, nil
	}}
	if err := ClassifyChild(f.p.dir, actZ, 0, 0, h); err != nil {
		t.Fatal(err)
	}
	rec := f.readRecord(actZ)
	if rec.ParseOK || rec.Record != nil || rec.WritesComplete || !reflect.DeepEqual(rec.ParseReasons, []string{"writes-too-many"}) {
		t.Fatalf("%+v", rec)
	}
}

func TestChildMemoryPlanAndImage(t *testing.T) {
	const gib = int64(1) << 30
	if d, s := scaleLimits(8*gib, 0); d != 8*gib || s != 0 {
		t.Fatalf("%d %d", d, s)
	}
	if d, s := scaleLimits(8*gib, 64*gib); d != 8*gib || s != 5*gib {
		t.Fatalf("%d %d", d, s)
	}
	if d, s := scaleLimits(8*gib, 4*gib); d != 2*gib || s != 2*gib/8*5 {
		t.Fatalf("%d %d", d, s)
	}
	if got := parseMemTotal("MemFree: 1 kB\nMemTotal:       16384 kB\n"); got != 16384<<10 {
		t.Fatalf("%d", got)
	}
	for _, bad := range []string{"", "MemTotal: x kB", "MemTotal:", "MemTotal: 0 kB", "MemTotal: -5 kB"} {
		if parseMemTotal(bad) != 0 {
			t.Errorf("%q", bad)
		}
	}
	if childExe("/opt/kiln", "linux") != "/proc/self/exe" || childExe("/opt/kiln", "darwin") != "/opt/kiln" {
		t.Fatal("the child image")
	}
}

// The child refuses an action that has not ended and one that has a verdict.
func TestClassifyChildGuards(t *testing.T) {
	l0, l1 := playLine("", "b0", 0), playLine("", "b1", 0)
	f := newStepFixCustom(t, planOf(nil, []string{l0, l1}, nil), nil, true)
	f.give("b0", actB, l0, 0, cannedOpts{})
	f.must(actB, 0, 2, l0)
	if err := ClassifyChild(f.p.dir, actB, 0, 0, ChildHooks{}); err == nil || !strings.Contains(err.Error(), "not the end of the action") {
		t.Fatalf("a mid-action step was classified: %v", err)
	}
	// A failing step ends the action.
	bad := playLine("", "z", 3)
	g := newStepFixCustom(t, planOf(nil, []string{bad, l1}, nil), nil, true)
	g.give("z", actB, bad, 3, cannedOpts{})
	g.must(actB, 0, 2, bad)
	os.Remove(RecordFile(g.p.dir, actB))
	if err := ClassifyChild(g.p.dir, actB, 0, 0, ChildHooks{}); err != nil {
		t.Fatalf("a failing step was refused: %v", err)
	}
	// A verdict already there, record.failed included.
	os.Remove(RecordFile(g.p.dir, actB))
	writeBytes(t, RecordFailedFile(g.p.dir, actB), []byte("x"), 0o600)
	if err := ClassifyChild(g.p.dir, actB, 0, 0, ChildHooks{}); err == nil || !strings.Contains(err.Error(), "classified-twice") {
		t.Fatalf("%v", err)
	}
	// A step that did not start ends the action too.
	h := newStepFixCustom(t, planOf(nil, []string{l0, l1}, nil), nil, true)
	os.Chmod(h.p.strace, 0o644)
	h.must(actB, 0, 2, l0)
	os.Remove(RecordFile(h.p.dir, actB))
	if err := ClassifyChild(h.p.dir, actB, 0, 0, ChildHooks{}); err != nil {
		t.Fatalf("a step that did not start was refused: %v", err)
	}
}
