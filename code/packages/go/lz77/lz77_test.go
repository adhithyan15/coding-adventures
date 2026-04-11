package lz77

// Comprehensive tests for the LZ77 compression implementation.
//
// Test vectors come from the CMP00 specification and cover all key cases:
// literals, backreferences, overlapping matches, edge cases, and round-trip
// invariants. Table-driven tests are idiomatic Go for functions with many
// input/output pairs.

import (
	"bytes"
	"testing"
)

// --- Helpers ---

// mustEncode is a convenience wrapper for Encode with default parameters.
func mustEncode(data []byte) []Token {
	return Encode(data, 4096, 255, 3)
}

// mustDecode is a convenience wrapper for Decode with no initial buffer.
func mustDecode(tokens []Token) []byte {
	return Decode(tokens, nil)
}

// --- Specification Test Vectors ---

// TestEmptyInput verifies that empty input produces no tokens.
func TestEmptyInput(t *testing.T) {
	tokens := mustEncode([]byte{})
	if len(tokens) != 0 {
		t.Errorf("encode(empty): got %d tokens, want 0", len(tokens))
	}

	decoded := mustDecode([]Token{})
	if len(decoded) != 0 {
		t.Errorf("decode(empty): got %d bytes, want 0", len(decoded))
	}
}

// TestNoRepetition verifies that unique bytes all become literal tokens.
//
// Input: "ABCDE" (no repeated substrings).
// Expected: 5 tokens, each (0, 0, byte).
func TestNoRepetition(t *testing.T) {
	tokens := mustEncode([]byte("ABCDE"))
	if len(tokens) != 5 {
		t.Fatalf("encode(ABCDE): got %d tokens, want 5", len(tokens))
	}
	for i, tok := range tokens {
		if tok.Offset != 0 || tok.Length != 0 {
			t.Errorf("token[%d]: got (%d,%d,%d), want (0,0,byte)", i, tok.Offset, tok.Length, tok.NextChar)
		}
	}
}

// TestAllIdenticalBytes verifies the overlap mechanism for a run of identical bytes.
//
// Input: "AAAAAAA" (7 × A).
// Expected: First A as literal, then one backreference with overlap covering
// the remaining 5 bytes (with overlap), then final A.
// Specifically: [(0,0,65), (1,5,65)]
func TestAllIdenticalBytes(t *testing.T) {
	tokens := mustEncode([]byte("AAAAAAA"))
	if len(tokens) != 2 {
		t.Fatalf("encode(AAAAAAA): got %d tokens, want 2", len(tokens))
	}
	if tokens[0] != (Token{0, 0, 'A'}) {
		t.Errorf("token[0]: got %+v, want {0,0,'A'}", tokens[0])
	}
	if tokens[1].Offset != 1 {
		t.Errorf("token[1].Offset: got %d, want 1", tokens[1].Offset)
	}
	if tokens[1].Length != 5 {
		t.Errorf("token[1].Length: got %d, want 5", tokens[1].Length)
	}
	if tokens[1].NextChar != 'A' {
		t.Errorf("token[1].NextChar: got %d, want 'A'", tokens[1].NextChar)
	}

	// Verify decode produces the original.
	if got := mustDecode(tokens); !bytes.Equal(got, []byte("AAAAAAA")) {
		t.Errorf("decode round-trip: got %q, want %q", got, "AAAAAAA")
	}
}

// TestRepeatedPair verifies non-overlapping backreference for ABABABAB.
//
// Input: "ABABABAB".
// Expected: Literal A, literal B, backreference (offset=2, length=5, nextChar='B').
func TestRepeatedPair(t *testing.T) {
	tokens := mustEncode([]byte("ABABABAB"))
	if len(tokens) != 3 {
		t.Fatalf("encode(ABABABAB): got %d tokens, want 3", len(tokens))
	}
	if tokens[0] != (Token{0, 0, 'A'}) {
		t.Errorf("token[0]: got %+v, want {0,0,'A'}", tokens[0])
	}
	if tokens[1] != (Token{0, 0, 'B'}) {
		t.Errorf("token[1]: got %+v, want {0,0,'B'}", tokens[1])
	}
	if tokens[2].Offset != 2 || tokens[2].Length != 5 || tokens[2].NextChar != 'B' {
		t.Errorf("token[2]: got (%d,%d,%d), want (2,5,'B')", tokens[2].Offset, tokens[2].Length, tokens[2].NextChar)
	}

	if got := mustDecode(tokens); !bytes.Equal(got, []byte("ABABABAB")) {
		t.Errorf("decode round-trip: got %q, want %q", got, "ABABABAB")
	}
}

// TestSubstringReuseNoMatch verifies min_match threshold.
//
// Input: "AABCBBABC" with default min_match=3.
// Expected: All literal tokens (no match of length ≥ 3).
func TestSubstringReuseNoMatch(t *testing.T) {
	tokens := mustEncode([]byte("AABCBBABC"))
	if len(tokens) != 9 {
		t.Fatalf("encode(AABCBBABC): got %d tokens, want 9", len(tokens))
	}
	for i, tok := range tokens {
		if tok.Offset != 0 || tok.Length != 0 {
			t.Errorf("token[%d]: expected literal, got (%d,%d,%d)", i, tok.Offset, tok.Length, tok.NextChar)
		}
	}

	if got := mustDecode(tokens); !bytes.Equal(got, []byte("AABCBBABC")) {
		t.Errorf("decode round-trip failed")
	}
}

// TestSubstringReuseWithLowerMinMatch verifies that min_match=2 triggers matches.
func TestSubstringReuseWithLowerMinMatch(t *testing.T) {
	tokens := Encode([]byte("AABCBBABC"), 4096, 255, 2)
	// With min_match=2, some backreferences should appear.
	if got := mustDecode(tokens); !bytes.Equal(got, []byte("AABCBBABC")) {
		t.Errorf("decode round-trip failed with min_match=2")
	}
}

// --- Round-Trip Invariant Tests ---

// TestRoundTrip verifies decode(encode(x)) == x for various inputs.
func TestRoundTrip(t *testing.T) {
	tests := []struct {
		name string
		data []byte
	}{
		{name: "empty", data: []byte{}},
		{name: "single byte A", data: []byte("A")},
		{name: "single byte null", data: []byte{0x00}},
		{name: "single byte 0xFF", data: []byte{0xFF}},
		{name: "hello world", data: []byte("hello world")},
		{name: "the quick brown fox", data: []byte("the quick brown fox")},
		{name: "ababababab", data: []byte("ababababab")},
		{name: "aaaaaaaaaa", data: []byte("aaaaaaaaaa")},
		{name: "null bytes", data: []byte{0x00, 0x00, 0x00}},
		{name: "ff bytes", data: []byte{0xFF, 0xFF, 0xFF}},
		{name: "all bytes 0..255", data: func() []byte { b := make([]byte, 256); for i := range b { b[i] = byte(i) }; return b }()},
		{name: "repeating pattern", data: []byte{0x00, 0x01, 0x02, 0x00, 0x01, 0x02}},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			tokens := mustEncode(tc.data)
			got := mustDecode(tokens)
			if !bytes.Equal(got, tc.data) {
				t.Errorf("round-trip failed: got %q, want %q", got, tc.data)
			}
		})
	}
}

// TestCompressDecompressRoundTrip verifies the full compress/decompress cycle.
func TestCompressDecompressRoundTrip(t *testing.T) {
	cases := [][]byte{
		{},
		[]byte("A"),
		[]byte("ABCDE"),
		[]byte("AAAAAAA"),
		[]byte("ABABABAB"),
		[]byte("hello world"),
	}

	for _, data := range cases {
		compressed := Compress(data, 4096, 255, 3)
		got := Decompress(compressed)
		if !bytes.Equal(got, data) {
			t.Errorf("compress/decompress round-trip failed for %q", data)
		}
	}
}

// --- Parameter Tests ---

// TestWindowSizeLimit verifies offsets never exceed windowSize.
func TestWindowSizeLimit(t *testing.T) {
	// Large buffer of Ys with X at start and end — the final X cannot reference
	// the first X if windowSize < 5001.
	data := append([]byte("X"), append(bytes.Repeat([]byte("Y"), 5000), 'X')...)
	tokens := Encode(data, 100, 255, 3)
	for _, tok := range tokens {
		if int(tok.Offset) > 100 {
			t.Errorf("offset %d exceeds windowSize 100", tok.Offset)
		}
	}
}

// TestMaxMatchLimit verifies match lengths never exceed maxMatch.
func TestMaxMatchLimit(t *testing.T) {
	data := bytes.Repeat([]byte("A"), 1000)
	tokens := Encode(data, 4096, 50, 3)
	for _, tok := range tokens {
		if int(tok.Length) > 50 {
			t.Errorf("length %d exceeds maxMatch 50", tok.Length)
		}
	}
}

// TestMinMatchThreshold verifies matches shorter than minMatch are not emitted.
func TestMinMatchThreshold(t *testing.T) {
	// "AABAA" — 'A' at position 4 could match at offset 1 or 2, length 1.
	// With min_match=2, length-1 matches should not be emitted.
	tokens := Encode([]byte("AABAA"), 4096, 255, 2)
	for _, tok := range tokens {
		if tok.Length != 0 && tok.Length < 2 {
			t.Errorf("length %d is below minMatch 2", tok.Length)
		}
	}
}

// --- Edge Cases ---

// TestSingleByteLiteral verifies a single byte encodes as a literal token.
func TestSingleByteLiteral(t *testing.T) {
	tokens := mustEncode([]byte("X"))
	if len(tokens) != 1 {
		t.Fatalf("got %d tokens, want 1", len(tokens))
	}
	if tokens[0] != (Token{0, 0, 'X'}) {
		t.Errorf("got %+v, want {0,0,'X'}", tokens[0])
	}
}

// TestExactWindowBoundary verifies a match at exactly windowSize offset.
func TestExactWindowBoundary(t *testing.T) {
	window := 10
	data := append(bytes.Repeat([]byte("X"), window), 'X')
	tokens := Encode(data, window, 255, 3)
	// Verify decode round-trip.
	if got := mustDecode(tokens); !bytes.Equal(got, data) {
		t.Errorf("round-trip failed at window boundary")
	}
	// There should be some match.
	found := false
	for _, tok := range tokens {
		if tok.Offset > 0 {
			found = true
			break
		}
	}
	if !found {
		t.Error("expected at least one match at window boundary")
	}
}

// TestOverlappingMatchDecode verifies byte-by-byte copy handles overlapping matches.
//
// Start with [A, B] and apply (Offset=2, Length=5, NextChar='Z').
// The overlapping match should produce ABABAB (5 bytes copied byte-by-byte),
// then append Z. Total: ABABABAZ (8 bytes).
func TestOverlappingMatchDecode(t *testing.T) {
	tokens := []Token{
		{0, 0, 'A'},
		{0, 0, 'B'},
		{2, 5, 'Z'},
	}
	got := mustDecode(tokens)
	want := []byte("ABABABAZ")
	if !bytes.Equal(got, want) {
		t.Errorf("overlapping match: got %q, want %q", got, want)
	}
}

// TestBinaryWithNulls verifies null bytes are handled correctly.
func TestBinaryWithNulls(t *testing.T) {
	data := []byte{0x00, 0x00, 0x00, 0xFF, 0xFF}
	tokens := mustEncode(data)
	if got := mustDecode(tokens); !bytes.Equal(got, data) {
		t.Errorf("binary with nulls round-trip failed")
	}
}

// TestVeryLongInput verifies large files compress and decompress correctly.
func TestVeryLongInput(t *testing.T) {
	data := append(bytes.Repeat([]byte("Hello, World! "), 100), bytes.Repeat([]byte("X"), 500)...)
	tokens := mustEncode(data)
	if got := mustDecode(tokens); !bytes.Equal(got, data) {
		t.Errorf("long input round-trip failed")
	}
}

// TestAllSameByteCompresses verifies a long run of identical bytes is compressed well.
func TestAllSameByteCompresses(t *testing.T) {
	data := bytes.Repeat([]byte("A"), 10000)
	tokens := mustEncode(data)
	// 1 literal + ~39 backreferences of length 255 + 1 partial = ~41 tokens.
	// Should compress far below the raw 10000 bytes.
	if len(tokens) >= 50 {
		t.Errorf("expected < 50 tokens for 10000 identical bytes, got %d", len(tokens))
	}
	if got := mustDecode(tokens); !bytes.Equal(got, data) {
		t.Errorf("long identical-byte round-trip failed")
	}
}

// --- Serialisation Tests ---

// TestSerialiseFormat verifies the fixed-width binary format structure.
//
// Format: 4-byte header (count) + N × 4-byte tokens.
func TestSerialiseFormat(t *testing.T) {
	tokens := []Token{
		{0, 0, 65},
		{2, 5, 66},
	}
	serialised := serialiseTokens(tokens)
	// 4 bytes for count + 2 tokens × 4 bytes = 12 bytes total.
	want := 4 + 2*4
	if len(serialised) != want {
		t.Errorf("serialised length: got %d, want %d", len(serialised), want)
	}
}

// TestDeserialiseRoundTrip verifies serialise → deserialise is a no-op.
func TestDeserialiseRoundTrip(t *testing.T) {
	tokens := []Token{
		{0, 0, 65},
		{1, 3, 66},
		{2, 5, 67},
	}
	serialised := serialiseTokens(tokens)
	got := deserialiseTokens(serialised)
	if len(got) != len(tokens) {
		t.Fatalf("deserialise count: got %d, want %d", len(got), len(tokens))
	}
	for i := range tokens {
		if got[i] != tokens[i] {
			t.Errorf("token[%d]: got %+v, want %+v", i, got[i], tokens[i])
		}
	}
}

// TestDeserialiseEmpty verifies empty serialised data returns nil tokens.
func TestDeserialiseEmpty(t *testing.T) {
	got := deserialiseTokens([]byte{})
	if len(got) != 0 {
		t.Errorf("deserialise empty: got %d tokens, want 0", len(got))
	}
}

// --- Behaviour Tests ---

// TestNoExpansionOnIncompressibleData verifies worst-case size bound.
//
// N bytes of unique data → N tokens of (0, 0, byte).
// Serialised: 4 bytes header + N × 4 bytes.
func TestNoExpansionOnIncompressibleData(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	compressed := Compress(data, 4096, 255, 3)
	maxSize := 4*len(data) + 10
	if len(compressed) > maxSize {
		t.Errorf("incompressible: got %d bytes, want ≤ %d", len(compressed), maxSize)
	}
}

// TestCompressionOfRepetitiveData verifies repetitive data is reduced.
func TestCompressionOfRepetitiveData(t *testing.T) {
	data := bytes.Repeat([]byte("ABC"), 100)
	compressed := Compress(data, 4096, 255, 3)
	if len(compressed) >= len(data) {
		t.Errorf("expected compression: got %d bytes, original %d bytes", len(compressed), len(data))
	}
}

// TestDeterministicCompression verifies compression is deterministic.
func TestDeterministicCompression(t *testing.T) {
	data := []byte("hello world test")
	result1 := Compress(data, 4096, 255, 3)
	result2 := Compress(data, 4096, 255, 3)
	if !bytes.Equal(result1, result2) {
		t.Error("compression is not deterministic")
	}
}

// TestInitialBuffer verifies Decode with a non-empty initial buffer.
func TestInitialBuffer(t *testing.T) {
	// Seed the decoder with [A, B] and apply a backreference.
	tokens := []Token{{2, 3, 'Z'}}
	got := Decode(tokens, []byte("AB"))
	// start = 2 - 2 = 0; copy 3 bytes from output[0] = A, output[1] = B, output[2] = A
	// then append 'Z' → ABABAZ
	want := []byte("ABABAZ")
	if !bytes.Equal(got, want) {
		t.Errorf("initial buffer decode: got %q, want %q", got, want)
	}
}
