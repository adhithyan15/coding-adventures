"""suggest() — the main entry point for vm-type-suggestions.

Algorithm
---------
For each function in fn_list:

  1. Skip fully-typed parameters (``type_hint != "any"``).

  2. For each untyped parameter at position ``N``, find the ``load_mem``
     instruction whose first source operand is ``"arg[N]"``.  This is the
     instruction vm-core's profiler uses to record what type was passed for
     that argument on each call.

  3. Classify the observation:
     - No instruction found OR ``observation_count == 0``  → NO_DATA
     - ``observed_type == "polymorphic"``                  → MIXED
     - Any other non-None ``observed_type``                → CERTAIN

  4. For CERTAIN: produce ``suggestion = "declare '{param}: {type}'"``

  5. Build a ``ParamSuggestion`` and add it to the report.

Why ``load_mem [arg[N]]``?
--------------------------
In the IIR produced by gradual-typing language compilers, function arguments
are loaded into SSA registers at the very start of the function body via
``load_mem`` instructions whose source operand names the argument slot:

    load_mem %r0 <- arg[0] : any
    load_mem %r1 <- arg[1] : any
    ...

``vm-core``'s profiler calls ``instr.record_observation(rt)`` after every
instruction that produces a value, including these ``load_mem`` instructions.
So after N calls to ``add(a, b)``, the ``load_mem arg[0]`` instruction has
``observed_type="u8"`` and ``observation_count=N`` — exactly what we need.

Edge cases
----------
- A parameter may have no corresponding ``load_mem`` if the compiler
  optimised it away or used a different convention.  We emit NO_DATA.
- Multiple ``load_mem [arg[N]]`` instructions may exist (e.g. after
  inlining or loop unrolling).  We use the first one found — it always
  has the highest ``observation_count``.
- Already-typed parameters (``type_hint != "any"``) are silently skipped;
  they contribute nothing to the report.
"""

from __future__ import annotations

from interpreter_ir.function import IIRFunction
from interpreter_ir.opcodes import DYNAMIC_TYPE, POLYMORPHIC_TYPE

from vm_type_suggestions.types import Confidence, ParamSuggestion, SuggestionReport


def suggest(
    fn_list: list[IIRFunction],
    *,
    program_name: str = "program",
) -> SuggestionReport:
    """Analyse profiled IIR functions and return parameter type suggestions.

    Parameters
    ----------
    fn_list:
        ``IIRFunction`` objects whose instructions carry profiler annotations
        (``observed_type``, ``observation_count``).  Typically the
        ``module.functions`` list from a post-run ``IIRModule``.
    program_name:
        A friendly label for the output report.

    Returns
    -------
    SuggestionReport
        A structured report with all untyped parameters classified as
        CERTAIN / MIXED / NO_DATA.  Use ``.actionable()`` to get only
        the CERTAIN suggestions.

    Example::

        from vm_type_suggestions import suggest

        module = ...  # IIRModule after execution under vm-core
        report = suggest(module.functions, program_name=module.name)
        for s in report.actionable():
            print(f"  {s.function}.{s.param_name}: {s.suggestion}")
    """
    all_suggestions: list[ParamSuggestion] = []
    total_calls: int = 0

    for fn in fn_list:
        # Build a fast lookup: arg_index → first matching load_mem instruction.
        arg_loaders: dict[int, object] = _find_arg_loaders(fn)

        for param_index, (param_name, type_hint) in enumerate(fn.params):
            # Already typed — the compiler knows; no suggestion needed.
            if type_hint != DYNAMIC_TYPE:
                continue

            instr = arg_loaders.get(param_index)

            if instr is None or instr.observation_count == 0:  # type: ignore[union-attr]
                suggestion = ParamSuggestion(
                    function=fn.name,
                    param_name=param_name,
                    param_index=param_index,
                    observed_type=None,
                    call_count=0,
                    confidence=Confidence.NO_DATA,
                    suggestion=None,
                )
            elif instr.observed_type == POLYMORPHIC_TYPE:  # type: ignore[union-attr]
                suggestion = ParamSuggestion(
                    function=fn.name,
                    param_name=param_name,
                    param_index=param_index,
                    observed_type=POLYMORPHIC_TYPE,
                    call_count=instr.observation_count,  # type: ignore[union-attr]
                    confidence=Confidence.MIXED,
                    suggestion=None,
                )
            else:
                observed = instr.observed_type  # type: ignore[union-attr]
                count = instr.observation_count  # type: ignore[union-attr]
                suggestion = ParamSuggestion(
                    function=fn.name,
                    param_name=param_name,
                    param_index=param_index,
                    observed_type=observed,
                    call_count=count,
                    confidence=Confidence.CERTAIN,
                    suggestion=f"declare '{param_name}: {observed}'",
                )
                total_calls += count

            all_suggestions.append(suggestion)

    return SuggestionReport(
        program_name=program_name,
        total_calls=total_calls,
        suggestions=all_suggestions,
    )


def _find_arg_loaders(fn: IIRFunction) -> dict[int, object]:
    """Return a mapping from arg index to the first load_mem instruction for it.

    Scans all instructions in the function body for ``load_mem`` instructions
    whose first source operand matches the pattern ``"arg[N]"``.  Returns the
    first match for each index (highest observation_count, since it's always
    the first load of the argument).

    Parameters
    ----------
    fn:
        The function to scan.

    Returns
    -------
    dict[int, IIRInstr]
        Maps argument position → ``IIRInstr`` for that position's loader.
        Missing keys mean no ``load_mem [arg[N]]`` was found for that slot.
    """
    loaders: dict[int, object] = {}

    for instr in fn.instructions:
        if instr.op != "load_mem":
            continue
        if not instr.srcs or not isinstance(instr.srcs[0], str):
            continue
        src = instr.srcs[0]
        if not (src.startswith("arg[") and src.endswith("]")):
            continue
        try:
            idx = int(src[4:-1])
        except ValueError:
            continue
        # Keep only the first occurrence (first = highest observation count).
        if idx not in loaders:
            loaders[idx] = instr

    return loaders
