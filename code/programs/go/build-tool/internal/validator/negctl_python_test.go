package validator

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func negctlPkg(root string) []discovery.Package {
	return frontPackages(root, "pkg")
}

const negctlPkgDir = "code/packages/lua/pkg/"

func TestNegctlUserWithDeclarationPasses(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{
		negctlPkgDir + "BUILD": "# needs-toolchain: python\npython3 ../../../../scripts/warrant_negctl.py\n",
	})
	if err := ValidateNegctlUsersDeclarePython(negctlPkg(root)); err != nil {
		t.Fatal(err)
	}
}

func TestNegctlUserWithoutDeclarationFails(t *testing.T) {
	root := t.TempDir()
	writeTree(t, root, map[string]string{
		negctlPkgDir + "BUILD": "python3 ../../../../scripts/warrant_negctl.py\n",
	})
	err := ValidateNegctlUsersDeclarePython(negctlPkg(root))
	if err == nil {
		t.Fatal("want failure")
	}
	for _, want := range []string{"lua/pkg/BUILD", "needs-toolchain: python"} {
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
	if err := ValidateNegctlUsersDeclarePython(negctlPkg(root)); err != nil {
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
		if ValidateNegctlUsersDeclarePython(negctlPkg(root)) == nil {
			t.Errorf("%s: want failure (substring semantics)", name)
		}
	}
}

func TestNegctlEveryVariantIsChecked(t *testing.T) {
	for _, variant := range []string{"BUILD", "BUILD_linux", "BUILD_mac", "BUILD_mac_and_linux", "BUILD_windows"} {
		bad := t.TempDir()
		writeTree(t, bad, map[string]string{negctlPkgDir + variant: "python3 warrant_negctl.py\n"})
		err := ValidateNegctlUsersDeclarePython(negctlPkg(bad))
		if err == nil || !strings.Contains(err.Error(), variant) {
			t.Errorf("%s without declaration: got %v", variant, err)
		}
		good := t.TempDir()
		writeTree(t, good, map[string]string{negctlPkgDir + variant: "# needs-toolchain: python\npython3 warrant_negctl.py\n"})
		if err := ValidateNegctlUsersDeclarePython(negctlPkg(good)); err != nil {
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
	err := ValidateNegctlUsersDeclarePython(negctlPkg(root))
	if err == nil || !strings.Contains(err.Error(), "BUILD_windows") {
		t.Fatalf("got %v", err)
	}
}
