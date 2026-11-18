"""IR feature levels: which tag belongs to which level, and ``requires`` inference.

VER04r section 4.1 groups the IR into **feature levels**; every suite, op and fault of a
corpus declares ``requires``, and every lane declares the levels it implements.  This
module is the one place the oracle keeps the *tag to level* table, so the lowerer
(:mod:`.lower`), the typed IR compiler (:mod:`.ir_eval`) and the corpus writer
(:mod:`.corpus_v2`) cannot disagree about it.

The levels, in one paragraph
----------------------------
``ir.core`` is the base: the 26 tags of Appendix A.  ``ir.fixed`` is exactly the 35
built-ins of Appendix A.1 (the bit-pattern, conversion, checked-arithmetic, endian and
``unwrap_or`` family).  ``ir.tuple`` is the tuple literal, ``project`` and, by [D35],
``match`` on a ``Result``.  ``ir.data`` is everything else about structured data
(``some``/``none``, records, variants, list literals, ``match`` on anything but a
``Result``).  ``ir.seq`` is reserved.  ``ir.fixed`` and ``ir.tuple`` are *siblings*: each
depends on ``ir.core`` only (section 16.10 R1).

``requires`` (section 16.10 R9)
-------------------------------
The ``requires`` of a suite, op or fault is the **sorted union of the levels of every tag in
its expressions and in the call closure of every** ``model_call`` -- a model has no
``requires`` of its own, so a suite that calls a model that uses ``rotl`` requires
``ir.fixed`` although its own body names no built-in.  :func:`ir_levels` computes it from the
IR; the lowerer adds the second half of R9, which is on **types** (a binder, parameter,
result or sub-expression whose static type is or contains a tuple adds ``ir.tuple``, even
where no tuple tag appears; :func:`type_levels`).  Since slice S10t-1 the tuple tags and ``match``
on a ``Result`` (``ir.tuple``) are lowered, typed, costed and evaluated like the core tags.

``match`` (section 4.1, [D35])
------------------------------
``match`` is ``ir.tuple`` exactly when its scrutinee is a ``Result`` and ``ir.data`` for any other
type.  The lowerer lowers a ``match`` only on a ``Result`` (any other scrutinee is refused at the
source with ``ir.data``), so an IR ``match`` is ``ir.tuple`` here; the typed IR compiler
(:mod:`.ir_eval`), which knows the scrutinee's static type, refuses a ``match`` on anything else
with ``ir.data``.

``Option<T>`` (section 16.10 D23, R8)
-------------------------------------
``Option`` is an **IR-only intermediate**.  The nine built-ins in :data:`OPTION_PRODUCERS`
produce one and ``unwrap_or`` is its only consumer, as the first operand and directly.  Every
other use is the level ``ir.data``: both the lowerer and the typed IR compiler refuse it.
"""

from __future__ import annotations

from collections.abc import Mapping

from . import ast as A
from .errors import OracleError
from .irtree import children, is_node

CORE = "ir.core"
FIXED = "ir.fixed"
TUPLE = "ir.tuple"
DATA = "ir.data"
SEQ = "ir.seq"

#: The 26 tags of ``ir.core`` (Appendix A): the 21 tags v1 emits plus ``neg concat take drop
#: is_err``.
CORE_TAGS = frozenset(
    [
        "lit",
        "var",
        "subject_op",
        "model_call",
        "len",
        "ok",
        "err",
        "eq",
        "ne",
        "lt",
        "gt",
        "le",
        "ge",
        "add",
        "sub",
        "mul",
        "and",
        "or",
        "implies",
        "not",
        "if",
        "concat",
        "take",
        "drop",
        "is_err",
        "neg",
    ]
)

#: The nine built-ins whose result is an ``Option<T>`` (section 16.10 D23).
OPTION_PRODUCERS = frozenset(
    [
        "try",
        "checked_add",
        "checked_sub",
        "checked_mul",
        "div",
        "rem",
        "div_trunc",
        "rem_trunc",
        "at",
    ]
)

#: Exactly the 35 names of ``ir.fixed`` (section 4.1, Appendix A.1, [D24]).  ``try`` is
#: spelled ``try`` (an earlier draft of the spec said ``try_nat``, which does not exist).
FIXED_TAGS = frozenset(
    [
        "bit_and",
        "bit_or",
        "bit_xor",
        "bit_not",
        "shl",
        "shr",
        "sar",
        "rotl",
        "rotr",
        "popcount",
        "clz",
        "ctz",
        "bit",
        "widen",
        "wrap",
        "try",
        "to_int",
        "to_nat",
        "max_value",
        "min_value",
        "checked_add",
        "checked_sub",
        "checked_mul",
        "div",
        "rem",
        "div_trunc",
        "rem_trunc",
        "imin",
        "imax",
        "be_bytes",
        "le_bytes",
        "from_be",
        "from_le",
        "at",
        "unwrap_or",
    ]
)

#: Tags of ``ir.tuple`` (Appendix A.1).  ``match`` is type-directed and not in this table.
TUPLE_TAGS = frozenset(["tuple", "project"])
#: Tags of ``ir.data`` that need no type to be classified.
DATA_TAGS = frozenset(["some", "none", "ctor", "list"])
#: Tags of the reserved ``ir.seq``.
SEQ_TAGS = frozenset(["fold", "map", "all", "any", "range", "count"])

#: Built-ins that take one static type argument, written in position 1 of their IR node
#: (``["widen", "U32", e]``, ``["max_value", "I64"]``).
STATIC_TYPE_TAGS = frozenset(["widen", "wrap", "try", "max_value", "min_value", "from_be", "from_le"])

#: How many IR operands each ``ir.fixed`` tag has (the static type argument is not one).
FIXED_ARITY = {
    "max_value": 0,
    "min_value": 0,
    **dict.fromkeys(
        [
            "widen",
            "wrap",
            "try",
            "to_int",
            "to_nat",
            "bit_not",
            "popcount",
            "clz",
            "ctz",
            "be_bytes",
            "le_bytes",
            "from_be",
            "from_le",
        ],
        1,
    ),
    **dict.fromkeys(
        [
            "bit_and",
            "bit_or",
            "bit_xor",
            "shl",
            "shr",
            "sar",
            "rotl",
            "rotr",
            "bit",
            "checked_add",
            "checked_sub",
            "checked_mul",
            "div",
            "rem",
            "div_trunc",
            "rem_trunc",
            "imin",
            "imax",
            "at",
            "unwrap_or",
        ],
        2,
    ),
}


def tag_level(tag: str) -> str | None:
    """The level of an IR tag, or ``None`` for an unknown tag or the type-directed ``match``."""
    if tag in CORE_TAGS:
        return CORE
    if tag in FIXED_TAGS:
        return FIXED
    if tag in TUPLE_TAGS:
        return TUPLE
    if tag in DATA_TAGS:
        return DATA
    if tag in SEQ_TAGS:
        return SEQ
    return None


def type_levels(ty: A.Ty, data_names: frozenset | set = frozenset()) -> set[str]:
    """The levels a *type* brings (section 16.10 R9; section 4.1), looking inside type arguments.

    A tuple type, or any type that contains one (``List<(U8,U8)>``,
    ``Result<(U64,Nat),E>``), adds ``ir.tuple``: the rule is on the **type**, not on a tag,
    because a consumer that does not implement tuples cannot hold the value.  The values of
    ``ir.data`` (section 4.1: lists, records, variants) add ``ir.data`` the same way; a record
    or variant is recognised by its name in ``data_names``.  ``Option`` is not counted here: it
    is never allowed as a type at all (:func:`type_refusal`).
    """
    found: set[str] = set()
    stack = [ty]
    while stack:
        t = stack.pop()
        if not isinstance(t, A.Ty):
            continue
        if t.head == "()":
            found.add(TUPLE)
        elif t.head == "List" or t.head in data_names:
            found.add(DATA)
        stack.extend(t.args)
    return found


def type_refusal(ty: A.Ty) -> str | None:
    """The level that *refuses* ``ty`` where a type may appear, or ``None`` if it is allowed.

    Only ``Option<T>`` is refused (``ir.data``, D23): the oracle's composite domains already
    enumerate tuple, list and record binders (slice S4), so those types are **counted** in
    ``requires`` (:func:`type_levels`) and not refused; a consumer that lacks the level
    records the suite ``not-live`` (section 4.1).
    """
    stack = [ty]
    while stack:
        t = stack.pop()
        if not isinstance(t, A.Ty):
            continue
        if t.head == "Option":
            return DATA
        stack.extend(t.args)
    return None


def ir_levels(
    irs: object, models: Mapping[str, object] | None = None, *, follow: bool = True
) -> list[str]:
    """The sorted ``requires`` of one or more IR trees (section 16.10 R9, the tag part).

    ``irs`` is one IR tree or a list of trees (a suite row's ``admit``, ``body`` and the two
    sides of its oracle).  ``models`` maps a model name to its IR body; the **call closure**
    of every ``model_call`` is included, each model once.  The result is the sorted,
    duplicate-free list of level names, ``[]`` for no tags at all.  With ``follow=False`` a
    ``model_call`` is not followed (the caller adds the callee's own, already closed, requires).  An unknown tag is an
    ``ir`` error (never skipped, section 4.2 rule 7).  A ``match`` is ``ir.tuple`` (see the module
    docstring): its patterns are not expressions and are never visited, only the scrutinee and the
    arm bodies are.
    """
    trees = irs if _is_tree_list(irs) else [irs]
    models = models or {}
    levels: set[str] = set()
    seen_models: set[str] = set()
    stack = list(trees)
    while stack:
        node = stack.pop()
        if not is_node(node):
            continue
        tag = node[0]
        level = TUPLE if tag == "match" else tag_level(tag)
        if level is None:
            raise OracleError("ir", "an unknown IR tag")
        levels.add(level)
        if tag == "model_call" and follow:
            name = node[1]
            if name not in seen_models:
                seen_models.add(name)
                body = models.get(name)
                if body is None:
                    raise OracleError("ir", "a call to an unknown model")
                stack.append(body)
        stack.extend(children(node))
    return sorted(levels)


def _is_tree_list(value: object) -> bool:
    """A list of IR trees, as opposed to one tree (a tree starts with a tag string)."""
    return isinstance(value, list) and (not value or not isinstance(value[0], str))
