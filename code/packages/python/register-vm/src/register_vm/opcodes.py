"""Opcode definitions for the register-based virtual machine.

Opcodes are organized into logical groups by their first nibble, mirroring
the structure of V8's Ignition bytecode handler table. This grouping makes
the instruction set easy to navigate and extends cleanly.

  0x0_ — Accumulator loads (literals and sentinels)
  0x1_ — Register moves (accumulator ↔ register, register ↔ register)
  0x2_ — Variable access (globals, locals, context slots)
  0x3_ — Arithmetic and bitwise operations
  0x4_ — Comparison and type-test operations
  0x5_ — Control flow (jumps, loops)
  0x6_ — Calls and function-related control
  0x7_ — Property access (named and keyed)
  0x8_ — Object / array / closure creation
  0x9_ — Iteration protocol
  0xA_ — Exception handling
  0xB_ — Context and module variable access
  0xF_ — VM meta-instructions (stack check, debugger, halt)

Each opcode is an ``IntEnum`` value so it can be compared directly to integer
operands stored in ``RegisterInstruction.opcode``, and can still be printed
as a human-readable name in traces and error messages.

Example usage::

    from register_vm.opcodes import Opcode

    instr = RegisterInstruction(opcode=Opcode.ADD, operands=[2])
    print(instr.opcode)  # Opcode.ADD
    print(instr.opcode == 0x30)  # True — IntEnum compares by value
"""

from enum import IntEnum


class Opcode(IntEnum):
    """All register VM opcodes organized by functional category.

    The numeric values are chosen to match the logical grouping: each group
    occupies one "row" in the 0x00–0xFF space, giving room to add opcodes
    within a group without renumbering existing ones.

    Arithmetic truth table (for ``ADD`` with two integer operands):

        ┌──────┬──────┬────────────┐
        │  a   │  b   │  a + b     │
        ├──────┼──────┼────────────┤
        │  3   │  2   │  5         │
        │ "hi" │  "!" │ "hi!"      │
        │ "hi" │  2   │ "hi2"      │
        │  1.5 │  1.5 │  3.0       │
        └──────┴──────┴────────────┘

    String concatenation follows JavaScript semantics: if *either* operand is
    a string, both are coerced to strings and concatenated.
    """

    # ------------------------------------------------------------------
    # 0x0_ Accumulator loads — put a value directly into the accumulator
    # without touching any register.
    # ------------------------------------------------------------------

    LDA_CONSTANT = 0x00
    """Load constant[operands[0]] into the accumulator.

    The ``constants`` list in ``CodeObject`` holds arbitrary Python values.
    This is the primary way to push literals (numbers, strings, booleans) into
    the computation path.

    Example bytecode for ``x = 42``:
        LDA_CONSTANT 0  ; constants[0] = 42
        STAR 0          ; registers[0] = acc
    """

    LDA_ZERO = 0x01
    """Load the integer 0 into the accumulator.

    A common special case that avoids a constant-pool entry.
    Equivalent to ``LDA_CONSTANT`` with a constant of ``0``.
    """

    LDA_SMI = 0x02
    """Load a small integer (SMI) into the accumulator.

    The integer value is encoded directly in ``operands[0]``, which
    avoids a constant-pool lookup for small literal integers.
    V8 uses the range [-32768, 32767]; we use the full Python int range.
    """

    LDA_UNDEFINED = 0x03
    """Load the ``UNDEFINED`` sentinel into the accumulator.

    Represents JavaScript's ``undefined`` — a distinct value from ``None``
    (which maps to ``null``).
    """

    LDA_NULL = 0x04
    """Load ``None`` (null) into the accumulator."""

    LDA_TRUE = 0x05
    """Load ``True`` into the accumulator."""

    LDA_FALSE = 0x06
    """Load ``False`` into the accumulator."""

    # ------------------------------------------------------------------
    # 0x1_ Register moves
    # ------------------------------------------------------------------

    LDAR = 0x10
    """Load a register into the accumulator.

    ``operands[0]`` is the source register index.
    After execution: ``acc = registers[operands[0]]``.
    """

    STAR = 0x11
    """Store the accumulator into a register.

    ``operands[0]`` is the destination register index.
    After execution: ``registers[operands[0]] = acc``.
    """

    MOV = 0x12
    """Copy one register to another.

    ``operands = [src, dst]``.
    After execution: ``registers[dst] = registers[src]``.
    The accumulator is not touched.
    """

    # ------------------------------------------------------------------
    # 0x2_ Variable access — globals, locals, and lexical context slots
    # ------------------------------------------------------------------

    LDA_GLOBAL = 0x20
    """Load a global variable into the accumulator.

    ``operands[0]`` is an index into ``CodeObject.names``.
    Raises ``VMError`` if the name is not found in the globals dict.
    """

    STA_GLOBAL = 0x21
    """Store the accumulator into a global variable.

    ``operands[0]`` is an index into ``CodeObject.names``.
    """

    LDA_LOCAL = 0x22
    """Load a local (register-file) variable into the accumulator.

    Alias for ``LDAR`` — provided for readability in compiler output.
    """

    STA_LOCAL = 0x23
    """Store the accumulator to a local (register-file) variable.

    Alias for ``STAR`` — provided for readability in compiler output.
    """

    LDA_CONTEXT_SLOT = 0x24
    """Load a value from a context slot.

    ``operands = [depth, index]``.
    Walks ``depth`` parent links up the context chain, then reads slot
    ``index``.  Used to implement closures and block scoping.
    """

    STA_CONTEXT_SLOT = 0x25
    """Store the accumulator to a context slot.

    ``operands = [depth, index]``.
    """

    LDA_CURRENT_CONTEXT_SLOT = 0x26
    """Load a value from the current (depth-0) context.

    ``operands[0]`` is the slot index.  Faster than ``LDA_CONTEXT_SLOT``
    with ``depth=0`` because no parent-chain walk is needed.
    """

    STA_CURRENT_CONTEXT_SLOT = 0x27
    """Store the accumulator to the current context.

    ``operands[0]`` is the slot index.
    """

    # ------------------------------------------------------------------
    # 0x3_ Arithmetic and bitwise operations
    # All binary ops read the *left* operand from the accumulator and the
    # *right* operand from ``registers[operands[0]]``.
    # The result is written back to the accumulator.
    # ------------------------------------------------------------------

    ADD = 0x30
    """Add the accumulator and a register.

    Semantics:
    - Both numeric (int or float) → numeric addition.
    - Either operand is a string → string concatenation (coerce both).
    - Otherwise → ``VMError``.

    Records type feedback in ``operands[1]`` (if present) for an
    optimizer to later specialize on monomorphic int + int calls.
    """

    SUB = 0x31
    """Subtract register from accumulator (``acc = acc - reg``)."""

    MUL = 0x32
    """Multiply accumulator by register (``acc = acc * reg``)."""

    DIV = 0x33
    """Divide accumulator by register (``acc = acc / reg``).

    Integer division produces a float, matching Python's ``/``.
    Division by zero raises ``VMError``.
    """

    MOD = 0x34
    """Modulo: ``acc = acc % reg``.

    Zero modulus raises ``VMError``.
    """

    POW = 0x35
    """Exponentiation: ``acc = acc ** reg``."""

    ADD_SMI = 0x36
    """Add a small integer literal to the accumulator.

    ``operands[0]`` is the integer to add (no register lookup).
    Equivalent to ``LDA_SMI n; ADD reg`` but faster for common ``i += 1``
    patterns in loops.
    """

    SUB_SMI = 0x37
    """Subtract a small integer literal from the accumulator.

    ``operands[0]`` is the integer to subtract.
    """

    BITWISE_AND = 0x38
    """Bitwise AND: ``acc = int(acc) & int(reg)``."""

    BITWISE_OR = 0x39
    """Bitwise OR: ``acc = int(acc) | int(reg)``."""

    BITWISE_XOR = 0x3A
    """Bitwise XOR: ``acc = int(acc) ^ int(reg)``."""

    BITWISE_NOT = 0x3B
    """Bitwise NOT: ``acc = ~int(acc)`` (unary, no register operand)."""

    SHIFT_LEFT = 0x3C
    """Left shift: ``acc = int(acc) << int(reg)``."""

    SHIFT_RIGHT = 0x3D
    """Arithmetic right shift: ``acc = int(acc) >> int(reg)``."""

    SHIFT_RIGHT_LOGICAL = 0x3E
    """Logical (unsigned) right shift.

    Python integers are arbitrary precision and don't have a natural unsigned
    interpretation, so we mask to 32 bits before shifting:
    ``acc = (int(acc) & 0xFFFFFFFF) >> int(reg)``.
    """

    NEGATE = 0x3F
    """Arithmetic negation: ``acc = -acc`` (unary, no register operand)."""

    # ------------------------------------------------------------------
    # 0x4_ Comparisons — result is always a Python bool in the accumulator
    # ------------------------------------------------------------------

    TEST_EQUAL = 0x40
    """Loose equality (``==``): ``acc = (acc == reg)``."""

    TEST_NOT_EQUAL = 0x41
    """Loose inequality (``!=``): ``acc = (acc != reg)``."""

    TEST_STRICT_EQUAL = 0x42
    """Strict equality — same type AND value.

    Unlike ``TEST_EQUAL``, this does *not* coerce types.  For our Python
    implementation the distinction matters between int ``0`` and ``False``
    (which Python considers ``==`` but not strictly equal in JS semantics).
    We implement it as identity check first, then same-type value check.
    """

    TEST_STRICT_NOT_EQUAL = 0x43
    """Strict inequality: ``acc = not (acc is reg or (type(acc) is type(reg) and acc == reg))``."""

    TEST_LESS_THAN = 0x44
    """Less-than comparison: ``acc = (acc < reg)``."""

    TEST_GREATER_THAN = 0x45
    """Greater-than comparison: ``acc = (acc > reg)``."""

    TEST_LESS_THAN_OR_EQUAL = 0x46
    """Less-than-or-equal: ``acc = (acc <= reg)``."""

    TEST_GREATER_THAN_OR_EQUAL = 0x47
    """Greater-than-or-equal: ``acc = (acc >= reg)``."""

    TEST_IN = 0x48
    """Membership test: ``acc = (acc in reg)``.

    ``reg`` must be an object (dict), list, or string.
    """

    TEST_INSTANCEOF = 0x49
    """Type check (simplified): ``acc = isinstance(acc, type(reg))``."""

    TEST_UNDETECTABLE = 0x4A
    """True if value is ``None`` or ``UNDEFINED``: ``acc = (acc is None or acc is UNDEFINED)``."""

    LOGICAL_NOT = 0x4B
    """Boolean NOT: ``acc = not bool(acc)``."""

    TYPEOF = 0x4C
    """Return the type name of the accumulator as a string.

    Mapping (mirrors JavaScript ``typeof``):
        int / float  → ``"number"``
        str          → ``"string"``
        bool         → ``"boolean"``
        None         → ``"null"``
        UNDEFINED    → ``"undefined"``
        VMObject     → ``"object"``
        list         → ``"array"``
        VMFunction   → ``"function"``
    """

    # ------------------------------------------------------------------
    # 0x5_ Control flow
    # Jumps use a *relative* offset in ``operands[0]``.
    # Positive offsets jump forward; negative offsets loop back.
    # The offset is applied *after* the IP has already advanced past the
    # current instruction, so ``JUMP 0`` is a no-op.
    # ------------------------------------------------------------------

    JUMP = 0x50
    """Unconditional relative jump: ``ip += operands[0]``."""

    JUMP_IF_TRUE = 0x51
    """Jump if accumulator is truthy: ``if bool(acc): ip += operands[0]``."""

    JUMP_IF_FALSE = 0x52
    """Jump if accumulator is falsy: ``if not bool(acc): ip += operands[0]``."""

    JUMP_IF_NULL = 0x53
    """Jump if accumulator is ``None``."""

    JUMP_IF_UNDEFINED = 0x54
    """Jump if accumulator is ``UNDEFINED``."""

    JUMP_IF_NULL_OR_UNDEFINED = 0x55
    """Jump if accumulator is ``None`` or ``UNDEFINED``."""

    JUMP_IF_TO_BOOLEAN_TRUE = 0x56
    """Jump if ``bool(acc)`` would be ``True`` (same as ``JUMP_IF_TRUE`` in this VM)."""

    JUMP_IF_TO_BOOLEAN_FALSE = 0x57
    """Jump if ``bool(acc)`` would be ``False``."""

    JUMP_LOOP = 0x58
    """Backward jump for loop bodies.

    Semantically identical to ``JUMP`` but with a negative offset.
    Distinguished so the VM can count loop back-edges for future
    on-stack-replacement (OSR) optimizations.
    """

    # ------------------------------------------------------------------
    # 0x6_ Calls and function-related control
    # ------------------------------------------------------------------

    CALL_ANY_RECEIVER = 0x60
    """Call a function with any receiver.

    ``operands = [callable_reg, first_arg_reg, argc, feedback_slot]``.

    The function object is read from ``registers[callable_reg]``.
    Arguments are ``registers[first_arg_reg .. first_arg_reg + argc - 1]``.
    The return value lands in the accumulator.
    """

    CALL_PROPERTY = 0x61
    """Call a method on an object (receiver is passed explicitly).

    ``operands = [callable_reg, receiver_reg, first_arg_reg, argc, feedback_slot]``.
    """

    CALL_UNDEFINED_RECEIVER = 0x62
    """Call with ``undefined`` as the implicit receiver.

    ``operands = [callable_reg, first_arg_reg, argc, feedback_slot]``.
    """

    CONSTRUCT = 0x63
    """Invoke a constructor (``new`` expression).

    Creates a new ``VMObject`` and passes it as the receiver.
    The return value replaces the new object if the constructor returns
    an object; otherwise the new object is used.
    """

    CONSTRUCT_WITH_SPREAD = 0x64
    """Construct with a spread argument (not fully implemented — raises VMError)."""

    CALL_WITH_SPREAD = 0x65
    """Call with a spread argument (not fully implemented — raises VMError)."""

    RETURN = 0x66
    """Return from the current call frame.

    The accumulator holds the return value.  This causes ``_run_frame``
    to exit and return the accumulator to the caller.
    """

    SUSPEND_GENERATOR = 0x67
    """Suspend a generator (not implemented — raises VMError)."""

    RESUME_GENERATOR = 0x68
    """Resume a generator (not implemented — raises VMError)."""

    # ------------------------------------------------------------------
    # 0x7_ Property access
    # ------------------------------------------------------------------

    LDA_NAMED_PROPERTY = 0x70
    """Load a named property from an object into the accumulator.

    ``operands = [obj_reg, name_idx, feedback_slot]``.
    Looks up ``CodeObject.names[name_idx]`` in the object's ``properties``
    dict.  Records the object's ``hidden_class_id`` in the feedback slot to
    enable inline-cache (IC) optimization.
    """

    STA_NAMED_PROPERTY = 0x71
    """Store the accumulator to a named property on an object.

    ``operands = [obj_reg, name_idx, feedback_slot]``.
    """

    LDA_KEYED_PROPERTY = 0x72
    """Load a keyed property: ``acc = obj[key]``.

    ``operands = [obj_reg, key_reg, feedback_slot]``.
    Supports dict lookup (by string key) and list indexing (by int).
    """

    STA_KEYED_PROPERTY = 0x73
    """Store the accumulator to a keyed property: ``obj[key] = acc``.

    ``operands = [obj_reg, key_reg, feedback_slot]``.
    """

    LDA_NAMED_PROPERTY_NO_FEEDBACK = 0x74
    """Load a named property without recording feedback.

    ``operands = [obj_reg, name_idx]``.
    Used in cold paths or for properties on non-tracked objects.
    """

    STA_NAMED_PROPERTY_NO_FEEDBACK = 0x75
    """Store a named property without recording feedback.

    ``operands = [obj_reg, name_idx]``.
    """

    DELETE_PROPERTY_STRICT = 0x76
    """Delete a property from an object (strict mode).

    ``operands = [obj_reg, key_reg]``.
    """

    DELETE_PROPERTY_SLOPPY = 0x77
    """Delete a property from an object (sloppy mode, same behavior here)."""

    # ------------------------------------------------------------------
    # 0x8_ Object / array / closure / context creation
    # ------------------------------------------------------------------

    CREATE_OBJECT_LITERAL = 0x80
    """Create an empty ``VMObject`` and place it in the accumulator."""

    CREATE_ARRAY_LITERAL = 0x81
    """Create an empty list and place it in the accumulator."""

    CREATE_REGEXP_LITERAL = 0x82
    """Create a regexp literal (returns the pattern string for now)."""

    CREATE_CLOSURE = 0x83
    """Create a ``VMFunction`` from a ``CodeObject`` in the constant pool.

    ``operands[0]`` is the constant index for the inner ``CodeObject``.
    The new function captures the current frame's context.
    """

    CREATE_CONTEXT = 0x84
    """Push a new lexical context onto the context chain.

    ``operands[0]`` is the number of slots in the new context.
    The current context becomes the parent.
    """

    CLONE_OBJECT = 0x85
    """Shallow-clone a ``VMObject``: ``acc = copy.copy(acc)``."""

    # ------------------------------------------------------------------
    # 0x9_ Iteration protocol
    # ------------------------------------------------------------------

    GET_ITERATOR = 0x90
    """Get an iterator from an iterable.

    Stores an iterator object in the accumulator.
    For lists / dicts, this wraps the Python iterator in a ``VMObject``.
    """

    CALL_ITERATOR_STEP = 0x91
    """Advance the iterator.

    After this instruction the accumulator holds a ``VMObject`` with
    ``done`` and ``value`` properties (like a JS iterator result).
    """

    GET_ITERATOR_DONE = 0x92
    """Load the ``done`` property of the current iterator result.

    Shorthand for ``LDA_NAMED_PROPERTY_NO_FEEDBACK`` on the result object.
    """

    GET_ITERATOR_VALUE = 0x93
    """Load the ``value`` property of the current iterator result."""

    # ------------------------------------------------------------------
    # 0xA_ Exception handling
    # ------------------------------------------------------------------

    THROW = 0xA0
    """Raise a ``VMError`` with the accumulator's value as the message.

    If the accumulator is a string, it is used as the error message.
    Otherwise the string representation is used.
    """

    RETHROW = 0xA1
    """Re-raise the most recently caught exception (not implemented — raises VMError)."""

    # ------------------------------------------------------------------
    # 0xB_ Context and module variable access
    # ------------------------------------------------------------------

    PUSH_CONTEXT = 0xB0
    """Push a new context (alias for ``CREATE_CONTEXT`` for clarity).

    ``operands[0]`` is the number of slots.
    """

    POP_CONTEXT = 0xB1
    """Pop the current context, restoring the parent.

    Used when leaving a block scope.
    """

    LDA_MODULE_VARIABLE = 0xB4
    """Load a module-level variable (treated as global in this VM).

    ``operands[0]`` is the name index.
    """

    STA_MODULE_VARIABLE = 0xB5
    """Store the accumulator to a module-level variable.

    ``operands[0]`` is the name index.
    """

    # ------------------------------------------------------------------
    # 0xF_ VM meta-instructions
    # ------------------------------------------------------------------

    STACK_CHECK = 0xF0
    """Check the call-stack depth and raise ``VMError`` if it exceeds ``max_depth``.

    Placed at the entry of each function by the compiler (or manually in
    test bytecode) to guard against runaway recursion.
    """

    DEBUGGER = 0xF1
    """Breakpoint hint — currently a no-op in this implementation.

    A real VM would transfer control to the debugger here.
    """

    HALT = 0xFF
    """Terminate the VM unconditionally and return the accumulator.

    If execution reaches the end of a ``CodeObject`` without an explicit
    ``RETURN`` or ``HALT``, the VM implicitly halts.
    """
