package com.codingadventures.ctcompare

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals

/** The bit-returning equality variants (CT01): agree with the Booleans, 0 or 1 only. */
class CtCompareBitTest {
    @Test
    fun ctEqBitAgreesOnEveryBytePair() {
        for (a in 0 until 256) {
            for (b in 0 until 256) {
                val left = byteArrayOf(a.toByte())
                val right = byteArrayOf(b.toByte())
                val want = if (CtCompare.ctEq(left, right)) 1 else 0
                assertEquals(want, CtCompare.ctEqBit(left, right), "$a vs $b")
                assertEquals(want, CtCompare.ctEqFixedBit(left, right), "$a vs $b")
            }
        }
    }

    @Test
    fun lengthsAndEveryBitPosition() {
        assertEquals(1, CtCompare.ctEqBit(ByteArray(0), ByteArray(0)))
        assertEquals(0, CtCompare.ctEqBit(byteArrayOf(1), byteArrayOf(1, 2)))
        val base = ByteArray(64)
        for (i in base.indices) {
            for (bit in 0 until 8) {
                val other = base.copyOf()
                other[i] = (other[i].toInt() xor (1 shl bit)).toByte()
                assertEquals(0, CtCompare.ctEqBit(base, other), "byte $i bit $bit")
            }
        }
    }

    @Test
    fun ctEqU64Bit() {
        val values = longArrayOf(0L, 1L, 2L, Long.MIN_VALUE, Long.MAX_VALUE, -1L, 0x0123456789abcdefL)
        for (x in values) {
            for (y in values) assertEquals(if (x == y) 1 else 0, CtCompare.ctEqU64Bit(x, y))
            for (bit in 0 until 64) assertEquals(0, CtCompare.ctEqU64Bit(x, x xor (1L shl bit)))
        }
    }

    @Test
    fun equalityBitFeedsSelection() {
        val a = "left".toByteArray()
        val b = "rite".toByteArray()
        assertContentEquals(a, CtCompare.ctSelectBytes(a, b, CtCompare.ctEqBit("k".toByteArray(), "k".toByteArray())))
        assertContentEquals(b, CtCompare.ctSelectBytes(a, b, CtCompare.ctEqBit("k".toByteArray(), "j".toByteArray())))
        assertContentEquals(a, CtCompare.ctSelectBytes(a, b, CtCompare.ctEqU64Bit(7L, 7L)))
    }
}
