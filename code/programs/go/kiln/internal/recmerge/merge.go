// Package recmerge is the pure merge engine of KILN04 stage 4
// (code/specs/KILN04-traced-read-sets.md, section 7 and 7.1).
//
// # What problem it solves
//
// Legacy BUILDs share one tree, so a BUILD can read something another BUILD
// made.  Such a read depends on which actions ran first, and that is in no
// cache key.  The recorder (stage 4, PR 2) therefore records raw facts per
// shard, and this package is the one place that sees ALL shards at once and
// decides which actions may become a manifest and a pass.
//
//	shards' records.jsonl --(recfmt.DecodeShard)--> []*recfmt.Shard
//	                                                     |
//	             Merge(Input)  <---- plan, regenerated digests, Keys
//	                  |
//	              Result: which actions passed, which did not and why,
//	                      manifests and passes to hand to the ledger
//
// Merge does no I/O and reads no clock: the caller passes the bytes already
// decoded, the regenerated digests, the time, and a Keys that knows how to
// compute weak and strong keys from the merge job's own verified tree.  So
// every rule is tested with records built in memory.
//
// # The shape of a decision
//
// There are three levels of "no", each larger than the last:
//
//	action     the action gets a reason; no manifest, no pass for it
//	shard-void no action ASSIGNED to that shard is emitted
//	run-void   nothing is emitted at all
//
// run-void is for anything that makes a rule's INPUT untrustworthy.  The
// cross-action rules quantify over every action's writes ("a path another
// action wrote, in any shard"), so a single action whose writes are unknown
// could have written the very path a different action in a different shard
// probed as absent.  Voiding only the first action's shard would leave that
// second action looking clean.  That is gap G1 of section 7.1, and why an
// unknown write set voids the RUN.
//
// Rules are evaluated completely even after a void, so the report can say
// everything that is wrong, not just the first thing.
//
// # An action runs in several shards
//
// A shard runs its assigned packages AND their prerequisites, so one action
// has a raw record in every shard that needs it (gap G3).  Each such record is
// a COPY.  All copies take part in every rule (a reason found on any copy is
// the action's), but only the copy in the shard the partition assigns the
// action to can become a manifest.
//
// # Rule numbers
//
// The comments cite B1-B6 (binding), M1-M15 (the rules) as in the design
// table of section 7.1.  B1 (the merge's own tree verifies, HEAD matches the
// workflow's sha) happens before Merge is called: Merge is given the
// already-regenerated digests as Expected.
package recmerge

import (
	"encoding/hex"
	"fmt"
	mbits "math/bits"
	"sort"
	"strings"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/ledger"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// Keys computes keys from the MERGE job's own tree.  The merge never trusts a
// digest from a shard (section 7.1, G4): it takes only the shard's machine
// facts, recomputes, and compares.
//
// A production Keys wraps legacy.NewTraced(graph, legacy.Options{Source: own
// source, Host: facts, Environ: facts.Environ(), OS: "linux", Arch: "x64"},
// facts.Root, facts.ExecutorArgs), memoised per distinct facts (PR 3).
type Keys interface {
	// Weak returns the weak key of action under facts, or a non-empty refusal
	// (legacy.Refusal.Reason) when the action has no traced key.  An error is
	// the merge job's own malfunction.
	Weak(f recfmt.HostFacts, action string) (k [32]byte, refusal string, err error)
	// Strong returns the strong key for the entry names, each digested as it
	// is now in the merge's verified tree.
	Strong(f recfmt.HostFacts, action string, names []string) ([32]byte, error)
}

// Run is what the merge job knows about its own run.
type Run struct {
	Repository string // owner/name, GITHUB_REPOSITORY
	ID         string // GITHUB_RUN_ID
	Attempt    int    // GITHUB_RUN_ATTEMPT
	Revision   string // HEAD, already equal to GITHUB_SHA (B1)
}

// Expected is what the merge job regenerated from its own tree (B1, D10).
type Expected struct {
	GraphSHA256 string
	StepsSHA256 string
	// Plan is the shard partition derived from the regenerated build plan.  Its
	// length is the shard count.
	Plan []recfmt.ShardPlan
	// Steps maps a package rel_path to recfmt.StepsDigest of the BUILD lines
	// this OS runs (B6).
	Steps map[string]string
}

// Input is everything Merge needs.
type Input struct {
	Run      Run
	Expected Expected
	Shards   []*recfmt.Shard
	Keys     Keys
	// Now stamps the manifests and passes (whole seconds, UTC).
	Now time.Time
	// Kiln is the ledger record's "kiln" field, e.g. "kiln/0.1.0 traced/2
	// recorder/1".
	Kiln string
	// HonourWritesComplete is the PR 4 switch (section 7.1, G1).  False (the
	// only production setting until the classifier reports writes_complete):
	// every trace-incomplete reason is an unknown write set.  True: a record
	// that says writes_complete=true loses only its own manifest for
	// read-side trace-incomplete reasons; a record without the field is still
	// unknown.
	HonourWritesComplete bool
	// WorkBudget bounds the visited nodes and comparisons of every overlap
	// walk together (0 means DefaultWorkBudget).  Exceeding it is run-void
	// "size": a hostile record cannot make the merge quadratic.
	WorkBudget int64
	// MaxRetainedBytes bounds the path and entry text the merge holds
	// (0 means DefaultMaxRetainedBytes), as KILN04 2.7 asks.
	MaxRetainedBytes int64
}

// Defaults of the two bounds above.
const (
	DefaultWorkBudget       int64 = 500_000_000
	DefaultMaxRetainedBytes int64 = 4 << 30
)

// Outcome is the verdict on one action.
type Outcome struct {
	// Action is a package path taken from a shard's record: valid UTF-8 but
	// otherwise raw.  Whatever prints it (the report, PR 3) must sanitise it.
	Action string
	// Assigned is the shard the partition assigns the action to, or -1.
	Assigned int
	// Reasons are the action's reasons, sorted, at most 16 per kind.
	Reasons []string
	// Recordable is true when no rule M5-M15 produced a reason, whatever the
	// void level: the number the kill criterion is computed from (an upper
	// bound while the run is void).
	Recordable bool
	// ShardVoid is true when the action's assigned shard is void (M4).
	ShardVoid bool
	// Emitted is true when the action became a manifest and a pass.
	Emitted bool
	// Truncated: a copy of the action did not run to completion (status other
	// than built, steps_run below steps_total, daemon-left): M-1.
	Truncated bool
	// OrderDependent: the action's behaviour depended on another action's
	// recorded writes: M-3.  Its writes in another order are not the recorded
	// ones, so under the strict verdict it voids the whole run.
	OrderDependent bool
	// RelaxedEmittable is the REPORT-ONLY verdict: would the action be
	// emitted if the M-1 and M-3 classes cost only the action itself?  It is
	// never turned into a manifest or a pass (see Result.RelaxedRunVoid).
	RelaxedEmittable bool
}

// Result is the whole verdict.  Every slice is sorted, so equal inputs give
// byte-equal results whatever order the shards or records arrived in.
type Result struct {
	// Emitted is true when the run is not void (Manifests and Passes may still
	// be empty).
	Emitted bool
	// RunVoid lists the causes of a run-void, sorted, at most MaxCauses;
	// RunVoidCount is the true number.
	RunVoid      []string
	RunVoidCount int
	// ShardVoid maps a shard index to its void tokens.
	ShardVoid map[int][]string
	Outcomes  []Outcome
	// Manifests and Passes are index-aligned and sorted by action.  They come
	// from the STRICT verdict only.
	Manifests []ledger.Manifest
	Passes    []ledger.Record
	// RelaxedRunVoid and RelaxedRunVoidCount are the causes that remain when
	// the M-1 (truncated execution) and M-3 (order dependence) classes are
	// charged to their action instead of the run.  Every other cause stays.
	//
	// THE RELAXED VERDICT IS REPORT-ONLY.  It exists so the M2 report can say
	// what action-level handling would yield; no manifest, pass or ledger
	// record is ever derived from it, and a test holds that line
	// (TestRelaxedVerdictNeverEmits).  Whether a relaxed rule is sound is a
	// separate question (KILN04 section 14, second-order order dependence).
	RelaxedRunVoid      []string
	RelaxedRunVoidCount int
}

// MaxCauses bounds Result.RunVoid.
const MaxCauses = 64

const (
	osName   = "linux"
	archName = "x64"
	maxHits  = recfmt.MaxReasonsPerKind
)

// ---------------------------------------------------------------------------

// reasons is an action's reason set.  any records that a reason was ever
// added; m is trimmed to the smallest maxHits per kind whenever it grows,
// which is safe because "the smallest k of a union" equals "the smallest k of
// the union of each part's smallest k".
type reasons struct {
	m   map[string]bool
	any bool
	mt  *meter
	// kept is the size after the last trim; trims counts them (tests).
	kept, trims int
	// order is set when a reason means the action's behaviour depended on
	// another action's recorded writes (M-3).
	order bool
}

// intraActionKind is the M-3 ALLOW-LIST: the reason kinds of a shard's own
// record that describe the action's own environment and nothing about other
// actions.  Every other kind, including a daemon, any spelling of
// reads-product or unstable-path, and a kind this code has never heard of,
// marks the action order-dependent (fail closed).  Checked against the
// classifier's kinds (internal/traced) and the reasons the merge itself adds.
func intraActionKind(k string) bool {
	switch k {
	case "failed", // its BUILD exited non-zero
		"precondition",     // a gate or refusal (KILN03 hard reason, probe, no key)
		"network",          // it used the network (section 5.4)
		"host-read",        // it read outside the repository (5.3)
		"state-write",      // it wrote a declared host state root
		"writes-tracked",   // it wrote a tracked path (a taint source, not a reader)
		"vcs-read",         // it read .git
		"trace-visible",    // it noticed it was traced
		"unscrubbed",       // the environment was not the scrubbed one
		"trace-incomplete", // the trace is partial (M1 decides the run)
		"daemon-left":      // a process outlived a step (M-1 decides the run)
		return true
	}
	return strings.HasPrefix(k, "manifest-") // too large or invalid manifest
}

// dep adds an order-dependence reason: reads-product (M5-M7, M9-M14) or
// unstable-path (M8, M9).  Exactly the reasons the merge derives from
// OTHER actions' records are these, so this is the whole M-3 list.
func (r *reasons) dep(s string) {
	r.order = true
	r.add(s)
}

func (r *reasons) add(s string) {
	r.any = true
	// Each reason costs a sanitise, a map insert and, now and then, a sort:
	// charge it, so a world with millions of hits ends as run-void "size".
	r.mt.spend(int64(16 + len(s)/8))
	if r.m == nil {
		r.m = map[string]bool{}
	}
	r.m[s] = true
	// Trim only when the set has DOUBLED since the last trim (and passed 512):
	// a set that stays large because it holds many kinds at 16 each would
	// otherwise be re-sorted on every add.  A trim costs n log n, charged.
	if len(r.m) > max(512, 2*r.kept) {
		r.trims++
		n := int64(len(r.m))
		r.mt.spend(n * int64(mbits.Len64(uint64(n))))
		kept := map[string]bool{}
		for _, k := range traced.CapReasons(r.m) {
			kept[k] = true
		}
		r.m = kept
		r.kept = len(kept)
	}
}

func (r *reasons) product(path string) { r.dep("reads-product:" + traced.Sanitize(path)) }

type copyRec struct {
	sh  *recfmt.Shard
	rec *recfmt.ShardRecord
	// recipeBad: B6 found the steps differ from the regenerated ones.
	recipeBad bool
}

type act struct {
	name     string
	id       int
	copies   []*copyRec
	assigned int
	w, tw    []string
	binds    []string
	rs       reasons
}

type merger struct {
	in     Input
	causes *causeSet // strict verdict: every cause
	// relaxed holds the causes that are NOT charged to an action in the
	// relaxed (report-only) verdict.
	relaxed   *causeSet
	relaxLost map[string]string // action -> why (M-1 and M-3 classes)
	mt        *meter
	malformed bool
	n         int // shard count

	shards    []*recfmt.Shard
	byIndex   map[int]*recfmt.Shard // artifact index -> shard (first of any duplicates)
	planPkgs  []map[string]bool
	assigned  map[string]int
	shardVoid map[int]map[string]bool

	acts    map[string]*act
	ordered []*act

	wIx, twIx *pathIndex
	shardW    map[*recfmt.Shard]*pathIndex
	shardInv  map[*recfmt.Shard]pathSet
	tainted   pathSet
}

// void records a cause that voids the run in BOTH verdicts.
func (m *merger) void(format string, args ...any) {
	c := traced.Sanitize(fmt.Sprintf(format, args...))
	m.causes.add(c)
	m.relaxed.add(c)
}

// voidStrict records a cause of the M-1 / M-3 classes: it voids the run in the
// strict verdict and only costs the action in the relaxed one.
func (m *merger) voidStrict(action, class, format string, args ...any) {
	m.causes.add(traced.Sanitize(fmt.Sprintf(format, args...)))
	if m.relaxLost[action] == "" {
		m.relaxLost[action] = class
	}
}

// Merge applies B1-B6 and M1-M15 to the shards and returns the verdict.
func Merge(in Input) Result {
	m := &merger{
		in: in, causes: newCauseSet(), relaxed: newCauseSet(), relaxLost: map[string]string{}, byIndex: map[int]*recfmt.Shard{},
		assigned: map[string]int{}, shardVoid: map[int]map[string]bool{}, acts: map[string]*act{},
		shardW: map[*recfmt.Shard]*pathIndex{}, shardInv: map[*recfmt.Shard]pathSet{},
	}
	budget := in.WorkBudget
	if budget <= 0 {
		budget = DefaultWorkBudget
	}
	m.mt = &meter{max: budget}
	m.checkPlan()
	m.collectShards()
	m.checkBindings()
	if m.malformed {
		// A record that fails validation was not written by the classifier; the
		// run is void and nothing below is worth analysing (or worth the time a
		// hostile record could make it cost).
		return m.finish()
	}
	if m.tooBig() {
		// Nothing below can be trusted to finish in bounded memory.
		m.void("size: the records hold more than the retained-text cap")
		return m.finish()
	}
	m.buildActions()
	m.checkWeakKeys()
	m.checkRecipes()
	m.checkHeaders()
	m.buildIndexes()
	m.completeness()
	m.perAction()
	m.sharedPorts()
	m.daemons()
	m.taintedMetadata()
	m.orderDependence()
	if m.mt.over() {
		m.void("size: the overlap walks exceeded the work budget")
	}
	return m.finish()
}

// ---------------------------------------------------------------------------
// The plan.

// checkPlan makes sure the regenerated plan can describe a partition: shard
// indexes 0..N-1, every assigned package also run by its shard, and every
// package assigned to exactly one shard.
func (m *merger) checkPlan() {
	plan := m.in.Expected.Plan
	m.n = len(plan)
	if m.n == 0 {
		m.void("plan: no shards")
		return
	}
	m.planPkgs = make([]map[string]bool, m.n)
	seen := make([]bool, m.n)
	for _, p := range plan {
		if p.Index < 0 || p.Index >= m.n || seen[p.Index] {
			m.void("plan: shard indexes are not 0..%d (index %d)", m.n-1, p.Index)
			return
		}
		seen[p.Index] = true
		set := map[string]bool{}
		for _, pkg := range p.Packages {
			if err := key.ValidatePath(pkg); err != nil || set[pkg] {
				m.void("plan: shard %d names package %q invalidly or twice", p.Index, pkg)
			}
			set[pkg] = true
		}
		m.planPkgs[p.Index] = set
		for _, pkg := range p.Assigned {
			if !set[pkg] {
				m.void("plan: shard %d is assigned %q, which it does not run", p.Index, pkg)
			}
			if prev, dup := m.assigned[pkg]; dup {
				m.void("plan: %q is assigned to shards %d and %d", pkg, prev, p.Index)
			}
			m.assigned[pkg] = p.Index
		}
	}
	for _, p := range plan {
		for _, pkg := range p.Packages {
			if _, ok := m.assigned[pkg]; !ok {
				m.void("plan: %q is run by shard %d but assigned to none", pkg, p.Index)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Binding (B2-B4).

func (m *merger) collectShards() {
	for _, s := range m.in.Shards {
		if s == nil {
			m.void("binding: a nil shard was supplied")
			continue
		}
		m.shards = append(m.shards, s)
	}
	sort.SliceStable(m.shards, func(a, b int) bool {
		x, y := m.shards[a], m.shards[b]
		if x.ArtifactIndex != y.ArtifactIndex {
			return x.ArtifactIndex < y.ArtifactIndex
		}
		return x.BindingID < y.BindingID
	})
	for _, s := range m.shards {
		if _, dup := m.byIndex[s.ArtifactIndex]; dup {
			m.void("binding B2: shard %d was supplied more than once", s.ArtifactIndex)
			continue
		}
		m.byIndex[s.ArtifactIndex] = s
	}
	for i := 0; i < m.n; i++ {
		if m.byIndex[i] == nil {
			m.void("binding B2: shard %d is missing", i)
		}
	}
}

func (m *merger) checkBindings() {
	exp, run := m.in.Expected, m.in.Run
	partition, err := recfmt.PartitionDigest(exp.Plan)
	if err != nil {
		m.void("plan: %v", err)
	}
	for _, s := range m.shards {
		i, b := s.ArtifactIndex, &s.Binding
		bad := func(what string) { m.void("binding shard %d: %s", i, what) }
		if i < 0 || i >= m.n {
			bad("the artifact index is outside the plan")
			// Nothing of this shard has been validated: never analyse it.
			m.malformed = true
			continue
		}
		for _, err := range []error{b.Validate(), s.Header.Validate(), s.Facts.Validate(), s.Inventory.Validate()} {
			if err != nil {
				bad(err.Error())
				m.malformed = true
			}
		}
		switch {
		case b.ShardIndex != i:
			bad(fmt.Sprintf("B2 the binding says it is shard %d", b.ShardIndex))
		case s.Header.ShardIndex != i:
			bad(fmt.Sprintf("B2 the header says it is shard %d", s.Header.ShardIndex))
		}
		if b.ShardCount != m.n || s.Header.ShardCount != m.n {
			bad(fmt.Sprintf("B2 shard_count %d/%d, the plan has %d", b.ShardCount, s.Header.ShardCount, m.n))
		}
		if s.Header.BindingID != s.BindingID {
			bad("B4 the header names another binding")
		}
		if b.Repository != run.Repository {
			bad("B3 another repository")
		}
		if b.Run != run.ID {
			bad("B3 another run")
		}
		if b.Attempt > run.Attempt {
			bad("B3 a later attempt")
		}
		if b.Revision != run.Revision {
			bad("B3 another revision")
		}
		if b.Recorder != legacy.RecorderVersion {
			bad(fmt.Sprintf("B3 recorder %d, this merge is for %d", b.Recorder, legacy.RecorderVersion))
		}
		if b.GraphSHA256 != exp.GraphSHA256 {
			bad("B3 the graph differs from the regenerated one")
		}
		if b.StepsSHA256 != exp.StepsSHA256 {
			bad("B3 the steps differ from the regenerated ones")
		}
		if b.PartitionSHA256 != partition {
			bad("B3 the partition differs from the regenerated one")
		}
		if b.HostFactsSHA256 != s.HostFactsSHA256 {
			bad("B3 host-facts.json is not the file the binding names")
		}
		if b.InventorySHA256 != s.InventorySHA256 {
			bad("B3 inventory.json is not the file the binding names")
		}
		m.checkRecordSet(s)
	}
}

// checkRecordSet is B4: every record line carries the shard's binding id, and
// the shard's records are exactly one per package it runs.
func (m *merger) checkRecordSet(s *recfmt.Shard) {
	i := s.ArtifactIndex
	seen := map[string]bool{}
	for k := range s.Records {
		r := &s.Records[k]
		if err := r.Validate(); err != nil {
			m.void("binding shard %d: B4 %v", i, err)
			m.malformed = true
			continue
		}
		switch {
		case r.BindingID != s.BindingID:
			m.void("binding shard %d: B4 a record of another binding: %s", i, r.Action)
		case seen[r.Action]:
			m.void("binding shard %d: B4 two records for %s", i, r.Action)
		case !m.planPkgs[i][r.Action]:
			m.void("binding shard %d: B4 a record for %s, which the shard does not run", i, r.Action)
		}
		seen[r.Action] = true
	}
	for pkg := range m.planPkgs[i] {
		if !seen[pkg] {
			m.void("binding shard %d: B4 no record for %s", i, pkg)
		}
	}
}

// buildActions groups every record of every shard by action.  Records that
// failed validation are still grouped (the run is void already) so the report
// sees them.
func (m *merger) buildActions() {
	names := map[string]bool{}
	for _, s := range m.shards {
		for k := range s.Records {
			names[s.Records[k].Action] = true
		}
	}
	sorted := make([]string, 0, len(names))
	for n := range names {
		sorted = append(sorted, n)
	}
	sort.Strings(sorted)
	for id, n := range sorted {
		a := &act{name: n, id: id, assigned: -1}
		a.rs.mt = m.mt
		if s, ok := m.assigned[n]; ok {
			a.assigned = s
		}
		m.acts[n] = a
		m.ordered = append(m.ordered, a)
	}
	for _, s := range m.shards {
		for k := range s.Records {
			a := m.acts[s.Records[k].Action]
			a.copies = append(a.copies, &copyRec{sh: s, rec: &s.Records[k]})
		}
	}
	for _, a := range m.ordered {
		// Copies in a fixed order, whatever order the shards were given.
		sort.SliceStable(a.copies, func(x, y int) bool {
			if a.copies[x].sh.ArtifactIndex != a.copies[y].sh.ArtifactIndex {
				return a.copies[x].sh.ArtifactIndex < a.copies[y].sh.ArtifactIndex
			}
			return a.copies[x].sh.BindingID < a.copies[y].sh.BindingID
		})
		w, tw, b := map[string]bool{}, map[string]bool{}, map[string]bool{}
		for _, c := range a.copies {
			for _, p := range c.rec.Record.Writes {
				w[p] = true
			}
			for _, p := range c.rec.Record.TrackedWrites {
				tw[p] = true
			}
			for _, p := range c.rec.Record.Binds {
				b[p] = true
			}
		}
		a.w, a.tw, a.binds = keys(w), keys(tw), keys(b)
	}
}

func keys(m map[string]bool) []string {
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// ---------------------------------------------------------------------------
// B5, B6.

// checkWeakKeys is B5: the merge recomputes every record's weak key from the
// shard's machine facts and its OWN Source, and requires the same answer.  A
// shard that computed its key from different bytes (a rewritten BUILD, a
// different image, another environment) is refused, and no digest of the
// shard's is ever used.
func (m *merger) checkWeakKeys() {
	if m.in.Keys == nil {
		m.void("keys: the merge has no key source")
		return
	}
	for _, a := range m.ordered {
		for _, c := range a.copies {
			r, i := c.rec, c.sh.ArtifactIndex
			k, refusal, err := m.in.Keys.Weak(c.sh.Facts, a.name)
			switch {
			case err != nil:
				m.void("keys: %s: %v", a.name, err)
			case r.Weak != "" && refusal != "":
				m.void("binding shard %d: B5 the merge refuses %s (%s), the shard keyed it", i, a.name, refusal)
			case r.Weak != "" && r.Weak != hex.EncodeToString(k[:]):
				m.void("binding shard %d: B5 the weak key of %s differs", i, a.name)
			case r.Refusal != "" && refusal != r.Refusal:
				m.void("binding shard %d: B5 the shard refused %s (%s), the merge says %q", i, a.name, r.Refusal, refusal)
			}
		}
	}
}

// checkRecipes is B6: a built action's steps must be the lines the merge's own
// build tool says this OS runs.  A BUILD rewritten mid-run would show here.
func (m *merger) checkRecipes() {
	for _, a := range m.ordered {
		for _, c := range a.copies {
			if c.rec.Status != recfmt.StatusBuilt {
				continue
			}
			if want, ok := m.in.Expected.Steps[a.name]; !ok || want != c.rec.StepsSHA {
				c.recipeBad = true
				a.rs.add("trace-incomplete:recipe-mismatch")
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Headers (M3, M4).

func (m *merger) checkHeaders() {
	for _, s := range m.shards {
		i, h := s.ArtifactIndex, &s.Header
		if h.PostInventoryUnexplained > 0 { // M3
			m.void("M3 shard %d: %d paths on the disk after the build are explained by no trace", i, h.PostInventoryUnexplained)
		}
		if h.TrackedChangedUnexplained > 0 { // M-2
			m.void("M2b shard %d: %d changed tracked paths are covered by no tracked_writes", i, h.TrackedChangedUnexplained)
		}
		for _, tok := range h.Void {
			switch tok {
			case "survivor", "records-too-large": // M3
				m.void("M3 shard %d: %s", i, tok)
			case "tracked-changed", "state-preloaded": // M4
				if m.shardVoid[i] == nil {
					m.shardVoid[i] = map[string]bool{}
				}
				m.shardVoid[i][tok] = true
			default:
				m.void("shard %d: unknown void token %q", i, tok)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// Indexes.

func (m *merger) buildIndexes() {
	m.wIx, m.twIx = newPathIndex(m.mt), newPathIndex(m.mt)
	taint := map[string]bool{}
	for _, a := range m.ordered {
		for _, p := range a.w {
			m.wIx.add(p, a.id)
		}
		for _, p := range a.tw {
			m.twIx.add(p, a.id)
			taint[p] = true
		}
	}
	m.wIx.freeze()
	m.twIx.freeze()
	m.tainted = newPathSet(keys(taint), m.mt)
	for _, s := range m.shards {
		if _, ok := m.shardInv[s]; !ok {
			m.shardInv[s] = newPathSet(s.Inventory.Paths, m.mt)
		}
		ix := newPathIndex(m.mt)
		for _, a := range m.ordered {
			for _, c := range a.copies {
				if c.sh != s {
					continue
				}
				for _, p := range c.rec.Record.Writes {
					ix.add(p, a.id)
				}
			}
		}
		ix.freeze()
		m.shardW[s] = ix
	}
}

// ---------------------------------------------------------------------------
// M1, M2: completeness.  Both are run-void.

func hasKind(l []string, kind string) bool {
	for _, r := range l {
		if recfmt.ReasonKind(r) == kind {
			return true
		}
	}
	return false
}

// Classes of unknown write set.
const (
	unkNone      = iota
	unkHard      // the trace or the record cannot be trusted: run-void in both verdicts
	unkTruncated // the action did not run to completion (M-1): run-void when strict
)

// isDaemonKind lists the reason kinds that mean a process outlived a step.
// "daemon" is a server the action started (and bound or reached); "daemon-left"
// is a process still running when the step ended.  Both matter to M13; the
// second is also a truncated execution, because the survivor may still write.
func isDaemonKind(k string) bool { return k == "daemon" || k == "daemon-left" }

func hasDaemon(l []string) bool {
	for _, r := range l {
		if isDaemonKind(recfmt.ReasonKind(r)) {
			return true
		}
	}
	return false
}

// unknown says why a copy's write set is unknown, and in which class.
func (m *merger) unknown(c *copyRec) (string, int) {
	r := c.rec
	switch {
	case r.Unknown != "":
		return "unknown_writes " + r.Unknown, unkHard
	case r.Complete != nil && !*r.Complete:
		return "writes_complete is false", unkHard
	case c.recipeBad || hasReason(r.Record.Reasons, "trace-incomplete:recipe-mismatch"):
		// The steps that ran are not the steps the weak key's recipe covers, so
		// what was traced says nothing about what the recipe writes.  No
		// writes_complete claim can repair that (L-1).
		return "recipe-mismatch", unkHard
	case hasKind(r.Record.Reasons, "trace-incomplete"):
		if !m.in.HonourWritesComplete {
			return "trace-incomplete (the classifier does not report writes_complete yet)", unkHard
		}
		if r.Complete == nil {
			return "trace-incomplete without writes_complete", unkHard
		}
	}
	for _, st := range r.Steps {
		if !st.Clean {
			return "a step's tracer did not finish cleanly", unkHard
		}
	}
	switch {
	case r.Status != recfmt.StatusBuilt:
		// A failed action stopped part way, whatever steps_run says: the build
		// on main, which does not fail here, would have run the rest (M-1).
		return "status " + r.Status, unkTruncated
	case r.StepsRun < r.StepsTotal:
		return "steps-missing", unkTruncated
	case hasKind(r.Record.Reasons, "daemon-left"):
		return "daemon-left", unkTruncated
	}
	return "", unkNone
}

func hasReason(l []string, want string) bool {
	for _, r := range l {
		if r == want {
			return true
		}
	}
	return false
}

func (m *merger) completeness() {
	for _, a := range m.ordered {
		for _, c := range a.copies {
			switch why, class := m.unknown(c); class { // M1
			case unkHard:
				m.void("M1 shard %d: %s: %s", c.sh.ArtifactIndex, why, a.name)
			case unkTruncated:
				m.voidStrict(a.name, "truncated", "M1 shard %d: %s: %s", c.sh.ArtifactIndex, why, a.name)
			}
			// M2: an untracked path the action observed and did not make must be
			// explained by the pre-BUILD inventory or by another record's write
			// IN THE SAME SHARD (the shards are separate trees, so a writer in
			// another shard explains nothing here).
			inv, sw := m.shardInv[c.sh], m.shardW[c.sh]
			for _, u := range c.rec.Record.UntrackedObservations {
				if !m.mt.spend(1) {
					return
				}
				explained := false
				inv.overlapping(u, func(string) bool { explained = true; return false })
				if !explained {
					sw.overlapping(u, func(_ string, o []int) bool {
						explained = anyOther(o, a.id)
						return !explained
					})
				}
				if !explained {
					m.void("M2 shard %d: %s observed %s, which nothing explains", c.sh.ArtifactIndex, a.name, u)
				}
			}
		}
	}
}

// ---------------------------------------------------------------------------
// M5-M12, M15: the per-action rules.

// others walks index ix over q (by mode) and calls hit for the first maxHits
// paths held by some action other than self.  Stopping at maxHits is sound:
// reasons keep only the smallest maxHits per kind and the walk is bytewise
// ordered (descendants) or nearest-root-first (ancestors, at most the depth).
func others(self int, walkFn func(fn walk) bool, hit func(p string)) {
	n := 0
	walkFn(func(p string, o []int) bool {
		if anyOther(o, self) {
			hit(p)
			n++
		}
		return n < maxHits
	})
}

func (m *merger) perAction() {
	for _, a := range m.ordered {
		if m.mt.over() {
			return
		}
		for _, c := range a.copies {
			rec := &c.rec.Record
			// M15: the record's own reasons, a failed status, a gate, a refusal.
			for _, r := range rec.Reasons {
				if !intraActionKind(recfmt.ReasonKind(r)) {
					a.rs.order = true // M-3: an allow-list, see intraActionKind
				}
				a.rs.add(r)
			}
			if c.rec.Status != recfmt.StatusBuilt {
				a.rs.add("failed")
			}
			if c.rec.Gate != "" {
				a.rs.add("precondition:" + traced.Sanitize(c.rec.Gate))
			}
			if c.rec.Refusal != "" {
				a.rs.add("precondition:" + traced.Sanitize(c.rec.Refusal))
			}
			inv := m.shardInv[c.sh]

			// M5 and M6: an untracked path the action saw, or an absent name it
			// probed, that another action wrote (before or after: another job
			// could run them in the other order).  overlap, so a write under a
			// probed directory and a probed name under a written root both count.
			for _, list := range [][]string{rec.UntrackedObservations, rec.AbsentObservations} {
				for _, u := range list {
					if !m.mt.spend(1) {
						return
					}
					others(a.id, func(fn walk) bool { return m.wIx.overlapping(u, fn) },
						func(w string) { a.rs.product(w) })
				}
			}
			// M9: a pre-existing leftover (the inventory) the action saw or probed.
			for _, list := range [][]string{rec.UntrackedObservations, rec.AbsentObservations} {
				for _, u := range list {
					n := 0
					inv.overlapping(u, func(i string) bool { a.rs.product(i); n++; return n < maxHits })
				}
			}
			for _, t := range rec.Traversed {
				if !m.mt.spend(1) {
					return
				}
				// M9 (traversed): a pre-existing leftover that is, or contains, a
				// component the action walked through.
				inv.ancestors(t, func(string) bool { a.rs.dep("trace-incomplete:unstable-path"); return false })
				// M8: a component another action replaced or removed.
				for _, ix := range []*pathIndex{m.wIx, m.twIx} {
					others(a.id, func(fn walk) bool { return ix.ancestors(t, fn) },
						func(string) { a.rs.dep("trace-incomplete:unstable-path") })
				}
			}
			for _, e := range rec.Entries {
				if !m.mt.spend(1) {
					return
				}
				switch {
				case strings.HasPrefix(e, key.EntryList):
					d := e[len(key.EntryList):]
					// M7: a listing in which another action created or removed an
					// entry.  Any write root that overlaps d counts: beneath d (an
					// entry changed), equal to d, or ABOVE d (the directory itself
					// lies in another action's output).  d == "." is everything.
					others(a.id, func(fn walk) bool { return m.wIx.overlapping(d, fn) },
						func(w string) { a.rs.product(w) })
					// M11 (list): a tainted path that overlaps the listed
					// directory: beneath it (an entry changed) OR above it (the
					// directory itself was moved or replaced).
					n := 0
					m.tainted.overlapping(d, func(t string) bool { a.rs.product(t); n++; return n < maxHits })
				case strings.HasPrefix(e, key.EntryPackage), strings.HasPrefix(e, key.EntryPath):
					p := e[strings.IndexByte(e, ':')+1:]
					// M11 (pkg and path): a tainted path overlapping the entry.
					n := 0
					m.tainted.overlapping(p, func(t string) bool { a.rs.product(t); n++; return n < maxHits })
				}
			}
		}
		// M10: write roots that overlap another action's.  The union over every
		// copy of THIS action is used, and "another action" excludes this
		// action's own copies, so an action never voids itself.
		for _, w := range a.w {
			if !m.mt.spend(1) {
				return
			}
			others(a.id, func(fn walk) bool { return m.wIx.overlapping(w, fn) },
				func(p string) { a.rs.product(p) })
		}
	}
}

// M14: two actions that bind the same inet address (same type, address and
// port) both lose: one's server may have answered the other's client.
func (m *merger) sharedPorts() {
	holders := map[string][]*act{}
	for _, a := range m.ordered {
		for _, b := range a.binds {
			holders[b] = append(holders[b], a)
		}
	}
	for _, h := range holders {
		if len(h) > 1 {
			for _, a := range h {
				a.rs.dep("reads-product:shared-port")
			}
		}
	}
}

// M13 (G11): the record carries only a SANITISED, CAPPED reason list, so a
// daemon's address cannot be matched against other actions' binds.  The rule
// is: when action C reported a daemon (kind "daemon" or "daemon-left", which
// survive the cap because traced.CapReasons keeps one reason of every kind),
// every other action whose writes overlap something C observed untracked, and
// every other action that bound a port, is voided.
//
// It is a superset of an address match ONLY for binds the record lists:
// RawRecord.Binds omits port 0 and abstract or unnamed sockets, so a server
// reached through one of those is not covered.  That is vacuous under §4.7:
// every step has its own network namespace (G10), so two actions can never
// share an inet port or an abstract socket; only a pathname socket inside the
// checkout crosses actions, and that is what the untracked-overlap half
// catches.  No marker is added for it.
func (m *merger) daemons() {
	var ds []*act
	for _, a := range m.ordered {
		for _, c := range a.copies {
			if hasDaemon(c.rec.Record.Reasons) {
				ds = append(ds, a)
				break
			}
		}
	}
	for _, d := range ds {
		for _, c := range d.copies {
			for _, u := range c.rec.Record.UntrackedObservations {
				if !m.mt.spend(1) {
					return
				}
				m.wIx.overlapping(u, func(_ string, o []int) bool {
					for _, id := range o {
						if id != d.id {
							m.ordered[id].rs.dep("reads-product:daemon-peer")
						}
					}
					return true
				})
			}
		}
		for _, a := range m.ordered {
			if !m.mt.spend(1) {
				return
			}
			if a != d && len(a.binds) > 0 {
				a.rs.dep("reads-product:daemon-peer")
			}
		}
	}
}

// M12: a tainted path that is, or lies in, the action's own package or its
// recipe, a .gitignore (it decides what counts as untracked), the build tool
// or Kiln.  A BUILD variant of the action lies inside its package, so the
// package overlap covers it.
func (m *merger) taintedMetadata() {
	var ignores []string
	m.tainted.all(func(t string) bool {
		if t == ".gitignore" || strings.HasSuffix(t, "/.gitignore") {
			ignores = append(ignores, t)
		}
		return true
	})
	common := []string{}
	for _, root := range []string{"code/programs/go/build-tool", "code/kiln"} {
		m.tainted.overlapping(root, func(t string) bool { common = append(common, t); return len(common) < maxHits })
	}
	for _, a := range m.ordered {
		n := 0
		m.tainted.overlapping(a.name, func(t string) bool { a.rs.product(t); n++; return n < maxHits })
		for _, t := range ignores[:min(len(ignores), maxHits)] {
			a.rs.product(t)
		}
		for _, t := range common {
			a.rs.product(t)
		}
		_ = m.mt.spend(int64(len(ignores)%maxHits + len(common) + 1))
	}
}

// ---------------------------------------------------------------------------
// Emission.

// orderDependence is M-3.  An action whose own behaviour depended on another
// action's recorded writes (a reads-product or unstable-path reason derived by
// M5-M14, or already in its record) would write something else in another
// order, so its recorded write set is only one of several possible.  Every
// cross-action rule quantifies over write sets, so one such action is a
// second-order unknown: the strict verdict voids the run.  The relaxed,
// report-only verdict charges it to the action alone.
func (m *merger) orderDependence() {
	for _, a := range m.ordered {
		if a.rs.order {
			m.voidStrict(a.name, "order", "M3 order-dependent: %s read another action's output or listing", a.name)
		}
	}
}

// tooBig is the retained-text cap of KILN04 2.7: the merge holds every path
// and entry of every record, so their total length is bounded.
func (m *merger) tooBig() bool {
	limit := m.in.MaxRetainedBytes
	if limit <= 0 {
		limit = DefaultMaxRetainedBytes
	}
	var n int64
	add := func(l []string) {
		for _, s := range l {
			n += int64(len(s)) + 16
		}
	}
	for _, s := range m.shards {
		add(s.Inventory.Paths)
		add(s.Facts.Env)
		add(s.Facts.ExecutorArgs)
		n += int64(len(s.Facts.Image) + len(s.Facts.Root) + len(s.Facts.Home) + len(s.Facts.Tmpdir))
		for k := range s.Records {
			r := &s.Records[k]
			n += int64(len(r.Action))
			add(r.Record.Entries)
			add(r.Record.Writes)
			add(r.Record.TrackedWrites)
			add(r.Record.AbsentObservations)
			add(r.Record.UntrackedObservations)
			add(r.Record.Traversed)
			add(r.Record.Binds)
			add(r.Record.Reasons)
		}
		if n > limit {
			return true
		}
	}
	return n > limit
}

func (m *merger) finish() Result {
	res := Result{ShardVoid: map[int][]string{}}
	res.RunVoidCount = m.causes.count()
	res.RunVoid = m.causes.list()
	res.RelaxedRunVoidCount = m.relaxed.count()
	res.RelaxedRunVoid = m.relaxed.list()
	res.Emitted = res.RunVoidCount == 0
	relaxedOK := res.RelaxedRunVoidCount == 0
	for i, toks := range m.shardVoid {
		res.ShardVoid[i] = keys(toks)
	}
	at := m.in.Now.UTC().Truncate(time.Second)
	for _, a := range m.ordered {
		o := Outcome{Action: a.name, Assigned: a.assigned, Recordable: !a.rs.any,
			Truncated: m.relaxLost[a.name] == "truncated", OrderDependent: a.rs.order}
		o.ShardVoid = a.assigned >= 0 && len(m.shardVoid[a.assigned]) > 0
		if (res.Emitted || relaxedOK) && m.in.Keys != nil && !a.rs.any && !o.ShardVoid && a.assigned >= 0 {
			if man, pass, why := m.emit(a, at); why == "" {
				// The relaxed verdict only needs to know the action COULD be
				// emitted; nothing is appended unless the strict run is clean.
				o.RelaxedEmittable = relaxedOK && m.relaxLost[a.name] == ""
				if res.Emitted {
					o.Emitted = true
					res.Manifests = append(res.Manifests, man)
					res.Passes = append(res.Passes, pass)
				}
			} else {
				a.rs.add(why)
			}
		}
		o.Reasons = traced.CapReasons(a.rs.m)
		if o.Reasons == nil {
			o.Reasons = []string{}
		}
		res.Outcomes = append(res.Outcomes, o)
	}
	return res
}

// emit builds the manifest and pass for an action that survived every rule,
// from the copy in its ASSIGNED shard (G3).  A non-empty why means the action
// cannot be emitted after all, and is added as its reason.
func (m *merger) emit(a *act, at time.Time) (ledger.Manifest, ledger.Record, string) {
	var c *copyRec
	for _, cc := range a.copies {
		if cc.sh.ArtifactIndex == a.assigned {
			c = cc
		}
	}
	if c == nil {
		return ledger.Manifest{}, ledger.Record{}, "manifest-invalid:no record in the assigned shard"
	}
	names := c.rec.Record.Entries
	if len(names) > ledger.MaxManifestEntries {
		return ledger.Manifest{}, ledger.Record{}, "manifest-too-large"
	}
	own := false
	for _, n := range names {
		if len(n) > ledger.MaxManifestNameBytes {
			return ledger.Manifest{}, ledger.Record{}, "manifest-too-large"
		}
		if err := key.CheckEntryName(n); err != nil {
			return ledger.Manifest{}, ledger.Record{}, "manifest-invalid:" + traced.Sanitize(err.Error())
		}
		own = own || n == key.EntryPackage+a.name
	}
	if !own {
		// Without pkg:<action> a change to the action's own sources would still
		// hit.  The classifier always adds it, so its absence means a record
		// the classifier did not write.
		return ledger.Manifest{}, ledger.Record{}, "manifest-invalid:no pkg entry for the action itself"
	}
	raw, err := hex.DecodeString(c.rec.Weak)
	if err != nil || len(raw) != 32 {
		return ledger.Manifest{}, ledger.Record{}, "manifest-invalid:weak key"
	}
	var weak [32]byte
	copy(weak[:], raw)
	strong, err := m.in.Keys.Strong(c.sh.Facts, a.name, names)
	if err != nil {
		return ledger.Manifest{}, ledger.Record{}, "manifest-invalid:strong key: " + traced.Sanitize(err.Error())
	}
	man := ledger.Manifest{Weak: weak, Target: a.name, OS: osName, Arch: archName,
		Entries: append([]string(nil), names...), Revision: m.in.Run.Revision, Run: m.in.Run.ID, At: at}
	pass := ledger.Record{Key: strong, Target: a.name, OS: osName, Arch: archName, Kiln: m.in.Kiln,
		Revision: m.in.Run.Revision, Run: m.in.Run.ID, At: at, Kind: ledger.KindPass}
	if err := man.Validate(); err != nil {
		return ledger.Manifest{}, ledger.Record{}, "manifest-invalid:" + traced.Sanitize(err.Error())
	}
	if err := pass.Validate(); err != nil {
		return ledger.Manifest{}, ledger.Record{}, "manifest-invalid:" + traced.Sanitize(err.Error())
	}
	return man, pass, ""
}
