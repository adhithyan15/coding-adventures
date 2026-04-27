package com.codingadventures.wasmexecution

import com.codingadventures.wasmtypes.ValueType
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class WasmExecutionHelpersTest {
    @Test
    fun defaultValueCoercionAndConstExprsWork() {
        assertEquals(0, defaultValue(ValueType.I32).value)
        assertEquals(0L, defaultValue(ValueType.I64).value)
        assertEquals(0f, defaultValue(ValueType.F32).value)
        assertEquals(0.0, defaultValue(ValueType.F64).value)

        assertEquals(i32(7), coerceValue(7, ValueType.I32))
        assertEquals(i64(9L), coerceValue(9L, ValueType.I64))
        assertEquals(3.5f, unwrapValue(f32(3.5f)))
        assertFailsWith<TrapError> { coerceValue("oops", ValueType.I32) }
        assertFailsWith<TrapError> { coerceValue(i64(1), ValueType.I32) }

        assertEquals(12, evaluateConstExpr(concat(byteArrayOf(0x41), WasmExecutionTestSupport.encodeSigned32(12), byteArrayOf(0x0B)), emptyList()).value)
        assertEquals(99L, evaluateConstExpr(concat(byteArrayOf(0x42), WasmExecutionTestSupport.encodeSigned64(99), byteArrayOf(0x0B)), emptyList()).value)
        assertEquals(2.5f, evaluateConstExpr(concat(byteArrayOf(0x43), f32Bytes(2.5f), byteArrayOf(0x0B)), emptyList()).value)
        assertEquals(6.25, evaluateConstExpr(concat(byteArrayOf(0x44), f64Bytes(6.25), byteArrayOf(0x0B)), emptyList()).value)
        assertEquals(i32(33), evaluateConstExpr(byteArrayOf(0x23, 0x00, 0x0B), listOf(i32(33))))
        assertFailsWith<TrapError> { evaluateConstExpr(byteArrayOf(0x41, 0x00), emptyList()) }
    }

    @Test
    fun linearMemoryAndTableHelpersHandleEdgeCases() {
        val memory = LinearMemory(1, 2)
        assertEquals(1, memory.size())
        assertEquals(PAGE_SIZE, memory.byteLength())

        memory.storeI32(0, 0x11223344)
        memory.storeI64(8, 0x1020_3040_5060_7080L)
        memory.storeF32(24, 1.5f)
        memory.storeF64(32, 9.25)
        memory.storeI32_8(48, 0xFF)
        memory.storeI32_16(50, 0xFFFF)
        memory.storeI64_8(56, 0x1FF)
        memory.storeI64_16(58, 0x1FFFF)
        memory.storeI64_32(60, 0x1FFFFFFFFL)
        memory.writeBytes(72, byteArrayOf(1, 2, 3))

        assertEquals(0x11223344, memory.loadI32(0))
        assertEquals(0x1020_3040_5060_7080L, memory.loadI64(8))
        assertEquals(1.5f, memory.loadF32(24))
        assertEquals(9.25, memory.loadF64(32))
        assertEquals(-1, memory.loadI32_8s(48))
        assertEquals(255, memory.loadI32_8u(48))
        assertEquals(-1, memory.loadI32_16s(50))
        assertEquals(65535, memory.loadI32_16u(50))
        assertEquals(-1L, memory.loadI64_8s(56))
        assertEquals(255L, memory.loadI64_8u(56))
        assertEquals(-1L, memory.loadI64_16s(58))
        assertEquals(65535L, memory.loadI64_16u(58))
        assertEquals(-1L, memory.loadI64_32s(60))
        assertEquals(4_294_967_295L, memory.loadI64_32u(60))
        assertContentEquals(byteArrayOf(1, 2, 3), byteArrayOf(memory.loadI32_8u(72).toByte(), memory.loadI32_8u(73).toByte(), memory.loadI32_8u(74).toByte()))
        assertEquals(1, memory.grow(1))
        assertEquals(-1, memory.grow(1))
        assertFailsWith<TrapError> { memory.loadI32(PAGE_SIZE * 2) }

        val table = Table(1, 2)
        table.set(0, 7)
        assertEquals(7, table.get(0))
        assertEquals(1, table.grow(1))
        assertEquals(-1, table.grow(1))
        assertFailsWith<TrapError> { table.get(5) }
    }

    private fun f32Bytes(value: Float): ByteArray = ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putFloat(value).array()

    private fun f64Bytes(value: Double): ByteArray = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putDouble(value).array()

    private fun concat(vararg parts: ByteArray): ByteArray = parts.fold(ByteArray(0)) { acc, part -> acc + part }
}
