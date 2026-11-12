package source

import (
	"bytes"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
)

func bytesReader(b []byte) io.Reader { return bytes.NewReader(b) }

func gitRepo(t *testing.T, files map[string]string) string {
	t.Helper()
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("git not installed")
	}
	dir := t.TempDir()
	writeTree(t, dir, files)
	for _, args := range [][]string{
		{"init", "-q"}, {"config", "user.email", "k@example.com"}, {"config", "user.name", "k"},
		{"config", "commit.gpgsign", "false"}, {"add", "-A"}, {"commit", "-q", "-m", "x"},
	} {
		cmd := exec.Command("git", append([]string{"-C", dir}, args...)...)
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, out)
		}
	}
	return dir
}

func TestGitDescribeAndUntracked(t *testing.T) {
	dir := gitRepo(t, map[string]string{"a.txt": "x", ".gitignore": "ignored.txt\n"})
	g := NewGit(dir)
	if u, err := g.Untracked(); err != nil || len(u) != 0 {
		t.Fatalf("clean tree: %v %v", u, err)
	}
	info, err := g.Describe("a.txt")
	if err != nil || info.Link || info.Exec {
		t.Fatalf("Describe(a.txt) = %+v, %v", info, err)
	}
	if _, err := g.Describe("nope"); err == nil {
		t.Fatal("untracked path described")
	}
	if _, err := g.Describe("a:b"); err == nil {
		t.Fatal("non-portable path described")
	}
	writeTree(t, dir, map[string]string{"ignored.txt": "i", "sub/new.txt": "n", "b.txt": "b"})
	u, err := g.Untracked()
	if err != nil || len(u) != 2 || u[0] != "b.txt" || u[1] != "sub/new.txt" {
		t.Fatalf("Untracked = %v, %v", u, err)
	}
}

func TestGitDescribeExecAndLink(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("exec bits and symlinks need Unix")
	}
	dir := gitRepo(t, map[string]string{"a.txt": "x"})
	if err := os.WriteFile(filepath.Join(dir, "run.sh"), []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink("a.txt", filepath.Join(dir, "link")); err != nil {
		t.Fatal(err)
	}
	if err := os.Mkdir(filepath.Join(dir, "d"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "d", "f"), []byte("f"), 0o644); err != nil {
		t.Fatal(err)
	}
	for _, args := range [][]string{{"add", "-A"}, {"commit", "-q", "-m", "y"}} {
		if out, err := exec.Command("git", append([]string{"-C", dir}, args...)...).CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v %s", args, err, out)
		}
	}
	g := NewGit(dir)
	if i, err := g.Describe("run.sh"); err != nil || !i.Exec || i.Link {
		t.Fatalf("run.sh = %+v, %v", i, err)
	}
	if i, err := g.Describe("link"); err != nil || !i.Link || i.Target != "a.txt" || i.Exec {
		t.Fatalf("link = %+v, %v", i, err)
	}
	f := NewFolder(dir)
	if i, err := f.Describe("run.sh"); err != nil || !i.Exec {
		t.Fatalf("folder run.sh = %+v, %v", i, err)
	}
	if i, err := f.Describe("link"); err != nil || !i.Link || i.Target != "a.txt" {
		t.Fatalf("folder link = %+v, %v", i, err)
	}
	if i, err := f.Describe("a.txt"); err != nil || i.Exec || i.Link {
		t.Fatalf("folder a.txt = %+v, %v", i, err)
	}
	if _, err := f.Describe("d"); err == nil {
		t.Fatal("directory described as a file")
	}
	if _, err := f.Describe("missing"); err == nil {
		t.Fatal("missing path described")
	}
	if _, err := f.Describe("a\\b"); err == nil {
		t.Fatal("non-portable path described")
	}
	// A dangling link is still a link: Readlink does not follow it.
	if err := os.Symlink("nowhere", filepath.Join(dir, "dangling")); err != nil {
		t.Fatal(err)
	}
	if i, err := f.Describe("dangling"); err != nil || !i.Link || i.Target != "nowhere" {
		t.Fatalf("dangling = %+v, %v", i, err)
	}
}

func TestGitDescribeNotARepo(t *testing.T) {
	g := NewGit(t.TempDir())
	if _, err := g.Describe("a"); err == nil {
		t.Fatal("non-repo described a path")
	}
	if _, err := g.Untracked(); err == nil {
		t.Fatal("non-repo reported cleanliness")
	}
}

func TestFolderDescribeMissingRoot(t *testing.T) {
	f := NewFolder(filepath.Join(t.TempDir(), "gone"))
	if _, err := f.Describe("a"); err == nil {
		t.Fatal("missing root accepted")
	}
}
