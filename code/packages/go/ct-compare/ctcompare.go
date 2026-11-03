package ctcompare

import "math/bits"

func CTEq(left, right []byte) bool {
	if len(left) != len(right) {
		return false
	}

	var accumulator byte
	for index := range left {
		accumulator |= left[index] ^ right[index]
	}
	return accumulator == 0
}

func CTEqFixed(left, right []byte) bool {
	return CTEq(left, right)
}

// CTSelectBytes returns a fresh copy of left when choice is 1 and of right
// when choice is 0, without branching on choice.
//
// Why an int and not a bool: Go has no conversion from bool to an integer, so
// the only way to turn a bool into a mask is `if choice { mask = 0xFF }`, a
// branch on the secret bit that CT01 forbids. Like Go's own
// crypto/subtle.ConstantTimeCopy, this function therefore takes the bit as an
// int that must be exactly 0 or 1.
//
// The mask is computed arithmetically:
//
//	choice = 0  ->  byte(-0) = 0x00  ->  result is right
//	choice = 1  ->  byte(-1) = 0xFF  ->  result is left
//
// Validation tests choice&^1 (choice with bit 0 cleared), which is 0 for both
// valid values, so a valid call takes the same path whichever bit is secret.
// Any other value, or a length mismatch, panics before either input is read.
func CTSelectBytes(left, right []byte, choice int) []byte {
	if len(left) != len(right) {
		panic("ctselectbytes requires equal-length slices")
	}
	if choice&^1 != 0 {
		panic("ctselectbytes requires choice to be 0 or 1")
	}

	mask := byte(-choice)
	output := make([]byte, len(left))
	for index := range left {
		output[index] = right[index] ^ ((left[index] ^ right[index]) & mask)
	}
	return output
}

func CTEqU64(left, right uint64) bool {
	diff := left ^ right
	return bits.LeadingZeros64(diff) == 64
}

// ── Bit-returning equality (CT01, "How a lane receives the selection bit") ──
//
// CTSelectBytes takes its selection bit as an int, because Go cannot turn a
// bool into an integer without a branch. A caller that writes
//
//	CTSelectBytes(a, b, CTEq(x, y))   // does not compile: bool, not int
//
// would need `if CTEq(x, y) { bit = 1 }`, the very branch CT01 removes. So
// each equality also comes in a form that returns the bit itself, computed
// arithmetically from the difference accumulator:
//
//	acc = 0          uint32(acc) - 1 = 0xFFFFFFFF  >> 31 = 1   (equal)
//	acc = 1 .. 255   uint32(acc) - 1 < 2^31        >> 31 = 0   (different)
//
// This is the same identity as crypto/subtle.ConstantTimeByteEq. Use it as
//
//	CTSelectBytes(a, b, CTEqBit(x, y))

// CTEqBit returns 1 when left and right are equal and 0 otherwise. A length
// mismatch returns 0 without reading either input; lengths are public.
func CTEqBit(left, right []byte) int {
	if len(left) != len(right) {
		return 0
	}
	var accumulator byte
	for index := range left {
		accumulator |= left[index] ^ right[index]
	}
	return int((uint32(accumulator) - 1) >> 31)
}

// CTEqFixedBit is CTEqBit for inputs of a declared public width.
func CTEqFixedBit(left, right []byte) int {
	return CTEqBit(left, right)
}

// CTEqU64Bit returns 1 when left == right and 0 otherwise.
//
// diff | -diff has its top bit set exactly when diff != 0 (for diff > 0,
// one of diff and its two's-complement negation has the top bit set), so
// shifting it down and flipping gives the equality bit:
//
//	diff = 0   0 | 0 = 0        >> 63 = 0   xor 1 = 1
//	diff ≠ 0   top bit set      >> 63 = 1   xor 1 = 0
func CTEqU64Bit(left, right uint64) int {
	diff := left ^ right
	return int(((diff | -diff) >> 63) ^ 1)
}
