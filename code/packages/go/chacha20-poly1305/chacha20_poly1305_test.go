package chacha20poly1305

import (
	"bytes"
	"encoding/hex"
	"testing"
)

// ===================================================================
// Helper
// ===================================================================

func mustHex(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic(err)
	}
	return b
}

// ===================================================================
// RFC 8439 Test Vectors
// ===================================================================

// ChaCha20 (Section 2.4.2)
var (
	chacha20Key   = mustHex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
	chacha20Nonce = mustHex("000000000000004a00000000")
	chacha20PT    = []byte("Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.")
	chacha20CT    = mustHex(
		"6e2e359a2568f98041ba0728dd0d6981" +
			"e97e7aec1d4360c20a27afccfd9fae0b" +
			"f91b65c5524733ab8f593dabcd62b357" +
			"1639d624e65152ab8f530c359f0861d8" +
			"07ca0dbf500d6a6156a38e088a22b65e" +
			"52bc514d16ccf806818ce91ab7793736" +
			"5af90bbf74a35be6b40b8eedf2785e42" +
			"874d")
)

// Poly1305 (Section 2.5.2)
var (
	poly1305Key = mustHex("85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b")
	poly1305Msg = []byte("Cryptographic Forum Research Group")
	poly1305Tag = mustHex("a8061dc1305136c6c22b8baf0c0127a9")
)

// AEAD (Section 2.8.2)
var (
	aeadKey   = mustHex("808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f")
	aeadNonce = mustHex("070000004041424344454647")
	aeadAAD   = mustHex("50515253c0c1c2c3c4c5c6c7")
	aeadPT    = []byte("Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.")
	aeadCT    = mustHex(
		"d31a8d34648e60db7b86afbc53ef7ec2" +
			"a4aded51296e08fea9e2b5a736ee62d6" +
			"3dbea45e8ca9671282fafb69da92728b" +
			"1a71de0a9e060b2905d6a5b67ecd3b36" +
			"92ddbd7f2d778b8c9803aee328091b58" +
			"fab324e4fad675945585808b4831d7bc" +
			"3ff4def08e4b7a9de576d26586cec64b" +
			"6116")
	aeadTag = mustHex("1ae10b594f09e26a7e902ecbd0600691")
)

// ===================================================================
// Low-level Tests
// ===================================================================

func TestRotl32(t *testing.T) {
	tests := []struct {
		value    uint32
		shift    uint
		expected uint32
	}{
		{0xAABBCCDD, 16, 0xCCDDAABB},
		{0x12345678, 0, 0x12345678},
		{0x80000000, 1, 0x00000001},
		{0x00000001, 7, 0x00000080},
	}
	for _, tc := range tests {
		got := rotl32(tc.value, tc.shift)
		if got != tc.expected {
			t.Errorf("rotl32(0x%08x, %d) = 0x%08x, want 0x%08x",
				tc.value, tc.shift, got, tc.expected)
		}
	}
}

func TestQuarterRound(t *testing.T) {
	// RFC 8439 Section 2.1.1 quarter round test vector
	state := [16]uint32{
		0x879531e0, 0xc5ecf37d, 0x516461b1, 0xc9a62f8a,
		0x44c20ef3, 0x3390af7f, 0xd9fc690b, 0x2a5f714c,
		0x53372767, 0xb00a5631, 0x974c541a, 0x359e9963,
		0x5c971061, 0x3d631689, 0x2098d9d6, 0x91dbd320,
	}
	quarterRound(&state, 2, 7, 8, 13)

	if state[2] != 0xbdb886dc {
		t.Errorf("state[2] = 0x%08x, want 0xbdb886dc", state[2])
	}
	if state[7] != 0xcfacafd2 {
		t.Errorf("state[7] = 0x%08x, want 0xcfacafd2", state[7])
	}
	if state[8] != 0xe46bea80 {
		t.Errorf("state[8] = 0x%08x, want 0xe46bea80", state[8])
	}
	if state[13] != 0xccc07c79 {
		t.Errorf("state[13] = 0x%08x, want 0xccc07c79", state[13])
	}
}

func TestPad16(t *testing.T) {
	tests := []struct {
		inputLen    int
		expectedLen int
	}{
		{0, 0},
		{1, 15},
		{10, 6},
		{16, 0},
		{17, 15},
		{32, 0},
	}
	for _, tc := range tests {
		p := pad16(make([]byte, tc.inputLen))
		if len(p) != tc.expectedLen {
			t.Errorf("pad16(len=%d) produced %d bytes, want %d",
				tc.inputLen, len(p), tc.expectedLen)
		}
	}
}

func TestConstantTimeCompare(t *testing.T) {
	if !constantTimeCompare([]byte("hello"), []byte("hello")) {
		t.Error("equal slices should compare as equal")
	}
	if constantTimeCompare([]byte("hello"), []byte("world")) {
		t.Error("different slices should compare as not equal")
	}
	if constantTimeCompare([]byte("hello"), []byte("hell")) {
		t.Error("different length slices should compare as not equal")
	}
	if !constantTimeCompare([]byte{}, []byte{}) {
		t.Error("empty slices should compare as equal")
	}
}

// ===================================================================
// ChaCha20 Tests
// ===================================================================

func TestChaCha20Block(t *testing.T) {
	// RFC 8439 Section 2.3.2
	key := mustHex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
	nonce := mustHex("000000090000004a00000000")
	block := chacha20Block(key, 1, nonce)

	// Verify first 4 bytes
	expected := mustHex("10f1e7e4")
	if !bytes.Equal(block[:4], expected) {
		t.Errorf("first 4 bytes = %x, want %x", block[:4], expected)
	}
	if len(block) != 64 {
		t.Errorf("block length = %d, want 64", len(block))
	}
}

func TestChaCha20EncryptRFC(t *testing.T) {
	ct, err := ChaCha20Encrypt(chacha20PT, chacha20Key, chacha20Nonce, 1)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, chacha20CT) {
		t.Errorf("ciphertext mismatch:\ngot:  %x\nwant: %x", ct, chacha20CT)
	}
}

func TestChaCha20DecryptIsEncrypt(t *testing.T) {
	ct, err := ChaCha20Encrypt(chacha20PT, chacha20Key, chacha20Nonce, 1)
	if err != nil {
		t.Fatal(err)
	}
	pt, err := ChaCha20Encrypt(ct, chacha20Key, chacha20Nonce, 1)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, chacha20PT) {
		t.Error("decrypt(encrypt(pt)) != pt")
	}
}

func TestChaCha20EmptyPlaintext(t *testing.T) {
	ct, err := ChaCha20Encrypt([]byte{}, chacha20Key, chacha20Nonce, 0)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 0 {
		t.Errorf("expected empty ciphertext, got %d bytes", len(ct))
	}
}

func TestChaCha20SingleByte(t *testing.T) {
	ct, err := ChaCha20Encrypt([]byte{0x00}, chacha20Key, chacha20Nonce, 0)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 1 {
		t.Fatalf("expected 1 byte, got %d", len(ct))
	}
	pt, _ := ChaCha20Encrypt(ct, chacha20Key, chacha20Nonce, 0)
	if !bytes.Equal(pt, []byte{0x00}) {
		t.Error("roundtrip failed for single byte")
	}
}

func TestChaCha20MultiBlock(t *testing.T) {
	data := make([]byte, 512)
	for i := range data {
		data[i] = byte(i % 256)
	}
	ct, err := ChaCha20Encrypt(data, chacha20Key, chacha20Nonce, 0)
	if err != nil {
		t.Fatal(err)
	}
	pt, _ := ChaCha20Encrypt(ct, chacha20Key, chacha20Nonce, 0)
	if !bytes.Equal(pt, data) {
		t.Error("roundtrip failed for multi-block data")
	}
}

func TestChaCha20InvalidKeyLength(t *testing.T) {
	_, err := ChaCha20Encrypt([]byte("hello"), []byte("short"), chacha20Nonce, 0)
	if err == nil {
		t.Error("expected error for invalid key length")
	}
}

func TestChaCha20InvalidNonceLength(t *testing.T) {
	_, err := ChaCha20Encrypt([]byte("hello"), chacha20Key, []byte("short"), 0)
	if err == nil {
		t.Error("expected error for invalid nonce length")
	}
}

// ===================================================================
// Poly1305 Tests
// ===================================================================

func TestPoly1305RFC(t *testing.T) {
	tag, err := Poly1305Mac(poly1305Msg, poly1305Key)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(tag, poly1305Tag) {
		t.Errorf("tag mismatch:\ngot:  %x\nwant: %x", tag, poly1305Tag)
	}
}

func TestPoly1305EmptyMessage(t *testing.T) {
	tag, err := Poly1305Mac([]byte{}, poly1305Key)
	if err != nil {
		t.Fatal(err)
	}
	if len(tag) != 16 {
		t.Errorf("tag length = %d, want 16", len(tag))
	}
}

func TestPoly1305SingleByte(t *testing.T) {
	tag, err := Poly1305Mac([]byte{0x00}, poly1305Key)
	if err != nil {
		t.Fatal(err)
	}
	if len(tag) != 16 {
		t.Errorf("tag length = %d, want 16", len(tag))
	}
}

func TestPoly1305DifferentMessages(t *testing.T) {
	tag1, _ := Poly1305Mac([]byte("hello"), poly1305Key)
	tag2, _ := Poly1305Mac([]byte("world"), poly1305Key)
	if bytes.Equal(tag1, tag2) {
		t.Error("different messages should produce different tags")
	}
}

func TestPoly1305DifferentKeys(t *testing.T) {
	key2 := make([]byte, 32)
	for i := range key2 {
		key2[i] = byte(i)
	}
	tag1, _ := Poly1305Mac([]byte("hello"), poly1305Key)
	tag2, _ := Poly1305Mac([]byte("hello"), key2)
	if bytes.Equal(tag1, tag2) {
		t.Error("different keys should produce different tags")
	}
}

func TestPoly1305InvalidKeyLength(t *testing.T) {
	_, err := Poly1305Mac([]byte("hello"), []byte("short"))
	if err == nil {
		t.Error("expected error for invalid key length")
	}
}

// ===================================================================
// AEAD Tests
// ===================================================================

func TestAEADEncryptRFC(t *testing.T) {
	ct, tag, err := AEADEncrypt(aeadPT, aeadKey, aeadNonce, aeadAAD)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(ct, aeadCT) {
		t.Errorf("ciphertext mismatch:\ngot:  %x\nwant: %x", ct, aeadCT)
	}
	if !bytes.Equal(tag, aeadTag) {
		t.Errorf("tag mismatch:\ngot:  %x\nwant: %x", tag, aeadTag)
	}
}

func TestAEADDecryptRFC(t *testing.T) {
	pt, err := AEADDecrypt(aeadCT, aeadKey, aeadNonce, aeadAAD, aeadTag)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, aeadPT) {
		t.Errorf("plaintext mismatch:\ngot:  %s\nwant: %s", pt, aeadPT)
	}
}

func TestAEADRoundtrip(t *testing.T) {
	key := make([]byte, 32)
	nonce := make([]byte, 12)
	for i := range key {
		key[i] = byte(i)
	}
	for i := range nonce {
		nonce[i] = byte(i)
	}
	plaintext := []byte("Hello, ChaCha20-Poly1305!")
	aad := []byte("additional data")

	ct, tag, err := AEADEncrypt(plaintext, key, nonce, aad)
	if err != nil {
		t.Fatal(err)
	}
	pt, err := AEADDecrypt(ct, key, nonce, aad, tag)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, plaintext) {
		t.Error("roundtrip failed")
	}
}

func TestAEADEmptyPlaintext(t *testing.T) {
	key := make([]byte, 32)
	nonce := make([]byte, 12)
	aad := []byte("authenticate only")

	ct, tag, err := AEADEncrypt([]byte{}, key, nonce, aad)
	if err != nil {
		t.Fatal(err)
	}
	if len(ct) != 0 {
		t.Errorf("expected empty ciphertext, got %d bytes", len(ct))
	}
	if len(tag) != 16 {
		t.Errorf("tag length = %d, want 16", len(tag))
	}
	pt, err := AEADDecrypt(ct, key, nonce, aad, tag)
	if err != nil {
		t.Fatal(err)
	}
	if len(pt) != 0 {
		t.Errorf("expected empty plaintext, got %d bytes", len(pt))
	}
}

func TestAEADEmptyAAD(t *testing.T) {
	key := make([]byte, 32)
	nonce := make([]byte, 12)

	ct, tag, err := AEADEncrypt([]byte("secret"), key, nonce, []byte{})
	if err != nil {
		t.Fatal(err)
	}
	pt, err := AEADDecrypt(ct, key, nonce, []byte{}, tag)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, []byte("secret")) {
		t.Error("roundtrip with empty AAD failed")
	}
}

func TestAEADTamperedCiphertext(t *testing.T) {
	key := make([]byte, 32)
	nonce := make([]byte, 12)
	aad := []byte("aad")

	ct, tag, _ := AEADEncrypt([]byte("secret message"), key, nonce, aad)

	// Flip one bit
	tampered := make([]byte, len(ct))
	copy(tampered, ct)
	tampered[0] ^= 0x01

	_, err := AEADDecrypt(tampered, key, nonce, aad, tag)
	if err == nil {
		t.Error("expected authentication failure for tampered ciphertext")
	}
	if err != ErrAuthFailed {
		t.Errorf("expected ErrAuthFailed, got %v", err)
	}
}

func TestAEADTamperedTag(t *testing.T) {
	key := make([]byte, 32)
	nonce := make([]byte, 12)

	ct, _, _ := AEADEncrypt([]byte("secret"), key, nonce, []byte{})

	badTag := make([]byte, 16)
	_, err := AEADDecrypt(ct, key, nonce, []byte{}, badTag)
	if err != ErrAuthFailed {
		t.Errorf("expected ErrAuthFailed, got %v", err)
	}
}

func TestAEADWrongAAD(t *testing.T) {
	key := make([]byte, 32)
	nonce := make([]byte, 12)

	ct, tag, _ := AEADEncrypt([]byte("secret"), key, nonce, []byte("correct aad"))

	_, err := AEADDecrypt(ct, key, nonce, []byte("wrong aad"), tag)
	if err != ErrAuthFailed {
		t.Errorf("expected ErrAuthFailed, got %v", err)
	}
}

func TestAEADWrongKey(t *testing.T) {
	key1 := make([]byte, 32)
	key2 := make([]byte, 32)
	key2[0] = 1
	nonce := make([]byte, 12)

	ct, tag, _ := AEADEncrypt([]byte("secret"), key1, nonce, []byte{})

	_, err := AEADDecrypt(ct, key2, nonce, []byte{}, tag)
	if err != ErrAuthFailed {
		t.Errorf("expected ErrAuthFailed, got %v", err)
	}
}

func TestAEADWrongNonce(t *testing.T) {
	key := make([]byte, 32)
	nonce1 := make([]byte, 12)
	nonce2 := make([]byte, 12)
	nonce2[0] = 1

	ct, tag, _ := AEADEncrypt([]byte("secret"), key, nonce1, []byte{})

	_, err := AEADDecrypt(ct, key, nonce2, []byte{}, tag)
	if err != ErrAuthFailed {
		t.Errorf("expected ErrAuthFailed, got %v", err)
	}
}

func TestAEADInvalidKeyLength(t *testing.T) {
	_, _, err := AEADEncrypt([]byte("hello"), []byte("short"), make([]byte, 12), nil)
	if err == nil {
		t.Error("expected error for invalid key length")
	}
}

func TestAEADInvalidNonceLength(t *testing.T) {
	_, _, err := AEADEncrypt([]byte("hello"), make([]byte, 32), []byte("short"), nil)
	if err == nil {
		t.Error("expected error for invalid nonce length")
	}
}

func TestAEADInvalidTagLength(t *testing.T) {
	_, err := AEADDecrypt([]byte("hello"), make([]byte, 32), make([]byte, 12), nil, []byte("short"))
	if err == nil {
		t.Error("expected error for invalid tag length")
	}
}

func TestAEADLargePlaintext(t *testing.T) {
	key := make([]byte, 32)
	nonce := make([]byte, 12)
	plaintext := bytes.Repeat([]byte("A"), 1024)

	ct, tag, err := AEADEncrypt(plaintext, key, nonce, []byte{})
	if err != nil {
		t.Fatal(err)
	}
	pt, err := AEADDecrypt(ct, key, nonce, []byte{}, tag)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(pt, plaintext) {
		t.Error("large plaintext roundtrip failed")
	}
}
