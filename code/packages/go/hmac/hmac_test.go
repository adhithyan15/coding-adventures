package hmac_test

import (
	"bytes"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/hmac"
)

// ===========================================================================
// RFC 4231 — HMAC-SHA256
// ===========================================================================

func TestHmacSHA256_RFC4231_TC1(t *testing.T) {
	key := bytes.Repeat([]byte{0x0b}, 20)
	data := []byte("Hi There")
	want := "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
	if got := hmac.HmacSHA256Hex(key, data); got != want {
		t.Errorf("TC1 SHA256: got %s, want %s", got, want)
	}
}

func TestHmacSHA256_RFC4231_TC2(t *testing.T) {
	want := "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
	if got := hmac.HmacSHA256Hex([]byte("Jefe"), []byte("what do ya want for nothing?")); got != want {
		t.Errorf("TC2 SHA256: got %s, want %s", got, want)
	}
}

func TestHmacSHA256_RFC4231_TC3(t *testing.T) {
	key := bytes.Repeat([]byte{0xaa}, 20)
	data := bytes.Repeat([]byte{0xdd}, 50)
	want := "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
	if got := hmac.HmacSHA256Hex(key, data); got != want {
		t.Errorf("TC3 SHA256: got %s, want %s", got, want)
	}
}

func TestHmacSHA256_RFC4231_TC6(t *testing.T) {
	key := bytes.Repeat([]byte{0xaa}, 131)
	data := []byte("Test Using Larger Than Block-Size Key - Hash Key First")
	want := "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
	if got := hmac.HmacSHA256Hex(key, data); got != want {
		t.Errorf("TC6 SHA256: got %s, want %s", got, want)
	}
}

func TestHmacSHA256_RFC4231_TC7(t *testing.T) {
	key := bytes.Repeat([]byte{0xaa}, 131)
	data := []byte("This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.")
	want := "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2"
	if got := hmac.HmacSHA256Hex(key, data); got != want {
		t.Errorf("TC7 SHA256: got %s, want %s", got, want)
	}
}

// ===========================================================================
// RFC 4231 — HMAC-SHA512
// ===========================================================================

func TestHmacSHA512_RFC4231_TC1(t *testing.T) {
	key := bytes.Repeat([]byte{0x0b}, 20)
	want := "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
	if got := hmac.HmacSHA512Hex(key, []byte("Hi There")); got != want {
		t.Errorf("TC1 SHA512: got %s, want %s", got, want)
	}
}

func TestHmacSHA512_RFC4231_TC2(t *testing.T) {
	want := "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
	if got := hmac.HmacSHA512Hex([]byte("Jefe"), []byte("what do ya want for nothing?")); got != want {
		t.Errorf("TC2 SHA512: got %s, want %s", got, want)
	}
}

func TestHmacSHA512_RFC4231_TC3(t *testing.T) {
	key := bytes.Repeat([]byte{0xaa}, 20)
	data := bytes.Repeat([]byte{0xdd}, 50)
	want := "fa73b0089d56a284efb0f0756c890be9b1b5dbdd8ee81a3655f83e33b2279d39bf3e848279a722c806b485a47e67c807b946a337bee8942674278859e13292fb"
	if got := hmac.HmacSHA512Hex(key, data); got != want {
		t.Errorf("TC3 SHA512: got %s, want %s", got, want)
	}
}

func TestHmacSHA512_RFC4231_TC6(t *testing.T) {
	key := bytes.Repeat([]byte{0xaa}, 131)
	data := []byte("Test Using Larger Than Block-Size Key - Hash Key First")
	want := "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598"
	if got := hmac.HmacSHA512Hex(key, data); got != want {
		t.Errorf("TC6 SHA512: got %s, want %s", got, want)
	}
}

// ===========================================================================
// RFC 2202 — HMAC-MD5
// ===========================================================================

func TestHmacMD5_RFC2202_TC1(t *testing.T) {
	key := bytes.Repeat([]byte{0x0b}, 16)
	if got := hmac.HmacMD5Hex(key, []byte("Hi There")); got != "9294727a3638bb1c13f48ef8158bfc9d" {
		t.Errorf("MD5 TC1: got %s", got)
	}
}

func TestHmacMD5_RFC2202_TC2(t *testing.T) {
	want := "750c783e6ab0b503eaa86e310a5db738"
	if got := hmac.HmacMD5Hex([]byte("Jefe"), []byte("what do ya want for nothing?")); got != want {
		t.Errorf("MD5 TC2: got %s", got)
	}
}

func TestHmacMD5_RFC2202_TC6(t *testing.T) {
	key := bytes.Repeat([]byte{0xaa}, 80)
	data := []byte("Test Using Larger Than Block-Size Key - Hash Key First")
	if got := hmac.HmacMD5Hex(key, data); got != "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd" {
		t.Errorf("MD5 TC6: got %s", got)
	}
}

// ===========================================================================
// RFC 2202 — HMAC-SHA1
// ===========================================================================

func TestHmacSHA1_RFC2202_TC1(t *testing.T) {
	key := bytes.Repeat([]byte{0x0b}, 20)
	want := "b617318655057264e28bc0b6fb378c8ef146be00"
	if got := hmac.HmacSHA1Hex(key, []byte("Hi There")); got != want {
		t.Errorf("SHA1 TC1: got %s, want %s", got, want)
	}
}

func TestHmacSHA1_RFC2202_TC2(t *testing.T) {
	want := "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
	if got := hmac.HmacSHA1Hex([]byte("Jefe"), []byte("what do ya want for nothing?")); got != want {
		t.Errorf("SHA1 TC2: got %s", got)
	}
}

func TestHmacSHA1_RFC2202_TC6(t *testing.T) {
	key := bytes.Repeat([]byte{0xaa}, 80)
	data := []byte("Test Using Larger Than Block-Size Key - Hash Key First")
	if got := hmac.HmacSHA1Hex(key, data); got != "aa4ae5e15272d00e95705637ce8a3b55ed402112" {
		t.Errorf("SHA1 TC6: got %s", got)
	}
}

// ===========================================================================
// Return lengths
// ===========================================================================

func TestReturnLengths(t *testing.T) {
	key, msg := []byte("key"), []byte("msg")
	if n := len(hmac.HmacMD5(key, msg)); n != 16 {
		t.Errorf("MD5 len: want 16, got %d", n)
	}
	if n := len(hmac.HmacSHA1(key, msg)); n != 20 {
		t.Errorf("SHA1 len: want 20, got %d", n)
	}
	if n := len(hmac.HmacSHA256(key, msg)); n != 32 {
		t.Errorf("SHA256 len: want 32, got %d", n)
	}
	if n := len(hmac.HmacSHA512(key, msg)); n != 64 {
		t.Errorf("SHA512 len: want 64, got %d", n)
	}
}

// ===========================================================================
// Key handling
// ===========================================================================

func TestEmptyKey(t *testing.T) {
	if n := len(hmac.HmacSHA256([]byte{}, []byte("msg"))); n != 32 {
		t.Error("empty key should produce 32-byte result")
	}
}

func TestEmptyMessage(t *testing.T) {
	if n := len(hmac.HmacSHA256([]byte("key"), []byte{})); n != 32 {
		t.Error("empty message should produce 32-byte result")
	}
}

func TestKeyLongerThanBlock(t *testing.T) {
	k65 := bytes.Repeat([]byte{0x01}, 65)
	k66 := bytes.Repeat([]byte{0x01}, 66)
	r1 := hmac.HmacSHA256(k65, []byte("msg"))
	r2 := hmac.HmacSHA256(k66, []byte("msg"))
	if bytes.Equal(r1, r2) {
		t.Error("65-byte and 66-byte keys should produce different tags")
	}
}

// ===========================================================================
// Authentication properties
// ===========================================================================

func TestDeterministic(t *testing.T) {
	if !bytes.Equal(hmac.HmacSHA256([]byte("k"), []byte("m")), hmac.HmacSHA256([]byte("k"), []byte("m"))) {
		t.Error("HMAC must be deterministic")
	}
}

func TestKeySensitivity(t *testing.T) {
	if bytes.Equal(hmac.HmacSHA256([]byte("k1"), []byte("m")), hmac.HmacSHA256([]byte("k2"), []byte("m"))) {
		t.Error("different keys must produce different tags")
	}
}

func TestMessageSensitivity(t *testing.T) {
	if bytes.Equal(hmac.HmacSHA256([]byte("k"), []byte("m1")), hmac.HmacSHA256([]byte("k"), []byte("m2"))) {
		t.Error("different messages must produce different tags")
	}
}
