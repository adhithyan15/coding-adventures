package record

// The in-sandbox structure probe (KILN04 section 4.8, decision P14, stage 4 PR
// 2b-2c-2)
// =====================================================================
//
// `prepare` proves several things about the sandbox from the OUTSIDE: the host
// scan (hostscan.go) walks the read-only roots on the host and the sandbox
// configuration is checked against the host. A proof from outside can still be
// wrong about what a step really SEES. A directory under a read-only root can
// be a mount point on the host, and the sandbox binds each root without its
// submounts, so a socket the host scan never entered is what the step finds
// beneath it:
//
//	host:     /usr/lib/x  is a mount point         (the host scan stops here)
//	             |-- daemon.sock   <- on the mounted filesystem
//	sandbox:  /usr is bound non-recursively; /usr/lib/x is the PLAIN directory
//	          under the mount, and what lies in THAT is what a step reaches
//
// So after the configuration is sealed, `prepare` starts the real sandbox
// four times under the real strace, exactly as `kiln record step` will, and asks
// the sandbox itself:
//
//	pipes    the step's descriptors 1 and 2 are pipes (fds 3 and 4 of strace's
//	         wiring really arrive as the step's stdout and stderr)
//	walk     no socket and no FIFO exists anywhere in the sandbox's new root
//	         outside the roots the host scan already covered (`/proc` and the
//	         read-only roots and the checkout are pruned)
//	shadow   no socket and no FIFO lies under a host mount point that is inside
//	         a read-only root, the checkout or a state root (only run when there
//	         is such a mount point: see ShadowedMounts)
//	signals  a self-check of the tracer: this strace prints "killed by" lines
//	         (signal=all) and the daemon-left rule fires for a survivor. A strace
//	         that cannot show a signal death cannot be trusted to show a daemon
//
// Each run is held to the same strictness as a recorded step, and more: no write
// event at all (a probe has nothing to write), the exact argv, the digests of
// the sealed configuration, and a trace whose init exit line is known. Any
// failure fails `prepare`, and nothing is sealed.
//
// This file is portable: the decision of what to run, what each run must show,
// and the sealed `pre/probes.json`. Starting the processes is probe_run.go.
//
// Reading the facts back
// ----------------------
// `pre/probes.json` is sealed with the rest of `pre/`: it is the PROOF that
// `prepare` ran the probes, with the digest of every trace. The traces
// themselves are in `<dir>/probe/` (never sealed, never read by `collect`,
// never uploaded): they are for the person who has to find out why a CI run
// failed.

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"path"
	"regexp"
	"strings"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// ProbesSchema is the schema of pre/probes.json; ProbesFile its name in pre/.
const (
	ProbesSchema = "kiln-record-probes/1"
	ProbesFile   = "probes.json"
	// ProbeDir is `<dir>/probe`, where the probes' traces are kept.
	ProbeDir = "probe"
)

// The probes, in the order they run.
const (
	ProbePipes   = "pipes"
	ProbeWalk    = "walk"
	ProbeShadow  = "shadow"
	ProbeSignals = "signals"
)

// The limits (decision D8 of the design): one probe may take a minute, the four
// together ten; a probe trace may not exceed 64 MiB; the signals probe is a
// sleeper that must be gone in 30 seconds.
const (
	ProbeTimeout       = 60 * time.Second
	ProbeTotalTimeout  = 10 * time.Minute
	ProbeTraceCap      = 64 << 20
	ProbeSignalsBudget = 30 * time.Second
	// MaxShadowed bounds the mount points the shadow probe is given: a host with
	// more is not one this recorder understands.
	MaxShadowed = 256
)

// ProbeSpec is one probe: its name and the argv the step is started with.
type ProbeSpec struct {
	Name string
	Argv []string
}

// ProbeRunRecord is what the sealed file says about one run.
type ProbeRunRecord struct {
	Name        string   `json:"name"`
	Argv        []string `json:"argv"`
	Exit        int      `json:"exit"`
	TraceBytes  int64    `json:"trace_bytes"`
	TraceSHA256 string   `json:"trace_sha256"`
}

// ProbeReport is pre/probes.json.
type ProbeReport struct {
	Schema string `json:"schema"`
	// Action is the action whose package directory the probes ran in: the first
	// action of the shard, or "" for a shard that runs none (then nothing ran).
	Action string `json:"action"`
	// Shadowed are the host mount points the shadow probe looked beneath.
	Shadowed []string         `json:"shadowed"`
	Runs     []ProbeRunRecord `json:"runs"`
	// Toolchains is reserved for the toolchain probes (PR 3); it is empty here,
	// and a decoder of this version refuses anything else.
	Toolchains []json.RawMessage `json:"toolchains"`
}

// ProbeInput is what the structure probe is given. The hook
// Options.Structure takes it and returns the report.
type ProbeInput struct {
	// Dir is the record directory; the traces go to <Dir>/probe.
	Dir string
	// Cfg is the sealed configuration, ConfigPath where it is and ConfigSHA256
	// its digest (the binding's).
	Cfg          *sandbox.Config
	ConfigPath   string
	ConfigSHA256 string
	// Action is the first action of the shard ("" when it has none).
	Action string
	// KilnExe and Strace are the pinned binaries; Env is the environment strace
	// is started with (stepProcessEnv).
	KilnExe, Strace string
	Env             []string
}

// ProbeSpecs are the probes for a configuration, in order. shadowed is the
// output of ShadowedMounts: the shadow probe is part of the list exactly when
// it is not empty.
func ProbeSpecs(cfg *sandbox.Config, shadowed []string) []ProbeSpec {
	// walk: everything but /proc, the read-only roots and the checkout (the host
	// scan and the inventory cover those). A glob character in a path is
	// escaped, because find's -path takes a pattern.
	walk := []string{"find", "/", "(", "-path", "/proc"}
	for _, r := range append(append([]string{}, cfg.RoRoots...), cfg.RepoRoot) {
		walk = append(walk, "-o", "-path", findEscape(r))
	}
	walk = append(walk, ")", "-prune", "-o", "(", "-type", "s", "-o", "-type", "p", ")", "-print")

	specs := []ProbeSpec{
		{ProbePipes, []string{"readlink", "/proc/self/fd/1", "/proc/self/fd/2"}},
		{ProbeWalk, walk},
	}
	if len(shadowed) > 0 {
		// The mount points are starting points, not patterns: find takes them
		// literally. -xdev keeps it on the filesystem under the mount.
		argv := append([]string{"find"}, shadowed...)
		argv = append(argv, "-xdev", "(", "-type", "s", "-o", "-type", "p", ")", "-print")
		specs = append(specs, ProbeSpec{ProbeShadow, argv})
	}
	return append(specs, ProbeSpec{ProbeSignals, []string{"sh", "-c", "sleep 300 & echo started"}})
}

// findEscape escapes the characters find's -path pattern treats as special.
func findEscape(p string) string {
	var b strings.Builder
	for i := 0; i < len(p); i++ {
		switch p[i] {
		case '\\', '*', '?', '[':
			b.WriteByte('\\')
		}
		b.WriteByte(p[i])
	}
	return b.String()
}

// ProbeObservation is what the supervisor saw of one run (probe_run.go fills
// it; the checks here are pure, so they are tested with no sandbox).
type ProbeObservation struct {
	Started, TimedOut, Truncated, PipesHeld bool
	StraceExit, StraceSignal                int
	// Bytes strace wrote to ITS stdout and stderr, and to the step's stderr.
	StraceStdout, StraceStderr, StepStderr int64
	// StepStdout is what the step wrote to its stdout (the first 4 KiB) and
	// StepStdoutBytes how many bytes it was in all.
	StepStdout      []byte
	StepStdoutBytes int64
	Elapsed         time.Duration
}

var pipeLine = regexp.MustCompile(`^pipe:\[[0-9]+\]$`)

// CheckProbeRun applies the strict checks to one run. obs is what happened and
// tr the parsed trace (nil when the trace did not parse: that is reported by
// the caller, with the parser's reason, before this is called).
func CheckProbeRun(spec ProbeSpec, obs ProbeObservation, tr *trace.Trace) error {
	fail := func(format string, a ...any) error {
		return fmt.Errorf("probe %s: %s", spec.Name, fmt.Sprintf(format, a...))
	}
	switch {
	case !obs.Started:
		return fail("strace did not start")
	case obs.TimedOut:
		return fail("it did not finish in time and was killed")
	case obs.StraceSignal != 0:
		return fail("strace was killed by signal %d", obs.StraceSignal)
	case obs.StraceExit != 0:
		return fail("strace exited %d (the step's own status is the trace's)", obs.StraceExit)
	case obs.StraceStdout != 0 || obs.StraceStderr != 0:
		return fail("strace itself wrote to its standard output or error (%d and %d bytes)", obs.StraceStdout, obs.StraceStderr)
	case obs.Truncated:
		return fail("the trace is truncated (cap %d bytes)", ProbeTraceCap)
	case obs.PipesHeld:
		return fail("a process outlived strace and held its pipes open")
	case obs.StepStderr != 0:
		return fail("the step wrote %d bytes to its standard error", obs.StepStderr)
	case tr == nil:
		return fail("there is no trace")
	}
	ev := tr.StepEvent()
	if ev == nil {
		return fail("the step's own execve is not in the trace")
	}
	if ev.Argv == nil || !sameStrings(ev.Argv, spec.Argv) {
		return fail("the step that ran is not the one asked for (its argv is not the probe's)")
	}
	if ex := tr.StepExit(); ex == nil || ex.Killed || ex.Code != 0 {
		return fail("the step did not exit 0")
	}
	for i := range tr.Events {
		if tr.Events[i].Kind == trace.KindWrite {
			e := tr.Events[i]
			return fail("a write event at trace line %d (%s %s): a probe writes nothing", e.Line, e.Syscall, printable(e.Path))
		}
	}
	if tr.InitExitLine <= 0 {
		return fail("the init's exit call is not in the trace")
	}
	return checkProbeOutput(spec.Name, obs, tr, fail)
}

// checkProbeOutput is the part of the checks that differs per probe.
func checkProbeOutput(name string, obs ProbeObservation, tr *trace.Trace, fail func(string, ...any) error) error {
	out := obs.StepStdout
	exact := obs.StepStdoutBytes == int64(len(out)) // all of it was kept
	switch name {
	case ProbePipes:
		lines := strings.Split(strings.TrimSuffix(string(out), "\n"), "\n")
		if !exact || !bytes.HasSuffix(out, []byte("\n")) || len(lines) != 2 || !pipeLine.MatchString(lines[0]) || !pipeLine.MatchString(lines[1]) {
			return fail("the step's descriptors 1 and 2 are not two pipes (it printed %q)", printable(string(out)))
		}
	case ProbeWalk, ProbeShadow:
		if obs.StepStdoutBytes != 0 {
			return fail("a socket or FIFO is reachable in the sandbox: %q", printable(firstLine(out)))
		}
	case ProbeSignals:
		if !exact || string(out) != "started\n" {
			return fail("the sleeper did not report it started (it printed %q)", printable(string(out)))
		}
		if !traced.DaemonLeft(tr) {
			return fail("the survivor was not seen: this strace does not show the deaths of the processes left behind")
		}
		if obs.Elapsed >= ProbeSignalsBudget {
			return fail("it took %s: the sandbox did not kill the survivor when the step ended", obs.Elapsed.Round(time.Second))
		}
	default:
		return fail("unknown probe")
	}
	return nil
}

func firstLine(b []byte) string {
	if i := bytes.IndexByte(b, '\n'); i >= 0 {
		b = b[:i]
	}
	if len(b) > 200 {
		b = b[:200]
	}
	return string(b)
}

func sameStrings(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

// CheckProbeReport requires a report to be exactly what ProbeSpecs says would
// have run for cfg: the action, the list of runs in order, each argv, an exit of
// 0, and a trace of a plausible size with a SHA-256. The sealed file is checked
// with it by `prepare` (what the hook returned) and by VerifyPre (what was
// read back), so a hand-edited probes.json that dropped a probe is refused.
func CheckProbeReport(cfg *sandbox.Config, rep ProbeReport, actions []string) error {
	if rep.Schema != ProbesSchema {
		return fmt.Errorf("probes: schema %q", printable(rep.Schema))
	}
	if len(rep.Toolchains) != 0 {
		return fmt.Errorf("probes: toolchains must be empty in this version")
	}
	if len(rep.Shadowed) > MaxShadowed {
		return fmt.Errorf("probes: %d shadowed mount points (at most %d)", len(rep.Shadowed), MaxShadowed)
	}
	for i, m := range rep.Shadowed {
		if !path.IsAbs(m) || path.Clean(m) != m || m == "/" || strings.HasPrefix(m, "-") || printable(m) != m || (i > 0 && m <= rep.Shadowed[i-1]) {
			return fmt.Errorf("probes: shadowed mount point %q is not an absolute, clean path in strict order", printable(m))
		}
	}
	if rep.Action == "" {
		if len(actions) != 0 || len(rep.Runs) != 0 || len(rep.Shadowed) != 0 {
			return fmt.Errorf("probes: no action, but the shard runs %d and the report holds runs", len(actions))
		}
		return nil
	}
	if err := recfmt.CheckPath(rep.Action); err != nil {
		return fmt.Errorf("probes: action: %v", err)
	}
	known := false
	for _, a := range actions {
		known = known || a == rep.Action
	}
	if !known {
		return fmt.Errorf("probes: action %q is not an action of this shard", printable(rep.Action))
	}
	specs := ProbeSpecs(cfg, rep.Shadowed)
	if len(rep.Runs) != len(specs) {
		return fmt.Errorf("probes: %d runs, want %d", len(rep.Runs), len(specs))
	}
	for i, sp := range specs {
		r := rep.Runs[i]
		switch {
		case r.Name != sp.Name:
			return fmt.Errorf("probes: run %d is %q, want %q", i, printable(r.Name), sp.Name)
		case !sameStrings(r.Argv, sp.Argv):
			return fmt.Errorf("probes: run %q did not use the probe's argv", sp.Name)
		case r.Exit != 0:
			return fmt.Errorf("probes: run %q exited %d", sp.Name, r.Exit)
		case r.TraceBytes <= 0 || r.TraceBytes > ProbeTraceCap:
			return fmt.Errorf("probes: run %q has a trace of %d bytes", sp.Name, r.TraceBytes)
		case !hexDigest(r.TraceSHA256):
			return fmt.Errorf("probes: run %q has no trace digest", sp.Name)
		}
	}
	return nil
}

// EncodeProbes renders the report the way every pre/ file is rendered: compact,
// no HTML escaping, no trailing newline, no null list.
func EncodeProbes(rep ProbeReport) ([]byte, error) {
	if rep.Shadowed == nil {
		rep.Shadowed = []string{}
	}
	if rep.Runs == nil {
		rep.Runs = []ProbeRunRecord{}
	}
	if rep.Toolchains == nil {
		rep.Toolchains = []json.RawMessage{}
	}
	for i := range rep.Runs {
		if rep.Runs[i].Argv == nil {
			rep.Runs[i].Argv = []string{}
		}
	}
	return marshalJSON(rep)
}

// DecodeProbes reads pre/probes.json strictly: no unknown key, no trailing
// data, and the bytes must be exactly what EncodeProbes writes for what they
// say (so a reordered, indented or null-for-empty file is refused).
func DecodeProbes(data []byte) (ProbeReport, error) {
	var rep ProbeReport
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&rep); err != nil {
		return ProbeReport{}, fmt.Errorf("probes: %w", err)
	}
	if _, err := dec.Token(); err != io.EOF {
		return ProbeReport{}, fmt.Errorf("probes: unexpected data after the JSON value")
	}
	again, err := EncodeProbes(rep)
	if err != nil {
		return ProbeReport{}, err
	}
	if !bytes.Equal(again, data) {
		return ProbeReport{}, fmt.Errorf("probes: not in the canonical form EncodeProbes writes")
	}
	return rep, nil
}
