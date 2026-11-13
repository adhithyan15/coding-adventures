package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cigates"
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

func TestRngCorpusSelectsEveryFixedLane(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/rng", Language: "python"},
		{Name: "rust/rng", Language: "rust"},
		{Name: "go/rng", Language: "go"},
		{Name: "c/rng", Language: "c"},
		{Name: "cpp/rng", Language: "cpp"},
		{Name: "swift/rng", Language: "swift"},
		{Name: "typescript/rng", Language: "typescript"},
		{Name: "java/rng", Language: "java"},
		{Name: "kotlin/rng", Language: "kotlin"},
		{Name: "csharp/rng", Language: "csharp"},
		{Name: "fsharp/rng", Language: "fsharp"},
		{Name: "dart/rng", Language: "dart"},
		{Name: "ruby/rng", Language: "ruby"},
		{Name: "elixir/rng", Language: "elixir"},
		{Name: "lua/rng", Language: "lua"},
		{Name: "perl/rng", Language: "perl"},
		{Name: "haskell/rng", Language: "haskell"},
	}
	corpus := []string{"code/specs/fixtures/rng-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]bool{
		"python/rng": true, "rust/rng": true, "go/rng": true,
		"c/rng": true, "cpp/rng": true, "swift/rng": true,
		"typescript/rng": true, "java/rng": true, "kotlin/rng": true,
		"csharp/rng": true, "fsharp/rng": true, "dart/rng": true,
		"ruby/rng": true, "elixir/rng": true, "lua/rng": true,
		"perl/rng": true, "haskell/rng": true,
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	if got, _ := repositoryInputConsumersForChange(corpus, packages, "rust"); !reflect.DeepEqual(got, map[string]bool{"rust/rng": true}) {
		t.Fatalf("rust-only run selected %v", got)
	}
	for path, want := range map[string]bool{
		"code/specs/fixtures/rng-v1/cases.json":  true,
		"code/specs/fixtures/rng-v10/cases.json": false,
		"code/specs/fixtures/rng-v1":             false,
		"code/specs/rng.md":                      false,
	} {
		if isRngCorpus(path) != want {
			t.Errorf("isRngCorpus(%q) != %v", path, want)
		}
	}
}

func TestCtCompareInputsSelectTheirConsumers(t *testing.T) {
	corpusLanes := []string{"dart", "haskell", "lua", "ocaml", "perl", "swift"}
	packages := []discovery.Package{
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "rust/test-bootstrap", Language: "rust"},
	}
	for _, lane := range corpusLanes {
		packages = append(packages, discovery.Package{Name: lane + "/ct-compare", Language: lane})
	}
	allCt := map[string]bool{"rust/ct-compare": true}
	for _, lane := range corpusLanes {
		allCt[lane+"/ct-compare"] = true
	}
	for _, tc := range []struct {
		input string
		want  map[string]bool
	}{
		// cases.json is read by every lane; the Rust adapter also owns the
		// TFV projection and the Python reference scripts.
		{"code/specs/fixtures/ct-compare-v1/cases.json", allCt},
		{"code/specs/fixtures/ct-compare-v1/cases.tfv", map[string]bool{"rust/ct-compare": true}},
		{"code/scripts/ct_compare_tfv.py", map[string]bool{"rust/ct-compare": true}},
		{"code/scripts/ct_compare_conformance.py", map[string]bool{"rust/ct-compare": true}},
	} {
		got, err := repositoryInputConsumersForChange([]string{tc.input}, packages, "all")
		if err != nil || !reflect.DeepEqual(got, tc.want) {
			t.Fatalf("%s: got %v, err %v", tc.input, got, err)
		}
		if _, err := repositoryInputConsumersForChange([]string{tc.input}, packages[1:2], "all"); err == nil {
			t.Fatal("missing CT01 consumer must fail closed")
		}
		if got, err := repositoryInputConsumersForChange([]string{tc.input}, packages, "go"); err != nil || len(got) != 0 {
			t.Fatalf("single-language filtering: %v, %v", got, err)
		}
	}
	// A single corpus lane is selectable on its own.
	got, err := repositoryInputConsumersForChange(
		[]string{"code/specs/fixtures/ct-compare-v1/cases.json"}, packages, "lua")
	if err != nil || !reflect.DeepEqual(got, map[string]bool{"lua/ct-compare": true}) {
		t.Fatalf("lua filter: got %v, err %v", got, err)
	}
	for _, input := range []string{
		"code/specs/fixtures/ct-compare-v10/cases.json",
		"code/specs/fixtures/ct-compare-v1",
		"docs/code/specs/fixtures/ct-compare-v1/cases.json",
		`code\specs\fixtures\ct-compare-v1\cases.json`,
		"code/scripts/ct_compare_tfv.pyc",
		"code/scripts/tests/test_ct_compare_tfv.py",
	} {
		if isCtCompareInput(input) {
			t.Errorf("lookalike selected CT01 consumer: %s", input)
		}
		if isCtCompareCorpus(input) {
			t.Errorf("lookalike selected a CT01 corpus lane: %s", input)
		}
	}
	if isCtCompareCorpus("code/specs/fixtures/ct-compare-v1/cases.tfv") ||
		isCtCompareCorpus("code/scripts/tfv.py") {
		t.Fatal("corpus lanes must not read the TFV projection or scripts")
	}
	if !isCtCompareInput("code/scripts/tfv.py") {
		t.Fatal("TFV reference changes must select the CT01 adapter too")
	}
}

func TestIsTestBootstrapInput(t *testing.T) {
	cases := []struct {
		path string
		want bool
	}{
		{"code/specs/fixtures/test-bootstrap-v1/cases.json", true},
		{"code/specs/fixtures/test-bootstrap-v1/cases.tfv", true},
		{"code/scripts/tfv.py", true},
		{"code/scripts/bootstrap_harness_conformance.py", true},
		// Lookalikes: another corpus version, another fixture, the
		// validator's own tests (CI runs those on the contract path), and
		// paths that merely contain the right words.
		{"code/specs/fixtures/test-bootstrap-v10/cases.json", false},
		{"code/specs/fixtures/test-bootstrap-v1", false},
		{"code/specs/fixtures/ct-compare-v1/cases.json", false},
		{"code/scripts/tests/test_bootstrap_harness_conformance.py", false},
		{"code/scripts/tfv.pyc", false},
		{"docs/code/specs/fixtures/test-bootstrap-v1/cases.json", false},
		{`code\specs\fixtures\test-bootstrap-v1\cases.json`, false},
	}
	for _, tc := range cases {
		if got := isTestBootstrapInput(tc.path); got != tc.want {
			t.Errorf("isTestBootstrapInput(%q) = %v, want %v", tc.path, got, tc.want)
		}
	}
}

func TestTestBootstrapCorpusSelectsTheHarness(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/test-bootstrap", Language: "rust"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	corpus := []string{"code/specs/fixtures/test-bootstrap-v1/cases.tfv"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"rust/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[1:], "all"); err == nil {
		t.Fatal("a corpus change with the harness missing must fail closed")
	}
}

func TestNativePerlBuildInputsSelectTheirRuntimeConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "perl/conduit", Language: "perl"},
		{Name: "perl/irc-server-native", Language: "perl"},
		{Name: "perl/programs/conduit-hello", Language: "perl"},
		{Name: "perl/gf256_native", Language: "perl"},
		{Name: "perl/polynomial_native", Language: "perl"},
		{Name: "perl/paint_codec_png_native", Language: "perl"},
	}
	want := map[string]bool{
		"perl/conduit":                true,
		"perl/irc-server-native":      true,
		"perl/programs/conduit-hello": true,
		"perl/gf256_native":           true,
		"perl/polynomial_native":      true,
		"perl/paint_codec_png_native": true,
	}
	for _, input := range []string{
		"code/scripts/check-perl-extension-windows.ps1",
		"code/packages/rust/perl-bridge/Cargo.toml",
		"code/packages/rust/perl-bridge/build.rs",
		"code/packages/rust/perl-bridge/src/perl_bridge_shim.c",
		"code/packages/rust/perl-bridge/src/lib.rs",
	} {
		for _, language := range []string{"all", "perl"} {
			got, err := repositoryInputConsumersForChange([]string{input}, packages, language)
			if err != nil || !reflect.DeepEqual(got, want) {
				t.Fatalf("%s (%s): got %v, err %v; want all native Perl consumers", input, language, got, err)
			}
		}
		if _, err := repositoryInputConsumersForChange([]string{input}, packages[:2], "all"); err == nil {
			t.Fatalf("%s: a missing runtime consumer must fail closed", input)
		}
		if got, err := repositoryInputConsumersForChange([]string{input}, nil, "go"); err != nil || len(got) != 0 {
			t.Fatalf("%s: another language should not select Perl: %v, %v", input, got, err)
		}
	}
	for _, other := range []string{
		"code/scripts/check-perl-extension-windows.ps1.bak",
		"code/packages/rust/perl-bridge/README.md",
		"code/packages/rust/perl-bridge-other/src/lib.rs",
	} {
		got, err := repositoryInputConsumersForChange([]string{other}, packages, "all")
		if err != nil || len(got) != 0 {
			t.Fatalf("unrelated input %s selected %v: %v", other, got, err)
		}
	}
}

func TestNativePerlRecipesRequestRustForHelperOnlyChanges(t *testing.T) {
	root := findRepoRoot("")
	names := []string{
		"perl/conduit", "perl/irc-server-native", "perl/programs/conduit-hello",
		"perl/gf256_native", "perl/polynomial_native", "perl/paint_codec_png_native",
	}
	paths := []string{
		"code/packages/perl/conduit", "code/packages/perl/irc-server-native", "code/programs/perl/conduit-hello",
		"code/packages/perl/gf256_native", "code/packages/perl/polynomial_native", "code/packages/perl/paint_codec_png_native",
	}
	for _, platform := range []string{"linux", "darwin", "windows"} {
		var packages []discovery.Package
		for i, path := range paths {
			fronts := make(map[string]string)
			for _, front := range []string{"BUILD", "BUILD_windows"} {
				raw, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(path), front))
				if err != nil {
					t.Fatal(err)
				}
				fronts[front] = string(raw)
			}
			packages = append(packages, discovery.Package{
				Name: names[i], Language: "perl",
				ExtraToolchains: discovery.ExtraToolchainsForSnapshot(fronts, platform),
			})
		}
		consumers, err := repositoryInputConsumersForChange(
			[]string{"code/scripts/check-perl-extension-windows.ps1"}, packages, "all")
		if err != nil {
			t.Fatal(err)
		}
		needed := computeLanguagesNeeded(packages, consumers, false, nil)
		if !needed["perl"] || !needed["rust"] {
			t.Fatalf("%s helper-only edit needs Perl and Rust: %v", platform, needed)
		}
		registry, err := cigates.Load(filepath.Join(root, filepath.FromSlash(cigates.DefaultRegistryPath)))
		if err != nil {
			t.Fatal(err)
		}
		verdicts, err := cigates.Evaluate(registry, consumers,
			[]string{"code/scripts/check-perl-extension-windows.ps1"}, false)
		if err != nil {
			t.Fatal(err)
		}
		if !verdicts["build-windows-os-suites"] {
			t.Fatalf("%s helper-only plan must execute the Windows native suite", platform)
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
