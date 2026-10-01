package barcodelayout1d

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"reflect"
	"strings"
	"testing"
	"unicode/utf8"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const corpusSHA256V1 = "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388"

const maxFixtureBytesV1 = 131072

type fixtureCaseV1 struct {
	ID        string         `json:"id"`
	Operation string         `json:"operation"`
	Input     map[string]any `json:"input"`
	Expected  map[string]any `json:"expected"`
}

func TestBarcodeLayout1DV1Conformance(t *testing.T) {
	data, err := readBoundedFixtureV1("../../../specs/fixtures/barcode-layout-1d-v1/cases.json")
	if err != nil {
		t.Fatal(err)
	}
	digest := sha256.Sum256(data)
	if got := hex.EncodeToString(digest[:]); got != corpusSHA256V1 {
		t.Fatalf("corpus digest = %s", got)
	}
	var fixture struct {
		SchemaVersion int             `json:"schema_version"`
		Profile       string          `json:"profile"`
		Cases         []fixtureCaseV1 `json:"cases"`
	}
	if err := validateJSONEnvelopeV1(data); err != nil {
		t.Fatal(err)
	}
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	if err := decoder.Decode(&fixture); err != nil {
		t.Fatal(err)
	}
	if fixture.SchemaVersion != 1 || fixture.Profile != "barcode-layout-1d-v1" {
		t.Fatal("invalid fixture identity")
	}
	if len(fixture.Cases) != 56 {
		t.Fatalf("case count = %d, want 56", len(fixture.Cases))
	}

	for _, item := range fixture.Cases {
		item := item
		t.Run(item.ID, func(t *testing.T) {
			actual, err := dispatchFixtureV1(item)
			expectedError, wantsError := item.Expected["error"].(string)
			if wantsError {
				var portable *Barcode1DV1Error
				if !errors.As(err, &portable) {
					t.Fatalf("wanted %s, got %v", expectedError, err)
				}
				if portable.ID != expectedError {
					t.Fatalf("error = %s, want %s", portable.ID, expectedError)
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			if normalized := normalizeJSONV1(t, actual); !reflect.DeepEqual(normalized, item.Expected) {
				t.Fatalf("expected %#v\nactual   %#v", item.Expected, normalized)
			}
		})
	}
}

func readBoundedFixtureV1(path string) ([]byte, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	info, err := file.Stat()
	if err != nil {
		return nil, err
	}
	if info.Size() < 1 || info.Size() > maxFixtureBytesV1 {
		return nil, fmt.Errorf("fixture size %d is outside its envelope", info.Size())
	}
	data, err := io.ReadAll(io.LimitReader(file, maxFixtureBytesV1+1))
	if err != nil {
		return nil, err
	}
	if len(data) > maxFixtureBytesV1 {
		return nil, errors.New("fixture-size-limit")
	}
	return data, nil
}

func validateJSONEnvelopeV1(data []byte) error {
	if !utf8.Valid(data) {
		return errors.New("fixture-invalid-utf8")
	}
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	if err := scanJSONValueV1(decoder, 0); err != nil {
		return err
	}
	if _, err := decoder.Token(); !errors.Is(err, io.EOF) {
		return errors.New("fixture-trailing-data")
	}
	return nil
}

func scanJSONValueV1(decoder *json.Decoder, depth int) error {
	if depth > 8 {
		return errors.New("fixture-depth-limit")
	}
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	delim, composite := token.(json.Delim)
	if !composite {
		if number, ok := token.(json.Number); ok {
			if _, err := number.Int64(); err != nil {
				return errors.New("fixture-invalid-number")
			}
		}
		return nil
	}
	switch delim {
	case '{':
		names := map[string]struct{}{}
		for decoder.More() {
			nameToken, err := decoder.Token()
			if err != nil {
				return err
			}
			name, ok := nameToken.(string)
			if !ok {
				return errors.New("fixture-invalid-object-key")
			}
			if _, exists := names[name]; exists {
				return errors.New("fixture-duplicate-key")
			}
			names[name] = struct{}{}
			if err := scanJSONValueV1(decoder, depth+1); err != nil {
				return err
			}
		}
		_, err = decoder.Token()
		return err
	case '[':
		for decoder.More() {
			if err := scanJSONValueV1(decoder, depth+1); err != nil {
				return err
			}
		}
		_, err = decoder.Token()
		return err
	default:
		return errors.New("fixture-invalid-json")
	}
}

func TestBarcodeLayout1DV1TextFailsBeforeNativeResolution(t *testing.T) {
	runs := []Barcode1DV1Run{{Color: "bar", Modules: 1, SourceLabel: "A", SourceIndex: 0, Role: "data"}}
	text := "A"
	calls := 0
	options := []*Barcode1DV1SceneOptions{
		{HumanReadableText: &text},
		{RenderConfig: &Barcode1DV1RenderConfig{IncludeHumanReadableText: true}},
	}
	for _, option := range options {
		_, err := projectSceneV1WithProbe(runs, option, func() { calls++ })
		var portable *Barcode1DV1Error
		if !errors.As(err, &portable) || portable.ID != "human-readable-text-unsupported" {
			t.Fatalf("got %v", err)
		}
	}
	if calls != 0 {
		t.Fatalf("native resolver called %d times", calls)
	}
}

func TestBarcodeLayout1DV1ResultsAreDeepOwned(t *testing.T) {
	caller := map[string]string{"owner": "caller"}
	runs, err := ExpandBinaryV1("101", Barcode1DV1BinaryOptions{SourceLabel: "A", Role: "data"})
	if err != nil {
		t.Fatal(err)
	}
	options := &Barcode1DV1SceneOptions{QuietZoneModules: intPointerV1(10), Label: stringPointerV1("1D barcode"), Metadata: caller, RenderConfig: &Barcode1DV1RenderConfig{ModuleWidth: intPointerV1(4), BarHeight: intPointerV1(120), Foreground: stringPointerV1("#000000"), Background: stringPointerV1("#ffffff")}}
	first, err := ProjectSceneV1(runs, options)
	if err != nil {
		t.Fatal(err)
	}
	caller["owner"] = "changed"
	if first.Metadata["owner"] != "caller" {
		t.Fatal("scene metadata aliases caller input")
	}
	second, err := ProjectSceneV1(runs, &Barcode1DV1SceneOptions{QuietZoneModules: intPointerV1(10), Label: stringPointerV1("1D barcode"), Metadata: map[string]string{"owner": "caller"}, RenderConfig: options.RenderConfig})
	if err != nil {
		t.Fatal(err)
	}
	first.Metadata["owner"] = "first"
	if second.Metadata["owner"] != "caller" {
		t.Fatal("scene metadata aliases prior result")
	}
	firstRect := first.Instructions[0].(paintinstructions.PaintRectInstruction)
	secondRect := second.Instructions[0].(paintinstructions.PaintRectInstruction)
	firstRect.Metadata["role"] = "changed"
	if secondRect.Metadata["role"] != "data" {
		t.Fatal("rectangle metadata aliases prior result")
	}
}

func TestBarcodeLayout1DV1PublicAPIsApplyDefaults(t *testing.T) {
	runs, err := ExpandWidthV1("NW", Barcode1DV1WidthOptions{SourceLabel: "A", SourceIndex: 0, Role: "data"})
	if err != nil {
		t.Fatal(err)
	}
	if runs[0].Modules != 1 || runs[1].Modules != 3 || runs[0].Color != "bar" {
		t.Fatalf("width defaults not applied: %#v", runs)
	}
	scene, err := ProjectSceneV1(runs, &Barcode1DV1SceneOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if scene.Width != 96 || scene.Height != 120 || scene.Background != "#ffffff" || scene.Metadata["label"] != "1D barcode" {
		t.Fatalf("scene defaults not applied: %#v", scene)
	}
}

func TestCanonicalRunEncodingDoesNotEscapeNonASCIIOrHTML(t *testing.T) {
	runs := []Barcode1DV1Run{{Color: "bar", Modules: 1, Role: "data", SourceIndex: 0, SourceLabel: "<é\u2028"}}
	want := []byte(`[{"color":"bar","modules":1,"role":"data","sourceIndex":0,"sourceLabel":"<é "}]`)
	if got := canonicalRunsJSONV1(runs); !bytes.Equal(got, want) {
		t.Fatalf("canonical JSON = %q, want %q", got, want)
	}
}

func TestFixtureLoaderRejectsDuplicateKeysAndInvalidRepeatCounts(t *testing.T) {
	if err := validateJSONEnvelopeV1([]byte(`{"a":1,"a":2}`)); err == nil || err.Error() != "fixture-duplicate-key" {
		t.Fatalf("duplicate key error = %v", err)
	}
	if _, err := runsV1(map[string]any{"repeatRuns": map[string]any{"count": json.Number("40980")}}); err == nil || err.Error() != "too-many-runs" {
		t.Fatalf("repeatRuns error = %v", err)
	}
	if _, err := symbolsV1(map[string]any{"repeatSymbols": map[string]any{"count": json.Number("40980")}}); err == nil || err.Error() != "too-many-symbols" {
		t.Fatalf("repeatSymbols error = %v", err)
	}
	if _, err := patternV1(map[string]any{"repeat": map[string]any{"token": "1", "count": json.Number("-1")}}); err == nil || err.Error() != "fixture-invalid-count" {
		t.Fatalf("repeat error = %v", err)
	}
}

func dispatchFixtureV1(item fixtureCaseV1) (map[string]any, error) {
	switch item.Operation {
	case "expand-binary":
		pattern, err := patternV1(item.Input)
		if err != nil {
			return nil, err
		}
		runs, err := ExpandBinaryV1(pattern, Barcode1DV1BinaryOptions{SourceLabel: stringV1(item.Input, "sourceLabel", ""), SourceIndex: intV1(item.Input, "sourceIndex", 0), Role: stringV1(item.Input, "role", "")})
		if err != nil {
			return nil, err
		}
		return runResultV1(runs), nil
	case "expand-width":
		pattern, err := patternV1(item.Input)
		if err != nil {
			return nil, err
		}
		runs, err := ExpandWidthV1(pattern, Barcode1DV1WidthOptions{SourceLabel: stringV1(item.Input, "sourceLabel", ""), SourceIndex: intV1(item.Input, "sourceIndex", 0), Role: stringV1(item.Input, "role", ""), NarrowMarker: optionalStringPointerV1(item.Input, "narrowMarker"), WideMarker: optionalStringPointerV1(item.Input, "wideMarker"), NarrowModules: optionalIntPointerV1(item.Input, "narrowModules"), WideModules: optionalIntPointerV1(item.Input, "wideModules"), StartingColor: optionalStringPointerV1(item.Input, "startingColor")})
		if err != nil {
			return nil, err
		}
		return runResultV1(runs), nil
	case "compute-layout":
		runs, err := runsV1(item.Input)
		if err != nil {
			return nil, err
		}
		symbols, err := symbolsV1(item.Input)
		if err != nil {
			return nil, err
		}
		layout, err := ComputeLayoutV1(runs, intV1(item.Input, "quietZoneModules", 0), symbols)
		if err != nil {
			return nil, err
		}
		return map[string]any{"layout": layoutNodeV1(layout)}, nil
	case "project-scene":
		runs, err := runsV1(item.Input)
		if err != nil {
			return nil, err
		}
		options, err := sceneOptionsV1(item.Input)
		if err != nil {
			return nil, err
		}
		scene, err := ProjectSceneV1(runs, options)
		if err != nil {
			return nil, err
		}
		return map[string]any{"scene": sceneNodeV1(scene)}, nil
	default:
		return nil, fmt.Errorf("unknown operation %s", item.Operation)
	}
}

func runResultV1(runs []Barcode1DV1Run) map[string]any {
	if len(runs) <= 1000 {
		values := make([]any, len(runs))
		for i, run := range runs {
			values[i] = runNodeV1(run)
		}
		return map[string]any{"runs": values}
	}
	content := int64(0)
	for _, run := range runs {
		content += run.Modules
	}
	encoded := canonicalRunsJSONV1(runs)
	digest := sha256.Sum256(encoded)
	return map[string]any{"runDigest": map[string]any{"runCount": len(runs), "contentModules": content, "firstRun": runNodeV1(runs[0]), "lastRun": runNodeV1(runs[len(runs)-1]), "runsSha256": hex.EncodeToString(digest[:])}}
}

func canonicalRunsJSONV1(runs []Barcode1DV1Run) []byte {
	var result strings.Builder
	result.WriteByte('[')
	for index, run := range runs {
		if index > 0 {
			result.WriteByte(',')
		}
		result.WriteString(`{"color":`)
		appendCanonicalStringV1(&result, run.Color)
		result.WriteString(`,"modules":`)
		result.WriteString(fmt.Sprint(run.Modules))
		result.WriteString(`,"role":`)
		appendCanonicalStringV1(&result, run.Role)
		result.WriteString(`,"sourceIndex":`)
		result.WriteString(fmt.Sprint(run.SourceIndex))
		result.WriteString(`,"sourceLabel":`)
		appendCanonicalStringV1(&result, run.SourceLabel)
		result.WriteByte('}')
	}
	result.WriteByte(']')
	return []byte(result.String())
}

func appendCanonicalStringV1(result *strings.Builder, value string) {
	const hex = "0123456789abcdef"
	result.WriteByte('"')
	for _, character := range value {
		switch character {
		case '"':
			result.WriteString(`\"`)
		case '\\':
			result.WriteString(`\\`)
		case '\b':
			result.WriteString(`\b`)
		case '\f':
			result.WriteString(`\f`)
		case '\n':
			result.WriteString(`\n`)
		case '\r':
			result.WriteString(`\r`)
		case '\t':
			result.WriteString(`\t`)
		default:
			if character < 0x20 {
				result.WriteString(`\u00`)
				result.WriteByte(hex[byte(character)>>4])
				result.WriteByte(hex[byte(character)&0xf])
			} else {
				result.WriteRune(character)
			}
		}
	}
	result.WriteByte('"')
}

func runNodeV1(run Barcode1DV1Run) map[string]any {
	return map[string]any{"color": run.Color, "modules": run.Modules, "sourceLabel": run.SourceLabel, "sourceIndex": run.SourceIndex, "role": run.Role}
}
func layoutNodeV1(layout Barcode1DV1Layout) map[string]any {
	symbols := make([]any, len(layout.SymbolLayouts))
	for i, s := range layout.SymbolLayouts {
		symbols[i] = map[string]any{"label": s.Label, "startModule": s.StartModule, "endModule": s.EndModule, "sourceIndex": s.SourceIndex, "role": s.Role}
	}
	return map[string]any{"leftQuietZoneModules": layout.LeftQuietZoneModules, "rightQuietZoneModules": layout.RightQuietZoneModules, "contentModules": layout.ContentModules, "totalModules": layout.TotalModules, "symbolLayouts": symbols}
}
func sceneNodeV1(scene paintinstructions.PaintScene) map[string]any {
	rectangles := make([]any, len(scene.Instructions))
	for i, instruction := range scene.Instructions {
		rect := instruction.(paintinstructions.PaintRectInstruction)
		rectangles[i] = map[string]any{"x": rect.X, "y": rect.Y, "width": rect.Width, "height": rect.Height, "fill": rect.Fill, "metadata": metadataNodeV1(rect.Metadata)}
	}
	return map[string]any{"width": scene.Width, "height": scene.Height, "background": scene.Background, "rectangles": rectangles, "metadata": metadataNodeV1(scene.Metadata)}
}
func metadataNodeV1(metadata paintinstructions.Metadata) map[string]any {
	result := make(map[string]any, len(metadata))
	for k, v := range metadata {
		result[k] = v.(string)
	}
	return result
}

func sceneOptionsV1(input map[string]any) (*Barcode1DV1SceneOptions, error) {
	render, _ := input["renderConfig"].(map[string]any)
	metadata := map[string]string{}
	if raw, ok := input["metadata"].(map[string]any); ok {
		for k, v := range raw {
			metadata[k] = v.(string)
		}
	}
	var text *string
	if value, ok := input["humanReadableText"].(string); ok {
		text = &value
	}
	symbols, err := symbolsV1(input)
	if err != nil {
		return nil, err
	}
	return &Barcode1DV1SceneOptions{QuietZoneModules: optionalIntPointerV1(input, "quietZoneModules"), Label: optionalStringPointerV1(input, "label"), Metadata: metadata, HumanReadableText: text, Symbols: symbols, RenderConfig: &Barcode1DV1RenderConfig{ModuleWidth: optionalIntPointerV1(render, "moduleWidth"), BarHeight: optionalIntPointerV1(render, "barHeight"), Foreground: optionalStringPointerV1(render, "foreground"), Background: optionalStringPointerV1(render, "background"), IncludeHumanReadableText: boolV1(render, "includeHumanReadableText", false)}}, nil
}

func runsV1(input map[string]any) ([]Barcode1DV1Run, error) {
	if values, ok := input["runs"].([]any); ok {
		if len(values) > maxRunsV1 {
			return nil, failV1("too-many-runs")
		}
		result := make([]Barcode1DV1Run, len(values))
		for i, value := range values {
			run := value.(map[string]any)
			result[i] = Barcode1DV1Run{Color: stringV1(run, "color", ""), Modules: intV1(run, "modules", 0), SourceLabel: stringV1(run, "sourceLabel", ""), SourceIndex: intV1(run, "sourceIndex", 0), Role: stringV1(run, "role", "")}
		}
		return result, nil
	}
	repeat := input["repeatRuns"].(map[string]any)
	count, err := checkedFixtureCountV1(repeat, "count", maxRunsV1, "too-many-runs")
	if err != nil {
		return nil, err
	}
	result := make([]Barcode1DV1Run, count)
	color := stringV1(repeat, "firstColor", "")
	for i := range result {
		result[i] = Barcode1DV1Run{Color: color, Modules: intV1(repeat, "modules", 0), SourceLabel: stringV1(repeat, "sourceLabel", ""), SourceIndex: intV1(repeat, "sourceIndex", 0), Role: stringV1(repeat, "role", "")}
		color = toggleColorV1(color)
	}
	return result, nil
}
func symbolsV1(input map[string]any) ([]Barcode1DV1SymbolDescriptor, error) {
	if values, ok := input["symbols"].([]any); ok {
		if len(values) > maxSymbolsV1 {
			return nil, failV1("too-many-symbols")
		}
		result := make([]Barcode1DV1SymbolDescriptor, len(values))
		for i, value := range values {
			symbol := value.(map[string]any)
			result[i] = Barcode1DV1SymbolDescriptor{Label: stringV1(symbol, "label", ""), Modules: intV1(symbol, "modules", 0), SourceIndex: intV1(symbol, "sourceIndex", 0), Role: stringV1(symbol, "role", "")}
		}
		return result, nil
	}
	repeat, ok := input["repeatSymbols"].(map[string]any)
	if !ok {
		return nil, nil
	}
	count, err := checkedFixtureCountV1(repeat, "count", maxSymbolsV1, "too-many-symbols")
	if err != nil {
		return nil, err
	}
	result := make([]Barcode1DV1SymbolDescriptor, count)
	for i := range result {
		result[i] = Barcode1DV1SymbolDescriptor{Label: stringV1(repeat, "label", ""), Modules: intV1(repeat, "modules", 0), SourceIndex: int64(i), Role: stringV1(repeat, "role", "")}
	}
	return result, nil
}
func patternV1(input map[string]any) (string, error) {
	if value, ok := input["pattern"].(string); ok {
		return value, nil
	}
	repeat := input["repeat"].(map[string]any)
	count, err := checkedFixtureCountV1(repeat, "count", maxPatternScalarsV1+1, "pattern-too-long")
	if err != nil {
		return "", err
	}
	token, suffix := stringV1(repeat, "token", ""), stringV1(repeat, "suffix", "")
	tokenScalars, tokenOK := scalarCountV1(token)
	suffixScalars, suffixOK := scalarCountV1(suffix)
	if !tokenOK || !suffixOK {
		return "", failV1("invalid-binary-token")
	}
	if tokenScalars != 0 && count > (maxPatternScalarsV1+1-suffixScalars)/tokenScalars {
		return "", failV1("pattern-too-long")
	}
	return strings.Repeat(token, count) + suffix, nil
}

func checkedFixtureCountV1(value map[string]any, key string, limit int, errorID string) (int, error) {
	raw, ok := value[key].(json.Number)
	if !ok {
		return 0, errors.New("fixture-invalid-type")
	}
	count, err := raw.Int64()
	if err != nil || count < 0 {
		return 0, errors.New("fixture-invalid-count")
	}
	if count > int64(limit) {
		return 0, failV1(errorID)
	}
	return int(count), nil
}
func intV1(value map[string]any, key string, fallback int64) int64 {
	raw, ok := value[key]
	if !ok {
		return fallback
	}
	number := raw.(json.Number)
	result, err := number.Int64()
	if err != nil {
		panic(err)
	}
	return result
}
func stringV1(value map[string]any, key, fallback string) string {
	raw, ok := value[key]
	if !ok {
		return fallback
	}
	return raw.(string)
}
func boolV1(value map[string]any, key string, fallback bool) bool {
	raw, ok := value[key]
	if !ok {
		return fallback
	}
	return raw.(bool)
}

func intPointerV1(value int64) *int64      { return &value }
func stringPointerV1(value string) *string { return &value }
func optionalIntPointerV1(value map[string]any, key string) *int64 {
	if _, ok := value[key]; !ok {
		return nil
	}
	result := intV1(value, key, 0)
	return &result
}
func optionalStringPointerV1(value map[string]any, key string) *string {
	if _, ok := value[key]; !ok {
		return nil
	}
	result := stringV1(value, key, "")
	return &result
}
func normalizeJSONV1(t *testing.T, value any) any {
	t.Helper()
	data, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	var result any
	if err := decoder.Decode(&result); err != nil {
		t.Fatal(err)
	}
	return result
}
