// ============================================================================
// Opcodes.swift — Register VM opcode table, V8 Ignition-style
// ============================================================================
//
// V8's Ignition bytecode interpreter uses a register-based design where each
// instruction operates on explicit register operands plus an implicit
// "accumulator" register (similar to an x86 EAX). This opcode table mirrors
// Ignition's categories and naming conventions as an educational reference.
//
// CATEGORY MAP
// 0x0_  Accumulator loads       — load a constant/literal into the accumulator
// 0x1_  Register moves          — copy values between named registers
// 0x2_  Variable access         — read/write globals, locals, and context slots
// 0x3_  Arithmetic              — integer/float arithmetic and bitwise ops
// 0x4_  Comparisons             — relational and type tests, store bool in acc
// 0x5_  Control flow            — jumps, conditional branches, loops
// 0x6_  Calls                   — function/method calls, return, generators
// 0x7_  Property access         — named/keyed get/set, delete
// 0x8_  Object/array creation   — literals, closures, contexts
// 0x9_  Iteration               — for-of iterator protocol helpers
// 0xA_  Exceptions              — throw/rethrow
// 0xB_  Context/scope           — push/pop scope contexts, module variables
// 0xF_  VM control              — stack check, debugger break, halt
//
// ============================================================================

/// All register VM opcodes, organised by category.
///
/// The raw `UInt8` value is the bytecode encoding: the high nibble is the
/// category and the low nibble is the operation within that category.
/// This is purely a convention for readability — the VM uses a `switch`
/// on the raw value and does not rely on the nibble structure at runtime.
public enum Opcode: UInt8, CaseIterable, CustomStringConvertible {

    // ── 0x0_  Accumulator loads ──────────────────────────────────────────────
    // These are the cheapest "push a value" operations. Each loads something
    // into the accumulator register without touching any named register.
    //
    // Analogous to x86 `MOV EAX, <imm>`.

    /// Load a constant from the constant pool into the accumulator.
    /// Operand 0: index into `CodeObject.constants`.
    case ldaConstant  = 0x00

    /// Load integer zero into the accumulator. No operands.
    case ldaZero      = 0x01

    /// Load a small integer (SMI) literal into the accumulator.
    /// Operand 0: the integer value (sign-extended).
    case ldaSmi       = 0x02

    /// Load `undefined` into the accumulator. No operands.
    case ldaUndefined = 0x03

    /// Load `null` into the accumulator. No operands.
    case ldaNull      = 0x04

    /// Load `true` into the accumulator. No operands.
    case ldaTrue      = 0x05

    /// Load `false` into the accumulator. No operands.
    case ldaFalse     = 0x06

    // ── 0x1_  Register moves ─────────────────────────────────────────────────
    // Transfer values between the accumulator and named registers, or between
    // two named registers. Every frame has `CodeObject.registerCount` slots.

    /// Load accumulator from register.
    /// Operand 0: source register index.
    case ldar = 0x10

    /// Store accumulator to register.
    /// Operand 0: destination register index.
    case star = 0x11

    /// Copy one register to another.
    /// Operand 0: destination register index.
    /// Operand 1: source register index.
    case mov  = 0x12

    // ── 0x2_  Variable access ────────────────────────────────────────────────
    // Global variables live in a flat dictionary; locals are in the frame's
    // register array; context slots support closure variables captured from
    // outer scopes.

    /// Load a global variable into the accumulator.
    /// Operand 0: index into `CodeObject.names`.
    case ldaGlobal            = 0x20

    /// Store the accumulator to a global variable.
    /// Operand 0: index into `CodeObject.names`.
    case staGlobal            = 0x21

    /// Load a local variable (register) into the accumulator.
    /// Operand 0: register index (same as `ldar`, kept separate for clarity).
    case ldaLocal             = 0x22

    /// Store the accumulator to a local variable (register).
    /// Operand 0: register index.
    case staLocal             = 0x23

    /// Load a value from a context slot at a given scope depth.
    /// Operand 0: scope depth (0 = current, 1 = parent, …).
    /// Operand 1: slot index within that scope's `Context.slots`.
    case ldaContextSlot       = 0x24

    /// Store the accumulator to a context slot at a given scope depth.
    /// Operand 0: scope depth.
    /// Operand 1: slot index.
    case staContextSlot       = 0x25

    /// Load a value from the current (innermost) context slot.
    /// Operand 0: slot index.
    case ldaCurrentContextSlot = 0x26

    /// Store the accumulator to the current (innermost) context slot.
    /// Operand 0: slot index.
    case staCurrentContextSlot = 0x27

    // ── 0x3_  Arithmetic ─────────────────────────────────────────────────────
    // Binary operations read the left operand from a register and the right
    // operand from the accumulator, then write the result to the accumulator.
    //
    // Unary operations (`bitwiseNot`, `negate`) operate solely on the
    // accumulator.
    //
    // Feedback slots record the runtime types seen at each site so that a
    // hypothetical JIT compiler could specialise the code.

    /// Add register to accumulator. Operand 0: left register. Operand 1: feedback slot.
    case add              = 0x30

    /// Subtract accumulator from register. Operand 0: left register. Operand 1: feedback slot.
    case sub              = 0x31

    /// Multiply register by accumulator. Operand 0: left register. Operand 1: feedback slot.
    case mul              = 0x32

    /// Divide register by accumulator. Operand 0: left register. Operand 1: feedback slot.
    case div              = 0x33

    /// Modulo: register mod accumulator. Operand 0: left register. Operand 1: feedback slot.
    case mod_             = 0x34

    /// Exponentiation: register ** accumulator. Operand 0: left register. Operand 1: feedback slot.
    case pow_             = 0x35

    /// Add a small integer literal to the accumulator.
    /// Operand 0: SMI value.
    case addSmi           = 0x36

    /// Subtract a small integer literal from the accumulator.
    /// Operand 0: SMI value.
    case subSmi           = 0x37

    /// Bitwise AND: register & accumulator. Operand 0: left register.
    case bitwiseAnd       = 0x38

    /// Bitwise OR: register | accumulator. Operand 0: left register.
    case bitwiseOr        = 0x39

    /// Bitwise XOR: register ^ accumulator. Operand 0: left register.
    case bitwiseXor       = 0x3A

    /// Bitwise NOT: ~accumulator. No operands.
    case bitwiseNot       = 0x3B

    /// Shift left: register << accumulator. Operand 0: left register.
    case shiftLeft        = 0x3C

    /// Arithmetic shift right: register >> accumulator. Operand 0: left register.
    case shiftRight       = 0x3D

    /// Logical (unsigned) shift right: register >>> accumulator. Operand 0: left register.
    case shiftRightLogical = 0x3E

    /// Arithmetic negation: accumulator = -accumulator. No operands.
    case negate           = 0x3F

    // ── 0x4_  Comparisons ────────────────────────────────────────────────────
    // Each comparison reads the left operand from a register and the right
    // from the accumulator, then stores a boolean result in the accumulator.
    // The boolean can then be used with conditional jumps.

    /// Abstract equality (==). Operand 0: left register.
    case testEqual              = 0x40

    /// Abstract inequality (!=). Operand 0: left register.
    case testNotEqual           = 0x41

    /// Strict equality (===). Operand 0: left register.
    case testStrictEqual        = 0x42

    /// Strict inequality (!==). Operand 0: left register.
    case testStrictNotEqual     = 0x43

    /// Less-than (<). Operand 0: left register.
    case testLessThan           = 0x44

    /// Greater-than (>). Operand 0: left register.
    case testGreaterThan        = 0x45

    /// Less-than-or-equal (<=). Operand 0: left register.
    case testLessThanOrEqual    = 0x46

    /// Greater-than-or-equal (>=). Operand 0: left register.
    case testGreaterThanOrEqual = 0x47

    /// Test `in` operator: accumulator in object at register. Operand 0: object register.
    case testIn                 = 0x48

    /// Test `instanceof`: accumulator instanceof constructor at register. Operand 0: constructor register.
    case testInstanceOf         = 0x49

    /// Test if accumulator is undetectable (null or undefined). No operands.
    case testUndetectable       = 0x4A

    /// Logical NOT: accumulator = !accumulator. No operands.
    case logicalNot             = 0x4B

    /// Type-of: store type string of accumulator in accumulator. No operands.
    case typeOf                 = 0x4C

    // ── 0x5_  Control flow ───────────────────────────────────────────────────
    // All jumps use an absolute instruction index (not a byte offset) for
    // simplicity in this educational implementation. V8 uses relative byte
    // offsets; we trade bytecode density for readability.

    /// Unconditional jump to instruction index.
    /// Operand 0: target instruction index.
    case jump                  = 0x50

    /// Jump if accumulator is `true`. Operand 0: target index.
    case jumpIfTrue            = 0x51

    /// Jump if accumulator is `false`. Operand 0: target index.
    case jumpIfFalse           = 0x52

    /// Jump if accumulator is `null`. Operand 0: target index.
    case jumpIfNull            = 0x53

    /// Jump if accumulator is `undefined`. Operand 0: target index.
    case jumpIfUndefined       = 0x54

    /// Jump if accumulator is `null` or `undefined`. Operand 0: target index.
    case jumpIfNullOrUndefined = 0x55

    /// Jump if ToBoolean(accumulator) is `true`. Operand 0: target index.
    case jumpIfToBooleanTrue   = 0x56

    /// Jump if ToBoolean(accumulator) is `false`. Operand 0: target index.
    case jumpIfToBooleanFalse  = 0x57

    /// Back-edge jump for loop bodies. Operand 0: target index.
    /// Triggers a stack check (same as `stackCheck`).
    case jumpLoop              = 0x58

    // ── 0x6_  Calls ──────────────────────────────────────────────────────────
    // In V8 Ignition all calls pass through the interpreter; JIT compilation
    // is deferred until a function is called enough times. Here we support
    // native Swift closures stored as `.function` values.

    /// Call function in accumulator with any receiver.
    /// Operand 0: first argument register.
    /// Operand 1: argument count.
    case callAnyReceiver      = 0x60

    /// Call property (method call with explicit receiver).
    /// Operand 0: receiver register.
    /// Operand 1: first argument register.
    /// Operand 2: argument count.
    case callProperty         = 0x61

    /// Call function with undefined as receiver.
    /// Operand 0: first argument register.
    /// Operand 1: argument count.
    case callUndefinedReceiver = 0x62

    /// Construct (new) a function. Operand 0: constructor register. Operand 1: argument count.
    case construct            = 0x63

    /// Construct with spread operator. Operand 0: constructor. Operand 1: argument count.
    case constructWithSpread  = 0x64

    /// Call with spread operator. Operand 0: first argument. Operand 1: argument count.
    case callWithSpread       = 0x65

    /// Return the accumulator to the caller. No operands.
    case return_              = 0x66

    /// Suspend a generator, saving state. Operand 0: generator register.
    case suspendGenerator     = 0x67

    /// Resume a suspended generator. Operand 0: generator register.
    case resumeGenerator      = 0x68

    // ── 0x7_  Property access ────────────────────────────────────────────────

    /// Load a named property from the object in a register into the accumulator.
    /// Operand 0: object register.
    /// Operand 1: name index in `CodeObject.names`.
    /// Operand 2: feedback slot.
    case ldaNamedProperty           = 0x70

    /// Store the accumulator to a named property of the object in a register.
    /// Operand 0: object register.
    /// Operand 1: name index.
    case staNamedProperty           = 0x71

    /// Load a keyed property: `object[key]`. Operand 0: object register. Operand 1: key register.
    case ldaKeyedProperty           = 0x72

    /// Store keyed property: `object[key] = acc`. Operand 0: object register. Operand 1: key register.
    case staKeyedProperty           = 0x73

    /// Same as `ldaNamedProperty` but without feedback recording.
    case ldaNamedPropertyNoFeedback = 0x74

    /// Same as `staNamedProperty` but without feedback recording.
    case staNamedPropertyNoFeedback = 0x75

    /// Delete property in strict mode. Operand 0: object register. Operand 1: key register.
    case deletePropertyStrict       = 0x76

    /// Delete property in sloppy mode. Operand 0: object register. Operand 1: key register.
    case deletePropertySloppy       = 0x77

    // ── 0x8_  Object/array creation ──────────────────────────────────────────

    /// Create an object literal. Operand 0: constant pool index with template.
    case createObjectLiteral = 0x80

    /// Create an array literal. Operand 0: constant pool index with template.
    case createArrayLiteral  = 0x81

    /// Create a RegExp literal. Operand 0: pattern name index. Operand 1: flags name index.
    case createRegExpLiteral = 0x82

    /// Create a closure from a `CodeObject` constant.
    /// Operand 0: constant pool index of the `CodeObject`.
    case createClosure       = 0x83

    /// Push a new scope `Context` for the current function.
    /// Operand 0: slot count for the new context.
    case createContext       = 0x84

    /// Clone an existing object. Operand 0: source object register.
    case cloneObject         = 0x85

    // ── 0x9_  Iteration ──────────────────────────────────────────────────────
    // The iterator protocol: call `Symbol.iterator` on a value, step it, then
    // test the `done` flag and extract `value`.

    /// Get the iterator for the value in the accumulator.
    case getIterator      = 0x90

    /// Call `iterator.next()`. Operand 0: iterator register.
    case callIteratorStep  = 0x91

    /// Test whether the iterator result is `done`. Operand 0: result register.
    case getIteratorDone  = 0x92

    /// Extract the `value` field from an iterator result. Operand 0: result register.
    case getIteratorValue = 0x93

    // ── 0xA_  Exceptions ─────────────────────────────────────────────────────

    /// Throw the value in the accumulator as an exception. No operands.
    case throw_  = 0xA0

    /// Re-throw the current exception. No operands.
    case reThrow = 0xA1

    // ── 0xB_  Context/scope ──────────────────────────────────────────────────

    /// Push a new context (created by `createContext`) as the current scope.
    /// Operand 0: context register.
    case pushContext       = 0xB0

    /// Pop the current context, restoring the parent. No operands.
    case popContext        = 0xB1

    /// Load a module variable into the accumulator.
    /// Operand 0: name index. Operand 1: depth.
    case ldaModuleVariable = 0xB4

    /// Store the accumulator to a module variable.
    /// Operand 0: name index. Operand 1: depth.
    case staModuleVariable = 0xB5

    // ── 0xF_  VM control ─────────────────────────────────────────────────────

    /// Check for stack overflow. Throws `VMError` when call depth exceeds the limit.
    case stackCheck = 0xF0

    /// Trigger a debugger breakpoint (no-op in this implementation). No operands.
    case debugger_  = 0xF1

    /// Halt execution immediately; returns accumulator to caller. No operands.
    case halt       = 0xFF

    // ── Description ──────────────────────────────────────────────────────────

    /// Human-readable opcode name, suitable for disassembly output.
    public var description: String {
        switch self {
        case .ldaConstant:   return "LdaConstant"
        case .ldaZero:       return "LdaZero"
        case .ldaSmi:        return "LdaSmi"
        case .ldaUndefined:  return "LdaUndefined"
        case .ldaNull:       return "LdaNull"
        case .ldaTrue:       return "LdaTrue"
        case .ldaFalse:      return "LdaFalse"
        case .ldar:          return "Ldar"
        case .star:          return "Star"
        case .mov:           return "Mov"
        case .ldaGlobal:     return "LdaGlobal"
        case .staGlobal:     return "StaGlobal"
        case .ldaLocal:      return "LdaLocal"
        case .staLocal:      return "StaLocal"
        case .ldaContextSlot:        return "LdaContextSlot"
        case .staContextSlot:        return "StaContextSlot"
        case .ldaCurrentContextSlot: return "LdaCurrentContextSlot"
        case .staCurrentContextSlot: return "StaCurrentContextSlot"
        case .add:           return "Add"
        case .sub:           return "Sub"
        case .mul:           return "Mul"
        case .div:           return "Div"
        case .mod_:          return "Mod"
        case .pow_:          return "Pow"
        case .addSmi:        return "AddSmi"
        case .subSmi:        return "SubSmi"
        case .bitwiseAnd:    return "BitwiseAnd"
        case .bitwiseOr:     return "BitwiseOr"
        case .bitwiseXor:    return "BitwiseXor"
        case .bitwiseNot:    return "BitwiseNot"
        case .shiftLeft:     return "ShiftLeft"
        case .shiftRight:    return "ShiftRight"
        case .shiftRightLogical: return "ShiftRightLogical"
        case .negate:        return "Negate"
        case .testEqual:              return "TestEqual"
        case .testNotEqual:           return "TestNotEqual"
        case .testStrictEqual:        return "TestStrictEqual"
        case .testStrictNotEqual:     return "TestStrictNotEqual"
        case .testLessThan:           return "TestLessThan"
        case .testGreaterThan:        return "TestGreaterThan"
        case .testLessThanOrEqual:    return "TestLessThanOrEqual"
        case .testGreaterThanOrEqual: return "TestGreaterThanOrEqual"
        case .testIn:                 return "TestIn"
        case .testInstanceOf:         return "TestInstanceOf"
        case .testUndetectable:       return "TestUndetectable"
        case .logicalNot:    return "LogicalNot"
        case .typeOf:        return "TypeOf"
        case .jump:                  return "Jump"
        case .jumpIfTrue:            return "JumpIfTrue"
        case .jumpIfFalse:           return "JumpIfFalse"
        case .jumpIfNull:            return "JumpIfNull"
        case .jumpIfUndefined:       return "JumpIfUndefined"
        case .jumpIfNullOrUndefined: return "JumpIfNullOrUndefined"
        case .jumpIfToBooleanTrue:   return "JumpIfToBooleanTrue"
        case .jumpIfToBooleanFalse:  return "JumpIfToBooleanFalse"
        case .jumpLoop:              return "JumpLoop"
        case .callAnyReceiver:      return "CallAnyReceiver"
        case .callProperty:         return "CallProperty"
        case .callUndefinedReceiver: return "CallUndefinedReceiver"
        case .construct:            return "Construct"
        case .constructWithSpread:  return "ConstructWithSpread"
        case .callWithSpread:       return "CallWithSpread"
        case .return_:              return "Return"
        case .suspendGenerator:     return "SuspendGenerator"
        case .resumeGenerator:      return "ResumeGenerator"
        case .ldaNamedProperty:           return "LdaNamedProperty"
        case .staNamedProperty:           return "StaNamedProperty"
        case .ldaKeyedProperty:           return "LdaKeyedProperty"
        case .staKeyedProperty:           return "StaKeyedProperty"
        case .ldaNamedPropertyNoFeedback: return "LdaNamedPropertyNoFeedback"
        case .staNamedPropertyNoFeedback: return "StaNamedPropertyNoFeedback"
        case .deletePropertyStrict:       return "DeletePropertyStrict"
        case .deletePropertySloppy:       return "DeletePropertySloppy"
        case .createObjectLiteral: return "CreateObjectLiteral"
        case .createArrayLiteral:  return "CreateArrayLiteral"
        case .createRegExpLiteral: return "CreateRegExpLiteral"
        case .createClosure:       return "CreateClosure"
        case .createContext:       return "CreateContext"
        case .cloneObject:         return "CloneObject"
        case .getIterator:      return "GetIterator"
        case .callIteratorStep:  return "CallIteratorStep"
        case .getIteratorDone:  return "GetIteratorDone"
        case .getIteratorValue: return "GetIteratorValue"
        case .throw_:   return "Throw"
        case .reThrow:  return "ReThrow"
        case .pushContext:       return "PushContext"
        case .popContext:        return "PopContext"
        case .ldaModuleVariable: return "LdaModuleVariable"
        case .staModuleVariable: return "StaModuleVariable"
        case .stackCheck: return "StackCheck"
        case .debugger_:  return "Debugger"
        case .halt:       return "Halt"
        }
    }
}
