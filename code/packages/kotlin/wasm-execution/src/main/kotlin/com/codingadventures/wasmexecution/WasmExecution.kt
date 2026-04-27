package com.codingadventures.wasmexecution

import com.codingadventures.wasmtypes.FuncType
import com.codingadventures.wasmtypes.FunctionBody
import com.codingadventures.wasmtypes.GlobalType
import com.codingadventures.wasmtypes.ValueType
import com.codingadventures.wasmtypes.BLOCK_TYPE_EMPTY
import java.nio.ByteBuffer
import java.nio.ByteOrder

const val VERSION: String = "0.1.0"
const val PAGE_SIZE: Int = 65_536

data class WasmValue(val type: ValueType, val value: Any)

data class ImportedGlobal(val type: GlobalType, val value: WasmValue)

fun interface HostFunction {
    fun call(args: List<WasmValue>): List<WasmValue>
}

interface TypedHostFunction : HostFunction {
    val type: FuncType
}

interface HostInterface {
    fun resolveFunction(moduleName: String, name: String): TypedHostFunction? = null

    fun resolveGlobal(moduleName: String, name: String): ImportedGlobal? = null

    fun resolveMemory(moduleName: String, name: String): LinearMemory? = null

    fun resolveTable(moduleName: String, name: String): Table? = null
}

class TrapError(message: String) : RuntimeException(message)

class LinearMemory(minPages: Int, private val maxPages: Int? = null) {
    private var data = ByteArray(minPages.coerceAtLeast(0) * PAGE_SIZE)
    private fun view(): ByteBuffer = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN)

    fun size(): Int = data.size / PAGE_SIZE

    fun grow(pages: Int): Int {
        val previousSize = size()
        val nextSize = previousSize + pages
        if (maxPages != null && nextSize > maxPages) {
            return -1
        }
        if (nextSize > 65_536) {
            return -1
        }
        data = data.copyOf(nextSize * PAGE_SIZE)
        return previousSize
    }

    fun byteLength(): Int = data.size

    fun loadI32(address: Int): Int {
        ensureAddress(address, 4)
        return view().getInt(address)
    }

    fun loadI64(address: Int): Long {
        ensureAddress(address, 8)
        return view().getLong(address)
    }

    fun loadF32(address: Int): Float {
        ensureAddress(address, 4)
        return view().getFloat(address)
    }

    fun loadF64(address: Int): Double {
        ensureAddress(address, 8)
        return view().getDouble(address)
    }

    fun loadI32_8s(address: Int): Int {
        ensureAddress(address, 1)
        return data[address].toInt()
    }

    fun loadI32_8u(address: Int): Int {
        ensureAddress(address, 1)
        return data[address].toInt() and 0xFF
    }

    fun loadI32_16s(address: Int): Int {
        ensureAddress(address, 2)
        return view().getShort(address).toInt()
    }

    fun loadI32_16u(address: Int): Int {
        ensureAddress(address, 2)
        return view().getShort(address).toInt() and 0xFFFF
    }

    fun loadI64_8s(address: Int): Long = loadI32_8s(address).toLong()

    fun loadI64_8u(address: Int): Long = loadI32_8u(address).toLong()

    fun loadI64_16s(address: Int): Long = loadI32_16s(address).toLong()

    fun loadI64_16u(address: Int): Long = loadI32_16u(address).toLong()

    fun loadI64_32s(address: Int): Long = loadI32(address).toLong()

    fun loadI64_32u(address: Int): Long = loadI32(address).toLong() and 0xFFFF_FFFFL

    fun storeI32(address: Int, value: Int) {
        ensureAddress(address, 4)
        view().putInt(address, value)
    }

    fun storeI64(address: Int, value: Long) {
        ensureAddress(address, 8)
        view().putLong(address, value)
    }

    fun storeF32(address: Int, value: Float) {
        ensureAddress(address, 4)
        view().putFloat(address, value)
    }

    fun storeF64(address: Int, value: Double) {
        ensureAddress(address, 8)
        view().putDouble(address, value)
    }

    fun storeByte(address: Int, value: Int) {
        ensureAddress(address, 1)
        data[address] = value.toByte()
    }

    fun storeI32_8(address: Int, value: Int) = storeByte(address, value)

    fun storeI32_16(address: Int, value: Int) {
        ensureAddress(address, 2)
        view().putShort(address, value.toShort())
    }

    fun storeI64_8(address: Int, value: Long) = storeByte(address, value.toInt())

    fun storeI64_16(address: Int, value: Long) {
        ensureAddress(address, 2)
        view().putShort(address, value.toShort())
    }

    fun storeI64_32(address: Int, value: Long) = storeI32(address, value.toInt())

    fun storeBytes(address: Int, values: ByteArray) {
        ensureAddress(address, values.size)
        values.copyInto(data, address)
    }

    fun writeBytes(address: Int, values: ByteArray) = storeBytes(address, values)

    private fun ensureAddress(address: Int, width: Int) {
        if (address < 0 || address + width > data.size) {
            throw TrapError("memory access out of bounds at address $address")
        }
    }
}

class Table(minSize: Int, val maxSize: Int? = null) {
    private var entries = arrayOfNulls<Int>(minSize)

    fun get(index: Int): Int? {
        ensureIndex(index)
        return entries[index]
    }

    fun set(index: Int, value: Int?) {
        ensureIndex(index)
        entries[index] = value
    }

    fun size(): Int = entries.size

    fun grow(delta: Int): Int {
        val oldSize = entries.size
        val newSize = oldSize + delta
        if (maxSize != null && newSize > maxSize) {
            return -1
        }
        entries = entries.copyOf(newSize)
        return oldSize
    }

    private fun ensureIndex(index: Int) {
        if (index < 0 || index >= entries.size) {
            throw TrapError("table index out of bounds: $index")
        }
    }
}

fun i32(value: Int): WasmValue = WasmValue(ValueType.I32, value)

fun i64(value: Long): WasmValue = WasmValue(ValueType.I64, value)

fun f32(value: Float): WasmValue = WasmValue(ValueType.F32, value)

fun f64(value: Double): WasmValue = WasmValue(ValueType.F64, value)

fun defaultValue(type: ValueType): WasmValue =
    when (type) {
        ValueType.I32 -> i32(0)
        ValueType.I64 -> i64(0L)
        ValueType.F32 -> f32(0f)
        ValueType.F64 -> f64(0.0)
    }

fun coerceValue(rawValue: Any, type: ValueType): WasmValue {
    if (rawValue is WasmValue) {
        if (rawValue.type != type) {
            throw TrapError("expected $type argument but received ${rawValue.type}")
        }
        return rawValue
    }

    val number = rawValue as? Number ?: throw TrapError("unsupported host value: $rawValue")
    return when (type) {
        ValueType.I32 -> i32(number.toInt())
        ValueType.I64 -> i64(number.toLong())
        ValueType.F32 -> f32(number.toFloat())
        ValueType.F64 -> f64(number.toDouble())
    }
}

fun unwrapValue(value: WasmValue): Any = value.value

fun evaluateConstExpr(expression: ByteArray, globals: List<WasmValue>): WasmValue {
    if (expression.size < 2 || (expression.last().toInt() and 0xFF) != 0x0B) {
        throw TrapError("constant expression must end with opcode 0x0B")
    }

    val opcode = expression[0].toInt() and 0xFF
    val offset = 1

    return when (opcode) {
        0x41 -> i32(readSignedLeb32(expression, offset).value)
        0x42 -> i64(readSignedLeb64(expression, offset).value)
        0x43 -> {
            val value = ByteBuffer.wrap(expression, offset, 4).order(ByteOrder.LITTLE_ENDIAN).float
            f32(value)
        }
        0x44 -> {
            val value = ByteBuffer.wrap(expression, offset, 8).order(ByteOrder.LITTLE_ENDIAN).double
            f64(value)
        }
        0x23 -> {
            val index = readUnsignedLeb(expression, offset).value.toInt()
            globals.getOrNull(index) ?: throw TrapError("undefined global index $index")
        }
        else -> throw TrapError("unsupported const expression opcode 0x${opcode.toString(16)}")
    }
}

private data class UnsignedLeb(val value: Long, val bytesConsumed: Int)

private data class SignedLeb32(val value: Int, val bytesConsumed: Int)

private data class SignedLeb64(val value: Long, val bytesConsumed: Int)

private data class MemArg(val offset: Int, val bytesConsumed: Int)

private data class BlockType(val value: Int, val bytesConsumed: Int)

private data class BlockBounds(val elsePc: Int?, val endPc: Int)

private data class Label(val stackHeight: Int, val branchArity: Int)

private class BranchSignal(val depth: Int) : RuntimeException() {
    override fun fillInStackTrace(): Throwable = this
}

private class ReturnSignal : RuntimeException() {
    override fun fillInStackTrace(): Throwable = this
}

class WasmExecutionEngine(
    private val memory: LinearMemory?,
    private val tables: List<Table>,
    private val globals: MutableList<WasmValue>,
    private val globalTypes: MutableList<GlobalType>,
    private val funcTypes: List<FuncType>,
    private val funcBodies: List<FunctionBody?>,
    private val hostFunctions: List<TypedHostFunction?>,
) {
    fun callFunction(funcIndex: Int, args: List<WasmValue>): List<WasmValue> {
        val funcType = requireFunctionType(funcIndex)
        if (args.size != funcType.params.size) {
            throw TrapError("function $funcIndex expects ${funcType.params.size} arguments, got ${args.size}")
        }

        hostFunctions.getOrNull(funcIndex)?.let { hostFunction ->
            return hostFunction.call(args)
        }

        val body = funcBodies.getOrNull(funcIndex) ?: throw TrapError("no body for function $funcIndex")
        val locals = (args + body.locals.map { defaultValue(it) }).toMutableList()
        val stack = ArrayDeque<WasmValue>()
        val labels = mutableListOf<Label>()
        try {
            executeRange(body.code, 0, body.code.size, stack, locals, labels)
        } catch (_: ReturnSignal) {
            // Early return preserves the stack as the result source.
        }
        return collectResults(stack, funcType.results.size)
    }

    private fun executeRange(
        code: ByteArray,
        startPc: Int,
        endPc: Int,
        stack: ArrayDeque<WasmValue>,
        locals: MutableList<WasmValue>,
        labels: MutableList<Label>,
    ) {
        var pc = startPc
        while (pc < endPc) {
            when (val opcode = code[pc++].toInt() and 0xFF) {
                0x00 -> throw TrapError("unreachable instruction executed")
                0x01 -> Unit
                0x02 -> {
                    val blockType = readBlockType(code, pc)
                    val bodyStart = pc + blockType.bytesConsumed
                    val bounds = findBlockBounds(code, bodyStart)
                    labels += Label(stack.size, blockResultArity(blockType.value))
                    try {
                        executeRange(code, bodyStart, bounds.endPc, stack, locals, labels)
                    } catch (signal: BranchSignal) {
                        if (signal.depth != 0) throw BranchSignal(signal.depth - 1)
                    } finally {
                        labels.removeLast()
                    }
                    pc = bounds.endPc + 1
                }
                0x03 -> {
                    val blockType = readBlockType(code, pc)
                    val bodyStart = pc + blockType.bytesConsumed
                    val bounds = findBlockBounds(code, bodyStart)
                    while (true) {
                        labels += Label(stack.size, blockParamArity(blockType.value))
                        try {
                            executeRange(code, bodyStart, bounds.endPc, stack, locals, labels)
                            break
                        } catch (signal: BranchSignal) {
                            if (signal.depth != 0) throw BranchSignal(signal.depth - 1)
                        } finally {
                            labels.removeLast()
                        }
                    }
                    pc = bounds.endPc + 1
                }
                0x04 -> {
                    val blockType = readBlockType(code, pc)
                    val bodyStart = pc + blockType.bytesConsumed
                    val bounds = findBlockBounds(code, bodyStart)
                    val condition = asI32(pop(stack)) != 0
                    val branchStart = if (condition) bodyStart else (bounds.elsePc ?: bounds.endPc) + if (bounds.elsePc == null) 0 else 1
                    val branchEnd = if (condition) bounds.elsePc ?: bounds.endPc else bounds.endPc
                    labels += Label(stack.size, blockResultArity(blockType.value))
                    try {
                        executeRange(code, branchStart, branchEnd, stack, locals, labels)
                    } catch (signal: BranchSignal) {
                        if (signal.depth != 0) throw BranchSignal(signal.depth - 1)
                    } finally {
                        labels.removeLast()
                    }
                    pc = bounds.endPc + 1
                }
                0x05 -> throw TrapError("unexpected else")
                0x0B -> return
                0x0C -> {
                    val decoded = readUnsignedLeb(code, pc)
                    pc += decoded.bytesConsumed
                    branchTo(decoded.value.toInt(), stack, labels)
                }
                0x0D -> {
                    val decoded = readUnsignedLeb(code, pc)
                    pc += decoded.bytesConsumed
                    if (asI32(pop(stack)) != 0) {
                        branchTo(decoded.value.toInt(), stack, labels)
                    }
                }
                0x0F -> throw ReturnSignal()
                0x10 -> {
                    val decoded = readUnsignedLeb(code, pc)
                    pc += decoded.bytesConsumed
                    pushAll(stack, callDirect(decoded.value.toInt(), stack))
                }
                0x11 -> {
                    val typeIndex = readUnsignedLeb(code, pc)
                    val tableIndex = readUnsignedLeb(code, pc + typeIndex.bytesConsumed)
                    pc += typeIndex.bytesConsumed + tableIndex.bytesConsumed
                    pushAll(stack, callIndirect(typeIndex.value.toInt(), tableIndex.value.toInt(), stack))
                }
                0x1A -> pop(stack)
                0x1B -> {
                    val condition = asI32(pop(stack))
                    val second = pop(stack)
                    val first = pop(stack)
                    stack.addFirst(if (condition != 0) first else second)
                }
                0x20 -> {
                    val decoded = readUnsignedLeb(code, pc)
                    pc += decoded.bytesConsumed
                    val index = decoded.value.toInt()
                    if (index !in locals.indices) throw TrapError("undefined local index $index")
                    stack.addFirst(locals[index])
                }
                0x21 -> {
                    val decoded = readUnsignedLeb(code, pc)
                    pc += decoded.bytesConsumed
                    val index = decoded.value.toInt()
                    if (index !in locals.indices) throw TrapError("undefined local index $index")
                    locals[index] = pop(stack)
                }
                0x22 -> {
                    val decoded = readUnsignedLeb(code, pc)
                    pc += decoded.bytesConsumed
                    val index = decoded.value.toInt()
                    if (index !in locals.indices) throw TrapError("undefined local index $index")
                    val value = pop(stack)
                    locals[index] = value
                    stack.addFirst(value)
                }
                0x23 -> {
                    val decoded = readUnsignedLeb(code, pc)
                    pc += decoded.bytesConsumed
                    val index = decoded.value.toInt()
                    stack.addFirst(globals.getOrElse(index) { throw TrapError("undefined global index $index") })
                }
                0x24 -> {
                    val decoded = readUnsignedLeb(code, pc)
                    pc += decoded.bytesConsumed
                    val index = decoded.value.toInt()
                    if (index !in globals.indices) throw TrapError("undefined global index $index")
                    if (!globalTypes[index].mutable) throw TrapError("global $index is immutable")
                    globals[index] = pop(stack)
                }
                else -> pc = executeNonControlOpcode(opcode, code, pc, stack)
            }
        }
    }

    private fun executeNonControlOpcode(opcode: Int, code: ByteArray, pc: Int, stack: ArrayDeque<WasmValue>): Int {
        fun load(memArg: MemArg, loader: (Int) -> WasmValue): Int {
            requireMemory()
            stack.addFirst(loader(effectiveAddress(asI32(pop(stack)), memArg.offset)))
            return pc + memArg.bytesConsumed
        }

        fun store(memArg: MemArg, writer: (Int) -> Unit): Int {
            requireMemory()
            writer(effectiveAddress(asI32(pop(stack)), memArg.offset))
            return pc + memArg.bytesConsumed
        }

        return when (opcode) {
            0x28 -> readMemArg(code, pc).let { load(it) { address -> i32(memory!!.loadI32(address)) } }
            0x29 -> readMemArg(code, pc).let { load(it) { address -> i64(memory!!.loadI64(address)) } }
            0x2A -> readMemArg(code, pc).let { load(it) { address -> f32(memory!!.loadF32(address)) } }
            0x2B -> readMemArg(code, pc).let { load(it) { address -> f64(memory!!.loadF64(address)) } }
            0x2C -> readMemArg(code, pc).let { load(it) { address -> i32(memory!!.loadI32_8s(address)) } }
            0x2D -> readMemArg(code, pc).let { load(it) { address -> i32(memory!!.loadI32_8u(address)) } }
            0x2E -> readMemArg(code, pc).let { load(it) { address -> i32(memory!!.loadI32_16s(address)) } }
            0x2F -> readMemArg(code, pc).let { load(it) { address -> i32(memory!!.loadI32_16u(address)) } }
            0x30 -> readMemArg(code, pc).let { load(it) { address -> i64(memory!!.loadI64_8s(address)) } }
            0x31 -> readMemArg(code, pc).let { load(it) { address -> i64(memory!!.loadI64_8u(address)) } }
            0x32 -> readMemArg(code, pc).let { load(it) { address -> i64(memory!!.loadI64_16s(address)) } }
            0x33 -> readMemArg(code, pc).let { load(it) { address -> i64(memory!!.loadI64_16u(address)) } }
            0x34 -> readMemArg(code, pc).let { load(it) { address -> i64(memory!!.loadI64_32s(address)) } }
            0x35 -> readMemArg(code, pc).let { load(it) { address -> i64(memory!!.loadI64_32u(address)) } }
            0x36 -> readMemArg(code, pc).let { memArg -> val value = asI32(pop(stack)); store(memArg) { address -> memory!!.storeI32(address, value) } }
            0x37 -> readMemArg(code, pc).let { memArg -> val value = asI64(pop(stack)); store(memArg) { address -> memory!!.storeI64(address, value) } }
            0x38 -> readMemArg(code, pc).let { memArg -> val value = asF32(pop(stack)); store(memArg) { address -> memory!!.storeF32(address, value) } }
            0x39 -> readMemArg(code, pc).let { memArg -> val value = asF64(pop(stack)); store(memArg) { address -> memory!!.storeF64(address, value) } }
            0x3A -> readMemArg(code, pc).let { memArg -> val value = asI32(pop(stack)); store(memArg) { address -> memory!!.storeI32_8(address, value) } }
            0x3B -> readMemArg(code, pc).let { memArg -> val value = asI32(pop(stack)); store(memArg) { address -> memory!!.storeI32_16(address, value) } }
            0x3C -> readMemArg(code, pc).let { memArg -> val value = asI64(pop(stack)); store(memArg) { address -> memory!!.storeI64_8(address, value) } }
            0x3D -> readMemArg(code, pc).let { memArg -> val value = asI64(pop(stack)); store(memArg) { address -> memory!!.storeI64_16(address, value) } }
            0x3E -> readMemArg(code, pc).let { memArg -> val value = asI64(pop(stack)); store(memArg) { address -> memory!!.storeI64_32(address, value) } }
            else -> executeNumericOpcode(opcode, code, pc, stack)
        }
    }

    private fun executeNumericOpcode(opcode: Int, code: ByteArray, pc: Int, stack: ArrayDeque<WasmValue>): Int =
        when (opcode) {
            0x3F -> {
                requireMemory()
                pc + readZeroByteImmediate(code, pc).also { stack.addFirst(i32(memory!!.size())) }
            }
            0x40 -> {
                requireMemory()
                pc + readZeroByteImmediate(code, pc).also { stack.addFirst(i32(memory!!.grow(asI32(pop(stack))))) }
            }
            0x41 -> readSignedLeb32(code, pc).let { stack.addFirst(i32(it.value)); pc + it.bytesConsumed }
            0x42 -> readSignedLeb64(code, pc).let { stack.addFirst(i64(it.value)); pc + it.bytesConsumed }
            0x43 -> {
                ensureRemaining(code, pc, 4)
                stack.addFirst(f32(ByteBuffer.wrap(code, pc, 4).order(ByteOrder.LITTLE_ENDIAN).float))
                pc + 4
            }
            0x44 -> {
                ensureRemaining(code, pc, 8)
                stack.addFirst(f64(ByteBuffer.wrap(code, pc, 8).order(ByteOrder.LITTLE_ENDIAN).double))
                pc + 8
            }
            0x45 -> {
                stack.addFirst(i32(if (asI32(pop(stack)) == 0) 1 else 0))
                pc
            }
            0x46 -> {
                compareI32(stack) { left, right -> left == right }
                pc
            }
            0x47 -> {
                compareI32(stack) { left, right -> left != right }
                pc
            }
            0x48 -> {
                compareI32(stack) { left, right -> left < right }
                pc
            }
            0x49 -> {
                compareI32(stack) { left, right -> Integer.compareUnsigned(left, right) < 0 }
                pc
            }
            0x4A -> {
                compareI32(stack) { left, right -> left > right }
                pc
            }
            0x4B -> {
                compareI32(stack) { left, right -> Integer.compareUnsigned(left, right) > 0 }
                pc
            }
            0x4C -> {
                compareI32(stack) { left, right -> left <= right }
                pc
            }
            0x4D -> {
                compareI32(stack) { left, right -> Integer.compareUnsigned(left, right) <= 0 }
                pc
            }
            0x4E -> {
                compareI32(stack) { left, right -> left >= right }
                pc
            }
            0x4F -> {
                compareI32(stack) { left, right -> Integer.compareUnsigned(left, right) >= 0 }
                pc
            }
            0x67 -> {
                stack.addFirst(i32(Integer.numberOfLeadingZeros(asI32(pop(stack)))))
                pc
            }
            0x68 -> {
                stack.addFirst(i32(Integer.numberOfTrailingZeros(asI32(pop(stack)))))
                pc
            }
            0x69 -> {
                stack.addFirst(i32(Integer.bitCount(asI32(pop(stack)))))
                pc
            }
            0x6A -> binaryI32(stack, Int::plus).let { pc }
            0x6B -> binaryI32(stack) { left, right -> left - right }.let { pc }
            0x6C -> binaryI32(stack) { left, right -> left * right }.let { pc }
            0x6D -> {
                val right = asI32(pop(stack))
                val left = asI32(pop(stack))
                if (right == 0) throw TrapError("integer divide by zero")
                if (left == Int.MIN_VALUE && right == -1) throw TrapError("integer overflow")
                stack.addFirst(i32(left / right))
                pc
            }
            0x6E -> {
                val right = asI32(pop(stack))
                val left = asI32(pop(stack))
                if (right == 0) throw TrapError("integer divide by zero")
                stack.addFirst(i32(Integer.divideUnsigned(left, right)))
                pc
            }
            0x6F -> {
                val right = asI32(pop(stack))
                val left = asI32(pop(stack))
                if (right == 0) throw TrapError("integer divide by zero")
                stack.addFirst(i32(left % right))
                pc
            }
            0x70 -> {
                val right = asI32(pop(stack))
                val left = asI32(pop(stack))
                if (right == 0) throw TrapError("integer divide by zero")
                stack.addFirst(i32(Integer.remainderUnsigned(left, right)))
                pc
            }
            0x71 -> binaryI32(stack) { left, right -> left and right }.let { pc }
            0x72 -> binaryI32(stack) { left, right -> left or right }.let { pc }
            0x73 -> binaryI32(stack) { left, right -> left xor right }.let { pc }
            0x74 -> binaryI32(stack) { left, right -> left shl (right and 31) }.let { pc }
            0x75 -> binaryI32(stack) { left, right -> left shr (right and 31) }.let { pc }
            0x76 -> binaryI32(stack) { left, right -> left ushr (right and 31) }.let { pc }
            0x77 -> binaryI32(stack, Integer::rotateLeft).let { pc }
            0x78 -> binaryI32(stack, Integer::rotateRight).let { pc }
            else -> throw TrapError("unsupported opcode 0x${opcode.toString(16)}")
        }

    private fun callDirect(funcIndex: Int, stack: ArrayDeque<WasmValue>): List<WasmValue> {
        val funcType = requireFunctionType(funcIndex)
        val args = buildList {
            repeat(funcType.params.size) {
                add(0, pop(stack))
            }
        }
        return callFunction(funcIndex, args)
    }

    private fun callIndirect(expectedTypeIndex: Int, tableIndex: Int, stack: ArrayDeque<WasmValue>): List<WasmValue> {
        val table = tables.getOrNull(tableIndex) ?: throw TrapError("undefined table index $tableIndex")
        val elementIndex = asI32(pop(stack))
        val funcIndex = table.get(elementIndex) ?: throw TrapError("uninitialized table element")
        val expected = funcTypes.getOrNull(expectedTypeIndex) ?: throw TrapError("undefined type")
        val actual = requireFunctionType(funcIndex)
        if (expected != actual) {
            throw TrapError("indirect call type mismatch")
        }
        return callDirect(funcIndex, stack)
    }

    private fun requireFunctionType(funcIndex: Int): FuncType =
        funcTypes.getOrNull(funcIndex) ?: throw TrapError("undefined function index $funcIndex")

    private fun requireMemory() {
        if (memory == null) {
            throw TrapError("no linear memory")
        }
    }

    private fun branchTo(depth: Int, stack: ArrayDeque<WasmValue>, labels: MutableList<Label>): Nothing {
        val labelIndex = labels.size - 1 - depth
        if (labelIndex < 0) throw TrapError("branch target $depth out of range")
        val target = labels[labelIndex]
        val carried =
            MutableList(target.branchArity) { i32(0) }.also {
                for (index in target.branchArity - 1 downTo 0) {
                    it[index] = pop(stack)
                }
            }
        while (stack.size > target.stackHeight) {
            pop(stack)
        }
        pushAll(stack, carried)
        throw BranchSignal(depth)
    }

    private fun binaryI32(stack: ArrayDeque<WasmValue>, operation: (Int, Int) -> Int) {
        val right = asI32(pop(stack))
        val left = asI32(pop(stack))
        stack.addFirst(i32(operation(left, right)))
    }

    private fun compareI32(stack: ArrayDeque<WasmValue>, comparison: (Int, Int) -> Boolean) {
        val right = asI32(pop(stack))
        val left = asI32(pop(stack))
        stack.addFirst(i32(if (comparison(left, right)) 1 else 0))
    }

    private fun pushAll(stack: ArrayDeque<WasmValue>, values: List<WasmValue>) {
        values.forEach { stack.addFirst(it) }
    }

    private fun collectResults(stack: ArrayDeque<WasmValue>, resultCount: Int): List<WasmValue> {
        val results = MutableList(resultCount) { i32(0) }
        for (index in resultCount - 1 downTo 0) {
            results[index] = pop(stack)
        }
        return results
    }

    private fun pop(stack: ArrayDeque<WasmValue>): WasmValue =
        stack.removeFirstOrNull() ?: throw TrapError("operand stack underflow")
}

private fun asI32(value: WasmValue): Int {
    if (value.type != ValueType.I32) throw TrapError("expected i32 but found ${value.type}")
    return (value.value as Number).toInt()
}

private fun asI64(value: WasmValue): Long {
    if (value.type != ValueType.I64) throw TrapError("expected i64 but found ${value.type}")
    return (value.value as Number).toLong()
}

private fun asF32(value: WasmValue): Float {
    if (value.type != ValueType.F32) throw TrapError("expected f32 but found ${value.type}")
    return (value.value as Number).toFloat()
}

private fun asF64(value: WasmValue): Double {
    if (value.type != ValueType.F64) throw TrapError("expected f64 but found ${value.type}")
    return (value.value as Number).toDouble()
}

private fun readUnsignedLeb(code: ByteArray, offset: Int): UnsignedLeb {
    var result = 0L
    var shift = 0
    var bytesConsumed = 0
    while (offset + bytesConsumed < code.size) {
        val current = code[offset + bytesConsumed].toInt() and 0xFF
        result = result or ((current and 0x7F).toLong() shl shift)
        bytesConsumed++
        if ((current and 0x80) == 0) return UnsignedLeb(result, bytesConsumed)
        shift += 7
    }
    throw TrapError("unterminated unsigned LEB128 immediate")
}

private fun readSignedLeb32(code: ByteArray, offset: Int): SignedLeb32 {
    var result = 0
    var shift = 0
    var bytesConsumed = 0
    var current: Int
    do {
        if (offset + bytesConsumed >= code.size) throw TrapError("unterminated signed LEB128 immediate")
        current = code[offset + bytesConsumed].toInt() and 0xFF
        result = result or ((current and 0x7F) shl shift)
        shift += 7
        bytesConsumed++
    } while ((current and 0x80) != 0)
    if (shift < 32 && (current and 0x40) != 0) {
        result = result or (-1 shl shift)
    }
    return SignedLeb32(result, bytesConsumed)
}

private fun readSignedLeb64(code: ByteArray, offset: Int): SignedLeb64 {
    var result = 0L
    var shift = 0
    var bytesConsumed = 0
    var current: Int
    do {
        if (offset + bytesConsumed >= code.size) throw TrapError("unterminated signed LEB128 immediate")
        current = code[offset + bytesConsumed].toInt() and 0xFF
        result = result or ((current and 0x7F).toLong() shl shift)
        shift += 7
        bytesConsumed++
    } while ((current and 0x80) != 0)
    if (shift < 64 && (current and 0x40) != 0) {
        result = result or (-1L shl shift)
    }
    return SignedLeb64(result, bytesConsumed)
}

private fun readMemArg(code: ByteArray, offset: Int): MemArg {
    val align = readUnsignedLeb(code, offset)
    val memOffset = readUnsignedLeb(code, offset + align.bytesConsumed)
    return MemArg(memOffset.value.toInt(), align.bytesConsumed + memOffset.bytesConsumed)
}

private fun readBlockType(code: ByteArray, offset: Int): BlockType {
    ensureRemaining(code, offset, 1)
    val first = code[offset].toInt() and 0xFF
    return if (first == BLOCK_TYPE_EMPTY || isValueTypeByte(first)) {
        BlockType(first, 1)
    } else {
        readSignedLeb32(code, offset).let { BlockType(it.value, it.bytesConsumed) }
    }
}

private fun findBlockBounds(code: ByteArray, offset: Int): BlockBounds {
    var depth = 1
    var elsePc: Int? = null
    var pc = offset
    while (pc < code.size) {
        when (val opcode = code[pc++].toInt() and 0xFF) {
            0x02, 0x03, 0x04 -> {
                val blockType = readBlockType(code, pc)
                pc += blockType.bytesConsumed
                depth++
            }
            0x05 -> if (depth == 1 && elsePc == null) elsePc = pc - 1
            0x0B -> {
                depth--
                if (depth == 0) return BlockBounds(elsePc, pc - 1)
            }
            else -> pc = skipImmediate(code, pc, opcode)
        }
    }
    throw TrapError("unterminated structured control block")
}

private fun skipImmediate(code: ByteArray, offset: Int, opcode: Int): Int =
    when (opcode) {
        0x0C, 0x0D, 0x10, 0x20, 0x21, 0x22, 0x23, 0x24 -> offset + readUnsignedLeb(code, offset).bytesConsumed
        0x11 -> {
            val typeIndex = readUnsignedLeb(code, offset)
            val tableIndex = readUnsignedLeb(code, offset + typeIndex.bytesConsumed)
            offset + typeIndex.bytesConsumed + tableIndex.bytesConsumed
        }
        in 0x28..0x3E -> offset + readMemArg(code, offset).bytesConsumed
        0x3F, 0x40 -> offset + readZeroByteImmediate(code, offset)
        0x41 -> offset + readSignedLeb32(code, offset).bytesConsumed
        0x42 -> offset + readSignedLeb64(code, offset).bytesConsumed
        0x43 -> offset + 4
        0x44 -> offset + 8
        else -> offset
    }

private fun readZeroByteImmediate(code: ByteArray, offset: Int): Int {
    ensureRemaining(code, offset, 1)
    if ((code[offset].toInt() and 0xFF) != 0) throw TrapError("expected zero-byte memory immediate")
    return 1
}

private fun effectiveAddress(base: Int, offset: Int): Int {
    val address = Integer.toUnsignedLong(base) + Integer.toUnsignedLong(offset)
    if (address > Int.MAX_VALUE.toLong()) throw TrapError("memory access out of bounds at address $address")
    return address.toInt()
}

private fun blockResultArity(blockType: Int): Int =
    when {
        blockType == BLOCK_TYPE_EMPTY -> 0
        isValueTypeByte(blockType) -> 1
        else -> 0
    }

private fun blockParamArity(blockType: Int): Int =
    when {
        blockType == BLOCK_TYPE_EMPTY -> 0
        isValueTypeByte(blockType) -> 0
        else -> 0
    }

private fun isValueTypeByte(byteValue: Int): Boolean =
    byteValue == ValueType.I32.code ||
        byteValue == ValueType.I64.code ||
        byteValue == ValueType.F32.code ||
        byteValue == ValueType.F64.code

private fun ensureRemaining(code: ByteArray, offset: Int, length: Int) {
    if (offset < 0 || offset + length > code.size) {
        throw TrapError("unexpected end of bytecode")
    }
}
