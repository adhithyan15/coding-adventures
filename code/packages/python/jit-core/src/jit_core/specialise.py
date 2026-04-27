"""Specialization pass: IIRFunction → list[CIRInstr].

The specialization pass is the heart of jit-core.  It walks the flat list of
``IIRInstr`` objects in an ``IIRFunction``, consults the type-feedback slots
filled by ``vm-core``'s profiler, and emits typed ``CIRInstr`` objects.

How the pass works
------------------
For each ``IIRInstr``:

1.  **Determine the specialization type** using ``_spec_type()``:

    - If ``type_hint`` is concrete (not ``"any"``): use it directly.
    - Elif ``observed_type`` is concrete, not polymorphic, and
      ``observation_count >= min_observations``: use it.
    - Otherwise: fall back to ``"any"`` → generic runtime call.

2.  **Emit typed CIR**:

    - Typed arithmetic/bitwise/comparison: emit type guards for each variable
      source, then the specialized instruction (e.g. ``add_u8``).
    - Generic (type = ``"any"`` or polymorphic): emit ``call_runtime`` with
      a ``"generic_{op}"`` argument for binary ops.
    - Control-flow (``label``, ``jmp``, ``jmp_if_true``, ``jmp_if_false``):
      pass through as-is (no specialization needed at the CIR level).
    - ``const``: emit ``const_{type}`` using the literal value's Python type.
    - ``ret`` / ``ret_void``: emit ``ret_{type}`` / ``ret_void``.
    - ``call`` / ``call_builtin``: pass through — backends handle them.
    - ``cast`` / ``type_assert``: pass through.

Type guard emission
-------------------
Guards are only emitted when:

- ``type_hint == "any"`` (statically typed instructions don't need guards)
- AND the specialization type is concrete
- AND the source operand is a variable name (``str``), not a literal

Each guard is:

    CIRInstr(op="type_assert", dest=None,
             srcs=[var_name, concrete_type], type="void",
             deopt_to=instr.deopt_anchor)

Special-case op mappings
------------------------
Some (op, type) combinations map to non-trivial CIR ops:

    ("add", "str")  → call_runtime "str_concat"
    ("jmp_if_false", "bool") → br_false_bool
    ("jmp_if_true",  "bool") → br_true_bool
    ("neg", type)  → neg_{type}  (unary — handled separately)
    ("not", type)  → not_{type}
"""

from __future__ import annotations

from interpreter_ir import IIRFunction, IIRInstr

from jit_core.cir import CIRInstr

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# Binary ops that map to {op}_{type} by default.
_BINARY_OPS: frozenset[str] = frozenset({
    "add", "sub", "mul", "div", "mod",
    "and", "or", "xor", "shl", "shr",
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
})

# Unary ops that map to {op}_{type}.
_UNARY_OPS: frozenset[str] = frozenset({"neg", "not"})

# Special (op, type) → CIR op overrides.
# When the op is "call_runtime", the runtime name is prepended to srcs.
_SPECIAL_OPS: dict[tuple[str, str], str] = {
    ("add", "str"): "call_runtime",        # str_concat
    ("jmp_if_false", "bool"): "br_false_bool",
    ("jmp_if_true", "bool"): "br_true_bool",
}

# Runtime function names for special cases.
_RUNTIME_NAMES: dict[tuple[str, str], str] = {
    ("add", "str"): "str_concat",
}

# Ops that pass through unchanged (no specialization at the CIR level).
_PASSTHROUGH_OPS: frozenset[str] = frozenset({
    "label", "jmp", "jmp_if_true", "jmp_if_false",
    "call", "call_builtin",
    "cast", "type_assert",
    "load_reg", "store_reg", "load_mem", "store_mem",
    "io_in", "io_out",
})


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------

def specialise(fn: IIRFunction, min_observations: int = 5) -> list[CIRInstr]:
    """Translate ``fn``'s instructions into typed ``CIRInstr`` objects.

    Parameters
    ----------
    fn:
        The ``IIRFunction`` to specialize.  Its ``IIRInstr`` objects should
        have been populated with type-feedback by ``vm-core``'s profiler.
    min_observations:
        Minimum number of times an ``"any"``-typed instruction must have been
        profiled before its observed type is used.  Lower values produce more
        aggressive specialization but riskier guards.

    Returns
    -------
    list[CIRInstr]
        Flat sequence of typed CIR instructions ready for the optimizer and
        backend.
    """
    result: list[CIRInstr] = []
    for instr in fn.instructions:
        result.extend(_translate(instr, min_observations))
    return result


# ---------------------------------------------------------------------------
# Per-instruction translation
# ---------------------------------------------------------------------------

def _translate(instr: IIRInstr, min_obs: int) -> list[CIRInstr]:
    op = instr.op

    # --- const ---
    if op == "const":
        return [_translate_const(instr)]

    # --- ret_void ---
    if op == "ret_void":
        return [CIRInstr(op="ret_void", dest=None, srcs=[], type="void")]

    # --- ret ---
    if op == "ret":
        return [_translate_ret(instr, min_obs)]

    # --- passthrough ops ---
    if op in _PASSTHROUGH_OPS:
        spec_type = _spec_type(instr, min_obs)
        return [CIRInstr(op=op, dest=instr.dest, srcs=list(instr.srcs), type=spec_type)]

    # --- binary ops ---
    if op in _BINARY_OPS:
        return _translate_binary(instr, min_obs)

    # --- unary ops ---
    if op in _UNARY_OPS:
        return _translate_unary(instr, min_obs)

    # --- fallback: emit as generic ---
    spec_type = _spec_type(instr, min_obs)
    return [CIRInstr(op=op, dest=instr.dest, srcs=list(instr.srcs), type=spec_type)]


def _translate_const(instr: IIRInstr) -> CIRInstr:
    value = instr.srcs[0] if instr.srcs else 0
    t = instr.type_hint if instr.type_hint != "any" else _literal_type(value)
    return CIRInstr(op=f"const_{t}", dest=instr.dest, srcs=[value], type=t)


def _translate_ret(instr: IIRInstr, min_obs: int) -> CIRInstr:
    spec_type = _spec_type(instr, min_obs)
    return CIRInstr(
        op=f"ret_{spec_type}", dest=None, srcs=list(instr.srcs), type=spec_type
    )


def _translate_binary(instr: IIRInstr, min_obs: int) -> list[CIRInstr]:
    spec_type = _spec_type(instr, min_obs)
    result: list[CIRInstr] = []

    if spec_type == "any":
        # Generic path — emit call_runtime "generic_{op}"
        runtime_name = f"generic_{instr.op}"
        result.append(CIRInstr(
            op="call_runtime",
            dest=instr.dest,
            srcs=[runtime_name] + list(instr.srcs),
            type="any",
        ))
        return result

    # Check for special (op, type) overrides.
    special_cir_op = _SPECIAL_OPS.get((instr.op, spec_type))
    if special_cir_op == "call_runtime":
        runtime_name = _RUNTIME_NAMES.get((instr.op, spec_type), f"generic_{instr.op}")
        result.append(CIRInstr(
            op="call_runtime",
            dest=instr.dest,
            srcs=[runtime_name] + list(instr.srcs),
            type=spec_type,
        ))
        return result

    # Concrete type path — emit guards then specialized op.
    if instr.type_hint == "any":
        # Guards only needed when the instruction was untyped in source.
        deopt = instr.deopt_anchor
        for src in instr.srcs:
            if isinstance(src, str):
                result.append(CIRInstr(
                    op="type_assert",
                    dest=None,
                    srcs=[src, spec_type],
                    type="void",
                    deopt_to=deopt,
                ))

    cir_op = special_cir_op if special_cir_op else f"{instr.op}_{spec_type}"
    result.append(CIRInstr(
        op=cir_op,
        dest=instr.dest,
        srcs=list(instr.srcs),
        type=spec_type,
    ))
    return result


def _translate_unary(instr: IIRInstr, min_obs: int) -> list[CIRInstr]:
    spec_type = _spec_type(instr, min_obs)
    result: list[CIRInstr] = []

    if spec_type == "any":
        result.append(CIRInstr(
            op="call_runtime",
            dest=instr.dest,
            srcs=[f"generic_{instr.op}"] + list(instr.srcs),
            type="any",
        ))
        return result

    if instr.type_hint == "any":
        deopt = instr.deopt_anchor
        for src in instr.srcs:
            if isinstance(src, str):
                result.append(CIRInstr(
                    op="type_assert",
                    dest=None,
                    srcs=[src, spec_type],
                    type="void",
                    deopt_to=deopt,
                ))

    result.append(CIRInstr(
        op=f"{instr.op}_{spec_type}",
        dest=instr.dest,
        srcs=list(instr.srcs),
        type=spec_type,
    ))
    return result


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _spec_type(instr: IIRInstr, min_obs: int) -> str:
    """Return the type to specialise on, or ``"any"`` for the generic path."""
    if instr.type_hint != "any":
        return instr.type_hint
    if instr.observed_type is None:
        return "any"
    if instr.is_polymorphic():
        return "any"
    if instr.observation_count < min_obs:
        return "any"
    return instr.observed_type


def _literal_type(value: object) -> str:
    """Infer an IIR type string from a Python literal value."""
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
