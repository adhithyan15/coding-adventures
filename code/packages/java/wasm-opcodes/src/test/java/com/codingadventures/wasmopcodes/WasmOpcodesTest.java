package com.codingadventures.wasmopcodes;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import org.junit.jupiter.api.Test;

class WasmOpcodesTest {
    @Test
    void exposesVersion() {
        assertEquals("0.1.0", WasmOpcodes.VERSION);
    }

    @Test
    void loadsFullOpcodeTable() {
        assertTrue(WasmOpcodes.OPCODES.size() >= 172);
        assertEquals(WasmOpcodes.OPCODES.size(), WasmOpcodes.OPCODES_BY_NAME.size());
    }

    @Test
    void looksUpOpcodeByByte() {
        WasmOpcodes.OpcodeInfo info = WasmOpcodes.getOpcode(0x6A);

        assertNotNull(info);
        assertEquals("i32.add", info.name());
        assertEquals("numeric_i32", info.category());
        assertEquals(2, info.stackPop());
        assertEquals(1, info.stackPush());
    }

    @Test
    void looksUpOpcodeByName() {
        WasmOpcodes.OpcodeInfo info = WasmOpcodes.getOpcodeByName("call_indirect");

        assertNotNull(info);
        assertEquals(0x11, info.opcode());
        assertEquals(List.of("typeidx", "tableidx"), info.immediates());
    }

    @Test
    void returnsNullForUnknownOpcodeOrName() {
        assertNull(WasmOpcodes.getOpcode(0x06));
        assertNull(WasmOpcodes.getOpcode(0xFF));
        assertNull(WasmOpcodes.getOpcodeByName(""));
        assertNull(WasmOpcodes.getOpcodeByName("i32.foo"));
    }

    @Test
    void preservesImmediateAndCategoryMetadata() {
        assertEquals(List.of("i32"), WasmOpcodes.getOpcodeByName("i32.const").immediates());
        assertEquals(List.of("memarg"), WasmOpcodes.getOpcodeByName("i32.store").immediates());
        assertEquals("conversion", WasmOpcodes.getOpcode(0xBF).category());
        assertEquals("numeric_f64", WasmOpcodes.getOpcodeByName("f64.sqrt").category());
    }
}
