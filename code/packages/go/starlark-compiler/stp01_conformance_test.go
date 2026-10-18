package starlarkcompiler

import (
	"bytes"
	"encoding/json"
	"os"
	"reflect"
	"strings"
	"testing"
)

const stpFixture = "../../../specs/fixtures/starlark-opcode-provider-v1/cases.json"

func stpFixtureBytes(t *testing.T) []byte {
	t.Helper()
	f, err := os.Open(stpFixture)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	b, err := stpRead(f)
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func TestSTP01SharedCorpus(t *testing.T) {
	c, err := stpDecode(stpFixtureBytes(t))
	if err != nil {
		t.Fatal(err)
	}
	counts, err := stpCompare(c)
	if err != nil {
		t.Fatal(err)
	}
	if counts != [5]int{61, 256, 34, 8, 195} {
		t.Fatalf("visits/holes %v", counts)
	}
}

// Mutations are transported through JSON again: parsing and provider comparison
// are separate gates, so a fixture cannot teach the provider new expected data.
func stpMutate(t *testing.T, change func(map[string]any)) []byte {
	t.Helper()
	var doc map[string]any
	if err := json.Unmarshal(stpFixtureBytes(t), &doc); err != nil {
		t.Fatal(err)
	}
	change(doc)
	b, err := json.Marshal(doc)
	if err != nil {
		t.Fatal(err)
	}
	return b
}

func TestSTP01TransportRejectsMalformed(t *testing.T) {
	b := stpFixtureBytes(t)
	for name, bad := range map[string][]byte{
		"duplicate-root":   bytes.Replace(b, []byte(`"schema_version": 1`), []byte(`"schema_version": 1,"schema_version":1`), 1),
		"escaped-key":      bytes.Replace(b, []byte(`"profile":`), []byte(`"pro\u0066ile":"standalone-provider-v1","profile":`), 1),
		"nested-duplicate": bytes.Replace(b, []byte(`"byte": 1`), []byte(`"byte":1,"byte":1`), 1),
		"trailing":         append(append([]byte{}, b...), []byte(` {}`)...),
		"bom":              append([]byte{0xef, 0xbb, 0xbf}, b...),
		"utf8":             bytes.Replace(b, []byte("LoadConst"), []byte{0xff}, 1),
		"lone-high":        bytes.Replace(b, []byte("LoadConst"), []byte(`\ud800`), 1),
		"lone-low":         bytes.Replace(b, []byte("LoadConst"), []byte(`\udc00`), 1),
		"reversed":         bytes.Replace(b, []byte("LoadConst"), []byte(`\udc00\ud800`), 1),
		"nonadjacent":      bytes.Replace(b, []byte("LoadConst"), []byte(`\ud800x\udc00`), 1),
		"oversize":         bytes.Repeat([]byte(" "), 131073),
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := stpDecode(bad); err == nil {
				t.Fatal("accepted malformed transport")
			}
		})
	}
	for _, number := range []string{"true", "1.0", "1e0", "-1", "256", "32768", "65535", "99999999999999999999999999999999", "NaN"} {
		t.Run("byte-"+number, func(t *testing.T) {
			bad := bytes.Replace(b, []byte(`"byte": 1`), []byte(`"byte": `+number), 1)
			if _, err := stpDecode(bad); err == nil {
				t.Fatal("accepted invalid byte")
			}
		})
	}
	for _, key := range []string{"schema_version", "profile", "opcodes", "decode", "operators", "unknown_lookups"} {
		for _, missing := range []bool{false, true} {
			bad := stpMutate(t, func(d map[string]any) {
				if missing {
					delete(d, key)
				} else {
					d[key] = nil
				}
			})
			if _, err := stpDecode(bad); err == nil {
				t.Fatalf("accepted missing/null %s", key)
			}
		}
	}
	for name, change := range map[string]func(map[string]any){
		"extra":        func(d map[string]any) { d["extension"] = 0 },
		"version":      func(d map[string]any) { d["schema_version"] = 2 },
		"profile":      func(d map[string]any) { d["profile"] = "future" },
		"wrong-row":    func(d map[string]any) { d["opcodes"].([]any)[0] = true },
		"missing-byte": func(d map[string]any) { delete(d["opcodes"].([]any)[0].(map[string]any), "byte") },
		"null-symbol":  func(d map[string]any) { d["opcodes"].([]any)[0].(map[string]any)["symbol"] = nil },
		"missing-null": func(d map[string]any) { delete(d["decode"].([]any)[0].(map[string]any), "symbol") },
		"half-null":    func(d map[string]any) { d["decode"].([]any)[0].(map[string]any)["symbol"] = "Halt" },
		"groups":       func(d map[string]any) { d["operators"].(map[string]any)["Binary"] = []any{} },
		"operator": func(d map[string]any) {
			d["operators"].(map[string]any)["binary"].([]any)[0].(map[string]any)["operator"] = strings.Repeat("x", 33)
		},
		"symbol-limit":   func(d map[string]any) { d["opcodes"].([]any)[0].(map[string]any)["symbol"] = strings.Repeat("x", 65) },
		"remove":         func(d map[string]any) { d["opcodes"] = d["opcodes"].([]any)[1:] },
		"reorder":        func(d map[string]any) { a := d["opcodes"].([]any); a[0], a[1] = a[1], a[0] },
		"duplicate":      func(d map[string]any) { a := d["opcodes"].([]any); a[1] = a[0] },
		"unknown-roster": func(d map[string]any) { d["unknown_lookups"].([]any)[0].(map[string]any)["operator"] = "+" },
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := stpDecode(stpMutate(t, change)); err == nil {
				t.Fatal("accepted invalid shape")
			}
		})
	}
}

func TestSTP01SemanticCorruption(t *testing.T) {
	for name, change := range map[string]func(map[string]any){
		"name":     func(d map[string]any) { d["opcodes"].([]any)[0].(map[string]any)["symbol"] = "Bogus" },
		"category": func(d map[string]any) { d["opcodes"].([]any)[0].(map[string]any)["category"] = "module" },
		"hole": func(d map[string]any) {
			r := d["decode"].([]any)[0].(map[string]any)
			r["symbol"] = "InventedHole"
			r["category"] = "vm_control"
		},
		"operator": func(d map[string]any) {
			d["operators"].(map[string]any)["binary"].([]any)[0].(map[string]any)["symbol"] = "Sub"
		},
		"coordinated": func(d map[string]any) {
			for _, group := range []string{"opcodes", "decode"} {
				for _, v := range d[group].([]any) {
					r := v.(map[string]any)
					if r["symbol"] == "Add" {
						r["symbol"] = "Counterfeit"
					}
				}
			}
			for _, v := range d["operators"].(map[string]any) {
				for _, v := range v.([]any) {
					r := v.(map[string]any)
					if r["symbol"] == "Add" {
						r["symbol"] = "Counterfeit"
					}
				}
			}
		},
	} {
		t.Run(name, func(t *testing.T) {
			c, err := stpDecode(stpMutate(t, change))
			if err != nil {
				t.Fatalf("semantic mutation should reach actual API comparison: %v", err)
			}
			if _, err = stpCompare(c); err == nil {
				t.Fatal("provider accepted semantic corruption")
			}
		})
	}
}

func TestSTP01ReaderBoundaries(t *testing.T) {
	for _, good := range []string{`"\ud83d\ude00"`, `"�"`, `"\\ud800"`, `{"a":1}`, `[[[[[[[[]]]]]]]]`} {
		if _, err := stpJSON([]byte(good)); err != nil {
			t.Fatalf("valid %q: %v", good, err)
		}
	}
	for _, bad := range []string{`[[[[[[[[[]]]]]]]]]`, `{"a":1,"\u0061":2}`, `"\ud800"`, `"\udc00"`, `"\ud800\u1234"`, `"\u12"`, `"line` + "\n" + `break"`, `[1,]`, `{} false`} {
		if _, err := stpJSON([]byte(bad)); err == nil {
			t.Fatalf("accepted %q", bad)
		}
	}
	boundary := append([]byte(`{}`), bytes.Repeat([]byte(" "), 131070)...)
	if _, err := stpJSON(boundary); err != nil {
		t.Fatal(err)
	}
	if _, err := stpRead(bytes.NewReader(boundary)); err != nil {
		t.Fatal(err)
	}
	if _, err := stpRead(bytes.NewReader(append(boundary, ' '))); err == nil {
		t.Fatal("unbounded read")
	}
	b := stpFixtureBytes(t)
	if _, err := stpDecode(bytes.Replace(b, []byte(`"byte": 0`), []byte(`"byte": -0`), 1)); err != nil {
		t.Fatal(err)
	}
	if _, err := stpDecode(append([]byte(" \n\t"), b...)); err != nil {
		t.Fatal(err)
	}
}

func TestSTP01OwnedResults(t *testing.T) {
	canonical, other, changed := append([]Op(nil), AllOps()...), AllOps(), AllOps()
	changed[0] = Halt
	for i, j := 0, len(changed)-1; i < j; i, j = i+1, j-1 {
		changed[i], changed[j] = changed[j], changed[i]
	}
	changed = append(changed[:1], Add, Sub)
	if !reflect.DeepEqual(other, canonical) || !reflect.DeepEqual(AllOps(), canonical) {
		t.Fatal("AllOps storage escaped")
	}
	for _, api := range stpGroups() {
		canonical := map[string]Op{}
		for key, value := range api.makeMap() {
			canonical[key] = value
		}
		other, changed := api.makeMap(), api.makeMap()
		keys := make([]string, 0, len(changed))
		for k := range changed {
			keys = append(keys, k)
		}
		delete(changed, keys[0])
		changed[keys[1]] = Halt
		changed["unknown-owned"] = Halt
		if !reflect.DeepEqual(other, canonical) || !reflect.DeepEqual(api.makeMap(), canonical) {
			t.Fatal("map storage escaped")
		}
		for key, want := range canonical {
			if got, ok := api.lookup(key); !ok || got != want {
				t.Fatalf("direct lookup changed %s", key)
			}
		}
		if _, ok := api.lookup("unknown-owned"); ok {
			t.Fatal("insert escaped")
		}
	}
}
