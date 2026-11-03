package validator

// POSIX shell syntax in BUILD_windows (rule `windows-posix-syntax`)
// =================================================================
//
// The executor runs every non-comment line of a `BUILD_windows` as its own
//
//	cmd /C <line>
//
// (executor.shellCommandForOS). `cmd.exe` is not a POSIX shell, so a line
// copied from a `BUILD` file breaks there, in one of two ways:
//
//	line                                                      under cmd /C
//	if command -v mise >/dev/null 2>&1; then ...; fi          "-v was unexpected at this time"   (loud)
//	where swift >/dev/null 2>/dev/null && swift test ||       cmd cannot open /dev/null, so the  (QUIET)
//	  echo Swift not available - skipping                     redirect fails, `where` never runs,
//	                                                          and the line always takes the echo
//
// The quiet shape is the dangerous one. It exits 0, so CI is green, while the
// package's tests never run on Windows at all. Before this rule, 49 Swift
// `BUILD_windows` files were written exactly that way.
//
// What is rejected
// ----------------
//
// Only constructs that cannot mean anything useful to cmd:
//
//	construct            example                       cmd alternative
//	/dev/null            >/dev/null 2>&1               >nul 2>nul
//	command -v           command -v perl               where perl >nul 2>nul
//	$(...)               X=$(mise which perl)          one program (python -c / perl -e), or for /f
//	${...}               ${PERL_BIN:-}                 %PERL_BIN%
//	[ ... ] / [[ ... ]]  if [ -f x ]; then             if exist x (...) / if defined X (...)
//	; then|fi|do|...     ...; then ...; fi             if ... (...) else (...)
//
// The keywords count only directly after a `;`. POSIX needs a separator before
// `then`, `fi`, `do`, `done` and `esac`, while cmd's own loop is written
// `for %i in (a b) do ...` with no `;`, and `echo done` is just a word.
//
// What is left alone
// ------------------
//
// cmd's quoting is double quotes only, and what is inside them is handed to the
// program as-is. So double-quoted text is the program's business and is not
// read:
//
//	python -c "import os; print(os.devnull)"    ok: Python's `;`
//	perl -e "print ${x}"                        ok: Perl's `${`
//	bash -c "command -v x >/dev/null && y"      ok: bash's whole payload
//
// That is the whole exemption for a shell hand-off too: `bash -c "..."`'s
// payload is double-quoted, so it is never read. Everything outside the
// payload is still cmd's, so `bash -c "y" >/dev/null` is rejected: cmd opens
// the redirect target before bash ever starts.
//
// Single quotes are NOT quotes to cmd. It still splits `bash -c 'a && b'` at
// the `&&` and redirects at a `>` inside them, and a program that parses its
// command line the Microsoft C runtime way does not strip them either
// (`perl -e 'print 1'` famously fails on Windows). So single-quoted text is
// checked like any other; write a hand-off's payload in double quotes.
//
// Blank lines, `#` comments (the BUILD format's) and `rem` lines (a cmd no-op
// that ignores the rest of its line) are skipped, and so is a Starlark
// BUILD_windows (six Go packages have one), whose `[ ... ]` are lists. Shared `BUILD` files are not
// read here: they run under `sh -c` everywhere but Windows, and Windows uses
// them only when a package has no `BUILD_windows`, which
// validateWindowsEnvAssignments covers for the one shape the executor
// half-translates.
//
// This is a guard for the shapes the repository writes, not a cmd parser: it
// splits a line at cmd's `&&`, `||`, `&` and `|` (cmdCommands) and matches
// each command's unquoted text against the table above.

import (
	"bytes"
	"fmt"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/starlark"
)

// WindowsPOSIXSyntaxRule is the rule's name, as the spec and diagnostics use it.
const WindowsPOSIXSyntaxRule = "windows-posix-syntax"

// WindowsPOSIXSyntaxLesson names the lesson that explains the rule.
const WindowsPOSIXSyntaxLesson = "lessons.d/build-windows-lines-run-under-cmd-c-so-a-posix-redirect-like.md"

// posixConstruct is one POSIX-only construct the rule rejects.
type posixConstruct struct {
	// name says what was found, as the diagnostic prints it.
	name string
	// pattern finds it in a command's unquoted text.
	pattern *regexp.Regexp
	// alternative is the cmd way to say the same thing.
	alternative string
}

// wordStart is what may precede a construct that must stand as its own word:
// the start of the command, whitespace, or a character cmd or sh would end a
// word at.
const wordStart = `(?:^|[\s(!;])`

// windowsPOSIXConstructs is checked in order; a line is reported once, for the
// first construct it contains.
var windowsPOSIXConstructs = []posixConstruct{
	{
		name:        "`/dev/null`",
		pattern:     regexp.MustCompile(`/dev/null`),
		alternative: "`nul` (`>nul 2>nul`)",
	},
	{
		name:        "`command -v`",
		pattern:     regexp.MustCompile(wordStart + `command\s+-v(?:\s|$)`),
		alternative: "`where TOOL >nul 2>nul`",
	},
	{
		name:        "`$(...)` command substitution",
		pattern:     regexp.MustCompile(`\$\(`),
		alternative: "one program that does the whole step (`python -c \"...\"`, `perl -e \"...\"`) or cmd's `for /f \"delims=\" %i in ('COMMAND') do ...` on the same line",
	},
	{
		name:        "`${...}` parameter expansion",
		pattern:     regexp.MustCompile(`\$\{`),
		alternative: "`%VAR%`",
	},
	{
		name:        "a `[ ... ]` test",
		pattern:     regexp.MustCompile(wordStart + `\[\[?(?:\s|$)`),
		alternative: "`if exist PATH (...)`, `if defined VAR (...)` or `if \"%VAR%\"==\"x\" (...)`",
	},
	{
		name:        "a POSIX `; then` / `; fi` / `; do` / `; done` / `; esac` keyword",
		pattern:     regexp.MustCompile(`;\s*(?:then|fi|do|done|esac)(?:[\s;)]|$)`),
		alternative: "cmd's `if CONDITION (...) else (...)` and `for %i in (...) do ...`",
	},
}

// remLine matches a cmd `rem` comment line (optionally `@rem`).
var remLine = regexp.MustCompile(`(?i)^@?rem(?:\s|$)`)

// WindowsPOSIXConstructInLine returns the first POSIX-only construct a
// `BUILD_windows` line uses where cmd would have to run it, and the cmd
// alternative; both are "" when the line is fine.
func WindowsPOSIXConstructInLine(line string) (construct, alternative string) {
	trimmed := strings.TrimSpace(line)
	if trimmed == "" || strings.HasPrefix(trimmed, "#") || remLine.MatchString(trimmed) {
		return "", ""
	}
	for _, command := range cmdCommands(trimmed) {
		command = strings.TrimSpace(strings.TrimLeft(strings.TrimSpace(command), "( "))
		if command == "" {
			continue
		}
		visible := unquoted(command)
		for _, c := range windowsPOSIXConstructs {
			if c.pattern.MatchString(visible) {
				return c.name, c.alternative
			}
		}
	}
	return "", ""
}

// validateWindowsPOSIXSyntax rejects every `BUILD_windows` line that uses
// POSIX shell syntax cmd cannot run. It reads each package's file directly,
// on every host, because the broken line only ever fails (or quietly skips)
// on Windows, and only when a pull request happens to rebuild that package
// there.
//
// A file the reader refuses (a symlink, an oversized file) is reported by
// validateWindowsBuildLines; this rule just skips it.
func validateWindowsPOSIXSyntax(packages []discovery.Package) []string {
	var problems []string
	for _, pkg := range packages {
		path := filepath.Join(pkg.Path, "BUILD_windows")
		data, _ := readWindowsBuildFile(path)
		if data == nil {
			continue
		}
		data = bytes.TrimPrefix(data, []byte("\xEF\xBB\xBF"))
		// A Starlark BUILD_windows (`load(...)`, `go_library(...)`) is
		// evaluated, not run line by line, so its `[ ... ]` lists are Starlark.
		if starlark.IsStarlarkBuild(string(data)) {
			continue
		}
		for number, line := range strings.Split(string(data), "\n") {
			construct, alternative := WindowsPOSIXConstructInLine(line)
			if construct == "" {
				continue
			}
			problems = append(problems, fmt.Sprintf(
				"%s (%s line %d): [%s] %s is POSIX shell syntax, but each BUILD_windows line runs "+
					"as its own `cmd /C`: %q. Use %s, or run the POSIX part explicitly as "+
					"`bash -c \"...\"` (see %s).",
				pkg.Name, filepath.ToSlash(path), number+1, WindowsPOSIXSyntaxRule, construct,
				truncateForMessage(strings.TrimSpace(line)), alternative, WindowsPOSIXSyntaxLesson,
			))
		}
	}
	return problems
}
