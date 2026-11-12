package source

import (
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"
	"time"
)

func TestMatch(t *testing.T) {
	cases := []struct {
		pat, path string
		want      bool
	}{
		{"a.go", "a.go", true},
		{"a.go", "b.go", false},
		{"*.go", "a.go", true},
		{"*.go", "x/a.go", false},
		{"src/*.go", "src/a.go", true},
		{"src/*.go", "src/x/a.go", false},
		{"**/*.go", "a.go", true},
		{"**/*.go", "x/y/a.go", true},
		{"**/*.go", "x/y/a.rs", false},
		{"src/**", "src/a", true},
		{"src/**", "src/x/y", true},
		{"src/**", "src", false},
		{"src/**", "other/a", false},
		{"**", "anything/at/all", true},
		{"a/**/b", "a/b", true},
		{"a/**/b", "a/x/y/b", true},
		{"a/**/b", "a/x/c", false},
		{"a/**/**/b", "a/b", true},
		{"a?c", "abc", true},
		{"a?c", "ac", false},
		{"a?c", "a/c", false},
		{"a*c", "a/c", false},
		{"a*b*c", "aXXbYYc", true},
		{"a*b*c", "aXXbYY", false},
		{"*", "", true},
		{"*é", "café", true},
		{"?", "é", true},
		{"a/b", "a", false},
		{"a", "a/b", false},
		{"a**b", "axxb", true},
		{"a**b", "a/b", false},
	}
	for _, c := range cases {
		if got := Match(c.pat, c.path); got != c.want {
			t.Errorf("Match(%q,%q)=%v want %v", c.pat, c.path, got, c.want)
		}
	}
}

func TestValidatePattern(t *testing.T) {
	for _, p := range []string{"", "/a", "a//b", "a/", "../a", "a/../b"} {
		if ValidatePattern(p) == nil {
			t.Errorf("%q should be invalid", p)
		}
	}
	for _, p := range []string{"a", "**/*.go", "a b/c*"} {
		if err := ValidatePattern(p); err != nil {
			t.Errorf("%q: %v", p, err)
		}
	}
}

// writeTree creates files (rel path -> content) under dir.
func writeTree(t *testing.T, dir string, files map[string]string) {
	t.Helper()
	for p, c := range files {
		full := filepath.Join(dir, filepath.FromSlash(p))
		if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(full, []byte(c), 0o644); err != nil {
			t.Fatal(err)
		}
	}
}

func sampleTree() map[string]string {
	return map[string]string{
		"README.md":         "hi",
		"src/a.go":          "package a",
		"src/deep/b.go":     "package b",
		"docs/guide.md":     "guide",
		"with space/x y.go": "x",
		"-dash.txt":         "d",
		"üñí/ç.txt":         "u",
		"new\nline.txt":     "nl",
	}
}

func skipIfNoGit(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("git is not available; skipping Git source tests")
	}
}

func gitRun(t *testing.T, dir string, args ...string) string {
	t.Helper()
	full := append([]string{"-C", dir, "-c", "user.name=t", "-c", "user.email=t@example.com",
		"-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"}, args...)
	out, err := exec.Command("git", full...).CombinedOutput()
	if err != nil {
		t.Fatalf("git %v: %v\n%s", args, err, out)
	}
	return strings.TrimSpace(string(out))
}

// newRepo makes a committed repo containing files.  Names with newlines are
// not creatable on Windows, so callers drop them there.
func newRepo(t *testing.T, files map[string]string) string {
	t.Helper()
	dir := t.TempDir()
	gitRun(t, dir, "init", "-q")
	writeTree(t, dir, files)
	gitRun(t, dir, "add", "-A")
	gitRun(t, dir, "commit", "-q", "-m", "one")
	return dir
}

// portableTree is sampleTree minus names some filesystems cannot hold.
func portableTree(t *testing.T) map[string]string {
	tree := sampleTree()
	probe := filepath.Join(t.TempDir(), "a\nb")
	if err := os.WriteFile(probe, nil, 0o644); err != nil {
		delete(tree, "new\nline.txt")
	}
	return tree
}

func keys(m map[string]string) []string {
	var out []string
	for k := range m {
		out = append(out, k)
	}
	sortStrings(out)
	return out
}

func TestGitFilesAndOddNames(t *testing.T) {
	skipIfNoGit(t)
	tree := portableTree(t)
	dir := newRepo(t, tree)
	g := NewGit(dir)

	got, err := g.Files("", nil)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(got, keys(tree)) {
		t.Fatalf("got %q\nwant %q", got, keys(tree))
	}
	got, _ = g.Files("src", []string{"**/*.go"})
	if !reflect.DeepEqual(got, []string{"src/a.go", "src/deep/b.go"}) {
		t.Fatalf("root+pattern: %q", got)
	}
	got, _ = g.Files(".", []string{"*.md"})
	if !reflect.DeepEqual(got, []string{"README.md"}) {
		t.Fatalf("pattern: %q", got)
	}
	if _, err := g.Files("../x", nil); err == nil {
		t.Fatal("bad root accepted")
	}
	if _, err := g.Files("", []string{"/abs"}); err == nil {
		t.Fatal("bad pattern accepted")
	}

	rc, err := g.Open("with space/x y.go")
	if err != nil {
		t.Fatal(err)
	}
	b, _ := io.ReadAll(rc)
	rc.Close()
	if string(b) != "x" {
		t.Fatalf("content %q", b)
	}
	if _, err := g.Open("../escape"); err == nil {
		t.Fatal("bad path opened")
	}
	if _, err := g.Open("missing.txt"); err == nil {
		t.Fatal("missing opened")
	}
	if _, err := g.Open("src"); err == nil {
		t.Fatal("directory opened")
	}

	rev, err := g.Revision()
	if err != nil || len(rev) < 40 || rev != gitRun(t, dir, "rev-parse", "HEAD") {
		t.Fatalf("revision %q %v", rev, err)
	}
}

func TestGitNotARepo(t *testing.T) {
	skipIfNoGit(t)
	g := NewGit(t.TempDir())
	if _, err := g.Files("", nil); err == nil {
		t.Fatal("expected error")
	}
	if _, err := g.Revision(); err == nil {
		t.Fatal("expected error")
	}
}

func TestGitChanged(t *testing.T) {
	skipIfNoGit(t)
	dir := newRepo(t, map[string]string{"a.txt": "1", "b.txt": "1"})
	g := NewGit(dir)
	base, _ := g.Revision()
	writeTree(t, dir, map[string]string{"a.txt": "2", "sp ace/-new.txt": "n", "ü.txt": "n"})
	gitRun(t, dir, "add", "-A")
	gitRun(t, dir, "commit", "-q", "-m", "two")

	paths, known, err := g.Changed(base)
	if err != nil || !known {
		t.Fatalf("known=%v err=%v", known, err)
	}
	want := []string{"a.txt", "sp ace/-new.txt", "ü.txt"}
	if !reflect.DeepEqual(paths, want) {
		t.Fatalf("got %q want %q", paths, want)
	}
	paths, known, err = g.Changed("HEAD")
	if err != nil || !known || len(paths) != 0 {
		t.Fatalf("HEAD..HEAD: %v %v %v", paths, known, err)
	}

	// Unknown revision: not an error.
	if p, known, err := g.Changed("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"); err != nil || known || p != nil {
		t.Fatalf("unknown rev: %v %v %v", p, known, err)
	}

	// Option injection and malformed revisions are rejected outright.
	out := filepath.Join(t.TempDir(), "pwned")
	for _, bad := range []string{"--output=" + out, "-p", "", "a..b", "a b", "x\ny"} {
		if _, _, err := g.Changed(bad); err == nil {
			t.Errorf("Changed(%q) accepted", bad)
		}
	}
	if _, err := os.Stat(out); err == nil {
		t.Fatal("option injection wrote a file")
	}
}

func TestGitChangedNoMergeBase(t *testing.T) {
	skipIfNoGit(t)
	dir := newRepo(t, map[string]string{"a.txt": "1"})
	gitRun(t, dir, "checkout", "-q", "--orphan", "other")
	writeTree(t, dir, map[string]string{"o.txt": "1"})
	gitRun(t, dir, "add", "-A")
	gitRun(t, dir, "commit", "-q", "-m", "orphan")
	main := gitRun(t, dir, "rev-list", "--max-parents=0", "--branches", "-n", "1")
	// Find the first branch's root, which shares no history with HEAD.
	for _, c := range strings.Fields(gitRun(t, dir, "rev-list", "--max-parents=0", "--all")) {
		if c != gitRun(t, dir, "rev-parse", "HEAD") {
			main = c
		}
	}
	if p, known, err := NewGit(dir).Changed(main); err != nil || known || p != nil {
		t.Fatalf("no merge base: %v %v %v", p, known, err)
	}
}

func TestFolderBasics(t *testing.T) {
	dir := t.TempDir()
	tree := portableTree(t)
	tree[".kilnignore"] = "# comment\n\n  build/**  \r\nnode_modules\n**/*.log\r\n"
	tree["build/out.o"] = "o"
	tree["node_modules/m/x.js"] = "j"
	tree["deep/a.log"] = "l"
	tree["top.log"] = "l"
	tree[".git/config"] = "should never be listed"
	writeTree(t, dir, tree)
	f := NewFolder(dir)

	got, err := f.Files("", nil)
	if err != nil {
		t.Fatal(err)
	}
	want := keys(portableTree(t))
	want = append(want, ".kilnignore")
	sortStrings(want)
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %q\nwant %q", got, want)
	}
	got, _ = f.Files("src", []string{"*.go"})
	if !reflect.DeepEqual(got, []string{"src/a.go"}) {
		t.Fatalf("got %q", got)
	}

	rc, err := f.Open("src/a.go")
	if err != nil {
		t.Fatal(err)
	}
	rc.Close()

	if p, known, err := f.Changed("anything"); p != nil || known || err != nil {
		t.Fatal("Changed must be unknown")
	}

	r1, err := f.Revision()
	if err != nil || len(r1) != 64 {
		t.Fatalf("rev %q %v", r1, err)
	}
	r2, _ := f.Revision()
	if r1 != r2 {
		t.Fatal("revision unstable")
	}
	writeTree(t, dir, map[string]string{"src/a.go": "package changed"})
	if r3, _ := f.Revision(); r3 == r1 {
		t.Fatal("revision ignored a content change")
	}
	writeTree(t, dir, map[string]string{"src/a.go": "package a", "build/more": "ignored"})
	if r4, _ := f.Revision(); r4 != r1 {
		t.Fatal("ignored file or restored content changed revision")
	}
}

func TestFolderBadIgnore(t *testing.T) {
	for _, body := range []string{"build/\n", "/abs\n", "../up\n"} {
		dir := t.TempDir()
		writeTree(t, dir, map[string]string{".kilnignore": body, "a": "x"})
		f := NewFolder(dir)
		if _, err := f.Files("", nil); err == nil || !strings.Contains(err.Error(), ".kilnignore line 1") {
			t.Errorf("%q: err=%v", body, err)
		}
		if _, err := f.Revision(); err == nil {
			t.Errorf("%q: Revision accepted bad ignore", body)
		}
	}
}

func TestFolderMissingDir(t *testing.T) {
	f := NewFolder(filepath.Join(t.TempDir(), "nope"))
	if _, err := f.Files("", nil); err == nil {
		t.Fatal("expected error")
	}
	if _, err := f.Revision(); err == nil {
		t.Fatal("expected error")
	}
}

func TestIgnoreFileMustBeRegular(t *testing.T) {
	dir := t.TempDir()
	if err := os.Mkdir(filepath.Join(dir, IgnoreFile), 0o755); err != nil {
		t.Fatal(err)
	}
	if _, err := NewFolder(dir).Files("", nil); err == nil {
		t.Fatal("directory accepted as .kilnignore")
	}
}

func TestIgnoreFileSymlinkRefused(t *testing.T) {
	dir, outside := t.TempDir(), t.TempDir()
	writeTree(t, outside, map[string]string{"ign": "*\n"})
	if err := os.Symlink(filepath.Join(outside, "ign"), filepath.Join(dir, IgnoreFile)); err != nil {
		t.Skipf("cannot create symlinks here: %v", err)
	}
	if _, err := NewFolder(dir).Files("", nil); err == nil {
		t.Fatal("symlinked .kilnignore followed")
	}
}

func TestIgnoreLimits(t *testing.T) {
	cases := map[string]string{
		"too big":        strings.Repeat("# padding padding\n", 5000),
		"too many":       strings.Repeat("x\n", MaxIgnorePatterns+1),
		"too deep":       strings.Repeat("a/", MaxPatternSegments) + "a\n",
		"huge one-liner": strings.Repeat("a", MaxIgnoreBytes+1),
	}
	for name, body := range cases {
		dir := t.TempDir()
		writeTree(t, dir, map[string]string{IgnoreFile: body})
		if _, err := NewFolder(dir).Files("", nil); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// Exactly at the caps is fine.
	dir := t.TempDir()
	writeTree(t, dir, map[string]string{IgnoreFile: strings.Repeat("x\n", MaxIgnorePatterns-1) + strings.Repeat("a/", MaxPatternSegments-1) + "a"})
	if _, err := NewFolder(dir).Files("", nil); err != nil {
		t.Fatalf("at the caps: %v", err)
	}
}

func TestBadIgnoreErrorIsSanitised(t *testing.T) {
	dir := t.TempDir()
	writeTree(t, dir, map[string]string{IgnoreFile: "/\x1b[31m" + strings.Repeat("z", 500) + "\n"})
	_, err := NewFolder(dir).Files("", nil)
	if err == nil || strings.ContainsRune(err.Error(), 0x1b) || len(err.Error()) > 200 {
		t.Fatalf("error not sanitised: %v", err)
	}
}

func TestMatchIsNotExponential(t *testing.T) {
	pat := strings.Repeat("**/", 30) + "x"
	path := strings.Repeat("d/", 39) + "y"
	start := time.Now()
	if Match(pat, path) {
		t.Fatal("should not match")
	}
	if !Match(pat, strings.Repeat("d/", 39)+"x") {
		t.Fatal("should match")
	}
	if el := time.Since(start); el > 2*time.Second {
		t.Fatalf("matching took %v", el)
	}
}

func TestPortablePaths(t *testing.T) {
	bad := []string{"a\\b", "a:b", "C:/x", "con", "CON.txt", "dir/nul.tar.gz", "Aux", "com1", "LPT9.x", "prn ", "x/COM5/y"}
	for _, p := range bad {
		if checkSourcePath(p) == nil {
			t.Errorf("%q accepted", p)
		}
		if _, err := filter([]string{p}, "", nil); err == nil {
			t.Errorf("filter accepted %q", p)
		}
	}
	for _, p := range []string{"console", "com0", "com10", "lpt", "comm1", "a.con", "src/a.go", "-dash", "é"} {
		if err := checkSourcePath(p); err != nil {
			t.Errorf("%q rejected: %v", p, err)
		}
	}
	if _, err := normRoot("a:b"); err == nil {
		t.Fatal("root a:b accepted")
	}
}

func TestGitEnvAndArgs(t *testing.T) {
	in := []string{"PATH=/bin", "HOME=/h", "GIT_DIR=/evil", "git_work_tree=/evil", "GIT_SSH_COMMAND=x", "GITHUB_TOKEN=t",
		"GIT_CONFIG_GLOBAL=/evil", "NPM_TOKEN=t", "GIT_CONFIG_NOSYSTEM=0", "LC_ALL=de"}
	got := gitEnv(in)
	joined := "\n" + strings.Join(got, "\n") + "\n"
	for _, bad := range []string{"GIT_DIR=/evil", "git_work_tree", "GIT_SSH_COMMAND", "GITHUB_TOKEN", "GIT_CONFIG_GLOBAL", "NPM_TOKEN", "NOSYSTEM=0", "LC_ALL=de"} {
		if strings.Contains(joined, bad) {
			t.Errorf("%s survived: %v", bad, got)
		}
	}
	for _, want := range []string{"PATH=/bin", "HOME=/h", "GIT_CONFIG_NOSYSTEM=1", "GIT_OPTIONAL_LOCKS=0", "GIT_TERMINAL_PROMPT=0", "LC_ALL=C"} {
		if !strings.Contains(joined, "\n"+want+"\n") {
			t.Errorf("missing %s", want)
		}
	}
	h := strings.Join(gitHardening(), " ")
	for _, want := range []string{"--no-pager", "core.fsmonitor=false", "core.hooksPath=" + os.DevNull, "protocol.file.allow=never"} {
		if !strings.Contains(h, want) {
			t.Errorf("hardening lacks %s", want)
		}
	}
}

func TestGitHostileRepoConfigDoesNotRun(t *testing.T) {
	skipIfNoGit(t)
	if runtime.GOOS == "windows" {
		t.Skip("uses a POSIX shell script")
	}
	dir := newRepo(t, map[string]string{"a.txt": "1"})
	marker := filepath.Join(t.TempDir(), "ran")
	hook := filepath.Join(t.TempDir(), "mon.sh")
	os.WriteFile(hook, []byte("#!/bin/sh\ntouch '"+marker+"'\n"), 0o755)
	gitRun(t, dir, "config", "core.fsmonitor", hook)
	if _, err := NewGit(dir).Files("", nil); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(marker); err == nil {
		t.Fatal("repo-configured fsmonitor ran")
	}
}

func TestSymlinksNotFollowed(t *testing.T) {
	root := t.TempDir()
	outside := t.TempDir()
	writeTree(t, outside, map[string]string{"secret.txt": "top secret"})
	dir := filepath.Join(root, "proj")
	writeTree(t, dir, map[string]string{"a.txt": "a"})
	if err := os.Symlink(filepath.Join(outside, "secret.txt"), filepath.Join(dir, "leak.txt")); err != nil {
		t.Skipf("cannot create symlinks here: %v", err)
	}
	if err := os.Symlink(outside, filepath.Join(dir, "linkdir")); err != nil {
		t.Skipf("cannot create symlinks here: %v", err)
	}
	f := NewFolder(dir)
	got, err := f.Files("", nil)
	if err != nil {
		t.Fatal(err)
	}
	// Symlinks are entries but nothing beneath linkdir is listed.
	if !reflect.DeepEqual(got, []string{"a.txt", "leak.txt", "linkdir"}) {
		t.Fatalf("got %q", got)
	}
	for _, p := range []string{"leak.txt", "linkdir/secret.txt"} {
		if rc, err := f.Open(p); err == nil {
			rc.Close()
			t.Fatalf("Open(%q) followed a symlink out of the root", p)
		}
	}
	r1, err := f.Revision()
	if err != nil {
		t.Fatal(err)
	}
	// Changing the outside file must not change the revision (never read).
	writeTree(t, outside, map[string]string{"secret.txt": "changed"})
	if r2, _ := f.Revision(); r2 != r1 {
		t.Fatal("revision depended on a file outside the root")
	}
	// Retargeting the link does change it.
	os.Remove(filepath.Join(dir, "leak.txt"))
	os.Symlink(filepath.Join(outside, "other"), filepath.Join(dir, "leak.txt"))
	if r3, _ := f.Revision(); r3 == r1 {
		t.Fatal("symlink target not part of revision")
	}
}

func TestSymlinkedParentDirRefused(t *testing.T) {
	root := t.TempDir()
	outside := t.TempDir()
	writeTree(t, outside, map[string]string{"s.txt": "x"})
	dir := filepath.Join(root, "proj")
	writeTree(t, dir, map[string]string{"a.txt": "a"})
	if err := os.Symlink(outside, filepath.Join(dir, "d")); err != nil {
		t.Skipf("cannot create symlinks here: %v", err)
	}
	if _, err := openRegular(dir, "d/s.txt"); err == nil {
		t.Fatal("opened through a symlinked directory")
	}
}

func TestGitAndFolderAgree(t *testing.T) {
	skipIfNoGit(t)
	tree := portableTree(t)
	dir := newRepo(t, tree)
	// An untracked file: Git ignores it only if it is gitignored; use
	// .gitignore + .kilnignore with the same effect, and list both files.
	writeTree(t, dir, map[string]string{".gitignore": "*.log\n", ".kilnignore": "**/*.log\n*.log\n", "x.log": "l", "sub/y.log": "l"})
	gitRun(t, dir, "add", "-A")
	gitRun(t, dir, "commit", "-q", "-m", "ignores")

	gf, err := NewGit(dir).Files("", nil)
	if err != nil {
		t.Fatal(err)
	}
	ff, err := NewFolder(dir).Files("", nil)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(gf, ff) {
		t.Fatalf("git %q\nfolder %q", gf, ff)
	}
	for _, pats := range [][]string{{"**/*.go"}, {"docs/*", "*.md"}} {
		a, _ := NewGit(dir).Files("", pats)
		b, _ := NewFolder(dir).Files("", pats)
		if !reflect.DeepEqual(a, b) {
			t.Fatalf("%v: git %q folder %q", pats, a, b)
		}
	}
}

func TestIOErrorPaths(t *testing.T) {
	dir := t.TempDir()
	root, err := os.OpenRoot(dir)
	if err != nil {
		t.Fatal(err)
	}
	defer root.Close()
	if _, _, err := digestEntry(root, "missing"); err == nil {
		t.Fatal("digest of a missing file succeeded")
	}
	writeTree(t, dir, map[string]string{"a": "x"})
	// Parent directory of the requested file does not exist.
	if _, err := openRegular(dir, "nodir/a"); err == nil {
		t.Fatal("expected error")
	}
	// Source root itself does not exist.
	if _, err := openRegular(filepath.Join(dir, "gone"), "a"); err == nil {
		t.Fatal("expected error")
	}
	// A path the validator rejects reaches filter from a source.
	if _, err := filter([]string{"ok", "bad//path"}, "", nil); err == nil {
		t.Fatal("invalid listed path accepted")
	}
}
