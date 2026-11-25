package main

import (
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"runtime"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

// CV2-F13: a spec-only edit to `code/specs/<crate>.warrant` must select the Rust crate whose
// BUILD runs `gen_evidence --check <crate>` against it, together with the consumers the spec
// already selected (the grammar guard for every document, the oracle and the generator for the
// four corpus-v2 adopters, the lane registry rows).  The crate lives in `code/packages/rust`,
// the spec in `code/specs`, so path-prefix mapping alone selects neither.
func TestWarrantSpecEditSelectsTheCrateThatChecksEvidenceAgainstIt(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/warrant-grammar", Language: "rust"},
		{Name: "rust/warrant-corpus-gen", Language: "rust"},
		{Name: "python/warrant-oracle", Language: "python"},
		// the ct-compare spec also feeds the shared lane artifacts
		{Name: "rust/ca-assurance-report", Language: "rust"},
		{Name: "python/warrant-corpus", Language: "python"},
		{Name: "python/ct-compare", Language: "python"},
		{Name: "typescript/warrant-corpus", Language: "typescript"},
		{Name: "typescript/ct-compare", Language: "typescript"},
		{Name: "go/warrant-corpus", Language: "go"},
		{Name: "go/ct-compare", Language: "go"},
	}
	crates := warrantEvidenceRowCrates()
	if len(crates) == 0 {
		t.Fatal("no warrantEvidenceRow rows")
	}
	for _, crate := range crates {
		packages = append(packages, discovery.Package{Name: "rust/" + crate, Language: "rust"})
	}
	adopters := map[string]bool{}
	for _, adopter := range warrantAdopters {
		adopters[adopter.spec] = true
	}
	for _, crate := range crates {
		spec := "code/specs/" + crate + ".warrant"
		want := map[string]bool{"rust/" + crate: true, "rust/warrant-grammar": true}
		if adopters[spec] {
			want["rust/warrant-corpus-gen"] = true
		}
		if isWarrantOracleInput(spec) {
			want["python/warrant-oracle"] = true
		}
		got, err := repositoryInputConsumersForChange([]string{spec}, packages, "all")
		if crate == "ct-compare" {
			// the spec is also a lane artifact: it selects these lane runners too
			for _, lane := range []string{"rust/ca-assurance-report", "python/warrant-corpus", "python/ct-compare",
				"typescript/warrant-corpus", "typescript/ct-compare", "go/warrant-corpus", "go/ct-compare"} {
				want[lane] = true
			}
		}
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("%s selected %v, want %v (err %v)", spec, got, want, err)
		}
		// Rust-only runs still select it; a non-Rust run does not.
		if got, err := repositoryInputConsumersForChange([]string{spec}, packages, "rust"); err != nil || !got["rust/"+crate] {
			t.Errorf("%s rust-only selected %v (err %v)", spec, got, err)
		}
		if got, err := repositoryInputConsumersForChange([]string{spec}, packages, "python"); err != nil || got["rust/"+crate] {
			t.Errorf("%s python-only selected %v (err %v)", spec, got, err)
		}
	}
	// The four corpus-v2 adopters are all covered.
	for _, adopter := range warrantAdopters {
		if adopter.bindings == "" {
			// logic-gates-basic (LGE-S1c) is not a Rust evidence crate; logicGatesBasicRow covers it.
			continue
		}
		found := false
		for _, crate := range crates {
			found = found || adopter.module == crate
		}
		if !found {
			t.Errorf("adopter %s has no warrantEvidenceRow", adopter.module)
		}
	}
	// Lookalikes select none of these crates: another crate's spec, a suffix, a directory
	// spelling, a draft, a different directory, and a Windows spelling.
	for _, changed := range []string{
		"code/specs/lzw2.warrant",
		"code/specs/lzw.warrant.draft",
		"code/specs/lzw.warrant/x",
		"code/specs/fixtures/lzw.warrant",
		"code/specs/lz.warrant",
		"code/specs/LZW.warrant",
		"docs/code/specs/lzw.warrant",
		`code\specs\lzw.warrant`,
		"code/specs/x25519.warrant.bak",
		"code/specs/vos-sched-rr2.warrant",
	} {
		got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all")
		if err != nil {
			t.Fatalf("%s: %v", changed, err)
		}
		for _, crate := range crates {
			if got["rust/"+crate] {
				t.Errorf("lookalike %s selected rust/%s", changed, crate)
			}
		}
	}
	// A crate missing from discovery is a hard error, like every other consumer.
	if _, err := repositoryInputConsumersForChange([]string{"code/specs/lzw.warrant"}, packages[:3], "all"); err == nil || !strings.Contains(err.Error(), "rust/lzw") {
		t.Errorf("a missing rust/lzw must fail closed naming it, got %v", err)
	}
	// Each row's declared paths are exactly its predicate (the Kiln export keys on them).
	for _, crate := range crates {
		row := warrantEvidenceRow(crate)
		if !reflect.DeepEqual(row.paths, []string{"code/specs/" + crate + ".warrant"}) || !row.matches(row.paths[0]) || row.matches(row.paths[0]+"x") {
			t.Errorf("row %s: paths %v do not match its predicate", row.name, row.paths)
		}
	}
}

// warrantEvidenceRowCrates lists the crate directory of every warrantEvidenceRow row.
func warrantEvidenceRowCrates() []string {
	var crates []string
	for _, row := range repositoryInputConsumers {
		if row.language == "rust" && strings.HasPrefix(row.inputs, warrantEvidenceInputsPrefix) {
			crates = append(crates, strings.TrimPrefix(row.name, "rust/"))
		}
	}
	return crates
}

// Every Rust crate that checks committed evidence against a spec must have a row that selects
// it when that spec changes: a new adopter that forgets its row recreates CV2-F13.  A crate
// is an adopter when its directory has an evidence.bindings or any BUILD* line runs
// `gen_evidence ... --check` (argument order does not matter).  The VOS crates are covered by
// vosImplementationRow rows and must satisfy the same test.  Reverse check: every
// warrantEvidenceRow row whose crate directory exists in this checkout is an adopter, so a
// stale entry fails.  Reads whatever this checkout has (a sparse checkout sees fewer).
func TestWarrantEvidenceRowsCoverEveryEvidenceCrate(t *testing.T) {
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not locate test source")
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(sourceFile), "..", "..", "..", ".."))
	rustRoot := filepath.Join(repoRoot, "code", "packages", "rust")
	check := regexp.MustCompile(`gen_evidence\b[^\n]*--check`)
	covered := map[string]bool{}
	scanned := 0
	bindings, err := filepath.Glob(filepath.Join(rustRoot, "*", "evidence.bindings"))
	if err != nil {
		t.Fatal(err)
	}
	for _, file := range bindings {
		scanned++
		covered[filepath.Base(filepath.Dir(file))] = true
	}
	builds, err := filepath.Glob(filepath.Join(rustRoot, "*", "BUILD*"))
	if err != nil {
		t.Fatal(err)
	}
	for _, file := range builds {
		data, err := os.ReadFile(file)
		if err != nil {
			t.Errorf("read %s: %v", file, err)
			continue
		}
		scanned++
		for _, line := range strings.Split(string(data), "\n") {
			if !strings.HasPrefix(strings.TrimSpace(line), "#") && check.MatchString(line) {
				covered[filepath.Base(filepath.Dir(file))] = true
			}
		}
	}
	t.Logf("scanned %d BUILD and evidence.bindings files, %d evidence crates", scanned, len(covered))
	for dir := range covered {
		spec := "code/specs/" + dir + ".warrant"
		found := false
		for _, row := range repositoryInputConsumers {
			if row.name == "rust/"+dir && row.language == "rust" && row.matches(spec) {
				found = true
			}
		}
		if !found {
			t.Errorf("rust/%s checks committed evidence but no repository input row selects it when %s changes (add warrantEvidenceRow(%q))", dir, spec, dir)
		}
	}
	for _, dir := range warrantEvidenceRowCrates() {
		if _, err := os.Stat(filepath.Join(rustRoot, dir)); err == nil && !covered[dir] {
			t.Errorf("warrantEvidenceRow(%q) is stale: code/packages/rust/%s has no evidence.bindings and no gen_evidence --check", dir, dir)
		}
	}
}

// The build tool's own tests read every crate's evidence.bindings, so adding one selects them.
func TestRustEvidenceBindingsSelectTheBuildTool(t *testing.T) {
	packages := []discovery.Package{
		{Name: "go/programs/build-tool", Language: "go"},
		{Name: "python/foo", Language: "python"},
	}
	const want = "go/programs/build-tool"
	for _, changed := range []string{"code/packages/rust/foo/evidence.bindings", "code/packages/rust/vos-x/evidence.bindings"} {
		for _, lang := range []string{"all", "go"} {
			if got, err := repositoryInputConsumersForChange([]string{changed}, packages, lang); err != nil || !got[want] {
				t.Errorf("%s (%s) selected %v, err %v; want %s", changed, lang, got, err, want)
			}
		}
		for _, lang := range []string{"rust", "python"} {
			if got, err := repositoryInputConsumersForChange([]string{changed}, packages, lang); err != nil || got[want] {
				t.Errorf("%s (%s) selected %v, err %v; want no build-tool", changed, lang, got, err)
			}
		}
		if !globmatch.MatchPath(rustEvidenceBindingsGlob, changed) {
			t.Errorf("glob does not match %s", changed)
		}
	}
	for _, changed := range []string{
		"code/packages/rust/foo/src/evidence.bindings",
		"code/packages/rust/foo/evidence.bindings.bak",
		"code/packages/rust/foo/evidence.bindings/x",
		"code/packages/rust/evidence.bindings",
		"code/packages/rust/../evidence.bindings",
		"code/packages/rust/foo/BUILD",
		"code/packages/python/foo/evidence.bindings",
		"code/programs/rust/foo/evidence.bindings",
		`code\packages\rust\foo\evidence.bindings`,
		`code/packages/rust/fo\o/evidence.bindings`,
	} {
		if isRustEvidenceBindings(changed) {
			t.Errorf("predicate accepted %q", changed)
		}
		if got, err := repositoryInputConsumersForChange([]string{changed}, packages, "all"); err != nil || got[want] {
			t.Errorf("%q selected %v, err %v; want no build-tool", changed, got, err)
		}
		// The glob may be wider than the predicate (it also spells `..` and a backslash as a
		// segment); it only has to cover everything the predicate accepts.
		if strings.Contains(changed, "..") || strings.Contains(changed, `\`) {
			continue
		}
		if globmatch.MatchPath(rustEvidenceBindingsGlob, changed) {
			t.Errorf("glob matched %q", changed)
		}
	}
}
