package record

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
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
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

func tryGit(dir string, args ...string) (string, error) {
	cmd := exec.Command("git", args...)
	cmd.Dir = dir
	cmd.Env = append(os.Environ(),
		"GIT_AUTHOR_NAME=t", "GIT_AUTHOR_EMAIL=t@example.com",
		"GIT_COMMITTER_NAME=t", "GIT_COMMITTER_EMAIL=t@example.com")
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

// newRepo makes a committed repository holding files (path -> content) and
// returns its root. objectFormat is "" for the default (SHA-1).
func newRepo(t *testing.T, files map[string]string, initArgs ...string) string {
	t.Helper()
	requireGit(t)
	dir := t.TempDir()
	runGit(t, dir, append([]string{"init", "-q"}, initArgs...)...)
	runGit(t, dir, "config", "commit.gpgsign", "false")
	runGit(t, dir, "config", "core.autocrlf", "false")
	for p, c := range files {
		writeFile(t, dir, p, c)
	}
	runGit(t, dir, "add", "-A")
	runGit(t, dir, "commit", "-q", "-m", "init")
	return dir
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
