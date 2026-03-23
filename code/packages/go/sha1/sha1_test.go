package ca_sha1

import (
	"encoding/hex"
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════════
// FIPS 180-4 Test Vectors
// ═══════════════════════════════════════════════════════════════════════════

func TestFIPSEmptyString(t *testing.T) {
	got := HexString([]byte{})
	want := "da39a3ee5e6b4b0d3255bfef95601890afd80709"
	if got != want {
		t.Errorf("sha1('') = %s, want %s", got, want)
	}
}

func TestFIPSAbc(t *testing.T) {
	got := HexString([]byte("abc"))
	want := "a9993e364706816aba3e25717850c26c9cd0d89d"
	if got != want {
		t.Errorf("sha1('abc') = %s, want %s", got, want)
	}
}

func TestFIPS448BitMessage(t *testing.T) {
	msg := "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
	if len(msg) != 56 {
		t.Fatalf("test message should be 56 bytes, got %d", len(msg))
	}
	got := HexString([]byte(msg))
	want := "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
	if got != want {
		t.Errorf("sha1(56-byte msg) = %s, want %s", got, want)
	}
}

func TestFIPSMillionA(t *testing.T) {
	got := HexString([]byte(make([]byte, 1_000_000)))
	for i := range make([]byte, 1_000_000) {
		_ = i
	}
	data := make([]byte, 1_000_000)
	for i := range data {
		data[i] = 'a'
	}
	got = HexString(data)
	want := "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
	if got != want {
		t.Errorf("sha1(1M 'a') = %s, want %s", got, want)
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Output format
// ═══════════════════════════════════════════════════════════════════════════

func TestDigestLength(t *testing.T) {
	digest := Sum1([]byte("test"))
	if len(digest) != 20 {
		t.Errorf("digest length = %d, want 20", len(digest))
	}
}

func TestHexStringLength(t *testing.T) {
	s := HexString([]byte("test"))
	if len(s) != 40 {
		t.Errorf("hex string length = %d, want 40", len(s))
	}
}

func TestHexStringLowercase(t *testing.T) {
	s := HexString([]byte("abc"))
	for _, c := range s {
		if c >= 'A' && c <= 'F' {
			t.Errorf("hex string contains uppercase: %s", s)
		}
	}
}

func TestDeterministic(t *testing.T) {
	a := Sum1([]byte("hello"))
	b := Sum1([]byte("hello"))
	if a != b {
		t.Error("sha1(hello) gave different results on two calls")
	}
}

func TestAvalanche(t *testing.T) {
	h1 := Sum1([]byte("hello"))
	h2 := Sum1([]byte("helo"))
	if h1 == h2 {
		t.Error("one-char change produced same hash")
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Block boundary tests
// ═══════════════════════════════════════════════════════════════════════════

func TestBlockBoundary55(t *testing.T) {
	// 55 bytes fits in one block (55 + 1 + 8 = 64)
	result := Sum1(make([]byte, 55))
	if len(result) != 20 {
		t.Error("unexpected digest length")
	}
}

func TestBlockBoundary56(t *testing.T) {
	// 56 bytes forces two blocks
	result := Sum1(make([]byte, 56))
	if len(result) != 20 {
		t.Error("unexpected digest length")
	}
}

func TestBlockBoundary64(t *testing.T) {
	result := Sum1(make([]byte, 64))
	if len(result) != 20 {
		t.Error("unexpected digest length")
	}
}

func TestBlockBoundary128(t *testing.T) {
	result := Sum1(make([]byte, 128))
	if len(result) != 20 {
		t.Error("unexpected digest length")
	}
}

func TestBoundariesDiffer(t *testing.T) {
	sizes := []int{55, 56, 63, 64, 127, 128}
	seen := make(map[[20]byte]int)
	for _, n := range sizes {
		d := Sum1(make([]byte, n))
		if prev, ok := seen[d]; ok {
			t.Errorf("sizes %d and %d produce the same hash", prev, n)
		}
		seen[d] = n
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════════════════════

func TestNullByte(t *testing.T) {
	r := Sum1([]byte{0x00})
	if len(r) != 20 {
		t.Error("unexpected length")
	}
	if r == Sum1([]byte{}) {
		t.Error("null byte should differ from empty")
	}
}

func TestAllBytes(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	r := Sum1(data)
	if len(r) != 20 {
		t.Error("unexpected length for all-256-bytes input")
	}
}

func TestHexStringMatchesSum1(t *testing.T) {
	data := []byte("hello")
	d := Sum1(data)
	h := HexString(data)
	if hex.EncodeToString(d[:]) != h {
		t.Error("HexString doesn't match Sum1")
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Streaming API
// ═══════════════════════════════════════════════════════════════════════════

func TestStreamingSingleWrite(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	got := h.HexDigest()
	want := HexString([]byte("abc"))
	if got != want {
		t.Errorf("streaming = %s, oneshot = %s", got, want)
	}
}

func TestStreamingSplitAtByte(t *testing.T) {
	h := New()
	h.Write([]byte("ab"))
	h.Write([]byte("c"))
	if h.HexDigest() != HexString([]byte("abc")) {
		t.Error("split at byte produced wrong result")
	}
}

func TestStreamingSplitAtBlock(t *testing.T) {
	data := make([]byte, 128)
	for i := range data {
		data[i] = byte(i % 256)
	}
	h := New()
	h.Write(data[:64])
	h.Write(data[64:])
	if h.Sum1() != Sum1(data) {
		t.Error("split at block boundary produced wrong result")
	}
}

func TestStreamingByteAtATime(t *testing.T) {
	data := make([]byte, 100)
	for i := range data {
		data[i] = byte(i)
	}
	h := New()
	for _, b := range data {
		h.Write([]byte{b})
	}
	if h.Sum1() != Sum1(data) {
		t.Error("byte-at-a-time streaming produced wrong result")
	}
}

func TestStreamingEmpty(t *testing.T) {
	h := New()
	if h.Sum1() != Sum1([]byte{}) {
		t.Error("empty streaming hash doesn't match empty oneshot")
	}
}

func TestStreamingNonDestructive(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	d1 := h.Sum1()
	d2 := h.Sum1()
	if d1 != d2 {
		t.Error("Sum1() is not idempotent")
	}
}

func TestStreamingHexDigest(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	if h.HexDigest() != "a9993e364706816aba3e25717850c26c9cd0d89d" {
		t.Error("HexDigest wrong for 'abc'")
	}
}

func TestStreamingMillionA(t *testing.T) {
	data := make([]byte, 1_000_000)
	for i := range data {
		data[i] = 'a'
	}
	h := New()
	h.Write(data[:500_000])
	h.Write(data[500_000:])
	if h.Sum1() != Sum1(data) {
		t.Error("streaming million-a doesn't match oneshot")
	}
}
