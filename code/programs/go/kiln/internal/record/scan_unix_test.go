//go:build unix

package record

import (
	"net"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

func cleanRepo(t *testing.T) string {
	t.Helper()
	return newRepo(t, map[string]string{
		"code/packages/x/a/BUILD":   "echo a\n",
		"code/packages/x/a/src.txt": "a\n",
		"README.md":                 "hi\n",
	})
}

func TestScanTreeCleanRepo(t *testing.T) {
	if err := ScanTree(cleanRepo(t)); err != nil {
		t.Fatal(err)
	}
}

func TestScanTreeRefusals(t *testing.T) {
	cases := map[string]struct {
		mutate func(t *testing.T, root string)
		want   string
	}{
		"untracked BUILD_linux beside a BUILD": {func(t *testing.T, r string) {
			writeFile(t, r, "code/packages/x/a/BUILD_linux", "echo hijack\n")
		}, "code/packages/x/a/BUILD_linux: an untracked BUILD"},
		"a whole untracked package": {func(t *testing.T, r string) {
			writeFile(t, r, "code/packages/x/new/BUILD", "echo new\n")
		}, "code/packages/x/new/BUILD"},
		"an untracked BUILD deep in an ignored directory": {func(t *testing.T, r string) {
			writeFile(t, r, ".gitignore", "target/\n")
			writeFile(t, r, "code/packages/x/a/target/deep/BUILD", "echo\n")
		}, "target/deep/BUILD"},
		"an untracked BUILD symlink": {func(t *testing.T, r string) {
			symlink(t, r, "code/packages/x/a/BUILD_windows", "BUILD")
		}, "BUILD_windows"},
		"an untracked directory named BUILD": {func(t *testing.T, r string) {
			if err := os.MkdirAll(filepath.Join(r, "code", "BUILDdir"), 0o755); err != nil {
				t.Fatal(err)
			}
		}, "BUILDdir"},
		"a hard-linked tracked file": {func(t *testing.T, r string) {
			if err := os.Link(filepath.Join(r, "README.md"), filepath.Join(r, "code", "alias.md")); err != nil {
				t.Fatal(err)
			}
		}, "a hard link (2 names)"},
		"a hard link inside an untracked directory": {func(t *testing.T, r string) {
			writeFile(t, r, "scratch/one", "x")
			if err := os.Link(filepath.Join(r, "scratch/one"), filepath.Join(r, "scratch/two")); err != nil {
				t.Fatal(err)
			}
		}, "scratch/"},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			root := cleanRepo(t)
			c.mutate(t, root)
			err := ScanTree(root)
			if err == nil || !strings.Contains(err.Error(), c.want) {
				t.Fatalf("got %v, want %q", err, c.want)
			}
		})
	}
}

func TestScanTreeAcceptsWhatIsNotAProblem(t *testing.T) {
	root := cleanRepo(t)
	// A tracked file named BUILD*, an untracked non-BUILD file, a symlink (its
	// own link count is not the file's), and a hard link INSIDE .git, which is
	// not walked.
	writeFile(t, root, "notes.txt", "x")
	symlink(t, root, "code/link", "../README.md")
	if err := os.Link(filepath.Join(root, ".git", "HEAD"), filepath.Join(root, ".git", "HEAD2")); err != nil {
		t.Fatal(err)
	}
	if err := ScanTree(root); err != nil {
		t.Fatal(err)
	}
}

func TestScanTreeAcceptsATrackedBuildVariant(t *testing.T) {
	root := newRepo(t, map[string]string{"p/BUILD": "a\n", "p/BUILD_windows": "b\n"})
	if err := ScanTree(root); err != nil {
		t.Fatal(err)
	}
}

func TestScanTreeNamesAtMostMaxReported(t *testing.T) {
	root := cleanRepo(t)
	for i := 0; i < MaxReported+5; i++ {
		writeFile(t, root, "code/many/BUILD_"+string(rune('a'+i)), "x")
	}
	err := ScanTree(root)
	te, ok := err.(*TreeError)
	if !ok || len(te.Problems) != MaxReported || te.Total != MaxReported+5 {
		t.Fatalf("%v", err)
	}
}

func TestScanTreeStopsAtTheLimit(t *testing.T) {
	old := scanLimit
	scanLimit = 3
	defer func() { scanLimit = old }()
	root := cleanRepo(t)
	for i := 0; i < 6; i++ {
		writeFile(t, root, "code/many/BUILD_"+string(rune('a'+i)), "x")
	}
	if err := ScanTree(root); err == nil || !strings.Contains(err.Error(), "stopping") {
		t.Fatal(err)
	}
}

func TestScanTreeNeedsATopLevelRoot(t *testing.T) {
	root := cleanRepo(t)
	if err := ScanTree(filepath.Join(root, "code")); err == nil {
		t.Fatal("a subdirectory was accepted as the root")
	}
	if err := ScanTree(filepath.Join(t.TempDir(), "absent")); err == nil {
		t.Fatal("a missing root was accepted")
	}
}

func TestCheckInventory(t *testing.T) {
	ok := recfmt.Inventory{Schema: recfmt.SchemaInventory, Paths: []string{"a/BUIL", "a/notBUILD", "a/target"}}
	if err := CheckInventory(ok); err != nil {
		t.Fatal(err)
	}
	for _, p := range []string{"a/BUILD", "a/BUILD_linux", "BUILD", "a/BUILDER"} {
		bad := recfmt.Inventory{Schema: recfmt.SchemaInventory, Paths: []string{p}}
		if err := CheckInventory(bad); err == nil || !strings.Contains(err.Error(), "BUILD") {
			t.Errorf("%s: %v", p, err)
		}
	}
}

// ---------------------------------------------------------------------------
// HostScan

func shortTemp(t *testing.T) string {
	t.Helper()
	// A pathname socket's path is limited to about 100 bytes, which a test's
	// own temp directory can exceed on macOS.
	d, err := os.MkdirTemp("", "ks")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(d) })
	real, err := filepath.EvalSymlinks(d)
	if err != nil {
		t.Fatal(err)
	}
	return real
}

func listenUnix(t *testing.T, p string) {
	t.Helper()
	l, err := net.Listen("unix", p)
	if err != nil {
		t.Skipf("cannot bind a unix socket here: %v", err)
	}
	t.Cleanup(func() { l.Close() })
}

func TestHostScanCleanRoots(t *testing.T) {
	a, b := shortTemp(t), shortTemp(t)
	write(t, filepath.Join(a, "bin", "tool"), "x", 0o755)
	write(t, filepath.Join(b, "lib", "x.so"), "x", 0o644)
	if err := HostScan([]string{a, b}); err != nil {
		t.Fatal(err)
	}
}

func TestHostScanFindsASocketAndAFifo(t *testing.T) {
	root := shortTemp(t)
	if err := os.MkdirAll(filepath.Join(root, "run", "deep"), 0o755); err != nil {
		t.Fatal(err)
	}
	listenUnix(t, filepath.Join(root, "run", "d.sock"))
	if err := syscall.Mkfifo(filepath.Join(root, "run", "deep", "pipe"), 0o600); err != nil {
		t.Fatal(err)
	}
	err := HostScan([]string{root})
	if err == nil || !strings.Contains(err.Error(), "d.sock") || !strings.Contains(err.Error(), "deep/pipe") ||
		!strings.Contains(err.Error(), "2 problems") {
		t.Fatalf("%v", err)
	}
}

func TestHostScanRootThatIsAFifoOrAFile(t *testing.T) {
	d := shortTemp(t)
	if err := HostScan([]string{mkfifo(t, d)}); err == nil {
		t.Fatal("a FIFO root was accepted")
	}
	f := filepath.Join(d, "afile")
	write(t, f, "x", 0o644)
	if err := HostScan([]string{f}); err != nil {
		t.Fatal(err)
	}
}

func TestHostScanDoesNotFollowSymlinks(t *testing.T) {
	outside, root := shortTemp(t), shortTemp(t)
	if err := syscall.Mkfifo(filepath.Join(outside, "pipe"), 0o600); err != nil {
		t.Fatal(err)
	}
	// A link to a directory holding a FIFO, a link to the FIFO itself, and a
	// ROOT that is a link (as /bin is): none is entered or counted.
	if err := os.Symlink(outside, filepath.Join(root, "dirlink")); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(filepath.Join(outside, "pipe"), filepath.Join(root, "filelink")); err != nil {
		t.Fatal(err)
	}
	rootLink := filepath.Join(shortTemp(t), "rootlink")
	if err := os.Symlink(outside, rootLink); err != nil {
		t.Fatal(err)
	}
	if err := HostScan([]string{root, rootLink}); err != nil {
		t.Fatal(err)
	}
}

func TestHostScanDoesNotCrossAMount(t *testing.T) {
	root := shortTemp(t)
	mount := filepath.Join(root, "mnt")
	plain := filepath.Join(root, "plain")
	for _, d := range []string{mount, plain} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
		if err := syscall.Mkfifo(filepath.Join(d, "pipe"), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	old, oldMounts := devOf, mountPoints
	defer func() { devOf, mountPoints = old, oldMounts }()
	mountPoints = func() map[string]bool { return map[string]bool{mount: true} }
	devOf = func(fi os.FileInfo) (uint64, bool) {
		if fi.Name() == "mnt" {
			return 99, true // pretend a different file system is mounted here
		}
		return old(fi)
	}
	err := HostScan([]string{root})
	if err == nil || strings.Contains(err.Error(), "mnt") || !strings.Contains(err.Error(), "plain/pipe") || !strings.Contains(err.Error(), "1 problem") {
		t.Fatalf("%v", err)
	}
	// Without the pretence the mounted directory is part of the root.
	devOf = old
	mountPoints = oldMounts
	if err := HostScan([]string{root}); err == nil || !strings.Contains(err.Error(), "2 problems") {
		t.Fatalf("%v", err)
	}
}

func TestHostScanSkipsWhatItCannotReadAndFailsOnMissingRoots(t *testing.T) {
	root := shortTemp(t)
	locked := filepath.Join(root, "locked")
	write(t, filepath.Join(locked, "x"), "x", 0o644)
	if err := os.Chmod(locked, 0); err != nil {
		t.Fatal(err)
	}
	defer os.Chmod(locked, 0o755)
	if os.Geteuid() != 0 { // root reads everything
		if err := HostScan([]string{root}); err != nil {
			t.Fatalf("an unreadable directory failed the scan: %v", err)
		}
	}
	if err := HostScan([]string{filepath.Join(root, "absent")}); err == nil {
		t.Fatal("a missing root passed")
	}
}

func TestHostScanNoRoots(t *testing.T) {
	if err := HostScan(nil); err != nil {
		t.Fatal(err)
	}
}
