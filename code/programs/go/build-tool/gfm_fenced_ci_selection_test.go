package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"slices"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

const gfmFencedTestPath = "code/specs/gfm-fenced-block-v1/cases.json"

func gfmFencedPackages(root string) []discovery.Package {
	return []discovery.Package{
		{Name: "typescript/gfm-parser", Path: filepath.Join(root, "code/packages/typescript/gfm-parser"), Language: "typescript"},
		{Name: "rust/extra", Path: filepath.Join(root, "code/packages/rust/extra"), Language: "rust"},
	}
}

func TestGFMFencedFixtureExactSelection(t *testing.T) {
	root := t.TempDir()
	packages := gfmFencedPackages(root)
	for _, platform := range []string{"linux", "darwin", "windows"} {
		for _, lang := range []string{"all", "typescript", "rust"} {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{gfmFencedTestPath}, packages, root, platform, lang)
			want := map[string]bool{}
			if lang != "rust" {
				want["typescript/gfm-parser"] = true
			}
			if err != nil || !reflect.DeepEqual(got, want) {
				t.Fatalf("%s/%s got %v/%v", platform, lang, got, err)
			}
		}
	}
	for _, path := range []string{strings.ToUpper(gfmFencedTestPath), strings.ReplaceAll(gfmFencedTestPath, "/", "\\"), gfmFencedTestPath + ".bak", strings.Replace(gfmFencedTestPath, "cases.json", "schema.json", 1), strings.Replace(gfmFencedTestPath, "cases.json", "README.md", 1), strings.Replace(gfmFencedTestPath, "cases.json", "nested/cases.json", 1)} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %s: %v/%v", path, got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{gfmFencedTestPath, "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux")
	if err != nil || len(got) != 2 || !got["typescript/gfm-parser"] || !got["rust/extra"] {
		t.Fatalf("union %v/%v", got, err)
	}
	for _, files := range [][]string{nil, {}} {
		got, err := changedPackageRootsForPlatform(files, packages, root, "linux")
		if err != nil || len(got) != 0 || (files == nil) != (got == nil) {
			t.Fatalf("nil/empty %v/%v", got, err)
		}
	}
	for _, lang := range []string{"all", "typescript", "rust"} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{gfmFencedTestPath}, packages[1:], root, "linux", lang)
		if lang == "rust" {
			if err != nil || len(got) != 0 {
				t.Fatal(got, err)
			}
		} else if err == nil || got != nil {
			t.Fatalf("missing reader %v/%v", got, err)
		}
	}
	wrongLane := gfmFencedPackages(root)
	wrongLane[0].Language = "go"
	got, err = changedPackageRootsForPlatformAndLanguage([]string{gfmFencedTestPath}, wrongLane, root, "linux", "typescript")
	if err == nil || got != nil {
		t.Fatalf("mismatched reader lane %v/%v", got, err)
	}
	// No fixture stat is required: a deleted or renamed old path still selects.
	if _, err := os.Stat(filepath.Join(root, filepath.FromSlash(gfmFencedTestPath))); !os.IsNotExist(err) {
		t.Fatal("fixture should be absent")
	}
}

func TestGFMFencedFixturePlanAndAtomicity(t *testing.T) {
	root := t.TempDir()
	packages := append(gfmFencedPackages(root), discovery.Package{Name: "typescript/gfm", Path: filepath.Join(root, "code/packages/typescript/gfm"), Language: "typescript"})
	for _, pkg := range packages {
		if err := os.MkdirAll(pkg.Path, 0755); err != nil {
			t.Fatal(err)
		}
		for _, front := range []string{"BUILD", "BUILD_windows"} {
			if err := os.WriteFile(filepath.Join(pkg.Path, front), []byte("npm test -- --run\n"), 0644); err != nil {
				t.Fatal(err)
			}
		}
	}
	// Real file dependencies must propagate the fixture seed to its facade.
	for path, data := range map[string]string{
		filepath.Join(packages[0].Path, "package.json"): `{"name":"@coding-adventures/gfm-parser","dependencies":{}}`,
		filepath.Join(packages[2].Path, "package.json"): `{"name":"@coding-adventures/gfm","dependencies":{"@coding-adventures/gfm-parser":"file:../gfm-parser"}}`,
	} {
		if err := os.WriteFile(path, []byte(data), 0644); err != nil {
			t.Fatal(err)
		}
	}
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	changed, err := changedPackageRootsForPlatform([]string{gfmFencedTestPath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	out := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(packages, graph, affected, changed, []string{gfmFencedTestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code != 0 {
		t.Fatal(code)
	}
	data, err := os.ReadFile(out)
	if err != nil {
		t.Fatal(err)
	}
	var p plan.BuildPlan
	if err = json.Unmarshal(data, &p); err != nil {
		t.Fatal(err)
	}
	if p.Force || !p.LanguagesNeeded["typescript"] || p.LanguagesNeeded["rust"] {
		t.Fatalf("plan toolchains/force %+v", p.LanguagesNeeded)
	}
	for _, platform := range []string{"linux", "darwin", "windows"} {
		// AffectedPackages is set membership, not execution order. The graph
		// retains dependency ordering; map iteration may vary this projection.
		got := append([]string(nil), p.StateForPlatform(platform).AffectedPackages...)
		slices.Sort(got)
		if !reflect.DeepEqual(got, []string{"typescript/gfm", "typescript/gfm-parser"}) {
			t.Fatal(platform, got)
		}
	}
	missing := packages[1:2]
	g, err := resolver.ResolveDependencies(missing)
	if err != nil {
		t.Fatal(err)
	}
	if code := emitBuildPlan(missing, g, nil, nil, []string{gfmFencedTestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code == 0 {
		t.Fatal("missing reader wrote plan")
	}
	if after, err := os.ReadFile(out); err != nil || string(after) != string(data) {
		t.Fatal("failed plan replaced output", err)
	}
	if err := os.Remove(out); err != nil {
		t.Fatal(err)
	}
	if code := emitBuildPlan(missing, g, nil, nil, []string{gfmFencedTestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code == 0 {
		t.Fatal("missing reader created plan")
	}
	if _, err := os.Stat(out); !os.IsNotExist(err) {
		t.Fatal("failed plan created output")
	}
}
