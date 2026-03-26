package bitset

import (
	"testing"
)

// ===========================================================================
// Constructor tests
// ===========================================================================

// --- NewBitset ---

func TestNewBitsetZero(t *testing.T) {
	// NewBitset(0) creates an empty bitset with no words allocated.
	bs := NewBitset(0)
	if bs.Len() != 0 {
		t.Errorf("expected Len()=0, got %d", bs.Len())
	}
	if bs.Capacity() != 0 {
		t.Errorf("expected Capacity()=0, got %d", bs.Capacity())
	}
	if bs.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", bs.Popcount())
	}
}

func TestNewBitsetSmall(t *testing.T) {
	// A bitset of size 10 should have length=10 and capacity rounded to 64.
	bs := NewBitset(10)
	if bs.Len() != 10 {
		t.Errorf("expected Len()=10, got %d", bs.Len())
	}
	if bs.Capacity() != 64 {
		t.Errorf("expected Capacity()=64, got %d", bs.Capacity())
	}
	// All bits start as zero.
	for i := 0; i < 10; i++ {
		if bs.Test(i) {
			t.Errorf("expected bit %d to be 0", i)
		}
	}
}

func TestNewBitsetExactMultiple(t *testing.T) {
	// 128 bits = exactly 2 words.
	bs := NewBitset(128)
	if bs.Len() != 128 {
		t.Errorf("expected Len()=128, got %d", bs.Len())
	}
	if bs.Capacity() != 128 {
		t.Errorf("expected Capacity()=128, got %d", bs.Capacity())
	}
}

func TestNewBitsetLarge(t *testing.T) {
	// 200 bits requires ceil(200/64) = 4 words = 256 capacity.
	bs := NewBitset(200)
	if bs.Len() != 200 {
		t.Errorf("expected Len()=200, got %d", bs.Len())
	}
	if bs.Capacity() != 256 {
		t.Errorf("expected Capacity()=256, got %d", bs.Capacity())
	}
}

// --- BitsetFromInteger ---

func TestFromIntegerZero(t *testing.T) {
	bs := BitsetFromInteger(0)
	if bs.Len() != 0 {
		t.Errorf("expected Len()=0, got %d", bs.Len())
	}
	if bs.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", bs.Popcount())
	}
}

func TestFromIntegerFive(t *testing.T) {
	// 5 = binary 101, highest bit at position 2, so length=3.
	bs := BitsetFromInteger(5)
	if bs.Len() != 3 {
		t.Errorf("expected Len()=3, got %d", bs.Len())
	}
	if !bs.Test(0) {
		t.Error("expected bit 0 to be set")
	}
	if bs.Test(1) {
		t.Error("expected bit 1 to be clear")
	}
	if !bs.Test(2) {
		t.Error("expected bit 2 to be set")
	}
	if bs.Popcount() != 2 {
		t.Errorf("expected Popcount()=2, got %d", bs.Popcount())
	}
}

func TestFromIntegerOne(t *testing.T) {
	bs := BitsetFromInteger(1)
	if bs.Len() != 1 {
		t.Errorf("expected Len()=1, got %d", bs.Len())
	}
	if !bs.Test(0) {
		t.Error("expected bit 0 to be set")
	}
}

func TestFromIntegerPowerOfTwo(t *testing.T) {
	// 2^63 = only the highest bit of a single word.
	bs := BitsetFromInteger(1 << 63)
	if bs.Len() != 64 {
		t.Errorf("expected Len()=64, got %d", bs.Len())
	}
	if !bs.Test(63) {
		t.Error("expected bit 63 to be set")
	}
	if bs.Popcount() != 1 {
		t.Errorf("expected Popcount()=1, got %d", bs.Popcount())
	}
}

func TestFromIntegerMaxUint64(t *testing.T) {
	bs := BitsetFromInteger(^uint64(0))
	if bs.Len() != 64 {
		t.Errorf("expected Len()=64, got %d", bs.Len())
	}
	if bs.Popcount() != 64 {
		t.Errorf("expected Popcount()=64, got %d", bs.Popcount())
	}
}

// --- BitsetFromBinaryStr ---

func TestFromBinaryStrEmpty(t *testing.T) {
	bs, err := BitsetFromBinaryStr("")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if bs.Len() != 0 {
		t.Errorf("expected Len()=0, got %d", bs.Len())
	}
}

func TestFromBinaryStrSimple(t *testing.T) {
	// "1010" -> bit 3=1, bit 2=0, bit 1=1, bit 0=0
	bs, err := BitsetFromBinaryStr("1010")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if bs.Len() != 4 {
		t.Errorf("expected Len()=4, got %d", bs.Len())
	}
	if bs.Test(0) {
		t.Error("expected bit 0 to be clear")
	}
	if !bs.Test(1) {
		t.Error("expected bit 1 to be set")
	}
	if bs.Test(2) {
		t.Error("expected bit 2 to be clear")
	}
	if !bs.Test(3) {
		t.Error("expected bit 3 to be set")
	}

	// Verify it's equivalent to from_integer(10).
	val, err := bs.ToInteger()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if val != 10 {
		t.Errorf("expected ToInteger()=10, got %d", val)
	}
}

func TestFromBinaryStrAllOnes(t *testing.T) {
	bs, err := BitsetFromBinaryStr("1111")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if bs.Popcount() != 4 {
		t.Errorf("expected Popcount()=4, got %d", bs.Popcount())
	}
	if !bs.All() {
		t.Error("expected All() to be true")
	}
}

func TestFromBinaryStrAllZeros(t *testing.T) {
	bs, err := BitsetFromBinaryStr("0000")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if bs.Len() != 4 {
		t.Errorf("expected Len()=4, got %d", bs.Len())
	}
	if bs.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", bs.Popcount())
	}
}

func TestFromBinaryStrInvalid(t *testing.T) {
	_, err := BitsetFromBinaryStr("102")
	if err == nil {
		t.Error("expected error for invalid binary string")
	}
	bitsetErr, ok := err.(*BitsetError)
	if !ok {
		t.Errorf("expected *BitsetError, got %T", err)
	}
	if bitsetErr == nil {
		t.Fatal("bitsetErr is nil")
	}
}

func TestFromBinaryStrInvalidLetters(t *testing.T) {
	_, err := BitsetFromBinaryStr("abc")
	if err == nil {
		t.Error("expected error for non-binary string")
	}
}

func TestFromBinaryStrLong(t *testing.T) {
	// A string longer than 64 characters to exercise multi-word paths.
	s := "1" + string(make([]byte, 99)) // won't work, let's build it properly
	_ = s
	// Build a 100-char binary string: "1" followed by 99 "0"s.
	str := "1" + repeatChar('0', 99)
	bs, err := BitsetFromBinaryStr(str)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if bs.Len() != 100 {
		t.Errorf("expected Len()=100, got %d", bs.Len())
	}
	// Only the highest bit (bit 99) should be set.
	if !bs.Test(99) {
		t.Error("expected bit 99 to be set")
	}
	if bs.Popcount() != 1 {
		t.Errorf("expected Popcount()=1, got %d", bs.Popcount())
	}
}

// repeatChar returns a string of n copies of the given byte.
func repeatChar(ch byte, n int) string {
	b := make([]byte, n)
	for i := range b {
		b[i] = ch
	}
	return string(b)
}

// ===========================================================================
// Single-bit operation tests
// ===========================================================================

func TestSetAndTest(t *testing.T) {
	bs := NewBitset(100)
	bs.Set(0)
	bs.Set(42)
	bs.Set(99)

	if !bs.Test(0) {
		t.Error("expected bit 0 to be set")
	}
	if !bs.Test(42) {
		t.Error("expected bit 42 to be set")
	}
	if !bs.Test(99) {
		t.Error("expected bit 99 to be set")
	}
	if bs.Test(1) {
		t.Error("expected bit 1 to be clear")
	}
	if bs.Popcount() != 3 {
		t.Errorf("expected Popcount()=3, got %d", bs.Popcount())
	}
}

func TestSetIdempotent(t *testing.T) {
	// Setting a bit that's already set should be a no-op.
	bs := NewBitset(10)
	bs.Set(5)
	bs.Set(5)
	if bs.Popcount() != 1 {
		t.Errorf("expected Popcount()=1, got %d", bs.Popcount())
	}
}

func TestSetAutoGrow(t *testing.T) {
	// Setting a bit beyond len should grow the bitset.
	bs := NewBitset(10)
	bs.Set(100)
	if bs.Len() != 101 {
		t.Errorf("expected Len()=101, got %d", bs.Len())
	}
	if !bs.Test(100) {
		t.Error("expected bit 100 to be set")
	}
	// Capacity should have grown to accommodate bit 100.
	if bs.Capacity() < 101 {
		t.Errorf("expected Capacity() >= 101, got %d", bs.Capacity())
	}
}

func TestSetAutoGrowFromEmpty(t *testing.T) {
	// Growing from an empty bitset.
	bs := NewBitset(0)
	bs.Set(3)
	if bs.Len() != 4 {
		t.Errorf("expected Len()=4, got %d", bs.Len())
	}
	if bs.Capacity() != 64 {
		t.Errorf("expected Capacity()=64, got %d", bs.Capacity())
	}
	if !bs.Test(3) {
		t.Error("expected bit 3 to be set")
	}
}

func TestClear(t *testing.T) {
	bs := NewBitset(10)
	bs.Set(5)
	if !bs.Test(5) {
		t.Error("expected bit 5 to be set")
	}
	bs.Clear(5)
	if bs.Test(5) {
		t.Error("expected bit 5 to be clear after Clear")
	}
}

func TestClearIdempotent(t *testing.T) {
	// Clearing a bit that's already clear should be a no-op.
	bs := NewBitset(10)
	bs.Clear(5) // already 0
	if bs.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", bs.Popcount())
	}
}

func TestClearBeyondLen(t *testing.T) {
	// Clearing beyond len should be a no-op (no growth).
	bs := NewBitset(10)
	bs.Clear(999)
	if bs.Len() != 10 {
		t.Errorf("expected Len()=10, got %d", bs.Len())
	}
}

func TestTestBeyondLen(t *testing.T) {
	// Testing beyond len returns false (no growth).
	bs := NewBitset(10)
	if bs.Test(999) {
		t.Error("expected Test(999) to be false")
	}
	if bs.Len() != 10 {
		t.Errorf("expected Len()=10, got %d", bs.Len())
	}
}

func TestToggle(t *testing.T) {
	bs := NewBitset(10)

	// Toggle 0 -> 1.
	bs.Toggle(5)
	if !bs.Test(5) {
		t.Error("expected bit 5 to be set after toggle")
	}

	// Toggle 1 -> 0.
	bs.Toggle(5)
	if bs.Test(5) {
		t.Error("expected bit 5 to be clear after second toggle")
	}
}

func TestToggleAutoGrow(t *testing.T) {
	bs := NewBitset(10)
	bs.Toggle(200)
	if bs.Len() != 201 {
		t.Errorf("expected Len()=201, got %d", bs.Len())
	}
	if !bs.Test(200) {
		t.Error("expected bit 200 to be set")
	}
}

func TestToggleAllBitsInWord(t *testing.T) {
	// Toggle every bit in a 64-bit bitset, then toggle them all back.
	bs := NewBitset(64)
	for i := 0; i < 64; i++ {
		bs.Toggle(i)
	}
	if bs.Popcount() != 64 {
		t.Errorf("expected Popcount()=64, got %d", bs.Popcount())
	}
	for i := 0; i < 64; i++ {
		bs.Toggle(i)
	}
	if bs.Popcount() != 0 {
		t.Errorf("expected Popcount()=0 after double toggle, got %d", bs.Popcount())
	}
}

// ===========================================================================
// Bulk bitwise operation tests
// ===========================================================================

func TestAndSameSize(t *testing.T) {
	a := BitsetFromInteger(0b1100) // bits 2,3
	b := BitsetFromInteger(0b1010) // bits 1,3
	c := a.And(b)

	val, err := c.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	// Only bit 3 is in both.
	if val != 0b1000 {
		t.Errorf("expected 0b1000, got 0b%b", val)
	}
}

func TestAndDifferentSizes(t *testing.T) {
	a := NewBitset(200)
	a.Set(0)
	a.Set(100)
	a.Set(150)

	b := NewBitset(100)
	b.Set(0)
	b.Set(50)

	c := a.And(b)
	// Only bit 0 is set in both.
	if c.Popcount() != 1 {
		t.Errorf("expected Popcount()=1, got %d", c.Popcount())
	}
	if !c.Test(0) {
		t.Error("expected bit 0 to be set")
	}
	if c.Len() != 200 {
		t.Errorf("expected Len()=200, got %d", c.Len())
	}
}

func TestOrSameSize(t *testing.T) {
	a := BitsetFromInteger(0b1100) // bits 2,3
	b := BitsetFromInteger(0b1010) // bits 1,3
	c := a.Or(b)

	val, err := c.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	// bits 1,2,3
	if val != 0b1110 {
		t.Errorf("expected 0b1110, got 0b%b", val)
	}
}

func TestOrDifferentSizes(t *testing.T) {
	a := NewBitset(200)
	a.Set(0)
	a.Set(150)

	b := NewBitset(50)
	b.Set(0)
	b.Set(30)

	c := a.Or(b)
	if c.Len() != 200 {
		t.Errorf("expected Len()=200, got %d", c.Len())
	}
	if c.Popcount() != 3 {
		t.Errorf("expected Popcount()=3, got %d", c.Popcount())
	}
	if !c.Test(0) || !c.Test(30) || !c.Test(150) {
		t.Error("expected bits 0, 30, 150 to be set")
	}
}

func TestXorSameSize(t *testing.T) {
	a := BitsetFromInteger(0b1100) // bits 2,3
	b := BitsetFromInteger(0b1010) // bits 1,3
	c := a.Xor(b)

	val, err := c.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	// bits 1,2 differ
	if val != 0b0110 {
		t.Errorf("expected 0b0110, got 0b%b", val)
	}
}

func TestXorWithSelf(t *testing.T) {
	// XOR of a bitset with itself should be all zeros.
	a := BitsetFromInteger(0b11011011)
	c := a.Xor(a)
	if c.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", c.Popcount())
	}
}

func TestNotSimple(t *testing.T) {
	a := BitsetFromInteger(0b1010) // len=4, bits 1,3 set
	b := a.Not()

	val, err := b.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	// Flip within len=4: bits 0,2 set -> 0101 = 5
	if val != 0b0101 {
		t.Errorf("expected 0b0101, got 0b%b", val)
	}
	if b.Len() != 4 {
		t.Errorf("expected Len()=4, got %d", b.Len())
	}
}

func TestNotEmpty(t *testing.T) {
	a := NewBitset(0)
	b := a.Not()
	if b.Len() != 0 {
		t.Errorf("expected Len()=0, got %d", b.Len())
	}
	if b.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", b.Popcount())
	}
}

func TestNotAllOnes(t *testing.T) {
	// NOT of all-ones should be all-zeros.
	a := BitsetFromInteger(0b1111)
	b := a.Not()
	if b.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", b.Popcount())
	}
}

func TestDoubleNotIsIdentity(t *testing.T) {
	a := BitsetFromInteger(0b10110101)
	b := a.Not().Not()
	if !a.Equal(b) {
		t.Error("expected double NOT to be identity")
	}
}

func TestNotCleanTrailingBits(t *testing.T) {
	// Create a bitset where len is not a multiple of 64.
	// NOT should not leave trailing bits set.
	bs := NewBitset(10)
	bs.Set(0)
	bs.Set(2)
	notBs := bs.Not()

	// Popcount should be 10 - 2 = 8 (bits 1,3,4,5,6,7,8,9).
	if notBs.Popcount() != 8 {
		t.Errorf("expected Popcount()=8, got %d", notBs.Popcount())
	}
	// Capacity bits beyond len should NOT be set.
	if notBs.Capacity() > notBs.Len() {
		// Manually check no bits beyond len are set by looking at word.
		remaining := bitOffset(notBs.length)
		if remaining != 0 {
			lastWord := notBs.words[len(notBs.words)-1]
			mask := ^((uint64(1) << uint(remaining)) - 1)
			if lastWord&mask != 0 {
				t.Error("trailing bits are set after NOT")
			}
		}
	}
}

func TestAndNotSimple(t *testing.T) {
	a := BitsetFromInteger(0b1110) // bits 1,2,3
	b := BitsetFromInteger(0b1010) // bits 1,3
	c := a.AndNot(b)

	val, err := c.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	// Only bit 2 remains.
	if val != 0b0100 {
		t.Errorf("expected 0b0100, got 0b%b", val)
	}
}

func TestAndNotDifferentSizes(t *testing.T) {
	a := NewBitset(200)
	a.Set(0)
	a.Set(100)
	a.Set(150)

	b := NewBitset(50)
	b.Set(0)

	c := a.AndNot(b)
	// Bits 100 and 150 remain (bit 0 is in b, so removed).
	if c.Popcount() != 2 {
		t.Errorf("expected Popcount()=2, got %d", c.Popcount())
	}
	if c.Test(0) {
		t.Error("expected bit 0 to be clear")
	}
	if !c.Test(100) || !c.Test(150) {
		t.Error("expected bits 100, 150 to be set")
	}
}

// Test that bulk operations don't modify their operands.
func TestBulkOperationsNonDestructive(t *testing.T) {
	a := BitsetFromInteger(0b1100)
	b := BitsetFromInteger(0b1010)

	aOrig, _ := a.ToInteger()
	bOrig, _ := b.ToInteger()

	_ = a.And(b)
	_ = a.Or(b)
	_ = a.Xor(b)
	_ = a.Not()
	_ = a.AndNot(b)

	aVal, _ := a.ToInteger()
	bVal, _ := b.ToInteger()
	if aVal != aOrig {
		t.Errorf("And/Or/Xor/Not/AndNot modified operand a")
	}
	if bVal != bOrig {
		t.Errorf("And/Or/Xor/Not/AndNot modified operand b")
	}
}

// ===========================================================================
// Counting and query tests
// ===========================================================================

func TestPopcount(t *testing.T) {
	bs := BitsetFromInteger(0b10110) // bits 1,2,4
	if bs.Popcount() != 3 {
		t.Errorf("expected Popcount()=3, got %d", bs.Popcount())
	}
}

func TestPopcountEmpty(t *testing.T) {
	bs := NewBitset(0)
	if bs.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", bs.Popcount())
	}
}

func TestPopcountMultiWord(t *testing.T) {
	bs := NewBitset(200)
	for i := 0; i < 200; i++ {
		bs.Set(i)
	}
	if bs.Popcount() != 200 {
		t.Errorf("expected Popcount()=200, got %d", bs.Popcount())
	}
}

func TestLenAndCapacity(t *testing.T) {
	bs := NewBitset(100)
	if bs.Len() != 100 {
		t.Errorf("expected Len()=100, got %d", bs.Len())
	}
	if bs.Capacity() != 128 {
		t.Errorf("expected Capacity()=128, got %d", bs.Capacity())
	}
}

func TestAny(t *testing.T) {
	bs := NewBitset(100)
	if bs.Any() {
		t.Error("expected Any()=false for empty bitset")
	}
	bs.Set(50)
	if !bs.Any() {
		t.Error("expected Any()=true after setting a bit")
	}
}

func TestAnyEmpty(t *testing.T) {
	bs := NewBitset(0)
	if bs.Any() {
		t.Error("expected Any()=false for zero-length bitset")
	}
}

func TestAllVacuousTruth(t *testing.T) {
	bs := NewBitset(0)
	if !bs.All() {
		t.Error("expected All()=true for empty bitset (vacuous truth)")
	}
}

func TestAllTrue(t *testing.T) {
	bs, _ := BitsetFromBinaryStr("1111")
	if !bs.All() {
		t.Error("expected All()=true")
	}
}

func TestAllFalse(t *testing.T) {
	bs, _ := BitsetFromBinaryStr("1110")
	if bs.All() {
		t.Error("expected All()=false")
	}
}

func TestAllMultiWord(t *testing.T) {
	// Create a bitset larger than 64 bits where all bits are set.
	bs := NewBitset(100)
	for i := 0; i < 100; i++ {
		bs.Set(i)
	}
	if !bs.All() {
		t.Error("expected All()=true when all 100 bits are set")
	}
	// Clear one bit.
	bs.Clear(50)
	if bs.All() {
		t.Error("expected All()=false after clearing bit 50")
	}
}

func TestAllExactMultipleOf64(t *testing.T) {
	bs := NewBitset(64)
	for i := 0; i < 64; i++ {
		bs.Set(i)
	}
	if !bs.All() {
		t.Error("expected All()=true for full 64-bit word")
	}
}

func TestNone(t *testing.T) {
	bs := NewBitset(100)
	if !bs.None() {
		t.Error("expected None()=true for zeroed bitset")
	}
	bs.Set(0)
	if bs.None() {
		t.Error("expected None()=false after setting a bit")
	}
}

// ===========================================================================
// Iteration tests
// ===========================================================================

func TestIterSetBitsEmpty(t *testing.T) {
	bs := NewBitset(0)
	result := bs.IterSetBits()
	if len(result) != 0 {
		t.Errorf("expected empty slice, got %v", result)
	}
}

func TestIterSetBitsNoSet(t *testing.T) {
	bs := NewBitset(100)
	result := bs.IterSetBits()
	if len(result) != 0 {
		t.Errorf("expected empty slice, got %v", result)
	}
}

func TestIterSetBitsSimple(t *testing.T) {
	bs := BitsetFromInteger(0b10100101) // bits 0,2,5,7
	result := bs.IterSetBits()
	expected := []int{0, 2, 5, 7}
	if !intSliceEqual(result, expected) {
		t.Errorf("expected %v, got %v", expected, result)
	}
}

func TestIterSetBitsMultiWord(t *testing.T) {
	bs := NewBitset(200)
	bs.Set(0)
	bs.Set(63)
	bs.Set(64)
	bs.Set(127)
	bs.Set(128)
	bs.Set(199)

	result := bs.IterSetBits()
	expected := []int{0, 63, 64, 127, 128, 199}
	if !intSliceEqual(result, expected) {
		t.Errorf("expected %v, got %v", expected, result)
	}
}

func TestIterSetBitsAscending(t *testing.T) {
	// Verify results are always in ascending order.
	bs := NewBitset(300)
	bs.Set(250)
	bs.Set(10)
	bs.Set(100)
	bs.Set(0)
	bs.Set(299)

	result := bs.IterSetBits()
	for i := 1; i < len(result); i++ {
		if result[i] <= result[i-1] {
			t.Errorf("results not in ascending order: %v", result)
			break
		}
	}
}

// intSliceEqual compares two int slices for equality.
func intSliceEqual(a, b []int) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

// ===========================================================================
// Conversion tests
// ===========================================================================

func TestToIntegerSimple(t *testing.T) {
	bs := BitsetFromInteger(42)
	val, err := bs.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	if val != 42 {
		t.Errorf("expected 42, got %d", val)
	}
}

func TestToIntegerEmpty(t *testing.T) {
	bs := NewBitset(0)
	val, err := bs.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	if val != 0 {
		t.Errorf("expected 0, got %d", val)
	}
}

func TestToIntegerOverflow(t *testing.T) {
	bs := NewBitset(200)
	bs.Set(0)
	bs.Set(100)
	_, err := bs.ToInteger()
	if err == nil {
		t.Error("expected error for multi-word bitset")
	}
}

func TestToIntegerZeroed(t *testing.T) {
	// A bitset with allocated words but no set bits.
	bs := NewBitset(100)
	val, err := bs.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	if val != 0 {
		t.Errorf("expected 0, got %d", val)
	}
}

func TestToBinaryStrSimple(t *testing.T) {
	bs := BitsetFromInteger(5) // binary 101
	s := bs.ToBinaryStr()
	if s != "101" {
		t.Errorf("expected '101', got %q", s)
	}
}

func TestToBinaryStrEmpty(t *testing.T) {
	bs := NewBitset(0)
	s := bs.ToBinaryStr()
	if s != "" {
		t.Errorf("expected empty string, got %q", s)
	}
}

func TestToBinaryStrWithLeadingZeros(t *testing.T) {
	// from_binary_str("0010") preserves length=4, so output should be "0010".
	bs, _ := BitsetFromBinaryStr("0010")
	s := bs.ToBinaryStr()
	if s != "0010" {
		t.Errorf("expected '0010', got %q", s)
	}
}

func TestBinaryStrRoundTrip(t *testing.T) {
	// from_binary_str -> to_binary_str should be identity.
	original := "10110011"
	bs, err := BitsetFromBinaryStr(original)
	if err != nil {
		t.Fatal(err)
	}
	result := bs.ToBinaryStr()
	if result != original {
		t.Errorf("expected %q, got %q", original, result)
	}
}

func TestIntegerRoundTrip(t *testing.T) {
	// from_integer -> to_integer should be identity for values that fit.
	for _, v := range []uint64{0, 1, 5, 42, 255, 1023, ^uint64(0)} {
		bs := BitsetFromInteger(v)
		val, err := bs.ToInteger()
		if v == 0 {
			if err != nil {
				t.Fatalf("unexpected error for value %d: %v", v, err)
			}
			if val != 0 {
				t.Errorf("for value %d: expected 0, got %d", v, val)
			}
			continue
		}
		if err != nil {
			t.Fatalf("unexpected error for value %d: %v", v, err)
		}
		if val != v {
			t.Errorf("expected %d, got %d", v, val)
		}
	}
}

func TestStringMethod(t *testing.T) {
	bs := BitsetFromInteger(5)
	s := bs.String()
	if s != "Bitset(101)" {
		t.Errorf("expected 'Bitset(101)', got %q", s)
	}
}

func TestStringMethodEmpty(t *testing.T) {
	bs := NewBitset(0)
	s := bs.String()
	if s != "Bitset()" {
		t.Errorf("expected 'Bitset()', got %q", s)
	}
}

// ===========================================================================
// Equality tests
// ===========================================================================

func TestEqualSame(t *testing.T) {
	a := BitsetFromInteger(42)
	b := BitsetFromInteger(42)
	if !a.Equal(b) {
		t.Error("expected Equal to be true")
	}
}

func TestEqualDifferentBits(t *testing.T) {
	a := BitsetFromInteger(42)
	b := BitsetFromInteger(43)
	if a.Equal(b) {
		t.Error("expected Equal to be false for different bits")
	}
}

func TestEqualDifferentLen(t *testing.T) {
	a := NewBitset(10)
	b := NewBitset(20)
	// Both are all-zeros but different lengths.
	if a.Equal(b) {
		t.Error("expected Equal to be false for different lengths")
	}
}

func TestEqualDifferentCapacity(t *testing.T) {
	// Two bitsets with same len and same bits but different capacity.
	// Create two bitsets that end up with same bits/len.
	a2 := NewBitset(3)
	a2.Set(0)
	a2.Set(2)

	b2 := NewBitset(3)
	b2.Set(0)
	b2.Set(2)

	if !a2.Equal(b2) {
		t.Error("expected Equal to be true for same bits and length")
	}
}

func TestEqualEmpty(t *testing.T) {
	a := NewBitset(0)
	b := NewBitset(0)
	if !a.Equal(b) {
		t.Error("expected two empty bitsets to be Equal")
	}
}

// ===========================================================================
// Edge cases and stress tests
// ===========================================================================

func TestSetClearAllBits(t *testing.T) {
	// Set every bit in a multi-word bitset, then clear them all.
	bs := NewBitset(200)
	for i := 0; i < 200; i++ {
		bs.Set(i)
	}
	if bs.Popcount() != 200 {
		t.Errorf("expected Popcount()=200, got %d", bs.Popcount())
	}
	for i := 0; i < 200; i++ {
		bs.Clear(i)
	}
	if bs.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", bs.Popcount())
	}
}

func TestWordBoundary(t *testing.T) {
	// Test bits at word boundaries (63, 64, 127, 128).
	bs := NewBitset(200)
	boundaries := []int{0, 63, 64, 127, 128, 191, 192, 199}
	for _, i := range boundaries {
		bs.Set(i)
	}
	for _, i := range boundaries {
		if !bs.Test(i) {
			t.Errorf("expected bit %d to be set", i)
		}
	}
	if bs.Popcount() != len(boundaries) {
		t.Errorf("expected Popcount()=%d, got %d", len(boundaries), bs.Popcount())
	}
}

func TestGrowthDoubling(t *testing.T) {
	// Verify doubling growth: start from 0, grow to progressively larger sizes.
	bs := NewBitset(0)
	bs.Set(3)
	if bs.Capacity() != 64 {
		t.Errorf("expected Capacity()=64 after first growth, got %d", bs.Capacity())
	}
	bs.Set(100)
	if bs.Capacity() != 128 {
		t.Errorf("expected Capacity()=128 after second growth, got %d", bs.Capacity())
	}
	bs.Set(200)
	if bs.Capacity() != 256 {
		t.Errorf("expected Capacity()=256 after third growth, got %d", bs.Capacity())
	}
	bs.Set(500)
	if bs.Capacity() != 512 {
		t.Errorf("expected Capacity()=512 after fourth growth, got %d", bs.Capacity())
	}
}

func TestSetWithinCapacityUpdatesLen(t *testing.T) {
	// Set a bit within capacity but beyond len should update len without
	// allocating more memory.
	bs := NewBitset(10) // len=10, capacity=64
	bs.Set(50)          // within capacity, but beyond len
	if bs.Len() != 51 {
		t.Errorf("expected Len()=51, got %d", bs.Len())
	}
	if bs.Capacity() != 64 {
		t.Errorf("expected Capacity()=64 (no growth), got %d", bs.Capacity())
	}
}

func TestAndOrXorIdentities(t *testing.T) {
	a := BitsetFromInteger(0b11001100)

	// a AND a == a
	if !a.And(a).Equal(a) {
		t.Error("a AND a should equal a")
	}

	// a OR a == a
	if !a.Or(a).Equal(a) {
		t.Error("a OR a should equal a")
	}

	// a XOR a == 0
	if a.Xor(a).Popcount() != 0 {
		t.Error("a XOR a should have popcount 0")
	}

	// a AND NOT a == 0
	if a.AndNot(a).Popcount() != 0 {
		t.Error("a AND NOT a should have popcount 0")
	}
}

func TestDeMorgansLaw(t *testing.T) {
	// De Morgan's Law: NOT(A AND B) == (NOT A) OR (NOT B)
	a := BitsetFromInteger(0b11001100)
	b := BitsetFromInteger(0b10101010)

	lhs := a.And(b).Not()
	rhs := a.Not().Or(b.Not())

	if !lhs.Equal(rhs) {
		t.Errorf("De Morgan's Law failed: NOT(A AND B) != NOT(A) OR NOT(B)")
	}
}

func TestBitsetErrorString(t *testing.T) {
	err := &BitsetError{Message: "test error"}
	if err.Error() != "test error" {
		t.Errorf("expected 'test error', got %q", err.Error())
	}
}

// ===========================================================================
// Helper function tests
// ===========================================================================

func TestWordsNeeded(t *testing.T) {
	cases := []struct {
		bits     int
		expected int
	}{
		{0, 0},
		{1, 1},
		{64, 1},
		{65, 2},
		{128, 2},
		{129, 3},
		{200, 4},
	}
	for _, tc := range cases {
		got := wordsNeeded(tc.bits)
		if got != tc.expected {
			t.Errorf("wordsNeeded(%d) = %d, expected %d", tc.bits, got, tc.expected)
		}
	}
}

func TestWordIndex(t *testing.T) {
	cases := []struct {
		bit      int
		expected int
	}{
		{0, 0},
		{63, 0},
		{64, 1},
		{127, 1},
		{128, 2},
		{137, 2},
	}
	for _, tc := range cases {
		got := wordIndex(tc.bit)
		if got != tc.expected {
			t.Errorf("wordIndex(%d) = %d, expected %d", tc.bit, got, tc.expected)
		}
	}
}

func TestBitOffset(t *testing.T) {
	cases := []struct {
		bit      int
		expected int
	}{
		{0, 0},
		{63, 63},
		{64, 0},
		{65, 1},
		{137, 9},
	}
	for _, tc := range cases {
		got := bitOffset(tc.bit)
		if got != tc.expected {
			t.Errorf("bitOffset(%d) = %d, expected %d", tc.bit, got, tc.expected)
		}
	}
}

func TestBitmask(t *testing.T) {
	// bitmask(0) should be 1, bitmask(1) should be 2, etc.
	if bitmask(0) != 1 {
		t.Errorf("bitmask(0) = %d, expected 1", bitmask(0))
	}
	if bitmask(1) != 2 {
		t.Errorf("bitmask(1) = %d, expected 2", bitmask(1))
	}
	if bitmask(63) != 1<<63 {
		t.Errorf("bitmask(63) = %d, expected %d", bitmask(63), uint64(1)<<63)
	}
	// bitmask(64) should wrap to bit 0.
	if bitmask(64) != 1 {
		t.Errorf("bitmask(64) = %d, expected 1", bitmask(64))
	}
}

func TestWordAt(t *testing.T) {
	words := []uint64{10, 20, 30}
	if wordAt(words, 0) != 10 {
		t.Errorf("expected 10, got %d", wordAt(words, 0))
	}
	if wordAt(words, 2) != 30 {
		t.Errorf("expected 30, got %d", wordAt(words, 2))
	}
	// Out of bounds returns 0.
	if wordAt(words, 5) != 0 {
		t.Errorf("expected 0, got %d", wordAt(words, 5))
	}
}

func TestMax(t *testing.T) {
	if max(3, 5) != 5 {
		t.Errorf("max(3,5) = %d, expected 5", max(3, 5))
	}
	if max(5, 3) != 5 {
		t.Errorf("max(5,3) = %d, expected 5", max(5, 3))
	}
	if max(5, 5) != 5 {
		t.Errorf("max(5,5) = %d, expected 5", max(5, 5))
	}
}

func TestCleanTrailingBits(t *testing.T) {
	// Manually set trailing bits and verify cleanTrailingBits clears them.
	bs := NewBitset(10) // len=10, capacity=64
	// Manually corrupt: set bit 50 in the word without updating length.
	bs.words[0] |= 1 << 50
	bs.cleanTrailingBits()
	// Bit 50 should be cleared because it's beyond length=10.
	if bs.words[0]&(1<<50) != 0 {
		t.Error("cleanTrailingBits failed to clear trailing bit")
	}
}

func TestCleanTrailingBitsMultipleOf64(t *testing.T) {
	// When len is a multiple of 64, there are no trailing bits.
	bs := NewBitset(64)
	bs.Set(63)
	bs.cleanTrailingBits()
	if !bs.Test(63) {
		t.Error("cleanTrailingBits incorrectly cleared a valid bit")
	}
}

func TestCleanTrailingBitsEmptyBitset(t *testing.T) {
	// Should not panic on empty bitset.
	bs := NewBitset(0)
	bs.cleanTrailingBits() // should be a no-op
}

// ===========================================================================
// Additional coverage tests
// ===========================================================================

func TestFromBinaryStrSingleBit(t *testing.T) {
	bs, err := BitsetFromBinaryStr("1")
	if err != nil {
		t.Fatal(err)
	}
	if bs.Len() != 1 {
		t.Errorf("expected Len()=1, got %d", bs.Len())
	}
	if !bs.Test(0) {
		t.Error("expected bit 0 to be set")
	}
}

func TestFromBinaryStrSingleZero(t *testing.T) {
	bs, err := BitsetFromBinaryStr("0")
	if err != nil {
		t.Fatal(err)
	}
	if bs.Len() != 1 {
		t.Errorf("expected Len()=1, got %d", bs.Len())
	}
	if bs.Test(0) {
		t.Error("expected bit 0 to be clear")
	}
}

func TestAndWithEmpty(t *testing.T) {
	a := BitsetFromInteger(0b1010)
	b := NewBitset(0)
	c := a.And(b)
	// AND with empty should produce all zeros with length = max(4, 0) = 4.
	if c.Len() != 4 {
		t.Errorf("expected Len()=4, got %d", c.Len())
	}
	if c.Popcount() != 0 {
		t.Errorf("expected Popcount()=0, got %d", c.Popcount())
	}
}

func TestOrWithEmpty(t *testing.T) {
	a := BitsetFromInteger(0b1010)
	b := NewBitset(0)
	c := a.Or(b)
	if c.Len() != 4 {
		t.Errorf("expected Len()=4, got %d", c.Len())
	}
	val, _ := c.ToInteger()
	if val != 0b1010 {
		t.Errorf("expected 0b1010, got 0b%b", val)
	}
}

func TestXorWithEmpty(t *testing.T) {
	a := BitsetFromInteger(0b1010)
	b := NewBitset(0)
	c := a.Xor(b)
	val, _ := c.ToInteger()
	if val != 0b1010 {
		t.Errorf("expected 0b1010, got 0b%b", val)
	}
}

func TestAndNotWithEmpty(t *testing.T) {
	a := BitsetFromInteger(0b1010)
	b := NewBitset(0)
	c := a.AndNot(b)
	val, _ := c.ToInteger()
	if val != 0b1010 {
		t.Errorf("expected 0b1010, got 0b%b", val)
	}
}

func TestEqualAfterFromIntegerAndBinaryStr(t *testing.T) {
	// from_integer(10) and from_binary_str("1010") should be equal.
	a := BitsetFromInteger(10)
	b, _ := BitsetFromBinaryStr("1010")
	if !a.Equal(b) {
		t.Error("expected from_integer(10) == from_binary_str(\"1010\")")
	}
}

func TestPopcountAfterMultipleOperations(t *testing.T) {
	bs := NewBitset(100)
	bs.Set(10)
	bs.Set(20)
	bs.Set(30)
	bs.Clear(20)
	bs.Toggle(40)
	// Should have bits 10, 30, 40 set.
	if bs.Popcount() != 3 {
		t.Errorf("expected Popcount()=3, got %d", bs.Popcount())
	}
}

func TestIterSetBitsAfterClear(t *testing.T) {
	bs := NewBitset(100)
	bs.Set(10)
	bs.Set(20)
	bs.Set(30)
	bs.Clear(20)
	result := bs.IterSetBits()
	expected := []int{10, 30}
	if !intSliceEqual(result, expected) {
		t.Errorf("expected %v, got %v", expected, result)
	}
}

func TestToBinaryStrMultiWord(t *testing.T) {
	bs := NewBitset(100)
	bs.Set(0)
	bs.Set(99)
	s := bs.ToBinaryStr()
	// Should be 100 chars long, starting with "1" (bit 99) and ending with "1" (bit 0).
	if len(s) != 100 {
		t.Errorf("expected len=100, got %d", len(s))
	}
	if s[0] != '1' {
		t.Error("expected first char (bit 99) to be '1'")
	}
	if s[99] != '1' {
		t.Error("expected last char (bit 0) to be '1'")
	}
	// Count '1's -- should be exactly 2.
	count := 0
	for _, ch := range s {
		if ch == '1' {
			count++
		}
	}
	if count != 2 {
		t.Errorf("expected 2 ones, got %d", count)
	}
}

func TestNotMultiWord(t *testing.T) {
	bs := NewBitset(100)
	bs.Set(0)
	bs.Set(50)
	notBs := bs.Not()
	// Should have 100 - 2 = 98 bits set.
	if notBs.Popcount() != 98 {
		t.Errorf("expected Popcount()=98, got %d", notBs.Popcount())
	}
	if notBs.Test(0) {
		t.Error("expected bit 0 to be clear in NOT")
	}
	if notBs.Test(50) {
		t.Error("expected bit 50 to be clear in NOT")
	}
	if !notBs.Test(1) {
		t.Error("expected bit 1 to be set in NOT")
	}
}

func TestToggleFromEmptyBitset(t *testing.T) {
	bs := NewBitset(0)
	bs.Toggle(5)
	if bs.Len() != 6 {
		t.Errorf("expected Len()=6, got %d", bs.Len())
	}
	if !bs.Test(5) {
		t.Error("expected bit 5 to be set after toggle on empty")
	}
}

func TestEnsureCapacityWithinCapacity(t *testing.T) {
	// Set bit within capacity but beyond length.
	bs := NewBitset(10) // cap=64
	bs.Set(50)          // within cap, extends length
	if bs.Len() != 51 {
		t.Errorf("expected Len()=51, got %d", bs.Len())
	}
	if bs.Capacity() != 64 {
		t.Errorf("expected unchanged Capacity()=64, got %d", bs.Capacity())
	}
}

func TestFromBinaryStrWithSpaces(t *testing.T) {
	// Spaces are invalid characters.
	_, err := BitsetFromBinaryStr("10 10")
	if err == nil {
		t.Error("expected error for binary string with spaces")
	}
}

func TestToIntegerSingleWordNoOverflow(t *testing.T) {
	// A bitset that uses multiple words but only the first has bits set.
	bs := NewBitset(200)
	bs.Set(5)
	val, err := bs.ToInteger()
	if err != nil {
		t.Fatal(err)
	}
	if val != 32 {
		t.Errorf("expected 32, got %d", val)
	}
}
