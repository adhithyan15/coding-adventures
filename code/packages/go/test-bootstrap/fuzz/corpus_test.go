package fuzz

import (
	"os"
	"path/filepath"
	"testing"
)

// repoFixture walks up from the test's working directory to the repository root
// (the directory holding code/specs/fixtures/fuzz-v1/cases.json) and reads the
// committed fixture.  There is no copy: the test reads the real file.
func repoFixture(t testing.TB) []byte {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		path := filepath.Join(dir, "code", "specs", "fixtures", "fuzz-v1", "cases.json")
		if data, err := os.ReadFile(path); err == nil {
			return data
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find code/specs/fixtures/fuzz-v1/cases.json above the working directory")
		}
		dir = parent
	}
}

func loadCases(t testing.TB) []any {
	t.Helper()
	cases, err := ParseCorpus(repoFixture(t))
	if err != nil {
		t.Fatal(err)
	}
	return cases
}

func TestCorpusConformsToAllTwentyFiveCases(t *testing.T) {
	report := EvaluateCorpusReport(loadCases(t), FuzzV1CaseCount)
	for _, p := range report.Problems {
		t.Error(p)
	}
	if report.Evaluated != 25 {
		t.Fatalf("evaluated %d cases, want exactly 25", report.Evaluated)
	}
	if got := EvaluateCorpus(loadCases(t)); len(got) != 0 {
		t.Fatalf("EvaluateCorpus reported %v", got)
	}
}
