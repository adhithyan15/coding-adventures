"""Type definitions for the register-based virtual machine.

This module defines all shared data structures used by the VM.  Keeping
types separate from the execution engine (``vm.py``) avoids circular
imports and makes it easy to import just the type annotations from tests
or external tools.

Value representation
--------------------
In a real JS engine, values are NaN-boxed 64-bit words that pack the
type tag into the NaN bits of an IEEE-754 double.  We don't need that
efficiency here, so we use a plain Python ``Union`` type and let the
dispatch in ``vm.py`` branch on ``isinstance`` checks.

The special ``UNDEFINED`` sentinel mirrors JavaScript's ``undefined``
value (distinct from Python's ``None``, which maps to JS ``null``).

Hidden classes and inline caches
---------------------------------
Every ``VMObject`` carries a ``hidden_class_id`` integer that represents
its *shape* — the set of property names and their order.  When the VM
loads a named property, it records that class-id in a ``FeedbackSlot``.

Over time the slot transitions through a state machine:

    Uninitialized → Monomorphic (1 class) → Polymorphic (2–4 classes)
                                          → Megamorphic (5+ classes, generic)

In a production engine, monomorphic slots are compiled to a direct
memory offset load.  Here the feedback is for educational purposes only.

Call frames
-----------
The ``CallFrame`` dataclass captures everything needed to execute one
function activation:

* ``code``         — the ``CodeObject`` being executed
* ``ip``           — instruction pointer (index into ``code.instructions``)
* ``accumulator``  — the single implicit-result register
* ``registers``    — the explicit register file (size = ``code.register_count``)
* ``feedback_vector`` — one ``FeedbackSlot`` per inline-cache site
* ``context``      — the lexical context chain for closures
* ``caller_frame`` — the enclosing frame (``None`` at the top level)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass


# ---------------------------------------------------------------------------
# Undefined sentinel
# ---------------------------------------------------------------------------

class _Undefined:
    """Singleton sentinel representing JavaScript's ``undefined`` value.

    We use a dedicated class (rather than ``None``) so that code can
    distinguish between an unset variable (``undefined``) and an explicit
    ``null`` (Python's ``None``).

    The module-level constant ``UNDEFINED`` is the only instance you
    should ever use — do **not** create additional ``_Undefined()`` objects.
    """

    _instance: _Undefined | None = None

    def __new__(cls) -> _Undefined:
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __repr__(self) -> str:
        return "undefined"

    def __bool__(self) -> bool:
        # ``undefined`` is falsy in JavaScript.
        return False


UNDEFINED: _Undefined = _Undefined()
"""The unique ``undefined`` sentinel value.

Use ``is UNDEFINED`` to check for it — never compare with ``==``.
"""


# ---------------------------------------------------------------------------
# Value union
# ---------------------------------------------------------------------------

VMValue = int | float | str | bool | None | "_Undefined" | "VMObject" | list | "VMFunction"
"""The set of all possible values a register or context slot can hold.

Ordering of the union matters for isinstance checks in the VM:
``bool`` must be checked before ``int`` because ``bool`` is a subclass
of ``int`` in Python.
"""


# ---------------------------------------------------------------------------
# Object model
# ---------------------------------------------------------------------------

@dataclass
class VMObject:
    """A heap-allocated object with a fixed *shape* (hidden class).

    In a JIT-compiled engine, objects with the same ``hidden_class_id``
    have the same property layout, enabling offset-based property access.
    Here the class-id is simply an integer assigned by ``feedback.new_hidden_class_id()``.

    Attributes:
        hidden_class_id: Identifies the object's shape for inline caches.
        properties: A string-keyed dict of property values.  In a real
            engine this would be a fixed-layout array, but a dict is
            equivalent for our purposes.

    Example::

        obj = VMObject(hidden_class_id=0)
        obj.properties["x"] = 42
        obj.properties["y"] = "hello"
    """

    hidden_class_id: int
    properties: dict[str, VMValue] = field(default_factory=dict)


@dataclass
class VMFunction:
    """A first-class function value: a ``CodeObject`` plus a captured context.

    This is the runtime representation of a closure.  The ``code`` field
    holds the bytecode; ``context`` holds the lexical environment that was
    live when the closure was created.

    Example: compiling ``function add(a, b) { return a + b; }`` produces a
    ``VMFunction`` with a two-parameter ``CodeObject`` and a ``None`` context
    (no free variables to capture).
    """

    code: CodeObject
    context: Context | None


# ---------------------------------------------------------------------------
# Code objects
# ---------------------------------------------------------------------------

@dataclass
class CodeObject:
    """A compiled unit of bytecode — the static "program" for one function.

    Analogous to CPython's ``PyCodeObject`` or the JVM's ``Code`` attribute.

    Attributes:
        instructions:       Flat list of ``RegisterInstruction`` objects.
        constants:          Indexed constant pool (strings, numbers, nested
                            ``CodeObject`` instances for inner functions, etc.).
        names:              Indexed list of identifier strings (global names,
                            property names, etc.).
        register_count:     Number of slots in the register file.  Each call
                            frame allocates this many registers.
        feedback_slot_count: Number of inline-cache slots.  Each call frame
                            allocates this many ``FeedbackSlot`` entries.
        parameter_count:    Number of formal parameters.
        name:               Human-readable name for traces and error messages.

    Example — bytecode for ``return 1 + 2``::

        CodeObject(
            instructions=[
                RegisterInstruction(Opcode.LDA_CONSTANT, [0]),  # acc = 1
                RegisterInstruction(Opcode.STAR, [0]),          # r0 = acc
                RegisterInstruction(Opcode.LDA_CONSTANT, [1]),  # acc = 2
                RegisterInstruction(Opcode.ADD, [0]),           # acc = acc + r0
                RegisterInstruction(Opcode.RETURN),
            ],
            constants=[1, 2],
            names=[],
            register_count=1,
            feedback_slot_count=0,
        )
    """

    instructions: list[RegisterInstruction]
    constants: list[VMValue]
    names: list[str]
    register_count: int
    feedback_slot_count: int
    parameter_count: int = 0
    name: str = "anonymous"


@dataclass
class RegisterInstruction:
    """A single bytecode instruction.

    An instruction consists of an opcode (from ``Opcode``) and a list of
    integer operands whose meaning depends on the opcode.

    Attributes:
        opcode:        The operation to perform (``int`` or ``Opcode`` enum).
        operands:      Variable-length list of integer operands.
        feedback_slot: Optional index into the frame's feedback vector.
                       Stored here as a convenience so opcodes that
                       don't encode feedback in ``operands`` can still
                       record IC data.

    The ``opcode`` field accepts either an ``Opcode`` enum value or a plain
    ``int`` for cases where the compiler emits raw integers.  The VM always
    compares with ``match`` on the ``int`` value.
    """

    opcode: int
    operands: list[int] = field(default_factory=list)
    feedback_slot: int | None = None


# ---------------------------------------------------------------------------
# Call frame
# ---------------------------------------------------------------------------

@dataclass
class CallFrame:
    """Runtime state for a single function activation.

    A new ``CallFrame`` is pushed onto the implicit call stack (via Python's
    own call stack, since we recurse into ``_run_frame``) for each function
    call.  When the function returns, the frame is discarded and control
    returns to the caller.

    Attributes:
        code:            The ``CodeObject`` being executed.
        ip:              The instruction pointer — index of the *next*
                         instruction to execute.
        accumulator:     The single hidden accumulator register.  Most
                         operations read their left operand from here and
                         write their result back here.
        registers:       The explicit register file.  Indexed by operand.
        feedback_vector: Inline-cache state for each IC site in this function.
        context:         Current lexical context (for closure variable access).
        caller_frame:    The frame that initiated this call (or ``None`` at
                         the top level).
    """

    code: CodeObject
    ip: int = 0
    accumulator: VMValue = field(default_factory=lambda: UNDEFINED)
    registers: list[VMValue] = field(default_factory=list)
    feedback_vector: list[FeedbackSlot] = field(default_factory=list)
    context: Context | None = None
    caller_frame: CallFrame | None = None


# ---------------------------------------------------------------------------
# Feedback slots — discriminated union via tagged dataclasses
# ---------------------------------------------------------------------------

@dataclass
class SlotUninitialized:
    """The inline-cache site has never been reached.

    A JIT compiler would emit a call to the *slow path* and then update
    this slot after the first execution.
    """

    kind: str = "uninitialized"


@dataclass
class SlotMonomorphic:
    """The site has seen exactly one type combination.

    ``types`` is a list of ``(lhs_type, rhs_type)`` pairs, but in the
    monomorphic state it always has exactly one element.  A JIT can emit
    a fast-path check for that specific type pair.
    """

    kind: str = "monomorphic"
    types: list[tuple[str, str]] = field(default_factory=list)


@dataclass
class SlotPolymorphic:
    """The site has seen 2–4 distinct type combinations.

    A JIT emits a short linear scan through the known type pairs before
    falling back to the slow path.
    """

    kind: str = "polymorphic"
    types: list[tuple[str, str]] = field(default_factory=list)


@dataclass
class SlotMegamorphic:
    """The site has seen 5+ distinct type combinations.

    The inline cache is abandoned; the VM always takes the slow path.
    Profiling still happens but cannot be optimized further.
    """

    kind: str = "megamorphic"


FeedbackSlot = SlotUninitialized | SlotMonomorphic | SlotPolymorphic | SlotMegamorphic
"""A single inline-cache entry.

State machine diagram::

    Uninitialized ─── first seen ──→ Monomorphic
    Monomorphic   ─── new type ───→ Polymorphic
    Polymorphic   ─── 5th type ───→ Megamorphic
    Megamorphic   ─── (terminal) ──→ Megamorphic
"""

TypePair = tuple[str, str]
"""A ``(lhs_type_name, rhs_type_name)`` pair recorded in binary op feedback."""


# ---------------------------------------------------------------------------
# Results
# ---------------------------------------------------------------------------

@dataclass
class VMResult:
    """The final result of executing a ``CodeObject``.

    Attributes:
        return_value: The value in the accumulator when execution halted.
        output:       Lines printed via the built-in ``print`` global.
        error:        ``None`` on success; a ``VMError`` on failure.
    """

    return_value: VMValue
    output: list[str]
    error: VMError | None


@dataclass
class VMError(Exception):
    """A runtime error raised by the VM.

    Attributes:
        message:           Human-readable description.
        instruction_index: The ``ip`` value at the time of the error.
        opcode:            The opcode that triggered the error.
    """

    message: str
    instruction_index: int
    opcode: int

    def __str__(self) -> str:
        return f"VMError at ip={self.instruction_index}: {self.message}"


# ---------------------------------------------------------------------------
# Lexical context
# ---------------------------------------------------------------------------

@dataclass
class Context:
    """A lexical scope in the closure chain.

    Contexts form a singly-linked list from innermost to outermost scope.
    Each context holds a flat array of ``VMValue`` slots.

    Example — two-level closure::

        outer_ctx = Context(slots=[10, 20])  # depth 0 from outer function
        inner_ctx = Context(slots=[99], parent=outer_ctx)  # depth 0 from inner

        # inner function reads outer variable at depth=1, index=0 → 10
        get_slot(inner_ctx, depth=1, index=0)  # → 10
    """

    slots: list[VMValue]
    parent: Context | None = None


# ---------------------------------------------------------------------------
# Execution trace
# ---------------------------------------------------------------------------

@dataclass
class TraceStep:
    """A snapshot of VM state immediately before and after one instruction.

    Used by ``execute_with_trace`` to produce a human-readable execution
    log for debugging, education, and visualization tools.

    Attributes:
        frame_depth:       Call-stack depth (0 = outermost function).
        ip:                The instruction pointer *before* execution.
        instruction:       The instruction that executed.
        acc_before:        Accumulator value before the instruction.
        acc_after:         Accumulator value after the instruction.
        registers_before:  Shallow copy of the register file before.
        registers_after:   Shallow copy of the register file after.

    Example trace for ``LDA_CONSTANT 0; STAR 0; HALT``::

        TraceStep(frame_depth=0, ip=0, instruction=LDA_CONSTANT[0],
                  acc_before=undefined, acc_after=42,
                  registers_before=[undefined], registers_after=[undefined])
        TraceStep(frame_depth=0, ip=1, instruction=STAR[0],
                  acc_before=42, acc_after=42,
                  registers_before=[undefined], registers_after=[42])
        TraceStep(frame_depth=0, ip=2, instruction=HALT[],
                  acc_before=42, acc_after=42,
                  registers_before=[42], registers_after=[42])
    """

    frame_depth: int
    ip: int
    instruction: RegisterInstruction
    acc_before: VMValue
    acc_after: VMValue
    registers_before: list[VMValue]
    registers_after: list[VMValue]
