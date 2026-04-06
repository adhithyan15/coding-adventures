"""Lexical scope management for the register VM.

Contexts form a singly-linked chain from innermost to outermost scope.
Each context holds a flat list of ``VMValue`` slots — conceptually an
array of "closed-over" variables for a particular function activation.

Why a separate module?
-----------------------
Separating scope logic from the VM dispatch loop keeps each file
focused and testable.  The scope API is a pure data-structure
manipulation (no VM state dependencies), so it can be unit-tested
without spinning up a full VM.

Context chain diagram for a three-level closure::

    ┌──────────────────────────┐
    │  inner_ctx  (depth 0)    │  ← frame.context always points here
    │  slots: [99]             │
    │  parent ─────────────────┼──→ ┌──────────────────────────┐
    └──────────────────────────┘    │  middle_ctx  (depth 1)   │
                                    │  slots: [10, 20]         │
                                    │  parent ─────────────────┼──→ ...
                                    └──────────────────────────┘

To read ``middle_ctx.slots[1]`` from inside the innermost function::

    get_slot(inner_ctx, depth=1, index=1)  # → 20

Depth 0 always refers to the *current* context.
"""

from register_vm.types import UNDEFINED, Context, VMValue


def new_context(parent: "Context | None", slot_count: int) -> Context:
    """Create a new lexical context with ``slot_count`` uninitialized slots.

    All slots are initialized to ``UNDEFINED``, matching JavaScript's
    behaviour for uninitialized ``let`` / ``const`` declarations (though
    in JS they'd be in the *temporal dead zone* rather than ``undefined``).

    Args:
        parent:     The enclosing context, or ``None`` for the outermost scope.
        slot_count: Number of variable slots in this scope.

    Returns:
        A fresh ``Context`` with ``slot_count`` ``UNDEFINED`` slots.

    Examples::

        outer = new_context(None, 2)   # [UNDEFINED, UNDEFINED]
        inner = new_context(outer, 1)  # [UNDEFINED], parent → outer

        assert inner.parent is outer
        assert len(inner.slots) == 1
    """
    return Context(slots=[UNDEFINED] * slot_count, parent=parent)


def get_slot(ctx: Context, depth: int, idx: int) -> VMValue:
    """Read a slot from the context at the given chain depth.

    Walks up the parent chain ``depth`` times, then reads ``slots[idx]``.

    Args:
        ctx:   The innermost context (depth 0).
        depth: Number of parent links to follow (0 = current context).
        idx:   Slot index within the target context.

    Returns:
        The value stored at that slot.

    Raises:
        IndexError: If ``depth`` exceeds the length of the context chain,
                    or ``idx`` is out of range for the target context's
                    slot array.

    Examples::

        outer = new_context(None, 3)
        outer.slots[1] = 42
        inner = new_context(outer, 0)

        get_slot(inner, depth=1, idx=1)  # → 42
        get_slot(inner, depth=0, idx=0)  # IndexError (inner has 0 slots)
    """
    target = _walk_chain(ctx, depth)
    return target.slots[idx]


def set_slot(ctx: Context, depth: int, idx: int, value: VMValue) -> None:
    """Write a value to a slot in the context at the given chain depth.

    Args:
        ctx:   The innermost context (depth 0).
        depth: Number of parent links to follow (0 = current context).
        idx:   Slot index within the target context.
        value: The value to store.

    Raises:
        IndexError: If the depth or index is out of range.

    Examples::

        outer = new_context(None, 2)
        inner = new_context(outer, 1)

        set_slot(inner, depth=1, idx=0, value=99)
        assert outer.slots[0] == 99

        set_slot(inner, depth=0, idx=0, value="hello")
        assert inner.slots[0] == "hello"
    """
    target = _walk_chain(ctx, depth)
    target.slots[idx] = value


def _walk_chain(ctx: Context, depth: int) -> Context:
    """Walk up the context chain by ``depth`` parent links.

    Args:
        ctx:   Starting context (depth 0).
        depth: Number of parent links to follow.

    Returns:
        The context at the requested depth.

    Raises:
        IndexError: If the chain is shorter than ``depth`` links.
    """
    current = ctx
    for _ in range(depth):
        if current.parent is None:
            raise IndexError(
                f"Context chain is shorter than requested depth {depth}"
            )
        current = current.parent
    return current
