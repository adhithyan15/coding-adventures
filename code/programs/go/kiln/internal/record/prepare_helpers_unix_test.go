//go:build unix

package record

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

func TestExecToolReportsOutputAndExit(t *testing.T) {
	d := canonTemp(t)
	if err := execTool(ToolRun{Path: "/bin/sh", Args: []string{"-c", "echo fine"}, Dir: d, Env: []string{"PATH=/bin"}, Timeout: 10 * time.Second}); err != nil {
		t.Fatal(err)
	}
	err := execTool(ToolRun{Path: "/bin/sh", Args: []string{"-c", "echo boom >&2; echo out; exit 3"}, Dir: d, Env: []string{"PATH=/bin"}, Timeout: 10 * time.Second})
	if err == nil || !strings.Contains(err.Error(), "exit status 3") || !strings.Contains(err.Error(), "boom") || !strings.Contains(err.Error(), "out") {
		t.Fatalf("%v", err)
	}
	// Output is capped: a chatty failing tool cannot fill the error message.
	err = execTool(ToolRun{Path: "/bin/sh", Args: []string{"-c", "head -c 100000 /dev/zero | tr '\\0' x; exit 1"}, Dir: d, Env: []string{"PATH=/usr/bin:/bin"}, Timeout: 10 * time.Second})
	if err == nil || len(err.Error()) > 5000 {
		t.Fatalf("%d bytes", len(err.Error()))
	}
	// The environment is exactly what was given: no inherited credential.
	t.Setenv("KILN_TEST_SECRET_VALUE", "leak")
	err = execTool(ToolRun{Path: "/bin/sh", Args: []string{"-c", `test -z "$KILN_TEST_SECRET_VALUE" || { echo leaked; exit 9; }`}, Dir: d, Env: []string{"PATH=/bin"}, Timeout: 10 * time.Second})
	if err != nil {
		t.Fatalf("the child inherited the process environment: %v", err)
	}
	if err := execTool(ToolRun{Path: "/bin/sh", Args: []string{"-c", "exec sleep 30"}, Dir: d, Env: []string{"PATH=/bin:/usr/bin"}, Timeout: 200 * time.Millisecond}); err == nil {
		t.Fatal("a tool that outlived its timeout was not stopped")
	}
	if err := execTool(ToolRun{Path: filepath.Join(d, "absent"), Dir: d, Timeout: time.Second}); err == nil {
		t.Fatal("a missing tool ran")
	}
}

func TestCapBufferKeepsTheFirstBytes(t *testing.T) {
	var c capBuffer
	n, err := c.Write([]byte(strings.Repeat("a", 5000)))
	if n != 5000 || err != nil || len(c.String()) != 4096 {
		t.Fatalf("%d %v %d", n, err, len(c.String()))
	}
	if n, _ := c.Write([]byte("more")); n != 4 || len(c.String()) != 4096 {
		t.Fatal("kept more than the cap")
	}
}

func script(t *testing.T, body string) string {
	t.Helper()
	p := filepath.Join(canonTemp(t), "strace")
	writeBytes(t, p, []byte("#!/bin/sh\n"+body+"\n"), 0o755)
	return p
}

func TestStraceVersion(t *testing.T) {
	got, err := straceVersion(script(t, `echo "strace -- version 6.8"; echo "Copyright (c) 1991-2024"`))
	if err != nil || got != "strace -- version 6.8" {
		t.Fatalf("%q %v", got, err)
	}
	if _, err := straceVersion(script(t, `exit 1`)); err == nil {
		t.Fatal("a failing strace was accepted")
	}
	if _, err := straceVersion(script(t, `true`)); err == nil || !strings.Contains(err.Error(), "printed nothing") {
		t.Fatalf("%v", err)
	}
	if _, err := straceVersion(filepath.Join(t.TempDir(), "absent")); err == nil {
		t.Fatal("a missing strace was accepted")
	}
}

func TestKernelRelease(t *testing.T) {
	got, err := kernelRelease()
	if runtime.GOOS == "linux" {
		if err != nil || got == "" || strings.ContainsAny(got, "\n") {
			t.Fatalf("%q %v", got, err)
		}
		return
	}
	if err == nil {
		t.Fatalf("%q: no /proc here, yet no error", got)
	}
}

func TestEnsureTmpdir(t *testing.T) {
	d := canonTemp(t)
	fresh := filepath.Join(d, "fresh")
	if err := ensureTmpdir(fresh); err != nil {
		t.Fatal(err)
	}
	if st, err := os.Stat(fresh); err != nil || st.Mode().Perm() != 0o700 {
		t.Fatalf("%v %v", st, err)
	}
	if err := ensureTmpdir(fresh); err != nil { // existing and empty
		t.Fatal(err)
	}
	file := filepath.Join(d, "file")
	write(t, file, "x", 0o644)
	for name, p := range map[string]string{
		"a file":                    file,
		"a missing parent":          filepath.Join(d, "no", "such", "dir"),
		"a FIFO":                    mkfifo(t, d),
		"a not-empty tmpdir":        func() string { write(t, filepath.Join(d, "full", "x"), "x", 0o644); return filepath.Join(d, "full") }(),
		"a tmpdir behind a symlink": func() string { l := filepath.Join(d, "lnk"); os.Symlink(fresh, l); return l }(),
	} {
		if err := ensureTmpdir(p); err == nil {
			t.Errorf("%s was accepted", name)
		}
	}
	// A parent we cannot look inside: an error that is not "does not exist".
	if os.Geteuid() != 0 {
		locked := filepath.Join(d, "locked")
		write(t, filepath.Join(locked, "x"), "x", 0o644)
		os.Chmod(locked, 0)
		defer os.Chmod(locked, 0o755)
		if err := ensureTmpdir(filepath.Join(locked, "x")); err == nil {
			t.Error("an unreadable parent was accepted")
		}
	}
}

func TestSealWriterWritesOnceAndAdoptsOnlyRegularFiles(t *testing.T) {
	d := canonTemp(t)
	w := &sealWriter{pre: d, sums: map[string]string{}}
	if err := w.write("a.json", []byte("x")); err != nil {
		t.Fatal(err)
	}
	// O_EXCL: a file that is already there is never overwritten.
	if err := w.write("a.json", []byte("y")); err == nil {
		t.Fatal("overwrote a file")
	}
	if got, _ := os.ReadFile(filepath.Join(d, "a.json")); string(got) != "x" {
		t.Fatalf("%q", got)
	}
	if err := w.write("no/dir/b.json", []byte("x")); err == nil {
		t.Fatal("wrote into a missing directory")
	}
	// adopt: a regular file, never a symlink, a FIFO or a missing file.
	write(t, filepath.Join(d, "tool-wrote"), "data", 0o644)
	if err := w.adopt("tool-wrote"); err != nil || w.sums["tool-wrote"] != sumOf([]byte("data")) {
		t.Fatalf("%v %v", err, w.sums)
	}
	if err := os.Symlink(filepath.Join(d, "tool-wrote"), filepath.Join(d, "link")); err != nil {
		t.Fatal(err)
	}
	if err := syscall.Mkfifo(filepath.Join(d, "fifo"), 0o600); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"link", "fifo", "missing"} {
		if err := w.adopt(name); err == nil {
			t.Errorf("adopted %s", name)
		}
	}
}

func TestSealWriterSealFailsWhenSealedAlready(t *testing.T) {
	d := canonTemp(t)
	w := &sealWriter{pre: d, sums: map[string]string{}}
	write(t, filepath.Join(d, "SEALED"), "x", 0o644)
	if err := w.seal(); err == nil {
		t.Fatal("sealed over an existing SEALED")
	}
}

func TestPrepareAcceptsAnExistingEmptyDirectory(t *testing.T) {
	p := newPrep(t)
	p.commit()
	if err := os.Mkdir(p.dir, 0o700); err != nil {
		t.Fatal(err)
	}
	p.mustPrepare()
}

func TestPrepareRefusesTheKilnBinaryInsideAReadOnlyRoot(t *testing.T) {
	if realHome == "" {
		t.Skip("no home directory")
	}
	root, err := os.MkdirTemp(realHome, ".kiln-prep-root-")
	if err != nil {
		t.Skipf("cannot make a directory under the home directory: %v", err)
	}
	defer os.RemoveAll(root)
	if root, err = filepath.EvalSymlinks(root); err != nil {
		t.Fatal(err)
	}
	p := newPrep(t)
	p.roots = []string{`{"path": "` + root + `", "kind": "image"}`}
	p.commit()
	writeBytes(t, filepath.Join(root, "kiln"), []byte("k"), 0o755)
	p.kiln = filepath.Join(root, "kiln")
	t.Cleanup(p.writable)
	// The binary's own directory is an ro root, so a BUILD could read it; and
	// the root overlaps the kiln binary (the private-region check says so
	// first, which is the same refusal from the other side).
	_, err = Prepare(p.options())
	if err == nil || !strings.Contains(err.Error(), "overlaps the kiln binary") {
		t.Fatalf("%v", err)
	}
}

func TestCheckDirArgAndResolveBinaryOnUnreadablePaths(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root reads everything")
	}
	d := canonTemp(t)
	locked := filepath.Join(d, "locked")
	if err := os.Mkdir(locked, 0o700); err != nil {
		t.Fatal(err)
	}
	write(t, filepath.Join(locked, "bin"), "x", 0o755)
	if err := os.Chmod(locked, 0); err != nil {
		t.Fatal(err)
	}
	defer os.Chmod(locked, 0o755)
	if _, err := checkDirArg(filepath.Join(locked, "rec")); err == nil || strings.Contains(err.Error(), "no such file") {
		t.Fatalf("%v", err)
	}
	if _, err := ResolveBinary("bin", filepath.Join(locked, "bin")); err == nil {
		t.Fatal("an unreadable binary was hashed")
	}
	// An existing empty directory that cannot be listed.
	unlisted := filepath.Join(d, "unlisted")
	if err := os.Mkdir(unlisted, 0o300); err != nil { // write+execute, no read
		t.Fatal(err)
	}
	defer os.Chmod(unlisted, 0o755)
	if _, err := checkDirArg(unlisted); err == nil {
		t.Fatal("a directory that cannot be listed was accepted")
	}
	// A file that exists but cannot be opened for reading.
	f := filepath.Join(d, "noread")
	write(t, f, "x", 0o755)
	os.Chmod(f, 0)
	defer os.Chmod(f, 0o644)
	if _, err := ResolveBinary("bin", f); err == nil {
		t.Fatal("an unreadable file was hashed")
	}
}

func TestScanTreeReportsAnUnreadableDirectory(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root reads everything")
	}
	root := cleanRepo(t)
	d := filepath.Join(root, "locked")
	if err := os.Mkdir(d, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Chmod(d, 0); err != nil {
		t.Fatal(err)
	}
	defer os.Chmod(d, 0o755)
	if err := ScanTree(root); err == nil || !strings.Contains(err.Error(), "permission denied") {
		t.Fatalf("%v", err)
	}
}

// The formats the shard writes are decoded by the same recfmt package the merge
// uses; a binding built here must survive that decoder's own validation.
func TestBindingFromPrepareIsValid(t *testing.T) {
	p := newPrep(t)
	p.commit()
	res := p.mustPrepare()
	if err := res.Binding.Validate(); err != nil {
		t.Fatal(err)
	}
	if res.Binding.ShardIndex != 0 || res.Binding.Recorder != legacy.RecorderVersion {
		t.Fatalf("%+v", res.Binding)
	}
}
