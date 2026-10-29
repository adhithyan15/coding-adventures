package sha512

import (
	"encoding/hex"
	"errors"
	"os"
	"strconv"
	"strings"
	"testing"
)

func fixtureLines(t *testing.T) []string {
	t.Helper()
	data, err := os.ReadFile("../../../specs/fixtures/sha512-v1/cases.txt")
	if err != nil {
		t.Fatal(err)
	}
	if len(data) == 0 || len(data) > 4096 {
		t.Fatalf("fixture size %d outside bounds", len(data))
	}
	var lines []string
	for _, line := range strings.Split(string(data), "\n") {
		if line != "" && !strings.HasPrefix(line, "#") {
			lines = append(lines, line)
		}
	}
	if len(lines) == 0 || len(lines) > 64 {
		t.Fatalf("fixture row count %d outside bounds", len(lines))
	}
	return lines
}

func TestPortableCorpus(t *testing.T) {
	for _, line := range fixtureLines(t) {
		if !strings.HasPrefix(line, "V|") {
			continue
		}
		fields := strings.Split(line, "|")
		if len(fields) != 5 || len(fields[2]) > 8192 || len(fields[4]) != 128 {
			t.Fatalf("malformed vector row %q", line)
		}
		data, err := hex.DecodeString(fields[2])
		if err != nil {
			t.Fatal(err)
		}
		if len(data) > 4096 {
			t.Fatal("decoded input exceeds bound")
		}
		if got := HexString(data); got != fields[4] {
			t.Fatalf("%s one-shot: %s", fields[1], got)
		}
		d := New()
		offset := 0
		if fields[3] != "" {
			chunks := strings.Split(fields[3], ",")
			if len(chunks) > 64 {
				t.Fatal("too many chunks")
			}
			for _, raw := range chunks {
				size, err := strconv.Atoi(raw)
				if err != nil || size <= 0 || offset+size > len(data) {
					t.Fatalf("invalid chunk %q", raw)
				}
				if _, err := d.Write(data[offset : offset+size]); err != nil {
					t.Fatal(err)
				}
				offset += size
			}
		}
		if offset != len(data) {
			t.Fatalf("%s chunks cover %d of %d bytes", fields[1], offset, len(data))
		}
		if got := d.HexDigest(); got != fields[4] {
			t.Fatalf("%s streaming: %s", fields[1], got)
		}
	}
}

func TestPortableLengthCorpus(t *testing.T) {
	for _, line := range fixtureLines(t) {
		if !strings.HasPrefix(line, "L|") {
			continue
		}
		fields := strings.Split(line, "|")
		hi, err := strconv.ParseUint(fields[2], 16, 64)
		if err != nil {
			t.Fatal(err)
		}
		lo, err := strconv.ParseUint(fields[3], 16, 64)
		if err != nil {
			t.Fatal(err)
		}
		encoded, err := encodeBitLength(hi, lo)
		if fields[4] == "ERROR" {
			if !errors.Is(err, ErrMessageTooLong) {
				t.Fatalf("%s: %v", fields[1], err)
			}
			continue
		}
		if err != nil || hex.EncodeToString(encoded[:]) != fields[4] {
			t.Fatalf("%s length encoding", fields[1])
		}
	}
}

func TestWriteConsumesInputWithoutAliasing(t *testing.T) {
	input := []byte("abc")
	d := New()
	if _, err := d.Write(input); err != nil {
		t.Fatal(err)
	}
	input[0] = 'z'
	if d.Sum512() != Sum512([]byte("abc")) {
		t.Fatal("digest changed after caller mutation")
	}
}

func TestLengthOverflowWriteIsAtomic(t *testing.T) {
	d := New()
	d.byteCountHigh, d.byteCountLow = 0x1fffffffffffffff, 0xffffffffffffffff
	beforeState, beforeBuf := d.state, append([]byte(nil), d.buf...)
	if n, err := d.Write([]byte{1}); n != 0 || !errors.Is(err, ErrMessageTooLong) {
		t.Fatalf("Write overflow = %d, %v", n, err)
	}
	if d.state != beforeState || string(d.buf) != string(beforeBuf) {
		t.Fatal("overflowing Write mutated digest state")
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// FIPS 180-4 Test Vectors
// ═══════════════════════════════════════════════════════════════════════════

func TestFIPSEmptyString(t *testing.T) {
	got := HexString([]byte{})
	want := "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
	if got != want {
		t.Errorf("sha512('') = %s, want %s", got, want)
	}
}

func TestFIPSAbc(t *testing.T) {
	got := HexString([]byte("abc"))
	want := "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
	if got != want {
		t.Errorf("sha512('abc') = %s, want %s", got, want)
	}
}

func TestFIPS896BitMessage(t *testing.T) {
	msg := "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
	if len(msg) != 112 {
		t.Fatalf("test message should be 112 bytes, got %d", len(msg))
	}
	got := HexString([]byte(msg))
	want := "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909"
	if got != want {
		t.Errorf("sha512(112-byte msg) = %s, want %s", got, want)
	}
}

func TestFIPSMillionA(t *testing.T) {
	data := make([]byte, 1_000_000)
	for i := range data {
		data[i] = 'a'
	}
	got := HexString(data)
	want := "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973ebde0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b"
	if got != want {
		t.Errorf("sha512(1M 'a') = %s, want %s", got, want)
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Output format
// ═══════════════════════════════════════════════════════════════════════════

func TestDigestLength(t *testing.T) {
	digest := Sum512([]byte("test"))
	if len(digest) != 64 {
		t.Errorf("digest length = %d, want 64", len(digest))
	}
}

func TestHexStringLength(t *testing.T) {
	s := HexString([]byte("test"))
	if len(s) != 128 {
		t.Errorf("hex string length = %d, want 128", len(s))
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
	a := Sum512([]byte("hello"))
	b := Sum512([]byte("hello"))
	if a != b {
		t.Error("sha512(hello) gave different results on two calls")
	}
}

func TestAvalanche(t *testing.T) {
	h1 := Sum512([]byte("hello"))
	h2 := Sum512([]byte("helo"))
	if h1 == h2 {
		t.Error("one-char change produced same hash")
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Block boundary tests (128-byte blocks)
// ═══════════════════════════════════════════════════════════════════════════

func TestBlockBoundary111(t *testing.T) {
	result := Sum512(make([]byte, 111))
	if len(result) != 64 {
		t.Error("unexpected digest length")
	}
}

func TestBlockBoundary112(t *testing.T) {
	result := Sum512(make([]byte, 112))
	if len(result) != 64 {
		t.Error("unexpected digest length")
	}
}

func TestBlockBoundary128(t *testing.T) {
	result := Sum512(make([]byte, 128))
	if len(result) != 64 {
		t.Error("unexpected digest length")
	}
}

func TestBlockBoundary256(t *testing.T) {
	result := Sum512(make([]byte, 256))
	if len(result) != 64 {
		t.Error("unexpected digest length")
	}
}

func TestBoundariesDiffer(t *testing.T) {
	sizes := []int{111, 112, 127, 128, 255, 256}
	seen := make(map[[64]byte]int)
	for _, n := range sizes {
		d := Sum512(make([]byte, n))
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
	r := Sum512([]byte{0x00})
	if len(r) != 64 {
		t.Error("unexpected length")
	}
	if r == Sum512([]byte{}) {
		t.Error("null byte should differ from empty")
	}
}

func TestAllBytes(t *testing.T) {
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i)
	}
	r := Sum512(data)
	if len(r) != 64 {
		t.Error("unexpected length for all-256-bytes input")
	}
}

func TestHexStringMatchesSum512(t *testing.T) {
	data := []byte("hello")
	d := Sum512(data)
	h := HexString(data)
	if hex.EncodeToString(d[:]) != h {
		t.Error("HexString doesn't match Sum512")
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
	data := make([]byte, 256)
	for i := range data {
		data[i] = byte(i % 256)
	}
	h := New()
	h.Write(data[:128])
	h.Write(data[128:])
	if h.Sum512() != Sum512(data) {
		t.Error("split at block boundary produced wrong result")
	}
}

func TestStreamingByteAtATime(t *testing.T) {
	data := make([]byte, 200)
	for i := range data {
		data[i] = byte(i % 256)
	}
	h := New()
	for _, b := range data {
		h.Write([]byte{b})
	}
	if h.Sum512() != Sum512(data) {
		t.Error("byte-at-a-time streaming produced wrong result")
	}
}

func TestStreamingEmpty(t *testing.T) {
	h := New()
	if h.Sum512() != Sum512([]byte{}) {
		t.Error("empty streaming hash doesn't match empty oneshot")
	}
}

func TestStreamingNonDestructive(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	d1 := h.Sum512()
	d2 := h.Sum512()
	if d1 != d2 {
		t.Error("Sum512() is not idempotent")
	}
}

func TestStreamingHexDigest(t *testing.T) {
	h := New()
	h.Write([]byte("abc"))
	want := "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
	if h.HexDigest() != want {
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
	if h.Sum512() != Sum512(data) {
		t.Error("streaming million-a doesn't match oneshot")
	}
}
