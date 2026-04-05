// WasmValidator.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// WasmValidator -- WebAssembly 1.0 Structural Validator
// ============================================================================
//
// The validator checks a parsed WasmModule for structural correctness:
//   - Type indices are in range
//   - Function indices are in range
//   - Memory limits are valid
//   - Export names are unique
//   - Start function has correct signature
//   - At most one memory and one table (WASM 1.0)
//
// This is a structural validator, not a full type-checking validator.
// Full bytecode validation is deferred to the execution engine which
// validates as it runs.
//
// ============================================================================

import WasmTypes

// ============================================================================
// MARK: - Validation Error
// ============================================================================

/// Errors that can occur during validation.
public enum ValidationError: Error, Equatable {
    case invalidTypeIndex(UInt32)
    case invalidFuncIndex(UInt32)
    case multipleMemories
    case multipleTables
    case memoryLimitExceeded(UInt32)
    case memoryLimitOrder(min: UInt32, max: UInt32)
    case tableLimitOrder(min: UInt32, max: UInt32)
    case duplicateExportName(String)
    case exportIndexOutOfRange(String, UInt32)
    case startFunctionBadType
    case functionCodeMismatch(functions: Int, code: Int)
}

// ============================================================================
// MARK: - ValidatedModule
// ============================================================================

/// A validated module with resolved type information.
public struct ValidatedModule {
    /// The original parsed module.
    public let module: WasmModule
    /// All function type signatures (imports + module functions).
    public let funcTypes: [FuncType]

    public init(module: WasmModule, funcTypes: [FuncType]) {
        self.module = module
        self.funcTypes = funcTypes
    }
}

// ============================================================================
// MARK: - Validate Function
// ============================================================================

private let MAX_MEMORY_PAGES: UInt32 = 65536

/// Validate a parsed WASM module for structural correctness.
///
/// Returns a ValidatedModule on success, or throws ValidationError.
public func validate(_ module: WasmModule) throws -> ValidatedModule {

    // Count imported entities by kind.
    var numImportedFuncs: Int = 0
    var numImportedTables: Int = 0
    var numImportedMemories: Int = 0
    var numImportedGlobals: Int = 0

    for imp in module.imports {
        switch imp.kind {
        case .function:
            numImportedFuncs += 1
        case .table:
            numImportedTables += 1
        case .memory:
            numImportedMemories += 1
        case .global:
            numImportedGlobals += 1
        }
    }

    // Total counts.
    let totalFuncs = numImportedFuncs + module.functions.count
    let totalTables = numImportedTables + module.tables.count
    let totalMemories = numImportedMemories + module.memories.count
    let totalGlobals = numImportedGlobals + module.globals.count

    // WASM 1.0: at most one memory and one table.
    if totalMemories > 1 {
        throw ValidationError.multipleMemories
    }
    if totalTables > 1 {
        throw ValidationError.multipleTables
    }

    // Validate function type indices.
    for imp in module.imports {
        if case .function(let typeIndex) = imp.typeInfo {
            if typeIndex >= module.types.count {
                throw ValidationError.invalidTypeIndex(typeIndex)
            }
        }
    }

    for typeIdx in module.functions {
        if typeIdx >= module.types.count {
            throw ValidationError.invalidTypeIndex(typeIdx)
        }
    }

    // Validate memory limits.
    for mem in module.memories {
        if mem.limits.min > MAX_MEMORY_PAGES {
            throw ValidationError.memoryLimitExceeded(mem.limits.min)
        }
        if let max = mem.limits.max {
            if max > MAX_MEMORY_PAGES {
                throw ValidationError.memoryLimitExceeded(max)
            }
            if mem.limits.min > max {
                throw ValidationError.memoryLimitOrder(min: mem.limits.min, max: max)
            }
        }
    }

    // Validate table limits.
    for table in module.tables {
        if let max = table.limits.max {
            if table.limits.min > max {
                throw ValidationError.tableLimitOrder(min: table.limits.min, max: max)
            }
        }
    }

    // Validate export names are unique.
    var exportNames = Set<String>()
    for exp in module.exports {
        if exportNames.contains(exp.name) {
            throw ValidationError.duplicateExportName(exp.name)
        }
        exportNames.insert(exp.name)

        // Validate export indices.
        switch exp.kind {
        case .function:
            if exp.index >= totalFuncs {
                throw ValidationError.exportIndexOutOfRange(exp.name, exp.index)
            }
        case .table:
            if exp.index >= totalTables {
                throw ValidationError.exportIndexOutOfRange(exp.name, exp.index)
            }
        case .memory:
            if exp.index >= totalMemories {
                throw ValidationError.exportIndexOutOfRange(exp.name, exp.index)
            }
        case .global:
            if exp.index >= totalGlobals {
                throw ValidationError.exportIndexOutOfRange(exp.name, exp.index)
            }
        }
    }

    // Validate start function.
    if let startIdx = module.start {
        if startIdx >= totalFuncs {
            throw ValidationError.invalidFuncIndex(startIdx)
        }
        // Start function must have type [] -> [].
        let funcType: FuncType
        if startIdx < numImportedFuncs {
            // Imported function.
            if case .function(let typeIdx) = module.imports[Int(startIdx)].typeInfo {
                funcType = module.types[Int(typeIdx)]
            } else {
                throw ValidationError.startFunctionBadType
            }
        } else {
            let localIdx = Int(startIdx) - numImportedFuncs
            let typeIdx = module.functions[localIdx]
            funcType = module.types[Int(typeIdx)]
        }
        if !funcType.params.isEmpty || !funcType.results.isEmpty {
            throw ValidationError.startFunctionBadType
        }
    }

    // Validate function count matches code count.
    if module.functions.count != module.code.count {
        throw ValidationError.functionCodeMismatch(
            functions: module.functions.count, code: module.code.count)
    }

    // Build the combined function type array.
    var funcTypes: [FuncType] = []
    for imp in module.imports {
        if case .function(let typeIdx) = imp.typeInfo {
            funcTypes.append(module.types[Int(typeIdx)])
        }
    }
    for typeIdx in module.functions {
        funcTypes.append(module.types[Int(typeIdx)])
    }

    return ValidatedModule(module: module, funcTypes: funcTypes)
}
