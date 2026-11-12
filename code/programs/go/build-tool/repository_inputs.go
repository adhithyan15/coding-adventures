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
		name: "perl/conduit", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
	},
	{
		name: "perl/irc-server-native", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
	},
	{
		name: "perl/programs/conduit-hello", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
	},
	{
		name:     "rust/ct-compare",
		language: "rust",
		inputs:   "the CT01 corpus, L0 transport and independent projection",
		matches:  isCtCompareInput,
	},
	{
		name:     "dart/ct-compare",
		language: "dart",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
	},
	{
		name:     "haskell/ct-compare",
		language: "haskell",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
	},
	{
		name:     "lua/ct-compare",
		language: "lua",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
	},
	{
		name:     "ocaml/ct-compare",
		language: "ocaml",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
	},
	{
		name:     "perl/ct-compare",
		language: "perl",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
	},
	{
		name:     "swift/ct-compare",
		language: "swift",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
	},
	{
		name:     "rust/mosaic-package-resolver",
		language: "rust",
		inputs:   "Mosaic manifests and sources under code/",
		matches:  isMosaicRepositorySource,
	},
	{
		name:     "python/rng",
		language: "python",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "rust/rng",
		language: "rust",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "go/rng",
		language: "go",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "c/rng",
		language: "c",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "cpp/rng",
		language: "cpp",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "swift/rng",
		language: "swift",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "typescript/rng",
		language: "typescript",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "java/rng",
		language: "java",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "kotlin/rng",
		language: "kotlin",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "csharp/rng",
		language: "csharp",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "fsharp/rng",
		language: "fsharp",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "dart/rng",
		language: "dart",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "ruby/rng",
		language: "ruby",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "elixir/rng",
		language: "elixir",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "lua/rng",
		language: "lua",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "perl/rng",
		language: "perl",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "haskell/rng",
		language: "haskell",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
	},
	{
		name:     "rust/test-bootstrap",
		language: "rust",
		inputs:   "the TF01 self-corpus and its Python reference",
		matches:  isTestBootstrapInput,
	},
	{
		name:     "rust/warrant-grammar",
		language: "rust",
		inputs:   "canonical Warrant grammars, syntax corpus, vector fixtures and transport gate",
		matches:  isWarrantGrammarInput,
	},
	{
		name:     "rust/ca-assurance-report",
		language: "rust",
		inputs:   "the Warrant lane spec, corpus, consumer registry, schema and TFV transport",
		matches:  isWarrantAssuranceInput,
	},
}

// The assurance report validates the shared Warrant lane artifacts directly.
func isWarrantAssuranceInput(changed string) bool {
	switch changed {
	case "code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json",
		"code/scripts/tfv.py":
		return true
	}
	return false
}

// Canonical inputs live outside the Rust artifact package. Exact file paths
// and slash-delimited prefixes avoid selecting lookalike versions/directories.
// Validator-test edits select the consumer too: its BUILD runs those tests.
func isWarrantGrammarInput(changed string) bool {
	return strings.HasPrefix(changed, "code/grammars/warrant/") ||
		strings.HasPrefix(changed, "code/specs/fixtures/warrant-syntax-v1/") ||
		// WX-E: the pinned vector tables the package's tests load (the pin in
		// the .warrant document fails closed when these bytes change).
		strings.HasPrefix(changed, "code/specs/fixtures/warrant-vectors-synthetic-v1/") ||
		changed == "code/scripts/warrant_syntax_corpus.py" ||
		changed == "code/scripts/tests/test_warrant_syntax_corpus.py" ||
		changed == "code/scripts/tfv.py" ||
		changed == "code/specs/ct-compare.warrant" ||
		changed == "code/specs/fixtures/ct-compare-warrant-v1/cases.tfv" ||
		changed == "code/specs/fixtures/ct-compare-warrant-v1/corpus.json" ||
		changed == "code/specs/fixtures/ct-compare-warrant-v1/consumers.json" ||
		changed == "code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json" ||
		changed == "code/specs/VER04-warrant-language.md" ||
		changed == "code/specs/VER04a-warrant-source-admission.md"
}

// These Perl packages load Rust cdylibs and share a helper outside their own
// roots. Editing only the helper or bridge must still exercise the real XS
// consumers, rather than produce an empty incremental build plan.
func isNativePerlBuildInput(changed string) bool {
	switch changed {
	case "code/scripts/check-perl-extension-windows.ps1",
		"code/packages/rust/perl-bridge/Cargo.toml",
		"code/packages/rust/perl-bridge/build.rs":
		return true
	}
	return strings.HasPrefix(changed, "code/packages/rust/perl-bridge/src/")
}

// CT01 is embedded in an L0 test, so edits outside the crate must still run it.
// Exact reference paths and the slash-delimited fixture prefix exclude other
// versions, path lookalikes and unrelated scripts. Missing consumers fail closed.
func isCtCompareInput(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/ct-compare-v1/") ||
		changed == "code/scripts/ct_compare_tfv.py" ||
		changed == "code/scripts/ct_compare_conformance.py" ||
		changed == "code/scripts/tfv.py"
}

// isCtCompareCorpus selects the CT01 lanes whose tests read cases.json directly
// (or, for OCaml, compare a vendored copy with it). They do not read the TFV
// projection or the Python scripts, so only the exact corpus file selects them.
//
//	code/specs/fixtures/ct-compare-v1/cases.json   → true
//	code/specs/fixtures/ct-compare-v1/cases.tfv    → false (Rust L0 transport only)
//	code/specs/fixtures/ct-compare-v10/cases.json  → false
func isCtCompareCorpus(changed string) bool {
	return changed == "code/specs/fixtures/ct-compare-v1/cases.json"
}

// isTestBootstrapInput reports whether a changed path is one the bootstrap
// harness's BUILD reads: its self-corpus (embedded with include_str!, so a
// corpus edit changes the test binary) or the stdlib reference that
// re-derives the corpus and generates cases.tfv.
//
//	code/specs/fixtures/test-bootstrap-v1/cases.json     → true
//	code/specs/fixtures/test-bootstrap-v1/cases.tfv      → true
//	code/scripts/tfv.py                                  → true
//	code/scripts/bootstrap_harness_conformance.py        → true
//	code/specs/fixtures/test-bootstrap-v10/cases.json    → false
//	code/specs/fixtures/ct-compare-v1/cases.json         → false
//	code/scripts/tests/test_bootstrap_harness_conformance.py → false
func isTestBootstrapInput(changed string) bool {
	switch changed {
	case "code/scripts/tfv.py", "code/scripts/bootstrap_harness_conformance.py":
		return true
	}
	return strings.HasPrefix(changed, "code/specs/fixtures/test-bootstrap-v1/")
}

// isRngCorpus reports whether a changed path is in the shared rng corpus,
// which every rng lane's tests read (Rust embeds it with include_str!).
//
//	code/specs/fixtures/rng-v1/cases.json     → true
//	code/specs/fixtures/rng-v10/cases.json    → false
//	code/specs/rng.md                         → false (prose; the corpus pins behaviour)
func isRngCorpus(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/rng-v1/")
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
