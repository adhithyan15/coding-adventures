package record

// Running the structure probes (decision P14; the decisions of what they must
// show are in probe.go)
// ====================================================================
//
// Each probe is one start of the real thing, built the way `kiln record step`
// builds a step (step.go), so what it proves is true of a step:
//
//	strace <StraceArgs, -o /proc/self/fd/5> \
//	   kiln sandbox-exec --config <dir>/pre/sandbox.json --config-sha256 <H> \
//	        --stdout-fd 3 --stderr-fd 4 -- <probe argv>
//
//	cwd   the package directory of the shard's first action (CheckHost has just
//	      proved the sandbox accepts exactly that)
//	env   KILN_ACTION, KILN_STEP=0, KILN_STEP_COUNT=1, the recorded PATH and the
//	      validated LD_LIBRARY_PATH, as stepProcessEnv gives a step
//
// The trace pipe, the two output pipes and strace's own two streams are the
// five pipes runStrace wires; the trace goes to `<dir>/probe/<name>.trace`, a
// file created O_EXCL in a directory made 0700 for the purpose. Nothing here is
// sealed, and `collect` never reads `probe/`.
//
// A probe runs under a deadline (decision D8): one minute, and ten for all of
// them. At the deadline strace is killed (and `--kill-on-exit` takes every
// tracee with it), and the probe fails.
//
// After the four runs the places a step could have written must still be empty:
// tmpdir, and every state root (absent or an empty directory). A probe writes
// nothing, so anything there means the host was not what the scan said, and the
// shard is `unscrubbed`.

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// probeLimits are the parser's limits for a probe trace: its cap is 64 MiB.
func probeLimits() trace.Limits {
	return trace.Limits{MaxLineBytes: 1 << 20, MaxTotalBytes: ProbeTraceCap, MaxEvents: 1 << 20}
}

// RunStructureProbes is the real Options.Structure.
func RunStructureProbes(in ProbeInput) (ProbeReport, error) {
	rep := ProbeReport{Schema: ProbesSchema, Action: in.Action, Shadowed: []string{}, Runs: []ProbeRunRecord{}, Toolchains: nil}
	if in.Action == "" {
		return rep, nil // a shard with no action runs no step: nothing to ask the sandbox
	}
	shadowed, err := ShadowedMounts(in.Cfg)
	if err != nil {
		return ProbeReport{}, err
	}
	rep.Shadowed = shadowed
	probeDir := filepath.Join(in.Dir, ProbeDir)
	if err := os.Mkdir(probeDir, 0o700); err != nil {
		return ProbeReport{}, fmt.Errorf("create probe directory: %w", err)
	}
	start := time.Now()
	for _, spec := range ProbeSpecs(in.Cfg, shadowed) {
		left := ProbeTotalTimeout - time.Since(start)
		if left <= 0 {
			return ProbeReport{}, fmt.Errorf("the probes together took more than %s", ProbeTotalTimeout)
		}
		rec, err := runProbe(in, probeDir, spec, min(ProbeTimeout, left))
		if err != nil {
			return ProbeReport{}, err
		}
		rep.Runs = append(rep.Runs, rec)
	}
	if err := probesLeftNothing(in.Cfg); err != nil {
		return ProbeReport{}, err
	}
	return rep, nil
}

// runProbe runs one probe and applies every check to it.
func runProbe(in ProbeInput, probeDir string, spec ProbeSpec, deadline time.Duration) (ProbeRunRecord, error) {
	tracePath := filepath.Join(probeDir, spec.Name+".trace")
	f, err := os.OpenFile(tracePath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return ProbeRunRecord{}, fmt.Errorf("probe %s: the trace file: %w", spec.Name, err)
	}
	defer f.Close()
	sargs, err := sandbox.StraceArgs("/proc/self/fd/" + fmt.Sprint(traceFD))
	if err != nil {
		return ProbeRunRecord{}, err
	}
	argv := append(sargs, in.KilnExe, sandbox.ExecCommand, "--config", in.ConfigPath, "--config-sha256", in.ConfigSHA256,
		"--stdout-fd", "3", "--stderr-fd", "4", "--")
	argv = append(argv, spec.Argv...)

	stdout, stderr := &capture{}, &capture{}
	o := StepOptions{Stdout: stdout, Stderr: stderr, Deadline: deadline}
	o.applyDefaults()
	sink := &traceSink{w: f, h: sha256.New(), limit: ProbeTraceCap}
	began := time.Now()
	run := runStrace(o, in.Cfg.PackageDir(in.Action), in.Strace, argv, in.Env, sink)
	elapsed := time.Since(began)
	if err := f.Sync(); err != nil && sink.reason == "" {
		sink.reason = TruncWriteFailed
	}
	if err := f.Close(); err != nil && sink.reason == "" {
		sink.reason = TruncWriteFailed
	}
	if run.pipesHeld {
		// A relay may still be writing to the captures: do not read them.
		return ProbeRunRecord{}, fmt.Errorf("probe %s: a process outlived strace and held its pipes open (trace: %s)", spec.Name, tracePath)
	}
	obs := ProbeObservation{
		Started: run.started, TimedOut: run.timedOut, Truncated: sink.reason != "", PipesHeld: run.pipesHeld,
		StraceExit: run.code, StraceSignal: run.signal,
		StraceStdout: run.stdout.n, StraceStderr: run.stderr.n, StepStderr: stderr.n,
		StepStdout: stdout.b, StepStdoutBytes: stdout.n, Elapsed: elapsed,
	}
	// What strace itself said, for whoever has to find out why a CI run failed.
	diag := func(err error) error {
		if run.stderr.n > 0 {
			return fmt.Errorf("%w (strace said: %q; trace: %s)", err, printable(string(run.stderr.b)), tracePath)
		}
		return fmt.Errorf("%w (trace: %s)", err, tracePath)
	}
	if !run.started || run.timedOut || run.signal != 0 || run.code != 0 || sink.reason != "" {
		return ProbeRunRecord{}, diag(CheckProbeRun(spec, obs, nil))
	}
	tr, why := parseHelperTrace(tracePath, in.KilnExe, in.Cfg.PackageDir(in.Action), in.ConfigSHA256, run.code, false, probeLimits())
	if why != "" {
		return ProbeRunRecord{}, diag(fmt.Errorf("probe %s: the trace does not parse: %s", spec.Name, why))
	}
	if err := CheckProbeRun(spec, obs, tr); err != nil {
		return ProbeRunRecord{}, diag(err)
	}
	return ProbeRunRecord{Name: spec.Name, Argv: spec.Argv, Exit: run.code, TraceBytes: sink.n, TraceSHA256: hex.EncodeToString(sink.h.Sum(nil))}, nil
}

// probesLeftNothing requires tmpdir to be empty and every state root to be
// absent or an empty directory, as before the probes.
func probesLeftNothing(cfg *sandbox.Config) error {
	entries, err := os.ReadDir(cfg.Tmpdir)
	if err != nil {
		return fmt.Errorf("probes: tmpdir %q: %v", printable(cfg.Tmpdir), err)
	}
	if len(entries) != 0 {
		return fmt.Errorf("probes: unscrubbed:tmpdir-written: tmpdir %q holds %d entries after the probes, which write nothing", printable(cfg.Tmpdir), len(entries))
	}
	for _, s := range cfg.StateRoots {
		if err := stateRootEmpty(s, os.Lstat, os.ReadDir); err != nil {
			return fmt.Errorf("probes: %w", err)
		}
	}
	return nil
}
