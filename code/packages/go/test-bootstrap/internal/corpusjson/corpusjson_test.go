package corpusjson

import (
	"math/big"
	"strings"
	"testing"
)

func bi(n int64) *big.Int { return big.NewInt(n) }

// These are the Canon tests of the pbt package, kept beside the implementation
// so this package's own coverage gate sees them (pbt keeps its copies: it
// exercises Canon through its wrapper).

type panickyString struct{}

func (panickyString) String() string { return panic0() }
func (panickyString) Error() string  { return panic0() }
func panic0() string                 { panic("String method bug") }

type cyc struct {
	Next *cyc
	N    int
}

func TestCanonRendersCommonShapes(t *testing.T) {
	var nilBig *big.Int
	var nilPtr *int
	type rec struct {
		A int
		B string
	}
	cases := []struct {
		in   any
		want string
	}{
		{true, "true"},
		{false, "false"},
		{bi(-12), "-12"},
		{*bi(7), "7"},
		{nilBig, "nil"},
		{nilPtr, "nil"},
		{nil, "nil"},
		{[]byte{0, 255}, "0x00ff"},
		{[]int{1, 2}, "[1,2]"},
		{[2]int8{-1, 3}, "[-1,3]"},
		{uint64(18446744073709551615), "18446744073709551615"},
		{3.5, "3.5"},
		{"a\"b", `"a\"b"`},
		{map[string]int{"b": 2, "a": 1}, `{"a":1,"b":2}`},
		{rec{3, "x"}, `{3,"x"}`},
		{[]any{bi(1), []byte{1}, "s"}, `[1,0x01,"s"]`},
		{(*int)(new(int)), "0"},
		{make(chan int), "<chan int>"},
		{func() {}, "<func()>"},
		{panickyString{}, "{}"}, // a struct is walked structurally, never by calling String
	}
	for _, c := range cases {
		if got := Canon(c.in); got != c.want {
			t.Errorf("Canon(%#v) = %q, want %q", c.in, got, c.want)
		}
	}
}

func TestCanonIsTotalForHostileWitnesses(t *testing.T) {
	// Depth: a 200,000-deep []any chain renders as "..." past depth 16.
	var deep any = []any{}
	for i := 0; i < 200_000; i++ {
		deep = []any{deep}
	}
	got := Canon(deep)
	if !strings.Contains(got, "...") || len(got) > 200 {
		t.Fatalf("deep witness rendered as %d bytes: %.60s", len(got), got)
	}
	// Maps and pointers nest too.
	var deepMap any = map[string]any{}
	for i := 0; i < 100_000; i++ {
		deepMap = map[string]any{"k": deepMap}
	}
	if got := Canon(deepMap); !strings.Contains(got, "...") {
		t.Fatalf("deep map: %.60s", got)
	}
	// A pointer cycle terminates.
	c := &cyc{N: 1}
	c.Next = c
	if got := Canon(c); !strings.Contains(got, "...") {
		t.Fatalf("cycle: %.80s", got)
	}
	// A wide witness truncates instead of rendering a million nodes.
	wide := make([]int, 1_000_000)
	got = Canon(wide)
	if !strings.Contains(got, "...") || len(got) > 100_000 {
		t.Fatalf("wide witness rendered as %d bytes", len(got))
	}
	// A shared (DAG) witness: 2^40 paths, bounded by the node budget.
	var dag any = 1
	for i := 0; i < 40; i++ {
		dag = []any{dag, dag}
	}
	if got := Canon(dag); len(got) > 200_000 {
		t.Fatalf("DAG witness exploded to %d bytes", len(got))
	}
	// A wide map truncates too.
	m := map[int]int{}
	for i := 0; i < 50_000; i++ {
		m[i] = i
	}
	if got := Canon(m); len(got) > 300_000 {
		t.Fatalf("wide map rendered as %d bytes", len(got))
	}
}

func TestParseIsStrictAndLabelled(t *testing.T) {
	for _, doc := range []string{``, `nope`, `{"schema_version":1,"cases":[]} x`, `{"schema_version":2,"cases":[]}`,
		`{"schema_version":1}`, `{"schema_version":1,"cases":{}}`, `[1]`, `{"schema_version":1,"cases":[],"cases":[]}`,
		`{"schema_version":1,"cases":[`, `{"schema_version":1,"cases":[}`,
		`{"schema_version":1,"cases":` + strings.Repeat("[", MaxDepth+2) + strings.Repeat("]", MaxDepth+2) + `}`,
		`{"schema_version":1,"cases":[],"pad":"` + strings.Repeat("a", MaxBytes) + `"}`,
		`{1:2}`,
	} {
		if _, err := Parse([]byte(doc), "my corpus"); err == nil || !strings.Contains(err.Error(), "my corpus") {
			t.Errorf("%.40q: %v", doc, err)
		}
	}
	cases, err := Parse([]byte(`{"schema_version":1,"cases":[1,"a",null,true,{"x":[{"y":[]}]}]}`), "c")
	if err != nil || len(cases) != 5 {
		t.Fatalf("%v %v", cases, err)
	}
}

func TestDecodeIsStrict(t *testing.T) {
	good := map[string]any{
		"true":  true,
		"text":  "s",
		"int":   map[string]any{"kind": "int", "value": "-12"},
		"bytes": map[string]any{"kind": "bytes", "hex": "00ff"},
		"list":  []any{true, map[string]any{"kind": "int", "value": "0"}},
	}
	for name, node := range good {
		if _, err := Decode(node); err != nil {
			t.Errorf("%s: %v", name, err)
		}
	}
	v, _ := Decode(map[string]any{"kind": "bytes", "hex": "00ff"})
	if b, ok := v.([]byte); !ok || len(b) != 2 || b[0] != 0 || b[1] != 255 {
		t.Fatalf("%v", v)
	}
	for name, node := range map[string]any{
		"float":         1.5,
		"nil":           nil,
		"leading zero":  map[string]any{"kind": "int", "value": "01"},
		"upper hex":     map[string]any{"kind": "bytes", "hex": "FF"},
		"odd hex":       map[string]any{"kind": "bytes", "hex": "f"},
		"extra key":     map[string]any{"kind": "int", "value": "1", "z": 1},
		"unknown kind":  map[string]any{"kind": "x"},
		"long int":      map[string]any{"kind": "int", "value": strings.Repeat("9", MaxIntDigits+1)},
		"long bytes":    map[string]any{"kind": "bytes", "hex": strings.Repeat("00", MaxSize+1)},
		"long list":     make([]any, MaxSize+1),
		"nested error":  []any{[]any{1.5}},
		"non-str value": map[string]any{"kind": "int", "value": 1},
	} {
		if _, err := Decode(node); err == nil {
			t.Errorf("%s accepted", name)
		}
	}
	var deep any = "x"
	for i := 0; i < 1000; i++ {
		deep = []any{deep}
	}
	if _, err := Decode(deep); err == nil {
		t.Fatal("depth cap")
	}
}

func TestSameDoesNotConflateKinds(t *testing.T) {
	if Same(true, bi(1)) || Same(bi(1), true) || Same([]byte{1}, []any{bi(1)}) || Same("1", bi(1)) ||
		Same([]any{bi(1)}, []any{bi(1), bi(2)}) || Same([]any{bi(1)}, []any{bi(2)}) || Same(nil, nil) ||
		Same((*big.Int)(nil), bi(0)) || Same(bi(0), (*big.Int)(nil)) || Same(true, false) || Same("a", "b") ||
		Same([]byte{1}, []byte{2}) || Same([]byte{1}, "x") || Same("x", []byte{1}) || Same(bi(1), "1") {
		t.Fatal("Same conflated two different kinds")
	}
	if !Same([]any{bi(1), true, "s", []byte{2}}, []any{bi(1), true, "s", []byte{2}}) {
		t.Fatal("equal trees")
	}
	var a, b any = []any{}, []any{}
	for i := 0; i < 1000; i++ {
		a, b = []any{a}, []any{b}
	}
	if Same(a, b) {
		t.Fatal("absurd depth is never the same")
	}
}

func TestClipShowAndTextChecks(t *testing.T) {
	if Clip("abc", 3) != "abc" || Clip("abcd", 3) != "abc..." {
		t.Fatal("Clip")
	}
	if got := Show(strings.Repeat("x", 1000)); len(got) > MaxFragmentLen+3 {
		t.Fatal(len(got))
	}
	err := CaseErr("%s", strings.Repeat("y", 1000))
	if len(err.Error()) > MaxProblemLen+3 {
		t.Fatal("CaseErr must clip")
	}
	for in, want := range map[string]bool{"0": true, "-5": true, "12": true, "": false, "-": false, "-0": false, "01": false, "+1": false, "1_0": false, "1 ": false} {
		if IsCanonicalInt(in) != want {
			t.Errorf("IsCanonicalInt(%q)", in)
		}
	}
	for in, want := range map[string]bool{"": true, "00ff": true, "0F": false, "f": false, "zz": false, "0g": false} {
		if IsLowercaseHex(in) != want {
			t.Errorf("IsLowercaseHex(%q)", in)
		}
	}
}
