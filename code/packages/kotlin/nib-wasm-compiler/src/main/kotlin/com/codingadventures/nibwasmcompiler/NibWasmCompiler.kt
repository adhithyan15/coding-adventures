package com.codingadventures.nibwasmcompiler

import java.io.ByteArrayOutputStream
import java.nio.file.Files
import java.nio.file.Path

data class NibFunction(val name: String, val params: List<String>, val expression: String)

data class PackageResult(
    val source: String,
    val functions: List<NibFunction>,
    val wasmBytes: ByteArray,
    val wasmPath: Path? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is PackageResult &&
            source == other.source &&
            functions == other.functions &&
            wasmBytes.contentEquals(other.wasmBytes) &&
            wasmPath == other.wasmPath

    override fun hashCode(): Int = 31 * (31 * source.hashCode() + functions.hashCode()) + wasmBytes.contentHashCode()
}

class PackageError(val stage: String, message: String) : RuntimeException(message)

object NibWasmCompiler {
    const val VERSION = "0.1.0"
    private const val MAX_SOURCE_LENGTH = 1_000_000
    private const val MAX_EXPR_NESTING = 256
    private val functionRegex =
        Regex("""fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*->\s*u4\s*\{\s*return\s+([^;]+);\s*\}""", RegexOption.DOT_MATCHES_ALL)

    fun compileSource(source: String): PackageResult {
        val functions = parse(source)
        validate(functions)
        return PackageResult(source, functions, emitModule(functions))
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

    private fun parse(source: String): List<NibFunction> {
        if (source.length > MAX_SOURCE_LENGTH) {
            throw PackageError("parse", "source exceeds $MAX_SOURCE_LENGTH characters")
        }
        val functions = mutableListOf<NibFunction>()
        var cursor = 0
        for (match in functionRegex.findAll(source)) {
            if (source.substring(cursor, match.range.first).trim().isNotEmpty()) {
                throw PackageError("parse", "unexpected text before function")
            }
            functions += NibFunction(match.groupValues[1], parseParams(match.groupValues[2]), match.groupValues[3].trim())
            cursor = match.range.last + 1
        }
        if (source.substring(cursor).trim().isNotEmpty() || functions.isEmpty()) {
            throw PackageError("parse", "expected one or more Nib functions")
        }
        return functions
    }

    private fun parseParams(text: String): List<String> {
        if (text.trim().isEmpty()) return emptyList()
        return text.split(",").map { piece ->
            val parts = piece.trim().split(Regex("""\s*:\s*"""))
            if (parts.size != 2 || parts[1] != "u4" || !Regex("""[A-Za-z_][A-Za-z0-9_]*""").matches(parts[0])) {
                throw PackageError("parse", "parameters must be `name: u4`")
            }
            parts[0]
        }
    }

    private fun validate(functions: List<NibFunction>) {
        val byName = functions.associateBy { it.name }
        if (byName.size != functions.size) throw PackageError("validate", "duplicate function")
        for (function in functions) {
            emitExpr(Section(), function.expression, byName, function.params.withIndex().associate { it.value to it.index }, false, 0)
        }
    }

    private fun emitModule(functions: List<NibFunction>): ByteArray {
        val module = Section()
        module.write(byteArrayOf(0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00))

        val types = Section()
        types.u32(functions.size)
        for (function in functions) types.funcType(function.params.size, 1)
        module.write(section(1, types.bytes()))

        val functionSection = Section()
        functionSection.u32(functions.size)
        functions.indices.forEach { functionSection.u32(it) }
        module.write(section(3, functionSection.bytes()))

        val exports = Section()
        exports.u32(functions.size)
        functions.forEachIndexed { index, function -> exports.export(function.name, 0x00, index) }
        module.write(section(7, exports.bytes()))

        val byName = functions.associateBy { it.name }
        val code = Section()
        code.u32(functions.size)
        for (function in functions) {
            val body = Section()
            body.u32(0)
            emitExpr(body, function.expression, byName, function.params.withIndex().associate { it.value to it.index }, true, 0)
            body.write(0x0b)
            val bytes = body.bytes()
            code.u32(bytes.size)
            code.write(bytes)
        }
        module.write(section(10, code.bytes()))
        return module.bytes()
    }

    private fun emitExpr(out: Section, expression: String, functions: Map<String, NibFunction>, params: Map<String, Int>, emit: Boolean, depth: Int) {
        if (depth > MAX_EXPR_NESTING) {
            throw PackageError("validate", "expression nesting exceeds $MAX_EXPR_NESTING")
        }
        val addParts = splitTopLevel(expression, "+%")
        if (addParts.size > 1) {
            emitExpr(out, addParts.first(), functions, params, emit, depth + 1)
            addParts.drop(1).forEach {
                emitExpr(out, it, functions, params, emit, depth + 1)
                if (emit) {
                    out.write(0x6a)
                    out.i32(15)
                    out.write(0x71)
                }
            }
            return
        }
        val trimmed = expression.trim()
        val literal = trimmed.toIntOrNull()
        if (literal != null) {
            if (literal !in 0..15) throw PackageError("validate", "u4 literal out of range: $literal")
            if (emit) out.i32(literal)
            return
        }
        val call = Regex("""([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)""").matchEntire(trimmed)
        if (call != null) {
            val target = functions[call.groupValues[1]] ?: throw PackageError("validate", "unknown function `${call.groupValues[1]}`")
            val args = splitArgs(call.groupValues[2])
            if (args.size != target.params.size) throw PackageError("validate", "wrong arity for `${target.name}`")
            args.forEach { emitExpr(out, it, functions, params, emit, depth + 1) }
            if (emit) {
                out.write(0x10)
                out.u32(functions.keys.indexOf(target.name))
            }
            return
        }
        val paramIndex = params[trimmed]
        if (paramIndex != null) {
            if (emit) {
                out.write(0x20)
                out.u32(paramIndex)
            }
            return
        }
        throw PackageError("validate", "unsupported expression `$expression`")
    }

    private fun splitArgs(text: String): List<String> = if (text.trim().isEmpty()) emptyList() else splitTopLevel(text, ",")

    private fun splitTopLevel(text: String, delimiter: String): List<String> {
        val parts = mutableListOf<String>()
        var depth = 0
        var start = 0
        var index = 0
        while (index < text.length) {
            when (text[index]) {
                '(' -> depth++
                ')' -> depth--
            }
            if (depth == 0 && text.startsWith(delimiter, index)) {
                parts += text.substring(start, index).trim()
                start = index + delimiter.length
                index += delimiter.length
                continue
            }
            index++
        }
        parts += text.substring(start).trim()
        return parts
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
