package com.codingadventures.brainfuckwasmcompiler

import java.io.ByteArrayOutputStream
import java.nio.file.Files
import java.nio.file.Path

data class PackageResult(
    val source: String,
    val operations: List<Char>,
    val wasmBytes: ByteArray,
    val wasmPath: Path? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is PackageResult &&
            source == other.source &&
            operations == other.operations &&
            wasmBytes.contentEquals(other.wasmBytes) &&
            wasmPath == other.wasmPath

    override fun hashCode(): Int = 31 * (31 * source.hashCode() + operations.hashCode()) + wasmBytes.contentHashCode()
}

class PackageError(val stage: String, message: String) : RuntimeException(message)

private data class ParsedProgram(val operations: List<Char>, val loopEnds: Map<Int, Int>)

object BrainfuckWasmCompiler {
    const val VERSION = "0.1.0"
    private const val MAX_SOURCE_LENGTH = 1_000_000
    private const val MAX_LOOP_NESTING = 512

    fun compileSource(source: String): PackageResult {
        val program = parse(source)
        return PackageResult(source, program.operations, emitModule(program))
    }

    fun packSource(source: String): PackageResult = compileSource(source)

    fun writeWasmFile(source: String, path: Path): PackageResult {
        val result = compileSource(source)
        try {
            Files.write(path, result.wasmBytes)
        } catch (error: java.io.IOException) {
            throw PackageError("write", error.message ?: "write failed")
        }
        return result.copy(wasmPath = path)
    }

    private fun parse(source: String): ParsedProgram {
        if (source.length > MAX_SOURCE_LENGTH) {
            throw PackageError("parse", "source exceeds $MAX_SOURCE_LENGTH characters")
        }
        val ops = mutableListOf<Char>()
        val stack = ArrayDeque<Int>()
        val loopEnds = mutableMapOf<Int, Int>()
        source.forEachIndexed { index, ch ->
            if (ch !in "><+-.,[]") return@forEachIndexed
            val opIndex = ops.size
            when (ch) {
                '[' -> {
                    stack.addLast(opIndex)
                    if (stack.size > MAX_LOOP_NESTING) {
                        throw PackageError("parse", "loop nesting exceeds $MAX_LOOP_NESTING")
                    }
                }
                ']' -> {
                    if (stack.isEmpty()) throw PackageError("parse", "unmatched ] at byte $index")
                    loopEnds[stack.removeLast()] = opIndex
                }
            }
            ops += ch
        }
        if (stack.isNotEmpty()) throw PackageError("parse", "unmatched [")
        return ParsedProgram(ops.toList(), loopEnds.toMap())
    }

    private fun emitModule(program: ParsedProgram): ByteArray {
        val operations = program.operations
        val needsWrite = '.' in operations
        val needsRead = ',' in operations
        val importCount = (if (needsWrite) 1 else 0) + (if (needsRead) 1 else 0)
        val writeIndex = if (needsWrite) 0 else -1
        val readIndex = if (needsRead) if (needsWrite) 1 else 0 else -1
        val startTypeIndex = importCount
        val startFunctionIndex = importCount

        val module = Section()
        module.write(byteArrayOf(0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00))

        val types = Section()
        types.u32(importCount + 1)
        if (needsWrite) types.funcType(4, 1)
        if (needsRead) types.funcType(4, 1)
        types.funcType(0, 0)
        module.write(section(1, types.bytes()))

        if (importCount > 0) {
            val imports = Section()
            imports.u32(importCount)
            if (needsWrite) imports.importFunction("wasi_snapshot_preview1", "fd_write", writeIndex)
            if (needsRead) imports.importFunction("wasi_snapshot_preview1", "fd_read", readIndex)
            module.write(section(2, imports.bytes()))
        }

        val functions = Section()
        functions.u32(1)
        functions.u32(startTypeIndex)
        module.write(section(3, functions.bytes()))

        val memory = Section()
        memory.u32(1)
        memory.write(0x00)
        memory.u32(1)
        module.write(section(5, memory.bytes()))

        val exports = Section()
        exports.u32(2)
        exports.export("_start", 0x00, startFunctionIndex)
        exports.export("memory", 0x02, 0)
        module.write(section(7, exports.bytes()))

        val code = Section()
        val body = functionBody(program, writeIndex, readIndex)
        code.u32(1)
        code.u32(body.size)
        code.write(body)
        module.write(section(10, code.bytes()))
        return module.bytes()
    }

    private fun functionBody(program: ParsedProgram, writeIndex: Int, readIndex: Int): ByteArray {
        val body = Section()
        body.u32(1)
        body.u32(3)
        body.write(0x7f)
        emitOps(body, program.operations, program.loopEnds, 0, program.operations.size, writeIndex, readIndex)
        body.write(0x0b)
        return body.bytes()
    }

    private fun emitOps(out: Section, ops: List<Char>, loopEnds: Map<Int, Int>, start: Int, end: Int, writeIndex: Int, readIndex: Int) {
        var index = start
        while (index < end) {
            when (ops[index]) {
                '>' -> addToLocal(out, 0, 1)
                '<' -> addToLocal(out, 0, -1)
                '+' -> mutateCell(out, 1)
                '-' -> mutateCell(out, -1)
                '.' -> emitWrite(out, writeIndex)
                ',' -> emitRead(out, readIndex)
                '[' -> {
                    val close = loopEnds[index] ?: throw PackageError("encode", "missing loop pair at operation $index")
                    out.write(0x02)
                    out.write(0x40)
                    out.write(0x03)
                    out.write(0x40)
                    loadCell(out)
                    out.write(0x45)
                    out.write(0x0d)
                    out.u32(1)
                    emitOps(out, ops, loopEnds, index + 1, close, writeIndex, readIndex)
                    out.write(0x0c)
                    out.u32(0)
                    out.write(0x0b)
                    out.write(0x0b)
                    index = close
                }
            }
            index++
        }
    }

    private fun loadCell(out: Section) {
        out.write(0x20)
        out.u32(0)
        out.write(0x2d)
        out.u32(0)
        out.u32(0)
    }

    private fun mutateCell(out: Section, delta: Int) {
        loadCell(out)
        out.i32(delta)
        out.write(0x6a)
        out.i32(255)
        out.write(0x71)
        out.write(0x21)
        out.u32(1)
        out.write(0x20)
        out.u32(0)
        out.write(0x20)
        out.u32(1)
        out.write(0x3a)
        out.u32(0)
        out.u32(0)
    }

    private fun addToLocal(out: Section, local: Int, delta: Int) {
        out.write(0x20)
        out.u32(local)
        out.i32(delta)
        out.write(0x6a)
        out.write(0x21)
        out.u32(local)
    }

    private fun emitWrite(out: Section, writeIndex: Int) {
        if (writeIndex < 0) return
        loadCell(out)
        out.write(0x21)
        out.u32(1)
        storeByteLocal(out, 30012, 1)
        storeI32Const(out, 30000, 30012)
        storeI32Const(out, 30004, 1)
        out.i32(1)
        out.i32(30000)
        out.i32(1)
        out.i32(30008)
        out.write(0x10)
        out.u32(writeIndex)
        out.write(0x21)
        out.u32(2)
    }

    private fun emitRead(out: Section, readIndex: Int) {
        if (readIndex < 0) return
        storeByteConst(out, 30012, 0)
        storeI32Const(out, 30000, 30012)
        storeI32Const(out, 30004, 1)
        out.i32(0)
        out.i32(30000)
        out.i32(1)
        out.i32(30008)
        out.write(0x10)
        out.u32(readIndex)
        out.write(0x21)
        out.u32(2)
        out.i32(30012)
        out.write(0x2d)
        out.u32(0)
        out.u32(0)
        out.write(0x21)
        out.u32(1)
        out.write(0x20)
        out.u32(0)
        out.write(0x20)
        out.u32(1)
        out.write(0x3a)
        out.u32(0)
        out.u32(0)
    }

    private fun storeByteLocal(out: Section, address: Int, local: Int) {
        out.i32(address)
        out.write(0x20)
        out.u32(local)
        out.write(0x3a)
        out.u32(0)
        out.u32(0)
    }

    private fun storeByteConst(out: Section, address: Int, value: Int) {
        out.i32(address)
        out.i32(value)
        out.write(0x3a)
        out.u32(0)
        out.u32(0)
    }

    private fun storeI32Const(out: Section, address: Int, value: Int) {
        out.i32(address)
        out.i32(value)
        out.write(0x36)
        out.u32(2)
        out.u32(0)
    }

    private fun section(id: Int, payload: ByteArray): ByteArray =
        Section().apply {
            write(id)
            u32(payload.size)
            write(payload)
        }.bytes()
}

private class Section {
    private val out = ByteArrayOutputStream()

    fun write(value: Int) {
        out.write(value and 0xff)
    }

    fun write(bytes: ByteArray) {
        out.write(bytes)
    }

    fun i32(value: Int) {
        write(0x41)
        s32(value)
    }

    fun u32(value: Int) {
        write(encodeUnsigned(value))
    }

    fun s32(value: Int) {
        write(encodeSigned(value))
    }

    fun funcType(paramCount: Int, resultCount: Int) {
        write(0x60)
        u32(paramCount)
        repeat(paramCount) { write(0x7f) }
        u32(resultCount)
        repeat(resultCount) { write(0x7f) }
    }

    fun importFunction(module: String, name: String, typeIndex: Int) {
        name(module)
        name(name)
        write(0x00)
        u32(typeIndex)
    }

    fun export(name: String, kind: Int, index: Int) {
        name(name)
        write(kind)
        u32(index)
    }

    private fun name(value: String) {
        val bytes = value.encodeToByteArray()
        u32(bytes.size)
        write(bytes)
    }

    fun bytes(): ByteArray = out.toByteArray()
}

private fun encodeUnsigned(value: Int): ByteArray {
    val out = ByteArrayOutputStream()
    var remaining = value
    do {
        var byte = remaining and 0x7f
        remaining = remaining ushr 7
        if (remaining != 0) byte = byte or 0x80
        out.write(byte)
    } while (remaining != 0)
    return out.toByteArray()
}

private fun encodeSigned(value: Int): ByteArray {
    val out = ByteArrayOutputStream()
    var remaining = value
    var more: Boolean
    do {
        var byte = remaining and 0x7f
        remaining = remaining shr 7
        val signBit = byte and 0x40 != 0
        more = !((remaining == 0 && !signBit) || (remaining == -1 && signBit))
        if (more) byte = byte or 0x80
        out.write(byte)
    } while (more)
    return out.toByteArray()
}
