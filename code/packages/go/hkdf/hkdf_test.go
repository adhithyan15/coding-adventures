package hkdf

import (
	"bytes"
	"encoding/hex"
	"errors"
	"testing"
)

// h decodes a hex string to bytes. Panics on invalid hex.
func h(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic(err)
	}
	return b
}

// seq returns a byte slice containing sequential values from start to end-1.
func seq(start, end int) []byte {
	b := make([]byte, end-start)
	for i := range b {
		b[i] = byte(start + i)
	}
	return b
}

// =============================================================================
// RFC 5869 Appendix A — Test Vectors
// =============================================================================

// ── Test Case 1: Basic SHA-256 ──────────────────────────────────────────────

func TestTC1Extract(t *testing.T) {
	ikm := h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
	salt := h("000102030405060708090a0b0c")
	prk := Extract(salt, ikm, SHA256)
	expected := "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
	if hex.EncodeToString(prk) != expected {
		t.Errorf("TC1 Extract:\n  got  %s\n  want %s", hex.EncodeToString(prk), expected)
	}
}

func TestTC1Expand(t *testing.T) {
	prk := h("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5")
	info := h("f0f1f2f3f4f5f6f7f8f9")
	okm, err := Expand(prk, info, 42, SHA256)
	if err != nil {
		t.Fatalf("TC1 Expand error: %v", err)
	}
	expected := "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
	if hex.EncodeToString(okm) != expected {
		t.Errorf("TC1 Expand:\n  got  %s\n  want %s", hex.EncodeToString(okm), expected)
	}
}

func TestTC1Combined(t *testing.T) {
	ikm := h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
	salt := h("000102030405060708090a0b0c")
	info := h("f0f1f2f3f4f5f6f7f8f9")
	okm, err := HKDF(salt, ikm, info, 42, SHA256)
	if err != nil {
		t.Fatalf("TC1 Combined error: %v", err)
	}
	expected := "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
	if hex.EncodeToString(okm) != expected {
		t.Errorf("TC1 Combined:\n  got  %s\n  want %s", hex.EncodeToString(okm), expected)
	}
}

// ── Test Case 2: Longer inputs ──────────────────────────────────────────────

func TestTC2Extract(t *testing.T) {
	ikm := seq(0x00, 0x50)  // 80 bytes: 0x00..0x4f
	salt := seq(0x60, 0xB0) // 80 bytes: 0x60..0xaf
	prk := Extract(salt, ikm, SHA256)
	expected := "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244"
	if hex.EncodeToString(prk) != expected {
		t.Errorf("TC2 Extract:\n  got  %s\n  want %s", hex.EncodeToString(prk), expected)
	}
}

func TestTC2Expand(t *testing.T) {
	prk := h("06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244")
	info := seq(0xB0, 0x100) // 80 bytes: 0xb0..0xff
	okm, err := Expand(prk, info, 82, SHA256)
	if err != nil {
		t.Fatalf("TC2 Expand error: %v", err)
	}
	expected := "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"
	if hex.EncodeToString(okm) != expected {
		t.Errorf("TC2 Expand:\n  got  %s\n  want %s", hex.EncodeToString(okm), expected)
	}
}

func TestTC2Combined(t *testing.T) {
	ikm := seq(0x00, 0x50)
	salt := seq(0x60, 0xB0)
	info := seq(0xB0, 0x100)
	okm, err := HKDF(salt, ikm, info, 82, SHA256)
	if err != nil {
		t.Fatalf("TC2 Combined error: %v", err)
	}
	expected := "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"
	if hex.EncodeToString(okm) != expected {
		t.Errorf("TC2 Combined:\n  got  %s\n  want %s", hex.EncodeToString(okm), expected)
	}
}

// ── Test Case 3: Empty salt and info ────────────────────────────────────────

func TestTC3Extract(t *testing.T) {
	ikm := h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
	prk := Extract(nil, ikm, SHA256)
	expected := "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"
	if hex.EncodeToString(prk) != expected {
		t.Errorf("TC3 Extract:\n  got  %s\n  want %s", hex.EncodeToString(prk), expected)
	}
}

func TestTC3Expand(t *testing.T) {
	prk := h("19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04")
	okm, err := Expand(prk, nil, 42, SHA256)
	if err != nil {
		t.Fatalf("TC3 Expand error: %v", err)
	}
	expected := "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
	if hex.EncodeToString(okm) != expected {
		t.Errorf("TC3 Expand:\n  got  %s\n  want %s", hex.EncodeToString(okm), expected)
	}
}

func TestTC3Combined(t *testing.T) {
	ikm := h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
	okm, err := HKDF(nil, ikm, nil, 42, SHA256)
	if err != nil {
		t.Fatalf("TC3 Combined error: %v", err)
	}
	expected := "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
	if hex.EncodeToString(okm) != expected {
		t.Errorf("TC3 Combined:\n  got  %s\n  want %s", hex.EncodeToString(okm), expected)
	}
}

// =============================================================================
// Edge Cases
// =============================================================================

func TestExpandExactlyHashLen(t *testing.T) {
	prk := h("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5")
	okm, err := Expand(prk, []byte("test"), 32, SHA256)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(okm) != 32 {
		t.Errorf("expected 32 bytes, got %d", len(okm))
	}
}

func TestExpandOneByte(t *testing.T) {
	prk := bytes.Repeat([]byte{0x01}, 32)
	okm, err := Expand(prk, nil, 1, SHA256)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(okm) != 1 {
		t.Errorf("expected 1 byte, got %d", len(okm))
	}
}

func TestExpandMaxLengthSHA256(t *testing.T) {
	prk := bytes.Repeat([]byte{0x01}, 32)
	okm, err := Expand(prk, nil, 255*32, SHA256)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(okm) != 8160 {
		t.Errorf("expected 8160 bytes, got %d", len(okm))
	}
}

func TestExpandExceedsMaxLength(t *testing.T) {
	prk := bytes.Repeat([]byte{0x01}, 32)
	_, err := Expand(prk, nil, 255*32+1, SHA256)
	if err == nil {
		t.Fatal("expected error for length > max")
	}
	if !errors.Is(err, ErrOutputTooLong) {
		t.Errorf("expected ErrOutputTooLong, got: %v", err)
	}
}

func TestExpandZeroLength(t *testing.T) {
	prk := bytes.Repeat([]byte{0x01}, 32)
	_, err := Expand(prk, nil, 0, SHA256)
	if err == nil {
		t.Fatal("expected error for length 0")
	}
	if !errors.Is(err, ErrOutputTooShort) {
		t.Errorf("expected ErrOutputTooShort, got: %v", err)
	}
}

func TestExpandNegativeLength(t *testing.T) {
	prk := bytes.Repeat([]byte{0x01}, 32)
	_, err := Expand(prk, nil, -1, SHA256)
	if err == nil {
		t.Fatal("expected error for negative length")
	}
	if !errors.Is(err, ErrOutputTooShort) {
		t.Errorf("expected ErrOutputTooShort, got: %v", err)
	}
}

func TestSHA512Basic(t *testing.T) {
	ikm := h("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
	salt := h("000102030405060708090a0b0c")
	prk := Extract(salt, ikm, SHA512)
	if len(prk) != 64 {
		t.Errorf("SHA-512 PRK should be 64 bytes, got %d", len(prk))
	}
	okm, err := Expand(prk, []byte("info"), 64, SHA512)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(okm) != 64 {
		t.Errorf("expected 64 bytes, got %d", len(okm))
	}
}

func TestSHA512EmptySalt(t *testing.T) {
	ikm := bytes.Repeat([]byte{0xab}, 32)
	prk := Extract(nil, ikm, SHA512)
	if len(prk) != 64 {
		t.Errorf("SHA-512 PRK should be 64 bytes, got %d", len(prk))
	}
}

func TestSHA512MaxLength(t *testing.T) {
	prk := bytes.Repeat([]byte{0x01}, 64)
	okm, err := Expand(prk, nil, 255*64, SHA512)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(okm) != 16320 {
		t.Errorf("expected 16320 bytes, got %d", len(okm))
	}
}

func TestSHA512ExceedsMax(t *testing.T) {
	prk := bytes.Repeat([]byte{0x01}, 64)
	_, err := Expand(prk, nil, 255*64+1, SHA512)
	if err == nil {
		t.Fatal("expected error for length > max")
	}
	if !errors.Is(err, ErrOutputTooLong) {
		t.Errorf("expected ErrOutputTooLong, got: %v", err)
	}
}

func TestDifferentInfoDifferentOKM(t *testing.T) {
	prk := bytes.Repeat([]byte{0x01}, 32)
	okm1, _ := Expand(prk, []byte("purpose-a"), 32, SHA256)
	okm2, _ := Expand(prk, []byte("purpose-b"), 32, SHA256)
	if bytes.Equal(okm1, okm2) {
		t.Error("different info should produce different OKM")
	}
}

func TestDifferentSaltDifferentPRK(t *testing.T) {
	ikm := bytes.Repeat([]byte{0x01}, 32)
	prk1 := Extract([]byte("salt-1"), ikm, SHA256)
	prk2 := Extract([]byte("salt-2"), ikm, SHA256)
	if bytes.Equal(prk1, prk2) {
		t.Error("different salts should produce different PRKs")
	}
}

func TestDeterministic(t *testing.T) {
	okm1, _ := HKDF([]byte("salt"), []byte("ikm"), []byte("info"), 42, SHA256)
	okm2, _ := HKDF([]byte("salt"), []byte("ikm"), []byte("info"), 42, SHA256)
	if !bytes.Equal(okm1, okm2) {
		t.Error("same inputs should produce same output")
	}
}

func TestRoundTripExtractExpand(t *testing.T) {
	salt := []byte("my-salt")
	ikm := []byte("my-input-keying-material")
	info := []byte("my-context")
	length := 48

	combined, err := HKDF(salt, ikm, info, length, SHA256)
	if err != nil {
		t.Fatalf("HKDF error: %v", err)
	}
	prk := Extract(salt, ikm, SHA256)
	manual, err := Expand(prk, info, length, SHA256)
	if err != nil {
		t.Fatalf("Expand error: %v", err)
	}
	if !bytes.Equal(combined, manual) {
		t.Error("combined HKDF should equal manual extract+expand")
	}
}
