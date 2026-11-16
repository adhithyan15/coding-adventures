package warrantcorpus

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func parses(t *testing.T, text string) any {
	t.Helper()
	var value any
	if code := laneCode(t, func() { value = parseJSON([]byte(text)) }); code != "" {
		t.Fatalf("%q: unexpected refusal %s", text, code)
	}
	return value
}

func refuses(t *testing.T, text string) {
	t.Helper()
	if code := laneCode(t, func() { parseJSON([]byte(text)) }); code != "malformed-json" {
		t.Fatalf("%q: code %q, want malformed-json", text, code)
	}
}

func TestParseJSONAcceptsEveryValueKind(t *testing.T) {
	got := parses(t, ` {"a": [1, -2, true, false, null, "x"], "b": {}, "c": []} `).(map[string]any)
	list := got["a"].([]any)
	if list[0] != int64(1) || list[1] != int64(-2) || list[2] != true || list[3] != false || list[4] != nil || list[5] != "x" {
		t.Fatalf("bad list %v", list)
	}
	if len(got["b"].(map[string]any)) != 0 || len(got["c"].([]any)) != 0 {
		t.Fatal("empty containers")
	}
}

func TestParseJSONRejectsDuplicateKeys(t *testing.T) {
	refuses(t, `{"a":1,"a":2}`)
	refuses(t, `{"x":{"a":1,"a":1}}`)
	// Duplicate after an escape: both spell the same key.
	refuses(t, `{"a":1,"a":2}`)
}

func TestParseJSONIntegerRules(t *testing.T) {
	if parses(t, "9007199254740991") != int64(9007199254740991) {
		t.Fatal("2^53-1 must be accepted")
	}
	if parses(t, "-9007199254740991") != int64(-9007199254740991) {
		t.Fatal("-(2^53-1) must be accepted")
	}
	for _, bad := range []string{
		"9007199254740992",     // 2^53
		"9007199254740993",     // would round to 2^53 in a double
		"18446744073709551615", // u64 max
		"99999999999999999999", // far beyond int64
		"-9007199254740992", "-0", "1.5", "1e3", "1E3", "2.0", "-", "+1", "01", "0x10", ".5", "NaN", "Infinity",
	} {
		refuses(t, bad)
	}
}

func TestParseJSONStructuralErrors(t *testing.T) {
	for _, bad := range []string{
		``, `   `, `{`, `[`, `{"a"`, `{"a":`, `{"a":1`, `{"a":1,}`, `[1,]`, `[1 2]`, `{"a" 1}`, `{1:2}`, `{"a":1 "b":2}`,
		`tru`, `nul`, `fals`, `"abc`, `"a\`, `[1]x`, `1 2`, `{} {}`,
	} {
		refuses(t, bad)
	}
}

func TestParseJSONStrings(t *testing.T) {
	if got := parses(t, `"a\"b\\c\/d\b\f\n\r\t"`); got != "a\"b\\c/d\b\f\n\r\t" {
		t.Fatalf("escapes: %q", got)
	}
	if got := parses(t, `"café 😀 é 😀"`); got != "café 😀 é 😀" {
		t.Fatalf("unicode: %q", got)
	}
	// A "__proto__" key is data, as in every lane.
	if _, has := parses(t, `{"__proto__": 1}`).(map[string]any)["__proto__"]; !has {
		t.Fatal("__proto__ must be a plain key")
	}
	for _, bad := range []string{
		"\"a\x01\"", "\"a\nb\"", `"\q"`, `"\u12"`, `"\u12g4"`, `"\u"`,
		`"\ud800"`,       // lone high surrogate
		`"\udc00"`,       // lone low surrogate
		`"\ud800x"`,      // high surrogate followed by text
		`"\ud800A"`,      // high surrogate followed by a non-surrogate escape
		`"\ud800\ud800"`, // two high surrogates
		`"\udc00\ud800"`, // reversed pair
		`"\ud800\`,       // truncated
	} {
		refuses(t, bad)
	}
}

func TestParseJSONRejectsInvalidUTF8(t *testing.T) {
	for _, bad := range []string{"\"\xff\"", "\"\xc3\"", "{\"a\xe2\x82\":1}", "\"\xed\xa0\x80\""} { // lone surrogate encoded as UTF-8
		if code := laneCode(t, func() { parseJSON([]byte(bad)) }); code != "malformed-json" {
			t.Fatalf("%q: %q", bad, code)
		}
	}
	// A BOM is data, not whitespace.
	refuses(t, "\xef\xbb\xbf{}")
}

func nested(depth int) string { return strings.Repeat("[", depth) + strings.Repeat("]", depth) }

func TestParseJSONDepthLimit(t *testing.T) {
	parses(t, nested(maxJSONDepth+1)) // depth counts from 0: 256 levels of nesting below the root
	refuses(t, nested(maxJSONDepth+2))
	refuses(t, strings.Repeat(`{"a":`, maxJSONDepth+2)+"1"+strings.Repeat("}", maxJSONDepth+2))
}

func TestRenderJSONIsCanonical(t *testing.T) {
	document := map[string]any{
		"b":     []any{int64(1), 2, "x", true, nil, map[string]any{}},
		"a":     map[string]any{"z": []any{}, "y": []string{"p", "q"}},
		"c":     []map[string]any{{"k": "v"}},
		"d":     false,
		"quote": "say \"hi\"\\ \b\f\n\r\t\x01\x1f\x7f",
		"wide":  "\u00e9\u20ac\U0001F600\u2028",
	}
	want := `{
  "a": {
    "y": [
      "p",
      "q"
    ],
    "z": []
  },
  "b": [
    1,
    2,
    "x",
    true,
    null,
    {}
  ],
  "c": [
    {
      "k": "v"
    }
  ],
  "d": false,
  "quote": "say \"hi\"\\ \b\f\n\r\t\u0001\u001f` + "\x7f" + `",
  "wide": "\u00e9\u20ac\ud83d\ude00\u2028"
}
`
	if got := renderJSON(document); got != want {
		t.Fatalf("got:\n%s\nwant:\n%s", got, want)
	}
	// Rendering then parsing round-trips and rendering again is byte-identical.
	again := renderJSON(parseJSON([]byte(renderJSON(document))))
	if again != renderJSON(document) {
		t.Fatal("render is not idempotent")
	}
	if got := renderJSON(int64(-7)); got != "-7\n" {
		t.Fatalf("scalar root: %q", got)
	}
}

func TestRenderJSONRefusesAnUnknownType(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("expected a panic")
		}
	}()
	renderJSON(map[string]any{"x": 1.5})
}

func TestReadBytes(t *testing.T) {
	dir := t.TempDir()
	good := filepath.Join(dir, "good.json")
	if err := os.WriteFile(good, []byte("abc"), 0o644); err != nil {
		t.Fatal(err)
	}
	if string(readBytes(good, 3)) != "abc" {
		t.Fatal("read at the limit")
	}
	wantCode(t, "input-limit", func() { readBytes(good, 2) })
	wantCode(t, "io", func() { readBytes(filepath.Join(dir, "missing"), 10) })
	wantCode(t, "io", func() { readBytes(dir, 10) }) // a directory
	link := filepath.Join(dir, "link.json")
	if err := os.Symlink(good, link); err == nil {
		wantCode(t, "io", func() { readBytes(link, 10) }) // never follow a final symlink
	}
}

func TestDecodeUTF8KeepsABOM(t *testing.T) {
	if decodeUTF8([]byte("\xef\xbb\xbfx"), "c") != "\xef\xbb\xbfx" {
		t.Fatal("BOM must be kept as data")
	}
	wantCode(t, "tfv", func() { decodeUTF8([]byte{0xff}, "tfv") })
}

func TestSha256Hex(t *testing.T) {
	if got := sha256Hex([]byte("abc")); got != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" {
		t.Fatalf("sha256(abc) = %s", got)
	}
}
