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
	// paths declares the same inputs as `matches`, as repo-relative paths
	// (a file, or a directory meaning "everything beneath it"). The
	// -emit-kiln-graph export publishes these so Kiln can key the consumer on
	// their content. A predicate that cannot be written as a finite path list
	// (the Mosaic walk: any .mll/.mil/.msl/mosaic-package.toml under code/)
	// leaves this nil; the export then lists no paths for it. A test pins
	// every declared path to its predicate so the two cannot drift.
	paths []string
	// globs declares inputs that are a pattern rather than a finite list
	// (`**` any depth including none, `*` within one segment), for
	// predicates such as the Mosaic walk. A test pins them to the predicate
	// over a fixture tree. A row must declare paths, globs, or both; the
	// export marks one that declares neither repo_inputs_unknown so a future
	// predicate-only row cannot silently export an empty list.
	globs []string
}

// mosaicRepositoryGlobs is exactly isMosaicRepositorySource as globs.
var mosaicRepositoryGlobs = []string{
	"code/**/?*.mil",
	"code/**/?*.mll",
	"code/**/?*.msl",
	"code/**/mosaic-package.toml",
}

// Declared input path sets, one per predicate in this file. They mirror the
// predicates exactly (see TestRepositoryInputPathsAgreeWithPredicates).
var (
	nativePerlBuildPaths = []string{
		"code/scripts/check-perl-extension-windows.ps1",
		"code/packages/rust/perl-bridge/Cargo.toml",
		"code/packages/rust/perl-bridge/build.rs",
		"code/packages/rust/perl-bridge/src",
	}
	ctCompareInputPaths = []string{
		"code/specs/fixtures/ct-compare-v1",
		"code/scripts/ct_compare_tfv.py",
		"code/scripts/ct_compare_conformance.py",
		"code/scripts/tfv.py",
	}
	ctCompareCorpusPaths = []string{"code/specs/fixtures/ct-compare-v1/cases.json"}
	// kilnToolchainPinPaths are the pin sources Kiln's committed
	// code/kiln/legacy/toolchains.json is checked against by its drift test
	// (realpolicy_test.go). A toolchain bump that edits only these files must
	// still re-run Kiln's tests, or the drift goes unnoticed until the next
	// Kiln change (#19269 bumped Rust without touching toolchains.json).
	kilnToolchainPinPaths = []string{
		"mise.toml",
		"code/specs/data/toolchain-pins.json",
		".github/workflows/ci.yml",
	}
	testBootstrapPaths = []string{
		"code/specs/fixtures/test-bootstrap-v1",
		"code/scripts/tfv.py",
		"code/scripts/bootstrap_harness_conformance.py",
	}
	rngCorpusPaths         = []string{"code/specs/fixtures/rng-v1"}
	hamurabiCorpusPaths    = []string{"code/specs/fixtures/hamurabi-v1"}
	lunarLanderCorpusPaths = []string{"code/specs/fixtures/lunar-lander-v1"}
	warrantGrammarPaths    = []string{
		"code/grammars/warrant",
		"code/specs/fixtures/warrant-syntax-v1",
		"code/specs/fixtures/warrant-vectors-synthetic-v1",
		"code/scripts/warrant_syntax_corpus.py",
		"code/scripts/tests/test_warrant_syntax_corpus.py",
		"code/scripts/tfv.py",
		"code/specs/VER04-warrant-language.md",
		"code/specs/VER04a-warrant-source-admission.md",
		"code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json",
	}
	// warrantConformanceFixture is the shared runner conformance fixture
	// (VER04i 9.1, WD-100). Every lane's runner package runs all of its cases
	// in its own tests, so an edit to any case selects each runner package.
	// The ct-compare pilots do not read it: their BUILD runs the corpus, not
	// the runner's own tests.
	warrantConformanceFixture = "code/specs/fixtures/warrant-corpus-conformance-v1"
	// warrantPythonLaneInputs are the files the Python Warrant lane reads
	// from outside its own package: the spec the corpus was compiled from,
	// the corpus itself and its consumer registry. The ct-compare package's
	// BUILD re-runs the corpus and fails when its committed manifest drifted,
	// and the runner package's own tests run the real corpus once.
	warrantPythonLaneInputs = []string{
		"code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
	}
	// warrantPythonRunnerPackage is the shared runner. ct-compare installs it
	// editable from a sibling directory (a dev dependency, not a graph edge),
	// so an edit to the runner must select the pilot lane explicitly.
	warrantPythonRunnerPackage   = "code/packages/python/warrant-corpus"
	warrantPythonLaneCorpusPaths = append(append([]string{}, warrantPythonLaneInputs...), warrantConformanceFixture)
	warrantPythonLanePilotPaths  = append(append([]string{}, warrantPythonLaneInputs...), warrantPythonRunnerPackage)
	// The TypeScript Warrant lane (WD-102) reads the same spec, corpus and
	// registry. Its ct-compare BUILD re-runs the corpus, fails when the
	// committed manifest drifted and validates it; the runner package's own
	// tests run the real corpus once.
	warrantTypeScriptLaneInputs = warrantPythonLaneInputs
	// warrantTypeScriptRunnerPackage is the shared runner. ct-compare names it
	// as a file: devDependency, which is a graph edge, but the committed
	// manifest embeds the runner's source digest, so the pilot is selected by
	// an explicit prefix too: the selection must not depend on the dependency
	// resolver noticing a dev-only edge.
	warrantTypeScriptRunnerPackage   = "code/packages/typescript/warrant-corpus"
	warrantTypeScriptLaneCorpusPaths = append(append([]string{}, warrantTypeScriptLaneInputs...), warrantConformanceFixture)
	warrantTypeScriptLanePilotPaths  = append(append([]string{}, warrantTypeScriptLaneInputs...), warrantTypeScriptRunnerPackage)
	// The Go Warrant lane (WD-103) reads the same spec, corpus and registry. Its
	// ct-compare BUILD re-runs the corpus, fails when the committed manifest
	// drifted, proves every negative control and validates the manifest; the
	// runner package's own tests run the real corpus once.
	warrantGoLaneInputs = warrantPythonLaneInputs
	// warrantGoRunnerPackage is the shared runner. ct-compare's go.mod requires
	// it (a graph edge), but the committed manifest embeds the runner's source
	// digest, so the pilot is selected by an explicit prefix too: the selection
	// must not depend on the dependency resolver noticing the edge.
	warrantGoRunnerPackage   = "code/packages/go/warrant-corpus"
	warrantGoLaneCorpusPaths = warrantGoLaneInputs
	warrantGoLanePilotPaths  = append(append([]string{}, warrantGoLaneInputs...), warrantGoRunnerPackage)
	// warrantAssurancePaths are the shared Warrant lane artifacts the
	// assurance report validates directly (see isWarrantAssuranceInput).
	// The Kiln graph export reads them, so they must list every path the
	// predicate accepts; TestRepositoryInputPredicatesAreCoveredByDeclaredInputs
	// enforces that.
	warrantAssurancePaths = []string{
		"code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json",
		"code/scripts/tfv.py",
	}
)

var repositoryInputConsumers = []repositoryInputConsumer{
	{
		name: "perl/conduit", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
		paths:   nativePerlBuildPaths,
	},
	{
		name: "perl/irc-server-native", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
		paths:   nativePerlBuildPaths,
	},
	{
		name: "perl/programs/conduit-hello", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
		paths:   nativePerlBuildPaths,
	},
	{
		name: "perl/gf256_native", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
		paths:   nativePerlBuildPaths,
	},
	{
		name: "perl/polynomial_native", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
		paths:   nativePerlBuildPaths,
	},
	{
		name: "perl/paint_codec_png_native", language: "perl",
		inputs:  "the native Perl build helper and shared Rust XS bridge",
		matches: isNativePerlBuildInput,
		paths:   nativePerlBuildPaths,
	},
	{
		name:     "rust/ct-compare",
		language: "rust",
		inputs:   "the CT01 corpus, L0 transport and independent projection",
		matches:  isCtCompareInput,
		paths:    ctCompareInputPaths,
	},
	{
		name:     "dart/ct-compare",
		language: "dart",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
		paths:    ctCompareCorpusPaths,
	},
	{
		name:     "haskell/ct-compare",
		language: "haskell",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
		paths:    ctCompareCorpusPaths,
	},
	{
		name:     "lua/ct-compare",
		language: "lua",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
		paths:    ctCompareCorpusPaths,
	},
	{
		name:     "ocaml/ct-compare",
		language: "ocaml",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
		paths:    ctCompareCorpusPaths,
	},
	{
		name:     "perl/ct-compare",
		language: "perl",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
		paths:    ctCompareCorpusPaths,
	},
	{
		name:     "swift/ct-compare",
		language: "swift",
		inputs:   "the CT01 corpus",
		matches:  isCtCompareCorpus,
		paths:    ctCompareCorpusPaths,
	},
	{
		name:     "rust/mosaic-package-resolver",
		language: "rust",
		inputs:   "Mosaic manifests and sources under code/",
		matches:  isMosaicRepositorySource,
		globs:    mosaicRepositoryGlobs,
	},
	{
		name:     "python/rng",
		language: "python",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "rust/rng",
		language: "rust",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "rust/hamurabi",
		language: "rust",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "go/hamurabi",
		language: "go",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "python/hamurabi",
		language: "python",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "typescript/hamurabi",
		language: "typescript",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "rust/lunar-lander",
		language: "rust",
		inputs:   "the shared lunar lander conformance corpus",
		matches:  isLunarLanderCorpus,
		paths:    lunarLanderCorpusPaths,
	},
	{
		name:     "csharp/hamurabi",
		language: "csharp",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "fsharp/hamurabi",
		language: "fsharp",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "java/hamurabi",
		language: "java",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "kotlin/hamurabi",
		language: "kotlin",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "dart/hamurabi",
		language: "dart",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "swift/hamurabi",
		language: "swift",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "haskell/hamurabi",
		language: "haskell",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "c/hamurabi",
		language: "c",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "cpp/hamurabi",
		language: "cpp",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "ocaml/hamurabi",
		language: "ocaml",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "ruby/hamurabi",
		language: "ruby",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "elixir/hamurabi",
		language: "elixir",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "lua/hamurabi",
		language: "lua",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "perl/hamurabi",
		language: "perl",
		inputs:   "the shared hamurabi conformance corpus",
		matches:  isHamurabiCorpus,
		paths:    hamurabiCorpusPaths,
	},
	{
		name:     "go/lunar-lander",
		language: "go",
		inputs:   "the shared lunar lander conformance corpus",
		matches:  isLunarLanderCorpus,
		paths:    lunarLanderCorpusPaths,
	},
	{
		name:     "python/lunar-lander",
		language: "python",
		inputs:   "the shared lunar lander conformance corpus",
		matches:  isLunarLanderCorpus,
		paths:    lunarLanderCorpusPaths,
	},
	{
		name:     "typescript/lunar-lander",
		language: "typescript",
		inputs:   "the shared lunar lander conformance corpus",
		matches:  isLunarLanderCorpus,
		paths:    lunarLanderCorpusPaths,
	},
	{
		name:     "ruby/lunar-lander",
		language: "ruby",
		inputs:   "the shared lunar lander conformance corpus",
		matches:  isLunarLanderCorpus,
		paths:    lunarLanderCorpusPaths,
	},
	{
		name:     "elixir/lunar_lander",
		language: "elixir",
		inputs:   "the shared lunar lander conformance corpus",
		matches:  isLunarLanderCorpus,
		paths:    lunarLanderCorpusPaths,
	},
	{
		name:     "lua/lunar-lander",
		language: "lua",
		inputs:   "the shared lunar lander conformance corpus",
		matches:  isLunarLanderCorpus,
		paths:    lunarLanderCorpusPaths,
	},
	{
		name:     "perl/lunar-lander",
		language: "perl",
		inputs:   "the shared lunar lander conformance corpus",
		matches:  isLunarLanderCorpus,
		paths:    lunarLanderCorpusPaths,
	},
	{
		name:     "go/rng",
		language: "go",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "c/rng",
		language: "c",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "cpp/rng",
		language: "cpp",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "swift/rng",
		language: "swift",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "typescript/rng",
		language: "typescript",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "java/rng",
		language: "java",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "kotlin/rng",
		language: "kotlin",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "csharp/rng",
		language: "csharp",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "fsharp/rng",
		language: "fsharp",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "dart/rng",
		language: "dart",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "ruby/rng",
		language: "ruby",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "elixir/rng",
		language: "elixir",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "lua/rng",
		language: "lua",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "perl/rng",
		language: "perl",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "haskell/rng",
		language: "haskell",
		inputs:   "the shared rng conformance corpus",
		matches:  isRngCorpus,
		paths:    rngCorpusPaths,
	},
	{
		name:     "go/programs/kiln",
		language: "go",
		inputs:   "the toolchain pin sources its toolchains.json drift test reads",
		matches:  isKilnToolchainPinInput,
		paths:    kilnToolchainPinPaths,
	},
	{
		name:     "rust/test-bootstrap",
		language: "rust",
		inputs:   "the TF01 self-corpus and its Python reference",
		matches:  isTestBootstrapInput,
		paths:    testBootstrapPaths,
	},
	{
		name:     "rust/warrant-grammar",
		language: "rust",
		inputs:   "canonical Warrant grammars, syntax corpus, vector fixtures and transport gate",
		matches:  isWarrantGrammarInput,
		paths:    warrantGrammarPaths,
		globs:    []string{warrantDocumentGlob},
	},
	{
		name:     "rust/ca-assurance-report",
		language: "rust",
		inputs:   "the Warrant lane spec, corpus, consumer registry, schema and TFV transport",
		matches:  isWarrantAssuranceInput,
		paths:    warrantAssurancePaths,
	},
	{
		name:     "python/warrant-corpus",
		language: "python",
		inputs:   "the Warrant spec, corpus and consumer registry its tests run",
		matches:  isWarrantPythonRunnerInput,
		paths:    warrantPythonLaneCorpusPaths,
	},
	{
		name:     "python/ct-compare",
		language: "python",
		inputs:   "the Warrant spec, corpus, consumer registry and the shared Python runner",
		matches:  isWarrantPythonLanePilotInput,
		paths:    warrantPythonLanePilotPaths,
	},
	{
		name:     "typescript/warrant-corpus",
		language: "typescript",
		inputs:   "the Warrant spec, corpus and consumer registry its tests run",
		matches:  isWarrantTypeScriptRunnerInput,
		paths:    warrantTypeScriptLaneCorpusPaths,
	},
	{
		name:     "typescript/ct-compare",
		language: "typescript",
		inputs:   "the Warrant spec, corpus, consumer registry and the shared TypeScript runner",
		matches:  isWarrantTypeScriptLanePilotInput,
		paths:    warrantTypeScriptLanePilotPaths,
	},
	{
		name:     "go/warrant-corpus",
		language: "go",
		inputs:   "the Warrant spec, corpus and consumer registry its tests run",
		matches:  isWarrantGoLaneCorpusInput,
		paths:    warrantGoLaneCorpusPaths,
	},
	{
		name:     "go/ct-compare",
		language: "go",
		inputs:   "the Warrant spec, corpus, consumer registry and the shared Go runner",
		matches:  isWarrantGoLanePilotInput,
		paths:    warrantGoLanePilotPaths,
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

// isWarrantPythonLaneCorpusInput selects the Python runner package when the
// spec, corpus or registry its tests run changes. Exact paths only: a lookalike
// version of the fixture directory must not select it.
func isWarrantPythonLaneCorpusInput(changed string) bool {
	switch changed {
	case "code/specs/ct-compare.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/cases.tfv",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		"code/specs/fixtures/ct-compare-warrant-v1/consumers.json":
		return true
	}
	return false
}

// isWarrantConformanceInput reports whether a changed path is a file of the
// shared runner conformance fixture. A slash-delimited prefix, so a lookalike
// directory such as warrant-corpus-conformance-v10 is not selected.
func isWarrantConformanceInput(changed string) bool {
	return strings.HasPrefix(changed, warrantConformanceFixture+"/")
}

// isWarrantPythonRunnerInput selects python/warrant-corpus for the corpus
// inputs and the conformance fixture its tests run.
func isWarrantPythonRunnerInput(changed string) bool {
	return isWarrantPythonLaneCorpusInput(changed) || isWarrantConformanceInput(changed)
}

// isWarrantTypeScriptRunnerInput is the TypeScript counterpart.
func isWarrantTypeScriptRunnerInput(changed string) bool {
	return isWarrantTypeScriptLaneCorpusInput(changed) || isWarrantConformanceInput(changed)
}

// isWarrantPythonLanePilotInput selects python/ct-compare for the same files
// plus anything beneath the shared runner package, which the pilot installs
// as a sibling editable dev dependency.
func isWarrantPythonLanePilotInput(changed string) bool {
	return isWarrantPythonLaneCorpusInput(changed) ||
		strings.HasPrefix(changed, "code/packages/python/warrant-corpus/")
}

// isWarrantTypeScriptLaneCorpusInput selects the TypeScript runner package
// when the spec, corpus or registry its tests run changes. Exact paths only: a
// lookalike version of the fixture directory must not select it.
func isWarrantTypeScriptLaneCorpusInput(changed string) bool {
	return isWarrantPythonLaneCorpusInput(changed)
}

// isWarrantTypeScriptLanePilotInput selects typescript/ct-compare for the same
// files plus anything beneath the shared runner package.
func isWarrantTypeScriptLanePilotInput(changed string) bool {
	return isWarrantTypeScriptLaneCorpusInput(changed) ||
		strings.HasPrefix(changed, "code/packages/typescript/warrant-corpus/")
}

// isWarrantGoLaneCorpusInput selects the Go runner package when the spec, corpus
// or registry its tests run changes. Exact paths only: a lookalike version of the
// fixture directory must not select it.
func isWarrantGoLaneCorpusInput(changed string) bool {
	return isWarrantPythonLaneCorpusInput(changed)
}

// isWarrantGoLanePilotInput selects go/ct-compare for the same files plus
// anything beneath the shared runner package.
func isWarrantGoLanePilotInput(changed string) bool {
	return isWarrantGoLaneCorpusInput(changed) ||
		strings.HasPrefix(changed, "code/packages/go/warrant-corpus/")
}

// warrantDocumentGlob is exactly isWarrantDocument as a glob: the `?` keeps a
// bare ".warrant" out, and the suffix keeps ".warrant.draft" out.
const warrantDocumentGlob = "**/?*.warrant"

// isWarrantDocument reports whether a changed path is a Warrant document, in
// any directory. rust/warrant-grammar's v0_compat test parses and checks every
// .warrant file in the repository, so adding or editing one anywhere must run
// it; otherwise an invalid document lands on main unchecked and fails a later,
// unrelated PR that does rebuild warrant-grammar. Drafts are not documents.
//
//	code/specs/foo.warrant                     → true
//	code/packages/python/x/x.warrant           → true
//	code/specs/foo.warrant.draft, .warrant.bak → false
//	code/specs/.warrant, code/specs/foo.Warrant → false
//	code\specs\foo.warrant  (not a Git path)   → false
func isWarrantDocument(changed string) bool {
	if strings.Contains(changed, "\\") {
		return false
	}
	base := path.Base(changed)
	return strings.HasSuffix(base, ".warrant") && len(base) > len(".warrant")
}

// Canonical inputs live outside the Rust artifact package. Exact file paths
// and slash-delimited prefixes avoid selecting lookalike versions/directories.
// Validator-test edits select the consumer too: its BUILD runs those tests.
func isWarrantGrammarInput(changed string) bool {
	return isWarrantDocument(changed) ||
		strings.HasPrefix(changed, "code/grammars/warrant/") ||
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

// isKilnToolchainPinInput reports whether a changed path is one of the pin
// sources Kiln's toolchains.json must agree with.
func isKilnToolchainPinInput(changed string) bool {
	switch changed {
	case "mise.toml", "code/specs/data/toolchain-pins.json", ".github/workflows/ci.yml":
		return true
	}
	return false
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

// isHamurabiCorpus reports whether a changed path is in the shared hamurabi corpus,
// which the hamurabi package tests read.
//
//	code/specs/fixtures/hamurabi-v1/cases.json  → true
//	code/specs/fixtures/hamurabi-v10/cases.json → false
func isHamurabiCorpus(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/hamurabi-v1/")
}

// isLunarLanderCorpus reports whether a changed path is in the shared lunar lander corpus.
func isLunarLanderCorpus(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/lunar-lander-v1/")
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
