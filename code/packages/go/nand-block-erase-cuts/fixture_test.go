package nandblockerasecuts

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"reflect"
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"
)

const (
	fixturePath     = "../../../specs/fixtures/database-media-v1/nand-block-erase-cuts.json"
	fixtureSchema   = "coding-adventures.database-media.nand-block-erase-cuts.v1"
	fixtureMaxBytes = 64 * 1024
	fixtureMaxDepth = 32
	fixtureMaxCases = 32
	fixtureMaxSteps = 64
	maxHistoryBytes = 4096
	maxHistoryOps   = 64
)

type fixtureDocument struct {
	Cases []fixtureCase
}

type fixtureCase struct {
	Name     string
	PageSize int
	Ceiling  uint64
	Initial  Block
	Steps    []fixtureStep
}

type fixtureStep struct {
	Cut      Cut
	Expected fixtureExpected
}

type fixtureExpected struct {
	Block        Block
	Acknowledged bool
	ErrorCode    string
}

func readCanonicalFixture(t testing.TB) []byte {
	t.Helper()
	data, err := os.ReadFile(fixturePath)
	if err != nil {
		t.Fatalf("read canonical erase-cut fixture: %v", err)
	}
	if len(data) > fixtureMaxBytes {
		t.Fatalf("canonical fixture has %d bytes, cap is %d", len(data), fixtureMaxBytes)
	}
	return data
}

func TestCanonicalEraseCutFixtureIsModelCheckedBeforeRuntime(t *testing.T) {
	document, err := parseFixture(readCanonicalFixture(t))
	if err != nil {
		t.Fatalf("decode canonical fixture: %v", err)
	}
	if !fixtureMatchesModel(document) {
		t.Fatal("canonical fixture disagrees with independent block/page/counter model")
	}
	replayFixture(t, document)
}

func TestFixtureReaderRejectsMalformedAndUnboundedDocuments(t *testing.T) {
	canonical := readCanonicalFixture(t)
	mutated := bytes.Replace(canonical, []byte(`"schema":`), []byte(`"schema":"duplicate","schema":`), 1)
	if bytes.Equal(mutated, canonical) {
		t.Fatal("duplicate-key mutation anchor was not present")
	}
	escapedDuplicate := []byte(`{"schema":"x","\u0073chema":"y","cases":[]}`)
	unknownField := bytes.Replace(canonical, []byte(`"schema":`), []byte(`"extra":1,"schema":`), 1)
	numericFloat := bytes.Replace(canonical, []byte(`"page_size": 2`), []byte(`"page_size": 2.0`), 1)
	if bytes.Equal(numericFloat, canonical) {
		t.Fatal("numeric token mutation anchor was not present")
	}

	for name, raw := range map[string][]byte{
		"trailing-data":      append(append([]byte(nil), canonical...), []byte("false")...),
		"duplicate-key":      mutated,
		"escaped-duplicate":  escapedDuplicate,
		"invalid-utf8":       {0xff},
		"unknown-field":      unknownField,
		"floating-integer":   numericFloat,
		"over-byte-limit":    bytes.Repeat([]byte(" "), fixtureMaxBytes+1),
		"over-depth-limit":   []byte(strings.Repeat("[", fixtureMaxDepth+1) + "0" + strings.Repeat("]", fixtureMaxDepth+1)),
		"truncated-document": canonical[:len(canonical)-2],
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := parseFixture(raw); err == nil {
				t.Fatalf("parseFixture accepted malformed input of %d bytes", len(raw))
			}
		})
	}
}

func TestFixtureReaderRejectsInvalidUnicodeScalars(t *testing.T) {
	canonical := readCanonicalFixture(t)
	name := `"name": "zero-prefix-and-before-accept-preserve-state-before-retry"`
	for label, replacement := range map[string]string{
		"isolated-high":    `"name": "case-\ud800"`,
		"isolated-low":     `"name": "case-\udc00"`,
		"reversed-pair":    `"name": "case-\udc00\ud800"`,
		"missing-low-half": `"name": "case-\ud800`,
		"incomplete-hex":   `"name": "case-\u12"`,
		"malformed-hex":    `"name": "case-\u12G4"`,
	} {
		t.Run(label, func(t *testing.T) {
			raw := bytes.Replace(canonical, []byte(name), []byte(replacement), 1)
			if bytes.Equal(raw, canonical) {
				t.Fatal("case-name mutation anchor was not found")
			}
			if _, err := parseFixture(raw); err == nil {
				t.Fatal("fixture reader accepted an invalid Unicode scalar")
			}
		})
	}
}

func TestFixtureReaderAcceptsValidUnicodeScalarSpellings(t *testing.T) {
	canonical := readCanonicalFixture(t)
	name := `"name": "zero-prefix-and-before-accept-preserve-state-before-retry"`
	for label, testCase := range map[string]struct{ replacement, wantName string }{
		"surrogate-pair":      {`"name": "case-\ud83d\ude00"`, "case-😀"},
		"literal-replacement": {`"name": "case-�"`, "case-�"},
		"escaped-replacement": {`"name": "case-\ufffd"`, "case-�"},
		"escaped-backslash":   {`"name": "case-\\uD800"`, `case-\uD800`},
	} {
		t.Run(label, func(t *testing.T) {
			raw := bytes.Replace(canonical, []byte(name), []byte(testCase.replacement), 1)
			if bytes.Equal(raw, canonical) {
				t.Fatal("case-name mutation anchor was not found")
			}
			document, err := parseFixture(raw)
			if err != nil {
				t.Fatalf("valid Unicode spelling refused: %v", err)
			}
			if got := document.Cases[0].Name; got != testCase.wantName {
				t.Fatalf("decoded name %q, want %q", got, testCase.wantName)
			}
			if !fixtureMatchesModel(document) {
				t.Fatal("Unicode-only name edit changed the independent erase model")
			}
			replayFixture(t, document)
		})
	}
}

func TestWellFormedLateStateCorruptionFailsIndependentModel(t *testing.T) {
	document, err := parseFixture(readCanonicalFixture(t))
	if err != nil {
		t.Fatal(err)
	}
	last := &document.Cases[0].Steps[len(document.Cases[0].Steps)-1].Expected.Block
	last.Pages[0].Programmed = !last.Pages[0].Programmed
	if fixtureMatchesModel(document) {
		t.Fatal("well-formed but incorrect late marker matched model")
	}
}

func FuzzParseFixture(f *testing.F) {
	canonical, err := os.ReadFile(fixturePath)
	if err != nil {
		f.Fatalf("read canonical fuzz seed: %v", err)
	}
	f.Add(canonical)
	f.Add([]byte(`{"schema":"x","\u0073chema":"y","cases":[]}`))
	f.Add([]byte(`{"schema":"coding-adventures.database-media.nand-block-erase-cuts.v1","cases":[]}`))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > fixtureMaxBytes {
			return
		}
		document, parseErr := parseFixture(raw)
		if parseErr != nil {
			return
		}
		// Expected fields are untrusted until the independent model accepts all
		// steps. A corrupt vector is input refusal, not a runtime disagreement.
		if !fixtureMatchesModel(document) {
			return
		}
		replayFixture(t, document)
	})
}

func FuzzBlockHistory(f *testing.F) {
	f.Add([]byte{3, 2, 3, 1, 0, 0, 1, 2, 2, 1, 3, 0, 4, 0})
	f.Add([]byte{0, 0, 0, 0, 1, 0, 2, 1})
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > maxHistoryBytes {
			return
		}
		checkFuzzHistory(t, raw)
	})
}

func parseFixture(raw []byte) (fixtureDocument, error) {
	if len(raw) == 0 || len(raw) > fixtureMaxBytes || !utf8.Valid(raw) {
		return fixtureDocument{}, errors.New("invalid fixture byte sequence or size")
	}
	if err := validateSurrogateEscapes(raw); err != nil {
		return fixtureDocument{}, err
	}
	if err := scanJSON(raw); err != nil {
		return fixtureDocument{}, err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	var value any
	if err := decoder.Decode(&value); err != nil {
		return fixtureDocument{}, err
	}
	var extra any
	if err := decoder.Decode(&extra); err != io.EOF {
		return fixtureDocument{}, errors.New("fixture contains trailing JSON value")
	}

	root, err := object(value, "schema", "cases")
	if err != nil {
		return fixtureDocument{}, err
	}
	schema, ok := root["schema"].(string)
	if !ok || schema != fixtureSchema {
		return fixtureDocument{}, errors.New("unknown fixture schema")
	}
	cases, ok := root["cases"].([]any)
	if !ok || len(cases) == 0 || len(cases) > fixtureMaxCases {
		return fixtureDocument{}, errors.New("fixture case count outside bounds")
	}
	document := fixtureDocument{Cases: make([]fixtureCase, 0, len(cases))}
	for caseIndex, rawCase := range cases {
		parsed, parseErr := parseFixtureCase(rawCase)
		if parseErr != nil {
			return fixtureDocument{}, fmt.Errorf("case %d: %w", caseIndex, parseErr)
		}
		document.Cases = append(document.Cases, parsed)
	}
	return document, nil
}

func parseFixtureCase(value any) (fixtureCase, error) {
	fields, err := object(value, "name", "page_size", "ceiling", "initial_block", "steps")
	if err != nil {
		return fixtureCase{}, err
	}
	name, ok := fields["name"].(string)
	if !ok || name == "" {
		return fixtureCase{}, errors.New("case name must be nonempty text")
	}
	pageSize64, err := signedInteger(fields["page_size"])
	if err != nil || int64(int(pageSize64)) != pageSize64 {
		return fixtureCase{}, errors.New("page_size must be a representable integer token")
	}
	ceiling, err := unsignedInteger(fields["ceiling"])
	if err != nil {
		return fixtureCase{}, errors.New("ceiling must be an unsigned integer token")
	}
	block, err := parseBlock(fields["initial_block"])
	if err != nil {
		return fixtureCase{}, fmt.Errorf("initial_block: %w", err)
	}
	rawSteps, ok := fields["steps"].([]any)
	if !ok || len(rawSteps) == 0 || len(rawSteps) > fixtureMaxSteps {
		return fixtureCase{}, errors.New("steps count outside bounds")
	}
	result := fixtureCase{Name: name, PageSize: int(pageSize64), Ceiling: ceiling, Initial: block}
	for stepIndex, rawStep := range rawSteps {
		step, stepErr := parseFixtureStep(rawStep)
		if stepErr != nil {
			return fixtureCase{}, fmt.Errorf("step %d: %w", stepIndex, stepErr)
		}
		result.Steps = append(result.Steps, step)
	}
	return result, nil
}

func parseFixtureStep(value any) (fixtureStep, error) {
	fields, err := object(value, "cut", "expected")
	if err != nil {
		return fixtureStep{}, err
	}
	cut, err := parseCut(fields["cut"])
	if err != nil {
		return fixtureStep{}, err
	}
	expected, err := parseExpected(fields["expected"])
	if err != nil {
		return fixtureStep{}, err
	}
	return fixtureStep{Cut: cut, Expected: expected}, nil
}

func parseCut(value any) (Cut, error) {
	fields, ok := value.(map[string]any)
	if !ok {
		return Cut{}, errors.New("cut must be an object")
	}
	kind, ok := fields["kind"].(string)
	if !ok {
		return Cut{}, errors.New("cut kind must be text")
	}
	cut := Cut{Kind: CutKind(255)}
	switch kind {
	case "before_accept":
		cut.Kind = BeforeAccept
	case "after_prefix":
		cut.Kind = AfterPrefix
	case "after_media":
		cut.Kind = AfterMedia
	case "complete":
		cut.Kind = Complete
	}
	if kind == "after_prefix" {
		if err := exactKeys(fields, "kind", "prefix_pages"); err != nil {
			return Cut{}, err
		}
		prefix, err := signedInteger(fields["prefix_pages"])
		if err != nil || int64(int(prefix)) != prefix {
			return Cut{}, errors.New("prefix_pages must be an integer token")
		}
		cut.PrefixPages = int(prefix)
	} else if err := exactKeys(fields, "kind"); err != nil {
		return Cut{}, err
	}
	return cut, nil
}

func parseExpected(value any) (fixtureExpected, error) {
	fields, ok := value.(map[string]any)
	if !ok {
		return fixtureExpected{}, errors.New("expected result must be an object")
	}
	if errorCode, hasError := fields["error"]; hasError {
		if err := exactKeys(fields, "error"); err != nil {
			return fixtureExpected{}, err
		}
		text, ok := errorCode.(string)
		if !ok || !knownErrorCode(text) {
			return fixtureExpected{}, errors.New("expected error code is unknown")
		}
		return fixtureExpected{ErrorCode: text}, nil
	}
	if err := exactKeys(fields, "block", "acknowledged"); err != nil {
		return fixtureExpected{}, err
	}
	block, err := parseBlock(fields["block"])
	if err != nil {
		return fixtureExpected{}, err
	}
	ack, ok := fields["acknowledged"].(bool)
	if !ok {
		return fixtureExpected{}, errors.New("acknowledged must be boolean")
	}
	return fixtureExpected{Block: block, Acknowledged: ack}, nil
}

func parseBlock(value any) (Block, error) {
	fields, err := object(value, "pages", "completed_erases", "incomplete_erase")
	if err != nil {
		return Block{}, err
	}
	pageValues, ok := fields["pages"].([]any)
	if !ok || len(pageValues) > maxPagesPerBlock+1 {
		return Block{}, errors.New("pages must be a bounded array")
	}
	count, err := unsignedInteger(fields["completed_erases"])
	if err != nil {
		return Block{}, errors.New("completed_erases must be an unsigned integer token")
	}
	incomplete, ok := fields["incomplete_erase"].(bool)
	if !ok {
		return Block{}, errors.New("incomplete_erase must be boolean")
	}
	block := Block{CompletedErases: count, IncompleteErase: incomplete, Pages: make([]Page, 0, len(pageValues))}
	for _, rawPage := range pageValues {
		pageFields, pageErr := object(rawPage, "bytes_hex", "programmed")
		if pageErr != nil {
			return Block{}, pageErr
		}
		hexBytes, ok := pageFields["bytes_hex"].(string)
		if !ok || !isLowerHex(hexBytes) {
			return Block{}, errors.New("page bytes_hex must be lowercase even-width hex")
		}
		data, decodeErr := hex.DecodeString(hexBytes)
		if decodeErr != nil {
			return Block{}, decodeErr
		}
		programmed, ok := pageFields["programmed"].(bool)
		if !ok {
			return Block{}, errors.New("programmed must be boolean")
		}
		block.Pages = append(block.Pages, Page{Bytes: data, Programmed: programmed})
	}
	return block, nil
}

func object(value any, keys ...string) (map[string]any, error) {
	fields, ok := value.(map[string]any)
	if !ok {
		return nil, errors.New("value must be an object")
	}
	if err := exactKeys(fields, keys...); err != nil {
		return nil, err
	}
	return fields, nil
}

func exactKeys(fields map[string]any, expected ...string) error {
	if len(fields) != len(expected) {
		return errors.New("object fields differ from schema")
	}
	for _, key := range expected {
		if _, ok := fields[key]; !ok {
			return fmt.Errorf("required field %q is missing", key)
		}
	}
	return nil
}

func knownErrorCode(code string) bool {
	switch code {
	case "invalid_page_size", "invalid_block_state", "invalid_erase_count", "invalid_cut", "erase_limit":
		return true
	default:
		return false
	}
}

func signedInteger(value any) (int64, error) {
	number, ok := value.(json.Number)
	if !ok {
		return 0, errors.New("number token required")
	}
	return number.Int64()
}

func unsignedInteger(value any) (uint64, error) {
	number, ok := value.(json.Number)
	if !ok {
		return 0, errors.New("number token required")
	}
	return strconv.ParseUint(number.String(), 10, 64)
}

func isLowerHex(value string) bool {
	if len(value)%2 != 0 {
		return false
	}
	for _, digit := range []byte(value) {
		if !((digit >= '0' && digit <= '9') || (digit >= 'a' && digit <= 'f')) {
			return false
		}
	}
	return true
}

// encoding/json replaces unpaired UTF-16 surrogates with U+FFFD. Reject those
// escaped code units before token decoding so fixture names remain exact.
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

func scanJSON(raw []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	if err := scanValue(decoder, 0); err != nil {
		return err
	}
	if _, err := decoder.Token(); err != io.EOF {
		return errors.New("JSON document is not complete")
	}
	return nil
}

func scanValue(decoder *json.Decoder, depth int) error {
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	delimiter, isDelimiter := token.(json.Delim)
	if !isDelimiter {
		return nil
	}
	depth++
	if depth > fixtureMaxDepth {
		return errors.New("JSON nesting exceeds limit")
	}
	switch delimiter {
	case '{':
		seen := make(map[string]struct{})
		for decoder.More() {
			keyToken, keyErr := decoder.Token()
			if keyErr != nil {
				return keyErr
			}
			key, ok := keyToken.(string)
			if !ok {
				return errors.New("object key is not text")
			}
			if _, exists := seen[key]; exists {
				return errors.New("duplicate decoded JSON object key")
			}
			seen[key] = struct{}{}
			if err := scanValue(decoder, depth); err != nil {
				return err
			}
		}
		closing, closeErr := decoder.Token()
		if closeErr != nil || closing != json.Delim('}') {
			return errors.New("object is not closed")
		}
	case '[':
		for decoder.More() {
			if err := scanValue(decoder, depth); err != nil {
				return err
			}
		}
		closing, closeErr := decoder.Token()
		if closeErr != nil || closing != json.Delim(']') {
			return errors.New("array is not closed")
		}
	default:
		return errors.New("unexpected JSON delimiter")
	}
	return nil
}

func fixtureMatchesModel(document fixtureDocument) bool {
	for _, entry := range document.Cases {
		state := cloneForOracle(entry.Initial)
		for _, step := range entry.Steps {
			want, acknowledged, errorCode := referenceApply(entry.PageSize, entry.Ceiling, state, step.Cut)
			if errorCode != step.Expected.ErrorCode {
				return false
			}
			if errorCode == "" {
				if acknowledged != step.Expected.Acknowledged || !reflect.DeepEqual(want, step.Expected.Block) {
					return false
				}
				state = want
			} else if step.Expected.Acknowledged || len(step.Expected.Block.Pages) != 0 || step.Expected.Block.CompletedErases != 0 || step.Expected.Block.IncompleteErase {
				return false
			}
		}
	}
	return true
}

func replayFixture(t *testing.T, document fixtureDocument) {
	t.Helper()
	for _, entry := range document.Cases {
		state := cloneForOracle(entry.Initial)
		for stepIndex, step := range entry.Steps {
			before := cloneForOracle(state)
			got, err := Apply(entry.PageSize, entry.Ceiling, state, step.Cut)
			if step.Expected.ErrorCode != "" {
				assertRefusal(t, got, err, errorForCode(step.Expected.ErrorCode))
				if !reflect.DeepEqual(state, before) {
					t.Fatalf("%s step %d refusal mutated input block", entry.Name, stepIndex)
				}
				continue
			}
			if err != nil {
				t.Fatalf("%s step %d returned unexpected error: %v", entry.Name, stepIndex, err)
			}
			if got.Acknowledged != step.Expected.Acknowledged || !reflect.DeepEqual(got.Block, step.Expected.Block) {
				t.Fatalf("%s step %d runtime result = %+v; want %+v", entry.Name, stepIndex, got, step.Expected)
			}
			state = got.Block
		}
	}
}

func checkFuzzHistory(t *testing.T, raw []byte) {
	t.Helper()
	if len(raw) < 4 {
		return
	}
	pageCount := int(raw[0]%4) + 1
	pageSize := int(raw[1]%8) + 1
	ceiling := uint64(raw[2] % 5)
	state := Block{
		Pages:           make([]Page, pageCount),
		CompletedErases: uint64(raw[3]) % (ceiling + 1),
		IncompleteErase: raw[3]&0x80 != 0,
	}
	for pageIndex := range state.Pages {
		page := Page{Bytes: make([]byte, pageSize), Programmed: raw[(pageIndex+4)%len(raw)]&1 != 0}
		for offset := range page.Bytes {
			page.Bytes[offset] = raw[(pageIndex*pageSize+offset+4)%len(raw)]
		}
		if !page.Programmed {
			for offset := range page.Bytes {
				page.Bytes[offset] = 0xff
			}
		}
		state.Pages[pageIndex] = page
	}

	commandCount := (len(raw) - 4) / 2
	if commandCount > maxHistoryOps {
		commandCount = maxHistoryOps
	}
	for command := 0; command < commandCount; command++ {
		kindByte := raw[4+2*command]
		prefixByte := raw[5+2*command]
		prefix := int(prefixByte%byte(pageCount+2)) - 1
		cut := Cut{Kind: CutKind(kindByte % 5), PrefixPages: prefix}
		before := cloneForOracle(state)
		want, wantAck, wantErr := referenceApply(pageSize, ceiling, state, cut)
		got, err := Apply(pageSize, ceiling, state, cut)
		assertTransition(t, got, err, want, wantAck, wantErr)
		if wantErr == "" {
			state = got.Block
		} else if !reflect.DeepEqual(state, before) {
			t.Fatalf("history command %d changed caller block on refusal", command)
		}
	}
}
