package com.codingadventures.wasmmoduleparser

import com.codingadventures.wasmleb128.WasmLeb128
import com.codingadventures.wasmtypes.CustomSection
import com.codingadventures.wasmtypes.DataSegment
import com.codingadventures.wasmtypes.Element
import com.codingadventures.wasmtypes.Export
import com.codingadventures.wasmtypes.ExternalKind
import com.codingadventures.wasmtypes.FUNCREF
import com.codingadventures.wasmtypes.FunctionBody
import com.codingadventures.wasmtypes.Global
import com.codingadventures.wasmtypes.GlobalType
import com.codingadventures.wasmtypes.Import
import com.codingadventures.wasmtypes.Limits
import com.codingadventures.wasmtypes.MemoryType
import com.codingadventures.wasmtypes.TableType
import com.codingadventures.wasmtypes.ValueType
import com.codingadventures.wasmtypes.WasmModule
import com.codingadventures.wasmtypes.makeFuncType

const val VERSION: String = "0.1.0"

private val WASM_MAGIC = byteArrayOf(0x00, 0x61, 0x73, 0x6D)
private val WASM_VERSION = byteArrayOf(0x01, 0x00, 0x00, 0x00)

private const val SECTION_CUSTOM = 0
private const val SECTION_TYPE = 1
private const val SECTION_IMPORT = 2
private const val SECTION_FUNCTION = 3
private const val SECTION_TABLE = 4
private const val SECTION_MEMORY = 5
private const val SECTION_GLOBAL = 6
private const val SECTION_EXPORT = 7
private const val SECTION_START = 8
private const val SECTION_ELEMENT = 9
private const val SECTION_CODE = 10
private const val SECTION_DATA = 11

private const val FUNC_TYPE_PREFIX = 0x60
private const val END_OPCODE = 0x0B

class WasmParseError(message: String, val offset: Int) : RuntimeException(message)

class WasmModuleParser {
    fun parse(data: ByteArray): WasmModule = BinaryReader(data).parseModule()
}

private class BinaryReader(data: ByteArray) {
    private val data = data.copyOf()
    private var pos = 0

    fun parseModule(): WasmModule {
        validateHeader()
        val module = WasmModule()
        var lastSectionId = 0

        while (!atEnd()) {
            val sectionIdOffset = pos
            val sectionId = readByte()
            val payloadSize = readU32()
            val payloadStart = pos
            val payloadEnd = payloadStart + payloadSize

            if (payloadEnd > data.size) {
                throw WasmParseError(
                    "Section $sectionId payload extends beyond end of data (offset $payloadStart, size $payloadSize)",
                    payloadStart,
                )
            }

            if (sectionId != SECTION_CUSTOM) {
                if (sectionId < lastSectionId) {
                    throw WasmParseError(
                        "Section $sectionId appears out of order: already saw section $lastSectionId",
                        sectionIdOffset,
                    )
                }
                lastSectionId = sectionId
            }

            val payload = data.copyOfRange(payloadStart, payloadEnd)
            when (sectionId) {
                SECTION_TYPE -> parseTypeSection(module)
                SECTION_IMPORT -> parseImportSection(module)
                SECTION_FUNCTION -> parseFunctionSection(module)
                SECTION_TABLE -> parseTableSection(module)
                SECTION_MEMORY -> parseMemorySection(module)
                SECTION_GLOBAL -> parseGlobalSection(module)
                SECTION_EXPORT -> parseExportSection(module)
                SECTION_START -> parseStartSection(module)
                SECTION_ELEMENT -> parseElementSection(module)
                SECTION_CODE -> parseCodeSection(module)
                SECTION_DATA -> parseDataSection(module)
                SECTION_CUSTOM -> parseCustomSection(module, payload)
            }

            pos = payloadEnd
        }

        return module
    }

    private fun validateHeader() {
        if (data.size < 8) {
            throw WasmParseError("File too short: ${data.size} bytes (need at least 8 for the header)", 0)
        }

        for (index in 0 until 4) {
            if (data[index] != WASM_MAGIC[index]) {
                throw WasmParseError("Invalid magic bytes at offset $index", index)
            }
        }
        pos = 4

        for (index in 0 until 4) {
            if (data[4 + index] != WASM_VERSION[index]) {
                throw WasmParseError("Unsupported WASM version at offset ${4 + index}", 4 + index)
            }
        }
        pos = 8
    }

    private fun readByte(): Int {
        if (pos >= data.size) {
            throw WasmParseError("Unexpected end of data: expected 1 byte at offset $pos", pos)
        }
        return data[pos++].toInt() and 0xFF
    }

    private fun readBytes(count: Int): ByteArray {
        if (pos + count > data.size) {
            throw WasmParseError("Unexpected end of data: expected $count bytes at offset $pos", pos)
        }
        val slice = data.copyOfRange(pos, pos + count)
        pos += count
        return slice
    }

    private fun readU32(): Int {
        val offset = pos
        return try {
            val decoded = WasmLeb128.decodeUnsigned(data, pos)
            pos += decoded.bytesConsumed
            decoded.value.toInt()
        } catch (error: RuntimeException) {
            throw WasmParseError("Invalid LEB128 at offset $offset: ${error.message}", offset)
        }
    }

    private fun readString(): String = readBytes(readU32()).toString(Charsets.UTF_8)

    private fun readLimits(): Limits {
        val flagsOffset = pos
        val flags = readByte()
        val min = readU32()
        val max =
            when {
                (flags and 1) != 0 -> readU32()
                flags == 0 -> null
                else -> throw WasmParseError("Unknown limits flags byte 0x${flags.toString(16)} at offset $flagsOffset", flagsOffset)
            }
        return Limits(min, max)
    }

    private fun readGlobalType(): GlobalType {
        val typeOffset = pos
        val valueTypeByte = readByte()
        if (!isValidValueType(valueTypeByte)) {
            throw WasmParseError("Unknown value type byte 0x${valueTypeByte.toString(16)} at offset $typeOffset", typeOffset)
        }
        return GlobalType(ValueType.fromByte(valueTypeByte), readByte() != 0)
    }

    private fun readInitExpr(): ByteArray {
        val start = pos
        while (pos < data.size) {
            val current = data[pos++].toInt() and 0xFF
            if (current == END_OPCODE) {
                return data.copyOfRange(start, pos)
            }
        }
        throw WasmParseError("Init expression at offset $start never terminated with 0x0B (end opcode)", start)
    }

    private fun readValueTypeVec(): List<ValueType> {
        val count = readU32()
        return buildList {
            repeat(count) {
                val typeOffset = pos
                val valueTypeByte = readByte()
                if (!isValidValueType(valueTypeByte)) {
                    throw WasmParseError(
                        "Unknown value type byte 0x${valueTypeByte.toString(16)} at offset $typeOffset",
                        typeOffset,
                    )
                }
                add(ValueType.fromByte(valueTypeByte))
            }
        }
    }

    private fun parseTypeSection(module: WasmModule) {
        repeat(readU32()) {
            val prefixOffset = pos
            val prefix = readByte()
            if (prefix != FUNC_TYPE_PREFIX) {
                throw WasmParseError(
                    "Expected function type prefix 0x60 at offset $prefixOffset, got 0x${prefix.toString(16)}",
                    prefixOffset,
                )
            }
            module.types += makeFuncType(readValueTypeVec(), readValueTypeVec())
        }
    }

    private fun parseImportSection(module: WasmModule) {
        repeat(readU32()) {
            val moduleName = readString()
            val name = readString()
            val kindOffset = pos
            val kindByte = readByte()

            val (kind, typeInfo) =
                when (kindByte) {
                    0x00 -> ExternalKind.FUNCTION to readU32()
                    0x01 -> {
                        val elementTypeOffset = pos
                        val elementType = readByte()
                        if (elementType != FUNCREF) {
                            throw WasmParseError(
                                "Unknown table element type 0x${elementType.toString(16)} at offset $elementTypeOffset",
                                elementTypeOffset,
                            )
                        }
                        ExternalKind.TABLE to TableType(elementType, readLimits())
                    }
                    0x02 -> ExternalKind.MEMORY to MemoryType(readLimits())
                    0x03 -> ExternalKind.GLOBAL to readGlobalType()
                    else -> throw WasmParseError("Unknown import kind 0x${kindByte.toString(16)} at offset $kindOffset", kindOffset)
                }

            module.imports += Import(moduleName, name, kind, typeInfo)
        }
    }

    private fun parseFunctionSection(module: WasmModule) {
        repeat(readU32()) {
            module.functions += readU32()
        }
    }

    private fun parseTableSection(module: WasmModule) {
        repeat(readU32()) {
            val elementTypeOffset = pos
            val elementType = readByte()
            if (elementType != FUNCREF) {
                throw WasmParseError(
                    "Unknown table element type 0x${elementType.toString(16)} at offset $elementTypeOffset",
                    elementTypeOffset,
                )
            }
            module.tables += TableType(elementType, readLimits())
        }
    }

    private fun parseMemorySection(module: WasmModule) {
        repeat(readU32()) {
            module.memories += MemoryType(readLimits())
        }
    }

    private fun parseGlobalSection(module: WasmModule) {
        repeat(readU32()) {
            module.globals += Global(readGlobalType(), readInitExpr())
        }
    }

    private fun parseExportSection(module: WasmModule) {
        repeat(readU32()) {
            val name = readString()
            val kindOffset = pos
            val kindByte = readByte()
            val kind =
                try {
                    ExternalKind.fromByte(kindByte)
                } catch (_: IllegalArgumentException) {
                    throw WasmParseError("Unknown export kind 0x${kindByte.toString(16)} at offset $kindOffset", kindOffset)
                }
            module.exports += Export(name, kind, readU32())
        }
    }

    private fun parseStartSection(module: WasmModule) {
        module.start = readU32()
    }

    private fun parseElementSection(module: WasmModule) {
        repeat(readU32()) {
            val tableIndex = readU32()
            val offsetExpr = readInitExpr()
            val functionCount = readU32()
            val functionIndices = buildList {
                repeat(functionCount) {
                    add(readU32())
                }
            }
            module.elements += Element(tableIndex, offsetExpr, functionIndices)
        }
    }

    private fun parseCodeSection(module: WasmModule) {
        repeat(readU32()) { bodyIndex ->
            val bodySize = readU32()
            val bodyStart = pos
            val bodyEnd = bodyStart + bodySize

            if (bodyEnd > data.size) {
                throw WasmParseError(
                    "Code body $bodyIndex extends beyond end of data (offset $bodyStart, size $bodySize)",
                    bodyStart,
                )
            }

            val localDeclCount = readU32()
            val locals = mutableListOf<ValueType>()
            repeat(localDeclCount) {
                val groupCount = readU32()
                val typeOffset = pos
                val typeByte = readByte()
                if (!isValidValueType(typeByte)) {
                    throw WasmParseError("Unknown local type byte 0x${typeByte.toString(16)} at offset $typeOffset", typeOffset)
                }
                repeat(groupCount) {
                    locals += ValueType.fromByte(typeByte)
                }
            }

            val codeLength = bodyEnd - pos
            if (codeLength < 0) {
                throw WasmParseError("Code body $bodyIndex local declarations exceeded body size at offset $pos", pos)
            }

            module.code += FunctionBody(locals, readBytes(codeLength))
        }
    }

    private fun parseDataSection(module: WasmModule) {
        repeat(readU32()) {
            val memoryIndex = readU32()
            val offsetExpr = readInitExpr()
            module.data += DataSegment(memoryIndex, offsetExpr, readBytes(readU32()))
        }
    }

    private fun parseCustomSection(module: WasmModule, payload: ByteArray) {
        val reader = BinaryReader(payload)
        val name = reader.readString()
        module.customs += CustomSection(name, reader.readBytes(payload.size - reader.pos))
    }

    private fun atEnd(): Boolean = pos >= data.size

    private fun isValidValueType(valueType: Int): Boolean =
        valueType == ValueType.I32.code ||
            valueType == ValueType.I64.code ||
            valueType == ValueType.F32.code ||
            valueType == ValueType.F64.code
}
