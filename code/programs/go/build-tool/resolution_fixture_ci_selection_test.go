package main

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

const resolutionFixturePrefix = "code/specs/fixtures/build-tool-v1/cases/"

// This table is deliberately independent of production's routing table. A
// fixture-only edit must select its actual native readers, not every front.
var resolutionExpectedReaders = map[string]string{
	"build-deps-comment": "GHB",
	"dart-field-aware":   "GHY", "dotnet-cross-language-field-aware": "GHY", "haskell-field-aware": "GHY",
	"dotnet-csharp-field-aware": "GHY", "dotnet-fsharp-field-aware": "GHY",
	"gradle-java-field-aware": "GHY", "gradle-kotlin-field-aware": "GHY",
	"ecosystem-scoped-aliases": "GY",
	"elixir-field-aware":       "GH", "go-field-aware": "GH", "perl-field-aware": "GH",
	"ruby-field-aware": "GH", "swift-field-aware": "GH", "typescript-field-aware": "GH",
	"elixir-program-package": "BR", "elixir-self-edge": "R",
	"lua-cycle": "H", "lua-field-aware": "H", "lua-program-package": "H",
	"python-diamond": "H", "python-field-aware": "H", "rust-field-aware": "H",
	"lua-utf8": "GHLPBRST", "lua-invalid-utf8": "GHLPBRST",
	"ocaml-field-aware": "G",
}

var resolutionReaderRoots = map[rune]struct{ name, path, language string }{
	'G': {"go/programs/build-tool", "code/programs/go/build-tool", "go"},
	'H': {"haskell/programs/build-tool", "code/programs/haskell/build-tool", "haskell"},
	'L': {"lua/programs/build-tool", "code/programs/lua/build-tool", "lua"},
	'P': {"perl/programs/build-tool", "code/programs/perl/build-tool", "perl"},
	'Y': {"python/programs/build-tool", "code/programs/python/build-tool", "python"},
	'B': {"ruby/programs/build-tool", "code/programs/ruby/build-tool", "ruby"},
	'R': {"rust/programs/build-tool", "code/programs/rust/build-tool", "rust"},
	'S': {"swift/programs/build-tool", "code/programs/swift/build-tool", "swift"},
	'T': {"typescript/programs/build-tool", "code/programs/typescript/build-tool", "typescript"},
}

func resolutionTestPath(stem string) string {
	return resolutionFixturePrefix + "resolution-" + stem + ".json"
}

func resolutionTestNames(stem string) []string {
	names := []string{}
	for _, code := range resolutionExpectedReaders[stem] {
		if reader, ok := resolutionReaderRoots[code]; ok {
			names = append(names, reader.name)
		}
	}
	sort.Strings(names)
	return names
}

func resolutionTestPackages(root string) []discovery.Package {
	packages := []discovery.Package{}
	for _, reader := range resolutionReaderRoots {
		packages = append(packages, discovery.Package{
			Name: reader.name, Path: filepath.Join(root, filepath.FromSlash(reader.path)), Language: reader.language,
		})
	}
	return append(packages, discovery.Package{
		Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust",
	})
}

func TestResolutionFixtureExactReadersAndPlatforms(t *testing.T) {
	root := t.TempDir()
	packages := resolutionTestPackages(root)
	for stem := range resolutionExpectedReaders {
		want := resolutionTestNames(stem)
		if len(want) == 0 {
			t.Fatalf("%s has no native reader", stem)
		}
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{resolutionTestPath(stem)}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
				t.Fatalf("%s %s: got %v, err %v; want %v", goos, stem, sortedChangedRoots(got), err, want)
			}
		}
	}
	for _, path := range []string{
		resolutionFixturePrefix + "resolution-.json", resolutionFixturePrefix + "resolution-lua-utf8.JSON",
		resolutionFixturePrefix + "resolution-lua-utf8.json.bak", resolutionFixturePrefix + "Resolution-lua-utf8.json",
		resolutionFixturePrefix + "nested/resolution-lua-utf8.json",
		resolutionFixturePrefix + "resolution-nested/lua-utf8.json",
		resolutionFixturePrefix + "resolution-lua-utf8\\child.json",
		"code/specs/fixtures/build-tool-v1/other/resolution-lua-utf8.json",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q: %v, %v", path, got, err)
		}
	}
	for _, changed := range [][]string{nil, {}} {
		got, err := changedPackageRootsForPlatform(changed, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("empty diff: %v, %v", got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{resolutionTestPath("ocaml-field-aware"), "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux")
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), []string{"go/programs/build-tool", "rust/extra"}) {
		t.Fatalf("fixture plus ordinary edit: %v, %v", sortedChangedRoots(got), err)
	}
}

func TestResolutionFixtureUnknownFailsClosedAndLanguageFilters(t *testing.T) {
	root := t.TempDir()
	packages := resolutionTestPackages(root)
	unknown := resolutionTestPath("future-native-case")
	for _, language := range []string{"all", "ruby"} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{unknown, "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux", language)
		if err == nil || got != nil {
			t.Fatalf("unknown %s returned %v, %v", language, got, err)
		}
	}
	for stem, codes := range resolutionExpectedReaders {
		for code, reader := range resolutionReaderRoots {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{resolutionTestPath(stem)}, packages, root, "linux", reader.language)
			want := []string{}
			if strings.ContainsRune(codes, code) {
				want = []string{reader.name}
			}
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) && !(len(want) == 0 && len(got) == 0) {
				t.Fatalf("%s %s: got %v, err %v; want %v", stem, reader.language, got, err, want)
			}
			if len(want) == 0 {
				continue
			}
			missing := make([]discovery.Package, 0, len(packages)-1)
			for _, pkg := range packages {
				if pkg.Name != reader.name {
					missing = append(missing, pkg)
				}
			}
			for _, filter := range []string{"all", reader.language} {
				got, err = changedPackageRootsForPlatformAndLanguage([]string{resolutionTestPath(stem)}, missing, root, "linux", filter)
				if err == nil || !strings.Contains(err.Error(), reader.name) || got != nil {
					t.Fatalf("missing %s %s %s: %v, %v", stem, reader.name, filter, got, err)
				}
			}
		}
	}
}

func TestResolutionFixturePlanIsUnforcedAndAtomic(t *testing.T) {
	root := t.TempDir()
	packages := resolutionTestPackages(root)
	planPath := filepath.Join(root, "plan.json")
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	for _, stem := range []string{"ocaml-field-aware", "elixir-self-edge", "lua-utf8"} {
		path := resolutionTestPath(stem)
		changed, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil {
			t.Fatal(err)
		}
		affected := affectedForGraph(graph, changed, nil, false)
		if code := emitBuildPlan(packages, graph, affected, changed, []string{path}, false, nil, "origin/main", root, planPath, false, 0, false, "", "all"); code != 0 {
			t.Fatalf("%s plan exit %d", stem, code)
		}
		data, err := os.ReadFile(planPath)
		if err != nil {
			t.Fatal(err)
		}
		var built plan.BuildPlan
		if err := json.Unmarshal(data, &built); err != nil {
			t.Fatal(err)
		}
		want := resolutionTestNames(stem)
		if built.Force || !reflect.DeepEqual(sortedStrings(built.AffectedPackages), want) {
			t.Fatalf("%s affected %v, force %t; want %v", stem, built.AffectedPackages, built.Force, want)
		}
		for _, goos := range []string{"linux", "darwin", "windows"} {
			if got := sortedStrings(built.StateForPlatform(goos).AffectedPackages); !reflect.DeepEqual(got, want) {
				t.Fatalf("%s %s affected %v; want %v", stem, goos, got, want)
			}
		}
		for code, reader := range resolutionReaderRoots {
			if built.LanguagesNeeded[reader.language] != strings.ContainsRune(resolutionExpectedReaders[stem], code) {
				t.Fatalf("%s toolchain %s: %v", stem, reader.language, built.LanguagesNeeded)
			}
		}
	}
	missing := []discovery.Package{}
	for _, pkg := range packages {
		if pkg.Name != "go/programs/build-tool" {
			missing = append(missing, pkg)
		}
	}
	missingGraph, err := resolver.ResolveDependencies(missing)
	if err != nil {
		t.Fatal(err)
	}
	atomicPlan := filepath.Join(root, "missing-plan.json")
	if code := emitBuildPlan(missing, missingGraph, map[string]bool{}, map[string]bool{}, []string{resolutionTestPath("ocaml-field-aware")}, false, nil, "origin/main", root, atomicPlan, false, 0, false, "", "all"); code != 1 {
		t.Fatalf("missing native reader plan exit %d", code)
	}
	if _, err := os.Stat(atomicPlan); !os.IsNotExist(err) {
		t.Fatalf("partial plan: %v", err)
	}
}

func TestResolutionFixtureCorpusAndNativeReaderDrift(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	caseDir := filepath.Join(root, "code", "specs", "fixtures", "build-tool-v1", "cases")
	entries, err := os.ReadDir(caseDir)
	if err != nil {
		t.Fatal(err)
	}
	actualCases := []string{}
	for _, entry := range entries {
		name := entry.Name()
		if entry.IsDir() || !strings.HasPrefix(name, "resolution-") || !strings.HasSuffix(name, ".json") {
			continue
		}
		stem := strings.TrimSuffix(strings.TrimPrefix(name, "resolution-"), ".json")
		actualCases = append(actualCases, stem)
		data, err := os.ReadFile(filepath.Join(caseDir, name))
		if err != nil {
			t.Fatal(err)
		}
		var envelope struct {
			Domain string `json:"domain"`
		}
		if err := json.Unmarshal(data, &envelope); err != nil || envelope.Domain != "resolution" {
			t.Fatalf("%s domain %q, error %v", name, envelope.Domain, err)
		}
	}
	expectedCases := make([]string, 0, len(resolutionExpectedReaders))
	for stem := range resolutionExpectedReaders {
		expectedCases = append(expectedCases, stem)
	}
	sort.Strings(actualCases)
	sort.Strings(expectedCases)
	if !reflect.DeepEqual(actualCases, expectedCases) || len(actualCases) != 26 {
		t.Fatalf("checked resolution cases %v differ from reader relation %v", actualCases, expectedCases)
	}
	// These native test sources are independent evidence for the routing map.
	// Haskell explicitly lists its dynamically assembled Gradle/.NET names
	// elsewhere in the same test, and Rust has tests inside src/resolver.rs.
	readerSources := map[rune][]string{
		'G': {"go/build-tool/internal/resolver/resolver_test.go"},
		'H': {"haskell/build-tool/test/ResolutionUtf8Spec.hs"},
		'L': {"lua/build-tool/tests/test_resolution_utf8.lua"},
		'P': {"perl/build-tool/t/13-resolution-utf8.t"},
		'Y': {"python/build-tool/tests/test_resolver.py"},
		'B': {"ruby/build-tool/test/test_resolution_utf8.rb", "ruby/build-tool/test/test_identity_registry.rb"},
		'R': {"rust/build-tool/src/resolver.rs", "rust/build-tool/tests/self_edge_cli.rs", "rust/build-tool/tests/rockspec_utf8_cli.rs"},
		'S': {"swift/build-tool/Tests/BuildToolCoreTests/ResolverTests.swift"},
		'T': {"typescript/build-tool/tests/resolver.test.ts"},
	}
	for code, sources := range readerSources {
		contents := []byte{}
		for _, source := range sources {
			data, err := os.ReadFile(filepath.Join(root, "code", "programs", filepath.FromSlash(source)))
			if err != nil {
				t.Fatal(err)
			}
			contents = append(contents, data...)
		}
		for _, stem := range expectedCases {
			found := bytes.Contains(contents, []byte("resolution-"+stem+".json"))
			// Haskell constructs four filenames from a bounded language loop.
			// Both the template and its language lists must remain present.
			if code == 'H' && (stem == "gradle-java-field-aware" || stem == "gradle-kotlin-field-aware") {
				found = bytes.Contains(contents, []byte("resolution-gradle-")) && bytes.Contains(contents, []byte("[\"java\", \"kotlin\"]"))
			}
			if code == 'H' && (stem == "dotnet-csharp-field-aware" || stem == "dotnet-fsharp-field-aware") {
				found = bytes.Contains(contents, []byte("resolution-dotnet-")) && bytes.Contains(contents, []byte("[\"csharp\", \"fsharp\"]"))
			}
			want := strings.ContainsRune(resolutionExpectedReaders[stem], code)
			if found != want {
				t.Errorf("native reader %c case %s: source reference %t, route %t", code, stem, found, want)
			}
		}
	}
}

func TestResolutionFixtureRenameSelectsDeletedSource(t *testing.T) {
	root := t.TempDir()
	path := resolutionTestPath("ocaml-field-aware")
	fixture := filepath.Join(root, filepath.FromSlash(path))
	if err := os.MkdirAll(filepath.Dir(fixture), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(fixture, []byte("fixture\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	git := func(args ...string) {
		t.Helper()
		cmd := exec.Command("git", args...)
		cmd.Dir = root
		if output, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, output)
		}
	}
	git("init", "-q")
	git("-c", "user.name=Fixture Test", "-c", "user.email=fixture@example.invalid", "add", ".")
	git("-c", "user.name=Fixture Test", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "baseline")
	git("branch", "base")
	if err := os.Rename(fixture, filepath.Join(filepath.Dir(fixture), "renamed.json")); err != nil {
		t.Fatal(err)
	}
	git("add", "-A")
	git("-c", "user.name=Fixture Test", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "rename")
	changed := gitdiff.GetChangedFiles(root, "base")
	if !containsPath(changed, path) {
		t.Fatalf("rename omitted deleted source: %v", changed)
	}
	got, err := changedPackageRootsForPlatform(changed, resolutionTestPackages(root), root, "linux")
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), resolutionTestNames("ocaml-field-aware")) {
		t.Fatalf("renamed source selected %v, error %v", sortedChangedRoots(got), err)
	}
}
