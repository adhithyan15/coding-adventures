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

// underRunnerHome makes a fresh directory beneath the runner's real home
// directory, outside /tmp and /var/tmp: the sandbox refuses a root beneath a
// home whose ancestors the step could rename (M2), so S1 needs a home whose
// ancestors (the runner's /home/runner, here) are created on the sealed root
// tmpfs.  No root is needed: the runner user owns its home.
func underRunnerHome(t *testing.T) string {
	t.Helper()
	uh, err := os.UserHomeDir()
	if err == nil {
		uh, err = filepath.EvalSymlinks(uh)
	}
	if err != nil || uh == "" || uh == "/" || uh == "/tmp" || strings.HasPrefix(uh, "/tmp/") || strings.HasPrefix(uh, "/var/tmp/") {
		skipOrFail(t, "no home directory outside /tmp to build the fixture home in (%q, %v)", uh, err)
	}
	home, err := os.MkdirTemp(uh, ".kiln-s1-")
	if err != nil {
		skipOrFail(t, "cannot create a directory in %s: %v", uh, err)
	}
	t.Cleanup(func() { os.RemoveAll(home) })
	return home
}

// TestSandboxROBeneathHome binds a tool root two levels under $HOME
// (~/.tools/bin, next to a writable state root ~/.tools/cache, as ~/.cargo/bin
// sits next to ~/.cargo/registry) and one directly under it (~/.direct), and
// checks that:
//
//   - the step reads them at their own paths, through $HOME's fresh tmpfs;
//   - it cannot write into them, and $HOME itself stays writable;
//   - it cannot swap a mount point: not the roots themselves (EBUSY), and not
//     their ancestor ~/.tools (renaming a directory that merely CONTAINS a
//     mount succeeds in the kernel, which is why the anchor is a read-only
//     tmpfs), and not an ancestor of $HOME itself (M2: those live on the
//     sealed root tmpfs, so `mv "$(dirname "$HOME")"` fails);
//   - the writable state root beneath the anchor is still writable and is the
//     host's directory.
func TestSandboxROBeneathHome(t *testing.T) {
	f := newSandboxFixture(t)
	home := underRunnerHome(t)
	bin := filepath.Join(home, ".tools", "bin")
	direct := filepath.Join(home, ".direct")
	cache := filepath.Join(home, ".tools", "cache")
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
	if err := os.WriteFile(filepath.Join(home, "host-secret"), []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	var env []string
	for _, e := range f.env {
		if strings.HasPrefix(e, "HOME=") {
			e = "HOME=" + home
		}
		env = append(env, e)
	}
	f.writeConfig(t, func(c *sandbox.Config) {
		c.Home = home
		c.RoRoots = append(imageRoots(), bin, direct)
		c.StateRoots = append(c.StateRoots, cache)
		c.EnvByAction = map[string][]string{"pkg": env}
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
		// A direct child of home is its own mount point: neither rm nor mv
		// can take it away (M1's regression for the real-directory case).
		try("rename-direct", `mv "$HOME/.direct" "$HOME/.d2"`),
		try("remove-direct", `rmdir "$HOME/.direct"`),
		try("rm-rf-direct", `rm -rf "$HOME/.direct"`),
		try("remove-direct-file", `rm -f "$HOME/.direct/d"`),
		// M2: the ancestors of home are on the sealed root tmpfs.
		try("rename-home", `mv "$HOME" "$HOME.gone"`),
		try("rename-home-parent", `mv "$(dirname "$HOME")" "$(dirname "$HOME").gone"`),
		try("make-beside-home", `mkdir "$(dirname "$HOME")/sibling"`),
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
		"rm-rf-direct:fail", "remove-direct-file:fail",
		"rename-home:fail", "rename-home-parent:fail", "make-beside-home:fail",
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
	for _, p := range []string{filepath.Join(bin, "x"), filepath.Join(direct, "x"), filepath.Join(home, ".tools", "y"),
		filepath.Join(home, ".tools", "z"), filepath.Join(home, "ok"), filepath.Join(home, ".moved"),
		home + ".gone", filepath.Dir(home) + ".gone", filepath.Join(filepath.Dir(home), "sibling")} {
		if _, err := os.Lstat(p); err == nil {
			t.Errorf("%s exists on the host", p)
		}
	}
	if b, err := os.ReadFile(filepath.Join(direct, "d")); err != nil || string(b) != "direct\n" {
		t.Errorf("the direct root's file on the host: %q, %v", b, err)
	}
}

// TestSandboxROBeneathHomeRefusals runs the layouts the review found, through
// the real command: each is refused with status 125 before anything is mounted,
// and the step never runs.
//
//   - M1: a symlink root beneath home (the sandbox would recreate it as a plain
//     inode of home's tmpfs that the step could `rm` and replace);
//   - M2: a root beneath a home that lies inside /tmp (the step could rename
//     home's ancestors and rebuild the root itself).
func TestSandboxROBeneathHomeRefusals(t *testing.T) {
	f := newSandboxFixture(t)
	marker := filepath.Join(f.pkg, "ran")
	run := func(want string) {
		t.Helper()
		r := f.sandboxRun(t, "pkg", "sh", "-c", "touch "+marker)
		if r.code != sandbox.ExitFailure || !strings.Contains(r.stderr, want) {
			t.Errorf("exit %d, stderr %q; want 125 and %q", r.code, r.stderr, want)
		}
		if _, err := os.Stat(marker); err == nil {
			t.Error("a refused configuration still ran the step")
		}
	}

	// M1, with a home outside /tmp so that only the symlink is wrong.
	home := underRunnerHome(t)
	real := filepath.Join(f.base, "real-tool-dir")
	if err := os.MkdirAll(real, 0o755); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(home, ".dotnet")
	if err := os.Symlink(real, link); err != nil {
		t.Fatal(err)
	}
	f.writeConfig(t, func(c *sandbox.Config) {
		c.Home = home
		c.RoRoots = append(imageRoots(), link)
	})
	run("is a symlink")

	// M2: the fixture's own home is under /tmp.
	bin := filepath.Join(f.home, ".cargo", "bin")
	if err := os.MkdirAll(bin, 0o755); err != nil {
		t.Fatal(err)
	}
	f.writeConfig(t, func(c *sandbox.Config) {
		c.RoRoots = append(imageRoots(), bin)
	})
	run("rename home's ancestors")
}
