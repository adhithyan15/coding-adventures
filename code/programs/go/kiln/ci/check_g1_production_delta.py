#!/usr/bin/env python3
"""Validate that the held-out G1 B/C pair differs by only matcher.go."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path


TARGET = "code/packages/go/lexer/matcher.go"
BASELINE_BLOB = "e2ecb4d52a32122f2056a632f7a4e645860f9667"
CANDIDATE_BLOB = "72ca3a12145dcc74b84ccb3baf757bd6d32c4bcb"
ROOTS = ("code/grammars", "code/packages/go", "code/programs/go/kiln")


class CheckError(Exception):
    """A failed check with a stable, user-readable category."""


def _run_git(root: Path, *args: str) -> bytes:
    try:
        result = subprocess.run(
            ["git", "-C", str(root), *args],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as exc:
        raise CheckError(f"required tool error: could not run git: {exc}") from exc
    if result.returncode:
        detail = result.stderr.decode("utf-8", "replace").strip()
        raise CheckError(f"source enumeration error: git exited {result.returncode}: {detail}")
    return result.stdout


def production_blobs(root: Path, ref: str = "HEAD") -> dict[str, str]:
    raw = _run_git(root, "ls-tree", "-r", "-z", ref, "--", *ROOTS)
    blobs: dict[str, str] = {}
    for record in raw.split(b"\0"):
        if not record:
            continue
        metadata, path_bytes = record.split(b"\t", 1)
        mode, kind, object_id = metadata.decode("ascii").split()
        path = path_bytes.decode("utf-8", "surrogateescape")
        if (
            kind == "blob"
            and path.endswith(".go")
            and not path.endswith("_test.go")
            and "/ci/" not in path
        ):
            blobs[path] = object_id
    return blobs


def validate_delta(baseline: dict[str, str], candidate: dict[str, str]) -> None:
    changed = {
        path
        for path in baseline.keys() | candidate.keys()
        if baseline.get(path) != candidate.get(path)
    }
    if not changed:
        raise CheckError("candidate source delta missing: no compiled production Go file changed")
    if changed != {TARGET}:
        raise CheckError(
            "unexpected compiled production Go delta: " + ", ".join(sorted(changed))
        )
    if TARGET not in baseline or TARGET not in candidate:
        raise CheckError(f"candidate source delta missing: {TARGET} absent from one checkout")
    if baseline[TARGET] != BASELINE_BLOB or candidate[TARGET] != CANDIDATE_BLOB:
        raise CheckError(
            "unexpected candidate source delta: lexer blobs do not match the reviewed pair"
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--baseline-ref", default="HEAD")
    parser.add_argument("--candidate-ref", default="HEAD")
    args = parser.parse_args()
    if shutil.which("git") is None:
        print("required tool error: git is not installed", file=sys.stderr)
        return 2
    try:
        baseline = production_blobs(args.baseline, args.baseline_ref)
        candidate = production_blobs(args.candidate, args.candidate_ref)
        validate_delta(baseline, candidate)
    except CheckError as exc:
        print(str(exc), file=sys.stderr)
        return 2
    print(f"compiled production Go delta: exactly {TARGET}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
