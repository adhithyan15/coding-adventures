//go:build unix

package record

// Tests of `kiln record step` (KILN04 section 4.8, decision P12).
//
// They run on any Unix, with no strace and no sandbox. The "strace" the supervisor
// starts is a shell script named by tools.json (the supervisor never looks
// anything up on PATH, so that is the one place a test can put it), and the
// "kiln" is a plain file whose hash is in the binding. The script finds the
// trace file after `-o`, takes the LAST argument (the step's `sh -c` line) and
// runs it with `eval`, so a test decides what the "tracer" does by choosing the
// line: `exit 3`, `echo boom >&2`, `kill -9 $$`, `head -c 5000 /dev/zero >>
// "$tf"; exec sleep 30`. Descriptors 3 and 4 are the step's stdout and stderr
// pipes, exactly as the real sandbox-exec receives them.
//
// The real thing, with the real strace and the real sandbox, is
// TestSandboxRecordStep in the parent package (the required kiln-sandbox job).

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// planSteps is the plan the fixture gives its three packages: step lines that
// are harmless for the fake tracer to eval (descriptor 3 and 4 are the step's
// output).
const planSteps = `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[` +
	`{"package":"code/packages/x/a","steps":["echo hi >&3; echo oops >&4"]},` +
	`{"package":"code/packages/x/b","steps":["exit 0","exit 0"]},` +
	`{"package":"code/packages/x/z","steps":["exit 0"]}]}`

const (
	actA = "code/packages/x/a"
	actB = "code/packages/x/b"
	actZ = "code/packages/x/z"
)

// stepFix is a prepared shard and a fake tracer.
type stepFix struct {
	t          *testing.T
	p          *prep
	pre        *Pre
	aux        string // outside the record directory and the checkout
	args, envf string
	count      string
	stdout     bytes.Buffer
	stderr     bytes.Buffer
}

func newStepFix(t *testing.T) *stepFix {
	t.Helper()
	p := newPrep(t)
	f := &stepFix{t: t, p: p, aux: filepath.Join(p.base, "aux")}
	if err := os.MkdirAll(f.aux, 0o755); err != nil {
		t.Fatal(err)
	}
	f.args, f.envf, f.count = filepath.Join(f.aux, "args"), filepath.Join(f.aux, "env"), filepath.Join(f.aux, "count")
	// The fake strace: find the trace file, keep the arguments and the
	// environment where the test can read them, count the runs, then run the
	// step's line as shell code.
	script := `#!/bin/sh
tf=""
while [ $# -gt 0 ]; do
  case "$1" in -o) tf="$2"; shift 2;; --) shift; break;; *) shift;; esac
done
echo run >> '` + f.count + `'
printf '%s\n' "$@" > '` + f.args + `'
env | sort > '` + f.envf + `'
for last; do :; done
eval "$last"
`
	writeBytes(t, p.strace, []byte(script), 0o755)
	p.steps = planSteps
	p.commit()
	p.mustPrepare()
	pre, err := VerifyPre(p.dir)
	if err != nil {
		t.Fatal(err)
	}
	f.pre = pre
	return f
}

// env is a legitimate environment for a step: the build tool's own plus the
// three KILN variables.
func (f *stepFix) env(action string, step, count int) []string {
	return append(append([]string{}, f.pre.Tools.BuildToolEnv...),
		EnvAction+"="+action, fmt.Sprintf("%s=%d", EnvStep, step), fmt.Sprintf("%s=%d", EnvStepCount, count))
}

// opts are the options of a legitimate run of action's step.
func (f *stepFix) opts(action string, step, count int, line string) StepOptions {
	return StepOptions{
		Dir: f.p.dir, Argv: []string{"sh", "-c", line}, Environ: f.env(action, step, count),
		Cwd: f.pre.Config.PackageDir(action), KilnExe: f.p.kiln,
		Stdout: &f.stdout, Stderr: &f.stderr, Poll: 5 * time.Millisecond, Drain: 2 * time.Second,
	}
}

func (f *stepFix) run(action string, step, count int, line string) (*StepResult, error) {
	f.t.Helper()
	return RunStep(f.opts(action, step, count, line))
}

func (f *stepFix) must(action string, step, count int, line string) *StepResult {
	f.t.Helper()
	res, err := f.run(action, step, count, line)
	if err != nil {
		f.t.Fatalf("RunStep: %v", err)
	}
	return res
}

func (f *stepFix) runs() int {
	data, err := os.ReadFile(f.count)
	if err != nil {
		return 0
	}
	return strings.Count(string(data), "run\n")
}

func (f *stepFix) refused(o StepOptions, want string) {
	f.t.Helper()
	before := f.runs()
	res, err := RunStep(o)
	if err == nil {
		f.t.Fatalf("a step that must be refused ran: %+v", res)
	}
	if !strings.Contains(err.Error(), want) {
		f.t.Fatalf("error %q does not mention %q", err, want)
	}
	if f.runs() != before {
		f.t.Fatalf("the tracer was started for a refused step")
	}
}

func (f *stepFix) sidecar(action string, step int) StepSidecar {
	f.t.Helper()
	data, err := os.ReadFile(StepFile(f.p.dir, action, step, "json"))
	if err != nil {
		f.t.Fatal(err)
	}
	sc, err := DecodeStepSidecar(data)
	if err != nil {
		f.t.Fatalf("%v\n%s", err, data)
	}
	return sc
}

// ---------------------------------------------------------------------------

func TestStepRunsTheStepAndWritesTheSidecar(t *testing.T) {
	f := newStepFix(t)
	line := planSteps[strings.Index(planSteps, "echo hi"):strings.Index(planSteps, `"]},{"package":"code/packages/x/b"`)]
	if line != "echo hi >&3; echo oops >&4" {
		t.Fatalf("test setup: %q", line)
	}
	before := time.Now().UTC().Add(-time.Second)
	res := f.must(actA, 0, 1, line)
	if res.ExitCode != 0 {
		t.Fatalf("exit %d", res.ExitCode)
	}
	// The step's two streams reach the supervisor's, separately.
	if f.stdout.String() != "hi\n" || f.stderr.String() != "oops\n" {
		t.Fatalf("relayed %q and %q", f.stdout.String(), f.stderr.String())
	}
	sc := f.sidecar(actA, 0)
	if fmt.Sprint(sc) != fmt.Sprint(res.Sidecar) {
		t.Fatalf("the returned sidecar differs from the file's:\n%+v\n%+v", sc, res.Sidecar)
	}
	trace, err := os.ReadFile(StepFile(f.p.dir, actA, 0, "trace"))
	if err != nil {
		t.Fatal(err)
	}
	sum := sha256.Sum256(trace)
	line256 := sha256.Sum256([]byte(line))
	want := StepSidecar{
		Schema: StepSchema, Action: actA, Step: 0, StepCount: 1, BindingID: f.pre.BindingID,
		LineSHA256: hex.EncodeToString(line256[:]), LineInPlan: true, Started: true,
		StraceExit: 0, StraceStdoutEmpty: true, StraceStderrEmpty: true, RootStatus: 0,
		TraceBytes: int64(len(trace)), TraceSHA256: hex.EncodeToString(sum[:]),
		StepNetNS: true, Clean: true, Unclean: []string{},
	}
	got := sc
	got.StartedAt, got.EndedAt = "", ""
	if fmt.Sprintf("%+v", got) != fmt.Sprintf("%+v", want) {
		t.Fatalf("sidecar\n got %+v\nwant %+v", got, want)
	}
	st, _ := time.Parse(time.RFC3339Nano, sc.StartedAt)
	en, _ := time.Parse(time.RFC3339Nano, sc.EndedAt)
	if st.Before(before) || en.Before(st) || en.After(time.Now().Add(time.Second)) {
		t.Fatalf("times %s %s", sc.StartedAt, sc.EndedAt)
	}
	// The sidecar is exactly the encoder's bytes: compact, no newline.
	raw, _ := os.ReadFile(StepFile(f.p.dir, actA, 0, "json"))
	if bytes.HasSuffix(raw, []byte("\n")) || bytes.Contains(raw, []byte(": ")) {
		t.Fatalf("not compact: %q", raw)
	}
	// The directory layout: an action's name never became a path component.
	if !strings.HasPrefix(StepDir(f.p.dir, actA), filepath.Join(f.p.dir, "traces")+"/") || strings.Contains(StepDir(f.p.dir, actA), "code/packages") {
		t.Fatalf("StepDir %s", StepDir(f.p.dir, actA))
	}
	for _, d := range []string{filepath.Join(f.p.dir, "traces"), StepDir(f.p.dir, actA)} {
		if st, err := os.Stat(d); err != nil || st.Mode().Perm() != 0o700 {
			t.Fatalf("%s: %v %v", d, st, err)
		}
	}
}

// The strace argument list is exactly what the sandbox package exports, then the
// sandbox helper, the sealed configuration and the two descriptors.
func TestStepStartsStraceWithTheExactArgumentsAndEnvironment(t *testing.T) {
	f := newStepFix(t)
	f.must(actB, 0, 2, "exit 0")
	data, err := os.ReadFile(f.args)
	if err != nil {
		t.Fatal(err)
	}
	got := strings.Split(strings.TrimSuffix(string(data), "\n"), "\n")
	want := []string{f.p.kiln, "sandbox-exec", "--config", filepath.Join(f.p.dir, "pre", "sandbox.json"),
		"--stdout-fd", "3", "--stderr-fd", "4", "--", "sh", "-c", "exit 0"}
	// The fake strace shifted off everything up to and including "--".
	if strings.Join(got, "\x00") != strings.Join(want, "\x00") {
		t.Fatalf("argv after strace's options:\n%q\nwant\n%q", got, want)
	}
	envData, err := os.ReadFile(f.envf)
	if err != nil {
		t.Fatal(err)
	}
	var keep []string
	for _, l := range strings.Split(strings.TrimSpace(string(envData)), "\n") {
		// The shell adds PWD and _ and SHLVL of its own.
		if strings.HasPrefix(l, "PWD=") || strings.HasPrefix(l, "_=") || strings.HasPrefix(l, "SHLVL=") || strings.HasPrefix(l, "OLDPWD=") {
			continue
		}
		keep = append(keep, l)
	}
	wantEnv := []string{"KILN_ACTION=" + actB, "KILN_STEP=0", "KILN_STEP_COUNT=2", "PATH=" + f.pre.Tools.Path}
	if strings.Join(keep, "\n") != strings.Join(sortedCopy(wantEnv), "\n") {
		t.Fatalf("the tracer's environment:\n%q\nwant\n%q", keep, sortedCopy(wantEnv))
	}
}

func sortedCopy(s []string) []string {
	out := append([]string{}, s...)
	sort.Strings(out)
	return out
}

func TestStepStrace_ArgvIsStraceArgs(t *testing.T) {
	// The first arguments strace itself gets are the S3 list, ending in the
	// trace file and "--". A recording tracer sees them all.
	f := newStepFix(t)
	body := "#!/bin/sh\nprintf '%s\\n' \"$@\" > '" + f.args + "'\nexit 0\n"
	writeBytes(t, f.p.strace, []byte(body), 0o755)
	// The fixture's strace is hashed into the binding: put the new body there.
	f.reseal(map[string][]byte{}, true)
	f.must(actZ, 0, 1, "exit 0")
	data, _ := os.ReadFile(f.args)
	lines := strings.Split(strings.TrimSuffix(string(data), "\n"), "\n")
	trace := StepFile(f.p.dir, actZ, 0, "trace")
	i := indexOf(lines, "-o")
	if lines[0] != "-f" || i < 0 || lines[i+1] != trace || lines[i+2] != "--" || lines[i+3] != f.p.kiln {
		t.Fatalf("%q", lines)
	}
}

func indexOf(l []string, s string) int {
	for i, v := range l {
		if v == s {
			return i
		}
	}
	return -1
}

func TestStepMirrorsTheRootsStatus(t *testing.T) {
	f := newStepFix(t)
	res := f.must(actB, 0, 2, "exit 3")
	sc := res.Sidecar
	if res.ExitCode != 3 || sc.StraceExit != 3 || sc.RootStatus != 3 || sc.Exit125 || !sc.Clean || !sc.StepNetNS {
		t.Fatalf("%d %+v", res.ExitCode, sc)
	}
	// Step 1 of the same action runs next, and sees step 0's sidecar.
	if res := f.must(actB, 1, 2, "exit 0"); res.ExitCode != 0 {
		t.Fatalf("%d", res.ExitCode)
	}
}

// Exit 125 is a failure of the step, recorded and mirrored, and never run again
// without the sandbox or at all: the tracer starts exactly once.
func TestStepExit125IsRecordedAndNeverRetried(t *testing.T) {
	f := newStepFix(t)
	res := f.must(actZ, 0, 1, "exit 125")
	if res.ExitCode != 125 || !res.Sidecar.Exit125 || res.Sidecar.RootStatus != 125 || !res.Sidecar.Clean {
		t.Fatalf("%d %+v", res.ExitCode, res.Sidecar)
	}
	if n := f.runs(); n != 1 {
		t.Fatalf("the tracer was started %d times", n)
	}
	data, _ := os.ReadFile(f.args)
	if strings.Count(string(data), "sandbox-exec") != 1 {
		t.Fatalf("not run through the sandbox exactly once: %s", data)
	}
}

func TestStepRecordsAnUncleanTracer(t *testing.T) {
	cases := []struct {
		name, line, reason string
		code, exit         int
	}{
		{"stderr", "echo boom >&2", UncleanStderr, 0, 0},
		{"stdout", "echo not-the-steps", UncleanStdout, 0, 0},
		{"both", "echo a; echo b >&2; exit 4", UncleanStdout, 4, 4},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			f := newStepFix(t)
			res := f.must(actZ, 0, 1, c.line)
			sc := res.Sidecar
			if sc.Clean || !containsStr(sc.Unclean, c.reason) || res.ExitCode != c.code || sc.StraceExit != c.exit {
				t.Fatalf("%d %+v", res.ExitCode, sc)
			}
			if (c.name == "stderr") && (sc.StraceStderrEmpty || !sc.StraceStdoutEmpty) {
				t.Fatalf("%+v", sc)
			}
			if (c.name == "stdout") && (sc.StraceStdoutEmpty || !sc.StraceStderrEmpty) {
				t.Fatalf("%+v", sc)
			}
			// What strace itself wrote is not step output and never relayed.
			if f.stdout.Len() != 0 || f.stderr.Len() != 0 {
				t.Fatalf("strace's own output reached the build log: %q %q", f.stdout.String(), f.stderr.String())
			}
			// The sidecar decodes: clean is false with reasons listed.
			if got := f.sidecar(actZ, 0); got.Clean || len(got.Unclean) == 0 {
				t.Fatalf("%+v", got)
			}
		})
	}
}

func exists(p string) bool {
	_, err := os.Lstat(p)
	return err == nil
}

func containsStr(l []string, s string) bool { return indexOf(l, s) >= 0 }

func TestStepASignalledStraceIsUncleanAndExits128PlusTheSignal(t *testing.T) {
	f := newStepFix(t)
	res := f.must(actZ, 0, 1, "kill -9 $$")
	sc := res.Sidecar
	if res.ExitCode != 128+9 || sc.StraceExit != -1 || sc.StraceSignal != 9 || sc.RootStatus != -1 || sc.Clean || !containsStr(sc.Unclean, UncleanSignalled) || !sc.StepNetNS {
		t.Fatalf("%d %+v", res.ExitCode, sc)
	}
}

func TestStepKillsStraceWhenTheTraceIsTooLarge(t *testing.T) {
	f := newStepFix(t)
	o := f.opts(actZ, 0, 1, `head -c 20000 /dev/zero >> "$tf"; exec sleep 30`)
	o.TraceCap = 1000
	start := time.Now()
	res, err := RunStep(o)
	if err != nil {
		t.Fatal(err)
	}
	if time.Since(start) > 20*time.Second {
		t.Fatal("the supervisor waited for the step instead of killing it")
	}
	sc := res.Sidecar
	if !sc.Truncated || sc.TruncatedReason != TruncTooLarge || sc.Clean || !containsStr(sc.Unclean, UncleanTruncated+":"+TruncTooLarge) ||
		sc.StraceSignal != 9 || res.ExitCode != 137 || sc.TraceBytes < 20000 {
		t.Fatalf("%d %+v", res.ExitCode, sc)
	}
	// The recorded digest is of the file as it ended.
	trace, _ := os.ReadFile(StepFile(f.p.dir, actZ, 0, "trace"))
	if sum := sha256.Sum256(trace); sc.TraceSHA256 != hex.EncodeToString(sum[:]) || sc.TraceBytes != int64(len(trace)) {
		t.Fatalf("%+v", sc)
	}
}

func TestStepDiskFloor(t *testing.T) {
	t.Run("at the start nothing runs", func(t *testing.T) {
		f := newStepFix(t)
		o := f.opts(actZ, 0, 1, "exit 0")
		o.FreeBytes = func(string) (uint64, error) { return 100, nil }
		o.DiskFloor = 1000
		f.refused(o, "under the floor")
		if _, err := os.Stat(StepFile(f.p.dir, actZ, 0, "trace")); err == nil {
			t.Fatal("a trace file was created for a step that did not run")
		}
	})
	t.Run("an unmeasurable disk", func(t *testing.T) {
		f := newStepFix(t)
		o := f.opts(actZ, 0, 1, "exit 0")
		o.FreeBytes = func(string) (uint64, error) { return 0, errors.New("statfs: boom") }
		f.refused(o, "statfs: boom")
	})
	t.Run("while it runs", func(t *testing.T) {
		f := newStepFix(t)
		o := f.opts(actZ, 0, 1, "exec sleep 30")
		var calls atomic.Int32
		o.DiskFloor = 1000
		o.FreeBytes = func(string) (uint64, error) {
			if calls.Add(1) == 1 {
				return 1 << 40, nil
			}
			return 10, nil
		}
		res, err := RunStep(o)
		if err != nil {
			t.Fatal(err)
		}
		sc := res.Sidecar
		if !sc.Truncated || sc.TruncatedReason != TruncDiskFloor || sc.Clean || res.ExitCode != 137 {
			t.Fatalf("%d %+v", res.ExitCode, sc)
		}
	})
	t.Run("a measurement that fails while it runs", func(t *testing.T) {
		f := newStepFix(t)
		o := f.opts(actZ, 0, 1, "exec sleep 30")
		var calls atomic.Int32
		o.FreeBytes = func(string) (uint64, error) {
			if calls.Add(1) == 1 {
				return 1 << 40, nil
			}
			return 0, errors.New("gone")
		}
		res, err := RunStep(o)
		if err != nil || res.Sidecar.TruncatedReason != TruncDiskFloor {
			t.Fatalf("%v %+v", err, res)
		}
	})
}

// A step may be run once. The second attempt runs nothing and leaves a marker
// that `collect` reads.
func TestStepRunsOnlyOnce(t *testing.T) {
	f := newStepFix(t)
	f.must(actZ, 0, 1, "exit 0")
	f.refused(f.opts(actZ, 0, 1, "exit 0"), "already run")
	if !exists(StepFile(f.p.dir, actZ, 0, "rerun")) {
		t.Fatal("no rerun marker")
	}
	if f.runs() != 1 {
		t.Fatalf("%d runs", f.runs())
	}
	// And again: the marker was already there, which changes nothing.
	f.refused(f.opts(actZ, 0, 1, "exit 0"), "already run")
	// The first run's files are untouched.
	if sc := f.sidecar(actZ, 0); !sc.Clean {
		t.Fatalf("%+v", sc)
	}
	// A marker without the step's own files: the step stays refused.
	f2 := newStepFix(t)
	if err := os.MkdirAll(StepDir(f2.p.dir, actZ), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := os.Mkdir(filepath.Join(f2.p.dir, "traces"), 0o700); err != nil && !os.IsExist(err) {
		t.Fatal(err)
	}
	writeBytes(t, StepFile(f2.p.dir, actZ, 0, "rerun"), nil, 0o600)
	f2.refused(f2.opts(actZ, 0, 1, "exit 0"), "run twice")
}

func TestStepOnlyRunsStepsInOrder(t *testing.T) {
	f := newStepFix(t)
	f.refused(f.opts(actB, 1, 2, "exit 0"), "before step 0 finished")
	f.must(actB, 0, 2, "exit 0")
	// Step 0 again is a rerun; step 1 is next.
	f.must(actB, 1, 2, "exit 0")
	// A later step that started before an earlier one is out of order: fake
	// it with a stray file for step 1 of a fresh action.
	f2 := newStepFix(t)
	if err := os.MkdirAll(StepDir(f2.p.dir, actB), 0o700); err != nil {
		t.Fatal(err)
	}
	writeBytes(t, StepFile(f2.p.dir, actB, 1, "trace"), nil, 0o600)
	f2.refused(f2.opts(actB, 0, 2, "exit 0"), "after step 1 had started")
	// Files that are not a step's (no number, no extension) are ignored.
	f3 := newStepFix(t)
	if err := os.MkdirAll(StepDir(f3.p.dir, actZ), 0o700); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"notes", "x.json", "-1.trace"} {
		writeBytes(t, filepath.Join(StepDir(f3.p.dir, actZ), name), nil, 0o600)
	}
	f3.must(actZ, 0, 1, "exit 0")
}

func TestStepRefusesAnEnvironmentThatIsNotTheRecordedOne(t *testing.T) {
	f := newStepFix(t)
	env := f.env(actZ, 0, 1)
	// Which entry of build_tool_env to disturb: a real one that is not a KILN one.
	if len(f.pre.Tools.BuildToolEnv) < 2 {
		t.Fatalf("the fixture's build tool environment is too small: %v", f.pre.Tools.BuildToolEnv)
	}
	victim := f.pre.Tools.BuildToolEnv[0]
	name := victim[:strings.IndexByte(victim, '=')]
	cases := map[string][]string{
		"an extra variable":        append(append([]string{}, env...), "LD_PRELOAD=/tmp/x.so"),
		"a missing variable":       without(env, victim),
		"a changed value":          replaceStr(env, victim, name+"=changed"),
		"a duplicate name":         append(append([]string{}, env...), victim),
		"a duplicate of the same":  append(append([]string{}, env...), name+"=other"),
		"an entry with no =":       append(append([]string{}, env...), "JUSTANAME"),
		"an entry with no name":    append(append([]string{}, env...), "=value"),
		"no KILN_ACTION":           without(env, EnvAction+"="+actZ),
		"no KILN_STEP":             without(env, EnvStep+"=0"),
		"no KILN_STEP_COUNT":       without(env, EnvStepCount+"=1"),
		"a duplicated KILN_ACTION": append(append([]string{}, env...), EnvAction+"="+actA),
	}
	for label, e := range cases {
		o := f.opts(actZ, 0, 1, "exit 0")
		o.Environ = e
		t.Log(label)
		f.refused(o, "environment")
	}
	// The error names variables, never values.
	o := f.opts(actZ, 0, 1, "exit 0")
	o.Environ = replaceStr(env, victim, name+"=SECRET-VALUE-123")
	_, err := RunStep(o)
	if err == nil || strings.Contains(err.Error(), "SECRET-VALUE-123") || !strings.Contains(err.Error(), name) {
		t.Fatalf("%v", err)
	}
	// And the legitimate one runs.
	f.must(actZ, 0, 1, "exit 0")
}

func without(l []string, s string) []string {
	var out []string
	for _, v := range l {
		if v != s {
			out = append(out, v)
		}
	}
	return out
}

func replaceStr(l []string, from, to string) []string {
	out := append([]string{}, l...)
	for i, v := range out {
		if v == from {
			out[i] = to
		}
	}
	return out
}

func TestStepRefusesWhatThePlanDoesNotHave(t *testing.T) {
	f := newStepFix(t)
	for label, c := range map[string]struct {
		action, step, count, want string
	}{
		"an unplanned action":      {"code/packages/x/nope", "0", "1", "not an action"},
		"an empty action":          {"", "0", "1", "not an action"},
		"a dotdot action":          {"code/packages/x/../a", "0", "1", "not an action"},
		"a very long action":       {strings.Repeat("a", 5000), "0", "1", "not an action"},
		"an action with a newline": {"code/packages/x/a\nb", "0", "1", "not an action"},
		"a leading zero step":      {actZ, "00", "1", "canonical"},
		"a signed step":            {actZ, "+0", "1", "canonical"},
		"a negative step":          {actZ, "-1", "1", "canonical"},
		"a non-numeric step":       {actZ, "x", "1", "canonical"},
		"an empty step":            {actZ, "", "1", "canonical"},
		"a huge step":              {actZ, "9999999", "1", "canonical"},
		"a leading zero count":     {actZ, "0", "01", "canonical"},
		"a wrong count":            {actZ, "0", "2", "plan has 1"},
		"a step beyond the count":  {actB, "2", "2", "out of range"},
		"a step as large as count": {actZ, "1", "1", "out of range"},
	} {
		o := f.opts(actZ, 0, 1, "exit 0")
		o.Environ = append(append([]string{}, f.pre.Tools.BuildToolEnv...), EnvAction+"="+c.action, EnvStep+"="+c.step, EnvStepCount+"="+c.count)
		o.Cwd = f.pre.Config.PackageDir(actZ)
		func() {
			defer func() {
				if r := recover(); r != nil {
					t.Fatalf("%s: %v", label, r)
				}
			}()
			f.refused(o, c.want)
		}()
	}
}

func TestStepRefusesAnArgvThatIsNotShDashC(t *testing.T) {
	f := newStepFix(t)
	for label, argv := range map[string][]string{
		"empty":        nil,
		"only sh":      {"sh"},
		"no line":      {"sh", "-c"},
		"another flag": {"sh", "-x", "exit 0"},
		"bash":         {"bash", "-c", "exit 0"},
		"a path":       {"/bin/sh", "-c", "exit 0"},
		"extra args":   {"sh", "-c", "exit 0", "x"},
		"two flags":    {"sh", "-c", "-c", "exit 0", "y"},
	} {
		o := f.opts(actZ, 0, 1, "exit 0")
		o.Argv = argv
		t.Log(label)
		f.refused(o, "sh -c")
	}
}

func TestStepRefusesAnotherWorkingDirectory(t *testing.T) {
	f := newStepFix(t)
	for _, cwd := range []string{f.p.root, f.pre.Config.PackageDir(actA), f.pre.Config.PackageDir(actZ) + "/", "", "relative"} {
		o := f.opts(actZ, 0, 1, "exit 0")
		o.Cwd = cwd
		f.refused(o, "working directory")
	}
}

func TestStepRefusesAProgramThatIsNotThePinnedOne(t *testing.T) {
	t.Run("kiln modified", func(t *testing.T) {
		f := newStepFix(t)
		writeBytes(t, f.p.kiln, []byte("another kiln"), 0o755)
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "not the binary prepare hashed")
	})
	t.Run("strace modified", func(t *testing.T) {
		f := newStepFix(t)
		writeBytes(t, f.p.strace, []byte("#!/bin/sh\nexit 0\n"), 0o755)
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "not the binary prepare hashed")
	})
	t.Run("kiln at another path", func(t *testing.T) {
		f := newStepFix(t)
		other := filepath.Join(f.p.base, "bin", "kiln-copy")
		data, _ := os.ReadFile(f.p.kiln)
		writeBytes(t, other, data, 0o755)
		o := f.opts(actZ, 0, 1, "exit 0")
		o.KilnExe = other
		f.refused(o, "kiln_exe")
	})
	t.Run("kiln missing", func(t *testing.T) {
		f := newStepFix(t)
		o := f.opts(actZ, 0, 1, "exit 0")
		o.KilnExe = filepath.Join(f.p.base, "bin", "absent")
		f.refused(o, "kiln binary")
	})
	t.Run("strace missing", func(t *testing.T) {
		f := newStepFix(t)
		if err := os.Remove(f.p.strace); err != nil {
			t.Fatal(err)
		}
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "strace")
	})
}

// A strace that cannot be started: nothing ran, so nothing was under the
// sandbox, and the sidecar says so (StepNetNS is true only for a step that ran).
func TestStepNetNSIsFalseWhenStraceCouldNotBeStarted(t *testing.T) {
	f := newStepFix(t)
	if err := os.Chmod(f.p.strace, 0o644); err != nil {
		t.Fatal(err)
	}
	res := f.must(actZ, 0, 1, "exit 0")
	sc := res.Sidecar
	if res.ExitCode != ExitRefused || sc.Started || sc.StepNetNS || sc.Clean || !containsStr(sc.Unclean, UncleanNotStarted) || sc.StraceExit != -1 {
		t.Fatalf("%d %+v", res.ExitCode, sc)
	}
	if got := f.sidecar(actZ, 0); got.StepNetNS || got.Started {
		t.Fatalf("%+v", got)
	}
}

func TestStepStepNetNSIsTrueOnlyForAStepThatRan(t *testing.T) {
	f := newStepFix(t)
	if sc := f.must(actZ, 0, 1, "exit 9").Sidecar; !sc.StepNetNS || !sc.Started {
		t.Fatalf("%+v", sc)
	}
	// Every refusal runs nothing and writes no sidecar at all, so there is no
	// record that could claim a network namespace.
	f2 := newStepFix(t)
	o := f2.opts(actZ, 0, 1, "exit 0")
	o.Argv = []string{"bash", "-c", "exit 0"}
	f2.refused(o, "sh -c")
	if exists(StepFile(f2.p.dir, actZ, 0, "json")) {
		t.Fatal("a refused step left a sidecar")
	}
}

func TestStepLineInPlan(t *testing.T) {
	f := newStepFix(t)
	if sc := f.must(actZ, 0, 1, "exit 0").Sidecar; !sc.LineInPlan {
		t.Fatalf("%+v", sc)
	}
	f2 := newStepFix(t)
	if sc := f2.must(actZ, 0, 1, "exit 0 # not the planned line").Sidecar; sc.LineInPlan {
		t.Fatalf("%+v", sc)
	}
}

func TestStepBrokenRelayTargetDoesNotStallTheStep(t *testing.T) {
	f := newStepFix(t)
	o := f.opts(actZ, 0, 1, "{ yes hello | head -c 300000 >&3; } 2>/dev/null; exit 0")
	o.Stdout = errWriter{}
	res, err := RunStep(o)
	if err != nil || res.ExitCode != 0 || !res.Sidecar.Clean {
		t.Fatalf("%v %+v", err, res)
	}
}

type errWriter struct{}

func (errWriter) Write([]byte) (int, error) { return 0, errors.New("closed") }

// A survivor that still holds the step's pipes must not hold the build.
func TestStepASurvivorHoldingThePipesIsRecordedNotWaitedFor(t *testing.T) {
	f := newStepFix(t)
	o := f.opts(actZ, 0, 1, "sleep 6 & exit 0")
	o.Drain = 300 * time.Millisecond
	start := time.Now()
	res, err := RunStep(o)
	if err != nil {
		t.Fatal(err)
	}
	if time.Since(start) > 5*time.Second {
		t.Fatalf("waited %s", time.Since(start))
	}
	if res.Sidecar.Clean || !containsStr(res.Sidecar.Unclean, UncleanPipesHeld) || res.ExitCode != 0 {
		t.Fatalf("%d %+v", res.ExitCode, res.Sidecar)
	}
}

func TestStepFailsWhenTheTraceOrSidecarCannotBeAccountedFor(t *testing.T) {
	t.Run("the trace vanished", func(t *testing.T) {
		f := newStepFix(t)
		_, err := f.run(actZ, 0, 1, `rm "$tf"`)
		if err == nil || !strings.Contains(err.Error(), "the trace") {
			t.Fatalf("%v", err)
		}
	})
	t.Run("the trace is not a regular file", func(t *testing.T) {
		f := newStepFix(t)
		_, err := f.run(actZ, 0, 1, `rm "$tf"; mkfifo "$tf"`)
		if err == nil || !strings.Contains(err.Error(), "the trace") {
			t.Fatalf("%v", err)
		}
	})
	t.Run("the sidecar exists already", func(t *testing.T) {
		f := newStepFix(t)
		_, err := f.run(actZ, 0, 1, `touch "${tf%.trace}.json"`)
		if err == nil || !strings.Contains(err.Error(), "sidecar") {
			t.Fatalf("%v", err)
		}
	})
}

func TestStepRefusesAnUnusableTracesDirectory(t *testing.T) {
	t.Run("a symlink", func(t *testing.T) {
		f := newStepFix(t)
		other := filepath.Join(f.p.base, "elsewhere")
		if err := os.Mkdir(other, 0o700); err != nil {
			t.Fatal(err)
		}
		if err := os.Symlink(other, filepath.Join(f.p.dir, "traces")); err != nil {
			t.Skip(err)
		}
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "not a real directory")
	})
	t.Run("a file", func(t *testing.T) {
		f := newStepFix(t)
		writeBytes(t, filepath.Join(f.p.dir, "traces"), nil, 0o600)
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "traces of")
	})
	t.Run("open to others", func(t *testing.T) {
		f := newStepFix(t)
		if err := os.Mkdir(filepath.Join(f.p.dir, "traces"), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.Chmod(filepath.Join(f.p.dir, "traces"), 0o755); err != nil {
			t.Fatal(err)
		}
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "accessible to group or others")
	})
	t.Run("the trace file exists", func(t *testing.T) {
		d := canonTemp(t)
		if _, err := createTrace(d, actZ, 0); err != nil {
			t.Fatal(err)
		}
		if _, err := createTrace(d, actZ, 0); err == nil || !errors.Is(err, os.ErrExist) {
			t.Fatalf("O_EXCL: %v", err)
		}
	})
	t.Run("the trace directory is unreadable", func(t *testing.T) {
		f := newStepFix(t)
		f.must(actZ, 0, 1, "exit 0")
		if err := os.Chmod(StepDir(f.p.dir, actZ), 0o300); err != nil {
			t.Fatal(err)
		}
		defer os.Chmod(StepDir(f.p.dir, actZ), 0o700)
		if os.Geteuid() == 0 {
			t.Skip("root reads anything")
		}
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "traces of")
	})
}

func TestStepDoesNotStartWhenPreIsUnreadable(t *testing.T) {
	f := newStepFix(t)
	o := f.opts(actZ, 0, 1, "exit 0")
	o.Dir = filepath.Join(f.p.base, "no-such-record-dir")
	f.refused(o, "no-such-record-dir")
	o.Dir = "relative"
	f.refused(o, "absolute")
}

func TestStepStdoutAndStderrAreOptional(t *testing.T) {
	f := newStepFix(t)
	o := f.opts(actZ, 0, 1, "echo x >&3; echo y >&4")
	o.Stdout, o.Stderr = nil, nil
	if res, err := RunStep(o); err != nil || res.ExitCode != 0 {
		t.Fatalf("%v %+v", err, res)
	}
}

func TestStepDefaultsAreTheDesignReviewsFigures(t *testing.T) {
	if DefaultTraceCap != 1<<30 || DefaultDiskFloor != 4<<30 || DefaultPoll != 250*time.Millisecond || DefaultDrain != 10*time.Second {
		t.Fatal("G8 says: 1 GiB cap, 4 GiB floor, 250 ms poll")
	}
	// A real run with the defaults (the fixture's temp disk is not under 4 GiB
	// on a developer's machine or a runner; skip when it is).
	f := newStepFix(t)
	if free, err := freeBytes(f.p.dir); err != nil || free < DefaultDiskFloor+1<<30 {
		t.Skipf("not enough free disk for the default floor: %d %v", free, err)
	}
	o := f.opts(actZ, 0, 1, "exit 0")
	o.Poll, o.Drain = 0, 0
	if res, err := RunStep(o); err != nil || res.ExitCode != 0 {
		t.Fatalf("%v %+v", err, res)
	}
}

func TestFreeBytes(t *testing.T) {
	n, err := freeBytes(canonTemp(t))
	if err != nil || n == 0 {
		t.Fatalf("%d %v", n, err)
	}
	if _, err := freeBytes(filepath.Join(canonTemp(t), "absent")); err == nil {
		t.Fatal("a missing path has free space")
	}
}

func TestCaptureKeepsTheFirstBytesAndCountsAll(t *testing.T) {
	var c capture
	c.Write([]byte(strings.Repeat("a", 5000)))
	c.Write([]byte("more"))
	if len(c.b) != 4096 || c.n != 5004 {
		t.Fatalf("%d %d", len(c.b), c.n)
	}
}

// ---------------------------------------------------------------------------
// Tampering with pre/ (decision P11: step re-verifies it).

// reseal rewrites SEALED for the files as they are now. When fixBinding is
// set, it first makes the binding's digests say what the files are, so a test
// can reach the checks behind the digests. extra replaces named files first.
func (f *stepFix) reseal(extra map[string][]byte, fixBinding bool) {
	f.t.Helper()
	f.p.writable()
	pre := filepath.Join(f.p.dir, "pre")
	for rel, data := range extra {
		if err := os.WriteFile(filepath.Join(pre, filepath.FromSlash(rel)), data, 0o644); err != nil {
			f.t.Fatal(err)
		}
	}
	sum := func(rel string) string {
		data, err := os.ReadFile(filepath.Join(pre, filepath.FromSlash(rel)))
		if err != nil {
			f.t.Fatal(err)
		}
		s := sha256.Sum256(data)
		return hex.EncodeToString(s[:])
	}
	if fixBinding {
		data, _ := os.ReadFile(filepath.Join(pre, "binding.json"))
		b, err := recfmt.DecodeBinding(data)
		if err != nil {
			f.t.Fatal(err)
		}
		b.GraphSHA256, b.StepsSHA256, b.HostFactsSHA256 = sum("graph.json"), sum("steps.json"), sum("host-facts.json")
		b.InventorySHA256, b.SandboxConfigSHA256 = sum("inventory.json"), sum("sandbox.json")
		b.StraceSHA256 = sumFile(f.t, f.p.strace)
		nb, err := recfmt.EncodeBinding(b)
		if err != nil {
			f.t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(pre, "binding.json"), nb, 0o644); err != nil {
			f.t.Fatal(err)
		}
	}
	var rels []string
	filepathWalkFiles(f.t, pre, func(rel string) {
		if rel != "SEALED" {
			rels = append(rels, rel)
		}
	})
	rels = sortedCopy(rels)
	var b strings.Builder
	for _, rel := range rels {
		fmt.Fprintf(&b, "%s  %s\n", sum(rel), rel)
	}
	if err := os.WriteFile(filepath.Join(pre, "SEALED"), []byte(b.String()), 0o644); err != nil {
		f.t.Fatal(err)
	}
}

func sumFile(t *testing.T, p string) string {
	t.Helper()
	b, err := ResolveBinary("x", p)
	if err != nil {
		t.Fatal(err)
	}
	return b.SHA256
}

func filepathWalkFiles(t *testing.T, root string, fn func(rel string)) {
	t.Helper()
	err := filepath.Walk(root, func(p string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.Mode().IsRegular() {
			rel, _ := filepath.Rel(root, p)
			fn(filepath.ToSlash(rel))
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
}

func TestStepRefusesATamperedPre(t *testing.T) {
	pre := func(f *stepFix, rel string) string { return filepath.Join(f.p.dir, "pre", filepath.FromSlash(rel)) }
	cases := []struct {
		name   string
		mutate func(t *testing.T, f *stepFix)
		want   string
	}{
		{"a modified file", func(t *testing.T, f *stepFix) {
			f.p.writable()
			data, _ := os.ReadFile(pre(f, "graph.json"))
			os.WriteFile(pre(f, "graph.json"), append(data, ' '), 0o644)
		}, "was modified"},
		{"a modified action", func(t *testing.T, f *stepFix) {
			f.p.writable()
			p := pre(f, ActionFile(actZ))
			data, _ := os.ReadFile(p)
			os.WriteFile(p, append(data, ' '), 0o644)
		}, "was modified"},
		{"an extra file", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.WriteFile(pre(f, "extra.json"), []byte("{}"), 0o644)
		}, "not listed in SEALED"},
		{"an extra action file", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.WriteFile(pre(f, "actions/"+strings.Repeat("0", 64)+".json"), []byte("{}"), 0o644)
		}, "not listed in SEALED"},
		{"an extra directory", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.Mkdir(pre(f, "more"), 0o755)
		}, "unexpected directory"},
		{"a symlink", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.Symlink("graph.json", pre(f, "link"))
		}, "symlink"},
		{"a symlink in place of a file", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.Remove(pre(f, "tools.json"))
			os.Symlink(pre(f, "graph.json"), pre(f, "tools.json"))
		}, "symlink"},
		{"a missing file", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.Remove(pre(f, "steps.json"))
		}, "missing"},
		{"a missing SEALED", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.Remove(pre(f, "SEALED"))
		}, "SEALED"},
		{"a symlinked SEALED", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.Rename(pre(f, "SEALED"), pre(f, "SEALED.real"))
			os.Symlink(pre(f, "SEALED.real"), pre(f, "SEALED"))
		}, "SEALED"},
		{"a directory for SEALED", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.Remove(pre(f, "SEALED"))
			os.Mkdir(pre(f, "SEALED"), 0o755)
		}, "SEALED"},
		{"an empty SEALED", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.WriteFile(pre(f, "SEALED"), nil, 0o644)
		}, "newline"},
		{"no final newline", func(t *testing.T, f *stepFix) {
			f.p.writable()
			data, _ := os.ReadFile(pre(f, "SEALED"))
			os.WriteFile(pre(f, "SEALED"), bytes.TrimSuffix(data, []byte("\n")), 0o644)
		}, "newline"},
		{"a malformed line", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.WriteFile(pre(f, "SEALED"), []byte("not a digest line\n"), 0o644)
		}, "line 1"},
		{"a single space", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.WriteFile(pre(f, "SEALED"), []byte(strings.Repeat("a", 64)+" binding.json\n"), 0o644)
		}, "line 1"},
		{"an uppercase digest", func(t *testing.T, f *stepFix) {
			f.p.writable()
			data, _ := os.ReadFile(pre(f, "SEALED"))
			os.WriteFile(pre(f, "SEALED"), append([]byte(strings.Repeat("A", 64)+"  aaa\n"), data...), 0o644)
		}, "line 1"},
		{"an unsorted list", func(t *testing.T, f *stepFix) {
			f.p.writable()
			data, _ := os.ReadFile(pre(f, "SEALED"))
			lines := strings.Split(strings.TrimSuffix(string(data), "\n"), "\n")
			lines[0], lines[1] = lines[1], lines[0]
			os.WriteFile(pre(f, "SEALED"), []byte(strings.Join(lines, "\n")+"\n"), 0o644)
		}, "is not after"},
		{"a repeated path", func(t *testing.T, f *stepFix) {
			f.p.writable()
			data, _ := os.ReadFile(pre(f, "SEALED"))
			lines := strings.Split(strings.TrimSuffix(string(data), "\n"), "\n")
			lines = append([]string{lines[0]}, lines...)
			os.WriteFile(pre(f, "SEALED"), []byte(strings.Join(lines, "\n")+"\n"), 0o644)
		}, "is not after"},
		{"a missing required entry", func(t *testing.T, f *stepFix) {
			f.p.writable()
			data, _ := os.ReadFile(pre(f, "SEALED"))
			var out []string
			for _, l := range strings.Split(strings.TrimSuffix(string(data), "\n"), "\n") {
				if !strings.HasSuffix(l, "  tools.json") {
					out = append(out, l)
				}
			}
			os.WriteFile(pre(f, "SEALED"), []byte(strings.Join(out, "\n")+"\n"), 0o644)
		}, "does not list tools.json"},
		{"a path that escapes", func(t *testing.T, f *stepFix) {
			f.p.writable()
			os.WriteFile(pre(f, "SEALED"), []byte(strings.Repeat("a", 64)+"  ../x\n"), 0o644)
		}, "line 1"},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			f := newStepFix(t)
			c.mutate(t, f)
			f.refused(f.opts(actZ, 0, 1, "exit 0"), c.want)
			if exists(StepFile(f.p.dir, actZ, 0, "trace")) {
				t.Fatal("a trace was created under a tampered pre/")
			}
		})
	}
}

// A tamperer who also fixes SEALED is caught by what SEALED does not cover: the
// binding's digests, and the strict decoders behind them.
func TestStepRefusesAResealedPreWhoseContentIsWrong(t *testing.T) {
	cases := []struct {
		name  string
		files map[string][]byte
		fix   bool
		want  string
	}{
		{"a graph the binding does not name", map[string][]byte{"graph.json": []byte(`{"x":1}`)}, false, "different graph.json"},
		{"a sandbox configuration the binding does not name", map[string][]byte{"sandbox.json": []byte(`{}`)}, false, "different sandbox.json"},
		{"a host facts file the binding does not name", map[string][]byte{"host-facts.json": []byte(`{}`)}, false, "different host-facts.json"},
		{"an inventory the binding does not name", map[string][]byte{"inventory.json": []byte(`{}`)}, false, "different inventory.json"},
		{"steps the binding does not name", map[string][]byte{"steps.json": []byte(`{}`)}, false, "different steps.json"},
		{"a sandbox configuration that does not parse", map[string][]byte{"sandbox.json": []byte(`{"repo_root":"x"}`)}, true, "sandbox.json"},
		{"tools that are not JSON", map[string][]byte{"tools.json": []byte(`nope`)}, true, "tools"},
		{"tools with an unknown key", map[string][]byte{"tools.json": []byte(`{"schema":"kiln-record-tools/1","git":"/g","strace":"/s","path":"/p","build_tool_env":[],"x":1}`)}, true, "tools"},
		{"tools with trailing data", map[string][]byte{"tools.json": []byte(`{"schema":"kiln-record-tools/1","git":"/g","strace":"/s","path":"/p","build_tool_env":[]} {}`)}, true, "unexpected data"},
		{"tools of another schema", map[string][]byte{"tools.json": []byte(`{"schema":"x","git":"/g","strace":"/s","path":"/p","build_tool_env":[]}`)}, true, "schema"},
		{"tools with a relative strace", map[string][]byte{"tools.json": []byte(`{"schema":"kiln-record-tools/1","git":"/g","strace":"strace","path":"/p","build_tool_env":[]}`)}, true, "absolute"},
		{"tools with an unclean git", map[string][]byte{"tools.json": []byte(`{"schema":"kiln-record-tools/1","git":"/a/../g","strace":"/s","path":"/p","build_tool_env":[]}`)}, true, "clean"},
		{"tools with a duplicated name", map[string][]byte{"tools.json": []byte(`{"schema":"kiln-record-tools/1","git":"/g","strace":"/s","path":"/p","build_tool_env":["A=1","A=2"]}`)}, true, "twice"},
		{"tools with an entry with no =", map[string][]byte{"tools.json": []byte(`{"schema":"kiln-record-tools/1","git":"/g","strace":"/s","path":"/p","build_tool_env":["A"]}`)}, true, "NAME=value"},
		{"a binding that is not valid", map[string][]byte{"binding.json": []byte(`{}`)}, false, "binding"},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			f := newStepFix(t)
			files := c.files
			if _, ok := files["binding.json"]; ok {
				f.reseal(files, false)
			} else {
				f.reseal(files, c.fix)
			}
			f.refused(f.opts(actZ, 0, 1, "exit 0"), c.want)
		})
	}
	t.Run("an action file with another name", func(t *testing.T) {
		f := newStepFix(t)
		data, _ := os.ReadFile(filepath.Join(f.p.dir, "pre", filepath.FromSlash(ActionFile(actZ))))
		f.p.writable()
		os.Remove(filepath.Join(f.p.dir, "pre", filepath.FromSlash(ActionFile(actZ))))
		f.reseal(map[string][]byte{"actions/" + strings.Repeat("1", 64) + ".json": data}, false)
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "whose file is")
	})
	t.Run("an action the sandbox has no environment for", func(t *testing.T) {
		f := newStepFix(t)
		a := Action{Schema: ActionSchema, Action: "code/packages/x/ghost", Language: "x", Weak: strings.Repeat("a", 64), Steps: []string{"true"}}
		data, err := EncodeAction(a)
		if err != nil {
			t.Fatal(err)
		}
		f.reseal(map[string][]byte{ActionFile(a.Action): data}, false)
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "no environment")
	})
	t.Run("an action file that does not decode", func(t *testing.T) {
		f := newStepFix(t)
		f.reseal(map[string][]byte{ActionFile(actZ): []byte(`{"schema":"x"}`)}, false)
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "action")
	})
	t.Run("a symlinked pre directory", func(t *testing.T) {
		f := newStepFix(t)
		f.p.writable()
		real := filepath.Join(f.p.dir, "pre-real")
		os.Rename(filepath.Join(f.p.dir, "pre"), real)
		os.Symlink(real, filepath.Join(f.p.dir, "pre"))
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "not a real directory")
	})
	t.Run("pre is missing", func(t *testing.T) {
		f := newStepFix(t)
		f.p.writable()
		os.RemoveAll(filepath.Join(f.p.dir, "pre"))
		f.refused(f.opts(actZ, 0, 1, "exit 0"), "pre/")
	})
}

func TestPreExposesTheVerifiedPlan(t *testing.T) {
	f := newStepFix(t)
	if got := f.pre.Actions(); strings.Join(got, ",") != actA+","+actB+","+actZ {
		t.Fatalf("%v", got)
	}
	if a, ok := f.pre.Action(actB); !ok || len(a.Steps) != 2 {
		t.Fatalf("%+v %v", a, ok)
	}
	if _, ok := f.pre.Action("nope"); ok {
		t.Fatal("an unplanned action exists")
	}
	if f.pre.BindingID != sumOf(readPreFile(t, f.p.dir, "binding.json")) || f.pre.Binding.Revision != f.p.sha {
		t.Fatalf("%s", f.pre.BindingID)
	}
	if f.pre.ConfigPath != filepath.Join(f.p.dir, "pre", "sandbox.json") {
		t.Fatal(f.pre.ConfigPath)
	}
}

func readPreFile(t *testing.T, dir, rel string) []byte {
	t.Helper()
	b, err := os.ReadFile(filepath.Join(dir, "pre", rel))
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func TestCheckSealedPath(t *testing.T) {
	for _, ok := range []string{"graph.json", "actions/" + strings.Repeat("a", 64) + ".json"} {
		if err := checkSealedPath(ok); err != nil {
			t.Errorf("%q: %v", ok, err)
		}
	}
	for _, bad := range []string{"", "SEALED", "/abs", "a/../b", "./a", "a//b", "a\\b", "a\x00b", "a\nb", "x/y", "actions/", "actions/a/b", "a/b/c"} {
		if err := checkSealedPath(bad); err == nil {
			t.Errorf("%q was accepted", bad)
		}
	}
}

func TestHashPreFileRefusals(t *testing.T) {
	d := canonTemp(t)
	if _, _, err := hashPreFile(filepath.Join(d, "absent"), false); err == nil {
		t.Fatal("absent")
	}
	if _, _, err := hashPreFile(d, false); err == nil {
		t.Fatal("a directory was hashed")
	}
	f := filepath.Join(d, "f")
	writeBytes(t, f, []byte("abc"), 0o600)
	data, sum, err := hashPreFile(f, true)
	if err != nil || string(data) != "abc" || sum != sumOf([]byte("abc")) {
		t.Fatalf("%q %s %v", data, sum, err)
	}
	if data, _, err := hashPreFile(f, false); err != nil || data != nil && len(data) != 0 {
		t.Fatalf("%q %v", data, err)
	}
}

func TestEnvNamesAndCheckStepEnv(t *testing.T) {
	if _, err := envNames([]string{"A=1", "B="}); err != nil {
		t.Fatal(err)
	}
	for _, bad := range [][]string{{"A"}, {"=x"}, {"A=1", "A=1"}} {
		if _, err := envNames(bad); err == nil {
			t.Errorf("%v accepted", bad)
		}
	}
	want := []string{"HOME=/h", "PATH=/p"}
	good := []string{"HOME=/h", "KILN_ACTION=a", "PATH=/p", "KILN_STEP=0", "KILN_STEP_COUNT=1"}
	if vars, err := checkStepEnv(good, want); err != nil || vars[EnvAction] != "a" || vars[EnvStep] != "0" || vars[EnvStepCount] != "1" {
		t.Fatalf("%v %v", vars, err)
	}
	// Many differences are reported, capped.
	var many, wantMany []string
	for i := 0; i < 20; i++ {
		many = append(many, fmt.Sprintf("V%02d=1", i))
		wantMany = append(wantMany, fmt.Sprintf("W%02d=1", i))
	}
	_, err := checkStepEnv(append(many, "KILN_ACTION=a", "KILN_STEP=0", "KILN_STEP_COUNT=1"), wantMany)
	if err == nil || !strings.Contains(err.Error(), "...") {
		t.Fatalf("%v", err)
	}
	if _, err := checkStepEnv(good, []string{"X"}); err == nil || !strings.Contains(err.Error(), "build_tool_env") {
		t.Fatalf("%v", err)
	}
}

func TestCanonicalInt(t *testing.T) {
	for s, want := range map[string]int{"0": 0, "7": 7, "10": 10, "123456": 123456} {
		if n, err := canonicalInt("x", s); err != nil || n != want {
			t.Errorf("%q: %d %v", s, n, err)
		}
	}
	for _, s := range []string{"", "00", "01", "-1", "+1", "1.0", "1e3", "1234567", " 1", "1 ", "\u0661"} {
		if _, err := canonicalInt("x", s); err == nil {
			t.Errorf("%q accepted", s)
		}
	}
}

func TestStepSidecarValidationAndStrictDecoding(t *testing.T) {
	good := StepSidecar{
		Schema: StepSchema, Action: actZ, Step: 0, StepCount: 1, BindingID: strings.Repeat("a", 64),
		LineSHA256: strings.Repeat("b", 64), Started: true, StartedAt: "2026-11-13T00:00:00Z", EndedAt: "2026-11-13T00:00:01.5Z",
		StraceStdoutEmpty: true, StraceStderrEmpty: true, TraceSHA256: strings.Repeat("c", 64), StepNetNS: true,
		Clean: true, Unclean: []string{},
	}
	data, err := EncodeStepSidecar(good)
	if err != nil {
		t.Fatal(err)
	}
	back, err := DecodeStepSidecar(data)
	if err != nil || fmt.Sprint(back) != fmt.Sprint(good) {
		t.Fatalf("%v %+v", err, back)
	}
	// A nil Unclean is rendered as a list.
	n := good
	n.Unclean = nil
	if d, err := EncodeStepSidecar(n); err != nil || !bytes.Contains(d, []byte(`"unclean":[]`)) {
		t.Fatalf("%v %s", err, d)
	}
	mods := map[string]func(s *StepSidecar){
		"schema":                  func(s *StepSidecar) { s.Schema = "x" },
		"negative step":           func(s *StepSidecar) { s.Step = -1 },
		"step beyond the count":   func(s *StepSidecar) { s.Step = 1 },
		"zero count":              func(s *StepSidecar) { s.StepCount = 0 },
		"binding id":              func(s *StepSidecar) { s.BindingID = "xyz" },
		"line digest":             func(s *StepSidecar) { s.LineSHA256 = "" },
		"trace digest":            func(s *StepSidecar) { s.TraceSHA256 = strings.Repeat("C", 64) },
		"start time":              func(s *StepSidecar) { s.StartedAt = "yesterday" },
		"end time":                func(s *StepSidecar) { s.EndedAt = "" },
		"exit too small":          func(s *StepSidecar) { s.StraceExit, s.RootStatus = -2, -2 },
		"exit too large":          func(s *StepSidecar) { s.StraceExit, s.RootStatus = 256, 256 },
		"signal too large":        func(s *StepSidecar) { s.StraceSignal = 65 },
		"exit and signal differ":  func(s *StepSidecar) { s.StraceExit = -1; s.RootStatus = -1 },
		"signal with an exit":     func(s *StepSidecar) { s.StraceSignal = 9 },
		"root status differs":     func(s *StepSidecar) { s.RootStatus = 1 },
		"exit125 without 125":     func(s *StepSidecar) { s.Exit125 = true },
		"negative trace size":     func(s *StepSidecar) { s.TraceBytes = -1 },
		"truncated without why":   func(s *StepSidecar) { s.Truncated = true },
		"why without truncated":   func(s *StepSidecar) { s.TruncatedReason = TruncTooLarge },
		"an unknown why":          func(s *StepSidecar) { s.Truncated, s.TruncatedReason = true, "x" },
		"netns without start":     func(s *StepSidecar) { s.StepNetNS = false },
		"null unclean":            func(s *StepSidecar) { s.Unclean = nil },
		"clean with reasons":      func(s *StepSidecar) { s.Unclean = []string{"x"} },
		"unclean with no reasons": func(s *StepSidecar) { s.Clean = false },
		"clean but truncated": func(s *StepSidecar) {
			s.Truncated, s.TruncatedReason = true, TruncDiskFloor
		},
		"clean with stderr": func(s *StepSidecar) { s.StraceStderrEmpty = false },
		"clean with stdout": func(s *StepSidecar) { s.StraceStdoutEmpty = false },
		"clean but signalled": func(s *StepSidecar) {
			s.StraceExit, s.RootStatus, s.StraceSignal = -1, -1, 9
		},
	}
	for name, m := range mods {
		s := good
		s.Unclean = []string{}
		m(&s)
		if name == "null unclean" {
			// Encode repairs nil; decode must not.
			data := bytes.Replace(data, []byte(`"unclean":[]`), []byte(`"unclean":null`), 1)
			if _, err := DecodeStepSidecar(data); err == nil {
				t.Errorf("%s: decoded", name)
			}
			continue
		}
		if err := s.Validate(); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// Unstarted is valid and unclean.
	ns := good
	ns.Started, ns.StepNetNS, ns.Clean, ns.Unclean = false, false, false, []string{UncleanNotStarted}
	ns.StraceExit, ns.RootStatus = -1, -1
	if _, err := EncodeStepSidecar(ns); err != nil {
		t.Fatal(err)
	}
	for name, bad := range map[string][]byte{
		"unknown key":   bytes.Replace(data, []byte(`"step":0`), []byte(`"step":0,"x":1`), 1),
		"trailing data": append(append([]byte{}, data...), []byte(` {}`)...),
		"not json":      []byte(`nope`),
		"empty":         nil,
	} {
		if _, err := DecodeStepSidecar(bad); err == nil {
			t.Errorf("%s: decoded", name)
		}
	}
	if _, err := EncodeStepSidecar(StepSidecar{}); err == nil {
		t.Fatal("an empty sidecar was encoded")
	}
}
