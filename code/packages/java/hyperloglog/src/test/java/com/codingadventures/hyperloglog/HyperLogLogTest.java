package com.codingadventures.hyperloglog;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class HyperLogLogTest {
    @Test
    void startsEmpty() {
        HyperLogLog hll = new HyperLogLog();
        assertEquals(0, hll.count());
        assertEquals(0, hll.len());
    }

    @Test
    void ignoresDuplicatesAndGrowsForUniqueValues() {
        HyperLogLog hll = new HyperLogLog();
        for (int index = 0; index < 1_000; index++) {
            hll.add("same");
        }
        assertTrue(hll.count() < 10);

        HyperLogLog spread = new HyperLogLog();
        for (int index = 0; index < 1_000; index++) {
            spread.add("item-" + index);
        }
        assertTrue(spread.count() >= 800);
        assertTrue(spread.count() <= 1_200);
    }

    @Test
    void mergesSketchesWithSamePrecision() {
        HyperLogLog left = new HyperLogLog(10);
        HyperLogLog right = new HyperLogLog(10);
        for (int index = 0; index < 200; index++) {
            left.add("left-" + index);
            right.add("right-" + index);
        }

        HyperLogLog merged = left.merge(right);
        assertTrue(merged.count() >= left.count());
        assertTrue(merged.count() >= right.count());
    }

    @Test
    void rejectsPrecisionMismatches() {
        HyperLogLog left = new HyperLogLog(10);
        HyperLogLog right = new HyperLogLog(14);
        assertNull(left.tryMerge(right));
        assertThrows(HyperLogLogError.class, () -> left.merge(right));
    }

    @Test
    void exposesHelperMath() {
        assertEquals(12_288, HyperLogLog.memoryBytes(14));
        assertEquals(14, HyperLogLog.optimalPrecision(0.01));
        assertTrue(HyperLogLog.errorRateForPrecision(14) > 0.008);
    }

    @Test
    void supportsConstructionCloningAndRepresentation() {
        assertThrows(HyperLogLogError.class, () -> new HyperLogLog(2));
        assertNull(HyperLogLog.tryWithPrecision(99));

        HyperLogLog first = HyperLogLog.withPrecision(8);
        first.addBytes(new byte[]{1, 2, 3});
        first.add(true).add(123).add("sample");

        HyperLogLog clone = first.copy();

        assertEquals(first, clone);
        assertEquals(first.hashCode(), clone.hashCode());
        assertEquals(8, first.precision());
        assertEquals(256, first.numRegisters());
        assertTrue(first.toString().contains("precision=8"));
    }
}
