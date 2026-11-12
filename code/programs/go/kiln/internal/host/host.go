// Package host describes where Kiln is running and who it is (KILN00 section
// 13): the machine's identity, whether this run may write the shared ledger,
// which slice of the work it owns, where scratch space lives, and how to talk
// to the person watching.
package host

import (
	"fmt"
	"io"
	"os"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"unicode"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/envscrub"
)

// Identity is who is running: the commit, branch, triggering event and run.
type Identity struct {
	Revision string
	Branch   string
	Event    string
	RunID    string
	OS       string
	Arch     string
}

// Level is the severity or role of an Event.
type Level int

// Event levels.
const (
	Info       Level = iota // plain progress line
	Notice                  // highlighted note
	Warning                 // annotation: warning
	Error                   // annotation: error
	GroupStart              // begin a collapsible log group titled Message
	GroupEnd                // end the current group
)

// Event is something worth telling the human.  Both text fields may contain
// arbitrary, even hostile, text (file names, tool output); every Host makes
// them harmless before printing.
type Event struct {
	Level   Level
	Title   string // optional heading for annotations
	Message string
}

// Host describes where Kiln is running.
type Host interface {
	Identity() Identity
	// ImageID is OS version plus runner image id: the host fingerprint of
	// KILN00 section 8 item 3.  Only [A-Za-z0-9._-/] appear in it.
	ImageID() string
	// Trusted says whether this run may write the SHARED ledger (section
	// 10.2).  Implementations fail closed.  Only a trusted GitHub run is ever
	// trusted; a developer's machine is NOT.
	Trusted() bool
	// WritesLocalLedger says whether the run may write the local ledger
	// directory on this machine (never uploaded as the shared ledger).
	WritesLocalLedger() bool
	// Shard is this runner's slice: index in [0, count).
	Shard() (index, count int)
	// WorkDir returns a fresh private directory for work roots.  There is no
	// predictable fallback: failure is an error.
	WorkDir() (string, error)
	Report(Event)
}

// Getenv reads an environment variable ("" if unset).  Injected so tests
// never touch the real environment.
type Getenv func(string) string

// ciNames are well-known CI indicators beyond the GITHUB_/RUNNER_ families.
var ciNames = []string{"CI", "TF_BUILD", "JENKINS_URL", "BUILDKITE", "GITLAB_CI", "CIRCLECI", "TRAVIS", "TEAMCITY_VERSION"}

// inCI reports whether the environment shows ANY sign of a CI system.
func inCI(getenv Getenv, environ []string) bool {
	for _, n := range ciNames {
		if getenv(n) != "" {
			return true
		}
	}
	for _, e := range environ {
		n := strings.ToUpper(envscrub.EntryName(e))
		if strings.HasPrefix(n, "GITHUB_") || strings.HasPrefix(n, "RUNNER_") {
			return true
		}
	}
	return false
}

// Detect chooses the Host.  environ is the full "NAME=value" environment
// (os.Environ()) so GITHUB_* and RUNNER_* names can be enumerated.
//
//	GITHUB_ACTIONS is exactly "true"        -> GitHub
//	any other CI indicator is present       -> Untrusted (fail closed)
//	a clean environment, no CI indicators   -> Local
//
// The middle row matters: a CI system Kiln does not know, or a GitHub runner
// whose GITHUB_ACTIONS was tampered with, must never be mistaken for a
// developer's laptop.
func Detect(getenv Getenv, environ []string) Host {
	if getenv("GITHUB_ACTIONS") == "true" {
		return NewGitHub(getenv, os.Stdout)
	}
	if inCI(getenv, environ) || strings.EqualFold(getenv("GITHUB_ACTIONS"), "true") {
		return NewUntrusted(os.Stdout)
	}
	return NewLocal(os.Stdout)
}

// shardFrom reads KILN_SHARD_INDEX / KILN_SHARD_COUNT.  Anything missing or
// out of range falls back to 0/1, i.e. "do everything": a wrong shard can
// cost time but cannot skip work.
func shardFrom(getenv Getenv) (int, int) {
	idx, err1 := strconv.Atoi(getenv("KILN_SHARD_INDEX"))
	cnt, err2 := strconv.Atoi(getenv("KILN_SHARD_COUNT"))
	if err1 != nil || err2 != nil || cnt < 1 || idx < 0 || idx >= cnt {
		return 0, 1
	}
	return idx, cnt
}

// scratch lazily creates one private (mode 0700) directory.
type scratch struct {
	mkdirTemp func(dir, pattern string) (string, error)
	once      sync.Once
	dir       string
	err       error
}

func (s *scratch) get(base string) (string, error) {
	s.once.Do(func() {
		mk := s.mkdirTemp
		if mk == nil {
			mk = os.MkdirTemp
		}
		s.dir, s.err = mk(base, "kiln-")
	})
	return s.dir, s.err
}

// Local is a developer's own machine.
//
// Trusted is FALSE: "trusted" means "may write the SHARED ledger", and
// nothing a laptop records may reach it.  Local runs use the local ledger
// directory only, which WritesLocalLedger reports.
type Local struct {
	out io.Writer
	scratch
}

// NewLocal returns a Local host printing to out.
func NewLocal(out io.Writer) *Local { return &Local{out: out} }

// Identity reports only the OS and architecture.
func (l *Local) Identity() Identity { return Identity{OS: runtime.GOOS, Arch: runtime.GOARCH} }

// ImageID is "local/<os>/<arch>".
func (l *Local) ImageID() string { return "local/" + runtime.GOOS + "/" + runtime.GOARCH }

// Trusted is always false; see the type comment.
func (l *Local) Trusted() bool { return false }

// WritesLocalLedger is true: the local ledger is the point of a local run.
func (l *Local) WritesLocalLedger() bool { return true }

// Shard is always the whole job.
func (l *Local) Shard() (int, int) { return 0, 1 }

// WorkDir creates (once) a 0700 directory under the OS temp dir.
func (l *Local) WorkDir() (string, error) { return l.get(os.TempDir()) }

// Report prints plainly (see reportPlain).
func (l *Local) Report(e Event) { reportPlain(l.out, e) }

// Untrusted is the fail-closed host for any CI Kiln does not recognise (or a
// tampered GitHub environment).  It can neither write the shared ledger nor
// the local one.
type Untrusted struct {
	out io.Writer
	scratch
}

// NewUntrusted returns an Untrusted host printing to out.
func NewUntrusted(out io.Writer) *Untrusted { return &Untrusted{out: out} }

// Identity reports only the OS and architecture.
func (u *Untrusted) Identity() Identity { return Identity{OS: runtime.GOOS, Arch: runtime.GOARCH} }

// ImageID is "untrusted/<os>/<arch>".
func (u *Untrusted) ImageID() string { return "untrusted/" + runtime.GOOS + "/" + runtime.GOARCH }

// Trusted is always false.
func (u *Untrusted) Trusted() bool { return false }

// WritesLocalLedger is always false.
func (u *Untrusted) WritesLocalLedger() bool { return false }

// Shard is always the whole job.
func (u *Untrusted) Shard() (int, int) { return 0, 1 }

// WorkDir creates (once) a 0700 directory under the OS temp dir.
func (u *Untrusted) WorkDir() (string, error) { return u.get(os.TempDir()) }

// Report prints plainly (see reportPlain).
func (u *Untrusted) Report(e Event) { reportPlain(u.out, e) }

// ScrubSecrets removes credentials from a child environment; see the
// envscrub package for the exact list.
func ScrubSecrets(env []string) []string { return envscrub.Scrub(env) }

// clean removes every C0 and C1 control character (and the Unicode line and
// paragraph separators) except TAB, so no escape sequence, backspace, bell or
// stray line break can survive into a terminal or a log parser.
func clean(s string) string { return strings.Map(cleanRune, s) }

func cleanRune(r rune) rune {
	if r == '\t' {
		return r
	}
	if unicode.IsControl(r) || r == 0x2028 || r == 0x2029 {
		return -1
	}
	return r
}

// lines splits text into lines (CRLF, CR and LF all separate) and cleans
// each; empty input yields one empty line.
func lines(s string) []string {
	s = strings.ReplaceAll(strings.ReplaceAll(s, "\r\n", "\n"), "\r", "\n")
	parts := strings.Split(s, "\n")
	for i, p := range parts {
		parts[i] = clean(p)
	}
	return parts
}

// plainLine defuses a line destined for an ordinary log.  A runner executes a
// line as a workflow command if it starts (after optional whitespace) with
// "::" or "##[".  Rather than reason about every runner's whitespace rules,
// ANY line containing either sequence anywhere gets a visible prefix.
func plainLine(l string) string {
	if strings.Contains(l, "::") || strings.Contains(l, "##[") {
		return "kiln: " + l
	}
	return l
}

// reportPlain is the shared plain-text Report.
func reportPlain(out io.Writer, e Event) {
	switch e.Level {
	case GroupStart:
		for _, l := range lines(e.Message) {
			fmt.Fprintln(out, plainLine("== "+l+" =="))
		}
	case GroupEnd:
	default:
		prefix := map[Level]string{Warning: "warning: ", Error: "error: ", Notice: "note: "}[e.Level]
		if e.Title != "" {
			prefix += strings.Join(lines(e.Title), " ") + ": "
		}
		for _, l := range lines(e.Message) {
			fmt.Fprintln(out, plainLine(prefix+l))
		}
	}
}
