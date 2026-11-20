#!/usr/bin/env python3
"""Write (or ``--check``) the synthetic ``ir.data`` document of slice S11a and its committed dumps.

The document ``data_forms_v1.warrant`` exercises **every form of VER04r Appendix A.2**: records, variants
(with and without a payload), ``Option`` as a real type, the four tags ``some none ctor field``, ``match`` on
every kind of scrutinee (an ``Option``, a ``Bool``, a bare tuple, a record, a variant and a ``Result`` with a
data pattern) with the four data patterns ``some none ctor bool``, structural ``==``/``!=`` on the new types,
and the **intermediate use** of ``unwrap_or`` that must keep adding no ``ir.data`` (section 16.14 R29).  It is
generated, not hand-written, so that the committed text and this script cannot drift (``--check``, a test, and
a line of ``BUILD``); it is the one document the Rust generator's slice S11b has to reproduce.

Three files are written beside this script (the precedent is ``tuple-forms-v1`` and ``fixed-forms-v1``):

``data_forms_v1.warrant``
    the document (this script's text);
``data_forms_v1.oracle-lowering.json``
    the lowering dump (``lowering_dump``, schema ``warrant-lowering-parity-v1``): the lowered IR, the
    computed ``requires`` and the cost of every model, op ``admit``, claim and example, and (new in S11a,
    only for a document that declares one) the corpus ``types`` rows of its records and variants.  **This
    dump makes Appendix A.2 stop being provisional** (section 16.14 [D47]): it is the first reader of it;
``data_forms_v1.oracle-refusals.json``
    every document the oracle refuses and how: the reserved levels and feature names (``ir.list``, ``ir.seq``,
    ``err-value``, ``fixed-cast``), the corpus limits of section 2.2 (R34), the typing rules of R39, the
    corpus errors of Appendix A.2 and the **refusal order** of R35, each with the code, the level, the
    message and the source.

    python3 make_data_forms.py            # (re)write the three files
    python3 make_data_forms.py --check    # fail if any committed file differs

Evaluation, values, the stream, TFV and faults are *not* in these dumps: the oracle lowers, types and costs
``ir.data`` in this slice and still refuses to evaluate it (``level ir.data``; slice S11c-1 opens that, in a
directory of its own, ``tests/fixtures/data-eval-v1/``).

Only the standard library and the oracle package are used.  Names: models are plain, each has a subject op
``op_<model>`` with the same signature and a reference claim ``c_<model>``, and examples are ``e_*``.
"""

from __future__ import annotations

import pathlib
import re
import sys

from coding_adventures_warrant_oracle.corpus_v2 import canonical
from coding_adventures_warrant_oracle.errors import OracleError
from coding_adventures_warrant_oracle.lower import lower_document, lowering_refusal
from coding_adventures_warrant_oracle.lowering_dump import dump
from coding_adventures_warrant_oracle.parser import parse_source

HERE = pathlib.Path(__file__).resolve().parent
NAME = "data_forms_v1"

R = "Result<Option<U8>,DataError>"

#: The pure models.  Each is also checked by a generated subject op ``op_<name>`` with the same signature and a
#: reference claim ``c_<name>``: ``(name, parameters, result type, body)``.  Every model has a parameter, so
#: every op has one (the contract blocks below need it); an unused ``u: Bool`` stands where a model has none.
MODELS = [
    # -- Option: Some, a typed None, nesting, a context for None (an if branch either way round, an Ok) ----
    ("some_u8", "a: U8", "Option<U8>", "Some(a)"),
    ("none_u8", "u: Bool", "Option<U8>", "None"),
    ("none_bytes", "u: Bool", "Option<Bytes<2>>", "None"),
    ("some_lit", "u: Bool", "Option<Nat>", "Some(7)"),
    ("some_nested", "a: U8", "Option<Option<U8>>", "Some(Some(a))"),
    ("some_none", "u: Bool", "Option<Option<U8>>", "Some(None)"),
    ("opt_if", "c: Bool, a: U8", "Option<U8>", "if c then Some(a) else None"),
    ("opt_if_rev", "c: Bool, a: U8", "Option<U8>", "if c then None else Some(a)"),
    ("opt_bytes", "d: Bytes", "Option<Bytes>", "if len(d) == 0 then None else Some(d)"),
    ("opt_fixed_bytes", "d: Bytes<2>", "Option<Bytes<2>>", "Some(d)"),
    ("opt_pair", "a: Bool", "Option<(Bool,Nat)>", "Some((a, 1))"),
    ("opt_rec", "a: U8, b: U8", "Option<Point>", "Some(Point(a, b))"),
    (
        "res_opt",
        "a: U8",
        R,
        "if a > 1u8 then Err(DataError.Bad) else Ok(if a == 1u8 then Some(a) else None)",
    ),
    ("res_none", "u: Bool", R, "Ok(None)"),
    # -- structural equality on Option -----------------------------------------------------------------
    ("opt_eq", "a: Option<U8>, b: Option<U8>", "Bool", "a == b"),
    ("opt_ne", "a: Option<U8>, b: Option<U8>", "Bool", "a != b"),
    ("opt_is_none", "a: Option<U8>", "Bool", "a == None"),
    ("opt_is_some_lit", "a: Option<U8>", "Bool", "Some(1u8) == a"),
    # -- the intermediate use of ir.fixed (adds NO ir.data) against every other Option (adds it), R29 ----
    ("unwrap_binder", "o: Option<U8>", "U8", "unwrap_or(o, 5u8)"),
    ("inter_add", "a: U8, b: U8", "U8", "unwrap_or(checked_add(a, b), 0u8)"),
    ("inter_at", "d: Bytes, i: Nat", "U8", "unwrap_or(at(d, i), 0u8)"),
    ("inter_div_nat", "a: Nat, b: Nat", "Bool", "unwrap_or(div(a, b), 0) == 3"),
    ("prod_eq", "a: U8, b: U8", "Bool", "checked_add(a, b) == Some(a)"),
    ("prod_in_ok", "a: U8, b: U8", R, "Ok(rem(a, b))"),
    ("prod_in_if", "c: Bool, d: Bytes, i: Nat", "U8", "unwrap_or(if c then at(d, i) else None, 0u8)"),
    ("prod_try", "a: Nat", "Option<U8>", "try<U8>(a)"),
    # -- records: construction, field, nesting, postfix field, structural equality -----------------------
    ("mk_point", "a: U8, b: U8", "Point", "Point(a, b)"),
    ("px", "p: Point", "U8", "p.x"),
    ("py", "p: Point", "U8", "p.y"),
    ("mk_wrapped", "a: U8, b: U8, t: Bool", "Wrapped", "Wrapped(Point(a, b), t)"),
    ("wrapped_y", "w: Wrapped", "U8", "w.p.y"),
    ("mk_blob", "d: Bytes, n: Nat", "Blob", "Blob(d, n)"),
    ("mk_blob_lit", "u: Bool", "Blob", 'Blob(b"0102", 3)'),
    ("blob_len", "b: Blob", "Nat", "len(b.data)"),
    ("mk_slot", "o: Option<U8>, a: Bool", "Slot", "Slot(o, (a, 2))"),
    ("slot_o", "s: Slot", "Option<U8>", "s.o"),
    ("slot_t0", "s: Slot", "Bool", "s.t.0"),
    ("rec_eq", "a: Point, b: Point", "Bool", "a == b"),
    ("rec_ne", "a: Wrapped, b: Wrapped", "Bool", "a != b"),
    ("rec_postfix", "a: U8, b: U8", "U8", "mk_point(a, b).y"),
    ("rec_lit_field", "a: U8", "U8", "Point(a, 1u8).x"),
    # -- variants: every constructor shape, structural equality -----------------------------------------
    ("mk_circle", "r: U8", "Shape", "Circle(r)"),
    ("mk_rect", "w: U8, h: U8", "Shape", "Rect(w, h)"),
    ("mk_dot", "u: Bool", "Shape", "Dot"),
    ("mk_done", "n: Nat", "Msg", 'Done(n, b"")'),
    ("mk_opt_msg", "o: Option<U8>", "Msg", "Opt(o)"),
    ("mk_opt_some", "a: U8", "Msg", "Opt(Some(a))"),
    ("mk_opt_none", "u: Bool", "Msg", "Opt(None)"),
    ("mk_short", "u: Bool", "Msg", "Short"),
    ("mk_neg", "u: Bool", "Mode", "Neg"),
    ("var_eq", "a: Shape, b: Shape", "Bool", "a == b"),
    ("var_ne", "a: Shape, b: Shape", "Bool", "a != b"),
    ("var_eq_lit", "a: Shape", "Bool", "a == Dot"),
    # -- match on an Option ---------------------------------------------------------------------------
    ("m_opt", "o: Option<U8>", "U8", "match o with Some(x) -> x | None -> 0u8"),
    ("m_opt_wild", "o: Option<U8>", "Bool", "match o with Some(_) -> true | None -> false"),
    ("m_opt_wild_arm", "o: Option<U8>", "Bool", "match o with None -> false | _ -> true"),
    (
        "m_opt_nested",
        "o: Option<Option<U8>>",
        "Nat",
        "match o with Some(Some(x)) -> 2 | Some(None) -> 1 | None -> 0",
    ),
    (
        "m_opt_ret_opt",
        "o: Option<U8>",
        "Option<U8>",
        "match o with Some(x) -> Some(x) | None -> None",
    ),
    ("m_opt_bind_whole", "o: Option<U8>", "Bool", "match o with Some(_) -> true | whole -> whole == o"),
    ("m_opt_on_call", "a: U8", "Nat", "match some_u8(a) with Some(_) -> 1 | None -> 0"),
    ("m_opt_literal", "u: Bool", "Nat", "match Some(1u8) with Some(_) -> 1 | None -> 0"),
    (
        "m_opt_tuple",
        "o: Option<(Bool,Nat)>",
        "Nat",
        "match o with Some((true, n)) -> n | Some((false, _)) -> 0 | None -> 0",
    ),
    # -- match on a Bool, with a data pattern `bool` ---------------------------------------------------
    ("m_bool", "b: Bool", "Nat", "match b with true -> 1 | false -> 0"),
    ("m_bool_wild", "b: Bool", "Nat", "match b with true -> 1 | _ -> 0"),
    ("m_bool_expr", "a: U8", "Nat", "match a == 0u8 with true -> 1 | false -> 0"),
    # -- match on a bare tuple (ir.data by its scrutinee, ir.tuple by its type) ------------------------
    ("m_tuple", "t: (Bool,Bool)", "Bool", "match t with (a, b) -> a"),
    ("m_tuple_bool", "t: (Bool,Bool)", "Bool", "match t with (true, x) -> x | (false, _) -> false"),
    (
        "m_tuple_nested",
        "t: ((Bool,Bool),Nat)",
        "Nat",
        "match t with ((true, _), n) -> n | ((false, _), _) -> 0",
    ),
    # -- match on a record, with nested constructor patterns -------------------------------------------
    ("m_point", "p: Point", "U8", "match p with Point(x, y) -> x + y"),
    ("m_point_wild", "p: Point", "U8", "match p with Point(_, y) -> y"),
    (
        "m_wrapped",
        "w: Wrapped",
        "U8",
        "match w with Wrapped(Point(a, _), true) -> a | Wrapped(Point(_, b), false) -> b",
    ),
    ("m_blob", "b: Blob", "Nat", "match b with Blob(_, n) -> n"),
    (
        "m_slot",
        "s: Slot",
        "Nat",
        "match s with Slot(Some(_), (true, n)) -> n | Slot(Some(_), (false, _)) -> 1 | Slot(None, _) -> 0",
    ),
    # -- match on a variant, a payload-free variant included -------------------------------------------
    ("m_shape", "s: Shape", "U8", "match s with Circle(r) -> r | Rect(w, h) -> w + h | Dot -> 0u8"),
    ("m_shape_wild", "s: Shape", "Bool", "match s with Dot -> true | _ -> false"),
    ("m_mode", "o: Mode", "Nat", "match o with Add -> 0 | Sub -> 1 | Neg -> 2"),
    ("m_is_neg", "o: Mode", "Bool", "match o with Neg -> true | _ -> false"),
    (
        "m_msg",
        "m: Msg",
        "Nat",
        "match m with Short -> 0 | Done(n, _) -> n | Opt(Some(x)) -> to_nat(x) | Opt(None) -> 0",
    ),
    ("m_msg_bind", "m: Msg", "Msg", "match m with Short -> Short | other -> other"),
    ("m_shape_ret_var", "s: Shape", "Shape", "match s with Circle(r) -> Rect(r, r) | other -> other"),
    # -- match on a Result with a data pattern (ir.tuple AND ir.data) vs. plain (ir.tuple only) -----------
    (
        "m_res_bool",
        "r: Result<Bool,DataError>",
        "Nat",
        "match r with Ok(true) -> 1 | Ok(false) -> 0 | Err(_) -> 2",
    ),
    ("m_res_opt", f"r: {R}", "Nat", "match r with Ok(Some(_)) -> 1 | Ok(None) -> 0 | Err(_) -> 2"),
    (
        "m_res_rec",
        "r: Result<Point,DataError>",
        "U8",
        "match r with Ok(Point(x, _)) -> x | Err(DataError.Bad) -> 0u8 | Err(_) -> 1u8",
    ),
    ("m_res_err_unused", "r: Result<Bool,DataError>", "Nat", "match r with Ok(_) -> 1 | Err(e) -> 0"),
    ("m_res_plain", "r: Result<Bool,DataError>", "Bool", "match r with Ok(x) -> x | Err(_) -> false"),
    (
        "m_inner",
        "o: Option<U8>, b: Bool",
        "Nat",
        "match o with Some(_) -> (match b with true -> 1 | false -> 2) | None -> 0",
    ),
]

#: Operations that need an ``admit`` or a name of their own beyond the one per model.
EXTRA_OPS = [
    "op op_some_only(o: Option<U8> | match o with Some(_) -> true | None -> false) -> U8",
    "op op_point_ok(p: Point | p.x < 100u8) -> U8",
]


def _names(params: str) -> list[str]:
    return re.findall(r"(?:^|, )(\w+): ", params)


def main() -> str:
    out: list[str] = []
    w = out.append
    ops: list[tuple[str, list[str]]] = []

    def op(line: str) -> None:
        """Declare a subject op and remember its parameters for the contract blocks."""
        w(line)
        name, rest = line.split("op ", 1)[1].split("(", 1)
        ops.append((name.strip(), _names(rest.rsplit(")", 1)[0].split(") ->")[0])))

    w("warrant data_forms v1")
    w("  -- Synthetic ir.data document of slice S11a (VER04r Appendix A.2): records, variants, Option, the")
    w("  -- four tags some / none / ctor / field, match on every scrutinee with the four data patterns, and the")
    w("  -- intermediate use of ir.fixed.  GENERATED by make_data_forms.py; do not edit by hand.")
    w("  error DataError = Bad | Worse")
    w("  record Point(x: U8, y: U8)")
    w("  record Wrapped(p: Point, tag: Bool)")
    w("  record Blob(data: Bytes, n: Nat)")
    w("  record Slot(o: Option<U8>, t: (Bool,Nat))")
    w("  variant Shape = Circle(r: U8) | Rect(w: U8, h: U8) | Dot")
    w("  variant Mode = Add | Sub | Neg")
    w("  variant Msg = Short | Done(n: Nat, rest: Bytes) | Opt(o: Option<U8>)")
    w("")
    w("  -- pure models: one per form of Appendix A.2 --------------------------------------------------")
    for name, params, ret, body in MODELS:
        w(f"  model {name}({params}) -> {ret} = {body}")
    w("")
    w("  -- subject operations, claims and examples --------------------------------------------------")
    for name, params, ret, _body in MODELS:
        op(f"  op op_{name}({params}) -> {ret}")
    for line in EXTRA_OPS:
        op(f"  {line}")
    w("  assume platform:")
    w("    none")
    w("  assume environment:")
    w("    none")
    for name, params in ops:
        w(f"  labels {name}:")
        for prm in params:
            w(f"    {prm} secret")
    for name, params in ops:
        for prm in params:
            w(f"  input {name}.{prm}:")
            w("    hostile")
            w("    ensures no_panic, terminates")
    for name, _ in ops:
        w(f"  bound {name}:")
        w("    steps <= 64")
        w("    heap <= 0")
        w("    depth <= 8")
    w("")
    w("  -- claims: one reference claim per model (op_<m>(args) == m(args)) -----------------------------")
    for name, params, _ret, _body in MODELS:
        args = ", ".join(_names(params))
        w(f"  claim c_{name}:")
        w(f"    forall {params}. op_{name}({args}) == {name}({args})")
    w("  claim c_some_only:")
    w("    forall o: Option<U8> | match o with Some(_) -> true | None -> false. op_some_only(o) == m_opt(o)")
    w("  claim c_point_ok:")
    w("    forall p: Point | p.x < 100u8. op_point_ok(p) == px(p)")
    w("")
    w("  -- claims: data inline in the pure side, a subject call in an arm and as the scrutinee -------")
    w("  claim c_inline_match:")
    w("    forall o: Option<U8>. op_m_opt(o) == (match o with Some(x) -> x | None -> 0u8)")
    w("  claim c_inline_ctor:")
    w("    forall a: U8, b: U8. op_mk_point(a, b) == Point(a, b)")
    w("  claim c_inline_field:")
    w("    forall p: Point. op_px(p) == p.x")
    w("  claim c_arm_calls:")
    w("    forall o: Option<U8>. match o with Some(x) -> op_px(Point(x, x)) == x | None -> true")
    w("  claim c_scrutinee_calls:")
    w("    forall a: U8. match op_some_u8(a) with Some(x) -> x == a | None -> false")
    w("  claim c_option_binder_compare:")
    w("    forall o: Option<U8>. op_opt_is_none(o) == (o == None)")
    w("")
    w("  -- examples: Option, record and variant values in TFV (not evaluated here) ----------------------")
    for ex, body in (
        ("e_some", "op_some_u8(7u8) == Some(7u8)"),
        ("e_none", "op_none_u8(true) == None"),
        ("e_point", "op_mk_point(1u8, 2u8) == Point(1u8, 2u8)"),
        ("e_circle", "op_mk_circle(3u8) == Circle(3u8)"),
        ("e_dot", "op_mk_dot(true) == Dot"),
        ("e_done", 'op_mk_done(5) == Done(5, b"")'),
        ("e_opt_msg", "op_mk_opt_some(1u8) == Opt(Some(1u8))"),
        ("e_res_opt", "op_res_opt(1u8) == Ok(Some(1u8))"),
        ("e_nested", "op_some_nested(1u8) == Some(Some(1u8))"),
        ("e_wrapped", "op_mk_wrapped(1u8, 2u8, true) == Wrapped(Point(1u8, 2u8), true)"),
        ("e_field_arg", "op_px(Point(4u8, 5u8)) == 4u8"),
    ):
        w(f"  example {ex}:")
        w(f"    {body}")
    w("")
    return "\n".join(out) + "\n"


# -- the documents the oracle refuses, as (name, text appended to _BASE, expected oracle code) ---------
_BASE = (
    "warrant refusals v1\n"
    "  error E = A | B | C\n"
    "  record P(x: U8, y: Bool)\n"
    "  variant V = Circle(r: U8) | Rect(w: U8, h: U8) | Dot\n"
    "  variant K = Red | Green | Blue\n"
    "  op f(a: Bool) -> Result<Bool,E>\n"
    "  op g(a: Bool) -> Result<Bool,E>\n"
)

#: The reserved levels and the two feature names (R27, R28): ``oracle_level`` is the name carried.
LEVELS = [
    ("a-list-type-in-a-record", "  record Q(xs: List<U8>)\n", "ir.list"),
    ("a-list-type-in-a-variant", "  variant W = One(xs: List<U8>) | Two\n", "ir.list"),
    ("a-list-type-in-an-alias", "  model Word = List<U8>\n", "ir.list"),
    ("a-list-type-in-a-signature", "  op h(xs: List<U8>) -> Bool\n", "ir.list"),
    ("a-list-type-nested-in-a-signature", "  op h(a: Bool) -> Result<List<U8>,E>\n", "ir.list"),
    ("a-list-type-in-a-model", "  model m(xs: List<U8>) -> Bool = true\n", "ir.list"),
    ("a-list-type-in-a-result", "  model m(a: Bool) -> List<U8> = true\n", "ir.list"),
    ("a-list-type-in-a-binder", "  claim c:\n    forall xs: List<U8>. g(true) == g(true)\n", "ir.list"),
    ("a-list-literal", "  model m(a: U8) -> Bool = [a] == [a]\n", "ir.list"),
    ("slice", "  model m(a: Bytes) -> Bool = slice(a, 0, 1) == a\n", "ir.list"),
    ("to-list", "  model m(a: Bytes) -> Bool = to_list(a) == to_list(a)\n", "ir.list"),
    ("to-bytes", "  model m(a: Bytes) -> Bool = to_bytes(a) == a\n", "ir.list"),
    ("replicate", "  model m(a: U8) -> Bool = replicate(a, 2) == replicate(a, 2)\n", "ir.list"),
    ("range", "  model m(a: Nat) -> Bool = range(a) == range(a)\n", "ir.seq"),
    ("fold", "  model m(a: Nat) -> Bool = fold(a) == a\n", "ir.seq"),
    ("map", "  model m(a: Nat) -> Bool = map(a) == a\n", "ir.seq"),
    ("all", "  model m(a: Nat) -> Bool = all(a)\n", "ir.seq"),
    ("any", "  model m(a: Nat) -> Bool = any(a)\n", "ir.seq"),
    ("count", "  model m(a: Nat) -> Bool = count(a) == a\n", "ir.seq"),
    ("fixed-cast", "  model m(a: Bytes) -> Bool = fixed<4>(a) == None\n", "fixed-cast"),
    (
        "err-value-returned",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(x) -> x | Err(e) -> e == e\n",
        "err-value",
    ),
    (
        "err-value-in-a-claim",
        "  claim c:\n    forall a: Bool. g(a) == (match g(a) with Ok(x) -> Ok(x) | Err(e) -> Err(e))\n",
        "err-value",
    ),
    (
        "err-value-in-the-last-arm",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(x) -> x | Err(E.A) -> true | Err(e) -> e == E.A\n",
        "err-value",
    ),
]

#: The corpus limits of section 2.2 (R34), refused whole with ``limit`` before any expression is lowered.
_FIELDS_33 = ", ".join(f"f{i}: U8" for i in range(33))
_CTORS_65 = " | ".join(f"C{i}" for i in range(65))
_FIELDS_9 = ", ".join(f"f{i}: U8" for i in range(9))
_CASES_65 = " | ".join(f"Z{i}" for i in range(65))
LIMITS = [
    ("a-record-of-33-fields", f"  record Big({_FIELDS_33})\n", "limit"),
    ("a-variant-of-65-constructors", f"  variant Many = {_CTORS_65}\n", "limit"),
    ("a-constructor-of-9-fields", f"  variant Wide = Fat({_FIELDS_9}) | Thin\n", "limit"),
    ("an-error-type-of-65-cases", f"  error Huge = {_CASES_65}\n", "limit"),
]

#: The typing rules of R39 and the data typing of Appendix A.2 (oracle code ``type``).
TYPES = [
    ("a-none-with-no-context", "  model m(a: Bool) -> Bool = None == None\n", "type"),
    ("a-none-as-a-scrutinee", "  model m(a: Bool) -> Bool = match None with None -> true | _ -> false\n", "type"),
    ("a-none-in-a-bool-context", "  model m(a: Bool) -> Bool = None\n", "type"),
    (
        "a-some-is-not-lifted-to-a-wider-option",
        "  model m(d: Bytes<2>) -> Option<Bytes> = Some(d)\n",
        "type",
    ),
    ("an-option-nat-is-not-an-option-int", "  model m(a: Nat) -> Option<Int> = Some(a)\n", "type"),
    ("a-projection-of-a-bool", "  model m(a: Bool) -> Bool = a.0\n", "type"),
    ("a-field-of-a-tuple", "  model m(t: (Bool,Bool)) -> Bool = t.x\n", "type"),
    ("a-field-of-a-variant", "  model m(s: V) -> U8 = s.r\n", "type"),
    ("a-field-of-a-bool", "  model m(a: Bool) -> Bool = a.x\n", "type"),
    ("an-index-of-a-record", "  model m(p: P) -> U8 = p.0\n", "type"),
    ("an-unknown-field", "  model m(p: P) -> U8 = p.z\n", "type"),
    ("a-constructor-with-too-few-arguments", "  model m(a: U8) -> P = P(a)\n", "type"),
    ("a-constructor-with-too-many-arguments", "  model m(a: U8) -> V = Circle(a, a)\n", "type"),
    ("a-constructor-argument-of-the-wrong-type", "  model m(a: U8) -> P = P(a, a)\n", "type"),
    ("a-payload-free-constructor-with-parentheses", "  model m(a: U8) -> V = Dot()\n", "type"),
    ("a-constructor-with-fields-used-bare", "  model m(a: U8) -> V = Circle\n", "type"),
    ("an-ordering-on-a-record", "  model m(a: P, b: P) -> Bool = a < b\n", "type"),
    ("an-ordering-on-an-option", "  model m(a: Option<U8>, b: Option<U8>) -> Bool = a <= b\n", "type"),
    ("equality-of-two-record-types", "  model m(a: P, b: V) -> Bool = a == b\n", "type"),
    ("unwrap-or-of-a-non-option", "  model m(a: U8) -> U8 = unwrap_or(a, 0u8)\n", "type"),
    ("unwrap-or-with-a-default-of-the-wrong-type", "  model m(o: Option<U8>) -> U8 = unwrap_or(o, true)\n", "type"),
    ("some-with-two-arguments", "  model m(a: U8) -> Option<U8> = Some(a, a)\n", "type"),
    (
        "arms-with-no-common-type",
        "  model m(s: V) -> Bool = match s with Circle(_) -> true | Rect(_, _) -> 1 | Dot -> false\n",
        "type",
    ),
    (
        "a-constructor-pattern-of-another-type",
        "  model m(s: V) -> Bool = match s with Red -> true | _ -> false\n",
        "type",
    ),
    (
        "an-unknown-constructor-pattern",
        "  model m(s: V) -> Bool = match s with Foo(x) -> true | _ -> false\n",
        "type",
    ),
    (
        "a-constructor-pattern-with-too-few-fields",
        "  model m(s: V) -> Bool = match s with Rect(w) -> true | _ -> false\n",
        "type",
    ),
    (
        "a-constructor-pattern-with-fields-used-bare",
        "  model m(s: V) -> Bool = match s with Circle -> true | _ -> false\n",
        "type",
    ),
    ("a-bool-pattern-on-a-u8", "  model m(a: U8) -> Bool = match a with true -> true | _ -> false\n", "type"),
    ("a-some-pattern-on-a-bool", "  model m(a: Bool) -> Bool = match a with Some(x) -> x | _ -> false\n", "type"),
    ("a-none-pattern-on-a-bool", "  model m(a: Bool) -> Bool = match a with None -> true | _ -> false\n", "type"),
    (
        "a-tuple-pattern-on-an-option",
        "  model m(o: Option<U8>) -> Bool = match o with (a, b) -> true | _ -> false\n",
        "type",
    ),
    (
        "an-ok-pattern-on-an-option",
        "  model m(o: Option<U8>) -> Bool = match o with Ok(_) -> true | _ -> false\n",
        "type",
    ),
    (
        "a-record-declared-twice",
        "  record P(z: U8)\n",
        "type",
    ),
    ("a-constructor-named-like-a-record", "  variant W = P | Q\n", "type"),
    ("a-variant-of-one-constructor", "  variant W = Only(a: U8)\n", "type"),
    ("a-record-that-contains-itself", "  record Loop(next: Loop)\n", "type"),
    ("a-record-cycle-through-a-variant", "  record A1(v: B1)\n  variant B1 = X(a: A1) | Y\n", "type"),
]

#: The corpus errors of Appendix A.2 over the larger pattern set (oracle code ``corpus``, generator ``ir``).
CORPUS = [
    (
        "fall-through-a-bool",
        "  model m(a: Bool) -> Bool = match a with true -> true\n",
        "corpus",
    ),
    (
        "fall-through-an-option-without-none",
        "  model m(o: Option<U8>) -> Bool = match o with Some(_) -> true\n",
        "corpus",
    ),
    (
        "fall-through-an-option-without-some",
        "  model m(o: Option<U8>) -> Bool = match o with None -> true\n",
        "corpus",
    ),
    (
        "fall-through-a-nested-option",
        "  model m(o: Option<Option<U8>>) -> Bool = match o with Some(Some(_)) -> true | None -> false\n",
        "corpus",
    ),
    (
        "fall-through-a-variant-constructor",
        "  model m(s: V) -> Bool = match s with Circle(_) -> true | Rect(_, _) -> false\n",
        "corpus",
    ),
    (
        "fall-through-a-payload-free-variant",
        "  model m(k: K) -> Bool = match k with Red -> true | Green -> false\n",
        "corpus",
    ),
    (
        "fall-through-a-refutable-record-field",
        "  model m(p: P) -> Bool = match p with P(_, true) -> true\n",
        "corpus",
    ),
    (
        "fall-through-a-result-bool",
        "  model m(r: Result<Bool,E>) -> Bool = match r with Ok(true) -> true | Err(_) -> false\n",
        "corpus",
    ),
    (
        "fall-through-a-tuple-bool-component",
        "  model m(t: (Bool,Bool)) -> Bool = match t with (true, _) -> true\n",
        "corpus",
    ),
    (
        "unreachable-after-a-wild-on-an-option",
        "  model m(o: Option<U8>) -> Bool = match o with _ -> true | Some(_) -> false\n",
        "corpus",
    ),
    (
        "unreachable-a-bool-twice",
        "  model m(a: Bool) -> Bool = match a with true -> true | true -> false | false -> true\n",
        "corpus",
    ),
    (
        "unreachable-a-some-twice",
        "  model m(o: Option<U8>) -> Bool = match o with Some(_) -> true | Some(_) -> false | None -> true\n",
        "corpus",
    ),
    (
        "unreachable-a-constructor-after-a-bind",
        "  model m(s: V) -> Bool = match s with other -> true | Dot -> false\n",
        "corpus",
    ),
    (
        "unreachable-a-variant-constructor-twice",
        "  model m(k: K) -> Bool = match k with Red -> true | Red -> false | Green -> true | Blue -> true\n",
        "corpus",
    ),
    (
        "unreachable-after-every-constructor",
        "  model m(k: K) -> Bool = match k with Red -> true | Green -> true | Blue -> true | _ -> false\n",
        "corpus",
    ),
    (
        "unreachable-a-record-after-a-record",
        "  model m(p: P) -> Bool = match p with P(_, _) -> true | P(_, true) -> false\n",
        "corpus",
    ),
    (
        "duplicate-bind-in-a-constructor-pattern",
        "  model m(s: V) -> Bool = match s with Rect(a, a) -> true | _ -> false\n",
        "corpus",
    ),
    (
        "duplicate-bind-across-some-and-tuple",
        "  model m(o: Option<(Bool,Bool)>) -> Bool = match o with Some((a, a)) -> a | None -> false\n",
        "corpus",
    ),
    (
        "bind-in-a-constructor-shadows-a-parameter",
        "  model m(w: U8, s: V) -> Bool = match s with Rect(w, _) -> true | _ -> false\n",
        "corpus",
    ),
    (
        "bind-in-some-shadows-the-scrutinee-name",
        "  model m(o: Option<U8>) -> Bool = match o with Some(o) -> true | None -> false\n",
        "corpus",
    ),
    (
        "bind-in-some-shadows-result",
        "  model m(o: Option<U8>) -> Bool = match o with Some(result) -> true | None -> false\n",
        "corpus",
    ),
    (
        "bind-shadows-an-outer-bind-through-some",
        "  model m(o: Option<Option<U8>>) -> Bool = match o with Some(x) -> (match x with Some(x) -> true | None -> false) | None -> false\n",
        "corpus",
    ),
    (
        "bind-named-like-a-model",
        "  model helper(a: Bool) -> Bool = a\n  model m(o: Option<U8>) -> Bool = match o with Some(helper) -> true | None -> false\n",
        "corpus",
    ),
    (
        "bind-named-like-a-built-in",
        "  model m(o: Option<U8>) -> Bool = match o with Some(rotl) -> true | None -> false\n",
        "corpus",
    ),
]

#: The refusal ORDER of section 16.14 R35, each document having at least two refusals (oracle code per row).
ORDER = [
    # (1) the declaration limits come first, before a level refusal that is declared earlier
    (
        "order-limit-before-a-level",
        "  model m(a: U8) -> Bool = [a] == [a]\n" + f"  record Big({_FIELDS_33})\n",
        "limit",
    ),
    # (2) within one unit a level or feature is named in preference to a type error of the same unit ...
    (
        "order-level-over-a-type-error-of-the-same-unit",
        "  model m(a: Bytes) -> Bool = None == None and fixed<4>(a) == None\n",
        "level:fixed-cast",
    ),
    (
        "order-level-over-a-type-error-in-a-claim",
        "  claim c:\n    forall a: Bool. g(a) == (None == None and range(1) == 2)\n",
        "level:ir.seq",
    ),
    # ... and of several the first in source order
    (
        "order-the-first-level-in-source-order-list-first",
        "  model m(a: U8) -> Bool = to_list(a) == range(a)\n",
        "level:ir.list",
    ),
    (
        "order-the-first-level-in-source-order-seq-first",
        "  model m(a: U8) -> Bool = range(a) == to_list(a)\n",
        "level:ir.seq",
    ),
    (
        "order-a-list-type-before-a-body-level",
        "  model m(xs: List<U8>) -> Bool = fixed<4>(xs) == None\n",
        "level:ir.list",
    ),
    # (3) within a match every pattern is checked before any arm body
    (
        "order-a-later-pattern-before-an-earlier-arm-body",
        "  model m(s: V) -> Bool = match s with Dot -> 1 | Foo(x) -> true | _ -> false\n",
        "type",
    ),
    (
        "order-an-arm-body-level-over-a-pattern-type-error",
        "  model m(r: Result<Bool,E>, s: V) -> Bool = match s with Circle(_) -> true | Foo(_) -> false | _ -> range(1) == 1\n",
        "level:ir.seq",
    ),
    # ... and inside a pattern the structure comes before the duplicate-bind rule
    (
        "order-an-unknown-constructor-before-a-duplicate-bind",
        "  model m(t: (Bool,Bool,Bool)) -> Bool = match t with (a, a, Foo(x)) -> true | _ -> false\n",
        "type",
    ),
    (
        "order-a-field-count-before-a-shadowing-bind",
        "  model m(a: U8, s: V) -> Bool = match s with Rect(a) -> true | _ -> false\n",
        "type",
    ),
    (
        "order-a-duplicate-bind-after-a-sound-structure",
        "  model m(s: V) -> Bool = match s with Rect(a, a) -> true | _ -> false\n",
        "corpus",
    ),
    # (4) unreachable arms before fall-through, over every scrutinee type
    (
        "order-unreachable-before-fall-through-on-a-bool",
        "  model m(a: Bool) -> Bool = match a with true -> true | true -> false\n",
        "corpus",
    ),
    (
        "order-unreachable-before-fall-through-on-an-option",
        "  model m(o: Option<U8>) -> Bool = match o with None -> true | None -> false\n",
        "corpus",
    ),
    (
        "order-unreachable-before-fall-through-on-a-variant",
        "  model m(k: K) -> Bool = match k with Red -> true | Red -> false\n",
        "corpus",
    ),
    # (5) units are reported in declaration order
    (
        "order-an-earlier-level-before-a-later-type-error",
        "  model a1(a: U8) -> Bool = [a] == [a]\n  model a2(a: U8) -> Bool = a\n",
        "level:ir.list",
    ),
    (
        "order-an-earlier-type-error-before-a-later-level",
        "  model a1(a: U8) -> Bool = a\n  model a2(a: U8) -> Bool = [a] == [a]\n",
        "type",
    ),
    (
        "order-a-type-declaration-before-a-later-model",
        "  record Q(xs: List<U8>)\n  model m(a: Bytes) -> Bool = fixed<4>(a) == None\n",
        "level:ir.list",
    ),
    (
        "order-a-model-before-a-later-type-declaration",
        "  model m(a: Bytes) -> Bool = fixed<4>(a) == None\n  record Q(xs: List<U8>)\n",
        "level:fixed-cast",
    ),
    (
        "order-a-signature-before-a-later-model",
        "  op h(xs: List<U8>) -> Bool\n  model m(a: Nat) -> Bool = range(a) == a\n",
        "level:ir.list",
    ),
    (
        "order-a-model-before-a-later-signature",
        "  model m(a: Nat) -> Bool = range(a) == a\n  op h(xs: List<U8>) -> Bool\n",
        "level:ir.seq",
    ),
    (
        "order-a-claim-after-a-model",
        "  model m(a: Bytes) -> Bool = fixed<4>(a) == None\n  claim c:\n    forall a: Bool. g(a) == g(a) and [a] == [a]\n",
        "level:fixed-cast",
    ),
]

REFUSAL_GROUPS = [LEVELS, LIMITS, TYPES, CORPUS, ORDER]

#: The generator's code for each oracle code (VER04r 16.14 R35, A.2): ``corpus`` is the generator's ``ir``,
#: ``level`` and ``limit`` keep their names, and a ``type`` error is the generator's own typing error (its
#: checker names it, R13 (a)): no code to mirror.
GENERATOR_CODE = {"corpus": "ir", "level": "level", "limit": "limit"}


def refusal_documents() -> list[tuple[str, str, str]]:
    """``(name, whole document, expected)`` for every refusal.

    ``expected`` is the oracle code, or ``level:<name>`` for a ``level`` refusal that must carry that
    reserved level or feature name.  The :data:`LEVELS` rows give only the name.
    """
    rows = []
    for group in REFUSAL_GROUPS:
        for n, body, expected in group:
            if group is LEVELS:
                expected = f"level:{expected}"
            rows.append((n, _BASE + body, expected))
    return rows


def refusal_of(source: str) -> OracleError | None:
    """The oracle's refusal of a document: a raised error, or the first refusal a unit recorded (R35)."""
    try:
        spec = lower_document(parse_source(source.encode()))
    except OracleError as exc:
        return exc
    return lowering_refusal(spec)


def refusals() -> dict:
    """The refusal dump: every document with the oracle's code and message (never echoing input)."""
    rows = []
    for name, source, expected in refusal_documents():
        exc = refusal_of(source)
        assert exc is not None, f"{name} was not refused"
        code, _, level = expected.partition(":")
        assert exc.code == code and (not level or exc.level == level), (name, exc.code, exc.level, exc.message)
        rows.append(
            {
                "name": name,
                "oracle_code": exc.code,
                "oracle_level": exc.level or "",
                "message": exc.message,
                "source": source,
                "generator_code": GENERATOR_CODE.get(exc.code, ""),
            }
        )
    return {"schema": "warrant-refusals-parity-v1", "refusals": rows}


def build() -> dict[pathlib.Path, bytes]:
    doc = main().encode("utf-8")
    return {
        HERE / f"{NAME}.warrant": doc,
        HERE / f"{NAME}.oracle-lowering.json": dump(doc),
        HERE / f"{NAME}.oracle-refusals.json": canonical(refusals()),
    }


def run(argv: list[str]) -> int:
    files = build()
    if "--check" in argv:
        bad = [p.name for p, data in files.items() if not p.exists() or p.read_bytes() != data]
        for name in bad:
            print(f"DIFFERS: {name}", file=sys.stderr)
        if not bad:
            print(f"ok: {len(files)} files")
        return 1 if bad else 0
    for path, data in files.items():
        path.write_bytes(data)
        print(f"wrote {path.name} ({len(data)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(run(sys.argv[1:]))
