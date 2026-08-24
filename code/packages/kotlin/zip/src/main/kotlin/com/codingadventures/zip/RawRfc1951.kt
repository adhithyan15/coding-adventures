package com.codingadventures.zip

import java.io.IOException

/** Default and hard raw-inflate output ceiling: 256 MiB. */
const val MAX_RAW_OUTPUT: Int = 256 * 1024 * 1024

/** Complete payload-blind failure taxonomy, in fixture order. */
val RAW_INFLATE_ERROR_CODES: List<String> = listOf(
    "invalid-output-limit",
    "unexpected-eof",
    "reserved-block-type",
    "stored-length-mismatch",
    "huffman-oversubscribed",
    "incomplete-code-length-tree",
    "incomplete-literal-length-tree",
    "incomplete-distance-tree",
    "repeat-without-previous",
    "repeat-overrun",
    "invalid-literal-length-symbol",
    "reserved-distance-symbol",
    "invalid-back-reference",
    "output-limit-exceeded",
)

/** A stable raw-inflate failure whose message contains only its code. */
class RawInflateError(val code: String) : IOException(code)

/** Decoded bytes and the exact compressed byte count reached. */
data class RawInflateResult(val output: ByteArray, val bytesConsumed: Int)

/** Compress bytes as raw RFC 1951 without ZIP, zlib, or gzip framing. */
fun rawDeflate(data: ByteArray): ByteArray = deflateCompress(data)

/** Inflate raw RFC 1951 with a caller-lowerable output ceiling. */
fun rawInflate(data: ByteArray, maxOutput: Int = MAX_RAW_OUTPUT): ByteArray =
    rawInflateCounted(data, maxOutput).output

/** Inflate and report the exact final compressed input byte reached. */
fun rawInflateCounted(data: ByteArray, maxOutput: Int = MAX_RAW_OUTPUT): RawInflateResult =
    RawInflater.inflate(data, maxOutput)

/**
 * Hardened raw RFC 1951 decoder owned by ZIP.
 *
 * Production is a pure in-memory byte transform and owns no filesystem,
 * process, network, environment, clock, entropy, FFI, or credential authority.
 */
private object RawInflater {
    private enum class Completeness { CODE_LENGTH, LITERAL_LENGTH, DISTANCE }

    private data class Tables(val literalLength: HuffmanTable, val distance: HuffmanTable)

    private class HuffmanTable(
        private val codesByLength: Array<Map<Int, Int>>,
        private val maximumLength: Int,
    ) {
        fun decode(reader: BitReader): Int {
            if (maximumLength == 0) fail("unexpected-eof")
            var code = 0
            for (length in 1..maximumLength) {
                val bit = reader.readLsb(1) ?: fail("unexpected-eof")
                code = (code shl 1) or bit
                codesByLength[length][code]?.let { return it }
            }
            fail("unexpected-eof")
        }
    }

    /** A capped growable byte vector with overlap-safe back-reference copying. */
    private class OutputBuffer(private val maximum: Int) {
        private var data = ByteArray(minOf(maximum, 8192))
        var size: Int = 0
            private set

        fun add(value: Int) {
            ensure(1)
            data[size++] = value.toByte()
        }

        fun copy(distance: Int, length: Int) {
            if (distance <= 0 || distance > size) fail("invalid-back-reference")
            ensure(length)
            repeat(length) {
                data[size] = data[size - distance]
                size++
            }
        }

        fun ensure(additional: Int) {
            if (additional > maximum - size) fail("output-limit-exceeded")
            val required = size + additional
            if (required <= data.size) return
            val doubled = if (data.isEmpty()) 1 else data.size * 2
            data = data.copyOf(minOf(maximum, maxOf(required, doubled)))
        }

        fun toByteArray(): ByteArray = data.copyOf(size)
    }

    fun inflate(data: ByteArray, maximumOutput: Int): RawInflateResult {
        if (maximumOutput < 0 || maximumOutput > MAX_RAW_OUTPUT) fail("invalid-output-limit")
        val reader = BitReader(data)
        val output = OutputBuffer(maximumOutput)

        while (true) {
            val finalBlock = reader.readLsb(1) ?: fail("unexpected-eof")
            when (reader.readLsb(2) ?: fail("unexpected-eof")) {
                0 -> readStored(reader, output)
                1 -> fixedTables().also {
                    decodeCompressed(reader, output, it.literalLength, it.distance)
                }
                2 -> readDynamicTables(reader).also {
                    decodeCompressed(reader, output, it.literalLength, it.distance)
                }
                else -> fail("reserved-block-type")
            }
            if (finalBlock == 1) return RawInflateResult(output.toByteArray(), reader.position())
        }
    }

    private fun readStored(reader: BitReader, output: OutputBuffer) {
        reader.align()
        val length = reader.readLsb(16) ?: fail("unexpected-eof")
        val complement = reader.readLsb(16) ?: fail("unexpected-eof")
        if (length != (complement xor 0xffff)) fail("stored-length-mismatch")
        output.ensure(length)
        repeat(length) { output.add(reader.readLsb(8) ?: fail("unexpected-eof")) }
    }

    private fun fixedTables(): Tables {
        val literalLengths = IntArray(288)
        java.util.Arrays.fill(literalLengths, 0, 144, 8)
        java.util.Arrays.fill(literalLengths, 144, 256, 9)
        java.util.Arrays.fill(literalLengths, 256, 280, 7)
        java.util.Arrays.fill(literalLengths, 280, 288, 8)
        return Tables(
            buildHuffman(literalLengths, Completeness.LITERAL_LENGTH),
            buildHuffman(IntArray(32) { 5 }, Completeness.DISTANCE),
        )
    }

    private fun readDynamicTables(reader: BitReader): Tables {
        val literalCount = (reader.readLsb(5) ?: fail("unexpected-eof")) + 257
        val distanceCount = (reader.readLsb(5) ?: fail("unexpected-eof")) + 1
        val codeLengthCount = (reader.readLsb(4) ?: fail("unexpected-eof")) + 4
        if (literalCount > 286) fail("invalid-literal-length-symbol")

        val order = intArrayOf(16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15)
        val codeLengths = IntArray(19)
        repeat(codeLengthCount) { index ->
            codeLengths[order[index]] = reader.readLsb(3) ?: fail("unexpected-eof")
        }
        val codeLengthTable = buildHuffman(codeLengths, Completeness.CODE_LENGTH)

        val total = literalCount + distanceCount
        val lengths = IntArray(total)
        var count = 0
        while (count < total) {
            when (val symbol = codeLengthTable.decode(reader)) {
                in 0..15 -> lengths[count++] = symbol
                16 -> {
                    if (count == 0) fail("repeat-without-previous")
                    count = repeatInto(lengths, count, lengths[count - 1],
                        (reader.readLsb(2) ?: fail("unexpected-eof")) + 3)
                }
                17 -> count = repeatInto(lengths, count, 0,
                    (reader.readLsb(3) ?: fail("unexpected-eof")) + 3)
                18 -> count = repeatInto(lengths, count, 0,
                    (reader.readLsb(7) ?: fail("unexpected-eof")) + 11)
                else -> fail("unexpected-eof")
            }
        }

        val literalLengths = lengths.copyOfRange(0, literalCount)
        val distanceLengths = lengths.copyOfRange(literalCount, total)
        if (literalLengths[256] == 0) fail("incomplete-literal-length-tree")
        return Tables(
            buildHuffman(literalLengths, Completeness.LITERAL_LENGTH),
            buildHuffman(distanceLengths, Completeness.DISTANCE),
        )
    }

    private fun repeatInto(target: IntArray, start: Int, value: Int, count: Int): Int {
        if (count > target.size - start) fail("repeat-overrun")
        java.util.Arrays.fill(target, start, start + count, value)
        return start + count
    }

    private fun buildHuffman(lengths: IntArray, completeness: Completeness): HuffmanTable {
        val counts = IntArray(16)
        for (length in lengths) {
            if (length > 15) fail("huffman-oversubscribed")
            if (length > 0) counts[length]++
        }
        var left = 1
        for (length in 1..15) {
            left = left * 2 - counts[length]
            if (left < 0) fail("huffman-oversubscribed")
        }
        val symbolCount = counts.sum()
        if (left != 0) {
            when (completeness) {
                Completeness.CODE_LENGTH -> fail("incomplete-code-length-tree")
                Completeness.LITERAL_LENGTH -> fail("incomplete-literal-length-tree")
                Completeness.DISTANCE -> if (symbolCount != 0 && !(symbolCount == 1 && counts[1] == 1))
                    fail("incomplete-distance-tree")
            }
        }

        val nextCode = IntArray(16)
        var code = 0
        for (length in 1..15) {
            code = (code + counts[length - 1]) shl 1
            nextCode[length] = code
        }
        val tables = Array<MutableMap<Int, Int>>(16) { mutableMapOf() }
        var maximumLength = 0
        for (symbol in lengths.indices) {
            val length = lengths[symbol]
            if (length == 0) continue
            tables[length][nextCode[length]++] = symbol
            maximumLength = maxOf(maximumLength, length)
        }
        return HuffmanTable(Array(16) { tables[it].toMap() }, maximumLength)
    }

    private fun decodeCompressed(
        reader: BitReader,
        output: OutputBuffer,
        literalLength: HuffmanTable,
        distance: HuffmanTable,
    ) {
        while (true) {
            when (val symbol = literalLength.decode(reader)) {
                in 0..255 -> output.add(symbol)
                256 -> return
                in 257..285 -> {
                    val (baseLength, extraLengthBits) = LENGTH_TABLE[symbol - 257]
                    val length = baseLength + (reader.readLsb(extraLengthBits) ?: fail("unexpected-eof"))
                    val distanceSymbol = distance.decode(reader)
                    if (distanceSymbol >= 30) fail("reserved-distance-symbol")
                    val (baseDistance, extraDistanceBits) = DIST_TABLE[distanceSymbol]
                    val backwardDistance = baseDistance +
                        (reader.readLsb(extraDistanceBits) ?: fail("unexpected-eof"))
                    output.copy(backwardDistance, length)
                }
                else -> fail("invalid-literal-length-symbol")
            }
        }
    }

    private fun fail(code: String): Nothing = throw RawInflateError(code)
}
