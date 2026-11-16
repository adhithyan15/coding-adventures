package com.codingadventures.sha512

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.io.File
import java.io.FileInputStream
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.HexFormat
import java.util.Random
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class Sha512Test {

    private val idPattern = Regex("^[a-z0-9]+(?:-[a-z0-9]+)*$")
    private val hexLowerPattern = Regex("^[0-9a-f]*$")
    private val digestHexPattern = Regex("^[0-9a-f]{128}$")
    private val word64HexPattern = Regex("^[0-9a-f]{16}$")
    private val chunksCsvPattern = Regex("^[1-9][0-9]*(?:,[1-9][0-9]*)*$")

    private fun findFixturesFile(): File {
        var current: File? = File(System.getProperty("user.dir"))
        for (i in 0 until 10) {
            if (current == null) break
            val candidate = File(current, "code/specs/fixtures/sha512-v1/cases.txt")
            if (candidate.exists() && candidate.isFile) {
                return candidate
            }
            val candidate2 = File(current, "fixtures/sha512-v1/cases.txt")
            if (candidate2.exists() && candidate2.isFile) {
                return candidate2
            }
            current = current.parentFile
        }
        throw IllegalStateException("Could not find sha512-v1/cases.txt")
    }

    private fun readBoundedCorpus(file: File, maxBytes: Int): String {
        val buf = ByteArray(maxBytes + 1)
        var total = 0
        FileInputStream(file).use { input ->
            while (total < buf.size) {
                val read = input.read(buf, total, buf.size - total)
                if (read <= 0) break
                total += read
                if (total > maxBytes) {
                    throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_OVERSIZED)
                }
            }
        }
        if (total < 1 || total > maxBytes) {
            throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_OVERSIZED)
        }
        val decoder = StandardCharsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
        return decoder.decode(ByteBuffer.wrap(buf, 0, total)).toString()
    }

    private fun parseCorpus(content: String) {
        val lines = content.split("\r?\n".toRegex())
        var rowCount = 0
        var vCount = 0
        var lCount = 0

        for (line in lines) {
            val trimmed = line.trim()
            if (trimmed.isEmpty() || trimmed.startsWith("#")) {
                continue
            }
            rowCount++
            val parts = trimmed.split("|")
            if (parts.size != 5) {
                throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
            }
            val tag = parts[0]
            val id = parts[1]
            if (!idPattern.matches(id)) {
                throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
            }

            if (tag == "V") {
                vCount++
                val inputHex = parts[2]
                val chunkCsv = parts[3]
                val expectedDigestHex = parts[4]

                if (!hexLowerPattern.matches(inputHex) || (inputHex.length % 2 != 0) || inputHex.length > 8192) {
                    throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
                }
                if (!digestHexPattern.matches(expectedDigestHex)) {
                    throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
                }
                val input = if (inputHex.isEmpty()) ByteArray(0) else HexFormat.of().parseHex(inputHex)
                if (input.isEmpty()) {
                    if (chunkCsv.isNotEmpty()) {
                        throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
                    }
                } else {
                    if (chunkCsv.isEmpty() || !chunksCsvPattern.matches(chunkCsv)) {
                        throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
                    }
                    val chunks = chunkCsv.split(",")
                    if (chunks.size > 64) {
                        throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
                    }
                    var sum = 0
                    for (c in chunks) {
                        sum += c.toInt()
                    }
                    if (sum != input.size) {
                        throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
                    }
                }

                // Verify one-shot and streaming
                val digest = sha512(input)
                assertEquals(64, digest.size)
                assertEquals(expectedDigestHex, HexFormat.of().formatHex(digest))
                assertEquals(expectedDigestHex, sha512Hex(input))

                val hasher = Sha512.create()
                if (chunkCsv.isNotEmpty()) {
                    var offset = 0
                    for (chunkStr in chunkCsv.split(",")) {
                        val size = chunkStr.toInt()
                        hasher.update(input.copyOfRange(offset, offset + size))
                        offset += size
                    }
                } else {
                    hasher.update(input)
                }
                assertEquals(expectedDigestHex, hasher.hexDigest())
            } else if (tag == "L") {
                lCount++
                val highStr = parts[2]
                val lowStr = parts[3]
                val expected = parts[4]

                if (!word64HexPattern.matches(highStr) || !word64HexPattern.matches(lowStr)) {
                    throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
                }
                val high = highStr.toULong(16)
                val low = lowStr.toULong(16)

                if (expected == "ERROR") {
                    val ex = assertThrows<Sha512Exception> {
                        encodeBitLength(high, low)
                    }
                    assertEquals(Sha512Messages.ERR_MESSAGE_TOO_LONG, ex.message)
                    assertEquals(Sha512ErrorCode.MESSAGE_TOO_LONG, ex.code)
                } else {
                    if (!hexLowerPattern.matches(expected) || expected.length != 32) {
                        throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
                    }
                    val encoded = encodeBitLength(high, low)
                    assertEquals(16, encoded.size)
                    assertEquals(expected, HexFormat.of().formatHex(encoded))
                }
            } else {
                throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
            }
        }

        if (rowCount !in 1..64) {
            throw IllegalArgumentException(Sha512Messages.ERR_CORPUS_MALFORMED)
        }
        assertEquals(9, vCount, "Corpus must have exactly 9 V rows")
        assertEquals(6, lCount, "Corpus must have exactly 6 L rows")
    }

    @Test
    fun testNeutralConformanceCorpus() {
        val fixtureFile = findFixturesFile()
        val content = readBoundedCorpus(fixtureFile, 4096)
        parseCorpus(content)
    }

    @Test
    fun testCorpusReaderFailsClosedOnOversized() {
        val dummy = File.createTempFile("oversized", ".txt")
        dummy.deleteOnExit()
        java.nio.file.Files.write(dummy.toPath(), ByteArray(4097))
        assertThrows<IllegalArgumentException> {
            readBoundedCorpus(dummy, 4096)
        }

        val empty = File.createTempFile("empty", ".txt")
        empty.deleteOnExit()
        assertThrows<IllegalArgumentException> {
            readBoundedCorpus(empty, 4096)
        }
    }

    @Test
    fun testCorpusReaderFailsClosedOnMalformedRows() {
        // Unknown tag
        assertThrows<IllegalArgumentException> { parseCorpus("X|unknown|1234|1|digest\n") }
        // Wrong column count
        assertThrows<IllegalArgumentException> { parseCorpus("V|id|1234\n") }
        // Uppercase hex in V input
        assertThrows<IllegalArgumentException> { parseCorpus("V|id|AB|1|" + "0".repeat(128) + "\n") }
        // Uppercase hex in V digest
        assertThrows<IllegalArgumentException> { parseCorpus("V|id|aa|1|" + "A".repeat(128) + "\n") }
        // Chunk sizes not summing to input size
        assertThrows<IllegalArgumentException> { parseCorpus("V|id|aabb|1|" + "0".repeat(128) + "\n") }
        // Invalid ID characters
        assertThrows<IllegalArgumentException> { parseCorpus("V|ID_UPPER|aa|1|" + "0".repeat(128) + "\n") }
        // Uppercase in L row word
        assertThrows<IllegalArgumentException> { parseCorpus("L|id|000000000000000A|0000000000000000|00000000000000000000000000000000\n") }
        // Short L row word
        assertThrows<IllegalArgumentException> { parseCorpus("L|id|0|0|ERROR\n") }
    }

    // ─── 6-Class SHA-512 Resilience Showcase ──────────────────────────────────

    @Test
    fun testCveClass1CounterWraparoundPast61Bits() {
        val lowOverflow = 0x2000000000000001uL // > 2^61 bytes
        val encoded = encodeBitLength(0uL, lowOverflow)
        assertEquals("00000000000000010000000000000008", HexFormat.of().formatHex(encoded))

        val encodedHigh = encodeBitLength(1uL, 0uL)
        assertEquals("00000000000000080000000000000000", HexFormat.of().formatHex(encodedHigh))
    }

    @Test
    fun testCveClass2FipsOverflowFailureAtomicity() {
        val hasher = Sha512.create()
        hasher.update(byteArrayOf(1, 2, 3))
        val hexBefore = hasher.hexDigest()

        val ex = assertThrows<Sha512Exception> {
            encodeBitLength(0x2000000000000000uL, 0uL)
        }
        assertEquals(Sha512ErrorCode.MESSAGE_TOO_LONG, ex.code)

        // Hasher state must remain intact
        assertEquals(hexBefore, hasher.hexDigest())
        hasher.update(byteArrayOf(4, 5, 6))
        assertNotEquals(hexBefore, hasher.hexDigest())

        // Exercise Hasher.update counter carry
        val carryHasher = Sha512.create()
        carryHasher.setByteCountersForTesting(0uL, 0xFFFF_FFFF_FFFF_FFFEuL)
        carryHasher.update(byteArrayOf(1, 2, 3))
        assertNotNull(carryHasher.hexDigest())

        // Exercise Hasher.update 128-bit domain overflow failure atomicity
        val overflowHasher = Sha512.create()
        overflowHasher.update(byteArrayOf(9, 8, 7))
        overflowHasher.setByteCountersForTesting(0x1FFF_FFFF_FFFF_FFFFuL, 0xFFFF_FFFF_FFFF_FFFFuL)
        val snap = overflowHasher.hexDigest()

        val ofEx = assertThrows<Sha512Exception> {
            overflowHasher.update(byteArrayOf(1))
        }
        assertEquals(Sha512ErrorCode.MESSAGE_TOO_LONG, ofEx.code)
        assertEquals(snap, overflowHasher.hexDigest())
    }

    @Test
    fun testCveClass3PaddingBoundarySplits() {
        val std = MessageDigest.getInstance("SHA-512")

        for (boundary in intArrayOf(111, 112, 127, 128, 129, 239, 240, 255, 256, 257)) {
            val data = ByteArray(boundary) { i -> (i * 31 + 7).toByte() }
            val expected = std.digest(data)
            val actual = sha512(data)
            assertContentEquals(expected, actual, "Failed on boundary $boundary")
        }
    }

    @Test
    fun testCveClass4BufferAliasingAndContinuation() {
        val input = byteArrayOf(10, 20, 30, 40)
        val hasher = Sha512.create()
        hasher.update(input)

        input[0] = 99
        val snapshot1 = hasher.hexDigest()

        hasher.update(byteArrayOf(50, 60))
        val snapshot2 = hasher.hexDigest()
        assertFalse(snapshot1 == snapshot2)

        val copy = hasher.copy()
        assertEquals(snapshot2, copy.hexDigest())
        copy.update(byteArrayOf(70))
        assertFalse(hasher.hexDigest() == copy.hexDigest())
    }

    @Test
    fun testCveClass5FailClosedOnInvalidIndicesAndIntegerOverflow() {
        val hasher = Sha512.create()
        hasher.update(byteArrayOf(1, 2, 3))
        val before = hasher.hexDigest()

        // Integer overflow attack (offset = 1, length = Int.MAX_VALUE)
        val testData = byteArrayOf(1, 2, 3, 4)
        val ex = assertThrows<IndexOutOfBoundsException> {
            hasher.update(testData, 1, Int.MAX_VALUE)
        }
        assertEquals(Sha512Messages.ERR_INDEX_OUT_OF_BOUNDS, ex.message)
        assertEquals(before, hasher.hexDigest()) // Hasher state must be completely unmodified!

        assertThrows<IndexOutOfBoundsException> {
            hasher.update(testData, -1, 1)
        }
        assertThrows<IndexOutOfBoundsException> {
            hasher.update(testData, 5, 1)
        }
        assertThrows<IndexOutOfBoundsException> {
            hasher.update(testData, 0, 10)
        }
    }

    @Test
    fun testCveClass6PayloadBlindErrorDiagnostics() {
        val ex = assertThrows<Sha512Exception> {
            encodeBitLength(0xE000000000000000uL, 0uL)
        }
        assertEquals(Sha512Messages.ERR_MESSAGE_TOO_LONG, ex.message)
        assertEquals(Sha512ErrorCode.MESSAGE_TOO_LONG, ex.code)
        assertFalse(ex.message!!.contains("0xE000"))
    }

    // ─── Multi-chunk Streaming Cross-check & 10,000-Iteration Fuzz ─────────────

    @Test
    fun testMultiChunkAgainstJavaSecurityMessageDigest() {
        val std = MessageDigest.getInstance("SHA-512")
        val prng = Random(0x5EED512L)

        for (len in intArrayOf(0, 1, 16, 64, 127, 128, 129, 256, 1000, 4096)) {
            val data = ByteArray(len)
            prng.nextBytes(data)

            std.reset()
            std.update(data)
            val expected = std.digest()

            val hasher = Sha512.create()
            var offset = 0
            while (offset < len) {
                val chunkSize = minOf(len - offset, prng.nextInt(64) + 1)
                hasher.update(data, offset, chunkSize)
                offset += chunkSize
            }
            val actual = hasher.digest()
            assertContentEquals(expected, actual, "Mismatch on length $len")
        }
    }

    @Test
    fun testDeterministicFuzzLoop10kIterations() {
        val std = MessageDigest.getInstance("SHA-512")
        val prng = Random(0xDEADBEEF512L)

        for (iter in 0 until 10_000) {
            val len = prng.nextInt(300)
            val data = ByteArray(len)
            prng.nextBytes(data)

            std.reset()
            std.update(data)
            val expected = std.digest()

            val hasher = Sha512.create()
            var offset = 0
            while (offset < len) {
                val chunkSize = minOf(len - offset, prng.nextInt(50) + 1)
                hasher.update(data, offset, chunkSize)
                offset += chunkSize
            }
            assertContentEquals(expected, hasher.digest(), "Fuzz iteration failed at $iter")
        }
    }

    @Test
    fun testSha512ObjectDelegatesAndMessagesCoverage() {
        val data = byteArrayOf(1, 2, 3)
        assertContentEquals(sha512(data), Sha512.sha512(data))
        assertEquals(sha512Hex(data), Sha512.sha512Hex(data))

        val hasher = Sha512.create()
        hasher.update(byteArrayOf(1, 2, 3))
        hasher.update(byteArrayOf(4, 5, 6), 0, 0)
        assertEquals(sha512Hex(byteArrayOf(1, 2, 3)), hasher.hexDigest())

        for (code in Sha512ErrorCode.values()) {
            assertNotNull(Sha512Messages.messageFor(code))
        }
        assertEquals(Sha512Messages.ERR_UNKNOWN, Sha512Messages.messageFor(null))
    }
}
