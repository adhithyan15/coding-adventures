package ftlpagemap

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"testing"
)

const (
	pageMapFirstCaseName     = "publish-rewrite-only-after-ready-and-reuse-fresh-abort"
	reclamationFirstCaseName = "fenced-release-with-pending-reservation"
)

type unicodeFixtureSpelling struct {
	name        string
	replacement string
	want        string
	valid       bool
}

var unicodeFixtureSpellings = []unicodeFixtureSpelling{
	{name: "isolated-high", replacement: `case-\ud800`},
	{name: "isolated-low", replacement: `case-\udc00`},
	{name: "reversed-pair", replacement: `case-\udc00\ud800`},
	{name: "missing-low-half", replacement: `case-\ud800x`},
	{name: "wrong-low-half", replacement: `case-\ud800\u0041`},
	{name: "incomplete-escape", replacement: `case-\ud80`},
	{name: "invalid-hex", replacement: `case-\ud80G`},
	{name: "surrogate-pair", replacement: `case-\ud83d\ude00`, want: "case-😀", valid: true},
	{name: "literal-replacement", replacement: "case-�", want: "case-�", valid: true},
	{name: "escaped-replacement", replacement: `case-\ufffd`, want: "case-�", valid: true},
	{name: "escaped-backslash", replacement: `case-\\uD800`, want: `case-\uD800`, valid: true},
}

func TestPageMapFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	canonical := readFixtureForUnicode(t, "ftl-page-map.json")
	for _, spelling := range unicodeFixtureSpellings {
		if spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replaceFixtureCaseName(t, canonical, pageMapFirstCaseName, spelling.replacement)
			if _, err := parseFixture(raw); err == nil {
				t.Fatal("page-map fixture reader accepted invalid Unicode scalar spelling")
			}
		})
	}
}

func TestReclamationFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	canonical := readFixtureForUnicode(t, "ftl-page-reclamation.json")
	for _, spelling := range unicodeFixtureSpellings {
		if spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replaceFixtureCaseName(t, canonical, reclamationFirstCaseName, spelling.replacement)
			if _, err := parseReclamationFixture(raw); err == nil {
				t.Fatal("reclamation fixture reader accepted invalid Unicode scalar spelling")
			}
		})
	}
}

func TestPageMapFixturePreservesValidUnicodeScalarNames(t *testing.T) {
	canonical := readFixtureForUnicode(t, "ftl-page-map.json")
	for _, spelling := range unicodeFixtureSpellings {
		if !spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replaceFixtureCaseName(t, canonical, pageMapFirstCaseName, spelling.replacement)
			document, err := parseFixture(raw)
			if err != nil {
				t.Fatalf("valid scalar spelling refused: %v", err)
			}
			if got := document.Cases[0].Name; got != spelling.want {
				t.Fatalf("decoded case name %q, want %q", got, spelling.want)
			}
			if err := validateFixtureModel(document); err != nil {
				t.Fatalf("independent model rejected name-only edit: %v", err)
			}
			replayPageMapFixture(t, document)
		})
	}
}

func TestReclamationFixturePreservesValidUnicodeScalarNames(t *testing.T) {
	canonical := readFixtureForUnicode(t, "ftl-page-reclamation.json")
	for _, spelling := range unicodeFixtureSpellings {
		if !spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replaceFixtureCaseName(t, canonical, reclamationFirstCaseName, spelling.replacement)
			document, err := parseReclamationFixture(raw)
			if err != nil {
				t.Fatalf("valid scalar spelling refused: %v", err)
			}
			if got := document.Cases[0].Name; got != spelling.want {
				t.Fatalf("decoded case name %q, want %q", got, spelling.want)
			}
			if err := validateReclamationModel(document); err != nil {
				t.Fatalf("independent model rejected name-only edit: %v", err)
			}
			replayReclamationFixture(t, document)
		})
	}
}

// Both readers validate the same raw JSON string escapes before encoding/json
// decodes strings and silently replaces an unpaired UTF-16 code unit.
func validateSurrogateEscapes(raw []byte) error {
	for index := 0; index < len(raw); {
		if raw[index] != '"' {
			index++
			continue
		}
		index++
		for index < len(raw) && raw[index] != '"' {
			if raw[index] != '\\' {
				index++
				continue
			}
			if index+1 >= len(raw) {
				return errors.New("truncated JSON string escape")
			}
			if raw[index+1] != 'u' {
				index += 2
				continue
			}
			unit, ok := jsonHexUnit(raw[index+2:])
			if !ok {
				return errors.New("invalid JSON Unicode escape")
			}
			if unit >= 0xDC00 && unit <= 0xDFFF {
				return errors.New("unpaired low surrogate in JSON string")
			}
			if unit >= 0xD800 && unit <= 0xDBFF {
				if index+11 >= len(raw) || raw[index+6] != '\\' || raw[index+7] != 'u' {
					return errors.New("unpaired high surrogate in JSON string")
				}
				low, lowOK := jsonHexUnit(raw[index+8:])
				if !lowOK || low < 0xDC00 || low > 0xDFFF {
					return errors.New("unpaired high surrogate in JSON string")
				}
				index += 12
				continue
			}
			index += 6
		}
		if index >= len(raw) {
			return errors.New("unterminated JSON string")
		}
		index++
	}
	return nil
}

func jsonHexUnit(raw []byte) (uint16, bool) {
	if len(raw) < 4 {
		return 0, false
	}
	var result uint16
	for _, digit := range raw[:4] {
		result <<= 4
		switch {
		case digit >= '0' && digit <= '9':
			result |= uint16(digit - '0')
		case digit >= 'a' && digit <= 'f':
			result |= uint16(digit-'a') + 10
		case digit >= 'A' && digit <= 'F':
			result |= uint16(digit-'A') + 10
		default:
			return 0, false
		}
	}
	return result, true
}

func readFixtureForUnicode(t *testing.T, name string) []byte {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", name))
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func replaceFixtureCaseName(t *testing.T, raw []byte, oldName, replacement string) []byte {
	t.Helper()
	old := []byte(`"name": "` + oldName + `"`)
	newValue := []byte(`"name": "` + replacement + `"`)
	updated := bytes.Replace(raw, old, newValue, 1)
	if bytes.Equal(updated, raw) {
		t.Fatal("case-name replacement anchor was not found")
	}
	return updated
}

func replayPageMapFixture(t *testing.T, document fixtureDocument) {
	t.Helper()
	for _, testCase := range document.Cases {
		table, err := New(*testCase.LogicalCount, *testCase.PhysicalCount)
		if err != nil {
			t.Fatalf("%s: New: %v", testCase.Name, err)
		}
		for index, step := range testCase.Steps {
			table, err = replayStep(table, step)
			if err != nil {
				t.Fatalf("%s step %d (%s): %v", testCase.Name, index, step.Op, err)
			}
		}
	}
}

func replayReclamationFixture(t *testing.T, document reclamationDocument) {
	t.Helper()
	for _, testCase := range document.Cases {
		table, err := New(testCase.LogicalCount, testCase.PhysicalCount)
		if err != nil {
			t.Fatalf("%s: New: %v", testCase.Name, err)
		}
		for index, step := range testCase.Steps {
			table, err = replayReclamationStep(table, step)
			if err != nil {
				t.Fatalf("%s step %d (%s): %v", testCase.Name, index, step.Op, err)
			}
		}
	}
}
