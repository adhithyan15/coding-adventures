package com.codingadventures.polynomial

import com.codingadventures.gf256.GF256
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals

/**
 * Tests for GF(256) polynomial arithmetic.
 *
 * All arithmetic is over GF(2^8) with primitive polynomial 0x11D.
 * Test vectors are cross-checked against the TypeScript reference
 * implementation and the MA00/MA01 specifications.
 */
class PolynomialTest {

    // =========================================================================
    // normalize
    // =========================================================================

    @Test
    fun `normalize removes trailing zeros`() {
        assertContentEquals(intArrayOf(1), normalize(intArrayOf(1, 0, 0)))
    }

    @Test
    fun `normalize of all-zeros is empty`() {
        assertContentEquals(intArrayOf(), normalize(intArrayOf(0)))
        assertContentEquals(intArrayOf(), normalize(intArrayOf(0, 0, 0)))
    }

    @Test
    fun `normalize preserves already-normalised polynomial`() {
        assertContentEquals(intArrayOf(1, 2, 3), normalize(intArrayOf(1, 2, 3)))
    }

    @Test
    fun `normalize of empty is empty`() {
        assertContentEquals(intArrayOf(), normalize(intArrayOf()))
    }

    // =========================================================================
    // degree
    // =========================================================================

    @Test
    fun `degree of zero polynomial is -1`() {
        assertEquals(-1, degree(intArrayOf()))
        assertEquals(-1, degree(intArrayOf(0)))
    }

    @Test
    fun `degree of constant polynomial is 0`() {
        assertEquals(0, degree(intArrayOf(7)))
    }

    @Test
    fun `degree of x-squared term is 2`() {
        assertEquals(2, degree(intArrayOf(3, 0, 2)))
    }

    // =========================================================================
    // add / sub
    // =========================================================================

    @Test
    fun `add is XOR of each coefficient`() {
        // [1, 2, 3] + [4, 5] = [1^4, 2^5, 3] = [5, 7, 3]
        assertContentEquals(
            intArrayOf(5, 7, 3),
            add(intArrayOf(1, 2, 3), intArrayOf(4, 5))
        )
    }

    @Test
    fun `add with zero polynomial is identity`() {
        val p = intArrayOf(1, 2, 3)
        assertContentEquals(p, add(p, intArrayOf()))
        assertContentEquals(p, add(intArrayOf(), p))
    }

    @Test
    fun `add a polynomial to itself gives zero`() {
        // In GF(256), p + p = 0 for any polynomial p.
        val p = intArrayOf(1, 2, 3, 255)
        assertContentEquals(intArrayOf(), add(p, p))
    }

    @Test
    fun `sub equals add in GF(256)`() {
        val a = intArrayOf(3, 7, 0x53)
        val b = intArrayOf(0xCA, 1)
        assertContentEquals(add(a, b), sub(a, b))
    }

    @Test
    fun `add is commutative`() {
        val a = intArrayOf(1, 2, 3)
        val b = intArrayOf(4, 5, 6)
        assertContentEquals(add(a, b), add(b, a))
    }

    // =========================================================================
    // mul
    // =========================================================================

    @Test
    fun `mul by zero polynomial gives zero`() {
        val p = intArrayOf(1, 2, 3)
        assertContentEquals(intArrayOf(), mul(p, intArrayOf()))
        assertContentEquals(intArrayOf(), mul(intArrayOf(), p))
    }

    @Test
    fun `mul by one polynomial is identity`() {
        val p = intArrayOf(3, 7, 0x53)
        assertContentEquals(p, mul(p, intArrayOf(1)))
        assertContentEquals(p, mul(intArrayOf(1), p))
    }

    @Test
    fun `mul by scalar scales every coefficient`() {
        // [1, 2, 3] × [c] = [mul(1,c), mul(2,c), mul(3,c)]
        val c = 3
        val p = intArrayOf(1, 2, 3)
        val expected = IntArray(p.size) { GF256.mul(p[it], c) }
        assertContentEquals(expected, mul(p, intArrayOf(c)))
    }

    @Test
    fun `mul degree is sum of degrees`() {
        val a = intArrayOf(1, 0, 1)  // degree 2
        val b = intArrayOf(1, 1)     // degree 1
        val product = mul(a, b)
        assertEquals(degree(a) + degree(b), degree(product))
    }

    @Test
    fun `mul is commutative`() {
        val a = intArrayOf(1, 2, 3)
        val b = intArrayOf(4, 5)
        assertContentEquals(mul(a, b), mul(b, a))
    }

    @Test
    fun `mul is distributive over add`() {
        // a × (b + c) = a×b + a×c
        val a = intArrayOf(1, 2)
        val b = intArrayOf(3, 4, 5)
        val c = intArrayOf(6, 7)
        assertContentEquals(
            mul(a, add(b, c)),
            add(mul(a, b), mul(a, c))
        )
    }

    @Test
    fun `mul known RS generator - 2 check bytes`() {
        // buildGenerator(2) in RS gives [8, 6, 1] (little-endian)
        // Manually: start with [1], multiply by [2, 1] (x + alpha^1 = x + 2):
        //   [1] × [2, 1] = [2, 1]
        // then multiply by [4, 1] (x + alpha^2 = x + 4):
        //   [2, 1] × [4, 1] = [mul(2,4)^0, mul(2,1)^mul(1,4), 1]
        //                   = [8, 2^4, 1] = [8, 6, 1]
        val factor1 = intArrayOf(2, 1)   // (x + alpha^1)
        val factor2 = intArrayOf(4, 1)   // (x + alpha^2)
        val gen = mul(factor1, factor2)
        assertContentEquals(intArrayOf(8, 6, 1), gen)
    }

    // =========================================================================
    // divmod
    // =========================================================================

    @Test
    fun `divmod by zero throws`() {
        assertThrows<ArithmeticException> {
            divmod(intArrayOf(1, 2, 3), intArrayOf())
        }
    }

    @Test
    fun `divmod when dividend degree less than divisor`() {
        // x / (x^2 + 1) = quotient 0, remainder x
        val (q, r) = divmod(intArrayOf(0, 1), intArrayOf(1, 0, 1))
        assertContentEquals(intArrayOf(), q)
        assertContentEquals(intArrayOf(0, 1), r)
    }

    @Test
    fun `divmod basic case - exact division`() {
        // [2, 1] × [4, 1] = [8, 6, 1], so [8, 6, 1] / [2, 1] = [4, 1], rem 0
        val product = mul(intArrayOf(2, 1), intArrayOf(4, 1))
        val (q, r) = divmod(product, intArrayOf(2, 1))
        assertContentEquals(intArrayOf(4, 1), q)
        assertContentEquals(intArrayOf(), r)
    }

    @Test
    fun `divmod remainder has lower degree than divisor`() {
        val a = intArrayOf(1, 2, 3, 4)   // degree 3
        val b = intArrayOf(1, 1, 1)      // degree 2
        val (q, r) = divmod(a, b)
        assert(degree(r) < degree(b)) {
            "Remainder degree ${degree(r)} should be < divisor degree ${degree(b)}"
        }
    }

    @Test
    fun `divmod satisfies a equals b times q plus r`() {
        // The fundamental division identity: a = b*q + r
        val a = intArrayOf(3, 1, 4, 1, 5)
        val b = intArrayOf(1, 2, 1)
        val (q, r) = divmod(a, b)
        val reconstructed = add(mul(b, q), r)
        assertContentEquals(normalize(a), reconstructed)
    }

    @Test
    fun `divmod single byte divisor`() {
        // [6, 4, 2] / [2] = [div(6,2), div(4,2), div(2,2)] = [3, 2, 1]
        val (q, r) = divmod(intArrayOf(6, 4, 2), intArrayOf(2))
        assertContentEquals(intArrayOf(), r)
        // Verify b * q = a
        assertContentEquals(normalize(intArrayOf(6, 4, 2)), mul(intArrayOf(2), q))
    }

    // =========================================================================
    // eval
    // =========================================================================

    @Test
    fun `eval of zero polynomial at any point is 0`() {
        for (x in 0..255) {
            assertEquals(0, eval(intArrayOf(), x))
        }
    }

    @Test
    fun `eval of constant polynomial is that constant`() {
        assertEquals(7, eval(intArrayOf(7), 0))
        assertEquals(7, eval(intArrayOf(7), 5))
        assertEquals(7, eval(intArrayOf(7), 255))
    }

    @Test
    fun `eval of polynomial at 0 is constant term`() {
        // p(0) = p[0] (only the constant term survives when x = 0)
        val p = intArrayOf(42, 7, 3, 1)
        assertEquals(42, eval(p, 0))
    }

    @Test
    fun `eval linear polynomial`() {
        // p(x) = 3 + 5x  →  p(2) = 3 XOR mul(5, 2)
        val p = intArrayOf(3, 5)
        val expected = GF256.add(3, GF256.mul(5, 2))
        assertEquals(expected, eval(p, 2))
    }

    @Test
    fun `eval agrees with direct computation`() {
        // p(x) = 1 + 2x + 3x^2; evaluate at x = 7
        // = 1 XOR mul(2,7) XOR mul(3, mul(7,7))
        val p = intArrayOf(1, 2, 3)
        val x = 7
        val manual = GF256.add(
            GF256.add(1, GF256.mul(2, x)),
            GF256.mul(3, GF256.mul(x, x))
        )
        assertEquals(manual, eval(p, x))
    }

    @Test
    fun `eval generator roots are zero - RS property`() {
        // The RS generator polynomial g = [8, 6, 1] has roots alpha^1=2, alpha^2=4.
        val gen = intArrayOf(8, 6, 1)
        assertEquals(0, eval(gen, 2),  "g(alpha^1) should be 0")
        assertEquals(0, eval(gen, 4),  "g(alpha^2) should be 0")
    }

    // =========================================================================
    // gcd
    // =========================================================================

    @Test
    fun `gcd of polynomial with itself is normalised self`() {
        val p = intArrayOf(1, 2, 3)
        val g = gcd(p, p)
        // GCD is defined up to a scalar; the leading coefficient may differ.
        // The key property: g divides p with zero remainder.
        val (_, r) = divmod(p, g)
        assertContentEquals(intArrayOf(), r)
    }

    @Test
    fun `gcd of polynomial with zero is that polynomial`() {
        val p = intArrayOf(1, 2, 3)
        val g = gcd(p, intArrayOf())
        assertContentEquals(normalize(p), g)
    }

    @Test
    fun `gcd of coprime polynomials is constant`() {
        // p = (x + 2)(x + 4) = [8, 6, 1]; q = (x + 3) = [3, 1]
        // These share no common root, so GCD should be degree 0.
        val p = mul(intArrayOf(2, 1), intArrayOf(4, 1))  // [8, 6, 1]
        val q = intArrayOf(3, 1)
        val g = gcd(p, q)
        assertEquals(0, degree(g))
    }

    @Test
    fun `gcd divides both inputs`() {
        val a = mul(intArrayOf(2, 1), intArrayOf(4, 1))  // (x+2)(x+4)
        val b = mul(intArrayOf(2, 1), intArrayOf(3, 1))  // (x+2)(x+3)
        val g = gcd(a, b)
        // g must divide both a and b
        val (_, ra) = divmod(a, g)
        val (_, rb) = divmod(b, g)
        assertContentEquals(intArrayOf(), ra)
        assertContentEquals(intArrayOf(), rb)
    }

    // =========================================================================
    // poly helper
    // =========================================================================

    @Test
    fun `poly vararg constructor normalises`() {
        assertContentEquals(intArrayOf(1), poly(1, 0, 0))
        assertContentEquals(intArrayOf(), poly(0))
    }

    @Test
    fun `poly with multiple non-zero coefficients`() {
        assertContentEquals(intArrayOf(3, 0, 2), poly(3, 0, 2))
    }

    // =========================================================================
    // zero and one helpers
    // =========================================================================

    @Test
    fun `zero polynomial is empty`() {
        assertContentEquals(intArrayOf(), zero())
    }

    @Test
    fun `one polynomial is the constant 1`() {
        assertContentEquals(intArrayOf(1), one())
    }

    @Test
    fun `add with zero is identity via zero helper`() {
        val p = poly(3, 7, 0x53)
        assertContentEquals(p, add(p, zero()))
    }

    @Test
    fun `mul with one is identity via one helper`() {
        val p = poly(3, 7, 0x53)
        assertContentEquals(p, mul(p, one()))
    }
}
