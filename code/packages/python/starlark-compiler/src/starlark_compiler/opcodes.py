"""Starlark Opcodes — The instruction set for the Starlark virtual machine.

==========================================================================
Chapter 1: Why Starlark Has Its Own Opcodes
==========================================================================

The GenericVM is a blank slate — it has no built-in opcodes. Languages
register their own opcodes via ``vm.register_opcode(number, handler)``.
This module defines the opcode *numbers* and *names* for Starlark.

These opcodes are Starlark's "machine language." The Starlark compiler
translates Starlark source code into sequences of these opcodes, and the
Starlark VM executes them. A future Python plugin would define additional
opcodes (SETUP_EXCEPT, YIELD_VALUE, etc.) but reuse many of these.

==========================================================================
Chapter 2: Opcode Organization
==========================================================================

Opcodes are grouped by category using the high nibble (first hex digit):

    0x0_ = Stack operations      (push, pop, dup, load constants)
    0x1_ = Variable operations   (store/load by name or slot)
    0x2_ = Arithmetic            (add, sub, mul, div, bitwise)
    0x3_ = Comparison & boolean  (==, !=, <, >, in, not)
    0x4_ = Control flow          (jump, branch)
    0x5_ = Functions             (make, call, return)
    0x6_ = Collections           (build list, dict, tuple)
    0x7_ = Subscript & attribute (indexing, slicing, dot access)
    0x8_ = Iteration             (get_iter, for_iter, unpack)
    0x9_ = Module                (load statement)
    0xA_ = I/O                   (print)
    0xF_ = VM control            (halt)

This grouping mirrors the JVM's organization and makes it easy to tell
an instruction's category at a glance from its hex value.
"""

from __future__ import annotations

from enum import IntEnum


class Op(IntEnum):
    """Starlark bytecode opcodes.

    Each value is a single byte (0x00-0xFF). The high nibble groups opcodes
    by category. Handlers for each opcode are registered with the GenericVM
    by the Starlark VM plugin.

    Stack effect notation:
        → value     = pushes one value
        value →     = pops one value
        a b → c     = pops two, pushes one
    """

    # =====================================================================
    # Stack Operations (0x0_)
    # =====================================================================

    LOAD_CONST = 0x01
    """Push a constant from the pool. Operand: pool index. → value"""

    POP = 0x02
    """Discard top of stack. value →"""

    DUP = 0x03
    """Duplicate top of stack. value → value value"""

    LOAD_NONE = 0x04
    """Push None. → None"""

    LOAD_TRUE = 0x05
    """Push True. → True"""

    LOAD_FALSE = 0x06
    """Push False. → False"""

    # =====================================================================
    # Variable Operations (0x1_)
    # =====================================================================

    STORE_NAME = 0x10
    """Pop and store in named variable. Operand: name index. value →"""

    LOAD_NAME = 0x11
    """Push named variable's value. Operand: name index. → value"""

    STORE_LOCAL = 0x12
    """Pop and store in local slot. Operand: slot index. value →"""

    LOAD_LOCAL = 0x13
    """Push local slot's value. Operand: slot index. → value"""

    STORE_CLOSURE = 0x14
    """Pop and store in closure cell. Operand: cell index. value →"""

    LOAD_CLOSURE = 0x15
    """Push closure cell's value. Operand: cell index. → value"""

    # =====================================================================
    # Arithmetic Operations (0x2_)
    # =====================================================================

    ADD = 0x20
    """Pop two values, push a + b. Supports int, float, str, list.
    a b → result"""

    SUB = 0x21
    """Pop two values, push a - b. a b → result"""

    MUL = 0x22
    """Pop two values, push a * b. Also handles str * int. a b → result"""

    DIV = 0x23
    """Pop two values, push a / b (float division). a b → result"""

    FLOOR_DIV = 0x24
    """Pop two values, push a // b. a b → result"""

    MOD = 0x25
    """Pop two values, push a % b. Also handles str formatting. a b → result"""

    POWER = 0x26
    """Pop two values, push a ** b. a b → result"""

    NEGATE = 0x27
    """Pop one value, push -a. a → -a"""

    BIT_AND = 0x28
    """Pop two values, push a & b. a b → result"""

    BIT_OR = 0x29
    """Pop two values, push a | b. a b → result"""

    BIT_XOR = 0x2A
    """Pop two values, push a ^ b. a b → result"""

    BIT_NOT = 0x2B
    """Pop one value, push ~a. a → ~a"""

    LSHIFT = 0x2C
    """Pop two values, push a << b. a b → result"""

    RSHIFT = 0x2D
    """Pop two values, push a >> b. a b → result"""

    # =====================================================================
    # Comparison Operations (0x3_)
    # =====================================================================

    CMP_EQ = 0x30
    """Pop two values, push a == b. a b → bool"""

    CMP_NE = 0x31
    """Pop two values, push a != b. a b → bool"""

    CMP_LT = 0x32
    """Pop two values, push a < b. a b → bool"""

    CMP_GT = 0x33
    """Pop two values, push a > b. a b → bool"""

    CMP_LE = 0x34
    """Pop two values, push a <= b. a b → bool"""

    CMP_GE = 0x35
    """Pop two values, push a >= b. a b → bool"""

    CMP_IN = 0x36
    """Pop two values, push a in b. a b → bool"""

    CMP_NOT_IN = 0x37
    """Pop two values, push a not in b. a b → bool"""

    # =====================================================================
    # Boolean Operations (0x38)
    # =====================================================================

    NOT = 0x38
    """Pop one value, push logical not. a → !a"""

    # =====================================================================
    # Control Flow (0x4_)
    # =====================================================================

    JUMP = 0x40
    """Unconditional jump. Operand: target index."""

    JUMP_IF_FALSE = 0x41
    """Pop value, jump if falsy. Operand: target. value →"""

    JUMP_IF_TRUE = 0x42
    """Pop value, jump if truthy. Operand: target. value →"""

    JUMP_IF_FALSE_OR_POP = 0x43
    """If top is falsy, jump (keep value); else pop. For ``and`` short-circuit.
    Operand: target. value → value? (if jump) or → (if no jump)"""

    JUMP_IF_TRUE_OR_POP = 0x44
    """If top is truthy, jump (keep value); else pop. For ``or`` short-circuit.
    Operand: target. value → value? (if jump) or → (if no jump)"""

    # =====================================================================
    # Function Operations (0x5_)
    # =====================================================================

    MAKE_FUNCTION = 0x50
    """Create a function object. Operand: flags.
    code defaults → func"""

    CALL_FUNCTION = 0x51
    """Call function with N positional args. Operand: arg count.
    func args → result"""

    CALL_FUNCTION_KW = 0x52
    """Call function with keyword args. Operand: total arg count.
    func args kw_names → result"""

    RETURN = 0x53
    """Return from function. value →"""

    # =====================================================================
    # Collection Operations (0x6_)
    # =====================================================================

    BUILD_LIST = 0x60
    """Create list from N stack items. Operand: count. items → list"""

    BUILD_DICT = 0x61
    """Create dict from N key-value pairs. Operand: pair count.
    key1 val1 key2 val2 ... → dict"""

    BUILD_TUPLE = 0x62
    """Create tuple from N stack items. Operand: count. items → tuple"""

    LIST_APPEND = 0x63
    """Append value to list (for comprehensions). list value → list"""

    DICT_SET = 0x64
    """Set dict entry (for comprehensions). dict key value → dict"""

    # =====================================================================
    # Subscript & Attribute Operations (0x7_)
    # =====================================================================

    LOAD_SUBSCRIPT = 0x70
    """obj[key]. obj key → value"""

    STORE_SUBSCRIPT = 0x71
    """obj[key] = value. obj key value →"""

    LOAD_ATTR = 0x72
    """obj.attr. Operand: attr name index. obj → value"""

    STORE_ATTR = 0x73
    """obj.attr = value. Operand: attr name index. obj value →"""

    LOAD_SLICE = 0x74
    """obj[start:stop:step]. Operand: flags for which are present.
    obj start? stop? step? → value"""

    # =====================================================================
    # Iteration Operations (0x8_)
    # =====================================================================

    GET_ITER = 0x80
    """Get iterator from iterable. iterable → iterator"""

    FOR_ITER = 0x81
    """Get next from iterator, or jump to end. Operand: target.
    iterator → iterator value (if has next)
    iterator → (if exhausted, jumps to target)"""

    UNPACK_SEQUENCE = 0x82
    """Unpack N items from sequence. Operand: count. seq → items"""

    # =====================================================================
    # Module Operations (0x9_)
    # =====================================================================

    LOAD_MODULE = 0x90
    """Load a module (for load() statement). Operand: module name index.
    → module"""

    IMPORT_FROM = 0x91
    """Extract symbol from module. Operand: symbol name index.
    module → value"""

    # =====================================================================
    # I/O Operations (0xA_)
    # =====================================================================

    PRINT = 0xA0
    """Pop and print value, capture in output. value →"""

    # =====================================================================
    # VM Control (0xF_)
    # =====================================================================

    HALT = 0xFF
    """Stop execution."""


# =========================================================================
# Operator-to-opcode mappings (used by the compiler)
# =========================================================================

BINARY_OP_MAP: dict[str, Op] = {
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
}
"""Maps binary operator symbols to their bytecode opcodes.

Used by the compiler when it encounters an ``arith``, ``term``, ``shift``,
or other binary-expression grammar rule.
"""

COMPARE_OP_MAP: dict[str, Op] = {
    "==": Op.CMP_EQ,
    "!=": Op.CMP_NE,
    "<": Op.CMP_LT,
    ">": Op.CMP_GT,
    "<=": Op.CMP_LE,
    ">=": Op.CMP_GE,
    "in": Op.CMP_IN,
    "not in": Op.CMP_NOT_IN,
}
"""Maps comparison operator symbols to their bytecode opcodes."""

AUGMENTED_ASSIGN_MAP: dict[str, Op] = {
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
}
"""Maps augmented assignment operators to their underlying arithmetic opcodes."""

UNARY_OP_MAP: dict[str, Op] = {
    "-": Op.NEGATE,
    "+": Op.POP,  # unary + is a no-op on valid numeric types, but we still eval
    "~": Op.BIT_NOT,
}
"""Maps unary operator symbols to their bytecode opcodes.

Note: unary ``+`` doesn't have a dedicated opcode. It evaluates the
expression (for type checking) but doesn't change the value. We handle
this specially in the factor handler.
"""
