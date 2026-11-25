package com.codingadventures.rankstatistics

import com.fasterxml.jackson.databind.ObjectMapper
import java.nio.file.Files
import java.nio.file.Path
import java.util.Random
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class RankStatisticsTest {
    @Test fun medianEdgesAndOwnership() {
        val values = doubleArrayOf(4.0, 1.0, 3.0, 2.0)
        assertEquals(2.5, RankStatistics.median(values))
        assertContentEquals(doubleArrayOf(4.0, 1.0, 3.0, 2.0), values)
        val d = Math.nextUp(0.0)
        assertEquals(2 * d, RankStatistics.median(doubleArrayOf(d, 2 * d)))
        assertEquals(-2 * d, RankStatistics.median(doubleArrayOf(-2 * d, -d)))
        val normalMin = java.lang.Double.MIN_NORMAL
        assertEquals(normalMin, RankStatistics.median(doubleArrayOf(Math.nextDown(normalMin), normalMin)))
        assertEquals(-normalMin, RankStatistics.median(doubleArrayOf(-normalMin, -Math.nextDown(normalMin))))
        assertEquals(0.0, RankStatistics.median(doubleArrayOf(-Double.MAX_VALUE, Double.MAX_VALUE)))
        assertEquals(Double.MAX_VALUE, RankStatistics.median(doubleArrayOf(Double.MAX_VALUE, Double.MAX_VALUE)))
        assertFailsWith<IllegalArgumentException> { RankStatistics.median(doubleArrayOf()) }
        assertFailsWith<IllegalArgumentException> { RankStatistics.median(doubleArrayOf(Double.NaN)) }
        assertNull(RankStatistics.medianInterval(doubleArrayOf(1.0), 95))
        assertFailsWith<IllegalArgumentException> { RankStatistics.medianInterval(doubleArrayOf(1.0), 0) }
    }

    @Test fun exactCutoffMatchesIndependentSignEnumeration() {
        for (n in 1..12) for (confidence in 1..99) {
            var want = 0
            for (k in 1..(n / 2)) {
                var tail = 0L
                for (mask in 0 until (1L shl n)) if (java.lang.Long.bitCount(mask) < k) tail++
                if (200 * tail <= (100 - confidence) * (1L shl n)) want = k else break
            }
            assertEquals(want, RankStatistics.exactMedianRank(n, confidence))
        }
    }

    @Test fun deterministicGeneratedSamplesRespectOrderInvariants() {
        val random = Random(0x5eedL)
        repeat(500) {
            val values = DoubleArray(1 + random.nextInt(96)) { (random.nextInt(2001) - 1000).toDouble() }
            val original = values.copyOf()
            val median = RankStatistics.median(values)
            val interval = RankStatistics.medianInterval(values, 90)
            assertTrue(median in -1000.0..1000.0)
            if (interval != null) assertTrue(interval.low <= median && median <= interval.high)
            assertContentEquals(original, values)
        }
    }

    @Test fun canonicalFixtureDoesNotDrift() {
        val local = Path.of("testdata", "microbenchmark-pairs-v1.json")
        val canonical = Path.of("../../../specs/fixtures/microbenchmark-pairs-v1.json")
        assertContentEquals(Files.readAllBytes(canonical), Files.readAllBytes(local))
        val root = ObjectMapper().readTree(local.toFile())
        for (sample in root.path("cases")) {
            val ratios = sample.path("candidate_ns").map { it.longValue() / sample.path("baseline_ns").longValue().toDouble() }.toDoubleArray()
            assertEquals(sample.path("ratio").doubleValue(), RankStatistics.median(ratios), 1e-12)
            val ci = RankStatistics.medianInterval(ratios, root.path("confidence_percent").intValue())
            val expected = sample.path("interval")
            if (expected.isNull) assertNull(ci) else {
                assertTrue(ci != null)
                assertEquals(expected[0].doubleValue(), ci!!.low, 1e-12)
                assertEquals(expected[1].doubleValue(), ci.high, 1e-12)
            }
        }
    }

    @Test fun wideExactCutoffVectorsPinOrderStatisticEndpoints() {
        val vectors = listOf(
            intArrayOf(64, 95, 23, 40), intArrayOf(65, 99, 21, 43),
            intArrayOf(127, 95, 51, 75), intArrayOf(128, 95, 52, 75),
            intArrayOf(256, 99, 106, 149), intArrayOf(1000, 95, 468, 531),
            intArrayOf(10000, 50, 4965, 5034), intArrayOf(10000, 95, 4901, 5098),
            intArrayOf(10000, 99, 4870, 5129),
        )
        for ((size, confidence, low, high) in vectors) {
            val sample = DoubleArray(size) { it.toDouble() }
            val interval = RankStatistics.medianInterval(sample, confidence)
            assertTrue(interval != null)
            assertEquals(low.toDouble(), interval!!.low)
            assertEquals(high.toDouble(), interval.high)
        }
    }
}
