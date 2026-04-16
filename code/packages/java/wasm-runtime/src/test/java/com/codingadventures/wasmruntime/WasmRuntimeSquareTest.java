package com.codingadventures.wasmruntime;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.wasmleb128.WasmLeb128;
import com.codingadventures.wasmtypes.WasmModule;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import org.junit.jupiter.api.Test;

class WasmRuntimeSquareTest {
    @Test
    void loadAndRunExecutesSquareModule() {
        WasmRuntime runtime = new WasmRuntime();
        assertEquals(List.of(25), runtime.loadAndRun(buildSquareWasm(), "square", List.of(5)));
        assertEquals(List.of(0), runtime.loadAndRun(buildSquareWasm(), "square", List.of(0)));
        assertEquals(List.of(9), runtime.loadAndRun(buildSquareWasm(), "square", List.of(-3)));
    }

    @Test
    void squareModuleSupportsStepByStepFlow() {
        WasmRuntime runtime = new WasmRuntime();
        byte[] wasm = buildSquareWasm();

        WasmModule module = runtime.load(wasm);
        assertEquals(1, module.types.size());
        assertEquals(1, module.functions.size());
        assertEquals(1, module.exports.size());

        assertEquals(module, runtime.validate(module).module());

        WasmRuntime.WasmInstance instance = runtime.instantiate(module);
        assertTrue(instance.exports.containsKey("square"));
        assertEquals(List.of(49), runtime.call(instance, "square", List.of(7)));
    }

    private static byte[] buildSquareWasm() {
        byte[] typePayload = new byte[]{0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F};
        byte[] functionPayload = new byte[]{0x01, 0x00};
        byte[] exportPayload = concat(new byte[]{0x01}, makeString("square"), new byte[]{0x00, 0x00});
        byte[] bodyPayload = new byte[]{0x00, 0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B};
        byte[] codePayload = concat(new byte[]{0x01}, WasmLeb128.encodeUnsigned(bodyPayload.length), bodyPayload);

        return concat(
                new byte[]{0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00},
                makeSection(1, typePayload),
                makeSection(3, functionPayload),
                makeSection(7, exportPayload),
                makeSection(10, codePayload)
        );
    }

    private static byte[] makeSection(int id, byte[] payload) {
        return concat(new byte[]{(byte) id}, WasmLeb128.encodeUnsigned(payload.length), payload);
    }

    private static byte[] makeString(String value) {
        byte[] encoded = value.getBytes(StandardCharsets.UTF_8);
        return concat(WasmLeb128.encodeUnsigned(encoded.length), encoded);
    }

    private static byte[] concat(byte[]... parts) {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        for (byte[] part : parts) {
            output.writeBytes(part);
        }
        return output.toByteArray();
    }
}
