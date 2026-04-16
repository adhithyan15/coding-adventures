package com.codingadventures.wasmruntime

import com.codingadventures.wasmexecution.HostInterface
import com.codingadventures.wasmexecution.ImportedGlobal
import com.codingadventures.wasmexecution.LinearMemory
import com.codingadventures.wasmexecution.Table
import com.codingadventures.wasmexecution.TrapError
import com.codingadventures.wasmexecution.TypedHostFunction
import com.codingadventures.wasmexecution.WasmExecutionEngine
import com.codingadventures.wasmexecution.WasmValue
import com.codingadventures.wasmexecution.coerceValue
import com.codingadventures.wasmexecution.evaluateConstExpr
import com.codingadventures.wasmexecution.i32
import com.codingadventures.wasmexecution.unwrapValue
import com.codingadventures.wasmmoduleparser.WasmModuleParser
import com.codingadventures.wasmtypes.Export
import com.codingadventures.wasmtypes.ExternalKind
import com.codingadventures.wasmtypes.FuncType
import com.codingadventures.wasmtypes.FunctionBody
import com.codingadventures.wasmtypes.GlobalType
import com.codingadventures.wasmtypes.MemoryType
import com.codingadventures.wasmtypes.ValueType
import com.codingadventures.wasmtypes.WasmModule
import com.codingadventures.wasmtypes.makeFuncType
import com.codingadventures.wasmvalidator.ValidatedModule
import com.codingadventures.wasmvalidator.validate as validateModule
import java.nio.charset.StandardCharsets
import java.security.SecureRandom

const val VERSION: String = "0.1.0"

data class WasmInstance(
    val memory: LinearMemory?,
    val tables: MutableList<Table>,
    val globals: MutableList<WasmValue>,
    val globalTypes: MutableList<GlobalType>,
    val funcTypes: MutableList<FuncType>,
    val funcBodies: MutableList<FunctionBody?>,
    val hostFunctions: MutableList<TypedHostFunction?>,
    val exports: Map<String, Export>,
    val host: HostInterface?,
    internal val engine: WasmExecutionEngine,
)

class ProcExitError(val exitCode: Int) : RuntimeException("proc_exit($exitCode)")

fun interface WasiStdin {
    fun read(count: Int): Any?
}

interface WasiClock {
    fun realtimeNs(): Long

    fun monotonicNs(): Long

    fun resolutionNs(clockId: Int): Long
}

interface WasiRandom {
    fun fillBytes(buffer: ByteArray)
}

class SystemClock : WasiClock {
    override fun realtimeNs(): Long = System.currentTimeMillis() * 1_000_000L

    override fun monotonicNs(): Long = System.nanoTime()

    override fun resolutionNs(clockId: Int): Long = 1_000_000L
}

class SystemRandom : WasiRandom {
    private val random = SecureRandom()

    override fun fillBytes(buffer: ByteArray) {
        random.nextBytes(buffer)
    }
}

data class WasiConfig(
    val stdin: WasiStdin = WasiStdin { ByteArray(0) },
    val args: List<String> = emptyList(),
    val env: Map<String, String> = emptyMap(),
    val stdout: (String) -> Unit = {},
    val stderr: (String) -> Unit = {},
    val clock: WasiClock = SystemClock(),
    val random: WasiRandom = SystemRandom(),
)

open class WasiStub(private val config: WasiConfig = WasiConfig()) : HostInterface {
    companion object {
        private const val ENOSYS = 52
        private const val ESUCCESS = 0
        private const val EBADF = 8
        private const val EINVAL = 28
    }

    private var instanceMemory: LinearMemory? = null

    fun setMemory(memory: LinearMemory) {
        instanceMemory = memory
    }

    override fun resolveFunction(moduleName: String, name: String): TypedHostFunction? {
        if (moduleName != "wasi_snapshot_preview1") return null

        return when (name) {
            "fd_write" -> makeFdWrite()
            "fd_read" -> makeFdRead()
            "proc_exit" -> makeProcExit()
            "args_sizes_get" -> makeArgsSizesGet()
            "args_get" -> makeArgsGet()
            "environ_sizes_get" -> makeEnvironSizesGet()
            "environ_get" -> makeEnvironGet()
            "clock_res_get" -> makeClockResGet()
            "clock_time_get" -> makeClockTimeGet()
            "random_get" -> makeRandomGet()
            "sched_yield" -> makeSchedYield()
            else -> makeStub()
        }
    }

    override fun resolveGlobal(moduleName: String, name: String): ImportedGlobal? = null

    override fun resolveMemory(moduleName: String, name: String): LinearMemory? = null

    override fun resolveTable(moduleName: String, name: String): Table? = null

    private fun makeFdWrite(): TypedHostFunction =
        typedHostFunction(
            makeFuncType(
                listOf(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                listOf(ValueType.I32),
            )
        ) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val fd = (callArgs[0].value as Number).toInt()
            val iovsPtr = (callArgs[1].value as Number).toInt()
            val iovsLen = (callArgs[2].value as Number).toInt()
            val nwrittenPtr = (callArgs[3].value as Number).toInt()
            var totalWritten = 0

            for (i in 0 until iovsLen) {
                val bufPtr = memory.loadI32(iovsPtr + i * 8)
                val bufLen = memory.loadI32(iovsPtr + i * 8 + 4)
                val bytes = ByteArray(bufLen) { j -> memory.loadI32_8u(bufPtr + j).toByte() }
                val text = String(bytes, StandardCharsets.UTF_8)
                totalWritten += bufLen
                when (fd) {
                    1 -> config.stdout(text)
                    2 -> config.stderr(text)
                }
            }

            memory.storeI32(nwrittenPtr, totalWritten)
            listOf(i32(ESUCCESS))
        }

    private fun makeFdRead(): TypedHostFunction =
        typedHostFunction(
            makeFuncType(
                listOf(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                listOf(ValueType.I32),
            )
        ) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val fd = (callArgs[0].value as Number).toInt()
            if (fd != 0) {
                return@typedHostFunction listOf(i32(EBADF))
            }

            val iovsPtr = (callArgs[1].value as Number).toInt()
            val iovsLen = (callArgs[2].value as Number).toInt()
            val nreadPtr = (callArgs[3].value as Number).toInt()
            var totalRead = 0

            for (i in 0 until iovsLen) {
                val bufPtr = memory.loadI32(iovsPtr + i * 8)
                val bufLen = memory.loadI32(iovsPtr + i * 8 + 4)
                val chunk = normalizeInputChunk(config.stdin.read(bufLen), bufLen)
                for (j in chunk.indices) {
                    memory.storeI32_8(bufPtr + j, chunk[j].toInt())
                }
                totalRead += chunk.size
                if (chunk.size < bufLen) {
                    break
                }
            }

            memory.storeI32(nreadPtr, totalRead)
            listOf(i32(ESUCCESS))
        }

    private fun makeProcExit(): TypedHostFunction =
        typedHostFunction(makeFuncType(listOf(ValueType.I32), emptyList())) { callArgs ->
            throw ProcExitError((callArgs[0].value as Number).toInt())
        }

    private fun makeArgsSizesGet(): TypedHostFunction =
        typedHostFunction(makeFuncType(listOf(ValueType.I32, ValueType.I32), listOf(ValueType.I32))) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val argcPtr = (callArgs[0].value as Number).toInt()
            val argvBufSizePtr = (callArgs[1].value as Number).toInt()
            memory.storeI32(argcPtr, config.args.size)
            val bufSize = config.args.sumOf { it.toByteArray(StandardCharsets.UTF_8).size + 1 }
            memory.storeI32(argvBufSizePtr, bufSize)
            listOf(i32(ESUCCESS))
        }

    private fun makeArgsGet(): TypedHostFunction =
        typedHostFunction(makeFuncType(listOf(ValueType.I32, ValueType.I32), listOf(ValueType.I32))) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val argvPtr = (callArgs[0].value as Number).toInt()
            val argvBufPtr = (callArgs[1].value as Number).toInt()
            var offset = argvBufPtr

            config.args.forEachIndexed { index, arg ->
                memory.storeI32(argvPtr + index * 4, offset)
                arg.toByteArray(StandardCharsets.UTF_8).forEach { byte ->
                    memory.storeI32_8(offset++, byte.toInt())
                }
                memory.storeI32_8(offset++, 0)
            }

            listOf(i32(ESUCCESS))
        }

    private fun makeEnvironSizesGet(): TypedHostFunction =
        typedHostFunction(makeFuncType(listOf(ValueType.I32, ValueType.I32), listOf(ValueType.I32))) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val countPtr = (callArgs[0].value as Number).toInt()
            val bufSizePtr = (callArgs[1].value as Number).toInt()
            memory.storeI32(countPtr, config.env.size)
            val bufSize = config.env.entries.sumOf { "${it.key}=${it.value}".toByteArray(StandardCharsets.UTF_8).size + 1 }
            memory.storeI32(bufSizePtr, bufSize)
            listOf(i32(ESUCCESS))
        }

    private fun makeEnvironGet(): TypedHostFunction =
        typedHostFunction(makeFuncType(listOf(ValueType.I32, ValueType.I32), listOf(ValueType.I32))) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val environPtr = (callArgs[0].value as Number).toInt()
            val environBufPtr = (callArgs[1].value as Number).toInt()
            var offset = environBufPtr

            config.env.entries.forEachIndexed { index, entry ->
                memory.storeI32(environPtr + index * 4, offset)
                "${entry.key}=${entry.value}".toByteArray(StandardCharsets.UTF_8).forEach { byte ->
                    memory.storeI32_8(offset++, byte.toInt())
                }
                memory.storeI32_8(offset++, 0)
            }

            listOf(i32(ESUCCESS))
        }

    private fun makeClockResGet(): TypedHostFunction =
        typedHostFunction(makeFuncType(listOf(ValueType.I32, ValueType.I32), listOf(ValueType.I32))) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val clockId = (callArgs[0].value as Number).toInt()
            val resolutionPtr = (callArgs[1].value as Number).toInt()
            memory.storeI64(resolutionPtr, config.clock.resolutionNs(clockId))
            listOf(i32(ESUCCESS))
        }

    private fun makeClockTimeGet(): TypedHostFunction =
        typedHostFunction(
            makeFuncType(listOf(ValueType.I32, ValueType.I64, ValueType.I32), listOf(ValueType.I32))
        ) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val clockId = (callArgs[0].value as Number).toInt()
            val timePtr = (callArgs[2].value as Number).toInt()
            val timeNs =
                when (clockId) {
                    0 -> config.clock.realtimeNs()
                    1, 2, 3 -> config.clock.monotonicNs()
                    else -> return@typedHostFunction listOf(i32(EINVAL))
                }
            memory.storeI64(timePtr, timeNs)
            listOf(i32(ESUCCESS))
        }

    private fun makeRandomGet(): TypedHostFunction =
        typedHostFunction(makeFuncType(listOf(ValueType.I32, ValueType.I32), listOf(ValueType.I32))) { callArgs ->
            val memory = instanceMemory ?: return@typedHostFunction listOf(i32(ENOSYS))
            val bufPtr = (callArgs[0].value as Number).toInt()
            val bufLen = (callArgs[1].value as Number).toInt()
            val bytes = ByteArray(bufLen)
            config.random.fillBytes(bytes)
            memory.writeBytes(bufPtr, bytes)
            listOf(i32(ESUCCESS))
        }

    private fun makeSchedYield(): TypedHostFunction =
        typedHostFunction(makeFuncType(emptyList(), listOf(ValueType.I32))) {
            listOf(i32(ESUCCESS))
        }

    private fun makeStub(): TypedHostFunction =
        typedHostFunction(makeFuncType(emptyList(), listOf(ValueType.I32))) {
            listOf(i32(ENOSYS))
        }
}

class WasmRuntime(private val host: HostInterface? = null) {
    private val parser = WasmModuleParser()

    fun load(wasmBytes: ByteArray): WasmModule = parser.parse(wasmBytes)

    fun validate(module: WasmModule): ValidatedModule = validateModule(module)

    fun instantiate(module: WasmModule): WasmInstance {
        val funcTypes = mutableListOf<FuncType>()
        val funcBodies = mutableListOf<FunctionBody?>()
        val hostFunctions = mutableListOf<TypedHostFunction?>()
        val globalTypes = mutableListOf<GlobalType>()
        val globals = mutableListOf<WasmValue>()
        val tables = mutableListOf<Table>()
        var memory: LinearMemory? = null

        module.imports.forEach { importEntry ->
            when (importEntry.kind) {
                ExternalKind.FUNCTION -> {
                    val typeIndex = importEntry.typeInfo as Int
                    funcTypes += module.types[typeIndex]
                    funcBodies += null
                    hostFunctions += host?.resolveFunction(importEntry.moduleName, importEntry.name)
                }
                ExternalKind.MEMORY -> {
                    memory = host?.resolveMemory(importEntry.moduleName, importEntry.name) ?: memory
                }
                ExternalKind.TABLE -> {
                    host?.resolveTable(importEntry.moduleName, importEntry.name)?.let(tables::add)
                }
                ExternalKind.GLOBAL -> {
                    host?.resolveGlobal(importEntry.moduleName, importEntry.name)?.let { importedGlobal ->
                        globalTypes += importedGlobal.type
                        globals += importedGlobal.value
                    }
                }
            }
        }

        module.functions.forEachIndexed { index, typeIndex ->
            funcTypes += module.types[typeIndex]
            funcBodies += module.code[index]
            hostFunctions += null
        }

        if (memory == null && module.memories.isNotEmpty()) {
            val memoryType: MemoryType = module.memories.first()
            memory = LinearMemory(memoryType.limits.min, memoryType.limits.max)
        }

        module.tables.forEach { tableType ->
            tables += Table(tableType.limits.min, tableType.limits.max)
        }

        module.globals.forEach { global ->
            globals += evaluateConstExpr(global.initExpr, globals)
            globalTypes += global.globalType
        }

        memory?.let { linearMemory ->
            module.data.forEach { segment ->
                val offset = (evaluateConstExpr(segment.offsetExpr, globals).value as Number).toInt()
                linearMemory.storeBytes(offset, segment.data)
            }
        }

        module.elements.forEach { element ->
            val offset = (evaluateConstExpr(element.offsetExpr, globals).value as Number).toInt()
            val table = tables[element.tableIndex]
            element.functionIndices.forEachIndexed { index, functionIndex ->
                table.set(offset + index, functionIndex)
            }
        }

        val exports = module.exports.associateBy { it.name }
        val engine = WasmExecutionEngine(memory, tables, globals, globalTypes, funcTypes, funcBodies, hostFunctions)
        val instance = WasmInstance(memory, tables, globals, globalTypes, funcTypes, funcBodies, hostFunctions, exports, host, engine)

        if (host is WasiStub && memory != null) {
            host.setMemory(memory)
        }

        module.start?.let { startIndex ->
            engine.callFunction(startIndex, emptyList())
        }

        return instance
    }

    fun call(instance: WasmInstance, exportName: String, args: List<Any>): List<Any> {
        val export =
            instance.exports[exportName] ?: throw TrapError("export \"$exportName\" not found")
        if (export.kind != ExternalKind.FUNCTION) {
            throw TrapError("export \"$exportName\" is not a function")
        }

        val funcType = instance.funcTypes[export.index]
        val typedArgs = args.mapIndexed { index, value -> coerceValue(value, funcType.params[index]) }
        return instance.engine.callFunction(export.index, typedArgs).map(::unwrapValue)
    }

    fun loadAndRun(wasmBytes: ByteArray, exportName: String, args: List<Any>): List<Any> {
        val module = load(wasmBytes)
        validate(module)
        val instance = instantiate(module)
        return call(instance, exportName, args)
    }
}

private fun typedHostFunction(type: FuncType, fn: (List<WasmValue>) -> List<WasmValue>): TypedHostFunction =
    object : TypedHostFunction {
        override val type: FuncType = type

        override fun call(args: List<WasmValue>): List<WasmValue> = fn(args)
    }

private fun normalizeInputChunk(value: Any?, maxLen: Int): ByteArray {
    if (value == null) {
        return ByteArray(0)
    }

    val bytes =
        when (value) {
            is ByteArray -> value
            is String -> value.toByteArray(StandardCharsets.UTF_8)
            is List<*> -> ByteArray(value.size) { index ->
                val item = value[index] as? Number
                    ?: throw IllegalArgumentException("stdin callback list must contain only numbers")
                item.toByte()
            }
            else -> throw IllegalArgumentException("unsupported stdin callback value: ${value::class.qualifiedName}")
        }

    return if (bytes.size <= maxLen) bytes else bytes.copyOf(maxLen)
}
