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

const sourceFixturePrefix = "code/specs/fixtures/build-tool-v1/cases/source-collection-"

var sourceFixtureCases = []struct {
	name string
	want []string
}{
	{"extension.json", []string{"csharp", "fsharp", "elixir", "go", "haskell", "lua", "perl", "python", "ruby", "rust", "swift", "typescript"}},
	{"repository-rust-boundary.json", []string{"csharp", "fsharp", "swift"}},
	{"shared-input-conduit-before.json", []string{"csharp", "fsharp"}},
}

var sourceFixtureConsumers = []struct{ name, path, lang string }{
	{"dotnet/programs/build-tool-csharp", "code/programs/dotnet/build-tool-csharp", "csharp"},
	{"dotnet/programs/build-tool-fsharp", "code/programs/dotnet/build-tool-fsharp", "fsharp"},
	{"elixir/programs/build-tool", "code/programs/elixir/build-tool", "elixir"},
	{"go/programs/build-tool", "code/programs/go/build-tool", "go"},
	{"haskell/programs/build-tool", "code/programs/haskell/build-tool", "haskell"},
	{"lua/programs/build-tool", "code/programs/lua/build-tool", "lua"},
	{"perl/programs/build-tool", "code/programs/perl/build-tool", "perl"},
	{"python/programs/build-tool", "code/programs/python/build-tool", "python"},
	{"ruby/programs/build-tool", "code/programs/ruby/build-tool", "ruby"},
	{"rust/programs/build-tool", "code/programs/rust/build-tool", "rust"},
	{"swift/programs/build-tool", "code/programs/swift/build-tool", "swift"},
	{"typescript/programs/build-tool", "code/programs/typescript/build-tool", "typescript"},
}

func sourceFixturePackages(root string) []discovery.Package {
	packages := make([]discovery.Package, 0, len(sourceFixtureConsumers)+1)
	for _, consumer := range sourceFixtureConsumers {
		packages = append(packages, discovery.Package{
			Name: consumer.name, Path: filepath.Join(root, filepath.FromSlash(consumer.path)), Language: consumer.lang,
		})
	}
	return append(packages, discovery.Package{
		Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust",
	})
}

func sourceFixtureNames(languages []string) []string {
	names := make([]string, 0, len(languages))
	for _, language := range languages {
		for _, consumer := range sourceFixtureConsumers {
			if consumer.lang == language {
				names = append(names, consumer.name)
				break
			}
		}
	}
	sort.Strings(names)
	return names
}

func TestSourceCollectionFixtureSelectsExactSubfamilies(t *testing.T) {
	root := t.TempDir()
	packages := sourceFixturePackages(root)
	for _, fixture := range sourceFixtureCases {
		for _, goos := range []string{"linux", "darwin", "windows"} {
			path := sourceFixturePrefix + fixture.name
			got, err := changedPackageRootsForPlatform([]string{path}, packages, root, goos)
			if want := sourceFixtureNames(fixture.want); err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
				t.Fatalf("%s %q roots = %v, error = %v, want %v", goos, path, sortedChangedRoots(got), err, want)
			}
		}
	}
	for _, path := range []string{
		"code/specs/fixtures/build-tool-v1/cases/source-collection.json",
		sourceFixturePrefix + ".json",
		sourceFixturePrefix + "repository-.json",
		sourceFixturePrefix + "shared-input-.json",
		sourceFixturePrefix + "extension.json.bak",
		"code/specs/fixtures/build-tool-v1/cases/Source-collection-extension.json",
		"code/specs/fixtures/build-tool-v1/cases/nested/source-collection-extension.json",
		sourceFixturePrefix + "nested/child.json",
		sourceFixturePrefix + "extension\\child.json",
		"code/specs/fixtures/build-tool-v1/other/source-collection-extension.json",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q selected %v, error = %v", path, got, err)
		}
	}
	for _, changed := range [][]string{nil, {}} {
		got, err := changedPackageRootsForPlatform(changed, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("empty diff selected %v, error = %v", got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{
		sourceFixturePrefix + "repository-rust-boundary.json",
		sourceFixturePrefix + "shared-input-conduit-before.json",
		"code/packages/rust/extra/src/lib.rs",
	}, packages, root, "linux")
	want := append(sourceFixtureNames([]string{"csharp", "fsharp", "swift"}), "rust/extra")
	sort.Strings(want)
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
		t.Fatalf("multiple subfamilies and ordinary edit = %v, error = %v, want %v", sortedChangedRoots(got), err, want)
	}
}

func TestSourceCollectionFixtureLanguageFilterAndMissingRoot(t *testing.T) {
	root := t.TempDir()
	packages := sourceFixturePackages(root)
	for _, fixture := range sourceFixtureCases {
		path := sourceFixturePrefix + fixture.name
		for _, consumer := range sourceFixtureConsumers {
			got, err := changedPackageRootsForPlatformAndLanguage([]string{path}, packages, root, "linux", consumer.lang)
			want := []string{}
			for _, lang := range fixture.want {
				if lang == consumer.lang {
					want = []string{consumer.name}
				}
			}
			if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
				t.Fatalf("%s %q roots = %v, error = %v, want %v", consumer.lang, path, sortedChangedRoots(got), err, want)
			}
		}
		for index, consumer := range sourceFixtureConsumers {
			if !containsPath(fixture.want, consumer.lang) {
				continue
			}
			missing := append([]discovery.Package(nil), packages[:index]...)
			missing = append(missing, packages[index+1:]...)
			for _, language := range []string{"all", consumer.lang} {
				got, err := changedPackageRootsForPlatformAndLanguage([]string{path}, missing, root, "linux", language)
				if err == nil || !strings.Contains(err.Error(), consumer.name) || got != nil {
					t.Fatalf("%s missing %s returned %v, error = %v", language, consumer.name, got, err)
				}
			}
		}
	}
}

func TestSourceCollectionFixtureRenameRetainsDeletedSource(t *testing.T) {
	root := t.TempDir()
	path := sourceFixturePrefix + "extension.json"
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
	selected, err := changedPackageRootsForPlatform(changed, sourceFixturePackages(root), root, "linux")
	if want := sourceFixtureNames(sourceFixtureCases[0].want); err != nil || !reflect.DeepEqual(sortedChangedRoots(selected), want) {
		t.Fatalf("renamed source selected %v, error = %v, want %v", sortedChangedRoots(selected), err, want)
	}
}

func TestSourceCollectionFixturePlanIsUnforcedCompleteAndAtomic(t *testing.T) {
	root := t.TempDir()
	packages := sourceFixturePackages(root)
	missing := packages[1:]
	missingGraph, err := resolver.ResolveDependencies(missing)
	if err != nil {
		t.Fatal(err)
	}
	path := sourceFixturePrefix + "extension.json"
	planPath := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(missing, missingGraph, map[string]bool{}, map[string]bool{},
		[]string{path}, false, nil, "origin/main", root, planPath,
		false, 0, false, "", "all"); code != 1 {
		t.Fatalf("missing consumer plan exit = %d, want 1", code)
	}
	if _, err := os.Stat(planPath); !os.IsNotExist(err) {
		t.Fatalf("partial plan written: %v", err)
	}
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	for _, fixture := range sourceFixtureCases {
		path := sourceFixturePrefix + fixture.name
		changed, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil {
			t.Fatal(err)
		}
		affected := affectedForGraph(graph, changed, nil, false)
		if code := emitBuildPlan(packages, graph, affected, changed,
			[]string{path}, false, nil, "origin/main", root, planPath,
			false, 0, false, "", "all"); code != 0 {
			t.Fatalf("%s complete plan exit = %d", fixture.name, code)
		}
		data, err := os.ReadFile(planPath)
		if err != nil {
			t.Fatal(err)
		}
		var built plan.BuildPlan
		if err := json.Unmarshal(data, &built); err != nil {
			t.Fatal(err)
		}
		want := sourceFixtureNames(fixture.want)
		if built.Force || !reflect.DeepEqual(sortedStrings(built.AffectedPackages), want) {
			t.Fatalf("%s unforced affected roots = %v, force = %t, want %v", fixture.name, built.AffectedPackages, built.Force, want)
		}
		for _, goos := range []string{"linux", "darwin", "windows"} {
			if got := sortedStrings(built.StateForPlatform(goos).AffectedPackages); !reflect.DeepEqual(got, want) {
				t.Fatalf("%s %s affected roots = %v, want %v", fixture.name, goos, got, want)
			}
		}
		wantToolchains := map[string]bool{}
		for _, lang := range fixture.want {
			if lang == "csharp" || lang == "fsharp" {
				lang = "dotnet"
			}
			wantToolchains[lang] = true
		}
		for toolchain, needed := range built.LanguagesNeeded {
			if needed != wantToolchains[toolchain] {
				t.Errorf("%s toolchain %s = %t, want %t", fixture.name, toolchain, needed, wantToolchains[toolchain])
			}
		}
		for toolchain := range wantToolchains {
			if !built.LanguagesNeeded[toolchain] {
				t.Errorf("%s missing native toolchain %s from %v", fixture.name, toolchain, built.LanguagesNeeded)
			}
		}
	}
}

func TestSourceCollectionFixtureCheckedCorpusAndNativeReaders(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	caseDir := filepath.Join(root, "code", "specs", "fixtures", "build-tool-v1", "cases")
	entries, err := os.ReadDir(caseDir)
	if err != nil {
		t.Fatal(err)
	}
	counts := map[string]int{"local": 0, "repository": 0, "shared": 0}
	for _, entry := range entries {
		name := entry.Name()
		if entry.IsDir() || !strings.HasPrefix(name, "source-collection-") || !strings.HasSuffix(name, ".json") {
			continue
		}
		switch {
		case strings.HasPrefix(name, "source-collection-repository-"):
			counts["repository"]++
		case strings.HasPrefix(name, "source-collection-shared-input-"):
			counts["shared"]++
		default:
			counts["local"]++
		}
	}
	if counts["local"] != 7 || counts["repository"] != 9 || counts["shared"] != 4 {
		t.Fatalf("checked source fixture subfamilies = %v", counts)
	}

	// Search independent native test sources, not the Go production selector or
	// this test's own expected map. C# and F# enumerate every checked case,
	// whereas Elixir uses the same glob and filters to seven local cases.
	readers := map[string]map[string]bool{
		"local": {}, "repository": {}, "shared": {},
	}
	err = filepath.WalkDir(filepath.Join(root, "code", "programs"), func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if entry.IsDir() {
			switch entry.Name() {
			case "node_modules", ".build", "target", "bin", "obj", "coverage", ".venv":
				return filepath.SkipDir
			}
			return nil
		}
		switch filepath.Ext(entry.Name()) {
		case ".go", ".cs", ".exs", ".fs", ".hs", ".lua", ".t", ".py", ".rb", ".rs", ".swift", ".ts":
		default:
			return nil
		}
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		parts := strings.Split(filepath.ToSlash(rel), "/")
		if len(parts) < 5 || parts[0] != "code" || parts[1] != "programs" || !strings.HasPrefix(parts[3], "build-tool") {
			return nil
		}
		info, err := entry.Info()
		if err != nil {
			return err
		}
		if info.Size() > 1<<20 {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		pathWithSlashes := "/" + filepath.ToSlash(rel) + "/"
		isTest := strings.Contains(pathWithSlashes, "/tests/") ||
			strings.Contains(pathWithSlashes, "/test/") ||
			strings.Contains(pathWithSlashes, "/t/") ||
			strings.Contains(entry.Name(), "Test") || strings.Contains(entry.Name(), "test") ||
			(strings.HasSuffix(filepath.ToSlash(rel), "rust/build-tool/src/hasher.rs") && bytes.Contains(data, []byte("#[cfg(test)]")))
		if entry.Name() == "source_collection_fixture_ci_selection_test.go" || !isTest {
			return nil
		}
		name := parts[2] + "/programs/" + parts[3]
		if parts[2] == "dotnet" && bytes.Contains(data, []byte("source-collection-*.json")) {
			for _, family := range []string{"local", "repository", "shared"} {
				readers[family][name] = true
			}
			return nil
		}
		if parts[2] == "elixir" && bytes.Contains(data, []byte("source-collection-*.json")) &&
			bytes.Contains(data, []byte("length(cases) == 7")) {
			readers["local"][name] = true
		}
		if bytes.Contains(data, []byte("source-collection-extension.json")) ||
			bytes.Contains(data, []byte("source-collection-declared.json")) ||
			bytes.Contains(data, []byte("source-collection-registry-roles.json")) ||
			(parts[2] == "perl" && bytes.Contains(data, []byte("source-collection-extension"))) {
			readers["local"][name] = true
		}
		if bytes.Contains(data, []byte("source-collection-repository-")) {
			readers["repository"][name] = true
		}
		if bytes.Contains(data, []byte("source-collection-shared-input-")) {
			readers["shared"][name] = true
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	for _, fixture := range sourceFixtureCases {
		family := "local"
		if strings.HasPrefix(fixture.name, "repository-") {
			family = "repository"
		} else if strings.HasPrefix(fixture.name, "shared-input-") {
			family = "shared"
		}
		if got, want := sortedChangedRoots(readers[family]), sourceFixtureNames(fixture.want); !reflect.DeepEqual(got, want) {
			t.Fatalf("native %s readers = %v, expected = %v", family, got, want)
		}
	}
}

func TestSourceCollectionFixtureConsumersResolveThroughDiscovery(t *testing.T) {
	root := t.TempDir()
	for _, consumer := range sourceFixtureConsumers {
		dir := filepath.Join(root, filepath.FromSlash(consumer.path))
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "BUILD"), []byte("echo native fixture\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	packages, err := discovery.DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	for _, fixture := range sourceFixtureCases {
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{sourceFixturePrefix + fixture.name}, packages, root, goos)
			if want := sourceFixtureNames(fixture.want); err != nil || !reflect.DeepEqual(sortedChangedRoots(got), want) {
				t.Fatalf("%s %s discovered roots = %v, error = %v, want %v", goos, fixture.name, sortedChangedRoots(got), err, want)
			}
		}
	}
}
