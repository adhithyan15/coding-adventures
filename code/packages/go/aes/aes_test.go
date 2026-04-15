package aes

import (
	"bytes"
	"encoding/hex"
	"strings"
	"testing"
)

// h decodes a hex string (spaces ignored) to bytes. Panics on bad input.
func h(hexStr string) []byte {
	b, err := hex.DecodeString(strings.ReplaceAll(hexStr, " ", ""))
	if err != nil {
		panic(err)
	}
	return b
}

// =============================================================================
// FIPS 197 Known-Answer Tests
// =============================================================================

// TestAES128_AppendixB tests AES-128 with the FIPS 197 Appendix B vector.
func TestAES128_AppendixB_Encrypt(t *testing.T) {
	key := h("2b7e151628aed2a6 abf7158809cf4f3c")
	plain := h("3243f6a8885a308d 313198a2e0370734")
	want := h("3925841d02dc09fb dc118597196a0b32")
	ct, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatalf("EncryptBlock error: %v", err)
	}
	if !bytes.Equal(ct, want) {
		t.Errorf("AES-128 encrypt = %X, want %X", ct, want)
	}
}

func TestAES128_AppendixB_Decrypt(t *testing.T) {
	key := h("2b7e151628aed2a6 abf7158809cf4f3c")
	ct := h("3925841d02dc09fb dc118597196a0b32")
	want := h("3243f6a8885a308d 313198a2e0370734")
	pt, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, want) {
		t.Errorf("AES-128 decrypt = %X, want %X", pt, want)
	}
}

func TestAES128_AppendixC1(t *testing.T) {
	// FIPS 197 Appendix C.1 — AES-128 with sequential key 000102…0f.
	key := h("000102030405060708090a0b0c0d0e0f")
	plain := h("00112233445566778899aabbccddeeff")
	want := h("69c4e0d86a7b0430d8cdb78070b4c55a")
	ct, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, want) {
		t.Errorf("AES-128 C.1 encrypt = %X, want %X", ct, want)
	}
}

func TestAES128_AppendixC1_Decrypt(t *testing.T) {
	key := h("000102030405060708090a0b0c0d0e0f")
	ct := h("69c4e0d86a7b0430d8cdb78070b4c55a")
	want := h("00112233445566778899aabbccddeeff")
	pt, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, want) {
		t.Errorf("AES-128 C.1 decrypt = %X, want %X", pt, want)
	}
}

func TestAES128_RoundTrip(t *testing.T) {
	key := h("2b7e151628aed2a6abf7158809cf4f3c")
	for start := 0; start < 256; start += 32 {
		plain := make([]byte, 16)
		for i := range plain {
			plain[i] = byte((start + i) & 0xFF)
		}
		ct, err := EncryptBlock(plain, key)
		if err != nil {
			t.Fatal(err)
		}
		got, err := DecryptBlock(ct, key)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(got, plain) {
			t.Errorf("start=%d: AES-128 round-trip mismatch", start)
		}
	}
}

// TestAES192 uses the FIPS 197 Appendix C.2 vector.
func TestAES192_AppendixC2_Encrypt(t *testing.T) {
	key := h("000102030405060708090a0b0c0d0e0f1011121314151617")
	plain := h("00112233445566778899aabbccddeeff")
	want := h("dda97ca4864cdfe06eaf70a0ec0d7191")
	ct, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, want) {
		t.Errorf("AES-192 encrypt = %X, want %X", ct, want)
	}
}

func TestAES192_AppendixC2_Decrypt(t *testing.T) {
	key := h("000102030405060708090a0b0c0d0e0f1011121314151617")
	ct := h("dda97ca4864cdfe06eaf70a0ec0d7191")
	want := h("00112233445566778899aabbccddeeff")
	pt, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, want) {
		t.Errorf("AES-192 decrypt = %X, want %X", pt, want)
	}
}

func TestAES192_RoundTrip(t *testing.T) {
	key := h("000102030405060708090a0b0c0d0e0f1011121314151617")
	for start := 0; start < 256; start += 32 {
		plain := make([]byte, 16)
		for i := range plain {
			plain[i] = byte((start + i) & 0xFF)
		}
		ct, _ := EncryptBlock(plain, key)
		got, _ := DecryptBlock(ct, key)
		if !bytes.Equal(got, plain) {
			t.Errorf("start=%d: AES-192 round-trip mismatch", start)
		}
	}
}

// TestAES256 tests AES-256 with two vectors.
func TestAES256_Vector1_Encrypt(t *testing.T) {
	key := h("603deb1015ca71be2b73aef0857d7781 1f352c073b6108d72d9810a30914dff4")
	plain := h("6bc1bee22e409f96e93d7e117393172a")
	want := h("f3eed1bdb5d2a03c064b5a7e3db181f8")
	ct, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, want) {
		t.Errorf("AES-256 v1 encrypt = %X, want %X", ct, want)
	}
}

func TestAES256_Vector1_Decrypt(t *testing.T) {
	key := h("603deb1015ca71be2b73aef0857d7781 1f352c073b6108d72d9810a30914dff4")
	ct := h("f3eed1bdb5d2a03c064b5a7e3db181f8")
	want := h("6bc1bee22e409f96e93d7e117393172a")
	pt, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, want) {
		t.Errorf("AES-256 v1 decrypt = %X, want %X", pt, want)
	}
}

func TestAES256_AppendixC3(t *testing.T) {
	// FIPS 197 Appendix C.3 — AES-256 with sequential 32-byte key.
	key := h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
	plain := h("00112233445566778899aabbccddeeff")
	want := h("8ea2b7ca516745bfeafc49904b496089")
	ct, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, want) {
		t.Errorf("AES-256 C.3 encrypt = %X, want %X", ct, want)
	}
	pt, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, plain) {
		t.Errorf("AES-256 C.3 decrypt round-trip mismatch")
	}
}

func TestAES256_RoundTrip(t *testing.T) {
	key := h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
	for start := 0; start < 256; start += 32 {
		plain := make([]byte, 16)
		for i := range plain {
			plain[i] = byte((start + i) & 0xFF)
		}
		ct, _ := EncryptBlock(plain, key)
		got, _ := DecryptBlock(ct, key)
		if !bytes.Equal(got, plain) {
			t.Errorf("start=%d: AES-256 round-trip mismatch", start)
		}
	}
}

// =============================================================================
// S-box properties
// =============================================================================

func TestSBOX_Length(t *testing.T) {
	if len(SBOX) != 256 {
		t.Errorf("SBOX len = %d, want 256", len(SBOX))
	}
}

func TestINV_SBOX_Length(t *testing.T) {
	if len(INV_SBOX) != 256 {
		t.Errorf("INV_SBOX len = %d, want 256", len(INV_SBOX))
	}
}

func TestSBOX_IsBijection(t *testing.T) {
	// S-box must be a permutation (all 256 outputs distinct).
	seen := make(map[byte]bool)
	for b := 0; b < 256; b++ {
		v := SBOX[b]
		if seen[v] {
			t.Errorf("SBOX is not a bijection: value %02x appears twice", v)
		}
		seen[v] = true
	}
}

func TestINV_SBOX_IsBijection(t *testing.T) {
	seen := make(map[byte]bool)
	for b := 0; b < 256; b++ {
		v := INV_SBOX[b]
		if seen[v] {
			t.Errorf("INV_SBOX is not a bijection: value %02x appears twice", v)
		}
		seen[v] = true
	}
}

func TestSBOX_INV_SBOX_AreInverses(t *testing.T) {
	for b := 0; b < 256; b++ {
		if INV_SBOX[SBOX[b]] != byte(b) {
			t.Errorf("INV_SBOX[SBOX[%02x]] = %02x, want %02x", b, INV_SBOX[SBOX[b]], b)
		}
	}
}

func TestSBOX_KnownValues(t *testing.T) {
	// FIPS 197 Figure 7 spot-checks.
	tests := []struct{ in, out byte }{
		{0x00, 0x63},
		{0x01, 0x7c},
		{0xff, 0x16},
		{0x53, 0xed},
	}
	for _, tt := range tests {
		if SBOX[tt.in] != tt.out {
			t.Errorf("SBOX[%02x] = %02x, want %02x", tt.in, SBOX[tt.in], tt.out)
		}
	}
}

func TestINV_SBOX_KnownValues(t *testing.T) {
	if INV_SBOX[0x63] != 0x00 {
		t.Errorf("INV_SBOX[0x63] = %02x, want 0x00", INV_SBOX[0x63])
	}
	if INV_SBOX[0x7c] != 0x01 {
		t.Errorf("INV_SBOX[0x7c] = %02x, want 0x01", INV_SBOX[0x7c])
	}
}

func TestSBOX_NoFixedPoints(t *testing.T) {
	// The affine constant 0x63 ensures no byte maps to itself.
	for b := 0; b < 256; b++ {
		if SBOX[b] == byte(b) {
			t.Errorf("fixed point at %02x: SBOX[%02x] = %02x", b, b, SBOX[b])
		}
	}
}

// =============================================================================
// ExpandKey tests
// =============================================================================

func TestExpandKey_AES128_RoundCount(t *testing.T) {
	rks, err := ExpandKey(make([]byte, 16))
	if err != nil {
		t.Fatal(err)
	}
	if len(rks) != 11 { // Nr+1 = 11
		t.Errorf("AES-128 round key count = %d, want 11", len(rks))
	}
}

func TestExpandKey_AES192_RoundCount(t *testing.T) {
	rks, err := ExpandKey(make([]byte, 24))
	if err != nil {
		t.Fatal(err)
	}
	if len(rks) != 13 { // Nr+1 = 13
		t.Errorf("AES-192 round key count = %d, want 13", len(rks))
	}
}

func TestExpandKey_AES256_RoundCount(t *testing.T) {
	rks, err := ExpandKey(make([]byte, 32))
	if err != nil {
		t.Fatal(err)
	}
	if len(rks) != 15 { // Nr+1 = 15
		t.Errorf("AES-256 round key count = %d, want 15", len(rks))
	}
}

func TestExpandKey_RoundKeyShape(t *testing.T) {
	// Each round key must be a 4×4 matrix (4 rows, each a [4]byte).
	for _, keyLen := range []int{16, 24, 32} {
		rks, err := ExpandKey(make([]byte, keyLen))
		if err != nil {
			t.Fatal(err)
		}
		for i, rk := range rks {
			if len(rk) != 4 {
				t.Errorf("key len %d, round key %d: got %d rows, want 4", keyLen, i, len(rk))
			}
		}
	}
}

func TestExpandKey_FirstRoundKeyEqualsKey(t *testing.T) {
	// The first round key must equal the key bytes (column-major).
	key := h("2b7e151628aed2a6abf7158809cf4f3c")
	rks, err := ExpandKey(key)
	if err != nil {
		t.Fatal(err)
	}
	// Reconstruct: column-major order
	var reconstructed [16]byte
	idx := 0
	for col := 0; col < 4; col++ {
		for row := 0; row < 4; row++ {
			reconstructed[idx] = rks[0][row][col]
			idx++
		}
	}
	if !bytes.Equal(reconstructed[:], key) {
		t.Errorf("first round key does not match key bytes")
	}
}

func TestExpandKey_DifferentKeysYieldDifferentRoundKeys(t *testing.T) {
	rks1, _ := ExpandKey(make([]byte, 16))
	key2 := make([]byte, 16)
	key2[0] = 1
	rks2, _ := ExpandKey(key2)
	// First round keys should differ.
	if rks1[0][0][0] == rks2[0][0][0] && rks1[0][0][1] == rks2[0][0][1] {
		t.Error("different keys produced identical first round keys")
	}
}

func TestExpandKey_InvalidKeyLength(t *testing.T) {
	_, err := ExpandKey(make([]byte, 15))
	if err == nil {
		t.Error("expected error for 15-byte key")
	}
}

func TestExpandKey_InvalidKeyLength17(t *testing.T) {
	_, err := ExpandKey(make([]byte, 17))
	if err == nil {
		t.Error("expected error for 17-byte key")
	}
}

// =============================================================================
// Block size validation
// =============================================================================

func TestEncryptBlock_WrongBlockSize_Short(t *testing.T) {
	_, err := EncryptBlock(make([]byte, 15), make([]byte, 16))
	if err == nil {
		t.Error("expected error for 15-byte block")
	}
}

func TestEncryptBlock_WrongBlockSize_Long(t *testing.T) {
	_, err := EncryptBlock(make([]byte, 17), make([]byte, 16))
	if err == nil {
		t.Error("expected error for 17-byte block")
	}
}

func TestDecryptBlock_WrongBlockSize(t *testing.T) {
	_, err := DecryptBlock(make([]byte, 15), make([]byte, 16))
	if err == nil {
		t.Error("expected error for 15-byte block")
	}
}

func TestEncryptBlock_WrongKeySize(t *testing.T) {
	_, err := EncryptBlock(make([]byte, 16), make([]byte, 10))
	if err == nil {
		t.Error("expected error for 10-byte key")
	}
}

func TestDecryptBlock_WrongKeySize(t *testing.T) {
	_, err := DecryptBlock(make([]byte, 16), make([]byte, 20))
	if err == nil {
		t.Error("expected error for 20-byte key")
	}
}

// =============================================================================
// Round-trip tests across all key sizes
// =============================================================================

func TestRoundTrip_AllZeros(t *testing.T) {
	for _, keyLen := range []int{16, 24, 32} {
		key := make([]byte, keyLen)
		plain := make([]byte, 16)
		ct, err := EncryptBlock(plain, key)
		if err != nil {
			t.Fatal(err)
		}
		got, err := DecryptBlock(ct, key)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(got, plain) {
			t.Errorf("keyLen=%d all-zeros round-trip mismatch", keyLen)
		}
	}
}

func TestRoundTrip_AllFF(t *testing.T) {
	for _, keyLen := range []int{16, 24, 32} {
		key := bytes.Repeat([]byte{0xFF}, keyLen)
		plain := bytes.Repeat([]byte{0xFF}, 16)
		ct, _ := EncryptBlock(plain, key)
		got, _ := DecryptBlock(ct, key)
		if !bytes.Equal(got, plain) {
			t.Errorf("keyLen=%d all-FF round-trip mismatch", keyLen)
		}
	}
}

func TestRoundTrip_IdentityKeyAndPlain(t *testing.T) {
	for _, keyLen := range []int{16, 24, 32} {
		key := make([]byte, keyLen)
		for i := range key {
			key[i] = byte(i)
		}
		plain := make([]byte, 16)
		for i := range plain {
			plain[i] = byte(i)
		}
		ct, _ := EncryptBlock(plain, key)
		got, _ := DecryptBlock(ct, key)
		if !bytes.Equal(got, plain) {
			t.Errorf("keyLen=%d identity round-trip mismatch", keyLen)
		}
	}
}

func TestEncrypt_AvalancheEffect(t *testing.T) {
	// Changing one plaintext bit should change many output bytes (avalanche).
	key := make([]byte, 16)
	plain1 := make([]byte, 16)
	plain2 := make([]byte, 16)
	plain2[0] = 0x01
	ct1, _ := EncryptBlock(plain1, key)
	ct2, _ := EncryptBlock(plain2, key)
	diffBits := 0
	for i := 0; i < 16; i++ {
		v := ct1[i] ^ ct2[i]
		for v != 0 {
			diffBits += int(v & 1)
			v >>= 1
		}
	}
	if diffBits < 32 {
		t.Errorf("poor avalanche: only %d bits differ (want > 32)", diffBits)
	}
}
