// VirtualMachine.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// GenericVM -- A Pluggable Stack-Based Bytecode Interpreter
// ============================================================================
//
// The GenericVM uses the Strategy Pattern: instead of hardcoding opcode
// handlers, languages register their own handlers at runtime. Each handler
// is a closure that receives the VM, instruction, and code object.
//
// For WASM, we also support context-aware handlers that receive an
// additional context object (WasmExecutionContext) carrying per-execution
// state like linear memory, tables, globals, and the label stack.
//
// ============================================================================
// Typed Stack
// ============================================================================
//
// Some VMs (like WASM) have typed operand stacks where every value carries
// a type tag. The GenericVM provides a parallel typedStack for this purpose.
// Languages that need typing use pushTyped/popTyped; languages that don't
// use the untyped push/pop.
//
// ============================================================================

import Foundation

// ============================================================================
// MARK: - VM Types
// ============================================================================

/// A value that can live on the VM stack or in variables.
public enum VMValue: Equatable {
    case int(Int)
    case int32(Int32)
    case int64(Int64)
    case uint32(UInt32)
    case uint64(UInt64)
    case float(Float)
    case double(Double)
    case string(String)
    case bool(Bool)
    case null

    public static func == (lhs: VMValue, rhs: VMValue) -> Bool {
        switch (lhs, rhs) {
        case (.int(let a), .int(let b)): return a == b
        case (.int32(let a), .int32(let b)): return a == b
        case (.int64(let a), .int64(let b)): return a == b
        case (.uint32(let a), .uint32(let b)): return a == b
        case (.uint64(let a), .uint64(let b)): return a == b
        case (.float(let a), .float(let b)): return a == b
        case (.double(let a), .double(let b)): return a == b
        case (.string(let a), .string(let b)): return a == b
        case (.bool(let a), .bool(let b)): return a == b
        case (.null, .null): return true
        default: return false
        }
    }
}

/// A typed value: a numeric payload tagged with its type code.
public struct TypedVMValue: Equatable {
    /// Type tag (for WASM: 0x7F=i32, 0x7E=i64, 0x7D=f32, 0x7C=f64).
    public let type: UInt8
    /// The raw value.
    public let value: VMValue

    public init(type: UInt8, value: VMValue) {
        self.type = type
        self.value = value
    }
}

/// A single instruction in the bytecode stream.
public struct Instruction {
    /// The opcode byte.
    public let opcode: UInt8
    /// The decoded operand, or nil if no immediates.
    public let operand: Any?

    public init(opcode: UInt8, operand: Any? = nil) {
        self.opcode = opcode
        self.operand = operand
    }
}

/// A complete code object containing instructions and metadata.
public struct CodeObject {
    public let instructions: [Instruction]
    public let constants: [VMValue]
    public let names: [String]

    public init(instructions: [Instruction], constants: [VMValue] = [], names: [String] = []) {
        self.instructions = instructions
        self.constants = constants
        self.names = names
    }
}

// ============================================================================
// MARK: - VM Errors
// ============================================================================

/// Errors that can occur during VM execution.
public enum VMError: Error {
    case stackUnderflow
    case invalidOpcode(UInt8)
    case executionHalted(String)
    case maxRecursionDepthExceeded
}

// ============================================================================
// MARK: - Handler Types
// ============================================================================

/// A function that handles one specific opcode.
public typealias OpcodeHandler = (GenericVM, Instruction, CodeObject) -> Void

/// An opcode handler that receives an additional context object.
public typealias ContextOpcodeHandler = (GenericVM, Instruction, CodeObject, AnyObject) -> Void

// ============================================================================
// MARK: - GenericVM
// ============================================================================

/// A pluggable stack-based bytecode interpreter.
///
/// The core VM provides the execution engine (stack, PC, eval loop), and
/// languages plug in their specific behavior through registered handlers.
public class GenericVM {

    // -- Operand stacks --

    /// The untyped operand stack.
    public var stack: [VMValue] = []

    /// The typed operand stack (for WASM and JVM-like VMs).
    public var typedStack: [TypedVMValue] = []

    // -- Program counter --

    /// Current instruction index.
    public private(set) var pc: Int = 0

    /// Whether execution should stop.
    public var halted: Bool = false

    // -- Handler registries --

    /// Registered opcode handlers (opcode byte -> handler).
    private var handlers: [UInt8: OpcodeHandler] = [:]

    /// Registered context-aware opcode handlers.
    private var contextHandlers: [UInt8: ContextOpcodeHandler] = [:]

    // -- Safety --

    /// Maximum call stack depth (default 1024).
    public var maxRecursionDepth: Int = 1024

    // -- Variables (for simple VMs) --
    public var variables: [String: VMValue] = [:]

    public init() {}

    // ========================================================================
    // MARK: - Handler Registration
    // ========================================================================

    /// Register a handler for a specific opcode.
    public func registerOpcode(_ opcode: UInt8, handler: @escaping OpcodeHandler) {
        handlers[opcode] = handler
    }

    /// Register a context-aware handler for a specific opcode.
    public func registerContextOpcode(_ opcode: UInt8, handler: @escaping ContextOpcodeHandler) {
        contextHandlers[opcode] = handler
    }

    // ========================================================================
    // MARK: - Stack Operations
    // ========================================================================

    /// Push a value onto the untyped stack.
    public func push(_ value: VMValue) {
        stack.append(value)
    }

    /// Pop a value from the untyped stack.
    public func pop() -> VMValue {
        guard !stack.isEmpty else {
            halted = true
            return .null
        }
        return stack.removeLast()
    }

    /// Push a typed value onto the typed stack.
    public func pushTyped(_ value: TypedVMValue) {
        typedStack.append(value)
    }

    /// Pop a typed value from the typed stack.
    public func popTyped() -> TypedVMValue {
        guard !typedStack.isEmpty else {
            halted = true
            return TypedVMValue(type: 0, value: .null)
        }
        return typedStack.removeLast()
    }

    /// Peek at the top typed value without removing it.
    public func peekTyped() -> TypedVMValue {
        guard !typedStack.isEmpty else {
            return TypedVMValue(type: 0, value: .null)
        }
        return typedStack.last!
    }

    // ========================================================================
    // MARK: - PC Control
    // ========================================================================

    /// Advance the program counter by 1.
    public func advancePc() {
        pc += 1
    }

    /// Jump to a specific instruction index.
    public func jumpTo(_ target: Int) {
        pc = target
    }

    // ========================================================================
    // MARK: - Execution
    // ========================================================================

    /// Reset the VM state (preserves handlers).
    public func reset() {
        stack = []
        typedStack = []
        pc = 0
        halted = false
        variables = [:]
    }

    /// Execute a code object until completion.
    public func execute(_ code: CodeObject) {
        pc = 0
        halted = false

        while pc < code.instructions.count && !halted {
            let instruction = code.instructions[pc]
            let oldPc = pc

            if let handler = handlers[instruction.opcode] {
                handler(self, instruction, code)
            } else {
                // Unknown opcode -- halt.
                halted = true
                break
            }

            // If the handler didn't advance the PC, do it automatically.
            if pc == oldPc {
                pc += 1
            }
        }
    }

    /// Execute a code object with an additional context (for WASM).
    ///
    /// Context-aware handlers receive the context as their fourth argument.
    /// Regular handlers still work but don't receive the context.
    public func executeWithContext(_ code: CodeObject, context: AnyObject) {
        pc = 0
        halted = false

        while pc < code.instructions.count && !halted {
            let instruction = code.instructions[pc]
            let oldPc = pc

            if let ctxHandler = contextHandlers[instruction.opcode] {
                ctxHandler(self, instruction, code, context)
            } else if let handler = handlers[instruction.opcode] {
                handler(self, instruction, code)
            } else {
                halted = true
                break
            }

            // If the handler didn't advance the PC, do it automatically.
            if pc == oldPc {
                pc += 1
            }
        }
    }

    /// Set the maximum recursion depth.
    public func setMaxRecursionDepth(_ depth: Int) {
        maxRecursionDepth = depth
    }
}
