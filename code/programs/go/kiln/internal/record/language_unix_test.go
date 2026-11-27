//go:build unix

package record

// The recording selector, prepare side (KILN04 7.3)
// =================================================
//
// `kiln record prepare --language L` plans only language L's
// prerequisite-closed packages and seals L in the binding. The default, `all`,
// must change NOTHING: not one argument of the build tool, not one byte of the
// binding (and so not one binding id, which every record line names).

import (
	"bytes"
	"encoding/json"
	"reflect"
	"strings"
	"testing"
)

// planCall is the build tool's argv of the plan export (the one with
// -emit-plan), as the fake tool saw it.
func planCall(t *testing.T, p *prep) []string {
	t.Helper()
	for _, c := range p.toolCalls {
		for _, a := range c {
			if a == "-emit-plan" {
				return c[1:]
			}
		}
	}
	t.Fatal("the build tool was never asked for a plan")
	return nil
}

func argHas(l []string, s string) bool {
	for _, x := range l {
		if x == s {
			return true
		}
	}
	return false
}

// The default selection is byte-identical to what prepare did before the
// selector existed: PlanArgs spells the same arguments, and the binding is the
// same bytes (no "language" key).
func TestDefaultSelectionChangesNothing(t *testing.T) {
	// The arguments as they were written before the parameter existed.
	was := func(root string, count int, planPath string) []string {
		return []string{"-root", root, "-force", "-language", "all", "-shard-count", "2",
			"-shard-weights", ShardWeightsPath, "-emit-plan", planPath}
	}
	for _, l := range []string{"", "all"} {
		if got, want := PlanArgs("/r", 2, "/p", l), was("/r", 2, "/p"); !reflect.DeepEqual(got, want) {
			t.Errorf("PlanArgs(%q) = %v, want %v", l, got, want)
		}
	}

	for _, l := range []string{"", "all"} {
		p := newPrep(t)
		p.commit()
		p.language = l
		res := p.mustPrepare()
		call := planCall(t, p)
		if !argHas(call, "-language") || argHas(call, "-plan-closed-language") {
			t.Errorf("language %q: plan call %v", l, call)
		}
		raw := readPre(t, res, "binding.json")
		if bytes.Contains(raw, []byte("language")) {
			t.Errorf("language %q: the default binding carries a language key: %s", l, raw)
		}
		if res.Binding.Language != "" {
			t.Errorf("language %q: binding.Language = %q", l, res.Binding.Language)
		}
	}
}

// `--language x` passes -plan-closed-language x INSTEAD of -language all, never
// both, only to the plan export (the graph and steps exports stay whole), and
// seals x beside the plan hash.
func TestLanguageSelectionIsPassedAndSealed(t *testing.T) {
	if got := PlanArgs("/r", 2, "/p", "go"); !argHas(got, "-plan-closed-language") || argHas(got, "-language") ||
		got[argIndex(got, "-plan-closed-language")+1] != "go" {
		t.Fatalf("PlanArgs(go) = %v", got)
	}
	p := newPrep(t)
	p.commit()
	p.language = "x"
	res := p.mustPrepare()
	call := planCall(t, p)
	if !argHas(call, "-plan-closed-language") || argHas(call, "-language") || argHas(call, "all") {
		t.Fatalf("plan call %v", call)
	}
	for _, c := range p.toolCalls {
		if argHas(c, "-emit-kiln-graph") && (argHas(c, "-plan-closed-language") || argHas(c, "-language")) {
			t.Fatalf("the graph export was narrowed: %v", c)
		}
	}
	var b map[string]any
	if err := json.Unmarshal(readPre(t, res, "binding.json"), &b); err != nil {
		t.Fatal(err)
	}
	if b["language"] != "x" || res.Binding.Language != "x" {
		t.Fatalf("binding: %v", b)
	}
	if b["partition_sha256"] == "" {
		t.Fatal("the plan hash is not sealed")
	}
}

func argIndex(l []string, s string) int {
	for i, x := range l {
		if x == s {
			return i
		}
	}
	return -1
}

// A name that is not a language token is refused before any process starts.
func TestLanguageSelectionIsValidatedBeforeAnyWork(t *testing.T) {
	for _, bad := range []string{"Go", "go lang", "-x", "a/b", strings.Repeat("a", 40), "go;rm"} {
		p := newPrep(t)
		p.commit()
		p.language = bad
		if _, err := p.prepare(); err == nil || !strings.Contains(err.Error(), "--language") {
			t.Errorf("%q: %v", bad, err)
		}
		if len(p.toolCalls) != 0 {
			t.Errorf("%q: the build tool ran", bad)
		}
	}
}
