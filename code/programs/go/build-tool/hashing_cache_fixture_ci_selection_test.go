package main

import (
	"bytes"
	"encoding/json"
	"io/fs"
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

// This test relation is independent of the production selector. In
// particular, a direct fixture read is not a claim that the front implements
// the complete cache-decision oracle: several native suites assert digests.
var hashingCacheExpectedReaders = map[string]string{
	"corrupt":                           "CFLYT",
	"dependency-change-after":           "CFL",
	"dependency-order-before":           "CFL",
	"failed-prior-record":               "CFL",
	"hit":                               "CFLY",
	"local-boundary-union":              "CFL",
	"missing":                           "CFLGPYBS",
	"shared-input-conduit-after":        "CFL",
	"shared-input-conduit-before":       "CFL",
	"shared-input-sha256-native-after":  "CFL",
	"shared-input-sha256-native-before": "CFL",
}

var hashingCacheTestRoots = map[rune]struct{ name, path, language string }{
	'C': {"dotnet/programs/build-tool-csharp", "code/programs/dotnet/build-tool-csharp", "csharp"},
	'F': {"dotnet/programs/build-tool-fsharp", "code/programs/dotnet/build-tool-fsharp", "fsharp"},
	'L': {"lua/programs/build-tool", "code/programs/lua/build-tool", "lua"},
	'G': {"go/programs/build-tool", "code/programs/go/build-tool", "go"},
	'P': {"perl/programs/build-tool", "code/programs/perl/build-tool", "perl"},
	'Y': {"python/programs/build-tool", "code/programs/python/build-tool", "python"},
	'B': {"ruby/programs/build-tool", "code/programs/ruby/build-tool", "ruby"},
	'S': {"swift/programs/build-tool", "code/programs/swift/build-tool", "swift"},
	'T': {"typescript/programs/build-tool", "code/programs/typescript/build-tool", "typescript"},
}

const hashingCacheTestPrefix = "code/specs/fixtures/build-tool-v1/cases/hashing-cache-"

func hashingCacheTestPath(stem string) string { return hashingCacheTestPrefix + stem + ".json" }

func hashingCacheTestPackages(root string) []discovery.Package {
	packages := make([]discovery.Package, 0, len(hashingCacheTestRoots)+1)
	for _, reader := range hashingCacheTestRoots {
		packages = append(packages, discovery.Package{
			Name: reader.name, Path: filepath.Join(root, filepath.FromSlash(reader.path)), Language: reader.language,
		})
	}
	return append(packages, discovery.Package{
		Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust",
	})
}

func hashingCacheTestNames(stem string) []string {
	names := []string{}
	for _, code := range hashingCacheExpectedReaders[stem] {
		names = append(names, hashingCacheTestRoots[code].name)
	}
	sort.Strings(names)
	return names
}

func TestHashingCacheFixtureExactReadersAndPlatforms(t *testing.T) {
	root := t.TempDir()
	packages := hashingCacheTestPackages(root)
	for stem := range hashingCacheExpectedReaders {
		want := hashingCacheTestNames(stem)
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{hashingCacheTestPath(stem)}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
				t.Fatalf("%s %s: got %v, err %v; want %v", goos, stem, sortedChangedRoots(got), err, want)
			}
		}
	}
	for _, path := range []string{
		hashingCacheTestPrefix + ".json", hashingCacheTestPrefix + "missing.JSON",
		hashingCacheTestPrefix + "missing.json.bak", hashingCacheTestPrefix + "missing\\child.json",
		"code/specs/fixtures/build-tool-v1/cases/nested/hashing-cache-missing.json",
		"code/specs/fixtures/build-tool-v1/cases/hashing-cache-nested/missing.json",
		"code/specs/fixtures/build-tool-v1/other/hashing-cache-missing.json",
		"code/specs/fixtures/build-tool-v1/cases/Hashing-cache-missing.json",
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
	got, err := changedPackageRootsForPlatform([]string{hashingCacheTestPath("hit"), "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux")
	want := append(hashingCacheTestNames("hit"), "rust/extra")
	sort.Strings(want)
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
		t.Fatalf("fixture plus ordinary edit: %v, %v; want %v", sortedChangedRoots(got), err, want)
	}
}

func TestHashingCacheFixtureUnknownLanguageAndMissingReader(t *testing.T) {
	root := t.TempDir()
	packages := hashingCacheTestPackages(root)
	for _, language := range []string{"all", "ruby"} {
		got, err := changedPackageRootsForPlatformAndLanguage(
			[]string{hashingCacheTestPath("new-case"), "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux", language,
		)
		if err == nil || got != nil {
			t.Fatalf("unknown case %s: got %v, error %v", language, got, err)
		}
	}
	for stem, codes := range hashingCacheExpectedReaders {
		for code, reader := range hashingCacheTestRoots {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{hashingCacheTestPath(stem)}, packages, root, "linux", reader.language)
			want := []string{}
			if strings.ContainsRune(codes, code) {
				want = []string{reader.name}
			}
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) && !(len(want) == 0 && len(got) == 0) {
				t.Fatalf("%s %s: got %v, error %v; want %v", stem, reader.language, got, err, want)
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
			got, err = changedPackageRootsForPlatformAndLanguage([]string{hashingCacheTestPath(stem)}, missing, root, "linux", reader.language)
			if err == nil || !strings.Contains(err.Error(), reader.name) || got != nil {
				t.Fatalf("missing %s %s: got %v, error %v", stem, reader.name, got, err)
			}
		}
	}
}

func TestHashingCacheFixtureCheckedCorpus(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	caseDir := filepath.Join(root, "code", "specs", "fixtures", "build-tool-v1", "cases")
	entries, err := os.ReadDir(caseDir)
	if err != nil {
		t.Fatal(err)
	}
	actual := []string{}
	for _, entry := range entries {
		name := entry.Name()
		if entry.IsDir() || !strings.HasPrefix(name, "hashing-cache-") || !strings.HasSuffix(name, ".json") {
			continue
		}
		stem := strings.TrimSuffix(strings.TrimPrefix(name, "hashing-cache-"), ".json")
		actual = append(actual, stem)
		data, err := os.ReadFile(filepath.Join(caseDir, name))
		if err != nil {
			t.Fatal(err)
		}
		var envelope struct {
			Domain string `json:"domain"`
		}
		if err := json.Unmarshal(data, &envelope); err != nil || envelope.Domain != "hashing_cache" {
			t.Fatalf("%s domain %q, error %v", name, envelope.Domain, err)
		}
	}
	expected := make([]string, 0, len(hashingCacheExpectedReaders))
	for stem := range hashingCacheExpectedReaders {
		expected = append(expected, stem)
	}
	sort.Strings(actual)
	sort.Strings(expected)
	if !reflect.DeepEqual(actual, expected) || len(actual) != 11 {
		t.Fatalf("checked hashing-cache cases %v differ from native-reader relation %v", actual, expected)
	}
}

func TestHashingCacheFixtureNativeSourceReferenceDrift(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	// C#/F# enumerate the complete glob, Python constructs exactly three
	// names, and Lua lists the complete roster. The other fronts use literals.
	readerSources := map[rune]string{
		'C': "dotnet/build-tool-csharp/tests/BuildTool.CSharp.Tests/HasherConformanceTests.cs",
		'F': "dotnet/build-tool-fsharp/tests/BuildTool.FSharp.Tests/HasherConformanceTests.fs",
		'L': "lua/build-tool/tests/test_dependency_hashing.lua",
		'G': "go/build-tool/internal/hasher/hasher_test.go",
		'P': "perl/build-tool/t/16-source-registry.t",
		'Y': "python/build-tool/tests/test_hasher.py",
		'B': "ruby/build-tool/test/test_hasher.rb",
		'S': "swift/build-tool/Tests/BuildToolCoreTests/HasherTests.swift",
		'T': "typescript/build-tool/tests/hasher.test.ts",
	}
	for code, source := range readerSources {
		data, err := os.ReadFile(filepath.Join(root, "code", "programs", filepath.FromSlash(source)))
		if err != nil {
			t.Fatal(err)
		}
		if code == 'C' || code == 'F' {
			if !bytes.Contains(data, []byte("hashing-cache-*.json")) {
				t.Errorf("%c lost its complete-case glob", code)
			}
		}
		if code == 'Y' && (!bytes.Contains(data, []byte(`f"hashing-cache-{state}.json"`)) ||
			!bytes.Contains(data, []byte(`for state in ("missing", "hit", "corrupt")`))) {
			t.Error("Python hashing-cache dynamic case roster drifted")
		}
		for stem, codes := range hashingCacheExpectedReaders {
			found := bytes.Contains(data, []byte("hashing-cache-"+stem+".json"))
			if code == 'C' || code == 'F' || code == 'Y' && (stem == "missing" || stem == "hit" || stem == "corrupt") {
				found = true
			}
			want := strings.ContainsRune(codes, code)
			if found != want {
				t.Errorf("native reader %c case %s: source reference %t, route %t", code, stem, found, want)
			}
		}
	}
	// Catch a future literal consumer in another test file, not just the
	// pinned source above. Dynamic readers are checked by their exact roster.
	for code, reader := range hashingCacheTestRoots {
		front := filepath.Join(root, filepath.FromSlash(reader.path))
		found := map[string]bool{}
		err := filepath.WalkDir(front, func(path string, entry fs.DirEntry, walkErr error) error {
			if walkErr != nil {
				return walkErr
			}
			if entry.IsDir() {
				switch entry.Name() {
				case "node_modules", "target", ".build", "dist-newstyle", "coverage", ".venv":
					return filepath.SkipDir
				}
				return nil
			}
			rel, err := filepath.Rel(front, path)
			if err != nil {
				return err
			}
			slash := "/" + filepath.ToSlash(rel)
			lower := strings.ToLower(slash)
			if !strings.Contains(lower, "/test/") && !strings.Contains(lower, "/tests/") &&
				!strings.Contains(lower, "/t/") && !strings.HasSuffix(entry.Name(), "_test.go") {
				return nil
			}
			data, err := os.ReadFile(path)
			if err != nil {
				return err
			}
			for stem := range hashingCacheExpectedReaders {
				if bytes.Contains(data, []byte("hashing-cache-"+stem+".json")) {
					found[stem] = true
				}
			}
			return nil
		})
		if err != nil {
			t.Fatal(err)
		}
		for stem, codes := range hashingCacheExpectedReaders {
			literal := found[stem]
			if code == 'C' || code == 'F' || code == 'Y' && (stem == "missing" || stem == "hit" || stem == "corrupt") {
				literal = true
			}
			if literal != strings.ContainsRune(codes, code) {
				t.Errorf("%s discovered case %s = %t, route = %t", reader.name, stem, literal, strings.ContainsRune(codes, code))
			}
		}
	}
}

func TestHashingCacheFixturePlanUnforcedToolchainsAndAtomicity(t *testing.T) {
	root := t.TempDir()
	packages := hashingCacheTestPackages(root)
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	for _, stem := range []string{"missing", "hit", "corrupt", "shared-input-conduit-before"} {
		path := hashingCacheTestPath(stem)
		changed, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil {
			t.Fatal(err)
		}
		affected := affectedForGraph(graph, changed, nil, false)
		planPath := filepath.Join(root, stem+"-plan.json")
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
		want := hashingCacheTestNames(stem)
		if built.Force || !reflect.DeepEqual(sortedStrings(built.AffectedPackages), want) {
			t.Fatalf("%s affected %v, force %t; want %v", stem, built.AffectedPackages, built.Force, want)
		}
		for _, goos := range []string{"linux", "darwin", "windows"} {
			if got := sortedStrings(built.StateForPlatform(goos).AffectedPackages); !reflect.DeepEqual(got, want) {
				t.Fatalf("%s %s affected %v; want %v", stem, goos, got, want)
			}
		}
		for code, reader := range hashingCacheTestRoots {
			if code == 'C' || code == 'F' {
				continue // Both language fronts require the one dotnet toolchain.
			}
			if built.LanguagesNeeded[reader.language] != strings.ContainsRune(hashingCacheExpectedReaders[stem], code) {
				t.Fatalf("%s toolchain %s: %v", stem, reader.language, built.LanguagesNeeded)
			}
		}
		if !built.LanguagesNeeded["dotnet"] {
			t.Fatalf("%s must enable the C#/F# dotnet toolchain: %v", stem, built.LanguagesNeeded)
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
	missingPlan := filepath.Join(root, "missing-reader-plan.json")
	if code := emitBuildPlan(missing, missingGraph, map[string]bool{}, map[string]bool{}, []string{hashingCacheTestPath("missing")}, false, nil, "origin/main", root, missingPlan, false, 0, false, "", "all"); code != 1 {
		t.Fatalf("missing native reader plan exit %d", code)
	}
	if _, err := os.Stat(missingPlan); !os.IsNotExist(err) {
		t.Fatalf("partial missing-reader plan: %v", err)
	}
	unknownPlan := filepath.Join(root, "unknown-case-plan.json")
	if code := emitBuildPlan(packages, graph, map[string]bool{}, map[string]bool{}, []string{hashingCacheTestPath("future-case")}, false, nil, "origin/main", root, unknownPlan, false, 0, false, "", "all"); code != 1 {
		t.Fatalf("unknown case plan exit %d", code)
	}
	if _, err := os.Stat(unknownPlan); !os.IsNotExist(err) {
		t.Fatalf("partial unknown-case plan: %v", err)
	}
}

func TestHashingCacheFixtureRenameSelectsDeletedSource(t *testing.T) {
	root := t.TempDir()
	path := hashingCacheTestPath("hit")
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
	got, err := changedPackageRootsForPlatform(changed, hashingCacheTestPackages(root), root, "linux")
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), hashingCacheTestNames("hit")) {
		t.Fatalf("renamed source selected %v, error %v", sortedChangedRoots(got), err)
	}
}
