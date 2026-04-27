package com.codingadventures.wasmsimulator

import java.nio.ByteBuffer
import java.nio.ByteOrder

const val VERSION: String = "0.1.0"

const val OP_END = 0x0B
const val OP_LOCAL_GET = 0x20
const val OP_LOCAL_SET = 0x21
const val OP_I32_CONST = 0x41
const val OP_I32_ADD = 0x6A
const val OP_I32_SUB = 0x6B

data class WasmInstruction(val opcode: Int, val mnemonic: String, val operand: Int?, val size: Int)

data class WasmStepTrace(
    val pc: Int,
    val instruction: WasmInstruction,
    val stackBefore: List<Int>,
    val stackAfter: List<Int>,
    val localsSnapshot: List<Int>,
    val description: String,
    val halted: Boolean,
)

class WasmDecoder {
    fun decode(bytecode: ByteArray, pc: Int): WasmInstruction {
        val opcode = bytecode[pc].toInt() and 0xFF
        return when (opcode) {
            OP_I32_CONST ->
                WasmInstruction(
                    opcode = opcode,
                    mnemonic = "i32.const",
                    operand = ByteBuffer.wrap(bytecode, pc + 1, 4).order(ByteOrder.LITTLE_ENDIAN).int,
                    size = 5,
                )
            OP_I32_ADD -> WasmInstruction(opcode, "i32.add", null, 1)
            OP_I32_SUB -> WasmInstruction(opcode, "i32.sub", null, 1)
            OP_LOCAL_GET -> WasmInstruction(opcode, "local.get", bytecode[pc + 1].toInt() and 0xFF, 2)
            OP_LOCAL_SET -> WasmInstruction(opcode, "local.set", bytecode[pc + 1].toInt() and 0xFF, 2)
            OP_END -> WasmInstruction(opcode, "end", null, 1)
            else -> error("Unknown WASM opcode: 0x${opcode.toString(16).uppercase()} at PC=$pc")
        }
    }
}

class WasmSimulator(localCount: Int) {
    private val bytecodeStack = ArrayDeque<Int>()
    private var bytecode = ByteArray(0)
    var pc = 0
        private set
    var cycle = 0
        private set
    var halted: Boolean = false
        private set

    val locals: IntArray = IntArray(localCount)

    val stack: List<Int>
        get() = bytecodeStack.reversed()

    fun load(program: ByteArray) {
        bytecode = program.copyOf()
        pc = 0
        cycle = 0
        halted = false
        bytecodeStack.clear()
        locals.fill(0)
    }

    fun step(): WasmStepTrace {
        check(!halted) { "simulator is halted" }

        val instruction = WasmDecoder().decode(bytecode, pc)
        val stackBefore = stack
        val description =
            when (instruction.opcode) {
                OP_I32_CONST -> {
                    bytecodeStack.addFirst(requireNotNull(instruction.operand))
                    "push ${instruction.operand}"
                }
                OP_I32_ADD -> {
                    val right = pop()
                    val left = pop()
                    val result = left + right
                    bytecodeStack.addFirst(result)
                    "pop $right and $left, push $result"
                }
                OP_I32_SUB -> {
                    val right = pop()
                    val left = pop()
                    val result = left - right
                    bytecodeStack.addFirst(result)
                    "pop $right and $left, push $result"
                }
                OP_LOCAL_GET -> {
                    val index = requireNotNull(instruction.operand)
                    bytecodeStack.addFirst(locals[index])
                    "push local[$index]"
                }
                OP_LOCAL_SET -> {
                    val index = requireNotNull(instruction.operand)
                    locals[index] = pop()
                    "store into local[$index]"
                }
                OP_END -> {
                    halted = true
                    "halt"
                }
                else -> error("Unsupported opcode ${instruction.opcode}")
            }

        val currentPc = pc
        pc += instruction.size
        cycle += 1
        return WasmStepTrace(
            pc = currentPc,
            instruction = instruction,
            stackBefore = stackBefore,
            stackAfter = stack,
            localsSnapshot = locals.toList(),
            description = description,
            halted = halted,
        )
    }

    fun run(program: ByteArray): List<WasmStepTrace> {
        load(program)
        val traces = mutableListOf<WasmStepTrace>()
        while (!halted) {
            traces += step()
        }
        return traces
    }

    fun reset() {
        bytecode = ByteArray(0)
        pc = 0
        cycle = 0
        halted = false
        bytecodeStack.clear()
        locals.fill(0)
    }

    private fun pop(): Int = bytecodeStack.removeFirstOrNull() ?: error("stack underflow")
}

fun encodeI32Const(value: Int): ByteArray = ByteBuffer.allocate(5).order(ByteOrder.LITTLE_ENDIAN).put(OP_I32_CONST.toByte()).putInt(value).array()

fun encodeI32Add(): ByteArray = byteArrayOf(OP_I32_ADD.toByte())

fun encodeI32Sub(): ByteArray = byteArrayOf(OP_I32_SUB.toByte())

fun encodeLocalGet(index: Int): ByteArray = byteArrayOf(OP_LOCAL_GET.toByte(), index.toByte())

fun encodeLocalSet(index: Int): ByteArray = byteArrayOf(OP_LOCAL_SET.toByte(), index.toByte())

fun encodeEnd(): ByteArray = byteArrayOf(OP_END.toByte())

fun assembleWasm(instructions: List<ByteArray>): ByteArray {
    val output = ArrayList<Byte>(instructions.sumOf { it.size })
    instructions.forEach { output.addAll(it.toList()) }
    return output.toByteArray()
}
