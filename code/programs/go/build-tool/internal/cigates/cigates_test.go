package cigates

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

// realRegistryPath locates the checked-in registry from this package's
// directory: internal/cigates → internal → build-tool → go → programs → code.
const realRegistryPath = "../../../../../specs/data/ci-gates.json"

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

// testRegistry is a two-gate stand-in with one package-only match and one
// path-only match, so each clause can be exercised in isolation.
func testRegistry() *Registry {
	return &Registry{
		SchemaVersion: 1,
		Gates: map[string]Gate{
			"alpha-job": {
				Scope:       ScopeJob,
				Description: "Fires on a package.",
				Packages:    []string{"rust/alpha", "python/alpha"},
				Paths:       []string{"code/fixtures/alpha/**"},
			},
			"beta-job": {
				Scope:       ScopeJob,
				Description: "Fires on a path.",
				Packages:    []string{"ruby/beta"},
				Paths:       []string{"code/grammars/beta/**", "code/scripts/beta.py"},
			},
		},
	}
}

func writeRegistry(t *testing.T, body string) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "ci-gates.json")
	if err := os.WriteFile(path, []byte(body), 0o644); err != nil {
		t.Fatalf("write fixture registry: %v", err)
	}
	return path
}

func assertAll(t *testing.T, got map[string]bool, want bool, context string) {
	t.Helper()
	for id, v := range got {
		if v != want {
			t.Errorf("%s: gate %q = %t, want %t", context, id, v, want)
		}
	}
}

// ---------------------------------------------------------------------------
// The five "run everything" escapes
// ---------------------------------------------------------------------------

func TestForceRunsEverything(t *testing.T) {
	got := Evaluate(testRegistry(), map[string]bool{}, []string{"README.md"}, true)
	assertAll(t, got, true, "force")
}

func TestNilAffectedRunsEverything(t *testing.T) {
	// nil affected means "rebuild all" — distinct from an empty map, which
	// means "nothing changed".
	got := Evaluate(testRegistry(), nil, []string{"README.md"}, false)
	assertAll(t, got, true, "nil affected set")
}

func TestNilChangedFilesRunsEverything(t *testing.T) {
	// Change detection could not produce a file list, so we know nothing.
	got := Evaluate(testRegistry(), map[string]bool{}, nil, false)
	assertAll(t, got, true, "nil changed files")
}

func TestMainPushRunsEverythingViaForce(t *testing.T) {
	// Main merges are the safety net that makes gating pull requests safe: a
	// gate that is wrong on a PR is still caught on merge. ci.yml passes -force
	// on main, so a main push reaches Evaluate as force=true — there is no
	// separate is-main parameter, which keeps the safety net on the one code
	// path that production actually exercises.
	got := Evaluate(testRegistry(), map[string]bool{}, []string{"code/learning/x.md"}, true)
	assertAll(t, got, true, "main push (force)")
}

func TestWorkflowChangeSelfTestsEveryGate(t *testing.T) {
	got := Evaluate(testRegistry(), map[string]bool{}, []string{CIWorkflowPath}, false)
	assertAll(t, got, true, "ci.yml changed")
}

func TestRegistryChangeSelfTestsEveryGate(t *testing.T) {
	got := Evaluate(testRegistry(), map[string]bool{}, []string{RegistryPath}, false)
	assertAll(t, got, true, "registry changed")
}

// ---------------------------------------------------------------------------
// The two clauses
// ---------------------------------------------------------------------------

func TestPackageIntersectionFiresOnlyThatGate(t *testing.T) {
	affected := map[string]bool{"rust/alpha": true}
	got := Evaluate(testRegistry(), affected, []string{"code/packages/rust/alpha/src/lib.rs"}, false)

	if !got["alpha-job"] {
		t.Error("alpha-job should fire: rust/alpha is in the affected closure")
	}
	if got["beta-job"] {
		t.Error("beta-job should not fire: nothing it declares was touched")
	}
}

func TestPathGlobFiresWithoutAnyAffectedPackage(t *testing.T) {
	// This is the case a package-only registry gets wrong. Files under
	// code/grammars map to ZERO packages, so the affected closure is empty and
	// only the path clause can save the gate.
	got := Evaluate(testRegistry(), map[string]bool{}, []string{"code/grammars/beta/beta.tokens"}, false)

	if !got["beta-job"] {
		t.Error("beta-job should fire from its path glob even with an empty affected closure")
	}
	if got["alpha-job"] {
		t.Error("alpha-job should not fire")
	}
}

func TestExactPathMatchFires(t *testing.T) {
	got := Evaluate(testRegistry(), map[string]bool{}, []string{"code/scripts/beta.py"}, false)
	if !got["beta-job"] {
		t.Error("beta-job should fire on its exact declared path")
	}
}

// TestUnrelatedChangeSkipsEveryGate is the load-bearing negative. Without it a
// gate that never fires is indistinguishable from a gate that always passes.
func TestUnrelatedChangeSkipsEveryGate(t *testing.T) {
	affected := map[string]bool{"typescript/human-language-data": true}
	changed := []string{
		"code/learning/human-languages/hindi/lessons/ch01.md",
		"code/packages/typescript/human-language-data/src/index.ts",
	}

	got := Evaluate(testRegistry(), affected, changed, false)
	assertAll(t, got, false, "unrelated change")
}

func TestNearMissPackageNameDoesNotFire(t *testing.T) {
	// "rust/alpha-core" is not "rust/alpha". Matching must be exact, not prefix.
	affected := map[string]bool{"rust/alpha-core": true}
	got := Evaluate(testRegistry(), affected, []string{"code/packages/rust/alpha-core/src/lib.rs"}, false)
	if got["alpha-job"] {
		t.Error("alpha-job fired on rust/alpha-core; package matching must be exact")
	}
}

// ---------------------------------------------------------------------------
// Output naming
// ---------------------------------------------------------------------------

func TestOutputName(t *testing.T) {
	cases := map[string]string{
		"d18f-message-conformance": "run_d18f_message_conformance",
		"ruby-grammar-regen-check": "run_ruby_grammar_regen_check",
		"contracts-adj-stdlib":     "run_contracts_adj_stdlib",
		"already_underscored":      "run_already_underscored",
	}
	for id, want := range cases {
		if got := OutputName(id); got != want {
			t.Errorf("OutputName(%q) = %q, want %q", id, got, want)
		}
	}
}

func TestOutputNamesNeverCollideWithToolchainFlags(t *testing.T) {
	// validator.validateCIFullBuildToolchains asserts on "needs_<lang>" flags.
	// A gate output that landed in that namespace would be read as a toolchain
	// flag by the validator.
	reg := loadRealRegistry(t)
	for _, id := range SortedGateIDs(reg) {
		if strings.HasPrefix(OutputName(id), "needs_") {
			t.Errorf("gate %q produces output %q, which collides with the toolchain-flag namespace", id, OutputName(id))
		}
	}
}

// ---------------------------------------------------------------------------
// Load validation
// ---------------------------------------------------------------------------

func TestLoadRejectsFutureSchemaVersion(t *testing.T) {
	path := writeRegistry(t, `{"schema_version": 99, "gates": {"a": {"description": "d", "paths": ["x"]}}}`)
	if _, err := Load(path); err == nil || !strings.Contains(err.Error(), "newer than this build tool") {
		t.Fatalf("want future-version rejection, got %v", err)
	}
}

func TestLoadRejectsEmptyRegistry(t *testing.T) {
	path := writeRegistry(t, `{"schema_version": 1, "gates": {}}`)
	if _, err := Load(path); err == nil || !strings.Contains(err.Error(), "no gates") {
		t.Fatalf("want empty-registry rejection, got %v", err)
	}
}

func TestLoadRejectsGateThatCanNeverFire(t *testing.T) {
	// A gate with neither packages nor paths is dead code that reports "skipped"
	// forever. Silence is the failure mode we are guarding against.
	path := writeRegistry(t, `{"schema_version": 1, "gates": {"a": {"description": "d"}}}`)
	if _, err := Load(path); err == nil || !strings.Contains(err.Error(), "could never fire") {
		t.Fatalf("want never-fires rejection, got %v", err)
	}
}

func TestLoadRejectsUndescribedGate(t *testing.T) {
	path := writeRegistry(t, `{"schema_version": 1, "gates": {"a": {"paths": ["x"]}}}`)
	if _, err := Load(path); err == nil || !strings.Contains(err.Error(), "no description") {
		t.Fatalf("want missing-description rejection, got %v", err)
	}
}

func TestLoadRejectsUnknownScope(t *testing.T) {
	path := writeRegistry(t, `{"schema_version": 1, "gates": {"a": {"scope": "workflow", "description": "d", "paths": ["x"]}}}`)
	if _, err := Load(path); err == nil || !strings.Contains(err.Error(), "unknown scope") {
		t.Fatalf("want unknown-scope rejection, got %v", err)
	}
}

func TestLoadRejectsUppercaseGateID(t *testing.T) {
	path := writeRegistry(t, `{"schema_version": 1, "gates": {"Alpha": {"description": "d", "paths": ["x"]}}}`)
	if _, err := Load(path); err == nil || !strings.Contains(err.Error(), "lowercase") {
		t.Fatalf("want gate-id rejection, got %v", err)
	}
}

func TestLoadDefaultsScopeToJob(t *testing.T) {
	path := writeRegistry(t, `{"schema_version": 1, "gates": {"a": {"description": "d", "paths": ["x"]}}}`)
	reg, err := Load(path)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if got := reg.Gates["a"].EffectiveScope(); got != ScopeJob {
		t.Errorf("EffectiveScope() = %q, want %q", got, ScopeJob)
	}
}

func TestSortedGateIDsIsStable(t *testing.T) {
	ids := SortedGateIDs(testRegistry())
	if !sort.StringsAreSorted(ids) {
		t.Errorf("SortedGateIDs returned unsorted ids: %v", ids)
	}
}

// ---------------------------------------------------------------------------
// The real registry, against real-shaped changes
// ---------------------------------------------------------------------------

func loadRealRegistry(t *testing.T) *Registry {
	t.Helper()
	reg, err := Load(realRegistryPath)
	if err != nil {
		t.Fatalf("load checked-in registry: %v", err)
	}
	return reg
}

func jobGates(reg *Registry) []string {
	var ids []string
	for _, id := range SortedGateIDs(reg) {
		if reg.Gates[id].EffectiveScope() == ScopeJob {
			ids = append(ids, id)
		}
	}
	return ids
}

// TestHumanLanguagesChangeSkipsEveryJobGate is the case that motivated this
// package: a curriculum pull request should not run a Ruby grammar regeneration
// or six-language crypto conformance.
func TestHumanLanguagesChangeSkipsEveryJobGate(t *testing.T) {
	reg := loadRealRegistry(t)
	affected := map[string]bool{
		"typescript/human-language-data":      true,
		"typescript/programs/language-ladder": true,
	}
	changed := []string{
		"code/learning/human-languages/hindi/lessons/ch22.md",
		"code/learning/human-languages/BACKLOG.d/hindi.md",
		"code/packages/typescript/human-language-data/src/hindi.ts",
		"code/packages/typescript/human-language-data/CHANGELOG.d/0042.md",
	}

	got := Evaluate(reg, affected, changed, false)
	for _, id := range jobGates(reg) {
		if got[id] {
			t.Errorf("gate %q fired on a human-languages change; it should skip", id)
		}
	}
}

func TestGrammarSourceChangeFiresRubyGrammarGate(t *testing.T) {
	reg := loadRealRegistry(t)
	// A .grammar edit maps to no package at all, so only the path clause can
	// fire this gate. That is the whole point of having one.
	got := Evaluate(reg, map[string]bool{}, []string{"code/grammars/lattice/lattice.grammar"}, false)

	if !got["ruby-grammar-regen-check"] {
		t.Error("ruby-grammar-regen-check must fire on a code/grammars change")
	}
	if got["d18f-message-conformance"] {
		t.Error("d18f-message-conformance should not fire on a grammar change")
	}
}

func TestD18FManifestChangeFiresOnlyD18F(t *testing.T) {
	reg := loadRealRegistry(t)
	got := Evaluate(reg, map[string]bool{}, []string{"code/fixtures/chief-of-staff-message/v1/manifest.json"}, false)

	if !got["d18f-message-conformance"] {
		t.Error("d18f-message-conformance must fire on its own manifest")
	}
	// D18F and D18Q declare the SAME six packages; only the fixture path tells
	// them apart. If this ever regresses, both will fire on every crypto change.
	if got["d18q-channel-key-grant-conformance"] {
		t.Error("d18q fired on the D18F manifest; the fixture path is their only discriminator")
	}
	if got["ruby-grammar-regen-check"] {
		t.Error("ruby-grammar-regen-check should not fire on a D18F manifest change")
	}
}

func TestCryptoPackageChangeFiresBothCryptoManifestGates(t *testing.T) {
	reg := loadRealRegistry(t)
	// Both D18F and D18Q run against chief-of-staff-channel-crypto, so a source
	// change in it must fire both. (In a real run the affected closure would
	// also pull in channel-endpoints and channel-store as dependents, firing
	// D18P and D18T too; this test pins the direct-match half.)
	affected := map[string]bool{"rust/chief-of-staff-channel-crypto": true}
	changed := []string{"code/packages/rust/chief-of-staff-channel-crypto/src/message.rs"}

	got := Evaluate(reg, affected, changed, false)
	for _, id := range []string{"d18f-message-conformance", "d18q-channel-key-grant-conformance"} {
		if !got[id] {
			t.Errorf("gate %q must fire on a chief-of-staff-channel-crypto change", id)
		}
	}
	if got["unicode17-swift-conformance"] {
		t.Error("unicode17-swift-conformance should not fire on a crypto change")
	}
}

func TestUnicodeGeneratorChangeFiresAllFiveUnicodeGates(t *testing.T) {
	reg := loadRealRegistry(t)
	// generate_tracked_artifact_unicode17.py renders and --checks EVERY language
	// target regardless of --self-check-runtime, so all five jobs share one
	// condition. Gating them differently would be wrong.
	got := Evaluate(reg, map[string]bool{}, []string{"code/scripts/generate_tracked_artifact_unicode17.py"}, false)

	for _, lang := range []string{"elixir", "lua", "perl", "haskell", "swift"} {
		id := "unicode17-" + lang + "-conformance"
		if !got[id] {
			t.Errorf("gate %q must fire when the Unicode 17 generator changes", id)
		}
	}
}

func TestRealRegistryDeclaresBothClausesForEveryJobGate(t *testing.T) {
	// A job gate with no path clause cannot see fixture, grammar, spec or script
	// edits, because those map to zero packages. Enforce the invariant rather
	// than trusting review to catch it.
	reg := loadRealRegistry(t)
	for _, id := range jobGates(reg) {
		gate := reg.Gates[id]
		if len(gate.Packages) == 0 {
			t.Errorf("job gate %q declares no packages", id)
		}
		if len(gate.Paths) == 0 {
			t.Errorf("job gate %q declares no paths; it cannot see fixture or script edits", id)
		}
	}
}
