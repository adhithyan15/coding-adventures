package legacy

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// PlanSchema names the plan format.
const PlanSchema = "kiln-legacy-plan/1"

// MaxPlanBytes caps a plan read back by `kiln explain`.
const MaxPlanBytes = 256 << 20

// maxBuildBytes caps one BUILD file read for the recipe.
const maxBuildBytes = 16 << 20

// Cacheability classes (KILN03 section 7).
const (
	ClassCacheable   = "cacheable"
	ClassUnpinned    = "unpinned"
	ClassUncacheable = "uncacheable"
)

// ReasonToolsEnv marks work still undone (7c-3).  Until tool and environment
// identity exist, no package may be called cacheable, so every package carries
// it.  It is an "incomplete" reason, not a hard one: it stops a package being
// cacheable but does not hide the more informative `unpinned` class.  (The
// escape scans landed in 7c-2; their reasons are hard.)
const ReasonToolsEnv = "m1-incomplete:tools-env"

// Components are the digests a key is made of, so two plans can be compared
// to see WHICH part changed (KILN03 section 9).  Tools and env are empty in
// 7c-1 and therefore not listed.
type Components struct {
	Own      string `json:"own"`
	Reads    string `json:"reads"`
	Consumed string `json:"consumed"`
	Recipe   string `json:"recipe"`
}

// Record is one package's entry in a plan.
type Record struct {
	Name        string     `json:"name"`
	Key         string     `json:"key"`
	Class       string     `json:"class"`
	Reasons     []string   `json:"reasons"`
	ClosureSize int        `json:"closure_size"`
	Components  Components `json:"components"`
}

// Sizes summarises closure sizes.
type Sizes struct {
	Min    int `json:"min"`
	Median int `json:"median"`
	P90    int `json:"p90"`
	Max    int `json:"max"`
}

// Histogram is the first number M2 needs (KILN03 section 9).
type Histogram struct {
	Classes      map[string]int `json:"classes"`
	Reasons      map[string]int `json:"reasons"`
	ClosureSizes Sizes          `json:"closure_sizes"`
}

// Plan is the output of `kiln legacy-plan`.
type Plan struct {
	Schema    string    `json:"schema"`
	OS        string    `json:"os"`
	Arch      string    `json:"arch"`
	Host      string    `json:"host"`
	Packages  []Record  `json:"packages"`
	Histogram Histogram `json:"histogram"`
}

// refuseDirty fails if the Source can report untracked, non-ignored files and
// reports any (KILN03 section 9): build products would otherwise enter D(p).
func refuseDirty(src Source) error {
	c, ok := src.(source.Cleaner)
	if !ok {
		return nil
	}
	u, err := c.Untracked()
	if err != nil {
		return err
	}
	if len(u) == 0 {
		return nil
	}
	shown := make([]string, 0, 3)
	for _, f := range u[:min(3, len(u))] {
		shown = append(shown, short(f))
	}
	return fmt.Errorf("tree is not clean: %d untracked file(s), e.g. %s (run legacy-plan before any BUILD)",
		len(u), strings.Join(shown, ", "))
}

// BuildPlan computes every package's legacy key for opt.OS (KILN03).
func BuildPlan(g *Graph, opt Options) (*Plan, error) {
	if opt.Source == nil || opt.Host == nil {
		return nil, fmt.Errorf("legacy plan: source and host are required")
	}
	known := false
	for _, o := range OSes {
		known = known || o == opt.OS
	}
	if !known {
		return nil, fmt.Errorf("legacy plan: unknown os %q (want linux, darwin or windows)", short(opt.OS))
	}
	if opt.Arch != "x64" && opt.Arch != "arm64" {
		return nil, fmt.Errorf("legacy plan: unknown arch %q", short(opt.Arch))
	}
	if err := refuseDirty(opt.Source); err != nil {
		return nil, err
	}
	pl, err := newPlanner(g, opt)
	if err != nil {
		return nil, err
	}
	names := make(map[string]bool, len(g.Packages))
	for _, p := range g.Packages {
		names[p.Name] = true
	}
	if pl.pol, err = loadPolicy(opt.Source, names); err != nil {
		return nil, err
	}
	data, err := readSmall(opt.Source, BoundaryPath, maxPolicyBytes)
	if err != nil {
		return nil, err
	}
	if pl.bounds, err = parseBoundaries(data); err != nil {
		return nil, err
	}
	if err := pl.computeOwn(); err != nil {
		return nil, err
	}
	pl.memberItems()
	if _, err := pl.buildToolDigest(); err != nil {
		return nil, err
	}
	recs := make([]Record, len(g.Packages))
	if err := forEach(len(recs), opt.workers(), func(i int) error {
		r, err := pl.record(i)
		if err != nil {
			return fmt.Errorf("package %s: %w", short(g.Packages[i].Name), err)
		}
		recs[i] = r
		return nil
	}); err != nil {
		return nil, err
	}
	return &Plan{
		Schema: PlanSchema, OS: opt.OS, Arch: opt.Arch, Host: opt.Host.ImageID(),
		Packages: recs, Histogram: histogram(recs),
	}, nil
}

// record builds one package's key and classification.
func (pl *planner) record(p int) (Record, error) {
	pkg := pl.g.Packages[p]
	members := pl.closure(p)

	// Reads: the union over every member, de-duplicated.
	seen := map[readItem]bool{}
	var items []readItem
	for _, q := range members {
		for _, it := range pl.items[q] {
			if !seen[it] {
				seen[it] = true
				items = append(items, it)
			}
		}
	}
	var reads []key.FileEntry
	var consumed []key.Consumed
	for _, it := range items {
		r, err := pl.resolve(it)
		if err != nil {
			return Record{}, err
		}
		if r.file != nil {
			reads = append(reads, *r.file)
		} else {
			consumed = append(consumed, *r.consumed)
		}
	}
	reasons := map[string]bool{ReasonToolsEnv: true}
	hard := false
	// Escape, indirection and unscanned rules (KILN03 6).  Hard reasons, but
	// the key is still computed below: measurement never stops.
	scan, err := pl.scanReasons(members, items)
	if err != nil {
		return Record{}, err
	}
	for _, r := range scan {
		reasons[r] = true
		hard = true
	}
	unpinned := false
	for _, q := range members {
		m := pl.g.Packages[q]
		if q != p {
			d := pl.ownDigest[q]
			consumed = append(consumed, key.Consumed{Target: "pkg:" + m.Name, Phase: key.PhaseBuild, Key: d[:]})
		}
		if m.RepoInputsUnknown {
			reasons["unknown-repo-inputs:"+m.Name] = true
			hard = true
		}
		if !pl.pol.pinned[m.Language] {
			reasons["unpinned:"+m.Language] = true
			unpinned = true
		}
	}
	if pl.pol.never[pkg.Name] {
		reasons["never:"+pkg.Name] = true
		hard = true
	}
	td, err := pl.buildToolDigest()
	if err != nil {
		return Record{}, err
	}
	consumed = append(consumed, key.Consumed{Target: "tool:build-tool", Phase: key.PhaseBuild, Key: td[:]})

	sort.Slice(reads, func(a, b int) bool { return reads[a].Path < reads[b].Path })
	sort.Slice(consumed, func(a, b int) bool { return consumed[a].Target < consumed[b].Target })

	inputs, err := pl.entriesOf(pl.own[p])
	if err != nil {
		return Record{}, err
	}
	rec := Record{Name: pkg.Name, ClosureSize: len(members)}
	own := pl.ownDigest[p]
	rec.Components.Own = hexOf(own[:])
	rd, err := key.FilesDigest("reads", reads)
	if err != nil {
		return Record{}, err
	}
	rec.Components.Reads = hexOf(rd[:])
	cd, err := key.ConsumedDigest(consumed)
	if err != nil {
		return Record{}, err
	}
	rec.Components.Consumed = hexOf(cd[:])

	if bf := pkg.BuildFiles[pl.opt.OS]; bf == nil {
		reasons["no-build-file:"+pl.opt.OS] = true
		hard = true
	} else if info, err := pl.opt.Source.Describe(pkg.RelPath + "/" + *bf); err != nil {
		return Record{}, err
	} else if info.Link {
		// A symlinked BUILD file could point anywhere.  Fail closed for THIS
		// package (no key) and keep planning the others.
		reasons["escape:"+*bf] = true
		hard = true
	} else {
		contents, err := readSmall(pl.opt.Source, pkg.RelPath+"/"+*bf, maxBuildBytes)
		if err != nil {
			return Record{}, err
		}
		recipe, err := key.LegacyRecipeFiles([]key.NamedFile{{Name: *bf, Contents: contents}})
		if err != nil {
			return Record{}, err
		}
		rs := sha256.Sum256(recipe)
		rec.Components.Recipe = hexOf(rs[:])
		image := pl.opt.Host.ImageID()
		k, err := key.Key(key.Action{
			SemanticsVersion: 0,
			OS:               keyOS(pl.opt.OS), Arch: pl.opt.Arch,
			HostImageID: &image,
			Target:      pkg.Name, Phase: key.PhaseTest,
			Recipe: recipe, Inputs: inputs, Reads: reads, Consumed: consumed,
		})
		if err != nil {
			return Record{}, err
		}
		rec.Key = key.Hex(k)
	}
	rec.Reasons = make([]string, 0, len(reasons))
	for r := range reasons {
		rec.Reasons = append(rec.Reasons, r)
	}
	sort.Strings(rec.Reasons)
	rec.Class = classify(hard, unpinned)
	return rec, nil
}

// classify implements KILN03 section 7 for 7c-1.  A hard reason makes a
// package uncacheable; failing that, an unpinned language makes it `unpinned`;
// failing that, the standing m1-incomplete reasons keep it uncacheable.
// `cacheable` is unreachable until those reasons are retired (7c-3).
func classify(hard, unpinned bool) string {
	switch {
	case hard:
		return ClassUncacheable
	case unpinned:
		return ClassUnpinned
	}
	return ClassUncacheable
}

// reasonGroup collapses a reason to the kind the histogram counts: reasons
// that embed a package or file name are grouped by their prefix.
func reasonGroup(r string) string {
	for _, p := range []string{"unknown-repo-inputs:", "never:", "escape:", "indirect:", "unscanned:", "unknown-toolchain:"} {
		if strings.HasPrefix(r, p) {
			return p[:len(p)-1]
		}
	}
	return r
}

func histogram(recs []Record) Histogram {
	h := Histogram{Classes: map[string]int{}, Reasons: map[string]int{}}
	sizes := make([]int, 0, len(recs))
	for _, r := range recs {
		h.Classes[r.Class]++
		groups := map[string]bool{}
		for _, why := range r.Reasons {
			groups[reasonGroup(why)] = true
		}
		for g := range groups {
			h.Reasons[g]++
		}
		sizes = append(sizes, r.ClosureSize)
	}
	if n := len(sizes); n > 0 {
		sort.Ints(sizes)
		h.ClosureSizes = Sizes{Min: sizes[0], Median: sizes[n/2], P90: sizes[(9*n+9)/10-1], Max: sizes[n-1]}
	}
	return h
}

// Marshal renders the plan deterministically (map keys sort, fixed field
// order, no HTML escaping, trailing newline).
func (p *Plan) Marshal() ([]byte, error) {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", "  ")
	if err := enc.Encode(p); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

// ParsePlan reads a plan back strictly.
func ParsePlan(data []byte) (*Plan, error) {
	var p Plan
	if err := strictDecode(data, &p); err != nil {
		return nil, fmt.Errorf("plan: %w", err)
	}
	if p.Schema != PlanSchema {
		return nil, fmt.Errorf("plan: schema %q, want %q", short(p.Schema), PlanSchema)
	}
	return &p, nil
}

// ReadPlanFile loads a plan file (regular file, size-capped).
func ReadPlanFile(path string) (*Plan, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	fi, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if !fi.Mode().IsRegular() {
		return nil, fmt.Errorf("plan %s: not a regular file", path)
	}
	data, err := io.ReadAll(io.LimitReader(f, MaxPlanBytes+1))
	if err != nil {
		return nil, err
	}
	if len(data) > MaxPlanBytes {
		return nil, fmt.Errorf("plan %s exceeds the %d byte limit", path, MaxPlanBytes)
	}
	return ParsePlan(data)
}

// Find returns the record named name.
func (p *Plan) Find(name string) (Record, bool) {
	i := sort.Search(len(p.Packages), func(i int) bool { return p.Packages[i].Name >= name })
	if i < len(p.Packages) && p.Packages[i].Name == name {
		return p.Packages[i], true
	}
	return Record{}, false
}

// WriteFileAtomic writes data to path through a temporary file in the same
// directory (write, fsync, close, rename), so a reader sees the old file or
// the whole new one.  An existing symlink at path is refused rather than
// followed or replaced, and so is an existing non-regular file.  The
// containing directory is not fsynced: a plan is derived data that can be
// regenerated, unlike a ledger segment.
func WriteFileAtomic(path string, data []byte) error {
	if fi, err := os.Lstat(path); err == nil {
		if !fi.Mode().IsRegular() {
			return fmt.Errorf("%s: refusing to replace a symlink or non-regular file", path)
		}
	} else if !os.IsNotExist(err) {
		return err
	}
	tmp, err := os.CreateTemp(filepath.Dir(path), ".kiln-plan-*")
	if err != nil {
		return err
	}
	_ = tmp.Chmod(0o644) // best effort; CreateTemp makes it 0600
	name := tmp.Name()
	fail := func(err error) error {
		_ = tmp.Close()
		_ = os.Remove(name)
		return err
	}
	if _, err := tmp.Write(data); err != nil {
		return fail(err)
	}
	if err := tmp.Sync(); err != nil {
		return fail(err)
	}
	if err := tmp.Close(); err != nil {
		_ = os.Remove(name)
		return err
	}
	if err := os.Rename(name, path); err != nil {
		_ = os.Remove(name)
		return err
	}
	return nil
}
