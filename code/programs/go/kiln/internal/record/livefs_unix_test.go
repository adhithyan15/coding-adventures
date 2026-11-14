//go:build unix

package record

// Tests of the live no-follow disk view and of Tree.Head (KILN04 section 4.8,
// decision P13). The /proc/self/fd check is the kernel's own name for an open
// file (F_GETPATH on macOS), so a link ANYWHERE in the path, not only in the last
// component, is refused.

import (
	"bytes"
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

func TestLiveFSLstatAndReadlink(t *testing.T) {
	d := canonTemp(t)
	write(t, filepath.Join(d, "f"), "x", 0o644)
	os.Mkdir(filepath.Join(d, "dir"), 0o755)
	os.Symlink("f", filepath.Join(d, "l"))
	os.Symlink("nowhere", filepath.Join(d, "dangling"))
	mkfifo(t, d)
	var fsys LiveFS
	for name, want := range map[string]traced.NodeKind{"f": traced.KindFile, "dir": traced.KindDir, "l": traced.KindLink, "dangling": traced.KindLink, "fifo": traced.KindOther} {
		info, err := fsys.Lstat(filepath.Join(d, name))
		if err != nil || info.Kind != want {
			t.Errorf("%s: %+v %v", name, info, err)
		}
		if want != traced.KindOther && want != traced.KindDir && (info.Ctime.IsZero() || time.Since(info.Ctime) > time.Hour) {
			t.Errorf("%s: ctime %v", name, info.Ctime)
		}
	}
	// A link is reported as a link, never followed.
	if info, _ := fsys.Lstat(filepath.Join(d, "dangling")); info.Kind != traced.KindLink {
		t.Fatal("followed a dangling link")
	}
	if got, err := fsys.Readlink(filepath.Join(d, "l")); err != nil || got != "f" {
		t.Fatalf("%q %v", got, err)
	}
	// Absence is fs.ErrNotExist, as the classifier requires.
	if _, err := fsys.Lstat(filepath.Join(d, "nope")); !errors.Is(err, fs.ErrNotExist) {
		t.Fatalf("%v", err)
	}
	if _, err := fsys.Readlink(filepath.Join(d, "f")); err == nil {
		t.Fatal("readlink of a file")
	}
}

func TestLiveFSReadHead(t *testing.T) {
	d := canonTemp(t)
	big := bytes.Repeat([]byte("ab"), 100<<10)
	writeBytes(t, filepath.Join(d, "big"), big, 0o644)
	write(t, filepath.Join(d, "small"), "#!/bin/sh\n", 0o755)
	write(t, filepath.Join(d, "empty"), "", 0o644)
	var fsys LiveFS
	if got, err := fsys.ReadHead(filepath.Join(d, "small"), 100); err != nil || string(got) != "#!/bin/sh\n" {
		t.Fatalf("%q %v", got, err)
	}
	if got, err := fsys.ReadHead(filepath.Join(d, "small"), 4); err != nil || string(got) != "#!/b" {
		t.Fatalf("%q %v", got, err)
	}
	// At most 64 KiB, whatever was asked.
	if got, err := fsys.ReadHead(filepath.Join(d, "big"), 1<<30); err != nil || len(got) != maxLiveHead || !bytes.Equal(got, big[:maxLiveHead]) {
		t.Fatalf("%d %v", len(got), err)
	}
	if got, err := fsys.ReadHead(filepath.Join(d, "big"), -1); err != nil || len(got) != maxLiveHead {
		t.Fatalf("%d %v", len(got), err)
	}
	if got, err := fsys.ReadHead(filepath.Join(d, "empty"), 10); err != nil || len(got) != 0 {
		t.Fatalf("%q %v", got, err)
	}
	if _, err := fsys.ReadHead(filepath.Join(d, "absent"), 10); err == nil {
		t.Fatal("read an absent file")
	}
	for _, p := range []string{"relative/path", d + "/a/../small", d + "/small/", d + "/./small", d + "//small"} {
		if _, err := fsys.ReadHead(p, 10); err == nil {
			t.Errorf("read the unclean path %q", p)
		}
	}
}

// A symlink is never followed: not as the last component, and not as an
// ancestor either (the file's real name differs from the one asked for).
func TestLiveFSNeverFollowsASymlink(t *testing.T) {
	d := canonTemp(t)
	write(t, filepath.Join(d, "real", "f"), "secret", 0o644)
	os.Symlink(filepath.Join(d, "real", "f"), filepath.Join(d, "link"))
	os.Symlink("real", filepath.Join(d, "dirlink"))
	var fsys LiveFS
	if got, err := fsys.ReadHead(filepath.Join(d, "link"), 10); err == nil {
		t.Fatalf("followed the last component: %q", got)
	}
	if got, err := fsys.ReadHead(filepath.Join(d, "dirlink", "f"), 10); err == nil {
		t.Fatalf("followed an ancestor: %q", got)
	} else if !strings.Contains(err.Error(), "opened as") {
		t.Fatalf("refused, but not by the name check: %v", err)
	}
	// The same file by its real name is read.
	if got, err := fsys.ReadHead(filepath.Join(d, "real", "f"), 10); err != nil || string(got) != "secret" {
		t.Fatalf("%q %v", got, err)
	}
}

// A FIFO is not opened at all: the open of a FIFO for reading would wait for a
// writer, so the read must refuse it before, in bounded time.
func TestLiveFSNeverOpensANonRegularFile(t *testing.T) {
	d := canonTemp(t)
	fifo := mkfifo(t, d)
	var fsys LiveFS
	done := make(chan error, 1)
	go func() { _, err := fsys.ReadHead(fifo, 10); done <- err }()
	select {
	case err := <-done:
		if err == nil || !strings.Contains(err.Error(), "not a regular file") {
			t.Fatalf("%v", err)
		}
	case <-time.After(10 * time.Second):
		t.Fatal("ReadHead of a FIFO hung")
	}
	// Nor a directory, nor a socket.
	if _, err := fsys.ReadHead(d, 10); err == nil {
		t.Fatal("read a directory")
	}
	sock := filepath.Join(d, "s")
	fd, err := syscall.Socket(syscall.AF_UNIX, syscall.SOCK_STREAM, 0)
	if err != nil {
		t.Skip(err)
	}
	defer syscall.Close(fd)
	if err := syscall.Bind(fd, &syscall.SockaddrUnix{Name: sock}); err != nil {
		t.Skip(err)
	}
	if _, err := fsys.ReadHead(sock, 10); err == nil {
		t.Fatal("read a socket")
	}
	// The same through the verified read of a tracked file.
	if _, err := readVerifiedHead(fifo, blobID(""), 100, 10); err == nil {
		t.Fatal("hashed a FIFO")
	}
}

// A name that does not match what the kernel reports for the handle is refused:
// here a hard-to-spot case, a path through a link to a directory (the name check
// is what catches it), and a swap of the file between lstat and open is caught
// by the inode comparison.
func TestOpenRegularCheckedRefusesAMismatch(t *testing.T) {
	d := canonTemp(t)
	write(t, filepath.Join(d, "a", "f"), "x", 0o644)
	os.Symlink("a", filepath.Join(d, "b"))
	if f, _, err := openRegularChecked(filepath.Join(d, "b", "f")); err == nil {
		f.Close()
		t.Fatal("opened through a link to a directory")
	}
	f, st, err := openRegularChecked(filepath.Join(d, "a", "f"))
	if err != nil || st.Size() != 1 {
		t.Fatalf("%v", err)
	}
	f.Close()
	// An unlinked-but-open name is not the name asked for either: fdPath says
	// so (Linux appends " (deleted)").
	g, _ := os.Open(filepath.Join(d, "a", "f"))
	os.Remove(filepath.Join(d, "a", "f"))
	if name, err := fdPath(g); runtime.GOOS == "linux" && (err != nil || name == filepath.Join(d, "a", "f")) {
		t.Fatalf("fdPath of a deleted file is %q (%v)", name, err)
	}
	g.Close()
}

// --- DiskTree.Head ----------------------------------------------------------

func diskTreeFixture(t *testing.T) (*DiskTree, string) {
	t.Helper()
	root := canonTemp(t)
	write(t, filepath.Join(root, "s.sh"), "#!/bin/sh\necho hi\n", 0o755)
	write(t, filepath.Join(root, "dir", "f.txt"), "hello", 0o644)
	os.Symlink("s.sh", filepath.Join(root, "ln"))
	tree := map[string]entry{
		"s.sh":      {modeExec, blobID("#!/bin/sh\necho hi\n")},
		"dir/f.txt": {modeFile, blobID("hello")},
		"ln":        {modeSymlink, blobID("s.sh")},
		"gone":      {modeFile, blobID("gone")},
	}
	data, err := EncodeTree(tree, func(string) (string, error) { return "s.sh", nil })
	if err != nil {
		t.Fatal(err)
	}
	tf, err := DecodeTree(data)
	if err != nil {
		t.Fatal(err)
	}
	return NewDiskTree(tf, root), root
}

func TestDiskTreeHeadIsTheDiskBytesOnlyWhenTheyAreTheCommittedOnes(t *testing.T) {
	d, root := diskTreeFixture(t)
	if got, err := d.Head("s.sh", 100); err != nil || string(got) != "#!/bin/sh\necho hi\n" {
		t.Fatalf("%q %v", got, err)
	}
	if got, err := d.Head("s.sh", 4); err != nil || string(got) != "#!/b" {
		t.Fatalf("%q %v", got, err)
	}
	// Changed on the disk: the committed bytes are not there, so Head says so
	// (the classifier then reads the disk as an untracked exec read, and checks
	// it for later changes).
	d2, root2 := diskTreeFixture(t)
	write(t, filepath.Join(root2, "s.sh"), "#!/bin/evil\n", 0o755)
	if got, err := d2.Head("s.sh", 100); err == nil {
		t.Fatalf("returned the changed bytes %q", got)
	} else if !strings.Contains(err.Error(), "not the committed content") {
		t.Fatalf("%v", err)
	}
	// A cached verified head is served without reading the disk again: this is
	// the SAME action's later exec of the same script.
	write(t, filepath.Join(root, "s.sh"), "#!/bin/evil\n", 0o755)
	if got, err := d.Head("s.sh", 100); err != nil || string(got) != "#!/bin/sh\necho hi\n" {
		t.Fatalf("the cache was not used: %q %v", got, err)
	}
	// Not a tracked regular file: a link, a directory, an untracked path, a
	// tracked file the disk lacks, one that is a link on the disk.
	for _, p := range []string{"ln", "dir", "nothing", "gone"} {
		if _, err := d.Head(p, 10); err == nil {
			t.Errorf("Head(%q) succeeded", p)
		}
	}
	// A tracked file swapped for a symlink to the right content is refused too.
	d3, root3 := diskTreeFixture(t)
	os.Remove(filepath.Join(root3, "s.sh"))
	write(t, filepath.Join(root3, "other"), "#!/bin/sh\necho hi\n", 0o755)
	os.Symlink("other", filepath.Join(root3, "s.sh"))
	if _, err := d3.Head("s.sh", 10); err == nil {
		t.Fatal("followed a link standing in for a tracked file")
	}
}

func TestReadVerifiedHeadBounds(t *testing.T) {
	d := canonTemp(t)
	big := bytes.Repeat([]byte("z"), 5000)
	writeBytes(t, filepath.Join(d, "big"), big, 0o644)
	id := blobID(string(big))
	if got, err := readVerifiedHead(filepath.Join(d, "big"), id, 1<<20, 100); err != nil || len(got) != 100 {
		t.Fatalf("%d %v", len(got), err)
	}
	if _, err := readVerifiedHead(filepath.Join(d, "big"), id, 1000, 100); err == nil || !strings.Contains(err.Error(), "over") {
		t.Fatalf("a file over the limit was hashed: %v", err)
	}
	if _, err := readVerifiedHead(filepath.Join(d, "big"), "abc", 1<<20, 100); err == nil {
		t.Fatal("a bad object id")
	}
	// SHA-256 repositories work the same way.
	if got, err := readVerifiedHead(filepath.Join(d, "big"), blobID256(string(big)), 1<<20, 10); err != nil || len(got) != 10 {
		t.Fatalf("%d %v", len(got), err)
	}
}

// DiskTree is a traced.Tree, and the classifier's own contract holds: Stat
// answers from the sealed tree, with no disk.
func TestDiskTreeIsATracedTree(t *testing.T) {
	d, _ := diskTreeFixture(t)
	var tree traced.Tree = d
	if n := tree.Stat("ln"); n.Kind != traced.KindLink || n.Target != "s.sh" {
		t.Fatalf("%+v", n)
	}
	if n := tree.Stat("dir"); n.Kind != traced.KindDir {
		t.Fatalf("%+v", n)
	}
}
