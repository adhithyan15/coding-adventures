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

const bezier2DFlatteningTestPath = "code/specs/fixtures/bezier2d-flattening-v1/cases.json"

func bezier2DFlatteningPackages(root string) []discovery.Package {
	return []discovery.Package{
		{Name: "csharp/bezier2d", Path: filepath.Join(root, "code/packages/csharp/bezier2d"), Language: "csharp"},
		{Name: "fsharp/bezier2d", Path: filepath.Join(root, "code/packages/fsharp/bezier2d"), Language: "fsharp"},
		{Name: "rust/extra", Path: filepath.Join(root, "code/packages/rust/extra"), Language: "rust"},
	}
}

func TestBezier2DFlatteningFixtureExactSelection(t *testing.T) {
	root := t.TempDir()
	packages := bezier2DFlatteningPackages(root)
	for _, platform := range []string{"linux", "darwin", "windows"} {
		for _, lang := range []string{"all", "csharp", "fsharp", "rust"} {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{bezier2DFlatteningTestPath}, packages, root, platform, lang)
			want := map[string]bool{}
			if lang == "all" || lang == "csharp" {
				want["csharp/bezier2d"] = true
			}
			if lang == "all" || lang == "fsharp" {
				want["fsharp/bezier2d"] = true
			}
			if err != nil || !reflect.DeepEqual(got, want) {
				t.Fatalf("%s/%s got %v/%v, want %v", platform, lang, got, err, want)
			}
		}
	}

	for _, path := range []string{
		strings.ToUpper(bezier2DFlatteningTestPath),
		strings.ReplaceAll(bezier2DFlatteningTestPath, "/", "\\"),
		bezier2DFlatteningTestPath + ".bak",
		strings.Replace(bezier2DFlatteningTestPath, "cases.json", "schema.json", 1),
		strings.Replace(bezier2DFlatteningTestPath, "cases.json", "nested/cases.json", 1),
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %s: %v/%v", path, got, err)
		}
	}

	got, err := changedPackageRootsForPlatform([]string{bezier2DFlatteningTestPath, "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux")
	if err != nil || len(got) != 3 || !got["csharp/bezier2d"] || !got["fsharp/bezier2d"] || !got["rust/extra"] {
		t.Fatalf("union %v/%v", got, err)
	}

	for _, files := range [][]string{nil, {}} {
		got, err := changedPackageRootsForPlatform(files, packages, root, "linux")
		if err != nil || len(got) != 0 || (files == nil) != (got == nil) {
			t.Fatalf("nil/empty %v/%v", got, err)
		}
	}

	for _, missing := range []string{"csharp/bezier2d", "fsharp/bezier2d"} {
		remaining := make([]discovery.Package, 0, len(packages)-1)
		for _, pkg := range packages {
			if pkg.Name != missing {
				remaining = append(remaining, pkg)
			}
		}
		got, err := changedPackageRootsForPlatform([]string{bezier2DFlatteningTestPath}, remaining, root, "linux")
		if err == nil || got != nil {
			t.Fatalf("missing %s: %v/%v", missing, got, err)
		}
	}

	// Selection is path-based so deletion and rename-old paths still work.
	if _, err := os.Stat(filepath.Join(root, filepath.FromSlash(bezier2DFlatteningTestPath))); !os.IsNotExist(err) {
		t.Fatal("fixture should be absent")
	}
}

func TestBezier2DFlatteningFixturePlanAndAtomicity(t *testing.T) {
	root := t.TempDir()
	packages := bezier2DFlatteningPackages(root)
	for _, pkg := range packages {
		if err := os.MkdirAll(pkg.Path, 0o755); err != nil {
			t.Fatal(err)
		}
		for _, front := range []string{"BUILD", "BUILD_windows"} {
			if err := os.WriteFile(filepath.Join(pkg.Path, front), []byte("dotnet test\n"), 0o644); err != nil {
				t.Fatal(err)
			}
		}
	}

	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	changed, err := changedPackageRootsForPlatform([]string{bezier2DFlatteningTestPath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	out := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(packages, graph, affected, changed, []string{bezier2DFlatteningTestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code != 0 {
		t.Fatal(code)
	}
	data, err := os.ReadFile(out)
	if err != nil {
		t.Fatal(err)
	}
	var built plan.BuildPlan
	if err := json.Unmarshal(data, &built); err != nil {
		t.Fatal(err)
	}
	if built.Force || !built.LanguagesNeeded["dotnet"] || built.LanguagesNeeded["rust"] {
		t.Fatalf("plan toolchains/force %+v", built.LanguagesNeeded)
	}
	for _, platform := range []string{"linux", "darwin", "windows"} {
		got := append([]string(nil), built.StateForPlatform(platform).AffectedPackages...)
		slices.Sort(got)
		if !reflect.DeepEqual(got, []string{"csharp/bezier2d", "fsharp/bezier2d"}) {
			t.Fatal(platform, got)
		}
	}

	missing := packages[1:]
	missingGraph, err := resolver.ResolveDependencies(missing)
	if err != nil {
		t.Fatal(err)
	}
	if code := emitBuildPlan(missing, missingGraph, nil, nil, []string{bezier2DFlatteningTestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code == 0 {
		t.Fatal("missing reader wrote plan")
	}
	if after, err := os.ReadFile(out); err != nil || string(after) != string(data) {
		t.Fatal("failed plan replaced output", err)
	}
	if err := os.Remove(out); err != nil {
		t.Fatal(err)
	}
	if code := emitBuildPlan(missing, missingGraph, nil, nil, []string{bezier2DFlatteningTestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code == 0 {
		t.Fatal("missing reader created plan")
	}
	if _, err := os.Stat(out); !os.IsNotExist(err) {
		t.Fatal("failed plan created output")
	}
}
