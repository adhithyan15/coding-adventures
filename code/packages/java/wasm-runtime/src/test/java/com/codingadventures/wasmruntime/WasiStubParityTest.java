package com.codingadventures.wasmruntime;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;

import com.codingadventures.wasmexecution.WasmExecution;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class WasiStubParityTest {
    @Test
    void supportsArgsEnvClockRandomYieldAndFallbacks() {
        Map<String, String> env = new LinkedHashMap<>();
        env.put("HOME", "/tmp");
        env.put("TERM", "xterm");
        WasmRuntime.WasiStub wasi = new WasmRuntime.WasiStub(
                new WasmRuntime.WasiConfig(
                        count -> new byte[0],
                        List.of("prog", "--flag"),
                        env,
                        text -> {},
                        text -> {},
                        new WasmRuntime.WasiClock() {
                            @Override
                            public long realtimeNs() {
                                return 123L;
                            }

                            @Override
                            public long monotonicNs() {
                                return 456L;
                            }

                            @Override
                            public long resolutionNs(int clockId) {
                                return 789L + clockId;
                            }
                        },
                        buffer -> {
                            for (int index = 0; index < buffer.length; index++) {
                                buffer[index] = (byte) (index + 1);
                            }
                        }
                )
        );
        WasmExecution.LinearMemory memory = new WasmExecution.LinearMemory(1);
        wasi.setMemory(memory);

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "args_sizes_get").call(List.of(
                WasmExecution.i32(0),
                WasmExecution.i32(4)
        )));
        assertEquals(2, memory.loadI32(0));
        assertEquals(12, memory.loadI32(4));

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "args_get").call(List.of(
                WasmExecution.i32(8),
                WasmExecution.i32(32)
        )));
        assertEquals("prog", readCString(memory, memory.loadI32(8)));
        assertEquals("--flag", readCString(memory, memory.loadI32(12)));

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "environ_sizes_get").call(List.of(
                WasmExecution.i32(64),
                WasmExecution.i32(68)
        )));
        assertEquals(2, memory.loadI32(64));
        assertEquals("HOME=/tmp".getBytes(StandardCharsets.UTF_8).length + "TERM=xterm".getBytes(StandardCharsets.UTF_8).length + 2, memory.loadI32(68));

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "environ_get").call(List.of(
                WasmExecution.i32(72),
                WasmExecution.i32(96)
        )));
        assertEquals("HOME=/tmp", readCString(memory, memory.loadI32(72)));
        assertEquals("TERM=xterm", readCString(memory, memory.loadI32(76)));

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "clock_res_get").call(List.of(
                WasmExecution.i32(1),
                WasmExecution.i32(128)
        )));
        assertEquals(790L, memory.loadI64(128));

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get").call(List.of(
                WasmExecution.i32(0),
                WasmExecution.i64(0L),
                WasmExecution.i32(136)
        )));
        assertEquals(123L, memory.loadI64(136));

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get").call(List.of(
                WasmExecution.i32(1),
                WasmExecution.i64(0L),
                WasmExecution.i32(144)
        )));
        assertEquals(456L, memory.loadI64(144));

        assertEquals(List.of(WasmExecution.i32(28)), wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get").call(List.of(
                WasmExecution.i32(99),
                WasmExecution.i64(0L),
                WasmExecution.i32(152)
        )));

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "random_get").call(List.of(
                WasmExecution.i32(160),
                WasmExecution.i32(4)
        )));
        assertArrayEquals(new byte[]{1, 2, 3, 4}, readBytes(memory, 160, 4));

        assertEquals(List.of(WasmExecution.i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "sched_yield").call(List.of()));
        assertEquals(List.of(WasmExecution.i32(52)), wasi.resolveFunction("wasi_snapshot_preview1", "path_open").call(List.of()));
    }

    @Test
    void returnsExpectedErrorsWithoutMemoryOrForBadFd() {
        WasmRuntime.WasiStub wasi = new WasmRuntime.WasiStub();
        assertEquals(List.of(WasmExecution.i32(52)), wasi.resolveFunction("wasi_snapshot_preview1", "args_sizes_get").call(List.of(
                WasmExecution.i32(0),
                WasmExecution.i32(4)
        )));

        WasmExecution.LinearMemory memory = new WasmExecution.LinearMemory(1);
        wasi.setMemory(memory);
        assertEquals(List.of(WasmExecution.i32(8)), wasi.resolveFunction("wasi_snapshot_preview1", "fd_read").call(List.of(
                WasmExecution.i32(1),
                WasmExecution.i32(0),
                WasmExecution.i32(0),
                WasmExecution.i32(4)
        )));
    }

    private static String readCString(WasmExecution.LinearMemory memory, int address) {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        int cursor = address;
        while (memory.loadI32_8u(cursor) != 0) {
            output.write(memory.loadI32_8u(cursor));
            cursor++;
        }
        return new String(output.toByteArray(), StandardCharsets.UTF_8);
    }

    private static byte[] readBytes(WasmExecution.LinearMemory memory, int address, int length) {
        byte[] bytes = new byte[length];
        for (int index = 0; index < length; index++) {
            bytes[index] = (byte) memory.loadI32_8u(address + index);
        }
        return bytes;
    }
}
