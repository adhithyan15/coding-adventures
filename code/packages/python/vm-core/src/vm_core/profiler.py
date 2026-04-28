"""VMProfiler — runtime type observation for InterpreterIR instructions.

The profiler watches each instruction execute and fills in three linked
views on the IIRInstr object:

- ``observed_slot``    — the V8 Ignition-style state machine (LANG17);
                         exposes UNINIT / MONO / POLY / MEGA plus the
                         list of distinct types seen
- ``observed_type``    — legacy two-state view (``None`` / concrete /
                         ``"polymorphic"``)
- ``observation_count`` — legacy integer counter

A JIT reads these to decide whether to specialise, emit a polymorphic
dispatch table, or give up.

The profiler only records observations for instructions whose
``type_hint`` is ``"any"`` — statically-typed instructions already know
their type and don't need profiling.  Consequences:

- A **fully-typed** program pays **zero** profiler overhead.
- A **partially-typed** program only profiles the ``"any"`` slots.
- A **fully-untyped** program observes every value-producing instruction.

Types in this pipeline are *optional*.  The VM will run regardless of
how much (or how little) static type information the frontend provided;
the profiler and the JIT together fill in what the type-checker left
blank.

Pluggable type mapping
----------------------

Every language frontend has different runtime values:

- Tetrad has ``u8`` ints.
- BASIC has ``u16`` / ``str``.
- Lisp has ``cons`` / ``symbol`` / ``closure`` heap objects.
- Python has ``int`` / ``str`` / ``list`` / ``dict`` / arbitrary
  class instances.
- JavaScript has ``Number`` / ``String`` / ``Object`` / ``Array``
  tagged values.

The profiler cannot know how to map these to IIR type strings on its
own.  The ``type_mapper`` argument on ``VMCore`` (threaded through to
``VMProfiler``) is a callable every frontend can override to reflect
its own type ontology.  The default :func:`default_type_mapper` below
handles Python primitives and is a sensible starting point for
prototypes.

Default Python-primitive mapping::

    Python type / value             IIR type
    ──────────────────────────────  ────────
    bool                            "bool"
    int in 0..255                   "u8"
    int in 256..65535               "u16"
    int in 65536..2^32-1            "u32"
    int >= 2^32 or negative         "u64"
    float                           "f64"
    str                             "str"
    anything else                   "any"

Note: ``bool`` is checked before ``int`` because ``isinstance(True, int)``
is True in Python (bool is a subclass of int).
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from interpreter_ir import IIRInstr

# Type alias for the pluggable type mapper — a callable from runtime
# value to IIR type string.  Exposed publicly so frontends can declare
# their mappers with a consistent signature.
TypeMapper = Callable[[Any], str]


def default_type_mapper(value: Any) -> str:
    """Map a Python runtime value to an IIR type string.

    Intended for prototype frontends and for the Tetrad runtime's u8
    programs.  Real language frontends should supply their own mapper
    via ``VMCore(type_mapper=my_mapper)``.

    The returned string is stored verbatim in
    ``IIRInstr.observed_slot.observations`` — the state machine only
    compares strings with ``==``, so anything hashable works.  Returning
    ``"any"`` for unknown values keeps the slot conservative and
    prevents mistaken JIT specialisation.
    """
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        if 0 <= value <= 255:
            return "u8"
        if 0 <= value <= 65535:
            return "u16"
        if 0 <= value <= 0xFFFF_FFFF:
            return "u32"
        return "u64"
    if isinstance(value, float):
        return "f64"
    if isinstance(value, str):
        return "str"
    return "any"


# Historical alias — kept as a non-underscore-prefixed export so
# in-tree callers can migrate incrementally.  Prefer
# ``default_type_mapper`` going forward.
_python_type_to_iir = default_type_mapper


class VMProfiler:
    """Observes instruction results and annotates IIRInstr feedback slots.

    Always-on: runs inline in the dispatch loop with constant overhead
    (one equality check, one mapper call, one state-machine advance).

    Only instructions with ``type_hint == "any"`` are profiled.
    Concrete-type instructions are skipped so fully-typed programs pay
    zero cost.

    Parameters
    ----------
    type_mapper:
        Callable that maps a runtime value to an IIR type string.  If
        ``None``, :func:`default_type_mapper` is used.  Supply a custom
        mapper when the VM is hosting a language whose runtime values
        are not Python primitives (Lisp cons cells, Ruby tagged
        pointers, JS Values, etc.).
    """

    def __init__(self, type_mapper: TypeMapper | None = None) -> None:
        self._total_observations: int = 0
        self._type_mapper: TypeMapper = (
            type_mapper if type_mapper is not None else default_type_mapper
        )

    def observe(self, instr: IIRInstr, result: Any) -> None:
        """Record the runtime type of ``result`` on ``instr``.

        Called by the dispatch loop after each instruction that produces
        a value (i.e. ``instr.dest is not None``).  Advances the V8-style
        state machine on ``instr.observed_slot`` and keeps the legacy
        ``observed_type`` / ``observation_count`` views in sync.
        """
        if instr.type_hint != "any":
            return
        rt = self._type_mapper(result)
        instr.record_observation(rt)
        self._total_observations += 1

    @property
    def type_mapper(self) -> TypeMapper:
        """The active value-to-type mapper (read-only)."""
        return self._type_mapper

    @property
    def total_observations(self) -> int:
        """Total number of profiling events recorded this session."""
        return self._total_observations
