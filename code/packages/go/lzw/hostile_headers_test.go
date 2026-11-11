package lzw_test

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

	"github.com/adhithyan15/coding-adventures/code/packages/go/lzw"
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

func TestMaxHeaderOnly(t *testing.T) {
	bounded(t, func() []byte { return lzw.Decompress(bytes.Repeat([]byte{0xFF}, 4)) })
	bounded(t, func() []byte { return lzw.Decompress(bytes.Repeat([]byte{0xFF}, 64)) })
}

func TestMaxDeclaredLengthOnSmallStream(t *testing.T) {
	c := lzw.Compress([]byte("hello hello hello"))
	binary.BigEndian.PutUint32(c[:4], 0xFFFFFFFF)
	if out := bounded(t, func() []byte { return lzw.Decompress(c) }); string(out) != "hello hello hello" {
		t.Fatalf("got %q", out)
	}
}

func TestDeclaredLengthTruncatesOutput(t *testing.T) {
	c := lzw.Compress([]byte("hello hello hello"))
	binary.BigEndian.PutUint32(c[:4], 5)
	if out := bounded(t, func() []byte { return lzw.Decompress(c) }); string(out) != "hello" {
		t.Fatalf("got %q", out)
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
		if got := lzw.Decompress(lzw.Compress(d)); !bytes.Equal(got, d) {
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
		if got := lzw.Decompress(lzw.Compress(d)); !bytes.Equal(got, d) {
			t.Fatalf("round trip failed for input %d", i)
		}
	}
}
