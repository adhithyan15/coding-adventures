package brotli_test

// Tests for CMP06 Brotli compression and decompression.
//
// Test strategy:
//   - Every round-trip test calls Compress then Decompress and verifies
//     the result is byte-for-byte identical to the original.
//   - Some tests also verify properties of the compressed output
//     (e.g., compression ratio, header bytes).
//   - Tests are derived directly from the CMP06 specification test cases.

import (
	"bytes"
	"encoding/binary"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/brotli"
)

// roundtrip is a shared helper: compress data then decompress and verify equality.
//
// Using t.Helper() ensures that failure messages report the calling test's
// location rather than this function's location.
func roundtrip(t *testing.T, data []byte) {
	t.Helper()
	compressed, err := brotli.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}
	decompressed, err := brotli.Decompress(compressed)
	if err != nil {
		t.Fatalf("Decompress error: %v", err)
	}
	if !bytes.Equal(decompressed, data) {
		t.Errorf("round-trip mismatch:\n  want len=%d, got len=%d", len(data), len(decompressed))
		if len(data) <= 64 {
			t.Errorf("  want: %q\n   got: %q", data, decompressed)
		}
	}
}

// ---------------------------------------------------------------------------
// Spec test case 1: Round-trip empty input
// ---------------------------------------------------------------------------

// TestSpecEmpty verifies that compressing and decompressing an empty byte
// slice produces an empty byte slice.
//
// The spec defines a special encoding for empty input:
//   - Header: [0x00000000][0x01][0x00][0x00][0x00][0x00][0x00]
//   - ICC table: 1 entry (sentinel 63, length 1)
//   - Bit stream: 0x00 (the single "0" bit for sentinel, padded)
func TestSpecEmpty(t *testing.T) {
	compressed, err := brotli.Compress(nil)
	if err != nil {
		t.Fatalf("Compress(nil) error: %v", err)
	}
	if len(compressed) == 0 {
		t.Fatal("expected non-empty compressed output even for empty input")
	}

	// Verify the original_length field in the header is 0.
	origLen := int(binary.BigEndian.Uint32(compressed[0:4]))
	if origLen != 0 {
		t.Errorf("header original_length: want 0, got %d", origLen)
	}

	decompressed, err := brotli.Decompress(compressed)
	if err != nil {
		t.Fatalf("Decompress error: %v", err)
	}
	if len(decompressed) != 0 {
		t.Errorf("expected empty output, got %q", decompressed)
	}
}

// ---------------------------------------------------------------------------
// Spec test case 2: Round-trip single byte
// ---------------------------------------------------------------------------

// TestSpecSingleByte verifies that individual byte values survive round-trip.
//
// Single-byte input has no possible LZ matches, so it encodes as a literal
// in context bucket 0 (no preceding byte).  The compressed output will be
// larger than 1 byte due to header and Huffman table overhead.
func TestSpecSingleByte(t *testing.T) {
	roundtrip(t, []byte{0x00})
	roundtrip(t, []byte{0xFF})
	roundtrip(t, []byte{0x42}) // 'B'
	roundtrip(t, []byte("A"))
	roundtrip(t, []byte("\n"))
}

// ---------------------------------------------------------------------------
// Spec test case 3: Round-trip all 256 distinct bytes
// ---------------------------------------------------------------------------

// TestSpecAllBytes verifies that all 256 byte values survive round-trip.
//
// Random/incompressible data (all distinct bytes) will produce output LARGER
// than the input because the overhead of the header and Huffman tables
// exceeds any compression gain.  The spec allows this — correctness matters,
// not ratio, for incompressible data.
func TestSpecAllBytes(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	compressed, err := brotli.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}
	// Per spec: output may be larger than input for incompressible data.
	// We only require round-trip correctness.
	decompressed, err := brotli.Decompress(compressed)
	if err != nil {
		t.Fatalf("Decompress error: %v", err)
	}
	if !bytes.Equal(decompressed, data) {
		t.Error("all-bytes round-trip mismatch")
	}
}

// ---------------------------------------------------------------------------
// Spec test case 4: Round-trip 1024 × 'A' (all copies, no leading literals)
// ---------------------------------------------------------------------------

// TestSpecRepeatedByte verifies that highly compressible data (1024 copies
// of 'A') round-trips correctly.
//
// The encoder must emit "AAAA" as 4 literals (since the window is empty at
// the start), then one or more copy commands covering the remaining 1020 'A's.
// Total compressed output should be much smaller than 1024 bytes.
func TestSpecRepeatedByte(t *testing.T) {
	data := bytes.Repeat([]byte("A"), 1024)
	compressed, err := brotli.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}

	// Verify the compressed output is significantly smaller than input.
	// 1024 'A's should compress to a tiny fraction of original size.
	if len(compressed) >= len(data) {
		t.Errorf("expected compression: compressed=%d, original=%d", len(compressed), len(data))
	}

	decompressed, err := brotli.Decompress(compressed)
	if err != nil {
		t.Fatalf("Decompress error: %v", err)
	}
	if !bytes.Equal(decompressed, data) {
		t.Errorf("1024×'A' round-trip mismatch: want len=1024, got len=%d", len(decompressed))
	}
}

// ---------------------------------------------------------------------------
// Spec test case 5: Round-trip English prose ≥ 1024 bytes
// ---------------------------------------------------------------------------

// TestSpecEnglishProse verifies compression ratio on typical ASCII text.
//
// The spec requires: compressed size < 80% of input size.
// This is achievable on English prose because:
//   - Context modeling gives each context bucket a narrow probability
//     distribution (e.g., after 't' we almost always see 'h', 'o', 'r', ...)
//   - LZ matching finds repeated phrases like "the", "of", "and"
//   - Together these produce much shorter codes than a single Huffman tree
const englishProse = `The quick brown fox jumps over the lazy dog.
Pack my box with five dozen liquor jugs.
How vexingly quick daft zebras jump!
The five boxing wizards jump quickly.
Sphinx of black quartz, judge my vow.
Two driven jocks help fax my big quiz.
Five quacking zephyrs jolt my wax bed.
The jay, pig, fox, zebra and my wolves quack!
Blowzy red vixens fight for a quick jump.
Joaquin Phoenix was gazed by the brown fox quickly jumping over lazy dogs.
The complexity of Brotli compression comes from context modeling and insert-copy commands.
Context modeling assigns each literal to one of four buckets based on the preceding byte.
After a space or punctuation mark, English typically starts a new word with a consonant.
After a lowercase letter, another lowercase letter is most likely to follow in normal text.
After a digit, another digit or punctuation typically follows in numeric contexts.
This is why context modeling improves compression ratio significantly over DEFLATE alone.
Insert-and-copy commands bundle a literal run with a back-reference into a single symbol.
This reduces the overhead compared to DEFLATEs separate literal and match token streams.
The sliding window in CMP06 is 65535 bytes, compared to 4096 in DEFLATE (CMP05).
Larger windows allow matching repeated phrases across longer distances in documents.`

func TestSpecEnglishProse(t *testing.T) {
	data := []byte(englishProse)
	if len(data) < 1024 {
		t.Fatalf("test text too short: %d bytes (need ≥ 1024)", len(data))
	}

	compressed, err := brotli.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}

	// Spec requirement: compressed < 80% of input.
	threshold := len(data) * 80 / 100
	if len(compressed) >= threshold {
		t.Errorf("insufficient compression ratio: compressed=%d, threshold=%d (80%% of %d)",
			len(compressed), threshold, len(data))
	}

	decompressed, err := brotli.Decompress(compressed)
	if err != nil {
		t.Fatalf("Decompress error: %v", err)
	}
	if !bytes.Equal(decompressed, data) {
		t.Error("English prose round-trip mismatch")
	}
}

// ---------------------------------------------------------------------------
// Spec test case 6: Round-trip 512 random-ish bytes (deterministic seed)
// ---------------------------------------------------------------------------

// TestSpecBinaryBlob verifies that binary (incompressible) data round-trips
// correctly.  We use a deterministic "random" sequence via a simple LCG
// (Linear Congruential Generator) to avoid importing math/rand.
//
// No compression ratio requirement is imposed — random data is incompressible.
func TestSpecBinaryBlob(t *testing.T) {
	// Generate 512 pseudo-random bytes with a fixed seed (LCG: x = 1664525*x + 1013904223).
	data := make([]byte, 512)
	state := uint32(0xDEADBEEF)
	for i := range data {
		state = state*1664525 + 1013904223
		data[i] = byte(state >> 24)
	}
	roundtrip(t, data)
}

// ---------------------------------------------------------------------------
// Spec test case 7: Context transitions in "abc123ABC"
// ---------------------------------------------------------------------------

// TestSpecContextTransitions verifies that context bucket changes mid-stream
// are handled correctly.
//
// In "abc123ABC":
//   - 'a' appears in ctx 0 (no previous byte at start)
//   - 'b', 'c' appear in ctx 3 (after lowercase)
//   - '1' appears in ctx 3 (after 'c', lowercase)
//   - '2', '3' appear in ctx 1 (after digit)
//   - 'A' appears in ctx 1 (after '3', digit)
//   - 'B', 'C' appear in ctx 2 (after uppercase)
func TestSpecContextTransitions(t *testing.T) {
	data := []byte("abc123ABC")
	roundtrip(t, data)
}

// TestContextTransitionsExtended tests longer sequences with all four context
// buckets explicitly exercised.
func TestContextTransitionsExtended(t *testing.T) {
	// This sequence exercises:
	//   ctx 0: start, after space ' ', after '!'
	//   ctx 1: after digits
	//   ctx 2: after uppercase
	//   ctx 3: after lowercase
	roundtrip(t, []byte("Hello World! 123abc DEF ghi"))
	roundtrip(t, []byte("abc123ABC xyz 456 DEF"))
}

// ---------------------------------------------------------------------------
// Spec test case 8: Long-distance match (offset > 4096)
// ---------------------------------------------------------------------------

// TestSpecLongDistanceMatch verifies that distance codes 24–31 (offsets 4097–65535)
// work correctly.
//
// We create input where a 10-byte sequence is repeated with a gap of > 4096
// bytes between them.  This forces the encoder to use one of the extended
// distance codes (codes 24–31) that are absent in CMP05/DEFLATE.
func TestSpecLongDistanceMatch(t *testing.T) {
	// Pattern: "XYZXYZXYZX" + 5000 bytes of filler + "XYZXYZXYZX"
	// The second occurrence of the pattern is offset > 5000 bytes from the first.
	pattern := []byte("XYZXYZXYZX")
	filler := bytes.Repeat([]byte("abcdefghij"), 500) // 5000 bytes
	data := make([]byte, 0, len(pattern)+len(filler)+len(pattern))
	data = append(data, pattern...)
	data = append(data, filler...)
	data = append(data, pattern...)

	roundtrip(t, data)

	// Additionally verify it works for offsets just beyond code 23's range (4096).
	// Build: 10-byte pattern + exactly 4087 bytes of filler + same 10-byte pattern.
	// Total gap = 4097 bytes → needs dist code ≥ 24.
	filler2 := bytes.Repeat([]byte("Z"), 4087)
	data2 := make([]byte, 0, len(pattern)+len(filler2)+len(pattern))
	data2 = append(data2, pattern...)
	data2 = append(data2, filler2...)
	data2 = append(data2, pattern...)

	roundtrip(t, data2)
}

// ---------------------------------------------------------------------------
// Spec test case 9: Wire format verification
// ---------------------------------------------------------------------------

// TestSpecWireFormat verifies the structure of the 10-byte header.
//
// The spec defines:
//   Bytes 0–3: original_length (big-endian uint32)
//   Byte  4:   icc_entry_count
//   Byte  5:   dist_entry_count
//   Bytes 6–9: ctx0–ctx3 entry counts
func TestSpecWireFormat(t *testing.T) {
	data := []byte("hello world hello world")
	compressed, err := brotli.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}

	if len(compressed) < 10 {
		t.Fatalf("compressed output too short for header: %d bytes", len(compressed))
	}

	// Verify original_length field.
	origLen := int(binary.BigEndian.Uint32(compressed[0:4]))
	if origLen != len(data) {
		t.Errorf("header original_length: want %d, got %d", len(data), origLen)
	}

	// icc_entry_count must be ≥ 1 (at least sentinel ICC 63).
	iccCount := int(compressed[4])
	if iccCount == 0 {
		t.Error("icc_entry_count must be ≥ 1")
	}

	// This input has a match ("hello world" repeated), so dist_entry_count > 0.
	distCount := int(compressed[5])
	if distCount == 0 {
		t.Error("dist_entry_count must be > 0 for input with matches")
	}

	// Literal entry counts: at least one context bucket must be non-zero.
	totalLit := int(compressed[6]) + int(compressed[7]) + int(compressed[8]) + int(compressed[9])
	if totalLit == 0 {
		t.Error("total literal entries must be > 0 for non-empty input")
	}

	// Decompress to verify the payload is valid.
	decompressed, err := brotli.Decompress(compressed)
	if err != nil {
		t.Fatalf("Decompress error: %v", err)
	}
	if !bytes.Equal(decompressed, data) {
		t.Error("wire-format round-trip mismatch")
	}
}

// TestWireFormatEmptyInput directly tests the spec-mandated empty input encoding.
//
// Per spec:
//   Header: [0x00000000][0x01][0x00][0x00][0x00][0x00][0x00]
//   ICC table: symbol=63, code_length=1
//   Bit stream: 0x00
func TestWireFormatEmptyInput(t *testing.T) {
	compressed, err := brotli.Compress([]byte{})
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}

	// Expected: 10 header + 2 ICC entry + 1 bit stream byte = 13 bytes.
	if len(compressed) != 13 {
		t.Errorf("empty input compressed length: want 13, got %d", len(compressed))
	}

	// Header bytes 0–3: original_length = 0.
	if compressed[0] != 0 || compressed[1] != 0 || compressed[2] != 0 || compressed[3] != 0 {
		t.Errorf("original_length bytes should be zero, got %v", compressed[0:4])
	}

	// Byte 4: icc_entry_count = 1.
	if compressed[4] != 1 {
		t.Errorf("icc_entry_count: want 1, got %d", compressed[4])
	}

	// Byte 5: dist_entry_count = 0.
	if compressed[5] != 0 {
		t.Errorf("dist_entry_count: want 0, got %d", compressed[5])
	}

	// ICC entry: symbol=63, code_length=1.
	if compressed[10] != 63 {
		t.Errorf("ICC sentinel symbol: want 63, got %d", compressed[10])
	}
	if compressed[11] != 1 {
		t.Errorf("ICC sentinel code_length: want 1, got %d", compressed[11])
	}

	// Bit stream: 0x00.
	if compressed[12] != 0x00 {
		t.Errorf("bit stream: want 0x00, got 0x%02x", compressed[12])
	}
}

// ---------------------------------------------------------------------------
// Additional round-trip tests
// ---------------------------------------------------------------------------

// TestRoundTripVariety exercises many different data shapes for completeness.
func TestRoundTripVariety(t *testing.T) {
	tests := []struct {
		name string
		data []byte
	}{
		{"zeros_100", make([]byte, 100)},
		{"ff_100", bytes.Repeat([]byte{0xFF}, 100)},
		{"alphabet", []byte("abcdefghijklmnopqrstuvwxyz")},
		{"digits", []byte("0123456789")},
		{"uppercase", []byte("ABCDEFGHIJKLMNOPQRSTUVWXYZ")},
		{"mixed_ascii", []byte("Hello, World! 123 ABC xyz.")},
		{"repeated_phrase", bytes.Repeat([]byte("The quick brown fox "), 50)},
		{"json_like", []byte(`{"key":"value","n":42,"arr":[1,2,3],"nested":{"a":true}}`)},
		{"html_like", bytes.Repeat([]byte("<div class=\"container\"><p>Hello</p></div>"), 20)},
		{"binary_00_ff", func() []byte {
			d := make([]byte, 512)
			for i := range d {
				d[i] = byte(i % 256)
			}
			return d
		}()},
		{"two_bytes_repeated", bytes.Repeat([]byte("AB"), 200)},
		{"three_bytes_repeated", bytes.Repeat([]byte("ABC"), 200)},
		{"newlines", bytes.Repeat([]byte("line\n"), 100)},
		{"tabs_spaces", bytes.Repeat([]byte("\t  text  \n"), 50)},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			roundtrip(t, tt.data)
		})
	}
}

// TestRoundTripMatchLengths exercises different match lengths to cover the
// full ICC copy range.
//
// Each ICC group covers a range of copy lengths.  By testing boundary values,
// we verify that the encoder correctly selects the ICC code and emits the
// right extra bits.
func TestRoundTripMatchLengths(t *testing.T) {
	// Test copy lengths that hit different ICC entries:
	// 4, 5, 6, 8–9, 10–11, 14–17, 18–25, 26–33, 34–49, 50–65, 66–97...
	for _, copyLen := range []int{4, 5, 6, 8, 9, 10, 11, 14, 17, 18, 25, 26, 33, 34, 49, 50, 65, 66, 97, 98, 129, 130, 193, 194, 257, 258} {
		// Build data: prefix of 'A's followed by 'B' followed by same prefix.
		// This forces a match of exactly copyLen bytes.
		prefix := bytes.Repeat([]byte("A"), copyLen)
		data := make([]byte, 0, copyLen*2+1)
		data = append(data, prefix...)
		data = append(data, 'B')
		data = append(data, prefix...)

		t.Run("copy_len_"+string(rune('0'+copyLen/100))+string(rune('0'+(copyLen/10)%10))+string(rune('0'+copyLen%10)),
			func(t *testing.T) {
				roundtrip(t, data)
			})
	}
}

// TestRoundTripDistanceCodes exercises distance codes, including the extended
// codes 24–31 that are new in CMP06.
func TestRoundTripDistanceCodes(t *testing.T) {
	// Test offsets that hit different distance code ranges.
	// Each distance code covers a range; we test at the boundary of each.
	offsets := []int{
		1, 2, 3, 4, // codes 0–3 (exact)
		5, 6,       // code 4 (extra 1 bit)
		9, 12,      // code 6
		65, 96,     // code 12
		257, 384,   // code 16
		513, 768,   // code 18
		1025, 1536, // code 20
		2049, 3072, // code 22
		4097, 6144, // code 24 (new in CMP06!)
		8193, 12288, // code 26
		16385, 24576, // code 28
		32769, 49152, // code 30
	}

	for _, offset := range offsets {
		// Build data: 10-byte pattern + (offset-10) filler bytes + same 10-byte pattern.
		// Use a unique pattern to ensure the LZ matcher finds it.
		pattern := []byte("XYZABCDEFG")
		padLen := offset - len(pattern)
		if padLen < 0 {
			padLen = 0
		}
		filler := bytes.Repeat([]byte{byte(offset % 97)}, padLen)
		data := make([]byte, 0, len(pattern)+len(filler)+len(pattern))
		data = append(data, pattern...)
		data = append(data, filler...)
		data = append(data, pattern...)

		// Only test if the resulting data is reasonable in size.
		if len(data) > 70000 {
			continue
		}

		t.Run("offset_"+string(rune('0'+offset/10000))+string(rune('0'+(offset/1000)%10)), func(t *testing.T) {
			roundtrip(t, data)
		})
	}
}

// TestOverlappingCopy verifies that overlapping copies (distance < length)
// work correctly.  These produce repeating patterns and require byte-by-byte
// copy rather than bulk memcopy.
//
// Example: output is "AAAAAA" (6 'A's), distance=1, copy_length=5.
// Byte-by-byte: output[n] = output[n-1] each time → all 'A'.
func TestOverlappingCopy(t *testing.T) {
	// 4 'A's (literals) then many more 'A's (overlapping copies).
	roundtrip(t, bytes.Repeat([]byte("A"), 50))
	roundtrip(t, bytes.Repeat([]byte("AB"), 50))
	roundtrip(t, bytes.Repeat([]byte("ABCD"), 50))
}

// TestInsertLengthGroups tests large insert runs to exercise ICC codes with
// larger insert bases (codes 32–62).
func TestInsertLengthGroups(t *testing.T) {
	// Build data where there are many literals before a copy.
	// Pattern: N random-ish literals + "MATCHME12" repeated.
	for _, insertLen := range []int{3, 4, 5, 6, 7, 8, 9, 10, 17, 18} {
		prefix := make([]byte, insertLen)
		for i := range prefix {
			prefix[i] = byte('a' + i%26)
		}
		pattern := []byte("MATCHME12")
		data := make([]byte, 0, insertLen+len(pattern)*3)
		data = append(data, prefix...)
		data = append(data, pattern...)
		data = append(data, pattern...)
		data = append(data, pattern...)

		t.Run("insert_"+string(rune('0'+insertLen/10))+string(rune('0'+insertLen%10)), func(t *testing.T) {
			roundtrip(t, data)
		})
	}
}

// TestCompressionRatioRepetitive verifies that highly repetitive input
// achieves significant compression.
func TestCompressionRatioRepetitive(t *testing.T) {
	// "ABCABC..." 600 times = 3600 bytes should compress to < 50%.
	data := bytes.Repeat([]byte("ABCABC"), 600)
	compressed, err := brotli.Compress(data)
	if err != nil {
		t.Fatalf("Compress error: %v", err)
	}
	threshold := len(data) / 2
	if len(compressed) >= threshold {
		t.Errorf("expected compression: compressed=%d, original=%d, threshold=%d",
			len(compressed), len(data), threshold)
	}
	roundtrip(t, data)
}

// TestNilAndEmpty verify both nil and []byte{} produce valid empty output.
func TestNilAndEmpty(t *testing.T) {
	roundtrip(t, nil)
	roundtrip(t, []byte{})
}

// TestSingleByteAllValues verifies all 256 single-byte values round-trip.
func TestSingleByteAllValues(t *testing.T) {
	for i := 0; i < 256; i++ {
		data := []byte{byte(i)}
		compressed, err := brotli.Compress(data)
		if err != nil {
			t.Errorf("Compress(%d) error: %v", i, err)
			continue
		}
		decompressed, err := brotli.Decompress(compressed)
		if err != nil {
			t.Errorf("Decompress(%d) error: %v", i, err)
			continue
		}
		if !bytes.Equal(decompressed, data) {
			t.Errorf("byte %d round-trip failed", i)
		}
	}
}

// TestLargeInput verifies that inputs larger than the ICC max match length
// are handled correctly (the encoder may need multiple copy commands).
func TestLargeInput(t *testing.T) {
	// 10000 'A's — requires multiple copy commands since max copy = 258.
	data := bytes.Repeat([]byte("A"), 10000)
	roundtrip(t, data)
}

// TestMultipleMatches exercises inputs where many LZ matches occur.
func TestMultipleMatches(t *testing.T) {
	roundtrip(t, []byte("ABCABCABCABC"))
	roundtrip(t, []byte("hello hello hello world world"))
	roundtrip(t, bytes.Repeat([]byte("the quick brown fox "), 10))
}
