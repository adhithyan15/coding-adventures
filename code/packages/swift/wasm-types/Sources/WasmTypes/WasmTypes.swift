// WasmTypes.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// WasmTypes -- Pure Type Definitions for the WebAssembly 1.0 Type System
// ============================================================================
//
// Every value, function, memory region, and import/export in a WASM module is
// described by a type. This module is the source of truth for all those type
// definitions.
//
// The WASM 1.0 type system is intentionally minimal. It has just four numeric
// value types, a handful of composite types (function types, table types,
// memory types, global types), and a handful of structural types (imports,
// exports, function bodies, etc.).
//
// ============================================================================
// THE WASM BINARY SECTION LAYOUT
// ============================================================================
//
// A valid .wasm file looks like this at the top level:
//
//   +--------------------------------------------------------------------+
//   |  Magic bytes: 0x00 0x61 0x73 0x6D  ("\0asm")                      |
//   |  Version:     0x01 0x00 0x00 0x00  (1)                            |
//   +--------------------------------------------------------------------+
//   |  Section 1:  Type section    -> FuncType[]                         |
//   |  Section 2:  Import section  -> Import[]                           |
//   |  Section 3:  Function section-> type-index[]                       |
//   |  Section 4:  Table section   -> TableType[]                        |
//   |  Section 5:  Memory section  -> MemoryType[]                       |
//   |  Section 6:  Global section  -> Global[]                           |
//   |  Section 7:  Export section  -> Export[]                           |
//   |  Section 8:  Start section   -> function index (optional)          |
//   |  Section 9:  Element section -> Element[]                          |
//   |  Section 10: Code section    -> FunctionBody[]                     |
//   |  Section 11: Data section    -> DataSegment[]                      |
//   |  Section 0:  Custom sections -> CustomSection[] (any number)       |
//   +--------------------------------------------------------------------+
//
// ============================================================================

import Foundation

// ============================================================================
// MARK: - ValueType
// ============================================================================

/// The four numeric types in WASM 1.0.
///
/// Every local variable, function parameter, function result, and global
/// variable has exactly one of these four types.
///
///   +----------+--------+------------------------------------------------+
///   | Name     | Byte   | Meaning                                        |
///   +----------+--------+------------------------------------------------+
///   | i32      | 0x7F   | 32-bit integer (signed or unsigned by opcode)  |
///   | i64      | 0x7E   | 64-bit integer                                 |
///   | f32      | 0x7D   | 32-bit IEEE 754 floating-point                 |
///   | f64      | 0x7C   | 64-bit IEEE 754 floating-point                 |
///   +----------+--------+------------------------------------------------+
public enum ValueType: UInt8, Equatable, Hashable {
    case i32 = 0x7F
    case i64 = 0x7E
    case f32 = 0x7D
    case f64 = 0x7C
}

// ============================================================================
// MARK: - BlockType
// ============================================================================

/// Block produces no result value (statement, not expression).
/// In WASM 1.0 a block type is either 0x40 (empty) or a ValueType byte.
public let BLOCK_TYPE_EMPTY: UInt8 = 0x40

// ============================================================================
// MARK: - ExternalKind
// ============================================================================

/// Classifies what kind of definition is being imported or exported.
///
///   +----------+--------+---------------------------------------------------+
///   | Kind     | Byte   | What it refers to                                 |
///   +----------+--------+---------------------------------------------------+
///   | function | 0x00   | A function defined (or imported) in the module    |
///   | table    | 0x01   | A table (array of function references)            |
///   | memory   | 0x02   | A linear memory region                           |
///   | global   | 0x03   | A global variable                                |
///   +----------+--------+---------------------------------------------------+
public enum ExternalKind: UInt8, Equatable {
    case function = 0x00
    case table    = 0x01
    case memory   = 0x02
    case global   = 0x03
}

// ============================================================================
// MARK: - FuncType
// ============================================================================

/// The type signature of a function.
///
/// Binary encoding:
///   0x60  [n uleb128] [param_type]*n  [m uleb128] [result_type]*m
public struct FuncType: Equatable {
    /// Parameter types, in left-to-right declaration order.
    public let params: [ValueType]
    /// Result types (WASM 1.0: at most one).
    public let results: [ValueType]

    public init(params: [ValueType], results: [ValueType]) {
        self.params = params
        self.results = results
    }
}

// ============================================================================
// MARK: - Limits
// ============================================================================

/// Describes the size constraints of a memory or table.
///
/// Binary encoding:
///   0x00 [min uleb128]               -- no maximum
///   0x01 [min uleb128] [max uleb128] -- with maximum
public struct Limits: Equatable {
    /// Minimum number of pages (memory) or elements (table).
    public let min: UInt32
    /// Maximum number of pages/elements, or nil if unbounded.
    public let max: UInt32?

    public init(min: UInt32, max: UInt32? = nil) {
        self.min = min
        self.max = max
    }
}

// ============================================================================
// MARK: - MemoryType
// ============================================================================

/// The type of a linear memory region.
public struct MemoryType: Equatable {
    public let limits: Limits

    public init(limits: Limits) {
        self.limits = limits
    }
}

// ============================================================================
// MARK: - TableType
// ============================================================================

/// The funcref element-type byte used in TableType.
public let FUNCREF: UInt8 = 0x70

/// The type of a WASM table.
public struct TableType: Equatable {
    /// Element type tag. Always 0x70 (funcref) in WASM 1.0.
    public let elementType: UInt8
    public let limits: Limits

    public init(elementType: UInt8, limits: Limits) {
        self.elementType = elementType
        self.limits = limits
    }
}

// ============================================================================
// MARK: - GlobalType
// ============================================================================

/// The type of a global variable.
///
///   +--------+--------+----------------------------------------------+
///   | mutable| byte   | Allowed operations                           |
///   +--------+--------+----------------------------------------------+
///   |  false |  0x00  | global.get only                              |
///   |  true  |  0x01  | global.get and global.set                    |
///   +--------+--------+----------------------------------------------+
public struct GlobalType: Equatable {
    public let valueType: ValueType
    /// true -> global.set is allowed; false -> read-only constant.
    public let mutable: Bool

    public init(valueType: ValueType, mutable: Bool) {
        self.valueType = valueType
        self.mutable = mutable
    }
}

// ============================================================================
// MARK: - Import
// ============================================================================

/// The type-specific information carried by an import declaration.
public enum ImportTypeInfo: Equatable {
    case function(typeIndex: UInt32)
    case table(TableType)
    case memory(MemoryType)
    case global(GlobalType)
}

/// A single import declaration.
public struct Import: Equatable {
    /// The module namespace, e.g. "env" or "wasi_snapshot_preview1".
    public let moduleName: String
    /// The field name within the module namespace.
    public let name: String
    /// What kind of entity is being imported.
    public let kind: ExternalKind
    /// Type information whose shape depends on `kind`.
    public let typeInfo: ImportTypeInfo

    public init(moduleName: String, name: String, kind: ExternalKind, typeInfo: ImportTypeInfo) {
        self.moduleName = moduleName
        self.name = name
        self.kind = kind
        self.typeInfo = typeInfo
    }
}

// ============================================================================
// MARK: - Export
// ============================================================================

/// A single export declaration.
public struct Export: Equatable {
    /// The name under which this definition is visible to the host.
    public let name: String
    /// What kind of entity is exported.
    public let kind: ExternalKind
    /// Index into the corresponding index space.
    public let index: UInt32

    public init(name: String, kind: ExternalKind, index: UInt32) {
        self.name = name
        self.kind = kind
        self.index = index
    }
}

// ============================================================================
// MARK: - Global
// ============================================================================

/// A module-defined global variable with its initializer.
public struct Global: Equatable {
    public let globalType: GlobalType
    /// Raw bytes of the constant init expression, including the trailing
    /// `end` opcode (0x0B).
    public let initExpr: [UInt8]

    public init(globalType: GlobalType, initExpr: [UInt8]) {
        self.globalType = globalType
        self.initExpr = initExpr
    }
}

// ============================================================================
// MARK: - Element
// ============================================================================

/// A table initializer segment.
public struct Element: Equatable {
    /// Index of the table to initialize (always 0 in WASM 1.0).
    public let tableIndex: UInt32
    /// Constant-expression bytes yielding the starting slot index.
    public let offsetExpr: [UInt8]
    /// Function indices to write into successive table slots.
    public let functionIndices: [UInt32]

    public init(tableIndex: UInt32, offsetExpr: [UInt8], functionIndices: [UInt32]) {
        self.tableIndex = tableIndex
        self.offsetExpr = offsetExpr
        self.functionIndices = functionIndices
    }
}

// ============================================================================
// MARK: - DataSegment
// ============================================================================

/// A linear-memory initializer.
public struct DataSegment: Equatable {
    /// Index of the memory to initialize (always 0 in WASM 1.0).
    public let memoryIndex: UInt32
    /// Constant-expression bytes yielding the byte address in memory.
    public let offsetExpr: [UInt8]
    /// The raw bytes to copy into memory.
    public let data: [UInt8]

    public init(memoryIndex: UInt32, offsetExpr: [UInt8], data: [UInt8]) {
        self.memoryIndex = memoryIndex
        self.offsetExpr = offsetExpr
        self.data = data
    }
}

// ============================================================================
// MARK: - FunctionBody
// ============================================================================

/// The body of a module-defined function.
public struct FunctionBody: Equatable {
    /// The types of all local variables declared in this function body
    /// (parameters are NOT included -- those are in the FuncType).
    public let locals: [ValueType]
    /// Raw opcode bytes for the function body, including the trailing
    /// `end` opcode (0x0B).
    public let code: [UInt8]

    public init(locals: [ValueType], code: [UInt8]) {
        self.locals = locals
        self.code = code
    }
}

// ============================================================================
// MARK: - CustomSection
// ============================================================================

/// An arbitrary named byte blob.
public struct CustomSection: Equatable {
    /// The name of this custom section (e.g. "name", "producers").
    public let name: String
    /// Raw byte payload.
    public let data: [UInt8]

    public init(name: String, data: [UInt8]) {
        self.name = name
        self.data = data
    }
}

// ============================================================================
// MARK: - WasmModule
// ============================================================================

/// A mutable in-memory representation of a WASM 1.0 module.
///
/// This class mirrors the eleven standard sections of the WASM binary format
/// plus zero or more custom sections. A parser reads the binary and populates
/// these arrays; a code generator or validator reads them.
public class WasmModule: Equatable {
    /// Function type signatures (type section).
    public var types: [FuncType] = []

    /// Imported definitions from the host (import section).
    public var imports: [Import] = []

    /// Type-section indices for each module-defined function (function section).
    public var functions: [UInt32] = []

    /// Table definitions (table section). WASM 1.0 allows at most one.
    public var tables: [TableType] = []

    /// Memory definitions (memory section). WASM 1.0 allows at most one.
    public var memories: [MemoryType] = []

    /// Module-defined globals with their init expressions (global section).
    public var globals: [Global] = []

    /// Exported definitions, visible to the host (export section).
    public var exports: [Export] = []

    /// The index of the start function, or nil if absent.
    public var start: UInt32? = nil

    /// Table initializer segments (element section).
    public var elements: [Element] = []

    /// Function bodies in the same order as `functions` (code section).
    public var code: [FunctionBody] = []

    /// Memory initializer segments (data section).
    public var data: [DataSegment] = []

    /// Custom (non-standard) sections.
    public var customs: [CustomSection] = []

    public init() {}

    public static func == (lhs: WasmModule, rhs: WasmModule) -> Bool {
        return lhs.types == rhs.types &&
               lhs.imports == rhs.imports &&
               lhs.functions == rhs.functions &&
               lhs.tables == rhs.tables &&
               lhs.memories == rhs.memories &&
               lhs.globals == rhs.globals &&
               lhs.exports == rhs.exports &&
               lhs.start == rhs.start &&
               lhs.elements == rhs.elements &&
               lhs.code == rhs.code &&
               lhs.data == rhs.data &&
               lhs.customs == rhs.customs
    }
}
