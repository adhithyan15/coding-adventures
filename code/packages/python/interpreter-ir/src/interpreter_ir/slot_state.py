"""``SlotState`` and ``SlotKind`` — the V8 Ignition-style feedback slot.

A *feedback slot* is a per-instruction record of which runtime types have
been observed flowing through one dynamically-typed IIR instruction.  A
JIT uses the slot's *kind* to decide whether to specialise the
instruction on its observed type, emit a small dispatch table, or give
up and fall back to generic runtime calls.

State machine
-------------

Every slot walks through this monotonic progression — it can move only
forward, never back::

    UNINITIALIZED  ──(first observation)──►  MONOMORPHIC
    MONOMORPHIC    ──(same type again)──────► MONOMORPHIC
    MONOMORPHIC    ──(2nd distinct type)───►  POLYMORPHIC
    POLYMORPHIC    ──(3rd or 4th distinct)──► POLYMORPHIC
    POLYMORPHIC    ──(5th distinct type)───►  MEGAMORPHIC
    MEGAMORPHIC    ──(any observation)──────► MEGAMORPHIC

The cap at four stored observations (transitioning to MEGAMORPHIC on
the fifth distinct type) is borrowed verbatim from V8's inline-cache
machinery.  It ensures that the ``observations`` list is O(1) in size —
a megamorphic call site in a long-running program otherwise grows
without bound.

Why this is *here* and not in ``vm-core``
------------------------------------------

An earlier draft of LANG17 placed ``SlotState`` in ``vm_core.metrics``
to keep LANG01 free of "runtime state" concerns.  In practice
``IIRInstr`` already carries runtime-observation fields
(``observed_type``, ``observation_count``), and having ``SlotState``
live *with* the thing it annotates (``IIRInstr.observed_slot``) sits
better with the Python layering — it avoids an import cycle between
``vm-core`` and ``interpreter-ir``, and lets any LANG-pipeline
consumer (vm-core, jit-core, aot-core, a debugger, LSP integration,
Tetrad runtime) reference the type without depending on vm-core.

Language agnosticism
--------------------

The type strings stored in ``observations`` are defined by the
language frontend, not by vm-core.  For Tetrad everything is
``"u8"``; for a Lisp frontend a slot might see
``["cons", "nil", "symbol"]``; for a JavaScript frontend
``["Number", "String", "Object"]``; for a Python frontend
``["int", "str", "list"]``.  The state machine doesn't interpret the
strings — it only compares them for equality.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum

__all__ = ["SlotKind", "SlotState", "MAX_POLYMORPHIC_OBSERVATIONS"]


# Maximum number of distinct types to remember before transitioning the
# slot to MEGAMORPHIC.  Four is the V8 Ignition value and the one
# ``tetrad-vm`` has used historically; we keep it here as a module-level
# constant so downstream code can import it rather than hard-coding ``4``.
MAX_POLYMORPHIC_OBSERVATIONS: int = 4


class SlotKind(Enum):
    """The four states of a feedback slot's type profile.

    The progression is strictly monotonic: a slot only moves forward.

    - ``UNINITIALIZED`` — never reached; feedback slot just allocated.
      JIT waits; no data yet.
    - ``MONOMORPHIC`` — exactly one type seen across all observations.
      JIT specialises aggressively on that type.
    - ``POLYMORPHIC`` — two to four distinct types seen.  JIT emits a
      small dispatch table with type guards per arm.
    - ``MEGAMORPHIC`` — five or more distinct types seen (observations
      list is discarded to cap memory).  JIT skips specialisation.
    """

    UNINITIALIZED = "uninitialized"
    MONOMORPHIC = "monomorphic"
    POLYMORPHIC = "polymorphic"
    MEGAMORPHIC = "megamorphic"


@dataclass
class SlotState:
    """Runtime type-profile for one feedback slot.

    Attributes
    ----------
    kind:
        Current IC state — see :class:`SlotKind`.
    observations:
        Ordered list of distinct type strings seen so far.  Bounded by
        :data:`MAX_POLYMORPHIC_OBSERVATIONS`; discarded entirely when the
        slot transitions to ``MEGAMORPHIC`` so a long-running
        megamorphic site does not grow memory without bound.
    count:
        Total number of observations recorded (including revisits of
        the same type).  Used by JITs to decide whether the profile is
        warm enough to trust.

    Example
    -------

    >>> slot = SlotState()
    >>> slot.kind
    <SlotKind.UNINITIALIZED: 'uninitialized'>
    >>> slot.record("int")
    >>> slot.kind
    <SlotKind.MONOMORPHIC: 'monomorphic'>
    >>> slot.record("int")   # same type again
    >>> slot.kind
    <SlotKind.MONOMORPHIC: 'monomorphic'>
    >>> slot.record("str")   # second distinct type
    >>> slot.kind
    <SlotKind.POLYMORPHIC: 'polymorphic'>
    >>> slot.count
    3
    """

    kind: SlotKind = SlotKind.UNINITIALIZED
    observations: list[str] = field(default_factory=list)
    count: int = 0

    # ------------------------------------------------------------------
    # State transitions
    # ------------------------------------------------------------------

    def record(self, type_name: str) -> None:
        """Advance the state machine by one observation.

        ``type_name`` is whatever string the language frontend uses to
        identify the runtime type (``"u8"``, ``"cons"``, ``"Number"``,
        …).  The state machine does not interpret the string — it only
        compares it with ``==``, so any hashable string-like identity
        works.
        """
        self.count += 1

        # MEGAMORPHIC is terminal — no further state updates.
        if self.kind is SlotKind.MEGAMORPHIC:
            return

        if type_name in self.observations:
            # Already seen; state is unchanged (we only track *distinct*
            # types for the kind calculation).
            return

        # First time seeing ``type_name``.  Will we transition?
        if len(self.observations) >= MAX_POLYMORPHIC_OBSERVATIONS:
            # Fifth distinct type: discard the observations list to
            # cap memory and mark the slot megamorphic.
            self.observations = []
            self.kind = SlotKind.MEGAMORPHIC
            return

        self.observations.append(type_name)
        # Re-derive the kind from the list length.
        if len(self.observations) == 1:
            self.kind = SlotKind.MONOMORPHIC
        else:
            # 2, 3, or 4 distinct types.
            self.kind = SlotKind.POLYMORPHIC

    # ------------------------------------------------------------------
    # Read helpers
    # ------------------------------------------------------------------

    def is_specialisable(self) -> bool:
        """True when the slot has enough data to JIT-specialise on.

        Returns True only for ``MONOMORPHIC`` slots.  Polymorphic sites
        need a dispatch table (which is a different codegen path), and
        megamorphic sites should not be specialised at all.
        """
        return self.kind is SlotKind.MONOMORPHIC

    def is_megamorphic(self) -> bool:
        """True when the slot has gone megamorphic (≥5 distinct types)."""
        return self.kind is SlotKind.MEGAMORPHIC

    def dominant_type(self) -> str | None:
        """Return the single type string for a MONOMORPHIC slot, else None.

        JITs use this to ask "can I specialise here?" in one call.
        """
        if self.kind is SlotKind.MONOMORPHIC and self.observations:
            return self.observations[0]
        return None
