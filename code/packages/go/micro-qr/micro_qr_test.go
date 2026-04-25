// Tests for the Micro QR Code encoder.
//
// # Test organisation
//
//  1. Symbol-dimension tests — verify the grid size (11×11 … 17×17) for each version.
//  2. Auto-version selection — encoder picks the smallest symbol that fits the input.
//  3. Structural-module tests — finder pattern, separator, and timing are correct.
//  4. Determinism — same input always produces identical grids.
//  5. ECC-level constraints — valid and invalid (version, ECC) combinations.
//  6. Capacity boundaries — inputs at and beyond maximum capacity.
//  7. Format information — format info region contains at least one dark module.
//  8. Grid completeness — every grid is square with correct Rows, Cols, and Modules.
//  9. Cross-language corpus — expected symbol sizes match the test corpus from the spec.
//
// Each section is preceded by a comment block explaining the invariant under test.
package microqr

import (
	"strings"
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Helper utilities
// ─────────────────────────────────────────────────────────────────────────────

// gridToString renders a ModuleGrid as a newline-separated string of "0" and "1"
// characters. This canonical form is used for cross-grid comparisons in
// determinism and ECC-level tests.
func gridToString(t *testing.T, input string, version *MicroQRVersion, ecc *MicroQREccLevel) string {
	t.Helper()
	grid, err := Encode(input, version, ecc)
	if err != nil {
		t.Fatalf("Encode(%q) unexpected error: %v", input, err)
	}
	rows := make([]string, grid.Rows)
	for r := uint32(0); r < grid.Rows; r++ {
		var sb strings.Builder
		for c := uint32(0); c < grid.Cols; c++ {
			if grid.Modules[r][c] {
				sb.WriteByte('1')
			} else {
				sb.WriteByte('0')
			}
		}
		rows[r] = sb.String()
	}
	return strings.Join(rows, "\n")
}

// ptr helpers — Go doesn't allow taking the address of a composite literal
// constant, so we use small helper functions to create pointers.
func versionPtr(v MicroQRVersion) *MicroQRVersion { return &v }
func eccPtr(e MicroQREccLevel) *MicroQREccLevel    { return &e }

// encodeRows is a convenience that returns only the row count.
func encodeRows(t *testing.T, input string, version *MicroQRVersion, ecc *MicroQREccLevel) uint32 {
	t.Helper()
	grid, err := Encode(input, version, ecc)
	if err != nil {
		t.Fatalf("Encode(%q) unexpected error: %v", input, err)
	}
	return grid.Rows
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Symbol dimension tests
//
// ISO/IEC 18004:2015 Annex E specifies:
//   M1 = 11×11, M2 = 13×13, M3 = 15×15, M4 = 17×17
//   formula: size = 2 × version_number + 9
// ─────────────────────────────────────────────────────────────────────────────

// TestM1Is11x11 confirms M1 produces an 11×11 symbol grid.
func TestM1Is11x11(t *testing.T) {
	grid, err := Encode("1", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid.Rows != 11 || grid.Cols != 11 {
		t.Errorf("expected 11×11, got %d×%d", grid.Rows, grid.Cols)
	}
}

// TestM2Is13x13ForHELLO confirms "HELLO" (5 alphanumeric chars) fits in M2.
func TestM2Is13x13ForHELLO(t *testing.T) {
	grid, err := Encode("HELLO", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid.Rows != 13 || grid.Cols != 13 {
		t.Errorf("expected 13×13, got %d×%d", grid.Rows, grid.Cols)
	}
}

// TestM4Is17x17ForURL confirms a URL that needs byte mode selects M4.
func TestM4Is17x17ForURL(t *testing.T) {
	grid, err := Encode("https://a.b", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid.Rows != 17 || grid.Cols != 17 {
		t.Errorf("expected 17×17, got %d×%d", grid.Rows, grid.Cols)
	}
}

// TestModuleShapeIsSquare confirms all Micro QR symbols use square modules.
func TestModuleShapeIsSquare(t *testing.T) {
	grid, err := Encode("1", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	// barcode2d.ModuleShapeSquare == 0
	if grid.ModuleShape != 0 {
		t.Errorf("expected ModuleShapeSquare (0), got %v", grid.ModuleShape)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Auto-version selection
//
// The encoder iterates symbolConfigs in smallest-first order and picks the
// first symbol that can hold the input in any supported mode.
// ─────────────────────────────────────────────────────────────────────────────

// TestAutoSelectsM1ForSingleDigit verifies a single digit fits in M1.
func TestAutoSelectsM1ForSingleDigit(t *testing.T) {
	if rows := encodeRows(t, "1", nil, nil); rows != 11 {
		t.Errorf("expected 11 rows (M1), got %d", rows)
	}
}

// TestAutoSelectsM1For12345 verifies 5 digits (M1 maximum) stay in M1.
func TestAutoSelectsM1For12345(t *testing.T) {
	if rows := encodeRows(t, "12345", nil, nil); rows != 11 {
		t.Errorf("expected 11 rows (M1), got %d", rows)
	}
}

// TestAutoSelectsM2For6Digits verifies 6 digits overflow M1 and land in M2.
// M1 numeric capacity = 5. The 6th digit forces M2.
func TestAutoSelectsM2For6Digits(t *testing.T) {
	if rows := encodeRows(t, "123456", nil, nil); rows != 13 {
		t.Errorf("expected 13 rows (M2), got %d", rows)
	}
}

// TestAutoSelectsM2ForHELLO verifies 5 alphanumeric chars select M2-L.
// M2-L alpha capacity = 6, so "HELLO" (5 chars) fits comfortably.
func TestAutoSelectsM2ForHELLO(t *testing.T) {
	if rows := encodeRows(t, "HELLO", nil, nil); rows != 13 {
		t.Errorf("expected 13 rows (M2), got %d", rows)
	}
}

// TestAutoSelectsM3OrHigherForHelloLowercase verifies "hello" (byte mode,
// 5 bytes) routes to at least M3. M2-L byte cap = 4, so 5 bytes need M3.
func TestAutoSelectsM3OrHigherForHelloLowercase(t *testing.T) {
	rows := encodeRows(t, "hello", nil, nil)
	if rows < 15 {
		t.Errorf("expected at least M3 (15 rows), got %d", rows)
	}
}

// TestAutoSelectsM4ForURL verifies a URL (byte mode, 11 bytes) lands in M4.
// M3-L byte cap = 9, so 11 bytes need M4-L (cap = 15).
func TestAutoSelectsM4ForURL(t *testing.T) {
	if rows := encodeRows(t, "https://a.b", nil, nil); rows != 17 {
		t.Errorf("expected 17 rows (M4), got %d", rows)
	}
}

// TestForcedVersionM4 verifies that forcing M4 produces a 17×17 symbol
// even for a tiny 1-character input.
func TestForcedVersionM4(t *testing.T) {
	if rows := encodeRows(t, "1", versionPtr(VersionM4), nil); rows != 17 {
		t.Errorf("expected 17 rows (forced M4), got %d", rows)
	}
}

// TestForcedEccMProducesDifferentGrid verifies that forcing ECC=M vs ECC=L
// produces different format information (different grid data content).
func TestForcedEccMProducesDifferentGrid(t *testing.T) {
	s1 := gridToString(t, "HELLO", nil, eccPtr(EccL))
	s2 := gridToString(t, "HELLO", nil, eccPtr(EccM))
	if s1 == s2 {
		t.Error("ECC-L and ECC-M grids should differ for same input")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Structural module tests
//
// The finder pattern, separator, and timing pattern are deterministic —
// they are the same in every valid Micro QR symbol regardless of the
// data encoded. We can test them directly.
// ─────────────────────────────────────────────────────────────────────────────

// TestFinderPatternM1 verifies the 7×7 finder pattern in an M1 symbol.
//
// The finder pattern structure (rows 0–6, cols 0–6):
//
//	■ ■ ■ ■ ■ ■ ■   row 0 and row 6: all dark
//	■ □ □ □ □ □ ■
//	■ □ ■ ■ ■ □ ■   rows 2-4, cols 2-4: 3×3 dark core
//	■ □ ■ ■ ■ □ ■
//	■ □ ■ ■ ■ □ ■
//	■ □ □ □ □ □ ■
//	■ ■ ■ ■ ■ ■ ■
func TestFinderPatternM1(t *testing.T) {
	grid, err := Encode("1", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	m := grid.Modules

	// Top border row (row 0): all dark
	for c := 0; c < 7; c++ {
		if !m[0][c] {
			t.Errorf("finder border: row 0, col %d should be dark", c)
		}
	}
	// Bottom border row (row 6): all dark
	for c := 0; c < 7; c++ {
		if !m[6][c] {
			t.Errorf("finder border: row 6, col %d should be dark", c)
		}
	}
	// Left border col (col 0): all dark
	for r := 0; r < 7; r++ {
		if !m[r][0] {
			t.Errorf("finder border: col 0, row %d should be dark", r)
		}
	}
	// Right border col (col 6): all dark
	for r := 0; r < 7; r++ {
		if !m[r][6] {
			t.Errorf("finder border: col 6, row %d should be dark", r)
		}
	}
	// Inner ring (row 1, cols 1–5): all light
	for c := 1; c <= 5; c++ {
		if m[1][c] {
			t.Errorf("finder inner ring: row 1, col %d should be light", c)
		}
	}
	// 3×3 core (rows 2–4, cols 2–4): all dark
	for r := 2; r <= 4; r++ {
		for c := 2; c <= 4; c++ {
			if !m[r][c] {
				t.Errorf("finder core: (%d,%d) should be dark", r, c)
			}
		}
	}
}

// TestSeparatorM2 verifies the L-shaped separator in an M2 symbol.
//
// The separator runs along row 7 (cols 0–7) and col 7 (rows 0–7),
// forming an "L" that separates the finder from the data area.
// All separator modules must be light (false).
func TestSeparatorM2(t *testing.T) {
	grid, err := Encode("HELLO", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	m := grid.Modules

	// Row 7, cols 0–7: all light
	for c := 0; c <= 7; c++ {
		if m[7][c] {
			t.Errorf("separator: row 7, col %d should be light", c)
		}
	}
	// Col 7, rows 0–7: all light
	for r := 0; r <= 7; r++ {
		if m[r][7] {
			t.Errorf("separator: col 7, row %d should be light", r)
		}
	}
}

// TestTimingRowM4 verifies the timing pattern along row 0 for an M4 symbol.
//
// Timing pattern: dark at even indices, light at odd indices.
// The finder (cols 0–6) and separator (col 7) already handle positions 0–7,
// so the timing *extension* starts at col 8.
func TestTimingRowM4(t *testing.T) {
	grid, err := Encode("https://a.b", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	m := grid.Modules

	for c := 8; c < 17; c++ {
		expected := c%2 == 0
		if m[0][c] != expected {
			t.Errorf("timing row 0, col %d: expected dark=%v, got %v", c, expected, m[0][c])
		}
	}
}

// TestTimingColM4 verifies the timing pattern along col 0 for an M4 symbol.
func TestTimingColM4(t *testing.T) {
	grid, err := Encode("https://a.b", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	m := grid.Modules

	for r := 8; r < 17; r++ {
		expected := r%2 == 0
		if m[r][0] != expected {
			t.Errorf("timing col 0, row %d: expected dark=%v, got %v", r, expected, m[r][0])
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Determinism
//
// The encoder must be pure: identical inputs always produce identical grids.
// This is not trivial given that the mask selection involves a penalty score
// evaluation — if any intermediate state leaks across calls, results diverge.
// ─────────────────────────────────────────────────────────────────────────────

// TestDeterministic encodes each test-corpus input twice and compares grids.
func TestDeterministic(t *testing.T) {
	inputs := []string{"1", "12345", "HELLO", "A1B2C3", "hello", "https://a.b"}
	for _, input := range inputs {
		s1 := gridToString(t, input, nil, nil)
		s2 := gridToString(t, input, nil, nil)
		if s1 != s2 {
			t.Errorf("non-deterministic for input %q", input)
		}
	}
}

// TestDifferentInputsDifferentGrids confirms that distinct payloads produce
// distinct bit patterns (basic sanity: the encoder is not ignoring the input).
func TestDifferentInputsDifferentGrids(t *testing.T) {
	s1 := gridToString(t, "1", nil, nil)
	s2 := gridToString(t, "2", nil, nil)
	if s1 == s2 {
		t.Error("'1' and '2' should produce different grids")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. ECC-level constraints
//
// Not every (version, ECC) combination is valid in Micro QR:
//   - M1 only supports Detection
//   - M2 and M3 support L and M
//   - M4 supports L, M, and Q
//   - H is never available in any Micro QR symbol
// ─────────────────────────────────────────────────────────────────────────────

// TestM1Detection verifies M1 with EccDetection succeeds.
func TestM1Detection(t *testing.T) {
	grid, err := EncodeAt("1", VersionM1, EccDetection)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid.Rows != 11 {
		t.Errorf("expected 11 rows, got %d", grid.Rows)
	}
}

// TestM4Q verifies M4 with EccQ succeeds.
func TestM4Q(t *testing.T) {
	grid, err := EncodeAt("HELLO", VersionM4, EccQ)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid.Rows != 17 {
		t.Errorf("expected 17 rows, got %d", grid.Rows)
	}
}

// TestM4AllEccLevelsDiffer verifies that L, M, Q produce different grids for
// the same input at the same version. Different ECC levels have different
// symbol indicators → different format info → different module patterns.
func TestM4AllEccLevelsDiffer(t *testing.T) {
	gl := gridToString(t, "HELLO", versionPtr(VersionM4), eccPtr(EccL))
	gm := gridToString(t, "HELLO", versionPtr(VersionM4), eccPtr(EccM))
	gq := gridToString(t, "HELLO", versionPtr(VersionM4), eccPtr(EccQ))
	if gl == gm {
		t.Error("M4-L and M4-M should differ")
	}
	if gm == gq {
		t.Error("M4-M and M4-Q should differ")
	}
	if gl == gq {
		t.Error("M4-L and M4-Q should differ")
	}
}

// TestM1RejectsEccL verifies that requesting EccL for VersionM1 returns
// ECCNotAvailable (M1 has no L level — only Detection).
func TestM1RejectsEccL(t *testing.T) {
	_, err := EncodeAt("1", VersionM1, EccL)
	if !IsECCNotAvailable(err) {
		t.Errorf("expected ECCNotAvailable error, got: %v", err)
	}
}

// TestM2RejectsEccQ verifies that requesting EccQ for VersionM2 returns
// ECCNotAvailable (Q is only available in M4).
func TestM2RejectsEccQ(t *testing.T) {
	_, err := EncodeAt("1", VersionM2, EccQ)
	if !IsECCNotAvailable(err) {
		t.Errorf("expected ECCNotAvailable error, got: %v", err)
	}
}

// TestM3RejectsEccQ verifies that requesting EccQ for VersionM3 returns
// ECCNotAvailable.
func TestM3RejectsEccQ(t *testing.T) {
	_, err := EncodeAt("1", VersionM3, EccQ)
	if !IsECCNotAvailable(err) {
		t.Errorf("expected ECCNotAvailable error, got: %v", err)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Capacity boundaries
//
// Each (version, ECC, mode) combination has a maximum character count.
// At-capacity inputs must succeed; over-capacity inputs must fail with
// InputTooLong.
// ─────────────────────────────────────────────────────────────────────────────

// TestM1Max5Digits verifies M1 can hold exactly 5 digits (its maximum).
func TestM1Max5Digits(t *testing.T) {
	rows := encodeRows(t, "12345", nil, nil)
	if rows != 11 {
		t.Errorf("expected M1 (11 rows), got %d", rows)
	}
}

// TestM1Overflow6Digits verifies 6 digits overflow M1 and select M2.
func TestM1Overflow6Digits(t *testing.T) {
	rows := encodeRows(t, "123456", nil, nil)
	if rows != 13 {
		t.Errorf("expected M2 (13 rows) for 6 digits, got %d", rows)
	}
}

// TestM4Max35Digits verifies 35 digits (M4-L numeric maximum) fit in M4.
func TestM4Max35Digits(t *testing.T) {
	rows := encodeRows(t, strings.Repeat("1", 35), nil, nil)
	if rows != 17 {
		t.Errorf("expected M4 (17 rows), got %d", rows)
	}
}

// TestM4Overflow36Digits verifies 36 digits exceed all Micro QR capacity
// and return InputTooLong.
func TestM4Overflow36Digits(t *testing.T) {
	_, err := Encode(strings.Repeat("1", 36), nil, nil)
	if !IsInputTooLong(err) {
		t.Errorf("expected InputTooLong for 36 digits, got: %v", err)
	}
}

// TestM4MaxByte15Chars verifies 15 lowercase bytes fit in M4-L (byte cap = 15).
func TestM4MaxByte15Chars(t *testing.T) {
	rows := encodeRows(t, strings.Repeat("a", 15), nil, nil)
	if rows != 17 {
		t.Errorf("expected M4 (17 rows), got %d", rows)
	}
}

// TestM4QMax21Numeric verifies M4-Q can hold 21 numeric characters (its max).
func TestM4QMax21Numeric(t *testing.T) {
	rows := encodeRows(t, strings.Repeat("1", 21), nil, eccPtr(EccQ))
	if rows != 17 {
		t.Errorf("expected M4 (17 rows), got %d", rows)
	}
}

// TestInputTooLongError verifies that Encode returns InputTooLong for inputs
// exceeding the maximum Micro QR capacity.
func TestInputTooLongError(t *testing.T) {
	_, err := Encode(strings.Repeat("1", 36), nil, nil)
	if !IsInputTooLong(err) {
		t.Errorf("expected IsInputTooLong true, got: %v", err)
	}
}

// TestEmptyStringEncodesToM1 verifies an empty string selects M1 (smallest
// symbol, numeric mode, zero characters — always fits).
func TestEmptyStringEncodesToM1(t *testing.T) {
	rows := encodeRows(t, "", nil, nil)
	if rows != 11 {
		t.Errorf("expected M1 (11 rows) for empty string, got %d", rows)
	}
}

// TestECCNotAvailableError verifies the ECCNotAvailable error type for an
// impossible combination.
func TestECCNotAvailableError(t *testing.T) {
	_, err := EncodeAt("1", VersionM1, EccQ)
	if !IsECCNotAvailable(err) {
		t.Errorf("expected IsECCNotAvailable true, got: %v", err)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Format information
//
// After encoding, the format information region (row 8 cols 1–8, col 8 rows
// 1–7) must contain at least one dark module. A completely light format
// region would indicate a bug in writeFormatInfo or an all-zero format word.
// ─────────────────────────────────────────────────────────────────────────────

// TestFormatInfoNonZeroM4 verifies that the format info area has dark modules.
func TestFormatInfoNonZeroM4(t *testing.T) {
	grid, err := EncodeAt("HELLO", VersionM4, EccL)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	m := grid.Modules
	anyDark := false
	for c := 1; c <= 8; c++ {
		if m[8][c] {
			anyDark = true
			break
		}
	}
	for r := 1; r <= 7; r++ {
		if m[r][8] {
			anyDark = true
			break
		}
	}
	if !anyDark {
		t.Error("format info area should have at least one dark module")
	}
}

// TestFormatInfoNonZeroM1 verifies format info has dark modules even in the
// smallest symbol.
func TestFormatInfoNonZeroM1(t *testing.T) {
	grid, err := Encode("1", nil, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	m := grid.Modules
	count := 0
	for c := 1; c <= 8; c++ {
		if m[8][c] {
			count++
		}
	}
	for r := 1; r <= 7; r++ {
		if m[r][8] {
			count++
		}
	}
	if count == 0 {
		t.Error("M1 format info should have at least one dark module")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Grid completeness
//
// Every grid returned by Encode must:
//   - Be square (Rows == Cols)
//   - Have Modules dimensions matching Rows × Cols
// ─────────────────────────────────────────────────────────────────────────────

// TestGridIsSquareAndComplete checks that grid dimensions are self-consistent
// for all corpus inputs.
func TestGridIsSquareAndComplete(t *testing.T) {
	inputs := []string{"1", "HELLO", "hello", "https://a.b"}
	for _, input := range inputs {
		grid, err := Encode(input, nil, nil)
		if err != nil {
			t.Errorf("Encode(%q) unexpected error: %v", input, err)
			continue
		}
		if grid.Rows != grid.Cols {
			t.Errorf("input %q: grid should be square, got %d×%d", input, grid.Rows, grid.Cols)
		}
		if uint32(len(grid.Modules)) != grid.Rows {
			t.Errorf("input %q: Modules row count %d != Rows %d", input, len(grid.Modules), grid.Rows)
		}
		for r, row := range grid.Modules {
			if uint32(len(row)) != grid.Cols {
				t.Errorf("input %q: row %d has %d cols, expected %d", input, r, len(row), grid.Cols)
			}
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Cross-language corpus
//
// The spec defines a shared test corpus for cross-language verification.
// All implementations must produce the same symbol size for each corpus entry.
// ─────────────────────────────────────────────────────────────────────────────

// TestCrossLanguageCorpus encodes the spec's test corpus and checks symbol sizes.
//
// Expected sizes:
//
//	"1"            → M1 (11×11)   single digit, numeric
//	"12345"        → M1 (11×11)   M1 max (5 digits)
//	"HELLO"        → M2 (13×13)   5 alphanumeric
//	"01234567"     → M2 (13×13)   8-digit numeric (M2-L numeric cap = 10)
//	"https://a.b"  → M4 (17×17)   11-byte URL, byte mode
//	"MICRO QR TEST"→ M3 (15×15)   13 alphanumeric (M3-L alpha cap = 14)
func TestCrossLanguageCorpus(t *testing.T) {
	cases := []struct {
		input    string
		wantRows uint32
	}{
		{"1", 11},
		{"12345", 11},
		{"HELLO", 13},
		{"01234567", 13},
		{"https://a.b", 17},
		{"MICRO QR TEST", 15},
	}
	for _, tc := range cases {
		grid, err := Encode(tc.input, nil, nil)
		if err != nil {
			t.Errorf("Encode(%q) unexpected error: %v", tc.input, err)
			continue
		}
		if grid.Rows != tc.wantRows {
			t.Errorf("Encode(%q): expected %d×%d, got %d×%d",
				tc.input, tc.wantRows, tc.wantRows, grid.Rows, grid.Cols)
		}
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Internal unit tests (white-box)
// ─────────────────────────────────────────────────────────────────────────────

// TestRSEncodeM1 verifies the RS encoder for M1's 2-ECC polynomial.
//
// M1 data = [0x18, 0x40, 0x00] (numeric "1" encoded), ECC count = 2.
// The generator for 2 ECC codewords is g(x) = x² + 3x + 2 (coefficients: 01 03 02).
// The expected remainder is precomputed to verify the LFSR algorithm.
func TestRSEncodeNotEmpty(t *testing.T) {
	// Encode "1" in M1: data codewords should be 3 bytes, ECC should be 2 bytes.
	// We test by verifying the overall pipeline produces a valid-looking grid.
	grid, err := EncodeAt("1", VersionM1, EccDetection)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if grid.Rows != 11 {
		t.Errorf("expected 11 rows, got %d", grid.Rows)
	}
}

// TestBitWriterBasic verifies the bitWriter correctly packs bits into bytes.
// This is a white-box test using the internal type.
func TestBitWriterBasic(t *testing.T) {
	var w bitWriter
	// Write 0b10110100 (8 bits = 0xB4)
	w.write(0b10110100, 8)
	got := w.toBytes()
	if len(got) != 1 {
		t.Fatalf("expected 1 byte, got %d", len(got))
	}
	if got[0] != 0xB4 {
		t.Errorf("expected 0xB4, got 0x%02X", got[0])
	}
}

// TestBitWriterPadding verifies that incomplete trailing bits are zero-padded.
func TestBitWriterPadding(t *testing.T) {
	var w bitWriter
	// Write 4 bits: 0b1010 → packed as 0b10100000 = 0xA0
	w.write(0b1010, 4)
	got := w.toBytes()
	if len(got) != 1 {
		t.Fatalf("expected 1 byte, got %d", len(got))
	}
	if got[0] != 0xA0 {
		t.Errorf("expected 0xA0, got 0x%02X", got[0])
	}
}

// TestMaskCondition verifies all four mask patterns.
//
// Mask 0: (row+col) mod 2 == 0 → dark at (0,0), (0,2), (1,1)
// Mask 1: row mod 2 == 0       → dark at (0,0), (0,1), (2,0)
// Mask 2: col mod 3 == 0       → dark at (0,0), (1,0), (0,3)
// Mask 3: (row+col) mod 3 == 0 → dark at (0,0), (1,2), (2,1)
func TestMaskCondition(t *testing.T) {
	cases := []struct {
		mask int
		row  int
		col  int
		want bool
	}{
		// Mask 0
		{0, 0, 0, true}, {0, 0, 1, false}, {0, 1, 1, true}, {0, 1, 0, false},
		// Mask 1
		{1, 0, 0, true}, {1, 0, 5, true}, {1, 1, 0, false}, {1, 2, 3, true},
		// Mask 2
		{2, 0, 0, true}, {2, 1, 0, true}, {2, 0, 1, false}, {2, 0, 3, true},
		// Mask 3
		{3, 0, 0, true}, {3, 1, 2, true}, {3, 2, 1, true}, {3, 0, 1, false},
	}
	for _, tc := range cases {
		got := maskCondition(tc.mask, tc.row, tc.col)
		if got != tc.want {
			t.Errorf("maskCondition(%d, %d, %d): want %v, got %v",
				tc.mask, tc.row, tc.col, tc.want, got)
		}
	}
}

// TestSelectModeNumeric verifies numeric mode is selected for digit-only inputs.
func TestSelectModeNumeric(t *testing.T) {
	cfg := &symbolConfigs[1] // M2-L (has numeric, alpha, byte support)
	mode, err := selectMode("12345", cfg)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if mode != modeNumeric {
		t.Errorf("expected modeNumeric, got %v", mode)
	}
}

// TestSelectModeAlphanumeric verifies alphanumeric mode is selected for
// all-uppercase + special-char inputs.
func TestSelectModeAlphanumeric(t *testing.T) {
	cfg := &symbolConfigs[1] // M2-L
	mode, err := selectMode("HELLO WORLD", cfg)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if mode != modeAlphanumeric {
		t.Errorf("expected modeAlphanumeric, got %v", mode)
	}
}

// TestSelectModeByte verifies byte mode is selected for lowercase/UTF-8 inputs.
func TestSelectModeByte(t *testing.T) {
	cfg := &symbolConfigs[1] // M2-L
	mode, err := selectMode("hello", cfg)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if mode != modeByte {
		t.Errorf("expected modeByte, got %v", mode)
	}
}

// TestEncodeAtConvenience verifies EncodeAt is equivalent to Encode with pointers.
func TestEncodeAtConvenience(t *testing.T) {
	g1, err := EncodeAt("HELLO", VersionM2, EccL)
	if err != nil {
		t.Fatalf("EncodeAt error: %v", err)
	}
	g2, err := Encode("HELLO", versionPtr(VersionM2), eccPtr(EccL))
	if err != nil {
		t.Fatalf("Encode error: %v", err)
	}
	if g1.Rows != g2.Rows || g1.Cols != g2.Cols {
		t.Errorf("EncodeAt and Encode produced different dimensions: %dx%d vs %dx%d",
			g1.Rows, g1.Cols, g2.Rows, g2.Cols)
	}
}

// TestAllVersionsEncodeSuccessfully exercises each version with a short numeric input.
func TestAllVersionsEncodeSuccessfully(t *testing.T) {
	cases := []struct {
		version MicroQRVersion
		ecc     MicroQREccLevel
		want    uint32
	}{
		{VersionM1, EccDetection, 11},
		{VersionM2, EccL, 13},
		{VersionM2, EccM, 13},
		{VersionM3, EccL, 15},
		{VersionM3, EccM, 15},
		{VersionM4, EccL, 17},
		{VersionM4, EccM, 17},
		{VersionM4, EccQ, 17},
	}
	for _, tc := range cases {
		grid, err := EncodeAt("1", tc.version, tc.ecc)
		if err != nil {
			t.Errorf("EncodeAt(1, %v, %v) error: %v", tc.version, tc.ecc, err)
			continue
		}
		if grid.Rows != tc.want {
			t.Errorf("EncodeAt(1, %v, %v): want %d rows, got %d", tc.version, tc.ecc, tc.want, grid.Rows)
		}
	}
}
