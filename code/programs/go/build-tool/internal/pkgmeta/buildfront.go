package pkgmeta

// BUILD fronts: the pure half of the package-local BUILD rule.
//
// Everything here is a function of strings. Filesystem reads, Git statuses
// and package facts (language, Starlark) live in the callers; see the
// "BUILD fronts" section of the package documentation.

import (
	"regexp"
	"strings"
)

// BuildFrontNames are the five file names a package's BUILD front can have.
var BuildFrontNames = []string{"BUILD", "BUILD_linux", "BUILD_mac", "BUILD_mac_and_linux", "BUILD_windows"}

// UnsupportedMarker is the text of the platform-unsupported protocol:
// a BUILD of `echo BUILD_TOOL_UNSUPPORTED:<CODE> -- skipped` marks a package
// unsupported on that platform, and its dependents are not run.
const UnsupportedMarker = "BUILD_TOOL_UNSUPPORTED:"

// buildFrontLanguages is the closed language gate for package-local BUILD
// fronts. A language belongs here only if every kind of dependent of its
// packages builds the prerequisite from source in its own BUILD and reads
// nothing a prerequisite's BUILD run leaves behind.
//
// rust: all 1,549 Rust BUILD fronts reference siblings only through
// `cargo ... --manifest-path` (source), repository scripts or the shared
// _windows_cargo.sh. Cross-language dependents of Rust crates run `cargo
// build` themselves (swift/IrcServerNative/BUILD,
// go/paint-codec-png-native/BUILD, go/conduit/tools/run-tests.sh,
// cpp/conduit/tools/run-tests.sh, dart/activation-functions-native/tools/
// run-tests.sh).
//
// NOT admitted, with the counter-evidence: lua (lua/lexer/BUILD is `cd tests
// && busted .` and needs the rocks its prerequisites' BUILDs install),
// typescript (typescript/bitset/BUILD reads ../test-bootstrap/dist/index.js,
// built by that package's BUILD). wasm and go are the obvious next
// candidates.
//
// To add a language: (a) show every kind of dependent of that language's
// packages builds the prerequisite from source in its own BUILD and reads
// nothing a prerequisite's BUILD run produces, (b) list the sampled BUILDs
// here, (c) add a row to the evidence table in code/specs/12-build-system.md.
var buildFrontLanguages = map[string]bool{"rust": true}

// IsBuildFront reports whether relative — slash-separated, package-relative —
// is exactly one of the five root BUILD front names. Case-sensitive; a
// backslash, a nested path or any other suffix never matches.
func IsBuildFront(relative string) bool {
	for _, name := range BuildFrontNames {
		if relative == name {
			return true
		}
	}
	return false
}

// BuildFrontLocalFor reports whether packages of language may have
// package-local BUILD fronts (the closed gate above).
func BuildFrontLocalFor(language string) bool {
	return buildFrontLanguages[language]
}

// PackageFronts is one package's BUILD fronts, for ScanBuildFrontReferences.
type PackageFronts struct {
	Name   string            // package name, e.g. "lua/wasm_execution"
	Dir    string            // directory basename, e.g. "wasm_execution"
	Fronts map[string]string // front name -> content
}

// FrontInvocation is a BUILD line that runs a BUILD front but names no
// other package's directory.
type FrontInvocation struct {
	Package string
	Front   string
	Line    int
	Text    string
}

var (
	// MENTION: the word BUILD (or a platform variant) as a whole token.
	// BUILD_TOOL_UNSUPPORTED does not match: `_` continues the token.
	buildFrontMention = regexp.MustCompile(`(?:^|[^A-Za-z0-9_.-])BUILD(?:_windows|_mac_and_linux|_mac|_linux)?(?:$|[^A-Za-z0-9_.-])`)
	// INVOCATION: an interpreter or ./ path whose operand is a BUILD front.
	buildFrontInvocation = regexp.MustCompile(
		`(?:^|[\s;&|(])(?:sh|bash|dash|zsh|ksh|source|\.|call|CALL)\s+["']?(?:[^\s"';&|()]*[/\\])?BUILD(?:_windows|_mac_and_linux|_mac|_linux)?["']?(?:$|[\s;&|)])` +
			`|(?:^|[\s;&|(])["']?\.{1,2}[/\\](?:[^\s"';&|()]*[/\\])?BUILD(?:_windows|_mac_and_linux|_mac|_linux)?["']?(?:$|[\s;&|)])`)
	buildFrontRem = regexp.MustCompile(`(?i)^@?rem(\s|$)`)
)

// IsFrontInvocation reports whether a (non-comment) line runs a BUILD front.
func IsFrontInvocation(line string) bool {
	return buildFrontInvocation.MatchString(line)
}

// IsCommentLine reports whether a BUILD/script line is blank or a shell,
// batch (`::`, `REM`) comment.
func IsCommentLine(line string) bool {
	trimmed := strings.TrimSpace(line)
	return trimmed == "" || strings.HasPrefix(trimmed, "#") || strings.HasPrefix(trimmed, "::") || buildFrontRem.MatchString(trimmed)
}

// ScanBuildFrontReferences finds the packages whose BUILD fronts are run by
// another package's BUILD.
//
// For every front of every package D, every non-comment line that mentions a
// BUILD front is tokenised on every byte outside [A-Za-z0-9_.-]; each other
// package whose directory basename is one of the tokens is marked executed.
// The scan is deliberately over-approximate (basename matching ignores the
// language, so every `*/conduit` is marked when one conduit's BUILD is run):
// an over-approximation only costs CI time. A line that looks like an
// invocation (INVOCATION) but resolves to no package is returned in
// unresolved; the validator refuses those, so selection never has to guess.
func ScanBuildFrontReferences(packages []PackageFronts) (executed map[string]bool, unresolved []FrontInvocation) {
	executed = map[string]bool{}
	byDir := map[string][]string{}
	for _, pkg := range packages {
		byDir[pkg.Dir] = append(byDir[pkg.Dir], pkg.Name)
	}
	for _, pkg := range packages {
		for _, front := range BuildFrontNames {
			content, ok := pkg.Fronts[front]
			if !ok {
				continue
			}
			for index, line := range strings.Split(content, "\n") {
				if IsCommentLine(line) || !buildFrontMention.MatchString(line) {
					continue
				}
				resolved := false
				tokens := strings.FieldsFunc(line, func(r rune) bool {
					return !(r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z' || r >= '0' && r <= '9' || r == '_' || r == '.' || r == '-')
				})
				for _, token := range tokens {
					for _, name := range byDir[token] {
						if name != pkg.Name {
							executed[name] = true
							resolved = true
						}
					}
				}
				if !resolved && buildFrontInvocation.MatchString(line) {
					unresolved = append(unresolved, FrontInvocation{Package: pkg.Name, Front: front, Line: index + 1, Text: strings.TrimSpace(line)})
				}
			}
		}
	}
	return executed, unresolved
}
