#!/usr/bin/env python3
"""Validate the bootstrap-harness self-corpus and generate its TFV file.

`code/specs/fixtures/test-bootstrap-v1/cases.json` pins the behaviour every
language's bootstrap harness (TF01) must reproduce. This script is the
*independent* reference: standard-library Python, written apart from any lane.
It re-derives every expected value, so a lane that passes the corpus agrees
with two separate implementations, not just with itself.

    python3 code/scripts/bootstrap_harness_conformance.py           # check (CI)
    python3 code/scripts/bootstrap_harness_conformance.py --write   # regenerate cases.tfv

What `--check` enforces (TF01 section 4):

* a closed shape for every case, and unique ids;
* for `deep_equal` and `error_is`: at least one passing and one failing case,
  and every failure id exercised, because a harness that never fails would
  pass a corpus that only expects passes;
* every expected verdict, hash, output and order re-derived here;
* SplitMix64 agreeing with the published reference (seed 0 -> e220a8397b1dcdaf);
* every `tfv_reject` text really rejected by the strict parser;
* `cases.tfv` exactly what `cases.json` generates.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tfv

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "code/specs/fixtures/test-bootstrap-v1"
MASK = (1 << 64) - 1

SHAPES = {
    "deep_equal": {"actual", "expected", "verdict"},
    "error_is": {"raised", "expected", "verdict"},
    "splitmix64": {"seed", "outputs"},
    "fnv1a64": {"input", "hash"},
    "shuffle": {"seed", "ids", "order"},
    "sub_seed": {"seed", "test_id", "sub_seed"},
    "tfv_reject": {"text"},
}
FAILURE_IDS = {
    "deep_equal": {"not-deep-equal"},
    "error_is": {"no-error", "wrong-error"},
}


# --- reference primitives (TF01 section 5.1) -------------------------------
class SplitMix64:
    def __init__(self, seed: int) -> None:
        self.state = seed & MASK

    def next(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
        return z ^ (z >> 31)

    def bounded(self, n: int) -> int:
        threshold = ((1 << 64) - n) % n
        while True:
            x = self.next()
            if x >= threshold:
                return x % n


def fnv1a64(data: bytes) -> int:
    h = 0xCBF29CE484222325
    for byte in data:
        h = ((h ^ byte) * 0x100000001B3) & MASK
    return h


def shuffled(seed: int, ids: list[str]) -> list[str]:
    order = sorted(ids, key=lambda s: s.encode("utf-8"))
    rng = SplitMix64(seed)
    for i in range(len(order) - 1, 0, -1):
        j = rng.bounded(i + 1)
        order[i], order[j] = order[j], order[i]
    return order


def sub_seed(run_seed: int, test_id: str) -> int:
    return SplitMix64(run_seed ^ fnv1a64(test_id.encode("utf-8"))).next()


# --- reference verdicts (TF01 section 3) ----------------------------------
def deep_equal_verdict(
    actual: tfv.Value, expected: tfv.Value
) -> tuple[str, str | None]:
    return (
        ("pass", None)
        if tfv.values_equal(actual, expected)
        else ("fail", "not-deep-equal")
    )


def error_is_verdict(raised: tfv.Value, expected: tfv.Value) -> tuple[str, str | None]:
    if raised[0] == "null":
        return "fail", "no-error"
    return ("pass", None) if raised == expected else ("fail", "wrong-error")


# --- corpus handling --------------------------------------------------------
def load_cases() -> list[dict]:
    raw = (FIXTURE / "cases.json").read_text(encoding="utf-8")
    document = json.loads(raw, object_pairs_hook=_no_duplicate_keys)
    if set(document) != {"schema_version", "cases"} or document["schema_version"] != 1:
        raise SystemExit("cases.json must be {schema_version: 1, cases: [...]}")
    return document["cases"]


def _no_duplicate_keys(pairs: list[tuple[str, object]]) -> dict:
    keys = [key for key, _ in pairs]
    if len(keys) != len(set(keys)):
        raise SystemExit(f"duplicate JSON key in {keys}")
    return dict(pairs)


def to_records(cases: list[dict]) -> list[tfv.Record]:
    records = []
    for case in cases:
        fields = [("op", ("text", case["op"]))]
        fields += [
            (key, tfv.from_json(value))
            for key, value in case.items()
            if key not in ("id", "op")
        ]
        records.append((case["id"], fields))
    return records


def check(cases: list[dict]) -> list[str]:
    problems: list[str] = []
    ids = [case.get("id") for case in cases]
    if len(ids) != len(set(ids)):
        problems.append("duplicate case id")
    seen_verdicts: dict[str, set[str]] = {"deep_equal": set(), "error_is": set()}
    seen_failures: dict[str, set[str]] = {"deep_equal": set(), "error_is": set()}
    reference_vector_present = False
    for case in cases:
        cid, op = case.get("id"), case.get("op")
        if op not in SHAPES:
            problems.append(f"{cid}: unknown op {op!r}")
            continue
        shape = SHAPES[op] | {"id", "op"}
        if case.get("verdict") == "fail":
            shape = shape | {"failure_id"}
        if set(case) != shape:
            problems.append(f"{cid}: fields {sorted(case)} != {sorted(shape)}")
            continue
        value = {k: tfv.from_json(v) for k, v in case.items() if k not in ("id", "op")}
        if op in ("deep_equal", "error_is"):
            evaluate = deep_equal_verdict if op == "deep_equal" else error_is_verdict
            verdict, failure = evaluate(
                value["actual" if op == "deep_equal" else "raised"], value["expected"]
            )
            if (verdict, failure) != (case["verdict"], case.get("failure_id")):
                problems.append(f"{cid}: reference says {verdict}/{failure}")
            seen_verdicts[op].add(case["verdict"])
            if failure:
                seen_failures[op].add(failure)
        elif op == "splitmix64":
            rng = SplitMix64(value["seed"][1])
            got = [("int", rng.next()) for _ in value["outputs"][1]]
            if got != value["outputs"][1]:
                problems.append(f"{cid}: SplitMix64 outputs disagree")
            if value["seed"][1] == 0 and value["outputs"][1][:1] == [
                ("int", 0xE220A8397B1DCDAF)
            ]:
                reference_vector_present = True
        elif op == "fnv1a64":
            if fnv1a64(value["input"][1]) != value["hash"][1]:
                problems.append(f"{cid}: FNV-1a disagrees")
        elif op == "shuffle":
            ids_in = [v[1] for v in value["ids"][1]]
            want = [v[1] for v in value["order"][1]]
            if shuffled(value["seed"][1], ids_in) != want:
                problems.append(f"{cid}: shuffle order disagrees")
        elif op == "sub_seed":
            if sub_seed(value["seed"][1], value["test_id"][1]) != value["sub_seed"][1]:
                problems.append(f"{cid}: sub-seed disagrees")
        elif op == "tfv_reject":
            try:
                tfv.parse_file(value["text"][1])
            except tfv.TfvError:
                pass
            else:
                problems.append(f"{cid}: the strict parser accepted this text")
    for op in ("deep_equal", "error_is"):
        if seen_verdicts[op] != {"pass", "fail"}:
            problems.append(f"{op}: needs at least one pass and one fail case")
        missing = FAILURE_IDS[op] - seen_failures[op]
        if missing:
            problems.append(f"{op}: failure ids never exercised: {sorted(missing)}")
    if not reference_vector_present:
        problems.append("splitmix64: the published seed-0 reference vector is missing")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--write", action="store_true", help="regenerate cases.tfv")
    args = parser.parse_args(argv)
    cases = load_cases()
    problems = check(cases)
    generated = tfv.encode_file(to_records(cases))
    target = FIXTURE / "cases.tfv"
    if args.write:
        target.write_text(generated, encoding="utf-8")
    elif not target.exists() or target.read_text(encoding="utf-8") != generated:
        problems.append("cases.tfv is stale: run with --write")
    elif tfv.parse_file(generated) != to_records(cases):
        problems.append("cases.tfv does not round-trip")
    for problem in problems:
        print(f"test-bootstrap-v1: {problem}", file=sys.stderr)
    print(f"test-bootstrap-v1: {len(cases)} cases, {len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
