package lzss_test

import (
	"bytes"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lzss"
)

// ─── Helpers ──────────────────────────────────────────────────────────────────

func rt(data []byte) []byte {
	return lzss.Decompress(lzss.Compress(data))
}

func bs(s string) []byte { return []byte(s) }

// ─── Spec vectors ─────────────────────────────────────────────────────────────

func TestEncodeEmpty(t *testing.T) {
	got := lzss.Encode([]byte{}, lzss.DefaultWindowSize, lzss.DefaultMaxMatch, lzss.DefaultMinMatch)
	if len(got) != 0 {
		t.Errorf("want empty, got %v", got)
	}
}

func TestEncodeSingleByte(t *testing.T) {
	got := lzss.Encode([]byte{65}, lzss.DefaultWindowSize, lzss.DefaultMaxMatch, lzss.DefaultMinMatch)
	if len(got) != 1 || got[0].Kind != lzss.KindLiteral || got[0].Byte != 65 {
		t.Errorf("want [Literal(65)], got %v", got)
	}
}

func TestEncodeNoRepetition(t *testing.T) {
	tokens := lzss.Encode(bs("ABCDE"), lzss.DefaultWindowSize, lzss.DefaultMaxMatch, lzss.DefaultMinMatch)
	if len(tokens) != 5 {
		t.Fatalf("want 5 tokens, got %d", len(tokens))
	}
	for _, tok := range tokens {
		if tok.Kind != lzss.KindLiteral {
			t.Errorf("expected all literals, got Kind=%d", tok.Kind)
		}
	}
}

func TestEncodeAABCBBABC(t *testing.T) {
	tokens := lzss.Encode(bs("AABCBBABC"), lzss.DefaultWindowSize, lzss.DefaultMaxMatch, lzss.DefaultMinMatch)
	// Expect 6 literals + 1 match
	if len(tokens) != 7 {
		t.Fatalf("want 7 tokens, got %d", len(tokens))
	}
	last := tokens[6]
	if last.Kind != lzss.KindMatch || last.Offset != 5 || last.Length != 3 {
		t.Errorf("want Match(5,3), got Kind=%d Offset=%d Length=%d", last.Kind, last.Offset, last.Length)
	}
}

func TestEncodeABABAB(t *testing.T) {
	tokens := lzss.Encode(bs("ABABAB"), lzss.DefaultWindowSize, lzss.DefaultMaxMatch, lzss.DefaultMinMatch)
	if len(tokens) != 3 {
		t.Fatalf("want 3 tokens, got %d", len(tokens))
	}
	if tokens[0].Kind != lzss.KindLiteral || tokens[0].Byte != 'A' {
		t.Errorf("token 0: want Literal('A')")
	}
	if tokens[1].Kind != lzss.KindLiteral || tokens[1].Byte != 'B' {
		t.Errorf("token 1: want Literal('B')")
	}
	m := tokens[2]
	if m.Kind != lzss.KindMatch || m.Offset != 2 || m.Length != 4 {
		t.Errorf("token 2: want Match(2,4), got Offset=%d Length=%d", m.Offset, m.Length)
	}
}

func TestEncodeAllIdentical(t *testing.T) {
	tokens := lzss.Encode(bs("AAAAAAA"), lzss.DefaultWindowSize, lzss.DefaultMaxMatch, lzss.DefaultMinMatch)
	if len(tokens) != 2 {
		t.Fatalf("want 2 tokens, got %d", len(tokens))
	}
	if tokens[0].Kind != lzss.KindLiteral {
		t.Errorf("token 0: want literal")
	}
	m := tokens[1]
	if m.Kind != lzss.KindMatch || m.Offset != 1 || m.Length != 6 {
		t.Errorf("token 1: want Match(1,6), got Offset=%d Length=%d", m.Offset, m.Length)
	}
}

// ─── Encode properties ────────────────────────────────────────────────────────

func TestMatchOffsetPositive(t *testing.T) {
	for _, tok := range lzss.Encode(bs("ABABABAB"), lzss.DefaultWindowSize, lzss.DefaultMaxMatch, lzss.DefaultMinMatch) {
		if tok.Kind == lzss.KindMatch && tok.Offset == 0 {
			t.Error("match offset must be >= 1")
		}
	}
}

func TestMatchLengthGeMinMatch(t *testing.T) {
	min := lzss.DefaultMinMatch
	for _, tok := range lzss.Encode(bs("ABABABABABAB"), lzss.DefaultWindowSize, lzss.DefaultMaxMatch, min) {
		if tok.Kind == lzss.KindMatch && int(tok.Length) < min {
			t.Errorf("match length %d < min_match %d", tok.Length, min)
		}
	}
}

func TestMatchOffsetWithinWindow(t *testing.T) {
	ws := 8
	data := bs("ABCABCABCABC")
	for _, tok := range lzss.Encode(data, ws, lzss.DefaultMaxMatch, lzss.DefaultMinMatch) {
		if tok.Kind == lzss.KindMatch && int(tok.Offset) > ws {
			t.Errorf("offset %d > window_size %d", tok.Offset, ws)
		}
	}
}

func TestMatchLengthWithinMax(t *testing.T) {
	maxM := 5
	for _, tok := range lzss.Encode(bytes.Repeat([]byte{65}, 100), lzss.DefaultWindowSize, maxM, lzss.DefaultMinMatch) {
		if tok.Kind == lzss.KindMatch && int(tok.Length) > maxM {
			t.Errorf("length %d > max_match %d", tok.Length, maxM)
		}
	}
}

// ─── Decode ───────────────────────────────────────────────────────────────────

func TestDecodeEmpty(t *testing.T) {
	got := lzss.Decode(nil, 0)
	if len(got) != 0 {
		t.Errorf("want empty, got %v", got)
	}
}

func TestDecodeSingleLiteral(t *testing.T) {
	got := lzss.Decode([]lzss.Token{lzss.Literal(65)}, 1)
	if !bytes.Equal(got, []byte{65}) {
		t.Errorf("want [65], got %v", got)
	}
}

func TestDecodeOverlappingMatch(t *testing.T) {
	// Literal('A') + Match(offset=1, length=6) → "AAAAAAA"
	tokens := []lzss.Token{lzss.Literal(65), lzss.Match(1, 6)}
	got := lzss.Decode(tokens, 7)
	if !bytes.Equal(got, bs("AAAAAAA")) {
		t.Errorf("want AAAAAAA, got %s", got)
	}
}

func TestDecodeABABAB(t *testing.T) {
	tokens := []lzss.Token{lzss.Literal(65), lzss.Literal(66), lzss.Match(2, 4)}
	got := lzss.Decode(tokens, 6)
	if !bytes.Equal(got, bs("ABABAB")) {
		t.Errorf("want ABABAB, got %s", got)
	}
}

// ─── Round-trip ───────────────────────────────────────────────────────────────

func TestRoundTripEmpty(t *testing.T) {
	if !bytes.Equal(rt([]byte{}), []byte{}) {
		t.Error("empty round-trip failed")
	}
}

func TestRoundTripSingleByte(t *testing.T) {
	if !bytes.Equal(rt(bs("A")), bs("A")) {
		t.Error("single byte round-trip failed")
	}
}

func TestRoundTripNoRepetition(t *testing.T) {
	if !bytes.Equal(rt(bs("ABCDE")), bs("ABCDE")) {
		t.Error("no-repetition round-trip failed")
	}
}

func TestRoundTripAllIdentical(t *testing.T) {
	if !bytes.Equal(rt(bs("AAAAAAA")), bs("AAAAAAA")) {
		t.Error("all-identical round-trip failed")
	}
}

func TestRoundTripABABAB(t *testing.T) {
	if !bytes.Equal(rt(bs("ABABAB")), bs("ABABAB")) {
		t.Error("ABABAB round-trip failed")
	}
}

func TestRoundTripAABCBBABC(t *testing.T) {
	if !bytes.Equal(rt(bs("AABCBBABC")), bs("AABCBBABC")) {
		t.Error("AABCBBABC round-trip failed")
	}
}

func TestRoundTripHelloWorld(t *testing.T) {
	if !bytes.Equal(rt(bs("hello world")), bs("hello world")) {
		t.Error("hello world round-trip failed")
	}
}

func TestRoundTripBinaryNulls(t *testing.T) {
	data := []byte{0, 0, 0, 255, 255}
	if !bytes.Equal(rt(data), data) {
		t.Error("binary nulls round-trip failed")
	}
}

func TestRoundTripFullByteRange(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	if !bytes.Equal(rt(data), data) {
		t.Error("full byte range round-trip failed")
	}
}

func TestRoundTripRepeatedPattern(t *testing.T) {
	data := bytes.Repeat([]byte{0, 1, 2}, 100)
	if !bytes.Equal(rt(data), data) {
		t.Error("repeated pattern round-trip failed")
	}
}

func TestRoundTripLong(t *testing.T) {
	data := bytes.Repeat(bs("ABCDEF"), 500)
	if !bytes.Equal(rt(data), data) {
		t.Error("long round-trip failed")
	}
}

// ─── Wire format ─────────────────────────────────────────────────────────────

func TestCompressDeterministic(t *testing.T) {
	data := bs("hello world test")
	if !bytes.Equal(lzss.Compress(data), lzss.Compress(data)) {
		t.Error("compress is not deterministic")
	}
}

func TestCompressEmptyHeader(t *testing.T) {
	c := lzss.Compress([]byte{})
	if len(c) < 8 {
		t.Fatal("compressed empty must be at least 8 bytes")
	}
	if c[0] != 0 || c[1] != 0 || c[2] != 0 || c[3] != 0 {
		t.Error("original_length field should be 0")
	}
}

func TestCraftedLargeBlockCountIsSafe(t *testing.T) {
	// Craft a header claiming 2^30 blocks with only 16 bytes of payload.
	bad := make([]byte, 16)
	bad[4] = 0x40 // block_count = 0x40000000 (2^30)
	result := lzss.Decompress(bad)
	_ = result // must not panic
}

// ─── Compression effectiveness ────────────────────────────────────────────────

func TestRepetitiveDataCompresses(t *testing.T) {
	data := bytes.Repeat(bs("ABC"), 1000)
	if len(lzss.Compress(data)) >= len(data) {
		t.Error("repetitive data should compress")
	}
}

func TestAllSameByteCompresses(t *testing.T) {
	data := bytes.Repeat([]byte{0x42}, 10000)
	compressed := lzss.Compress(data)
	if len(compressed) >= len(data) {
		t.Error("all-same-byte data should compress")
	}
	if !bytes.Equal(lzss.Decompress(compressed), data) {
		t.Error("all-same-byte round-trip failed")
	}
}
