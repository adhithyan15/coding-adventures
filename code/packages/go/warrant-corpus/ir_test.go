package warrantcorpus

import (
	"strings"
	"testing"
)

var testOps = map[string]*OpSig{
	"eq":    {Name: "eq", Params: []Param{{"a", parseType("Bytes")}, {"b", parseType("Bytes")}}, Ret: parseType("Bool")},
	"fixed": {Name: "fixed", Params: []Param{{"a", parseType("Bytes<N>")}}, Ret: parseType("Bool"), WidthParams: []string{"N"}},
}

func node(t *testing.T, text string) any { t.Helper(); return parseJSON([]byte(text)) }

func models(t *testing.T, text string) map[string]any {
	return parseJSON([]byte(text)).(map[string]any)
}

func evalPure(t *testing.T, expr string, scope []string, env []any) any {
	t.Helper()
	c := newCompiler(models(t, `{"same":{"params":["a","b"],"body":["eq",["var","a"],["var","b"]]},"id":{"params":["a"],"body":["var","a"]}}`), nil, testOps)
	return c.compile(node(t, expr), scope)(env)
}

func TestIREvaluatesEveryNodeKind(t *testing.T) {
	scope := []string{"a", "b"}
	env := []any{uint64(3), uint64(5)}
	cases := map[string]any{
		`["lit", true]`:                                      true,
		`["lit", 7]`:                                         uint64(7),
		`["var", "b"]`:                                       uint64(5),
		`["eq", ["var","a"], ["lit", 3]]`:                    true,
		`["ne", ["var","a"], ["var","b"]]`:                   true,
		`["lt", ["var","a"], ["var","b"]]`:                   true,
		`["le", ["var","a"], ["lit", 3]]`:                    true,
		`["gt", ["var","a"], ["var","b"]]`:                   false,
		`["ge", ["var","b"], ["var","b"]]`:                   true,
		`["and", ["lit", true], ["lit", false]]`:             false,
		`["or", ["lit", false], ["lit", true]]`:              true,
		`["implies", ["lit", false], ["lit", false]]`:        true,
		`["implies", ["lit", true], ["lit", false]]`:         false,
		`["not", ["lit", false]]`:                            true,
		`["if", ["lit", true], ["lit", 1], ["lit", 2]]`:      uint64(1),
		`["if", ["lit", false], ["lit", 1], ["lit", 2]]`:     uint64(2),
		`["add", ["var","a"], ["var","b"]]`:                  uint64(8),
		`["sub", ["var","b"], ["var","a"]]`:                  uint64(2),
		`["mul", ["var","a"], ["var","b"]]`:                  uint64(15),
		`["model_call", "same", [["var","a"], ["var","a"]]]`: true,
		`["model_call", "id", [["var","b"]]]`:                uint64(5),
	}
	for expr, want := range cases {
		if got := evalPure(t, expr, scope, env); got != want {
			t.Errorf("%s = %v, want %v", expr, got, want)
		}
	}
	bytesEnv := []any{[]byte{1, 2}, []byte{1, 3}}
	if got := evalPure(t, `["len", ["var","a"]]`, scope, bytesEnv); got != uint64(2) {
		t.Fatalf("len = %v", got)
	}
	if got := evalPure(t, `["lt", ["var","a"], ["var","b"]]`, scope, bytesEnv); got != true {
		t.Fatal("byte ordering")
	}
	if got := evalPure(t, `["ok", ["var","a"]]`, scope, bytesEnv); !valuesEqual(got, Pair{"ok", []byte{1, 2}}) {
		t.Fatalf("ok = %v", got)
	}
	if got := evalPure(t, `["err", "E"]`, scope, env); !valuesEqual(got, Pair{"err", "E"}) {
		t.Fatalf("err = %v", got)
	}
	if got := evalPure(t, `["name", "E"]`, scope, env); !valuesEqual(got, Pair{"name", "E"}) {
		t.Fatalf("name = %v", got)
	}
}

func TestIRCompileRefusals(t *testing.T) {
	scope := []string{"a"}
	bad := map[string]string{
		"unknown tag":      `["frobnicate", 1]`,
		"not a node":       `5`,
		"empty node":       `[]`,
		"non-string tag":   `[1, 2]`,
		"lit arity":        `["lit"]`,
		"lit string":       `["lit", "x"]`,
		"lit negative":     `["lit", -1]`,
		"var unknown":      `["var", "zzz"]`,
		"var arity":        `["var"]`,
		"var non-string":   `["var", 1]`,
		"len arity":        `["len"]`,
		"ok arity":         `["ok"]`,
		"err arity":        `["err"]`,
		"err non-string":   `["err", 1]`,
		"not arity":        `["not"]`,
		"if arity":         `["if", ["lit", true]]`,
		"binary arity":     `["eq", ["lit", 1]]`,
		"arithmetic arity": `["add", ["lit", 1]]`,
		"model unknown":    `["model_call", "nope", []]`,
		"model arity":      `["model_call", "id", []]`,
		"model malformed":  `["model_call", "id"]`,
		"model non-list":   `["model_call", "id", 5]`,
		"subject in model": `["subject_op", "eq", []]`,
		"subject arity":    `["subject_op"]`,
	}
	for name, expr := range bad {
		expr := expr
		c := newCompiler(models(t, `{"id":{"params":["a"],"body":["var","a"]}}`), nil, testOps)
		if code := laneCode(t, func() { c.compile(node(t, expr), scope) }); code != "unsupported-ir" {
			t.Errorf("%s: code %q", name, code)
		}
	}
}

func TestIRModelValidation(t *testing.T) {
	for name, body := range map[string]string{
		"not an object":    `{"m": 5}`,
		"extra key":        `{"m": {"params":[],"body":["lit",true],"x":1}}`,
		"missing body":     `{"m": {"params":[]}}`,
		"bad params":       `{"m": {"params":"a","body":["lit",true]}}`,
		"non-string param": `{"m": {"params":[1],"body":["lit",true]}}`,
		"recursive":        `{"m": {"params":[],"body":["model_call","m",[]]}}`,
		"mutual recursion": `{"m": {"params":[],"body":["model_call","n",[]]},"n":{"params":[],"body":["model_call","m",[]]}}`,
		"subject in body":  `{"m": {"params":[],"body":["subject_op","eq",[]]}}`,
	} {
		body := body
		c := newCompiler(models(t, body), func(string, []any, int, bool) any { return nil }, testOps)
		if code := laneCode(t, func() { c.compile(node(t, `["model_call","m",[]]`), nil) }); code != "unsupported-ir" {
			t.Errorf("%s: code %q", name, code)
		}
	}
}

func TestIRRuntimeTypeErrors(t *testing.T) {
	scope := []string{"a"}
	for expr, code := range map[string]string{
		`["not", ["lit", 1]]`:                        "ir-type",
		`["and", ["lit", true], ["lit", 1]]`:         "ir-type", // right side is evaluated and checked
		`["if", ["lit", 1], ["lit", 1], ["lit", 2]]`: "ir-type",
		`["lt", ["lit", true], ["lit", false]]`:      "ir-type",
		`["lt", ["lit", 1], ["var", "a"]]`:           "ir-type",
		`["len", ["lit", 1]]`:                        "ir-type",
		`["add", ["lit", true], ["lit", 1]]`:         "ir-overflow",
		`["sub", ["lit", 1], ["lit", 2]]`:            "ir-overflow",
	} {
		expr, code := expr, code
		c := newCompiler(nil, nil, testOps)
		run := c.compile(node(t, expr), scope)
		if got := laneCode(t, func() { run([]any{[]byte{1}}) }); got != code {
			t.Errorf("%s: code %q, want %q", expr, got, code)
		}
	}
	max := "18446744073709551615"
	c := newCompiler(nil, nil, nil)
	_ = max
	for _, op := range []string{"add", "mul"} {
		run := c.compile([]any{op, []any{"var", "a"}, []any{"var", "a"}}, []string{"a"})
		wantCode(t, "ir-overflow", func() { run([]any{^uint64(0)}) })
	}
	run := c.compile([]any{"mul", []any{"var", "a"}, []any{"lit", int64(0)}}, []string{"a"})
	if run([]any{^uint64(0)}) != uint64(0) {
		t.Fatal("mul by zero")
	}
}

func TestIRBounds(t *testing.T) {
	deep := `["lit", true]`
	for i := 0; i < maxIRDepth+2; i++ {
		deep = `["not", ` + deep + `]`
	}
	c := newCompiler(nil, nil, testOps)
	wantCode(t, "ir-limit", func() { c.compile(node(t, deep), nil) })

	// Each model calls the next one twice: 2^n evaluations. The static cost bound
	// refuses it before any case runs.
	var b strings.Builder
	b.WriteString(`{"m0":{"params":[],"body":["lit",true]}`)
	for i := 1; i < 20; i++ {
		b.WriteString(`,"m` + itoa(i) + `":{"params":[],"body":["and",["model_call","m` + itoa(i-1) + `",[]],["model_call","m` + itoa(i-1) + `",[]]]}`)
	}
	b.WriteString("}")
	c = newCompiler(models(t, b.String()), nil, testOps)
	wantCode(t, "ir-limit", func() { c.compile(node(t, `["model_call","m19",[]]`), nil) })
	// A shallow chain is fine.
	c = newCompiler(models(t, b.String()), nil, testOps)
	c.compile(node(t, `["model_call","m3",[]]`), nil)
}

func TestIRSubjectOp(t *testing.T) {
	var gotOp string
	var gotWidth int
	var gotHas bool
	hook := func(op string, args []any, width int, hasWidth bool) any {
		gotOp, gotWidth, gotHas = op, width, hasWidth
		return true
	}
	scope := []string{"a"}
	c := newCompiler(nil, hook, testOps)
	run := c.compile(node(t, `["subject_op","eq",[["var","a"],["var","a"]]]`), scope)
	if run([]any{[]byte{}}) != true || gotOp != "eq" || gotHas {
		t.Fatal("plain subject call")
	}
	run = c.compile(node(t, `["subject_op","fixed",[["var","a"]],[4]]`), scope)
	run([]any{[]byte{}})
	if gotOp != "fixed" || gotWidth != 4 || !gotHas {
		t.Fatalf("width: %d %v", gotWidth, gotHas)
	}
	for name, expr := range map[string]string{
		"unknown op":        `["subject_op","nope",[]]`,
		"missing width":     `["subject_op","fixed",[["var","a"]]]`,
		"width on plain op": `["subject_op","eq",[],[1]]`,
		"bad width list":    `["subject_op","fixed",[],[1,2]]`,
		"width not a list":  `["subject_op","fixed",[],4]`,
		"negative width":    `["subject_op","fixed",[],[-1]]`,
		"string width":      `["subject_op","fixed",[],["x"]]`,
		"args not a list":   `["subject_op","eq",5]`,
		"name not a string": `["subject_op",1,[]]`,
	} {
		expr := expr
		if code := laneCode(t, func() { c.compile(node(t, expr), scope) }); code != "unsupported-ir" {
			t.Errorf("%s: code %q", name, code)
		}
	}
}
