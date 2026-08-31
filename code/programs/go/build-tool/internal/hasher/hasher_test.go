package hasher

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

type sourceCollectionFixture struct {
	Input struct {
		Options struct {
			Mode         string   `json:"mode"`
			DeclaredSrcs []string `json:"declared_srcs"`
			Candidates   []struct {
				Path       string `json:"path"`
				Kind       string `json:"kind"`
				ContentHex string `json:"content_hex"`
			} `json:"candidates"`
		} `json:"options"`
	} `json:"input"`
	Expected struct {
		Result struct {
			Files []struct {
				Path   string `json:"path"`
				Digest string `json:"digest"`
			} `json:"files"`
		} `json:"result"`
	} `json:"expected"`
}

type hashingCacheFixture struct {
	Input struct {
		Options struct {
			Package      string   `json:"package"`
			IncludePaths []string `json:"include_paths"`
		} `json:"options"`
	} `json:"input"`
	Expected struct {
		Result struct {
			PackageDigest string `json:"package_digest"`
		} `json:"result"`
	} `json:"expected"`
}

func fixturePath(t *testing.T, name string) string {
	t.Helper()
	_, filename, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("cannot locate hasher fixture directory")
	}
	return filepath.Clean(filepath.Join(
		filepath.Dir(filename), "..", "..", "..", "..", "..", "specs", "fixtures", "build-tool-v1", "cases", name,
	))
}

func readJSONFixture[T any](t *testing.T, name string) T {
	t.Helper()
	contents, err := os.ReadFile(fixturePath(t, name))
	if err != nil {
		t.Fatal(err)
	}
	var fixture T
	if err := json.Unmarshal(contents, &fixture); err != nil {
		t.Fatal(err)
	}
	return fixture
}

func materializeSourceFixture(t *testing.T, fixture sourceCollectionFixture) string {
	t.Helper()
	root := t.TempDir()
	linkTargets := map[string]string{}
	for _, candidate := range fixture.Input.Options.Candidates {
		if candidate.Kind == "symlink" || candidate.Kind == "reparse_point" {
			linkTargets[candidate.Path] = t.TempDir()
		}
	}
	for _, candidate := range fixture.Input.Options.Candidates {
		if candidate.Kind != "file" {
			continue
		}
		skipped := false
		for linkPath, target := range linkTargets {
			prefix := linkPath + "/"
			if strings.HasPrefix(candidate.Path, prefix) {
				path := filepath.Join(target, filepath.FromSlash(strings.TrimPrefix(candidate.Path, prefix)))
				if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
					t.Fatal(err)
				}
				contents, err := hex.DecodeString(candidate.ContentHex)
				if err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(path, contents, 0o644); err != nil {
					t.Fatal(err)
				}
				skipped = true
				break
			}
		}
		if skipped {
			continue
		}
		path := filepath.Join(root, filepath.FromSlash(candidate.Path))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		contents, err := hex.DecodeString(candidate.ContentHex)
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, contents, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	for linkPath, target := range linkTargets {
		path := filepath.Join(root, filepath.FromSlash(linkPath))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		// Some Windows hosts require Developer Mode or elevated symlink rights.
		// An unavailable inert link is equivalent to an absent candidate here;
		// the dedicated real-link regression below records an explicit skip.
		_ = os.Symlink(target, path)
	}
	return root
}

func relativePaths(t *testing.T, root string, files []string) []string {
	t.Helper()
	paths := make([]string, 0, len(files))
	for _, file := range files {
		rel, err := filepath.Rel(root, file)
		if err != nil {
			t.Fatal(err)
		}
		paths = append(paths, filepath.ToSlash(rel))
	}
	return paths
}

func hashingV1Digest(t *testing.T, files map[string][]byte) string {
	t.Helper()
	paths := make([]string, 0, len(files))
	for path := range files {
		paths = append(paths, path)
	}
	sort.Strings(paths)
	hash := sha256.New()
	for _, path := range paths {
		pathBytes := []byte(path)
		if err := binary.Write(hash, binary.BigEndian, uint64(len(pathBytes))); err != nil {
			t.Fatal(err)
		}
		if _, err := hash.Write(pathBytes); err != nil {
			t.Fatal(err)
		}
		contents := files[path]
		if err := binary.Write(hash, binary.BigEndian, uint64(len(contents))); err != nil {
			t.Fatal(err)
		}
		if _, err := hash.Write(contents); err != nil {
			t.Fatal(err)
		}
	}
	return hex.EncodeToString(hash.Sum(nil))
}

// makeFixture creates a temporary directory tree for testing.
func makeFixture(t *testing.T, tree map[string]string) string {
	t.Helper()
	root := t.TempDir()
	for relPath, content := range tree {
		absPath := filepath.Join(root, filepath.FromSlash(relPath))
		if err := os.MkdirAll(filepath.Dir(absPath), 0755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(absPath, []byte(content), 0644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

// emptyHash is the SHA256 hash of the empty string — used as the default
// when there are no files or no dependencies.
func emptyHash() string {
	h := sha256.Sum256([]byte(""))
	return hex.EncodeToString(h[:])
}

// ---------------------------------------------------------------------------
// Tests for collectSourceFiles
// ---------------------------------------------------------------------------

func TestCollectSourceFilesPython(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD":            "echo build",
		"pkg/pyproject.toml":   "[project]\nname = \"test\"\n",
		"pkg/src/main.py":      "print('hello')\n",
		"pkg/src/helper.py":    "pass\n",
		"pkg/README.md":        "docs",             // should be excluded
		"pkg/data/config.json": `{"key": "value"}`, // should be excluded
	})

	pkg := discovery.Package{
		Name:     "python/pkg",
		Path:     filepath.Join(root, "pkg"),
		Language: "python",
	}

	files := collectSourceFiles(pkg)
	// Expected: BUILD, pyproject.toml, helper.py, main.py (sorted by relative path)
	if len(files) != 4 {
		names := make([]string, len(files))
		for i, f := range files {
			names[i] = filepath.Base(f)
		}
		t.Fatalf("expected 4 files, got %d: %v", len(files), names)
	}
}

func TestCollectSourceFilesGo(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD":        "go build .",
		"pkg/go.mod":       "module test\n",
		"pkg/go.sum":       "hash\n",
		"pkg/main.go":      "package main\n",
		"pkg/main_test.go": "package main\n",
		"pkg/README.md":    "docs",
	})

	pkg := discovery.Package{
		Name:     "go/pkg",
		Path:     filepath.Join(root, "pkg"),
		Language: "go",
	}

	files := collectSourceFiles(pkg)
	// Expected: BUILD, go.mod, go.sum, main.go, main_test.go
	if len(files) != 5 {
		names := make([]string, len(files))
		for i, f := range files {
			names[i] = filepath.Base(f)
		}
		t.Fatalf("expected 5 files, got %d: %v", len(files), names)
	}
}

func TestCollectSourceFilesRuby(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD":       "bundle exec rake",
		"pkg/Gemfile":     "source 'https://rubygems.org'\n",
		"pkg/Rakefile":    "task :default\n",
		"pkg/lib.gemspec": "spec\n",
		"pkg/lib/main.rb": "puts 'hi'\n",
		"pkg/README.md":   "docs",
	})

	pkg := discovery.Package{
		Name:     "ruby/pkg",
		Path:     filepath.Join(root, "pkg"),
		Language: "ruby",
	}

	files := collectSourceFiles(pkg)
	// Expected: BUILD, Gemfile, Rakefile, lib.gemspec, main.rb
	if len(files) != 5 {
		names := make([]string, len(files))
		for i, f := range files {
			names[i] = filepath.Base(f)
		}
		t.Fatalf("expected 5 files, got %d: %v", len(files), names)
	}
}

func TestCollectSourceFilesDart(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD":                 "dart run bin/hello_world.dart",
		"pkg/pubspec.yaml":          "name: hello_world\n",
		"pkg/pubspec.lock":          "packages:\n",
		"pkg/analysis_options.yaml": "include: package:lints/recommended.yaml\n",
		"pkg/bin/hello_world.dart":  "void main() => print('hi');\n",
		"pkg/README.md":             "docs",
	})

	pkg := discovery.Package{
		Name:     "dart/pkg",
		Path:     filepath.Join(root, "pkg"),
		Language: "dart",
	}

	files := collectSourceFiles(pkg)
	if len(files) != 5 {
		names := make([]string, len(files))
		for i, f := range files {
			names[i] = filepath.Base(f)
		}
		t.Fatalf("expected 5 files, got %d: %v", len(files), names)
	}
}

func TestCollectSourceFilesGradleLanguages(t *testing.T) {
	for _, tc := range []struct {
		language   string
		sourceName string
	}{
		{language: "java", sourceName: "Main.java"},
		{language: "kotlin", sourceName: "Main.kt"},
	} {
		t.Run(tc.language, func(t *testing.T) {
			root := makeFixture(t, map[string]string{
				"pkg/BUILD":                     "gradle test",
				"pkg/settings.gradle.kts":       "includeBuild(\"../dependency\")\n",
				"pkg/build.gradle.kts":          "plugins {}\n",
				"pkg/src/main/" + tc.sourceName: "class Main\n",
				"pkg/README.md":                 "docs",
			})
			pkg := discovery.Package{
				Name:     tc.language + "/pkg",
				Path:     filepath.Join(root, "pkg"),
				Language: tc.language,
			}

			files := collectSourceFiles(pkg)
			if len(files) != 4 {
				names := make([]string, len(files))
				for index, file := range files {
					names[index] = filepath.Base(file)
				}
				t.Fatalf("expected 4 Gradle inputs, got %d: %v", len(files), names)
			}
		})
	}
}

func TestCollectSourceFilesOCaml(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD":                       "opam exec -- dune build\n",
		"pkg/BUILD_windows":               "opam exec -- dune build\n",
		"pkg/.ocamlformat":                "version=0.27.0\n",
		"pkg/coding-adventures-pkg.opam":  "opam-version: \"2.0\"\n",
		"pkg/dune-project":                "(lang dune 3.16)\n",
		"pkg/src/dune":                    "(library (name pkg))\n",
		"pkg/src/pkg.ml":                  "let value = 1\n",
		"pkg/src/pkg.mli":                 "val value : int\n",
		"pkg/README.md":                   "docs\n",
		"pkg/_build/default/generated.ml": "let generated = true\n",
	})
	pkg := discovery.Package{Name: "ocaml/pkg", Path: filepath.Join(root, "pkg"), Language: "ocaml"}

	files := collectSourceFiles(pkg)
	got := make(map[string]bool, len(files))
	for _, file := range files {
		rel, err := filepath.Rel(pkg.Path, file)
		if err != nil {
			t.Fatal(err)
		}
		got[filepath.ToSlash(rel)] = true
	}
	for _, want := range []string{
		"BUILD", "BUILD_windows", ".ocamlformat", "coding-adventures-pkg.opam",
		"dune-project", "src/dune", "src/pkg.ml", "src/pkg.mli",
	} {
		if !got[want] {
			t.Errorf("missing OCaml hash input %q: %v", want, got)
		}
	}
	for _, excluded := range []string{"README.md", "_build/default/generated.ml"} {
		if got[excluded] {
			t.Errorf("unexpected OCaml hash input %q", excluded)
		}
	}
}

func TestNeutralSourceCollectionFixtures(t *testing.T) {
	for _, name := range []string{
		"source-collection-extension.json",
		"source-collection-declared.json",
	} {
		t.Run(name, func(t *testing.T) {
			fixture := readJSONFixture[sourceCollectionFixture](t, name)
			root := materializeSourceFixture(t, fixture)
			pkg := discovery.Package{
				Name:         "ocaml/demo",
				Path:         root,
				Language:     "ocaml",
				DeclaredSrcs: fixture.Input.Options.DeclaredSrcs,
			}

			var files []string
			if fixture.Input.Options.Mode == "declared_sources" {
				files = resolveDeclaredSrcs(pkg)
			} else {
				files = collectSourceFiles(pkg)
			}
			got := relativePaths(t, root, files)
			want := make([]string, 0, len(fixture.Expected.Result.Files))
			for _, expected := range fixture.Expected.Result.Files {
				want = append(want, expected.Path)
				contents, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(expected.Path)))
				if err != nil {
					t.Fatal(err)
				}
				digest := sha256.Sum256(contents)
				if hex.EncodeToString(digest[:]) != expected.Digest {
					t.Fatalf("unexpected digest for %s", expected.Path)
				}
			}
			if strings.Join(got, "\n") != strings.Join(want, "\n") {
				t.Fatalf("source fixture mismatch\n got: %v\nwant: %v", got, want)
			}
		})
	}
}

func TestCollectSourceFilesIncludesOnlyExactBuildFronts(t *testing.T) {
	tree := map[string]string{}
	for _, name := range []string{"BUILD", "BUILD_mac", "BUILD_linux", "BUILD_windows", "BUILD_mac_and_linux", "BUILD_preview"} {
		tree["pkg/"+name] = "echo build\n"
	}
	root := makeFixture(t, tree)
	pkgPath := filepath.Join(root, "pkg")
	got := relativePaths(t, pkgPath, collectSourceFiles(discovery.Package{Name: "go/demo", Path: pkgPath}))
	want := []string{"BUILD", "BUILD_linux", "BUILD_mac", "BUILD_mac_and_linux", "BUILD_windows"}
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("BUILD-front mismatch: got %v, want %v", got, want)
	}
}

func TestResolveDeclaredSrcsRetainsRootManifestOnly(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD":            "go_library(name='demo')\n",
		"pkg/demo.opam":        "opam-version: \"2.0\"\n",
		"pkg/nested/demo.opam": "opam-version: \"2.0\"\n",
		"pkg/src/main.ml":      "let main = ()\n",
	})
	pkgPath := filepath.Join(root, "pkg")
	pkg := discovery.Package{Name: "ocaml/demo", Path: pkgPath, Language: "ocaml", DeclaredSrcs: []string{"src/**/*.ml"}}
	got := relativePaths(t, pkgPath, resolveDeclaredSrcs(pkg))
	want := []string{"BUILD", "demo.opam", "src/main.ml"}
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("declared manifest mismatch: got %v, want %v", got, want)
	}

	pkg.DeclaredSrcs = append(pkg.DeclaredSrcs, "nested/*.opam")
	got = relativePaths(t, pkgPath, resolveDeclaredSrcs(pkg))
	want = []string{"BUILD", "demo.opam", "nested/demo.opam", "src/main.ml"}
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("explicit nested manifest mismatch: got %v, want %v", got, want)
	}
}

func TestCollectorsDoNotFollowDirectoryLinks(t *testing.T) {
	root := t.TempDir()
	pkgPath := filepath.Join(root, "pkg")
	if err := os.MkdirAll(pkgPath, 0o755); err != nil {
		t.Fatal(err)
	}
	external := t.TempDir()
	if err := os.WriteFile(filepath.Join(external, "external.ml"), []byte("let external = true\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(external, filepath.Join(pkgPath, "linked")); err != nil {
		t.Skipf("directory links unavailable on this host: %v", err)
	}
	pkg := discovery.Package{Name: "ocaml/demo", Path: pkgPath, Language: "ocaml"}
	if got := collectSourceFiles(pkg); len(got) != 0 {
		t.Fatalf("extension collector crossed directory link: %v", got)
	}
	pkg.DeclaredSrcs = []string{"**/*.ml"}
	if got := resolveDeclaredSrcs(pkg); len(got) != 0 {
		t.Fatalf("declared collector crossed directory link: %v", got)
	}
}

func TestCollectSourceFilesEmpty(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{
		Name:     "unknown/empty",
		Path:     root,
		Language: "unknown",
	}

	files := collectSourceFiles(pkg)
	if len(files) != 0 {
		t.Fatalf("expected 0 files, got %d", len(files))
	}
}

func TestGeneratedDirectoryRegistryIsExactAndComplete(t *testing.T) {
	want := []string{
		".build", ".cargo", ".claude", ".dart_tool", ".git", ".gradle", ".hg",
		".mypy_cache", ".pytest_cache", ".ruff_cache", ".stack-work", ".svn", ".tox",
		".venv", "Pods", "__pycache__", "_build", "build", "cover", "deps", "dist",
		"dist-newstyle", "gradle-build", "node_modules", "target", "vendor",
	}
	got := make([]string, 0, len(generatedDirectoryComponents))
	for component := range generatedDirectoryComponents {
		got = append(got, component)
	}
	sort.Strings(got)
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("generated-directory registry mismatch: got %v, want %v", got, want)
	}
}

func TestRepositoryRelativePackagePath(t *testing.T) {
	canonical := discovery.Package{
		Name: "ignored/name",
		Path: filepath.Join(t.TempDir(), "code", "programs", "go", "demo"),
	}
	if got, err := repositoryRelativePackagePath(canonical); err != nil || got != "code/programs/go/demo" {
		t.Fatalf("canonical program path: got %q, err %v", got, err)
	}
	for _, tc := range []struct {
		name string
		want string
	}{
		{name: "python/demo", want: "code/packages/python/demo"},
		{name: "go/programs/demo", want: "code/programs/go/demo"},
	} {
		got, err := repositoryRelativePackagePath(discovery.Package{Name: tc.name, Path: t.TempDir()})
		if err != nil || got != tc.want {
			t.Fatalf("identity %q: got %q, err %v", tc.name, got, err)
		}
	}
	if _, err := repositoryRelativePackagePath(discovery.Package{Name: "invalid", Path: t.TempDir()}); err == nil {
		t.Fatal("invalid package identity must fail closed")
	}
}

func TestPortablePathValidation(t *testing.T) {
	for _, path := range []string{"", "/absolute", "a//b", "a/./b", "a/../b", "a\\b", "a\x00b"} {
		if err := validateRepositoryPath(path); err == nil {
			t.Errorf("expected %q to be rejected", path)
		}
	}
	if err := validateRepositoryPath("code/packages/go/\U0001f600"); err != nil {
		t.Fatalf("portable UTF-8 path rejected: %v", err)
	}
	if runtime.GOOS != "windows" {
		root := t.TempDir()
		path := filepath.Join(root, "literal\\name.go")
		if err := os.WriteFile(path, []byte("package demo\n"), 0o644); err != nil {
			t.Fatal(err)
		}
		if _, err := portableRelativePath(root, path); err == nil {
			t.Fatal("POSIX backslash filename must not alias a portable nested path")
		}
	}
}

// ---------------------------------------------------------------------------
// Tests for HashPackage
// ---------------------------------------------------------------------------

func TestHashPackageDeterministic(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD":       "echo build",
		"pkg/src/main.py": "print('hello')\n",
	})

	pkg := discovery.Package{
		Name:     "python/pkg",
		Path:     filepath.Join(root, "pkg"),
		Language: "python",
	}

	hash1 := HashPackage(pkg)
	hash2 := HashPackage(pkg)

	if hash1 != hash2 {
		t.Fatal("hash should be deterministic")
	}
	if len(hash1) != 64 {
		t.Fatalf("expected 64-char hex hash, got %d chars", len(hash1))
	}
}

func TestHashPackageMatchesHashingV1Oracle(t *testing.T) {
	fixture := readJSONFixture[hashingCacheFixture](t, "hashing-cache-missing.json")
	root := makeFixture(t, map[string]string{"pkg/src/data.bin": "abc"})
	pkgPath := filepath.Join(root, "pkg")
	pkg := discovery.Package{
		Name:         fixture.Input.Options.Package,
		Path:         pkgPath,
		DeclaredSrcs: []string{"src/data.bin"},
	}
	if got := HashPackage(pkg); got != fixture.Expected.Result.PackageDigest {
		t.Fatalf("hashing-v1 oracle mismatch: got %s, want %s", got, fixture.Expected.Result.PackageDigest)
	}
}

func TestHashPackageFramesRepositoryPathAndRawContent(t *testing.T) {
	root := t.TempDir()
	pkgPath := filepath.Join(root, "demo")
	if err := os.MkdirAll(filepath.Join(pkgPath, "src"), 0o755); err != nil {
		t.Fatal(err)
	}
	contents := []byte{0x00, 0xff, '\r', '\n', 'a'}
	if err := os.WriteFile(filepath.Join(pkgPath, "src", "\U0001f600.bin"), contents, 0o644); err != nil {
		t.Fatal(err)
	}
	pkg := discovery.Package{Name: "go/demo", Path: pkgPath, DeclaredSrcs: []string{"src/*.bin"}}
	want := hashingV1Digest(t, map[string][]byte{"code/packages/go/demo/src/\U0001f600.bin": contents})
	if got := HashPackage(pkg); got != want {
		t.Fatalf("raw package frame mismatch: got %s, want %s", got, want)
	}
}

func TestHashPackageChangesWhenSameContentFileIsRenamed(t *testing.T) {
	root := makeFixture(t, map[string]string{"pkg/src/a.py": "same\n"})
	pkgPath := filepath.Join(root, "pkg")
	pkg := discovery.Package{Name: "python/demo", Path: pkgPath, Language: "python"}
	before := HashPackage(pkg)
	if err := os.Rename(filepath.Join(pkgPath, "src", "a.py"), filepath.Join(pkgPath, "src", "b.py")); err != nil {
		t.Fatal(err)
	}
	after := HashPackage(pkg)
	if before == after {
		t.Fatal("same-content rename must change the package digest")
	}
}

func TestHashPackageChangesOnModification(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD":       "echo build",
		"pkg/src/main.py": "print('hello')\n",
	})

	pkg := discovery.Package{
		Name:     "python/pkg",
		Path:     filepath.Join(root, "pkg"),
		Language: "python",
	}

	hash1 := HashPackage(pkg)

	// Modify the file.
	os.WriteFile(filepath.Join(root, "pkg/src/main.py"), []byte("print('world')\n"), 0644)

	hash2 := HashPackage(pkg)
	if hash1 == hash2 {
		t.Fatal("hash should change when file is modified")
	}
}

func TestHashPackageEmptyPackage(t *testing.T) {
	root := t.TempDir()
	pkg := discovery.Package{
		Name:     "unknown/empty",
		Path:     root,
		Language: "unknown",
	}

	hash := HashPackage(pkg)
	if hash != emptyHash() {
		t.Fatalf("expected empty hash, got %s", hash)
	}
}

// ---------------------------------------------------------------------------
// Tests for HashDeps
// ---------------------------------------------------------------------------

func TestHashDepsNoDeps(t *testing.T) {
	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	hashes := map[string]string{
		"python/pkg-a": "abc123",
	}

	hash := HashDeps("python/pkg-a", graph, hashes)
	if hash != emptyHash() {
		t.Fatalf("expected empty hash for no deps, got %s", hash)
	}
}

func TestHashDepsWithDeps(t *testing.T) {
	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")
	graph.AddNode("python/pkg-b")
	graph.AddEdge("python/pkg-b", "python/pkg-a") // pkg-a depends on pkg-b

	hashes := map[string]string{
		"python/pkg-a": "hash-a",
		"python/pkg-b": "hash-b",
	}

	hash := HashDeps("python/pkg-a", graph, hashes)
	if hash == emptyHash() {
		t.Fatal("expected non-empty hash when deps exist")
	}

	// The hash should be deterministic.
	hash2 := HashDeps("python/pkg-a", graph, hashes)
	if hash != hash2 {
		t.Fatal("hash should be deterministic")
	}
}

func TestHashDepsChangesWhenDepChanges(t *testing.T) {
	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")
	graph.AddNode("python/pkg-b")
	graph.AddEdge("python/pkg-b", "python/pkg-a")

	hashes1 := map[string]string{
		"python/pkg-a": "hash-a",
		"python/pkg-b": "hash-b-v1",
	}

	hashes2 := map[string]string{
		"python/pkg-a": "hash-a",
		"python/pkg-b": "hash-b-v2",
	}

	h1 := HashDeps("python/pkg-a", graph, hashes1)
	h2 := HashDeps("python/pkg-a", graph, hashes2)

	if h1 == h2 {
		t.Fatal("deps hash should change when a dependency's hash changes")
	}
}

func TestHashDepsNodeNotInGraph(t *testing.T) {
	graph := directedgraph.New()
	hash := HashDeps("nonexistent", graph, map[string]string{})
	if hash != emptyHash() {
		t.Fatalf("expected empty hash for missing node, got %s", hash)
	}
}

func TestHashDepsTransitive(t *testing.T) {
	// Chain: C depends on B depends on A.
	// pkg-c's deps hash should include both A and B.
	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")
	graph.AddNode("python/pkg-b")
	graph.AddNode("python/pkg-c")
	graph.AddEdge("python/pkg-a", "python/pkg-b") // B depends on A
	graph.AddEdge("python/pkg-b", "python/pkg-c") // C depends on B

	hashes := map[string]string{
		"python/pkg-a": "hash-a",
		"python/pkg-b": "hash-b",
		"python/pkg-c": "hash-c",
	}

	hashC := HashDeps("python/pkg-c", graph, hashes)
	if hashC == emptyHash() {
		t.Fatal("pkg-c should have deps hash (depends on both A and B)")
	}

	// Changing A should change C's deps hash.
	hashes2 := map[string]string{
		"python/pkg-a": "hash-a-CHANGED",
		"python/pkg-b": "hash-b",
		"python/pkg-c": "hash-c",
	}
	hashC2 := HashDeps("python/pkg-c", graph, hashes2)
	if hashC == hashC2 {
		t.Fatal("changing transitive dep A should change C's deps hash")
	}
}
