package record

// The classification child (KILN04 section 4.8, decision P13)
// ==========================================================
//
// `kiln record step` classifies the action it ended in a separate process, so
// that nothing the classification does can take the supervisor, or the step's
// exit status, with it:
//
//	kiln record step                      the supervisor, holds the locks
//	   |  at the action's end
//	   v
//	kiln record classify-action           the child: --dir D --action A --step N
//	      --data-limit B                    a fresh process, a minimal environment
//
// What goes wrong in a classifier that a goroutine cannot survive:
//
//	out of memory       a heavy build's trace is up to 1 GiB and 8 million events
//	                    (ClassifyLimits); the Go runtime aborts the WHOLE process
//	                    when it cannot get memory
//	a fatal runtime     "concurrent map writes", a stack overflow, a corrupted
//	error               heap: not recoverable by `recover`
//	a hang              a bug that never returns
//
// In the child each of those is just a process that did not exit 0, and the
// parent turns any such exit into `record.failed`.
//
// Memory bound. Two settings, both made for the Go runtime:
//
//	GOMEMLIMIT      soft, in the environment: the collector works harder as the
//	                heap nears it (DefaultChildSoftMemory, 5 GiB)
//	RLIMIT_DATA     hard, set by the child on itself (Linux): the kernel refuses
//	                private writable memory past it (DefaultChildDataLimit, 8
//	                GiB, or half of MemTotal if that is less, with the soft limit
//	                scaled to match; the child also sets oom_score_adj 1000 so the
//	                kernel's own OOM killer takes it first), and the runtime then aborts with "out of memory", which
//	                the parent reports. RLIMIT_AS is NOT used: the Go runtime
//	                reserves large ranges of ADDRESS SPACE it never touches (heap
//	                arenas, per-thread stacks), so an address-space limit fails a
//	                healthy program long before it uses the memory the limit is
//	                meant to bound, and RLIMIT_DATA counts only memory that can be
//	                written.
//
// Deadline. The parent kills the child at 10 minutes (StepOptions.ClassifyTimeout).
//
// Trust. The child is handed paths and numbers, nothing else: the record
// directory, the action's name, the step's number. It re-verifies `pre/` with
// VerifyPre, reads the step's sidecar from disk (it must be the sidecar of that
// step of that action), and re-reads every sealed file it uses through
// Pre.ReadSealed. It writes record.json itself, atomically, and the parent
// decodes it again before it believes it.
//
// The child inherits no descriptor of the supervisor's (every one it opens is
// close-on-exec), so it holds neither lock; it runs while the parent holds both.

import (
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"runtime"
	"runtime/debug"
	"strconv"
	"strings"
	"time"
)

// The child's memory bound (see above).
const (
	DefaultChildSoftMemory int64 = 5 << 30
	DefaultChildDataLimit  int64 = 8 << 30
)

// ChildCommand is the subcommand's name under `kiln record`.
const ChildCommand = "classify-action"

// childArgs renders the child's argument list (after the program name).
func childArgs(dir, action string, step int, dataLimit int64) []string {
	return []string{"record", ChildCommand, "--dir", dir, "--action", action, "--step", strconv.Itoa(step), "--data-limit", strconv.FormatInt(dataLimit, 10)}
}

// runClassifyChild starts the child and waits for it, killing it at the
// deadline. It returns "" when the child exited 0, and otherwise the reason.
func runClassifyChild(o StepOptions, pre *Pre, kilnPath, action string, step int) string {
	args := childArgs(pre.Dir, action, step, o.ChildDataLimit)
	env := []string{"GOMEMLIMIT=" + strconv.FormatInt(o.ChildSoftMemory, 10), "PATH=" + pre.Tools.Path}
	var cmd *exec.Cmd
	if o.ChildCmd != nil {
		cmd = o.ChildCmd(args, env)
	} else {
		cmd = exec.Command(childExe(kilnPath, runtime.GOOS), args...)
		cmd.Env = env
		cmd.Dir = string(os.PathSeparator) // never the checkout
	}
	var errText capture
	cmd.Stdin = nil
	cmd.Stdout, cmd.Stderr = io.Discard, &errText
	cmd.SysProcAttr = childAttr()
	cmd.WaitDelay = 5 * time.Second
	// Pdeathsig is tied to the thread that forks: keep this one for the start.
	runtime.LockOSThread()
	err := cmd.Start()
	runtime.UnlockOSThread()
	if err != nil {
		return "the classifier could not be started: " + err.Error()
	}
	done := make(chan error, 1)
	go func() { done <- cmd.Wait() }()
	timer := time.NewTimer(o.ClassifyTimeout)
	defer timer.Stop()
	select {
	case err = <-done:
	case <-timer.C:
		cmd.Process.Kill()
		<-done
		return fmt.Sprintf("timeout: the classifier ran longer than %s and was killed", o.ClassifyTimeout)
	}
	if err != nil {
		// A non-zero exit, a signal (the kernel's OOM killer, a crash), or an
		// I/O error on the pipes: all the same to the record.
		return fmt.Sprintf("the classifier failed: %v: %s", err, string(errText.b))
	}
	return ""
}

// childExe is the program the child runs. On Linux that is /proc/self/exe, the
// very image the supervisor is running (which VerifyPre has already shown to be
// the sealed Kiln), so a file swapped at the Kiln path after that check cannot be
// what runs; elsewhere, the path.
func childExe(kilnPath, goos string) string {
	if goos == "linux" {
		return "/proc/self/exe"
	}
	return kilnPath
}

// scaleLimits fits the memory bound to the machine: the hard data limit is the
// smaller of the configured one and half of MemTotal, the soft limit the same
// 5:8 share of it as the defaults. With MemTotal unknown (0) the configured
// limit stands and the soft limit is left to GOMEMLIMIT.
func scaleLimits(dataLimit, memTotal int64) (data, soft int64) {
	if memTotal <= 0 {
		return dataLimit, 0
	}
	data = dataLimit
	if half := memTotal / 2; half < data {
		data = half
	}
	return data, data / 8 * 5
}

// parseMemTotal reads MemTotal (in bytes) from the text of /proc/meminfo; 0 if
// it is not there.
func parseMemTotal(meminfo string) int64 {
	for _, line := range strings.Split(meminfo, "\n") {
		if rest, ok := strings.CutPrefix(line, "MemTotal:"); ok {
			f := strings.Fields(rest)
			if len(f) >= 1 {
				if kb, err := strconv.ParseInt(f[0], 10, 64); err == nil && kb > 0 {
					return kb << 10
				}
			}
		}
	}
	return 0
}

// ChildHooks are the child's seams, for tests; the zero value is the real thing.
type ChildHooks = ClassifyDeps

// ClassifyChild is the child's whole job (`kiln record classify-action`): verify
// pre/, read the step's sidecar, classify the action and write its record.json
// atomically. Any failure, a panic included, is an error; the caller exits
// non-zero, and the parent writes record.failed.
func ClassifyChild(dir, action string, step int, dataLimit int64, h ChildHooks) (err error) {
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("panic: %v", r)
		}
	}()
	if dataLimit > 0 {
		// Be the OOM killer's first choice (so a runaway classification dies
		// before the runner's other work does), and fit the bound to the machine.
		adjustOOM()
		var total int64
		if data, err := os.ReadFile("/proc/meminfo"); err == nil {
			total = parseMemTotal(string(data))
		}
		data, soft := scaleLimits(dataLimit, total)
		if soft > 0 {
			debug.SetMemoryLimit(soft)
		}
		if err := setDataLimit(uint64(data)); err != nil {
			return fmt.Errorf("memory limit: %w", err)
		}
	}
	pre, err := VerifyPre(dir)
	if err != nil {
		return err
	}
	planned, ok := pre.Action(action)
	if !ok {
		return fmt.Errorf("%q is not an action of this shard's plan", printable(action))
	}
	if step < 0 || step >= len(planned.Steps) {
		return fmt.Errorf("step %d is out of range (%d steps)", step, len(planned.Steps))
	}
	last, err := readSidecar(dir, action, step)
	if err != nil {
		return fmt.Errorf("the step's sidecar: %w", err)
	}
	// Guards of its own: it classifies only an action that has ended, once.
	if step != len(planned.Steps)-1 && last.Started && last.RootStatus == 0 {
		return fmt.Errorf("step %d is not the end of the action (it is not the last step, started, and exited 0)", step)
	}
	for _, f := range []string{RecordFile(dir, action), RecordFailedFile(dir, action)} {
		if _, err := os.Lstat(f); err == nil {
			return fmt.Errorf("classified-twice: %q already has a verdict", printable(action))
		}
	}
	rec, err := classifyAction(h, pre, action, step, last)
	if err != nil {
		return err
	}
	data, err := EncodeClassified(rec)
	if err != nil {
		return fmt.Errorf("encode: %w", err)
	}
	if err := writeAtomic(RecordFile(dir, action), data); err != nil {
		if errors.Is(err, os.ErrExist) {
			// A second classification of one action: what is on disk is for
			// fewer steps than ran.
			return fmt.Errorf("classified-twice: record.json already exists")
		}
		return fmt.Errorf("record.json: %w", err)
	}
	return nil
}
