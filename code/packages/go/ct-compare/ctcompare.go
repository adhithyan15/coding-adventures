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

func CTSelectBytes(left, right []byte, choice bool) []byte {
	if len(left) != len(right) {
		panic("ctselectbytes requires equal-length slices")
	}

	var mask byte
	if choice {
		mask = 0xFF
	}
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
