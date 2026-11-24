package com.codingadventures.logicgates

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/**
 * Exhaustive validation of the 11 basic gates.
 *
 * Type confusion (Long, Char, Boolean, Double arguments) is a COMPILE-time
 * error in Kotlin because there is no implicit widening and the API has no
 * Number/Any overloads; only out-of-range Ints can reach runtime.
 */
class LogicGatesExhaustiveTest {
    private val g = LogicGates

    private val unary: Map<String, Pair<(Int) -> Int, List<Int>>> = mapOf(
        "NOT" to (g::NOT to listOf(1, 0)),
        "nandNOT" to (g::nandNOT to listOf(1, 0)),
    )

    // Truth tables listed in order (0,0) (0,1) (1,0) (1,1).
    private val binary: Map<String, Pair<(Int, Int) -> Int, List<Int>>> = mapOf(
        "AND" to (g::AND to listOf(0, 0, 0, 1)),
        "OR" to (g::OR to listOf(0, 1, 1, 1)),
        "XOR" to (g::XOR to listOf(0, 1, 1, 0)),
        "NAND" to (g::NAND to listOf(1, 1, 1, 0)),
        "NOR" to (g::NOR to listOf(1, 0, 0, 0)),
        "XNOR" to (g::XNOR to listOf(1, 0, 0, 1)),
        "nandAND" to (g::nandAND to listOf(0, 0, 0, 1)),
        "nandOR" to (g::nandOR to listOf(0, 1, 1, 1)),
        "nandXOR" to (g::nandXOR to listOf(0, 1, 1, 0)),
    )

    private val invalid: List<Int> =
        (-1024..1024).filter { it != 0 && it != 1 } +
            listOf(Int.MIN_VALUE, Int.MIN_VALUE + 1, Int.MAX_VALUE - 1, Int.MAX_VALUE)

    @Test fun unaryTruthTables() {
        for ((name, p) in unary) {
            assertEquals(p.second[0], p.first(0), name)
            assertEquals(p.second[1], p.first(1), name)
        }
    }

    @Test fun binaryTruthTables() {
        for ((name, p) in binary) {
            var i = 0
            for (a in 0..1) for (b in 0..1) {
                val r = p.first(a, b)
                assertEquals(p.second[i++], r, "$name($a,$b)")
                assertTrue(r == 0 || r == 1)
            }
        }
    }

    @Test fun nandBuiltMatchesDirect() {
        for (a in 0..1) {
            assertEquals(g.NOT(a), g.nandNOT(a))
            for (b in 0..1) {
                assertEquals(g.AND(a, b), g.nandAND(a, b))
                assertEquals(g.OR(a, b), g.nandOR(a, b))
                assertEquals(g.XOR(a, b), g.nandXOR(a, b))
            }
        }
    }

    private fun check(e: InvalidBitException, param: String, value: Int) {
        assertEquals(param, e.parameter)
        assertEquals(value, e.value)
    }

    @Test fun unaryRejectsEveryInvalid() {
        for ((name, p) in unary) for (v in invalid) {
            val e = assertThrows<InvalidBitException>("$name($v)") { p.first(v) }
            check(e, "a", v)
        }
    }

    @Test fun binaryRejectsEveryInvalidInEveryPosition() {
        for ((name, p) in binary) for (v in invalid) {
            for (other in listOf(0, 1)) {
                check(assertThrows<InvalidBitException>("$name($v,$other)") { p.first(v, other) }, "a", v)
                check(assertThrows<InvalidBitException>("$name($other,$v)") { p.first(other, v) }, "b", v)
            }
            // Both invalid: the FIRST parameter is reported.
            check(assertThrows<InvalidBitException>("$name($v,$v)") { p.first(v, v) }, "a", v)
        }
    }

    @Test fun exceptionIsIllegalArgumentException() {
        assertThrows<IllegalArgumentException> { g.NAND(2, 0) }
    }

    @Test fun multiInputGatesStillValidate() {
        assertThrows<InvalidBitException> { g.AND_N(1, 2, 1) }
        assertThrows<InvalidBitException> { g.OR_N(0, 0, 5) }
        assertThrows<InvalidBitException> { g.XOR_N(0, 7) }
        assertThrows<IllegalArgumentException> { g.AND_N(1) }
        assertEquals(0, g.XOR_N())
    }
}
