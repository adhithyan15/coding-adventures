package com.codingadventures.logicgates

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/** LGE-KT-NARY: shape, snapshot, cap and naming behaviour of AND_N/OR_N/XOR_N. */
class NaryGatesTest {
    private val g = LogicGates

    private val nary: Map<String, Pair<(IntArray) -> Int, Int>> = mapOf(
        "AND_N" to ({ a: IntArray -> g.AND_N(*a) } to 2),
        "OR_N" to ({ a: IntArray -> g.OR_N(*a) } to 2),
        "XOR_N" to ({ a: IntArray -> g.XOR_N(*a) } to 0),
    )

    private fun expected(name: String, v: IntArray): Int = when (name) {
        "AND_N" -> if (v.all { it == 1 }) 1 else 0
        "OR_N" -> if (v.any { it == 1 }) 1 else 0
        else -> v.sum() % 2
    }

    @Test
    fun nullFromJavaCallerIsInvalidShape() {
        for (name in listOf("AND_N", "OR_N", "XOR_N")) {
            val m = LogicGates::class.java.getMethod(name, IntArray::class.java)
            val ex = assertThrows<java.lang.reflect.InvocationTargetException> {
                m.invoke(LogicGates, null as Any?)
            }
            val cause = ex.cause
            assertTrue(cause is InvalidShapeException, "$name: $cause")
            assertEquals("inputs", (cause as InvalidShapeException).parameter)
        }
    }

    @Test
    fun lengthRules() {
        assertThrows<InvalidShapeException> { g.AND_N(1) }
        assertThrows<InvalidShapeException> { g.AND_N() }
        assertThrows<InvalidShapeException> { g.OR_N(0) }
        assertThrows<InvalidShapeException> { g.OR_N() }
        assertEquals(0, g.XOR_N())
        assertEquals(1, g.XOR_N(1))
    }

    @Test
    fun maxInputsBoundary() {
        assertEquals(65536, LogicGates.MAX_INPUTS)
        val ok = IntArray(LogicGates.MAX_INPUTS) { 1 }
        assertEquals(1, g.AND_N(*ok))
        assertEquals(1, g.OR_N(*ok))
        assertEquals(0, g.XOR_N(*ok))
        val big = IntArray(LogicGates.MAX_INPUTS + 1) { 1 }
        assertThrows<InvalidShapeException> { g.AND_N(*big) }
        assertThrows<InvalidShapeException> { g.OR_N(*big) }
        assertThrows<InvalidShapeException> { g.XOR_N(*big) }
    }

    @Test
    fun namesFirstBadElement() {
        for ((name, pair) in nary) {
            val ex = assertThrows<InvalidBitException> { pair.first(intArrayOf(1, 0, 7, 9)) }
            assertEquals("inputs[2]", ex.parameter, name)
            assertEquals(7, ex.value, name)
        }
    }

    @Test
    fun shapeWinsOverBit() {
        val tooBig = IntArray(LogicGates.MAX_INPUTS + 1) { 1 }.also { it[0] = 5 }
        for ((name, pair) in nary) {
            assertThrows<InvalidShapeException>(name) { pair.first(tooBig) }
        }
        assertThrows<InvalidShapeException> { g.AND_N(5) }
        assertThrows<InvalidShapeException> { g.OR_N(5) }
    }

    @Test
    fun exhaustiveOverSmallDomains() {
        val domain = intArrayOf(-1, 0, 1, 2)
        for ((name, pair) in nary) {
            val (fn, min) = pair
            for (len in 0..8) {
                var combos = 1
                repeat(len) { combos *= domain.size }
                for (code in 0 until combos) {
                    val arr = IntArray(len)
                    var c = code
                    for (i in 0 until len) { arr[i] = domain[c % domain.size]; c /= domain.size }
                    val before = arr.copyOf()
                    val firstBad = arr.indexOfFirst { it != 0 && it != 1 }
                    when {
                        len < min -> assertThrows<InvalidShapeException> { fn(arr) }
                        firstBad >= 0 -> {
                            val ex = assertThrows<InvalidBitException> { fn(arr) }
                            assertEquals("inputs[$firstBad]", ex.parameter)
                        }
                        else -> assertEquals(expected(name, arr), fn(arr))
                    }
                    assertTrue(before.contentEquals(arr), "$name mutated caller array")
                }
            }
        }
    }
}
