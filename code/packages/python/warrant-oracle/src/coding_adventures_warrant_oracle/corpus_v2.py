"""Derive the whole expected ``warrant-corpus-v2`` fixture from a ``.warrant`` file (VER04r 10.3).

``derive`` is the oracle's *independent writer* of ``corpus.json``, ``chunks/*.txt`` and
``cases.tfv``.  It reads the specification source and ``corpus.profile`` and nothing
else: no generator code, no generator output, no data table shared with it.  The
verifier (:mod:`.verify_v2`) then compares what it derived with the bytes on disk.

The three checks of section 10.3, and where each lives
-------------------------------------------------------
1. **Lowering.**  ``types``, ``ops``, ``models``, ``model_order``, ``examples`` and each
   suite's ``admit``, ``body``, ``oracle`` and ``result_type`` come from
   :mod:`.lower` (unoptimised, syntax-directed) -- never from the corpus.
2. **Enumeration and evaluation.**  Per suite: ``cases``, ``discarded``, ``expected_calls``,
   ``wraps``, ``ir_cost``, every chunk line (``first_admitted``, ``calls``, the two chunk
   digests), the suite digests, the v1 whole-suite digests and the slim digests, from
   :mod:`.suites` and :mod:`.stream`.
3. **Faults and controls.**  ``faults`` and ``controls`` (``required_claims``,
   ``blind_claims``, ``slim_witness``) from :mod:`.faults`; the witness chunks feed
   ``slim_chunks``, so a wrong witness shows up in check 2's slim digests too.

Closed key set (section 1.4, provisional)
-----------------------------------------
Every key of the section is written, an empty object or list where there is nothing to
say; the *conditional* keys (``expected_sha256``, ``v1_*``, ``slim_expected_sha256``,
an example's ``width``, an op's ``fresh``/``width_params``/``admit``) are omitted, never
``null``.  The generator is not derivable (``generator`` is a name and version the oracle
cannot know) so its shape alone is checked.
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass, field

from . import ast as A
from . import tfv
from .corpus_v1 import build_program
from .errors import OracleError, SizeError
from .faults import IMPLEMENTED, derive_controls
from .ir_eval import WrapsInAdmit, admit_fn, run_admit
from .lower import FIXED, Spec, _has_subject, _substitute, lower_document
from .parser import parse_source
from .profile import Profile, parse_profile
from .stream import slim_base_set
from .suites import SuitePlan, Work

PROFILE = "warrant-corpus-v2"
MAX_SLIM_CHUNKS = 32
_CLAIM = re.compile(r"[A-Za-z_][A-Za-z0-9_]{0,63}$")
_RESERVED = frozenset(
    ["con", "prn", "aux", "nul"]
    + [f"com{i}" for i in range(1, 10)]
    + [f"lpt{i}" for i in range(1, 10)]
)


@dataclass
class Derived:
    """Everything the oracle expects on disk."""

    corpus: dict
    chunk_files: dict = field(default_factory=dict)  # file name -> bytes
    tfv: bytes = b""
    results: dict = field(default_factory=dict)  # claim -> SuiteResult


def canonical(obj: object) -> bytes:
    """The VER04i section 9.1 canonical writer: sorted keys, two-space indent, LF, final LF, ASCII."""
    return (json.dumps(obj, sort_keys=True, indent=2, ensure_ascii=True) + "\n").encode("ascii")


def chunk_file_name(claim: str) -> str:
    """``chunks/<file>.txt``: the claim with every ``/`` replaced by ``.`` (section 5.5)."""
    parts = claim.split("/")
    if len(parts) > 2 or any(not _CLAIM.match(p) for p in parts):
        raise OracleError("generation", "W0701: a claim name is not a legal file name")
    name = claim.replace("/", ".")
    if any(part.lower() in _RESERVED for part in name.split(".")):
        raise OracleError("generation", "W0701: a claim file name is a Windows device name")
    return name + ".txt"


def _require_core(spec: Spec) -> None:
    """Refuse the whole document if anything is above ``ir.core`` (fail-closed, section 13).

    The oracle computes a digest for every suite, so it cannot verify a corpus it cannot
    evaluate; the level is *named*, never skipped.
    """
    for model in spec.models.values():
        if model.unsupported is not None:
            raise model.unsupported
    for op in spec.ops.values():
        if op.unsupported is not None:
            raise op.unsupported
    for suite in spec.suites:
        if suite.unsupported is not None:
            raise suite.unsupported
        if suite.refusal is not None:
            raise suite.refusal
    for ex in spec.examples:
        if ex.unsupported is not None:
            raise ex.unsupported
    if spec.ensures:
        raise OracleError("level", "ensures rows need the inout and ensures slice", level="ensures")
    for decl in spec.doc.decls:
        if isinstance(decl, A.OpDecl) and any(p.inout for p in decl.params):
            raise OracleError("level", "inout parameters are not verified yet", level="inout")
        if isinstance(decl, A.AssumeDecl) and decl.rows:
            raise OracleError(
                "generation", "an assume row has no corpus shape yet (VER04r 16.4 item 4 is OPEN)"
            )


# -- JSON values, decoded by declared type (section 2.3) ------------------------------------


def value_json(v: object, ty: A.Ty) -> object:
    """An untagged JSON value of declared type ``ty`` (``ir.core`` and ``ir.tuple`` types).

    A tuple is a JSON array in component order (section 2.3); a ``Result`` over a tuple is
    ``{"ok": [..]}``.
    """
    head = ty.head
    if head == "Bool":
        return bool(v)
    if head in FIXED or head == "Nat":
        return str(v)
    if head == "Bytes":
        return v.hex()
    if head == "()":
        return [value_json(x, t) for x, t in zip(v, ty.args)]
    if head == "Result":
        if v[0] == "ok":
            return {"ok": value_json(v[1], ty.args[0])}
        return {"err": v[1]}
    raise OracleError("level", "a value of this type needs a later level", level="ir.data")


MAX_EXAMPLE_BYTES = 4096  # the section 2.3 limit on a ``Bytes`` value in ``corpus.json``
REASONS = ("not-op-eq-pure", "value-over-limit")  # the closed vocabulary of [D34]


class _NotOpEqPure(Exception):
    """An example whose shape is not ``op(args) == pure`` (internal: decides a reason)."""


def example_id(spec: Spec, ex) -> str:
    return f"warrant/{spec.name}/example/{ex.name}"


def _over_limit(v: object, ty: A.Ty) -> bool:
    """Is a ``Bytes`` inside ``v`` (itself, under an ``ok`` or in a tuple) over the section 2.3 limit?"""
    if ty.head == "Bytes":
        return len(v) > MAX_EXAMPLE_BYTES
    if ty.head == "()":
        return any(_over_limit(x, t) for x, t in zip(v, ty.args))
    if ty.head == "Result" and v[0] == "ok":
        return _over_limit(v[1], ty.args[0])
    return False


def _evaluate_example(spec: Spec, prog, ex):
    """One example, in the normative order of section 8.4: shape, arguments, pure side, checks.

    Returns ``(op, statics, ptypes, ret, args, expected)``.  The order is the contract of the
    ``value-over-limit`` decision, so the steps are numbered the way the spec numbers them:

    1. the arguments are evaluated left to right, then the pure side (an error ends the run,
       so the *first* error is the one seen);
    2. any error that is not a size error is raised as it is (a refusal);
    3. a :class:`SizeError` is raised as it is too: the caller decides whether it qualifies;
    4. only if everything evaluated do the [D18] (v0 wrap) and [D19] (outside ``admit``)
       checks run, and each refuses.
    """
    node = ex.subject
    # The generator's rule (``lower_example``, W0701): exactly one subject call, its arguments
    # free of subject calls, on one side of an equality, the other side pure.  ``ex.subject`` is
    # set only for an ``==`` with a subject on exactly one side; the rest is tested here.
    if node is None or node[0] != "subject_op" or any(_has_subject(a) for a in node[2]):
        raise _NotOpEqPure
    op = spec.ops[node[1]]
    statics = dict(zip(op.statics, node[3])) if len(node) == 4 else {}
    if len(statics) > 1:
        raise OracleError("generation", "an example with several static widths has no shape")
    stats = prog.stats
    before = stats.wraps
    try:
        stats.counting = True
        args = [prog.compile(a, {}, False).fn({}) for a in node[2]]
        expected = prog.compile(ex.expected, {}, False).fn({})
    finally:
        stats.counting = False
    wrapped = stats.wraps > before
    # [D19]: an example is a call, so its arguments must be inside the op's ``admit``.
    admit = admit_fn(prog, op, node[3] if len(node) == 4 else None)
    try:
        inside = admit is None or run_admit(
            prog, admit, {n: v for (n, _), v in zip(op.params, args)}, spec.version == "v0", op.name
        )
    except WrapsInAdmit:
        raise OracleError(
            "generation",
            f"W0701: example `{ex.name}`: U8 arithmetic wraps while evaluating the admit of "
            f"op `{op.name}`; v0 harness evaluates U8 as Nat (VER04r 3.3 rule 2)",
        ) from None
    if not inside:
        raise OracleError(
            "generation",
            f"W0701: example `{ex.name}` calls op `{op.name}` with arguments outside "
            "the op's admit",
        )
    if spec.version == "v0" and wrapped:
        # [D18], section 3.3 rule 2: an example's evaluation wraps in a v0 document.
        raise OracleError(
            "generation",
            f"W0701: example `{ex.name}`: U8 arithmetic wraps; v0 harness evaluates U8 as Nat",
        )
    ptypes = [(n, _static(t, statics)) for n, t in op.params]
    return op, statics, ptypes, _static(op.ret, statics), args, expected


def omit_reason(spec: Spec, prog, ex) -> str:
    """The computed reason an ``omit_example`` line may excuse ``ex`` (section 8.4, [D34]).

    ``not-op-eq-pure`` is tested first, from the shape alone.  Otherwise the example is
    evaluated as for emission; the first error decides: a size error qualifies as
    ``value-over-limit``, any other error refuses.  A completed evaluation that passed the
    [D18]/[D19] checks qualifies only when a final argument or the pure value is over
    4,096 bytes.  An example that qualifies for nothing is a stale profile (``corpus``).
    The messages name the id and never a value.
    """
    eid = example_id(spec, ex)
    try:
        _, _, ptypes, ret, args, expected = _evaluate_example(spec, prog, ex)
    except _NotOpEqPure:
        return "not-op-eq-pure"
    except SizeError:
        return "value-over-limit"
    if any(_over_limit(a, t) for a, (_, t) in zip(args, ptypes)) or _over_limit(expected, ret):
        return "value-over-limit"
    raise OracleError(
        "corpus", f"an omit_example line names an example that is emittable: `{eid}`"
    )


def _examples(spec: Spec, prog, omit: list[str] = ()) -> tuple[list[dict], bytes, list[dict]]:
    """``(examples rows, cases.tfv, omitted_examples rows)``; omitted examples are not evaluated
    beyond the reason decision, not encoded and not admit-checked (section 16.10 R4)."""
    known = {example_id(spec, ex) for ex in spec.examples}
    for eid in omit:
        if eid not in known:
            raise OracleError("corpus", "an omit_example line names no example of the spec")
    rows: list[dict] = []
    lines: list[str] = []
    omitted: list[dict] = []
    for ex in spec.examples:
        eid = example_id(spec, ex)
        if eid in omit:
            omitted.append({"id": eid, "reason": omit_reason(spec, prog, ex)})
            continue
        try:
            op, statics, ptypes, ret, args, expected = _evaluate_example(spec, prog, ex)
        except _NotOpEqPure:
            raise OracleError(
                "generation", "W0701: an example is one subject call compared with a pure value"
            ) from None
        width = next(iter(statics.values())) if statics else None
        row = {
            "id": eid,
            "op": op.name,
            "args": [value_json(a, t) for a, (_, t) in zip(args, ptypes)],
            "expected": value_json(expected, ret),
            "result_type": str(op.ret),
        }
        if width is not None:
            row["width"] = width
        rows.append(row)
        lines.append(tfv.record(eid, op.name, ptypes, args, width, expected, ret))
    return rows, tfv.document(lines), sorted(omitted, key=lambda r: r["id"].encode())


def _static(ty: A.Ty, statics: dict) -> A.Ty:
    return _substitute(ty, statics)


def _types(spec: Spec) -> dict:
    out: dict = {}
    for name, cases in spec.errors.items():
        out[name] = {"kind": "error", "cases": list(cases)}
    for name, fields in spec.records.items():
        out[name] = {"kind": "record", "fields": [[n, str(t)] for n, t in fields]}
    for name, ctors in spec.variants.items():
        out[name] = {
            "kind": "variant",
            "ctors": [[c, [str(t) for _, t in fs]] for c, fs in ctors],
        }
    return out


def _slim_set(plan: SuitePlan, prof: Profile, witnesses: set[int]) -> list[int]:
    """Base set (stride clamped) + witness chunks + ``slim_chunk`` lines, ascending, <= 32."""
    m = plan.chunks_total
    chosen = set(slim_base_set(plan.suite.id, m, prof.slim_stride)) | witnesses
    for claim, index in prof.slim_chunks:
        if claim == plan.suite.claim:
            if index >= m:
                raise OracleError("generation", "a slim_chunk line names a chunk past the suite")
            chosen.add(index)
    if len(chosen) > MAX_SLIM_CHUNKS:
        raise OracleError("generation", "more than 32 slim chunks in one suite")
    return sorted(chosen)


def derive(
    spec_bytes: bytes,
    profile_raw: bytes,
    *,
    spec_ref: str,
    generator: dict,
    work: Work | None = None,
    implemented: tuple = IMPLEMENTED,
) -> Derived:
    """Derive the expected fixture (see the module docstring); raises :class:`OracleError`.

    ``implemented`` is the fault-evaluation gate (:data:`.faults.IMPLEMENTED`, ``ir.core`` only until
    the atomic flip of slice S10c, VER04r 13 and [D29]): the levels whose faults the witness search
    evaluates.  It is a library parameter that tests (and the committed fault dump of S10t-1) pass;
    no command line or environment switch reaches it.
    """
    spec = lower_document(parse_source(spec_bytes))
    prof = parse_profile(profile_raw)
    _require_core(spec)
    prog = build_program(spec, max_cost=prof.max_ir_cost)
    work = work if work is not None else Work("full")
    names = {s.claim for s in spec.suites}
    for claim, _ in prof.slim_chunks:
        if claim not in names:
            raise OracleError("generation", "a slim_chunk line names an undeclared claim")
    files: dict = {}
    for suite in spec.suites:
        name = chunk_file_name(suite.claim)
        if name.lower() in {f.lower() for f in files}:
            raise OracleError("generation", "W0701: two claims map to one file name")
        files[name] = None
    plans = {
        s.claim: SuitePlan(
            spec, prog, s, chunk_size=prof.chunk_size, budget=prof.budget, work=work
        )
        for s in spec.suites
    }
    fault_rows, controls, witnesses, accepted_blind = derive_controls(
        spec, prog, plans, prof.faults, prof.chunk_size, prof.accept_blind, implemented
    )
    out = Derived(corpus={})
    suite_rows = []
    for suite in spec.suites:
        plan = plans[suite.claim]
        slim = _slim_set(plan, prof, witnesses.get(suite.claim, set()))
        res = plan.run("full", slim_chunks=slim)
        out.results[suite.claim] = res
        text = "".join(c.line() + "\n" for c in res.chunks).encode("ascii")
        out.chunk_files[chunk_file_name(suite.claim)] = text
        suite_rows.append(_suite_row(suite, plan, res, text))
    examples, out.tfv, omitted = _examples(spec, prog, prof.omit_examples)
    ops = {n: o.corpus_row_v2() for n, o in spec.ops.items()}
    models = {
        n: {"params": [[p, str(t)] for p, t in m.params], "body": m.ir}
        for n, m in spec.models.items()
    }
    levels = {"ir.core"}
    for row in [*suite_rows, *ops.values(), *fault_rows, *controls]:
        levels.update(row["requires"])
    out.corpus = {
        "profile": PROFILE,
        "schema_version": 2,
        "module": spec.name,
        "document": spec.version,
        "spec": spec_ref,
        "spec_sha256": hashlib.sha256(spec_bytes).hexdigest(),
        "profile_sha256": hashlib.sha256(profile_raw).hexdigest(),
        "generator": generator,
        "budget": prof.budget,
        "chunk_size": prof.chunk_size,
        "max_ir_cost": prof.max_ir_cost,
        "levels": sorted(levels),
        "types": _types(spec),
        "ops": ops,
        "models": models,
        "model_order": list(spec.models),
        "fault_models": {},
        "suites": suite_rows,
        "examples": examples,
        "faults": fault_rows,
        "controls": controls,
        "accepted_blind_faults": accepted_blind,
        "assumes": [],
    }
    if omitted:
        # A conditional key (sections 1.4, 16.10 R4): written only when an example is omitted,
        # so no emitted byte of an existing fixture moves.
        out.corpus["omitted_examples"] = omitted
    return out


def _suite_row(suite, plan: SuitePlan, res, chunk_text: bytes) -> dict:
    ref = suite.kind == "reference"
    row = {
        "id": suite.id,
        "kind": "exhaustive",
        "claim": suite.claim,
        "ops": list(suite.ops),
        # Computed (section 16.10 R9): the levels of every tag of ``admit`` and ``body`` and of
        # their model closure, plus ``ir.tuple`` for a tuple type.  ``["ir.core"]`` for every
        # document the oracle emits today.
        "requires": list(suite.requires),
        "binders": [
            [b.name, {"type": str(b.ty), "domain": b.desc.to_json()}] for b in plan.binders
        ],
        "demoted": res.demoted,
        "domain_label": res.label,
        "admit": suite.admit,
        "body": suite.body,
        "oracle": (
            {"kind": "reference", "subject": suite.subject, "expected": suite.expected}
            if ref
            else {"kind": "predicate"}
        ),
        "result_type": res.result_type,
        "stream": res.header,
        "product": res.product,
        "cases": res.cases,
        "discarded": res.discarded,
        "expected_calls": res.expected_calls,
        "wraps": res.wraps,
        "ir_cost": res.ir_cost,
        "chunks": res.chunks_total,
        "chunks_sha256": hashlib.sha256(chunk_text).hexdigest(),
        "inputs_sha256": res.inputs_sha256,
        "slim_chunks": res.slim_chunks,
        "slim_inputs_sha256": res.slim_inputs_sha256,
    }
    if ref:
        row["expected_sha256"] = res.expected_sha256
        row["slim_expected_sha256"] = res.slim_expected_sha256
    if res.v1_inputs_sha256 is not None:
        row["v1_inputs_sha256"] = res.v1_inputs_sha256
        if ref:
            row["v1_expected_sha256"] = res.v1_expected_sha256
    return row

