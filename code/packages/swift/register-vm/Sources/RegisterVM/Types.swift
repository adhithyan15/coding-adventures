// ============================================================================
// Types.swift — Core data types for the register VM
// ============================================================================
//
// This file defines all the value and bookkeeping types that flow through the
// register VM at runtime.
//
// DATA MODEL OVERVIEW
// ───────────────────
// VMValue           — the universal value type; every register holds one
// VMObject          — heap-allocated property bag (JS object model)
// CodeObject        — compiled bytecode unit (like a Python code object)
// RegisterInstruction — a single decoded instruction with opcode + operands
// CallFrame         — live state for one activation (one function call)
// Context           — scope chain node for capturing closure variables
// VMResult          — return value from `RegisterVM.execute(_:)`
// VMError           — typed error thrown during execution
//
// INDIRECT ENUM
// ─────────────
// `VMValue` is marked `indirect` so that `.object(VMObject)`, `.array`,
// and `.function` can hold arbitrarily nested values without the enum
// becoming a fixed-size monster.  The compiler inserts a heap pointer
// automatically for the recursive cases.
//
// HIDDEN CLASS IDs
// ────────────────
// V8 assigns "hidden classes" (also called "maps") to objects that share the
// same property layout.  When two objects have the same hidden class ID,
// a JIT compiler knows they have the same property offsets and can emit a
// fast inline cache hit.  In this educational VM we use a monotonically
// increasing integer to simulate that concept.
//
// ============================================================================

import Foundation

// ============================================================================
// MARK: - VMValue
// ============================================================================

/// A value that the register VM can hold in any register or on the call stack.
///
/// The `indirect` keyword lets `.object`, `.array`, and `.function` store
/// other `VMValue` instances recursively without blowing up the enum's size.
///
/// Truth table for `isTruthy`:
/// ┌─────────────────────────┬─────────┐
/// │ Value                   │ Truthy  │
/// ├─────────────────────────┼─────────┤
/// │ .boolean(false)         │  false  │
/// │ .null                   │  false  │
/// │ .undefined              │  false  │
/// │ .integer(0)             │  false  │
/// │ .float(0.0)             │  false  │
/// │ .string("")             │  false  │
/// │ everything else         │  true   │
/// └─────────────────────────┴─────────┘
public indirect enum VMValue {
    /// A signed machine-word integer (64-bit on 64-bit platforms).
    case integer(Int)

    /// A 64-bit floating-point number.
    case float(Double)

    /// An immutable string.
    case string(String)

    /// A boolean.
    case boolean(Bool)

    /// JavaScript `null`.
    case null

    /// JavaScript `undefined`.
    case undefined

    /// A heap-allocated JS-style object (property bag).
    case object(VMObject)

    /// A JS-style array (ordered list of values).
    case array([VMValue])

    /// A callable: a compiled `CodeObject` paired with the optional lexical
    /// scope `Context` at the time the function was created (closure).
    case function(CodeObject, Context?)

    /// Whether this value is "truthy" in the JS sense.
    ///
    /// Only six values are falsy: `false`, `null`, `undefined`, `0`, `0.0`,
    /// and the empty string.  Everything else — including empty arrays and
    /// objects — is truthy.
    public var isTruthy: Bool {
        switch self {
        case .boolean(let b):      return b
        case .null:                return false
        case .undefined:           return false
        case .integer(let n):      return n != 0
        case .float(let f):        return f != 0.0 && !f.isNaN
        case .string(let s):       return !s.isEmpty
        case .object:              return true
        case .array:               return true
        case .function:            return true
        }
    }
}

// ============================================================================
// MARK: - VMObject
// ============================================================================

/// A heap-allocated JS-style object: a dictionary of named properties.
///
/// The `hiddenClassId` simulates V8's hidden class (also called a "map").
/// When multiple objects share the same hidden class ID, a JIT can generate
/// a single inline cache entry covering all of them.
///
/// Example — creating an object with two properties:
/// ```swift
/// let obj = VMObject(hiddenClassId: nextHiddenClassId(), properties: [
///     "x": .integer(1),
///     "y": .integer(2),
/// ])
/// ```
public class VMObject {
    /// The hidden class identifier assigned at construction time.
    public let hiddenClassId: Int

    /// The property store.  Property names are always `String`s in this VM.
    public var properties: [String: VMValue]

    /// Create a new object.
    /// - Parameters:
    ///   - hiddenClassId: A unique shape identifier (use `nextHiddenClassId()`).
    ///   - properties: Initial property values (default empty).
    public init(hiddenClassId: Int, properties: [String: VMValue] = [:]) {
        self.hiddenClassId = hiddenClassId
        self.properties = properties
    }
}

// ============================================================================
// MARK: - CodeObject
// ============================================================================

/// A compiled bytecode unit — the "code" half of a function.
///
/// In CPython, this is `PyCodeObject`.  In V8, it is `SharedFunctionInfo`
/// paired with `BytecodeArray`.  Here we keep it simple: one struct that holds
/// everything the interpreter needs to execute a function.
///
/// ```
/// ┌─────────────────────────────────────┐
/// │ CodeObject                          │
/// │  name:           "add"              │
/// │  parameterCount: 2                  │
/// │  registerCount:  4                  │
/// │  constants:      [1, 2, 3.14, "hi"] │
/// │  names:          ["x", "result"]    │
/// │  instructions:   [LdaConstant 0,    │
/// │                   Star r0,          │
/// │                   Return]           │
/// └─────────────────────────────────────┘
/// ```
public struct CodeObject {
    /// The bytecode sequence; executed sequentially unless a jump occurs.
    public var instructions: [RegisterInstruction]

    /// Literal values referenced by `ldaConstant` and friends.
    public var constants: [VMValue]

    /// Identifier strings referenced by `ldaGlobal`, `ldaNamedProperty`, etc.
    public var names: [String]

    /// Total number of registers this function uses.  The call frame
    /// pre-allocates `registerCount` slots initialised to `.undefined`.
    public var registerCount: Int

    /// Number of inline-cache feedback slots.  Each binary-op or property
    /// access instruction that records type feedback has a slot index.
    public var feedbackSlotCount: Int

    /// Number of formal parameters (not including `this`).
    public var parameterCount: Int

    /// Debug name shown in error messages and disassembly.
    public var name: String

    /// Create a `CodeObject`.
    public init(
        instructions: [RegisterInstruction],
        constants: [VMValue],
        names: [String],
        registerCount: Int,
        feedbackSlotCount: Int,
        parameterCount: Int = 0,
        name: String = "anonymous"
    ) {
        self.instructions = instructions
        self.constants = constants
        self.names = names
        self.registerCount = registerCount
        self.feedbackSlotCount = feedbackSlotCount
        self.parameterCount = parameterCount
        self.name = name
    }
}

// ============================================================================
// MARK: - RegisterInstruction
// ============================================================================

/// A single decoded instruction ready for the interpreter dispatch loop.
///
/// Encoding example:
/// ```
/// ldaConstant  [0]          → load constants[0] into accumulator
/// add          [r1, slot:0] → acc = registers[r1] + acc; record at slot 0
/// return_      []           → return accumulator
/// ```
public struct RegisterInstruction {
    /// The raw opcode byte.
    public var opcode: UInt8

    /// Decoded operand values.  Meaning depends on the opcode; see `Opcode`
    /// doc comments for per-opcode semantics.
    public var operands: [Int]

    /// Optional inline-cache feedback slot index for type-recording opcodes.
    public var feedbackSlot: Int?

    /// Create from a raw byte.
    public init(opcode: UInt8, operands: [Int] = [], feedbackSlot: Int? = nil) {
        self.opcode = opcode
        self.operands = operands
        self.feedbackSlot = feedbackSlot
    }

    /// Convenience initialiser that takes a typed `Opcode` value.
    public init(opcode: Opcode, operands: [Int] = [], feedbackSlot: Int? = nil) {
        self.init(opcode: opcode.rawValue, operands: operands, feedbackSlot: feedbackSlot)
    }
}

// ============================================================================
// MARK: - CallFrame
// ============================================================================

/// Live execution state for one function invocation.
///
/// The interpreter keeps a linked list of frames via `callerFrame`.
/// When a `return_` opcode fires, the VM discards the current frame and
/// resumes execution from where the caller left off.
///
/// ```
/// ┌──────────────────────────────────────┐
/// │ CallFrame (callee)                   │
/// │  ip: 7                               │
/// │  accumulator: .integer(42)           │
/// │  registers: [.integer(1), .undefined]│
/// │  feedbackVector: [uninitialized, …]  │
/// │  callerFrame: ──────────────────────►│
/// └──────────────────────────────────────┘ CallFrame (caller)
/// ```
public class CallFrame {
    /// The `CodeObject` being executed in this frame.
    public var code: CodeObject

    /// Instruction pointer — index of the *next* instruction to execute.
    public var ip: Int

    /// The implicit accumulator register.  Most opcodes read from or write to
    /// this rather than naming a register explicitly.
    public var accumulator: VMValue

    /// Named registers.  Indexed by `Int`; pre-allocated to `.undefined`.
    public var registers: [VMValue]

    /// Per-site type-feedback vector.  One entry per `feedbackSlotCount`.
    public var feedbackVector: [FeedbackSlot]

    /// The current lexical scope (closure variable storage).
    public var context: Context?

    /// The frame that called into this one; `nil` for the top-level frame.
    public weak var callerFrame: CallFrame?

    /// Create a fresh call frame for the given code object.
    /// - Parameters:
    ///   - code: The function to execute.
    ///   - callerFrame: The enclosing frame (`nil` at the top level).
    public init(code: CodeObject, callerFrame: CallFrame?) {
        self.code = code
        self.ip = 0
        self.accumulator = .undefined
        self.registers = Array(repeating: .undefined, count: max(code.registerCount, 1))
        self.feedbackVector = FeedbackSlot.newVector(size: code.feedbackSlotCount)
        self.context = nil
        self.callerFrame = callerFrame
    }
}

// ============================================================================
// MARK: - VMResult
// ============================================================================

/// The result returned by `RegisterVM.execute(_:)`.
public struct VMResult {
    /// The value that the top-level code object left in the accumulator.
    public var returnValue: VMValue

    /// Any strings emitted via `print` (the global `print` function stub).
    public var output: [String]

    /// Non-nil if execution terminated with an error.
    public var error: VMError?
}

// ============================================================================
// MARK: - VMError
// ============================================================================

/// A typed error thrown during VM execution.
///
/// Carries enough context to produce a useful error message:
/// ```
/// VMError: Division by zero at instruction 7 (opcode 0x33)
/// ```
public struct VMError: Error {
    /// Human-readable description of what went wrong.
    public var message: String

    /// Index into `CodeObject.instructions` at which the error occurred.
    public var instructionIndex: Int

    /// Raw opcode byte of the faulting instruction.
    public var opcode: UInt8

    /// Create a `VMError`.
    public init(message: String, instructionIndex: Int = 0, opcode: UInt8 = 0) {
        self.message = message
        self.instructionIndex = instructionIndex
        self.opcode = opcode
    }
}

// ============================================================================
// MARK: - Context
// ============================================================================

/// A scope node in the lexical environment chain.
///
/// Closures capture a `Context` at the point they are created.  When they
/// execute, the VM walks up the chain via `parent` references to resolve
/// free variables.
///
/// ```
/// global context (depth 2)
///   └── outer function context (depth 1)
///         └── inner function context (depth 0, current)
/// ```
///
/// `ldaContextSlot depth:0 idx:1` reads `slots[1]` from the current context.
/// `ldaContextSlot depth:1 idx:0` reads `slots[0]` from the parent context.
public class Context {
    /// Closure variable storage.
    public var slots: [VMValue]

    /// The enclosing scope; `nil` at the global level.
    public var parent: Context?

    /// Create a new context with `slotCount` pre-allocated slots.
    /// - Parameters:
    ///   - slotCount: Number of captured variables this scope holds.
    ///   - parent: The enclosing scope context.
    public init(slotCount: Int, parent: Context? = nil) {
        self.slots = Array(repeating: .undefined, count: slotCount)
        self.parent = parent
    }

    /// Read a slot, walking up `depth` levels in the scope chain first.
    ///
    /// - Parameters:
    ///   - depth: 0 = this context, 1 = parent, 2 = grandparent, …
    ///   - idx: Slot index within that context.
    /// - Returns: The stored value, or `.undefined` if the chain is shorter
    ///   than `depth` or the index is out of range.
    public func getSlot(depth: Int, idx: Int) -> VMValue {
        var ctx: Context? = self
        for _ in 0..<depth {
            ctx = ctx?.parent
        }
        guard let c = ctx, idx < c.slots.count else { return .undefined }
        return c.slots[idx]
    }

    /// Write a slot, walking up `depth` levels first.
    ///
    /// - Parameters:
    ///   - depth: 0 = this context, 1 = parent, …
    ///   - idx: Slot index.
    ///   - value: Value to store.
    public func setSlot(depth: Int, idx: Int, value: VMValue) {
        var ctx: Context? = self
        for _ in 0..<depth {
            ctx = ctx?.parent
        }
        guard let c = ctx, idx < c.slots.count else { return }
        c.slots[idx] = value
    }
}
