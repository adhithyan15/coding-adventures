package source

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/envscrub"
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
	// bin and path pin the program and the PATH it runs with. Empty means the
	// default: look "git" up on the process PATH.
	bin, path string
	tm        treeModes // lazily loaded index modes, see Describe
}

// NewGit returns a Source for the work tree at dir.
func NewGit(dir string) *Git { return &Git{dir: dir} }

// NewGitAt is NewGit with the git program fixed to bin (an absolute path) and
// git's PATH fixed to path. The recorder uses it: a lookup on the process PATH
// can find a `git` a BUILD planted in a directory the checkout is bound into,
// and run it, unsandboxed, with the recorder's privileges (KILN04 4.8, H1).
func NewGitAt(dir, bin, path string) *Git { return &Git{dir: dir, bin: bin, path: path} }

// run executes git in the work tree and returns stdout.
func (g *Git) run(args ...string) ([]byte, error) {
	full := append(gitHardening(), "-C", g.dir)
	full = append(full, args...)
	prog := "git"
	env := gitEnv(os.Environ())
	if g.bin != "" {
		prog = g.bin
		env = withPath(env, g.path)
	}
	cmd := exec.Command(prog, full...)
	cmd.Env = env
	var out, errb bytes.Buffer
	cmd.Stdout, cmd.Stderr = &out, &errb
	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("git %s: %w: %s", args[0], err, strings.TrimSpace(errb.String()))
	}
	return out.Bytes(), nil
}

// Output runs one git subcommand in the work tree, with the same hardening
// and scrubbed environment as every other call here, and returns its stdout.
// It exists for internal/record, which needs plumbing (`ls-tree`, `ls-files
// --stage`, `config`) that Source has no reason to expose. There is no shell:
// each element of args is one argv element. A non-zero exit is an error that
// wraps the *exec.ExitError.
func (g *Git) Output(args ...string) ([]byte, error) { return g.run(args...) }

// withPath replaces PATH in env (or adds it).
func withPath(env []string, path string) []string {
	out := make([]string, 0, len(env)+1)
	for _, e := range env {
		if envscrub.EntryName(e) != "PATH" {
			out = append(out, e)
		}
	}
	return append(out, "PATH="+path)
}

// gitHardening are options placed before every subcommand.  None is needed
// for correctness; each closes a way a repository's own configuration could
// run code or reach out:
//
//	--no-pager                 never spawn a pager
//	core.fsmonitor=false       a repo config could name a program to run
//	core.hooksPath=<null>      no hooks, whatever the repo says
//	protocol.file.allow=never  no local-file transport from submodule tricks
//
// The null device is os.DevNull ("/dev/null" or "NUL").  `--end-of-options` is
// deliberately not used: older gits lack it, and the two places a caller's
// text reaches git are already safe (a leading "-" is rejected, and the text
// is first resolved to a hex object id, then `--` precedes pathspecs).
func gitHardening() []string {
	return []string{"--no-pager",
		// `git replace` refs make an object id name different content. Every
		// id Kiln reads from git must mean what its hash says.
		"--no-replace-objects",
		"-c", "core.fsmonitor=false",
		"-c", "core.hooksPath=" + os.DevNull,
		"-c", "protocol.file.allow=never"}
}

// gitEnv builds git's environment: the caller's, minus credentials
// (ScrubSecrets), minus EVERY GIT_* variable, plus a few fixed ones.
//
// Why drop all GIT_*: GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, GIT_EXEC_PATH,
// GIT_SSH_COMMAND, GIT_CONFIG_* and friends can each redirect git to other
// files or make it run another program.  The only selector Kiln wants is the
// explicit `-C <dir>`, so no GIT_* variable is whitelisted.
func gitEnv(environ []string) []string {
	out := []string{}
	for _, e := range envscrub.Scrub(environ) {
		name := strings.ToUpper(envscrub.EntryName(e))
		if strings.HasPrefix(name, "GIT_") || name == "LC_ALL" {
			continue
		}
		out = append(out, e)
	}
	// GIT_NO_REPLACE_OBJECTS is the environment spelling of --no-replace-objects
	// (belt and braces); GIT_NO_LAZY_FETCH stops a partial clone from reaching
	// the network to fill in a missing object.
	return append(out, "GIT_CONFIG_NOSYSTEM=1", "GIT_OPTIONAL_LOCKS=0", "GIT_TERMINAL_PROMPT=0", "LC_ALL=C",
		"GIT_NO_REPLACE_OBJECTS=1", "GIT_NO_LAZY_FETCH=1")
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
		if err := checkSourcePath(p); err != nil {
			return nil, false, err
		}
	}
	sortStrings(paths)
	return paths, true, nil
}

// openRegular opens dir/path if and only if it is a regular file inside dir.
// It uses os.Root, so the kernel-level lookup refuses ".." and any symlink
// that leaves dir atomically (no check-then-open window), then insists the
// leaf is not itself a symlink and that the opened HANDLE is a regular file
// (so a FIFO or device swapped in later is refused).
func openRegular(dir, path string) (io.ReadCloser, error) {
	root, err := os.OpenRoot(dir)
	if err != nil {
		return nil, err
	}
	defer root.Close()
	return openRegularIn(root, path)
}

func openRegularIn(root *os.Root, path string) (*os.File, error) {
	if err := checkSourcePath(path); err != nil {
		return nil, err
	}
	name := filepath.FromSlash(path)
	fi, err := root.Lstat(name)
	if err != nil {
		return nil, err
	}
	if !fi.Mode().IsRegular() {
		return nil, fmt.Errorf("%s: not a regular file (symlinks are not followed)", short(path))
	}
	f, err := root.Open(name)
	if err != nil {
		return nil, err
	}
	hi, err := f.Stat()
	if err != nil || !hi.Mode().IsRegular() {
		f.Close()
		return nil, fmt.Errorf("%s: not a regular file", short(path))
	}
	return f, nil
}
