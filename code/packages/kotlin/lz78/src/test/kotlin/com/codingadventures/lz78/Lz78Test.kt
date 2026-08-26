package com.codingadventures.lz78

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class Lz78Test {
    @Test
    fun `matches the published token vectors`() {
        assertEquals(emptyList(), Lz78.encode(bytes("")))
        assertEquals(listOf(Token(0, 65)), Lz78.encode(bytes("A")))
        assertEquals(
            listOf(Token(0, 65), Token(0, 66), Token(0, 67), Token(0, 68), Token(0, 69)),
            Lz78.encode(bytes("ABCDE")),
        )
        assertEquals(
            listOf(Token(0, 65), Token(1, 65), Token(2, 65), Token(1, 0)),
            Lz78.encode(bytes("AAAAAAA")),
        )
        assertEquals(
            listOf(
                Token(0, 65),
                Token(1, 66),
                Token(0, 67),
                Token(0, 66),
                Token(4, 65),
                Token(4, 67),
            ),
            Lz78.encode(bytes("AABCBBABC")),
        )
        assertEquals(
            listOf(Token(0, 65), Token(0, 66), Token(1, 66), Token(3, 0)),
            Lz78.encode(bytes("ABABAB")),
        )
    }

    @Test
    fun `round trips text binary and dictionary boundaries`() {
        val examples = listOf(
            bytes(""),
            bytes("ABCDE"),
            bytes("AAAAAAA"),
            bytes("ABABABAB"),
            bytes("hello world hello world"),
            byteArrayOf(0, 0, 0, -1, -1, 0, 1, 2, 0, 1, 2),
        )
        for (input in examples) {
            assertContentEquals(input, Lz78.decompress(Lz78.compress(input)))
        }

        val literals = Lz78.encode(bytes("AAAA"), maxDictionarySize = 1)
        assertTrue(literals.all { it.dictionaryIndex == 0 })
        assertContentEquals(bytes("AAAA"), Lz78.decode(literals, 4))
        assertContentEquals(bytes("AAAA"), Lz78.decompress(Lz78.compress(bytes("AAAA"), 1)))
    }

    @Test
    fun `serialises the exact big endian teaching format`() {
        val wire = Lz78.serialize(listOf(Token(0, 65), Token(1, 66)), 3)
        assertContentEquals(
            byteArrayOf(0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 65, 0, 0, 1, 66, 0),
            wire,
        )
        val decoded = Lz78.deserialize(wire)
        assertEquals(3, decoded.originalLength)
        assertEquals(listOf(Token(0, 65), Token(1, 66)), decoded.tokens)
        val originalHash = decoded.hashCode()
        assertFailsWith<UnsupportedOperationException> {
            (decoded.tokens as MutableList).add(Token(0, 0))
        }
        assertEquals(originalHash, decoded.hashCode())
        assertContentEquals(ByteArray(8), Lz78.compress(ByteArray(0)))
    }

    @Test
    fun `rejects malformed or noncanonical streams`() {
        assertFailsWith<IllegalArgumentException> { Lz78.encode(bytes("x"), 0) }
        assertFailsWith<IllegalArgumentException> { Lz78.encode(bytes("x"), 65_537) }
        assertFailsWith<IllegalArgumentException> { Lz78.decode(listOf(Token(1, 65)), 1) }
        assertFailsWith<IllegalArgumentException> { Lz78.decode(emptyList(), 1, 0) }
        assertFailsWith<IllegalArgumentException> { Lz78.decode(emptyList(), 0, -1) }
        assertFailsWith<IllegalArgumentException> { Lz78.serialize(listOf(Token(65_536, 0)), 0) }
        assertFailsWith<IllegalArgumentException> { Lz78.deserialize(ByteArray(7)) }
        assertFailsWith<IllegalArgumentException> {
            Lz78.deserialize(byteArrayOf(0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 65))
        }
        assertFailsWith<IllegalArgumentException> {
            Lz78.deserialize(byteArrayOf(0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 65, 1))
        }
        assertFailsWith<IllegalArgumentException> {
            Lz78.deserialize(byteArrayOf(0, 0, 0, 0, 0, 0, 0, 0, 99))
        }
        assertFailsWith<IllegalArgumentException> {
            Lz78.decompress(byteArrayOf(0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 65, 0))
        }
        assertFailsWith<IllegalArgumentException> {
            Lz78.decompress(byteArrayOf(0x7f, -1, -1, -1, 0, 0, 0, 0))
        }
    }

    private fun bytes(value: String): ByteArray = value.encodeToByteArray()
}
