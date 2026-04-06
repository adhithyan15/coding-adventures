/**
 * opcodes.ts — The Instruction Set Architecture (ISA) for the Register VM.
 *
 * This file defines every opcode the VM understands. The design mirrors
 * V8's Ignition bytecode interpreter: all operations work through a single
 * implicit accumulator register plus an explicit register file.
 *
 * ## Accumulator Model (V8 Ignition style)
 *
 * Unlike a stack machine (where operands live on a stack) or a pure
 * register machine (where every instruction names its destination
 * register), Ignition uses an *accumulator* as the implicit output
 * register for most operations:
 *
 *   - LOAD operations write INTO the accumulator.
 *   - Binary operations READ one operand from the accumulator and one
 *     from a named register, then write the RESULT back into the
 *     accumulator.
 *   - STORE operations copy the accumulator to a named register or
 *     memory location.
 *
 * This hybrid reduces instruction size: most instructions only need to
 * encode the "other" register, not both source and destination.
 *
 * ## Opcode categories (nibble-grouped for readability)
 *
 *   0x0_ — Accumulator loads (immediate / literal values)
 *   0x1_ — Register moves (accumulator ↔ register file)
 *   0x2_ — Variable access (globals, locals, context slots)
 *   0x3_ — Arithmetic & bitwise
 *   0x4_ — Comparisons & type tests
 *   0x5_ — Control flow (jumps)
 *   0x6_ — Function calls & generators
 *   0x7_ — Property access (named & keyed)
 *   0x8_ — Object/array/closure creation
 *   0x9_ — Iteration protocol
 *   0xA_ — Exception control
 *   0xB_ — Context / scope management
 *   0xF_ — VM meta-instructions (debugging, halting)
 */

/** The complete set of opcodes as a const enum (erased at emit time). */
export const enum Opcode {
  // ─────────────────────────────────────────────────────────────────────
  // 0x0_ — Accumulator loads
  //
  // These instructions write a value directly into the accumulator without
  // reading from any register. They are the cheapest possible instructions
  // because there is no register-file lookup cost.
  // ─────────────────────────────────────────────────────────────────────

  /** Load a value from the constant pool: acc = constants[idx] */
  LDA_CONSTANT = 0x00,

  /** Load the integer zero: acc = 0   (avoids a constant-pool entry) */
  LDA_ZERO = 0x01,

  /** Load a small integer (SMI) encoded directly in the operand: acc = smi */
  LDA_SMI = 0x02,

  /** Load undefined: acc = undefined */
  LDA_UNDEFINED = 0x03,

  /** Load null: acc = null */
  LDA_NULL = 0x04,

  /** Load true: acc = true */
  LDA_TRUE = 0x05,

  /** Load false: acc = false */
  LDA_FALSE = 0x06,

  // ─────────────────────────────────────────────────────────────────────
  // 0x1_ — Register moves
  //
  // Move data between the accumulator and named registers, or between two
  // registers. V8 uses these heavily for function prologues (moving
  // parameters from argument registers into named locals) and epilogues
  // (materialising the return value into the accumulator).
  // ─────────────────────────────────────────────────────────────────────

  /** Load accumulator from register: acc = registers[r] */
  LDAR = 0x10,

  /** Store accumulator to register: registers[r] = acc */
  STAR = 0x11,

  /** Move register to register: registers[dst] = registers[src] */
  MOV = 0x12,

  // ─────────────────────────────────────────────────────────────────────
  // 0x2_ — Variable access
  //
  // JavaScript's scoping rules are complex (lexical scoping + closures +
  // module namespaces). These instructions cover the four principal storage
  // locations that Ignition addresses:
  //
  //   1. Globals   — the single shared global object (window / globalThis)
  //   2. Locals    — fast-path: current frame's register file by slot index
  //   3. Context   — closure cells in an ancestor activation context
  //   4. Module    — module-level exports/imports
  //
  // "Context slots" model how JavaScript closures work: when a function
  // closes over a variable, that variable is *promoted* from a stack
  // register into a heap-allocated context object. The depth operand
  // walks up the context chain to find the right scope.
  // ─────────────────────────────────────────────────────────────────────

  /** acc = globals[names[nameIdx]] */
  LDA_GLOBAL = 0x20,

  /** globals[names[nameIdx]] = acc */
  STA_GLOBAL = 0x21,

  /** acc = registers[localIdx]  (same as LDAR but semantically "local var") */
  LDA_LOCAL = 0x22,

  /** registers[localIdx] = acc */
  STA_LOCAL = 0x23,

  /** acc = context[depth][slotIdx]  (walk up depth parent links) */
  LDA_CONTEXT_SLOT = 0x24,

  /** context[depth][slotIdx] = acc */
  STA_CONTEXT_SLOT = 0x25,

  /** acc = currentContext[slotIdx]  (shortcut: depth = 0) */
  LDA_CURRENT_CONTEXT_SLOT = 0x26,

  /** currentContext[slotIdx] = acc */
  STA_CURRENT_CONTEXT_SLOT = 0x27,

  // ─────────────────────────────────────────────────────────────────────
  // 0x3_ — Arithmetic & bitwise
  //
  // All binary instructions take ONE register operand (the right-hand
  // side) plus an optional feedback-slot index. The left-hand side is
  // always the accumulator. The result is written back to the accumulator.
  //
  // The feedback slot lets the VM record what types it sees at runtime,
  // enabling speculative optimisation (see feedback.ts for the state
  // machine that drives this).
  //
  // Truth table for ADD with type coercion (matching JS):
  //   number + number → number
  //   string + ?      → string  (toString coercion)
  //   ?      + string → string
  //   other           → NaN
  // ─────────────────────────────────────────────────────────────────────

  /** acc = acc + registers[r]  (with feedback) */
  ADD = 0x30,

  /** acc = acc - registers[r] */
  SUB = 0x31,

  /** acc = acc * registers[r] */
  MUL = 0x32,

  /** acc = acc / registers[r] */
  DIV = 0x33,

  /** acc = acc % registers[r] */
  MOD = 0x34,

  /** acc = acc ** registers[r] */
  POW = 0x35,

  /** acc = acc + smi  (SMI fast-path — avoids register lookup) */
  ADD_SMI = 0x36,

  /** acc = acc - smi */
  SUB_SMI = 0x37,

  /** acc = acc & registers[r] */
  BITWISE_AND = 0x38,

  /** acc = acc | registers[r] */
  BITWISE_OR = 0x39,

  /** acc = acc ^ registers[r] */
  BITWISE_XOR = 0x3a,

  /** acc = ~acc  (unary bitwise NOT) */
  BITWISE_NOT = 0x3b,

  /** acc = acc << registers[r] */
  SHIFT_LEFT = 0x3c,

  /** acc = acc >> registers[r]  (signed) */
  SHIFT_RIGHT = 0x3d,

  /** acc = acc >>> registers[r]  (unsigned) */
  SHIFT_RIGHT_LOGICAL = 0x3e,

  /** acc = -acc  (unary arithmetic negation) */
  NEGATE = 0x3f,

  // ─────────────────────────────────────────────────────────────────────
  // 0x4_ — Comparisons & type tests
  //
  // Each comparison writes a boolean to the accumulator. The second
  // operand comes from a named register (or is implicit for unary ops).
  //
  // JS has two equality operators:
  //   == / !=  perform type coercion (TEST_EQUAL / TEST_NOT_EQUAL)
  //   === / !== are strict (TEST_STRICT_EQUAL / TEST_STRICT_NOT_EQUAL)
  //
  // For simplicity this VM implements only reference/value equality
  // (no full JS type-coercion algorithm).
  // ─────────────────────────────────────────────────────────────────────

  /** acc = (acc == registers[r])   — abstract equality */
  TEST_EQUAL = 0x40,

  /** acc = (acc != registers[r]) */
  TEST_NOT_EQUAL = 0x41,

  /** acc = (acc === registers[r])  — strict equality */
  TEST_STRICT_EQUAL = 0x42,

  /** acc = (acc !== registers[r]) */
  TEST_STRICT_NOT_EQUAL = 0x43,

  /** acc = (acc < registers[r]) */
  TEST_LESS_THAN = 0x44,

  /** acc = (acc > registers[r]) */
  TEST_GREATER_THAN = 0x45,

  /** acc = (acc <= registers[r]) */
  TEST_LESS_THAN_OR_EQUAL = 0x46,

  /** acc = (acc >= registers[r]) */
  TEST_GREATER_THAN_OR_EQUAL = 0x47,

  /**
   * acc = (key in obj)
   * operand[0] = register holding the object
   * acc must hold the key string before this instruction
   */
  TEST_IN = 0x48,

  /** acc = (acc instanceof registers[r]) */
  TEST_INSTANCEOF = 0x49,

  /** acc = (acc is undetectable, i.e. null or undefined) */
  TEST_UNDETECTABLE = 0x4a,

  /** acc = !acc  (logical NOT, coerces to boolean) */
  LOGICAL_NOT = 0x4b,

  /**
   * acc = typeof acc
   * Returns one of: 'number', 'string', 'boolean', 'undefined', 'object', 'function'
   */
  TYPEOF = 0x4c,

  // ─────────────────────────────────────────────────────────────────────
  // 0x5_ — Control flow
  //
  // All jump offsets are RELATIVE, measured from the instruction *after*
  // the jump. A positive offset skips forward; negative loops back.
  //
  //   before: ip = N (pointing at JUMP)
  //   after advance: ip = N + 1
  //   apply offset: ip = (N + 1) + offset
  //
  // JUMP_LOOP is semantically identical to JUMP but allows the profiler
  // to identify hot backward edges for OSR (on-stack replacement).
  // ─────────────────────────────────────────────────────────────────────

  /** Unconditional relative jump: ip += offset */
  JUMP = 0x50,

  /** Jump if acc is truthy */
  JUMP_IF_TRUE = 0x51,

  /** Jump if acc is falsy */
  JUMP_IF_FALSE = 0x52,

  /** Jump if acc === null */
  JUMP_IF_NULL = 0x53,

  /** Jump if acc === undefined */
  JUMP_IF_UNDEFINED = 0x54,

  /** Jump if acc === null || acc === undefined */
  JUMP_IF_NULL_OR_UNDEFINED = 0x55,

  /** Jump if ToBoolean(acc) === true  (same as JUMP_IF_TRUE for this VM) */
  JUMP_IF_TO_BOOLEAN_TRUE = 0x56,

  /** Jump if ToBoolean(acc) === false */
  JUMP_IF_TO_BOOLEAN_FALSE = 0x57,

  /** Backward jump for loops (semantically identical to JUMP) */
  JUMP_LOOP = 0x58,

  // ─────────────────────────────────────────────────────────────────────
  // 0x6_ — Function calls, construction, and generators
  //
  // V8 distinguishes "any receiver" calls (where the receiver / `this`
  // might be anything) from "property" calls (method calls where the
  // receiver is the object), mainly so the IC (inline cache) can
  // specialise. In this educational VM we treat them identically.
  //
  // Operand layout for CALL_*:
  //   operands[0] = register holding the callable
  //   operands[1] = first argument register
  //   operands[2] = argument count (argc)
  //   operands[3] = feedback slot
  // ─────────────────────────────────────────────────────────────────────

  /**
   * Call with any receiver.
   * acc = callable(...args)
   */
  CALL_ANY_RECEIVER = 0x60,

  /**
   * Call a property (method): receiver.method(...args)
   * Same operand layout as CALL_ANY_RECEIVER.
   */
  CALL_PROPERTY = 0x61,

  /**
   * Call with undefined receiver (strict-mode function call).
   */
  CALL_UNDEFINED_RECEIVER = 0x62,

  /**
   * Construct: acc = new registers[calleeReg](...args)
   * Like CALL but invokes the constructor protocol.
   */
  CONSTRUCT = 0x63,

  /** Construct with spread: new callable(...spreadArg, ...moreArgs) */
  CONSTRUCT_WITH_SPREAD = 0x64,

  /** Call with spread: callable(...spreadArg, ...moreArgs) */
  CALL_WITH_SPREAD = 0x65,

  /**
   * Return from the current call frame.
   * The accumulator value becomes the caller's accumulator.
   */
  RETURN = 0x66,

  /** Suspend a generator at a yield point. */
  SUSPEND_GENERATOR = 0x67,

  /** Resume a previously suspended generator. */
  RESUME_GENERATOR = 0x68,

  // ─────────────────────────────────────────────────────────────────────
  // 0x7_ — Property access (named & keyed)
  //
  // "Named property" access uses a string literal from the names table:
  //   obj.foo  →  LDA_NAMED_PROPERTY [objReg, nameIdx, feedbackSlot]
  //
  // "Keyed property" access uses a dynamic key in the accumulator:
  //   obj[key]  →  LDA_KEYED_PROPERTY [objReg, feedbackSlot]
  //
  // The "no feedback" variants are used for accesses inside try/catch
  // blocks or other places where the IC is intentionally not updated.
  // ─────────────────────────────────────────────────────────────────────

  /**
   * acc = registers[objReg][names[nameIdx]]
   * Records hiddenClassId in feedback[slot].
   */
  LDA_NAMED_PROPERTY = 0x70,

  /**
   * registers[objReg][names[nameIdx]] = acc
   * Records the property write in feedback[slot].
   */
  STA_NAMED_PROPERTY = 0x71,

  /**
   * acc = registers[objReg][acc]  (acc = key before this instruction)
   */
  LDA_KEYED_PROPERTY = 0x72,

  /**
   * registers[objReg][key] = acc  (key in separate register or acc)
   */
  STA_KEYED_PROPERTY = 0x73,

  /** Like LDA_NAMED_PROPERTY but does NOT update the feedback vector. */
  LDA_NAMED_PROPERTY_NO_FEEDBACK = 0x74,

  /** Like STA_NAMED_PROPERTY but does NOT update the feedback vector. */
  STA_NAMED_PROPERTY_NO_FEEDBACK = 0x75,

  /** delete obj.prop  in strict mode — throws if property is non-configurable */
  DELETE_PROPERTY_STRICT = 0x76,

  /** delete obj.prop  in sloppy mode — silently ignores non-configurable */
  DELETE_PROPERTY_SLOPPY = 0x77,

  // ─────────────────────────────────────────────────────────────────────
  // 0x8_ — Object / array / closure creation
  //
  // These instructions allocate new heap objects.  In the real V8 each
  // one encodes a boilerplate index so the GC can track allocation sites
  // for escape analysis.  Here we keep it simple.
  // ─────────────────────────────────────────────────────────────────────

  /**
   * Create an empty object: acc = {}
   * Assigns a fresh hiddenClassId.
   */
  CREATE_OBJECT_LITERAL = 0x80,

  /** Create an empty array: acc = [] */
  CREATE_ARRAY_LITERAL = 0x81,

  /** Create a RegExp object (stored in constants): acc = /pattern/ */
  CREATE_REGEXP_LITERAL = 0x82,

  /**
   * Create a closure over the current context.
   * operands[0] = index into constants[] of the nested CodeObject
   * acc = { kind: 'function', code: constants[codeIdx], context: frame.context }
   */
  CREATE_CLOSURE = 0x83,

  /** Push a new context frame onto the context chain. */
  CREATE_CONTEXT = 0x84,

  /** Clone a shallow copy of an object literal. */
  CLONE_OBJECT = 0x85,

  // ─────────────────────────────────────────────────────────────────────
  // 0x9_ — Iteration protocol
  //
  // JavaScript iteration is protocol-based: any object can be iterable
  // if it implements [Symbol.iterator]().  These instructions implement
  // the low-level mechanics that compiled for-of / for-await loops use.
  // ─────────────────────────────────────────────────────────────────────

  /** acc = acc[Symbol.iterator]()  (obtain an iterator) */
  GET_ITERATOR = 0x90,

  /** Call iterator.next() and store result in acc */
  CALL_ITERATOR_STEP = 0x91,

  /** acc = iterResult.done */
  GET_ITERATOR_DONE = 0x92,

  /** acc = iterResult.value */
  GET_ITERATOR_VALUE = 0x93,

  // ─────────────────────────────────────────────────────────────────────
  // 0xA_ — Exception control
  // ─────────────────────────────────────────────────────────────────────

  /** throw acc  — raises a VMError with the accumulator as message */
  THROW = 0xa0,

  /** Re-throw the current exception (inside a catch handler) */
  RETHROW = 0xa1,

  // ─────────────────────────────────────────────────────────────────────
  // 0xB_ — Context / scope management
  // ─────────────────────────────────────────────────────────────────────

  /** Push a new scope context onto the context chain */
  PUSH_CONTEXT = 0xb0,

  /** Pop the current scope context, restoring the parent */
  POP_CONTEXT = 0xb1,

  /** acc = module.exports[names[nameIdx]] */
  LDA_MODULE_VARIABLE = 0xb4,

  /** module.exports[names[nameIdx]] = acc */
  STA_MODULE_VARIABLE = 0xb5,

  // ─────────────────────────────────────────────────────────────────────
  // 0xF_ — VM meta-instructions
  // ─────────────────────────────────────────────────────────────────────

  /**
   * Stack overflow check.
   * Implemented by incrementing callDepth and throwing if it exceeds
   * the configured maximum.  V8 inserts this at every function entry.
   */
  STACK_CHECK = 0xf0,

  /**
   * Trigger a debugger breakpoint (or no-op if not attached).
   */
  DEBUGGER = 0xf1,

  /**
   * Stop execution and return the accumulator.
   * Unlike RETURN, HALT stops the *entire* VM (not just the current frame).
   */
  HALT = 0xff,
}

// ─────────────────────────────────────────────────────────────────────────────
// Reverse-lookup table: opcode number → human-readable name.
// Built once at module load time from the enum entries above.
// ─────────────────────────────────────────────────────────────────────────────

const OPCODE_NAMES: Map<number, string> = new Map([
  // 0x0_ Accumulator loads
  [Opcode.LDA_CONSTANT, 'LDA_CONSTANT'],
  [Opcode.LDA_ZERO, 'LDA_ZERO'],
  [Opcode.LDA_SMI, 'LDA_SMI'],
  [Opcode.LDA_UNDEFINED, 'LDA_UNDEFINED'],
  [Opcode.LDA_NULL, 'LDA_NULL'],
  [Opcode.LDA_TRUE, 'LDA_TRUE'],
  [Opcode.LDA_FALSE, 'LDA_FALSE'],
  // 0x1_ Register moves
  [Opcode.LDAR, 'LDAR'],
  [Opcode.STAR, 'STAR'],
  [Opcode.MOV, 'MOV'],
  // 0x2_ Variable access
  [Opcode.LDA_GLOBAL, 'LDA_GLOBAL'],
  [Opcode.STA_GLOBAL, 'STA_GLOBAL'],
  [Opcode.LDA_LOCAL, 'LDA_LOCAL'],
  [Opcode.STA_LOCAL, 'STA_LOCAL'],
  [Opcode.LDA_CONTEXT_SLOT, 'LDA_CONTEXT_SLOT'],
  [Opcode.STA_CONTEXT_SLOT, 'STA_CONTEXT_SLOT'],
  [Opcode.LDA_CURRENT_CONTEXT_SLOT, 'LDA_CURRENT_CONTEXT_SLOT'],
  [Opcode.STA_CURRENT_CONTEXT_SLOT, 'STA_CURRENT_CONTEXT_SLOT'],
  // 0x3_ Arithmetic
  [Opcode.ADD, 'ADD'],
  [Opcode.SUB, 'SUB'],
  [Opcode.MUL, 'MUL'],
  [Opcode.DIV, 'DIV'],
  [Opcode.MOD, 'MOD'],
  [Opcode.POW, 'POW'],
  [Opcode.ADD_SMI, 'ADD_SMI'],
  [Opcode.SUB_SMI, 'SUB_SMI'],
  [Opcode.BITWISE_AND, 'BITWISE_AND'],
  [Opcode.BITWISE_OR, 'BITWISE_OR'],
  [Opcode.BITWISE_XOR, 'BITWISE_XOR'],
  [Opcode.BITWISE_NOT, 'BITWISE_NOT'],
  [Opcode.SHIFT_LEFT, 'SHIFT_LEFT'],
  [Opcode.SHIFT_RIGHT, 'SHIFT_RIGHT'],
  [Opcode.SHIFT_RIGHT_LOGICAL, 'SHIFT_RIGHT_LOGICAL'],
  [Opcode.NEGATE, 'NEGATE'],
  // 0x4_ Comparisons
  [Opcode.TEST_EQUAL, 'TEST_EQUAL'],
  [Opcode.TEST_NOT_EQUAL, 'TEST_NOT_EQUAL'],
  [Opcode.TEST_STRICT_EQUAL, 'TEST_STRICT_EQUAL'],
  [Opcode.TEST_STRICT_NOT_EQUAL, 'TEST_STRICT_NOT_EQUAL'],
  [Opcode.TEST_LESS_THAN, 'TEST_LESS_THAN'],
  [Opcode.TEST_GREATER_THAN, 'TEST_GREATER_THAN'],
  [Opcode.TEST_LESS_THAN_OR_EQUAL, 'TEST_LESS_THAN_OR_EQUAL'],
  [Opcode.TEST_GREATER_THAN_OR_EQUAL, 'TEST_GREATER_THAN_OR_EQUAL'],
  [Opcode.TEST_IN, 'TEST_IN'],
  [Opcode.TEST_INSTANCEOF, 'TEST_INSTANCEOF'],
  [Opcode.TEST_UNDETECTABLE, 'TEST_UNDETECTABLE'],
  [Opcode.LOGICAL_NOT, 'LOGICAL_NOT'],
  [Opcode.TYPEOF, 'TYPEOF'],
  // 0x5_ Control flow
  [Opcode.JUMP, 'JUMP'],
  [Opcode.JUMP_IF_TRUE, 'JUMP_IF_TRUE'],
  [Opcode.JUMP_IF_FALSE, 'JUMP_IF_FALSE'],
  [Opcode.JUMP_IF_NULL, 'JUMP_IF_NULL'],
  [Opcode.JUMP_IF_UNDEFINED, 'JUMP_IF_UNDEFINED'],
  [Opcode.JUMP_IF_NULL_OR_UNDEFINED, 'JUMP_IF_NULL_OR_UNDEFINED'],
  [Opcode.JUMP_IF_TO_BOOLEAN_TRUE, 'JUMP_IF_TO_BOOLEAN_TRUE'],
  [Opcode.JUMP_IF_TO_BOOLEAN_FALSE, 'JUMP_IF_TO_BOOLEAN_FALSE'],
  [Opcode.JUMP_LOOP, 'JUMP_LOOP'],
  // 0x6_ Calls
  [Opcode.CALL_ANY_RECEIVER, 'CALL_ANY_RECEIVER'],
  [Opcode.CALL_PROPERTY, 'CALL_PROPERTY'],
  [Opcode.CALL_UNDEFINED_RECEIVER, 'CALL_UNDEFINED_RECEIVER'],
  [Opcode.CONSTRUCT, 'CONSTRUCT'],
  [Opcode.CONSTRUCT_WITH_SPREAD, 'CONSTRUCT_WITH_SPREAD'],
  [Opcode.CALL_WITH_SPREAD, 'CALL_WITH_SPREAD'],
  [Opcode.RETURN, 'RETURN'],
  [Opcode.SUSPEND_GENERATOR, 'SUSPEND_GENERATOR'],
  [Opcode.RESUME_GENERATOR, 'RESUME_GENERATOR'],
  // 0x7_ Property access
  [Opcode.LDA_NAMED_PROPERTY, 'LDA_NAMED_PROPERTY'],
  [Opcode.STA_NAMED_PROPERTY, 'STA_NAMED_PROPERTY'],
  [Opcode.LDA_KEYED_PROPERTY, 'LDA_KEYED_PROPERTY'],
  [Opcode.STA_KEYED_PROPERTY, 'STA_KEYED_PROPERTY'],
  [Opcode.LDA_NAMED_PROPERTY_NO_FEEDBACK, 'LDA_NAMED_PROPERTY_NO_FEEDBACK'],
  [Opcode.STA_NAMED_PROPERTY_NO_FEEDBACK, 'STA_NAMED_PROPERTY_NO_FEEDBACK'],
  [Opcode.DELETE_PROPERTY_STRICT, 'DELETE_PROPERTY_STRICT'],
  [Opcode.DELETE_PROPERTY_SLOPPY, 'DELETE_PROPERTY_SLOPPY'],
  // 0x8_ Object/array creation
  [Opcode.CREATE_OBJECT_LITERAL, 'CREATE_OBJECT_LITERAL'],
  [Opcode.CREATE_ARRAY_LITERAL, 'CREATE_ARRAY_LITERAL'],
  [Opcode.CREATE_REGEXP_LITERAL, 'CREATE_REGEXP_LITERAL'],
  [Opcode.CREATE_CLOSURE, 'CREATE_CLOSURE'],
  [Opcode.CREATE_CONTEXT, 'CREATE_CONTEXT'],
  [Opcode.CLONE_OBJECT, 'CLONE_OBJECT'],
  // 0x9_ Iteration
  [Opcode.GET_ITERATOR, 'GET_ITERATOR'],
  [Opcode.CALL_ITERATOR_STEP, 'CALL_ITERATOR_STEP'],
  [Opcode.GET_ITERATOR_DONE, 'GET_ITERATOR_DONE'],
  [Opcode.GET_ITERATOR_VALUE, 'GET_ITERATOR_VALUE'],
  // 0xA_ Exceptions
  [Opcode.THROW, 'THROW'],
  [Opcode.RETHROW, 'RETHROW'],
  // 0xB_ Context/scope
  [Opcode.PUSH_CONTEXT, 'PUSH_CONTEXT'],
  [Opcode.POP_CONTEXT, 'POP_CONTEXT'],
  [Opcode.LDA_MODULE_VARIABLE, 'LDA_MODULE_VARIABLE'],
  [Opcode.STA_MODULE_VARIABLE, 'STA_MODULE_VARIABLE'],
  // 0xF_ VM control
  [Opcode.STACK_CHECK, 'STACK_CHECK'],
  [Opcode.DEBUGGER, 'DEBUGGER'],
  [Opcode.HALT, 'HALT'],
]);

/**
 * Return the human-readable name for a numeric opcode.
 *
 * @example
 *   opcodeName(0x00) // → 'LDA_CONSTANT'
 *   opcodeName(0xFF) // → 'HALT'
 *   opcodeName(0x99) // → 'UNKNOWN(0x99)'
 */
export function opcodeName(op: number): string {
  return OPCODE_NAMES.get(op) ?? `UNKNOWN(0x${op.toString(16).toUpperCase().padStart(2, '0')})`;
}
