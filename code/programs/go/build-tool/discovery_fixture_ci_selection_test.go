package main

import (
	"path/filepath"
	"reflect"
	"sort"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

const discoveryRegistryFixturePath = "code/specs/fixtures/build-tool-v1/cases/discovery-language-registry.json"

var discoveryRegistryConsumerRoots = []struct {
	name string
	path string
	lang string
}{
	{"dotnet/programs/build-tool-csharp", "code/programs/dotnet/build-tool-csharp", "csharp"},
	{"dotnet/programs/build-tool-fsharp", "code/programs/dotnet/build-tool-fsharp", "fsharp"},
	{"go/programs/build-tool", "code/programs/go/build-tool", "go"},
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
