// Package pkgmeta decides which files in a package are *package-local
// metadata*: files a change to which must re-run the package's own BUILD but
// can never change what any other package builds or tests.
//
// # Why this exists
//
// Diff-based change detection normally works in two steps:
//
//	changed file ──(path prefix)──▶ changed package ──(graph)──▶ dependents
//
// The second arrow is right for source: if rust/bounded-json's parser
// changes, every crate that links it must be rebuilt and re-tested. It is
// wrong for a file nothing downstream reads. Adding
// rust/bounded-json/required_capabilities.json or a CHANGELOG entry used to
// select 556 packages, because rust/bounded-json sits under most of the Rust
// tree. None of those 555 dependents can observe either file.
//
// So a metadata-only change is selected the way a repository input consumer
// is (see repository_inputs.go in the main package):
//
//	affected = AffectedNodes(packages with a non-metadata change)
//	         ∪ packages whose only changes are metadata      — no dependents
//	         + prerequisites of all of the above              — so they build
//
// The package itself still rebuilds, because its own BUILD may check the file:
// rust/ct-compare and rust/gf256 verify their effect_certificate.json,
// assurance_report.json and evidence/ manifests, and many packages' tests read
// their own required_capabilities.json.
//
// # The closed list, and the evidence for each entry
//
// A file belongs here only if no *dependent* of its package can read it. The
// list was built from repository-wide searches (recorded in
// code/specs/12-build-system.md, "Package-local metadata"):
//
//	path (package-relative)      who reads it
//	──────────────────────────   ─────────────────────────────────────────────
//	README.md                    the package's own docs/tests (Rust
//	                             include_str!, pyproject readme, gemspec files);
//	                             no dependent's BUILD or test opens it
//	CHANGELOG.md, CHANGELOG.d/   repository changelog checkers (code/scripts,
//	                             scheduled by path, not by package)
//	required_capabilities.json   the package's own analyzer/cage checks and
//	                             tests; the generated cage (gen_capabilities.go,
//	                             src/capabilities.rs) is *source* and still
//	                             propagates when it is regenerated
//	effect_certificate.json      the package's own --verify-cert step
//	assurance_report.json        the package's own --check step
//	evidence/                    the package's own warrant evidence steps
//
// Readers in *other* packages exist, but none of them is a dependent of the
// owner, so dependency propagation never selected them anyway:
// rust/required-capabilities-compiler reads
// rust/weather-agent-e2e/required_capabilities.json (declared in the
// repository source-input boundary), rust/ca-assurance-report's tests read
// rust/gf256's evidence, and rust/ca-capability-analyzer's tests read
// rust/ct-compare. Removing propagation therefore loses no package that
// propagation ever reached for a reason.
//
// # Matching rules
//
//   - Only the package ROOT counts. tests/fixtures/README.md is a test input
//     (rust/html-lexer reads rust/html-parser's), so a nested README is
//     ordinary source.
//   - Exact, case-sensitive basenames. readme.md, README.md.bak, CHANGELOG
//     and evidence.bindings (read by rust/warrant-grammar) are ordinary source.
//   - CHANGELOG.d/ and evidence/ match only strict descendants: a FILE named
//     CHANGELOG.d or evidence is ordinary source.
//   - Raw slash-separated Git paths. A backslash never matches, so an
//     unusual spelling falls back to the conservative behaviour.
//
// When in doubt a file is NOT metadata: a wrongly-propagating file costs CI
// time, a wrongly-local one lets a broken dependent merge.
package pkgmeta

import "strings"

// rootFiles are package-root files that are metadata when they change.
var rootFiles = map[string]struct{}{
	"README.md":                  {},
	"CHANGELOG.md":               {},
	"required_capabilities.json": {},
	"effect_certificate.json":    {},
	"assurance_report.json":      {},
}

// rootDirectories are package-root directories whose every descendant is
// metadata. The trailing slash is part of the prefix.
var rootDirectories = []string{
	"CHANGELOG.d/",
	"evidence/",
}

// IsPackageLocal reports whether relative — a slash-separated path relative
// to the package root, exactly as Git spells it — is package-local metadata.
//
//	README.md                          → true
//	CHANGELOG.d/2026-11-11-x.md        → true
//	evidence/test-manifest.json        → true
//	required_capabilities.json         → true
//	src/lib.rs                         → false (source)
//	docs/README.md                     → false (not at the root)
//	readme.md, README.md.bak           → false (not the exact name)
//	CHANGELOG.d, evidence              → false (a file, not the directory)
//	evidence.bindings                  → false (read by other packages)
//	""                                 → false (the package root itself)
func IsPackageLocal(relative string) bool {
	if strings.Contains(relative, "\\") {
		return false
	}
	if _, ok := rootFiles[relative]; ok {
		return true
	}
	for _, prefix := range rootDirectories {
		if rest, ok := strings.CutPrefix(relative, prefix); ok && rest != "" {
			return true
		}
	}
	return false
}
