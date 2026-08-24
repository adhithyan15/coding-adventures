package validator

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func newGitRepo(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	cmd := exec.Command("git", "init", "--quiet", root)
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("git init: %v\n%s", err, output)
	}
	return root
}

func trackFile(t *testing.T, root, rel string) {
	t.Helper()
	path := filepath.Join(root, filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("mkdir for %s: %v", rel, err)
	}
	if err := os.WriteFile(path, []byte("fixture\n"), 0o644); err != nil {
		t.Fatalf("write %s: %v", rel, err)
	}
	cmd := exec.Command("git", "-C", root, "add", "--", rel)
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("git add %s: %v\n%s", rel, err, output)
	}
}

func TestTrackedArtifacts_CleanRepositoryPasses(t *testing.T) {
	root := newGitRepo(t)
	trackFile(t, root, "code/packages/typescript/example/package.json")

	if err := ValidateNoTrackedNodeModules(root); err != nil {
		t.Fatalf("expected pass, got: %v", err)
	}
}

func TestTrackedArtifacts_TrackedNodeModulesFails(t *testing.T) {
	root := newGitRepo(t)
	trackFile(t, root, "code/packages/typescript/example/node_modules/dependency/index.js")

	err := ValidateNoTrackedNodeModules(root)
	if err == nil {
		t.Fatal("expected tracked node_modules content to fail validation, got nil")
	}
	if !strings.Contains(err.Error(), "code/packages/typescript/example/node_modules/dependency/index.js") {
		t.Fatalf("error did not name the tracked artifact: %v", err)
	}
}

func TestTrackedArtifacts_OnlyExactPathComponentFails(t *testing.T) {
	root := newGitRepo(t)
	trackFile(t, root, "docs/node_modules_notes.md")
	trackFile(t, root, "code/packages/typescript/node_modules-helper/index.ts")

	if err := ValidateNoTrackedNodeModules(root); err != nil {
		t.Fatalf("similar names must not be rejected: %v", err)
	}
}

func TestTrackedArtifacts_NonRepositoryFailsClosed(t *testing.T) {
	err := ValidateNoTrackedNodeModules(t.TempDir())
	if err == nil {
		t.Fatal("expected git inspection failure, got nil")
	}
	if !strings.Contains(err.Error(), "listing tracked files") {
		t.Fatalf("expected actionable git failure, got: %v", err)
	}
}

func TestTrackedArtifacts_ParsesNULTerminatedStageOutput(t *testing.T) {
	raw := []byte("100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa 0\tgood/node_modules_notes.md\x00" +
		"120000 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb 0\tcode/pkg/node_modules\x00" +
		"100644 cccccccccccccccccccccccccccccccccccccccc 0\tother/node_modules/file with spaces.js\x00")

	got, err := trackedNodeModulesPaths(raw)
	if err != nil {
		t.Fatalf("parse stage output: %v", err)
	}
	want := []string{"code/pkg/node_modules", "other/node_modules/file with spaces.js"}
	if strings.Join(got, "|") != strings.Join(want, "|") {
		t.Fatalf("got %q, want %q", got, want)
	}
}

func TestTrackedArtifacts_MalformedStageOutputFailsClosed(t *testing.T) {
	if _, err := trackedNodeModulesPaths([]byte("missing-tab\x00")); err == nil {
		t.Fatal("expected malformed git output to fail closed, got nil")
	}
}
