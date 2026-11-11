package lz78_test

// Security regression tests: hostile headers must not drive work or memory.
//
// Each case is a few bytes of input whose header claims an enormous count or
// length.  A decompressor that sizes an allocation or loop from that claim
// would allocate gigabytes or run for minutes.  These tests assert bounded
// time and bounded cumulative allocation (runtime.MemStats.TotalAlloc), plus
// round-trip checks over every input of length <= 2 and a seeded random sample.

import (
	"bytes"
	"encoding/binary"
	"math/rand"
	"runtime"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lz78"
)

const (
	limitAlloc = 64 << 20
	limitTime  = 10 * time.Second
)

var _ = binary.BigEndian

// bounded runs fn and fails the test if it takes too long or allocates too much.
func bounded(t *testing.T, fn func() []byte) []byte {
	t.Helper()
	var before, after runtime.MemStats
	runtime.GC()
	runtime.ReadMemStats(&before)
	start := time.Now()
	out := fn()
	elapsed := time.Since(start)
	runtime.ReadMemStats(&after)
	if elapsed > limitTime {
		t.Fatalf("hostile input took %v", elapsed)
	}
	if alloc := after.TotalAlloc - before.TotalAlloc; alloc > limitAlloc {
		t.Fatalf("hostile input allocated %d bytes", alloc)
	}
	return out
}

func hdr(length, count uint32) []byte {
	b := make([]byte, 8)
	binary.BigEndian.PutUint32(b[0:4], length)
	binary.BigEndian.PutUint32(b[4:8], count)
	return b
}

func TestMaxHeaderOnly(t *testing.T) {
	out := bounded(t, func() []byte { return lz78.Decompress(bytes.Repeat([]byte{0xFF}, 8)) })
	if len(out) != 0 {
		t.Fatalf("got %v", out)
	}
}

func TestMaxLengthZeroTokens(t *testing.T) {
	out := bounded(t, func() []byte { return lz78.Decompress(hdr(0xFFFFFFFF, 0)) })
	if len(out) != 0 {
		t.Fatalf("got %v", out)
	}
}

func TestCountLargerThanRemainingBytesYieldsPresentTokens(t *testing.T) {
	data := append(hdr(2, 0xFFFFFFFF), 0, 0, 'h', 0, 0, 0, 'i', 0)
	if out := bounded(t, func() []byte { return lz78.Decompress(data) }); string(out) != "hi" {
		t.Fatalf("got %q", out)
	}
	data = append(data, 0, 0, 0)
	if out := lz78.Decompress(data); string(out) != "hi" {
		t.Fatalf("got %q", out)
	}
}

func TestLengthLargerThanTokensSupply(t *testing.T) {
	data := append(hdr(0xFFFFFFFF, 1), 0, 0, 'h', 0)
	if out := bounded(t, func() []byte { return lz78.Decompress(data) }); string(out) != "h" {
		t.Fatalf("got %q", out)
	}
}

func TestDanglingDictIndexDoesNotPanic(t *testing.T) {
	data := append(hdr(10, 2), 0, 0, 'a', 0, 0x27, 0x0F, 'b', 0)
	if out := bounded(t, func() []byte { return lz78.Decompress(data) }); string(out) != "a" {
		t.Fatalf("got %q", out)
	}
}

func TestChainTokensStopAtDeclaredLength(t *testing.T) {
	// 20k tokens each extending the previous entry would expand to ~2e8 bytes.
	const n = 20000
	data := hdr(1000, n)
	for i := 0; i < n; i++ {
		data = append(data, byte(i>>8), byte(i), 'a', 0)
	}
	if out := bounded(t, func() []byte { return lz78.Decompress(data) }); len(out) != 1000 {
		t.Fatalf("len %d", len(out))
	}
}

func TestRoundTripAllInputsUpToLength2(t *testing.T) {
	inputs := [][]byte{{}}
	for a := 0; a < 256; a++ {
		inputs = append(inputs, []byte{byte(a)})
		for b := 0; b < 256; b++ {
			inputs = append(inputs, []byte{byte(a), byte(b)})
		}
	}
	for _, d := range inputs {
		if got := lz78.Decompress(lz78.Compress(d, 65536)); !bytes.Equal(got, d) {
			t.Fatalf("round trip failed for %v: got %v", d, got)
		}
	}
}

func TestRoundTripSeededRandomInputs(t *testing.T) {
	rng := rand.New(rand.NewSource(0x5EED))
	alphabets := []int{2, 4, 16, 256}
	for i := 0; i < 300; i++ {
		d := make([]byte, 3+rng.Intn(597))
		for j := range d {
			d[j] = byte(rng.Intn(alphabets[i%4]))
		}
		if got := lz78.Decompress(lz78.Compress(d, 65536)); !bytes.Equal(got, d) {
			t.Fatalf("round trip failed for input %d", i)
		}
	}
}
