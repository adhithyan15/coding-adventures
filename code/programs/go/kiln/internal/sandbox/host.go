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
	// The configuration is read, then trusted: it must not be somewhere a step
	// can write.
	for _, w := range append([]string{c.RepoRoot, c.Home, c.Tmpdir}, c.StateRoots...) {
		if within(h.ConfigPath, w) {
			return fmt.Errorf("sandbox-exec: the configuration lies inside %q, where a step could rewrite it", clip(w))
		}
	}
	for _, d := range []struct{ what, p string }{
		{"repo_root", c.RepoRoot}, {"home", c.Home}, {"tmpdir", c.Tmpdir}, {"working directory", h.Cwd},
	} {
		if err := canonicalDir(d.what, d.p); err != nil {
			return err
		}
	}
	for _, s := range c.StateRoots {
		if err := canonicalOrCreatable("state root", s); err != nil {
			return err
		}
	}
	for _, m := range c.Masks {
		if err := checkMaskTarget(m, within(m, c.RepoRoot)); err != nil {
			return err
		}
	}
	// The binary: a real file, outside the checkout (Validate checked the
	// names; this is the symlink-proof half).
	real, err := filepath.EvalSymlinks(h.Exe)
	if err != nil {
		return fmt.Errorf("sandbox-exec: cannot resolve the kiln binary: %v", err)
	}
	if within(real, c.RepoRoot) {
		return errors.New("sandbox-exec: the kiln binary is inside repo_root")
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
// and must be reached through none.  One that does not exist is skipped, except
// inside the checkout: there the step could create it, so a missing mask is a
// refusal rather than a hole.
func checkMaskTarget(m string, inRepo bool) error {
	st, err := os.Lstat(m)
	if errors.Is(err, os.ErrNotExist) {
		if inRepo {
			return fmt.Errorf("sandbox-exec: mask %q does not exist inside the checkout, so a step could create it", clip(m))
		}
		near, derr := deepestExisting(m)
		if derr != nil {
			return fmt.Errorf("sandbox-exec: mask %q: %v", clip(m), derr)
		}
		return mustBeCanonical("mask ancestor", near)
	}
	if err != nil {
		return fmt.Errorf("sandbox-exec: mask %q: %v", clip(m), err)
	}
	if st.Mode()&os.ModeSymlink != 0 {
		return fmt.Errorf("sandbox-exec: mask %q is a symlink", clip(m))
	}
	return mustBeCanonical("mask", m)
}
