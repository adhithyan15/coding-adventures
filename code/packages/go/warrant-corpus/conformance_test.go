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
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
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

// The fixture family grows by categories (VER04r section 11, [D40]). Every
// category this lane knows must have a row; a row it does not know is accepted
// only when its name starts with "v2-", and then only digest-checked, never run
// (this runner does not read v2 yet). Rows are ordered: the four v1 rows in
// their present order, then the v2 rows, the known ones in the kit's order.
var conformanceV2Order = []string{"v2-value", "v2-arith", "v2-ir", "v2-domain", "v2-chunk", "v2-fault", "v2-binding"}

// checkConformanceIndex validates dir/index.json against the files beside it
// and returns the first violation, or nil, plus the number of rows checked.
func checkConformanceIndex(dir string) (rows int, err error) {
	fail := func(format string, args ...any) (int, error) { return 0, fmt.Errorf(format, args...) }
	read := func(name string) (map[string]any, []byte, error) {
		data, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			return nil, nil, err
		}
		var document map[string]any
		if guardErr := guard(func() { document, _ = parseJSON(data).(map[string]any) }); guardErr != nil || document == nil {
			return nil, nil, fmt.Errorf("%s is not a strict JSON object", name)
		}
		return document, data, nil
	}
	index, _, err := read("index.json")
	if err != nil {
		return 0, err
	}
	if index["schema"] != conformanceSchema {
		return fail("wrong index schema")
	}
	items, _ := index["files"].([]any)
	var names []string
	seen := map[string]bool{}
	for _, item := range items {
		row, _ := item.(map[string]any)
		name, _ := row["category"].(string)
		if seen[name] {
			return fail("duplicate category row %q", name)
		}
		seen[name] = true
		names = append(names, name)
	}
	if len(names) < len(conformanceCategories) {
		return fail("v1 rows missing")
	}
	for i, category := range conformanceCategories {
		if names[i] != category {
			return fail("v1 rows missing or out of order")
		}
	}
	last := -1
	for _, name := range names[len(conformanceCategories):] {
		if !strings.HasPrefix(name, "v2-") {
			return fail("unknown category row %q", name)
		}
		for position, known := range conformanceV2Order {
			if known == name {
				if position < last {
					return fail("v2 rows out of order")
				}
				last = position
			}
		}
	}
	total := int64(0)
	for _, item := range items {
		row := item.(map[string]any)
		category, _ := row["category"].(string)
		path, _ := row["path"].(string)
		// The category name is safe to splice into a file name: non-empty, no
		// separators and no "..". Every row's path is then exactly <category>.json.
		if category == "" || strings.ContainsAny(category, `/\`) || strings.Contains(category, "..") {
			return fail("%q: unsafe category name", category)
		}
		if path != category+".json" {
			return fail("%s: bad path", category)
		}
		body, data, err := read(path)
		if err != nil {
			if os.IsNotExist(err) {
				return fail("%s: file missing", category)
			}
			return fail("%s: %v", category, err)
		}
		if sha256Hex(data) != row["sha256"] {
			return fail("%s: sha256 moved", category)
		}
		if body["category"] != category {
			return fail("%s: wrong category in file", category)
		}
		cases, _ := body["cases"].([]any)
		if int64(len(cases)) != row["cases"] {
			return fail("%s: case count moved", category)
		}
		total += int64(len(cases))
	}
	if total != index["total_cases"] {
		return fail("total_cases is not the sum of the rows")
	}
	return len(items), nil
}

func TestConformanceIndexMatchesTheFiles(t *testing.T) {
	rows, err := checkConformanceIndex(conformanceDir(t))
	if err != nil {
		t.Fatal(err)
	}
	if rows < len(conformanceCategories) {
		t.Fatalf("only %d rows", rows)
	}
	index := loadFixture(t, "index.json")
	for _, item := range index["files"].([]any) {
		row := item.(map[string]any)
		for _, category := range conformanceCategories {
			if row["category"] == category && int64(len(fixtureCases(t, category))) != row["cases"] {
				t.Errorf("%s: case count moved", category)
			}
		}
	}
}

// scratchConformance copies the fixture to a temp dir, then appends extra index
// rows and writes extra files.
func scratchConformance(t *testing.T, extraRows []map[string]any, files map[string][]byte) string {
	t.Helper()
	src, dst := conformanceDir(t), t.TempDir()
	entries, err := os.ReadDir(src)
	if err != nil {
		t.Fatal(err)
	}
	for _, entry := range entries {
		data, err := os.ReadFile(filepath.Join(src, entry.Name()))
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dst, entry.Name()), data, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	raw, err := os.ReadFile(filepath.Join(dst, "index.json"))
	if err != nil {
		t.Fatal(err)
	}
	var document map[string]any
	if err := json.Unmarshal(raw, &document); err != nil {
		t.Fatal(err)
	}
	total := document["total_cases"].(float64)
	rows := document["files"].([]any)
	for _, row := range extraRows {
		rows = append(rows, row)
		if n, ok := row["cases"].(int); ok {
			total += float64(n)
		}
	}
	document["files"], document["total_cases"] = rows, total
	out, err := json.MarshalIndent(document, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dst, "index.json"), append(out, '\n'), 0o644); err != nil {
		t.Fatal(err)
	}
	for name, data := range files {
		if err := os.WriteFile(filepath.Join(dst, name), data, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return dst
}

// extraConformanceFile is a minimal category file and its index row.
func extraConformanceFile(t *testing.T, category string, count int) (map[string]any, []byte) {
	t.Helper()
	cases := make([]any, count)
	for i := range cases {
		cases[i] = map[string]any{"id": fmt.Sprintf("%s/%d", category, i)}
	}
	data, err := json.MarshalIndent(map[string]any{"schema": conformanceSchema, "category": category, "cases": cases}, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	data = append(data, '\n')
	return map[string]any{"cases": count, "category": category, "path": category + ".json", "sha256": sha256Hex(data)}, data
}

func TestConformanceIndexToleratesDigestCheckedV2Rows(t *testing.T) {
	row, data := extraConformanceFile(t, "v2-x", 2)
	rows, err := checkConformanceIndex(scratchConformance(t, []map[string]any{row}, map[string][]byte{"v2-x.json": data}))
	if err != nil || rows != len(conformanceCategories)+1 {
		t.Fatalf("rows %d, err %v", rows, err)
	}
	value, valueData := extraConformanceFile(t, "v2-value", 1)
	arith, arithData := extraConformanceFile(t, "v2-arith", 1)
	files := map[string][]byte{"v2-value.json": valueData, "v2-arith.json": arithData}
	if _, err := checkConformanceIndex(scratchConformance(t, []map[string]any{value, arith}, files)); err != nil {
		t.Fatalf("kit order refused: %v", err)
	}
}

func TestConformanceIndexRefusesBadExtraRows(t *testing.T) {
	good, goodData := extraConformanceFile(t, "v2-x", 2)
	other, otherData := extraConformanceFile(t, "extra", 2)
	badDigest, badCount := map[string]any{}, map[string]any{}
	for k, v := range good {
		badDigest[k], badCount[k] = v, v
	}
	badDigest["sha256"] = strings.Repeat("0", 64)
	badCount["cases"] = 3
	badPath, boolCount := map[string]any{}, map[string]any{}
	for k, v := range good {
		badPath[k], boolCount[k] = v, v
	}
	badPath["path"] = "v2-y.json"
	boolCount["cases"] = true
	value, valueData := extraConformanceFile(t, "v2-value", 1)
	arith, arithData := extraConformanceFile(t, "v2-arith", 1)
	tests := []struct {
		name  string
		rows  []map[string]any
		files map[string][]byte
		want  string
	}{
		{"unknown non-v2 row", []map[string]any{other}, map[string][]byte{"extra.json": otherData}, "unknown category"},
		{"v2 row with a wrong digest", []map[string]any{badDigest}, map[string][]byte{"v2-x.json": goodData}, "sha256"},
		{"v2 row with a wrong case count", []map[string]any{badCount}, map[string][]byte{"v2-x.json": goodData}, "case count"},
		{"v2 row whose path is not <category>.json", []map[string]any{badPath}, map[string][]byte{"v2-x.json": goodData, "v2-y.json": goodData}, "bad path"},
		{"v2 row with a boolean case count", []map[string]any{boolCount}, map[string][]byte{"v2-x.json": goodData}, "case count"},
		{"v2 row with a missing file", []map[string]any{good}, nil, "file missing"},
		{"v2 rows out of order", []map[string]any{arith, value}, map[string][]byte{"v2-value.json": valueData, "v2-arith.json": arithData}, "out of order"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			_, err := checkConformanceIndex(scratchConformance(t, tc.rows, tc.files))
			if err == nil || !strings.Contains(err.Error(), tc.want) {
				t.Fatalf("got %v, want an error containing %q", err, tc.want)
			}
		})
	}
}

func TestConformanceIndexRefusesAMissingV1Row(t *testing.T) {
	dir := scratchConformance(t, nil, nil)
	path := filepath.Join(dir, "index.json")
	raw, _ := os.ReadFile(path)
	var document map[string]any
	if err := json.Unmarshal(raw, &document); err != nil {
		t.Fatal(err)
	}
	rows := document["files"].([]any)
	document["files"] = rows[:len(rows)-1]
	out, _ := json.Marshal(document)
	if err := os.WriteFile(path, out, 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := checkConformanceIndex(dir); err == nil || !strings.Contains(err.Error(), "v1 rows") {
		t.Fatalf("got %v", err)
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
