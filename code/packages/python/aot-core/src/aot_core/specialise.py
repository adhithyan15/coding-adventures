"""AOT specialization pass: IIRFunction + inferred types → list[CIRInstr].

This pass is the AOT analog of ``jit-core``'s ``specialise()`` function.
The two are structurally identical; the only difference is how the
*specialization type* is determined:

- **JIT**: consults ``IIRInstr.observed_type`` from the runtime profiler.
- **AOT**: consults the ``inferred`` type map produced by ``infer.infer_types()``.

In both cases, ``type_hint`` takes priority when it is concrete (not ``"any"``).

How the spec type is chosen (``_spec_type``)
--------------------------------------------
1. If ``instr.type_hint != "any"`` → use it directly (statically typed source).
2. Elif the instruction has a ``dest`` in ``inferred`` and its type is not
   ``"any"`` → use the inferred type.
3. Elif the instruction is ``ret`` and its first src resolves to a known type
   in ``inferred`` → use that type.
4. Otherwise → ``"any"`` → generic runtime call path.

Guard emission
--------------
Type guards (``type_assert``) are emitted the same way as in jit-core:
only when ``type_hint == "any"`` (statically untyped instruction) and the
specialization type is concrete.  For AOT the backend is responsible for
handling guard failures (typically a trap / abort rather than a JIT deopt).

Passthrough ops
---------------
The same ``_PASSTHROUGH_OPS`` set from jit-core applies.  These ops carry no
type information at the CIR level and are copied verbatim.
"""

from __future__ import annotations

from interpreter_ir import IIRFunction, IIRInstr
from jit_core.cir import CIRInstr

# ---------------------------------------------------------------------------
# Op tables (mirrors jit-core/specialise.py)
# ---------------------------------------------------------------------------

_BINARY_OPS: frozenset[str] = frozenset({
    "add", "sub", "mul", "div", "mod",
    "and", "or", "xor", "shl", "shr",
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
})

_UNARY_OPS: frozenset[str] = frozenset({"neg", "not"})

_SPECIAL_OPS: dict[tuple[str, str], str] = {
    ("add", "str"): "call_runtime",
}

_RUNTIME_NAMES: dict[tuple[str, str], str] = {
    ("add", "str"): "str_concat",
}

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

def aot_specialise(
    fn: IIRFunction,
    inferred: dict[str, str] | None = None,
) -> list[CIRInstr]:
    """Translate ``fn``'s instructions into typed ``CIRInstr`` objects.

    Parameters
    ----------
    fn:
        The ``IIRFunction`` to specialize.
    inferred:
        Type map from ``infer.infer_types(fn)``.  If ``None``, only
        ``type_hint`` fields are used; untyped instructions fall back to the
        generic runtime-call path.

    Returns
    -------
    list[CIRInstr]
        Flat sequence of typed CIR instructions ready for the optimizer and
        backend.
    """
    env = inferred or {}
    result: list[CIRInstr] = []
    for instr in fn.instructions:
        result.extend(_translate(instr, env))
    return result


# ---------------------------------------------------------------------------
# Per-instruction translation
# ---------------------------------------------------------------------------

def _translate(instr: IIRInstr, inferred: dict[str, str]) -> list[CIRInstr]:
    op = instr.op

    if op == "const":
        return [_translate_const(instr, inferred)]

    if op == "ret_void":
        return [CIRInstr(op="ret_void", dest=None, srcs=[], type="void")]

    if op == "ret":
        return [_translate_ret(instr, inferred)]

    if op in _PASSTHROUGH_OPS:
        spec_type = _spec_type(instr, inferred)
        return [CIRInstr(op=op, dest=instr.dest, srcs=list(instr.srcs), type=spec_type)]

    if op in _BINARY_OPS:
        return _translate_binary(instr, inferred)

    if op in _UNARY_OPS:
        return _translate_unary(instr, inferred)

    spec_type = _spec_type(instr, inferred)
    return [CIRInstr(op=op, dest=instr.dest, srcs=list(instr.srcs), type=spec_type)]


def _translate_const(instr: IIRInstr, inferred: dict[str, str]) -> CIRInstr:
    value = instr.srcs[0] if instr.srcs else 0
    t = instr.type_hint if instr.type_hint != "any" else _literal_type(value)
    return CIRInstr(op=f"const_{t}", dest=instr.dest, srcs=[value], type=t)


def _translate_ret(instr: IIRInstr, inferred: dict[str, str]) -> CIRInstr:
    spec_type = _spec_type(instr, inferred)
    return CIRInstr(
        op=f"ret_{spec_type}", dest=None, srcs=list(instr.srcs), type=spec_type
    )


def _translate_binary(instr: IIRInstr, inferred: dict[str, str]) -> list[CIRInstr]:
    spec_type = _spec_type(instr, inferred)
    result: list[CIRInstr] = []

    if spec_type == "any":
        result.append(CIRInstr(
            op="call_runtime",
            dest=instr.dest,
            srcs=[f"generic_{instr.op}"] + list(instr.srcs),
            type="any",
        ))
        return result

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

    cir_op = special_cir_op if special_cir_op else f"{instr.op}_{spec_type}"
    result.append(CIRInstr(
        op=cir_op,
        dest=instr.dest,
        srcs=list(instr.srcs),
        type=spec_type,
    ))
    return result


def _translate_unary(instr: IIRInstr, inferred: dict[str, str]) -> list[CIRInstr]:
    spec_type = _spec_type(instr, inferred)
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

def _spec_type(instr: IIRInstr, inferred: dict[str, str]) -> str:
    """Return the type to specialise on, or ``"any"`` for the generic path."""
    if instr.type_hint != "any":
        return instr.type_hint
    if instr.dest and instr.dest in inferred:
        t = inferred[instr.dest]
        if t != "any":
            return t
    # ret has no dest — check the return value's type in the env.
    if instr.op == "ret" and instr.srcs:
        src = instr.srcs[0]
        if isinstance(src, str) and src in inferred:
            return inferred[src]
    return "any"


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
