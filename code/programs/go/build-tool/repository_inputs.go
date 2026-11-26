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
	// treePaths marks the subset of paths whose cache input kind is explicitly
	// recursive tree. A file/directory substitution is rejected by the hasher.
	treePaths []string
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
	// goFuzzReferenceVectorPaths is the Go fuzz engine's generated oracle table
	// (820 vectors from scripts/fuzz_conformance.py). java/test-bootstrap's
	// ReferenceVectorsTest parses this Go source file directly, so an edit to it
	// must re-run the Java tests too (VER06f); csharp/test-bootstrap's
	// FuzzReferenceVectorsTests (VER06h), ruby/test_bootstrap's
	// test_fuzz_vectors.rb (VER06i) and c/test-bootstrap's test_fuzz_vectors.c
	// (VER06j, scans it line by line), lua/test_bootstrap's test_fuzz_vectors.lua
	// (VER06p) and elixir/test_bootstrap's fuzz_corpus_test.exs (VER06n) read it
	// the same way, and so does swift/test-bootstrap's
	// FuzzReferenceVectorsTests (VER06l).
	goFuzzReferenceVectorPaths = []string{"code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go"}
	// vosGrammarPaths are the grammar pairs vos-manifest and vos-link embed with
	// include_str! (VOS-033); vosShapeGrammarPaths the Rust lexical pair
	// vos-shape-check embeds. Neither is a Cargo edge, so without a row an edit
	// to them runs no test (VOS-D058).
	vosGrammarPaths = []string{
		"code/grammars/vos_contract",
		"code/grammars/vos_document",
	}
	// pifGrammarPaths are what rust/pif-grammar embeds with include_str! (PIF-001):
	// the canonical Proof Interchange Format grammar pair and the VER32 spec whose
	// section 4.6 its worked-example fixture is compared with. Neither is a Cargo
	// edge, so without a row an edit to them runs no test.
	pifGrammarPaths = []string{
		"code/grammars/pif",
		"code/specs/VER32-proof-interchange-format.md",
	}
	// vosToolPaths are the sources a VOS implementation's BUILD reads or runs outside its own
	// Cargo edges (VOS-D066): `vos-link check-doc` (a standalone cargo root, run by manifest
	// path) with the crates it and `vos-shape-check` are built from, the evidence generator
	// (`cargo run -p coding_adventures_warrant_grammar --example gen_evidence`) with its
	// crates, the capability analyzer, and the two vos_* grammars vos-manifest embeds.  The
	// list is the `cargo tree -e normal` closure of those tools on the day it was written;
	// it covers no tool a BUILD does not run and does not follow a later new dependency
	// (TestVosToolPathsAreWorkspaceCrates only checks that each directory exists).
	// The implementation's own generated document is added per row (vosImplementationRow).
	vosToolPaths = []string{
		"code/programs/rust/vos-link",
		"code/packages/rust/vos-manifest",
		"code/packages/rust/vos-shape-check",
		"code/packages/rust/warrant-grammar",
		"code/packages/rust/ca-capability-analyzer",
		"code/packages/rust/sha256",
		"code/packages/rust/test-bootstrap",
		"code/packages/rust/zeroize",
		"code/packages/rust/grammar-tools",
		"code/packages/rust/directed-graph",
		"code/packages/rust/graph",
		"code/packages/rust/lexer",
		"code/packages/rust/parser",
		"code/packages/rust/state-machine",
		"code/packages/rust/toml-lexer",
		"code/packages/rust/toml-parser",
		// VOS-039a: vos-link reads the evidence manifest with bounded-json, and every
		// generated BUILD runs its sabotage lines through the shared negative-control checker.
		"code/packages/rust/bounded-json",
		"code/scripts/warrant_negctl.py",
		"code/grammars/vos_contract",
		"code/grammars/vos_document",
	}
	// vosCapacityProbePaths are the files the capacity probe's BUILD reads outside its Cargo
	// edges (VOS-028b): `check` re-runs and validates the committed results file, `table --check`
	// compares the generated block of the VOS30 spec with it, and the two Warrant documents
	// (the fixture's and the real scheduler's) are what its E1 rows digest.
	vosCapacityProbePaths = []string{
		"code/specs/data/vos30-capacity-results.json",
		"code/specs/VOS30-proof-route-and-capacity.md",
		"code/specs/vos-probe-ready.warrant",
		"code/specs/vos-sched-fp-bitmap.warrant",
	}
	// vosPublishGlobs are exactly isVosPublishInput as globs: the two files `vos-link check-publish`
	// reads in every direct `vos-*` child of the two Rust roots (VOS-D104).
	vosPublishGlobs = []string{
		"code/packages/rust/vos-*/Cargo.toml",
		"code/packages/rust/vos-*/vos.toml",
		"code/programs/rust/vos-*/Cargo.toml",
		"code/programs/rust/vos-*/vos.toml",
	}
	vosShapeGrammarPaths = []string{
		"code/grammars/rust/rust1.97.0-e2021.tokens",
		"code/grammars/rust/rust1.97.0-e2024.tokens",
	}
	testBootstrapPaths = []string{
		"code/specs/fixtures/test-bootstrap-v1",
		"code/scripts/tfv.py",
		"code/scripts/bootstrap_harness_conformance.py",
	}
	// pbtV1CorpusPaths is the shared property-engine conformance corpus
	// (VER05 section 6). python/test-bootstrap's tests/test_pbt_corpus.py reads
	// cases.json from the repository root, so an edit selects that package;
	// go/test-bootstrap's pbt/corpus_test.go reads the same file (VER05d), and so
	// does java/test-bootstrap's CorpusConformanceTest (VER05e);
	// kotlin/test-bootstrap (VER05f), csharp/test-bootstrap (VER05g), c/test-bootstrap (VER05i) and
	// ruby/test_bootstrap's test_corpus_conformance.rb (VER05h),
	// swift/test-bootstrap's CorpusConformanceTests (VER05k),
	// ruby/test_bootstrap's test_corpus_conformance.rb (VER05h),
	// lua/test_bootstrap's test_corpus_conformance.lua (VER05o),
	// elixir/test_bootstrap's corpus_conformance_test.exs (VER05m) and
	// fsharp/test-bootstrap's FsCorpusRunner (VER05l) read it too.
	// fuzz-v1 is read by ruby/test_bootstrap's test_fuzz_corpus.rb (VER06i) and
	// lua/test_bootstrap's test_fuzz_corpus.lua (VER06p) and elixir/test_bootstrap's
	// fuzz_corpus_test.exs (VER06n) and
	// swift/test-bootstrap's FuzzCorpusConformanceTests (VER06l).
	pbtV1CorpusPaths = []string{"code/specs/fixtures/pbt-v1"}
	// fuzzV1CorpusPaths is the shared fuzz-engine conformance corpus (VER06
	// section 8). python/test-bootstrap's tests/test_fuzz_corpus.py and
	// go/test-bootstrap's fuzz/corpus_test.go read cases.json from the
	// repository root, so an edit selects those packages; so do
	// java/test-bootstrap, kotlin/test-bootstrap, csharp/test-bootstrap and
	// c/test-bootstrap (test_fuzz_corpus.c, VER06j), ruby/test_bootstrap (VER06i),
	// elixir/test_bootstrap (fuzz_corpus_test.exs, VER06n) and lua/test_bootstrap
	// (test_fuzz_corpus.lua, VER06p).
	fuzzV1CorpusPaths = []string{"code/specs/fixtures/fuzz-v1"}
	// trigReferenceCorpusPaths is the bit-exact TRIG-REF-1 conformance corpus
	// (PHY00a section 6). Every converged trig port's tests decode it, so an
	// edit to any of its files selects each converted port. Python, Rust and
	// TypeScript adopted it in TC-2, C++ in TC-5; each later slice adds one consumer row.
	trigReferenceCorpusPaths = []string{"code/specs/fixtures/trig-reference-v1"}
	// trigReferenceInputPaths adds what regenerates and validates the corpus. The
	// python/trig BUILD runs the generator's --check (byte-for-byte regeneration, the
	// sensitivity gates and the C/Rust/bc cross-checks) and the validator tests, so
	// an edit to any of these files must select it, or the generator could change
	// without anything running it.
	trigReferenceInputPaths = []string{
		"code/specs/fixtures/trig-reference-v1",
		"code/scripts/generate_trig_reference_fixture.py",
		"code/scripts/generate_trig_reference_dart_fixtures.py",
		"code/scripts/trig_ref_exact.py",
		"code/scripts/trig_ref_softfloat.py",
		"code/scripts/trig_reference_xcheck",
		"code/scripts/tests/test_trig_reference_fixture.py",
		"code/scripts/tests/test_trig_reference_dart_fixtures.py",
	}

	rngCorpusPaths                 = []string{"code/specs/fixtures/rng-v1"}
	fenwickTreeContractPaths       = []string{"code/specs/DT06-fenwick-tree.md", "code/specs/fixtures/fenwick-tree-v1"}
	dt17HashFunctionsPaths         = []string{"code/specs/DT17-hash-functions.md", "code/specs/DT17b-hash-functions-dart-profile.md", "code/specs/fixtures/hash-functions-v1"}
	microbenchmarkPairsFixturePath = "code/specs/fixtures/microbenchmark-pairs-v1.json"
	fixedPointInputPaths           = []string{
		".cargo/config.toml",
		"code/packages/rust/.cargo/config.toml",
		"code/packages/rust/Cargo.lock",
		"code/packages/rust/Cargo.toml",
		"code/specs/fixed-point.warrant",
		"code/specs/fixtures/fixed-point-q16-v1/cases.json",
	}
	// The verifier BUILD recipes explicitly register these tree roots; all
	// other repository inputs remain file-only.
	fixedPointSourceTreePaths = []string{
		"code/packages/rust/fixed-point/src",
	}
	fixedPointEvidenceTreePaths = []string{
		"code/packages/rust/fixed-point/evidence",
	}
	rustWorkspaceCargoInputs = []string{
		".cargo/config.toml",
		"code/packages/rust/.cargo/config.toml",
		"code/packages/rust/Cargo.lock",
		"code/packages/rust/Cargo.toml",
	}
	fixedPointBindingsInput      = "code/packages/rust/fixed-point/evidence.bindings"
	fixedPointReporterFileInputs = append(append([]string{}, []string{
		"code/packages/rust/fixed-point/Cargo.toml",
		"code/packages/rust/fixed-point/effect_certificate.json",
		fixedPointBindingsInput,
		"code/packages/rust/fixed-point/assurance-profile.json",
		"code/specs/fixed-point.warrant",
	}...), rustWorkspaceCargoInputs...)
	fixedPointWarrantIRFileInputs = append(append([]string{}, []string{
		"code/packages/rust/fixed-point/Cargo.toml",
		fixedPointBindingsInput,
		"code/specs/fixed-point.warrant",
	}...), rustWorkspaceCargoInputs...)
	fixedPointReporterTreePaths  = append(append([]string{}, fixedPointSourceTreePaths...), fixedPointEvidenceTreePaths...)
	fixedPointWarrantIRTreePaths = append([]string{}, fixedPointSourceTreePaths...)
	fixedPointReporterInputs     = append(append([]string{}, fixedPointReporterFileInputs...), fixedPointReporterTreePaths...)
	fixedPointWarrantIRInputs    = append(append([]string{}, fixedPointWarrantIRFileInputs...), fixedPointWarrantIRTreePaths...)
	dt18HashMapPaths             = []string{"code/specs/DT18-hash-map.md", "code/specs/DT18-hash-map-dart-profile.md", "code/specs/fixtures/hash-map-v1"}
	swiftHashMapFixtureInputs    = []string{"code/specs/fixtures/hash-map-v1/cases.json"}
	gameClockInputPaths          = []string{
		".cargo/config.toml",
		"code/packages/rust/.cargo/config.toml",
		"code/packages/rust/Cargo.lock",
		"code/packages/rust/Cargo.toml",
		"code/specs/fixtures/game-clock-v1/cases.json",
		"code/specs/game-clock.warrant",
	}
	typescriptGameClockCorpusPaths = []string{"code/specs/fixtures/game-clock-v1"}
	gameInputFrameV1Paths          = []string{"code/specs/fixtures/game-input-frame-v1"}
	typescriptGameInputFrameInputs = []string{
		".cargo/config.toml",
		"code/packages/rust/.cargo/config.toml",
		"code/packages/rust/Cargo.lock",
		"code/packages/rust/Cargo.toml",
		"code/packages/typescript/tsconfig.base.json",
		"code/specs/fixtures/game-input-frame-v1/cases.json",
	}
	biquadSosFixturePaths  = []string{"code/specs/fixtures/biquad-sos-v1"}
	hamurabiCorpusPaths    = []string{"code/specs/fixtures/hamurabi-v1"}
	lunarLanderCorpusPaths = []string{"code/specs/fixtures/lunar-lander-v1"}
	warrantGrammarPaths    = []string{
		"code/grammars/warrant",
		"code/specs/fixtures/warrant-syntax-v1",
		"code/specs/fixtures/warrant-profile-algorithm-v1",
		"code/specs/fixtures/warrant-vectors-synthetic-v1",
		"code/scripts/warrant_syntax_corpus.py",
		"code/scripts/tests/test_warrant_syntax_corpus.py",
		// WD-HARN1c: the shared negative-control checker and its tests, which
		// warrant-grammar's BUILD runs (the tests are the Windows-executed proof
		// that the check's bail-out and plan rules work).  A script edit selects
		// only this package, not every adopter that calls the script.
		"code/scripts/warrant_negctl.py",
		"code/scripts/tests/test_warrant_negctl.py",
		"code/scripts/tfv.py",
		"code/specs/VER04-warrant-language.md",
		"code/specs/VER04a-warrant-source-admission.md",
		"code/specs/VER04p-warrant-floats.md",
		"code/scripts/ver04p_domain_witnesses.py",
		"code/scripts/tests/test_ver04p_domain_witnesses.py",
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
	warrantGoLaneCorpusPaths = append(append([]string{}, warrantGoLaneInputs...), warrantConformanceFixture)
	warrantGoLanePilotPaths  = append(append([]string{}, warrantGoLaneInputs...), warrantGoRunnerPackage)
	// warrantOraclePaths are the files the independent Warrant oracle
	// (python/warrant-oracle, VER04r section 10) reads: every adopter
	// specification it parses and the one v1 corpus it gates against. The
	// oracle re-derives everything from the source, so a change to any of
	// these files must re-run it (that is the gate of VER04r 10.4).
	warrantOraclePaths = append([]string{
		"code/specs/ct-compare.warrant",
		"code/specs/gf256.warrant",
		"code/specs/wasm-leb128.warrant",
		"code/specs/logic-gates.warrant",
		"code/specs/logic-gates-basic.warrant", // LGE-S1c: the document the basic-gates fixture is generated from
		"code/specs/lzw.warrant",
		"code/specs/arithmetic.warrant",
		"code/specs/x25519.warrant",
		"code/specs/fixtures/ct-compare-warrant-v1/corpus.json",
		// CV2-S3d: the BUILD also runs verify-v2 on each adopter's committed v2 fixture, with
		// the Rust bindings its profile's faults are cross-checked against (the bindings of
		// every warrantAdopters row; the specs are already listed above).
	}, warrantAdopterBindings()...)
	// warrantAdopters is the table of warrant-corpus-v2 adopters (VER04r 16.10 [D32]): one
	// row per module whose committed fixture directory is checked by two packages that do
	// not contain it. rust/warrant-corpus-gen's BUILD runs the generator's --check against
	// the fixture, and python/warrant-oracle's BUILD runs verify-v2 on it, with the Rust
	// evidence.bindings whose faults= lists must equal the fixture's corpus.profile. An edit
	// to the spec, to any file of the fixture or to the bindings must select both, or a
	// stale fixture merges green (the gap D11h of VER04r 13 names). Adding an adopter is a
	// row here, the two BUILD lines (BUILD and BUILD_windows of each package) and a case in
	// warrant_corpus_v2_inputs_test.go.
	warrantAdopters = []warrantAdopter{
		{
			module:   "ct-compare",
			spec:     "code/specs/ct-compare.warrant",
			fixture:  "code/specs/fixtures/ct-compare-warrant-v2",
			bindings: "code/packages/rust/ct-compare/evidence.bindings",
		},
		{
			module:   "x25519",
			spec:     "code/specs/x25519.warrant",
			fixture:  "code/specs/fixtures/x25519-warrant-v2",
			bindings: "code/packages/rust/x25519/evidence.bindings",
		},
		{
			module:   "lzw",
			spec:     "code/specs/lzw.warrant",
			fixture:  "code/specs/fixtures/lzw-warrant-v2",
			bindings: "code/packages/rust/lzw/evidence.bindings",
		},
		{
			module:   "wasm-leb128",
			spec:     "code/specs/wasm-leb128.warrant",
			fixture:  "code/specs/fixtures/wasm-leb128-warrant-v2",
			bindings: "code/packages/rust/wasm-leb128/evidence.bindings",
		},
		{
			// LG02 section 5 (LGE-S1c): the cross-lane basic-gates corpus has NO bindings: Rust's
			// evidence is generated from logic-gates.warrant, so the generator and the oracle run
			// without --bindings and the profile is compared with evidence.bindings by
			// code/scripts/logic_gates_basic_drift.py instead (check 10).
			module:  "logic-gates-basic",
			spec:    "code/specs/logic-gates-basic.warrant",
			fixture: "code/specs/fixtures/logic-gates-basic-warrant-v2",
		},
	}
	// warrantAcceptBlindFixture is the oracle's small synthetic corpus that the
	// generator's tests regenerate byte for byte (accept_blind, D15); it lives in
	// the oracle's directory, so only the generator needs a row for it.
	warrantAcceptBlindFixture = "code/packages/python/warrant-oracle/tests/fixtures/accept-blind-v2"
	// warrantFixedFormsFixture is the oracle's synthetic ir.fixed document and its committed
	// lowering dump (CV2-S10a). The generator's tests/fixed_parity.rs lowers the first and
	// reproduces the second byte for byte (CV2-S10b), reading both from this directory; it
	// lies in the oracle's package, so only the generator needs a row for it.
	warrantFixedFormsFixture = "code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v1"
	// warrantOmitExampleFixture is the oracle's synthetic omit_example corpus (CV2-S10-1): its
	// omit.warrant and corpus.profile are what the generator's tests/omit_example.rs
	// regenerates corpus/ from, byte for byte. It lies in the oracle's directory, so only the
	// generator needs a row for it.
	warrantOmitExampleFixture = "code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2"
	// warrantTupleFormsFixture is the oracle's synthetic ir.tuple document, its profile and its four
	// committed dumps (lowering, evaluation, faults, refusals; CV2-S10t-1). The generator's
	// tests/tuple_parity.rs reproduces the dumps byte for byte (CV2-S10t-2), reading them from this
	// directory; it lies in the oracle's package, so only the generator needs a row for it.
	warrantTupleFormsFixture = "code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1"
	// warrantFixedEvalFixture is the oracle's synthetic ir.fixed EVALUATION document, its profile and its
	// five committed dumps (lowering, evaluation, faults, values, domains; CV2-S10c-1). The generator's
	// tests/fixed_eval_parity.rs reproduces the dumps byte for byte (CV2-S10c-2), reading them from this
	// directory; it lies in the oracle's package, so only the generator needs a row for it.
	warrantFixedEvalFixture = "code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1"
	// warrantOffByOneConsumedFixture is the oracle's synthetic off-by-one-consumed document, its
	// profile and its committed faults and refusals dumps (CV2-S10o-1). The generator's slice
	// S10o-2 reproduces the dumps byte for byte, reading them from this directory; it lies in the
	// oracle's package, so only the generator needs a row for it.
	warrantOffByOneConsumedFixture = "code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1"
	// warrantScalarFaultsFixture is the oracle's synthetic scalar-faults document, its profile and its
	// committed faults and refusals dumps (CV2-S11f-1: saturate-add, saturate-sub, shift-mask and the
	// scalar missing-error). The generator's tests/scalar_faults_parity.rs reproduces the dumps byte
	// for byte (CV2-S11f-2), reading them from this directory; it lies in the oracle's package, so
	// only the generator needs a row for it.
	warrantScalarFaultsFixture = "code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v1"
	// warrantDataFormsFixture is the oracle's synthetic ir.data document, its lowering dump and its refusals
	// dump (CV2-S11a: records, variants, Option, the four tags and the data patterns). The generator's
	// tests/data_parity.rs reproduces the lowering dump byte for byte, including its conditional `types`
	// object, and refuses every document of the refusals dump with the generator code of its row
	// (CV2-S11b), reading them from this directory; it lies in the oracle's package, so only the
	// generator needs a row for it.
	warrantDataFormsFixture = "code/packages/python/warrant-oracle/tests/fixtures/data-forms-v1"
	// warrantDataEvalFixture is the oracle's synthetic ir.data evaluation document, its profile and its six
	// committed dumps: lowering, evaluation (a suite on each stream header), faults under the gate before and
	// after the slice, values, domains and refusals (CV2-S11c-1). It is the generator's byte-parity target
	// for slice S11c-2, which reads the dumps from this directory; it lies in the oracle's package, so only
	// the generator needs a row for it.
	warrantDataEvalFixture = "code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1"
	// warrantCorpusGenPaths are the inputs of the generator's --check line and of its
	// cross-package parity tests.
	warrantCorpusGenPaths = append(warrantAdopterInputs(), []string{
		warrantAcceptBlindFixture,
		warrantFixedFormsFixture,
		warrantOmitExampleFixture,
		warrantTupleFormsFixture,
		warrantFixedEvalFixture,
		warrantOffByOneConsumedFixture,
		warrantScalarFaultsFixture,
		warrantDataFormsFixture,
		warrantDataEvalFixture,
	}...)
	// warrantOracleRowPaths adds each adopter's fixture directory to the oracle's exact files.
	// warrantOracleConformanceScripts are the v1 conformance checker and its tests. The oracle's BUILD
	// runs both (VER04r 16.13 R23, S5b), beside make_conformance.py --check against the conformance
	// fixture directory, so an edit to any of them or to the fixture selects the oracle in addition to
	// the three lane runners the fixture already selects.
	warrantOracleConformanceScripts = []string{
		"code/scripts/warrant_corpus_conformance.py",
		"code/scripts/tests/test_warrant_corpus_conformance.py",
	}
	warrantOracleRowPaths = append(append(append([]string{}, warrantOraclePaths...), warrantAdopterFixtures()...),
		append([]string{warrantConformanceFixture}, warrantOracleConformanceScripts...)...)
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
		name:     "go/microbenchmark",
		language: "go",
		inputs:   "the shared paired microbenchmark results fixture",
		matches:  isMicrobenchmarkPairsFixture,
		paths:    []string{microbenchmarkPairsFixturePath},
	},
	{
		name:     "go/rank-statistics",
		language: "go",
		inputs:   "the shared paired microbenchmark results fixture",
		matches:  isMicrobenchmarkPairsFixture,
		paths:    []string{microbenchmarkPairsFixturePath},
	},
	{
		name:     "python/microbenchmark",
		language: "python",
		inputs:   "the shared paired microbenchmark results fixture",
		matches:  isMicrobenchmarkPairsFixture,
		paths:    []string{microbenchmarkPairsFixturePath},
	},
	{
		name:     "python/rank-statistics",
		language: "python",
		inputs:   "the shared paired microbenchmark results fixture",
		matches:  isMicrobenchmarkPairsFixture,
		paths:    []string{microbenchmarkPairsFixturePath},
	},
	{
		name:     "dart/hash-map",
		language: "dart",
		inputs:   "the shared DT18 hash-map contract and v1 fixture",
		matches:  isDt18HashMapInput,
		paths:    dt18HashMapPaths,
	},
	{
		name:     "swift/hash-map",
		language: "swift",
		inputs:   "the shared DT18 hash-map contract and v1 fixture",
		matches:  isDt18HashMapInput,
		paths:    dt18HashMapPaths,
	},
	{
		name:     "rust/hash-functions",
		language: "rust",
		inputs:   "the shared DT17 byte-hash and Dart profile contract plus v1 fixture",
		matches:  isDt17HashFunctionsInput,
		paths:    dt17HashFunctionsPaths,
	},
	{
		name:     "dart/hash-functions",
		language: "dart",
		inputs:   "the shared DT17 byte-hash and Dart profile contract plus v1 fixture",
		matches:  isDt17HashFunctionsInput,
		paths:    dt17HashFunctionsPaths,
	},
	{
		name:     "dart/fenwick-tree",
		language: "dart",
		inputs:   "the shared DT06 Fenwick contract and v1 conformance fixture",
		matches:  isFenwickTreeContractInput,
		paths:    fenwickTreeContractPaths,
	},
	{
		name:     "python/fenwick-tree",
		language: "python",
		inputs:   "the shared DT06 Fenwick contract and v1 conformance fixture",
		matches:  isFenwickTreeContractInput,
		paths:    fenwickTreeContractPaths,
	},
	{
		name:     "python/dsp-filters",
		language: "python",
		inputs:   "the shared Biquad/SOS conformance fixture read by package tests",
		matches:  isBiquadSosFixture,
		paths:    biquadSosFixturePaths,
	},
	{
		name:     "rust/dsp-filters",
		language: "rust",
		inputs:   "the shared Biquad/SOS conformance fixture read by package tests",
		matches:  isBiquadSosFixture,
		paths:    biquadSosFixturePaths,
	},
	{
		name:     "typescript/dsp-filters",
		language: "typescript",
		inputs:   "the shared Biquad/SOS conformance fixture read by package tests",
		matches:  isBiquadSosFixture,
		paths:    biquadSosFixturePaths,
	},
	{
		name:     "rust/vos-manifest",
		language: "rust",
		inputs:   "the vos_contract and vos_document grammar pairs it embeds",
		matches:  isVosGrammarInput,
		paths:    vosGrammarPaths,
	},
	{
		name:     "rust/pif-grammar",
		language: "rust",
		inputs:   "the pif.tokens and pif.grammar pair it embeds and the VER32 section 4.6 worked example its fixture is checked against",
		matches:  isPifGrammarInput,
		paths:    pifGrammarPaths,
	},
	{
		name:     "rust/programs/vos-link",
		language: "rust",
		inputs:   "the vos_contract and vos_document grammar pairs vos-manifest embeds, and the Cargo.toml and vos.toml of every vos-* crate, which its BUILD sweeps outside Cargo (check-publish, VOS-D104)",
		matches:  func(changed string) bool { return isVosGrammarInput(changed) || isVosPublishInput(changed) },
		paths:    vosGrammarPaths,
		globs:    vosPublishGlobs,
	},
	vosImplementationRow("vos-sched-fp-bitmap"),
	vosImplementationRow("vos-sched-any"),
	vosImplementationRow("vos-sched-rr"),
	vosImplementationRow("vos-probe-ready"),
	// CV2-F13: the non-VOS Rust crates whose BUILD runs `gen_evidence -- --check <crate>`.
	// TestWarrantEvidenceRowsCoverEveryEvidenceCrate pins this list to the checkout.
	warrantEvidenceRow("arithmetic"),
	warrantEvidenceRow("audio-device-sink"),
	warrantEvidenceRow("ct-compare"),
	warrantEvidenceRow("gf256"),
	warrantEvidenceRow("hash-functions"),
	warrantEvidenceRow("logic-gates"),
	logicGatesBasicRow(),
	warrantEvidenceRow("lzw"),
	warrantEvidenceRow("note-frequency"),
	warrantEvidenceRow("pcm-audio"),
	warrantEvidenceRow("sha1"),
	warrantEvidenceRow("wasm-leb128"),
	warrantEvidenceRow("x25519"),
	warrantEvidenceRow("zeroize"),
	{
		// The coverage test above reads every crate's evidence.bindings and BUILD files, so a
		// PR that adds an adopter (and with it an evidence.bindings) must run the build tool's
		// tests; otherwise the missing row is found in some later unrelated PR.  BUILD edits
		// are deliberately not a trigger: any Rust BUILD edit would select the build tool.
		name:     "go/programs/build-tool",
		language: "go",
		inputs:   "the evidence.bindings of every Rust crate, which TestWarrantEvidenceRowsCoverEveryEvidenceCrate reads (CV2-F13)",
		matches:  isRustEvidenceBindings,
		globs:    []string{rustEvidenceBindingsGlob},
	},
	vosContractRow("vos-sched-contract"),
	{
		name:     "rust/programs/vos-capacity-probe",
		language: "rust",
		inputs:   "the committed results file code/specs/data/vos30-capacity-results.json, the VOS30 spec and the two Warrant documents its E1 rows digest, which its BUILD checks outside Cargo (check, table --check)",
		matches:  isVosCapacityProbeInput,
		paths:    vosCapacityProbePaths,
	},
	{
		name:     "rust/vos-shape-check",
		language: "rust",
		inputs:   "the Rust 2021 and 2024 lexical token grammars it embeds",
		matches:  isVosShapeGrammarInput,
		paths:    vosShapeGrammarPaths,
	},
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
		name:     "rust/fixed-point",
		language: "rust",
		inputs:   "the GBL-020a1 Warrant model, v1 corpus, and exact Cargo workspace/build configuration",
		matches:  isFixedPointInput,
		paths:    fixedPointInputPaths,
	},
	{
		name:     "rust/game-clock",
		language: "rust",
		inputs:   "the GAME10 Warrant model, v1 corpus, and exact Cargo workspace/build configuration",
		matches:  isGameClockInput,
		paths:    gameClockInputPaths,
	},
	{
		name:     "typescript/game-clock",
		language: "typescript",
		inputs:   "the shared GAME10 fixed-step clock v1 corpus",
		matches:  isGameClockCorpusInput,
		paths:    typescriptGameClockCorpusPaths,
	},
	{
		name:     "rust/game-input-frame",
		language: "rust",
		inputs:   "the GAME input action frame v1 corpus",
		matches:  isGameInputFrameV1,
		paths:    gameInputFrameV1Paths,
	},
	{
		name:     "typescript/game-input-frame",
		language: "typescript",
		inputs:   "the shared GAME input action frame v1 cases and exact TypeScript/Rust proof-tool configuration inputs",
		matches:  isTypeScriptGameInputFrameInput,
		paths:    typescriptGameInputFrameInputs,
	},
	{
		name:     "python/trig",
		language: "python",
		inputs:   "the TRIG-REF-1 bit-exact corpus and the generator, oracle and validator its BUILD runs (--check)",
		matches:  isTrigReferenceInput,
		paths:    trigReferenceInputPaths,
	},
	{
		name:     "rust/trig",
		language: "rust",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "typescript/trig",
		language: "typescript",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "c/trig",
		language: "c",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "cpp/trig",
		language: "cpp",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "ruby/trig",
		language: "ruby",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "haskell/trig",
		language: "haskell",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "java/trig",
		language: "java",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "kotlin/trig",
		language: "kotlin",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "swift/trig",
		language: "swift",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "csharp/trig",
		language: "csharp",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "fsharp/trig",
		language: "fsharp",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "lua/trig",
		language: "lua",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "perl/trig",
		language: "perl",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "elixir/trig",
		language: "elixir",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		name:     "go/trig",
		language: "go",
		inputs:   "the shared TRIG-REF-1 bit-exact conformance corpus",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
	},
	{
		// Dart's test reads the generated trig_reference_vectors.dart, not the TSVs;
		// python/trig's BUILD --check keeps that file in step with the corpus, and a
		// corpus change re-runs these tests against it.
		name:     "dart/trig",
		language: "dart",
		inputs:   "the shared TRIG-REF-1 corpus, via the generated Dart vector source",
		matches:  isTrigReferenceCorpus,
		paths:    trigReferenceCorpusPaths,
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
		name:     "python/test-bootstrap",
		language: "python",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "python/test-bootstrap",
		language: "python",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "go/test-bootstrap",
		language: "go",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "go/test-bootstrap",
		language: "go",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "java/test-bootstrap",
		language: "java",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "java/test-bootstrap",
		language: "java",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "java/test-bootstrap",
		language: "java",
		inputs:   "the Go fuzz engine's reference-vector table its differential test parses",
		matches:  isGoFuzzReferenceVectors,
		paths:    goFuzzReferenceVectorPaths,
	},
	{
		name:     "kotlin/test-bootstrap",
		language: "kotlin",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "fsharp/test-bootstrap",
		language: "fsharp",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "fsharp/test-bootstrap",
		language: "fsharp",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "c/test-bootstrap",
		language: "c",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "cpp/test-bootstrap",
		language: "cpp",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "c/test-bootstrap",
		language: "c",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "c/test-bootstrap",
		language: "c",
		inputs:   "the Go fuzz engine's reference-vector table its differential test parses",
		matches:  isGoFuzzReferenceVectors,
		paths:    goFuzzReferenceVectorPaths,
	},
	{
		name:     "csharp/test-bootstrap",
		language: "csharp",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "ruby/test_bootstrap",
		language: "ruby",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "perl/test-bootstrap",
		language: "perl",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "swift/test-bootstrap",
		language: "swift",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "lua/test_bootstrap",
		language: "lua",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "elixir/test_bootstrap",
		language: "elixir",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "lua/test_bootstrap",
		language: "lua",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "lua/test_bootstrap",
		language: "lua",
		inputs:   "the Go fuzz engine's reference-vector table its differential test parses",
		matches:  isGoFuzzReferenceVectors,
		paths:    goFuzzReferenceVectorPaths,
	},
	{
		name:     "ruby/test_bootstrap",
		language: "ruby",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "swift/test-bootstrap",
		language: "swift",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "swift/test-bootstrap",
		language: "swift",
		inputs:   "the Go fuzz engine's reference-vector table its differential test parses",
		matches:  isGoFuzzReferenceVectors,
		paths:    goFuzzReferenceVectorPaths,
	},
	{
		name:     "ruby/test_bootstrap",
		language: "ruby",
		inputs:   "the Go fuzz engine's reference-vector table its differential test parses",
		matches:  isGoFuzzReferenceVectors,
		paths:    goFuzzReferenceVectorPaths,
	},
	{
		name:     "haskell/test-bootstrap",
		language: "haskell",
		inputs:   "the shared pbt-v1 property-engine conformance corpus",
		matches:  isPbtV1Corpus,
		paths:    pbtV1CorpusPaths,
	},
	{
		name:     "elixir/test_bootstrap",
		language: "elixir",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "elixir/test_bootstrap",
		language: "elixir",
		inputs:   "the Go fuzz engine's reference-vector table its differential test parses",
		matches:  isGoFuzzReferenceVectors,
		paths:    goFuzzReferenceVectorPaths,
	},
	{
		name:     "kotlin/test-bootstrap",
		language: "kotlin",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "csharp/test-bootstrap",
		language: "csharp",
		inputs:   "the shared fuzz-v1 fuzz-engine conformance corpus",
		matches:  isFuzzV1Corpus,
		paths:    fuzzV1CorpusPaths,
	},
	{
		name:     "csharp/test-bootstrap",
		language: "csharp",
		inputs:   "the Go fuzz engine's reference-vector table its differential test parses",
		matches:  isGoFuzzReferenceVectors,
		paths:    goFuzzReferenceVectorPaths,
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
		inputs:   "canonical Warrant grammars, syntax corpus, vector fixtures, transport gate and the shared negative-control checker",
		matches:  isWarrantGrammarInput,
		paths:    warrantGrammarPaths,
		globs:    []string{warrantDocumentGlob},
	},
	{
		name:      "rust/ca-assurance-report",
		language:  "rust",
		inputs:    "the Warrant lane inputs plus fixed-point package evidence consumed by its assurance sandbox",
		matches:   isFixedPointReporterInput,
		paths:     append(append([]string{}, warrantAssurancePaths...), fixedPointReporterInputs...),
		treePaths: fixedPointReporterTreePaths,
	},
	{
		name:      "rust/warrant-ir",
		language:  "rust",
		inputs:    "the fixed-point Warrant model, source tree and evidence bindings read by its proof integration test",
		matches:   isFixedPointWarrantIRInput,
		paths:     fixedPointWarrantIRInputs,
		treePaths: fixedPointWarrantIRTreePaths,
	},
	{
		name:     "rust/arithmetic",
		language: "rust",
		inputs:   "the Warrant consumer registry its assurance_report.json lists lane rungs from (VER04i 8.3)",
		matches:  isWarrantLaneRegistryInput,
		paths:    warrantLaneRegistryPaths,
	},
	{
		name:     "rust/ct-compare",
		language: "rust",
		inputs:   "the Warrant consumer registry its assurance_report.json lists lane rungs from (VER04i 8.3)",
		matches:  isWarrantLaneRegistryInput,
		paths:    warrantLaneRegistryPaths,
	},
	{
		name:     "rust/gf256",
		language: "rust",
		inputs:   "the Warrant consumer registry its assurance_report.json lists lane rungs from (VER04i 8.3)",
		matches:  isWarrantLaneRegistryInput,
		paths:    warrantLaneRegistryPaths,
	},
	{
		name:     "rust/logic-gates",
		language: "rust",
		inputs:   "the Warrant consumer registry its assurance_report.json lists lane rungs from (VER04i 8.3)",
		matches:  isWarrantLaneRegistryInput,
		paths:    warrantLaneRegistryPaths,
	},
	{
		name:     "rust/lzw",
		language: "rust",
		inputs:   "the Warrant consumer registry its assurance_report.json lists lane rungs from (VER04i 8.3)",
		matches:  isWarrantLaneRegistryInput,
		paths:    warrantLaneRegistryPaths,
	},
	{
		name:     "rust/wasm-leb128",
		language: "rust",
		inputs:   "the Warrant consumer registry its assurance_report.json lists lane rungs from (VER04i 8.3)",
		matches:  isWarrantLaneRegistryInput,
		paths:    warrantLaneRegistryPaths,
	},
	{
		name:     "python/warrant-corpus",
		language: "python",
		inputs:   "the Warrant spec, corpus and consumer registry its tests run",
		matches:  isWarrantPythonRunnerInput,
		paths:    warrantPythonLaneCorpusPaths,
	},
	{
		name:     "python/warrant-oracle",
		language: "python",
		inputs:   "the adopter Warrant specs, the v1 ct-compare corpus, each adopter's bindings and committed v2 fixture the oracle gates (warrantAdopters), and the conformance fixture with the v1 checker and its tests its BUILD runs (S5b)",
		matches:  isWarrantOracleInput,
		paths:    warrantOracleRowPaths,
	},
	{
		name:     "rust/warrant-corpus-gen",
		language: "rust",
		inputs:   "each adopter's spec, bindings and committed v2 fixture its BUILD --check regenerates (VER04r 16.8, D32; warrantAdopters), and the oracle's accept-blind, fixed-forms, omit-example, tuple-forms, fixed-eval, off-by-one-consumed, scalar-faults, data-forms and data-eval fixtures its tests reproduce",
		matches:  isWarrantCorpusGenInput,
		paths:    warrantCorpusGenPaths,
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
		matches:  isWarrantGoRunnerInput,
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

// isMicrobenchmarkPairsFixture matches only the canonical, externally owned
// paired-results fixture. It is an exact file input: neighboring names,
// directory paths, and noncanonical spellings must not select these tests.
func isMicrobenchmarkPairsFixture(changed string) bool {
	return changed == microbenchmarkPairsFixturePath
}

// repositoryInputsForPackage returns exact repository-relative files that
// package tests read and that must participate in the fallback cache key.
func repositoryInputsForPackage(packageName string) []string {
	switch packageName {
	case "go/microbenchmark", "go/rank-statistics", "python/microbenchmark", "python/rank-statistics":
		return []string{microbenchmarkPairsFixturePath}
	case "rust/game-clock":
		return gameClockInputPaths
	case "typescript/game-input-frame":
		return typescriptGameInputFrameInputs
	case "swift/hash-map", "dart/hash-map":
		return swiftHashMapFixtureInputs
	default:
		return nil
	}
}

// isDt18HashMapInput selects the two exact shared contracts and the
// versioned v1 fixture tree, excluding neighboring spec and fixture names.
func isDt18HashMapInput(changed string) bool {
	if path.Clean(changed) != changed {
		return false
	}
	return changed == "code/specs/DT18-hash-map.md" ||
		changed == "code/specs/DT18-hash-map-dart-profile.md" ||
		strings.HasPrefix(changed, "code/specs/fixtures/hash-map-v1/")
}

// warrantLaneRegistryPaths is the consumer registry. Every Rust package that
// uses ca-assurance-report records one rung per registry lane in its
// assurance_report.json (WD-004b2), so adding, flipping or removing a row
// stales all of their reports; their BUILD runs `--check`, which re-derives the
// rungs. Listed by name because the registry is data no Cargo graph edge sees.
var warrantLaneRegistryPaths = []string{"code/specs/fixtures/ct-compare-warrant-v1/consumers.json"}

// isWarrantLaneRegistryInput is true only for the registry file itself.
func isWarrantLaneRegistryInput(changed string) bool {
	return changed == "code/specs/fixtures/ct-compare-warrant-v1/consumers.json"
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

func isRustWorkspaceCargoInput(changed string) bool {
	for _, input := range rustWorkspaceCargoInputs {
		if changed == input {
			return true
		}
	}
	return false
}

func isFixedPointPackageFile(changed string) bool {
	return changed == "code/packages/rust/fixed-point/Cargo.toml" ||
		changed == "code/packages/rust/fixed-point/effect_certificate.json" ||
		changed == fixedPointBindingsInput ||
		changed == "code/packages/rust/fixed-point/assurance-profile.json" ||
		isUnderExactInputTree(changed, "code/packages/rust/fixed-point/src") ||
		isUnderExactInputTree(changed, "code/packages/rust/fixed-point/evidence")
}

func isUnderExactInputTree(changed, root string) bool {
	return changed == root || strings.HasPrefix(changed, root+"/")
}

func isFixedPointReporterInput(changed string) bool {
	return isWarrantAssuranceInput(changed) || isFixedPointPackageFile(changed) ||
		changed == "code/specs/fixed-point.warrant" || isRustWorkspaceCargoInput(changed)
}

func isFixedPointWarrantIRInput(changed string) bool {
	return changed == "code/packages/rust/fixed-point/Cargo.toml" ||
		changed == "code/specs/fixed-point.warrant" ||
		changed == fixedPointBindingsInput ||
		isUnderExactInputTree(changed, "code/packages/rust/fixed-point/src") ||
		isRustWorkspaceCargoInput(changed)
}

// warrantAdopter is one row of warrantAdopters: the module, its Warrant spec, its committed
// v2 fixture directory and the Rust evidence.bindings that fixture's profile is checked against.
// bindings is optional: empty means the fixture is generated and verified without --bindings.
type warrantAdopter struct {
	module   string
	spec     string
	fixture  string
	bindings string
}

// warrantAdopterBindings lists every adopter's bindings file, in table order.
func warrantAdopterBindings() []string {
	var paths []string
	for _, adopter := range warrantAdopters {
		if adopter.bindings != "" {
			paths = append(paths, adopter.bindings)
		}
	}
	return paths
}

// warrantAdopterFixtures lists every adopter's fixture directory, in table order.
func warrantAdopterFixtures() []string {
	var paths []string
	for _, adopter := range warrantAdopters {
		paths = append(paths, adopter.fixture)
	}
	return paths
}

// warrantAdopterInputs lists, per adopter in table order, its spec, fixture directory and
// bindings: the declared inputs of the generator's --check line.
func warrantAdopterInputs() []string {
	var paths []string
	for _, adopter := range warrantAdopters {
		paths = append(paths, adopter.spec, adopter.fixture)
		if adopter.bindings != "" {
			paths = append(paths, adopter.bindings)
		}
	}
	return paths
}

// isWarrantOracleInput selects python/warrant-oracle when an adopter spec it
// parses, the v1 corpus it gates, the Rust bindings its verify-v2 lines read, or
// a file of a committed v2 fixture changes, or (S5b) a file of the conformance
// fixture, the v1 conformance checker or its test file. Exact files, plus each
// fixture directory by slash-delimited prefix (so ct-compare-warrant-v20 and
// warrant-corpus-conformance-v10 are not it).
func isWarrantOracleInput(changed string) bool {
	for _, path := range warrantOraclePaths {
		if changed == path {
			return true
		}
	}
	for _, path := range warrantOracleConformanceScripts {
		if changed == path {
			return true
		}
	}
	return isWarrantCorpusV2Fixture(changed) || isWarrantConformanceInput(changed)
}

// isWarrantCorpusV2Fixture reports whether a changed path is a file of a
// committed adopter v2 fixture directory (a warrantAdopters row).
//
//	code/specs/fixtures/ct-compare-warrant-v2/corpus.json         -> true
//	code/specs/fixtures/ct-compare-warrant-v2/chunks/eq_agrees.txt -> true
//	code/specs/fixtures/x25519-warrant-v2/corpus.json             -> true
//	code/specs/fixtures/ct-compare-warrant-v20/corpus.json        -> false
//	code/specs/fixtures/ct-compare-warrant-v2                     -> false (a directory, not a file)
func isWarrantCorpusV2Fixture(changed string) bool {
	for _, adopter := range warrantAdopters {
		if strings.HasPrefix(changed, adopter.fixture+"/") {
			return true
		}
	}
	return false
}

// isWarrantCorpusGenInput selects rust/warrant-corpus-gen, whose BUILD runs the
// generator's --check against each committed v2 fixture with its adopter's spec
// and bindings, and whose tests regenerate the oracle's accept-blind and omit-example
// (CV2-S10-1) fixtures reproduce the oracle's fixed-forms lowering dump (CV2-S10b) and the four tuple-forms dumps (CV2-S10t-2).
func isWarrantCorpusGenInput(changed string) bool {
	for _, adopter := range warrantAdopters {
		if changed == adopter.spec || (adopter.bindings != "" && changed == adopter.bindings) {
			return true
		}
	}
	return isWarrantCorpusV2Fixture(changed) ||
		strings.HasPrefix(changed, warrantAcceptBlindFixture+"/") ||
		strings.HasPrefix(changed, warrantFixedFormsFixture+"/") ||
		strings.HasPrefix(changed, warrantOmitExampleFixture+"/") ||
		strings.HasPrefix(changed, warrantTupleFormsFixture+"/") ||
		strings.HasPrefix(changed, warrantFixedEvalFixture+"/") ||
		strings.HasPrefix(changed, warrantOffByOneConsumedFixture+"/") ||
		strings.HasPrefix(changed, warrantScalarFaultsFixture+"/") ||
		strings.HasPrefix(changed, warrantDataFormsFixture+"/") ||
		strings.HasPrefix(changed, warrantDataEvalFixture+"/")
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

// isWarrantGoRunnerInput is the Go counterpart: go/warrant-corpus runs the
// conformance fixture in its own tests too (VER04r R23).
func isWarrantGoRunnerInput(changed string) bool {
	return isWarrantGoLaneCorpusInput(changed) || isWarrantConformanceInput(changed)
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
		// ADL-001a: the profile-header and `realizes` diagnostics corpus that
		// tests/profile_algorithm.rs reads from disk.
		strings.HasPrefix(changed, "code/specs/fixtures/warrant-profile-algorithm-v1/") ||
		// WX-E: the pinned vector tables the package's tests load (the pin in
		// the .warrant document fails closed when these bytes change).
		strings.HasPrefix(changed, "code/specs/fixtures/warrant-vectors-synthetic-v1/") ||
		changed == "code/scripts/warrant_syntax_corpus.py" ||
		changed == "code/scripts/tests/test_warrant_syntax_corpus.py" ||
		changed == "code/scripts/warrant_negctl.py" ||
		changed == "code/scripts/tests/test_warrant_negctl.py" ||
		changed == "code/scripts/ver04p_domain_witnesses.py" ||
		changed == "code/scripts/tests/test_ver04p_domain_witnesses.py" ||
		changed == "code/scripts/tfv.py" ||
		changed == "code/specs/ct-compare.warrant" ||
		changed == "code/specs/fixtures/ct-compare-warrant-v1/cases.tfv" ||
		changed == "code/specs/fixtures/ct-compare-warrant-v1/corpus.json" ||
		changed == "code/specs/fixtures/ct-compare-warrant-v1/consumers.json" ||
		changed == "code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json" ||
		changed == "code/specs/VER04-warrant-language.md" ||
		changed == "code/specs/VER04a-warrant-source-admission.md" ||
		changed == "code/specs/VER04p-warrant-floats.md"
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

// isPifGrammarInput reports whether a changed path is one rust/pif-grammar
// embeds (slash-delimited prefix: no lookalike directory).
//
//	code/grammars/pif/pif.tokens                    → true
//	code/specs/VER32-proof-interchange-format.md    → true
//	code/grammars/pifx/pif.tokens                   → false
func isPifGrammarInput(changed string) bool {
	return strings.HasPrefix(changed, "code/grammars/pif/") ||
		changed == "code/specs/VER32-proof-interchange-format.md"
}

// isVosGrammarInput reports whether a changed path is in a VOS grammar pair
// that vos-manifest embeds (slash-delimited prefixes: no lookalike directory).
//
//	code/grammars/vos_contract/vos_contract.tokens → true
//	code/grammars/vos_contract2/x                  → false
func isVosGrammarInput(changed string) bool {
	return strings.HasPrefix(changed, "code/grammars/vos_contract/") ||
		strings.HasPrefix(changed, "code/grammars/vos_document/")
}

// isVosPublishInput reports whether a changed path is a file `vos-link check-publish` reads: the
// `Cargo.toml` or `vos.toml` of a DIRECT child named `vos-*` of `code/packages/rust` or
// `code/programs/rust` (VOS-D104).  The sweep does not recurse, so neither does this.
//
//	code/packages/rust/vos-manifest/Cargo.toml        → true
//	code/programs/rust/vos-capacity-probe/vos.toml    → true
//	code/packages/rust/vos-manifest/src/Cargo.toml    → false (nested)
//	code/packages/rust/vosx/Cargo.toml                → false
//	code/packages/python/vos-a/Cargo.toml             → false
func isVosPublishInput(changed string) bool {
	parts := strings.Split(changed, "/")
	return len(parts) == 5 &&
		parts[0] == "code" &&
		(parts[1] == "packages" || parts[1] == "programs") &&
		parts[2] == "rust" &&
		strings.HasPrefix(parts[3], "vos-") &&
		(parts[4] == "Cargo.toml" || parts[4] == "vos.toml")
}

// isVosToolInput reports whether a changed path is a source of a tool that a VOS
// implementation's BUILD runs outside Cargo (VOS-D066): a directory of vosToolPaths
// (the repository-relative path with a trailing slash, never a lookalike) or one of the
// two vos_* grammars.
//
//	code/programs/rust/vos-link/src/check.rs   → true
//	code/packages/rust/vos-manifest-x/src/a.rs → false
func isVosToolInput(changed string) bool {
	for _, d := range vosToolPaths {
		if strings.HasPrefix(changed, d+"/") {
			return true
		}
	}
	return false
}

// vosImplementationRow is the row of one VOS implementation crate (`code/packages/rust/<crate>`,
// kind production, reference or any).  It selects the crate when a VOS tool changes
// (isVosToolInput) and when ITS generated document `code/specs/<crate>.warrant` changes: that
// file lives outside the package, so without the row a PR that edits only the document (say,
// weakens a claim) selects nothing and `check-doc` and `gen_evidence --check` never run
// (VOS02 D-GD3: the document name is exactly the crate name).
func vosImplementationRow(crate string) repositoryInputConsumer {
	doc := "code/specs/" + crate + ".warrant"
	return repositoryInputConsumer{
		name:     "rust/" + crate,
		language: "rust",
		inputs:   "its generated document " + doc + " and the VOS tools its BUILD runs outside Cargo (vos-link check-doc, vos-shape-check, the evidence generator, the capability analyzer)",
		matches:  func(changed string) bool { return changed == doc || isVosToolInput(changed) },
		paths:    append([]string{doc}, vosToolPaths...),
	}
}

// rustEvidenceBindingsGlob is exactly isRustEvidenceBindings as a glob.
const rustEvidenceBindingsGlob = "code/packages/rust/*/evidence.bindings"

// isRustEvidenceBindings reports whether a changed path is `code/packages/rust/<dir>/evidence.bindings`:
// one directory segment (no nesting, no `..`, no backslash), the exact file name.
func isRustEvidenceBindings(changed string) bool {
	parts := strings.Split(changed, "/")
	return len(parts) == 5 &&
		parts[0] == "code" && parts[1] == "packages" && parts[2] == "rust" &&
		parts[3] != "" && parts[3] != "." && parts[3] != ".." && !strings.Contains(parts[3], `\`) &&
		parts[4] == "evidence.bindings"
}

// warrantEvidenceInputsPrefix starts the inputs text of every warrantEvidenceRow row; the
// coverage test recognises those rows by it.
const warrantEvidenceInputsPrefix = "its Warrant spec "

// warrantEvidenceRow is the row of one non-VOS Rust crate whose BUILD runs
// `cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- --check <crate>`
// (CV2-F13).  The spec `code/specs/<crate>.warrant` lives outside the package, so without the
// row a PR that edits only the spec (weakens a claim, say) selects the generator, the oracle
// and the grammar crate but not the crate whose BUILD checks its committed evidence against
// that spec: the check then runs first in some unrelated later PR.  The spec name is exactly the
// crate name (VER04h).  The VOS crates (vosImplementationRow) already carry the same rule.
// TestWarrantEvidenceRowsCoverEveryEvidenceCrate pins the rows to the BUILD and evidence.bindings
// files of the checkout.
//
//	code/specs/lzw.warrant        → rust/lzw
//	code/specs/lzw.warrant.draft  → nothing
//	code/specs/lzw2.warrant       → nothing (a lookalike name)
func warrantEvidenceRow(crate string) repositoryInputConsumer {
	spec := "code/specs/" + crate + ".warrant"
	return repositoryInputConsumer{
		name:     "rust/" + crate,
		language: "rust",
		inputs:   warrantEvidenceInputsPrefix + spec + ", which its BUILD checks the committed evidence against (gen_evidence --check, CV2-F13)",
		matches:  func(changed string) bool { return changed == spec },
		paths:    []string{spec},
	}
}

// logicGatesBasicPaths are the exact files (and the one fixture directory) the Rust
// logic-gates BUILD checks outside its own package directory with
// code/scripts/logic_gates_basic_drift.py (LG02 section 4.3, LGE-S1b).
var logicGatesBasicPaths = []string{
	"code/specs/logic-gates.warrant",
	"code/specs/logic-gates-basic.warrant",
	"code/specs/fixtures/logic-gates-basic-warrant-v2",
	"code/scripts/logic_gates_basic_drift.py",
	"code/scripts/tests/test_logic_gates_basic_drift.py",
}

// logicGatesBasicFixtureDir is the one directory of logicGatesBasicPaths whose every file counts.
const logicGatesBasicFixtureDir = "code/specs/fixtures/logic-gates-basic-warrant-v2"

// isLogicGatesBasicInput reports whether a changed path is an input of the basic-gates drift
// guard: an exact file, or a file beneath the fixture directory (no `..`, `.` or empty segment, no
// backslash, no lookalike directory name).
//
//	code/specs/logic-gates-basic.warrant                              → true
//	code/specs/logic-gates-basic.warrant.draft                        → false
//	code/specs/logic-gates-basic2.warrant                             → false
//	code/specs/fixtures/logic-gates-basic-warrant-v2/corpus.json      → true
//	code/specs/fixtures/logic-gates-basic-warrant-v2x/corpus.json     → false
func isLogicGatesBasicInput(changed string) bool {
	for _, p := range logicGatesBasicPaths {
		if p != logicGatesBasicFixtureDir && changed == p {
			return true
		}
	}
	rest, ok := strings.CutPrefix(changed, logicGatesBasicFixtureDir+"/")
	if !ok || strings.Contains(changed, `\`) {
		return false
	}
	for _, seg := range strings.Split(rest, "/") {
		if seg == "" || seg == "." || seg == ".." {
			return false
		}
	}
	return true
}

// logicGatesBasicRow selects rust/logic-gates when the basic-gates Warrant document, its fixture
// or its drift script (or the script's tests) changes: they live under code/specs and
// code/scripts, outside the package, and without the row the guard could be skipped in the very PR
// that breaks it.  TestLogicGatesBasicRowIsPinnedToTheBuildLine pins it to the BUILD files.
func logicGatesBasicRow() repositoryInputConsumer {
	return repositoryInputConsumer{
		name:      "rust/logic-gates",
		language:  "rust",
		inputs:    "the basic-gates Warrant document, its fixture and the drift guard its BUILD runs (LG02 4.3)",
		matches:   isLogicGatesBasicInput,
		paths:     append([]string{}, logicGatesBasicPaths...),
		treePaths: []string{logicGatesBasicFixtureDir},
	}
}

// vosContractRow is the row of one VOS contract crate (`code/packages/rust/<crate>`, kind
// contract, VOS-039b-1).  Its BUILD runs `vos-link check-contract` and `check-component` outside
// Cargo (the generated Rust of the contract crate does not depend on them), so a change to a VOS
// tool or grammar must select it.  A contract has no generated document of its own (the
// implementations embed its parts), so unlike vosImplementationRow nothing under code/specs
// selects it; its own files select it through the package path.
func vosContractRow(crate string) repositoryInputConsumer {
	return repositoryInputConsumer{
		name:     "rust/" + crate,
		language: "rust",
		inputs:   "the VOS tools its BUILD runs outside Cargo (vos-link check-contract, vos-link check-component, vos-shape-check, the capability analyzer)",
		matches:  isVosToolInput,
		paths:    append([]string{}, vosToolPaths...),
	}
}

// isVosCapacityProbeInput reports whether a changed path is one of the exact files the
// capacity probe checks outside its Cargo edges (VOS-028b): no prefix, no lookalike.
//
//	code/specs/data/vos30-capacity-results.json   → true
//	code/specs/data/vos30-capacity-results.json.x → false
//	code/specs/VOS30-proof-route-and-capacity.md  → true
func isVosCapacityProbeInput(changed string) bool {
	for _, p := range vosCapacityProbePaths {
		if changed == p {
			return true
		}
	}
	return false
}

// isVosShapeGrammarInput reports whether a changed path is one of the two Rust
// token grammars vos-shape-check embeds.
func isVosShapeGrammarInput(changed string) bool {
	return changed == "code/grammars/rust/rust1.97.0-e2021.tokens" ||
		changed == "code/grammars/rust/rust1.97.0-e2024.tokens"
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

// isPbtV1Corpus reports whether a changed path is inside the pbt-v1 corpus
// directory (cases.json and the generated cases.tfv).
//
//	code/specs/fixtures/pbt-v1/cases.json   → true
//	code/specs/fixtures/pbt-v1/cases.tfv    → true
//	code/specs/fixtures/pbt-v10/cases.json  → false
//	code/specs/fixtures/rng-v1/cases.json   → false
func isPbtV1Corpus(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/pbt-v1/")
}

// isFuzzV1Corpus reports whether a changed path is inside the fuzz-v1 corpus
// directory (cases.json and the generated cases.tfv).
//
//	code/specs/fixtures/fuzz-v1/cases.json   → true
//	code/specs/fixtures/fuzz-v1/cases.tfv    → true
//	code/specs/fixtures/fuzz-v10/cases.json  → false
//	code/specs/fixtures/pbt-v1/cases.json    → false
func isFuzzV1Corpus(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/fuzz-v1/")
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

// isGoFuzzReferenceVectors reports whether a changed path is the Go fuzz engine's
// generated reference-vector table (exact path; its neighbours are not inputs):
//
//	code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go → true
//	code/packages/go/test-bootstrap/fuzz/mutate_test.go           → false
func isGoFuzzReferenceVectors(changed string) bool {
	return changed == "code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go"
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

// isFixedPointInput selects only the exact repository-owned inputs consumed by
// the Rust fixed-point package. Nearby versions and lookalikes remain unselected.
func isFixedPointInput(changed string) bool {
	for _, input := range fixedPointInputPaths {
		if changed == input {
			return true
		}
	}
	return false
}

// isGameClockInput selects the exact repository-owned files hashed for the
// Rust GAME10 consumer. The corpus consumer below retains its narrower reader
// relation; nearby versions and lookalikes do not select this package.
func isGameClockInput(changed string) bool {
	for _, input := range gameClockInputPaths {
		if changed == input {
			return true
		}
	}
	return false
}

// isGameClockCorpusInput selects only the shared GAME10 v1 vectors. The
// TypeScript consumer intentionally does not read or execute the Warrant spec.
func isGameClockCorpusInput(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/game-clock-v1/")
}

// isGameInputFrameV1 selects only files inside GAME's exact v1 vector directory.
// Neighboring versions, directory-only paths, and similarly named fixtures do not select it.
func isGameInputFrameV1(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/game-input-frame-v1/")
}

func isTypeScriptGameInputFrameInput(changed string) bool {
	for _, input := range typescriptGameInputFrameInputs {
		if changed == input {
			return true
		}
	}
	return false
}

// isBiquadSosFixture selects files in the versioned shared DSP fixture while
// excluding adjacent lookalike versions.
func isBiquadSosFixture(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/biquad-sos-v1/")
}

// isFenwickTreeContractInput selects only the shared DT06 contract and v1
// fixture, excluding neighboring versions and lookalike paths.
func isFenwickTreeContractInput(changed string) bool {
	if path.Clean(changed) != changed {
		return false
	}
	return changed == "code/specs/DT06-fenwick-tree.md" ||
		strings.HasPrefix(changed, "code/specs/fixtures/fenwick-tree-v1/")
}

// isDt17HashFunctionsInput selects the two exact shared contracts and the
// versioned v1 fixture tree, excluding neighboring spec and fixture names.
func isDt17HashFunctionsInput(changed string) bool {
	if path.Clean(changed) != changed {
		return false
	}
	return changed == "code/specs/DT17-hash-functions.md" ||
		changed == "code/specs/DT17b-hash-functions-dart-profile.md" ||
		strings.HasPrefix(changed, "code/specs/fixtures/hash-functions-v1/")
}

// isTrigReferenceCorpus reports whether a changed path is a file of the shared
// TRIG-REF-1 conformance corpus. A slash-delimited prefix, so a lookalike
// directory such as trig-reference-v10 is not selected.
func isTrigReferenceCorpus(changed string) bool {
	return strings.HasPrefix(changed, "code/specs/fixtures/trig-reference-v1/")
}

// isTrigReferenceInput reports whether a changed path is the TRIG-REF-1 corpus or
// one of the files that generates and validates it. The python/trig BUILD runs the
// generator's --check, so each of these must select that package:
//
//	code/specs/fixtures/trig-reference-v1/edge.tsv            → true
//	code/scripts/generate_trig_reference_fixture.py           → true
//	code/scripts/trig_reference_xcheck/xcheck.c               → true
//	code/scripts/trig_ref_exact.py                            → true
//	code/scripts/generate_trig_reference_fixture.py.bak       → false (exact filenames)
func isTrigReferenceInput(changed string) bool {
	if isTrigReferenceCorpus(changed) {
		return true
	}
	switch changed {
	case "code/scripts/generate_trig_reference_fixture.py",
		"code/scripts/generate_trig_reference_dart_fixtures.py",
		"code/scripts/trig_ref_exact.py",
		"code/scripts/trig_ref_softfloat.py",
		"code/scripts/tests/test_trig_reference_fixture.py",
		"code/scripts/tests/test_trig_reference_dart_fixtures.py":
		return true
	}
	return strings.HasPrefix(changed, "code/scripts/trig_reference_xcheck/")
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
