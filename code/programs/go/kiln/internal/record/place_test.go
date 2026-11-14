//go:build unix

package record

import (
	"crypto/sha256"
	"encoding/hex"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
)

// canonTemp is a temp directory whose path has no symlink in it (macOS's
// $TMPDIR lives behind /var -> /private/var, which Canonical refuses by design).
func canonTemp(t *testing.T) string {
	t.Helper()
	d, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	return d
}

// mkfifo makes a FIFO named fifo in d and returns its path.
func mkfifo(t *testing.T, d string) string {
	t.Helper()
	p := filepath.Join(d, "fifo")
	if err := syscall.Mkfifo(p, 0o600); err != nil {
		t.Fatal(err)
	}
	return p
}

func write(t *testing.T, p, content string, mode os.FileMode) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(p, []byte(content), mode); err != nil {
		t.Fatal(err)
	}
}

func TestWithin(t *testing.T) {
	for _, c := range []struct {
		p, root string
		want    bool
	}{
		{"/a/b", "/a", true}, {"/a", "/a", true}, {"/ab", "/a", false}, {"/a", "/a/b", false}, {"/", "/a", false},
		{"/a/bc", "/a/b", false}, {"/a/b/c", "/a/b", true},
	} {
		if got := within(c.p, c.root); got != c.want {
			t.Errorf("within(%q, %q) = %v", c.p, c.root, got)
		}
		if got := overlap(c.p, c.root); got != (c.want || within(c.root, c.p)) {
			t.Errorf("overlap(%q, %q) = %v", c.p, c.root, got)
		}
	}
}

func TestResolveBinaryHashesTheFile(t *testing.T) {
	d := canonTemp(t)
	p := filepath.Join(d, "kiln")
	write(t, p, "binary bytes", 0o755)
	b, err := ResolveBinary("kiln", p)
	if err != nil {
		t.Fatal(err)
	}
	sum := sha256.Sum256([]byte("binary bytes"))
	if b.Path != p || b.SHA256 != hex.EncodeToString(sum[:]) {
		t.Fatalf("%+v", b)
	}
}

func TestResolveBinaryRefusals(t *testing.T) {
	d := canonTemp(t)
	real := filepath.Join(d, "real")
	write(t, real, "x", 0o755)
	if err := os.Symlink(real, filepath.Join(d, "link")); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(d, filepath.Join(d, "dirlink")); err != nil {
		t.Fatal(err)
	}
	if err := os.Mkdir(filepath.Join(d, "adir"), 0o755); err != nil {
		t.Fatal(err)
	}
	for name, c := range map[string]struct{ path, want string }{
		"relative":          {"kiln", "not an absolute"},
		"unclean":           {d + "/a/../real", "not clean"},
		"missing":           {filepath.Join(d, "nope"), "no such file"},
		"symlinked file":    {filepath.Join(d, "link"), "not canonical"},
		"symlinked parent":  {filepath.Join(d, "dirlink", "real"), "not canonical"},
		"a directory":       {filepath.Join(d, "adir"), "not a regular file"},
		"a FIFO is refused": {mkfifo(t, d), ""},
	} {
		_, err := ResolveBinary("the binary", c.path)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: %v", name, err)
		}
	}
}

func TestCheckOutside(t *testing.T) {
	regions := []Region{{"the checkout", "/repo"}, {"tmpdir", "/tmp/kiln-tmp"}, {"nothing", ""}}
	if err := CheckOutside("kiln", "/opt/bin/kiln", regions); err != nil {
		t.Fatal(err)
	}
	for _, p := range []string{"/repo/kiln", "/repo", "/tmp/kiln-tmp/x/kiln"} {
		if err := CheckOutside("kiln", p, regions); err == nil {
			t.Errorf("%s accepted", p)
		}
	}
	// A sibling that merely shares a prefix is outside.
	if err := CheckOutside("kiln", "/repository/kiln", regions); err != nil {
		t.Fatal(err)
	}
}

func TestCheckRoRoots(t *testing.T) {
	private := []Region{{"the record directory", "/home/runner/work/_temp/kiln-record"}, {"empty", ""}}
	// /home/runner/.rustup does not overlap the record directory: fine.
	if err := CheckRoRoots([]string{"/usr", "/etc", "/home/runner/.rustup"}, private); err != nil {
		t.Fatal(err)
	}
	for _, r := range []string{"/home", "/home/runner/work", "/home/runner/work/_temp/kiln-record", "/home/runner/work/_temp/kiln-record/pre"} {
		if err := CheckRoRoots([]string{r}, private); err == nil {
			t.Errorf("%s accepted", r)
		}
	}
}

func TestCheckDirArg(t *testing.T) {
	d := canonTemp(t)
	// absent, parent canonical: to be created
	if create, err := checkDirArg(filepath.Join(d, "rec")); err != nil || !create {
		t.Fatalf("%v %v", create, err)
	}
	// existing and empty
	empty := filepath.Join(d, "empty")
	if err := os.Mkdir(empty, 0o700); err != nil {
		t.Fatal(err)
	}
	if create, err := checkDirArg(empty); err != nil || create {
		t.Fatalf("%v %v", create, err)
	}
	full := filepath.Join(d, "full")
	write(t, filepath.Join(full, "stale"), "x", 0o644)
	file := filepath.Join(d, "file")
	write(t, file, "x", 0o644)
	if err := os.Symlink(empty, filepath.Join(d, "link")); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(d, filepath.Join(d, "dirlink")); err != nil {
		t.Fatal(err)
	}
	for name, c := range map[string]struct{ dir, want string }{
		"relative":         {"rec", "absolute"},
		"unclean":          {d + "/x/../rec", "clean"},
		"not empty":        {full, "not empty"},
		"a file":           {file, "not a directory"},
		"a symlink":        {filepath.Join(d, "link"), "symlink"},
		"parent symlinked": {filepath.Join(d, "dirlink", "rec"), "not canonical"},
		"missing parent":   {filepath.Join(d, "no", "rec"), "no such file"},
	} {
		if _, err := checkDirArg(c.dir); err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("%s: %v", name, err)
		}
	}
}
