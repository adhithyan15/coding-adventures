package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

const bf03TestPath = "code/specs/fixtures/compiler-ir-v1/cases.json"

func bf03Packages(root string) []discovery.Package {
	return []discovery.Package{
		{Name: "go/compiler-ir", Path: filepath.Join(root, "code/packages/go/compiler-ir"), Language: "go"},
		{Name: "rust/extra", Path: filepath.Join(root, "code/packages/rust/extra"), Language: "rust"},
	}
}

func TestBF03FixtureExactSelection(t *testing.T) {
	root := t.TempDir()
	packages := bf03Packages(root)
	for _, platform := range []string{"linux", "darwin", "windows"} {
		for _, lang := range []string{"all", "go", "rust"} {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{bf03TestPath}, packages, root, platform, lang)
			want := map[string]bool{}
			if lang != "rust" {
				want["go/compiler-ir"] = true
			}
			if err != nil || !reflect.DeepEqual(got, want) {
				t.Fatalf("%s/%s got %v/%v", platform, lang, got, err)
			}
		}
	}
	for _, path := range []string{strings.ReplaceAll(bf03TestPath, "/", "\\"), bf03TestPath + ".bak", strings.Replace(bf03TestPath, "cases.json", "schema.json", 1), strings.Replace(bf03TestPath, "cases.json", "README.md", 1), strings.Replace(bf03TestPath, "cases.json", "nested/cases.json", 1)} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %s: %v/%v", path, got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{bf03TestPath, "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux")
	if err != nil || len(got) != 2 || !got["go/compiler-ir"] || !got["rust/extra"] {
		t.Fatalf("union %v/%v", got, err)
	}
	for _, files := range [][]string{nil, {}} {
		got, err := changedPackageRootsForPlatform(files, packages, root, "linux")
		if err != nil || len(got) != 0 || (files == nil) != (got == nil) {
			t.Fatalf("nil/empty %v/%v", got, err)
		}
	}
	for _, lang := range []string{"all", "go", "rust"} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{bf03TestPath}, packages[1:], root, "linux", lang)
		if lang == "rust" {
			if err != nil || len(got) != 0 {
				t.Fatal(got, err)
			}
		} else if err == nil || got != nil {
			t.Fatalf("missing reader %v/%v", got, err)
		}
	}
	// No fixture stat is required: a deleted or renamed old path still selects.
	if _, err := os.Stat(filepath.Join(root, filepath.FromSlash(bf03TestPath))); !os.IsNotExist(err) {
		t.Fatal("fixture should be absent")
	}
}

func TestBF03FixturePlanAndAtomicity(t *testing.T) {
	root := t.TempDir()
	packages := bf03Packages(root)
	for _, pkg := range packages {
		if err := os.MkdirAll(pkg.Path, 0755); err != nil {
			t.Fatal(err)
		}
		for _, front := range []string{"BUILD", "BUILD_windows"} {
			if err := os.WriteFile(filepath.Join(pkg.Path, front), []byte("go test ./... -v -cover\n"), 0644); err != nil {
				t.Fatal(err)
			}
		}
	}
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	changed, err := changedPackageRootsForPlatform([]string{bf03TestPath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	out := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(packages, graph, affected, changed, []string{bf03TestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code != 0 {
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
	if p.Force || !p.LanguagesNeeded["go"] || p.LanguagesNeeded["rust"] {
		t.Fatalf("plan toolchains/force %+v", p.LanguagesNeeded)
	}
	for _, platform := range []string{"linux", "darwin", "windows"} {
		if !reflect.DeepEqual(p.StateForPlatform(platform).AffectedPackages, []string{"go/compiler-ir"}) {
			t.Fatal(platform, p.StateForPlatform(platform).AffectedPackages)
		}
	}
	missing := packages[1:]
	g, err := resolver.ResolveDependencies(missing)
	if err != nil {
		t.Fatal(err)
	}
	if code := emitBuildPlan(missing, g, nil, nil, []string{bf03TestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code == 0 {
		t.Fatal("missing reader wrote plan")
	}
	if after, err := os.ReadFile(out); err != nil || string(after) != string(data) {
		t.Fatal("failed plan replaced output", err)
	}
	if err := os.Remove(out); err != nil {
		t.Fatal(err)
	}
	if code := emitBuildPlan(missing, g, nil, nil, []string{bf03TestPath}, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code == 0 {
		t.Fatal("missing reader created plan")
	}
	if _, err := os.Stat(out); !os.IsNotExist(err) {
		t.Fatal("failed plan created output")
	}
}
