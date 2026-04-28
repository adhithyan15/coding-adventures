// ============================================================================
// BloomFilterTest.java — Unit Tests for BloomFilter
// ============================================================================
//
// Tests cover:
//   1. Construction (auto-sized and explicit)
//   2. No false negatives — added elements are always found
//   3. Probably-absent items can return false (no false negative guarantee)
//   4. Statistics (bitCount, hashCount, fillRatio, estimatedFPR)
//   5. isOverCapacity
//   6. Static utilities (optimalM, optimalK, capacityForMemory)
//   7. toString format
//   8. Edge cases (large number of elements, different types)
//   9. Input validation (IllegalArgumentException guards)
// ============================================================================

package com.codingadventures.bloomfilter;

import org.junit.jupiter.api.Test;
import java.util.HashSet;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.*;

class BloomFilterTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    void constructorSizesFilter() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        assertTrue(bf.bitCount() > 0, "bit count should be positive");
        assertTrue(bf.hashCount() >= 1, "hash count should be at least 1");
        assertEquals(0, bf.bitsSet());
        assertEquals(0, bf.size());
    }

    @Test
    void explicitConstructorSetsParams() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 5, true);
        assertEquals(1000, bf.bitCount());
        assertEquals(5, bf.hashCount());
    }

    @Test
    void optimalParamsFor1PercentFPR() {
        // For n=1000, p=0.01 the standard formula gives m≈9585 bits, k≈7
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        // Allow some rounding tolerance — exact values depend on ceiling
        assertTrue(bf.bitCount()  > 8000, "m should be > 8000 for 1% FPR");
        assertTrue(bf.hashCount() >= 5,   "k should be >= 5 for 1% FPR");
    }

    // =========================================================================
    // 2. Zero false negatives
    // =========================================================================

    @Test
    void addedElementIsAlwaysFound() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        bf.add("hello");
        assertTrue(bf.contains("hello"), "added element must be found");
    }

    @Test
    void addedElementsAreAlwaysFound() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        String[] words = {"apple", "banana", "cherry", "date", "elderberry"};
        for (String w : words) bf.add(w);
        for (String w : words) {
            assertTrue(bf.contains(w), "added element must always be found: " + w);
        }
    }

    @Test
    void zeroFalseNegativesForManyElements() {
        // Add 500 elements and verify every one is found (zero false negatives).
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        for (int i = 0; i < 500; i++) {
            bf.add("element-" + i);
        }
        for (int i = 0; i < 500; i++) {
            assertTrue(bf.contains("element-" + i),
                "element-" + i + " was added, must be found");
        }
    }

    @Test
    void emptyFilterContainsNothing() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        // Very unlikely all bits for a random string are set in an empty filter
        assertFalse(bf.contains("absolutely-not-added"));
    }

    // =========================================================================
    // 3. False positive rate stays within expected bounds
    // =========================================================================

    @Test
    void falsePositiveRateWithinBounds() {
        // Add 1000 distinct elements, then probe 10,000 distinct non-added elements.
        // The FPR should be ≤ 5% (target is 1%, allowing 5× buffer for statistics).
        int n = 1000;
        BloomFilter<String> bf = new BloomFilter<>(n, 0.01);

        Set<String> added = new HashSet<>();
        for (int i = 0; i < n; i++) {
            String s = "member:" + i;
            bf.add(s);
            added.add(s);
        }

        int falsePositives = 0;
        int probes = 10_000;
        for (int i = 0; i < probes; i++) {
            String s = "nonmember:" + i;
            if (!added.contains(s) && bf.contains(s)) {
                falsePositives++;
            }
        }

        double actualFPR = (double) falsePositives / probes;
        assertTrue(actualFPR <= 0.05,
            "FPR should be ≤ 5%, got: " + actualFPR * 100 + "%");
    }

    // =========================================================================
    // 4. Statistics
    // =========================================================================

    @Test
    void bitsSetIncreasesOnAdd() {
        BloomFilter<String> bf = new BloomFilter<>(10_000, 0.01);
        assertEquals(0, bf.bitsSet());
        bf.add("first");
        assertTrue(bf.bitsSet() > 0);
    }

    @Test
    void fillRatioIsZeroForEmptyFilter() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        assertEquals(0.0, bf.fillRatio());
    }

    @Test
    void fillRatioIncreasesAfterAdds() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        bf.add("a");
        assertTrue(bf.fillRatio() > 0.0);
        assertTrue(bf.fillRatio() < 1.0);
    }

    @Test
    void estimatedFPRIsZeroForEmptyFilter() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        assertEquals(0.0, bf.estimatedFalsePositiveRate());
    }

    @Test
    void estimatedFPRRisesAsFillIncreases() {
        BloomFilter<String> bf = new BloomFilter<>(100, 0.01);
        for (int i = 0; i < 500; i++) bf.add("x" + i);  // deliberately over-fill
        assertTrue(bf.estimatedFalsePositiveRate() > 0.0);
    }

    @Test
    void sizeBytesMatchesBitCount() {
        BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);
        // sizeBytes must cover all m bits
        assertTrue(bf.sizeBytes() * 8 >= bf.bitCount());
    }

    // =========================================================================
    // 5. isOverCapacity
    // =========================================================================

    @Test
    void notOverCapacityWhenEmpty() {
        BloomFilter<String> bf = new BloomFilter<>(100, 0.01);
        assertFalse(bf.isOverCapacity());
    }

    @Test
    void notOverCapacityAtExpectedLoad() {
        BloomFilter<String> bf = new BloomFilter<>(100, 0.01);
        for (int i = 0; i < 100; i++) bf.add("e" + i);
        assertFalse(bf.isOverCapacity());
    }

    @Test
    void isOverCapacityWhenExceeded() {
        BloomFilter<String> bf = new BloomFilter<>(10, 0.01);
        for (int i = 0; i < 11; i++) bf.add("e" + i);
        assertTrue(bf.isOverCapacity());
    }

    @Test
    void explicitFilterNeverOverCapacity() {
        BloomFilter<String> bf = new BloomFilter<>(100, 5, true);
        for (int i = 0; i < 1000; i++) bf.add("e" + i);
        assertFalse(bf.isOverCapacity(), "explicit filter has no capacity limit");
    }

    // =========================================================================
    // 6. Static utilities
    // =========================================================================

    @Test
    void optimalMGrowsWithN() {
        long m100  = BloomFilter.optimalM(100,   0.01);
        long m1000 = BloomFilter.optimalM(1000,  0.01);
        assertTrue(m1000 > m100, "more elements → larger bit array");
    }

    @Test
    void optimalMGrowsWithDecreasingFPR() {
        long m1pct   = BloomFilter.optimalM(1000, 0.01);
        long m01pct  = BloomFilter.optimalM(1000, 0.001);
        assertTrue(m01pct > m1pct, "tighter FPR → larger bit array");
    }

    @Test
    void optimalKReturnsAtLeastOne() {
        assertTrue(BloomFilter.optimalK(100, 1000) >= 1);
    }

    @Test
    void capacityForMemoryIsPositive() {
        long cap = BloomFilter.capacityForMemory(1_000_000, 0.01);
        assertTrue(cap > 0);
    }

    @Test
    void capacityForMemoryGrowsWithMemory() {
        long c1 = BloomFilter.capacityForMemory(100_000, 0.01);
        long c2 = BloomFilter.capacityForMemory(200_000, 0.01);
        assertTrue(c2 > c1);
    }

    // =========================================================================
    // 7. toString
    // =========================================================================

    @Test
    void toStringContainsKeyFields() {
        BloomFilter<String> bf = new BloomFilter<>(100, 0.01);
        bf.add("a");
        String s = bf.toString();
        assertTrue(s.contains("BloomFilter"), "should start with BloomFilter");
        assertTrue(s.contains("m="), "should show m");
        assertTrue(s.contains("k="), "should show k");
    }

    // =========================================================================
    // 8. Different element types
    // =========================================================================

    @Test
    void worksWithIntegers() {
        BloomFilter<Integer> bf = new BloomFilter<>(1000, 0.01);
        bf.add(42);
        bf.add(100);
        assertTrue(bf.contains(42));
        assertTrue(bf.contains(100));
    }

    @Test
    void worksWithLong() {
        BloomFilter<Long> bf = new BloomFilter<>(1000, 0.01);
        bf.add(1_000_000_000_000L);
        assertTrue(bf.contains(1_000_000_000_000L));
    }

    // =========================================================================
    // 9. Input validation
    // =========================================================================

    @Test
    void constructorRejectsZeroExpectedItems() {
        assertThrows(IllegalArgumentException.class, () ->
            new BloomFilter<>(0, 0.01));
    }

    @Test
    void constructorRejectsNegativeExpectedItems() {
        assertThrows(IllegalArgumentException.class, () ->
            new BloomFilter<>(-1, 0.01));
    }

    @Test
    void constructorRejectsZeroFPR() {
        assertThrows(IllegalArgumentException.class, () ->
            new BloomFilter<>(1000, 0.0));
    }

    @Test
    void constructorRejectsOneFPR() {
        assertThrows(IllegalArgumentException.class, () ->
            new BloomFilter<>(1000, 1.0));
    }

    @Test
    void constructorRejectsNegativeFPR() {
        assertThrows(IllegalArgumentException.class, () ->
            new BloomFilter<>(1000, -0.01));
    }

    @Test
    void explicitConstructorRejectsZeroBits() {
        assertThrows(IllegalArgumentException.class, () ->
            new BloomFilter<>(0, 5, true));
    }

    @Test
    void explicitConstructorRejectsZeroHashFunctions() {
        assertThrows(IllegalArgumentException.class, () ->
            new BloomFilter<>(100, 0, true));
    }

    @Test
    void optimalMRejectsZeroN() {
        assertThrows(IllegalArgumentException.class, () ->
            BloomFilter.optimalM(0, 0.01));
    }

    @Test
    void optimalMRejectsInvalidP() {
        assertThrows(IllegalArgumentException.class, () ->
            BloomFilter.optimalM(1000, 0.0));
        assertThrows(IllegalArgumentException.class, () ->
            BloomFilter.optimalM(1000, 1.1));
    }
}
