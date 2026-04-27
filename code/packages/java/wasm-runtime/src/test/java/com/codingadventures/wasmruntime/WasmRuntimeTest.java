package com.codingadventures.wasmruntime;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.wasmexecution.WasmExecution;
import com.codingadventures.wasmleb128.WasmLeb128;
import com.codingadventures.wasmtypes.WasmModule;
import com.codingadventures.wasmtypes.WasmTypes;
import com.codingadventures.wasmtypes.WasmTypes.ExternalKind;
import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.jupiter.api.Test;

class WasmRuntimeTest {
    @Test
    void loadsMinimalModule() {
        WasmRuntime runtime = new WasmRuntime();
        WasmModule module = runtime.load(buildMinimalWasm());

        assertEquals(1, module.types.size());
    }

    @Test
    void validatesParsedModule() {
        WasmRuntime runtime = new WasmRuntime();
        WasmModule module = runtime.load(buildMinimalWasm());

        assertSame(module, runtime.validate(module).module());
    }

    @Test
    void instantiatesMemoryAndDataSegments() {
        WasmRuntime runtime = new WasmRuntime();
        WasmModule module = new WasmModule();
        module.memories.add(new WasmTypes.MemoryType(new WasmTypes.Limits(1, null)));
        module.data.add(new WasmTypes.DataSegment(0, i32ConstExpr(256), "Hi".getBytes(StandardCharsets.UTF_8)));

        WasmRuntime.WasmInstance instance = runtime.instantiate(module);

        assertNotNull(instance.memory);
        assertEquals('H', instance.memory.loadI32_8u(256));
        assertEquals('i', instance.memory.loadI32_8u(257));
    }

    @Test
    void resolvesFunctionImportsAndCallsExports() {
        WasmExecution.HostInterface host = new WasmExecution.HostInterface() {
            @Override
            public WasmExecution.HostFunction resolveFunction(String moduleName, String name) {
                if ("env".equals(moduleName) && "double".equals(name)) {
                    return new WasmExecution.HostFunction() {
                        @Override
                        public WasmTypes.FuncType type() {
                            return WasmTypes.makeFuncType(List.of(ValueType.I32), List.of(ValueType.I32));
                        }

                        @Override
                        public List<WasmExecution.WasmValue> call(List<WasmExecution.WasmValue> args) {
                            return List.of(WasmExecution.i32(((Number) args.get(0).value()).intValue() * 2));
                        }
                    };
                }
                return null;
            }
        };

        WasmRuntime runtime = new WasmRuntime(host);
        WasmModule module = new WasmModule();
        module.types.add(WasmTypes.makeFuncType(List.of(ValueType.I32), List.of(ValueType.I32)));
        module.imports.add(new WasmTypes.Import("env", "double", ExternalKind.FUNCTION, 0));
        module.exports.add(new WasmTypes.Export("double", ExternalKind.FUNCTION, 0));

        WasmRuntime.WasmInstance instance = runtime.instantiate(module);

        assertNotNull(instance.hostFunctions.get(0));
        assertEquals(List.of(42), runtime.call(instance, "double", List.of(21)));
    }

    @Test
    void loadAndRunExecutesExportedFunction() {
        WasmRuntime runtime = new WasmRuntime();
        assertEquals(List.of(42), runtime.loadAndRun(buildAnswerWasm(), "answer", List.of()));
    }

    @Test
    void startFunctionRunsDuringInstantiation() {
        WasmRuntime runtime = new WasmRuntime();
        WasmModule module = new WasmModule();
        module.types.add(WasmTypes.makeFuncType(List.of(), List.of()));
        module.functions.add(0);
        module.code.add(new WasmTypes.FunctionBody(List.of(), concat(new byte[]{0x41}, WasmLeb128.encodeSigned(99), new byte[]{0x24, 0x00, 0x0B})));
        module.globals.add(new WasmTypes.Global(new WasmTypes.GlobalType(ValueType.I32, true), i32ConstExpr(0)));
        module.start = 0;

        WasmRuntime.WasmInstance instance = runtime.instantiate(module);

        assertEquals(99, instance.globals.get(0).value());
    }

    @Test
    void throwsForMissingOrNonFunctionExport() {
        WasmRuntime runtime = new WasmRuntime();
        WasmRuntime.WasmInstance emptyInstance = runtime.instantiate(new WasmModule());

        assertThrows(WasmExecution.TrapError.class, () -> runtime.call(emptyInstance, "missing", List.of()));

        WasmModule module = new WasmModule();
        module.memories.add(new WasmTypes.MemoryType(new WasmTypes.Limits(1, null)));
        module.exports.add(new WasmTypes.Export("memory", ExternalKind.MEMORY, 0));
        WasmRuntime.WasmInstance instance = runtime.instantiate(module);

        assertThrows(WasmExecution.TrapError.class, () -> runtime.call(instance, "memory", List.of()));
    }

    @Test
    void preservesHostReferenceOnInstance() {
        WasmRuntime.WasiStub wasi = new WasmRuntime.WasiStub();
        WasmRuntime runtime = new WasmRuntime(wasi);
        WasmRuntime.WasmInstance instance = runtime.instantiate(new WasmModule());

        assertSame(wasi, instance.host);
        assertNull(new WasmRuntime().instantiate(new WasmModule()).host);
    }

    @Test
    void instantiatesImportedMemoryTableAndGlobal() {
        WasmExecution.LinearMemory importedMemory = new WasmExecution.LinearMemory(1);
        WasmExecution.Table importedTable = new WasmExecution.Table(1);
        WasmExecution.ImportedGlobal importedGlobal = new WasmExecution.ImportedGlobal(
                new WasmTypes.GlobalType(ValueType.I32, false),
                WasmExecution.i32(7)
        );
        WasmExecution.HostInterface host = new WasmExecution.HostInterface() {
            @Override
            public WasmExecution.LinearMemory resolveMemory(String moduleName, String name) {
                return importedMemory;
            }

            @Override
            public WasmExecution.Table resolveTable(String moduleName, String name) {
                return importedTable;
            }

            @Override
            public WasmExecution.ImportedGlobal resolveGlobal(String moduleName, String name) {
                return importedGlobal;
            }
        };

        WasmModule module = new WasmModule();
        module.imports.add(new WasmTypes.Import("env", "memory", ExternalKind.MEMORY, null));
        module.imports.add(new WasmTypes.Import("env", "table", ExternalKind.TABLE, null));
        module.imports.add(new WasmTypes.Import("env", "global", ExternalKind.GLOBAL, null));

        WasmRuntime.WasmInstance instance = new WasmRuntime(host).instantiate(module);

        assertSame(importedMemory, instance.memory);
        assertSame(importedTable, instance.tables.get(0));
        assertEquals(WasmExecution.i32(7), instance.globals.get(0));
    }

    @Test
    void wasiFdWriteCapturesStdoutAndByteCount() {
        AtomicReference<String> stdout = new AtomicReference<>("");
        WasmRuntime.WasiStub wasi = new WasmRuntime.WasiStub(
                new WasmRuntime.WasiConfig(
                        count -> new byte[0],
                        List.of("prog"),
                        Map.of("HOME", "/tmp"),
                        stdout::set,
                        text -> {},
                        new WasmRuntime.SystemClock(),
                        new WasmRuntime.SystemRandom()
                )
        );
        WasmExecution.LinearMemory memory = new WasmExecution.LinearMemory(1);
        wasi.setMemory(memory);

        byte[] text = "Hello".getBytes(StandardCharsets.UTF_8);
        int iovsPtr = 0;
        int nwrittenPtr = 64;
        int bufPtr = 128;
        memory.storeBytes(bufPtr, text);
        memory.storeI32(iovsPtr, bufPtr);
        memory.storeI32(iovsPtr + 4, text.length);

        WasmExecution.HostFunction fdWrite = wasi.resolveFunction("wasi_snapshot_preview1", "fd_write");
        List<WasmExecution.WasmValue> result = fdWrite.call(List.of(
                WasmExecution.i32(1),
                WasmExecution.i32(iovsPtr),
                WasmExecution.i32(1),
                WasmExecution.i32(nwrittenPtr)
        ));

        assertEquals(List.of(WasmExecution.i32(0)), result);
        assertEquals("Hello", stdout.get());
        assertEquals(5, memory.loadI32(nwrittenPtr));
    }

    @Test
    void wasiFdReadCopiesStdinIntoMemory() {
        WasmRuntime.WasiStub wasi = new WasmRuntime.WasiStub(
                new WasmRuntime.WasiConfig(
                        count -> "abc",
                        List.of(),
                        Map.of(),
                        text -> {},
                        text -> {},
                        new WasmRuntime.SystemClock(),
                        new WasmRuntime.SystemRandom()
                )
        );
        WasmExecution.LinearMemory memory = new WasmExecution.LinearMemory(1);
        wasi.setMemory(memory);

        int iovsPtr = 0;
        int nreadPtr = 64;
        int bufPtr = 128;
        memory.storeI32(iovsPtr, bufPtr);
        memory.storeI32(iovsPtr + 4, 8);

        WasmExecution.HostFunction fdRead = wasi.resolveFunction("wasi_snapshot_preview1", "fd_read");
        List<WasmExecution.WasmValue> result = fdRead.call(List.of(
                WasmExecution.i32(0),
                WasmExecution.i32(iovsPtr),
                WasmExecution.i32(1),
                WasmExecution.i32(nreadPtr)
        ));

        assertEquals(List.of(WasmExecution.i32(0)), result);
        assertEquals(3, memory.loadI32(nreadPtr));
        assertEquals('a', memory.loadI32_8u(bufPtr));
        assertEquals('b', memory.loadI32_8u(bufPtr + 1));
        assertEquals('c', memory.loadI32_8u(bufPtr + 2));
    }

    @Test
    void procExitErrorCarriesExitCode() {
        WasmRuntime.ProcExitError error = assertThrows(
                WasmRuntime.ProcExitError.class,
                () -> new WasmRuntime.WasiStub()
                        .resolveFunction("wasi_snapshot_preview1", "proc_exit")
                        .call(List.of(WasmExecution.i32(42)))
        );

        assertEquals("proc_exit(42)", error.getMessage());
        assertEquals(42, error.exitCode);
    }

    @Test
    void systemClockAndRandomProvideConcreteImplementations() {
        WasmRuntime.SystemClock clock = new WasmRuntime.SystemClock();
        long realtime = clock.realtimeNs();
        long monotonic = clock.monotonicNs();

        assertTrue(realtime > 0);
        assertTrue(monotonic > 0);
        assertEquals(1_000_000L, clock.resolutionNs(0));

        byte[] bytes = new byte[8];
        new WasmRuntime.SystemRandom().fillBytes(bytes);
        assertEquals(8, bytes.length);
        assertFalse(java.util.Arrays.equals(new byte[8], bytes));
    }

    private static byte[] i32ConstExpr(int value) {
        return concat(new byte[]{0x41}, WasmLeb128.encodeSigned(value), new byte[]{0x0B});
    }

    private static byte[] buildMinimalWasm() {
        return concat(
                new byte[]{0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00},
                makeSection(1, new byte[]{0x01, 0x60, 0x00, 0x00})
        );
    }

    private static byte[] buildAnswerWasm() {
        byte[] typePayload = new byte[]{0x01, 0x60, 0x00, 0x01, 0x7F};
        byte[] functionPayload = new byte[]{0x01, 0x00};
        byte[] exportPayload = concat(
                new byte[]{0x01},
                makeString("answer"),
                new byte[]{0x00, 0x00}
        );
        byte[] bodyPayload = new byte[]{0x00, 0x41, 0x2A, 0x0B};
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
