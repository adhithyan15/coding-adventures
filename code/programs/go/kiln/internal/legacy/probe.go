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
)

// Probe statuses.  Only ProbeMismatch is a finding; the others say why a probe
// proved nothing.  An absent tool is not an error: its identity still comes
// from the pin.
const (
	ProbeOK       = "ok"       // ran; the pin was found as a whole token, or there is nothing to compare
	ProbeAbsent   = "absent"   // no binary on the fixed PATH
	ProbeRefused  = "refused"  // the binary resolves inside the checkout, so the PR would choose what runs
	ProbeMismatch = "mismatch" // ran (or failed to), and the output does not carry the pin
)

// ProbeRun is one probe execution: no shell, an explicit environment, a
// private working directory, a timeout and an output cap.
type ProbeRun struct {
	Path    string
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
	Home string      // expands $HOME roots on the PATH; "" drops them
	Run  ProbeRunner // nil: execute for real
}

// ProbeResult is the outcome of one tool's probes (the worst of them, when a
// tool has several).
type ProbeResult struct {
	Tool   string `json:"tool"`
	Status string `json:"status"`
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

// execProbe is the real ProbeRunner.
func execProbe(r ProbeRun) ([]byte, error) {
	ctx, cancel := context.WithTimeout(context.Background(), r.Timeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, r.Path, r.Args...)
	cmd.Env = r.Env
	cmd.Dir = r.Dir
	cmd.Stdin = nil
	out := &capWriter{limit: r.Limit}
	cmd.Stdout, cmd.Stderr = out, out
	cmd.WaitDelay = time.Second
	err := cmd.Run()
	if _, exit := err.(*exec.ExitError); exit && ctx.Err() == nil {
		err = nil // a non-zero exit still printed its version, maybe
	}
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

// inside reports whether path lies under root (both symlink-resolved).
func inside(path, root string) bool {
	rel, err := filepath.Rel(root, path)
	return err == nil && rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator)) && !filepath.IsAbs(rel)
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
func (t *Toolchains) Probe(names []string, osName string, opt ProbeOptions) ([]ProbeResult, error) {
	run := opt.Run
	if run == nil {
		run = execProbe
	}
	root := opt.Root
	if root != "" {
		r, err := filepath.EvalSymlinks(root)
		if err != nil {
			return nil, fmt.Errorf("probe: %w", err)
		}
		if root, err = filepath.Abs(r); err != nil {
			return nil, fmt.Errorf("probe: %w", err)
		}
	}
	used, _ := t.selected(names, osName)
	tools, _ := t.ToolsFor(names, osName, "")
	pathVar := t.ConstructPATH(tools, opt.Home)
	dirs := filepath.SplitList(pathVar)
	dir, err := os.MkdirTemp("", "kiln-probe-")
	if err != nil {
		return nil, fmt.Errorf("probe: %w", err)
	}
	defer os.RemoveAll(dir)
	env := []string{"PATH=" + pathVar, "LANG=C", "LC_ALL=C", "TZ=UTC", "HOME=" + dir}

	worst := map[string]string{}
	note := func(tool, status string) {
		if cur, ok := worst[tool]; !ok || severity(status) > severity(cur) {
			worst[tool] = status
		}
	}
	for _, e := range used {
		for _, argv := range e.Probes {
			tool := argv[0]
			bin, ok := lookPath(tool, dirs)
			if !ok {
				note(tool, ProbeAbsent)
				continue
			}
			// Judge where the binary really lives, but execute it under the name
			// the PATH gave: multi-call binaries (rustup proxies, busybox) pick
			// their behaviour from argv[0].
			real, err := filepath.EvalSymlinks(bin)
			if err != nil {
				note(tool, ProbeAbsent) // a dangling link: nothing to run
				continue
			}
			if root != "" && inside(real, root) {
				note(tool, ProbeRefused)
				continue
			}
			out, runErr := run(ProbeRun{Path: bin, Args: argv[1:], Env: env, Dir: dir, Timeout: ProbeTimeout, Limit: ProbeOutputLimit})
			pin, versioned := e.Pins[tool]
			if !versioned || pin == ImagePin {
				note(tool, ProbeOK)
				continue
			}
			if runErr == nil && probeMatches(string(out), pin) {
				note(tool, ProbeOK)
			} else {
				note(tool, ProbeMismatch)
			}
		}
	}
	res := make([]ProbeResult, 0, len(worst))
	for tool, s := range worst {
		res = append(res, ProbeResult{Tool: tool, Status: s})
	}
	sort.Slice(res, func(a, b int) bool { return res[a].Tool < res[b].Tool })
	return res, nil
}
