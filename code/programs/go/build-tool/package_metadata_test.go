package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

// metadataWorld writes a miniature Cargo repository:
//
//	rust/pre ──▶ rust/meta ──▶ rust/dep          (build edges)
//	                 ⋮
//	                 └ ┄ ┄ ┄▶ rust/smoke             (dev-dependency trigger)
//	rust/other                                       (unrelated)
//
// rust/meta plays rust/bounded-json: a library with dependents, whose
// capability manifest or changelog changes on its own.
func metadataWorld(t *testing.T) (string, []discovery.Package) {
	t.Helper()
	root := t.TempDir()
	files := map[string]string{
		"code/packages/rust/pre/Cargo.toml":  "[package]\nname = \"pre\"\n",
		"code/packages/rust/meta/Cargo.toml": "[package]\nname = \"meta\"\n\n[dependencies]\npre = { path = \"../pre\" }\n",
		"code/packages/rust/dep/Cargo.toml":  "[package]\nname = \"dep\"\n\n[dependencies]\nmeta = { path = \"../meta\" }\n",
		"code/packages/rust/smoke/Cargo.toml": "[package]\nname = \"smoke\"\n\n[dev-dependencies]\n" +
			"meta = { path = \"../meta\" }\n",
		"code/packages/rust/other/Cargo.toml": "[package]\nname = \"other\"\n",
	}
	var packages []discovery.Package
	for _, name := range []string{"pre", "meta", "dep", "smoke", "other"} {
		packages = append(packages, discovery.Package{
			Name:     "rust/" + name,
			Path:     filepath.Join(root, "code", "packages", "rust", name),
			Language: "rust",
		})
	}
	for rel, content := range files {
		abs := filepath.Join(root, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(abs), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(abs, []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	for _, pkg := range packages {
		for _, front := range []string{"BUILD", "BUILD_windows"} {
			if err := os.WriteFile(filepath.Join(pkg.Path, front), []byte("cargo test\n"), 0o644); err != nil {
				t.Fatal(err)
			}
		}
	}
	return root, packages
}

// TestPackageLocalMetadataPlan pins, for the top-level set run() computes and
// for every platform in an emitted plan, that a metadata-only change selects
// the package and its prerequisites but not its dependents or dev-dependents,
// while any source change still propagates exactly as before.
func TestPackageLocalMetadataPlan(t *testing.T) {
	root, packages := metadataWorld(t)
	graph, triggers, err := resolver.ResolveWithTestTriggersAgainst(packages, packages)
	if err != nil {
		t.Fatal(err)
	}

	everyMetadataFile := []string{
		"code/packages/rust/meta/README.md",
		"code/packages/rust/meta/CHANGELOG.md",
		"code/packages/rust/meta/CHANGELOG.d/2026-11-11-0123abcd-capabilities.md",
		"code/packages/rust/meta/required_capabilities.json",
		"code/packages/rust/meta/effect_certificate.json",
		"code/packages/rust/meta/assurance_report.json",
		"code/packages/rust/meta/evidence/test-manifest.json",
	}
	sourceClosure := []string{"rust/dep", "rust/meta", "rust/pre", "rust/smoke"}

	cases := []struct {
		name    string
		changed []string
		want    []string
	}{
		{
			name:    "capability manifest and changelog only: the package and its prerequisite",
			changed: []string{"code/packages/rust/meta/required_capabilities.json", "code/packages/rust/meta/CHANGELOG.md"},
			want:    []string{"rust/meta", "rust/pre"},
		},
		{
			name:    "every metadata class at once is still local",
			changed: everyMetadataFile,
			want:    []string{"rust/meta", "rust/pre"},
		},
		{
			name:    "a source change propagates to dependents and dev-dependents",
			changed: []string{"code/packages/rust/meta/src/lib.rs"},
			want:    sourceClosure,
		},
		{
			name:    "metadata plus source in the same package propagates",
			changed: append([]string{"code/packages/rust/meta/src/lib.rs"}, everyMetadataFile...),
			want:    sourceClosure,
		},
		{
			name:    "Cargo.toml is a source input, not metadata",
			changed: []string{"code/packages/rust/meta/Cargo.toml"},
			want:    sourceClosure,
		},
		{
			name:    "a nested README is a possible test input and propagates",
			changed: []string{"code/packages/rust/meta/tests/fixtures/README.md"},
			want:    sourceClosure,
		},
		{
			name:    "metadata in one package, source in an unrelated one",
			changed: []string{"code/packages/rust/meta/README.md", "code/packages/rust/other/src/lib.rs"},
			want:    []string{"rust/meta", "rust/other", "rust/pre"},
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			// The top-level set, composed the way run() composes it.
			changed, err := changedPackageRootsForPlatform(tc.changed, packages, root, "linux")
			if err != nil {
				t.Fatal(err)
			}
			local := packageLocalChangeRoots(tc.changed, packages, root, changed)
			affected := graph.AffectedNodes(changed)
			affected = withDevDependents(affected, triggers)
			affected = expandAffectedSetWithPrereqs(graph, affected)
			affected = withPackageLocalChanges(graph, affected, local)
			if got := affectedListFromSet(affected); !reflect.DeepEqual(got, tc.want) {
				t.Fatalf("top-level affected = %v, want %v", got, tc.want)
			}

			out := filepath.Join(root, "plan.json")
			if code := emitBuildPlan(packages, graph, affected, changed, tc.changed, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code != 0 {
				t.Fatalf("emitBuildPlan exit %d", code)
			}
			data, err := os.ReadFile(out)
			if err != nil {
				t.Fatal(err)
			}
			var p plan.BuildPlan
			if err := json.Unmarshal(data, &p); err != nil {
				t.Fatal(err)
			}
			if !reflect.DeepEqual(p.AffectedPackages, tc.want) {
				t.Fatalf("plan affected = %v, want %v", p.AffectedPackages, tc.want)
			}
			for _, platform := range []string{"linux", "darwin", "windows"} {
				if got := p.StateForPlatform(platform).AffectedPackages; !reflect.DeepEqual(got, tc.want) {
					t.Fatalf("%s affected = %v, want %v", platform, got, tc.want)
				}
			}
			if !p.LanguagesNeeded["rust"] {
				t.Fatalf("a selected Rust package must request the Rust toolchain: %v", p.LanguagesNeeded)
			}
		})
	}
}

func TestPackageLocalChangeRootsExcludesPropagatingAndNil(t *testing.T) {
	root, packages := metadataWorld(t)
	if got := packageLocalChangeRoots(nil, packages, root, nil); got != nil {
		t.Fatalf("nil change list = %v, want nil (no Git answer)", got)
	}
	changed := []string{"code/packages/rust/meta/README.md", "code/packages/rust/other/CHANGELOG.md"}
	got := packageLocalChangeRoots(changed, packages, root, map[string]bool{"rust/other": true})
	if want := map[string]bool{"rust/meta": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("local = %v, want %v (a package that is already a root is not local)", got, want)
	}
}

// A local-only package never fires Cargo dev-dependency triggers and never
// reaches dependents, but a nil ("everything") set stays nil.
func TestWithPackageLocalChangesSelectsNoDependents(t *testing.T) {
	graph, triggers := triggerWorld()
	empty := affectedForGraphWithTriggers(graph, triggers, map[string]bool{}, nil, false)
	got := withPackageLocalChanges(graph, empty, map[string]bool{"emit": true})
	if want := map[string]bool{"emit": true, "base": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("affected = %v, want %v", got, want)
	}
	if withPackageLocalChanges(graph, nil, map[string]bool{"emit": true}) != nil {
		t.Fatal("a forced or fallback (nil) set must stay nil")
	}
}
