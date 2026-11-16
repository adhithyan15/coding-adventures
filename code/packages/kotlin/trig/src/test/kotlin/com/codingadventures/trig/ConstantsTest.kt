// ============================================================================
// ConstantsTest.kt — the constants' bits equal the fixture manifest (PHY00a 6.5 e)
// ============================================================================
//
// Kotlin has no hexadecimal floating-point literal, so the reference constants
// are built with Double.fromBits (or written as decimals).  Either way the test
// decodes the constant's own bits with Double.toRawBits and compares them to
// the manifest's table, so a typo in a hex digit cannot survive.

package com.codingadventures.trig

import java.nio.file.Files
import java.nio.file.Paths
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ConstantsTest {

    private val manifest: String = String(
        Files.readAllBytes(Paths.get("../../../specs/fixtures/trig-reference-v1/manifest.json")),
        Charsets.UTF_8,
    )

    private fun manifestConstant(name: String): Long {
        val c = Regex("\"$name\": \"([0-9a-f]{16})\"").find(manifest)
        assertTrue(c != null, "manifest constant $name")
        return java.lang.Long.parseUnsignedLong(c.groupValues[1], 16)
    }

    @Test
    fun constantsMatchTheManifest() {
        assertEquals(manifestConstant("PI"), Trig.PI.toRawBits())
        assertEquals(manifestConstant("TWO_PI"), Trig.TWO_PI.toRawBits())
        assertEquals(manifestConstant("HALF_PI"), Trig.HALF_PI.toRawBits())
        assertEquals(manifestConstant("INV_PIO2"), Trig.INV_PIO2.toRawBits())
        for (i in 0 until 5) {
            assertEquals(manifestConstant("PIO2_${i + 1}"), Trig.PIO2[i].toRawBits(), "PIO2_${i + 1}")
        }
        assertEquals(manifestConstant("MAGIC"), Trig.MAGIC.toRawBits())
        assertEquals(manifestConstant("LIMIT"), Trig.LIMIT.toRawBits())
        assertEquals(manifestConstant("TINY"), Trig.TINY.toRawBits())
        for (n in 1..9) assertEquals(manifestConstant("S$n"), Trig.S[n].toRawBits(), "S$n")
        for (n in 2..9) assertEquals(manifestConstant("C$n"), Trig.C[n].toRawBits(), "C$n")
        assertEquals(manifestConstant("RAD_PER_DEG"), Trig.RAD_PER_DEG.toRawBits())
        assertEquals(manifestConstant("DEG_PER_RAD"), Trig.DEG_PER_RAD.toRawBits())
    }

    /** Decimal literals the source still writes parse to the pinned bit patterns. */
    @Test
    fun decimalLiteralsHaveThePinnedBits() {
        assertEquals(0x3CD203AF9EE75616L, 1e-15.toRawBits())     // tan pole threshold, sqrt tolerance
        assertEquals(0x7FE1CCF385EBC8A0L, 1.0e308.toRawBits())   // tan pole sentinel
        assertEquals(0x3C670EF54646D497L, 1e-17.toRawBits())     // atan series cut-off
        assertEquals(0x01A56E1FC2F8F359L, 1e-300.toRawBits())    // sqrt subnormal floor
        assertEquals(0x3E40000000000000L, 7.450580596923828e-9.toRawBits()) // 2^-27
    }

    @Test
    fun degreeConstantsEqualTheDivisions() {
        assertEquals(Trig.RAD_PER_DEG, Trig.PI / 180.0)
        assertEquals(Trig.DEG_PER_RAD, 180.0 / Trig.PI)
    }
}
