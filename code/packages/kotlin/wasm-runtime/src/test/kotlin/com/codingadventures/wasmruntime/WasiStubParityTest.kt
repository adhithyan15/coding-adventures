package com.codingadventures.wasmruntime

import com.codingadventures.wasmexecution.LinearMemory
import com.codingadventures.wasmexecution.WasmValue
import com.codingadventures.wasmexecution.i32
import com.codingadventures.wasmexecution.i64
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals

class WasiStubParityTest {
    @Test
    fun supportsArgsEnvClockRandomYieldAndFallbacks() {
        val wasi =
            WasiStub(
                WasiConfig(
                    stdin = WasiStdin { ByteArray(0) },
                    args = listOf("prog", "--flag"),
                    env = mapOf("HOME" to "/tmp", "TERM" to "xterm"),
                    stdout = {},
                    stderr = {},
                    clock =
                        object : WasiClock {
                            override fun realtimeNs(): Long = 123L

                            override fun monotonicNs(): Long = 456L

                            override fun resolutionNs(clockId: Int): Long = 789L + clockId
                        },
                    random =
                        object : WasiRandom {
                            override fun fillBytes(buffer: ByteArray) {
                                buffer.indices.forEach { buffer[it] = (it + 1).toByte() }
                            }
                        },
                ),
            )
        val memory = LinearMemory(1)
        wasi.setMemory(memory)

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "args_sizes_get")!!.call(listOf(i32(0), i32(4))))
        assertEquals(2, memory.loadI32(0))
        assertEquals(12, memory.loadI32(4))

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "args_get")!!.call(listOf(i32(8), i32(32))))
        assertEquals("prog", readCString(memory, memory.loadI32(8)))
        assertEquals("--flag", readCString(memory, memory.loadI32(12)))

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "environ_sizes_get")!!.call(listOf(i32(64), i32(68))))
        assertEquals(2, memory.loadI32(64))

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "environ_get")!!.call(listOf(i32(72), i32(96))))
        assertEquals("HOME=/tmp", readCString(memory, memory.loadI32(72)))
        assertEquals("TERM=xterm", readCString(memory, memory.loadI32(76)))

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "clock_res_get")!!.call(listOf(i32(1), i32(128))))
        assertEquals(790L, memory.loadI64(128))

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!!.call(listOf(i32(0), i64(0), i32(136))))
        assertEquals(123L, memory.loadI64(136))

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!!.call(listOf(i32(1), i64(0), i32(144))))
        assertEquals(456L, memory.loadI64(144))

        assertEquals(listOf(i32(28)), wasi.resolveFunction("wasi_snapshot_preview1", "clock_time_get")!!.call(listOf(i32(99), i64(0), i32(152))))

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "random_get")!!.call(listOf(i32(160), i32(4))))
        assertContentEquals(byteArrayOf(1, 2, 3, 4), readBytes(memory, 160, 4))

        assertEquals(listOf(i32(0)), wasi.resolveFunction("wasi_snapshot_preview1", "sched_yield")!!.call(emptyList()))
        assertEquals(listOf(i32(52)), wasi.resolveFunction("wasi_snapshot_preview1", "path_open")!!.call(emptyList()))
    }

    @Test
    fun returnsExpectedErrorsWithoutMemoryOrForBadFd() {
        val wasi = WasiStub()
        assertEquals(listOf(i32(52)), wasi.resolveFunction("wasi_snapshot_preview1", "args_sizes_get")!!.call(listOf(i32(0), i32(4))))

        val memory = LinearMemory(1)
        wasi.setMemory(memory)
        assertEquals(listOf(i32(8)), wasi.resolveFunction("wasi_snapshot_preview1", "fd_read")!!.call(listOf(i32(1), i32(0), i32(0), i32(4))))
    }

    private fun readCString(memory: LinearMemory, address: Int): String {
        val bytes = mutableListOf<Byte>()
        var cursor = address
        while (memory.loadI32_8u(cursor) != 0) {
            bytes += memory.loadI32_8u(cursor).toByte()
            cursor++
        }
        return bytes.toByteArray().toString(Charsets.UTF_8)
    }

    private fun readBytes(memory: LinearMemory, address: Int, length: Int): ByteArray =
        ByteArray(length) { index -> memory.loadI32_8u(address + index).toByte() }
}
