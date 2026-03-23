/**
 * Starlark Opcodes -- The instruction set for the Starlark virtual machine.
 *
 * ==========================================================================
 * Chapter 1: Why Starlark Has Its Own Opcodes
 * ==========================================================================
 *
 * The GenericVM is a blank slate -- it has no built-in opcodes. Languages
 * register their own opcodes via ``vm.registerOpcode(number, handler)``.
 * This module defines the opcode *numbers* and *names* for Starlark.
 *
 * These opcodes are Starlark's "machine language." The Starlark compiler
 * translates Starlark source code into sequences of these opcodes, and the
 * Starlark VM executes them. A future Python plugin would define additional
 * opcodes (SETUP_EXCEPT, YIELD_VALUE, etc.) but reuse many of these.
 *
 * ==========================================================================
 * Chapter 2: Opcode Organization
 * ==========================================================================
 *
 * Opcodes are grouped by category using the high nibble (first hex digit):
 *
 *     0x0_ = Stack operations      (push, pop, dup, load constants)
 *     0x1_ = Variable operations   (store/load by name or slot)
 *     0x2_ = Arithmetic            (add, sub, mul, div, bitwise)
 *     0x3_ = Comparison & boolean  (==, !=, <, >, in, not)
 *     0x4_ = Control flow          (jump, branch)
 *     0x5_ = Functions             (make, call, return)
 *     0x6_ = Collections           (build list, dict, tuple)
 *     0x7_ = Subscript & attribute (indexing, slicing, dot access)
 *     0x8_ = Iteration             (get_iter, for_iter, unpack)
 *     0x9_ = Module                (load statement)
 *     0xA_ = I/O                   (print)
 *     0xF_ = VM control            (halt)
 *
 * This grouping mirrors the JVM's organization and makes it easy to tell
 * an instruction's category at a glance from its hex value.
 */

// =========================================================================
// Op -- The Starlark opcode enumeration
// =========================================================================

/**
 * Starlark bytecode opcodes.
 *
 * Each value is a single byte (0x00-0xFF). The high nibble groups opcodes
 * by category. Handlers for each opcode are registered with the GenericVM
 * by the Starlark VM plugin.
 *
 * We use a plain ``const`` object with ``as const`` rather than a TypeScript
 * enum, following the same convention as the virtual-machine package. This
 * produces cleaner JavaScript output and is easier to iterate over.
 *
 * Stack effect notation:
 *     -> value     = pushes one value
 *     value ->     = pops one value
 *     a b -> c     = pops two, pushes one
 */
export const Op = {
  // =====================================================================
  // Stack Operations (0x0_)
  // =====================================================================

  /** Push a constant from the pool. Operand: pool index. -> value */
  LOAD_CONST: 0x01,

  /** Discard top of stack. value -> */
  POP: 0x02,

  /** Duplicate top of stack. value -> value value */
  DUP: 0x03,

  /** Push None. -> None */
  LOAD_NONE: 0x04,

  /** Push True. -> True */
  LOAD_TRUE: 0x05,

  /** Push False. -> False */
  LOAD_FALSE: 0x06,

  // =====================================================================
  // Variable Operations (0x1_)
  // =====================================================================

  /** Pop and store in named variable. Operand: name index. value -> */
  STORE_NAME: 0x10,

  /** Push named variable's value. Operand: name index. -> value */
  LOAD_NAME: 0x11,

  /** Pop and store in local slot. Operand: slot index. value -> */
  STORE_LOCAL: 0x12,

  /** Push local slot's value. Operand: slot index. -> value */
  LOAD_LOCAL: 0x13,

  /** Pop and store in closure cell. Operand: cell index. value -> */
  STORE_CLOSURE: 0x14,

  /** Push closure cell's value. Operand: cell index. -> value */
  LOAD_CLOSURE: 0x15,

  // =====================================================================
  // Arithmetic Operations (0x2_)
  // =====================================================================

  /** Pop two values, push a + b. Supports int, float, str, list. a b -> result */
  ADD: 0x20,

  /** Pop two values, push a - b. a b -> result */
  SUB: 0x21,

  /** Pop two values, push a * b. Also handles str * int. a b -> result */
  MUL: 0x22,

  /** Pop two values, push a / b (float division). a b -> result */
  DIV: 0x23,

  /** Pop two values, push a // b. a b -> result */
  FLOOR_DIV: 0x24,

  /** Pop two values, push a % b. Also handles str formatting. a b -> result */
  MOD: 0x25,

  /** Pop two values, push a ** b. a b -> result */
  POWER: 0x26,

  /** Pop one value, push -a. a -> -a */
  NEGATE: 0x27,

  /** Pop two values, push a & b. a b -> result */
  BIT_AND: 0x28,

  /** Pop two values, push a | b. a b -> result */
  BIT_OR: 0x29,

  /** Pop two values, push a ^ b. a b -> result */
  BIT_XOR: 0x2a,

  /** Pop one value, push ~a. a -> ~a */
  BIT_NOT: 0x2b,

  /** Pop two values, push a << b. a b -> result */
  LSHIFT: 0x2c,

  /** Pop two values, push a >> b. a b -> result */
  RSHIFT: 0x2d,

  // =====================================================================
  // Comparison Operations (0x3_)
  // =====================================================================

  /** Pop two values, push a == b. a b -> bool */
  CMP_EQ: 0x30,

  /** Pop two values, push a != b. a b -> bool */
  CMP_NE: 0x31,

  /** Pop two values, push a < b. a b -> bool */
  CMP_LT: 0x32,

  /** Pop two values, push a > b. a b -> bool */
  CMP_GT: 0x33,

  /** Pop two values, push a <= b. a b -> bool */
  CMP_LE: 0x34,

  /** Pop two values, push a >= b. a b -> bool */
  CMP_GE: 0x35,

  /** Pop two values, push a in b. a b -> bool */
  CMP_IN: 0x36,

  /** Pop two values, push a not in b. a b -> bool */
  CMP_NOT_IN: 0x37,

  // =====================================================================
  // Boolean Operations (0x38)
  // =====================================================================

  /** Pop one value, push logical not. a -> !a */
  NOT: 0x38,

  // =====================================================================
  // Control Flow (0x4_)
  // =====================================================================

  /** Unconditional jump. Operand: target index. */
  JUMP: 0x40,

  /** Pop value, jump if falsy. Operand: target. value -> */
  JUMP_IF_FALSE: 0x41,

  /** Pop value, jump if truthy. Operand: target. value -> */
  JUMP_IF_TRUE: 0x42,

  /**
   * If top is falsy, jump (keep value); else pop. For ``and`` short-circuit.
   * Operand: target. value -> value? (if jump) or -> (if no jump)
   */
  JUMP_IF_FALSE_OR_POP: 0x43,

  /**
   * If top is truthy, jump (keep value); else pop. For ``or`` short-circuit.
   * Operand: target. value -> value? (if jump) or -> (if no jump)
   */
  JUMP_IF_TRUE_OR_POP: 0x44,

  // =====================================================================
  // Function Operations (0x5_)
  // =====================================================================

  /** Create a function object. Operand: flags. code defaults -> func */
  MAKE_FUNCTION: 0x50,

  /** Call function with N positional args. Operand: arg count. func args -> result */
  CALL_FUNCTION: 0x51,

  /** Call function with keyword args. Operand: total arg count. func args kw_names -> result */
  CALL_FUNCTION_KW: 0x52,

  /** Return from function. value -> */
  RETURN: 0x53,

  // =====================================================================
  // Collection Operations (0x6_)
  // =====================================================================

  /** Create list from N stack items. Operand: count. items -> list */
  BUILD_LIST: 0x60,

  /** Create dict from N key-value pairs. Operand: pair count. key1 val1 ... -> dict */
  BUILD_DICT: 0x61,

  /** Create tuple from N stack items. Operand: count. items -> tuple */
  BUILD_TUPLE: 0x62,

  /** Append value to list (for comprehensions). list value -> list */
  LIST_APPEND: 0x63,

  /** Set dict entry (for comprehensions). dict key value -> dict */
  DICT_SET: 0x64,

  // =====================================================================
  // Subscript & Attribute Operations (0x7_)
  // =====================================================================

  /** obj[key]. obj key -> value */
  LOAD_SUBSCRIPT: 0x70,

  /** obj[key] = value. obj key value -> */
  STORE_SUBSCRIPT: 0x71,

  /** obj.attr. Operand: attr name index. obj -> value */
  LOAD_ATTR: 0x72,

  /** obj.attr = value. Operand: attr name index. obj value -> */
  STORE_ATTR: 0x73,

  /** obj[start:stop:step]. Operand: flags for which are present. obj start? stop? step? -> value */
  LOAD_SLICE: 0x74,

  // =====================================================================
  // Iteration Operations (0x8_)
  // =====================================================================

  /** Get iterator from iterable. iterable -> iterator */
  GET_ITER: 0x80,

  /**
   * Get next from iterator, or jump to end. Operand: target.
   * iterator -> iterator value (if has next)
   * iterator -> (if exhausted, jumps to target)
   */
  FOR_ITER: 0x81,

  /** Unpack N items from sequence. Operand: count. seq -> items */
  UNPACK_SEQUENCE: 0x82,

  // =====================================================================
  // Module Operations (0x9_)
  // =====================================================================

  /** Load a module (for load() statement). Operand: module name index. -> module */
  LOAD_MODULE: 0x90,

  /** Extract symbol from module. Operand: symbol name index. module -> value */
  IMPORT_FROM: 0x91,

  // =====================================================================
  // I/O Operations (0xA_)
  // =====================================================================

  /** Pop and print value, capture in output. value -> */
  PRINT: 0xa0,

  // =====================================================================
  // VM Control (0xF_)
  // =====================================================================

  /** Stop execution. */
  HALT: 0xff,
} as const;

/**
 * The type of an individual Starlark opcode value.
 *
 * TypeScript's ``typeof Op[keyof typeof Op]`` extracts the union of all
 * numeric values in the Op object. This lets us type-check that instructions
 * only contain valid Starlark opcodes.
 */
export type OpValue = (typeof Op)[keyof typeof Op];

// =========================================================================
// Operator-to-opcode mappings (used by the compiler)
// =========================================================================

/**
 * Maps binary operator symbols to their bytecode opcodes.
 *
 * Used by the compiler when it encounters an ``arith``, ``term``, ``shift``,
 * or other binary-expression grammar rule. The compiler extracts the operator
 * token ("+", "-", etc.) and looks up the corresponding opcode here.
 *
 * Example flow for ``1 + 2``:
 *   1. Parser produces: ASTNode("arith", [INT("1"), PLUS("+"), INT("2")])
 *   2. Compiler compiles INT("1") -> LOAD_CONST 0
 *   3. Compiler compiles INT("2") -> LOAD_CONST 1
 *   4. Compiler looks up "+" in BINARY_OP_MAP -> Op.ADD
 *   5. Compiler emits ADD
 */
export const BINARY_OP_MAP: Record<string, number> = {
  "+": Op.ADD,
  "-": Op.SUB,
  "*": Op.MUL,
  "/": Op.DIV,
  "//": Op.FLOOR_DIV,
  "%": Op.MOD,
  "**": Op.POWER,
  "&": Op.BIT_AND,
  "|": Op.BIT_OR,
  "^": Op.BIT_XOR,
  "<<": Op.LSHIFT,
  ">>": Op.RSHIFT,
};

/**
 * Maps comparison operator symbols to their bytecode opcodes.
 *
 * The comparison handler extracts the operator from a ``comp_op`` AST node
 * and looks it up here. Note the special ``"not in"`` entry -- the parser
 * produces two tokens ("not" and "in") which the compiler concatenates.
 */
export const COMPARE_OP_MAP: Record<string, number> = {
  "==": Op.CMP_EQ,
  "!=": Op.CMP_NE,
  "<": Op.CMP_LT,
  ">": Op.CMP_GT,
  "<=": Op.CMP_LE,
  ">=": Op.CMP_GE,
  in: Op.CMP_IN,
  "not in": Op.CMP_NOT_IN,
};

/**
 * Maps augmented assignment operators to their underlying arithmetic opcodes.
 *
 * When the compiler encounters ``x += 1``, it needs to:
 *   1. Load x
 *   2. Load 1
 *   3. Emit the arithmetic opcode for "+=" (which is ADD)
 *   4. Store back to x
 *
 * This map provides step 3 -- it tells us which arithmetic opcode corresponds
 * to each augmented assignment operator.
 */
export const AUGMENTED_ASSIGN_MAP: Record<string, number> = {
  "+=": Op.ADD,
  "-=": Op.SUB,
  "*=": Op.MUL,
  "/=": Op.DIV,
  "//=": Op.FLOOR_DIV,
  "%=": Op.MOD,
  "&=": Op.BIT_AND,
  "|=": Op.BIT_OR,
  "^=": Op.BIT_XOR,
  "<<=": Op.LSHIFT,
  ">>=": Op.RSHIFT,
  "**=": Op.POWER,
};

/**
 * Maps unary operator symbols to their bytecode opcodes.
 *
 * Note: unary ``+`` doesn't have a dedicated opcode. It evaluates the
 * expression (for type checking) but doesn't change the value. The factor
 * handler treats it as a no-op.
 */
export const UNARY_OP_MAP: Record<string, number> = {
  "-": Op.NEGATE,
  "+": Op.POP, // unary + is a no-op on valid numeric types, but we still eval
  "~": Op.BIT_NOT,
};
