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

const discoveryRegistryFixturePath = "code/specs/fixtures/build-tool-v1/cases/discovery-language-registry.json"

var discoveryRegistryConsumerRoots = []struct {
	name string
	path string
	lang string
}{
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

func discoveryRegistryPackages(root string) []discovery.Package {
	packages := make([]discovery.Package, 0, len(discoveryRegistryConsumerRoots)+1)
	for _, consumer := range discoveryRegistryConsumerRoots {
		packages = append(packages, discovery.Package{
			Name: consumer.name, Path: filepath.Join(root, filepath.FromSlash(consumer.path)), Language: consumer.lang,
		})
	}
	packages = append(packages, discovery.Package{
		Name: "rust/extra", Path: filepath.Join(root, "code", "packages", "rust", "extra"), Language: "rust",
	})
	return packages
}

func sortedChangedRoots(roots map[string]bool) []string {
	result := make([]string, 0, len(roots))
	for name, selected := range roots {
		if selected {
			result = append(result, name)
		}
	}
	sort.Strings(result)
	return result
}

func TestDiscoveryRegistryFixtureSelectsAllNativeConsumers(t *testing.T) {
	root := t.TempDir()
	packages := discoveryRegistryPackages(root)
	want := make([]string, 0, len(discoveryRegistryConsumerRoots))
	for _, consumer := range discoveryRegistryConsumerRoots {
		want = append(want, consumer.name)
	}
	sort.Strings(want)
	for _, goos := range []string{"linux", "darwin", "windows"} {
		got, err := changedPackageRootsForPlatform([]string{discoveryRegistryFixturePath}, packages, root, goos)
		if err != nil {
			t.Fatalf("%s: %v", goos, err)
		}
		if names := sortedChangedRoots(got); !reflect.DeepEqual(names, want) {
			t.Fatalf("%s: roots = %v, want %v", goos, names, want)
		}
		if got["rust/extra"] {
			t.Fatalf("%s: fixture change forced unrelated package", goos)
		}
	}
}

func TestDiscoveryRegistryFixtureSelectionIsExactAndUnionsPackageEdits(t *testing.T) {
	root := t.TempDir()
	packages := discoveryRegistryPackages(root)
	near := []string{"code/specs/fixtures/build-tool-v1/cases/discovery-language-registry-example.json"}
	got, err := changedPackageRootsForPlatform(near, packages, root, "linux")
	if err != nil || len(got) != 0 {
		t.Fatalf("near path selected roots = %v, error = %v", got, err)
	}
	// A deleted fixture still appears as a changed path and requires consumers.
	got, err = changedPackageRootsForPlatform([]string{
		discoveryRegistryFixturePath,
		"code/packages/rust/extra/src/lib.rs",
	}, packages, root, "linux")
	if err != nil || len(got) != len(discoveryRegistryConsumerRoots)+1 || !got["rust/extra"] {
		t.Fatalf("fixture and package union = %v, error = %v", got, err)
	}
}

func TestDiscoveryRegistryFixtureSelectionFailsClosedWhenConsumerMissing(t *testing.T) {
	root := t.TempDir()
	packages := discoveryRegistryPackages(root)
	packages = packages[1:]
	_, err := changedPackageRootsForPlatform([]string{discoveryRegistryFixturePath}, packages, root, "linux")
	if err == nil {
		t.Fatal("missing C# native consumer was silently omitted")
	}
}

func TestDiscoveryRegistryFixtureSingleLanguageSelection(t *testing.T) {
	root := t.TempDir()
	var lua []discovery.Package
	for _, pkg := range discoveryRegistryPackages(root) {
		if pkg.Language == "lua" {
			lua = append(lua, pkg)
		}
	}
	got, err := changedPackageRootsForPlatformAndLanguage(
		[]string{discoveryRegistryFixturePath}, lua, root, "linux", "lua",
	)
	if err != nil || !reflect.DeepEqual(sortedChangedRoots(got), []string{"lua/programs/build-tool"}) {
		t.Fatalf("single-language fixture roots = %v, error = %v", got, err)
	}
}

func TestDiscoveryRegistryFixtureRenameKeepsDeletedSourcePath(t *testing.T) {
	root := t.TempDir()
	fixture := filepath.Join(root, filepath.FromSlash(discoveryRegistryFixturePath))
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
	if err := os.Rename(fixture, filepath.Join(filepath.Dir(fixture), "discovery-language-registry-renamed.json")); err != nil {
		t.Fatal(err)
	}
	git("add", "-A")
	git("-c", "user.name=Fixture Test", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "rename")
	if changed := gitdiff.GetChangedFiles(root, "base"); !containsPath(changed, discoveryRegistryFixturePath) {
		t.Fatalf("rename diff omitted old fixture path: %v", changed)
	}
}

func TestDiscoveryRegistryFixtureBuildPlanWritesNothingWhenConsumerMissing(t *testing.T) {
	root := t.TempDir()
	packages := discoveryRegistryPackages(root)[1:]
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	planPath := filepath.Join(root, "missing-consumer-plan.json")
	if code := emitBuildPlan(
		packages, graph, map[string]bool{}, map[string]bool{},
		[]string{discoveryRegistryFixturePath}, false, nil, "origin/main",
		root, planPath, false, 0, false, "", "all",
	); code != 1 {
		t.Fatalf("missing fixture consumer plan exit = %d, want 1", code)
	}
	if _, err := os.Stat(planPath); !os.IsNotExist(err) {
		t.Fatalf("plan unexpectedly written on missing consumer: %v", err)
	}
}

func TestDiscoveryRegistryFixtureBuildPlanKeepsEveryPlatformAndToolchain(t *testing.T) {
	root := t.TempDir()
	packages := discoveryRegistryPackages(root)
	graph, err := resolver.ResolveDependencies(packages)
	if err != nil {
		t.Fatal(err)
	}
	changed, err := changedPackageRootsForPlatform([]string{discoveryRegistryFixturePath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	affected := affectedForGraph(graph, changed, nil, false)
	planPath := filepath.Join(root, "plan.json")
	if code := emitBuildPlan(
		packages, graph, affected, changed, []string{discoveryRegistryFixturePath},
		false, nil, "origin/main", root, planPath, false, 0, false, "", "all",
	); code != 0 {
		t.Fatalf("emitBuildPlan exit = %d", code)
	}
	data, err := os.ReadFile(planPath)
	if err != nil {
		t.Fatal(err)
	}
	var built plan.BuildPlan
	if err := json.Unmarshal(data, &built); err != nil {
		t.Fatal(err)
	}
	if built.Force || built.AffectedPackages == nil || len(built.AffectedPackages) != len(sharedDiscoveryFixtureConsumers) {
		t.Fatalf("fixture plan should select exactly %d packages without force: %#v", len(sharedDiscoveryFixtureConsumers), built.AffectedPackages)
	}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		state := built.StateForPlatform(goos)
		if state.AffectedPackages == nil || len(state.AffectedPackages) != len(sharedDiscoveryFixtureConsumers) {
			t.Fatalf("%s affected packages = %v", goos, state.AffectedPackages)
		}
	}
	for _, toolchain := range []string{"dotnet", "elixir", "go", "haskell", "lua", "perl", "python", "ruby", "rust", "swift", "typescript"} {
		if !built.LanguagesNeeded[toolchain] {
			t.Errorf("missing native fixture toolchain %s", toolchain)
		}
	}
}

func TestDiscoveryRegistryFixtureConsumerMapTracksNativeReferences(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	programs := filepath.Join(root, "code", "programs")
	references := map[string]bool{}
	err := filepath.WalkDir(programs, func(path string, entry fs.DirEntry, walkErr error) error {
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
		isTest := strings.Contains("/"+filepath.ToSlash(rel)+"/", "/tests/") ||
			strings.Contains("/"+filepath.ToSlash(rel)+"/", "/test/") ||
			strings.Contains("/"+filepath.ToSlash(rel)+"/", "/t/") ||
			strings.Contains(entry.Name(), "Test") || strings.Contains(entry.Name(), "test") ||
			(strings.HasSuffix(filepath.ToSlash(rel), "rust/build-tool/src/discovery.rs") && bytes.Contains(data, []byte("#[cfg(test)]")))
		if entry.Name() == "discovery_fixture_ci_selection_test.go" || !isTest {
			return nil
		}
		if bytes.Contains(data, []byte("discovery-language-registry.json")) {
			references[parts[2]+"/programs/"+parts[3]] = true
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	want := make([]string, 0, len(sharedDiscoveryFixtureConsumers))
	for _, consumer := range sharedDiscoveryFixtureConsumers {
		want = append(want, consumer.name)
	}
	sort.Strings(want)
	if got := sortedChangedRoots(references); !reflect.DeepEqual(got, want) {
		t.Fatalf("direct native references = %v, detector consumers = %v", got, want)
	}
}

func TestDiscoveryRegistryFixtureConsumersResolveThroughDiscovery(t *testing.T) {
	root := t.TempDir()
	for _, consumer := range discoveryRegistryConsumerRoots {
		dir := filepath.Join(root, filepath.FromSlash(consumer.path))
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "BUILD"), []byte("echo fixture\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	packages, err := discovery.DiscoverPackages(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	selected, err := changedPackageRootsForPlatform([]string{discoveryRegistryFixturePath}, packages, root, "linux")
	if err != nil {
		t.Fatal(err)
	}
	if len(selected) != len(sharedDiscoveryFixtureConsumers) {
		t.Fatalf("discovery selected %d native consumers, want %d", len(selected), len(sharedDiscoveryFixtureConsumers))
	}
}
