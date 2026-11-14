package record

// Binary placement (KILN04 section 4.8, stage 4b-1, decision P3; D15)
// ===================================================================
//
// The recorder trusts three programs: Kiln itself, the build tool that exports
// the graph and runs the BUILDs, and strace. A BUILD runs in a sandbox that can
// write the checkout, and the whole argument that "the recorder's own files
// cannot be rewritten by what it records" rests on those programs living
// somewhere the checkout, the scratch directories and the state roots do not
// reach:
//
//	              +------------------+      a BUILD can write here
//	   checkout --|  R/              |  <-- (and can run anything it wrote)
//	              +------------------+
//	   kiln, build-tool  must be OUTSIDE R, outside TMPDIR, outside every state
//	                     root, and outside every ro_roots entry (a read-only
//	                     root is visible to the BUILD: it need not read kiln)
//
// A path is only as good as what it RESOLVES to. `/tmp/bin/kiln` that is a
// symlink into the checkout passes a string test and fails the real one, so a
// binary must be a regular file whose path equals its own EvalSymlinks result:
// no symlink in any component. The same rule gives sandbox.CheckHost its
// canonical `kiln_exe`, so the path we record is the path the sandbox will
// demand.
//
// The hash recorded in the binding is of the bytes that were opened here, with
// a non-following open and an fstat on the handle, so the file that was
// checked is the file that was hashed.

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
)

// Binary is a resolved, hashed program.
type Binary struct {
	// Path is canonical: absolute, clean, and equal to its own resolution.
	Path string
	// SHA256 is the lowercase hex digest of the file's bytes.
	SHA256 string
}

// Region is a named place a program must stay out of.
type Region struct {
	Name string
	Path string
}

// within reports whether p is root or lies beneath it. Both are clean
// absolute POSIX paths, which is all the recorder ever handles (it refuses to
// run anywhere but Linux).
func within(p, root string) bool {
	return p == root || len(p) > len(root) && p[:len(root)] == root && p[len(root)] == '/'
}

// overlap reports whether either path lies within the other.
func overlap(a, b string) bool { return within(a, b) || within(b, a) }

// Canonical requires p to be absolute, clean and free of symlinks in every
// component, and returns it. what names the path in the error.
func Canonical(what, p string) (string, error) {
	if !filepath.IsAbs(p) {
		return "", fmt.Errorf("%s %q is not an absolute path", what, printable(p))
	}
	if filepath.Clean(p) != p {
		return "", fmt.Errorf("%s %q is not clean", what, printable(p))
	}
	real, err := filepath.EvalSymlinks(p)
	if err != nil {
		return "", fmt.Errorf("%s %q: %v", what, printable(p), err)
	}
	if real != p {
		return "", fmt.Errorf("%s %q is not canonical (it resolves to %q): a symlink in the path could redirect it", what, printable(p), printable(real))
	}
	return p, nil
}

// ResolveBinary checks that p is a canonical regular file and hashes it.
func ResolveBinary(what, p string) (Binary, error) {
	if _, err := Canonical(what, p); err != nil {
		return Binary{}, err
	}
	f, err := openNoFollow(p)
	if err != nil {
		return Binary{}, fmt.Errorf("%s %q: %v", what, printable(p), err)
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil {
		return Binary{}, fmt.Errorf("%s %q: %v", what, printable(p), err)
	}
	if !st.Mode().IsRegular() {
		return Binary{}, fmt.Errorf("%s %q is not a regular file", what, printable(p))
	}
	h := sha256.New()
	if _, err := io.Copy(h, f); err != nil {
		return Binary{}, fmt.Errorf("%s %q: %v", what, printable(p), err)
	}
	return Binary{Path: p, SHA256: hex.EncodeToString(h.Sum(nil))}, nil
}

// CheckOutside refuses a path that lies inside any region. It is used for the
// two binaries and for the record directory.
func CheckOutside(what, p string, regions []Region) error {
	for _, r := range regions {
		if r.Path != "" && within(p, r.Path) {
			return fmt.Errorf("%s %q lies inside %s %q, where a BUILD could rewrite or read it", what, printable(p), r.Name, printable(r.Path))
		}
	}
	return nil
}

// CheckRoRoots refuses a read-only root that overlaps a path the recorder keeps
// to itself (the record directory, RUNNER_TEMP, the binaries, the directories
// of the GitHub command files). A root that CONTAINS one would make it visible
// to every BUILD; a root INSIDE one is refused for symmetry, because a root
// that lives in the recorder's scratch space is not a root of the image.
func CheckRoRoots(roRoots []string, private []Region) error {
	for _, r := range roRoots {
		for _, p := range private {
			if p.Path != "" && overlap(r, p.Path) {
				return fmt.Errorf("ro root %q overlaps %s %q, which the recorder keeps out of the sandbox", printable(r), p.Name, printable(p.Path))
			}
		}
	}
	return nil
}

// checkDirArg validates --dir and says whether it must be created. It must be
// absolute, clean, canonical in every ancestor, not a symlink itself, and
// either absent or an empty real directory: a directory with content is a
// previous attempt's leftovers, and mixing them with this run's would let a
// stale file be sealed as if it were fresh.
func checkDirArg(dir string) (create bool, err error) {
	if !filepath.IsAbs(dir) || filepath.Clean(dir) != dir {
		return false, fmt.Errorf("--dir %q must be an absolute, clean path", printable(dir))
	}
	st, err := os.Lstat(dir)
	switch {
	case os.IsNotExist(err):
		// Absent: its parent must be canonical, so that creating it cannot be
		// redirected through a symlink in an ancestor.
		if _, err := Canonical("the parent of --dir", filepath.Dir(dir)); err != nil {
			return false, err
		}
		return true, nil
	case err != nil:
		return false, fmt.Errorf("--dir %q: %v", printable(dir), err)
	case st.Mode()&os.ModeSymlink != 0:
		return false, fmt.Errorf("--dir %q is a symlink", printable(dir))
	case !st.IsDir():
		return false, fmt.Errorf("--dir %q is not a directory", printable(dir))
	}
	if _, err := Canonical("--dir", dir); err != nil {
		return false, err
	}
	entries, err := os.ReadDir(dir)
	if err != nil {
		return false, fmt.Errorf("--dir %q: %v", printable(dir), err)
	}
	if len(entries) != 0 {
		return false, fmt.Errorf("--dir %q is not empty: a record directory must be fresh", printable(dir))
	}
	if err := claimDir("--dir", dir, st); err != nil {
		return false, err
	}
	return false, nil
}

// claimDir takes over an EXISTING directory the recorder is about to keep
// private: it must belong to this user and must not be writable by group or
// others (anyone who can write it can swap what is in it between our checks and
// our use), and then its mode is forced to 0700. A directory we create ourselves
// is made 0700 at birth and needs no claim.
func claimDir(what, p string, st os.FileInfo) error {
	if uid, ok := ownerOf(st); ok && uid != uint32(os.Geteuid()) {
		return fmt.Errorf("%s %q is owned by uid %d, not by this user", what, printable(p), uid)
	}
	if st.Mode().Perm()&0o022 != 0 {
		return fmt.Errorf("%s %q is writable by group or others (mode %v)", what, printable(p), st.Mode().Perm())
	}
	if err := os.Chmod(p, 0o700); err != nil {
		return fmt.Errorf("%s %q: %v", what, printable(p), err)
	}
	return nil
}
