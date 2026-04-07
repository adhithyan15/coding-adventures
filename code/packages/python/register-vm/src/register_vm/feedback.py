"""Inline-cache feedback tracking for the register VM.

This module implements the *type-feedback vector* used by V8's Ignition
interpreter to record what types flow through each operation site.  The
information drives adaptive optimization: a JIT compiler can inspect the
vector and emit specialized machine code for the most common type
combinations.

How feedback works
------------------
Each ``CodeObject`` declares a ``feedback_slot_count``.  When a call frame
is created, the VM allocates a list of ``FeedbackSlot`` entries, one per
slot.  Each slot starts as ``SlotUninitialized``.

As the interpreter executes, it calls ``record_binary_op``,
``record_property_load``, or ``record_call_site`` to update the relevant
slot.  The slot transitions through a *state machine*:

    Uninitialized ──→ Monomorphic (1 shape)
    Monomorphic   ──→ Polymorphic (2–4 shapes)
    Polymorphic   ──→ Megamorphic (5+ shapes — give up)
    Megamorphic   ──→ Megamorphic (terminal)

The threshold of 4 shapes before going megamorphic matches V8 Ignition's
default ``--max-inlined-source-size`` heuristic.

Hidden class IDs
----------------
Every ``VMObject`` has a monotonically increasing ``hidden_class_id`` assigned
by ``new_hidden_class_id()``.  Two objects with the same id have been through
the same sequence of property additions and therefore have the same layout.
The property-load feedback records the *class id* (not the property name)
so the optimizer can emit ``object.properties["x"]`` as a direct array-offset
load once it knows the object is always of class 3.

Thread safety
-------------
The ``_next_hidden_class_id`` counter is module-level state.  In a concurrent
interpreter you would protect it with a lock or use an atomic increment.
For this single-threaded educational implementation we leave it as a plain
global.
"""

from register_vm.types import (
    UNDEFINED,
    FeedbackSlot,
    SlotMegamorphic,
    SlotMonomorphic,
    SlotPolymorphic,
    SlotUninitialized,
    TypePair,
    VMFunction,
    VMObject,
    VMValue,
)

# Module-level counter for assigning unique hidden class IDs.
_next_hidden_class_id: int = 0


def new_hidden_class_id() -> int:
    """Allocate and return the next monotonically increasing hidden class ID.

    Each call returns a unique integer.  IDs are never reused within a
    process lifetime, so two objects with the same ID are guaranteed to
    have been created with the same shape sequence.

    Returns:
        A non-negative integer unique to this allocation.

    Example::

        id_a = new_hidden_class_id()  # 0
        id_b = new_hidden_class_id()  # 1
        assert id_a != id_b
    """
    global _next_hidden_class_id
    result = _next_hidden_class_id
    _next_hidden_class_id += 1
    return result


def new_vector(size: int) -> list[FeedbackSlot]:
    """Create a fresh feedback vector of ``size`` slots, all uninitialized.

    Called by the VM when creating a new call frame.

    Args:
        size: Number of inline-cache sites in this function.

    Returns:
        A list of ``SlotUninitialized`` entries.

    Example::

        vec = new_vector(3)
        assert len(vec) == 3
        assert all(isinstance(s, SlotUninitialized) for s in vec)
    """
    return [SlotUninitialized() for _ in range(size)]


def value_type(v: VMValue) -> str:
    """Return a JavaScript-style type name for a VM value.

    The mapping mirrors ``typeof`` in JavaScript, which is important for
    generating meaningful feedback that matches what a JS JIT would see.

    Mapping table:

        ┌──────────────┬─────────────┐
        │  Python type  │  JS typeof  │
        ├──────────────┼─────────────┤
        │  bool         │ "boolean"   │
        │  int / float  │ "number"    │
        │  str          │ "string"    │
        │  None         │ "null"      │
        │  _Undefined   │ "undefined" │
        │  VMObject     │ "object"    │
        │  list         │ "array"     │
        │  VMFunction   │ "function"  │
        │  (other)      │ "unknown"   │
        └──────────────┴─────────────┘

    Note: ``bool`` must be checked before ``int`` because ``bool`` is a
    subclass of ``int`` in Python — ``isinstance(True, int)`` is ``True``.

    Args:
        v: Any VM value.

    Returns:
        A type-name string.

    Examples::

        value_type(42)        # "number"
        value_type(3.14)      # "number"
        value_type("hello")   # "string"
        value_type(True)      # "boolean"
        value_type(None)      # "null"
        value_type(UNDEFINED) # "undefined"
    """
    if isinstance(v, bool):
        return "boolean"
    if isinstance(v, (int, float)):
        return "number"
    if isinstance(v, str):
        return "string"
    if v is None:
        return "null"
    if v is UNDEFINED:
        return "undefined"
    if isinstance(v, VMObject):
        return "object"
    if isinstance(v, list):
        return "array"
    if isinstance(v, VMFunction):
        return "function"
    return "unknown"


def record_binary_op(
    vector: list[FeedbackSlot],
    slot: int,
    left: VMValue,
    right: VMValue,
) -> None:
    """Record a binary operation type pair into the feedback vector.

    Determines the type names of ``left`` and ``right``, then calls
    ``_update_slot`` to advance the slot's state machine.

    Args:
        vector: The frame's feedback vector.
        slot:   Index of the IC slot for this binary operation.
        left:   Left operand value (usually the accumulator).
        right:  Right operand value (from a register).

    Example::

        vec = new_vector(1)
        record_binary_op(vec, 0, 5, 3)
        assert isinstance(vec[0], SlotMonomorphic)
        assert vec[0].types == [("number", "number")]
    """
    if slot < 0 or slot >= len(vector):
        return
    pair: TypePair = (value_type(left), value_type(right))
    vector[slot] = _update_slot(vector[slot], pair)


def record_property_load(
    vector: list[FeedbackSlot],
    slot: int,
    hidden_class_id: int,
) -> None:
    """Record a named-property load using the object's hidden class ID.

    Instead of a ``(lhs, rhs)`` pair, property loads record the *shape*
    of the receiver object.  We encode this as the pair
    ``("object_<id>", "property")``.

    Args:
        vector:          The frame's feedback vector.
        slot:            Index of the IC slot.
        hidden_class_id: The ``hidden_class_id`` of the receiver ``VMObject``.

    Example::

        vec = new_vector(1)
        record_property_load(vec, 0, hidden_class_id=7)
        assert vec[0].types == [("object_7", "property")]
    """
    if slot < 0 or slot >= len(vector):
        return
    pair: TypePair = (f"object_{hidden_class_id}", "property")
    vector[slot] = _update_slot(vector[slot], pair)


def record_call_site(
    vector: list[FeedbackSlot],
    slot: int,
    callee_type: str,
) -> None:
    """Record what kind of function was called at a call site.

    The pair is ``(callee_type, "call")`` — the left element describes
    the callee (e.g. ``"function"``, ``"builtin"``) and the right is
    always the literal string ``"call"`` to distinguish call feedback
    from binary-op feedback when inspecting the vector.

    Args:
        vector:       The frame's feedback vector.
        slot:         Index of the IC slot.
        callee_type:  Type name of the callee (``"function"`` or ``"builtin"``).

    Example::

        vec = new_vector(1)
        record_call_site(vec, 0, "function")
        assert vec[0].types == [("function", "call")]
    """
    if slot < 0 or slot >= len(vector):
        return
    pair: TypePair = (callee_type, "call")
    vector[slot] = _update_slot(vector[slot], pair)


def _update_slot(current: FeedbackSlot, pair: TypePair) -> FeedbackSlot:
    """Advance a feedback slot's state machine given a new type pair.

    This is the core of the inline-cache recording logic.  The state
    machine enforces a monotonic progression — slots never go *backwards*
    (e.g. from Polymorphic to Monomorphic).

    State machine transitions:

        Uninitialized        → Monomorphic([pair])
        Monomorphic([pair])  → Monomorphic([pair])   (same pair → no-op)
        Monomorphic([p])     → Polymorphic([pair, p]) (different pair)
        Polymorphic(ps)      → Polymorphic([pair, *ps])  if len(ps) < 4
        Polymorphic(ps)      → Megamorphic()             if len(ps) >= 4
        Megamorphic          → Megamorphic               (terminal)

    The deduplication check in the Monomorphic and Polymorphic cases
    prevents the slot from inflating when the same type pair repeats
    many times (e.g. inside a tight loop).

    Args:
        current: The current slot state.
        pair:    The new ``(lhs_type, rhs_type)`` pair observed.

    Returns:
        The updated slot (may be the same object if no transition occurred).

    Examples::

        s = SlotUninitialized()
        s = _update_slot(s, ("number", "number"))
        # → SlotMonomorphic(types=[("number", "number")])

        s = _update_slot(s, ("number", "number"))
        # → SlotMonomorphic(types=[("number", "number")])  (unchanged)

        s = _update_slot(s, ("string", "number"))
        # → SlotPolymorphic(types=[("string","number"), ("number","number")])
    """
    match current:
        case SlotUninitialized():
            return SlotMonomorphic(types=[pair])

        case SlotMonomorphic(types=existing_types):
            if pair in existing_types:
                # Deduplicate: same type pair again, no state change.
                return current
            # New type pair — promote to polymorphic.
            return SlotPolymorphic(types=[pair, *existing_types])

        case SlotPolymorphic(types=existing_types):
            if pair in existing_types:
                # Already tracking this pair.
                return current
            if len(existing_types) >= 4:
                # Exceeded the polymorphic threshold — go megamorphic.
                return SlotMegamorphic()
            return SlotPolymorphic(types=[pair, *existing_types])

        case SlotMegamorphic():
            # Terminal state — no further transitions possible.
            return current

        case _:
            # Defensive fallback for unexpected slot types.
            return current
