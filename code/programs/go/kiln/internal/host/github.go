package host

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"runtime"
	"strings"
)

// GitHub is a GitHub Actions runner.  All inputs come from environment
// variables read through the injected Getenv.
type GitHub struct {
	getenv Getenv
	out    io.Writer
	scratch
}

// NewGitHub returns a GitHub Actions host writing workflow commands to out.
func NewGitHub(getenv Getenv, out io.Writer) *GitHub { return &GitHub{getenv: getenv, out: out} }

// Identity reads GITHUB_SHA, the branch (GITHUB_HEAD_REF on pull requests,
// else GITHUB_REF_NAME), GITHUB_EVENT_NAME and GITHUB_RUN_ID.
func (g *GitHub) Identity() Identity {
	branch := g.getenv("GITHUB_HEAD_REF")
	if branch == "" {
		branch = g.getenv("GITHUB_REF_NAME")
	}
	return Identity{
		Revision: g.getenv("GITHUB_SHA"),
		Branch:   branch,
		Event:    g.getenv("GITHUB_EVENT_NAME"),
		RunID:    g.getenv("GITHUB_RUN_ID"),
		OS:       runtime.GOOS,
		Arch:     runtime.GOARCH,
	}
}

// safeComponent keeps [A-Za-z0-9._-]; anything else becomes "_", and empty
// becomes "unknown", so the fingerprint is stable, printable and never
// accidentally empty.
func safeComponent(s string) string {
	if s == "" {
		return "unknown"
	}
	return strings.Map(func(r rune) rune {
		switch {
		case r >= 'a' && r <= 'z', r >= 'A' && r <= 'Z', r >= '0' && r <= '9', r == '.', r == '_', r == '-':
			return r
		}
		return '_'
	}, s)
}

// ImageID is "github/<os>/<arch>/<ImageOS>/<ImageVersion>".  GitHub sets
// ImageOS and ImageVersion on hosted runners only; on a self-hosted runner a
// missing value becomes "unknown" (see safeComponent).
func (g *GitHub) ImageID() string {
	return strings.Join([]string{"github", safeComponent(runtime.GOOS), safeComponent(runtime.GOARCH),
		safeComponent(g.getenv("ImageOS")), safeComponent(g.getenv("ImageVersion"))}, "/")
}

// maxEventBytes caps how much of GITHUB_EVENT_PATH is read.
const maxEventBytes = 8 << 20

// eventDefaultBranch reads repository.default_branch from the JSON file GitHub
// writes at GITHUB_EVENT_PATH.  The file is opened first and the HANDLE is
// checked (regular file, bounded size), so there is no check-then-open race.
// Any problem returns an error and the caller fails closed.
func (g *GitHub) eventDefaultBranch() (string, error) {
	path := g.getenv("GITHUB_EVENT_PATH")
	if path == "" {
		return "", fmt.Errorf("GITHUB_EVENT_PATH is not set")
	}
	f, err := openEvent(path)
	if err != nil {
		return "", err
	}
	defer f.Close()
	fi, err := f.Stat()
	if err != nil {
		return "", err
	}
	if !fi.Mode().IsRegular() {
		return "", fmt.Errorf("event file is not a regular file")
	}
	data, err := io.ReadAll(io.LimitReader(f, maxEventBytes+1))
	if err != nil {
		return "", err
	}
	if len(data) > maxEventBytes {
		return "", fmt.Errorf("event file exceeds %d bytes", maxEventBytes)
	}
	var ev struct {
		Repository struct {
			DefaultBranch string `json:"default_branch"`
		} `json:"repository"`
	}
	if err := json.Unmarshal(data, &ev); err != nil {
		return "", err
	}
	if ev.Repository.DefaultBranch == "" {
		return "", fmt.Errorf("event file has no repository.default_branch")
	}
	return ev.Repository.DefaultBranch, nil
}

// Trusted implements KILN00 section 10.2 exactly.  A run is trusted only if
// ALL of these hold; every missing, unreadable or unexpected value means NOT
// trusted (fail closed):
//
//	GITHUB_ACTIONS  == "true"
//	default branch  = repository.default_branch from the GitHub-generated
//	                  JSON at GITHUB_EVENT_PATH.  KILN_DEFAULT_BRANCH is NOT a
//	                  source of truth: if set it may only NARROW, i.e. it must
//	                  equal the event file's value or the run is untrusted.
//	GITHUB_REF      == "refs/heads/" + default branch   (so merge-queue refs
//	                   "refs/heads/gh-readonly-queue/..." and "refs/pull/N/merge"
//	                   can never match)
//	GITHUB_EVENT_NAME is one of
//	    push, schedule, workflow_dispatch
//
// workflow_dispatch can run on any branch, but the ref test above already
// restricts it to the default ref.  pull_request, pull_request_target and any
// unknown event are never trusted.
//
//	event \ ref        default branch   other ref
//	push               trusted          no
//	schedule           trusted          no
//	workflow_dispatch  trusted          no
//	pull_request(_target), merge_group, unknown, empty: no
//
// LIMIT OF THIS CHECK.  Trusted() reads the process environment and a file
// path from it.  A step that ran earlier in the SAME JOB can forge both
// (through $GITHUB_ENV, or by rewriting the file), so the answer is only
// meaningful in a job that has executed no untrusted code before Kiln starts
// (KILN00 section 9.6: untrusted actions run in their own job, with secrets
// scrubbed; section 10.2: only such runs write the shared ledger).
func (g *GitHub) Trusted() bool {
	if g.getenv("GITHUB_ACTIONS") != "true" {
		return false
	}
	def, err := g.eventDefaultBranch()
	if err != nil {
		return false
	}
	if narrow := g.getenv("KILN_DEFAULT_BRANCH"); narrow != "" && narrow != def {
		return false
	}
	if g.getenv("GITHUB_REF") != "refs/heads/"+def {
		return false
	}
	switch g.getenv("GITHUB_EVENT_NAME") {
	case "push", "schedule", "workflow_dispatch":
		return true
	}
	return false
}

// WritesLocalLedger is false: a CI runner has no local ledger of its own;
// its ledger is restored and saved by the CI cache (KILN00 section 13).
func (g *GitHub) WritesLocalLedger() bool { return false }

// Shard reads KILN_SHARD_INDEX and KILN_SHARD_COUNT (0/1 when absent or bad).
func (g *GitHub) Shard() (int, int) { return shardFrom(g.getenv) }

// WorkDir creates (once) a 0700 directory under RUNNER_TEMP, else the OS temp
// dir.
func (g *GitHub) WorkDir() (string, error) {
	base := g.getenv("RUNNER_TEMP")
	if base == "" {
		base = os.TempDir()
	}
	return g.get(base)
}

// escapeData escapes the message part of a workflow command, per GitHub's
// rules: "%" first (so later escapes are not double-escaped), then CR and LF.
func escapeData(s string) string {
	s = strings.ReplaceAll(s, "%", "%25")
	s = strings.ReplaceAll(s, "\r", "%0D")
	return strings.ReplaceAll(s, "\n", "%0A")
}

// escapeProperty additionally escapes ":" and "," which delimit properties.
func escapeProperty(s string) string {
	return strings.ReplaceAll(strings.ReplaceAll(escapeData(s), ":", "%3A"), ",", "%2C")
}

// cleanKeepNL is clean() that keeps CR and LF, which a workflow command
// then escapes to %0D / %0A instead of dropping them.
func cleanKeepNL(s string) string {
	return strings.Map(func(r rune) rune {
		if r == '\r' || r == '\n' {
			return r
		}
		return cleanRune(r)
	}, s)
}

// Report prints the event as a workflow command.  Untrusted text can never
// start a command of its own: control characters are stripped, "%", CR and
// LF are escaped inside commands, and plain lines are defused by plainLine.
func (g *GitHub) Report(e Event) {
	cmd := func(name string) {
		props := ""
		if e.Title != "" {
			props = " title=" + escapeProperty(cleanKeepNL(e.Title))
		}
		fmt.Fprintf(g.out, "::%s%s::%s\n", name, props, escapeData(cleanKeepNL(e.Message)))
	}
	switch e.Level {
	case Error:
		cmd("error")
	case Warning:
		cmd("warning")
	case Notice:
		cmd("notice")
	case GroupStart:
		fmt.Fprintf(g.out, "::group::%s\n", escapeData(cleanKeepNL(e.Message)))
	case GroupEnd:
		fmt.Fprintln(g.out, "::endgroup::")
	default:
		for _, l := range lines(e.Message) {
			fmt.Fprintln(g.out, plainLine(l))
		}
	}
}
