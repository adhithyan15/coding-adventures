package host

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

// GitHub is a GitHub Actions runner.  All inputs come from environment
// variables read through the injected Getenv.
type GitHub struct {
	getenv Getenv
	out    io.Writer
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

// ImageID is "github/<os>/<arch>/<ImageOS>/<ImageVersion>".  GitHub sets
// ImageOS and ImageVersion on hosted runners only; on a self-hosted runner a
// missing value becomes "unknown" so the fingerprint is stable but plainly
// weak, never an accidental empty match.
func (g *GitHub) ImageID() string {
	or := func(v string) string {
		if v == "" {
			return "unknown"
		}
		return v
	}
	return strings.Join([]string{"github", runtime.GOOS, runtime.GOARCH,
		or(g.getenv("ImageOS")), or(g.getenv("ImageVersion"))}, "/")
}

// DefaultBranch is KILN_DEFAULT_BRANCH, or "main" when unset or empty.
func (g *GitHub) DefaultBranch() string {
	if b := g.getenv("KILN_DEFAULT_BRANCH"); b != "" {
		return b
	}
	return "main"
}

// Trusted implements KILN00 section 10.2 exactly.  A run is trusted only if
// ALL of these hold; every missing or unexpected value means NOT trusted
// (fail closed):
//
//	GITHUB_ACTIONS  == "true"
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
func (g *GitHub) Trusted() bool {
	if g.getenv("GITHUB_ACTIONS") != "true" {
		return false
	}
	if g.getenv("GITHUB_REF") != "refs/heads/"+g.DefaultBranch() {
		return false
	}
	switch g.getenv("GITHUB_EVENT_NAME") {
	case "push", "schedule", "workflow_dispatch":
		return true
	}
	return false
}

// Shard reads KILN_SHARD_INDEX and KILN_SHARD_COUNT (0/1 when absent or bad).
func (g *GitHub) Shard() (int, int) { return shardFrom(g.getenv) }

// WorkDir is RUNNER_TEMP (else the OS temp dir) plus "kiln".
func (g *GitHub) WorkDir() string {
	base := g.getenv("RUNNER_TEMP")
	if base == "" {
		base = os.TempDir()
	}
	return filepath.Join(base, "kiln")
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

// plainLine makes arbitrary text safe to print as an ordinary log line: a
// line that begins with "::" (or the legacy "##[") would be run as a workflow
// command, so such lines get a visible prefix.
func plainLine(l string) string {
	t := strings.TrimLeft(l, " \t")
	if strings.HasPrefix(t, "::") || strings.HasPrefix(t, "##[") {
		return "kiln: " + l
	}
	return l
}

// Report prints the event as a workflow command.  Untrusted text can never
// start a command of its own: "%", CR and LF are escaped inside commands, and
// plain lines are defused by plainLine.
func (g *GitHub) Report(e Event) {
	cmd := func(name string) {
		props := ""
		if e.Title != "" {
			props = " title=" + escapeProperty(e.Title)
		}
		fmt.Fprintf(g.out, "::%s%s::%s\n", name, props, escapeData(e.Message))
	}
	switch e.Level {
	case Error:
		cmd("error")
	case Warning:
		cmd("warning")
	case Notice:
		cmd("notice")
	case GroupStart:
		fmt.Fprintf(g.out, "::group::%s\n", escapeData(e.Message))
	case GroupEnd:
		fmt.Fprintln(g.out, "::endgroup::")
	default:
		for _, l := range strings.Split(strings.ReplaceAll(strings.ReplaceAll(e.Message, "\r\n", "\n"), "\r", "\n"), "\n") {
			fmt.Fprintln(g.out, plainLine(l))
		}
	}
}
