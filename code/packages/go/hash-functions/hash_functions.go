// Package hashfunctions implements non-cryptographic hash functions from
// scratch. The algorithms here are building blocks for hash tables, Bloom
// filters, sketches, and parser/runtime experiments.
package hashfunctions

import (
	"math/big"
	"math/bits"
)

const (
	Djb2OffsetBasis                 uint64 = 5381
	Fnv32OffsetBasis                uint32 = 0x811c9dc5
	Fnv32Prime                      uint32 = 0x01000193
	Fnv64OffsetBasis                uint64 = 0xcbf29ce484222325
	Fnv64Prime                      uint64 = 0x00000100000001b3
	PolynomialRollingDefaultBase    uint64 = 31
	PolynomialRollingDefaultModulus uint64 = (1 << 61) - 1
)

const (
	murmur3C1 uint32 = 0xcc9e2d51
	murmur3C2 uint32 = 0x1b873593
)

// Fnv1a32 computes the 32-bit Fowler-Noll-Vo FNV-1a hash.
func Fnv1a32(data []byte) uint32 {
	hash := Fnv32OffsetBasis
	for _, b := range data {
		hash ^= uint32(b)
		hash *= Fnv32Prime
	}
	return hash
}

// Fnv1a64 computes the 64-bit Fowler-Noll-Vo FNV-1a hash.
func Fnv1a64(data []byte) uint64 {
	hash := Fnv64OffsetBasis
	for _, b := range data {
		hash ^= uint64(b)
		hash *= Fnv64Prime
	}
	return hash
}

// Djb2 computes Dan Bernstein's shift-and-add hash, bounded to 64 bits.
func Djb2(data []byte) uint64 {
	hash := Djb2OffsetBasis
	for _, b := range data {
		hash = (hash << 5) + hash + uint64(b)
	}
	return hash
}

// PolynomialRolling computes the default polynomial rolling hash.
func PolynomialRolling(data []byte) uint64 {
	return PolynomialRollingWithParams(
		data,
		PolynomialRollingDefaultBase,
		PolynomialRollingDefaultModulus,
	)
}

// PolynomialRollingWithParams evaluates the byte polynomial modulo modulus.
func PolynomialRollingWithParams(data []byte, base uint64, modulus uint64) uint64 {
	if modulus == 0 {
		panic("modulus must be positive")
	}

	hash := big.NewInt(0)
	baseInt := new(big.Int).SetUint64(base)
	modulusInt := new(big.Int).SetUint64(modulus)
	byteInt := new(big.Int)

	for _, b := range data {
		hash.Mul(hash, baseInt)
		hash.Add(hash, byteInt.SetUint64(uint64(b)))
		hash.Mod(hash, modulusInt)
	}

	return hash.Uint64()
}

// Murmur3_32 computes Austin Appleby's MurmurHash3 32-bit variant.
func Murmur3_32(data []byte) uint32 {
	return Murmur3_32WithSeed(data, 0)
}

// Murmur3_32WithSeed computes MurmurHash3 with an explicit 32-bit seed.
func Murmur3_32WithSeed(data []byte, seed uint32) uint32 {
	hash := seed
	blockCount := len(data) / 4

	for blockIndex := 0; blockIndex < blockCount; blockIndex++ {
		offset := blockIndex * 4
		k := uint32(data[offset]) |
			uint32(data[offset+1])<<8 |
			uint32(data[offset+2])<<16 |
			uint32(data[offset+3])<<24

		k *= murmur3C1
		k = bits.RotateLeft32(k, 15)
		k *= murmur3C2

		hash ^= k
		hash = bits.RotateLeft32(hash, 13)
		hash = hash*5 + 0xe6546b64
	}

	tailOffset := blockCount * 4
	var k uint32
	switch len(data) & 3 {
	case 3:
		k ^= uint32(data[tailOffset+2]) << 16
		fallthrough
	case 2:
		k ^= uint32(data[tailOffset+1]) << 8
		fallthrough
	case 1:
		k ^= uint32(data[tailOffset])
		k *= murmur3C1
		k = bits.RotateLeft32(k, 15)
		k *= murmur3C2
		hash ^= k
	}

	hash ^= uint32(len(data))
	return fmix32(hash)
}

// AvalancheScore estimates the fraction of output bits that flip after a
// one-bit input change. A strong mixing function should land near 0.5.
func AvalancheScore(hashFn func([]byte) uint64, outputBits int, sampleSize int) float64 {
	if outputBits <= 0 || outputBits > 64 {
		panic("outputBits must be in 1..64")
	}
	if sampleSize <= 0 {
		panic("sampleSize must be positive")
	}

	var totalBitFlips uint64
	var totalTrials uint64
	for sampleIndex := 0; sampleIndex < sampleSize; sampleIndex++ {
		input := deterministicBytes(sampleIndex)
		original := hashFn(input)
		for bitPosition := 0; bitPosition < len(input)*8; bitPosition++ {
			flipped := append([]byte(nil), input...)
			flipped[bitPosition/8] ^= 1 << (bitPosition % 8)
			totalBitFlips += uint64(bits.OnesCount64(original ^ hashFn(flipped)))
			totalTrials += uint64(outputBits)
		}
	}

	return float64(totalBitFlips) / float64(totalTrials)
}

// DistributionTest returns the chi-squared statistic for bucket distribution.
func DistributionTest(hashFn func([]byte) uint64, inputs [][]byte, numBuckets int) float64 {
	if numBuckets <= 0 {
		panic("numBuckets must be positive")
	}
	if len(inputs) == 0 {
		panic("inputs must not be empty")
	}

	counts := make([]uint64, numBuckets)
	for _, input := range inputs {
		bucket := hashFn(input) % uint64(numBuckets)
		counts[bucket]++
	}

	expected := float64(len(inputs)) / float64(numBuckets)
	var chi2 float64
	for _, observed := range counts {
		delta := float64(observed) - expected
		chi2 += delta * delta / expected
	}
	return chi2
}

func fmix32(hash uint32) uint32 {
	hash ^= hash >> 16
	hash *= 0x85ebca6b
	hash ^= hash >> 13
	hash *= 0xc2b2ae35
	hash ^= hash >> 16
	return hash
}

func deterministicBytes(sampleIndex int) []byte {
	state := uint32(0x9e3779b9) ^ uint32(sampleIndex)
	result := make([]byte, 8)
	for index := range result {
		state = state*1664525 + 1013904223
		result[index] = byte(state)
	}
	return result
}
