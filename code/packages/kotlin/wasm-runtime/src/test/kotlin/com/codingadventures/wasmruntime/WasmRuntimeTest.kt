package com.codingadventures.wasmruntime

import com.codingadventures.wasmexecution.HostInterface
import com.codingadventures.wasmexecution.LinearMemory
import com.codingadventures.wasmexecution.Table
import com.codingadventures.wasmexecution.TrapError
import com.codingadventures.wasmexecution.TypedHostFunction
import com.codingadventures.wasmexecution.WasmValue
import com.codingadventures.wasmexecution.i32
import com.codingadventures.wasmleb128.WasmLeb128
import com.codingadventures.wasmtypes.DataSegment
import com.codingadventures.wasmtypes.Export
import com.codingadventures.wasmtypes.ExternalKind
import com.codingadventures.wasmtypes.FunctionBody
import com.codingadventures.wasmtypes.Global
import com.codingadventures.wasmtypes.GlobalType
import com.codingadventures.wasmtypes.Limits
import com.codingadventures.wasmtypes.MemoryType
import com.codingadventures.wasmtypes.ValueType
import com.codingadventures.wasmtypes.WasmModule
import com.codingadventures.wasmtypes.makeFuncType
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame

class WasmRuntimeTest {
    @Test
    fun loadsMinimalModule() {
        val runtime = WasmRuntime()
        val module = runtime.load(buildMinimalWasm())
        assertEquals(1, module.types.size)
    }

    @Test
    fun validatesParsedModule() {
        val runtime = WasmRuntime()
        val module = runtime.load(buildMinimalWasm())
        assertSame(module, runtime.validate(module).module)
    }

    @Test
    fun instantiatesMemoryAndDataSegments() {
        val runtime = WasmRuntime()
        val module =
            WasmModule().apply {
                memories += MemoryType(Limits(1, null))
                data += DataSegment(0, i32ConstExpr(256), "Hi".encodeToByteArray())
            }

        val instance = runtime.instantiate(module)

        assertNotNull(instance.memory)
        assertEquals('H'.code, instance.memory!!.loadI32_8u(256))
        assertEquals('i'.code, instance.memory!!.loadI32_8u(257))
    }

    @Test
    fun resolvesFunctionImportsAndCallsExports() {
        val host =
            object : HostInterface {
                override fun resolveFunction(moduleName: String, name: String): TypedHostFunction? =
                    if (moduleName == "env" && name == "double") {
                        object : TypedHostFunction {
                            override val type = makeFuncType(listOf(ValueType.I32), listOf(ValueType.I32))

                            override fun call(args: List<WasmValue>): List<WasmValue> = listOf(i32((args[0].value as Int) * 2))
                        }
                    } else {
                        null
                    }
            }

        val runtime = WasmRuntime(host)
        val module =
            WasmModule().apply {
                types += makeFuncType(listOf(ValueType.I32), listOf(ValueType.I32))
                imports += com.codingadventures.wasmtypes.Import("env", "double", ExternalKind.FUNCTION, 0)
                exports += Export("double", ExternalKind.FUNCTION, 0)
            }

        val instance = runtime.instantiate(module)
        assertNotNull(instance.hostFunctions[0])
        assertEquals(listOf(42), runtime.call(instance, "double", listOf(21)))
    }

    @Test
    fun loadAndRunExecutesExportedFunction() {
        val runtime = WasmRuntime()
        assertEquals(listOf(42), runtime.loadAndRun(buildAnswerWasm(), "answer", emptyList()))
    }

    @Test
    fun startFunctionRunsDuringInstantiation() {
        val runtime = WasmRuntime()
        val module =
            WasmModule().apply {
                types += makeFuncType(emptyList(), emptyList())
                functions += 0
                code += FunctionBody(emptyList(), concat(byteArrayOf(0x41), WasmLeb128.encodeSigned(99), byteArrayOf(0x24, 0x00, 0x0B)))
                globals += Global(GlobalType(ValueType.I32, true), i32ConstExpr(0))
                start = 0
            }

        val instance = runtime.instantiate(module)
        assertEquals(99, instance.globals[0].value)
    }

    @Test
    fun throwsForMissingOrNonFunctionExport() {
        val runtime = WasmRuntime()
        val emptyInstance = runtime.instantiate(WasmModule())
        assertFailsWith<TrapError> { runtime.call(emptyInstance, "missing", emptyList()) }

        val module =
            WasmModule().apply {
                memories += MemoryType(Limits(1, null))
                exports += Export("memory", ExternalKind.MEMORY, 0)
            }
        val instance = runtime.instantiate(module)
        assertFailsWith<TrapError> { runtime.call(instance, "memory", emptyList()) }
    }

    @Test
    fun preservesHostReferenceOnInstance() {
        val wasi = WasiStub()
        val runtime = WasmRuntime(wasi)
        val instance = runtime.instantiate(WasmModule())
        assertSame(wasi, instance.host)
        assertNull(WasmRuntime().instantiate(WasmModule()).host)
    }

    @Test
    fun wasiFdWriteCapturesStdoutAndByteCount() {
        var stdout = ""
        val wasi =
            WasiStub(
                WasiConfig(
                    args = listOf("prog"),
                    env = mapOf("HOME" to "/tmp"),
                    stdout = { stdout = it },
                ),
            )
        val memory = LinearMemory(1)
        wasi.setMemory(memory)

        val text = "Hello".encodeToByteArray()
        val iovsPtr = 0
        val nwrittenPtr = 64
        val bufPtr = 128
        memory.storeBytes(bufPtr, text)
        memory.storeI32(iovsPtr, bufPtr)
        memory.storeI32(iovsPtr + 4, text.size)

        val result =
            wasi.resolveFunction("wasi_snapshot_preview1", "fd_write")!!.call(
                listOf(i32(1), i32(iovsPtr), i32(1), i32(nwrittenPtr)),
            )

        assertEquals(listOf(i32(0)), result)
        assertEquals("Hello", stdout)
        assertEquals(5, memory.loadI32(nwrittenPtr))
    }

    @Test
    fun wasiFdReadCopiesStdinIntoMemory() {
        val wasi =
            WasiStub(
                WasiConfig(
                    stdin = WasiStdin { "abc" },
                ),
            )
        val memory = LinearMemory(1)
        wasi.setMemory(memory)

        val iovsPtr = 0
        val nreadPtr = 64
        val bufPtr = 128
        memory.storeI32(iovsPtr, bufPtr)
        memory.storeI32(iovsPtr + 4, 8)

        val result =
            wasi.resolveFunction("wasi_snapshot_preview1", "fd_read")!!.call(
                listOf(i32(0), i32(iovsPtr), i32(1), i32(nreadPtr)),
            )

        assertEquals(listOf(i32(0)), result)
        assertEquals(3, memory.loadI32(nreadPtr))
        assertEquals('a'.code, memory.loadI32_8u(bufPtr))
        assertEquals('b'.code, memory.loadI32_8u(bufPtr + 1))
        assertEquals('c'.code, memory.loadI32_8u(bufPtr + 2))
    }

    @Test
    fun procExitErrorCarriesExitCode() {
        val error =
            assertFailsWith<ProcExitError> {
                WasiStub().resolveFunction("wasi_snapshot_preview1", "proc_exit")!!.call(listOf(i32(42)))
            }

        assertEquals("proc_exit(42)", error.message)
        assertEquals(42, error.exitCode)
    }

    private fun i32ConstExpr(value: Int): ByteArray = concat(byteArrayOf(0x41), WasmLeb128.encodeSigned(value), byteArrayOf(0x0B))

    private fun buildMinimalWasm(): ByteArray =
        concat(
            byteArrayOf(0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00),
            makeSection(1, byteArrayOf(0x01, 0x60, 0x00, 0x00)),
        )

    private fun buildAnswerWasm(): ByteArray {
        val typePayload = byteArrayOf(0x01, 0x60, 0x00, 0x01, 0x7F)
        val functionPayload = byteArrayOf(0x01, 0x00)
        val exportPayload = concat(byteArrayOf(0x01), makeString("answer"), byteArrayOf(0x00, 0x00))
        val bodyPayload = byteArrayOf(0x00, 0x41, 0x2A, 0x0B)
        val codePayload = concat(byteArrayOf(0x01), WasmLeb128.encodeUnsigned(bodyPayload.size.toLong()), bodyPayload)

        return concat(
            byteArrayOf(0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00),
            makeSection(1, typePayload),
            makeSection(3, functionPayload),
            makeSection(7, exportPayload),
            makeSection(10, codePayload),
        )
    }

    private fun makeSection(id: Int, payload: ByteArray): ByteArray = concat(byteArrayOf(id.toByte()), WasmLeb128.encodeUnsigned(payload.size.toLong()), payload)

    private fun makeString(value: String): ByteArray = concat(WasmLeb128.encodeUnsigned(value.encodeToByteArray().size.toLong()), value.encodeToByteArray())

    private fun concat(vararg parts: ByteArray): ByteArray {
        val output = ArrayList<Byte>(parts.sumOf { it.size })
        parts.forEach { output.addAll(it.toList()) }
        return output.toByteArray()
    }
}
