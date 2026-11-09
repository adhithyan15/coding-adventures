package lzss_test

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

	"github.com/adhithyan15/coding-adventures/code/packages/go/lzss"
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

func hdr(length, blocks uint32) []byte {
	b := make([]byte, 8)
	binary.BigEndian.PutUint32(b[0:4], length)
	binary.BigEndian.PutUint32(b[4:8], blocks)
	return b
}

func TestMaxHeaderOnly(t *testing.T) {
	out := bounded(t, func() []byte { return lzss.Decompress(bytes.Repeat([]byte{0xFF}, 8)) })
	if len(out) != 0 {
		t.Fatalf("got %v", out)
	}
}

func TestMaxLengthOneLiteral(t *testing.T) {
	data := append(hdr(0xFFFFFFFF, 1), 0x00, 'x')
	if out := bounded(t, func() []byte { return lzss.Decompress(data) }); string(out) != "x" {
		t.Fatalf("got %q", out)
	}
}

func TestBlockCountLargerThanRemainingBytes(t *testing.T) {
	data := append(hdr(2, 0xFFFFFFFF), 0x00, 'h', 'i')
	if out := bounded(t, func() []byte { return lzss.Decompress(data) }); string(out) != "hi" {
		t.Fatalf("got %q", out)
	}
}

func TestLengthInconsistentWithTokens(t *testing.T) {
	data := append(hdr(1, 1), 0x00, 'h', 'i')
	if out := lzss.Decompress(data); string(out) != "h" {
		t.Fatalf("got %q", out)
	}
}

func TestDecodeNegativeLengthDoesNotPanic(t *testing.T) {
	toks := []lzss.Token{lzss.Literal('a')}
	if out := lzss.Decode(toks, -1); string(out) != "a" {
		t.Fatalf("got %q", out)
	}
}

func TestInvalidMatchOffsetsDoNotPanic(t *testing.T) {
	for _, data := range [][]byte{
		append(hdr(100, 1), 0x01, 0, 0, 5),
		append(hdr(100, 1), 0x01, 0xFF, 0xFF, 255),
	} {
		if out := bounded(t, func() []byte { return lzss.Decompress(data) }); len(out) != 0 {
			t.Fatalf("got %v", out)
		}
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
		if got := lzss.Decompress(lzss.Compress(d)); !bytes.Equal(got, d) {
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
		if got := lzss.Decompress(lzss.Compress(d)); !bytes.Equal(got, d) {
			t.Fatalf("round trip failed for input %d", i)
		}
	}
}
