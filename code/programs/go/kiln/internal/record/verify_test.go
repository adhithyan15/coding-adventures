package record

import (
	"context"
	"crypto/sha1"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

var baseFiles = map[string]string{
	"README.md":       "hello\n",
	"src/a.go":        "package a\n",
	"src/deep/b.go":   "package b\n",
	"with space/x y":  "odd name\n",
	"docs/guide.txt":  "line one\nline two\n",
	"empty":           "",
	"src/deep/c/d.go": "package d\n",
}

// wantTreeError fails unless err is a *TreeError mentioning every fragment.
func wantTreeError(t *testing.T, err error, fragments ...string) *TreeError {
	t.Helper()
	var te *TreeError
	if !errors.As(err, &te) {
		t.Fatalf("want a *TreeError, got %v", err)
	}
	for _, f := range fragments {
		if !strings.Contains(err.Error(), f) {
			t.Errorf("error does not mention %q:\n%v", f, err)
		}
	}
	return te
}

func TestVerifyTreeCleanRepository(t *testing.T) {
	dir := newRepo(t, baseFiles)
	res, err := VerifyTree(dir, "")
	if err != nil || res.Commit != runGit(t, dir, "rev-parse", "HEAD") {
		t.Fatalf("clean tree: %v %v", res, err)
	}
	// Pinned to the commit it just returned: still fine.
	if _, err := VerifyTree(dir, res.Commit); err != nil {
		t.Fatal(err)
	}
}

func TestVerifyTreeIgnoresUntrackedFiles(t *testing.T) {
	// VerifyTree is about TRACKED content; untracked files are the
	// inventory's business.
	dir := newRepo(t, baseFiles)
	writeFile(t, dir, "scratch.txt", "x")
	writeFile(t, dir, "newdir/y", "y")
	if _, err := VerifyTree(dir, ""); err != nil {
		t.Fatal(err)
	}
}

func TestVerifyTreeModifiedByte(t *testing.T) {
	dir := newRepo(t, baseFiles)
	// Same length, one byte different: only the hash can tell.
	writeFile(t, dir, "src/a.go", "package b\n")
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "src/a.go", "content differs")
}

func TestVerifyTreeEmptyAndSizeChanges(t *testing.T) {
	dir := newRepo(t, baseFiles)
	writeFile(t, dir, "empty", "now not empty")
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "empty", "content differs")
}

func TestVerifyTreeMissingFileAndDirectory(t *testing.T) {
	dir := newRepo(t, baseFiles)
	remove(t, dir, "README.md")
	remove(t, dir, "src/deep")
	_, err := VerifyTree(dir, "")
	te := wantTreeError(t, err, "README.md: missing on disk", "src/deep")
	if te.Total != 3 { // README.md, src/deep/b.go, src/deep/c/d.go
		t.Errorf("total = %d, want 3:\n%v", te.Total, err)
	}
}

func TestVerifyTreeIndexDiffersFromHead(t *testing.T) {
	dir := newRepo(t, baseFiles)
	// A staged edit: disk and index agree, HEAD does not.
	writeFile(t, dir, "README.md", "edited\n")
	runGit(t, dir, "add", "README.md")
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "README.md", "index has")

	// A staged new file.
	dir = newRepo(t, baseFiles)
	writeFile(t, dir, "new.txt", "n")
	runGit(t, dir, "add", "new.txt")
	_, err = VerifyTree(dir, "")
	wantTreeError(t, err, "new.txt", "in the index but not in HEAD")

	// A file removed from the index only.
	dir = newRepo(t, baseFiles)
	runGit(t, dir, "rm", "--cached", "-q", "README.md")
	_, err = VerifyTree(dir, "")
	wantTreeError(t, err, "README.md", "in HEAD but not in the index")

	// A mode change staged in the index.
	dir = newRepo(t, baseFiles)
	runGit(t, dir, "update-index", "--chmod=+x", "README.md")
	_, err = VerifyTree(dir, "")
	wantTreeError(t, err, "README.md", "index has 100755")
}

func TestVerifyTreeUnmergedIndexEntry(t *testing.T) {
	dir := newRepo(t, baseFiles)
	blob := runGit(t, dir, "rev-parse", "HEAD:README.md")
	runGit(t, dir, "update-index", "--force-remove", "README.md")
	// Stages 1 (base), 2 (ours) and 3 (theirs) are what a conflicted merge
	// leaves in the index.
	runGitStdin(t, dir, fmt.Sprintf("100644 %s 1\tREADME.md\n100644 %s 2\tREADME.md\n", blob, blob),
		"update-index", "--index-info")
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "README.md", "stage")
}

func TestVerifyTreeSkipWorktreeAndAssumeUnchanged(t *testing.T) {
	dir := newRepo(t, baseFiles)
	runGit(t, dir, "update-index", "--skip-worktree", "README.md")
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "README.md", "skip-worktree")

	dir = newRepo(t, baseFiles)
	runGit(t, dir, "update-index", "--assume-unchanged", "src/a.go")
	// The file is also edited: Git would not notice, VerifyTree must refuse
	// on the bit alone.
	writeFile(t, dir, "src/a.go", "package zzz\n")
	_, err = VerifyTree(dir, "")
	wantTreeError(t, err, "src/a.go", "assume-unchanged")
}

func TestVerifyTreeSparseCheckoutConfig(t *testing.T) {
	for _, spelling := range []string{"true", "yes", "on", "1"} {
		dir := newRepo(t, baseFiles)
		runGit(t, dir, "config", "core.sparseCheckout", spelling)
		_, err := VerifyTree(dir, "")
		wantTreeError(t, err, "core.sparseCheckout")
	}
	// Explicitly false is fine.
	dir := newRepo(t, baseFiles)
	runGit(t, dir, "config", "core.sparseCheckout", "false")
	if _, err := VerifyTree(dir, ""); err != nil {
		t.Fatal(err)
	}
}

func TestVerifyTreeEolCrlfChangesBytes(t *testing.T) {
	dir := newRepo(t, map[string]string{
		".gitattributes": "*.txt text eol=crlf\n",
		"a.txt":          "one\ntwo\n",
		"b.bin":          "keep\n",
	})
	// Re-checkout a.txt: Git smudges LF to CRLF on the way out, so the disk
	// bytes no longer hash to the committed blob.
	remove(t, dir, "a.txt")
	runGit(t, dir, "checkout", "--", "a.txt")
	data, err := os.ReadFile(filepath.Join(dir, "a.txt"))
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(data), "\r\n") {
		t.Skip("this git did not apply eol=crlf on checkout")
	}
	_, err = VerifyTree(dir, "")
	wantTreeError(t, err, "a.txt", "content differs")
}

func TestVerifyTreeReplacedBySymlinkAndDirectory(t *testing.T) {
	dir := newRepo(t, baseFiles)
	remove(t, dir, "README.md")
	symlink(t, dir, "README.md", "src/a.go")
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "README.md", "symlink")

	dir = newRepo(t, baseFiles)
	remove(t, dir, "README.md")
	if err := os.Mkdir(filepath.Join(dir, "README.md"), 0o755); err != nil {
		t.Fatal(err)
	}
	_, err = VerifyTree(dir, "")
	wantTreeError(t, err, "README.md", "a directory", "non-regular")
}

func TestVerifyTreeSymlinkRetargetedAndIntact(t *testing.T) {
	dir := newRepo(t, baseFiles)
	symlink(t, dir, "link", "src/a.go")
	runGit(t, dir, "add", "link")
	runGit(t, dir, "commit", "-q", "-m", "link")
	if _, err := VerifyTree(dir, ""); err != nil {
		t.Fatalf("an intact symlink: %v", err)
	}

	remove(t, dir, "link")
	symlink(t, dir, "link", "src/deep/b.go")
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "link", "symlink target differs")

	// A symlink replaced by a regular file.
	remove(t, dir, "link")
	writeFile(t, dir, "link", "src/a.go")
	_, err = VerifyTree(dir, "")
	wantTreeError(t, err, "link", "tracked as a symlink but is an irregular file")
}

func TestVerifyTreeAncestorDirectoryReplacedBySymlink(t *testing.T) {
	// `src` becomes a link to a directory that has files of the same names
	// and the same contents. Every lstat of `src/a.go` succeeds and every
	// hash matches; only the ancestor check can see this.
	dir := newRepo(t, baseFiles)
	mirror := t.TempDir()
	for p, c := range baseFiles {
		if strings.HasPrefix(p, "src/") {
			writeFile(t, mirror, strings.TrimPrefix(p, "src/"), c)
		}
	}
	remove(t, dir, "src")
	symlink(t, dir, "src", mirror)
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "src/a.go", "not a real directory")
}

func TestVerifyTreeAncestorMissingIsCachedOnce(t *testing.T) {
	dir := newRepo(t, baseFiles)
	remove(t, dir, "src")
	_, err := VerifyTree(dir, "")
	te := wantTreeError(t, err, "directory src is missing")
	if te.Total != 3 {
		t.Fatalf("total = %d", te.Total)
	}
}

func TestVerifyTreeNamesAtMostSixteenPaths(t *testing.T) {
	files := map[string]string{}
	for i := 0; i < 20; i++ {
		files[fmt.Sprintf("f%02d.txt", i)] = "x"
	}
	dir := newRepo(t, files)
	for p := range files {
		writeFile(t, dir, p, "y")
	}
	_, err := VerifyTree(dir, "")
	te := wantTreeError(t, err, "20 problems", "... and 4 more")
	if len(te.Problems) != MaxReported || te.Total != 20 {
		t.Fatalf("problems = %d, total = %d", len(te.Problems), te.Total)
	}
	// Sorted: the first sixteen names, whatever order the workers finished.
	if !strings.HasPrefix(te.Problems[0], "f00.txt") || !strings.HasPrefix(te.Problems[15], "f15.txt") {
		t.Fatalf("not the first sixteen in order: %v", te.Problems)
	}
	if strings.Contains(err.Error(), "f16.txt") {
		t.Fatal("named a seventeenth path")
	}
}

func TestVerifyTreeSingularProblemWording(t *testing.T) {
	dir := newRepo(t, baseFiles)
	writeFile(t, dir, "src/a.go", "package q\n")
	_, err := VerifyTree(dir, "")
	if !strings.Contains(err.Error(), "1 problem\n") {
		t.Fatalf("%q", err)
	}
}

func TestVerifyTreeSHA256Repository(t *testing.T) {
	requireGit(t)
	probe := t.TempDir()
	if out, err := tryGit(probe, "init", "-q", "--object-format=sha256"); err != nil {
		t.Skipf("this git cannot make SHA-256 repositories: %s", out)
	}
	dir := newRepo(t, baseFiles, "--object-format=sha256")
	res, err := VerifyTree(dir, "")
	if err != nil || len(res.Commit) != 64 {
		t.Fatalf("clean SHA-256 tree: %v %v", res, err)
	}
	writeFile(t, dir, "src/a.go", "package b\n")
	_, err = VerifyTree(dir, "")
	wantTreeError(t, err, "src/a.go", "content differs")
}

func TestVerifyTreeNotARepository(t *testing.T) {
	requireGit(t)
	if _, err := VerifyTree(t.TempDir(), ""); err == nil {
		t.Fatal("a directory that is not a repository was accepted")
	}
}

func TestVerifyTreeNoCommits(t *testing.T) {
	requireGit(t)
	dir := t.TempDir()
	runGit(t, dir, "init", "-q")
	if _, err := VerifyTree(dir, ""); err == nil {
		t.Fatal("a repository without HEAD was accepted")
	}
}

func TestVerifyTreeFileSizeCap(t *testing.T) {
	dir := newRepo(t, baseFiles)
	saved := maxFileBytes
	maxFileBytes = 10
	defer func() { maxFileBytes = saved }()
	_, err := VerifyTree(dir, "")
	wantTreeError(t, err, "over the 10 byte limit")
}

func TestVerifyTreeBudget(t *testing.T) {
	dir := newRepo(t, baseFiles)
	saved := budget
	budget = time.Nanosecond
	defer func() { budget = saved }()
	_, err := VerifyTree(dir, "")
	if err == nil || !strings.Contains(err.Error(), "budget") {
		t.Fatalf("an exhausted budget must be an error, got %v", err)
	}
}

func TestVerifyTreeParallelismDoesNotChangeTheAnswer(t *testing.T) {
	files := map[string]string{}
	for i := 0; i < 60; i++ {
		files[fmt.Sprintf("d%d/f%d", i%7, i)] = fmt.Sprint(i)
	}
	dir := newRepo(t, files)
	saved := workers
	defer func() { workers = saved }()
	for _, w := range []int{1, 3, 8} {
		workers = w
		if _, err := VerifyTree(dir, ""); err != nil {
			t.Fatalf("%d workers: %v", w, err)
		}
	}
}

// --- units that need no repository -----------------------------------------

func TestHashBodyDetectsShrinkAndGrowth(t *testing.T) {
	good := func(body string, size int64) string {
		return hashBody(sha1.New(), strings.NewReader(body), size)
	}
	if why := good("abc", 3); why != "" {
		t.Fatal(why)
	}
	if why := good("ab", 3); !strings.Contains(why, "shrank") {
		t.Fatalf("shrink: %q", why)
	}
	if why := good("abcd", 3); !strings.Contains(why, "grew") {
		t.Fatalf("growth: %q", why)
	}
	if why := hashBody(sha1.New(), errReader{}, 3); !strings.Contains(why, "cannot be read") {
		t.Fatalf("read error: %q", why)
	}
}

type errReader struct{}

func (errReader) Read([]byte) (int, error) { return 0, errors.New("boom") }

func TestCtxReaderStopsAtTheDeadline(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	r := &ctxReader{ctx: ctx, r: strings.NewReader("abc")}
	buf := make([]byte, 2)
	if n, err := r.Read(buf); err != nil || n != 2 {
		t.Fatalf("%d %v", n, err)
	}
	cancel()
	if _, err := r.Read(buf); err == nil {
		t.Fatal("read after cancel succeeded")
	}
}

func TestNewHasherByIDLength(t *testing.T) {
	if h, err := newHasher(strings.Repeat("a", 40)); err != nil || h.Size() != 20 {
		t.Fatalf("sha1: %v", err)
	}
	if h, err := newHasher(strings.Repeat("a", 64)); err != nil || h.Size() != 32 {
		t.Fatalf("sha256: %v", err)
	}
	if _, err := newHasher("abc"); err == nil {
		t.Fatal("a 3-digit id was accepted")
	}
}

func TestCheckRefusesUnsupportedModeAndShortID(t *testing.T) {
	dir := t.TempDir()
	writeFile(t, dir, "f", "x")
	v := &verifier{root: dir, ctx: context.Background(), dirs: map[string]error{}}
	if why := v.check("f", entry{mode: "100664", id: strings.Repeat("a", 40)}); !strings.Contains(why, "unsupported mode") {
		t.Fatalf("%q", why)
	}
	if why := v.check("f", entry{mode: modeFile, id: "abc"}); !strings.Contains(why, "neither SHA-1 nor SHA-256") {
		t.Fatalf("%q", why)
	}
	if why := v.check("nope", entry{mode: modeFile, id: strings.Repeat("a", 40)}); !strings.Contains(why, "missing") {
		t.Fatalf("%q", why)
	}
	// A symlink entry whose id is not an object id.
	symlink(t, dir, "l", "f")
	if why := v.check("l", entry{mode: modeSymlink, id: "abc"}); !strings.Contains(why, "neither") {
		t.Fatalf("%q", why)
	}
}

func TestTreeErrorAndPrintable(t *testing.T) {
	if got := printable("a\nb\x00c"); got != "a?b?c" {
		t.Fatalf("%q", got)
	}
	if got := printable(strings.Repeat("é", 300)); !strings.HasSuffix(got, "...") || len([]rune(got)) != 203 {
		t.Fatalf("%d runes", len([]rune(got)))
	}
	var p problems
	if p.err("x") != nil {
		t.Fatal("no problems must be a nil error")
	}
	p.add("b", "why")
	p.add("a", "why2")
	p.add("a", "why1")
	e := p.err("tree").(*TreeError)
	if e.Problems[0] != "a: why1" || e.Problems[1] != "a: why2" || e.Problems[2] != "b: why" {
		t.Fatalf("%v", e.Problems)
	}
	if short("0123456789abcdef") != "0123456789ab" || short("ab") != "ab" {
		t.Fatal("short")
	}
}

// modeInfo is an os.FileInfo that only knows its mode.
type modeInfo struct{ m os.FileMode }

func (m modeInfo) Name() string       { return "x" }
func (m modeInfo) Size() int64        { return 0 }
func (m modeInfo) Mode() os.FileMode  { return m.m }
func (m modeInfo) ModTime() time.Time { return time.Time{} }
func (m modeInfo) IsDir() bool        { return m.m.IsDir() }
func (m modeInfo) Sys() any           { return nil }

func TestKindNamesEveryIrregularFile(t *testing.T) {
	for mode, want := range map[os.FileMode]string{
		os.ModeDir:        "a directory",
		os.ModeNamedPipe:  "a FIFO",
		os.ModeSocket:     "a socket",
		os.ModeDevice:     "a device",
		os.ModeSymlink:    "a symlink",
		os.ModeIrregular:  "an irregular file",
		os.ModeCharDevice: "an irregular file",
	} {
		// ModeCharDevice alone (without ModeDevice) is not a real mode, so
		// it lands in the fallback; the others are exact.
		if got := kind(modeInfo{mode}); got != want {
			t.Errorf("kind(%v) = %q, want %q", mode, got, want)
		}
	}
}

func TestSplitNUL(t *testing.T) {
	if got := splitNUL([]byte("a\x00\x00b\x00")); !reflect.DeepEqual(got, []string{"a", "b"}) {
		t.Fatalf("%q", got)
	}
}
