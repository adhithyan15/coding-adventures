package main

// Package-local metadata
// ======================
//
// A pull request that only adds rust/bounded-json/required_capabilities.json
// and a CHANGELOG entry used to select 556 packages: prefix matching mapped
// both files to rust/bounded-json, and the dependency graph then pulled in
// every crate that links it. None of those dependents can read either file.
// Splitting one capability-manifest PR into ten branches would have cost
// about 2,200 package builds, and the Windows job's 300-minute step limit
// does not survive sets that large.
//
// internal/pkgmeta names the files that are *package-local metadata* — the
// package's own BUILD may check them, but no dependent's build or tests can
// read them — and records the repository-wide evidence for each one. This
// file wires that classification into selection. A package whose ONLY
// changes are metadata is selected the way a repository input consumer is:
//
//	affected = AffectedNodes(propagating roots)        — the normal closure
//	         ∪ dev-dependents of that closure, one hop  — withDevDependents
//	         + prerequisites                            — so it can build
//	         ∪ input consumers + their prerequisites    — repository_inputs.go
//	         ∪ local-only packages + their prerequisites — this file
//
// A local-only package therefore:
//
//   - is built and tested itself (its BUILD may verify the manifest, the
//     effect certificate or the evidence it just changed);
//   - gets its prerequisites, so a fresh CI runner can build it;
//   - does NOT add its dependents, and does NOT fire Cargo dev-dependency
//     test triggers: its library did not change, so nothing that links it
//     has anything new to test.
//
// Only an addition or in-place edit (Git status A or M) can be local; a
// deletion, rename or type change of a metadata file is an ordinary change,
// because packaging that lists the file needs it to exist in a dependent's
// build. Local and ordinary roots are both computed by selectPackageRoots
// (main.go) from one platform-filtered path list, so a package can never fall
// out of both sets.
//
// A package with any other change is an ordinary root and propagates as
// before, metadata included. So does every root seeded for an external
// fixture or by -force, and the hash-cache fallback is unchanged: it is the
// conservative path used when Git gives no answer.
//
// The same rule is the `metadata_policy: "package_local"` variant of the
// neutral diff_selection contract
// (code/specs/fixtures/build-tool-v1/cases/package-metadata-selection-*.json),
// replayed by internal/graphdiff.

import (
	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
)

// withoutRoots returns local minus every package in roots: one ordinary
// change — or any fixture-consumer seed — makes the whole package an
// ordinary root.
func withoutRoots(local, roots map[string]bool) map[string]bool {
	for name := range roots {
		delete(local, name)
	}
	return local
}

// withPackageLocalChanges adds local-only packages and their prerequisites,
// but not their dependents, to an affected set. The selection rule is exactly
// the one repository input consumers use, so it shares their implementation;
// the separate name keeps the two reasons for "no dependents" apart at the
// call sites. A nil affected set ("everything") is returned unchanged.
func withPackageLocalChanges(
	graph *directedgraph.Graph,
	affected map[string]bool,
	local map[string]bool,
) map[string]bool {
	return withRepositoryInputConsumers(graph, affected, local)
}
