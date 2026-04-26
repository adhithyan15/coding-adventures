package zstd

// Tests for the zstd package — CMP07.
//
// Test numbering follows the Rust reference:
//   TC-1  Empty input
//   TC-2  Single byte
//   TC-3  All 256 byte values
//   TC-4  RLE block (1024 identical bytes)
//   TC-5  English prose (strong LZ77 matches)
//   TC-6  Pseudo-random data (no useful compression)
//   TC-7  200 KB single-byte run (multi-block)
//   TC-8  Repeat-offset alternating pattern
//   TC-9  Deterministic output
//   TC-10 Wire-format validation (manual frame)
//
// Plus unit tests for each internal helper, ensuring high coverage of
// every codepath including the FSE codec, bit writer/reader, and the
// literals/sequences section encode/decode.

import (
	"math/bits"
	"testing"
)

// ─── Helpers ──────────────────────────────────────────────────────────────────

// roundTrip compresses data and immediately decompresses it,
// returning the decompressed bytes. Fails the test on any error.
func roundTrip(t *testing.T, data []byte) []byte {
	t.Helper()
	compressed := Compress(data)
	got, err := Decompress(compressed)
	if err != nil {
		t.Fatalf("round-trip failed: %v", err)
	}
	return got
}

// assertBytes fails with a useful diff if got != want.
func assertBytes(t *testing.T, got, want []byte, label string) {
	t.Helper()
	if len(got) != len(want) {
		t.Fatalf("%s: length mismatch: got %d, want %d", label, len(got), len(want))
	}
	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("%s: byte[%d] = %02x, want %02x", label, i, got[i], want[i])
		}
	}
}

// ─── TC-1: empty input ────────────────────────────────────────────────────────

// TestTC1Empty verifies that an empty input produces a valid ZStd frame
// and decompresses back to empty bytes without panic or error.
func TestTC1Empty(t *testing.T) {
	got := roundTrip(t, []byte{})
	assertBytes(t, got, []byte{}, "empty round-trip")
}

// ─── TC-2: single byte ────────────────────────────────────────────────────────

// TestTC2SingleByte verifies the smallest non-empty input: one byte.
func TestTC2SingleByte(t *testing.T) {
	input := []byte{0x42}
	got := roundTrip(t, input)
	assertBytes(t, got, input, "single byte")
}

// ─── TC-3: all 256 byte values ────────────────────────────────────────────────

// TestTC3AllBytes exercises literal encoding of every possible byte value
// (0x00 through 0xFF), including zero bytes and high-value bytes.
func TestTC3AllBytes(t *testing.T) {
	input := make([]byte, 256)
	for i := range input {
		input[i] = byte(i)
	}
	got := roundTrip(t, input)
	assertBytes(t, got, input, "all 256 bytes")
}

// ─── TC-4: RLE block ──────────────────────────────────────────────────────────

// TestTC4RLE verifies that 1024 identical bytes are detected as an RLE block.
// Expected compressed size: 4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header)
// + 1 (RLE byte) = 17 bytes < 30.
func TestTC4RLE(t *testing.T) {
	input := make([]byte, 1024)
	for i := range input {
		input[i] = 'A'
	}
	compressed := Compress(input)
	got, err := Decompress(compressed)
	if err != nil {
		t.Fatalf("decompress failed: %v", err)
	}
	assertBytes(t, got, input, "RLE round-trip")
	if len(compressed) >= 30 {
		t.Errorf("RLE of 1024 bytes compressed to %d (expected < 30)", len(compressed))
	}
}

// ─── TC-5: English prose ──────────────────────────────────────────────────────

// TestTC5Prose checks that repeated English text achieves at least 20%
// compression (output ≤ 80% of input size). Repeated text has strong LZ77
// back-reference opportunities.
func TestTC5Prose(t *testing.T) {
	phrase := "the quick brown fox jumps over the lazy dog "
	text := ""
	for i := 0; i < 25; i++ {
		text += phrase
	}
	input := []byte(text)
	compressed := Compress(input)
	got, err := Decompress(compressed)
	if err != nil {
		t.Fatalf("decompress failed: %v", err)
	}
	assertBytes(t, got, input, "prose round-trip")
	threshold := len(input) * 80 / 100
	if len(compressed) >= threshold {
		t.Errorf("prose: compressed %d bytes (input %d), expected < %d (80%%)",
			len(compressed), len(input), threshold)
	}
}

// ─── TC-6: pseudo-random data ─────────────────────────────────────────────────

// TestTC6Random verifies that LCG pseudo-random bytes round-trip exactly,
// even though no significant compression is expected. The compressor must
// fall back to raw blocks gracefully.
func TestTC6Random(t *testing.T) {
	seed := uint32(42)
	input := make([]byte, 512)
	for i := range input {
		seed = seed*1664525 + 1013904223
		input[i] = byte(seed & 0xFF)
	}
	got := roundTrip(t, input)
	assertBytes(t, got, input, "random round-trip")
}

// ─── TC-7: 200 KB single-byte run (multi-block) ───────────────────────────────

// TestTC7MultiBlock verifies that 200 KB > maxBlockSize (128 KB) is handled
// correctly by splitting into multiple blocks. Both should be RLE blocks since
// all bytes are identical.
func TestTC7MultiBlock(t *testing.T) {
	input := make([]byte, 200*1024)
	for i := range input {
		input[i] = 'x'
	}
	got := roundTrip(t, input)
	assertBytes(t, got, input, "multiblock round-trip")
}

// ─── TC-8: repeat-offset pattern ─────────────────────────────────────────────

// TestTC8RepeatOffset checks that an alternating pattern with long runs of 'X'
// and repeated "ABCDEFGH" achieves at least 30% compression (≤ 70% of input).
// Both the X runs and the pattern repetitions give strong LZ77 matches.
func TestTC8RepeatOffset(t *testing.T) {
	pattern := []byte("ABCDEFGH")
	input := make([]byte, 0, len(pattern)+10*(128+len(pattern)))
	input = append(input, pattern...)
	for i := 0; i < 10; i++ {
		for j := 0; j < 128; j++ {
			input = append(input, 'X')
		}
		input = append(input, pattern...)
	}
	compressed := Compress(input)
	got, err := Decompress(compressed)
	if err != nil {
		t.Fatalf("decompress failed: %v", err)
	}
	assertBytes(t, got, input, "repeat-offset round-trip")
	threshold := len(input) * 70 / 100
	if len(compressed) >= threshold {
		t.Errorf("repeat-offset: compressed %d (input %d), expected < %d (70%%)",
			len(compressed), len(input), threshold)
	}
}

// ─── TC-9: deterministic output ───────────────────────────────────────────────

// TestTC9Deterministic verifies that compressing the same data twice produces
// identical bytes. This is required for reproducible builds and cache invalidation.
func TestTC9Deterministic(t *testing.T) {
	data := []byte{}
	for i := 0; i < 50; i++ {
		data = append(data, []byte("hello, ZStd world! ")...)
	}
	c1 := Compress(data)
	c2 := Compress(data)
	assertBytes(t, c1, c2, "deterministic compress")
}

// ─── TC-10: wire-format validation ────────────────────────────────────────────

// TestTC10WireFormat tests that our decoder reads the wire format correctly
// without depending on our encoder at all. The frame is manually constructed.
//
// Frame layout:
//
//	[0..3]  Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
//	[4]     FHD = 0x20:
//	          bits [7:6] = 00 → FCS flag 0
//	          bit  [5]   = 1  → Single_Segment = 1
//	          bits [4:0] = 0  → no checksum, no dict
//	        With Single_Segment=1 and FCS_flag=00, FCS is 1 byte.
//	[5]     FCS = 0x05 (content_size = 5)
//	[6..8]  Block header: Last=1, Type=Raw, Size=5
//	          = (5 << 3) | (0 << 1) | 1 = 41 = 0x29
//	          = [0x29, 0x00, 0x00]
//	[9..13] b"hello"
func TestTC10WireFormat(t *testing.T) {
	frame := []byte{
		0x28, 0xB5, 0x2F, 0xFD, // magic
		0x20,                   // FHD: Single_Segment=1, FCS=1byte
		0x05,                   // FCS = 5
		0x29, 0x00, 0x00,       // block header: last=1, raw, size=5
		'h', 'e', 'l', 'l', 'o',
	}
	got, err := Decompress(frame)
	if err != nil {
		t.Fatalf("Decompress failed: %v", err)
	}
	assertBytes(t, got, []byte("hello"), "wire-format")
}

// ─── Additional round-trip tests ──────────────────────────────────────────────

// TestRTBinaryData verifies round-trip for binary data with repeating byte values
// including zeros and 0xFF.
func TestRTBinaryData(t *testing.T) {
	input := make([]byte, 300)
	for i := range input {
		input[i] = byte(i % 256)
	}
	got := roundTrip(t, input)
	assertBytes(t, got, input, "binary data")
}

// TestRTAllZeros verifies round-trip for a buffer of 1000 zero bytes.
func TestRTAllZeros(t *testing.T) {
	input := make([]byte, 1000)
	got := roundTrip(t, input)
	assertBytes(t, got, input, "all zeros")
}

// TestRTAllFF verifies round-trip for a buffer of 1000 0xFF bytes.
func TestRTAllFF(t *testing.T) {
	input := make([]byte, 1000)
	for i := range input {
		input[i] = 0xFF
	}
	got := roundTrip(t, input)
	assertBytes(t, got, input, "all 0xFF")
}

// TestRTHelloWorld verifies the most basic hello-world case.
func TestRTHelloWorld(t *testing.T) {
	input := []byte("hello world")
	got := roundTrip(t, input)
	assertBytes(t, got, input, "hello world")
}

// TestRTRepeatedPattern verifies a repeating 6-byte pattern of 3000 bytes.
func TestRTRepeatedPattern(t *testing.T) {
	pat := []byte("ABCDEF")
	input := make([]byte, 0, 3000)
	for len(input) < 3000 {
		input = append(input, pat...)
	}
	input = input[:3000]
	got := roundTrip(t, input)
	assertBytes(t, got, input, "repeated pattern")
}

// ─── Error path tests ─────────────────────────────────────────────────────────

// TestDecompressBadMagic verifies that a wrong magic number produces an error.
func TestDecompressBadMagic(t *testing.T) {
	// Valid ZStd header but wrong magic.
	bad := []byte{0x00, 0x00, 0x00, 0x00, 0xE0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}
	_, err := Decompress(bad)
	if err == nil {
		t.Fatal("expected error for bad magic, got nil")
	}
}

// TestDecompressTooShort verifies that a truncated input (< 5 bytes) is rejected.
func TestDecompressTooShort(t *testing.T) {
	_, err := Decompress([]byte{0x28, 0xB5})
	if err == nil {
		t.Fatal("expected error for truncated input, got nil")
	}
}

// TestDecompressTruncatedBlock verifies that a truncated block header is caught.
func TestDecompressTruncatedBlock(t *testing.T) {
	// Valid magic + FHD + FCS, but no block header.
	data := []byte{
		0x28, 0xB5, 0x2F, 0xFD, // magic
		0x60,                   // FHD: FCS=8bytes, Single_Segment=1
		0, 0, 0, 0, 0, 0, 0, 0, // FCS (8 bytes)
		// No block header follows.
	}
	_, err := Decompress(data)
	if err == nil {
		t.Fatal("expected error for truncated block header, got nil")
	}
}

// ─── Unit tests for internal helpers ──────────────────────────────────────────

// TestLLToCodeSmall verifies that LL codes 0..15 are identity mappings.
func TestLLToCodeSmall(t *testing.T) {
	for i := 0; i < 16; i++ {
		got := llToCode(uint32(i))
		if got != i {
			t.Errorf("llToCode(%d) = %d, want %d", i, got, i)
		}
	}
}

// TestMLToCodeSmall verifies that ML codes for match lengths 3..34 are sequential.
func TestMLToCodeSmall(t *testing.T) {
	for i := 3; i < 35; i++ {
		got := mlToCode(uint32(i))
		want := i - 3
		if got != want {
			t.Errorf("mlToCode(%d) = %d, want %d", i, got, want)
		}
	}
}

// TestLiteralsRoundtripShort verifies encode/decode of a short literals section (< 32 bytes).
func TestLiteralsRoundtripShort(t *testing.T) {
	lits := make([]byte, 20)
	for i := range lits {
		lits[i] = byte(i)
	}
	encoded := encodeLiteralsSection(lits)
	decoded, _, err := decodeLiteralsSection(encoded)
	if err != nil {
		t.Fatalf("decodeLiteralsSection: %v", err)
	}
	assertBytes(t, decoded, lits, "short literals")
}

// TestLiteralsRoundtripMedium verifies encode/decode of a medium literals section (< 4096 bytes).
func TestLiteralsRoundtripMedium(t *testing.T) {
	lits := make([]byte, 200)
	for i := range lits {
		lits[i] = byte(i % 256)
	}
	encoded := encodeLiteralsSection(lits)
	decoded, _, err := decodeLiteralsSection(encoded)
	if err != nil {
		t.Fatalf("decodeLiteralsSection: %v", err)
	}
	assertBytes(t, decoded, lits, "medium literals")
}

// TestLiteralsRoundtripLarge verifies encode/decode of a large literals section (> 4096 bytes).
func TestLiteralsRoundtripLarge(t *testing.T) {
	lits := make([]byte, 5000)
	for i := range lits {
		lits[i] = byte(i % 256)
	}
	encoded := encodeLiteralsSection(lits)
	decoded, _, err := decodeLiteralsSection(encoded)
	if err != nil {
		t.Fatalf("decodeLiteralsSection: %v", err)
	}
	assertBytes(t, decoded, lits, "large literals")
}

// TestRevBitRoundtrip verifies that the backward bit writer and reader are
// exact inverses of each other.
//
// Write order:  A=0b101 (3 bits), B=0b11001100 (8 bits), C=0b1 (1 bit)
// Read order:   C first, then B, then A  (reversed)
func TestRevBitRoundtrip(t *testing.T) {
	bw := &revBitWriter{}
	bw.addBits(0b101, 3)      // A — written first → read last
	bw.addBits(0b11001100, 8) // B
	bw.addBits(0b1, 1)        // C — written last → read first
	bw.flush()
	buf := bw.finish()

	br, err := newRevBitReader(buf)
	if err != nil {
		t.Fatalf("newRevBitReader: %v", err)
	}
	if got := br.readBits(1); got != 0b1 {
		t.Errorf("C: got %b, want %b", got, 0b1)
	}
	if got := br.readBits(8); got != 0b11001100 {
		t.Errorf("B: got %08b, want %08b", got, 0b11001100)
	}
	if got := br.readBits(3); got != 0b101 {
		t.Errorf("A: got %b, want %b", got, 0b101)
	}
}

// TestRevBitSingleBit verifies writing and reading a single bit.
func TestRevBitSingleBit(t *testing.T) {
	for _, bit := range []uint64{0, 1} {
		bw := &revBitWriter{}
		bw.addBits(bit, 1)
		bw.flush()
		buf := bw.finish()
		br, err := newRevBitReader(buf)
		if err != nil {
			t.Fatalf("single bit %d: newRevBitReader: %v", bit, err)
		}
		got := br.readBits(1)
		if got != bit {
			t.Errorf("single bit %d: got %d", bit, got)
		}
	}
}

// TestRevBitZeroNb verifies that addBits(x, 0) is a no-op.
func TestRevBitZeroNb(t *testing.T) {
	bw := &revBitWriter{}
	bw.addBits(0xFF, 0) // should be ignored
	bw.addBits(0b10, 2)
	bw.flush()
	buf := bw.finish()
	br, err := newRevBitReader(buf)
	if err != nil {
		t.Fatalf("newRevBitReader: %v", err)
	}
	got := br.readBits(2)
	if got != 0b10 {
		t.Errorf("got %b, want 0b10", got)
	}
}

// TestRevBitEmptyError verifies that newRevBitReader rejects an empty buffer.
func TestRevBitEmptyError(t *testing.T) {
	_, err := newRevBitReader([]byte{})
	if err == nil {
		t.Fatal("expected error for empty bitstream, got nil")
	}
}

// TestRevBitZeroLastByteError verifies that newRevBitReader rejects a zero sentinel byte.
func TestRevBitZeroLastByteError(t *testing.T) {
	_, err := newRevBitReader([]byte{0x00})
	if err == nil {
		t.Fatal("expected error for zero last byte, got nil")
	}
}

// TestFSEDecodeTableCoverage verifies that every slot in the LL decode table
// holds a valid symbol index (within the norm array bounds).
func TestFSEDecodeTableCoverage(t *testing.T) {
	dt := buildDecodeTable(llNorm[:], llAccLog)
	if len(dt) != 1<<llAccLog {
		t.Fatalf("decode table size: got %d, want %d", len(dt), 1<<llAccLog)
	}
	for i, cell := range dt {
		if int(cell.sym) >= len(llNorm) {
			t.Errorf("slot %d: sym=%d out of range [0,%d)", i, cell.sym, len(llNorm))
		}
	}
}

// TestFSEDecodeTableML verifies the ML decode table size and valid symbols.
func TestFSEDecodeTableML(t *testing.T) {
	dt := buildDecodeTable(mlNorm[:], mlAccLog)
	if len(dt) != 1<<mlAccLog {
		t.Fatalf("ML decode table size: got %d, want %d", len(dt), 1<<mlAccLog)
	}
	for i, cell := range dt {
		if int(cell.sym) >= len(mlNorm) {
			t.Errorf("ML slot %d: sym=%d out of range", i, cell.sym)
		}
	}
}

// TestFSEDecodeTableOF verifies the OF decode table size and valid symbols.
func TestFSEDecodeTableOF(t *testing.T) {
	dt := buildDecodeTable(ofNorm[:], ofAccLog)
	if len(dt) != 1<<ofAccLog {
		t.Fatalf("OF decode table size: got %d, want %d", len(dt), 1<<ofAccLog)
	}
	for i, cell := range dt {
		if int(cell.sym) >= len(ofNorm) {
			t.Errorf("OF slot %d: sym=%d out of range", i, cell.sym)
		}
	}
}

// TestSeqCountRoundtrip verifies that sequence count encoding and decoding
// are exact inverses for all interesting boundary values.
func TestSeqCountRoundtrip(t *testing.T) {
	cases := []int{0, 1, 50, 127, 128, 1000, 0x7FFE}
	for _, n := range cases {
		enc := encodeSeqCount(n)
		got, _, err := decodeSeqCount(enc)
		if err != nil {
			t.Errorf("seqCount %d: decode error: %v", n, err)
			continue
		}
		if got != n {
			t.Errorf("seqCount %d: decoded %d", n, got)
		}
	}
}

// TestFSETwoSequenceRoundtrip encodes and decodes two sequences to verify that
// FSE state transitions work correctly with multiple sequences.
func TestFSETwoSequenceRoundtrip(t *testing.T) {
	seqs := []seq{
		{ll: 2, ml: 4, off: 1},
		{ll: 0, ml: 3, off: 2},
	}
	bitstream := encodeSequencesSection(seqs)

	dtLL := buildDecodeTable(llNorm[:], llAccLog)
	dtML := buildDecodeTable(mlNorm[:], mlAccLog)
	dtOF := buildDecodeTable(ofNorm[:], ofAccLog)

	br, err := newRevBitReader(bitstream)
	if err != nil {
		t.Fatalf("newRevBitReader: %v", err)
	}
	stateLL := uint16(br.readBits(llAccLog))
	stateML := uint16(br.readBits(mlAccLog))
	stateOF := uint16(br.readBits(ofAccLog))

	for i, expected := range seqs {
		llCode := fseDecodeSym(&stateLL, dtLL, br)
		ofCode := fseDecodeSym(&stateOF, dtOF, br)
		mlCode := fseDecodeSym(&stateML, dtML, br)

		llInfo := llCodes[llCode]
		mlInfo := mlCodes[mlCode]
		llDec := llInfo[0] + uint32(br.readBits(uint8(llInfo[1])))
		mlDec := mlInfo[0] + uint32(br.readBits(uint8(mlInfo[1])))
		ofRaw := (uint32(1) << ofCode) | uint32(br.readBits(ofCode))
		offDec := ofRaw - 3

		if llDec != expected.ll {
			t.Errorf("seq %d LL: got %d, want %d", i, llDec, expected.ll)
		}
		if mlDec != expected.ml {
			t.Errorf("seq %d ML: got %d, want %d", i, mlDec, expected.ml)
		}
		if offDec != expected.off {
			t.Errorf("seq %d OFF: got %d, want %d", i, offDec, expected.off)
		}
	}
}

// TestFSESingleSequenceRoundtrip encodes a single sequence and verifies that
// decoding it gives back the exact same (ll, ml, off) values. This isolates
// the FSE codec from the block-level encode/decode.
func TestFSESingleSequenceRoundtrip(t *testing.T) {
	seqs := []seq{{ll: 3, ml: 5, off: 2}}

	eeLL, stLL := buildEncodeTable(llNorm[:], llAccLog)
	eeML, stML := buildEncodeTable(mlNorm[:], mlAccLog)
	eeOF, stOF := buildEncodeTable(ofNorm[:], ofAccLog)

	szLL := uint32(1) << llAccLog
	szML := uint32(1) << mlAccLog
	szOF := uint32(1) << ofAccLog

	stateLL := szLL
	stateML := szML
	stateOF := szOF
	bw := &revBitWriter{}

	for i := len(seqs) - 1; i >= 0; i-- {
		s := seqs[i]
		llCode := llToCode(s.ll)
		mlCode := mlToCode(s.ml)
		rawOff := s.off + 3
		ofCode := uint8(31 - bits.LeadingZeros32(rawOff))
		ofExtra := rawOff - (uint32(1) << ofCode)

		bw.addBits(uint64(ofExtra), ofCode)
		mlExtra := s.ml - mlCodes[mlCode][0]
		bw.addBits(uint64(mlExtra), uint8(mlCodes[mlCode][1]))
		llExtra := s.ll - llCodes[llCode][0]
		bw.addBits(uint64(llExtra), uint8(llCodes[llCode][1]))

		fseEncodeSym(&stateOF, ofCode, eeOF, stOF, bw)
		fseEncodeSym(&stateML, uint8(mlCode), eeML, stML, bw)
		fseEncodeSym(&stateLL, uint8(llCode), eeLL, stLL, bw)
	}

	bw.addBits(uint64(stateOF-szOF), ofAccLog)
	bw.addBits(uint64(stateML-szML), mlAccLog)
	bw.addBits(uint64(stateLL-szLL), llAccLog)
	bw.flush()
	bitstream := bw.finish()

	dtLL := buildDecodeTable(llNorm[:], llAccLog)
	dtML := buildDecodeTable(mlNorm[:], mlAccLog)
	dtOF := buildDecodeTable(ofNorm[:], ofAccLog)

	br, err := newRevBitReader(bitstream)
	if err != nil {
		t.Fatalf("newRevBitReader: %v", err)
	}
	stateLLd := uint16(br.readBits(llAccLog))
	stateMLd := uint16(br.readBits(mlAccLog))
	stateOFd := uint16(br.readBits(ofAccLog))

	llCode := fseDecodeSym(&stateLLd, dtLL, br)
	ofCode := fseDecodeSym(&stateOFd, dtOF, br)
	mlCode := fseDecodeSym(&stateMLd, dtML, br)

	llInfo := llCodes[llCode]
	mlInfo := mlCodes[mlCode]
	llDec := llInfo[0] + uint32(br.readBits(uint8(llInfo[1])))
	mlDec := mlInfo[0] + uint32(br.readBits(uint8(mlInfo[1])))
	ofRaw := (uint32(1) << ofCode) | uint32(br.readBits(ofCode))
	offDec := ofRaw - 3

	if llDec != 3 {
		t.Errorf("LL: got %d, want 3", llDec)
	}
	if mlDec != 5 {
		t.Errorf("ML: got %d, want 5", mlDec)
	}
	if offDec != 2 {
		t.Errorf("OFF: got %d, want 2", offDec)
	}
}

// TestIsAllSame verifies the helper function for RLE detection.
func TestIsAllSame(t *testing.T) {
	if !isAllSame([]byte{}) {
		t.Error("empty slice should be all-same")
	}
	if !isAllSame([]byte{0x42}) {
		t.Error("single byte should be all-same")
	}
	if !isAllSame([]byte{5, 5, 5, 5}) {
		t.Error("uniform slice should be all-same")
	}
	if isAllSame([]byte{5, 5, 6, 5}) {
		t.Error("non-uniform slice should not be all-same")
	}
}

// TestLLToCodeBoundaries verifies LL code mapping at the grouping boundaries.
func TestLLToCodeBoundaries(t *testing.T) {
	// Code 16: baseline 16, 1 extra bit → covers 16-17
	if got := llToCode(16); got != 16 {
		t.Errorf("llToCode(16) = %d, want 16", got)
	}
	if got := llToCode(17); got != 16 {
		t.Errorf("llToCode(17) = %d, want 16", got)
	}
	// Code 17: baseline 18, 1 extra bit → covers 18-19
	if got := llToCode(18); got != 17 {
		t.Errorf("llToCode(18) = %d, want 17", got)
	}
}

// TestMLToCodeBoundaries verifies ML code mapping at the grouping boundaries.
func TestMLToCodeBoundaries(t *testing.T) {
	// Code 32: baseline 35, 1 extra bit → covers 35-36
	if got := mlToCode(35); got != 32 {
		t.Errorf("mlToCode(35) = %d, want 32", got)
	}
	if got := mlToCode(36); got != 32 {
		t.Errorf("mlToCode(36) = %d, want 32", got)
	}
}

// TestSeqCountBoundaryValues exercises all three encoding ranges of encodeSeqCount
// and decodeSeqCount, including the 3-byte (0xFF) path.
func TestSeqCountBoundaryValues(t *testing.T) {
	// 3-byte path: count >= 0x7FFF
	for _, n := range []int{0x7FFF, 0x8000, 0xFFFF} {
		enc := encodeSeqCount(n)
		got, _, err := decodeSeqCount(enc)
		if err != nil {
			t.Errorf("seqCount %d: decode error: %v", n, err)
			continue
		}
		if got != n {
			t.Errorf("seqCount %d: decoded %d", n, got)
		}
	}
}

// TestDecodeSeqCountTruncated verifies that truncated 2-byte and 3-byte sequence
// count encodings are caught with errors.
func TestDecodeSeqCountTruncated(t *testing.T) {
	// 2-byte path: b0 in [128, 0xFE], but only 1 byte provided
	_, _, err := decodeSeqCount([]byte{0x80})
	if err == nil {
		t.Error("expected error for truncated 2-byte seq count")
	}
	// 3-byte path: b0=0xFF, but only 1 byte provided
	_, _, err = decodeSeqCount([]byte{0xFF})
	if err == nil {
		t.Error("expected error for truncated 3-byte seq count")
	}
}

// TestDecodeLiteralsTruncatedHeaders tests error paths in decodeLiteralsSection
// for truncated 2-byte and 3-byte headers.
func TestDecodeLiteralsTruncatedHeaders(t *testing.T) {
	// 2-byte header: size_format=01 (b0 has bits [3:2]=01 → b0 & 0b1100 = 0b0100)
	// b0 = 0b0000_0100 = 0x04 (type=0, size_format=1, size_high=0)
	_, _, err := decodeLiteralsSection([]byte{0x04})
	if err == nil {
		t.Error("expected error for truncated 2-byte literal header")
	}

	// 3-byte header: size_format=11 (b0 has bits [3:2]=11 → b0 & 0b1100 = 0b1100)
	// b0 = 0b0000_1100 = 0x0C (type=0, size_format=3, size_high=0)
	_, _, err = decodeLiteralsSection([]byte{0x0C})
	if err == nil {
		t.Error("expected error for truncated 3-byte literal header")
	}
	_, _, err = decodeLiteralsSection([]byte{0x0C, 0x00})
	if err == nil {
		t.Error("expected error for truncated 3-byte literal header (2 bytes)")
	}
}

// TestDecodeLiteralsUnsupportedType verifies that non-Raw literals type is rejected.
func TestDecodeLiteralsUnsupportedType(t *testing.T) {
	// Literals_Block_Type = 2 (bits [1:0] = 0b10)
	_, _, err := decodeLiteralsSection([]byte{0x02})
	if err == nil {
		t.Error("expected error for unsupported literals type 2")
	}
}

// TestDecodeLiteralsDataTruncated verifies that a literals header claiming more
// bytes than are available is caught.
func TestDecodeLiteralsDataTruncated(t *testing.T) {
	// 1-byte header: n=31 (max for 1-byte), but no data follows.
	// b0 = (31 << 3) | 0 = 0xF8
	_, _, err := decodeLiteralsSection([]byte{0xF8})
	if err == nil {
		t.Error("expected error for truncated literals data")
	}
}

// TestDecompressReservedBlockType verifies that block type 3 (reserved) returns an error.
func TestDecompressReservedBlockType(t *testing.T) {
	// Construct a valid frame header, then a block with type=3.
	// FHD=0xE0, FCS=8 bytes of zeros, then block: type=3, last=1, size=0
	// block header bits: last=1, type=3 (bits[2:1]=11), size=0
	// hdr = (0 << 3) | (3 << 1) | 1 = 7 = 0x07
	frame := []byte{
		0x28, 0xB5, 0x2F, 0xFD, // magic
		0xE0,                   // FHD: FCS=8bytes, Single_Segment=1
		0, 0, 0, 0, 0, 0, 0, 0, // FCS (8 bytes)
		0x07, 0x00, 0x00,       // block: last=1, type=3 (reserved), size=0
	}
	_, err := Decompress(frame)
	if err == nil {
		t.Error("expected error for reserved block type 3")
	}
}

// TestDecompressRLEBlock verifies that an RLE block (type=1) decodes correctly
// via the Decompress path directly (not just through Compress).
func TestDecompressRLEBlock(t *testing.T) {
	// Manually construct a frame with an RLE block of 'Z' repeated 10 times.
	// hdr: last=1, type=1 (RLE), size=10
	// hdr = (10 << 3) | (1 << 1) | 1 = 83 = 0x53
	frame := []byte{
		0x28, 0xB5, 0x2F, 0xFD, // magic
		0x20,                   // FHD: Single_Segment=1, FCS=1byte
		0x0A,                   // FCS = 10
		0x53, 0x00, 0x00,       // block: last=1, RLE, size=10
		'Z',                    // RLE byte
	}
	got, err := Decompress(frame)
	if err != nil {
		t.Fatalf("Decompress RLE block: %v", err)
	}
	want := make([]byte, 10)
	for i := range want {
		want[i] = 'Z'
	}
	assertBytes(t, got, want, "RLE block decode")
}

// TestDecompressMissingRLEByte verifies that an RLE block with no payload byte errors.
func TestDecompressMissingRLEByte(t *testing.T) {
	// RLE block header but no payload byte.
	frame := []byte{
		0x28, 0xB5, 0x2F, 0xFD,
		0x20,
		0x05,
		// block: last=1, RLE, size=5 → hdr = (5<<3)|(1<<1)|1 = 43 = 0x2B
		0x2B, 0x00, 0x00,
		// missing RLE byte
	}
	_, err := Decompress(frame)
	if err == nil {
		t.Error("expected error for missing RLE byte")
	}
}

// TestDecompressWindowDescriptor verifies that frames with Single_Segment=0
// (which have a Window_Descriptor byte) are decoded correctly.
func TestDecompressWindowDescriptor(t *testing.T) {
	// FHD: FCS_flag=00, Single_Segment=0 → Window_Descriptor present + FCS=0 bytes
	// FHD = 0b0000_0000 = 0x00
	// Window_Descriptor = 0x50 (just any byte, we skip it)
	// block: last=1, raw, size=3 → (3<<3)|(0<<1)|1 = 25 = 0x19
	frame := []byte{
		0x28, 0xB5, 0x2F, 0xFD, // magic
		0x00,                   // FHD: Single_Segment=0, FCS_flag=0, no dict
		0x50,                   // Window_Descriptor (skipped)
		// No FCS (FCS_flag=00 + Single_Segment=0 → 0 FCS bytes)
		0x19, 0x00, 0x00,       // block: last=1, raw, size=3
		'a', 'b', 'c',
	}
	got, err := Decompress(frame)
	if err != nil {
		t.Fatalf("Decompress with window descriptor: %v", err)
	}
	assertBytes(t, got, []byte("abc"), "window descriptor")
}

// TestDecompressUnsupportedFSEModes verifies that non-Predefined FSE modes
// in the sequences section produce an error.
func TestDecompressUnsupportedFSEModes(t *testing.T) {
	// Build a minimal compressed block where the modes byte is non-zero.
	// We'll craft it so the decompressor reaches the modes check.
	// Literals section: 1-byte header for 0 literals = 0x00
	// Seq count: 1 (0x01)
	// Modes byte: 0x04 (ML mode = 1, non-Predefined)
	blockData := []byte{
		0x00,       // literal section header: 0 literals
		0x01,       // sequence count = 1
		0x04,       // modes byte: ML mode = 1 (non-zero)
		0x00,       // bogus bitstream
	}
	// Wrap in a frame.
	bsize := len(blockData)
	hdr := uint32(bsize<<3) | (2 << 1) | 1 // type=2 (compressed), last=1
	frame := []byte{
		0x28, 0xB5, 0x2F, 0xFD,
		0xE0,
		0, 0, 0, 0, 0, 0, 0, 0,
		byte(hdr), byte(hdr >> 8), byte(hdr >> 16),
	}
	frame = append(frame, blockData...)
	_, err := Decompress(frame)
	if err == nil {
		t.Error("expected error for unsupported FSE modes")
	}
}

// TestLargerCompressionRatio verifies that a highly compressible string (long
// repetition of a short phrase) actually shrinks.
func TestLargerCompressionRatio(t *testing.T) {
	text := []byte{}
	phrase := []byte("AAAAAA")
	for i := 0; i < 100; i++ {
		text = append(text, phrase...)
	}
	compressed := Compress(text)
	got, err := Decompress(compressed)
	if err != nil {
		t.Fatalf("decompress failed: %v", err)
	}
	assertBytes(t, got, text, "high-ratio")
	if len(compressed) >= len(text) {
		t.Errorf("high-ratio: compressed %d >= input %d", len(compressed), len(text))
	}
}
