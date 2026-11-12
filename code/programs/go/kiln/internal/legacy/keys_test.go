package legacy

import (
	"bytes"
	"testing"
)

// TestKeyChanges is KILN03 section 10: the key of x/a must change when any
// file it could observe changes.
func TestKeyChanges(t *testing.T) {
	cases := []struct {
		name  string
		apply func(f *fixture)
	}{
		{"file in P", func(f *fixture) { f.write("code/packages/x/a/src.txt", "changed") }},
		{"new file in P", func(f *fixture) { f.write("code/packages/x/a/new.txt", "new") }},
		{"file in direct dep", func(f *fixture) { f.write("code/packages/x/outer/b/src.txt", "changed") }},
		{"file in deep dep", func(f *fixture) { f.write("code/packages/x/c/src.txt", "changed") }},
		{"file in dev-dep of P", func(f *fixture) { f.write("code/packages/x/t/src.txt", "changed") }},
		{"file in package nested in a dependency", func(f *fixture) { f.write("code/packages/x/c/ext/n/src.txt", "changed") }},
		{"manifest of package enclosing a dependency", func(f *fixture) { f.write("code/packages/x/outer/Cargo.toml", "changed") }},
		{"ancestor config, same dir level", func(f *fixture) { f.write("code/packages/x/.npmrc", "changed") }},
		{"ancestor config, repo root", func(f *fixture) { f.write("mise.toml", "changed") }},
		{"new ancestor config file", func(f *fixture) { f.write("code/tsconfig.base.json", "{}") }},
		{"boundary input", func(f *fixture) { f.write("code/shared/cfg.json", "changed") }},
		{"repo_inputs directory, top file", func(f *fixture) { f.write("code/grammars/g/one.txt", "changed") }},
		{"repo_inputs directory, deep file", func(f *fixture) { f.write("code/grammars/g/sub/two.txt", "changed") }},
		{"repo_inputs directory, new file", func(f *fixture) { f.write("code/grammars/g/sub/three.txt", "new") }},
		{"glob-matched file", func(f *fixture) { f.write("code/lang/x.mll", "changed") }},
		{"new glob-matched file", func(f *fixture) { f.write("code/lang/y.mll", "new") }},
		{"selected BUILD text", func(f *fixture) { f.write("code/packages/x/a/BUILD", "echo changed\n") }},
		{"build tool source", func(f *fixture) { f.write(BuildToolPath+"/main.go", "package main // changed\n") }},
		{"module the build tool replaces", func(f *fixture) { f.write("code/packages/go/lib/lib.go", "package lib // changed\n") }},
		{"go.work", func(f *fixture) { f.write("go.work", "go 1.27\n") }},
		{"vendor directory", func(f *fixture) { f.write("vendor/mod/x.go", "package x\n") }},
	}
	base := newFixture(t)
	want := base.plan(baseGraph()).rec(t, "x/a").Key
	if len(want) != 64 {
		t.Fatalf("base key %q", want)
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			f := newFixture(t)
			c.apply(f)
			if got := f.plan(baseGraph()).rec(t, "x/a").Key; got == want {
				t.Fatalf("key did not change")
			}
		})
	}
}

func TestKeyStaysTheSame(t *testing.T) {
	base := newFixture(t)
	bp := base.plan(baseGraph())
	want := bp.rec(t, "x/a")
	cases := []struct {
		name  string
		apply func(f *fixture)
	}{
		{"unrelated package", func(f *fixture) { f.write("code/packages/x/z/src.txt", "changed") }},
		{"dev-dep of a member other than P", func(f *fixture) { f.write("code/packages/x/u/src.txt", "changed") }},
		{"ignore-listed ancestor files", func(f *fixture) {
			f.write("README.md", "changed")
			f.write("code/packages/x/README.md", "changed")
			f.write("code/LICENSE", "x")
			f.write("code/CHANGELOG.txt", "x")
			f.write("code/.gitignore", "x")
		}},
		{"file in a sibling directory", func(f *fixture) { f.write("code/other/file.txt", "x") }},
		{"boundary that does not apply (excluded root)", func(f *fixture) { f.write("code/shared/y.json", "changed") }},
		{"a non-matching glob candidate", func(f *fixture) { f.write("code/lang/.other", "x") }},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			f := newFixture(t)
			c.apply(f)
			if got := f.plan(baseGraph()).rec(t, "x/a"); got.Key != want.Key {
				t.Fatalf("key changed")
			}
		})
	}
	// The bare ".mll" is kept out by "?*.mll": editing it changes nobody.
	f := newFixture(t)
	f.write("code/lang/.mll", "changed")
	if f.plan(baseGraph()).rec(t, "x/a").Key != want.Key {
		t.Fatal("empty-stem glob match leaked into the key")
	}
}

// Another OS's BUILD text changes P's inputs (it is a file in P) but not the
// recipe component, which holds only the file this OS runs.
func TestRecipeComponentIsOnlyTheSelectedFile(t *testing.T) {
	a := newFixture(t).plan(baseGraph()).rec(t, "x/a")
	f := newFixture(t)
	f.write("code/packages/x/a/BUILD_windows", "echo changed\n")
	b := f.plan(baseGraph()).rec(t, "x/a")
	if a.Components.Recipe != b.Components.Recipe || a.Components.Recipe == "" {
		t.Fatal("recipe component changed with an unselected BUILD file")
	}
	if a.Key == b.Key || a.Components.Own == b.Components.Own {
		t.Fatal("an unselected BUILD file is still one of P's inputs")
	}
	// And the selected text moves the recipe component.
	f.write("code/packages/x/a/BUILD", "echo changed\n")
	if c := f.plan(baseGraph()).rec(t, "x/a"); c.Components.Recipe == a.Components.Recipe {
		t.Fatal("selected BUILD did not change the recipe component")
	}
}

func TestNestedAndEnclosingOfP(t *testing.T) {
	base := newFixture(t).plan(baseGraph())
	// x/outer/b is nested in x/outer: its key must follow the enclosing manifest.
	f := newFixture(t)
	f.write("code/packages/x/outer/Cargo.toml", "changed")
	p := f.plan(baseGraph())
	if p.rec(t, "x/b").Key == base.rec(t, "x/b").Key {
		t.Fatal("nested package ignores its enclosing package's manifest")
	}
	// x/outer contains x/b, which contains no deps but x/c: nested edge pulls both.
	f = newFixture(t)
	f.write("code/packages/x/c/src.txt", "changed")
	p = f.plan(baseGraph())
	if p.rec(t, "x/outer").Key == base.rec(t, "x/outer").Key {
		t.Fatal("enclosing package ignores a change in its nested package's dependency")
	}
	if p.rec(t, "x/z").Key != base.rec(t, "x/z").Key {
		t.Fatal("unrelated key moved")
	}
}

func TestClosureCyclesAndOrder(t *testing.T) {
	mk := func(order []string) *Graph {
		// a <-> b dependency cycle; a and b are dev-deps of each other too.
		pk := map[string]Package{
			"c1": mkpkg("c1", "code/packages/x/a", "x", "c2"),
			"c2": mkpkg("c2", "code/packages/x/outer/b", "x", "c1", "c3"),
			"c3": mkpkg("c3", "code/packages/x/c", "x", "c3"), // self-edge
		}
		c1, c2 := pk["c1"], pk["c2"]
		c1.DevDeps = []string{"c2"}
		c2.DevDeps = []string{"c1"}
		pk["c1"], pk["c2"] = c1, c2
		g := &Graph{Schema: GraphSchema}
		for _, n := range order {
			g.Packages = append(g.Packages, pk[n])
		}
		return g
	}
	f := newFixture(t)
	p := f.plan(mk([]string{"c1", "c2", "c3"}))
	for _, n := range []string{"c1", "c2"} {
		if got := p.rec(t, n).ClosureSize; got != 3 {
			t.Fatalf("%s closure size %d, want 3", n, got)
		}
	}
	// Same cycle, listed with the deps reversed: identical closure sizes.
	g2 := mk([]string{"c1", "c2", "c3"})
	g2.Packages[1].Deps["linux"] = []string{"c3", "c1"}
	if got := f.plan(g2).rec(t, "c1").ClosureSize; got != 3 {
		t.Fatalf("reversed deps: closure size %d", got)
	}
	// dev_deps are seeded only at P: a closure member's dev_deps are not followed.
	g3 := &Graph{Schema: GraphSchema, Packages: []Package{
		mkpkg("d1", "code/packages/x/a", "x", "d2"),
		mkpkg("d2", "code/packages/x/c", "x"),
		mkpkg("d3", "code/packages/x/t", "x"),
		mkpkg("d4", "code/packages/x/u", "x"),
	}}
	g3.Packages[1].DevDeps = []string{"d3"} // d2's dev dep: not in d1's closure
	g3.Packages[0].DevDeps = []string{"d4"}
	p3 := f.plan(g3)
	if got := p3.rec(t, "d1").ClosureSize; got != 3 { // d1, d2 (dep), d4 (own dev dep)
		t.Fatalf("d1 closure %d, want 3", got)
	}
	if got := p3.rec(t, "d2").ClosureSize; got != 2 { // d2 + its own dev dep d3
		t.Fatalf("d2 closure %d, want 2", got)
	}
}

func TestDeterminism(t *testing.T) {
	f := newFixture(t)
	var outs [][]byte
	for _, w := range []int{1, 8, 1} {
		o := f.opts("linux")
		o.Workers = w
		p, err := BuildPlan(baseGraph(), o)
		if err != nil {
			t.Fatal(err)
		}
		b, err := p.Marshal()
		if err != nil {
			t.Fatal(err)
		}
		outs = append(outs, b)
	}
	if !bytes.Equal(outs[0], outs[1]) || !bytes.Equal(outs[0], outs[2]) {
		t.Fatal("plans differ between runs")
	}
}

func TestPlatformsAndHostChangeKeys(t *testing.T) {
	f := newFixture(t)
	g := baseGraph()
	lin, _ := BuildPlan(g, f.opts("linux"))
	mac, _ := BuildPlan(g, f.opts("darwin"))
	if lin.rec(t, "x/a").Key == mac.rec(t, "x/a").Key {
		t.Fatal("platform not in key")
	}
	o := f.opts("linux")
	o.Host = imager("other/image")
	other, _ := BuildPlan(g, o)
	if other.rec(t, "x/a").Key == lin.rec(t, "x/a").Key {
		t.Fatal("host image not in key")
	}
	o = f.opts("linux")
	o.Arch = "arm64"
	arm, _ := BuildPlan(g, o)
	if arm.rec(t, "x/a").Key == lin.rec(t, "x/a").Key {
		t.Fatal("arch not in key")
	}
	if lin.Host != "test/image" || lin.OS != "linux" {
		t.Fatalf("plan header: %+v", lin)
	}
}

func TestPerOSDeps(t *testing.T) {
	f := newFixture(t)
	g := baseGraph()
	g.Packages[0].Deps["windows"] = nil // x/a has no deps on windows
	lin := f.plan(g).rec(t, "x/a")
	win, err := BuildPlan(g, f.opts("windows"))
	if err != nil {
		t.Fatal(err)
	}
	if lin.ClosureSize <= win.rec(t, "x/a").ClosureSize {
		t.Fatalf("per-OS deps ignored: %d vs %d", lin.ClosureSize, win.rec(t, "x/a").ClosureSize)
	}
}

func TestNoBuildFileOnThisOS(t *testing.T) {
	f := newFixture(t)
	g := baseGraph()
	g.Packages[7].BuildFiles["windows"] = nil // x/z
	p, err := BuildPlan(g, f.opts("windows"))
	if err != nil {
		t.Fatal(err)
	}
	r := p.rec(t, "x/z")
	if r.Key != "" || r.Class != ClassUncacheable || !contains(r.Reasons, "no-build-file:windows") || r.Components.Recipe != "" {
		t.Fatalf("%+v", r)
	}
}

func TestSelectedBuildFileMissing(t *testing.T) {
	f := newFixture(t)
	g := baseGraph()
	bf := "BUILD_missing"
	g.Packages[7].BuildFiles["linux"] = &bf
	if _, err := BuildPlan(g, f.opts("linux")); err == nil {
		t.Fatal("missing selected BUILD file accepted")
	}
}

func TestBuildFileVariantIsPartOfRecipe(t *testing.T) {
	f := newFixture(t)
	f.write("code/packages/x/z/BUILD_linux", "echo z\n")
	g := baseGraph()
	a := f.plan(g).rec(t, "x/z")
	bf := "BUILD_linux"
	g.Packages[7].BuildFiles["linux"] = &bf
	b := f.plan(g).rec(t, "x/z")
	if a.Components.Recipe == b.Components.Recipe {
		t.Fatal("same text under two names must give two recipes")
	}
}
