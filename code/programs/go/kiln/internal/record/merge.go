package record

// The merge job (KILN04 section 7.1, stage 4 PR 3a)
// =========================================================================
//
// A recording shard records RAW observations and decides nothing: `kiln record
// collect` writes one `records.jsonl` per shard, and no shard may turn its own
// records into a manifest or a pass, because section 7's rules are cross-shard
// (another action's leftover, another shard's write, a shared port, a daemon).
// This file is the other half: the job that reads every shard, applies those
// rules once, and writes the ledger.
//
// The rules themselves are NOT here. They are `recmerge.Merge`, which is pure,
// fuzz-tested against a brute-force oracle, and already implements B1-B6 (the
// binding) and M1-M15 (the cross-action rules). This file is the job around
// it, and it is deliberately thin, because every decision it could make is a
// decision `recmerge` already makes:
//
//	verify our own tree       B1: the checkout is byte-for-byte the revision
//	read the exports          graph.json, steps.json, build-plan.json
//	decode the shards         recfmt.DecodeShard, from the exact uploaded bytes
//	recompute every key       TracedKeys, from OUR Source, never from a shard
//	recmerge.Merge            B1-B6, M1-M15
//	write the ledger          only when the run is not void
//
// The trust boundary is the same one `prepare` and `collect` use: the job is
// only meaningful in a trusted run, `legacy.NewTraced` fails closed on
// anything it cannot key, and an unverifiable input voids the run instead of
// being repaired.
//
// Two things this file deliberately does NOT do:
//
//   - It does not run the build tool. It takes the three exports as inputs
//     (`--graph`, `--steps`, `--plan`), exactly as `kiln legacy-plan` takes
//     `--graph`, so the job is a pure function of (its own verified tree, the
//     exports, the shard artifacts) and can be tested anywhere. The CI job
//     that produces the exports runs the build tool at the base revision, as
//     `prepare` does.
//   - It does not write anything when the verdict is void (M1): "all or
//     nothing" is rule 3 of section 7, and a partial emit is the one way this
//     job could produce a false hit.

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/ledger"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recmerge"
)

// The names of the four files a shard artifact must hold. They are the sealed
// `pre/` names `prepare` writes and `collect` copies into the shard artifact.
const (
	ShardBindingFile   = "binding.json"
	ShardHostFactsFile = "host-facts.json"
	ShardInventoryFile = "inventory.json"
	ShardRecordsFile   = "records.jsonl"
)

// MaxShardFileBytes bounds any one shard file the job reads. `recfmt` enforces
// the real per-format caps; this is the outer bound so a huge file is refused
// before it is read into memory.
const MaxShardFileBytes = 256 << 20

// ShardArtifact is one uploaded shard: the directory its artifact unpacked
// into, and the shard index read from the artifact's NAME (never from a file
// inside it, so a shard cannot claim to be another shard).
type ShardArtifact struct {
	Index int
	Dir   string
}

// MergeOptions is everything the merge job needs. Every field is required
// unless its comment says otherwise.
type MergeOptions struct {
	// Language is the merge job's own declaration of the recording selection
	// (record-merge --language): a language name, or ""/"all" for every
	// language. Every shard's binding must equal it, and with "all" the plan
	// must cover every graph package (KILN04 7.3).
	Language string
	// Root is the checkout. It must be a repository top level, and its tree
	// must be byte-for-byte Run.Revision (B1).
	Root string
	// Source reads the tracked tree. It is required, and it must be the
	// hardened Git source (`source.NewGitAt` with `source.PinnedEnv`), because
	// every digest and key below is computed through it; a Source that could
	// follow a symlink or reach a hook would key something other than the
	// commit. A nil Source is refused rather than defaulted.
	Source legacy.Source
	// GraphPath, StepsPath and PlanPath are the build tool's exports at the
	// base revision, the same three `prepare` seals.
	GraphPath string
	StepsPath string
	PlanPath  string
	// ShardCount is the shard count the plan and every shard must agree on.
	ShardCount int
	// Shards is every shard artifact, one per index in [0, ShardCount).
	Shards []ShardArtifact
	// Run identifies the workflow run the shards belong to.
	Run recmerge.Run
	// Kiln is the ledger record's "kiln" field, e.g.
	// "kiln/0.1.0 traced/2 recorder/2".
	Kiln string
	// Now stamps the manifests and passes (whole seconds, UTC).
	Now time.Time
	// HonourWritesComplete is recmerge's switch (section 7.1, G1). It stays
	// false: the classifier computes writes_complete, but turning the relaxed
	// verdict on changes what a cache hit means and is a separate decision.
	HonourWritesComplete bool
	// WorkBudget and MaxRetainedBytes bound recmerge's work (0 = its default).
	WorkBudget       int64
	MaxRetainedBytes int64

	// LedgerDir is the ledger directory to write. It is required unless
	// DryRun is set.
	LedgerDir string
	// Scope is the ledger scope: ledger.Shared in CI, ledger.Local on a
	// developer's machine. The Authority decides whether the write is allowed.
	Scope ledger.Scope
	// Authority is the run's trust. A shared ledger needs Trusted().
	Authority ledger.Authority
	// DryRun computes the verdict and writes nothing, which is what a report
	// run and every test does.
	DryRun bool
}

// MergeReport is what the job did, for the job log and for a test.
type MergeReport struct {
	// Revision is the commit B1 verified the tree is.
	Revision string
	// Verdict is recmerge's result: the void causes, the outcomes and the
	// records that were (or would be) written.
	Verdict recmerge.Result
	// Written is true when the ledger was actually written.
	Written bool
	// Manifests and Passes are the counts that were (or would be) written.
	Manifests, Passes int
	// Recordable, Actions are the recordable actions of the total, which is
	// the number the kill criterion (section 13) is computed from.
	Recordable, Actions int
	// Reasons is the per-reason count over every action, sorted.
	Reasons []ReasonCount
	// PlanPackages is the number of actions the regenerated plan assigns, so a
	// reader can tell an empty run from a refused one.
	PlanPackages int
	// Language is the recording selection the merge job declared and every
	// shard's binding was required to seal ("" is every language).
	Language string
	// ForeignDropped counts the actions that lost their record because they
	// observed an absent or untracked path inside a package the selection left
	// out (recmerge.ForeignRead). A count, never a path.
	ForeignDropped int
}

// expectedFor is everything the merge regenerates from its own exports (D10),
// including the recording selection (KILN04 7.3).
//
// language is the MERGE JOB'S OWN declaration (record-merge --language; "" is
// every language). It is not read from the shards: a shard's binding is
// something the recorder wrote, and a run whose bindings all omitted the field
// would otherwise turn the foreign rule off by itself. recmerge instead
// requires every binding to EQUAL this value (B3), so a blank binding under
// `go`, and a `go` binding under the default, are both void.
//
// Under a selection the steps export stays WHOLE (like the graph it describes
// the repository, not the run), so it names packages the plan does not run:
// those are ignored. With every language recorded such an entry is still a
// broken input and is refused, exactly as before.
//
// The unselected ("foreign") set is OUR graph minus OUR plan, computed for every
// selection. With the default selection it must be EMPTY (the plan covers every
// graph package), or recmerge voids the run: a language-restricted plan can
// never be read as `all`. Under a selection, a plan package of another language
// and a plan package with a prerequisite outside the plan (closure, checked on
// OUR graph) are both void as well. The shards' records play no part here.
func expectedFor(graph *legacy.Graph, steps *legacy.Steps, part *Partition, language, graphSHA, stepsSHA string) (recmerge.Expected, error) {
	exp := recmerge.Expected{
		GraphSHA256: graphSHA,
		StepsSHA256: stepsSHA,
		Plan:        append([]recfmt.ShardPlan(nil), part.Shards...),
		Steps:       map[string]string{},
		Language:    language,
	}
	// B6: the steps the merge's own tree runs, per package rel_path. A steps
	// entry's "package" field IS the graph's rel_path (legacy.Steps.Lines is
	// looked up that way), so no name -> path mapping is involved; the plan is
	// consulted only to refuse an export that names something no shard runs,
	// which is a broken input rather than a void run -- nothing a shard could
	// have caused.
	run := map[string]bool{}
	for _, sh := range part.Shards {
		for _, rel := range sh.Packages {
			run[rel] = true
		}
	}
	seen := map[string]bool{}
	for _, p := range steps.Packages {
		if err := key.ValidatePath(p.Package); err != nil {
			return exp, fmt.Errorf("steps export names %q, which is not a path: %v", printable(p.Package), err)
		}
		if seen[p.Package] {
			return exp, fmt.Errorf("steps export names %q twice", printable(p.Package))
		}
		seen[p.Package] = true
		if !run[p.Package] {
			if exp.Language != "" {
				continue
			}
			return exp, fmt.Errorf("steps export names %q, which no shard of the plan runs", printable(p.Package))
		}
		exp.Steps[p.Package] = recfmt.StepsDigest(p.Steps)
	}
	byName := make(map[string]string, len(graph.Packages))
	for _, p := range graph.Packages {
		byName[p.Name] = p.RelPath
	}
	for _, p := range graph.Packages {
		if !run[p.RelPath] {
			exp.Foreign = append(exp.Foreign, p.RelPath)
			continue
		}
		if exp.Language == "" {
			continue
		}
		if p.Language != exp.Language {
			exp.PlanOutsideSelection++
		}
		// Closure: every prerequisite, on every platform, test-trigger
		// providers (dev deps) included, is a package the plan runs. The build
		// tool checked this when it chose the packages; the merge checks it
		// again on the graph it already loaded instead of trusting that.
		open := false
		check := func(name string) {
			if rel, ok := byName[name]; !ok || !run[rel] {
				open = true
			}
		}
		for _, os := range legacy.OSes {
			for _, d := range p.Deps[os] {
				check(d)
			}
		}
		for _, d := range p.DevDeps {
			check(d)
		}
		if open {
			exp.PlanNotClosed++
		}
	}
	sort.Strings(exp.Foreign)
	return exp, nil
}

// ReasonCount is one row of the report's reason histogram.
type ReasonCount struct {
	Reason string
	Count  int
}

// Merge runs the merge job. It returns an error only when the JOB itself is
// broken or an input cannot be read: every other outcome, including a void
// run, is a *MergeReport with rectified cause names and a nil error, exactly
// as `collect` treats a void shard.
func Merge(o MergeOptions) (*MergeReport, error) {
	if o.Root == "" || o.GraphPath == "" || o.StepsPath == "" || o.PlanPath == "" {
		return nil, errors.New("root, graph, steps and plan are required")
	}
	if o.ShardCount < 1 || o.ShardCount > recfmt.MaxShards {
		return nil, fmt.Errorf("shard count %d is outside 1..%d", o.ShardCount, recfmt.MaxShards)
	}
	if o.Kiln == "" {
		return nil, errors.New("the ledger record's kiln id is required")
	}
	if o.Run.Repository == "" || o.Run.ID == "" || o.Run.Revision == "" {
		return nil, errors.New("repository, run id and revision are required")
	}
	if !o.DryRun {
		if o.LedgerDir == "" {
			return nil, errors.New("a ledger directory is required unless --dry-run is set")
		}
		if o.Scope != ledger.Shared && o.Scope != ledger.Local {
			return nil, fmt.Errorf("scope %q must be shared or local", o.Scope)
		}
		// Fail before any work when the run may not write this scope: a merge
		// job that computes a full verdict and then cannot record it has done
		// the expensive half for nothing.
		if err := ledger.CheckAuthority(o.Authority, o.Scope); err != nil {
			return nil, err
		}
	}
	if o.Source == nil {
		return nil, errors.New("a Source is required (the keys are computed through it)")
	}
	root, err := filepath.Abs(o.Root)
	if err != nil {
		return nil, fmt.Errorf("root: %w", err)
	}
	rep := &MergeReport{}

	// B1. Our own tree is the revision every shard claims, or the whole run is
	// meaningless: every digest and key below is computed from it.
	verified, err := VerifyTree(root, o.Run.Revision)
	if err != nil {
		return nil, fmt.Errorf("B1: %w", err)
	}
	rep.Revision = verified.Commit

	// The three exports. Each is read ONCE: the bytes that are hashed for the
	// binding are the bytes that are parsed (the same rule `prepare` keeps).
	graphData, err := readRegular(o.GraphPath, legacy.MaxGraphBytes, "graph")
	if err != nil {
		return nil, err
	}
	stepsData, err := readRegular(o.StepsPath, legacy.MaxStepsBytes, "steps")
	if err != nil {
		return nil, err
	}
	planData, err := readRegular(o.PlanPath, MaxPlanBytes, "plan")
	if err != nil {
		return nil, err
	}
	graph, err := legacy.ParseGraph(graphData)
	if err != nil {
		return nil, fmt.Errorf("graph: %w", err)
	}
	steps, err := legacy.DecodeSteps(stepsData)
	if err != nil {
		return nil, fmt.Errorf("steps: %w", err)
	}
	if steps.OS != "linux" {
		return nil, fmt.Errorf("steps export is for %q, not linux", printable(steps.OS))
	}
	part, err := ParsePartition(planData, o.ShardCount)
	if err != nil {
		return nil, err
	}
	// The selection is the merge job's own declaration (o.Language), validated
	// like a binding's; every shard's binding must equal it (B3).
	language := bindingLanguage(o.Language)
	if err := recfmt.CheckLanguage(language); err != nil {
		return nil, fmt.Errorf("--language: %w", err)
	}
	exp, err := expectedFor(graph, steps, part, language, sum256(graphData), sum256(stepsData))
	if err != nil {
		return nil, err
	}
	// The shards. Every one is decoded from the exact bytes read here, so the
	// binding check in recmerge compares against what was really uploaded.
	shards, err := decodeShards(o.Shards)
	if err != nil {
		return nil, err
	}
	rep.Language = exp.Language
	for _, sh := range part.Shards {
		rep.PlanPackages += len(sh.Assigned)
	}

	// The keys, from OUR tree. Never from a shard.
	keys, err := NewTracedKeys(TracedKeysOptions{
		Graph:  graph,
		Source: o.Source,
		Root:   root,
	})
	if err != nil {
		return nil, err
	}
	if err != nil {
		return nil, err
	}

	rep.Verdict = recmerge.Merge(recmerge.Input{
		Run:                  o.Run,
		Expected:             exp,
		Shards:               shards,
		Keys:                 keys,
		Now:                  o.Now,
		Kiln:                 o.Kiln,
		HonourWritesComplete: o.HonourWritesComplete,
		WorkBudget:           o.WorkBudget,
		MaxRetainedBytes:     o.MaxRetainedBytes,
	})
	rep.Manifests = len(rep.Verdict.Manifests)
	rep.Passes = len(rep.Verdict.Passes)
	rep.Actions = len(rep.Verdict.Outcomes)
	for _, oc := range rep.Verdict.Outcomes {
		if oc.Recordable {
			rep.Recordable++
		}
	}
	rep.Reasons = countReasons(rep.Verdict.Outcomes)
	for _, oc := range rep.Verdict.Outcomes {
		for _, r := range oc.Reasons {
			if r == recmerge.ForeignRead {
				rep.ForeignDropped++
				break
			}
		}
	}

	// All or nothing (section 7, rule 3). A void run writes nothing: not the
	// manifests it managed to validate, not the passes.
	if !rep.Verdict.Emitted || o.DryRun {
		return rep, nil
	}
	if err := writeLedger(o, rep.Verdict); err != nil {
		return nil, err
	}
	rep.Written = true
	return rep, nil
}

// writeLedger writes the manifests and then the passes of a non-void run.
//
// Manifests first, passes second, and never the other way round: a pass whose
// manifest is missing is inert (stage 5 needs both: the manifest supplies the
// entry names, the pass says the strong key passed), while a manifest whose
// pass is missing is equally inert. So the order cannot create a hit that the
// verdict did not grant, and a failure between the two leaves the ledger
// exactly as consistent as it was.
func writeLedger(o MergeOptions, v recmerge.Result) error {
	if len(v.Manifests) > 0 {
		ms := ledger.OpenManifests(o.LedgerDir, o.Scope)
		if err := ms.Record(v.Manifests, o.Authority, o.Now); err != nil {
			return fmt.Errorf("write manifests: %w", err)
		}
	}
	if len(v.Passes) > 0 {
		st := ledger.Open(o.LedgerDir, o.Scope)
		if err := st.Record(v.Passes, o.Authority, o.Now); err != nil {
			return fmt.Errorf("write passes: %w", err)
		}
	}
	return nil
}

// decodeShards reads and strictly decodes every shard artifact. An unreadable
// or malformed artifact is an error, not a void run: it means the job cannot
// see what it must judge, and guessing would be the one way to produce a false
// hit from a missing input.
func decodeShards(arts []ShardArtifact) ([]*recfmt.Shard, error) {
	seen := map[int]bool{}
	dirs := map[string]bool{}
	out := make([]*recfmt.Shard, 0, len(arts))
	for _, a := range arts {
		if a.Index < 0 {
			return nil, fmt.Errorf("shard %q has a negative index", printable(a.Dir))
		}
		if seen[a.Index] {
			return nil, fmt.Errorf("shard %d was supplied twice", a.Index)
		}
		seen[a.Index] = true
		dir, err := filepath.Abs(a.Dir)
		if err != nil {
			return nil, fmt.Errorf("shard %d: %w", a.Index, err)
		}
		if dirs[dir] {
			return nil, fmt.Errorf("shard %d: directory %q was supplied twice", a.Index, printable(dir))
		}
		dirs[dir] = true
		binding, err := readRegular(filepath.Join(dir, ShardBindingFile), recfmt.MaxBindingBytes, "binding")
		if err != nil {
			return nil, err
		}
		facts, err := readRegular(filepath.Join(dir, ShardHostFactsFile), recfmt.MaxHostFactsBytes, "host facts")
		if err != nil {
			return nil, err
		}
		inv, err := readRegular(filepath.Join(dir, ShardInventoryFile), recfmt.MaxInventoryBytes, "inventory")
		if err != nil {
			return nil, err
		}
		records, err := openRegular(filepath.Join(dir, ShardRecordsFile), "records")
		if err != nil {
			return nil, err
		}
		s, derr := recfmt.DecodeShard(a.Index, recfmt.ShardFiles{
			Binding: binding, HostFacts: facts, Inventory: inv, Records: records,
		}, recfmt.DefaultLimits)
		cerr := records.Close()
		if derr != nil {
			return nil, fmt.Errorf("shard %d: %w", a.Index, derr)
		}
		if cerr != nil {
			return nil, fmt.Errorf("shard %d: %w", a.Index, cerr)
		}
		out = append(out, s)
	}
	return out, nil
}

// countReasons is the report's reason histogram. Every cause name is one
// recmerge produced, so it is already a fixed token without path text
// (section 7.1, M15); the report never invents one.
func countReasons(ocs []recmerge.Outcome) []ReasonCount {
	m := map[string]int{}
	for _, oc := range ocs {
		for _, r := range oc.Reasons {
			m[r]++
		}
	}
	out := make([]ReasonCount, 0, len(m))
	for r, n := range m {
		out = append(out, ReasonCount{Reason: r, Count: n})
	}
	sort.Slice(out, func(a, b int) bool { return out[a].Reason < out[b].Reason })
	return out
}

// readRegular reads a regular file no larger than max. A symlink, a FIFO, a
// device or a directory is refused rather than followed, so a planted path
// cannot redirect the job at another file.
func readRegular(p string, max int64, what string) ([]byte, error) {
	f, err := openRegular(p, what)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	data, err := io.ReadAll(io.LimitReader(f, max+1))
	if err != nil {
		return nil, fmt.Errorf("%s %q: %w", what, printable(p), err)
	}
	if int64(len(data)) > max {
		return nil, fmt.Errorf("%s %q is over the %d byte limit", what, printable(p), max)
	}
	return data, nil
}

// openRegular opens p for reading only if it is a regular file, checking the
// descriptor rather than the path so a swap between the two cannot help.
func openRegular(p, what string) (*os.File, error) {
	st, err := os.Lstat(p)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", what, err)
	}
	if !st.Mode().IsRegular() {
		return nil, fmt.Errorf("%s %q is not a regular file", what, printable(p))
	}
	f, err := os.Open(p)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", what, err)
	}
	fst, err := f.Stat()
	if err != nil {
		f.Close()
		return nil, fmt.Errorf("%s: %w", what, err)
	}
	if !fst.Mode().IsRegular() || !os.SameFile(st, fst) {
		f.Close()
		return nil, fmt.Errorf("%s %q is not a regular file", what, printable(p))
	}
	return f, nil
}

// sum256 is the hex SHA-256 of the bytes, which is what a binding names.
func sum256(b []byte) string {
	d := sha256.Sum256(b)
	return hex.EncodeToString(d[:])
}
