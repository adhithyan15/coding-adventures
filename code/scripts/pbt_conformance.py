#!/usr/bin/env python3
"""Validate the PBT (Property-Based Testing) conformance corpus and generate cases.tfv.

`code/specs/fixtures/pbt-v1/cases.json` pins the behaviour every language's
property testing engine (VER05) must reproduce. This script is the independent
reference: standard-library Python, written apart from any lane. It re-derives
every expected value, so a lane that passes the corpus agrees with an independent
oracle, not just with itself.

    python code/scripts/pbt_conformance.py           # check (CI)
    python code/scripts/pbt_conformance.py --write   # regenerate cases.tfv

What `--check` enforces (VER05 section 6):
* closed schema for every case, and unique case IDs;
* primitive generators agree with SplitMix64 deterministic reference outputs;
* bounded generators produce unbiased uniform draws within [min, max];
* integrated shrinking minimizes counterexamples to exact canonical targets;
* type refinements and bounds are strictly preserved during shrinking;
* replay reproducibility: exact reproduction of failing draw and minimal witness;
* filter exhaustion properly reported with closed failure_id 'property-exhausted';
* failure ID validation rejects unregistered IDs;
* cases.tfv matches cases.json compilation byte-for-byte.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Callable

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tfv

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "code/specs/fixtures/pbt-v1"
MASK = (1 << 64) - 1

SHAPES = {
    "gen_primitive": {"type", "seed", "count", "outputs"},
    "gen_bounded": {"min", "max", "seed", "count", "outputs"},
    "gen_bytes": {"min_len", "max_len", "seed", "count", "outputs"},
    "shrink_int": {
        "initial",
        "target",
        "min",
        "max",
        "property",
        "expected_minimal",
        "expected_steps",
    },
    "shrink_refinement": {
        "initial",
        "target",
        "min",
        "max",
        "property",
        "expected_minimal",
        "expected_steps",
    },
    "shrink_list": {
        "initial",
        "elem_target",
        "property",
        "expected_minimal",
        "expected_steps",
    },
    "shrink_bytes": {
        "initial",
        "property",
        "expected_minimal",
        "expected_steps",
    },
    "replay_consistency": {
        "seed",
        "property",
        "initial_fail",
        "expected_minimal",
    },
    "filter_exhausted": {
        "max_retries",
        "verdict",
        "failure_id",
    },
    "failure_id_valid": {"input", "valid"},
    "corpus_reject": {"text"},
}

REGISTERED_FAILURE_IDS = {
    "property-counterexample",
    "property-exhausted",
    "property-timeout",
    "property-panic",
}


# --- PRNG Reference (TF01 §5.1 / VER05 §2) ---------------------------------
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
        assert 1 <= n <= (1 << 32), f"bounded(n) requires 1 <= n <= 2^32, got {n}"
        threshold = ((1 << 64) - n) % n
        while True:
            x = self.next()
            if x >= threshold:
                return x % n

    def bounded_u64(self, n: int) -> int:
        assert 1 <= n <= (1 << 64), f"bounded_u64(n) requires 1 <= n <= 2^64, got {n}"
        if n <= (1 << 32):
            return self.bounded(n)
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


def sub_seed(run_seed: int, property_id: str) -> int:
    return SplitMix64(run_seed ^ fnv1a64(property_id.encode("utf-8"))).next()


# --- Generators Reference (VER05 §3) ---------------------------------------
def gen_primitive(seed: int, type_name: str, count: int) -> list[tfv.Value]:
    rng = SplitMix64(seed)
    outputs: list[tfv.Value] = []
    for _ in range(count):
        if type_name == "bool":
            outputs.append(("bool", rng.bounded(2) == 1))
        elif type_name == "u8":
            outputs.append(("int", rng.bounded(256)))
        elif type_name == "u64":
            outputs.append(("int", rng.next()))
        elif type_name == "i128":
            w1 = rng.next()
            w2 = rng.next()
            val = (w1 << 64) | w2
            if val >= (1 << 127):
                val -= 1 << 128
            outputs.append(("int", val))
        else:
            raise ValueError(f"unknown primitive type: {type_name}")
    return outputs


def gen_bounded(seed: int, min_val: int, max_val: int, count: int) -> list[tfv.Value]:
    assert min_val <= max_val
    rng = SplitMix64(seed)
    span = max_val - min_val + 1
    outputs: list[tfv.Value] = []
    for _ in range(count):
        if span <= (1 << 32):
            offset = rng.bounded(span)
        else:
            offset = rng.bounded_u64(span)
        outputs.append(("int", min_val + offset))
    return outputs


def gen_bytes(seed: int, min_len: int, max_len: int, count: int) -> list[tfv.Value]:
    assert 0 <= min_len <= max_len
    rng = SplitMix64(seed)
    span = max_len - min_len + 1
    outputs: list[tfv.Value] = []
    for _ in range(count):
        length = min_len + rng.bounded(span)
        data = bytes(rng.bounded(256) for _ in range(length))
        outputs.append(("bytes", data))
    return outputs


# --- Property Predicates for Shrinking (VER05 §4) ---------------------------
PROPERTIES: dict[str, Callable[[Any], bool]] = {
    "int_le_100": lambda x: x <= 100,
    "int_in_range_neg10_50": lambda x: -10 <= x <= 50,
    "int_is_even": lambda x: x % 2 == 0,
    "int_abs_le_10": lambda x: abs(x) <= 10,
    "list_no_consecutive_duplicates": lambda lst: all(
        lst[i] != lst[i + 1] for i in range(len(lst) - 1)
    ),
    "list_sum_le_20": lambda lst: sum(lst) <= 20,
    "bytes_len_lt_5": lambda b: len(b) < 5,
    "bytes_no_zero_byte": lambda b: 0 not in b,
    "refinement_x_le_50": lambda x: x <= 50,
}


# --- Integrated Shrinking Reference (VER05 §4) -----------------------------
def shrink_int(
    initial: int,
    target: int,
    min_val: int,
    max_val: int,
    prop_name: str,
    max_steps: int = 1000,
) -> tuple[int, int]:
    prop = PROPERTIES[prop_name]
    assert not prop(initial), (
        f"initial value {initial} must falsify property {prop_name}"
    )
    current = initial
    steps = 0

    if min_val <= target <= max_val and target != current and not prop(target):
        return target, 1

    while steps < max_steps:
        if current == target:
            break
        diff = current - target
        progress = False

        if abs(diff) > 1:
            step = diff // 2
            candidate = current - step
            if (
                min_val <= candidate <= max_val
                and not prop(candidate)
                and candidate != current
            ):
                current = candidate
                steps += 1
                progress = True

        if not progress:
            step = 1 if diff > 0 else -1
            candidate = current - step
            if (
                min_val <= candidate <= max_val
                and not prop(candidate)
                and candidate != current
            ):
                current = candidate
                steps += 1
                progress = True

        if not progress:
            break

    return current, steps


def shrink_list(
    initial: list[int],
    elem_target: int,
    prop_name: str,
    max_steps: int = 1000,
) -> tuple[list[int], int]:
    prop = PROPERTIES[prop_name]
    assert not prop(initial), f"initial list must falsify property {prop_name}"
    current = list(initial)
    steps = 0

    if not prop([]) and len(current) > 0:
        return [], 1

    while len(current) > 1 and steps < max_steps:
        mid = len(current) // 2
        left = current[:mid]
        if not prop(left):
            current = left
            steps += 1
            continue
        right = current[mid:]
        if not prop(right):
            current = right
            steps += 1
            continue
        break

    i = len(current) - 1
    while i >= 0 and len(current) > 1 and steps < max_steps:
        cand = current[:i] + current[i + 1 :]
        if not prop(cand):
            current = cand
            steps += 1
            i = min(i, len(current) - 1)
        else:
            i -= 1

    for i in range(len(current)):
        while steps < max_steps:
            if current[i] == elem_target:
                break
            diff = current[i] - elem_target
            progress = False
            if abs(diff) > 1:
                step = diff // 2
                cand_val = current[i] - step
                cand_list = current[:i] + [cand_val] + current[i + 1 :]
                if not prop(cand_list):
                    current = cand_list
                    steps += 1
                    progress = True
            if not progress:
                step = 1 if diff > 0 else -1
                cand_val = current[i] - step
                cand_list = current[:i] + [cand_val] + current[i + 1 :]
                if not prop(cand_list):
                    current = cand_list
                    steps += 1
                    progress = True
            if not progress:
                break

    return current, steps


def shrink_bytes(
    initial_bytes: bytes,
    prop_name: str,
    max_steps: int = 1000,
) -> tuple[bytes, int]:
    initial = list(initial_bytes)
    shrunk_list, steps = shrink_list(initial, 0, prop_name, max_steps)
    return bytes(shrunk_list), steps


# --- Replay Reference (VER05 §5) -------------------------------------------
def simulate_replay(seed: int, prop_name: str) -> tuple[int, int]:
    rng = SplitMix64(seed)
    prop = PROPERTIES[prop_name]
    initial_fail: int | None = None
    for _ in range(1000):
        val = rng.bounded(1000)
        if not prop(val):
            initial_fail = val
            break
    assert initial_fail is not None, (
        f"property {prop_name} was not falsified within 1000 draws"
    )
    minimal, _ = shrink_int(initial_fail, 0, 0, 1000, prop_name)
    return initial_fail, minimal


# --- Corpus Handling and Checking (VER05 §6) -------------------------------
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

    ops_seen: set[str] = set()
    seen_registered_ids: set[str] = set()
    rejected_id_present = False

    for case in cases:
        cid, op = case.get("id"), case.get("op")
        if not cid or not isinstance(cid, str):
            problems.append(f"case missing valid string id: {case}")
            continue
        if op not in SHAPES:
            problems.append(f"{cid}: unknown op {op!r}")
            continue
        ops_seen.add(op)
        shape = SHAPES[op] | {"id", "op"}
        if set(case) != shape:
            problems.append(f"{cid}: fields {sorted(case)} != {sorted(shape)}")
            continue

        try:
            value = {
                k: tfv.from_json(v) for k, v in case.items() if k not in ("id", "op")
            }
        except tfv.TfvError as err:
            problems.append(f"{cid}: TFV conversion error: {err}")
            continue

        if op == "gen_primitive":
            ref = gen_primitive(value["seed"][1], value["type"][1], value["count"][1])
            if value["outputs"][1] != ref:
                problems.append(f"{cid}: gen_primitive outputs mismatch")

        elif op == "gen_bounded":
            ref = gen_bounded(
                value["seed"][1],
                value["min"][1],
                value["max"][1],
                value["count"][1],
            )
            if value["outputs"][1] != ref:
                problems.append(f"{cid}: gen_bounded outputs mismatch")

        elif op == "gen_bytes":
            ref = gen_bytes(
                value["seed"][1],
                value["min_len"][1],
                value["max_len"][1],
                value["count"][1],
            )
            if value["outputs"][1] != ref:
                problems.append(f"{cid}: gen_bytes outputs mismatch")

        elif op in ("shrink_int", "shrink_refinement"):
            minimal, steps = shrink_int(
                value["initial"][1],
                value["target"][1],
                value["min"][1],
                value["max"][1],
                value["property"][1],
            )
            if minimal != value["expected_minimal"][1]:
                problems.append(
                    f"{cid}: {op} minimal mismatch: got {value['expected_minimal'][1]}, want {minimal}"
                )
            if steps != value["expected_steps"][1]:
                problems.append(
                    f"{cid}: {op} step count mismatch: got {value['expected_steps'][1]}, want {steps}"
                )

        elif op == "shrink_list":
            init_list = [item[1] for item in value["initial"][1]]
            exp_list = [item[1] for item in value["expected_minimal"][1]]
            minimal_list, steps = shrink_list(
                init_list,
                value["elem_target"][1],
                value["property"][1],
            )
            if minimal_list != exp_list:
                problems.append(
                    f"{cid}: shrink_list minimal mismatch: got {exp_list}, want {minimal_list}"
                )
            if steps != value["expected_steps"][1]:
                problems.append(
                    f"{cid}: shrink_list steps mismatch: got {value['expected_steps'][1]}, want {steps}"
                )

        elif op == "shrink_bytes":
            init_bytes = value["initial"][1]
            exp_bytes = value["expected_minimal"][1]
            minimal_bytes, steps = shrink_bytes(
                init_bytes,
                value["property"][1],
            )
            if minimal_bytes != exp_bytes:
                problems.append(
                    f"{cid}: shrink_bytes minimal mismatch: got {exp_bytes}, want {minimal_bytes}"
                )
            if steps != value["expected_steps"][1]:
                problems.append(
                    f"{cid}: shrink_bytes steps mismatch: got {value['expected_steps'][1]}, want {steps}"
                )

        elif op == "replay_consistency":
            initial_fail, minimal = simulate_replay(
                value["seed"][1], value["property"][1]
            )
            if initial_fail != value["initial_fail"][1]:
                problems.append(
                    f"{cid}: replay initial_fail mismatch: got {value['initial_fail'][1]}, want {initial_fail}"
                )
            if minimal != value["expected_minimal"][1]:
                problems.append(
                    f"{cid}: replay minimal mismatch: got {value['expected_minimal'][1]}, want {minimal}"
                )

        elif op == "filter_exhausted":
            if (
                value["verdict"][1] != "fail"
                or value["failure_id"][1] != "property-exhausted"
            ):
                problems.append(
                    f"{cid}: filter_exhausted must report fail/property-exhausted"
                )

        elif op == "failure_id_valid":
            candidate = value["input"][1]
            got = candidate in REGISTERED_FAILURE_IDS
            if got != value["valid"][1]:
                problems.append(
                    f"{cid}: failure_id_valid membership disagrees for {candidate}"
                )
            if got:
                seen_registered_ids.add(candidate)
            else:
                rejected_id_present = True

        elif op == "corpus_reject":
            text = value["text"][1]
            try:
                tfv.parse_file(text)
            except tfv.TfvError:
                pass
            else:
                problems.append(
                    f"{cid}: corpus_reject text was accepted by strict parser"
                )

    missing_ops = set(SHAPES) - ops_seen
    if missing_ops:
        problems.append(
            f"pbt-v1: operations never exercised in corpus: {sorted(missing_ops)}"
        )

    missing_fids = REGISTERED_FAILURE_IDS - seen_registered_ids
    if missing_fids:
        problems.append(
            f"pbt-v1: registered failure IDs missing: {sorted(missing_fids)}"
        )

    if not rejected_id_present:
        problems.append(
            "pbt-v1: failure_id_valid needs at least one unregistered-ID rejection"
        )

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
        target.write_text(generated, encoding="utf-8", newline="\n")
    elif not target.exists() or target.read_text(encoding="utf-8") != generated:
        problems.append("cases.tfv is stale: run with --write")
    elif tfv.parse_file(generated) != to_records(cases):
        problems.append("cases.tfv does not round-trip")

    for problem in problems:
        print(f"pbt-v1: {problem}", file=sys.stderr)
    print(f"pbt-v1: {len(cases)} cases, {len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
