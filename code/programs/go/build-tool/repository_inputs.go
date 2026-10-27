package main

// Repository-wide input consumers
// ===============================
//
// Change detection normally works in two steps:
//
//	changed file ──(path prefix)──▶ changed package ──(graph)──▶ dependents
//
// That model has a blind spot for a *repository-wide guard*: a test that does
// not read its own package directory but walks the whole tree. The example
// that motivated this file is rust/mosaic-package-resolver. Its tests
//
//   - every_repository_requirement_is_satisfied — parses every
//     mosaic-package.toml under code/ and checks each [dependencies]
//     requirement against the version of the package a build would find;
//   - the layout-binding guard — compiles every .mll under code/ and checks
//     that every slot a component requires is bound;
//
// so a pull request that only bumps mosaic-pkg-toolkit's version, or only
// edits an app's .mll, changes the guard's input without touching a single
// file inside code/packages/rust/mosaic-package-resolver. Prefix matching maps
// that change to the Mosaic package, never to the guard, and CI reported green
// on exactly the drift the guard exists to catch.
//
// # Why not an ordinary graph edge?
//
// The obvious fix — seed the guard crate as a *changed* package, the way
// main.go already seeds go/compiler-ir for its external fixture — is wrong in
// a costly way here. A changed package drags in all of its transitive
// dependents, and mosaic-package-resolver is a library: the artifact builder,
// every Mosaic emitter, mosaic-compile, mosaic-dev and VisiCalc all link it.
// Measured on the real repository that is 72 packages for a one-line .mll
// edit, and none of those dependents' code changed — only the data the guard
// reads did.
//
// So a repository input consumer is selected the way a *test* is selected,
// not the way a *library* is:
//
//	affected = AffectedNodes(changed packages)      — the normal closure
//	         ∪ input consumers whose paths changed  — no dependents
//	         + prerequisites of all of the above    — so it can build
//
// The prerequisites are still added (expandAffectedSetWithPrereqs), because a
// fresh CI runner has to be able to build the consumer.
//
// # Rules every entry follows (shared with the fixture seeds in main.go)
//
//   - Paths are raw Git paths, slash-separated and repository-relative. They
//     are not cleaned: a backslash or nested lookalike must not broaden
//     selection.
//   - Nothing is stat'ed, so a deleted or renamed-away source still selects
//     the consumer — a deletion is a change to what the guard walks.
//   - A single-language invocation (-language X) only selects consumers of
//     language X, exactly like every other seed.
//   - A matching change whose consumer has gone missing from discovery is a
//     hard error, not a silent skip: a guard that quietly stops being
//     scheduled is indistinguishable from a guard that passes.
//
// Adding a consumer is one table row. Keep `matches` narrow and literal; a
// predicate that matches too much costs CI time, one that matches too little
// recreates the blind spot this file exists to close.

import (
	"fmt"
	"path"
	"sort"
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// repositoryInputConsumer names one package whose tests read files outside
// its own directory, and the predicate over changed paths that selects it.
type repositoryInputConsumer struct {
	name     string // qualified package name, e.g. "rust/mosaic-package-resolver"
	language string // its discovery language, for -language filtering
	inputs   string // human description, printed when the consumer is selected
	matches  func(changed string) bool
}

var repositoryInputConsumers = []repositoryInputConsumer{
	{
		name:     "rust/mosaic-package-resolver",
		language: "rust",
		inputs:   "Mosaic manifests and sources under code/",
		matches:  isMosaicRepositorySource,
	},
}

// isMosaicRepositorySource reports whether a changed path is one the Mosaic
// repository guards walk: any mosaic-package.toml, or any .mil (interface),
// .mll (layout) or .msl (style) source, anywhere under code/ — packages,
// programs, and the fixture packages inside Rust emitter crates alike.
//
//	code/packages/mosaic/mosaic-pkg-toolkit/mosaic-package.toml   → true
//	code/programs/mosaic/task-app/src/TaskApp.mll                 → true
//	code/packages/rust/mosaic-emit-qt/fixtures/x/src/Grid.msl     → true
//	code/packages/mosaic/mosaic-pkg-toolkit/README.md             → false
//	code/x/mosaic-package.toml.bak, code/x/a.mill, docs/a.mil     → false
//	code\packages\mosaic\x\A.mil  (not a Git path)                → false
func isMosaicRepositorySource(changed string) bool {
	if !strings.HasPrefix(changed, "code/") || strings.Contains(changed, "\\") {
		return false
	}
	base := path.Base(changed)
	if base == "mosaic-package.toml" {
		return true
	}
	switch ext := path.Ext(base); ext {
	case ".mil", ".mll", ".msl":
		return len(base) > len(ext)
	default:
		return false
	}
}

// repositoryInputConsumersForChange returns the consumers selected by
// changedFiles, honoring a single-language invocation. It returns nil for a
// nil change list (no Git diff: the caller is in fallback or force mode) and
// an error when a selected consumer is missing from packages.
func repositoryInputConsumersForChange(
	changedFiles []string,
	packages []discovery.Package,
	language string,
) (map[string]bool, error) {
	if changedFiles == nil {
		return nil, nil
	}
	selected := make(map[string]bool)
	for _, consumer := range repositoryInputConsumers {
		if language != "all" && language != consumer.language {
			continue
		}
		hit := false
		for _, changed := range changedFiles {
			if consumer.matches(changed) {
				hit = true
				break
			}
		}
		if !hit {
			continue
		}
		found := false
		for _, pkg := range packages {
			if pkg.Name == consumer.name && pkg.Language == consumer.language {
				found = true
				break
			}
		}
		if !found {
			return nil, fmt.Errorf("repository input consumer %q (%s) is missing from discovered packages", consumer.name, consumer.inputs)
		}
		selected[consumer.name] = true
	}
	return selected, nil
}

// withRepositoryInputConsumers adds selected consumers — and their
// prerequisites, but not their dependents — to an affected set. A nil
// affected set means "everything" (force or hash fallback) and is returned
// unchanged; so is any set when nothing was selected.
func withRepositoryInputConsumers(
	graph *directedgraph.Graph,
	affected map[string]bool,
	consumers map[string]bool,
) map[string]bool {
	if affected == nil || len(consumers) == 0 {
		return affected
	}
	union := make(map[string]bool, len(affected)+len(consumers))
	for name := range affected {
		union[name] = true
	}
	for name := range consumers {
		union[name] = true
	}
	return expandAffectedSetWithPrereqs(graph, union)
}

// sortedConsumerNames is for the one-line progress message.
func sortedConsumerNames(consumers map[string]bool) []string {
	names := make([]string, 0, len(consumers))
	for name := range consumers {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}
