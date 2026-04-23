"""Classification logic for the JIT insight pass.

This module answers two questions for every ``IIRInstr`` in a compiled
function:

1. **What dispatch cost did the JIT incur?**
   ``_classify_cost()`` maps an instruction to one of the four ``DispatchCost``
   levels by inspecting ``type_hint``, ``op``, ``srcs``, and the profiler
   counters.

2. **Which register is responsible for the cost?**
   ``_find_root_register()`` traces the data-flow chain backward from the
   flagged instruction to identify the SSA register whose ``type_hint=="any"``
   triggered the guard or generic dispatch.

Design
------
Both functions are pure — they receive an instruction list and an index and
return a result; no global state is touched.  This makes them easy to test
in isolation and compose in ``analyze.py``.

Classification algorithm (from the spec)
-----------------------------------------
::

    if instr.type_hint != "any":
        → NONE  (statically typed — JIT compiles to a direct typed op)

    elif instr.op == "type_assert":
        → GUARD  (the JIT inserted this guard because type_hint is "any"
                   but inferred type is concrete)

    elif instr.op == "call_runtime" and "generic_" in instr.srcs[0]:
        → GENERIC_CALL  (inferred type is also "any" — full dynamic dispatch)

    elif instr.observation_count > 0 and deopt_count > 0:
        → DEOPT  (a guard was emitted but failed at runtime)

    else:
        → NONE

Root register tracing
---------------------
When a guard is on ``%r0``, the *actual* overhead comes from whatever
variable fed into ``%r0`` without a type annotation.  The tracer walks back
through ``load_reg`` and ``load_mem`` chains to find the furthest-back
register still marked ``type_hint=="any"`` — typically a function parameter.

Example::

    type_assert %r0, "int"   ← flagged instruction (GUARD)
      %r0 = load_mem [arg[0]] : any  ← root register is here
"""

from __future__ import annotations

from interpreter_ir.instr import IIRInstr
from interpreter_ir.opcodes import DYNAMIC_TYPE

from jit_profiling_insights.types import DispatchCost


def _classify_cost(instr: IIRInstr) -> DispatchCost:
    """Classify the dispatch cost of a single instruction.

    Parameters
    ----------
    instr:
        The instruction to classify.  Must have the standard ``IIRInstr``
        fields; the optional ``deopt_count`` field is read via ``getattr``
        with a fallback of 0 so that older versions of interpreter-ir that
        do not yet carry per-instruction deopt counters still work.

    Returns
    -------
    DispatchCost
        The classified cost level.  ``NONE`` means no overhead.
    """
    # Statically typed — JIT emits a direct typed operation.
    if instr.type_hint != DYNAMIC_TYPE:
        return DispatchCost.NONE

    # A type_assert instruction IS the guard the JIT inserted.  The JIT
    # emits one of these for each use of an "any"-typed register when it
    # has successfully inferred a concrete type from profiling.
    if instr.op == "type_assert":
        return DispatchCost.GUARD

    # call_runtime with a "generic_*" callee means the JIT could not infer
    # a concrete type at all and fell back to the full runtime dispatch table.
    if instr.op == "call_runtime" and instr.srcs and isinstance(instr.srcs[0], str):
        if "generic_" in instr.srcs[0]:
            return DispatchCost.GENERIC_CALL

    # Deoptimisation: a guard was emitted (deopt_anchor set) but at runtime
    # the type check failed, causing the interpreter to take over.
    # We read deopt_count via getattr so that forward-compatibility is
    # maintained — when jit-core adds per-instruction deopt counters to
    # IIRInstr this will pick them up automatically.
    deopt_count: int = getattr(instr, "deopt_count", 0)
    if instr.observation_count > 0 and deopt_count > 0:
        return DispatchCost.DEOPT

    return DispatchCost.NONE


def _find_root_register(
    instr: IIRInstr,
    instructions: list[IIRInstr],
    instr_index: int,
) -> str:
    """Trace back along the data-flow chain to find the root untyped register.

    Starting from the first source operand of ``instr``, walk backward
    through ``load_reg`` and ``load_mem`` instructions (which are the SSA
    edges in IIR) to find the furthest-back register whose
    ``type_hint == "any"`` is the true root cause of the dispatch overhead.

    The search is bounded by ``instr_index`` so we never look past the
    current instruction.  We also stop as soon as we leave the ``"any"``
    type chain (i.e., we hit a typed instruction).

    Parameters
    ----------
    instr:
        The flagged instruction (a ``type_assert`` or ``call_runtime``).
    instructions:
        The full instruction list for the function, used for data-flow lookup.
    instr_index:
        The index of ``instr`` within ``instructions``.

    Returns
    -------
    str
        The name of the root SSA register (e.g. ``"%r0"`` or a parameter
        name like ``"n"``).  Falls back to the first source operand of
        ``instr`` if no chain is found.
    """
    # Grab the primary source operand — the register we're guarding.
    if not instr.srcs:
        return instr.dest or "%unknown"

    primary = instr.srcs[0]
    if not isinstance(primary, str):
        # Literal immediate — no register to trace.
        return str(primary)

    current_reg = primary
    current_type = instr.type_hint

    # Build a reverse lookup: dest register → instruction that defines it.
    # We only scan instructions before the current one (SSA invariant).
    defs: dict[str, IIRInstr] = {}
    for i in range(instr_index):
        candidate = instructions[i]
        if candidate.dest is not None:
            defs[candidate.dest] = candidate

    # Walk the def-use chain until we can't go further.
    visited: set[str] = set()
    while current_reg in defs and current_reg not in visited:
        visited.add(current_reg)
        defining_instr = defs[current_reg]

        # Only keep tracing if this definition is also untyped.
        if defining_instr.type_hint != DYNAMIC_TYPE:
            break

        # Memory loads and register copies carry the type through directly.
        if defining_instr.op in ("load_mem", "load_reg", "const"):
            if defining_instr.srcs and isinstance(defining_instr.srcs[0], str):
                next_reg = defining_instr.srcs[0]
                current_reg = next_reg
                current_type = defining_instr.type_hint
            else:
                break
        else:
            # Arithmetic or other ops — the register is the root cause.
            break

    return current_reg


def _savings_description(cost: DispatchCost, call_count: int, op: str) -> str:
    """Generate a human-readable description of what adding a type removes.

    The description is terse and concrete — it names the specific overhead
    that would be eliminated, not just "performance would improve".

    Parameters
    ----------
    cost:
        The classified dispatch cost.
    call_count:
        How many times the instruction executed.
    op:
        The instruction mnemonic (used to name the guard type).
    """
    if cost == DispatchCost.GUARD:
        return f"would eliminate 1 type_assert per call ({call_count:,} branches total)"
    if cost == DispatchCost.GENERIC_CALL:
        return (
            f"would replace generic runtime dispatch with a direct typed call "
            f"({call_count:,} calls, ~10× speedup each)"
        )
    if cost == DispatchCost.DEOPT:
        return (
            f"would prevent interpreter fallback on every guard failure "
            f"({call_count:,} observations, ~100× cost)"
        )
    return "no overhead"
