package com.codingadventures.wasmvalidator

import com.codingadventures.wasmtypes.WasmModule
import com.codingadventures.wasmtypes.ExternalKind
import com.codingadventures.wasmtypes.Global
import com.codingadventures.wasmtypes.GlobalType
import com.codingadventures.wasmtypes.Import
import com.codingadventures.wasmtypes.Limits
import com.codingadventures.wasmtypes.MemoryType
import com.codingadventures.wasmtypes.ValueType
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertSame

class WasmValidatorTest {
    @Test
    fun exposesVersion() {
        assertEquals("0.1.0", VERSION)
    }

    @Test
    fun returnsValidatedModuleWrapper() {
        val module = WasmModule()
        val validated = validate(module)

        assertSame(module, validated.module)
        assertEquals(0, validated.funcTypes.size)
    }

    @Test
    fun rejectsMultipleMemories() {
        val module =
            WasmModule().apply {
                memories += MemoryType(Limits(1, null))
                memories += MemoryType(Limits(1, null))
            }

        val error = assertFailsWith<ValidationError> { validateStructure(module) }
        assertEquals(ValidationErrorKind.MULTIPLE_MEMORIES, error.kind)
    }

    @Test
    fun allowsImportedGlobalInConstExpr() {
        val module =
            WasmModule().apply {
                imports += Import("env", "seed", ExternalKind.GLOBAL, GlobalType(ValueType.I32, false))
                globals += Global(GlobalType(ValueType.I32, false), byteArrayOf(0x23, 0x00, 0x0B))
            }

        val indexSpaces = validateStructure(module)
        assertEquals(2, indexSpaces.globalTypes.size)
    }
}
