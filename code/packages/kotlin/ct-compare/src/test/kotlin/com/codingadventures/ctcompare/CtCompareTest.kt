package com.codingadventures.ctcompare

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class CtCompareTest {
    @Test
    fun ctEqMatchesByteEquality() {
        assertTrue(CtCompare.ctEq("abcdef".encodeToByteArray(), "abcdef".encodeToByteArray()))
        assertTrue(CtCompare.ctEq(ByteArray(0), ByteArray(0)))
        assertFalse(CtCompare.ctEq("abcdef".encodeToByteArray(), "abcdeg".encodeToByteArray()))
        assertFalse(CtCompare.ctEq("abcdef".encodeToByteArray(), "bbcdef".encodeToByteArray()))
        assertFalse(CtCompare.ctEq("abc".encodeToByteArray(), "abcd".encodeToByteArray()))
    }

    @Test
    fun ctEqDetectsEverySingleBitPosition() {
        val baseline = ByteArray(32) { 0x42 }
        for (index in baseline.indices) {
            for (bit in 0 until 8) {
                val flipped = baseline.copyOf()
                flipped[index] = (flipped[index].toInt() xor (1 shl bit)).toByte()
                assertFalse(CtCompare.ctEq(baseline, flipped))
            }
        }
    }

    @Test
    fun ctEqFixedIsDynamicAlias() {
        assertTrue(CtCompare.ctEqFixed(ByteArray(16), ByteArray(16)))
        val different = ByteArray(16)
        different[15] = 1
        assertFalse(CtCompare.ctEqFixed(ByteArray(16), different))
    }

    @Test
    fun ctSelectBytesChoosesWithoutMutatingInputs() {
        val left = ByteArray(256) { it.toByte() }
        val right = ByteArray(256) { (255 - it).toByte() }

        assertContentEquals(left, CtCompare.ctSelectBytes(left, right, 1))
        assertContentEquals(right, CtCompare.ctSelectBytes(left, right, 0))
        assertContentEquals(ByteArray(0), CtCompare.ctSelectBytes(ByteArray(0), ByteArray(0), 1))
        assertFailsWith<IllegalArgumentException> {
            CtCompare.ctSelectBytes(ByteArray(1), ByteArray(2), 1)
        }
    }

    @Test
    fun ctEqU64HandlesEdges() {
        assertTrue(CtCompare.ctEqU64(0L, 0L))
        assertTrue(CtCompare.ctEqU64(-1L, -1L))
        assertFalse(CtCompare.ctEqU64(0L, Long.MIN_VALUE))

        val baseline = 0x1234_5678_9ABC_DEF0L
        for (bit in 0 until 64) {
            assertFalse(CtCompare.ctEqU64(baseline, baseline xor (1L shl bit)))
        }
    }

    @Test
    fun ctSelectBytesRejectsChoiceOtherThanZeroOrOne() {
        for (bad in intArrayOf(2, -1, 3, 255, Int.MIN_VALUE)) {
            assertFailsWith<IllegalArgumentException> {
                CtCompare.ctSelectBytes(byteArrayOf(1), byteArrayOf(2), bad)
            }
        }
    }

    @Test
    fun ctSelectBytesReturnsAFreshCopy() {
        val left = byteArrayOf(1, 2, 3)
        val picked = CtCompare.ctSelectBytes(left, byteArrayOf(4, 5, 6), 1)
        left[0] = 99
        assertContentEquals(byteArrayOf(1, 2, 3), picked)
    }
}
