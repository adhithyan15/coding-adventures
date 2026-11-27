//go:build linux && amd64

package main

// KILN04 D19, end to end: signals in a real strace trace of a real sandbox.
//
// strace prints "+++ killed by SIG +++" only for signals in its -e signal set,
// so the recorder runs it with signal=all; and the daemon-left rule keys on the
// init's exit_group call line.  Both rest on how strace and the kernel really
// behave, which no canned trace can show.  This file runs the shapes that
// matter: a process killed by SIGPIPE, one the step kills and reaps itself,
// and a survivor of the step.

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// stracedRun is what runStraced returns.
type stracedRun struct {
	f       *sandboxFixture
	tr      *trace.Trace
	refusal *trace.Error // set by runStracedOr when a tolerated refusal ended the parse
	stdout  string       // what the step wrote to its descriptor 1 (logged, never an error)
	stderr  string       // and to its descriptor 2
	data    string       // the data file the script may read
	trf     string       // the raw trace file
	// notifs is, for a run with --exec-notify-fd, the system-call numbers of the
	// exec notifications the harness's responder answered (KILN04 D48).
	notifs  []int32
	elapsed time.Duration // wall time of the whole strace run
}

// tail is the last n lines of the raw trace, each cut short, numbered.
func (r *stracedRun) tail(n int) string {
	raw, _ := os.ReadFile(r.trf)
	lines := strings.Split(strings.TrimRight(string(raw), "\n"), "\n")
	lo := len(lines) - n
	if lo < 0 {
		lo = 0
	}
	var b strings.Builder
	for i := lo; i < len(lines); i++ {
		l := lines[i]
		if len(l) > 300 {
			l = l[:300] + "..."
		}
		fmt.Fprintf(&b, "%5d  %s\n", i+1, l)
	}
	return b.String()
}

// runStraced runs `sh -c script` in the sandbox under the exact strace argv the
// recorder uses (sandbox.StraceArgs), and parses the trace in helper mode.
//
// "{data}" in the script is replaced with the path of a data file holding
// "hello\n".  Under -cover the test binary writes coverage files when the root
// exits, which are traced calls by the root after the spawn (helper-activity,
// rightly), so it traces an UNINSTRUMENTED kiln built into its own temporary
// directory and makes that both the strace target and the configured kiln_exe.
//
// A trace the parser refuses fails the test at once, with the lines around the
// refused one, each cut short, so the next fix needs no second CI run.
func runStraced(t *testing.T, script string) *stracedRun {
	t.Helper()
	return runStracedWith(t, script, nil)
}

// runStracedWith is runStraced with a hook that prepares the fixture (put a
// helper binary in the checkout, say) before the step runs.  "{pkg}" in the
// script is the checkout directory the step starts in.
func runStracedWith(t *testing.T, script string, setup func(f *sandboxFixture)) *stracedRun {
	t.Helper()
	return runStracedOr(t, script, setup, nil)
}

// runStracedOr is runStracedWith that lets the caller accept some refusals of
// the trace: when tolerate says yes to a refusal, the run is returned with
// refusal set and tr nil, for the caller to judge.  Any other refusal fails the
// test as before.
func runStracedOr(t *testing.T, script string, setup func(f *sandboxFixture), tolerate func(*trace.Error) bool) *stracedRun {
	t.Helper()
	return runStracedImpl(t, script, setup, tolerate, false)
}

// runStracedImpl is runStracedOr, optionally started the way `kiln record step`
// will start the sandbox once it supervises exec heads (KILN04 D48): a
// connected socket pair, the child end as strace's descriptor 5 (after the two
// output pipes, 3 and 4), `--exec-notify-fd 5` in the sandbox's arguments, and
// a responder on this side that answers every notification CONTINUE.
//
// strace itself has one known race on a dying thread (its own PTRACE_LISTEN call
// fails and it exits 1; see strace_retry_test.go).  When a run fails with exactly
// that, the whole step is run again from a fresh fixture, up to
// stracePtraceListenAttempts runs in all; every other failure, and the last race
// hit, fails the test as before.  Each test logs `STRACE_LISTEN_RETRIES=N` (0 when
// the race never hit) so CI can count how often it happens.
func runStracedImpl(t *testing.T, script string, setup func(f *sandboxFixture), tolerate func(*trace.Error) bool, notify bool) *stracedRun {
	t.Helper()
	retries := 0
	defer func() { t.Logf("STRACE_LISTEN_RETRIES=%d", retries) }()
	for attempt := 1; ; attempt++ {
		r, raced := runStracedAttempt(t, script, setup, tolerate, notify, attempt)
		if !raced {
			return r
		}
		retries++
	}
}

// runStracedAttempt is one run of runStracedImpl.  It reports raced (and a nil
// run) only when strace hit its PTRACE_LISTEN race and another attempt remains;
// the caller runs it again.  Everything else is judged here, as it always was.
func runStracedAttempt(t *testing.T, script string, setup func(f *sandboxFixture), tolerate func(*trace.Error) bool, notify bool, attempt int) (*stracedRun, bool) {
	t.Helper()
	straceBin, err := exec.LookPath("strace")
	if err != nil {
		skipOrFail(t, "strace is not installed")
	}
	f := newSandboxFixture(t)
	goBin, err := exec.LookPath("go")
	if err != nil {
		skipOrFail(t, "the go tool is not available to build an uninstrumented kiln")
	}
	kiln := filepath.Join(t.TempDir(), "kiln")
	build := exec.Command(goBin, "build", "-o", kiln, ".")
	if out, err := build.CombinedOutput(); err != nil {
		t.Fatalf("go build: %v\n%s", err, out)
	}
	if kiln, err = filepath.EvalSymlinks(kiln); err != nil {
		t.Fatal(err)
	}
	f.exe = kiln
	f.writeConfig(t, nil)
	data := filepath.Join(f.pkg, "data.txt")
	if err := os.WriteFile(data, []byte("hello\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	r := &stracedRun{f: f, data: data, trf: filepath.Join(f.base, "trace.txt")}
	// The exported argv, exactly what the recorder runs (KILN04 section 4.3).
	args, err := sandbox.StraceArgs(r.trf)
	if err != nil {
		t.Fatal(err)
	}
	if setup != nil {
		setup(f)
	}
	script = strings.ReplaceAll(script, "{data}", data)
	script = strings.ReplaceAll(script, "{pkg}", f.pkg)
	// The step's descriptors 1 and 2 are pipes the harness owns (fds 3 and 4 of
	// strace, as the `record step` supervisor does), so what the step prints
	// (a shell's "Killed" / "Terminated" job notice, "yes: Broken pipe") is
	// kept apart from strace's OWN standard error, which must stay empty.
	sandboxArgs := []string{f.exe, sandbox.ExecCommand, "--config", f.cfg, "--stdout-fd", "3", "--stderr-fd", "4"}
	if notify {
		sandboxArgs = append(sandboxArgs, "--exec-notify-fd", "5")
	}
	args = append(append(args, sandboxArgs...), "--", "sh", "-c", script)
	outR, outW, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	errR, errW, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, straceBin, args...)
	cmd.Dir = f.pkg
	cmd.Env = []string{"KILN_ACTION=pkg", "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"}
	cmd.ExtraFiles = []*os.File{outW, errW}
	var resp *execResponder
	var notifyChild *os.File
	if notify {
		resp, notifyChild = newExecResponder(t, respContinue)
		cmd.ExtraFiles = append(cmd.ExtraFiles, notifyChild)
	}
	var out, eb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &eb
	read := func(f *os.File, into *string) <-chan struct{} {
		done := make(chan struct{})
		go func() {
			b, _ := io.ReadAll(f)
			*into = string(b)
			close(done)
		}()
		return done
	}
	var stepOut, stepErr string
	start := time.Now()
	if err := cmd.Start(); err != nil {
		if resp != nil {
			notifyChild.Close()
			resp.stop()
		}
		outR.Close()
		outW.Close()
		errR.Close()
		errW.Close()
		t.Fatalf("starting strace: %v", err)
	}
	// Our copies of the write ends must go, or the readers never see EOF.
	outW.Close()
	errW.Close()
	if resp != nil {
		// The sandbox holds the only other copy of this end, so the responder
		// sees end of file once the init has closed its own.
		notifyChild.Close()
		resp.begin()
	}
	doneOut, doneErr := read(outR, &stepOut), read(errR, &stepErr)
	runErr := cmd.Wait()
	if resp != nil {
		resp.stop()
		r.notifs = resp.numbers()
	}
	<-doneOut
	<-doneErr
	outR.Close()
	errR.Close()
	r.elapsed = time.Since(start)
	r.stdout, r.stderr = stepOut, stepErr
	t.Logf("step stdout: %q\nstep stderr: %q", stepOut, stepErr)
	if runErr != nil {
		code := -1
		var ee *exec.ExitError
		if errors.As(runErr, &ee) {
			code = ee.ExitCode()
		}
		if stracePtraceListenRace(code, eb.Bytes()) {
			if straceShouldRetry(attempt, true) {
				// Our own synthetic fixture: strace's stderr is safe to print.
				t.Logf("strace hit its PTRACE_LISTEN race (attempt %d of %d), running the step again: %s",
					attempt, stracePtraceListenAttempts, strings.TrimSpace(eb.String()))
				return nil, true
			}
			t.Fatalf("strace hit its PTRACE_LISTEN race on all %d attempts: %v\nstrace stderr: %s\n--- trace tail ---\n%s",
				attempt, runErr, eb.String(), r.tail(25))
		}
		t.Fatalf("strace: %v\nstrace stderr: %s\n--- trace tail ---\n%s", runErr, eb.String(), r.tail(25))
	}
	if eb.Len() != 0 {
		t.Fatalf("strace wrote to standard error: %q\n--- trace tail ---\n%s", eb.String(), r.tail(25))
	}
	fh, err := os.Open(r.trf)
	if err != nil {
		t.Fatal(err)
	}
	defer fh.Close()
	probeTraceWall(t, "signals", r.trf, r.elapsed)
	r.tr, err = trace.ParseOptions(fh, f.pkg, trace.Options{HelperRoot: true, HelperExe: f.exe})
	if err != nil {
		raw, _ := os.ReadFile(r.trf)
		lines := strings.Split(string(raw), "\n")
		at := 0
		var te *trace.Error
		if errors.As(err, &te) {
			at = te.Line
			if tolerate != nil && tolerate(te) {
				r.tr, r.refusal = nil, te
				return r, false
			}
		}
		lo, hi := at-15, at+5
		if lo < 0 {
			lo = 0
		}
		if hi > len(lines) {
			hi = len(lines)
		}
		var win strings.Builder
		for i := lo; i < hi; i++ {
			l := lines[i]
			if len(l) > 400 {
				l = l[:400] + "..."
			}
			fmt.Fprintf(&win, "%5d  %s\n", i+1, l)
		}
		t.Fatalf("the trace was refused: %v (line %d of %d)\n--- around the refused line ---\n%s\n--- trace tail ---\n%s",
			err, at, len(lines), win.String(), r.tail(25))
	}
	return r, false
}

// exitOf finds the exit of the process with this signal, and its index in
// Trace.Exits (-1 when there is none).
func exitWithSignal(tr *trace.Trace, sig string) (trace.Exit, int) {
	for i, ex := range tr.Exits {
		if ex.Killed && ex.Signal == sig && ex.Pid != tr.InitPid {
			return ex, i
		}
	}
	return trace.Exit{}, -1
}

// TestSandboxStraceSignals runs the signal shapes of KILN04 D19 against real
// strace.  Every subtest logs the tail of its trace, pass or fail.
func TestSandboxStraceSignals(t *testing.T) {
	cases := []struct {
		name, script string
		check        func(t *testing.T, r *stracedRun)
	}{
		{"sigpipe", "yes | head -n 1 >/dev/null", func(t *testing.T, r *stracedRun) {
			// `yes` is killed by SIGPIPE once `head` has quit.  Unless SIGPIPE
			// reached the sandbox IGNORED: an ignored signal stays ignored
			// across exec, and no shell can reset it (POSIX: a signal ignored
			// on entry to a non-interactive shell cannot be trapped or reset,
			// so `trap - PIPE` would not help).  Then `yes` gets EPIPE and
			// exits non-zero instead.  Both are accepted; either way the
			// trace must have parsed, and the death or failure is logged.
			if _, i := exitWithSignal(r.tr, "SIGPIPE"); i < 0 {
				failed := false
				for _, ex := range r.tr.Exits {
					if ex.Pid != r.tr.InitPid && !ex.Killed && ex.Code != 0 {
						failed = true
					}
				}
				if !failed {
					t.Errorf("no exit killed by SIGPIPE, and none failed: %+v", r.tr.Exits)
				}
				t.Logf("SIGPIPE was apparently ignored: yes exited non-zero instead (exits %+v)", r.tr.Exits)
			}
			if traced.DaemonLeft(r.tr) {
				t.Error("DaemonLeft is true for a SIGPIPE death")
			}
		}},
		{"term-waited", "sleep 30 & p=$!; sleep 1; kill -TERM $p; wait $p; cat {data}", func(t *testing.T, r *stracedRun) {
			if _, i := exitWithSignal(r.tr, "SIGTERM"); i < 0 {
				t.Errorf("no exit killed by SIGTERM: %+v", r.tr.Exits)
			}
			if traced.DaemonLeft(r.tr) {
				t.Error("DaemonLeft is true for a reaped SIGTERM death")
			}
		}},
		{"kill9-waited", "sleep 30 & p=$!; sleep 1; kill -KILL $p; wait $p; cat {data}", func(t *testing.T, r *stracedRun) {
			_, i := exitWithSignal(r.tr, "SIGKILL")
			if i < 0 {
				t.Fatalf("no exit killed by SIGKILL: %+v", r.tr.Exits)
			}
			if traced.DaemonLeft(r.tr) {
				t.Error("DaemonLeft is true for a SIGKILL the step reaped itself")
			}
			if r.tr.InitExitLine == 0 || r.tr.ExitLines[i] >= r.tr.InitExitLine {
				t.Errorf("the reaped child's exit is at line %d, not before init's exit_group at %d", r.tr.ExitLines[i], r.tr.InitExitLine)
			}
		}},
		{"daemon-left", "sleep 300 & sleep 1; echo started", func(t *testing.T, r *stracedRun) {
			if !strings.Contains(r.stdout, "started") {
				t.Errorf("the step's stdout (fd 3) lacks %q: %q", "started", r.stdout)
			}
			if r.tr.InitExitLine == 0 {
				t.Error("InitExitLine is 0: the init's exit_group was not seen")
			}
			ex, i := exitWithSignal(r.tr, "SIGKILL")
			if i < 0 {
				t.Fatalf("no survivor killed by SIGKILL: %+v", r.tr.Exits)
			}
			// F2 of the design: is the survivor's line before or after the
			// init's own exit line?  Not asserted; the first CI run decides
			// what the spec says.
			where := "after"
			for j, e := range r.tr.Exits {
				if e.Pid == r.tr.InitPid && r.tr.ExitLines[i] < r.tr.ExitLines[j] {
					where = "BEFORE"
				}
			}
			t.Logf("F2: survivor pid %d exit line %d is %s init's exit line; init exit_group at line %d",
				ex.Pid, r.tr.ExitLines[i], where, r.tr.InitExitLine)
			if r.tr.InitExitLine > 0 && r.tr.ExitLines[i] <= r.tr.InitExitLine {
				t.Errorf("the survivor's exit (line %d) is not after init's exit_group (line %d)", r.tr.ExitLines[i], r.tr.InitExitLine)
			}
			if !traced.DaemonLeft(r.tr) {
				t.Error("DaemonLeft is false for a survivor")
			}
			if r.elapsed >= 30*time.Second {
				t.Errorf("the step took %v: the sleeper outlived the sandbox", r.elapsed)
			}
		}},
		{"setsid", "setsid sleep 300 >/dev/null 2>&1 & sleep 1; true", func(t *testing.T, r *stracedRun) {
			if !traced.DaemonLeft(r.tr) {
				t.Error("DaemonLeft is false for a setsid survivor")
			}
		}},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			r := runStraced(t, c.script)
			// The tail, always: on a failure it is the evidence, on a pass it
			// is what the spec's open questions are decided from.
			defer func() { t.Logf("trace tail (%s):\n%s", c.name, r.tail(25)) }()
			c.check(t, r)
		})
	}
}
