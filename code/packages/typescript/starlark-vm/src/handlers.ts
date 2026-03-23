/**
 * Starlark VM Opcode Handlers -- The execution semantics for Starlark bytecode.
 *
 * ==========================================================================
 * Chapter 1: What Opcode Handlers Do
 * ==========================================================================
 *
 * Each handler is a function that implements one Starlark bytecode instruction.
 * The GenericVM's eval loop calls the handler whenever it encounters the
 * corresponding opcode. The handler mutates the VM state (stack, PC, variables)
 * and optionally returns output text.
 *
 * All handlers follow the same signature (the {@link OpcodeHandler} type):
 *
 *     (vm: GenericVM, instr: Instruction, code: CodeObject) => string | null
 *
 * - ``vm`` -- The VM instance. Use ``vm.push()``, ``vm.pop()``, ``vm.advancePc()``.
 * - ``instr`` -- The instruction being executed (opcode + optional operand).
 * - ``code`` -- The CodeObject being run (for constant/name pool access).
 *
 * Returns a string if the handler produces output (e.g., PRINT), else null.
 *
 * ==========================================================================
 * Chapter 2: Starlark Type Semantics
 * ==========================================================================
 *
 * Starlark has a small, well-defined type system. Each handler must respect
 * the type rules:
 *
 * - **int + int -> int**, **float + float -> float**, **int + float -> float**
 * - **str + str -> str** (concatenation), **str * int -> str** (repetition)
 * - **list + list -> list** (concatenation), **list * int -> list** (repetition)
 * - Division always produces **float** (even ``4 / 2 -> 2.0``)
 * - Floor division ``//`` produces **int** (for int operands)
 * - Truthiness: ``0``, ``0.0``, ``""``, ``[]``, ``{}``, ``()``, ``null``, ``false`` are falsy
 *
 * ==========================================================================
 * Chapter 3: The Iterator Protocol
 * ==========================================================================
 *
 * Starlark's for-loops use an iterator protocol:
 *
 * 1. ``GET_ITER`` -- Convert an iterable to a StarlarkIterator.
 * 2. ``FOR_ITER`` -- Get the next value, or jump to end if exhausted.
 *
 * @module
 */

import type {
  CodeObject,
  Instruction,
  VMValue,
} from "@coding-adventures/virtual-machine";
import {
  GenericVM,
  VMTypeError,
  MaxRecursionError,
} from "@coding-adventures/virtual-machine";
import {
  VMError,
  DivisionByZeroError,
  InvalidOperandError,
  StackUnderflowError,
  UndefinedNameError,
} from "@coding-adventures/virtual-machine";

import {
  StarlarkFunction,
  StarlarkIterator,
  Op,
  isTruthy,
  starlarkRepr,
} from "./types.js";

// =========================================================================
// Numeric type check helper
// =========================================================================

/**
 * Check if a value is numeric (number, but not boolean).
 *
 * In JavaScript, ``typeof true === "boolean"`` and ``typeof 42 === "number"``,
 * so we need to explicitly exclude booleans. This mirrors the Python
 * reference implementation's ``_is_numeric()`` check which excludes ``bool``
 * (since Python's ``bool`` is a subclass of ``int``).
 */
function isNumeric(value: VMValue): value is number {
  return typeof value === "number";
}

// =========================================================================
// Stack Handlers (0x01-0x06)
// =========================================================================

/**
 * LOAD_CONST -- Push a constant from the pool.
 *
 * This is the most common instruction. Every literal value (42, "hello",
 * True) in the source code becomes a LOAD_CONST in the bytecode.
 * The operand is an index into the CodeObject's constants array.
 */
export function handleLoadConst(
  vm: GenericVM,
  instr: Instruction,
  code: CodeObject,
): string | null {
  const index = instr.operand as number;
  if (typeof index !== "number" || index < 0 || index >= code.constants.length) {
    throw new InvalidOperandError(
      `LOAD_CONST operand ${index} out of range (pool has ${code.constants.length} entries)`,
    );
  }
  vm.push(code.constants[index]);
  vm.advancePc();
  return null;
}

/**
 * POP -- Discard the top of stack.
 *
 * Used when an expression's result isn't needed (e.g., a function call
 * used only for its side effects).
 */
export function handlePop(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  vm.pop();
  vm.advancePc();
  return null;
}

/**
 * DUP -- Duplicate the top of stack.
 *
 * Pushes a copy of the top value without removing it. Used when the same
 * value is needed in two places (e.g., ``x = y = 42``).
 */
export function handleDup(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  vm.push(vm.peek());
  vm.advancePc();
  return null;
}

/**
 * LOAD_NONE -- Push null (Starlark's None).
 */
export function handleLoadNone(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  vm.push(null);
  vm.advancePc();
  return null;
}

/**
 * LOAD_TRUE -- Push true (Starlark's True).
 */
export function handleLoadTrue(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  vm.push(true);
  vm.advancePc();
  return null;
}

/**
 * LOAD_FALSE -- Push false (Starlark's False).
 */
export function handleLoadFalse(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  vm.push(false);
  vm.advancePc();
  return null;
}

// =========================================================================
// Variable Handlers (0x10-0x15)
// =========================================================================

/**
 * STORE_NAME -- Pop and store in a named variable.
 *
 * The operand indexes into the CodeObject's names array to get the
 * variable name. The value on top of the stack is stored under that name
 * in the VM's variables dictionary.
 */
export function handleStoreName(
  vm: GenericVM,
  instr: Instruction,
  code: CodeObject,
): string | null {
  if (vm.isFrozen()) {
    throw new VMError("Cannot modify variables -- module is frozen");
  }
  const index = instr.operand as number;
  if (typeof index !== "number" || index < 0 || index >= code.names.length) {
    throw new InvalidOperandError(
      `STORE_NAME operand ${index} out of range (names pool has ${code.names.length} entries)`,
    );
  }
  const name = code.names[index] as string;
  const value = vm.pop();
  vm.variables[name] = value;
  vm.advancePc();
  return null;
}

/**
 * LOAD_NAME -- Push the value of a named variable.
 *
 * Looks up the name in variables first, then in builtins. This mirrors
 * Python's LEGB rule (simplified): local -> global -> builtin.
 */
export function handleLoadName(
  vm: GenericVM,
  instr: Instruction,
  code: CodeObject,
): string | null {
  const index = instr.operand as number;
  if (typeof index !== "number" || index < 0 || index >= code.names.length) {
    throw new InvalidOperandError(
      `LOAD_NAME operand ${index} out of range (names pool has ${code.names.length} entries)`,
    );
  }
  const name = code.names[index] as string;

  // Check variables first, then builtins
  if (name in vm.variables) {
    vm.push(vm.variables[name]);
  } else {
    const builtin = vm.getBuiltin(name);
    if (builtin !== undefined) {
      vm.push(builtin as unknown as VMValue);
    } else {
      throw new UndefinedNameError(`Undefined variable: '${name}'`);
    }
  }
  vm.advancePc();
  return null;
}

/**
 * STORE_LOCAL -- Pop and store in a local variable slot.
 *
 * Local variables are stored by index (slot number) rather than by name.
 * This is faster because array indexing is O(1) while hash lookups have
 * overhead. Real VMs like the JVM use the same optimization.
 */
export function handleStoreLocal(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const index = instr.operand as number;
  while (vm.locals.length <= index) {
    vm.locals.push(null);
  }
  vm.locals[index] = vm.pop();
  vm.advancePc();
  return null;
}

/**
 * LOAD_LOCAL -- Push a value from a local variable slot.
 */
export function handleLoadLocal(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const index = instr.operand as number;
  if (index >= vm.locals.length) {
    throw new UndefinedNameError(`Local variable slot ${index} not yet assigned`);
  }
  vm.push(vm.locals[index]);
  vm.advancePc();
  return null;
}

/**
 * STORE_CLOSURE -- Store in a closure cell (delegates to STORE_LOCAL).
 *
 * In a full implementation, closures would use shared "cell" objects that
 * persist across function boundaries. For now, we reuse the locals mechanism.
 */
export function handleStoreClosure(
  vm: GenericVM,
  instr: Instruction,
  code: CodeObject,
): string | null {
  return handleStoreLocal(vm, instr, code);
}

/**
 * LOAD_CLOSURE -- Load from a closure cell (delegates to LOAD_LOCAL).
 */
export function handleLoadClosure(
  vm: GenericVM,
  instr: Instruction,
  code: CodeObject,
): string | null {
  return handleLoadLocal(vm, instr, code);
}

// =========================================================================
// Arithmetic Handlers (0x20-0x2D)
// =========================================================================

/**
 * ADD -- Pop two values, push a + b.
 *
 * Supports multiple type combinations:
 * - int + int -> int
 * - float + float -> float
 * - int + float -> float (type promotion)
 * - str + str -> str (concatenation: "hello" + " world" -> "hello world")
 * - list + list -> list (concatenation: [1] + [2] -> [1, 2])
 */
export function handleAdd(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();

  if (typeof a === "string" && typeof b === "string") {
    vm.push(a + b);
  } else if (Array.isArray(a) && Array.isArray(b)) {
    vm.push([...a, ...b]);
  } else if (isNumeric(a) && isNumeric(b)) {
    vm.push(a + b);
  } else {
    throw new VMTypeError(
      `Cannot add ${typeof a} and ${typeof b}`,
    );
  }
  vm.advancePc();
  return null;
}

/**
 * SUB -- Pop two values, push a - b.
 *
 * Only works on numeric types. Starlark does not allow subtracting strings
 * or lists.
 */
export function handleSub(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (!isNumeric(a) || !isNumeric(b)) {
    throw new VMTypeError(
      `Cannot subtract ${typeof b} from ${typeof a}`,
    );
  }
  vm.push(a - b);
  vm.advancePc();
  return null;
}

/**
 * MUL -- Pop two values, push a * b.
 *
 * Supports str * int (repetition), list * int (repetition), and numeric
 * multiplication.
 */
export function handleMul(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();

  if (typeof a === "string" && typeof b === "number") {
    vm.push(a.repeat(Math.max(0, b)));
  } else if (typeof a === "number" && typeof b === "string") {
    vm.push(b.repeat(Math.max(0, a)));
  } else if (Array.isArray(a) && typeof b === "number") {
    const result: VMValue[] = [];
    for (let i = 0; i < b; i++) result.push(...a);
    vm.push(result);
  } else if (typeof a === "number" && Array.isArray(b)) {
    const result: VMValue[] = [];
    for (let i = 0; i < a; i++) result.push(...b);
    vm.push(result);
  } else if (isNumeric(a) && isNumeric(b)) {
    vm.push(a * b);
  } else {
    throw new VMTypeError(
      `Cannot multiply ${typeof a} and ${typeof b}`,
    );
  }
  vm.advancePc();
  return null;
}

/**
 * DIV -- Pop two values, push a / b (always float division).
 *
 * In Starlark (like Python 3), ``/`` always produces a float:
 * ``4 / 2`` -> ``2.0``, not ``2``.
 */
export function handleDiv(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (!isNumeric(a) || !isNumeric(b)) {
    throw new VMTypeError(
      `Cannot divide ${typeof a} by ${typeof b}`,
    );
  }
  if (b === 0) {
    throw new DivisionByZeroError("Division by zero");
  }
  vm.push(a / b); // JS division is always float
  vm.advancePc();
  return null;
}

/**
 * FLOOR_DIV -- Pop two values, push a // b (integer division).
 *
 * Uses ``Math.floor()`` to round toward negative infinity, matching
 * Python's ``//`` operator semantics.
 */
export function handleFloorDiv(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (!isNumeric(a) || !isNumeric(b)) {
    throw new VMTypeError(
      `Cannot floor-divide ${typeof a} by ${typeof b}`,
    );
  }
  if (b === 0) {
    throw new DivisionByZeroError("Floor division by zero");
  }
  vm.push(Math.floor(a / b));
  vm.advancePc();
  return null;
}

/**
 * MOD -- Pop two values, push a % b.
 *
 * For strings, supports Python-style formatting: ``"Hello, %s" % name``.
 * For numbers, computes the modulo operation.
 */
export function handleMod(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (typeof a === "string") {
    // String formatting: "Hello, %s" % name
    const args = Array.isArray(b) ? b : [b];
    let result = a;
    let idx = 0;
    result = result.replace(/%[sd]/g, () => {
      return idx < args.length ? String(args[idx++]) : "";
    });
    vm.push(result);
  } else if (isNumeric(a) && isNumeric(b)) {
    if (b === 0) {
      throw new DivisionByZeroError("Modulo by zero");
    }
    // Use Python-style modulo (result has same sign as divisor)
    vm.push(((a % b) + b) % b);
  } else {
    throw new VMTypeError(
      `Cannot compute ${typeof a} % ${typeof b}`,
    );
  }
  vm.advancePc();
  return null;
}

/**
 * POWER -- Pop two values, push a ** b.
 */
export function handlePower(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (!isNumeric(a) || !isNumeric(b)) {
    throw new VMTypeError(
      `Cannot compute ${typeof a} ** ${typeof b}`,
    );
  }
  vm.push(Math.pow(a, b));
  vm.advancePc();
  return null;
}

/**
 * NEGATE -- Pop one value, push -a.
 */
export function handleNegate(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const a = vm.pop();
  if (!isNumeric(a)) {
    throw new VMTypeError(`Cannot negate ${typeof a}`);
  }
  vm.push(-a);
  vm.advancePc();
  return null;
}

/**
 * BIT_AND -- Pop two values, push a & b.
 *
 * Bitwise AND only works on integers. This mirrors Python's behavior
 * where ``3.0 & 1`` raises a TypeError.
 */
export function handleBitAnd(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (typeof a !== "number" || typeof b !== "number" || !Number.isInteger(a) || !Number.isInteger(b)) {
    throw new VMTypeError(
      `Cannot bitwise AND ${typeof a} and ${typeof b}`,
    );
  }
  vm.push(a & b);
  vm.advancePc();
  return null;
}

/**
 * BIT_OR -- Pop two values, push a | b.
 */
export function handleBitOr(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (typeof a !== "number" || typeof b !== "number" || !Number.isInteger(a) || !Number.isInteger(b)) {
    throw new VMTypeError(
      `Cannot bitwise OR ${typeof a} and ${typeof b}`,
    );
  }
  vm.push(a | b);
  vm.advancePc();
  return null;
}

/**
 * BIT_XOR -- Pop two values, push a ^ b.
 */
export function handleBitXor(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (typeof a !== "number" || typeof b !== "number" || !Number.isInteger(a) || !Number.isInteger(b)) {
    throw new VMTypeError(
      `Cannot bitwise XOR ${typeof a} and ${typeof b}`,
    );
  }
  vm.push(a ^ b);
  vm.advancePc();
  return null;
}

/**
 * BIT_NOT -- Pop one value, push ~a.
 */
export function handleBitNot(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const a = vm.pop();
  if (typeof a !== "number" || !Number.isInteger(a)) {
    throw new VMTypeError(`Cannot bitwise NOT ${typeof a}`);
  }
  vm.push(~a);
  vm.advancePc();
  return null;
}

/**
 * LSHIFT -- Pop two values, push a << b.
 */
export function handleLshift(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (typeof a !== "number" || typeof b !== "number" || !Number.isInteger(a) || !Number.isInteger(b)) {
    throw new VMTypeError(
      `Cannot left-shift ${typeof a} by ${typeof b}`,
    );
  }
  vm.push(a << b);
  vm.advancePc();
  return null;
}

/**
 * RSHIFT -- Pop two values, push a >> b.
 */
export function handleRshift(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  if (typeof a !== "number" || typeof b !== "number" || !Number.isInteger(a) || !Number.isInteger(b)) {
    throw new VMTypeError(
      `Cannot right-shift ${typeof a} by ${typeof b}`,
    );
  }
  vm.push(a >> b);
  vm.advancePc();
  return null;
}

// =========================================================================
// Comparison Handlers (0x30-0x37)
// =========================================================================

/**
 * CMP_EQ -- Pop two values, push a == b.
 *
 * Uses JavaScript's ``===`` for primitive comparison. For arrays and objects,
 * a deep comparison would be needed in a full implementation, but for now
 * we use reference equality (matching the Python reference impl behavior
 * for most types).
 */
export function handleCmpEq(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  vm.push(deepEqual(a, b));
  vm.advancePc();
  return null;
}

/** CMP_NE -- Pop two values, push a != b. */
export function handleCmpNe(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  vm.push(!deepEqual(a, b));
  vm.advancePc();
  return null;
}

/** CMP_LT -- Pop two values, push a < b. */
export function handleCmpLt(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  vm.push((a as number) < (b as number));
  vm.advancePc();
  return null;
}

/** CMP_GT -- Pop two values, push a > b. */
export function handleCmpGt(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  vm.push((a as number) > (b as number));
  vm.advancePc();
  return null;
}

/** CMP_LE -- Pop two values, push a <= b. */
export function handleCmpLe(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  vm.push((a as number) <= (b as number));
  vm.advancePc();
  return null;
}

/** CMP_GE -- Pop two values, push a >= b. */
export function handleCmpGe(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  vm.push((a as number) >= (b as number));
  vm.advancePc();
  return null;
}

/**
 * CMP_IN -- Pop two values, push a in b.
 *
 * Supports:
 * - element in list (linear scan)
 * - substring in string
 * - key in dict
 */
export function handleCmpIn(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  vm.push(containsValue(b, a));
  vm.advancePc();
  return null;
}

/** CMP_NOT_IN -- Pop two values, push a not in b. */
export function handleCmpNotIn(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const b = vm.pop();
  const a = vm.pop();
  vm.push(!containsValue(b, a));
  vm.advancePc();
  return null;
}

// =========================================================================
// Boolean Handler (0x38)
// =========================================================================

/**
 * NOT -- Pop one value, push logical not.
 *
 * Uses Starlark truthiness rules (see {@link isTruthy}).
 */
export function handleNot(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const a = vm.pop();
  vm.push(!isTruthy(a));
  vm.advancePc();
  return null;
}

// =========================================================================
// Control Flow Handlers (0x40-0x44)
// =========================================================================

/**
 * JUMP -- Unconditional jump to target.
 *
 * The operand is the instruction index to jump to. This is used for
 * ``while`` loops (jump back to the condition), ``if/else`` (skip the
 * else branch), and other control flow constructs.
 */
export function handleJump(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const target = instr.operand as number;
  vm.jumpTo(target);
  return null;
}

/**
 * JUMP_IF_FALSE -- Pop value, jump if falsy.
 *
 * Used for ``if`` conditions and ``while`` loop conditions. If the value
 * is falsy, execution skips to the target (the else branch or loop end).
 */
export function handleJumpIfFalse(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const target = instr.operand as number;
  const value = vm.pop();
  if (!isTruthy(value)) {
    vm.jumpTo(target);
  } else {
    vm.advancePc();
  }
  return null;
}

/**
 * JUMP_IF_TRUE -- Pop value, jump if truthy.
 */
export function handleJumpIfTrue(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const target = instr.operand as number;
  const value = vm.pop();
  if (isTruthy(value)) {
    vm.jumpTo(target);
  } else {
    vm.advancePc();
  }
  return null;
}

/**
 * JUMP_IF_FALSE_OR_POP -- Short-circuit AND.
 *
 * Used to implement the ``and`` operator:
 * - If top is falsy: keep it on stack and jump (short-circuit)
 * - If top is truthy: pop it and fall through (evaluate next operand)
 *
 * This is how ``a and b`` works: if ``a`` is falsy, the result is ``a``
 * (don't even evaluate ``b``). If ``a`` is truthy, the result is ``b``.
 */
export function handleJumpIfFalseOrPop(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const target = instr.operand as number;
  const value = vm.peek();
  if (!isTruthy(value)) {
    vm.jumpTo(target); // Keep falsy value on stack
  } else {
    vm.pop(); // Discard truthy value, evaluate next operand
    vm.advancePc();
  }
  return null;
}

/**
 * JUMP_IF_TRUE_OR_POP -- Short-circuit OR.
 *
 * Used to implement the ``or`` operator:
 * - If top is truthy: keep it on stack and jump (short-circuit)
 * - If top is falsy: pop it and fall through (evaluate next operand)
 */
export function handleJumpIfTrueOrPop(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const target = instr.operand as number;
  const value = vm.peek();
  if (isTruthy(value)) {
    vm.jumpTo(target); // Keep truthy value on stack
  } else {
    vm.pop(); // Discard falsy value, evaluate next operand
    vm.advancePc();
  }
  return null;
}

// =========================================================================
// Function Handlers (0x50-0x53)
// =========================================================================

/**
 * MAKE_FUNCTION -- Create a function object from a CodeObject.
 *
 * Stack layout (top to bottom):
 * 1. CodeObject -- the function body's compiled bytecode
 * 2. param_names tuple -- if flag bit 3 (0x08) is set
 *
 * Flags:
 * - 0x01 -- has default values
 * - 0x02 -- has *args
 * - 0x04 -- has **kwargs
 * - 0x08 -- has param_names tuple on stack
 */
export function handleMakeFunction(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const flags = (instr.operand as number) || 0;

  // Pop the param_names array if present (flag bit 3).
  // When flag 0x08 is set, the stack layout (top to bottom) is:
  //   [param_names, funcCode, ...]
  // So we pop param_names first, then the CodeObject.
  let paramNames: string[] = [];
  if (flags & 0x08) {
    const names = vm.pop();
    if (Array.isArray(names)) {
      paramNames = names as string[];
    }
  }

  // Pop the CodeObject
  const funcCode = vm.pop() as CodeObject;

  const defaults: VMValue[] = [];

  const func = new StarlarkFunction(
    funcCode,
    defaults,
    "<function>",
    paramNames.length > 0
      ? paramNames.length
      : funcCode.names
        ? funcCode.names.length
        : 0,
    paramNames,
  );
  vm.push(func as unknown as VMValue);
  vm.advancePc();
  return null;
}

/**
 * CALL_FUNCTION -- Call a function with N positional arguments.
 *
 * Stack layout before call: [func, arg1, arg2, ..., argN]
 * Operand: N (number of arguments)
 *
 * For user-defined StarlarkFunctions, this creates a new execution context
 * (saving the current state), runs the function's bytecode, then restores
 * the caller's state. This is how function calls work in all stack-based VMs.
 *
 * For builtins, it simply invokes the TypeScript implementation.
 */
export function handleCallFunction(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const argc = (instr.operand as number) || 0;

  // Pop arguments (in reverse order)
  const args: VMValue[] = [];
  for (let i = 0; i < argc; i++) {
    args.unshift(vm.pop());
  }

  // Pop the callable
  const func = vm.pop();

  if (func instanceof StarlarkFunction) {
    executeFunction(vm, func, args);
  } else if (
    typeof func === "object" &&
    func !== null &&
    "implementation" in func
  ) {
    // Built-in function
    const builtin = func as { implementation: (...args: VMValue[]) => VMValue };
    const result = builtin.implementation(...args);
    vm.push(result);
    vm.advancePc();
  } else {
    throw new VMTypeError(`'${typeof func}' object is not callable`);
  }

  return null;
}

/**
 * Execute a StarlarkFunction by running its CodeObject.
 *
 * This creates a mini-execution context: save the current state, run
 * the function's bytecode, then restore the caller's state.
 *
 * This is the "sub-execution" approach -- simpler than a full coroutine
 * or continuation model. The trade-off is that deeply nested function
 * calls consume host stack frames, which is why we have the recursion limit.
 */
function executeFunction(
  vm: GenericVM,
  func: StarlarkFunction,
  args: VMValue[],
): void {
  // Save current execution state
  const savedPc = vm.pc;
  const savedHalted = vm.halted;
  const savedVars = { ...vm.variables };
  const savedLocals = [...vm.locals];

  // Push a call frame for recursion tracking
  vm.pushFrame({
    returnAddress: vm.pc + 1,
    savedVariables: savedVars,
    savedLocals: savedLocals,
  });

  // Set up function context
  vm.locals = [...args];
  vm.pc = 0;
  vm.halted = false;

  // Execute the function's code
  const funcCode = func.code;
  let returnValue: VMValue = null;

  while (!vm.halted && vm.pc < funcCode.instructions.length) {
    const instruction = funcCode.instructions[vm.pc];

    // Check for RETURN -- special handling
    if (instruction.opcode === Op.RETURN) {
      returnValue = vm.stack.length > 0 ? vm.pop() : null;
      break;
    } else if (instruction.opcode === Op.HALT) {
      returnValue = null;
      break;
    } else {
      // Use the VM's step mechanism -- but we need to call the handler directly
      vm.step(funcCode);
    }
  }

  // Restore caller's state
  vm.pc = savedPc;
  vm.halted = savedHalted;
  vm.variables = savedVars;
  vm.locals = savedLocals;

  // Pop the call frame
  if (vm.callStack.length > 0) {
    vm.popFrame();
  }

  // Push return value
  vm.push(returnValue);
  vm.advancePc();
}

/**
 * CALL_FUNCTION_KW -- Call a function with keyword arguments.
 *
 * Stack layout:
 * 1. The callable (function or builtin)
 * 2. All argument values (positional first, then keyword values)
 * 3. An array of keyword names on the very top
 *
 * The operand is the total argument count (positional + keyword).
 *
 * Example: ``f(a, x=1, y=2)`` produces:
 *     [func, a_value, 1, 2, ["x", "y"]]
 */
export function handleCallFunctionKw(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const argc = (instr.operand as number) || 0;

  // Step 1: Pop the keyword names array from the top
  const kwNames = vm.pop() as string[];
  const kwCount = Array.isArray(kwNames) ? kwNames.length : 0;

  // Step 2: Pop all argument values
  const allValues: VMValue[] = [];
  for (let i = 0; i < argc; i++) {
    allValues.unshift(vm.pop());
  }

  // Step 3: Split into positional and keyword arguments
  const posCount = argc - kwCount;
  const posArgs = allValues.slice(0, posCount);
  const kwValues = allValues.slice(posCount);
  const kwargs: Record<string, VMValue> = {};
  for (let i = 0; i < kwCount; i++) {
    kwargs[kwNames[i]] = kwValues[i];
  }

  // Step 4: Pop the callable
  const func = vm.pop();

  if (func instanceof StarlarkFunction) {
    const paramNames = [...func.paramNames];

    // Start with positional arguments
    const finalArgs: VMValue[] = [...posArgs];

    // Pad to the full parameter count
    while (finalArgs.length < func.paramCount) {
      finalArgs.push(null);
    }

    // Fill in keyword arguments by matching names to positions
    for (const [kwName, kwValue] of Object.entries(kwargs)) {
      const idx = paramNames.indexOf(kwName);
      if (idx >= 0) {
        if (idx < posCount) {
          throw new VMTypeError(
            `got multiple values for argument '${kwName}'`,
          );
        }
        finalArgs[idx] = kwValue;
      } else {
        throw new VMTypeError(
          `unexpected keyword argument '${kwName}'`,
        );
      }
    }

    executeFunction(vm, func, finalArgs);
  } else if (
    typeof func === "object" &&
    func !== null &&
    "implementation" in func
  ) {
    // Built-in: pass positional args only
    const builtin = func as { implementation: (...args: VMValue[]) => VMValue };
    const result = builtin.implementation(...posArgs);
    vm.push(result);
    vm.advancePc();
  } else {
    throw new VMTypeError(`'${typeof func}' object is not callable`);
  }

  return null;
}

/**
 * RETURN -- Return from a function.
 *
 * At the top level (not inside a function), RETURN halts the VM.
 * Inside a function, RETURN is handled specially by ``executeFunction()``.
 */
export function handleReturn(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  vm.halted = true;
  return null;
}

// =========================================================================
// Collection Handlers (0x60-0x64)
// =========================================================================

/**
 * BUILD_LIST -- Create a list from N stack items.
 *
 * Pops N items from the stack and creates a new list. Items are popped
 * in reverse order and reversed to maintain source order.
 */
export function handleBuildList(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const count = (instr.operand as number) || 0;
  const items: VMValue[] = [];
  for (let i = 0; i < count; i++) {
    items.unshift(vm.pop());
  }
  vm.push(items);
  vm.advancePc();
  return null;
}

/**
 * BUILD_DICT -- Create a dict from N key-value pairs on the stack.
 *
 * Stack contains alternating keys and values: key1, val1, key2, val2, ...
 */
export function handleBuildDict(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const count = (instr.operand as number) || 0;
  const pairs: [string, VMValue][] = [];
  for (let i = 0; i < count; i++) {
    const value = vm.pop();
    const key = vm.pop();
    pairs.unshift([key as string, value]);
  }
  const dict: Record<string, VMValue> = {};
  for (const [k, v] of pairs) {
    dict[String(k)] = v;
  }
  vm.push(dict as unknown as VMValue);
  vm.advancePc();
  return null;
}

/**
 * BUILD_TUPLE -- Create a tuple from N stack items.
 *
 * In JavaScript we represent tuples as frozen arrays. The semantics are
 * the same as BUILD_LIST but the result is immutable.
 */
export function handleBuildTuple(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const count = (instr.operand as number) || 0;
  const items: VMValue[] = [];
  for (let i = 0; i < count; i++) {
    items.unshift(vm.pop());
  }
  // We use a regular array for tuples -- immutability is enforced by Starlark semantics
  vm.push(items);
  vm.advancePc();
  return null;
}

/**
 * LIST_APPEND -- Append value to list (for comprehensions).
 *
 * Stack: ... list value -> ... list
 * The list stays on the stack for the next iteration.
 */
export function handleListAppend(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const value = vm.pop();
  const list = vm.peek();
  if (!Array.isArray(list)) {
    throw new VMTypeError(`LIST_APPEND requires a list, got ${typeof list}`);
  }
  list.push(value);
  vm.advancePc();
  return null;
}

/**
 * DICT_SET -- Set dict entry (for comprehensions).
 *
 * Stack: ... dict key value -> ... dict
 */
export function handleDictSet(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const value = vm.pop();
  const key = vm.pop();
  const dict = vm.peek();
  if (typeof dict !== "object" || dict === null || Array.isArray(dict)) {
    throw new VMTypeError(`DICT_SET requires a dict, got ${typeof dict}`);
  }
  (dict as Record<string, VMValue>)[String(key)] = value;
  vm.advancePc();
  return null;
}

// =========================================================================
// Subscript & Attribute Handlers (0x70-0x74)
// =========================================================================

/**
 * LOAD_SUBSCRIPT -- obj[key].
 *
 * Supports list indexing, dict key lookup, and string indexing.
 */
export function handleLoadSubscript(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const key = vm.pop();
  const obj = vm.pop();

  if (Array.isArray(obj)) {
    const index = key as number;
    // Support negative indexing (Python-style)
    const actualIndex = index < 0 ? obj.length + index : index;
    if (actualIndex < 0 || actualIndex >= obj.length) {
      throw new VMError(`Index ${key} out of range for list of length ${obj.length}`);
    }
    vm.push(obj[actualIndex]);
  } else if (typeof obj === "string") {
    const index = key as number;
    const actualIndex = index < 0 ? obj.length + index : index;
    if (actualIndex < 0 || actualIndex >= obj.length) {
      throw new VMError(`Index ${key} out of range for string of length ${obj.length}`);
    }
    vm.push(obj[actualIndex]);
  } else if (typeof obj === "object" && obj !== null) {
    const dict = obj as Record<string, VMValue>;
    const k = String(key);
    if (!(k in dict)) {
      throw new VMError(`Key '${k}' not found in dict`);
    }
    vm.push(dict[k]);
  } else {
    throw new VMError(`Cannot subscript ${typeof obj}`);
  }

  vm.advancePc();
  return null;
}

/**
 * STORE_SUBSCRIPT -- obj[key] = value.
 */
export function handleStoreSubscript(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  if (vm.isFrozen()) {
    throw new VMError("Cannot modify collections -- module is frozen");
  }
  const value = vm.pop();
  const key = vm.pop();
  const obj = vm.pop();

  if (Array.isArray(obj)) {
    const index = key as number;
    const actualIndex = index < 0 ? obj.length + index : index;
    obj[actualIndex] = value;
  } else if (typeof obj === "object" && obj !== null) {
    (obj as Record<string, VMValue>)[String(key)] = value;
  } else {
    throw new VMError(`Cannot store subscript on ${typeof obj}`);
  }

  vm.advancePc();
  return null;
}

/**
 * LOAD_ATTR -- obj.attr.
 *
 * In Starlark, only a few types have attributes:
 * - string methods (upper, lower, split, join, etc.)
 * - dict methods (keys, values, items, get, etc.)
 * - list methods (append, extend, insert, etc.)
 *
 * We implement the most common methods as bound functions.
 */
export function handleLoadAttr(
  vm: GenericVM,
  instr: Instruction,
  code: CodeObject,
): string | null {
  const index = instr.operand as number;
  const attrName = code.names[index] as string;
  const obj = vm.pop();

  const method = getStarlarkAttr(obj, attrName);
  if (method === undefined) {
    throw new VMError(`'${typeof obj}' has no attribute '${attrName}'`);
  }
  vm.push(method);
  vm.advancePc();
  return null;
}

/**
 * STORE_ATTR -- obj.attr = value.
 *
 * Starlark does not support attribute assignment.
 */
export function handleStoreAttr(
  _vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  throw new VMError("Starlark does not support attribute assignment");
}

/**
 * LOAD_SLICE -- obj[start:stop:step].
 *
 * Operand flags indicate which parts are present:
 * - bit 0: start present
 * - bit 1: stop present
 * - bit 2: step present
 */
export function handleLoadSlice(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const flags = (instr.operand as number) || 0;

  const step = flags & 0x04 ? (vm.pop() as number) : undefined;
  const stop = flags & 0x02 ? (vm.pop() as number) : undefined;
  const start = flags & 0x01 ? (vm.pop() as number) : undefined;

  const obj = vm.pop();

  if (Array.isArray(obj) || typeof obj === "string") {
    const result = sliceSequence(obj, start, stop, step);
    vm.push(result);
  } else {
    throw new VMError(`Cannot slice ${typeof obj}`);
  }

  vm.advancePc();
  return null;
}

// =========================================================================
// Iteration Handlers (0x80-0x82)
// =========================================================================

/**
 * GET_ITER -- Convert an iterable to a StarlarkIterator.
 *
 * Supports lists, strings, dicts (iterates over keys), and tuples.
 */
export function handleGetIter(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const iterable = vm.pop();

  if (Array.isArray(iterable)) {
    vm.push(new StarlarkIterator(iterable) as unknown as VMValue);
  } else if (typeof iterable === "string") {
    vm.push(new StarlarkIterator([...iterable]) as unknown as VMValue);
  } else if (typeof iterable === "object" && iterable !== null) {
    // Dict -- iterate over keys
    const keys = Object.keys(iterable as Record<string, unknown>);
    vm.push(new StarlarkIterator(keys) as unknown as VMValue);
  } else {
    throw new VMTypeError(`'${typeof iterable}' is not iterable`);
  }

  vm.advancePc();
  return null;
}

/**
 * FOR_ITER -- Get next value from iterator, or jump if exhausted.
 *
 * If the iterator has more values: push the next value and advance PC.
 * If exhausted: pop the iterator and jump to the target (end of loop).
 */
export function handleForIter(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const target = instr.operand as number;
  const iterator = vm.peek();

  if (iterator instanceof StarlarkIterator) {
    const result = iterator.next();
    if (result.done) {
      vm.pop(); // Pop the exhausted iterator
      vm.jumpTo(target);
    } else {
      vm.push(result.value as VMValue);
      vm.advancePc();
    }
  } else {
    throw new VMTypeError("FOR_ITER requires a StarlarkIterator");
  }

  return null;
}

/**
 * UNPACK_SEQUENCE -- Unpack N items from a sequence.
 *
 * Pops a sequence, pushes its elements in reverse order so they can
 * be stored in the correct order by subsequent STORE instructions.
 *
 * Example: ``a, b, c = [1, 2, 3]`` unpacks [1,2,3] and pushes 3, 2, 1
 * so that STORE a gets 1, STORE b gets 2, STORE c gets 3.
 */
export function handleUnpackSequence(
  vm: GenericVM,
  instr: Instruction,
  _code: CodeObject,
): string | null {
  const count = instr.operand as number;
  const seq = vm.pop();

  let items: VMValue[];
  if (Array.isArray(seq)) {
    items = seq;
  } else if (typeof seq === "string") {
    items = [...seq] as VMValue[];
  } else {
    throw new VMError(`Cannot unpack ${typeof seq}`);
  }

  if (items.length !== count) {
    throw new VMError(
      `Cannot unpack ${items.length} values into ${count} variables`,
    );
  }

  // Push in reverse so STORE operations work left-to-right
  for (let i = items.length - 1; i >= 0; i--) {
    vm.push(items[i]);
  }
  vm.advancePc();
  return null;
}

// =========================================================================
// Module Handlers (0x90-0x91)
// =========================================================================

/**
 * LOAD_MODULE -- Load a Starlark module (for load() statement).
 *
 * In a full implementation, this would find and execute the module file.
 * For now, we push a placeholder dict with the module name.
 */
export function handleLoadModule(
  vm: GenericVM,
  instr: Instruction,
  code: CodeObject,
): string | null {
  const index = instr.operand as number;
  const moduleName = code.names[index] as string;
  vm.push({ __name__: moduleName } as unknown as VMValue);
  vm.advancePc();
  return null;
}

/**
 * IMPORT_FROM -- Extract a symbol from a loaded module.
 */
export function handleImportFrom(
  vm: GenericVM,
  instr: Instruction,
  code: CodeObject,
): string | null {
  const index = instr.operand as number;
  const symbolName = code.names[index] as string;
  const module = vm.peek();

  if (
    typeof module === "object" &&
    module !== null &&
    symbolName in (module as Record<string, unknown>)
  ) {
    vm.push((module as Record<string, VMValue>)[symbolName]);
  } else {
    throw new VMError(`Cannot import '${symbolName}' from module`);
  }
  vm.advancePc();
  return null;
}

// =========================================================================
// I/O Handler (0xA0)
// =========================================================================

/**
 * PRINT -- Pop and print a value.
 *
 * The output is captured in the VM's output array rather than going to
 * stdout. This makes testing and debugging much easier.
 */
export function handlePrint(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  const value = vm.pop();
  const output = starlarkRepr(value);
  vm.output.push(output);
  vm.advancePc();
  return output;
}

// =========================================================================
// Halt Handler (0xFF)
// =========================================================================

/**
 * HALT -- Stop execution.
 *
 * Sets the VM's halted flag to true, which causes the eval loop to stop.
 */
export function handleHalt(
  vm: GenericVM,
  _instr: Instruction,
  _code: CodeObject,
): string | null {
  vm.halted = true;
  return null;
}

// =========================================================================
// Internal Helpers
// =========================================================================

/**
 * Deep equality comparison for Starlark values.
 *
 * Needed because JavaScript's ``===`` only compares references for arrays
 * and objects, but Starlark's ``==`` compares by value.
 */
function deepEqual(a: VMValue, b: VMValue): boolean {
  if (a === b) return true;
  if (a === null || b === null) return a === b;
  if (typeof a !== typeof b) return false;

  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!deepEqual(a[i], b[i])) return false;
    }
    return true;
  }

  if (
    typeof a === "object" &&
    typeof b === "object" &&
    !Array.isArray(a) &&
    !Array.isArray(b)
  ) {
    const aObj = a as Record<string, VMValue>;
    const bObj = b as Record<string, VMValue>;
    const aKeys = Object.keys(aObj);
    const bKeys = Object.keys(bObj);
    if (aKeys.length !== bKeys.length) return false;
    for (const key of aKeys) {
      if (!deepEqual(aObj[key], bObj[key])) return false;
    }
    return true;
  }

  return false;
}

/**
 * Check if a container contains a value (implements ``in`` operator).
 */
function containsValue(container: VMValue, element: VMValue): boolean {
  if (Array.isArray(container)) {
    return container.some((item) => deepEqual(item, element));
  }
  if (typeof container === "string" && typeof element === "string") {
    return container.includes(element);
  }
  if (typeof container === "object" && container !== null) {
    return String(element) in (container as Record<string, unknown>);
  }
  throw new VMTypeError(`'${typeof element}' in '${typeof container}' is not supported`);
}

/**
 * Slice a sequence (list or string) with Python-style start:stop:step.
 */
function sliceSequence(
  seq: VMValue[] | string,
  start?: number,
  stop?: number,
  step?: number,
): VMValue[] | string {
  const len = seq.length;
  const s = step ?? 1;

  if (s === 0) {
    throw new VMError("Slice step cannot be zero");
  }

  let actualStart: number;
  let actualStop: number;

  if (s > 0) {
    actualStart = start !== undefined ? normalizeIndex(start, len) : 0;
    actualStop = stop !== undefined ? normalizeIndex(stop, len) : len;
  } else {
    actualStart = start !== undefined ? normalizeIndex(start, len) : len - 1;
    actualStop = stop !== undefined ? normalizeIndex(stop, len) : -1;
  }

  const items: (VMValue | string)[] = [];
  if (s > 0) {
    for (let i = actualStart; i < actualStop; i += s) {
      items.push(typeof seq === "string" ? seq[i] : (seq as VMValue[])[i]);
    }
  } else {
    for (let i = actualStart; i > actualStop; i += s) {
      items.push(typeof seq === "string" ? seq[i] : (seq as VMValue[])[i]);
    }
  }

  return typeof seq === "string"
    ? items.join("")
    : (items as VMValue[]);
}

/**
 * Normalize a slice index to be within bounds.
 */
function normalizeIndex(index: number, length: number): number {
  if (index < 0) index += length;
  if (index < 0) return 0;
  if (index > length) return length;
  return index;
}

/**
 * Get a Starlark attribute (method) from an object.
 *
 * Implements the common string, list, and dict methods.
 */
function getStarlarkAttr(obj: VMValue, name: string): VMValue | undefined {
  if (typeof obj === "string") {
    return getStringAttr(obj, name);
  }
  if (Array.isArray(obj)) {
    return getListAttr(obj, name);
  }
  if (typeof obj === "object" && obj !== null && !Array.isArray(obj)) {
    return getDictAttr(obj as Record<string, VMValue>, name);
  }
  return undefined;
}

/**
 * String methods for Starlark.
 */
function getStringAttr(s: string, name: string): VMValue | undefined {
  const methods: Record<string, VMValue> = {
    upper: {
      name: "upper",
      implementation: () => s.toUpperCase(),
    } as unknown as VMValue,
    lower: {
      name: "lower",
      implementation: () => s.toLowerCase(),
    } as unknown as VMValue,
    strip: {
      name: "strip",
      implementation: () => s.trim(),
    } as unknown as VMValue,
    lstrip: {
      name: "lstrip",
      implementation: () => s.trimStart(),
    } as unknown as VMValue,
    rstrip: {
      name: "rstrip",
      implementation: () => s.trimEnd(),
    } as unknown as VMValue,
    startswith: {
      name: "startswith",
      implementation: (...args: VMValue[]) => s.startsWith(args[0] as string),
    } as unknown as VMValue,
    endswith: {
      name: "endswith",
      implementation: (...args: VMValue[]) => s.endsWith(args[0] as string),
    } as unknown as VMValue,
    split: {
      name: "split",
      implementation: (...args: VMValue[]) => {
        if (args.length === 0 || args[0] === null || args[0] === undefined) {
          return s.split(/\s+/).filter((x) => x.length > 0);
        }
        return s.split(args[0] as string);
      },
    } as unknown as VMValue,
    join: {
      name: "join",
      implementation: (...args: VMValue[]) => {
        const items = args[0] as VMValue[];
        return items.map(String).join(s);
      },
    } as unknown as VMValue,
    replace: {
      name: "replace",
      implementation: (...args: VMValue[]) =>
        s.split(args[0] as string).join(args[1] as string),
    } as unknown as VMValue,
    find: {
      name: "find",
      implementation: (...args: VMValue[]) => s.indexOf(args[0] as string),
    } as unknown as VMValue,
    count: {
      name: "count",
      implementation: (...args: VMValue[]) => {
        const sub = args[0] as string;
        if (sub.length === 0) return s.length + 1;
        let count = 0;
        let pos = 0;
        while ((pos = s.indexOf(sub, pos)) !== -1) {
          count++;
          pos += sub.length;
        }
        return count;
      },
    } as unknown as VMValue,
    format: {
      name: "format",
      implementation: (...args: VMValue[]) => {
        let result = s;
        let idx = 0;
        result = result.replace(/\{\}/g, () => String(args[idx++] ?? ""));
        return result;
      },
    } as unknown as VMValue,
    title: {
      name: "title",
      implementation: () =>
        s.replace(/\w\S*/g, (txt) =>
          txt.charAt(0).toUpperCase() + txt.substring(1).toLowerCase(),
        ),
    } as unknown as VMValue,
    capitalize: {
      name: "capitalize",
      implementation: () =>
        s.charAt(0).toUpperCase() + s.slice(1).toLowerCase(),
    } as unknown as VMValue,
    isdigit: {
      name: "isdigit",
      implementation: () => s.length > 0 && /^\d+$/.test(s),
    } as unknown as VMValue,
    isalpha: {
      name: "isalpha",
      implementation: () => s.length > 0 && /^[a-zA-Z]+$/.test(s),
    } as unknown as VMValue,
  };
  return methods[name];
}

/**
 * List methods for Starlark.
 */
function getListAttr(list: VMValue[], name: string): VMValue | undefined {
  const methods: Record<string, VMValue> = {
    append: {
      name: "append",
      implementation: (...args: VMValue[]) => {
        list.push(args[0]);
        return null;
      },
    } as unknown as VMValue,
    extend: {
      name: "extend",
      implementation: (...args: VMValue[]) => {
        const items = args[0] as VMValue[];
        list.push(...items);
        return null;
      },
    } as unknown as VMValue,
    insert: {
      name: "insert",
      implementation: (...args: VMValue[]) => {
        list.splice(args[0] as number, 0, args[1]);
        return null;
      },
    } as unknown as VMValue,
    pop: {
      name: "pop",
      implementation: (...args: VMValue[]) => {
        if (args.length === 0) return list.pop() ?? null;
        const idx = args[0] as number;
        const [removed] = list.splice(idx, 1);
        return removed ?? null;
      },
    } as unknown as VMValue,
    remove: {
      name: "remove",
      implementation: (...args: VMValue[]) => {
        const idx = list.indexOf(args[0]);
        if (idx === -1) throw new VMError(`${args[0]} not in list`);
        list.splice(idx, 1);
        return null;
      },
    } as unknown as VMValue,
    index: {
      name: "index",
      implementation: (...args: VMValue[]) => {
        const idx = list.indexOf(args[0]);
        if (idx === -1) throw new VMError(`${args[0]} not in list`);
        return idx;
      },
    } as unknown as VMValue,
    clear: {
      name: "clear",
      implementation: () => {
        list.length = 0;
        return null;
      },
    } as unknown as VMValue,
  };
  return methods[name];
}

/**
 * Dict methods for Starlark.
 */
function getDictAttr(
  dict: Record<string, VMValue>,
  name: string,
): VMValue | undefined {
  const methods: Record<string, VMValue> = {
    keys: {
      name: "keys",
      implementation: () => Object.keys(dict),
    } as unknown as VMValue,
    values: {
      name: "values",
      implementation: () => Object.values(dict),
    } as unknown as VMValue,
    items: {
      name: "items",
      implementation: () =>
        Object.entries(dict).map(([k, v]) => [k, v]),
    } as unknown as VMValue,
    get: {
      name: "get",
      implementation: (...args: VMValue[]) => {
        const key = String(args[0]);
        if (key in dict) return dict[key];
        return args.length > 1 ? args[1] : null;
      },
    } as unknown as VMValue,
    pop: {
      name: "pop",
      implementation: (...args: VMValue[]) => {
        const key = String(args[0]);
        if (key in dict) {
          const val = dict[key];
          delete dict[key];
          return val;
        }
        if (args.length > 1) return args[1];
        throw new VMError(`Key '${key}' not found in dict`);
      },
    } as unknown as VMValue,
    update: {
      name: "update",
      implementation: (...args: VMValue[]) => {
        const other = args[0] as Record<string, VMValue>;
        Object.assign(dict, other);
        return null;
      },
    } as unknown as VMValue,
    setdefault: {
      name: "setdefault",
      implementation: (...args: VMValue[]) => {
        const key = String(args[0]);
        if (key in dict) return dict[key];
        const defaultVal = args.length > 1 ? args[1] : null;
        dict[key] = defaultVal;
        return defaultVal;
      },
    } as unknown as VMValue,
  };
  return methods[name];
}
