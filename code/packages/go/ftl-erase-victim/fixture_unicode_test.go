package ftlerasevictim

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"testing"
)

type planningFixtureScalarSpelling struct {
	name        string
	replacement string
	want        string
	valid       bool
}

var planningFixtureScalarSpellings = []planningFixtureScalarSpelling{
	{name: "isolated-high", replacement: `case-\ud800`},
	{name: "isolated-low", replacement: `case-\udc00`},
	{name: "reversed-pair", replacement: `case-\udc00\ud800`},
	{name: "missing-low-half", replacement: `case-\ud800x`},
	{name: "wrong-low-half", replacement: `case-\ud800\u0041`},
	{name: "incomplete-escape", replacement: `case-\ud80`},
	{name: "invalid-hex", replacement: `case-\ud80G`},
	{name: "bmp-escape", replacement: `case-\u20ac`, want: "case-€", valid: true},
	{name: "minimum-pair", replacement: `case-\ud800\udc00`, want: "case-𐀀", valid: true},
	{name: "maximum-pair", replacement: `case-\udbff\udfff`, want: "case-􏿿", valid: true},
	{name: "middle-pair", replacement: `case-\ud83d\ude00`, want: "case-😀", valid: true},
	{name: "literal-replacement", replacement: "case-�", want: "case-�", valid: true},
	{name: "escaped-replacement", replacement: `case-\ufffd`, want: "case-�", valid: true},
	{name: "escaped-backslash", replacement: `case-\\uD800`, want: `case-\uD800`, valid: true},
}

func TestVictimFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	testFixtureScalarSpellings(t, "ftl-erase-victim.json", "highest-stale-score", func(raw []byte) (any, error) { doc, err := parseFixture(raw); return doc, err })
}

func TestCopyAwareVictimFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	testFixtureScalarSpellings(t, "ftl-copy-aware-victim.json", "ratio beats stale count", func(raw []byte) (any, error) { doc, err := parseCopyFixture(raw); return doc, err })
}

func TestVictimFixturePreservesValidUnicodeScalars(t *testing.T) {
	testFixtureValidScalarSpellings(t, "ftl-erase-victim.json", "highest-stale-score", func(raw []byte) (any, error) { doc, err := parseFixture(raw); return doc, err }, func(document any) ([]string, error) {
		doc := document.(fixtureDocument)
		if err := validateFixtureModel(doc); err != nil {
			return nil, err
		}
		assertFixtureModelAndRuntime(t, doc)
		names := make([]string, len(doc.Cases))
		for i := range doc.Cases {
			names[i] = doc.Cases[i].Name
		}
		return names, nil
	})
}

func TestCopyAwareVictimFixturePreservesValidUnicodeScalars(t *testing.T) {
	testFixtureValidScalarSpellings(t, "ftl-copy-aware-victim.json", "ratio beats stale count", func(raw []byte) (any, error) { doc, err := parseCopyFixture(raw); return doc, err }, func(document any) ([]string, error) {
		doc := document.(copyFixtureDocument)
		if err := validateCopyFixtureModel(doc); err != nil {
			return nil, err
		}
		for _, item := range doc.Cases {
			got, found, err := SelectCopyAware(fixtureStatuses(item.Status), *item.PagesPerBlock, *item.Excluded)
			if err != nil || found != *item.Expected.Found || got != copyFixtureVictim(*item.Expected) {
				return nil, errors.New("copy-aware fixture runtime disagreed after model admission")
			}
		}
		names := make([]string, len(doc.Cases))
		for i := range doc.Cases {
			names[i] = doc.Cases[i].Name
		}
		return names, nil
	})
}

type fixtureDocumentParser func([]byte) (any, error)
type validFixtureModelReplay func(any) ([]string, error)

func testFixtureScalarSpellings(t *testing.T, filename, firstName string, parse fixtureDocumentParser) {
	t.Helper()
	canonical := readPlanningFixture(t, filename)
	for _, spelling := range planningFixtureScalarSpellings {
		if spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replacePlanningCaseName(t, canonical, firstName, spelling.replacement)
			if _, err := parse(raw); err == nil {
				t.Fatal("fixture reader accepted an invalid Unicode scalar spelling")
			}
		})
	}
}

func testFixtureValidScalarSpellings(t *testing.T, filename, firstName string, parse fixtureDocumentParser, validateAndReplay validFixtureModelReplay) {
	t.Helper()
	canonical := readPlanningFixture(t, filename)
	for _, spelling := range planningFixtureScalarSpellings {
		if !spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replacePlanningCaseName(t, canonical, firstName, spelling.replacement)
			document, err := parse(raw)
			if err != nil {
				t.Fatalf("valid scalar spelling refused: %v", err)
			}
			names, err := validateAndReplay(document)
			if err != nil {
				t.Fatalf("model/replay rejected name-only edit: %v", err)
			}
			if len(names) == 0 {
				t.Fatal("model/replay produced no case names")
			}
			if names[0] != spelling.want {
				t.Fatalf("decoded name %q, want %q", names[0], spelling.want)
			}
		})
	}
}

// Both planning readers must reject invalid UTF-16 surrogate escapes before
// encoding/json replaces lone code units with U+FFFD.
func validatePlanningSurrogateEscapes(raw []byte) error {
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
			unit, ok := planningJSONHexUnit(raw[index+2:])
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
				low, valid := planningJSONHexUnit(raw[index+8:])
				if !valid || low < 0xDC00 || low > 0xDFFF {
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

func planningJSONHexUnit(raw []byte) (uint16, bool) {
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

func readPlanningFixture(t *testing.T, filename string) []byte {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", filename))
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func replacePlanningCaseName(t *testing.T, raw []byte, oldName, replacement string) []byte {
	t.Helper()
	old := []byte(`"name": "` + oldName + `"`)
	newValue := []byte(`"name": "` + replacement + `"`)
	updated := bytes.Replace(raw, old, newValue, 1)
	if bytes.Equal(updated, raw) {
		t.Fatal("case-name replacement anchor was not found")
	}
	return updated
}
