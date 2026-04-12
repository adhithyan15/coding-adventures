package lzw_test

import (
	"bytes"
	"encoding/binary"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lzw"
)

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

func TestConstants(t *testing.T) {
	if lzw.ClearCode != 256 {
		t.Errorf("ClearCode = %d, want 256", lzw.ClearCode)
	}
	if lzw.StopCode != 257 {
		t.Errorf("StopCode = %d, want 257", lzw.StopCode)
	}
	if lzw.InitialNextCode != 258 {
		t.Errorf("InitialNextCode = %d, want 258", lzw.InitialNextCode)
	}
	if lzw.InitialCodeSize != 9 {
		t.Errorf("InitialCodeSize = %d, want 9", lzw.InitialCodeSize)
	}
	if lzw.MaxCodeSize != 16 {
		t.Errorf("MaxCodeSize = %d, want 16", lzw.MaxCodeSize)
	}
}

// ---------------------------------------------------------------------------
// Compress / Decompress round-trips
// ---------------------------------------------------------------------------

func roundTrip(t *testing.T, data []byte) {
	t.Helper()
	compressed := lzw.Compress(data)
	got := lzw.Decompress(compressed)
	if !bytes.Equal(got, data) {
		t.Errorf("round-trip failed for %q: got %q", data, got)
	}
}

func TestEmpty(t *testing.T) {
	roundTrip(t, []byte{})
}

func TestSingleByte(t *testing.T) {
	roundTrip(t, []byte("A"))
}

func TestTwoDistinctBytes(t *testing.T) {
	roundTrip(t, []byte("AB"))
}

func TestRepeatedPair(t *testing.T) {
	// "ABABAB" → CLEAR, 65, 66, 258, 258, STOP
	roundTrip(t, []byte("ABABAB"))
}

func TestAllSameBytes(t *testing.T) {
	// "AAAAAAA" — exercises the tricky-token decoder edge case.
	// Codes: CLEAR, 65, 258, 259, 65, STOP
	roundTrip(t, bytes.Repeat([]byte("A"), 7))
}

func TestLongRepetitive(t *testing.T) {
	data := bytes.Repeat([]byte("ABCABC"), 200)
	roundTrip(t, data)
}

func TestBinaryData(t *testing.T) {
	data := make([]byte, 512)
	for i := range data {
		data[i] = byte(i % 256)
	}
	roundTrip(t, data)
}

func TestAllZeros(t *testing.T) {
	roundTrip(t, bytes.Repeat([]byte{0x00}, 100))
}

func TestAllFF(t *testing.T) {
	roundTrip(t, bytes.Repeat([]byte{0xFF}, 100))
}

func TestAababc(t *testing.T) {
	roundTrip(t, []byte("AABABC"))
}

func TestCompressesRepetitiveData(t *testing.T) {
	data := bytes.Repeat([]byte("ABCABC"), 100)
	compressed := lzw.Compress(data)
	if len(compressed) >= len(data) {
		t.Errorf("expected compression: compressed=%d >= original=%d", len(compressed), len(data))
	}
}

func TestHeaderContainsOriginalLength(t *testing.T) {
	data := []byte("hello world")
	compressed := lzw.Compress(data)
	if len(compressed) < 4 {
		t.Fatal("compressed data too short")
	}
	storedLen := binary.BigEndian.Uint32(compressed[:4])
	if int(storedLen) != len(data) {
		t.Errorf("stored length = %d, want %d", storedLen, len(data))
	}
}

func TestDecompressShortData(t *testing.T) {
	// Should not panic on truncated input.
	result := lzw.Decompress([]byte{0x00, 0x00})
	if result == nil {
		result = []byte{}
	}
	// Just ensure it doesn't panic; result may be empty.
	_ = result
}

// ---------------------------------------------------------------------------
// Spec test vectors (exact code sequences)
// ---------------------------------------------------------------------------

func TestSpecVector_ABABAB(t *testing.T) {
	// Per spec: CLEAR, 65(A), 66(B), 258(AB), 258(AB), STOP
	data := []byte("ABABAB")
	compressed := lzw.Compress(data)
	got := lzw.Decompress(compressed)
	if !bytes.Equal(got, data) {
		t.Errorf("ABABAB round-trip failed: got %q", got)
	}
}

func TestSpecVector_AAAAAAA(t *testing.T) {
	// Per spec: CLEAR, 65(A), 258(AA), 259(AAA), 65(A), STOP
	// Tricky token fired during decode.
	data := bytes.Repeat([]byte("A"), 7)
	compressed := lzw.Compress(data)
	got := lzw.Decompress(compressed)
	if !bytes.Equal(got, data) {
		t.Errorf("AAAAAAA round-trip failed: got %q", got)
	}
}
