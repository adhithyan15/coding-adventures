package discovery

import (
	"os"
	"path/filepath"
	"testing"
)

// ---------------------------------------------------------------------------
// Helpers: create temporary directory structures for testing
// ---------------------------------------------------------------------------

// makeFixture creates a temporary directory tree and returns its root path.
// The tree map keys are relative paths (using "/" separators); values are
// file contents. Directories are created implicitly.
func makeFixture(t *testing.T, tree map[string]string) string {
	t.Helper()
	root := t.TempDir()
	for relPath, content := range tree {
		absPath := filepath.Join(root, filepath.FromSlash(relPath))
		if err := os.MkdirAll(filepath.Dir(absPath), 0755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(absPath, []byte(content), 0644); err != nil {
			t.Fatal(err)
		}
	}
	return root
}

// ---------------------------------------------------------------------------
// Tests for readLines
// ---------------------------------------------------------------------------

func TestReadLinesSkipsBlanksAndComments(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"test.txt": "alpha\n\n# comment\nbeta\n  gamma  \n",
	})
	lines := readLines(filepath.Join(root, "test.txt"))
	if len(lines) != 3 {
		t.Fatalf("expected 3 lines, got %d: %v", len(lines), lines)
	}
	if lines[0] != "alpha" || lines[1] != "beta" || lines[2] != "gamma" {
		t.Fatalf("unexpected lines: %v", lines)
	}
}

func TestReadLinesMissingFile(t *testing.T) {
	lines := readLines("/nonexistent/path/file.txt")
	if lines != nil {
		t.Fatalf("expected nil for missing file, got %v", lines)
	}
}

// ---------------------------------------------------------------------------
// Tests for inferLanguage
// ---------------------------------------------------------------------------

func TestInferLanguagePython(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/python/logic-gates")
	if lang != "python" {
		t.Fatalf("expected python, got %s", lang)
	}
}

func TestInferLanguageRuby(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/ruby/logic_gates")
	if lang != "ruby" {
		t.Fatalf("expected ruby, got %s", lang)
	}
}

func TestInferLanguageGo(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/go/directed-graph")
	if lang != "go" {
		t.Fatalf("expected go, got %s", lang)
	}
}

func TestInferLanguageUnknown(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/rust/something")
	if lang != "unknown" {
		t.Fatalf("expected unknown, got %s", lang)
	}
}

// ---------------------------------------------------------------------------
// Tests for inferPackageName
// ---------------------------------------------------------------------------

func TestInferPackageName(t *testing.T) {
	name := inferPackageName("/repo/code/packages/python/logic-gates", "python")
	if name != "python/logic-gates" {
		t.Fatalf("expected python/logic-gates, got %s", name)
	}
}

// ---------------------------------------------------------------------------
// Tests for getBuildFile / GetBuildFileForPlatform
// ---------------------------------------------------------------------------

func TestGetBuildFileGeneric(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD": "echo build",
	})
	got := GetBuildFileForPlatform(root, "darwin")
	if got != filepath.Join(root, "BUILD") {
		t.Fatalf("expected BUILD, got %s", got)
	}
}

func TestGetBuildFileMacPreferred(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":     "echo generic",
		"BUILD_mac": "echo mac",
	})
	got := GetBuildFileForPlatform(root, "darwin")
	if filepath.Base(got) != "BUILD_mac" {
		t.Fatalf("expected BUILD_mac, got %s", got)
	}
}

func TestGetBuildFileLinuxPreferred(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":       "echo generic",
		"BUILD_linux": "echo linux",
	})
	got := GetBuildFileForPlatform(root, "linux")
	if filepath.Base(got) != "BUILD_linux" {
		t.Fatalf("expected BUILD_linux, got %s", got)
	}
}

func TestGetBuildFileNone(t *testing.T) {
	root := t.TempDir()
	got := GetBuildFileForPlatform(root, "darwin")
	if got != "" {
		t.Fatalf("expected empty string, got %s", got)
	}
}

func TestGetBuildFileMacNotOnLinux(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":     "echo generic",
		"BUILD_mac": "echo mac",
	})
	got := GetBuildFileForPlatform(root, "linux")
	// On linux, BUILD_mac should not be preferred — should fall back to BUILD.
	if filepath.Base(got) != "BUILD" {
		t.Fatalf("expected BUILD on linux, got %s", got)
	}
}

// ---------------------------------------------------------------------------
// Tests for DiscoverPackages
// ---------------------------------------------------------------------------

func TestDiscoverSimplePackage(t *testing.T) {
	// A minimal fixture: DIRS → pkg-a → BUILD
	root := makeFixture(t, map[string]string{
		"DIRS":               "packages",
		"packages/DIRS":      "python",
		"packages/python/DIRS": "pkg-a",
		"packages/python/pkg-a/BUILD":           "echo build\n",
		"packages/python/pkg-a/pyproject.toml":  "[project]\nname = \"coding-adventures-pkg-a\"\n",
		"packages/python/pkg-a/src/main.py":     "print('hello')\n",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 1 {
		t.Fatalf("expected 1 package, got %d", len(packages))
	}

	pkg := packages[0]
	if pkg.Name != "python/pkg-a" {
		t.Errorf("expected name python/pkg-a, got %s", pkg.Name)
	}
	if pkg.Language != "python" {
		t.Errorf("expected language python, got %s", pkg.Language)
	}
	if len(pkg.BuildCommands) != 1 || pkg.BuildCommands[0] != "echo build" {
		t.Errorf("unexpected build commands: %v", pkg.BuildCommands)
	}
}

func TestDiscoverMultiplePackages(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"DIRS":                "packages",
		"packages/DIRS":       "python",
		"packages/python/DIRS": "pkg-a\npkg-b",
		"packages/python/pkg-a/BUILD": "echo a",
		"packages/python/pkg-b/BUILD": "echo b",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 2 {
		t.Fatalf("expected 2 packages, got %d", len(packages))
	}
	// Should be sorted by name.
	if packages[0].Name != "python/pkg-a" {
		t.Errorf("expected python/pkg-a first, got %s", packages[0].Name)
	}
	if packages[1].Name != "python/pkg-b" {
		t.Errorf("expected python/pkg-b second, got %s", packages[1].Name)
	}
}

func TestDiscoverNoDIRS(t *testing.T) {
	root := t.TempDir()
	packages := DiscoverPackages(root)
	if len(packages) != 0 {
		t.Fatalf("expected 0 packages, got %d", len(packages))
	}
}

func TestDiscoverNoBUILD(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"DIRS":           "subdir",
		"subdir/nothing": "just a file",
	})
	packages := DiscoverPackages(root)
	if len(packages) != 0 {
		t.Fatalf("expected 0 packages, got %d", len(packages))
	}
}

func TestDiscoverDiamondStructure(t *testing.T) {
	// Mimic the Python test fixture: diamond with 4 packages.
	root := makeFixture(t, map[string]string{
		"DIRS":                          "pkgs",
		"pkgs/DIRS":                     "python",
		"pkgs/python/DIRS":              "pkg-a\npkg-b\npkg-c\npkg-d",
		"pkgs/python/pkg-a/BUILD":       "echo a",
		"pkgs/python/pkg-a/src/main.py": "pass",
		"pkgs/python/pkg-b/BUILD":       "echo b",
		"pkgs/python/pkg-b/src/main.py": "pass",
		"pkgs/python/pkg-c/BUILD":       "echo c",
		"pkgs/python/pkg-c/src/main.py": "pass",
		"pkgs/python/pkg-d/BUILD":       "echo d",
		"pkgs/python/pkg-d/src/main.py": "pass",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 4 {
		t.Fatalf("expected 4 packages, got %d", len(packages))
	}

	// All should be python language.
	for _, pkg := range packages {
		if pkg.Language != "python" {
			t.Errorf("expected python for %s, got %s", pkg.Name, pkg.Language)
		}
	}
}

func TestDiscoverMultiLanguage(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"DIRS":           "packages\nprograms",
		"packages/DIRS":  "python\nruby\ngo",
		"packages/python/DIRS": "lib-py",
		"packages/python/lib-py/BUILD": "echo py",
		"packages/ruby/DIRS": "lib-rb",
		"packages/ruby/lib-rb/BUILD": "echo rb",
		"packages/go/DIRS": "lib-go",
		"packages/go/lib-go/BUILD": "echo go",
		"programs/DIRS": "python",
		"programs/python/DIRS": "app",
		"programs/python/app/BUILD": "echo app",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 4 {
		t.Fatalf("expected 4 packages, got %d", len(packages))
	}

	langs := make(map[string]int)
	for _, pkg := range packages {
		langs[pkg.Language]++
	}
	if langs["python"] != 2 || langs["ruby"] != 1 || langs["go"] != 1 {
		t.Errorf("unexpected language distribution: %v", langs)
	}
}

func TestDiscoverWithComments(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"DIRS": "# This is a comment\npkgs\n\n",
		"pkgs/DIRS": "python",
		"pkgs/python/DIRS": "# skip this\npkg-a\n# also skip",
		"pkgs/python/pkg-a/BUILD": "echo a",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 1 {
		t.Fatalf("expected 1 package, got %d", len(packages))
	}
}

func TestDiscoverBUILDStopsRecursion(t *testing.T) {
	// If a directory has a BUILD file, we don't look inside it for DIRS.
	root := makeFixture(t, map[string]string{
		"DIRS":                "pkg-a",
		"pkg-a/BUILD":        "echo top",
		"pkg-a/DIRS":         "sub",
		"pkg-a/sub/BUILD":    "echo sub",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 1 {
		t.Fatalf("expected 1 package (BUILD stops recursion), got %d", len(packages))
	}
	if packages[0].Name != "unknown/pkg-a" {
		t.Errorf("expected unknown/pkg-a, got %s", packages[0].Name)
	}
}
