package com.codingadventures.lz78

import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.ArrayList
import java.util.Collections

data class Token(val dictionaryIndex: Int, val nextByte: Int) {
    init {
        require(dictionaryIndex in 0..0xffff) { "dictionary index must fit in 16 bits" }
        require(nextByte in 0..0xff) { "next byte must fit in 8 bits" }
    }
}

class EncodedTokens(tokens: List<Token>, val originalLength: Int) {
    val tokens: List<Token> = Collections.unmodifiableList(ArrayList(tokens))

    init {
        require(originalLength >= 0) { "original length must be non-negative" }
    }

    override fun equals(other: Any?): Boolean =
        other is EncodedTokens && tokens == other.tokens && originalLength == other.originalLength

    override fun hashCode(): Int = 31 * tokens.hashCode() + originalLength
}

object Lz78 {
    const val MAX_DICTIONARY_SIZE = 65_536
    const val DEFAULT_MAX_OUTPUT = 256 * 1024 * 1024

    fun encode(input: ByteArray, maxDictionarySize: Int = MAX_DICTIONARY_SIZE): List<Token> {
        require(maxDictionarySize in 1..MAX_DICTIONARY_SIZE) {
            "dictionary size must be between 1 and 65536"
        }
        val dictionary = mutableMapOf(ByteSequence(ByteArray(0)) to 0)
        val tokens = mutableListOf<Token>()
        var offset = 0
        var dictionarySize = 1
        while (offset < input.size) {
            var phraseIndex = 0
            var cursor = offset
            while (cursor < input.size) {
                val candidate = dictionary[ByteSequence(input.copyOfRange(offset, cursor + 1))] ?: break
                phraseIndex = candidate
                cursor++
            }
            if (cursor == input.size) {
                tokens += Token(phraseIndex, 0)
                break
            }
            val nextByte = input[cursor].toInt() and 0xff
            tokens += Token(phraseIndex, nextByte)
            if (dictionarySize < maxDictionarySize) {
                dictionary[ByteSequence(input.copyOfRange(offset, cursor + 1))] = dictionarySize++
            }
            offset = cursor + 1
        }
        return Collections.unmodifiableList(ArrayList(tokens))
    }

    fun decode(
        tokens: List<Token>,
        originalLength: Int,
        maxOutput: Int = DEFAULT_MAX_OUTPUT,
    ): ByteArray {
        require(originalLength >= 0) { "original length must be non-negative" }
        require(maxOutput >= 0 && originalLength <= maxOutput) { "decoded data exceeds configured limit" }
        val dictionary = mutableListOf(ByteArray(0))
        val output = ByteArrayOutputStream(minOf(originalLength, 8192))
        for (token in tokens) {
            require(token.dictionaryIndex < dictionary.size) {
                "token references an unavailable dictionary entry"
            }
            val phrase = dictionary[token.dictionaryIndex]
            require(phrase.size <= originalLength - output.size()) {
                "decoded data exceeds declared length"
            }
            output.write(phrase)
            val hasNextByte = output.size() < originalLength
            if (hasNextByte) output.write(token.nextByte)
            if (dictionary.size < MAX_DICTIONARY_SIZE && hasNextByte) {
                dictionary += phrase + token.nextByte.toByte()
            }
        }
        require(output.size() == originalLength) { "decoded data does not match declared length" }
        return output.toByteArray()
    }

    fun serialize(tokens: List<Token>, originalLength: Int): ByteArray {
        require(originalLength >= 0) { "original length must be non-negative" }
        val wireLength = 8L + 4L * tokens.size
        require(wireLength <= Int.MAX_VALUE) { "token stream is too large" }
        val buffer = ByteBuffer.allocate(wireLength.toInt()).order(ByteOrder.BIG_ENDIAN)
        buffer.putInt(originalLength)
        buffer.putInt(tokens.size)
        for (token in tokens) {
            buffer.putShort(token.dictionaryIndex.toShort())
            buffer.put(token.nextByte.toByte())
            buffer.put(0)
        }
        return buffer.array()
    }

    fun deserialize(data: ByteArray): EncodedTokens {
        require(data.size >= 8) { "truncated LZ78 header" }
        val buffer = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        val originalLength = buffer.int
        val tokenCount = buffer.int
        require(originalLength >= 0 && tokenCount >= 0) { "negative LZ78 header field" }
        require(8L + 4L * tokenCount == data.size.toLong()) {
            "LZ78 stream length does not match its header"
        }
        val tokens = buildList(tokenCount) {
            repeat(tokenCount) {
                val dictionaryIndex = buffer.short.toInt() and 0xffff
                val nextByte = buffer.get().toInt() and 0xff
                require(buffer.get().toInt() == 0) { "reserved LZ78 byte must be zero" }
                add(Token(dictionaryIndex, nextByte))
            }
        }
        return EncodedTokens(tokens, originalLength)
    }

    fun compress(
        input: ByteArray,
        maxDictionarySize: Int = MAX_DICTIONARY_SIZE,
    ): ByteArray = serialize(encode(input, maxDictionarySize), input.size)

    fun decompress(data: ByteArray, maxOutput: Int = DEFAULT_MAX_OUTPUT): ByteArray =
        deserialize(data).let { decode(it.tokens, it.originalLength, maxOutput) }

    private class ByteSequence(bytes: ByteArray) {
        private val value = bytes.copyOf()

        override fun equals(other: Any?): Boolean = other is ByteSequence && value.contentEquals(other.value)
        override fun hashCode(): Int = value.contentHashCode()
    }
}
