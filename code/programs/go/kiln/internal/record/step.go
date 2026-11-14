package record

// `kiln record step`: the per-step supervisor (KILN04 section 4.8, decision P12)
// ==============================================================================
//
// The build tool runs every BUILD line of a shard through
//
//	kiln record step --dir <dir> -- sh -c <line>
//
// (its -exec-wrapper, decision P1), with KILN_ACTION, KILN_STEP and
// KILN_STEP_COUNT added to its own environment. This file is that command:
//
//	build tool                      the wrapper never trusts what it is given
//	   |
//	   v
//	kiln record step                outside the sandbox, not traced itself
//	   |  verifies pre/, the environment, the step's place in the plan
//	   |  creates traces/<sha>/<n>.trace   (O_EXCL: a step runs once)
//	   v
//	strace -f ... -o <n>.trace -- kiln sandbox-exec --config pre/sandbox.json
//	   |                              --stdout-fd 3 --stderr-fd 4 -- sh -c <line>
//	   |   fd 3, fd 4: fresh pipes; the supervisor copies them to its own
//	   |   stdout and stderr, so the build log still shows the step's output
//	   |   strace's own stdout and stderr: two more pipes, kept apart
//	   v
//	the step (sh -c <line>), in the sandbox's PID, mount and network namespaces
//
// and afterwards writes `<n>.json`, the sidecar of stepfile.go, and exits with
// the traced root's status, so the build tool sees a passing step pass and a
// failing one fail.
//
// What it refuses (exit 125, nothing run, nothing but a diagnostic written)
// -------------------------------------------------------------------------
//
//	pre/ is not what `prepare` sealed             the binding, the actions, the
//	                                              sandbox configuration, the tools
//	the Kiln or strace binary is not the pinned   their SHA-256 is in the binding
//	  one, or Kiln is not the configured kiln_exe
//	the environment is not exactly build_tool_env plus the three KILN_ variables
//	KILN_ACTION is not an action of the plan; KILN_STEP / KILN_STEP_COUNT are not
//	  canonical integers, the count is not the plan's, or the step is not the next
//	the argv is not exactly  sh -c <line>
//	the working directory is not <root>/<action>
//	the disk is short
//
// Nothing is looked up on PATH: strace is the path in tools.json, the sandbox
// helper is this very binary, and the child's PATH is the recorded one.
//
// What it does NOT do: classify. It never opens its own trace. The facts it
// records are the ones the trace cannot hold (how strace itself fared, and how
// big the trace was allowed to grow); `collect`, in the next PR, parses the
// trace and applies the classifier.
//
// Trace size (G8). A trace of a real step is large (the compiler's every
// `openat`), and an rlimit on strace would be inherited by the step, which would
// then die of SIGXFSZ when a BUILD writes a big file. So the trace goes to
// ordinary disk and a goroutine polls it: above the cap, or with the disk under
// its floor, strace is killed with SIGKILL (`--kill-on-exit` makes the kernel
// kill every tracee with it) and the sidecar says `truncated`.
//
// If the supervisor itself is killed, strace must not outlive it: the child is
// started with Pdeathsig = SIGKILL, from a goroutine pinned to its OS thread
// (Pdeathsig fires when the THREAD that forked the child exits, and an unpinned
// goroutine's thread may be retired by the Go runtime at any moment).

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// The default limits (design review G8).
const (
	DefaultTraceCap  int64  = 1 << 30 // a trace above 1 GiB is cut
	DefaultDiskFloor uint64 = 4 << 30 // free space under 4 GiB stops a step
	DefaultPoll             = 250 * time.Millisecond
)

// The three variables the build tool adds to its own environment.
const (
	EnvAction    = "KILN_ACTION"
	EnvStep      = "KILN_STEP"
	EnvStepCount = "KILN_STEP_COUNT"
)

// ExitRefused is the exit status of a step the supervisor did not run, or ran
// and could not account for. It is 125 for the same reason the sandbox uses it:
// "the recorder could not do what it was asked", which a build treats as a
// failure and which is never retried.
const ExitRefused = 125

// DefaultDrain bounds how long the supervisor waits for the step's output to drain
// once strace is gone. A survivor that holds a pipe open must not hold the
// supervisor, and with it the build, forever.
const DefaultDrain = 10 * time.Second

// StepOptions are the inputs of RunStep.
type StepOptions struct {
	// Dir is the record directory (absolute, canonical).
	Dir string
	// Argv is what followed `--`: it must be exactly sh, -c, <line>.
	Argv []string
	// Environ is the supervisor's inherited environment; Cwd its working
	// directory; KilnExe the running binary.
	Environ []string
	Cwd     string
	KilnExe string
	// Stdout and Stderr receive the step's output.
	Stdout, Stderr io.Writer

	// Limits; zero means the default.
	TraceCap  int64
	DiskFloor uint64
	Poll      time.Duration
	// Drain bounds the wait for the step's output once strace has exited.
	Drain time.Duration

	// Seams, for tests. Each defaults to the real thing.
	FreeBytes func(dir string) (uint64, error)
	Now       func() time.Time
}

// StepResult is what RunStep reports when the step was supervised.
type StepResult struct {
	// ExitCode is what the supervisor exits with: the traced root's status.
	ExitCode int
	Sidecar  StepSidecar
}

// RunStep supervises one step. An error means nothing ran (or the sidecar could
// not be written); the caller exits ExitRefused.
func RunStep(o StepOptions) (*StepResult, error) {
	if o.TraceCap == 0 {
		o.TraceCap = DefaultTraceCap
	}
	if o.DiskFloor == 0 {
		o.DiskFloor = DefaultDiskFloor
	}
	if o.Poll == 0 {
		o.Poll = DefaultPoll
	}
	if o.Drain == 0 {
		o.Drain = DefaultDrain
	}
	if o.FreeBytes == nil {
		o.FreeBytes = freeBytes
	}
	if o.Now == nil {
		o.Now = time.Now
	}
	if o.Stdout == nil {
		o.Stdout = io.Discard
	}
	if o.Stderr == nil {
		o.Stderr = io.Discard
	}

	// 1. pre/ is what prepare sealed, and the programs are the pinned ones.
	pre, err := VerifyPre(o.Dir)
	if err != nil {
		return nil, err
	}
	kiln, err := ResolveBinary("the kiln binary", o.KilnExe)
	if err != nil {
		return nil, err
	}
	strace, err := ResolveBinary("strace", pre.Tools.Strace)
	if err != nil {
		return nil, err
	}
	if kiln.SHA256 != pre.Binding.KilnSHA256 {
		return nil, fmt.Errorf("the running kiln is not the binary prepare hashed into the binding")
	}
	if strace.SHA256 != pre.Binding.StraceSHA256 {
		return nil, fmt.Errorf("strace %q is not the binary prepare hashed into the binding", printable(strace.Path))
	}
	if kiln.Path != pre.Config.KilnExe {
		return nil, fmt.Errorf("the running kiln %q is not the sandbox configuration's kiln_exe %q", printable(kiln.Path), printable(pre.Config.KilnExe))
	}

	// 2. The environment is exactly the recorded one.
	vars, err := checkStepEnv(o.Environ, pre.Tools.BuildToolEnv)
	if err != nil {
		return nil, err
	}

	// 3. The step is one the plan has, and the next one.
	action, step, count, err := checkStepPlace(pre, vars)
	if err != nil {
		return nil, err
	}
	planned, _ := pre.Action(action)
	if len(o.Argv) != 3 || o.Argv[0] != "sh" || o.Argv[1] != "-c" {
		return nil, fmt.Errorf("the step's command must be exactly: sh -c <line>")
	}
	line := o.Argv[2]
	if want := pre.Config.PackageDir(action); o.Cwd != want {
		return nil, fmt.Errorf("the working directory %q is not the package directory %q of KILN_ACTION", printable(o.Cwd), printable(want))
	}

	// 4. Disk, then the trace file.
	if free, err := o.FreeBytes(o.Dir); err != nil {
		return nil, fmt.Errorf("free space of the record directory: %v", err)
	} else if free < o.DiskFloor {
		return nil, fmt.Errorf("only %d bytes are free under the record directory, under the floor of %d: the step was not run", free, o.DiskFloor)
	}
	traceFile, err := createTrace(o.Dir, action, step)
	if err != nil {
		return nil, err
	}
	sargs, err := sandbox.StraceArgs(traceFile)
	if err != nil {
		return nil, err
	}
	argv := append(sargs, kiln.Path, sandbox.ExecCommand, "--config", pre.ConfigPath,
		"--stdout-fd", "3", "--stderr-fd", "4", "--", "sh", "-c", line)

	// 5. Run it.
	lineSum := sha256.Sum256([]byte(line))
	sc := StepSidecar{
		Schema: StepSchema, Action: action, Step: step, StepCount: count, BindingID: pre.BindingID,
		LineSHA256: hex.EncodeToString(lineSum[:]), LineInPlan: planned.Steps[step] == line,
		Unclean: []string{},
	}
	sc.StartedAt = o.Now().UTC().Format(time.RFC3339Nano)
	run := runStrace(o, strace.Path, argv, []string{
		EnvAction + "=" + action, EnvStep + "=" + strconv.Itoa(step), EnvStepCount + "=" + strconv.Itoa(count),
		"PATH=" + pre.Tools.Path,
	}, traceFile)
	sc.EndedAt = o.Now().UTC().Format(time.RFC3339Nano)

	// 6. Account for it.
	sc.Started = run.started
	sc.StepNetNS = run.started // see StepSidecar.StepNetNS
	sc.StraceExit, sc.StraceSignal = run.code, run.signal
	sc.RootStatus = run.code
	sc.Exit125 = run.code == ExitRefused
	sc.StraceStdoutEmpty, sc.StraceStderrEmpty = run.stdout.n == 0, run.stderr.n == 0
	sc.Truncated, sc.TruncatedReason = run.truncated != "", run.truncated
	switch {
	case !run.started:
		sc.Unclean = append(sc.Unclean, UncleanNotStarted)
	case run.signal != 0:
		sc.Unclean = append(sc.Unclean, UncleanSignalled)
	}
	if sc.Truncated {
		sc.Unclean = append(sc.Unclean, UncleanTruncated+":"+run.truncated)
	}
	if run.started && !sc.StraceStdoutEmpty {
		sc.Unclean = append(sc.Unclean, UncleanStdout)
	}
	if run.started && !sc.StraceStderrEmpty {
		sc.Unclean = append(sc.Unclean, UncleanStderr)
	}
	if run.pipesHeld {
		sc.Unclean = append(sc.Unclean, UncleanPipesHeld)
	}
	sc.Clean = len(sc.Unclean) == 0
	if sc.TraceBytes, sc.TraceSHA256, err = hashTrace(traceFile); err != nil {
		return nil, fmt.Errorf("the trace: %w", err)
	}
	data, err := EncodeStepSidecar(sc)
	if err != nil {
		return nil, err
	}
	if err := writeExcl(StepFile(o.Dir, action, step, "json"), data); err != nil {
		return nil, fmt.Errorf("the step's sidecar: %w", err)
	}
	if !run.started {
		return &StepResult{ExitCode: ExitRefused, Sidecar: sc}, nil
	}
	code := run.code
	if run.signal != 0 {
		code = 128 + run.signal
	}
	return &StepResult{ExitCode: code, Sidecar: sc}, nil
}

// checkStepEnv requires env to be build_tool_env plus the three KILN variables,
// no name twice, and returns the three. The comparison is of sets of NAME=value
// entries (which, with no name twice, is the multiset comparison of §4.8). An
// error names variables, never values.
func checkStepEnv(env, want []string) (map[string]string, error) {
	got, err := envNames(env)
	if err != nil {
		return nil, fmt.Errorf("the environment: %w", err)
	}
	vars := map[string]string{}
	for _, name := range []string{EnvAction, EnvStep, EnvStepCount} {
		e, ok := got[name]
		if !ok {
			return nil, fmt.Errorf("the environment has no %s", name)
		}
		vars[name] = e[len(name)+1:]
		delete(got, name)
	}
	wantNames, err := envNames(want)
	if err != nil {
		return nil, fmt.Errorf("build_tool_env: %w", err)
	}
	var bad []string
	for name, e := range got {
		if w, ok := wantNames[name]; !ok {
			bad = append(bad, "extra "+name)
		} else if w != e {
			bad = append(bad, "different "+name)
		}
	}
	for name := range wantNames {
		if _, ok := got[name]; !ok {
			bad = append(bad, "missing "+name)
		}
	}
	if len(bad) > 0 {
		sort.Strings(bad)
		if len(bad) > 8 {
			bad = append(bad[:8], "...")
		}
		return nil, fmt.Errorf("the environment is not the build tool's recorded environment: %s", strings.Join(bad, ", "))
	}
	return vars, nil
}

// canonicalInt reads a decimal integer with no sign, no leading zero and at
// most six digits: the only spelling the build tool writes.
func canonicalInt(what, s string) (int, error) {
	if s == "" || len(s) > 6 || (len(s) > 1 && s[0] == '0') {
		return 0, fmt.Errorf("%s %q is not a canonical decimal integer", what, printable(s))
	}
	n := 0
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return 0, fmt.Errorf("%s %q is not a canonical decimal integer", what, printable(s))
		}
		n = n*10 + int(s[i]-'0')
	}
	return n, nil
}

// checkStepPlace validates KILN_ACTION, KILN_STEP and KILN_STEP_COUNT against
// the plan and against what has already run.
func checkStepPlace(pre *Pre, vars map[string]string) (action string, step, count int, err error) {
	action = vars[EnvAction]
	planned, ok := pre.Action(action)
	if !ok {
		return "", 0, 0, fmt.Errorf("KILN_ACTION %q is not an action of this shard's plan", printable(action))
	}
	if step, err = canonicalInt(EnvStep, vars[EnvStep]); err != nil {
		return "", 0, 0, err
	}
	if count, err = canonicalInt(EnvStepCount, vars[EnvStepCount]); err != nil {
		return "", 0, 0, err
	}
	if count != len(planned.Steps) {
		return "", 0, 0, fmt.Errorf("KILN_STEP_COUNT is %d, but the plan has %d steps for %q", count, len(planned.Steps), printable(action))
	}
	if step >= count {
		return "", 0, 0, fmt.Errorf("KILN_STEP %d is out of range (%d steps)", step, count)
	}
	if err := checkOrder(pre.Dir, action, step); err != nil {
		return "", 0, 0, err
	}
	return action, step, count, nil
}

// checkOrder requires the steps before this one to have finished (their
// sidecars exist) and none of this step or a later one to have started. A step
// that is run a second time leaves a `.rerun` marker, which `collect` treats as
// a reason to void the shard: two runs of one step could not be told apart in a
// single trace.
func checkOrder(dir, action string, step int) error {
	for j := 0; j < step; j++ {
		st, err := os.Lstat(StepFile(dir, action, j, "json"))
		if err != nil || !st.Mode().IsRegular() {
			return fmt.Errorf("step %d of %q was asked to run before step %d finished", step, printable(action), j)
		}
	}
	entries, err := os.ReadDir(StepDir(dir, action))
	if errors.Is(err, os.ErrNotExist) {
		return nil
	}
	if err != nil {
		return fmt.Errorf("the traces of %q: %v", printable(action), err)
	}
	for _, e := range entries {
		n, ext, ok := strings.Cut(e.Name(), ".")
		if !ok {
			continue
		}
		j, err := canonicalInt("", n)
		if err != nil || j < step {
			continue
		}
		if j > step {
			return fmt.Errorf("step %d of %q was asked to run after step %d had started", step, printable(action), j)
		}
		if ext == "rerun" {
			// An earlier attempt to run it again was refused. The step itself
			// stays refused: the shard is void already.
			return fmt.Errorf("step %d of %q was already run twice", step, printable(action))
		}
		// This step has run (or started) already. Say so where `collect` will
		// see it; the marker may exist already, which changes nothing.
		if err := writeExcl(StepFile(dir, action, step, "rerun"), []byte{}); err != nil && !errors.Is(err, os.ErrExist) {
			return fmt.Errorf("step %d of %q has already run, and its marker could not be written: %v", step, printable(action), err)
		}
		return fmt.Errorf("step %d of %q has already run: a step runs once", step, printable(action))
	}
	return nil
}

// ensureDir makes a directory the supervisor owns: absent (then created 0700)
// or a real directory that group and others can not touch. A symlink is
// refused, so no component can be redirected.
func ensureDir(p string) error {
	st, err := os.Lstat(p)
	switch {
	case errors.Is(err, os.ErrNotExist):
		return os.Mkdir(p, 0o700)
	case err != nil:
		return err
	case st.Mode()&os.ModeSymlink != 0 || !st.IsDir():
		return fmt.Errorf("%q is not a real directory", printable(p))
	case st.Mode().Perm()&0o077 != 0:
		return fmt.Errorf("%q is accessible to group or others (mode %o)", printable(p), st.Mode().Perm())
	}
	return nil
}

// writeExcl creates p with O_EXCL (never overwrites, never follows a symlink
// in the last component) and writes data.
func writeExcl(p string, data []byte) error {
	f, err := os.OpenFile(p, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return err
	}
	if _, err := f.Write(data); err != nil {
		f.Close()
		return err
	}
	return f.Close()
}

// createTrace makes traces/<sha>/ and creates the step's trace file empty,
// O_EXCL. strace then opens it by name.
func createTrace(dir, action string, step int) (string, error) {
	for _, d := range []string{filepath.Join(dir, "traces"), StepDir(dir, action)} {
		if err := ensureDir(d); err != nil {
			return "", fmt.Errorf("traces: %w", err)
		}
	}
	p := StepFile(dir, action, step, "trace")
	if err := writeExcl(p, nil); err != nil {
		return "", fmt.Errorf("the trace file: %w", err)
	}
	return p, nil
}

// hashTrace measures the finished trace: its size and SHA-256, read through a
// non-following handle.
func hashTrace(p string) (int64, string, error) {
	f, err := openNoFollow(p)
	if err != nil {
		return 0, "", err
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil {
		return 0, "", err
	}
	if !st.Mode().IsRegular() {
		return 0, "", fmt.Errorf("not a regular file")
	}
	h := sha256.New()
	n, err := io.Copy(h, f)
	if err != nil {
		return 0, "", err
	}
	return n, hex.EncodeToString(h.Sum(nil)), nil
}

// capture keeps the first 4 KiB written to it and counts every byte.
type capture struct {
	b []byte
	n int64
}

func (c *capture) Write(p []byte) (int, error) {
	if room := 4096 - len(c.b); room > 0 {
		c.b = append(c.b, p[:min(room, len(p))]...)
	}
	c.n += int64(len(p))
	return len(p), nil
}

// straceRun is what happened to the child.
type straceRun struct {
	started        bool
	code, signal   int // code is -1 when signalled or never started
	stdout, stderr *capture
	truncated      string // "" or a Trunc* reason
	pipesHeld      bool
}

// runStrace starts strace with the step's two output pipes as descriptors 3
// and 4, relays them, polls the trace, and waits.
func runStrace(o StepOptions, straceBin string, argv, env []string, traceFile string) straceRun {
	// The child's fork must come from a thread that lives as long as this
	// process, because Pdeathsig is tied to the forking thread.
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	run := straceRun{code: -1, stdout: &capture{}, stderr: &capture{}}
	outR, outW, err := os.Pipe()
	if err != nil {
		return run
	}
	errR, errW, err := os.Pipe()
	if err != nil {
		outR.Close()
		outW.Close()
		return run
	}
	cmd := exec.Command(straceBin, argv...)
	cmd.Env = env
	cmd.Stdin = nil // /dev/null
	cmd.Stdout, cmd.Stderr = run.stdout, run.stderr
	cmd.ExtraFiles = []*os.File{outW, errW} // descriptors 3 and 4 of strace
	cmd.SysProcAttr = childAttr()
	// A survivor that keeps strace's own pipes open must not hold us.
	cmd.WaitDelay = o.Drain
	if err := cmd.Start(); err != nil {
		outR.Close()
		outW.Close()
		errR.Close()
		errW.Close()
		return run
	}
	run.started = true
	// Our copies of the write ends must go, or the readers never see EOF.
	outW.Close()
	errW.Close()

	relay := func(dst io.Writer, src *os.File, done chan<- struct{}) {
		if _, err := io.Copy(dst, src); err != nil {
			io.Copy(io.Discard, src) // a broken log must not stall the step
		}
		done <- struct{}{}
	}
	done := make(chan struct{}, 2)
	go relay(o.Stdout, outR, done)
	go relay(o.Stderr, errR, done)

	var mu sync.Mutex
	stop := make(chan struct{})
	var watcher sync.WaitGroup
	watcher.Add(1)
	go func() {
		defer watcher.Done()
		tick := time.NewTicker(o.Poll)
		defer tick.Stop()
		for {
			select {
			case <-stop:
				return
			case <-tick.C:
			}
			reason := ""
			if st, err := os.Stat(traceFile); err == nil && st.Size() > o.TraceCap {
				reason = TruncTooLarge
			} else if free, err := o.FreeBytes(o.Dir); err != nil || free < o.DiskFloor {
				reason = TruncDiskFloor // an unmeasurable disk is a full disk
			}
			if reason != "" {
				mu.Lock()
				run.truncated = reason
				mu.Unlock()
				cmd.Process.Kill()
				return
			}
		}
	}()
	waitErr := cmd.Wait()
	close(stop)
	watcher.Wait()
	if errors.Is(waitErr, exec.ErrWaitDelay) {
		run.pipesHeld = true
	}
	// The step's own output: wait for it to drain, but only so long.
	timeout := time.NewTimer(o.Drain)
	defer timeout.Stop()
	for i := 0; i < 2; i++ {
		select {
		case <-done:
		case <-timeout.C:
			run.pipesHeld = true
			i = 2
		}
	}
	outR.Close()
	errR.Close()
	if ps := cmd.ProcessState; ps != nil {
		if ws, ok := ps.Sys().(syscall.WaitStatus); ok && ws.Signaled() {
			run.signal = int(ws.Signal())
		} else {
			run.code = ps.ExitCode()
		}
	}
	return run
}
