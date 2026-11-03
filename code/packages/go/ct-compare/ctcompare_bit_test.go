package ctcompare

import "testing"

// The bit variants must agree with the boolean ones on every input, and be
// exactly 0 or 1. All 65,536 one-byte pairs are checked exhaustively.
func TestCTEqBitAgreesOnEveryBytePair(t *testing.T) {
	for a := 0; a < 256; a++ {
		for b := 0; b < 256; b++ {
			left, right := []byte{byte(a)}, []byte{byte(b)}
			want := 0
			if CTEq(left, right) {
				want = 1
			}
			if got := CTEqBit(left, right); got != want {
				t.Fatalf("CTEqBit(%#x, %#x) = %d, want %d", a, b, got, want)
			}
			if got := CTEqFixedBit(left, right); got != want {
				t.Fatalf("CTEqFixedBit(%#x, %#x) = %d, want %d", a, b, got, want)
			}
		}
	}
}

func TestCTEqBitLengthsAndPositions(t *testing.T) {
	if CTEqBit([]byte{}, []byte{}) != 1 {
		t.Fatal("empty inputs are equal")
	}
	if CTEqBit([]byte{1}, []byte{1, 2}) != 0 {
		t.Fatal("a length mismatch is not equal")
	}
	base := make([]byte, 64)
	for i := range base {
		for bit := 0; bit < 8; bit++ {
			other := append([]byte(nil), base...)
			other[i] ^= 1 << bit
			if CTEqBit(base, other) != 0 {
				t.Fatalf("difference at byte %d bit %d not detected", i, bit)
			}
		}
	}
}

func TestCTEqU64Bit(t *testing.T) {
	values := []uint64{0, 1, 2, 1 << 63, (1 << 63) - 1, ^uint64(0), 0x0123456789abcdef}
	for _, x := range values {
		for _, y := range values {
			want := 0
			if x == y {
				want = 1
			}
			if got := CTEqU64Bit(x, y); got != want {
				t.Fatalf("CTEqU64Bit(%#x, %#x) = %d, want %d", x, y, got, want)
			}
		}
		for bit := 0; bit < 64; bit++ {
			if CTEqU64Bit(x, x^(1<<bit)) != 0 {
				t.Fatalf("single-bit difference %d from %#x not detected", bit, x)
			}
		}
	}
}

// The point of the bit variants: equality feeds selection with no branch.
func TestEqualityBitFeedsSelection(t *testing.T) {
	a, b := []byte("left"), []byte("rite")
	if got := CTSelectBytes(a, b, CTEqBit([]byte("k"), []byte("k"))); string(got) != "left" {
		t.Fatalf("equal keys must select left, got %q", got)
	}
	if got := CTSelectBytes(a, b, CTEqBit([]byte("k"), []byte("j"))); string(got) != "rite" {
		t.Fatalf("different keys must select right, got %q", got)
	}
	if got := CTSelectBytes(a, b, CTEqU64Bit(7, 7)); string(got) != "left" {
		t.Fatalf("equal u64s must select left, got %q", got)
	}
}
