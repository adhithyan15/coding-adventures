"""Check a committed ``warrant-corpus-v1`` corpus against the oracle's own reading.

This is the **ct-compare gate** of VER04r section 1.2 / slice S4: before any v2
corpus exists, the oracle proves it is an honest second reader by reproducing,
from ``ct-compare.warrant`` alone, everything the Rust generator committed in
``code/specs/fixtures/ct-compare-warrant-v1/corpus.json``:

1. **Lowering** (check 1).  The oracle's own unoptimised lowering of the
   operations, models, examples, suite ``admit``/``lhs``/``rhs`` equals the
   corpus IR, compared as canonical JSON values.  A generator that lowers
   ``a - b`` as ``b - a`` is caught here and nowhere else.
2. **Enumeration and evaluation** (check 2).  Case counts, discarded counts,
   ``expected_calls`` and both SHA-256 stream digests of every suite.
3. **Faults and controls** (check 3).  The fault registry and control rows
   re-derived from the operation signatures (the v1 ct-compare profile).

The corpus is *data being checked*, never truth: nothing here evaluates the
corpus IR.  The corpus is read with a strict JSON reader (duplicate keys
rejected), size-capped before parsing.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field

from .domains import _Listed
from .errors import OracleError
from .ir_eval import ModelDef, Program
from .lower import Spec, lower_document
from .parser import parse_source
from .suites import SuitePlan

V1_BUDGET = 400_000
MAX_CORPUS_BYTES = 16 << 20


@dataclass
class Report:
    """The outcome of a gate run: mismatches (empty means pass) and a summary."""

    mismatches: list = field(default_factory=list)
    suites: dict = field(default_factory=dict)
    notes: list = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.mismatches

    def miss(self, code: str, where: str, message: str) -> None:
        self.mismatches.append({"code": code, "where": where, "message": message})


def _pairs(pairs: list) -> dict:
    out: dict = {}
    for key, value in pairs:
        if key in out:
            raise OracleError("corpus", "duplicate object key in corpus")
        out[key] = value
    return out


def load_json(raw: bytes, limit: int = MAX_CORPUS_BYTES) -> object:
    """Strict JSON: size cap, UTF-8, duplicate keys and non-finite numbers refused."""
    if len(raw) > limit:
        raise OracleError("limit", "corpus exceeds its size cap")

    def reject(_: str) -> None:
        raise OracleError("corpus", "non-finite number in corpus")

    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=_pairs, parse_constant=reject)
    except OracleError:
        raise
    except (UnicodeDecodeError, ValueError, RecursionError) as exc:
        raise OracleError("corpus", "corpus is not valid JSON") from exc


def build_program(spec: Spec, **kwargs) -> Program:
    """A :class:`Program` over every model the oracle could lower (``ir.core`` only)."""
    models = {
        n: ModelDef(n, m.params, m.ret, m.ir)
        for n, m in spec.models.items()
        if m.unsupported is None
    }
    return Program(models, spec.ops, **kwargs)


# -- v1 spellings ------------------------------------------------------------------------


def v1_binder(desc) -> dict:
    """The v1 ``{"domain": ..., "type": ...}`` object of a binder descriptor."""
    if not isinstance(desc, _Listed):
        raise OracleError("generation", "the v1 profile has only leaf domains")
    name = desc.name
    if name == "bytes_n":
        return {"domain": f"bytes_n({desc.extra['n']})", "type": str(desc.ty)}
    return {"domain": name, "type": str(desc.ty)}


def typed_json(value: object, ty: str) -> object:
    """The v1 typed JSON of an example value (``corpus.rs`` ``typed_json``)."""
    if ty == "Bool":
        return value
    if ty.startswith("Bytes"):
        return {"type": ty, "value": value.hex()}
    if ty in ("U8", "U64", "Nat"):
        return {"type": ty, "value": str(value)}
    if ty.startswith("Result<"):
        if value[0] == "ok":
            return {"type": ty, "value": {"ok": typed_json(value[1], "Bytes")}}
        return {"type": ty, "value": {"err": value[1]}}
    raise OracleError("generation", "an example value outside the v1 profile")


def v1_faults(ops: dict) -> list[dict]:
    """The v1 fault registry by operation signature (VER04h section 6)."""
    rows = []
    for name, op in ops.items():
        types = [str(t) for _, t in op.params]
        ret = str(op.ret)
        if ret == "Bool" and types in (["Bytes", "Bytes"], ["Bytes<N>", "Bytes<N>"]):
            faults = [("ignore-last-byte", "input"), ("invert", "output")]
        elif ret == "Bool" and types == ["U64", "U64"]:
            faults = [("ignore-high-bit", "input"), ("invert", "output")]
        elif ret.startswith("Result<") and types == ["Bytes", "Bytes", "Bool"]:
            faults = [("missing-error", "output"), ("swap-choice", "input")]
        else:
            raise OracleError("generation", "an operation has no v1 fault profile")
        rows.extend({"applies": a, "fault": f, "op": name} for f, a in faults)
    return rows


def v1_controls(spec: Spec, faults: list[dict]) -> list[dict]:
    rows = []
    for f in faults:
        required = [s.id for s in spec.suites if s.ops == [f["op"]]]
        if not required:
            raise OracleError("generation", "a faulted operation has no claim target")
        rows.append(
            {
                "fault": f["fault"],
                "id": f"warrant/{spec.name}/negctl/exhaustive/{f['op']}/{f['fault']}",
                "op": f["op"],
                "required_claims": required,
                "target_kind": "exhaustive",
            }
        )
    # The generator lists controls sorted by (operation, fault), whereas the fault
    # registry is in declaration order; both orders are part of the corpus.
    rows.sort(key=lambda r: (r["op"], r["fault"]))
    return rows


def _canon(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def same(a: object, b: object) -> bool:
    """Type-strict equality by canonical JSON bytes.

    Python's ``==`` says ``True == 1 == 1.0``; a corpus comparison must not
    (``["lit", 1]`` is not ``["lit", true]``, ``"discarded": false`` is not ``0``,
    a float ``cases`` is not an integer).  Canonical JSON writes each of those
    differently, and fixes key order, so equal text means equal value *and* type.
    """
    try:
        return _canon(a) == _canon(b)
    except (TypeError, ValueError):
        return False


SUITE_KEYS = frozenset(
    [
        "id",
        "kind",
        "claim",
        "ops",
        "binders",
        "admit",
        "lhs",
        "rhs",
        "cases",
        "discarded",
        "expected_calls",
        "domain_text",
        "inputs_sha256",
        "expected_sha256",
        "result_type",
    ]
)


# -- the gate -------------------------------------------------------------------------------


def verify_v1(
    spec_bytes: bytes, corpus_raw: bytes, *, chunk_size: int = 1 << 16, digests: bool = True
) -> tuple[Report, list]:
    """Run the three oracle checks against a v1 corpus.

    ``digests=False`` skips check 2 (the enumeration) and runs only the cheap
    lowering and registry checks; it exists so a mutation test does not pay for
    572,109 cases to prove that a changed IR node is noticed.

    Returns ``(report, suite_results)``; ``suite_results`` are the
    :class:`SuiteResult` objects of the oracle's own full-depth enumeration (these
    carry the v2 chunk digests, ready to be emitted).
    """
    report = Report()
    corpus = load_json(corpus_raw)
    if not isinstance(corpus, dict):
        raise OracleError("corpus", "corpus root must be an object")
    keys = {
        "schema_version",
        "profile",
        "module",
        "spec",
        "spec_sha256",
        "generator",
        "types",
        "ops",
        "models",
        "examples",
        "suites",
        "faults",
        "controls",
    }
    if set(corpus) != keys:
        raise OracleError("corpus", "corpus has unexpected or missing top-level fields")
    version = corpus["schema_version"]
    if corpus["profile"] != "warrant-corpus-v1" or type(version) is not int or version != 1:
        raise OracleError("corpus", "not a warrant-corpus-v1 document")
    rows = corpus["suites"]
    if not isinstance(rows, list) or not all(isinstance(r, dict) for r in rows):
        raise OracleError("corpus", "suites must be a list of objects")
    doc = parse_source(spec_bytes)
    spec = lower_document(doc)
    if corpus["module"] != spec.name:
        report.miss("profile", "module", "corpus module differs from the spec")
    if corpus["spec_sha256"] != hashlib.sha256(spec_bytes).hexdigest():
        report.miss("spec-digest", "spec_sha256", "spec digest differs")
    # -- check 1: lowering ---------------------------------------------------------------
    if not same(corpus["ops"], {n: o.corpus_row() for n, o in spec.ops.items()}):
        report.miss("lowering", "ops", "operation signatures differ")
    if not same(corpus["types"], {n: list(c) for n, c in spec.errors.items()}):
        report.miss("lowering", "types", "error types differ")
    mine = {
        n: {"params": [p for p, _ in m.params], "body": m.ir}
        for n, m in spec.models.items()
        if m.unsupported is None
    }
    theirs = corpus["models"] if isinstance(corpus["models"], dict) else None
    if theirs is None:
        report.miss("lowering", "models", "models must be an object")
    else:
        unchecked = {n for n, m in spec.models.items() if m.unsupported is not None}
        for name in sorted(set(mine) | set(theirs) | unchecked):
            if name in spec.models and spec.models[name].unsupported is not None:
                # Never agreement: a model the oracle cannot lower is *unchecked*.
                report.miss(
                    "level",
                    f"models.{name}",
                    "model needs a level the oracle does not implement; not checked",
                )
            elif not same(mine.get(name), theirs.get(name)):
                report.miss(
                    "lowering", f"models.{name}", "model IR differs from the oracle's lowering"
                )
    prog = build_program(spec)
    _check_examples(spec, prog, corpus["examples"], report)
    # -- check 3: faults and controls (order is part of the corpus) ------------------------
    try:
        faults = v1_faults(spec.ops)
        if not same(corpus["faults"], faults):
            report.miss("faults", "faults", "fault registry differs (set or order)")
        if not same(corpus["controls"], v1_controls(spec, faults)):
            report.miss("controls", "controls", "control rows differ (set or order)")
    except OracleError as exc:
        report.miss("faults", "faults", exc.message)
    # -- check 1 and 2 per suite -------------------------------------------------------------
    results = []
    if [r.get("claim") for r in rows] != [s.claim for s in spec.suites]:
        report.miss("suite-set", "suites", "suite set or order differs from the claims of the spec")
        return report, results
    for suite, row in zip(spec.suites, rows):
        where = f"suites.{suite.claim}"
        if set(row) != SUITE_KEYS:
            report.miss("suite-keys", where, "suite row has unexpected or missing keys")
            continue
        try:
            plan = SuitePlan(spec, prog, suite, chunk_size=chunk_size, budget=V1_BUDGET)
        except OracleError as exc:
            if exc.code != "level":
                raise
            report.miss("level", where, "suite needs a level the oracle does not implement")
            continue
        want = {
            "id": suite.id,
            "kind": "exhaustive",
            "claim": suite.claim,
            "ops": suite.ops,
            "binders": [[b.name, v1_binder(b.desc)] for b in plan.binders],
            "admit": suite.admit,
            "lhs": suite.subject,
            "rhs": suite.expected,
            "result_type": str(suite.result),
            "domain_text": " x ".join(f"{b.ty}[{b.size}]" for b in plan.binders),
        }
        for key, value in want.items():
            if not same(row[key], value):
                report.miss("lowering", f"{where}.{key}", f"suite {key} differs from the oracle")
        if suite.kind != "reference":
            report.miss("lowering", where, "a v1 suite must be a reference claim")
            continue
        if not digests:
            continue
        result = plan.run("full")
        results.append(result)
        checks = {
            "cases": result.cases,
            "discarded": result.discarded,
            "expected_calls": result.expected_calls,
            "inputs_sha256": result.v1_inputs_sha256,
            "expected_sha256": result.v1_expected_sha256,
        }
        for key, value in checks.items():
            if not same(row[key], value):
                report.miss("enumeration", f"{where}.{key}", f"suite {key} differs from the oracle")
        report.suites[suite.claim] = {
            "cases": result.cases,
            "discarded": result.discarded,
            "chunks": result.chunks_total,
            "v1_inputs_sha256": result.v1_inputs_sha256,
            "v1_expected_sha256": result.v1_expected_sha256,
            "v2_inputs_sha256": result.inputs_sha256,
            "v2_expected_sha256": result.expected_sha256,
        }
    return report, results


def _check_examples(spec: Spec, prog: Program, corpus_examples: object, report: Report) -> None:
    mine = []
    for ex in spec.examples:
        if ex.unsupported is not None or ex.subject is None:
            report.miss("lowering", f"examples.{ex.name}", "an example outside the v1 profile")
            continue
        node = ex.subject
        op = spec.ops[node[1]]
        args = []
        for arg, (_, pty) in zip(node[2], op.params):
            value = prog.compile(arg, {}, allow_subject=False).fn({})
            args.append(typed_json(value, str(pty)))
        expected = prog.compile(ex.expected, {}, allow_subject=False).fn({})
        mine.append(
            {
                "id": f"warrant/{spec.name}/example/{ex.name}",
                "op": node[1],
                "args": args,
                "expected": typed_json(expected, str(op.ret)),
                "result_type": str(op.ret),
                "width": node[3][0] if len(node) == 4 else None,
            }
        )
    if not same(corpus_examples, mine):
        report.miss(
            "lowering", "examples", "examples differ from the oracle's evaluation of the source"
        )
