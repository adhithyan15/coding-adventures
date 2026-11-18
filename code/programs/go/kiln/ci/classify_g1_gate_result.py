#!/usr/bin/env python3
"""Validate the official G1 gate's process status and emitted measurements."""

import re
import sys
from pathlib import Path


FLOOR_CROSSING_NS = 4_757_143  # ceil(66,600 bytes / 14,000,000 bytes/s)


class InvalidReceipt(ValueError):
    pass


def parse_exit_code(value: str) -> int:
    if value == "":
        raise InvalidReceipt("missing official process exit code")
    try:
        return int(value)
    except ValueError as error:
        raise InvalidReceipt("invalid official process exit code") from error


def classify(log: str, exit_code: int, arm: str) -> str:
    if arm not in {"baseline", "candidate"}:
        raise InvalidReceipt(f"unknown arm: {arm}")
    if len(re.findall(r"^=== RUN\s+TestSandboxTraceGateSynthetic$", log, re.M)) != 1:
        raise InvalidReceipt("official test did not start exactly once")
    terminal = re.findall(r"^--- (PASS|FAIL): TestSandboxTraceGateSynthetic(?:\s|$)", log, re.M)
    if len(terminal) != 1:
        raise InvalidReceipt("official test lacks exactly one terminal PASS/FAIL")
    census = re.findall(
        r"^.*PROBE: trace synthetic-4mib-census bytes=4195800 lines=58968 .*gated=0 repeats=7$",
        log,
        re.M,
    )
    if len(census) != 1:
        raise InvalidReceipt("missing exact ungated census marker")
    slices = re.findall(
        r"^.*PROBE: trace synthetic-slice bytes=66600 lines=936 best_mb_s=([0-9]+\.[0-9]+) gated=1 repeats=7$",
        log,
        re.M,
    )
    if len(slices) != 1:
        raise InvalidReceipt("missing exact official slice marker")
    rate = float(slices[0])
    if rate <= 0:
        raise InvalidReceipt("official rounded rate is not positive")

    if terminal[0] == "PASS":
        if exit_code != 0 or rate < 14.0:
            raise InvalidReceipt("PASS marker, exit code, and reported floor outcome disagree")
        return "PASS"

    failure = re.findall(
        r"synthetic-slice: the grammar path's best of 7 passes read a 66600 byte slice in "
        r"([0-9]+(?:\.[0-9]+)?)(ns|us|µs|ms|s) \(([0-9]+(?:\.[0-9]+)?) MB/s\), "
        r"below the 14 MB/s floor or not a measurement \(gate G1, KILN04 section 4\.9\)",
        log,
    )
    if exit_code != 1 or len(failure) != 1:
        raise InvalidReceipt("FAIL is not a single exact G1 throughput failure with exit 1")
    amount, unit, error_rate = failure[0]
    scale = {"ns": 1, "us": 1_000, "µs": 1_000, "ms": 1_000_000, "s": 1_000_000_000}[unit]
    elapsed_ns = float(amount) * scale
    if elapsed_ns < FLOOR_CROSSING_NS or float(error_rate) > 14.0:
        raise InvalidReceipt("failure is zero/too-fast or does not show a below-floor measurement")
    if float(error_rate) != rate:
        raise InvalidReceipt("error and probe rates disagree")
    if arm == "candidate":
        return f"CANDIDATE_GATE_MISS rate_mb_s={rate:.1f} elapsed_ns={elapsed_ns:.0f}"
    return f"MEASURED_BELOW_FLOOR rate_mb_s={rate:.1f} elapsed_ns={elapsed_ns:.0f}"


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: classify_g1_gate_result.py LOG EXIT_CODE baseline|candidate", file=sys.stderr)
        return 2
    try:
        exit_code = parse_exit_code(sys.argv[2])
        result = classify(Path(sys.argv[1]).read_text(errors="replace"), exit_code, sys.argv[3])
    except (OSError, ValueError) as error:
        print(f"INVALID: {error}", file=sys.stderr)
        return 2
    print(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
