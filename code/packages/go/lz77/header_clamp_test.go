package lz77

// Internal tests for the header clamp helpers. They need unexported access, so
// this file is in package lz77 rather than lz77_test.
//
// The 32-bit wrap (uint32 0xFFFFFFFF becoming int -1) cannot happen on a 64-bit
// test host, so these tests check the properties the clamp guarantees for any
// int width: the result is never negative, never exceeds the byte-derived
// limit, and equals the claimed value when that value already fits.

import "testing"

func TestClampCountTable(t *testing.T) {
	cases := []struct {
		name    string
		claimed uint32
		maxItem int
		want    int
	}{
		{"zero claim, zero room", 0, 0, 0},
		{"zero claim", 0, 10, 0},
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

func TestDeserialiseMaxCountHeaderIsBounded(t *testing.T) {
	// Count 0xFFFFFFFF followed by one complete token (offset 0, length 0,
	// next 'a'). The clamp must cap the count at the one token present.
	data := []byte{0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 'a'}
	tokens := deserialiseTokens(data)
	if len(tokens) != 1 {
		t.Fatalf("got %d tokens, want 1", len(tokens))
	}
	if tokens[0].NextChar != 'a' {
		t.Fatalf("got next char %q, want 'a'", tokens[0].NextChar)
	}
}
