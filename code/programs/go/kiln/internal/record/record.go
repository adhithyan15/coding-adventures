// Package record is the shard side of Kiln's recorder (KILN04 stage 4,
// section 4.8): the checks and inventories the recorder runs on the checkout
// BEFORE it lets any BUILD loose on it.
//
// This first slice holds the two pieces that need nothing from the sandbox:
//
//	VerifyTree   proves the working tree is byte-for-byte the commit (G5)
//	Inventory    lists everything on disk the commit does not hold
//
// Everything Kiln later records is keyed from file contents read through a
// `source.Source`, and a Source reads the DISK. If the disk is not the commit
// (a smudge filter rewrote line endings, a sparse checkout left files out, a
// previous job left a stray edit) then a digest taken from it describes a tree
// nobody committed, and a cache entry stored under it would be served to a
// build of the real commit. VerifyTree is the proof that this cannot happen.
//
// Inventory is the other half of the same bargain. The merge job must be able
// to say "this untracked path existed BEFORE the build, so it is nobody's
// product" (rules M2 and M9 of section 7.1), and for that it needs the list of
// what was there. The list is deliberately coarse: a directory that holds no
// tracked file is ONE entry, the same granularity as a write root.
//
// Both functions use Git only through `source.Git.Output` (no shell, a scrubbed
// environment, no hooks) and the filesystem only through `lstat` and
// non-following opens, so a hostile checkout cannot redirect them.
package record

import (
	"fmt"
	"sort"
	"strings"
)

// MaxReported is how many paths an error names (section 4.8, G5 step 5). A
// tree that is wrong in a thousand places is wrong; the first sixteen are
// enough to find the cause, and an error that prints the other 984 is just a
// way to fill a log.
const MaxReported = 16

// problems accumulates what is wrong and renders it as one error. It is not
// safe for concurrent use; the verifier guards it with a mutex.
type problems struct {
	list []problem
}

type problem struct{ path, why string }

func (p *problems) add(path, why string) { p.list = append(p.list, problem{path, why}) }

// err returns nil when nothing is wrong. Otherwise the problems are sorted
// (so two runs over the same bad tree print the same message, whatever order
// the workers finished in) and at most MaxReported are named.
func (p *problems) err(what string) error {
	if len(p.list) == 0 {
		return nil
	}
	sort.Slice(p.list, func(i, j int) bool {
		if p.list[i].path != p.list[j].path {
			return p.list[i].path < p.list[j].path
		}
		return p.list[i].why < p.list[j].why
	})
	e := &TreeError{What: what, Total: len(p.list)}
	for i, pr := range p.list {
		if i < MaxReported {
			e.Problems = append(e.Problems, fmt.Sprintf("%s: %s", printable(pr.path), pr.why))
		}
		// The FULL list, as the raw paths and never printed: the list is sorted
		// by path, so a path with two problems is adjacent to itself.
		if n := len(e.Paths); n == 0 || e.Paths[n-1] != pr.path {
			e.Paths = append(e.Paths, pr.path)
		}
	}
	return e
}

// TreeError says the checkout is not the commit (or could not be checked).
type TreeError struct {
	What     string
	Problems []string // at most MaxReported, sorted by path
	Total    int      // how many there were
	// Paths is EVERY path that has a problem, raw (not made printable),
	// sorted bytewise and without repeats; unlike Problems it is not cut at
	// MaxReported. `collect` needs the whole set to decide which changed
	// tracked paths some record's tracked_writes explains and which nobody's
	// does (KILN04 7.1, M-2); it is for comparison, never for printing. A
	// path "." means the problem is not a path (a sparse checkout).
	Paths []string
	// Partial says the check stopped before it compared the DISK: the commit's
	// tree and the index disagreed (or the index had an unresolved merge, or a
	// gitlink), so Paths are paths of that disagreement, not files whose bytes
	// changed. Nothing is known about the disk, and a caller that counts changes
	// must not take the list as the whole story.
	Partial bool
}

func (e *TreeError) Error() string {
	var b strings.Builder
	b.WriteString(e.What)
	b.WriteString(": ")
	b.WriteString(fmt.Sprint(e.Total))
	b.WriteString(" problem")
	if e.Total != 1 {
		b.WriteString("s")
	}
	for _, p := range e.Problems {
		b.WriteString("\n\t")
		b.WriteString(p)
	}
	if more := e.Total - len(e.Problems); more > 0 {
		fmt.Fprintf(&b, "\n\t... and %d more", more)
	}
	return b.String()
}

// printable makes a path safe to put in a log line: control characters (a
// newline in a file name could forge a second log line) become '?', and a very
// long name is cut.
func printable(s string) string {
	r := []rune(strings.Map(func(r rune) rune {
		if r < 0x20 || r == 0x7f {
			return '?'
		}
		return r
	}, s))
	if len(r) > 200 {
		return string(r[:200]) + "..."
	}
	return string(r)
}
