package main

import (
	"bytes"
	"io/fs"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

const packageMetadataFixturePath = "code/specs/fixtures/build-tool-v1/cases/package-metadata-selection-local-only.json"

var packageMetadataFixtureReaders = []string{
	"dart/programs/build-tool",
	"go/programs/build-tool",
	"java/programs/build-tool",
	"kotlin/programs/build-tool",
	"ocaml/programs/build-tool",
}

func packageMetadataFixturePackages(root string) []discovery.Package {
	var packages []discovery.Package
	for _, reader := range []struct{ name, language, dir string }{
		{"dart/programs/build-tool", "dart", "code/programs/dart/build-tool"},
		{"go/programs/build-tool", "go", "code/programs/go/build-tool"},
		{"java/programs/build-tool", "java", "code/programs/java/build-tool"},
		{"kotlin/programs/build-tool", "kotlin", "code/programs/kotlin/build-tool"},
		{"ocaml/programs/build-tool", "ocaml", "code/programs/ocaml/build-tool"},
		{"python/extra", "python", "code/packages/python/extra"},
	} {
		packages = append(packages, discovery.Package{
			Name: reader.name, Path: filepath.Join(root, filepath.FromSlash(reader.dir)), Language: reader.language,
		})
	}
	return packages
}

// A changed package-metadata case seeds every native reader on every
// platform, including a new case name; lookalike spellings seed nothing.
func TestPackageMetadataFixtureSelectionIsExactAndUnioned(t *testing.T) {
	root := t.TempDir()
	packages := packageMetadataFixturePackages(root)
	for _, path := range []string{
		packageMetadataFixturePath,
		"code/specs/fixtures/build-tool-v1/cases/package-metadata-selection-new-case.json",
	} {
		for _, goos := range []string{"linux", "darwin", "windows"} {
			got, err := changedPackageRootsForPlatform([]string{path}, packages, root, goos)
			if err != nil || !reflect.DeepEqual(testTriggerSelectedNames(got), packageMetadataFixtureReaders) {
				t.Fatalf("%s %s roots = %v, error = %v, want %v", goos, path, got, err, packageMetadataFixtureReaders)
			}
		}
	}
	for _, path := range []string{
		"code/specs/fixtures/build-tool-v1/cases/package-metadata-selection-.json",
		"code/specs/fixtures/build-tool-v1/cases/package-metadata-selection-local-only.json.bak",
		"code/specs/fixtures/build-tool-v1/cases/nested/package-metadata-selection-local-only.json",
		"code/specs/fixtures/build-tool-v1/cases/Package-metadata-selection-local-only.json",
		"code\\specs\\fixtures\\build-tool-v1\\cases\\package-metadata-selection-local-only.json",
	} {
		got, err := changedPackageRootsForPlatform([]string{path}, packages, root, "linux")
		if err != nil || len(got) != 0 {
			t.Fatalf("lookalike %q selected %v, error = %v", path, got, err)
		}
	}
	got, err := changedPackageRootsForPlatform([]string{packageMetadataFixturePath, "code/packages/python/extra/src/lib.py"}, packages, root, "linux")
	want := append(append([]string{}, packageMetadataFixtureReaders...), "python/extra")
	if err != nil || !reflect.DeepEqual(testTriggerSelectedNames(got), want) {
		t.Fatalf("fixture and ordinary union = %v, error = %v, want %v", got, err, want)
	}
}

func TestPackageMetadataFixtureSelectionHonorsLanguageAndFailsClosed(t *testing.T) {
	root := t.TempDir()
	packages := packageMetadataFixturePackages(root)
	for _, tc := range []struct {
		language string
		want     []string
	}{
		{"go", []string{"go/programs/build-tool"}},
		{"java", []string{"java/programs/build-tool"}},
		{"ocaml", []string{"ocaml/programs/build-tool"}},
		{"python", []string{}},
	} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{packageMetadataFixturePath}, packages, root, "linux", tc.language)
		if err != nil || !reflect.DeepEqual(testTriggerSelectedNames(got), tc.want) {
			t.Fatalf("%s roots = %v, error = %v, want %v", tc.language, got, err, tc.want)
		}
	}
	withoutGo := append(append([]discovery.Package{}, packages[:1]...), packages[2:]...)
	for _, language := range []string{"all", "go"} {
		got, err := changedPackageRootsForPlatformAndLanguage([]string{packageMetadataFixturePath}, withoutGo, root, "linux", language)
		if err == nil || got != nil || !strings.Contains(err.Error(), "go/programs/build-tool") {
			t.Fatalf("%s: missing reader returned %v, error = %v", language, got, err)
		}
	}
}

// Every build-tool test source that mentions the variant (its filename
// prefix, or the metadata_policy key it skips on) is a selected reader, and
// every selected reader mentions it.
func TestPackageMetadataFixtureConsumerMapTracksNativeReferences(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	programs := filepath.Join(root, "code", "programs")
	references := map[string]bool{}
	err := filepath.WalkDir(programs, func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if entry.IsDir() {
			switch entry.Name() {
			case "node_modules", ".build", "target", "bin", "obj", "coverage", ".venv", "_build", ".dart_tool", "build", ".gradle":
				return filepath.SkipDir
			}
			return nil
		}
		switch filepath.Ext(entry.Name()) {
		case ".cs", ".dart", ".ex", ".exs", ".fs", ".fsx", ".go", ".hs", ".java", ".kt", ".lua", ".ml", ".mli", ".pl", ".pm", ".py", ".rb", ".rs", ".swift", ".t", ".ts":
		default:
			return nil
		}
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		parts := strings.Split(filepath.ToSlash(rel), "/")
		if len(parts) < 5 || parts[0] != "code" || parts[1] != "programs" || parts[3] != "build-tool" {
			return nil
		}
		lower := strings.ToLower(filepath.ToSlash(rel))
		if !strings.Contains(lower, "/test/") && !strings.Contains(lower, "/tests/") && !strings.Contains(strings.ToLower(entry.Name()), "test") {
			return nil
		}
		if entry.Name() == "package_metadata_fixture_ci_selection_test.go" {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		if bytes.Contains(data, []byte("package-metadata-selection-")) || bytes.Contains(data, []byte("metadata_policy")) {
			references[parts[2]+"/programs/"+parts[3]] = true
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if got := testTriggerSelectedNames(references); !reflect.DeepEqual(got, packageMetadataFixtureReaders) {
		t.Fatalf("native references = %v, selected readers = %v", got, packageMetadataFixtureReaders)
	}
	var consumers []string
	for _, consumer := range packageMetadataFixtureConsumers {
		consumers = append(consumers, consumer.name)
	}
	if !reflect.DeepEqual(consumers, packageMetadataFixtureReaders) {
		t.Fatalf("packageMetadataFixtureConsumers = %v, want %v", consumers, packageMetadataFixtureReaders)
	}
}
