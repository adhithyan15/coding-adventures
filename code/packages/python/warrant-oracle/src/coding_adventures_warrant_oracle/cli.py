"""Command line of the independent Warrant oracle.

::

    warrant-oracle verify-v1 --spec ct-compare.warrant --corpus corpus.json \
                             [--emit-v2 scratch.json]
    warrant-oracle emit-v2   --spec gf256.warrant --out scratch.json \
                             [--depth slim|full] [--suite NAME ...]
    warrant-oracle check     --spec x.warrant

Exit status: ``0`` the gate passed; ``1`` a mismatch (the corpus and the oracle
disagree); ``2`` the input could not be read or refused (a resource bound, a
malformed document).  Failures print a suite id and a check name, never input bytes.
"""

from __future__ import annotations

import argparse
import pathlib
import sys

from .corpus_v1 import verify_v1
from .emit import census, dumps, emit, suite_row
from .errors import OracleError
from .ir_eval import MAX_IR_COST
from .lower import lower_document
from .parser import parse_source
from .suites import DEFAULT_BUDGET, DEFAULT_CHUNK_SIZE

MAX_SPEC_READ = (1 << 20) + 1


def _read(path: str, limit: int) -> bytes:
    p = pathlib.Path(path)
    try:
        if p.is_symlink() or not p.is_file():
            raise OracleError("source", "input must be a regular file")
        with p.open("rb") as handle:
            data = handle.read(limit + 1)
    except OSError as exc:
        raise OracleError("source", "could not read an input file") from exc
    if len(data) > limit:
        raise OracleError("limit", "an input file exceeds its size cap")
    return data


def _write(path: str, text: str) -> None:
    pathlib.Path(path).write_text(text, encoding="utf-8", newline="\n")


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(prog="warrant-oracle", description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="command", required=True)
    v = sub.add_parser("verify-v1", help="gate a committed v1 corpus (ct-compare)")
    v.add_argument("--spec", required=True)
    v.add_argument("--corpus", required=True)
    v.add_argument("--emit-v2", metavar="OUT", help="also write the oracle's v2 chunk digests")
    e = sub.add_parser("emit-v2", help="write the oracle's own v2 enumeration digests")
    e.add_argument("--spec", required=True)
    e.add_argument("--out", required=True)
    e.add_argument("--depth", choices=("slim", "full"), default="slim")
    e.add_argument("--suite", action="append", default=None)
    e.add_argument("--chunk-size", type=int, default=DEFAULT_CHUNK_SIZE)
    e.add_argument("--budget", type=int, default=DEFAULT_BUDGET)
    e.add_argument(
        "--max-ir-cost",
        type=int,
        default=MAX_IR_COST,
        help="diagnostic override of MAX_IR_COST (spec value 2000)",
    )
    e.add_argument("--cost-model", choices=("all-branches", "taken-path"), default="all-branches")
    c = sub.add_parser("check", help="parse and lower a document; report levels")
    c.add_argument("--spec", required=True)
    return ap


def run(argv: list[str]) -> int:
    args = build_parser().parse_args(argv)
    try:
        spec_bytes = _read(args.spec, MAX_SPEC_READ - 1)
        if args.command == "check":
            sys.stdout.write(dumps(census(lower_document(parse_source(spec_bytes)))))
            return 0
        if args.command == "emit-v2":
            tree = emit(
                spec_bytes,
                depth=args.depth,
                only=set(args.suite) if args.suite else None,
                chunk_size=args.chunk_size,
                budget=args.budget,
                max_ir_cost=args.max_ir_cost,
                cost_model=args.cost_model,
            )
            _write(args.out, dumps(tree))
            print(f"warrant-oracle: emit-v2: {tree['module']}: {tree['summary']}")
            return 0
        corpus_raw = _read(args.corpus, 16 << 20)
        report, results = verify_v1(spec_bytes, corpus_raw)
        if args.emit_v2:
            tree = {
                "schema": "warrant-oracle-enumeration-v0",
                "depth": "full",
                "chunk_size": DEFAULT_CHUNK_SIZE,
                "suites": {r.claim: suite_row(r) for r in results},
            }
            _write(args.emit_v2, dumps(tree))
    except OracleError as exc:
        print(f"warrant-oracle: {exc.code}: {exc.message}", file=sys.stderr)
        return 2
    for miss in report.mismatches:
        print(
            f"warrant-oracle: MISMATCH {miss['code']} {miss['where']}: {miss['message']}",
            file=sys.stderr,
        )
    cases = sum(s["cases"] for s in report.suites.values())
    status = "ok" if report.ok else "FAILED"
    print(f"warrant-oracle: verify-v1: {status}: {cases} cases across {len(report.suites)} suites")
    return 0 if report.ok else 1


def main() -> None:
    raise SystemExit(run(sys.argv[1:]))
