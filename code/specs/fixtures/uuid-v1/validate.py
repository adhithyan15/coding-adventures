"""Validate the neutral uuid fixture against the independent oracle.

Checks:
1. ``cases.json`` parses and carries the expected schema/oracle tags.
2. Regeneration via ``generate.build_fixture`` is identical to the checked-in
   file (deterministic provenance: no hand edits).
3. Every expectation re-derives from ``oracle`` (no copied outputs).
4. All invalid inputs are rejected by the oracle.

Exit 0 on success, 1 with a diagnostic on the first failure.
"""

from __future__ import annotations

import json
import sys

from generate import ROOT, build_fixture
from oracle import (
    NAMESPACES,
    UuidOracleError,
    format_uuid,
    parse,
    v3,
    v5,
    variant_of,
    version_of,
)


def fail(message: str) -> int:
    print(f"uuid-v1 fixture INVALID: {message}")
    return 1


def main() -> int:
    try:
        checked_in = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        return fail(f"cannot read cases.json: {exc}")

    if checked_in.get("schema") != "uuid-v1":
        return fail(f"bad schema tag: {checked_in.get('schema')!r}")
    if checked_in.get("oracle") != "uuid-oracle-py-v1":
        return fail(f"bad oracle tag: {checked_in.get('oracle')!r}")

    regenerated = build_fixture()
    if regenerated != checked_in:
        return fail("cases.json differs from generate.py output (hand edit?)")

    for name, raw in NAMESPACES.items():
        row = checked_in["namespaces"].get(name)
        if row is None or row.get("hex") != raw.hex():
            return fail(f"namespace {name} mismatch")
        if row.get("string") != format_uuid(raw):
            return fail(f"namespace {name} string mismatch")

    for row in checked_in["known_answers"]:
        ns = NAMESPACES[row["namespace"]]
        if row["op"] == "v3":
            if format_uuid(v3(ns, row["name"])) != row["expected"]:
                return fail(f"v3 known-answer {row['id']} mismatch")
        elif row["op"] == "v5":
            if format_uuid(v5(ns, row["name"])) != row["expected"]:
                return fail(f"v5 known-answer {row['id']} mismatch")
        else:
            return fail(f"known-answer {row['id']} has unknown op {row['op']!r}")
    for row in checked_in["v3_cases"]:
        ns = NAMESPACES[row["namespace"]]
        if format_uuid(v3(ns, row["name"])) != row["expected"]:
            return fail(f"v3 case {row['id']} mismatch")
    for row in checked_in["v5_cases"]:
        ns = NAMESPACES[row["namespace"]]
        if format_uuid(v5(ns, row["name"])) != row["expected"]:
            return fail(f"v5 case {row['id']} mismatch")

    for row in checked_in["parse_cases"]:
        raw = parse(row["input"])
        if raw.hex() != row["hex"]:
            return fail(f"parse case {row['id']} bytes mismatch")
        if format_uuid(raw) != row["string"]:
            return fail(f"parse case {row['id']} string mismatch")
        if version_of(raw) != row["version"]:
            return fail(f"parse case {row['id']} version mismatch")
        if variant_of(raw) != row["variant"]:
            return fail(f"parse case {row['id']} variant mismatch")

    for row in checked_in["byte_patterns"]:
        raw = bytes.fromhex(row["hex"])
        if len(raw) != 16:
            return fail(f"byte pattern {row['id']} is not 16 bytes")
        if format_uuid(raw) != row["string"]:
            return fail(f"byte pattern {row['id']} string mismatch")
        if version_of(raw) != row["version"]:
            return fail(f"byte pattern {row['id']} version mismatch")
        if variant_of(raw) != row["variant"]:
            return fail(f"byte pattern {row['id']} variant mismatch")

    for text in checked_in["invalid_inputs"]:
        try:
            parse(text)
        except UuidOracleError:
            continue
        return fail(f"invalid input accepted by oracle: {text!r}")

    counts = {k: len(v) for k, v in checked_in.items() if isinstance(v, list)}
    print(f"uuid-v1 fixture valid: {counts}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
