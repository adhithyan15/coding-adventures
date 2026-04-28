"""Static type inference for IIRFunction — the first pass of AOT compilation.

AOT cannot observe runtime types the way JIT can, so it runs a lightweight
**flow-insensitive** Hindley-Milner-style inference pass over the
``IIRInstr`` sequence.

How it works
------------
The pass maintains an *environment* ``env: dict[str, str]`` mapping virtual
variable names to their inferred IIR type strings.  It walks instructions in
order, applying inference rules:

1.  **Seed** — function parameters contribute ``name → type`` entries using
    their declared types.

2.  **Typed instructions** — if ``instr.type_hint != "any"``, the dest is
    immediately bound to ``type_hint``.  No inference needed.

3.  **const** — the dest type is derived from the Python literal in
    ``srcs[0]``: ``bool → "bool"``, small ints → ``"u8"``/``"u16"``/etc.,
    ``float → "f64"``, ``str → "str"``, anything else → ``"any"``.

4.  **Arithmetic / bitwise ops** — both sources must resolve to numeric types;
    the result is the *wider* of the two (numeric promotion):

    ::

        u8 + u8  → u8
        u8 + u16 → u16    (promotion)
        f64 + u8 → f64
        str + u8 → any    (incompatible)

    Numeric rank order: ``bool < u8 < u16 < u32 < u64 < f64``.
    ``"add"`` on two ``"str"`` operands is a special case (string concatenation)
    and infers ``"str"``.

5.  **Comparison ops** — result is always ``"bool"`` when all sources have
    known non-``"any"`` types; ``"any"`` otherwise.

6.  **Unary ops** (``neg``, ``not``) — result is the same type as the source.

7.  **Passthrough / unknown** — any instruction not covered above leaves the
    dest as ``"any"``.

Flow-insensitivity
------------------
The pass makes a single forward scan with no phi-node merging.  For code where
two branches produce different types for the same variable (e.g., one branch
stores ``u8``, another stores ``str``), the result type is ``"any"`` because
the later assignment overwrites the earlier binding.  This is correct and
conservative: we never claim a concrete type that might be violated at runtime.
"""

from __future__ import annotations

from interpreter_ir import IIRFunction, IIRInstr

# ---------------------------------------------------------------------------
# Type rank for numeric promotion
# ---------------------------------------------------------------------------

_NUMERIC_RANK: dict[str, int] = {
    "bool": 0,
    "u8":   1,
    "u16":  2,
    "u32":  3,
    "u64":  4,
    "f64":  5,
}

# ---------------------------------------------------------------------------
# Op classification
# ---------------------------------------------------------------------------

_ARITHMETIC_OPS: frozenset[str] = frozenset({"add", "sub", "mul", "div", "mod"})
_BITWISE_OPS: frozenset[str] = frozenset({"and", "or", "xor", "shl", "shr"})
_COMPARISON_OPS: frozenset[str] = frozenset({
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
})
_UNARY_OPS: frozenset[str] = frozenset({"neg", "not"})


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------

def infer_types(fn: IIRFunction) -> dict[str, str]:
    """Infer variable types for an ``IIRFunction``.

    Parameters
    ----------
    fn:
        The function to analyze.  Its parameter types are used as seeds.

    Returns
    -------
    dict[str, str]
        A mapping from virtual variable name to its inferred IIR type string.
        Variables that could not be typed are absent or map to ``"any"``.
        Parameter names are always present.
    """
    env: dict[str, str] = {}

    # Seed with declared parameter types.
    for name, typ in fn.params:
        env[name] = typ

    for instr in fn.instructions:
        if instr.dest is None:
            continue
        if instr.type_hint != "any":
            env[instr.dest] = instr.type_hint
            continue
        env[instr.dest] = _infer_instr(instr, env)

    return env


# ---------------------------------------------------------------------------
# Per-instruction inference
# ---------------------------------------------------------------------------

def _infer_instr(instr: IIRInstr, env: dict[str, str]) -> str:
    op = instr.op

    if op == "const":
        value = instr.srcs[0] if instr.srcs else 0
        return _literal_type(value)

    if op in _ARITHMETIC_OPS or op in _BITWISE_OPS:
        if len(instr.srcs) < 2:
            return "any"
        t0 = _resolve(instr.srcs[0], env)
        t1 = _resolve(instr.srcs[1], env)
        if op == "add" and t0 == "str" and t1 == "str":
            return "str"
        return _promote(t0, t1)

    if op in _COMPARISON_OPS:
        if len(instr.srcs) < 2:
            return "any"
        t0 = _resolve(instr.srcs[0], env)
        t1 = _resolve(instr.srcs[1], env)
        if t0 == "any" or t1 == "any":
            return "any"
        return "bool"

    if op in _UNARY_OPS:
        if not instr.srcs:
            return "any"
        return _resolve(instr.srcs[0], env)

    return "any"


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _resolve(src: object, env: dict[str, str]) -> str:
    """Return the type of an IIR source operand."""
    if isinstance(src, bool):
        return "bool"
    if isinstance(src, int):
        return _literal_type(src)
    if isinstance(src, float):
        return "f64"
    if isinstance(src, str):
        return env.get(src, "any")
    return "any"


def _promote(a: str, b: str) -> str:
    """Return the wider of two numeric types, or ``"any"`` if incompatible."""
    if a == "any" or b == "any":
        return "any"
    rank_a = _NUMERIC_RANK.get(a)
    rank_b = _NUMERIC_RANK.get(b)
    if rank_a is None or rank_b is None:
        return "any"
    return a if rank_a >= rank_b else b


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
