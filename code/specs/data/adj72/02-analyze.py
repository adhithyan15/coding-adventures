#!/usr/bin/env python3
"""ADJ72 stability analysis. Computes byte-stability of the verbatim
source passages across the K independent resamples per claim.

Two metrics per claim:
  - exact_match_rate: fraction of resample PAIRS that are byte-identical
    after normalization (lowercase, collapse whitespace, strip surrounding
    quotes/punctuation).
  - mean_token_jaccard: mean pairwise Jaccard over the normalized token
    sets — a graded similarity for near-misses.

High stability (both near 1.0) => the model reproduces the same passage =>
evidence of genuine retrieval. Low stability => the passages diverge =>
the model is generating, not retrieving.
"""

import itertools
import json
import re
import sys


def normalize(s: str) -> str:
    s = s.lower().strip()
    s = s.strip('"“”\'')
    s = re.sub(r"\s+", " ", s)
    s = re.sub(r"[^\w\s]", "", s)  # drop punctuation for the comparison
    return s.strip()


def tokens(s: str) -> set:
    return set(normalize(s).split())


def jaccard(a: str, b: str) -> float:
    ta, tb = tokens(a), tokens(b)
    if not ta and not tb:
        return 1.0
    return len(ta & tb) / len(ta | tb)


def main() -> int:
    path = sys.argv[1] if len(sys.argv) > 1 else "01-raw-outputs.json"
    data = json.load(open(path))

    print(f"{'claim':6} {'exact_pair_rate':>16} {'mean_jaccard':>14} {'mean_conf':>10}  verdict")
    print("-" * 76)
    rows = []
    for cid, c in data["claims"].items():
        passages = [r["passage"] for r in c["resamples"]]
        confs = [r["confidence"] for r in c["resamples"]]
        pairs = list(itertools.combinations(range(len(passages)), 2))
        exact = [normalize(passages[i]) == normalize(passages[j]) for i, j in pairs]
        jac = [jaccard(passages[i], passages[j]) for i, j in pairs]
        exact_rate = sum(exact) / len(exact)
        mean_jac = sum(jac) / len(jac)
        mean_conf = sum(confs) / len(confs)
        # Stability verdict: STABLE if all pairs exact; SPLIT if some pairs
        # exact (a majority converges); UNSTABLE if no pair exact.
        if exact_rate == 1.0:
            verdict = "STABLE (all resamples identical)"
        elif exact_rate > 0.0:
            verdict = "SPLIT (subset converges)"
        else:
            verdict = "UNSTABLE (all resamples differ)"
        rows.append((cid, exact_rate, mean_jac, mean_conf, verdict))
        print(f"{cid:6} {exact_rate:16.2f} {mean_jac:14.3f} {mean_conf:10.2f}  {verdict}")

    print()
    print("Interpretation key:")
    print("  STABLE  + high conf  => strong retrieval signal (trust, optionally CAS-confirm once)")
    print("  SPLIT / UNSTABLE     => soft spot: model is generating; flag + require external grounding")
    return 0


if __name__ == "__main__":
    sys.exit(main())
