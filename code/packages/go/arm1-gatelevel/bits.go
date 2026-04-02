// =========================================================================
// bits.go — Bit Conversion Helpers for 32-bit ARM1 Gate-Level Simulation
// =========================================================================
//
// Converts between integer values and bit slices (LSB-first). The ARM1
// uses 32-bit data paths, so most conversions use width=32.
//
// LSB-first ordering matches how ripple-carry adders process data:
// bit 0 feeds the first full adder, bit 1 feeds the second, etc.
//
//   IntToBits(5, 32) → [1, 0, 1, 0, 0, 0, ..., 0]  (32 elements)
//   BitsToInt(...)   → 5

package arm1gatelevel

// IntToBits converts a uint32 to a slice of 32 bits (LSB first).
//
// This is the bridge between the integer world (test programs, external API)
// and the gate-level world (slices of 0s and 1s flowing through gates).
func IntToBits(value uint32, width int) []int {
	result, _ := StartNew[[]int]("arm1-gatelevel.IntToBits", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			op.AddProperty("value", value)
			op.AddProperty("width", width)
			bits := make([]int, width)
			for i := 0; i < width; i++ {
				bits[i] = int((value >> i) & 1)
			}
			return rf.Generate(true, false, bits)
		}).GetResult()
	return result
}

// BitsToInt converts a slice of bits (LSB first) to a uint32.
func BitsToInt(bits []int) uint32 {
	result, _ := StartNew[uint32]("arm1-gatelevel.BitsToInt", 0,
		func(_ *Operation[uint32], rf *ResultFactory[uint32]) *OperationResult[uint32] {
			var res uint32
			for i, bit := range bits {
				if i >= 32 {
					break
				}
				res |= uint32(bit) << i
			}
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// BitsToIntSigned converts a slice of bits (LSB first) to a signed int32,
// treating the highest bit as the sign bit.
func BitsToIntSigned(bits []int) int32 {
	result, _ := StartNew[int32]("arm1-gatelevel.BitsToIntSigned", 0,
		func(_ *Operation[int32], rf *ResultFactory[int32]) *OperationResult[int32] {
			return rf.Generate(true, false, int32(BitsToInt(bits)))
		}).GetResult()
	return result
}
