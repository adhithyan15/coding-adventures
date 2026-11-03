package com.codingadventures.rng

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import kotlin.test.fail

/**
 * The shared rng corpus (spec rng.md, "Conformance corpus"), read from its
 * flat form `code/specs/fixtures/rng-v1/cases.txt`.
 *
 * The corpus pins exact outputs every language's rng package must reproduce;
 * its results come from the independent reference
 * `code/scripts/rng_conformance.py`. Each line is whitespace-separated:
 *
 * ```
 *  case <id> <generator> <seed>
 *  next_u32 <result>
 *  next_u64 <result>
 *  next_float <16 hex digits of the IEEE-754 bits>
 *  next_int_in_range <min> <max> <result | error>
 *  end
 * ```
 *
 * Gradle runs tests from the project directory, so the path is relative to
 * `code/packages/kotlin/rng`.
 */
class ConformanceTest {

    /** The three generators behind one interface. */
    private class AnyRng(name: String, seed: Long) {
        private val lcg = if (name == "lcg") Lcg(seed) else null
        private val xs = if (name == "xorshift64") Xorshift64(seed) else null
        private val pcg = if (name == "pcg32") Pcg32(seed) else null

        init {
            require(lcg != null || xs != null || pcg != null) { "unknown generator $name" }
        }

        fun u32(): Long = lcg?.nextU32() ?: xs?.nextU32() ?: pcg!!.nextU32()
        fun u64(): Long = lcg?.nextU64() ?: xs?.nextU64() ?: pcg!!.nextU64()
        fun f64(): Double = lcg?.nextFloat() ?: xs?.nextFloat() ?: pcg!!.nextFloat()
        fun inRange(min: Long, max: Long): Long =
            lcg?.nextIntInRange(min, max) ?: xs?.nextIntInRange(min, max) ?: pcg!!.nextIntInRange(min, max)
    }

    @Test
    fun everyCorpusCaseMatches() {
        var cases = 0
        var calls = 0
        var id = ""
        var g: AnyRng? = null
        for (line in File("../../../specs/fixtures/rng-v1/cases.txt").readLines()) {
            if (line.isEmpty() || line.startsWith("#")) continue
            val t = line.split(" ")
            when (t[0]) {
                "case" -> {
                    id = t[1]
                    g = AnyRng(t[2], t[3].toULong().toLong())
                    cases++
                }
                "end" -> g = null
                "next_u32" -> {
                    calls++
                    assertEquals(t[1].toULong().toLong(), g!!.u32(), id)
                }
                "next_u64" -> {
                    calls++
                    assertEquals(t[1].toULong().toLong(), g!!.u64(), id)
                }
                "next_float" -> {
                    calls++
                    assertEquals(t[1].toULong(16).toLong(), g!!.f64().toRawBits(), id)
                }
                "next_int_in_range" -> {
                    calls++
                    val min = t[1].toLong()
                    val max = t[2].toLong()
                    val gen = g!!
                    if (t[3] == "error") {
                        // Thrown before any draw, so the sequence carries on.
                        assertFailsWith<IllegalArgumentException>(id) { gen.inRange(min, max) }
                    } else {
                        assertEquals(t[3].toLong(), gen.inRange(min, max), id)
                    }
                }
                else -> fail("malformed corpus line: $line")
            }
        }
        // Guard against a truncated or empty file passing vacuously.
        assertTrue(cases >= 27, "cases: $cases")
        assertTrue(calls >= 150, "calls: $calls")
    }

    // ── Regressions for the v1 range bug ─────────────────────────────────────

    @Test
    fun wholeLongDomainNoLongerDividesByZero() {
        val g = Lcg(3)
        val values = List(64) { g.nextIntInRange(Long.MIN_VALUE, Long.MAX_VALUE) }
        assertTrue(values.any { it < 0 } && values.any { it > 0 })
    }

    @Test
    fun wideRangesReachBeyondTwoToThe32() {
        val g = Pcg32(3)
        assertTrue(List(64) { g.nextIntInRange(0, 1L shl 40) }.any { it >= 1L shl 32 })
    }

    @Test
    fun halfDomainPlusOneTerminates() {
        val g = Xorshift64(3)
        repeat(100) { assertTrue(g.nextIntInRange(Long.MIN_VALUE, 0) <= 0) }
    }
}
