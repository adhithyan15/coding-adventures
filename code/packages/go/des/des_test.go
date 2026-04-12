package des

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
// NIST FIPS 46-3 / SP 800-20 Known-Answer Tests
// =============================================================================

func TestEncryptBlock_FIPSVector1(t *testing.T) {
	// Classic DES example from Stallings / FIPS 46 worked example.
	// Key = 133457799BBCDFF1 (with parity bits in bit-8 of each byte).
	key := h("133457799BBCDFF1")
	plain := h("0123456789ABCDEF")
	want := h("85E813540F0AB405")
	ct, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatalf("EncryptBlock error: %v", err)
	}
	if !bytes.Equal(ct, want) {
		t.Errorf("EncryptBlock = %X, want %X", ct, want)
	}
}

func TestEncryptBlock_SP800_20_Table1_Row0(t *testing.T) {
	// SP 800-20 Table 1 — plaintext variable, key=0101010101010101.
	key := h("0101010101010101")
	ct, err := EncryptBlock(h("95F8A5E5DD31D900"), key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, h("8000000000000000")) {
		t.Errorf("got %X", ct)
	}
}

func TestEncryptBlock_SP800_20_Table1_Row1(t *testing.T) {
	key := h("0101010101010101")
	ct, err := EncryptBlock(h("DD7F121CA5015619"), key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, h("4000000000000000")) {
		t.Errorf("got %X", ct)
	}
}

func TestEncryptBlock_SP800_20_Table1_Row2(t *testing.T) {
	key := h("0101010101010101")
	ct, err := EncryptBlock(h("2E8653104F3834EA"), key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, h("2000000000000000")) {
		t.Errorf("got %X", ct)
	}
}

func TestEncryptBlock_SP800_20_Table2_Row0(t *testing.T) {
	// SP 800-20 Table 2 — key variable, plaintext=0000000000000000.
	ct, err := EncryptBlock(h("0000000000000000"), h("8001010101010101"))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, h("95A8D72813DAA94D")) {
		t.Errorf("got %X", ct)
	}
}

func TestEncryptBlock_SP800_20_Table2_Row1(t *testing.T) {
	ct, err := EncryptBlock(h("0000000000000000"), h("4001010101010101"))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, h("0EEC1487DD8C26D5")) {
		t.Errorf("got %X", ct)
	}
}

func TestEncryptBlock_ParityBitKey(t *testing.T) {
	// Key with only parity bit set — round-trip rather than hardcoded vector
	// because parity-bit handling details vary across implementations.
	key := h("0000000000000080")
	plain := h("0000000000000000")
	ct, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatal(err)
	}
	got, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(got, plain) {
		t.Errorf("round-trip failed: got %X", got)
	}
}

// =============================================================================
// DecryptBlock tests
// =============================================================================

func TestDecryptBlock_FIPSVector1(t *testing.T) {
	key := h("133457799BBCDFF1")
	ct := h("85E813540F0AB405")
	want := h("0123456789ABCDEF")
	pt, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, want) {
		t.Errorf("DecryptBlock = %X, want %X", pt, want)
	}
}

func TestDecryptBlock_RoundTrip(t *testing.T) {
	key := h("133457799BBCDFF1")
	plain := h("0123456789ABCDEF")
	ct, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatal(err)
	}
	got, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(got, plain) {
		t.Errorf("round-trip mismatch: got %X, want %X", got, plain)
	}
}

func TestDecryptBlock_RoundTripAllBytes(t *testing.T) {
	// Round-trip a spread of byte values to check correctness broadly.
	key := h("FEDCBA9876543210")
	for start := 0; start < 256; start += 16 {
		plain := make([]byte, 8)
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
			t.Errorf("start=%d: round-trip mismatch", start)
		}
	}
}

func TestDecryptBlock_MultipleKeys(t *testing.T) {
	keys := [][]byte{
		h("133457799BBCDFF0"),
		h("FFFFFFFFFFFFFFFF"),
		h("0000000000000000"),
		h("FEDCBA9876543210"),
	}
	plain := h("0123456789ABCDEF")
	for _, key := range keys {
		ct, err := EncryptBlock(plain, key)
		if err != nil {
			t.Fatal(err)
		}
		got, err := DecryptBlock(ct, key)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(got, plain) {
			t.Errorf("key=%X: round-trip mismatch", key)
		}
	}
}

// =============================================================================
// ExpandKey tests
// =============================================================================

func TestExpandKey_Returns16Subkeys(t *testing.T) {
	key := h("0133457799BBCDFF")
	subkeys, err := ExpandKey(key)
	if err != nil {
		t.Fatal(err)
	}
	if len(subkeys) != 16 {
		t.Errorf("len(subkeys) = %d, want 16", len(subkeys))
	}
}

func TestExpandKey_SubkeysAre6Bytes(t *testing.T) {
	key := h("0133457799BBCDFF")
	subkeys, err := ExpandKey(key)
	if err != nil {
		t.Fatal(err)
	}
	for i, sk := range subkeys {
		if len(sk) != 6 {
			t.Errorf("subkey[%d] len = %d, want 6", i, len(sk))
		}
	}
}

func TestExpandKey_DifferentKeysYieldDifferentSubkeys(t *testing.T) {
	sk1, _ := ExpandKey(h("0133457799BBCDFF"))
	sk2, _ := ExpandKey(h("FEDCBA9876543210"))
	if bytes.Equal(sk1[0], sk2[0]) {
		t.Error("different keys produced same first subkey")
	}
}

func TestExpandKey_SubkeysNotAllSame(t *testing.T) {
	// A degenerate key schedule would produce identical subkeys -- broken.
	key := h("0133457799BBCDFF")
	subkeys, _ := ExpandKey(key)
	seen := make(map[string]bool)
	for _, sk := range subkeys {
		seen[string(sk)] = true
	}
	if len(seen) <= 1 {
		t.Error("all 16 subkeys are identical -- key schedule is broken")
	}
}

func TestExpandKey_InvalidKeyLength(t *testing.T) {
	_, err := ExpandKey(make([]byte, 7))
	if err == nil {
		t.Error("expected error for 7-byte key")
	}
}

func TestExpandKey_InvalidKeyLengthLong(t *testing.T) {
	_, err := ExpandKey(make([]byte, 9))
	if err == nil {
		t.Error("expected error for 9-byte key")
	}
}

// =============================================================================
// ECB mode
// =============================================================================

var ecbKey = h("0133457799BBCDFF")

func TestECBEncrypt_SingleBlock(t *testing.T) {
	// 8-byte input → 16 bytes out (1 data block + 1 full PKCS#7 padding block)
	plain := h("0123456789ABCDEF")
	ct, err := ECBEncrypt(plain, ecbKey)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 16 {
		t.Errorf("len(ct) = %d, want 16", len(ct))
	}
}

func TestECBEncrypt_SubBlock(t *testing.T) {
	// Less than 8 bytes → padded to 8 → 8 bytes ciphertext
	ct, err := ECBEncrypt([]byte("hello"), ecbKey)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 8 {
		t.Errorf("len(ct) = %d, want 8", len(ct))
	}
}

func TestECBEncrypt_MultiBlock(t *testing.T) {
	// 16 bytes → 24 bytes (2 data blocks + 1 padding block)
	ct, err := ECBEncrypt(make([]byte, 16), ecbKey)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 24 {
		t.Errorf("len(ct) = %d, want 24", len(ct))
	}
}

func TestECBEncrypt_EmptyInput(t *testing.T) {
	// Empty input → 8 bytes (full padding block only)
	ct, err := ECBEncrypt([]byte{}, ecbKey)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 8 {
		t.Errorf("len(ct) = %d, want 8", len(ct))
	}
}

func TestECBEncrypt_Deterministic(t *testing.T) {
	plain := []byte("Hello, World!!!")
	ct1, _ := ECBEncrypt(plain, ecbKey)
	ct2, _ := ECBEncrypt(plain, ecbKey)
	if !bytes.Equal(ct1, ct2) {
		t.Error("ECBEncrypt is not deterministic")
	}
}

func TestECBDecrypt_RoundTripShort(t *testing.T) {
	plain := []byte("hello")
	ct, _ := ECBEncrypt(plain, ecbKey)
	got, err := ECBDecrypt(ct, ecbKey)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(got, plain) {
		t.Errorf("round-trip mismatch: got %q", got)
	}
}

func TestECBDecrypt_RoundTripExactBlock(t *testing.T) {
	plain := []byte("ABCDEFGH")
	ct, _ := ECBEncrypt(plain, ecbKey)
	got, _ := ECBDecrypt(ct, ecbKey)
	if !bytes.Equal(got, plain) {
		t.Errorf("round-trip mismatch: got %q", got)
	}
}

func TestECBDecrypt_RoundTripMultiBlock(t *testing.T) {
	plain := []byte("The quick brown fox jumps")
	ct, _ := ECBEncrypt(plain, ecbKey)
	got, _ := ECBDecrypt(ct, ecbKey)
	if !bytes.Equal(got, plain) {
		t.Errorf("round-trip mismatch: got %q", got)
	}
}

func TestECBDecrypt_RoundTripEmpty(t *testing.T) {
	plain := []byte{}
	ct, _ := ECBEncrypt(plain, ecbKey)
	got, _ := ECBDecrypt(ct, ecbKey)
	if !bytes.Equal(got, plain) {
		t.Errorf("round-trip mismatch: got %q", got)
	}
}

func TestECBDecrypt_RoundTripLarge(t *testing.T) {
	plain := make([]byte, 256)
	for i := range plain {
		plain[i] = byte(i)
	}
	ct, _ := ECBEncrypt(plain, ecbKey)
	got, _ := ECBDecrypt(ct, ecbKey)
	if !bytes.Equal(got, plain) {
		t.Error("large round-trip mismatch")
	}
}

func TestECBDecrypt_InvalidLengthNotMultipleOf8(t *testing.T) {
	_, err := ECBDecrypt(make([]byte, 7), ecbKey)
	if err == nil {
		t.Error("expected error for non-multiple-of-8 ciphertext")
	}
}

func TestECBDecrypt_InvalidEmpty(t *testing.T) {
	_, err := ECBDecrypt([]byte{}, ecbKey)
	if err == nil {
		t.Error("expected error for empty ciphertext")
	}
}

func TestECBDecrypt_BadPaddingRaises(t *testing.T) {
	// Corrupt the last byte of the ciphertext to generate invalid padding.
	plain := []byte("test data")
	ct, _ := ECBEncrypt(plain, ecbKey)
	ct[len(ct)-1] ^= 0xFF
	_, err := ECBDecrypt(ct, ecbKey)
	if err == nil {
		t.Error("expected error for corrupted ciphertext padding")
	}
}

// =============================================================================
// Triple DES (TDEA)
// =============================================================================

var (
	tdeaK1    = h("0123456789ABCDEF")
	tdeaK2    = h("23456789ABCDEF01")
	tdeaK3    = h("456789ABCDEF0123")
	tdeaPlain = h("6BC1BEE22E409F96")
	tdeaCT    = h("3B6423D418DEFC23")
)

func TestTDEAEncryptBlock(t *testing.T) {
	// NIST SP 800-67 EDE ordering: E_K1(D_K2(E_K3(P))).
	ct, err := TDEAEncryptBlock(tdeaPlain, tdeaK1, tdeaK2, tdeaK3)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, tdeaCT) {
		t.Errorf("TDEAEncryptBlock = %X, want %X", ct, tdeaCT)
	}
}

func TestTDEADecryptBlock(t *testing.T) {
	// D_K3(E_K2(D_K1(C))) — inverse of EDE.
	pt, err := TDEADecryptBlock(tdeaCT, tdeaK1, tdeaK2, tdeaK3)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, tdeaPlain) {
		t.Errorf("TDEADecryptBlock = %X, want %X", pt, tdeaPlain)
	}
}

func TestTDEARoundTripRandomKeys(t *testing.T) {
	k1 := h("FEDCBA9876543210")
	k2 := h("0F1E2D3C4B5A6978")
	k3 := h("7869584A3B2C1D0E")
	plain := h("0123456789ABCDEF")
	ct, err := TDEAEncryptBlock(plain, k1, k2, k3)
	if err != nil {
		t.Fatal(err)
	}
	got, err := TDEADecryptBlock(ct, k1, k2, k3)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(got, plain) {
		t.Errorf("round-trip mismatch: got %X", got)
	}
}

func TestTDEA_BackwardCompat_K1EqK2EqK3(t *testing.T) {
	// When K1=K2=K3, 3DES EDE reduces to single DES:
	//   E(K, D(K, E(K, P))) = E(K, P)   since D(K, E(K, x)) = x
	key := h("0133457799BBCDFF")
	plain := h("0123456789ABCDEF")
	tdeaCT, err := TDEAEncryptBlock(plain, key, key, key)
	if err != nil {
		t.Fatal(err)
	}
	desCT, err := EncryptBlock(plain, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(tdeaCT, desCT) {
		t.Errorf("3DES(K,K,K) ≠ DES(K): got %X, want %X", tdeaCT, desCT)
	}
}

func TestTDEADecrypt_BackwardCompat(t *testing.T) {
	key := h("FEDCBA9876543210")
	ct := h("0123456789ABCDEF")
	tdeaPT, err := TDEADecryptBlock(ct, key, key, key)
	if err != nil {
		t.Fatal(err)
	}
	desPT, err := DecryptBlock(ct, key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(tdeaPT, desPT) {
		t.Errorf("3DES decrypt(K,K,K) ≠ DES decrypt(K)")
	}
}

func TestTDEARoundTripAllSameBlock(t *testing.T) {
	k1 := h("1234567890ABCDEF")
	k2 := h("FEDCBA0987654321")
	k3 := h("0F0F0F0F0F0F0F0F")
	for _, val := range []byte{0x00, 0xFF, 0xA5, 0x5A} {
		plain := bytes.Repeat([]byte{val}, 8)
		ct, err := TDEAEncryptBlock(plain, k1, k2, k3)
		if err != nil {
			t.Fatal(err)
		}
		got, err := TDEADecryptBlock(ct, k1, k2, k3)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(got, plain) {
			t.Errorf("val=0x%02X: round-trip mismatch", val)
		}
	}
}

// =============================================================================
// Invalid input handling
// =============================================================================

func TestEncryptBlock_WrongBlockSize_Short(t *testing.T) {
	_, err := EncryptBlock(make([]byte, 7), ecbKey)
	if err == nil {
		t.Error("expected error for 7-byte block")
	}
}

func TestEncryptBlock_WrongBlockSize_Long(t *testing.T) {
	_, err := EncryptBlock(make([]byte, 16), ecbKey)
	if err == nil {
		t.Error("expected error for 16-byte block")
	}
}

func TestDecryptBlock_WrongBlockSize(t *testing.T) {
	_, err := DecryptBlock(make([]byte, 9), ecbKey)
	if err == nil {
		t.Error("expected error for 9-byte block")
	}
}

func TestEncryptBlock_WrongKeySize(t *testing.T) {
	_, err := EncryptBlock(make([]byte, 8), make([]byte, 4))
	if err == nil {
		t.Error("expected error for 4-byte key")
	}
}

func TestDecryptBlock_WrongKeySize(t *testing.T) {
	_, err := DecryptBlock(make([]byte, 8), make([]byte, 16))
	if err == nil {
		t.Error("expected error for 16-byte key")
	}
}

func TestECBEncrypt_WrongKeySize(t *testing.T) {
	_, err := ECBEncrypt([]byte("hello"), make([]byte, 4))
	if err == nil {
		t.Error("expected error for 4-byte key")
	}
}

func TestECBDecrypt_WrongKeySize(t *testing.T) {
	_, err := ECBDecrypt(make([]byte, 8), make([]byte, 4))
	if err == nil {
		t.Error("expected error for 4-byte key")
	}
}
