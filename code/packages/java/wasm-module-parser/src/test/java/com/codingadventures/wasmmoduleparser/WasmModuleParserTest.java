package com.codingadventures.wasmmoduleparser;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.codingadventures.wasmleb128.WasmLeb128;
import com.codingadventures.wasmtypes.WasmModule;
import com.codingadventures.wasmtypes.WasmTypes.ExternalKind;
import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import org.junit.jupiter.api.Test;

class WasmModuleParserTest {
    private static final byte[] WASM_HEADER = new byte[]{0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00};
    private static final int I32 = 0x7F;
    private static final int FUNCREF = 0x70;

    private final WasmModuleParser parser = new WasmModuleParser();

    @Test
    void exposesVersion() {
        assertEquals("0.1.0", WasmModuleParser.VERSION);
    }

    @Test
    void parsesMinimalModule() {
        WasmModule module = parser.parse(WASM_HEADER);

        assertEquals(0, module.types.size());
        assertEquals(0, module.imports.size());
        assertEquals(0, module.functions.size());
        assertEquals(0, module.tables.size());
        assertEquals(0, module.memories.size());
        assertEquals(0, module.globals.size());
        assertEquals(0, module.exports.size());
        assertNull(module.start);
        assertEquals(0, module.elements.size());
        assertEquals(0, module.code.size());
        assertEquals(0, module.data.size());
        assertEquals(0, module.customs.size());
    }

    @Test
    void parsesTypeFunctionExportAndCodeSections() {
        byte[] typeSection = makeSection(1, new byte[]{1, 0x60, 2, (byte) I32, (byte) I32, 1, (byte) I32});
        byte[] functionSection = makeSection(3, new byte[]{1, 0});
        byte[] exportSection = makeSection(7, concat(new byte[]{1}, makeString("main"), new byte[]{0x00, 0x00}));
        byte[] codeBody = new byte[]{1, 1, (byte) I32, 0x20, 0x00, 0x21, 0x02, 0x20, 0x02, 0x0B};
        byte[] codeSection = makeSection(10, concat(new byte[]{1}, encodeUnsigned(codeBody.length), codeBody));

        WasmModule module = parser.parse(makeWasm(typeSection, functionSection, exportSection, codeSection));

        assertEquals(1, module.types.size());
        assertEquals(Arrays.asList(ValueType.I32, ValueType.I32), module.types.get(0).params());
        assertEquals(Arrays.asList(ValueType.I32), module.types.get(0).results());
        assertEquals(Arrays.asList(0), module.functions);
        assertEquals(1, module.exports.size());
        assertEquals("main", module.exports.get(0).name());
        assertEquals(ExternalKind.FUNCTION, module.exports.get(0).kind());
        assertEquals(Arrays.asList(ValueType.I32), module.code.get(0).locals());
        assertArrayEquals(new byte[]{0x20, 0x00, 0x21, 0x02, 0x20, 0x02, 0x0B}, module.code.get(0).code());
    }

    @Test
    void parsesImportsMemoryGlobalsDataElementsStartAndCustomSection() {
        byte[] typeSection = makeSection(1, new byte[]{1, 0x60, 0, 0});
        byte[] importPayload = concat(
                new byte[]{1},
                makeString("env"),
                makeString("host_add"),
                new byte[]{0x00, 0x00}
        );
        byte[] memorySection = makeSection(5, new byte[]{1, 0x01, 0x01, 0x04});
        byte[] tableSection = makeSection(4, new byte[]{1, (byte) FUNCREF, 0x00, 0x05});
        byte[] globalSection = makeSection(6, new byte[]{1, (byte) I32, 0x00, 0x41, 0x2A, 0x0B});
        byte[] startSection = makeSection(8, new byte[]{0x01});
        byte[] elementSection = makeSection(9, new byte[]{1, 0x00, 0x41, 0x02, 0x0B, 0x02, 0x05, 0x06});
        byte[] dataSection = makeSection(11, concat(new byte[]{1, 0x00, 0x41, 0x03, 0x0B, 0x03}, "abc".getBytes(StandardCharsets.UTF_8)));
        byte[] customSection = makeSection(0, concat(makeString("name"), new byte[]{0x41, 0x42}));

        WasmModule module = parser.parse(makeWasm(
                typeSection,
                importPayload.length == 0 ? new byte[0] : makeSection(2, importPayload),
                tableSection,
                memorySection,
                globalSection,
                startSection,
                elementSection,
                dataSection,
                customSection
        ));

        assertEquals(1, module.imports.size());
        assertEquals("env", module.imports.get(0).moduleName());
        assertEquals("host_add", module.imports.get(0).name());
        assertEquals(ExternalKind.FUNCTION, module.imports.get(0).kind());
        assertEquals(1, module.tables.size());
        assertEquals(FUNCREF, module.tables.get(0).elementType());
        assertEquals(1, module.memories.size());
        assertEquals(1, module.memories.get(0).limits().min());
        assertEquals(Integer.valueOf(4), module.memories.get(0).limits().max());
        assertEquals(1, module.globals.size());
        assertEquals(ValueType.I32, module.globals.get(0).globalType().valueType());
        assertEquals(Integer.valueOf(1), module.start);
        assertEquals(Arrays.asList(5, 6), module.elements.get(0).functionIndices());
        assertArrayEquals("abc".getBytes(StandardCharsets.UTF_8), module.data.get(0).data());
        assertEquals(1, module.customs.size());
        assertEquals("name", module.customs.get(0).name());
    }

    @Test
    void rejectsInvalidMagic() {
        WasmModuleParser.WasmParseError error = assertThrows(
                WasmModuleParser.WasmParseError.class,
                () -> parser.parse(new byte[]{0x01, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00})
        );

        assertEquals(0, error.offset());
    }

    @Test
    void rejectsTruncatedSectionPayload() {
        byte[] broken = concat(WASM_HEADER, new byte[]{0x01, 0x05, 0x01, 0x60});

        WasmModuleParser.WasmParseError error = assertThrows(
                WasmModuleParser.WasmParseError.class,
                () -> parser.parse(broken)
        );

        assertEquals(10, error.offset());
    }

    @Test
    void rejectsOutOfOrderSections() {
        byte[] exportSection = makeSection(7, concat(new byte[]{1}, makeString("main"), new byte[]{0x00, 0x00}));
        byte[] typeSection = makeSection(1, new byte[]{1, 0x60, 0, 0});

        WasmModuleParser.WasmParseError error = assertThrows(
                WasmModuleParser.WasmParseError.class,
                () -> parser.parse(makeWasm(exportSection, typeSection))
        );

        assertEquals(18, error.offset());
    }

    private static byte[] makeWasm(byte[]... sections) {
        return concat(WASM_HEADER, concat(sections));
    }

    private static byte[] makeSection(int id, byte[] payload) {
        return concat(new byte[]{(byte) id}, encodeUnsigned(payload.length), payload);
    }

    private static byte[] makeString(String value) {
        byte[] encoded = value.getBytes(StandardCharsets.UTF_8);
        return concat(encodeUnsigned(encoded.length), encoded);
    }

    private static byte[] encodeUnsigned(int value) {
        return WasmLeb128.encodeUnsigned(value);
    }

    private static byte[] concat(byte[]... parts) {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        for (byte[] part : parts) {
            output.writeBytes(part);
        }
        return output.toByteArray();
    }
}
