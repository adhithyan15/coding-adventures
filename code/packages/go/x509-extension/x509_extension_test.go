package x509extension

import (
	"encoding/hex"
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"strings"
	"testing"

	derasn1 "github.com/adhithyan15/coding-adventures/code/packages/go/der-asn1"
	dertlv "github.com/adhithyan15/coding-adventures/code/packages/go/der-tlv"
)

func fixture(t *testing.T, name string) map[string]any {
	t.Helper()
	path := filepath.Join("..", "..", "..", "specs", "fixtures", name, "cases.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var result map[string]any
	if err := json.Unmarshal(data, &result); err != nil {
		t.Fatal(err)
	}
	return result
}

func object(value any) map[string]any { return value.(map[string]any) }
func array(value any) []any           { return value.([]any) }
func text(value any) string           { return value.(string) }
func integer(value any) int           { return int(value.(float64)) }

func materialize(segments any) []byte {
	var result []byte
	for _, raw := range array(segments) {
		segment := object(raw)
		if value, ok := segment["hex"]; ok {
			decoded, err := hex.DecodeString(text(value))
			if err != nil {
				panic(err)
			}
			result = append(result, decoded...)
		} else {
			decoded, err := hex.DecodeString(text(segment["repeat_hex"]))
			if err != nil || len(decoded) != 1 {
				panic("invalid repeat fixture")
			}
			for range integer(segment["count"]) {
				result = append(result, decoded[0])
			}
		}
	}
	return result
}

func merged(defaults, overrides map[string]any, name string) any {
	if value, ok := overrides[name]; ok {
		return value
	}
	return defaults[name]
}

func limits(upstream, testCase map[string]any) derasn1.ASN1Limits {
	defaults := object(upstream["defaults"])
	overrides := map[string]any{}
	if value, ok := testCase["limits"]; ok {
		overrides = object(value)
	}
	derOverrides := map[string]any{}
	if value, ok := overrides["der"]; ok {
		derOverrides = object(value)
	}
	derDefaults := object(defaults["der"])
	maxValue := merged(derDefaults, derOverrides, "max_value_len")
	maxValueLen := uint64(math.MaxUint64)
	if number, ok := maxValue.(float64); ok {
		maxValueLen = uint64(number)
	}
	return derasn1.ASN1Limits{
		DER: dertlv.Limits{
			MaxInputLen:  integer(merged(derDefaults, derOverrides, "max_input_len")),
			MaxValueLen:  maxValueLen,
			MaxElements:  integer(merged(derDefaults, derOverrides, "max_elements")),
			MaxTagNumber: uint32(merged(derDefaults, derOverrides, "max_tag_number").(float64)),
		},
		MaxDepth:         integer(merged(defaults, overrides, "max_depth")),
		MaxTotalElements: integer(merged(defaults, overrides, "max_total_elements")),
		MaxOIDArcs:       integer(merged(defaults, overrides, "max_oid_arcs")),
	}
}

func attempt(decoder *derasn1.ASN1Decoder, root derasn1.ASN1Element) map[string]any {
	value, err := DecodeX509Extension(decoder, root)
	if err != nil {
		x509Err := err.(*Error)
		result := map[string]any{
			"outcome": "error", "error_id": string(x509Err.Kind),
			"offset": x509Err.Offset, "offset_scope": "extension-element",
			"elements_read": decoder.ElementsRead(),
		}
		if x509Err.ASN1Kind != nil {
			result["asn1_error_id"] = string(*x509Err.ASN1Kind)
		}
		if x509Err.FramingKind != nil {
			result["framing_error_id"] = string(*x509Err.FramingKind)
		}
		return result
	}
	arcs := make([]string, 0, len(value.ExtensionID().Arcs()))
	for _, arc := range value.ExtensionID().Arcs() {
		arcs = append(arcs, strconv.FormatUint(arc, 10))
	}
	return map[string]any{
		"outcome": "value", "extension_id_arcs_decimal": arcs,
		"critical":            value.Critical(),
		"extension_value_hex": hex.EncodeToString(value.ExtensionValue()),
		"elements_read":       decoder.ElementsRead(),
	}
}

func normalize(value any) any {
	encoded, err := json.Marshal(value)
	if err != nil {
		panic(err)
	}
	var result any
	if err := json.Unmarshal(encoded, &result); err != nil {
		panic(err)
	}
	return result
}

func TestPortableConformance(t *testing.T) {
	document := fixture(t, "x509-extension-v1")
	upstream := fixture(t, "der-asn1-v1")
	cases := array(document["cases"])
	if len(cases) != 48 {
		t.Fatalf("got %d cases", len(cases))
	}
	for _, raw := range cases {
		testCase := object(raw)
		t.Run(text(testCase["id"]), func(t *testing.T) {
			decoder := derasn1.NewDecoder(limits(upstream, testCase))
			root, err := decoder.DecodeExact(materialize(testCase["input"]))
			if err != nil {
				t.Fatal(err)
			}
			var actual map[string]any
			if text(testCase["operation"]) == "extension-script" {
				events := make([]any, 0, len(array(testCase["actions"])))
				for range array(testCase["actions"]) {
					events = append(events, attempt(decoder, root))
				}
				actual = map[string]any{"outcome": "script", "events": events}
			} else {
				actual = attempt(decoder, root)
			}
			if !reflect.DeepEqual(normalize(actual), testCase["expected"]) {
				t.Fatalf("got %#v want %#v", normalize(actual), testCase["expected"])
			}
			if hostile, ok := testCase["redacted_input_hex"]; ok {
				encoded, _ := json.Marshal(actual)
				if strings.Contains(string(encoded), text(hostile)) {
					t.Fatal("hostile payload leaked")
				}
			}
		})
	}
}
