package com.codingadventures.wasmvalidator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertSame;

import com.codingadventures.wasmtypes.WasmModule;
import com.codingadventures.wasmtypes.WasmTypes;
import com.codingadventures.wasmtypes.WasmTypes.ExternalKind;
import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import org.junit.jupiter.api.Test;

class WasmValidatorTest {
    @Test
    void exposesVersion() {
        assertEquals("0.1.0", WasmValidator.VERSION);
    }

    @Test
    void returnsValidatedModuleWrapper() {
        WasmModule module = new WasmModule();
        WasmValidator.ValidatedModule validated = WasmValidator.validate(module);

        assertSame(module, validated.module());
        assertEquals(0, validated.funcTypes().size());
    }

    @Test
    void rejectsMultipleMemories() {
        WasmModule module = new WasmModule();
        module.memories.add(new WasmTypes.MemoryType(new WasmTypes.Limits(1, null)));
        module.memories.add(new WasmTypes.MemoryType(new WasmTypes.Limits(1, null)));

        WasmValidator.ValidationError error = assertThrows(
                WasmValidator.ValidationError.class,
                () -> WasmValidator.validateStructure(module)
        );

        assertEquals(WasmValidator.ValidationErrorKind.MULTIPLE_MEMORIES, error.kind());
    }

    @Test
    void allowsImportedGlobalInConstExpr() {
        WasmModule module = new WasmModule();
        module.imports.add(new WasmTypes.Import(
                "env",
                "seed",
                ExternalKind.GLOBAL,
                new WasmTypes.GlobalType(ValueType.I32, false)
        ));
        module.globals.add(new WasmTypes.Global(
                new WasmTypes.GlobalType(ValueType.I32, false),
                new byte[]{0x23, 0x00, 0x0B}
        ));

        WasmValidator.IndexSpaces indexSpaces = WasmValidator.validateStructure(module);

        assertEquals(2, indexSpaces.globalTypes().size());
    }
}
