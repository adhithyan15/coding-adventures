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
// A file belongs here only if no *dependent* of its package can read it,
// whether as an input to its build, its tests or its toolchain. The list was
// built from repository-wide searches (recorded in
// code/specs/12-build-system.md, "Package-local metadata"):
//
//	path (package-relative)        who reads it
//	─────────────────────────────  ──────────────────────────────────────────
//	CHANGELOG.md                   repository changelog checks (code/scripts,
//	                               scheduled by path); packaging file lists
//	                               (gemspec, cabal extra-doc-files, Perl
//	                               MANIFEST, npm "files") that a dependent's
//	                               install needs only to EXIST
//	CHANGELOG.d/<name>.md          the same checks, and the package's own
//	                               doc-shard check
//	required_capabilities.json     the package's own analyzer/cage checks and
//	                               tests; the generated cage (gen_capabilities.go,
//	                               src/capabilities.rs) is *source* and still
//	                               propagates when it is regenerated
//	effect_certificate.json        the package's own --verify-cert step
//	assurance_report.json          the package's own --check step
//	evidence/**.{json,cnf,drat}    the package's own warrant evidence steps
//
// README.md is deliberately NOT here: a Python dependent's
// `pip install -e ../<dep>` makes hatchling read the dependency's
// `readme = "README.md"`, so deleting one breaks the dependent's install.
//
// Packaging that needs a file only to exist is safe because only additions
// and modifications can be local: the caller (gitdiff.ClassifyChangedFiles)
// treats a deletion, rename or type change of any of these paths as an
// ordinary change that propagates.
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
//   - Only the package ROOT counts. A nested sub/required_capabilities.json,
//     x/CHANGELOG.d/y.md or x/evidence/y.json is ordinary source.
//   - Exact, case-sensitive names. changelog.md, CHANGELOG.md.bak,
//     evidence.bindings (read by rust/warrant-grammar) are ordinary source.
//   - Closed by USE as well as by name: CHANGELOG.d/ holds only direct
//     `*.md` shards and evidence/ only the data extensions present in the
//     repository today (.json, .cnf, .drat). A script, source file or
//     anything else placed there is ordinary source, so it cannot become a
//     non-propagating way to change what something executes.
//   - Raw slash-separated Git paths. A backslash never matches, so an
//     unusual spelling falls back to the conservative behaviour.
//
// When in doubt a file is NOT metadata: a wrongly-propagating file costs CI
// time, a wrongly-local one lets a broken dependent merge.
package pkgmeta

import "strings"

// rootFiles are package-root files that are metadata when they change.
var rootFiles = map[string]struct{}{
	"CHANGELOG.md":               {},
	"required_capabilities.json": {},
	"effect_certificate.json":    {},
	"assurance_report.json":      {},
}

// EvidenceExtensions are the only file extensions accepted under evidence/.
// They are exactly the extensions of every tracked evidence file today
// (`git ls-files 'code/**/evidence/**'`: test/proof/bench manifests in JSON,
// DIMACS CNF problems and DRAT proofs). Extending the list needs the same
// survey and a check that nothing executes the new type.
var EvidenceExtensions = []string{".json", ".cnf", ".drat"}

// IsPackageLocal reports whether relative — a slash-separated path relative
// to the package root, exactly as Git spells it — is package-local metadata.
//
//	CHANGELOG.md                       → true
//	CHANGELOG.d/2026-11-11-x.md        → true
//	evidence/test-manifest.json        → true
//	evidence/proofs/abc.drat           → true
//	required_capabilities.json         → true
//	README.md                          → false (dependents' packaging reads it)
//	src/lib.rs                         → false (source)
//	sub/required_capabilities.json     → false (not at the root)
//	CHANGELOG.d/nested/x.md            → false (shards are direct children)
//	CHANGELOG.d/run.sh, evidence/x.sh  → false (not a data type)
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
	if shard, ok := strings.CutPrefix(relative, "CHANGELOG.d/"); ok {
		return !strings.Contains(shard, "/") && hasStemAndSuffix(shard, ".md")
	}
	if rest, ok := strings.CutPrefix(relative, "evidence/"); ok {
		for _, segment := range strings.Split(rest, "/") {
			if segment == "" || segment == "." || segment == ".." {
				return false
			}
		}
		base := rest[strings.LastIndex(rest, "/")+1:]
		for _, extension := range EvidenceExtensions {
			if hasStemAndSuffix(base, extension) {
				return true
			}
		}
	}
	return false
}

// hasStemAndSuffix reports whether name ends with suffix and has at least
// one character before it: ".md" alone is not a shard.
func hasStemAndSuffix(name, suffix string) bool {
	return len(name) > len(suffix) && strings.HasSuffix(name, suffix)
}
