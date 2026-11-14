//go:build !windows

package sandbox

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestCheckHostRoRootBeneathHomeRefusesSymlink is M1: a symlink root beneath
// home would be recreated as a plain inode of home's writable tmpfs, which the
// step can rm and replace; only real directories and files are mount points.
func TestCheckHostRoRootBeneathHomeRefusesSymlink(t *testing.T) {
	f := newHostFixture(t)
	real := filepath.Join(f.base, "real")
	os.MkdirAll(real, 0o755)
	os.Symlink(real, filepath.Join(f.cfg.Home, ".dotnet"))
	os.MkdirAll(filepath.Join(f.cfg.Home, ".sub"), 0o755)
	os.Symlink(real, filepath.Join(f.cfg.Home, ".sub", "bin"))
	for _, link := range []string{filepath.Join(f.cfg.Home, ".dotnet"), filepath.Join(f.cfg.Home, ".sub", "bin")} {
		f.cfg.RoRoots = []string{"/usr", link}
		// No Validate here: CheckHost is the layer under test, and on Linux
		// t.TempDir() lies under /tmp, where Validate's M2 rule (home's
		// ancestors must be unmovable) rightly refuses any root beneath home.
		if err := f.cfg.CheckHost("pkg", f.facts()); err == nil || !strings.Contains(err.Error(), "symlink") {
			t.Errorf("%s: %v", link, err)
		}
	}
	// A symlink root OUTSIDE home (/bin) is still fine, and so are a real
	// directory and a file beneath home.
	os.MkdirAll(filepath.Join(f.cfg.Home, ".realdir"), 0o755)
	os.WriteFile(filepath.Join(f.cfg.Home, ".realfile"), nil, 0o644)
	outside := filepath.Join(f.base, "rolink")
	os.Symlink(real, outside)
	f.cfg.RoRoots = []string{"/usr", outside, filepath.Join(f.cfg.Home, ".realdir"), filepath.Join(f.cfg.Home, ".realfile")}
	if err := f.cfg.CheckHost("pkg", f.facts()); err != nil {
		t.Fatal(err)
	}
	// A missing one is an error from either check.
	if err := checkRoRootBeneathHome(filepath.Join(f.cfg.Home, ".absent")); err == nil {
		t.Error("an absent root accepted")
	}
}
