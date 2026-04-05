package sha256

import (
	"bytes"
	"encoding/hex"
	"math/bits"
	"testing"
)

// === FIPS 180-4 Test Vectors =================================================

func TestFIPSEmptyString(t *testing.T) {
	got := HexString([]byte{})
	want := "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
	if got != want {
		t.Errorf("sha256('') = %s, want %s", got, want)
	}
}

func TestFIPSAbc(t *testing.T) {
	got := HexString([]byte("abc"))
	want := "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
	if got != want {
		t.Errorf("sha256('abc') = %s, want %s", got, want)
	}
}

func TestFIPS448BitMessage(t *testing.T) {
	msg := "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
	if len(msg) != 56 {
		t.Fatalf("test message should be 56 bytes, got %d", len(msg))
	}
	got := HexString([]byte(msg))
	want := "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
	if got != want {
		t.Errorf("sha256(56-byte msg) = %s, want %s", got, want)
	}
}

func TestFIPSMillionA(t *testing.T) {
	data := make([]byte, 1_000_000)
	for i := range data {
		data[i] = 'a'
	}
	got := HexString(data)
	want := "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
	if got != want {
		t.Errorf("sha256(million 'a') = %s, want %s", got, want)
	}
}

// === Return Type and Format ==================================================

func TestReturnLength(t *testing.T) {
	digest := Sum256([]byte("test"))
	if len(digest) != 32 {
		t.Errorf("digest length = %d, want 32", len(digest))
	}
}

func TestDeterministic(t *testing.T) {
	d1 := Sum256([]byte("hello"))
	d2 := Sum256([]byte("hello"))
	if d1 != d2 {
		t.Error("same input produced different digests")
	}
}

func TestAvalancheEffect(t *testing.T) {
	d1 := Sum256([]byte("hello"))
	d2 := Sum256([]byte("helo"))
	if d1 == d2 {
		t.Fatal("different inputs produced same digest")
	}
	bitsDiff := 0
	for i := range d1 {
		bitsDiff += bits.OnesCount8(d1[i] ^ d2[i])
	}
	// 256 bits total, expect ~128 different, require at least 50
	if bitsDiff < 50 {
		t.Errorf("only %d bits differ (expected ~128 of 256)", bitsDiff)
	}
}

func TestHexStringLength(t *testing.T) {
	result := HexString([]byte("abc"))
	if len(result) != 64 {
		t.Errorf("hex length = %d, want 64", len(result))
	}
}

func TestHexStringLowercase(t *testing.T) {
	result := HexString([]byte("abc"))
	for _, c := range result {
		if c >= 'A' && c <= 'F' {
			t.Errorf("hex contains uppercase: %s", result)
			break
		}
	}
}

// === Block Boundary Tests ====================================================

func TestBlockBoundaries(t *testing.T) {
	lengths := []int{55, 56, 63, 64, 119, 120, 127, 128}
	digests := make(map[string]int)
	for _, n := range lengths {
		data := bytes.Repeat([]byte("x"), n)
		d := Sum256(data)
		if len(d) != 32 {
			t.Errorf("length %d: digest size = %d, want 32", n, len(d))
		}
		h := hex.EncodeToString(d[:])
		if _, exists := digests[h]; exists {
			t.Errorf("length %d: duplicate digest with length %d", n, digests[h])
		}
		digests[h] = n
	}
}

// === Edge Cases ==============================================================

func TestSingleZeroByte(t *testing.T) {
	d1 := Sum256([]byte{0x00})
	d2 := Sum256([]byte{})
	if d1 == d2 {
		t.Error("zero byte and empty should differ")
	}
}

func TestAllByteValues(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	d := Sum256(data)
	if len(d) != 32 {
		t.Errorf("digest length = %d, want 32", len(d))
	}
}

func TestDifferentSingleBytes(t *testing.T) {
	seen := make(map[[32]byte]bool)
	for i := 0; i < 256; i++ {
		d := Sum256([]byte{byte(i)})
		if seen[d] {
			t.Errorf("byte %d: duplicate digest", i)
		}
		seen[d] = true
	}
}

// === Streaming API ===========================================================

func TestStreamingSingleUpdate(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	if h.Sum256() != Sum256([]byte("abc")) {
		t.Error("streaming single update != one-shot")
	}
}

func TestStreamingSplit(t *testing.T) {
	h := New()
	h.Write([]byte("ab"))
	h.Write([]byte("c"))
	if h.Sum256() != Sum256([]byte("abc")) {
		t.Error("streaming split != one-shot")
	}
}

func TestStreamingBlockBoundary(t *testing.T) {
	data := bytes.Repeat([]byte("x"), 128)
	h := New()
	h.Write(data[:64])
	h.Write(data[64:])
	if h.Sum256() != Sum256(data) {
		t.Error("streaming at block boundary != one-shot")
	}
}

func TestStreamingManyTiny(t *testing.T) {
	data := make([]byte, 100)
	for i := range data {
		data[i] = byte(i)
	}
	h := New()
	for _, b := range data {
		h.Write([]byte{b})
	}
	if h.Sum256() != Sum256(data) {
		t.Error("streaming byte-at-a-time != one-shot")
	}
}

func TestStreamingEmpty(t *testing.T) {
	h := New()
	if h.Sum256() != Sum256([]byte{}) {
		t.Error("empty streaming != empty one-shot")
	}
}

func TestStreamingNonDestructive(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	d1 := h.Sum256()
	d2 := h.Sum256()
	if d1 != d2 {
		t.Error("Sum256() not idempotent")
	}
}

func TestStreamingUpdateAfterDigest(t *testing.T) {
	h := New()
	h.Write([]byte("ab"))
	_ = h.Sum256()
	h.Write([]byte("c"))
	if h.Sum256() != Sum256([]byte("abc")) {
		t.Error("update after digest failed")
	}
}

func TestStreamingHexDigest(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	want := "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
	if h.HexDigest() != want {
		t.Errorf("HexDigest() = %s, want %s", h.HexDigest(), want)
	}
}

func TestStreamingCopyIndependent(t *testing.T) {
	h := New()
	h.Write([]byte("ab"))
	h2 := h.Copy()
	h2.Write([]byte("c"))
	h.Write([]byte("x"))
	if h2.Sum256() != Sum256([]byte("abc")) {
		t.Error("copy divergence failed for copy")
	}
	if h.Sum256() != Sum256([]byte("abx")) {
		t.Error("copy divergence failed for original")
	}
}

func TestStreamingCopySameResult(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	h2 := h.Copy()
	if h.Sum256() != h2.Sum256() {
		t.Error("copy should produce same digest")
	}
}

func TestStreamingFIPSVector(t *testing.T) {
	h := New()
	h.Write(bytes.Repeat([]byte("a"), 500_000))
	h.Write(bytes.Repeat([]byte("a"), 500_000))
	if h.Sum256() != Sum256(bytes.Repeat([]byte("a"), 1_000_000)) {
		t.Error("streaming million 'a' != one-shot")
	}
}

func TestStreamingVariousChunks(t *testing.T) {
	data := bytes.Repeat([]byte("a"), 200)
	expected := Sum256(data)
	for _, chunkSize := range []int{1, 7, 13, 32, 63, 64, 65, 100, 200} {
		h := New()
		for i := 0; i < len(data); i += chunkSize {
			end := i + chunkSize
			if end > len(data) {
				end = len(data)
			}
			h.Write(data[i:end])
		}
		if h.Sum256() != expected {
			t.Errorf("chunk size %d: digest mismatch", chunkSize)
		}
	}
}
