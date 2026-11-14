package sandbox

import (
	"errors"
	"fmt"
	"os"
	"path"
	"path/filepath"
)

// HostFacts are the observations CheckHost compares the configuration with.
// The caller gathers them; keeping them as plain values lets the checks run
// (and be tested) on any OS.
type HostFacts struct {
	// Exe is what /proc/self/exe resolves to.
	Exe string
	// Cwd is the working directory the step will start in.
	Cwd string
	// ConfigPath is the configuration file's own path.
	ConfigPath string
}

// CheckHost verifies the configuration against the real filesystem before
// anything is mounted.  Every path the sandbox mounts onto must be CANONICAL
// (no symlink in any component): the mounts are made at "<new root>/<path>",
// and an absolute symlink in a component would resolve against the host's
// root, not the new one, and put a mount somewhere the plan never meant.
func (c *Config) CheckHost(action string, h HostFacts) error {
	if h.Exe != c.KilnExe {
		return fmt.Errorf("sandbox-exec: the running binary is %q, but the configuration names %q", clip(h.Exe), clip(c.KilnExe))
	}
	if want := c.PackageDir(action); h.Cwd != want {
		return fmt.Errorf("sandbox-exec: the working directory %q is not the package directory %q of KILN_ACTION", clip(h.Cwd), clip(want))
	}
	if err := checkPath("config path", h.ConfigPath); err != nil {
		return err
	}
	// The configuration is read host-side, before the pivot, and then trusted:
	// it must not be somewhere a step can reach. After the pivot a host path is
	// visible only if it lies inside repo_root, a state root or an ro root
	// (every bind is non-recursive, and Validate refuses an ro root that is or
	// contains home), and tmpdir is the staging area. $HOME itself is a fresh
	// tmpfs the host's copy never appears in, so a configuration under home that
	// none of those exposes is safe. The exposure set is checked in both
	// directions.
	for _, w := range append(append([]string{c.RepoRoot, c.Tmpdir}, c.StateRoots...), c.RoRoots...) {
		if within(h.ConfigPath, w) || within(w, h.ConfigPath) {
			return fmt.Errorf("sandbox-exec: the configuration lies inside or over %q, where a step could reach or rewrite it", clip(w))
		}
	}
	for _, d := range []struct{ what, p string }{
		{"repo_root", c.RepoRoot}, {"home", c.Home}, {"tmpdir", c.Tmpdir}, {"working directory", h.Cwd},
	} {
		if err := canonicalDir(d.what, d.p); err != nil {
			return err
		}
	}
	for _, r := range c.RoRoots {
		if err := checkRoRoot(r); err != nil {
			return err
		}
		if _, beneath := HomeAnchor(c.Home, r); beneath {
			if err := checkRoRootBeneathHome(r); err != nil {
				return err
			}
		}
	}
	for _, s := range c.StateRoots {
		if err := canonicalOrCreatable("state root", s); err != nil {
			return err
		}
	}
	for _, m := range c.Masks {
		if err := checkMaskTarget(m); err != nil {
			return err
		}
	}
	// The binary: a real file, outside the checkout (Validate checked the
	// names; this is the symlink-proof half).
	real, err := filepath.EvalSymlinks(h.Exe)
	if err != nil {
		return fmt.Errorf("sandbox-exec: cannot resolve the kiln binary: %v", err)
	}
	for _, w := range append([]string{c.RepoRoot, c.Tmpdir}, c.StateRoots...) {
		if within(real, w) {
			return fmt.Errorf("sandbox-exec: the kiln binary is inside %q", clip(w))
		}
	}
	if real != h.Exe {
		return fmt.Errorf("sandbox-exec: the kiln binary path %q is not canonical (it resolves to %q)", clip(h.Exe), clip(real))
	}
	// The configuration file: canonical too, and not where a step writes.
	realCfg, err := filepath.EvalSymlinks(h.ConfigPath)
	if err != nil {
		return fmt.Errorf("sandbox-exec: cannot resolve the configuration: %v", err)
	}
	if realCfg != h.ConfigPath {
		return fmt.Errorf("sandbox-exec: the configuration path %q is not canonical (it resolves to %q)", clip(h.ConfigPath), clip(realCfg))
	}
	return nil
}

// canonicalDir requires p to be a real directory reached through no symlink.
func canonicalDir(what, p string) error {
	st, err := os.Lstat(p)
	if err != nil {
		return fmt.Errorf("sandbox-exec: %s %q: %v", what, clip(p), err)
	}
	if !st.IsDir() {
		return fmt.Errorf("sandbox-exec: %s %q is not a real directory", what, clip(p))
	}
	return mustBeCanonical(what, p)
}

func mustBeCanonical(what, p string) error {
	real, err := filepath.EvalSymlinks(p)
	if err != nil {
		return fmt.Errorf("sandbox-exec: %s %q: %v", what, clip(p), err)
	}
	if real != p {
		return fmt.Errorf("sandbox-exec: %s %q is not canonical (it resolves to %q)", what, clip(p), clip(real))
	}
	return nil
}

// deepestExisting returns the nearest ancestor of p (or p itself) that exists.
func deepestExisting(p string) (string, error) {
	for {
		if _, err := os.Lstat(p); err == nil {
			return p, nil
		} else if !errors.Is(err, os.ErrNotExist) {
			return "", err
		}
		if p == "/" || p == "." {
			return "", errors.New("no existing ancestor")
		}
		p = path.Dir(p)
	}
}

// canonicalOrCreatable requires a state root to be a canonical directory, or
// absent with a canonical directory as its nearest existing ancestor (the
// sandbox creates the rest).
func canonicalOrCreatable(what, p string) error {
	near, err := deepestExisting(p)
	if err != nil {
		return fmt.Errorf("sandbox-exec: %s %q: %v", what, clip(p), err)
	}
	if err := canonicalDir(what, near); err != nil {
		return err
	}
	return nil
}

// checkMaskTarget checks one mask.  A target that exists must not be a symlink
// and must be reached through none.  One that does not exist is refused,
// wherever it is: a mask the sandbox cannot place would silently be no mask.
func checkMaskTarget(m string) error {
	st, err := os.Lstat(m)
	if errors.Is(err, os.ErrNotExist) {
		return fmt.Errorf("sandbox-exec: mask %q does not exist", clip(m))
	}
	if err != nil {
		return fmt.Errorf("sandbox-exec: mask %q: %v", clip(m), err)
	}
	if st.Mode()&os.ModeSymlink != 0 {
		return fmt.Errorf("sandbox-exec: mask %q is a symlink", clip(m))
	}
	return mustBeCanonical("mask", m)
}

// checkRoRoot requires an allowed read-only root to exist, and to be either a
// symlink (the entry itself, recreated in the new root) or a canonical
// directory or file.
func checkRoRoot(p string) error {
	st, err := os.Lstat(p)
	if err != nil {
		return fmt.Errorf("sandbox-exec: ro root %q: %v", clip(p), err)
	}
	if st.Mode()&os.ModeSymlink != 0 {
		// The entry is recreated under <new root>/<parent>: the parent must not
		// resolve through a host symlink into some other host directory.
		return mustBeCanonical("ro root parent", filepath.Dir(p))
	}
	return mustBeCanonical("ro root", p)
}

// checkRoRootBeneathHome refuses a symlink root beneath home.  A recreated
// symlink is an ordinary inode in home's step-writable tmpfs, not a mount
// point, so the kernel would let the step `rm` it and build a directory of its
// own at the same path (S1, KILN04 section 4.7); a bind of a real directory or
// file is a mount point and cannot be removed or renamed (EBUSY).  Roots beneath
// home must be real directories or files.
func checkRoRootBeneathHome(p string) error {
	st, err := os.Lstat(p)
	if err != nil {
		return fmt.Errorf("sandbox-exec: ro root %q: %v", clip(p), err)
	}
	if st.Mode()&os.ModeSymlink != 0 {
		return fmt.Errorf("sandbox-exec: ro root %q beneath home is a symlink; a recreated symlink is not a mount point and the step could replace it (list its target directory instead)", clip(p))
	}
	return nil
}
