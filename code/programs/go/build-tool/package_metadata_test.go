package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
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

// additive gives every path the status a plain edit has.
func additive(paths ...string) gitdiff.ChangeStatuses {
	statuses := gitdiff.ChangeStatuses{}
	for _, path := range paths {
		statuses[path] = 'M'
	}
	return statuses
}

// planFor composes the top-level affected set the way run() does and emits a
// plan the way the detect job does, returning the top-level list and each
// platform's list.
func planFor(t *testing.T, root string, packages []discovery.Package, changed []string, statuses gitdiff.ChangeStatuses) ([]string, map[string][]string) {
	t.Helper()
	graph, triggers, err := resolver.ResolveWithTestTriggersAgainst(packages, packages)
	if err != nil {
		t.Fatal(err)
	}
	roots, local, err := selectPackageRoots(changed, statuses, packages, root, "linux", "all")
	if err != nil {
		t.Fatal(err)
	}
	affected := graph.AffectedNodes(roots)
	affected = withDevDependents(affected, triggers)
	affected = expandAffectedSetWithPrereqs(graph, affected)
	affected = withPackageLocalChanges(graph, affected, local)

	out := filepath.Join(root, "plan.json")
	if code := emitBuildPlanAgainst(nil, packages, graph, affected, roots, changed, statuses, false, nil, "origin/main", root, out, false, 0, false, "", "all"); code != 0 {
		t.Fatalf("emitBuildPlanAgainst exit %d", code)
	}
	data, err := os.ReadFile(out)
	if err != nil {
		t.Fatal(err)
	}
	var p plan.BuildPlan
	if err := json.Unmarshal(data, &p); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(p.AffectedPackages, affectedListFromSet(affected)) {
		t.Fatalf("plan affected = %v, top-level = %v", p.AffectedPackages, affectedListFromSet(affected))
	}
	platforms := map[string][]string{}
	for _, platform := range []string{"linux", "darwin", "windows"} {
		platforms[platform] = p.StateForPlatform(platform).AffectedPackages
	}
	return affectedListFromSet(affected), platforms
}

// TestPackageLocalMetadataPlan pins, for the top-level set run() computes and
// for every platform in an emitted plan, that an added or modified metadata
// file selects the package and its prerequisites but not its dependents or
// dev-dependents, while source changes, README.md, deletions, renames and
// type changes still propagate exactly as before.
func TestPackageLocalMetadataPlan(t *testing.T) {
	root, packages := metadataWorld(t)

	everyMetadataFile := []string{
		"code/packages/rust/meta/CHANGELOG.md",
		"code/packages/rust/meta/CHANGELOG.d/2026-11-11-0123abcd-capabilities.md",
		"code/packages/rust/meta/required_capabilities.json",
		"code/packages/rust/meta/effect_certificate.json",
		"code/packages/rust/meta/assurance_report.json",
		"code/packages/rust/meta/evidence/test-manifest.json",
		"code/packages/rust/meta/evidence/proofs/a.drat",
	}
	sourceClosure := []string{"rust/dep", "rust/meta", "rust/pre", "rust/smoke"}
	local := []string{"rust/meta", "rust/pre"}

	cases := []struct {
		name     string
		changed  []string
		statuses gitdiff.ChangeStatuses
		want     []string
	}{
		{
			name:     "capability manifest and changelog only: the package and its prerequisite",
			changed:  []string{"code/packages/rust/meta/required_capabilities.json", "code/packages/rust/meta/CHANGELOG.md"},
			statuses: additive("code/packages/rust/meta/required_capabilities.json", "code/packages/rust/meta/CHANGELOG.md"),
			want:     local,
		},
		{
			name:     "every metadata class at once is still local",
			changed:  everyMetadataFile,
			statuses: additive(everyMetadataFile...),
			want:     local,
		},
		{
			name:     "an added changelog shard is local",
			changed:  []string{"code/packages/rust/meta/CHANGELOG.d/2026-11-11-0123abcd-new.md"},
			statuses: gitdiff.ChangeStatuses{"code/packages/rust/meta/CHANGELOG.d/2026-11-11-0123abcd-new.md": 'A'},
			want:     local,
		},
		{
			name:     "README.md propagates: dependents' packaging reads it",
			changed:  []string{"code/packages/rust/meta/README.md"},
			statuses: additive("code/packages/rust/meta/README.md"),
			want:     sourceClosure,
		},
		{
			name:     "a deleted changelog propagates: a dependent may need it to exist",
			changed:  []string{"code/packages/rust/meta/CHANGELOG.md"},
			statuses: gitdiff.ChangeStatuses{"code/packages/rust/meta/CHANGELOG.md": 'D'},
			want:     sourceClosure,
		},
		{
			name:     "a rename inside evidence/ carries a deletion and propagates",
			changed:  []string{"code/packages/rust/meta/evidence/old.json", "code/packages/rust/meta/evidence/new.json"},
			statuses: gitdiff.ChangeStatuses{"code/packages/rust/meta/evidence/old.json": 'D', "code/packages/rust/meta/evidence/new.json": 'A'},
			want:     sourceClosure,
		},
		{
			name:     "a type change (file to symlink) propagates",
			changed:  []string{"code/packages/rust/meta/required_capabilities.json"},
			statuses: gitdiff.ChangeStatuses{"code/packages/rust/meta/required_capabilities.json": 'T'},
			want:     sourceClosure,
		},
		{
			name:     "an unknown status propagates",
			changed:  []string{"code/packages/rust/meta/CHANGELOG.md"},
			statuses: nil,
			want:     sourceClosure,
		},
		{
			name:     "a script under evidence/ is not data and propagates",
			changed:  []string{"code/packages/rust/meta/evidence/run.sh"},
			statuses: additive("code/packages/rust/meta/evidence/run.sh"),
			want:     sourceClosure,
		},
		{
			name:     "a source change propagates to dependents and dev-dependents",
			changed:  []string{"code/packages/rust/meta/src/lib.rs"},
			statuses: additive("code/packages/rust/meta/src/lib.rs"),
			want:     sourceClosure,
		},
		{
			name:     "metadata plus source in the same package propagates",
			changed:  append([]string{"code/packages/rust/meta/src/lib.rs"}, everyMetadataFile...),
			statuses: additive(append([]string{"code/packages/rust/meta/src/lib.rs"}, everyMetadataFile...)...),
			want:     sourceClosure,
		},
		{
			name:     "Cargo.toml is a source input, not metadata",
			changed:  []string{"code/packages/rust/meta/Cargo.toml"},
			statuses: additive("code/packages/rust/meta/Cargo.toml"),
			want:     sourceClosure,
		},
		{
			name:     "a nested capability manifest is a possible test input and propagates",
			changed:  []string{"code/packages/rust/meta/tests/fixtures/required_capabilities.json"},
			statuses: additive("code/packages/rust/meta/tests/fixtures/required_capabilities.json"),
			want:     sourceClosure,
		},
		{
			name:     "metadata in one package, source in an unrelated one",
			changed:  []string{"code/packages/rust/meta/CHANGELOG.md", "code/packages/rust/other/src/lib.rs"},
			statuses: additive("code/packages/rust/meta/CHANGELOG.md", "code/packages/rust/other/src/lib.rs"),
			want:     []string{"rust/meta", "rust/other", "rust/pre"},
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			top, platforms := planFor(t, root, packages, tc.changed, tc.statuses)
			if !reflect.DeepEqual(top, tc.want) {
				t.Fatalf("top-level affected = %v, want %v", top, tc.want)
			}
			for platform, got := range platforms {
				if !reflect.DeepEqual(got, tc.want) {
					t.Fatalf("%s affected = %v, want %v", platform, got, tc.want)
				}
			}
		})
	}
}

// A metadata change beside an edit to a BUILD front that the platform does
// not select must keep the owner on every platform. On Linux and macOS the
// BUILD_windows edit is filtered out, so the package is local there; on
// Windows the edit is the selected front and the package propagates.
// (Before the fix, local roots were classified from the unfiltered list and
// the owner fell out of both sets on Linux and macOS.)
func TestPackageLocalMetadataBesideNonSelectedBuildFront(t *testing.T) {
	root, packages := metadataWorld(t)
	sourceClosure := []string{"rust/dep", "rust/meta", "rust/pre", "rust/smoke"}
	for _, tc := range []struct {
		name     string
		changed  []string
		statuses gitdiff.ChangeStatuses
	}{
		{
			name:     "edited BUILD_windows",
			changed:  []string{"code/packages/rust/meta/effect_certificate.json", "code/packages/rust/meta/BUILD_windows"},
			statuses: additive("code/packages/rust/meta/effect_certificate.json", "code/packages/rust/meta/BUILD_windows"),
		},
		{
			name:     "added BUILD_mac",
			changed:  []string{"code/packages/rust/meta/required_capabilities.json", "code/packages/rust/meta/BUILD_mac"},
			statuses: gitdiff.ChangeStatuses{"code/packages/rust/meta/required_capabilities.json": 'M', "code/packages/rust/meta/BUILD_mac": 'A'},
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if tc.name == "added BUILD_mac" {
				if err := os.WriteFile(filepath.Join(root, "code/packages/rust/meta/BUILD_mac"), []byte("cargo test\n"), 0o644); err != nil {
					t.Fatal(err)
				}
				t.Cleanup(func() { os.Remove(filepath.Join(root, "code/packages/rust/meta/BUILD_mac")) })
			}
			_, platforms := planFor(t, root, packages, tc.changed, tc.statuses)
			for platform, got := range platforms {
				front := strings.TrimPrefix(filepath.Base(tc.changed[1]), "BUILD_")
				selected := map[string]string{"windows": "windows", "mac": "darwin"}[front] == platform
				want := []string{"rust/meta", "rust/pre"}
				if selected {
					want = sourceClosure
				}
				if !reflect.DeepEqual(got, want) {
					t.Fatalf("%s affected = %v, want %v", platform, got, want)
				}
			}
			for _, goos := range []string{"linux", "darwin", "windows"} {
				roots, local, err := selectPackageRoots(tc.changed, tc.statuses, packages, root, goos, "all")
				if err != nil {
					t.Fatal(err)
				}
				if !roots["rust/meta"] && !local["rust/meta"] {
					t.Fatalf("%s: the owner fell out of both sets: roots = %v, local = %v", goos, roots, local)
				}
			}
		})
	}
}

func TestSelectPackageRootsSeparatesLocalFromSeeds(t *testing.T) {
	root, packages := metadataWorld(t)
	if roots, local, err := selectPackageRoots(nil, nil, packages, root, "linux", "all"); roots != nil || local != nil || err != nil {
		t.Fatalf("nil change list = %v, %v, %v; want nil (no Git answer)", roots, local, err)
	}
	changed := []string{"code/packages/rust/meta/CHANGELOG.md", "code/packages/rust/other/CHANGELOG.md", "code/packages/rust/other/src/lib.rs"}
	roots, local, err := selectPackageRoots(changed, additive(changed...), packages, root, "linux", "all")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"rust/meta": true}; !reflect.DeepEqual(local, want) {
		t.Fatalf("local = %v, want %v (a package that is already a root is not local)", local, want)
	}
	if want := map[string]bool{"rust/other": true}; !reflect.DeepEqual(roots, want) {
		t.Fatalf("roots = %v, want %v", roots, want)
	}
	// The compatibility wrapper has no statuses, so metadata propagates.
	legacy, err := changedPackageRootsForPlatformAndLanguage(changed, packages, root, "linux", "all")
	if err != nil || !legacy["rust/meta"] || !legacy["rust/other"] {
		t.Fatalf("changedPackageRootsForPlatformAndLanguage = %v, %v", legacy, err)
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
