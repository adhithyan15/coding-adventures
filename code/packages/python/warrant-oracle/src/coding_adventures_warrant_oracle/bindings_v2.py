"""The reference binding validator of Warrant corpus v2 (VER04r section 7, slice S5c, [D43]).

What a binding is, and why a validator is part of the kit
---------------------------------------------------------
A **binding** is the data-only line file ``evidence.bindings`` that tells a lane runner which export of
the library under test implements which Warrant operation, which host representation carries each
Warrant type across the boundary, and how a host failure becomes a declared ``Err`` case (VER04i
sections 5.1 and 5.2, VER04r section 7).  Eighteen lane runners will each parse such a file.  If each
decided for itself what "invalid" means, one lane would accept a binding another refuses, and a wrong
error mapping (two cases that share a discriminator, ``err_any`` on a two-case error type) would pass
*vacuously* in the lane that missed it (section 7.5).  So the kit pins the rules as conformance
cases, and the cases must be derived by someone other than the lanes: this module.

It is the oracle's **reference** implementation.  It is

* **lane-neutral.**  It never names a host language.  Everything a lane knows and the file cannot say
  is passed in as the case's ``host`` object: the closed lists of host representations (per Warrant
  value name), of exports that exist, of error discriminators that exist, and of representations that
  are immutable by construction.  A lane's own validator reads the same four lists from its own
  runtime (the exports it can resolve, the built-in classes it allows ...).
* **pure.**  Bytes, a corpus fragment (``types`` and ``ops``) and a host object in; a normalized
  dictionary or a refusal out.  It reads no file and imports no lane.
* **closed.**  A refusal is always ``BindingInvalid`` with one ``reason`` from :data:`REASONS`, the
  closed set S5c fixes (R20).  The set is listed in the package README.

The two ways to call it
-----------------------

``violations(binding, corpus, host)``
    every violation, in the order the checks run.  The kit builder demands exactly **one** for a reject
    case (section 11: each case triggers exactly one error, so no case depends on which of two errors
    an implementation meets first).
``validate_binding(binding, corpus, host)``
    the first violation raised as :class:`BindingRefused`, or the **normalized** binding.

The order of the checks (so that "the first" is well defined)
-------------------------------------------------------------

1. *The bytes* (one violation, then stop): ``too-large``, ``invalid-utf8``, ``bom``, ``cr``,
   ``control-character``.
2. *The lines* (every line is read; if any violation is found, stop): ``syntax``, ``unknown-directive``,
   ``duplicate-directive``, ``bad-value``, ``op-key``, ``type-shape`` for a repeated or unknown key.
3. *The model* (every check runs, in this order): ``missing-directive``; ``bad-value`` for a ``package``
   whose lane is not the ``lane`` line's; ``level-unknown`` and ``level-implication``; ``values-name``,
   ``values-representation``; the ``type`` lines (``type-unknown``, ``type-shape``, and ``export-unknown`` for a
   ``host=`` that is no export); ``values-missing`` and ``type-missing`` (what the corpus's signatures need);
   ``op-unknown`` for an ``op`` line the corpus does not declare; then the corpus's ops in corpus order,
   each with ``op-missing`` or ``export-missing`` / ``export-unknown``, ``args``, ``ret``, ``width``, the error
   rules, ``inout-param`` / ``inout-form`` and ``fresh``.

The ``reason`` set, one line each
---------------------------------

=========================  =====================================================================================
reason                     the refusal
=========================  =====================================================================================
``too-large``              more than 65,536 bytes (a binding is size-capped before it is parsed, section 14)
``invalid-utf8``           the bytes are not UTF-8
``bom``                    the text starts with a byte order mark
``cr``                     a carriage return anywhere: the file is LF-only
``control-character``      any other C0 control character, or DEL (tabs included: the separator is one space)
``syntax``                 a line that is not ``directive`` plus single-space-separated printable tokens, or a
                           token that is not of the shape its directive needs (``key=value``, a list, a name)
``unknown-directive``      the first token is not a directive of section 7.1
``duplicate-directive``    a one-line directive twice, or two ``op`` lines for one op, or two ``type`` lines
                           for one type
``bad-value``              a directive argument that is not of its form (``depth``, ``seed``, ``source``,
                           ``lane``, ``package``, ``module``, a repeated level)
``missing-directive``      one of ``lane package module import source seed depth levels`` is absent
``level-unknown``          a ``levels`` entry that is no level of section 4.1 (reserved ones are known)
``level-implication``      a level declared without a level it needs (the lattice of section 4.1)
``values-name``            a ``values`` key that is not a Warrant value name, or is given twice
``values-representation``  a ``values`` host name outside that Warrant name's closed list
``values-missing``         a Warrant value name the corpus signatures need has no ``values`` entry
``type-unknown``           a ``type`` line for a name that is no record or variant of the corpus
``type-shape``             a ``type`` line whose ``host``, ``fields`` or ``ctors`` is missing, extra, repeated
                           or does not match the corpus declaration
``type-missing``           a record or variant an op signature uses has no ``type`` line
``op-unknown``             an ``op`` line for an op the corpus does not declare
``op-missing``             a corpus op with no ``op`` line (the binding is **total**, VER04i section 5 rule 1)
``op-key``                 an ``op`` key that is unknown or given twice
``export-missing``         an ``op`` line with no ``export=``
``export-unknown``         an ``export=`` or ``type host=`` name that is not in the host's export list
``args``                   ``args=`` of the wrong arity, or a representation not allowed for that parameter
``ret``                    ``ret=`` that is not a representation allowed for the result
``width``                  ``width=`` absent on an op with ``width_params``, present on one without, or not
                           ``implicit`` or ``runtime``
``errors-form``            more than one of ``errors=``, ``error=``, ``err_any=`` on one op
``errors-missing``         an op whose result is a ``Result`` and which maps no error
``errors-unexpected``      an op whose result is not a ``Result`` and which maps an error
``errors-case``            a case that the op's error type does not declare, a case mapped twice, a malformed
                           ``CASE:discriminator`` pair
``errors-incomplete``      an ``errors=`` that leaves a declared case unmapped (VER04i section 5 rule 1)
``discriminator-shared``   two cases of one op with the same discriminator (section 7.5)
``discriminator-bare``     ``Error``, ``Exception`` or ``BaseException`` as a class: a binding cannot
                           swallow arbitrary failures (section 7.5)
``discriminator-unknown``  a discriminator not in the host's closed list (a class that is not found)
``err-any-multi``          ``err_any=`` on an error type with two or more cases (section 7.5)
``panics-form``            ``panics`` as an ``errors=`` discriminator, or ``error=`` on an error type with
                           two or more cases (``panics`` is only for a single-case ``error=``)
``inout-param``            ``inout=`` that names a parameter that is not ``inout`` in the corpus, names one
                           twice, is malformed, or leaves an ``inout`` parameter without a form
``inout-form``             a form that is not one of ``mutable growable cell returned``, a reserved record
                           form, or a form that does not fit the parameter's type
``fresh``                  ``fresh=`` on an op that is not ``fresh`` in the corpus, a value other than
                           ``probe`` or ``immutable``, or ``immutable`` for a mutable representation
=========================  =====================================================================================

What the file may contain (section 7.1 and VER04i section 5.1)
--------------------------------------------------------------

::

    lane python                              one value, ``[a-z][a-z0-9]*``
    package python/ct-compare                ``<lane>/<name>``, the lane being the ``lane`` line's
    module ct_compare                        the Warrant module name
    import coding_adventures_ct_compare      one printable token; resolved by the lane, not here
    source src/coding_adventures_ct_compare  a relative path: no leading ``/``, no ``..``, ``\\`` or ``:``
    seed 0x5741_5252_414e_5421               a ``u64``, decimal or lowercase ``0x`` hex, ``_`` between digit groups,
                                             leading zeros allowed (at most 20 / 16 significant digits)
    depth full|slim
    levels ir.core,ir.fixed,...              comma-separated level names, no repeats
    values Bool=bool Bytes=bytes ...         ``WarrantName=hostname`` pairs, from a closed list per name
    type Point host=Point fields=x,y         a record: the host field names in the corpus field order
    type Decoded host=Decoded ctors=A,B      a variant: exactly the corpus constructor names
    op eq export=ct_eq [args=..] [ret=..] [width=..] [errors=..|error=..|err_any=..] [inout=..] [fresh=..]

Lines are LF-separated; a blank line and a line whose first character is ``#`` are ignored (a comment may
hold any valid UTF-8 without control characters; everything else is printable ASCII).  A ``#`` that is not
the first character of a line is an ordinary character, not the start of a comment.

The normalized binding (the shape R20 leaves to S5c)
----------------------------------------------------

Comments, blank lines, line order and the order of ``levels`` entries do not survive; every value is text
except ``levels`` (a sorted list) and the structured parts::

    {"lane", "package", "module", "import", "source",
     "seed": "<decimal text>", "depth", "levels": [sorted],
     "values": {Name: host},
     "types": {Name: {"host": H, "fields": [..]} | {"host": H, "ctors": [..corpus order..]}},
     "ops": {name: {"export": E, "args": [..]?, "ret": H?, "width": W?,
                    "errors": {CASE: disc}? | "error": CASE? | "err_any": CASE?,
                    "inout": {param: form}?, "fresh": "probe"|"immutable"?}}}

A key is present only when the binding states it, except ``fresh``, which is present (``"probe"`` when the
line says nothing) for every op that is ``fresh`` in the corpus: the default is part of the contract.

Readings of the spec this module takes (each is listed as a question in the slice report)
-----------------------------------------------------------------------------------------

* ``depth`` and ``levels`` are required (the spec shows them without brackets, unlike the optional op keys).
* Every level other than ``ir.core`` needs ``ir.core``; ``ir.data`` needs ``ir.tuple``; ``ir.list`` needs
  ``ir.data``.  Requirements are checked **directly** (the transitive closure follows), so a missing level
  is one violation, not a cascade.  Reserved level names (``ir.list``, ``ir.seq``, ``ir.inout.records``)
  are *known*: the validator knows every name of section 4.1 and accepts a lane that declares one.
* ``panics`` is the discriminator of ``error=CASE`` and exists only if the host list names it.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from .conformance import KitError, Refused, Row, check_type_table, finish_case, render

CATEGORY = "v2-binding"

#: The closed set of ``reason`` values S5c fixes (R20), in the order of the table above.
REASONS = (
    "too-large",
    "invalid-utf8",
    "bom",
    "cr",
    "control-character",
    "syntax",
    "unknown-directive",
    "duplicate-directive",
    "bad-value",
    "missing-directive",
    "level-unknown",
    "level-implication",
    "values-name",
    "values-representation",
    "values-missing",
    "type-unknown",
    "type-shape",
    "type-missing",
    "op-unknown",
    "op-missing",
    "op-key",
    "export-missing",
    "export-unknown",
    "args",
    "ret",
    "width",
    "errors-form",
    "errors-missing",
    "errors-unexpected",
    "errors-case",
    "errors-incomplete",
    "discriminator-shared",
    "discriminator-bare",
    "discriminator-unknown",
    "err-any-multi",
    "panics-form",
    "inout-param",
    "inout-form",
    "fresh",
)

#: A binding is size-capped before it is parsed (section 14: 64 KiB).
MAX_BINDING_BYTES = 64 * 1024

#: The directives that take exactly one argument and may appear once.
SINGLE = ("lane", "package", "module", "import", "source", "seed", "depth", "levels")
#: Absent => ``missing-directive`` (the readings above).
REQUIRED = SINGLE

#: The Warrant value names a ``values`` line may bind (section 7.1).
VALUE_NAMES = ("Bool", "U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64", "Nat", "Bytes", "List", "Option", "Tuple")

#: Every level of section 4.1 (reserved ones included, R20) and the levels each one needs directly.
LEVEL_NEEDS = {
    "ir.core": (),
    "ir.fixed": ("ir.core",),
    "ir.tuple": ("ir.core",),
    "ir.data": ("ir.tuple",),
    "ir.list": ("ir.data",),
    "ir.seq": ("ir.core",),
    "ir.inout.records": ("ir.core",),
}

#: The ``inout`` host forms of section 7.6, and the record forms that are reserved (``ir.inout.records``).
INOUT_FORMS = ("mutable", "growable", "cell", "returned")
RESERVED_FORM_PREFIXES = ("struct:", "slice-of:struct:", "vec-of:struct:", "codec:")

#: A bare class would let a binding swallow arbitrary failures (section 7.5).
BARE_CLASSES = ("Error", "Exception", "BaseException")

OP_KEYS = ("export", "args", "ret", "width", "errors", "error", "err_any", "inout", "fresh")
TYPE_KEYS = ("host", "fields", "ctors")

_TOKEN = re.compile(r"[\x21-\x7e]+")
_IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_LANE = re.compile(r"[a-z][a-z0-9]*")
# A package directory name as the repository spells it: elixir/gf256_native, swift/Barcode1D, csharp/Mosaic.Flux.
_PACKAGE_NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]*")
_SEED = re.compile(r"(?:[0-9]+(?:_[0-9]+)*|0x[0-9a-fA-F]+(?:_[0-9a-fA-F]+)*)")


@dataclass(frozen=True)
class Violation:
    """One refusal: the closed ``reason`` and a human detail (never part of the kit)."""

    reason: str
    detail: str


class BindingRefused(Exception):
    """``BindingInvalid`` with its ``reason`` (the first violation of :func:`violations`)."""

    def __init__(self, violation: Violation) -> None:
        super().__init__(f"BindingInvalid({violation.reason}): {violation.detail}")
        self.reason = violation.reason
        self.detail = violation.detail


class BindingKitError(KitError):
    """The ``corpus`` or ``host`` of a case is malformed: a kit error, never a vector (R45 f)."""


# ================================================================================================
# 1. The corpus fragment and the host lists a case carries
# ================================================================================================
#
# A signature type is the text of section 2.1.  The validator needs only its *shape*: which Warrant
# value name it is keyed by (``values`` holds one entry per name, never one per ``Bytes<N>``), which
# record or variant it names, and what a ``Result`` wraps.  ``Bytes<N>`` with a named width (``N`` is a
# ``width_params`` entry) is still ``Bytes``.


@dataclass(frozen=True)
class Ty:
    """``kind`` is ``prim`` (Bool, a fixed-width integer, Nat), ``bytes``, ``tuple``, ``list``, ``option``,
    ``result`` or ``named``; ``args`` its parts; ``name`` the primitive or declared name."""

    kind: str
    name: str = ""
    args: tuple = ()


_PRIMS = frozenset({"Bool", "U8", "U16", "U32", "U64", "I8", "I16", "I32", "I64", "Nat"})


def parse_ty(text: object, types: dict, widths: tuple = ()) -> Ty:
    """The shape of a signature type, or :class:`BindingKitError` (a corpus that breaks its own schema)."""
    if not isinstance(text, str):
        raise BindingKitError(f"a type is text, not {text!r}")
    pos = 0

    def fail(why: str) -> BindingKitError:
        return BindingKitError(f"the type {text!r}: {why}")

    def name() -> str:
        nonlocal pos
        m = _IDENT.match(text, pos)
        if not m:
            raise fail("a name was expected")
        pos = m.end()
        return m.group(0)

    def eat(ch: str) -> None:
        nonlocal pos
        if text[pos : pos + 1] != ch:
            raise fail(f"{ch!r} was expected")
        pos += 1

    def one() -> Ty:
        nonlocal pos
        if text[pos : pos + 1] == "(":
            pos += 1
            parts = [one()]
            while text[pos : pos + 1] == ",":
                pos += 1
                parts.append(one())
            eat(")")
            if len(parts) < 2:
                raise fail("a tuple has at least two components")
            return Ty("tuple", args=tuple(parts))
        head = name()
        if head == "Bytes":
            if text[pos : pos + 1] == "<":
                end = text.find(">", pos)
                width = text[pos + 1 : end] if end >= 0 else ""
                if not (re.fullmatch(r"0|[1-9][0-9]{0,3}", width) and int(width) <= 4096 or width in widths):
                    raise fail("a Bytes width is a decimal literal 0..=4096 or a width parameter of the op")
                pos = end + 1
            return Ty("bytes")
        if head in ("List", "Option"):
            eat("<")
            inner = one()
            eat(">")
            return Ty(head.lower(), args=(inner,))
        if head == "Result":
            eat("<")
            ok = one()
            eat(",")
            err = name()
            eat(">")
            if err not in types or types[err].get("kind") != "error":
                raise fail("the error type of a Result is a declared error type")
            return Ty("result", name=err, args=(ok,))
        if head in _PRIMS:
            return Ty("prim", name=head)
        if head in types and types[head].get("kind") in ("record", "variant"):
            return Ty("named", name=head)
        raise fail(f"{head!r} is not a value type")

    ty = one()
    if pos != len(text):
        raise fail("trailing text")
    return ty


def key_of(ty: Ty) -> str | None:
    """The Warrant value name a ``values`` entry binds for ``ty`` (``None`` for a record or variant)."""
    if ty.kind == "prim":
        return ty.name
    if ty.kind == "named":
        return None
    if ty.kind == "result":
        return key_of(ty.args[0])
    return {"bytes": "Bytes", "tuple": "Tuple", "list": "List", "option": "Option"}[ty.kind]


@dataclass
class OpSig:
    name: str
    params: list  # [(param name, Ty)]
    ret: Ty
    width_params: list
    inout: list
    fresh: bool


@dataclass
class Corpus:
    """The ``types`` and ``ops`` a binding is validated against, with every signature parsed."""

    types: dict
    ops: dict  # name -> OpSig, in corpus order


def read_corpus(corpus: object) -> Corpus:
    """Check the schema of a case's ``corpus`` (exactly ``types`` and ``ops``) and parse its signatures."""
    if not isinstance(corpus, dict) or set(corpus) != {"types", "ops"}:
        raise BindingKitError("corpus is an object with exactly the keys types and ops")
    types, ops = corpus["types"], corpus["ops"]
    if not isinstance(types, dict) or not isinstance(ops, dict):
        raise BindingKitError("corpus.types and corpus.ops are objects")
    for tname, row in types.items():
        kind = row.get("kind") if isinstance(row, dict) else None
        if kind == "error":
            if not row.get("cases") or not all(isinstance(c, str) for c in row["cases"]):
                raise BindingKitError(f"error type {tname}: cases is a non-empty list of names")
        elif kind == "record":
            if not row.get("fields"):
                raise BindingKitError(f"record {tname}: fields is a non-empty list")
        elif kind == "variant":
            if not row.get("ctors"):
                raise BindingKitError(f"variant {tname}: ctors is a non-empty list")
        else:
            raise BindingKitError(f"type {tname}: kind is error, record or variant")
    # R113 (VER04r 16.22): R108 and R117 reach ``corpus.types``, with the code of a corpus.  ``v2-binding`` has no such
    # code (its closed set is ``BindingInvalid`` and the 39 reasons of R85 (d), none of them a table defect), so a case
    # with a bad table breaks its own schema: a kit error, never a vector.
    try:
        check_type_table(types, "corpus")
    except Refused as exc:
        raise BindingKitError(f"corpus.types: {exc.why}") from exc
    sigs: dict = {}
    for oname, row in ops.items():
        if not isinstance(row, dict) or "params" not in row or "ret" not in row:
            raise BindingKitError(f"op {oname}: params and ret are required")
        widths = tuple(row.get("width_params", []))
        params = [(p[0], parse_ty(p[1], types, widths)) for p in row["params"]]
        names = [p for p, _ in params]
        inout = list(row.get("inout", []))
        if not set(inout) <= set(names):
            raise BindingKitError(f"op {oname}: inout names a parameter that does not exist")
        sigs[oname] = OpSig(
            oname, params, parse_ty(row["ret"], types, widths), list(widths), inout, bool(row.get("fresh"))
        )
        if row.get("fresh") and inout:
            raise BindingKitError(f"op {oname}: fresh and inout together (VER04r section 6)")
    # Field and payload types must parse too (they are what the signatures need, recursively).
    for row in types.values():
        if row["kind"] == "record":
            for _, t in row["fields"]:
                parse_ty(t, types)
        elif row["kind"] == "variant":
            for _, payload in row["ctors"]:
                for t in payload:
                    parse_ty(t, types)
    return Corpus(types, sigs)


@dataclass
class Host:
    """The four closed lists a lane knows and a binding file cannot say."""

    values: dict  # Warrant value name -> [host representation names]
    exports: frozenset
    discriminators: frozenset
    immutable: frozenset


def read_host(host: object) -> Host:
    """Check the schema of a case's ``host``: ``values`` (name -> list), ``exports``, ``discriminators``, ``immutable``."""
    keys = {"values", "exports", "discriminators", "immutable"}
    if not isinstance(host, dict) or set(host) != keys:
        raise BindingKitError(f"host is an object with exactly the keys {sorted(keys)}")
    values = host["values"]
    if not isinstance(values, dict) or not set(values) <= set(VALUE_NAMES):
        raise BindingKitError("host.values maps Warrant value names to lists of host names")
    for lst in [*values.values(), host["exports"], host["discriminators"], host["immutable"]]:
        if not isinstance(lst, list) or not all(isinstance(s, str) and s for s in lst) or len(set(lst)) != len(lst):
            raise BindingKitError("every host list is a list of distinct non-empty names")
    return Host(
        {k: list(v) for k, v in values.items()},
        frozenset(host["exports"]),
        frozenset(host["discriminators"]),
        frozenset(host["immutable"]),
    )


# ================================================================================================
# 2. The bytes, then the lines
# ================================================================================================


@dataclass
class OpLine:
    name: str
    kv: dict
    line: int


@dataclass
class TypeLine:
    name: str
    kv: dict
    line: int


@dataclass
class Parsed:
    """What the line phase read: single directives, ``values`` pairs, ``type`` and ``op`` lines."""

    single: dict = field(default_factory=dict)  # directive -> (text, line)
    values: list = field(default_factory=list)  # [(name, host, line)]
    types: dict = field(default_factory=dict)  # name -> TypeLine
    ops: dict = field(default_factory=dict)  # name -> OpLine


def byte_violation(binding: bytes) -> tuple[Violation | None, str]:
    """The one violation of the bytes, if any, and the decoded text (section 7.1; the first four rows of the table)."""
    if len(binding) > MAX_BINDING_BYTES:
        return Violation("too-large", f"{len(binding)} bytes"), ""
    try:
        text = binding.decode("utf-8")  # strict: no overlong form, no surrogate, nothing past U+10FFFF
    except UnicodeDecodeError:
        return Violation("invalid-utf8", "not UTF-8"), ""
    if text.startswith("\ufeff"):
        return Violation("bom", "a byte order mark"), ""
    if "\r" in text:
        return Violation("cr", "a carriage return"), ""
    for ch in text:
        if (ord(ch) < 0x20 and ch != "\n") or ord(ch) == 0x7F:
            return Violation("control-character", f"U+{ord(ch):04X}"), ""
    return None, text


def _pairs(tokens: list[str]) -> list[tuple[str, str]] | None:
    """``key=value`` tokens as pairs (split at the first ``=``), or ``None`` if one is malformed."""
    out = []
    for tok in tokens:
        key, eq, value = tok.partition("=")
        if not eq or not key or not value:
            return None
        out.append((key, value))
    return out


def parse_lines(text: str) -> tuple[Parsed, list[Violation]]:
    """Read every line (never stopping at the first problem) into :class:`Parsed` and the line violations."""
    parsed, found = Parsed(), []

    def bad(reason: str, line: int, detail: str) -> None:
        found.append(Violation(reason, f"line {line}: {detail}"))

    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()  # the final LF ends the last line; it does not begin an empty one
    for number, raw in enumerate(lines, 1):
        if raw == "" or raw.startswith("#"):
            continue
        tokens = raw.split(" ")
        if not all(_TOKEN.fullmatch(t) for t in tokens):
            bad("syntax", number, "directive and arguments are printable ASCII tokens separated by one space")
            continue
        directive, args = tokens[0], tokens[1:]
        if directive in SINGLE:
            _single(parsed, directive, args, number, bad)
        elif directive == "values":
            _values(parsed, args, number, bad)
        elif directive == "type":
            _type(parsed, args, number, bad)
        elif directive == "op":
            _op(parsed, args, number, bad)
        else:
            bad("unknown-directive", number, directive)
    return parsed, found


def _single(parsed: Parsed, directive: str, args: list, line: int, bad) -> None:
    if len(args) != 1:
        bad("syntax", line, f"{directive} takes one argument")
        return
    if directive in parsed.single:
        bad("duplicate-directive", line, directive)
        return
    value = args[0]
    ok = True
    if directive == "lane":
        ok = bool(_LANE.fullmatch(value))
    elif directive == "package":
        lane, slash, rest = value.partition("/")
        ok = bool(slash and _LANE.fullmatch(lane) and _PACKAGE_NAME.fullmatch(rest))
    elif directive == "module":
        ok = bool(_IDENT.fullmatch(value))
    elif directive == "source":
        ok = not (value.startswith("/") or "\\" in value or ":" in value) and all(
            seg not in ("", "..") for seg in value.split("/")
        )
    elif directive == "seed":
        seed = seed_value(value) if _SEED.fullmatch(value) else None
        ok = seed is not None and seed < 1 << 64
    elif directive == "depth":
        ok = value in ("full", "slim")
    elif directive == "levels":
        items = value.split(",")
        if "" in items:
            bad("syntax", line, "levels is a comma-separated list")
            return
        ok = len(set(items)) == len(items)
    if not ok:
        bad("bad-value", line, f"{directive} {value}")
        return
    parsed.single[directive] = (value, line)


#: The most significant digits a ``u64`` can have: ``2**64 - 1`` is 20 decimal digits, 16 hex digits.
SEED_MAX_DECIMAL_DIGITS = 20
SEED_MAX_HEX_DIGITS = 16


def seed_value(text: str) -> int | None:
    """The integer of a ``seed`` text, or ``None`` if it has too many significant digits.

    The regex has already allowed only digit groups and ``_``.  Leading zeros are allowed and mean
    nothing, so they are stripped, with the ``_``, **before** ``int()`` sees the text: Python refuses
    to convert a decimal string of more than 4,300 digits (``ValueError``), and a 64 KiB binding can
    hold far more than that.  More than 20 significant decimal digits, or more than 16 significant hex
    digits, cannot fit a ``u64``, so the answer is ``None`` (``bad-value``) without converting at all.
    """
    body = text.replace("_", "")
    if body.startswith("0x"):
        digits, base, limit = body[2:].lstrip("0"), 16, SEED_MAX_HEX_DIGITS
    else:
        digits, base, limit = body.lstrip("0"), 10, SEED_MAX_DECIMAL_DIGITS
    if len(digits) > limit:
        return None
    return int(digits, base) if digits else 0


def _values(parsed: Parsed, args: list, line: int, bad) -> None:
    if "values" in parsed.single:
        bad("duplicate-directive", line, "values")
        return
    pairs = _pairs(args) if args else None
    if pairs is None:
        bad("syntax", line, "values takes Name=host pairs")
        return
    parsed.single["values"] = ("", line)  # marks the directive as seen; the pairs are in parsed.values
    parsed.values = [(k, v, line) for k, v in pairs]


def _type(parsed: Parsed, args: list, line: int, bad) -> None:
    if not args or not _IDENT.fullmatch(args[0]):
        bad("syntax", line, "type takes a name")
        return
    pairs = _pairs(args[1:])
    if pairs is None:
        bad("syntax", line, "type keys are key=value")
        return
    if args[0] in parsed.types:
        bad("duplicate-directive", line, f"type {args[0]}")
        return
    kv: dict = {}
    for key, value in pairs:
        if key not in TYPE_KEYS or key in kv:
            bad("type-shape", line, f"{key} is unknown or repeated")
        else:
            kv[key] = value
    parsed.types[args[0]] = TypeLine(args[0], kv, line)


def _op(parsed: Parsed, args: list, line: int, bad) -> None:
    if not args or not _IDENT.fullmatch(args[0]):
        bad("syntax", line, "op takes a name")
        return
    pairs = _pairs(args[1:])
    if pairs is None:
        bad("syntax", line, "op keys are key=value")
        return
    if args[0] in parsed.ops:
        bad("duplicate-directive", line, f"op {args[0]}")
        return
    kv: dict = {}
    for key, value in pairs:
        if key not in OP_KEYS or key in kv:
            bad("op-key", line, f"{key} is unknown or repeated")
        else:
            kv[key] = value
    parsed.ops[args[0]] = OpLine(args[0], kv, line)


# ================================================================================================
# 3. The model: levels, values, types, ops
# ================================================================================================


def _needs(ty: Ty, corpus: Corpus, values: dict, named: dict) -> None:
    """Collect the value names and the records/variants a signature type needs (ordered sets, as dicts)."""
    if ty.kind == "result":
        _needs(ty.args[0], corpus, values, named)
        return
    if ty.kind == "named":
        if ty.name not in named:
            named[ty.name] = True
            row = corpus.types[ty.name]
            parts = [t for _, t in row["fields"]] if row["kind"] == "record" else [t for _, p in row["ctors"] for t in p]
            for text in parts:
                _needs(parse_ty(text, corpus.types), corpus, values, named)
        return
    values[key_of(ty)] = True
    for child in ty.args:
        _needs(child, corpus, values, named)


def _list(text: str) -> list[str]:
    return text.split(",")


def _pair_list(text: str) -> list[tuple[str, str]] | None:
    """``A:x,B:y`` as pairs split at the first ``:`` (a discriminator or a form may itself hold a ``:``)."""
    out = []
    for item in _list(text):
        head, colon, tail = item.partition(":")
        if not colon or not _IDENT.fullmatch(head) or not tail:
            return None
        out.append((head, tail))
    return out


def violations(binding: bytes, corpus: object, host: object) -> list[Violation]:
    """Every violation of ``binding`` against ``corpus`` and ``host``, in the order of the checks (module docstring)."""
    corpus_ = read_corpus(corpus)
    host_ = read_host(host)
    return _run(binding, corpus_, host_)[0]


def _run(binding: bytes, corpus: Corpus, host: Host) -> tuple[list[Violation], dict | None]:
    first, text = byte_violation(binding)
    if first:
        return [first], None
    parsed, found = parse_lines(text)
    if found:
        return found, None
    out: list[Violation] = []
    normalized: dict = {}

    def bad(reason: str, detail: str) -> None:
        out.append(Violation(reason, detail))

    single = {k: v[0] for k, v in parsed.single.items()}
    for name in REQUIRED:
        if name not in single:
            bad("missing-directive", name)
    for name in ("lane", "module", "import", "source", "depth"):
        if name in single:
            normalized[name] = single[name]
    if "package" in single:
        normalized["package"] = single["package"]
        if "lane" in single and single["package"].split("/")[0] != single["lane"]:
            bad("bad-value", "the package's lane is not the lane")
    if "seed" in single:
        normalized["seed"] = str(seed_value(single["seed"]))  # a refused seed never gets here
    _levels(single, normalized, bad)
    values = _check_values(parsed, host, normalized, bad)
    types = _check_types(parsed, corpus, host, normalized, bad)
    _check_needs(corpus, values, types, bad)
    _check_ops(parsed, corpus, host, values, types, normalized, bad)
    return out, normalized


def _levels(single: dict, normalized: dict, bad) -> None:
    if "levels" not in single:
        return
    declared = single["levels"].split(",")
    known = [lv for lv in declared if lv in LEVEL_NEEDS]
    for lv in declared:
        if lv not in LEVEL_NEEDS:
            bad("level-unknown", lv)
    for lv in sorted(known):
        for need in LEVEL_NEEDS[lv]:
            if need not in declared:
                bad("level-implication", f"{lv} needs {need}")
    normalized["levels"] = sorted(declared)


def _check_values(parsed: Parsed, host: Host, normalized: dict, bad) -> dict:
    """The ``values`` pairs: Warrant names from the closed list, host names from that name's closed list."""
    mapped: dict = {}
    for name, rep, line in parsed.values:
        if name not in VALUE_NAMES or name in mapped:
            bad("values-name", f"line {line}: {name}")
            continue
        mapped[name] = rep  # recorded even when the host name is refused: one fault, one violation
        if rep not in host.values.get(name, []):
            bad("values-representation", f"line {line}: {name}={rep}")
    normalized["values"] = dict(mapped)
    return mapped


def _check_types(parsed: Parsed, corpus: Corpus, host: Host, normalized: dict, bad) -> dict:
    """The ``type`` lines: a record's fields or a variant's constructors, bound to a host export."""
    out: dict = {}
    for name, line in parsed.types.items():
        row = corpus.types.get(name)
        if row is None or row["kind"] not in ("record", "variant"):
            bad("type-unknown", f"line {line.line}: {name}")
            continue
        entry: dict = {}
        target = line.kv.get("host")
        if target is None:
            bad("type-shape", f"line {line.line}: {name} has no host=")
        elif target not in host.exports:
            bad("export-unknown", f"line {line.line}: {target}")
        else:
            entry["host"] = target
        want, other = ("fields", "ctors") if row["kind"] == "record" else ("ctors", "fields")
        if other in line.kv or want not in line.kv:
            bad("type-shape", f"line {line.line}: {name} is a {row['kind']}: it takes {want}=")
        else:
            names = _list(line.kv[want])
            if not all(_IDENT.fullmatch(n) for n in names) or len(set(names)) != len(names):
                bad("type-shape", f"line {line.line}: {want} are distinct names")
            elif row["kind"] == "record" and len(names) != len(row["fields"]):
                bad("type-shape", f"line {line.line}: {len(names)} fields, the corpus has {len(row['fields'])}")
            elif row["kind"] == "variant" and set(names) != {c for c, _ in row["ctors"]}:
                bad("type-shape", f"line {line.line}: the constructors are not the corpus's")
            else:
                entry[want] = names if row["kind"] == "record" else [c for c, _ in row["ctors"]]
        out[name] = entry
    normalized["types"] = {k: v for k, v in out.items()}
    return out


def _check_needs(corpus: Corpus, values: dict, types: dict, bad) -> None:
    """What the corpus's signatures need: one ``values`` entry per value name, one ``type`` line per named type."""
    need_values: dict = {}
    need_types: dict = {}
    for op in corpus.ops.values():
        for _, ty in op.params:
            _needs(ty, corpus, need_values, need_types)
        _needs(op.ret, corpus, need_values, need_types)
    for name in need_values:
        if name not in values:
            bad("values-missing", name)
    for name in need_types:
        if name not in types:
            bad("type-missing", name)


def _rep_fits(entry: str, ty: Ty, host: Host, types: dict) -> bool:
    """May ``entry`` be the host representation of ``ty`` (an ``args=`` or ``ret=`` item)?"""
    if ty.kind == "result":
        return _rep_fits(entry, ty.args[0], host, types)
    key = key_of(ty)
    if key is not None:
        return entry in host.values.get(key, [])
    bound = types.get(ty.name, {}).get("host")
    return bound is None or entry == bound


def _check_ops(parsed: Parsed, corpus: Corpus, host: Host, values: dict, types: dict, normalized: dict, bad) -> None:
    ops_out: dict = {}
    for name, line in parsed.ops.items():
        if name not in corpus.ops:
            bad("op-unknown", f"line {line.line}: {name}")
    for name, sig in corpus.ops.items():
        line = parsed.ops.get(name)
        if line is None:
            bad("op-missing", name)
            continue
        ops_out[name] = _check_op(sig, line, corpus, host, values, types, bad)
    normalized["ops"] = ops_out


def _check_op(sig: OpSig, line: OpLine, corpus: Corpus, host: Host, values: dict, types: dict, bad) -> dict:
    kv, out = line.kv, {}
    where = f"line {line.line}: op {sig.name}"

    export = kv.get("export")
    if export is None:
        bad("export-missing", where)
    elif export not in host.exports:
        bad("export-unknown", f"{where}: {export}")
    else:
        out["export"] = export

    if "args" in kv:
        items = _list(kv["args"])
        if len(items) != len(sig.params):
            bad("args", f"{where}: {len(items)} items for {len(sig.params)} parameters")
        else:
            for item, (pname, ty) in zip(items, sig.params):
                if not _rep_fits(item, ty, host, types):
                    bad("args", f"{where}: {item} for {pname}")
        out["args"] = items
    if "ret" in kv:
        if not _rep_fits(kv["ret"], sig.ret, host, types):
            bad("ret", f"{where}: {kv['ret']}")
        out["ret"] = kv["ret"]

    _check_width(sig, kv, out, where, bad)
    _check_errors(sig, kv, out, corpus, host, where, bad)
    _check_inout(sig, kv, out, where, bad)
    _check_fresh(sig, kv, out, host, values, types, where, bad)
    return out


def _check_width(sig: OpSig, kv: dict, out: dict, where: str, bad) -> None:
    value = kv.get("width")
    if sig.width_params and value is None:
        bad("width", f"{where}: width= is required (the op has width parameters)")
    elif value is not None and not sig.width_params:
        bad("width", f"{where}: width= on an op with no width parameters")
    elif value is not None and value not in ("implicit", "runtime"):
        bad("width", f"{where}: width={value}")
    elif value is not None:
        out["width"] = value


def _check_errors(sig: OpSig, kv: dict, out: dict, corpus: Corpus, host: Host, where: str, bad) -> None:
    """Section 7.5: the closed, exact mapping of a host failure to a declared ``Err`` case."""
    given = [k for k in ("errors", "error", "err_any") if k in kv]
    is_result = sig.ret.kind == "result"
    if len(given) > 1:
        bad("errors-form", f"{where}: {', '.join(given)} together")
        return
    if not given:
        if is_result:
            bad("errors-missing", where)
        return
    if not is_result:
        bad("errors-unexpected", f"{where}: {given[0]}=")
        return
    cases = corpus.types[sig.ret.name]["cases"]
    form = given[0]
    if form == "errors":
        _check_errors_map(kv["errors"], cases, out, host, where, bad)
    elif form == "error":
        if len(cases) != 1:
            bad("panics-form", f"{where}: error= needs a single-case error type")
        elif kv["error"] != cases[0]:
            bad("errors-case", f"{where}: {kv['error']}")
        elif "panics" not in host.discriminators:
            bad("discriminator-unknown", f"{where}: this host has no panics")
        else:
            out["error"] = kv["error"]
    else:
        if len(cases) != 1:
            bad("err-any-multi", f"{where}: err_any= needs a single-case error type")
        elif kv["err_any"] != cases[0]:
            bad("errors-case", f"{where}: {kv['err_any']}")
        else:
            out["err_any"] = kv["err_any"]


def _check_errors_map(text: str, cases: list, out: dict, host: Host, where: str, bad) -> None:
    pairs = _pair_list(text)
    if pairs is None:
        bad("errors-case", f"{where}: errors= is CASE:discriminator pairs")
        return
    mapping: dict = {}
    for case, disc in pairs:
        if case not in cases or case in mapping:
            bad("errors-case", f"{where}: {case}")
            continue
        mapping[case] = disc
        if disc in BARE_CLASSES:
            bad("discriminator-bare", f"{where}: {disc}")
        elif disc == "panics":
            bad("panics-form", f"{where}: panics is only for a single-case error=")
        elif disc not in host.discriminators:
            bad("discriminator-unknown", f"{where}: {disc}")
    shared = sorted({d for d in mapping.values() if list(mapping.values()).count(d) > 1})
    for disc in shared:
        bad("discriminator-shared", f"{where}: {disc}")
    missing = [c for c in cases if c not in mapping]
    if missing and len(mapping) == len(pairs):  # a case that was refused above is not also "unmapped"
        bad("errors-incomplete", f"{where}: {', '.join(missing)}")
    out["errors"] = mapping


def _form_fits(form: str, ty: Ty) -> bool:
    if form == "returned":
        return True
    if form in ("mutable", "growable"):
        return ty.kind == "bytes"
    return ty.kind == "prim"  # cell: an integer or a Bool, for languages with no reference to one


def _check_inout(sig: OpSig, kv: dict, out: dict, where: str, bad) -> None:
    pairs = _pair_list(kv["inout"]) if "inout" in kv else []
    if pairs is None:
        bad("inout-param", f"{where}: inout= is param:form pairs")
        return
    types = dict(sig.params)
    forms: dict = {}
    for param, form in pairs:
        if param not in sig.inout or param in forms:
            bad("inout-param", f"{where}: {param}")
            continue
        forms[param] = form
        if form not in INOUT_FORMS:
            reserved = form.startswith(RESERVED_FORM_PREFIXES)
            bad("inout-form", f"{where}: {param}:{form}{' (reserved: ir.inout.records)' if reserved else ''}")
        elif not _form_fits(form, types[param]):
            bad("inout-form", f"{where}: {form} does not fit the type of {param}")
    for param in sig.inout:
        if param not in forms and len(forms) == len(pairs):
            bad("inout-param", f"{where}: {param} has no form")
    if forms:
        out["inout"] = forms


def _immutable(ty: Ty, override: str | None, host: Host, values: dict, types: dict) -> bool:
    """Is every host representation that carries ``ty`` immutable by construction (section 7.4)?

    ``override`` is a ``ret=`` item, which replaces the representation of the outermost type only.  A
    record or variant counts as the host type its ``type`` line names; a tuple, a list or an option
    counts as its container *and* every component, since a mutable ``bytearray`` inside an immutable
    ``tuple`` can still be written through.  A representation the binding has not given (a missing
    ``values`` entry or ``type`` line) is already a violation of its own and is not judged twice.
    """
    if ty.kind == "result":
        return _immutable(ty.args[0], override, host, values, types)
    key = key_of(ty)
    rep = override or (values.get(key) if key is not None else types.get(ty.name, {}).get("host"))
    if rep is not None and rep not in host.immutable:
        return False
    return all(_immutable(child, None, host, values, types) for child in ty.args)


def _check_fresh(sig: OpSig, kv: dict, out: dict, host: Host, values: dict, types: dict, where: str, bad) -> None:
    value = kv.get("fresh")
    if value is None:
        if sig.fresh:
            out["fresh"] = "probe"  # the default is part of the contract
        return
    if not sig.fresh:
        bad("fresh", f"{where}: fresh= on an op that is not fresh")
    elif value not in ("probe", "immutable"):
        bad("fresh", f"{where}: fresh={value}")
    elif value == "immutable" and not _immutable(sig.ret, kv.get("ret"), host, values, types):
        bad("fresh", f"{where}: the result is not immutable by construction")
    else:
        out["fresh"] = value


def validate_binding(binding: bytes, corpus: object, host: object) -> dict:
    """The normalized binding, or :class:`BindingRefused` for the first violation."""
    found, normalized = _run(binding, read_corpus(corpus), read_host(host))
    if found:
        raise BindingRefused(found[0])
    return normalized


# ================================================================================================
# 4. The kit: deriving ``v2-binding`` cases (section 11; every case is ``["ir.core"]``)
# ================================================================================================
#
# A case is ``binding_hex`` (the file's bytes), ``corpus`` (``types`` and ``ops``) and ``host`` (the
# four closed lists); the outcome is ``{"outcome":"accept","normalized":{..}}`` or
# ``{"outcome":"reject","error":"BindingInvalid","reason":R}``.  The kit builder derives the outcome
# here and **requires exactly one violation** of a reject case, so a lane that meets two problems in a
# different order than this module still maps to the same reason.


def derive_binding(case: dict) -> dict:
    """The outcome of one case's fields (no intent, no envelope)."""
    data = case["binding_hex"]
    if not isinstance(data, str) or not re.fullmatch(r"(?:[0-9a-f]{2})*", data):
        raise BindingKitError("binding_hex is lowercase hex")
    found, normalized = _run(bytes.fromhex(data), read_corpus(case["corpus"]), read_host(case["host"]))
    if not found:
        return {"outcome": "accept", "normalized": normalized}
    if len(found) != 1:
        raise BindingKitError(f"a binding case triggers exactly one violation, this one {len(found)}: {found}")
    return {"outcome": "reject", "error": "BindingInvalid", "reason": found[0].reason}


def build_binding(rows: list[Row]) -> tuple[str, dict]:
    """``(text of v2-binding.json, its index row)`` from the hand-written table."""
    import hashlib

    cases = []
    for row in rows:
        case_id = f"{CATEGORY}/{row.slug}"
        try:
            expect = derive_binding(row.fields)
        except KitError as exc:
            raise KitError(str(exc), case_id) from exc
        if expect["outcome"] == "reject" and expect["reason"] not in REASONS:
            raise KitError("a reason outside the closed set", case_id)
        if expect["outcome"] == "accept" and row.intent.get("normalized") != expect["normalized"]:
            # The normalized form of an accepted case is written by hand and compared by equality, never by
            # sub-object: an extra or missing key in the derivation must be seen.
            raise KitError("the table's normalized binding is not equal to the derived one", case_id)
        cases.append(finish_case(CATEGORY, row, expect))
    ids = [c["id"] for c in cases]
    if len(set(ids)) != len(ids):
        raise KitError(f"a duplicate id in {CATEGORY}")
    text = render(CATEGORY, cases)
    return text, {
        "category": CATEGORY,
        "path": f"{CATEGORY}.json",
        "cases": len(cases),
        "sha256": hashlib.sha256(text.encode("ascii")).hexdigest(),
    }
