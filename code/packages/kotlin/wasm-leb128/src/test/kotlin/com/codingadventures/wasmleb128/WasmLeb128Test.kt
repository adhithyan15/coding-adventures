package com.codingadventures.wasmleb128

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class WasmLeb128Test {
    @Test
    fun decodesUnsignedValues() {
        val decoding = WasmLeb128.decodeUnsigned(byteArrayOf(0xE5.toByte(), 0x8E.toByte(), 0x26))

        assertEquals(624485L, decoding.value)
        assertEquals(3, decoding.bytesConsumed)
    }

    @Test
    fun decodesSignedValues() {
        val decoding = WasmLeb128.decodeSigned(byteArrayOf(0x7E))

        assertEquals(-2, decoding.value)
        assertEquals(1, decoding.bytesConsumed)
    }

    @Test
    fun encodesUnsignedAndSignedValues() {
        assertContentEquals(byteArrayOf(0xE5.toByte(), 0x8E.toByte(), 0x26), WasmLeb128.encodeUnsigned(624485))
        assertContentEquals(byteArrayOf(0x7E), WasmLeb128.encodeSigned(-2))
    }

    @Test
    fun rejectsUnterminatedSequence() {
        assertFailsWith<WasmLeb128.LEB128Error> {
            WasmLeb128.decodeUnsigned(byteArrayOf(0x80.toByte(), 0x80.toByte()))
        }
    }
}
