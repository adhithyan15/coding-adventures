package ftlnandrelocation

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"testing"
)

const (
	relocationScalarFirstCase = "zero-prefix-marker-and-media-complete-all-ones"
	blockScalarFirstCase      = "zero-prefix-quarantine-and-lost-ack"
	copyScalarFirstCase       = "before_accept_keeps_destination_free"
	evacuationScalarFirstCase = "replan-after-spent-prefix-and-lost-erase-ack"
)

type relocationUnicodeSpelling struct {
	name        string
	replacement string
	want        string
	valid       bool
}

var relocationUnicodeSpellings = []relocationUnicodeSpelling{
	{name: "isolated-high", replacement: `case-\ud800`},
	{name: "upper-isolated-high", replacement: `case-\uDBFF`},
	{name: "isolated-low", replacement: `case-\udc00`},
	{name: "upper-isolated-low", replacement: `case-\uDFFF`},
	{name: "reversed-pair", replacement: `case-\udc00\ud800`},
	{name: "missing-low-half", replacement: `case-\ud800x`},
	{name: "wrong-low-half", replacement: `case-\ud800\u0041`},
	{name: "incomplete-escape", replacement: `case-\ud80`},
	{name: "nonhex-escape", replacement: `case-\ud80G`},
	{name: "bmp-escape", replacement: `case-\u0061`, want: "case-a", valid: true},
	{name: "supplementary-lower-endpoint", replacement: `case-\ud800\udc00`, want: "case-𐀀", valid: true},
	{name: "supplementary-upper-endpoint", replacement: `case-\udbff\udfff`, want: "case-􏿿", valid: true},
	{name: "supplementary-middle", replacement: `case-\ud83d\ude00`, want: "case-😀", valid: true},
	{name: "literal-replacement", replacement: "case-�", want: "case-�", valid: true},
	{name: "escaped-replacement", replacement: `case-\ufffd`, want: "case-�", valid: true},
	{name: "escaped-backslash", replacement: `case-\\uD800`, want: `case-\uD800`, valid: true},
}

func TestRelocationFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	checkRelocationUnicodeRefusals(t, "ftl-nand-relocation.json", relocationScalarFirstCase, func(raw []byte) error {
		_, err := parseRelocationFixture(raw)
		return err
	}, "ftl-nand-relocation reader accepted invalid Unicode scalar spelling")
}

func TestBlockFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	checkRelocationUnicodeRefusals(t, "ftl-block-reclamation.json", blockScalarFirstCase, func(raw []byte) error {
		_, err := parseBlockFixture(raw)
		return err
	}, "ftl-block-reclamation reader accepted invalid Unicode scalar spelling")
}

func TestCopyFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	checkRelocationUnicodeRefusals(t, "ftl-live-page-copy.json", copyScalarFirstCase, func(raw []byte) error {
		_, err := parseCopyFixture(raw)
		return err
	}, "ftl-live-page-copy reader accepted invalid Unicode scalar spelling")
}

func TestEvacuationFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	checkRelocationUnicodeRefusals(t, "ftl-evacuation.json", evacuationScalarFirstCase, func(raw []byte) error {
		_, err := parseEvacuationFixture(raw)
		return err
	}, "ftl-evacuation reader accepted invalid Unicode scalar spelling")
}

func TestRelocationFixturePreservesValidUnicodeScalarNames(t *testing.T) {
	checkValidRelocationFixtureNames(t, "ftl-nand-relocation.json", relocationScalarFirstCase, func(raw []byte) (string, error) {
		doc, err := parseRelocationFixture(raw)
		if err != nil {
			return "", err
		}
		if err := validateRelocationFixtureModel(doc); err != nil {
			return "", err
		}
		if err := replayRelocationFixtureDocument(doc); err != nil {
			return "", err
		}
		return doc.Cases[0].Name, nil
	})
}

func TestBlockFixturePreservesValidUnicodeScalarNames(t *testing.T) {
	checkValidRelocationFixtureNames(t, "ftl-block-reclamation.json", blockScalarFirstCase, func(raw []byte) (string, error) {
		doc, err := parseBlockFixture(raw)
		if err != nil {
			return "", err
		}
		if err := validateBlockFixtureModel(doc); err != nil {
			return "", err
		}
		if err := replayBlockFixtureDocument(doc); err != nil {
			return "", err
		}
		return doc.Cases[0].Name, nil
	})
}

func TestCopyFixturePreservesValidUnicodeScalarNames(t *testing.T) {
	checkValidRelocationFixtureNames(t, "ftl-live-page-copy.json", copyScalarFirstCase, func(raw []byte) (string, error) {
		doc, err := parseCopyFixture(raw)
		if err != nil {
			return "", err
		}
		if err := validateCopyFixtureModel(doc); err != nil {
			return "", err
		}
		if err := replayCopyFixtureDocument(doc); err != nil {
			return "", err
		}
		return doc.Cases[0].Name, nil
	})
}

func TestEvacuationFixturePreservesValidUnicodeScalarNames(t *testing.T) {
	checkValidRelocationFixtureNames(t, "ftl-evacuation.json", evacuationScalarFirstCase, func(raw []byte) (string, error) {
		doc, err := parseEvacuationFixture(raw)
		if err != nil {
			return "", err
		}
		if _, err := validateEvacuationFixtureModel(doc); err != nil {
			return "", err
		}
		if err := replayEvacuationFixture(doc); err != nil {
			return "", err
		}
		return doc.Cases[0].Name, nil
	})
}

func checkRelocationUnicodeRefusals(t *testing.T, fixture, firstCase string, parse func([]byte) error, message string) {
	t.Helper()
	canonical := readRelocationUnicodeFixture(t, fixture)
	for _, spelling := range relocationUnicodeSpellings {
		if spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replaceRelocationFixtureCaseName(t, canonical, firstCase, spelling.replacement)
			if err := parse(raw); err == nil {
				t.Fatal(message)
			}
		})
	}
}

func checkValidRelocationFixtureNames(t *testing.T, fixture, firstCase string, parse func([]byte) (string, error)) {
	t.Helper()
	canonical := readRelocationUnicodeFixture(t, fixture)
	for _, spelling := range relocationUnicodeSpellings {
		if !spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replaceRelocationFixtureCaseName(t, canonical, firstCase, spelling.replacement)
			got, err := parse(raw)
			if err != nil {
				t.Fatalf("valid scalar spelling refused: %v", err)
			}
			if got != spelling.want {
				t.Fatalf("decoded case name %q, want %q", got, spelling.want)
			}
		})
	}
}

func readRelocationUnicodeFixture(t *testing.T, name string) []byte {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", name))
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func replaceRelocationFixtureCaseName(t *testing.T, raw []byte, oldName, replacement string) []byte {
	t.Helper()
	old := []byte(`"name": "` + oldName + `"`)
	newValue := []byte(`"name": "` + replacement + `"`)
	updated := bytes.Replace(raw, old, newValue, 1)
	if bytes.Equal(updated, raw) {
		t.Fatal("case-name replacement anchor was not found")
	}
	return updated
}

func replayRelocationFixtureDocument(document fixtureDocument) error {
	for _, testCase := range document.Cases {
		device, err := New(*testCase.LogicalCount, *testCase.PhysicalCount, *testCase.PageSize)
		if err != nil {
			return err
		}
		for _, step := range testCase.Steps {
			device, err = replayFixtureStep(device, step)
			if err != nil {
				return err
			}
		}
	}
	return nil
}

// validateSurrogateEscapes rejects raw JSON Unicode escapes that do not form
// Unicode scalar values before encoding/json can replace unpaired halves.
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
