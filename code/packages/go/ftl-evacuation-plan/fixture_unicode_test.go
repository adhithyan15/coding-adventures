package ftlevacuationplan

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
)

type planningScalarSpelling struct {
	name        string
	replacement string
	want        string
	valid       bool
}

var planningScalarSpellings = []planningScalarSpelling{
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

type planningFixtureParser func([]byte) (any, error)
type planningFixtureReplay func(any) ([]string, error)

func TestEvacuationFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	testPlanningScalarRefusals(t, "ftl-evacuation-plan.json", "two-live-sources-exact-spare-reserve", func(raw []byte) (any, error) {
		return parseFixture(raw)
	})
}

func TestDrainFixtureRejectsInvalidUnicodeScalars(t *testing.T) {
	testPlanningScalarRefusals(t, "ftl-retirement-drain-plan.json", "all-live-draining-target-and-zero-erase-budget", func(raw []byte) (any, error) {
		return parseFixtureSchema(raw, drainFixtureSchema)
	})
}

func TestEvacuationFixturePreservesValidUnicodeScalars(t *testing.T) {
	testPlanningScalarPositives(t, "ftl-evacuation-plan.json", "two-live-sources-exact-spare-reserve", func(raw []byte) (any, error) {
		return parseFixture(raw)
	}, replayEvacuationFixtureAfterModel)
}

func TestDrainFixturePreservesValidUnicodeScalars(t *testing.T) {
	testPlanningScalarPositives(t, "ftl-retirement-drain-plan.json", "all-live-draining-target-and-zero-erase-budget", func(raw []byte) (any, error) {
		return parseFixtureSchema(raw, drainFixtureSchema)
	}, replayDrainFixtureAfterModel)
}

func testPlanningScalarRefusals(t *testing.T, filename, caseName string, parse planningFixtureParser) {
	t.Helper()
	canonical := readPlanningFixture(t, filename)
	for _, spelling := range planningScalarSpellings {
		if spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replacePlanningFixtureCaseName(t, canonical, caseName, spelling.replacement)
			if _, err := parse(raw); err == nil {
				t.Fatal("fixture reader accepted an invalid Unicode scalar spelling")
			}
		})
	}
}

func testPlanningScalarPositives(t *testing.T, filename, caseName string, parse planningFixtureParser, replay planningFixtureReplay) {
	t.Helper()
	canonical := readPlanningFixture(t, filename)
	for _, spelling := range planningScalarSpellings {
		if !spelling.valid {
			continue
		}
		t.Run(spelling.name, func(t *testing.T) {
			raw := replacePlanningFixtureCaseName(t, canonical, caseName, spelling.replacement)
			document, err := parse(raw)
			if err != nil {
				t.Fatalf("valid scalar spelling refused: %v", err)
			}
			names, err := replay(document)
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

func planningTableFromFixture(item fixtureCase) (ftlpagemap.Table, error) {
	table, err := ftlpagemap.New(*item.LogicalCount, *item.PhysicalCount)
	if err != nil {
		return ftlpagemap.Table{}, err
	}
	for _, logical := range *item.ProgramLogicals {
		table, _, err = table.Begin(logical)
		if err != nil {
			return ftlpagemap.Table{}, err
		}
		table, err = table.Finish(ftlpagemap.MediaReady)
		if err != nil {
			return ftlpagemap.Table{}, err
		}
	}
	return table, nil
}

func replayEvacuationFixtureAfterModel(value any) ([]string, error) {
	document := value.(fixtureDocument)
	if err := validateFixtureModel(document); err != nil {
		return nil, err
	}
	for _, item := range document.Cases {
		table, err := planningTableFromFixture(item)
		if err != nil {
			return nil, err
		}
		before := table.Snapshot()
		plan, err := PlanEvacuation(table, *item.PagesPerBlock, *item.Block, *item.ExcludedBlocks)
		if errorName(err) != fixtureError(item.Expected.Error) {
			return nil, errors.New("evacuation runtime error differs from fixture declaration")
		}
		if err == nil && !reflect.DeepEqual(plan, runtimePlan(item.Expected.Plan)) {
			return nil, errors.New("evacuation runtime plan differs from fixture declaration")
		}
		if !reflect.DeepEqual(table.Snapshot(), before) {
			return nil, errors.New("evacuation runtime mutated input table")
		}
	}
	names := make([]string, len(document.Cases))
	for index := range document.Cases {
		names[index] = document.Cases[index].Name
	}
	return names, nil
}

func replayDrainFixtureAfterModel(value any) ([]string, error) {
	document := value.(fixtureDocument)
	if err := validateDrainFixtureModel(document); err != nil {
		return nil, err
	}
	for _, item := range document.Cases {
		table, err := planningTableFromFixture(item)
		if err != nil {
			return nil, err
		}
		before := table.Snapshot()
		plan, err := PlanDrain(table, *item.PagesPerBlock, *item.Block, *item.ExcludedBlocks)
		if errorName(err) != fixtureError(item.Expected.Error) {
			return nil, errors.New("drain runtime error differs from fixture declaration")
		}
		if err == nil && !reflect.DeepEqual(plan, runtimePlan(item.Expected.Plan)) {
			return nil, errors.New("drain runtime plan differs from fixture declaration")
		}
		if !reflect.DeepEqual(table.Snapshot(), before) {
			return nil, errors.New("drain runtime mutated input table")
		}
	}
	names := make([]string, len(document.Cases))
	for index := range document.Cases {
		names[index] = document.Cases[index].Name
	}
	return names, nil
}

// validatePlanningSurrogateEscapes rejects lone UTF-16 code units before
// encoding/json silently replaces them with U+FFFD.
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

func replacePlanningFixtureCaseName(t *testing.T, raw []byte, name, replacement string) []byte {
	t.Helper()
	old := []byte(`"name": "` + name + `"`)
	newValue := []byte(`"name": "` + replacement + `"`)
	updated := bytes.Replace(raw, old, newValue, 1)
	if bytes.Equal(updated, raw) {
		t.Fatal("case-name replacement anchor was not found")
	}
	return updated
}
