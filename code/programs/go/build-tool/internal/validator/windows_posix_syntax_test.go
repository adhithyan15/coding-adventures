package validator

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// Each POSIX-only construct, alone and in the shapes the repository actually
// had on main before the rule existed.
func TestWindowsPOSIXConstructRejected(t *testing.T) {
	cases := []struct {
		name, line, construct string
	}{
		// The quiet one: always skips, never fails.
		{"dev-null-redirect", `where swift >/dev/null 2>/dev/null && swift test || echo Swift not available - skipping`, "/dev/null"},
		{"dev-null-in-subshell", `( cd ..\x && cargo build ) && (where swift >/dev/null 2>/dev/null && ( swift test ) || echo skip)`, "/dev/null"},
		{"dev-null-stdin", `swift test </dev/null`, "/dev/null"},
		{"command-v", `command -v perl`, "command -v"},
		{"command-v-in-if", `if command -v mise >nul 2>nul; then x; fi`, "command -v"},
		{"command-substitution", `set X=$(mise which perl)`, "$(...)"},
		{"arithmetic", `echo $((1 + 2))`, "$(...)"},
		{"parameter-expansion", `prove -I${PWD}/lib t`, "${...}"},
		{"bracket-test", `if [ -f x ] && echo y`, "[ ... ]"},
		{"double-bracket-test", `if [[ -f x ]] && echo y`, "[ ... ]"},
		{"bracket-after-bang", `if ![ -f x ] && echo y`, "[ ... ]"},
		{"then", `if exist x; then echo y`, "; then"},
		{"fi", `echo a; fi`, "; then"},
		{"do", `for f in a b; do echo $f`, "; then"},
		{"done", `echo a; done`, "; then"},
		{"esac", `case x in a) echo a;; esac`, "; then"},
		{"keyword-before-paren", `echo a; done)`, "; then"},
		// Single quotes are not quotes to cmd or to an MSVCRT program.
		{"single-quoted-not-quoting", `perl -e 'print ${x}'`, "${...}"},
		// cmd splits and redirects inside single quotes: only a double-quoted
		// hand-off payload belongs to the other shell.
		{"single-quoted-hand-off", `bash -c 'command -v x >/dev/null && echo ${HOME}'`, "/dev/null"},
		// Only the payload of a hand-off belongs to the other shell.
		{"hand-off-cmd-redirect", `bash -c "echo hi" >/dev/null`, "/dev/null"},
		{"after-hand-off", `bash -c "true" && command -v x`, "command -v"},
		{"case-insensitive-shell-not-exempt-for-tail", `sh -c "true" 2>/dev/null`, "/dev/null"},
		// The Perl native packages' line, verbatim in shape.
		{"perl-conduit", `if command -v mise >/dev/null 2>&1; then PERL_BIN=$(mise which perl 2>/dev/null || true); fi && cargo build --release`, "/dev/null"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			construct, alternative := WindowsPOSIXConstructInLine(tc.line)
			if !strings.Contains(construct, tc.construct) {
				t.Fatalf("WindowsPOSIXConstructInLine(%q) = %q, want a construct mentioning %q", tc.line, construct, tc.construct)
			}
			if alternative == "" {
				t.Fatalf("construct %q has no cmd alternative", construct)
			}
		})
	}
}

// The cmd idioms the repository uses, and POSIX text that cmd never runs.
func TestWindowsPOSIXConstructAccepted(t *testing.T) {
	lines := map[string]string{
		"where-nul":          `where swift >nul 2>nul && swift test || echo Swift not available on this runner - skipping`,
		"nul-2-and-1":        `cabal --version >nul 2>&1 && cabal test`,
		"if-exist":           `if exist "%ProgramFiles%\cabal\bin\cabal.exe" ("%ProgramFiles%\cabal\bin\cabal.exe" test all) else (cabal test all)`,
		"percent-var":        `set PYTHONPATH=src&& .venv\Scripts\python.exe -m pytest tests -v`,
		"set-quoted":         `set "PERL5LIB=%CD%\lib" && prove -l -v t\`,
		"cmd-for-do":         `for %i in (a b) do echo %i`,
		"cmd-for-f":          `for /f "delims=" %i in ('where perl') do echo %i`,
		"echo-done":          `echo done`,
		"echo-then":          `echo then fi do done esac`,
		"bracket-in-word":    `echo [ok] [x]`,
		"bash-payload":       `bash -c "while ! mkdir /tmp/lock 2>/dev/null; do sleep 1; done; (cd ../x && npm ci); EC=$?; exit $EC"`,
		"bash-exe-payload":   `bash.exe -c "command -v x >/dev/null && [ -f y ]"`,
		"sh-payload":         `sh -c "if [ -f x ]; then echo $(cat x); fi"`,
		"powershell-payload": `powershell -NoProfile -Command "$x = $(Get-Date); ${env:X}"`,
		"python-c-dollar":    `.venv\Scripts\python -c "import os, sys; print(os.devnull); x = '${y}'; z = '$(w)'"`,
		"perl-e-dollar":      `perl -e "my $x = ${y}; print qq{/dev/null}; if (1) { 1 }; done()"`,
		"node-e":             `node -e "if ([1].length) { console.log('then; fi') }"`,
		"comment":            `# where swift >/dev/null && $(x) [ -f y ]; then`,
		"indented-comment":   `   # command -v x`,
		"rem":                `REM # build-tool: deps=swift/conduit $(x) /dev/null`,
		"at-rem":             `@rem command -v x`,
		"blank":              "   ",
		"unsupported":        `echo BUILD_TOOL_UNSUPPORTED:NO_SWIFT_HERE -- skipped`,
		"dollar-alone":       `echo costs $5`,
		"command-word":       `npm run command -- -verbose`,
	}
	for name, line := range lines {
		t.Run(name, func(t *testing.T) {
			if construct, _ := WindowsPOSIXConstructInLine(line); construct != "" {
				t.Fatalf("WindowsPOSIXConstructInLine(%q) = %q, want accepted", line, construct)
			}
		})
	}
}

// The validator reads BUILD_windows only, reports file, line, construct and
// alternative, and reports every offending line.
func TestValidateWindowsPOSIXSyntax(t *testing.T) {
	broken := haskellPackage(t, "swift", map[string]string{
		"BUILD_windows": "# Windows BUILD\n" +
			"where swift >nul 2>nul && swift build || echo skip\n" +
			"\n" +
			"where swift >/dev/null 2>/dev/null && swift test || echo Swift not available - skipping\n" +
			"if [ -f x ]; then echo y; fi\n",
		// The shared BUILD runs under sh -c and is not this rule's concern.
		"BUILD": "command -v swift >/dev/null && swift test\n",
	})
	unixOnly := haskellPackage(t, "perl", map[string]string{
		"BUILD":               "if command -v mise >/dev/null 2>&1; then echo $(mise which perl); fi\n",
		"BUILD_mac_and_linux": "[ -f x ] && echo ${y}\n",
	})
	fine := haskellPackage(t, "typescript", map[string]string{
		"BUILD_windows": "bash -c \"command -v x >/dev/null; exit $?\"\nnpm ci --quiet\n",
	})

	starlarkFile := haskellPackage(t, "go", map[string]string{
		"BUILD_windows": "load(\"code/packages/starlark/library-rules/go_library.star\", \"go_library\")\n\n_targets = [\n    go_library(name = \"x\", deps = [ ]),\n]\n",
	})

	problems := validateWindowsPOSIXSyntax([]discovery.Package{broken, unixOnly, fine, starlarkFile})
	if len(problems) != 2 {
		t.Fatalf("expected 2 problems, got %d:\n%s", len(problems), strings.Join(problems, "\n"))
	}
	for i, want := range [][]string{
		{"swift/pkg", "BUILD_windows line 4", "[" + WindowsPOSIXSyntaxRule + "]", "`/dev/null`", "`nul` (`>nul 2>nul`)", "cmd /C", "bash -c", WindowsPOSIXSyntaxLesson},
		{"swift/pkg", "BUILD_windows line 5", "`[ ... ]` test", "if exist PATH"},
	} {
		for _, fragment := range want {
			if !strings.Contains(problems[i], fragment) {
				t.Errorf("problem %d %q does not mention %q", i, problems[i], fragment)
			}
		}
	}
}

// A byte-order mark must not hide the first line, and a CRLF file is read by
// line all the same.
func TestValidateWindowsPOSIXSyntaxBOMAndCRLF(t *testing.T) {
	pkg := haskellPackage(t, "swift", map[string]string{
		"BUILD_windows": "\xEF\xBB\xBF[ -f x ] && echo y\r\nwhere swift >nul 2>nul\r\n",
	})
	problems := validateWindowsPOSIXSyntax([]discovery.Package{pkg})
	if len(problems) != 1 || !strings.Contains(problems[0], "line 1") {
		t.Fatalf("expected one problem on line 1, got %v", problems)
	}
}

// The rule is wired into the BUILD validation the CLI runs.
func TestValidateBuildFilesRunsWindowsPOSIXSyntax(t *testing.T) {
	pkg := haskellPackage(t, "swift", map[string]string{
		"BUILD":         "swift test\n",
		"BUILD_windows": "where swift >/dev/null 2>/dev/null && swift test || echo skip\n",
	})
	graph := directedgraph.New()
	graph.AddNode(pkg.Name)
	err := ValidateBuildFiles([]discovery.Package{pkg}, graph)
	if err == nil || !strings.Contains(err.Error(), "["+WindowsPOSIXSyntaxRule+"]") {
		t.Fatalf("expected a %s problem, got %v", WindowsPOSIXSyntaxRule, err)
	}
}

// The diagnostic points at a lesson; that lesson must exist. The test runs from
// internal/validator, six levels below the repository root.
func TestWindowsPOSIXSyntaxLessonExists(t *testing.T) {
	path := filepath.Join("..", "..", "..", "..", "..", "..", filepath.FromSlash(WindowsPOSIXSyntaxLesson))
	if _, err := os.Stat(path); err != nil {
		t.Fatalf("lesson %s named by the diagnostic is missing: %v", WindowsPOSIXSyntaxLesson, err)
	}
}
