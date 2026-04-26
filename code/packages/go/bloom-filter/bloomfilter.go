package bloomfilter

import (
	"errors"
	"fmt"
	"math"
)

const (
	defaultExpectedItems     = 1000
	defaultFalsePositiveRate = 0.01
)

var (
	ErrInvalidExpectedItems     = errors.New("expectedItems must be positive")
	ErrInvalidFalsePositiveRate = errors.New("falsePositiveRate must be in the open interval (0, 1)")
	ErrInvalidBitCount          = errors.New("bitCount must be positive")
	ErrInvalidHashCount         = errors.New("hashCount must be positive")
)

type BloomFilter struct {
	bitCount      int
	hashCount     int
	expectedItems int
	bits          []byte
	bitsSet       int
	itemsAdded    int
}

func Default() *BloomFilter {
	return MustNew(defaultExpectedItems, defaultFalsePositiveRate)
}

func New(expectedItems int, falsePositiveRate float64) (*BloomFilter, error) {
	if expectedItems <= 0 {
		return nil, fmt.Errorf("%w: %d", ErrInvalidExpectedItems, expectedItems)
	}
	if falsePositiveRate <= 0 || falsePositiveRate >= 1 || math.IsNaN(falsePositiveRate) {
		return nil, fmt.Errorf("%w: %f", ErrInvalidFalsePositiveRate, falsePositiveRate)
	}

	bitCount := OptimalM(expectedItems, falsePositiveRate)
	hashCount := OptimalK(bitCount, expectedItems)
	return fromParts(bitCount, hashCount, expectedItems), nil
}

func MustNew(expectedItems int, falsePositiveRate float64) *BloomFilter {
	filter, err := New(expectedItems, falsePositiveRate)
	if err != nil {
		panic(err)
	}
	return filter
}

func FromParams(bitCount int, hashCount int) (*BloomFilter, error) {
	if bitCount <= 0 {
		return nil, fmt.Errorf("%w: %d", ErrInvalidBitCount, bitCount)
	}
	if hashCount <= 0 {
		return nil, fmt.Errorf("%w: %d", ErrInvalidHashCount, hashCount)
	}
	return fromParts(bitCount, hashCount, 0), nil
}

func MustFromParams(bitCount int, hashCount int) *BloomFilter {
	filter, err := FromParams(bitCount, hashCount)
	if err != nil {
		panic(err)
	}
	return filter
}

func (bf *BloomFilter) Add(element any) {
	for _, idx := range bf.hashIndices(element) {
		byteIdx := idx / 8
		bitMask := byte(1 << (idx % 8))
		if bf.bits[byteIdx]&bitMask == 0 {
			bf.bits[byteIdx] |= bitMask
			bf.bitsSet++
		}
	}
	bf.itemsAdded++
}

func (bf *BloomFilter) Contains(element any) bool {
	for _, idx := range bf.hashIndices(element) {
		byteIdx := idx / 8
		bitMask := byte(1 << (idx % 8))
		if bf.bits[byteIdx]&bitMask == 0 {
			return false
		}
	}
	return true
}

func (bf *BloomFilter) BitCount() int {
	return bf.bitCount
}

func (bf *BloomFilter) HashCount() int {
	return bf.hashCount
}

func (bf *BloomFilter) BitsSet() int {
	return bf.bitsSet
}

func (bf *BloomFilter) FillRatio() float64 {
	if bf.bitCount == 0 {
		return 0
	}
	return float64(bf.bitsSet) / float64(bf.bitCount)
}

func (bf *BloomFilter) EstimatedFalsePositiveRate() float64 {
	if bf.bitsSet == 0 {
		return 0
	}
	return math.Pow(bf.FillRatio(), float64(bf.hashCount))
}

func (bf *BloomFilter) IsOverCapacity() bool {
	return bf.expectedItems > 0 && bf.itemsAdded > bf.expectedItems
}

func (bf *BloomFilter) SizeBytes() int {
	return len(bf.bits)
}

func (bf *BloomFilter) String() string {
	return fmt.Sprintf(
		"BloomFilter(m=%d, k=%d, bits_set=%d/%d (%.2f%%), ~fp=%.4f%%)",
		bf.bitCount,
		bf.hashCount,
		bf.bitsSet,
		bf.bitCount,
		bf.FillRatio()*100,
		bf.EstimatedFalsePositiveRate()*100,
	)
}

func OptimalM(expectedItems int, falsePositiveRate float64) int {
	return int(math.Ceil(-float64(expectedItems) * math.Log(falsePositiveRate) / math.Pow(math.Log(2), 2)))
}

func OptimalK(bitCount int, expectedItems int) int {
	return max(1, int(math.Round((float64(bitCount)/float64(expectedItems))*math.Log(2))))
}

func CapacityForMemory(memoryBytes int, falsePositiveRate float64) int {
	bitCount := memoryBytes * 8
	return int(math.Floor(-float64(bitCount) * math.Pow(math.Log(2), 2) / math.Log(falsePositiveRate)))
}

func fromParts(bitCount int, hashCount int, expectedItems int) *BloomFilter {
	return &BloomFilter{
		bitCount:      bitCount,
		hashCount:     hashCount,
		expectedItems: expectedItems,
		bits:          make([]byte, (bitCount+7)/8),
	}
}

func (bf *BloomFilter) hashIndices(element any) []int {
	raw := []byte(fmt.Sprint(element))
	h1 := fmix32(fnv1a32(raw))
	h2 := fmix32(djb2(raw))
	h2 |= 1

	indices := make([]int, 0, bf.hashCount)
	for i := 0; i < bf.hashCount; i++ {
		idx := (uint64(h1) + uint64(i)*uint64(h2)) % uint64(bf.bitCount)
		indices = append(indices, int(idx))
	}
	return indices
}

func fnv1a32(bytes []byte) uint32 {
	var hash uint32 = 0x811c9dc5
	for _, b := range bytes {
		hash ^= uint32(b)
		hash *= 0x01000193
	}
	return hash
}

func djb2(bytes []byte) uint32 {
	var hash uint32 = 5381
	for _, b := range bytes {
		hash = hash*33 + uint32(b)
	}
	return hash
}

func fmix32(h uint32) uint32 {
	h ^= h >> 16
	h *= 0x85ebca6b
	h ^= h >> 13
	h *= 0xc2b2ae35
	h ^= h >> 16
	return h
}
