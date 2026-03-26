package wasmleb128

// Tests for the wasmleb128 package.
//
// We cover all 11 required test cases:
//   1.  Zero
//   2.  One-byte unsigned (value=3)
//   3.  One-byte signed negative (0x7E → -2)
//   4.  Multi-byte unsigned ([0xE5, 0x8E, 0x26] → 624485)
//   5.  Max u32 (4294967295)
//   6.  Max i32 (2147483647)
//   7.  Min i32 (-2147483648)
//   8.  Round-trip: encode then decode returns original
//   9.  Unterminated input → LEB128Error
//  10.  Non-zero offset
//  11.  Encode/decode of negative values (signed)

import (
	"testing"
)

// ---------------------------------------------------------------------------
// Package loads (kept from scaffold)
// ---------------------------------------------------------------------------

func TestPackageLoads(t *testing.T) {
	t.Log("wasm-leb128 package loaded successfully")
}

// ---------------------------------------------------------------------------
// Test case 1 — Zero
// ---------------------------------------------------------------------------

func TestDecodeUnsignedZero(t *testing.T) {
	value, consumed, err := DecodeUnsigned([]byte{0x00}, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 0 {
		t.Errorf("expected 0, got %d", value)
	}
	if consumed != 1 {
		t.Errorf("expected 1 byte consumed, got %d", consumed)
	}
}

func TestDecodeSignedZero(t *testing.T) {
	value, consumed, err := DecodeSigned([]byte{0x00}, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 0 {
		t.Errorf("expected 0, got %d", value)
	}
	if consumed != 1 {
		t.Errorf("expected 1 byte consumed, got %d", consumed)
	}
}

func TestEncodeUnsignedZero(t *testing.T) {
	encoded := EncodeUnsigned(0)
	if len(encoded) != 1 || encoded[0] != 0x00 {
		t.Errorf("expected [0x00], got %v", encoded)
	}
}

func TestEncodeSignedZero(t *testing.T) {
	encoded := EncodeSigned(0)
	if len(encoded) != 1 || encoded[0] != 0x00 {
		t.Errorf("expected [0x00], got %v", encoded)
	}
}

// ---------------------------------------------------------------------------
// Test case 2 — One-byte unsigned (value = 3)
// ---------------------------------------------------------------------------

func TestDecodeUnsignedOneByte(t *testing.T) {
	// 0x03 = 3; MSB=0 → single byte, no continuation.
	value, consumed, err := DecodeUnsigned([]byte{0x03}, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 3 {
		t.Errorf("expected 3, got %d", value)
	}
	if consumed != 1 {
		t.Errorf("expected 1 byte consumed, got %d", consumed)
	}
}

func TestDecodeUnsigned127(t *testing.T) {
	// 0x7F = 127; largest one-byte unsigned LEB128 value.
	value, consumed, err := DecodeUnsigned([]byte{0x7F}, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 127 {
		t.Errorf("expected 127, got %d", value)
	}
	if consumed != 1 {
		t.Errorf("expected 1 byte consumed, got %d", consumed)
	}
}

func TestEncodeUnsigned3(t *testing.T) {
	encoded := EncodeUnsigned(3)
	if len(encoded) != 1 || encoded[0] != 0x03 {
		t.Errorf("expected [0x03], got %v", encoded)
	}
}

// ---------------------------------------------------------------------------
// Test case 3 — One-byte signed negative (0x7E → -2)
// ---------------------------------------------------------------------------

func TestDecodeSignedOneByte(t *testing.T) {
	// 0x7E = 0b0111_1110
	// MSB=0 → last byte; bit 6 = 1 → negative → sign extend
	// result = 0b111_1110 | -(1<<7) = 126 | -128 = -2
	value, consumed, err := DecodeSigned([]byte{0x7E}, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != -2 {
		t.Errorf("expected -2, got %d", value)
	}
	if consumed != 1 {
		t.Errorf("expected 1 byte consumed, got %d", consumed)
	}
}

func TestDecodeSignedMinusOne(t *testing.T) {
	// 0x7F = 0b0111_1111; bit 6 = 1 → -1
	value, _, err := DecodeSigned([]byte{0x7F}, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != -1 {
		t.Errorf("expected -1, got %d", value)
	}
}

func TestEncodeSignedMinusTwo(t *testing.T) {
	encoded := EncodeSigned(-2)
	if len(encoded) != 1 || encoded[0] != 0x7E {
		t.Errorf("expected [0x7E], got %v", encoded)
	}
}

func TestEncodeSignedMinusOne(t *testing.T) {
	encoded := EncodeSigned(-1)
	if len(encoded) != 1 || encoded[0] != 0x7F {
		t.Errorf("expected [0x7F], got %v", encoded)
	}
}

// ---------------------------------------------------------------------------
// Test case 4 — Multi-byte: [0xE5, 0x8E, 0x26] → 624485
// ---------------------------------------------------------------------------

func TestDecodeUnsignedMultiByte(t *testing.T) {
	// 624485 = 0x98765
	// Group 1 (bits 0-6):  0x65 | 0x80 = 0xE5
	// Group 2 (bits 7-13): 0x0E | 0x80 = 0x8E
	// Group 3 (bits 14-20):0x26         = 0x26
	data := []byte{0xE5, 0x8E, 0x26}
	value, consumed, err := DecodeUnsigned(data, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 624485 {
		t.Errorf("expected 624485, got %d", value)
	}
	if consumed != 3 {
		t.Errorf("expected 3 bytes consumed, got %d", consumed)
	}
}

func TestDecodeSignedMultiByte(t *testing.T) {
	// 624485 is positive; signed and unsigned should agree.
	data := []byte{0xE5, 0x8E, 0x26}
	value, consumed, err := DecodeSigned(data, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 624485 {
		t.Errorf("expected 624485, got %d", value)
	}
	if consumed != 3 {
		t.Errorf("expected 3 bytes consumed, got %d", consumed)
	}
}

func TestDecodeUnsigned128TwoByte(t *testing.T) {
	// 128 requires 2 bytes: [0x80, 0x01]
	// 0x80 = 1_0000000 (continuation, payload=0)
	// 0x01 = 0_0000001 (last, payload=1)
	// value = 0 | (1 << 7) = 128
	value, consumed, err := DecodeUnsigned([]byte{0x80, 0x01}, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 128 {
		t.Errorf("expected 128, got %d", value)
	}
	if consumed != 2 {
		t.Errorf("expected 2 bytes consumed, got %d", consumed)
	}
}

func TestEncodeUnsigned128(t *testing.T) {
	encoded := EncodeUnsigned(128)
	expected := []byte{0x80, 0x01}
	if len(encoded) != len(expected) {
		t.Fatalf("expected len %d, got %d: %v", len(expected), len(encoded), encoded)
	}
	for i, b := range expected {
		if encoded[i] != b {
			t.Errorf("byte %d: expected 0x%02X, got 0x%02X", i, b, encoded[i])
		}
	}
}

// ---------------------------------------------------------------------------
// Test case 5 — Max u32: 4294967295
// ---------------------------------------------------------------------------

func TestDecodeUnsignedMaxU32(t *testing.T) {
	// 0xFFFFFFFF = 4294967295
	// 5-byte encoding: [0xFF, 0xFF, 0xFF, 0xFF, 0x0F]
	// Last byte 0x0F contributes bits 28–31 = 0b1111
	data := []byte{0xFF, 0xFF, 0xFF, 0xFF, 0x0F}
	value, consumed, err := DecodeUnsigned(data, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 4294967295 {
		t.Errorf("expected 4294967295, got %d", value)
	}
	if consumed != 5 {
		t.Errorf("expected 5 bytes consumed, got %d", consumed)
	}
}

func TestEncodeUnsignedMaxU32(t *testing.T) {
	encoded := EncodeUnsigned(4294967295)
	expected := []byte{0xFF, 0xFF, 0xFF, 0xFF, 0x0F}
	if len(encoded) != len(expected) {
		t.Fatalf("expected len %d, got %d", len(expected), len(encoded))
	}
	for i, b := range expected {
		if encoded[i] != b {
			t.Errorf("byte %d: expected 0x%02X, got 0x%02X", i, b, encoded[i])
		}
	}
}

// ---------------------------------------------------------------------------
// Test case 6 — Max i32: 2147483647
// ---------------------------------------------------------------------------

func TestDecodeSignedMaxI32(t *testing.T) {
	// 2147483647 = 0x7FFFFFFF
	// Encoding: [0xFF, 0xFF, 0xFF, 0xFF, 0x07]
	// Last byte 0x07 = 0b0000_0111: bit 6 = 0 → positive ✓
	data := []byte{0xFF, 0xFF, 0xFF, 0xFF, 0x07}
	value, consumed, err := DecodeSigned(data, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 2147483647 {
		t.Errorf("expected 2147483647, got %d", value)
	}
	if consumed != 5 {
		t.Errorf("expected 5 bytes consumed, got %d", consumed)
	}
}

func TestEncodeSignedMaxI32(t *testing.T) {
	encoded := EncodeSigned(2147483647)
	expected := []byte{0xFF, 0xFF, 0xFF, 0xFF, 0x07}
	if len(encoded) != len(expected) {
		t.Fatalf("expected len %d, got %d: %v", len(expected), len(encoded), encoded)
	}
	for i, b := range expected {
		if encoded[i] != b {
			t.Errorf("byte %d: expected 0x%02X, got 0x%02X", i, b, encoded[i])
		}
	}
}

// ---------------------------------------------------------------------------
// Test case 7 — Min i32: -2147483648
// ---------------------------------------------------------------------------

func TestDecodeSignedMinI32(t *testing.T) {
	// -2147483648 = -2^31
	// Encoding: [0x80, 0x80, 0x80, 0x80, 0x78]
	// Last byte 0x78 = 0b0111_1000: bit 6 = 1 → negative → sign extend ✓
	data := []byte{0x80, 0x80, 0x80, 0x80, 0x78}
	value, consumed, err := DecodeSigned(data, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != -2147483648 {
		t.Errorf("expected -2147483648, got %d", value)
	}
	if consumed != 5 {
		t.Errorf("expected 5 bytes consumed, got %d", consumed)
	}
}

func TestEncodeSignedMinI32(t *testing.T) {
	encoded := EncodeSigned(-2147483648)
	expected := []byte{0x80, 0x80, 0x80, 0x80, 0x78}
	if len(encoded) != len(expected) {
		t.Fatalf("expected len %d, got %d: %v", len(expected), len(encoded), encoded)
	}
	for i, b := range expected {
		if encoded[i] != b {
			t.Errorf("byte %d: expected 0x%02X, got 0x%02X", i, b, encoded[i])
		}
	}
}

// ---------------------------------------------------------------------------
// Test case 8 — Round-trip: encode then decode returns original value
// ---------------------------------------------------------------------------

func TestRoundTripUnsigned(t *testing.T) {
	values := []uint64{
		0, 1, 63, 64, 127, 128, 255, 256,
		16383, 16384, 2097151, 268435455, 4294967295,
	}

	for _, v := range values {
		encoded := EncodeUnsigned(v)
		decoded, consumed, err := DecodeUnsigned(encoded, 0)
		if err != nil {
			t.Errorf("RoundTrip unsigned %d: unexpected error: %v", v, err)
			continue
		}
		if decoded != v {
			t.Errorf("RoundTrip unsigned %d: got %d (encoded: %v)", v, decoded, encoded)
		}
		if consumed != len(encoded) {
			t.Errorf("RoundTrip unsigned %d: consumed %d of %d bytes", v, consumed, len(encoded))
		}
	}
}

func TestRoundTripSigned(t *testing.T) {
	values := []int64{
		0, 1, -1, 63, 64, -64, -65, 127, -128,
		2147483647, -2147483648, 100, -100, 1000, -1000,
	}

	for _, v := range values {
		encoded := EncodeSigned(v)
		decoded, consumed, err := DecodeSigned(encoded, 0)
		if err != nil {
			t.Errorf("RoundTrip signed %d: unexpected error: %v", v, err)
			continue
		}
		if decoded != v {
			t.Errorf("RoundTrip signed %d: got %d (encoded: %v)", v, decoded, encoded)
		}
		if consumed != len(encoded) {
			t.Errorf("RoundTrip signed %d: consumed %d of %d bytes", v, consumed, len(encoded))
		}
	}
}

// ---------------------------------------------------------------------------
// Test case 9 — Unterminated: [0x80, 0x80] → LEB128Error
// ---------------------------------------------------------------------------

func TestUnterminatedUnsigned(t *testing.T) {
	// Both bytes have continuation bit set — no terminating byte.
	_, _, err := DecodeUnsigned([]byte{0x80, 0x80}, 0)
	if err == nil {
		t.Fatal("expected error for unterminated sequence, got nil")
	}
	leb128Err, ok := err.(*LEB128Error)
	if !ok {
		t.Fatalf("expected *LEB128Error, got %T", err)
	}
	if leb128Err.Offset != 0 {
		t.Errorf("expected offset 0, got %d", leb128Err.Offset)
	}
	if leb128Err.Message == "" {
		t.Error("expected non-empty message")
	}
}

func TestUnterminatedSigned(t *testing.T) {
	_, _, err := DecodeSigned([]byte{0x80, 0x80}, 0)
	if err == nil {
		t.Fatal("expected error for unterminated sequence, got nil")
	}
}

func TestUnterminatedOneByte(t *testing.T) {
	// Single byte with continuation bit set.
	_, _, err := DecodeUnsigned([]byte{0x80}, 0)
	if err == nil {
		t.Fatal("expected error for single continuation byte, got nil")
	}
}

func TestUnterminatedEmpty(t *testing.T) {
	// Empty input.
	_, _, err := DecodeUnsigned([]byte{}, 0)
	if err == nil {
		t.Fatal("expected error for empty input, got nil")
	}
}

func TestUnterminatedAtOffset(t *testing.T) {
	// Start at offset 1, which has only 2 continuation bytes remaining.
	data := []byte{0x01, 0x80, 0x80}
	_, _, err := DecodeUnsigned(data, 1)
	if err == nil {
		t.Fatal("expected error")
	}
	leb128Err := err.(*LEB128Error)
	if leb128Err.Offset != 1 {
		t.Errorf("expected error at offset 1, got offset %d", leb128Err.Offset)
	}
}

func TestLEB128ErrorImplementsError(t *testing.T) {
	err := &LEB128Error{Message: "test", Offset: 5}
	if err.Error() != "test" {
		t.Errorf("Error() returned %q, expected 'test'", err.Error())
	}
	// Verify it satisfies the error interface.
	var _ error = err
}

// ---------------------------------------------------------------------------
// Test case 10 — Non-zero offset
// ---------------------------------------------------------------------------

func TestNonZeroOffsetUnsigned(t *testing.T) {
	// Junk byte at 0, then [0xE5, 0x8E, 0x26] = 624485 starting at offset 1.
	data := []byte{0xFF, 0xE5, 0x8E, 0x26}
	value, consumed, err := DecodeUnsigned(data, 1)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 624485 {
		t.Errorf("expected 624485, got %d", value)
	}
	if consumed != 3 {
		t.Errorf("expected 3 bytes consumed, got %d", consumed)
	}
}

func TestNonZeroOffsetSigned(t *testing.T) {
	// 0x7E = -2 at offset 1.
	data := []byte{0x00, 0x7E}
	value, consumed, err := DecodeSigned(data, 1)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != -2 {
		t.Errorf("expected -2, got %d", value)
	}
	if consumed != 1 {
		t.Errorf("expected 1 byte consumed, got %d", consumed)
	}
}

func TestNonZeroOffsetPastEnd(t *testing.T) {
	data := []byte{0x03}
	_, _, err := DecodeUnsigned(data, 5)
	if err == nil {
		t.Fatal("expected error for offset past end")
	}
}

func TestMultiByteAtOffset(t *testing.T) {
	// [0x80, 0x01] = 128 at offset 3.
	data := []byte{0x00, 0x00, 0x00, 0x80, 0x01, 0x00}
	value, consumed, err := DecodeUnsigned(data, 3)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 128 {
		t.Errorf("expected 128, got %d", value)
	}
	if consumed != 2 {
		t.Errorf("expected 2 bytes consumed, got %d", consumed)
	}
}

// ---------------------------------------------------------------------------
// Test case 11 — Encode/decode of negative values (signed)
// ---------------------------------------------------------------------------

func TestEncodeDecodeNegativeValues(t *testing.T) {
	// Specific byte-level expectations for common negative values.
	tests := []struct {
		value    int64
		expected []byte
	}{
		{-1, []byte{0x7F}},
		{-2, []byte{0x7E}},
		{-64, []byte{0x40}},
		{-65, []byte{0xBF, 0x7F}},
		{-128, []byte{0x80, 0x7F}},
		{-129, []byte{0xFF, 0x7E}},
	}

	for _, tc := range tests {
		encoded := EncodeSigned(tc.value)
		if len(encoded) != len(tc.expected) {
			t.Errorf("EncodeSigned(%d): expected len %d, got %d: %v",
				tc.value, len(tc.expected), len(encoded), encoded)
			continue
		}
		for i, b := range tc.expected {
			if encoded[i] != b {
				t.Errorf("EncodeSigned(%d) byte %d: expected 0x%02X, got 0x%02X",
					tc.value, i, b, encoded[i])
			}
		}

		// Also verify the round-trip.
		decoded, _, err := DecodeSigned(encoded, 0)
		if err != nil {
			t.Errorf("DecodeSigned of EncodeSigned(%d): %v", tc.value, err)
			continue
		}
		if decoded != tc.value {
			t.Errorf("RoundTrip(%d): got %d", tc.value, decoded)
		}
	}
}

func TestPositive64NeedsTwoBytes(t *testing.T) {
	// +64 = 0x40. Bit 6 of 0x40 is set, so a single byte would be misread as -64.
	// The encoder must emit 2 bytes: [0xC0, 0x00].
	encoded := EncodeSigned(64)
	if len(encoded) != 2 {
		t.Fatalf("expected 2 bytes for +64, got %d: %v", len(encoded), encoded)
	}
	if encoded[0] != 0xC0 || encoded[1] != 0x00 {
		t.Errorf("expected [0xC0, 0x00], got %v", encoded)
	}
	decoded, _, err := DecodeSigned(encoded, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if decoded != 64 {
		t.Errorf("expected 64, got %d", decoded)
	}
}

// ---------------------------------------------------------------------------
// Additional edge cases
// ---------------------------------------------------------------------------

func TestDataAfterLEB128IsIgnored(t *testing.T) {
	// Only the first LEB128 value should be consumed.
	data := []byte{0x03, 0xFF, 0xFF}
	value, consumed, err := DecodeUnsigned(data, 0)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if value != 3 {
		t.Errorf("expected 3, got %d", value)
	}
	if consumed != 1 {
		t.Errorf("expected 1 byte consumed, got %d", consumed)
	}
}

func TestMultipleValuesInBuffer(t *testing.T) {
	// Encode several values into one buffer and decode them sequentially.
	values := []uint64{0, 1, 127, 128, 255, 624485, 4294967295}
	var buf []byte
	for _, v := range values {
		buf = append(buf, EncodeUnsigned(v)...)
	}

	offset := 0
	for _, expected := range values {
		got, consumed, err := DecodeUnsigned(buf, offset)
		if err != nil {
			t.Fatalf("DecodeUnsigned at offset %d: %v", offset, err)
		}
		if got != expected {
			t.Errorf("at offset %d: expected %d, got %d", offset, expected, got)
		}
		offset += consumed
	}
}

func TestMaxU32FiveBytes(t *testing.T) {
	encoded := EncodeUnsigned(4294967295)
	if len(encoded) != 5 {
		t.Errorf("expected 5 bytes for max u32, got %d", len(encoded))
	}
}

func TestBytesConsumedMatchesLength(t *testing.T) {
	// For every encode, bytes_consumed should equal len(encoded).
	unsignedValues := []uint64{0, 1, 127, 128, 16383, 16384, 4294967295}
	for _, v := range unsignedValues {
		enc := EncodeUnsigned(v)
		_, consumed, err := DecodeUnsigned(enc, 0)
		if err != nil {
			t.Errorf("unsigned %d: %v", v, err)
		}
		if consumed != len(enc) {
			t.Errorf("unsigned %d: consumed=%d len=%d", v, consumed, len(enc))
		}
	}

	signedValues := []int64{0, 1, -1, 63, 64, -64, -65, 2147483647, -2147483648}
	for _, v := range signedValues {
		enc := EncodeSigned(v)
		_, consumed, err := DecodeSigned(enc, 0)
		if err != nil {
			t.Errorf("signed %d: %v", v, err)
		}
		if consumed != len(enc) {
			t.Errorf("signed %d: consumed=%d len=%d", v, consumed, len(enc))
		}
	}
}
