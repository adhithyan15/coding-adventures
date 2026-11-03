"""Validate CT01's neutral oracle without calling a language implementation.

The fixture is a published contract, not a snapshot of any package's output.
This validator recomputes its functional answers and its *logical* public-work
counts directly from canonical input text. It makes no timing assertion.

It also compiles `cases.json` into `cases.tfv`, the TF01 line format that
harnesses read (a test harness may not depend on a JSON parser), and fails if
the committed `cases.tfv` is not exactly what `cases.json` generates:

    python3 code/scripts/ct_compare_conformance.py           # check (CI)
    python3 code/scripts/ct_compare_conformance.py --write   # regenerate cases.tfv
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from typing import Any

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import tfv  # a sibling script, found through the path set above

ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_CORPUS = ROOT / "code" / "specs" / "fixtures" / "ct-compare-v1" / "cases.json"
BYTE_HEX = re.compile(r"(?:[0-9a-f]{2})*\Z")
U64_HEX = re.compile(r"[0-9a-f]{16}\Z")
CASE_ID = re.compile(r"[a-z][a-z0-9-]*\Z")
ERROR_ID = "CT_SELECT_LENGTH_MISMATCH"


def _object_without_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """JSON permits some parsers to silently keep the last duplicate key.

    A conformance corpus must not admit two possible interpretations, so reject
    duplicates while decoding, before any semantic validation occurs.
    """

    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_json(text: str) -> Any:
    return json.loads(text, object_pairs_hook=_object_without_duplicate_keys)


def _fields(value: Any, required: set[str], where: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != required:
        raise ValueError(f"{where} fields must be exactly {sorted(required)}")
    return value


def _byte_hex(value: Any, where: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) > 8192
        or BYTE_HEX.fullmatch(value) is None
    ):
        raise ValueError(f"{where} must be canonical lowercase even-length hex")
    return value


def _u64_hex(value: Any, where: str) -> str:
    if not isinstance(value, str) or U64_HEX.fullmatch(value) is None:
        raise ValueError(f"{where} must be exactly 16 lowercase u64 hex digits")
    return value


def _public_work(
    positions: int = 0, *, select: bool = False, u64: bool = False
) -> dict[str, int]:
    """Count logical units only; these are not machine instructions or cycles."""

    return {
        "byte_positions": positions,
        "byte_reads": 2 * positions,
        "byte_writes": positions if select else 0,
        "u64_bits": 64 if u64 else 0,
    }


def _validate_case(case: Any) -> None:
    if not isinstance(case, dict):
        raise TypeError("case fields must be an object")
    operation = case.get("operation")
    common = {"id", "operation", "expected", "expected_work"}
    if operation == "eq":
        required = common | {"left_hex", "right_hex"}
    elif operation == "eq_fixed":
        required = common | {"fixed_size_bytes", "left_hex", "right_hex"}
    elif operation == "select_bytes":
        required = common | {"left_hex", "right_hex", "choice"}
    elif operation == "eq_u64":
        required = common | {"left_u64", "right_u64"}
    else:
        raise ValueError(f"unknown operation: {operation}")
    _fields(case, required, "case")
    case_id = case["id"]
    if (
        not isinstance(case_id, str)
        or len(case_id) > 80
        or CASE_ID.fullmatch(case_id) is None
    ):
        raise ValueError("case id must be canonical kebab-case")

    if operation == "eq_u64":
        left = _u64_hex(case["left_u64"], "left_u64")
        right = _u64_hex(case["right_u64"], "right_u64")
        expected = {"equal": int(left, 16) == int(right, 16)}
        work = _public_work(u64=True)
    else:
        left = _byte_hex(case["left_hex"], "left_hex")
        right = _byte_hex(case["right_hex"], "right_hex")
        left_bytes = bytes.fromhex(left)
        right_bytes = bytes.fromhex(right)
        same_length = len(left_bytes) == len(right_bytes)
        positions = len(left_bytes) if same_length else 0
        if operation == "eq_fixed":
            width = case["fixed_size_bytes"]
            if (
                type(width) is not int
                or not 0 <= width <= 4096
                or (len(left_bytes) != width or len(right_bytes) != width)
            ):
                raise ValueError("fixed_size_bytes must equal both input lengths")
        if operation == "select_bytes":
            if type(case["choice"]) is not bool:
                raise ValueError("choice must be boolean")
            expected = (
                {"output_hex": left if case["choice"] else right, "fresh_output": True}
                if same_length
                else {"error_id": ERROR_ID}
            )
            work = _public_work(positions, select=same_length)
        else:
            expected = {"equal": left_bytes == right_bytes}
            work = _public_work(positions)

    if (
        type(case["expected"]) is not dict
        or case["expected"] != expected
        or (
            any(
                type(value) is not type(expected[key])
                for key, value in case["expected"].items()
            )
        )
    ):
        raise ValueError(f"{case_id} expected result disagrees with independent oracle")
    observed_work = _fields(case["expected_work"], set(work), "expected_work")
    if (
        any(type(observed_work[key]) is not int for key in work)
        or observed_work != work
    ):
        raise ValueError(f"{case_id} expected_work disagrees with public input shape")


def validate_document(document: Any) -> int:
    """Return the validated case count or raise ValueError on any ambiguity."""

    root = _fields(document, {"schema_version", "profile", "cases"}, "corpus")
    if type(root["schema_version"]) is not int or root["schema_version"] != 1:
        raise ValueError("schema_version must be 1")
    if root["profile"] != "ct-compare-v1":
        raise ValueError("profile must be ct-compare-v1")
    cases = root["cases"]
    if not isinstance(cases, list) or not 1 <= len(cases) <= 256:
        raise ValueError("cases must contain 1..256 entries")
    seen: set[str] = set()
    for case in cases:
        _validate_case(case)
        if case["id"] in seen:
            raise ValueError(f"duplicate case id: {case['id']}")
        seen.add(case["id"])
    return len(cases)


# ---------------------------------------------------------------------------
# cases.json -> cases.tfv
# ---------------------------------------------------------------------------
# One TFV record per case, keyed by the case id. The JSON transport's hex
# strings become TFV bytes (or, for eq_u64, integers), and the nested
# `expected` and `expected_work` objects are flattened so an adapter reads
# every field by name without a map lookup:
#
#   field            kind    present for
#   ---------------  ------  -------------------------------------------
#   op               text    every case: eq, eq_fixed, select_bytes, eq_u64
#   left, right      bytes   byte operations
#   left, right      int     eq_u64 (0 .. 2^64-1)
#   width            int     eq_fixed
#   choice           bool    select_bytes
#   equal            bool    eq, eq_fixed, eq_u64
#   output           bytes   select_bytes that succeeds
#   fresh_output     bool    select_bytes that succeeds (always true)
#   error_id         text    select_bytes that must fail
#   work_positions, work_reads, work_writes, work_u64_bits
#                    int     every case: CT01's logical public-work counts
def to_records(document: Any) -> list[tfv.Record]:
    records: list[tfv.Record] = []
    for case in document["cases"]:
        operation = case["operation"]
        fields: list[tuple[str, tfv.Value]] = [("op", ("text", operation))]
        if operation == "eq_u64":
            fields += [
                ("left", ("int", int(case["left_u64"], 16))),
                ("right", ("int", int(case["right_u64"], 16))),
            ]
        else:
            fields += [
                ("left", ("bytes", bytes.fromhex(case["left_hex"]))),
                ("right", ("bytes", bytes.fromhex(case["right_hex"]))),
            ]
        if operation == "eq_fixed":
            fields.append(("width", ("int", case["fixed_size_bytes"])))
        if operation == "select_bytes":
            fields.append(("choice", ("bool", case["choice"])))
        expected = case["expected"]
        if "equal" in expected:
            fields.append(("equal", ("bool", expected["equal"])))
        elif "error_id" in expected:
            fields.append(("error_id", ("text", expected["error_id"])))
        else:
            fields += [
                ("output", ("bytes", bytes.fromhex(expected["output_hex"]))),
                ("fresh_output", ("bool", expected["fresh_output"])),
            ]
        work = case["expected_work"]
        fields += [
            ("work_positions", ("int", work["byte_positions"])),
            ("work_reads", ("int", work["byte_reads"])),
            ("work_writes", ("int", work["byte_writes"])),
            ("work_u64_bits", ("int", work["u64_bits"])),
        ]
        records.append((case["id"], fields))
    return records


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--corpus", type=pathlib.Path, default=DEFAULT_CORPUS)
    parser.add_argument(
        "--write", action="store_true", help="regenerate cases.tfv beside the corpus"
    )
    args = parser.parse_args(argv)
    document = parse_json(args.corpus.read_text(encoding="utf-8"))
    count = validate_document(document)
    # Validation first: only a corpus the oracle accepts may be compiled.
    records = to_records(document)
    generated = tfv.encode_file(records)
    target = args.corpus.with_name("cases.tfv")
    if args.write:
        target.write_text(generated, encoding="utf-8")
    elif not target.exists() or target.read_text(encoding="utf-8") != generated:
        print("ct-compare-v1: cases.tfv is stale: run with --write", file=sys.stderr)
        return 1
    elif tfv.parse_file(generated) != records:
        print("ct-compare-v1: cases.tfv does not round-trip", file=sys.stderr)
        return 1
    print(f"ct-compare-v1: {count} cases valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
