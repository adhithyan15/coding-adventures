package validator

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// The two shapes the repository uses, verbatim.
const (
	haskellPrefixFront = `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; if command -v cabal >/dev/null 2>&1; then cabal test all; else echo 'cabal not found -- skipping'; fi`
	haskellInlineFront = `if command -v cabal >/dev/null 2>&1; then if [ "$(uname -s)" = Darwin ]; then DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal test all; else cabal test all; fi; else echo 'cabal not found -- skipping'; fi`
)

// haskellPackage lays out one package directory holding the given BUILD-family
// files and returns it as discovery would.
func haskellPackage(t *testing.T, lang string, files map[string]string) discovery.Package {
	t.Helper()
	dir := filepath.Join(t.TempDir(), "code", "packages", lang, "pkg")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	for name, content := range files {
		writeBuildFile(t, dir, name, content)
	}
	return discovery.Package{Name: lang + "/pkg", Path: dir, Language: lang}
}

func TestHaskellDarwinLinkerAcceptsBothShapes(t *testing.T) {
	cases := map[string]string{
		"prefix":            haskellPrefixFront + "\n",
		"inline":            haskellInlineFront + "\n",
		"prefix-bare-cabal": `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test && cabal build --ghc-options=-Werror` + "\n",
		"commented-prefix":  "# The Darwin GHC bindist links CLT SDK stubs: use the matching CLT linker.\n# Scope the selection to Cabal and its children.\n" + haskellPrefixFront + "\n",
		"inline-top-level":  `if [ "$(uname -s)" = Darwin ]; then DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal test all; else cabal test all; fi` + "\n",
		"echo-only":         "echo 'haskell not supported here'\n",
		"unsupported":       "echo BUILD_TOOL_UNSUPPORTED:NO_GHC_HERE -- skipped\n",
	}
	for name, build := range cases {
		t.Run(name, func(t *testing.T) {
			pkg := haskellPackage(t, "haskell", map[string]string{"BUILD": build})
			if problems := validateHaskellDarwinLinker([]discovery.Package{pkg}); len(problems) != 0 {
				t.Fatalf("expected no problems, got %v", problems)
			}
		})
	}
}

func TestHaskellDarwinLinkerRejectsMissingSelection(t *testing.T) {
	cases := map[string]string{
		"standard":       "if command -v cabal >/dev/null 2>&1; then cabal test all; else echo 'cabal not found -- skipping'; fi\n",
		"bare":           "cabal test\n",
		"script":         "sh tools/run-tests.sh\n",
		"wrong-os":       `if [ "$(uname -s)" = Linux ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test` + "\n",
		"unconditional":  "export DEVELOPER_DIR=/Library/Developer/CommandLineTools; cabal test\n",
		"longer-path":    `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools-old; fi; cabal test` + "\n",
		"xcode-path":     `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer; fi; cabal test` + "\n",
		"echo-chained":   "echo building && cabal test\n",
		"second-line":    haskellPrefixFront + "\ncabal build\n",
		"comment-only":   "# if [ \"$(uname -s)\" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi;\ncabal test\n",
		"comment-target": "#" + haskellPrefixFront + "\ncabal test all\n",
	}
	for name, build := range cases {
		t.Run(name, func(t *testing.T) {
			pkg := haskellPackage(t, "haskell", map[string]string{"BUILD": build})
			problems := validateHaskellDarwinLinker([]discovery.Package{pkg})
			if len(problems) != 1 {
				t.Fatalf("expected one problem, got %v", problems)
			}
			for _, want := range []string{"haskell/pkg", "BUILD", "CommandLineTools", HaskellDarwinLinkerLesson} {
				if !strings.Contains(problems[0], want) {
					t.Errorf("problem %q does not mention %q", problems[0], want)
				}
			}
		})
	}
}

// A BUILD whose every line is a comment launches nothing, so it has nothing
// to select; the orphan-package-root gate owns "a BUILD must be runnable".
func TestHaskellDarwinLinkerIgnoresCommentOnlyBuild(t *testing.T) {
	pkg := haskellPackage(t, "haskell", map[string]string{"BUILD": "# nothing to run\n"})
	if problems := validateHaskellDarwinLinker([]discovery.Package{pkg}); len(problems) != 0 {
		t.Fatalf("expected no problems, got %v", problems)
	}
}

// Only the file Darwin resolves counts: BUILD_mac outranks BUILD, and
// BUILD_windows never runs on macOS.
func TestHaskellDarwinLinkerReadsTheDarwinResolvedFile(t *testing.T) {
	t.Run("BUILD_mac fixes an unfixed BUILD", func(t *testing.T) {
		pkg := haskellPackage(t, "haskell", map[string]string{
			"BUILD":     "cabal test\n",
			"BUILD_mac": haskellPrefixFront + "\n",
		})
		if problems := validateHaskellDarwinLinker([]discovery.Package{pkg}); len(problems) != 0 {
			t.Fatalf("expected no problems, got %v", problems)
		}
	})
	t.Run("unfixed BUILD_mac_and_linux is rejected", func(t *testing.T) {
		pkg := haskellPackage(t, "haskell", map[string]string{
			"BUILD":               haskellPrefixFront + "\n",
			"BUILD_mac_and_linux": "cabal test\n",
		})
		problems := validateHaskellDarwinLinker([]discovery.Package{pkg})
		if len(problems) != 1 || !strings.Contains(problems[0], "BUILD_mac_and_linux") {
			t.Fatalf("expected one BUILD_mac_and_linux problem, got %v", problems)
		}
	})
	t.Run("BUILD_windows is not read", func(t *testing.T) {
		pkg := haskellPackage(t, "haskell", map[string]string{
			"BUILD":         haskellPrefixFront + "\n",
			"BUILD_windows": "cabal test\n",
		})
		if problems := validateHaskellDarwinLinker([]discovery.Package{pkg}); len(problems) != 0 {
			t.Fatalf("expected no problems, got %v", problems)
		}
	})
	t.Run("Windows-only package has no Darwin front", func(t *testing.T) {
		pkg := haskellPackage(t, "haskell", map[string]string{"BUILD_windows": "cabal test\n"})
		if problems := validateHaskellDarwinLinker([]discovery.Package{pkg}); len(problems) != 0 {
			t.Fatalf("expected no problems, got %v", problems)
		}
	})
}

func TestHaskellDarwinLinkerIgnoresOtherLanguagesAndStarlark(t *testing.T) {
	for _, lang := range []string{"rust", "python", "ocaml", "unknown"} {
		t.Run(lang, func(t *testing.T) {
			pkg := haskellPackage(t, lang, map[string]string{"BUILD": "cabal test\n"})
			if problems := validateHaskellDarwinLinker([]discovery.Package{pkg}); len(problems) != 0 {
				t.Fatalf("expected no problems, got %v", problems)
			}
		})
	}
	t.Run("starlark", func(t *testing.T) {
		pkg := haskellPackage(t, "haskell", map[string]string{"BUILD": "haskell_library(name = \"pkg\")\n"})
		pkg.IsStarlark = true
		if problems := validateHaskellDarwinLinker([]discovery.Package{pkg}); len(problems) != 0 {
			t.Fatalf("expected no problems, got %v", problems)
		}
	})
}

// The rule is wired into the same entry point every other BUILD rule uses,
// so `--validate-build-files` (and therefore the CI detect job) enforces it.
func TestValidateBuildFilesReportsHaskellDarwinLinker(t *testing.T) {
	pkg := haskellPackage(t, "haskell", map[string]string{"BUILD": "cabal test\n"})
	pkg.BuildCommands = []string{"cabal test"}
	graph := graphWithEdges()
	graph.AddNode(pkg.Name)
	err := ValidateBuildFiles([]discovery.Package{pkg}, graph)
	if err == nil || !strings.Contains(err.Error(), HaskellDarwinLinkerLesson) {
		t.Fatalf("expected the Haskell Darwin linker problem, got %v", err)
	}
}
