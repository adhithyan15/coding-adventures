package com.codingadventures.wasmtypes

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class WasmTypesTest {
    @Test
    fun buildsFunctionTypesAndModuleContainers() {
        val signature = makeFuncType(listOf(ValueType.I32), listOf(ValueType.I64))
        val module = WasmModule()
        module.types += signature
        module.functions += 0

        assertEquals(1, module.types.size)
        assertEquals(ValueType.I32, signature.params.first())
        assertEquals(ValueType.I64, signature.results.first())
        assertNull(module.start)
    }

    @Test
    fun preservesBytePayloadsInStructuralTypes() {
        val global = Global(GlobalType(ValueType.I32, false), byteArrayOf(0x41, 0x2A, 0x0B))
        val segment = DataSegment(0, byteArrayOf(0x41, 0x00, 0x0B), byteArrayOf(1, 2, 3))

        assertEquals(3, global.initExpr.size)
        assertEquals(3, segment.data.size)
    }
}
