// Tests for repo-root auto-detection.
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cigates"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func TestEmitBuildPlanCeilingFailureWritesNoPlanOrGateOutputs(t *testing.T) {
	patterns := make([]string, 20)
	files := make([]string, 10)
	for index := range patterns {
		patterns[index] = strings.Repeat(string(rune('a'+index)), 499)
	}
	for index := range files {
		files[index] = strings.Repeat(string(rune('0'+index)), 499)
	}
	files[len(files)-1] += "9"

	root := t.TempDir()
	registryPath := filepath.Join(root, "ci-gates.json")
	registry := cigates.Registry{SchemaVersion: 1, Gates: map[string]cigates.Gate{
		"bounded-job": {Description: "Boundary.", Paths: patterns},
	}}
	data, err := json.Marshal(registry)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(registryPath, data, 0o644); err != nil {
		t.Fatal(err)
	}
	planPath := filepath.Join(root, "plan.json")
	githubOutput := filepath.Join(root, "github-output.txt")
	const sentinel = "existing=true\n"
	if err := os.WriteFile(githubOutput, []byte(sentinel), 0o644); err != nil {
		t.Fatal(err)
	}
	t.Setenv("GITHUB_OUTPUT", githubOutput)

	code := emitBuildPlan(
		nil,
		directedgraph.New(),
		map[string]bool{},
		map[string]bool{},
		files,
		false,
		map[string]bool{},
		"origin/main",
		root,
		planPath,
		false,
		0,
		false,
		registryPath,
	)
	if code != 1 {
		t.Fatalf("emitBuildPlan code = %d, want 1", code)
	}
	if _, err := os.Stat(planPath); !os.IsNotExist(err) {
		t.Fatalf("plan file exists or stat failed unexpectedly: %v", err)
	}
	got, err := os.ReadFile(githubOutput)
	if err != nil {
		t.Fatal(err)
	}
	if string(got) != sentinel {
		t.Fatalf("GITHUB_OUTPUT changed on ceiling failure: %q", got)
	}
}

// TestFindRepoRootAcceptsGitDirectory covers the normal checkout case, where
// .git is a directory.
func TestFindRepoRootAcceptsGitDirectory(t *testing.T) {
	root := t.TempDir()
	if err := os.Mkdir(filepath.Join(root, ".git"), 0o755); err != nil {
		t.Fatalf("failed to create .git dir: %v", err)
	}
	nested := filepath.Join(root, "code", "packages", "go", "some-pkg")
	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatalf("failed to create nested dir: %v", err)
	}

	got := findRepoRoot(nested)
	want, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve absolute root: %v", err)
	}
	if got != want {
		t.Errorf("findRepoRoot(%q) = %q, want %q", nested, got, want)
	}
}

// TestFindRepoRootAcceptsGitWorktreeFile covers running from inside a git
// worktree, where .git is a regular file containing "gitdir: <path>" rather
// than a directory. Before this fix, findRepoRoot required .git to be a
// directory and would walk straight past a worktree root, silently
// resolving to whatever ancestor directory happened to contain a real .git
// directory (e.g. the main checkout the worktree was created from).
func TestFindRepoRootAcceptsGitWorktreeFile(t *testing.T) {
	root := t.TempDir()
	gitFile := filepath.Join(root, ".git")
	if err := os.WriteFile(gitFile, []byte("gitdir: /somewhere/else/.git/worktrees/example\n"), 0o644); err != nil {
		t.Fatalf("failed to create .git worktree file: %v", err)
	}
	nested := filepath.Join(root, "code", "packages", "go", "some-pkg")
	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatalf("failed to create nested dir: %v", err)
	}

	got := findRepoRoot(nested)
	want, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve absolute root: %v", err)
	}
	if got != want {
		t.Errorf("findRepoRoot(%q) = %q, want %q (worktree root, not an outer ancestor)", nested, got, want)
	}
}

// TestFindRepoRootReturnsEmptyWhenNoGitFound covers the case where no .git
// entry exists anywhere up the tree.
func TestFindRepoRootReturnsEmptyWhenNoGitFound(t *testing.T) {
	root := t.TempDir()
	nested := filepath.Join(root, "a", "b", "c")
	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatalf("failed to create nested dir: %v", err)
	}

	// t.TempDir() itself lives under the OS temp dir, which should not
	// contain a .git anywhere above it in practice; if it does, this test
	// would be unreliable, but that's not the case in CI or local dev.
	got := findRepoRoot(nested)
	if got != "" {
		t.Errorf("findRepoRoot(%q) = %q, want empty string", nested, got)
	}
}

func TestRunReportsStablePackageHashFailure(t *testing.T) {
	root := t.TempDir()
	packageDir := filepath.Join(root, "code", "packages", "go", "demo")
	if err := os.MkdirAll(packageDir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(packageDir, "BUILD"), []byte("echo building\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(packageDir, "main.go"), []byte("package demo\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	originalArgs := os.Args
	originalFlags := flag.CommandLine
	originalStderr := os.Stderr
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		os.Args = originalArgs
		flag.CommandLine = originalFlags
		os.Stderr = originalStderr
		reader.Close()
		writer.Close()
	})
	flag.CommandLine = flag.NewFlagSet("build-tool-test", flag.ContinueOnError)
	os.Args = []string{"build-tool", "-root", root, "-force", "-dry-run", "-validate-build-files=false"}
	os.Stderr = writer

	exitCode := runWithPackageHasher(func(pkg discovery.Package) (string, error) {
		if pkg.Name != "go/demo" {
			t.Fatalf("unexpected package passed to hasher: %s", pkg.Name)
		}
		return "", errors.New("host path intentionally withheld")
	})
	if err := writer.Close(); err != nil {
		t.Fatal(err)
	}
	os.Stderr = originalStderr
	stderr, err := io.ReadAll(reader)
	if err != nil {
		t.Fatal(err)
	}

	if exitCode != 2 {
		t.Fatalf("run exit code = %d, want 2; stderr=%s", exitCode, stderr)
	}
	want := "Error: HASH_PACKAGE_FAILED \"go/demo\"\n"
	if string(stderr) != want {
		t.Fatalf("stderr = %q, want %q", stderr, want)
	}
	if strings.Contains(string(stderr), root) || strings.Contains(string(stderr), "host path") {
		t.Fatalf("front door leaked hash failure details: %s", stderr)
	}
}

func TestFormatHashPackageErrorEscapesControlCharacters(t *testing.T) {
	got := formatHashPackageError("go/demo\n::error::forged\t\x1b")
	want := `Error: HASH_PACKAGE_FAILED "go/demo\n::error::forged\t\x1b"`
	if got != want {
		t.Fatalf("hash failure diagnostic = %q, want %q", got, want)
	}
	if strings.ContainsAny(got, "\n\r\t\x1b") {
		t.Fatalf("hash failure diagnostic contains a raw control character: %q", got)
	}
}
