// ============================================================================
// ConformanceTest.kt — bit-exact conformance against the TRIG-REF-1 vectors
// ============================================================================
//
// code/specs/fixtures/trig-reference-v1 holds every vector as one row
//
//     op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// where x and y are IEEE binary64 *bit patterns* (16 lowercase hex digits),
// never decimals.  This test decodes x with Double.fromBits, calls the public
// function, encodes the result with Double.toRawBits and compares the two
// 64-bit integers (PHY00a 6.5).  Printed doubles are never compared: they hide
// the last digits.  The only relaxation is NaN, which matches any NaN (VER04p
// "same": the payload and sign of a NaN are not portable).
//
// Three kinds of check live here:
//
//   1. fixture integrity  — the files on disk are the ones the manifest hashes;
//   2. bit-for-bit match  — every row, per group, failing rows counted per op;
//   3. accuracy vs `cr`   — every row that carries a correctly rounded value is
//                           within the stated number of representable steps.
//
// plus the named contract points of PHY00a 3.3.  The constants' bits are
// asserted in ConstantsTest.

package com.codingadventures.trig

import java.nio.file.Files
import java.nio.file.Paths
import java.security.MessageDigest
import java.util.TreeMap
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.test.fail

class ConformanceTest {

    /** Gradle runs tests with the project directory (code/packages/kotlin/trig) as cwd. */
    private val fixture = Paths.get("../../../specs/fixtures/trig-reference-v1")

    private val groups = listOf("edge", "sweep-binade", "sweep-cw", "sweep-prng")
    private val qnanBits = 0x7FF8000000000000L

    /** One parsed fixture row. */
    private data class Row(val op: String, val x: String, val y: String, val cr: String, val flags: String)

    // -------------------------------------------------------------------------
    // Helpers: bit casts, parsing, dispatch
    // -------------------------------------------------------------------------

    private fun fromHex(hex: String): Double = Double.fromBits(java.lang.Long.parseUnsignedLong(hex, 16))

    private fun load(group: String): List<Row> {
        val text = String(Files.readAllBytes(fixture.resolve("$group.tsv")), Charsets.UTF_8)
        assertTrue(text.endsWith("\n"), "$group: final newline")
        val rows = ArrayList<Row>()
        for (line in text.split("\n")) {
            if (line.isEmpty()) continue
            val f = line.split("\t")
            assertEquals(5, f.size, line)
            rows.add(Row(f[0], f[1], f[2], f[3], f[4]))
        }
        return rows
    }

    /** Call the public function named by [op]; an unknown op fails the test. */
    private fun call(op: String, x: String): Double {
        if (op == "atan2") {
            val comma = x.indexOf(',')
            return Trig.atan2(fromHex(x.substring(0, comma)), fromHex(x.substring(comma + 1)))
        }
        val v = fromHex(x)
        return when (op) {
            "sin" -> Trig.sin(v)
            "cos" -> Trig.cos(v)
            "tan" -> Trig.tan(v)
            "radians" -> Trig.radians(v)
            "degrees" -> Trig.degrees(v)
            "sqrt" -> Trig.sqrt(v)
            "atan" -> Trig.atan(v)
            else -> fail("unknown op in fixture: $op")
        }
    }

    /** True when the row's result matches the reference under VER04p "same". */
    private fun matches(r: Row): Boolean {
        if (r.y == "error") {
            // sqrt of a negative: this port's idiomatic error is ArithmeticException.
            return try {
                call(r.op, r.x)
                false
            } catch (expected: ArithmeticException) {
                true
            }
        }
        val got = call(r.op, r.x)
        val want = java.lang.Long.parseUnsignedLong(r.y, 16)
        if (want == qnanBits) return got.isNaN()
        return got.toRawBits() == want
    }

    // -------------------------------------------------------------------------
    // 1. Fixture integrity
    // -------------------------------------------------------------------------

    private fun manifest(): String =
        String(Files.readAllBytes(fixture.resolve("manifest.json")), Charsets.UTF_8)

    @Test
    fun manifestHashesAndCountsMatchTheFiles() {
        val m = manifest()
        assertTrue(m.contains("\"algorithm\": \"TRIG-REF-1\""))
        for (group in groups) {
            val data = Files.readAllBytes(fixture.resolve("$group.tsv"))
            val sha = MessageDigest.getInstance("SHA-256").digest(data).joinToString("") { "%02x".format(it) }
            val g = Regex(
                "\"file\": \"$group\\.tsv\",\\s*\"name\": \"$group\",\\s*\"rows\": (\\d+),\\s*\"sha256\": \"([0-9a-f]{64})\""
            ).find(m)
            assertTrue(g != null, "manifest group $group")
            assertEquals(g.groupValues[2], sha, "$group sha256")
            assertEquals(g.groupValues[1].toLong(), data.count { it == '\n'.code.toByte() }.toLong(), "$group row count")
        }
    }

    @Test
    fun rowSyntaxIsClosed() {
        val hex = Regex("[0-9a-f]{16}")
        val ops = listOf("sin", "cos", "tan", "radians", "degrees", "sqrt", "atan", "atan2")
        for (group in groups) {
            for (r in load(group)) {
                assertTrue(r.op in ops, "op ${r.op}")
                val inputs = if (r.op == "atan2") r.x.split(",") else listOf(r.x)
                assertEquals(if (r.op == "atan2") 2 else 1, inputs.size)
                for (input in inputs) assertTrue(hex.matches(input), "x field $input")
                assertTrue(r.y == "error" || hex.matches(r.y), "y ${r.y}")
                assertTrue(r.cr == "-" || hex.matches(r.cr), "cr ${r.cr}")
            }
        }
    }

    // -------------------------------------------------------------------------
    // 2. Bit-for-bit match, per group, failing rows counted per op
    // -------------------------------------------------------------------------

    private fun checkGroup(group: String) {
        val failing = TreeMap<String, Int>()
        val first = ArrayList<String>()
        var checked = 0
        for (r in load(group)) {
            checked++
            if (!matches(r)) {
                failing.merge(r.op, 1) { a, b -> a + b }
                if (first.size < 8) first.add("${r.op} ${r.x} want ${r.y}")
            }
        }
        assertTrue(checked > 0, "$group has rows")
        assertTrue(failing.isEmpty(), "$group: failing rows per op $failing; first $first")
    }

    @Test fun edgeVectorsMatchBitForBit() = checkGroup("edge")
    @Test fun sweepBinadeVectorsMatchBitForBit() = checkGroup("sweep-binade")
    @Test fun sweepCwVectorsMatchBitForBit() = checkGroup("sweep-cw")
    @Test fun sweepPrngVectorsMatchBitForBit() = checkGroup("sweep-prng")

    // -------------------------------------------------------------------------
    // 3. Accuracy against the correctly rounded column
    // -------------------------------------------------------------------------

    /** Map a double's bits onto a monotone integer line so that steps can be counted. */
    private fun ordered(bits: Long): Long {
        val mag = bits and Long.MAX_VALUE
        return if (bits < 0) -mag else mag
    }

    @Test
    fun accuracyAgainstTheCorrectlyRoundedColumn() {
        val limit = mapOf("sin" to 3L, "cos" to 3L, "radians" to 1L, "degrees" to 1L)
        // PHY00a 3.8: NO accuracy claim for tan (sin / cos plus a division and a pinned
        // pole sentinel), although tan rows carry a cr.  A tan gate is a separate follow-up.
        val noClaim = "tan"
        val worst = TreeMap<String, Long>()
        for (group in groups) {
            for (r in load(group)) {
                if (r.cr == "-" || r.op == noClaim) continue
                // Fail closed: any other op with a cr value must have a stated limit.
                assertTrue(r.op in limit, "op ${r.op} has a cr value but no accuracy limit")
                val got = call(r.op, r.x).toRawBits()
                val steps = Math.abs(ordered(got) - ordered(java.lang.Long.parseUnsignedLong(r.cr, 16)))
                worst.merge(r.op, steps) { a, b -> maxOf(a, b) }
            }
        }
        for ((op, lim) in limit) {
            val w = worst[op] ?: 0L
            assertTrue(w <= lim, "$op: worst $w steps > $lim")
        }
    }

    @Test
    fun anUnknownOpFails() {
        assertFailsWith<AssertionError> { call("sinh", "0000000000000000") }
    }

    // -------------------------------------------------------------------------
    // Contract points of PHY00a 3.3 asserted directly
    // -------------------------------------------------------------------------

    @Test
    fun sinOfNegativeZeroIsNegativeZero() {
        assertEquals(Long.MIN_VALUE, Trig.sin(-0.0).toRawBits())
        assertEquals(0L, Trig.sin(0.0).toRawBits())
    }

    @Test
    fun cosOfZeroIsOne() {
        assertEquals(1.0, Trig.cos(0.0))
        assertEquals(1.0, Trig.cos(-0.0))
    }

    @Test
    fun tinyArgumentsAreReturnedUnchanged() {
        val tiny = fromHex("3e3fffffffffffff")
        assertEquals(tiny.toRawBits(), Trig.sin(tiny).toRawBits())
        assertEquals(Double.MIN_VALUE.toRawBits(), Trig.sin(Double.MIN_VALUE).toRawBits())
        assertEquals(1.0, Trig.cos(tiny))
    }

    @Test
    fun outsideTheDomainIsNaN() {
        val xs = doubleArrayOf(
            Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY, 1e20,
            536870912.0 * (1.0 + Math.ulp(1.0)), Double.MAX_VALUE,
        )
        for (x in xs) {
            assertTrue(Trig.sin(x).isNaN(), "sin $x")
            assertTrue(Trig.cos(x).isNaN(), "cos $x")
            assertTrue(Trig.tan(x).isNaN(), "tan $x")
        }
    }

    @Test
    fun theLastInDomainValueIsFinite() {
        assertEquals(0x3FD4E67C0E2622DFL, Trig.sin(536870912.0).toRawBits())
        assertEquals(0xBFEE3EDD2DFEEF90uL.toLong(), Trig.cos(536870912.0).toRawBits())
    }

    @Test
    fun radiansAndDegreesAreSingleMultiplies() {
        assertEquals(Trig.PI, Trig.radians(180.0))
        assertEquals(180.0, Trig.degrees(Trig.PI))
        assertEquals(Double.POSITIVE_INFINITY, Trig.degrees(1.7e308))
        assertTrue(Trig.radians(Double.NaN).isNaN())
        assertEquals(Long.MIN_VALUE, Trig.radians(-0.0).toRawBits())
        assertEquals(Long.MIN_VALUE, Trig.degrees(-0.0).toRawBits())
    }

    @Test
    fun tanPoleSentinel() {
        assertEquals(1e308, Trig.tan(Trig.PI / 2.0))
        assertEquals(-1e308, Trig.tan(-Trig.PI / 2.0))
    }

    @Test
    fun atan2OfNaNIsNaN() {
        val nan = Double.NaN
        assertTrue(Trig.atan2(nan, 1.0).isNaN())
        assertTrue(Trig.atan2(1.0, nan).isNaN())
        assertTrue(Trig.atan2(-1.0, nan).isNaN())
        assertTrue(Trig.atan2(nan, nan).isNaN())
        assertTrue(Trig.atan2(nan, -1.0).isNaN())
        assertTrue(Trig.atan2(nan, 0.0).isNaN())
    }

    @Test
    fun atan2PinnedConventions() {
        assertEquals(Trig.PI, Trig.atan2(-0.0, -1.0))
        assertEquals(0.0, Trig.atan2(0.0, 0.0))
        assertTrue(Trig.atan2(Double.POSITIVE_INFINITY, Double.POSITIVE_INFINITY).isNaN())
        assertEquals(Trig.PI / 2.0, Trig.atan2(1e300, 1e-300))
    }

    @Test
    fun atanKeepsTheSignOfZero() {
        assertEquals(Long.MIN_VALUE, Trig.atan(-0.0).toRawBits())
    }

    @Test
    fun negativeQuadrantArithmetic() {
        // k = -1 and k = -5 map to quadrant 3: "and 3" is the floor modulus.
        val halfPi = Trig.PI / 2.0
        assertEquals(-Trig.sin(halfPi), Trig.sin(-halfPi))
        assertEquals(0.0, Trig.cos(-5 * halfPi), 1e-15)
        assertFalse(Trig.sin(-5 * halfPi).isNaN())
    }
}
