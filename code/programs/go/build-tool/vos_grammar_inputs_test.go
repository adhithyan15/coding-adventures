package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

// VOS-033 / VOS-D058: the grammar files the VOS crates embed with include_str!
// are not Cargo edges, so a row must select the consumers when they change.
func TestVosGrammarInputsSelectTheirConsumers(t *testing.T) {
	cases := []struct {
		path        string
		grammar, sh bool
	}{
		{"code/grammars/vos_contract/vos_contract.tokens", true, false},
		{"code/grammars/vos_contract/vos_contract.grammar", true, false},
		{"code/grammars/vos_document/vos_document.tokens", true, false},
		{"code/grammars/vos_contract2/x.tokens", false, false},
		{"code/grammars/vos_contract", false, false},
		{"code/grammars/toml/toml.tokens", false, false},
		{"code/grammars/rust/rust1.97.0-e2021.tokens", false, true},
		{"code/grammars/rust/rust1.97.0-e2024.tokens", false, true},
		{"code/grammars/rust/rust1.97.0-e2018.tokens", false, false},
		{"code\\grammars\\vos_contract\\x", false, false},
	}
	for _, c := range cases {
		if got := isVosGrammarInput(c.path); got != c.grammar {
			t.Errorf("isVosGrammarInput(%q) = %v", c.path, got)
		}
		if got := isVosShapeGrammarInput(c.path); got != c.sh {
			t.Errorf("isVosShapeGrammarInput(%q) = %v", c.path, got)
		}
	}
	want := map[string]bool{"rust/vos-manifest": false, "rust/programs/vos-link": false, "rust/vos-shape-check": false}
	for _, r := range repositoryInputConsumers {
		if _, ok := want[r.name]; ok {
			want[r.name] = true
			if r.language != "rust" || len(r.paths) == 0 {
				t.Errorf("%s: language %q paths %v", r.name, r.language, r.paths)
			}
		}
	}
	for n, ok := range want {
		if !ok {
			t.Errorf("no repository input row for %s", n)
		}
	}
}

// VOS-D066: the tools a VOS implementation's BUILD runs outside Cargo, and the implementation's
// own generated document, select that implementation.
func TestVosToolInputsSelectImplementations(t *testing.T) {
	var row *repositoryInputConsumer
	for i := range repositoryInputConsumers {
		if repositoryInputConsumers[i].name == "rust/vos-sched-fp-bitmap" {
			row = &repositoryInputConsumers[i]
		}
	}
	if row == nil || row.language != "rust" || len(row.paths) == 0 {
		t.Fatalf("no usable repository input row for rust/vos-sched-fp-bitmap: %+v", row)
	}
	for path, want := range map[string]bool{
		"code/programs/rust/vos-link/src/check.rs":       true,
		"code/programs/rust/vos-link/Cargo.toml":         true,
		"code/packages/rust/vos-manifest/src/part.rs":    true,
		"code/packages/rust/vos-shape-check/src/a.rs":    true,
		"code/packages/rust/warrant-grammar/src/a.rs":    true,
		"code/packages/rust/ca-capability-analyzer/a.rs": true,
		"code/packages/rust/lexer/src/a.rs":              true,
		"code/grammars/vos_document/vos_document.tokens": true,
		// The implementation's own generated document, and only its own.
		"code/specs/vos-sched-fp-bitmap.warrant":       true,
		"code/specs/vos-sched-any.warrant":             false,
		"code/specs/vos-sched-fp-bitmap.warrant.x":     false,
		"code/specs/sub/vos-sched-fp-bitmap.warrant":   false,
		"code/programs/rust/vos-link":                  false,
		"code/packages/rust/vos-manifest-x/src/a.rs":   false,
		"code/packages/rust/lexer-x/src/a.rs":          false,
		"code/programs/rust/vos-linker/src/a.rs":       false,
		"code/packages/rust/vos-sched-contract/a.part": false,
		"code\\programs\\rust\\vos-link\\src\\a.rs":    false,
	} {
		if got := row.matches(path); got != want {
			t.Errorf("rust/vos-sched-fp-bitmap matches(%q) = %v, want %v", path, got, want)
		}
	}
	other := vosImplementationRow("vos-sched-any")
	if !other.matches("code/specs/vos-sched-any.warrant") || other.matches("code/specs/vos-sched-fp-bitmap.warrant") {
		t.Error("a row selects its own document only")
	}
	// The registered row of the -any crate (VOS-012a): present, selects its own document and the
	// VOS tools, and nothing of its sibling.
	var anyRow *repositoryInputConsumer
	for i := range repositoryInputConsumers {
		if repositoryInputConsumers[i].name == "rust/vos-sched-any" {
			anyRow = &repositoryInputConsumers[i]
		}
	}
	if anyRow == nil || anyRow.language != "rust" || len(anyRow.paths) == 0 {
		t.Fatalf("no usable repository input row for rust/vos-sched-any: %+v", anyRow)
	}
	for path, want := range map[string]bool{
		"code/specs/vos-sched-any.warrant":          true,
		"code/specs/vos-sched-fp-bitmap.warrant":    false,
		"code/specs/vos-sched-any.warrant.x":        false,
		"code/programs/rust/vos-link/src/check.rs":  true,
		"code/packages/rust/vos-manifest/src/a.rs":  true,
		"code/packages/rust/vos-sched-any/src/a.rs": false,
	} {
		if got := anyRow.matches(path); got != want {
			t.Errorf("rust/vos-sched-any matches(%q) = %v, want %v", path, got, want)
		}
	}
}

// The registered row of the round-robin crate (VOS-012b): present, selects its own document and the
// VOS tools, and nothing of its siblings (the swap exercise: replacing one implementation
// regenerates only its own).
func TestVosRoundRobinRowSelectsOnlyItsOwnDocument(t *testing.T) {
	var rrRow *repositoryInputConsumer
	for i := range repositoryInputConsumers {
		if repositoryInputConsumers[i].name == "rust/vos-sched-rr" {
			rrRow = &repositoryInputConsumers[i]
		}
	}
	if rrRow == nil || rrRow.language != "rust" || len(rrRow.paths) == 0 {
		t.Fatalf("no usable repository input row for rust/vos-sched-rr: %+v", rrRow)
	}
	for path, want := range map[string]bool{
		"code/specs/vos-sched-rr.warrant":          true,
		"code/specs/vos-sched-fp-bitmap.warrant":   false,
		"code/specs/vos-sched-any.warrant":         false,
		"code/specs/vos-sched-rr.warrant.x":        false,
		"code/programs/rust/vos-link/src/check.rs": true,
		"code/packages/rust/vos-manifest/src/a.rs": true,
		"code/packages/rust/vos-sched-rr/src/a.rs": false,
	} {
		if got := rrRow.matches(path); got != want {
			t.Errorf("rust/vos-sched-rr matches(%q) = %v, want %v", path, got, want)
		}
	}
}

// The capacity probe (VOS-028b) and its fixture have rows: an edit to the committed results file,
// to the VOS30 spec or to the fixture's document selects the probe, and nothing unrelated does.
func TestVosCapacityProbeAndFixtureRows(t *testing.T) {
	rows := map[string]*repositoryInputConsumer{}
	for i := range repositoryInputConsumers {
		switch repositoryInputConsumers[i].name {
		case "rust/programs/vos-capacity-probe", "rust/vos-probe-ready":
			rows[repositoryInputConsumers[i].name] = &repositoryInputConsumers[i]
		}
	}
	probe, fixture := rows["rust/programs/vos-capacity-probe"], rows["rust/vos-probe-ready"]
	if probe == nil || fixture == nil || probe.language != "rust" || fixture.language != "rust" || len(probe.paths) == 0 {
		t.Fatalf("missing rows: probe %+v fixture %+v", probe, fixture)
	}
	for path, want := range map[string]bool{
		"code/specs/data/vos30-capacity-results.json":    true,
		"code/specs/data/vos30-capacity-results.json.x":  false,
		"code/specs/VOS30-proof-route-and-capacity.md":   true,
		"code/specs/VOS30-proof-route-and-capacity.mdx":  false,
		"code/specs/vos-probe-ready.warrant":             true,
		"code/specs/vos-sched-rr.warrant":                false,
		"code/specs/data/other.json":                     false,
		"code/programs/rust/vos-capacity-probe/src/a.rs": false,
	} {
		if got := probe.matches(path); got != want {
			t.Errorf("rust/programs/vos-capacity-probe matches(%q) = %v, want %v", path, got, want)
		}
	}
	for _, p := range probe.paths {
		if !probe.matches(p) {
			t.Errorf("declared path %s is not matched by its own predicate", p)
		}
	}
	for path, want := range map[string]bool{
		"code/specs/vos-probe-ready.warrant":          true,
		"code/specs/vos-sched-rr.warrant":             false,
		"code/packages/rust/warrant-grammar/src/a.rs": true,
		"code/packages/rust/vos-probe-ready/src/a.rs": false,
	} {
		if got := fixture.matches(path); got != want {
			t.Errorf("rust/vos-probe-ready matches(%q) = %v, want %v", path, got, want)
		}
	}
}

// Every directory a VOS tool path names exists, so a rename cannot silently orphan the row.
func TestVosToolPathsExist(t *testing.T) {
	for _, p := range vosToolPaths {
		if _, err := os.Stat(filepath.Join("..", "..", "..", "..", p)); err != nil {
			t.Errorf("vosToolPaths: %s: %v", p, err)
		}
	}
}

// VOS-040c: the model-SAT lane's directories exist, so a rename cannot orphan the contract row.
func TestVosContractSatPathsExist(t *testing.T) {
	if len(vosContractSatPaths) != 4 {
		t.Fatalf("vosContractSatPaths = %v, want the lane and its three solver crates", vosContractSatPaths)
	}
	for _, p := range vosContractSatPaths {
		if _, err := os.Stat(filepath.Join("..", "..", "..", "..", p)); err != nil {
			t.Errorf("vosContractSatPaths: %s: %v", p, err)
		}
		for _, q := range vosToolPaths {
			if p == q {
				t.Errorf("%s is in both vosToolPaths and vosContractSatPaths: the SAT paths must stay contract-only", p)
			}
		}
	}
}

// The registered row of a contract crate (VOS-039b-1): present, selects the VOS tools and
// grammars its BUILD runs outside Cargo, and nothing else (no document of its own, no lookalike).
func TestVosContractRowSelectsTheVosTools(t *testing.T) {
	var row *repositoryInputConsumer
	for i := range repositoryInputConsumers {
		if repositoryInputConsumers[i].name == "rust/vos-sched-contract" {
			row = &repositoryInputConsumers[i]
		}
	}
	if row == nil || row.language != "rust" || len(row.paths) == 0 {
		t.Fatalf("no usable repository input row for rust/vos-sched-contract: %+v", row)
	}
	for path, want := range map[string]bool{
		"code/programs/rust/vos-link/src/contract_check.rs": true,
		"code/packages/rust/vos-manifest/src/warrant.rs":    true,
		"code/packages/rust/vos-shape-check/src/a.rs":       true,
		"code/packages/rust/warrant-grammar/src/a.rs":       true,
		// VOS-040c: the model-SAT lane and the solver crates it runs with select the contract
		// row (they are not Cargo edges of the crate), as lookalikes and neighbours do not.
		"code/programs/rust/vos-model-sat/src/lane.rs":  true,
		"code/programs/rust/vos-model-sat/Cargo.toml":   true,
		"code/packages/rust/bit-blaster/src/aig.rs":     true,
		"code/packages/rust/cdcl-sat/src/lib.rs":        true,
		"code/packages/rust/drat-check/src/lib.rs":      true,
		"code/packages/rust/cdcl-sat-extra/src/lib.rs":  false,
		"code/packages/rust/bit-blaster2/src/lib.rs":    false,
		"code/programs/rust/vos-model-sat-x/src/lib.rs": false,
		"code/specs/vos-sched-any.warrant":              false,
		"code/specs/vos-sched-contract.warrant":         false,
		"code/packages/rust/vos-manifest-x/src/a.rs":    false,
		"code/programs/rust/vos-linker/src/a.rs":        false,
	} {
		if got := row.matches(path); got != want {
			t.Errorf("rust/vos-sched-contract matches(%q) = %v, want %v", path, got, want)
		}
	}
}

// VOS-040c: the SAT paths select ONLY the contract row.  Every implementation row and the
// vos-probe-ready fixture row must ignore a solver edit, or each one would be re-run for it.
func TestVosContractSatPathsSelectNoImplementationRow(t *testing.T) {
	probes := []string{
		"code/programs/rust/vos-model-sat/src/lane.rs",
		"code/packages/rust/bit-blaster/src/aig.rs",
		"code/packages/rust/cdcl-sat/src/lib.rs",
		"code/packages/rust/drat-check/src/lib.rs",
	}
	checked := 0
	for _, row := range repositoryInputConsumers {
		if !strings.HasPrefix(row.name, "rust/vos-") || row.name == "rust/vos-sched-contract" {
			continue
		}
		if strings.HasPrefix(row.name, "rust/vos-capacity-probe") || row.name == "rust/vos-link" {
			continue // their own rows: not VOS implementation rows
		}
		for _, p := range probes {
			if row.matches != nil && row.matches(p) {
				t.Errorf("%s matches the SAT-lane path %s; it runs no SAT line", row.name, p)
			}
		}
		checked++
	}
	if checked == 0 {
		t.Fatal("no VOS implementation row was checked")
	}
}

// VOS-D104: `vos-link check-publish` reads the Cargo.toml and vos.toml of every direct `vos-*` child
// of the two Rust roots in the vos-link BUILD, so an edit to one must select vos-link.  The row
// declares the files as globs; they must agree with the predicate, nested and lookalike paths
// included.
func TestVosPublishInputsSelectVosLinkAndGlobsAgree(t *testing.T) {
	cases := []struct {
		path string
		want bool
	}{
		{"code/packages/rust/vos-manifest/Cargo.toml", true},
		{"code/packages/rust/vos-manifest/vos.toml", true},
		{"code/programs/rust/vos-capacity-probe/Cargo.toml", true},
		{"code/programs/rust/vos-link/vos.toml", true},
		{"code/packages/rust/vos-new-crate/Cargo.toml", true},
		{"code/packages/rust/vos-/Cargo.toml", true},
		{"code/packages/rust/vos-manifest/src/Cargo.toml", false},
		{"code/packages/rust/vos-manifest/tests/fixtures/vos.toml", false},
		{"code/packages/rust/vosx/Cargo.toml", false},
		{"code/packages/rust/vos/Cargo.toml", false},
		{"code/packages/rust/xvos-a/Cargo.toml", false},
		{"code/packages/rust/vos-manifest/Cargo.lock", false},
		{"code/packages/rust/vos-manifest/Cargo.toml.bak", false},
		{"code/packages/python/vos-a/Cargo.toml", false},
		{"code/programs/go/vos-a/vos.toml", false},
		{"code/specs/vos-a/Cargo.toml", false},
		{"code/packages/rust/Cargo.toml", false},
		{"packages/rust/vos-a/Cargo.toml", false},
		{"code\\packages\\rust\\vos-a\\Cargo.toml", false},
	}
	var row *repositoryInputConsumer
	for i := range repositoryInputConsumers {
		if repositoryInputConsumers[i].name == "rust/programs/vos-link" {
			row = &repositoryInputConsumers[i]
		}
	}
	if row == nil || len(row.globs) == 0 {
		t.Fatalf("no vos-link row with globs: %+v", row)
	}
	for _, c := range cases {
		if got := isVosPublishInput(c.path); got != c.want {
			t.Errorf("isVosPublishInput(%q) = %v, want %v", c.path, got, c.want)
		}
		if got := row.matches(c.path); got != c.want {
			t.Errorf("vos-link row matches(%q) = %v, want %v", c.path, got, c.want)
		}
		globHit := false
		for _, g := range vosPublishGlobs {
			if globmatch.MatchPath(g, c.path) {
				globHit = true
			}
		}
		if globHit != c.want {
			t.Errorf("globs on %q = %v, want %v", c.path, globHit, c.want)
		}
	}
	// The row keeps its grammar inputs.
	if !row.matches("code/grammars/vos_contract/vos_contract.tokens") {
		t.Error("the vos-link row lost its grammar inputs")
	}
}
