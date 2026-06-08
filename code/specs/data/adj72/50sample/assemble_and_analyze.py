#!/usr/bin/env python3
"""Assemble the 12 round/batch outputs into per-claim resample triples and
compute byte-stability. Scored at the STRICT 3-of-3 identical bar (per the
n=4 probe lesson: a 2-of-3 majority can be a shared approximation)."""

import itertools
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "out")


def normalize(s):
    if s is None:
        return None
    s = s.lower().strip().strip('"“”\'')
    s = re.sub(r"\s+", " ", s)
    s = re.sub(r"[^\w\s]", "", s)
    return s.strip()


def load_round(r):
    """Return {id: record} merged across the 4 batches of round r."""
    merged = {}
    for b in range(1, 5):
        path = os.path.join(OUT, f"round{r}_batch{b}.json")
        arr = json.load(open(path))
        for rec in arr:
            merged[rec["id"]] = rec
    return merged


def main():
    claims = json.load(open(os.path.join(HERE, "claims.json")))["claims"]
    rounds = [load_round(1), load_round(2), load_round(3)]

    per_claim = {}
    for c in claims:
        cid = c["id"]
        recs = [rounds[i].get(cid, {}) for i in range(3)]
        passages = [r.get("verbatim_source_passage") for r in recs]
        exists = [r.get("exists") for r in recs]
        confs = [r.get("confidence_passage_is_verbatim_accurate") for r in recs]
        per_claim[cid] = {
            "category": c["category"],
            "passages": passages,
            "exists": exists,
            "confs": confs,
        }

    # Stability buckets
    print(f"{'id':5} {'cat':3} {'stability':24} {'exists':14} {'meanconf':>8}")
    print("-" * 70)
    buckets = {"STABLE_3of3": [], "SPLIT_2of3": [], "UNSTABLE": [], "STABLE_NULL": []}
    rows = []
    for cid, d in per_claim.items():
        norm = [normalize(p) for p in d["passages"]]
        # Treat all-null (model says no verbatim source exists) distinctly
        all_null = all(p is None for p in d["passages"])
        pairs = list(itertools.combinations(range(3), 2))
        exact = [norm[i] == norm[j] and norm[i] is not None for i, j in pairs]
        exact_rate = sum(exact) / len(exact)
        if all_null:
            bucket = "STABLE_NULL"  # consistently "no verbatim source" = honest stable backtrack
        elif exact_rate == 1.0:
            bucket = "STABLE_3of3"
        elif exact_rate > 0.0:
            bucket = "SPLIT_2of3"
        else:
            bucket = "UNSTABLE"
        buckets[bucket].append(cid)
        ex = d["exists"]
        ex_str = f"{sum(1 for e in ex if e is True)}T/{sum(1 for e in ex if e is False)}F"
        confs = [c for c in d["confs"] if isinstance(c, (int, float))]
        mc = sum(confs) / len(confs) if confs else float("nan")
        rows.append((cid, d["category"], bucket, ex_str, mc))
        print(f"{cid:5} {d['category']:3} {bucket:24} {ex_str:14} {mc:8.2f}")

    print()
    print("Bucket counts:")
    for b, ids in buckets.items():
        print(f"  {b:14}: {len(ids):2}  {ids}")

    # Cross-tab stability bucket x pre-registered category
    print()
    print("Stability bucket x pre-registered category:")
    cats = ["A", "B", "C"]
    bks = ["STABLE_3of3", "SPLIT_2of3", "UNSTABLE", "STABLE_NULL"]
    print(f"{'':14} " + " ".join(f"{c:>4}" for c in cats))
    for bk in bks:
        counts = {c: 0 for c in cats}
        for cid, d in per_claim.items():
            if cid in buckets[bk]:
                counts[d["category"]] += 1
        print(f"{bk:14} " + " ".join(f"{counts[c]:4}" for c in cats))

    json.dump(per_claim, open(os.path.join(HERE, "assembled.json"), "w"), indent=2)
    print("\nwrote assembled.json")


if __name__ == "__main__":
    main()
