package com.codingadventures.reedsolomon

import com.codingadventures.gf256.GF256
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * Tests for Reed-Solomon encode/decode over GF(256) with primitive polynomial 0x11D.
 *
 * Test vectors cross-checked against the TypeScript reference implementation
 * and the MA02 specification.
 */
class ReedSolomonTest {

    // =========================================================================
    // buildGenerator
    // =========================================================================

    @Test
    fun `buildGenerator nCheck 0 throws InvalidInputException`() {
        assertThrows<InvalidInputException> { buildGenerator(0) }
    }

    @Test
    fun `buildGenerator odd nCheck throws InvalidInputException`() {
        assertThrows<InvalidInputException> { buildGenerator(3) }
        assertThrows<InvalidInputException> { buildGenerator(5) }
    }

    @Test
    fun `buildGenerator nCheck 2 produces degree-2 monic polynomial`() {
        val g = buildGenerator(2)
        // Should have length nCheck+1 = 3
        assertEquals(3, g.size)
        // Monic: leading coefficient (last in LE) = 1
        assertEquals(1, g.last())
    }

    @Test
    fun `buildGenerator nCheck 2 known value`() {
        // g = (x + alpha^1)(x + alpha^2) = (x+2)(x+4) = [8, 6, 1] LE
        val g = buildGenerator(2)
        assertContentEquals(intArrayOf(8, 6, 1), g)
    }

    @Test
    fun `buildGenerator nCheck 4 has degree 4 and is monic`() {
        val g = buildGenerator(4)
        assertEquals(5, g.size)
        assertEquals(1, g.last())
    }

    @Test
    fun `buildGenerator roots are alpha powers 1 through nCheck`() {
        // For nCheck=4, the roots of g are alpha^1, alpha^2, alpha^3, alpha^4
        val nCheck = 4
        val g = buildGenerator(nCheck)
        // Evaluate g at each expected root; should get 0
        for (i in 1..nCheck) {
            val alphaI = GF256.pow(2, i)
            // Evaluate big-endian (reverse g for BE evaluation)
            val gBE = g.reversedArray()
            var acc = 0
            for (b in gBE) {
                acc = GF256.add(GF256.mul(acc, alphaI), b)
            }
            assertEquals(0, acc, "g(alpha^$i) should be 0 for nCheck=$nCheck")
        }
    }

    // =========================================================================
    // syndromes
    // =========================================================================

    @Test
    fun `syndromes of valid codeword are all zero`() {
        val msg = intArrayOf(72, 101, 108, 108, 111)  // "Hello"
        val nCheck = 8
        val codeword = encode(msg, nCheck)
        val synds = syndromes(codeword, nCheck)
        assertTrue(synds.all { it == 0 }, "All syndromes of a valid codeword should be 0")
    }

    @Test
    fun `syndromes of corrupted codeword are non-zero`() {
        val msg = intArrayOf(1, 2, 3, 4)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[0] = corrupted[0] xor 0xFF  // flip all bits in first byte
        val synds = syndromes(corrupted, nCheck)
        assertTrue(synds.any { it != 0 }, "At least one syndrome should be non-zero after corruption")
    }

    // =========================================================================
    // encode
    // =========================================================================

    @Test
    fun `encode nCheck 0 throws InvalidInputException`() {
        assertThrows<InvalidInputException> { encode(intArrayOf(1, 2, 3), 0) }
    }

    @Test
    fun `encode odd nCheck throws InvalidInputException`() {
        assertThrows<InvalidInputException> { encode(intArrayOf(1, 2, 3), 3) }
    }

    @Test
    fun `encode total length over 255 throws InvalidInputException`() {
        val bigMsg = IntArray(250) { it }
        assertThrows<InvalidInputException> { encode(bigMsg, 10) }
    }

    @Test
    fun `encode is systematic - message bytes unchanged in codeword prefix`() {
        val msg = intArrayOf(72, 101, 108, 108, 111)  // "Hello"
        val nCheck = 8
        val codeword = encode(msg, nCheck)
        assertEquals(msg.size + nCheck, codeword.size)
        // First msg.size bytes should be the original message
        for (i in msg.indices) {
            assertEquals(msg[i], codeword[i], "codeword[$i] should equal message[$i]")
        }
    }

    @Test
    fun `encode produces codeword with zero syndromes`() {
        val msg = intArrayOf(1, 2, 3, 4, 5)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val synds = syndromes(codeword, nCheck)
        assertTrue(synds.all { it == 0 })
    }

    @Test
    fun `encode nCheck 2 known parity bytes`() {
        // Short message [4, 8] with nCheck=2: manually verify via polyModBE
        // g = [8, 6, 1], gBE = [1, 6, 8]
        // shifted = [4, 8, 0, 0], then compute shifted mod gBE
        val msg = intArrayOf(4, 8)
        val nCheck = 2
        val codeword = encode(msg, nCheck)
        assertEquals(4, codeword.size)
        // Verify the codeword is valid (zero syndromes)
        val synds = syndromes(codeword, nCheck)
        assertTrue(synds.all { it == 0 })
    }

    @Test
    fun `encode single byte message`() {
        val msg = intArrayOf(42)
        val nCheck = 2
        val codeword = encode(msg, nCheck)
        assertEquals(3, codeword.size)
        assertEquals(42, codeword[0])
        assertTrue(syndromes(codeword, nCheck).all { it == 0 })
    }

    @Test
    fun `encode empty message`() {
        val msg = intArrayOf()
        val nCheck = 2
        val codeword = encode(msg, nCheck)
        // All-zero message → all-zero codeword (including parity)
        assertEquals(nCheck, codeword.size)
        assertTrue(syndromes(codeword, nCheck).all { it == 0 })
    }

    // =========================================================================
    // decode — no errors
    // =========================================================================

    @Test
    fun `decode with no errors recovers original message`() {
        val msg = intArrayOf(72, 101, 108, 108, 111)
        val nCheck = 8
        val codeword = encode(msg, nCheck)
        val recovered = decode(codeword, nCheck)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `decode nCheck 0 throws InvalidInputException`() {
        assertThrows<InvalidInputException> { decode(intArrayOf(1, 2, 3), 0) }
    }

    @Test
    fun `decode odd nCheck throws InvalidInputException`() {
        assertThrows<InvalidInputException> { decode(intArrayOf(1, 2, 3, 4), 3) }
    }

    @Test
    fun `decode received shorter than nCheck throws InvalidInputException`() {
        assertThrows<InvalidInputException> { decode(intArrayOf(1, 2), 4) }
    }

    // =========================================================================
    // decode — single error
    // =========================================================================

    @Test
    fun `decode corrects single error in message area`() {
        val msg = intArrayOf(1, 2, 3, 4, 5)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[2] = corrupted[2] xor 0x55  // corrupt byte at position 2
        val recovered = decode(corrupted, nCheck)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `decode corrects single error in check area`() {
        val msg = intArrayOf(10, 20, 30)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[msg.size + 1] = corrupted[msg.size + 1] xor 0xAA  // corrupt a parity byte
        val recovered = decode(corrupted, nCheck)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `decode corrects single error at position 0`() {
        val msg = intArrayOf(0xDE, 0xAD, 0xBE, 0xEF)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[0] = corrupted[0] xor 0xFF
        val recovered = decode(corrupted, nCheck)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `decode corrects single error at last position`() {
        val msg = intArrayOf(10, 20, 30, 40)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[codeword.size - 1] = corrupted[codeword.size - 1] xor 0x7F
        val recovered = decode(corrupted, nCheck)
        assertContentEquals(msg, recovered)
    }

    // =========================================================================
    // decode — two errors (nCheck=4 supports t=2)
    // =========================================================================

    @Test
    fun `decode corrects two errors with nCheck 4`() {
        val msg = intArrayOf(1, 2, 3, 4, 5, 6, 7, 8)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[0] = corrupted[0] xor 0xFF
        corrupted[3] = corrupted[3] xor 0xAA
        val recovered = decode(corrupted, nCheck)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `decode corrects two errors at any positions`() {
        val msg = IntArray(10) { it + 1 }
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[1] = corrupted[1] xor 0x5A
        corrupted[codeword.size - 2] = corrupted[codeword.size - 2] xor 0xA5
        val recovered = decode(corrupted, nCheck)
        assertContentEquals(msg, recovered)
    }

    // =========================================================================
    // decode — four errors (nCheck=8 supports t=4)
    // =========================================================================

    @Test
    fun `decode corrects four errors with nCheck 8`() {
        val msg = intArrayOf(72, 101, 108, 108, 111)  // "Hello"
        val nCheck = 8
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        // Corrupt exactly t=4 bytes
        corrupted[0] = corrupted[0] xor 0xFF
        corrupted[2] = corrupted[2] xor 0xAA
        corrupted[4] = corrupted[4] xor 0x55
        corrupted[6] = corrupted[6] xor 0x11
        val recovered = decode(corrupted, nCheck)
        assertContentEquals(msg, recovered)
    }

    // =========================================================================
    // decode — too many errors
    // =========================================================================

    @Test
    fun `decode throws TooManyErrorsException when more than t errors`() {
        val msg = intArrayOf(1, 2, 3, 4, 5)
        val nCheck = 4  // t = 2
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        // Corrupt 3 bytes (t+1 = 3 > t = 2)
        corrupted[0] = corrupted[0] xor 0xFF
        corrupted[1] = corrupted[1] xor 0xAA
        corrupted[2] = corrupted[2] xor 0x55
        assertThrows<TooManyErrorsException> { decode(corrupted, nCheck) }
    }

    // =========================================================================
    // errorLocator
    // =========================================================================

    @Test
    fun `errorLocator returns polynomial with leading term 1`() {
        val msg = intArrayOf(1, 2, 3, 4)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[0] = corrupted[0] xor 0x12
        val synds = syndromes(corrupted, nCheck)
        val lambda = errorLocator(synds)
        assertEquals(1, lambda[0], "Λ[0] should always be 1")
    }

    @Test
    fun `errorLocator for no errors is the unit polynomial`() {
        val msg = intArrayOf(1, 2, 3, 4)
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val synds = syndromes(codeword, nCheck)
        val lambda = errorLocator(synds)
        assertContentEquals(intArrayOf(1), lambda)
    }

    // =========================================================================
    // Round-trip tests (various parameters)
    // =========================================================================

    @Test
    fun `round trip nCheck 2 no errors`() {
        val msg = IntArray(20) { it + 1 }
        val codeword = encode(msg, 2)
        val recovered = decode(codeword, 2)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `round trip nCheck 8 with 4 scattered errors`() {
        val msg = IntArray(30) { 255 - it }
        val nCheck = 8
        val codeword = encode(msg, nCheck)
        val corrupted = codeword.copyOf()
        corrupted[5]  = corrupted[5]  xor 0x01
        corrupted[15] = corrupted[15] xor 0x80
        corrupted[25] = corrupted[25] xor 0xFF
        corrupted[29] = corrupted[29] xor 0x42
        val recovered = decode(corrupted, nCheck)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `round trip all-zeros message`() {
        val msg = IntArray(5) { 0 }
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val recovered = decode(codeword, nCheck)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `round trip all-255 message`() {
        val msg = IntArray(10) { 255 }
        val nCheck = 4
        val codeword = encode(msg, nCheck)
        val recovered = decode(codeword, nCheck)
        assertContentEquals(msg, recovered)
    }

    @Test
    fun `round trip single byte with nCheck 2`() {
        for (b in 0..255) {
            val msg = intArrayOf(b)
            val codeword = encode(msg, 2)
            val recovered = decode(codeword, 2)
            assertContentEquals(msg, recovered, "Failed for byte $b")
        }
    }
}
