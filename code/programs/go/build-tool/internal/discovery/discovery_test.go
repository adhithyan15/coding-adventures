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

func TestInferLanguageRust(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/rust/logic-gates")
	if lang != "rust" {
		t.Fatalf("expected rust, got %s", lang)
	}
}

func TestInferLanguageTypescript(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/typescript/logic-gates")
	if lang != "typescript" {
		t.Fatalf("expected typescript, got %s", lang)
	}
}

func TestInferLanguageDart(t *testing.T) {
	lang := inferLanguage("/repo/code/programs/dart/hello-world")
	if lang != "dart" {
		t.Fatalf("expected dart, got %s", lang)
	}
}

func TestInferLanguageHaskell(t *testing.T) {
	lang := inferLanguage("/repo/code/programs/haskell/build-tool")
	if lang != "haskell" {
		t.Fatalf("expected haskell, got %s", lang)
	}
}

func TestInferLanguageWasm(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/wasm/graph")
	if lang != "wasm" {
		t.Fatalf("expected wasm, got %s", lang)
	}
}

func TestInferLanguageCSharp(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/csharp/graph")
	if lang != "csharp" {
		t.Fatalf("expected csharp, got %s", lang)
	}
}

func TestInferLanguageFSharp(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/fsharp/graph")
	if lang != "fsharp" {
		t.Fatalf("expected fsharp, got %s", lang)
	}
}

func TestInferLanguageUnknown(t *testing.T) {
	lang := inferLanguage("/repo/code/packages/zig/something")
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

func TestInferPackageNameProgram(t *testing.T) {
	name := inferPackageName("/repo/code/programs/go/grammar-tools", "go")
	if name != "go/programs/grammar-tools" {
		t.Fatalf("expected go/programs/grammar-tools, got %s", name)
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
// Tests for BUILD_windows support
// ---------------------------------------------------------------------------

func TestGetBuildFileWindowsPreferred(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":         "echo generic",
		"BUILD_windows": "echo windows",
	})
	got := GetBuildFileForPlatform(root, "windows")
	if filepath.Base(got) != "BUILD_windows" {
		t.Fatalf("expected BUILD_windows, got %s", got)
	}
}

func TestGetBuildFileWindowsFallback(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD": "echo generic",
	})
	got := GetBuildFileForPlatform(root, "windows")
	if filepath.Base(got) != "BUILD" {
		t.Fatalf("expected BUILD on windows fallback, got %s", got)
	}
}

func TestGetBuildFileWindowsNotOnMac(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":         "echo generic",
		"BUILD_windows": "echo windows",
	})
	got := GetBuildFileForPlatform(root, "darwin")
	// On macOS, BUILD_windows should not be used — fall back to BUILD.
	if filepath.Base(got) != "BUILD" {
		t.Fatalf("expected BUILD on darwin, got %s", got)
	}
}

// ---------------------------------------------------------------------------
// Tests for BUILD_mac_and_linux support
// ---------------------------------------------------------------------------

func TestGetBuildFileMacAndLinuxOnMac(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":               "echo generic",
		"BUILD_mac_and_linux": "echo unix",
	})
	got := GetBuildFileForPlatform(root, "darwin")
	if filepath.Base(got) != "BUILD_mac_and_linux" {
		t.Fatalf("expected BUILD_mac_and_linux on darwin, got %s", got)
	}
}

func TestGetBuildFileMacAndLinuxOnLinux(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":               "echo generic",
		"BUILD_mac_and_linux": "echo unix",
	})
	got := GetBuildFileForPlatform(root, "linux")
	if filepath.Base(got) != "BUILD_mac_and_linux" {
		t.Fatalf("expected BUILD_mac_and_linux on linux, got %s", got)
	}
}

func TestGetBuildFileMacAndLinuxNotOnWindows(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":               "echo generic",
		"BUILD_mac_and_linux": "echo unix",
	})
	got := GetBuildFileForPlatform(root, "windows")
	// On Windows, BUILD_mac_and_linux should not be used — fall back to BUILD.
	if filepath.Base(got) != "BUILD" {
		t.Fatalf("expected BUILD on windows, got %s", got)
	}
}

func TestGetBuildFileMacOverridesMacAndLinux(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"BUILD":               "echo generic",
		"BUILD_mac":           "echo mac",
		"BUILD_mac_and_linux": "echo unix",
	})
	got := GetBuildFileForPlatform(root, "darwin")
	// BUILD_mac is more specific than BUILD_mac_and_linux.
	if filepath.Base(got) != "BUILD_mac" {
		t.Fatalf("expected BUILD_mac (most specific), got %s", got)
	}
}

// ---------------------------------------------------------------------------
// Tests for DiscoverPackages (recursive BUILD file discovery)
// ---------------------------------------------------------------------------

func TestDiscoverSimplePackage(t *testing.T) {
	// A minimal fixture: nested directories with a BUILD file at the leaf.
	root := makeFixture(t, map[string]string{
		"packages/python/pkg-a/BUILD":          "echo build\n",
		"packages/python/pkg-a/pyproject.toml": "[project]\nname = \"coding-adventures-pkg-a\"\n",
		"packages/python/pkg-a/src/main.py":    "print('hello')\n",
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

func TestDiscoverEmptyDirectory(t *testing.T) {
	root := t.TempDir()
	packages := DiscoverPackages(root)
	if len(packages) != 0 {
		t.Fatalf("expected 0 packages, got %d", len(packages))
	}
}

func TestDiscoverNoBUILD(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"subdir/nothing": "just a file",
	})
	packages := DiscoverPackages(root)
	if len(packages) != 0 {
		t.Fatalf("expected 0 packages, got %d", len(packages))
	}
}

func TestDiscoverDiamondStructure(t *testing.T) {
	// Four packages at the same level — the diamond dependency shape.
	root := makeFixture(t, map[string]string{
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
		"packages/python/lib-py/BUILD": "echo py",
		"packages/ruby/lib-rb/BUILD":   "echo rb",
		"packages/go/lib-go/BUILD":     "echo go",
		"packages/rust/lib-rs/BUILD":   "echo rs",
		"packages/dart/lib-dart/BUILD": "echo dart",
		"programs/python/app/BUILD":    "echo app",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 6 {
		t.Fatalf("expected 6 packages, got %d", len(packages))
	}

	langs := make(map[string]int)
	for _, pkg := range packages {
		langs[pkg.Language]++
	}
	if langs["python"] != 2 || langs["ruby"] != 1 || langs["go"] != 1 || langs["rust"] != 1 || langs["dart"] != 1 {
		t.Errorf("unexpected language distribution: %v", langs)
	}
}

func TestDiscoverBUILDStopsRecursion(t *testing.T) {
	// If a directory has a BUILD file, we don't look inside it for sub-packages.
	root := makeFixture(t, map[string]string{
		"pkg-a/BUILD":     "echo top",
		"pkg-a/sub/BUILD": "echo sub",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 1 {
		t.Fatalf("expected 1 package (BUILD stops recursion), got %d", len(packages))
	}
	if packages[0].Name != "unknown/pkg-a" {
		t.Errorf("expected unknown/pkg-a, got %s", packages[0].Name)
	}
}

func TestDiscoverSkipsSkipListDirs(t *testing.T) {
	// Directories in the skip list should be completely ignored, even if
	// they contain BUILD files.
	root := makeFixture(t, map[string]string{
		"packages/python/pkg-a/BUILD":              "echo a",
		"packages/python/pkg-a/.venv/BUILD":        "echo venv",
		"packages/python/pkg-a/node_modules/BUILD": "echo node",
		".git/hooks/BUILD":                         "echo git",
		"packages/python/pkg-b/BUILD":              "echo b",
		"packages/python/pkg-b/__pycache__/BUILD":  "echo pycache",
	})

	packages := DiscoverPackages(root)
	// Should only find pkg-a and pkg-b (BUILD stops recursion at pkg level,
	// so .venv and node_modules inside pkg-a are irrelevant). .git at root
	// is skipped.
	if len(packages) != 2 {
		t.Fatalf("expected 2 packages, got %d: %v", len(packages), packages)
	}
}

func TestDiscoverSkipsTargetDir(t *testing.T) {
	// The "target" directory (Rust build output) should be skipped.
	root := makeFixture(t, map[string]string{
		"packages/rust/lib-rs/BUILD":              "echo rs",
		"packages/rust/lib-rs/target/debug/BUILD": "echo target",
	})

	packages := DiscoverPackages(root)
	// BUILD at lib-rs stops recursion, so target is not reached anyway.
	// But verify the package is found.
	if len(packages) != 1 {
		t.Fatalf("expected 1 package, got %d", len(packages))
	}
	if packages[0].Language != "rust" {
		t.Errorf("expected rust, got %s", packages[0].Language)
	}
}

func TestDiscoverSkipsRootLevelSkipDirs(t *testing.T) {
	// Skip-list directories at the root level should be ignored.
	root := makeFixture(t, map[string]string{
		"packages/python/pkg-a/BUILD":  "echo a",
		".claude/worktrees/test/BUILD": "echo claude",
		"vendor/some-dep/BUILD":        "echo vendor",
	})

	packages := DiscoverPackages(root)
	if len(packages) != 1 {
		t.Fatalf("expected 1 package, got %d: %v", len(packages), packages)
	}
	if packages[0].Name != "python/pkg-a" {
		t.Errorf("expected python/pkg-a, got %s", packages[0].Name)
	}
}
