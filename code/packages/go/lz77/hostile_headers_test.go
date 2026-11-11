package lz77_test

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

	"github.com/adhithyan15/coding-adventures/code/packages/go/lz77"
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

func TestMaxCountHeaderOnly(t *testing.T) {
	out := bounded(t, func() []byte { return lz77.Decompress([]byte{0xFF, 0xFF, 0xFF, 0xFF}) })
	if len(out) != 0 {
		t.Fatalf("got %v", out)
	}
}

func TestCountLargerThanRemainingBytesYieldsPresentTokens(t *testing.T) {
	data := append([]byte{0xFF, 0xFF, 0xFF, 0xFF}, 0, 0, 0, 'h', 0, 0, 0, 'i')
	if out := bounded(t, func() []byte { return lz77.Decompress(data) }); string(out) != "hi" {
		t.Fatalf("got %q", out)
	}
	data = append(data, 0, 0, 0) // trailing partial token ignored
	if out := lz77.Decompress(data); string(out) != "hi" {
		t.Fatalf("got %q", out)
	}
}

func TestCountSmallerThanRemainingBytesStopsAtCount(t *testing.T) {
	data := []byte{0, 0, 0, 1, 0, 0, 0, 'h', 0, 0, 0, 'i'}
	if out := lz77.Decompress(data); string(out) != "h" {
		t.Fatalf("got %q", out)
	}
}

func TestInvalidOffsetsDoNotPanic(t *testing.T) {
	for _, data := range [][]byte{
		{0, 0, 0, 1, 0, 5, 3, 'x'},
		{0, 0, 0, 1, 0, 0, 3, 'x'},
	} {
		if out := lz77.Decompress(data); len(out) != 0 {
			t.Fatalf("got %v", out)
		}
	}
	if out := lz77.Decompress([]byte{0, 0, 0, 2, 0, 0, 0, 'a', 0, 9, 2, 'z'}); string(out) != "a" {
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
		if got := lz77.Decompress(lz77.Compress(d, 4096, 255, 3)); !bytes.Equal(got, d) {
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
		if got := lz77.Decompress(lz77.Compress(d, 4096, 255, 3)); !bytes.Equal(got, d) {
			t.Fatalf("round trip failed for input %d", i)
		}
	}
}
