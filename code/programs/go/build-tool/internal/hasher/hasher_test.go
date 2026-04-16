package hasher

import (
	"crypto/sha256"
	"encoding/hex"
	"os"
	"path/filepath"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

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
