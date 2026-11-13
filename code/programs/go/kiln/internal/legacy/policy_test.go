package legacy

import (
	"strings"
	"testing"
)

func TestUnknownRepoInputsMakesClosureUncacheable(t *testing.T) {
	f := newFixture(t)
	g := baseGraph()
	g.Packages[2].RepoInputsUnknown = true // x/c, a deep dep of x/a
	p := f.plan(g)
	for _, n := range []string{"x/c", "x/b", "x/a"} {
		r := p.rec(t, n)
		if r.Class != ClassUncacheable || !contains(r.Reasons, "unknown-repo-inputs:x/c") {
			t.Fatalf("%s: %+v", n, r)
		}
		if r.Key == "" {
			t.Fatalf("%s: key must still be computed", n)
		}
	}
	if r := p.rec(t, "x/z"); contains(r.Reasons, "unknown-repo-inputs:x/c") {
		t.Fatal("unrelated package tainted")
	}
	if p.Histogram.Reasons["unknown-repo-inputs"] < 3 {
		t.Fatalf("histogram: %v", p.Histogram.Reasons)
	}
}

func TestUnpinnedClassKeepsKey(t *testing.T) {
	f := newFixture(t)
	p := f.plan(baseGraph())
	r := p.rec(t, "x/a")
	if r.Class != ClassUnpinned || r.Key == "" {
		t.Fatalf("%+v", r)
	}
	for _, want := range []string{"unpinned:x", ReasonToolsEnv} {
		if !contains(r.Reasons, want) {
			t.Fatalf("missing reason %s in %v", want, r.Reasons)
		}
	}
	for i := 1; i < len(r.Reasons); i++ {
		if r.Reasons[i-1] >= r.Reasons[i] {
			t.Fatalf("reasons not sorted: %v", r.Reasons)
		}
	}
	// A closure language that is not pinned is enough, even if P's is.
	g := baseGraph()
	g.Packages[2].Language = "other" // x/c
	f.write("code/kiln/legacy/pinned.json", `{"languages": ["x"]}`)
	p = f.plan(g)
	if r := p.rec(t, "x/a"); r.Class != ClassUnpinned || !contains(r.Reasons, "unpinned:other") || contains(r.Reasons, "unpinned:x") {
		t.Fatalf("%+v", r)
	}
	// Everything pinned: only the standing m1-incomplete reasons remain, so
	// nothing is cacheable yet.
	g.Packages[2].Language = "x"
	r = f.plan(g).rec(t, "x/a")
	if r.Class != ClassUncacheable || len(r.Reasons) != 2 {
		t.Fatalf("%+v", r)
	}
}

func TestNeverList(t *testing.T) {
	f := newFixture(t)
	f.write("code/kiln/legacy/never.json", `{"packages": ["x/z"]}`)
	r := f.plan(baseGraph()).rec(t, "x/z")
	if r.Class != ClassUncacheable || !contains(r.Reasons, "never:x/z") {
		t.Fatalf("%+v", r)
	}
	// Never is per package, not per closure.
	if r := f.plan(baseGraph()).rec(t, "x/a"); hasPrefixReason(r.Reasons, "never:") {
		t.Fatal("never leaked to another package")
	}
}

func TestPolicyFileErrors(t *testing.T) {
	bad := map[string]map[string]string{
		"missing pinned":    {"code/kiln/legacy/pinned.json": ""},
		"pinned unknown":    {"code/kiln/legacy/pinned.json": `{"languages": [], "extra": 1}`},
		"pinned unsorted":   {"code/kiln/legacy/pinned.json": `{"languages": ["b","a"]}`},
		"pinned trailing":   {"code/kiln/legacy/pinned.json": `{"languages": []} {}`},
		"pinned not json":   {"code/kiln/legacy/pinned.json": `nope`},
		"never unknown pkg": {"code/kiln/legacy/never.json": `{"packages": ["nope"]}`},
		"never unknown key": {"code/kiln/legacy/never.json": `{"pkgs": []}`},
		"never missing":     {"code/kiln/legacy/never.json": ""},
		"pinned too large":  {"code/kiln/legacy/pinned.json": `{"languages": ["` + strings.Repeat("a", maxPolicyBytes) + `"]}`},
	}
	for name, files := range bad {
		t.Run(name, func(t *testing.T) {
			f := newFixture(t)
			for p, c := range files {
				if c == "" {
					f.remove(p)
				} else {
					f.write(p, c)
				}
			}
			if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
				t.Fatal("accepted")
			}
		})
	}
}

func TestBoundaryFileErrors(t *testing.T) {
	bad := map[string]string{
		"":                       "",
		"not json":               "nope",
		"wrong schema":           `{"schema_version": 2, "boundaries": []}`,
		"unknown applies_to key": `{"schema_version":1,"boundaries":[{"id":"x","applies_to":{"exact_roots":[],"glob_roots":["a"]},"inputs":[]}]}`,
		"bad root":               `{"schema_version":1,"boundaries":[{"id":"x","applies_to":{"exact_roots":["../a"]},"inputs":[]}]}`,
		"bad input":              `{"schema_version":1,"boundaries":[{"id":"x","applies_to":{},"inputs":[{"path":"/abs"}]}]}`,
		"missing applies_to":     `{"schema_version":1,"boundaries":[{"id":"x","inputs":[]}]}`,
	}
	for name, content := range bad {
		t.Run(name, func(t *testing.T) {
			f := newFixture(t)
			if content == "" {
				f.remove(BoundaryPath)
			} else {
				f.write(BoundaryPath, content)
			}
			if _, err := BuildPlan(baseGraph(), f.opts("linux")); err == nil {
				t.Fatal("accepted")
			}
		})
	}
}

func TestBoundaryAppliesTo(t *testing.T) {
	b := boundary{exact: []string{"a/exact"}, descendant: []string{"a/tree"}, excluded: []string{"a/tree/skip"}}
	for rel, want := range map[string]bool{
		"a/exact": true, "a/exact/child": false, "a/tree": true, "a/tree/x/y": true,
		"a/tree/skip": true, "a/tree/skip/deep": true, "a/tree/skipper": true, "a/treehouse": false, "b": false,
	} {
		if got := b.appliesTo(rel); got != want {
			t.Errorf("appliesTo(%q) = %v, want %v", rel, got, want)
		}
	}
}

// A package under a descendant root picks up the boundary input, and so does
// one under the excluded root: exclusions are ignored for reads, because
// honouring them could only drop an input.
func TestBoundaryDescendantAndExcluded(t *testing.T) {
	f := newFixture(t)
	f.write("code/packages/y/in/BUILD", "x")
	f.write("code/packages/y/skip/BUILD", "x")
	g := baseGraph()
	g.Packages = append(g.Packages, mkpkg("y/in", "code/packages/y/in", "y"), mkpkg("y/skip", "code/packages/y/skip", "y"))
	base := f.plan(g)
	f.write("code/shared/y.json", "changed")
	p := f.plan(g)
	for _, n := range []string{"y/in", "y/skip"} {
		if p.rec(t, n).Key == base.rec(t, n).Key {
			t.Fatalf("%s: boundary input ignored (excluded roots must not drop reads)", n)
		}
	}
}

func TestMissingReadPathIsPartOfKey(t *testing.T) {
	f := newFixture(t)
	g := baseGraph()
	g.Packages[2].RepoInputs = []string{"code/not/there.txt"} // x/c
	a := f.plan(g).rec(t, "x/a").Key
	f.write("code/not/there.txt", "now it exists")
	if f.plan(g).rec(t, "x/a").Key == a {
		t.Fatal("a read path appearing must change the key")
	}
}

func TestRepoInputsFileIsAReadsEntry(t *testing.T) {
	f := newFixture(t)
	g := baseGraph()
	g.Packages[2].RepoInputs = []string{"code/shared/cfg.json", "code/grammars/g"}
	a := f.plan(g).rec(t, "x/a")
	f.write("code/shared/cfg.json", "changed2")
	b := f.plan(g).rec(t, "x/a")
	if a.Key == b.Key || a.Components.Reads == b.Components.Reads {
		t.Fatal("file-valued repo_inputs must be a reads entry")
	}
	// A directory contributes through consumed, not reads.
	f.write("code/grammars/g/one.txt", "changed2")
	c := f.plan(g).rec(t, "x/a")
	if c.Components.Reads != b.Components.Reads || c.Components.Consumed == b.Components.Consumed {
		t.Fatal("directory-valued repo_inputs must be a consumed tree digest")
	}
}

func TestTreeDigestCoversContentNotJustPaths(t *testing.T) {
	f := newFixture(t)
	a := f.plan(baseGraph()).rec(t, "x/c").Components.Consumed
	f.write("code/grammars/g/one.txt", "ONE") // same path, same size, new content
	b := f.plan(baseGraph()).rec(t, "x/c").Components.Consumed
	if a == b {
		t.Fatal("tree digest ignored content")
	}
}
