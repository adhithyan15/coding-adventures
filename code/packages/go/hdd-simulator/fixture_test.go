package hddsimulator

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"
	"testing"
	"unicode/utf8"
)

const (
	fixturePath     = "../../../specs/fixtures/database-media-v1/hdd-simulator.json"
	maxFixtureBytes = 64 * 1024
	maxJSONDepth    = 64
)

type fixtureEnvelope struct {
	Schema       string            `json:"schema"`
	Profile      json.RawMessage   `json:"profile"`
	InitialState json.RawMessage   `json:"initial_state"`
	Cases        []json.RawMessage `json:"cases"`
	TimingProbes []json.RawMessage `json:"timing_probes"`
}

type fixtureCase struct {
	Name       string            `json:"name"`
	Operations []json.RawMessage `json:"operations"`
}

type fixtureOperation struct {
	Op             string          `json:"op"`
	LBA            json.RawMessage `json:"lba"`
	DataHex        *string         `json:"data_hex"`
	Timing         json.RawMessage `json:"timing"`
	State          json.RawMessage `json:"state"`
	Error          *string         `json:"error"`
	StateUnchanged *bool           `json:"state_unchanged"`
}

type fixtureState struct {
	Tick      json.RawMessage `json:"tick"`
	HeadTrack json.RawMessage `json:"head_track"`
}

type fixtureTiming struct {
	StartTick     json.RawMessage `json:"start_tick"`
	SeekTicks     json.RawMessage `json:"seek_ticks"`
	RotationTicks json.RawMessage `json:"rotation_ticks"`
	TransferTicks json.RawMessage `json:"transfer_ticks"`
	TotalTicks    json.RawMessage `json:"total_ticks"`
	EndTick       json.RawMessage `json:"end_tick"`
}

type fixtureProbe struct {
	Name         string          `json:"name"`
	StartTick    json.RawMessage `json:"start_tick"`
	HeadTrack    json.RawMessage `json:"head_track"`
	TargetTrack  json.RawMessage `json:"target_track"`
	TargetSector json.RawMessage `json:"target_sector"`
	Timing       json.RawMessage `json:"timing"`
	Error        *string         `json:"error"`
}

type parsedOperation struct {
	name          string
	lba           int
	invokeRuntime bool
	boolLBA       bool
	data          []byte
	timing        *IOTiming
	state         *fixtureStateValue
	err           error
}

type fixtureStateValue struct {
	tick uint64
	head int
}

type parsedCase struct {
	name string
	ops  []parsedOperation
}

type parsedProbe struct {
	name         string
	tick         uint64
	head         int
	targetTrack  int
	targetSector int
	timing       *IOTiming
	err          error
}

type parsedFixture struct {
	cases  []parsedCase
	probes []parsedProbe
}

func readSharedFixture(t *testing.T) []byte {
	t.Helper()
	file, err := os.Open(fixturePath)
	if err != nil {
		t.Fatal(err)
	}
	defer file.Close()
	data, err := io.ReadAll(io.LimitReader(file, maxFixtureBytes+1))
	if err != nil {
		t.Fatal(err)
	}
	if len(data) > maxFixtureBytes {
		t.Fatalf("fixture exceeds %d-byte reader cap", maxFixtureBytes)
	}
	return data
}

func parseFixture(data []byte) (parsedFixture, error) {
	var parsed parsedFixture
	if len(data) == 0 || len(data) > maxFixtureBytes {
		return parsed, fmt.Errorf("fixture byte length outside 1..%d", maxFixtureBytes)
	}
	if err := rejectDuplicateKeysAndTrailingJSON(data); err != nil {
		return parsed, err
	}
	if err := exactObjectKeys(data, "schema", "profile", "initial_state", "cases", "timing_probes"); err != nil {
		return parsed, fmt.Errorf("top-level shape: %w", err)
	}
	var envelope fixtureEnvelope
	if err := decodeStrict(data, &envelope); err != nil {
		return parsed, err
	}
	if envelope.Schema != "coding-adventures.database-media.hdd-simulator.v1" {
		return parsed, fmt.Errorf("unsupported schema %q", envelope.Schema)
	}
	if err := validateProfile(envelope.Profile); err != nil {
		return parsed, err
	}
	if err := validateInitial(envelope.InitialState); err != nil {
		return parsed, err
	}
	if len(envelope.Cases) == 0 || envelope.Cases == nil {
		return parsed, errors.New("cases must be a non-empty array")
	}
	if len(envelope.TimingProbes) == 0 || envelope.TimingProbes == nil {
		return parsed, errors.New("timing_probes must be a non-empty array")
	}
	seenNames := make(map[string]bool, len(envelope.Cases))
	for i, rawCase := range envelope.Cases {
		caseValue, err := parseCase(rawCase)
		if err != nil {
			return parsed, fmt.Errorf("case %d: %w", i, err)
		}
		if seenNames[caseValue.name] {
			return parsed, fmt.Errorf("duplicate case name %q", caseValue.name)
		}
		seenNames[caseValue.name] = true
		parsed.cases = append(parsed.cases, caseValue)
	}
	for i, rawProbe := range envelope.TimingProbes {
		probe, err := parseProbe(rawProbe)
		if err != nil {
			return parsed, fmt.Errorf("timing probe %d: %w", i, err)
		}
		parsed.probes = append(parsed.probes, probe)
	}
	return parsed, nil
}

func validateProfile(raw json.RawMessage) error {
	fields, err := exactObject(raw, "surfaces", "tracks_per_surface", "sectors_per_track", "bytes_per_sector", "seek_ticks_per_track", "transfer_ticks_per_sector")
	if err != nil {
		return fmt.Errorf("profile shape: %w", err)
	}
	values := []struct {
		name string
		key  string
		want int64
	}{
		{"surfaces", "surfaces", 2}, {"tracks_per_surface", "tracks_per_surface", 4},
		{"sectors_per_track", "sectors_per_track", 4}, {"bytes_per_sector", "bytes_per_sector", 8},
		{"seek_ticks_per_track", "seek_ticks_per_track", 3}, {"transfer_ticks_per_sector", "transfer_ticks_per_sector", 1},
	}
	for _, value := range values {
		got, err := exactInt64(fields[value.key])
		if err != nil || got != value.want {
			return fmt.Errorf("profile %s must be integer %d", value.name, value.want)
		}
	}
	return nil
}

func validateInitial(raw json.RawMessage) error {
	fields, err := exactObject(raw, "tick", "head_track", "sector_count", "sector_fill_byte")
	if err != nil {
		return fmt.Errorf("initial_state shape: %w", err)
	}
	expect := []struct {
		name string
		key  string
		want int64
	}{
		{"tick", "tick", 0}, {"head_track", "head_track", 0},
		{"sector_count", "sector_count", sectorCount}, {"sector_fill_byte", "sector_fill_byte", 0},
	}
	for _, field := range expect {
		got, err := exactInt64(fields[field.key])
		if err != nil || got != field.want {
			return fmt.Errorf("initial_state %s must be integer %d", field.name, field.want)
		}
	}
	return nil
}

func parseCase(raw json.RawMessage) (parsedCase, error) {
	var result parsedCase
	if err := exactObjectKeys(raw, "name", "operations"); err != nil {
		return result, err
	}
	var value fixtureCase
	if err := decodeStrict(raw, &value); err != nil {
		return result, err
	}
	if value.Name == "" || len(value.Operations) == 0 || value.Operations == nil {
		return result, errors.New("case name and non-empty operations are required")
	}
	result.name = value.Name
	for i, rawOperation := range value.Operations {
		operation, err := parseOperation(rawOperation)
		if err != nil {
			return result, fmt.Errorf("operation %d: %w", i, err)
		}
		result.ops = append(result.ops, operation)
	}
	return result, nil
}

func parseOperation(raw json.RawMessage) (parsedOperation, error) {
	var result parsedOperation
	var err error
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fields); err != nil || fields == nil {
		return result, errors.New("operation must be an object")
	}
	var value fixtureOperation
	if err := decodeStrict(raw, &value); err != nil {
		return result, err
	}
	if value.Op != "read" && value.Op != "write" {
		return result, fmt.Errorf("unknown operation %q", value.Op)
	}
	if len(value.LBA) == 0 {
		return result, errors.New("lba is required")
	}
	result.name = value.Op
	result.lba, result.boolLBA, result.invokeRuntime, err = parseLBA(value.LBA)
	if err != nil {
		return result, fmt.Errorf("lba: %w", err)
	}
	if value.Error == nil {
		if err := requireKeys(fields, "op", "lba", "data_hex", "timing", "state"); err != nil {
			return result, err
		}
		if value.DataHex == nil {
			return result, errors.New("data_hex is required")
		}
		data, err := decodeHex(*value.DataHex)
		if err != nil || len(data) != bytesPerSector {
			return result, errors.New("successful data_hex must encode exactly eight bytes")
		}
		result.data = data
		timing, err := parseTiming(value.Timing)
		if err != nil {
			return result, fmt.Errorf("timing: %w", err)
		}
		result.timing = &timing
		state, err := parseState(value.State)
		if err != nil {
			return result, fmt.Errorf("state: %w", err)
		}
		result.state = &state
		if value.StateUnchanged != nil {
			return result, errors.New("successful operation has state_unchanged")
		}
		if value.Op == "read" && !result.invokeRuntime {
			return result, errors.New("successful read LBA must fit host int")
		}
		if value.Op == "write" && len(data) != bytesPerSector {
			return result, errors.New("successful write data length mismatch")
		}
		return result, nil
	}

	if *value.Error == "InvalidLbaError" {
		if value.StateUnchanged == nil || !*value.StateUnchanged || value.Timing != nil || len(value.State) != 0 {
			return result, errors.New("invalid-LBA operation requires state_unchanged=true and no timing/state")
		}
		if value.Op == "read" {
			if err := requireKeys(fields, "op", "lba", "error", "state_unchanged"); err != nil {
				return result, err
			}
		} else if value.DataHex != nil {
			if err := requireKeys(fields, "op", "lba", "data_hex", "error", "state_unchanged"); err != nil {
				return result, err
			}
			data, err := decodeHex(*value.DataHex)
			if err != nil {
				return result, err
			}
			result.data = data
		} else {
			if err := requireKeys(fields, "op", "lba", "error", "state_unchanged"); err != nil {
				return result, err
			}
		}
		result.err = ErrInvalidLBA
		if result.boolLBA && result.invokeRuntime {
			return result, errors.New("boolean LBA parser state is inconsistent")
		}
		if result.boolLBA || !result.invokeRuntime {
			result.invokeRuntime = false
		}
		return result, nil
	}
	if *value.Error == "InvalidSectorDataError" {
		if value.Op != "write" || value.DataHex == nil || value.StateUnchanged == nil || !*value.StateUnchanged || value.Timing != nil || len(value.State) != 0 {
			return result, errors.New("invalid-sector-data vector must be a short write with state_unchanged=true")
		}
		if err := requireKeys(fields, "op", "lba", "data_hex", "error", "state_unchanged"); err != nil {
			return result, err
		}
		data, err := decodeHex(*value.DataHex)
		if err != nil || len(data) == bytesPerSector {
			return result, errors.New("invalid-sector-data vector must have a non-eight-byte payload")
		}
		result.data = data
		result.err = ErrInvalidSectorData
		return result, nil
	}
	return result, fmt.Errorf("unknown error name %q", *value.Error)
}

func parseProbe(raw json.RawMessage) (parsedProbe, error) {
	var result parsedProbe
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fields); err != nil || fields == nil {
		return result, errors.New("probe must be an object")
	}
	var value fixtureProbe
	if err := decodeStrict(raw, &value); err != nil {
		return result, err
	}
	if value.Name == "" {
		return result, errors.New("probe name is required")
	}
	result.name = value.Name
	var err error
	if result.tick, err = exactUint64(value.StartTick); err != nil || result.tick > maxTick {
		return result, errors.New("start_tick outside portable clock range")
	}
	if result.head, err = smallInt(value.HeadTrack, 0, tracksPerSurface-1); err != nil {
		return result, errors.New("head_track outside [0,4)")
	}
	if result.targetTrack, err = smallInt(value.TargetTrack, 0, tracksPerSurface-1); err != nil {
		return result, errors.New("target_track outside [0,4)")
	}
	if result.targetSector, err = smallInt(value.TargetSector, 0, sectorsPerTrack-1); err != nil {
		return result, errors.New("target_sector outside [0,4)")
	}
	if value.Error != nil {
		if *value.Error != "ClockOverflowError" || value.Timing != nil {
			return result, fmt.Errorf("unknown or malformed timing-probe error %q", *value.Error)
		}
		if err := requireKeys(fields, "name", "start_tick", "head_track", "target_track", "target_sector", "error"); err != nil {
			return result, err
		}
		result.err = ErrClockOverflow
		return result, nil
	}
	if len(value.Timing) == 0 {
		return result, errors.New("successful probe requires timing")
	}
	if err := requireKeys(fields, "name", "start_tick", "head_track", "target_track", "target_sector", "timing"); err != nil {
		return result, err
	}
	timing, err := parseTiming(value.Timing)
	if err != nil {
		return result, err
	}
	result.timing = &timing
	return result, nil
}

func parseState(raw json.RawMessage) (fixtureStateValue, error) {
	var result fixtureStateValue
	if err := exactObjectKeys(raw, "tick", "head_track"); err != nil {
		return result, err
	}
	var value fixtureState
	if err := decodeStrict(raw, &value); err != nil {
		return result, err
	}
	tick, err := exactUint64(value.Tick)
	if err != nil || tick > maxTick {
		return result, errors.New("tick outside portable clock range")
	}
	head, err := smallInt(value.HeadTrack, 0, tracksPerSurface-1)
	if err != nil {
		return result, errors.New("head_track outside [0,4)")
	}
	return fixtureStateValue{tick: tick, head: head}, nil
}

func parseTiming(raw json.RawMessage) (IOTiming, error) {
	var result IOTiming
	if err := exactObjectKeys(raw, "start_tick", "seek_ticks", "rotation_ticks", "transfer_ticks", "total_ticks", "end_tick"); err != nil {
		return result, err
	}
	var value fixtureTiming
	if err := decodeStrict(raw, &value); err != nil {
		return result, err
	}
	fields := []struct {
		name string
		raw  json.RawMessage
		out  *uint64
	}{
		{"start_tick", value.StartTick, &result.StartTick},
		{"seek_ticks", value.SeekTicks, &result.SeekTicks},
		{"rotation_ticks", value.RotationTicks, &result.RotationTicks},
		{"transfer_ticks", value.TransferTicks, &result.TransferTicks},
		{"total_ticks", value.TotalTicks, &result.TotalTicks},
		{"end_tick", value.EndTick, &result.EndTick},
	}
	for _, field := range fields {
		n, err := exactUint64(field.raw)
		if err != nil {
			return result, fmt.Errorf("%s must be an exact nonnegative integer", field.name)
		}
		*field.out = n
	}
	if result.StartTick > maxTick || result.EndTick > maxTick ||
		result.SeekTicks > 9 || result.RotationTicks > 3 || result.TransferTicks != 1 ||
		result.SeekTicks+result.RotationTicks+result.TransferTicks != result.TotalTicks ||
		result.TotalTicks > maxTick-result.StartTick || result.StartTick+result.TotalTicks != result.EndTick {
		return result, errors.New("timing fields violate component or portable-clock invariants")
	}
	return result, nil
}

func parseLBA(raw json.RawMessage) (lba int, boolLBA, invokeRuntime bool, err error) {
	trimmed := bytes.TrimSpace(raw)
	if bytes.Equal(trimmed, []byte("true")) || bytes.Equal(trimmed, []byte("false")) {
		return 0, true, false, nil
	}
	if !integerSpelling(trimmed) {
		return 0, false, false, errors.New("LBA must be an exact JSON integer or boolean")
	}
	n, parseErr := strconv.ParseInt(string(trimmed), 10, 64)
	if parseErr != nil {
		var numberErr *strconv.NumError
		if errors.As(parseErr, &numberErr) && errors.Is(numberErr.Err, strconv.ErrRange) {
			return 0, false, false, nil
		}
		return 0, false, false, errors.New("LBA must be an exact JSON integer or boolean")
	}
	maxInt := int64(^uint(0) >> 1)
	minInt := -maxInt - 1
	if n < minInt || n > maxInt {
		return 0, false, false, nil
	}
	return int(n), false, true, nil
}

func exactInt64(raw json.RawMessage) (int64, error) {
	trimmed := bytes.TrimSpace(raw)
	if !integerSpelling(trimmed) {
		return 0, errors.New("not an exact int64 JSON integer")
	}
	return strconv.ParseInt(string(trimmed), 10, 64)
}

func exactUint64(raw json.RawMessage) (uint64, error) {
	trimmed := bytes.TrimSpace(raw)
	if !integerSpelling(trimmed) || len(trimmed) > 0 && trimmed[0] == '-' {
		return 0, errors.New("not an exact uint64 JSON integer")
	}
	return strconv.ParseUint(string(trimmed), 10, 64)
}

func integerSpelling(raw []byte) bool {
	if len(raw) == 0 {
		return false
	}
	start := 0
	if raw[0] == '-' {
		start = 1
	}
	if start == len(raw) {
		return false
	}
	for _, b := range raw[start:] {
		if b < '0' || b > '9' {
			return false
		}
	}
	return true
}

func smallInt(raw json.RawMessage, min, max int) (int, error) {
	n, err := exactInt64(raw)
	if err != nil || n < int64(min) || n > int64(max) {
		return 0, errors.New("integer out of range")
	}
	return int(n), nil
}

func decodeHex(value string) ([]byte, error) {
	if len(value)%2 != 0 || strings.ToLower(value) != value {
		return nil, errors.New("hex bytes must be even-length lowercase text")
	}
	result, err := hex.DecodeString(value)
	if err != nil {
		return nil, fmt.Errorf("malformed hex bytes: %w", err)
	}
	return result, nil
}

func exactObject(raw []byte, keys ...string) (map[string]json.RawMessage, error) {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fields); err != nil || fields == nil {
		return nil, errors.New("expected object")
	}
	if err := requireKeys(fields, keys...); err != nil {
		return nil, err
	}
	return fields, nil
}

func exactObjectKeys(raw []byte, keys ...string) error {
	_, err := exactObject(raw, keys...)
	return err
}

func requireKeys(fields map[string]json.RawMessage, keys ...string) error {
	if len(fields) != len(keys) {
		return fmt.Errorf("expected exactly keys %v, got %d", keys, len(fields))
	}
	for _, key := range keys {
		if _, ok := fields[key]; !ok {
			return fmt.Errorf("missing required key %q", key)
		}
	}
	return nil
}

func decodeStrict(raw []byte, out any) error {
	return json.Unmarshal(raw, out)
}

func rejectDuplicateKeysAndTrailingJSON(raw []byte) error {
	if !utf8.Valid(raw) {
		return errors.New("fixture is not valid UTF-8")
	}
	if err := validateSurrogateEscapes(raw); err != nil {
		return err
	}
	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.UseNumber()
	if err := walkJSONValue(decoder, 0); err != nil {
		return err
	}
	if _, err := decoder.Token(); err != io.EOF {
		if err == nil {
			return errors.New("trailing JSON value")
		}
		return err
	}
	return nil
}

// encoding/json replaces unpaired UTF-16 surrogates with U+FFFD. Reject those
// escaped code units before decoding so fixture strings retain their source
// Unicode scalar values exactly.
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

func walkJSONValue(decoder *json.Decoder, depth int) error {
	if depth > maxJSONDepth {
		return errors.New("JSON nesting limit exceeded")
	}
	token, err := decoder.Token()
	if err != nil {
		return err
	}
	delimiter, isDelimiter := token.(json.Delim)
	if !isDelimiter {
		return nil
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
			if _, exists := seen[key]; exists {
				return fmt.Errorf("duplicate object key %q", key)
			}
			seen[key] = struct{}{}
			if err := walkJSONValue(decoder, depth+1); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil || end != json.Delim('}') {
			return errors.New("unterminated JSON object")
		}
	case '[':
		for decoder.More() {
			if err := walkJSONValue(decoder, depth+1); err != nil {
				return err
			}
		}
		end, err := decoder.Token()
		if err != nil || end != json.Delim(']') {
			return errors.New("unterminated JSON array")
		}
	default:
		return errors.New("unexpected JSON delimiter")
	}
	return nil
}

func replayFixture(t *testing.T, fixture parsedFixture) {
	t.Helper()
	for _, testCase := range fixture.cases {
		d := New()
		for operationIndex, operation := range testCase.ops {
			before, err := d.Snapshot()
			if err != nil {
				t.Fatalf("case %s operation %d snapshot: %v", testCase.name, operationIndex, err)
			}
			if operation.err != nil {
				var actual error
				if operation.boolLBA || !operation.invokeRuntime {
					actual = ErrInvalidLBA
				} else if operation.name == "read" {
					result, callErr := d.ReadSector(operation.lba)
					actual = callErr
					if result.Data != nil || result.Timing != (IOTiming{}) {
						t.Fatalf("case %s operation %d returned data with refusal: %#v", testCase.name, operationIndex, result)
					}
				} else {
					result, callErr := d.WriteSector(operation.lba, operation.data)
					actual = callErr
					if result != (IOTiming{}) {
						t.Fatalf("case %s operation %d returned timing with refusal: %#v", testCase.name, operationIndex, result)
					}
				}
				if !errors.Is(actual, operation.err) {
					t.Fatalf("case %s operation %d error = %v, want %v", testCase.name, operationIndex, actual, operation.err)
				}
				after, err := d.Snapshot()
				if err != nil || after != before {
					t.Fatalf("case %s operation %d refusal changed state: before=%+v after=%+v err=%v", testCase.name, operationIndex, before, after, err)
				}
				continue
			}
			var actualTiming IOTiming
			if operation.name == "read" {
				result, err := d.ReadSector(operation.lba)
				if err != nil {
					t.Fatalf("case %s operation %d read: %v", testCase.name, operationIndex, err)
				}
				if !bytes.Equal(result.Data, operation.data) {
					t.Fatalf("case %s operation %d bytes = %x, want %x", testCase.name, operationIndex, result.Data, operation.data)
				}
				actualTiming = result.Timing
			} else {
				var err error
				actualTiming, err = d.WriteSector(operation.lba, operation.data)
				if err != nil {
					t.Fatalf("case %s operation %d write: %v", testCase.name, operationIndex, err)
				}
			}
			if actualTiming != *operation.timing {
				t.Fatalf("case %s operation %d timing = %+v, want %+v", testCase.name, operationIndex, actualTiming, *operation.timing)
			}
			after, err := d.Snapshot()
			if err != nil {
				t.Fatal(err)
			}
			expected := before
			expected.Tick = operation.state.tick
			expected.HeadTrack = operation.state.head
			if operation.name == "write" {
				copy(expected.Sectors[operation.lba][:], operation.data)
			}
			if after != expected {
				t.Fatalf("case %s operation %d snapshot mismatch", testCase.name, operationIndex)
			}
		}
	}
	for _, probe := range fixture.probes {
		d := New()
		d.tick, d.headTrack = probe.tick, probe.head
		lba := probe.targetTrack*sectorsPerCylinder + probe.targetSector
		result, err := d.ReadSector(lba)
		if probe.err != nil {
			if !errors.Is(err, probe.err) || result.Data != nil || result.Timing != (IOTiming{}) {
				t.Fatalf("probe %s result=%+v error=%v, want clock overflow and zero result", probe.name, result, err)
			}
			if d.tick != probe.tick || d.headTrack != probe.head {
				t.Fatalf("overflow probe %s mutated device", probe.name)
			}
			continue
		}
		if err != nil || result.Timing != *probe.timing {
			t.Fatalf("probe %s timing=%+v error=%v, want %+v", probe.name, result.Timing, err, *probe.timing)
		}
	}
}

// validateFixtureModel checks every declared outcome using the independently
// stepped clock and per-sector model before the fixture may reach the runtime.
func validateFixtureModel(fixture parsedFixture) error {
	for _, testCase := range fixture.cases {
		state := model{}
		for operationIndex, operation := range testCase.ops {
			before := state
			var wantTiming IOTiming
			var wantData []byte
			var wantErr error
			if operation.boolLBA || !operation.invokeRuntime {
				wantErr = ErrInvalidLBA
			} else {
				op := byte('r')
				if operation.name == "write" {
					op = 'w'
				}
				wantTiming, wantData, wantErr = modelStep(&state, op, operation.lba, operation.data)
			}
			if !sameFixtureError(operation.err, wantErr) {
				return fmt.Errorf("case %s operation %d declared error %v, model gives %v", testCase.name, operationIndex, operation.err, wantErr)
			}
			if wantErr != nil {
				if state != before {
					return fmt.Errorf("case %s operation %d model refusal changed state", testCase.name, operationIndex)
				}
				continue
			}
			if operation.timing == nil || *operation.timing != wantTiming {
				return fmt.Errorf("case %s operation %d timing differs from model", testCase.name, operationIndex)
			}
			if operation.name == "read" && !bytes.Equal(operation.data, wantData) {
				return fmt.Errorf("case %s operation %d read bytes differ from model", testCase.name, operationIndex)
			}
			if operation.state == nil || operation.state.tick != state.tick || operation.state.head != state.head {
				return fmt.Errorf("case %s operation %d resulting clock/head differs from model", testCase.name, operationIndex)
			}
		}
	}
	for _, probe := range fixture.probes {
		lba := probe.targetTrack*sectorsPerCylinder + probe.targetSector
		wantTiming, wantErr := modelTiming(model{tick: probe.tick, head: probe.head}, lba)
		if !sameFixtureError(probe.err, wantErr) {
			return fmt.Errorf("timing probe %s declared error %v, model gives %v", probe.name, probe.err, wantErr)
		}
		if wantErr == nil && (probe.timing == nil || *probe.timing != wantTiming) {
			return fmt.Errorf("timing probe %s differs from model", probe.name)
		}
	}
	return nil
}

func sameFixtureError(got, want error) bool {
	if got == nil || want == nil {
		return got == nil && want == nil
	}
	return errors.Is(got, want) && errors.Is(want, got)
}

func TestSharedFixture(t *testing.T) {
	fixture, err := parseFixture(readSharedFixture(t))
	if err != nil {
		t.Fatal(err)
	}
	if err := validateFixtureModel(fixture); err != nil {
		t.Fatalf("fixture declarations fail independent model: %v", err)
	}
	replayFixture(t, fixture)
}

func TestFixtureModelAdmissionRejectsEditedExpectations(t *testing.T) {
	canonical := readSharedFixture(t)
	assertRejected := func(name string, raw []byte) {
		t.Helper()
		t.Run(name, func(t *testing.T) {
			fixture, err := parseFixture(raw)
			if err != nil {
				t.Fatalf("variant should remain structurally valid: %v", err)
			}
			if err := validateFixtureModel(fixture); err == nil {
				t.Fatal("inconsistent declaration passed model admission")
			}
		})
	}
	mutate := func(name, old, next string) []byte {
		t.Helper()
		changed := bytes.Replace(canonical, []byte(old), []byte(next), 1)
		if bytes.Equal(changed, canonical) {
			t.Fatalf("%s mutation anchor was not found", name)
		}
		return changed
	}
	assertRejected("isolated LBA change", mutate("isolated LBA", `"lba": 11`, `"lba": 1`))
	assertRejected("read bytes change", mutate("read bytes", `"data_hex": "0000000000000000"`, `"data_hex": "ffffffffffffffff"`))
	changedTiming := bytes.Replace(canonical, []byte(`"seek_ticks": 3,
            "rotation_ticks": 0,`), []byte(`"seek_ticks": 2,
            "rotation_ticks": 1,`), 1)
	if bytes.Equal(changedTiming, canonical) {
		t.Fatal("timing mutation anchors were not found")
	}
	assertRejected("internally consistent wrong timing", changedTiming)
	assertRejected("error change", mutate("error", `"error": "InvalidSectorDataError"`, `"error": "InvalidLbaError"`))
	assertRejected("maximum-edge probe target change", mutate("maximum-edge probe", `"target_sector": 2,`+"\n      "+`"timing": {`, `"target_sector": 1,`+"\n      "+`"timing": {`))
}

func TestFixtureModelAdmissionAcceptsConsistentEditAndSavedFuzzInput(t *testing.T) {
	saved, err := os.ReadFile("testdata/db-e119-f20.json")
	if err != nil {
		t.Fatal(err)
	}
	savedFixture, err := parseFixture(saved)
	if err != nil {
		t.Fatalf("saved fuzz input is structurally invalid: %v", err)
	}
	if err := validateFixtureModel(savedFixture); err == nil {
		t.Fatal("saved f20 input should be rejected by the stepped model")
	}

	canonical := readSharedFixture(t)
	var document map[string]any
	if err := json.Unmarshal(canonical, &document); err != nil {
		t.Fatal(err)
	}
	cases := document["cases"].([]any)
	second := cases[1].(map[string]any)
	operation := second["operations"].([]any)[0].(map[string]any)
	operation["lba"] = 1
	var modelState model
	timing, _, err := modelStep(&modelState, 'r', 1, nil)
	if err != nil {
		t.Fatal(err)
	}
	declaredTiming := operation["timing"].(map[string]any)
	declaredTiming["start_tick"] = timing.StartTick
	declaredTiming["seek_ticks"] = timing.SeekTicks
	declaredTiming["rotation_ticks"] = timing.RotationTicks
	declaredTiming["transfer_ticks"] = timing.TransferTicks
	declaredTiming["total_ticks"] = timing.TotalTicks
	declaredTiming["end_tick"] = timing.EndTick
	declaredState := operation["state"].(map[string]any)
	declaredState["tick"] = modelState.tick
	declaredState["head_track"] = modelState.head
	consistent, err := json.Marshal(document)
	if err != nil {
		t.Fatal(err)
	}
	fixture, err := parseFixture(consistent)
	if err != nil {
		t.Fatalf("consistent edit parse: %v", err)
	}
	if err := validateFixtureModel(fixture); err != nil {
		t.Fatalf("consistent edit rejected by model: %v", err)
	}
	replayFixture(t, fixture)
}

func TestFixtureReaderRejectsMalformedDocuments(t *testing.T) {
	const expectedDepthLimit = 64
	valid := readSharedFixture(t)
	nestedScalar := func(depth int) []byte {
		return append(append(bytes.Repeat([]byte{'['}, depth), '0'), bytes.Repeat([]byte{']'}, depth)...)
	}
	if err := rejectDuplicateKeysAndTrailingJSON(nestedScalar(expectedDepthLimit)); err != nil {
		t.Fatalf("depth-%d nested scalar refused: %v", expectedDepthLimit, err)
	}
	if err := rejectDuplicateKeysAndTrailingJSON(nestedScalar(expectedDepthLimit + 1)); err == nil {
		t.Fatalf("depth-%d nested scalar was accepted", expectedDepthLimit+1)
	}
	deep := append(bytes.Repeat([]byte{'['}, expectedDepthLimit+2), bytes.Repeat([]byte{']'}, expectedDepthLimit+2)...)
	for name, input := range map[string][]byte{
		"truncated":        []byte(`{"schema":"`),
		"trailing":         append(append([]byte(nil), valid...), []byte(` {}`)...),
		"duplicate-root":   []byte(`{"schema":"a","schema":"b"}`),
		"wrong-root-type":  []byte(`null`),
		"wrong-schema":     []byte(`{"schema":"wrong","profile":{},"initial_state":{},"cases":[],"timing_probes":[]}`),
		"oversized":        bytes.Repeat([]byte{' '}, maxFixtureBytes+1),
		"empty-case-array": []byte(`{"schema":"coding-adventures.database-media.hdd-simulator.v1","profile":{},"initial_state":{},"cases":[],"timing_probes":[]}`),
		"invalid-utf8":     []byte{'{', '"', 'x', '"', ':', '"', 0xff, '"', '}'},
		"deep-nesting":     deep,
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := parseFixture(input); err == nil {
				t.Fatal("malformed fixture accepted")
			}
		})
	}
}

func TestFixtureReaderUnicodeScalarBoundary(t *testing.T) {
	raw := readSharedFixture(t)
	controls := []struct {
		name  string
		value []byte
		valid bool
	}{
		{"ordinary Unicode name", []byte(`"name": "é`), true},
		{"valid surrogate pair", []byte(`"name": "\ud83d\ude00`), true},
		{"unpaired high surrogate", []byte(`"name": "\ud800`), false},
		{"unpaired low surrogate", []byte(`"name": "\udfff`), false},
		{"mismatched pair", []byte(`"name": "\ud83d\u0041`), false},
		{"escaped backslash", []byte(`"name": "\\ud800`), true},
	}
	for _, control := range controls {
		t.Run(control.name, func(t *testing.T) {
			variant := bytes.Replace(raw, []byte(`"name": "`), control.value, 1)
			if bytes.Equal(variant, raw) {
				t.Fatal("fixture name mutation did not change input")
			}
			fixture, err := parseFixture(variant)
			if (err == nil) != control.valid {
				t.Fatalf("parseFixture acceptance = %v, want %v (err=%v)", err == nil, control.valid, err)
			}
			if err == nil {
				replayFixture(t, fixture)
			}
		})
	}
	duplicateName := bytes.Replace(raw, []byte(`"name": "seek-and-rotation-advance-together"`), []byte(`"name": "read-initial-lba-zer\u006f"`), 1)
	if bytes.Equal(duplicateName, raw) {
		t.Fatal("decoded duplicate-name mutation anchor not found")
	}
	if _, err := parseFixture(duplicateName); err == nil {
		t.Fatal("decoded duplicate case name was accepted")
	}
}

func FuzzFixtureReader(f *testing.F) {
	data, err := os.ReadFile(fixturePath)
	if err != nil {
		f.Fatalf("read canonical fixture seed: %v", err)
	}
	f.Add(data)
	saved, err := os.ReadFile("testdata/db-e119-f20.json")
	if err != nil {
		f.Fatalf("read saved f20 fixture seed: %v", err)
	}
	f.Add(saved)
	f.Add([]byte(`{"schema":"x","schema":"y"}`))
	f.Add([]byte(`{}`))
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > maxFixtureBytes {
			return
		}
		fixture, err := parseFixture(input)
		if err == nil && validateFixtureModel(fixture) == nil {
			replayFixture(t, fixture)
		}
	})
}
