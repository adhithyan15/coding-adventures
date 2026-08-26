package com.codingadventures.deflate

import com.codingadventures.lzss.LZSS
import com.codingadventures.lzss.Literal
import com.codingadventures.lzss.Match
import com.codingadventures.lzss.Token
import java.io.ByteArrayOutputStream
import java.util.zip.DataFormatException
import java.util.zip.Inflater

object Deflate {
    const val DEFAULT_MAX_OUTPUT = 256 * 1024 * 1024

    private val lengthBase = intArrayOf(
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31,
        35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258,
    )
    private val lengthExtra = intArrayOf(
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
        3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    )
    private val distanceBase = intArrayOf(
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129,
        193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097,
        6145, 8193, 12289, 16385, 24577,
    )
    private val distanceExtra = intArrayOf(
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6,
        6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
    )
    private val codeLengthPermutation = intArrayOf(
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    )

    fun compress(data: ByteArray): ByteArray {
        val tokens = LZSS.encode(data, 32_768, 255, 3)
        val fixedBits = fixedBlockBits(tokens)
        val dynamic = planDynamic(tokens)
        val writer = BitWriter()
        if (dynamic.totalBits < fixedBits) {
            emitDynamicBlock(writer, tokens, dynamic)
        } else {
            emitFixedBlock(writer, tokens)
        }
        return writer.finish()
    }

    fun decompress(data: ByteArray): ByteArray = inflate(data)

    fun inflate(data: ByteArray, maxOutput: Int = DEFAULT_MAX_OUTPUT): ByteArray {
        require(maxOutput >= 0) { "maximum output must be non-negative" }
        val inflater = Inflater(true)
        inflater.setInput(data)
        val output = ByteArrayOutputStream(minOf(maxOutput, 8192))
        val buffer = ByteArray(8192)
        try {
            while (!inflater.finished()) {
                val count = inflater.inflate(buffer)
                require(count <= maxOutput - output.size()) { "DEFLATE output exceeds configured limit" }
                output.write(buffer, 0, count)
                if (count == 0 && !inflater.finished()) {
                    when {
                        inflater.needsInput() -> throw IllegalArgumentException("truncated DEFLATE stream")
                        else -> throw IllegalArgumentException("malformed DEFLATE stream")
                    }
                }
            }
            require(inflater.remaining == 0) { "trailing bytes after DEFLATE stream" }
            return output.toByteArray()
        } catch (exception: DataFormatException) {
            throw IllegalArgumentException("malformed DEFLATE stream", exception)
        } finally {
            inflater.end()
        }
    }

    private fun emitFixedBlock(writer: BitWriter, tokens: List<Token>) {
        writer.writeBits(1, 1)
        writer.writeBits(1, 2)
        for (token in tokens) {
            when (token) {
                is Literal -> writeFixedSymbol(writer, token.value)
                is Match -> {
                    writeLength(writer, token.length)
                    writeDistance(writer, token.offset)
                }
            }
        }
        writeFixedSymbol(writer, 256)
    }

    private fun writeLength(writer: BitWriter, length: Int) {
        for (index in lengthBase.indices) {
            val base = lengthBase[index]
            val extraBits = lengthExtra[index]
            val maximum = base + (1 shl extraBits) - 1
            if (length in base..maximum) {
                writeFixedSymbol(writer, 257 + index)
                writer.writeBits(length - base, extraBits)
                return
            }
        }
        throw IllegalArgumentException("LZSS match length cannot be represented by DEFLATE")
    }

    private fun writeDistance(writer: BitWriter, distance: Int) {
        for (index in distanceBase.indices) {
            val base = distanceBase[index]
            val extraBits = distanceExtra[index]
            val maximum = base + (1 shl extraBits) - 1
            if (distance in base..maximum) {
                writer.writeBits(reverseBits(index, 5), 5)
                writer.writeBits(distance - base, extraBits)
                return
            }
        }
        throw IllegalArgumentException("LZSS match distance cannot be represented by DEFLATE")
    }

    private fun writeFixedSymbol(writer: BitWriter, symbol: Int) {
        val (code, bitCount) = when (symbol) {
            in 0..143 -> 0x30 + symbol to 8
            in 144..255 -> 0x190 + symbol - 144 to 9
            in 256..279 -> symbol - 256 to 7
            in 280..287 -> 0xc0 + symbol - 280 to 8
            else -> throw IllegalArgumentException("invalid fixed Huffman symbol")
        }
        writer.writeBits(reverseBits(code, bitCount), bitCount)
    }

    private fun reverseBits(value: Int, bitCount: Int): Int {
        var result = 0
        repeat(bitCount) { index -> result = (result shl 1) or ((value ushr index) and 1) }
        return result
    }

    internal fun candidateBitCosts(data: ByteArray): LongArray {
        val tokens = LZSS.encode(data, 32_768, 255, 3)
        return longArrayOf(fixedBlockBits(tokens), planDynamic(tokens).totalBits)
    }

    private fun lengthIndex(length: Int): Int {
        for (index in lengthBase.indices) {
            val maximum = lengthBase[index] + (1 shl lengthExtra[index]) - 1
            if (length in lengthBase[index]..maximum) return index
        }
        throw IllegalArgumentException("LZSS match length cannot be represented by DEFLATE")
    }

    private fun distanceIndex(distance: Int): Int {
        for (index in distanceBase.indices) {
            val maximum = distanceBase[index] + (1 shl distanceExtra[index]) - 1
            if (distance in distanceBase[index]..maximum) return index
        }
        throw IllegalArgumentException("LZSS match distance cannot be represented by DEFLATE")
    }

    private fun fixedSymbolBits(symbol: Int): Int = when (symbol) {
        in 0..143 -> 8
        in 144..255 -> 9
        in 256..279 -> 7
        in 280..287 -> 8
        else -> throw IllegalArgumentException("invalid fixed Huffman symbol")
    }

    private fun fixedBlockBits(tokens: List<Token>): Long {
        var bits = 3L
        for (token in tokens) {
            when (token) {
                is Literal -> bits += fixedSymbolBits(token.value)
                is Match -> {
                    val lengthIndex = lengthIndex(token.length)
                    val distanceIndex = distanceIndex(token.offset)
                    bits += fixedSymbolBits(257 + lengthIndex) + lengthExtra[lengthIndex]
                    bits += 5L + distanceExtra[distanceIndex]
                }
            }
        }
        return bits + fixedSymbolBits(256)
    }

    private fun lengthLimitedHuffman(frequencies: LongArray, maxLength: Int): IntArray {
        val lengths = IntArray(frequencies.size)
        val present = frequencies.indices.filter { frequencies[it] > 0 }
        if (present.isEmpty()) return lengths
        if (present.size == 1) {
            lengths[present[0]] = 1
            return lengths
        }
        require(present.size <= (1 shl maxLength)) { "alphabet exceeds Huffman length limit" }

        val originals = present.mapIndexed { index, symbol ->
            PackageItem(frequencies[symbol], listOf(index))
        }.sortedWith(compareBy<PackageItem> { it.weight }.thenBy { it.covers[0] })
        var items = originals
        repeat(maxLength - 1) {
            val packaged = buildList {
                var index = 0
                while (index + 1 < items.size) {
                    val left = items[index]
                    val right = items[index + 1]
                    add(PackageItem(left.weight + right.weight, left.covers + right.covers))
                    index += 2
                }
            }
            items = buildList(originals.size + packaged.size) {
                var originalIndex = 0
                var packageIndex = 0
                while (originalIndex < originals.size && packageIndex < packaged.size) {
                    if (originals[originalIndex].weight <= packaged[packageIndex].weight) {
                        add(originals[originalIndex++])
                    } else {
                        add(packaged[packageIndex++])
                    }
                }
                addAll(originals.subList(originalIndex, originals.size))
                addAll(packaged.subList(packageIndex, packaged.size))
            }
        }

        val take = 2 * present.size - 2
        check(items.size >= take) { "package-merge produced an incomplete final list" }
        val depths = IntArray(present.size)
        for (item in items.take(take)) {
            for (covered in item.covers) depths[covered]++
        }
        var kraft = 0L
        val limit = 1L shl maxLength
        for (index in present.indices) {
            val depth = depths[index]
            check(depth in 1..maxLength) { "package-merge produced an invalid code length" }
            lengths[present[index]] = depth
            kraft += 1L shl (maxLength - depth)
        }
        check(kraft <= limit) { "package-merge violated Kraft's inequality" }
        return lengths
    }

    private fun canonicalCodes(lengths: IntArray): Array<HuffmanCode> {
        val codes = Array(lengths.size) { HuffmanCode(0, 0) }
        val maxLength = lengths.maxOrNull() ?: 0
        if (maxLength == 0) return codes
        val counts = IntArray(maxLength + 1)
        for (length in lengths) if (length > 0) counts[length]++
        val nextCode = IntArray(maxLength + 1)
        var code = 0
        for (bits in 1..maxLength) {
            code = (code + counts[bits - 1]) shl 1
            nextCode[bits] = code
        }
        for (symbol in lengths.indices) {
            val length = lengths[symbol]
            if (length > 0) codes[symbol] = HuffmanCode(nextCode[length]++, length)
        }
        return codes
    }

    private fun runLengthEncode(lengths: IntArray): List<CodeLengthItem> = buildList {
        var index = 0
        while (index < lengths.size) {
            val current = lengths[index]
            var run = 1
            while (index + run < lengths.size && lengths[index + run] == current) run++
            var remaining = run
            if (current == 0) {
                while (remaining >= 11) {
                    val count = minOf(remaining, 138)
                    add(CodeLengthItem(18, 7, count - 11))
                    remaining -= count
                }
                while (remaining >= 3) {
                    val count = minOf(remaining, 10)
                    add(CodeLengthItem(17, 3, count - 3))
                    remaining -= count
                }
                repeat(remaining) { add(CodeLengthItem(0, 0, 0)) }
            } else {
                add(CodeLengthItem(current, 0, 0))
                remaining--
                while (remaining >= 3) {
                    val count = minOf(remaining, 6)
                    add(CodeLengthItem(16, 2, count - 3))
                    remaining -= count
                }
                repeat(remaining) { add(CodeLengthItem(current, 0, 0)) }
            }
            index += run
        }
    }

    private fun planDynamic(tokens: List<Token>): DynamicPlan {
        val literalLengthFrequencies = LongArray(286)
        val distanceFrequencies = LongArray(30)
        literalLengthFrequencies[256] = 1
        for (token in tokens) {
            when (token) {
                is Literal -> literalLengthFrequencies[token.value]++
                is Match -> {
                    literalLengthFrequencies[257 + lengthIndex(token.length)]++
                    distanceFrequencies[distanceIndex(token.offset)]++
                }
            }
        }
        val literalLengthFull = lengthLimitedHuffman(literalLengthFrequencies, 15)
        val distanceFull = lengthLimitedHuffman(distanceFrequencies, 15)
        if (distanceFull.none { it > 0 }) distanceFull[0] = 1

        var literalLengthCount = 286
        while (literalLengthCount > 257 && literalLengthFull[literalLengthCount - 1] == 0) {
            literalLengthCount--
        }
        var distanceCount = 30
        while (distanceCount > 1 && distanceFull[distanceCount - 1] == 0) distanceCount--
        val literalLengths = literalLengthFull.copyOf(literalLengthCount)
        val distanceLengths = distanceFull.copyOf(distanceCount)
        val literalCodes = canonicalCodes(literalLengthFull)
        val distanceCodes = canonicalCodes(distanceFull)

        val combined = literalLengths + distanceLengths
        val runLengths = runLengthEncode(combined)
        val codeLengthFrequencies = LongArray(19)
        for (item in runLengths) codeLengthFrequencies[item.symbol]++
        val codeLengthLengths = lengthLimitedHuffman(codeLengthFrequencies, 7)
        val codeLengthCodes = canonicalCodes(codeLengthLengths)
        var codeLengthCount = 19
        while (codeLengthCount > 4 &&
            codeLengthLengths[codeLengthPermutation[codeLengthCount - 1]] == 0
        ) {
            codeLengthCount--
        }

        var totalBits = 3L + 5 + 5 + 4 + 3L * codeLengthCount
        for (item in runLengths) {
            totalBits += codeLengthLengths[item.symbol] + item.extraBits
        }
        for (token in tokens) {
            when (token) {
                is Literal -> totalBits += literalLengthFull[token.value]
                is Match -> {
                    val lengthIndex = lengthIndex(token.length)
                    val distanceIndex = distanceIndex(token.offset)
                    totalBits += literalLengthFull[257 + lengthIndex] + lengthExtra[lengthIndex]
                    totalBits += distanceFull[distanceIndex] + distanceExtra[distanceIndex]
                }
            }
        }
        totalBits += literalLengthFull[256]
        return DynamicPlan(
            literalLengths = literalLengths,
            distanceLengths = distanceLengths,
            literalCodes = literalCodes,
            distanceCodes = distanceCodes,
            codeLengthLengths = codeLengthLengths,
            codeLengthCodes = codeLengthCodes,
            codeLengthCount = codeLengthCount,
            runLengths = runLengths,
            totalBits = totalBits,
        )
    }

    private fun emitDynamicBlock(writer: BitWriter, tokens: List<Token>, plan: DynamicPlan) {
        writer.writeBits(1, 1)
        writer.writeBits(2, 2)
        writer.writeBits(plan.literalLengths.size - 257, 5)
        writer.writeBits(plan.distanceLengths.size - 1, 5)
        writer.writeBits(plan.codeLengthCount - 4, 4)
        repeat(plan.codeLengthCount) { index ->
            writer.writeBits(plan.codeLengthLengths[codeLengthPermutation[index]], 3)
        }
        for (item in plan.runLengths) {
            writer.writeHuffman(plan.codeLengthCodes[item.symbol])
            writer.writeBits(item.extraValue, item.extraBits)
        }
        for (token in tokens) {
            when (token) {
                is Literal -> writer.writeHuffman(plan.literalCodes[token.value])
                is Match -> {
                    val lengthIndex = lengthIndex(token.length)
                    writer.writeHuffman(plan.literalCodes[257 + lengthIndex])
                    writer.writeBits(token.length - lengthBase[lengthIndex], lengthExtra[lengthIndex])
                    val distanceIndex = distanceIndex(token.offset)
                    writer.writeHuffman(plan.distanceCodes[distanceIndex])
                    writer.writeBits(
                        token.offset - distanceBase[distanceIndex],
                        distanceExtra[distanceIndex],
                    )
                }
            }
        }
        writer.writeHuffman(plan.literalCodes[256])
    }

    private data class PackageItem(val weight: Long, val covers: List<Int>)

    private data class HuffmanCode(val code: Int, val bits: Int)

    private data class CodeLengthItem(val symbol: Int, val extraBits: Int, val extraValue: Int)

    private data class DynamicPlan(
        val literalLengths: IntArray,
        val distanceLengths: IntArray,
        val literalCodes: Array<HuffmanCode>,
        val distanceCodes: Array<HuffmanCode>,
        val codeLengthLengths: IntArray,
        val codeLengthCodes: Array<HuffmanCode>,
        val codeLengthCount: Int,
        val runLengths: List<CodeLengthItem>,
        val totalBits: Long,
    )

    private class BitWriter {
        private val output = ByteArrayOutputStream()
        private var currentByte = 0
        private var bitOffset = 0

        fun writeBits(value: Int, count: Int) {
            repeat(count) { index ->
                currentByte = currentByte or (((value ushr index) and 1) shl bitOffset)
                bitOffset++
                if (bitOffset == 8) {
                    output.write(currentByte)
                    currentByte = 0
                    bitOffset = 0
                }
            }
        }

        fun writeHuffman(code: HuffmanCode) {
            writeBits(reverseBits(code.code, code.bits), code.bits)
        }

        fun finish(): ByteArray {
            if (bitOffset != 0) output.write(currentByte)
            return output.toByteArray()
        }
    }
}
