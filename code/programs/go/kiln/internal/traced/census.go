package traced

import (
	"sort"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// The census observer (KILN04, "Census side file")
// ================================================
//
// capReasons keeps at most 16 reasons of each kind, which is right for a
// record (it must stay small) and wrong for a person asking "which host paths
// void recordings, and how often?".  Worse, `trace-incomplete:proc-path` has
// no path in its text at all.  A Census is an optional observer a caller hangs
// on Input: the classifier tells it about every path-carrying reason BEFORE the
// cap, and it counts.
//
//	classifier.reason("host-read:$HOME/.cache/x")  ->  Census.Note("host-read", "$HOME/.cache/x")
//	classifier.reasonAt(rProc, "/proc/1/cmdline")  ->  Census.Note("trace-incomplete:proc-path", ...)
//
// It never feeds back: nothing in the classifier reads it, so a record made
// with a Census is byte-identical to one made without.  It is bounded (a
// hostile trace can name millions of paths): at most MaxCensusPerKind distinct
// paths per kind and MaxCensusEntries overall, after which a NEW pair only
// bumps that kind's Overflow.  Pairs already held keep counting.

// The bounds.  Per kind, so a flood of one kind (a thousand host reads) cannot
// starve a rarer, more interesting one (a late proc-path); and overall, so many
// kinds together are still bounded.  There are about ten kinds, so the overall
// bound is a safety net well above kinds x per-kind: were it near the sum of
// the per-kind caps, early kinds could fill it and starve a late one again.
const (
	MaxCensusPerKind = 1024
	MaxCensusEntries = 16384
)

// CensusEntry is one distinct (kind, path) pair and how often it was raised.
type CensusEntry struct {
	Kind  string `json:"kind"`
	Path  string `json:"path"`
	Count int64  `json:"count"`
}

type censusKey struct{ kind, path string }

// Op classes (census only)
// ------------------------
//
// `host-read` says an action looked at a host path; it does not say HOW.  The
// answer matters for the fix: a stat of /usr/lib/x is a lookup the image could
// vouch for, a read of it is content, a listing sees every sibling.  The
// classifier knows the call it is judging, so it tells the observer, in a CLOSED
// vocabulary (OpClasses): the class is chosen by the classifier's own state,
// never copied from the trace.
//
//	listing        getdents: the call saw every name in a directory
//	absent-probe   the look found nothing (ENOENT/ENOTDIR): a probe
//	open           open/openat/creat that found something
//	stat           a stat, access, readlink or other metadata call that found it
//	lookup         a link crossed on the way to a path, not the call's own target
//	exec           execve/execveat (and what the kernel loaded for it)
//	chdir          chdir
//	write          a call that can write
//	other          anything else
//
// Precedence: listing, then absent-probe, then the call's own kind.  Nothing
// reads these counts for a verdict: record.json is byte-identical with and
// without a Census (TestCensusDoesNotChangeTheRecord).
var OpClasses = []string{"listing", "absent-probe", "open", "stat", "lookup", "exec", "chdir", "write", "other"}

// IsOpClass says s is a member of OpClasses.
func IsOpClass(s string) bool {
	for _, c := range OpClasses {
		if s == c {
			return true
		}
	}
	return false
}

// opKindOf is the call's own kind for the op class, from the parser's table
// kind (a fixed vocabulary).
func opKindOf(ev *trace.Event) string {
	switch ev.Kind {
	case trace.KindOpen:
		return "open"
	case trace.KindMetadata:
		return "stat"
	case trace.KindListing:
		return "listing"
	case trace.KindDirectory:
		return "chdir"
	case trace.KindWrite:
		return "write"
	case trace.KindProcess:
		return "exec"
	}
	return "other"
}

// opClass is the class of the observation in progress.
func (c *classifier) opClass() string {
	switch {
	case c.obsListing:
		return "listing"
	case !c.obsPresent:
		return "absent-probe"
	case IsOpClass(c.opKind):
		return c.opKind
	}
	return "other"
}

// noteOp counts one occurrence of kind raised by an observation of the given
// class.  A class outside OpClasses is dropped, so the table stays bounded.
func (c *Census) noteOp(kind, class string) {
	if c == nil || !IsOpClass(class) {
		return
	}
	if c.Ops == nil {
		c.Ops = map[string]map[string]int64{}
	}
	m := c.Ops[kind]
	if m == nil {
		m = map[string]int64{}
		c.Ops[kind] = m
	}
	m[class]++
}

// Census collects path-carrying reasons.  Use NewCensus.  It is not safe for
// concurrent use (a classifier is serial).
type Census struct {
	m     map[censusKey]int64
	nKind map[string]int
	// Total is every Note, held or not; Overflow counts, per kind, the Notes of
	// pairs dropped because a bound was reached.
	Total    int64
	Overflow map[string]int64
	// Ops counts, per kind, the occurrences by op class (OpClasses).  Only
	// host-read is split so far.
	Ops map[string]map[string]int64
}

// NewCensus returns an empty observer.
func NewCensus() *Census {
	return &Census{m: map[censusKey]int64{}, nKind: map[string]int{}, Overflow: map[string]int64{}}
}

// Note records one occurrence of a path of unknown provenance: it is made
// printable (sanitize) so a newline, a bidi override or an escape in a file
// name cannot reach a log or forge a line.  The kind is a fixed program string.
func (c *Census) Note(kind, path string) {
	if c == nil {
		return
	}
	c.noteClean(kind, sanitize(path))
}

// noteClean records a path the classifier already made printable (redact and
// the reason text both end in sanitize).  Sanitising twice is not idempotent:
// a cut multi-byte rune turns the first pass's "..." into a longer tail.
func (c *Census) noteClean(kind, path string) {
	k := censusKey{kind, path}
	c.Total++
	if _, ok := c.m[k]; !ok && (c.nKind[kind] >= MaxCensusPerKind || len(c.m) >= MaxCensusEntries) {
		c.Overflow[kind]++
		return
	}
	if _, ok := c.m[k]; !ok {
		c.nKind[kind]++
	}
	c.m[k]++
}

// Entries returns the held pairs sorted by kind, then path.
func (c *Census) Entries() []CensusEntry {
	if c == nil {
		return nil
	}
	out := make([]CensusEntry, 0, len(c.m))
	for k, n := range c.m {
		out = append(out, CensusEntry{Kind: k.kind, Path: k.path, Count: n})
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].Kind != out[j].Kind {
			return out[i].Kind < out[j].Kind
		}
		return out[i].Path < out[j].Path
	})
	return out
}

// censusPrefixes are the reason prefixes that carry a path, longest first so
// "host-read:setup-modified:" is not taken for "host-read:".
var censusPrefixes = []string{
	"host-read:setup-modified:", "host-read:tool-not-keyed:",
	"host-read:", "writes-tracked:", "trace-visible:", "state-write:", "network:", "daemon:",
}

// noteReason feeds a finished reason string to the observer when it carries a
// path.  Reasons raised through reasonAt carry none of their own and are fed
// there.
func (c *Census) noteReason(r string) {
	for _, p := range censusPrefixes {
		if len(r) > len(p) && r[:len(p)] == p {
			c.noteClean(p[:len(p)-1], r[len(p):])
			return
		}
	}
}
