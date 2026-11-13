//go:build linux && amd64

package main

// S1 (KILN04 section 4.7): read-only roots beneath $HOME, end to end.  The
// pure half (validation, mount order) is in internal/sandbox; this runs the
// real namespaces, so it lives in the required kiln-sandbox job.

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// TestSandboxROBeneathHome binds a tool root two levels under $HOME
// (~/.tools/bin, next to a writable state root ~/.tools/cache, as ~/.cargo/bin
// sits next to ~/.cargo/registry) and one directly under it (~/.direct), and
// checks that:
//
//   - the step reads them at their own paths, through $HOME's fresh tmpfs;
//   - it cannot write into them, and $HOME itself stays writable;
//   - it cannot swap a mount point: not the roots themselves (EBUSY), and not
//     their ancestor ~/.tools either (renaming a directory that merely CONTAINS
//     a mount succeeds in the kernel, which is why the anchor is a read-only
//     tmpfs: nothing in it can be created, renamed or removed);
//   - the writable state root beneath the anchor is still writable and is the
//     host's directory.
func TestSandboxROBeneathHome(t *testing.T) {
	f := newSandboxFixture(t)
	bin := filepath.Join(f.home, ".tools", "bin")
	direct := filepath.Join(f.home, ".direct")
	cache := filepath.Join(f.home, ".tools", "cache")
	for _, d := range []string{bin, direct} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.WriteFile(filepath.Join(bin, "hello"), []byte("tool\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(direct, "d"), []byte("direct\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	// Something at the host's $HOME that the sandbox must not show.
	if err := os.WriteFile(filepath.Join(f.home, "host-secret"), []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	f.writeConfig(t, func(c *sandbox.Config) {
		c.RoRoots = append(imageRoots(), bin, direct)
		c.StateRoots = append(c.StateRoots, cache)
	})
	try := func(name, cmd string) string {
		return `if ` + cmd + ` 2>/dev/null; then echo ` + name + `:ok; else echo ` + name + `:fail; fi`
	}
	script := strings.Join([]string{
		`cat "$HOME/.tools/bin/hello" "$HOME/.direct/d"`,
		try("write-in-root", `touch "$HOME/.tools/bin/x"`),
		try("write-in-direct", `touch "$HOME/.direct/x"`),
		try("write-in-anchor", `touch "$HOME/.tools/y"`),
		try("mkdir-in-anchor", `mkdir "$HOME/.tools/z"`),
		try("rename-anchor", `mv "$HOME/.tools" "$HOME/.moved"`),
		try("rename-root", `mv "$HOME/.tools/bin" "$HOME/.tools/bin2"`),
		try("remove-root", `rmdir "$HOME/.tools/bin"`),
		try("rename-direct", `mv "$HOME/.direct" "$HOME/.d2"`),
		try("remove-direct", `rmdir "$HOME/.direct"`),
		try("write-home", `touch "$HOME/ok"`),
		try("write-state", `echo state > "$HOME/.tools/cache/w"`),
		`test -e "$HOME/host-secret" && echo secret:visible || echo secret:hidden`,
		`awk '$5 == "'"$HOME"'/.tools" {print "anchor-mount:" $6}' /proc/self/mountinfo`,
	}, "; ")
	r := f.sandboxRun(t, "pkg", "sh", "-c", script)
	if r.code != 0 || r.stderr != "" {
		t.Fatalf("exit %d, stderr %q, stdout %q", r.code, r.stderr, r.stdout)
	}
	for _, want := range []string{
		"tool\ndirect\n",
		"write-in-root:fail", "write-in-direct:fail", "write-in-anchor:fail", "mkdir-in-anchor:fail",
		"rename-anchor:fail", "rename-root:fail", "remove-root:fail", "rename-direct:fail", "remove-direct:fail",
		"write-home:ok", "write-state:ok", "secret:hidden",
	} {
		if !strings.Contains(r.stdout, want) {
			t.Errorf("missing %q in:\n%s", want, r.stdout)
		}
	}
	if !strings.Contains(r.stdout, "anchor-mount:ro") {
		t.Errorf("the anchor is not a read-only mount:\n%s", r.stdout)
	}
	// The host: the state root's write landed there; nothing else did.
	if b, err := os.ReadFile(filepath.Join(cache, "w")); err != nil || string(b) != "state\n" {
		t.Errorf("the state root write: %q, %v", b, err)
	}
	for _, p := range []string{filepath.Join(bin, "x"), filepath.Join(direct, "x"), filepath.Join(f.home, ".tools", "y"),
		filepath.Join(f.home, ".tools", "z"), filepath.Join(f.home, "ok"), filepath.Join(f.home, ".moved")} {
		if _, err := os.Lstat(p); err == nil {
			t.Errorf("%s exists on the host", p)
		}
	}
}
