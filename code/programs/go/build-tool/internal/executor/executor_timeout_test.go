package executor

import (
	"context"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// Tests for the per-command timeout (CommandTimeout).
//
// Every test here runs a real shell command, because the thing under test is
// the interaction between a timer, a process tree and the pipes that carry its
// output; a fake would test none of that. The helpers pick a "sleep" that
// exists on the host: `sleep` on Unix, `ping -n` on Windows (the classic cmd
// idiom, since cmd has no sleep and `timeout` refuses redirected stdin).

// sleepCommand returns a command line that does nothing for about secs seconds.
func sleepCommand(secs int) string {
	if runtime.GOOS == "windows" {
		return "ping -n " + itoa(secs+1) + " 127.0.0.1 >NUL"
	}
	return "sleep " + itoa(secs)
}

func itoa(n int) string { return strconv.Itoa(n) }

// withCommandTimeout sets the package-level timeout (and a short WaitDelay)
// for one test and restores both afterwards.
func withCommandTimeout(t *testing.T, d time.Duration) {
	t.Helper()
	oldTimeout, oldDelay := CommandTimeout, commandWaitDelay
	CommandTimeout, commandWaitDelay = d, 500*time.Millisecond
	t.Cleanup(func() { CommandTimeout, commandWaitDelay = oldTimeout, oldDelay })
}

func timeoutPkg(t *testing.T, commands ...string) discovery.Package {
	t.Helper()
	root := makeFixture(t, map[string]string{"pkg/BUILD": "echo x"})
	return discovery.Package{
		Name:          "rust/lzw",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: commands,
		Language:      "rust",
	}
}

// A command that outlives the timeout fails the package promptly, and the
// message says which package, which step, what text, and how long.
func TestCommandTimeoutFailsPromptlyAndNamesCommand(t *testing.T) {
	withCommandTimeout(t, 300*time.Millisecond)
	slow := sleepCommand(60)
	pkg := timeoutPkg(t, slow, "echo should-not-run")

	began := time.Now()
	result := runPackageBuild(pkg, false)
	if took := time.Since(began); took > 20*time.Second {
		t.Fatalf("timeout did not fire promptly: took %v", took)
	}

	if result.Status != "failed" {
		t.Fatalf("a timeout must fail the package, got status %q", result.Status)
	}
	if result.ReturnCode != timeoutReturnCode {
		t.Errorf("return code = %d, want %d", result.ReturnCode, timeoutReturnCode)
	}
	if result.StepsRun != 1 || result.StepsTotal != 2 {
		t.Errorf("steps = %d/%d, want 1/2", result.StepsRun, result.StepsTotal)
	}
	for _, want := range []string{"rust/lzw", "command 1 of 2", "`" + slow + "`", "timed out", "300ms"} {
		if !strings.Contains(result.Stderr, want) {
			t.Errorf("stderr missing %q:\n%s", want, result.Stderr)
		}
	}
	if strings.Contains(result.Stdout, "should-not-run") {
		t.Errorf("command after the timeout ran: %q", result.Stdout)
	}
}

// Output the hung command itself printed before hanging is also kept.
func TestCommandTimeoutKeepsOutputOfTheHungCommand(t *testing.T) {
	withCommandTimeout(t, 400*time.Millisecond)
	cmd := "echo partial-output && " + sleepCommand(60)
	result := runPackageBuild(timeoutPkg(t, cmd), false)
	if result.Status != "failed" {
		t.Fatalf("status = %q, want failed", result.Status)
	}
	if !strings.Contains(result.Stdout, "partial-output") {
		t.Errorf("output of the hung command lost: %q", result.Stdout)
	}
}

// The WaitDelay case: a background grandchild inherits stdout and outlives the
// shell. A bare cmd.Run would block until the grandchild exits (60s here).
func TestCommandTimeoutBoundsGrandchildHoldingPipe(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("needs a POSIX shell to background a pipe-holding child")
	}
	withCommandTimeout(t, 5*time.Second)
	pkg := timeoutPkg(t, "sleep 60 & echo shell-done")

	began := time.Now()
	result := runPackageBuild(pkg, false)
	if took := time.Since(began); took > 20*time.Second {
		t.Fatalf("grandchild holding the pipe blocked the build for %v", took)
	}
	if result.Status != "failed" {
		t.Fatalf("status = %q, want failed (stderr: %s)", result.Status, result.Stderr)
	}
	if !strings.Contains(result.Stderr, "still holding its output open") {
		t.Errorf("stderr does not explain the lingering child:\n%s", result.Stderr)
	}
	if !strings.Contains(result.Stdout, "shell-done") {
		t.Errorf("output lost: %q", result.Stdout)
	}
}

// A grandchild in its OWN session escapes the process-group kill; the timeout
// must still return, via WaitDelay, instead of waiting for it.
func TestCommandTimeoutBoundsEscapedGrandchild(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("setsid is POSIX only")
	}
	if _, err := exec.LookPath("setsid"); err != nil {
		t.Skip("setsid not available")
	}
	withCommandTimeout(t, 300*time.Millisecond)
	pkg := timeoutPkg(t, "setsid sleep 60 & sleep 60")

	began := time.Now()
	result := runPackageBuild(pkg, false)
	if took := time.Since(began); took > 20*time.Second {
		t.Fatalf("escaped grandchild blocked the build for %v", took)
	}
	if result.Status != "failed" || result.ReturnCode != timeoutReturnCode {
		t.Fatalf("status/code = %q/%d, want failed/%d", result.Status, result.ReturnCode, timeoutReturnCode)
	}
}

// The kill reaches the whole group: a same-group grandchild must be dead after
// the timeout (we detect that by it never writing its marker file).
func TestCommandTimeoutKillsProcessTree(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("marker-file probe uses a POSIX shell")
	}
	withCommandTimeout(t, 400*time.Millisecond)
	pkg := timeoutPkg(t, "(sleep 2 && echo alive > survived.txt) & sleep 60")
	result := runPackageBuild(pkg, false)
	if result.Status != "failed" {
		t.Fatalf("status = %q, want failed", result.Status)
	}
	time.Sleep(2500 * time.Millisecond)
	if matches, _ := filepath.Glob(filepath.Join(pkg.Path, "survived.txt")); len(matches) != 0 {
		t.Fatalf("grandchild survived the timeout: %v", matches)
	}
}

// A command that finishes well inside the limit is untouched.
func TestCommandTimeoutLeavesNormalCommandAlone(t *testing.T) {
	withCommandTimeout(t, 30*time.Second)
	result := runPackageBuild(timeoutPkg(t, "echo fine", "echo also-fine"), false)
	if result.Status != "built" || result.ReturnCode != 0 {
		t.Fatalf("status/code = %q/%d (stderr: %s)", result.Status, result.ReturnCode, result.Stderr)
	}
	if !strings.Contains(result.Stdout, "fine") || result.StepsRun != 2 {
		t.Errorf("stdout=%q steps=%d", result.Stdout, result.StepsRun)
	}
}

// An ordinary nonzero exit is still an ordinary failure, not a "timeout".
func TestCommandTimeoutDoesNotRelabelOrdinaryFailure(t *testing.T) {
	withCommandTimeout(t, 30*time.Second)
	result := runPackageBuild(timeoutPkg(t, "exit 3"), false)
	if result.Status != "failed" || result.ReturnCode != 3 {
		t.Fatalf("status/code = %q/%d, want failed/3", result.Status, result.ReturnCode)
	}
	if strings.Contains(result.Stderr, "timed out") {
		t.Errorf("ordinary failure mislabelled: %s", result.Stderr)
	}
}

// Zero disables the timeout: a command longer than any tiny limit completes.
func TestCommandTimeoutZeroDisables(t *testing.T) {
	withCommandTimeout(t, 0)
	result := runPackageBuild(timeoutPkg(t, sleepCommand(2)), false)
	if result.Status != "built" {
		t.Fatalf("with timeout 0 the command must run to completion, got %q (stderr: %s)", result.Status, result.Stderr)
	}
}

// The default is what main installs; the executor's own zero value is "off"
// so every existing caller and test keeps its old behaviour.
func TestDefaultCommandTimeoutValue(t *testing.T) {
	if DefaultCommandTimeout != 90*time.Minute {
		t.Errorf("DefaultCommandTimeout = %v, want 90m", DefaultCommandTimeout)
	}
	if CommandTimeout != 0 {
		t.Errorf("executor package default must be off, got %v", CommandTimeout)
	}
}

func TestFormatTimeoutMessage(t *testing.T) {
	msg := formatTimeoutMessage("go/x", 3, 5, "make test", 90*time.Minute, 5432*time.Millisecond)
	for _, want := range []string{"go/x", "command 3 of 5", "make test", "1h30m0s", "5.4s"} {
		if !strings.Contains(msg, want) {
			t.Errorf("message %q missing %q", msg, want)
		}
	}
}

// withRootContext installs a cancellable root context (what main wires to
// SIGINT/SIGTERM) for one test.
func withRootContext(t *testing.T) context.CancelFunc {
	t.Helper()
	ctx, cancel := context.WithCancel(context.Background())
	old := RootContext
	RootContext = ctx
	t.Cleanup(func() { RootContext = old; cancel() })
	return cancel
}

// Cancelling the root context (an interrupt) kills a running command's whole
// process group through the same Cancel path as a timeout, and the package
// FAILS saying it was interrupted. Setpgid detaches children from the
// terminal's Ctrl-C, so this is the only thing that stops them.
func TestInterruptKillsRunningCommandGroup(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("marker-file probe uses a POSIX shell")
	}
	withCommandTimeout(t, 0)
	cancel := withRootContext(t)
	pkg := timeoutPkg(t, "(sleep 2 && echo alive > survived.txt) & sleep 60")

	go func() {
		time.Sleep(300 * time.Millisecond)
		cancel()
	}()
	began := time.Now()
	result := runPackageBuild(pkg, false)
	if took := time.Since(began); took > 20*time.Second {
		t.Fatalf("interrupt did not stop the command: %v", took)
	}
	if result.Status != "failed" || result.ReturnCode != interruptReturnCode {
		t.Fatalf("status/code = %q/%d, want failed/%d", result.Status, result.ReturnCode, interruptReturnCode)
	}
	if !strings.Contains(result.Stderr, "interrupted") || strings.Contains(result.Stderr, "timed out") {
		t.Errorf("stderr should say interrupted: %s", result.Stderr)
	}
	time.Sleep(2500 * time.Millisecond)
	if matches, _ := filepath.Glob(filepath.Join(pkg.Path, "survived.txt")); len(matches) != 0 {
		t.Fatalf("child group outlived the interrupt: %v", matches)
	}
}

// An already-cancelled root context stops later commands from starting at all.
func TestInterruptBeforeStartFails(t *testing.T) {
	cancel := withRootContext(t)
	cancel()
	result := runPackageBuild(timeoutPkg(t, "echo should-not-run"), false)
	if result.Status != "failed" || strings.Contains(result.Stdout, "should-not-run") {
		t.Fatalf("status=%q stdout=%q", result.Status, result.Stdout)
	}
}

// A command that exits cleanly but leaves a same-group daemon holding stdout
// fails (WaitDelay), and the leftover group is killed so it does not outlive
// the build.
func TestWaitDelayKillsLeftoverGroup(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("marker-file probe uses a POSIX shell")
	}
	withCommandTimeout(t, 30*time.Second)
	pkg := timeoutPkg(t, "(sleep 2 && echo alive > survived.txt) & echo done")
	result := runPackageBuild(pkg, false)
	if result.Status != "failed" {
		t.Fatalf("status = %q, want failed", result.Status)
	}
	time.Sleep(2500 * time.Millisecond)
	if matches, _ := filepath.Glob(filepath.Join(pkg.Path, "survived.txt")); len(matches) != 0 {
		t.Fatalf("leftover daemon survived the build: %v", matches)
	}
}
