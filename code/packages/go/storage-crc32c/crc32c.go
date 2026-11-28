// Package storagecrc32c implements the portable reflected Castagnoli checksum.
package storagecrc32c

const reflectedCastagnoli uint32 = 0x82f63b78

// Checksum returns the CRC-32C checksum of data, starting with a zero seed.
func Checksum(data []byte) uint32 {
	return Update(0, data)
}

// Update continues a CRC-32C checksum from previous over data.
func Update(previous uint32, data []byte) uint32 {
	register := ^previous
	for _, value := range data {
		register ^= uint32(value)
		for bit := 0; bit < 8; bit++ {
			if register&1 != 0 {
				register = (register >> 1) ^ reflectedCastagnoli
			} else {
				register >>= 1
			}
		}
	}
	return ^register
}
