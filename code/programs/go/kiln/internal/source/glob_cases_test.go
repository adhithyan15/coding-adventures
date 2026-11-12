package source

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// TestGlobCasesV1 checks the matcher against the shared equivalence fixture
// (KILN03 D4).  The same file is meant to be checked by the old build tool's
// matcher, so the two can never disagree about what a repo_input_globs
// pattern selects.
func TestGlobCasesV1(t *testing.T) {
	data, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "..", "specs", "fixtures", "kiln", "glob-cases-v1.json"))
	if err != nil {
		t.Fatal(err)
	}
	var f struct {
		SchemaVersion   int      `json:"schema_version"`
		Description     string   `json:"description"`
		InvalidPatterns []string `json:"invalid_patterns"`
		Cases           []struct {
			Pattern string `json:"pattern"`
			Path    string `json:"path"`
			Match   bool   `json:"match"`
		} `json:"cases"`
	}
	dec := json.NewDecoder(bytesReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&f); err != nil {
		t.Fatal(err)
	}
	if f.SchemaVersion != 1 || len(f.Cases) < 50 {
		t.Fatalf("fixture looks wrong: schema %d, %d cases", f.SchemaVersion, len(f.Cases))
	}
	for _, c := range f.Cases {
		if got := Match(c.Pattern, c.Path); got != c.Match {
			t.Errorf("Match(%q, %q) = %v, fixture says %v", c.Pattern, c.Path, got, c.Match)
		}
	}
	for _, p := range f.InvalidPatterns {
		if ValidatePattern(p) == nil {
			t.Errorf("ValidatePattern(%q) accepted a pattern the fixture calls invalid", p)
		}
	}
}
