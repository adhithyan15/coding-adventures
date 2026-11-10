package lzss

// Internal tests for the header clamp helpers. They need unexported access, so
// this file is in package lzss rather than lzss_test.
//
// The 32-bit wrap (uint32 0xFFFFFFFF becoming int -1) cannot happen on a 64-bit
// test host, so these tests check the guarantees the clamp gives on any int
// width: never negative, bounded by the byte-derived limit where one exists,
// and equal to the claimed value when that value already fits.

import (
	"math"
	"testing"
)

func TestClampCountTable(t *testing.T) {
	cases := []struct {
		name    string
		claimed uint32
		maxItem int
		want    int
	}{
		{"zero claim, zero room", 0, 0, 0},
		{"claim fits", 3, 10, 3},
		{"claim equals limit", 10, 10, 10},
		{"claim above limit is clamped", 11, 10, 10},
		{"max uint32 with no room", 0xFFFFFFFF, 0, 0},
		{"max uint32 clamped to room", 0xFFFFFFFF, 5, 5},
		{"negative room yields zero", 7, -1, 0},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			if got := clampCount(c.claimed, c.maxItem); got != c.want {
				t.Fatalf("clampCount(%#x, %d) = %d, want %d", c.claimed, c.maxItem, got, c.want)
			}
		})
	}
}

func TestClampCountNeverNegativeOrAboveLimit(t *testing.T) {
	claims := []uint32{0, 1, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFE, 0xFFFFFFFF}
	limits := []int{0, 1, 7, 4096}
	for _, v := range claims {
		for _, m := range limits {
			got := clampCount(v, m)
			if got < 0 || got > m {
				t.Fatalf("clampCount(%#x, %d) = %d, outside [0, %d]", v, m, got, m)
			}
		}
	}
}

func TestClampLengthNeverNegative(t *testing.T) {
	for _, v := range []uint32{0, 1, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF} {
		got := clampLength(v)
		if got < 0 {
			t.Fatalf("clampLength(%#x) = %d, want non-negative", v, got)
		}
		// On a 64-bit host the value fits, so it must pass through unchanged.
		if uint64(math.MaxInt) > uint64(v) && got != int(v) {
			t.Fatalf("clampLength(%#x) = %d, want %d", v, got, int(v))
		}
	}
}

func TestDeserialiseMaxHeaderValuesAreBounded(t *testing.T) {
	// original length 0xFFFFFFFF and block count 0xFFFFFFFF, followed by one
	// block whose flag byte is 0 (one literal 'a'). The block count clamps to
	// the bytes present, and the length must stay non-negative.
	data := []byte{
		0xFF, 0xFF, 0xFF, 0xFF,
		0xFF, 0xFF, 0xFF, 0xFF,
		0x00, 'a',
	}
	tokens, originalLength := DeserialiseTokens(data)
	if len(tokens) != 1 {
		t.Fatalf("got %d tokens, want 1", len(tokens))
	}
	if originalLength < 0 {
		t.Fatalf("originalLength = %d, want non-negative", originalLength)
	}
}
