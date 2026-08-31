// Tests for repo-root auto-detection.
package main

import (
	"errors"
	"flag"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

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
