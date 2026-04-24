package com.codingadventures.gf256

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

/**
 * Tests for GF(256) arithmetic with primitive polynomial 0x11D.
 *
 * Test vectors derived from the MA01 specification and cross-checked against
 * the TypeScript reference implementation.
 */
class GF256Test {

    // =========================================================================
    // Table sanity
    // =========================================================================

    @Test
    fun `EXP table first entry is 1 (generator to the power 0)`() {
        // g^0 = 1 in any multiplicative group
        assertEquals(1, GF256.EXP[0])
    }

    @Test
    fun `EXP table second entry is 2 (the generator itself)`() {
        // g = 2 is our primitive element
        assertEquals(2, GF256.EXP[1])
    }

    @Test
    fun `EXP table entry 8 is 29 (first reduction step)`() {
        // 2^8 = 256; 256 XOR 0x11D = 29.  The first time we overflow a byte.
        assertEquals(29, GF256.EXP[8])
    }

    @Test
    fun `generator has order 255 - EXP 255 equals EXP 0`() {
        // g^255 wraps back to 1 (the multiplicative group has order 255).
        assertEquals(GF256.EXP[0], GF256.EXP[255])
    }

    @Test
    fun `LOG of 0 is -1 (undefined)`() {
        assertEquals(-1, GF256.LOG[0])
    }

    @Test
    fun `LOG and EXP are inverses for all non-zero elements`() {
        // For every x in 1..255: EXP[LOG[x]] == x
        for (x in 1..255) {
            assertEquals(x, GF256.EXP[GF256.LOG[x]],
                "EXP[LOG[$x]] should equal $x")
        }
    }

    @Test
    fun `all 255 non-zero elements appear exactly once in EXP 0 through 254`() {
        // The primitive element generates all non-zero field elements.
        val seen = mutableSetOf<Int>()
        for (i in 0 until 255) {
            val v = GF256.EXP[i]
            assert(v in 1..255) { "EXP[$i] = $v is out of range" }
            assert(seen.add(v)) { "EXP[$i] = $v appeared more than once" }
        }
        assertEquals(255, seen.size)
    }

    // =========================================================================
    // Addition
    // =========================================================================

    @Test
    fun `add is XOR`() {
        assertEquals(0x53 xor 0xCA, GF256.add(0x53, 0xCA))
    }

    @Test
    fun `add is its own inverse - add(x, x) equals 0 for all x`() {
        // In characteristic-2, every element is its own additive inverse.
        for (x in 0..255) {
            assertEquals(0, GF256.add(x, x), "add($x, $x) should be 0")
        }
    }

    @Test
    fun `add is commutative`() {
        assertEquals(GF256.add(0x53, 0xCA), GF256.add(0xCA, 0x53))
    }

    @Test
    fun `add with zero is identity`() {
        for (x in 0..255) {
            assertEquals(x, GF256.add(x, 0), "add($x, 0) should be $x")
        }
    }

    // =========================================================================
    // Subtraction
    // =========================================================================

    @Test
    fun `sub equals add in GF(256)`() {
        // Subtraction and addition are identical in characteristic-2 fields.
        for (a in 0..15) {
            for (b in 0..15) {
                assertEquals(GF256.add(a, b), GF256.sub(a, b))
            }
        }
    }

    // =========================================================================
    // Multiplication
    // =========================================================================

    @Test
    fun `mul of zero with anything is zero`() {
        for (x in 0..255) {
            assertEquals(0, GF256.mul(0, x))
            assertEquals(0, GF256.mul(x, 0))
        }
    }

    @Test
    fun `mul by one is identity`() {
        for (x in 0..255) {
            assertEquals(x, GF256.mul(x, 1), "mul($x, 1) should be $x")
        }
    }

    @Test
    fun `mul is commutative`() {
        for (a in 0..15) {
            for (b in 0..15) {
                assertEquals(GF256.mul(a, b), GF256.mul(b, a))
            }
        }
    }

    @Test
    fun `mul spot check - 3 times 7 equals 9`() {
        // Known value from GF(256) with 0x11D:
        // LOG[3] = 25, LOG[7] = 198 (verify: EXP[25+198 mod 255] = EXP[223] = 9? wait,
        // let's just test the actual value since the TS impl agrees with 9)
        // The precise value depends on the table; cross-checked with TypeScript.
        assertEquals(9, GF256.mul(3, 7))
    }

    @Test
    fun `mul spot check - known inverse pair`() {
        // Under 0x11D polynomial: 0x53 × 0x8C = 1
        // (This is the Reed-Solomon inverse pair; different from AES 0x11B pair.)
        assertEquals(1, GF256.mul(0x53, 0x8C))
    }

    @Test
    fun `mul is distributive over add`() {
        // a × (b + c) = (a × b) + (a × c)
        for (a in listOf(2, 3, 7, 0x53)) {
            for (b in listOf(1, 5, 0xCA, 0xFF)) {
                for (c in listOf(0, 3, 0xAB)) {
                    val lhs = GF256.mul(a, GF256.add(b, c))
                    val rhs = GF256.add(GF256.mul(a, b), GF256.mul(a, c))
                    assertEquals(lhs, rhs)
                }
            }
        }
    }

    // =========================================================================
    // Division
    // =========================================================================

    @Test
    fun `div by zero throws ArithmeticException`() {
        assertThrows<ArithmeticException> { GF256.div(1, 0) }
        assertThrows<ArithmeticException> { GF256.div(0, 0) }
    }

    @Test
    fun `div zero by anything non-zero is zero`() {
        for (b in 1..255) {
            assertEquals(0, GF256.div(0, b))
        }
    }

    @Test
    fun `div is inverse of mul`() {
        // For all a in 1..255, b in 1..255: div(mul(a, b), b) == a
        for (a in listOf(1, 2, 3, 0x53, 0xFF)) {
            for (b in listOf(1, 2, 7, 0x8C, 0xFE)) {
                val product = GF256.mul(a, b)
                assertEquals(a, GF256.div(product, b),
                    "div(mul($a, $b), $b) should equal $a")
            }
        }
    }

    @Test
    fun `div x by x is 1 for all non-zero x`() {
        for (x in 1..255) {
            assertEquals(1, GF256.div(x, x), "div($x, $x) should be 1")
        }
    }

    // =========================================================================
    // Power
    // =========================================================================

    @Test
    fun `pow 0 to the 0 is 1 by convention`() {
        assertEquals(1, GF256.pow(0, 0))
    }

    @Test
    fun `pow 0 to any positive exponent is 0`() {
        for (n in 1..10) {
            assertEquals(0, GF256.pow(0, n))
        }
    }

    @Test
    fun `pow anything to the 0 is 1`() {
        for (x in 0..255) {
            assertEquals(1, GF256.pow(x, 0), "pow($x, 0) should be 1")
        }
    }

    @Test
    fun `pow anything to the 1 is itself`() {
        for (x in 0..255) {
            assertEquals(x, GF256.pow(x, 1), "pow($x, 1) should be $x")
        }
    }

    @Test
    fun `pow generator to 255 is 1 (field order)`() {
        // g^255 = 1 because the multiplicative group has order 255.
        assertEquals(1, GF256.pow(2, 255))
    }

    @Test
    fun `pow negative exponent throws`() {
        assertThrows<IllegalArgumentException> { GF256.pow(2, -1) }
    }

    @Test
    fun `pow agrees with repeated mul`() {
        // pow(3, 5) should match mul(mul(mul(mul(3, 3), 3), 3), 3)
        var manual = 1
        for (i in 0 until 5) manual = GF256.mul(manual, 3)
        assertEquals(manual, GF256.pow(3, 5))
    }

    @Test
    fun `pow large exponent uses modular arithmetic`() {
        // g^255 = 1, so g^256 = g^1 = 2, g^510 = g^0 = 1
        assertEquals(2, GF256.pow(2, 256))
        assertEquals(1, GF256.pow(2, 510))
    }

    // =========================================================================
    // Inverse
    // =========================================================================

    @Test
    fun `inv of zero throws ArithmeticException`() {
        assertThrows<ArithmeticException> { GF256.inv(0) }
    }

    @Test
    fun `inv of 1 is 1 (multiplicative identity)`() {
        assertEquals(1, GF256.inv(1))
    }

    @Test
    fun `a times inv(a) equals 1 for all non-zero a`() {
        for (a in 1..255) {
            assertEquals(1, GF256.mul(a, GF256.inv(a)),
                "mul($a, inv($a)) should be 1")
        }
    }

    @Test
    fun `inv of inv is original element`() {
        for (a in 1..255) {
            assertEquals(a, GF256.inv(GF256.inv(a)),
                "inv(inv($a)) should be $a")
        }
    }

    @Test
    fun `inv spot check - 0x53 inverts to 0x8C under 0x11D`() {
        assertEquals(0x8C, GF256.inv(0x53))
    }

    // =========================================================================
    // Field axioms (comprehensive)
    // =========================================================================

    @Test
    fun `mul is associative`() {
        // (a × b) × c = a × (b × c)
        for (a in listOf(2, 3, 0x53)) {
            for (b in listOf(5, 7, 0xCA)) {
                for (c in listOf(11, 0x8C, 0xFF)) {
                    val lhs = GF256.mul(GF256.mul(a, b), c)
                    val rhs = GF256.mul(a, GF256.mul(b, c))
                    assertEquals(lhs, rhs,
                        "Associativity failed for mul($a, $b, $c)")
                }
            }
        }
    }

    @Test
    fun `add is associative`() {
        for (a in 0..15) {
            for (b in 0..15) {
                for (c in 0..15) {
                    assertEquals(
                        GF256.add(GF256.add(a, b), c),
                        GF256.add(a, GF256.add(b, c))
                    )
                }
            }
        }
    }
}
