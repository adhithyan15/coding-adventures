package record

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"testing"
)

// requireGit skips the test when git is missing and isolates the test from
// the developer's own git configuration (autocrlf, templates, hooks), which
// would otherwise change what "a clean checkout" means.
func requireGit(t *testing.T) {
	t.Helper()
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("git is not installed")
	}
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("USERPROFILE", home)
	t.Setenv("XDG_CONFIG_HOME", filepath.Join(home, "xdg"))
	t.Setenv("GIT_CONFIG_NOSYSTEM", "1")
}

// quietGitEnv turns off git's background housekeeping for one git process.
//
// Newer git (the ubuntu-26.04 runner's) runs `git maintenance run --auto`
// after commit, fetch and friends. That creates and deletes
// .git/objects/maintenance.lock (and may auto-gc, rewriting loose objects and
// packs) in a process that can outlive the command that started it. The tests
// build a template repository once and then walk and copy it; a lock file that
// readdir lists and that is gone by the time of the lstat made
// TestStepRefusesAResealedPreWhoseContentIsWrong fail with
// "lstat .../.git/objects/maintenance.lock: no such file or directory" (D29).
// A fixture repository has nothing to maintain, so no git the helpers start
// may do it: the race is removed rather than tolerated. (GIT_CONFIG_COUNT
// config has the highest precedence short of -c, so no user or repo config
// can turn maintenance back on.)
func quietGitEnv() []string {
	return []string{
		"GIT_CONFIG_COUNT=3",
		"GIT_CONFIG_KEY_0=maintenance.auto", "GIT_CONFIG_VALUE_0=false",
		"GIT_CONFIG_KEY_1=gc.auto", "GIT_CONFIG_VALUE_1=0",
		"GIT_CONFIG_KEY_2=gc.autoDetach", "GIT_CONFIG_VALUE_2=false",
	}
}

// runGit runs git in dir and returns its trimmed output; failure is fatal.
func runGit(t *testing.T, dir string, args ...string) string {
	t.Helper()
	out, err := tryGit(dir, args...)
	if err != nil {
		t.Fatalf("git %s: %v\n%s", strings.Join(args, " "), err, out)
	}
	return out
}

// runGitStdin is runGit with text on standard input.
func runGitStdin(t *testing.T, dir, stdin string, args ...string) {
	t.Helper()
	if out, err := tryGitStdin(dir, stdin, args...); err != nil {
		t.Fatalf("git %s: %v\n%s", strings.Join(args, " "), err, out)
	}
}

// tryGitStdin returns git's trimmed output (the new object id for mktree).
func tryGitStdin(dir, stdin string, args ...string) (string, error) {
	cmd := exec.Command("git", args...)
	cmd.Dir = dir
	cmd.Stdin = strings.NewReader(stdin)
	cmd.Env = append(os.Environ(), quietGitEnv()...)
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

func tryGit(dir string, args ...string) (string, error) {
	cmd := exec.Command("git", args...)
	cmd.Dir = dir
	cmd.Env = append(os.Environ(),
		"GIT_AUTHOR_NAME=t", "GIT_AUTHOR_EMAIL=t@example.com",
		"GIT_COMMITTER_NAME=t", "GIT_COMMITTER_EMAIL=t@example.com")
	cmd.Env = append(cmd.Env, quietGitEnv()...)
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

// newRepo makes a committed repository holding files (path -> content) and
// returns its root. objectFormat is "" for the default (SHA-1).
//
// Making a repository costs five git processes, and the suite makes hundreds of
// them from a handful of distinct file sets. So each distinct (files, init
// arguments) pair is built ONCE per test binary, in a template directory, and
// every call gets its own COPY of it in t.TempDir(): same objects, same commit
// id, same index, a private working tree the test may do anything to. A git
// repository holds no absolute path, so a copy is as good as the original.
func newRepo(t *testing.T, files map[string]string, initArgs ...string) string {
	t.Helper()
	requireGit(t)
	return newRepoAt(t, t.TempDir(), files, initArgs...)
}

// newRepoAt is newRepo into the directory dest (made if absent).
func newRepoAt(t *testing.T, dest string, files map[string]string, initArgs ...string) string {
	t.Helper()
	requireGit(t)
	if err := os.MkdirAll(dest, 0o755); err != nil {
		t.Fatal(err)
	}
	copyTree(t, templateRepo(t, files, initArgs), dest)
	return dest
}

var (
	templatesMu   sync.Mutex
	templatesRoot string
	templates     = map[string]string{}
)

// TestMain removes the template repositories when the binary is done. (The
// classification child tests re-run this binary as a helper; it makes none.)
func TestMain(m *testing.M) {
	code := m.Run()
	if templatesRoot != "" {
		// A sealed record directory is read-only (0555): open every directory up
		// first, or RemoveAll cannot delete what is in it.
		filepath.Walk(templatesRoot, func(p string, info os.FileInfo, err error) error {
			if err == nil && info.IsDir() {
				os.Chmod(p, 0o755)
			}
			return nil
		})
		os.RemoveAll(templatesRoot)
	}
	os.Exit(code)
}

// ensureRootLocked makes the shared root (canonical: a prepared world must live
// at a path with no symlink in it, which /var on macOS is) when templatesMu is held.
func ensureRootLocked(t *testing.T) {
	t.Helper()
	if templatesRoot != "" {
		return
	}
	root, err := os.MkdirTemp("", "kiln-record-templates-")
	if err != nil {
		t.Fatal(err)
	}
	if root, err = filepath.EvalSymlinks(root); err != nil {
		t.Fatal(err)
	}
	templatesRoot = root
}

// sharedRoot is the directory the template repositories and the prepared worlds
// of the collect tests live in: outside every t.TempDir(), removed by TestMain.
func sharedRoot(t *testing.T) string {
	t.Helper()
	templatesMu.Lock()
	defer templatesMu.Unlock()
	ensureRootLocked(t)
	return templatesRoot
}

// templateRepo builds, once, the repository for files and initArgs.
func templateRepo(t *testing.T, files map[string]string, initArgs []string) string {
	t.Helper()
	templatesMu.Lock()
	defer templatesMu.Unlock()
	names := make([]string, 0, len(files))
	for p := range files {
		names = append(names, p)
	}
	sort.Strings(names)
	h := sha256.New()
	fmt.Fprintf(h, "%q\n", initArgs)
	for _, p := range names {
		fmt.Fprintf(h, "%q %q\n", p, files[p])
	}
	key := hex.EncodeToString(h.Sum(nil))
	if d, ok := templates[key]; ok {
		return d
	}
	ensureRootLocked(t)
	dir := filepath.Join(templatesRoot, fmt.Sprint(len(templates)))
	if err := os.Mkdir(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	runGit(t, dir, append([]string{"init", "-q"}, initArgs...)...)
	runGit(t, dir, "config", "commit.gpgsign", "false")
	runGit(t, dir, "config", "core.autocrlf", "false")
	// Persist the same switch in the template, so a git a test later runs in a
	// COPY of it (which does not go through runGit) is quiet too.
	runGit(t, dir, "config", "maintenance.auto", "false")
	runGit(t, dir, "config", "gc.auto", "0")
	runGit(t, dir, "config", "gc.autoDetach", "false")
	for _, p := range names {
		writeFile(t, dir, p, files[p])
	}
	runGit(t, dir, "add", "-A")
	runGit(t, dir, "commit", "-q", "-m", "init")
	templates[key] = dir
	return dir
}

// copyTree copies a directory tree: directories, regular files (with their
// modes) and symlinks (as symlinks, never followed).
func copyTree(t *testing.T, src, dst string) {
	t.Helper()
	err := filepath.WalkDir(src, func(p string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(src, p)
		to := filepath.Join(dst, rel)
		info, err := d.Info()
		if err != nil {
			// Belt and braces beside quietGitEnv: a transient file under .git
			// (a *.lock) that vanished between readdir and lstat is not part
			// of the repository. Anything else vanishing is a real error.
			if errors.Is(err, fs.ErrNotExist) && strings.HasSuffix(p, ".lock") &&
				(rel == ".git" || strings.HasPrefix(rel, ".git"+string(filepath.Separator))) {
				return nil
			}
			return err
		}
		switch {
		case d.IsDir():
			return os.MkdirAll(to, info.Mode().Perm()|0o700)
		case info.Mode()&os.ModeSymlink != 0:
			target, err := os.Readlink(p)
			if err != nil {
				return err
			}
			return os.Symlink(target, to)
		default:
			data, err := os.ReadFile(p)
			if err != nil {
				return err
			}
			return os.WriteFile(to, data, info.Mode().Perm())
		}
	})
	if err != nil {
		t.Fatal(err)
	}
}

func writeFile(t *testing.T, dir, rel, content string) {
	t.Helper()
	full := filepath.Join(dir, filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(full, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

func remove(t *testing.T, dir, rel string) {
	t.Helper()
	if err := os.RemoveAll(filepath.Join(dir, filepath.FromSlash(rel))); err != nil {
		t.Fatal(err)
	}
}

// symlink makes dir/rel a symlink to target, skipping the test where the
// platform will not allow one (Windows without the privilege).
func symlink(t *testing.T, dir, rel, target string) {
	t.Helper()
	full := filepath.Join(dir, filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(target, full); err != nil {
		t.Skipf("cannot create symlinks: %v", err)
	}
}
