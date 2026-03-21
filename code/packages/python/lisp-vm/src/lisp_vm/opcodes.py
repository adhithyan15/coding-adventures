"""Lisp Opcodes — The instruction set for McCarthy's 1960 Lisp.

==========================================================================
Chapter 1: Why Lisp Has Its Own Opcodes
==========================================================================

The GenericVM is a blank slate — it has no built-in opcodes. Languages
register their own opcodes via ``vm.register_opcode(number, handler)``.
This module defines the opcode *numbers* and *names* for Lisp.

Lisp needs fewer opcodes than Starlark because Lisp's data model is
simpler: everything is either an atom (number, symbol) or a cons cell
(pair). No lists, dicts, tuples, slices, or attribute access needed.

However, Lisp adds opcodes that Starlark doesn't need:
- ``CONS``, ``CAR``, ``CDR`` — cons cell operations
- ``MAKE_SYMBOL`` — symbol interning
- ``IS_ATOM``, ``IS_NIL`` — type predicates
- ``TAIL_CALL`` — tail call optimization (reuse call frame)

==========================================================================
Chapter 2: Opcode Organization
==========================================================================

Opcodes are grouped by category using the high nibble (first hex digit):

    0x0_ = Stack operations      (push constants, nil, true)
    0x1_ = Variable operations   (store/load by name or slot)
    0x2_ = Arithmetic            (add, sub, mul, div)
    0x3_ = Comparison            (eq, lt, gt)
    0x4_ = Control flow          (jump, branch)
    0x5_ = Functions             (closures, call, tail call, return)
    0x7_ = Lisp-specific         (cons cells, symbols, predicates)
    0xA_ = I/O                   (print)
    0xF_ = VM control            (halt)

This mirrors the Starlark opcode organization.
"""

from __future__ import annotations

from enum import IntEnum


class LispOp(IntEnum):
    """Lisp bytecode opcodes.

    Each value is a single byte (0x00-0xFF). The high nibble groups opcodes
    by category. Handlers for each opcode are registered with the GenericVM
    by the Lisp VM plugin.

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

    LOAD_NIL = 0x03
    """Push the NIL sentinel. → NIL

    NIL is Lisp's "nothing" value — the empty list, the false value,
    the end-of-list marker. It is a distinct Python object, not None
    or 0 or False.
    """

    LOAD_TRUE = 0x04
    """Push True (Lisp's 't'). → True"""

    # =====================================================================
    # Variable Operations (0x1_)
    # =====================================================================

    STORE_NAME = 0x10
    """Pop and store in a named variable. Operand: name index. value →"""

    LOAD_NAME = 0x11
    """Push a named variable's value. Operand: name index. → value"""

    STORE_LOCAL = 0x12
    """Pop and store in a local slot. Operand: slot index. value →"""

    LOAD_LOCAL = 0x13
    """Push a local slot's value. Operand: slot index. → value"""

    # =====================================================================
    # Arithmetic (0x2_)
    # =====================================================================

    ADD = 0x20
    """Add two numbers. a b → (a + b)"""

    SUB = 0x21
    """Subtract. a b → (a - b)"""

    MUL = 0x22
    """Multiply. a b → (a * b)"""

    DIV = 0x23
    """Integer divide. a b → (a // b)"""

    # =====================================================================
    # Comparison (0x3_)
    # =====================================================================

    CMP_EQ = 0x30
    """Equality. a b → (1 if a == b else 0)

    For Lisp's 'eq': compares by identity for heap objects (same address),
    by value for numbers and NIL.
    """

    CMP_LT = 0x31
    """Less than. a b → (1 if a < b else 0)"""

    CMP_GT = 0x32
    """Greater than. a b → (1 if a > b else 0)"""

    # =====================================================================
    # Control Flow (0x4_)
    # =====================================================================

    JUMP = 0x40
    """Unconditional jump. Operand: target PC."""

    JUMP_IF_FALSE = 0x41
    """Jump if top is falsy. Operand: target PC. value →

    Falsy values: NIL, 0, False.
    """

    JUMP_IF_TRUE = 0x42
    """Jump if top is truthy. Operand: target PC. value →"""

    # =====================================================================
    # Functions (0x5_)
    # =====================================================================

    MAKE_CLOSURE = 0x50
    """Create a closure from top-of-stack CodeObject. → closure_address

    Pops a CodeObject, captures the current environment, allocates a
    LispClosure on the GC heap, and pushes the heap address.
    """

    CALL_FUNCTION = 0x51
    """Call a function. Operand: argc. [func, arg1, ..., argN] → result

    Pops N arguments and the callable, executes the function, and
    pushes the return value.
    """

    TAIL_CALL = 0x52
    """Tail call optimization. Operand: argc. [func, arg1, ..., argN] → (continues)

    Like CALL_FUNCTION but reuses the current call frame instead of
    pushing a new one. This enables unbounded recursion for tail-recursive
    functions. The VM rebinds arguments in the existing local slots and
    resets the PC to 0.

    This is a GenericVM-level feature — any functional language compiler
    can emit TAIL_CALL when a function call is in tail position.
    """

    RETURN = 0x53
    """Return from a function. value → (to caller)"""

    # =====================================================================
    # Lisp-Specific Operations (0x7_)
    # =====================================================================

    CONS = 0x70
    """Create a cons cell. cdr car → address

    Pops car (top) and cdr (below), allocates a ConsCell on the GC heap,
    and pushes the heap address. Note: car is pushed last (on top).
    """

    CAR = 0x71
    """Get the first element of a cons cell. address → car_value"""

    CDR = 0x72
    """Get the second element of a cons cell. address → cdr_value"""

    MAKE_SYMBOL = 0x73
    """Intern a symbol. Operand: name index. → address

    Looks up the name in the constant pool, interns it via the SymbolTable,
    and pushes the heap address. Two references to 'foo get the same address.
    """

    IS_ATOM = 0x74
    """Test if value is an atom (not a cons cell). value → (1 or 0)

    Returns 1 if the value is NOT a cons cell address, 0 if it is.
    Numbers, symbols, strings, NIL are all atoms.
    """

    IS_NIL = 0x75
    """Test if value is NIL. value → (1 or 0)"""

    # =====================================================================
    # I/O (0xA_)
    # =====================================================================

    PRINT = 0xA0
    """Print the top of stack. value → (produces output)"""

    # =====================================================================
    # VM Control (0xF_)
    # =====================================================================

    HALT = 0xFF
    """Stop execution."""
