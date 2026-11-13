package record

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

func inventoryOf(t *testing.T, dir string) []string {
	t.Helper()
	inv, err := Inventory(dir)
	if err != nil {
		t.Fatal(err)
	}
	if inv.Schema != recfmt.SchemaInventory {
		t.Fatalf("schema %q", inv.Schema)
	}
	return inv.Paths
}

func TestInventoryCleanTreeIsEmptyAndNonNull(t *testing.T) {
	dir := newRepo(t, baseFiles)
	got := inventoryOf(t, dir)
	if got == nil || len(got) != 0 {
		t.Fatalf("want an empty non-nil list, got %#v", got)
	}
	// It must encode as [] not null: the formats refuse null lists.
	inv, _ := Inventory(dir)
	data, err := recfmt.EncodeInventory(inv)
	if err != nil || !strings.Contains(string(data), `"paths":[]`) {
		t.Fatalf("%s %v", data, err)
	}
}

func TestInventoryUntrackedFileBesideTrackedOnes(t *testing.T) {
	dir := newRepo(t, baseFiles)
	writeFile(t, dir, "src/scratch.txt", "x")
	writeFile(t, dir, "src/deep/more.txt", "x")
	writeFile(t, dir, "top.txt", "x")
	want := []string{"src/deep/more.txt", "src/scratch.txt", "top.txt"}
	if got := inventoryOf(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestInventoryCollapsesAnAllUntrackedDirectory(t *testing.T) {
	dir := newRepo(t, baseFiles)
	// A big untracked tree under a tracked directory, and one at the top.
	for _, p := range []string{"src/target/debug/a.o", "src/target/debug/b.o", "src/target/release/c.o", "build/x/y/z"} {
		writeFile(t, dir, p, "x")
	}
	want := []string{"build", "src/target"}
	if got := inventoryOf(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestInventoryListsEmptyUntrackedDirectoriesAndIgnoredFiles(t *testing.T) {
	files := map[string]string{"keep/a": "a", ".gitignore": "*.log\ntarget/\n"}
	dir := newRepo(t, files)
	if err := os.MkdirAll(filepath.Join(dir, "keep", "emptydir"), 0o755); err != nil {
		t.Fatal(err)
	}
	// Ignored files are exactly the leftovers the inventory exists to name.
	writeFile(t, dir, "keep/build.log", "x")
	writeFile(t, dir, "target/out", "x")
	want := []string{"keep/build.log", "keep/emptydir", "target"}
	if got := inventoryOf(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestInventorySkipsRootDotGitOnly(t *testing.T) {
	dir := newRepo(t, baseFiles)
	// A nested .git directory is ordinary untracked content (collapsed).
	writeFile(t, dir, "src/.git/config", "x")
	got := inventoryOf(t, dir)
	if want := []string{"src/.git"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
	for _, p := range got {
		if p == ".git" || strings.HasPrefix(p, ".git/") {
			t.Fatalf("the root .git leaked: %v", got)
		}
	}
}

func TestInventorySkipsDotGitFileOfALinkedWorktree(t *testing.T) {
	dir := newRepo(t, baseFiles)
	wt := filepath.Join(t.TempDir(), "wt")
	runGit(t, dir, "worktree", "add", "-q", "-b", "side", wt)
	if fi, err := os.Lstat(filepath.Join(wt, ".git")); err != nil || fi.IsDir() {
		t.Skip("this git does not make a .git file for linked worktrees")
	}
	if got := inventoryOf(t, wt); len(got) != 0 {
		t.Fatalf("the .git file was listed: %v", got)
	}
}

func TestInventoryDoesNotEnterGitlinks(t *testing.T) {
	dir := newRepo(t, baseFiles)
	commit := runGit(t, dir, "rev-parse", "HEAD")
	runGit(t, dir, "update-index", "--add", "--cacheinfo", "160000,"+commit+",vendor/sub")
	runGit(t, dir, "commit", "-q", "-m", "gitlink")
	writeFile(t, dir, "vendor/sub/inside.txt", "x")
	writeFile(t, dir, "vendor/other.txt", "x")
	want := []string{"vendor/other.txt"}
	if got := inventoryOf(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestInventoryNeverFollowsSymlinks(t *testing.T) {
	dir := newRepo(t, baseFiles)
	outside := t.TempDir()
	writeFile(t, outside, "secret.txt", "x")
	writeFile(t, outside, "deeper/more.txt", "x")
	symlink(t, dir, "src/escape", outside)        // untracked link out of the tree
	symlink(t, dir, "src/loop", ".")              // a link to its own directory
	symlink(t, dir, "src/deep/up", "..")          // a link to an ancestor
	symlink(t, dir, "selfloop", "selfloop")       // a link to itself
	symlink(t, dir, "pair-a", "pair-b")           // a two-link cycle
	symlink(t, dir, "pair-b", "pair-a")           //
	symlink(t, dir, "dangling", "does-not-exist") // a broken link
	want := []string{"dangling", "pair-a", "pair-b", "selfloop", "src/deep/up", "src/escape", "src/loop"}
	if got := inventoryOf(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestInventoryOrderIsByteOrderAndDeterministic(t *testing.T) {
	dir := newRepo(t, map[string]string{"a/x": "x", "m/y": "y"})
	// "a.b" < "a/c" < "a0" in byte order ('.' 0x2e, '/' 0x2f, '0' 0x30), a
	// different order from walking directory by directory.
	for _, p := range []string{"a.b", "a/c", "a0", "m/Z", "m/a", "B"} {
		writeFile(t, dir, p, "x")
	}
	want := []string{"B", "a.b", "a/c", "a0", "m/Z", "m/a"}
	first := inventoryOf(t, dir)
	if !reflect.DeepEqual(first, want) {
		t.Fatalf("got %v, want %v", first, want)
	}
	for i := 0; i < 5; i++ {
		if again := inventoryOf(t, dir); !reflect.DeepEqual(again, first) {
			t.Fatalf("run %d differs: %v", i, again)
		}
	}
	// And the formats accept it: sorted, unique, valid paths.
	inv, _ := Inventory(dir)
	data, err := recfmt.EncodeInventory(inv)
	if err != nil {
		t.Fatal(err)
	}
	back, err := recfmt.DecodeInventory(data)
	if err != nil || !reflect.DeepEqual(back.Paths, want) {
		t.Fatalf("%v %v", back, err)
	}
}

func TestInventoryTrackedChangesAreNotInventory(t *testing.T) {
	// A modified, deleted or type-changed TRACKED path is VerifyTree's
	// business; the inventory is only about what the commit does not hold.
	dir := newRepo(t, baseFiles)
	writeFile(t, dir, "README.md", "changed")
	remove(t, dir, "src/a.go")
	if got := inventoryOf(t, dir); len(got) != 0 {
		t.Fatalf("got %v", got)
	}
}

func TestInventoryErrors(t *testing.T) {
	requireGit(t)
	if _, err := Inventory(t.TempDir()); err == nil {
		t.Fatal("not a repository")
	}
}
