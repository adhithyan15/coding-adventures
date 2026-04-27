package com.codingadventures.hyperloglog

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class HyperLogLogTest {
    @Test
    fun startsEmpty() {
        val hll = HyperLogLog()
        assertEquals(0, hll.count())
        assertEquals(0, hll.len())
    }

    @Test
    fun ignoresDuplicatesAndGrowsForUniqueValues() {
        val hll = HyperLogLog()
        repeat(1_000) { hll.add("same") }
        assertTrue(hll.count() < 10)

        val spread = HyperLogLog()
        repeat(1_000) { index -> spread.add("item-$index") }
        assertTrue(spread.count() in 800..1_200)
    }

    @Test
    fun mergesSketchesWithSamePrecision() {
        val left = HyperLogLog(10)
        val right = HyperLogLog(10)
        repeat(200) { index ->
            left.add("left-$index")
            right.add("right-$index")
        }

        val merged = left.merge(right)
        assertTrue(merged.count() >= left.count())
        assertTrue(merged.count() >= right.count())
    }

    @Test
    fun rejectsPrecisionMismatches() {
        val left = HyperLogLog(10)
        val right = HyperLogLog(14)
        assertNull(left.tryMerge(right))
        assertFailsWith<HyperLogLogError> { left.merge(right) }
    }

    @Test
    fun exposesHelperMath() {
        assertEquals(12_288, HyperLogLog.memoryBytes(14))
        assertEquals(14, HyperLogLog.optimalPrecision(0.01))
        assertTrue(HyperLogLog.errorRateForPrecision(14) > 0.008)
    }

    @Test
    fun supportsConstructionCloningAndRepresentation() {
        assertFailsWith<HyperLogLogError> { HyperLogLog(2) }
        assertNull(HyperLogLog.tryWithPrecision(99))

        val first = HyperLogLog.withPrecision(8)
        first.addBytes(byteArrayOf(1, 2, 3))
        first.add(true).add(123).add("sample")

        val clone = first.copy()

        assertEquals(first, clone)
        assertEquals(first.hashCode(), clone.hashCode())
        assertEquals(8, first.precision())
        assertEquals(256, first.numRegisters())
        assertTrue(first.toString().contains("precision=8"))
    }
}
