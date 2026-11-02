#!/usr/bin/env python3
"""Validate the rng conformance corpus and fill in its expected results.

`code/specs/fixtures/rng-v1/cases.json` pins exact outputs of the three
Tier 1 generators in `code/specs/rng.md`. Every language's `rng` package must
reproduce them. This script is the reference: standard-library Python written
from the spec, sharing no code with any `rng` package.

    python3 code/scripts/rng_conformance.py           # check (CI)
    python3 code/scripts/rng_conformance.py --write   # fill in results, regenerate cases.txt

A case names a generator, a seed and a list of calls. `--write` runs the calls
on the reference and records each result; the default mode re-runs them and
fails on any difference. Because the results come from this reference, the
check also anchors the reference itself to facts it did not produce:

* the spec's published seed-1 `next_u32` table for all three generators;
* every range result lies in `[min, max]`;
* for every generator the corpus exercises each regime of
  `next_int_in_range` (spec v2): a single-value range, the 32-bit regime with
  a rejection actually taken, the 64-bit regime with a rejection actually
  taken, and the whole 64-bit domain;
* `min > max` is an error that consumes no draw.

Integers are decimal strings and floats are 16 lowercase hex digits of their
IEEE-754 bits, so no JSON parser can round them.
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "code/specs/fixtures/rng-v1/cases.json"
FLAT = "cases.txt"  # generated beside cases.json; see flat_form()

M32 = (1 << 32) - 1
M64 = (1 << 64) - 1
I64_MIN, I64_MAX = -(1 << 63), (1 << 63) - 1
A = 6364136223846793005
C = 1442695040888963407
RANGE_ERROR = "RNG_RANGE_INVALID"

# The spec's "Known Reference Values (seed = 1)" table.
PUBLISHED_SEED_1 = {
    "lcg": [1817669548, 2187888307, 2784682393],
    "xorshift64": [1082269761, 201397313, 1854285353],
    "pcg32": [1412771199, 1791099446, 124312908],
}


# --- the three generators, from the spec ------------------------------------
class Lcg:
    def __init__(self, seed: int) -> None:
        self.state = seed & M64

    def next_u32(self) -> int:
        self.state = (self.state * A + C) & M64
        return self.state >> 32


class Xorshift64:
    def __init__(self, seed: int) -> None:
        self.state = (seed & M64) or 1  # 0 is a fixed point

    def next_u32(self) -> int:
        x = self.state
        x ^= (x << 13) & M64
        x ^= x >> 7
        x ^= (x << 17) & M64
        self.state = x
        return x & M32


class Pcg32:
    def __init__(self, seed: int) -> None:
        state = C  # 0 * A + C
        state = (state + (seed & M64)) & M64
        self.state = (state * A + C) & M64

    def next_u32(self) -> int:
        old = self.state
        self.state = (old * A + C) & M64
        xorshifted = (((old >> 18) ^ old) >> 27) & M32
        rot = old >> 59
        return ((xorshifted >> rot) | (xorshifted << ((32 - rot) % 32))) & M32


GENERATORS = {"lcg": Lcg, "xorshift64": Xorshift64, "pcg32": Pcg32}


class Draws:
    """A generator plus the derived operations, counting rejections."""

    def __init__(self, generator: str, seed: int) -> None:
        self.gen = GENERATORS[generator](seed)
        self.rejections = {"32": 0, "64": 0}

    def next_u32(self) -> int:
        return self.gen.next_u32()

    def next_u64(self) -> int:
        hi = self.gen.next_u32()
        return (hi << 32) | self.gen.next_u32()

    def next_float_bits(self) -> int:
        value = self.gen.next_u32() / 2**32
        return struct.unpack("<Q", struct.pack("<d", value))[0]

    def next_int_in_range(self, lo: int, hi: int) -> tuple[int, str]:
        """The value, and which regime produced it (spec v2)."""
        if not (I64_MIN <= lo <= hi <= I64_MAX):
            raise ValueError(RANGE_ERROR)
        span = hi - lo + 1
        if span <= 1 << 32:
            threshold = ((1 << 32) - span) % span
            while (r := self.next_u32()) < threshold:
                self.rejections["32"] += 1
            return lo + r % span, "32" if span > 1 else "single"
        if span < 1 << 64:
            threshold = ((1 << 64) - span) % span
            while (r := self.next_u64()) < threshold:
                self.rejections["64"] += 1
            return lo + r % span, "64"
        return lo + self.next_u64(), "full"


# --- corpus -----------------------------------------------------------------
CALL_KEYS = {
    "next_u32": {"op"},
    "next_u64": {"op"},
    "next_float": {"op"},
    "next_int_in_range": {"op", "min", "max"},
}


def run_case(case: dict) -> tuple[list[dict], dict[str, int], set[str]]:
    """Run a case's calls on the reference. Returns the calls with their
    results, the rejection counts, and the regimes seen."""
    draws = Draws(case["generator"], int(case["seed"]))
    out: list[dict] = []
    regimes: set[str] = set()
    for call in case["calls"]:
        op = call["op"]
        done = {key: call[key] for key in ("op", "min", "max") if key in CALL_KEYS[op]}
        if op == "next_u32":
            done["result"] = str(draws.next_u32())
        elif op == "next_u64":
            done["result"] = str(draws.next_u64())
        elif op == "next_float":
            done["result_bits"] = f"{draws.next_float_bits():016x}"
        else:
            try:
                value, regime = draws.next_int_in_range(
                    int(call["min"]), int(call["max"])
                )
            except ValueError:
                done["error"] = RANGE_ERROR
            else:
                done["result"] = str(value)
                regimes.add(regime)
        out.append(done)
    return out, draws.rejections, regimes


def load() -> dict:
    def no_duplicate_keys(pairs: list[tuple[str, object]]) -> dict:
        keys = [key for key, _ in pairs]
        if len(keys) != len(set(keys)):
            raise SystemExit(f"duplicate JSON key in {keys}")
        return dict(pairs)

    return json.loads(
        FIXTURE.read_text(encoding="utf-8"), object_pairs_hook=no_duplicate_keys
    )


def canonical_int(text: object) -> bool:
    """A decimal string with no '+', no leading zeros and never '-0'.

    Length is capped so int() never meets Python's digit limit."""
    if not isinstance(text, str) or not 1 <= len(text) <= 21 or not text.isascii():
        return False
    digits = text.removeprefix("-")
    return digits.isdigit() and text == str(int(text)) and text != "-0"


def call_is_well_formed(call: object, *, results_required: bool = True) -> bool:
    if not isinstance(call, dict) or call.get("op") not in CALL_KEYS:
        return False
    op = call["op"]
    if not results_required:
        results = set(call) - CALL_KEYS[op]
    elif op == "next_float":
        results = {"result_bits"}
    elif op == "next_int_in_range" and "error" in call:
        results = {"error"}
    else:
        results = {"result"}
    if set(call) != CALL_KEYS[op] | results:
        return False
    return op != "next_int_in_range" or all(
        canonical_int(call[k]) for k in ("min", "max")
    )


def check(document: object, *, results_required: bool = True) -> list[str]:
    if (
        not isinstance(document, dict)
        or set(document) != {"schema_version", "cases"}
        or document["schema_version"] != 1
        or not isinstance(document["cases"], list)
    ):
        return ["cases.json must be {schema_version: 1, cases: [...]}"]
    problems: list[str] = []
    ids = [
        case.get("id") if isinstance(case, dict) else None for case in document["cases"]
    ]
    if len(ids) != len(set(ids)):
        problems.append("duplicate case id")
    coverage: dict[str, set[str]] = {generator: set() for generator in GENERATORS}
    for case in document["cases"]:
        cid = case.get("id") if isinstance(case, dict) else None
        if not isinstance(case, dict) or set(case) != {
            "id",
            "generator",
            "seed",
            "calls",
        }:
            problems.append(f"{cid}: fields must be id, generator, seed, calls")
            continue
        if case["generator"] not in GENERATORS:
            problems.append(f"{cid}: unknown generator {case['generator']!r}")
            continue
        if not canonical_int(case["seed"]) or not 0 <= int(case["seed"]) <= M64:
            problems.append(f"{cid}: seed must be a canonical u64 decimal string")
            continue
        if not isinstance(case["calls"], list) or not all(
            call_is_well_formed(call, results_required=results_required)
            for call in case["calls"]
        ):
            problems.append(
                f"{cid}: a call has the wrong fields or a non-canonical integer"
            )
            continue
        expected, rejections, regimes = run_case(case)
        if expected != case["calls"]:
            problems.append(f"{cid}: results differ from the reference (run --write?)")
        for call in expected:
            in_range = call["op"] == "next_int_in_range" and "result" in call
            if in_range and not int(call["min"]) <= int(call["result"]) <= int(
                call["max"]
            ):
                problems.append(f"{cid}: a range result lies outside [min, max]")
        seen = coverage[case["generator"]]
        seen |= regimes
        seen |= {f"reject{bits}" for bits, count in rejections.items() if count}
        if any("error" in call for call in expected):
            seen.add("error")
    needed = {"single", "32", "reject32", "64", "reject64", "full", "error"}
    for generator, seen in coverage.items():
        if needed - seen:
            problems.append(f"{generator}: never exercised: {sorted(needed - seen)}")
    for generator, values in PUBLISHED_SEED_1.items():
        draws = Draws(generator, 1)
        if [draws.next_u32() for _ in values] != values:
            problems.append(
                f"{generator}: reference disagrees with the spec's seed-1 table"
            )
    return problems


def flat_form(document: dict) -> str:
    """The corpus as whitespace-separated lines, for lanes with no JSON reader.

    Every token is free of spaces, so a lane can read it with fscanf, a line
    split, or a stream extractor::

        case <id> <generator> <seed>
        next_u32 <result>
        next_u64 <result>
        next_float <16 hex digits of the IEEE-754 bits>
        next_int_in_range <min> <max> <result | error>
        end

    It is generated from cases.json by --write; the check fails if stale.
    """
    lines = [
        "# rng-v1 corpus, flat form. Generated from cases.json by",
        "# code/scripts/rng_conformance.py --write; do not edit.",
    ]
    for case in document["cases"]:
        lines.append(f"case {case['id']} {case['generator']} {case['seed']}")
        for call in case["calls"]:
            op = call["op"]
            if op == "next_float":
                lines.append(f"{op} {call['result_bits']}")
            elif op == "next_int_in_range":
                outcome = "error" if "error" in call else call["result"]
                lines.append(f"{op} {call['min']} {call['max']} {outcome}")
            else:
                lines.append(f"{op} {call['result']}")
        lines.append("end")
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--write", action="store_true", help="fill in results")
    args = parser.parse_args(argv)
    document = load()
    if args.write:
        # Calls may lack results here; only their shape must be valid.
        problems = [
            p for p in check(document, results_required=False) if "differ from" not in p
        ]
        if not any(": " in p and "never exercised" not in p for p in problems):
            for case in document["cases"]:
                case["calls"] = run_case(case)[0]
            FIXTURE.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
            (FIXTURE.parent / FLAT).write_text(flat_form(document), encoding="utf-8")
    problems = check(document)
    flat = FIXTURE.parent / FLAT
    if not problems and (
        not flat.exists() or flat.read_text(encoding="utf-8") != flat_form(document)
    ):
        problems.append(f"{FLAT} is stale: run with --write")
    for problem in problems:
        print(f"rng-v1: {problem}", file=sys.stderr)
    print(f"rng-v1: {len(document['cases'])} cases, {len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
