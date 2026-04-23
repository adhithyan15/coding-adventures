package com.codingadventures.wasmtypes;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

class WasmTypesTest {
    @Test
    void buildsFunctionTypesAndModuleContainers() {
        WasmTypes.FuncType signature = WasmTypes.makeFuncType(
                List.of(WasmTypes.ValueType.I32),
                List.of(WasmTypes.ValueType.I64)
        );

        WasmModule module = new WasmModule();
        module.types.add(signature);
        module.functions.add(0);

        assertEquals(1, module.types.size());
        assertEquals(WasmTypes.ValueType.I32, signature.params().get(0));
        assertEquals(WasmTypes.ValueType.I64, signature.results().get(0));
        assertNull(module.start);
    }

    @Test
    void preservesBytePayloadsInStructuralTypes() {
        WasmTypes.Global global = new WasmTypes.Global(
                new WasmTypes.GlobalType(WasmTypes.ValueType.I32, false),
                new byte[]{0x41, 0x2A, 0x0B}
        );
        WasmTypes.DataSegment segment = new WasmTypes.DataSegment(0, new byte[]{0x41, 0x00, 0x0B}, new byte[]{1, 2, 3});

        assertEquals(3, global.initExpr().length);
        assertEquals(3, segment.data().length);
    }
}
