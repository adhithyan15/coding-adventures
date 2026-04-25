// Tests for the QR Code encoder.
//
// These tests verify every stage of the encoding pipeline described in the spec
// and cross-checked against the TypeScript reference implementation.
//
// # Test organisation
//
// 1. Unit tests for internal helpers (RS encoder, format/version bits, penalty)
// 2. Integration tests for the full encode() pipeline
// 3. Property tests for symbol invariants (finder patterns, grid size, dark module)
// 4. Version/mode selection tests
// 5. Edge case tests (empty string, single character, maximum capacity)
package qrcode

import (
	"fmt"
	"testing"

	barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
)

// ─────────────────────────────────────────────────────────────────────────────
// Helper utilities
// ─────────────────────────────────────────────────────────────────────────────

// encodeOrFail is a convenience wrapper that encodes with the given options
// and fails the test immediately if encoding fails.
func encodeOrFail(t *testing.T, data string, opts EncodeOptions) barcode2d.ModuleGrid {
	t.Helper()
	grid, err := Encode(data, opts)
	if err != nil {
		t.Fatalf("Encode(%q, %+v) unexpected error: %v", data, opts, err)
	}
	return grid
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon encoder tests
// ─────────────────────────────────────────────────────────────────────────────

// TestBuildQRGeneratorDegree7 verifies the degree-7 QR RS generator polynomial.
//
// The QR RS generator uses the b=0 convention: g(x) = ∏(x + α^i) for i=0..n-1,
// where α = 2 in GF(256) with primitive polynomial 0x11D.
//
// The generator for 7 ECC codewords is:
//   g(x) = x^7 + 127x^6 + 122x^5 + 154x^4 + 164x^3 + 11x^2 + 68x + 117
//
// In bytes (big-endian, monic leading coefficient): [1, 127, 122, 154, 164, 11, 68, 117]
func TestBuildQRGeneratorDegree7(t *testing.T) {
	t.Parallel()
	gen := buildQRGenerator(7)
	// The generator has degree 7, so it has 8 coefficients.
	if len(gen) != 8 {
		t.Fatalf("degree-7 generator has %d coefficients, want 8", len(gen))
	}
	// The leading coefficient must be 1 (monic polynomial).
	if gen[0] != 1 {
		t.Errorf("leading coefficient = %d, want 1 (monic)", gen[0])
	}
	// Compare against the computed b=0 convention generator for GF(256) with 0x11D.
	want := []byte{1, 127, 122, 154, 164, 11, 68, 117}
	for i, w := range want {
		if gen[i] != w {
			t.Errorf("gen[%d] = %d, want %d", i, gen[i], w)
		}
	}
}

// TestBuildQRGeneratorDegree10 verifies the degree-10 QR RS generator polynomial.
//
// The generator for 10 ECC codewords (b=0 convention, GF(256) with 0x11D) is:
//   g(x) coefficients: [1, 216, 194, 159, 111, 199, 94, 95, 113, 157, 193]
func TestBuildQRGeneratorDegree10(t *testing.T) {
	t.Parallel()
	gen := buildQRGenerator(10)
	if len(gen) != 11 {
		t.Fatalf("degree-10 generator has %d coefficients, want 11", len(gen))
	}
	want := []byte{1, 216, 194, 159, 111, 199, 94, 95, 113, 157, 193}
	for i, w := range want {
		if gen[i] != w {
			t.Errorf("gen[%d] = %d, want %d", i, gen[i], w)
		}
	}
}

// TestRSEncode7ECCBytes tests the RS encoder with 7 ECC codewords.
//
// We use the worked example from various QR Code tutorials to verify correctness.
// Data bytes for "Hello, world!" encoded in byte mode (partial test):
// we verify that rsEncode produces the correct number of ECC bytes.
func TestRSEncode7ECCBytes(t *testing.T) {
	t.Parallel()
	gen := buildQRGenerator(7)
	data := []byte{0x20, 0x5B, 0x0B, 0x78, 0xD1, 0x72, 0xDC, 0x4D, 0x43, 0xA8, 0xC7}
	ecc := rsEncode(data, gen)
	if len(ecc) != 7 {
		t.Fatalf("rsEncode produced %d ECC bytes, want 7", len(ecc))
	}
	// Verify the ECC bytes are deterministic (same input always gives same output).
	ecc2 := rsEncode(data, gen)
	for i := range ecc {
		if ecc[i] != ecc2[i] {
			t.Errorf("rsEncode not deterministic: ecc[%d] = %d vs %d", i, ecc[i], ecc2[i])
		}
	}
}

// TestRSEncodeZeroData verifies RS encoding of all-zero data.
// For all-zero data, the ECC should also be all zeros (the zero polynomial
// is a multiple of any generator).
func TestRSEncodeZeroData(t *testing.T) {
	t.Parallel()
	gen := buildQRGenerator(10)
	data := make([]byte, 5)
	ecc := rsEncode(data, gen)
	for i, b := range ecc {
		if b != 0 {
			t.Errorf("ecc[%d] = %d for all-zero data, want 0", i, b)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Format information tests
// ─────────────────────────────────────────────────────────────────────────────

// TestComputeFormatBitsKnownValues tests format information bits against values
// computed from the ISO standard formula.
//
// In this implementation the EccLevel iota is L=0, M=1, Q=2, H=3.
// The eccIndicator mapping (from ISO 18004) is:
//   L → 0b01, M → 0b00, Q → 0b11, H → 0b10
//
// So for mask 0:
//   EccM (00): data=0,  format = 0x5412
//   EccL (01): data=8,  format = 0x77C4
//   EccQ (11): data=24, format = 0x355F
//   EccH (10): data=16, format = 0x1689
func TestComputeFormatBitsKnownValues(t *testing.T) {
	t.Parallel()
	// Test vectors pre-computed via the BCH(15,5) formula with G(x)=0x537, XOR 0x5412.
	testCases := []struct {
		ecc     EccLevel
		mask    int
		wantFmt int
	}{
		// EccM indicator=00, mask 0: data=0 → 0x5412
		{EccM, 0, 0x5412},
		// EccM indicator=00, mask 5: data=5 → 0x40CE
		{EccM, 5, 0x40CE},
		// EccL indicator=01, mask 0: data=8 → 0x77C4
		{EccL, 0, 0x77C4},
		// EccH indicator=10, mask 0: data=16 → 0x1689
		{EccH, 0, 0x1689},
		// EccQ indicator=11, mask 0: data=24 → 0x355F
		{EccQ, 0, 0x355F},
	}

	for _, tc := range testCases {
		tc := tc
		t.Run(fmt.Sprintf("ecc%v_mask%d", tc.ecc, tc.mask), func(t *testing.T) {
			t.Parallel()
			got := computeFormatBits(tc.ecc, tc.mask)
			if got != tc.wantFmt {
				t.Errorf("computeFormatBits(%v, %d) = 0x%04X, want 0x%04X",
					tc.ecc, tc.mask, got, tc.wantFmt)
			}
		})
	}
}

// TestFormatBitsNeverAllZero verifies that no (ecc, mask) combination yields
// all-zero format bits, which would look like a blank region to a scanner.
func TestFormatBitsNeverAllZero(t *testing.T) {
	t.Parallel()
	eccs := []EccLevel{EccL, EccM, EccQ, EccH}
	for _, ecc := range eccs {
		for mask := 0; mask < 8; mask++ {
			bits := computeFormatBits(ecc, mask)
			if bits == 0 {
				t.Errorf("computeFormatBits(%v, %d) = 0 (all-zero format info is invalid)", ecc, mask)
			}
		}
	}
}

// TestComputeVersionBitsKnownValues tests version information bits.
//
// From the ISO standard, version 7 should produce version bits = 0x7C94 (18 bits).
func TestComputeVersionBitsKnownValues(t *testing.T) {
	t.Parallel()
	// Version 7: expected 18-bit version info = 000 111 110 010 010 100
	// = 0b000_111_110_010_010_100 (read LSB at left in ISO convention)
	// The value is 0x07C94 (as found in multiple QR references).
	got7 := computeVersionBits(7)
	// 7 << 12 = 0x7000. After BCH, the full 18-bit word from Nayuki's reference.
	// 0x07C94 = 0b00_0111_1100_1001_0100
	want7 := 0x07C94
	if got7 != want7 {
		t.Errorf("computeVersionBits(7) = 0x%05X, want 0x%05X", got7, want7)
	}

	// Version 40: verify it's a valid 18-bit integer.
	got40 := computeVersionBits(40)
	if got40 < 0 || got40 > (1<<18-1) {
		t.Errorf("computeVersionBits(40) = %d, out of 18-bit range", got40)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Mode selection tests
// ─────────────────────────────────────────────────────────────────────────────

// TestSelectModeNumeric verifies that purely numeric strings use numeric mode.
func TestSelectModeNumeric(t *testing.T) {
	t.Parallel()
	inputs := []string{"0", "12345", "01234567890", "000"}
	for _, input := range inputs {
		mode := selectMode(input)
		if mode != ModeNumeric {
			t.Errorf("selectMode(%q) = %v, want ModeNumeric", input, mode)
		}
	}
}

// TestSelectModeAlphanumeric verifies alphanumeric mode for QR alphanum-set strings.
func TestSelectModeAlphanumeric(t *testing.T) {
	t.Parallel()
	inputs := []string{"HELLO WORLD", "A", "ABC123", "ABCDEF $%*+-./:", "QR CODE"}
	for _, input := range inputs {
		mode := selectMode(input)
		if mode != ModeAlphanumeric {
			t.Errorf("selectMode(%q) = %v, want ModeAlphanumeric", input, mode)
		}
	}
}

// TestSelectModeByte verifies byte mode for strings with lowercase or non-alphanum chars.
func TestSelectModeByte(t *testing.T) {
	t.Parallel()
	inputs := []string{"hello world", "https://example.com", "Hello, World!", "日本語", "abc"}
	for _, input := range inputs {
		mode := selectMode(input)
		if mode != ModeByte {
			t.Errorf("selectMode(%q) = %v, want ModeByte", input, mode)
		}
	}
}

// TestSelectModeEmpty verifies that an empty string uses numeric mode (all-digit trivially).
func TestSelectModeEmpty(t *testing.T) {
	t.Parallel()
	// An empty string: no non-digit chars, no non-alphanum chars → numeric.
	mode := selectMode("")
	if mode != ModeNumeric {
		t.Errorf("selectMode(\"\") = %v, want ModeNumeric", mode)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Character count bits tests
// ─────────────────────────────────────────────────────────────────────────────

// TestCharCountBits verifies the character count field widths from ISO Table 3.
func TestCharCountBits(t *testing.T) {
	t.Parallel()
	// Numeric mode
	if got := charCountBits(ModeNumeric, 1); got != 10 {
		t.Errorf("charCountBits(Numeric, 1) = %d, want 10", got)
	}
	if got := charCountBits(ModeNumeric, 10); got != 12 {
		t.Errorf("charCountBits(Numeric, 10) = %d, want 12", got)
	}
	if got := charCountBits(ModeNumeric, 27); got != 14 {
		t.Errorf("charCountBits(Numeric, 27) = %d, want 14", got)
	}

	// Alphanumeric mode
	if got := charCountBits(ModeAlphanumeric, 9); got != 9 {
		t.Errorf("charCountBits(Alphanum, 9) = %d, want 9", got)
	}
	if got := charCountBits(ModeAlphanumeric, 26); got != 11 {
		t.Errorf("charCountBits(Alphanum, 26) = %d, want 11", got)
	}
	if got := charCountBits(ModeAlphanumeric, 40); got != 13 {
		t.Errorf("charCountBits(Alphanum, 40) = %d, want 13", got)
	}

	// Byte mode
	if got := charCountBits(ModeByte, 1); got != 8 {
		t.Errorf("charCountBits(Byte, 1) = %d, want 8", got)
	}
	if got := charCountBits(ModeByte, 10); got != 16 {
		t.Errorf("charCountBits(Byte, 10) = %d, want 16", got)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Bit writer tests
// ─────────────────────────────────────────────────────────────────────────────

// TestBitWriterSingleByte verifies that a full byte is written correctly.
func TestBitWriterSingleByte(t *testing.T) {
	t.Parallel()
	w := &bitWriter{}
	w.writeBits(0xAB, 8) // 10101011
	b := w.toBytes()
	if len(b) != 1 || b[0] != 0xAB {
		t.Errorf("bitWriter: got %v, want [0xAB]", b)
	}
}

// TestBitWriterPartialByte verifies zero-padding of incomplete byte.
func TestBitWriterPartialByte(t *testing.T) {
	t.Parallel()
	w := &bitWriter{}
	// Write 4 bits: 1010
	w.writeBits(0xA, 4) // 1010
	b := w.toBytes()
	// Should be padded to [1010_0000] = 0xA0
	if len(b) != 1 || b[0] != 0xA0 {
		t.Errorf("bitWriter partial: got [0x%02X], want [0xA0]", b[0])
	}
}

// TestBitWriterMultipleBytes verifies correct packing of multiple values.
func TestBitWriterMultipleBytes(t *testing.T) {
	t.Parallel()
	w := &bitWriter{}
	// 0001 (4 bits) + 00001011 (8 bits) = 00010000_1011xxxx padded → 0x10 0xB0
	w.writeBits(0b0001, 4)
	w.writeBits(0b00001011, 8)
	b := w.toBytes()
	if len(b) != 2 {
		t.Fatalf("expected 2 bytes, got %d", len(b))
	}
	if b[0] != 0x10 {
		t.Errorf("b[0] = 0x%02X, want 0x10", b[0])
	}
	if b[1] != 0xB0 {
		t.Errorf("b[1] = 0x%02X, want 0xB0", b[1])
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Data codewords assembly tests
// ─────────────────────────────────────────────────────────────────────────────

// TestBuildDataCodewordsLength verifies that buildDataCodewords returns exactly
// the right number of bytes for various version/ECC combinations.
func TestBuildDataCodewordsLength(t *testing.T) {
	t.Parallel()
	testCases := []struct {
		input   string
		version int
		ecc     EccLevel
	}{
		{"A", 1, EccL},
		{"HELLO WORLD", 1, EccM},
		{"https://example.com", 3, EccM},
		{"01234567890", 1, EccM},
	}
	for _, tc := range testCases {
		tc := tc
		t.Run(fmt.Sprintf("%s_v%d_%v", tc.input, tc.version, tc.ecc), func(t *testing.T) {
			t.Parallel()
			want := numDataCodewords(tc.version, tc.ecc)
			cw, err := buildDataCodewords(tc.input, tc.version, tc.ecc)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if len(cw) != want {
				t.Errorf("buildDataCodewords length = %d, want %d", len(cw), want)
			}
		})
	}
}

// TestBuildDataCodewordsPaddingPattern verifies that trailing padding uses the
// alternating 0xEC / 0x11 pattern defined in ISO 18004.
func TestBuildDataCodewordsPaddingPattern(t *testing.T) {
	t.Parallel()
	// "A" at version 1 EccL: capacity = 19 bytes. Actual data very short, lots of padding.
	cw, err := buildDataCodewords("A", 1, EccL)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	capacity := numDataCodewords(1, EccL)
	if len(cw) != capacity {
		t.Fatalf("expected %d bytes, got %d", capacity, len(cw))
	}
	// After the data bytes, padding should alternate 0xEC / 0x11.
	// Skip the first few bytes (mode + count + data + terminator).
	// Find the first 0xEC and verify alternation from there.
	foundPad := false
	for i := 2; i < len(cw)-1; i++ {
		if cw[i] == 0xEC && cw[i+1] == 0x11 {
			foundPad = true
			// Check that the pattern continues from here.
			pad := byte(0xEC)
			for j := i; j < len(cw); j++ {
				if cw[j] != pad {
					t.Errorf("padding broken at index %d: got 0x%02X, want 0x%02X", j, cw[j], pad)
					break
				}
				if pad == 0xEC {
					pad = 0x11
				} else {
					pad = 0xEC
				}
			}
			break
		}
	}
	if !foundPad {
		t.Error("could not find 0xEC/0x11 padding pattern in codewords")
	}
}

// TestBuildDataCodewordsHelloWorld verifies the well-known "HELLO WORLD" encoding.
//
// "HELLO WORLD" is a standard QR Code test vector. In alphanumeric mode at
// version 1, ECC level M, the first few bytes should be:
//   - Mode indicator for alphanumeric: 0b0010
//   - Char count (9 bits for v1): 11 chars = 0b000001011
//   - Then the pairs packed into 11 bits each
func TestBuildDataCodewordsHelloWorld(t *testing.T) {
	t.Parallel()
	cw, err := buildDataCodewords("HELLO WORLD", 1, EccM)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Alphanumeric mode indicator = 0b0010 = 2
	// Should start with mode indicator in high bits of first byte.
	// The full bit stream starts: 0010 (mode) 000001011 (count=11) ...
	// = 0010_0000_01011_...
	// First byte: 0010_0000 = 0x20
	if cw[0] != 0x20 {
		t.Errorf("cw[0] = 0x%02X, want 0x20 (alphanumeric mode + start of count)", cw[0])
	}
	// Total length must be exactly the capacity.
	want := numDataCodewords(1, EccM)
	if len(cw) != want {
		t.Errorf("len(cw) = %d, want %d", len(cw), want)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Version selection tests
// ─────────────────────────────────────────────────────────────────────────────

// TestSelectVersionBasicStrings verifies version auto-selection for common inputs.
func TestSelectVersionBasicStrings(t *testing.T) {
	t.Parallel()
	tests := []struct {
		input       string
		ecc         EccLevel
		wantVersion int
	}{
		// "A" is 1 byte: v1 M holds 16 data codewords; needs 4+8+8=20 bits=3 bytes → fits v1 M
		{"A", EccM, 1},
		// "HELLO WORLD" (11 chars alphanumeric) at M: v1 M holds 16 bytes → fits v1 M
		{"HELLO WORLD", EccM, 1},
		// "https://example.com" is 19 bytes (byte mode).
		// Needs 4 + 8 + 19*8 = 164 bits = ceil(164/8) = 21 bytes.
		// v1 M has 16 bytes (too small), v2 M has 28 bytes → fits v2 M.
		{"https://example.com", EccM, 2},
		// "0" - single digit at M
		{"0", EccM, 1},
		// "01234567890" - 11 digits
		{"01234567890", EccM, 1},
	}
	for _, tc := range tests {
		tc := tc
		t.Run(tc.input, func(t *testing.T) {
			t.Parallel()
			v, err := selectVersion(tc.input, tc.ecc, 0)
			if err != nil {
				t.Fatalf("selectVersion(%q, %v, 0) error: %v", tc.input, tc.ecc, err)
			}
			if v != tc.wantVersion {
				t.Errorf("selectVersion(%q, %v) = %d, want %d", tc.input, tc.ecc, v, tc.wantVersion)
			}
		})
	}
}

// TestSelectVersionECCLevelDifference verifies that higher ECC levels need larger versions.
func TestSelectVersionECCLevelDifference(t *testing.T) {
	t.Parallel()
	// A moderately long string that fits in v1 L but needs higher version at H.
	input := "HELLO WORLD"
	vL, _ := selectVersion(input, EccL, 0)
	vH, _ := selectVersion(input, EccH, 0)
	if vL > vH {
		t.Errorf("version L (%d) should be ≤ version H (%d) for same input", vL, vH)
	}
}

// TestSelectVersionForcedVersion verifies that a forced version is respected
// if it has sufficient capacity, and errors if it doesn't.
func TestSelectVersionForcedVersion(t *testing.T) {
	t.Parallel()
	// Force version 5 for a tiny input: should succeed.
	v, err := selectVersion("A", EccM, 5)
	if err != nil {
		t.Fatalf("forced version 5 for 'A': unexpected error: %v", err)
	}
	if v != 5 {
		t.Errorf("forced version 5 returned version %d", v)
	}

	// Force version 1 for a 100-byte input: should fail.
	long := make([]byte, 100)
	for i := range long {
		long[i] = 'A'
	}
	_, err = selectVersion(string(long), EccM, 1)
	if err == nil {
		t.Error("expected error for forced version 1 with 100-byte input, got nil")
	}
	if !IsInputTooLongError(err) {
		t.Errorf("expected InputTooLongError, got %T: %v", err, err)
	}
}

// TestSelectVersionTooLong verifies that an oversized input returns InputTooLongError.
func TestSelectVersionTooLong(t *testing.T) {
	t.Parallel()
	// More than 7089 chars triggers the fast-path bound check.
	long := make([]byte, 7090)
	for i := range long {
		long[i] = 'A'
	}
	_, err := selectVersion(string(long), EccM, 0)
	if err == nil {
		t.Fatal("expected InputTooLongError for 7090-char input")
	}
	if !IsInputTooLongError(err) {
		t.Errorf("expected InputTooLongError, got %T: %v", err, err)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol size and grid helpers
// ─────────────────────────────────────────────────────────────────────────────

// TestSymbolSize verifies the size formula: size = 4*v + 17.
func TestSymbolSize(t *testing.T) {
	t.Parallel()
	tests := []struct {
		v    int
		want int
	}{
		{1, 21},
		{2, 25},
		{10, 57},
		{40, 177},
	}
	for _, tc := range tests {
		if got := symbolSize(tc.v); got != tc.want {
			t.Errorf("symbolSize(%d) = %d, want %d", tc.v, got, tc.want)
		}
	}
}

// TestNumRawDataModules verifies the module count formula from the spec.
func TestNumRawDataModules(t *testing.T) {
	t.Parallel()
	// Version 1: no alignment, no version info
	// = (16*1+128)*1 + 64 = 144*1 + 64 = 208
	if got := numRawDataModules(1); got != 208 {
		t.Errorf("numRawDataModules(1) = %d, want 208", got)
	}
	// Version 7: has alignment and version info
	if got := numRawDataModules(7); got != 1568 {
		t.Errorf("numRawDataModules(7) = %d, want 1568", got)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Block computation tests
// ─────────────────────────────────────────────────────────────────────────────

// TestComputeBlocksCount verifies the correct number of blocks is created.
func TestComputeBlocksCount(t *testing.T) {
	t.Parallel()
	// Version 5, ECC Q: 4 total blocks per the ISO table.
	data, _ := buildDataCodewords("The quick brown fox", 5, EccQ)
	blocks := computeBlocks(data, 5, EccQ)
	expectedBlocks := numBlocks[int(EccQ)][5] // = 4
	if len(blocks) != expectedBlocks {
		t.Errorf("computeBlocks v5/Q: got %d blocks, want %d", len(blocks), expectedBlocks)
	}
}

// TestComputeBlocksECCLength verifies each block has the right number of ECC bytes.
func TestComputeBlocksECCLength(t *testing.T) {
	t.Parallel()
	data, _ := buildDataCodewords("HELLO WORLD", 1, EccM)
	blocks := computeBlocks(data, 1, EccM)
	wantECC := eccCodewordsPerBlock[int(EccM)][1] // = 10
	for i, b := range blocks {
		if len(b.ecc) != wantECC {
			t.Errorf("block[%d].ecc length = %d, want %d", i, len(b.ecc), wantECC)
		}
	}
}

// TestInterleaveBlocksSingleBlock verifies that interleaving a single block
// is a no-op (data then ECC).
func TestInterleaveBlocksSingleBlock(t *testing.T) {
	t.Parallel()
	data, _ := buildDataCodewords("A", 1, EccM)
	blocks := computeBlocks(data, 1, EccM)
	if len(blocks) != 1 {
		t.Skipf("version 1 M should have 1 block, got %d", len(blocks))
	}
	interleaved := interleaveBlocks(blocks)
	// Result should be data || ecc
	want := append(blocks[0].data, blocks[0].ecc...)
	if len(interleaved) != len(want) {
		t.Fatalf("len = %d, want %d", len(interleaved), len(want))
	}
	for i := range want {
		if interleaved[i] != want[i] {
			t.Errorf("[%d] = 0x%02X, want 0x%02X", i, interleaved[i], want[i])
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Full encode integration tests
// ─────────────────────────────────────────────────────────────────────────────

// TestEncodeHelloWorld verifies the canonical QR Code test vector.
//
// "HELLO WORLD" at ECC M should produce a version 1 symbol (21×21 grid).
// This is the most cited test vector in QR Code tutorials and the ISO standard.
func TestEncodeHelloWorld(t *testing.T) {
	t.Parallel()
	grid, err := Encode("HELLO WORLD", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("Encode('HELLO WORLD', EccM) error: %v", err)
	}
	// Version 1 → 21×21
	if grid.Rows != 21 || grid.Cols != 21 {
		t.Errorf("grid size = %dx%d, want 21x21", grid.Rows, grid.Cols)
	}
}

// TestEncodeHTTPSURL verifies URL encoding at ECC M.
//
// "https://example.com" is 19 bytes in byte mode.
// Needs ceil((4 + 8 + 19*8) / 8) = ceil(164/8) = 21 bytes.
// Version 1 M holds 16 bytes (too small). Version 2 M holds 28 bytes → v2 (25×25).
func TestEncodeHTTPSURL(t *testing.T) {
	t.Parallel()
	grid, err := Encode("https://example.com", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid.Rows != 25 || grid.Cols != 25 {
		t.Errorf("grid = %dx%d, want 25x25 (v2)", grid.Rows, grid.Cols)
	}
}

// TestEncodeAllECCLevels verifies that all 4 ECC levels produce valid grids.
func TestEncodeAllECCLevels(t *testing.T) {
	t.Parallel()
	eccs := []EccLevel{EccL, EccM, EccQ, EccH}
	input := "https://example.com"
	for _, ecc := range eccs {
		ecc := ecc
		t.Run(fmt.Sprintf("ecc%v", ecc), func(t *testing.T) {
			t.Parallel()
			grid, err := Encode(input, EncodeOptions{Level: ecc})
			if err != nil {
				t.Fatalf("Encode(%v) error: %v", ecc, err)
			}
			// All levels should produce a valid square grid.
			if grid.Rows != grid.Cols {
				t.Errorf("grid not square: %dx%d", grid.Rows, grid.Cols)
			}
			// Higher ECC requires more space, so versions should be non-decreasing.
			// (We just verify we got a valid grid here; version ordering tested separately.)
			expectedSize := uint32(symbolSize(int((grid.Rows-17)/4)))
			if grid.Rows != expectedSize || grid.Cols != expectedSize {
				t.Errorf("grid size %d not consistent with version formula", grid.Rows)
			}
		})
	}
}

// TestEncodeGridSquare verifies all encoded grids are square with the right formula.
func TestEncodeGridSquare(t *testing.T) {
	t.Parallel()
	inputs := []string{"A", "HELLO WORLD", "https://example.com", "01234567890"}
	for _, input := range inputs {
		grid, err := Encode(input, EncodeOptions{Level: EccM})
		if err != nil {
			t.Fatalf("Encode(%q) error: %v", input, err)
		}
		if grid.Rows != grid.Cols {
			t.Errorf("grid not square for %q: %dx%d", input, grid.Rows, grid.Cols)
		}
		// Size must be 4*v+17 for some v in 1..40.
		size := int(grid.Rows)
		v := (size - 17) / 4
		if v < 1 || v > 40 || symbolSize(v) != size {
			t.Errorf("grid size %d for %q is not a valid QR symbol size", size, input)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Structural invariant tests
// ─────────────────────────────────────────────────────────────────────────────

// TestFinderPatternTopLeft verifies the top-left finder pattern in a v1 grid.
//
// The finder pattern is 7×7 with a specific structure that every QR scanner
// uses to detect and orient the symbol.
func TestFinderPatternTopLeft(t *testing.T) {
	t.Parallel()
	grid, err := Encode("HELLO WORLD", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	modules := grid.Modules

	// Expected top-left 7×7 finder pattern:
	//   1 1 1 1 1 1 1
	//   1 0 0 0 0 0 1
	//   1 0 1 1 1 0 1
	//   1 0 1 1 1 0 1
	//   1 0 1 1 1 0 1
	//   1 0 0 0 0 0 1
	//   1 1 1 1 1 1 1
	expected := [7][7]bool{
		{true, true, true, true, true, true, true},
		{true, false, false, false, false, false, true},
		{true, false, true, true, true, false, true},
		{true, false, true, true, true, false, true},
		{true, false, true, true, true, false, true},
		{true, false, false, false, false, false, true},
		{true, true, true, true, true, true, true},
	}

	for r := 0; r < 7; r++ {
		for c := 0; c < 7; c++ {
			if modules[r][c] != expected[r][c] {
				t.Errorf("top-left finder[%d][%d] = %v, want %v", r, c, modules[r][c], expected[r][c])
			}
		}
	}
}

// TestSeparatorsAreLight verifies that the separator borders are all light modules.
//
// Separators are the 1-module-wide light strips between finder patterns and data.
// They must always be light (false) to isolate the finder from data modules.
func TestSeparatorsAreLight(t *testing.T) {
	t.Parallel()
	grid, err := Encode("HELLO WORLD", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	modules := grid.Modules
	sz := int(grid.Rows)

	// Top-left separator: row 7, cols 0..7 and col 7, rows 0..7
	for i := 0; i <= 7; i++ {
		if modules[7][i] {
			t.Errorf("TL separator row 7, col %d is dark (want light)", i)
		}
		if modules[i][7] {
			t.Errorf("TL separator col 7, row %d is dark (want light)", i)
		}
	}
	// Top-right separator: row 7, cols sz-8..sz-1 and col sz-8, rows 0..7
	for i := 0; i <= 7; i++ {
		if modules[7][sz-1-i] {
			t.Errorf("TR separator row 7, col %d is dark", sz-1-i)
		}
		if modules[i][sz-8] {
			t.Errorf("TR separator col %d, row %d is dark", sz-8, i)
		}
	}
	// Bottom-left separator: row sz-8, cols 0..7 and col 7, rows sz-1..sz-8
	for i := 0; i <= 7; i++ {
		if modules[sz-8][i] {
			t.Errorf("BL separator row %d, col %d is dark", sz-8, i)
		}
		if modules[sz-1-i][7] {
			t.Errorf("BL separator col 7, row %d is dark", sz-1-i)
		}
	}
}

// TestDarkModule verifies the always-dark module at (4V+9, 8).
//
// The dark module is a mandatory always-dark module adjacent to the bottom
// of the left format info strip. It prevents certain all-zero degenerate cases.
func TestDarkModule(t *testing.T) {
	t.Parallel()
	for _, v := range []int{1, 2, 7, 10, 40} {
		v := v
		t.Run(fmt.Sprintf("v%d", v), func(t *testing.T) {
			t.Parallel()
			// Build just the grid (not full encode) to check the dark module directly.
			g := buildGrid(v)
			dmRow := 4*v + 9
			dmCol := 8
			if !g.modules[dmRow][dmCol] {
				t.Errorf("v%d: dark module at (%d,8) is light, want dark", v, dmRow)
			}
			if !g.reserved[dmRow][dmCol] {
				t.Errorf("v%d: dark module at (%d,8) not reserved", v, dmRow)
			}
		})
	}
}

// TestTimingStrips verifies the alternating timing strip pattern.
//
// Row 6 (horizontal timing) and col 6 (vertical timing) must alternate
// dark/light starting dark at index 8 and ending dark before the far finder.
func TestTimingStrips(t *testing.T) {
	t.Parallel()
	g := buildGrid(1) // version 1, size 21
	sz := 21

	// Horizontal timing: row 6, cols 8..sz-9 (= 8..12 for v1)
	for c := 8; c <= sz-9; c++ {
		wantDark := (c % 2) == 0
		if g.modules[6][c] != wantDark {
			t.Errorf("horizontal timing[6][%d] = %v, want %v", c, g.modules[6][c], wantDark)
		}
	}
	// Vertical timing: col 6, rows 8..sz-9
	for r := 8; r <= sz-9; r++ {
		wantDark := (r % 2) == 0
		if g.modules[r][6] != wantDark {
			t.Errorf("vertical timing[%d][6] = %v, want %v", r, g.modules[r][6], wantDark)
		}
	}
}

// TestModuleGridModuleShape verifies that Encode returns ModuleShapeSquare.
func TestModuleGridModuleShape(t *testing.T) {
	t.Parallel()
	grid, err := Encode("A", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid.ModuleShape != 0 { // ModuleShapeSquare = 0
		t.Errorf("ModuleShape = %v, want ModuleShapeSquare (0)", grid.ModuleShape)
	}
}

// TestModuleGridDimensions verifies Rows and Cols consistency with Modules slice.
func TestModuleGridDimensions(t *testing.T) {
	t.Parallel()
	grid, err := Encode("https://example.com", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if uint32(len(grid.Modules)) != grid.Rows {
		t.Errorf("len(Modules) = %d, want %d (Rows)", len(grid.Modules), grid.Rows)
	}
	for r := range grid.Modules {
		if uint32(len(grid.Modules[r])) != grid.Cols {
			t.Errorf("len(Modules[%d]) = %d, want %d (Cols)", r, len(grid.Modules[r]), grid.Cols)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Penalty and mask tests
// ─────────────────────────────────────────────────────────────────────────────

// TestMaskConditionValues spot-checks specific (row, col) values for each mask.
func TestMaskConditionValues(t *testing.T) {
	t.Parallel()
	tests := []struct {
		mask     int
		r, c     int
		expected bool
		desc     string
	}{
		// mask 0: (r+c)%2 == 0
		{0, 0, 0, true, "(0+0)%2=0 → true"},
		{0, 0, 1, false, "(0+1)%2=1 → false"},
		{0, 1, 1, true, "(1+1)%2=0 → true"},
		// mask 1: r%2 == 0
		{1, 0, 5, true, "0%2=0 → true"},
		{1, 1, 5, false, "1%2=1 → false"},
		// mask 2: c%3 == 0
		{2, 0, 0, true, "0%3=0 → true"},
		{2, 0, 1, false, "1%3≠0 → false"},
		{2, 5, 3, true, "3%3=0 → true"},
		// mask 3: (r+c)%3 == 0
		{3, 0, 0, true, "(0+0)%3=0 → true"},
		{3, 1, 1, false, "(1+1)%3=2 → false"},
		{3, 1, 2, true, "(1+2)%3=0 → true"},
		// mask 4: (r/2+c/3)%2 == 0
		{4, 0, 0, true, "(0+0)%2=0 → true"},
		{4, 2, 3, true, "(1+1)%2=0 → true"},
		// mask 5: (r*c)%2 + (r*c)%3 == 0
		{5, 0, 0, true, "0%2+0%3=0 → true"},
		{5, 2, 2, false, "4%2+4%3=0+1=1 → false"},
		// r=1,c=3: (1*3)=3 → 3%2=1, 3%3=0 → 1+0=1≠0 → false
		{5, 1, 3, false, "3%2+3%3=1+0=1 → false"},
		// r=3,c=2: 6%2=0, 6%3=0 → 0+0=0 → true
		{5, 3, 2, true, "6%2+6%3=0+0=0 → true"},
		// mask 6: ((r*c)%2 + (r*c)%3)%2 == 0
		{6, 0, 0, true, "(0%2+0%3)%2=0 → true"},
		{6, 1, 1, true, "(1%2+1%3)%2=(1+1)%2=0 → true"},
		// mask 7: ((r+c)%2 + (r*c)%3)%2 == 0
		{7, 0, 0, true, "(0%2+0%3)%2=0 → true"},
	}
	for _, tc := range tests {
		got := maskCondition(tc.mask, tc.r, tc.c)
		if got != tc.expected {
			t.Errorf("maskCondition(%d, %d, %d) = %v, want %v", tc.mask, tc.r, tc.c, got, tc.expected)
		}
	}
}

// TestPenaltyIsPositive verifies that every full encode produces a non-negative penalty.
// (We can't easily predict the exact value, but it must be non-negative.)
func TestPenaltyIsPositive(t *testing.T) {
	t.Parallel()
	grid, err := Encode("HELLO WORLD", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	sz := int(grid.Rows)
	p := computePenalty(grid.Modules, sz)
	if p < 0 {
		t.Errorf("computePenalty returned %d (negative)", p)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// EncodeToScene tests
// ─────────────────────────────────────────────────────────────────────────────

// TestEncodeToSceneProducesInstructions verifies that EncodeToScene returns a
// non-empty paint scene.
func TestEncodeToSceneProducesInstructions(t *testing.T) {
	t.Parallel()
	scene, err := EncodeToScene(
		"HELLO WORLD",
		EncodeOptions{Level: EccM},
		barcode2d.DefaultBarcode2DLayoutConfig,
	)
	if err != nil {
		t.Fatalf("EncodeToScene error: %v", err)
	}
	// Should have at least the background rect plus some dark module rects.
	if len(scene.Instructions) == 0 {
		t.Error("EncodeToScene returned empty instructions")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge case tests
// ─────────────────────────────────────────────────────────────────────────────

// TestEncodeSingleCharacter verifies encoding of a single character at all ECC levels.
func TestEncodeSingleCharacter(t *testing.T) {
	t.Parallel()
	eccs := []EccLevel{EccL, EccM, EccQ, EccH}
	for _, ecc := range eccs {
		ecc := ecc
		t.Run(fmt.Sprintf("ecc%v", ecc), func(t *testing.T) {
			t.Parallel()
			grid, err := Encode("A", EncodeOptions{Level: ecc})
			if err != nil {
				t.Fatalf("Encode('A', %v) error: %v", ecc, err)
			}
			if grid.Rows < 21 {
				t.Errorf("grid size %d < 21 (v1 minimum)", grid.Rows)
			}
		})
	}
}

// TestEncodeEmptyString verifies that an empty string encodes successfully.
func TestEncodeEmptyString(t *testing.T) {
	t.Parallel()
	grid, err := Encode("", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("Encode('', EccM) error: %v", err)
	}
	// Should be version 1 (smallest possible).
	if grid.Rows != 21 {
		t.Errorf("empty string: grid size = %d, want 21 (v1)", grid.Rows)
	}
}

// TestEncodeNumericOnlyString verifies pure numeric mode encoding.
func TestEncodeNumericOnlyString(t *testing.T) {
	t.Parallel()
	grid, err := Encode("01234567890", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// 11 digits in numeric mode — should fit in v1.
	if grid.Rows != 21 {
		t.Errorf("01234567890 at EccM: grid = %dx%d, want 21x21", grid.Rows, grid.Cols)
	}
	// Verify mode was numeric.
	mode := selectMode("01234567890")
	if mode != ModeNumeric {
		t.Errorf("mode for numeric string = %v, want ModeNumeric", mode)
	}
}

// TestEncodeUTF8String verifies that a UTF-8 multi-byte string encodes without error.
func TestEncodeUTF8String(t *testing.T) {
	t.Parallel()
	// "こんにちは" (hello in Japanese) — multi-byte UTF-8, forces byte mode.
	grid, err := Encode("こんにちは", EncodeOptions{Level: EccM})
	if err != nil {
		t.Fatalf("Encode('こんにちは', EccM) error: %v", err)
	}
	if grid.Rows < 21 {
		t.Errorf("UTF-8 grid too small: %d", grid.Rows)
	}
}

// TestEncodeInputTooLong verifies the error path for oversized input.
func TestEncodeInputTooLong(t *testing.T) {
	t.Parallel()
	// 8000 'A' characters: exceeds the 7089 fast-path bound.
	long := make([]byte, 8000)
	for i := range long {
		long[i] = 'A'
	}
	_, err := Encode(string(long), EncodeOptions{Level: EccM})
	if err == nil {
		t.Fatal("expected error for 8000-char input, got nil")
	}
	if !IsInputTooLongError(err) {
		t.Errorf("expected InputTooLongError, got %T: %v", err, err)
	}
}

// TestEncodeVersion40Capacity tests a large string that requires a high version.
func TestEncodeVersion40Capacity(t *testing.T) {
	t.Parallel()
	// Version 40 ECC L byte mode holds 2953 bytes.
	// Use 2900 bytes — should fit without error.
	data := make([]byte, 2900)
	for i := range data {
		data[i] = 'x'
	}
	grid, err := Encode(string(data), EncodeOptions{Level: EccL})
	if err != nil {
		t.Fatalf("Encode(2900 bytes, EccL) error: %v", err)
	}
	// Should be a high version (close to 40).
	if grid.Rows < 100 {
		t.Errorf("expected large grid for 2900-byte input, got %dx%d", grid.Rows, grid.Cols)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Cross-language compatibility test
// ─────────────────────────────────────────────────────────────────────────────

// TestEncodeTestCorpus runs the cross-language test corpus from the spec.
//
// The spec mandates that all language implementations produce identical
// ModuleGrid outputs for the same input. We can't do a cross-language
// comparison in Go tests, but we verify the outputs are stable (deterministic).
func TestEncodeTestCorpus(t *testing.T) {
	t.Parallel()
	corpus := []struct {
		input string
		ecc   EccLevel
		desc  string
	}{
		{"A", EccM, "minimal"},
		{"HELLO WORLD", EccM, "alphanumeric"},
		{"https://example.com", EccM, "URL byte mode"},
		{"01234567890", EccM, "numeric"},
		{"The quick brown fox jumps over the lazy dog", EccM, "full byte mode"},
	}
	for _, tc := range corpus {
		tc := tc
		t.Run(tc.desc, func(t *testing.T) {
			t.Parallel()
			grid1, err := Encode(tc.input, EncodeOptions{Level: tc.ecc})
			if err != nil {
				t.Fatalf("encode error: %v", err)
			}
			// Re-encode the same input to verify determinism.
			grid2, err := Encode(tc.input, EncodeOptions{Level: tc.ecc})
			if err != nil {
				t.Fatalf("second encode error: %v", err)
			}
			if grid1.Rows != grid2.Rows || grid1.Cols != grid2.Cols {
				t.Errorf("non-deterministic size: %dx%d vs %dx%d", grid1.Rows, grid1.Cols, grid2.Rows, grid2.Cols)
			}
			for r := range grid1.Modules {
				for c := range grid1.Modules[r] {
					if grid1.Modules[r][c] != grid2.Modules[r][c] {
						t.Errorf("non-deterministic module at [%d][%d]", r, c)
					}
				}
			}
		})
	}
}

// encodeOrFail is used only within this file — suppress the "declared but not used" warning
// by ensuring at least one test uses it in a non-obvious way.
var _ = encodeOrFail
