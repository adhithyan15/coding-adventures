package com.codingadventures.wasmmoduleparser

import com.codingadventures.wasmleb128.WasmLeb128
import com.codingadventures.wasmtypes.ExternalKind
import com.codingadventures.wasmtypes.FUNCREF
import com.codingadventures.wasmtypes.ValueType
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

class WasmModuleParserTest {
    private val parser = WasmModuleParser()

    @Test
    fun exposesVersion() {
        assertEquals("0.1.0", VERSION)
    }

    @Test
    fun parsesMinimalModule() {
        val module = parser.parse(WASM_HEADER)

        assertEquals(0, module.types.size)
        assertEquals(0, module.imports.size)
        assertEquals(0, module.functions.size)
        assertEquals(0, module.tables.size)
        assertEquals(0, module.memories.size)
        assertEquals(0, module.globals.size)
        assertEquals(0, module.exports.size)
        assertNull(module.start)
        assertEquals(0, module.elements.size)
        assertEquals(0, module.code.size)
        assertEquals(0, module.data.size)
        assertEquals(0, module.customs.size)
    }

    @Test
    fun parsesTypeFunctionExportAndCodeSections() {
        val typeSection = makeSection(1, byteArrayOf(1, 0x60, 2, I32.toByte(), I32.toByte(), 1, I32.toByte()))
        val functionSection = makeSection(3, byteArrayOf(1, 0))
        val exportSection = makeSection(7, concat(byteArrayOf(1), makeString("main"), byteArrayOf(0x00, 0x00)))
        val codeBody = byteArrayOf(1, 1, I32.toByte(), 0x20, 0x00, 0x21, 0x02, 0x20, 0x02, 0x0B)
        val codeSection = makeSection(10, concat(byteArrayOf(1), encodeUnsigned(codeBody.size), codeBody))

        val module = parser.parse(makeWasm(typeSection, functionSection, exportSection, codeSection))

        assertEquals(listOf(ValueType.I32, ValueType.I32), module.types[0].params)
        assertEquals(listOf(ValueType.I32), module.types[0].results)
        assertEquals(listOf(0), module.functions)
        assertEquals("main", module.exports[0].name)
        assertEquals(ExternalKind.FUNCTION, module.exports[0].kind)
        assertEquals(listOf(ValueType.I32), module.code[0].locals)
        assertContentEquals(byteArrayOf(0x20, 0x00, 0x21, 0x02, 0x20, 0x02, 0x0B), module.code[0].code)
    }

    @Test
    fun parsesImportsMemoryGlobalsDataElementsStartAndCustomSection() {
        val typeSection = makeSection(1, byteArrayOf(1, 0x60, 0, 0))
        val importSection =
            makeSection(
                2,
                concat(byteArrayOf(1), makeString("env"), makeString("host_add"), byteArrayOf(0x00, 0x00)),
            )
        val tableSection = makeSection(4, byteArrayOf(1, FUNCREF.toByte(), 0x00, 0x05))
        val memorySection = makeSection(5, byteArrayOf(1, 0x01, 0x01, 0x04))
        val globalSection = makeSection(6, byteArrayOf(1, I32.toByte(), 0x00, 0x41, 0x2A, 0x0B))
        val startSection = makeSection(8, byteArrayOf(0x01))
        val elementSection = makeSection(9, byteArrayOf(1, 0x00, 0x41, 0x02, 0x0B, 0x02, 0x05, 0x06))
        val dataSection = makeSection(11, concat(byteArrayOf(1, 0x00, 0x41, 0x03, 0x0B, 0x03), "abc".encodeToByteArray()))
        val customSection = makeSection(0, concat(makeString("name"), byteArrayOf(0x41, 0x42)))

        val module = parser.parse(makeWasm(typeSection, importSection, tableSection, memorySection, globalSection, startSection, elementSection, dataSection, customSection))

        assertEquals("env", module.imports[0].moduleName)
        assertEquals("host_add", module.imports[0].name)
        assertEquals(ExternalKind.FUNCTION, module.imports[0].kind)
        assertEquals(FUNCREF, module.tables[0].elementType)
        assertEquals(1, module.memories[0].limits.min)
        assertEquals(4, module.memories[0].limits.max)
        assertEquals(ValueType.I32, module.globals[0].globalType.valueType)
        assertEquals(1, module.start)
        assertEquals(listOf(5, 6), module.elements[0].functionIndices)
        assertContentEquals("abc".encodeToByteArray(), module.data[0].data)
        assertEquals("name", module.customs[0].name)
    }

    @Test
    fun rejectsInvalidMagic() {
        val error = assertFailsWith<WasmParseError> {
            parser.parse(byteArrayOf(0x01, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00))
        }

        assertEquals(0, error.offset)
    }

    @Test
    fun rejectsTruncatedSectionPayload() {
        val broken = concat(WASM_HEADER, byteArrayOf(0x01, 0x05, 0x01, 0x60))
        val error = assertFailsWith<WasmParseError> { parser.parse(broken) }
        assertEquals(10, error.offset)
    }

    @Test
    fun rejectsOutOfOrderSections() {
        val exportSection = makeSection(7, concat(byteArrayOf(1), makeString("main"), byteArrayOf(0x00, 0x00)))
        val typeSection = makeSection(1, byteArrayOf(1, 0x60, 0, 0))

        val error = assertFailsWith<WasmParseError> {
            parser.parse(makeWasm(exportSection, typeSection))
        }

        assertEquals(18, error.offset)
    }

    private fun makeWasm(vararg sections: ByteArray): ByteArray = concat(WASM_HEADER, *sections)

    private fun makeSection(id: Int, payload: ByteArray): ByteArray = concat(byteArrayOf(id.toByte()), encodeUnsigned(payload.size), payload)

    private fun makeString(value: String): ByteArray = concat(encodeUnsigned(value.encodeToByteArray().size), value.encodeToByteArray())

    private fun encodeUnsigned(value: Int): ByteArray = WasmLeb128.encodeUnsigned(value.toLong())

    private fun concat(vararg parts: ByteArray): ByteArray {
        val output = ArrayList<Byte>(parts.sumOf { it.size })
        parts.forEach { output.addAll(it.toList()) }
        return output.toByteArray()
    }

    companion object {
        private val WASM_HEADER = byteArrayOf(0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00)
        private const val I32 = 0x7F
    }
}
