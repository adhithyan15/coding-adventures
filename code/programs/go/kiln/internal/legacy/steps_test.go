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

// Two different recipes must never decode to the same lines: the JSON decoder
// turns an invalid byte, or a lone surrogate escape, into U+FFFD.
func TestPoCDecodeLossy(t *testing.T) {
	head := `{"schema":"kiln-legacy-steps/1","os":"linux","packages":[`
	for name, doc := range map[string]string{
		"raw invalid byte":     head + "{\"package\":\"a\",\"steps\":[\"x\xffy\"]}]}",
		"lone surrogate":       head + `{"package":"a","steps":["x\ud800y"]}]}`,
		"literal U+FFFD":       head + `{"package":"a","steps":["x` + "�" + `y"]}]}`,
		"escaped U+FFFD":       head + `{"package":"a","steps":["x�y"]}]}`,
		"empty line":           head + `{"package":"a","steps":[""]}]}`,
		"null step":            head + `{"package":"a","steps":[null]}]}`,
		"null package":         head + `null]}`,
		"null steps":           head + `{"package":"a","steps":null}]}`,
		"invalid byte in name": head + "{\"package\":\"a\xff\",\"steps\":[]}]}",
	} {
		if _, err := DecodeSteps([]byte(doc)); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// A valid non-ASCII line is fine and round-trips exactly.
	s, err := DecodeSteps([]byte(head + `{"package":"a","steps":["echo é ✓"]}]}`))
	if err != nil || s.Packages[0].Steps[0] != "echo é ✓" {
		t.Fatalf("%v %v", s, err)
	}
}

// Deep nesting is refused at the ninth level, so a megabyte of '[' costs a few
// tokens, not a megabyte of decoder state. The check is on the count of tokens
// consumed (bounded by the depth limit), not on a clock.
func TestDecodeStepsDepthLimit(t *testing.T) {
	deep := strings.Repeat("[", 1<<20)
	if _, err := DecodeSteps([]byte(deep)); err == nil || !strings.Contains(err.Error(), "deeper than") {
		t.Fatalf("got %v", err)
	}
	// Exactly the limit passes this pre-pass (the typed decode rejects the
	// truncated document afterwards).
	if err := checkTokens([]byte(strings.Repeat("[", MaxStepsDepth))); err != nil {
		t.Fatalf("depth %d must pass the depth check: %v", MaxStepsDepth, err)
	}
	if err := checkTokens([]byte(strings.Repeat("{\"a\":", MaxStepsDepth+1))); err == nil || !strings.Contains(err.Error(), "deeper than") {
		t.Fatalf("objects: %v", err)
	}
	// The real shape is four deep and passes.
	if err := checkTokens([]byte(okSteps)); err != nil {
		t.Fatal(err)
	}
}

func TestDecodeStepsScalarBudget(t *testing.T) {
	var b strings.Builder
	b.WriteString(`{"schema":"kiln-legacy-steps/1","os":"linux","packages":[{"package":"a","steps":[`)
	for i := 0; i <= MaxStepsScalars; i++ {
		if i > 0 {
			b.WriteByte(',')
		}
		b.WriteString(`"x"`)
	}
	b.WriteString(`]}]}`)
	if _, err := DecodeSteps([]byte(b.String())); err == nil || !strings.Contains(err.Error(), "more than") {
		t.Fatalf("got %v", err)
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
