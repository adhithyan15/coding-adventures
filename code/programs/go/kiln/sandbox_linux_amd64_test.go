//go:build linux && amd64

package main

// End-to-end tests of `kiln sandbox-exec` (KILN04 section 4.7).  They need
// unprivileged user namespaces and skip, loudly, where the kernel refuses
// them.  The test binary doubles as the kiln binary: TestMain turns a
// `sandbox-exec` or `sandbox-init` invocation into the real command, so the
// sandbox re-executes /proc/self/exe and finds the code under test.

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// requireEnv turns every skip in this file into a failure.  CI sets it, so a
// runner that cannot give the sandbox its namespaces fails loudly instead of
// reporting a green run in which nothing was tested.
const requireEnv = "KILN_REQUIRE_SANDBOX"

// withCover adds GOCOVERDIR to the environment of a child that is this test
// binary acting as kiln, when the binary is coverage-instrumented: otherwise
// the Go runtime prints "warning: GOCOVERDIR not set" to standard error when
// the child exits, which the tests (rightly) do not tolerate.  It is for the
// ROOT's environment only; the sandbox hands the step exactly the configured
// list, which TestSandboxEnvironmentIsExactlyTheConfiguredList checks.
func withCover(t *testing.T, env []string) []string {
	t.Helper()
	if testing.CoverMode() == "" {
		return env
	}
	return append(env, "GOCOVERDIR="+t.TempDir())
}

// skipOrFail skips the test, or fails it when the sandbox is required.
func skipOrFail(t *testing.T, format string, args ...any) {
	t.Helper()
	if os.Getenv(requireEnv) == "1" {
		t.Fatalf(requireEnv+"=1 forbids skipping: "+format, args...)
	}
	t.Skipf(format, args...)
}

const probeCommand = "sandbox-probe"

func TestMain(m *testing.M) {
	if len(os.Args) > 1 {
		switch os.Args[1] {
		case sandbox.ExecCommand, sandbox.InitCommand:
			os.Exit(run(os.Args[1:], os.Stdout, os.Stderr))
		case probeCommand:
			os.Exit(probe(os.Stdout))
		}
	}
	os.Exit(m.Run())
}

// probe tries the calls the filter must refuse and prints what each returned.
// It runs INSIDE the sandbox, as the step.
func probe(out *os.File) int {
	say := func(name string, e syscall.Errno) { out.WriteString(name + "=" + e.Error() + "\n") }
	// clone(CLONE_UNTRACED): a child strace -f would never attach to.
	r, _, e := syscall.RawSyscall6(syscall.SYS_CLONE, 0x00800000|uintptr(syscall.SIGCHLD), 0, 0, 0, 0, 0)
	if e == 0 && r == 0 {
		syscall.RawSyscall(syscall.SYS_EXIT_GROUP, 0, 0, 0) // the child of a call that must have failed
	}
	say("clone-untraced", e)
	_, _, e = syscall.RawSyscall6(syscall.SYS_CLONE, uintptr(syscall.CLONE_NEWUSER)|uintptr(syscall.SIGCHLD), 0, 0, 0, 0, 0)
	say("clone-newuser", e)
	_, _, e = syscall.RawSyscall(syscall.SYS_UNSHARE, uintptr(syscall.CLONE_NEWUSER), 0, 0)
	say("unshare", e)
	_, _, e = syscall.RawSyscall6(syscall.SYS_MOUNT, 0, 0, 0, 0, 0, 0)
	say("mount", e)
	_, _, e = syscall.RawSyscall(syscall.SYS_PTRACE, 0, 0, 0)
	say("ptrace", e)
	_, _, e = syscall.RawSyscall(435, 0, 0, 0) // clone3
	say("clone3", e)
	// A thread-style clone must still be allowed: run a goroutine.
	done := make(chan struct{})
	go func() { close(done) }()
	<-done
	out.WriteString("threads=ok\n")
	// The network namespace has a working loopback and nothing else.
	if ln, err := net.Listen("tcp", "127.0.0.1:0"); err != nil {
		out.WriteString("loopback=" + err.Error() + "\n")
	} else {
		go func() {
			if c, err := ln.Accept(); err == nil {
				c.Close()
			}
		}()
		if c, err := net.DialTimeout("tcp", ln.Addr().String(), 5*time.Second); err != nil {
			out.WriteString("loopback=" + err.Error() + "\n")
		} else {
			c.Close()
			out.WriteString("loopback=ok\n")
		}
		ln.Close()
	}
	if c, err := net.DialTimeout("tcp", "192.0.2.1:80", 2*time.Second); err == nil {
		c.Close()
		out.WriteString("external=reachable\n")
	} else {
		out.WriteString("external=refused\n")
	}
	if _, err := net.DialTimeout("unix", "/run/docker.sock", time.Second); err == nil {
		out.WriteString("docker=reachable\n")
	} else {
		out.WriteString("docker=refused\n")
	}
	return 0
}

// imageRoots are the system directories a step needs, as the recorder would
// list them from host-roots.json: those of /usr, /bin, /sbin, /lib*, /etc that
// exist on this machine.  /bin and /lib are symlinks on most images.
func imageRoots() []string {
	var out []string
	for _, p := range []string{"/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32", "/etc"} {
		if _, err := os.Lstat(p); err == nil {
			out = append(out, p)
		}
	}
	return out
}

type sandboxFixture struct {
	base, repo, pkg, home, tmp, state, git, cfg string
	exe                                         string
	env                                         []string
}

// newSandboxFixture lays out a checkout, $HOME, TMPDIR and a state root as
// siblings (the repository may not be under TMPDIR), writes the configuration
// and skips if the kernel will not give us a user namespace.
func newSandboxFixture(t *testing.T) *sandboxFixture {
	t.Helper()
	base, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	f := &sandboxFixture{base: base}
	f.repo = filepath.Join(base, "repo")
	f.pkg = filepath.Join(f.repo, "pkg")
	f.home = filepath.Join(base, "home")
	f.tmp = filepath.Join(base, "tmp")
	f.state = filepath.Join(base, "state")
	f.git = filepath.Join(f.repo, ".git")
	for _, d := range []string{f.pkg, f.home, f.tmp, f.state, f.git} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	os.WriteFile(filepath.Join(f.git, "config"), []byte("secret token"), 0o644)
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	f.exe, _ = filepath.EvalSymlinks(exe)
	f.env = []string{"HOME=" + f.home, "TMPDIR=" + f.tmp, "PATH=/usr/bin:/bin", "LANG=C.UTF-8", "TZ=UTC", "PROBE=" + f.exe}
	f.cfg = filepath.Join(base, "cfg.json")
	f.writeConfig(t, nil)
	probeSandbox(t, f)
	return f
}

func (f *sandboxFixture) writeConfig(t *testing.T, edit func(*sandbox.Config)) {
	t.Helper()
	c := &sandbox.Config{
		RepoRoot: f.repo, Home: f.home, Tmpdir: f.tmp,
		RoRoots:     imageRoots(),
		StateRoots:  []string{f.state},
		Masks:       []string{f.git},
		EnvByAction: map[string][]string{"pkg": f.env},
		KilnExe:     f.exe,
	}
	if edit != nil {
		edit(c)
	}
	data, err := json.Marshal(c)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(f.cfg, data, 0o600); err != nil {
		t.Fatal(err)
	}
}

var (
	probeOnce sync.Once
	probeErr  string
)

// probeSandbox skips (or, with KILN_REQUIRE_SANDBOX=1, fails) when the kernel
// will not give the sandbox what it needs.  It probes the real thing, once: a
// trivial step through `sandbox-exec` needs a user namespace, a write of the
// id maps, mounts (tmpfs, bind, mount_setattr, proc, pivot_root), a network
// namespace and its loopback, and seccomp.  A status of 125 means the sandbox
// could not be built, and its stderr says why.
func probeSandbox(t *testing.T, f *sandboxFixture) {
	t.Helper()
	probeOnce.Do(func() {
		cmd := exec.Command(f.exe, sandbox.ExecCommand, "--config", f.cfg, "--", "true")
		cmd.Dir = f.pkg
		cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "PATH=/usr/bin:/bin"})
		var eb bytes.Buffer
		cmd.Stderr = &eb
		if err := cmd.Run(); err != nil {
			probeErr = err.Error() + ": " + strings.TrimSpace(eb.String())
		}
	})
	if probeErr != "" {
		skipOrFail(t, "the sandbox cannot be built here (%s); on Ubuntu runners run: sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0", probeErr)
	}
}

type result struct {
	stdout, stderr string
	code           int
	elapsed        time.Duration
}

// sandboxRun runs `kiln sandbox-exec --config cfg -- argv...` the way the build
// tool's -exec-wrapper does: KILN_ACTION in the wrapper's environment, the
// working directory the package's.
func (f *sandboxFixture) sandboxRun(t *testing.T, action string, argv ...string) result {
	t.Helper()
	return f.sandboxRunTimeout(t, 45*time.Second, action, argv...)
}

// sandboxRunTimeout is sandboxRun with its own bound.  On a timeout the test
// fails with what the step had printed so far.
func (f *sandboxFixture) sandboxRunTimeout(t *testing.T, limit time.Duration, action string, argv ...string) result {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), limit)
	defer cancel()
	args := append([]string{sandbox.ExecCommand, "--config", f.cfg, "--"}, argv...)
	cmd := exec.CommandContext(ctx, f.exe, args...)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=" + action, "KILN_STEP=0", "KILN_STEP_COUNT=1", "SECRET_FROM_PARENT=leak", "PATH=/usr/bin:/bin"})
	var out, errb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &errb
	start := time.Now()
	err := cmd.Run()
	r := result{stdout: out.String(), stderr: errb.String(), elapsed: time.Since(start)}
	if ee, ok := err.(*exec.ExitError); ok {
		r.code = ee.ExitCode()
	} else if err != nil {
		t.Fatalf("%v\nstderr: %s", err, r.stderr)
	}
	if ctx.Err() != nil {
		t.Fatalf("timed out; stdout %q stderr %q", r.stdout, r.stderr)
	}
	return r
}

func TestSandboxIdentityAndPrivileges(t *testing.T) {
	f := newSandboxFixture(t)
	r := f.sandboxRun(t, "pkg", "sh", "-c", "id -u; id -g; echo pid=$$; grep -E '^(CapInh|CapPrm|CapEff|CapBnd|CapAmb|NoNewPrivs|Seccomp):' /proc/self/status; ls /")
	if r.code != 0 {
		t.Fatalf("exit %d: %s", r.code, r.stderr)
	}
	for _, want := range []string{"\n1000\n", "pid=1\n", "CapInh:\t0000000000000000", "CapPrm:\t0000000000000000", "CapEff:\t0000000000000000",
		"CapBnd:\t0000000000000000", "CapAmb:\t0000000000000000", "NoNewPrivs:\t1", "Seccomp:\t2", "\nusr\n"} {
		if !strings.Contains("\n"+r.stdout, want) {
			t.Errorf("output lacks %q:\n%s", want, r.stdout)
		}
	}
	if strings.HasPrefix(r.stdout, "0\n") {
		t.Error("the step runs as uid 0")
	}
}

func TestSandboxFilesystem(t *testing.T) {
	f := newSandboxFixture(t)
	script := strings.Join([]string{
		`echo mine > out.txt && echo repo-write-ok`,
		`if touch /usr/kiln-probe 2>/dev/null; then echo usr-writable; else echo usr-readonly; fi`,
		`echo "git:[$(ls -A ` + f.git + ` | wc -l)]"`,
		`if echo x > ` + f.git + `/new 2>/dev/null; then echo git-writable; else echo git-readonly; fi`,
		`echo h > "$HOME/h" && echo home-ok`,
		`echo t > "$TMPDIR/t" && echo tmp-ok`,
		`echo s > ` + f.state + `/s && echo state-ok`,
		`cd ` + f.pkg + ` && echo "cwd=$(pwd)"`,
	}, "; ")
	r := f.sandboxRun(t, "pkg", "sh", "-c", script)
	if r.code != 0 {
		t.Fatalf("exit %d: %s", r.code, r.stderr)
	}
	for _, want := range []string{"repo-write-ok", "usr-readonly", "git:[0]", "git-readonly", "home-ok", "tmp-ok", "state-ok", "cwd=" + f.pkg} {
		if !strings.Contains(r.stdout, want) {
			t.Errorf("output lacks %q:\n%s", want, r.stdout)
		}
	}
	// What the step wrote to the repository and the state root is on the host.
	for p, want := range map[string]string{filepath.Join(f.pkg, "out.txt"): "mine\n", filepath.Join(f.state, "s"): "s\n"} {
		if got, _ := os.ReadFile(p); string(got) != want {
			t.Errorf("%s = %q, want %q", p, got, want)
		}
	}
	// Nothing the step wrote to $HOME or TMPDIR reached the host, and the host
	// .git is untouched.
	for _, p := range []string{filepath.Join(f.home, "h"), filepath.Join(f.tmp, "t"), filepath.Join(f.git, "new")} {
		if _, err := os.Stat(p); err == nil {
			t.Errorf("%s leaked to the host", p)
		}
	}
	if got, _ := os.ReadFile(filepath.Join(f.git, "config")); string(got) != "secret token" {
		t.Error("the host .git was altered")
	}
}

func TestSandboxSeccompRefusals(t *testing.T) {
	f := newSandboxFixture(t)
	// The test binary lives under /tmp, which the step does not see: run a copy
	// from inside the checkout.
	self, err := os.ReadFile(f.exe)
	if err != nil {
		t.Fatal(err)
	}
	probeBin := filepath.Join(f.pkg, "probe-bin")
	if err := os.WriteFile(probeBin, self, 0o755); err != nil {
		t.Fatal(err)
	}
	r := f.sandboxRun(t, "pkg", probeBin, probeCommand)
	if r.code != 0 {
		t.Fatalf("exit %d: %s\n%s", r.code, r.stderr, r.stdout)
	}
	for _, want := range []string{
		"clone-untraced=operation not permitted", "clone-newuser=operation not permitted", "unshare=operation not permitted",
		"mount=operation not permitted", "ptrace=operation not permitted", "clone3=function not implemented", "threads=ok",
		"loopback=ok", "external=refused", "docker=refused",
	} {
		if !strings.Contains(r.stdout, want) {
			t.Errorf("output lacks %q:\n%s", want, r.stdout)
		}
	}
}

func TestSandboxKillsSurvivorsWhenPID1Exits(t *testing.T) {
	f := newSandboxFixture(t)
	// The background sleeper holds the step's standard output open.  If it
	// survived the step, the pipe would stay open and the run would last the
	// full 300 seconds (the 45 second timeout would fail the test first).
	r := f.sandboxRun(t, "pkg", "sh", "-c", "sleep 300 & echo started")
	if r.code != 0 || !strings.Contains(r.stdout, "started") {
		t.Fatalf("exit %d %q %q", r.code, r.stdout, r.stderr)
	}
	if r.elapsed > 30*time.Second {
		t.Errorf("took %v: the background process survived PID 1", r.elapsed)
	}
}

func TestSandboxEnvironmentIsExactlyTheConfiguredList(t *testing.T) {
	f := newSandboxFixture(t)
	// No shell: `env` itself, found through the CONFIGURED PATH.
	r := f.sandboxRun(t, "pkg", "env")
	if r.code != 0 {
		t.Fatalf("exit %d: %s", r.code, r.stderr)
	}
	got := strings.Split(strings.TrimSuffix(r.stdout, "\n"), "\n")
	if strings.Join(got, "\n") != strings.Join(f.env, "\n") {
		t.Errorf("environment:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(f.env, "\n"))
	}
	for _, line := range got {
		if strings.HasPrefix(line, "KILN_") || strings.HasPrefix(line, "SECRET_") || strings.HasPrefix(line, "GOCOVERDIR") {
			t.Errorf("%q reached the step", line)
		}
	}
}

func TestSandboxStdinAndDescriptors(t *testing.T) {
	f := newSandboxFixture(t)
	// Standard input is /dev/null: cat ends at once, though our stdin is a pipe
	// that never closes.
	pr, pw, _ := os.Pipe()
	defer pr.Close()
	defer pw.Close()
	args := []string{sandbox.ExecCommand, "--config", f.cfg, "--", "sh", "-c", "cat; echo eof; ls /proc/self/fd | sort -n | tail -1"}
	cmd := exec.Command(f.exe, args...)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "PATH=/usr/bin:/bin"})
	cmd.Stdin = pr
	cmd.ExtraFiles = []*os.File{pr, pw} // descriptors 3 and 4 in the root
	done := make(chan struct{})
	var out []byte
	var err error
	go func() { out, err = cmd.CombinedOutput(); close(done) }()
	select {
	case <-done:
	case <-time.After(30 * time.Second):
		cmd.Process.Kill()
		t.Fatal("cat did not see end of file: stdin is not /dev/null")
	}
	if err != nil {
		t.Fatalf("%v: %s", err, out)
	}
	lines := strings.Fields(string(out))
	if len(lines) < 2 || lines[0] != "eof" {
		t.Fatalf("%q", out)
	}
	if n, _ := strconv.Atoi(lines[1]); n > 3 { // 3 is ls's own directory descriptor
		t.Errorf("descriptor %d reached the step", n)
	}
}

func TestSandboxExitStatusAndErrors(t *testing.T) {
	f := newSandboxFixture(t)
	if r := f.sandboxRun(t, "pkg", "sh", "-c", "exit 7"); r.code != 7 {
		t.Errorf("exit %d, want 7 (%s)", r.code, r.stderr)
	}
	if r := f.sandboxRun(t, "pkg", "no-such-command-anywhere"); r.code != 127 {
		t.Errorf("exit %d, want 127 (%s)", r.code, r.stderr)
	}
	// An action the configuration does not name is refused before anything runs.
	r := f.sandboxRun(t, "other", "true")
	if r.code != sandbox.ExitFailure || !strings.Contains(r.stderr, "KILN_ACTION") {
		t.Errorf("unknown action: %d %q", r.code, r.stderr)
	}
	// No KILN_ACTION at all.
	cmd := exec.Command(f.exe, sandbox.ExecCommand, "--config", f.cfg, "--", "true")
	cmd.Dir, cmd.Env = f.pkg, withCover(t, []string{"PATH=/usr/bin"})
	var eb bytes.Buffer
	cmd.Stderr = &eb
	if err := cmd.Run(); err == nil || !strings.Contains(eb.String(), "KILN_ACTION") {
		t.Errorf("missing KILN_ACTION: %v %q", err, eb.String())
	}
	// The wrong working directory.
	cmd = exec.Command(f.exe, sandbox.ExecCommand, "--config", f.cfg, "--", "true")
	cmd.Dir, cmd.Env = f.repo, withCover(t, []string{"KILN_ACTION=pkg", "PATH=/usr/bin"})
	eb.Reset()
	cmd.Stderr = &eb
	if err := cmd.Run(); err == nil || !strings.Contains(eb.String(), "package directory") {
		t.Errorf("wrong cwd: %v %q", err, eb.String())
	}
}

func TestSandboxRefusesKilnBinaryInsideTheCheckout(t *testing.T) {
	f := newSandboxFixture(t)
	f.writeConfig(t, func(c *sandbox.Config) { c.KilnExe = filepath.Join(f.repo, "kiln") })
	r := f.sandboxRun(t, "pkg", "true")
	if r.code != sandbox.ExitFailure || !strings.Contains(r.stderr, "kiln_exe") {
		t.Errorf("%d %q", r.code, r.stderr)
	}
	// A configuration naming another binary than the running one.
	f.writeConfig(t, func(c *sandbox.Config) { c.KilnExe = filepath.Join(f.base, "elsewhere-kiln") })
	r = f.sandboxRun(t, "pkg", "true")
	if r.code != sandbox.ExitFailure || !strings.Contains(r.stderr, "running binary") {
		t.Errorf("%d %q", r.code, r.stderr)
	}
}

func TestSandboxInitRefusesToRunOutsideANamespace(t *testing.T) {
	// `sandbox-init` is internal: started by hand it is not PID 1.
	cmd := exec.Command(os.Args[0], sandbox.InitCommand, "--config", "/c", "--sha256", strings.Repeat("0", 64),
		"--action", "a", "--uid", "1", "--gid", "1", "--", "true")
	var eb bytes.Buffer
	cmd.Stderr = &eb
	err := cmd.Run()
	if ee, ok := err.(*exec.ExitError); !ok || ee.ExitCode() != sandbox.ExitFailure || !strings.Contains(eb.String(), "PID 1") {
		t.Errorf("%v %q", err, eb.String())
	}
}

func TestSandboxStepCannotSignalOutside(t *testing.T) {
	f := newSandboxFixture(t)
	// The step is its own session and process group leader inside a PID
	// namespace of its own: kill(0, sig) reaches only itself (ignored: it is
	// PID 1).  Were it in strace's group, these would kill strace and the
	// build tool, and this test process with them.
	r := f.sandboxRun(t, "pkg", "sh", "-c", "kill -USR1 0; kill -KILL 0; echo alive; cut -d' ' -f5,6 /proc/self/stat")
	if r.code != 0 || !strings.HasPrefix(r.stdout, "alive\n") {
		t.Fatalf("exit %d, stdout %q, stderr %q", r.code, r.stdout, r.stderr)
	}
	// pgrp and session of a child of the step: both are the step's (pid 1).
	if got := strings.TrimSpace(strings.TrimPrefix(r.stdout, "alive\n")); got != "1 1" {
		t.Errorf("process group and session %q, want %q", got, "1 1")
	}
	// This process, and its process group, are alive: we are still running.
	if err := syscall.Kill(0, 0); err != nil {
		t.Fatal(err)
	}
}

func TestSandboxNoHostIPC(t *testing.T) {
	f := newSandboxFixture(t)
	script := strings.Join([]string{
		`sed -n '3,$p' /proc/net/dev | cut -d: -f1 | tr -d ' '`,
		`for p in /run/docker.sock /var/run/docker.sock /run/user /run/dbus /var/run/dbus; do test -e $p && echo "present:$p"; done`,
		`echo "run:$(ls -A /run | wc -l)"`,
		`echo "dev:$(ls -A /dev | tr '\n' ' ')"`,
		`echo x > /dev/null && echo devnull-ok`,
		`if echo x > /dev/leak 2>/dev/null; then echo dev-writable; else echo dev-readonly; fi`,
		`if echo x > /run/own 2>/dev/null; then echo run-writable; else echo run-readonly; fi`,
		`echo "core:$(awk '/core file/{print $5,$6}' /proc/self/limits)"`,
		`echo "ipc:$(cat /proc/sysvipc/shm | wc -l)"`,
	}, "; ")
	r := f.sandboxRun(t, "pkg", "sh", "-c", script)
	if r.code != 0 {
		t.Fatalf("exit %d: %s", r.code, r.stderr)
	}
	out := r.stdout
	if !strings.HasPrefix(out, "lo\nrun:0\n") {
		t.Errorf("network interfaces: %q (want lo only)", out)
	}
	if strings.Contains(out, "present:") {
		t.Errorf("a host service path exists:\n%s", out)
	}
	for _, want := range []string{"run:0", "dev:fd full null random stderr stdin stdout tty urandom zero ", "devnull-ok", "dev-readonly", "run-writable", "core:1 1", "ipc:1"} {
		if !strings.Contains(out, want) {
			t.Errorf("output lacks %q:\n%s", want, out)
		}
	}
}

// TestSandboxStraceTraceSetNamesAreValid checks every name of the -e trace=
// set against the strace this runner has, one at a time, so an invalid name
// (futimens is not a system call on x86-64) fails loudly with its name instead
// of as an opaque strace error in the real-strace test.
func TestSandboxStraceTraceSetNamesAreValid(t *testing.T) {
	straceBin, err := exec.LookPath("strace")
	if err != nil {
		skipOrFail(t, "strace is not installed")
	}
	names := strings.Split(requiredTraceSet, ",")
	if len(names) < 20 {
		t.Fatalf("the trace set has only %d names", len(names))
	}
	// The options the recorder relies on, too.
	for _, opt := range []string{"--kill-on-exit", "--pidns-translation", "--quiet=attach"} {
		if out, err := exec.Command(straceBin, "-o", os.DevNull, opt, "true").CombinedOutput(); err != nil {
			t.Errorf("strace does not accept %s: %v: %s", opt, err, strings.TrimSpace(string(out)))
		}
	}
	for _, name := range names {
		out, err := exec.Command(straceBin, "-o", os.DevNull, "-e", "trace="+name, "true").CombinedOutput()
		if err != nil {
			t.Errorf("strace does not accept trace=%s: %v: %s", name, err, strings.TrimSpace(string(out)))
		}
	}
}

// requiredTraceSet is the -e trace= set of KILN04 section 4.3.
const requiredTraceSet = "%file,%process,%network,getdents,getdents64,fchdir," +
	"io_uring_setup,open_by_handle_at,bpf,setns,unshare,fsopen,fsmount," +
	"fsconfig,fspick,open_tree,move_mount,mount_setattr,ptrace," +
	"process_vm_readv,pidfd_getfd,inotify_add_watch,fanotify_mark," +
	"fchmod,fchown,fsetxattr,fremovexattr"

// TestSandboxRealStrace runs the sandbox under the real strace the way the
// recorder will, and parses the trace with the stage 1a parser in helper mode.
// It pins what the canned fixtures only assume: that the root makes exactly one
// spawn and nothing after it (no pidfd-probe clone from os/exec, no mapping
// writes), that the init's setup is tolerated, that --kill-on-exit is accepted,
// and strace's escaping of the -y annotation.
func TestSandboxRealStrace(t *testing.T) {
	straceBin, err := exec.LookPath("strace")
	if err != nil {
		skipOrFail(t, "strace is not installed")
	}
	f := newSandboxFixture(t)
	data := filepath.Join(f.pkg, "data.txt")
	if err := os.WriteFile(data, []byte("hello\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	trf := filepath.Join(f.base, "trace.txt")
	args := []string{"-f", "--quiet=attach", "-y", "-s", "4096", "--kill-on-exit", "--pidns-translation", "-e", "signal=none",
		"-e", "trace=" + requiredTraceSet, "-o", trf, "--",
		f.exe, sandbox.ExecCommand, "--config", f.cfg, "--", "sh", "-c", "cat " + data}
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, straceBin, args...)
	cmd.Dir = f.pkg
	cmd.Env = withCover(t, []string{"KILN_ACTION=pkg", "KILN_STEP=0", "KILN_STEP_COUNT=1", "PATH=/usr/bin:/bin"})
	var out, eb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &eb
	if err := cmd.Run(); err != nil {
		t.Fatalf("strace: %v\nstderr: %s", err, eb.String())
	}
	if eb.Len() != 0 {
		t.Fatalf("strace wrote to standard error: %q", eb.String())
	}
	if out.String() != "hello\n" {
		t.Fatalf("step output %q", out.String())
	}
	fh, err := os.Open(trf)
	if err != nil {
		t.Fatal(err)
	}
	defer fh.Close()
	tr, err := trace.ParseOptions(fh, f.pkg, trace.Options{HelperRoot: true, HelperExe: f.exe})
	if err != nil {
		// Show the lines around the refused one (its number is in the error),
		// each cut short, so the next fix needs no second CI run to find them.
		raw, _ := os.ReadFile(trf)
		lines := strings.Split(string(raw), "\n")
		at := 0
		var te *trace.Error
		if errors.As(err, &te) {
			at = te.Line
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
		t.Fatalf("the trace was refused: %v (line %d of %d)\n--- around the refused line ---\n%s", err, at, len(lines), win.String())
	}
	if ex := tr.StepExit(); ex == nil || ex.Killed || ex.Code != 0 {
		t.Errorf("step exit %+v", ex)
	}
	step := tr.StepEvent()
	if step == nil || !strings.HasSuffix(step.Path, "/sh") || len(step.Argv) != 3 || step.Argv[0] != "sh" || step.Argv[1] != "-c" {
		t.Fatalf("step event %+v", step)
	}
	var sawCatExec, sawRead bool
	for _, e := range tr.Events {
		if e.Syscall == "execve" && e.Result.OK && strings.HasSuffix(e.Path, "/cat") {
			sawCatExec = true
		}
		if (e.Syscall == "openat" || e.Syscall == "open") && e.Result.OK && e.Path == data {
			sawRead = true
		}
	}
	if !sawCatExec || !sawRead {
		t.Errorf("exec of cat: %v, read of %s: %v", sawCatExec, data, sawRead)
	}
}

// TestSandboxNoReachableSocketsOrFifos: a pathname Unix socket or FIFO belongs
// to the filesystem, not to the network namespace, and a read-only bind does
// not stop connect() or an open for writing.  The runner's .NET diagnostic
// socket in /tmp lets a step make Runner.Worker load a library.  So the new
// root is an allowlist; this test walks EVERY mount of the sandbox and finds no
// socket or FIFO at all (the step creates none), and checks that sockets and
// FIFOs the test makes on the host, in the places such things live, are not
// visible.
func TestSandboxNoReachableSocketsOrFifos(t *testing.T) {
	f := newSandboxFixture(t)
	// Host sockets and FIFOs, outside the sandbox, where the runner's own
	// live: /tmp (the .NET diagnostic socket), /var/tmp, and a pretend /run.
	var hostPaths []string
	stamp := strconv.Itoa(os.Getpid()) + "-" + strconv.FormatInt(time.Now().UnixNano(), 36)
	for _, dir := range []string{"/tmp", "/var/tmp"} {
		sock := filepath.Join(dir, "kiln-sbx-"+stamp+".sock")
		ln, err := net.Listen("unix", sock)
		if err != nil {
			t.Logf("cannot make a socket in %s: %v", dir, err)
			continue
		}
		defer ln.Close()
		defer os.Remove(sock)
		fifo := filepath.Join(dir, "kiln-sbx-"+stamp+".fifo")
		if err := syscall.Mkfifo(fifo, 0o600); err != nil {
			t.Logf("cannot make a FIFO in %s: %v", dir, err)
			continue
		}
		defer os.Remove(fifo)
		hostPaths = append(hostPaths, sock, fifo)
	}
	if len(hostPaths) == 0 {
		t.Fatal("no host socket or FIFO could be created to test with")
	}
	script := strings.Join([]string{
		`for p in ` + strings.Join(hostPaths, " ") + `; do if test -e "$p"; then echo "visible:$p"; fi; done`,
		// Every mount, in parallel (/usr on a runner image is huge), each with
		// find, which only lstats: it never opens a FIFO, so it cannot block.
		// -quit ends a mount's walk at its first hit.  Progress lines make a
		// timeout diagnosable: a mount with "start:" and no "done:" is the slow one.
		`n=0; for m in $(awk '{print $5}' /proc/self/mountinfo | sort -u); do n=$((n+1)); ( echo "start:$m"; find "$m" -xdev \( -type s -o -type p \) -print -quit 2>/dev/null | sed 's/^/found:/'; echo "done:$m" ) & done; wait`,
		`echo "mounts:$n"`,
	}, "; ")
	r := f.sandboxRunTimeout(t, 4*time.Minute, "pkg", "sh", "-c", script)
	if r.code != 0 {
		t.Fatalf("exit %d: %s", r.code, r.stderr)
	}
	if strings.Contains(r.stdout, "visible:") || strings.Contains(r.stdout, "found:") {
		t.Errorf("a socket or FIFO is reachable from the sandbox:\n%s", r.stdout)
	}
	m := regexp.MustCompile(`mounts:(\d+)`).FindStringSubmatch(r.stdout)
	if m == nil {
		t.Fatalf("no mount count in %q", r.stdout)
	}
	if n, _ := strconv.Atoi(m[1]); n < 8 {
		t.Errorf("only %d mounts walked; the walk did not cover the sandbox", n)
	}
}
