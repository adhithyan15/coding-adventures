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
