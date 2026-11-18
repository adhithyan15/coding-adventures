package validator

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func writeTree(t *testing.T, root string, files map[string]string) {
	t.Helper()
	for rel, text := range files {
		full := filepath.Join(root, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(full), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(full, []byte(text), 0o644); err != nil {
			t.Fatal(err)
		}
	}
}

func frontPackages(root string, names ...string) []discovery.Package {
	var packages []discovery.Package
	for _, name := range names {
		packages = append(packages, discovery.Package{Name: "lua/" + name, Path: filepath.Join(root, "code", "packages", "lua", name)})
	}
	return packages
}

func TestScriptFrontInvocations(t *testing.T) {
	for _, tc := range []struct {
		text string
		want int
	}{
		{"cargo test\n", 0},
		{"# sh BUILD\n", 0},
		{"REM call BUILD_windows\n", 0},
		{"echo BUILD is great\n", 0},
		{"sh ../x/BUILD\n", 1},
		{"cd ../x && bash BUILD\n", 1},
		{"call ..\\x\\BUILD_windows\n", 1},
		{"./BUILD\n", 1},
		{"ok\nsource ../x/BUILD_mac\n", 1},
	} {
		if got := scriptFrontInvocations("t.sh", tc.text); len(got) != tc.want {
			t.Errorf("%q: %d findings, want %d: %v", tc.text, len(got), tc.want, got)
		}
	}
}

func TestValidateBuildFrontInvocationsUsesTheIndex(t *testing.T) {
	root := t.TempDir()
	git := func(args ...string) {
		t.Helper()
		cmd := exec.Command("git", args...)
		cmd.Dir = root
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, out)
		}
	}
	git("init", "-q")
	writeTree(t, root, map[string]string{
		"code/packages/lua/a/BUILD":       "(cd ../b && sh BUILD)\n",
		"code/packages/lua/b/BUILD":       "luarocks make\n",
		"code/packages/lua/b/tools/ok.sh": "cargo test\n",
	})
	git("add", ".")
	packages := frontPackages(root, "a", "b")
	if err := ValidateBuildFrontInvocations(root, packages); err != nil {
		t.Fatalf("a BUILD that names its target package was rejected: %v", err)
	}

	// A line that runs a BUILD but names no other package is refused.
	writeTree(t, root, map[string]string{"code/packages/lua/a/BUILD": "for d in ../*/; do (cd $d && sh BUILD); done\n"})
	err := ValidateBuildFrontInvocations(root, packages)
	if err == nil || !strings.Contains(err.Error(), "lua/a/BUILD:1") {
		t.Fatalf("unresolved invocation: err = %v", err)
	}

	// A tracked script that runs a BUILD front is refused; an untracked one is not listed.
	writeTree(t, root, map[string]string{
		"code/packages/lua/a/BUILD":       "luarocks make\n",
		"code/packages/lua/b/tools/go.sh": "cd ../a && sh BUILD\n",
	})
	if err := ValidateBuildFrontInvocations(root, packages); err != nil {
		t.Fatalf("an untracked script is not part of the repository: %v", err)
	}
	git("add", ".")
	err = ValidateBuildFrontInvocations(root, packages)
	if err == nil || !strings.Contains(err.Error(), "code/packages/lua/b/tools/go.sh:1") {
		t.Fatalf("script invocation: err = %v", err)
	}
}

func TestValidateBuildFrontInvocationsReportsGitFailure(t *testing.T) {
	if err := ValidateBuildFrontInvocations(filepath.Join(t.TempDir(), "missing"), nil); err == nil {
		t.Fatal("a repository Git cannot list must fail the validation")
	}
}

// The checked-out repository itself. A checkout lacking packages whose
// BUILD another package runs (conduit-hello runs python/conduit's) would read
// as unresolved, so it is skipped; any checkout that has the probe runs it.
func TestRepositoryBuildFrontInvocationsResolve(t *testing.T) {
	root, err := filepath.Abs(filepath.Join("..", "..", "..", "..", "..", ".."))
	if err != nil {
		t.Fatal(err)
	}
	if _, statErr := os.Stat(filepath.Join(root, "code", "specs", "12-build-system.md")); statErr != nil {
		t.Fatalf("repository root not found at %s", root)
	}
	// Skip only when the package whose BUILD conduit-hello runs is absent.
	// CI's sparse pattern excludes only human-languages, so the probe exists
	// there and the check really runs; core.sparseCheckout alone is not a
	// reason to skip.
	if _, statErr := os.Stat(filepath.Join(root, "code", "packages", "python", "conduit", "BUILD")); statErr != nil {
		t.Skip("python/conduit absent: packages whose BUILD is run by another package are missing")
	}
	packages, err := discovery.DiscoverPackagesAllPlatforms(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	if err := ValidateBuildFrontInvocations(root, packages); err != nil {
		t.Fatal(err)
	}
}
