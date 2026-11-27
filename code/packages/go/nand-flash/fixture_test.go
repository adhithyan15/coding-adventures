package nandflash

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"
)

const (
	maxFixtureBytes = 256 * 1024
	maxFixtureDepth = 32
)

// fixtureInteger refuses JSON null, fractions, exponents and values that do
// not fit signed 64-bit. The shared vector's numbers are all nonnegative.
type fixtureInteger int64

func (number *fixtureInteger) UnmarshalJSON(data []byte) error {
	if len(data) == 0 || bytes.Equal(data, []byte("null")) {
		return errors.New("integer must not be missing or null")
	}
	value, err := strconv.ParseInt(string(data), 10, 64)
	if err != nil || value < -int64(maxSafeEraseCycles) || value > int64(maxSafeEraseCycles) {
		return errors.New("integer must be an exact JSON integer")
	}
	*number = fixtureInteger(value)
	return nil
}

type traceDocument struct {
	Format                *string           `json:"format"`
	Description           *string           `json:"description"`
	Geometry              *traceGeometry    `json:"geometry"`
	InitialState          *traceState       `json:"initial_state"`
	Operations            []*traceOperation `json:"operations"`
	StatesAfterOperations []*traceState     `json:"states_after_operations"`
}

type traceGeometry struct {
	BlockCount     *fixtureInteger `json:"block_count"`
	PagesPerBlock  *fixtureInteger `json:"pages_per_block"`
	PageSizeBytes  *fixtureInteger `json:"page_size_bytes"`
	MaxEraseCycles json.RawMessage `json:"max_erase_cycles"`
}

type traceState struct {
	Pages       [][]*string       `json:"pages"`
	EraseCounts []*fixtureInteger `json:"erase_counts"`
	Programmed  [][]*bool         `json:"programmed"`
}

type traceOperation struct {
	Op     *string         `json:"op"`
	Block  *fixtureInteger `json:"block"`
	Page   *fixtureInteger `json:"page,omitempty"`
	Data   *string         `json:"data,omitempty"`
	Result json.RawMessage `json:"result,omitempty"`
	Error  *traceError     `json:"error,omitempty"`
}

type traceError struct {
	Type    *string `json:"type"`
	Message *string `json:"message"`
}

// readTrace bounds bytes before decoding and validates the JSON token stream
// for duplicate keys, maximum nesting, exactly one root object, and EOF.
func readTrace(path string) (traceDocument, error) {
	file, err := os.Open(path)
	if err != nil {
		return traceDocument{}, err
	}
	defer file.Close()
	raw, err := io.ReadAll(io.LimitReader(file, maxFixtureBytes+1))
	if err != nil {
		return traceDocument{}, err
	}
	return decodeTrace(raw)
}

func decodeTrace(raw []byte) (traceDocument, error) {
	if len(raw) > maxFixtureBytes {
		return traceDocument{}, errors.New("fixture exceeds byte budget")
	}
	if !utf8.Valid(raw) {
		return traceDocument{}, errors.New("fixture is not UTF-8")
	}
	if err := validateSurrogateEscapes(raw); err != nil {
		return traceDocument{}, err
	}
	if err := validateJSONTokens(raw); err != nil {
		return traceDocument{}, err
	}
	if err := validateExactFixtureObjects(raw); err != nil {
		return traceDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	decoder.DisallowUnknownFields()
	var document traceDocument
	if err := decoder.Decode(&document); err != nil {
		return traceDocument{}, err
	}
	if err := requireEOF(decoder); err != nil {
		return traceDocument{}, err
	}
	return document, nil
}

// exactObject requires both the exact decoded member names and their presence.
// A generic object pass is needed because encoding/json's struct matching is
// deliberately case-insensitive and pointer fields conflate missing and null.
func exactObject(value any, want ...string) (map[string]any, error) {
	object, ok := value.(map[string]any)
	if !ok || len(object) != len(want) {
		return nil, errors.New("fixture object has unexpected shape")
	}
	for _, key := range want {
		if _, exists := object[key]; !exists {
			return nil, errors.New("fixture object is missing exact member " + key)
		}
	}
	return object, nil
}

func validateExactFixtureObjects(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var rootValue any
	if err := decoder.Decode(&rootValue); err != nil {
		return err
	}
	root, err := exactObject(rootValue, "format", "description", "geometry", "initial_state", "operations", "states_after_operations")
	if err != nil {
		return err
	}
	if _, err := exactObject(root["geometry"], "block_count", "pages_per_block", "page_size_bytes", "max_erase_cycles"); err != nil {
		return err
	}
	if err := exactStateObject(root["initial_state"]); err != nil {
		return err
	}
	operations, ok := root["operations"].([]any)
	if !ok {
		return errors.New("operations must be an array")
	}
	for _, item := range operations {
		operation, ok := item.(map[string]any)
		if !ok {
			return errors.New("operation must be an object")
		}
		name, ok := operation["op"].(string)
		if !ok {
			return errors.New("operation name must be a string")
		}
		_, hasError := operation["error"]
		_, hasResult := operation["result"]
		if hasError == hasResult {
			return errors.New("operation must contain result or error")
		}
		var keys []string
		switch name {
		case "read":
			keys = []string{"op", "block", "page"}
		case "program":
			keys = []string{"op", "block", "page", "data"}
		case "erase_block":
			keys = []string{"op", "block"}
		default:
			return errors.New("unknown operation name")
		}
		wantKeys := append(append([]string(nil), keys...), "result")
		if hasError {
			wantKeys[len(wantKeys)-1] = "error"
		}
		if _, err := exactObject(operation, wantKeys...); err != nil {
			return err
		}
		if hasError {
			if _, err := exactObject(operation["error"], "type", "message"); err != nil {
				return err
			}
		}
	}
	states, ok := root["states_after_operations"].([]any)
	if !ok {
		return errors.New("states_after_operations must be an array")
	}
	for _, state := range states {
		if err := exactStateObject(state); err != nil {
			return err
		}
	}
	return nil
}

func exactStateObject(value any) error {
	if _, err := exactObject(value, "pages", "erase_counts", "programmed"); err != nil {
		return err
	}
	return nil
}

// encoding/json replaces unpaired UTF-16 surrogates with U+FFFD. Check the
// escaped code units first so malformed fixture text cannot be normalized.
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

func validateJSONTokens(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	root, err := decoder.Token()
	if err != nil {
		return err
	}
	if root != json.Delim('{') {
		return errors.New("fixture root must be an object")
	}
	if err := walkJSONValue(decoder, root, 1); err != nil {
		return err
	}
	if _, err := decoder.Token(); err != io.EOF {
		if err == nil {
			return errors.New("fixture has trailing JSON")
		}
		return err
	}
	return nil
}

func walkJSONValue(decoder *json.Decoder, token json.Token, depth int) error {
	delimiter, isContainer := token.(json.Delim)
	if !isContainer {
		return nil
	}
	if depth > maxFixtureDepth {
		return errors.New("fixture exceeds nesting budget")
	}
	switch delimiter {
	case '{':
		seen := make(map[string]struct{})
		for decoder.More() {
			keyToken, err := decoder.Token()
			if err != nil {
				return err
			}
			key, ok := keyToken.(string)
			if !ok {
				return errors.New("object key is not a string")
			}
			if _, duplicate := seen[key]; duplicate {
				return errors.New("fixture has duplicate object key")
			}
			seen[key] = struct{}{}
			value, err := decoder.Token()
			if err != nil {
				return err
			}
			if err := walkJSONValue(decoder, value, depth+1); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil || end != json.Delim('}') {
			return errors.New("fixture object is not terminated")
		}
	case '[':
		for decoder.More() {
			value, err := decoder.Token()
			if err != nil {
				return err
			}
			if err := walkJSONValue(decoder, value, depth+1); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil || end != json.Delim(']') {
			return errors.New("fixture array is not terminated")
		}
	default:
		return errors.New("unexpected fixture delimiter")
	}
	return nil
}

func requireEOF(decoder *json.Decoder) error {
	var trailing json.RawMessage
	if err := decoder.Decode(&trailing); err != io.EOF {
		if err == nil {
			return errors.New("fixture has trailing JSON")
		}
		return err
	}
	return nil
}

func requiredInt(value *fixtureInteger, field string) (int64, error) {
	if value == nil {
		return 0, errors.New(field + " is required and non-null")
	}
	return int64(*value), nil
}

func hostInt(value int64, field string) (int, error) {
	converted := int(value)
	if int64(converted) != value {
		return 0, errors.New(field + " exceeds host int range")
	}
	return converted, nil
}

func validateTrace(document traceDocument) (*Device, error) {
	if document.Format == nil || *document.Format != "coding-adventures-nand-flash-v1" ||
		document.Description == nil || document.Geometry == nil || document.InitialState == nil ||
		document.Operations == nil || len(document.Operations) == 0 ||
		document.StatesAfterOperations == nil || len(document.Operations) != len(document.StatesAfterOperations) {
		return nil, errors.New("fixture header or operation/state dimensions are invalid")
	}
	blocks, err := requiredInt(document.Geometry.BlockCount, "block_count")
	if err != nil {
		return nil, err
	}
	pagesPerBlock, err := requiredInt(document.Geometry.PagesPerBlock, "pages_per_block")
	if err != nil {
		return nil, err
	}
	pageSize, err := requiredInt(document.Geometry.PageSizeBytes, "page_size_bytes")
	if err != nil {
		return nil, err
	}
	if blocks < 1 || blocks > maxBlocks || pagesPerBlock < 1 || pagesPerBlock > maxPagesPerBlock || pageSize < 1 || pageSize > maxPageSizeBytes {
		return nil, errors.New("fixture geometry exceeds DB01 dimensions")
	}
	var eraseLimit *uint64
	if len(document.Geometry.MaxEraseCycles) == 0 {
		return nil, errors.New("max_erase_cycles field is required")
	}
	if !bytes.Equal(document.Geometry.MaxEraseCycles, []byte("null")) {
		var exact fixtureInteger
		if err := json.Unmarshal(document.Geometry.MaxEraseCycles, &exact); err != nil || exact < 0 || uint64(exact) > maxSafeEraseCycles {
			return nil, errors.New("max_erase_cycles is invalid")
		}
		limit := uint64(exact)
		eraseLimit = &limit
	}
	blockCount, err := hostInt(blocks, "block_count")
	if err != nil {
		return nil, err
	}
	pagesPerBlockInt, err := hostInt(pagesPerBlock, "pages_per_block")
	if err != nil {
		return nil, err
	}
	pageSizeInt, err := hostInt(pageSize, "page_size_bytes")
	if err != nil {
		return nil, err
	}
	device, err := New(Config{
		Geometry:       Geometry{Blocks: blockCount, PagesPerBlock: pagesPerBlockInt, PageSizeBytes: pageSizeInt},
		MaxEraseCycles: eraseLimit,
	})
	if err != nil {
		return nil, err
	}
	if err := validateTraceState(document.InitialState, device); err != nil {
		return nil, errors.New("initial_state: " + err.Error())
	}
	for index, operation := range document.Operations {
		if operation == nil {
			return nil, errors.New("operation must not be null")
		}
		if err := validateTraceOperation(operation, document.StatesAfterOperations[index], device); err != nil {
			return nil, errors.New("operation " + strconv.Itoa(index) + ": " + err.Error())
		}
	}
	return device, nil
}

func validateTraceOperation(operation *traceOperation, state *traceState, device *Device) error {
	if operation.Op == nil || operation.Block == nil || state == nil {
		return errors.New("operation name, block and resulting state are required")
	}
	block64 := int64(*operation.Block)
	if block64 < 0 {
		return errors.New("block is negative")
	}
	block, err := hostInt(block64, "block")
	if err != nil {
		return err
	}
	var page int
	if *operation.Op == "read" || *operation.Op == "program" {
		page64, err := requiredInt(operation.Page, "page")
		if err != nil || page64 < 0 {
			return errors.New("page is missing, null or negative")
		}
		page, err = hostInt(page64, "page")
		if err != nil {
			return err
		}
	} else if *operation.Op == "erase_block" {
		if operation.Page != nil {
			return errors.New("erase_block must not contain page")
		}
	} else {
		return errors.New("unknown operation name")
	}

	var got error
	switch *operation.Op {
	case "read":
		var data []byte
		data, got = device.Read(block, page)
		if got == nil {
			var expected string
			if len(operation.Result) == 0 || json.Unmarshal(operation.Result, &expected) != nil || expected != strings.ToLower(expected) {
				return errors.New("read result must be lowercase hex string")
			}
			decoded, err := hex.DecodeString(expected)
			if err != nil || !reflect.DeepEqual(data, decoded) {
				return errors.New("read result differs from trace")
			}
		}
	case "program":
		if operation.Data == nil {
			return errors.New("program data is required and non-null")
		}
		data, err := hex.DecodeString(*operation.Data)
		if err != nil || *operation.Data != strings.ToLower(*operation.Data) {
			return errors.New("program data must be lowercase hex")
		}
		got = device.Program(block, page, data)
	case "erase_block":
		got = device.EraseBlock(block)
	}
	if err := matchTraceOutcome(operation, got); err != nil {
		return err
	}
	return validateTraceState(state, device)
}

func matchTraceOutcome(operation *traceOperation, got error) error {
	if operation.Error == nil {
		if got != nil {
			return errors.New("unexpected operation error: " + got.Error())
		}
		if len(operation.Result) == 0 {
			return errors.New("successful operation result is missing")
		}
		if *operation.Op != "read" && !bytes.Equal(operation.Result, []byte("null")) {
			return errors.New("successful program/erase result must be null")
		}
		return nil
	}
	if operation.Error.Type == nil || operation.Error.Message == nil {
		return errors.New("error type and message are required and non-null")
	}
	if len(operation.Result) != 0 {
		return errors.New("error operation must not include result")
	}
	var expected error
	switch *operation.Error.Type {
	case "ProgramError":
		expected = ErrProgrammed
	case "EraseLimitError":
		expected = ErrEraseLimit
	default:
		return errors.New("unknown error type")
	}
	if !errors.Is(got, expected) || got.Error() != *operation.Error.Message {
		return errors.New("operation error or message differs from trace")
	}
	return nil
}

func validateTraceState(expected *traceState, device *Device) error {
	snapshot, err := device.Snapshot()
	if err != nil {
		return err
	}
	geometry, err := device.Geometry()
	if err != nil {
		return err
	}
	if len(expected.Pages) != geometry.Blocks || len(expected.EraseCounts) != geometry.Blocks || len(expected.Programmed) != geometry.Blocks {
		return errors.New("block dimensions differ")
	}
	for block := 0; block < geometry.Blocks; block++ {
		if len(expected.Pages[block]) != geometry.PagesPerBlock || len(expected.Programmed[block]) != geometry.PagesPerBlock || expected.EraseCounts[block] == nil || *expected.EraseCounts[block] < 0 || uint64(*expected.EraseCounts[block]) != snapshot.EraseCounts[block] {
			return errors.New("page or erase-count dimensions differ")
		}
		for page := 0; page < geometry.PagesPerBlock; page++ {
			if expected.Pages[block][page] == nil || expected.Programmed[block][page] == nil {
				return errors.New("page bytes and marker must not be null")
			}
			if *expected.Pages[block][page] != snapshot.Pages[block][page] || *expected.Programmed[block][page] != snapshot.Programmed[block][page] {
				return errors.New("page bytes or program marker differ")
			}
		}
	}
	return nil
}

func TestCanonicalNANDTrace(t *testing.T) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "nand-flash.json")
	document, err := readTrace(path)
	if err != nil {
		t.Fatal(err)
	}
	_, err = validateTrace(document)
	if err != nil {
		t.Fatal(err)
	}
}

func TestTraceReaderRejectsMalformedDocuments(t *testing.T) {
	cases := map[string]string{
		"duplicate-key":  `{"format":"a","format":"b"}`,
		"trailing-value": `{} {}`,
		"wrong-root":     `null`,
		"invalid-utf8":   string([]byte{'{', '"', 'x', '"', ':', '"', 0xFF, '"', '}'}),
		"surrogate":      `{"x":"\ud800"}`,
		"too-deep":       "{" + strings.Repeat(`"x":[`, maxFixtureDepth) + "0" + strings.Repeat("]", maxFixtureDepth) + "}",
		"too-large":      `{"x":"` + strings.Repeat("x", maxFixtureBytes) + `"}`,
	}
	if err := validateSurrogateEscapes([]byte(`{"x":"\ud83d\ude00"}`)); err != nil {
		t.Fatalf("valid surrogate pair rejected: %v", err)
	}
	for name, input := range cases {
		t.Run(name, func(t *testing.T) {
			if _, err := decodeTrace([]byte(input)); err == nil {
				t.Fatal("malformed input was accepted")
			}
		})
	}
	var number fixtureInteger
	for _, raw := range []string{`null`, `1.0`, `1e0`, `9007199254740992`} {
		if err := json.Unmarshal([]byte(raw), &number); err == nil {
			t.Errorf("integer decoder accepted %s", raw)
		}
	}
}

func TestTraceAdapterRejectsLargeHostAddress(t *testing.T) {
	if strconv.IntSize != 32 {
		t.Skip("host-int address overflow is specific to 32-bit Go")
	}
	fixture := []byte(`{"format":"coding-adventures-nand-flash-v1","description":"x","geometry":{"block_count":1,"pages_per_block":1,"page_size_bytes":1,"max_erase_cycles":null},"initial_state":{"pages":[["ff"]],"erase_counts":[0],"programmed":[[false]]},"operations":[{"op":"read","block":2147483648,"page":0,"result":"ff"}],"states_after_operations":[{"pages":[["ff"]],"erase_counts":[0],"programmed":[[false]]}]}`)
	document, err := decodeTrace(fixture)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := validateTrace(document); err == nil || !strings.Contains(err.Error(), "operation 0: block exceeds host int range") {
		t.Fatalf("adapter error = %v; want 32-bit block conversion refusal", err)
	}
}

func TestExactFixtureObjectMembers(t *testing.T) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "nand-flash.json")
	canonical, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	admit := func(raw []byte) error {
		document, err := decodeTrace(raw)
		if err != nil {
			return err
		}
		_, err = validateTrace(document)
		return err
	}
	if err := admit(canonical); err != nil {
		t.Fatalf("canonical null controls refused: %v", err)
	}

	mutate := func(label string, from, to []byte) []byte {
		t.Helper()
		changed := bytes.Replace(canonical, from, to, 1)
		if bytes.Equal(changed, canonical) {
			t.Fatalf("%s mutation anchor not found", label)
		}
		return changed
	}
	reject := map[string][]byte{
		"root alias":                   mutate("root alias", []byte(`"format"`), []byte(`"FORMAT"`)),
		"root alias beside exact":      mutate("root alias beside exact", []byte(`"format": "coding-adventures-nand-flash-v1",`), []byte(`"format": "coding-adventures-nand-flash-v1", "FORMAT": "coding-adventures-nand-flash-v1",`)),
		"geometry alias":               mutate("geometry alias", []byte(`"block_count"`), []byte(`"Block_Count"`)),
		"geometry alias beside exact":  mutate("geometry alias beside exact", []byte(`"block_count": 2,`), []byte(`"block_count": 2, "BLOCK_COUNT": 2,`)),
		"state alias":                  mutate("state alias", []byte(`"erase_counts"`), []byte(`"ERASE_COUNTS"`)),
		"state alias beside exact":     mutate("state alias beside exact", []byte(`"pages": [`), []byte(`"ERASE_COUNTS": [0, 0], "pages": [`)),
		"operation alias":              mutate("operation alias", []byte(`"op"`), []byte(`"OP"`)),
		"operation alias beside exact": mutate("operation alias beside exact", []byte(`"op": "program",`), []byte(`"op": "program", "OP": "program",`)),
		"error alias":                  mutate("error alias", []byte(`"message"`), []byte(`"MESSAGE"`)),
		"error alias beside exact":     mutate("error alias beside exact", []byte(`"type": "ProgramError",`), []byte(`"type": "ProgramError", "TYPE": "ProgramError",`)),
		"erase nullable extra":         mutate("erase nullable extra", []byte(`"op": "erase_block",`), []byte(`"op": "erase_block", "page": null,`)),
		"read unused data":             mutate("read unused data", []byte(`"op": "read",`), []byte(`"op": "read", "data": "ffff",`)),
		"success read null error":      mutate("success read null error", []byte(`"op": "read",`), []byte(`"op": "read", "error": null,`)),
		"conflicting outcomes":         mutate("conflicting outcomes", []byte(`"result": "ffff"`), []byte(`"result": "ffff", "error": null`)),
		"nested extra error field":     mutate("nested extra error field", []byte(`"type": "ProgramError"`), []byte(`"type": "ProgramError", "extra": 1`)),
		"missing geometry member":      mutate("missing geometry member", []byte(`"page_size_bytes": 2,`), nil),
		"missing operation page":       mutate("missing operation page", []byte(`"page": 0,`), nil),
	}
	for name, raw := range reject {
		t.Run(name, func(t *testing.T) {
			if err := admit(raw); err == nil {
				t.Fatal("non-exact fixture object was accepted")
			}
		})
	}

	// JSON spelling may differ while the decoded exact key remains identical.
	escapedKey := mutate("escaped exact key", []byte(`"format"`), []byte(`"for\u006dat"`))
	if err := admit(escapedKey); err != nil {
		t.Fatalf("escaped exact key refused: %v", err)
	}
	escapedUnicode := mutate("escaped Unicode description", []byte("Portable logical SLC NAND"), []byte(`Portable logical SLC NAND \u03bb`))
	if err := admit(escapedUnicode); err != nil {
		t.Fatalf("escaped Unicode description refused: %v", err)
	}
}

func FuzzExactFixtureObjects(f *testing.F) {
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "database-media-v1", "nand-flash.json")
	canonical, err := os.ReadFile(path)
	if err != nil {
		f.Fatal(err)
	}
	f.Add(canonical)
	f.Add([]byte(`{"format":"x","FORMAT":"y"}`))
	f.Add([]byte(`{"operations":[{"op":"erase_block","page":null}]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > maxFixtureBytes {
			return
		}
		document, err := decodeTrace(raw)
		if err == nil {
			_, _ = validateTrace(document)
		}
	})
}
