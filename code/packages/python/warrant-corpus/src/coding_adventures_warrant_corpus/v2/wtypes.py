"""Warrant types, as VER04r section 2.1 writes them down.

One grammar names a type everywhere: in ``ops``, in binders, in a stream's
context and in a manifest::

    type = "Bool" | "U8" | "U16" | "U32" | "U64" | "I8" | "I16" | "I32" | "I64"
         | "Nat" | "Bytes" | "Bytes<" N ">" | "(" type {"," type} ")"
         | "List<" type ">" | "Option<" type ">" | "Result<" type "," E ">"
         | Name

:func:`parse_type` turns that text into a :class:`Ty`; ``str(ty)`` is the
inverse, so ``str(parse_type(text))`` is the canonical spelling.

``Int`` is the mathematical integer of VER04 section 3.  It exists **inside the
IR only**: the type of ``Nat - Nat`` and of a ``lit`` node that says
``"type": "Int"``.  A value, a binder, a field or an op signature may not have
it; a *model parameter* may ([D75], R111), also under ``Option``, ``Result``,
a tuple or ``List``.  That is what the ``allow_int`` switch below is for.

Two of the shapes here are only half-known while an IR tree is being typed
bottom-up.  A bare ``ok(e)`` has no error type yet and a bare ``err("CASE")`` no
payload type yet (Appendix A, R41); the placeholder ``?`` stands for the missing
half and :func:`unify` fills it in from whatever the node meets.
"""

from __future__ import annotations

import re
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any

from .errors import Refusal

# name -> (bit width, signed)
FIXED: dict[str, tuple[int, bool]] = {
    "U8": (8, False),
    "U16": (16, False),
    "U32": (32, False),
    "U64": (64, False),
    "I8": (8, True),
    "I16": (16, True),
    "I32": (32, True),
    "I64": (64, True),
}
PRIMITIVE = frozenset({"Bool", "Nat", *FIXED})

MAX_FIXED_WIDTH = 4096  # VER04h: bounds every Bytes<N>
MAX_TYPE_DEPTH = 16  # enclosing constructors; a bare name is depth 0

UNKNOWN = "?"

# A Warrant value in its Python form (see :mod:`.values`).  Deliberately loose: the
# shape of a value is decided by its *Warrant* type (a ``bool``, an ``int``, a
# ``bytes``, a tuple ...), which Python's type system cannot express.
Value = Any


@dataclass(frozen=True)
class Ty:
    """A parsed type: a ``head`` and the ``args`` the head takes.

    ===============  ==========================================================
    head             args
    ===============  ==========================================================
    ``Bool`` ``U8``  none (also ``Nat`` and ``Int``)
    ``Bytes``        none, or ``(N,)`` for ``Bytes<N>`` (``N`` an ``int``, or the
                     ``str`` name of a static width parameter)
    ``()``           the component types of a tuple
    ``List`` ``Option``  the element type
    ``Result``       ``(ok_type, Ty(error_name))``
    ``Name``         none: a record, variant or error type from ``types``
    ``?``            none: the unknown half of a bare ``ok`` or ``err``
    ===============  ==========================================================
    """

    head: str
    args: tuple = ()

    def __str__(self) -> str:
        if self.head == "Bytes" and self.args:
            return f"Bytes<{self.args[0]}>"
        if self.head == "()":
            return "(" + ",".join(str(a) for a in self.args) + ")"
        if self.head in ("List", "Option"):
            return f"{self.head}<{self.args[0]}>"
        if self.head == "Result":
            return f"Result<{self.args[0]},{self.args[1]}>"
        return self.head

    @property
    def is_fixed(self) -> bool:
        return self.head in FIXED

    @property
    def is_number(self) -> bool:
        """Fixed-width, ``Nat`` or ``Int``: the types arithmetic is defined on."""
        return self.head in FIXED or self.head in ("Nat", "Int")

    @property
    def is_exact(self) -> bool:
        """``Nat`` or ``Int``: the unbounded-but-windowed types of section 3.2."""
        return self.head in ("Nat", "Int")

    @property
    def width(self) -> int:
        """Bit width of a fixed-width type."""
        return FIXED[self.head][0]

    @property
    def signed(self) -> bool:
        return FIXED[self.head][1]


BOOL = Ty("Bool")
NAT = Ty("Nat")
INT = Ty("Int")
BYTES = Ty("Bytes")
UNKNOWN_TY = Ty(UNKNOWN)


def fixed_range(ty: Ty) -> tuple[int, int]:
    """The inclusive value range of a fixed-width type (two's complement if signed)."""
    bits, signed = FIXED[ty.head]
    if signed:
        return -(1 << (bits - 1)), (1 << (bits - 1)) - 1
    return 0, (1 << bits) - 1


# ---------------------------------------------------------------------------
# Reading the canonical text of a type (section 2.1)
# ---------------------------------------------------------------------------
# The text is cut into tokens first and parsed second.  Cutting is one linear
# pass over the characters, so a hostile string (a run of a million digits, a
# million nested brackets) costs time proportional to its length and nothing
# more; the parser then looks at tokens, never at raw characters.
_TOKEN = re.compile(r"[A-Za-z_][A-Za-z0-9_]*|[0-9]+|[<>(),]")
_WORD_FIRST = re.compile(r"[A-Za-z_]")
MAX_WIDTH_DIGITS = 4  # 4096 is the largest legal width, so a longer run is refused


def _cut_into_tokens(text: str) -> list[str]:
    """Split ``text`` into names, digit runs and the five punctuation marks.

    Anything else, whitespace included, makes the text *not a type*: the grammar
    has no spaces, so ``"Option< U8>"`` is refused rather than tidied.
    """
    tokens: list[str] = []
    cursor = 0
    while cursor < len(text):
        hit = _TOKEN.match(text, cursor)
        if hit is None:
            raise Refusal("type", f"unexpected character at offset {cursor}")
        tokens.append(hit.group(0))
        cursor = hit.end()
    return tokens


def _is_word(token: str) -> bool:
    return _WORD_FIRST.match(token) is not None


def _declared_kind(types: dict, name: str) -> object:
    """The ``kind`` of the declaration called ``name``, or ``None`` if there is none."""
    row = types.get(name)
    return row.get("kind") if isinstance(row, dict) else None


class _TypeReader:
    """A small recursive-descent reader over the tokens of one type text.

    The grammar of section 2.1, one method per production::

        type   = tuple | Name [ "<" args ">" ]
        tuple  = "(" type { "," type } ")"

    ``enclosing`` counts the constructors that wrap the type being read (rule 1
    of section 2.2, R40): a bare name is at depth 0 and each ``(``, ``List<``,
    ``Option<`` or ``Result<`` around it adds one.  Sixteen are legal.
    """

    def __init__(
        self,
        tokens: list[str],
        types: dict | None,
        allow_int: bool,
        widths: frozenset[str],
    ) -> None:
        self.tokens = tokens
        self.at = 0
        self.types = types
        self.allow_int = allow_int
        self.widths = widths

    # -- token plumbing --------------------------------------------------------
    def peek(self) -> str:
        return self.tokens[self.at] if self.at < len(self.tokens) else ""

    def take(self) -> str:
        token = self.peek()
        if not token:
            raise Refusal("type", "the type text ends too soon")
        self.at += 1
        return token

    def want(self, mark: str) -> None:
        if self.take() != mark:
            raise Refusal("type", f"expected {mark!r}")

    def word(self) -> str:
        token = self.take()
        if not _is_word(token):
            raise Refusal("type", "expected a name")
        return token

    # -- productions ------------------------------------------------------------
    def read(self, enclosing: int) -> Ty:
        if enclosing > MAX_TYPE_DEPTH:
            raise Refusal("type", "constructors nested more than 16 deep")
        if self.peek() == "(":
            return self.tuple_of(enclosing)
        name = self.word()
        if name in ("List", "Option"):
            self.want("<")
            element = self.read(enclosing + 1)
            self.want(">")
            return Ty(name, (element,))
        if name == "Result":
            self.want("<")
            payload = self.read(enclosing + 1)
            self.want(",")
            error = self.word()
            self.want(">")
            if self.types is not None and _declared_kind(self.types, error) != "error":
                raise Refusal("type", f"{error} is not a declared error type")
            return Ty("Result", (payload, Ty(error)))
        if name == "Bytes":
            return self.bytes_of()
        if name == "Int":
            if not self.allow_int:
                raise Refusal("type", "Int is a name inside the IR only")
            return INT
        if name in PRIMITIVE:
            return Ty(name)
        if self.types is not None and _declared_kind(self.types, name) in (
            "record",
            "variant",
        ):
            return Ty(name)
        raise Refusal("type", f"{name} names no type")

    def tuple_of(self, enclosing: int) -> Ty:
        self.want("(")
        parts = [self.read(enclosing + 1)]
        while self.peek() == ",":
            self.at += 1
            parts.append(self.read(enclosing + 1))
        self.want(")")
        if not 2 <= len(parts) <= 8:
            raise Refusal("type", f"a tuple of {len(parts)} components")
        return Ty("()", tuple(parts))

    def bytes_of(self) -> Ty:
        """``Bytes`` alone, or ``Bytes<N>`` with a literal or a static width name."""
        if self.peek() != "<":
            return BYTES
        self.at += 1
        token = self.take()
        if token.isdigit():
            if len(token) > MAX_WIDTH_DIGITS or (token[0] == "0" and token != "0"):
                raise Refusal("type", "a width is a plain decimal of at most 4096")
            width: int | str = int(token)
            if width > MAX_FIXED_WIDTH:
                raise Refusal("type", "a width over 4096")
        elif _is_word(token) and token in self.widths:
            width = token
        else:
            raise Refusal("type", "a width is a decimal literal")
        self.want(">")
        return Ty("Bytes", (width,))


def parse_type(
    text: object,
    types: dict | None,
    *,
    allow_int: bool = False,
    widths: frozenset[str] = frozenset(),
) -> Ty:
    """Parse the canonical type text ``text``; refuse with the kit code ``type``.

    ``types`` is the section 2.2 table: a bare name must be a ``record`` or
    ``variant`` in it, and the error name of a ``Result`` an ``error`` in it.
    ``None`` is the *open* mode of a descriptor, which carries no table: there
    only the error name of a ``Result`` is free text.  ``widths`` are the static
    width parameters (``Bytes<N>`` with a name for ``N``) of an op signature.
    """
    if not isinstance(text, str):
        raise Refusal("type", "a type is text")
    reader = _TypeReader(_cut_into_tokens(text), types, allow_int, widths)
    ty = reader.read(0)
    if reader.at != len(reader.tokens):
        raise Refusal("type", "text follows the end of the type")
    return ty


# ---------------------------------------------------------------------------
# The ``types`` table itself (section 2.2): limits every consumer enforces
# ---------------------------------------------------------------------------
# "Rules, all enforced at generation and again by every consumer."  The table is
# a hostile input like any other part of a corpus, so before a name in it is
# looked up (by ``parse_type``, a codec or a descriptor) the table as a whole is
# held to the rules.  The *code* of the refusal belongs to the context that is
# reading the table -- ``type`` for a ``v2-value`` case, ``corpus`` for a
# ``v2-chunk`` case or a corpus part, ``descriptor`` for a ``v2-domain`` case --
# so the checker takes it as an argument.
MAX_ERROR_CASES = 64  # rule 2: one to 64 cases
MAX_RECORD_FIELDS = 32  # rule 3: one to 32 fields
MIN_VARIANT_CTORS = 2  # rule 3: a one-constructor sum is a record
MAX_VARIANT_CTORS = 64
MAX_CTOR_FIELDS = 8  # rule 3: a constructor has zero to 8 fields

# The reserved built-ins a declared name may not take (section 2.2, R58): the
# names of the type grammar.  A table row called ``U8`` would otherwise be
# unreachable (the grammar reads ``U8`` as the integer) or, worse, ambiguous.
BUILTIN_TYPE_NAMES = frozenset(
    {"Bool", "Nat", "Int", "Bytes", "List", "Option", "Result", *FIXED}
)

_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*\Z")


def _is_name(value: object) -> bool:
    return type(value) is str and _NAME.match(value) is not None


def check_types_table(types: object, code: str) -> None:
    """Refuse, with ``code``, a ``types`` table that breaks a rule of section 2.2.

    Checked, for every row: the name is not a reserved built-in; an ``error``
    type has 1 to 64 cases, each a name of the grammar of rule 4, all different
    (cases are compared by name, rule 2); a ``record`` has 1 to 32 fields; a
    ``variant`` has 2 to 64 constructors; a constructor has 0 to 8 fields; field
    and constructor names match ``[A-Za-z_][A-Za-z0-9_]*`` (rule 4) and a
    constructor is not a built-in either.  A row that is not one of those three
    kinds, or whose lists are not lists of the right shape, is refused too: it
    cannot be counted.

    R117: a field or constructor component may not *be* an error type.  An error
    name occurs only as the second argument of ``Result``; as a field type, alone
    or under ``Option``, a tuple or ``List``, it names no value.

    The table is walked once, with no recursion, so its size costs time
    proportional to itself and nothing more.
    """

    def refuse(why: str) -> Refusal:
        return Refusal(code, f"the types table breaks section 2.2: {why}")

    if type(types) is not dict:
        raise refuse("it is an object")
    for name, row in types.items():
        if not _is_name(name) or name in BUILTIN_TYPE_NAMES:
            raise refuse("a declared type name is a built-in or not a name")
        if type(row) is not dict:
            raise refuse("a row is an object")
        kind = row.get("kind")
        if kind == "error":
            cases = row.get("cases")
            if type(cases) is not list or not 1 <= len(cases) <= MAX_ERROR_CASES:
                raise refuse("an error type has one to 64 cases")
            if not all(_is_name(c) for c in cases) or len(set(cases)) != len(cases):
                raise refuse("the cases of an error type are different names")
        elif kind == "record":
            fields = row.get("fields")
            if type(fields) is not list or not 1 <= len(fields) <= MAX_RECORD_FIELDS:
                raise refuse("a record has one to 32 fields")
            for entry in fields:
                if type(entry) is not list or len(entry) != 2 or not _is_name(entry[0]):
                    raise refuse("a field is a [name, type] pair with a legal name")
                if type(entry[1]) is not str:
                    raise refuse("a field type is text")
            if len({entry[0] for entry in fields}) != len(fields):
                raise refuse("a record repeats a field name")  # R108
        elif kind == "variant":
            ctors = row.get("ctors")
            if (
                type(ctors) is not list
                or not MIN_VARIANT_CTORS <= len(ctors) <= MAX_VARIANT_CTORS
            ):
                raise refuse("a variant has two to 64 constructors")
            for entry in ctors:
                if type(entry) is not list or len(entry) != 2:
                    raise refuse("a constructor is a [name, [types]] pair")
                ctor, payload = entry
                if not _is_name(ctor) or ctor in BUILTIN_TYPE_NAMES:
                    raise refuse("a constructor name is a built-in or not a name")
                if type(payload) is not list or len(payload) > MAX_CTOR_FIELDS:
                    raise refuse("a constructor has zero to 8 fields")
                if not all(type(t) is str for t in payload):
                    raise refuse("a constructor field type is text")
            if len({entry[0] for entry in ctors}) != len(ctors):
                raise refuse("a variant repeats a constructor name")  # R108
        else:
            raise refuse("a row is an error, a record or a variant")
    _check_references(types, refuse)


def _field_types(row: dict) -> list[str]:
    """The type texts a record's fields or a variant's constructors hold."""
    if row["kind"] == "record":
        return [entry[1] for entry in row["fields"]]
    if row["kind"] == "variant":
        return [text for entry in row["ctors"] for text in entry[1]]
    return []


def _check_references(types: dict, refuse: Callable[[str], Refusal]) -> None:
    """R108: every name a field type uses is declared, and no type contains itself.

    The shape of every row is already known to be right.  A field type is read
    only as tokens: each name in it that is not a built-in of the grammar must
    be a row of the table (``Option<Ghost>`` names ``Ghost``, and so does the
    error name of a ``Result``).  The names then form a graph -- an edge from
    a record or variant to each declared record or variant its fields mention,
    *through* ``Option``, ``Result``, ``List`` and tuples alike -- and a cycle in
    it is a type that contains itself, directly or through a chain (rule 1).
    The cycle search keeps its own stack, so a long chain costs time and no
    Python recursion: a table that contains itself is a hostile input and must
    be refused, never walked.
    """
    edges: dict[str, list[str]] = {}
    for name, row in types.items():
        targets: list[str] = []
        for text in _field_types(row):
            try:
                tokens = _cut_into_tokens(text)
            except Refusal as error:
                raise refuse(
                    "a field type has a character outside the type syntax"
                ) from error
            enclosing: list[str] = []  # the open brackets: "R" Result, "N" other
            for place, token in enumerate(tokens):
                previous = tokens[place - 1] if place else ""
                if token == "(":
                    enclosing.append("N")
                elif token == "<":
                    enclosing.append("R" if previous == "Result" else "N")
                elif token in (">", ")") and enclosing:
                    enclosing.pop()
                if not _is_word(token) or token in BUILTIN_TYPE_NAMES:
                    continue
                if token not in types:
                    raise refuse("a field type names a type the table does not declare")
                if types[token]["kind"] != "error":
                    targets.append(token)
                elif previous != "," or not enclosing or enclosing[-1] != "R":
                    # R117: an error type is the second argument of `Result`
                    # and nothing else.
                    raise refuse("a field type is an error type, which has no values")
        edges[name] = targets
    # Depth-first search, iterative: 1 = on the current path, 2 = finished.
    state: dict[str, int] = {}
    for start in edges:
        if start in state:
            continue
        state[start] = 1
        stack = [(start, iter(edges[start]))]
        while stack:
            node, successors = stack[-1]
            for follow in successors:
                if state.get(follow) == 1:
                    raise refuse("a record or variant contains itself")
                if follow not in state:
                    state[follow] = 1
                    stack.append((follow, iter(edges[follow])))
                    break
            else:
                state[node] = 2
                stack.pop()


def forget_widths(ty: Ty) -> Ty:
    """Replace every *named* width in ``ty`` by plain ``Bytes`` (R70)."""
    if ty.head == "Bytes" and ty.args and isinstance(ty.args[0], str):
        return BYTES
    if not any(isinstance(a, Ty) for a in ty.args):
        return ty
    return Ty(
        ty.head, tuple(forget_widths(a) if isinstance(a, Ty) else a for a in ty.args)
    )


def bind_widths(ty: Ty, widths: dict[str, int]) -> Ty:
    """Replace each static width name in ``ty`` by its number (section 6, B2).

    An op signature may say ``Bytes<N>`` with ``N`` one of the op's
    ``width_params``; a call supplies the numbers (the fourth element of
    ``subject_op``) and the signature is *bound* before any argument is typed.
    """
    if ty.head == "Bytes" and ty.args and isinstance(ty.args[0], str):
        return Ty("Bytes", (widths[ty.args[0]],))
    if not any(isinstance(a, Ty) for a in ty.args):
        return ty
    return Ty(
        ty.head,
        tuple(bind_widths(a, widths) if isinstance(a, Ty) else a for a in ty.args),
    )


# ---------------------------------------------------------------------------
# Relations used by the IR checker (VER04r section 16.16 R51, restricted to the
# shapes `ir.core` can produce)
# ---------------------------------------------------------------------------
def flows(source: Ty, target: Ty) -> bool:
    """May a value of type ``source`` be passed where ``target`` is wanted?

    The checker's *assignability*: a type flows to itself; ``Nat`` to ``Int``;
    ``Bytes<N>`` to ``Bytes``; ``Result<S,E>`` to ``Result<T,E>`` when ``S``
    flows to ``T`` (the same ``E``).  The unknown ``?`` flows both ways: it is a
    hole the node it meets will fill.  Nothing else lifts: ``U8`` does not flow
    to ``Nat`` and ``Bool`` is not ``U8``.
    """
    if UNKNOWN in (source.head, target.head) or source == target:
        return True
    if source == NAT and target == INT:
        return True
    if source.head == "Bytes" and target == BYTES:
        return True
    if source.head == "Result" and target.head == "Result":
        return flows(source.args[0], target.args[0]) and flows(
            source.args[1], target.args[1]
        )
    if source.head == target.head == "Option":
        return flows(source.args[0], target.args[0])
    return False


def unify(a: Ty, b: Ty) -> Ty | None:
    """The common type of two operands or two branches, or ``None``.

    Equal types; ``?`` and anything; ``Nat`` and ``Int`` to ``Int``; two
    ``Bytes`` types of different width to plain ``Bytes``; two ``Result`` types
    component-wise.  ``None`` means "no common type", which the checker reports
    as ``ill-typed``.
    """
    if a.head == UNKNOWN:
        return b
    if b.head == UNKNOWN or a == b:
        return a
    if {a, b} == {NAT, INT}:
        return INT
    if a.head == b.head == "Bytes":
        return BYTES
    if a.head == b.head == "Result":
        ok, err = unify(a.args[0], b.args[0]), unify(a.args[1], b.args[1])
        return Ty("Result", (ok, err)) if ok and err else None
    if a.head == b.head == "Option":
        inner = unify(a.args[0], b.args[0])
        return Ty("Option", (inner,)) if inner else None
    return None


def is_undetermined(ty: Ty) -> bool:
    """Does ``ty`` still hold a ``?`` (the unknown half of a bare ``ok``/``err``)?"""
    return ty.head == UNKNOWN or any(
        isinstance(arg, Ty) and is_undetermined(arg) for arg in ty.args
    )
