package com.codingadventures.wasmopcodes

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class WasmOpcodesTest {
    @Test
    fun exposesVersion() {
        assertEquals("0.1.0", WasmOpcodes.VERSION)
    }

    @Test
    fun loadsFullOpcodeTable() {
        assertTrue(WasmOpcodes.OPCODES.size >= 172)
        assertEquals(WasmOpcodes.OPCODES.size, WasmOpcodes.OPCODES_BY_NAME.size)
    }

    @Test
    fun looksUpOpcodeByByte() {
        val info = WasmOpcodes.getOpcode(0x6A)

        assertEquals("i32.add", info?.name)
        assertEquals("numeric_i32", info?.category)
        assertEquals(2, info?.stackPop)
        assertEquals(1, info?.stackPush)
    }

    @Test
    fun looksUpOpcodeByName() {
        val info = WasmOpcodes.getOpcodeByName("call_indirect")

        assertEquals(0x11, info?.opcode)
        assertEquals(listOf("typeidx", "tableidx"), info?.immediates)
    }

    @Test
    fun returnsNullForUnknownOpcodeOrName() {
        assertNull(WasmOpcodes.getOpcode(0x06))
        assertNull(WasmOpcodes.getOpcode(0xFF))
        assertNull(WasmOpcodes.getOpcodeByName(""))
        assertNull(WasmOpcodes.getOpcodeByName("i32.foo"))
    }

    @Test
    fun preservesImmediateAndCategoryMetadata() {
        assertEquals(listOf("i32"), WasmOpcodes.getOpcodeByName("i32.const")?.immediates)
        assertEquals(listOf("memarg"), WasmOpcodes.getOpcodeByName("i32.store")?.immediates)
        assertEquals("conversion", WasmOpcodes.getOpcode(0xBF)?.category)
        assertEquals("numeric_f64", WasmOpcodes.getOpcodeByName("f64.sqrt")?.category)
    }
}
