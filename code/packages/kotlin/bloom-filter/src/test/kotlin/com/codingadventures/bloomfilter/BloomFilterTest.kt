// ============================================================================
// BloomFilterTest.kt — Unit Tests for BloomFilter (Kotlin)
// ============================================================================

package com.codingadventures.bloomfilter

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class BloomFilterTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    fun constructorSizesFilter() {
        val bf = BloomFilter<String>(1000, 0.01)
        assertTrue(bf.bitCount > 0)
        assertTrue(bf.hashCount >= 1)
        assertEquals(0, bf.bitsSet)
        assertEquals(0, bf.size)
    }

    @Test
    fun explicitConstructorSetsParams() {
        val bf = BloomFilter.explicit<String>(1000, 5)
        assertEquals(1000, bf.bitCount)
        assertEquals(5, bf.hashCount)
    }

    @Test
    fun optimalParamsFor1PercentFPR() {
        val bf = BloomFilter<String>(1000, 0.01)
        assertTrue(bf.bitCount > 8000, "m should be > 8000 for 1% FPR")
        assertTrue(bf.hashCount >= 5, "k should be >= 5 for 1% FPR")
    }

    // =========================================================================
    // 2. Zero false negatives
    // =========================================================================

    @Test
    fun addedElementIsAlwaysFound() {
        val bf = BloomFilter<String>(1000, 0.01)
        bf.add("hello")
        assertTrue(bf.contains("hello"))
    }

    @Test
    fun addedElementsAreAlwaysFound() {
        val bf = BloomFilter<String>(1000, 0.01)
        val words = listOf("apple", "banana", "cherry", "date", "elderberry")
        words.forEach { bf.add(it) }
        words.forEach { assertTrue(bf.contains(it), "added element must be found: $it") }
    }

    @Test
    fun zeroFalseNegativesForManyElements() {
        val bf = BloomFilter<String>(1000, 0.01)
        repeat(500) { bf.add("element-$it") }
        repeat(500) {
            assertTrue(bf.contains("element-$it"), "element-$it was added, must be found")
        }
    }

    @Test
    fun emptyFilterContainsNothing() {
        val bf = BloomFilter<String>(1000, 0.01)
        assertFalse(bf.contains("absolutely-not-added"))
    }

    // =========================================================================
    // 3. False positive rate within bounds
    // =========================================================================

    @Test
    fun falsePositiveRateWithinBounds() {
        val n = 1000
        val bf = BloomFilter<String>(n, 0.01)
        val added = mutableSetOf<String>()
        repeat(n) { i ->
            val s = "member:$i"
            bf.add(s)
            added.add(s)
        }

        val probes = 10_000
        var falsePositives = 0
        repeat(probes) { i ->
            val s = "nonmember:$i"
            if (s !in added && bf.contains(s)) falsePositives++
        }

        val actualFPR = falsePositives.toDouble() / probes
        assertTrue(actualFPR <= 0.05, "FPR should be ≤ 5%, got: ${actualFPR * 100}%")
    }

    // =========================================================================
    // 4. Statistics
    // =========================================================================

    @Test
    fun bitsSetIncreasesOnAdd() {
        val bf = BloomFilter<String>(10_000, 0.01)
        assertEquals(0, bf.bitsSet)
        bf.add("first")
        assertTrue(bf.bitsSet > 0)
    }

    @Test
    fun fillRatioIsZeroForEmptyFilter() {
        val bf = BloomFilter<String>(1000, 0.01)
        assertEquals(0.0, bf.fillRatio)
    }

    @Test
    fun fillRatioIncreasesAfterAdds() {
        val bf = BloomFilter<String>(1000, 0.01)
        bf.add("a")
        assertTrue(bf.fillRatio > 0.0)
        assertTrue(bf.fillRatio < 1.0)
    }

    @Test
    fun estimatedFPRIsZeroForEmptyFilter() {
        val bf = BloomFilter<String>(1000, 0.01)
        assertEquals(0.0, bf.estimatedFalsePositiveRate)
    }

    @Test
    fun estimatedFPRRisesAsFillIncreases() {
        val bf = BloomFilter<String>(100, 0.01)
        repeat(500) { bf.add("x$it") }
        assertTrue(bf.estimatedFalsePositiveRate > 0.0)
    }

    @Test
    fun sizeBytesCoversAllBits() {
        val bf = BloomFilter<String>(1000, 0.01)
        assertTrue(bf.sizeBytes * 8 >= bf.bitCount)
    }

    // =========================================================================
    // 5. isOverCapacity
    // =========================================================================

    @Test
    fun notOverCapacityWhenEmpty() {
        assertFalse(BloomFilter<String>(100, 0.01).isOverCapacity)
    }

    @Test
    fun notOverCapacityAtExpectedLoad() {
        val bf = BloomFilter<String>(100, 0.01)
        repeat(100) { bf.add("e$it") }
        assertFalse(bf.isOverCapacity)
    }

    @Test
    fun isOverCapacityWhenExceeded() {
        val bf = BloomFilter<String>(10, 0.01)
        repeat(11) { bf.add("e$it") }
        assertTrue(bf.isOverCapacity)
    }

    @Test
    fun explicitFilterNeverOverCapacity() {
        val bf = BloomFilter.explicit<String>(100, 5)
        repeat(1000) { bf.add("e$it") }
        assertFalse(bf.isOverCapacity)
    }

    // =========================================================================
    // 6. Static utilities
    // =========================================================================

    @Test
    fun optimalMGrowsWithN() {
        val m100  = BloomFilter.optimalM(100,  0.01)
        val m1000 = BloomFilter.optimalM(1000, 0.01)
        assertTrue(m1000 > m100)
    }

    @Test
    fun optimalMGrowsWithDecreasingFPR() {
        val m1pct  = BloomFilter.optimalM(1000, 0.01)
        val m01pct = BloomFilter.optimalM(1000, 0.001)
        assertTrue(m01pct > m1pct)
    }

    @Test
    fun optimalKReturnsAtLeastOne() {
        assertTrue(BloomFilter.optimalK(100, 1000) >= 1)
    }

    @Test
    fun capacityForMemoryIsPositive() {
        assertTrue(BloomFilter.capacityForMemory(1_000_000, 0.01) > 0)
    }

    @Test
    fun capacityForMemoryGrowsWithMemory() {
        val c1 = BloomFilter.capacityForMemory(100_000, 0.01)
        val c2 = BloomFilter.capacityForMemory(200_000, 0.01)
        assertTrue(c2 > c1)
    }

    // =========================================================================
    // 7. toString
    // =========================================================================

    @Test
    fun toStringContainsKeyFields() {
        val bf = BloomFilter<String>(100, 0.01)
        bf.add("a")
        val s = bf.toString()
        assertTrue(s.contains("BloomFilter"))
        assertTrue(s.contains("m="))
        assertTrue(s.contains("k="))
    }

    // =========================================================================
    // 8. Different element types
    // =========================================================================

    @Test
    fun worksWithIntegers() {
        val bf = BloomFilter<Int>(1000, 0.01)
        bf.add(42)
        bf.add(100)
        assertTrue(bf.contains(42))
        assertTrue(bf.contains(100))
    }

    @Test
    fun worksWithLong() {
        val bf = BloomFilter<Long>(1000, 0.01)
        bf.add(1_000_000_000_000L)
        assertTrue(bf.contains(1_000_000_000_000L))
    }

    // =========================================================================
    // 9. Input validation
    // =========================================================================

    @Test
    fun constructorRejectsZeroExpectedItems() {
        assertThrows<IllegalArgumentException> { BloomFilter<String>(0, 0.01) }
    }

    @Test
    fun constructorRejectsNegativeExpectedItems() {
        assertThrows<IllegalArgumentException> { BloomFilter<String>(-1, 0.01) }
    }

    @Test
    fun constructorRejectsZeroFPR() {
        assertThrows<IllegalArgumentException> { BloomFilter<String>(1000, 0.0) }
    }

    @Test
    fun constructorRejectsOneFPR() {
        assertThrows<IllegalArgumentException> { BloomFilter<String>(1000, 1.0) }
    }

    @Test
    fun constructorRejectsNegativeFPR() {
        assertThrows<IllegalArgumentException> { BloomFilter<String>(1000, -0.01) }
    }

    @Test
    fun explicitConstructorRejectsZeroBits() {
        assertThrows<IllegalArgumentException> { BloomFilter.explicit<String>(0, 5) }
    }

    @Test
    fun explicitConstructorRejectsZeroHashFunctions() {
        assertThrows<IllegalArgumentException> { BloomFilter.explicit<String>(100, 0) }
    }

    @Test
    fun optimalMRejectsZeroN() {
        assertThrows<IllegalArgumentException> { BloomFilter.optimalM(0, 0.01) }
    }

    @Test
    fun optimalMRejectsInvalidP() {
        assertThrows<IllegalArgumentException> { BloomFilter.optimalM(1000, 0.0) }
        assertThrows<IllegalArgumentException> { BloomFilter.optimalM(1000, 1.1) }
    }

    // =========================================================================
    // 10. Internal hash functions (determinism)
    // =========================================================================

    @Test
    fun fnv1a32IsStable() {
        // Known test vector: fnv1a32("") == 0x811C9DC5 (2166136261 as unsigned)
        val empty = BloomFilter.fnv1a32("".toByteArray())
        assertEquals(0x811C9DC5.toInt(), empty)
    }

    @Test
    fun djb2IsStable() {
        // Known test vector: djb2("") == 5381 (initial value, no bytes processed)
        // After folding to 32 bits: 5381L xor (5381L ushr 32) = 5381 (high bits are 0)
        val empty = BloomFilter.djb2_32("".toByteArray())
        assertEquals(5381, empty)
    }
}
