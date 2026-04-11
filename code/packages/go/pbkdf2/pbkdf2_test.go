package pbkdf2_test

import (
	"encoding/hex"
	"fmt"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/pbkdf2"
)

// mustHex decodes a hex string, panicking on error (test helper).
func mustHex(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic(err)
	}
	return b
}

// ─────────────────────────────────────────────────────────────────────────────
// RFC 6070 test vectors — PBKDF2-HMAC-SHA1
// ─────────────────────────────────────────────────────────────────────────────

func TestRFC6070_SHA1_c1(t *testing.T) {
	// Password: "password", Salt: "salt", c: 1, dkLen: 20
	dk, err := pbkdf2.PBKDF2HmacSHA1([]byte("password"), []byte("salt"), 1, 20)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	want := mustHex("0c60c80f961f0e71f3a9b524af6012062fe037a6")
	if string(dk) != string(want) {
		t.Errorf("got %x, want %x", dk, want)
	}
}

func TestRFC6070_SHA1_c4096(t *testing.T) {
	dk, err := pbkdf2.PBKDF2HmacSHA1([]byte("password"), []byte("salt"), 4096, 20)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	want := mustHex("4b007901b765489abead49d926f721d065a429c1")
	if string(dk) != string(want) {
		t.Errorf("got %x, want %x", dk, want)
	}
}

func TestRFC6070_SHA1_LongPasswordSalt(t *testing.T) {
	dk, err := pbkdf2.PBKDF2HmacSHA1(
		[]byte("passwordPASSWORDpassword"),
		[]byte("saltSALTsaltSALTsaltSALTsaltSALTsalt"),
		4096,
		25,
	)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	want := mustHex("3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038")
	if string(dk) != string(want) {
		t.Errorf("got %x, want %x", dk, want)
	}
}

func TestRFC6070_SHA1_NullBytes(t *testing.T) {
	dk, err := pbkdf2.PBKDF2HmacSHA1([]byte("pass\x00word"), []byte("sa\x00lt"), 4096, 16)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	want := mustHex("56fa6aa75548099dcc37d7f03425e0c3")
	if string(dk) != string(want) {
		t.Errorf("got %x, want %x", dk, want)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// RFC 7914 test vector — PBKDF2-HMAC-SHA256
// ─────────────────────────────────────────────────────────────────────────────

func TestRFC7914_SHA256_c1_64bytes(t *testing.T) {
	dk, err := pbkdf2.PBKDF2HmacSHA256([]byte("passwd"), []byte("salt"), 1, 64)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	want := mustHex(
		"55ac046e56e3089fec1691c22544b605" +
			"f94185216dde0465e68b9d57c20dacbc" +
			"49ca9cccf179b645991664b39d77ef31" +
			"7c71b845b1e30bd509112041d3a19783",
	)
	if string(dk) != string(want) {
		t.Errorf("got %x, want %x", dk, want)
	}
}

func TestSHA256_OutputLength(t *testing.T) {
	dk, err := pbkdf2.PBKDF2HmacSHA256([]byte("key"), []byte("salt"), 1, 32)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(dk) != 32 {
		t.Errorf("expected 32 bytes, got %d", len(dk))
	}
}

func TestSHA256_TruncationConsistency(t *testing.T) {
	// Requesting fewer bytes must equal the prefix of a longer request.
	short, _ := pbkdf2.PBKDF2HmacSHA256([]byte("key"), []byte("salt"), 1, 16)
	full, _ := pbkdf2.PBKDF2HmacSHA256([]byte("key"), []byte("salt"), 1, 32)
	if string(short) != string(full[:16]) {
		t.Errorf("truncation mismatch: %x vs %x", short, full[:16])
	}
}

func TestSHA256_MultiBlock(t *testing.T) {
	// 64 bytes = 2 blocks of 32; first block must match single-block result.
	dk64, _ := pbkdf2.PBKDF2HmacSHA256([]byte("password"), []byte("salt"), 1, 64)
	dk32, _ := pbkdf2.PBKDF2HmacSHA256([]byte("password"), []byte("salt"), 1, 32)
	if len(dk64) != 64 {
		t.Errorf("expected 64 bytes, got %d", len(dk64))
	}
	if string(dk64[:32]) != string(dk32) {
		t.Errorf("block 1 mismatch: %x vs %x", dk64[:32], dk32)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// SHA-512 sanity checks
// ─────────────────────────────────────────────────────────────────────────────

func TestSHA512_OutputLength(t *testing.T) {
	dk, err := pbkdf2.PBKDF2HmacSHA512([]byte("secret"), []byte("nacl"), 1, 64)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(dk) != 64 {
		t.Errorf("expected 64 bytes, got %d", len(dk))
	}
}

func TestSHA512_Truncation(t *testing.T) {
	short, _ := pbkdf2.PBKDF2HmacSHA512([]byte("secret"), []byte("nacl"), 1, 32)
	full, _ := pbkdf2.PBKDF2HmacSHA512([]byte("secret"), []byte("nacl"), 1, 64)
	if string(short) != string(full[:32]) {
		t.Errorf("truncation mismatch")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Hex variants
// ─────────────────────────────────────────────────────────────────────────────

func TestHexVariant_SHA1(t *testing.T) {
	h, err := pbkdf2.PBKDF2HmacSHA1Hex([]byte("password"), []byte("salt"), 1, 20)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if h != "0c60c80f961f0e71f3a9b524af6012062fe037a6" {
		t.Errorf("unexpected hex: %s", h)
	}
}

func TestHexVariant_SHA256MatchesBytes(t *testing.T) {
	dk, _ := pbkdf2.PBKDF2HmacSHA256([]byte("passwd"), []byte("salt"), 1, 32)
	h, _ := pbkdf2.PBKDF2HmacSHA256Hex([]byte("passwd"), []byte("salt"), 1, 32)
	if h != fmt.Sprintf("%x", dk) {
		t.Errorf("hex mismatch: %s vs %x", h, dk)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation / error paths
// ─────────────────────────────────────────────────────────────────────────────

func TestEmptyPassword(t *testing.T) {
	_, err := pbkdf2.PBKDF2HmacSHA256([]byte{}, []byte("salt"), 1, 32)
	if err != pbkdf2.ErrEmptyPassword {
		t.Errorf("expected ErrEmptyPassword, got %v", err)
	}
}

func TestZeroIterations(t *testing.T) {
	_, err := pbkdf2.PBKDF2HmacSHA256([]byte("pw"), []byte("salt"), 0, 32)
	if err != pbkdf2.ErrInvalidIterations {
		t.Errorf("expected ErrInvalidIterations, got %v", err)
	}
}

func TestNegativeIterations(t *testing.T) {
	_, err := pbkdf2.PBKDF2HmacSHA256([]byte("pw"), []byte("salt"), -1, 32)
	if err != pbkdf2.ErrInvalidIterations {
		t.Errorf("expected ErrInvalidIterations, got %v", err)
	}
}

func TestZeroKeyLength(t *testing.T) {
	_, err := pbkdf2.PBKDF2HmacSHA256([]byte("pw"), []byte("salt"), 1, 0)
	if err != pbkdf2.ErrInvalidKeyLength {
		t.Errorf("expected ErrInvalidKeyLength, got %v", err)
	}
}

func TestEmptySaltAllowed(t *testing.T) {
	// RFC 8018 does not forbid empty salt.
	dk, err := pbkdf2.PBKDF2HmacSHA256([]byte("password"), []byte{}, 1, 32)
	if err != nil {
		t.Fatalf("unexpected error for empty salt: %v", err)
	}
	if len(dk) != 32 {
		t.Errorf("expected 32 bytes, got %d", len(dk))
	}
}

func TestDeterministic(t *testing.T) {
	a, _ := pbkdf2.PBKDF2HmacSHA256([]byte("secret"), []byte("nacl"), 100, 32)
	b, _ := pbkdf2.PBKDF2HmacSHA256([]byte("secret"), []byte("nacl"), 100, 32)
	if string(a) != string(b) {
		t.Error("expected deterministic output")
	}
}

func TestDifferentSalts(t *testing.T) {
	a, _ := pbkdf2.PBKDF2HmacSHA256([]byte("password"), []byte("salt1"), 1, 32)
	b, _ := pbkdf2.PBKDF2HmacSHA256([]byte("password"), []byte("salt2"), 1, 32)
	if string(a) == string(b) {
		t.Error("different salts must produce different keys")
	}
}

func TestDifferentPasswords(t *testing.T) {
	a, _ := pbkdf2.PBKDF2HmacSHA256([]byte("password1"), []byte("salt"), 1, 32)
	b, _ := pbkdf2.PBKDF2HmacSHA256([]byte("password2"), []byte("salt"), 1, 32)
	if string(a) == string(b) {
		t.Error("different passwords must produce different keys")
	}
}

func TestDifferentIterations(t *testing.T) {
	a, _ := pbkdf2.PBKDF2HmacSHA256([]byte("password"), []byte("salt"), 1, 32)
	b, _ := pbkdf2.PBKDF2HmacSHA256([]byte("password"), []byte("salt"), 2, 32)
	if string(a) == string(b) {
		t.Error("different iteration counts must produce different keys")
	}
}
