package com.codingadventures.kotlin.microbenchmark

import com.codingadventures.rankstatistics.RankStatistics
import java.util.Collections
import java.util.Random

private const val MAXIMUM_PAIRS = 10_000

data class Config(
    val pairs: Int = 40,
    val confidencePercent: Int = 95,
    val minimumFactor: Double = 1.05,
    val seed: Long = 0,
    val targetNs: Long = 100_000,
    val maxBatch: Long = 65_536,
) {
    fun validate() {
        require(pairs in 1..MAXIMUM_PAIRS) { "pairs must be from 1 through 10000" }
        require(confidencePercent in 1..99) { "confidence must be from 1 through 99" }
        require(minimumFactor.isFinite() && minimumFactor > 1.0) { "factor must be finite and greater than 1" }
        require(targetNs > 0 && maxBatch > 0) { "target and max batch must be positive signed 64-bit integers" }
    }
}

data class Observation(val baselineNs: Long, val candidateNs: Long, val order: String)
data class Analysis(val ratio: Double, val low: Double?, val high: Double?, val verdict: String) {
    val hasInterval: Boolean get() = low != null && high != null
}
data class Report(val config: Config, val batchSize: Long, val pairs: List<Observation>, val analysis: Analysis)
typealias Batch = (Long) -> Long
typealias Clock = () -> Long

/**
 * Paired collector with deterministic balance and exact shared rank statistics.
 * Each sample is an adjacent pair at one common batch size; the seeded shuffle
 * distributes which implementation runs first, without relying on global RNG state.
 */
object Microbenchmark {
    const val BASELINE_FIRST = "baseline-first"
    const val CANDIDATE_FIRST = "candidate-first"

    fun analyze(pairs: List<Observation>, confidencePercent: Int, factor: Double): Analysis {
        require(pairs.isNotEmpty()) { "at least one pair is required" }
        require(pairs.size <= MAXIMUM_PAIRS) { "pairs must be from 1 through 10000" }
        require(confidencePercent in 1..99) { "confidence must be from 1 through 99" }
        require(factor.isFinite() && factor > 1.0) { "factor must be finite and greater than 1" }
        val ratios = pairs.map { pair ->
            require(pair.order == BASELINE_FIRST || pair.order == CANDIDATE_FIRST) { "pair has invalid order" }
            require(pair.baselineNs > 0 && pair.candidateNs > 0) { "pair durations must be positive signed 64-bit integers" }
            pair.candidateNs.toDouble() / pair.baselineNs.toDouble()
        }.toDoubleArray()
        val median = RankStatistics.median(ratios)
        val interval = RankStatistics.medianInterval(ratios, confidencePercent)
            ?: return Analysis(median, null, null, "inconclusive")
        val fast = 1.0 / factor
        val verdict = when {
            interval.high < fast -> "faster"
            interval.low > factor -> "slower"
            interval.low >= fast && interval.high <= factor -> "equivalent"
            else -> "inconclusive"
        }
        return Analysis(median, interval.low, interval.high, verdict)
    }

    fun collect(config: Config, expectedChecksum: Long, baseline: Batch, candidate: Batch, clock: Clock): Report {
        config.validate()
        var last = -1L
        fun read(): Long {
            val tick = clock()
            require(tick >= 0 && (last < 0 || tick >= last)) { "clock must be nonnegative and nonregressing" }
            last = tick
            return tick
        }
        fun wanted(size: Long) = Math.multiplyExact(expectedChecksum, size)
        fun verify(batch: Batch) { require(batch(1) == wanted(1)) { "checksum mismatch" } }
        fun measure(batch: Batch, size: Long, calibration: Boolean): Long {
            val expected = wanted(size)
            val start = read()
            val checksum = batch(size)
            val end = read()
            require(checksum == expected) { "checksum mismatch" }
            require(end > start || calibration && end == start) { "measured duration must be positive" }
            return end - start
        }
        verify(baseline)
        verify(candidate)
        var size = 1L
        while (true) {
            val baseNs = measure(baseline, size, true)
            val candidateNs = measure(candidate, size, true)
            if (baseNs >= config.targetNs && candidateNs >= config.targetNs) break
            require(size < config.maxBatch) { "calibration limit exhausted" }
            size = if (size > config.maxBatch / 2) config.maxBatch else size * 2
        }
        val schedule = MutableList(config.pairs) { if (it < (config.pairs + 1) / 2) BASELINE_FIRST else CANDIDATE_FIRST }
        Collections.shuffle(schedule, Random(config.seed))
        val observations = schedule.map { order ->
            val b: Long
            val c: Long
            if (order == BASELINE_FIRST) { b = measure(baseline, size, false); c = measure(candidate, size, false) }
            else { c = measure(candidate, size, false); b = measure(baseline, size, false) }
            Observation(b, c, order)
        }
        return Report(config, size, observations, analyze(observations, config.confidencePercent, config.minimumFactor))
    }

    /** Relative monotonic ticks avoid exposing System.nanoTime's arbitrary origin. */
    fun monotonicClock(): Clock {
        val origin = System.nanoTime()
        return { System.nanoTime() - origin }
    }
}
