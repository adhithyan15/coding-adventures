package traced

import "sort"

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
// hostile trace can name millions of paths): at most MaxCensusEntries distinct
// (kind, path) pairs, after which a NEW pair only bumps Overflow.  Pairs
// already held keep counting.

// MaxCensusEntries bounds the distinct (kind, path) pairs one Census holds.
const MaxCensusEntries = 4096

// CensusEntry is one distinct (kind, path) pair and how often it was raised.
type CensusEntry struct {
	Kind  string `json:"kind"`
	Path  string `json:"path"`
	Count int64  `json:"count"`
}

type censusKey struct{ kind, path string }

// Census collects path-carrying reasons.  Use NewCensus.  It is not safe for
// concurrent use (a classifier is serial).
type Census struct {
	m map[censusKey]int64
	// Total is every Note, held or not; Overflow is the Notes of pairs dropped
	// because the bound was reached.
	Total, Overflow int64
}

// NewCensus returns an empty observer.
func NewCensus() *Census { return &Census{m: map[censusKey]int64{}} }

// Note records one occurrence.  The path is made printable (sanitize) so a
// newline, a bidi override or an escape in a file name cannot reach a log or
// forge a line; the kind is a fixed program string.
func (c *Census) Note(kind, path string) {
	if c == nil {
		return
	}
	k := censusKey{kind, sanitize(path)}
	c.Total++
	if _, ok := c.m[k]; !ok && len(c.m) >= MaxCensusEntries {
		c.Overflow++
		return
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
			c.Note(p[:len(p)-1], r[len(p):])
			return
		}
	}
}
