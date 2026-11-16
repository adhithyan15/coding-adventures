package warrantcorpus

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestParseBindingAcceptsTheReferenceFile(t *testing.T) {
	b := parseBinding([]byte("# comment\n\n" + bindingText(nil) + "   \r\n"))
	if b.Lane != "go" || b.Package != "go/ct-compare" || b.Module != "ct_compare" || b.ImportPath != importPath || b.Source != "." {
		t.Fatalf("%+v", b)
	}
	if b.Seed != 0x5741525241525421-0x5741525241525421+0x5741_5252_414e_5421 {
		t.Fatalf("seed %x", b.Seed)
	}
	if len(b.Ops) != 4 || b.Ops["eq_fixed"].Width != widthImplicit || b.Ops["eq"].Width != widthNone {
		t.Fatalf("ops: %+v", b.Ops)
	}
	if rows := b.Ops["select_bytes"].Errors; len(rows) != 1 || rows[0].Variant != "CT_SELECT_LENGTH_MISMATCH" || rows[0].Sentinel != "ErrRefLength" {
		t.Fatalf("errors: %+v", rows)
	}
	if b.Values["U64"] != "uint64" {
		t.Fatal("values")
	}
	if string(b.Raw) == "" {
		t.Fatal("the raw bytes are kept for the digest")
	}
}

func TestParseBindingSeeds(t *testing.T) {
	text := func(seed string) string { return strings.Replace(bindingText(nil), "0x5741_5252_414e_5421", seed, 1) }
	for seed, want := range map[string]uint64{"0": 0, "123": 123, "0123": 123, "1_000": 1000, "0xFF": 255, "0x1_0": 16, "18446744073709551615": ^uint64(0), "0xffffffffffffffff": ^uint64(0)} {
		if got := parseBinding([]byte(text(seed))).Seed; got != want {
			t.Errorf("seed %s = %d, want %d", seed, got, want)
		}
	}
	for _, seed := range []string{"18446744073709551616", "0x10000000000000000", "0x", "0x_", "-1", "1.5", "abc", "0xg"} {
		seed := seed
		if code := laneCode(t, func() { parseBinding([]byte(text(seed))) }); code != "syntax" {
			t.Errorf("seed %q: %q", seed, code)
		}
	}
}

func TestParseBindingRefusals(t *testing.T) {
	good := bindingText(nil)
	replace := func(old, new string) string { return strings.Replace(good, old, new, 1) }
	cases := []struct{ name, code, text string }{
		{"not utf-8", "syntax", "\xff" + good},
		{"too large", "input-limit", good + strings.Repeat("#", maxBindingBytes)},
		{"unknown directive", "syntax", good + "frobnicate x\n"},
		{"duplicate single", "syntax", good + "lane go\n"},
		{"single arity", "syntax", replace("lane go", "lane go extra")},
		{"missing lane", "incomplete", replace("lane go\n", "")},
		{"missing ops", "incomplete", "lane go\npackage go/x\nmodule m\nimport a/b\nsource .\nseed 1\n"},
		{"import with dotdot", "syntax", replace("import "+importPath, "import a/../b")},
		{"import with space-free junk", "syntax", replace("import "+importPath, "import a//b")},
		{"import dot", "syntax", replace("import "+importPath, "import .")},
		{"source dotdot", "syntax", replace("source .", "source ../x")},
		{"source absolute", "syntax", replace("source .", "source /etc")},
		{"source dot component", "syntax", replace("source .", "source a/./b")},
		{"values malformed", "syntax", replace("values ", "values Bool ")},
		{"values duplicate", "syntax", good + "values Bool=bool\n"},
		{"values int", "values", replace("U64=uint64", "U64=int")},
		{"values float", "values", replace("U64=uint64", "U64=float64")},
		{"values unknown scalar", "values", replace("U64=uint64", "Float=float64")},
		{"values wrong bytes", "values", replace("Bytes=[]byte", "Bytes=string")},
		{"op without export", "syntax", replace("op eq export=RefEq", "op eq")},
		{"op name", "syntax", replace("op eq export=RefEq", "op Eq export=RefEq")},
		{"op duplicate", "syntax", good + "op eq export=RefEq\n"},
		{"op unknown option", "syntax", replace("op eq export=RefEq", "op eq export=RefEq color=red")},
		{"op option without value", "syntax", replace("op eq export=RefEq", "op eq export=")},
		{"op duplicate option", "syntax", replace("op eq export=RefEq", "op eq export=RefEq export=RefEq")},
		{"op only name", "syntax", replace("op eq export=RefEq", "op eq color=red")},
		{"op bad export", "syntax", replace("op eq export=RefEq", "op eq export=1x")},
		{"op bad width", "syntax", replace("width=implicit", "width=static")},
		{"op bad variant", "syntax", replace("CT_SELECT_LENGTH_MISMATCH:", "bad:")},
		{"op no sentinel", "syntax", replace("CT_SELECT_LENGTH_MISMATCH:ErrRefLength", "CT_SELECT_LENGTH_MISMATCH")},
		{"op sentinel not Err", "bad-error", replace("ErrRefLength", "Exception")},
		{"op op", "syntax", "op\n" + good},
	}
	for _, c := range cases {
		c := c
		if code := laneCode(t, func() { parseBinding([]byte(c.text)) }); code != c.code {
			t.Errorf("%s: code %q, want %q", c.name, code, c.code)
		}
	}
	// `Err` alone is a legal sentinel name.
	parseBinding([]byte(replace("ErrRefLength", "Err")))
	// Several rows in one errors= list.
	b := parseBinding([]byte(replace("CT_SELECT_LENGTH_MISMATCH:ErrRefLength", "A:ErrOne,B:ErrTwo,")))
	if len(b.Ops["select_bytes"].Errors) != 2 {
		t.Fatal("two rows")
	}
	parseBinding([]byte(replace("values ", "values U8=uint8 ")))
}

func resolveFor(t *testing.T, text string, registry *Registry, pkg string) error {
	t.Helper()
	return guard(func() { resolveExports(parseBinding([]byte(text)), registry, pkg) })
}

func TestResolveExportsProvenanceUsesTheRealRuntime(t *testing.T) {
	pkgDir, err := filepath.Abs(".")
	if err != nil {
		t.Fatal(err)
	}
	// Positive: the reference functions are defined in this package's own
	// directory, which is the declared source root.
	if err := resolveFor(t, bindingText(nil), refRegistry(), pkgDir); err != nil {
		t.Fatalf("the reference library should resolve: %v", err)
	}
	other := t.TempDir()
	for name, c := range map[string]struct {
		text     string
		registry func(*Registry)
		pkg      string
		code     string
	}{
		"source root elsewhere": {bindingText(nil), nil, other, "source-mismatch"},
		"missing source root":   {bindingText(nil), nil, filepath.Join(other, "none"), "import"},
		"wrong import":          {strings.Replace(bindingText(nil), "import "+importPath, "import example.com/other", 1), nil, pkgDir, "source-mismatch"},
		"export missing":        {strings.Replace(bindingText(nil), "export=RefEq\n", "export=Nope\n", 1), nil, pkgDir, "export"},
		"impl under the wrong name": {bindingText(nil), func(r *Registry) {
			r.Exports["RefEq"] = Export{Call: r.Exports["RefEq"].Call, Impl: RefEqU64}
		}, pkgDir, "source-mismatch"},
		"closure as impl": {bindingText(nil), func(r *Registry) {
			r.Exports["RefEq"] = Export{Call: r.Exports["RefEq"].Call, Impl: func(a, b []byte) bool { return true }}
		}, pkgDir, "source-mismatch"},
		"method value as impl": {bindingText(nil), func(r *Registry) {
			r.Exports["RefEq"] = Export{Call: r.Exports["RefEq"].Call, Impl: (&strings.Builder{}).Len}
		}, pkgDir, "source-mismatch"},
		"impl not a function": {bindingText(nil), func(r *Registry) {
			r.Exports["RefEq"] = Export{Call: r.Exports["RefEq"].Call, Impl: 5}
		}, pkgDir, "export"},
		"impl nil function": {bindingText(nil), func(r *Registry) {
			var fn func()
			r.Exports["RefEq"] = Export{Call: r.Exports["RefEq"].Call, Impl: fn}
		}, pkgDir, "export"},
		"no impl":          {bindingText(nil), func(r *Registry) { r.Exports["RefEq"] = Export{Call: r.Exports["RefEq"].Call} }, pkgDir, "export"},
		"no call":          {bindingText(nil), func(r *Registry) { r.Exports["RefEq"] = Export{Impl: RefEq} }, pkgDir, "export"},
		"sentinel missing": {bindingText(nil), func(r *Registry) { delete(r.Errors, "ErrRefLength") }, pkgDir, "bad-error"},
		"sentinel nil":     {strings.Replace(bindingText(nil), "ErrRefLength", "ErrNil", 1), nil, pkgDir, "bad-error"},
	} {
		registry := refRegistry()
		if c.registry != nil {
			c.registry(registry)
		}
		err := resolveFor(t, c.text, registry, c.pkg)
		var lane *LaneError
		if !errors.As(err, &lane) || lane.Code != c.code {
			t.Errorf("%s: %v, want code %s", name, err, c.code)
		}
	}
	wantCode(t, "registry", func() { resolveExports(parseBinding([]byte(bindingText(nil))), nil, pkgDir) })
	// A file path the runtime reports that does not exist (a -trimpath build).
	previous := symbolOf
	defer func() { symbolOf = previous }()
	symbolOf = func(impl any) (string, string, bool) {
		name, _, ok := funcSymbol(impl)
		return name, "relative/trimmed.go", ok
	}
	if err := resolveFor(t, bindingText(nil), refRegistry(), pkgDir); err == nil || !strings.Contains(err.Error(), "-trimpath") {
		t.Fatalf("a trimmed path must be refused: %v", err)
	}
}

func TestFuncSymbol(t *testing.T) {
	name, file, ok := funcSymbol(RefEq)
	if !ok || name != importPath+".RefEq" || !strings.HasSuffix(filepath.ToSlash(file), "/helpers_test.go") {
		t.Fatalf("(%q, %q, %v)", name, file, ok)
	}
	if _, _, ok := funcSymbol(nil); ok {
		t.Fatal("nil is not a function")
	}
	if _, err := os.Stat(file); err != nil {
		t.Fatalf("the recorded file should exist on this host: %v", err)
	}
}

// subjectFor builds a Subject over the reference registry for bindings text.
func subjectFor(t *testing.T, f *fixture, text string, fault string, faultOp string) (*Subject, *Corpus) {
	t.Helper()
	mustWrite(t, f.binding, []byte(text))
	var prepared *Prepared
	if code := laneCode(t, func() { prepared = prepare(f.config(), refRegistry()) }); code != "" {
		t.Fatalf("prepare: %s", code)
	}
	var fl *Fault
	if fault != "" {
		fl = faults[fault]
	}
	return prepared.subject(fl, faultOp), prepared.Corpus
}

// failureOf runs one call and returns the verdict failure id ("" when it passes).
func failureOf(s *Subject, op string, args []any, width int, hasWidth bool) (result any, failure string) {
	return evalSubject(func([]any) any { return s.call(op, args, width, hasWidth) }, nil)
}

func TestSubjectCallsTheLibraryAndTypeChecks(t *testing.T) {
	f := newFixture(t, fixtureOptions{})
	s, _ := subjectFor(t, f, bindingText(nil), "", "")
	got, failure := failureOf(s, "eq", []any{[]byte{1}, []byte{1}}, 0, false)
	if failure != "" || got != true || s.callsTo("eq") != 1 || s.totalCalls([]string{"eq", "eq_u64"}) != 1 {
		t.Fatalf("(%v, %q) calls=%d", got, failure, s.callsTo("eq"))
	}
	got, failure = failureOf(s, "select_bytes", []any{[]byte{1}, []byte{2}, true}, 0, false)
	if failure != "" || !valuesEqual(got, Pair{"ok", []byte{1}}) {
		t.Fatalf("(%v, %q)", got, failure)
	}
	got, failure = failureOf(s, "select_bytes", []any{[]byte{1}, []byte{2, 3}, true}, 0, false)
	if failure != "" || !valuesEqual(got, Pair{"err", "CT_SELECT_LENGTH_MISMATCH"}) {
		t.Fatalf("(%v, %q)", got, failure)
	}
	// The argument handed to the library is a copy: the caller's value is intact.
	original := []byte{7, 7}
	failureOf(s, "select_bytes", []any{original, []byte{1, 1}, true}, 0, false)
	if original[0] != 7 {
		t.Fatal("the enumerated domain was touched")
	}
	// Width checks.
	wantCode(t, "width", func() { s.call("eq_fixed", []any{[]byte{1}, []byte{1}}, 2, true) })
}

func TestSubjectVerdicts(t *testing.T) {
	f := newFixture(t, fixtureOptions{})
	type verdict struct {
		name, binding string
		args          []any
		op            string
		want          string
	}
	mismatch := []any{[]byte{1}, []byte{2, 3}, true}
	same := []any{[]byte{1}, []byte{2}, true}
	for _, v := range []verdict{
		{"wrong adapter type int", bindingText(map[string]string{"eq": "op eq export=RefEqReturnsInt"}), []any{[]byte{}, []byte{}}, "eq", TypeMismatch},
		{"nil result", bindingText(map[string]string{"eq": "op eq export=RefEqReturnsNil"}), []any{[]byte{}, []byte{}}, "eq", TypeMismatch},
		{"stray pair", bindingText(map[string]string{"eq": "op eq export=RefEqReturnsPair"}), []any{[]byte{}, []byte{}}, "eq", TypeMismatch},
		{"alias of argument", bindingText(map[string]string{"select_bytes": "op select_bytes export=RefSelectAlias errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength"}), same, "select_bytes", TypeMismatch},
		{"unmapped error", bindingText(map[string]string{"select_bytes": "op select_bytes export=RefSelectOther errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength"}), mismatch, "select_bytes", UnexpectedError},
		{"wrapped sentinel", bindingText(map[string]string{"select_bytes": "op select_bytes export=RefSelectWrapped errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength"}), mismatch, "select_bytes", UnexpectedError},
		{"panic", bindingText(map[string]string{"select_bytes": "op select_bytes export=RefSelectPanics errors=CT_SELECT_LENGTH_MISMATCH:ErrRefLength"}), mismatch, "select_bytes", UnexpectedError},
	} {
		s, _ := subjectFor(t, f, v.binding, "", "")
		if _, failure := failureOf(s, v.op, v.args, 0, false); failure != v.want {
			t.Errorf("%s: failure %q, want %q", v.name, failure, v.want)
		}
	}
	// The correct library passes the same calls.
	s, _ := subjectFor(t, f, bindingText(nil), "", "")
	if _, failure := failureOf(s, "select_bytes", same, 0, false); failure != "" {
		t.Fatalf("fresh result flagged: %q", failure)
	}
}

func TestSubjectFaultsOnlyTouchTheirOp(t *testing.T) {
	f := newFixture(t, fixtureOptions{})
	s, _ := subjectFor(t, f, bindingText(nil), "invert", "eq")
	if got, _ := failureOf(s, "eq", []any{[]byte{1}, []byte{1}}, 0, false); got != false {
		t.Fatal("invert should flip eq")
	}
	if got, _ := failureOf(s, "eq_u64", []any{uint64(1), uint64(1)}, 0, false); got != true {
		t.Fatal("the fault is scoped to one op")
	}
	// Without an op, the fault applies to every op it fits.
	s, _ = subjectFor(t, f, bindingText(nil), "invert", "")
	if got, _ := failureOf(s, "eq_u64", []any{uint64(1), uint64(1)}, 0, false); got != false {
		t.Fatal("unscoped fault applies everywhere")
	}
	// A fault that does not apply to the op is skipped.
	s, _ = subjectFor(t, f, bindingText(nil), "swap-choice", "")
	if got, _ := failureOf(s, "eq", []any{[]byte{1}, []byte{1}}, 0, false); got != true {
		t.Fatal("swap-choice does not apply to eq")
	}
	// missing-error turns the declared error into an Ok of the first Bytes argument.
	s, _ = subjectFor(t, f, bindingText(nil), "missing-error", "select_bytes")
	got, failure := failureOf(s, "select_bytes", []any{[]byte{1}, []byte{2, 3}, true}, 0, false)
	if failure != "" || !valuesEqual(got, Pair{"ok", []byte{1}}) {
		t.Fatalf("(%v, %q)", got, failure)
	}
}

func TestSubjectRuntimeWidthAndSignatureWidths(t *testing.T) {
	f := newFixture(t, fixtureOptions{})
	text := bindingText(map[string]string{"eq_fixed": "op eq_fixed export=RefWidth width=runtime"})
	s, _ := subjectFor(t, f, text, "", "")
	if got, failure := failureOf(s, "eq_fixed", []any{[]byte{1, 2}, []byte{1, 2}}, 2, true); failure != "" || got != true {
		t.Fatalf("(%v, %q)", got, failure)
	}
	wantCode(t, "width", func() { s.call("eq_fixed", []any{[]byte{1, 2}, []byte{1, 2}}, 0, false) })
	// A signature that states a length asserts it per call.
	s.sigs = map[string]*OpSig{"eq": {Name: "eq", Params: []Param{param("a", "Bytes<2>"), param("b", "Bytes<2>")}, Ret: parseType("Bool")}}
	wantCode(t, "width", func() { s.call("eq", []any{[]byte{1}, []byte{1, 2}}, 0, false) })
	wantCode(t, "width", func() { s.call("eq", []any{uint64(1), []byte{1, 2}}, 0, false) })
}

func TestRequireFreshRestoresWhatItMutates(t *testing.T) {
	a, b := []byte{1, 2}, []byte{3, 4}
	result := []byte{9, 9}
	requireFresh(Pair{"ok", result}, []any{a, b, true})
	if !valuesEqual(a, []byte{1, 2}) || !valuesEqual(b, []byte{3, 4}) || !valuesEqual(result, []byte{9, 9}) {
		t.Fatal("requireFresh must restore everything it flips")
	}
	requireFresh(true, []any{a}) // not bytes: nothing to check
	requireFresh(Pair{"err", "E"}, []any{a})
	requireFresh([]byte(nil), []any{a})
	for _, shared := range []func() ([]byte, []any){
		func() ([]byte, []any) { return a, []any{a} },        // same slice
		func() ([]byte, []any) { return a[:1], []any{a} },    // prefix of an argument
		func() ([]byte, []any) { return a[1:], []any{b, a} }, // suffix of another argument
	} {
		out, args := shared()
		failure := ""
		func() {
			defer func() {
				if r := recover(); r != nil {
					failure = r.(subjectFailure).id
				}
			}()
			requireFresh(out, args)
		}()
		if failure != TypeMismatch {
			t.Errorf("sharing not detected (%q)", failure)
		}
		a = []byte{1, 2} // restore for the next case
	}
}
