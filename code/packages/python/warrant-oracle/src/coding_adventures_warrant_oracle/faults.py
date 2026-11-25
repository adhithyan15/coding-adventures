"""Faults and negative controls, re-derived from the spec (VER04r sections 5.6, 8, 10.3 check 3).

What a fault is
---------------
A *fault* is a deliberate bug the lane injects into one operation to prove its claims
can see it: ``invert`` flips a ``Bool`` result, ``ignore-last-byte`` overwrites the last
byte of every ``Bytes`` argument.  VER04r section 8.1 makes a fault **data** in the corpus
(an IR expression) instead of a per-lane wrapper.  There are three kinds:

``pre``      ``rewrites``: ``[{"param": p, "expr": IR}]`` -- parameters rewritten before
             the call, every expression over *all* parameters as passed in, all applied
             simultaneously;
``post``     ``expr`` over the parameters and ``result`` -- the result is rewritten;
``replace``  ``expr`` supplies the result without trusting the call's value.

A *control* is one ``(op, fault)`` pair together with the claims that must catch it.

Why the oracle re-derives all of it
-----------------------------------
Check 3 of section 10.3: the oracle instantiates every fault from the portable names of
section 8.2 and the operation signatures, re-runs the **witness search** of section 5.6
(an independent implementation of "which claims can see this fault, and at which chunk")
and compares every fault row, control row, ``required_claims``, ``blind_claims`` and
``slim_witness`` with the corpus.

The witness search, in one paragraph
------------------------------------
For a candidate claim (one whose ``ops`` mention the control's op) that is a
``reference`` suite whose subject call is ``op(b1, .., bk)`` with every argument a plain
binder, the *reference stands in for the subject*: the oracle applies the fault to the
expected value and looks for the **lowest admitted product ordinal** at which the
faulted value differs.  Found: the claim is *witnessed* at that ordinal's chunk.  Not
found anywhere: *blind* (no enumerated case can see it).  No stand-in (a predicate
claim, a computed argument): *unproven*.  A fault whose level this oracle does not
implement is **data** (section 10.4): the row is instantiated and compared but never
evaluated, and a candidate with a stand-in is ``deferred`` to that level.

The ``ir.tuple`` templates (slice S10t-1; VER04r 8.2, 16.10 R3 and R11)
---------------------------------------------------------------------
``ok-as-err``, ``drop-consumed``, ``swap-components``, the tuple-payload ``missing-error`` and the
``Ok`` forms of ``flip-result-bit`` and ``drop-last-result-byte`` are instantiated here, as **data**:
their JSON is what the S10t-1 committed dump
(``tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-faults.json``) pins and what the generator's
S10t-2 must reproduce byte for byte (R11).  Before the flip (S10c, D29) their *evaluation* is gated
exactly as ``ir.fixed``'s was: :data:`IMPLEMENTED` held ``ir.core`` only, so a control whose fault
needed ``ir.tuple`` (or ``ir.fixed``) recorded ``{"claim": C, "deferred": "ir.tuple"}`` witnesses and no
witness search runs.  The gate is reachable only through the ``implemented`` parameter of
:func:`derive_controls`, which the tests use; no command line or environment switch exists.

``off-by-one-consumed`` (slice S10o-1; VER04r 8.2 and 16.10 [D36])
-----------------------------------------------------------------
The second component ``n`` of a tuple result, or of the tuple inside an ``Ok``, becomes ``n + 1``
when ``n`` is even and ``n - 1`` when it is odd.  Unlike the faults above, **VER04r 8.2 fixes its JSON**
(the dump follows the spec, R11), and it requires ``ir.fixed`` plus ``ir.tuple``.  Its committed faults
dump lives in a fixture directory of its own (``tests/fixtures/off-by-one-consumed-v1``), which the
generator's slice S10o-2 must reproduce byte for byte.

``saturate-add``, ``saturate-sub``, ``shift-mask`` and the scalar ``missing-error`` (slice S11f-1)
------------------------------------------------------------------------------------------------
Three more scalar faults of VER04j (2.7, 3.7) and the scalar payloads of ``missing-error`` join the
registry.  Their JSON is **normative in VER04r 8.2** (the blocks for the saturating pair, ``shift-mask``
and ``missing-error``; decisions [D50] (``missing-error`` for every payload) and [D51] (the three scalar
faults join the registry), details R31 and R33, all in 16.14), and the committed dump in
``tests/fixtures/scalar-faults-v1`` is the file the generator's slice S11f-2 must reproduce byte for
byte.  None needs ``ir.data``; the rows are live from this slice because the ``ir.fixed`` fault gate has been
open since the S10c flip.  Each builder says, beside its code, why the form equals what the Rust harness
does (``V1_FAULTS`` and ``scalar_adapter_v1`` in ``rust/warrant-grammar/src/evidence.rs``).

``field-zero`` and the record and ``Option`` payloads of ``missing-error`` (slice S11c-1)
---------------------------------------------------------------------------------------
The two faults that need ``ir.data`` (VER04r 8.2, [D49] and [D50], R31, R32): the first numeric field of a
record result becomes ``0`` (inside an ``Ok`` for a ``Result``), and ``missing-error`` builds a record from its
fields (``["ctor", "N", [..]]``) or writes ``["none", "T"]`` for an ``Option`` leaf with no parameter of its
type.  Their JSON is **normative** (8.2) and the committed dump in ``tests/fixtures/data-eval-v1`` is what the
generator's slice S11c-2 must reproduce byte for byte.  **The oracle's ``ir.data`` fault gate opens here**:
:data:`IMPLEMENTED` gains ``ir.data``, so a fault that needs it is evaluated by the witness search and a
``{"deferred": "ir.data"}`` witness is no longer derivable (``verify-v2`` rejects one, as S10c did for
``ir.fixed``).

**The first comparison of a fault row's type with a declared type (R67, R94).**  Until this slice nothing compared
the type a template compiles to with the op's declared result (or, for a ``pre`` fault, the parameter): the witness
search kept the closure and dropped the type.  :func:`_compile_fault` now types the expression with the declared
type as its *expected* type (a bytes literal in a plain-``Bytes`` position is a ``Bytes``, R67) and requires the
result to have a common type with the declaration (:func:`_fault_unify`, R94), and every fault row is typed once
in :func:`derive_controls` even when no claim can stand in for the subject.
"""

from __future__ import annotations

from dataclasses import dataclass

from . import ast as A
from . import levels as L
from .errors import OracleError
from .ir_eval import _UNKNOWN, Program
from .lower import BOOL, FIXED, NAT, OpSig, Spec, _substitute
from .suites import SuitePlan

CORE = "ir.core"
FIXED_LEVEL = "ir.fixed"
TUPLE_LEVEL = "ir.tuple"
DATA_LEVEL = "ir.data"
#: The levels whose *faults* this oracle evaluates (the witness search).  A fault that needs any
#: other level is instantiated and compared as opaque data (section 10.4) and its witnesses are
#: ``deferred``.  ``ir.fixed`` and ``ir.tuple`` are implemented (lowering, typing, cost, evaluation) and, since the
#: atomic flip (S10c, D29), their fault evaluation is open here too; ``ir.data`` follows from slice S11c-1 with no
#: flip and no version bump ([D52]: each tool opens its own ``ir.data`` gate in its own evaluator PR).  The closed
#: gate (``("ir.core",)``) stays reachable through ``derive_controls(..., implemented=...)``, never through a switch.
IMPLEMENTED = (CORE, FIXED_LEVEL, TUPLE_LEVEL, DATA_LEVEL)
#: The name a ``match`` binds the payload of an ``Ok`` to in the ``Ok`` forms (VER04r 8.2); an op
#: parameter of that name would be shadowed, a corpus error, so the templates refuse it.
OK_PAYLOAD = "ok_payload"
#: ``U64`` max shifted right by one: the mask of ``ignore-high-bit`` (section 8.2).
U64_LOW_63 = (1 << 63) - 1


def _lit(ty: str, value: int) -> list:
    return ["lit", {"type": ty, "value": str(value)}]


def _var(name: str) -> list:
    return ["var", name]


def _bytes_params(op: OpSig) -> list[str]:
    return [n for n, t in op.params if t.head == "Bytes"]


def _fail(message: str) -> OracleError:
    return OracleError("generation", f"fault: {message}")


# -- the portable fault names of section 8.2, instantiated by signature -------------------
# Each builder returns ``(kind, levels, payload)``: ``levels`` is the list of levels of the template
# (the row's ``requires`` adds the levels of the op's types, section 16.10 R9), ``payload`` is
# ``expr`` for post and replace, ``rewrites`` for pre.  A template the signature cannot take is an
# error and nothing is skipped.  ``errors`` maps an error type to its cases.


def _result(var: str = "result") -> list:
    return _var(var)


def _zero(ty) -> list:
    """The zero literal of a type (section 16.10 R3): ``0``, ``false`` or zero bytes."""
    head = ty.head
    if head == "Bool":
        return ["lit", False]
    if head in FIXED or head == "Nat":
        return _lit(head, 0)
    if head == "Bytes":
        if not ty.args:
            return ["lit", {"type": "Bytes", "value": ""}]
        if isinstance(ty.args[0], int):
            return ["lit", {"type": "Bytes", "value": "00" * ty.args[0]}]
        raise _fail("a named width has no zero literal")
    raise _fail("a component of this type has no zero literal")


def _ok_form(op: OpSig, body) -> list:
    """The ``Ok`` form (VER04r 8.2): rewrite the payload of an ``Ok``, leave an ``Err`` alone.

    ``["match", R, [[["ok",["bind","ok_payload"]], ["ok", F]], [["err_any"], R]]]`` where ``F`` is
    ``body`` applied to ``["var","ok_payload"]``.
    """
    if any(p == OK_PAYLOAD for p, _ in op.params):
        raise _fail("an operation parameter is named ok_payload")
    return [
        "match",
        _result(),
        [[["ok", ["bind", OK_PAYLOAD]], ["ok", body(_var(OK_PAYLOAD))]], [["err_any"], _result()]],
    ]


def _invert(op: OpSig, errors) -> tuple:
    if op.ret != BOOL:
        raise _fail("invert needs a Bool result")
    return "post", [CORE], ["not", _var("result")]


def _zero_option(ty) -> list:
    """The zero of ``Option<T>``: ``["none", "T"]`` with ``T`` its canonical name (Appendix A.2).

    A named width inside ``T`` (``Bytes<M>``) has no canonical name an IR ``none`` can carry (a static width is
    a parameter of the op, not a type of the corpus), so it is a ``fault``, nothing skipped, as for ``_zero``.
    """
    stack = [ty.args[0]]
    while stack:
        t = stack.pop()
        if t.head == "Bytes" and t.args and not isinstance(t.args[0], int):
            raise _fail("a named width has no zero literal")
        stack.extend(a for a in t.args if isinstance(a, A.Ty))
    return ["none", str(ty.args[0])]


def _build(ty, op: OpSig, records: dict, variants: frozenset) -> list:
    """``build(ty)`` of VER04r 8.2 (the one rule of [D50], R31): a template for a value of type ``ty``.

    * a **tuple** ``(T1,..,Tn)`` is ``["tuple", [build(T1), .., build(Tn)]]``; a **record** ``N`` with field types
      ``T1..Tn`` is ``["ctor", "N", [build(T1), .., build(Tn)]]``: a composite is always **built**, never taken
      whole from a parameter of its type (R31 (a); that is R3 as the tools have written it since S10t);
    * a **leaf** (``Bool``, ``Nat``, a fixed-width type, ``Bytes``, ``Bytes<N>``, ``Option<T>`` or a variant) is
      ``["var", p]`` for the first parameter, in parameter order, whose type is **exactly** ``ty``, else the **zero**
      of ``ty`` (``false``, ``0``, an empty or all-zero ``Bytes``, ``["none", "T"]``); a variant has no zero (its
      default value is the decision ``wrong-ctor`` needs, [D49]);
    * anything else (a ``Result``, a list) has no template.
    """
    if ty.head == "()":
        return ["tuple", [_build(t, op, records, variants) for t in ty.args]]
    if ty.head in records:
        return ["ctor", ty.head, [_build(t, op, records, variants) for _, t in records[ty.head]]]
    if not (ty.head in ("Bool", "Nat", "Bytes", "Option") or ty.head in FIXED or ty.head in variants):
        raise _fail("missing-error has no template for this payload type")
    param = next((n for n, t in op.params if t == ty), None)
    if param is not None:
        return _var(param)
    if ty.head in variants:  # no parameter of its type, and a variant has no zero (its default is wrong-ctor's decision)
        raise _fail("missing-error has no template for this payload type")
    return _zero_option(ty) if ty.head == "Option" else _zero(ty)


def _levels_of(expr: list) -> list:
    """The template levels of a ``build`` result (8.2): ``ir.core`` for a bare ``var``/``lit``, else
    ``ir.tuple`` if it holds a ``tuple`` node and ``ir.data`` if it holds a ``ctor`` or ``none`` node, sorted."""
    nodes = [expr]
    found: set = set()
    while nodes:
        node = nodes.pop()
        if isinstance(node, list) and node and isinstance(node[0], str):
            if node[0] == "tuple":
                found.add(TUPLE_LEVEL)
            elif node[0] in ("ctor", "none"):
                found.add(DATA_LEVEL)
            nodes.extend(node[1:])
        elif isinstance(node, list):
            nodes.extend(node)
    return sorted(found) or [CORE]


def _missing_error(op: OpSig, errors, records: dict, variants: frozenset) -> tuple:
    """``if is_err(result) then Ok(<payload>) else result`` (VER04r 8.2, the one rule for every payload).

    ``P = build(X)`` for the payload type ``X`` of the ``Result`` (:func:`_build`):

    * A ``Bytes`` payload (any ``Bytes<M>`` too) is the first ``Bytes`` argument, matched loosely (any
      ``Bytes<M>``, the rule since S3c that R31 (16.14) keeps for byte parity); with **no** ``Bytes`` parameter it is the zero
      ``Bytes`` literal (new in S11f-1: such an op was refused before).
    * A **tuple** payload (R3, ``ir.tuple``) is the tuple of ``build`` of its components, and a **record** payload
      (S11c-1, ``ir.data``) the ``ctor`` of ``build`` of its fields; a component that is itself a tuple or a record is
      built in turn (a nested tuple had no template before S11c-1).
    * A **leaf** payload (``Bool``, ``Nat``, a fixed-width type, S11f-1; an ``Option`` or a variant, S11c-1) is the first
      parameter whose type is *exactly* the payload type, else the type's zero: a literal, or ``["none", "T"]`` for an
      ``Option``.  A variant with no such parameter has no template (``fault``).

    Cost (a check): ``4 + cost(P)``, 5 for a leaf.  ``requires`` is ``["ir.core"]`` when ``P`` is a ``var`` or a
    ``lit`` and otherwise ``ir.tuple`` if it holds a ``tuple`` node together with ``ir.data`` if it holds a ``ctor`` or
    a ``none`` node, sorted (8.2, R31 (e)).  The harness (``scalar_adapter_v1``) replaces an ``Err`` with ``Ok`` of a
    value built from the arguments; the ``Ok`` form here is the portable statement of it, and the difference from the
    harness (this takes a parameter where the harness returns the first element of the payload's domain, so
    ``Ok(a)`` for ``Result<U8,E>`` where the harness has ``Ok(0)``) is the one [D50] records.

    ``records`` is the document's record table (:attr:`.lower.Spec.records`) and ``variants`` the names of its
    variants: there is **one rule**, no call that does not say what the document declares (owner decision D71: the
    data-blind bridge S11c-1 first shipped, which refused an ``Option`` payload for the sake of the committed
    ``scalar-faults-v1`` refusals dump, is gone and that dump's row with it).  A variant leaf with no parameter of
    its type is refused everywhere with ``missing-error has no template for this payload type``.
    """
    if op.ret.head != "Result":
        raise _fail("missing-error needs a Result result")
    payload = op.ret.args[0]
    if payload.head == "Bytes":
        params = _bytes_params(op)
        leaf = _var(params[0]) if params else _zero(payload)
        return "replace", [CORE], ["if", ["is_err", _var("result")], ["ok", leaf], _var("result")]
    built = _build(payload, op, records, variants)
    return "replace", _levels_of(built), ["if", ["is_err", _var("result")], ["ok", built], _var("result")]


def _swap_choice(op: OpSig, errors) -> tuple:
    if ("choice", BOOL) not in op.params:
        raise _fail("swap-choice needs a Bool parameter named choice")
    return "pre", [CORE], [{"param": "choice", "expr": ["not", _var("choice")]}]


def _off_by_one(op: OpSig, errors) -> tuple:
    if op.ret != NAT and op.ret.head not in FIXED:
        raise _fail("off-by-one needs a Nat or fixed-width result")
    return "post", [CORE], ["add", _var("result"), _lit(str(op.ret), 1)]


def _truncate_last(op: OpSig, errors) -> tuple:
    if op.ret.head != "Bytes":
        raise _fail("truncate-last needs a Bytes result")
    r = _var("result")
    length = ["len", r]
    expr = ["if", ["gt", length, _lit("Nat", 0)], ["take", r, ["sub", length, _lit("Nat", 1)]], r]
    return "post", [CORE], expr


def _ignore_last_byte(op: OpSig, errors) -> tuple:
    names = _bytes_params(op)
    if not names:
        raise _fail("ignore-last-byte needs a Bytes parameter")
    rewrites = []
    for name in names:
        p = _var(name)
        length = ["len", p]
        expr = [
            "if",
            ["gt", length, _lit("Nat", 0)],
            ["concat", ["take", p, ["sub", length, _lit("Nat", 1)]], ["lit", {"type": "Bytes", "value": "00"}]],
            p,
        ]
        rewrites.append({"param": name, "expr": expr})
    return "pre", [CORE], rewrites


def _xor_lsb(op: OpSig, errors) -> tuple:
    if op.ret.head not in FIXED:
        raise _fail("xor-lsb needs a fixed-width result")
    return "post", [FIXED_LEVEL], ["bit_xor", _var("result"), _lit(str(op.ret), 1)]


def _ignore_high_bit(op: OpSig, errors) -> tuple:
    names = [n for n, t in op.params if t.head == "U64"]
    if not names:
        raise _fail("ignore-high-bit needs a U64 parameter")
    return (
        "pre",
        [FIXED_LEVEL],
        [
            {"param": n, "expr": ["bit_and", _var(n), _lit("U64", U64_LOW_63)]}
            for n in names
        ],
    )


def _flip_expr(src: list) -> list:
    """The ``Bytes`` form of ``flip-result-bit`` over the value ``src`` (VER04r 8.2, exact JSON)."""
    n0, n1 = _lit("Nat", 0), _lit("Nat", 1)
    u1, u0 = _lit("U8", 1), _lit("U8", 0)
    b01 = ["lit", {"type": "Bytes", "value": "01"}]
    return [
        "if",
        ["gt", ["len", src], n0],
        [
            "concat",
            ["be_bytes", ["bit_xor", ["unwrap_or", ["at", src, n0], u0], u1]],
            ["drop", src, n1],
        ],
        b01,
    ]


def _drop_last_expr(src: list) -> list:
    """The ``Bytes`` form of ``drop-last-result-byte`` over ``src`` (VER04r 8.2, exact JSON)."""
    n0, n1 = _lit("Nat", 0), _lit("Nat", 1)
    b00 = ["lit", {"type": "Bytes", "value": "00"}]
    return ["if", ["gt", ["len", src], n0], ["take", src, ["sub", ["len", src], n1]], b00]


def _flip_result_bit(op: OpSig, errors) -> tuple:
    """``flip-result-bit`` [D27, D35]: a fixed-width, ``Bytes``/``Bytes<N>`` or ``Result`` of those."""
    ret = op.ret
    if ret.head in FIXED:
        return "post", [FIXED_LEVEL], ["bit_xor", _var("result"), _lit(str(ret), 1)]
    if ret.head == "Bytes":
        return "post", [FIXED_LEVEL], _flip_expr(_var("result"))
    if ret.head == "Result" and ret.args[0].head == "Bytes":
        return "post", [FIXED_LEVEL, TUPLE_LEVEL], _ok_form(op, _flip_expr)
    if ret.head == "Result" and ret.args[0].head in FIXED:
        t = str(ret.args[0])
        return (
            "post",
            [FIXED_LEVEL, TUPLE_LEVEL],
            _ok_form(op, lambda src: ["bit_xor", src, _lit(t, 1)]),
        )
    raise _fail("flip-result-bit needs a fixed-width, Bytes or Result of those result")


def _drop_last_result_byte(op: OpSig, errors) -> tuple:
    """``drop-last-result-byte`` [D27, D35]: ``Bytes``/``Bytes<N>`` or a ``Result`` of one."""
    ret = op.ret
    if ret.head == "Bytes":
        return "post", [CORE], _drop_last_expr(_var("result"))
    if ret.head == "Result" and ret.args[0].head == "Bytes":
        return "post", [CORE, TUPLE_LEVEL], _ok_form(op, _drop_last_expr)
    raise _fail("drop-last-result-byte needs a Bytes or Result<Bytes, E> result")


def _tuple_target(op: OpSig, name: str) -> tuple:
    """``(tuple type, inside_ok)`` of a composite fault's result, else a fault error."""
    ret = op.ret
    if ret.head == "()":
        return ret, False
    if ret.head == "Result" and ret.args[0].head == "()":
        return ret.args[0], True
    raise _fail(f"{name} needs a tuple or Result<tuple, E> result")


def _project(src: list, index: int) -> list:
    return ["project", src, index]


def _drop_consumed(op: OpSig, errors) -> tuple:
    """The second component replaced by ``0`` (VER04j 4.8), inside an ``Ok`` for a ``Result``."""
    tup, inside = _tuple_target(op, "drop-consumed")
    second = tup.args[1]
    if second.head not in FIXED and second.head != "Nat":
        raise _fail("drop-consumed needs a numeric second component")

    def rewrite(src: list) -> list:
        n = len(tup.args)
        return ["tuple", [_project(src, 0), _zero(second), *(_project(src, i) for i in range(2, n))]]

    expr = _ok_form(op, rewrite) if inside else rewrite(_var("result"))
    return "post", [TUPLE_LEVEL], expr


def _off_by_one_consumed(op: OpSig, errors) -> tuple:
    """``off-by-one-consumed`` [D36]: the second component ``n`` becomes ``n + 1`` (even) or ``n - 1`` (odd).

    The fault of VER04j 4.8 ("right value, wrong bytes consumed") as the Rust harness applies it:
    the pair ``2k, 2k+1`` is exchanged, so both directions occur and the value never leaves its
    type.  Applies exactly where ``drop-consumed`` does (a tuple, or a ``Result`` of one, whose second
    component is ``Nat`` or fixed-width) and its JSON is the normative block of VER04r 8.2:

    * ``Nat`` component ``C``:
      ``if unwrap_or(rem(C, 2), 0) == 0 then C + 1 else C - 1`` (``Int`` literals in the parity
      test, ``Nat`` literals in the arms; ``add``/``sub`` on a ``Nat`` are exact, so the value is
      always a ``Nat`` value: an odd ``n`` has ``n >= 1`` and the largest ``Nat`` is odd);
    * fixed-width component: ``bit_xor(C, 1)``, the harness's wrapping ``+ 1`` / ``- 1`` on the bit
      pattern (an even pattern has bit 0 clear, an odd one has it set, so no carry or borrow occurs).

    ``requires`` is ``ir.fixed`` and ``ir.tuple`` in all four combinations.  An ``Err`` is unchanged
    (the ``err_any`` arm of the ``Ok`` form returns ``result``).
    """
    tup, inside = _tuple_target(op, "off-by-one-consumed")
    second = tup.args[1]
    if second.head not in FIXED and second.head != "Nat":
        raise _fail("off-by-one-consumed needs a numeric second component")

    def rewrite(src: list) -> list:
        c = _project(src, 1)
        if second.head == "Nat":
            x = [
                "if",
                ["eq", ["unwrap_or", ["rem", c, _lit("Int", 2)], _lit("Int", 0)], _lit("Int", 0)],
                ["add", c, _lit("Nat", 1)],
                ["sub", c, _lit("Nat", 1)],
            ]
        else:
            x = ["bit_xor", c, _lit(second.head, 1)]
        n = len(tup.args)
        return ["tuple", [_project(src, 0), x, *(_project(src, i) for i in range(2, n))]]

    expr = _ok_form(op, rewrite) if inside else rewrite(_var("result"))
    return "post", [FIXED_LEVEL, TUPLE_LEVEL], expr


def _swap_components(op: OpSig, errors) -> tuple:
    """The components of a homogeneous tuple exchanged (reversed, so a pair is swapped)."""
    tup, inside = _tuple_target(op, "swap-components")
    if len(set(tup.args)) != 1:
        raise _fail("swap-components needs a homogeneous tuple")

    def rewrite(src: list) -> list:
        return ["tuple", [_project(src, i) for i in reversed(range(len(tup.args)))]]

    expr = _ok_form(op, rewrite) if inside else rewrite(_var("result"))
    return "post", [TUPLE_LEVEL], expr


def _ok_as_err(op: OpSig, errors) -> tuple:
    """Success reported as the first error case of the error type (VER04j 4.8)."""
    if op.ret.head != "Result" or op.ret.args[0].head != "()":
        raise _fail("ok-as-err needs a Result<tuple, E> result")
    cases = (errors or {}).get(op.ret.args[1].head)
    if not cases:
        raise _fail("ok-as-err needs the error type's cases")
    expr = [
        "match",
        _result(),
        [[["ok", ["wild"]], ["err", cases[0]]], [["err_any"], _result()]],
    ]
    return "post", [TUPLE_LEVEL], expr


def _field_zero(op: OpSig, errors, records: dict, variants: frozenset) -> tuple:
    """``field-zero`` [D49, R32]: the first numeric field of a record result becomes ``0`` (8.2, exact JSON).

    The fault of VER04j 4.8 as the Rust harness applies it (``fault_field_zero``): inside an ``Ok`` for a
    ``Result``, the first field, in declaration order, whose type is ``Nat`` or fixed-width (the harness tests
    for an integer *value*, ``V::Int`` or ``V::Fixed``, and a field's declared type decides which; ``Int`` is never
    a field type) is replaced by the zero of its type and every other field is read back unchanged::

        F(S) = ["ctor", "N", [["field", S, 0], .., ["lit", {"type": "T", "value": "0"}], .., ["field", S, n-1]]]
        bare record result       F(["var", "result"])
        Result<N,E>              ["match", R, [[["ok", ["bind", "ok_payload"]], ["ok", F(["var","ok_payload"])]],
                                               [["err_any"], R]]]

    Applies to ``N`` or ``Result<N,E>`` with ``N`` a record having at least one numeric field.  Anything else has
    no template (code ``fault``, nothing skipped): a record with no numeric field (arithmetic's ``AluResult``), a
    variant, a tuple, a scalar, ``Bytes``, an ``Option`` (of a record too) and a ``Result`` whose payload is not a
    record.  Kind ``post``; ``requires`` is ``["ir.data"]`` bare and ``["ir.data", "ir.tuple"]`` in the ``Ok`` form
    (``ctor`` and ``field`` are ``ir.data``, ``match`` on a ``Result`` is ``ir.tuple``, and ``ir.core`` is not listed
    although ``lit`` and ``ok`` are core tags).  Typing: ``field`` has the declared type of its field and the literal
    the type of field ``k``, so ``F`` has exactly the type ``N``.  An ``Err`` is returned unchanged.  Cost (a check):
    ``2n`` bare, ``2n + 3`` in the ``Ok`` form.
    """
    ret = op.ret
    inside = ret.head == "Result"
    target = ret.args[0] if inside else ret
    if target.head not in records:
        raise _fail("field-zero needs a record or Result<record, E> result")
    fields = records[target.head]
    k = next((i for i, (_, t) in enumerate(fields) if t.head in FIXED or t.head == "Nat"), None)
    if k is None:
        raise _fail("field-zero needs a record with a numeric field")
    zero = _lit(fields[k][1].head, 0)

    def rewrite(src: list) -> list:
        return ["ctor", target.head, [zero if i == k else ["field", src, i] for i in range(len(fields))]]

    expr = _ok_form(op, rewrite) if inside else rewrite(_var("result"))
    return "post", [DATA_LEVEL, TUPLE_LEVEL] if inside else [DATA_LEVEL], expr


def _saturate(op: OpSig, name: str, adding: bool) -> tuple:
    """``saturate-add`` / ``saturate-sub`` [saturating pair, VER04r 8.2]: the saturating sum or difference.

    Applies to an op whose **first two** parameters share one fixed-width type ``T`` and whose result is
    ``T`` or ``Result<T,E>`` (anything else, a ``Nat`` parameter, two widths, another result, is refused
    with the code ``fault``).  Kind ``replace``.  With ``A``, ``B`` the two parameters::

        unsigned add  unwrap_or(checked_add(A, B), max_value(T))
        signed   add  unwrap_or(checked_add(A, B), if B < 0 then min_value(T) else max_value(T))
        unsigned sub  unwrap_or(checked_sub(A, B), min_value(T))
        signed   sub  unwrap_or(checked_sub(A, B), if B < 0 then max_value(T) else min_value(T))

    Why it equals the harness: ``V1_FAULTS`` has ``Replace("p0.saturating_add(p1)")`` and
    ``Replace("p0.saturating_sub(p1)")`` (``rust/warrant-grammar/src/evidence.rs``), and in
    ``scalar_adapter_v1`` a ``Replace`` arm *replaces the call*: the export is never called, nothing can
    panic and the adapter returns ``Ok`` of the value on every input.  Hence the ``["ok", V]`` wrapper
    for a ``Result`` without any test of ``result``.  ``checked_*`` is ``Some`` exactly when the exact
    value is in ``T``, so ``unwrap_or`` gives it back; on overflow only one bound is possible: an
    unsigned sum passes ``MAX``, an unsigned difference ``0`` (= ``min_value``, kept rather than a ``0``
    literal so the four rows differ only in tag and bound), a signed sum goes up iff ``B > 0``, a signed
    difference up iff ``B < 0``.  ``B = 0`` never overflows, so the choice made for it is never used.
    ``requires`` is the template's ``["ir.fixed"]`` (the union over its tags would add ``ir.core``).
    Cost (a check): ``V`` is 5 unsigned and 9 signed; the ``Result`` form adds 1.
    """
    t = op.params[0][1] if len(op.params) >= 2 else None
    if t is None or t.head not in FIXED or op.params[1][1] != t:
        raise _fail(f"{name} needs two leading parameters of one fixed-width type")
    if op.ret != t and not (op.ret.head == "Result" and op.ret.args[0] == t):
        raise _fail(f"{name} needs a result of the parameters' type or a Result of it")
    a, b = _var(op.params[0][0]), _var(op.params[1][0])
    ty = t.head
    signed = FIXED[ty][1]
    mx, mn = ["max_value", ty], ["min_value", ty]
    checked = ["checked_add" if adding else "checked_sub", a, b]
    if not signed:
        bound = mx if adding else mn
    else:
        neg = ["lt", b, _lit(ty, 0)]
        bound = ["if", neg, mn, mx] if adding else ["if", neg, mx, mn]
    v = ["unwrap_or", checked, bound]
    return "replace", [FIXED_LEVEL], v if op.ret == t else ["ok", v]


def _saturate_add(op: OpSig, errors) -> tuple:
    return _saturate(op, "saturate-add", True)


def _saturate_sub(op: OpSig, errors) -> tuple:
    return _saturate(op, "saturate-sub", False)


def _shift_mask(op: OpSig, errors) -> tuple:
    """``shift-mask`` [VER04r 8.2]: the last parameter ``p`` becomes ``p mod w``, ``w`` the FIRST parameter's width.

    Kind ``pre`` with the one rewrite ``{"param": p, "expr": X}``; ``X`` is
    ``unwrap_or(rem(p, w), 0)`` for an unsigned last parameter and ``unwrap_or(rem_trunc(p, w), 0)`` for a
    signed one, the literals in the last parameter's own type ``L``.  Needs a fixed-width first
    parameter (the width) and a fixed-width last one; a ``Nat`` (the harness computes it as ``u32``, a
    host type the corpus cannot see) or a non-integer is refused with the code ``fault``.

    Why it equals the harness: ``V1_FAULTS`` has ``PreLast("p % {w}")`` (``evidence.rs``) and the
    generated adapter line is ``let p0 = if fault == Fault::ShiftMask { let p = p0; p % 8 } else { p0 };``
    (logic-gates' ``warrant_evidence.rs``): the last argument is rewritten before the call, in its own
    integer type, with ``{w}`` the width of the *first* argument's type, and the real export is then
    called.  Rust's ``%`` is the remainder on an unsigned type and the *truncating* remainder (sign of
    the dividend) on a signed one: ``rem`` and ``rem_trunc``.  ``w`` is never 0 or -1 and always fits
    ``L``, so the ``Option`` is never ``None`` and the default is never taken.  Template level
    ``["ir.fixed"]``; cost (a check) 5.
    """
    if not op.params:
        raise _fail("shift-mask needs parameters")
    first, (last_name, last) = op.params[0][1], op.params[-1]
    if first.head not in FIXED or last.head not in FIXED:
        raise _fail("shift-mask needs a fixed-width first and last parameter")
    width = FIXED[first.head][0]
    rem = "rem_trunc" if FIXED[last.head][1] else "rem"
    x = ["unwrap_or", [rem, _var(last_name), _lit(last.head, width)], _lit(last.head, 0)]
    return "pre", [FIXED_LEVEL], [{"param": last_name, "expr": x}]


REGISTRY = {
    "invert": _invert,
    "missing-error": _missing_error,
    "swap-choice": _swap_choice,
    "off-by-one": _off_by_one,
    "truncate-last": _truncate_last,
    "ignore-last-byte": _ignore_last_byte,
    "xor-lsb": _xor_lsb,
    "ignore-high-bit": _ignore_high_bit,
    "flip-result-bit": _flip_result_bit,
    "drop-last-result-byte": _drop_last_result_byte,
    "ok-as-err": _ok_as_err,
    "drop-consumed": _drop_consumed,
    "swap-components": _swap_components,
    "off-by-one-consumed": _off_by_one_consumed,
    "saturate-add": _saturate_add,
    "saturate-sub": _saturate_sub,
    "shift-mask": _shift_mask,
    "field-zero": _field_zero,
}
#: The builders that read the document's records (and the names of its variants): called with two more arguments.
_DATA_BUILDERS = frozenset(["missing-error", "field-zero"])


def fault_row(
    op: OpSig,
    fault: str,
    errors: dict | None = None,
    data_names: frozenset = frozenset(),
    records: dict | None = None,
) -> dict:
    """The ``faults`` row for ``fault`` on ``op`` (section 8.1; keys per section 16.6 item 3).

    ``requires`` is the sorted union of the template's levels and the levels of the op's own types
    (R9): a fault on an operation whose parameter or result type is or contains a tuple needs
    ``ir.tuple`` even where the template names no tuple tag.  ``errors`` (error type -> cases) is
    needed by ``ok-as-err``; ``data_names`` are the record and variant names, for ``ir.data``.  ``records`` maps a
    record name to its ``[(field, type), ..]`` (:attr:`.lower.Spec.records`), which ``field-zero`` and the record
    payload of ``missing-error`` read (S11c-1); a name in ``data_names`` that is not in ``records`` is a variant.
    A caller always passes the document's tables (D71: there is no data-blind call); with none given the document
    declares no record and no variant (``records=None`` is ``{}``), so ``field-zero`` has no template and a
    ``missing-error`` payload is built from tuples and scalars only.
    """
    builder = REGISTRY.get(fault)
    if builder is None:
        raise _fail("an unknown fault name")
    if fault in _DATA_BUILDERS:
        records = records or {}
        variants = frozenset(data_names) - frozenset(records)
        kind, levels, payload = builder(op, errors, records, variants)
    else:
        kind, levels, payload = builder(op, errors)
    needs = set(levels)
    for _, ty in [*op.params, ("result", op.ret)]:
        needs |= L.type_levels(ty, data_names)
    row = {"fault": fault, "op": op.name, "kind": kind, "requires": sorted(needs)}
    row["rewrites" if kind == "pre" else "expr"] = payload
    return row


# -- stand-ins -------------------------------------------------------------------------------


@dataclass
class _Search:
    """One (suite, fault) witness search."""

    claim: str
    fault: dict
    plan: SuitePlan
    # per-position: (parameter name, binder name)
    positions: list
    statics: dict
    witness: int | None = None  # lowest admitted product ordinal at which the fault is visible
    found: bool = False


def _stand_in(plan: SuitePlan, op: OpSig, fault: dict) -> tuple[list, dict] | None:
    """``(positions, statics)`` if the suite's reference can stand in for the subject, else ``None``.

    The subject side must be exactly ``op(b1, .., bk)`` with every argument a plain
    binder.  For a ``pre`` fault a binder passed at two positions must be rewritten at
    both or at neither: otherwise there is no single rewritten binder and the claim is
    *unproven* (section 16.6 item 5).
    """
    suite = plan.suite
    if suite.kind != "reference":
        return None
    node = suite.subject
    if not (isinstance(node, list) and node and node[0] == "subject_op" and node[1] == op.name):
        return None
    names = {b.name for b in plan.binders}
    args = node[2]
    if not all(isinstance(a, list) and len(a) == 2 and a[0] == "var" and a[1] in names for a in args):
        return None
    positions = [(p, a[1]) for (p, _), a in zip(op.params, args)]
    statics: dict = {}
    if len(node) == 4:
        statics = dict(zip(op.statics, node[3]))
    if fault["kind"] == "pre":
        rewritten = {r["param"] for r in fault["rewrites"]}
        by_binder: dict = {}
        for p, b in positions:
            by_binder.setdefault(b, set()).add(p in rewritten)
        if any(len(flags) > 1 for flags in by_binder.values()):
            return None
    return positions, statics


def _fault_component(a: A.Ty, b: A.Ty) -> A.Ty | None:
    """The common type of two *tuple components* of a fault expression and its declaration, or ``None``.

    Inside a tuple nothing lifts (R51: a tuple is invariant, and R67's second test needs a one-byte literal in a
    ``Bytes<2>`` component to be refused), with the two exceptions a fault row needs: the unknown error type of a
    bare ``ok`` or ``err`` merges (as in :func:`.ir_eval._common_exact`), and ``Nat`` and ``Int`` unify to ``Int``,
    the one widening VER04r 8.2 states for a fault: ``off-by-one-consumed``'s second component is ``Int`` where the
    op declares ``Nat`` ("a ``Nat`` component widens to ``Int``", committed in the wasm-leb128 fixture).
    """
    if a == b:
        return a
    if {a.head, b.head} == {"Nat", "Int"}:
        return A.Ty("Int")
    if a.head == b.head == "Option":
        inner = _fault_component(a.args[0], b.args[0])
        return A.Ty("Option", (inner,)) if inner is not None else None
    if a.head == b.head == "Result":
        payload = (
            b.args[0] if a.args[0] == _UNKNOWN else a.args[0] if b.args[0] == _UNKNOWN else _fault_component(a.args[0], b.args[0])
        )
        error = a.args[1] if b.args[1] == _UNKNOWN else b.args[1] if a.args[1] == _UNKNOWN else a.args[1]
        ok = payload is not None and (a.args[1] == b.args[1] or _UNKNOWN in (a.args[1], b.args[1]))
        return A.Ty("Result", (payload, error)) if ok else None
    if a.head == b.head == "()" and len(a.args) == len(b.args):
        parts = [_fault_component(x, y) for x, y in zip(a.args, b.args)]
        return None if any(x is None for x in parts) else A.Ty("()", tuple(parts))
    return None


def _fault_unify(a: A.Ty, b: A.Ty) -> A.Ty | None:
    """The common type of a fault expression's type and the declared type it is compared with (R94), or ``None``.

    R51's ``unify`` (``common_type``): equal types; ``Nat`` and ``Int`` to ``Int``; two ``Bytes`` types of different
    or forgotten width to ``Bytes``; ``Option`` and ``Result`` of types that unify, a ``Result`` with the same error
    type, or the unknown error type of a bare ``ok`` (``?``).  It is **not** "flows to" (R94): a fault may change a
    length, so plain ``Bytes`` against a declared ``Bytes<32>`` is accepted.  A tuple unifies only component by
    component through :func:`_fault_component`.  A pair with no common type (a ``Bool`` for a ``Bytes`` parameter, a
    ``Bytes`` for a ``U8`` result) is ``None``.
    """
    if a == b:
        return a
    if {a.head, b.head} == {"Nat", "Int"}:
        return A.Ty("Int")
    if a.head == b.head == "Bytes":
        return A.Ty("Bytes")
    if a.head == b.head == "Option":
        inner = _fault_unify(a.args[0], b.args[0])
        return A.Ty("Option", (inner,)) if inner is not None else None
    if a.head == b.head == "Result":
        if not (a.args[1] == b.args[1] or _UNKNOWN in (a.args[1], b.args[1])):
            return None
        payload = b.args[0] if a.args[0] == _UNKNOWN else a.args[0] if b.args[0] == _UNKNOWN else _fault_unify(a.args[0], b.args[0])
        error = a.args[1] if b.args[1] == _UNKNOWN else b.args[1]
        return A.Ty("Result", (payload, error)) if payload is not None else None
    return _fault_component(a, b)


def _typed(prog: Program, op: OpSig, expr: list, tenv: dict, declared: A.Ty | None, what: str):
    """Compile ``expr`` with ``declared`` as its expected type and require a common type with it (R67, R94).

    ``declared`` is ``None`` for a ``pre`` rewrite that names no parameter of the op: there is nothing to compare, and
    what such a row means is not this check's business (it was compiled, and never compared, before S11c-1).

    The comparison is made on **every branch**, not only on the whole: an ``if`` merges the types of its branches
    (:func:`.ir_eval._common`) and a branch ``ok(true)`` beside ``result`` would otherwise pass for ``Result<U8,E>``,
    so each branch of an ``if`` and each arm of a ``match`` is compared with the declared type as it is compiled
    (:attr:`.ir_eval.Program.branch_check`), and then the whole expression is.
    """

    def check(got: A.Ty, want: A.Ty) -> None:
        if _fault_unify(got, want) is None:
            raise _fail(f"the {what} of a fault on `{op.name}` has no common type with the declared type")

    prog.branch_check = check
    try:
        compiled = prog.compile(expr, tenv, False, expected=declared)
    finally:
        prog.branch_check = None
    if declared is not None and _fault_unify(compiled.ty, declared) is None:
        raise _fail(f"the {what} of a fault on `{op.name}` has no common type with the declared type")
    return compiled


def _compile_fault(prog: Program, op: OpSig, fault: dict, statics: dict):
    """Type-check and compile the fault expressions at the stand-in's static widths.

    Since slice S11c-1 the type an expression compiles to is **compared** with the declared one (VER04r 16.17 R67,
    16.19 R94), which nothing did before: a ``pre`` rewrite with the type of the parameter it rewrites, a ``post`` or
    ``replace`` expression with the op's result, each as the *expected* type of its position (so a zero literal has
    the type it is the zero of, R67) and each by the common-type rule of R94 (:func:`_fault_unify`), never by "flows
    to", and in every branch of an ``if`` and arm of a ``match`` (:func:`_typed`).  A pair with no common type is a
    ``generation`` error (code ``fault``); no committed row has one.
    """
    tenv = {p: _substitute(t, statics) for p, t in op.params}
    if fault["kind"] == "pre":
        return [
            (r["param"], _typed(prog, op, r["expr"], tenv, tenv.get(r["param"]), "rewrite").fn)
            for r in fault["rewrites"]
        ]
    tenv["result"] = _substitute(op.ret, statics)
    return _typed(prog, op, fault["expr"], tenv, tenv["result"], "expression").fn


def _search_suite(prog: Program, ops: dict, searches: list[_Search]) -> None:
    """One enumeration pass over a suite serves every pending fault (lowest ordinal wins)."""
    plan = searches[0].plan
    expected = plan.expected
    compiled = [
        (s, _compile_fault(prog, ops[s.fault["op"]], s.fault, s.statics)) for s in searches
    ]
    pending = list(compiled)
    try:
        for ordinal, env in plan.iter_admitted():
            want = expected(env)
            for item in list(pending):
                s, fn = item
                penv = {p: env[b] for p, b in s.positions}
                if s.fault["kind"] == "pre":
                    new = {param: rewrite(penv) for param, rewrite in fn}
                    renv = dict(env)
                    given: dict = {}
                    for p, b in s.positions:
                        if p in new:
                            if b in given and given[b] != repr(new[p]):
                                raise _fail("one binder is rewritten to two different values")
                            given[b] = repr(new[p])
                            renv[b] = new[p]
                    got = expected(renv)
                else:
                    penv["result"] = want
                    got = fn(penv)
                if repr(got) != repr(want):
                    s.witness = ordinal
                    s.found = True
                    pending.remove(item)
            if not pending:
                return
    except OracleError as exc:
        raise _fail("a fault failed to evaluate on the stand-in") from exc


def _gated(row: dict, implemented: tuple) -> str | None:
    """The first level of the fault's ``requires`` the gate does not implement, or ``None``."""
    return next((lv for lv in row["requires"] if lv not in implemented), None)


def derive_controls(
    spec: Spec,
    prog: Program,
    plans: dict[str, SuitePlan],
    faults: list[tuple[str, str]],
    chunk_size: int,
    accept_blind: list[tuple[str, str]] | None = None,
    implemented: tuple = IMPLEMENTED,
) -> tuple[list[dict], list[dict], dict[str, set[int]], list[str]]:
    """``(fault rows, control rows, witness chunks per claim, accepted blind faults)``.

    The input is the profile's ``fault`` lines and its ``accept_blind`` lines.  A fault every
    candidate of which is blind fails generation unless an ``accept_blind`` line names it; its
    control row is then emitted with no ``required_claims`` and its control id is listed (sorted
    by bytes) in the fourth result, which ``accepted_blind_faults`` records ([D15], 16.8).  An
    ``accept_blind`` line for a fault that is *not* all-blind is a profile error (``corpus``).

    Rows are sorted by ``(op, fault)`` by bytes, independent of the order of the
    profile lines (section 16.6 item 2); the three claim lists keep suite order.

    ``implemented`` is the **fault-evaluation gate** (the levels whose faults the witness search
    evaluates).  The shipped value is :data:`IMPLEMENTED` (``ir.core``, ``ir.fixed``, ``ir.tuple``, ``ir.data``): a fault whose ``requires``
    names any other level is emitted as data and each of its candidate claims that has a stand-in is
    ``{"claim": C, "deferred": L}`` with ``L`` the first level of the fault's ``requires`` that is not
    implemented (a fault needing two unimplemented levels names the first in sorted order).  The
    atomic flip (S10c) opened the gate to ``ir.fixed`` and ``ir.tuple``; only tests pass a narrower
    value (the closed gate is ``("ir.core",)``).
    """
    for name, op in spec.ops.items():
        if any(p == "result" for p, _ in op.params):
            raise OracleError("generation", f"W0701: operation {name} has a parameter named result")
    wanted = sorted(
        {(op, fault) for op, fault in faults}, key=lambda r: (r[0].encode(), r[1].encode())
    )
    accept = set(accept_blind or ())
    accepted_ids: list[str] = []
    rows: list[dict] = []
    data_names = frozenset([*spec.records, *spec.variants])
    for op_name, fault in wanted:
        if op_name not in spec.ops:
            raise _fail("a profile fault names an undeclared operation")
        row = fault_row(spec.ops[op_name], fault, spec.errors, data_names, dict(spec.records))
        if not spec.ops[op_name].statics:
            # Every row is typed once, even if no claim can stand in for the subject (S11c-1, R67): a row whose
            # candidates are all ``unproven`` or ``deferred`` would otherwise never be compiled.  An op with a
            # width parameter is typed at the widths of each stand-in instead (R94: a named width cannot be
            # unified with a literal one).
            _compile_fault(prog, spec.ops[op_name], row, {})
        rows.append(row)
    # Phase 1: classify every candidate; collect the searches per suite.
    classified: dict[tuple, list] = {}
    per_suite: dict[str, list[_Search]] = {}
    for row in rows:
        op = spec.ops[row["op"]]
        entries = []
        for suite in spec.suites:
            if op.name not in suite.ops:
                continue
            plan = plans[suite.claim]
            stand = _stand_in(plan, op, row)
            if stand is None:
                entries.append((suite, "unproven", None))
            elif _gated(row, implemented) is not None:
                entries.append((suite, "deferred", None))
            else:
                search = _Search(suite.claim, row, plan, stand[0], stand[1])
                per_suite.setdefault(suite.claim, []).append(search)
                entries.append((suite, "search", search))
        if not entries:
            raise _fail("no claim calls a faulted operation")
        classified[(row["op"], row["fault"])] = entries
    # Phase 2: one pass per suite.
    for searches in per_suite.values():
        _search_suite(prog, spec.ops, searches)
    # Phase 3: the control rows.
    controls: list[dict] = []
    witness_chunks: dict[str, set[int]] = {}
    for row in rows:
        level = _gated(row, implemented) or row["requires"][0]
        required: list[str] = []
        blind: list[str] = []
        witness: list[dict] = []
        for suite, state, search in classified[(row["op"], row["fault"])]:
            if state == "search":
                state = "witnessed" if search.found else "blind"
            if state == "blind":
                blind.append(suite.id)
                continue
            required.append(suite.id)
            if state == "witnessed":
                chunk = search.witness // chunk_size
                witness.append({"claim": suite.id, "chunk": chunk})
                witness_chunks.setdefault(suite.claim, set()).add(chunk)
            elif state == "deferred":
                witness.append({"claim": suite.id, "deferred": level})
            else:
                witness.append({"claim": suite.id, "unproven": True})
        control_id = f"warrant/{spec.name}/negctl/exhaustive/{row['op']}/{row['fault']}"
        accepted = (row["op"], row["fault"]) in accept
        if not required:
            # Every candidate is proven blind: the corpus cannot see this fault, so it is an
            # error unless the profile's accept_blind line names it (section 16.8 [D15]).
            if not accepted:
                raise _fail("every candidate claim is blind to a fault; accept_blind excuses it")
            accepted_ids.append(control_id)
        elif accepted:
            raise OracleError(
                "corpus", "an accept_blind line names a fault that is not all-blind"
            )
        controls.append(
            {
                "id": control_id,
                "op": row["op"],
                "fault": row["fault"],
                "target_kind": "exhaustive",
                "requires": list(row["requires"]),
                "required_claims": required,
                "blind_claims": blind,
                "slim_witness": witness,
            }
        )
    return rows, controls, witness_chunks, sorted(accepted_ids, key=str.encode)

