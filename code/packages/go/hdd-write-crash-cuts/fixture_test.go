package hddwritecrashcuts

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
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"
)

const (
	fixturePath    = "../../../specs/fixtures/database-media-v1/hdd-write-crash-cuts.json"
	fixtureSchema  = "coding-adventures.database-media.hdd-write-crash-cuts.v1"
	fixtureDigest  = "2a916447ba7327b171b8d64baf8a690b99607fdf1bae68a1e48e637dcf8d2ea1"
	maxFixtureSize = 64 * 1024
	maxJSONDepth   = 32
)

type declaredCase struct {
	outcome string
	refusal string
	receipt *Receipt
	prefix  *int
	after   [][]byte
}

type fixtureCase struct {
	name     string
	before   [][]byte
	lba      int64
	data     []byte
	cut      Cut
	declared declaredCase
}

type fixture struct {
	cases []fixtureCase
}

func readCanonicalFixture(t testing.TB) []byte {
	t.Helper()
	file, err := os.Open(fixturePath)
	if err != nil {
		t.Fatal(err)
	}
	defer file.Close()
	data, err := io.ReadAll(io.LimitReader(file, maxFixtureSize+1))
	if err != nil {
		t.Fatal(err)
	}
	if len(data) > maxFixtureSize {
		t.Fatalf("canonical fixture exceeds %d byte cap", maxFixtureSize)
	}
	return data
}

func parseFixture(data []byte) (fixture, error) {
	var result fixture
	if len(data) == 0 || len(data) > maxFixtureSize {
		return result, fmt.Errorf("fixture length outside 1..%d", maxFixtureSize)
	}
	if !utf8.Valid(data) {
		return result, errors.New("fixture is not valid UTF-8")
	}
	if err := scanJSON(data); err != nil {
		return result, err
	}
	root, err := exactObject(data, "schema", "profile", "cases")
	if err != nil {
		return result, fmt.Errorf("top-level object: %w", err)
	}
	schema, err := stringValue(root["schema"])
	if err != nil || schema != fixtureSchema {
		return result, errors.New("unsupported fixture schema")
	}
	profile, err := exactObject(root["profile"], "sector_count", "bytes_per_sector", "media_write_order")
	if err != nil {
		return result, fmt.Errorf("profile: %w", err)
	}
	sectorCount, err := integerValue(profile["sector_count"])
	if err != nil || sectorCount != SectorCount {
		return result, errors.New("profile sector_count mismatch")
	}
	bytesPerSector, err := integerValue(profile["bytes_per_sector"])
	if err != nil || bytesPerSector != BytesPerSector {
		return result, errors.New("profile bytes_per_sector mismatch")
	}
	order, err := stringValue(profile["media_write_order"])
	if err != nil || order != "ascending_offset" {
		return result, errors.New("profile media_write_order mismatch")
	}
	var rawCases []json.RawMessage
	if err := json.Unmarshal(root["cases"], &rawCases); err != nil || len(rawCases) == 0 {
		return result, errors.New("cases must be a nonempty array")
	}
	seenNames := make(map[string]struct{}, len(rawCases))
	for index, rawCase := range rawCases {
		parsed, err := parseCase(rawCase)
		if err != nil {
			return fixture{}, fmt.Errorf("case %d: %w", index, err)
		}
		if _, exists := seenNames[parsed.name]; exists {
			return fixture{}, fmt.Errorf("duplicate case name %q", parsed.name)
		}
		seenNames[parsed.name] = struct{}{}
		if err := validateDeclaration(parsed); err != nil {
			return fixture{}, fmt.Errorf("case %q declaration: %w", parsed.name, err)
		}
		result.cases = append(result.cases, parsed)
	}
	return result, nil
}

func parseCase(raw json.RawMessage) (fixtureCase, error) {
	var result fixtureCase
	fields, err := exactObject(raw, "name", "before_sectors_hex", "lba", "data_hex", "cut", "expected")
	if err != nil {
		return result, err
	}
	result.name, err = stringValue(fields["name"])
	if err != nil || strings.TrimSpace(result.name) == "" {
		return result, errors.New("case name must be nonempty text")
	}
	result.before, err = parseImage(fields["before_sectors_hex"])
	if err != nil {
		return result, fmt.Errorf("before image: %w", err)
	}
	result.lba, err = integerValue(fields["lba"])
	if err != nil {
		return result, fmt.Errorf("LBA: %w", err)
	}
	dataText, err := stringValue(fields["data_hex"])
	if err != nil {
		return result, errors.New("data_hex must be text")
	}
	result.data, err = parseHexBytes(dataText, BytesPerSector)
	if err != nil {
		return result, fmt.Errorf("data_hex: %w", err)
	}
	result.cut, err = parseCut(fields["cut"])
	if err != nil {
		return result, fmt.Errorf("cut: %w", err)
	}
	result.declared, err = parseExpected(fields["expected"], result.lba)
	if err != nil {
		return result, fmt.Errorf("expected: %w", err)
	}
	return result, nil
}

func parseCut(raw json.RawMessage) (Cut, error) {
	fields, err := objectMap(raw)
	if err != nil {
		return Cut{}, err
	}
	kind, err := stringValue(fields["kind"])
	if err != nil {
		return Cut{}, errors.New("cut kind must be text")
	}
	cut := Cut{Kind: CutKind(kind)}
	switch cut.Kind {
	case BeforeMediaWrite, AfterMediaWriteBeforeResponse, Acknowledged:
		if err := requireKeys(fields, "kind"); err != nil {
			return Cut{}, err
		}
	case DuringMediaWrite:
		if err := requireKeys(fields, "kind", "written_prefix"); err != nil {
			return Cut{}, err
		}
		prefix, err := integerValue(fields["written_prefix"])
		if err != nil || prefix < 1 || prefix > BytesPerSector-1 {
			return Cut{}, errors.New("during-write prefix outside 1..7")
		}
		value := int(prefix)
		cut.WrittenPrefix = &value
	default:
		return Cut{}, errors.New("unknown cut kind")
	}
	return cut, nil
}

func parseExpected(raw json.RawMessage, lba int64) (declaredCase, error) {
	var result declaredCase
	fields, err := objectMap(raw)
	if err != nil {
		return result, err
	}
	validLBA := lba >= 0 && lba < SectorCount
	if validLBA {
		err = requireKeys(fields, "outcome", "receipt", "evidence", "after_sectors_hex")
	} else {
		err = requireKeys(fields, "outcome", "refusal", "receipt", "evidence", "after_sectors_hex")
	}
	if err != nil {
		return result, err
	}
	result.outcome, err = stringValue(fields["outcome"])
	if err != nil {
		return result, errors.New("outcome must be text")
	}
	if !validLBA {
		result.refusal, err = stringValue(fields["refusal"])
		if err != nil || result.refusal != ErrInvalidLBA.Error() {
			return result, errors.New("invalid-LBA case must declare invalid_lba refusal")
		}
	}
	if !bytes.Equal(bytes.TrimSpace(fields["receipt"]), []byte("null")) {
		receiptFields, err := exactObject(fields["receipt"], "lba", "data_hex")
		if err != nil {
			return result, fmt.Errorf("receipt: %w", err)
		}
		receiptLBA, err := integerValue(receiptFields["lba"])
		if err != nil {
			return result, errors.New("receipt LBA must be an integer")
		}
		receiptHex, err := stringValue(receiptFields["data_hex"])
		if err != nil {
			return result, errors.New("receipt data_hex must be text")
		}
		data, err := parseHexBytes(receiptHex, BytesPerSector)
		if err != nil {
			return result, fmt.Errorf("receipt data: %w", err)
		}
		value := &Receipt{LBA: receiptLBA}
		copy(value.Data[:], data)
		result.receipt = value
	}
	if !bytes.Equal(bytes.TrimSpace(fields["evidence"]), []byte("null")) {
		evidence, err := exactObject(fields["evidence"], "persisted_prefix_bytes")
		if err != nil {
			return result, fmt.Errorf("evidence: %w", err)
		}
		prefix, err := integerValue(evidence["persisted_prefix_bytes"])
		if err != nil || prefix < 0 || prefix > BytesPerSector {
			return result, errors.New("persisted prefix evidence outside 0..8")
		}
		value := int(prefix)
		result.prefix = &value
	}
	result.after, err = parseImage(fields["after_sectors_hex"])
	if err != nil {
		return result, fmt.Errorf("after image: %w", err)
	}
	return result, nil
}

func validateDeclaration(testCase fixtureCase) error {
	declared := testCase.declared
	if testCase.lba < 0 || testCase.lba >= SectorCount {
		if declared.outcome != "refused" || declared.refusal != ErrInvalidLBA.Error() ||
			declared.receipt != nil || declared.prefix != nil || !equalImage(declared.after, testCase.before) {
			return errors.New("invalid-LBA declaration is inconsistent")
		}
		return nil
	}
	want := oracleResult(testCase.before, testCase.lba, testCase.data, testCase.cut)
	if declared.outcome != want.outcome || declared.refusal != "" ||
		!reflectReceiptEqual(declared.receipt, want.receipt) ||
		!intPointerEqual(declared.prefix, want.prefix) || !equalImage(declared.after, want.image) {
		return errors.New("declared result disagrees with independent byte-array oracle")
	}
	return nil
}

type oracleOutput struct {
	image   [][]byte
	outcome string
	receipt *Receipt
	prefix  *int
}

func oracleResult(before [][]byte, lba int64, data []byte, cut Cut) oracleOutput {
	out := oracleOutput{image: cloneImage(before), outcome: "crashed_without_receipt"}
	var persisted int
	switch cut.Kind {
	case BeforeMediaWrite:
		persisted = 0
	case DuringMediaWrite:
		persisted = *cut.WrittenPrefix
		for offset := 0; offset < persisted; offset++ {
			out.image[int(lba)][offset] = data[offset]
		}
	case AfterMediaWriteBeforeResponse:
		persisted = 8
		for offset := 0; offset < BytesPerSector; offset++ {
			out.image[int(lba)][offset] = data[offset]
		}
	case Acknowledged:
		persisted = 8
		for offset := 0; offset < BytesPerSector; offset++ {
			out.image[int(lba)][offset] = data[offset]
		}
		out.outcome = "acknowledged"
		value := &Receipt{LBA: lba}
		copy(value.Data[:], data)
		out.receipt = value
	}
	out.prefix = &persisted
	return out
}

func replayFixture(t *testing.T, parsed fixture) {
	t.Helper()
	for _, testCase := range parsed.cases {
		before := cloneImage(testCase.before)
		result, err := Apply(testCase.before, testCase.lba, testCase.data, testCase.cut)
		if testCase.lba < 0 || testCase.lba >= SectorCount {
			if !errors.Is(err, ErrInvalidLBA) || !reflect.DeepEqual(result, Result{}) {
				t.Fatalf("case %q invalid-LBA runtime result=%+v error=%v", testCase.name, result, err)
			}
			if !equalImage(testCase.before, before) {
				t.Fatalf("case %q refusal changed its input image", testCase.name)
			}
			continue
		}
		if err != nil {
			t.Fatalf("case %q runtime refusal: %v", testCase.name, err)
		}
		want := testCase.declared
		if !equalImage(result.Image, want.after) || !reflectReceiptEqual(result.Receipt, want.receipt) ||
			want.prefix == nil || result.PersistedPrefixBytes != *want.prefix {
			t.Fatalf("case %q runtime result=%+v; declaration=%+v", testCase.name, result, want)
		}
		if !equalImage(testCase.before, before) {
			t.Fatalf("case %q runtime changed caller input", testCase.name)
		}
	}
}

func parseImage(raw json.RawMessage) ([][]byte, error) {
	var sectors []string
	if err := json.Unmarshal(raw, &sectors); err != nil || len(sectors) != SectorCount {
		return nil, errors.New("sector array must contain exactly 32 strings")
	}
	image := make([][]byte, SectorCount)
	for index, sector := range sectors {
		data, err := parseHexBytes(sector, BytesPerSector)
		if err != nil {
			return nil, fmt.Errorf("sector %d: %w", index, err)
		}
		image[index] = data
	}
	return image, nil
}

func parseHexBytes(value string, size int) ([]byte, error) {
	if len(value) != size*2 || strings.ToLower(value) != value {
		return nil, fmt.Errorf("expected %d lowercase hex characters", size*2)
	}
	data, err := hex.DecodeString(value)
	if err != nil || len(data) != size {
		return nil, errors.New("invalid hexadecimal bytes")
	}
	return data, nil
}

func exactObject(raw json.RawMessage, keys ...string) (map[string]json.RawMessage, error) {
	fields, err := objectMap(raw)
	if err != nil {
		return nil, err
	}
	if err := requireKeys(fields, keys...); err != nil {
		return nil, err
	}
	return fields, nil
}

func objectMap(raw json.RawMessage) (map[string]json.RawMessage, error) {
	trimmed := bytes.TrimSpace(raw)
	if len(trimmed) == 0 || trimmed[0] != '{' {
		return nil, errors.New("expected object")
	}
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(trimmed, &fields); err != nil || fields == nil {
		return nil, errors.New("invalid object")
	}
	return fields, nil
}

func requireKeys(fields map[string]json.RawMessage, keys ...string) error {
	if len(fields) != len(keys) {
		return fmt.Errorf("object has %d fields; want %d", len(fields), len(keys))
	}
	for _, key := range keys {
		if _, exists := fields[key]; !exists {
			return fmt.Errorf("missing field %q", key)
		}
	}
	return nil
}

func stringValue(raw json.RawMessage) (string, error) {
	trimmed := bytes.TrimSpace(raw)
	if len(trimmed) == 0 || trimmed[0] != '"' {
		return "", errors.New("expected string token")
	}
	var value string
	if err := json.Unmarshal(trimmed, &value); err != nil {
		return "", err
	}
	return value, nil
}

func integerValue(raw json.RawMessage) (int64, error) {
	text := string(bytes.TrimSpace(raw))
	if text == "" || !isJSONInteger(text) {
		return 0, errors.New("expected integer token")
	}
	value, err := strconv.ParseInt(text, 10, 64)
	if err != nil {
		return 0, err
	}
	return value, nil
}

func isJSONInteger(text string) bool {
	if text == "0" {
		return true
	}
	index := 0
	if text[0] == '-' {
		index++
		if index == len(text) {
			return false
		}
	}
	if text[index] < '1' || text[index] > '9' {
		return false
	}
	for index++; index < len(text); index++ {
		if text[index] < '0' || text[index] > '9' {
			return false
		}
	}
	return true
}

func scanJSON(data []byte) error {
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	if err := scanJSONValue(decoder, 1); err != nil {
		return err
	}
	if _, err := decoder.Token(); err != io.EOF {
		return errors.New("trailing JSON value or invalid trailing bytes")
	}
	return nil
}

func scanJSONValue(decoder *json.Decoder, depth int) error {
	if depth > maxJSONDepth {
		return errors.New("JSON nesting exceeds reader limit")
	}
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	delim, ok := token.(json.Delim)
	if !ok {
		return nil
	}
	switch delim {
	case '{':
		seen := make(map[string]struct{})
		for decoder.More() {
			keyToken, err := decoder.Token()
			if err != nil {
				return err
			}
			key, ok := keyToken.(string)
			if !ok {
				return errors.New("object key is not text")
			}
			if _, exists := seen[key]; exists {
				return fmt.Errorf("duplicate object key %q", key)
			}
			seen[key] = struct{}{}
			if err := scanJSONValue(decoder, depth+1); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil || end != json.Delim('}') {
			return errors.New("unterminated object")
		}
	case '[':
		for decoder.More() {
			if err := scanJSONValue(decoder, depth+1); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil || end != json.Delim(']') {
			return errors.New("unterminated array")
		}
	default:
		return errors.New("unexpected JSON delimiter")
	}
	return nil
}

func equalImage(left, right [][]byte) bool { return reflect.DeepEqual(left, right) }

func reflectReceiptEqual(left, right *Receipt) bool { return reflect.DeepEqual(left, right) }

func intPointerEqual(left, right *int) bool { return reflect.DeepEqual(left, right) }

func TestCanonicalCrashCutFixtureModelFirst(t *testing.T) {
	data := readCanonicalFixture(t)
	if actual := fmt.Sprintf("%x", sha256.Sum256(data)); actual != fixtureDigest {
		t.Fatalf("shared fixture SHA-256=%s; want %s", actual, fixtureDigest)
	}
	parsed, err := parseFixture(data)
	if err != nil {
		t.Fatal(err)
	}
	if len(parsed.cases) != 12 {
		t.Fatalf("fixture has %d cases; want 12", len(parsed.cases))
	}
	replayFixture(t, parsed)
}

func TestFixtureRejectsMalformedAndWrongDeclarations(t *testing.T) {
	canonical := string(readCanonicalFixture(t))
	changed := func(old, replacement string) []byte {
		t.Helper()
		if strings.Count(canonical, old) != 1 {
			t.Fatalf("fixture anchor count for %q is %d", old, strings.Count(canonical, old))
		}
		return []byte(strings.Replace(canonical, old, replacement, 1))
	}
	large := bytes.Repeat([]byte{' '}, maxFixtureSize+1)
	malformed := []struct {
		name string
		data []byte
	}{
		{"duplicate key", []byte(`{"schema":"a","schema":"b"}`)},
		{"trailing value", append(readCanonicalFixture(t), []byte(` {}`)...)},
		{"truncated JSON", []byte(canonical[:len(canonical)-2])},
		{"boolean LBA", changed(`"lba": 32,`, `"lba": true,`)},
		{"floating LBA", changed(`"lba": 32,`, `"lba": 32.0,`)},
		{"malformed hex", changed(`"a0a1a23ca46275a7"`, `"g0a1a23ca46275a7"`)},
		{"wrong declared sector", changed(`"a0a1a23ca46275a7"`, `"a0a1a23ca46275a6"`)},
		{"unexpected cut field", changed(`"kind": "before_media_write"`, `"kind": "before_media_write", "written_prefix": 0`)},
		{"wrong evidence", changed(`"persisted_prefix_bytes": 3`, `"persisted_prefix_bytes": 4`)},
		{"duplicate name", changed(`"name": "during-prefix-6-at-last-lba"`, `"name": "during-prefix-5-at-first-lba"`)},
		{"oversized input", large},
	}
	for _, test := range malformed {
		t.Run(test.name, func(t *testing.T) {
			if _, err := parseFixture(test.data); err == nil {
				t.Fatal("malformed fixture accepted")
			}
		})
	}
}

func FuzzCrashCutFixture(f *testing.F) {
	canonical := readCanonicalFixture(f)
	f.Add(canonical)
	f.Add([]byte(`{"schema":"coding-adventures.database-media.hdd-write-crash-cuts.v1","schema":"duplicate"}`))
	f.Add([]byte(`{"schema":`))
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > maxFixtureSize {
			return
		}
		parsed, err := parseFixture(input)
		if err != nil {
			return
		}
		replayFixture(t, parsed)
	})
}
