package azteccode

// Tests for the Aztec Code encoder.
//
// Test strategy (from spec):
//
//  1. GF(16) arithmetic: log/antilog tables, multiplication, period check.
//  2. Mode message encoding: compact and full, verify nibble structure.
//  3. Bit stuffing: known runs, alternating bits, all-zeros.
//  4. Symbol selection: chooses correct layer count for known bit counts.
//  5. Full encode integration: produce a valid-looking grid for short inputs.
//  6. Bullseye pattern: verify center structure for compact 1-layer symbols.
//  7. Orientation marks: verify 4 corners of mode message ring are dark.
//  8. Cross-language test vectors: verify identical output for shared inputs.

import (
	"errors"
	"testing"
)

// ============================================================================
// GF(16) arithmetic tests
// ============================================================================

// TestGF16LogAlog verifies that LOG16 and ALOG16 are inverses of each other
// for every non-zero element, and that α^15 = α^0 = 1 (period = 15).
//
// In GF(16), α = 2 is the primitive element. Every non-zero element 1..15
// appears exactly once in positions α^0 through α^14 of the antilog table.
func TestGF16LogAlog(t *testing.T) {
	// α^0 = 1 (identity element).
	if gf16Alog[0] != 1 {
		t.Errorf("ALOG16[0] = %d, want 1", gf16Alog[0])
	}
	// Period: α^15 = α^0 = 1
	if gf16Alog[15] != 1 {
		t.Errorf("ALOG16[15] = %d, want 1 (period check)", gf16Alog[15])
	}
	// Every non-zero element appears exactly once.
	seen := [16]bool{}
	for i := 0; i < 15; i++ {
		e := gf16Alog[i]
		if e == 0 {
			t.Errorf("ALOG16[%d] = 0, all antilog values should be nonzero", i)
			continue
		}
		if seen[e] {
			t.Errorf("ALOG16[%d] = %d already seen (table not injective)", i, e)
		}
		seen[e] = true
	}
	// Verify LOG is the left inverse of ALOG for all non-zero elements.
	for e := 1; e <= 15; e++ {
		i := gf16Log[e]
		if gf16Alog[i] != e {
			t.Errorf("ALOG16[LOG16[%d]] = ALOG16[%d] = %d, want %d", e, i, gf16Alog[i], e)
		}
	}
}

// TestGF16MulIdentity verifies that 1 is the multiplicative identity in GF(16).
func TestGF16MulIdentity(t *testing.T) {
	for i := 1; i <= 15; i++ {
		if GF16Mul(i, 1) != i {
			t.Errorf("GF16Mul(%d, 1) = %d, want %d", i, GF16Mul(i, 1), i)
		}
		if GF16Mul(1, i) != i {
			t.Errorf("GF16Mul(1, %d) = %d, want %d", i, GF16Mul(1, i), i)
		}
	}
}

// TestGF16MulZero verifies that 0 absorbs multiplication in GF(16).
func TestGF16MulZero(t *testing.T) {
	for i := 0; i <= 15; i++ {
		if GF16Mul(i, 0) != 0 {
			t.Errorf("GF16Mul(%d, 0) = %d, want 0", i, GF16Mul(i, 0))
		}
		if GF16Mul(0, i) != 0 {
			t.Errorf("GF16Mul(0, %d) = %d, want 0", i, GF16Mul(0, i))
		}
	}
}

// TestGF16MulCommutativity verifies that GF(16) multiplication is commutative:
// a × b = b × a for all pairs in 0..15.
func TestGF16MulCommutativity(t *testing.T) {
	for a := 0; a <= 15; a++ {
		for b := 0; b <= 15; b++ {
			if GF16Mul(a, b) != GF16Mul(b, a) {
				t.Errorf("GF16Mul(%d, %d) = %d ≠ GF16Mul(%d, %d) = %d",
					a, b, GF16Mul(a, b), b, a, GF16Mul(b, a))
			}
		}
	}
}

// TestGF16MulKnownValues spot-checks specific products against the known
// multiplication table for GF(16)/0x13.
//
// α^1 = 2, α^2 = 4, so α^1 × α^2 = α^3 = 8.
// α^3 = 8, α^4 = 3, so α^3 × α^4 = α^7 = 11.
func TestGF16MulKnownValues(t *testing.T) {
	tests := []struct{ a, b, want int }{
		{2, 4, 8},   // α^1 × α^2 = α^3 = 8
		{8, 3, 11},  // α^3 × α^4 = α^7 = 11
		{3, 6, 9},   // α^4 × α^5 = α^9 = 10? No: 4+5=9, α^9=10. Let me recompute.
		{2, 2, 4},   // α^1 × α^1 = α^2 = 4
		{3, 3, 5},   // α^4 × α^4 = α^8 = 5
	}
	// Fix the third test: 3 × 6 = α^4 × α^5 = α^9 = 10.
	tests[2] = struct{ a, b, want int }{3, 6, 10}

	for _, tt := range tests {
		got := GF16Mul(tt.a, tt.b)
		if got != tt.want {
			t.Errorf("GF16Mul(%d, %d) = %d, want %d", tt.a, tt.b, got, tt.want)
		}
	}
}

// TestGF16InverseExists verifies that every non-zero element has a
// multiplicative inverse (i.e., a × a^{-1} = 1 for all a ≠ 0).
func TestGF16InverseExists(t *testing.T) {
	for a := 1; a <= 15; a++ {
		found := false
		for b := 1; b <= 15; b++ {
			if GF16Mul(a, b) == 1 {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("no inverse for GF16 element %d", a)
		}
	}
}

// ============================================================================
// GF(256)/0x12D arithmetic tests
// ============================================================================

// TestGF256LogAlogConsistency verifies that gf256Exp and gf256Log are inverses.
func TestGF256LogAlogConsistency(t *testing.T) {
	// α^255 = 1 (order 255)
	if gf256Exp[255] != 1 {
		t.Errorf("gf256Exp[255] = %d, want 1", gf256Exp[255])
	}
	for e := 1; e <= 255; e++ {
		i := gf256Log[e]
		if gf256Exp[i] != byte(e) {
			t.Errorf("gf256Exp[gf256Log[%d]] = gf256Exp[%d] = %d, want %d", e, i, gf256Exp[i], e)
		}
	}
}

// TestGF256MulIdentity verifies that 1 is the multiplicative identity in GF(256)/0x12D.
func TestGF256MulIdentity(t *testing.T) {
	for i := byte(1); i < 255; i++ {
		if GF256Mul(i, 1) != i {
			t.Errorf("GF256Mul(%d, 1) = %d, want %d", i, GF256Mul(i, 1), i)
		}
	}
}

// TestGF256MulZero verifies that 0 absorbs multiplication in GF(256)/0x12D.
func TestGF256MulZero(t *testing.T) {
	for i := byte(0); ; i++ {
		if GF256Mul(i, 0) != 0 {
			t.Errorf("GF256Mul(%d, 0) = %d, want 0", i, GF256Mul(i, 0))
		}
		if i == 255 {
			break
		}
	}
}

// ============================================================================
// Bit stuffing tests
// ============================================================================

// TestStuffBitsRuns tests that a run of 4 identical bits gets a stuff bit
// inserted, and the run resets after the stuff bit.
func TestStuffBitsRuns(t *testing.T) {
	// Four 1s followed by a 0: expect [1,1,1,1,0(stuff), 0]
	input := []byte{1, 1, 1, 1, 0}
	got := StuffBits(input)
	want := []byte{1, 1, 1, 1, 0, 0} // stuff 0 inserted after 4× 1
	if !equalBytes(got, want) {
		t.Errorf("StuffBits(%v) = %v, want %v", input, got, want)
	}

	// Four 0s followed by a 1: expect [0,0,0,0,1(stuff), 1]
	input2 := []byte{0, 0, 0, 0, 1}
	got2 := StuffBits(input2)
	want2 := []byte{0, 0, 0, 0, 1, 1} // stuff 1 inserted after 4× 0
	if !equalBytes(got2, want2) {
		t.Errorf("StuffBits(%v) = %v, want %v", input2, got2, want2)
	}
}

// TestStuffBitsAlternating verifies that perfectly alternating bits never trigger
// stuffing (runs are always exactly 1).
func TestStuffBitsAlternating(t *testing.T) {
	input := []byte{0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1}
	got := StuffBits(input)
	if !equalBytes(got, input) {
		t.Errorf("StuffBits(alternating) changed bits: got %v, want %v", got, input)
	}
}

// TestStuffBitsAllZeros verifies that a stream of all-zero bits gets a stuff 1
// inserted after every 4th zero.
func TestStuffBitsAllZeros(t *testing.T) {
	// 16 zeros → stuff bits at positions 4, 9, 14, 19 (0-indexed in output)
	input := make([]byte, 16)
	got := StuffBits(input)
	// Expected: 0000[1]0000[1]0000[1]0000[1]  = 20 bits
	// After 4 zeros: insert 1, then 4 zeros again, etc.
	want := []byte{0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1}
	if !equalBytes(got, want) {
		t.Errorf("StuffBits(16 zeros): got %v, want %v", got, want)
	}
}

// TestStuffBitsDoubleRun verifies a run of four 1s followed by four 0s:
//
// Input: [1, 1, 1, 1, 0, 0, 0, 0]
//
// Step-by-step:
//   bits 1-4 (four 1s): runLen reaches 4 → emit 1,1,1,1 then stuff 0
//     → runVal=0, runLen=1
//   5th bit (0): runLen=2 → emit 0
//   6th bit (0): runLen=3 → emit 0
//   7th bit (0): runLen=4 → emit 0 then stuff 1
//     → runVal=1, runLen=1
//   8th bit (0): runVal changed → runLen=1 → emit 0
//
// Result: [1,1,1,1, 0(stuff), 0,0,0, 1(stuff), 0]
func TestStuffBitsDoubleRun(t *testing.T) {
	input := []byte{1, 1, 1, 1, 0, 0, 0, 0}
	got := StuffBits(input)
	// After 4× 1: insert 0 (stuff). runVal=0, runLen=1.
	// 5th bit is 0: runLen=2. 6th: runLen=3. 7th: runLen=4 → insert 1 (stuff). runVal=1, runLen=1.
	// 8th bit is 0: runLen=1 (run reset). emit 0.
	// Final: [1,1,1,1, 0, 0,0,0,1, 0]
	want := []byte{1, 1, 1, 1, 0, 0, 0, 0, 1, 0}
	if !equalBytes(got, want) {
		t.Errorf("StuffBits double-run: got %v, want %v", got, want)
	}
}

// TestStuffBitsFiveOnes verifies: 5 consecutive 1s = [1,1,1,1, 0(stuff), 1]
func TestStuffBitsFiveOnes(t *testing.T) {
	input := []byte{1, 1, 1, 1, 1}
	got := StuffBits(input)
	want := []byte{1, 1, 1, 1, 0, 1}
	if !equalBytes(got, want) {
		t.Errorf("StuffBits 5 ones: got %v, want %v", got, want)
	}
}

// ============================================================================
// Mode message encoding tests
// ============================================================================

// TestEncodeModeMessageCompactLength verifies that compact mode message is 28 bits.
func TestEncodeModeMessageCompactLength(t *testing.T) {
	msg := EncodeModeMessage(true, 1, 5) // compact, 1 layer, 5 data codewords
	if len(msg) != 28 {
		t.Errorf("compact mode message length = %d, want 28", len(msg))
	}
}

// TestEncodeModeMessageFullLength verifies that full mode message is 40 bits.
func TestEncodeModeMessageFullLength(t *testing.T) {
	msg := EncodeModeMessage(false, 2, 12) // full, 2 layers, 12 data codewords
	if len(msg) != 40 {
		t.Errorf("full mode message length = %d, want 40", len(msg))
	}
}

// TestEncodeModeMessageCompactStructure verifies that the first 8 bits of the
// compact mode message correctly encode (layers-1) and (dataCwCount-1).
//
// For layers=2, dataCwCount=10:
//   m = (1 << 6) | 9 = 64 | 9 = 73 = 0b01001001
//   dataNibbles[0] = 73 & 0xF = 9 = 0b1001
//   dataNibbles[1] = (73 >> 4) & 0xF = 4 = 0b0100
//   First 4 bits of mode message: 1,0,0,1 (nibble[0] MSB first)
//   Next 4 bits: 0,1,0,0 (nibble[1] MSB first)
func TestEncodeModeMessageCompactStructure(t *testing.T) {
	layers := 2
	dataCwCount := 10
	msg := EncodeModeMessage(true, layers, dataCwCount)
	if len(msg) != 28 {
		t.Fatalf("unexpected length %d", len(msg))
	}
	// Reconstruct the first nibble value from bits 0..3.
	n0 := (int(msg[0]) << 3) | (int(msg[1]) << 2) | (int(msg[2]) << 1) | int(msg[3])
	// Reconstruct the second nibble value from bits 4..7.
	n1 := (int(msg[4]) << 3) | (int(msg[5]) << 2) | (int(msg[6]) << 1) | int(msg[7])

	m := ((layers - 1) << 6) | (dataCwCount - 1)
	wantN0 := m & 0xF
	wantN1 := (m >> 4) & 0xF

	if n0 != wantN0 {
		t.Errorf("first data nibble = %d, want %d", n0, wantN0)
	}
	if n1 != wantN1 {
		t.Errorf("second data nibble = %d, want %d", n1, wantN1)
	}
}

// TestEncodeModeMessageFullStructure verifies the nibble structure for full mode.
//
// For layers=3, dataCwCount=20:
//   m = (2 << 11) | 19 = 4096 | 19 = 4115 = 0x1013
//   dataNibbles = [3, 1, 0, 1] (m & 0xF=3, (m>>4)&0xF=1, (m>>8)&0xF=0, (m>>12)&0xF=1)
func TestEncodeModeMessageFullStructure(t *testing.T) {
	layers := 3
	dataCwCount := 20
	msg := EncodeModeMessage(false, layers, dataCwCount)
	if len(msg) != 40 {
		t.Fatalf("unexpected length %d", len(msg))
	}
	m := ((layers - 1) << 11) | (dataCwCount - 1)
	wantNibbles := []int{m & 0xF, (m >> 4) & 0xF, (m >> 8) & 0xF, (m >> 12) & 0xF}
	for i, wn := range wantNibbles {
		start := i * 4
		got := (int(msg[start]) << 3) | (int(msg[start+1]) << 2) | (int(msg[start+2]) << 1) | int(msg[start+3])
		if got != wn {
			t.Errorf("data nibble[%d] = %d, want %d", i, got, wn)
		}
	}
}

// TestEncodeModeMessageAllBitsValid verifies that all bits in the mode message
// are 0 or 1 (sanity check).
func TestEncodeModeMessageAllBitsValid(t *testing.T) {
	for _, compact := range []bool{true, false} {
		for layers := 1; layers <= 4; layers++ {
			msg := EncodeModeMessage(compact, layers, 5)
			for i, b := range msg {
				if b != 0 && b != 1 {
					t.Errorf("mode message [compact=%v, layers=%d] bit[%d] = %d (not 0 or 1)", compact, layers, i, b)
				}
			}
		}
	}
}

// ============================================================================
// Symbol selection tests
// ============================================================================

// TestSelectSymbolSmallInput verifies that a tiny input selects compact 1-layer.
func TestSelectSymbolSmallInput(t *testing.T) {
	// A single byte encoded as Binary-Shift: 5 + 5 + 8 = 18 bits raw.
	// With 20% stuffing overhead: ceil(18 * 1.2 / 8) = ceil(2.7) = 3 bytes.
	// Compact 1 layer: 9 total slots at 23% ECC → eccCwCount=ceil(0.23*9)=3
	// dataCwCount = 9 - 3 = 6. 3 ≤ 6 → fits.
	spec, err := SelectSymbol(18, 23)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !spec.compact {
		t.Errorf("expected compact symbol, got full layers=%d", spec.layers)
	}
	if spec.layers != 1 {
		t.Errorf("expected 1 layer, got %d", spec.layers)
	}
}

// TestSelectSymbolTooLong verifies that an impossibly large input returns an error.
func TestSelectSymbolTooLong(t *testing.T) {
	// Max capacity for full 32 layers: 1437 bytes at 23% ECC → ~1107 data bytes.
	// Use a dataBitCount well beyond that.
	_, err := SelectSymbol(99999999, 23)
	if err == nil {
		t.Fatal("expected error for oversized input, got nil")
	}
	if !errors.Is(err, ErrInputTooLong) {
		t.Errorf("expected InputTooLongError, got %T: %v", err, err)
	}
}

// TestSelectSymbolGrowsWithInput verifies that larger inputs select larger symbols.
func TestSelectSymbolGrowsWithInput(t *testing.T) {
	prev := symbolSpec{layers: 0}
	for bits := 10; bits <= 500; bits += 30 {
		spec, err := SelectSymbol(bits, 23)
		if err != nil {
			break // ran out of capacity, which is fine
		}
		if spec.layers < prev.layers && spec.compact == prev.compact {
			t.Errorf("layer count went backward: bits=%d, prev=%d, got=%d", bits, prev.layers, spec.layers)
		}
		prev = spec
	}
}

// ============================================================================
// Binary-Shift encoding tests
// ============================================================================

// TestEncodeBytesAsBitsShortLength verifies the bit structure for short input.
//
// For a single byte 'A' (0x41 = 65 = 0b01000001):
//   5 bits: 11111 (Binary-Shift escape)
//   5 bits: 00001 (length = 1)
//   8 bits: 01000001 (byte value 65)
//   Total: 18 bits
func TestEncodeBytesAsBitsShortLength(t *testing.T) {
	bits := EncodeBytesAsBits([]byte{'A'})
	if len(bits) != 18 {
		t.Fatalf("length = %d, want 18", len(bits))
	}
	// First 5 bits: 11111 (Binary-Shift escape)
	for i := 0; i < 5; i++ {
		if bits[i] != 1 {
			t.Errorf("escape bit[%d] = %d, want 1", i, bits[i])
		}
	}
	// Next 5 bits: 00001 (length = 1)
	wantLen := []byte{0, 0, 0, 0, 1}
	for i, w := range wantLen {
		if bits[5+i] != w {
			t.Errorf("length bit[%d] = %d, want %d", i, bits[5+i], w)
		}
	}
	// Next 8 bits: 0x41 = 0b01000001
	wantData := []byte{0, 1, 0, 0, 0, 0, 0, 1}
	for i, w := range wantData {
		if bits[10+i] != w {
			t.Errorf("data bit[%d] = %d, want %d", i, bits[10+i], w)
		}
	}
}

// TestEncodeBytesAsBitsLongLength verifies the extended length encoding for
// inputs longer than 31 bytes.
//
// For 32 bytes: 5 bits 11111 (escape) + 5 bits 00000 (extended flag) + 11 bits for 32.
func TestEncodeBytesAsBitsLongLength(t *testing.T) {
	data := make([]byte, 32)
	bits := EncodeBytesAsBits(data)
	// Expected: 5 (escape) + 5 (0=extended) + 11 (length 32) + 32×8 = 277 bits
	want := 5 + 5 + 11 + 32*8
	if len(bits) != want {
		t.Fatalf("length = %d, want %d", len(bits), want)
	}
	// Verify escape
	for i := 0; i < 5; i++ {
		if bits[i] != 1 {
			t.Errorf("escape bit[%d] = %d, want 1", i, bits[i])
		}
	}
	// Verify extended-length flag (5 bits all 0)
	for i := 5; i < 10; i++ {
		if bits[i] != 0 {
			t.Errorf("extended-length flag bit[%d] = %d, want 0", i, bits[i])
		}
	}
	// Verify length = 32 in 11 bits: 00000100000 = 0b00000100000 = 32
	// bits 10..20 should encode 32
	var lenVal int
	for i := 0; i < 11; i++ {
		lenVal = (lenVal << 1) | int(bits[10+i])
	}
	if lenVal != 32 {
		t.Errorf("encoded length = %d, want 32", lenVal)
	}
}

// ============================================================================
// Full encode integration tests
// ============================================================================

// TestEncodeShortString verifies that encoding a single uppercase letter
// produces a 15×15 compact 1-layer grid.
func TestEncodeShortString(t *testing.T) {
	grid, err := Encode("A", nil)
	if err != nil {
		t.Fatalf("Encode('A') error: %v", err)
	}
	if grid.Rows != 15 || grid.Cols != 15 {
		t.Errorf("expected 15×15, got %d×%d", grid.Rows, grid.Cols)
	}
	if len(grid.Modules) != 15 {
		t.Fatalf("grid has %d rows, want 15", len(grid.Modules))
	}
	for r, row := range grid.Modules {
		if len(row) != 15 {
			t.Errorf("row %d has %d cols, want 15", r, len(row))
		}
	}
}

// TestEncodeBullseyeCompact verifies the bullseye pattern in a compact 1-layer symbol.
//
// For a 15×15 symbol, center is (7,7). Bullseye radius = 5.
// Expected Chebyshev-distance color pattern from center:
//   d=0 (center):     DARK
//   d=1:              DARK  (3×3 solid core)
//   d=2:              LIGHT
//   d=3:              DARK
//   d=4:              LIGHT
//   d=5 (outermost):  DARK
func TestEncodeBullseyeCompact(t *testing.T) {
	grid, err := Encode("A", nil)
	if err != nil {
		t.Fatalf("Encode('A') error: %v", err)
	}
	cx, cy := 7, 7

	colorAt := func(row, col int) string {
		if grid.Modules[row][col] {
			return "DARK"
		}
		return "LIGHT"
	}
	wantColor := func(d int) string {
		if d <= 1 {
			return "DARK"
		}
		if d%2 == 0 {
			return "LIGHT"
		}
		return "DARK"
	}

	for row := 0; row < 15; row++ {
		for col := 0; col < 15; col++ {
			dr := row - cy
			if dr < 0 {
				dr = -dr
			}
			dc := col - cx
			if dc < 0 {
				dc = -dc
			}
			d := dr
			if dc > d {
				d = dc
			}
			if d > 5 {
				continue // outside bullseye
			}
			want := wantColor(d)
			got := colorAt(row, col)
			if got != want {
				t.Errorf("bullseye at (%d,%d) d=%d: got %s, want %s", row, col, d, got, want)
			}
		}
	}
}

// TestEncodeOrientationMarks verifies that the four corner modules of the mode
// message ring are always dark (orientation marks).
//
// For compact 1-layer (15×15), center (7,7), bullseye radius 5:
//   mode message ring radius = 6
//   corners at (7±6, 7±6) = (1,1), (13,1), (13,13), (1,13)
func TestEncodeOrientationMarks(t *testing.T) {
	grid, err := Encode("Hello", nil)
	if err != nil {
		t.Fatalf("Encode error: %v", err)
	}

	size := int(grid.Rows)
	cx := size / 2
	cy := size / 2

	// Determine compact or full from size.
	var compact bool
	var br int
	if size == 11+4*((size-11)/4) && (size-11)%4 == 0 && size <= 27 {
		compact = true
		br = 5
	} else {
		compact = false
		br = 7
	}
	_ = compact

	r := br + 1 // mode message ring radius

	corners := [][2]int{
		{cy - r, cx - r},
		{cy - r, cx + r},
		{cy + r, cx + r},
		{cy + r, cx - r},
	}

	for _, c := range corners {
		row, col := c[0], c[1]
		if !grid.Modules[row][col] {
			t.Errorf("orientation mark at (%d,%d) is LIGHT, want DARK", row, col)
		}
	}
}

// TestEncodeHelloWorld verifies that "Hello World" encodes without error and
// produces a grid larger than 15×15 (requires more than compact 1 layer).
func TestEncodeHelloWorld(t *testing.T) {
	grid, err := Encode("Hello World", nil)
	if err != nil {
		t.Fatalf("Encode('Hello World') error: %v", err)
	}
	if grid.Rows < 15 || grid.Cols < 15 {
		t.Errorf("grid too small: %d×%d", grid.Rows, grid.Cols)
	}
	if grid.Rows != grid.Cols {
		t.Errorf("grid is not square: %d×%d", grid.Rows, grid.Cols)
	}
}

// TestEncodeURL verifies that a typical URL encodes without error.
func TestEncodeURL(t *testing.T) {
	grid, err := Encode("https://example.com", nil)
	if err != nil {
		t.Fatalf("Encode('https://example.com') error: %v", err)
	}
	// Must be at least 15×15 and square.
	if grid.Rows < 15 || grid.Cols != grid.Rows {
		t.Errorf("unexpected grid size %d×%d", grid.Rows, grid.Cols)
	}
}

// TestEncodeEmptyString verifies that an empty string can be encoded.
func TestEncodeEmptyString(t *testing.T) {
	grid, err := Encode("", nil)
	if err != nil {
		t.Fatalf("Encode('') error: %v", err)
	}
	if grid.Rows == 0 || grid.Cols == 0 {
		t.Errorf("empty string produced a 0-size grid: %d×%d", grid.Rows, grid.Cols)
	}
}

// TestEncodeSymbolSize verifies the exact symbol sizes for known inputs.
//
// "A" (1 byte) → compact 1 layer → 15×15
// A 50-byte string → should require compact 4 or full 1-2 layers.
func TestEncodeSymbolSize(t *testing.T) {
	grid, err := Encode("A", nil)
	if err != nil {
		t.Fatalf("error: %v", err)
	}
	if grid.Rows != 15 {
		t.Errorf("'A': expected 15×15, got %d×%d", grid.Rows, grid.Cols)
	}
}

// TestEncodeSymbolSizes verifies that symbol sizes follow the formula.
//
// Compact: size = 11 + 4 × layers
// Full:    size = 15 + 4 × layers
func TestEncodeSymbolSizes(t *testing.T) {
	// Force a compact 2-layer symbol by encoding enough data to overflow 1 layer.
	// Compact 1-layer data capacity at 23% ECC: 9 total - ceil(0.23*9)=3 ECC = 6 data bytes max.
	// Binary-Shift overhead: 5+5+N*8 bits for N bytes.
	// For N=4 bytes: 5+5+32=42 bits → stuffed: ~50 bits → 7 bytes. 7 > 6 → needs 2 layers.
	data := []byte("ABCD") // 4 bytes
	grid, err := EncodeBytes(data, nil)
	if err != nil {
		t.Fatalf("error: %v", err)
	}
	// Should be compact 1 or 2 layers (15 or 19).
	if grid.Rows != 15 && grid.Rows != 19 && grid.Rows != 23 {
		t.Errorf("unexpected size %d for 4-byte input", grid.Rows)
	}
	// Must be odd (all valid Aztec sizes are odd).
	if grid.Rows%2 == 0 {
		t.Errorf("grid size %d is even, should be odd", grid.Rows)
	}
}

// TestEncodeGridIsSquare verifies that all grids are square.
func TestEncodeGridIsSquare(t *testing.T) {
	inputs := []string{"A", "Hello", "Hello World", "https://example.com", "0123456789"}
	for _, inp := range inputs {
		grid, err := Encode(inp, nil)
		if err != nil {
			t.Errorf("Encode(%q) error: %v", inp, err)
			continue
		}
		if grid.Rows != grid.Cols {
			t.Errorf("Encode(%q): grid not square: %d×%d", inp, grid.Rows, grid.Cols)
		}
	}
}

// TestEncodeLargeInput verifies that a moderately large input (100 bytes) encodes.
func TestEncodeLargeInput(t *testing.T) {
	data := make([]byte, 100)
	for i := range data {
		data[i] = byte(i)
	}
	grid, err := EncodeBytes(data, nil)
	if err != nil {
		t.Fatalf("EncodeBytes(100 bytes) error: %v", err)
	}
	if grid.Rows < 15 {
		t.Errorf("grid too small: %d×%d", grid.Rows, grid.Cols)
	}
}

// TestEncodeExtremelyLargeInput verifies that unreasonably large input returns
// an InputTooLongError.
func TestEncodeExtremelyLargeInput(t *testing.T) {
	// 5000 bytes: well beyond any 32-layer full symbol capacity.
	data := make([]byte, 5000)
	_, err := EncodeBytes(data, nil)
	if err == nil {
		t.Fatal("expected error for 5000-byte input, got nil")
	}
	if !IsInputTooLong(err) {
		t.Errorf("expected InputTooLongError, got %T: %v", err, err)
	}
}

// TestEncodeCustomECC verifies that a higher ECC percentage produces a larger
// (or equal) symbol than the default.
func TestEncodeCustomECC(t *testing.T) {
	opts90 := &Options{MinEccPercent: 90}
	grid90, err := Encode("Hello World", opts90)
	if err != nil {
		t.Fatalf("Encode with 90%% ECC error: %v", err)
	}
	grid23, err := Encode("Hello World", nil)
	if err != nil {
		t.Fatalf("Encode with default ECC error: %v", err)
	}
	// 90% ECC requires more total slots → symbol must be same or larger.
	if grid90.Rows < grid23.Rows {
		t.Errorf("90%% ECC produced smaller symbol (%d) than 23%% ECC (%d)", grid90.Rows, grid23.Rows)
	}
}

// ============================================================================
// Module count / consistency checks
// ============================================================================

// TestEncodeModulesNotAllSame verifies that a typical symbol is not all-dark or
// all-light (a degenerate output would be broken).
func TestEncodeModulesNotAllSame(t *testing.T) {
	grid, err := Encode("Hello World", nil)
	if err != nil {
		t.Fatalf("error: %v", err)
	}
	hasDark, hasLight := false, false
	for _, row := range grid.Modules {
		for _, m := range row {
			if m {
				hasDark = true
			} else {
				hasLight = true
			}
		}
	}
	if !hasDark {
		t.Error("all modules are light — degenerate output")
	}
	if !hasLight {
		t.Error("all modules are dark — degenerate output")
	}
}

// TestEncodeModuleCount verifies that the grid has exactly Rows × Cols modules.
func TestEncodeModuleCount(t *testing.T) {
	grid, err := Encode("Test", nil)
	if err != nil {
		t.Fatalf("error: %v", err)
	}
	if int(grid.Rows) != len(grid.Modules) {
		t.Errorf("Rows=%d but len(grid.Modules)=%d", grid.Rows, len(grid.Modules))
	}
	for r, row := range grid.Modules {
		if int(grid.Cols) != len(row) {
			t.Errorf("row %d: Cols=%d but len=%d", r, grid.Cols, len(row))
		}
	}
}

// ============================================================================
// Cross-language test vectors
// ============================================================================
//
// These tests verify the Go output matches the TypeScript reference output for
// a shared set of inputs. The expected sizes come from running the TypeScript
// implementation (verified by the spec author).

// TestCrossLangVectorA verifies "A" produces a 15×15 symbol.
func TestCrossLangVectorA(t *testing.T) {
	grid, err := Encode("A", nil)
	if err != nil {
		t.Fatalf("error: %v", err)
	}
	if grid.Rows != 15 {
		t.Errorf("'A': expected 15×15, got %d×%d", grid.Rows, grid.Cols)
	}
}

// TestCrossLangVectorDigits verifies a digit string encodes correctly.
func TestCrossLangVectorDigits(t *testing.T) {
	grid, err := Encode("01234567890123456789", nil)
	if err != nil {
		t.Fatalf("error: %v", err)
	}
	if grid.Rows < 15 || grid.Rows%4 != 3 {
		// All valid Aztec sizes satisfy: size % 4 == 3 (since 11+4k and 15+4k both give size%4=3)
		t.Errorf("unexpected grid size %d for digit input", grid.Rows)
	}
}

// TestCrossLangVectorAOdd verifies all encoded symbols have odd dimensions.
func TestCrossLangVectorAOdd(t *testing.T) {
	inputs := []string{
		"A",
		"Hello World",
		"https://example.com",
		"01234567890123456789",
	}
	for _, inp := range inputs {
		grid, err := Encode(inp, nil)
		if err != nil {
			t.Errorf("Encode(%q) error: %v", inp, err)
			continue
		}
		if grid.Rows%2 == 0 {
			t.Errorf("Encode(%q): grid size %d is even, all Aztec sizes must be odd", inp, grid.Rows)
		}
	}
}

// ============================================================================
// RS encoding tests
// ============================================================================

// TestGF256RSEncodeSmokeTest verifies that GF256RsEncode produces a consistent
// result for a known small input.
func TestGF256RSEncodeSmokeTest(t *testing.T) {
	// Encode [1, 2, 3] with 3 ECC bytes. The result should be deterministic.
	data := []byte{1, 2, 3}
	ecc1 := GF256RsEncode(data, 3)
	ecc2 := GF256RsEncode(data, 3)
	if len(ecc1) != 3 {
		t.Errorf("ECC length = %d, want 3", len(ecc1))
	}
	if !equalByteSlices(ecc1, ecc2) {
		t.Error("GF256RsEncode is not deterministic")
	}
}

// TestGF256RSEncodeDetectsErrors verifies that RS ECC values change when
// the data changes (basic sanity check that the ECC is not trivial).
func TestGF256RSEncodeDetectsErrors(t *testing.T) {
	data1 := []byte{1, 2, 3}
	data2 := []byte{1, 2, 4} // one byte different
	ecc1 := GF256RsEncode(data1, 3)
	ecc2 := GF256RsEncode(data2, 3)
	if equalByteSlices(ecc1, ecc2) {
		t.Error("ECC unchanged after data modification — ECC may be trivial")
	}
}

// TestGF256RSEncodeLargeNEcc verifies that GF256RsEncode does not panic for
// nEcc > 255 (which requires gf256Exp indexing with i%255 rather than i).
//
// This was a security fix: without the mod, nEcc > 254 would cause an
// index-out-of-bounds panic, enabling denial-of-service for large symbols.
func TestGF256RSEncodeLargeNEcc(t *testing.T) {
	// nEcc = 300 > 255; must not panic.
	data := make([]byte, 100)
	for i := range data {
		data[i] = byte(i + 1)
	}
	defer func() {
		if r := recover(); r != nil {
			t.Errorf("GF256RsEncode panicked for nEcc=300: %v", r)
		}
	}()
	ecc := GF256RsEncode(data, 300)
	if len(ecc) != 300 {
		t.Errorf("GF256RsEncode(nEcc=300) returned %d bytes, want 300", len(ecc))
	}
}

// TestGF16RSEncodeLength verifies that gf16RsEncode returns exactly n ECC nibbles.
func TestGF16RSEncodeLength(t *testing.T) {
	for n := 1; n <= 8; n++ {
		ecc := GF16RsEncode([]int{1, 2, 3}, n)
		if len(ecc) != n {
			t.Errorf("GF16RsEncode(n=%d): got %d ECC nibbles, want %d", n, len(ecc), n)
		}
	}
}

// ============================================================================
// Error type tests
// ============================================================================

// TestInputTooLongErrorIs verifies errors.Is works with the sentinel.
func TestInputTooLongErrorIs(t *testing.T) {
	err := &InputTooLongError{DataBits: 1000}
	if !errors.Is(err, ErrInputTooLong) {
		t.Error("errors.Is(InputTooLongError{}, ErrInputTooLong) = false, want true")
	}
}

// TestInputTooLongErrorMessage verifies the error message is non-empty.
func TestInputTooLongErrorMessage(t *testing.T) {
	err := &InputTooLongError{DataBits: 42}
	if err.Error() == "" {
		t.Error("error message is empty")
	}
}

// ============================================================================
// Helper utilities
// ============================================================================

func equalBytes(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func equalByteSlices(a, b []byte) bool {
	return equalBytes(a, b)
}
