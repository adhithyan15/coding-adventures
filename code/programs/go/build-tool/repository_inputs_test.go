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

func TestIsMosaicRepositorySource(t *testing.T) {
	cases := []struct {
		path string
		want bool
	}{
		{"code/packages/mosaic/mosaic-pkg-toolkit/mosaic-package.toml", true},
		{"code/programs/mosaic/task-app/mosaic-package.toml", true},
		{"code/programs/mosaic/task-app/src/TaskApp.mll", true},
		{"code/packages/mosaic/mosaic-pkg-toolkit/src/Button.mil", true},
		{"code/packages/mosaic/mosaic-pkg-toolkit/src/Button.dark.msl", true},
		{"code/packages/rust/mosaic-emit-qt/fixtures/host-slider/src/Slider.mll", true},
		{"code/packages/rust/mosaic-app-conformance/package/mosaic-package.toml", true},
		// Not a guard input: other files in a Mosaic package, lookalike
		// names, paths outside code/, and non-Git (backslash) spellings.
		{"code/packages/mosaic/mosaic-pkg-toolkit/README.md", false},
		{"code/packages/mosaic/mosaic-pkg-toolkit/Cargo.toml", false},
		{"code/packages/mosaic/x/mosaic-package.toml.bak", false},
		{"code/packages/mosaic/x/old-mosaic-package.toml", false},
		{"code/packages/mosaic/x/src/Button.mill", false},
		{"code/packages/mosaic/x/src/.mil", false},
		{"docs/mosaic/Button.mil", false},
		{"mosaic-package.toml", false},
		{`code\packages\mosaic\x\Button.mil`, false},
	}
	for _, tc := range cases {
		if got := isMosaicRepositorySource(tc.path); got != tc.want {
			t.Errorf("isMosaicRepositorySource(%q) = %v, want %v", tc.path, got, tc.want)
		}
	}
}

func TestRepositoryInputConsumersForChange(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/mosaic-package-resolver", Language: "rust"},
		{Name: "mosaic/mosaic-pkg-toolkit", Language: "mosaic"},
	}
	mll := []string{"code/programs/mosaic/task-app/src/TaskApp.mll"}

	cases := []struct {
		name     string
		changed  []string
		packages []discovery.Package
		language string
		want     map[string]bool
		wantErr  bool
	}{
		{"mosaic source selects the guard", mll, packages, "all", map[string]bool{"rust/mosaic-package-resolver": true}, false},
		{"rust-only invocation still selects it", mll, packages, "rust", map[string]bool{"rust/mosaic-package-resolver": true}, false},
		{"other single-language invocation does not", mll, packages, "mosaic", map[string]bool{}, false},
		{"unrelated change selects nothing", []string{"code/packages/mosaic/mosaic-pkg-toolkit/README.md"}, packages, "all", map[string]bool{}, false},
		{"empty change list selects nothing", []string{}, packages, "all", map[string]bool{}, false},
		{"nil change list is fallback mode", nil, packages, "all", nil, false},
		{"missing guard is an error", mll, packages[1:], "all", nil, true},
		{"missing guard is irrelevant to another language", mll, packages[1:], "mosaic", map[string]bool{}, false},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got, err := repositoryInputConsumersForChange(tc.changed, tc.packages, tc.language)
			if (err != nil) != tc.wantErr {
				t.Fatalf("error = %v, wantErr %v", err, tc.wantErr)
			}
			if !reflect.DeepEqual(got, tc.want) {
				t.Fatalf("got %v, want %v", got, tc.want)
			}
		})
	}
}

// mosaicGuardWorld writes a miniature repository with the real dependency
// shape: the guard crate depends on the manifest crate, the artifact builder
// depends on the guard, and a Mosaic app depends on the toolkit through its
// mosaic-package.toml.
func mosaicGuardWorld(t *testing.T) (string, []discovery.Package) {
	t.Helper()
	root := t.TempDir()
	files := map[string]string{
		"code/packages/rust/mosaic-package-manifest/Cargo.toml": "[package]\nname = \"mosaic-package-manifest\"\n",
		"code/packages/rust/mosaic-package-resolver/Cargo.toml": "[package]\nname = \"mosaic-package-resolver\"\n\n[dependencies]\n" +
			"mosaic-package-manifest = { path = \"../mosaic-package-manifest\" }\n",
		"code/packages/rust/mosaic-package-artifact-builder/Cargo.toml": "[package]\nname = \"mosaic-package-artifact-builder\"\n\n[dependencies]\n" +
			"mosaic-package-resolver = { path = \"../mosaic-package-resolver\" }\n",
		"code/packages/mosaic/mosaic-pkg-toolkit/mosaic-package.toml": "[package]\nname = \"mosaic-pkg-toolkit\"\nversion = \"0.16.0\"\n",
		"code/programs/mosaic/task-app/mosaic-package.toml":           "[package]\nname = \"task-app\"\n\n[dependencies]\nmosaic-pkg-toolkit = \"0.16.0\"\n",
	}
	packages := []discovery.Package{
		{Name: "rust/mosaic-package-manifest", Path: filepath.Join(root, "code/packages/rust/mosaic-package-manifest"), Language: "rust"},
		{Name: "rust/mosaic-package-resolver", Path: filepath.Join(root, "code/packages/rust/mosaic-package-resolver"), Language: "rust"},
		{Name: "rust/mosaic-package-artifact-builder", Path: filepath.Join(root, "code/packages/rust/mosaic-package-artifact-builder"), Language: "rust"},
		{Name: "mosaic/mosaic-pkg-toolkit", Path: filepath.Join(root, "code/packages/mosaic/mosaic-pkg-toolkit"), Language: "mosaic"},
		{Name: "mosaic/programs/task-app", Path: filepath.Join(root, "code/programs/mosaic/task-app"), Language: "mosaic"},
	}
	for rel, content := range files {
		abs := filepath.Join(root, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(abs), 0755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(abs, []byte(content), 0644); err != nil {
			t.Fatal(err)
		}
	}
	for _, pkg := range packages {
		for _, front := range []string{"BUILD", "BUILD_windows"} {
			if err := os.WriteFile(filepath.Join(pkg.Path, front), []byte("cargo test\n"), 0644); err != nil {
				t.Fatal(err)
			}
		}
	}
	return root, packages
}

// TestRepositoryInputConsumersPlan drives emitBuildPlan for the three
// changes the CI gap was about, plus a control, and pins the per-platform
// affected set CI shards consume. The artifact builder is the canary: it
// depends on the guard, so it must appear only when the guard's own code
// changed, never merely because a Mosaic source did.
func TestRepositoryInputConsumersPlan(t *testing.T) {
	root, packages := mosaicGuardWorld(t)
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}

	cases := []struct {
		name    string
		changed []string
		want    []string
	}{
		{
			name:    "toolkit version bump: dependents and the guard, not the guard's dependents",
			changed: []string{"code/packages/mosaic/mosaic-pkg-toolkit/mosaic-package.toml"},
			want: []string{
				"mosaic/mosaic-pkg-toolkit", "mosaic/programs/task-app",
				"rust/mosaic-package-manifest", "rust/mosaic-package-resolver",
			},
		},
		{
			name:    "app layout edit: the app, its prerequisites, and the guard",
			changed: []string{"code/programs/mosaic/task-app/src/TaskApp.mll"},
			want: []string{
				"mosaic/mosaic-pkg-toolkit", "mosaic/programs/task-app",
				"rust/mosaic-package-manifest", "rust/mosaic-package-resolver",
			},
		},
		{
			name:    "control: a non-source file in a Mosaic package does not select the guard",
			changed: []string{"code/packages/mosaic/mosaic-pkg-toolkit/README.md"},
			want:    []string{"mosaic/mosaic-pkg-toolkit", "mosaic/programs/task-app"},
		},
		{
			name:    "control: a guard code change still reaches its dependents",
			changed: []string{"code/packages/rust/mosaic-package-resolver/src/lib.rs"},
			want: []string{
				"rust/mosaic-package-artifact-builder", "rust/mosaic-package-manifest",
				"rust/mosaic-package-resolver",
			},
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			// The top-level set, computed the way run() computes it.
			changed, err := changedPackageRootsForPlatform(tc.changed, packages, root, "linux")
			if err != nil {
				t.Fatal(err)
			}
			consumers, err := repositoryInputConsumersForChange(tc.changed, packages, "all")
			if err != nil {
				t.Fatal(err)
			}
			affected := withRepositoryInputConsumers(graph, affectedForGraph(graph, changed, nil, false), consumers)
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
			if !p.LanguagesNeeded["rust"] && consumers["rust/mosaic-package-resolver"] {
				t.Fatalf("selecting the guard must request the Rust toolchain: %v", p.LanguagesNeeded)
			}
		})
	}

	// Force and hash-fallback (nil) sets mean "everything" and stay nil.
	if got := withRepositoryInputConsumers(graph, nil, map[string]bool{"rust/mosaic-package-resolver": true}); got != nil {
		t.Fatalf("nil affected set must stay nil, got %v", got)
	}

	// A plan for a Mosaic change without the guard package fails closed.
	withoutGuard := []discovery.Package{packages[0], packages[2], packages[3], packages[4]}
	g, err := resolver.ResolveDependencies(withoutGuard)
	if err != nil {
		t.Fatal(err)
	}
	mll := []string{"code/programs/mosaic/task-app/src/TaskApp.mll"}
	if code := emitBuildPlan(withoutGuard, g, map[string]bool{}, map[string]bool{}, mll, false, nil, "origin/main", root, filepath.Join(root, "missing.json"), false, 0, false, "", "all"); code == 0 {
		t.Fatal("a plan was written although the guard consumer is missing")
	}
	if _, err := os.Stat(filepath.Join(root, "missing.json")); !os.IsNotExist(err) {
		t.Fatal("a failed plan created its output file")
	}
}
