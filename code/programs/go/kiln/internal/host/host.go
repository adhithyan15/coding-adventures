// Package host describes where Kiln is running and who it is (KILN00 section
// 13): the machine's identity, whether this run may write the shared ledger,
// which slice of the work it owns, where scratch space lives, and how to talk
// to the person watching.
package host

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"sync"
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

// Event is something worth telling the human.  Both fields may contain
// arbitrary, even hostile, text (file names, tool output); adapters must make
// it harmless.
type Event struct {
	Level   Level
	Title   string // optional heading for annotations
	Message string
}

// Host describes where Kiln is running.
type Host interface {
	Identity() Identity
	// ImageID is OS version plus runner image id: the host fingerprint of
	// KILN00 section 8 item 3.
	ImageID() string
	// Trusted says whether this run may write the shared ledger (section
	// 10.2).  Implementations fail closed.
	Trusted() bool
	// Shard is this runner's slice: index in [0, count).
	Shard() (index, count int)
	// WorkDir is where work roots go.
	WorkDir() string
	Report(Event)
}

// Getenv reads an environment variable ("" if unset).  Injected so tests
// never touch the real environment.
type Getenv func(string) string

// Detect returns the GitHub Actions host when GITHUB_ACTIONS is exactly
// "true", otherwise the Local host.
func Detect(getenv Getenv) Host {
	if getenv("GITHUB_ACTIONS") == "true" {
		return NewGitHub(getenv, os.Stdout)
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

// Local is a developer's own machine.
//
// Trusted is true because a local run writes only the LOCAL ledger directory;
// nothing a laptop records is ever uploaded as the shared ledger (that
// happens only from a trusted GitHub run).
type Local struct {
	out io.Writer

	once      sync.Once
	dir       string
	mkdirTemp func(dir, pattern string) (string, error)
}

// NewLocal returns a Local host printing to out.
func NewLocal(out io.Writer) *Local { return &Local{out: out, mkdirTemp: os.MkdirTemp} }

// Identity reports only the OS and architecture.
func (l *Local) Identity() Identity { return Identity{OS: runtime.GOOS, Arch: runtime.GOARCH} }

// ImageID is "local/<os>/<arch>".
func (l *Local) ImageID() string { return "local/" + runtime.GOOS + "/" + runtime.GOARCH }

// Trusted is always true; see the type comment.
func (l *Local) Trusted() bool { return true }

// Shard is always the whole job.
func (l *Local) Shard() (int, int) { return 0, 1 }

// WorkDir creates (once) a private directory under the OS temp dir.  If
// creation fails it falls back to a fixed name under the temp dir.
func (l *Local) WorkDir() string {
	l.once.Do(func() {
		d, err := l.mkdirTemp(os.TempDir(), "kiln-")
		if err != nil {
			d = filepath.Join(os.TempDir(), "kiln")
		}
		l.dir = d
	})
	return l.dir
}

// Report prints plainly: no markup, one line per message line.
func (l *Local) Report(e Event) {
	switch e.Level {
	case GroupStart:
		fmt.Fprintf(l.out, "== %s ==\n", e.Message)
	case GroupEnd:
	default:
		prefix := map[Level]string{Warning: "warning: ", Error: "error: ", Notice: "note: "}[e.Level]
		if e.Title != "" {
			prefix += e.Title + ": "
		}
		fmt.Fprintf(l.out, "%s%s\n", prefix, e.Message)
	}
}

// secretSuffixes and secretNames document what ScrubSecrets removes.
var (
	secretNames    = []string{"GITHUB_TOKEN", "ACTIONS_RUNTIME_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_URL"}
	secretSuffixes = []string{"_TOKEN", "_SECRET", "_PASSWORD"}
	secretPrefixes = []string{"INPUT_"}
)

// ScrubSecrets returns env ("NAME=value" entries) without anything that looks
// like a credential (KILN00 section 9.6 item 3), preserving order.  Removed:
//
//	GITHUB_TOKEN, ACTIONS_RUNTIME_TOKEN,
//	ACTIONS_ID_TOKEN_REQUEST_TOKEN, ACTIONS_ID_TOKEN_REQUEST_URL,
//	any name ending in _TOKEN, _SECRET or _PASSWORD,
//	any name starting with INPUT_ (action inputs may carry secrets).
//
// Names compare case-insensitively because Windows environment names are.
// Over-removal only costs a child process a variable it should not have had.
func ScrubSecrets(env []string) []string {
	out := make([]string, 0, len(env))
	for _, e := range env {
		name := e
		// Start at 1 so Windows' "=C:=C:\dir" pseudo-entries keep their name.
		if i := strings.Index(e[min(1, len(e)):], "="); i >= 0 {
			name = e[:i+min(1, len(e))]
		}
		if !isSecretName(strings.ToUpper(name)) {
			out = append(out, e)
		}
	}
	return out
}

func isSecretName(n string) bool {
	for _, s := range secretNames {
		if n == s {
			return true
		}
	}
	for _, s := range secretSuffixes {
		if strings.HasSuffix(n, s) {
			return true
		}
	}
	for _, s := range secretPrefixes {
		if strings.HasPrefix(n, s) {
			return true
		}
	}
	return false
}
