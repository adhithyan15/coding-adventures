package lz78_test

import (
	"bytes"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lz78"
)

const defaultMaxDict = 65536

// ─── Spec vectors ─────────────────────────────────────────────────────────────

func TestEmptyInput(t *testing.T) {
	tokens := lz78.Encode(nil, defaultMaxDict)
	if len(tokens) != 0 {
		t.Fatalf("empty: want 0 tokens, got %d", len(tokens))
	}
	out := lz78.Decode(nil, 0)
	if len(out) != 0 {
		t.Fatalf("empty decode: want empty, got %v", out)
	}
}

func TestSingleByte(t *testing.T) {
	tokens := lz78.Encode([]byte("A"), defaultMaxDict)
	if len(tokens) != 1 {
		t.Fatalf("single byte: want 1 token, got %d", len(tokens))
	}
	if tokens[0] != (lz78.Token{DictIndex: 0, NextChar: 65}) {
		t.Fatalf("single byte token: got %+v", tokens[0])
	}
	out := lz78.Decode(tokens, 1)
	if !bytes.Equal(out, []byte("A")) {
		t.Fatalf("decode: want A, got %v", out)
	}
}

func TestNoRepetition(t *testing.T) {
	tokens := lz78.Encode([]byte("ABCDE"), defaultMaxDict)
	if len(tokens) != 5 {
		t.Fatalf("ABCDE: want 5 tokens, got %d", len(tokens))
	}
	for _, tok := range tokens {
		if tok.DictIndex != 0 {
			t.Fatalf("expected all literals, got DictIndex=%d", tok.DictIndex)
		}
	}
}

func TestAABCBBABC(t *testing.T) {
	want := []lz78.Token{
		{0, 65}, {1, 66}, {0, 67}, {0, 66}, {4, 65}, {4, 67},
	}
	got := lz78.Encode([]byte("AABCBBABC"), defaultMaxDict)
	if len(got) != len(want) {
		t.Fatalf("AABCBBABC: want %d tokens, got %d", len(want), len(got))
	}
	for i, tok := range want {
		if got[i] != tok {
			t.Fatalf("token[%d]: want %+v, got %+v", i, tok, got[i])
		}
	}
	out := lz78.Decode(got, len("AABCBBABC"))
	if !bytes.Equal(out, []byte("AABCBBABC")) {
		t.Fatalf("decode AABCBBABC: got %v", out)
	}
}

func TestABABAB(t *testing.T) {
	want := []lz78.Token{
		{0, 65}, {0, 66}, {1, 66}, {3, 0},
	}
	got := lz78.Encode([]byte("ABABAB"), defaultMaxDict)
	if len(got) != len(want) {
		t.Fatalf("ABABAB: want %d tokens, got %d", len(want), len(got))
	}
	for i, tok := range want {
		if got[i] != tok {
			t.Fatalf("token[%d]: want %+v, got %+v", i, tok, got[i])
		}
	}
	if !bytes.Equal(lz78.Decompress(lz78.Compress([]byte("ABABAB"), defaultMaxDict)), []byte("ABABAB")) {
		t.Fatal("ABABAB round-trip failed")
	}
}

func TestAllIdenticalBytes(t *testing.T) {
	tokens := lz78.Encode([]byte("AAAAAAA"), defaultMaxDict)
	if len(tokens) != 4 {
		t.Fatalf("AAAAAAA: want 4 tokens, got %d", len(tokens))
	}
}

// ─── Round-trip tests ─────────────────────────────────────────────────────────

func TestRoundTrip(t *testing.T) {
	cases := []string{
		"", "A", "ABCDE", "AAAAAAA", "ABABABAB", "AABCBBABC",
		"hello world", "the quick brown fox", "ababababab", "aaaaaaaaaa",
	}
	for _, s := range cases {
		got := lz78.Decompress(lz78.Compress([]byte(s), defaultMaxDict))
		if !bytes.Equal(got, []byte(s)) {
			t.Errorf("round-trip %q: got %q", s, got)
		}
	}
}

func TestBinaryRoundTrip(t *testing.T) {
	cases := [][]byte{
		{0, 0, 0},
		{255, 255, 255},
		func() []byte {
			b := make([]byte, 256)
			for i := range b {
				b[i] = byte(i)
			}
			return b
		}(),
		{0, 1, 2, 0, 1, 2},
		{0, 0, 0, 255, 255},
	}
	for _, data := range cases {
		got := lz78.Decompress(lz78.Compress(data, defaultMaxDict))
		if !bytes.Equal(got, data) {
			t.Errorf("binary round-trip failed for %v", data)
		}
	}
}

// ─── Parameter tests ──────────────────────────────────────────────────────────

func TestMaxDictSizeRespected(t *testing.T) {
	data := []byte("ABCABCABCABCABC")
	tokens := lz78.Encode(data, 10)
	for _, tok := range tokens {
		if int(tok.DictIndex) >= 10 {
			t.Fatalf("DictIndex %d exceeds maxDictSize=10", tok.DictIndex)
		}
	}
}

func TestMaxDictSize1(t *testing.T) {
	tokens := lz78.Encode([]byte("AAAA"), 1)
	for _, tok := range tokens {
		if tok.DictIndex != 0 {
			t.Fatalf("maxDictSize=1: want all literals, got DictIndex=%d", tok.DictIndex)
		}
	}
}

// ─── Edge cases ───────────────────────────────────────────────────────────────

func TestSingleByteLiteral(t *testing.T) {
	tokens := lz78.Encode([]byte("X"), defaultMaxDict)
	if len(tokens) != 1 || tokens[0] != (lz78.Token{0, 88}) {
		t.Fatalf("X: want [(0,88)], got %v", tokens)
	}
}

func TestBinaryWithNulls(t *testing.T) {
	data := []byte{0, 0, 0, 255, 255}
	got := lz78.Decompress(lz78.Compress(data, defaultMaxDict))
	if !bytes.Equal(got, data) {
		t.Fatalf("binary nulls: got %v", got)
	}
}

func TestVeryLongInput(t *testing.T) {
	chunk := []byte("Hello, World! ")
	var data []byte
	for i := 0; i < 100; i++ {
		data = append(data, chunk...)
	}
	for i := 0; i < 256; i++ {
		data = append(data, byte(i))
	}
	got := lz78.Decompress(lz78.Compress(data, defaultMaxDict))
	if !bytes.Equal(got, data) {
		t.Fatal("very long input round-trip failed")
	}
}

func TestFullByteRange(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	tokens := lz78.Encode(data, defaultMaxDict)
	for _, tok := range tokens {
		if tok.DictIndex != 0 {
			t.Fatalf("full byte range (first pass): expected all literals, got DictIndex=%d", tok.DictIndex)
		}
	}
	got := lz78.Decompress(lz78.Compress(data, defaultMaxDict))
	if !bytes.Equal(got, data) {
		t.Fatal("full byte range round-trip failed")
	}
}

// ─── Serialisation tests ──────────────────────────────────────────────────────

func TestCompressFormatSize(t *testing.T) {
	data := []byte("AB")
	compressed := lz78.Compress(data, defaultMaxDict)
	tokens := lz78.Encode(data, defaultMaxDict)
	want := 8 + len(tokens)*4
	if len(compressed) != want {
		t.Fatalf("format size: want %d, got %d", want, len(compressed))
	}
}

func TestDecompressEmpty(t *testing.T) {
	got := lz78.Decompress(lz78.Compress(nil, defaultMaxDict))
	if len(got) != 0 {
		t.Fatalf("empty: want empty, got %v", got)
	}
}

func TestDeterministic(t *testing.T) {
	data := []byte("hello world test data repeated repeated")
	a := lz78.Compress(data, defaultMaxDict)
	b := lz78.Compress(data, defaultMaxDict)
	if !bytes.Equal(a, b) {
		t.Fatal("compression is not deterministic")
	}
}

// ─── Behaviour tests ──────────────────────────────────────────────────────────

func TestRepetitiveDataCompresses(t *testing.T) {
	chunk := []byte("ABC")
	var data []byte
	for i := 0; i < 1000; i++ {
		data = append(data, chunk...)
	}
	compressed := lz78.Compress(data, defaultMaxDict)
	if len(compressed) >= len(data) {
		t.Fatalf("expected compression: in=%d, out=%d", len(data), len(compressed))
	}
}

func TestAllSameByteCompresses(t *testing.T) {
	data := bytes.Repeat([]byte("A"), 10000)
	compressed := lz78.Compress(data, defaultMaxDict)
	if len(compressed) >= len(data) {
		t.Fatalf("expected compression: in=%d, out=%d", len(data), len(compressed))
	}
	got := lz78.Decompress(compressed)
	if !bytes.Equal(got, data) {
		t.Fatal("all-same-byte round-trip failed")
	}
}

func TestIncompressibleDataBound(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	compressed := lz78.Compress(data, defaultMaxDict)
	if len(compressed) > 4*len(data)+10 {
		t.Fatalf("incompressible expansion too large: %d > %d", len(compressed), 4*len(data)+10)
	}
}
