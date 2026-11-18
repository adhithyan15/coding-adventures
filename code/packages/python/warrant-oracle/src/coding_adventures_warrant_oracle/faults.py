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
exactly as ``ir.fixed``'s is: :data:`IMPLEMENTED` holds ``ir.core`` only, so a control whose fault
needs ``ir.tuple`` (or ``ir.fixed``) records ``{"claim": C, "deferred": "ir.tuple"}`` witnesses and no
witness search runs.  The gate is reachable only through the ``implemented`` parameter of
:func:`derive_controls`, which the tests use; no command line or environment switch exists.
"""

from __future__ import annotations

from dataclasses import dataclass

from . import levels as L
from .errors import OracleError
from .ir_eval import Program
from .lower import BOOL, FIXED, NAT, OpSig, Spec, _substitute
from .suites import SuitePlan

CORE = "ir.core"
FIXED_LEVEL = "ir.fixed"
TUPLE_LEVEL = "ir.tuple"
#: The levels whose *faults* this oracle evaluates (the witness search).  A fault that needs any
#: other level is instantiated and compared as opaque data (section 10.4) and its witnesses are
#: ``deferred``.  ``ir.tuple`` is implemented (lowering, typing, cost, evaluation, S10t-1) but its
#: fault evaluation is **gated** like ``ir.fixed``'s until the atomic flip (S10c, D29): the flip
#: adds ``ir.fixed`` and ``ir.tuple`` here.  Tests reach the gate through ``derive_controls(...,
#: implemented=...)``, never through a switch.
IMPLEMENTED = (CORE,)
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


def _missing_error(op: OpSig, errors) -> tuple:
    """``if is_err(result) then Ok(<payload>) else result``.

    A ``Bytes`` payload is the first ``Bytes`` argument (section 8.2).  A **tuple** payload (R3,
    ``ir.tuple``) is the tuple of ``ci``: for the ``i``-th component, the first parameter, in
    parameter order, of exactly that type if one exists, else that type's zero.
    """
    if op.ret.head == "Result" and op.ret.args[0].head == "()":
        parts = []
        for component in op.ret.args[0].args:
            param = next((n for n, t in op.params if t == component), None)
            parts.append(_var(param) if param is not None else _zero(component))
        expr = ["if", ["is_err", _var("result")], ["ok", ["tuple", parts]], _var("result")]
        return "replace", [TUPLE_LEVEL], expr
    if op.ret.head != "Result" or op.ret.args[0].head != "Bytes" or not _bytes_params(op):
        raise _fail("missing-error needs a Result<Bytes, E> or Result<tuple, E> result and a Bytes parameter")
    first = _bytes_params(op)[0]
    expr = ["if", ["is_err", _var("result")], ["ok", _var(first)], _var("result")]
    return "replace", [CORE], expr


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
}


def fault_row(
    op: OpSig, fault: str, errors: dict | None = None, data_names: frozenset = frozenset()
) -> dict:
    """The ``faults`` row for ``fault`` on ``op`` (section 8.1; keys per section 16.6 item 3).

    ``requires`` is the sorted union of the template's levels and the levels of the op's own types
    (R9): a fault on an operation whose parameter or result type is or contains a tuple needs
    ``ir.tuple`` even where the template names no tuple tag.  ``errors`` (error type -> cases) is
    needed by ``ok-as-err``; ``data_names`` are the record and variant names, for ``ir.data``.
    """
    builder = REGISTRY.get(fault)
    if builder is None:
        raise _fail("an unknown fault name")
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


def _compile_fault(prog: Program, op: OpSig, fault: dict, statics: dict):
    """Type-check and compile the fault expressions at the stand-in's static widths."""
    tenv = {p: _substitute(t, statics) for p, t in op.params}
    if fault["kind"] == "pre":
        return [(r["param"], prog.compile(r["expr"], tenv, False).fn) for r in fault["rewrites"]]
    tenv["result"] = _substitute(op.ret, statics)
    return prog.compile(fault["expr"], tenv, False).fn


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
    evaluates).  The shipped value is :data:`IMPLEMENTED` (``ir.core``): a fault whose ``requires``
    names any other level is emitted as data and each of its candidate claims that has a stand-in is
    ``{"claim": C, "deferred": L}`` with ``L`` the first level of the fault's ``requires`` that is not
    implemented (a fault needing two unimplemented levels names the first in sorted order).  The
    atomic flip (S10c) turns the gate on; until then only tests pass another value.
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
        rows.append(fault_row(spec.ops[op_name], fault, spec.errors, data_names))
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

