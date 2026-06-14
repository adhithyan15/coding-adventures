"""validate_cir_for_lowering — pre-lowering validation of CIR instruction lists.

Before calling ``lower_cir_to_ir_program()`` you can call this function
to get a full list of every reason the instruction list is not lowerable.
The lowerer itself calls this function first and raises ``CIRLoweringError``
if any errors are found, but calling the validator directly gives you all
errors at once (rather than stopping at the first bad instruction).

Why validate separately?
-------------------------

The lowerer is a single-pass emitter: it raises the moment it hits an
unsupported op.  For diagnostics, it is useful to know *all* problems
in the instruction list upfront — for example, an IDE plugin may want to
highlight every unsupported call site rather than just the first one.
The validator fills that role.

What the validator checks
--------------------------

1. **Empty list** — An empty ``list[CIRInstr]`` produces an ``IrProgram``
   with only a LABEL and no real instructions.  This is not useful for
   any backend, so it is flagged immediately.

2. **``call_runtime`` ops** — Generic runtime dispatch cannot be lowered
   without backend-specific ABI knowledge.  Each occurrence is reported.

3. **``io_in`` / ``io_out`` ops** — I/O ops are platform-specific.
   Each occurrence is reported.

4. **``type == "any"`` on arithmetic and comparison ops** — The specialisation
   pass should have resolved all types before the instruction list reaches the
   lowerer.  If any instruction still has ``type == "any"`` and its op is not
   a control-flow or meta op (label, jmp, type_assert, ret_void), the
   lowerer cannot select the right IR opcode family.

Usage
-----
::

    from codegen_core import CIRInstr
    from cir_to_compiler_ir import validate_cir_for_lowering

    instrs = [
        CIRInstr("call_runtime", None, ["allocate_list"], "any"),
        CIRInstr("add_any", "x", ["a", "b"], "any"),
    ]
    errors = validate_cir_for_lowering(instrs)
    # errors == [
    #   "unsupported op 'call_runtime' at index 0: allocate_list",
    #   "unresolved type 'any' at index 1: add_any x = a, b",
    # ]
"""

from __future__ import annotations

from codegen_core import CIRInstr

# Ops that are fine to have type=="any" — they are control-flow or meta ops
# that do not need a type to select an IR opcode family.
_TYPE_AGNOSTIC_OPS: frozenset[str] = frozenset(
    {
        "label",
        "jmp",
        "jmp_if_true",
        "jmp_if_false",
        "br_true_bool",
        "br_false_bool",
        "type_assert",
        "ret_void",
        "call",
    }
)


def validate_cir_for_lowering(instrs: list[CIRInstr]) -> list[str]:
    """Validate a CIR instruction list for lowering to ``IrProgram``.

    Runs all pre-lowering checks and returns a list of human-readable error
    strings.  An empty return value means the list is safe to lower.

    Args:
        instrs: The ``list[CIRInstr]`` to validate.  Produced by
                ``jit_core.specialise()`` or ``aot_core.aot_specialise()``.

    Returns:
        A list of error strings.  Empty if the list is valid for lowering.

    The checks performed (in order):

    1. **Empty list** → ``"empty instruction list"``
    2. **``call_runtime`` ops** → unsupported in V1
    3. **``io_in`` / ``io_out`` ops** → backend-specific, unsupported in V1
    4. **``type == "any"`` on data ops** → specialisation did not resolve type

    Example::

        errors = validate_cir_for_lowering([])
        assert errors == ["empty instruction list"]

        errors = validate_cir_for_lowering([
            CIRInstr("call_runtime", None, ["alloc"], "any")
        ])
        assert "unsupported op 'call_runtime' at index 0" in errors[0]
    """
    errors: list[str] = []

    # ── Check 1: empty list ──────────────────────────────────────────────────
    #
    # An empty instruction list has no useful semantics — it would produce an
    # IrProgram with only a LABEL and no code at all.  Flag it immediately
    # rather than letting the lowerer silently produce a degenerate program.
    if not instrs:
        errors.append("empty instruction list")
        return errors  # No point continuing — there are no instructions.

    # ── Checks 2-4: per-instruction validation ──────────────────────────────
    for i, instr in enumerate(instrs):
        instr_repr = str(instr)

        # Check 2 & 3: unsupported op families
        if instr.op == "call_runtime":
            # srcs[0] is the runtime function name, useful for diagnostics
            rt_name = str(instr.srcs[0]) if instr.srcs else "<unnamed>"
            errors.append(
                f"unsupported op 'call_runtime' at index {i}: {rt_name}"
                f" — {instr_repr}"
            )
        elif instr.op in ("io_in", "io_out"):
            errors.append(
                f"unsupported op '{instr.op}' at index {i}: {instr_repr}"
            )

        # Check 4: unresolved type on data ops
        #
        # The specialisation pass assigns a concrete type string to every
        # arithmetic and comparison op.  "any" means the profiler did not
        # see enough typed observations to specialise — the instruction still
        # refers to the generic IIR interpreter.  The lowerer cannot map
        # "add_any" to either ADD (integer) or F64_ADD (float) without
        # knowing the type.
        if (
            instr.type == "any"
            and instr.op not in _TYPE_AGNOSTIC_OPS
            # call_runtime already flagged above — don't double-report
            and instr.op != "call_runtime"
        ):
            errors.append(
                f"unresolved type 'any' at index {i}: {instr_repr}"
            )

    return errors
