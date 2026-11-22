//go:build unix

package record

// The toolchain prober (probe_toolchain.go) with an injected process starter:
// no sandbox is started. realProbeExec is exercised with plain `sh`.

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// proberFix is a prober over a real, canonical temp tree, with a fake starter.
type proberFix struct {
	t      *testing.T
	dir    string
	kiln   string
	base   *sandbox.Config
	p      *toolchainProber
	cmds   []probeCmd
	result func(probeCmd) ([]byte, int, bool, error)
}

func newProberFix(t *testing.T) *proberFix {
	t.Helper()
	base := canonTemp(t)
	f := &proberFix{t: t, dir: filepath.Join(base, "record"), kiln: filepath.Join(base, "bin", "kiln")}
	writeBytes(t, f.kiln, []byte("kiln"), 0o755)
	if err := os.Mkdir(f.dir, 0o700); err != nil {
		t.Fatal(err)
	}
	home, tmp := filepath.Join(base, "home"), filepath.Join(base, "tmp")
	for _, d := range []string{home, tmp} {
		if err := os.Mkdir(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	f.base = &sandbox.Config{
		RepoRoot: filepath.Join(base, "repo"), Home: home, Tmpdir: tmp, KilnExe: f.kiln,
		RoRoots: []string{"/usr"}, StateRoots: []string{filepath.Join(base, "state")}, Masks: []string{filepath.Join(base, "repo", ".git")},
		EnvByAction: map[string][]string{"x": {"PATH=/usr/bin"}},
	}
	f.p = newToolchainProber(f.dir, f.kiln, "/usr/bin:/bin")
	f.p.exec = func(c probeCmd) ([]byte, int, bool, error) {
		f.cmds = append(f.cmds, c)
		if f.result != nil {
			return f.result(c)
		}
		return []byte("sed 4.9\n"), 0, false, nil
	}
	return f
}

func (f *proberFix) run(env []string, argv0 string, args ...string) ([]byte, error) {
	return f.p.Run(legacy.ProbeRun{Path: "/usr/bin/" + argv0, Argv0: argv0, Args: args, Env: env, Dir: "/host/tmp", Timeout: time.Minute, Limit: 1000})
}

var probeEnv0 = []string{"PATH=/usr/bin:/bin", "LANG=C", "LC_ALL=C", "TZ=UTC", "HOME=/host/tmp/kiln-probe-1"}

func TestProberRefusesBeforeItIsArmed(t *testing.T) {
	f := newProberFix(t)
	if _, err := f.run(probeEnv0, "sed", "--version"); err == nil || !strings.Contains(err.Error(), "before the sandbox configuration") {
		t.Fatalf("%v", err)
	}
	if f.p.Err() == nil || len(f.cmds) != 0 {
		t.Fatalf("a probe ran, or the refusal was not remembered: %v %d", f.p.Err(), len(f.cmds))
	}
}

func TestProberRunsAProbeInThePlainSandbox(t *testing.T) {
	f := newProberFix(t)
	if err := f.p.Arm(f.base); err != nil {
		t.Fatal(err)
	}
	out, err := f.run(probeEnv0, "sed", "--version")
	if err != nil || string(out) != "sed 4.9\n" {
		t.Fatalf("%q %v", out, err)
	}
	cfgPath := filepath.Join(f.dir, "probe", "toolchains-sandbox.json")
	data, err := os.ReadFile(cfgPath)
	if err != nil {
		t.Fatal(err)
	}
	sum := sha256.Sum256(data)
	c := f.cmds[0]
	want := []string{"sandbox-exec", "--config", cfgPath, "--config-sha256", hex.EncodeToString(sum[:]), "--", "sed", "--version"}
	if c.Path != f.kiln || strings.Join(c.Args, "\x00") != strings.Join(want, "\x00") {
		t.Fatalf("%+v", c)
	}
	if strings.Join(c.Env, " ") != "KILN_ACTION=p PATH=/usr/bin:/bin" {
		t.Errorf("process env %v", c.Env)
	}
	if c.Dir != filepath.Join(f.dir, "probe", "root", "p") || c.Timeout != time.Minute || c.Limit != 1000 {
		t.Errorf("%+v", c)
	}
	// The configuration: an EMPTY checkout, no state, no masks, the probe's
	// environment with HOME replaced.
	cfg, err := sandbox.ParseConfig(data)
	if err != nil {
		t.Fatal(err)
	}
	if cfg.RepoRoot != filepath.Join(f.dir, "probe", "root") || len(cfg.StateRoots) != 0 || len(cfg.Masks) != 0 {
		t.Errorf("%+v", cfg)
	}
	if cfg.Home != f.base.Home || cfg.Tmpdir != filepath.Join(f.dir, "probe", "tmp") || cfg.Tmpdir == f.base.Tmpdir || cfg.KilnExe != f.kiln || len(cfg.RoRoots) != 1 || cfg.RoRoots[0] != "/usr" {
		t.Errorf("%+v", cfg)
	}
	env := cfg.EnvByAction["p"]
	if len(cfg.EnvByAction) != 1 || strings.Join(env, " ") != "PATH=/usr/bin:/bin LANG=C LC_ALL=C TZ=UTC HOME="+f.base.Home {
		t.Errorf("env %v", cfg.EnvByAction)
	}
	if ents, _ := os.ReadDir(filepath.Join(f.dir, "probe", "root", "p")); len(ents) != 0 {
		t.Errorf("the probes' checkout is not empty: %v", ents)
	}
	// A second probe reuses the file (it is created O_EXCL) and the same digest.
	if _, err := f.run(probeEnv0, "xtool", "-V"); err != nil {
		t.Fatal(err)
	}
	if f.cmds[1].Args[4] != f.cmds[0].Args[4] {
		t.Error("another configuration digest")
	}
	if f.p.Err() != nil {
		t.Fatal(f.p.Err())
	}
}

func TestProberStatusesAreMismatchesNotFatal(t *testing.T) {
	cases := map[string]struct {
		res  func(probeCmd) ([]byte, int, bool, error)
		want string
	}{
		"the tool fails":         {func(probeCmd) ([]byte, int, bool, error) { return []byte("x"), 3, false, nil }, "exited 3"},
		"killed by a signal":     {func(probeCmd) ([]byte, int, bool, error) { return nil, -1, false, nil }, "exited -1"},
		"the deadline kills it":  {func(probeCmd) ([]byte, int, bool, error) { return []byte("partial"), -1, true, nil }, "did not finish"},
		"exit 0 after a timeout": {func(probeCmd) ([]byte, int, bool, error) { return nil, 0, true, nil }, "did not finish"},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			f := newProberFix(t)
			f.result = c.res
			if err := f.p.Arm(f.base); err != nil {
				t.Fatal(err)
			}
			out, err := f.run(probeEnv0, "sed", "--version")
			if err == nil || !strings.Contains(err.Error(), c.want) {
				t.Fatalf("%q %v", out, err)
			}
			if f.p.Err() != nil {
				t.Fatalf("a tool's failure is a mismatch, not a failed prepare: %v", f.p.Err())
			}
		})
	}
	// ... which is what legacy.Probe makes of it (a failed run never matches).
}

func TestProberFatalErrors(t *testing.T) {
	cases := map[string]func(f *proberFix) error{
		"it cannot start the sandbox": func(f *proberFix) error {
			f.result = func(probeCmd) ([]byte, int, bool, error) { return nil, 0, false, errors.New("exec format error") }
			_, err := f.run(probeEnv0, "sed")
			return err
		},
		"no HOME": func(f *proberFix) error {
			_, err := f.run([]string{"PATH=/usr/bin"}, "sed")
			return err
		},
		"an argv0 with a slash": func(f *proberFix) error {
			_, err := f.run(probeEnv0, "../sed")
			return err
		},
		"an empty argv0": func(f *proberFix) error {
			_, err := f.p.Run(legacy.ProbeRun{Env: probeEnv0, Timeout: time.Second, Limit: 1})
			return err
		},
		"the environment changes": func(f *proberFix) error {
			if _, err := f.run(probeEnv0, "sed"); err != nil {
				return err
			}
			_, err := f.run(append(append([]string{}, probeEnv0...), "EXTRA=1"), "sed")
			return err
		},
		"another kiln": func(f *proberFix) error {
			f.p.kilnExe = "/elsewhere/kiln"
			_, err := f.run(probeEnv0, "sed")
			return err
		},
		"a configuration the sandbox refuses": func(f *proberFix) error {
			f.p.base.RoRoots = []string{"/nonexistent-kiln-root"}
			_, err := f.run(probeEnv0, "sed")
			return err
		},
		"an environment entry of Kiln's": func(f *proberFix) error {
			_, err := f.run(append(append([]string{}, probeEnv0...), "KILN_X=1"), "sed")
			return err
		},
	}
	for name, mod := range cases {
		t.Run(name, func(t *testing.T) {
			f := newProberFix(t)
			if err := f.p.Arm(f.base); err != nil {
				t.Fatal(err)
			}
			if err := mod(f); err == nil {
				t.Fatal("no error")
			}
			if f.p.Err() == nil {
				t.Fatal("the failure was not remembered for prepare")
			}
			first := f.p.Err()
			f.p.fail(errors.New("later"))
			if f.p.Err() != first {
				t.Fatal("the FIRST fatal error must stay")
			}
		})
	}
}

func TestProberArm(t *testing.T) {
	f := newProberFix(t)
	if err := f.p.Arm(nil); err == nil {
		t.Error("armed with no configuration")
	}
	if err := f.p.Arm(f.base); err != nil {
		t.Fatal(err)
	}
	if err := f.p.Arm(f.base); err == nil {
		t.Error("armed twice")
	}
	// The structure probe makes <dir>/probe first: a real directory there is fine.
	g := newProberFix(t)
	if err := os.Mkdir(filepath.Join(g.dir, "probe"), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := g.p.Arm(g.base); err != nil {
		t.Fatalf("a probe directory that already exists: %v", err)
	}
	// A symlink or a file in its place is refused.
	h := newProberFix(t)
	if err := os.Symlink(h.dir, filepath.Join(h.dir, "probe")); err != nil {
		t.Skip("no symlinks")
	}
	if err := h.p.Arm(h.base); err == nil {
		t.Error("a symlink as the probe directory")
	}
	i := newProberFix(t)
	writeBytes(t, filepath.Join(i.dir, "probe"), []byte("x"), 0o600)
	if err := i.p.Arm(i.base); err == nil {
		t.Error("a file as the probe directory")
	}
	// The checkout of the probes must not exist yet.
	j := newProberFix(t)
	if err := os.MkdirAll(filepath.Join(j.dir, "probe", "root"), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := j.p.Arm(j.base); err == nil {
		t.Error("a pre-existing probes' checkout")
	}
}

func TestRealProbeExec(t *testing.T) {
	sh := func(script string, timeout time.Duration, limit int) ([]byte, int, bool, error) {
		return realProbeExec(probeCmd{Path: "/bin/sh", Args: []string{"-c", script}, Env: []string{"PATH=/usr/bin:/bin"}, Dir: t.TempDir(), Timeout: timeout, Limit: limit})
	}
	out, exit, timedOut, err := sh("echo out; echo err >&2; exit 3", 10*time.Second, 1000)
	if err != nil || exit != 3 || timedOut || string(out) != "out\nerr\n" {
		t.Fatalf("%q %d %v %v", out, exit, timedOut, err)
	}
	out, exit, _, err = sh("echo fine", 10*time.Second, 1000)
	if err != nil || exit != 0 || string(out) != "fine\n" {
		t.Fatalf("%q %d %v", out, exit, err)
	}
	// Combined output is capped.
	out, _, _, err = sh("i=0; while [ $i -lt 500 ]; do echo 0123456789; i=$((i+1)); done", 10*time.Second, 10)
	if err != nil || len(out) != 10 {
		t.Fatalf("%d %v", len(out), err)
	}
	// stdin is /dev/null.
	if out, exit, _, _ = sh("cat; echo done", 10*time.Second, 100); exit != 0 || string(out) != "done\n" {
		t.Fatalf("%q %d", out, exit)
	}
	// The deadline kills the probe, and a grandchild holding the pipe open does
	// not make the caller wait for it.
	began := time.Now()
	_, exit, timedOut, err = sh("sleep 30 & sleep 30", 200*time.Millisecond, 100)
	if err != nil || !timedOut || exit == 0 {
		t.Fatalf("%d %v %v", exit, timedOut, err)
	}
	if time.Since(began) > 20*time.Second {
		t.Fatalf("the deadline did not stop the wait: %v", time.Since(began))
	}
	// A program that does not exist is an error of the recorder, not a status.
	if _, _, _, err := realProbeExec(probeCmd{Path: "/nonexistent/kiln", Timeout: time.Second, Limit: 1}); err == nil {
		t.Fatal("started a program that does not exist")
	}
}

func TestLimitedBuffer(t *testing.T) {
	b := &limitedBuffer{limit: 4}
	if n, err := b.Write([]byte("abcdef")); n != 6 || err != nil {
		t.Fatal(n, err)
	}
	b.Write([]byte("zz"))
	if b.b.String() != "abcd" {
		t.Fatalf("%q", b.b.String())
	}
}

func TestProberOneRefusalIsAMismatchButAllRefusedIsBroken(t *testing.T) {
	f := newProberFix(t)
	if err := f.p.Arm(f.base); err != nil {
		t.Fatal(err)
	}
	f.result = func(c probeCmd) ([]byte, int, bool, error) {
		if c.Args[len(c.Args)-2] == "refused-tool" { // argv0 is second to last
			return []byte("no root"), 125, false, nil
		}
		return []byte("ok"), 0, false, nil
	}
	if _, err := f.run(probeEnv0, "good"); err != nil {
		t.Fatal(err)
	}
	if _, err := f.run(probeEnv0, "refused-tool", "x"); err == nil || !strings.Contains(err.Error(), "refused the probe (exit 125)") {
		t.Fatalf("%v", err)
	}
	if f.p.Err() != nil {
		t.Fatalf("one refusal among working probes must stay a mismatch: %v", f.p.Err())
	}

	// Every probe that ran was refused: the recorder is malfunctioning.
	g := newProberFix(t)
	if err := g.p.Arm(g.base); err != nil {
		t.Fatal(err)
	}
	g.result = func(probeCmd) ([]byte, int, bool, error) { return []byte("no root"), 125, false, nil }
	if g.p.Err() != nil {
		t.Fatal("nothing ran yet")
	}
	g.run(probeEnv0, "sed", "--version")
	g.run(probeEnv0, "bash", "--version")
	if err := g.p.Err(); err == nil || !strings.Contains(err.Error(), "refused all 2 probes") {
		t.Fatalf("%v", err)
	}
	// A timeout is not a refusal.
	h := newProberFix(t)
	if err := h.p.Arm(h.base); err != nil {
		t.Fatal(err)
	}
	h.result = func(probeCmd) ([]byte, int, bool, error) { return nil, -1, true, nil }
	h.run(probeEnv0, "sed")
	if h.p.Err() != nil {
		t.Fatalf("%v", h.p.Err())
	}
}

func TestProbeSandboxHasItsOwnTmpdir(t *testing.T) {
	f := newProberFix(t)
	if err := f.p.Arm(f.base); err != nil {
		t.Fatal(err)
	}
	st, err := os.Lstat(filepath.Join(f.dir, "probe", ProbeTmpDir))
	if err != nil || !st.IsDir() || st.Mode().Perm() != 0o700 {
		t.Fatalf("%v %v", st, err)
	}
	if _, err := f.run(probeEnv0, "sed"); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(f.dir, "probe", ProbeConfigFile))
	cfg, err := sandbox.ParseConfig(data)
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Tmpdir == f.base.Tmpdir || cfg.Tmpdir != filepath.Join(f.dir, "probe", ProbeTmpDir) {
		t.Fatalf("the probes share the steps' tmpdir: %q", cfg.Tmpdir)
	}
	// An existing tmpdir is refused (it must be fresh).
	g := newProberFix(t)
	if err := os.MkdirAll(filepath.Join(g.dir, "probe", ProbeTmpDir), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := g.p.Arm(g.base); err == nil {
		t.Error("a pre-existing probe tmpdir")
	}
}
