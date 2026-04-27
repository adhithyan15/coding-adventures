package com.codingadventures.wasmvalidator

import com.codingadventures.wasmleb128.WasmLeb128
import com.codingadventures.wasmtypes.ExternalKind
import com.codingadventures.wasmtypes.FuncType
import com.codingadventures.wasmtypes.GlobalType
import com.codingadventures.wasmtypes.Limits
import com.codingadventures.wasmtypes.MemoryType
import com.codingadventures.wasmtypes.TableType
import com.codingadventures.wasmtypes.ValueType
import com.codingadventures.wasmtypes.WasmModule

const val VERSION: String = "0.1.0"

enum class ValidationErrorKind {
    INVALID_TYPE_INDEX,
    INVALID_FUNC_INDEX,
    INVALID_TABLE_INDEX,
    INVALID_MEMORY_INDEX,
    MULTIPLE_MEMORIES,
    MULTIPLE_TABLES,
    MEMORY_LIMIT_EXCEEDED,
    MEMORY_LIMIT_ORDER,
    TABLE_LIMIT_ORDER,
    DUPLICATE_EXPORT_NAME,
    EXPORT_INDEX_OUT_OF_RANGE,
    START_FUNCTION_BAD_TYPE,
    INIT_EXPR_INVALID,
}

class ValidationError(val kind: ValidationErrorKind, message: String) : RuntimeException(message)

data class ValidatedModule(
    val module: WasmModule,
    val funcTypes: List<FuncType> = emptyList(),
    val funcLocals: List<List<ValueType>> = emptyList(),
)

data class IndexSpaces(
    val funcTypes: List<FuncType>,
    val numImportedFuncs: Int,
    val tableTypes: List<TableType>,
    val memoryTypes: List<MemoryType>,
    val globalTypes: List<GlobalType>,
    val numImportedGlobals: Int,
    val numTypes: Int,
)

private const val MAX_MEMORY_PAGES = 65_536

fun validate(module: WasmModule): ValidatedModule {
    val indexSpaces = validateStructure(module)
    val funcLocals =
        module.code.mapIndexed { index, body ->
            val funcType = indexSpaces.funcTypes[indexSpaces.numImportedFuncs + index]
            funcType.params + body.locals
        }
    return ValidatedModule(module, indexSpaces.funcTypes, funcLocals)
}

fun validateStructure(module: WasmModule): IndexSpaces {
    val indexSpaces = buildIndexSpaces(module)

    if (indexSpaces.tableTypes.size > 1) {
        throw ValidationError(ValidationErrorKind.MULTIPLE_TABLES, "WASM 1.0 allows at most one table")
    }
    if (indexSpaces.memoryTypes.size > 1) {
        throw ValidationError(ValidationErrorKind.MULTIPLE_MEMORIES, "WASM 1.0 allows at most one memory")
    }

    indexSpaces.memoryTypes.forEach { validateMemoryLimits(it.limits) }
    indexSpaces.tableTypes.forEach { validateTableLimits(it.limits) }
    validateExports(module, indexSpaces)
    validateStartFunction(module, indexSpaces)

    module.globals.forEach { global ->
        validateConstExpr(global.initExpr, global.globalType.valueType, indexSpaces)
    }
    module.elements.forEach { element ->
        if (element.tableIndex != 0 || element.tableIndex >= indexSpaces.tableTypes.size) {
            throw ValidationError(ValidationErrorKind.INVALID_TABLE_INDEX, "Invalid element table index")
        }
        validateConstExpr(element.offsetExpr, ValueType.I32, indexSpaces)
        element.functionIndices.forEach { funcIndex ->
            ensureIndex(funcIndex, indexSpaces.funcTypes.size, ValidationErrorKind.INVALID_FUNC_INDEX, "Invalid element function index")
        }
    }
    module.data.forEach { segment ->
        if (segment.memoryIndex != 0 || segment.memoryIndex >= indexSpaces.memoryTypes.size) {
            throw ValidationError(ValidationErrorKind.INVALID_MEMORY_INDEX, "Invalid data memory index")
        }
        validateConstExpr(segment.offsetExpr, ValueType.I32, indexSpaces)
    }

    return indexSpaces
}

fun validateConstExpr(expr: ByteArray, expectedType: ValueType, indexSpaces: IndexSpaces) {
    if (expr.size < 2 || (expr.last().toInt() and 0xFF) != 0x0B) {
        throw ValidationError(ValidationErrorKind.INIT_EXPR_INVALID, "Constant expression must end with 'end'")
    }

    val actualType =
        when (expr[0].toInt() and 0xFF) {
            0x41 -> ValueType.I32
            0x42 -> ValueType.I64
            0x43 -> ValueType.F32
            0x44 -> ValueType.F64
            0x23 -> {
                val index = WasmLeb128.decodeUnsigned(expr, 1).value.toInt()
                if (index < 0 || index >= indexSpaces.numImportedGlobals) {
                    throw ValidationError(
                        ValidationErrorKind.INIT_EXPR_INVALID,
                        "Constant expressions may only reference imported globals",
                    )
                }
                indexSpaces.globalTypes[index].valueType
            }
            else -> throw ValidationError(
                ValidationErrorKind.INIT_EXPR_INVALID,
                "Opcode 0x${(expr[0].toInt() and 0xFF).toString(16)} is not allowed in a constant expression",
            )
        }

    if (actualType != expectedType) {
        throw ValidationError(
            ValidationErrorKind.INIT_EXPR_INVALID,
            "Constant expression has type $actualType but expected $expectedType",
        )
    }
}

private fun buildIndexSpaces(module: WasmModule): IndexSpaces {
    if (module.functions.size != module.code.size) {
        throw ValidationError(ValidationErrorKind.INVALID_FUNC_INDEX, "Function and code section sizes differ")
    }

    val funcTypes = mutableListOf<FuncType>()
    val tableTypes = mutableListOf<TableType>()
    val memoryTypes = mutableListOf<MemoryType>()
    val globalTypes = mutableListOf<GlobalType>()
    var numImportedFuncs = 0
    var numImportedGlobals = 0

    module.imports.forEach { entry ->
        when (entry.kind) {
            ExternalKind.FUNCTION -> {
                val typeIndex =
                    entry.typeInfo as? Int
                        ?: throw ValidationError(ValidationErrorKind.INVALID_TYPE_INDEX, "Imported function missing type index")
                ensureIndex(typeIndex, module.types.size, ValidationErrorKind.INVALID_TYPE_INDEX, "Invalid imported function type index")
                funcTypes += module.types[typeIndex]
                numImportedFuncs += 1
            }
            ExternalKind.TABLE -> tableTypes += entry.typeInfo as TableType
            ExternalKind.MEMORY -> memoryTypes += entry.typeInfo as MemoryType
            ExternalKind.GLOBAL -> {
                globalTypes += entry.typeInfo as GlobalType
                numImportedGlobals += 1
            }
        }
    }

    module.functions.forEach { typeIndex ->
        ensureIndex(typeIndex, module.types.size, ValidationErrorKind.INVALID_TYPE_INDEX, "Invalid function type index")
        funcTypes += module.types[typeIndex]
    }

    tableTypes += module.tables
    memoryTypes += module.memories
    globalTypes += module.globals.map { it.globalType }

    return IndexSpaces(funcTypes, numImportedFuncs, tableTypes, memoryTypes, globalTypes, numImportedGlobals, module.types.size)
}

private fun validateExports(module: WasmModule, indexSpaces: IndexSpaces) {
    val seen = mutableSetOf<String>()
    module.exports.forEach { exportEntry ->
        if (!seen.add(exportEntry.name)) {
            throw ValidationError(ValidationErrorKind.DUPLICATE_EXPORT_NAME, "Duplicate export name")
        }
        val upperBound =
            when (exportEntry.kind) {
                ExternalKind.FUNCTION -> indexSpaces.funcTypes.size
                ExternalKind.TABLE -> indexSpaces.tableTypes.size
                ExternalKind.MEMORY -> indexSpaces.memoryTypes.size
                ExternalKind.GLOBAL -> indexSpaces.globalTypes.size
            }
        if (exportEntry.index !in 0 until upperBound) {
            throw ValidationError(ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE, "Export index out of range")
        }
    }
}

private fun validateStartFunction(module: WasmModule, indexSpaces: IndexSpaces) {
    val start = module.start ?: return
    ensureIndex(start, indexSpaces.funcTypes.size, ValidationErrorKind.INVALID_FUNC_INDEX, "Invalid start function index")
    val startType = indexSpaces.funcTypes[start]
    if (startType.params.isNotEmpty() || startType.results.isNotEmpty()) {
        throw ValidationError(ValidationErrorKind.START_FUNCTION_BAD_TYPE, "Start function must have type () -> ()")
    }
}

private fun validateMemoryLimits(limits: Limits) {
    val max = limits.max
    if (max != null && max > MAX_MEMORY_PAGES) {
        throw ValidationError(ValidationErrorKind.MEMORY_LIMIT_EXCEEDED, "Memory limit exceeds WASM 1.0 maximum")
    }
    if (max != null && limits.min > max) {
        throw ValidationError(ValidationErrorKind.MEMORY_LIMIT_ORDER, "Memory min exceeds max")
    }
}

private fun validateTableLimits(limits: Limits) {
    val max = limits.max
    if (max != null && limits.min > max) {
        throw ValidationError(ValidationErrorKind.TABLE_LIMIT_ORDER, "Table min exceeds max")
    }
}

private fun ensureIndex(index: Int, length: Int, kind: ValidationErrorKind, message: String) {
    if (index < 0 || index >= length) {
        throw ValidationError(kind, message)
    }
}
