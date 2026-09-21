package dertlv

import (
	"encoding/hex"
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func loadFixture(t *testing.T) map[string]any {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", "der-tlv-v1", "cases.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var document map[string]any
	if err := json.Unmarshal(data, &document); err != nil {
		t.Fatal(err)
	}
	return document
}

func inputBytes(t *testing.T, raw any) []byte {
	t.Helper()
	var output []byte
	for _, item := range raw.([]any) {
		segment := item.(map[string]any)
		if text, ok := segment["hex"].(string); ok {
			decoded, err := hex.DecodeString(text)
			if err != nil {
				t.Fatal(err)
			}
			output = append(output, decoded...)
		} else {
			decoded, _ := hex.DecodeString(segment["repeat_hex"].(string))
			for range int(segment["count"].(float64)) {
				output = append(output, decoded[0])
			}
		}
	}
	return output
}

func fixtureLimits(document, testCase map[string]any) Limits {
	values := map[string]any{}
	for key, value := range document["defaults"].(map[string]any) {
		values[key] = value
	}
	if raw, ok := testCase["limits"].(map[string]any); ok {
		for key, value := range raw {
			values[key] = value
		}
	}
	var maxValue uint64
	if values["max_value_len"] == "host-max" {
		maxValue = math.MaxUint64
	} else {
		maxValue = uint64(values["max_value_len"].(float64))
	}
	return Limits{
		MaxInputLen:  int(values["max_input_len"].(float64)),
		MaxValueLen:  maxValue,
		MaxElements:  int(values["max_elements"].(float64)),
		MaxTagNumber: uint32(values["max_tag_number"].(float64)),
	}
}

func elementResult(element Element, offset int) map[string]any {
	return map[string]any{
		"outcome": "element", "element_offset": float64(offset),
		"tag":        map[string]any{"class": string(element.Tag.Class), "constructed": element.Tag.Constructed, "number": float64(element.Tag.Number)},
		"header_len": float64(len(element.Header())), "encoded_len": float64(len(element.Encoded())),
		"remainder_offset": float64(offset + len(element.Encoded())),
	}
}

func errorResult(err error) map[string]any {
	derError := err.(*Error)
	return map[string]any{"outcome": "error", "error_id": string(derError.Kind), "offset": float64(derError.Offset)}
}

func runDecode(testCase map[string]any, input []byte, limits Limits) map[string]any {
	if testCase["operation"] == "decode-one" {
		element, remainder, err := DecodeOne(input, limits)
		if err != nil {
			return errorResult(err)
		}
		result := elementResult(element, 0)
		if int(result["remainder_offset"].(float64)) != len(input)-len(remainder) {
			panic("remainder mismatch")
		}
		return result
	}
	element, err := DecodeExact(input, limits)
	if err != nil {
		return errorResult(err)
	}
	return elementResult(element, 0)
}

func runCursor(testCase map[string]any, input []byte, limits Limits) map[string]any {
	cursor, err := NewCursor(input, limits)
	if err != nil {
		panic(err)
	}
	events := []any{}
	for _, raw := range testCase["actions"].([]any) {
		if raw == "finish" {
			if err := cursor.Finish(); err != nil {
				events = append(events, errorResult(err))
			} else {
				events = append(events, map[string]any{"outcome": "finished"})
			}
			continue
		}
		offset := len(input) - len(cursor.Remaining())
		element, err := cursor.Read()
		if err != nil {
			events = append(events, errorResult(err))
		} else if element == nil {
			events = append(events, map[string]any{"outcome": "end"})
		} else {
			events = append(events, elementResult(*element, offset))
		}
	}
	return map[string]any{"events": events, "elements_read": float64(cursor.ElementsRead()), "remaining_offset": float64(len(input) - len(cursor.Remaining()))}
}

func TestPortableConformance(t *testing.T) {
	document := loadFixture(t)
	cases := document["cases"].([]any)
	if len(cases) != 54 {
		t.Fatalf("got %d cases", len(cases))
	}
	for _, raw := range cases {
		testCase := raw.(map[string]any)
		t.Run(testCase["id"].(string), func(t *testing.T) {
			input := inputBytes(t, testCase["input"])
			limits := fixtureLimits(document, testCase)
			var actual map[string]any
			if testCase["operation"] == "cursor" {
				actual = runCursor(testCase, input, limits)
			} else {
				actual = runDecode(testCase, input, limits)
			}
			if !reflect.DeepEqual(actual, testCase["expected"]) {
				t.Fatalf("actual %#v expected %#v", actual, testCase["expected"])
			}
			if hostile, ok := testCase["redacted_input_hex"].(string); ok {
				_, err := DecodeExact(input, limits)
				if err == nil || strings.Contains(err.Error(), hostile) {
					t.Fatalf("payload leaked: %v", err)
				}
			}
		})
	}
}

func TestElementSlicesShareCallerBuffer(t *testing.T) {
	input := []byte{0x04, 0x01, 0x2a}
	element, err := DecodeExact(input, DefaultLimits())
	if err != nil {
		t.Fatal(err)
	}
	input[2] = 0x7f
	if element.Value()[0] != 0x7f {
		t.Fatal("element copied input")
	}
}
