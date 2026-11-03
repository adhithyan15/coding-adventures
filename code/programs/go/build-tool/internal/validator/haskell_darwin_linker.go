package validator

// Haskell Darwin linker selection
// ===============================
//
// GHC 9.4.8's macOS bindist was configured against a Command Line Tools (CLT)
// SDK, and it keeps that sysroot baked into every link it drives. GitHub's
// macOS images can ship CLT 27 beside an *older* default Xcode. A Cabal
// process that inherits the Xcode developer directory therefore hands CLT 27
// SDK stubs to the old Xcode linker, which cannot parse their `arm64e.x1`
// targets:
//
//	ld: tapi error: malformed file ... unknown architecture arm64e.x1
//
// The repair is to point that one process tree at the matching linker:
//
//	DEVELOPER_DIR=/Library/Developer/CommandLineTools
//
// Why per package root, and why every line
// ----------------------------------------
//
// Cabal builds a package's whole local project (its sibling dependencies
// included) inside the single process the selected package root's BUILD line
// starts. Fixing a dependency's BUILD never reaches the root that selected it,
// so every Haskell root must carry the selection itself. And the executor runs
// each BUILD line as its own `sh -c` (executor.runCommands), so a selection on
// one line does not reach the next: every line that may launch GHC needs it.
//
// The two accepted shapes
// -----------------------
//
//	shape    line                                                         scope
//	prefix   if [ "$(uname -s)" = Darwin ]; then export \                 this line's sh -c
//	           DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; CMD
//	inline   if [ "$(uname -s)" = Darwin ]; then \                         the Darwin cabal
//	           DEVELOPER_DIR=/Library/Developer/CommandLineTools cabal …;
//	           else cabal …; fi
//
// Both are Darwin-only (Linux and Windows-under-a-POSIX-shell keep whatever
// they inherited), neither touches `xcode-select` or anything global, and in
// both the package command is the last thing the line runs, so its exit status
// is the line's exit status.
//
// Exempt lines
// ------------
//
//	line                                                   verdict
//	echo BUILD_TOOL_UNSUPPORTED:CODE -- skipped            exempt (never executed)
//	echo 'something'                                       exempt (no toolchain)
//	echo hi && cabal test                                  checked (operator present)
//	# DEVELOPER_DIR=... (comment)                          ignored; cannot satisfy
//
// Only the file Darwin actually resolves (BUILD_mac, then BUILD_mac_and_linux,
// then BUILD) is read. BUILD_windows never runs on macOS and is not checked.

import (
	"fmt"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// HaskellDarwinLinkerLesson names the lesson that explains this rule, so the
// error points a reader at the history rather than only at the symptom.
const HaskellDarwinLinkerLesson = "lessons.d/ghc-must-select-the-linker-matching-its-hardcoded-macos-sdk.md"

// haskellDarwinCLTSelection matches a `[ "$(uname -s)" = Darwin ]` test whose
// `then` branch begins by selecting the CLT developer directory, either as an
// `export` (prefix shape) or as an inline assignment on the Cabal command
// (inline shape). The value must end at whitespace, `;` or end of line so a
// longer path such as `.../CommandLineTools-old` does not count.
var haskellDarwinCLTSelection = regexp.MustCompile(
	`\[\s*"\$\(uname(?:\s+-s)?\)"\s*=\s*"?Darwin"?\s*\]\s*;\s*then\s+` +
		`(?:export\s+)?DEVELOPER_DIR=/Library/Developer/CommandLineTools(?:[\s;]|$)`,
)

// shellOperators are the characters that let an `echo` line do more than
// print: chaining, piping, substitution, or a subshell.
const shellOperators = ";&|`$()<>"

// haskellLineIsExempt reports whether a BUILD line launches no toolchain at
// all: the unsupported-protocol record, or a plain `echo` with no operators.
func haskellLineIsExempt(line string) bool {
	if unsupportedRecordRe.MatchString(line) {
		return true
	}
	fields := strings.Fields(line)
	return len(fields) > 0 && fields[0] == "echo" && !strings.ContainsAny(line, shellOperators)
}

// unsupportedRecordRe recognises the build tool's platform-unsupported
// protocol line (`echo BUILD_TOOL_UNSUPPORTED:CODE -- skipped`), the same
// closed shape executor.unsupportedBuildCode accepts. The executor never runs
// that line, so it needs no linker.
var unsupportedRecordRe = regexp.MustCompile(`^echo BUILD_TOOL_UNSUPPORTED:[A-Z][A-Z0-9_]{2,63} -- skipped$`)

// HaskellLineSelectsDarwinCLT reports whether one BUILD line selects the CLT
// developer directory for Darwin in an accepted shape.
func HaskellLineSelectsDarwinCLT(line string) bool {
	return haskellDarwinCLTSelection.MatchString(line)
}

// validateHaskellDarwinLinker rejects a Haskell package whose Darwin BUILD
// has a line that could launch GHC without selecting the CLT linker.
func validateHaskellDarwinLinker(packages []discovery.Package) []string {
	var problems []string
	for _, pkg := range packages {
		if pkg.IsStarlark || pkg.Language != "haskell" {
			continue
		}
		darwinBuild := discovery.GetBuildFileForPlatform(pkg.Path, "darwin")
		if darwinBuild == "" {
			continue
		}
		for _, line := range readBuildLines(darwinBuild) {
			if haskellLineIsExempt(line) || HaskellLineSelectsDarwinCLT(line) {
				continue
			}
			problems = append(problems, fmt.Sprintf(
				"%s (%s): %q runs on macOS without selecting the Command Line Tools linker. "+
					"GHC 9.4.8 links against its hardcoded CLT SDK, which an older Xcode linker "+
					"cannot read (`arm64e.x1`). Prefix the line with "+
					"`if [ \"$(uname -s)\" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; ` "+
					"(see %s).",
				pkg.Name, filepath.ToSlash(darwinBuild), truncateForMessage(line), HaskellDarwinLinkerLesson,
			))
			break
		}
	}
	return problems
}
