package pbt

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// repoFixture walks up from the test's working directory to the repository
// root (the directory holding code/specs/fixtures/pbt-v1/cases.json) and reads
// the committed fixture.  There is no copy: the test reads the real file.
func repoFixture(t testing.TB) []byte {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		path := filepath.Join(dir, "code", "specs", "fixtures", "pbt-v1", "cases.json")
		if data, err := os.ReadFile(path); err == nil {
			return data
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not find code/specs/fixtures/pbt-v1/cases.json above the working directory")
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
	report := EvaluateCorpusReport(loadCases(t), PBTV1CaseCount)
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

func TestParseCorpusIsStrict(t *testing.T) {
	bad := map[string]string{
		"not json":         `nope`,
		"empty":            ``,
		"trailing data":    `{"schema_version":1,"cases":[]} {}`,
		"wrong version":    `{"schema_version":2,"cases":[]}`,
		"string version":   `{"schema_version":"1","cases":[]}`,
		"missing cases":    `{"schema_version":1}`,
		"extra key":        `{"schema_version":1,"cases":[],"x":1}`,
		"cases not array":  `{"schema_version":1,"cases":{}}`,
		"not an object":    `[1]`,
		"duplicate key":    `{"schema_version":1,"cases":[],"cases":[]}`,
		"nested duplicate": `{"schema_version":1,"cases":[{"id":"a","id":"b"}]}`,
		"truncated":        `{"schema_version":1,"cases":[`,
		"deep nesting":     `{"schema_version":1,"cases":` + strings.Repeat("[", 100) + strings.Repeat("]", 100) + `}`,
		"unterminated obj": `{"schema_version":1,"cases":[{"id":"a"`,
		"mismatched close": `{"schema_version":1,"cases":[}`,
		"oversized":        `{"schema_version":1,"cases":[],"pad":"` + strings.Repeat("a", corpusMaxBytes) + `"}`,
	}
	for name, doc := range bad {
		if _, err := ParseCorpus([]byte(doc)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	cases, err := ParseCorpus([]byte(`{"schema_version":1,"cases":[1,"a",null,true,{"x":[]}]}`))
	if err != nil || len(cases) != 5 {
		t.Fatalf("reader must leave case-shape judgement to the evaluator: %v %v", cases, err)
	}
}
