package main

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"regexp"
	"sort"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/gitdiff"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/plan"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

// This independent table records actual native validator fixture reads, not
// merely which languages implement the validation domain.
var validationExpectedReaders = map[string]string{
	"clean-build": "", "clean-full": "", "dependency-oracles": "",
	"identity-manifest-ambiguous": "", "missing-build": "", "path-unsafe": "",
	"starlark-declarations-invalid": "", "toolchain-unsupported": "",
	"lua-windows-sibling-parity-absent": "G",
	"orphan-crates-clean":               "CFEHLPYBRST", "orphan-crates-unlisted": "CFEHLPYBRST",
	"orphan-exemptions-invalid": "CFEHLPYBRST", "orphan-exemptions-stale": "CFEHLPYBRST",
	"orphan-package-root-exemptions-invalid": "G", "orphan-package-root-exemptions-stale": "G",
	"orphan-package-roots-clean": "G", "orphan-package-roots-unlisted": "G",
	"tracked-artifacts-aliases": "CFEHLPYBRST", "tracked-artifacts-clean": "CFEHLPYBRST",
	"tracked-artifacts-forbidden": "CFEHLPYBRST", "tracked-artifacts-invalid": "CFEHLPYBRST",
	"tracked-artifacts-unicode-boundaries": "CFEHLPYBRST",
}

var validationTestRoots = map[rune]struct{ name, path, language string }{
	'C': {"dotnet/programs/build-tool-csharp", "code/programs/dotnet/build-tool-csharp", "csharp"},
	'F': {"dotnet/programs/build-tool-fsharp", "code/programs/dotnet/build-tool-fsharp", "fsharp"},
	'E': {"elixir/programs/build-tool", "code/programs/elixir/build-tool", "elixir"},
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

const validationTestPrefix = "code/specs/fixtures/build-tool-v1/cases/validation-"

func validationTestPath(stem string) string { return validationTestPrefix + stem + ".json" }

func validationTestPackages(root string) []discovery.Package {
	packages := make([]discovery.Package, 0, len(validationTestRoots)+1)
	for _, reader := range validationTestRoots {
		packages = append(packages, discovery.Package{Name: reader.name, Path: filepath.Join(root, filepath.FromSlash(reader.path)), Language: reader.language})
	}
	return append(packages, discovery.Package{Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust"})
}

func validationTestNames(stem string) []string {
	names := []string{}
	for _, code := range validationExpectedReaders[stem] {
		names = append(names, validationTestRoots[code].name)
	}
	sort.Strings(names)
	return names
}

func TestValidationFixtureExactReadersAndPlatforms(t *testing.T) {
	root := t.TempDir()
	packages := validationTestPackages(root)
	for stem := range validationExpectedReaders {
		want := validationTestNames(stem)
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{validationTestPath(stem)}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) && !(len(got) == 0 && len(want) == 0) {
				t.Fatalf("%s %s: got %v, err %v; want %v", goos, stem, sortedChangedRoots(got), err, want)
			}
		}
	}
	for _, path := range []string{
		validationTestPrefix + ".json", validationTestPrefix + "clean-build.JSON",
		validationTestPrefix + "clean-build.json.bak", validationTestPrefix + "clean-build\\child.json",
		"code/specs/fixtures/build-tool-v1/cases/nested/validation-clean-build.json",
		"code/specs/fixtures/build-tool-v1/cases/validation-nested/clean-build.json",
		"code/specs/fixtures/build-tool-v1/other/validation-clean-build.json",
		"code/specs/fixtures/build-tool-v1/cases/Validation-clean-build.json",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q: %v, %v", path, got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{validationTestPath("orphan-crates-clean"), "code/packages/rust/extra/src/lib.rs"}, packages, root, "linux")
	want := append(validationTestNames("orphan-crates-clean"), "rust/extra")
	sort.Strings(want)
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
		t.Fatalf("fixture plus ordinary edit: %v, %v; want %v", sortedChangedRoots(got), err, want)
	}
}

func TestValidationFixtureUnknownLanguageAndMissingReader(t *testing.T) {
	root := t.TempDir()
	packages := validationTestPackages(root)
	for _, language := range []string{"all", "ruby"} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{validationTestPath("future-case")}, packages, root, "linux", language)
		if err == nil || got != nil {
			t.Fatalf("unknown case %s: got %v, error %v", language, got, err)
		}
	}
	for stem, codes := range validationExpectedReaders {
		for code, reader := range validationTestRoots {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{validationTestPath(stem)}, packages, root, "linux", reader.language)
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
			got, err = changedPackageRootsForPlatformAndLanguage([]string{validationTestPath(stem)}, missing, root, "linux", reader.language)
			if err == nil || !strings.Contains(err.Error(), reader.name) || got != nil {
				t.Fatalf("missing %s %s: got %v, error %v", stem, reader.name, got, err)
			}
		}
	}
}

func TestValidationFixtureCheckedCorpus(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	caseDir := filepath.Join(root, "code", "specs", "fixtures", "build-tool-v1", "cases")
	entries, err := os.ReadDir(caseDir)
	if err != nil {
		t.Fatal(err)
	}
	actual := []string{}
	for _, entry := range entries {
		name := entry.Name()
		if entry.IsDir() || !strings.HasPrefix(name, "validation-") || !strings.HasSuffix(name, ".json") {
			continue
		}
		stem := strings.TrimSuffix(strings.TrimPrefix(name, "validation-"), ".json")
		actual = append(actual, stem)
		data, err := os.ReadFile(filepath.Join(caseDir, name))
		if err != nil {
			t.Fatal(err)
		}
		var envelope struct {
			Domain string `json:"domain"`
		}
		if err := json.Unmarshal(data, &envelope); err != nil || envelope.Domain != "validation" {
			t.Fatalf("%s domain %q, error %v", name, envelope.Domain, err)
		}
	}
	expected := make([]string, 0, len(validationExpectedReaders))
	for stem := range validationExpectedReaders {
		expected = append(expected, stem)
	}
	sort.Strings(actual)
	sort.Strings(expected)
	if !reflect.DeepEqual(actual, expected) || len(actual) != 22 {
		t.Fatalf("checked validation cases %v differ from native-reader relation %v", actual, expected)
	}
}

func TestValidationFixtureNativeSourceReferenceDrift(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	// These are test sources, not implementation files. Go has two distinct
	// validation test modules; Perl constructs its case filenames from qw lists.
	readerSources := map[rune][]string{
		'C': {"dotnet/build-tool-csharp/tests/BuildTool.CSharp.Tests/BuildToolTests.cs"},
		'F': {"dotnet/build-tool-fsharp/tests/BuildTool.FSharp.Tests/BuildToolTests.fs"},
		'E': {"elixir/build-tool/test/validator_test.exs"},
		'G': {"go/build-tool/internal/validator/package_roots_test.go", "go/build-tool/internal/validator/validator_test.go"},
		'H': {"haskell/build-tool/test/BuildToolSpec.hs"},
		'L': {"lua/build-tool/tests/test_validator.lua"},
		'P': {"perl/build-tool/t/11-validator.t"},
		'Y': {"python/build-tool/tests/test_validator.py"},
		'B': {"ruby/build-tool/test/test_validator.rb"},
		'R': {"rust/build-tool/src/validator.rs"},
		'S': {"swift/build-tool/Tests/BuildToolCoreTests/ValidatorTests.swift"},
		'T': {"typescript/build-tool/tests/validator.test.ts"},
	}
	perlDynamic := regexp.MustCompile(`(?s)for my \$name \(qw\(([^)]*)\)\) \{.*?validation-(orphan|tracked-artifacts)-\$name\.json`)
	for code, sources := range readerSources {
		all := []byte{}
		for _, source := range sources {
			data, err := os.ReadFile(filepath.Join(root, "code", "programs", filepath.FromSlash(source)))
			if err != nil {
				t.Fatal(err)
			}
			all = append(all, data...)
		}
		found := map[string]bool{}
		if code == 'P' {
			for _, match := range perlDynamic.FindAllSubmatch(all, -1) {
				for _, name := range strings.Fields(string(match[1])) {
					found[string(match[2])+"-"+name] = true
				}
			}
			if len(found) != 9 {
				t.Fatalf("Perl dynamic validation roster is %v", found)
			}
		} else {
			for stem := range validationExpectedReaders {
				found[stem] = bytes.Contains(all, []byte("validation-"+stem+".json"))
			}
		}
		for stem, codes := range validationExpectedReaders {
			if found[stem] != strings.ContainsRune(codes, code) {
				t.Errorf("native reader %c case %s: source reference %t, route %t", code, stem, found[stem], strings.ContainsRune(codes, code))
			}
		}
	}
}

func TestValidationFixturePlanUnforcedToolchainsAndAtomicity(t *testing.T) {
	root := t.TempDir()
	packages := validationTestPackages(root)
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	for _, stem := range []string{"orphan-crates-clean", "orphan-package-roots-clean", "clean-build"} {
		path := validationTestPath(stem)
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
		want := validationTestNames(stem)
		if built.Force || !reflect.DeepEqual(sortedStrings(built.AffectedPackages), want) && !(len(want) == 0 && len(built.AffectedPackages) == 0) {
			t.Fatalf("%s affected %v, force %t; want %v", stem, built.AffectedPackages, built.Force, want)
		}
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got := sortedStrings(built.StateForPlatform(goos).AffectedPackages)
			if !reflect.DeepEqual(got, want) && !(len(got) == 0 && len(want) == 0) {
				t.Fatalf("%s %s affected %v; want %v", stem, goos, got, want)
			}
		}
		for code, reader := range validationTestRoots {
			if code == 'C' || code == 'F' {
				continue
			}
			if built.LanguagesNeeded[reader.language] != strings.ContainsRune(validationExpectedReaders[stem], code) {
				t.Fatalf("%s toolchain %s: %v", stem, reader.language, built.LanguagesNeeded)
			}
		}
		if built.LanguagesNeeded["dotnet"] != strings.ContainsAny(validationExpectedReaders[stem], "CF") {
			t.Fatalf("%s dotnet toolchain: %v", stem, built.LanguagesNeeded)
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
	if code := emitBuildPlan(missing, missingGraph, map[string]bool{}, map[string]bool{}, []string{validationTestPath("orphan-package-roots-clean")}, false, nil, "origin/main", root, missingPlan, false, 0, false, "", "all"); code != 1 {
		t.Fatalf("missing native reader plan exit %d", code)
	}
	if _, err := os.Stat(missingPlan); !os.IsNotExist(err) {
		t.Fatalf("partial missing-reader plan: %v", err)
	}
	unknownPlan := filepath.Join(root, "unknown-case-plan.json")
	if code := emitBuildPlan(packages, graph, map[string]bool{}, map[string]bool{}, []string{validationTestPath("future-case")}, false, nil, "origin/main", root, unknownPlan, false, 0, false, "", "all"); code != 1 {
		t.Fatalf("unknown case plan exit %d", code)
	}
	if _, err := os.Stat(unknownPlan); !os.IsNotExist(err) {
		t.Fatalf("partial unknown-case plan: %v", err)
	}
}

func TestValidationFixtureRenameSelectsDeletedSource(t *testing.T) {
	root := t.TempDir()
	path := validationTestPath("orphan-package-roots-clean")
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
	got, err := changedPackageRootsForPlatform(changed, validationTestPackages(root), root, "linux")
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), validationTestNames("orphan-package-roots-clean")) {
		t.Fatalf("renamed source selected %v, error %v", sortedChangedRoots(got), err)
	}
}
