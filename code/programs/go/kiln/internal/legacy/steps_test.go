package legacy

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// okSteps is exactly the shape the build tool's -emit-kiln-steps writes
// (two-space indented, trailing newline).
const okSteps = `{
  "schema": "kiln-legacy-steps/1",
  "os": "linux",
  "packages": [
    {
      "package": "code/packages/go/a",
      "steps": [
        "go build ./...",
        "go test ./... -cover"
      ]
    },
    {
      "package": "code/packages/go/b",
      "steps": []
    }
  ]
}
`

func TestDecodeStepsOK(t *testing.T) {
	s, err := DecodeSteps([]byte(okSteps))
	if err != nil {
		t.Fatal(err)
	}
	got, ok := s.Lines("code/packages/go/a")
	if !ok || !reflect.DeepEqual(got, []string{"go build ./...", "go test ./... -cover"}) {
		t.Fatalf("Lines(a) = %q, %v", got, ok)
	}
	// A package with no commands is present, with an empty (non-nil) list.
	got, ok = s.Lines("code/packages/go/b")
	if !ok || got == nil || len(got) != 0 {
		t.Fatalf("Lines(b) = %#v, %v", got, ok)
	}
	if _, ok := s.Lines("code/packages/go/c"); ok {
		t.Fatal("an absent package must not be found")
	}
	if _, ok := s.Lines(""); ok {
		t.Fatal("the empty path must not be found")
	}
	// The returned slice is a copy: editing it must not change the export.
	got, _ = s.Lines("code/packages/go/a")
	got[0] = "rm -rf /"
	if again, _ := s.Lines("code/packages/go/a"); again[0] != "go build ./..." {
		t.Fatal("Lines returned the export's own slice")
	}
}

func TestDecodeStepsRejects(t *testing.T) {
	mut := func(from, to string) string {
		if !strings.Contains(okSteps, from) {
			t.Fatalf("test bug: %q not in the export", from)
		}
		return strings.Replace(okSteps, from, to, 1)
	}
	big := strings.Repeat("x", MaxStepLineSize+1)
	many := `"x"` + strings.Repeat(`,"x"`, MaxStepsPerPkg)
	bad := map[string]string{
		"not json":           "nope",
		"empty":              "",
		"trailing data":      okSteps + " {}",
		"unknown key":        mut(`"os": "linux"`, `"os": "linux", "extra": 1`),
		"duplicate key":      mut(`"os": "linux"`, `"os": "linux", "os": "darwin"`),
		"case variant key":   mut(`"os"`, `"OS"`),
		"wrong schema":       mut("kiln-legacy-steps/1", "kiln-legacy-steps/2"),
		"graph schema":       mut("kiln-legacy-steps/1", "kiln-legacy-graph/1"),
		"unknown os":         mut(`"linux"`, `"plan9"`),
		"empty os":           mut(`"linux"`, `""`),
		"null packages":      `{"schema":"kiln-legacy-steps/1","os":"linux","packages":null}`,
		"missing packages":   `{"schema":"kiln-legacy-steps/1","os":"linux"}`,
		"null steps":         mut(`"steps": []`, `"steps": null`),
		"missing steps":      mut(`"steps": []`, `"x": 1`),
		"unsorted":           mut("code/packages/go/b", "code/packages/go/0"),
		"repeated":           mut("code/packages/go/b", "code/packages/go/a"),
		"relative escape":    mut("code/packages/go/b", "code/../go/b"),
		"absolute":           mut("code/packages/go/b", "/code/packages/go/b"),
		"trailing slash":     mut("code/packages/go/b", "code/packages/go/b/"),
		"empty package":      mut("code/packages/go/b", ""),
		"step is a number":   mut(`"go build ./..."`, `7`),
		"step has NUL":       mut(`"go build ./..."`, `"go\u0000build"`),
		"step too long":      mut(`"go build ./..."`, `"`+big+`"`),
		"too many steps":     mut(`"steps": []`, `"steps": [`+many+`]`),
		"steps not a list":   mut(`"steps": []`, `"steps": "go build"`),
		"package not text":   mut(`"code/packages/go/b"`, `5`),
		"packages an object": mut(`"packages": [`, `"packages": {"a":[`),
	}
	for name, doc := range bad {
		if _, err := DecodeSteps([]byte(doc)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
}

func TestDecodeStepsSizeLimits(t *testing.T) {
	if _, err := DecodeSteps(make([]byte, MaxStepsBytes+1)); err == nil || !strings.Contains(err.Error(), "limit") {
		t.Fatalf("over-size input: %v", err)
	}
	saved := packageLimit
	packageLimit = 1
	defer func() { packageLimit = saved }()
	if _, err := DecodeSteps([]byte(okSteps)); err == nil || !strings.Contains(err.Error(), "limit 1") {
		t.Fatalf("package limit: %v", err)
	}
}

func TestReadStepsFile(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "steps.json")
	if err := os.WriteFile(path, []byte(okSteps), 0o644); err != nil {
		t.Fatal(err)
	}
	s, err := ReadStepsFile(path)
	if err != nil || len(s.Packages) != 2 {
		t.Fatalf("%v %+v", err, s)
	}
	if _, err := ReadStepsFile(filepath.Join(dir, "absent.json")); err == nil {
		t.Fatal("a missing file must fail")
	}
	if _, err := ReadStepsFile(dir); err == nil {
		t.Fatal("a directory must fail")
	}
	bad := filepath.Join(dir, "bad.json")
	if err := os.WriteFile(bad, []byte(`{"schema":"x"}`), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadStepsFile(bad); err == nil {
		t.Fatal("a bad document must fail")
	}
}

func TestRecordTmpdirIsAbsolute(t *testing.T) {
	if !strings.HasPrefix(RecordTmpdir, "/") {
		t.Fatal(RecordTmpdir)
	}
}
