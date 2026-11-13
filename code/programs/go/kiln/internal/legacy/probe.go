package legacy

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strings"
	"time"
	"unicode"
)

// Probe limits (KILN03 4).
const (
	ProbeTimeout     = 10 * time.Second
	ProbeOutputLimit = 64 << 10
	// ProbeTotalTimeout bounds ALL probes of one plan, so a PR that makes
	// every probe slow cannot stall planning for 10 s times the probe count.
	ProbeTotalTimeout = 120 * time.Second
)

// Probe statuses.  Only ProbeMismatch is a finding; the others say why a probe
// proved nothing.  An absent tool is not an error: its identity still comes
// from the pin.
const (
	ProbeOK       = "ok"       // ran; the pin was found as a whole token, or there is nothing to compare
	ProbeAbsent   = "absent"   // no binary on the fixed PATH
	ProbeRefused  = "refused"  // the binary resolves inside the checkout, so the PR would choose what runs
	ProbeMismatch = "mismatch" // ran (or failed to or exited non-zero), and the output does not carry the pin
	ProbeTimedOut = "timeout"  // the overall deadline passed before this probe ran
)

// ProbeRun is one probe execution: no shell, an explicit environment, a
// private working directory, a timeout and an output cap.
type ProbeRun struct {
	Path    string   // the real path, symlinks resolved
	Argv0   string   // the name the PATH gave, which multi-call binaries read
	Args    []string // without argv[0]
	Env     []string
	Dir     string
	Timeout time.Duration
	Limit   int
}

// ProbeRunner runs a probe and returns its stdout and stderr, capped.  A
// non-zero exit is not itself an error: the output is judged.  Tests inject
// their own.
type ProbeRunner func(ProbeRun) ([]byte, error)

// ProbeOptions turn probing on (`--probe`).
type ProbeOptions struct {
	Root string      // the checkout; a binary that resolves inside it is refused
	Home string      // expands $HOME roots on the PATH; "" or a non-absolute home drops them
	Run  ProbeRunner // nil: execute for real
	// Total bounds all probes together; 0 means ProbeTotalTimeout.
	Total time.Duration
}

// ProbeResult is the outcome of one tool's probes (the worst of them, when a
// tool has several).
type ProbeResult struct {
	Tool   string `json:"tool"`
	Status string `json:"status"`
	// Detail keeps a short normalised excerpt of the output of a mismatch,
	// for the report.
	Detail string `json:"detail,omitempty"`
}

// capWriter keeps at most limit bytes and silently discards the rest, so a
// chatty tool cannot exhaust memory.
type capWriter struct {
	buf   bytes.Buffer
	limit int
}

func (w *capWriter) Write(p []byte) (int, error) {
	if room := w.limit - w.buf.Len(); room > 0 {
		w.buf.Write(p[:min(room, len(p))])
	}
	return len(p), nil
}

// execProbe is the real ProbeRunner.  On Unix the child leads its own process
// group and a timeout kills the whole group, so a probe cannot leave
// grandchildren running.  A non-zero exit is returned as an error beside the
// output, which is kept for the report.
func execProbe(r ProbeRun) ([]byte, error) {
	ctx, cancel := context.WithTimeout(context.Background(), r.Timeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, r.Path, r.Args...)
	if r.Argv0 != "" {
		cmd.Args[0] = r.Argv0
	}
	cmd.Env = r.Env
	cmd.Dir = r.Dir
	cmd.Stdin = nil
	out := &capWriter{limit: r.Limit}
	cmd.Stdout, cmd.Stderr = out, out
	cmd.WaitDelay = time.Second
	isolate(cmd)
	err := cmd.Run()
	return out.buf.Bytes(), err
}

// lookPath finds name in dirs the way a shell would, with nothing but those
// directories: a regular, executable file; `.exe` on Windows.
func lookPath(name string, dirs []string) (string, bool) {
	for _, d := range dirs {
		cand := filepath.Join(d, name)
		if runtime.GOOS == "windows" {
			cand += ".exe"
		}
		fi, err := os.Stat(cand)
		if err == nil && fi.Mode().IsRegular() && (runtime.GOOS == "windows" || fi.Mode()&0o111 != 0) {
			return cand, true
		}
	}
	return "", false
}

// inside reports whether path lies under root.  It FAILS CLOSED: a path that
// cannot be related to root (a relative one, say) counts as inside, so the
// binary is refused rather than run.  On macOS and Windows, whose file systems
// are case-insensitive by default, the comparison folds case.
func inside(path, root string) bool {
	if runtime.GOOS == "darwin" || runtime.GOOS == "windows" {
		path, root = strings.ToLower(path), strings.ToLower(root)
	}
	rel, err := filepath.Rel(root, path)
	if err != nil {
		return true
	}
	return !(rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) || filepath.IsAbs(rel))
}

var (
	pathRe = regexp.MustCompile(`(^|[\s"'(=:])(?:[A-Za-z]:[\\/]|[\\/])[^\s"'()<>,;]*`)
	timeRe = regexp.MustCompile(`\d{4}-\d{2}-\d{2}([T ]\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?)?|\d{1,2}:\d{2}:\d{2}`)
)

// normalizeProbe makes probe output comparable across machines: absolute
// paths and timestamps are removed (a version mentioned only inside a path is
// not a claim about the tool), control characters become spaces.  Paths are
// recognised at a token start, so `linux/amd64` keeps its shape.
func normalizeProbe(out []byte) string {
	s := strings.Map(func(r rune) rune {
		if unicode.IsControl(r) && r != '\n' && r != '\t' {
			return ' '
		}
		return r
	}, string(out))
	s = pathRe.ReplaceAllString(s, "$1")
	return timeRe.ReplaceAllString(s, " ")
}

// probeMatches reports whether output carries pin as a WHOLE token (KILN03 4),
// so `1.83.0` does not match `11.83.01` or `1.83.0-nightly`.  Tokens split at
// white space and `,;()[]"'<>`; a token may carry a leading letters-only prefix
// (`v22.1.0`, `go1.26.1`), which is cut before the comparison.
func probeMatches(output, pin string) bool {
	tokens := strings.FieldsFunc(normalizeProbe([]byte(output)), func(r rune) bool {
		return unicode.IsSpace(r) || strings.ContainsRune(",;()[]\"'<>", r)
	})
	for _, t := range tokens {
		t = strings.TrimRight(t, ".:")
		if i := strings.IndexFunc(t, func(r rune) bool { return !unicode.IsLetter(r) }); i > 0 {
			if t[i:] == pin {
				return true
			}
		}
		if t == pin {
			return true
		}
	}
	return false
}

// severity orders statuses so the worst of a tool's probes is reported.
func severity(s string) int {
	switch s {
	case ProbeTimedOut:
		return 4
	case ProbeMismatch:
		return 3
	case ProbeRefused:
		return 2
	case ProbeAbsent:
		return 1
	}
	return 0
}

// Probe runs the probes of the entries a plan uses (KILN03 4) and reports one
// result per probed tool, sorted by name.  Each probe runs with no shell, an
// environment of exactly a fixed PATH (the toolchain directories of the used
// tools, ConstructPATH), `LANG=C`, `TZ=UTC` and a private HOME and working
// directory, a 10 s timeout and 64 KiB of output.  A binary whose real path
// (after symlinks) lies inside opt.Root is never run.
func (t *Toolchains) Probe(names []string, osName string, opt ProbeOptions) (res []ProbeResult, err error) {
	run := opt.Run
	if run == nil {
		run = execProbe
	}
	// Without a checkout root the "no binary from the PR" rule cannot be
	// judged, so there is no way to proceed: fail rather than run unchecked.
	if opt.Root == "" {
		return nil, fmt.Errorf("probe: the checkout root is required")
	}
	r, err := filepath.EvalSymlinks(opt.Root)
	if err != nil {
		return nil, fmt.Errorf("probe: %w", err)
	}
	root, err := filepath.Abs(r)
	if err != nil {
		return nil, fmt.Errorf("probe: %w", err)
	}
	used, _ := t.selected(names, osName)
	tools, _ := t.ToolsFor(names, osName, "")
	pathVar := t.ConstructPATH(tools, opt.Home) // drops a home that is not absolute and clean
	dirs := filepath.SplitList(pathVar)
	dir, err := os.MkdirTemp("", "kiln-probe-")
	if err != nil {
		return nil, fmt.Errorf("probe: %w", err)
	}
	defer func() {
		if rmErr := removeAll(dir); rmErr != nil && err == nil {
			res, err = nil, fmt.Errorf("probe: removing its working directory: %w", rmErr)
		}
	}()
	env := []string{"PATH=" + pathVar, "LANG=C", "LC_ALL=C", "TZ=UTC", "HOME=" + dir}
	total := opt.Total
	if total <= 0 {
		total = ProbeTotalTimeout
	}
	start := time.Now()

	worst := map[string]string{}
	detail := map[string]string{}
	note := func(tool, status string) {
		if cur, ok := worst[tool]; !ok || severity(status) > severity(cur) {
			worst[tool] = status
		}
	}
	for _, e := range used {
		for _, argv := range e.Probes {
			tool := argv[0]
			if time.Since(start) > total {
				note(tool, ProbeTimedOut)
				continue
			}
			bin, ok := lookPath(tool, dirs)
			if !ok {
				note(tool, ProbeAbsent)
				continue
			}
			real, err := filepath.EvalSymlinks(bin)
			if err == nil {
				real, err = filepath.Abs(real)
			}
			if err != nil {
				note(tool, ProbeAbsent) // a dangling link: nothing to run
				continue
			}
			if inside(real, root) {
				note(tool, ProbeRefused)
				continue
			}
			// Run the path that was checked, under the name the PATH gave:
			// multi-call binaries (rustup proxies, busybox) read argv[0].
			out, runErr := run(ProbeRun{Path: real, Argv0: filepath.Base(bin), Args: argv[1:], Env: env, Dir: dir, Timeout: ProbeTimeout, Limit: ProbeOutputLimit})
			pin, versioned := e.Pins[tool]
			if !versioned || pin == ImagePin {
				note(tool, ProbeOK)
				continue
			}
			// A failed run is a mismatch even when the output carries the pin.
			if runErr == nil && probeMatches(string(out), pin) {
				note(tool, ProbeOK)
				continue
			}
			note(tool, ProbeMismatch)
			if _, seen := detail[tool]; !seen {
				// The run error can name the runner's paths or user; it goes
				// through the same normalisation as the output.
				d := string(out)
				if runErr != nil {
					d = "(" + runErr.Error() + ") " + d
				}
				d = strings.Join(strings.Fields(normalizeProbe([]byte(d))), " ")
				detail[tool] = short(d)
			}
		}
	}
	res = make([]ProbeResult, 0, len(worst))
	for tool, s := range worst {
		r := ProbeResult{Tool: tool, Status: s}
		if s == ProbeMismatch {
			r.Detail = detail[tool]
		}
		res = append(res, r)
	}
	sort.Slice(res, func(a, b int) bool { return res[a].Tool < res[b].Tool })
	return res, nil
}

// removeAll is os.RemoveAll; tests replace it to see the error path.
var removeAll = os.RemoveAll
