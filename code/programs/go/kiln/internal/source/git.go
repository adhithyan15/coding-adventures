package source

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// Git is a Source backed by a Git work tree.  It shells out to the `git`
// binary and NEVER through a shell: every argument is a separate element of
// the argv slice, so a filename or revision cannot be reinterpreted as shell
// syntax.  Lists are read with -z (NUL separated) because newlines, quotes and
// non-ASCII bytes are all legal in file names and git would otherwise quote
// them.
//
// Revision is the id of HEAD, which describes COMMITTED state.  A dirty work
// tree has the same Revision as its clean twin; Kiln keys come from file
// contents, never from Revision, so this only affects explanations.
type Git struct {
	dir string
}

// NewGit returns a Source for the work tree at dir.
func NewGit(dir string) *Git { return &Git{dir: dir} }

// run executes git in the work tree and returns stdout.
func (g *Git) run(args ...string) ([]byte, error) {
	full := append([]string{"-C", g.dir}, args...)
	cmd := exec.Command("git", full...)
	cmd.Env = append(os.Environ(), "GIT_OPTIONAL_LOCKS=0", "LC_ALL=C")
	var out, errb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &errb
	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("git %s: %w: %s", args[0], err, strings.TrimSpace(errb.String()))
	}
	return out.Bytes(), nil
}

// splitNUL splits NUL-terminated output into its records.
func splitNUL(b []byte) []string {
	out := []string{}
	for _, f := range bytes.Split(b, []byte{0}) {
		if len(f) > 0 {
			out = append(out, string(f))
		}
	}
	return out
}

// Files lists tracked files (`git ls-files -z`) under root matching patterns.
func (g *Git) Files(root string, patterns []string) ([]string, error) {
	out, err := g.run("ls-files", "-z", "--")
	if err != nil {
		return nil, err
	}
	return filter(splitNUL(out), root, patterns)
}

// Open opens a regular file of the work tree.
func (g *Git) Open(path string) (io.ReadCloser, error) { return openRegular(g.dir, path) }

// Revision returns `git rev-parse HEAD`.
func (g *Git) Revision() (string, error) {
	out, err := g.run("rev-parse", "HEAD")
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(string(out)), nil
}

// validRevisionText rejects text that could be mistaken for an option or
// that has no business in a revision name.  The leading "-" check is the
// option-injection defence ("--output=/x" would otherwise be taken by git as
// a flag); ".." would let a caller smuggle a range.
func validRevisionText(s string) error {
	if s == "" {
		return fmt.Errorf("empty revision")
	}
	if s[0] == '-' {
		return fmt.Errorf("revision %q looks like an option", s)
	}
	if strings.Contains(s, "..") {
		return fmt.Errorf("revision %q contains a range", s)
	}
	for _, r := range s {
		if r < 0x20 || r == 0x7f || r == ' ' {
			return fmt.Errorf("revision %q contains a control or space character", s)
		}
	}
	return nil
}

// Changed lists files changed between since and HEAD, using the
// three-dot form (changes on this side since the merge base), exactly what a
// pull request shows.  A revision git cannot resolve yields known=false, not
// an error: Changed only explains results and never decides correctness.
func (g *Git) Changed(since string) ([]string, bool, error) {
	if err := validRevisionText(since); err != nil {
		return nil, false, err
	}
	// Resolve to a full object id first.  After this the text handed to
	// `diff` is hex that git itself produced.
	out, err := g.run("rev-parse", "--verify", "--quiet", since+"^{commit}")
	if err != nil {
		return nil, false, nil
	}
	id := strings.TrimSpace(string(out))
	out, err = g.run("diff", "--name-only", "-z", id+"...HEAD", "--")
	if err != nil {
		return nil, false, nil // e.g. no merge base: unknown, not fatal
	}
	paths := splitNUL(out)
	for _, p := range paths {
		if err := key.ValidatePath(p); err != nil {
			return nil, false, err
		}
	}
	sortStrings(paths)
	return paths, true, nil
}

// openRegular opens dir/path if and only if it is a regular file reached
// without crossing a symbolic link out of dir.  path must be a valid Kiln
// path, so it cannot contain ".." or start with "/".
func openRegular(dir, path string) (io.ReadCloser, error) {
	if err := key.ValidatePath(path); err != nil {
		return nil, err
	}
	full := filepath.Join(dir, filepath.FromSlash(path))
	fi, err := os.Lstat(full)
	if err != nil {
		return nil, err
	}
	if !fi.Mode().IsRegular() {
		return nil, fmt.Errorf("%s: not a regular file (symlinks are not followed)", path)
	}
	// A directory above the file may itself be a symlink that leaves dir.
	realRoot, err := filepath.EvalSymlinks(dir)
	if err != nil {
		return nil, err
	}
	realParent, err := filepath.EvalSymlinks(filepath.Dir(full))
	if err != nil {
		return nil, err
	}
	rel, err := filepath.Rel(realRoot, realParent)
	if err != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return nil, fmt.Errorf("%s: resolves outside the source root", path)
	}
	return os.Open(full)
}
