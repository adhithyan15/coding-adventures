// WasmRuntime.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// WasmRuntime -- The Complete WebAssembly 1.0 Runtime
// ============================================================================
//
// A WASM runtime composes all the lower-level packages into a single,
// easy-to-use API. It handles the full pipeline:
//
//   .wasm bytes  ->  Parse  ->  Validate  ->  Instantiate  ->  Execute
//       |              |           |             |              |
//   [UInt8]       WasmModule  ValidatedModule  WasmInstance  WasmValue[]
//
// The convenience method loadAndRun() does all four steps in one call:
//
//   let runtime = WasmRuntime()
//   let result = try runtime.loadAndRun(squareWasm, entry: "square", args: [5])
//   // result == [25]
//
// ============================================================================
// Instantiation
// ============================================================================
//
// Instantiation transforms a static module definition into a live instance:
//
// 1. Resolve imports: ask the host for functions/memory/tables/globals
// 2. Allocate memory: create LinearMemory from the memory section
// 3. Allocate tables: create Tables from the table section
// 4. Initialize globals: evaluate constant expressions
// 5. Apply data segments: copy bytes into memory
// 6. Apply element segments: copy function refs into tables
// 7. Call start function: if the module declares one
//
// ============================================================================

import Foundation
import WasmLeb128
import WasmTypes
import WasmOpcodes
import WasmModuleParser
import WasmValidator
import WasmExecution
import VirtualMachine

// ============================================================================
// MARK: - WasmInstance
// ============================================================================

/// A live, executable instance of a WASM module.
///
/// Contains all allocated runtime state and provides access to exports.
/// Think of a module as a class and an instance as an object: the module
/// defines the blueprint, the instance is a live entity with its own state.
public struct WasmInstance {
    /// The original parsed module.
    public let module: WasmModule

    /// Allocated linear memory (nil if module has no memory).
    public let memory: LinearMemory?

    /// Allocated tables.
    public let tables: [Table]

    /// Current global variable values.
    public var globals: [WasmValue]

    /// Global type descriptors.
    public let globalTypes: [GlobalType]

    /// All function type signatures (imports + module functions).
    public let funcTypes: [FuncType]

    /// Function bodies (nil for imported functions).
    public let funcBodies: [FunctionBody?]

    /// Host function implementations (nil for module-defined functions).
    public let hostFunctions: [HostFunction?]

    /// Export lookup table: name -> (kind, index).
    public let exports: [String: (kind: ExternalKind, index: UInt32)]

    /// The host interface used to resolve imports.
    public let host: HostInterface?
}

// ============================================================================
// MARK: - WasiStub
// ============================================================================

/// Minimal WASI implementation for programs that do I/O.
///
/// WASI (WebAssembly System Interface) provides host functions like
/// fd_write (stdout/stderr) and proc_exit. This stub implements just
/// enough to run Hello World programs:
///
///   fd_write: captures stdout/stderr output
///   proc_exit: terminates execution with an exit code
///
/// Everything else returns ENOSYS (errno 52: function not implemented).
public class WasiStub: HostInterface {
    /// Captured stdout output.
    public var stdoutOutput: String = ""

    /// Captured stderr output.
    public var stderrOutput: String = ""

    /// Exit code set by proc_exit.
    public var exitCode: Int32? = nil

    /// Optional callback for stdout writes.
    public var onStdout: ((String) -> Void)?

    /// The memory instance (set during instantiation).
    public var memory: LinearMemory?

    private let ENOSYS: Int32 = 52

    public init(onStdout: ((String) -> Void)? = nil) {
        self.onStdout = onStdout
    }

    // -- HostInterface conformance --

    public func resolveFunction(moduleName: String, name: String) -> HostFunction? {
        guard moduleName == "wasi_snapshot_preview1" || moduleName == "wasi_unstable" else {
            return nil
        }

        switch name {
        case "fd_write":
            return HostFunction(
                type: FuncType(params: [.i32, .i32, .i32, .i32], results: [.i32]),
                call: { [weak self] args in
                    return try self?.fdWrite(args) ?? [.i32(self?.ENOSYS ?? 52)]
                }
            )

        case "proc_exit":
            return HostFunction(
                type: FuncType(params: [.i32], results: []),
                call: { [weak self] args in
                    if case .i32(let code) = args[0] {
                        self?.exitCode = code
                    }
                    // Returning normally -- the caller should check exitCode.
                    return []
                }
            )

        default:
            // Return a stub that returns ENOSYS for any unimplemented function.
            // We don't know the exact signature, so provide a generic one.
            return HostFunction(
                type: FuncType(params: [.i32], results: [.i32]),
                call: { [weak self] _ in
                    return [.i32(self?.ENOSYS ?? 52)]
                }
            )
        }
    }

    public func resolveGlobal(moduleName: String, name: String) -> (type: GlobalType, value: WasmValue)? {
        return nil
    }

    public func resolveMemory(moduleName: String, name: String) -> LinearMemory? {
        return nil
    }

    public func resolveTable(moduleName: String, name: String) -> Table? {
        return nil
    }

    // -- fd_write implementation --
    //
    // fd_write(fd: i32, iovs: i32, iovsLen: i32, nwritten: i32) -> i32
    //
    // Reads iovec structures from memory (each is [ptr:i32, len:i32]),
    // concatenates the referenced byte ranges, and writes them to the
    // appropriate file descriptor.

    private func fdWrite(_ args: [WasmValue]) throws -> [WasmValue] {
        guard let mem = memory else { return [.i32(ENOSYS)] }

        let fd = try args[0].asI32()
        let iovsPtr = try args[1].asI32()
        let iovsLen = try args[2].asI32()
        let nwrittenPtr = try args[3].asI32()

        var totalBytes: Int32 = 0
        var output = ""

        for i in 0..<iovsLen {
            let iovOffset = Int(iovsPtr) + Int(i) * 8
            let ptr = try mem.loadI32(iovOffset)
            let len = try mem.loadI32(iovOffset + 4)

            if len > 0 {
                var bytes: [UInt8] = []
                for j in 0..<Int(len) {
                    let b = try mem.loadI32_8u(Int(ptr) + j)
                    bytes.append(UInt8(b))
                }
                if let str = String(bytes: bytes, encoding: .utf8) {
                    output += str
                }
                totalBytes &+= len
            }
        }

        // Write to appropriate fd.
        if fd == 1 {
            stdoutOutput += output
            onStdout?(output)
        } else if fd == 2 {
            stderrOutput += output
        }

        // Write number of bytes written.
        try mem.storeI32(Int(nwrittenPtr), totalBytes)

        return [.i32(0)]  // Success
    }
}

// ============================================================================
// MARK: - WasmRuntime
// ============================================================================

/// Complete WebAssembly 1.0 runtime.
///
/// Composes the parser, validator, and execution engine into a single
/// user-facing API. Optionally accepts a host interface for import
/// resolution (e.g., a WASI implementation).
///
/// Usage:
///
///   // Simple: compute square(5) from a .wasm binary
///   let runtime = WasmRuntime()
///   let result = try runtime.loadAndRun(squareWasm, entry: "square", args: [5])
///   // result == [25]
///
///   // With WASI for programs that do I/O:
///   let wasi = WasiStub()
///   let runtime = WasmRuntime(host: wasi)
///   try runtime.loadAndRun(helloWorldWasm)
///   print(wasi.stdoutOutput)
///
public class WasmRuntime {
    private let parser: WasmModuleParser
    private let host: HostInterface?

    /// Create a runtime with an optional host interface.
    public init(host: HostInterface? = nil) {
        self.parser = WasmModuleParser()
        self.host = host
    }

    // ========================================================================
    // MARK: - Parse
    // ========================================================================

    /// Parse a .wasm binary into a WasmModule.
    ///
    /// - Parameter wasmBytes: The raw .wasm binary data.
    /// - Returns: The parsed module structure.
    /// - Throws: WasmParseError on malformed binary data.
    public func load(_ wasmBytes: [UInt8]) throws -> WasmModule {
        return try parser.parse(wasmBytes)
    }

    // ========================================================================
    // MARK: - Validate
    // ========================================================================

    /// Validate a parsed module for structural correctness.
    ///
    /// - Parameter module: The parsed WASM module.
    /// - Returns: The validated module with resolved type information.
    /// - Throws: ValidationError on validation failures.
    public func validateModule(_ module: WasmModule) throws -> ValidatedModule {
        return try validate(module)
    }

    // ========================================================================
    // MARK: - Instantiate
    // ========================================================================

    /// Create a live instance from a parsed module.
    ///
    /// This allocates all runtime resources: memory, tables, globals.
    /// Resolves imports, applies data/element segments, and calls the
    /// start function if one is declared.
    ///
    /// - Parameter module: The parsed WASM module.
    /// - Returns: A live, executable instance.
    public func instantiate(_ module: WasmModule) throws -> WasmInstance {
        // Step 1: Build combined function type array (imports + module functions).
        var funcTypes: [FuncType] = []
        var funcBodies: [FunctionBody?] = []
        var hostFunctions: [HostFunction?] = []
        var globalTypes: [GlobalType] = []
        var globals: [WasmValue] = []

        // Step 2: Resolve imports.
        var memory: LinearMemory? = nil
        var tables: [Table] = []

        for imp in module.imports {
            switch imp.kind {
            case .function:
                if case .function(let typeIndex) = imp.typeInfo {
                    let funcType = module.types[Int(typeIndex)]
                    funcTypes.append(funcType)
                    funcBodies.append(nil)  // No body for imports.

                    let hostFunc = host?.resolveFunction(moduleName: imp.moduleName, name: imp.name)
                    hostFunctions.append(hostFunc)
                }

            case .memory:
                let importedMem = host?.resolveMemory(moduleName: imp.moduleName, name: imp.name)
                if let m = importedMem {
                    memory = m
                }

            case .table:
                let importedTable = host?.resolveTable(moduleName: imp.moduleName, name: imp.name)
                if let t = importedTable {
                    tables.append(t)
                }

            case .global:
                let importedGlobal = host?.resolveGlobal(moduleName: imp.moduleName, name: imp.name)
                if let g = importedGlobal {
                    globalTypes.append(g.type)
                    globals.append(g.value)
                }
            }
        }

        // Step 3: Add module-defined functions.
        for i in 0..<module.functions.count {
            let typeIdx = module.functions[i]
            funcTypes.append(module.types[Int(typeIdx)])
            if i < module.code.count {
                funcBodies.append(module.code[i])
            } else {
                funcBodies.append(nil)
            }
            hostFunctions.append(nil)
        }

        // Step 4: Allocate memory (from memory section, if not imported).
        if memory == nil && !module.memories.isEmpty {
            let memType = module.memories[0]
            memory = LinearMemory(
                initialPages: Int(memType.limits.min),
                maxPages: memType.limits.max.map { Int($0) }
            )
        }

        // Give WASI access to memory.
        if let wasi = host as? WasiStub {
            wasi.memory = memory
        }

        // Step 5: Allocate tables (from table section, if not imported).
        for tableType in module.tables {
            tables.append(Table(
                initialSize: Int(tableType.limits.min),
                maxSize: tableType.limits.max.map { Int($0) }
            ))
        }

        // Step 6: Initialize globals (from global section).
        for global in module.globals {
            globalTypes.append(global.globalType)
            let value = try evaluateConstExpr(global.initExpr, globals: globals)
            globals.append(value)
        }

        // Step 7: Apply data segments (copy bytes to memory).
        if let mem = memory {
            for seg in module.data {
                let offset = try evaluateConstExpr(seg.offsetExpr, globals: globals)
                if case .i32(let off) = offset {
                    try mem.writeBytes(Int(off), seg.data)
                }
            }
        }

        // Step 8: Apply element segments (copy func refs to tables).
        for elem in module.elements {
            if Int(elem.tableIndex) < tables.count {
                let table = tables[Int(elem.tableIndex)]
                let offset = try evaluateConstExpr(elem.offsetExpr, globals: globals)
                if case .i32(let off) = offset {
                    for j in 0..<elem.functionIndices.count {
                        try table.set(Int(off) + j, Int(elem.functionIndices[j]))
                    }
                }
            }
        }

        // Build the export map.
        var exports: [String: (kind: ExternalKind, index: UInt32)] = [:]
        for exp in module.exports {
            exports[exp.name] = (kind: exp.kind, index: exp.index)
        }

        let instance = WasmInstance(
            module: module,
            memory: memory,
            tables: tables,
            globals: globals,
            globalTypes: globalTypes,
            funcTypes: funcTypes,
            funcBodies: funcBodies,
            hostFunctions: hostFunctions,
            exports: exports,
            host: host
        )

        // Step 9: Call start function (if present).
        if let startIdx = module.start {
            let engine = WasmExecutionEngine(
                memory: instance.memory,
                tables: instance.tables,
                globals: instance.globals,
                globalTypes: instance.globalTypes,
                funcTypes: instance.funcTypes,
                funcBodies: instance.funcBodies,
                hostFunctions: instance.hostFunctions
            )
            _ = try engine.callFunction(Int(startIdx), [])
        }

        return instance
    }

    // ========================================================================
    // MARK: - Call
    // ========================================================================

    /// Call an exported function by name.
    ///
    /// - Parameters:
    ///   - instance: The live WASM instance.
    ///   - name: The export name (e.g., "square", "add", "_start").
    ///   - args: Arguments as plain numbers (converted to WasmValues).
    /// - Returns: Return values as plain numbers.
    /// - Throws: TrapError if the export doesn't exist or on runtime errors.
    public func call(_ instance: inout WasmInstance, name: String, args: [Double] = []) throws -> [Double] {
        guard let exp = instance.exports[name] else {
            throw TrapError("export \"\(name)\" not found")
        }
        guard exp.kind == .function else {
            throw TrapError("export \"\(name)\" is not a function")
        }

        let funcIndex = Int(exp.index)
        guard funcIndex < instance.funcTypes.count else {
            throw TrapError("function type not found for export \"\(name)\"")
        }
        let funcType = instance.funcTypes[funcIndex]

        // Convert plain numbers to WasmValues based on parameter types.
        var wasmArgs: [WasmValue] = []
        for i in 0..<args.count {
            if i < funcType.params.count {
                switch funcType.params[i] {
                case .i32: wasmArgs.append(.i32(Int32(args[i])))
                case .i64: wasmArgs.append(.i64(Int64(args[i])))
                case .f32: wasmArgs.append(.f32(Float(args[i])))
                case .f64: wasmArgs.append(.f64(args[i]))
                }
            } else {
                wasmArgs.append(.i32(Int32(args[i])))
            }
        }

        // Create execution engine and call.
        let engine = WasmExecutionEngine(
            memory: instance.memory,
            tables: instance.tables,
            globals: instance.globals,
            globalTypes: instance.globalTypes,
            funcTypes: instance.funcTypes,
            funcBodies: instance.funcBodies,
            hostFunctions: instance.hostFunctions
        )

        let results = try engine.callFunction(funcIndex, wasmArgs)

        // Propagate global mutations back to instance.
        instance.globals = engine.globals

        // Convert WasmValues back to plain numbers.
        return results.map { $0.numericValue }
    }

    // ========================================================================
    // MARK: - Convenience
    // ========================================================================

    /// Parse, validate, instantiate, and call in one step.
    ///
    /// This is the easiest way to run a WASM module:
    ///
    ///   let result = try runtime.loadAndRun(wasmBytes, entry: "square", args: [5])
    ///   // result == [25]
    ///
    /// - Parameters:
    ///   - wasmBytes: The raw .wasm binary.
    ///   - entry: The export function name to call (default: "_start").
    ///   - args: Arguments as plain numbers (default: []).
    /// - Returns: Return values as plain numbers.
    public func loadAndRun(_ wasmBytes: [UInt8], entry: String = "_start", args: [Double] = []) throws -> [Double] {
        let module = try load(wasmBytes)
        _ = try validateModule(module)
        var instance = try instantiate(module)
        return try call(&instance, name: entry, args: args)
    }
}
