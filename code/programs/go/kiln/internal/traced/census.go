package traced

import (
	"sort"
	"strings"

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
//
// The invariant (TestCensusKindCapsFitTheOverallBound): the per-kind caps of
// every kind the classifier can feed add up to at most MaxCensusEntries, so the
// overall bound never decides which kind loses.  The sub-coded kinds
// (unstable-path:*, exec-unreadable:*: thirty-two of them, the thirteen exechead-* codes of KILN04 D48 step 5 included) hold 256 paths each,
// the nine path kinds (the eight censusPrefixes plus proc-path) 1024:
// 9*1024 + 32*256 = 17408 <= 20480.
const (
	MaxCensusPerKind = 1024
	// MaxCensusPerSubKind is the cap of a sub-coded kind.
	MaxCensusPerSubKind = 256
	MaxCensusEntries    = 20480
)

// censusSubKindPrefixes start the kinds that have sub-codes.
var censusSubKindPrefixes = []string{"trace-incomplete:unstable-path:", "trace-incomplete:exec-unreadable:"}

// censusSubCodes is every sub-code reasonSub / censusNotes can be given, per
// reason (a fixed vocabulary: each is a string literal at its raise site).
var censusSubCodes = map[string][]string{
	"trace-incomplete:unstable-path": {
		"component-changed", "absent-changed", "final-changed", "followed-final-changed",
		"followed-absent-changed", "gone-but-found", "removed-before-made", "file-changed",
		"own-overlap", "through-missing", "dotdot", "fd-path-differs", "absent-before-made",
	},
	"trace-incomplete:exec-unreadable": {
		"modified-after-exec", "virtual-exe", "virtual-interpreter", "under-tmp", "in-repo", "host-disk",
		// KILN04 D48 step 5: the exec under the fresh tmpfs had a supervisor's
		// line and it was not believed; the sub-code says why (exechead.go).
		"exechead-pair", "exechead-kernel", "exechead-outside", "exechead-gone", "exechead-mismatch",
		"exechead-timeout", "exechead-unpinned", "exechead-fexecve", "exechead-same-image",
		"exechead-failed", "exechead-overload", "exechead-tainted", "exechead-notelf",
	},
}

// kindCap is the most distinct paths the census holds of kind.
func kindCap(kind string) int {
	for _, p := range censusSubKindPrefixes {
		if strings.HasPrefix(kind, p) {
			return MaxCensusPerSubKind
		}
	}
	return MaxCensusPerKind
}

// censusNotes collects the notes of a pass that walks Go maps, so they can be
// given to the census in sorted order (see checkStability).
type censusNote struct{ kind, path, op string }
type censusNotes struct{ list []censusNote }

// The buffer is bounded by what the pass walks: one note per unstable lookup, and
// the lookup sets already hold every one of those in memory.
func (n *censusNotes) add(c *Census, kind, path string) { n.addOp(c, kind, path, "") }

// addOp is add for a note that also carries the op class of the look that
// raised it ("" for none): flush hands it to Census.noteOp.
func (n *censusNotes) addOp(c *Census, kind, path, op string) {
	if c == nil {
		return
	}
	n.list = append(n.list, censusNote{kind, path, op})
}

// flush gives the buffered notes to the classifier's census, by kind then path
// (the path as the classifier redacts it).
func (n *censusNotes) flush(c *classifier) {
	if c.census == nil {
		return
	}
	type red struct{ kind, path, op string }
	out := make([]red, len(n.list))
	for i, e := range n.list {
		out[i] = red{e.kind, c.redact(e.path), e.op}
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].kind != out[j].kind {
			return out[i].kind < out[j].kind
		}
		return out[i].path < out[j].path
	})
	for _, e := range out {
		c.census.noteClean(e.kind, e.path)
		if e.op != "" {
			c.census.noteOp(e.kind, e.op)
		}
	}
	n.list = nil
}

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

// noteLookupOp remembers, for the census alone, the op class of the look that
// ENTERED abs into the lookup set named by set (see classifier.lookupOps).  A
// path can sit in several sets (it can be a traversed component in one call
// and a followed final in another), each entered by a different call, and the
// census must show the call whose set failed the rule.
func (c *classifier) noteLookupOp(set, abs string) {
	if c.census == nil {
		return
	}
	if c.lookupOps == nil {
		c.lookupOps = map[string]string{}
	}
	k := set + "\x00" + abs
	if _, ok := c.lookupOps[k]; !ok {
		c.lookupOps[k] = c.opClass()
	}
}

// lookupOpOf is the class noteLookupOp kept for abs in the set that the
// unstable-path sub-code names: "other" when none was.
func (c *classifier) lookupOpOf(sub, abs string) string {
	if op, ok := c.lookupOps[setOfSub(sub)+"\x00"+abs]; ok {
		return op
	}
	return "other"
}

// The lookup sets, as noteLookupOp names them.
const (
	setSeen, setAbsent, setFile, setFinal = "seen", "absent", "file", "final"
	setRelFinal, setRelAbs, setGone       = "relFinal", "relAbs", "gone"
	setBefore                             = "before"
	setOwn                                = "own"
)

// setOfSub is the lookup set whose rule the unstable-path sub-code names.
func setOfSub(sub string) string {
	switch sub {
	case "component-changed":
		return setSeen
	case "absent-changed":
		return setAbsent
	case "final-changed":
		return setFinal
	case "followed-final-changed":
		return setRelFinal
	case "followed-absent-changed":
		return setRelAbs
	case "gone-but-found", "removed-before-made":
		return setGone
	case "file-changed":
		return setFile
	case "own-overlap":
		return setOwn
	case "absent-before-made":
		return setBefore
	}
	return ""
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
	// Ops counts, per kind, the occurrences by op class (OpClasses).  Split
	// kinds: host-read, trace-incomplete:proc-path and every
	// trace-incomplete:unstable-path:<sub>.
	Ops map[string]map[string]int64
	// TmpfsExecs counts the successful execs under the step's fresh tmpfs that
	// reached the exec-head pairing (KILN04 D48 step 6, the vacuity counters): the
	// exec was believed or refused with an exechead-* sub-code, and either way the
	// mechanism was exercised.  It is bumped at the single place those sub-codes
	// are raised (loadChain), so it is never less than their sum.  Saturates at
	// MaxTmpfsExecs; nothing reads it for a verdict.
	TmpfsExecs int64
	// RestartedSpawns and BareOpens count how often the two KILN04 D50 parse
	// rules fired: a fork, vfork or clone that ended `= ? ERESTARTNOINTR` and
	// was dropped (rule 2b), and a successful read-only open with no -y
	// annotation that was accepted (rule 2c).  Summed over the action's steps
	// from trace.Trace; saturate at MaxParseRuleCount; nothing reads them for a
	// verdict.  They exist so the next census shows whether the rules matter.
	RestartedSpawns, BareOpens int64
}

// MaxParseRuleCount bounds Census.RestartedSpawns and Census.BareOpens.
const MaxParseRuleCount = 1 << 31

// noteParseRules adds one trace's D50 counters.
func (c *Census) noteParseRules(t *trace.Trace) {
	if c == nil {
		return
	}
	add := func(dst *int64, n int) {
		if n > 0 {
			*dst = min(*dst+int64(n), MaxParseRuleCount)
		}
	}
	add(&c.RestartedSpawns, t.RestartedSpawns)
	add(&c.BareOpens, t.BareOpens)
}

// MaxTmpfsExecs bounds Census.TmpfsExecs; it equals record.MaxExecHeadCount
// (the two bound the same kind of per-step count).
const MaxTmpfsExecs = 1 << 31

// noteTmpfsExec counts one exec under the tmpfs that reached the pairing.
func (c *Census) noteTmpfsExec() {
	if c != nil && c.TmpfsExecs < MaxTmpfsExecs {
		c.TmpfsExecs++
	}
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
	if _, ok := c.m[k]; !ok && (c.nKind[kind] >= kindCap(kind) || len(c.m) >= MaxCensusEntries) {
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
