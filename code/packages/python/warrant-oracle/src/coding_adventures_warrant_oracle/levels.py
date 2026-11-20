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
``match`` on a ``Result``.  ``ir.data`` (slice S11a, section 16.14 [D46] to [D48]) is the four tags of
Appendix A.2 (``some none ctor field``), record and variant types, ``Option`` as a real type and
``match`` on anything but a ``Result``.  ``ir.list`` (lists) and ``ir.seq`` (``fold map ...``) are
**reserved**: their names are closed, nothing of them is implemented and a document that needs one is
refused whole.  ``ir.fixed`` and ``ir.tuple`` are *siblings*: each depends on ``ir.core`` only (section
16.10 R1); ``ir.data`` implies ``ir.tuple`` only in a *lane's declared levels*, never in a ``requires``.

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

``match`` (section 4.1, [D35]; Appendix A.2)
--------------------------------------------
``match`` is ``ir.tuple`` exactly when its scrutinee is a ``Result`` and ``ir.data`` for any other
type, **and** ``ir.data`` as well whenever any of its patterns, at any depth, is a data pattern
(``some none ctor bool``).  The IR alone does not know the scrutinee's type, so :func:`ir_levels` takes
the levels of each ``match`` from the caller (``match_levels``, filled by the lowerer, which does know)
and otherwise assumes ``ir.tuple`` plus the data patterns it can see.

``Option<T>`` (section 16.10 D23 as amended by [D48]; section 16.14 R29)
------------------------------------------------------------------------
``Option<T>`` is a real type of ``ir.data``.  What stays ``ir.fixed`` and adds no ``ir.data`` is the
**intermediate use**: a node of one of the nine producers of :data:`OPTION_PRODUCERS` that is directly
the first operand of an ``unwrap_or``.  :func:`type_levels` counts every other ``Option`` type.
"""

from __future__ import annotations

from collections.abc import Mapping

from . import ast as A
from .errors import OracleError
from .irtree import children, is_node, match_arms

CORE = "ir.core"
FIXED = "ir.fixed"
TUPLE = "ir.tuple"
DATA = "ir.data"
SEQ = "ir.seq"
LIST = "ir.list"

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
#: Tags of ``ir.data`` (Appendix A.2, [D47]); none needs a type to be classified.  ``list`` is no longer
#: one of them: lists are the reserved ``ir.list`` (section 16.14 [D46], R27).
DATA_TAGS = frozenset(["some", "none", "ctor", "field"])
#: The tag of the reserved ``ir.list`` (the list literal).  It is refused, never counted.
LIST_TAGS = frozenset(["list"])
#: The data patterns of a ``match`` (Appendix A.2): any one of them adds ``ir.data`` to the ``match``.
DATA_PATTERNS = frozenset(["some", "none", "ctor", "bool"])
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
    """The levels a *type* brings (section 16.10 R9 as extended by section 16.14 R29), looking inside type arguments.

    A tuple type, or any type that contains one (``Option<(U8,U8)>``, ``Result<(U64,Nat),E>``), adds
    ``ir.tuple``: the rule is on the **type**, not on a tag, because a consumer that does not implement
    tuples cannot hold the value.  The same rule adds ``ir.data`` for a record or variant (recognised by
    its name in ``data_names``) and for ``Option<T>``, wherever they appear inside the type.  A list is
    **not counted**: a type that is or contains a list is refused whole (:func:`type_refusal`).

    The one exception of R29(d), the static type of a producer node in the intermediate use, is the
    caller's to apply: it does not call this function for that node.
    """
    found: set[str] = set()
    stack = [ty]
    while stack:
        t = stack.pop()
        if not isinstance(t, A.Ty):
            continue
        if t.head == "()":
            found.add(TUPLE)
        elif t.head == "Option" or t.head in data_names:
            found.add(DATA)
        stack.extend(t.args)
    return found


def type_refusal(ty: A.Ty) -> str | None:
    """The level that *refuses* ``ty`` where a type may appear, or ``None`` if it is allowed.

    Only a list is refused, and under the **reserved** level name ``ir.list`` (section 16.14 R27): the
    oracle implements tuples, records, variants and ``Option`` (:func:`type_levels` counts them), and a
    document whose type is or contains ``List<T>`` anywhere is refused whole.
    """
    stack = [ty]
    while stack:
        t = stack.pop()
        if not isinstance(t, A.Ty):
            continue
        if t.head == "List":
            return LIST
        stack.extend(t.args)
    return None


def has_data_pattern(pattern: object) -> bool:
    """Is ``pattern`` or any pattern inside it a data pattern (``some none ctor bool``, Appendix A.2)?"""
    stack = [pattern]
    while stack:
        p = stack.pop()
        if not is_node(p):
            continue
        if p[0] in DATA_PATTERNS:
            return True
        if p[0] == "ok" and len(p) == 2:
            stack.append(p[1])
        elif p[0] == "tuple" and len(p) == 2 and isinstance(p[1], list):
            stack.extend(p[1])
    return False


def ir_levels(
    irs: object,
    models: Mapping[str, object] | None = None,
    *,
    follow: bool = True,
    match_levels: Mapping[int, frozenset] | None = None,
) -> list[str]:
    """The sorted ``requires`` of one or more IR trees (section 16.10 R9, the tag part).

    ``irs`` is one IR tree or a list of trees (a suite row's ``admit``, ``body`` and the two
    sides of its oracle).  ``models`` maps a model name to its IR body; the **call closure**
    of every ``model_call`` is included, each model once.  The result is the sorted,
    duplicate-free list of level names, ``[]`` for no tags at all.  With ``follow=False`` a
    ``model_call`` is not followed (the caller adds the callee's own, already closed, requires).  An unknown tag is an
    ``ir`` error (never skipped, section 4.2 rule 7).  A ``match``'s levels depend on the static type of
    its scrutinee, which an IR tree does not carry: ``match_levels`` maps ``id(node)`` of a ``match`` to the
    levels the caller computed for it (the lowerer's), and a ``match`` not in it is ``ir.tuple`` (the
    ``Result`` case) plus ``ir.data`` if a data pattern is visible.  A ``match``'s patterns are not
    expressions and are never visited as such, only the scrutinee and the arm bodies are.  The tag of the
    reserved ``ir.list`` is never counted: it is refused (``level`` error naming ``ir.list``).
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
        if tag in LIST_TAGS:
            raise OracleError("level", "a list needs the reserved ir.list level", level=LIST)
        if tag == "match":
            given = match_levels.get(id(node)) if match_levels is not None else None
            if given is not None:
                levels.update(given)
            else:
                levels.add(TUPLE)
                if any(has_data_pattern(pattern) for pattern, _body in match_arms(node)):
                    levels.add(DATA)
        else:
            level = tag_level(tag)
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
