// Tests for the MD5 package.
//
// Test philosophy: we verify correctness against the RFC 1321 test vectors,
// which are the canonical ground truth for any MD5 implementation. We also
// probe edge cases around block boundaries, since padding bugs most often
// appear exactly at 55, 56, 64, 127, and 128 bytes — lengths where the
// padding math changes qualitatively.
//
// Test structure:
//  1. RFC 1321 test vectors — the gold standard
//  2. Output format — byte count, hex format, lowercase, determinism
//  3. Block boundary lengths — 55, 56, 64, 127, 128 bytes
//  4. Edge cases — null bytes, all byte values, single bytes
//  5. Streaming API — equivalence with one-shot, non-destructive, io.Writer
package ca_md5

import (
	"bytes"
	"strings"
	"testing"
)

// ── RFC 1321 Test Vectors ───────────────────────────────────────────────────
//
// These are the canonical test cases from RFC 1321 §A.5. Any correct MD5
// implementation must produce exactly these outputs. If your implementation
// fails even one of these, it is wrong.

func TestRFC1321EmptyString(t *testing.T) {
	// The MD5 of the empty message is not the MD5 of "nothing" — it is the
	// digest of the padded empty message, which is just the padding block:
	//   [0x80 0x00 ... 0x00 0x00 ... 0x00]  (64 bytes, length field = 0)
	got := HexString([]byte{})
	want := "d41d8cd98f00b204e9800998ecf8427e"
	if got != want {
		t.Errorf("HexString(\"\") = %q, want %q", got, want)
	}
}

func TestRFC1321SingleCharA(t *testing.T) {
	got := HexString([]byte("a"))
	want := "0cc175b9c0f1b6a831c399e269772661"
	if got != want {
		t.Errorf("HexString(\"a\") = %q, want %q", got, want)
	}
}

func TestRFC1321ABC(t *testing.T) {
	got := HexString([]byte("abc"))
	want := "900150983cd24fb0d6963f7d28e17f72"
	if got != want {
		t.Errorf("HexString(\"abc\") = %q, want %q", got, want)
	}
}

func TestRFC1321MessageDigest(t *testing.T) {
	got := HexString([]byte("message digest"))
	want := "f96b697d7cb7938d525a2f31aaf161d0"
	if got != want {
		t.Errorf("HexString(\"message digest\") = %q, want %q", got, want)
	}
}

func TestRFC1321AlphabetLower(t *testing.T) {
	// md5("abcdefghijklmnopqrstuvwxyz") from RFC 1321
	got := HexString([]byte("abcdefghijklmnopqrstuvwxyz"))
	want := "c3fcd3d76192e4007dfb496cca67e13b"
	if got != want {
		t.Errorf("HexString(alphabet) = %q, want %q", got, want)
	}
}

func TestRFC1321AlphanumericMixed(t *testing.T) {
	// md5("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")
	got := HexString([]byte("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"))
	want := "d174ab98d277d9f5a5611c2c9f419d9f"
	if got != want {
		t.Errorf("HexString(alphanumeric) = %q, want %q", got, want)
	}
}

func TestRFC1321LongNumericString(t *testing.T) {
	// md5("12345678901234567890123456789012345678901234567890123456789012345678901234567890")
	got := HexString([]byte("12345678901234567890123456789012345678901234567890123456789012345678901234567890"))
	want := "57edf4a22be3c955ac49da2e2107b67a"
	if got != want {
		t.Errorf("HexString(long numeric) = %q, want %q", got, want)
	}
}

// ── Output Format Tests ─────────────────────────────────────────────────────

func TestSumMD5Returns16Bytes(t *testing.T) {
	// The MD5 digest is always exactly 16 bytes, regardless of input size.
	digest := SumMD5([]byte("hello"))
	if len(digest) != 16 {
		t.Errorf("SumMD5 returned %d bytes, want 16", len(digest))
	}
}

func TestHexStringIs32Chars(t *testing.T) {
	// The hex encoding of 16 bytes is always exactly 32 characters
	// (2 hex chars per byte × 16 bytes = 32 chars).
	hex := HexString([]byte("hello"))
	if len(hex) != 32 {
		t.Errorf("HexString returned %d chars, want 32", len(hex))
	}
}

func TestHexStringIsLowercase(t *testing.T) {
	// MD5 convention is lowercase hex. Uppercase would still be a valid
	// representation of the same bytes, but convention and the RFC examples
	// use lowercase.
	hex := HexString([]byte("Hello, World!"))
	if hex != strings.ToLower(hex) {
		t.Errorf("HexString returned uppercase chars: %q", hex)
	}
}

func TestHexStringOnlyHexChars(t *testing.T) {
	// Verify the hex string contains only [0-9a-f].
	hex := HexString([]byte("test string"))
	for i, ch := range hex {
		if !((ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f')) {
			t.Errorf("HexString[%d] = %q, not a valid lowercase hex char", i, ch)
		}
	}
}

func TestDeterministic(t *testing.T) {
	// The same input must always produce the same digest.
	// MD5 is a pure function — no randomness.
	input := []byte("determinism test")
	first := HexString(input)
	for i := 0; i < 5; i++ {
		if got := HexString(input); got != first {
			t.Errorf("call %d: HexString returned %q, want %q", i+1, got, first)
		}
	}
}

func TestSumMD5AndHexStringAgree(t *testing.T) {
	// SumMD5 and HexString must agree — HexString is just the hex encoding of
	// the bytes returned by SumMD5.
	input := []byte("consistency check")
	digest := SumMD5(input)
	hexFromBytes := strings.ToLower(func() string {
		h := ""
		for _, b := range digest {
			h += string([]byte{
				"0123456789abcdef"[b>>4],
				"0123456789abcdef"[b&0xf],
			})
		}
		return h
	}())
	hexDirect := HexString(input)
	if hexDirect != hexFromBytes {
		t.Errorf("SumMD5 and HexString disagree: %q vs %q", hexFromBytes, hexDirect)
	}
}

// ── Block Boundary Tests ────────────────────────────────────────────────────
//
// MD5 processes data in 64-byte blocks. Padding behavior changes at specific
// lengths:
//
//   - 55 bytes: one block, pad fits in remaining 9 bytes (1 + 0 zeros + 8 length)
//   - 56 bytes: message + 0x80 = 57 bytes, need 64 zeros → 121 bytes →
//     need 63 more zeros to reach 184 = 56+64+64 ... no wait:
//     56+1=57, 57%64=57 ≠ 56, need 64-(57-56)=63 zeros → 120 bytes, + 8 = 128 (2 blocks)
//   - 64 bytes: message + 0x80 = 65 bytes, need 55 zeros → 120 bytes, +8 = 128 (2 blocks)
//   - 127 bytes: message + 0x80 = 128 bytes, 128%64=0 ≠ 56, need 56 more zeros
//     → 184 bytes, +8 = 192 (3 blocks)
//   - 128 bytes: message + 0x80 = 129, 129%64=1 ≠ 56, need 55 zeros → 184, +8 = 192 (3 blocks)
//
// Each of these boundary lengths must produce a unique, correct digest.

func TestBlockBoundary55Bytes(t *testing.T) {
	// 55-byte message: padding fits in the same block.
	// Structure: [55 data bytes | 0x80 | 8-byte length] = 64 bytes (1 block)
	data := bytes.Repeat([]byte("a"), 55)
	got := HexString(data)
	// Verified against Python hashlib.md5 reference implementation
	want := "ef1772b6dff9a122358552954ad0df65"
	if got != want {
		t.Errorf("55-byte boundary: got %q, want %q", got, want)
	}
}

func TestBlockBoundary56Bytes(t *testing.T) {
	// 56-byte message: forces an extra padding block.
	// The 0x80 byte makes 57 bytes. We need 56+64=120 bytes before the length,
	// so we add 63 zeros → 120 bytes total, then 8-byte length = 128 (2 blocks).
	data := bytes.Repeat([]byte("a"), 56)
	got := HexString(data)
	// Verified against Python hashlib.md5 reference implementation
	want := "3b0c8ac703f828b04c6c197006d17218"
	if got != want {
		t.Errorf("56-byte boundary: got %q, want %q", got, want)
	}
}

func TestBlockBoundary64Bytes(t *testing.T) {
	// 64-byte message: exactly one full block before padding.
	// After padding: 128 bytes (2 blocks).
	data := bytes.Repeat([]byte("a"), 64)
	got := HexString(data)
	want := "014842d480b571495a4a0363793f7367"
	if got != want {
		t.Errorf("64-byte boundary: got %q, want %q", got, want)
	}
}

func TestBlockBoundary127Bytes(t *testing.T) {
	// 127-byte message: two full blocks before padding.
	// After padding: 192 bytes (3 blocks).
	data := bytes.Repeat([]byte("a"), 127)
	got := HexString(data)
	// Verified against Python hashlib.md5 reference implementation
	want := "020406e1d05cdc2aa287641f7ae2cc39"
	if got != want {
		t.Errorf("127-byte boundary: got %q, want %q", got, want)
	}
}

func TestBlockBoundary128Bytes(t *testing.T) {
	// 128-byte message: exactly two full blocks before padding.
	// After padding: 192 bytes (3 blocks).
	data := bytes.Repeat([]byte("a"), 128)
	got := HexString(data)
	// Verified against Python hashlib.md5 reference implementation
	want := "e510683b3f5ffe4093d021808bc6ff70"
	if got != want {
		t.Errorf("128-byte boundary: got %q, want %q", got, want)
	}
}

func TestBlockBoundariesAreDistinct(t *testing.T) {
	// All the boundary-length digests must be different from each other.
	// If two lengths produce the same digest, we have a collision — which
	// would indicate a bug (not just MD5's known cryptographic weakness,
	// which involves chosen-prefix collisions, not same-length all-'a' inputs).
	lengths := []int{55, 56, 64, 127, 128}
	seen := make(map[string]int)
	for _, n := range lengths {
		data := bytes.Repeat([]byte("a"), n)
		h := HexString(data)
		if prev, ok := seen[h]; ok {
			t.Errorf("lengths %d and %d produced the same digest %q", prev, n, h)
		}
		seen[h] = n
	}
}

// ── Edge Case Tests ─────────────────────────────────────────────────────────

func TestNullByte(t *testing.T) {
	// A single null byte is NOT the same as an empty message.
	// The empty message digest is d41d8cd98f00b204e9800998ecf8427e.
	nullDigest := HexString([]byte{0x00})
	emptyDigest := HexString([]byte{})
	if nullDigest == emptyDigest {
		t.Errorf("null byte and empty string have the same digest: %q", nullDigest)
	}
}

func TestNullByteKnownValue(t *testing.T) {
	// Verify against a known-good value for the single null byte.
	got := HexString([]byte{0x00})
	want := "93b885adfe0da089cdf634904fd59f71"
	if got != want {
		t.Errorf("HexString(\\x00) = %q, want %q", got, want)
	}
}

func TestAllBytesUnique(t *testing.T) {
	// Every single-byte value 0x00..0xFF must produce a unique digest.
	// A collision here would indicate a catastrophic implementation error.
	seen := make(map[string]int)
	for i := 0; i < 256; i++ {
		h := HexString([]byte{byte(i)})
		if prev, ok := seen[h]; ok {
			t.Errorf("bytes 0x%02x and 0x%02x have the same digest: %q", prev, i, h)
		}
		seen[h] = i
	}
}

func TestAllBytesAre32CharsLowercase(t *testing.T) {
	// Every single-byte input must produce a valid 32-char lowercase hex string.
	for i := 0; i < 256; i++ {
		h := HexString([]byte{byte(i)})
		if len(h) != 32 {
			t.Errorf("byte 0x%02x: HexString returned %d chars, want 32", i, len(h))
		}
		if h != strings.ToLower(h) {
			t.Errorf("byte 0x%02x: HexString returned uppercase: %q", i, h)
		}
	}
}

func TestHighByteValues(t *testing.T) {
	// Test some specific high byte values to exercise all bit patterns.
	// Values verified against Python hashlib.md5 reference implementation.
	cases := []struct {
		input byte
		want  string
	}{
		{0xFF, "00594fd4f42ba43fc1ca0427a0576295"},
		{0x80, "8d39dd7eef115ea6975446ef4082951f"},
		{0x7F, "83acb6e67e50e31db6ed341dd2de1595"},
	}
	for _, tc := range cases {
		got := HexString([]byte{tc.input})
		if got != tc.want {
			t.Errorf("HexString(0x%02x) = %q, want %q", tc.input, got, tc.want)
		}
	}
}

func TestLongerRepeatedPattern(t *testing.T) {
	// A longer message with a repeated pattern — exercises multiple blocks.
	data := bytes.Repeat([]byte{0xAB, 0xCD}, 50) // 100 bytes
	got := HexString(data)
	// Each call must return the same value.
	if got2 := HexString(data); got != got2 {
		t.Errorf("non-deterministic: got %q then %q", got, got2)
	}
	if len(got) != 32 {
		t.Errorf("got %d chars, want 32", len(got))
	}
}

// ── Streaming API Tests ─────────────────────────────────────────────────────
//
// The streaming Digest must be equivalent to the one-shot SumMD5, regardless
// of how the data is split between Write calls. This is the key correctness
// invariant of the streaming interface.

func TestStreamingSingleWrite(t *testing.T) {
	// A single Write to the streaming API must equal the one-shot result.
	input := []byte("abc")
	d := New()
	d.Write(input)
	got := d.HexDigest()
	want := "900150983cd24fb0d6963f7d28e17f72"
	if got != want {
		t.Errorf("streaming single write: got %q, want %q", got, want)
	}
}

func TestStreamingSplitAtByte(t *testing.T) {
	// Splitting "abc" after the first byte: Write("a"), Write("bc").
	d := New()
	d.Write([]byte("a"))
	d.Write([]byte("bc"))
	got := d.HexDigest()
	want := "900150983cd24fb0d6963f7d28e17f72"
	if got != want {
		t.Errorf("split at byte 1: got %q, want %q", got, want)
	}
}

func TestStreamingSplitAtBlock(t *testing.T) {
	// Split a 128-byte message at the 64-byte block boundary.
	data := bytes.Repeat([]byte("a"), 128)
	want := HexString(data) // one-shot reference

	d := New()
	d.Write(data[:64])
	d.Write(data[64:])
	got := d.HexDigest()
	if got != want {
		t.Errorf("split at block boundary: got %q, want %q", got, want)
	}
}

func TestStreamingByteAtATime(t *testing.T) {
	// Feed "message digest" one byte at a time.
	// This is the most stress-full test of the buffer management logic.
	input := []byte("message digest")
	want := "f96b697d7cb7938d525a2f31aaf161d0" // RFC 1321 vector

	d := New()
	for _, b := range input {
		d.Write([]byte{b})
	}
	got := d.HexDigest()
	if got != want {
		t.Errorf("byte-at-a-time streaming: got %q, want %q", got, want)
	}
}

func TestStreamingEmpty(t *testing.T) {
	// An empty streaming hash must equal the one-shot hash of the empty message.
	d := New()
	got := d.HexDigest()
	want := "d41d8cd98f00b204e9800998ecf8427e"
	if got != want {
		t.Errorf("streaming empty: got %q, want %q", got, want)
	}
}

func TestStreamingNonDestructive(t *testing.T) {
	// SumMD5/HexDigest must be non-destructive: calling them multiple times
	// on the same Digest must return the same result, and Write calls after
	// calling SumMD5 must still work correctly.
	d := New()
	d.Write([]byte("abc"))

	// Call HexDigest three times — all must agree.
	first := d.HexDigest()
	second := d.HexDigest()
	third := d.HexDigest()
	if first != second || second != third {
		t.Errorf("HexDigest not idempotent: %q, %q, %q", first, second, third)
	}

	// Continue writing — must produce correct result.
	d.Write([]byte("defghijklmnopqrstuvwxyz"))
	got := d.HexDigest()
	want := HexString([]byte("abcdefghijklmnopqrstuvwxyz"))
	if got != want {
		t.Errorf("after continuing: got %q, want %q", got, want)
	}
}

func TestStreamingRFCVectorEmpty(t *testing.T) {
	// RFC 1321 test vector via streaming.
	d := New()
	if got := d.HexDigest(); got != "d41d8cd98f00b204e9800998ecf8427e" {
		t.Errorf("streaming RFC empty: got %q", got)
	}
}

func TestStreamingRFCVectorA(t *testing.T) {
	// RFC 1321 "a" via streaming.
	d := New()
	d.Write([]byte("a"))
	if got := d.HexDigest(); got != "0cc175b9c0f1b6a831c399e269772661" {
		t.Errorf("streaming RFC 'a': got %q", got)
	}
}

func TestStreamingRFCVectorABC(t *testing.T) {
	// RFC 1321 "abc" via streaming, split into three single-byte writes.
	d := New()
	d.Write([]byte("a"))
	d.Write([]byte("b"))
	d.Write([]byte("c"))
	if got := d.HexDigest(); got != "900150983cd24fb0d6963f7d28e17f72" {
		t.Errorf("streaming RFC 'abc': got %q", got)
	}
}

func TestStreamingRFCVectorMessageDigest(t *testing.T) {
	// RFC 1321 "message digest" via streaming, split at space.
	d := New()
	d.Write([]byte("message"))
	d.Write([]byte(" "))
	d.Write([]byte("digest"))
	if got := d.HexDigest(); got != "f96b697d7cb7938d525a2f31aaf161d0" {
		t.Errorf("streaming RFC 'message digest': got %q", got)
	}
}

func TestStreamingEqualsOneShotForAllLengths(t *testing.T) {
	// For messages of length 0..200, verify that streaming (byte-at-a-time)
	// equals the one-shot result. This systematically catches buffer management
	// bugs at and around every block boundary.
	for n := 0; n <= 200; n++ {
		data := make([]byte, n)
		for i := range data {
			data[i] = byte(i % 251) // 251 is prime, gives variety
		}

		want := HexString(data)

		d := New()
		for _, b := range data {
			d.Write([]byte{b})
		}
		got := d.HexDigest()

		if got != want {
			t.Errorf("length %d: streaming = %q, one-shot = %q", n, got, want)
		}
	}
}

func TestSumMD5StreamingMethod(t *testing.T) {
	// Test the SumMD5 method on Digest (returns [16]byte).
	d := New()
	d.Write([]byte("abc"))
	digest := d.SumMD5()
	if len(digest) != 16 {
		t.Errorf("Digest.SumMD5() returned %d bytes, want 16", len(digest))
	}
	// Must agree with the package-level SumMD5 function.
	expected := SumMD5([]byte("abc"))
	if digest != expected {
		t.Errorf("Digest.SumMD5() = %v, want %v", digest, expected)
	}
}

func TestWriteReturnsCorrectCount(t *testing.T) {
	// Write must return (len(p), nil) per the io.Writer contract.
	d := New()
	n, err := d.Write([]byte("hello"))
	if n != 5 {
		t.Errorf("Write returned n=%d, want 5", n)
	}
	if err != nil {
		t.Errorf("Write returned error: %v", err)
	}
}

func TestStreamingLargeInput(t *testing.T) {
	// Test with a large input (1 MB) split into 4 KB chunks.
	// This exercises many block boundaries and the buffer accumulation logic.
	size := 1 << 20 // 1 MB
	data := make([]byte, size)
	for i := range data {
		data[i] = byte(i & 0xFF)
	}

	want := HexString(data)

	d := New()
	chunkSize := 4096
	for i := 0; i < len(data); i += chunkSize {
		end := i + chunkSize
		if end > len(data) {
			end = len(data)
		}
		d.Write(data[i:end])
	}
	got := d.HexDigest()

	if got != want {
		t.Errorf("large input: streaming = %q, one-shot = %q", got, want)
	}
}

func TestNewCreatesIndependentDigests(t *testing.T) {
	// Two Digest instances must be completely independent.
	d1 := New()
	d2 := New()

	d1.Write([]byte("hello"))
	d2.Write([]byte("world"))

	// d1 and d2 must differ.
	if d1.HexDigest() == d2.HexDigest() {
		t.Errorf("two different Digests produced the same result")
	}

	// Each must match the one-shot result.
	if got, want := d1.HexDigest(), HexString([]byte("hello")); got != want {
		t.Errorf("d1: got %q, want %q", got, want)
	}
	if got, want := d2.HexDigest(), HexString([]byte("world")); got != want {
		t.Errorf("d2: got %q, want %q", got, want)
	}
}
