package validator

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func negctlCheck(root string) error {
	return ValidateNegctlUsersDeclarePython(root, negctlPkg(root))
}

func negctlPkg(root string) []discovery.Package {
	return frontPackages(root, "pkg")
}

const negctlPkgDir = "code/packages/lua/pkg/"

func TestNegctlUserWithDeclarationPasses(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{
		negctlPkgDir + "BUILD": "# needs-toolchain: python\npython3 ../../../../scripts/warrant_negctl.py\n",
	})
	if err := negctlCheck(root); err != nil {
		t.Fatal(err)
	}
}

func TestNegctlUserWithoutDeclarationFails(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{
		negctlPkgDir + "BUILD": "python3 ../../../../scripts/warrant_negctl.py\n",
	})
	err := negctlCheck(root)
	if err == nil {
		t.Fatal("want failure")
	}
	for _, want := range []string{"code/packages/lua/pkg/BUILD", "needs-toolchain: python"} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("error %q lacks %q", err, want)
		}
	}
}

func TestNegctlNonUsersAreIgnored(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{
		negctlPkgDir + "BUILD":       "cargo test\n# other_negctl.py\nsh other_negctl.py\n",
		negctlPkgDir + "README.md":   "warrant_negctl.py\n",
		negctlPkgDir + "src/BUILD_x": "warrant_negctl.py\n",
	})
	if err := negctlCheck(root); err != nil {
		t.Fatal(err)
	}
}

// The rule is the old test's substring match: comments and `.bak` lookalikes
// that contain the script name count as uses.
func TestNegctlSubstringSemanticsMatchOldTest(t *testing.T) {
	for name, text := range map[string]string{
		"comment":   "# runs warrant_negctl.py later\ncargo test\n",
		"lookalike": "cp x warrant_negctl.py.bak\n",
	} {
		root := t.TempDir()
		writeTree(t, root, map[string]string{negctlPkgDir + "BUILD": text})
		if negctlCheck(root) == nil {
			t.Errorf("%s: want failure (substring semantics)", name)
		}
	}
}

func TestNegctlEveryVariantIsChecked(t *testing.T) {
	for _, variant := range []string{"BUILD", "BUILD_linux", "BUILD_mac", "BUILD_mac_and_linux", "BUILD_windows"} {
		bad := t.TempDir()
		writeTree(t, bad, map[string]string{negctlPkgDir + variant: "python3 warrant_negctl.py\n"})
		err := negctlCheck(bad)
		if err == nil || !strings.Contains(err.Error(), variant) {
			t.Errorf("%s without declaration: got %v", variant, err)
		}
		good := t.TempDir()
		writeTree(t, good, map[string]string{negctlPkgDir + variant: "# needs-toolchain: python\npython3 warrant_negctl.py\n"})
		if err := negctlCheck(good); err != nil {
			t.Errorf("%s with declaration: %v", variant, err)
		}
	}
}

// A declaration in one variant does not cover a user in another.
func TestNegctlDeclarationIsPerFile(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{
		negctlPkgDir + "BUILD":         "# needs-toolchain: python\n",
		negctlPkgDir + "BUILD_windows": "python warrant_negctl.py\n",
	})
	err := negctlCheck(root)
	if err == nil || !strings.Contains(err.Error(), "BUILD_windows") {
		t.Fatalf("got %v", err)
	}
}

// CI validates on Linux, where a directory whose only file is BUILD_windows is
// not a host-discovered package. The caller passes the all-platforms list; the
// validator must flag that directory.
func TestNegctlWindowsOnlyPackageIsChecked(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{
		negctlPkgDir + "BUILD_windows": "python warrant_negctl.py\n",
	})
	if got, _ := discovery.DiscoverPackages(filepath.Join(root, "code")); len(got) != 0 && runtime.GOOS != "windows" {
		t.Fatalf("fixture should be invisible to host discovery, got %v", got)
	}
	all, err := discovery.DiscoverPackagesAllPlatforms(filepath.Join(root, "code"))
	if err != nil {
		t.Fatal(err)
	}
	err = ValidateNegctlUsersDeclarePython(root, all)
	if err == nil || !strings.Contains(err.Error(), "code/packages/lua/pkg/BUILD_windows") {
		t.Fatalf("got %v", err)
	}
}

// Read errors fail closed: a BUILD* that is a directory or a dangling symlink
// is a finding naming the file; a missing package directory is skipped.
func TestNegctlReadErrorsAreFindings(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{negctlPkgDir + "BUILD_linux/x": "y\n"})
	err := negctlCheck(root)
	if err == nil || !strings.Contains(err.Error(), "code/packages/lua/pkg/BUILD_linux") || !strings.Contains(err.Error(), "cannot read") {
		t.Fatalf("directory: got %v", err)
	}

	root = t.TempDir()
	writeTree(t, root, map[string]string{negctlPkgDir + "README.md": "x\n"})
	if err := os.Symlink(filepath.Join(root, "nowhere"), filepath.Join(root, filepath.FromSlash(negctlPkgDir), "BUILD_mac")); err != nil {
		t.Skip("symlinks unavailable:", err)
	}
	err = negctlCheck(root)
	if err == nil || !strings.Contains(err.Error(), "code/packages/lua/pkg/BUILD_mac") {
		t.Fatalf("dangling symlink: got %v", err)
	}

	if err := negctlCheck(t.TempDir()); err != nil {
		t.Fatalf("absent package directory must be skipped: %v", err)
	}
}

// A flagged file the build tool can never run gets rename/remove advice, not
// "add the directive".
func TestNegctlNonBuildNameGetsRenameAdvice(t *testing.T) {
	for _, name := range []string{"BUILD.bak", "BUILD-notes.md"} {
		root := t.TempDir()
		writeTree(t, root, map[string]string{negctlPkgDir + name: "python3 warrant_negctl.py\n"})
		err := negctlCheck(root)
		if err == nil || !strings.Contains(err.Error(), name) || !strings.Contains(err.Error(), "rename or remove it (the build tool never runs it)") {
			t.Errorf("%s: got %v", name, err)
		}
	}
}
