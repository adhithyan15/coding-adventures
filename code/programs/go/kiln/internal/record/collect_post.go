package record

// `collect`, part 2: the checkout after the build (KILN04 sections 4.8 and 7.1)
// =============================================================================
//
// The traces say what the BUILDs did. This file asks the disk, once the last BUILD
// has exited, whether the traces are the whole story. Three questions, each with
// its own failure and its own place in the shard:
//
//	is the tree still the commit?      VerifyTree(root, <the commit prepare vouched for>)
//	is the object store intact?        git fsck --connectivity-only
//	is anything NEW on the disk?       Inventory(root), against the pre-build inventory
//
// and what each answer becomes:
//
//	header void token "tracked-changed"   ANY failure of the first two, and any
//	                                      failure to take the third: the shard-level
//	                                      void (M4). The token is the only void the
//	                                      closed vocabulary has for "the checkout is
//	                                      not what the commit says".
//	tracked_changed_unexplained           tracked paths that changed and that no
//	                                      record's tracked_writes covers: a write no
//	                                      trace saw, which voids the RUN (M2b, M-2)
//	post_inventory_unexplained            paths that are on the disk now, were not
//	                                      before the build and that no record's
//	                                      writes cover: likewise (M3)
//
// FAILING CLOSED, not crashing. A plain Git error, an `fsck` failure or an
// inventory error is not a reason to stop before the records are written (the
// spec: it "must not crash `collect`"), and it is not a clean bill either: the
// reason we could not list what changed is exactly that we cannot say that
// nothing did. So each also counts as ONE unexplained change (or path): the
// counts mean "at least this many, and perhaps more", which is the strongest
// fact `collect` has, and the merge turns it into a run-void. (The spec asks for
// a shard-level void "like tracked-changed" here; the count is a deliberate,
// stricter, addition; see the divergence list of section 4.8.)
//
// "Covers". A tracked path changed on disk is explained when some record's
// tracked_writes names it or one of its ancestors (a rename or removal of a
// tracked directory changes every file beneath it, and the classifier records the
// directory). A new path on the disk is explained when some record's writes
// overlap it (the write roots and the inventory share a granularity: the highest
// ancestor that holds no tracked file). Both comparisons are on path COMPONENTS,
// so `a/b` does not cover `a/bc`.

import (
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// postResult is what the checks after the build found.
type postResult struct {
	void                 []string
	trackedUnexplained   int
	inventoryUnexplained int
	// problems is what an operator needs to find out WHY a check failed: the
	// errors the checks returned (which the shard cannot carry), made printable and
	// bounded. It is reported on the command line and never written to the shard.
	problems []string
}

// The bounds of postResult.problems.
const (
	maxPostProblems    = 32  // entries
	maxPostProblemText = 512 // bytes of one entry
	maxPostPaths       = 16  // unexplained paths named in one entry
)

// printableText makes text that came from the checkout (a path, a git error that
// quotes one) safe to print: only printable runes (a file name can hold a
// newline, a bidi override or an escape, and a CI log reads `::` lines as
// workflow commands) and at most max bytes, cut on a rune boundary.
func printableText(s string, max int) string {
	var b strings.Builder
	for _, r := range s {
		if !unicode.IsPrint(r) {
			r = '?'
		}
		if b.Len()+utf8.RuneLen(r) > max {
			break
		}
		b.WriteRune(r)
	}
	return b.String()
}

// note adds one diagnostic, sanitised and bounded; the list is bounded too.
func (r *postResult) note(format string, args ...any) {
	if len(r.problems) >= maxPostProblems {
		return
	}
	r.problems = append(r.problems, printableText(fmt.Sprintf(format, args...), maxPostProblemText))
}

// voidTrackedChanged is the one header token for "the checkout is not the
// commit, or could not be shown to be".
const voidTrackedChanged = "tracked-changed"

// postBuild runs the three checks and never returns an error: a failure is a
// void and a count, see the file comment.
func (c *collector) postBuild(o CollectOptions, g *source.Git, recs []recfmt.ShardRecord) postResult {
	var r postResult
	tw := trackedWriteSet(recs)

	// 1. The tree is the commit prepare vouched for (binding.revision, P10).
	vt, err := o.VerifyTree(g, o.Root, c.pre.Binding.Revision)
	var te *TreeError
	switch {
	case err == nil:
		// A VerifyTree that succeeds but names another commit is as bad as one
		// that fails: the pin was the point.
		if vt.Commit != c.pre.Binding.Revision {
			r.void = append(r.void, voidTrackedChanged)
			r.trackedUnexplained++
			r.note("verify-tree: vouched for %s, not the pinned commit", short(vt.Commit))
		}
	case errors.As(err, &te):
		r.void = append(r.void, voidTrackedChanged)
		n, unexplained := unexplainedPaths(te, tw)
		r.trackedUnexplained += n
		// A partial error (the index or the commit's tree disagreed, so the disk was
		// never compared) says nothing about the files themselves: the paths it
		// names are not the paths that changed on disk, so one more change is
		// counted that no record can explain.
		if te.Partial {
			r.trackedUnexplained++
		}
		// The count and the paths first: an entry is cut at maxPostProblemText, and
		// what an operator needs most is which files, not git's wording.
		r.note("verify-tree: %d unexplained: %s; %s", n, strings.Join(firstN(unexplained, maxPostPaths), ", "), strings.ReplaceAll(te.Error(), "\n\t", " | "))
	default:
		// Git itself failed (not a tree that differs): the paths that changed
		// cannot be listed, so none can be shown explained.
		r.void = append(r.void, voidTrackedChanged)
		r.trackedUnexplained++
		r.note("verify-tree: %v", err)
	}

	// 2. The object store is connected, with the pinned git and its allowlist
	// environment (g was built from them).
	if err := o.Fsck(g); err != nil {
		r.void = append(r.void, voidTrackedChanged)
		r.trackedUnexplained++
		r.note("fsck: %v", err)
	}

	// 3. Anything new on the disk.
	post, err := o.Inventory(g, o.Root)
	if err != nil {
		// An inventory that cannot be taken voids the shard and must not crash
		// `collect` before the records are written.
		r.void = append(r.void, voidTrackedChanged)
		r.inventoryUnexplained++
		r.note("inventory: %v", err)
	} else if unexplained, err := c.newUnexplained(post, recs); err != nil {
		r.void = append(r.void, voidTrackedChanged)
		r.inventoryUnexplained++
		r.note("inventory: comparing with the sealed one: %v", err)
	} else {
		r.inventoryUnexplained += len(unexplained)
		if len(unexplained) > 0 {
			r.note("inventory: %d new paths no record's writes cover: %s", len(unexplained), strings.Join(firstN(unexplained, maxPostPaths), ", "))
		}
	}
	r.void = uniqSorted(r.void)
	return r
}

// firstN returns at most n of the strings.
func firstN(l []string, n int) []string {
	if len(l) > n {
		return l[:n]
	}
	return l
}

// trackedWriteSet is the union of every record's tracked_writes.
func trackedWriteSet(recs []recfmt.ShardRecord) map[string]bool {
	m := map[string]bool{}
	for _, rec := range recs {
		for _, p := range rec.Record.TrackedWrites {
			m[p] = true
		}
	}
	return m
}

// unexplainedPaths returns how many changed paths of a TreeError no tracked
// write covers, and the paths themselves (for the report). A TreeError without a
// path list (one built by hand, or a problem that is not a path) counts as its
// Total, never as zero, and names nothing.
func unexplainedPaths(te *TreeError, tw map[string]bool) (int, []string) {
	if len(te.Paths) == 0 {
		if te.Total > 0 {
			return te.Total, nil
		}
		return 1, nil
	}
	var out []string
	for _, p := range te.Paths {
		if !coveredByAncestor(p, tw) {
			out = append(out, p)
		}
	}
	return len(out), out
}

// coveredByAncestor reports whether p, or any ancestor directory of p, is in
// the set. "." (a problem that is not a path) is never covered.
func coveredByAncestor(p string, set map[string]bool) bool {
	if p == "." || p == "" {
		return false
	}
	if set[p] {
		return true
	}
	for i := len(p) - 1; i > 0; i-- {
		if p[i] == '/' && set[p[:i]] {
			return true
		}
	}
	return false
}

// newUnexplained lists the paths the post-build inventory has that the
// pre-build one did not and that no record's writes cover. A path is covered
// when a write is that very path or one of its ancestors (the write roots and
// the inventory share one granularity: the highest ancestor that holds no
// tracked file). The converse is NOT an excuse: a new directory `a` with a
// traced write at `a/b/c` is unexplained, because the trace named something
// beneath it and not the directory itself, and whatever else is in `a` is
// nobody's.
func (c *collector) newUnexplained(post recfmt.Inventory, recs []recfmt.ShardRecord) ([]string, error) {
	data, err := c.pre.ReadSealed("inventory.json")
	if err != nil {
		return nil, err
	}
	pre, err := recfmt.DecodeInventory(data)
	if err != nil {
		return nil, err
	}
	was := make(map[string]bool, len(pre.Paths))
	for _, p := range pre.Paths {
		was[p] = true
	}
	writes := map[string]bool{}
	for _, rec := range recs {
		for _, w := range rec.Record.Writes {
			writes[w] = true
		}
	}
	var out []string
	for _, p := range post.Paths {
		if was[p] || coveredByAncestor(p, writes) {
			continue
		}
		out = append(out, p)
	}
	return out, nil
}

// readDirNames lists a directory's entry names, sorted.
func readDirNames(dir string) ([]string, error) {
	ents, err := os.ReadDir(dir)
	if err != nil {
		return nil, err
	}
	out := make([]string, 0, len(ents))
	for _, e := range ents {
		out = append(out, e.Name())
	}
	sort.Strings(out)
	return out, nil
}
