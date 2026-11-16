package warrantcorpus

// Run every warrant-corpus-conformance-v1 case against this lane (WD-100,
// VER04i 9.1). The fixture holds the behaviours every lane's runner
// re-discovered in review: strict JSON, canonical output, tagged values, path
// containment and the label digest. Each test runs one category through THIS
// lane's own functions and compares with the expected outcome stored in the
// fixture. Failure messages name the case id and never print the input bytes.

import (
	"bytes"
	"encoding/hex"
	"os"
	"path/filepath"
	"testing"
)

const conformanceSchema = "warrant-corpus-conformance-v1"

var conformanceCategories = []string{"json", "tagged", "containment", "label-digest"}

func conformanceDir(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	for {
		candidate := filepath.Join(dir, "code", "specs", "fixtures", conformanceSchema)
		if info, err := os.Stat(candidate); err == nil && info.IsDir() {
			return candidate
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("cannot find the conformance fixture above the test directory")
		}
		dir = parent
	}
}

// loadFixture reads one fixture file with this lane's own strict reader.
func loadFixture(t *testing.T, name string) map[string]any {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(conformanceDir(t), name))
	if err != nil {
		t.Fatal(err)
	}
	document, ok := parseJSON(data).(map[string]any)
	if !ok {
		t.Fatalf("%s is not an object", name)
	}
	return document
}

func fixtureCases(t *testing.T, category string) []map[string]any {
	t.Helper()
	file := loadFixture(t, category+".json")
	if file["schema"] != conformanceSchema || file["category"] != category {
		t.Fatalf("%s: wrong schema or category", category)
	}
	var cases []map[string]any
	for _, item := range file["cases"].([]any) {
		cases = append(cases, item.(map[string]any))
	}
	return cases
}

func caseHex(t *testing.T, c map[string]any) []byte {
	t.Helper()
	data, err := hex.DecodeString(c["input_hex"].(string))
	if err != nil {
		t.Fatalf("%v: bad input_hex", c["id"])
	}
	return data
}

// errorClass runs f and returns the lane error code it refused with, or "".
func errorClass(t *testing.T, f func()) string {
	t.Helper()
	err := guard(f)
	if err == nil {
		return ""
	}
	laneErr, ok := err.(*LaneError)
	if !ok {
		t.Fatalf("internal error: %v", err)
	}
	return laneErr.Code
}

func TestConformanceIndexMatchesTheFiles(t *testing.T) {
	index := loadFixture(t, "index.json")
	if index["schema"] != conformanceSchema {
		t.Fatal("wrong index schema")
	}
	rows := map[string]map[string]any{}
	for _, item := range index["files"].([]any) {
		row := item.(map[string]any)
		rows[row["category"].(string)] = row
	}
	total := int64(0)
	for _, category := range conformanceCategories {
		row, ok := rows[category]
		if !ok {
			t.Fatalf("index has no %s row", category)
		}
		data, err := os.ReadFile(filepath.Join(conformanceDir(t), row["path"].(string)))
		if err != nil {
			t.Fatal(err)
		}
		if sha256Hex(data) != row["sha256"] {
			t.Errorf("%s: sha256 moved", category)
		}
		count := int64(len(fixtureCases(t, category)))
		if count != row["cases"] {
			t.Errorf("%s: case count %d, index says %v", category, count, row["cases"])
		}
		total += count
	}
	if len(rows) != len(conformanceCategories) || total != index["total_cases"] {
		t.Errorf("index total %v, ran %d", index["total_cases"], total)
	}
}

func TestConformanceJSON(t *testing.T) {
	for _, c := range fixtureCases(t, "json") {
		id, data := c["id"].(string), caseHex(t, c)
		expect := c["expect"].(map[string]any)
		t.Run(id, func(t *testing.T) {
			var text string
			class := errorClass(t, func() { text = renderJSON(parseJSON(data)) })
			if expect["outcome"] == "reject" {
				if class != expect["error"] {
					t.Fatalf("error class %q, want %q", class, expect["error"])
				}
				return
			}
			if class != "" {
				t.Fatalf("refused with %q, want accept", class)
			}
			if sha256Hex([]byte(text)) != expect["canonical_sha256"] {
				t.Errorf("canonical rendering differs (sha256)")
			}
			if want, has := expect["canonical"]; has && text != want {
				t.Errorf("canonical text: got %q, want %q", text, want)
			}
		})
	}
}

// errorTypesOf reads a case's optional `errors` object (type name to variants).
func errorTypesOf(c map[string]any) ErrorTypes {
	out := ErrorTypes{}
	if raw, ok := c["errors"].(map[string]any); ok {
		for name, variants := range raw {
			for _, variant := range variants.([]any) {
				out[name] = append(out[name], variant.(string))
			}
		}
	}
	return out
}

// recorder is a hash.Hash that keeps what it is fed, so the lane's own encoder
// can be asked for the exact warrant-stream-v1 bytes of one value.
type recorder struct{ bytes.Buffer }

func (r *recorder) Sum([]byte) []byte { return nil }
func (r *recorder) Reset()            { r.Buffer.Reset() }
func (r *recorder) Size() int         { return 0 }
func (r *recorder) BlockSize() int    { return 1 }

// encodeStreamBytes is the warrant-stream-v1 encoding of one value (no prefix).
func encodeStreamBytes(value any, ty *Type) []byte {
	sink := &recorder{}
	hasher := newStreamHasher(nil)
	hasher.h = sink
	hasher.write(value, ty)
	hasher.flush()
	return sink.Bytes()
}

func TestConformanceTagged(t *testing.T) {
	for _, c := range fixtureCases(t, "tagged") {
		id, data := c["id"].(string), caseHex(t, c)
		expect := c["expect"].(map[string]any)
		typeText, errorTypes := c["type"].(string), errorTypesOf(c)
		t.Run(id, func(t *testing.T) {
			var stream []byte
			class := errorClass(t, func() {
				ty := parseType(typeText)
				stream = encodeStreamBytes(decodeJSON(parseJSON(data), ty, errorTypes), ty)
			})
			if expect["outcome"] == "reject" {
				if class != expect["error"] {
					t.Fatalf("error class %q, want %q", class, expect["error"])
				}
				return
			}
			if class != "" {
				t.Fatalf("refused with %q, want accept", class)
			}
			if hex.EncodeToString(stream) != expect["stream_hex"] {
				t.Errorf("stream bytes differ")
			}
			if sha256Hex(stream) != expect["stream_sha256"] {
				t.Errorf("stream sha256 differs")
			}
		})
	}
}

func TestConformanceContainment(t *testing.T) {
	for _, c := range fixtureCases(t, "containment") {
		expect := c["expect"].(map[string]any)
		t.Run(c["id"].(string), func(t *testing.T) {
			rel, inside := relativeInside(c["root"].(string), c["child"].(string), c["flavor"].(string))
			if inside != expect["inside"] {
				t.Errorf("inside = %v, want %v", inside, expect["inside"])
			}
			if (inside && rel == "") != expect["same"] {
				t.Errorf("same = %v, want %v", inside && rel == "", expect["same"])
			}
		})
	}
}

func TestConformanceLabelDigest(t *testing.T) {
	cases := fixtureCases(t, "label-digest")
	digests := map[string]string{}
	for _, c := range cases {
		digests[c["id"].(string)] = manifestLabelDigest(c["manifest"].(map[string]any))
	}
	for _, c := range cases {
		id := c["id"].(string)
		t.Run(id, func(t *testing.T) {
			if digests[id] != c["sha256"] {
				t.Errorf("digest differs")
			}
			if other, has := c["same_as"].(string); has && digests[id] != digests[other] {
				t.Errorf("must equal %s", other)
			}
			if other, has := c["differs_from"].(string); has && digests[id] == digests[other] {
				t.Errorf("must differ from %s", other)
			}
		})
	}
}

// Production calls the containment helper with the host flavour, so the device
// and `//` rules apply there too.
func TestProductionContainmentUsesTheHostFlavour(t *testing.T) {
	saved := hostFlavor
	defer func() { hostFlavor = saved }()
	hostFlavor = flavorWin32
	if rel, ok := relativeInside(`D:\repo`, `D:\repo\specs`, hostFlavor); !ok || rel != "specs" {
		t.Errorf("win32 inside: (%q, %v)", rel, ok)
	}
	for _, child := range []string{`\/?\D:\repo\x`, `//?/D:/repo/x`, `\\?\UNC\s\h\x`, `C:\tmp`} {
		if _, ok := relativeInside(`D:\repo`, child, hostFlavor); ok {
			t.Errorf("win32 accepted %s", child)
		}
	}
	hostFlavor = flavorPosix
	for _, child := range []string{"//repo/x", "/rr"} {
		if _, ok := relativeInside("/repo", child, hostFlavor); ok {
			t.Errorf("posix accepted %s", child)
		}
	}
}

func TestLexicalPathRefusesAnUnknownFlavor(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("expected a panic")
		}
	}()
	relativeInside("/r", "/r/x", "amiga")
}
