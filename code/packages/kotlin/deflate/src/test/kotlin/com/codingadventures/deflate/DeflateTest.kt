package com.codingadventures.deflate

import java.io.ByteArrayOutputStream
import java.util.HexFormat
import java.util.zip.DataFormatException
import java.util.zip.Inflater
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class DeflateTest {
    @Test
    fun `emits the exact empty and fixed vectors`() {
        assertContentEquals(hex("0300"), Deflate.compress(ByteArray(0)))
        assertContentEquals(hex("7374747472720600"), Deflate.compress(bytes("AAABBC")))
    }

    @Test
    fun `round trips representative inputs`() {
        val allBytes = ByteArray(256) { it.toByte() }
        val repetition = bytes("the quick brown fox jumps over the lazy dog ".repeat(80))
        val examples = listOf(
            ByteArray(0),
            bytes("A"),
            bytes("AAAAAAA"),
            bytes("AABCBBABC"),
            allBytes,
            repetition,
        )
        for (input in examples) {
            val compressed = Deflate.compress(input)
            assertContentEquals(input, Deflate.decompress(compressed))
            assertContentEquals(input, inflateWithJdk(compressed))
            assertEquals(1, compressed[0].toInt() and 1, "compress emits one final block")
            assertTrue(firstBlockType(compressed) in 1..2)
        }
        assertTrue(Deflate.compress(repetition).size < repetition.size / 4)
    }

    @Test
    fun `reads stored fixed and independent dynamic streams`() {
        assertContentEquals(bytes("foo"), Deflate.inflate(hex("010300fcff666f6f")))
        assertContentEquals(bytes("AAABBC"), Deflate.inflate(hex("7374747472720600")))
        assertContentEquals(hex(DYNAMIC_INPUT), Deflate.inflate(hex(DYNAMIC_STREAM)))
    }

    @Test
    fun `selects dynamic coding when it wins`() {
        val input = hex(DYNAMIC_INPUT)
        val compressed = Deflate.compress(input)
        assertEquals(2, firstBlockType(compressed))
        assertTrue(compressed.size < input.size)
        assertContentEquals(input, inflateWithJdk(compressed))
    }

    @Test
    fun `uses the exact candidate bit costs for the block decision`() {
        val input = hex(DYNAMIC_INPUT)
        val costs = Deflate.candidateBitCosts(input)
        assertTrue(costs[1] < costs[0])
        val compressed = Deflate.compress(input)
        assertEquals(2, firstBlockType(compressed))
        assertEquals((costs[1] + 7) / 8, compressed.size.toLong())
    }

    @Test
    fun `rejects truncation trailing bytes invalid blocks and bombs`() {
        val compressed = Deflate.compress(bytes("hello hello hello hello"))
        assertFailsWith<IllegalArgumentException> { Deflate.inflate(compressed.copyOf(compressed.size - 1)) }
        assertFailsWith<IllegalArgumentException> { Deflate.inflate(compressed + byteArrayOf(0)) }
        assertFailsWith<IllegalArgumentException> { Deflate.inflate(hex("07")) }
        val bomb = Deflate.compress(bytes("A".repeat(1_000)))
        assertFailsWith<IllegalArgumentException> { Deflate.inflate(bomb, 999) }
        assertFailsWith<IllegalArgumentException> { Deflate.inflate(bomb, -1) }
    }

    private fun firstBlockType(compressed: ByteArray): Int =
        (compressed[0].toInt() ushr 1) and 0x03

    private fun inflateWithJdk(compressed: ByteArray): ByteArray {
        val inflater = Inflater(true)
        inflater.setInput(compressed)
        val output = ByteArrayOutputStream()
        val buffer = ByteArray(256)
        try {
            while (!inflater.finished()) {
                val count = inflater.inflate(buffer)
                if (count == 0 && !inflater.finished() && (inflater.needsInput() || inflater.needsDictionary())) {
                    throw IllegalArgumentException("independent inflater stalled")
                }
                output.write(buffer, 0, count)
            }
            return output.toByteArray()
        } catch (exception: DataFormatException) {
            throw IllegalArgumentException("independent inflater rejected stream", exception)
        } finally {
            inflater.end()
        }
    }

    private fun bytes(value: String): ByteArray = value.encodeToByteArray()
    private fun hex(value: String): ByteArray = HexFormat.of().parseHex(value)

    private companion object {
        const val DYNAMIC_INPUT =
            "4141424142414142414141414143414142454241474144414141414341424241424141414241414241414342414242444241474141434142414243424441424242414345414241424248434141424141414141434141414441414244434441414144464141414141424141414142464242424241444344434141414241414141414141414141414442414141414143484141414141414341414542414241424744424141414342434141434743434343414241414141414246414548464141414141424443414243"
        const val DYNAMIC_STREAM =
            "4d8dd111c0200c4267031275ff890a36bd2bfa21bc0301fae45af2a3898d7a1d2f1b2e1b566808289603421dc7a3df4a0658ca4cad1b5ec4c5340cf445a39aeac137d1f99bfb02d181b6ac6971a1cf2c7b8c7a00"
    }
}
