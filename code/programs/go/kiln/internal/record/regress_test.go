package record

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Regression tests for the review of PR 2a (Fable and the security review).
// Each TestPoC* is the proof of concept the reviewers wrote, turned around so
// it fails if the hole comes back.

// A `git replace` ref makes `HEAD` name a commit whose content is something
// else. The disk here matches the REPLACEMENT, not the commit, and VerifyTree
// used to pass.
func TestPoCReplaceRef(t *testing.T) {
	dir := newRepo(t, map[string]string{"a.txt": "one\n"})
	c1 := runGit(t, dir, "rev-parse", "HEAD")
	writeFile(t, dir, "a.txt", "EVIL\n")
	runGit(t, dir, "add", "-A")
	tree2 := runGit(t, dir, "write-tree")
	c2 := runGit(t, dir, "commit-tree", tree2, "-m", "evil")
	runGit(t, dir, "replace", c1, c2)
	runGit(t, dir, "reset", "-q", "--hard", "HEAD") // with replacement active: disk = EVIL
	if got := runGit(t, dir, "rev-parse", "HEAD"); got != c1 {
		t.Fatalf("HEAD moved to %s", got)
	}
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "a.txt", "index has")
}

// A BUILD that ran `git commit -a` leaves a tree that equals its (new) HEAD.
// Only the pin to the pre-build commit catches it.
func TestPostBuildCommitMovesHead(t *testing.T) {
	dir := newRepo(t, baseFiles)
	res, err := VerifyTree(dir, "")
	if err != nil {
		t.Fatal(err)
	}
	writeFile(t, dir, "src/a.go", "package evil\n")
	runGit(t, dir, "commit", "-q", "-a", "-m", "by a BUILD")
	if _, err := VerifyTree(dir, ""); err != nil {
		t.Fatalf("an unpinned check sees a consistent tree: %v", err)
	}
	_, err = VerifyTree(dir, res.Commit)
	if err == nil || !strings.Contains(err.Error(), "the commit moved") {
		t.Fatalf("pinned check: %v", err)
	}
}

// A subdirectory of a repository is not a checkout: only its subtree would be
// checked. The same goes for a directory without a .git of its own that sits
// inside a parent repository.
func TestPoCSubdirRoot(t *testing.T) {
	dir := newRepo(t, map[string]string{"top.txt": "one\n", "sub/in.txt": "x\n"})
	writeFile(t, dir, "top.txt", "MODIFIED\n")
	for _, root := range []string{filepath.Join(dir, "sub")} {
		if _, err := VerifyTree(root, ""); err == nil || !strings.Contains(err.Error(), "top level") {
			t.Errorf("VerifyTree(%s): %v", root, err)
		}
		if _, err := Inventory(root); err == nil || !strings.Contains(err.Error(), "top level") {
			t.Errorf("Inventory(%s): %v", root, err)
		}
	}
	// A plain directory nested in the parent repository, with no .git.
	nested := filepath.Join(dir, "nested")
	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatal(err)
	}
	if _, err := VerifyTree(nested, ""); err == nil {
		t.Error("a root without its own .git, inside a parent repository, was accepted")
	}
	if _, err := Inventory(nested); err == nil {
		t.Error("Inventory accepted the same")
	}
}

// A submodule's contents are not verified, so a tree that has one is refused,
// whatever stands at its path.
func TestPoCGitlinkFile(t *testing.T) {
	for name, setup := range map[string]func(t *testing.T, dir string){
		"regular file":    func(t *testing.T, dir string) { writeFile(t, dir, "pkg/sub", "anything\n") },
		"empty directory": func(t *testing.T, dir string) { mkdir(t, dir, "pkg/sub") },
		"absent":          func(t *testing.T, dir string) { mkdir(t, dir, "pkg") },
		"directory with files": func(t *testing.T, dir string) {
			writeFile(t, dir, "pkg/sub/leftover.txt", "x")
		},
	} {
		dir := newRepo(t, map[string]string{"a.txt": "one\n"})
		c := runGit(t, dir, "rev-parse", "HEAD")
		runGit(t, dir, "update-index", "--add", "--cacheinfo", "160000,"+c+",pkg/sub")
		runGit(t, dir, "commit", "-q", "-m", "gitlink")
		setup(t, dir)
		_, err := VerifyTree(dir, "")
		if err == nil || !strings.Contains(err.Error(), "submodule-unsupported") || !strings.Contains(err.Error(), "pkg/sub") {
			t.Errorf("%s: VerifyTree: %v", name, err)
		}
		if _, err := Inventory(dir); err == nil || !strings.Contains(err.Error(), "submodule-unsupported") {
			t.Errorf("%s: Inventory: %v", name, err)
		}
	}
}

func mkdir(t *testing.T, dir, rel string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Join(dir, filepath.FromSlash(rel)), 0o755); err != nil {
		t.Fatal(err)
	}
}

// A tracked file replaced by a directory: the inventory used to skip it and
// everything inside it.
func TestPoCInventoryTrackedReplacedByDir(t *testing.T) {
	dir := newRepo(t, map[string]string{"p/f": "one\n", "p/g": "two\n"})
	remove(t, dir, "p/f")
	writeFile(t, dir, "p/f/hidden.bin", "x")
	_, err := Inventory(dir)
	if err == nil || !strings.Contains(err.Error(), "p/f") || !strings.Contains(err.Error(), "directory") {
		t.Fatalf("got %v", err)
	}
}

func TestInventoryStopsAtTheLimit(t *testing.T) {
	dir := newRepo(t, baseFiles)
	for _, n := range []string{"u1", "u2", "u3"} {
		writeFile(t, dir, n, "x")
	}
	saved := inventoryLimit
	inventoryLimit = 2
	defer func() { inventoryLimit = saved }()
	if _, err := Inventory(dir); err == nil || !strings.Contains(err.Error(), "more than 2 entries") {
		t.Fatalf("got %v", err)
	}
}

func TestParsersRefuseMalformedGitOutput(t *testing.T) {
	id := strings.Repeat("a", 40)
	if m, err := parseTree([]byte("100644 blob " + id + "\ta.txt\x00")); err != nil || m["a.txt"].id != id {
		t.Fatalf("%v %v", m, err)
	}
	for name, out := range map[string]string{
		"no tab":      "100644 blob " + id + " a.txt\x00",
		"two fields":  "100644 " + id + "\ta.txt\x00",
		"four fields": "100644 blob " + id + " x\ta.txt\x00",
		"duplicate":   "100644 blob " + id + "\ta\x00100644 blob " + id + "\ta\x00",
		"forged name": "100644 blob " + id + "\t.git/config\x00",
	} {
		if _, err := parseTree([]byte(out)); err == nil {
			t.Errorf("parseTree accepted %s", name)
		}
	}
	for name, out := range map[string]string{
		"no tab":    "100644 " + id + " 0 a\x00",
		"two":       "100644 " + id + "\ta\x00",
		"empty":     "100644 " + id + " 0\t\x00",
		"stage 2":   "100644 " + id + " 2\ta\x00",
		"duplicate": "100644 " + id + " 0\ta\x00100644 " + id + " 0\ta\x00",
	} {
		if _, err := parseIndex([]byte(out)); err == nil {
			t.Errorf("parseIndex accepted %s", name)
		}
	}
	var bad problems
	for name, out := range map[string]string{
		"short":    "H\x00",
		"no space": "Hxa\x00",
		"count":    "H a\x00H b\x00",
	} {
		if err := parseHidden([]byte(out), 1, &bad); err == nil {
			t.Errorf("parseHidden accepted %s", name)
		}
	}
	bad = problems{}
	if err := parseHidden([]byte("? x\x00"), 1, &bad); err != nil || len(bad.list) != 1 {
		t.Fatalf("an unexpected tag must be a problem: %v %v", err, bad.list)
	}
	if isHex("ABC") || !isHex("0f9a") {
		t.Fatal("isHex")
	}
}

func TestCheckTreePath(t *testing.T) {
	for _, bad := range []string{"", "/abs", "a//b", "a/", "./a", "a/./b", "../a", "a/../b", "a\\b", "a\x00b",
		".git", ".GIT/config", "a/.Git/x", "a/.git"} {
		if checkTreePath(bad) == "" {
			t.Errorf("%q accepted", bad)
		}
	}
	for _, ok := range []string{"a", "a/b", ".gitignore", ".github/x", "a b/c d", "x.git/y", "..a/b", "dir/.gitattributes"} {
		if why := checkTreePath(ok); why != "" {
			t.Errorf("%q refused: %s", ok, why)
		}
	}
}

// A tree object forged with plumbing can hold names `git add` would refuse.
func TestForgedTreeNamesAreRefused(t *testing.T) {
	for _, name := range []string{".git", ".GIT", "a\\b"} {
		dir := newRepo(t, map[string]string{"a.txt": "one\n"})
		blob := runGit(t, dir, "rev-parse", "HEAD:a.txt")
		cmd := "100644 blob " + blob + "\t" + name + "\n"
		tree, err := tryGitStdin(dir, cmd, "mktree")
		if err != nil {
			t.Skipf("this git will not forge %q: %s", name, tree)
		}
		commit := runGit(t, dir, "commit-tree", tree, "-m", "forged")
		runGit(t, dir, "update-ref", "HEAD", commit)
		_, verr := VerifyTree(dir, "")
		if verr == nil || strings.Contains(verr.Error(), "working tree is not HEAD") {
			t.Errorf("%q: want a tree-path refusal, got %v", name, verr)
		}
	}
}
