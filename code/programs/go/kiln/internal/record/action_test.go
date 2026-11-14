package record

import (
	"strings"
	"testing"
)

const weakHex = "3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c3b1c"

func TestActionRoundTrip(t *testing.T) {
	a := Action{Schema: ActionSchema, Action: "code/packages/go/parser", Language: "go", Weak: weakHex, Steps: []string{"go build ./...", "go test ./..."}, Tools: []string{"git", "go"}}
	data, err := EncodeAction(a)
	if err != nil {
		t.Fatal(err)
	}
	want := `{"schema":"kiln-record-action/2","action":"code/packages/go/parser","language":"go","weak":"` + weakHex + `","refusal":"","steps":["go build ./...","go test ./..."],"tools":["git","go"]}`
	if string(data) != want {
		t.Fatalf("%s", data)
	}
	back, err := DecodeAction(data)
	if err != nil || back.Action != a.Action || back.Weak != weakHex || len(back.Steps) != 2 || len(back.Tools) != 2 {
		t.Fatalf("%+v %v", back, err)
	}
	// The encoder writes nil steps as [] so the decoder (which refuses null)
	// accepts the file, and does not HTML-escape a shell line.
	r := Action{Schema: ActionSchema, Action: "code/p", Language: "x", Refusal: "unknown-toolchain:y"}
	data, err = EncodeAction(r)
	if err != nil || !strings.Contains(string(data), `"steps":[]`) || !strings.Contains(string(data), `"tools":[]`) {
		t.Fatalf("%s %v", data, err)
	}
	amp := Action{Schema: ActionSchema, Action: "code/p", Language: "x", Weak: weakHex, Steps: []string{"a && b > c"}}
	if data, _ := EncodeAction(amp); !strings.Contains(string(data), "a && b > c") {
		t.Fatalf("%s", data)
	}
}

func TestActionRefusals(t *testing.T) {
	ok := Action{Schema: ActionSchema, Action: "code/p", Language: "x", Weak: weakHex, Steps: []string{}, Tools: []string{}}
	mut := func(f func(a *Action)) Action { a := ok; f(&a); return a }
	for name, a := range map[string]Action{
		"schema":          mut(func(a *Action) { a.Schema = "x" }),
		"bad action path": mut(func(a *Action) { a.Action = "../x" }),
		"empty action":    mut(func(a *Action) { a.Action = "" }),
		"neither key":     mut(func(a *Action) { a.Weak = "" }),
		"both keys":       mut(func(a *Action) { a.Refusal = "r" }),
		"short weak":      mut(func(a *Action) { a.Weak = "abc" }),
		"uppercase weak":  mut(func(a *Action) { a.Weak = strings.ToUpper(weakHex) }),
		"non-hex weak":    mut(func(a *Action) { a.Weak = strings.Repeat("g", 64) }),
		"unsorted tools":  mut(func(a *Action) { a.Tools = []string{"go", "git"} }),
		"duplicate tool":  mut(func(a *Action) { a.Tools = []string{"go", "go"} }),
		"empty tool name": mut(func(a *Action) { a.Tools = []string{""} }),
		"control in tool": mut(func(a *Action) { a.Tools = []string{"g\no"} }),
	} {
		if _, err := EncodeAction(a); err == nil {
			t.Errorf("%s: encoded", name)
		}
	}
}

func TestDecodeActionIsStrict(t *testing.T) {
	good := `{"schema":"kiln-record-action/2","action":"code/p","language":"x","weak":"` + weakHex + `","refusal":"","steps":[],"tools":[]}`
	if _, err := DecodeAction([]byte(good)); err != nil {
		t.Fatal(err)
	}
	for name, in := range map[string]string{
		"unknown key":   strings.Replace(good, `"steps":[]`, `"steps":[],"extra":1`, 1),
		"trailing data": good + "{}",
		"null steps":    strings.Replace(good, `"steps":[]`, `"steps":null`, 1),
		"null tools":    strings.Replace(good, `"tools":[]`, `"tools":null`, 1),
		"no tools":      strings.Replace(good, `,"tools":[]`, ``, 1),
		"not json":      `{`,
		"wrong schema":  strings.Replace(good, "action/2", "action/1", 1),
	} {
		if _, err := DecodeAction([]byte(in)); err == nil {
			t.Errorf("%s: decoded", name)
		}
	}
}

func TestActionFileIsAHashNotAPath(t *testing.T) {
	f := ActionFile("code/packages/go/../../etc\n" + strings.Repeat("x", 5000))
	if strings.Contains(f[len("actions/"):], "/") || strings.Contains(f, "..") || len(f) != len("actions/")+64+len(".json") {
		t.Fatalf("%q", f)
	}
	if ActionFile("a") == ActionFile("b") || ActionFile("a") != ActionFile("a") {
		t.Fatal("not a function of the name")
	}
}
