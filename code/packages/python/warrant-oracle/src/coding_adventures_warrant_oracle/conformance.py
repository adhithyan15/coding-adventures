"""Derive the v2 conformance cases of ``warrant-corpus-conformance-v1`` (VER04r section 11, slice S5a).

What this module is, and what it is for
---------------------------------------
Section 11 adds six categories to the shared conformance kit: ``v2-value``, ``v2-arith``, ``v2-ir``,
``v2-domain``, ``v2-chunk`` and ``v2-fault``.  A *case* says "feed these inputs to the thing a lane
implements, and expect this outcome".  The kit must be written by someone other than the lanes it
tests, so this oracle derives every expectation, **independently**, and the case tables (hand-written,
beside ``make_conformance.py``) state each case's *intent*.  The two must agree or the build of the
fixture fails naming the case id (R19)::

    table row (inputs + intent)  --->  derive_*()  --->  expect
              \\_______ intent must be contained in expect, or KitError(case id) _______/

Nothing here reads a file: every function takes bytes or JSON-shaped data and returns data, so the
module is pure.  Only ``tests/fixtures/conformance-v2/make_conformance.py`` touches the disk.

The derivations reuse the oracle; none of them is a second evaluator:

===========  ==========================================================================================
category     the oracle code it stands on
===========  ==========================================================================================
v2-value     :func:`read_json` (the strict reader of VER04i 9.1), :func:`parse_type`,
             :func:`decode_value` (section 2.3, built on :func:`.ir_eval.decode_int_text`) and the
             stream encoder of :mod:`.stream`
v2-arith     one node of :class:`.ir_eval.Program` over two ``lit`` operands
v2-ir        :class:`.ir_eval.Program` with each ``subject_op`` rewritten to a pure stand-in model for
             the *value*, and :func:`.ir_eval.compile_calls` on the *unrewritten* tree for ``calls``
v2-domain    :func:`desc_from_json` over :func:`.domains.descriptor_size` and the leaf constructors
v2-chunk     :func:`check_chunk_file` (sections 5.5 and 14), then :meth:`.suites.SuitePlan.run_chunk`
v2-fault     :func:`.faults.fault_row` (the existing templates) and :func:`.faults._compile_fault`
===========  ==========================================================================================

The kit's own error vocabulary (section 16.13 R25)
--------------------------------------------------
The codes of the kit (``ir-overflow``, ``impure-guard``, ``chunk-file`` ...) are **neither the oracle's
codes nor any lane's**.  The oracle's are ``ir``, ``limit``, ``level``, ``generation``, ``corpus`` and
``ir-overflow``; :func:`classify_ir_refusal` is the one place that maps them (the four structural codes
``unknown-tag``, ``level-missing``, ``model-order`` and ``subject-forbidden`` are found by a static
pre-pass over the case, :func:`static_refusal`, before the oracle compiles anything).  Each case
triggers exactly one error, so the precedence of section 11's table never decides a table case.

Exit conventions (R19), used by ``make_conformance.py``: 0 identical, 1 the committed bytes differ,
2 a table and its derivation disagree (:class:`KitError`) or any other error.
"""

from __future__ import annotations

import hashlib
import re
import types as _pytypes
from dataclasses import dataclass, field

from . import ast as A
from . import domains as D
from . import ir_eval as E
from . import levels as L
from . import stream as S
from .corpus_v1 import build_program
from .corpus_v2 import canonical, chunk_file_name, derive
from .errors import OracleError
from .faults import _compile_fault, fault_row
from .irtree import children, is_node
from .lower import FIXED, OpSig, lower_document
from .parser import parse_source
from .profile import parse_profile
from .suites import SuitePlan

SCHEMA = "warrant-corpus-conformance-v1"

#: The six categories S5a derives, in the fixed index order of section 11 (``v2-binding`` is S5c's).
CATEGORIES = ("v2-value", "v2-arith", "v2-ir", "v2-domain", "v2-chunk", "v2-fault")

#: The closed error-code set of each category (section 11, "New categories").
ERROR_CODES = {
    "v2-value": frozenset({"value", "malformed-json", "type"}),
    "v2-arith": frozenset({"ir-overflow"}),
    "v2-ir": frozenset(
        {
            "unknown-tag",
            "level-missing",
            "ir-overflow",
            "impure-guard",
            "ir-cost",
            "ill-typed",
            "model-order",
            "subject-forbidden",
            "depth",
        }
    ),
    "v2-domain": frozenset({"descriptor", "index"}),
    "v2-chunk": frozenset({"chunk-file", "slim-chunks", "corpus"}),
    # [D74] (VER04r 16.22): ``ill-typed`` is the bad ``types`` table and the fault row with no common type.
    "v2-fault": frozenset({"ill-typed", "level-missing", "subject-forbidden"}),
    # S5c (:mod:`.bindings_v2`): one code; the ``reason`` beside it is the closed set of that module.
    "v2-binding": frozenset({"BindingInvalid"}),
}

#: Every S5 case has exactly this ``requires`` (R24): the kit at ``ir.core``.
CORE = ["ir.core"]

#: A slug (R26): lowercase words joined by single hyphens.
SLUG = re.compile(r"[a-z0-9]+(-[a-z0-9]+)*")
MAX_SUMMARY = 240  # the family's own schema check refuses a longer summary (lessons.d)
STREAM_HEX_LIMIT = 2048  # ``stream_hex`` is omitted past this many characters (section 11)
MAX_TYPE_DEPTH = 16  # section 2.2 rule 1
MAX_FIXED_WIDTH = 4096  # section 2.1: ``Bytes<N>`` and a ``Bytes`` value
SAFE_MAX = 2**53 - 1
MAX_JSON_DEPTH = 256  # nested containers; the 257th is refused (VER04i 9.1)


class KitError(Exception):
    """A table and its derivation disagree, or a case is outside the S5 kit (exit 2).

    ``case_id`` names the case when known, so the build fails *naming the case id* (R19).
    """

    def __init__(self, message: str, case_id: str | None = None) -> None:
        super().__init__(f"{case_id}: {message}" if case_id else message)
        self.case_id = case_id


class Refused(Exception):
    """A derivation outcome that is a kit error code (never a build failure): ``code`` is the kit's."""

    def __init__(self, code: str, why: str = "") -> None:
        super().__init__(f"{code}: {why}")
        self.code = code
        self.why = why


# ================================================================================================
# 1. The strict JSON reader (VER04i section 9.1), used by v2-value
# ================================================================================================
#
# ``input_hex`` carries the *bytes* of a JSON value, because a JSON string cannot carry invalid UTF-8,
# a lone surrogate or a BOM, and those are cases.  The reader is the profile every lane implements:
# strict UTF-8 (no overlong form, no surrogate, nothing past U+10FFFF), no BOM, JSON's four
# whitespace bytes, no ``NaN``/``Infinity``, **no fraction and no exponent ever**, a leading zero is
# refused, ``-0`` is refused, an integer past 2^53-1 is refused, a control character in a string is
# refused, an escape must be one of JSON's, a surrogate escape must be a pair, a duplicate key (after
# unescaping) is refused, and nesting past 256 containers is refused.

_WS = " \t\n\r"
_ESCAPES = {'"': '"', "\\": "\\", "/": "/", "b": "\b", "f": "\f", "n": "\n", "r": "\r", "t": "\t"}
_HEXDIGITS = "0123456789abcdefABCDEF"


class JsonReject(Exception):
    """The strict reader refused its input (the kit code is ``malformed-json``)."""


class _Reader:
    def __init__(self, text: str) -> None:
        self.s = text
        self.i = 0

    def peek(self) -> str:
        return self.s[self.i] if self.i < len(self.s) else ""

    def space(self) -> None:
        while self.peek() and self.peek() in _WS:
            self.i += 1

    def value(self, depth: int) -> object:
        self.space()
        ch = self.peek()
        if ch == "{":
            return self.obj(depth)
        if ch == "[":
            return self.arr(depth)
        if ch == '"':
            return self.string()
        for word, val in (("true", True), ("false", False), ("null", None)):
            if self.s.startswith(word, self.i):
                self.i += len(word)
                return val
        if ch == "-" or ch.isdigit():
            return self.number()
        raise JsonReject("syntax")

    def number(self) -> int:
        start = self.i
        if self.peek() == "-":
            self.i += 1
        digits = self.i
        while self.peek().isascii() and self.peek().isdigit():
            self.i += 1
        body = self.s[digits : self.i]
        if not body or (len(body) > 1 and body[0] == "0"):
            raise JsonReject("number-form")  # no digits, or a leading zero
        if self.peek() in (".", "e", "E"):
            raise JsonReject("number-form")  # no fraction, no exponent, ever
        text = self.s[start : self.i]
        if text == "-0":
            raise JsonReject("number-form")
        if len(body) > 16 or abs(int(text)) > SAFE_MAX:
            raise JsonReject("unsafe-integer")
        return int(text)

    def string(self) -> str:
        self.i += 1
        out: list[str] = []
        while True:
            ch = self.peek()
            if not ch:
                raise JsonReject("unterminated")
            self.i += 1
            if ch == '"':
                return "".join(out)
            if ord(ch) < 0x20:
                raise JsonReject("control-char")
            if ch != "\\":
                out.append(ch)
                continue
            esc = self.peek()
            self.i += 1
            if esc in _ESCAPES:
                out.append(_ESCAPES[esc])
            elif esc == "u":
                unit = self.hex4()
                if 0xD800 <= unit <= 0xDBFF:
                    if self.s[self.i : self.i + 2] != "\\u":
                        raise JsonReject("unpaired-surrogate")
                    self.i += 2
                    low = self.hex4()
                    if not 0xDC00 <= low <= 0xDFFF:
                        raise JsonReject("unpaired-surrogate")
                    out.append(chr(0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)))
                elif 0xDC00 <= unit <= 0xDFFF:
                    raise JsonReject("unpaired-surrogate")
                else:
                    out.append(chr(unit))
            else:
                raise JsonReject("bad-escape")

    def hex4(self) -> int:
        digits = self.s[self.i : self.i + 4]
        if len(digits) != 4 or any(d not in _HEXDIGITS for d in digits):
            raise JsonReject("bad-escape")
        self.i += 4
        return int(digits, 16)

    def arr(self, depth: int) -> list:
        if depth >= MAX_JSON_DEPTH:
            raise JsonReject("depth")
        self.i += 1
        items: list = []
        self.space()
        if self.peek() == "]":
            self.i += 1
            return items
        while True:
            items.append(self.value(depth + 1))
            self.space()
            ch = self.peek()
            self.i += 1
            if ch == "]":
                return items
            if ch != ",":
                raise JsonReject("syntax")

    def obj(self, depth: int) -> dict:
        if depth >= MAX_JSON_DEPTH:
            raise JsonReject("depth")
        self.i += 1
        out: dict = {}
        self.space()
        if self.peek() == "}":
            self.i += 1
            return out
        while True:
            self.space()
            if self.peek() != '"':
                raise JsonReject("syntax")
            key = self.string()
            if key in out:
                raise JsonReject("duplicate-key")  # compared after unescaping
            self.space()
            if self.peek() != ":":
                raise JsonReject("syntax")
            self.i += 1
            out[key] = self.value(depth + 1)
            self.space()
            ch = self.peek()
            self.i += 1
            if ch == "}":
                return out
            if ch != ",":
                raise JsonReject("syntax")


def read_json(data: bytes) -> object:
    """Parse ``data`` under the VER04i 9.1 reader profile, or raise :class:`JsonReject`."""
    try:
        text = data.decode("utf-8")  # strict: refuses overlong forms, surrogates, > U+10FFFF
    except UnicodeDecodeError as exc:
        raise JsonReject("utf8") from exc
    if text.startswith("﻿"):
        raise JsonReject("bom")
    reader = _Reader(text)
    value = reader.value(0)
    reader.space()
    if reader.peek():
        raise JsonReject("trailing-data")
    return value


# ================================================================================================
# 2. Type text (section 2.1) and JSON values decoded by declared type (section 2.3)
# ================================================================================================

_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_WIDTH = re.compile(r"0|[1-9][0-9]*")
_IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_HEX = re.compile(r"(?:[0-9a-f]{2})*")
_PRIMITIVE = frozenset({"Bool", "Nat", "Bytes", *FIXED})


def parse_type(text: object, types: dict | None, *, allow_int: bool = False, widths: frozenset = frozenset()) -> A.Ty:
    """The canonical type text of section 2.1 as a :class:`.ast.Ty`, or :class:`Refused` ``("type")``.

    ``types`` is the section 2.2 table (a name must be in it); ``None`` is the *open* mode of
    ``v2-domain``, which has no ``types`` field, where only the error name of a ``Result`` is free.
    ``Int`` exists inside the IR only and is refused as a value type unless ``allow_int`` (an IR
    ``lit``).  ``widths`` are the ``width_params`` of an op row (section 6, R70): the one place a *named* width
    may be written, ``Bytes<N>`` with ``N`` one of them (kept as ``Ty("Bytes", ("N",))``, which is what the
    oracle's lowering produces); everywhere else a width is a decimal literal.  The depth of a type expression is at most 16 (section 2.2 rule 1): a bare name is depth 0
    (the generator's ``resolve`` counts the same way) and every ``(``, ``List<``, ``Option<`` or
    ``Result<`` wrapper adds one, so 16 nested ``Result`` wrappers around ``Bool`` are accepted and 17 refused.
    """
    if not isinstance(text, str):
        raise Refused("type", "a type is text")
    pos = 0

    def name() -> str:
        nonlocal pos
        m = _NAME.match(text, pos)
        if not m:
            raise Refused("type", "a type name was expected")
        pos = m.end()
        return m.group(0)

    def expect(ch: str) -> None:
        nonlocal pos
        if text[pos : pos + 1] != ch:
            raise Refused("type", f"{ch!r} was expected")
        pos += 1

    def one(level: int) -> A.Ty:
        nonlocal pos
        if level > MAX_TYPE_DEPTH:
            raise Refused("type", "a type deeper than 16")
        if text[pos : pos + 1] == "(":
            pos += 1
            parts = [one(level + 1)]
            while text[pos : pos + 1] == ",":
                pos += 1
                parts.append(one(level + 1))
            expect(")")
            if not 2 <= len(parts) <= 8:
                raise Refused("type", "a tuple has 2 to 8 components")
            return A.Ty("()", tuple(parts))
        head = name()
        if head == "Bytes" and text[pos : pos + 1] == "<":
            pos += 1
            named = _NAME.match(text, pos)
            if named and named.group(0) in widths:
                pos = named.end()
                expect(">")
                return A.Ty("Bytes", (named.group(0),))
            m = _WIDTH.match(text, pos)
            if not m:
                raise Refused("type", "a Bytes width is a decimal literal")
            pos = m.end()
            expect(">")
            if int(m.group(0)) > MAX_FIXED_WIDTH:
                raise Refused("type", "Bytes<N> with N over 4096")
            return A.Ty("Bytes", (int(m.group(0)),))
        if head in ("List", "Option"):
            expect("<")
            inner = one(level + 1)
            expect(">")
            return A.Ty(head, (inner,))
        if head == "Result":
            expect("<")
            ok = one(level + 1)
            expect(",")
            err = name()
            expect(">")
            if types is not None and (err not in types or not isinstance(types[err], dict) or types[err].get("kind") != "error"):
                raise Refused("type", "the error type of a Result is a declared error type")
            return A.Ty("Result", (ok, A.Ty(err)))
        if head == "Int":
            if not allow_int:
                raise Refused("type", "Int is an IR-only type")
            return A.Ty("Int")
        if head in _PRIMITIVE:
            return A.Ty(head)
        if types is not None and head in types:
            row = types[head]
            if isinstance(row, dict) and row.get("kind") == "error":
                # R117 (VER04r 16.22): an error type is the second argument of a ``Result`` and nothing else.
                raise Refused("type", "an error type is not a value type")
            return A.Ty(head)
        raise Refused("type", "not a type of section 2.1, or not in types")

    ty = one(0)
    if pos != len(text):
        raise Refused("type", "trailing text after a type")
    return ty


def type_levels(ty: A.Ty, types: dict | None = None) -> set[str]:
    """The levels a type brings (R9, R69 b): ``ir.tuple`` for a tuple, ``ir.data`` for an option, a record or a
    variant, and the reserved ``ir.list`` for ``List<T>`` (the document-level oracle refuses a list whole, the kit
    names its level)."""
    names = {n for n, row in (types or {}).items() if row.get("kind") in ("record", "variant")}
    found = L.type_levels(ty, frozenset(names))
    stack = [ty]
    while stack:
        t = stack.pop()
        if isinstance(t, A.Ty):
            if t.head == "Option":
                found.add(L.DATA)
            if t.head == "List":
                found.add(L.LIST)
            stack.extend(a for a in t.args if isinstance(a, A.Ty))
    return found


def check_type_table(types: object, code: str) -> None:
    """R108, R110 (a), R117 (VER04r 16.21, 16.22): refuse a ``types`` table a consumer must not read, whole, with ``code``.

    Section 2.2 holds "at generation and again by every consumer", so the table is read whole and first, before
    any value, descriptor, row or expression.  Refused, each with the context's own code (``type`` in
    ``v2-value``, ``corpus`` in ``v2-chunk`` and a corpus, ``ill-typed`` in ``v2-ir`` and ``v2-fault``):

    * a record or variant that contains itself, directly or through any chain (through ``Option``, ``Result``,
      a tuple or a list; a mutually recursive pair is a chain), rule 1 and R118;
    * a repeated field or constructor name (rule 4), a name outside ``[A-Za-z_][A-Za-z0-9_]*``;
    * a row that is not an object, of a kind other than ``error``, ``record`` or ``variant``, or that is
      malformed, or a count outside rule 2 (1 to 64 distinct cases) or rule 3 (1 to 32 fields; 2 to 64
      constructors of 0 to 8 fields);
    * a field or constructor component whose type text the type parser refuses (malformed, deeper than 16, ``Int``,
      ``Bytes<N>`` over 4096, a name the table does not declare, ``Result<T,Q>`` with ``Q`` not an error type);
    * R117: a component that names an ``error`` row anywhere except as the second argument of ``Result``.

    The walk is iterative, so a hostile table cannot make it recurse.
    """
    if not isinstance(types, dict):
        raise Refused(code, "the types table is not an object")
    refs: dict[str, set[str]] = {}
    for name, row in types.items():
        if not isinstance(row, dict):
            raise Refused(code, "a types row is not an object")
        kind = row.get("kind")
        if kind == "error":
            cases = row.get("cases")
            if (
                not isinstance(cases, list)
                or not 1 <= len(cases) <= 64
                or not all(isinstance(c, str) for c in cases)
                or len(set(cases)) != len(cases)
            ):
                raise Refused(code, "an error type has 1 to 64 distinct case names (section 2.2 rule 2)")
            continue
        if kind not in ("record", "variant"):
            raise Refused(code, "a types row is neither an error, a record nor a variant")
        try:
            if kind == "record":
                rows = row["fields"]
                if not isinstance(rows, list) or not 1 <= len(rows) <= 32:
                    raise Refused(code, "a record has 1 to 32 fields (section 2.2 rule 3)")
                names = [f[0] for f in rows]
                texts = [f[1] for f in rows]
                if any(len(f) != 2 for f in rows):
                    raise TypeError
            else:
                rows = row["ctors"]
                if not isinstance(rows, list) or not 2 <= len(rows) <= 64:
                    raise Refused(code, "a variant has 2 to 64 constructors (section 2.2 rule 3)")
                if any(len(c) != 2 or not isinstance(c[1], list) or len(c[1]) > 8 for c in rows):
                    raise Refused(code, "a constructor has 0 to 8 fields (section 2.2 rule 3)")
                names = [c[0] for c in rows]
                texts = [t for c in rows for t in c[1]]
            repeated = len(set(names)) != len(names)  # an unhashable name is a malformed row
        except (KeyError, IndexError, TypeError):
            raise Refused(code, "a types row is malformed") from None
        if not all(isinstance(n, str) and _IDENT.fullmatch(n) for n in names):
            raise Refused(code, "a field or constructor name is outside [A-Za-z_][A-Za-z0-9_]* (section 2.2 rule 4)")
        if repeated:
            raise Refused(code, "a field or constructor name is repeated (section 2.2 rule 4)")
        used: set[str] = set()
        for text in texts:
            try:
                stack = [parse_type(text, types)]
            except Refused:
                raise Refused(code, "a field names a type the table does not declare, or is not a value type") from None
            while stack:
                t = stack.pop()
                if isinstance(t, A.Ty):
                    target = types.get(t.head)
                    if isinstance(target, dict) and target.get("kind") in ("record", "variant"):
                        used.add(t.head)
                    # ``parse_type`` has refused an error name that is not a Result's second argument (R117);
                    # that second argument is the only error name a parsed type holds, and is not walked.
                    stack.extend(t.args[:1] if t.head == "Result" else t.args)
        refs[name] = used
    state: dict[str, int] = {}  # absent: unvisited, 1: on the path, 2: done
    for start in refs:
        if start in state:
            continue
        path = [(start, iter(sorted(refs[start])))]
        state[start] = 1
        while path:
            node, it = path[-1]
            for nxt in it:
                if state.get(nxt) == 1:
                    raise Refused(code, "a record or variant contains itself (section 2.2 rule 1)")
                if nxt not in state:
                    state[nxt] = 1
                    path.append((nxt, iter(sorted(refs[nxt]))))
                    break
            else:
                state[node] = 2
                path.pop()


def _record_fields(types: dict, name: str) -> list[A.Ty]:
    return [parse_type(t, types) for _, t in types[name]["fields"]]


def decode_value(raw: object, ty: A.Ty, types: dict) -> object:
    """Decode the JSON value ``raw`` as ``ty`` (section 2.3) into the oracle's Python values.

    The reader holds the declared type and decodes to it, range-checking; a value that does not fit
    is :class:`Refused` ``("value")``, never a truncation.  The result shapes are the oracle's: ``bool``,
    ``int``, ``bytes``, tuples (tuples and records), ``("ok", v)`` / ``("err", case)``,
    ``("none",)`` / ``("some", v)``, ``("list", (..))`` and ``(ctor, (fields..))``.

    A lane decodes **every** form of section 2.3; the S5 tables only use the ``ir.core`` ones, and the
    case's ``requires`` (:func:`type_levels`) is what keeps a tuple or an ``Option`` out of the kit.
    """
    head = ty.head

    def refuse(why: str) -> Refused:
        return Refused("value", why)

    if head == "Bool":
        if type(raw) is not bool:
            raise refuse("a Bool is a JSON boolean")
        return raw
    if head == "Nat" or head in FIXED:
        try:
            return E.decode_int_text(raw, head)  # digits only, no +, no leading zero, <= 20 digits, range
        except OracleError as exc:
            raise refuse(exc.message) from exc
    if head == "Bytes":
        if not isinstance(raw, str) or not _HEX.fullmatch(raw):
            raise refuse("Bytes is lowercase even-length hex")
        n = len(raw) // 2
        if ty.args and n != ty.args[0]:
            raise refuse("Bytes<N> has exactly N bytes")
        if n > MAX_FIXED_WIDTH:
            raise refuse("a Bytes value has at most 4096 bytes")
        return bytes.fromhex(raw)
    if head == "Result":
        if not (isinstance(raw, dict) and len(raw) == 1):
            raise refuse("a Result has exactly one key")
        if "ok" in raw:
            return ("ok", decode_value(raw["ok"], ty.args[0], types))
        cases = types.get(ty.args[1].head, {}).get("cases") if types else None
        case = raw.get("err")
        if not isinstance(case, str) or (cases is not None and case not in cases):
            raise refuse("an err case is a name declared in the error type")
        return ("err", case)
    if head == "()":
        parts = ty.args
    elif head in types and types[head].get("kind") == "record":
        parts = tuple(_record_fields(types, head))
    else:
        parts = None
    if parts is not None:
        if not isinstance(raw, list) or len(raw) != len(parts):
            raise refuse("a tuple or record is an array of its arity")
        return tuple(decode_value(x, t, types) for x, t in zip(raw, parts))
    if head == "List":
        if not isinstance(raw, list) or len(raw) > MAX_FIXED_WIDTH:
            raise refuse("a List is an array of at most 4096 elements")
        return ("list", tuple(decode_value(x, ty.args[0], types) for x in raw))
    if head == "Option":
        if isinstance(raw, dict) and raw == {"none": True}:
            return ("none",)
        if isinstance(raw, dict) and len(raw) == 1 and "some" in raw:
            return ("some", decode_value(raw["some"], ty.args[0], types))
        raise refuse("an Option is {none: true} or {some: v}")
    if head in types and types[head].get("kind") == "variant":
        if not (isinstance(raw, dict) and set(raw) == {"ctor", "fields"}):
            raise refuse("a variant has exactly the keys ctor and fields")
        ctors = {c: fs for c, fs in types[head]["ctors"]}
        if raw["ctor"] not in ctors or not isinstance(raw["fields"], list):
            raise refuse("an undeclared constructor")
        declared = [parse_type(t, types) for t in ctors[raw["ctor"]]]
        if len(declared) != len(raw["fields"]):
            raise refuse("a constructor's field count")
        return (raw["ctor"], tuple(decode_value(x, t, types) for x, t in zip(raw["fields"], declared)))
    raise refuse("a value of this type cannot be decoded")


def _encoder_spec(types: dict) -> _pytypes.SimpleNamespace:
    """The slice of a lowered ``Spec`` that :func:`.stream.make_encoder` reads (records, variants)."""
    records = {
        n: [(f, parse_type(t, types)) for f, t in row["fields"]]
        for n, row in types.items()
        if row.get("kind") == "record"
    }
    variants = {
        n: [(c, [(f"f{i}", parse_type(t, types)) for i, t in enumerate(fs)]) for c, fs in row["ctors"]]
        for n, row in types.items()
        if row.get("kind") == "variant"
    }
    return _pytypes.SimpleNamespace(records=records, variants=variants, errors={})


def encode_value(value: object, ty: A.Ty, types: dict) -> bytes:
    """The section 2.4 bytes of one value: no header, no context prefix (R26)."""
    return S.make_encoder(_encoder_spec(types), ty)(value)


def value_to_json(value: object, ty: A.Ty) -> object:
    """The untagged JSON of a Python value of type ``ty`` (section 2.3), the inverse of :func:`decode_value`.

    Unlike :func:`.corpus_v2.value_json` this also writes ``Int`` (a decimal string, which only an IR
    intermediate or a ``lit`` may have) and refuses the unknown error type ``?`` of a bare ``ok``.
    """
    head = ty.head
    if head == "Bool":
        return bool(value)
    if head in FIXED or head in ("Nat", "Int"):
        return str(value)
    if head == "Bytes":
        return value.hex()
    if head == "()":
        return [value_to_json(x, t) for x, t in zip(value, ty.args)]
    if head == "Result":
        if value[0] == "ok":
            return {"ok": value_to_json(value[1], ty.args[0])}
        return {"err": value[1]}
    raise KitError(f"a value of type {ty} has no JSON writer in the S5 kit")


# ================================================================================================
# 3. v2-value and v2-arith
# ================================================================================================


def derive_value(case: dict) -> dict:
    """``{"outcome":"accept","stream_hex":X,"stream_sha256":D}`` or a ``reject`` (section 11).

    The order is the one of the code table: ``malformed-json`` (the reader refuses the bytes of
    ``input_hex``), then ``type`` (section 2.1/2.2), then ``value`` (section 2.3).
    """
    types = case["types"]
    try:
        raw = read_json(bytes.fromhex(case["input_hex"]))
    except JsonReject:
        return {"outcome": "reject", "error": "malformed-json"}
    try:
        check_type_table(types, "type")  # R108: the whole table, before the type or any value is read
        ty = parse_type(case["type"], types)
    except Refused as r:
        return {"outcome": "reject", "error": r.code}
    try:
        value = decode_value(raw, ty, types)
    except Refused as r:
        return {"outcome": "reject", "error": r.code}
    stream = encode_value(value, ty, types)
    out = {"outcome": "accept", "stream_sha256": hashlib.sha256(stream).hexdigest()}
    if len(stream.hex()) <= STREAM_HEX_LIMIT:
        out["stream_hex"] = stream.hex()
    return out


ARITH_OPS = ("add", "sub", "mul", "neg", "eq", "ne", "lt", "gt", "le", "ge")


def derive_arith(case: dict) -> dict:
    """The one evaluator node of ``case["op"]`` over ``lit`` operands: ``{"type","value"}`` or ``ir-overflow``.

    ``type`` is the type of **both** operands, so a ``Nat``/``Int`` mix is a ``v2-ir`` case.  The result
    type is the evaluator's: ``Int`` for ``Nat - Nat`` (section 3.2) and for every other exact node
    (``Nat + Nat`` is ``Int`` too, section 8.2's typing note), ``Bool`` (a JSON boolean) for a comparison.
    """
    ty, op = case["type"], case["op"]
    if op not in ARITH_OPS:
        raise KitError(f"unknown arithmetic op {op!r}")
    operands = [case["lhs"]] if op == "neg" else [case["lhs"], case["rhs"]]
    ir = [op, *(["lit", {"type": ty, "value": text}] for text in operands)]
    try:
        node = E.Program({}, {}, analyze_data=False).compile(ir, {}, False)
    except OracleError as exc:
        raise KitError(f"the oracle refuses the node: {exc}") from exc
    try:
        value = node.fn({})
    except OracleError as exc:
        if exc.code == "ir-overflow":
            return {"error": "ir-overflow"}
        raise KitError(f"unexpected oracle failure: {exc}") from exc
    return {"type": str(node.ty), "value": value if isinstance(value, bool) else str(value)}


# ================================================================================================
# 4. v2-ir: the IR evaluator with model stand-ins, and the pure call trace
# ================================================================================================

_STATIC_ORDER = ("unknown-tag", "level-missing", "model-order", "subject-forbidden")


def _nodes(ir: object):
    """Every IR node of a tree, pre-order (``match`` patterns are never visited, :func:`.irtree.children`)."""
    stack = [ir]
    while stack:
        node = stack.pop()
        if not is_node(node):
            continue
        yield node
        try:
            stack.extend(reversed(children(node)))
        except (IndexError, TypeError, OracleError):  # a malformed node: the oracle will name it
            continue


def reach(case: dict) -> tuple[list, list[str], list[str]]:
    """What the checked expression reaches (section 16.18 R69 a, g): ``(trees, models, ops)``.

    ``ir`` itself, then every model in the call closure of the trees it holds (a ``model_call``; the stand-in
    that ``subjects`` names for an op is *not* followed, R69 a names the closure of ``ir`` and the kit pins
    nothing about a stand-in's own tags), each once and in first-reached order, and every op a ``subject_op``
    names.  ``trees`` is ``ir``
    followed by the bodies of those models.  A model, an op or a ``types`` row that nothing here names is not
    reached and causes no ``level-missing`` and no ``requires`` (R69 g).
    """
    models: list[str] = []
    ops: list[str] = []
    trees = [case["ir"]]
    i = 0
    while i < len(trees):
        for node in _nodes(trees[i]):
            called = None
            if node[0] == "model_call" and len(node) > 1:
                called = node[1]
            elif node[0] == "subject_op" and len(node) > 1 and node[1] not in ops:
                ops.append(node[1])
            if isinstance(called, str) and called in case["models"] and called not in models:
                models.append(called)
                trees.append(case["models"][called]["body"])
        i += 1
    return trees, models, [o for o in ops if o in case["ops"]]


def reached_levels(case: dict, trees: list, models: list[str], ops: list[str]) -> set[str]:
    """The levels of the tags of the reached trees and of the types the case reaches (R69 b).

    Types: the ``env`` entries, the parameters of the reached models, the parameters and result of the
    reached ops (a named width of an op's ``width_params`` is plain ``Bytes`` for the level).  A type text
    that does not parse brings no level here: it is the oracle's ``ill-typed`` or ``type`` to name.
    """
    found: set[str] = set()
    for ir in trees:
        found |= set(L.ir_levels(ir, follow=False)) if is_node(ir) else set()
    texts = [(tagged["type"], frozenset(), False) for _name, tagged in case["env"]]
    for name in models:
        # [D75]: a model parameter may be ``Int`` (nothing else may), so its level is read with ``allow_int``.
        texts += [(t, frozenset(), True) for _, t in case["models"][name]["params"]]
    for name in ops:
        row = case["ops"][name]
        widths = frozenset(row.get("width_params", ()))
        texts += [(t, widths, False) for _, t in [*row["params"], ("ret", row["ret"])]]
    for text, widths, int_ok in texts:
        try:
            found |= type_levels(parse_type(text, case["types"], allow_int=int_ok, widths=widths), case["types"])
        except Refused:
            continue
    return found


def static_refusal(case: dict) -> Refused | None:
    """The four *structural* static codes of ``v2-ir``, found before the oracle compiles anything.

    * ``unknown-tag``: a tag of no level this document defines (:func:`.levels.tag_level`), **wherever it
      stands**, in a model that nothing calls too (section 4.2 rule 7);
    * ``level-missing``: a tag **or a type** of a defined level that is not in the case's ``levels``, decided
      over what ``ir`` reaches (:func:`reach`, section 16.18 R69): a model, an op or a ``types`` row that
      ``ir`` does not reach causes none;
    * ``model-order``: a ``model_call`` to a model that is missing from ``models`` or ``model_order``,
      or is not earlier in ``model_order`` than its caller (section 4.2 rule 1; the order is the
      ``model_order`` array, never the key order of ``models``);
    * ``subject-forbidden``: a ``subject_op`` in a model body, or anywhere in a case whose ``role`` is
      ``admit`` or ``fault`` (rule 3).

    ``match`` is ``ir.tuple`` (section 4.1 [D35]).  No order among these is normative; the tables never
    put two in one case.
    """
    levels = set(case["levels"])
    order = case["model_order"]
    where = {n: i for i, n in enumerate(order)}
    bodies = [(None, case["ir"])] + [(n, case["models"][n]["body"]) for n in order if n in case["models"]]
    for _owner, ir in bodies:
        for node in _nodes(ir):
            tag = node[0]
            if tag != "match" and L.tag_level(tag) is None:
                return Refused("unknown-tag", tag)
    trees, models, ops = reach(case)
    for ir in trees:
        for node in _nodes(ir):
            tag = node[0]
            level = L.TUPLE if tag == "match" else L.tag_level(tag)
            if level not in levels:
                return Refused("level-missing", tag)
    for level in sorted(reached_levels(case, trees, models, ops) - levels):
        return Refused("level-missing", f"a type of {level}")
    for owner, ir in bodies:
        for node in _nodes(ir):
            if node[0] == "subject_op" and (owner is not None or case.get("role", "body") != "body"):
                return Refused("subject-forbidden", node[1] if len(node) > 1 else "")
            if node[0] == "model_call":
                callee = node[1] if len(node) > 1 else None
                here = len(order) if owner is None else where[owner]
                if callee not in case["models"] or callee not in where or where[callee] >= here:
                    return Refused("model-order", str(callee))
    return None


def undeclared_err_case(case: dict) -> Refused | None:
    """``ill-typed`` for an ``err`` node whose case no error type in ``types`` declares (section 16.15 R41).

    The oracle's own ``err`` compile types the node as ``Result<?,?>`` without looking at ``types``, so the
    derivation layer makes the check: the case name must be in the ``cases`` of some ``error`` row.
    """
    declared = {c for row in case["types"].values() if row.get("kind") == "error" for c in row["cases"]}
    for ir in [case["ir"], *(case["models"][n]["body"] for n in case["model_order"] if n in case["models"])]:
        for node in _nodes(ir):
            if node[0] == "err" and len(node) == 2 and isinstance(node[1], str) and node[1] not in declared:
                return Refused("ill-typed", f"err({node[1]}) names a case no error type declares")
    return None


def classify_ir_refusal(exc: OracleError, levels: set[str]) -> Refused:
    """Map one oracle failure to the kit's static code (section 16.13 R25), or fail the build.

    ===========================  ==========================================  =============
    oracle                       condition                                   kit code
    ===========================  ==========================================  =============
    ``generation``               a subject call under an impure guard        impure-guard
    ``limit``                    ``ir-cost:`` / the ``max_ir_cost`` ceiling  ir-cost
    ``limit``                    IR nesting or model frames past 64          depth
    ``level``                    a level that ``levels`` lacks               level-missing
    ``ir``, ``type``             every other static type error               ill-typed
    ===========================  ==========================================  =============

    Anything else (``corpus``, another ``limit``) is outside the S5 kit: :class:`KitError`.
    """
    msg = exc.message
    if exc.code == "generation" and "impure guard" in msg:
        return Refused("impure-guard", msg)
    if exc.code == "limit" and (msg.startswith("ir-cost") or "max_ir_cost" in msg):
        return Refused("ir-cost", msg)
    if exc.code == "limit" and "nest" in msg:
        return Refused("depth", msg)
    if exc.code == "level" and exc.level is not None and exc.level not in levels:
        return Refused("level-missing", msg)
    if exc.code in ("ir", "type"):
        return Refused("ill-typed", msg)
    raise KitError(f"an oracle failure with no kit code: {exc}")


@dataclass
class IrCase:
    """A ``v2-ir`` case decoded and compiled: the oracle program, the typed tree and the environment."""

    prog: E.Program
    tenv: dict
    env: dict
    root: E.Compiled
    ir: object
    subjects: dict = field(default_factory=dict)


def _ops(case: dict) -> dict:
    out = {}
    for name, row in case["ops"].items():
        out[name] = op_sig(name, row, case["types"])
    return out


def order_of(case: dict) -> list[str]:
    """The models to compile, in ``model_order`` (a model that ``model_order`` omits is never built)."""
    return [n for n in case["model_order"] if n in case["models"]]


def _models(case: dict, ops: dict, max_cost: int) -> E.Program:
    """The program of the case's models, built in ``model_order``.

    A corpus model row is ``{"params", "body"}``: there is no declared return type, so a model's
    return type is the type of its body.  Each model is compiled as soon as it is added, so a static
    error in a body is found here, and a model that calls a later one cannot be built.
    """
    errors = {n: tuple(row["cases"]) for n, row in case["types"].items() if row.get("kind") == "error"}
    # ``ir.data`` is evaluated only for a case that declares it (slice S11c-1 opened it in the evaluator; S11a's
    # evaluator refused it for every case).  A case that does not is refused by the level, as before.
    data = L.DATA in set(case["levels"])
    defs: dict = {}
    for name in case["model_order"]:
        row = case["models"][name]
        # [D75] (VER04r 16.22): a model parameter may have the type ``Int`` (and, R111 (c), ``Int`` under ``Option``,
        # ``Result``, a tuple or ``List``); an op row, an ``env`` entry, a field and every value still refuse it.
        params = [(p, parse_type(t, case["types"], allow_int=True)) for p, t in row["params"]]
        probe = E.Program(dict(defs), ops, max_cost=max_cost, errors=errors, analyze_data=data)
        ret = probe.compile(row["body"], dict(params), False).ty
        defs[name] = E.ModelDef(name, params, ret, row["body"])
    prog = E.Program(defs, ops, max_cost=max_cost, errors=errors, analyze_data=data)
    # Every model is compiled *as a model* once more, called or not (R77, section 14): the probe above types a
    # body as an expression, so it never computed the depth of the model itself.  The depth of a model is 1 plus
    # the largest depth of a model its body calls, and over 64 is refused for a chain nothing calls too (the
    # oracle counted frames from the caller before S5e, so it accepted 65 uncalled models and refused 66).
    for name in order_of(case):
        prog.model(name)
    return prog


def _rewrite_subjects(ir: object, subjects: dict) -> object:
    """Each ``["subject_op", name, args]`` becomes ``["model_call", subjects[name], args]`` (section 11)."""
    if not is_node(ir):
        return ir
    if ir[0] == "subject_op":
        if ir[1] not in subjects:
            raise KitError(f"no stand-in model for the op {ir[1]!r} in subjects")
        return ["model_call", subjects[ir[1]], [_rewrite_subjects(a, subjects) for a in ir[2]]]
    if ir[0] == "model_call":
        return ["model_call", ir[1], [_rewrite_subjects(a, subjects) for a in ir[2]]]
    if ir[0] == "tuple":
        return ["tuple", [_rewrite_subjects(a, subjects) for a in ir[1]]]
    return [ir[0], *(_rewrite_subjects(c, subjects) if isinstance(c, list) else c for c in ir[1:])]


def calls_for(prog: E.Program, ir: object, tenv: dict) -> object:
    """The subject calls per case, from the **unrewritten** tree, by the pure trace of section 4.4.

    ``int`` when static, else a closure over the environment.  This is the *hook* the mutant test
    patches ("``and`` and ``if`` evaluate both sides, calls counted").
    """
    return E.compile_calls(prog, ir, tenv)


def compile_ir_case(case: dict) -> IrCase:
    """Decode, type and compile a ``v2-ir`` case, or raise :class:`Refused` with the kit's static code.

    The order is R115 (b)'s: the bound on ``max_ir_cost`` (one integer of the case, no table needed), then the
    ``types`` table, read whole (R108, R117: a recursive record or variant, a repeated name, an undeclared or error
    field type is ``ill-typed`` whether or not the case reaches it), then everything else.
    """
    max_cost = case.get("max_ir_cost", E.MAX_IR_COST)
    if type(max_cost) is not int or not E.MAX_IR_COST <= max_cost <= 65_536:
        raise Refused("ir-cost", "max_ir_cost outside 2,000..=65,536")
    check_type_table(case["types"], "ill-typed")
    pre = static_refusal(case)
    if pre is not None:
        raise pre
    pre = undeclared_err_case(case)
    if pre is not None:
        raise pre
    levels = set(case["levels"])
    try:
        try:
            ops = _ops(case)
            prog = _models(case, ops, max_cost)
        except Refused as exc:
            # R111 (c): an op row or a model parameter whose type text the reader refuses (``Int`` in an op row, an
            # undeclared name) is ``ill-typed`` in the lanes; ``type`` is not in the closed set of ``v2-ir``.
            if exc.code != "type":
                raise
            raise Refused("ill-typed", exc.why) from exc
        env, tenv = {}, {}
        for name, tagged in case["env"]:
            ty = parse_type(tagged["type"], case["types"])
            tenv[name] = ty
            env[name] = decode_value(tagged["value"], ty, case["types"])
        root = prog.compile(case["ir"], tenv, True)
    except OracleError as exc:
        raise classify_ir_refusal(exc, levels) from exc
    return IrCase(prog, tenv, env, root, case["ir"], case["subjects"])


def _undetermined(ty: A.Ty) -> bool:
    """Does a static type hold the unknown ``?`` (a bare ``ok``'s error type, a bare ``err``'s payload)?"""
    return ty.head == "?" or any(isinstance(a, A.Ty) and _undetermined(a) for a in ty.args)


def derive_ir(case: dict) -> dict:
    """``{"type","value","calls"}`` or ``{"error": E}`` (with ``class`` for ``ir-overflow``), section 11.

    The *value* comes from the tree with each ``subject_op`` rewritten to the stand-in model named in
    ``subjects``; ``calls`` from the unrewritten tree (:func:`calls_for`), so the two numbers come
    from different code paths.  The eight static codes are found before any node is evaluated, so
    each precedes ``ir-overflow``; ``class`` is ``corpus`` when the node that overflowed has no
    ``subject_op`` beneath it (the generator can see it) and ``case`` otherwise (section 3.2).  The
    split is read off the pure trace, :func:`.ir_eval.compile_trace`, which is what the generator
    evaluates: an overflow on the trace is the corpus's, any other is the subject's doing.
    """
    levels = set(case["levels"])
    try:
        built = compile_ir_case(case)
        calls = calls_for(built.prog, built.ir, built.tenv)  # the impure-guard refusal is static
    except Refused as r:
        return {"error": r.code}
    except OracleError as exc:
        return {"error": classify_ir_refusal(exc, levels).code}
    ops = built.prog.ops
    for op, model in built.subjects.items():
        if op not in ops:
            continue
        stand_in = built.prog.models.get(model)
        # R70: a model has no width_params, so the stand-in has plain Bytes in place of each named-width type.
        want = ([unstated(t) for _, t in ops[op].params], unstated(ops[op].ret))
        if stand_in is None or ([t for _, t in stand_in.params], stand_in.ret) != want:
            raise KitError(f"the stand-in {model!r} does not have the signature of the op {op!r}")
    rewritten = _rewrite_subjects(built.ir, built.subjects)
    # The rewritten tree is only evaluated: its cost (a stand-in's body counts as IR work, a
    # subject's does not) is not the case's cost, so it is compiled under the ceiling.
    plain = E.Program(
        built.prog.models, ops, max_cost=65_536, errors=built.prog.errors, analyze_data=built.prog.analyze_data
    )
    try:
        value = plain.compile(rewritten, built.tenv, False).fn(built.env)
        n_calls = calls(built.env) if callable(calls) else calls
    except OracleError as exc:
        if exc.code != "ir-overflow":
            raise KitError(f"unexpected oracle failure: {exc}") from exc
        trace = E.compile_trace(built.prog, built.ir, built.tenv)
        klass = "case"
        if trace is not None:
            try:
                trace(built.env)
            except OracleError as inner:
                if inner.code == "ir-overflow":
                    klass = "corpus"
        return {"error": "ir-overflow", "class": klass}
    if _undetermined(built.root.ty):
        raise KitError(f"R41: the static type {built.root.ty} of a kit case has an undetermined component")
    return {"type": str(built.root.ty), "value": value_to_json(value, built.root.ty), "calls": n_calls}


# ================================================================================================
# 5. v2-domain: a descriptor-JSON reader, then size() and nth()
# ================================================================================================


def _fits(d: dict, ty: A.Ty) -> bool:
    """Is the descriptor ``d`` one *for* ``ty``?  (Section 5.1: a descriptor that is not is ``descriptor``.)"""
    kind = d["d"]
    leaves = {
        "bool": A.Ty("Bool"),
        "u8_all": A.Ty("U8"),
        "u64_edge": A.Ty("U64"),
        "nat_0_64": A.Ty("Nat"),
        "nat_count": A.Ty("Nat"),
        "nat_shift": A.Ty("Nat"),
        "bytes_d1": A.Ty("Bytes"),
    }
    if kind in leaves:
        return ty == leaves[kind]
    if kind == "bytes_n":
        return ty == A.Ty("Bytes", (d["n"],))
    if kind in ("fx_full", "fx_edge"):
        return ty == A.Ty(d["t"])
    if kind == "result":
        return ty.head == "Result" and _fits(d["ok"], ty.args[0])
    if kind == "option":
        return ty.head == "Option" and _fits(d["of"], ty.args[0])
    if kind == "list":
        return ty.head == "List" and _fits(d["of"], ty.args[0])
    if kind == "tuple":
        return (
            ty.head == "()"
            and len(d["of"]) == len(ty.args)
            and all(_fits(p, t) for p, t in zip(d["of"], ty.args))
        )
    return ty.head not in _PRIMITIVE and ty.head not in ("Result", "Option", "List", "()")  # variant: a named type


def _build_desc(d: dict, ty: A.Ty) -> D.Desc:
    """Build the :class:`.domains.Desc` of a *validated* descriptor with the existing leaf constructors."""
    kind = d["d"]
    simple = {
        "bool": D.leaf_bool,
        "u8_all": D.leaf_u8_all,
        "u64_edge": D.leaf_u64_edge,
        "bytes_d1": D.leaf_bytes_d1,
        "nat_shift": D.leaf_nat_shift,
        "nat_0_64": lambda: D.leaf_nat_count(64),
    }
    if kind in simple:
        return simple[kind]()
    if kind == "bytes_n":
        return D.leaf_bytes_n(d["n"])
    if kind == "nat_count":
        return D.leaf_nat_count(d["k"])
    if kind == "fx_full":
        return D.leaf_fx_full(A.Ty(d["t"]))
    if kind == "fx_edge":
        return D.leaf_fx_edge(A.Ty(d["t"]))
    if kind == "result":
        return D.ResultDesc(ty, _build_desc(d["ok"], ty.args[0]), list(d["errs"]))
    if kind == "option":
        return D.OptionDesc(ty, _build_desc(d["of"], ty.args[0]))
    if kind == "list":
        return D.ListDesc(ty, _build_desc(d["of"], ty.args[0]))
    if kind == "tuple":
        return D.TupleDesc(ty, [_build_desc(p, t) for p, t in zip(d["of"], ty.args)])
    ctors = [(c, [_build_desc(f, A.Ty("?")) for f in fs]) for c, fs in d["ctors"]]
    return D.VariantDesc(ty, ctors)


def desc_from_json(d: object, ty: A.Ty) -> D.Desc:
    """The descriptor tree ``d`` (section 5.1 JSON) as a :class:`.domains.Desc` for a binder of ``ty``.

    :func:`.domains.descriptor_size` first recomputes every ``size`` and enforces the rules of the
    leaves and the depth limit of 16 (unknown ``d``, a missing or extra key, a wrong ``size``, an
    illegal ``t``); this function then checks that the tree is one **for** ``ty`` and that the built
    descriptor's own ``size()`` is the declared one.  Every refusal is :class:`Refused` ``("descriptor")``.
    """
    try:
        declared = D.descriptor_size(d)
    except OracleError as exc:
        raise Refused("descriptor", exc.message) from exc
    if not _fits(d, ty):
        raise Refused("descriptor", "a descriptor that is not one for the binder type")
    desc = _build_desc(d, ty)
    if desc.size() != declared:
        raise Refused("descriptor", "the built descriptor disagrees with its declared size")
    return desc


def schema_index(value: object, what: str) -> int:
    """A case field that must be a non-negative JSON integer; anything else is a kit error (R45 f), no outcome."""
    if type(value) is not int or value < 0:
        raise KitError(f"{what} must be a non-negative JSON integer, not {value!r}")
    return value


def derive_domain(case: dict) -> dict:
    """``{"size": S, "values_hex": [..]}`` or ``descriptor`` / ``index`` (section 11).

    The binder type is needed to encode the values.  ``index`` is one JSON integer, ``range`` is
    ``[lo, hi)`` (indices ``lo..hi-1``).  ``descriptor`` (every refusal of section 5.1) precedes
    ``index`` (``index >= size``, or a range with ``lo > hi`` or ``hi > size``).
    """
    if "index" in case:  # a schema violation is a kit error, never an outcome (R45 f)
        schema_index(case["index"], "index")
    else:
        if not isinstance(case["range"], list) or len(case["range"]) != 2:
            raise KitError("range is a pair [lo, hi]")
        for bound in case["range"]:
            schema_index(bound, "a range bound")
    try:
        ty = parse_type(case["type"], None)
        desc = desc_from_json(case["descriptor"], ty)
    except Refused as r:
        return {"error": "descriptor" if r.code in ("type", "descriptor") else r.code}
    size = desc.size()
    if "index" in case:
        i = case["index"]
        window = [i]
        ok = i < size
    else:
        lo, hi = case["range"]
        window = list(range(lo, hi)) if lo <= hi <= size else []
        ok = lo <= hi <= size
    if not ok:
        return {"error": "index"}
    encode = S.make_encoder(_encoder_spec({}), ty)
    return {"size": size, "values_hex": [encode(desc.nth(i)).hex() for i in window]}


def descriptor_levels(d: dict) -> set[str]:
    """The levels a descriptor needs: ``ir.tuple`` for ``tuple``, ``ir.data`` for ``variant``, ``option``, ``list``."""
    found = {L.CORE}
    stack = [d]
    while stack:
        node = stack.pop()
        if not isinstance(node, dict) or not isinstance(node.get("d"), str):
            continue
        if node["d"] == "tuple":
            found.add(L.TUPLE)
        elif node["d"] in ("variant", "option", "list"):
            found.add(L.DATA)
        for key in ("of", "ok"):
            sub = node.get(key)
            stack.extend(sub if isinstance(sub, list) else [sub])
    return found


# ================================================================================================
# 6. v2-chunk: the chunk-file check (sections 5.5 and 14), then one chunk enumerated alone
# ================================================================================================

CHUNK_CORPUS_KEYS = frozenset({"chunk_size", "max_ir_cost", "types", "ops", "models", "model_order"})
_ROW_BASE = frozenset(
    ["id", "kind", "claim", "ops", "requires", "binders", "demoted", "domain_label", "admit", "body", "oracle", "result_type", "stream", "product", "cases", "discarded", "expected_calls", "wraps", "ir_cost", "chunks", "chunks_sha256", "inputs_sha256", "slim_chunks", "slim_inputs_sha256"]
)
_LINE = re.compile(
    r"(0|[1-9][0-9]*) (0|[1-9][0-9]*) (0|[1-9][0-9]*) (0|[1-9][0-9]*) ([0-9a-f]{64})(?: ([0-9a-f]{64}))?"
)
MAX_CHUNKS = 4096
MAX_SLIM_CHUNKS = 32
MAX_BINDERS = 8  # section 5.3 and 14: a suite has 0 to 8 binders (R75)
MAX_SUITE_PRODUCT = 1 << 28  # section 14: ``MAX_SUITE_PRODUCT``


def _check_errs(d: dict, ty: A.Ty, types: dict) -> None:
    """R72, R103: a binder descriptor is one *for* its type and agrees with the ``types`` table, node by node.

    ``d`` and ``ty`` are walked together.  Every node must be of the kind the type needs (a ``result`` for a
    ``Result``, an ``option`` for an ``Option``, a ``list`` for a ``List``, a ``tuple`` for a tuple or a record, a
    ``variant`` for a variant, a leaf for its own type); a ``result``'s ``errs`` equals the declared ``cases`` of its
    error type, order included (R72); a ``tuple`` has exactly the component count of a tuple type or the field count
    of a record, and a ``variant``'s constructors equal the declared ones (names, order, field counts; R103), their
    fields walked against the declared field types.  Anything else, and a ``types`` row that is malformed, raises
    :class:`Refused` ``("corpus")``.
    """
    try:
        _walk_descriptor(d, ty, types)
    except (KeyError, IndexError, TypeError, ValueError, AttributeError) as exc:
        raise Refused("corpus", "a binder descriptor or its types row is malformed") from exc


def _walk_descriptor(d: dict, ty: A.Ty, types: dict) -> None:
    def bad(why: str) -> None:
        raise Refused("corpus", why)

    kind = d["d"]
    row = types.get(ty.head, {})
    if kind == "result":
        if ty.head != "Result":
            bad("a result descriptor for a type that is not a Result")
        err = types.get(ty.args[1].head, {})
        if err.get("kind") != "error" or d["errs"] != err["cases"]:
            bad("errs differs from the declared cases of the error type (R72)")
        _walk_descriptor(d["ok"], ty.args[0], types)
    elif kind in ("option", "list"):
        if ty.head != kind.capitalize():
            bad(f"a {kind} descriptor for a type that is not a {kind.capitalize()}")
        _walk_descriptor(d["of"], ty.args[0], types)
    elif kind == "tuple":
        if ty.head == "()":
            parts = list(ty.args)
        elif row.get("kind") == "record":
            parts = [parse_type(text, types) for _, text in row["fields"]]
        else:
            bad("a tuple descriptor for a type that is neither a tuple nor a record")
        if len(d["of"]) != len(parts):
            bad("a tuple descriptor's arity is not the component or field count of its type (R103)")
        for part, sub in zip(d["of"], parts):
            _walk_descriptor(part, sub, types)
    elif kind == "variant":
        if row.get("kind") != "variant":
            bad("a variant descriptor for a type that is not a declared variant")
        declared = [(name, list(texts)) for name, texts in row["ctors"]]
        if [(c[0], len(c[1])) for c in d["ctors"]] != [(n, len(t)) for n, t in declared]:
            bad("ctors differs from the declared constructors of the variant (R103)")
        for (_, fields), (_, texts) in zip(d["ctors"], declared):
            for part, text in zip(fields, texts):
                _walk_descriptor(part, parse_type(text, types), types)
    elif not _fits(d, ty):
        bad("a descriptor that is not one for the binder type")


def row_keys(row: dict) -> frozenset:
    """The exact key set of a suite row: the base keys, the reference digests, the v1 whole-suite digests."""
    keys = set(_ROW_BASE)
    reference = isinstance(row.get("oracle"), dict) and row["oracle"].get("kind") == "reference"
    if reference:
        keys |= {"expected_sha256", "slim_expected_sha256"}
    if str(row.get("stream", "")).startswith("warrant-stream-v1"):
        keys.add("v1_inputs_sha256")
        if reference:
            keys.add("v1_expected_sha256")
    return frozenset(keys)


def advance_admitted(running: int, line_cases: int, chunk_size: int) -> int:
    """The running prefix sum of admitted cases after one more chunk (``first_admitted`` of the next).

    A chunk's ``first_admitted`` is the number of **admitted** cases in the earlier chunks, never a
    product ordinal (chunk boundaries are on product ordinals, section 5.4).  This is the *hook* the
    mutant test patches ("``first_admitted`` counted on product ordinals").
    """
    return running + line_cases


def check_corpus_and_row(corpus: object, row: object, chunk: object) -> tuple[int, int]:
    """Validate the ``corpus`` and ``suite`` fields of a ``v2-chunk`` case: ``(m, chunk_size)`` or ``corpus``.

    ``corpus`` is refused for an unknown or missing key, an invalid ``chunk_size`` (a power of two in
    ``2^10..2^16``), a wrong descriptor ``size``, a wrong ``product``, a wrong chunk count, and
    ``chunk >= m`` (section 11, code ``corpus``).
    """
    if not isinstance(corpus, dict) or set(corpus) != CHUNK_CORPUS_KEYS:
        raise Refused("corpus", "the corpus part has the wrong keys")
    check_type_table(corpus["types"], "corpus")  # R108, before any descriptor is read
    size = schema_index(corpus["chunk_size"], "chunk_size")
    if size & (size - 1) or not (1 << 10) <= size <= (1 << 16):
        raise Refused("corpus", "chunk_size is a power of two in 2^10..2^16")
    cost = corpus["max_ir_cost"]
    if type(cost) is not int or not 2000 <= cost <= 65_536:
        raise Refused("corpus", "max_ir_cost is in 2,000..=65,536")
    if not isinstance(row, dict) or set(row) != row_keys(row):
        raise Refused("corpus", "the suite row has an unknown or missing key")
    if not isinstance(row["binders"], list) or len(row["binders"]) > MAX_BINDERS:
        raise Refused("corpus", "a suite has 0 to 8 binders (R75)")
    product = 1
    try:
        for binder in row["binders"]:
            product *= D.descriptor_size(binder[1]["domain"])
            _check_errs(binder[1]["domain"], parse_type(binder[1]["type"], corpus["types"]), corpus["types"])
    except (OracleError, Refused, KeyError, IndexError, TypeError) as exc:
        raise Refused("corpus", "a binder descriptor is wrong") from exc
    if product > MAX_SUITE_PRODUCT:
        raise Refused("corpus", "a suite product over 2^28 (R75)")
    if row["product"] != product:
        raise Refused("corpus", "product is not the product of the descriptor sizes")
    m = max(1, -(-product // size))
    if m > MAX_CHUNKS or row["chunks"] != m:
        raise Refused("corpus", "chunks is not ceil(product / chunk_size)")
    if chunk >= m:
        raise Refused("corpus", "chunk >= m")
    return m, size


def check_slim_chunks(row: dict, m: int) -> None:
    """``slim-chunks``: not ascending and duplicate-free, an index ``>= m``, chunk 0 or ``m-1`` omitted, over 32."""
    slim = row["slim_chunks"]
    for entry in slim if isinstance(slim, list) else []:
        schema_index(entry, "a slim_chunks entry")  # negative or non-integer: a kit error (R45 f)
    if (
        not isinstance(slim, list)
        or slim != sorted(set(slim))
        or len(slim) > MAX_SLIM_CHUNKS
        or not slim
        or slim[-1] >= m
        or slim[0] != 0
        or slim[-1] != m - 1
    ):
        raise Refused("slim-chunks", "slim_chunks is not a valid slim set")


def check_chunk_file(data: bytes, row: dict, m: int, chunk_size: int) -> list[tuple]:
    """Validate a chunk file against its suite row (sections 5.5 and 14): its lines, or ``chunk-file``.

    One line per chunk, exactly ``m`` of them, ``<c> <cases> <first_admitted> <calls> <inputs_hex>
    [<expected_hex>]`` with single spaces, canonical decimals, 64-digit lowercase hex and LF; the chunk
    index in sequence; ``first_admitted`` the running prefix sum of ``cases`` (:func:`advance_admitted`);
    ``sum(cases)`` the row's ``cases`` and ``discarded`` the rest of the product; ``sum(calls)`` the row's
    ``expected_calls`` ([D73], section 16.21: the per-chunk digests do not cover ``calls``); ``chunks_sha256`` the
    file's digest; the two suite digests the framing of the per-chunk digests.  The expected digest is
    present exactly when the suite is a ``reference`` suite.
    """
    bad = lambda why: Refused("chunk-file", why)  # noqa: E731
    if hashlib.sha256(data).hexdigest() != row["chunks_sha256"]:
        raise bad("chunks_sha256 is not the digest of the file")
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as exc:
        raise bad("the file is not ASCII") from exc
    lines = text.split("\n")
    if lines.pop() != "" or len(lines) != m:
        raise bad("the file has m LF-terminated lines")
    reference = row["oracle"]["kind"] == "reference"
    out: list[tuple] = []
    running = 0
    calls_total = 0
    for index, line in enumerate(lines):
        hit = _LINE.fullmatch(line)
        if not hit or (hit.group(6) is not None) != reference:
            raise bad("a line breaks the grammar of section 5.5")
        c, cases, first, calls = (int(hit.group(i)) for i in range(1, 5))
        calls_total += calls
        if c != index:
            raise bad("a chunk index out of sequence")
        if first != running:
            raise bad("first_admitted is not the running prefix sum")
        running = advance_admitted(running, cases, chunk_size)
        out.append((c, cases, first, calls, hit.group(5), hit.group(6)))
    if running != row["cases"] or row["discarded"] != row["product"] - running:
        raise bad("the cases do not sum to the row's cases")
    if calls_total != row["expected_calls"]:
        raise bad("the calls do not sum to the row's expected_calls (D73)")
    framed = S.chunk_digest_framing([bytes.fromhex(t[4]) for t in out])
    if framed != row["inputs_sha256"]:
        raise bad("inputs_sha256 is not the framing of the chunk digests")
    if reference and S.chunk_digest_framing([bytes.fromhex(t[5]) for t in out]) != row["expected_sha256"]:
        raise bad("expected_sha256 is not the framing of the chunk digests")
    return out


@dataclass
class Kit:
    """One kit document, derived: the oracle's corpus and chunk files, and a plan per claim.

    ``derived`` is what the oracle writes for the document (the **inputs** of the ``v2-chunk`` cases:
    the suite rows and chunk files); ``plans`` are the :class:`.suites.SuitePlan` objects that
    enumerate one chunk alone (the **derivation** of their outcomes).  Building a plan is cheap; the
    full ``derive`` is the expensive part (about 25 s per ``2**24`` suite) and happens once.
    """

    derived: object
    plans: dict
    chunk_size: int

    def row(self, claim: str) -> dict:
        return next(r for r in self.derived.corpus["suites"] if r["claim"] == claim)

    def corpus_part(self) -> dict:
        c = self.derived.corpus
        return {k: c[k] for k in sorted(CHUNK_CORPUS_KEYS)}

    def chunk_file(self, claim: str) -> bytes:
        return self.derived.chunk_files[chunk_file_name(claim)]


def build_kit(warrant: bytes, profile: bytes, *, spec_ref: str = "conformance-v2/kit") -> Kit:
    """Derive one kit document with :func:`.corpus_v2.derive` and plan every suite for single-chunk runs."""
    derived = derive(
        warrant, profile, spec_ref=spec_ref, generator={"name": "warrant-corpus-gen", "version": "0.6.0"}
    )
    spec = lower_document(parse_source(warrant))
    prof = parse_profile(profile)
    prog = build_program(spec, max_cost=prof.max_ir_cost)
    plans = {
        s.claim: SuitePlan(spec, prog, s, chunk_size=prof.chunk_size, budget=prof.budget)
        for s in spec.suites
    }
    return Kit(derived, plans, prof.chunk_size)


def derive_chunk(case: dict, kit: Kit) -> dict:
    """``{"cases","first_admitted","calls","inputs_sha256","expected_sha256"}`` or a closed ``v2-chunk`` error.

    The lane (a) validates the chunk file and the row's ``slim_chunks``, (b) enumerates chunk
    ``chunk`` **alone, by unranking**, and recomputes ``cases``, ``calls`` and the digests, and (c)
    reports ``first_admitted`` **from the validated file** (a lane that enumerates one chunk alone
    cannot compute it).  Here (b) is :meth:`.suites.SuitePlan.run_chunk`, which starts at product
    ordinal ``chunk * chunk_size`` and never touches an earlier chunk.
    """
    schema_index(case["chunk"], "chunk")
    try:
        m, size = check_corpus_and_row(case["corpus"], case["suite"], case["chunk"])
        check_slim_chunks(case["suite"], m)
        lines = check_chunk_file(bytes.fromhex(case["chunk_file_hex"]), case["suite"], m, size)
    except Refused as r:
        return {"error": r.code}
    plan = kit.plans[case["suite"]["claim"]]
    done = plan.run_chunk(case["chunk"])
    out = {
        "cases": done.cases,
        "first_admitted": lines[case["chunk"]][2],
        "calls": done.calls,
        "inputs_sha256": done.inputs.hex(),
    }
    if done.expected is not None:
        out["expected_sha256"] = done.expected.hex()
    return out


def chunk_line_expect(line: tuple) -> dict:
    """What a validated chunk-file line says about its own chunk (the build cross-checks the enumeration)."""
    out = {"cases": line[1], "first_admitted": line[2], "calls": line[3], "inputs_sha256": line[4]}
    if line[5] is not None:
        out["expected_sha256"] = line[5]
    return out


# ================================================================================================
# 7. v2-fault: the three wrappers of section 8.1
# ================================================================================================


def apply_pre(rewrites: list, penv: dict) -> dict:
    """The new parameter values of a ``pre`` fault: **all rewrites applied simultaneously** (section 8.1).

    Every expression reads the parameters *as they were passed in*; one rewrite never sees another's
    output.  This is the *hook* the mutant test patches ("``pre`` rewrites applied sequentially").
    """
    return {param: fn(penv) for param, fn in rewrites}


def post_value(fault: dict, fn, penv: dict) -> object:
    """The value of a ``post`` or ``replace`` expression over the parameters and ``result`` (a hook)."""
    return fn(penv)


def op_sig(name: str, row: dict, types: dict) -> OpSig:
    """The section 6 op row as an :class:`.lower.OpSig` (the row itself carries no name)."""
    statics = tuple(row.get("width_params", ()))
    widths = frozenset(statics)
    params = [(p, parse_type(t, types, widths=widths)) for p, t in row["params"]]
    return OpSig(name, statics, params, parse_type(row["ret"], types, widths=widths), bool(row.get("fresh", False)))


def instantiate_fault(name: str, op_name: str, op_row: dict, types: dict) -> dict:
    """The row of the portable fault ``name`` on the op, written by the oracle's own templates (section 8.2).

    ``fault_row`` is always told what the case declares (D71, no data-blind call): its record and variant names and
    the fields of its records, so ``field-zero`` and a record or ``Option`` payload of ``missing-error`` have their
    templates here as in a document.
    """
    errors = {n: tuple(row["cases"]) for n, row in types.items() if row.get("kind") == "error"}
    data_names = frozenset(n for n, row in types.items() if row.get("kind") in ("record", "variant"))
    # A table with an undeclared field type is a *case* ([D74], R110: ``derive_fault`` answers ``ill-typed``), so the
    # row is still written: a record that holds such a field has no template fields to offer (the table check, not
    # this builder, is what refuses it).
    records: dict = {}
    for rname, row in types.items():
        if isinstance(row, dict) and row.get("kind") == "record":
            try:
                records[rname] = [(f, parse_type(t, types)) for f, t in row["fields"]]
            except (Refused, KeyError, IndexError, TypeError, ValueError):
                continue
    return fault_row(op_sig(op_name, op_row, types), name, errors, data_names, records)


def fault_levels(fault: dict, op: OpSig) -> set[str]:
    """The levels a fault row *uses*: the levels of its tags and of the op's types (R9).

    The ``requires`` a row records lies between this set minus ``ir.core`` and this set: the templates
    list the levels beyond ``ir.core`` and add ``ir.core`` only when the core form is the template's
    own (``["ir.fixed"]`` for ``xor-lsb``, ``["ir.core", "ir.tuple"]`` for the ``Ok`` form of
    ``drop-last-result-byte``, section 8.2).  :func:`derive_fault` checks that.
    """
    trees = [r["expr"] for r in fault["rewrites"]] if fault["kind"] == "pre" else [fault["expr"]]
    needs = set(L.ir_levels(trees, follow=False))
    for _, ty in [*op.params, ("result", op.ret)]:
        needs |= type_levels(ty)
    return needs


def unstated(ty: A.Ty) -> A.Ty:
    """``ty`` with every *named* width left unstated: ``Bytes<N>`` with a name ``N`` is plain ``Bytes``.

    A lane wraps an op once, with no call site and so no static widths (section 16.18 R70), so it types a fault
    row with this reading of the op's signature.  A literal width (``Bytes<2>``) is kept.
    """
    if ty.head == "Bytes" and ty.args and isinstance(ty.args[0], str):
        return A.Ty("Bytes")
    return A.Ty(ty.head, tuple(unstated(a) if isinstance(a, A.Ty) else a for a in ty.args))


def derive_fault(case: dict) -> dict:
    """``{"args": [..]}`` for ``pre``, ``{"result": V}`` for ``post`` / ``replace``, or a closed error.

    The order is section 11's and R110 (b)'s: ``ill-typed`` for the ``types`` table (read whole and first: R108,
    R117), then ``level-missing`` (the row needs a level that ``levels`` lacks; a lane records such a control
    ``not-live``), then ``subject-forbidden`` (a fault expression contains a ``subject_op``), then ``ill-typed``
    again for the typing of the row: a fault expression that is ill-typed as a tree, or has no common type with
    the parameter or result it replaces (R94, R96 to R98; [D74], R110).  A ``pre`` fault reports **every** argument in parameter order after the rewrite,
    unchanged ones included (R26).  A fault's row must state the ``requires`` the oracle computes.
    """
    fault, types = case["fault"], case["types"]
    try:
        check_type_table(types, "ill-typed")
    except Refused as r:
        return {"error": r.code}
    try:
        named = op_sig(fault["op"], case["op"], types)
    except Refused as r:
        # R111 (c): an op row that names ``Int`` or an undeclared type is ``ill-typed``, not ``type``.
        return {"error": "ill-typed" if r.code == "type" else r.code}
    # R70 and R94: no call site, no widths.  The row is typed with each named width unstated (a parameter or
    # result `Bytes<N>` is plain `Bytes`), and `args` and `result` are decoded as `Bytes` values.
    op = OpSig(named.name, named.statics, [(p, unstated(t)) for p, t in named.params], unstated(named.ret), named.fresh)
    used = fault_levels(fault, op)
    stated = set(fault["requires"])
    if not (used - {L.CORE} <= stated <= used) or fault["requires"] != sorted(stated):
        raise KitError(f"the fault row says requires {fault['requires']}, the oracle finds {sorted(used)} in use")
    if not stated <= set(case["levels"]):
        return {"error": "level-missing"}
    trees = [r["expr"] for r in fault["rewrites"]] if fault["kind"] == "pre" else [fault["expr"]]
    if any(E.has_subject(t) for t in trees):
        return {"error": "subject-forbidden"}
    errors = {n: tuple(row["cases"]) for n, row in types.items() if row.get("kind") == "error"}
    prog = E.Program({}, {}, errors=errors, analyze_data=L.DATA in set(case["levels"]))
    try:
        # R94: the common-type check of each rewrite (or the post / replace expression) against what it replaces is
        # :func:`.faults._compile_fault`'s own (:func:`.faults._fault_unify`), so the kit and the oracle's derivation
        # are one rule (S5e's separate check, built on ``lower._unify``, had no tuple widening and is gone).
        compiled = _compile_fault(prog, op, fault, {})
    except OracleError as exc:
        # R110 (a): a row with no common type, or an expression ill-typed as a tree, is ``ill-typed`` ([D74]).
        # Only the typing codes are: anything else (a limit, say) is not one of the kit's codes and fails the build.
        if exc.code in ("ir", "type", "generation", "fault"):
            return {"error": "ill-typed"}
        raise KitError(f"the oracle cannot compile the fault: {exc}") from exc
    if len(case["args"]) != len(op.params):
        raise KitError("args is one value per parameter")
    penv = {p: decode_value(v, t, types) for (p, t), v in zip(op.params, case["args"])}
    decode_value(case["result"], op.ret, types)  # the call's result is always a value of the op's type
    if fault["kind"] == "pre":
        new = apply_pre(compiled, penv)
        args = [value_to_json(new.get(p, penv[p]), t) for p, t in op.params]
        for (_, t), v in zip(op.params, args):
            _check_derived(v, t, types)
        return {"args": args}
    penv["result"] = decode_value(case["result"], op.ret, types)
    out = post_value(fault, compiled, penv)
    # Typed as the op's *declared* result: a ``Nat`` result stays a ``Nat`` after ``+ 1``.
    result = value_to_json(out, op.ret)
    _check_derived(result, op.ret, types)
    return {"result": result}


def _check_derived(raw: object, ty: A.Ty, types: dict) -> None:
    """A derived value must read back as a value of its type (a fault whose output is out of range is a kit bug)."""
    try:
        decode_value(raw, ty, types)
    except Refused as exc:
        raise KitError(f"the derived value {raw!r} is not a value of its type: {exc}") from exc


# ================================================================================================
# 8. The envelope (D41), the build of a category and of the whole kit
# ================================================================================================


@dataclass
class Row:
    """One hand-written table row: the inputs of a case and its **intent**.

    ``fields`` are the category's case fields (everything but ``id``, ``summary``, ``requires``,
    ``expect``); ``intent`` is the outcome the author expects, stated as a (sub-)object of ``expect``:
    every key it names must be in the derived outcome with an equal value.  Keys the author cannot
    write by hand (a SHA-256) are simply left out.
    """

    slug: str
    summary: str
    fields: dict
    intent: dict


def contains(intent: object, got: object) -> bool:
    """Is ``intent`` a sub-object of ``got`` (type-strict: ``True`` is not ``1``)?"""
    if isinstance(intent, dict):
        return isinstance(got, dict) and all(k in got and contains(v, got[k]) for k, v in intent.items())
    if isinstance(intent, list):
        return isinstance(got, list) and len(intent) == len(got) and all(map(contains, intent, got))
    return type(intent) is type(got) and intent == got


def requires_of(category: str, fields: dict, expect: dict) -> list[str]:
    """The case's ``requires`` (section 11, R24): what a runner must implement to produce ``expect``.

    For ``level-missing`` and ``unknown-tag`` it is the case's ``levels`` (the level of the offending
    tag is deliberately not included); for every other case it is computed like a suite row's (R9) over
    the case's IR and types.  Every S5 case must come to ``["ir.core"]``: anything else fails the build.
    """
    if category == "v2-ir":
        if expect.get("error") in ("level-missing", "unknown-tag"):
            return sorted(fields["levels"])
        trees, models, ops = reach(fields)
        return sorted({L.CORE} | reached_levels(fields, trees, models, ops))
    if category == "v2-value":
        try:
            return sorted({L.CORE} | type_levels(parse_type(fields["type"], fields["types"]), fields["types"]))
        except Refused:
            return list(CORE)
    if category == "v2-domain":
        return sorted(descriptor_levels(fields["descriptor"]))
    if category == "v2-chunk":
        return sorted(fields["suite"]["requires"])
    if category == "v2-fault":
        if expect.get("error") == "level-missing":
            return sorted(fields["levels"])
        return sorted({*fields["fault"]["requires"], *CORE})
    return list(CORE)


def _check_envelope(category: str, case_id: str, slug: str, summary: str) -> None:
    if not SLUG.fullmatch(slug):
        raise KitError(f"the slug {slug!r} does not match [a-z0-9]+(-[a-z0-9]+)*", case_id)
    if not (summary.isascii() and summary.isprintable() and 0 < len(summary) <= MAX_SUMMARY):
        raise KitError("the summary is one printable ASCII line of at most 240 characters", case_id)


def finish_case(category: str, row: Row, expect: dict, fields: dict | None = None) -> dict:
    """Assemble one case: check the intent against the derivation, the closed error set and ``requires``."""
    case_id = f"{category}/{row.slug}"
    _check_envelope(category, case_id, row.slug, row.summary)
    fields = row.fields if fields is None else fields
    if not contains(row.intent, expect):
        raise KitError(f"the table intends {row.intent!r} but the oracle derives {expect!r}", case_id)
    code = expect.get("error")
    if code is not None and code not in ERROR_CODES[category]:
        raise KitError(f"{code!r} is not in the closed error set of {category}", case_id)
    if category == "v2-ir":
        if (code == "ir-overflow") != ("class" in expect):
            raise KitError("class is carried by ir-overflow and by nothing else", case_id)
        if fields.get("max_ir_cost") == E.MAX_IR_COST:
            raise KitError("max_ir_cost is present only when it is not 2,000", case_id)
    requires = requires_of(category, fields, expect)
    if requires != CORE:
        raise KitError(f"requires is {requires}; every S5 case has {CORE} (R24)", case_id)
    return {"id": case_id, "summary": row.summary, "requires": requires, **fields, "expect": expect}


_DERIVERS = {
    "v2-value": derive_value,
    "v2-arith": derive_arith,
    "v2-ir": derive_ir,
    "v2-domain": derive_domain,
    "v2-fault": derive_fault,
}


def derive_expect(category: str, fields: dict, kit_by_name: dict | None = None) -> dict:
    """The oracle's outcome for one case's fields (no intent, no envelope): what the mutant test compares."""
    if category == "v2-chunk":
        return derive_chunk(fields, kit_by_name[fields["_kit"]])
    return _DERIVERS[category](fields)


def build_category(category: str, rows: list[Row], kits: dict | None = None) -> list[dict]:
    """Every case of a category, in table order, each derived and checked against its intent.

    A ``v2-chunk`` row's ``fields`` carry ``_kit`` (the kit document's name), which is removed from
    the case; its derivation also cross-checks the single-chunk enumeration against the validated chunk
    file, because a case that is *accepted* must agree with the file it was built from.
    """
    cases = []
    for row in rows:
        try:
            expect = derive_expect(category, row.fields, kits)
        except KitError as exc:
            raise KitError(str(exc), f"{category}/{row.slug}") from exc
        fields = {k: v for k, v in row.fields.items() if not k.startswith("_")}
        if category == "v2-chunk" and "error" not in expect:
            want = chunk_line_expect(
                check_chunk_file(
                    bytes.fromhex(fields["chunk_file_hex"]),
                    fields["suite"],
                    fields["suite"]["chunks"],
                    fields["corpus"]["chunk_size"],
                )[fields["chunk"]]
            )
            if {k: v for k, v in expect.items() if k in want} != want:
                raise KitError(
                    "the enumeration of the chunk disagrees with its line of the chunk file",
                    f"{category}/{row.slug}",
                )
        cases.append(finish_case(category, row, expect, fields))
    ids = [c["id"] for c in cases]
    if len(set(ids)) != len(ids):
        raise KitError(f"a duplicate id in {category}")
    return cases


def render(category: str, cases: list[dict]) -> str:
    """The text of ``<category>.json``: the family's canonical writer (VER04i 9.1), LF, final LF, ASCII."""
    text = canonical({"schema": SCHEMA, "category": category, "cases": cases}).decode("ascii")
    return text


def build_files(tables: dict, kits: dict) -> tuple[dict, list[dict]]:
    """``({file name: text}, [index rows])`` of the six ``v2-*`` categories, in the fixed order of section 11.

    ``tables`` maps a category to its :class:`Row` list.  The index rows are what ``index.json`` must
    hold for these files (``category``, ``path``, ``cases``, ``sha256``); ``index.json`` itself is
    written by the family's v1 script, never by this module (R26).  Every id starts with its category, so ids are
    unique across the family as soon as they are unique within a category (:func:`build_category`).
    """
    files, rows = {}, []
    for category in CATEGORIES:
        cases = build_category(category, tables[category], kits)
        text = render(category, cases)
        name = f"{category}.json"
        files[name] = text
        rows.append(
            {
                "category": category,
                "path": name,
                "cases": len(cases),
                "sha256": hashlib.sha256(text.encode("ascii")).hexdigest(),
            }
        )
    return files, rows
