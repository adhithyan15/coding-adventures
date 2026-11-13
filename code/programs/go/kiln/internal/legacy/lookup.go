package legacy

// Lookup in the legacy plan (KILN04 stage 5a, sections 3.3, 9 and 9.1).
//
// `kiln legacy-plan --ledger <dir> --scope <shared|local>` adds to every
// package the answer to one question: "if the old build tool were asked to run
// this package's test now, would a traced pass already cover it?".  It is
// SHADOW ONLY.  It reports and it skips nothing; the plan is the whole effect.
//
// # The decision, per package
//
//	plan OS is not linux ............................ untraced-os
//	never list / KILN03 hard reason / probe disagreed  ineligible:<reason>   (13 item 5)
//	weak key refused (host-unknown, ...) ............ ineligible:<refusal>
//	no manifest recorded for the weak key ........... no-manifest
//	for each retained manifest, newest first (K <= 4):
//	    S = strong key from the entries' CURRENT digests
//	    a pass for S in the ledger ................... hit   (stop)
//	otherwise ....................................... miss
//
// Gating comes FIRST.  The traced keys carry tool pins only; the never list,
// the KILN03 preconditions and the probe results are in no key (KILN04 11.1
// item 13), so a hit would be unsound for a package they exclude.  Such a
// package gets no weak key lookup at all.
//
// # What a miss can and cannot say
//
// The spec asks a miss to name "the first entry whose digest differs".  That is
// not computable: a manifest stores entry NAMES, never digests (KILN04 8), and
// the pass ledger stores only strong keys, one hash over every entry.  Nothing
// at plan time knows what an entry's digest WAS when the pass was recorded.
// So a miss reports, for each manifest tried, its entry list in the order
// stored, each with the entry's state in THIS tree (`absent`, `directory`,
// `no-package`, `not-directory`, `present`) and the strong key that was looked
// up.  That is honest and still useful: a `no-package` or `absent` entry is the
// obvious suspect, and two runs of `kiln explain` on two trees show which
// strong key moved.  It never claims which entry changed.

import (
	"fmt"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/ledger"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// Lookup results (the histogram's classes; an ineligible one carries its reason).
const (
	ResultHit        = "hit"
	ResultMiss       = "miss"
	ResultNoManifest = "no-manifest"
	ResultUntracedOS = "untraced-os"
	ResultIneligible = "ineligible:"
)

// MaxPlanEntries caps the entries emitted per tried manifest (a manifest may
// hold 8,192, and each package may try 4).
const MaxPlanEntries = 256

// MaxLookupManifests is K of KILN04 3.3: at most this many manifests are tried.
const MaxLookupManifests = ledger.ManifestKeep

// lookupNote travels with every lookup plan, so a reader of the file alone
// cannot mistake it for a skip decision or for a diff.
const lookupNote = "shadow only: this reports predicted hits and skips nothing. " +
	"A miss cannot name the entry that changed: manifests store entry names, not digests, " +
	"and `now` is each entry's state in this tree, not a comparison."

// LookupOptions turn on lookup mode.
type LookupOptions struct {
	// Manifests and Passes are the two stores of one ledger directory.
	Manifests *ledger.ManifestStore
	Passes    *ledger.Store
	// Now is the clock for manifest retention (a parameter, never read here).
	Now time.Time
	// Root is the absolute checkout path the recorder keyed (`kiln:root`);
	// ExecutorFlags are the recorder's flags (`kiln:executor-args`).  A lookup
	// that differs from the recorder in either simply misses: both are in the
	// weak key.
	Root          string
	ExecutorFlags []string
}

// NewLookupOptions opens both stores from one directory and scope, so they
// cannot diverge.
func NewLookupOptions(dir string, scope ledger.Scope, now time.Time, root string, flags []string) *LookupOptions {
	return &LookupOptions{Manifests: ledger.OpenManifests(dir, scope), Passes: ledger.Open(dir, scope),
		Now: now, Root: root, ExecutorFlags: flags}
}

// check validates the options and refuses a tree that cannot be vouched for.
//
// THE FOLDER GUARD.  A plain folder cannot say whether it holds untracked
// files (source.Cleaner), so a build product in it would enter a package's
// digests and a prediction would describe a tree nobody committed.  Lookup is
// refused there rather than warned about, unlike the plain plan, because its
// output is a claim about the ledger (KILN04 9.1).
func (lo *LookupOptions) check(src Source) error {
	if lo.Manifests == nil || lo.Passes == nil {
		return fmt.Errorf("legacy plan: lookup needs a manifest store and a pass ledger")
	}
	// The scope comes from the stores themselves, and the two must agree, so a
	// plan can never be labelled with a scope its stores do not have.
	if lo.Manifests.Scope() != lo.Passes.Scope() || (lo.Passes.Scope() != ledger.Shared && lo.Passes.Scope() != ledger.Local) {
		return fmt.Errorf("legacy plan: the manifest store and the pass ledger must have one scope, shared or local")
	}
	if lo.Now.IsZero() {
		return fmt.Errorf("legacy plan: lookup needs a clock for manifest retention")
	}
	if _, ok := src.(source.Cleaner); !ok {
		return fmt.Errorf("legacy plan: refusing --ledger with this tree source: it cannot check that the tree is clean " +
			"(a plain folder lists no untracked files), so a prediction could describe files nobody committed; use --source git")
	}
	if !filepath.IsAbs(lo.Root) {
		return fmt.Errorf("legacy plan: lookup root %q must be an absolute path", short(lo.Root))
	}
	return nil
}

// EntryState is a manifest entry with its state in the tree being planned.
type EntryState struct {
	Name string `json:"name"`
	// Now is `present`, `absent`, `directory`, `no-package`, `not-directory`
	// or `unknown` (the entry could not be digested).  It is a description of
	// today's tree, never a comparison with the recorded one.
	Now string `json:"now"`
}

// ManifestTry is one manifest lookup tried.
type ManifestTry struct {
	ID       string `json:"id"`
	Run      string `json:"run"`
	At       string `json:"at"`
	Revision string `json:"revision"`
	// Strong is the strong key looked up; empty with Error when an entry could
	// not be digested.
	Strong string `json:"strong,omitempty"`
	Error  string `json:"error,omitempty"`
	// EntryCount is the manifest's real entry count.  Entries holds at most
	// MaxPlanEntries of them, in stored order, so a plan stays small;
	// EntriesTruncated says some were left out.  The strong key and
	// outside_static_closure always use the full list.
	EntryCount       int          `json:"entry_count"`
	EntriesTruncated bool         `json:"entries_truncated"`
	Entries          []EntryState `json:"entries"`
}

// PassView is the ledger's pass record that a hit found.
type PassView struct {
	Key      string `json:"key"`
	Target   string `json:"target"`
	OS       string `json:"os"`
	Arch     string `json:"arch"`
	Kiln     string `json:"kiln"`
	Revision string `json:"revision"`
	Run      string `json:"run"`
	At       string `json:"at"`
}

// LookupHit names the manifest that hit and the pass record it found.
type LookupHit struct {
	Manifest string   `json:"manifest"`
	Pass     PassView `json:"pass"`
}

// Lookup is one package's stage 5a result.
type Lookup struct {
	Result string `json:"result"`
	// Weak is the weak key, when one was computed.
	Weak string `json:"weak,omitempty"`
	// Manifests are the manifests tried, newest first, up to and including the
	// one that hit.
	Manifests []ManifestTry `json:"manifests,omitempty"`
	Hit       *LookupHit    `json:"hit,omitempty"`
	// OutsideStaticClosure lists the traced `pkg:` entries (of any retained
	// manifest) that are not in the package's static closure: dependencies the
	// old build tool does not know about, so bugs in the old graph (KILN04 9).
	//
	// The list holds at most MaxPlanEntries names (sorted, so the first ones);
	// OutsideCount is the full number and OutsideTruncated says some were left
	// out.  The histogram counts from the full set.
	OutsideStaticClosure []string `json:"outside_static_closure,omitempty"`
	OutsideCount         int      `json:"outside_count,omitempty"`
	OutsideTruncated     bool     `json:"outside_truncated,omitempty"`
}

// ManifestsInfo is what loading the manifest store did.
type ManifestsInfo struct {
	Segments   int `json:"segments"`
	Read       int `json:"read"`
	Retained   int `json:"retained"`
	Ignored    int `json:"ignored"`
	Expired    int `json:"expired"`
	Superseded int `json:"superseded"`
	Future     int `json:"future"`
	Dropped    int `json:"dropped"`
}

// LookupInfo is the plan-level record of a lookup run.
type LookupInfo struct {
	Scope string `json:"scope"`
	// ProbesRan says whether `--probe` ran.  Without it, a probe disagreement
	// could not gate anything, so skipping (M3) must require it (KILN04 9.1).
	ProbesRan bool          `json:"probes_ran"`
	Note      string        `json:"note"`
	Manifests ManifestsInfo `json:"manifests"`
	// Errors are store failures; each makes the affected packages ineligible
	// (a corrupt store means no hits, KILN04 8).
	Errors []string `json:"errors,omitempty"`
}

// Gate priorities: the never list first, then the other hard reasons, then
// probes.  Within one priority the reason that sorts first wins, so the
// reported reason never depends on map or scheduling order.
const (
	gateNever = iota
	gateHard
	gateProbe
)

type gates struct {
	set    bool
	prio   int
	reason string
}

func (g *gates) add(prio int, reason string) {
	if !g.set || prio < g.prio || (prio == g.prio && reason < g.reason) {
		g.set, g.prio, g.reason = true, prio, reason
	}
}

func (g gates) first() string { return g.reason }

// newTracedOn builds a Traced over an existing planner whose policy is already
// loaded, so lookup does not index the tree and read the policy twice.
func newTracedOn(pl *planner, root string, flags []string) (*Traced, error) {
	args, err := checkRootAndArgs(root, flags)
	if err != nil {
		return nil, err
	}
	if err := pl.checkGraphMatchesSource(); err != nil {
		return nil, err
	}
	return &Traced{pl: pl, root: root, args: args}, nil
}

// entryState describes an entry's current digest in words.
func entryState(name string, d [32]byte) string {
	switch {
	case strings.HasPrefix(name, key.EntryPackage):
		if d == key.MissingPackageDigest() {
			return "no-package"
		}
	case strings.HasPrefix(name, key.EntryPath):
		switch d {
		case key.AbsentDigest():
			return "absent"
		case key.DirectoryDigest():
			return "directory"
		}
	default:
		if d == key.NotDirectoryListingDigest() {
			return "not-directory"
		}
	}
	return "present"
}

// candidate is one manifest's strong key, held until one ledger lookup serves
// every package.
type candidate struct {
	id  [32]byte
	try ManifestTry
	key [32]byte
	ok  bool
}

func ineligible(weak *WeakKey, reason string) *Lookup {
	l := &Lookup{Result: ResultIneligible + reason}
	if weak != nil {
		l.Weak = key.Hex(weak.Key)
	}
	return l
}

// lookup fills every record's Lookup, the plan's lookup info and the histogram.
func (pl *planner) lookup(plan *Plan, lo *LookupOptions) error {
	info := &LookupInfo{Scope: lo.Passes.Scope().String(), Note: lookupNote, ProbesRan: pl.opt.Probe != nil}
	plan.Lookup = info
	recs := plan.Packages
	defer func() { plan.Histogram.Lookup, plan.Histogram.OutsideStaticClosure = lookupHistogram(recs) }()

	// Only Linux is traced (KILN04 4.1): any other plan has nothing to look up.
	if pl.opt.OS != "linux" {
		for i := range recs {
			recs[i].Lookup = &Lookup{Result: ResultUntracedOS}
		}
		return nil
	}
	tr, err := newTracedOn(pl, lo.Root, lo.ExecutorFlags)
	if err != nil {
		return err
	}

	// Pass 1: gating, then the weak key.  Gating is FIRST, so a gated package
	// never reaches the key (or a ledger).
	weaks := make([]*WeakKey, len(recs))
	if err := forEach(len(recs), pl.opt.workers(), func(i int) error {
		if g := recs[i].gate; g != "" {
			recs[i].Lookup = ineligible(nil, g)
			return nil
		}
		w, err := tr.WeakKey(pl.g.Packages[i].RelPath)
		if r, ok := IsRefusal(err); ok {
			recs[i].Lookup = ineligible(nil, r.Reason)
			return nil
		}
		if err != nil {
			return fmt.Errorf("package %s: %w", short(recs[i].Name), err)
		}
		weaks[i] = w
		return nil
	}); err != nil {
		return err
	}

	// Pass 2: the manifests, loaded for the plan's weak keys only (D11), so
	// memory follows the plan, not the store.
	wanted := map[[32]byte]struct{}{}
	for _, w := range weaks {
		if w != nil {
			wanted[w.Key] = struct{}{}
		}
	}
	set, rep, err := lo.Manifests.Load(lo.Now, wanted)
	info.Manifests = ManifestsInfo{Segments: rep.Segments, Read: rep.Read, Retained: rep.Retained, Ignored: rep.Ignored,
		Expired: rep.Expired, Superseded: rep.Superseded, Future: rep.Future, Dropped: len(rep.Dropped)}
	if err != nil {
		info.Errors = append(info.Errors, "manifest store: "+short(err.Error()))
		for i, w := range weaks {
			if w != nil {
				recs[i].Lookup = ineligible(w, "manifest-store-unusable")
			}
		}
		return nil
	}

	// Pass 3: strong keys from current digests, per package.
	cands := make([][]candidate, len(recs))
	outside := make([][]string, len(recs))
	if err := forEach(len(recs), pl.opt.workers(), func(i int) error {
		w := weaks[i]
		if w == nil {
			return nil
		}
		rms := set.Retained(w.Key)
		if len(rms) == 0 {
			recs[i].Lookup = &Lookup{Result: ResultNoManifest, Weak: key.Hex(w.Key)}
			return nil
		}
		inClosure := map[string]bool{}
		for _, q := range pl.closure(i) {
			inClosure[pl.g.Packages[q].RelPath] = true
		}
		out := map[string]bool{}
		for _, rm := range rms {
			c := candidate{id: rm.ID, try: ManifestTry{
				ID: key.Hex(rm.ID), Run: rm.Manifest.Run, Revision: rm.Manifest.Revision,
				At: rm.Manifest.At.UTC().Format(time.RFC3339), EntryCount: len(rm.Manifest.Entries),
			}}
			for _, e := range rm.Manifest.Entries {
				if rel, ok := strings.CutPrefix(e, key.EntryPackage); ok && !inClosure[rel] {
					out[rel] = true
				}
			}
			ds, err := tr.CurrentDigests(rm.Manifest.Entries)
			if err != nil {
				c.try.Error = short(err.Error())
				for _, e := range rm.Manifest.Entries[:min(len(rm.Manifest.Entries), MaxPlanEntries)] {
					c.try.Entries = append(c.try.Entries, EntryState{Name: e, Now: "unknown"})
				}
			} else if sk, err := w.Strong(ds); err != nil {
				c.try.Error = short(err.Error())
			} else {
				c.key, c.ok = sk.Key, true
				c.try.Strong = key.Hex(sk.Key)
			}
			if err == nil {
				for _, d := range ds[:min(len(ds), MaxPlanEntries)] {
					c.try.Entries = append(c.try.Entries, EntryState{Name: d.Name, Now: entryState(d.Name, d.Digest)})
				}
			}
			c.try.EntriesTruncated = len(rm.Manifest.Entries) > MaxPlanEntries
			cands[i] = append(cands[i], c)
		}
		for rel := range out {
			outside[i] = append(outside[i], "pkg:"+rel)
		}
		sort.Strings(outside[i])
		return nil
	}); err != nil {
		return err
	}

	// Pass 4: one pass-ledger read serves every package.
	var keys [][32]byte
	for _, cs := range cands {
		for _, c := range cs {
			if c.ok {
				keys = append(keys, c.key)
			}
		}
	}
	var found map[[32]byte]ledger.Record
	var passErr error
	if len(keys) > 0 {
		if found, passErr = lo.Passes.Lookup(keys); passErr != nil {
			info.Errors = append(info.Errors, "pass ledger: "+short(passErr.Error()))
		}
	}

	// Pass 5: decide, in plan order.
	for i, cs := range cands {
		if len(cs) == 0 {
			continue
		}
		l := &Lookup{Weak: key.Hex(weaks[i].Key), OutsideStaticClosure: outside[i], OutsideCount: len(outside[i])}
		if len(outside[i]) > MaxPlanEntries {
			l.OutsideStaticClosure, l.OutsideTruncated = outside[i][:MaxPlanEntries], true
		}
		recs[i].Lookup = l
		if passErr != nil {
			l.Result = ResultIneligible + "pass-ledger-unusable"
			continue
		}
		l.Result = ResultMiss
		for _, c := range cs {
			l.Manifests = append(l.Manifests, c.try)
			if rec, ok := found[c.key]; ok && c.ok {
				l.Result = ResultHit
				l.Hit = &LookupHit{Manifest: key.Hex(c.id), Pass: PassView{
					Key: key.Hex(rec.Key), Target: rec.Target, OS: rec.OS, Arch: rec.Arch, Kiln: rec.Kiln,
					Revision: rec.Revision, Run: rec.Run, At: rec.At.UTC().Format(time.RFC3339)}}
				break
			}
		}
	}
	return nil
}

// lookupHistogram counts lookup results per class and the packages with traced
// entries outside their static closure.
func lookupHistogram(recs []Record) (map[string]int, int) {
	h := map[string]int{}
	out := 0
	for _, r := range recs {
		if r.Lookup == nil {
			continue
		}
		class := r.Lookup.Result
		if rest, ok := strings.CutPrefix(class, ResultIneligible); ok {
			class = ResultIneligible + reasonGroup(rest)
		}
		h[class]++
		if r.Lookup.OutsideCount > 0 {
			out++
		}
	}
	return h, out
}
