package ctcompare

import (
	"bytes"
	"testing"
)

func TestCTEq(t *testing.T) {
	if !CTEq([]byte("abcdef"), []byte("abcdef")) {
		t.Fatal("equal bytes should compare true")
	}
	if !CTEq(nil, []byte{}) {
		t.Fatal("empty slices should compare true")
	}
	if CTEq([]byte("abcdef"), []byte("abcdeg")) {
		t.Fatal("different last byte should compare false")
	}
	if CTEq([]byte("abcdef"), []byte("bbcdef")) {
		t.Fatal("different first byte should compare false")
	}
	if CTEq([]byte("abc"), []byte("abcd")) {
		t.Fatal("length mismatch should compare false")
	}
}

func TestCTEqDetectsEveryBitPosition(t *testing.T) {
	base := bytes.Repeat([]byte{0x42}, 32)
	for index := range base {
		for bit := 0; bit < 8; bit++ {
			flipped := append([]byte(nil), base...)
			flipped[index] ^= 1 << bit
			if CTEq(base, flipped) {
				t.Fatalf("flip at byte %d bit %d not detected", index, bit)
			}
		}
	}
}

func TestCTEqFixed(t *testing.T) {
	if !CTEqFixed(bytes.Repeat([]byte{0x11}, 16), bytes.Repeat([]byte{0x11}, 16)) {
		t.Fatal("fixed equal bytes should compare true")
	}
	if CTEqFixed(bytes.Repeat([]byte{0x11}, 16), append(bytes.Repeat([]byte{0x11}, 15), 0x10)) {
		t.Fatal("fixed different bytes should compare false")
	}
}

func TestCTSelectBytes(t *testing.T) {
	left := make([]byte, 256)
	right := make([]byte, 256)
	for index := range left {
		left[index] = byte(index)
		right[index] = byte(255 - index)
	}

	if !bytes.Equal(CTSelectBytes(left, right, true), left) {
		t.Fatal("true should select left")
	}
	if !bytes.Equal(CTSelectBytes(left, right, false), right) {
		t.Fatal("false should select right")
	}
	if len(CTSelectBytes(nil, nil, true)) != 0 {
		t.Fatal("empty select should stay empty")
	}

	defer func() {
		if recover() == nil {
			t.Fatal("length mismatch should panic")
		}
	}()
	CTSelectBytes([]byte{1}, []byte{1, 2}, true)
}

func TestCTEqU64(t *testing.T) {
	if !CTEqU64(0, 0) || !CTEqU64(^uint64(0), ^uint64(0)) {
		t.Fatal("equal u64 values should compare true")
	}
	if CTEqU64(0, 1<<63) {
		t.Fatal("high-bit difference should compare false")
	}

	base := uint64(0x1234_5678_9ABC_DEF0)
	for bit := 0; bit < 64; bit++ {
		if CTEqU64(base, base^(1<<bit)) {
			t.Fatalf("flip at bit %d not detected", bit)
		}
	}
}
