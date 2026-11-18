"""Emit the oracle's own v2 enumeration digests for a ``.warrant`` document.

Until the v2 generator exists (slice S3) there is no corpus to compare with, so the
oracle writes what *it* computes -- per suite, per chunk -- to a scratch file
that S2/S3 output can later be diffed against.  Each suite is recorded as one of:

``ok``          enumerated; digests present;
``not-live``    needs an IR level this oracle does not implement (the level is
                named: ``ir.data``, ``ir.seq``);
``refused``     a resource bound or generation rule refused it (the code is named,
                e.g. ``limit: ir-cost`` -- see the README on gf256).

Nothing is ever skipped silently (VER04r section 4.1: "never skipped silently").
"""

from __future__ import annotations

import hashlib
import json

from .corpus_v1 import build_program
from .errors import OracleError
from .lower import Spec, lower_document
from .parser import parse_source
from .profile import Profile
from .suites import DEFAULT_BUDGET, DEFAULT_CHUNK_SIZE, SuitePlan, SuiteResult, Work


def suite_row(result: SuiteResult) -> dict:
    """The JSON row for one enumerated suite (chunk lines in the VER04r 5.5 format)."""
    return {
        "status": "ok",
        "claim": result.claim,
        "id": result.suite_id,
        "kind": result.kind,
        "oracle": "reference" if result.kind == "reference" else "relational",
        "binders": result.binder_types,
        "descriptors": result.descriptors,
        "demoted": result.demoted,
        "domain_label": result.label,
        "stream_header": result.header,
        "result_type": result.result_type,
        "product": result.product,
        "chunk_size": result.chunk_size,
        "chunks_total": result.chunks_total,
        "cases": result.cases,
        "discarded": result.discarded,
        "expected_calls": result.expected_calls,
        "wraps": result.wraps,
        "ir_cost": result.ir_cost,
        "slim_chunks": result.slim_chunks,
        "slim_inputs_sha256": result.slim_inputs_sha256,
        "slim_expected_sha256": result.slim_expected_sha256,
        "inputs_sha256": result.inputs_sha256,
        "expected_sha256": result.expected_sha256,
        "v1_inputs_sha256": result.v1_inputs_sha256,
        "v1_expected_sha256": result.v1_expected_sha256,
        "chunks_run": [c.index for c in result.chunks],
        "chunk_lines": [c.line() for c in result.chunks],
    }


def emit(
    spec_bytes: bytes,
    *,
    depth: str = "slim",
    only: set[str] | None = None,
    chunk_size: int = DEFAULT_CHUNK_SIZE,
    budget: int = DEFAULT_BUDGET,
    profile: Profile | None = None,
) -> dict:
    """Parse, lower and enumerate every suite of a document; return the JSON tree.

    ``profile`` (a parsed ``corpus.profile``) is the only way to change the IR cost
    bound: ``max_ir_cost`` defaults to 2,000 and is at most 65,536 (VER04r 4.2).  Its
    ``slim_stride`` is applied (clamped per suite); witness chunks and ``slim_chunk``
    lines are *not*: ``emit-v2`` is the diagnostic of the base set, and ``verify-v2``
    computes the full slim set (section 16.7).
    """
    if profile is not None:
        chunk_size, budget = profile.chunk_size, profile.budget
    max_ir_cost = profile.max_ir_cost if profile is not None else 2_000
    spec = lower_document(parse_source(spec_bytes))
    prog = build_program(spec, max_cost=max_ir_cost)
    work = Work(depth)
    rows: dict = {}
    for suite in spec.suites:
        if only is not None and suite.claim not in only:
            continue
        try:
            plan = SuitePlan(spec, prog, suite, chunk_size=chunk_size, budget=budget, work=work)
            stride = profile.slim_stride if profile is not None else "auto"
            rows[suite.claim] = suite_row(plan.run(depth, stride=stride))
        except OracleError as exc:
            if exc.code == "level":
                rows[suite.claim] = {
                    "status": "not-live",
                    "level": exc.level,
                    "reason": exc.message,
                }
            else:
                rows[suite.claim] = {"status": "refused", "code": exc.code, "reason": exc.message}
    return {
        "schema": "warrant-oracle-enumeration-v0",
        "module": spec.name,
        "document": spec.version,
        "spec_sha256": hashlib.sha256(spec_bytes).hexdigest(),
        "depth": depth,
        "chunk_size": chunk_size,
        "budget": budget,
        "max_ir_cost": max_ir_cost,
        "levels_implemented": ["ir.core", "ir.fixed", "ir.tuple"],
        "suites": rows,
        "summary": _summary(rows),
    }


def _summary(rows: dict) -> dict:
    out = {"ok": 0, "not-live": 0, "refused": 0}
    levels: dict = {}
    for row in rows.values():
        out[row["status"]] += 1
        if row["status"] == "not-live":
            levels[row["level"]] = levels.get(row["level"], 0) + 1
    out["not_live_by_level"] = levels
    return out


def census(spec: Spec) -> dict:
    """What each IR level would be needed for: a parse-only report of a document."""
    levels: dict = {}
    for suite in spec.suites:
        key = suite.unsupported.level if suite.unsupported is not None else "ir.core"
        levels[key] = levels.get(key, 0) + 1
    return {
        "module": spec.name,
        "document": spec.version,
        "ops": len(spec.ops),
        "models": len(spec.models),
        "models_not_ir_core": sum(1 for m in spec.models.values() if m.unsupported),
        "suites": len(spec.suites),
        "suites_by_level": levels,
        "examples": len(spec.examples),
    }


def dumps(tree: object) -> str:
    return json.dumps(tree, sort_keys=True, indent=1) + "\n"
