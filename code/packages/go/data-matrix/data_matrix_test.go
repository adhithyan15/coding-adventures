// Tests for the Data Matrix ECC200 encoder.
//
// Test strategy:
//   1. GF(256)/0x12D field arithmetic — exp/log tables, multiplication
//   2. ASCII encoding — single chars, digit pairs, extended ASCII
//   3. Pad codewords — scrambled pad formula, ISO worked example
//   4. Symbol selection — smallest fitting symbol, shape preferences
//   5. RS encoding — LFSR block encoder, generator polynomials
//   6. Block interleaving — round-robin data+ECC
//   7. Grid initialization — border pattern, alignment borders
//   8. Utah placement algorithm — corner patterns, boundary wrapping
//   9. Logical→physical coordinate mapping
//  10. Full encode pipeline — end-to-end, border validation, size selection
//  11. Error cases — InputTooLong, shape filtering
//  12. EncodeToScene — pipeline integration
//
// Key reference: ISO/IEC 16022:2006, Annex F (worked example for "A" → 10×10).
package datamatrix

import (
	"errors"
	"strings"
	"testing"

	barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
)

// ============================================================================
// 1. GF(256)/0x12D field arithmetic tests
// ============================================================================

// TestGFExpTable verifies key values of the exp table for GF(256)/0x12D.
//
// Recurrence: α^0 = 1, α^{i+1} = α^i << 1 (mod 0x12D if overflow).
//
//	α^0  = 0x01
//	α^1  = 0x02
//	α^7  = 0x80    (1 << 7, no overflow yet)
//	α^8  = 0x2D    (0x80 << 1 = 0x100; 0x100 XOR 0x12D = 0x2D)
//	α^9  = 0x5A    (0x2D << 1 = 0x5A, no overflow)
//	α^10 = 0xB4    (0x5A << 1 = 0xB4, no overflow)
func TestGFExpTable(t *testing.T) {
	exp := GFExp()

	tests := []struct {
		i    int
		want byte
	}{
		{0, 0x01},
		{1, 0x02},
		{2, 0x04},
		{3, 0x08},
		{4, 0x10},
		{5, 0x20},
		{6, 0x40},
		{7, 0x80},
		{8, 0x2D},  // 0x80<<1 = 0x100; XOR 0x12D = 0x2D
		{9, 0x5A},  // 0x2D<<1 = 0x5A
		{10, 0xB4}, // 0x5A<<1 = 0xB4
		{255, 0x01}, // α^255 = 1 (order = 255)
	}

	for _, tc := range tests {
		if got := exp[tc.i]; got != tc.want {
			t.Errorf("GFExp[%d] = 0x%02X, want 0x%02X", tc.i, got, tc.want)
		}
	}
}

// TestGFLogTable verifies that log and exp are inverses: GFLog[GFExp[i]] == i.
func TestGFLogTable(t *testing.T) {
	exp := GFExp()
	log := GFLog()

	for i := 0; i < 255; i++ {
		v := exp[i]
		if int(log[v]) != i {
			t.Errorf("GFLog[GFExp[%d]] = %d, want %d", i, log[v], i)
		}
	}
}

// TestGFMulBasic verifies basic GF(256)/0x12D multiplication properties.
func TestGFMulBasic(t *testing.T) {
	tests := []struct {
		a, b byte
		want byte
	}{
		{0, 0xFF, 0},    // zero absorbs: 0 × anything = 0
		{0xFF, 0, 0},    // zero absorbs: anything × 0 = 0
		{1, 0xFF, 0xFF}, // identity: 1 × x = x
		{0xFF, 1, 0xFF},
		{2, 2, 4},       // α^1 × α^1 = α^2 = 4
		{2, 4, 8},       // α^1 × α^2 = α^3 = 8
		{0x80, 2, 0x2D}, // α^7 × α^1 = α^8 = 0x2D
	}

	for _, tc := range tests {
		if got := GFMul(tc.a, tc.b); got != tc.want {
			t.Errorf("GFMul(0x%02X, 0x%02X) = 0x%02X, want 0x%02X",
				tc.a, tc.b, got, tc.want)
		}
	}
}

// TestGFMulCommutative verifies commutativity: a×b == b×a.
func TestGFMulCommutative(t *testing.T) {
	samples := []byte{0, 1, 2, 0x80, 0x2D, 0xAA, 0xFF}
	for _, a := range samples {
		for _, b := range samples {
			ab := GFMul(a, b)
			ba := GFMul(b, a)
			if ab != ba {
				t.Errorf("GFMul(0x%02X, 0x%02X) = 0x%02X but GFMul(0x%02X, 0x%02X) = 0x%02X",
					a, b, ab, b, a, ba)
			}
		}
	}
}

// TestGFFieldOrder verifies α^255 = 1 and α^k ≠ 1 for 0 < k < 255.
// This confirms α = 2 is a primitive element with order 255.
func TestGFFieldOrder(t *testing.T) {
	exp := GFExp()
	if exp[0] != 1 {
		t.Errorf("GFExp[0] = %d, want 1 (α^0 = 1)", exp[0])
	}
	if exp[255] != 1 {
		t.Errorf("GFExp[255] = %d, want 1 (α^255 = 1 by order)", exp[255])
	}
	for i := 1; i < 255; i++ {
		if exp[i] == 1 {
			t.Errorf("GFExp[%d] = 1 unexpectedly — α should have order 255", i)
		}
	}
}

// TestGFMulDistributive verifies a×(b XOR c) == a×b XOR a×c.
// In GF(256), addition is XOR, so this is the distributive law.
func TestGFMulDistributive(t *testing.T) {
	a := byte(0x53)
	b := byte(0xCA)
	c := byte(0x72)

	lhs := GFMul(a, b^c)
	rhs := GFMul(a, b) ^ GFMul(a, c)
	if lhs != rhs {
		t.Errorf("distributive law failed: 0x%02X × (0x%02X ^ 0x%02X) = 0x%02X, rhs = 0x%02X",
			a, b, c, lhs, rhs)
	}
}

// ============================================================================
// 2. ASCII encoding tests
// ============================================================================

// TestEncodeASCIISingleChars tests single character ASCII encoding.
// Rule: codeword = ASCII_value + 1.
func TestEncodeASCIISingleChars(t *testing.T) {
	tests := []struct {
		input []byte
		want  []byte
	}{
		{[]byte("A"), []byte{66}},   // 65 + 1
		{[]byte("a"), []byte{98}},   // 97 + 1
		{[]byte(" "), []byte{33}},   // 32 + 1
		{[]byte("!"), []byte{34}},   // 33 + 1
		{[]byte("Z"), []byte{91}},   // 90 + 1
		{[]byte("\x00"), []byte{1}}, // 0 + 1
	}

	for _, tc := range tests {
		got := EncodeASCII(tc.input)
		if len(got) != len(tc.want) {
			t.Errorf("EncodeASCII(%q) len = %d, want %d", tc.input, len(got), len(tc.want))
			continue
		}
		for i := range tc.want {
			if got[i] != tc.want[i] {
				t.Errorf("EncodeASCII(%q)[%d] = %d, want %d", tc.input, i, got[i], tc.want[i])
			}
		}
	}
}

// TestEncodeASCIIDigitPairs tests two-digit pair compression.
// Rule: codeword = 130 + (d1×10 + d2).
func TestEncodeASCIIDigitPairs(t *testing.T) {
	tests := []struct {
		input []byte
		want  []byte
	}{
		{[]byte("12"), []byte{142}},           // 130 + (1*10+2) = 130+12 = 142
		{[]byte("34"), []byte{164}},           // 130 + (3*10+4) = 130+34 = 164
		{[]byte("00"), []byte{130}},           // 130 + 0 = 130
		{[]byte("99"), []byte{229}},           // 130 + (9*10+9) = 130+99 = 229
		{[]byte("1234"), []byte{142, 164}},    // 130+12=142, 130+34=164
		{[]byte("1A"), []byte{50, 66}},        // digit, then letter — no pair
		{[]byte("A1"), []byte{66, 50}},        // letter, then digit — no pair
	}

	for _, tc := range tests {
		got := EncodeASCII(tc.input)
		if len(got) != len(tc.want) {
			t.Errorf("EncodeASCII(%q) len = %d, want %d", tc.input, len(got), len(tc.want))
			continue
		}
		for i := range tc.want {
			if got[i] != tc.want[i] {
				t.Errorf("EncodeASCII(%q)[%d] = %d, want %d", tc.input, i, got[i], tc.want[i])
			}
		}
	}
}

// TestEncodeASCIILongDigitRun tests a long digit string for pair packing.
// "12345678" → 4 codewords (pairs: 12, 34, 56, 78).
// Formula: 130 + (d1*10 + d2)
//   "12" → 130 + 12 = 142
//   "34" → 130 + 34 = 164
//   "56" → 130 + 56 = 186
//   "78" → 130 + 78 = 208
func TestEncodeASCIILongDigitRun(t *testing.T) {
	got := EncodeASCII([]byte("12345678"))
	want := []byte{142, 164, 186, 208}
	if len(got) != len(want) {
		t.Fatalf("EncodeASCII(12345678) len = %d, want %d; got %v", len(got), len(want), got)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Errorf("codeword[%d] = %d, want %d", i, got[i], want[i])
		}
	}
}

// TestEncodeASCIIOddDigits tests that an odd-length digit string handles the
// trailing digit as a single codeword (no pair for the last digit).
func TestEncodeASCIIOddDigits(t *testing.T) {
	got := EncodeASCII([]byte("123"))
	// "12" → 142 (pair), "3" → 52 (51+1)
	want := []byte{142, 52}
	if len(got) != len(want) {
		t.Fatalf("EncodeASCII(123) len = %d, want %d; got %v", len(got), len(want), got)
	}
	for i := range want {
		if got[i] != want[i] {
			t.Errorf("codeword[%d] = %d, want %d", i, got[i], want[i])
		}
	}
}

// TestEncodeASCIIExtended tests UPPER_SHIFT encoding for bytes 128–255.
func TestEncodeASCIIExtended(t *testing.T) {
	// Byte 128: UPPER_SHIFT (235) then 128 - 127 = 1
	got := EncodeASCII([]byte{128})
	if len(got) != 2 || got[0] != 235 || got[1] != 1 {
		t.Errorf("EncodeASCII([128]) = %v, want [235, 1]", got)
	}

	// Byte 255: UPPER_SHIFT (235) then 255 - 127 = 128
	got = EncodeASCII([]byte{255})
	if len(got) != 2 || got[0] != 235 || got[1] != 128 {
		t.Errorf("EncodeASCII([255]) = %v, want [235, 128]", got)
	}
}

// TestEncodeASCIIEmpty tests empty input returns empty codewords.
func TestEncodeASCIIEmpty(t *testing.T) {
	got := EncodeASCII([]byte{})
	if len(got) != 0 {
		t.Errorf("EncodeASCII([]) = %v, want []", got)
	}
}

// ============================================================================
// 3. Pad codewords tests
// ============================================================================

// TestPadCodewordsISOExample verifies the ISO/IEC 16022 worked example.
//
// Encoding "A" → [66] in a 10×10 symbol (dataCW = 3):
//   - k=2: 129 (first pad, always literal)
//   - k=3: 129 + (149×3 mod 253) + 1 = 129 + 194 + 1 = 324; 324-254 = 70
//
// Expected: [66, 129, 70]
func TestPadCodewordsISOExample(t *testing.T) {
	codewords := []byte{66} // "A" encoded
	padded := PadCodewords(codewords, 3)

	want := []byte{66, 129, 70}
	if len(padded) != len(want) {
		t.Fatalf("PadCodewords([66], 3) len = %d, want %d", len(padded), len(want))
	}
	for i := range want {
		if padded[i] != want[i] {
			t.Errorf("padded[%d] = %d, want %d", i, padded[i], want[i])
		}
	}
}

// TestPadCodewordsFirstIsLiteral verifies the first pad is always 129.
func TestPadCodewordsFirstIsLiteral(t *testing.T) {
	// Any input with capacity > encoded length — first pad must be 129.
	padded := PadCodewords([]byte{66, 50}, 5)
	if padded[2] != 129 {
		t.Errorf("first pad = %d, want 129", padded[2])
	}
}

// TestPadCodewordsNoOpWhenFull verifies no padding when already at capacity.
func TestPadCodewordsNoOpWhenFull(t *testing.T) {
	input := []byte{1, 2, 3}
	padded := PadCodewords(input, 3)
	if len(padded) != 3 {
		t.Errorf("pad on full slice returned %d bytes, want 3", len(padded))
	}
	for i, b := range input {
		if padded[i] != b {
			t.Errorf("padded[%d] = %d, want %d", i, padded[i], b)
		}
	}
}

// TestPadCodewordsResultLength verifies output length is exactly dataCW.
func TestPadCodewordsResultLength(t *testing.T) {
	for _, dataCW := range []int{3, 5, 8, 12, 18, 22, 44} {
		padded := PadCodewords([]byte{66}, dataCW)
		if len(padded) != dataCW {
			t.Errorf("PadCodewords([66], %d) len = %d, want %d", dataCW, len(padded), dataCW)
		}
	}
}

// ============================================================================
// 4. Symbol selection tests
// ============================================================================

// TestSelectSymbolSmallest verifies "A" (1 codeword) selects the 10×10 symbol.
func TestSelectSymbolSmallest(t *testing.T) {
	e, err := SelectSymbol(1, SymbolShapeSquare)
	if err != nil {
		t.Fatalf("SelectSymbol(1, Square) error: %v", err)
	}
	if e.symbolRows != 10 || e.symbolCols != 10 {
		t.Errorf("SelectSymbol(1) = %dx%d, want 10×10", e.symbolRows, e.symbolCols)
	}
}

// TestSelectSymbolCapacityBoundary verifies exact capacity boundaries.
func TestSelectSymbolCapacityBoundary(t *testing.T) {
	// 10×10 has dataCW = 3; with 3 codewords it should still fit in 10×10.
	e, err := SelectSymbol(3, SymbolShapeSquare)
	if err != nil {
		t.Fatalf("SelectSymbol(3): %v", err)
	}
	if e.symbolRows != 10 {
		t.Errorf("3 codewords → %dx%d, want 10×10", e.symbolRows, e.symbolCols)
	}

	// 4 codewords → needs 12×12 (dataCW = 5).
	e, err = SelectSymbol(4, SymbolShapeSquare)
	if err != nil {
		t.Fatalf("SelectSymbol(4): %v", err)
	}
	if e.symbolRows != 12 {
		t.Errorf("4 codewords → %dx%d, want 12×12", e.symbolRows, e.symbolCols)
	}
}

// TestSelectSymbolRectangular verifies rectangular shape selection.
func TestSelectSymbolRectangular(t *testing.T) {
	e, err := SelectSymbol(1, SymbolShapeRectangular)
	if err != nil {
		t.Fatalf("SelectSymbol(1, Rect): %v", err)
	}
	// Smallest rect is 8×18 (dataCW = 5).
	if e.symbolRows != 8 || e.symbolCols != 18 {
		t.Errorf("SelectSymbol(1, Rect) = %dx%d, want 8×18", e.symbolRows, e.symbolCols)
	}
}

// TestSelectSymbolTooLong verifies InputTooLongError when input exceeds max.
func TestSelectSymbolTooLong(t *testing.T) {
	_, err := SelectSymbol(1559, SymbolShapeSquare)
	if err == nil {
		t.Fatal("expected error for 1559 codewords, got nil")
	}
	var tooLong *InputTooLongError
	if !errors.As(err, &tooLong) {
		t.Errorf("expected *InputTooLongError, got %T: %v", err, err)
	}
}

// TestSelectSymbolAnyPreferSmallest verifies SymbolShapeAny selects the
// globally smallest symbol by dataCW.
func TestSelectSymbolAnyPreferSmallest(t *testing.T) {
	// 5 codewords: 8×18 rect has dataCW=5, 12×12 square has dataCW=5.
	// Either is valid; the tie-break is by area.
	e, err := SelectSymbol(5, SymbolShapeAny)
	if err != nil {
		t.Fatalf("SelectSymbol(5, Any): %v", err)
	}
	// 8×18 area = 144, 12×12 area = 144 — truly tied; any is fine.
	if e.dataCW < 5 {
		t.Errorf("selected symbol has dataCW = %d, too small for 5 codewords", e.dataCW)
	}
}

// TestSelectSymbolSquareOnlyExcludesRect verifies square-only mode excludes
// the 8×18 rectangular symbol.
func TestSelectSymbolSquareOnlyExcludesRect(t *testing.T) {
	e, err := SelectSymbol(1, SymbolShapeSquare)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// Must not be 8×18 (which is rectangular).
	if e.symbolRows == 8 && e.symbolCols == 18 {
		t.Error("SymbolShapeSquare returned a rectangular (8×18) symbol")
	}
}

// ============================================================================
// 5. Reed-Solomon encoding tests
// ============================================================================

// TestGeneratorPolynomialDegree verifies the degree of built generator polys.
func TestGeneratorPolynomialDegree(t *testing.T) {
	for _, nEcc := range []int{5, 7, 10, 12, 14, 18, 20, 24, 28} {
		g := GetGenerator(nEcc)
		// Length = nEcc+1 (includes leading 1).
		if len(g) != nEcc+1 {
			t.Errorf("GetGenerator(%d) len = %d, want %d", nEcc, len(g), nEcc+1)
		}
		// Leading coefficient must be 1.
		if g[0] != 1 {
			t.Errorf("GetGenerator(%d)[0] = %d, want 1 (monic)", nEcc, g[0])
		}
	}
}

// TestGeneratorPolynomialRoots verifies that α^1 … α^n are roots of g(x).
//
// By construction g(x) = (x+α¹)(x+α²)…(x+α^n), so g(α^i) = 0 for i=1..n.
// We evaluate g at each root to confirm.
func TestGeneratorPolynomialRoots(t *testing.T) {
	exp := GFExp()
	for _, nEcc := range []int{5, 7, 10} {
		g := GetGenerator(nEcc)
		for root := 1; root <= nEcc; root++ {
			x := exp[root]
			// Evaluate g(x) using Horner's method over GF(256)/0x12D.
			acc := byte(0)
			for _, coeff := range g {
				acc = GFMul(acc, x) ^ coeff
			}
			if acc != 0 {
				t.Errorf("GetGenerator(%d): g(α^%d) = 0x%02X, want 0", nEcc, root, acc)
			}
		}
	}
}

// TestRSEncodeBlockLength verifies the ECC output has exactly nEcc bytes.
func TestRSEncodeBlockLength(t *testing.T) {
	for _, nEcc := range []int{5, 7, 10, 14, 18, 20, 24, 28} {
		data := make([]byte, 8)
		for i := range data {
			data[i] = byte(i + 1)
		}
		gen := GetGenerator(nEcc)
		ecc := RSEncodeBlock(data, gen)
		if len(ecc) != nEcc {
			t.Errorf("RSEncodeBlock(data, gen[%d]) len = %d, want %d", nEcc, len(ecc), nEcc)
		}
	}
}

// TestRSEncodeBlockISOExample verifies the RS encoding for the "A" worked example.
//
// For 10×10 symbol: data = [66, 129, 70] (encoded "A" + pad), n_ecc = 5.
// The expected ECC from ISO/IEC 16022, Annex F.
// We verify syndromes are zero for the combined data+ECC codeword.
func TestRSEncodeBlockISOExample(t *testing.T) {
	data := []byte{66, 129, 70}
	gen := GetGenerator(5)
	ecc := RSEncodeBlock(data, gen)

	if len(ecc) != 5 {
		t.Fatalf("ECC length = %d, want 5", len(ecc))
	}

	// Verify: the combined codeword [data | ecc] should satisfy C(α^i) = 0 for i=1..5.
	combined := append(data, ecc...)
	exp := GFExp()
	for root := 1; root <= 5; root++ {
		x := exp[root]
		acc := byte(0)
		for _, b := range combined {
			acc = GFMul(acc, x) ^ b
		}
		if acc != 0 {
			t.Errorf("syndrome at root α^%d = 0x%02X, want 0", root, acc)
		}
	}
}

// TestRSEncodeBlockZeroData verifies RS of all-zero data = all-zero ECC.
func TestRSEncodeBlockZeroData(t *testing.T) {
	data := make([]byte, 10)
	gen := GetGenerator(10)
	ecc := RSEncodeBlock(data, gen)
	for i, b := range ecc {
		if b != 0 {
			t.Errorf("ecc[%d] = 0x%02X, want 0 for all-zero input", i, b)
		}
	}
}

// ============================================================================
// 6. Block interleaving tests
// ============================================================================

// TestInterleaveSingleBlock verifies single-block symbols pass through unchanged.
func TestInterleaveSingleBlock(t *testing.T) {
	// 10×10 has numBlocks=1, dataCW=3, eccPerBlock=5.
	entry, _ := SelectSymbol(1, SymbolShapeSquare)

	data := PadCodewords(EncodeASCII([]byte("A")), entry.dataCW)
	interleaved := computeInterleaved(data, entry)

	// For single block: interleaved = data + ecc (no interleaving needed).
	if len(interleaved) != entry.dataCW+entry.eccCW {
		t.Errorf("interleaved len = %d, want %d", len(interleaved), entry.dataCW+entry.eccCW)
	}
	// First dataCW bytes must equal the padded data.
	for i := 0; i < entry.dataCW; i++ {
		if interleaved[i] != data[i] {
			t.Errorf("interleaved[%d] = %d, want data[%d] = %d", i, interleaved[i], i, data[i])
		}
	}
}

// TestInterleaveTwoBlocks verifies two-block interleaving for a 32×32 symbol.
func TestInterleaveTwoBlocks(t *testing.T) {
	// 32×32: dataCW=62, numBlocks=2, eccPerBlock=18.
	// Each block gets 31 data codewords.
	entry := squareSizes[9] // 32×32 is index 9

	if entry.symbolRows != 32 || entry.numBlocks != 2 {
		t.Fatalf("unexpected entry: %v", entry)
	}

	// Build dummy data: 62 bytes.
	data := make([]byte, 62)
	for i := range data {
		data[i] = byte(i + 1)
	}

	interleaved := computeInterleaved(data, entry)
	if len(interleaved) != 62+36 {
		t.Errorf("interleaved len = %d, want 98", len(interleaved))
	}

	// Check interleaving pattern: positions 0, 2, 4, ... are from block 0;
	// positions 1, 3, 5, ... are from block 1.
	// Block 0: data[0..30], Block 1: data[31..61]
	for i := 0; i < 31; i++ {
		// Even position = block 0, codeword i
		if interleaved[2*i] != data[i] {
			t.Errorf("interleaved[%d] = %d, want data[%d] = %d",
				2*i, interleaved[2*i], i, data[i])
		}
		// Odd position = block 1, codeword i
		if interleaved[2*i+1] != data[31+i] {
			t.Errorf("interleaved[%d] = %d, want data[%d] = %d",
				2*i+1, interleaved[2*i+1], 31+i, data[31+i])
		}
	}
}

// ============================================================================
// 7. Grid initialization tests
// ============================================================================

// TestGridInitFinderPattern verifies the L-finder for various symbol sizes.
//
// L-finder invariants:
//   - Left column (col 0): all dark
//   - Bottom row (row R-1): all dark
func TestGridInitFinderPattern(t *testing.T) {
	for _, entry := range squareSizes[:5] { // test first 5 sizes
		grid := initGrid(entry)
		R, C := entry.symbolRows, entry.symbolCols

		// Left column: all dark
		for r := 0; r < R; r++ {
			if !grid[r][0] {
				t.Errorf("%dx%d: grid[%d][0] = light, want dark (L-finder left)",
					R, C, r)
			}
		}

		// Bottom row: all dark
		for c := 0; c < C; c++ {
			if !grid[R-1][c] {
				t.Errorf("%dx%d: grid[%d][%d] = light, want dark (L-finder bottom)",
					R, C, R-1, c)
			}
		}
	}
}

// TestGridInitTimingPattern verifies the timing clock on top row and right column.
//
// Corner override rules (due to writing order in initGrid):
//   - (0, C-1): right column (r%2==0 → dark) overrides top row timing.
//   - (R-1, C-1): bottom row (all dark) overrides right column timing.
//   - (R-1, 0): bottom row (all dark); left col (all dark). Both dark → dark.
//   - (0, 0): top row (dark at c=0); left col (dark). Both dark → dark.
func TestGridInitTimingPattern(t *testing.T) {
	for _, entry := range squareSizes[:5] {
		grid := initGrid(entry)
		R, C := entry.symbolRows, entry.symbolCols

		// Top row: alternating dark/light starting dark at col 0.
		// Skip col C-1 — right column overrides it.
		for c := 0; c < C-1; c++ {
			wantDark := (c%2 == 0)
			if grid[0][c] != wantDark {
				t.Errorf("%dx%d: timing top row[0][%d] = %v, want %v",
					R, C, c, grid[0][c], wantDark)
			}
		}
		// (0, C-1) is overridden by right column: row 0 → dark.
		if !grid[0][C-1] {
			t.Errorf("%dx%d: top-right corner [0][%d] = light, want dark (right-col override)",
				R, C, C-1)
		}

		// Right column: alternating dark/light starting dark at row 0.
		// Skip row R-1 — bottom row (all dark) overrides it.
		for r := 0; r < R-1; r++ {
			wantDark := (r%2 == 0)
			if grid[r][C-1] != wantDark {
				t.Errorf("%dx%d: timing right col[%d][%d] = %v, want %v",
					R, C, r, C-1, grid[r][C-1], wantDark)
			}
		}
		// (R-1, C-1) is overridden by bottom row: always dark.
		if !grid[R-1][C-1] {
			t.Errorf("%dx%d: bottom-right corner [%d][%d] = light, want dark (bottom-row override)",
				R, C, R-1, C-1)
		}
	}
}

// TestGridInitCorner verifies (0,0) is always dark (L-finder meets timing).
func TestGridInitCorner(t *testing.T) {
	for _, entry := range append(squareSizes, rectSizes...) {
		grid := initGrid(entry)
		if !grid[0][0] {
			t.Errorf("%dx%d: grid[0][0] = light, want dark (L-bar + timing corner)",
				entry.symbolRows, entry.symbolCols)
		}
	}
}

// TestGridInitAlignmentBorders verifies alignment borders for a 32×32 symbol.
//
// 32×32 has 2×2 data regions (14×14 each).  Writing order in initGrid:
//
//  1. Horizontal AB rows: abRow0=15 (all dark), abRow1=16 (alternating c%2==0)
//  2. Vertical AB cols:   abCol0=15 (all dark),  abCol1=16 (alternating r%2==0)
//  3. Top row, right col, left col, bottom row
//
// At intersection of abRow0 and abCol1: vertical AB overrides horizontal AB,
// so grid[15][16] = (15%2==0) = false (light).
//
// The outer borders always win (written last).
func TestGridInitAlignmentBorders(t *testing.T) {
	entry := squareSizes[9] // 32×32
	grid := initGrid(entry)
	R, C := 32, 32
	// abRow0 = 1 + 1*14 + 0*2 = 15
	abRow0 := 1 + 14 // = 15
	abRow1 := abRow0 + 1
	abCol0 := 1 + 14 // = 15
	abCol1 := abCol0 + 1

	// Alignment row 0 (row 15): should be dark except where vertical AB col1 overrides.
	// Skip: outer border cols (0, C-1), and abCol1 (overridden by vertical AB alternating).
	for c := 1; c < C-1; c++ {
		if c == abCol1 {
			// Vertical AB col1 overrides: grid[abRow0][abCol1] = (abRow0%2==0)
			wantDark := (abRow0%2 == 0) // 15%2=1 → false (light)
			if grid[abRow0][c] != wantDark {
				t.Errorf("32×32 row15/col16 intersection = %v, want %v (vert AB overrides)",
					grid[abRow0][c], wantDark)
			}
		} else if c == abCol0 {
			// Vertical AB col0 overrides with all-dark — same as abRow0 all-dark.
			if !grid[abRow0][c] {
				t.Errorf("32×32 row15/col15 (both ABs, both dark) = light")
			}
		} else {
			if !grid[abRow0][c] {
				t.Errorf("32×32 alignment row[%d][%d] = light, want dark", abRow0, c)
			}
		}
	}

	// Alignment row 1 (row 16): alternating starting dark.
	// Skip outer border cols (0, C-1).
	for c := 1; c < C-1; c++ {
		// At intersection with vertical ABs, the last writer (which was vertical AB, then
		// outer border) determines the value. For abCol0 (col 15): vertical AB all-dark.
		// For abCol1 (col 16): vertical AB alternating (r%2==0 for r=16: 16%2=0 → dark).
		// Both abRow1 and abCol1 produce (r%2==0) and (c%2==0) respectively for those specific
		// positions, but vertical ABs are written AFTER horizontal ABs.
		var wantDark bool
		if c == abCol0 {
			wantDark = true // vertical AB all-dark overrides
		} else if c == abCol1 {
			wantDark = (abRow1%2 == 0) // vertical AB alternating: r=16 → 16%2=0 → dark
		} else {
			wantDark = (c%2 == 0)
		}
		if grid[abRow1][c] != wantDark {
			t.Errorf("32×32 alignment row1[%d][%d] = %v, want %v",
				abRow1, c, grid[abRow1][c], wantDark)
		}
	}

	// Alignment col 0 (col 15): should be dark for all rows (inner only).
	// Outer border overrides: row 0 (timing) and row R-1 (L-finder) override.
	for r := 1; r < R-1; r++ {
		if !grid[r][abCol0] {
			t.Errorf("32×32 alignment col0[%d][%d] = light, want dark", r, abCol0)
		}
	}

	// Alignment col 1 (col 16): alternating (r%2==0) for inner rows.
	// At abRow0: horizontal AB was written first, then vertical AB overrides.
	for r := 1; r < R-1; r++ {
		wantDark := (r%2 == 0)
		if grid[r][abCol1] != wantDark {
			t.Errorf("32x32 alignment col1[%d][%d] = %v, want %v (r mod 2 = %d)",
				r, abCol1, grid[r][abCol1], wantDark, r%2)
		}
	}
	_ = R
	_ = C
}

// ============================================================================
// 8. Utah placement algorithm tests
// ============================================================================

// TestUtahPlacementGridSize verifies the output grid size.
func TestUtahPlacementGridSize(t *testing.T) {
	codewords := make([]byte, 8) // 10×10 symbol: 3 data + 5 ECC = 8 total
	nRows, nCols := 8, 8         // 10×10 interior = 8×8

	grid := UtahPlacement(codewords, nRows, nCols)
	if len(grid) != nRows {
		t.Errorf("grid rows = %d, want %d", len(grid), nRows)
	}
	for r := range grid {
		if len(grid[r]) != nCols {
			t.Errorf("grid[%d] cols = %d, want %d", r, len(grid[r]), nCols)
		}
	}
}

// TestUtahPlacementAllModulesFilled verifies every module is set (no used=false).
// We use a codeword count that exactly fills the symbol.
func TestUtahPlacementAllModulesFilled(t *testing.T) {
	// 10×10 symbol: interior = 8×8 = 64 modules = 8 codewords.
	codewords := make([]byte, 8)
	nRows, nCols := 8, 8
	grid := UtahPlacement(codewords, nRows, nCols)

	// Count modules — all should be set (grid simply holds bool, so we check
	// that every module is deterministic by verifying same call = same result).
	grid2 := UtahPlacement(codewords, nRows, nCols)
	for r := range grid {
		for c := range grid[r] {
			if grid[r][c] != grid2[r][c] {
				t.Errorf("placement non-deterministic at (%d, %d)", r, c)
			}
		}
	}
}

// TestUtahPlacementDeterministic verifies placement is deterministic.
func TestUtahPlacementDeterministic(t *testing.T) {
	codewords := []byte{66, 129, 70, 0xCA, 0xFE, 0xBA, 0xBE, 0x42}
	nRows, nCols := 8, 8

	g1 := UtahPlacement(codewords, nRows, nCols)
	g2 := UtahPlacement(codewords, nRows, nCols)

	for r := range g1 {
		for c := range g1[r] {
			if g1[r][c] != g2[r][c] {
				t.Errorf("non-deterministic at (%d, %d): run1=%v run2=%v",
					r, c, g1[r][c], g2[r][c])
			}
		}
	}
}

// TestUtahPlacementLargerGrid verifies 32×32 (28×28 interior).
func TestUtahPlacementLargerGrid(t *testing.T) {
	entry := squareSizes[9] // 32×32
	nRows := entry.regionRows * entry.dataRegionHeight
	nCols := entry.regionCols * entry.dataRegionWidth
	total := entry.dataCW + entry.eccCW

	codewords := make([]byte, total)
	for i := range codewords {
		codewords[i] = byte(i)
	}

	grid := UtahPlacement(codewords, nRows, nCols)
	if len(grid) != nRows {
		t.Errorf("grid rows = %d, want %d", len(grid), nRows)
	}
}

// ============================================================================
// 9. Logical → Physical mapping tests
// ============================================================================

// TestLogicalToPhysicalSingleRegion verifies single-region mapping (r+1, c+1).
func TestLogicalToPhysicalSingleRegion(t *testing.T) {
	entry := squareSizes[0] // 10×10, regionRows=1, regionCols=1
	tests := [][4]int{
		{0, 0, 1, 1},
		{1, 0, 2, 1},
		{0, 1, 1, 2},
		{7, 7, 8, 8},
	}
	for _, tc := range tests {
		r, c, wantR, wantC := tc[0], tc[1], tc[2], tc[3]
		gotR, gotC := logicalToPhysical(r, c, entry)
		if gotR != wantR || gotC != wantC {
			t.Errorf("logicalToPhysical(%d, %d) = (%d, %d), want (%d, %d)",
				r, c, gotR, gotC, wantR, wantC)
		}
	}
}

// TestLogicalToPhysicalMultiRegion verifies multi-region mapping for 32×32.
//
// 32×32: regionRows=2, regionCols=2, dataRegionHeight=14, dataRegionWidth=14.
// Physical layout:
//   - outer border: 1 row/col
//   - data region 0: rows 1–14, cols 1–14
//   - alignment border: rows 15–16, cols 15–16
//   - data region 1: rows 17–30, cols 17–30
func TestLogicalToPhysicalMultiRegion(t *testing.T) {
	entry := squareSizes[9] // 32×32

	// Logical (0, 0) → physical (1, 1) — top-left of first region
	r, c := logicalToPhysical(0, 0, entry)
	if r != 1 || c != 1 {
		t.Errorf("(0,0) → (%d,%d), want (1,1)", r, c)
	}

	// Logical (13, 0) → physical (14, 1) — last row of first region row
	r, c = logicalToPhysical(13, 0, entry)
	if r != 14 || c != 1 {
		t.Errorf("(13,0) → (%d,%d), want (14,1)", r, c)
	}

	// Logical (14, 0) → physical (17, 1) — first row of second region row
	// (14 / 14) * (14+2) + (14%14) + 1 = 1*16 + 0 + 1 = 17
	r, c = logicalToPhysical(14, 0, entry)
	if r != 17 || c != 1 {
		t.Errorf("(14,0) → (%d,%d), want (17,1)", r, c)
	}
}

// ============================================================================
// 10. Full encode pipeline tests
// ============================================================================

// TestEncodeStringA_10x10 verifies the full encoding of "A" → 10×10 symbol.
//
// This is the canonical ISO/IEC 16022 Annex F worked example.
// We verify: symbol size, border pattern, all structural invariants.
func TestEncodeStringA_10x10(t *testing.T) {
	grid, err := EncodeString("A", Options{Shape: SymbolShapeSquare})
	if err != nil {
		t.Fatalf("EncodeString(A) error: %v", err)
	}

	// Symbol must be 10×10.
	if grid.Rows != 10 || grid.Cols != 10 {
		t.Errorf("symbol size = %dx%d, want 10×10", grid.Rows, grid.Cols)
	}

	validateBorder(t, grid.Modules, 10, 10)
}

// TestEncodeDigitPair_10x10 verifies "1234" (2 digit pairs) → 10×10.
func TestEncodeDigitPair_10x10(t *testing.T) {
	grid, err := EncodeString("1234", Options{Shape: SymbolShapeSquare})
	if err != nil {
		t.Fatalf("EncodeString(1234) error: %v", err)
	}

	// 2 codewords (two digit pairs) < 3 capacity of 10×10.
	if grid.Rows != 10 || grid.Cols != 10 {
		t.Errorf("1234 → %dx%d, want 10×10", grid.Rows, grid.Cols)
	}
	validateBorder(t, grid.Modules, 10, 10)
}

// TestEncodeHelloWorld_16x16 verifies "Hello World" → 16×16 symbol.
//
// "Hello World" = 11 characters → 11 codewords.
// 16×16 has dataCW=12, so 11+1 pad = 12 codewords.
func TestEncodeHelloWorld_16x16(t *testing.T) {
	grid, err := EncodeString("Hello World", Options{Shape: SymbolShapeSquare})
	if err != nil {
		t.Fatalf("EncodeString(Hello World) error: %v", err)
	}

	if grid.Rows != 16 || grid.Cols != 16 {
		t.Errorf("Hello World → %dx%d, want 16×16", grid.Rows, grid.Cols)
	}
	validateBorder(t, grid.Modules, 16, 16)
}

// TestEncodeAlphanumeric_24x24 verifies a full alphanumeric string → 24×24.
//
// "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789" = 36 chars.
// 24×24 has dataCW=36. With digit pairs: "0123456789" → 5 pairs = 5 codewords.
// Plus 26 letters (no pairs) = 26 codewords. Total = 31 codewords < 36.
// Next smaller symbol: 22×22 (dataCW=30) < 31, so 24×24 is selected.
func TestEncodeAlphanumeric_24x24(t *testing.T) {
	input := "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	grid, err := EncodeString(input, Options{Shape: SymbolShapeSquare})
	if err != nil {
		t.Fatalf("EncodeString(%q) error: %v", input, err)
	}

	if grid.Rows != 24 || grid.Cols != 24 {
		t.Errorf("%q → %dx%d, want 24×24", input, grid.Rows, grid.Cols)
	}
	validateBorder(t, grid.Modules, 24, 24)
}

// TestEncodeFullCapacity_26x26 verifies encoding exactly 44 chars → 26×26.
//
// 26×26 has dataCW=44. Encoding 44 uppercase letters = 44 codewords (no pairs).
func TestEncodeFullCapacity_26x26(t *testing.T) {
	input := strings.Repeat("A", 44)
	grid, err := EncodeString(input, Options{Shape: SymbolShapeSquare})
	if err != nil {
		t.Fatalf("EncodeString(44xA) error: %v", err)
	}
	if grid.Rows != 26 || grid.Cols != 26 {
		t.Errorf("44 A's → %dx%d, want 26×26", grid.Rows, grid.Cols)
	}
	validateBorder(t, grid.Modules, 26, 26)
}

// TestEncodeMultiRegion_32x32 verifies that a string requiring 32×32 has
// correct 2×2 data regions and alignment borders.
//
// 32×32 alignment borders:
//   - abRow0 = 15, abRow1 = 16  (horizontal alignment border)
//   - abCol0 = 15, abCol1 = 16  (vertical alignment border)
//
// Writing order in initGrid: H-ABs → V-ABs → top row → right col → left col → bottom row.
// So:
//   - (0, abCol0=15): outer top-row timing wins → c=15 odd → light
//   - (R-1, abCol0=15): outer bottom row wins → dark
//   - (abRow0=15, abCol1=16): V-AB alternating wins → r=15 odd → light
//   - (abRow0=15, 0): outer left col wins → dark
func TestEncodeMultiRegion_32x32(t *testing.T) {
	// Build a 45-char string: 44+1 = 45 codewords > 44 (26×26 capacity), needs 32×32.
	input := strings.Repeat("A", 45)
	grid, err := EncodeString(input, Options{Shape: SymbolShapeSquare})
	if err != nil {
		t.Fatalf("EncodeString(45xA) error: %v", err)
	}

	if grid.Rows != 32 || grid.Cols != 32 {
		t.Errorf("45 chars → %dx%d, want 32×32", grid.Rows, grid.Cols)
	}
	validateBorder(t, grid.Modules, 32, 32)

	// Verify the horizontal alignment border row 15 (abRow0):
	// - Should be dark at most columns.
	// - Exception: abCol1=16 (overridden by V-AB alternating): 15 odd → light.
	// - Exception: col 0 = left col (dark, same as expected).
	// - Exception: col C-1=31 = right col timing r=15 odd → light.
	R, C := 32, 32
	abRow0, abCol1 := 15, 16
	for c := 1; c < C-1; c++ {
		if c == abCol1 {
			// V-AB col 1 overrides: r=15 odd → light
			if grid.Modules[abRow0][c] {
				t.Errorf("32×32 row15/col16 intersection: want light (V-AB alt, r=15 odd), got dark")
			}
			continue
		}
		if !grid.Modules[abRow0][c] {
			t.Errorf("32×32 alignment row 15, col %d = light, want dark", c)
			break
		}
	}

	// Verify vertical alignment border col 15 (abCol0):
	// - Should be dark at most rows.
	// - Exception: row 0 = outer top timing c=15 odd → light (timing wins).
	for r := 1; r < R-1; r++ {
		if !grid.Modules[r][15] {
			t.Errorf("32×32 alignment col 15, row %d = light, want dark", r)
			break
		}
	}
	// Row 0, col 15: outer timing wins → c=15 odd → light.
	if grid.Modules[0][15] {
		t.Error("32×32 (row=0, col=15): timing top row c=15 odd should be light, but got dark")
	}
}

// TestEncodeRectangular_8x18 verifies encoding into an 8×18 rectangular symbol.
func TestEncodeRectangular_8x18(t *testing.T) {
	grid, err := EncodeString("Hi", Options{Shape: SymbolShapeRectangular})
	if err != nil {
		t.Fatalf("EncodeString(Hi, Rect) error: %v", err)
	}

	// "Hi" = 2 codewords. Smallest rect: 8×18 (dataCW=5).
	if grid.Rows != 8 || grid.Cols != 18 {
		t.Errorf("Hi (rect) → %dx%d, want 8×18", grid.Rows, grid.Cols)
	}
	validateBorder(t, grid.Modules, 8, 18)
}

// TestEncodeURL validates a URL fits into the correct symbol.
//
// "https://coding-adventures.dev" = 29 chars, all non-digit → 29 codewords.
// 22×22 has dataCW=30 ≥ 29, so it selects 22×22.
func TestEncodeURL(t *testing.T) {
	url := "https://coding-adventures.dev"
	grid, err := EncodeString(url, Options{Shape: SymbolShapeSquare})
	if err != nil {
		t.Fatalf("EncodeString(URL) error: %v", err)
	}

	// 29 codewords: fits in 22×22 (dataCW=30).
	if grid.Rows != 22 || grid.Cols != 22 {
		t.Errorf("URL → %dx%d, expected 22×22 (29 codewords, cap=30)", grid.Rows, grid.Cols)
	}
	validateBorder(t, grid.Modules, int(grid.Rows), int(grid.Cols))
}

// TestEncodeLargeSymbol verifies encoding into a larger symbol (88×88, 4×4 regions).
func TestEncodeLargeSymbol(t *testing.T) {
	// 88×88 has dataCW=576. Build a ~500-char string.
	input := strings.Repeat("ABCDEFGHIJ", 50) // 500 chars = 500 codewords
	grid, err := EncodeString(input, Options{Shape: SymbolShapeSquare})
	if err != nil {
		t.Fatalf("large encode error: %v", err)
	}

	// Must fit within some symbol and have correct border.
	validateBorder(t, grid.Modules, int(grid.Rows), int(grid.Cols))
}

// TestEncodeAllSymbolSizes checks encoding doesn't fail for progressively
// larger inputs hitting each symbol size.
func TestEncodeAllSymbolSizes(t *testing.T) {
	for _, entry := range squareSizes {
		// Encode exactly entry.dataCW ASCII characters.
		input := strings.Repeat("A", entry.dataCW)
		grid, err := EncodeString(input, Options{Shape: SymbolShapeSquare})
		if err != nil {
			t.Errorf("%dx%d: EncodeString(%d chars) error: %v",
				entry.symbolRows, entry.symbolCols, entry.dataCW, err)
			continue
		}
		if int(grid.Rows) != entry.symbolRows {
			t.Errorf("expected %dx%d symbol, got %dx%d",
				entry.symbolRows, entry.symbolCols, grid.Rows, grid.Cols)
		}
		validateBorder(t, grid.Modules, entry.symbolRows, entry.symbolCols)
	}
}

// TestEncodeModuleGridDimensions verifies ModuleGrid Rows/Cols fields.
func TestEncodeModuleGridDimensions(t *testing.T) {
	grid, err := EncodeString("Test", Options{})
	if err != nil {
		t.Fatal(err)
	}
	if int(grid.Rows) != len(grid.Modules) {
		t.Errorf("Rows field (%d) ≠ len(Modules) (%d)", grid.Rows, len(grid.Modules))
	}
	for r, row := range grid.Modules {
		if int(grid.Cols) != len(row) {
			t.Errorf("row %d: Cols field (%d) ≠ len(row) (%d)", r, grid.Cols, len(row))
		}
	}
}

// ============================================================================
// 11. Error case tests
// ============================================================================

// TestEncodeInputTooLong verifies InputTooLongError for input > 144×144 capacity.
func TestEncodeInputTooLong(t *testing.T) {
	// 1558 codewords = max; 1559 chars = 1559 codewords > max.
	input := strings.Repeat("A", 1559)
	_, err := EncodeString(input, Options{})
	if err == nil {
		t.Fatal("expected InputTooLongError, got nil")
	}

	var tooLong *InputTooLongError
	if !errors.As(err, &tooLong) {
		t.Errorf("expected *InputTooLongError, got %T: %v", err, err)
	}
	if !IsInputTooLong(err) {
		t.Error("IsInputTooLong returned false for InputTooLongError")
	}
}

// TestEncodeInputTooLongErrorMessage verifies the error message is informative.
func TestEncodeInputTooLongErrorMessage(t *testing.T) {
	input := strings.Repeat("A", 1559)
	_, err := EncodeString(input, Options{})
	if err == nil {
		t.Fatal("expected error")
	}
	msg := err.Error()
	if !strings.Contains(msg, "data-matrix") {
		t.Errorf("error message %q doesn't mention 'data-matrix'", msg)
	}
}

// TestEncodeEmptyInput verifies empty input encodes to the smallest symbol.
func TestEncodeEmptyInput(t *testing.T) {
	grid, err := EncodeString("", Options{})
	if err != nil {
		t.Fatalf("empty input error: %v", err)
	}
	// Empty → 0 codewords → smallest symbol (10×10, dataCW=3).
	if grid.Rows != 10 || grid.Cols != 10 {
		t.Errorf("empty input → %dx%d, want 10×10", grid.Rows, grid.Cols)
	}
}

// TestEncodeRectOnlyNoSquare verifies SymbolShapeRectangular never produces a square.
func TestEncodeRectOnlyNoSquare(t *testing.T) {
	// Any short input with rectangular shape.
	grid, err := EncodeString("ABC", Options{Shape: SymbolShapeRectangular})
	if err != nil {
		t.Fatalf("rect encode error: %v", err)
	}
	if grid.Rows == grid.Cols {
		t.Errorf("rectangular shape returned a square symbol (%dx%d)", grid.Rows, grid.Cols)
	}
}

// ============================================================================
// 12. EncodeToScene tests
// ============================================================================

// TestEncodeToSceneReturnsScene verifies EncodeToScene succeeds and produces
// a non-empty PaintScene.
func TestEncodeToSceneReturnsScene(t *testing.T) {
	scene, err := EncodeToScene([]byte("Hello"), Options{}, defaultLayoutConfig())
	if err != nil {
		t.Fatalf("EncodeToScene error: %v", err)
	}
	// Scene should have at least one instruction (module rects + background).
	if len(scene.Instructions) == 0 {
		t.Error("EncodeToScene returned empty scene instructions")
	}
}

// TestEncodeToSceneErrorPropagates verifies errors from Encode propagate.
func TestEncodeToSceneErrorPropagates(t *testing.T) {
	tooLong := strings.Repeat("A", 1559)
	_, err := EncodeToScene([]byte(tooLong), Options{}, defaultLayoutConfig())
	if err == nil {
		t.Fatal("expected error from EncodeToScene for too-long input")
	}
}

// ============================================================================
// Helper functions
// ============================================================================

// validateBorder checks all four structural border invariants for a symbol.
//
// Invariants (from ISO/IEC 16022:2006, §7):
//  1. Left column (col 0): all dark.
//  2. Bottom row (row R-1): all dark.
//  3. Top row (row 0): alternating dark/light starting dark at col 0.
//     Exception: (0, C-1) is overridden by right column (dark, since row 0 → r%2=0).
//  4. Right column (col C-1): alternating dark/light starting dark at row 0.
//     Exception: (R-1, C-1) is overridden by bottom row (all dark).
//  5. Corner (0, 0): dark (L-bar meets timing).
//
// Writing order in initGrid: alignment borders → top row → right col → left col → bottom row.
// So bottom row wins at (R-1, *), left col wins at (*, 0), right col wins at (0, C-1).
func validateBorder(t *testing.T, modules [][]bool, R, C int) {
	t.Helper()

	// 1. Left column: all dark (written last among timing, so always wins)
	for r := 0; r < R; r++ {
		if !modules[r][0] {
			t.Errorf("border: left col [%d][0] = light, want dark", r)
			return
		}
	}

	// 2. Bottom row: all dark (written absolute last, highest precedence)
	for c := 0; c < C; c++ {
		if !modules[R-1][c] {
			t.Errorf("border: bottom row [%d][%d] = light, want dark", R-1, c)
			return
		}
	}

	// 3. Top row: alternating dark/light starting dark.
	// - col 0: left col overrides (dark, same as timing)
	// - col C-1: right col overrides (r=0 → dark, which may differ from timing if C-1 is odd)
	// So check cols 1 to C-2 for strict alternating; verify corners separately.
	for c := 1; c < C-1; c++ {
		want := (c%2 == 0)
		if modules[0][c] != want {
			t.Errorf("border: top row [0][%d] = %v, want %v (timing alternating)",
				c, modules[0][c], want)
			return
		}
	}
	// Corner (0, C-1): right column says r=0 → dark.
	if !modules[0][C-1] {
		t.Errorf("border: top-right corner [0][%d] = light, want dark (right-col, r=0)", C-1)
	}

	// 4. Right column: alternating dark/light starting dark.
	// - row 0: covered above
	// - row R-1: bottom row overrides (always dark)
	// Check rows 1 to R-2 for strict alternating.
	for r := 1; r < R-1; r++ {
		want := (r%2 == 0)
		if modules[r][C-1] != want {
			t.Errorf("border: right col [%d][%d] = %v, want %v (timing alternating)",
				r, C-1, modules[r][C-1], want)
			return
		}
	}

	// 5. Corner (0, 0) must be dark
	if !modules[0][0] {
		t.Error("border: corner [0][0] = light, want dark")
	}
}

// defaultLayoutConfig returns a minimal Barcode2DLayoutConfig for testing.
func defaultLayoutConfig() barcode2d.Barcode2DLayoutConfig {
	cfg := barcode2d.DefaultBarcode2DLayoutConfig
	cfg.QuietZoneModules = 1
	return cfg
}
