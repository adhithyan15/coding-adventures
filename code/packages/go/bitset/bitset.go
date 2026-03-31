// Package bitset provides a compact boolean array packed into 64-bit words.
//
// # What is a Bitset?
//
// A bitset stores a sequence of bits -- each one either 0 or 1 -- packed
// into machine-word-sized integers (uint64). Instead of using an entire
// byte to represent a single true/false value, a bitset packs 64 of them
// into a single word.
//
// Why does this matter?
//
//  1. Space: 10,000 booleans as []bool = 10,000 bytes.
//     As a bitset = ~1,250 bytes. That's an 8x improvement.
//
//  2. Speed: AND-ing two boolean slices loops over 10,000 elements.
//     AND-ing two bitsets loops over ~157 words. The CPU performs a single
//     64-bit AND instruction on each word, operating on 64 bits at once.
//
//  3. Ubiquity: Bitsets appear in Bloom filters, register allocators,
//     graph algorithms (visited sets), database bitmap indexes, filesystem
//     free-block bitmaps, network subnet masks, and garbage collectors.
//
// # Bit Ordering: LSB-First
//
// We use Least Significant Bit first ordering. Bit 0 is the least significant
// bit of word 0. Bit 63 is the most significant bit of word 0. Bit 64 is the
// least significant bit of word 1. And so on.
//
//	Word 0                              Word 1
//	┌─────────────────────────────┐     ┌─────────────────────────────┐
//	│ bit 63  ...  bit 2  bit 1  bit 0│ │ bit 127 ... bit 65  bit 64 │
//	└─────────────────────────────┘     └─────────────────────────────┘
//	MSB ◄─────────────────── LSB        MSB ◄─────────────────── LSB
//
// The three fundamental formulas that drive every bitset operation:
//
//	word_index = i / 64       (which word contains bit i?)
//	bit_offset = i % 64       (which position within that word?)
//	bitmask    = 1 << (i % 64)  (a mask with only bit i set)
//
// These are the heart of the entire implementation.
//
// # Operations
//
// Every public function is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery.
package bitset

import (
	"fmt"
	"math/bits"
	"strings"
)

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
//
// bitsPerWord is 64 because we use uint64 as our word type. Every formula in
// this package uses this constant rather than a magic number, so if someone
// ever wanted to experiment with uint32 words (32 bits), they'd only need to
// change this constant and the word type.

const bitsPerWord = 64

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------
//
// We have exactly one error type: BitsetError. It covers invalid binary
// strings passed to BitsetFromBinaryStr and overflow in ToInteger. This
// keeps the error surface minimal and focused.

// BitsetError represents an error that can occur when constructing or
// converting a bitset.
type BitsetError struct {
	// Message describes what went wrong.
	Message string
}

// Error implements the error interface.
func (e *BitsetError) Error() string {
	return e.Message
}

// ---------------------------------------------------------------------------
// The Bitset struct
// ---------------------------------------------------------------------------
//
// Internal Representation
// ~~~~~~~~~~~~~~~~~~~~~~~
//
// We store bits in a []uint64 called words. Each uint64 holds 64 bits.
// We also track length, the logical size -- the number of bits the user
// considers "addressable". The capacity is always len(words) * 64.
//
//	┌──────────────────────────────────────────────────────────────────┐
//	│                          capacity (256 bits = 4 words)           │
//	│                                                                  │
//	│  ┌──────────────────────────────────────────┐                    │
//	│  │              length (200 bits)             │ ··· unused ····  │
//	│  │  (highest addressable bit index + 1)       │ (always zero)   │
//	│  └──────────────────────────────────────────┘                    │
//	└──────────────────────────────────────────────────────────────────┘
//
// **Clean-trailing-bits invariant**: Bits beyond length in the last word are
// always zero. This is critical for correctness of Popcount, Any, All, None,
// equality, and ToInteger. Every operation that modifies the last word must
// clean trailing bits afterwards.

// Bitset is a compact data structure that packs boolean values into 64-bit
// words. It provides O(n/64) bulk bitwise operations (AND, OR, XOR, NOT),
// efficient iteration over set bits using trailing-zero-count, and
// ArrayList-style automatic growth when you set bits beyond the current size.
type Bitset struct {
	// words holds the packed bit storage. Each uint64 holds 64 bits.
	// words[0] holds bits 0-63, words[1] holds bits 64-127, etc.
	words []uint64

	// length is the logical size: the number of bits the user considers
	// addressable. Bits 0 through length-1 are "real". Bits from length
	// to capacity-1 exist in memory but are always zero (the
	// clean-trailing-bits invariant).
	length int
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------
//
// These small utility functions compute the word index, bit offset, and
// number of words needed for a given bit count. They're used throughout
// the implementation.

// wordsNeeded computes how many uint64 words we need to store bitCount bits.
//
// This is ceiling division: (bitCount + 63) / 64.
//
//	wordsNeeded(0)   = 0   (no bits, no words)
//	wordsNeeded(1)   = 1   (1 bit needs 1 word)
//	wordsNeeded(64)  = 1   (64 bits fit exactly in 1 word)
//	wordsNeeded(65)  = 2   (65 bits need 2 words)
//	wordsNeeded(128) = 2   (128 bits fit exactly in 2 words)
//	wordsNeeded(200) = 4   (200 bits need ceil(200/64) = 4 words)
func wordsNeeded(bitCount int) int {
	return (bitCount + bitsPerWord - 1) / bitsPerWord
}

// wordIndex computes which word contains bit i. Simply i / 64.
//
//	wordIndex(0)   = 0   (bit 0 is in word 0)
//	wordIndex(63)  = 0   (bit 63 is the last bit of word 0)
//	wordIndex(64)  = 1   (bit 64 is the first bit of word 1)
//	wordIndex(137) = 2   (bit 137 is in word 2)
func wordIndex(i int) int {
	return i / bitsPerWord
}

// bitOffset computes which bit position within its word bit i occupies.
// Simply i % 64.
//
//	bitOffset(0)   = 0
//	bitOffset(63)  = 63
//	bitOffset(64)  = 0   (first bit of the next word)
//	bitOffset(137) = 9   (137 - 2*64 = 9)
func bitOffset(i int) int {
	return i % bitsPerWord
}

// bitmask returns a uint64 mask with only bit i set within its word.
//
// This is 1 << (i % 64). We use this mask to isolate, set, clear,
// or toggle a single bit within a word using bitwise operations:
//
//	To set bit i:    word |= bitmask(i)     (OR with mask turns bit on)
//	To clear bit i:  word &= ^bitmask(i)    (AND with inverted mask turns bit off)
//	To test bit i:   (word & bitmask(i)) != 0  (AND with mask isolates the bit)
//	To toggle bit i: word ^= bitmask(i)     (XOR with mask flips the bit)
func bitmask(i int) uint64 {
	return 1 << uint(bitOffset(i))
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

// NewBitset creates a new bitset with all bits initially zero.
//
// The size parameter sets the logical length. The capacity is rounded up
// to the next multiple of 64.
//
// Example:
//
//	bs := NewBitset(100)
//	// bs.Len() == 100
//	// bs.Capacity() == 128  (2 words * 64 bits/word)
//	// bs.Popcount() == 0    (all bits start as zero)
//
// NewBitset(0) is valid and creates an empty bitset with length=0,
// capacity=0.
func NewBitset(size int) *Bitset {
	result, _ := StartNew[*Bitset]("bitset.NewBitset", nil,
		func(op *Operation[*Bitset], rf *ResultFactory[*Bitset]) *OperationResult[*Bitset] {
			op.AddProperty("size", size)
			return rf.Generate(true, false, &Bitset{
				words:  make([]uint64, wordsNeeded(size)),
				length: size,
			})
		}).GetResult()
	return result
}

// BitsetFromInteger creates a bitset from a non-negative integer.
//
// Bit 0 of the bitset is the least significant bit of value.
// The length of the result is the position of the highest set bit + 1.
// If value == 0, then length = 0.
//
// How it works:
//
//	value = 5  (binary: 101)
//	Highest set bit is at position 2
//	length = 64 - bits.LeadingZeros64(5) = 64 - 61 = 3
//	words = [5]
//
// Examples:
//
//	bs := BitsetFromInteger(5)  // binary: 101
//	// bs.Len() == 3
//	// bs.Test(0) == true   (bit 0 = 1)
//	// bs.Test(1) == false  (bit 1 = 0)
//	// bs.Test(2) == true   (bit 2 = 1)
func BitsetFromInteger(value uint64) *Bitset {
	result, _ := StartNew[*Bitset]("bitset.BitsetFromInteger", nil,
		func(op *Operation[*Bitset], rf *ResultFactory[*Bitset]) *OperationResult[*Bitset] {
			op.AddProperty("value", value)
			// Special case: zero produces an empty bitset.
			if value == 0 {
				return rf.Generate(true, false, NewBitset(0))
			}

			// The logical length is the position of the highest set bit + 1.
			// bits.Len64 returns the number of bits needed to represent value,
			// which is exactly position_of_highest_set_bit + 1.
			length := bits.Len64(value)

			return rf.Generate(true, false, &Bitset{
				words:  []uint64{value},
				length: length,
			})
		}).GetResult()
	return result
}

// BitsetFromBinaryStr creates a bitset from a string of '0' and '1'
// characters. The leftmost character is the highest-indexed bit
// (conventional binary notation, matching how humans write numbers).
// The rightmost character is bit 0.
//
// String-to-bits mapping:
//
//	Input string: "1 0 1 0"
//	Position:      3 2 1 0    (leftmost = highest bit index)
//
//	Bit 0 = '0' (rightmost char)
//	Bit 1 = '1'
//	Bit 2 = '0'
//	Bit 3 = '1' (leftmost char)
//
//	This is the same as the integer 10 (binary 1010).
//
// Returns a BitsetError if the string contains any character other than
// '0' or '1'.
//
// An empty string produces an empty bitset with length=0.
func BitsetFromBinaryStr(s string) (*Bitset, error) {
	return StartNew[*Bitset]("bitset.BitsetFromBinaryStr", nil,
		func(op *Operation[*Bitset], rf *ResultFactory[*Bitset]) *OperationResult[*Bitset] {
			// Validate: every character must be '0' or '1'.
			for _, ch := range s {
				if ch != '0' && ch != '1' {
					return rf.Fail(nil, &BitsetError{
						Message: fmt.Sprintf("invalid binary string: %q", s),
					})
				}
			}

			// Empty string produces an empty bitset.
			if len(s) == 0 {
				return rf.Generate(true, false, NewBitset(0))
			}

			// The string length is the logical length of the bitset.
			length := len(s)
			bs := NewBitset(length)

			// Walk the string from right to left (LSB to MSB).
			// The rightmost character (index len(s)-1) is bit 0.
			// The leftmost character (index 0) is bit len(s)-1.
			for i := 0; i < len(s); i++ {
				charIdx := len(s) - 1 - i // bit index (0 = rightmost = LSB)
				if s[charIdx] == '1' {
					wi := wordIndex(i)
					bs.words[wi] |= bitmask(i)
				}
			}

			// Clean trailing bits defensively.
			bs.cleanTrailingBits()

			return rf.Generate(true, false, bs)
		}).GetResult()
}

// ---------------------------------------------------------------------------
// Single-bit operations
// ---------------------------------------------------------------------------
//
// These are the bread-and-butter operations: set a bit, clear a bit,
// test whether a bit is set, toggle a bit. Each one translates to a
// single bitwise operation on the containing word.
//
// Growth semantics:
//   - Set(i) and Toggle(i) AUTO-GROW the bitset if i >= length.
//   - Test(i) and Clear(i) do NOT grow. They return false / do nothing
//     for out-of-range indices. This is safe because unallocated bits
//     are conceptually zero.

// Set sets bit i to 1. Auto-grows the bitset if i >= length.
//
// How auto-growth works:
//
// If i is beyond the current capacity, we double the capacity
// repeatedly until it's large enough (with a minimum of 64 bits).
// This is the same amortized O(1) strategy used by Go slices,
// Java's ArrayList, and Python's list.
//
//	Before: length=100, capacity=128 (2 words)
//	Set(200): 200 >= 128, so double: 128 -> 256. Now 200 < 256.
//	After: length=201, capacity=256 (4 words)
//
// The core operation uses OR to turn on the target bit:
//
//	words[2] = 0b...0000_0000
//	mask     = 0b...0010_0000   (bit 5 within the word)
//	result   = 0b...0010_0000   (bit 5 is now set)
//
// OR is idempotent: setting an already-set bit is a no-op.
func (b *Bitset) Set(i int) {
	_, _ = StartNew[struct{}]("bitset.Set", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("i", i)
			b.ensureCapacity(i)
			b.words[wordIndex(i)] |= bitmask(i)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Clear sets bit i to 0. No-op if i >= length (does not grow).
//
// Clearing a bit that's already 0 is a no-op. Clearing a bit beyond
// the bitset's length is also a no-op -- there's nothing to clear,
// because unallocated bits are conceptually zero.
//
// How it works:
//
// We AND the word with the inverted bitmask. The inverted mask has all
// bits set EXCEPT the target bit, so every other bit is preserved:
//
//	words[2] = 0b...0010_0100   (bits 2 and 5 set)
//	mask     = 0b...0010_0000   (bit 5)
//	^mask    = 0b...1101_1111   (everything except bit 5)
//	result   = 0b...0000_0100   (bit 5 cleared, bit 2 preserved)
func (b *Bitset) Clear(i int) {
	_, _ = StartNew[struct{}]("bitset.Clear", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("i", i)
			if i >= b.length {
				return rf.Generate(true, false, struct{}{}) // out of range: nothing to clear
			}
			b.words[wordIndex(i)] &^= bitmask(i)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Test returns whether bit i is set. Returns false if i >= length.
//
// This is a pure read operation -- it never modifies the bitset.
// Testing a bit beyond the bitset's length returns false because
// unallocated bits are conceptually zero.
//
// How it works:
//
// We AND the word with the bitmask. If the result is non-zero, the
// bit is set:
//
//	words[2] = 0b...0010_0100   (bits 2 and 5 set)
//	mask     = 0b...0010_0000   (bit 5)
//	result   = 0b...0010_0000   (non-zero -> bit 5 is set)
//
//	mask     = 0b...0000_1000   (bit 3)
//	result   = 0b...0000_0000   (zero -> bit 3 is not set)
func (b *Bitset) Test(i int) bool {
	result, _ := StartNew[bool]("bitset.Test", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("i", i)
			if i >= b.length {
				return rf.Generate(true, false, false) // out of range: conceptually zero
			}
			return rf.Generate(true, false, (b.words[wordIndex(i)]&bitmask(i)) != 0)
		}).GetResult()
	return result
}

// Toggle flips bit i (0 becomes 1, 1 becomes 0). Auto-grows if i >= length.
//
// How it works:
//
// XOR with the bitmask flips exactly one bit:
//
//	words[2] = 0b...0010_0100   (bits 2 and 5 set)
//	mask     = 0b...0010_0000   (bit 5)
//	result   = 0b...0000_0100   (bit 5 flipped to 0)
func (b *Bitset) Toggle(i int) {
	_, _ = StartNew[struct{}]("bitset.Toggle", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("i", i)
			b.ensureCapacity(i)
			b.words[wordIndex(i)] ^= bitmask(i)

			// Toggle might have set a bit in the last word's trailing region.
			// Clean trailing bits to maintain the invariant.
			b.cleanTrailingBits()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ---------------------------------------------------------------------------
// Bulk bitwise operations
// ---------------------------------------------------------------------------
//
// All bulk operations return a NEW bitset. They don't modify either
// operand. The result has length = max(a.length, b.length).
//
// When two bitsets have different lengths, the shorter one is
// "zero-extended" conceptually. In practice, we just stop reading
// from the shorter one's words once they run out and treat missing
// words as zero.
//
// Performance: each operation processes one 64-bit word per loop
// iteration, so 64 bits are handled in a single CPU instruction.
// This is the fundamental performance advantage of bitsets.

// And returns a new bitset where each bit is 1 only if BOTH corresponding
// input bits are 1.
//
// Truth table:
//
//	A  B  A&B
//	0  0   0
//	0  1   0
//	1  0   0
//	1  1   1
//
// AND is used for intersection: elements that are in both sets.
func (b *Bitset) And(other *Bitset) *Bitset {
	result, _ := StartNew[*Bitset]("bitset.And", nil,
		func(op *Operation[*Bitset], rf *ResultFactory[*Bitset]) *OperationResult[*Bitset] {
			resultLen := max(b.length, other.length)
			maxWords := max(len(b.words), len(other.words))
			resultWords := make([]uint64, maxWords)

			for i := 0; i < maxWords; i++ {
				a := wordAt(b.words, i)
				bw := wordAt(other.words, i)
				resultWords[i] = a & bw
			}

			res := &Bitset{words: resultWords, length: resultLen}
			res.cleanTrailingBits()
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Or returns a new bitset where each bit is 1 if EITHER (or both)
// corresponding input bits are 1.
//
// Truth table:
//
//	A  B  A|B
//	0  0   0
//	0  1   1
//	1  0   1
//	1  1   1
//
// OR is used for union: elements that are in either set.
func (b *Bitset) Or(other *Bitset) *Bitset {
	result, _ := StartNew[*Bitset]("bitset.Or", nil,
		func(op *Operation[*Bitset], rf *ResultFactory[*Bitset]) *OperationResult[*Bitset] {
			resultLen := max(b.length, other.length)
			maxWords := max(len(b.words), len(other.words))
			resultWords := make([]uint64, maxWords)

			for i := 0; i < maxWords; i++ {
				a := wordAt(b.words, i)
				bw := wordAt(other.words, i)
				resultWords[i] = a | bw
			}

			res := &Bitset{words: resultWords, length: resultLen}
			res.cleanTrailingBits()
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Xor returns a new bitset where each bit is 1 if the corresponding
// input bits DIFFER.
//
// Truth table:
//
//	A  B  A^B
//	0  0   0
//	0  1   1
//	1  0   1
//	1  1   0
//
// XOR is used for symmetric difference: elements in either set but not both.
func (b *Bitset) Xor(other *Bitset) *Bitset {
	result, _ := StartNew[*Bitset]("bitset.Xor", nil,
		func(op *Operation[*Bitset], rf *ResultFactory[*Bitset]) *OperationResult[*Bitset] {
			resultLen := max(b.length, other.length)
			maxWords := max(len(b.words), len(other.words))
			resultWords := make([]uint64, maxWords)

			for i := 0; i < maxWords; i++ {
				a := wordAt(b.words, i)
				bw := wordAt(other.words, i)
				resultWords[i] = a ^ bw
			}

			res := &Bitset{words: resultWords, length: resultLen}
			res.cleanTrailingBits()
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// Not returns a new bitset with every bit flipped within length.
//
// Truth table:
//
//	A  ~A
//	0   1
//	1   0
//
// NOT is used for complement: elements NOT in the set.
//
// Important: NOT flips bits within length, NOT within capacity.
// Bits beyond length remain zero (clean-trailing-bits invariant).
// The result has the same length as the input.
func (b *Bitset) Not() *Bitset {
	result, _ := StartNew[*Bitset]("bitset.Not", nil,
		func(op *Operation[*Bitset], rf *ResultFactory[*Bitset]) *OperationResult[*Bitset] {
			resultWords := make([]uint64, len(b.words))
			for i, w := range b.words {
				resultWords[i] = ^w
			}

			// Critical: clean trailing bits! The NOT operation flipped ALL bits
			// in every word, including the trailing bits beyond length that were
			// zero. We must zero them out again to maintain the invariant.
			//
			//     Before NOT: word[3] = 0b00000000_XXXXXXXX  (trailing bits are 0)
			//     After  NOT: word[3] = 0b11111111_xxxxxxxx  (trailing bits are 1!)
			//     After clean: word[3] = 0b00000000_xxxxxxxx  (trailing bits zeroed)
			res := &Bitset{words: resultWords, length: b.length}
			res.cleanTrailingBits()
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// AndNot returns a new bitset with bits in b that are NOT in other
// (set difference).
//
// This is equivalent to b & (^other), but more efficient because
// we don't need to create an intermediate NOT result.
//
// Truth table:
//
//	A  B  A & ^B
//	0  0    0
//	0  1    0
//	1  0    1
//	1  1    0
//
// AND-NOT is used for set difference: elements in A but not in B.
func (b *Bitset) AndNot(other *Bitset) *Bitset {
	result, _ := StartNew[*Bitset]("bitset.AndNot", nil,
		func(op *Operation[*Bitset], rf *ResultFactory[*Bitset]) *OperationResult[*Bitset] {
			resultLen := max(b.length, other.length)
			maxWords := max(len(b.words), len(other.words))
			resultWords := make([]uint64, maxWords)

			for i := 0; i < maxWords; i++ {
				a := wordAt(b.words, i)
				bw := wordAt(other.words, i)
				// a &^ bw: keep bits from a that are NOT in bw
				// Go's &^ operator is "AND NOT" (bit clear).
				resultWords[i] = a &^ bw
			}

			res := &Bitset{words: resultWords, length: resultLen}
			res.cleanTrailingBits()
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// ---------------------------------------------------------------------------
// Counting and query operations
// ---------------------------------------------------------------------------

// Popcount returns the number of set (1) bits. Named after the CPU
// instruction POPCNT (population count) that does this for a single word.
//
// We call bits.OnesCount64 on each word and sum the results. Go's
// bits.OnesCount64 compiles to the hardware POPCNT instruction on
// modern x86 CPUs, making this extremely fast.
//
// For a bitset with N bits, this runs in O(N/64) time -- we process
// 64 bits per loop iteration.
func (b *Bitset) Popcount() int {
	result, _ := StartNew[int]("bitset.Popcount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			count := 0
			for _, w := range b.words {
				count += bits.OnesCount64(w)
			}
			return rf.Generate(true, false, count)
		}).GetResult()
	return result
}

// Len returns the logical length: the number of addressable bits.
//
// This is the value passed to NewBitset(size), or the highest bit index + 1
// after any auto-growth operations.
func (b *Bitset) Len() int {
	result, _ := StartNew[int]("bitset.Len", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, b.length)
		}).GetResult()
	return result
}

// Capacity returns the allocated size in bits (always a multiple of 64).
//
// Capacity >= Len(). The difference (Capacity - Len) is "slack space" --
// bits that exist in memory but are always zero.
func (b *Bitset) Capacity() int {
	result, _ := StartNew[int]("bitset.Capacity", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(b.words)*bitsPerWord)
		}).GetResult()
	return result
}

// Any returns true if at least one bit is set.
//
// Short-circuits: returns as soon as it finds a non-zero word,
// without scanning the rest. This is O(1) in the best case
// (first word is non-zero) and O(N/64) in the worst case.
func (b *Bitset) Any() bool {
	result, _ := StartNew[bool]("bitset.Any", false,
		func(_ *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			for _, w := range b.words {
				if w != 0 {
					return rf.Generate(true, false, true)
				}
			}
			return rf.Generate(true, false, false)
		}).GetResult()
	return result
}

// All returns true if ALL bits in 0..length are set.
//
// For an empty bitset (length = 0), returns true -- this is vacuous truth,
// the same convention used by Python's all([]), Go's behavior for empty
// ranges, and mathematical logic ("for all x in {}, P(x)" is true).
//
// How it works:
//
// For each full word (words 0 through second-to-last), we check if
// every bit is set (word == ^uint64(0), i.e., all 64 bits are 1).
//
// For the last word, we only check the bits within length. We create
// a mask of the valid bits and check that all valid bits are set.
func (b *Bitset) All() bool {
	result, _ := StartNew[bool]("bitset.All", false,
		func(_ *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			// Vacuous truth: all bits of nothing are set.
			if b.length == 0 {
				return rf.Generate(true, false, true)
			}

			numWords := len(b.words)

			// Check all full words (all bits must be 1 = max uint64).
			for i := 0; i < numWords-1; i++ {
				if b.words[i] != ^uint64(0) {
					return rf.Generate(true, false, false)
				}
			}

			// Check the last word: only the bits within length matter.
			remaining := bitOffset(b.length)
			if remaining == 0 {
				// length is a multiple of 64, so the last word is a full word.
				return rf.Generate(true, false, b.words[numWords-1] == ^uint64(0))
			}

			// Create a mask for the valid bits: (1 << remaining) - 1
			// Example: remaining = 8 -> mask = 0xFF (bits 0-7)
			mask := (uint64(1) << uint(remaining)) - 1
			return rf.Generate(true, false, b.words[numWords-1] == mask)
		}).GetResult()
	return result
}

// None returns true if no bits are set. Equivalent to !Any().
func (b *Bitset) None() bool {
	result, _ := StartNew[bool]("bitset.None", false,
		func(_ *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, !b.Any())
		}).GetResult()
	return result
}

// ---------------------------------------------------------------------------
// Iteration
// ---------------------------------------------------------------------------

// IterSetBits returns the indices of all set bits in ascending order.
//
// How it works: trailing-zero-count trick
//
// For each non-zero word, we use bits.TrailingZeros64 to find the lowest
// set bit, record its index, then clear it with word &= word - 1:
//
//	word = 0b10100100   (bits 2, 5, 7 are set)
//
//	Step 1: trailing_zeros = 2  -> record base + 2
//	        word &= word - 1   -> 0b10100000  (clear bit 2)
//
//	Step 2: trailing_zeros = 5  -> record base + 5
//	        word &= word - 1   -> 0b10000000  (clear bit 5)
//
//	Step 3: trailing_zeros = 7  -> record base + 7
//	        word &= word - 1   -> 0b00000000  (clear bit 7)
//
//	word == 0, move to next word.
//
// The trick word &= word - 1 clears the lowest set bit. Here's why:
//
//	word     = 0b10100100
//	word - 1 = 0b10100011  (borrow propagates through trailing zeros)
//	AND      = 0b10100000  (lowest set bit is cleared)
//
// This is O(k) where k is the number of set bits, and it skips zero
// words entirely, making it very efficient for sparse bitsets.
func (b *Bitset) IterSetBits() []int {
	result, _ := StartNew[[]int]("bitset.IterSetBits", nil,
		func(_ *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			indices := make([]int, 0)

			for wordIdx, w := range b.words {
				baseIndex := wordIdx * bitsPerWord
				// Process each set bit in this word using the trailing-zeros trick.
				for w != 0 {
					// Find the lowest set bit.
					bitPos := bits.TrailingZeros64(w)
					index := baseIndex + bitPos

					// Only include bits within length (don't include trailing garbage).
					if index >= b.length {
						break
					}

					indices = append(indices, index)

					// Clear the lowest set bit: word &= word - 1
					w &= w - 1
				}
			}

			return rf.Generate(true, false, indices)
		}).GetResult()
	return result
}

// ---------------------------------------------------------------------------
// Conversion operations
// ---------------------------------------------------------------------------

// ToInteger converts the bitset to a uint64 integer.
//
// Returns an error if the bitset has set bits beyond position 63
// (i.e., it requires more than one word to represent).
//
// Returns 0 for an empty bitset.
func (b *Bitset) ToInteger() (uint64, error) {
	return StartNew[uint64]("bitset.ToInteger", 0,
		func(_ *Operation[uint64], rf *ResultFactory[uint64]) *OperationResult[uint64] {
			// Empty bitset = 0.
			if len(b.words) == 0 {
				return rf.Generate(true, false, uint64(0))
			}

			// Check that all words beyond the first are zero.
			for i := 1; i < len(b.words); i++ {
				if b.words[i] != 0 {
					return rf.Fail(uint64(0), &BitsetError{
						Message: "bitset value exceeds uint64 range",
					})
				}
			}

			return rf.Generate(true, false, b.words[0])
		}).GetResult()
}

// ToBinaryStr converts the bitset to a string of '0' and '1' characters
// with the highest bit on the left (conventional binary notation).
//
// This is the inverse of BitsetFromBinaryStr. An empty bitset produces
// an empty string "".
//
// Example:
//
//	bs := BitsetFromInteger(5)  // binary 101
//	bs.ToBinaryStr()            // returns "101"
func (b *Bitset) ToBinaryStr() string {
	result, _ := StartNew[string]("bitset.ToBinaryStr", "",
		func(_ *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			if b.length == 0 {
				return rf.Generate(true, false, "")
			}

			// Build the string from the highest bit (length-1) down to bit 0.
			// This produces conventional binary notation: MSB on the left.
			var sb strings.Builder
			sb.Grow(b.length)
			for i := b.length - 1; i >= 0; i-- {
				if b.Test(i) {
					sb.WriteByte('1')
				} else {
					sb.WriteByte('0')
				}
			}
			return rf.Generate(true, false, sb.String())
		}).GetResult()
	return result
}

// String returns a human-readable representation like "Bitset(101)".
//
// This implements the fmt.Stringer interface so bitsets print nicely
// with fmt.Println, fmt.Sprintf, etc.
func (b *Bitset) String() string {
	result, _ := StartNew[string]("bitset.String", "",
		func(_ *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, fmt.Sprintf("Bitset(%s)", b.ToBinaryStr()))
		}).GetResult()
	return result
}

// ---------------------------------------------------------------------------
// Equality
// ---------------------------------------------------------------------------

// Equal returns true if two bitsets have the same length and the same bits
// set. Capacity is irrelevant to equality -- a bitset with capacity=128
// can equal one with capacity=256 if their length and set bits match.
//
// Thanks to the clean-trailing-bits invariant, we can compare words
// directly -- trailing bits are always zero, so two bitsets with the same
// logical content will have identical word vectors (up to the number of
// words needed for the longer one).
func (b *Bitset) Equal(other *Bitset) bool {
	result, _ := StartNew[bool]("bitset.Equal", false,
		func(_ *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			if b.length != other.length {
				return rf.Generate(true, false, false)
			}

			// Compare word-by-word. If one has more words allocated, the
			// extra words must all be zero (due to clean-trailing-bits).
			maxWords := max(len(b.words), len(other.words))
			for i := 0; i < maxWords; i++ {
				a := wordAt(b.words, i)
				bw := wordAt(other.words, i)
				if a != bw {
					return rf.Generate(true, false, false)
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

// ensureCapacity ensures the bitset has capacity for bit i. If not, grow
// by doubling.
//
// After this call, i < capacity and length >= i + 1.
//
// Growth strategy:
//
// We double the capacity repeatedly until it exceeds i. The minimum
// capacity after growth is 64 (one word). This doubling strategy gives
// amortized O(1) growth, just like Go's append.
//
//	Example: capacity=128, Set(500)
//	  128 -> 256 -> 512 -> 1024  (stop: 500 < 1024)
func (b *Bitset) ensureCapacity(i int) {
	if i < b.Capacity() {
		// Already have room. But we might need to update length.
		if i >= b.length {
			b.length = i + 1
		}
		return
	}

	// Need to grow. Start with current capacity (or 64 as minimum).
	newCap := b.Capacity()
	if newCap < bitsPerWord {
		newCap = bitsPerWord
	}
	for newCap <= i {
		newCap *= 2
	}

	// Extend the word slice with zeros.
	newWordCount := newCap / bitsPerWord
	newWords := make([]uint64, newWordCount)
	copy(newWords, b.words)
	b.words = newWords

	// Update length to include the new bit.
	b.length = i + 1
}

// cleanTrailingBits zeroes out any bits beyond length in the last word.
//
// This maintains the clean-trailing-bits invariant. It must be called
// after any operation that might set bits beyond length:
//   - Not() flips all bits, including trailing ones
//   - Toggle() on the last word
//   - Bulk operations (AND, OR, XOR) when operands have different sizes
//
// How it works:
//
//	length = 200, capacity = 256
//	The last word holds bits 192-255, but only 192-199 are "real".
//	remaining = 200 % 64 = 8
//	mask = (1 << 8) - 1 = 0xFF  (bits 0-7)
//	words[3] &= 0xFF  -> zeroes out bits 8-63 of word 3
//
// If length is a multiple of 64, there are no trailing bits to clean.
func (b *Bitset) cleanTrailingBits() {
	if b.length == 0 || len(b.words) == 0 {
		return
	}

	remaining := bitOffset(b.length)
	if remaining != 0 {
		lastIdx := len(b.words) - 1
		mask := (uint64(1) << uint(remaining)) - 1
		b.words[lastIdx] &= mask
	}
}

// wordAt safely gets a word from a slice, returning 0 if the index is
// out of bounds. This simplifies bulk operations between bitsets of
// different sizes -- missing words are treated as zero.
func wordAt(words []uint64, i int) uint64 {
	if i < len(words) {
		return words[i]
	}
	return 0
}

// max returns the larger of two ints.
// (Go 1.21+ has built-in max, but we define it for clarity.)
func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
