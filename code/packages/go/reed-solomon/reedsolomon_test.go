package reedsolomon_test

import (
	"bytes"
	"errors"
	"testing"

	rs "github.com/adhithyan15/coding-adventures/code/packages/go/reed-solomon"
	"github.com/adhithyan15/coding-adventures/code/packages/go/gf256"
)

// =============================================================================
// Version
// =============================================================================

func TestVersion(t *testing.T) {
	if rs.Version == "" {
		t.Fatal("Version must not be empty")
	}
	// Verify it's a semver-like string with two dots
	dots := 0
	for _, c := range rs.Version {
		if c == '.' {
			dots++
		}
	}
	if dots != 2 {
		t.Fatalf("Version %q does not look like semver (need 2 dots)", rs.Version)
	}
}

// =============================================================================
// BuildGenerator
// =============================================================================

func TestBuildGeneratorDegree(t *testing.T) {
	for _, nCheck := range []int{2, 4, 8, 16} {
		g, err := rs.BuildGenerator(nCheck)
		if err != nil {
			t.Fatalf("BuildGenerator(%d) error: %v", nCheck, err)
		}
		if len(g) != nCheck+1 {
			t.Errorf("BuildGenerator(%d): want length %d, got %d", nCheck, nCheck+1, len(g))
		}
	}
}

func TestBuildGeneratorMonic(t *testing.T) {
	for _, nCheck := range []int{2, 4, 8, 16} {
		g, _ := rs.BuildGenerator(nCheck)
		if g[len(g)-1] != 1 {
			t.Errorf("BuildGenerator(%d): leading coefficient (LE last) = %d, want 1", nCheck, g[len(g)-1])
		}
	}
}

func TestBuildGeneratorKnownN2(t *testing.T) {
	// g(x) = (x+2)(x+4) = x² + 6x + 8 → LE: [8, 6, 1]
	g, err := rs.BuildGenerator(2)
	if err != nil {
		t.Fatal(err)
	}
	want := []byte{8, 6, 1}
	if !bytes.Equal(g, want) {
		t.Errorf("BuildGenerator(2) = %v, want %v", g, want)
	}
}

func TestBuildGeneratorAlphaRoots(t *testing.T) {
	// Every α^i for i=1..nCheck must be a root of g(x).
	for _, nCheck := range []int{2, 4, 8} {
		g, _ := rs.BuildGenerator(nCheck)
		for i := 1; i <= nCheck; i++ {
			alphaI := gf256.Power(2, i)
			// Evaluate g at alphaI using Horner (LE polynomial)
			var acc byte
			for j := len(g) - 1; j >= 0; j-- {
				acc = gf256.Add(gf256.Multiply(acc, alphaI), g[j])
			}
			if acc != 0 {
				t.Errorf("BuildGenerator(%d): g(α^%d) = %d, want 0", nCheck, i, acc)
			}
		}
	}
}

func TestBuildGeneratorInvalidNCheck(t *testing.T) {
	for _, bad := range []int{0, 1, 3, 5, 7} {
		_, err := rs.BuildGenerator(bad)
		if err == nil {
			t.Errorf("BuildGenerator(%d): expected error, got nil", bad)
		}
		var invErr *rs.InvalidInputError
		if !errors.As(err, &invErr) {
			t.Errorf("BuildGenerator(%d): expected InvalidInputError, got %T", bad, err)
		}
	}
}

// =============================================================================
// Encode
// =============================================================================

func TestEncodeOutputLength(t *testing.T) {
	msg := []byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	for _, nCheck := range []int{2, 4, 8} {
		cw, err := rs.Encode(msg, nCheck)
		if err != nil {
			t.Fatalf("Encode nCheck=%d: %v", nCheck, err)
		}
		if len(cw) != len(msg)+nCheck {
			t.Errorf("Encode nCheck=%d: len=%d, want %d", nCheck, len(cw), len(msg)+nCheck)
		}
	}
}

func TestEncodeMessagePreserved(t *testing.T) {
	msg := []byte{10, 20, 30, 40, 50}
	for _, nCheck := range []int{2, 4, 8} {
		cw, _ := rs.Encode(msg, nCheck)
		if !bytes.Equal(cw[:len(msg)], msg) {
			t.Errorf("Encode nCheck=%d: message bytes changed", nCheck)
		}
	}
}

func TestEncodeZeroMessage(t *testing.T) {
	msg := make([]byte, 5)
	cw, _ := rs.Encode(msg, 4)
	for _, b := range cw {
		if b != 0 {
			t.Errorf("Encode all-zero message: got non-zero byte %d in codeword", b)
		}
	}
}

func TestEncodeEmptyMessage(t *testing.T) {
	cw, err := rs.Encode([]byte{}, 4)
	if err != nil {
		t.Fatal(err)
	}
	if len(cw) != 4 {
		t.Errorf("Encode empty: len=%d, want 4", len(cw))
	}
}

func TestEncodeSingleByte(t *testing.T) {
	msg := []byte{0xAB}
	cw, _ := rs.Encode(msg, 4)
	if len(cw) != 5 {
		t.Errorf("len=%d, want 5", len(cw))
	}
	if cw[0] != 0xAB {
		t.Errorf("first byte changed: %d", cw[0])
	}
}

func TestEncodeMaxValidLength(t *testing.T) {
	// 247 + 8 = 255 — exactly at the GF(256) block size limit
	msg := make([]byte, 247)
	cw, err := rs.Encode(msg, 8)
	if err != nil {
		t.Fatal(err)
	}
	if len(cw) != 255 {
		t.Errorf("len=%d, want 255", len(cw))
	}
}

func TestEncodeInvalidNCheck(t *testing.T) {
	for _, bad := range []int{0, 1, 3} {
		_, err := rs.Encode([]byte{1, 2, 3}, bad)
		if err == nil {
			t.Errorf("Encode nCheck=%d: expected error", bad)
		}
	}
}

func TestEncodeExceedsMaxLength(t *testing.T) {
	msg := make([]byte, 248) // 248 + 8 = 256 > 255
	_, err := rs.Encode(msg, 8)
	if err == nil {
		t.Error("Encode 256-byte codeword: expected error, got nil")
	}
}

// =============================================================================
// Syndromes
// =============================================================================

func TestSyndromesZeroOnValidCodeword(t *testing.T) {
	for _, nCheck := range []int{2, 4, 8} {
		msg := []byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
		cw, _ := rs.Encode(msg, nCheck)
		s := rs.Syndromes(cw, nCheck)
		if len(s) != nCheck {
			t.Errorf("nCheck=%d: got %d syndromes, want %d", nCheck, len(s), nCheck)
		}
		for i, v := range s {
			if v != 0 {
				t.Errorf("nCheck=%d: S[%d]=%d, want 0", nCheck, i+1, v)
			}
		}
	}
}

func TestSyndromesNonZeroAfterCorruption(t *testing.T) {
	msg := []byte("hello world")
	cw, _ := rs.Encode(msg, 8)
	corrupted := make([]byte, len(cw))
	copy(corrupted, cw)
	corrupted[0] ^= 0xFF
	s := rs.Syndromes(corrupted, 8)
	anyNonZero := false
	for _, v := range s {
		if v != 0 {
			anyNonZero = true
		}
	}
	if !anyNonZero {
		t.Error("Expected non-zero syndrome after corruption")
	}
}

// =============================================================================
// Round-Trip
// =============================================================================

func TestRoundTripASCII(t *testing.T) {
	msg := []byte("Hello, World!")
	for _, nCheck := range []int{2, 4, 8} {
		cw, _ := rs.Encode(msg, nCheck)
		recovered, err := rs.Decode(cw, nCheck)
		if err != nil {
			t.Fatalf("nCheck=%d: Decode error: %v", nCheck, err)
		}
		if !bytes.Equal(recovered, msg) {
			t.Errorf("nCheck=%d: round-trip failed", nCheck)
		}
	}
}

func TestRoundTripAllZero(t *testing.T) {
	msg := make([]byte, 20)
	cw, _ := rs.Encode(msg, 4)
	recovered, err := rs.Decode(cw, 4)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(recovered, msg) {
		t.Error("round-trip failed for all-zero message")
	}
}

func TestRoundTripAllFF(t *testing.T) {
	msg := bytes.Repeat([]byte{0xFF}, 20)
	cw, _ := rs.Encode(msg, 4)
	recovered, _ := rs.Decode(cw, 4)
	if !bytes.Equal(recovered, msg) {
		t.Error("round-trip failed for all-0xFF message")
	}
}

func TestRoundTripEmpty(t *testing.T) {
	cw, _ := rs.Encode([]byte{}, 4)
	recovered, err := rs.Decode(cw, 4)
	if err != nil {
		t.Fatal(err)
	}
	if len(recovered) != 0 {
		t.Errorf("empty message: got %v", recovered)
	}
}

func TestRoundTripSingleByte(t *testing.T) {
	for _, b := range []byte{0x00, 0x01, 0xAB, 0xFF} {
		msg := []byte{b}
		cw, _ := rs.Encode(msg, 4)
		recovered, err := rs.Decode(cw, 4)
		if err != nil {
			t.Fatalf("byte 0x%02x: %v", b, err)
		}
		if !bytes.Equal(recovered, msg) {
			t.Errorf("byte 0x%02x: round-trip failed", b)
		}
	}
}

func TestRoundTripMaxLength(t *testing.T) {
	msg := make([]byte, 247) // 247 + 8 = 255
	for i := range msg {
		msg[i] = byte(i)
	}
	cw, _ := rs.Encode(msg, 8)
	recovered, err := rs.Decode(cw, 8)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(recovered, msg) {
		t.Error("max-length round-trip failed")
	}
}

// =============================================================================
// Error Correction
// =============================================================================

func corrupt(cw []byte, positions []int, magnitudes []byte) []byte {
	result := make([]byte, len(cw))
	copy(result, cw)
	for i, pos := range positions {
		result[pos] ^= magnitudes[i]
	}
	return result
}

func TestSingleErrorEveryPosition(t *testing.T) {
	// n_check=2 → t=1: correct every single corrupted position
	msg := []byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	cw, _ := rs.Encode(msg, 2)
	for pos := 0; pos < len(cw); pos++ {
		corrupted := corrupt(cw, []int{pos}, []byte{0x5A})
		recovered, err := rs.Decode(corrupted, 2)
		if err != nil {
			t.Errorf("pos=%d: %v", pos, err)
			continue
		}
		if !bytes.Equal(recovered, msg) {
			t.Errorf("pos=%d: wrong recovery", pos)
		}
	}
}

func TestTwoErrors(t *testing.T) {
	msg := []byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	cw, _ := rs.Encode(msg, 4)
	n := len(cw)
	for pos1 := 0; pos1 < n; pos1 += 3 {
		for pos2 := pos1 + 1; pos2 < n; pos2 += 4 {
			corrupted := corrupt(cw, []int{pos1, pos2}, []byte{0xDE, 0xAD})
			recovered, err := rs.Decode(corrupted, 4)
			if err != nil {
				t.Errorf("pos1=%d,pos2=%d: %v", pos1, pos2, err)
				continue
			}
			if !bytes.Equal(recovered, msg) {
				t.Errorf("pos1=%d,pos2=%d: wrong recovery", pos1, pos2)
			}
		}
	}
}

func TestFourErrors(t *testing.T) {
	msg := []byte("Reed-Solomon")
	cw, _ := rs.Encode(msg, 8)
	corrupted := corrupt(cw, []int{0, 3, 7, 10}, []byte{0xFF, 0xAA, 0x55, 0x0F})
	recovered, err := rs.Decode(corrupted, 8)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(recovered, msg) {
		t.Error("four-error correction failed")
	}
}

func TestEveryErrorMagnitude(t *testing.T) {
	msg := []byte{1, 2, 3, 4, 5}
	cw, _ := rs.Encode(msg, 2)
	for mag := byte(1); mag != 0; mag++ { // 1..255
		corrupted := corrupt(cw, []int{0}, []byte{mag})
		recovered, err := rs.Decode(corrupted, 2)
		if err != nil {
			t.Errorf("mag=0x%02x: %v", mag, err)
			continue
		}
		if !bytes.Equal(recovered, msg) {
			t.Errorf("mag=0x%02x: wrong recovery", mag)
		}
	}
}

func TestCheckBytesCorrupted(t *testing.T) {
	msg := make([]byte, 10)
	for i := range msg {
		msg[i] = byte(i + 1)
	}
	cw, _ := rs.Encode(msg, 4)
	corrupted := corrupt(cw, []int{len(msg), len(msg) + 1}, []byte{0xAA, 0xBB})
	recovered, err := rs.Decode(corrupted, 4)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(recovered, msg) {
		t.Error("check-byte correction failed")
	}
}

// =============================================================================
// Capacity Limits
// =============================================================================

func TestTooManyErrors(t *testing.T) {
	msg := make([]byte, 10)
	cw := make([]byte, len(msg)+4)
	encoded, _ := rs.Encode(msg, 4)
	copy(cw, encoded)
	cw[0] ^= 0xAA
	cw[3] ^= 0xBB
	cw[7] ^= 0xCC
	_, err := rs.Decode(cw, 4)
	if !errors.Is(err, rs.ErrTooManyErrors) {
		t.Errorf("expected ErrTooManyErrors, got %v", err)
	}
}

func TestExactlyAtCapacity(t *testing.T) {
	msg := make([]byte, 10)
	cw, _ := rs.Encode(msg, 4)
	cw[0] ^= 0xAA
	cw[5] ^= 0xBB
	_, err := rs.Decode(cw, 4)
	if err != nil {
		t.Errorf("expected success at capacity, got: %v", err)
	}
}

func TestTooManyErrorsN8(t *testing.T) {
	msg := []byte("Hello")
	cw, _ := rs.Encode(msg, 8)
	for i := 0; i < 5; i++ {
		cw[i] ^= byte((i + 1) * 17)
	}
	_, err := rs.Decode(cw, 8)
	if !errors.Is(err, rs.ErrTooManyErrors) {
		t.Errorf("expected ErrTooManyErrors for 5 errors, got %v", err)
	}
}

// =============================================================================
// ErrorLocator
// =============================================================================

func TestErrorLocatorNoErrors(t *testing.T) {
	cw, _ := rs.Encode([]byte("hello world"), 8)
	s := rs.Syndromes(cw, 8)
	lam := rs.ErrorLocator(s)
	if !bytes.Equal(lam, []byte{1}) {
		t.Errorf("no-error locator = %v, want [1]", lam)
	}
}

func TestErrorLocatorOneError(t *testing.T) {
	msg := make([]byte, 5)
	cw, _ := rs.Encode(msg, 8)
	cw[2] ^= 0x77
	s := rs.Syndromes(cw, 8)
	lam := rs.ErrorLocator(s)
	if len(lam) != 2 {
		t.Errorf("one-error locator len=%d, want 2", len(lam))
	}
	if lam[0] != 1 {
		t.Errorf("lam[0]=%d, want 1", lam[0])
	}
}

func TestErrorLocatorTwoErrors(t *testing.T) {
	msg := make([]byte, 10)
	cw, _ := rs.Encode(msg, 8)
	cw[1] ^= 0xAA
	cw[8] ^= 0xBB
	s := rs.Syndromes(cw, 8)
	lam := rs.ErrorLocator(s)
	if len(lam) != 3 {
		t.Errorf("two-error locator len=%d, want 3", len(lam))
	}
}

// =============================================================================
// Decode Validation
// =============================================================================

func TestDecodeInvalidNCheck(t *testing.T) {
	for _, bad := range []int{0, 1, 3} {
		_, err := rs.Decode(make([]byte, 10), bad)
		if err == nil {
			t.Errorf("Decode nCheck=%d: expected error", bad)
		}
	}
}

func TestDecodeReceivedTooShort(t *testing.T) {
	_, err := rs.Decode([]byte{1, 2, 3}, 4)
	if err == nil {
		t.Error("expected error for received < nCheck")
	}
}

func TestDecodeExactlyNCheckLength(t *testing.T) {
	cw, _ := rs.Encode([]byte{}, 4)
	recovered, err := rs.Decode(cw, 4)
	if err != nil {
		t.Fatal(err)
	}
	if len(recovered) != 0 {
		t.Errorf("expected empty message, got %v", recovered)
	}
}

// =============================================================================
// Test Vectors (cross-validated with Rust and TypeScript)
// =============================================================================

func TestVectorGenerator2(t *testing.T) {
	// g(x) = (x+2)(x+4) = x² + 6x + 8 → LE: [8, 6, 1]
	g, _ := rs.BuildGenerator(2)
	want := []byte{8, 6, 1}
	if !bytes.Equal(g, want) {
		t.Errorf("BuildGenerator(2) = %v, want %v", g, want)
	}
}

func TestVectorRoundTrip(t *testing.T) {
	msg := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	nCheck := 8
	cw, _ := rs.Encode(msg, nCheck)
	if len(cw) != 16 {
		t.Errorf("len=%d, want 16", len(cw))
	}
	if !bytes.Equal(cw[:8], msg) {
		t.Error("systematic: message bytes changed")
	}
	s := rs.Syndromes(cw, nCheck)
	for i, v := range s {
		if v != 0 {
			t.Errorf("S[%d]=%d after encode, want 0", i, v)
		}
	}
	recovered, err := rs.Decode(cw, nCheck)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(recovered, msg) {
		t.Error("round-trip failed")
	}
}

func TestVectorKnownCorrection(t *testing.T) {
	msg := []byte("Reed-Solomon")
	nCheck := 8
	cw, _ := rs.Encode(msg, nCheck)
	corrupted := corrupt(cw, []int{0, 3, 7, 10}, []byte{0xFF, 0xAA, 0x55, 0x0F})
	recovered, err := rs.Decode(corrupted, nCheck)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(recovered, msg) {
		t.Error("known-corruption test failed")
	}
}

func TestVectorAlternatingBits(t *testing.T) {
	msg := bytes.Repeat([]byte{0xAA, 0x55}, 10)
	nCheck := 8
	cw, _ := rs.Encode(msg, nCheck)
	s := rs.Syndromes(cw, nCheck)
	for _, v := range s {
		if v != 0 {
			t.Error("non-zero syndrome on alternating-bit codeword")
		}
	}
	recovered, _ := rs.Decode(cw, nCheck)
	if !bytes.Equal(recovered, msg) {
		t.Error("alternating-bit round-trip failed")
	}
}
