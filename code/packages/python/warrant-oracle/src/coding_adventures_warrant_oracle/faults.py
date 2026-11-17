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
"""

from __future__ import annotations

from dataclasses import dataclass

from .errors import OracleError
from .ir_eval import Program
from .lower import BOOL, FIXED, NAT, OpSig, Spec, _substitute
from .suites import SuitePlan

CORE = "ir.core"
FIXED_LEVEL = "ir.fixed"
#: The levels this oracle can evaluate.  A fault of any other level is instantiated and
#: compared as opaque data (section 10.4) and its witnesses are ``deferred``.
IMPLEMENTED = (CORE,)
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
# Each builder returns ``(kind, level, payload)``: ``payload`` is ``expr`` for post and
# replace, ``rewrites`` for pre.  A template the signature cannot take is an error and
# nothing is skipped.


def _invert(op: OpSig) -> tuple:
    if op.ret != BOOL:
        raise _fail("invert needs a Bool result")
    return "post", CORE, ["not", _var("result")]


def _missing_error(op: OpSig) -> tuple:
    if op.ret.head != "Result" or op.ret.args[0].head != "Bytes" or not _bytes_params(op):
        raise _fail("missing-error needs a Result<Bytes, E> result and a Bytes parameter")
    first = _bytes_params(op)[0]
    expr = ["if", ["is_err", _var("result")], ["ok", _var(first)], _var("result")]
    return "replace", CORE, expr


def _swap_choice(op: OpSig) -> tuple:
    if ("choice", BOOL) not in op.params:
        raise _fail("swap-choice needs a Bool parameter named choice")
    return "pre", CORE, [{"param": "choice", "expr": ["not", _var("choice")]}]


def _off_by_one(op: OpSig) -> tuple:
    if op.ret != NAT and op.ret.head not in FIXED:
        raise _fail("off-by-one needs a Nat or fixed-width result")
    return "post", CORE, ["add", _var("result"), _lit(str(op.ret), 1)]


def _truncate_last(op: OpSig) -> tuple:
    if op.ret.head != "Bytes":
        raise _fail("truncate-last needs a Bytes result")
    r = _var("result")
    length = ["len", r]
    expr = ["if", ["gt", length, _lit("Nat", 0)], ["take", r, ["sub", length, _lit("Nat", 1)]], r]
    return "post", CORE, expr


def _ignore_last_byte(op: OpSig) -> tuple:
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
    return "pre", CORE, rewrites


def _xor_lsb(op: OpSig) -> tuple:
    if op.ret.head not in FIXED:
        raise _fail("xor-lsb needs a fixed-width result")
    return "post", FIXED_LEVEL, ["bit_xor", _var("result"), _lit(str(op.ret), 1)]


def _ignore_high_bit(op: OpSig) -> tuple:
    names = [n for n, t in op.params if t.head == "U64"]
    if not names:
        raise _fail("ignore-high-bit needs a U64 parameter")
    return (
        "pre",
        FIXED_LEVEL,
        [
            {"param": n, "expr": ["bit_and", _var(n), _lit("U64", U64_LOW_63)]}
            for n in names
        ],
    )


REGISTRY = {
    "invert": _invert,
    "missing-error": _missing_error,
    "swap-choice": _swap_choice,
    "off-by-one": _off_by_one,
    "truncate-last": _truncate_last,
    "ignore-last-byte": _ignore_last_byte,
    "xor-lsb": _xor_lsb,
    "ignore-high-bit": _ignore_high_bit,
}


def fault_row(op: OpSig, fault: str) -> dict:
    """The ``faults`` row for ``fault`` on ``op`` (section 8.1; keys per section 16.6 item 3)."""
    builder = REGISTRY.get(fault)
    if builder is None:
        raise _fail("an unknown fault name")
    kind, level, payload = builder(op)
    row = {"fault": fault, "op": op.name, "kind": kind, "requires": [level]}
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


def derive_controls(
    spec: Spec,
    prog: Program,
    plans: dict[str, SuitePlan],
    faults: list[tuple[str, str]],
    chunk_size: int,
) -> tuple[list[dict], list[dict], dict[str, set[int]]]:
    """``(fault rows, control rows, witness chunks per claim)`` for the profile's ``fault`` lines.

    Rows are sorted by ``(op, fault)`` by bytes, independent of the order of the
    profile lines (section 16.6 item 2); the three claim lists keep suite order.
    """
    for name, op in spec.ops.items():
        if any(p == "result" for p, _ in op.params):
            raise OracleError("generation", f"W0701: operation {name} has a parameter named result")
    wanted = sorted(
        {(op, fault) for op, fault in faults}, key=lambda r: (r[0].encode(), r[1].encode())
    )
    rows: list[dict] = []
    for op_name, fault in wanted:
        if op_name not in spec.ops:
            raise _fail("a profile fault names an undeclared operation")
        rows.append(fault_row(spec.ops[op_name], fault))
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
            elif row["requires"][0] not in IMPLEMENTED:
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
        level = row["requires"][0]
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
        if not required:
            # Every candidate is proven blind: the corpus cannot see this fault, and
            # accepted_blind_faults is always [] for now (section 16.6 OPEN 1).
            raise _fail("every candidate claim is blind to a fault")
        controls.append(
            {
                "id": f"warrant/{spec.name}/negctl/exhaustive/{row['op']}/{row['fault']}",
                "op": row["op"],
                "fault": row["fault"],
                "target_kind": "exhaustive",
                "requires": [level],
                "required_claims": required,
                "blind_claims": blind,
                "slim_witness": witness,
            }
        )
    return rows, controls, witness_chunks

