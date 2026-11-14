package record

// The toolchain probes, run in a sandbox (KILN04 section 4.8, decisions P7 and
// P14, stage 4 PR 2b-2c-3)
// ===========================================================================
//
// A toolchain entry in `toolchains.json` pins a tool's version and says how to
// ask the tool (`probes`: `sed --version`). `kiln legacy-plan --probe` runs
// those on the host. The recorder must not: `prepare` runs on a checkout that
// is the PR's, and a probe is a program that a tool root names, so running it on
// the HOST gives that program the runner. Worse, `legacy.Toolchains.Probe` with
// a nil `Run` does exactly that (it falls back to `execProbe`). This file is the
// runner the recorder passes, and `prepare` NEVER leaves it nil:
//
//	legacy.Toolchains.Probe(names, "linux", ProbeOptions{..., Run: prober.Run})
//	     |   resolves each probe's binary on the fixed PATH, refuses one inside
//	     |   the checkout, then asks Run to execute it
//	     v
//	toolchainProber.Run          (here)
//	     |
//	     v   no strace: a probe has nothing to record, only a version to print
//	kiln sandbox-exec --config <dir>/probe/toolchains-sandbox.json
//	                  --config-sha256 <H> -- <argv0> <args...>
//
// The probe's own configuration
// -----------------------------
// It is the sealed configuration with every place a step could read or write
// taken away, because a probe needs only the read-only roots:
//
//	repo_root    <dir>/probe/root, an EMPTY directory (with the working directory
//	             <dir>/probe/root/p): the checkout is not visible at all
//	home, tmpdir, ro_roots, kiln_exe    as in pre/sandbox.json
//	state_roots  none          masks  none
//	env_by_action  {"p": [PATH=<the probe PATH>, LANG=C, LC_ALL=C, TZ=UTC, HOME=<home>]}
//
// The environment is the one `legacy.Toolchains.Probe` builds, with its HOME (a
// throw-away directory on the host) replaced by the sandbox's own HOME. The
// configuration is written when the FIRST probe runs, because that is when its
// environment is known; every later call must bring the same environment, or
// the prober refuses (an environment that changed between two probes is a bug,
// not a probe).
//
// The process that starts the sandbox gets `KILN_ACTION=p` and the recorder's
// safe PATH and nothing else; stdin is /dev/null; stdout and stderr are ONE
// stream, capped at the probe's limit.
//
// What a failure means (fail closed everywhere)
// ---------------------------------------------
//
//	the probe printed the pin, exit 0                ok
//	any other exit, a timeout, exit 125              an error returned WITH the
//	  (the sandbox refused to start the probe)       output: legacy.Probe judges a
//	                                                 failed run a MISMATCH, which
//	                                                 gates the packages using the
//	                                                 tool (they still run, and are
//	                                                 never emitted)
//	the sandbox could not be started at all, the     a FATAL error: Run returns it
//	  configuration was refused, the prober was      and keeps it, and `prepare`
//	  not armed, the environment changed             fails when it sees Err()
//
// The first group says something about the TOOL; the second says something about
// the RECORDER, and a recorder that cannot run its probes must not seal "no
// probes ran" as if it had. `legacy.Probe` turns every runner error into a
// mismatch, so the fatal error is remembered here and `prepare` checks Err()
// after the probes.
//
// The deadline is the limit's: 60 s per probe and 10 min for all of them
// (decision D8), given to `legacy.Probe` as ProbeOptions.Timeout and Total.
// The sandbox root is killed at the deadline (its own process group, and
// Pdeathsig so it cannot outlive Kiln); its init, the last process of the
// sandbox's PID namespace, takes every other process with it.

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"slices"
	"strings"
	"sync"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// Names inside the probe directory.
const (
	// ProbeRootDir is the empty repo_root of the toolchain probes' sandbox, and
	// ProbeActionDir the package directory beneath it (the cwd of every probe).
	ProbeRootDir   = "root"
	ProbeActionDir = "p"
	// ProbeConfigFile is the probes' own sandbox configuration, in <dir>/probe.
	ProbeConfigFile = "toolchains-sandbox.json"
	// probeAction is the KILN_ACTION of every toolchain probe.
	probeAction = "p"
)

// probeCmd is one process the prober starts.
type probeCmd struct {
	Path    string
	Args    []string
	Env     []string
	Dir     string
	Timeout time.Duration
	Limit   int
}

// probeExec starts a process and reports its combined output (capped), its exit
// status and whether the deadline killed it. err is set only when the process
// could not be started or waited for at all.
type probeExec func(probeCmd) (out []byte, exit int, timedOut bool, err error)

// toolchainProber is a legacy.ProbeRunner that runs every probe in the sandbox.
type toolchainProber struct {
	dir     string // the record directory
	kilnExe string
	path    string // the recorder's safe PATH, for the process that starts the sandbox
	exec    probeExec

	mu    sync.Mutex
	base  *sandbox.Config // pre/sandbox.json; nil until armed
	cfg   *sandbox.Config // the probes' own, built on the first run
	sha   string
	env   []string
	fatal error
}

// newToolchainProber makes a prober that is NOT yet armed: Run refuses until
// Arm has been called, so a Traced built before the sandbox configuration exists
// cannot run a probe by accident.
func newToolchainProber(dir, kilnExe, safePath string) *toolchainProber {
	return &toolchainProber{dir: dir, kilnExe: kilnExe, path: safePath, exec: realProbeExec}
}

func (p *toolchainProber) probeDir() string   { return filepath.Join(p.dir, ProbeDir) }
func (p *toolchainProber) configPath() string { return filepath.Join(p.probeDir(), ProbeConfigFile) }

// Arm gives the prober the sealed configuration (its home, tmpdir, read-only
// roots and kiln_exe) and makes the empty checkout the probes' sandbox uses.
func (p *toolchainProber) Arm(base *sandbox.Config) error {
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.base != nil {
		return errors.New("toolchain probes: armed twice")
	}
	if base == nil {
		return errors.New("toolchain probes: no sandbox configuration to build on")
	}
	if err := mkdirReal(p.probeDir()); err != nil {
		return err
	}
	root := filepath.Join(p.probeDir(), ProbeRootDir)
	if err := mkdirNew(root); err != nil {
		return err
	}
	if err := mkdirNew(filepath.Join(root, ProbeActionDir)); err != nil {
		return err
	}
	p.base = base
	return nil
}

// mkdirReal makes dir (0700), accepting a directory that is already there as
// long as it is a real one (the structure probe makes <dir>/probe too).
func mkdirReal(dir string) error {
	if err := os.Mkdir(dir, 0o700); err == nil {
		return nil
	} else if !os.IsExist(err) {
		return fmt.Errorf("toolchain probes: %w", err)
	}
	st, err := os.Lstat(dir)
	if err != nil || !st.IsDir() {
		return fmt.Errorf("toolchain probes: %q is not a real directory", printable(dir))
	}
	return nil
}

// mkdirNew makes a directory that must not exist yet.
func mkdirNew(dir string) error {
	if err := os.Mkdir(dir, 0o700); err != nil {
		return fmt.Errorf("toolchain probes: %w", err)
	}
	return nil
}

// Err is the first fatal error a run hit, or nil. `prepare` checks it after the
// probes, because legacy.Probe judges any runner error a mismatch.
func (p *toolchainProber) Err() error {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.fatal
}

// fail records a fatal error and returns it.
func (p *toolchainProber) fail(err error) ([]byte, error) {
	if p.fatal == nil {
		p.fatal = err
	}
	return nil, err
}

// Run is the legacy.ProbeRunner.
func (p *toolchainProber) Run(r legacy.ProbeRun) ([]byte, error) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.base == nil {
		return p.fail(errors.New("toolchain probes: a probe was run before the sandbox configuration was ready"))
	}
	if r.Argv0 == "" || strings.ContainsRune(r.Argv0, '/') {
		return p.fail(fmt.Errorf("toolchain probes: probe name %q is not a plain command name", printable(r.Argv0)))
	}
	env, err := p.probeEnv(r.Env)
	if err != nil {
		return p.fail(err)
	}
	if p.cfg == nil {
		if err := p.writeConfig(env); err != nil {
			return p.fail(err)
		}
	} else if !slices.Equal(env, p.env) {
		return p.fail(errors.New("toolchain probes: the probe environment changed between two probes"))
	}
	args := []string{sandbox.ExecCommand, "--config", p.configPath(), "--config-sha256", p.sha, "--", r.Argv0}
	args = append(args, r.Args...)
	cmd := probeCmd{
		Path: p.kilnExe, Args: args, Env: []string{sandbox.ActionEnv + "=" + probeAction, "PATH=" + p.path},
		Dir: p.cfg.PackageDir(probeAction), Timeout: r.Timeout, Limit: r.Limit,
	}
	out, exit, timedOut, err := p.exec(cmd)
	switch {
	case err != nil:
		return p.fail(fmt.Errorf("toolchain probes: could not run %s: %w", r.Argv0, err))
	case timedOut:
		return out, fmt.Errorf("the probe did not finish in %s", r.Timeout)
	case exit == sandbox.ExitFailure:
		// The sandbox itself refused (a root it could not bind, a binary it
		// could not exec): the probe proved nothing, which is a mismatch.
		return out, fmt.Errorf("the sandbox refused the probe (exit %d)", exit)
	case exit != 0:
		return out, fmt.Errorf("the probe exited %d", exit)
	}
	return out, nil
}

// probeEnv is the probe's environment with HOME replaced by the sandbox's.
func (p *toolchainProber) probeEnv(in []string) ([]string, error) {
	out := make([]string, 0, len(in))
	sawHome := false
	for _, e := range in {
		if strings.HasPrefix(e, "HOME=") {
			sawHome = true
			e = "HOME=" + p.base.Home
		}
		out = append(out, e)
	}
	if !sawHome {
		return nil, errors.New("toolchain probes: the probe environment has no HOME")
	}
	return out, nil
}

// writeConfig builds, validates and writes the probes' sandbox configuration.
func (p *toolchainProber) writeConfig(env []string) error {
	cfg := &sandbox.Config{
		RepoRoot: filepath.Join(p.probeDir(), ProbeRootDir),
		Home:     p.base.Home, Tmpdir: p.base.Tmpdir, KilnExe: p.base.KilnExe,
		RoRoots:     append([]string{}, p.base.RoRoots...),
		StateRoots:  []string{},
		Masks:       []string{},
		EnvByAction: map[string][]string{probeAction: append([]string{}, env...)},
	}
	if cfg.KilnExe != p.kilnExe {
		return fmt.Errorf("toolchain probes: the configuration names kiln %q, not %q", printable(cfg.KilnExe), printable(p.kilnExe))
	}
	data, err := MarshalConfig(cfg)
	if err != nil {
		return err
	}
	if _, err := sandbox.ParseConfig(data); err != nil {
		return fmt.Errorf("toolchain probes: the configuration does not parse: %w", err)
	}
	f, err := os.OpenFile(p.configPath(), os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return fmt.Errorf("toolchain probes: %w", err)
	}
	if _, err := f.Write(data); err != nil {
		f.Close()
		return fmt.Errorf("toolchain probes: %w", err)
	}
	if err := f.Close(); err != nil {
		return fmt.Errorf("toolchain probes: %w", err)
	}
	// The sandbox's own host check, on the file as written, so a configuration
	// the helper would refuse fails `prepare` here with the rule's own message.
	err = cfg.CheckHost(probeAction, sandbox.HostFacts{Exe: p.kilnExe, Cwd: cfg.PackageDir(probeAction), ConfigPath: p.configPath()})
	if err != nil {
		return fmt.Errorf("toolchain probes: %w", err)
	}
	sum := sha256.Sum256(data)
	p.cfg, p.sha, p.env = cfg, hex.EncodeToString(sum[:]), append([]string{}, env...)
	return nil
}

// realProbeExec starts the process for real. The child leads its own process
// group and dies with this process (see isolateProbe); at the deadline it is
// killed, and a descendant that kept the output pipe open is not waited for.
func realProbeExec(c probeCmd) ([]byte, int, bool, error) {
	// Pdeathsig is tied to the thread that forked the child.
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	ctx, cancel := context.WithTimeout(context.Background(), c.Timeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, c.Path, c.Args...)
	cmd.Env, cmd.Dir = c.Env, c.Dir
	cmd.Stdin = nil // /dev/null
	out := &limitedBuffer{limit: c.Limit}
	cmd.Stdout, cmd.Stderr = out, out
	cmd.WaitDelay = 5 * time.Second
	isolateProbe(cmd)
	err := cmd.Run()
	timedOut := errors.Is(ctx.Err(), context.DeadlineExceeded)
	var ee *exec.ExitError
	switch {
	case err == nil:
		return out.b.Bytes(), 0, timedOut, nil
	case errors.As(err, &ee):
		return out.b.Bytes(), ee.ExitCode(), timedOut, nil // -1: killed by a signal
	case timedOut:
		return out.b.Bytes(), -1, true, nil // killed, and a descendant held the pipe
	}
	return nil, 0, false, err
}

// limitedBuffer keeps the first limit bytes and drops the rest.
type limitedBuffer struct {
	b     bytes.Buffer
	limit int
}

func (l *limitedBuffer) Write(p []byte) (int, error) {
	if room := l.limit - l.b.Len(); room > 0 {
		l.b.Write(p[:min(room, len(p))])
	}
	return len(p), nil
}
