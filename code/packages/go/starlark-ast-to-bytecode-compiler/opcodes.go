// opcodes.go — Defines the complete set of bytecode opcodes for the Starlark VM.
//
// ════════════════════════════════════════════════════════════════════════
// WHY A SEPARATE OPCODE FILE?
// ════════════════════════════════════════════════════════════════════════
//
// The virtual-machine package defines a small set of general-purpose opcodes
// (LOAD_CONST, ADD, JUMP, etc.) that work for any simple language. But Starlark
// needs more opcodes for its richer semantics:
//
//   - Boolean literals (True, False, None)
//   - Floor division (//) and modulo (%)
//   - Bitwise operators (&, |, ^, ~, <<, >>)
//   - Rich comparisons (!=, <=, >=, in, not in)
//   - Short-circuit boolean operators (and, or)
//   - Collections (list, dict, tuple)
//   - Attribute access and subscript
//   - Iteration (for loops, comprehensions)
//   - Function definition and calling
//   - Module loading
//   - Loop control (break, continue)
//
// This file defines ALL 46 opcodes used by the Starlark compiler and VM.
// Some reuse the existing virtual-machine opcodes (same hex values), while
// new ones fill in the gaps.
//
// ════════════════════════════════════════════════════════════════════════
// OPCODE MAP — HEX LAYOUT
// ════════════════════════════════════════════════════════════════════════
//
// Opcodes are organized by category in hexadecimal ranges:
//
//   0x01-0x06  Stack manipulation (load constants, push literals)
//   0x10-0x15  Name/variable operations (store, load, closures)
//   0x20-0x2D  Arithmetic and bitwise operators
//   0x30-0x38  Comparison and logical operators
//   0x40-0x46  Control flow (jumps, break, continue)
//   0x50-0x53  Functions (make, call, return)
//   0x60-0x64  Collections (list, dict, tuple)
//   0x70-0x74  Subscript and attribute access
//   0x80-0x82  Iteration
//   0x90-0x91  Modules
//   0xA0       Print (moved from 0x60 to avoid conflict with BuildList)
//   0xFF       Halt
//
package starlarkcompiler

import (
	vm "github.com/adhithyan15/coding-adventures/code/packages/go/virtual-machine"
)

// ════════════════════════════════════════════════════════════════════════
// REUSED OPCODES — These match the existing virtual-machine definitions.
// ════════════════════════════════════════════════════════════════════════
//
// We create local aliases so that compiler code can refer to all opcodes
// consistently through this package, without mixing vm.OpXxx and local
// OpXxx references. Each alias maps to the exact same integer value
// defined in the virtual-machine package.

const (
	// OpLoadConst pushes a constant from the constants pool onto the stack.
	// Operand: index into the constants array.
	//   Before: [...]
	//   After:  [..., constants[index]]
	OpLoadConst = vm.OpLoadConst // 0x01

	// OpPop discards the top value from the stack.
	//   Before: [..., value]
	//   After:  [...]
	OpPop = vm.OpPop // 0x02

	// OpDup duplicates the top value on the stack.
	//   Before: [..., value]
	//   After:  [..., value, value]
	OpDup = vm.OpDup // 0x03

	// OpStoreName stores the top-of-stack value into a named variable.
	// Operand: index into the names array.
	//   Before: [..., value]
	//   After:  [...]
	//   Side effect: variables[names[index]] = value
	OpStoreName = vm.OpStoreName // 0x10

	// OpLoadName pushes the value of a named variable onto the stack.
	// Operand: index into the names array.
	//   Before: [...]
	//   After:  [..., variables[names[index]]]
	OpLoadName = vm.OpLoadName // 0x11

	// OpStoreLocal stores into a numbered local variable slot.
	// Operand: slot index.
	OpStoreLocal = vm.OpStoreLocal // 0x12

	// OpLoadLocal pushes a local variable by slot index.
	// Operand: slot index.
	OpLoadLocal = vm.OpLoadLocal // 0x13

	// Arithmetic operators pop two operands (b then a) and push the result.
	// Stack: [..., a, b] -> [..., a OP b]
	OpAdd = vm.OpAdd // 0x20  addition
	OpSub = vm.OpSub // 0x21  subtraction
	OpMul = vm.OpMul // 0x22  multiplication
	OpDiv = vm.OpDiv // 0x23  true division

	// Comparison operators pop two operands and push a boolean result.
	// Stack: [..., a, b] -> [..., a OP b]
	OpCmpEq = vm.OpCmpEq // 0x30  ==
	OpCmpLt = vm.OpCmpLt // 0x31  <
	OpCmpGt = vm.OpCmpGt // 0x32  >

	// Jump instructions modify the program counter.
	OpJump        = vm.OpJump        // 0x40  unconditional jump
	OpJumpIfFalse = vm.OpJumpIfFalse // 0x41  jump if TOS is falsy (pops)
	OpJumpIfTrue  = vm.OpJumpIfTrue  // 0x42  jump if TOS is truthy (pops)

	// OpHalt stops VM execution.
	OpHalt = vm.OpHalt // 0xFF
)

// ════════════════════════════════════════════════════════════════════════
// NEW OPCODES — Starlark-specific extensions.
// ════════════════════════════════════════════════════════════════════════
//
// These fill in the hex ranges left open by the base VM opcodes.
// Each category is documented with stack diagrams showing the before/after
// state of the operand stack.

const (
	// ── Stack Literals ─────────────────────────────────────────────────
	// These push well-known Starlark values without needing a constants entry.

	// OpLoadNone pushes Starlark's None value (Go nil) onto the stack.
	//   Before: [...]
	//   After:  [..., nil]
	OpLoadNone vm.OpCode = 0x04

	// OpLoadTrue pushes boolean true onto the stack.
	//   Before: [...]
	//   After:  [..., true]
	OpLoadTrue vm.OpCode = 0x05

	// OpLoadFalse pushes boolean false onto the stack.
	//   Before: [...]
	//   After:  [..., false]
	OpLoadFalse vm.OpCode = 0x06

	// ── Closure Variables ──────────────────────────────────────────────
	// Starlark functions can capture variables from enclosing scopes.
	// These opcodes support that by reading/writing "closure" slots that
	// the VM implementation can define as needed.

	// OpStoreClosure stores TOS into a closure variable.
	// Operand: closure slot index.
	OpStoreClosure vm.OpCode = 0x14

	// OpLoadClosure pushes a closure variable onto the stack.
	// Operand: closure slot index.
	OpLoadClosure vm.OpCode = 0x15

	// ── Extended Arithmetic ────────────────────────────────────────────
	// Starlark supports floor division (//), modulo (%), exponentiation (**),
	// and bitwise operators — none of which exist in the base VM.

	// OpFloorDiv: integer division that rounds toward negative infinity.
	//   7 // 2 = 3      (not 3.5)
	//  -7 // 2 = -4     (not -3, because floor rounds down)
	//   Stack: [..., a, b] -> [..., a // b]
	OpFloorDiv vm.OpCode = 0x24

	// OpMod: remainder after floor division.
	//   7 % 3 = 1
	//   Stack: [..., a, b] -> [..., a % b]
	OpMod vm.OpCode = 0x25

	// OpPower: exponentiation.
	//   2 ** 10 = 1024
	//   Stack: [..., a, b] -> [..., a ** b]
	OpPower vm.OpCode = 0x26

	// OpNegate: arithmetic negation (unary minus).
	//   -(5) = -5
	//   Stack: [..., a] -> [..., -a]
	OpNegate vm.OpCode = 0x27

	// Bitwise operators work on integers at the bit level.
	// Truth table for single-bit AND, OR, XOR:
	//
	//   a | b | a&b | a|b | a^b
	//   ──┼───┼─────┼─────┼─────
	//   0 | 0 |  0  |  0  |  0
	//   0 | 1 |  0  |  1  |  1
	//   1 | 0 |  0  |  1  |  1
	//   1 | 1 |  1  |  1  |  0

	// OpBitAnd: bitwise AND.
	//   Stack: [..., a, b] -> [..., a & b]
	OpBitAnd vm.OpCode = 0x28

	// OpBitOr: bitwise OR.
	//   Stack: [..., a, b] -> [..., a | b]
	OpBitOr vm.OpCode = 0x29

	// OpBitXor: bitwise XOR (exclusive or).
	//   Stack: [..., a, b] -> [..., a ^ b]
	OpBitXor vm.OpCode = 0x2A

	// OpBitNot: bitwise complement (unary).
	//   ~0 = -1 (flips all bits in two's complement)
	//   Stack: [..., a] -> [..., ~a]
	OpBitNot vm.OpCode = 0x2B

	// OpLShift: left bit shift.
	//   1 << 3 = 8
	//   Stack: [..., a, b] -> [..., a << b]
	OpLShift vm.OpCode = 0x2C

	// OpRShift: right bit shift.
	//   8 >> 2 = 2
	//   Stack: [..., a, b] -> [..., a >> b]
	OpRShift vm.OpCode = 0x2D

	// ── Extended Comparisons ───────────────────────────────────────────
	// The base VM has ==, <, > (0x30-0x32). We add the rest at 0x33+.
	// Note: the Go VM uses 0x31 for Lt and 0x32 for Gt, so we start new
	// comparison opcodes at 0x33 to avoid conflicts.

	// OpCmpNe: not equal.
	//   Stack: [..., a, b] -> [..., a != b]
	OpCmpNe vm.OpCode = 0x33

	// OpCmpLe: less than or equal.
	//   Stack: [..., a, b] -> [..., a <= b]
	OpCmpLe vm.OpCode = 0x34

	// OpCmpGe: greater than or equal.
	//   Stack: [..., a, b] -> [..., a >= b]
	OpCmpGe vm.OpCode = 0x35

	// OpCmpIn: membership test.
	//   "a" in ["a", "b"] -> True
	//   Stack: [..., element, container] -> [..., element in container]
	OpCmpIn vm.OpCode = 0x36

	// OpCmpNotIn: negated membership test.
	//   "c" not in ["a", "b"] -> True
	//   Stack: [..., element, container] -> [..., element not in container]
	OpCmpNotIn vm.OpCode = 0x37

	// OpNot: logical NOT.
	//   not True -> False
	//   not 0 -> True
	//   Stack: [..., a] -> [..., not a]
	OpNot vm.OpCode = 0x38

	// ── Control Flow Extensions ────────────────────────────────────────

	// OpJumpIfFalseOrPop: short-circuit AND.
	// If TOS is falsy, jump to target (leave TOS on stack).
	// If TOS is truthy, pop it and continue to the next instruction.
	// This implements "a and b" — if a is falsy, the result is a (skip b).
	//   Operand: jump target
	OpJumpIfFalseOrPop vm.OpCode = 0x43

	// OpJumpIfTrueOrPop: short-circuit OR.
	// If TOS is truthy, jump to target (leave TOS on stack).
	// If TOS is falsy, pop it and continue to the next instruction.
	// This implements "a or b" — if a is truthy, the result is a (skip b).
	//   Operand: jump target
	OpJumpIfTrueOrPop vm.OpCode = 0x44

	// OpBreak: exits the innermost for loop.
	// The VM or compiler must resolve this to a jump to the loop's exit.
	OpBreak vm.OpCode = 0x45

	// OpContinue: jumps to the next iteration of the innermost for loop.
	// The VM or compiler must resolve this to a jump to the loop header.
	OpContinue vm.OpCode = 0x46

	// ── Functions ──────────────────────────────────────────────────────

	// OpMakeFunction: creates a function object from a CodeObject.
	// Operand: index into constants where the CodeObject is stored.
	//   Before: [..., default_n, ..., default_1]  (n defaults on stack)
	//   After:  [..., function_object]
	OpMakeFunction vm.OpCode = 0x50

	// OpCallFunction: calls a function with positional arguments.
	// Operand: number of positional arguments.
	//   Before: [..., func, arg_1, ..., arg_n]
	//   After:  [..., return_value]
	OpCallFunction vm.OpCode = 0x51

	// OpCallFunctionKW: calls a function with keyword arguments.
	// Operand: number of keyword argument pairs (each pair is name, value).
	// The stack also holds a tuple of keyword names below the values.
	OpCallFunctionKW vm.OpCode = 0x52

	// OpReturnValue: returns from the current function.
	//   Before: [..., return_value]
	//   After:  (return_value passed to caller's stack)
	OpReturnValue vm.OpCode = 0x53

	// ── Collections ────────────────────────────────────────────────────

	// OpBuildList: creates a list from N items on the stack.
	// Operand: number of items.
	//   Before: [..., item_1, item_2, ..., item_n]
	//   After:  [..., [item_1, item_2, ..., item_n]]
	OpBuildList vm.OpCode = 0x60

	// OpBuildDict: creates a dict from N key-value pairs on the stack.
	// Operand: number of key-value pairs.
	//   Before: [..., key_1, val_1, key_2, val_2, ..., key_n, val_n]
	//   After:  [..., {key_1: val_1, ...}]
	OpBuildDict vm.OpCode = 0x61

	// OpBuildTuple: creates a tuple from N items on the stack.
	// Operand: number of items.
	//   Before: [..., item_1, ..., item_n]
	//   After:  [..., (item_1, ..., item_n)]
	OpBuildTuple vm.OpCode = 0x62

	// OpListAppend: appends TOS to a list at a given stack position.
	// Used in list comprehensions.
	// Operand: stack offset of the list being built.
	OpListAppend vm.OpCode = 0x63

	// OpDictSet: sets a key-value pair in a dict at a given stack position.
	// Used in dict comprehensions.
	// Operand: stack offset of the dict being built.
	OpDictSet vm.OpCode = 0x64

	// ── Subscript and Attribute Access ─────────────────────────────────

	// OpLoadSubscript: index or key lookup.
	//   lst[0], d["key"]
	//   Stack: [..., container, index] -> [..., container[index]]
	OpLoadSubscript vm.OpCode = 0x70

	// OpStoreSubscript: index or key assignment.
	//   lst[0] = x, d["key"] = x
	//   Stack: [..., value, container, index] -> [...]
	OpStoreSubscript vm.OpCode = 0x71

	// OpLoadAttr: attribute access.
	//   obj.name
	//   Operand: index into names for the attribute name.
	//   Stack: [..., obj] -> [..., obj.name]
	OpLoadAttr vm.OpCode = 0x72

	// OpStoreAttr: attribute assignment.
	//   obj.name = value
	//   Operand: index into names for the attribute name.
	//   Stack: [..., value, obj] -> [...]
	OpStoreAttr vm.OpCode = 0x73

	// OpLoadSlice: slice a sequence.
	//   lst[start:stop:step]
	//   Operand: number of slice arguments (2 or 3).
	//   Stack: [..., seq, start, stop] or [..., seq, start, stop, step]
	//   After:  [..., sliced_result]
	OpLoadSlice vm.OpCode = 0x74

	// ── Iteration ──────────────────────────────────────────────────────

	// OpGetIter: converts an iterable to an iterator.
	//   Stack: [..., iterable] -> [..., iterator]
	OpGetIter vm.OpCode = 0x80

	// OpForIter: advances the iterator and pushes the next value.
	// If the iterator is exhausted, jumps to the operand address.
	// Operand: jump target for loop exit.
	//   Stack (has next): [..., iterator] -> [..., iterator, next_value]
	//   Stack (exhausted): [..., iterator] -> [...] and jump
	OpForIter vm.OpCode = 0x81

	// OpUnpackSequence: unpacks TOS into N values.
	// Operand: number of values to unpack.
	//   Before: [..., sequence]
	//   After:  [..., item_n, ..., item_2, item_1]  (reversed for store order)
	OpUnpackSequence vm.OpCode = 0x82

	// ── Modules ────────────────────────────────────────────────────────

	// OpLoadModule: loads a module by name.
	// Operand: index into constants for the module path string.
	//   Stack: [...] -> [..., module_object]
	OpLoadModule vm.OpCode = 0x90

	// OpImportFrom: imports a symbol from the module on TOS.
	// Operand: index into names for the symbol name.
	//   Stack: [..., module] -> [..., module, symbol_value]
	OpImportFrom vm.OpCode = 0x91

	// ── Output ─────────────────────────────────────────────────────────

	// OpPrintValue: prints TOS.
	// Moved from 0x60 (vm.OpPrint) to avoid conflict with OpBuildList.
	// The GenericVM uses handler registration, so the hex value is arbitrary
	// as long as it's unique and documented.
	//   Stack: [..., value] -> [...]
	OpPrintValue vm.OpCode = 0xA0
)

// OpcodeName maps each opcode to a human-readable name for debugging
// and disassembly. This is useful for trace output and error messages.
var OpcodeName = map[vm.OpCode]string{
	OpLoadConst:        "LOAD_CONST",
	OpPop:              "POP",
	OpDup:              "DUP",
	OpLoadNone:         "LOAD_NONE",
	OpLoadTrue:         "LOAD_TRUE",
	OpLoadFalse:        "LOAD_FALSE",
	OpStoreName:        "STORE_NAME",
	OpLoadName:         "LOAD_NAME",
	OpStoreLocal:       "STORE_LOCAL",
	OpLoadLocal:        "LOAD_LOCAL",
	OpStoreClosure:     "STORE_CLOSURE",
	OpLoadClosure:      "LOAD_CLOSURE",
	OpAdd:              "ADD",
	OpSub:              "SUB",
	OpMul:              "MUL",
	OpDiv:              "DIV",
	OpFloorDiv:         "FLOOR_DIV",
	OpMod:              "MOD",
	OpPower:            "POWER",
	OpNegate:           "NEGATE",
	OpBitAnd:           "BIT_AND",
	OpBitOr:            "BIT_OR",
	OpBitXor:           "BIT_XOR",
	OpBitNot:           "BIT_NOT",
	OpLShift:           "LEFT_SHIFT",
	OpRShift:           "RIGHT_SHIFT",
	OpCmpEq:            "CMP_EQ",
	OpCmpLt:            "CMP_LT",
	OpCmpGt:            "CMP_GT",
	OpCmpNe:            "CMP_NE",
	OpCmpLe:            "CMP_LE",
	OpCmpGe:            "CMP_GE",
	OpCmpIn:            "CMP_IN",
	OpCmpNotIn:         "CMP_NOT_IN",
	OpNot:              "NOT",
	OpJump:             "JUMP",
	OpJumpIfFalse:      "JUMP_IF_FALSE",
	OpJumpIfTrue:       "JUMP_IF_TRUE",
	OpJumpIfFalseOrPop: "JUMP_IF_FALSE_OR_POP",
	OpJumpIfTrueOrPop:  "JUMP_IF_TRUE_OR_POP",
	OpBreak:            "BREAK",
	OpContinue:         "CONTINUE",
	OpMakeFunction:     "MAKE_FUNCTION",
	OpCallFunction:     "CALL_FUNCTION",
	OpCallFunctionKW:   "CALL_FUNCTION_KW",
	OpReturnValue:      "RETURN_VALUE",
	OpBuildList:        "BUILD_LIST",
	OpBuildDict:        "BUILD_DICT",
	OpBuildTuple:       "BUILD_TUPLE",
	OpListAppend:       "LIST_APPEND",
	OpDictSet:          "DICT_SET",
	OpLoadSubscript:    "LOAD_SUBSCRIPT",
	OpStoreSubscript:   "STORE_SUBSCRIPT",
	OpLoadAttr:         "LOAD_ATTR",
	OpStoreAttr:        "STORE_ATTR",
	OpLoadSlice:        "LOAD_SLICE",
	OpGetIter:          "GET_ITER",
	OpForIter:          "FOR_ITER",
	OpUnpackSequence:   "UNPACK_SEQUENCE",
	OpLoadModule:       "LOAD_MODULE",
	OpImportFrom:       "IMPORT_FROM",
	OpPrintValue:       "PRINT_VALUE",
	OpHalt:             "HALT",
}
