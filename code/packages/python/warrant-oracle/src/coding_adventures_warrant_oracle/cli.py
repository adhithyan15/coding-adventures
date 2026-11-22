"""Command line of the independent Warrant oracle.

::

    warrant-oracle verify-v1 --spec ct-compare.warrant --corpus corpus.json \
                             [--emit-v2 scratch.json]
    warrant-oracle emit-v2   --spec gf256.warrant --out scratch.json \
                             [--depth slim|full] [--suite NAME ...]
    warrant-oracle verify-v2 <corpus-dir> --warrant x.warrant --profile corpus.profile \
                             [--bindings evidence.bindings] [--expect-generator name@version]
    warrant-oracle check     --spec x.warrant
    warrant-oracle dump-lowering --spec x.warrant [--max-ir-cost N] (--out f.json | --check f.json)

Exit status: ``0`` the gate passed; ``1`` a mismatch (the corpus and the oracle
disagree); ``2`` the input could not be read or refused (a resource bound, a
malformed document); ``3`` (``emit-v2``) some suites are ``not-live`` or ``refused``
(the partial result is written; ``--allow-partial`` turns this into ``0``).  Failures print a suite id and a check name, never input bytes.
"""

from __future__ import annotations

import argparse
import os
import pathlib
import sys

from .corpus_v1 import verify_v1
from .emit import census, dumps, emit, suite_row
from .errors import OracleError
from .ir_eval import MAX_IR_COST
from .lower import lower_document
from .lowering_dump import dump
from .parser import parse_source
from .profile import parse_profile
from .suites import DEFAULT_BUDGET, DEFAULT_CHUNK_SIZE
from .verify_v2 import parse_expected_generator, verify_v2

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
    e.add_argument("--profile", help="a corpus.profile (the only way to raise max_ir_cost)")
    e.add_argument(
        "--allow-partial",
        action="store_true",
        help="exit 0 even if suites are not-live or refused (they are still listed)",
    )
    c = sub.add_parser("check", help="parse and lower a document; report levels")
    c.add_argument("--spec", required=True)
    d = sub.add_parser(
        "dump-lowering",
        help="write or check the canonical lowering dump of a document (ir.fixed parity, S10a)",
    )
    d.add_argument("--spec", required=True)
    d.add_argument(
        "--max-ir-cost",
        type=int,
        default=MAX_IR_COST,
        help="the corpus.profile bound the costs are computed under (default 2,000)",
    )
    where = d.add_mutually_exclusive_group(required=True)
    where.add_argument("--out", help="write the dump here")
    where.add_argument("--check", help="exit 1 unless this committed dump is what the oracle writes")
    w = sub.add_parser("verify-v2", help="gate an on-disk warrant-corpus-v2 fixture (S4 part 2)")
    w.add_argument("corpus_dir", help="a fixture directory: corpus.json, chunks/, cases.tfv")
    w.add_argument("--warrant", required=True, help="the .warrant source the corpus was made from")
    w.add_argument("--profile", required=True, help="the corpus.profile (budget, faults, ...)")
    w.add_argument("--bindings", help="an evidence.bindings whose faults= lists must match")
    w.add_argument(
        "--expect-generator",
        metavar="NAME@VERSION",
        help="the generator name and version corpus.json must record (a bump then fails the build)",
    )
    w.add_argument("--repo-root", help="the repository root the corpus's spec path is relative to")
    return ap


def _check_out(path: str, inputs: list[str]) -> None:
    """Refuse an output path that could clobber an input or a committed spec."""
    out = pathlib.Path(path)
    if out.is_symlink():
        raise OracleError("source", "the output path must not be a symbolic link")
    resolved = out.resolve()
    for other in inputs:
        if resolved == pathlib.Path(other).resolve():
            raise OracleError("source", "the output path is an input file")
        # A hard link has a different path but is the same file.
        if out.exists() and pathlib.Path(other).exists() and os.path.samefile(out, other):
            raise OracleError("source", "the output path is an input file")
    parts = resolved.parts
    if any(a == "code" and b == "specs" for a, b in zip(parts, parts[1:])):
        raise OracleError("source", "the output path lies inside the specification tree")


def run(argv: list[str]) -> int:
    args = build_parser().parse_args(argv)
    try:
        return _run(args)
    except RecursionError:
        print("warrant-oracle: limit: input nests too deeply", file=sys.stderr)
        return 2
    except OracleError as exc:
        print(f"warrant-oracle: {exc.code}: {exc.message}", file=sys.stderr)
        return 2


def _verify_v2(args: argparse.Namespace) -> int:
    report, _ = verify_v2(
        pathlib.Path(args.corpus_dir),
        pathlib.Path(args.warrant),
        pathlib.Path(args.profile),
        pathlib.Path(args.bindings) if args.bindings else None,
        repo_root=pathlib.Path(args.repo_root) if args.repo_root else None,
        expect_generator=(
            parse_expected_generator(args.expect_generator) if args.expect_generator is not None else None
        ),
    )
    for miss in report.mismatches:
        print(
            f"warrant-oracle: MISMATCH {miss['code']} {miss['where']}: {miss['message']}",
            file=sys.stderr,
        )
    cases = sum(s["cases"] for s in report.suites.values())
    status = "ok" if report.ok else "FAILED"
    omitted = next((n.split()[1] for n in report.notes if n.startswith("omitted_examples ")), None)
    extra = f", {omitted} omitted examples" if omitted else ""
    print(
        f"warrant-oracle: verify-v2: {status}: {cases} cases across {len(report.suites)} suites{extra}"
    )
    return 0 if report.ok else 1


def _dump_lowering(args: argparse.Namespace, spec_bytes: bytes) -> int:
    if not 2_000 <= args.max_ir_cost <= 65_536:
        raise OracleError("limit", "max_ir_cost is between 2,000 and 65,536")
    text = dump(spec_bytes, args.max_ir_cost)
    if args.check is not None:
        committed = _read(args.check, 16 << 20)
        if committed != text:
            print(f"warrant-oracle: dump-lowering: DIFFERS: {args.check}", file=sys.stderr)
            return 1
        print(f"warrant-oracle: dump-lowering: ok: {args.check} ({len(text)} bytes)")
        return 0
    _check_out(args.out, [args.spec])
    pathlib.Path(args.out).write_bytes(text)
    print(f"warrant-oracle: dump-lowering: wrote {args.out} ({len(text)} bytes)")
    return 0


def _run(args: argparse.Namespace) -> int:
    if args.command == "verify-v2":
        return _verify_v2(args)
    spec_bytes = _read(args.spec, MAX_SPEC_READ - 1)
    if args.command == "check":
        sys.stdout.write(dumps(census(lower_document(parse_source(spec_bytes)))))
        return 0
    if args.command == "dump-lowering":
        return _dump_lowering(args, spec_bytes)
    if args.command == "emit-v2":
        _check_out(args.out, [args.spec] + ([args.profile] if args.profile else []))
        profile = parse_profile(_read(args.profile, 64 * 1024)) if args.profile else None
        tree = emit(
            spec_bytes,
            depth=args.depth,
            only=set(args.suite) if args.suite else None,
            chunk_size=args.chunk_size,
            budget=args.budget,
            profile=profile,
        )
        _write(args.out, dumps(tree))
        print(f"warrant-oracle: emit-v2: {tree['module']}: {tree['summary']}")
        partial = tree["summary"]["not-live"] + tree["summary"]["refused"]
        if partial and not args.allow_partial:
            print(
                f"warrant-oracle: emit-v2: PARTIAL: {partial} suite(s) not-live or refused",
                file=sys.stderr,
            )
            return 3
        return 0
    if args.emit_v2:
        _check_out(args.emit_v2, [args.spec, args.corpus])
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
