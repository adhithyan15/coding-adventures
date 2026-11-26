package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
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

func TestMicrobenchmarkPairsFixtureSelectsFourConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/microbenchmark", Language: "go"},
		{Name: "go/rank-statistics", Language: "go"},
		{Name: "python/microbenchmark", Language: "python"},
		{Name: "python/rank-statistics", Language: "python"},
	}
	fixture := []string{microbenchmarkPairsFixturePath}
	want := map[string]bool{"go/microbenchmark": true, "go/rank-statistics": true, "python/microbenchmark": true, "python/rank-statistics": true}
	if got, err := repositoryInputConsumersForChange(fixture, packages, "all"); err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("all-language fixture selection got %v, err %v; want %v", got, err, want)
	}
	for language, expected := range map[string]map[string]bool{
		"go":     {"go/microbenchmark": true, "go/rank-statistics": true},
		"python": {"python/microbenchmark": true, "python/rank-statistics": true},
	} {
		if got, err := repositoryInputConsumersForChange(fixture, packages, language); err != nil || !reflect.DeepEqual(got, expected) {
			t.Errorf("%s fixture selection got %v, err %v; want %v", language, got, err, expected)
		}
	}
	for path, matches := range map[string]bool{
		microbenchmarkPairsFixturePath:                     true,
		microbenchmarkPairsFixturePath + ".bak":            false,
		"code/specs/fixtures/microbenchmark-pairs-v2.json": false,
		"code/specs/fixtures/other.json":                   false,
		`code\specs\fixtures\microbenchmark-pairs-v1.json`: false,
	} {
		if got := isMicrobenchmarkPairsFixture(path); got != matches {
			t.Errorf("isMicrobenchmarkPairsFixture(%q) = %v, want %v", path, got, matches)
		}
	}
	if got, err := repositoryInputConsumersForChange([]string{microbenchmarkPairsFixturePath}, packages[:3], "all"); err == nil || len(got) != 0 {
		t.Fatalf("missing consumer must fail closed with no partial selection: got %v, err %v", got, err)
	}
}

func TestGameClockInputsSelectRustConsumerAndGlobalWarrantChecker(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/game-clock", Language: "rust"},
		{Name: "rust/fixed-point", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "rust/warrant-ir", Language: "rust"},
		{Name: "typescript/game-clock", Language: "typescript"},
		{Name: "rust/warrant-grammar", Language: "rust"},
		{Name: "rust/wasm-leb128", Language: "rust"},
		// wasm-leb128 is a warrantAdopters row since its adopter PR (CV2 S10), so a change to its spec
		// selects the generator too, and a consumer missing from discovery is a hard error
		{Name: "rust/warrant-corpus-gen", Language: "rust"},
		{Name: "python/warrant-oracle", Language: "python"},
		{Name: "go/warrant-corpus", Language: "go"},
		{Name: "go/build-tool", Language: "go"},
	}
	corpus := []string{"code/specs/fixtures/game-clock-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"rust/game-clock": true, "typescript/game-clock": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	if got, err := repositoryInputConsumersForChange(corpus, packages, "rust"); err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/game-clock": true}) {
		t.Fatalf("rust-only selection got %v, err %v", got, err)
	}
	if got, err := repositoryInputConsumersForChange(corpus, packages, "typescript"); err != nil || !reflect.DeepEqual(got, map[string]bool{"typescript/game-clock": true}) {
		t.Fatalf("typescript-only selection got %v, err %v", got, err)
	}
	if got, err := repositoryInputConsumersForChange(corpus, packages, "go"); err != nil || len(got) != 0 {
		t.Fatalf("go-only selection got %v, err %v", got, err)
	}
	warrantPath := []string{"code/specs/game-clock.warrant"}
	if got, err := repositoryInputConsumersForChange(warrantPath, packages, "all"); err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/game-clock": true, "rust/warrant-grammar": true}) {
		t.Fatalf("GAME10 Warrant selection got %v, err %v", got, err)
	}
	if got, err := repositoryInputConsumersForChange(warrantPath, packages, "typescript"); err != nil || len(got) != 0 {
		t.Fatalf("TypeScript must not be selected for the Warrant spec: got %v, err %v", got, err)
	}
	if got, err := repositoryInputConsumersForChange(warrantPath, packages, "rust"); err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/game-clock": true, "rust/warrant-grammar": true}) {
		t.Fatalf("rust-only GAME10 Warrant selection got %v, err %v", got, err)
	}
	for _, input := range gameClockInputPaths {
		selected, err := repositoryInputConsumersForChange([]string{input}, packages, "rust")
		if err != nil || !selected["rust/game-clock"] {
			t.Fatalf("Rust-only input %s selected %v, err %v; want rust/game-clock", input, selected, err)
		}
	}
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/wasm-leb128.warrant"}, packages, "all"); err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/wasm-leb128": true, "rust/warrant-grammar": true, "python/warrant-oracle": true, "rust/warrant-corpus-gen": true}) {
		t.Fatalf("other Warrant consumers selection got %v, err %v", got, err)
	}
	for path, want := range map[string]bool{
		"code/specs/fixtures/game-clock-v1/cases.json":     true,
		"code/specs/fixtures/game-clock-v10/cases.json":    false,
		"code/specs/fixtures/game-clock-v1":                false,
		"code/specs/fixtures/game-clock-v1-old/cases.json": false,
		"code/specs/game-clock-v10.warrant":                false,
		"code/specs/game-clock.warrant.bak":                false,
		"code/specs/other-game-clock.warrant":              false,
	} {
		if got := isGameClockCorpusInput(path); got != want {
			t.Errorf("isGameClockCorpusInput(%q) = %v, want %v", path, got, want)
		}
	}
	for _, path := range gameClockInputPaths {
		if !isGameClockInput(path) {
			t.Errorf("Rust game-clock input %q was not selected", path)
		}
	}
	if !sort.StringsAreSorted(gameClockInputPaths) {
		t.Fatalf("Rust game-clock inputs are not sorted: %v", gameClockInputPaths)
	}
	for _, path := range []string{
		".cargo/config.toml.bak",
		"code/packages/rust/Cargo.toml.bak",
		"code/packages/rust/game-clock/../../Cargo.lock",
		"code/specs/fixtures/game-clock-v10/cases.json",
		"code/specs/game-clock.warrant.bak",
	} {
		if isGameClockInput(path) {
			t.Errorf("lookalike Rust game-clock path %q was selected", path)
		}
	}
	if !isGameClockInput("code/specs/game-clock.warrant") || isGameClockCorpusInput("code/specs/game-clock.warrant") {
		t.Fatal("the Warrant spec selects Rust only, not the TypeScript corpus consumer")
	}
}

func TestGameInputFrameV1SelectsRustAndTypeScriptConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/game-input-frame", Language: "rust"},
		{Name: "rust/game-clock", Language: "rust"},
		{Name: "rust/fixed-point", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "rust/warrant-ir", Language: "rust"},
		{Name: "typescript/game-input-frame", Language: "typescript"},
	}
	for _, changed := range typescriptGameInputFrameInputs {
		selected, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil || !selected["typescript/game-input-frame"] {
			t.Fatalf("%s selected %v, err %v; want TypeScript GAME frame", changed, selected, err)
		}
		tsSelected, err := repositoryInputConsumersForChange([]string{changed}, packages, "typescript")
		if err != nil || !reflect.DeepEqual(tsSelected, map[string]bool{"typescript/game-input-frame": true}) {
			t.Fatalf("typescript-only %s selected %v, err %v", changed, tsSelected, err)
		}
	}
	if selected, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/game-input-frame-v1/schema.json"}, packages, "all"); err != nil || !reflect.DeepEqual(selected, map[string]bool{"rust/game-input-frame": true}) {
		t.Fatalf("schema-only selector = %v, err %v; want Rust-only frame consumer", selected, err)
	}
	for path, want := range map[string]bool{
		"code/specs/fixtures/game-input-frame-v1/cases.json":       true,
		"code/specs/fixtures/game-input-frame-v10/cases.json":      false,
		"code/specs/fixtures/game-input-frame-v1-old/cases.json":   false,
		"code/specs/fixtures/game-input-frame-v1":                  false,
		"code/specs/fixtures/other-game-input-frame-v1/cases.json": false,
		"code/specs/fixtures/game-input-frame-v1ish/cases.json":    false,
	} {
		if got := isGameInputFrameV1(path); got != want {
			t.Errorf("isGameInputFrameV1(%q) = %v, want %v", path, got, want)
		}
	}
	if !isTypeScriptGameInputFrameInput(".cargo/config.toml") ||
		isTypeScriptGameInputFrameInput(".cargo/config.toml.bak") ||
		isTypeScriptGameInputFrameInput("code/packages/rust/Cargo.toml.bak") ||
		isTypeScriptGameInputFrameInput("code/specs/fixtures/game-input-frame-v10/cases.json") {
		t.Fatal("TypeScript GAME proof-input predicate is not exact")
	}
}

func TestTypeScriptGameInputFrameHashSelectorAndBoundaryShareExactInputs(t *testing.T) {
	if !sort.StringsAreSorted(typescriptGameInputFrameInputs) {
		t.Fatalf("TypeScript game-input-frame hash inputs are not byte-sorted: %v", typescriptGameInputFrameInputs)
	}
	selectorInputs := repositoryInputsByPackage()["typescript/game-input-frame"]
	if selectorInputs == nil || !reflect.DeepEqual(selectorInputs.paths, typescriptGameInputFrameInputs) {
		t.Fatalf("selector inputs = %#v, want shared sorted list %v", selectorInputs, typescriptGameInputFrameInputs)
	}
	boundaryPath := filepath.Join(toolchainFixtureRepoRoot(t), "code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json")
	data, err := os.ReadFile(boundaryPath)
	if err != nil {
		t.Fatal(err)
	}
	var boundary struct {
		Boundaries []struct {
			AppliesTo struct {
				ExactRoots []string `json:"exact_roots"`
			} `json:"applies_to"`
			Inputs []struct {
				Path string `json:"path"`
			} `json:"inputs"`
		} `json:"boundaries"`
	}
	if err := json.Unmarshal(data, &boundary); err != nil {
		t.Fatal(err)
	}
	boundaryInputs := make(map[string]bool)
	const packageRoot = "code/packages/typescript/game-input-frame"
	for _, row := range boundary.Boundaries {
		if !containsString(row.AppliesTo.ExactRoots, packageRoot) {
			continue
		}
		for _, input := range row.Inputs {
			boundaryInputs[input.Path] = true
		}
	}
	var sortedBoundaryInputs []string
	for input := range boundaryInputs {
		sortedBoundaryInputs = append(sortedBoundaryInputs, input)
	}
	sort.Strings(sortedBoundaryInputs)
	if !reflect.DeepEqual(sortedBoundaryInputs, typescriptGameInputFrameInputs) {
		t.Fatalf("exact-root boundary inputs = %v, want selector/hash inputs %v", sortedBoundaryInputs, typescriptGameInputFrameInputs)
	}
}

func TestBiquadSosFixtureSelectsAllLanguageConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/dsp-filters", Language: "python"},
		{Name: "rust/dsp-filters", Language: "rust"},
		{Name: "typescript/dsp-filters", Language: "typescript"},
	}
	for _, changed := range []string{
		"code/specs/fixtures/biquad-sos-v1/cases.json",
		"code/specs/fixtures/biquad-sos-v1/README.md",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		want := map[string]bool{
			"python/dsp-filters":     true,
			"rust/dsp-filters":       true,
			"typescript/dsp-filters": true,
		}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("%s selected %v, err %v; want all consumers", changed, got, err)
		}
		if got, err := repositoryInputConsumersForChange([]string{changed}, packages, "python"); err != nil || !reflect.DeepEqual(got, map[string]bool{"python/dsp-filters": true}) {
			t.Fatalf("python-only selection got %v, err %v", got, err)
		}
		if got, err := repositoryInputConsumersForChange([]string{changed}, packages, "typescript"); err != nil || !reflect.DeepEqual(got, map[string]bool{"typescript/dsp-filters": true}) {
			t.Fatalf("typescript-only selection got %v, err %v", got, err)
		}
		if got, err := repositoryInputConsumersForChange([]string{changed}, packages, "rust"); err != nil || !reflect.DeepEqual(got, map[string]bool{"rust/dsp-filters": true}) {
			t.Fatalf("rust-only selection got %v, err %v", got, err)
		}
	}
	for changed, shouldMatch := range map[string]bool{
		"code/specs/fixtures/biquad-sos-v1/cases.json":     true,
		"code/specs/fixtures/biquad-sos-v10/cases.json":    false,
		"code/specs/fixtures/biquad-sos-v1-old/cases.json": false,
		"code/specs/fixtures/biquad-sos-v1":                false,
		`code\specs\fixtures\biquad-sos-v1\cases.json`:     false,
	} {
		if got := isBiquadSosFixture(changed); got != shouldMatch {
			t.Errorf("isBiquadSosFixture(%q) = %v, want %v", changed, got, shouldMatch)
		}
		selected, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil {
			t.Fatal(err)
		}
		if shouldMatch && len(selected) != 3 {
			t.Errorf("expected all consumers for %q, got %v", changed, selected)
		}
		if !shouldMatch && len(selected) != 0 {
			t.Errorf("lookalike %q selected consumers %v", changed, selected)
		}
	}
	for _, missing := range []string{"python", "rust", "typescript"} {
		available := make([]discovery.Package, 0, len(packages)-1)
		for _, pkg := range packages {
			if pkg.Language != missing {
				available = append(available, pkg)
			}
		}
		if _, err := repositoryInputConsumersForChange(
			[]string{"code/specs/fixtures/biquad-sos-v1/cases.json"}, available, "all",
		); err == nil {
			t.Errorf("missing %s fixture consumer must fail closed", missing)
		}
	}
}

// TestBoundedJSONDevDependencySelectsRustDSPFilters pins the test-only Cargo
// edge that makes bounded-json edits rerun the Rust Biquad/SOS consumer.
func TestBoundedJSONDevDependencySelectsRustDSPFilters(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	packages, err := discovery.DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	graph, triggers, err := resolver.ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatalf("the repository Cargo dependency graph must resolve: %v", err)
	}
	if _, err := graph.IndependentGroups(); err != nil {
		t.Fatalf("the build graph must remain acyclic: %v", err)
	}
	changed := map[string]bool{"rust/bounded-json": true}
	withoutTestTriggers := affectedForGraph(graph, changed, nil, false)
	withTestTriggers := affectedForGraphWithTriggers(graph, triggers, changed, nil, false)
	if withoutTestTriggers["rust/dsp-filters"] {
		t.Fatal("dsp-filters must not be selected through a normal build edge from bounded-json")
	}
	if !withTestTriggers["rust/dsp-filters"] {
		t.Fatal("a bounded-json edit must select dsp-filters through its Cargo dev-dependency")
	}
}

func TestHamurabiCorpusSelectsRegisteredRustConsumer(t *testing.T) {
	languages := []string{
		"rust", "go", "python", "typescript", "csharp", "fsharp", "java", "kotlin", "dart",
		"swift", "haskell", "c", "cpp", "ocaml", "ruby", "elixir", "lua", "perl",
	}
	packages := make([]discovery.Package, 0, len(languages))
	wantAll := make(map[string]bool, len(languages))
	for _, language := range languages {
		name := language + "/hamurabi"
		packages = append(packages, discovery.Package{Name: name, Language: language})
		wantAll[name] = true
	}
	corpus := []string{"code/specs/fixtures/hamurabi-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(got, wantAll) {
		t.Fatalf("all-lane selection got %v, want %v", got, wantAll)
	}
	for _, language := range languages {
		packageName := language + "/hamurabi"
		got, err := repositoryInputConsumersForChange(corpus, packages, language)
		if err != nil || !reflect.DeepEqual(got, map[string]bool{packageName: true}) {
			t.Fatalf("%s-only selection got %v, err %v", language, got, err)
		}
	}
	for path, wantMatch := range map[string]bool{
		"code/specs/fixtures/hamurabi-v1/cases.json":     true,
		"code/specs/fixtures/hamurabi-v10/cases.json":    false,
		"code/specs/fixtures/hamurabi-v1":                false,
		"code/specs/fixtures/hamurabi-v1-old/cases.json": false,
	} {
		if got := isHamurabiCorpus(path); got != wantMatch {
			t.Errorf("isHamurabiCorpus(%q) = %v, want %v", path, got, wantMatch)
		}
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[1:], "rust"); err == nil {
		t.Fatal("missing registered Rust Hamurabi consumer must fail closed")
	}
}

func TestTrigReferenceCorpusSelectsConvertedPorts(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/trig", Language: "python"},
		{Name: "rust/trig", Language: "rust"},
		{Name: "typescript/trig", Language: "typescript"},
		{Name: "c/trig", Language: "c"},
		{Name: "cpp/trig", Language: "cpp"},
		{Name: "ruby/trig", Language: "ruby"},
		{Name: "haskell/trig", Language: "haskell"},
		{Name: "java/trig", Language: "java"},
		{Name: "kotlin/trig", Language: "kotlin"},
		{Name: "swift/trig", Language: "swift"},
		{Name: "csharp/trig", Language: "csharp"},
		{Name: "fsharp/trig", Language: "fsharp"},
		{Name: "lua/trig", Language: "lua"},
		{Name: "perl/trig", Language: "perl"},
		{Name: "elixir/trig", Language: "elixir"},
		{Name: "go/trig", Language: "go"},
		{Name: "dart/trig", Language: "dart"},
		{Name: "python/rng", Language: "python"},
	}
	for _, changed := range []string{
		"code/specs/fixtures/trig-reference-v1/edge.tsv",
		"code/specs/fixtures/trig-reference-v1/manifest.json",
		"code/specs/fixtures/trig-reference-v1/sweep-cw.tsv",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil {
			t.Fatal(err)
		}
		if want := map[string]bool{"python/trig": true, "rust/trig": true, "typescript/trig": true, "c/trig": true, "cpp/trig": true, "ruby/trig": true, "haskell/trig": true, "java/trig": true, "kotlin/trig": true, "swift/trig": true, "csharp/trig": true, "fsharp/trig": true, "lua/trig": true, "perl/trig": true, "elixir/trig": true, "go/trig": true, "dart/trig": true}; !reflect.DeepEqual(got, want) {
			t.Fatalf("%s selected %v, want %v", changed, got, want)
		}
	}
	for _, lang := range []string{"python", "rust", "typescript", "c", "cpp", "ruby", "haskell", "java", "kotlin", "swift", "csharp", "fsharp", "lua", "perl", "elixir", "go", "dart"} {
		want := lang + "/trig"
		got, _ := repositoryInputConsumersForChange([]string{"code/specs/fixtures/trig-reference-v1/edge.tsv"}, packages, lang)
		if !reflect.DeepEqual(got, map[string]bool{want: true}) {
			t.Fatalf("%s-only run selected %v, want only %s", lang, got, want)
		}
	}
	for path, want := range map[string]bool{
		"code/specs/fixtures/trig-reference-v1/edge.tsv":  true,
		"code/specs/fixtures/trig-reference-v10/edge.tsv": false,
		"code/specs/fixtures/trig-reference-v1":           false,
		"code/specs/fixtures/phy00-phy01-v1/cases/x.json": false,
	} {
		if isTrigReferenceCorpus(path) != want {
			t.Errorf("isTrigReferenceCorpus(%q) != %v", path, want)
		}
	}
	// The generator, its oracle, the cross-checks and the validator select python/trig
	// too, because its BUILD runs the generator's --check; lookalikes select nothing.
	for path, want := range map[string]bool{
		"code/scripts/generate_trig_reference_fixture.py":         true,
		"code/scripts/generate_trig_reference_dart_fixtures.py":   true,
		"code/scripts/tests/test_trig_reference_dart_fixtures.py": true,
		"code/scripts/trig_ref_exact.py":                          true,
		"code/scripts/trig_ref_softfloat.py":                      true,
		"code/scripts/tests/test_trig_reference_fixture.py":       true,
		"code/scripts/trig_reference_xcheck/xcheck.c":             true,
		"code/scripts/trig_reference_xcheck/search.c":             true,
		"code/specs/fixtures/trig-reference-v1/manifest.json":     true,
		"code/scripts/generate_trig_reference_fixture.py.bak":     false,
		"code/scripts/trig_ref_exact.pyc":                         false,
		"code/scripts/trig_reference_xcheck2/x.c":                 false,
		"code/scripts/generate_other_fixture.py":                  false,
	} {
		if isTrigReferenceInput(path) != want {
			t.Errorf("isTrigReferenceInput(%q) != %v", path, want)
		}
		got, err := repositoryInputConsumersForChange([]string{path}, packages, "all")
		if err != nil {
			t.Fatal(err)
		}
		// The generator, oracle and cross-check select only python/trig, whose
		// BUILD runs --check (the Dart generator and its test too); a corpus
		// file selects every one of the seventeen converged ports.
		wantCount := 0
		if want {
			wantCount = 1
			if isTrigReferenceCorpus(path) {
				wantCount = 17
			}
		}
		if len(got) != wantCount || (want && !got["python/trig"]) {
			t.Errorf("%s selected %v, want %d consumers including python/trig", path, got, wantCount)
		}
	}
	// A converted port whose package is missing from discovery is a hard error,
	// never a silent skip.
	if _, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/trig-reference-v1/edge.tsv"}, nil, "all"); err == nil {
		t.Fatal("missing consumer must be an error")
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
	for _, path := range []string{
		"code/packages/rust/fixed-point/src.bak/lib.rs",
		"code/packages/rust/fixed-point/src-v2/lib.rs",
		"code/packages/rust/fixed-point/evidence.backup/proof.json",
		"code/packages/rust/fixed-point/evidence.bindings.bak",
		"code/packages/rust/fixed-point/../fixed-point/src/lib.rs",
	} {
		if isFixedPointReporterInput(path) || isFixedPointWarrantIRInput(path) {
			t.Errorf("fixed-point verifier selector accepted lookalike %q", path)
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

func TestPbtV1CorpusPathMatcher(t *testing.T) {
	cases := []struct {
		path string
		want bool
	}{
		{"code/specs/fixtures/pbt-v1/cases.json", true},
		{"code/specs/fixtures/pbt-v1/cases.tfv", true},
		{"code/specs/fixtures/pbt-v10/cases.json", false},
		{"code/specs/fixtures/rng-v1/cases.json", false},
		{"code/specs/VER05-property-engine.md", false},
	}
	for _, tc := range cases {
		if got := isPbtV1Corpus(tc.path); got != tc.want {
			t.Errorf("isPbtV1Corpus(%q) = %v, want %v", tc.path, got, tc.want)
		}
	}
}

func TestPbtV1CorpusSelectsThePythonEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "python")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"python/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[1:], "python"); err == nil {
		t.Fatal("a corpus change with the engine package missing must fail closed")
	}
}

func TestFuzzV1CorpusPathMatcher(t *testing.T) {
	cases := []struct {
		path string
		want bool
	}{
		{"code/specs/fixtures/fuzz-v1/cases.json", true},
		{"code/specs/fixtures/fuzz-v1/cases.tfv", true},
		{"code/specs/fixtures/fuzz-v10/cases.json", false},
		{"code/specs/fixtures/pbt-v1/cases.json", false},
		{"code/specs/VER06-coverage-guided-fuzz-engine.md", false},
	}
	for _, tc := range cases {
		if got := isFuzzV1Corpus(tc.path); got != tc.want {
			t.Errorf("isFuzzV1Corpus(%q) = %v, want %v", tc.path, got, tc.want)
		}
	}
}

func TestFuzzV1CorpusSelectsThePythonEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "elixir/test_bootstrap", Language: "elixir"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "c/test-bootstrap", Language: "c"},
	}
	corpus := []string{"code/specs/fixtures/fuzz-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]bool{"c/test-bootstrap": true, "csharp/test-bootstrap": true, "elixir/test_bootstrap": true, "fsharp/test-bootstrap": true, "go/test-bootstrap": true, "java/test-bootstrap": true, "kotlin/test-bootstrap": true, "python/test-bootstrap": true, "ruby/test_bootstrap": true}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	got, err = repositoryInputConsumersForChange(corpus, packages, "python")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"python/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("python filter: got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[:4], "python"); err == nil {
		t.Fatal("a corpus change with the engine package missing must fail closed")
	}
	got, err = repositoryInputConsumersForChange(corpus, packages, "kotlin")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"kotlin/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("kotlin filter: got %v, want %v", got, want)
	}
	// The Kotlin fuzz binding is a consumer in its own right: dropping it fails closed.
	if _, err := repositoryInputConsumersForChange(corpus, append(packages[:4:4], packages[5:]...), "kotlin"); err == nil {
		t.Fatal("a fuzz-v1 corpus change with the Kotlin binding package missing must fail closed")
	}
}

func TestPbtV1CorpusSelectsTheJavaEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "lua/test_bootstrap", Language: "lua"},
		{Name: "elixir/test_bootstrap", Language: "elixir"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "c/test-bootstrap", Language: "c"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]bool{"c/test-bootstrap": true, "csharp/test-bootstrap": true, "elixir/test_bootstrap": true, "fsharp/test-bootstrap": true, "go/test-bootstrap": true, "java/test-bootstrap": true, "kotlin/test-bootstrap": true, "lua/test_bootstrap": true, "python/test-bootstrap": true, "ruby/test_bootstrap": true}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	got, err = repositoryInputConsumersForChange(corpus, packages, "java")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"java/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("java filter: got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(corpus, append(packages[:2:2], packages[3:]...), "java"); err == nil {
		t.Fatal("a corpus change with the Java engine package missing must fail closed")
	}
	// A change to the Java package's own tests is not a corpus change.
	if got, err := repositoryInputConsumersForChange([]string{"code/packages/java/test-bootstrap/src/test/java/com/codingadventures/testbootstrap/pbt/CorpusConformanceTest.java"}, packages, "all"); err != nil || len(got) != 0 {
		t.Fatalf("an engine-source change is selected by its package, not the corpus row: %v %v", got, err)
	}
}

func TestPbtV1CorpusSelectsTheCSharpEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "lua/test_bootstrap", Language: "lua"},
		{Name: "elixir/test_bootstrap", Language: "elixir"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "c/test-bootstrap", Language: "c"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]bool{"c/test-bootstrap": true, "csharp/test-bootstrap": true, "elixir/test_bootstrap": true, "fsharp/test-bootstrap": true, "go/test-bootstrap": true, "java/test-bootstrap": true, "kotlin/test-bootstrap": true, "lua/test_bootstrap": true, "python/test-bootstrap": true, "ruby/test_bootstrap": true}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	got, err = repositoryInputConsumersForChange(corpus, packages, "csharp")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"csharp/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("csharp filter: got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[1:], "csharp"); err == nil {
		t.Fatal("a corpus change with the C# engine package missing must fail closed")
	}
	// A change to the C# package's own tests or sources is not a corpus change.
	for _, own := range []string{
		"code/packages/csharp/test-bootstrap/tests/CodingAdventures.TestBootstrap.Tests/CorpusConformanceTests.cs",
		"code/packages/csharp/test-bootstrap/Pbt/Corpus.cs",
	} {
		if got, err := repositoryInputConsumersForChange([]string{own}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("an engine-source change is selected by its package, not the corpus row: %v %v", got, err)
		}
	}
}

func TestPbtV1CorpusSelectsTheCEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "c/test-bootstrap", Language: "c"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "c")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"c/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("c filter: got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[1:], "c"); err == nil {
		t.Fatal("a corpus change with the C engine package missing must fail closed")
	}
	// The TFV rendering of the corpus is part of the same directory.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/pbt-v1/cases.tfv"}, packages, "c"); err != nil || !got["c/test-bootstrap"] {
		t.Fatalf("cases.tfv must select the C engine: %v %v", got, err)
	}
	// The fuzz-v1 corpus and the Go vector table select it through their own rows (VER06j); see
	// TestFuzzV1CorpusAndGoVectorsSelectTheCEngine.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/fuzz-v1/cases.json"}, packages, "c"); err != nil || !got["c/test-bootstrap"] {
		t.Fatalf("fuzz-v1 must select the C package: %v %v", got, err)
	}
	// A change to the C package's own tests or sources is not a corpus change.
	for _, own := range []string{
		"code/packages/c/test-bootstrap/tests/test_corpus.c",
		"code/packages/c/test-bootstrap/src/pbt_corpus.c",
	} {
		if got, err := repositoryInputConsumersForChange([]string{own}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("an engine-source change is selected by its package, not the corpus row: %v %v", got, err)
		}
	}
}

func TestPbtV1CorpusSelectsTheKotlinBinding(t *testing.T) {
	packages := []discovery.Package{
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "kotlin")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"kotlin/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("kotlin filter: got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(corpus, append(packages[:1:1], packages[2:]...), "kotlin"); err == nil {
		t.Fatal("a corpus change with the Kotlin binding package missing must fail closed")
	}
	// A change to the Kotlin package's own tests is not a corpus change.
	if got, err := repositoryInputConsumersForChange([]string{"code/packages/kotlin/test-bootstrap/src/test/kotlin/com/codingadventures/testbootstrapkt/CorpusConformanceTest.kt"}, packages, "all"); err != nil || len(got) != 0 {
		t.Fatalf("a binding-source change is selected by its package, not the corpus row: %v %v", got, err)
	}
}

func TestFuzzV1CorpusSelectsTheFSharpBinding(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	corpus := []string{"code/specs/fixtures/fuzz-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "fsharp")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"fsharp/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("fsharp filter: got %v, want %v", got, want)
	}
	// The strict TFV twin of the corpus selects it too, and the Go reference vectors do not:
	// the F# binding does not read them (VER06m section 7).
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/fuzz-v1/cases.tfv"}, packages, "fsharp"); err != nil || !got["fsharp/test-bootstrap"] {
		t.Fatalf("cases.tfv must select the F# binding: %v %v", got, err)
	}
	if got, err := repositoryInputConsumersForChange([]string{"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go"}, packages, "fsharp"); err != nil || len(got) != 0 {
		t.Fatalf("the Go reference vectors must not select the F# binding: %v %v", got, err)
	}
	// Dropping the F# package while the corpus changes must fail closed.
	if _, err := repositoryInputConsumersForChange(corpus, append(packages[:1:1], packages[2:]...), "fsharp"); err == nil {
		t.Fatal("a fuzz-v1 change with the F# binding package missing must fail closed")
	}
	// The pbt-v1 corpus and the fuzz-v1 corpus stay distinct.
	if isFuzzV1Corpus("code/specs/fixtures/pbt-v1/cases.json") {
		t.Fatal("pbt-v1 must not match the fuzz-v1 matcher")
	}
}

func TestPbtV1CorpusSelectsTheFSharpBinding(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "fsharp")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"fsharp/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("fsharp filter: got %v, want %v", got, want)
	}
	// Dropping the F# package while the corpus changes must fail closed.
	if _, err := repositoryInputConsumersForChange(corpus, append(packages[:1:1], packages[2:]...), "fsharp"); err == nil {
		t.Fatal("a corpus change with the F# binding package missing must fail closed")
	}
	// A change to the F# package's own tests or sources is not a corpus change.
	for _, own := range []string{
		"code/packages/fsharp/test-bootstrap/tests/CodingAdventures.TestBootstrap.FSharp.Tests/FsCorpusRunner.fs",
		"code/packages/fsharp/test-bootstrap/TestBootstrap.fs",
	} {
		if got, err := repositoryInputConsumersForChange([]string{own}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("a binding-source change is selected by its package, not the corpus row: %v %v", got, err)
		}
	}
}

func TestPbtV1CorpusSelectsTheGoEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "lua/test_bootstrap", Language: "lua"},
		{Name: "elixir/test_bootstrap", Language: "elixir"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "c/test-bootstrap", Language: "c"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]bool{"c/test-bootstrap": true, "csharp/test-bootstrap": true, "elixir/test_bootstrap": true, "fsharp/test-bootstrap": true, "go/test-bootstrap": true, "java/test-bootstrap": true, "kotlin/test-bootstrap": true, "lua/test_bootstrap": true, "python/test-bootstrap": true, "ruby/test_bootstrap": true}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	got, err = repositoryInputConsumersForChange(corpus, packages, "go")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"go/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("go filter: got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(corpus, append(packages[:1:1], packages[2:]...), "go"); err == nil {
		t.Fatal("a corpus change with the Go engine package missing must fail closed")
	}
	// A change to the Go package's own tests is not a corpus change.
	if got, err := repositoryInputConsumersForChange([]string{"code/packages/go/test-bootstrap/pbt/corpus_test.go"}, packages, "all"); err != nil || len(got) != 0 {
		t.Fatalf("an engine-source change is selected by its package, not the corpus row: %v %v", got, err)
	}
}

func TestPbtV1CorpusSelectsTheRubyEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "ruby")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"ruby/test_bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("ruby filter: got %v, want %v", got, want)
	}
	// The generated TFV beside the JSON is part of the corpus directory.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/pbt-v1/cases.tfv"}, packages, "ruby"); err != nil || !got["ruby/test_bootstrap"] {
		t.Fatalf("cases.tfv must select the Ruby engine too: %v %v", got, err)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[:4], "ruby"); err == nil {
		t.Fatal("a corpus change with the Ruby engine package missing must fail closed")
	}
	// The fuzz-v1 corpus is a Ruby input too (test_fuzz_corpus.rb, VER06i), through its own row.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/fuzz-v1/cases.json"}, packages, "ruby"); err != nil || !got["ruby/test_bootstrap"] {
		t.Fatalf("fuzz-v1 must select the Ruby engine: %v %v", got, err)
	}
	// A change to the Ruby package's own tests is not a corpus change.
	if got, err := repositoryInputConsumersForChange([]string{"code/packages/ruby/test_bootstrap/test/test_corpus_conformance.rb"}, packages, "all"); err != nil || len(got) != 0 {
		t.Fatalf("an engine-source change is selected by its package, not the corpus row: %v %v", got, err)
	}
}

func TestFuzzV1CorpusSelectsTheGoEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	corpus := []string{"code/specs/fixtures/fuzz-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "go")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"go/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	// The generated TFV beside the JSON is part of the corpus directory.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/fuzz-v1/cases.tfv"}, packages, "go"); err != nil || !got["go/test-bootstrap"] {
		t.Fatalf("cases.tfv must select the engine too: %v %v", got, err)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[1:], "go"); err == nil {
		t.Fatal("a corpus change with the Go engine package missing must fail closed")
	}
	// Neighbouring fixture directories and the pbt corpus are not this corpus.
	for _, other := range []string{"code/specs/fixtures/fuzz-v10/cases.json", "code/specs/fixtures/fuzz-v2/cases.json", "code/packages/go/test-bootstrap/fuzz/corpus_test.go"} {
		if got, err := repositoryInputConsumersForChange([]string{other}, packages, "go"); err != nil || len(got) != 0 {
			t.Fatalf("%s must not select the corpus row: %v %v", other, got, err)
		}
	}
	if isFuzzV1Corpus("code/specs/fixtures/pbt-v1/cases.json") || isPbtV1Corpus("code/specs/fixtures/fuzz-v1/cases.json") {
		t.Fatal("the pbt-v1 and fuzz-v1 corpora must stay distinct")
	}
}

func TestFuzzV1CorpusSelectsTheJavaEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "fsharp/test-bootstrap", Language: "fsharp"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "elixir/test_bootstrap", Language: "elixir"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "c/test-bootstrap", Language: "c"},
	}
	corpus := []string{"code/specs/fixtures/fuzz-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]bool{"c/test-bootstrap": true, "csharp/test-bootstrap": true, "elixir/test_bootstrap": true, "fsharp/test-bootstrap": true, "go/test-bootstrap": true, "java/test-bootstrap": true, "kotlin/test-bootstrap": true, "python/test-bootstrap": true, "ruby/test_bootstrap": true}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	got, err = repositoryInputConsumersForChange(corpus, packages, "java")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"java/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("java filter: got %v, want %v", got, want)
	}
	// The generated TFV beside the JSON is part of the corpus directory.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/fuzz-v1/cases.tfv"}, packages, "java"); err != nil || !got["java/test-bootstrap"] {
		t.Fatalf("cases.tfv must select the engine too: %v %v", got, err)
	}
	// A corpus change with the Java engine package missing must fail closed.
	if _, err := repositoryInputConsumersForChange(corpus, append(packages[:3:3], packages[4:]...), "java"); err == nil {
		t.Fatal("a fuzz-v1 corpus change with the Java engine package missing must fail closed")
	}
	// Neighbouring fixture directories and the Java package's own files are not this corpus.
	for _, other := range []string{
		"code/specs/fixtures/fuzz-v10/cases.json",
		"code/specs/fixtures/fuzz-v2/cases.json",
		"code/packages/java/test-bootstrap/src/test/java/com/codingadventures/testbootstrap/fuzz/FuzzCorpusConformanceTest.java",
	} {
		if got, err := repositoryInputConsumersForChange([]string{other}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("%s must not select the corpus row: %v %v", other, got, err)
		}
	}
	// The pbt-v1 corpus selects the Java package through its own row, and the two stay distinct.
	got, err = repositoryInputConsumersForChange([]string{"code/specs/fixtures/pbt-v1/cases.json"}, packages, "java")
	if err != nil || !got["java/test-bootstrap"] {
		t.Fatalf("pbt-v1 must still select the Java engine: %v %v", got, err)
	}
	if isFuzzV1Corpus("code/specs/fixtures/pbt-v1/cases.json") || isPbtV1Corpus("code/specs/fixtures/fuzz-v1/cases.json") {
		t.Fatal("the pbt-v1 and fuzz-v1 corpora must stay distinct")
	}
}

func TestGoFuzzReferenceVectorsSelectTheJavaEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "elixir/test_bootstrap", Language: "elixir"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "c/test-bootstrap", Language: "c"},
	}
	vectors := []string{"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go"}
	got, err := repositoryInputConsumersForChange(vectors, packages, "all")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"c/test-bootstrap": true, "csharp/test-bootstrap": true, "elixir/test_bootstrap": true, "java/test-bootstrap": true, "ruby/test_bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	if _, err := repositoryInputConsumersForChange(vectors, packages[:2], "java"); err == nil {
		t.Fatal("a vector-table change with the Java engine package missing must fail closed")
	}
	for _, other := range []string{
		"code/packages/go/test-bootstrap/fuzz/mutate_test.go",
		"code/packages/go/test-bootstrap/fuzz/testdata/gen_reference_vectors.py",
		"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go.bak",
	} {
		if got, err := repositoryInputConsumersForChange([]string{other}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("%s must not select the vector row: %v %v", other, got, err)
		}
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

func TestFenwickContractSelectsDartAndPythonConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "dart/fenwick-tree", Language: "dart"},
		{Name: "python/fenwick-tree", Language: "python"},
	}
	cases := map[string]bool{
		"code/specs/DT06-fenwick-tree.md":                           true,
		"code/specs/fixtures/fenwick-tree-v1/cases.json":            true,
		"code/specs/fixtures/fenwick-tree-v1/schema.json":           true,
		"code/specs/fixtures/fenwick-tree-v1/README.md":             true,
		"code/specs/fixtures/fenwick-tree-v1/validate.py":           true,
		"code/specs/DT06-fenwick-tree.md.bak":                       false,
		"code/specs/fixtures/fenwick-tree-v10/cases.json":           false,
		"code/specs/fixtures/fenwick-tree-v1x/cases.json":           false,
		"code/specs/fixtures/fenwick-tree-v1":                       false,
		"code/specs/fixtures/fenwick-tree-v1/cases.json.backup":     true,
		"code/specs/DT06-fenwick-tree-extra.md":                     false,
		"code/specs/fixtures/fenwick-tree-v1/../fenwick-tree-v10/x": false,
	}
	for path, want := range cases {
		if got := isFenwickTreeContractInput(path); got != want {
			t.Errorf("isFenwickTreeContractInput(%q) = %v, want %v", path, got, want)
		}
		selected, err := repositoryInputConsumersForChange([]string{path}, packages, "all")
		if err != nil {
			t.Fatal(err)
		}
		wantSelected := map[string]bool{}
		if want {
			wantSelected["dart/fenwick-tree"] = true
			wantSelected["python/fenwick-tree"] = true
		}
		if !reflect.DeepEqual(selected, wantSelected) {
			t.Errorf("change %q selects %v, want %v", path, selected, wantSelected)
		}
	}

	for language, want := range map[string]map[string]bool{
		"dart":   {"dart/fenwick-tree": true},
		"python": {"python/fenwick-tree": true},
	} {
		selected, err := repositoryInputConsumersForChange(
			[]string{"code/specs/fixtures/fenwick-tree-v1/cases.json"}, packages, language,
		)
		if err != nil {
			t.Fatalf("language %s: %v", language, err)
		}
		if !reflect.DeepEqual(selected, want) {
			t.Errorf("language %s selects %v, want %v", language, selected, want)
		}
	}

	missingPython := packages[:1]
	if _, err := repositoryInputConsumersForChange(
		[]string{"code/specs/fixtures/fenwick-tree-v1/cases.json"}, missingPython, "all",
	); err == nil || !strings.Contains(err.Error(), `repository input consumer "python/fenwick-tree"`) {
		t.Errorf("missing Python consumer error = %v", err)
	}
	if selected, err := repositoryInputConsumersForChange(
		[]string{"code/specs/fixtures/fenwick-tree-v1/cases.json"}, missingPython, "dart",
	); err != nil || !reflect.DeepEqual(selected, map[string]bool{"dart/fenwick-tree": true}) {
		t.Errorf("Dart-only selection with Python absent = %v, %v", selected, err)
	}
}

func TestHashFunctionsV1SelectsOnlyRustAndDartPublicConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/hash-functions", Language: "rust"},
		{Name: "dart/hash-functions", Language: "dart"},
		{Name: "rust/hash-analysis", Language: "rust"},
		{Name: "dart/hash-analysis", Language: "dart"},
		{Name: "python/hash-functions", Language: "python"},
	}
	accepted := []string{
		"code/specs/DT17-hash-functions.md",
		"code/specs/DT17b-hash-functions-dart-profile.md",
		"code/specs/fixtures/hash-functions-v1/cases.json",
		"code/specs/fixtures/hash-functions-v1/schema.json",
		"code/specs/fixtures/hash-functions-v1/oracle.py",
		"code/specs/fixtures/hash-functions-v1/generate.py",
	}
	for _, changed := range accepted {
		if !isDt17HashFunctionsInput(changed) {
			t.Errorf("isDt17HashFunctionsInput(%q) = false, want true", changed)
		}
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil {
			t.Fatal(err)
		}
		want := map[string]bool{"rust/hash-functions": true, "dart/hash-functions": true}
		if !reflect.DeepEqual(got, want) {
			t.Errorf("consumers for %q = %v, want %v", changed, got, want)
		}
	}
	for _, changed := range []string{
		"code/specs/DT17-hash-functions.md.bak",
		"code/specs/DT17b-hash-functions-dart-profile.md.extra",
		"code/specs/DT17c-hash-functions.md",
		"code/specs/fixtures/hash-functions-v10/cases.json",
		"code/specs/fixtures/hash-functions-v1x/cases.json",
		"code/specs/fixtures/hash-functions-v1",
		"docs/code/specs/fixtures/hash-functions-v1/cases.json",
		`code\specs\fixtures\hash-functions-v1\cases.json`,
		"code/specs/fixtures/hash-functions-v1/../hash-functions-v10/cases.json",
	} {
		if isDt17HashFunctionsInput(changed) {
			t.Errorf("isDt17HashFunctionsInput(%q) = true, want false", changed)
		}
	}
	for language, want := range map[string]map[string]bool{
		"rust": {"rust/hash-functions": true},
		"dart": {"dart/hash-functions": true},
	} {
		got, err := repositoryInputConsumersForChange([]string{accepted[0]}, packages, language)
		if err != nil {
			t.Fatal(err)
		}
		if !reflect.DeepEqual(got, want) {
			t.Errorf("%s selector returned %v, want %v", language, got, want)
		}
	}
}

func TestHashFunctionsV1GraphExportsExactRepositoryInputs(t *testing.T) {
	root := t.TempDir()
	packages := []discovery.Package{
		{Name: "rust/hash-functions", Language: "rust", Path: filepath.Join(root, "code", "packages", "rust", "hash-functions")},
		{Name: "dart/hash-functions", Language: "dart", Path: filepath.Join(root, "code", "packages", "dart", "hash-functions")},
	}
	for _, pkg := range packages {
		if err := os.MkdirAll(pkg.Path, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	graph, err := buildKilnLegacyGraph(packages, root)
	if err != nil {
		t.Fatal(err)
	}
	dt17 := []string{
		"code/specs/DT17-hash-functions.md",
		"code/specs/DT17b-hash-functions-dart-profile.md",
		"code/specs/fixtures/hash-functions-v1",
	}
	// The Rust crate also checks its committed evidence against its own Warrant spec
	// (CV2-F13, warrantEvidenceRow); the Dart package does not read that spec.
	wants := map[string][]string{
		"rust/hash-functions": append(append([]string{}, dt17...), "code/specs/hash-functions.warrant"),
		"dart/hash-functions": dt17,
	}
	for _, name := range []string{"rust/hash-functions", "dart/hash-functions"} {
		want := wants[name]
		var found *kilnLegacyPackage
		for i := range graph.Packages {
			if graph.Packages[i].Name == name {
				found = &graph.Packages[i]
				break
			}
		}
		if found == nil {
			t.Fatalf("graph omitted %s", name)
		}
		if !reflect.DeepEqual(found.RepoInputs, want) {
			t.Errorf("%s repo_inputs = %v, want %v", name, found.RepoInputs, want)
		}
	}
}

func TestHashMapV1SelectsSwiftAndDartPublicConsumers(t *testing.T) {
	packages := []discovery.Package{
		{Name: "swift/hash-map", Language: "swift"},
		{Name: "dart/hash-map", Language: "dart"},
		{Name: "swift/hash-functions", Language: "swift"},
	}
	accepted := []string{
		"code/specs/DT18-hash-map.md",
		"code/specs/DT18-hash-map-dart-profile.md",
		"code/specs/fixtures/hash-map-v1/cases.json",
		"code/specs/fixtures/hash-map-v1/schema.json",
		"code/specs/fixtures/hash-map-v1/oracle.py",
	}
	for _, changed := range accepted {
		if !isDt18HashMapInput(changed) {
			t.Errorf("isDt18HashMapInput(%q) = false, want true", changed)
		}
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil {
			t.Fatal(err)
		}
		if want := map[string]bool{"swift/hash-map": true, "dart/hash-map": true}; !reflect.DeepEqual(got, want) {
			t.Errorf("consumers for %q = %v, want %v", changed, got, want)
		}
	}
	for _, changed := range []string{
		"code/specs/DT18-hash-map.md.bak",
		"code/specs/DT18-hash-map-dart-profile.md.extra",
		"code/specs/DT18x-hash-map.md",
		"code/specs/fixtures/hash-map-v10/cases.json",
		"code/specs/fixtures/hash-map-v1x/cases.json",
		"code/specs/fixtures/hash-map-v1",
		"code/specs/fixtures/hash-map-v1/cases.json.backup",
		"code/specs/fixtures/hash-map-v1/../hash-map-v10/cases.json",
	} {
		if changed == "code/specs/fixtures/hash-map-v1/cases.json.backup" {
			if !isDt18HashMapInput(changed) {
				t.Errorf("fixture-tree input %q was not selected", changed)
			}
			continue
		}
		if isDt18HashMapInput(changed) {
			t.Errorf("isDt18HashMapInput(%q) = true, want false", changed)
		}
	}
	for language, want := range map[string]map[string]bool{
		"swift": {"swift/hash-map": true},
		"dart":  {"dart/hash-map": true},
	} {
		got, err := repositoryInputConsumersForChange([]string{accepted[0]}, packages, language)
		if err != nil {
			t.Fatal(err)
		}
		if !reflect.DeepEqual(got, want) {
			t.Errorf("%s selector returned %v, want %v", language, got, want)
		}
	}
	if _, err := repositoryInputConsumersForChange([]string{accepted[2]}, packages[1:], "all"); err == nil || !strings.Contains(err.Error(), `repository input consumer "swift/hash-map"`) {
		t.Errorf("missing Swift consumer error = %v", err)
	}
}

func TestHashMapV1GraphExportsExactRepositoryInput(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{Name: "swift/hash-map", Language: "swift", Path: filepath.Join(root, "code", "packages", "swift", "hash-map")}
	if err := os.MkdirAll(pkg.Path, 0o755); err != nil {
		t.Fatal(err)
	}
	graph, err := buildKilnLegacyGraph([]discovery.Package{pkg}, root)
	if err != nil {
		t.Fatal(err)
	}
	for _, node := range graph.Packages {
		if node.Name != pkg.Name {
			continue
		}
		want := []string{"code/specs/DT18-hash-map-dart-profile.md", "code/specs/DT18-hash-map.md", "code/specs/fixtures/hash-map-v1"}
		if !reflect.DeepEqual(node.RepoInputs, want) {
			t.Fatalf("repo_inputs = %v, want %v", node.RepoInputs, want)
		}
		return
	}
	t.Fatal("graph omitted swift/hash-map")
}

func TestDiscoveredSwiftHashMapExportsDT18RepositoryInputs(t *testing.T) {
	root := findRepoRoot("")
	packageRoot := filepath.Join(root, "code", "packages", "swift", "hash-map")
	packages, err := discovery.DiscoverPackages(packageRoot)
	if err != nil {
		t.Fatalf("discover actual Swift hash-map package: %v", err)
	}
	if len(packages) != 1 || packages[0].Name != "swift/hash-map" {
		t.Fatalf("discovered packages = %#v, want actual swift/hash-map package", packages)
	}
	graph, err := buildKilnLegacyGraph(packages, root)
	if err != nil {
		t.Fatalf("export actual Swift hash-map graph node: %v", err)
	}
	for _, node := range graph.Packages {
		if node.Name != "swift/hash-map" {
			continue
		}
		want := []string{"code/specs/DT18-hash-map-dart-profile.md", "code/specs/DT18-hash-map.md", "code/specs/fixtures/hash-map-v1"}
		if !reflect.DeepEqual(node.RepoInputs, want) {
			t.Fatalf("actual swift/hash-map repo_inputs = %v, want %v", node.RepoInputs, want)
		}
		return
	}
	t.Fatal("graph omitted discovered swift/hash-map package")
}

func TestDiscoveredDartHashMapExportsDT18RepositoryInputs(t *testing.T) {
	root := findRepoRoot("")
	packageRoot := filepath.Join(root, "code", "packages", "dart", "hash-map")
	packages, err := discovery.DiscoverPackages(packageRoot)
	if err != nil {
		t.Fatalf("discover actual Dart hash-map package: %v", err)
	}
	if len(packages) != 1 || packages[0].Name != "dart/hash-map" {
		t.Fatalf("discovered packages = %#v, want actual dart/hash-map package", packages)
	}
	graph, err := buildKilnLegacyGraph(packages, root)
	if err != nil {
		t.Fatalf("export actual Dart hash-map graph node: %v", err)
	}
	for _, node := range graph.Packages {
		if node.Name != "dart/hash-map" {
			continue
		}
		want := []string{"code/specs/DT18-hash-map-dart-profile.md", "code/specs/DT18-hash-map.md", "code/specs/fixtures/hash-map-v1"}
		if !reflect.DeepEqual(node.RepoInputs, want) {
			t.Fatalf("actual dart/hash-map repo_inputs = %v, want %v", node.RepoInputs, want)
		}
		return
	}
	t.Fatal("graph omitted discovered dart/hash-map package")
}

func TestVer04pDomainDiagnosticSelectsWarrantGrammar(t *testing.T) {
	packages := []discovery.Package{{Name: "rust/warrant-grammar", Language: "rust"}}
	cases := map[string]bool{
		"code/specs/VER04p-warrant-floats.md":                true,
		"code/scripts/ver04p_domain_witnesses.py":            true,
		"code/scripts/tests/test_ver04p_domain_witnesses.py": true,
		"code/specs/VER04p-warrant-floats.md.bak":            false,
		"code/scripts/tests/test_other_domain_witnesses.py":  false,
		"code/specs/AUDIO-WORKSTATION-BACKLOG.md":            false,
	}
	for path, want := range cases {
		if got := isWarrantGrammarInput(path); got != want {
			t.Errorf("isWarrantGrammarInput(%q) = %v, want %v", path, got, want)
		}
		got, err := repositoryInputConsumersForChange([]string{path}, packages, "all")
		if err != nil {
			t.Fatal(err)
		}
		wantConsumers := map[string]bool{}
		if want {
			wantConsumers["rust/warrant-grammar"] = true
		}
		if !reflect.DeepEqual(got, wantConsumers) {
			t.Errorf("change %q selects %v, want %v", path, got, wantConsumers)
		}
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

func TestFixedPointInputConsumersSelectExactRustReaders(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/fixed-point", Language: "rust"},
		{Name: "rust/game-clock", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "rust/warrant-ir", Language: "rust"},
		{Name: "rust/warrant-grammar", Language: "rust"},
	}
	for _, input := range fixedPointInputPaths {
		selected, err := repositoryInputConsumersForChange([]string{input}, packages, "rust")
		want := map[string]bool{"rust/fixed-point": true}
		switch input {
		case ".cargo/config.toml", "code/packages/rust/.cargo/config.toml", "code/packages/rust/Cargo.toml", "code/packages/rust/Cargo.lock":
			want["rust/game-clock"] = true
			want["rust/ca-assurance-report"] = true
			want["rust/warrant-ir"] = true
		case "code/specs/fixed-point.warrant":
			want["rust/ca-assurance-report"] = true
			want["rust/warrant-ir"] = true
			want["rust/warrant-grammar"] = true
		}
		if err != nil || !reflect.DeepEqual(selected, want) {
			t.Fatalf("Rust input %s selected %v, err %v; want %v", input, selected, err, want)
		}
	}
	for _, consumerName := range []string{"rust/ca-assurance-report", "rust/warrant-ir"} {
		var found bool
		for _, consumer := range repositoryInputConsumers {
			if consumer.name != consumerName {
				continue
			}
			found = true
			for _, treePath := range consumer.treePaths {
				if !containsString(consumer.paths, treePath) || !consumer.matches(treePath+"/new-file.rs") {
					t.Errorf("%s tree registration %q is not covered by its exact selector/export paths", consumerName, treePath)
				}
			}
		}
		if !found {
			t.Errorf("missing fixed-point tree consumer %s", consumerName)
		}
	}
	for _, tc := range []struct {
		input string
		want  map[string]bool
	}{
		{"code/packages/rust/fixed-point/src/lib.rs", map[string]bool{"rust/ca-assurance-report": true, "rust/warrant-ir": true}},
		{"code/packages/rust/fixed-point/evidence/proof-manifest.json", map[string]bool{"rust/ca-assurance-report": true}},
		{"code/packages/rust/fixed-point/evidence.bindings", map[string]bool{"rust/ca-assurance-report": true, "rust/warrant-ir": true}},
		{"code/packages/rust/fixed-point/Cargo.toml", map[string]bool{"rust/ca-assurance-report": true, "rust/warrant-ir": true}},
	} {
		selected, err := repositoryInputConsumersForChange([]string{tc.input}, packages, "rust")
		if err != nil || !reflect.DeepEqual(selected, tc.want) {
			t.Fatalf("fixed-point verifier input %s selected %v, err %v; want %v", tc.input, selected, err, tc.want)
		}
	}
	if !sort.StringsAreSorted(fixedPointInputPaths) {
		t.Fatalf("fixed-point inputs are not byte-sorted: %v", fixedPointInputPaths)
	}
	for _, path := range []string{".cargo/config.toml.bak", "code/packages/rust/Cargo.lock.bak", "code/packages/rust/fixed-point/../../Cargo.toml", "code/specs/fixtures/fixed-point-q16-v10/cases.json", "code/specs/fixed-point.warrant.bak"} {
		if isFixedPointInput(path) {
			t.Errorf("fixed-point selector accepted lookalike %q", path)
		}
	}
}

func TestFixedPointBindingsSelectGoVerifierForAllLanguages(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/fixed-point", Language: "rust"},
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "rust/warrant-ir", Language: "rust"},
		{Name: "go/programs/build-tool", Language: "go"},
	}
	got, err := repositoryInputConsumersForChange(
		[]string{"code/packages/rust/fixed-point/evidence.bindings"}, packages, "all",
	)
	want := map[string]bool{
		"rust/ca-assurance-report": true,
		"rust/warrant-ir":          true,
		"go/programs/build-tool":   true,
	}
	if err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("all-language fixed-point bindings selection = %v, err %v; want %v", got, err, want)
	}
	gotRust, err := repositoryInputConsumersForChange(
		[]string{"code/packages/rust/fixed-point/evidence.bindings"}, packages, "rust",
	)
	wantRust := map[string]bool{
		"rust/ca-assurance-report": true,
		"rust/warrant-ir":          true,
	}
	if err != nil || !reflect.DeepEqual(gotRust, wantRust) {
		t.Fatalf("Rust-only fixed-point bindings selection = %v, err %v; want %v", gotRust, err, wantRust)
	}
}

func TestFuzzV1CorpusAndGoVectorsSelectTheCSharpEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "rust/ct-compare", Language: "rust"},
		{Name: "c/test-bootstrap", Language: "c"},
	}
	for _, change := range []string{
		"code/specs/fixtures/fuzz-v1/cases.json",
		"code/specs/fixtures/fuzz-v1/cases.tfv",
		"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go",
	} {
		got, err := repositoryInputConsumersForChange([]string{change}, packages, "csharp")
		if err != nil {
			t.Fatal(err)
		}
		if want := map[string]bool{"csharp/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
			t.Fatalf("%s: csharp filter: got %v, want %v", change, got, want)
		}
		// Dropping the C# package fails closed: the row is a real consumer.
		if _, err := repositoryInputConsumersForChange([]string{change}, packages[1:], "csharp"); err == nil {
			t.Fatalf("%s with the C# engine package missing must fail closed", change)
		}
	}
	// The C# package's own sources are selected by the package, not by a corpus row.
	for _, other := range []string{
		"code/packages/csharp/test-bootstrap/Fuzz/FuzzEngine.cs",
		"code/packages/csharp/test-bootstrap/tests/CodingAdventures.TestBootstrap.Tests/FuzzReferenceVectorsTests.cs",
		"code/specs/fixtures/fuzz-v10/cases.json",
	} {
		if got, err := repositoryInputConsumersForChange([]string{other}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("%s must not select a corpus row: %v %v", other, got, err)
		}
	}
}

func TestFuzzV1AndGoReferenceVectorsSelectTheRubyEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	for _, input := range []string{
		"code/specs/fixtures/fuzz-v1/cases.json",
		"code/specs/fixtures/fuzz-v1/cases.tfv",
		"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go",
	} {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "ruby")
		if err != nil || !got["ruby/test_bootstrap"] {
			t.Fatalf("%s must select the Ruby engine: %v %v", input, got, err)
		}
		// With the Ruby package missing the selection fails closed.
		if _, err := repositoryInputConsumersForChange([]string{input}, packages[:1], "ruby"); err == nil {
			t.Fatalf("%s with the Ruby engine package missing must fail closed", input)
		}
	}
	// Neighbouring files and the Ruby package's own tests are not these rows.
	for _, other := range []string{
		"code/specs/fixtures/fuzz-v10/cases.json",
		"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go.bak",
		"code/packages/ruby/test_bootstrap/test/test_fuzz_corpus.rb",
		"code/packages/ruby/test_bootstrap/test/test_fuzz_vectors.rb",
	} {
		if got, err := repositoryInputConsumersForChange([]string{other}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("%s must not select the Ruby rows: %v %v", other, got, err)
		}
	}
}

func TestFuzzV1CorpusAndGoVectorsSelectTheCEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "c/test-bootstrap", Language: "c"},
		{Name: "csharp/test-bootstrap", Language: "csharp"},
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "kotlin/test-bootstrap", Language: "kotlin"},
		{Name: "python/test-bootstrap", Language: "python"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	for _, change := range []string{
		"code/specs/fixtures/fuzz-v1/cases.json",
		"code/specs/fixtures/fuzz-v1/cases.tfv",
		"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go",
	} {
		got, err := repositoryInputConsumersForChange([]string{change}, packages, "c")
		if err != nil {
			t.Fatal(err)
		}
		if want := map[string]bool{"c/test-bootstrap": true}; !reflect.DeepEqual(got, want) {
			t.Fatalf("%s: c filter: got %v, want %v", change, got, want)
		}
		// Dropping the C package fails closed: each row is a real consumer.
		if _, err := repositoryInputConsumersForChange([]string{change}, packages[1:], "c"); err == nil {
			t.Fatalf("%s with the C engine package missing must fail closed", change)
		}
	}
	// The C package's own sources and tests are selected by the package, not by a corpus row; neighbouring
	// fixture directories are not this corpus.
	for _, other := range []string{
		"code/packages/c/test-bootstrap/src/fuzz_engine.c",
		"code/packages/c/test-bootstrap/tests/test_fuzz_corpus.c",
		"code/packages/c/test-bootstrap/tests/test_fuzz_vectors.c",
		"code/specs/fixtures/fuzz-v10/cases.json",
	} {
		if got, err := repositoryInputConsumersForChange([]string{other}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("%s must not select a corpus row: %v %v", other, got, err)
		}
	}
}

func TestPbtV1CorpusSelectsTheLuaEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "lua/test_bootstrap", Language: "lua"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "lua")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"lua/test_bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("lua filter: got %v, want %v", got, want)
	}
	// The generated TFV beside the JSON is part of the corpus directory.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/pbt-v1/cases.tfv"}, packages, "lua"); err != nil || !got["lua/test_bootstrap"] {
		t.Fatalf("cases.tfv must select the Lua engine too: %v %v", got, err)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[:3], "lua"); err == nil {
		t.Fatal("a corpus change with the Lua engine package missing must fail closed")
	}
	// The package's own tests are not a corpus change.
	if got, err := repositoryInputConsumersForChange([]string{"code/packages/lua/test_bootstrap/tests/test_corpus_conformance.lua"}, packages, "all"); err != nil || len(got) != 0 {
		t.Fatalf("a Lua test edit must not select anything here: %v %v", got, err)
	}
	// fuzz-v1 is not a Lua input.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/fuzz-v1/cases.json"}, packages, "lua"); err == nil && got["lua/test_bootstrap"] {
		t.Fatalf("fuzz-v1 must not select the Lua engine: %v", got)
	}
}

func TestPbtV1CorpusSelectsTheElixirEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "java/test-bootstrap", Language: "java"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "elixir/test_bootstrap", Language: "elixir"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	corpus := []string{"code/specs/fixtures/pbt-v1/cases.json"}
	got, err := repositoryInputConsumersForChange(corpus, packages, "elixir")
	if err != nil {
		t.Fatal(err)
	}
	if want := map[string]bool{"elixir/test_bootstrap": true}; !reflect.DeepEqual(got, want) {
		t.Fatalf("elixir filter: got %v, want %v", got, want)
	}
	// The generated TFV beside the JSON is part of the corpus directory.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/pbt-v1/cases.tfv"}, packages, "elixir"); err != nil || !got["elixir/test_bootstrap"] {
		t.Fatalf("cases.tfv must select the Elixir engine too: %v %v", got, err)
	}
	if _, err := repositoryInputConsumersForChange(corpus, packages[:3], "elixir"); err == nil {
		t.Fatal("a corpus change with the Elixir engine package missing must fail closed")
	}
	// The package's own tests are not a corpus change.
	if got, err := repositoryInputConsumersForChange([]string{"code/packages/elixir/test_bootstrap/test/corpus_conformance_test.exs"}, packages, "all"); err != nil || len(got) != 0 {
		t.Fatalf("an Elixir test edit must not select anything here: %v %v", got, err)
	}
	// fuzz-v1 is an Elixir input too, through its own row (VER06n), not this one.
	if isPbtV1Corpus("code/specs/fixtures/fuzz-v1/cases.json") {
		t.Fatal("fuzz-v1 must not match the pbt-v1 row")
	}
}

func TestFuzzV1AndGoReferenceVectorsSelectTheElixirEngine(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/test-bootstrap", Language: "go"},
		{Name: "ruby/test_bootstrap", Language: "ruby"},
		{Name: "elixir/test_bootstrap", Language: "elixir"},
		{Name: "rust/ct-compare", Language: "rust"},
	}
	for _, input := range []string{
		"code/specs/fixtures/fuzz-v1/cases.json",
		"code/specs/fixtures/fuzz-v1/cases.tfv",
		"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go",
	} {
		got, err := repositoryInputConsumersForChange([]string{input}, packages, "elixir")
		if err != nil || !reflect.DeepEqual(got, map[string]bool{"elixir/test_bootstrap": true}) {
			t.Fatalf("%s must select exactly the Elixir engine under the elixir filter: %v %v", input, got, err)
		}
		// With the Elixir package missing the selection fails closed.
		if _, err := repositoryInputConsumersForChange([]string{input}, []discovery.Package{packages[0], packages[1], packages[3]}, "elixir"); err == nil {
			t.Fatalf("%s with the Elixir engine package missing must fail closed", input)
		}
	}
	// The pbt-v1 corpus keeps its own row and still selects the Elixir engine.
	if got, err := repositoryInputConsumersForChange([]string{"code/specs/fixtures/pbt-v1/cases.json"}, packages, "elixir"); err != nil || !got["elixir/test_bootstrap"] {
		t.Fatalf("pbt-v1 must still select the Elixir engine: %v %v", got, err)
	}
	// Neighbouring files and the Elixir package's own tests are not these rows.
	for _, other := range []string{
		"code/specs/fixtures/fuzz-v10/cases.json",
		"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go.bak",
		"code/packages/elixir/test_bootstrap/test/fuzz_corpus_test.exs",
		"code/packages/elixir/test_bootstrap/lib/coding_adventures/test_bootstrap/fuzz/corpus.ex",
	} {
		if got, err := repositoryInputConsumersForChange([]string{other}, packages, "all"); err != nil || len(got) != 0 {
			t.Fatalf("%s must not select the Elixir rows: %v %v", other, got, err)
		}
	}
}
