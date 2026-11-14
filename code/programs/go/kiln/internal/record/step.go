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
//	kiln record step                outside the sandbox, not traced itself;
//	   |                            holds a SHARED flock on the record directory
//	   |  verifies pre/, the environment, the step's place in the plan
//	   |  creates traces/<sha>/<n>.trace   (O_EXCL: a step runs once)
//	   v
//	strace -f ... -o /proc/self/fd/5 -- kiln sandbox-exec --config pre/sandbox.json
//	   |        |                           --config-sha256 H
//	   |        |                           --stdout-fd 3 --stderr-fd 4 -- sh -c <line>
//	   |        +-- fd 5: a pipe; the supervisor reads it and writes <n>.trace,
//	   |            hashing and counting every byte (an exact cap, no polling)
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
//	the disk is short, or `collect` holds the record directory
//
// Nothing is looked up on PATH: strace is the path in tools.json, the sandbox
// helper is this very binary, and the child's PATH is the recorded one.
//
// Serial recording and classification (decision P13)
// --------------------------------------------------
// The supervisor's sidecar holds the facts the trace cannot (how strace itself
// fared, how big the trace was allowed to grow). Since P13 it ALSO classifies,
// because `traced.Classify` resolves paths against the disk and a later action
// can change what an earlier one resolved through (classify_step.go has the
// picture). Two rules make that sound:
//
//	serial     the build runs with -jobs 1, and `step` holds an EXCLUSIVE,
//	           non-blocking flock on traces/.serial from before its checks until
//	           it exits. A step that cannot take it leaves <step>.overlap, runs
//	           nothing and exits 125. Under the lock each step is numbered from
//	           traces/.seq (the sidecar's `seq`), so `collect` can see a gap.
//	at the end the step that ends an action (its last step, or one that did not
//	           start or exited non-zero) classifies the action BEFORE the lock is
//	           released, and writes traces/<sha>/record.json (or record.failed).
//	           The step's exit status is never changed by it.
//
// Lock order, always: the shared lock on the record directory, then the serial
// lock; released in the opposite order. Both are taken without waiting, so a
// cycle cannot become a deadlock (and `collect` takes only the directory lock).
//
// Trace size and the disk (G8)
// ----------------------------
// A trace of a real step is large (the compiler's every `openat`), and an
// rlimit on strace would be inherited by the step, which would then die of
// SIGXFSZ when a BUILD writes a big file. So the trace goes to ordinary disk.
// strace does not write the file itself: it writes to a PIPE (`-o
// /proc/self/fd/5`, strace's own descriptor 5, which `strace -o` opens by path
// like any file; opening a pipe through /proc gives the write end, and
// O_TRUNC means nothing to a pipe), and the supervisor reads the pipe and
// writes the file. That gives three things a polled file size could not:
//
//	an exact cap          the supervisor counts the bytes; at the cap it keeps the
//	                      bytes up to it, marks the step `trace-too-large`, kills
//	                      strace (`--kill-on-exit` makes the kernel kill every
//	                      tracee with it) and keeps draining so nothing blocks
//	a failed write        ENOSPC or EIO from the file marks `trace-write-failed`,
//	                      kills strace and keeps draining; a poll of free space
//	                      could miss a fast fallocate-and-unlink, a write error
//	                      cannot be missed
//	the digest            the SHA-256 is of exactly the bytes written, as they
//	                      were written, with no second pass over a gigabyte
//
// Only the check BEFORE the step starts (free space at least the floor) stays a
// measurement.
//
// Descriptors. Of strace's descriptors 3, 4 and 5 (ExtraFiles), the root (`kiln
// sandbox-exec`, a trusted program) inherits all and the init closes every
// descriptor from 3 up before it starts the step, so the step holds none of
// them. strace's own standard output and error are pipes too, drained by the
// same logic, so a survivor that holds ANY of the five pipes open is noticed
// (`pipesHeld`) whatever strace's exit status was.
//
// Death of the supervisor. If the supervisor is killed, strace must not outlive
// it: the child is started with Pdeathsig = SIGKILL, from a goroutine pinned to
// its OS thread (Pdeathsig fires when the THREAD that forked the child exits;
// the Go runtime does not normally retire threads, but pinning makes the
// thread's lifetime one we know, not one we hope for). And an orphaned supervisor (its build tool killed) must not go on
// writing into the record directory while `collect` reads it: `step` holds a
// shared flock on the directory for its whole run, `collect` takes it
// exclusively (and refuses if it cannot), and the command asks the kernel to
// kill the supervisor when the build tool dies (recordcmd.go).

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"hash"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// The default limits (design review G8).
const (
	DefaultTraceCap  int64  = 1 << 30 // a trace above 1 GiB is cut
	DefaultDiskFloor uint64 = 4 << 30 // free space under 4 GiB stops a step from starting
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

// DefaultDrain bounds how long the supervisor waits for the step's output and
// the trace to drain once strace is gone. A survivor that holds a pipe open must
// not hold the supervisor, and with it the build, forever.
const DefaultDrain = 10 * time.Second

// traceFD is the descriptor number strace sees the trace pipe as (ExtraFiles
// index 2: after the step's stdout and stderr pipes, descriptors 3 and 4).
const traceFD = 5

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

	// Limits; zero means the default. DiskFloor is a field because a heavy
	// build may need a smaller floor and a separate cap; the default is
	// unchanged (4 GiB), and the floor only decides whether a step may START.
	TraceCap  int64
	DiskFloor uint64
	// Drain bounds the wait for the step's output once strace has exited.
	Drain time.Duration

	// The epilogue's limits (decision P13); zero means the default. The
	// classification budget is 10 minutes and the action's traces together may
	// not exceed 2 GiB; both are fields so a test can make them small.
	ClassifyTimeout time.Duration
	MaxTraceTotal   int64
	// ChildSoftMemory and ChildDataLimit bound the classification child
	// (GOMEMLIMIT and RLIMIT_DATA; classify_child.go); zero means the default.
	ChildSoftMemory, ChildDataLimit int64
	// MaxRecordBytes bounds the record.json the supervisor reads back (zero:
	// DefaultMaxRecordBytes).
	MaxRecordBytes int64
	// ChildCmd builds the child's command from its arguments and environment;
	// the default runs this very binary (`kiln record classify-action`). A test
	// replaces it to run its own test binary as the child.
	ChildCmd func(args, env []string) *exec.Cmd

	// Seams, for tests. Each defaults to the real thing.
	FreeBytes func(dir string) (uint64, error)
	Now       func() time.Time
	// WrapTrace wraps the writer the trace is written through (the file), so a
	// test can make a write fail the way a full disk does.
	WrapTrace func(io.Writer) io.Writer
}

// StepResult is what RunStep reports when the step was supervised.
type StepResult struct {
	// ExitCode is what the supervisor exits with: the traced root's status.
	ExitCode int
	Sidecar  StepSidecar
	// Record is what the classification epilogue left (P13): RecordWritten,
	// RecordFailed (then RecordWhy says why), or "" when the step did not end
	// its action. It never changes ExitCode.
	Record    string
	RecordWhy string
}

// applyDefaults fills every zero limit and seam with its default.
func (o *StepOptions) applyDefaults() {
	if o.TraceCap == 0 {
		o.TraceCap = DefaultTraceCap
	}
	if o.DiskFloor == 0 {
		o.DiskFloor = DefaultDiskFloor
	}
	if o.Drain == 0 {
		o.Drain = DefaultDrain
	}
	if o.ClassifyTimeout == 0 {
		o.ClassifyTimeout = DefaultClassifyTimeout
	}
	if o.MaxTraceTotal == 0 {
		o.MaxTraceTotal = DefaultMaxTraceTotal
	}
	if o.MaxRecordBytes == 0 {
		o.MaxRecordBytes = DefaultMaxRecordBytes
	}
	if o.ChildSoftMemory == 0 {
		o.ChildSoftMemory = DefaultChildSoftMemory
	}
	if o.ChildDataLimit == 0 {
		o.ChildDataLimit = DefaultChildDataLimit
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
}

// RunStep supervises one step. An error means nothing ran (or the sidecar could
// not be written); the caller exits ExitRefused.
func RunStep(o StepOptions) (*StepResult, error) {
	o.applyDefaults()

	// 1. The shared lock first, before anything of the record directory is
	// read: `collect` takes the same lock exclusively, so it cannot start (or
	// have started and be reading) while a step, an orphaned supervisor
	// included, is working in the directory. The lock is held for the whole run.
	if _, err := Canonical("the record directory", o.Dir); err != nil {
		return nil, err
	}
	unlock, err := LockDir(o.Dir, false)
	if err != nil {
		return nil, err
	}
	defer unlock()
	// 1a. A directory that `collect` has read takes no more steps: nothing is run,
	// no sequence number is taken, no marker is written (a marker would be a
	// write into a directory somebody has already read). `collect` leaves the
	// tombstone before it reads anything, after the exclusive lock this step's
	// shared one excludes; a step that got the shared lock a moment after
	// `collect` let go of the exclusive one lands here.
	//
	// Only "it is not there" lets the step go on. Any other answer (an unreadable
	// traces/, an I/O error) is a refusal: a check that cannot be made is not a
	// check that passed.
	switch _, err := os.Lstat(filepath.Join(o.Dir, "traces", CollectedMarker)); {
	case err == nil:
		return nil, fmt.Errorf("the record directory was collected (traces/%s): no step may run in it", CollectedMarker)
	case !errors.Is(err, os.ErrNotExist) && !errors.Is(err, syscall.ENOTDIR):
		// (ENOTDIR, traces/ is not a directory at all, is left to the serial lock
		// below, whose refusal names exactly that; the step still does not run.)
		return nil, fmt.Errorf("whether the record directory was collected cannot be checked: %v", err)
	}
	// 1b. The serial lock (P13), second in the lock order. A step that cannot
	// take it runs nothing, and says so in a marker `collect` reads.
	serial, err := LockSerial(o.Dir)
	if err != nil {
		// Only a lock that is HELD says another step overlaps this one; any other
		// failure (a bad traces directory, a file that cannot be opened) is a
		// refusal with no claim about overlap.
		var busy *SerialBusyError
		if errors.As(err, &busy) {
			markOverlap(o.Dir, o.Environ)
		}
		return nil, err
	}
	defer serial.Close()
	// pre/ is what prepare sealed, and the programs are the pinned ones.
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
	defer traceFile.Close()
	// The step's number in the serial order, taken only now that nothing can
	// refuse it any more: every step that RUNS has one, and a refused step takes
	// none, so a gap means two steps shared the lock's protection wrongly.
	if err := serial.OpenSeq(o.Dir); err != nil {
		return nil, err
	}
	seq, err := serial.NextSeq()
	if err != nil {
		return nil, err
	}
	// strace's -o names its OWN descriptor, a pipe (see the header).
	sargs, err := sandbox.StraceArgs("/proc/self/fd/" + strconv.Itoa(traceFD))
	if err != nil {
		return nil, err
	}
	argv := append(sargs, kiln.Path, sandbox.ExecCommand, "--config", pre.ConfigPath,
		"--config-sha256", pre.Binding.SandboxConfigSHA256,
		"--stdout-fd", "3", "--stderr-fd", "4", "--", "sh", "-c", line)
	env := []string{
		EnvAction + "=" + action, EnvStep + "=" + strconv.Itoa(step), EnvStepCount + "=" + strconv.Itoa(count),
		"PATH=" + pre.Tools.Path,
	}
	// A pinned strace that needs the validated LD_LIBRARY_PATH (the one
	// prepare checked and gave the build tool) must find it again, or every
	// step would fail to start. Nothing else of build_tool_env is passed on.
	if names, err := envNames(pre.Tools.BuildToolEnv); err == nil {
		if e, ok := names["LD_LIBRARY_PATH"]; ok {
			env = append(env, e)
		}
	}

	// 5. Run it.
	lineSum := sha256.Sum256([]byte(line))
	sc := StepSidecar{
		Schema: StepSchema, Action: action, Step: step, StepCount: count, Seq: seq, BindingID: pre.BindingID,
		LineSHA256: hex.EncodeToString(lineSum[:]), LineInPlan: planned.Steps[step] == line,
		Unclean: []string{},
	}
	sc.StartedAt = o.Now().UTC().Format(time.RFC3339Nano)
	var tw io.Writer = traceFile
	if o.WrapTrace != nil {
		tw = o.WrapTrace(tw)
	}
	sink := &traceSink{w: tw, h: sha256.New(), limit: o.TraceCap}
	run := runStrace(o, strace.Path, argv, env, sink)
	sc.EndedAt = o.Now().UTC().Format(time.RFC3339Nano)
	// A write that failed only now (the file's close or flush) is a failed
	// write too.
	if err := traceFile.Sync(); err != nil && sink.reason == "" {
		sink.reason = TruncWriteFailed
	}
	if err := traceFile.Close(); err != nil && sink.reason == "" {
		sink.reason = TruncWriteFailed
	}

	// 6. Account for it.
	sc.Started = run.started
	sc.StepNetNS = run.started // see StepSidecar.StepNetNS
	sc.StraceExit, sc.StraceSignal = run.code, run.signal
	sc.RootStatus = run.code
	sc.Exit125 = run.code == ExitRefused
	sc.StraceStdoutEmpty, sc.StraceStderrEmpty = run.stdout.n == 0, run.stderr.n == 0
	sc.Truncated, sc.TruncatedReason = sink.reason != "", sink.reason
	switch {
	case !run.started:
		sc.Unclean = append(sc.Unclean, UncleanNotStarted)
	case run.signal != 0:
		sc.Unclean = append(sc.Unclean, UncleanSignalled)
	}
	if sc.Truncated {
		sc.Unclean = append(sc.Unclean, UncleanTruncated+":"+sink.reason)
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
	sc.TraceBytes, sc.TraceSHA256 = sink.n, hex.EncodeToString(sink.h.Sum(nil))
	// What strace said about itself, for whoever has to find out why a CI run
	// failed. A diagnostic only: failing to save it changes nothing.
	if run.stderr.n > 0 {
		writeExcl(StepFile(o.Dir, action, step, "strace-stderr"), run.stderr.b)
	}
	data, err := EncodeStepSidecar(sc)
	if err != nil {
		return nil, err
	}
	if err := writeExcl(StepFile(o.Dir, action, step, "json"), data); err != nil {
		return nil, fmt.Errorf("the step's sidecar: %w", err)
	}
	code := ExitRefused
	if run.started {
		code = run.code
		if run.signal != 0 {
			code = 128 + run.signal
		}
	}
	res := &StepResult{ExitCode: code, Sidecar: sc}
	// 7. The epilogue (P13): this step ends its action when it is the last, or
	// did not start, or failed (the build tool stops an action at its first
	// failing step). It runs under the serial lock, which is released only when
	// RunStep returns, and never changes the status.
	if step == count-1 || code != 0 || !run.started {
		res.Record, res.RecordWhy = classifyEpilogue(o, pre, kiln.Path, action, step)
	}
	return res, nil
}

// markOverlap leaves `<step>.overlap` for a step the serial lock refused, when
// the step can be named from its environment (an action that is a plain path
// and a canonical step number). Best effort: the step is refused either way,
// and the action then lacks the step, which `collect` sees too.
func markOverlap(dir string, environ []string) {
	names, err := envNames(environ)
	if err != nil {
		return
	}
	action, ok1 := names[EnvAction]
	stepS, ok2 := names[EnvStep]
	if !ok1 || !ok2 {
		return
	}
	action, stepS = action[len(EnvAction)+1:], stepS[len(EnvStep)+1:]
	step, err := canonicalInt(EnvStep, stepS)
	if err != nil || recfmt.CheckPath(action) != nil {
		return
	}
	for _, d := range []string{filepath.Join(dir, "traces"), StepDir(dir, action)} {
		if ensureDir(d) != nil {
			return
		}
	}
	writeExcl(StepFile(dir, action, step, "overlap"), []byte{})
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

// markRerun leaves the `<step>.rerun` marker `collect` reads. It may exist
// already, which changes nothing; any other failure is reported.
func markRerun(dir, action string, step int) error {
	if err := writeExcl(StepFile(dir, action, step, "rerun"), []byte{}); err != nil && !errors.Is(err, os.ErrExist) {
		return err
	}
	return nil
}

// checkOrder requires the steps before this one to have finished (their
// sidecars exist) and none of this step or a later one to have started. EVERY
// entry of the action's directory is looked at, and step numbers are compared
// as numbers ("10.json" sorts before "2.json"), so no early return can skip the
// marker: a step that is run a second time, or out of order, leaves a `.rerun`
// marker for `collect` to make the action an unknown write set on (P12). Two supervisors of the same step
// that race are sorted out by the O_EXCL of the trace file (createTrace), which
// leaves the marker too.
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
	var own, ownOther, later int // entries of this step, those that are not a marker, later steps' entries
	for _, e := range entries {
		n, ext, ok := strings.Cut(e.Name(), ".")
		if !ok || ext == "overlap" {
			// An overlap marker says a step was REFUSED the serial lock and ran
			// nothing: it is not evidence that the step (or a later one) ran.
			continue
		}
		j, err := canonicalInt("", n)
		switch {
		case err != nil || j < step:
		case j > step:
			later++
		default:
			own++
			if ext != "rerun" {
				ownOther++
			}
		}
	}
	switch {
	case own > 0:
		if err := markRerun(dir, action, step); err != nil {
			return fmt.Errorf("step %d of %q has already run, and its marker could not be written: %v", step, printable(action), err)
		}
		if ownOther == 0 {
			// Only a marker: an earlier attempt to run it again was refused.
			return fmt.Errorf("step %d of %q was already run twice", step, printable(action))
		}
		return fmt.Errorf("step %d of %q has already run: a step runs once", step, printable(action))
	case later > 0:
		if err := markRerun(dir, action, step); err != nil {
			return fmt.Errorf("step %d of %q is out of order, and its marker could not be written: %v", step, printable(action), err)
		}
		return fmt.Errorf("step %d of %q was asked to run after a later step had started", step, printable(action))
	}
	// An action that has been classified has ended: a step that runs now would
	// leave a record that is for fewer steps than ran. Nothing is run.
	for _, f := range []string{RecordFile(dir, action), RecordFailedFile(dir, action)} {
		if _, err := os.Lstat(f); err == nil {
			return fmt.Errorf("%q has already been classified: a step cannot run after its action ended", printable(action))
		}
	}
	return nil
}

// ensureDir makes a directory the supervisor owns: created 0700, or already a
// real directory, owned by this user, that group and others can not touch. It
// is safe against another supervisor making the same directory at the same
// moment (the build tool runs independent packages in parallel): it tries to
// create first and, when someone else got there, falls through to the checks
// instead of failing. A symlink is refused, so no component can be redirected.
func ensureDir(p string) error {
	err := os.Mkdir(p, 0o700)
	if err == nil {
		return nil
	}
	if !errors.Is(err, os.ErrExist) {
		return err
	}
	st, err := os.Lstat(p)
	switch {
	case err != nil:
		return err
	case st.Mode()&os.ModeSymlink != 0 || !st.IsDir():
		return fmt.Errorf("%q is not a real directory", printable(p))
	case st.Mode().Perm()&0o077 != 0:
		return fmt.Errorf("%q is accessible to group or others (mode %o)", printable(p), st.Mode().Perm())
	}
	if uid, ok := ownerOf(st); ok && uid != uint32(os.Geteuid()) {
		return fmt.Errorf("%q is owned by user %d, not by this user", printable(p), uid)
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

// createTrace makes traces/<sha>/ (if `prepare` did not already make traces/)
// and creates the step's trace file empty, O_EXCL, returning it open for
// writing. A trace that exists means the step ran, or is running: leave the
// marker and refuse.
func createTrace(dir, action string, step int) (*os.File, error) {
	for _, d := range []string{filepath.Join(dir, "traces"), StepDir(dir, action)} {
		if err := ensureDir(d); err != nil {
			return nil, fmt.Errorf("traces: %w", err)
		}
	}
	f, err := os.OpenFile(StepFile(dir, action, step, "trace"), os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if errors.Is(err, os.ErrExist) {
		if merr := markRerun(dir, action, step); merr != nil {
			return nil, fmt.Errorf("step %d of %q has already started, and its marker could not be written: %v", step, printable(action), merr)
		}
		return nil, fmt.Errorf("the trace file: step %d of %q has already started: a step runs once: %w", step, printable(action), err)
	}
	if err != nil {
		return nil, fmt.Errorf("the trace file: %w", err)
	}
	return f, nil
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

// traceSink receives the trace from strace's pipe and writes the file.
type traceSink struct {
	w     io.Writer
	h     hash.Hash
	limit int64
	n     int64 // bytes written, all hashed
	// reason is "" or a Trunc* reason; kill stops strace. Both are set by the
	// goroutine that copies, and read after it has finished.
	reason string
	kill   func()
	dead   bool // the cap was reached or a write failed: drain and discard
}

// copy reads r to the end. At the cap it keeps the bytes up to it and stops
// writing; on a write error it stops writing; either way it kills strace once
// and keeps reading, so the pipe never fills and strace never blocks.
func (s *traceSink) copy(r io.Reader) {
	buf := make([]byte, 64<<10)
	for {
		n, err := r.Read(buf)
		if n > 0 && !s.dead {
			chunk := buf[:n]
			if room := s.limit - s.n; int64(len(chunk)) > room {
				chunk = chunk[:room]
				s.stop(TruncTooLarge)
			}
			if len(chunk) > 0 {
				wn, werr := s.w.Write(chunk)
				if wn > 0 {
					s.h.Write(chunk[:wn])
					s.n += int64(wn)
				}
				if werr != nil || wn < len(chunk) {
					s.stop(TruncWriteFailed)
				}
			}
		}
		if err != nil {
			return
		}
	}
}

func (s *traceSink) stop(reason string) {
	if s.dead {
		return
	}
	s.dead, s.reason = true, reason
	if s.kill != nil {
		s.kill()
	}
}

// straceRun is what happened to the child.
type straceRun struct {
	started        bool
	code, signal   int // code is -1 when signalled or never started
	stdout, stderr *capture
	pipesHeld      bool
}

// runStrace starts strace with the step's two output pipes and the trace pipe
// as descriptors 3, 4 and 5, relays and records them, and waits.
func runStrace(o StepOptions, straceBin string, argv, env []string, sink *traceSink) straceRun {
	// The child's fork must come from a thread that lives as long as this
	// process, because Pdeathsig is tied to the forking thread.
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	run := straceRun{code: -1, stdout: &capture{}, stderr: &capture{}}
	// Five pipes: the step's stdout and stderr, the trace, and strace's own
	// stdout and stderr. rs are our read ends, ws the child's write ends.
	var rs, ws [5]*os.File
	closeAll := func() {
		for i := range rs {
			if rs[i] != nil {
				rs[i].Close()
			}
			if ws[i] != nil {
				ws[i].Close()
			}
		}
	}
	for i := range rs {
		r, w, err := os.Pipe()
		if err != nil {
			closeAll()
			return run
		}
		rs[i], ws[i] = r, w
	}
	cmd := exec.Command(straceBin, argv...)
	cmd.Env = env
	cmd.Stdin = nil // /dev/null
	cmd.Stdout, cmd.Stderr = ws[3], ws[4]
	cmd.ExtraFiles = []*os.File{ws[0], ws[1], ws[2]} // descriptors 3, 4 and 5 of strace
	cmd.SysProcAttr = childAttr()
	if err := cmd.Start(); err != nil {
		closeAll()
		return run
	}
	run.started = true
	// Our copies of the write ends must go, or the readers never see EOF.
	for _, w := range ws {
		w.Close()
	}
	sink.kill = func() { cmd.Process.Kill() }

	relay := func(dst io.Writer, src *os.File, done chan<- struct{}) {
		if _, err := io.Copy(dst, src); err != nil {
			io.Copy(io.Discard, src) // a broken log must not stall the step
		}
		done <- struct{}{}
	}
	// Two kinds of reader. The ones that only fill our own memory or file (the
	// trace sink and strace's own two streams) can not block once their pipe is
	// closed, and what they gathered goes into the sidecar, so they are always
	// waited for. The two that write to the caller's stdout and stderr CAN block
	// (a full pipe nobody reads) and tell us nothing the sidecar needs, so after
	// the drain timeout they get a short grace and are then abandoned.
	own := make(chan struct{}, 3)
	relays := make(chan struct{}, 2)
	go relay(o.Stdout, rs[0], relays)
	go relay(o.Stderr, rs[1], relays)
	go func() { sink.copy(rs[2]); own <- struct{}{} }()
	go relay(run.stdout, rs[3], own)
	go relay(run.stderr, rs[4], own)

	cmd.Wait()
	// Everything the step and strace wrote: wait for it to drain, but only so
	// long. A pipe that is still open after strace is gone is held by something
	// that outlived it, which is not a clean run whatever the exit status.
	timeout := time.NewTimer(o.Drain)
	defer timeout.Stop()
	nOwn, nRelay := 3, 2
	for nOwn > 0 || nRelay > 0 {
		select {
		case <-own:
			nOwn--
		case <-relays:
			nRelay--
		case <-timeout.C:
			run.pipesHeld = true
			for _, r := range rs {
				r.Close() // the readers end with a read error
			}
			for nOwn > 0 {
				<-own // the sidecar's fields are read only after the sink has finished
				nOwn--
			}
			grace := time.NewTimer(time.Second)
			for nRelay > 0 {
				select {
				case <-relays:
					nRelay--
				case <-grace.C:
					nRelay = 0 // abandoned: a relay stuck writing to the caller
				}
			}
			grace.Stop()
		}
	}
	for _, r := range rs {
		r.Close()
	}
	if ps := cmd.ProcessState; ps != nil {
		if wst, ok := ps.Sys().(syscall.WaitStatus); ok && wst.Signaled() {
			run.signal = int(wst.Signal())
		} else {
			run.code = ps.ExitCode()
		}
	}
	return run
}

// stepsMatch compares an action's planned lines with the steps export's.
func stepsMatch(a Action, st *legacy.Steps) error {
	lines, _ := st.Lines(a.Action)
	if len(lines) != len(a.Steps) {
		return fmt.Errorf("action %q plans %d steps, the sealed steps export has %d", printable(a.Action), len(a.Steps), len(lines))
	}
	for i := range lines {
		if lines[i] != a.Steps[i] {
			return fmt.Errorf("action %q step %d is not the line of the sealed steps export", printable(a.Action), i)
		}
	}
	return nil
}
