package canonicalcbor

import (
	"bytes"
	"errors"
	"fmt"
	"math"
	"strings"
	"testing"
	"unicode/utf8"
)

// ============================================================================
// 1. Unit Tests for Value, Kind, Constructors, and Defensive Copies
// ============================================================================

func TestValueConstructorsAndAccessors(t *testing.T) {
	// Unsigned
	u := NewUnsigned(12345)
	if u.Kind() != KindUnsigned || u.Unsigned() != 12345 {
		t.Fatalf("Unsigned mismatch")
	}
	if u.Kind().String() != "unsigned" {
		t.Fatalf("Kind.String mismatch")
	}

	// Negative
	neg := NewNegative(41) // -42
	if neg.Kind() != KindNegative || neg.Negative() != 41 {
		t.Fatalf("Negative mismatch")
	}
	if neg.Kind().String() != "negative" {
		t.Fatalf("Kind.String mismatch")
	}

	// Bytes with defensive copy
	origBytes := []byte{1, 2, 3, 4}
	bVal := NewBytes(origBytes)
	origBytes[0] = 99
	if bVal.Bytes()[0] != 1 {
		t.Fatalf("NewBytes did not defensively copy input")
	}
	returnedBytes := bVal.Bytes()
	returnedBytes[0] = 88
	if bVal.Bytes()[0] != 1 {
		t.Fatalf("Bytes() did not return defensive copy")
	}
	if bVal.Kind() != KindBytes || bVal.Kind().String() != "bytes" {
		t.Fatalf("Bytes kind mismatch")
	}
	// Nil bytes
	nilBVal := NewBytes(nil)
	if nilBVal.Bytes() == nil || len(nilBVal.Bytes()) != 0 {
		t.Fatalf("NewBytes(nil) should produce empty slice")
	}

	// Text
	tVal := NewText("hello world")
	if tVal.Kind() != KindText || tVal.Text() != "hello world" || tVal.Kind().String() != "text" {
		t.Fatalf("Text mismatch")
	}

	// Array with defensive copy
	origItems := []Value{NewUnsigned(1), NewUnsigned(2)}
	arrVal := NewArray(origItems)
	origItems[0] = NewUnsigned(99)
	if arrVal.Array()[0].Unsigned() != 1 {
		t.Fatalf("NewArray did not defensively copy input")
	}
	retItems := arrVal.Array()
	retItems[0] = NewUnsigned(88)
	if arrVal.Array()[0].Unsigned() != 1 {
		t.Fatalf("Array() did not return defensive copy")
	}
	if arrVal.Kind() != KindArray || arrVal.Kind().String() != "array" {
		t.Fatalf("Array kind mismatch")
	}
	// Nil array
	nilArrVal := NewArray(nil)
	if nilArrVal.Array() == nil || len(nilArrVal.Array()) != 0 {
		t.Fatalf("NewArray(nil) should produce empty slice")
	}

	// Map with defensive copy
	origEntries := []MapEntry{
		{Key: NewText("k"), Value: NewText("v")},
	}
	mapVal := NewMap(origEntries)
	origEntries[0].Value = NewText("mutated")
	if mapVal.Map()[0].Value.Text() != "v" {
		t.Fatalf("NewMap did not defensively copy input")
	}
	retEntries := mapVal.Map()
	retEntries[0].Value = NewText("mutated2")
	if mapVal.Map()[0].Value.Text() != "v" {
		t.Fatalf("Map() did not return defensive copy")
	}
	if mapVal.Kind() != KindMap || mapVal.Kind().String() != "map" {
		t.Fatalf("Map kind mismatch")
	}
	// Nil map
	nilMapVal := NewMap(nil)
	if nilMapVal.Map() == nil || len(nilMapVal.Map()) != 0 {
		t.Fatalf("NewMap(nil) should produce empty slice")
	}

	// Tag
	tagVal := NewTag(42, NewUnsigned(100))
	tagNum, inner := tagVal.Tag()
	if tagVal.Kind() != KindTag || tagNum != 42 || inner.Unsigned() != 100 || tagVal.Kind().String() != "tag" {
		t.Fatalf("Tag mismatch")
	}

	// Bool
	bTrue := NewBool(true)
	bFalse := NewBool(false)
	if bTrue.Kind() != KindBool || !bTrue.Bool() || bTrue.Kind().String() != "bool" {
		t.Fatalf("Bool true mismatch")
	}
	if bFalse.Kind() != KindBool || bFalse.Bool() {
		t.Fatalf("Bool false mismatch")
	}

	// Null
	nullVal := Null()
	if nullVal.Kind() != KindNull || !nullVal.IsNull() || nullVal.Kind().String() != "null" {
		t.Fatalf("Null mismatch")
	}

	// Unknown kind string
	unknownKind := Kind(999)
	if unknownKind.String() != "unknown" {
		t.Fatalf("unknown kind string mismatch")
	}
}

func TestValueEquality(t *testing.T) {
	cases := []struct {
		a, b  Value
		equal bool
	}{
		{NewUnsigned(10), NewUnsigned(10), true},
		{NewUnsigned(10), NewUnsigned(20), false},
		{NewUnsigned(10), NewNegative(10), false},
		{NewNegative(5), NewNegative(5), true},
		{NewNegative(5), NewNegative(6), false},
		{NewBytes([]byte{1, 2}), NewBytes([]byte{1, 2}), true},
		{NewBytes([]byte{1, 2}), NewBytes([]byte{1, 3}), false},
		{NewText("foo"), NewText("foo"), true},
		{NewText("foo"), NewText("bar"), false},
		{NewArray([]Value{NewUnsigned(1)}), NewArray([]Value{NewUnsigned(1)}), true},
		{NewArray([]Value{NewUnsigned(1)}), NewArray([]Value{NewUnsigned(2)}), false},
		{NewArray([]Value{NewUnsigned(1)}), NewArray([]Value{}), false},
		{
			NewMap([]MapEntry{{Key: NewText("a"), Value: NewUnsigned(1)}}),
			NewMap([]MapEntry{{Key: NewText("a"), Value: NewUnsigned(1)}}),
			true,
		},
		{
			NewMap([]MapEntry{{Key: NewText("a"), Value: NewUnsigned(1)}}),
			NewMap([]MapEntry{{Key: NewText("a"), Value: NewUnsigned(2)}}),
			false,
		},
		{
			NewMap([]MapEntry{{Key: NewText("a"), Value: NewUnsigned(1)}}),
			NewMap([]MapEntry{{Key: NewText("b"), Value: NewUnsigned(1)}}),
			false,
		},
		{
			NewMap([]MapEntry{{Key: NewText("a"), Value: NewUnsigned(1)}}),
			NewMap([]MapEntry{}),
			false,
		},
		{NewTag(1, NewUnsigned(10)), NewTag(1, NewUnsigned(10)), true},
		{NewTag(1, NewUnsigned(10)), NewTag(2, NewUnsigned(10)), false},
		{NewTag(1, NewUnsigned(10)), NewTag(1, NewUnsigned(20)), false},
		{NewBool(true), NewBool(true), true},
		{NewBool(true), NewBool(false), false},
		{Null(), Null(), true},
		{Null(), NewBool(false), false},
	}

	for i, tc := range cases {
		if tc.a.Equal(tc.b) != tc.equal {
			t.Errorf("case %d: Equal(%v, %v) got %v, want %v", i, tc.a, tc.b, tc.a.Equal(tc.b), tc.equal)
		}
	}
}

func TestConvenienceWrappers(t *testing.T) {
	v := NewUnsigned(42)
	encoded := Encode(v)
	if len(encoded) != 2 || encoded[0] != 0x18 || encoded[1] != 42 {
		t.Fatalf("unexpected Encode output: %v", encoded)
	}

	var dst []byte
	EncodeInto(v, &dst)
	if !bytes.Equal(dst, encoded) {
		t.Fatalf("unexpected EncodeInto output: %v", dst)
	}

	// Panic path for invalid values
	badMap := NewMap([]MapEntry{
		{Key: NewUnsigned(1), Value: NewUnsigned(10)},
		{Key: NewUnsigned(1), Value: NewUnsigned(20)}, // duplicate key
	})

	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Errorf("expected Encode to panic on duplicate map key")
			}
		}()
		Encode(badMap)
	}()

	func() {
		defer func() {
			if r := recover(); r == nil {
				t.Errorf("expected EncodeInto to panic on duplicate map key")
			}
		}()
		var out []byte
		EncodeInto(badMap, &out)
	}()
}

func TestErrorMethodsAndProperties(t *testing.T) {
	allCodes := []ErrorCode{
		ErrUnexpectedEof,
		ErrTrailingBytes,
		ErrReserved,
		ErrIndefinite,
		ErrNonMinimalInteger,
		ErrInvalidUtf8,
		ErrNonCanonicalMapOrder,
		ErrUnsupportedSimple,
		ErrFloatNotSupported,
		ErrTooDeep,
		ErrLengthTooLarge,
		ErrDuplicateMapKey,
		ErrEncodeTooDeep,
		ErrEncodeTooLarge,
	}

	for _, code := range allCodes {
		err := &Error{Code: code}
		msg := err.Error()
		if !strings.HasPrefix(msg, "canonical-cbor: ") {
			t.Errorf("error string for %s missing prefix: %q", code, msg)
		}
		if err.ID() != string(code) {
			t.Errorf("error ID mismatch: got %q, want %q", err.ID(), code)
		}
		if !errors.Is(err, &Error{Code: code}) {
			t.Errorf("errors.Is failed for %s", code)
		}
		if errors.Is(err, &Error{Code: "different-code"}) {
			t.Errorf("errors.Is false positive for %s", code)
		}
	}

	unknownErr := &Error{Code: "custom-code"}
	if unknownErr.Error() != "canonical-cbor: unknown error" {
		t.Errorf("unexpected fallback error message: %q", unknownErr.Error())
	}
}

// ============================================================================
// 2. Boundary & Negative Unit Tests
// ============================================================================

func TestIntegerWidthTransitions(t *testing.T) {
	// Unsigned width transitions:
	// 0..23 (1 byte inline)
	// 24..255 (2 bytes)
	// 256..65535 (3 bytes)
	// 65536..4294967295 (5 bytes)
	// 4294967296..MaxUint64 (9 bytes)
	unsignedCases := []struct {
		val         uint64
		expectedLen int
		majorLead   byte
	}{
		{0, 1, 0x00},
		{23, 1, 0x17},
		{24, 2, 0x18},
		{255, 2, 0x18},
		{256, 3, 0x19},
		{65535, 3, 0x19},
		{65536, 5, 0x1A},
		{4294967295, 5, 0x1A},
		{4294967296, 9, 0x1B},
		{math.MaxUint64, 9, 0x1B},
	}

	for _, tc := range unsignedCases {
		v := NewUnsigned(tc.val)
		enc, err := EncodeChecked(v)
		if err != nil {
			t.Fatalf("failed to encode unsigned %d: %v", tc.val, err)
		}
		if len(enc) != tc.expectedLen {
			t.Errorf("unsigned %d: got length %d, want %d", tc.val, len(enc), tc.expectedLen)
		}
		if enc[0] != tc.majorLead {
			t.Errorf("unsigned %d: got header %x, want %x", tc.val, enc[0], tc.majorLead)
		}
		dec, err := Decode(enc)
		if err != nil {
			t.Fatalf("failed to decode unsigned %d: %v", tc.val, err)
		}
		if dec.Unsigned() != tc.val {
			t.Errorf("unsigned %d round-trip mismatch: got %d", tc.val, dec.Unsigned())
		}
	}

	// Negative width transitions:
	negativeCases := []struct {
		val         uint64
		expectedLen int
		majorLead   byte
	}{
		{0, 1, 0x20},
		{23, 1, 0x37},
		{24, 2, 0x38},
		{255, 2, 0x38},
		{256, 3, 0x39},
		{65535, 3, 0x39},
		{65536, 5, 0x3A},
		{4294967295, 5, 0x3A},
		{4294967296, 9, 0x3B},
		{math.MaxUint64, 9, 0x3B},
	}

	for _, tc := range negativeCases {
		v := NewNegative(tc.val)
		enc, err := EncodeChecked(v)
		if err != nil {
			t.Fatalf("failed to encode negative %d: %v", tc.val, err)
		}
		if len(enc) != tc.expectedLen {
			t.Errorf("negative %d: got length %d, want %d", tc.val, len(enc), tc.expectedLen)
		}
		if enc[0] != tc.majorLead {
			t.Errorf("negative %d: got header %x, want %x", tc.val, enc[0], tc.majorLead)
		}
		dec, err := Decode(enc)
		if err != nil {
			t.Fatalf("failed to decode negative %d: %v", tc.val, err)
		}
		if dec.Negative() != tc.val {
			t.Errorf("negative %d round-trip mismatch: got %d", tc.val, dec.Negative())
		}
	}
}

func TestDepthTransitions(t *testing.T) {
	// Depth 127: passes
	val127 := buildNestedArray(127)
	enc127, err := EncodeChecked(val127)
	if err != nil {
		t.Fatalf("depth 127 encode failed: %v", err)
	}
	dec127, err := Decode(enc127)
	if err != nil {
		t.Fatalf("depth 127 decode failed: %v", err)
	}
	if !dec127.Equal(val127) {
		t.Fatalf("depth 127 round trip equality failed")
	}

	// Depth 128: passes
	val128 := buildNestedArray(128)
	enc128, err := EncodeChecked(val128)
	if err != nil {
		t.Fatalf("depth 128 encode failed: %v", err)
	}
	dec128, err := Decode(enc128)
	if err != nil {
		t.Fatalf("depth 128 decode failed: %v", err)
	}
	if !dec128.Equal(val128) {
		t.Fatalf("depth 128 round trip equality failed")
	}

	// Depth 129: fails on encode with encode-too-deep
	val129 := buildNestedArray(129)
	_, err = EncodeChecked(val129)
	if err == nil || err.(*Error).ID() != "encode-too-deep" {
		t.Fatalf("expected encode-too-deep, got %v", err)
	}

	// Depth 129 wire: fails on decode with too-deep
	wire129 := buildNestedArrayWire(129)
	_, err = Decode(wire129)
	if err == nil || err.(*Error).ID() != "too-deep" {
		t.Fatalf("expected too-deep, got %v", err)
	}

	// Tag depth recursion
	tagVal128 := Null()
	for i := 0; i < 128; i++ {
		tagVal128 = NewTag(uint64(i), tagVal128)
	}
	encTag, err := EncodeChecked(tagVal128)
	if err != nil {
		t.Fatalf("tag depth 128 encode failed: %v", err)
	}
	decTag, err := Decode(encTag)
	if err != nil {
		t.Fatalf("tag depth 128 decode failed: %v", err)
	}
	if !decTag.Equal(tagVal128) {
		t.Fatalf("tag depth 128 equality failed")
	}

	tagVal129 := NewTag(129, tagVal128)
	_, err = EncodeChecked(tagVal129)
	if err == nil || err.(*Error).ID() != "encode-too-deep" {
		t.Fatalf("expected tag depth 129 encode-too-deep, got %v", err)
	}
}

func TestSizeLimitTransitions(t *testing.T) {
	// Size limit 1_048_576 bytes
	// Header for 1,048,571 bytes payload is 5 bytes (0x5A + 4 bytes length).
	// Total encoded = 5 + 1,048,571 = 1,048,576 bytes. Exactly at the limit.
	valExact := NewBytes(make([]byte, 1048571))
	encExact, err := EncodeChecked(valExact)
	if err != nil {
		t.Fatalf("size 1,048,576 encode failed: %v", err)
	}
	if len(encExact) != MaxEncodedBytes {
		t.Fatalf("expected length %d, got %d", MaxEncodedBytes, len(encExact))
	}

	// Size 1,048,575 bytes (one below limit)
	valBelow := NewBytes(make([]byte, 1048570))
	encBelow, err := EncodeChecked(valBelow)
	if err != nil {
		t.Fatalf("size 1,048,575 encode failed: %v", err)
	}
	if len(encBelow) != MaxEncodedBytes-1 {
		t.Fatalf("expected length %d, got %d", MaxEncodedBytes-1, len(encBelow))
	}

	// Size 1,048,577 bytes (one above limit: 1048572 + 5 = 1048577)
	valAbove := NewBytes(make([]byte, 1048572))
	_, err = EncodeChecked(valAbove)
	if err == nil || err.(*Error).ID() != "encode-too-large" {
		t.Fatalf("expected encode-too-large, got %v", err)
	}
}

func TestUtf8BoundaryAndRejections(t *testing.T) {
	badUtf8Cases := []struct {
		name string
		raw  []byte
	}{
		{"surrogate lead U+D800", []byte{0xED, 0xA0, 0x80}},
		{"surrogate trail U+DFFF", []byte{0xED, 0xBF, 0xBF}},
		{"overlong NUL 2-byte", []byte{0xC0, 0x80}},
		{"overlong slash 3-byte", []byte{0xE0, 0x80, 0xAF}},
		{"overlong 4-byte", []byte{0xF0, 0x80, 0x80, 0xAF}},
		{"truncated 2-byte", []byte{0xC3}},
		{"truncated 3-byte", []byte{0xE2, 0x98}},
		{"out of range U+110000", []byte{0xF4, 0x90, 0x80, 0x80}},
		{"invalid byte 0xFF", []byte{0xFF}},
		{"invalid continuation byte", []byte{0x80}},
	}

	for _, tc := range badUtf8Cases {
		t.Run(tc.name, func(t *testing.T) {
			wire := []byte{0x60 | byte(len(tc.raw))}
			wire = append(wire, tc.raw...)
			_, err := Decode(wire)
			if err == nil || err.(*Error).ID() != "invalid-utf8" {
				t.Fatalf("expected invalid-utf8 for %s, got %v", tc.name, err)
			}
		})
	}
}

func TestHostileLengths(t *testing.T) {
	// Length declaring MaxUint64 in major type 2 (bytes), 3 (text), 4 (array), 5 (map)
	cases := []struct {
		name string
		wire []byte
	}{
		{"hostile-bytes", []byte{0x5B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF}},
		{"hostile-text", []byte{0x7B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF}},
		{"hostile-array", []byte{0x9B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF}},
		{"hostile-map", []byte{0xBB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF}},
		{"array-len-10-with-1-byte", []byte{0x8A, 0x00}},
		{"map-len-5-with-2-bytes", []byte{0xA5, 0x00, 0x01}},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			_, err := Decode(tc.wire)
			if err == nil || err.(*Error).ID() != "length-too-large" {
				t.Fatalf("expected length-too-large for %s, got %v", tc.name, err)
			}
		})
	}
}

func TestSecretSentinelLeakage(t *testing.T) {
	// Verify that error messages NEVER reflect payload contents, keys, or data
	secretStr := "SECRET_TOKEN_XYZ_987654321"
	secretBytes := []byte{0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE}

	// 1. Duplicate map keys with secret text
	dupMap := NewMap([]MapEntry{
		{Key: NewText(secretStr), Value: NewUnsigned(1)},
		{Key: NewText(secretStr), Value: NewUnsigned(2)},
	})
	_, err := EncodeChecked(dupMap)
	if err == nil {
		t.Fatal("expected duplicate map key error")
	}
	msg := err.Error()
	if strings.Contains(msg, secretStr) {
		t.Fatalf("error message leaked secret payload string: %q", msg)
	}

	// 2. Decode invalid map with secret key bytes
	encodedSecretKey, _ := EncodeChecked(NewBytes(secretBytes))
	var badMapWire []byte
	badMapWire = append(badMapWire, 0xA2) // map of 2 items
	badMapWire = append(badMapWire, encodedSecretKey...)
	badMapWire = append(badMapWire, 0x01)
	badMapWire = append(badMapWire, encodedSecretKey...)
	badMapWire = append(badMapWire, 0x02) // duplicate key wire

	_, err = Decode(badMapWire)
	if err == nil {
		t.Fatal("expected decode error")
	}
	msg = err.Error()
	if strings.Contains(msg, "DEADBEEF") || strings.Contains(msg, "deadbeef") {
		t.Fatalf("error message leaked secret hex bytes: %q", msg)
	}
	if strings.Contains(msg, string(secretBytes)) {
		t.Fatalf("error message leaked raw secret bytes: %q", msg)
	}

	// 3. SpannedMapEntry formatting redacts secret key and value payloads
	spanned := SpannedMapEntry{
		Key:       NewText(secretStr),
		Value:     NewBytes(secretBytes),
		ValueSpan: Span{Start: 5, End: 20},
	}
	if strings.Contains(spanned.String(), secretStr) {
		t.Fatalf("SpannedMapEntry.String() leaked secret text: %s", spanned.String())
	}
	if strings.Contains(fmt.Sprintf("%#v", spanned), secretStr) {
		t.Fatalf("SpannedMapEntry.GoString() leaked secret text: %#v", spanned)
	}
}

func TestAtomicEncodeIntoCheckedRollback(t *testing.T) {
	valTooDeep := buildNestedArray(129)
	dst := []byte{0x01, 0x02, 0x03}
	err := EncodeIntoChecked(valTooDeep, &dst)
	if err == nil {
		t.Fatal("expected error on too deep value")
	}
	if len(dst) != 3 || dst[0] != 0x01 || dst[1] != 0x02 || dst[2] != 0x03 {
		t.Fatalf("EncodeIntoChecked violated rollback on too-deep error: %v", dst)
	}

	valTooLarge := NewBytes(make([]byte, 1048572))
	err = EncodeIntoChecked(valTooLarge, &dst)
	if err == nil {
		t.Fatal("expected error on too large value")
	}
	if len(dst) != 3 || dst[0] != 0x01 || dst[1] != 0x02 || dst[2] != 0x03 {
		t.Fatalf("EncodeIntoChecked violated rollback on too-large error: %v", dst)
	}

	dupMap := NewMap([]MapEntry{
		{Key: NewUnsigned(1), Value: NewUnsigned(1)},
		{Key: NewUnsigned(1), Value: NewUnsigned(2)},
	})
	err = EncodeIntoChecked(dupMap, &dst)
	if err == nil {
		t.Fatal("expected error on duplicate map key")
	}
	if len(dst) != 3 || dst[0] != 0x01 || dst[1] != 0x02 || dst[2] != 0x03 {
		t.Fatalf("EncodeIntoChecked violated rollback on duplicate key: %v", dst)
	}

	// Verify nil dst returns error
	if err := EncodeIntoChecked(NewUnsigned(1), nil); err == nil {
		t.Fatal("expected error on nil dst")
	}
}

// ============================================================================
// 3. DecodeMapSpans Tests
// ============================================================================

func TestDecodeMapSpansValid(t *testing.T) {
	// Map with diverse entries:
	m := NewMap([]MapEntry{
		{Key: NewText("alpha"), Value: NewUnsigned(100)},
		{Key: NewText("beta"), Value: NewBytes([]byte{10, 20, 30})},
		{Key: NewText("gamma"), Value: NewArray([]Value{NewBool(true), Null()})},
	})

	encoded, err := EncodeChecked(m)
	if err != nil {
		t.Fatalf("EncodeChecked failed: %v", err)
	}

	spans, ok, err := DecodeMapSpans(encoded)
	if err != nil {
		t.Fatalf("DecodeMapSpans failed: %v", err)
	}
	if !ok {
		t.Fatalf("DecodeMapSpans returned ok=false for valid map")
	}
	if len(spans) != 3 {
		t.Fatalf("expected 3 entries, got %d", len(spans))
	}

	// Verify the invariant: encoded[entry.ValueSpan.Start:entry.ValueSpan.End] == EncodeChecked(entry.Value)
	for i, entry := range spans {
		spanBytes := encoded[entry.ValueSpan.Start:entry.ValueSpan.End]
		expectedValBytes, err := EncodeChecked(entry.Value)
		if err != nil {
			t.Fatalf("entry %d EncodeChecked failed: %v", i, err)
		}
		if !bytes.Equal(spanBytes, expectedValBytes) {
			t.Errorf("entry %d span mismatch: got %x, want %x", i, spanBytes, expectedValBytes)
		}
	}
}

func TestDecodeMapSpansPassThroughExceedingEncoderLimit(t *testing.T) {
	// CBR01 invariant: MaxEncodedBytes bounds the encoder only. The decoder has
	// no input-length limit. Inputs exist that decode and will not re-encode.
	// For those, b[s] is the value's canonical bytes even though EncodeChecked
	// on the outer map reports encode-too-large.
	//
	// Construct outer map wire bytes exceeding MaxEncodedBytes (1,048,576):
	// Map of 1 entry:
	// - Header: 0xA1 (major 5, length 1)
	// - Key: 0x00 (unsigned 0)
	// - Value: byte string of length 1,048,570 bytes (header 5 bytes + 1,048,570 payload = 1,048,575 bytes <= MaxEncodedBytes)
	// Outer map total wire bytes: 1 + 1 + 1,048,575 = 1,048,577 bytes (> 1,048,576).
	childPayloadLen := 1048570
	childValue := NewBytes(make([]byte, childPayloadLen))
	childEncoded, err := EncodeChecked(childValue)
	if err != nil {
		t.Fatalf("child EncodeChecked failed: %v", err)
	}
	if len(childEncoded) != 1048575 {
		t.Fatalf("unexpected child encoded len: %d", len(childEncoded))
	}

	var outerWire []byte
	outerWire = append(outerWire, 0xA1) // map of 1
	outerWire = append(outerWire, 0x00) // key: 0
	outerWire = append(outerWire, childEncoded...)

	if len(outerWire) != 1048577 {
		t.Fatalf("expected outer wire 1048577 bytes, got %d", len(outerWire))
	}

	// 1. DecodeMapSpans must succeed
	spans, ok, err := DecodeMapSpans(outerWire)
	if err != nil {
		t.Fatalf("DecodeMapSpans failed on >1MiB wire: %v", err)
	}
	if !ok || len(spans) != 1 {
		t.Fatalf("expected ok=true and 1 span, got ok=%v, count=%d", ok, len(spans))
	}

	// 2. EncodeChecked on outer map must fail with ErrEncodeTooLarge
	outerMap := NewMap([]MapEntry{
		{Key: spans[0].Key, Value: spans[0].Value},
	})
	_, err = EncodeChecked(outerMap)
	if err == nil {
		t.Fatal("expected EncodeChecked to fail with ErrEncodeTooLarge for outer map")
	}
	var cborErr *Error
	if !errors.As(err, &cborErr) || cborErr.Code != ErrEncodeTooLarge {
		t.Fatalf("expected ErrEncodeTooLarge, got %v", err)
	}

	// 3. But EncodeChecked on the child value succeeds, and b[s] == EncodeChecked(v)!
	childReEncoded, err := EncodeChecked(spans[0].Value)
	if err != nil {
		t.Fatalf("EncodeChecked on child failed: %v", err)
	}
	spanBytes := outerWire[spans[0].ValueSpan.Start:spans[0].ValueSpan.End]
	if !bytes.Equal(spanBytes, childReEncoded) {
		t.Fatalf("span bytes do not match EncodeChecked(child)")
	}
}

func TestDecodeMapSpansShapeMismatch(t *testing.T) {
	// Valid canonical CBOR that is not a map must return (nil, false, nil)
	nonMapValues := []Value{
		NewUnsigned(42),
		NewNegative(10),
		NewBytes([]byte{1, 2, 3}),
		NewText("hello"),
		NewArray([]Value{NewUnsigned(1), NewUnsigned(2)}),
		NewTag(1, NewUnsigned(99)),
		NewBool(true),
		Null(),
	}

	for i, v := range nonMapValues {
		enc, err := EncodeChecked(v)
		if err != nil {
			t.Fatalf("case %d EncodeChecked failed: %v", i, err)
		}
		spans, ok, err := DecodeMapSpans(enc)
		if err != nil {
			t.Errorf("case %d unexpected error: %v", i, err)
		}
		if ok {
			t.Errorf("case %d expected ok=false, got true", i)
		}
		if spans != nil {
			t.Errorf("case %d expected nil spans, got %v", i, spans)
		}
	}
}

func TestDecodeMapSpansRejections(t *testing.T) {
	// Empty data -> unexpected-eof
	spans, ok, err := DecodeMapSpans([]byte{})
	if err == nil || err.(*Error).ID() != "unexpected-eof" || ok || spans != nil {
		t.Fatalf("expected unexpected-eof for empty data, got ok=%v, err=%v", ok, err)
	}

	// Invalid non-map CBOR -> error (not shape mismatch!)
	// 0x18 0x05 (non-minimal integer 5)
	spans, ok, err = DecodeMapSpans([]byte{0x18, 0x05})
	if err == nil || err.(*Error).ID() != "non-minimal-integer" || ok {
		t.Fatalf("expected non-minimal-integer for malformed integer, got %v", err)
	}

	// Malformed map header: 0xBF (indefinite map)
	spans, ok, err = DecodeMapSpans([]byte{0xBF})
	if err == nil || err.(*Error).ID() != "indefinite" || ok {
		t.Fatalf("expected indefinite error, got %v", err)
	}

	// Out-of-order map keys
	outOfOrder := []byte{0xA2, 0x61, 0x62, 0x00, 0x61, 0x61, 0x01}
	spans, ok, err = DecodeMapSpans(outOfOrder)
	if err == nil || err.(*Error).ID() != "non-canonical-map-order" || ok {
		t.Fatalf("expected non-canonical-map-order, got %v", err)
	}

	// Trailing bytes after map
	validMap, _ := EncodeChecked(NewMap([]MapEntry{
		{Key: NewText("a"), Value: NewUnsigned(1)},
	}))
	withTrailing := append(validMap, 0x00)
	spans, ok, err = DecodeMapSpans(withTrailing)
	if err == nil || err.(*Error).ID() != "trailing-bytes" || ok {
		t.Fatalf("expected trailing-bytes, got %v", err)
	}
}

// ============================================================================
// 4. Property-Based & Model Equivalence Tests
// ============================================================================

// Pure deterministic LCG PRNG for reproducible property tests with zero external deps.
type testPRNG uint64

func (p *testPRNG) next() uint64 {
	*p = *p*6364136223846793005 + 1442695040888963407
	return uint64(*p)
}

func (p *testPRNG) nextN(n int) int {
	return int(p.next() % uint64(n))
}

func (p *testPRNG) randomValue(depth int) Value {
	if depth >= 5 {
		// Base cases at max depth
		switch p.nextN(4) {
		case 0:
			return NewUnsigned(p.next())
		case 1:
			return NewNegative(p.next())
		case 2:
			return NewBool(p.nextN(2) == 1)
		default:
			return Null()
		}
	}

	switch p.nextN(8) {
	case 0:
		return NewUnsigned(p.next())
	case 1:
		return NewNegative(p.next())
	case 2:
		bLen := p.nextN(16)
		b := make([]byte, bLen)
		for i := range b {
			b[i] = byte(p.next())
		}
		return NewBytes(b)
	case 3:
		tLen := p.nextN(10)
		chars := "abcdefghijklmnopqrstuvwxyz0123456789_-"
		sb := make([]byte, tLen)
		for i := range sb {
			sb[i] = chars[p.nextN(len(chars))]
		}
		return NewText(string(sb))
	case 4:
		arrLen := p.nextN(4)
		items := make([]Value, arrLen)
		for i := range items {
			items[i] = p.randomValue(depth + 1)
		}
		return NewArray(items)
	case 5:
		mapLen := p.nextN(4)
		entries := make([]MapEntry, mapLen)
		for i := range entries {
			// Distinct integer keys to prevent duplicate keys
			entries[i] = MapEntry{
				Key:   NewUnsigned(uint64(i * 100)),
				Value: p.randomValue(depth + 1),
			}
		}
		return NewMap(entries)
	case 6:
		return NewTag(p.next()%1000, p.randomValue(depth+1))
	default:
		if p.nextN(2) == 0 {
			return NewBool(p.nextN(2) == 1)
		}
		return Null()
	}
}

func TestPropertyRoundTripBijection(t *testing.T) {
	prng := testPRNG(1337)
	for i := 0; i < 200; i++ {
		val := prng.randomValue(0)
		enc, err := EncodeChecked(val)
		if err != nil {
			t.Fatalf("iteration %d: EncodeChecked failed: %v", i, err)
		}
		dec, err := Decode(enc)
		if err != nil {
			t.Fatalf("iteration %d: Decode failed: %v", i, err)
		}
		enc2, err := EncodeChecked(dec)
		if err != nil {
			t.Fatalf("iteration %d: EncodeChecked(Decode) failed: %v", i, err)
		}
		if !bytes.Equal(enc, enc2) {
			t.Fatalf("iteration %d: canonical bijection violation: enc1 != enc2", i)
		}
	}
}

func TestPropertyMapPermutationInvariance(t *testing.T) {
	// Permuting entries before NewMap MUST produce the exact same encoding
	entries := []MapEntry{
		{Key: NewText("first"), Value: NewUnsigned(1)},
		{Key: NewText("second"), Value: NewUnsigned(2)},
		{Key: NewText("third"), Value: NewUnsigned(3)},
		{Key: NewUnsigned(10), Value: NewText("ten")},
		{Key: NewBytes([]byte{1, 2}), Value: NewBool(true)},
	}

	m1 := NewMap(entries)
	enc1, err := EncodeChecked(m1)
	if err != nil {
		t.Fatalf("m1 encode failed: %v", err)
	}

	// Reverse order
	reversed := make([]MapEntry, len(entries))
	for i := range entries {
		reversed[i] = entries[len(entries)-1-i]
	}
	m2 := NewMap(reversed)
	enc2, err := EncodeChecked(m2)
	if err != nil {
		t.Fatalf("m2 encode failed: %v", err)
	}
	if !bytes.Equal(enc1, enc2) {
		t.Fatalf("map permutation produced different bytes: %x vs %x", enc1, enc2)
	}
}

func TestNonMinimalHeaderWideningRejection(t *testing.T) {
	// For each major type 0..6, taking an inline encoding (e.g. 5) and artificially
	// widening it to 1-byte (0x18), 2-byte (0x19), 4-byte (0x1A), 8-byte (0x1B) must
	// be strictly rejected with ErrNonMinimalInteger.
	for major := byte(0); major <= 6; major++ {
		mt := major << 5

		// 1-byte form for value 5: (mt | 24), 0x05
		w1 := []byte{mt | 24, 0x05}
		if major == 2 || major == 3 {
			w1 = append(w1, []byte("12345")...)
		} else if major == 4 {
			w1 = append(w1, []byte{0x00, 0x00, 0x00, 0x00, 0x00}...)
		} else if major == 5 {
			w1 = append(w1, []byte{0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x04, 0x04, 0x05, 0x05}...)
		} else if major == 6 {
			w1 = append(w1, 0x00) // tagged value
		}

		_, err := Decode(w1)
		if err == nil || err.(*Error).ID() != "non-minimal-integer" {
			t.Errorf("major %d 1-byte widening: expected non-minimal-integer, got %v", major, err)
		}

		// 2-byte form for value 200: (mt | 25), 0x00, 0xC8
		w2 := []byte{mt | 25, 0x00, 0xC8}
		if major == 6 {
			w2 = append(w2, 0x00)
		}
		_, err = Decode(w2)
		if err == nil || err.(*Error).ID() != "non-minimal-integer" {
			t.Errorf("major %d 2-byte widening: expected non-minimal-integer, got %v", major, err)
		}

		// 4-byte form for value 50000: (mt | 26), 0x00, 0x00, 0xC3, 0x50
		w4 := []byte{mt | 26, 0x00, 0x00, 0xC3, 0x50}
		if major == 6 {
			w4 = append(w4, 0x00)
		}
		_, err = Decode(w4)
		if err == nil || err.(*Error).ID() != "non-minimal-integer" {
			t.Errorf("major %d 4-byte widening: expected non-minimal-integer, got %v", major, err)
		}

		// 8-byte form for value 5000000: (mt | 27), 0x00, 0x00, 0x00, 0x00, 0x00, 0x4C, 0x4B, 0x40
		w8 := []byte{mt | 27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4C, 0x4B, 0x40}
		if major == 6 {
			w8 = append(w8, 0x00)
		}
		_, err = Decode(w8)
		if err == nil || err.(*Error).ID() != "non-minimal-integer" {
			t.Errorf("major %d 8-byte widening: expected non-minimal-integer, got %v", major, err)
		}
	}
}

// ============================================================================
// 5. Adversarial Fuzzing
// ============================================================================

func FuzzDecode(f *testing.F) {
	// Seed with corpus seeds
	f.Add([]byte{})
	f.Add([]byte{0x00})
	f.Add([]byte{0x17})
	f.Add([]byte{0x18, 0x18})
	f.Add([]byte{0x20})
	f.Add([]byte{0x40})
	f.Add([]byte{0x60})
	f.Add([]byte{0x80})
	f.Add([]byte{0xA0})
	f.Add([]byte{0xC0, 0x00})
	f.Add([]byte{0xF4})
	f.Add([]byte{0xF5})
	f.Add([]byte{0xF6})

	f.Fuzz(func(t *testing.T, data []byte) {
		val, err := Decode(data)
		if err != nil {
			// Expected rejection for arbitrary fuzzed bytes
			cborErr, ok := err.(*Error)
			if !ok {
				t.Fatalf("Decode error is not *Error: %T", err)
			}
			if !strings.HasPrefix(cborErr.Error(), "canonical-cbor: ") {
				t.Fatalf("error message missing prefix: %q", cborErr.Error())
			}
			return
		}

		// If data successfully decoded:
		if len(data) > MaxEncodedBytes {
			_, err := EncodeChecked(val)
			if err == nil {
				t.Fatalf("expected ErrEncodeTooLarge for input > MaxEncodedBytes")
			}
			var cborErr *Error
			if !errors.As(err, &cborErr) || cborErr.Code != ErrEncodeTooLarge {
				t.Fatalf("expected ErrEncodeTooLarge, got %v", err)
			}
			return
		}

		enc, err := EncodeChecked(val)
		if err != nil {
			t.Fatalf("Decode succeeded but EncodeChecked failed: %v", err)
		}
		if !bytes.Equal(enc, data) {
			t.Fatalf("canonical encoding drift: got %x, expected %x", enc, data)
		}
		val2, err := Decode(enc)
		if err != nil {
			t.Fatalf("re-decode failed: %v", err)
		}
		if !val.Equal(val2) {
			t.Fatalf("re-decoded value not structurally equal")
		}
	})
}

func FuzzEncodeDecodeBijection(f *testing.F) {
	f.Add(uint64(0), "hello", []byte{1, 2, 3})
	f.Add(uint64(100), "test-key", []byte{})
	f.Add(uint64(math.MaxUint64), "", []byte{0xFF, 0xFE})

	f.Fuzz(func(t *testing.T, num uint64, txt string, raw []byte) {
		if !utf8.ValidString(txt) {
			_, err := EncodeChecked(NewText(txt))
			if err == nil {
				t.Fatalf("expected ErrInvalidUtf8 for invalid string: %q", txt)
			}
			var cborErr *Error
			if !errors.As(err, &cborErr) || cborErr.Code != ErrInvalidUtf8 {
				t.Fatalf("expected ErrInvalidUtf8, got %v", err)
			}
			if !strings.HasPrefix(err.Error(), "canonical-cbor: ") {
				t.Fatalf("expected canonical-cbor: prefix, got %v", err.Error())
			}
			return
		}
		m := NewMap([]MapEntry{
			{Key: NewText(txt), Value: NewUnsigned(num)},
			{Key: NewBytes(raw), Value: Null()},
		})

		enc, err := EncodeChecked(m)
		if err != nil {
			// May fail on duplicate keys if text and bytes encode identically
			return
		}

		dec, err := Decode(enc)
		if err != nil {
			t.Fatalf("failed to decode validly encoded map: %v", err)
		}
		enc2, err := EncodeChecked(dec)
		if err != nil {
			t.Fatalf("failed to re-encode decoded map: %v", err)
		}
		if !bytes.Equal(enc, enc2) {
			t.Fatalf("canonical byte equality violated: %x vs %x", enc, enc2)
		}
	})
}

// ============================================================================
// 6. Additional Coverage Tests for 100% Branch Coverage
// ============================================================================

func TestAccessorsNilBranches(t *testing.T) {
	vBytes := Value{kind: KindBytes, bytes: nil}
	if vBytes.Bytes() != nil {
		t.Fatalf("expected nil bytes")
	}

	vArray := Value{kind: KindArray, array: nil}
	if vArray.Array() != nil {
		t.Fatalf("expected nil array")
	}

	vMap := Value{kind: KindMap, entries: nil}
	if vMap.Map() != nil {
		t.Fatalf("expected nil map")
	}

	vTag := Value{kind: KindTag, tag: 5, tagVal: nil}
	num, inner := vTag.Tag()
	if num != 5 || !inner.IsNull() {
		t.Fatalf("expected null inner on nil tagVal")
	}

	// Tag encoding with nil tagVal encodes Null
	enc, err := EncodeChecked(vTag)
	if err != nil {
		t.Fatalf("vTag encode failed: %v", err)
	}
	dec, err := Decode(enc)
	if err != nil {
		t.Fatalf("vTag decode failed: %v", err)
	}
	decNum, decInner := dec.Tag()
	if decNum != 5 || !decInner.IsNull() {
		t.Fatalf("decTag mismatch")
	}
}

func TestEqualNilTagAndFallback(t *testing.T) {
	tagNil1 := Value{kind: KindTag, tag: 1, tagVal: nil}
	tagNil2 := Value{kind: KindTag, tag: 1, tagVal: nil}
	tagNonNil := NewTag(1, NewUnsigned(10))

	if !tagNil1.Equal(tagNil2) {
		t.Errorf("tagNil1 should equal tagNil2")
	}
	if tagNil1.Equal(tagNonNil) {
		t.Errorf("tagNil1 should not equal tagNonNil")
	}
	if tagNonNil.Equal(tagNil1) {
		t.Errorf("tagNonNil should not equal tagNil1")
	}

	// Unknown kind fallback
	badKind1 := Value{kind: Kind(999)}
	badKind2 := Value{kind: Kind(999)}
	if badKind1.Equal(badKind2) {
		t.Errorf("unknown kind Equal should return false")
	}
}

func TestErrorIsWithNonError(t *testing.T) {
	err := &Error{Code: ErrUnexpectedEof}
	if err.Is(errors.New("other")) {
		t.Errorf("err.Is should return false for non-*Error")
	}
}

func TestEncodeSubItemErrors(t *testing.T) {
	// Nested array error propagation
	badItem := buildNestedArray(129)
	arr := NewArray([]Value{badItem})
	_, err := EncodeChecked(arr)
	if err == nil || err.(*Error).ID() != "encode-too-deep" {
		t.Fatalf("expected encode-too-deep from array item, got %v", err)
	}

	// Nested map key error propagation
	badMapKey := NewMap([]MapEntry{
		{Key: badItem, Value: NewUnsigned(1)},
	})
	_, err = EncodeChecked(badMapKey)
	if err == nil || err.(*Error).ID() != "encode-too-deep" {
		t.Fatalf("expected encode-too-deep from map key, got %v", err)
	}

	// Nested map value error propagation
	badMapVal := NewMap([]MapEntry{
		{Key: NewUnsigned(1), Value: badItem},
	})
	_, err = EncodeChecked(badMapVal)
	if err == nil || err.(*Error).ID() != "encode-too-deep" {
		t.Fatalf("expected encode-too-deep from map value, got %v", err)
	}

	// Nested tag error propagation
	badTag := NewTag(1, badItem)
	_, err = EncodeChecked(badTag)
	if err == nil || err.(*Error).ID() != "encode-too-deep" {
		t.Fatalf("expected encode-too-deep from tag, got %v", err)
	}

	// Unknown kind error
	badKind := Value{kind: Kind(999)}
	_, err = EncodeChecked(badKind)
	if err == nil || err.(*Error).ID() != "unsupported-simple" {
		t.Fatalf("expected unsupported-simple for unknown kind, got %v", err)
	}
}

func TestTruncatedHeadersAndPayloads(t *testing.T) {
	truncatedCases := []struct {
		name        string
		wire        []byte
		expectedErr string
	}{
		{"2-byte header truncated", []byte{0x19, 0x01}, "unexpected-eof"},
		{"4-byte header truncated", []byte{0x1A, 0x01, 0x02, 0x03}, "unexpected-eof"},
		{"8-byte header truncated", []byte{0x1B, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07}, "unexpected-eof"},
		{"bytes payload truncated", []byte{0x42, 0xAA}, "length-too-large"},
		{"text payload truncated", []byte{0x62, 0x61}, "length-too-large"},
		{"array payload truncated", []byte{0x82, 0x00}, "length-too-large"},
		{"map payload truncated", []byte{0xA2, 0x00, 0x01}, "length-too-large"},
		{"tag inner truncated", []byte{0xC0}, "unexpected-eof"},
		{"tag inner truncated argument", []byte{0xD8, 0x18}, "unexpected-eof"},
		{"simple 24 truncated", []byte{0xF8}, "unexpected-eof"},
	}

	for _, tc := range truncatedCases {
		t.Run(tc.name, func(t *testing.T) {
			_, err := Decode(tc.wire)
			if err == nil || err.(*Error).ID() != tc.expectedErr {
				t.Fatalf("expected %s for %s, got %v", tc.expectedErr, tc.name, err)
			}
		})
	}
}

func TestDecodeMapSpansMoreErrors(t *testing.T) {
	// Truncated map entry key
	_, ok, err := DecodeMapSpans([]byte{0xA1, 0x19, 0x01})
	if err == nil || err.(*Error).ID() != "unexpected-eof" || ok {
		t.Fatalf("expected unexpected-eof for truncated key, got %v", err)
	}

	// Truncated map entry val
	_, ok, err = DecodeMapSpans([]byte{0xA1, 0x00, 0x19, 0x01})
	if err == nil || err.(*Error).ID() != "unexpected-eof" || ok {
		t.Fatalf("expected unexpected-eof for truncated val, got %v", err)
	}

	// Bad map key (non-minimal)
	_, ok, err = DecodeMapSpans([]byte{0xA1, 0x18, 0x00, 0x01})
	if err == nil || err.(*Error).ID() != "non-minimal-integer" || ok {
		t.Fatalf("expected non-minimal-integer, got %v", err)
	}

	// Bad map val (non-minimal)
	_, ok, err = DecodeMapSpans([]byte{0xA1, 0x00, 0x18, 0x00})
	if err == nil || err.(*Error).ID() != "non-minimal-integer" || ok {
		t.Fatalf("expected non-minimal-integer, got %v", err)
	}

	// Hostile map length
	_, ok, err = DecodeMapSpans([]byte{0xBB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF})
	if err == nil || err.(*Error).ID() != "length-too-large" || ok {
		t.Fatalf("expected length-too-large, got %v", err)
	}
}

func TestReadValueElementErrors(t *testing.T) {
	// Array containing invalid child (non-minimal integer 5)
	badArrayChild := []byte{0x81, 0x18, 0x05}
	_, err := Decode(badArrayChild)
	if err == nil || err.(*Error).ID() != "non-minimal-integer" {
		t.Fatalf("expected non-minimal-integer in array child, got %v", err)
	}

	// Map containing invalid child value
	badMapChildVal := []byte{0xA1, 0x00, 0x18, 0x05}
	_, err = Decode(badMapChildVal)
	if err == nil || err.(*Error).ID() != "non-minimal-integer" {
		t.Fatalf("expected non-minimal-integer in map child, got %v", err)
	}
}

func TestEncodeBufferLimitHitsPushByte(t *testing.T) {
	// Array with 1,048,576 items hits pushByteChecked limit during item emission
	items := make([]Value, MaxEncodedBytes)
	for i := range items {
		items[i] = Null()
	}
	arr := NewArray(items)
	_, err := EncodeChecked(arr)
	if err == nil || err.(*Error).ID() != "encode-too-large" {
		t.Fatalf("expected encode-too-large, got %v", err)
	}
}

// ============================================================================
// 7. CVE Resilience Showcase
// ============================================================================
//
// This test suite explicitly showcases and validates that canonicalcbor is
// immune to the seven primary vulnerability classes associated with binary
// serialization and CBOR decoders:
//
//   1. Integer Overflow / Wraparound in Length & Offset Math
//   2. Out-Of-Memory (OOM) / Giant-Length Allocation Denial-of-Service
//   3. Stack-Overflow / Call-Depth Exhaustion via Deeply Nested Structures
//   4. Canonical Invariance / Key Malleability / Signature Bypass Attacks
//   5. UTF-8 Smuggling (Overlong Encodings & Surrogate Halves)
//   6. Trailing-Byte Smuggling & Stream Confusion
//   7. Secret & Key Material Leakage via Diagnostic Errors

func TestCVEResilienceShowcase(t *testing.T) {
	t.Run("CVE-1: Integer overflow and wraparound in length math", func(t *testing.T) {
		// Attack vectors: lengths near 2^64-1 designed to wrap around 64-bit addition
		// or bypass cursor bounds checks (c.pos + n).
		overflowVectors := [][]byte{
			// Major 2 (bytes) with length 2^64 - 1
			{0x5B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF},
			// Major 3 (text) with length 2^64 - 1
			{0x7B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF},
			// Major 4 (array) with length 2^64 - 1
			{0x9B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF},
			// Major 5 (map) with length 2^64 - 1 (would overflow count * 2)
			{0xBB, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF},
			// Major 5 (map) with count = math.MaxInt / 2 + 1
			{0xBA, 0x40, 0x00, 0x00, 0x01},
		}

		for i, vec := range overflowVectors {
			_, err := Decode(vec)
			if err == nil {
				t.Fatalf("vector %d: expected length error, got success", i)
			}
			cborErr, ok := err.(*Error)
			if !ok || cborErr.ID() != "length-too-large" {
				t.Fatalf("vector %d: expected length-too-large, got %v", i, err)
			}
		}
	})

	t.Run("CVE-2: OOM giant-length allocation denial-of-service", func(t *testing.T) {
		// Attack vectors: small input buffer declaring billions of elements to force
		// an uncontrolled heap allocation (e.g. make([]Value, 10^9)).
		// The decoder MUST check declared count against remaining bytes BEFORE allocating.
		oomVectors := []struct {
			name string
			wire []byte
		}{
			{"1-billion-element array in 5 bytes", []byte{0x9A, 0x3B, 0x9A, 0xCA, 0x00}},
			{"1-billion-entry map in 5 bytes", []byte{0xBA, 0x3B, 0x9A, 0xCA, 0x00}},
			{"100-million-byte string in 5 bytes", []byte{0x5A, 0x05, 0xF5, 0xE1, 0x00}},
			{"100-million-char text in 5 bytes", []byte{0x7A, 0x05, 0xF5, 0xE1, 0x00}},
		}

		for _, tc := range oomVectors {
			t.Run(tc.name, func(t *testing.T) {
				allocs := testing.AllocsPerRun(10, func() {
					_, err := Decode(tc.wire)
					if err == nil || err.(*Error).ID() != "length-too-large" {
						t.Fatalf("expected length-too-large, got %v", err)
					}
				})
				if allocs > 4 {
					t.Fatalf("heap allocation detected during OOM rejection: got %.1f allocs/run, want <= 4", allocs)
				}
			})
		}
	})

	t.Run("CVE-3: Stack-overflow via deep nesting", func(t *testing.T) {
		// Attack vectors: chains of singleton arrays, maps, or tags designed to blow the OS stack.
		// The decoder and encoder enforce MaxNestingDepth = 128.
		attackWire := buildNestedArrayWire(129)
		_, err := Decode(attackWire)
		if err == nil || err.(*Error).ID() != "too-deep" {
			t.Fatalf("expected too-deep on decode, got %v", err)
		}

		attackVal := buildNestedArray(129)
		_, err = EncodeChecked(attackVal)
		if err == nil || err.(*Error).ID() != "encode-too-deep" {
			t.Fatalf("expected encode-too-deep on encode, got %v", err)
		}
	})

	t.Run("CVE-4: Non-canonical encoding and duplicate-key signature bypass", func(t *testing.T) {
		// Attack vectors: submitting non-minimal integers, out-of-order map keys, or duplicate keys
		// to bypass digital signatures or create ambiguous interpretations across microservices.
		// 1. Widened integer (0 encoded as 0x18 0x00)
		_, err := Decode([]byte{0x18, 0x00})
		if err == nil || err.(*Error).ID() != "non-minimal-integer" {
			t.Fatalf("expected non-minimal-integer, got %v", err)
		}

		// 2. Out-of-order map keys {"b": 1, "a": 2}
		outOfOrder := []byte{0xA2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02}
		_, err = Decode(outOfOrder)
		if err == nil || err.(*Error).ID() != "non-canonical-map-order" {
			t.Fatalf("expected non-canonical-map-order, got %v", err)
		}

		// 3. Duplicate map keys {"a": 1, "a": 2}
		dupMapWire := []byte{0xA2, 0x61, 0x61, 0x01, 0x61, 0x61, 0x02}
		_, err = Decode(dupMapWire)
		if err == nil || err.(*Error).ID() != "non-canonical-map-order" {
			t.Fatalf("expected non-canonical-map-order for duplicate keys, got %v", err)
		}

		// 4. Duplicate map keys rejected on encode
		dupMapVal := NewMap([]MapEntry{
			{Key: NewText("admin"), Value: NewBool(false)},
			{Key: NewText("admin"), Value: NewBool(true)},
		})
		_, err = EncodeChecked(dupMapVal)
		if err == nil || err.(*Error).ID() != "duplicate-map-key" {
			t.Fatalf("expected duplicate-map-key, got %v", err)
		}
	})

	t.Run("CVE-5: UTF-8 smuggling via overlong and surrogate encodings", func(t *testing.T) {
		// Attack vectors: overlong NUL or slash to bypass path traversal filters or WAF rules.
		smugglingVectors := []struct {
			name string
			wire []byte
		}{
			{"Overlong 2-byte NUL", []byte{0x62, 0xC0, 0x80}},
			{"Overlong 3-byte slash", []byte{0x63, 0xE0, 0x80, 0xAF}},
			{"Overlong 4-byte slash", []byte{0x64, 0xF0, 0x80, 0x80, 0xAF}},
			{"Surrogate U+D800", []byte{0x63, 0xED, 0xA0, 0x80}},
			{"Surrogate U+DFFF", []byte{0x63, 0xED, 0xBF, 0xBF}},
		}

		for _, tc := range smugglingVectors {
			t.Run(tc.name, func(t *testing.T) {
				_, err := Decode(tc.wire)
				if err == nil || err.(*Error).ID() != "invalid-utf8" {
					t.Fatalf("expected invalid-utf8 for %s, got %v", tc.name, err)
				}
			})
		}

		// Ensure encoder strictly rejects invalid UTF-8 strings with atomic rollback:
		invalidStrings := []struct {
			name string
			str  string
		}{
			{"Overlong 2-byte NUL", "\xC0\x80"},
			{"Overlong 3-byte slash", "\xE0\x80\xAF"},
			{"Surrogate U+D800", "\xED\xA0\x80"},
			{"Surrogate U+DFFF", "\xED\xBF\xBF"},
			{"Raw 0xFF byte", "\xFF"},
		}
		for _, tc := range invalidStrings {
			t.Run("Encode-"+tc.name, func(t *testing.T) {
				_, err := EncodeChecked(NewText(tc.str))
				if err == nil || err.(*Error).ID() != "invalid-utf8" {
					t.Fatalf("expected invalid-utf8 on EncodeChecked for %s, got %v", tc.name, err)
				}
				dst := []byte{0xDE, 0xAD}
				err = EncodeIntoChecked(NewText(tc.str), &dst)
				if err == nil || err.(*Error).ID() != "invalid-utf8" {
					t.Fatalf("expected invalid-utf8 on EncodeIntoChecked for %s, got %v", tc.name, err)
				}
				if !bytes.Equal(dst, []byte{0xDE, 0xAD}) {
					t.Fatalf("atomic rollback violated on invalid UTF-8: dst was modified to %x", dst)
				}
			})
		}
	})

	t.Run("CVE-6: Trailing-byte smuggling and stream confusion", func(t *testing.T) {
		// Attack vectors: appending extra commands or records after a valid item.
		// The decoder must strictly fail with trailing-bytes.
		validItem := []byte{0x01} // unsigned 1
		smuggled := append(validItem, []byte{0xDE, 0xAD, 0xBE, 0xEF}...)

		_, err := Decode(smuggled)
		if err == nil || err.(*Error).ID() != "trailing-bytes" {
			t.Fatalf("expected trailing-bytes, got %v", err)
		}
	})

	t.Run("CVE-7: Secret and payload leakage in error diagnostics", func(t *testing.T) {
		// Attack vectors: injecting confidential passwords or tokens into malformed payloads
		// hoping error messages reflect input strings into logs or monitoring pipelines.
		confidentialToken := "SUPER_SECRET_AUTHENTICATION_KEY_12345"

		// Construct an invalid value containing the secret key in duplicate entries
		badMap := NewMap([]MapEntry{
			{Key: NewText(confidentialToken), Value: NewUnsigned(1)},
			{Key: NewText(confidentialToken), Value: NewUnsigned(2)},
		})

		_, err := EncodeChecked(badMap)
		if err == nil {
			t.Fatal("expected encode error")
		}

		errMsg := err.Error()
		if strings.Contains(errMsg, confidentialToken) {
			t.Fatalf("CRITICAL SECURITY FLAW: confidential token was leaked in error message: %q", errMsg)
		}
		if !strings.HasPrefix(errMsg, "canonical-cbor: ") {
			t.Fatalf("error message missing standard prefix: %q", errMsg)
		}

		// Also verify SpannedMapEntry formatting redacts secret key and value payloads:
		spanned := SpannedMapEntry{
			Key:       NewText(confidentialToken),
			Value:     NewText("SECRET_VALUE_99999"),
			ValueSpan: Span{Start: 2, End: 10},
		}
		strRep := fmt.Sprintf("%v", spanned)
		goStrRep := fmt.Sprintf("%#v", spanned)
		if strings.Contains(strRep, confidentialToken) || strings.Contains(strRep, "SECRET_VALUE_99999") {
			t.Fatalf("SpannedMapEntry.String() leaked secret material: %s", strRep)
		}
		if strings.Contains(goStrRep, confidentialToken) || strings.Contains(goStrRep, "SECRET_VALUE_99999") {
			t.Fatalf("SpannedMapEntry.GoString() leaked secret material: %s", goStrRep)
		}
	})
}
