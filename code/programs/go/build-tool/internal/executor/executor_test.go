package executor

import (
	"os"
	"path/filepath"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cache"
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

// ---------------------------------------------------------------------------
// Tests for runPackageBuild
// ---------------------------------------------------------------------------

func TestRunPackageBuildSuccess(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "echo hello",
	})

	pkg := discovery.Package{
		Name:          "python/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"echo hello"},
		Language:      "python",
	}

	result := runPackageBuild(pkg)
	if result.Status != "built" {
		t.Fatalf("expected built, got %s (stderr: %s)", result.Status, result.Stderr)
	}
	if result.ReturnCode != 0 {
		t.Fatalf("expected return code 0, got %d", result.ReturnCode)
	}
}

func TestRunPackageBuildFailure(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "exit 1",
	})

	pkg := discovery.Package{
		Name:          "python/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"exit 1"},
		Language:      "python",
	}

	result := runPackageBuild(pkg)
	if result.Status != "failed" {
		t.Fatalf("expected failed, got %s", result.Status)
	}
	if result.ReturnCode == 0 {
		t.Fatal("expected non-zero return code")
	}
}

func TestRunPackageBuildMultipleCommands(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "echo first\necho second",
	})

	pkg := discovery.Package{
		Name:          "python/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"echo first", "echo second"},
		Language:      "python",
	}

	result := runPackageBuild(pkg)
	if result.Status != "built" {
		t.Fatalf("expected built, got %s", result.Status)
	}
}

func TestRunPackageBuildStopsOnFailure(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg/BUILD": "echo first\nexit 1\necho third",
	})

	pkg := discovery.Package{
		Name:          "python/pkg",
		Path:          filepath.Join(root, "pkg"),
		BuildCommands: []string{"echo first", "exit 1", "echo third"},
		Language:      "python",
	}

	result := runPackageBuild(pkg)
	if result.Status != "failed" {
		t.Fatalf("expected failed, got %s", result.Status)
	}
	// "third" should not appear in stdout because execution stops at the failure.
	if contains(result.Stdout, "third") {
		t.Fatal("should not have executed command after failure")
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(substr) == 0 ||
		(len(s) > 0 && len(substr) > 0 && containsHelper(s, substr)))
}

func containsHelper(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// ---------------------------------------------------------------------------
// Tests for ExecuteBuilds
// ---------------------------------------------------------------------------

func TestExecuteBuildsSkipsCached(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()
	bc.Record("python/pkg-a", "hash-a", "deps-a", "success")

	results := ExecuteBuilds(packages, graph, bc, map[string]string{"python/pkg-a": "hash-a"}, map[string]string{"python/pkg-a": "deps-a"}, false, false, 1)

	if results["python/pkg-a"].Status != "skipped" {
		t.Fatalf("expected skipped, got %s", results["python/pkg-a"].Status)
	}
}

func TestExecuteBuildsForceOverridesCache(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()
	bc.Record("python/pkg-a", "hash-a", "deps-a", "success")

	results := ExecuteBuilds(packages, graph, bc, map[string]string{"python/pkg-a": "hash-a"}, map[string]string{"python/pkg-a": "deps-a"}, true, false, 1)

	if results["python/pkg-a"].Status != "built" {
		t.Fatalf("expected built (force), got %s", results["python/pkg-a"].Status)
	}
}

func TestExecuteBuildsDryRun(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()

	results := ExecuteBuilds(packages, graph, bc, map[string]string{"python/pkg-a": "hash-a"}, map[string]string{"python/pkg-a": "deps-a"}, false, true, 1)

	if results["python/pkg-a"].Status != "would-build" {
		t.Fatalf("expected would-build, got %s", results["python/pkg-a"].Status)
	}
}

func TestExecuteBuildsDepSkipped(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "exit 1",
		"pkg-b/BUILD": "echo b",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"exit 1"}, Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), BuildCommands: []string{"echo b"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")
	graph.AddNode("python/pkg-b")
	graph.AddEdge("python/pkg-a", "python/pkg-b") // B depends on A

	bc := cache.New()

	results := ExecuteBuilds(packages, graph, bc,
		map[string]string{"python/pkg-a": "ha", "python/pkg-b": "hb"},
		map[string]string{"python/pkg-a": "da", "python/pkg-b": "db"},
		true, false, 1)

	if results["python/pkg-a"].Status != "failed" {
		t.Fatalf("expected pkg-a failed, got %s", results["python/pkg-a"].Status)
	}
	if results["python/pkg-b"].Status != "dep-skipped" {
		t.Fatalf("expected pkg-b dep-skipped, got %s", results["python/pkg-b"].Status)
	}
}

func TestExecuteBuildsParallelLevel(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
		"pkg-b/BUILD": "echo b",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
		{Name: "python/pkg-b", Path: filepath.Join(root, "pkg-b"), BuildCommands: []string{"echo b"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")
	graph.AddNode("python/pkg-b")
	// No edges — both at level 0, can run in parallel.

	bc := cache.New()

	results := ExecuteBuilds(packages, graph, bc,
		map[string]string{"python/pkg-a": "ha", "python/pkg-b": "hb"},
		map[string]string{"python/pkg-a": "da", "python/pkg-b": "db"},
		true, false, 2)

	if results["python/pkg-a"].Status != "built" {
		t.Fatalf("expected pkg-a built, got %s", results["python/pkg-a"].Status)
	}
	if results["python/pkg-b"].Status != "built" {
		t.Fatalf("expected pkg-b built, got %s", results["python/pkg-b"].Status)
	}
}

func TestExecuteBuildsCacheUpdatedOnSuccess(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "echo a",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"echo a"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()

	ExecuteBuilds(packages, graph, bc,
		map[string]string{"python/pkg-a": "hash-a"},
		map[string]string{"python/pkg-a": "deps-a"},
		true, false, 1)

	entries := bc.Entries()
	if entries["python/pkg-a"].Status != "success" {
		t.Fatalf("expected cache status success, got %s", entries["python/pkg-a"].Status)
	}
}

func TestExecuteBuildsCacheUpdatedOnFailure(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD": "exit 1",
	})

	packages := []discovery.Package{
		{Name: "python/pkg-a", Path: filepath.Join(root, "pkg-a"), BuildCommands: []string{"exit 1"}, Language: "python"},
	}

	graph := directedgraph.New()
	graph.AddNode("python/pkg-a")

	bc := cache.New()

	ExecuteBuilds(packages, graph, bc,
		map[string]string{"python/pkg-a": "hash-a"},
		map[string]string{"python/pkg-a": "deps-a"},
		true, false, 1)

	entries := bc.Entries()
	if entries["python/pkg-a"].Status != "failed" {
		t.Fatalf("expected cache status failed, got %s", entries["python/pkg-a"].Status)
	}
}
