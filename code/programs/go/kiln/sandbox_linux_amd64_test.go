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
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

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
	return 0
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
	probeUserNS(t)
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
	return f
}

func (f *sandboxFixture) writeConfig(t *testing.T, edit func(*sandbox.Config)) {
	t.Helper()
	c := &sandbox.Config{
		RepoRoot: f.repo, Home: f.home, Tmpdir: f.tmp,
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

// probeUserNS skips when unprivileged user namespaces are unavailable: the
// documented condition of KILN04 section 4.3 ("No namespaces, no recording").
func probeUserNS(t *testing.T) {
	t.Helper()
	cmd := exec.Command("/bin/true")
	cmd.SysProcAttr = &syscall.SysProcAttr{Cloneflags: syscall.CLONE_NEWUSER}
	if err := cmd.Run(); err != nil {
		t.Skipf("unprivileged user namespaces are unavailable here (%v); on Ubuntu runners run: sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0", err)
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
	ctx, cancel := context.WithTimeout(context.Background(), 45*time.Second)
	defer cancel()
	args := append([]string{sandbox.ExecCommand, "--config", f.cfg, "--"}, argv...)
	cmd := exec.CommandContext(ctx, f.exe, args...)
	cmd.Dir = f.pkg
	cmd.Env = []string{"KILN_ACTION=" + action, "KILN_STEP=0", "KILN_STEP_COUNT=1", "SECRET_FROM_PARENT=leak", "PATH=/usr/bin:/bin"}
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
	r := f.sandboxRun(t, "pkg", f.exe, probeCommand)
	if r.code != 0 {
		t.Fatalf("exit %d: %s\n%s", r.code, r.stderr, r.stdout)
	}
	for _, want := range []string{
		"clone-untraced=operation not permitted", "clone-newuser=operation not permitted", "unshare=operation not permitted",
		"mount=operation not permitted", "ptrace=operation not permitted", "clone3=function not implemented", "threads=ok",
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
		if strings.HasPrefix(line, "KILN_") || strings.HasPrefix(line, "SECRET_") {
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
	cmd.Env = []string{"KILN_ACTION=pkg", "PATH=/usr/bin:/bin"}
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
	cmd.Dir, cmd.Env = f.pkg, []string{"PATH=/usr/bin"}
	var eb bytes.Buffer
	cmd.Stderr = &eb
	if err := cmd.Run(); err == nil || !strings.Contains(eb.String(), "KILN_ACTION") {
		t.Errorf("missing KILN_ACTION: %v %q", err, eb.String())
	}
	// The wrong working directory.
	cmd = exec.Command(f.exe, sandbox.ExecCommand, "--config", f.cfg, "--", "true")
	cmd.Dir, cmd.Env = f.repo, []string{"KILN_ACTION=pkg", "PATH=/usr/bin"}
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
