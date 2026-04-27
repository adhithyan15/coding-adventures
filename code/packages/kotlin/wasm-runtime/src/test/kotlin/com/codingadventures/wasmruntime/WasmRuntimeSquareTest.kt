package com.codingadventures.wasmruntime

import com.codingadventures.wasmleb128.WasmLeb128
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class WasmRuntimeSquareTest {
    @Test
    fun loadAndRunExecutesSquareModule() {
        val runtime = WasmRuntime()
        assertEquals(listOf(25), runtime.loadAndRun(buildSquareWasm(), "square", listOf(5)))
        assertEquals(listOf(0), runtime.loadAndRun(buildSquareWasm(), "square", listOf(0)))
        assertEquals(listOf(9), runtime.loadAndRun(buildSquareWasm(), "square", listOf(-3)))
    }

    @Test
    fun squareModuleSupportsStepByStepFlow() {
        val runtime = WasmRuntime()
        val wasm = buildSquareWasm()

        val module = runtime.load(wasm)
        assertEquals(1, module.types.size)
        assertEquals(1, module.functions.size)
        assertEquals(1, module.exports.size)

        assertEquals(module, runtime.validate(module).module)

        val instance = runtime.instantiate(module)
        assertTrue(instance.exports.containsKey("square"))
        assertEquals(listOf(49), runtime.call(instance, "square", listOf(7)))
    }

    private fun buildSquareWasm(): ByteArray {
        val typePayload = byteArrayOf(0x01, 0x60, 0x01, 0x7F.toByte(), 0x01, 0x7F.toByte())
        val functionPayload = byteArrayOf(0x01, 0x00)
        val exportPayload = concat(byteArrayOf(0x01), makeString("square"), byteArrayOf(0x00, 0x00))
        val bodyPayload = byteArrayOf(0x00, 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B)
        val codePayload = concat(byteArrayOf(0x01), WasmLeb128.encodeUnsigned(bodyPayload.size.toLong()), bodyPayload)

        return concat(
            byteArrayOf(0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00),
            makeSection(1, typePayload),
            makeSection(3, functionPayload),
            makeSection(7, exportPayload),
            makeSection(10, codePayload),
        )
    }

    private fun makeSection(id: Int, payload: ByteArray): ByteArray =
        concat(byteArrayOf(id.toByte()), WasmLeb128.encodeUnsigned(payload.size.toLong()), payload)

    private fun makeString(value: String): ByteArray {
        val encoded = value.toByteArray(Charsets.UTF_8)
        return concat(WasmLeb128.encodeUnsigned(encoded.size.toLong()), encoded)
    }

    private fun concat(vararg parts: ByteArray): ByteArray = parts.fold(ByteArray(0)) { acc, part -> acc + part }
}
