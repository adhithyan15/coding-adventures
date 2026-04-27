package com.codingadventures.wasmtypes

const val BLOCK_TYPE_EMPTY: Int = 0x40
const val FUNCREF: Int = 0x70

enum class ValueType(val code: Int) {
    I32(0x7F),
    I64(0x7E),
    F32(0x7D),
    F64(0x7C);

    companion object {
        fun fromByte(code: Int): ValueType =
            entries.firstOrNull { it.code == code }
                ?: throw IllegalArgumentException("Unknown value type byte 0x${code.toString(16)}")
    }
}

enum class ExternalKind(val code: Int) {
    FUNCTION(0x00),
    TABLE(0x01),
    MEMORY(0x02),
    GLOBAL(0x03);

    companion object {
        fun fromByte(code: Int): ExternalKind =
            entries.firstOrNull { it.code == code }
                ?: throw IllegalArgumentException("Unknown external kind byte 0x${code.toString(16)}")
    }
}

data class FuncType(val params: List<ValueType>, val results: List<ValueType>)

data class Limits(val min: Int, val max: Int?)

data class MemoryType(val limits: Limits)

data class TableType(val elementType: Int, val limits: Limits)

data class GlobalType(val valueType: ValueType, val mutable: Boolean)

data class Import(val moduleName: String, val name: String, val kind: ExternalKind, val typeInfo: Any)

data class Export(val name: String, val kind: ExternalKind, val index: Int)

data class Global(val globalType: GlobalType, val initExpr: ByteArray)

data class Element(val tableIndex: Int, val offsetExpr: ByteArray, val functionIndices: List<Int>)

data class DataSegment(val memoryIndex: Int, val offsetExpr: ByteArray, val data: ByteArray)

data class FunctionBody(val locals: List<ValueType>, val code: ByteArray)

data class CustomSection(val name: String, val data: ByteArray)

fun makeFuncType(params: List<ValueType>, results: List<ValueType>): FuncType = FuncType(params.toList(), results.toList())

class WasmModule {
    val types: MutableList<FuncType> = mutableListOf()
    val imports: MutableList<Import> = mutableListOf()
    val functions: MutableList<Int> = mutableListOf()
    val tables: MutableList<TableType> = mutableListOf()
    val memories: MutableList<MemoryType> = mutableListOf()
    val globals: MutableList<Global> = mutableListOf()
    val exports: MutableList<Export> = mutableListOf()
    var start: Int? = null
    val elements: MutableList<Element> = mutableListOf()
    val code: MutableList<FunctionBody> = mutableListOf()
    val data: MutableList<DataSegment> = mutableListOf()
    val customs: MutableList<CustomSection> = mutableListOf()
}
