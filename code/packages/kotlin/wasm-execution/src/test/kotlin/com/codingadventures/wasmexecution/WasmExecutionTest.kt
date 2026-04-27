package com.codingadventures.wasmexecution

import com.codingadventures.wasmtypes.BLOCK_TYPE_EMPTY
import com.codingadventures.wasmtypes.FunctionBody
import com.codingadventures.wasmtypes.GlobalType
import com.codingadventures.wasmtypes.ValueType
import com.codingadventures.wasmtypes.makeFuncType
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class WasmExecutionTest {
    @Test
    fun exposesVersion() {
        assertEquals("0.1.0", VERSION)
    }

    @Test
    fun returnsConstant() {
        val result = run(emptyList(), listOf(ValueType.I32), emptyList(), bytes(i32Const(42)))
        assertEquals(42, result[0].value)
    }

    @Test
    fun addsTwoArguments() {
        val result =
            run(
                listOf(ValueType.I32, ValueType.I32),
                listOf(ValueType.I32),
                emptyList(),
                bytes(localGet(0), localGet(1), byteArrayOf(0x6A)),
                listOf(i32(3), i32(4)),
            )
        assertEquals(7, result[0].value)
    }

    @Test
    fun usesDeclaredLocalAndLocalTee() {
        val result =
            run(
                listOf(ValueType.I32),
                listOf(ValueType.I32),
                listOf(ValueType.I32),
                bytes(localGet(0), localTee(1), dropInstr(), localGet(1)),
                listOf(i32(99)),
            )
        assertEquals(99, result[0].value)
    }

    @Test
    fun branchesThroughBlockLoopAndIf() {
        val result =
            run(
                emptyList(),
                listOf(ValueType.I32),
                listOf(ValueType.I32),
                bytes(
                    i32Const(3),
                    localSet(0),
                    byteArrayOf(0x02, ValueType.I32.code.toByte()),
                    byteArrayOf(0x03, BLOCK_TYPE_EMPTY.toByte()),
                    localGet(0),
                    byteArrayOf(0x45),
                    byteArrayOf(0x04, BLOCK_TYPE_EMPTY.toByte()),
                    localGet(0),
                    br(2),
                    byteArrayOf(0x0B),
                    localGet(0),
                    i32Const(1),
                    byteArrayOf(0x6B),
                    localSet(0),
                    br(0),
                    byteArrayOf(0x0B),
                    i32Const(-1),
                    byteArrayOf(0x0B),
                ),
            )
        assertEquals(0, result[0].value)
    }

    @Test
    fun takesElseBranch() {
        val result =
            run(
                emptyList(),
                listOf(ValueType.I32),
                emptyList(),
                bytes(i32Const(0), byteArrayOf(0x04, ValueType.I32.code.toByte()), i32Const(10), byteArrayOf(0x05), i32Const(20), byteArrayOf(0x0B)),
            )
        assertEquals(20, result[0].value)
    }

    @Test
    fun returnExitsFunctionEarly() {
        val result = run(emptyList(), listOf(ValueType.I32), emptyList(), bytes(i32Const(42), byteArrayOf(0x0F), i32Const(99)))
        assertEquals(42, result[0].value)
    }

    @Test
    fun callsHostAndIndirectFunctions() {
        val funcType = makeFuncType(listOf(ValueType.I32), listOf(ValueType.I32))
        val hostFunction =
            object : TypedHostFunction {
                override val type: com.codingadventures.wasmtypes.FuncType = funcType

                override fun call(args: List<WasmValue>): List<WasmValue> = listOf(i32((args[0].value as Int) * 2))
            }

        val table = Table(1).also { it.set(0, 0) }
        val engine =
            WasmExecutionEngine(
                memory = null,
                tables = listOf(table),
                globals = mutableListOf(),
                globalTypes = mutableListOf(),
                funcTypes = listOf(funcType),
                funcBodies = listOf(null),
                hostFunctions = listOf(hostFunction),
            )
        assertEquals(42, engine.callFunction(0, listOf(i32(21)))[0].value)

        val caller =
            WasmExecutionEngine(
                memory = null,
                tables = listOf(table),
                globals = mutableListOf(),
                globalTypes = mutableListOf(),
                funcTypes = listOf(funcType, makeFuncType(emptyList(), listOf(ValueType.I32))),
                funcBodies = listOf(null, body(emptyList(), bytes(i32Const(21), i32Const(0), byteArrayOf(0x11, 0x00, 0x00)))),
                hostFunctions = listOf(hostFunction, null),
            )
        assertEquals(42, caller.callFunction(1, emptyList())[0].value)
    }

    @Test
    fun updatesMutableGlobal() {
        val globals = mutableListOf(i32(0))
        val globalTypes = mutableListOf(GlobalType(ValueType.I32, true))
        val engine =
            WasmExecutionEngine(
                memory = null,
                tables = emptyList(),
                globals = globals,
                globalTypes = globalTypes,
                funcTypes = listOf(makeFuncType(emptyList(), emptyList())),
                funcBodies = listOf(body(emptyList(), bytes(i32Const(99), globalSet(0)))),
                hostFunctions = listOf(null),
            )

        engine.callFunction(0, emptyList())
        assertEquals(99, globals[0].value)
    }

    @Test
    fun roundTripsLoadsStoresAndMemoryGrowth() {
        val memory = LinearMemory(1, 4)
        val result =
            runWithMemory(
                memory = memory,
                params = emptyList(),
                results = listOf(ValueType.I32),
                locals = listOf(ValueType.I32, ValueType.I64, ValueType.F32, ValueType.F64, ValueType.I32, ValueType.I32),
                code =
                    bytes(
                        i32Const(0),
                        i32Const(123),
                        memOp(0x36, 2, 8),
                        i32Const(0),
                        memOp(0x28, 2, 8),
                        localSet(0),
                        i32Const(16),
                        i64Const(0x1020_3040_5060_7080L),
                        memOp(0x37, 3, 0),
                        i32Const(16),
                        memOp(0x29, 3, 0),
                        localSet(1),
                        i32Const(32),
                        f32Const(3.25f),
                        memOp(0x38, 2, 0),
                        i32Const(32),
                        memOp(0x2A, 2, 0),
                        localSet(2),
                        i32Const(40),
                        f64Const(Math.PI),
                        memOp(0x39, 3, 0),
                        i32Const(40),
                        memOp(0x2B, 3, 0),
                        localSet(3),
                        i32Const(48),
                        i32Const(0xFF),
                        memOp(0x3A, 0, 0),
                        i32Const(48),
                        memOp(0x2C, 0, 0),
                        localSet(4),
                        i32Const(2),
                        byteArrayOf(0x40, 0x00),
                        localSet(5),
                        localGet(0),
                        dropInstr(),
                        localGet(1),
                        dropInstr(),
                        localGet(2),
                        dropInstr(),
                        localGet(3),
                        dropInstr(),
                        localGet(4),
                        dropInstr(),
                        localGet(5),
                    ),
            )

        assertEquals(1, result[0].value)
        assertEquals(3, memory.size())
        assertEquals(123, memory.loadI32(8))
        assertEquals(0x1020_3040_5060_7080L, memory.loadI64(16))
    }

    @Test
    fun trapsWithoutMemoryAndOnUnknownFunction() {
        val noMemory = engine(emptyList(), listOf(ValueType.I32), emptyList(), bytes(i32Const(0), memOp(0x28, 2, 0)))
        assertFailsWith<TrapError> { noMemory.callFunction(0, emptyList()) }

        val empty =
            WasmExecutionEngine(
                memory = null,
                tables = emptyList(),
                globals = mutableListOf(),
                globalTypes = mutableListOf(),
                funcTypes = emptyList(),
                funcBodies = emptyList(),
                hostFunctions = emptyList(),
            )
        assertFailsWith<TrapError> { empty.callFunction(99, emptyList()) }
    }

    @Test
    fun trapsOnUnreachableAndIntegerDivideByZero() {
        val unreachable = engine(emptyList(), emptyList(), emptyList(), byteArrayOf(0x00, 0x0B))
        assertFailsWith<TrapError> { unreachable.callFunction(0, emptyList()) }

        val divideByZero = engine(emptyList(), listOf(ValueType.I32), emptyList(), bytes(i32Const(7), i32Const(0), byteArrayOf(0x6D)))
        assertFailsWith<TrapError> { divideByZero.callFunction(0, emptyList()) }
    }

    @Test
    fun comparesAndRotatesI32Values() {
        val result =
            run(
                emptyList(),
                listOf(ValueType.I32, ValueType.I32, ValueType.I32),
                listOf(ValueType.I32, ValueType.I32, ValueType.I32),
                bytes(
                    i32Const(-1),
                    i32Const(0),
                    byteArrayOf(0x49),
                    localSet(0),
                    i32Const(0x80000001.toInt()),
                    i32Const(1),
                    byteArrayOf(0x78),
                    localSet(1),
                    i32Const(0b10110011),
                    byteArrayOf(0x69),
                    localSet(2),
                    localGet(0),
                    localGet(1),
                    localGet(2),
                ),
            )

        assertEquals(0, result[0].value)
        assertEquals(0xC0000000.toInt(), result[1].value)
        assertEquals(5, result[2].value)
    }

    @Test
    fun coversRemainingI32ArithmeticAndBitwiseOpcodes() {
        val result =
            run(
                emptyList(),
                List(17) { ValueType.I32 },
                List(17) { ValueType.I32 },
                bytes(
                    i32Const(7), i32Const(7), byteArrayOf(0x46), localSet(0),
                    i32Const(1), i32Const(2), byteArrayOf(0x47), localSet(1),
                    i32Const(-1), i32Const(0), byteArrayOf(0x48), localSet(2),
                    i32Const(5), i32Const(3), byteArrayOf(0x4A), localSet(3),
                    i32Const(-1), i32Const(0), byteArrayOf(0x4B), localSet(4),
                    i32Const(3), i32Const(5), byteArrayOf(0x4C), localSet(5),
                    i32Const(0), i32Const(-1), byteArrayOf(0x4D), localSet(6),
                    i32Const(5), i32Const(3), byteArrayOf(0x4E), localSet(7),
                    i32Const(-1), i32Const(0), byteArrayOf(0x4F), localSet(8),
                    i32Const(1), byteArrayOf(0x67), localSet(9),
                    i32Const(8), byteArrayOf(0x68), localSet(10),
                    i32Const(6), i32Const(7), byteArrayOf(0x6C), localSet(11),
                    i32Const(-1), i32Const(2), byteArrayOf(0x6E), localSet(12),
                    i32Const(-7), i32Const(3), byteArrayOf(0x6F), localSet(13),
                    i32Const(7), i32Const(3), byteArrayOf(0x70), localSet(14),
                    i32Const(0xF0), i32Const(0x0F), byteArrayOf(0x72), localSet(15),
                    i32Const(1), i32Const(4), byteArrayOf(0x74), localSet(16),
                    localGet(0), localGet(1), localGet(2), localGet(3), localGet(4),
                    localGet(5), localGet(6), localGet(7), localGet(8), localGet(9),
                    localGet(10), localGet(11), localGet(12), localGet(13), localGet(14),
                    localGet(15), localGet(16),
                ),
            )

        assertEquals(listOf(1, 1, 1, 1, 1, 1, 1, 1, 1, 31, 3, 42, 2147483647, -1, 1, 255, 16), result.map { it.value })
    }

    @Test
    fun coversRemainingMemoryOpcodesAndMemorySize() {
        val memory = LinearMemory(1, 4)
        val result =
            runWithMemory(
                memory = memory,
                params = emptyList(),
                results = listOf(
                    ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I64, ValueType.I64,
                    ValueType.I64, ValueType.I64, ValueType.I64, ValueType.I32,
                ),
                locals = listOf(
                    ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I64, ValueType.I64,
                    ValueType.I64, ValueType.I64, ValueType.I64, ValueType.I32,
                ),
                code =
                    bytes(
                        i32Const(0), i32Const(0xFFFF), memOp(0x3B, 1, 0),
                        i32Const(0), memOp(0x2E, 1, 0), localSet(0),
                        i32Const(0), memOp(0x2F, 1, 0), localSet(1),
                        i32Const(4), i32Const(0xFF), memOp(0x3A, 0, 0),
                        i32Const(4), memOp(0x2D, 0, 0), localSet(2),
                        i32Const(8), i32Const(-1), memOp(0x36, 2, 0),
                        i32Const(8), memOp(0x34, 2, 0), localSet(3),
                        i32Const(8), memOp(0x35, 2, 0), localSet(4),
                        i32Const(4), memOp(0x30, 0, 0), localSet(5),
                        i32Const(0), memOp(0x33, 1, 0), localSet(6),
                        i32Const(12), i64Const(0x1FFFFFFFFL), memOp(0x3E, 2, 0),
                        i32Const(12), memOp(0x35, 2, 0), localSet(7),
                        byteArrayOf(0x3F, 0x00), localSet(8),
                        localGet(0), localGet(1), localGet(2), localGet(3), localGet(4),
                        localGet(5), localGet(6), localGet(7), localGet(8),
                    ),
            )

        assertEquals(-1, result[0].value)
        assertEquals(65535, result[1].value)
        assertEquals(255, result[2].value)
        assertEquals(-1L, result[3].value)
        assertEquals(4294967295L, result[4].value)
        assertEquals(-1L, result[5].value)
        assertEquals(65535L, result[6].value)
        assertEquals(4294967295L, result[7].value)
        assertEquals(1, result[8].value)
    }

    @Test
    fun trapsOnEngineEdgeCases() {
        val immutableGlobal =
            WasmExecutionEngine(
                memory = null,
                tables = emptyList(),
                globals = mutableListOf(i32(0)),
                globalTypes = mutableListOf(GlobalType(ValueType.I32, false)),
                funcTypes = listOf(makeFuncType(emptyList(), emptyList())),
                funcBodies = listOf(body(emptyList(), bytes(i32Const(1), globalSet(0)))),
                hostFunctions = listOf(null),
            )
        assertFailsWith<TrapError> { immutableGlobal.callFunction(0, emptyList()) }

        val badBranch = engine(emptyList(), emptyList(), emptyList(), bytes(br(0)))
        assertFailsWith<TrapError> { badBranch.callFunction(0, emptyList()) }

        val unexpectedElse = engine(emptyList(), emptyList(), emptyList(), byteArrayOf(0x05, 0x0B))
        assertFailsWith<TrapError> { unexpectedElse.callFunction(0, emptyList()) }

        val invalidMemoryImmediate = memoryEngine(bytes(byteArrayOf(0x3F, 0x01)))
        assertFailsWith<TrapError> { invalidMemoryImmediate.callFunction(0, emptyList()) }

        val operandUnderflow = engine(emptyList(), emptyList(), emptyList(), bytes(dropInstr()))
        assertFailsWith<TrapError> { operandUnderflow.callFunction(0, emptyList()) }
    }

    @Test
    fun trapsOnIndirectCallErrors() {
        val unary = makeFuncType(listOf(ValueType.I32), listOf(ValueType.I32))
        val nullary = makeFuncType(emptyList(), listOf(ValueType.I32))
        val hostFunction =
            object : TypedHostFunction {
                override val type = unary

                override fun call(args: List<WasmValue>): List<WasmValue> = listOf(i32(1))
            }

        val table = Table(1).also { it.set(0, 0) }
        val mismatch =
            WasmExecutionEngine(
                memory = null,
                tables = listOf(table),
                globals = mutableListOf(),
                globalTypes = mutableListOf(),
                funcTypes = listOf(unary, nullary, makeFuncType(emptyList(), listOf(ValueType.I32))),
                funcBodies = listOf(null, null, body(emptyList(), bytes(i32Const(0), byteArrayOf(0x11, 0x01, 0x00)))),
                hostFunctions = listOf(hostFunction, null, null),
            )
        assertFailsWith<TrapError> { mismatch.callFunction(2, emptyList()) }

        val uninitialized =
            WasmExecutionEngine(
                memory = null,
                tables = listOf(Table(1)),
                globals = mutableListOf(),
                globalTypes = mutableListOf(),
                funcTypes = listOf(unary, makeFuncType(emptyList(), listOf(ValueType.I32))),
                funcBodies = listOf(null, body(emptyList(), bytes(i32Const(0), byteArrayOf(0x11, 0x00, 0x00)))),
                hostFunctions = listOf(hostFunction, null),
            )
        assertFailsWith<TrapError> { uninitialized.callFunction(1, emptyList()) }
    }

    @Test
    fun coversHostInterfaceDefaultsAndImportedGlobalRecord() {
        val hostInterface = object : HostInterface {}
        assertEquals(null, hostInterface.resolveFunction("env", "f"))
        assertEquals(null, hostInterface.resolveGlobal("env", "g"))
        assertEquals(null, hostInterface.resolveMemory("env", "m"))
        assertEquals(null, hostInterface.resolveTable("env", "t"))

        val globalType = GlobalType(ValueType.I32, true)
        val imported = ImportedGlobal(globalType, i32(7))
        assertEquals(globalType, imported.type)
        assertEquals(i32(7), imported.value)
    }

    private fun run(
        params: List<ValueType>,
        results: List<ValueType>,
        locals: List<ValueType>,
        code: ByteArray,
        args: List<WasmValue> = emptyList(),
    ): List<WasmValue> = engine(params, results, locals, code).callFunction(0, args)

    private fun runWithMemory(
        memory: LinearMemory,
        params: List<ValueType>,
        results: List<ValueType>,
        locals: List<ValueType>,
        code: ByteArray,
    ): List<WasmValue> =
        WasmExecutionEngine(
            memory = memory,
            tables = emptyList(),
            globals = mutableListOf(),
            globalTypes = mutableListOf(),
            funcTypes = listOf(makeFuncType(params, results)),
            funcBodies = listOf(body(locals, code)),
            hostFunctions = listOf(null),
        ).callFunction(0, emptyList())

    private fun engine(
        params: List<ValueType>,
        results: List<ValueType>,
        locals: List<ValueType>,
        code: ByteArray,
    ): WasmExecutionEngine =
        WasmExecutionEngine(
            memory = null,
            tables = emptyList(),
            globals = mutableListOf(),
            globalTypes = mutableListOf(),
            funcTypes = listOf(makeFuncType(params, results)),
            funcBodies = listOf(body(locals, code)),
            hostFunctions = listOf(null),
        )

    private fun memoryEngine(code: ByteArray): WasmExecutionEngine =
        WasmExecutionEngine(
            memory = LinearMemory(1, 1),
            tables = emptyList(),
            globals = mutableListOf(),
            globalTypes = mutableListOf(),
            funcTypes = listOf(makeFuncType(emptyList(), emptyList())),
            funcBodies = listOf(body(emptyList(), code)),
            hostFunctions = listOf(null),
        )

    private fun body(locals: List<ValueType>, code: ByteArray): FunctionBody = FunctionBody(locals, concat(code, byteArrayOf(0x0B)))

    private fun bytes(vararg parts: ByteArray): ByteArray = concat(*parts)

    private fun i32Const(value: Int): ByteArray = concat(byteArrayOf(0x41), WasmExecutionTestSupport.encodeSigned32(value))

    private fun i64Const(value: Long): ByteArray = concat(byteArrayOf(0x42), WasmExecutionTestSupport.encodeSigned64(value))

    private fun f32Const(value: Float): ByteArray = concat(byteArrayOf(0x43), ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putFloat(value).array())

    private fun f64Const(value: Double): ByteArray = concat(byteArrayOf(0x44), ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putDouble(value).array())

    private fun localGet(index: Int): ByteArray = concat(byteArrayOf(0x20), WasmExecutionTestSupport.encodeUnsigned(index.toLong()))

    private fun localSet(index: Int): ByteArray = concat(byteArrayOf(0x21), WasmExecutionTestSupport.encodeUnsigned(index.toLong()))

    private fun localTee(index: Int): ByteArray = concat(byteArrayOf(0x22), WasmExecutionTestSupport.encodeUnsigned(index.toLong()))

    private fun globalSet(index: Int): ByteArray = concat(byteArrayOf(0x24), WasmExecutionTestSupport.encodeUnsigned(index.toLong()))

    private fun br(depth: Int): ByteArray = concat(byteArrayOf(0x0C), WasmExecutionTestSupport.encodeUnsigned(depth.toLong()))

    private fun memOp(opcode: Int, align: Int, offset: Int): ByteArray =
        concat(byteArrayOf(opcode.toByte()), WasmExecutionTestSupport.encodeUnsigned(align.toLong()), WasmExecutionTestSupport.encodeUnsigned(offset.toLong()))

    private fun dropInstr(): ByteArray = byteArrayOf(0x1A)

    private fun concat(vararg parts: ByteArray): ByteArray =
        ByteArrayOutputStream().use { output ->
            parts.forEach(output::writeBytes)
            output.toByteArray()
        }
}
