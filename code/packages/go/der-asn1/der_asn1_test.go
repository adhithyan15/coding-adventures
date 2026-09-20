package derasn1

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"strings"
	"testing"

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

func mergedValue(defaults map[string]any, overrides map[string]any, name string) any {
	if value, ok := overrides[name]; ok {
		return value
	}
	return defaults[name]
}

func fixtureDERLimits(defaults map[string]any, overrides map[string]any) dertlv.Limits {
	maxValue := mergedValue(defaults, overrides, "max_value_len")
	var maxValueLen uint64
	if stringValue, ok := maxValue.(string); ok && stringValue == "host-max" {
		maxValueLen = math.MaxUint64
	} else {
		maxValueLen = uint64(maxValue.(float64))
	}
	return dertlv.Limits{
		MaxInputLen:  integer(mergedValue(defaults, overrides, "max_input_len")),
		MaxValueLen:  maxValueLen,
		MaxElements:  integer(mergedValue(defaults, overrides, "max_elements")),
		MaxTagNumber: uint32(mergedValue(defaults, overrides, "max_tag_number").(float64)),
	}
}

func fixtureLimits(document, testCase map[string]any) ASN1Limits {
	defaults := object(document["defaults"])
	overrides := map[string]any{}
	if value, ok := testCase["limits"]; ok {
		overrides = object(value)
	}
	derOverrides := map[string]any{}
	if value, ok := overrides["der"]; ok {
		derOverrides = object(value)
	}
	return ASN1Limits{
		DER:              fixtureDERLimits(object(defaults["der"]), derOverrides),
		MaxDepth:         integer(mergedValue(defaults, overrides, "max_depth")),
		MaxTotalElements: integer(mergedValue(defaults, overrides, "max_total_elements")),
		MaxOIDArcs:       integer(mergedValue(defaults, overrides, "max_oid_arcs")),
	}
}

func tagProjection(element ASN1Element) map[string]any {
	tag := element.Tag()
	return map[string]any{
		"class": string(tag.Class), "constructed": tag.Constructed,
		"number": tag.Number,
	}
}

func errorProjection(err error, scope string) map[string]any {
	value := err.(*Error)
	result := map[string]any{
		"outcome": "error", "error_id": string(value.Kind),
		"offset": value.Offset, "offset_scope": scope,
	}
	if value.FramingKind != nil {
		result["framing_error_id"] = string(*value.FramingKind)
	}
	return result
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

func verifyUpstream(upstream, testCase map[string]any) map[string]any {
	id := text(testCase["der_tlv_case_id"])
	var referenced map[string]any
	for _, raw := range array(upstream["cases"]) {
		candidate := object(raw)
		if text(candidate["id"]) == id {
			referenced = candidate
			break
		}
	}
	if referenced == nil {
		panic("missing upstream case")
	}
	overrides := map[string]any{}
	if value, ok := referenced["limits"]; ok {
		overrides = object(value)
	}
	limits := DefaultLimits()
	limits.DER = fixtureDERLimits(object(upstream["defaults"]), overrides)
	decoder := NewDecoder(limits)
	element, err := decoder.DecodeExact(materialize(referenced["input"]))
	expected := object(referenced["expected"])
	if err != nil {
		projected := errorProjection(err, "operation-input")
		if expected["outcome"] != "error" || projected["framing_error_id"] != expected["error_id"] || normalize(projected["offset"]) != expected["offset"] {
			panic(fmt.Sprintf("upstream mismatch %s", id))
		}
	} else {
		if expected["outcome"] != "element" || !reflect.DeepEqual(normalize(tagProjection(element)), expected["tag"]) || normalize(len(element.Header())) != expected["header_len"] || normalize(len(element.Encoded())) != expected["encoded_len"] {
			panic(fmt.Sprintf("upstream mismatch %s", id))
		}
	}
	return map[string]any{"outcome": "upstream"}
}

func primitiveResult(operation string, element ASN1Element, limits ASN1Limits, tagNumber uint32) (map[string]any, error) {
	switch operation {
	case "decode-boolean":
		value, err := DecodeBoolean(element)
		return map[string]any{"outcome": "value", "boolean": value, "elements_read": 1}, err
	case "decode-integer", "integer-to-u64":
		value, err := DecodeInteger(element)
		if err != nil {
			return nil, err
		}
		result := map[string]any{"outcome": "value", "signed_hex": hex.EncodeToString(value.SignedBytes()), "negative": value.IsNegative()}
		if operation == "integer-to-u64" {
			unsigned, conversionErr := value.ToUint64()
			if conversionErr != nil {
				return nil, conversionErr
			}
			result["u64_decimal"] = strconv.FormatUint(unsigned, 10)
		}
		return result, nil
	case "decode-bit-string":
		value, err := DecodeBitString(element)
		return map[string]any{"outcome": "value", "bytes_hex": hex.EncodeToString(value.Bytes()), "unused_bits": value.UnusedBits(), "bit_length": value.BitLength()}, err
	case "decode-octet-string":
		value, err := DecodeOctetString(element)
		return map[string]any{"outcome": "value", "bytes_hex": hex.EncodeToString(value)}, err
	case "decode-implicit-octet-string":
		value, err := DecodeImplicitOctetString(element, tagNumber)
		return map[string]any{"outcome": "value", "bytes_hex": hex.EncodeToString(value)}, err
	case "decode-ia5-string":
		value, err := DecodeIA5String(element)
		return map[string]any{"outcome": "value", "text": value}, err
	case "decode-implicit-ia5-string":
		value, err := DecodeImplicitIA5String(element, tagNumber)
		return map[string]any{"outcome": "value", "text": value}, err
	case "decode-null":
		return map[string]any{"outcome": "value"}, DecodeNull(element)
	case "decode-object-identifier", "decode-implicit-object-identifier":
		var value ObjectIdentifier
		var err error
		if operation == "decode-object-identifier" {
			value, err = DecodeObjectIdentifier(element, limits)
		} else {
			value, err = DecodeImplicitObjectIdentifier(element, tagNumber, limits)
		}
		if err != nil {
			return nil, err
		}
		arcs := make([]string, value.ArcCount())
		for index, arc := range value.Arcs() {
			arcs[index] = strconv.FormatUint(arc, 10)
		}
		return map[string]any{"outcome": "value", "bytes_hex": hex.EncodeToString(value.Encoded()), "arcs_decimal": arcs, "arc_count": len(arcs)}, nil
	default:
		panic("unsupported primitive " + operation)
	}
}

func cursorResult(testCase map[string]any, decoder *ASN1Decoder, root ASN1Element) (map[string]any, error) {
	cursor, err := decoder.Sequence(root)
	if err != nil {
		return nil, err
	}
	total := len(cursor.Remaining())
	var events []any
	for _, raw := range array(testCase["actions"]) {
		action := text(raw)
		switch action {
		case "finish":
			if finishErr := cursor.Finish(); finishErr != nil {
				events = append(events, errorProjection(finishErr, "container-value"))
			} else {
				events = append(events, map[string]any{"outcome": "finished"})
			}
		case "read", "read-with-different-limits":
			active := decoder
			if action == "read-with-different-limits" {
				mismatch := decoder.Limits()
				mismatch.MaxTotalElements++
				active = NewDecoder(mismatch)
			}
			child, readErr := cursor.Read(active)
			if readErr != nil {
				events = append(events, errorProjection(readErr, "container-value"))
			} else if child == nil {
				events = append(events, map[string]any{"outcome": "end"})
			} else {
				events = append(events, map[string]any{"outcome": "value", "tag": tagProjection(*child), "depth": child.Depth()})
			}
		}
	}
	return map[string]any{"outcome": "value", "elements_read": decoder.ElementsRead(), "remaining_offset": total - len(cursor.Remaining()), "events": events}, nil
}

func runCase(document, upstream, testCase map[string]any) map[string]any {
	if _, ok := testCase["der_tlv_case_id"]; ok {
		return verifyUpstream(upstream, testCase)
	}
	limits := fixtureLimits(document, testCase)
	decoder := NewDecoder(limits)
	root, err := decoder.DecodeExact(materialize(testCase["input"]))
	operation := text(testCase["operation"])
	if err == nil {
		switch operation {
		case "decode-exact":
			return map[string]any{"outcome": "value", "tag": tagProjection(root), "header_hex": hex.EncodeToString(root.Header()), "value_hex": hex.EncodeToString(root.Value()), "encoded_hex": hex.EncodeToString(root.Encoded()), "depth": root.Depth(), "elements_read": decoder.ElementsRead()}
		case "cursor-script":
			var result map[string]any
			result, err = cursorResult(testCase, decoder, root)
			if err == nil {
				return result
			}
		case "sequence", "set":
			var cursor *ASN1Cursor
			if operation == "sequence" {
				cursor, err = decoder.Sequence(root)
			} else {
				cursor, err = decoder.Set(root)
			}
			if err == nil {
				return map[string]any{"outcome": "value", "elements_read": decoder.ElementsRead(), "remaining_offset": len(root.Value()) - len(cursor.Remaining())}
			}
		case "explicit":
			var child ASN1Element
			child, err = decoder.Explicit(root, uint32(integer(testCase["tag_number"])))
			if err == nil {
				return map[string]any{"outcome": "value", "tag": tagProjection(child), "value_hex": hex.EncodeToString(child.Value()), "depth": child.Depth(), "elements_read": decoder.ElementsRead()}
			}
		default:
			tagNumber := uint32(0)
			if value, ok := testCase["tag_number"]; ok {
				tagNumber = uint32(integer(value))
			}
			var result map[string]any
			result, err = primitiveResult(operation, root, limits, tagNumber)
			if err == nil {
				return result
			}
		}
	}
	scope := "operation-input"
	if operation == "explicit" {
		if typed, ok := err.(*Error); ok && typed.Kind == Framing {
			scope = "container-value"
		}
	}
	return errorProjection(err, scope)
}

func TestPortableConformance(t *testing.T) {
	document := fixture(t, "der-asn1-v1")
	upstream := fixture(t, "der-tlv-v1")
	if len(array(document["cases"])) != 109 || len(array(document["error_ids"])) != 22 {
		t.Fatal("closed DER ASN.1 profile changed")
	}
	for _, raw := range array(document["cases"]) {
		testCase := object(raw)
		id := text(testCase["id"])
		t.Run(id, func(t *testing.T) {
			actual := normalize(runCase(document, upstream, testCase))
			if !reflect.DeepEqual(actual, testCase["expected"]) {
				t.Fatalf("actual %#v\nexpected %#v", actual, testCase["expected"])
			}
			if redacted, ok := testCase["redacted_input_hex"]; ok {
				encoded, _ := json.Marshal(actual)
				if strings.Contains(string(encoded), text(redacted)) {
					t.Fatal("hostile payload leaked")
				}
			}
		})
	}
}

func TestLimitsAndOIDEquality(t *testing.T) {
	decoder := NewDecoder(DefaultLimits())
	element, err := decoder.DecodeExact([]byte{0x06, 0x03, 0x2a, 0x03, 0x04})
	if err != nil {
		t.Fatal(err)
	}
	oid, err := DecodeObjectIdentifier(element, DefaultLimits())
	if err != nil || !oid.Equals([]uint64{1, 2, 3, 4}) || oid.Equals([]uint64{1, 2, 3}) {
		t.Fatal("OID equality failed")
	}
}

func TestZeroValuesCannotForgeValidatedState(t *testing.T) {
	decoder := NewDecoder(ASN1Limits{MaxDepth: 1, MaxTotalElements: 1, MaxOIDArcs: 1})
	if _, err := decoder.Sequence(ASN1Element{}); err == nil {
		t.Fatal("zero-value ASN1Element bypassed validation")
	} else if typed, ok := err.(*Error); !ok || typed.Kind != UnexpectedTag {
		t.Fatalf("unexpected forged-element error: %#v", err)
	}

	if _, err := (DERInteger{}).ToUint64(); err == nil {
		t.Fatal("zero-value DERInteger bypassed validation")
	} else if typed, ok := err.(*Error); !ok || typed.Kind != EmptyInteger {
		t.Fatalf("unexpected zero-value integer error: %#v", err)
	}
}
