// Tests for repo-root auto-detection.
package main

import (
	"os"
	"path/filepath"
	"testing"
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
