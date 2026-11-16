package warrantcorpus

import "testing"

func sigOf(ret string, params ...Param) *OpSig { return &OpSig{Params: params, Ret: parseType(ret)} }

func param(name, ty string) Param { return Param{name, parseType(ty)} }

func TestFaultApplicability(t *testing.T) {
	eq := sigOf("Bool", param("a", "Bytes"), param("b", "Bytes"))
	eqU64 := sigOf("Bool", param("a", "U64"), param("b", "U64"))
	sel := sigOf("Result<Bytes,E>", param("a", "Bytes"), param("b", "Bytes"), param("choice", "Bool"))
	add := sigOf("U64", param("a", "U64"), param("b", "U64"))
	cat := sigOf("Bytes", param("a", "Bytes"))
	want := map[string]map[*OpSig]bool{
		"invert":           {eq: true, eqU64: true, sel: false, add: false},
		"ignore-last-byte": {eq: true, eqU64: false, sel: true, cat: true},
		"ignore-high-bit":  {eq: false, eqU64: true, add: true},
		"swap-choice":      {eq: false, sel: true},
		"missing-error":    {eq: false, sel: true},
		"xor-lsb":          {eq: false, add: true, sel: false},
		"off-by-one":       {eq: false, add: true},
		"return-first-arg": {eq: false, add: true, sel: false, cat: false},
		"truncate-last":    {eq: false, cat: true, sel: false},
		"drop-to-none":     {eq: false, add: false},
	}
	if len(faults) != 10 {
		t.Fatalf("%d faults", len(faults))
	}
	for name, table := range want {
		for sig, applies := range table {
			if faults[name].Applies(sig) != applies {
				t.Errorf("%s on %s: want %v", name, sig.Ret.Text, applies)
			}
		}
	}
}

func TestFaultTransformations(t *testing.T) {
	eq := sigOf("Bool", param("a", "Bytes"), param("b", "Bytes"))
	sel := sigOf("Result<Bytes,E>", param("a", "Bytes"), param("b", "Bytes"), param("choice", "Bool"))
	u64 := sigOf("U64", param("a", "U64"), param("b", "U64"))
	u8 := sigOf("U8", param("a", "U8"))

	in := []any{[]byte{1, 2}, []byte{}}
	out := faults["ignore-last-byte"].OnInput(in, eq)
	if !valuesEqual(out[0], []byte{1, 0}) || !valuesEqual(out[1], []byte{}) || !valuesEqual(in[0], []byte{1, 2}) {
		t.Fatalf("ignore-last-byte: %v (input must not be mutated: %v)", out, in)
	}
	if got := faults["ignore-high-bit"].OnInput([]any{^uint64(0), "x"}, sigOf("Bool", param("a", "U64"), param("b", "U64"))); got[0] != ^uint64(0)>>1 || got[1] != "x" {
		t.Fatalf("ignore-high-bit: %v", got)
	}
	if got := faults["swap-choice"].OnInput([]any{[]byte{}, []byte{}, true}, sel); got[2] != false {
		t.Fatalf("swap-choice: %v", got)
	}
	if got := faults["swap-choice"].OnInput([]any{[]byte{}, []byte{}, 5}, sel); got[2] != 5 {
		t.Fatal("swap-choice leaves a non-bool alone")
	}
	if faults["invert"].OnOutput(true, nil, eq) != false || faults["invert"].OnOutput(7, nil, eq) != 7 {
		t.Fatal("invert")
	}
	given := []any{[]byte{9}, []byte{8}, true}
	if got := faults["missing-error"].OnOutput(Pair{"err", "E"}, given, sel); !valuesEqual(got, Pair{"ok", []byte{9}}) {
		t.Fatalf("missing-error: %v", got)
	}
	if got := faults["missing-error"].OnOutput(Pair{"ok", []byte{1}}, given, sel); !valuesEqual(got, Pair{"ok", []byte{1}}) {
		t.Fatal("missing-error leaves Ok alone")
	}
	if got := faults["missing-error"].OnOutput(Pair{"err", "E"}, given, u64); !valuesEqual(got, Pair{"err", "E"}) {
		t.Fatal("no Bytes param: nothing to return")
	}
	if got := faults["missing-error"].OnOutput(true, given, sel); got != true {
		t.Fatal("missing-error on a non-pair")
	}
	if faults["xor-lsb"].OnOutput(uint64(2), nil, u64) != uint64(3) || faults["xor-lsb"].OnOutput(uint8(3), nil, u8) != uint8(2) || faults["xor-lsb"].OnOutput("x", nil, u64) != "x" {
		t.Fatal("xor-lsb")
	}
	if faults["off-by-one"].OnOutput(^uint64(0), nil, u64) != uint64(0) || faults["off-by-one"].OnOutput(uint8(255), nil, u8) != uint8(0) ||
		faults["off-by-one"].OnOutput(uint64(255), nil, u8) != uint64(0) || faults["off-by-one"].OnOutput(true, nil, u64) != true {
		t.Fatal("off-by-one")
	}
	if faults["return-first-arg"].OnOutput(uint64(0), []any{uint64(4), uint64(5)}, u64) != uint64(4) {
		t.Fatal("return-first-arg")
	}
	cat := sigOf("Bytes", param("a", "Bytes"))
	if got := faults["truncate-last"].OnOutput([]byte{1, 2, 3}, nil, cat); !valuesEqual(got, []byte{1, 2}) {
		t.Fatalf("truncate-last: %v", got)
	}
	if got := faults["truncate-last"].OnOutput([]byte{}, nil, cat); !valuesEqual(got, []byte{}) {
		t.Fatal("truncate-last on empty")
	}
	if faults["truncate-last"].OnOutput(5, nil, cat) != 5 {
		t.Fatal("truncate-last on a non-slice")
	}
	if got := faults["drop-to-none"].OnOutput(1, nil, u64); got.(Pair).Tag != "none" {
		t.Fatalf("drop-to-none: %v", got)
	}
	for name, fault := range faults {
		if name == "ignore-last-byte" || name == "ignore-high-bit" || name == "swap-choice" {
			continue
		}
		if got := fault.OnInput([]any{1}, eq); got[0] != 1 {
			t.Errorf("%s: input stage must be the identity", name)
		}
	}
	if faults["ignore-last-byte"].OnOutput(1, nil, eq) != 1 {
		t.Fatal("input faults leave the output alone")
	}
	if got := zeroLast("not bytes"); got != "not bytes" {
		t.Fatal("zeroLast on a non-slice")
	}
}

func TestGetFault(t *testing.T) {
	if getFault("invert").Name != "invert" {
		t.Fatal("getFault")
	}
	wantCode(t, "unknown-fault", func() { getFault("nope") })
}
