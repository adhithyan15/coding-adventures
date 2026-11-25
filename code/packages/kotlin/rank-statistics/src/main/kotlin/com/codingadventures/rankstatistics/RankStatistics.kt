package com.codingadventures.rankstatistics

import java.math.BigInteger

/** Reusable order statistics over finite binary64 observations. */
object RankStatistics {
    data class Interval(val low: Double, val high: Double)

    private fun sortedFinite(values: DoubleArray): DoubleArray {
        require(values.isNotEmpty()) { "sample must not be empty" }
        val copy = values.copyOf()
        require(copy.all(Double::isFinite)) { "sample values must be finite" }
        copy.sort()
        return copy
    }

    private fun midpoint(low: Double, high: Double): Double {
        val sum = low + high
        // Add first to retain subnormal tie-to-even rounding; split only on overflow.
        return if (sum.isFinite()) sum / 2.0 else low / 2.0 + high / 2.0
    }

    /** Returns the sample median without mutating the caller's array. */
    fun median(values: DoubleArray): Double {
        val ordered = sortedFinite(values)
        val middle = ordered.size / 2
        return if (ordered.size % 2 == 1) ordered[middle] else midpoint(ordered[middle - 1], ordered[middle])
    }

    /** Returns an exact closed interval, or null when no rank qualifies. */
    fun medianInterval(values: DoubleArray, confidencePercent: Int): Interval? {
        require(confidencePercent in 1..99) { "confidence must be from 1 through 99" }
        val ordered = sortedFinite(values)
        val rank = exactMedianRank(ordered.size, confidencePercent)
        return if (rank == 0) null else Interval(ordered[rank - 1], ordered[ordered.size - rank])
    }

    /**
     * Largest qualifying one-based rank; zero means the interval is unavailable.
     * The recurrence generates C(n,j) exactly. Its accumulated lower tail is
     * compared after clearing the probability denominator, so no rounded
     * floating probability can shift a confidence-boundary rank.
     */
    @JvmSynthetic
    internal fun exactMedianRank(n: Int, confidencePercent: Int): Int {
        var coefficient = BigInteger.ONE
        var tail = BigInteger.ZERO
        val maxTail = BigInteger.valueOf((100 - confidencePercent).toLong()).shiftLeft(n)
        val twoHundred = BigInteger.valueOf(200)
        var rank = 0
        for (j in 0..(n / 2)) {
            tail = tail.add(coefficient)
            if (tail.multiply(twoHundred) > maxTail) break
            rank = j + 1
            coefficient = coefficient.multiply(BigInteger.valueOf(n.toLong() - j)).divide(BigInteger.valueOf(j + 1L))
        }
        return rank
    }
}
