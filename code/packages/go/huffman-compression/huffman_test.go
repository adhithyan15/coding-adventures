package huffman_test

import (
	"bytes"
	"encoding/binary"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/huffman-compression"
)

// ---------------------------------------------------------------------------
// Round-trip helpers
// ---------------------------------------------------------------------------

// roundTrip compresses data and decompresses the result, asserting equality.
func roundTrip(t *testing.T, name string, data []byte) {
	t.Helper()
	compressed, err := huffman.Compress(data)
	if err != nil {
		t.Fatalf("%s: Compress error: %v", name, err)
	}
	got, err := huffman.Decompress(compressed)
	if err != nil {
		t.Fatalf("%s: Decompress error: %v", name, err)
	}
	if !bytes.Equal(got, data) {
		t.Errorf("%s: round-trip mismatch\n  want: %q\n   got: %q", name, data, got)
	}
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

func TestRoundTrip_Empty(t *testing.T) {
	roundTrip(t, "empty", []byte{})
}

func TestRoundTrip_SingleByte(t *testing.T) {
	roundTrip(t, "single byte", []byte("A"))
}

func TestRoundTrip_TwoDistinctBytes(t *testing.T) {
	roundTrip(t, "two distinct bytes", []byte("AB"))
}

func TestRoundTrip_SingleRepeatedByte(t *testing.T) {
	// Single distinct symbol: Huffman assigns code "0" by convention.
	// Each 'A' encodes as one zero bit.
	roundTrip(t, "all same", bytes.Repeat([]byte("A"), 10))
}

func TestRoundTrip_AAABBC(t *testing.T) {
	roundTrip(t, "AAABBC", []byte("AAABBC"))
}

func TestRoundTrip_LongRepetitive(t *testing.T) {
	data := bytes.Repeat([]byte("ABCABC"), 200)
	roundTrip(t, "long repetitive", data)
}

func TestRoundTrip_BinaryData(t *testing.T) {
	// 512 bytes cycling through all byte values 0–255.
	data := make([]byte, 512)
	for i := range data {
		data[i] = byte(i % 256)
	}
	roundTrip(t, "binary 0-255 cycle", data)
}

func TestRoundTrip_AllZeros(t *testing.T) {
	roundTrip(t, "all zeros", bytes.Repeat([]byte{0x00}, 100))
}

func TestRoundTrip_AllFF(t *testing.T) {
	roundTrip(t, "all 0xFF", bytes.Repeat([]byte{0xFF}, 100))
}

func TestRoundTrip_All256Bytes(t *testing.T) {
	// Include every possible byte value exactly once. This exercises the full
	// 256-symbol Huffman tree and maximum code-length table size.
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	roundTrip(t, "all 256 byte values", data)
}

func TestRoundTrip_SkewedDistribution(t *testing.T) {
	// Very skewed: 'A' appears 1000×, 'B' once. A should get code "0" (1 bit).
	data := append(bytes.Repeat([]byte("A"), 1000), 'B')
	roundTrip(t, "skewed distribution", data)
}

func TestRoundTrip_HelloWorld(t *testing.T) {
	roundTrip(t, "hello world", []byte("hello, world!"))
}

func TestRoundTrip_LongString(t *testing.T) {
	data := bytes.Repeat([]byte("the quick brown fox jumps over the lazy dog "), 50)
	roundTrip(t, "long natural text", data)
}

// ---------------------------------------------------------------------------
// Wire-format verification for "AAABBC"
// ---------------------------------------------------------------------------
//
// Expected canonical codes (per DT27 spec):
//   A: freq=3 → code length 1, canonical code "0"
//   B: freq=2 → code length 2, canonical code "10"
//   C: freq=1 → code length 2, canonical code "11"
//
// Code-length table (sorted by code_length, symbol_value):
//   (65, 1), (66, 2), (67, 2)
//
// Encoding "AAABBC":
//   A  A  A  B   B   C
//   0  0  0  10  10  11
//   Bit stream (LSB-first packing):
//     Bit 0: 0 (A)
//     Bit 1: 0 (A)
//     Bit 2: 0 (A)
//     Bit 3: 0 (B bit 0)
//     Bit 4: 1 (B bit 1)
//     Bit 5: 0 (B bit 0)
//     Bit 6: 1 (B bit 1)
//     Bit 7: 1 (C bit 0)
//   → Byte 0: bits[0..7] = 0b10100000 = ???
//
// Let's compute more carefully, LSB-first:
//   Bit stream as a sequence: 0,0,0,1,0,1,0,1,1
//   Byte 0 (bits 0-7): bit0=0, bit1=0, bit2=0, bit3=1, bit4=0, bit5=1, bit6=0, bit7=1
//                       = 0b10100 1000 bit packed: (1<<3)|(1<<5)|(1<<7) = 8+32+128 = 168 = 0xA8
//   Byte 1 (bit 8):    bit0=1 = 0b00000001 = 0x01
//
// So bit stream = [0xA8, 0x01].

func TestWireFormat_AAABBC(t *testing.T) {
	data := []byte("AAABBC")
	compressed, err := huffman.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}

	// ── Header ──────────────────────────────────────────────────────────────
	if len(compressed) < 8 {
		t.Fatalf("compressed too short: %d bytes", len(compressed))
	}

	originalLength := binary.BigEndian.Uint32(compressed[0:4])
	symbolCount := binary.BigEndian.Uint32(compressed[4:8])

	if originalLength != 6 {
		t.Errorf("original_length = %d, want 6", originalLength)
	}
	if symbolCount != 3 {
		t.Errorf("symbol_count = %d, want 3", symbolCount)
	}

	// ── Code-length table ────────────────────────────────────────────────────
	// Expected: (65,1), (66,2), (67,2) — sorted by (code_length, symbol).
	tableEnd := 8 + int(symbolCount)*2
	if len(compressed) < tableEnd {
		t.Fatalf("compressed too short for table: %d bytes", len(compressed))
	}

	table := compressed[8:tableEnd]
	wantTable := []byte{
		65, 1, // A: code length 1
		66, 2, // B: code length 2
		67, 2, // C: code length 2
	}
	if !bytes.Equal(table, wantTable) {
		t.Errorf("code-length table = %v, want %v", table, wantTable)
	}

	// ── Bit stream ───────────────────────────────────────────────────────────
	// Expected: [0xA8, 0x01]
	bitStream := compressed[tableEnd:]
	wantBits := []byte{0xA8, 0x01}
	if !bytes.Equal(bitStream, wantBits) {
		t.Errorf("bit stream = %#v, want %#v", bitStream, wantBits)
	}
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

func TestCompress_Empty_HeaderOnly(t *testing.T) {
	// Empty input must produce exactly 8 bytes: header with all zeros.
	compressed, err := huffman.Compress([]byte{})
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}
	if len(compressed) != 8 {
		t.Errorf("empty compress: got %d bytes, want 8", len(compressed))
	}
	// Both uint32 fields should be zero.
	origLen := binary.BigEndian.Uint32(compressed[0:4])
	symCount := binary.BigEndian.Uint32(compressed[4:8])
	if origLen != 0 {
		t.Errorf("empty: original_length = %d, want 0", origLen)
	}
	if symCount != 0 {
		t.Errorf("empty: symbol_count = %d, want 0", symCount)
	}
}

func TestDecompress_Empty_HeaderOnly(t *testing.T) {
	// 8-byte all-zero header should decompress to empty slice.
	header := make([]byte, 8)
	got, err := huffman.Decompress(header)
	if err != nil {
		t.Fatalf("Decompress error: %v", err)
	}
	if len(got) != 0 {
		t.Errorf("expected empty result, got %d bytes", len(got))
	}
}

func TestDecompress_TooShort(t *testing.T) {
	// Less than 8 bytes: must return error, not panic.
	_, err := huffman.Decompress([]byte{0x00, 0x01})
	if err == nil {
		t.Error("expected error for too-short input, got nil")
	}
}

func TestDecompress_TruncatedTable(t *testing.T) {
	// Header claims 3 symbols but table is missing.
	buf := make([]byte, 8)
	binary.BigEndian.PutUint32(buf[0:4], 5) // original_length = 5
	binary.BigEndian.PutUint32(buf[4:8], 3) // symbol_count = 3 (needs 6 more bytes)
	// No table bytes follow — should return error.
	_, err := huffman.Decompress(buf)
	if err == nil {
		t.Error("expected error for truncated table, got nil")
	}
}

func TestSingleDistinctByte_CodeIsZero(t *testing.T) {
	// When there is only one distinct byte value, the tree has a single leaf.
	// DT27 assigns it code "0". Each occurrence should encode as 1 bit.
	data := bytes.Repeat([]byte{42}, 8)
	compressed, err := huffman.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}
	// Table should have exactly 1 entry.
	symCount := binary.BigEndian.Uint32(compressed[4:8])
	if symCount != 1 {
		t.Errorf("symbol_count = %d, want 1", symCount)
	}
	// Round-trip must succeed.
	roundTrip(t, "single distinct byte 42×8", data)
}

// ---------------------------------------------------------------------------
// Compression effectiveness
// ---------------------------------------------------------------------------

func TestCompressionEffectiveness_SkewedDistribution(t *testing.T) {
	// With a highly skewed distribution the most frequent symbol gets just 1
	// bit. Total compressed size should be much smaller than original.
	data := append(
		bytes.Repeat([]byte("A"), 900),
		bytes.Repeat([]byte("B"), 80)...,
	)
	data = append(data, bytes.Repeat([]byte("C"), 20)...)

	compressed, err := huffman.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}
	// Overhead is headerSize + 3×2 = 14 bytes. Payload ≈ 900/8 + bits for B,C.
	// Original is 1000 bytes — compression should be at least 10% smaller.
	if len(compressed) >= len(data) {
		t.Errorf("expected compression: compressed=%d bytes, original=%d bytes",
			len(compressed), len(data))
	}
}
