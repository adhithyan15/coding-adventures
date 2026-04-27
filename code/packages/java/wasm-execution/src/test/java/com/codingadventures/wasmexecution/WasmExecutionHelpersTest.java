package com.codingadventures.wasmexecution;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.List;
import org.junit.jupiter.api.Test;

class WasmExecutionHelpersTest {
    @Test
    void defaultValueCoercionAndConstExprsWork() {
        assertEquals(0, WasmExecution.defaultValue(ValueType.I32).value());
        assertEquals(0L, WasmExecution.defaultValue(ValueType.I64).value());
        assertEquals(0.0f, WasmExecution.defaultValue(ValueType.F32).value());
        assertEquals(0.0d, WasmExecution.defaultValue(ValueType.F64).value());

        assertEquals(WasmExecution.i32(7), WasmExecution.coerceValue(7, ValueType.I32));
        assertEquals(WasmExecution.i64(9L), WasmExecution.coerceValue(9L, ValueType.I64));
        assertEquals(3.5f, WasmExecution.unwrapValue(WasmExecution.f32(3.5f)));
        assertThrows(WasmExecution.TrapError.class, () -> WasmExecution.coerceValue("oops", ValueType.I32));
        assertThrows(WasmExecution.TrapError.class, () -> WasmExecution.coerceValue(WasmExecution.i64(1), ValueType.I32));

        assertEquals(12, WasmExecution.evaluateConstExpr(concat(new byte[]{0x41}, WasmExecutionTestSupport.encodeSigned32(12), new byte[]{0x0B}), List.of()).value());
        assertEquals(99L, WasmExecution.evaluateConstExpr(concat(new byte[]{0x42}, WasmExecutionTestSupport.encodeSigned64(99), new byte[]{0x0B}), List.of()).value());
        assertEquals(2.5f, WasmExecution.evaluateConstExpr(concat(new byte[]{0x43}, f32Bytes(2.5f), new byte[]{0x0B}), List.of()).value());
        assertEquals(6.25d, WasmExecution.evaluateConstExpr(concat(new byte[]{0x44}, f64Bytes(6.25d), new byte[]{0x0B}), List.of()).value());
        assertEquals(WasmExecution.i32(33), WasmExecution.evaluateConstExpr(new byte[]{0x23, 0x00, 0x0B}, List.of(WasmExecution.i32(33))));
        assertThrows(WasmExecution.TrapError.class, () -> WasmExecution.evaluateConstExpr(new byte[]{0x41, 0x00}, List.of()));
    }

    @Test
    void linearMemoryAndTableHelpersHandleEdgeCases() {
        WasmExecution.LinearMemory memory = new WasmExecution.LinearMemory(1, 2);
        assertEquals(1, memory.size());
        assertEquals(WasmExecution.PAGE_SIZE, memory.byteLength());

        memory.storeI32(0, 0x11223344);
        memory.storeI64(8, 0x1020_3040_5060_7080L);
        memory.storeF32(24, 1.5f);
        memory.storeF64(32, 9.25d);
        memory.storeI32_8(48, 0xFF);
        memory.storeI32_16(50, 0xFFFF);
        memory.storeI64_8(56, 0x1FF);
        memory.storeI64_16(58, 0x1FFFF);
        memory.storeI64_32(60, 0x1FFFFFFFFL);
        memory.writeBytes(72, new byte[]{1, 2, 3});

        assertEquals(0x11223344, memory.loadI32(0));
        assertEquals(0x1020_3040_5060_7080L, memory.loadI64(8));
        assertEquals(1.5f, memory.loadF32(24));
        assertEquals(9.25d, memory.loadF64(32));
        assertEquals(-1, memory.loadI32_8s(48));
        assertEquals(255, memory.loadI32_8u(48));
        assertEquals(-1, memory.loadI32_16s(50));
        assertEquals(65535, memory.loadI32_16u(50));
        assertEquals(-1L, memory.loadI64_8s(56));
        assertEquals(255L, memory.loadI64_8u(56));
        assertEquals(-1L, memory.loadI64_16s(58));
        assertEquals(65535L, memory.loadI64_16u(58));
        assertEquals(-1L, memory.loadI64_32s(60));
        assertEquals(4_294_967_295L, memory.loadI64_32u(60));
        assertArrayEquals(new byte[]{1, 2, 3}, new byte[]{(byte) memory.loadI32_8u(72), (byte) memory.loadI32_8u(73), (byte) memory.loadI32_8u(74)});
        assertEquals(1, memory.grow(1));
        assertEquals(-1, memory.grow(1));
        assertThrows(WasmExecution.TrapError.class, () -> memory.loadI32(WasmExecution.PAGE_SIZE * 2));

        WasmExecution.Table table = new WasmExecution.Table(1, 2);
        table.set(0, 7);
        assertEquals(7, table.get(0));
        assertEquals(1, table.grow(1));
        assertEquals(-1, table.grow(1));
        assertThrows(WasmExecution.TrapError.class, () -> table.get(5));
    }

    private static byte[] f32Bytes(float value) {
        return ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putFloat(value).array();
    }

    private static byte[] f64Bytes(double value) {
        return ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putDouble(value).array();
    }

    private static byte[] concat(byte[]... parts) {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        for (byte[] part : parts) {
            output.writeBytes(part);
        }
        return output.toByteArray();
    }
}
