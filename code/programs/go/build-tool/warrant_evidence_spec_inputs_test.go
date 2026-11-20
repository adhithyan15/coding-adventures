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
	for _, crate := range warrantEvidenceCrates {
		packages = append(packages, discovery.Package{Name: "rust/" + crate, Language: "rust"})
	}
	adopters := map[string]bool{}
	for _, adopter := range warrantAdopters {
		adopters[adopter.spec] = true
	}
	for _, crate := range warrantEvidenceCrates {
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
			// the spec is also a lane artifact: it selects the lane runners too, so require
			// only that nothing wanted is missing
			for name := range want {
				if !got[name] {
					t.Errorf("%s did not select %s (got %v)", spec, name, got)
				}
			}
			got = want
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
		found := false
		for _, crate := range warrantEvidenceCrates {
			found = found || adopter.module == crate
		}
		if !found {
			t.Errorf("adopter %s is not a warrantEvidenceCrates entry", adopter.module)
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
		for _, crate := range warrantEvidenceCrates {
			if got["rust/"+crate] {
				t.Errorf("lookalike %s selected rust/%s", changed, crate)
			}
		}
	}
	// A crate missing from discovery is a hard error, like every other consumer.
	if _, err := repositoryInputConsumersForChange([]string{"code/specs/lzw.warrant"}, packages[:3], "all"); err == nil {
		t.Error("a missing rust/lzw must fail closed")
	}
	// Each row's declared paths are exactly its predicate (the Kiln export keys on them).
	for _, crate := range warrantEvidenceCrates {
		row := warrantEvidenceRow(crate)
		if !reflect.DeepEqual(row.paths, []string{"code/specs/" + crate + ".warrant"}) || !row.matches(row.paths[0]) || row.matches(row.paths[0]+"x") {
			t.Errorf("row %s: paths %v do not match its predicate", row.name, row.paths)
		}
	}
}

// The table must list every non-VOS crate whose BUILD runs `gen_evidence --check <name>`: a new
// adopter that forgets its row recreates CV2-F13.  Reads whatever BUILD files this checkout
// has (a sparse checkout sees fewer, a full one all); the VOS crates are covered by
// vosImplementationRow and pinned by their own test.
func TestWarrantEvidenceCratesCoverEveryGenEvidenceBuild(t *testing.T) {
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not locate test source")
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(sourceFile), "..", "..", "..", ".."))
	check := regexp.MustCompile(`gen_evidence\s+--\s+--check\s+([A-Za-z0-9_-]+)`)
	listed := map[string]bool{}
	for _, crate := range warrantEvidenceCrates {
		listed[crate] = true
	}
	vos := map[string]bool{}
	for _, row := range repositoryInputConsumers {
		vos[row.name] = true
	}
	matches, err := filepath.Glob(filepath.Join(repoRoot, "code", "packages", "rust", "*", "BUILD*"))
	if err != nil {
		t.Fatal(err)
	}
	for _, file := range matches {
		data, err := os.ReadFile(file)
		if err != nil {
			continue
		}
		dir := filepath.Base(filepath.Dir(file))
		for _, m := range check.FindAllStringSubmatch(string(data), -1) {
			if m[1] != dir {
				t.Errorf("%s checks evidence for %q, not its own directory", file, m[1])
			}
			if !listed[dir] && !strings.HasPrefix(dir, "vos-") {
				t.Errorf("%s runs gen_evidence --check %s but %q is not in warrantEvidenceCrates", file, dir, dir)
			}
		}
	}
	for _, crate := range warrantEvidenceCrates {
		if !vos["rust/"+crate] {
			t.Errorf("rust/%s has no repository input row", crate)
		}
	}
}
