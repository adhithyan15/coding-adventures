package warrantcorpus

import (
	"strings"
	"testing"
)

var selectErrors = ErrorTypes{"SelectError": {"CT_SELECT_LENGTH_MISMATCH"}}

func TestParseType(t *testing.T) {
	for _, text := range []string{"Bool", "U8", "U64", "Nat", "Bytes", "Bytes<4>", "Bytes<N>", "Result<Bytes,SelectError>", "Result<Result<Bool,E>,E>"} {
		if got := parseType(text); got.Text != text {
			t.Errorf("%s parsed as %s", text, got.Text)
		}
	}
	fixed := parseType("Bytes<4>")
	if !fixed.HasWidth || fixed.Width != 4 || fixed.WidthName != "" {
		t.Fatalf("%+v", fixed)
	}
	param := parseType("Bytes<N>")
	if !param.HasWidth || param.WidthName != "N" {
		t.Fatalf("%+v", param)
	}
	result := parseType("Result<Bytes,SelectError>")
	if result.Kind != kindResult || result.Ok.Kind != kindBytes || result.Error != "SelectError" {
		t.Fatalf("%+v", result)
	}
	for _, bad := range []string{"", "bool", "Int", "Bytes<4097>", "Bytes<99999999999999999999>", "Bytes<n>", "Option<U8>", "List<U8>",
		"Result<Bytes>", "Result<Bytes,selecterror>", "Result<Bytes,E", "Bytes<4", "U8x"} {
		bad := bad
		if code := laneCode(t, func() { parseType(bad) }); code != "unsupported-type" {
			t.Errorf("%q: code %q", bad, code)
		}
	}
	if parseType("Bytes<4096>").Width != 4096 {
		t.Fatal("4096 is the largest width")
	}
}

func TestValuesEqualIsStrict(t *testing.T) {
	equal := [][2]any{
		{true, true}, {uint64(7), uint64(7)}, {"x", "x"}, {[]byte{1, 2}, []byte{1, 2}}, {[]byte{}, []byte(nil)},
		{Pair{"ok", []byte{1}}, Pair{"ok", []byte{1}}}, {Pair{"err", "E"}, Pair{"err", "E"}},
	}
	for _, pair := range equal {
		if !valuesEqual(pair[0], pair[1]) {
			t.Errorf("%v should equal %v", pair[0], pair[1])
		}
	}
	different := [][2]any{
		{true, false}, {true, uint64(1)}, {uint64(1), true}, {uint64(1), uint64(2)}, {uint64(1), 1}, {"x", "y"}, {"x", []byte("x")},
		{[]byte{1}, []byte{2}}, {[]byte{1}, []byte{1, 2}}, {[]byte{1}, uint64(1)},
		{Pair{"ok", true}, Pair{"err", true}}, {Pair{"ok", true}, true}, {Pair{"err", "E"}, Pair{"err", "F"}},
		{3.5, 3.5}, {nil, nil},
	}
	for _, pair := range different {
		if valuesEqual(pair[0], pair[1]) {
			t.Errorf("%v should not equal %v", pair[0], pair[1])
		}
	}
}

func tagged(typeText string, value any) map[string]any {
	return map[string]any{"type": typeText, "value": value}
}

func TestDecodeJSON(t *testing.T) {
	cases := []struct {
		raw  any
		ty   string
		want any
	}{
		{true, "Bool", true},
		{tagged("Bytes", "00ff"), "Bytes", []byte{0, 255}},
		{tagged("Bytes", ""), "Bytes", []byte{}},
		{tagged("Bytes<2>", "0001"), "Bytes<2>", []byte{0, 1}},
		{tagged("Bytes<N>", "0001"), "Bytes<N>", []byte{0, 1}},
		{tagged("U64", "18446744073709551615"), "U64", ^uint64(0)},
		{tagged("U64", "0"), "U64", uint64(0)},
		{tagged("U8", "255"), "U8", uint64(255)},
		{tagged("Nat", "64"), "Nat", uint64(64)},
		{tagged("Result<Bytes,SelectError>", map[string]any{"ok": tagged("Bytes", "01")}), "Result<Bytes,SelectError>", Pair{"ok", []byte{1}}},
		{tagged("Result<Bytes,SelectError>", map[string]any{"err": "CT_SELECT_LENGTH_MISMATCH"}), "Result<Bytes,SelectError>", Pair{"err", "CT_SELECT_LENGTH_MISMATCH"}},
	}
	for _, c := range cases {
		if got := decodeJSON(c.raw, parseType(c.ty), selectErrors); !valuesEqual(got, c.want) {
			t.Errorf("%v as %s: got %v, want %v", c.raw, c.ty, got, c.want)
		}
	}
	bad := []struct {
		raw any
		ty  string
	}{
		{"true", "Bool"}, {int64(1), "Bool"},
		{tagged("Bytes", "0"), "Bytes"}, {tagged("Bytes", "0G"), "Bytes"}, {tagged("Bytes", "AB"), "Bytes"}, {tagged("Bytes", int64(0)), "Bytes"},
		{tagged("Bytes<2>", "00"), "Bytes<2>"}, {tagged("Bytes", "00"), "Bytes<2>"},
		{tagged("U64", "18446744073709551616"), "U64"}, {tagged("U64", "01"), "U64"}, {tagged("U64", "-1"), "U64"},
		{tagged("U64", int64(1)), "U64"}, {tagged("U64", ""), "U64"}, {tagged("U8", "256"), "U8"},
		{tagged("U64", "1"), "U8"},             // wrong tag for the type
		{map[string]any{"type": "U64"}, "U64"}, // missing value
		{map[string]any{"type": "U64", "value": "1", "x": "y"}, "U64"},
		{tagged("Result<Bytes,SelectError>", map[string]any{"err": "NOPE"}), "Result<Bytes,SelectError>"},
		{tagged("Result<Bytes,SelectError>", map[string]any{"err": int64(1)}), "Result<Bytes,SelectError>"},
		{tagged("Result<Bytes,SelectError>", map[string]any{"ok": tagged("Bytes", "00"), "err": "X"}), "Result<Bytes,SelectError>"},
		{tagged("Result<Bytes,SelectError>", "str"), "Result<Bytes,SelectError>"},
		{tagged("Result<Bytes,Other>", map[string]any{"err": "X"}), "Result<Bytes,SelectError>"},
	}
	for _, c := range bad {
		c := c
		if code := laneCode(t, func() { decodeJSON(c.raw, parseType(c.ty), selectErrors) }); code != "value" {
			t.Errorf("%v as %s: code %q, want value", c.raw, c.ty, code)
		}
	}
}

func TestLowerAndLift(t *testing.T) {
	original := []byte{1, 2, 3}
	copied := lower(original, parseType("Bytes")).([]byte)
	copied[0] = 9
	if original[0] != 1 || cap(copied) != len(copied) {
		t.Fatal("lower must hand out a fresh copy with no spare capacity")
	}
	if lower(uint64(200), parseType("U8")) != uint8(200) || lower(uint64(5), parseType("U64")) != uint64(5) || lower(true, parseType("Bool")) != true {
		t.Fatal("lower scalars")
	}
	if lift(uint8(200), parseType("U8")) != uint64(200) || lift(uint64(5), parseType("Nat")) != uint64(5) {
		t.Fatal("lift scalars")
	}
	result := parseType("Result<U8,SelectError>")
	if got := lift(Pair{"ok", uint8(7)}, result); !valuesEqual(got, Pair{"ok", uint64(7)}) {
		t.Fatalf("lift ok: %v", got)
	}
	if got := lift(Pair{"err", "E"}, result); !valuesEqual(got, Pair{"err", "E"}) {
		t.Fatalf("lift err: %v", got)
	}
}

type namedBytes []byte

func TestCheckValueIsExact(t *testing.T) {
	result := parseType("Result<Bytes,SelectError>")
	good := []struct {
		value any
		ty    *Type
	}{
		{true, parseType("Bool")}, {[]byte{1}, parseType("Bytes")}, {[]byte(nil), parseType("Bytes")},
		{uint8(3), parseType("U8")}, {uint64(3), parseType("U64")}, {uint64(3), parseType("Nat")},
		{Pair{"ok", []byte{1}}, result}, {Pair{"err", "CT_SELECT_LENGTH_MISMATCH"}, result},
	}
	for _, c := range good {
		if !checkValue(c.value, c.ty, selectErrors) {
			t.Errorf("%v should be a %s", c.value, c.ty.Text)
		}
	}
	bad := []struct {
		value any
		ty    *Type
	}{
		{1, parseType("Bool")}, {nil, parseType("Bool")}, {"true", parseType("Bool")},
		{namedBytes{1}, parseType("Bytes")}, {"abc", parseType("Bytes")}, {nil, parseType("Bytes")}, {[]int{1}, parseType("Bytes")},
		{3, parseType("U8")}, {uint64(3), parseType("U8")}, {uint16(3), parseType("U8")},
		{1, parseType("U64")}, {int64(1), parseType("U64")}, {uint32(1), parseType("U64")}, {uint8(1), parseType("Nat")}, {float64(1), parseType("U64")},
		{Pair{"ok", true}, result}, {Pair{"err", "NOPE"}, result}, {Pair{"err", 5}, result}, {Pair{"maybe", nil}, result}, {[]byte{1}, result}, {nil, result},
		{nil, &Type{Kind: "Option"}},
	}
	for _, c := range bad {
		if checkValue(c.value, c.ty, selectErrors) {
			t.Errorf("%T(%v) must not be a %s", c.value, c.value, c.ty.Text)
		}
	}
}

func TestStreamEncodingBytes(t *testing.T) {
	// Pin the warrant-stream-v1 bytes by hashing a hand-built stream: the digest
	// of the encoded bytes must equal the digest the hasher reports.
	prefix := streamPrefix("s", []string{"Bool", "U64"}, "Result<Bytes,SelectError>")
	want := []byte("warrant-stream-v1\x00")
	want = append(want, 1, 0, 0, 0, 's', 2, 0, 0, 0, 4, 0, 0, 0)
	want = append(want, "Bool"...)
	want = append(want, 3, 0, 0, 0)
	want = append(want, "U64"...)
	want = append(want, 25, 0, 0, 0)
	want = append(want, "Result<Bytes,SelectError>"...)
	if string(prefix) != string(want) {
		t.Fatalf("prefix\n%x\n%x", prefix, want)
	}
	body := []byte{1, 0x10, 5, 0, 0, 0, 0, 0, 0, 0, 0x50, 0x20, 2, 0, 0, 0, 0xaa, 0xbb, 0x51, 3, 0, 0, 0, 'E', 'R', 'R', 0}
	hasher := newStreamHasher(prefix)
	hasher.write(true, parseType("Bool"))
	hasher.write(uint64(5), parseType("U64"))
	hasher.write(Pair{"ok", []byte{0xaa, 0xbb}}, parseType("Result<Bytes,SelectError>"))
	hasher.write(Pair{"err", "ERR"}, parseType("Result<Bytes,SelectError>"))
	hasher.write(false, parseType("Bool"))
	if got, want := hasher.hexdigest(), sha256Hex(append(append([]byte(nil), prefix...), body...)); got != want {
		t.Fatalf("digest %s, want %s", got, want)
	}
}

func TestStreamHasherChunking(t *testing.T) {
	// Writing more than one 64 KiB chunk, including a single value larger than a
	// chunk, must hash the same bytes as hashing them in one piece.
	prefix := []byte("p")
	big := make([]byte, hasherChunk+100)
	for i := range big {
		big[i] = byte(i)
	}
	hasher := newStreamHasher(prefix)
	var all []byte
	all = append(all, prefix...)
	bytesType := parseType("Bytes")
	for i := 0; i < 5000; i++ {
		hasher.write([]byte{byte(i), 1}, bytesType)
		all = append(all, 0x20, 2, 0, 0, 0, byte(i), 1)
	}
	hasher.write(big, bytesType)
	all = append(all, 0x20, byte(len(big)), byte(len(big)>>8), byte(len(big)>>16), 0)
	all = append(all, big...)
	hasher.write(uint64(1), parseType("U8"))
	all = append(all, 0x10, 1, 0, 0, 0, 0, 0, 0, 0)
	if hasher.hexdigest() != sha256Hex(all) {
		t.Fatal("chunked hashing differs from one-shot hashing")
	}
}

func tfvInt(text string) TfvValue { return TfvValue{Kind: "int", Text: text} }

func TestDecodeTFV(t *testing.T) {
	result := parseType("Result<Bytes,SelectError>")
	if decodeTFV(TfvValue{Kind: "bool", Bool: true}, parseType("Bool"), nil) != true {
		t.Fatal("bool")
	}
	if got := decodeTFV(TfvValue{Kind: "bytes", Bytes: []byte{1}}, parseType("Bytes<1>"), nil); !valuesEqual(got, []byte{1}) {
		t.Fatal("bytes")
	}
	if decodeTFV(tfvInt("18446744073709551615"), parseType("U64"), nil) != ^uint64(0) {
		t.Fatal("u64")
	}
	okMap := TfvValue{Kind: "map", Pairs: [][2]TfvValue{{{Kind: "text", Text: "ok"}, {Kind: "bytes", Bytes: []byte{7}}}}}
	if got := decodeTFV(okMap, result, selectErrors); !valuesEqual(got, Pair{"ok", []byte{7}}) {
		t.Fatalf("ok map: %v", got)
	}
	errMap := TfvValue{Kind: "map", Pairs: [][2]TfvValue{{{Kind: "text", Text: "err"}, {Kind: "text", Text: "CT_SELECT_LENGTH_MISMATCH"}}}}
	if got := decodeTFV(errMap, result, selectErrors); !valuesEqual(got, Pair{"err", "CT_SELECT_LENGTH_MISMATCH"}) {
		t.Fatalf("err map: %v", got)
	}
	bad := []struct {
		raw TfvValue
		ty  string
	}{
		{TfvValue{Kind: "int"}, "Bool"}, {TfvValue{Kind: "bool"}, "Bytes"},
		{TfvValue{Kind: "bytes", Bytes: []byte{1}}, "Bytes<2>"},
		{tfvInt("-1"), "U64"}, {tfvInt("256"), "U8"}, {tfvInt("18446744073709551616"), "U64"}, {TfvValue{Kind: "bool"}, "U64"},
		{TfvValue{Kind: "map"}, "Result<Bytes,SelectError>"},
		{TfvValue{Kind: "map", Pairs: [][2]TfvValue{{{Kind: "bytes"}, {Kind: "bytes"}}}}, "Result<Bytes,SelectError>"},
		{TfvValue{Kind: "map", Pairs: [][2]TfvValue{{{Kind: "text", Text: "err"}, {Kind: "text", Text: "NOPE"}}}}, "Result<Bytes,SelectError>"},
		{TfvValue{Kind: "map", Pairs: [][2]TfvValue{{{Kind: "text", Text: "other"}, {Kind: "bytes"}}}}, "Result<Bytes,SelectError>"},
	}
	for i, c := range bad {
		c := c
		if code := laneCode(t, func() { decodeTFV(c.raw, parseType(c.ty), selectErrors) }); code != "tfv-value" {
			t.Errorf("bad[%d]: code %q", i, code)
		}
	}
}

func TestFromHexRefusesBadInput(t *testing.T) {
	if string(fromHex("")) != "" || string(fromHex("6162")) != "ab" {
		t.Fatal("fromHex")
	}
	wantCode(t, "value", func() { fromHex("zz") })
	if !strings.Contains(string(streamPrefix("x", nil, "Bool")), "warrant-stream-v1") {
		t.Fatal("prefix header")
	}
}
