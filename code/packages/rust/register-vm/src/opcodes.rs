//! # Opcode Constants for the Register VM
//!
//! This module defines every bytecode opcode understood by the register VM.
//! The opcode layout mirrors V8 Ignition's design — opcodes are grouped
//! into logical categories so the dispatch table stays readable.
//!
//! ## Opcode Encoding
//!
//! Each opcode is a single `u8`.  Operands are stored separately in the
//! [`RegisterInstruction`](crate::types::RegisterInstruction) struct so we
//! never have to worry about variable-length encodings at the VM level.
//!
//! ## Category Map
//!
//! | Range   | Category                          |
//! |---------|-----------------------------------|
//! | `0x00`–`0x06` | Accumulator loads           |
//! | `0x10`–`0x12` | Register moves              |
//! | `0x20`–`0x27` | Arithmetic                  |
//! | `0x30`–`0x34` | Comparison                  |
//! | `0x40`–`0x43` | Logical / bitwise           |
//! | `0x50`–`0x56` | Control flow / jumps        |
//! | `0x60`–`0x63` | Property access             |
//! | `0x70`–`0x72` | Element (array) access      |
//! | `0x80`–`0x84` | Calls & returns             |
//! | `0x90`–`0x92` | Object / array construction |
//! | `0xA0`        | Context (closure) ops       |
//! | `0xB0`–`0xB2` | Type introspection          |
//! | `0xFE`        | Stack-overflow check        |
//! | `0xFF`        | Halt / implicit return      |

// ── 0x0_ — Accumulator loads ─────────────────────────────────────────────────

/// Load a value from the constant pool into the accumulator.
/// Operand 0: index into `CodeObject::constants`.
pub const LDA_CONSTANT: u8 = 0x00;

/// Load the integer 0 into the accumulator.
/// Shorthand that avoids a constant-pool entry for the common case.
pub const LDA_ZERO: u8 = 0x01;

/// Load a small integer (SMI) into the accumulator.
/// Operand 0: the integer value (fits in i64 for our purposes).
pub const LDA_SMI: u8 = 0x02;

/// Load `undefined` into the accumulator.
pub const LDA_UNDEFINED: u8 = 0x03;

/// Load `null` into the accumulator.
pub const LDA_NULL: u8 = 0x04;

/// Load `true` into the accumulator.
pub const LDA_TRUE: u8 = 0x05;

/// Load `false` into the accumulator.
pub const LDA_FALSE: u8 = 0x06;

// ── 0x1_ — Register moves ────────────────────────────────────────────────────

/// Load a register into the accumulator.
/// Operand 0: register index.
pub const LDAR: u8 = 0x10;

/// Store the accumulator into a register.
/// Operand 0: register index.
pub const STAR: u8 = 0x11;

/// Move a value from one register to another without touching the accumulator.
/// Operand 0: source register, Operand 1: destination register.
pub const MOV: u8 = 0x12;

// ── 0x2_ — Arithmetic ────────────────────────────────────────────────────────

/// Add a register to the accumulator; result → accumulator.
/// Operand 0: register holding the right-hand side.
pub const ADD: u8 = 0x20;

/// Subtract a register from the accumulator; result → accumulator.
/// Operand 0: register holding the right-hand side.
pub const SUB: u8 = 0x21;

/// Multiply the accumulator by a register; result → accumulator.
/// Operand 0: register holding the right-hand side.
pub const MUL: u8 = 0x22;

/// Divide the accumulator by a register; result → accumulator.
/// Division by zero yields `NaN` (represented as `Float(f64::NAN)`).
/// Operand 0: register holding the right-hand side.
pub const DIV: u8 = 0x23;

/// Modulo: accumulator % register; result → accumulator.
/// Operand 0: register holding the right-hand side.
pub const MOD: u8 = 0x24;

/// Negate the accumulator (unary minus); result → accumulator.
pub const NEG: u8 = 0x25;

/// Increment the accumulator by 1; result → accumulator.
pub const INC: u8 = 0x26;

/// Decrement the accumulator by 1; result → accumulator.
pub const DEC: u8 = 0x27;

// ── 0x3_ — Comparison ────────────────────────────────────────────────────────

/// Strict equality (===): accumulator == register → Bool in accumulator.
/// Operand 0: register holding the right-hand side.
pub const TEST_EQUAL: u8 = 0x30;

/// Strict inequality (!==): accumulator != register → Bool in accumulator.
/// Operand 0: register holding the right-hand side.
pub const TEST_NOT_EQUAL: u8 = 0x31;

/// Less-than (<): accumulator < register → Bool in accumulator.
/// Operand 0: register holding the right-hand side.
pub const TEST_LESS_THAN: u8 = 0x32;

/// Greater-than (>): accumulator > register → Bool in accumulator.
/// Operand 0: register holding the right-hand side.
pub const TEST_GREATER_THAN: u8 = 0x33;

/// Less-than-or-equal (<=): accumulator <= register → Bool in accumulator.
/// Operand 0: register holding the right-hand side.
pub const TEST_LESS_THAN_OR_EQUAL: u8 = 0x34;

/// Greater-than-or-equal (>=): accumulator >= register → Bool in accumulator.
/// Operand 0: register holding the right-hand side.
pub const TEST_GREATER_THAN_OR_EQUAL: u8 = 0x35;

// ── 0x4_ — Logical / bitwise ─────────────────────────────────────────────────

/// Logical NOT of accumulator → Bool in accumulator.
pub const LOGICAL_NOT: u8 = 0x40;

/// Bitwise AND: accumulator & register → accumulator.
/// Operand 0: register holding the right-hand side.
pub const BITWISE_AND: u8 = 0x41;

/// Bitwise OR: accumulator | register → accumulator.
/// Operand 0: register holding the right-hand side.
pub const BITWISE_OR: u8 = 0x42;

/// Bitwise XOR: accumulator ^ register → accumulator.
/// Operand 0: register holding the right-hand side.
pub const BITWISE_XOR: u8 = 0x43;

// ── 0x5_ — Control flow ──────────────────────────────────────────────────────

/// Unconditional jump.
/// Operand 0: absolute instruction index to jump to.
pub const JUMP: u8 = 0x50;

/// Jump if the accumulator is falsy (false, null, undefined, 0, "").
/// Operand 0: absolute instruction index.
pub const JUMP_IF_FALSE: u8 = 0x51;

/// Jump if the accumulator is truthy.
/// Operand 0: absolute instruction index.
pub const JUMP_IF_TRUE: u8 = 0x52;

/// Jump if the accumulator is `null` or `undefined`.
/// Operand 0: absolute instruction index.
pub const JUMP_IF_NULL_OR_UNDEFINED: u8 = 0x53;

/// Jump if the accumulator is NOT `null` or `undefined`.
/// Operand 0: absolute instruction index.
pub const JUMP_IF_NOT_NULL_OR_UNDEFINED: u8 = 0x54;

/// Jump to a loop back-edge (identical semantics to JUMP, distinguished for
/// profiling purposes in real engines).
/// Operand 0: absolute instruction index.
pub const JUMP_LOOP: u8 = 0x55;

/// Return the accumulator value to the caller.
pub const RETURN: u8 = 0x56;

// ── 0x6_ — Property access ───────────────────────────────────────────────────

/// Load a named property from the object in a register into the accumulator.
/// Operand 0: register holding the object.
/// Operand 1: index into `CodeObject::names` giving the property name.
pub const LDA_NAMED_PROPERTY: u8 = 0x60;

/// Store the accumulator as a named property on an object in a register.
/// Operand 0: register holding the object.
/// Operand 1: index into `CodeObject::names` giving the property name.
pub const STA_NAMED_PROPERTY: u8 = 0x61;

/// Load a named global variable into the accumulator.
/// Operand 0: index into `CodeObject::names` giving the variable name.
pub const LDA_GLOBAL: u8 = 0x62;

/// Store the accumulator as a named global variable.
/// Operand 0: index into `CodeObject::names` giving the variable name.
pub const STA_GLOBAL: u8 = 0x63;

// ── 0x7_ — Element (array) access ────────────────────────────────────────────

/// Load an element from an array into the accumulator.
/// Operand 0: register holding the array.
/// Operand 1: register holding the (integer) index.
pub const LDA_KEYED_PROPERTY: u8 = 0x70;

/// Store the accumulator as an element of an array.
/// Operand 0: register holding the array.
/// Operand 1: register holding the (integer) index.
pub const STA_KEYED_PROPERTY: u8 = 0x71;

/// Get the `.length` property of an array or string into the accumulator.
/// Operand 0: register holding the array / string.
pub const GET_LENGTH: u8 = 0x72;

// ── 0x8_ — Calls & returns ───────────────────────────────────────────────────

/// Call a function with an arbitrary receiver.
/// The callee must be in the accumulator; argument registers are a contiguous
/// range starting at `first_arg_register`.
///
/// Operand 0: register holding the receiver (`this`).
/// Operand 1: first argument register.
/// Operand 2: argument count.
pub const CALL_ANY_RECEIVER: u8 = 0x80;

/// Call a function with a `undefined` receiver (strict-mode call).
/// Operand 0: register holding the callee.
/// Operand 1: first argument register.
/// Operand 2: argument count.
pub const CALL_UNDEFINED_RECEIVER: u8 = 0x81;

/// Tail-call variant of `CALL_ANY_RECEIVER`.  Reuses the current frame.
/// Operand 0: register holding the receiver.
/// Operand 1: first argument register.
/// Operand 2: argument count.
pub const TAIL_CALL: u8 = 0x82;

/// Call a runtime built-in identified by index.
/// Operand 0: built-in index (see `VM::call_runtime`).
/// Operand 1: first argument register.
/// Operand 2: argument count.
pub const CALL_RUNTIME: u8 = 0x83;

/// Intrinsic print — writes the accumulator's display string to output.
/// Convenience opcode; equivalent to CALL_RUNTIME 0.
pub const INTRINSIC_PRINT: u8 = 0x84;

// ── 0x9_ — Object / array construction ───────────────────────────────────────

/// Create a new empty object `{}` and load it into the accumulator.
pub const CREATE_OBJECT_LITERAL: u8 = 0x90;

/// Create a new empty array `[]` and load it into the accumulator.
pub const CREATE_ARRAY_LITERAL: u8 = 0x91;

/// Push the accumulator onto the array in a register.
/// Operand 0: register holding the target array.
pub const ARRAY_PUSH: u8 = 0x92;

// ── 0xA_ — Context (closure) ops ─────────────────────────────────────────────

/// Load a value from the closure context at a given depth and slot index.
/// Operand 0: scope depth (0 = current, 1 = parent, …).
/// Operand 1: slot index within that scope.
pub const LDA_CONTEXT_SLOT: u8 = 0xA0;

/// Store the accumulator into the closure context.
/// Operand 0: scope depth.
/// Operand 1: slot index.
pub const STA_CONTEXT_SLOT: u8 = 0xA1;

/// Create a new inner context with the given number of slots, inheriting the
/// current context as parent.
/// Operand 0: number of slots.
pub const CREATE_CONTEXT: u8 = 0xA2;

// ── 0xB_ — Type introspection ────────────────────────────────────────────────

/// Load a string describing the type of the accumulator (JS `typeof` semantics)
/// into the accumulator.
pub const TYPE_OF: u8 = 0xB0;

/// Test whether the accumulator is strictly equal to `undefined`.
/// Result: Bool in accumulator.
pub const TEST_UNDEFINED: u8 = 0xB1;

/// Test whether the accumulator is strictly equal to `null`.
/// Result: Bool in accumulator.
pub const TEST_NULL: u8 = 0xB2;

// ── 0xFE — Stack-overflow check ──────────────────────────────────────────────

/// Check that call depth has not exceeded the configured maximum.
/// Returns a `VMError` if the limit is reached.
pub const STACK_CHECK: u8 = 0xFE;

// ── 0xFF — Halt ───────────────────────────────────────────────────────────────

/// Halt execution and return the current accumulator value.
/// This is the terminal opcode; every well-formed program ends with it.
pub const HALT: u8 = 0xFF;

/// Returns the human-readable name of an opcode.
///
/// Useful for disassemblers, error messages, and debug output.
///
/// # Examples
///
/// ```
/// use register_vm::opcodes::{opcode_name, LDA_CONSTANT, HALT};
/// assert_eq!(opcode_name(LDA_CONSTANT), "LDA_CONSTANT");
/// assert_eq!(opcode_name(HALT), "HALT");
/// assert_eq!(opcode_name(0xEE), "UNKNOWN");
/// ```
pub fn opcode_name(op: u8) -> &'static str {
    match op {
        LDA_CONSTANT => "LDA_CONSTANT",
        LDA_ZERO => "LDA_ZERO",
        LDA_SMI => "LDA_SMI",
        LDA_UNDEFINED => "LDA_UNDEFINED",
        LDA_NULL => "LDA_NULL",
        LDA_TRUE => "LDA_TRUE",
        LDA_FALSE => "LDA_FALSE",
        LDAR => "LDAR",
        STAR => "STAR",
        MOV => "MOV",
        ADD => "ADD",
        SUB => "SUB",
        MUL => "MUL",
        DIV => "DIV",
        MOD => "MOD",
        NEG => "NEG",
        INC => "INC",
        DEC => "DEC",
        TEST_EQUAL => "TEST_EQUAL",
        TEST_NOT_EQUAL => "TEST_NOT_EQUAL",
        TEST_LESS_THAN => "TEST_LESS_THAN",
        TEST_GREATER_THAN => "TEST_GREATER_THAN",
        TEST_LESS_THAN_OR_EQUAL => "TEST_LESS_THAN_OR_EQUAL",
        TEST_GREATER_THAN_OR_EQUAL => "TEST_GREATER_THAN_OR_EQUAL",
        LOGICAL_NOT => "LOGICAL_NOT",
        BITWISE_AND => "BITWISE_AND",
        BITWISE_OR => "BITWISE_OR",
        BITWISE_XOR => "BITWISE_XOR",
        JUMP => "JUMP",
        JUMP_IF_FALSE => "JUMP_IF_FALSE",
        JUMP_IF_TRUE => "JUMP_IF_TRUE",
        JUMP_IF_NULL_OR_UNDEFINED => "JUMP_IF_NULL_OR_UNDEFINED",
        JUMP_IF_NOT_NULL_OR_UNDEFINED => "JUMP_IF_NOT_NULL_OR_UNDEFINED",
        JUMP_LOOP => "JUMP_LOOP",
        RETURN => "RETURN",
        LDA_NAMED_PROPERTY => "LDA_NAMED_PROPERTY",
        STA_NAMED_PROPERTY => "STA_NAMED_PROPERTY",
        LDA_GLOBAL => "LDA_GLOBAL",
        STA_GLOBAL => "STA_GLOBAL",
        LDA_KEYED_PROPERTY => "LDA_KEYED_PROPERTY",
        STA_KEYED_PROPERTY => "STA_KEYED_PROPERTY",
        GET_LENGTH => "GET_LENGTH",
        CALL_ANY_RECEIVER => "CALL_ANY_RECEIVER",
        CALL_UNDEFINED_RECEIVER => "CALL_UNDEFINED_RECEIVER",
        TAIL_CALL => "TAIL_CALL",
        CALL_RUNTIME => "CALL_RUNTIME",
        INTRINSIC_PRINT => "INTRINSIC_PRINT",
        CREATE_OBJECT_LITERAL => "CREATE_OBJECT_LITERAL",
        CREATE_ARRAY_LITERAL => "CREATE_ARRAY_LITERAL",
        ARRAY_PUSH => "ARRAY_PUSH",
        LDA_CONTEXT_SLOT => "LDA_CONTEXT_SLOT",
        STA_CONTEXT_SLOT => "STA_CONTEXT_SLOT",
        CREATE_CONTEXT => "CREATE_CONTEXT",
        TYPE_OF => "TYPE_OF",
        TEST_UNDEFINED => "TEST_UNDEFINED",
        TEST_NULL => "TEST_NULL",
        STACK_CHECK => "STACK_CHECK",
        HALT => "HALT",
        _ => "UNKNOWN",
    }
}
