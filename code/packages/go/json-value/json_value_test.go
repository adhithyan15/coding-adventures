package jsonvalue

import (
	"math"
	"reflect"
	"testing"
)

// ============================================================================
// Tests for FromAST (via Parse convenience function)
// ============================================================================
//
// These tests exercise the AST -> JsonValue conversion by parsing JSON text
// through the full pipeline (lexer -> parser -> AST -> JsonValue). Using
// Parse() rather than constructing AST nodes manually ensures we test the
// real integration path.

// TestParseEmptyObject verifies that an empty object produces a JsonObject
// with no pairs. This is the simplest possible JSON object.
func TestParseEmptyObject(t *testing.T) {
	val, err := Parse(`{}`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	obj, ok := val.(*JsonObject)
	if !ok {
		t.Fatalf("expected *JsonObject, got %T", val)
	}
	if len(obj.Pairs) != 0 {
		t.Errorf("expected 0 pairs, got %d", len(obj.Pairs))
	}
}

// TestParseEmptyArray verifies that an empty array produces a JsonArray
// with no elements.
func TestParseEmptyArray(t *testing.T) {
	val, err := Parse(`[]`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	arr, ok := val.(*JsonArray)
	if !ok {
		t.Fatalf("expected *JsonArray, got %T", val)
	}
	if len(arr.Elements) != 0 {
		t.Errorf("expected 0 elements, got %d", len(arr.Elements))
	}
}

// TestParseString verifies that a JSON string produces a JsonString.
// The lexer strips quotes and processes escapes, so the value is the
// raw string content.
func TestParseString(t *testing.T) {
	val, err := Parse(`"hello"`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	str, ok := val.(*JsonString)
	if !ok {
		t.Fatalf("expected *JsonString, got %T", val)
	}
	if str.Value != "hello" {
		t.Errorf("expected %q, got %q", "hello", str.Value)
	}
}

// TestParseEmptyString verifies that an empty JSON string produces
// a JsonString with an empty Value.
func TestParseEmptyString(t *testing.T) {
	val, err := Parse(`""`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	str, ok := val.(*JsonString)
	if !ok {
		t.Fatalf("expected *JsonString, got %T", val)
	}
	if str.Value != "" {
		t.Errorf("expected empty string, got %q", str.Value)
	}
}

// TestParseInteger verifies that a JSON integer (no decimal point or
// exponent) produces a JsonNumber with IsInteger=true.
func TestParseInteger(t *testing.T) {
	val, err := Parse(`42`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != 42.0 {
		t.Errorf("expected 42, got %v", num.Value)
	}
	if !num.IsInteger {
		t.Error("expected IsInteger=true")
	}
}

// TestParseZero verifies that JSON 0 is parsed correctly.
func TestParseZero(t *testing.T) {
	val, err := Parse(`0`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != 0.0 {
		t.Errorf("expected 0, got %v", num.Value)
	}
	if !num.IsInteger {
		t.Error("expected IsInteger=true for 0")
	}
}

// TestParseNegativeInteger verifies that negative integers are parsed
// correctly. The minus sign is part of the NUMBER token.
func TestParseNegativeInteger(t *testing.T) {
	val, err := Parse(`-17`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != -17.0 {
		t.Errorf("expected -17, got %v", num.Value)
	}
	if !num.IsInteger {
		t.Error("expected IsInteger=true")
	}
}

// TestParseFloat verifies that a number with a decimal point produces
// a JsonNumber with IsInteger=false.
func TestParseFloat(t *testing.T) {
	val, err := Parse(`3.14`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != 3.14 {
		t.Errorf("expected 3.14, got %v", num.Value)
	}
	if num.IsInteger {
		t.Error("expected IsInteger=false for 3.14")
	}
}

// TestParseExponent verifies that a number with an exponent is treated
// as a float, even if the result is mathematically an integer.
func TestParseExponent(t *testing.T) {
	val, err := Parse(`1e10`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != 1e10 {
		t.Errorf("expected 1e10, got %v", num.Value)
	}
	if num.IsInteger {
		t.Error("expected IsInteger=false for exponent notation")
	}
}

// TestParseTrue verifies that the JSON literal true produces JsonBool{true}.
func TestParseTrue(t *testing.T) {
	val, err := Parse(`true`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	b, ok := val.(*JsonBool)
	if !ok {
		t.Fatalf("expected *JsonBool, got %T", val)
	}
	if !b.Value {
		t.Error("expected true")
	}
}

// TestParseFalse verifies that the JSON literal false produces JsonBool{false}.
func TestParseFalse(t *testing.T) {
	val, err := Parse(`false`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	b, ok := val.(*JsonBool)
	if !ok {
		t.Fatalf("expected *JsonBool, got %T", val)
	}
	if b.Value {
		t.Error("expected false")
	}
}

// TestParseNull verifies that the JSON literal null produces JsonNull.
func TestParseNull(t *testing.T) {
	val, err := Parse(`null`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	_, ok := val.(*JsonNull)
	if !ok {
		t.Fatalf("expected *JsonNull, got %T", val)
	}
}

// TestParseSimpleObject verifies parsing of an object with one key-value pair.
func TestParseSimpleObject(t *testing.T) {
	val, err := Parse(`{"a": 1}`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	obj, ok := val.(*JsonObject)
	if !ok {
		t.Fatalf("expected *JsonObject, got %T", val)
	}
	if len(obj.Pairs) != 1 {
		t.Fatalf("expected 1 pair, got %d", len(obj.Pairs))
	}
	if obj.Pairs[0].Key != "a" {
		t.Errorf("expected key %q, got %q", "a", obj.Pairs[0].Key)
	}
	num, ok := obj.Pairs[0].Value.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber value, got %T", obj.Pairs[0].Value)
	}
	if num.Value != 1.0 {
		t.Errorf("expected value 1, got %v", num.Value)
	}
}

// TestParseMultiKeyObject verifies parsing of an object with multiple pairs.
// Keys should appear in insertion order.
func TestParseMultiKeyObject(t *testing.T) {
	val, err := Parse(`{"a": 1, "b": 2}`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	obj, ok := val.(*JsonObject)
	if !ok {
		t.Fatalf("expected *JsonObject, got %T", val)
	}
	if len(obj.Pairs) != 2 {
		t.Fatalf("expected 2 pairs, got %d", len(obj.Pairs))
	}
	if obj.Pairs[0].Key != "a" || obj.Pairs[1].Key != "b" {
		t.Errorf("expected keys [a, b], got [%s, %s]", obj.Pairs[0].Key, obj.Pairs[1].Key)
	}
}

// TestParseSimpleArray verifies parsing of an array of integers.
func TestParseSimpleArray(t *testing.T) {
	val, err := Parse(`[1, 2, 3]`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	arr, ok := val.(*JsonArray)
	if !ok {
		t.Fatalf("expected *JsonArray, got %T", val)
	}
	if len(arr.Elements) != 3 {
		t.Fatalf("expected 3 elements, got %d", len(arr.Elements))
	}
	for i, expected := range []float64{1, 2, 3} {
		num, ok := arr.Elements[i].(*JsonNumber)
		if !ok {
			t.Errorf("element %d: expected *JsonNumber, got %T", i, arr.Elements[i])
			continue
		}
		if num.Value != expected {
			t.Errorf("element %d: expected %v, got %v", i, expected, num.Value)
		}
	}
}

// TestParseMixedArray verifies parsing of an array with mixed JSON types.
func TestParseMixedArray(t *testing.T) {
	val, err := Parse(`[1, "two", true, null]`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	arr, ok := val.(*JsonArray)
	if !ok {
		t.Fatalf("expected *JsonArray, got %T", val)
	}
	if len(arr.Elements) != 4 {
		t.Fatalf("expected 4 elements, got %d", len(arr.Elements))
	}

	// Element 0: number 1
	if _, ok := arr.Elements[0].(*JsonNumber); !ok {
		t.Errorf("element 0: expected *JsonNumber, got %T", arr.Elements[0])
	}
	// Element 1: string "two"
	if s, ok := arr.Elements[1].(*JsonString); !ok || s.Value != "two" {
		t.Errorf("element 1: expected JsonString(two), got %T", arr.Elements[1])
	}
	// Element 2: boolean true
	if b, ok := arr.Elements[2].(*JsonBool); !ok || !b.Value {
		t.Errorf("element 2: expected JsonBool(true), got %T", arr.Elements[2])
	}
	// Element 3: null
	if _, ok := arr.Elements[3].(*JsonNull); !ok {
		t.Errorf("element 3: expected *JsonNull, got %T", arr.Elements[3])
	}
}

// TestParseNestedObject verifies parsing of nested objects.
func TestParseNestedObject(t *testing.T) {
	val, err := Parse(`{"a": {"b": 1}}`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	obj, ok := val.(*JsonObject)
	if !ok {
		t.Fatalf("expected *JsonObject, got %T", val)
	}
	inner, ok := obj.Pairs[0].Value.(*JsonObject)
	if !ok {
		t.Fatalf("expected inner *JsonObject, got %T", obj.Pairs[0].Value)
	}
	if inner.Pairs[0].Key != "b" {
		t.Errorf("expected inner key %q, got %q", "b", inner.Pairs[0].Key)
	}
}

// TestParseNestedArray verifies parsing of nested arrays.
func TestParseNestedArray(t *testing.T) {
	val, err := Parse(`[[1, 2], [3, 4]]`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	arr, ok := val.(*JsonArray)
	if !ok {
		t.Fatalf("expected *JsonArray, got %T", val)
	}
	if len(arr.Elements) != 2 {
		t.Fatalf("expected 2 elements, got %d", len(arr.Elements))
	}
	for i := 0; i < 2; i++ {
		if _, ok := arr.Elements[i].(*JsonArray); !ok {
			t.Errorf("element %d: expected *JsonArray, got %T", i, arr.Elements[i])
		}
	}
}

// TestParseComplexNested exercises deep nesting with mixed types.
func TestParseComplexNested(t *testing.T) {
	val, err := Parse(`{"users": [{"name": "Alice", "active": true}]}`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	obj, ok := val.(*JsonObject)
	if !ok {
		t.Fatalf("expected *JsonObject, got %T", val)
	}
	users, ok := obj.Pairs[0].Value.(*JsonArray)
	if !ok {
		t.Fatalf("expected *JsonArray for 'users', got %T", obj.Pairs[0].Value)
	}
	user, ok := users.Elements[0].(*JsonObject)
	if !ok {
		t.Fatalf("expected *JsonObject for user, got %T", users.Elements[0])
	}
	if len(user.Pairs) != 2 {
		t.Errorf("expected 2 pairs in user, got %d", len(user.Pairs))
	}
}

// TestParseStringWithEscapes verifies that escape sequences are processed.
// The lexer handles unescaping, so "hello\nworld" becomes "hello" + newline + "world".
func TestParseStringWithEscapes(t *testing.T) {
	val, err := Parse(`"hello\nworld"`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	str, ok := val.(*JsonString)
	if !ok {
		t.Fatalf("expected *JsonString, got %T", val)
	}
	if str.Value != "hello\nworld" {
		t.Errorf("expected %q, got %q", "hello\nworld", str.Value)
	}
}

// TestParseInvalidJSON verifies that invalid JSON produces an error.
func TestParseInvalidJSON(t *testing.T) {
	_, err := Parse(`not json`)
	if err == nil {
		t.Error("expected error for invalid JSON")
	}
}

// TestParseInvalidJSONBrace verifies that incomplete JSON produces an error.
func TestParseInvalidJSONBrace(t *testing.T) {
	_, err := Parse(`{`)
	if err == nil {
		t.Error("expected error for incomplete JSON")
	}
}

// TestFromASTNil verifies that a nil AST node produces an error.
func TestFromASTNil(t *testing.T) {
	_, err := FromAST(nil)
	if err == nil {
		t.Error("expected error for nil node")
	}
}

// ============================================================================
// Tests for ToNative
// ============================================================================

// TestToNativeObject verifies JsonObject -> map[string]interface{} conversion.
func TestToNativeObject(t *testing.T) {
	obj := &JsonObject{Pairs: []KeyValuePair{
		{Key: "a", Value: &JsonNumber{Value: 1, IsInteger: true}},
	}}
	native := ToNative(obj)
	m, ok := native.(map[string]interface{})
	if !ok {
		t.Fatalf("expected map[string]interface{}, got %T", native)
	}
	if m["a"] != 1 {
		t.Errorf("expected a=1, got a=%v", m["a"])
	}
}

// TestToNativeArray verifies JsonArray -> []interface{} conversion.
func TestToNativeArray(t *testing.T) {
	arr := &JsonArray{Elements: []JsonValue{
		&JsonNumber{Value: 1, IsInteger: true},
		&JsonNumber{Value: 2, IsInteger: true},
	}}
	native := ToNative(arr)
	s, ok := native.([]interface{})
	if !ok {
		t.Fatalf("expected []interface{}, got %T", native)
	}
	if len(s) != 2 || s[0] != 1 || s[1] != 2 {
		t.Errorf("expected [1, 2], got %v", s)
	}
}

// TestToNativeString verifies JsonString -> string conversion.
func TestToNativeString(t *testing.T) {
	native := ToNative(&JsonString{Value: "hello"})
	if native != "hello" {
		t.Errorf("expected %q, got %v", "hello", native)
	}
}

// TestToNativeIntNumber verifies JsonNumber(integer) -> int conversion.
func TestToNativeIntNumber(t *testing.T) {
	native := ToNative(&JsonNumber{Value: 42, IsInteger: true})
	if native != 42 {
		t.Errorf("expected 42 (int), got %v (%T)", native, native)
	}
}

// TestToNativeFloatNumber verifies JsonNumber(float) -> float64 conversion.
func TestToNativeFloatNumber(t *testing.T) {
	native := ToNative(&JsonNumber{Value: 3.14, IsInteger: false})
	if native != 3.14 {
		t.Errorf("expected 3.14, got %v", native)
	}
}

// TestToNativeBool verifies JsonBool -> bool conversion.
func TestToNativeBool(t *testing.T) {
	native := ToNative(&JsonBool{Value: true})
	if native != true {
		t.Errorf("expected true, got %v", native)
	}
}

// TestToNativeNull verifies JsonNull -> nil conversion.
func TestToNativeNull(t *testing.T) {
	native := ToNative(&JsonNull{})
	if native != nil {
		t.Errorf("expected nil, got %v", native)
	}
}

// TestToNativeNilValue verifies that a nil JsonValue produces nil.
func TestToNativeNilValue(t *testing.T) {
	native := ToNative(nil)
	if native != nil {
		t.Errorf("expected nil, got %v", native)
	}
}

// TestToNativeNested verifies recursive conversion of deeply nested structures.
func TestToNativeNested(t *testing.T) {
	val := &JsonObject{Pairs: []KeyValuePair{
		{Key: "items", Value: &JsonArray{Elements: []JsonValue{
			&JsonObject{Pairs: []KeyValuePair{
				{Key: "id", Value: &JsonNumber{Value: 1, IsInteger: true}},
				{Key: "name", Value: &JsonString{Value: "first"}},
			}},
		}}},
	}}
	native := ToNative(val)
	m := native.(map[string]interface{})
	items := m["items"].([]interface{})
	item := items[0].(map[string]interface{})
	if item["id"] != 1 || item["name"] != "first" {
		t.Errorf("unexpected nested native value: %v", native)
	}
}

// ============================================================================
// Tests for FromNative
// ============================================================================

// TestFromNativeMap verifies map[string]interface{} -> JsonObject conversion.
func TestFromNativeMap(t *testing.T) {
	val, err := FromNative(map[string]interface{}{"a": 1})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	obj, ok := val.(*JsonObject)
	if !ok {
		t.Fatalf("expected *JsonObject, got %T", val)
	}
	if len(obj.Pairs) != 1 {
		t.Errorf("expected 1 pair, got %d", len(obj.Pairs))
	}
}

// TestFromNativeSlice verifies []interface{} -> JsonArray conversion.
func TestFromNativeSlice(t *testing.T) {
	val, err := FromNative([]interface{}{1, 2})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	arr, ok := val.(*JsonArray)
	if !ok {
		t.Fatalf("expected *JsonArray, got %T", val)
	}
	if len(arr.Elements) != 2 {
		t.Errorf("expected 2 elements, got %d", len(arr.Elements))
	}
}

// TestFromNativeString verifies string -> JsonString conversion.
func TestFromNativeString(t *testing.T) {
	val, err := FromNative("hello")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	str, ok := val.(*JsonString)
	if !ok {
		t.Fatalf("expected *JsonString, got %T", val)
	}
	if str.Value != "hello" {
		t.Errorf("expected %q, got %q", "hello", str.Value)
	}
}

// TestFromNativeInt verifies int -> JsonNumber(integer) conversion.
func TestFromNativeInt(t *testing.T) {
	val, err := FromNative(42)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != 42 || !num.IsInteger {
		t.Errorf("expected JsonNumber(42, integer), got %v, %v", num.Value, num.IsInteger)
	}
}

// TestFromNativeFloat verifies float64 -> JsonNumber(float) conversion.
func TestFromNativeFloat(t *testing.T) {
	val, err := FromNative(3.14)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != 3.14 || num.IsInteger {
		t.Errorf("expected JsonNumber(3.14, float), got %v, %v", num.Value, num.IsInteger)
	}
}

// TestFromNativeBool verifies bool -> JsonBool conversion.
func TestFromNativeBool(t *testing.T) {
	val, err := FromNative(true)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	b, ok := val.(*JsonBool)
	if !ok {
		t.Fatalf("expected *JsonBool, got %T", val)
	}
	if !b.Value {
		t.Error("expected true")
	}
}

// TestFromNativeNil verifies nil -> JsonNull conversion.
func TestFromNativeNil(t *testing.T) {
	val, err := FromNative(nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if _, ok := val.(*JsonNull); !ok {
		t.Fatalf("expected *JsonNull, got %T", val)
	}
}

// TestFromNativeNested verifies deeply nested native structures.
func TestFromNativeNested(t *testing.T) {
	native := map[string]interface{}{
		"items": []interface{}{
			map[string]interface{}{
				"id":   1,
				"name": "first",
			},
		},
	}
	val, err := FromNative(native)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	obj, ok := val.(*JsonObject)
	if !ok {
		t.Fatalf("expected *JsonObject, got %T", val)
	}
	// Verify nested structure exists
	found := false
	for _, p := range obj.Pairs {
		if p.Key == "items" {
			arr, ok := p.Value.(*JsonArray)
			if !ok {
				t.Fatalf("expected *JsonArray for 'items', got %T", p.Value)
			}
			if len(arr.Elements) != 1 {
				t.Errorf("expected 1 element, got %d", len(arr.Elements))
			}
			found = true
		}
	}
	if !found {
		t.Error("'items' key not found")
	}
}

// TestFromNativeUnsupportedType verifies that unsupported types produce errors.
func TestFromNativeUnsupportedType(t *testing.T) {
	_, err := FromNative(struct{}{})
	if err == nil {
		t.Error("expected error for unsupported type (struct)")
	}
}

// TestFromNativeIntVariants verifies all integer type variants.
func TestFromNativeIntVariants(t *testing.T) {
	variants := []interface{}{
		int8(1), int16(2), int32(3), int64(4),
		uint(5), uint8(6), uint16(7), uint32(8), uint64(9),
	}
	for _, v := range variants {
		val, err := FromNative(v)
		if err != nil {
			t.Errorf("unexpected error for %T: %v", v, err)
			continue
		}
		num, ok := val.(*JsonNumber)
		if !ok {
			t.Errorf("expected *JsonNumber for %T, got %T", v, val)
			continue
		}
		if !num.IsInteger {
			t.Errorf("expected IsInteger=true for %T", v)
		}
	}
}

// TestFromNativeFloat32 verifies float32 -> JsonNumber conversion.
func TestFromNativeFloat32(t *testing.T) {
	val, err := FromNative(float32(1.5))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.IsInteger {
		t.Error("expected IsInteger=false for float32")
	}
}

// TestFromNativeJsonValue verifies that a JsonValue passed to FromNative
// is returned as-is (passthrough).
func TestFromNativeJsonValue(t *testing.T) {
	original := &JsonString{Value: "already a value"}
	val, err := FromNative(original)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != original {
		t.Error("expected same pointer for JsonValue passthrough")
	}
}

// ============================================================================
// Tests for ParseNative
// ============================================================================

// TestParseNativeObject verifies the full pipeline: text -> native types.
func TestParseNativeObject(t *testing.T) {
	result, err := ParseNative(`{"a": 1}`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	m, ok := result.(map[string]interface{})
	if !ok {
		t.Fatalf("expected map[string]interface{}, got %T", result)
	}
	if m["a"] != 1 {
		t.Errorf("expected a=1, got a=%v (%T)", m["a"], m["a"])
	}
}

// TestParseNativeInvalidJSON verifies that invalid JSON produces an error
// in the convenience function path.
func TestParseNativeInvalidJSON(t *testing.T) {
	_, err := ParseNative(`{`)
	if err == nil {
		t.Error("expected error for invalid JSON")
	}
}

// ============================================================================
// Round-trip Tests
// ============================================================================

// TestRoundTripSimple verifies that from_native -> to_native produces
// equivalent values for simple types.
func TestRoundTripSimple(t *testing.T) {
	tests := []struct {
		name  string
		input interface{}
	}{
		{"string", "hello"},
		{"int", 42},
		{"float", 3.14},
		{"bool_true", true},
		{"bool_false", false},
		{"nil", nil},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			jv, err := FromNative(tt.input)
			if err != nil {
				t.Fatalf("FromNative error: %v", err)
			}
			output := ToNative(jv)
			if !reflect.DeepEqual(output, tt.input) {
				t.Errorf("round-trip failed: input=%v (%T), output=%v (%T)",
					tt.input, tt.input, output, output)
			}
		})
	}
}

// TestRoundTripNested verifies round-trip for a complex nested structure.
func TestRoundTripNested(t *testing.T) {
	input := map[string]interface{}{
		"name": "Alice",
		"age":  42,
		"scores": []interface{}{
			95.5, 87.0, 92.3,
		},
		"active": true,
		"address": map[string]interface{}{
			"city": "Wonderland",
		},
	}

	jv, err := FromNative(input)
	if err != nil {
		t.Fatalf("FromNative error: %v", err)
	}
	output := ToNative(jv)

	// We can't use DeepEqual directly because map iteration order may differ
	// and int vs float differs. Check key structure manually.
	m, ok := output.(map[string]interface{})
	if !ok {
		t.Fatalf("expected map, got %T", output)
	}
	if m["name"] != "Alice" {
		t.Errorf("name: expected Alice, got %v", m["name"])
	}
	if m["active"] != true {
		t.Errorf("active: expected true, got %v", m["active"])
	}
}

// TestJsonValueError verifies the error type's message format.
func TestJsonValueError(t *testing.T) {
	err := &JsonValueError{Message: "test error"}
	expected := "json value error: test error"
	if err.Error() != expected {
		t.Errorf("expected %q, got %q", expected, err.Error())
	}
}

// TestParseNegativeFloat verifies negative floats are handled.
func TestParseNegativeFloat(t *testing.T) {
	val, err := Parse(`-3.14`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != -3.14 {
		t.Errorf("expected -3.14, got %v", num.Value)
	}
	if num.IsInteger {
		t.Error("expected IsInteger=false for -3.14")
	}
}

// TestParseLargeNumber verifies large numbers are handled correctly.
func TestParseLargeNumber(t *testing.T) {
	val, err := Parse(`9999999999999`)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	num, ok := val.(*JsonNumber)
	if !ok {
		t.Fatalf("expected *JsonNumber, got %T", val)
	}
	if num.Value != 9999999999999 {
		t.Errorf("expected 9999999999999, got %v", num.Value)
	}
}

// TestParseNativeComplex verifies parsing a complex JSON document to native.
func TestParseNativeComplex(t *testing.T) {
	json := `{
		"string": "hello",
		"number": 42,
		"float": 3.14,
		"bool": true,
		"null_val": null,
		"array": [1, 2, 3],
		"nested": {"key": "value"}
	}`
	result, err := ParseNative(json)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	m, ok := result.(map[string]interface{})
	if !ok {
		t.Fatalf("expected map, got %T", result)
	}
	if m["string"] != "hello" {
		t.Errorf("expected hello, got %v", m["string"])
	}
	if m["number"] != 42 {
		t.Errorf("expected 42, got %v (%T)", m["number"], m["number"])
	}
	if m["bool"] != true {
		t.Errorf("expected true, got %v", m["bool"])
	}
	if m["null_val"] != nil {
		t.Errorf("expected nil, got %v", m["null_val"])
	}
}

// TestFromNativeNestedError verifies that unsupported types in nested
// structures propagate errors.
func TestFromNativeNestedError(t *testing.T) {
	_, err := FromNative(map[string]interface{}{
		"bad": struct{}{},
	})
	if err == nil {
		t.Error("expected error for nested unsupported type")
	}
}

// TestFromNativeSliceError verifies errors in array elements.
func TestFromNativeSliceError(t *testing.T) {
	_, err := FromNative([]interface{}{struct{}{}})
	if err == nil {
		t.Error("expected error for unsupported element type")
	}
}

// TestParseNumberFormats verifies various number formats from the spec.
func TestParseNumberFormats(t *testing.T) {
	tests := []struct {
		input     string
		value     float64
		isInteger bool
	}{
		{"0", 0, true},
		{"-0", 0, true},
		{"123", 123, true},
		{"-123", -123, true},
		{"0.5", 0.5, false},
		{"1.0", 1.0, false},
		{"1e2", 100, false},
		{"1E2", 100, false},
		{"1.5e2", 150, false},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			val, err := Parse(tt.input)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			num, ok := val.(*JsonNumber)
			if !ok {
				t.Fatalf("expected *JsonNumber, got %T", val)
			}
			if math.Abs(num.Value-tt.value) > 1e-10 {
				t.Errorf("expected %v, got %v", tt.value, num.Value)
			}
			if num.IsInteger != tt.isInteger {
				t.Errorf("expected IsInteger=%v, got %v", tt.isInteger, num.IsInteger)
			}
		})
	}
}

// TestFromNativeBoolFalse verifies false -> JsonBool(false) conversion.
func TestFromNativeBoolFalse(t *testing.T) {
	val, err := FromNative(false)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	b, ok := val.(*JsonBool)
	if !ok {
		t.Fatalf("expected *JsonBool, got %T", val)
	}
	if b.Value {
		t.Error("expected false")
	}
}
