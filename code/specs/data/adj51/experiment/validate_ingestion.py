#!/usr/bin/env python3
"""Byte-accounting validator for the ADJ51 ingestion experiment.

Usage:
  python3 validate_ingestion.py <prose.txt> <ingestion.json>

Exits 0 on PASS; exits 1 with diagnostics on FAIL.

Contract:
  - byte_dispositions ranges union to exactly [0, len(prose_bytes))
  - no overlaps between ranges
  - every "extracted" disposition's observation_id refers to an entry
    in observations
  - every observation's source_span is byte-contained in some
    "extracted" range
  - allowed disposition tags: extracted, discarded_as_non_factual,
    below_extraction_threshold, ambiguous_but_flagged
  - observations are objects with at least: id, term, source_span,
    confidence; optionally raw_value (held string from source)
  - queries is an array of objects with at least: id, term, source_span
    (source_span may be null if the query is implied by the input's
    genre rather than literally present in the prose)

The validator does NOT enforce a domain. It only enforces the
byte-accounting and shape contracts.
"""

import json
import sys


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: validate_ingestion.py <prose.txt> <ingestion.json>")
        return 2

    prose_path, ingestion_path = sys.argv[1], sys.argv[2]
    with open(prose_path, "rb") as f:
        prose = f.read()
    with open(ingestion_path) as f:
        ingestion = json.load(f)

    n = len(prose)
    print(f"prose length: {n} bytes")

    observations = ingestion.get("observations", [])
    dispositions = ingestion.get("byte_dispositions", [])
    queries = ingestion.get("queries", [])

    print(f"observations: {len(observations)}")
    print(f"queries:      {len(queries)}")
    print(f"byte_dispositions: {len(dispositions)}")

    obs_by_id = {o["id"]: o for o in observations}

    allowed = {
        "extracted",
        "discarded_as_non_factual",
        "below_extraction_threshold",
        "ambiguous_but_flagged",
    }

    errors: list[str] = []
    coverage = [False] * n
    sorted_d = sorted(dispositions, key=lambda d: d["range"][0])
    last_end = 0
    for d in sorted_d:
        rng = d["range"]
        start, end = rng[0], rng[1]
        tag = d["disposition"]
        if tag not in allowed:
            errors.append(f"disallowed disposition tag: {tag!r}")
        if start < 0 or end > n or start >= end:
            errors.append(f"out-of-range or empty span [{start}, {end})")
            continue
        if start < last_end:
            errors.append(
                f"overlap or out-of-order: [{start}, {end}) starts before previous end {last_end}"
            )
        if start > last_end:
            errors.append(f"gap: bytes [{last_end}, {start}) are uncovered")
        for i in range(start, end):
            coverage[i] = True
        last_end = end
        if tag == "extracted":
            oid = d.get("observation_id")
            if oid not in obs_by_id:
                errors.append(
                    f"extracted disposition references unknown observation_id {oid!r}"
                )

    if last_end < n:
        errors.append(f"trailing gap: bytes [{last_end}, {n}) are uncovered")

    uncovered = [i for i, c in enumerate(coverage) if not c]
    if uncovered:
        errors.append(f"{len(uncovered)} bytes uncovered (first 5: {uncovered[:5]})")

    extracted_ranges = [
        d["range"] for d in dispositions if d["disposition"] == "extracted"
    ]
    for o in observations:
        for required in ("id", "term", "source_span", "confidence"):
            if required not in o:
                errors.append(f"observation {o.get('id')} missing field {required!r}")
        sp = o.get("source_span")
        if not sp:
            continue
        if not any(r[0] <= sp[0] and sp[1] <= r[1] for r in extracted_ranges):
            errors.append(
                f"observation {o['id']} source_span {sp} is not contained in any extracted disposition"
            )

    for q in queries:
        for required in ("id", "term"):
            if required not in q:
                errors.append(f"query {q.get('id')} missing field {required!r}")
        # source_span optional and may be null
    if not queries:
        errors.append(
            "no queries extracted — every input must yield at least one query for the engine to answer"
        )

    if errors:
        print("FAIL")
        for e in errors:
            print(f"  - {e}")
        return 1

    counts: dict[str, int] = {}
    for d in dispositions:
        counts[d["disposition"]] = counts.get(d["disposition"], 0) + (
            d["range"][1] - d["range"][0]
        )
    print("PASS — 100% byte coverage, no overlaps, all references valid")
    print("byte counts by disposition:")
    for tag in sorted(counts):
        print(f"  {tag}: {counts[tag]} bytes")
    print(f"observations extracted: {len(observations)}")
    print(f"queries extracted:      {len(queries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
