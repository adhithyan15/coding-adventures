#!/usr/bin/env python3
"""ADJ57 Layer 1 — the byte-coverage checker for the case -> IR decomposition.

The invariant: EVERY byte of the input case is either represented in a typed fact
or explicitly discarded with a reason. Nothing is silently dropped.

We enforce this with the strongest possible model — a PARTITION. The ingester
emits the case as an ordered list of `segments`, each either a `fact` (typed, with
a retrievable span = its own text) or a `discard` (with a reason). The checker
verifies the segments concatenate back to the EXACT input, byte for byte. If they
do, coverage is total by construction: no gaps (silent omission), no overlaps,
no LLM byte-arithmetic (the model emits literal text in order; the framework
derives offsets).

A `fact` segment additionally carries its typed interpretation (term); the byte
span IS the segment text, so every fact is trivially retrievable to source bytes.

Usage:
  python coverage.py <case.txt> <segments.json>
segments.json: [{"text": "...", "kind": "fact"|"discard",
                 "term": "...", "reason": "..."}]
"""
from __future__ import annotations

import json
import sys
from pathlib import Path


def check(case_text: str, segments: list[dict]) -> dict:
    reconstructed = "".join(s["text"] for s in segments)
    ok = reconstructed == case_text
    result: dict = {"covered": ok}
    if not ok:
        # locate the first byte where the partition diverges from the input
        i = 0
        m = min(len(reconstructed), len(case_text))
        while i < m and reconstructed[i] == case_text[i]:
            i += 1
        result["first_divergence_offset"] = i
        result["expected_next"] = case_text[i:i + 60]
        result["got_next"] = reconstructed[i:i + 60]
        result["reconstructed_len"] = len(reconstructed)
        result["case_len"] = len(case_text)
        return result

    facts = [s for s in segments if s.get("kind") == "fact"]
    discards = [s for s in segments if s.get("kind") == "discard"]
    fact_bytes = sum(len(s["text"]) for s in facts)
    discard_bytes = sum(len(s["text"]) for s in discards)
    total = len(case_text)
    # every fact must carry a typed term, every discard a reason — else the byte is
    # neither represented nor reasoned-about, which violates the invariant in spirit
    untyped = [i for i, s in enumerate(facts) if not s.get("term")]
    unreasoned = [i for i, s in enumerate(discards) if not s.get("reason")]
    result.update({
        "total_bytes": total,
        "facts": len(facts), "fact_bytes": fact_bytes,
        "discards": len(discards), "discard_bytes": discard_bytes,
        "pct_in_facts": round(100 * fact_bytes / total, 1) if total else 0,
        "pct_discarded": round(100 * discard_bytes / total, 1) if total else 0,
        "untyped_facts": untyped,
        "unreasoned_discards": unreasoned,
        "clean": ok and not untyped and not unreasoned,
    })
    return result


def main() -> None:
    case_text = Path(sys.argv[1]).read_text()
    segments = json.loads(Path(sys.argv[2]).read_text())
    r = check(case_text, segments)
    print(json.dumps({k: v for k, v in r.items() if k != "content"}, indent=2))
    if not r["covered"]:
        print(f"\n>>> COVERAGE VIOLATION at byte {r['first_divergence_offset']}:")
        print(f"    case expects: {r['expected_next']!r}")
        print(f"    segments gave: {r['got_next']!r}")
        sys.exit(1)
    if not r["clean"]:
        print("\n>>> COVERED but not CLEAN: "
              f"{len(r['untyped_facts'])} untyped facts, {len(r['unreasoned_discards'])} unreasoned discards")
        sys.exit(2)
    print(f"\n>>> TOTAL COVERAGE: {r['facts']} facts ({r['pct_in_facts']}%) + "
          f"{r['discards']} discards ({r['pct_discarded']}%) = 100% of {r['total_bytes']} bytes, every fact typed + every discard reasoned.")


if __name__ == "__main__":
    main()
