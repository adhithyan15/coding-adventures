package main

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// WD-004b2: every Rust package that records lane rungs in its
// assurance_report.json must re-run when the consumer registry changes.
func TestLaneRegistryEditSelectsEveryAssuranceReportAdopter(t *testing.T) {
	const registry = "code/specs/fixtures/ct-compare-warrant-v1/consumers.json"
	adopters := []string{"arithmetic", "ct-compare", "gf256", "logic-gates", "lzw", "wasm-leb128"}
	var packages []discovery.Package
	for _, name := range append([]string{"ca-assurance-report", "warrant-grammar", "other"}, adopters...) {
		packages = append(packages, discovery.Package{Name: "rust/" + name, Language: "rust"})
	}
	for _, name := range []string{"python/warrant-corpus", "python/warrant-oracle", "python/ct-compare",
		"typescript/warrant-corpus", "typescript/ct-compare", "go/warrant-corpus", "go/ct-compare"} {
		lang := name[:len(name)-len(name[indexByte(name, '/'):])]
		packages = append(packages, discovery.Package{Name: name, Language: lang})
	}
	if !isWarrantLaneRegistryInput(registry) {
		t.Fatal("the registry must be a lane registry input")
	}
	got, err := repositoryInputConsumersForChange([]string{registry}, packages, "rust")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]bool{"rust/ca-assurance-report": true, "rust/warrant-grammar": true}
	for _, name := range adopters {
		want["rust/"+name] = true
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("registry edit selected %v, want %v", got, want)
	}
	if _, ok := got["rust/other"]; ok {
		t.Fatal("an unrelated package must not be selected")
	}
	// Lookalikes are not the registry.
	for _, input := range []string{
		registry + ".bak", "code/specs/fixtures/ct-compare-warrant-v1/consumers.schema.json",
		"code/specs/fixtures/ct-compare-warrant-v10/consumers.json", `code\specs\fixtures\ct-compare-warrant-v1\consumers.json`,
	} {
		if isWarrantLaneRegistryInput(input) {
			t.Errorf("lookalike selected: %s", input)
		}
	}
	// An adopter missing from discovery fails closed.
	if _, err := repositoryInputConsumersForChange([]string{registry}, packages[:3], "rust"); err == nil {
		t.Fatal("a missing adopter must fail closed")
	}
}

func indexByte(s string, c byte) int {
	for i := 0; i < len(s); i++ {
		if s[i] == c {
			return i
		}
	}
	return len(s)
}

// WD-004b2: the three lanes' ct-compare BUILD files run the Rust loader through
// cargo, so a change scoped to one lane must still ask CI for Rust.
func TestWarrantLanePilotsRequestRustWhenOnlyTheLaneChanges(t *testing.T) {
	root := findRepoRoot("")
	for _, lane := range []string{"python", "typescript", "go"} {
		pkgPath := "code/packages/" + lane + "/ct-compare"
		for _, platform := range []string{"linux", "darwin", "windows"} {
			fronts := make(map[string]string)
			for _, front := range []string{"BUILD", "BUILD_windows"} {
				raw, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(pkgPath), front))
				if err != nil {
					t.Fatal(err)
				}
				fronts[front] = string(raw)
			}
			pkg := discovery.Package{
				Name: lane + "/ct-compare", Language: lane,
				ExtraToolchains: discovery.ExtraToolchainsForSnapshot(fronts, platform),
			}
			needed := computeLanguagesNeeded([]discovery.Package{pkg}, map[string]bool{pkg.Name: true}, false, nil)
			if !needed[lane] || !needed["rust"] {
				t.Errorf("%s/%s: a lane-only change needs its own toolchain and Rust: %v", lane, platform, needed)
			}
		}
	}
}
