package record

import (
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

func TestClassifyFileRoundTripsAndIsStrict(t *testing.T) {
	h := &legacy.HostRoots{
		Roots: []legacy.HostRoot{
			{Path: "/opt/x", Kind: "tool", Tool: "xtool"},
			{Path: "/usr", Kind: "image"},
			{Path: "/var/cache/z", Kind: "state", Ecosystem: "go", Fetchers: []string{"/a", "/b"}},
		},
		Ephemeral: []string{"/dev/null", "/dev/fd/*"},
	}
	// "/dev/fd/*" sorts before "/dev/null": the encoder refuses unsorted lists.
	if _, err := EncodeClassify(h, []string{"go"}); err == nil {
		t.Fatal("an unsorted ephemeral list was encoded")
	}
	h.Ephemeral = []string{"/dev/fd/*", "/dev/null"}
	data, err := EncodeClassify(h, []string{"rust", "go"}) // pinned is sorted for the caller
	if err != nil {
		t.Fatal(err)
	}
	want := `{"schema":"kiln-record-classify/1","host_roots":{"roots":[` +
		`{"path":"/opt/x","kind":"tool","tool":"xtool"},{"path":"/usr","kind":"image"},` +
		`{"path":"/var/cache/z","kind":"state","ecosystem":"go","fetchers":["/a","/b"]}],` +
		`"ephemeral":["/dev/fd/*","/dev/null"],"absent-ok":[]},"pinned":["go","rust"]}`
	if string(data) != want {
		t.Fatalf("%s", data)
	}
	c, err := DecodeClassify(data)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(c.HostRoots.Roots, h.Roots) || !reflect.DeepEqual(c.Pinned, []string{"go", "rust"}) {
		t.Fatalf("%+v", c)
	}
	for name, in := range map[string]string{
		"trailing data":    want + "{}",
		"an unknown key":   strings.Replace(want, `"pinned"`, `"extra":1,"pinned"`, 1),
		"a nested unknown": strings.Replace(want, `"kind":"image"`, `"kind":"image","x":1`, 1),
		"null roots":       strings.Replace(want, `"roots":[`, `"roots":null,"x":[`, 1),
		"null ephemeral":   strings.Replace(want, `"ephemeral":["/dev/fd/*","/dev/null"]`, `"ephemeral":null`, 1),
		"null absent-ok":   strings.Replace(want, `"absent-ok":[]`, `"absent-ok":null`, 1),
		"duplicate pinned": strings.Replace(want, `["go","rust"]`, `["go","go"]`, 1),
		"duplicate root":   strings.Replace(want, `{"path":"/usr","kind":"image"}`, `{"path":"/opt/x","kind":"image"}`, 1),
		"empty root path":  strings.Replace(want, `"path":"/usr"`, `"path":""`, 1),
		"unsorted fetches": strings.Replace(want, `["/a","/b"]`, `["/b","/a"]`, 1),
		"wrong schema":     strings.Replace(want, "classify/1", "classify/2", 1),
		"not json":         "{",
	} {
		if _, err := DecodeClassify([]byte(in)); err == nil {
			t.Errorf("%s: decoded", name)
		}
	}
	if _, err := EncodeClassify(nil, nil); err == nil {
		t.Fatal("encoded without host roots")
	}
	// An empty policy encodes with [] lists, never null.
	e, err := EncodeClassify(&legacy.HostRoots{}, nil)
	if err != nil || !strings.Contains(string(e), `"roots":[],"ephemeral":[],"absent-ok":[]},"pinned":[]`) {
		t.Fatalf("%s %v", e, err)
	}
}

func goodClassified() ClassifiedRecord {
	return ClassifiedRecord{
		Schema: ClassifiedSchema, Action: "code/packages/x/a", BindingID: strings.Repeat("a", 64), SealedSHA256: strings.Repeat("b", 64), StepsRun: 1, StepNetNS: true,
		ParseOK: true, ParseReasons: []string{}, WritesComplete: true,
		Record: &traced.RawRecord{Package: "code/packages/x/a", Entries: []string{"pkg:code/packages/x/a"}, Writes: []string{}, TrackedWrites: []string{},
			AbsentObservations: []string{}, Binds: []string{}, UntrackedObservations: []string{}, Traversed: []string{}, Reasons: []string{}},
	}
}

func TestClassifiedRecordRoundTripsAndIsStrict(t *testing.T) {
	good := goodClassified()
	data, err := EncodeClassified(good)
	if err != nil {
		t.Fatal(err)
	}
	if strings.HasSuffix(string(data), "\n") || !strings.HasPrefix(string(data), `{"schema":"kiln-record-classified/1","action":"code/packages/x/a","binding_id":"`) {
		t.Fatalf("%s", data)
	}
	back, err := DecodeClassified(data)
	if err != nil || !back.ParseOK || back.Record == nil || !back.WritesComplete || back.StepsRun != 1 {
		t.Fatalf("%+v %v", back, err)
	}
	// A parse failure carries its reasons and no raw record.
	bad := ClassifiedRecord{Schema: ClassifiedSchema, Action: good.Action, BindingID: good.BindingID, SealedSHA256: good.SealedSHA256, ParseReasons: []string{"step 0: x"}}
	data, err = EncodeClassified(bad)
	if err != nil || !strings.Contains(string(data), `"record":null`) {
		t.Fatalf("%s %v", data, err)
	}
	if _, err := DecodeClassified(data); err != nil {
		t.Fatal(err)
	}
	// nil reasons are written as [].
	none := good
	none.ParseReasons = nil
	if data, err := EncodeClassified(none); err != nil || !strings.Contains(string(data), `"parse_reasons":[]`) {
		t.Fatalf("%s %v", data, err)
	}
	mut := func(f func(c *ClassifiedRecord)) ClassifiedRecord { c := goodClassified(); f(&c); return c }
	for name, c := range map[string]ClassifiedRecord{
		"schema":                  mut(func(c *ClassifiedRecord) { c.Schema = "x" }),
		"bad action":              mut(func(c *ClassifiedRecord) { c.Action = "../x" }),
		"short binding":           mut(func(c *ClassifiedRecord) { c.BindingID = "abc" }),
		"short sealed digest":     mut(func(c *ClassifiedRecord) { c.SealedSHA256 = "abc" }),
		"negative steps":          mut(func(c *ClassifiedRecord) { c.StepsRun = -1 }),
		"ok with reasons":         mut(func(c *ClassifiedRecord) { c.ParseReasons = []string{"x"} }),
		"not ok without reasons":  mut(func(c *ClassifiedRecord) { c.ParseOK = false }),
		"ok without a record":     mut(func(c *ClassifiedRecord) { c.Record = nil }),
		"a record without ok":     mut(func(c *ClassifiedRecord) { c.ParseOK, c.ParseReasons = false, []string{"x"} }),
		"complete without record": mut(func(c *ClassifiedRecord) { c.ParseOK, c.ParseReasons, c.Record = false, []string{"x"}, nil }),
		"unsorted reasons": mut(func(c *ClassifiedRecord) {
			c.ParseOK, c.ParseReasons, c.Record, c.WritesComplete = false, []string{"b", "a"}, nil, false
		}),
		"another package": mut(func(c *ClassifiedRecord) { c.Record.Package = "code/other" }),
	} {
		if _, err := EncodeClassified(c); err == nil {
			t.Errorf("%s: encoded", name)
		}
	}
	good2, _ := EncodeClassified(good)
	for name, in := range map[string]string{
		"an unknown key": strings.Replace(string(good2), `"parse_ok"`, `"extra":1,"parse_ok"`, 1),
		"trailing data":  string(good2) + "{}",
		"null reasons":   strings.Replace(string(good2), `"parse_reasons":[]`, `"parse_reasons":null`, 1),
		"not json":       "{",
	} {
		if _, err := DecodeClassified([]byte(in)); err == nil {
			t.Errorf("%s: decoded", name)
		}
	}
}

func TestFlagValue(t *testing.T) {
	for name, c := range map[string]struct {
		argv []string
		want string
		ok   bool
	}{
		"present":      {[]string{"kiln", "x", "--sha256", "h", "--", "sh"}, "h", true},
		"absent":       {[]string{"kiln", "--", "sh"}, "", false},
		"after the --": {[]string{"kiln", "--", "--sha256", "h"}, "", false},
		"twice":        {[]string{"--sha256", "a", "--sha256", "b"}, "", false},
		"no value":     {[]string{"kiln", "--sha256"}, "", false},
		"nil":          {nil, "", false},
	} {
		got, ok := flagValue(c.argv, "--sha256")
		if got != c.want || ok != c.ok {
			t.Errorf("%s: %q %v", name, got, ok)
		}
	}
}

func TestSidecarSeqIsValidated(t *testing.T) {
	good := StepSidecar{
		Schema: StepSchema, Action: "code/p", Step: 0, StepCount: 1, Seq: 7, BindingID: strings.Repeat("a", 64),
		LineSHA256: strings.Repeat("b", 64), Started: true, StartedAt: "2026-11-13T00:00:00Z", EndedAt: "2026-11-13T00:00:01Z",
		TraceSHA256: strings.Repeat("c", 64), StepNetNS: true, Clean: true, StraceStdoutEmpty: true, StraceStderrEmpty: true,
	}
	data, err := EncodeStepSidecar(good)
	if err != nil || !strings.Contains(string(data), `"seq":7`) {
		t.Fatalf("%s %v", data, err)
	}
	back, err := DecodeStepSidecar(data)
	if err != nil || back.Seq != 7 {
		t.Fatalf("%+v %v", back, err)
	}
	neg := good
	neg.Seq = -1
	if _, err := EncodeStepSidecar(neg); err == nil {
		t.Fatal("a negative seq was encoded")
	}
	// A sidecar without seq is a schema/1 file: refused (the field is required
	// by the schema, and the schema string differs).
	old := strings.Replace(strings.Replace(string(data), `"seq":7,`, "", 1), "step/2", "step/1", 1)
	if _, err := DecodeStepSidecar([]byte(old)); err == nil {
		t.Fatal("a step/1 sidecar was decoded")
	}
}
