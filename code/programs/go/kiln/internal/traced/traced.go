// Package traced turns the parsed strace output of one legacy action into a
// RAW RECORD: what the action observed, what it wrote, and every reason its
// trace cannot be trusted.  It is stage 1b-i of KILN04
// (code/specs/KILN04-traced-read-sets.md, sections 5.1 to 5.4, 6 and 7).
//
// # Where it sits
//
//	strace text --(internal/trace, stage 1a)--> []*trace.Trace
//	                                               |
//	                          Classify (this package, stage 1b-i)
//	                                               |
//	                                           RawRecord
//	                                               |
//	               merge job (stage 4): cross-action checks, digests, manifest
//
// Stage 1a never resolves a path; this package does.  It never computes a
// digest either: digests come from the clean tracked tree at lookup time
// (stage 1b-ii), and the cross-action rules of section 7 need every action's
// record at once (stage 4).  Classify is therefore PURE: everything outside
// the traces arrives through four small interfaces and a Config, so the whole
// package is tested with in-memory fakes.
//
//	Tree   the tracked tree (what Git says is in the commit)
//	FS     the post-run disk: Lstat, Readlink, a few head bytes
//	Config the repository root, the package boundaries, host-roots.json,
//	       which tools the weak key pins, the scrubbed HOME and TMPDIR
//
// # The one idea
//
// A recorded run may be reused only if every observation it made would get
// the same answer again (KILN04 section 2).  So for each system call we ask
// one question: "what did this call LOOK AT, and could that have been
// something the action made itself?"  If the action made it, it is the
// action's own output, not an input.  If it did not, the name of the thing
// goes into the manifest (an ENTRY), or, when the thing is outside what a
// manifest can describe, into the REASONS that forbid recording at all.
//
// # Failing closed
//
// A reason is cheap; a false cache hit is not.  Whenever a path cannot be
// resolved, a component might have changed under the action, a file cannot be
// read, or a call falls outside the rules, Classify adds a reason and carries
// on collecting more.  A record with any reason is never turned into a
// manifest.
package traced

import (
	"fmt"
	"path"
	"sort"
	"strings"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// NodeKind says what a path is.
type NodeKind int

// The kinds.  KindOther is a FIFO, device or socket on the disk: never
// traversed, never read.
const (
	KindAbsent NodeKind = iota
	KindFile
	KindLink
	KindDir
	KindOther
)

// Node describes a TRACKED path.  Target is the link text of a KindLink.
type Node struct {
	Kind   NodeKind
	Target string
}

// Tree is the tracked tree: the files in the commit, as Source reports them
// (KILN04 section 5.2).  Every path is repository-relative and "/"-separated;
// "" is the root.
type Tree interface {
	// Stat describes a tracked path.  A directory is tracked when it contains
	// at least one tracked file.  KindAbsent means "not tracked".
	Stat(rel string) Node
	// Head returns up to n leading bytes of a tracked regular file.
	Head(rel string, n int) ([]byte, error)
}

// FileInfo is what the post-run disk says about one path.
type FileInfo struct {
	Kind NodeKind
	// Ctime is the inode change time.  A zero value means "unknown", which an
	// image-root check treats as a failure, never as "old enough".
	Ctime time.Time
}

// FS is the post-run filesystem, read through a narrow window: lstat and
// readlink for resolution (section 5.1), plus the first bytes of a file for
// the exec rules (a "#!" line, an ELF header).  The not-exist condition must
// satisfy errors.Is(err, fs.ErrNotExist); any other error makes the path
// unresolvable.
type FS interface {
	Lstat(path string) (FileInfo, error)
	Readlink(path string) (string, error)
	ReadHead(path string, n int) ([]byte, error)
}

// Config is everything a classification needs besides the traces.
type Config struct {
	// Root is the absolute path of the checkout as the traced action saw it.
	// It must be REAL (no symlinks in it): that is the form strace's -y
	// annotations print, and the form the working directory starts in.
	Root string
	// Packages answers "which package encloses this path" (nested aware).
	Packages *PackageIndex
	Tree     Tree
	FS       FS
	// HostRoots is the validated host-roots.json; nil means "no host root",
	// so every out-of-repository read is a host-read.
	HostRoots *legacy.HostRoots
	// Pinned is the set of ecosystems in pinned.json.
	Pinned map[string]bool
	// Tools is the set of tool names in the weak key's tools section.
	Tools map[string]bool
	// Home and Tmpdir are the scrubbed environment's HOME and TMPDIR (empty
	// when unset).  They expand $HOME and $TMPDIR in host-roots.json and are
	// redacted from reasons.
	Home, Tmpdir string
	// JobStart is the job-start stamp for the image ctime rule.
	JobStart time.Time
}

// Input is one action: a package and the trace of each of its steps, in order.
type Input struct {
	// Package is the action's package (its rel_path).
	Package string
	// Traces holds one parsed trace per BUILD step, in step order.  Created
	// paths and absent probes carry over from one step to the next.
	Traces []*trace.Trace
	// Recipe is the expected `sh -c` line of each step (KILN04 section 4.3).
	// When non-nil, CheckRecipe runs and its reason joins the record's.
	Recipe []string
}

// RawRecord is the output (KILN04 section 7, item 1).  Every slice is sorted
// and free of duplicates.  There are no digests in it.
type RawRecord struct {
	Package string `json:"package"`
	// Entries are the manifest entry names: pkg:<package>, path:<rel> and
	// list:<rel> (sections 5.2 and 11).
	Entries []string `json:"entries"`
	// Writes are in-repository paths the action wrote, coarsened to write
	// roots (section 7): the highest ancestor holding no tracked file, or the
	// path itself when its parent does.
	Writes []string `json:"writes"`
	// TrackedWrites are the TRACKED paths the action wrote: the candidates for
	// the run-wide tainted set.
	TrackedWrites []string `json:"tracked_writes"`
	// UntrackedObservations are in-repository untracked paths the action
	// observed and did not create.  The merge job decides what each means.
	UntrackedObservations []string `json:"untracked_observations"`
	// Traversed lists every in-repository directory or symlink the action's
	// paths went through.  The merge job uses it for the cross-action half of
	// the stability rule: a component another action replaced is unstable.
	Traversed []string `json:"traversed"`
	// Reasons are the section 6 reason strings, at most 16 per kind.
	Reasons []string `json:"reasons"`
}

// Limits on the sets a record carries.  Hitting one is a reason, not a crash.
const (
	maxListed         = 1 << 16
	maxReasonsPerKind = 16
	maxReasonText     = 200
)

// Classify classifies one action's traces.  The error is for a malformed
// Config or Input (a programming error); everything the traces can do wrong
// is a reason in the record.
func Classify(cfg Config, in Input) (*RawRecord, error) {
	p, err := prepare(cfg)
	if err != nil {
		return nil, err
	}
	if in.Package == "" || len(in.Traces) == 0 {
		return nil, fmt.Errorf("traced: an action needs a package and at least one trace")
	}
	for _, t := range in.Traces {
		if t == nil {
			return nil, fmt.Errorf("traced: nil trace")
		}
	}
	c := newClassifier(p)
	for _, t := range in.Traces {
		c.runTrace(t)
	}
	if in.Recipe != nil {
		for _, r := range CheckRecipe(in.Traces, in.Recipe) {
			c.reason(r)
		}
	}
	c.checkStability()
	return c.record(in.Package), nil
}

// prepared is a Config whose host-roots.json paths have been expanded.
type prepared struct {
	Config
	roots     []preparedRoot
	ephemeral []string
	absentOK  []string
}

type preparedRoot struct {
	path      string
	kind      string
	tool      string
	ecosystem string
}

func prepare(cfg Config) (*prepared, error) {
	if !strings.HasPrefix(cfg.Root, "/") || cfg.Root == "/" || path.Clean(cfg.Root) != cfg.Root {
		return nil, fmt.Errorf("traced: Root must be an absolute, clean path other than /, got %q", cfg.Root)
	}
	if cfg.Packages == nil || cfg.Tree == nil || cfg.FS == nil {
		return nil, fmt.Errorf("traced: Packages, Tree and FS are required")
	}
	for _, d := range []string{cfg.Home, cfg.Tmpdir} {
		if d != "" && (!strings.HasPrefix(d, "/") || path.Clean(d) != d) {
			return nil, fmt.Errorf("traced: Home and Tmpdir must be empty or absolute and clean, got %q", d)
		}
	}
	p := &prepared{Config: cfg}
	if cfg.HostRoots == nil {
		return p, nil
	}
	// $HOME and $TMPDIR are expanded here.  An entry that needs one that is
	// unset can never match anything, so it is dropped.
	expand := func(s string) (string, bool) {
		for _, v := range []struct{ name, val string }{{"$HOME", cfg.Home}, {"$TMPDIR", cfg.Tmpdir}} {
			if s == v.name || strings.HasPrefix(s, v.name+"/") {
				if v.val == "" {
					return "", false
				}
				return v.val + s[len(v.name):], true
			}
		}
		return s, true
	}
	for _, r := range cfg.HostRoots.Roots {
		if e, ok := expand(r.Path); ok {
			p.roots = append(p.roots, preparedRoot{path: e, kind: r.Kind, tool: r.Tool, ecosystem: r.Ecosystem})
		}
	}
	for _, s := range cfg.HostRoots.Ephemeral {
		if e, ok := expand(s); ok {
			p.ephemeral = append(p.ephemeral, e)
		}
	}
	for _, s := range cfg.HostRoots.AbsentOK {
		if e, ok := expand(s); ok {
			p.absentOK = append(p.absentOK, e)
		}
	}
	return p, nil
}

// sortedKeys returns a set's members sorted bytewise.
func sortedKeys(m map[string]bool) []string {
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// capReasons sorts reasons and keeps at most maxReasonsPerKind of each kind,
// the kind being the text before the first colon (section 6).
func capReasons(m map[string]bool) []string {
	all := sortedKeys(m)
	count := map[string]int{}
	out := []string{}
	for _, r := range all {
		kind := r
		if i := strings.IndexByte(r, ':'); i >= 0 {
			kind = r[:i]
		}
		if count[kind] >= maxReasonsPerKind {
			continue
		}
		count[kind]++
		out = append(out, r)
	}
	return out
}

// sanitize makes text from the traced action safe to print in a reason: no
// control characters (a filename can hold a newline) and a bounded length.
func sanitize(s string) string {
	var b strings.Builder
	for _, r := range s {
		if r < 0x20 || r == 0x7f {
			r = '?'
		}
		if b.Len() >= maxReasonText {
			b.WriteString("...")
			break
		}
		b.WriteRune(r)
	}
	return b.String()
}
