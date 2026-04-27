package aesmodes

import (
	"bytes"
	"encoding/hex"
	"testing"
)

// =============================================================================
// Helper: decode hex string to bytes (panics on failure for test clarity)
// =============================================================================

func hexDecode(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic("invalid hex in test: " + s)
	}
	return b
}

// =============================================================================
// NIST SP 800-38A Test Vectors
// =============================================================================

var (
	nistKey = hexDecode("2b7e151628aed2a6abf7158809cf4f3c")

	nistPlaintextBlocks = [][]byte{
		hexDecode("6bc1bee22e409f96e93d7e117393172a"),
		hexDecode("ae2d8a571e03ac9c9eb76fac45af8e51"),
		hexDecode("30c81c46a35ce411e5fbc1191a0a52ef"),
		hexDecode("f69f2445df4f9b17ad2b417be66c3710"),
	}
)

// =============================================================================
// PKCS#7 Padding Tests
// =============================================================================

func TestPKCS7PadShort(t *testing.T) {
	result := pkcs7Pad([]byte("hello")) // 5 bytes
	if len(result) != 16 {
		t.Fatalf("expected length 16, got %d", len(result))
	}
	for i := 5; i < 16; i++ {
		if result[i] != 11 {
			t.Fatalf("padding byte at %d: expected 11, got %d", i, result[i])
		}
	}
}

func TestPKCS7PadAligned(t *testing.T) {
	data := []byte("0123456789abcdef") // 16 bytes
	result := pkcs7Pad(data)
	if len(result) != 32 {
		t.Fatalf("expected length 32, got %d", len(result))
	}
	for i := 16; i < 32; i++ {
		if result[i] != 16 {
			t.Fatalf("padding byte at %d: expected 16, got %d", i, result[i])
		}
	}
}

func TestPKCS7Roundtrip(t *testing.T) {
	for length := 0; length <= 33; length++ {
		data := make([]byte, length)
		for i := range data {
			data[i] = byte(i % 256)
		}
		padded := pkcs7Pad(data)
		unpadded, err := pkcs7Unpad(padded)
		if err != nil {
			t.Fatalf("length %d: unpad error: %v", length, err)
		}
		if !bytes.Equal(unpadded, data) {
			t.Fatalf("length %d: roundtrip mismatch", length)
		}
	}
}

func TestPKCS7UnpadInvalid(t *testing.T) {
	_, err := pkcs7Unpad([]byte{})
	if err == nil {
		t.Fatal("expected error for empty data")
	}
	_, err = pkcs7Unpad([]byte("hello"))
	if err == nil {
		t.Fatal("expected error for non-aligned data")
	}
}

// =============================================================================
// ECB Mode Tests
// =============================================================================

func TestECBSingleBlock(t *testing.T) {
	expected := hexDecode("3ad77bb40d7a3660a89ecaf32466ef97")
	ct, err := EncryptECB(nistPlaintextBlocks[0], nistKey)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct[:16], expected) {
		t.Fatalf("ECB single block mismatch:\n  got  %x\n  want %x", ct[:16], expected)
	}
}

func TestECBRoundtrip(t *testing.T) {
	var plaintext []byte
	for _, b := range nistPlaintextBlocks {
		plaintext = append(plaintext, b...)
	}
	ct, err := EncryptECB(plaintext, nistKey)
	if err != nil {
		t.Fatal(err)
	}
	pt, err := DecryptECB(ct, nistKey)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, plaintext) {
		t.Fatal("ECB roundtrip mismatch")
	}
}

func TestECBIdenticalBlocks(t *testing.T) {
	block := bytes.Repeat([]byte{0x41}, 16)
	plaintext := bytes.Repeat(block, 3)
	ct, err := EncryptECB(plaintext, nistKey)
	if err != nil {
		t.Fatal(err)
	}
	// All three blocks should be identical (ECB's fatal flaw)
	if !bytes.Equal(ct[0:16], ct[16:32]) || !bytes.Equal(ct[16:32], ct[32:48]) {
		t.Fatal("ECB identical blocks should produce identical ciphertext")
	}
}

func TestECBEmpty(t *testing.T) {
	ct, err := EncryptECB([]byte{}, nistKey)
	if err != nil {
		t.Fatal(err)
	}
	pt, err := DecryptECB(ct, nistKey)
	if err != nil {
		t.Fatal(err)
	}
	if len(pt) != 0 {
		t.Fatal("expected empty plaintext")
	}
}

func TestECBDecryptInvalidLength(t *testing.T) {
	_, err := DecryptECB([]byte("short"), nistKey)
	if err == nil {
		t.Fatal("expected error for invalid ciphertext length")
	}
}

func TestECBVariousLengths(t *testing.T) {
	for _, length := range []int{1, 15, 16, 17, 31, 32, 48, 100} {
		plaintext := make([]byte, length)
		for i := range plaintext {
			plaintext[i] = byte(i % 256)
		}
		ct, err := EncryptECB(plaintext, nistKey)
		if err != nil {
			t.Fatalf("length %d: encrypt error: %v", length, err)
		}
		pt, err := DecryptECB(ct, nistKey)
		if err != nil {
			t.Fatalf("length %d: decrypt error: %v", length, err)
		}
		if !bytes.Equal(pt, plaintext) {
			t.Fatalf("length %d: roundtrip mismatch", length)
		}
	}
}

// =============================================================================
// CBC Mode Tests
// =============================================================================

var (
	cbcIV = hexDecode("000102030405060708090a0b0c0d0e0f")

	cbcCiphertextBlocks = [][]byte{
		hexDecode("7649abac8119b246cee98e9b12e9197d"),
		hexDecode("5086cb9b507219ee95db113a917678b2"),
		hexDecode("73bed6b8e3c1743b7116e69e22229516"),
		hexDecode("3ff1caa1681fac09120eca307586e1a7"),
	}
)

func TestCBCSingleBlock(t *testing.T) {
	ct, err := EncryptCBC(nistPlaintextBlocks[0], nistKey, cbcIV)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct[:16], cbcCiphertextBlocks[0]) {
		t.Fatalf("CBC single block mismatch:\n  got  %x\n  want %x", ct[:16], cbcCiphertextBlocks[0])
	}
}

func TestCBCAllNISTBlocks(t *testing.T) {
	var plaintext []byte
	for _, b := range nistPlaintextBlocks {
		plaintext = append(plaintext, b...)
	}
	ct, err := EncryptCBC(plaintext, nistKey, cbcIV)
	if err != nil {
		t.Fatal(err)
	}
	for i, expected := range cbcCiphertextBlocks {
		actual := ct[i*16 : (i+1)*16]
		if !bytes.Equal(actual, expected) {
			t.Fatalf("CBC block %d mismatch:\n  got  %x\n  want %x", i, actual, expected)
		}
	}
}

func TestCBCRoundtrip(t *testing.T) {
	var plaintext []byte
	for _, b := range nistPlaintextBlocks {
		plaintext = append(plaintext, b...)
	}
	ct, err := EncryptCBC(plaintext, nistKey, cbcIV)
	if err != nil {
		t.Fatal(err)
	}
	pt, err := DecryptCBC(ct, nistKey, cbcIV)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, plaintext) {
		t.Fatal("CBC roundtrip mismatch")
	}
}

func TestCBCDifferentIV(t *testing.T) {
	plaintext := bytes.Repeat([]byte{0x41}, 16)
	iv1 := make([]byte, 16)
	iv2 := bytes.Repeat([]byte{1}, 16)
	ct1, _ := EncryptCBC(plaintext, nistKey, iv1)
	ct2, _ := EncryptCBC(plaintext, nistKey, iv2)
	if bytes.Equal(ct1, ct2) {
		t.Fatal("different IVs should produce different ciphertexts")
	}
}

func TestCBCInvalidIV(t *testing.T) {
	_, err := EncryptCBC([]byte("test"), nistKey, []byte("short"))
	if err == nil {
		t.Fatal("expected error for invalid IV length")
	}
	_, err = DecryptCBC(make([]byte, 16), nistKey, []byte("short"))
	if err == nil {
		t.Fatal("expected error for invalid IV length on decrypt")
	}
}

func TestCBCEmpty(t *testing.T) {
	iv := make([]byte, 16)
	ct, err := EncryptCBC([]byte{}, nistKey, iv)
	if err != nil {
		t.Fatal(err)
	}
	pt, err := DecryptCBC(ct, nistKey, iv)
	if err != nil {
		t.Fatal(err)
	}
	if len(pt) != 0 {
		t.Fatal("expected empty plaintext")
	}
}

func TestCBCVariousLengths(t *testing.T) {
	iv := make([]byte, 16)
	for _, length := range []int{1, 15, 16, 17, 31, 32, 48, 100} {
		plaintext := make([]byte, length)
		for i := range plaintext {
			plaintext[i] = byte(i % 256)
		}
		ct, err := EncryptCBC(plaintext, nistKey, iv)
		if err != nil {
			t.Fatalf("length %d: encrypt error: %v", length, err)
		}
		pt, err := DecryptCBC(ct, nistKey, iv)
		if err != nil {
			t.Fatalf("length %d: decrypt error: %v", length, err)
		}
		if !bytes.Equal(pt, plaintext) {
			t.Fatalf("length %d: roundtrip mismatch", length)
		}
	}
}

// =============================================================================
// CTR Mode Tests
// =============================================================================

func TestCTRRoundtrip(t *testing.T) {
	nonce := make([]byte, 12)
	plaintext := []byte("Hello, CTR mode! This is a test of counter mode encryption.")
	ct, err := EncryptCTR(plaintext, nistKey, nonce)
	if err != nil {
		t.Fatal(err)
	}
	pt, err := DecryptCTR(ct, nistKey, nonce)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, plaintext) {
		t.Fatal("CTR roundtrip mismatch")
	}
}

func TestCTRSameLength(t *testing.T) {
	nonce := make([]byte, 12)
	for _, length := range []int{1, 5, 15, 16, 17, 31, 32, 100} {
		plaintext := bytes.Repeat([]byte{0x41}, length)
		ct, err := EncryptCTR(plaintext, nistKey, nonce)
		if err != nil {
			t.Fatalf("length %d: %v", length, err)
		}
		if len(ct) != length {
			t.Fatalf("length %d: ciphertext length %d != plaintext length %d", length, len(ct), length)
		}
	}
}

func TestCTRNonceReuseAttack(t *testing.T) {
	nonce := make([]byte, 12)
	p1 := []byte("Attack at dawn!!")
	p2 := []byte("Attack at dusk!!")
	c1, _ := EncryptCTR(p1, nistKey, nonce)
	c2, _ := EncryptCTR(p2, nistKey, nonce)

	ctXOR := xorBytes(c1, c2)
	ptXOR := xorBytes(p1, p2)
	if !bytes.Equal(ctXOR, ptXOR) {
		t.Fatal("nonce reuse: C1 XOR C2 should equal P1 XOR P2")
	}
}

func TestCTRInvalidNonce(t *testing.T) {
	_, err := EncryptCTR([]byte("test"), nistKey, []byte("short"))
	if err == nil {
		t.Fatal("expected error for invalid nonce length")
	}
}

func TestCTREmpty(t *testing.T) {
	nonce := make([]byte, 12)
	ct, err := EncryptCTR([]byte{}, nistKey, nonce)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 0 {
		t.Fatal("expected empty ciphertext")
	}
}

func TestCTRDecryptIsEncrypt(t *testing.T) {
	nonce := make([]byte, 12)
	plaintext := []byte("Symmetric!")
	ct, _ := EncryptCTR(plaintext, nistKey, nonce)
	// Encrypting the ciphertext should produce the plaintext (XOR self-inverse)
	pt, _ := EncryptCTR(ct, nistKey, nonce)
	if !bytes.Equal(pt, plaintext) {
		t.Fatal("CTR: encrypt(encrypt(P)) should equal P")
	}
}

// =============================================================================
// GCM Mode Tests
// =============================================================================

var (
	gcmKey       = hexDecode("feffe9928665731c6d6a8f9467308308")
	gcmIV        = hexDecode("cafebabefacedbaddecaf888")
	gcmPlaintext = hexDecode(
		"d9313225f88406e5a55909c5aff5269a" +
			"86a7a9531534f7da2e4c303d8a318a72" +
			"1c3c0c95956809532fcf0e2449a6b525" +
			"b16aedf5aa0de657ba637b391aafd255")
	gcmCiphertext = hexDecode(
		"42831ec2217774244b7221b784d0d49c" +
			"e3aa212f2c02a4e035c17e2329aca12e" +
			"21d514b25466931c7d8f6a5aac84aa05" +
			"1ba30b396a0aac973d58e091473f5985")
	gcmTag = hexDecode("4d5c2af327cd64a62cf35abd2ba6fab4")

	// Test Case 4 (with AAD)
	gcmAADTC4       = hexDecode("feedfacedeadbeeffeedfacedeadbeefabaddad2")
	gcmPlaintextTC4 = hexDecode(
		"d9313225f88406e5a55909c5aff5269a" +
			"86a7a9531534f7da2e4c303d8a318a72" +
			"1c3c0c95956809532fcf0e2449a6b525" +
			"b16aedf5aa0de657ba637b39")
	gcmCiphertextTC4 = hexDecode(
		"42831ec2217774244b7221b784d0d49c" +
			"e3aa212f2c02a4e035c17e2329aca12e" +
			"21d514b25466931c7d8f6a5aac84aa05" +
			"1ba30b396a0aac973d58e091")
	gcmTagTC4 = hexDecode("5bc94fbc3221a5db94fae95ae7121a47")
)

func TestGCMEncryptNISTTC3(t *testing.T) {
	ct, tag, err := EncryptGCM(gcmPlaintext, gcmKey, gcmIV, nil)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, gcmCiphertext) {
		t.Fatalf("GCM TC3 ciphertext mismatch:\n  got  %x\n  want %x", ct, gcmCiphertext)
	}
	if !bytes.Equal(tag, gcmTag) {
		t.Fatalf("GCM TC3 tag mismatch:\n  got  %x\n  want %x", tag, gcmTag)
	}
}

func TestGCMDecryptNISTTC3(t *testing.T) {
	pt, err := DecryptGCM(gcmCiphertext, gcmKey, gcmIV, nil, gcmTag)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, gcmPlaintext) {
		t.Fatal("GCM TC3 decrypt mismatch")
	}
}

func TestGCMEncryptNISTTC4(t *testing.T) {
	ct, tag, err := EncryptGCM(gcmPlaintextTC4, gcmKey, gcmIV, gcmAADTC4)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, gcmCiphertextTC4) {
		t.Fatalf("GCM TC4 ciphertext mismatch:\n  got  %x\n  want %x", ct, gcmCiphertextTC4)
	}
	if !bytes.Equal(tag, gcmTagTC4) {
		t.Fatalf("GCM TC4 tag mismatch:\n  got  %x\n  want %x", tag, gcmTagTC4)
	}
}

func TestGCMDecryptNISTTC4(t *testing.T) {
	pt, err := DecryptGCM(gcmCiphertextTC4, gcmKey, gcmIV, gcmAADTC4, gcmTagTC4)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, gcmPlaintextTC4) {
		t.Fatal("GCM TC4 decrypt mismatch")
	}
}

func TestGCMRoundtrip(t *testing.T) {
	plaintext := []byte("Hello, GCM! This is authenticated encryption.")
	aad := []byte("additional data")
	ct, tag, err := EncryptGCM(plaintext, gcmKey, gcmIV, aad)
	if err != nil {
		t.Fatal(err)
	}
	pt, err := DecryptGCM(ct, gcmKey, gcmIV, aad, tag)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, plaintext) {
		t.Fatal("GCM roundtrip mismatch")
	}
}

func TestGCMTamperedCiphertext(t *testing.T) {
	plaintext := []byte("Secret message!")
	ct, tag, _ := EncryptGCM(plaintext, gcmKey, gcmIV, nil)
	tampered := make([]byte, len(ct))
	copy(tampered, ct)
	tampered[0] ^= 1
	_, err := DecryptGCM(tampered, gcmKey, gcmIV, nil, tag)
	if err == nil {
		t.Fatal("expected authentication failure for tampered ciphertext")
	}
}

func TestGCMTamperedAAD(t *testing.T) {
	plaintext := []byte("Secret message!")
	aad := []byte("metadata")
	ct, tag, _ := EncryptGCM(plaintext, gcmKey, gcmIV, aad)
	_, err := DecryptGCM(ct, gcmKey, gcmIV, []byte("wrong"), tag)
	if err == nil {
		t.Fatal("expected authentication failure for tampered AAD")
	}
}

func TestGCMTamperedTag(t *testing.T) {
	plaintext := []byte("Secret message!")
	ct, tag, _ := EncryptGCM(plaintext, gcmKey, gcmIV, nil)
	badTag := make([]byte, 16)
	copy(badTag, tag)
	badTag[0] ^= 1
	_, err := DecryptGCM(ct, gcmKey, gcmIV, nil, badTag)
	if err == nil {
		t.Fatal("expected authentication failure for tampered tag")
	}
}

func TestGCMEmptyPlaintext(t *testing.T) {
	aad := []byte("authenticate only")
	ct, tag, err := EncryptGCM([]byte{}, gcmKey, gcmIV, aad)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 0 {
		t.Fatal("expected empty ciphertext")
	}
	if len(tag) != 16 {
		t.Fatal("expected 16-byte tag")
	}
	pt, err := DecryptGCM(ct, gcmKey, gcmIV, aad, tag)
	if err != nil {
		t.Fatal(err)
	}
	if len(pt) != 0 {
		t.Fatal("expected empty plaintext")
	}
}

func TestGCMInvalidIV(t *testing.T) {
	_, _, err := EncryptGCM([]byte("test"), gcmKey, []byte("short"), nil)
	if err == nil {
		t.Fatal("expected error for invalid IV length")
	}
}

func TestGCMInvalidTag(t *testing.T) {
	_, err := DecryptGCM([]byte("test"), gcmKey, gcmIV, nil, []byte("short"))
	if err == nil {
		t.Fatal("expected error for invalid tag length")
	}
}
