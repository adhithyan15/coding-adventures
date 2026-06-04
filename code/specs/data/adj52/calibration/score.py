#!/usr/bin/env python3
"""Deterministic calibration-regression scorer for the ADJ52 engine.

WHY THIS EXISTS
===============
The softmax "calibration fix" (run-2) made things WORSE and nobody could see it,
because it was judged on the noisy end-to-end blind-judge loop at n=3. That loop
fuses three different things into one win/loss bit — correctness, calibration,
defensibility — so a fix that helped the aggregate while regressing individual
cases slipped straight through. That hidden per-case regression IS the "entropy"
we keep adding.

The engine is DETERMINISTIC: given a frozen (rulebook.adj, program.adj), the
posterior is a pure function of the engine code. So we can freeze a corpus of
(rulebook, program, ground-truth label) tuples ONCE, then score any engine change
offline in milliseconds — no LLM, no noise — and decompose the blind judge's
single bit back into correctness vs calibration.

THE ANTI-ENTROPY GATE (the whole point)
=======================================
A fix is rejected if ANY case regresses — top-1 flips correct->wrong, or a case
becomes newly confidently-wrong — EVEN IF the aggregate improves. Aggregate-only
gates are exactly how softmax got in. Use `diff` mode to see the per-case signed
deltas, not just the headline.

USAGE
-----
  # score the current engine against a frozen corpus -> per-case records + metrics
  python score.py score corpus.json out/before.json

  # ... change the engine, rebuild, re-score ...
  python score.py score corpus.json out/after.json

  # the gate: what regressed, per case?
  python score.py diff out/before.json out/after.json

CORPUS FORMAT (corpus.json)
---------------------------
  [
    {"id": "case-7", "rulebook": "cases/case-7/rulebook.adj",
     "program": "cases/case-7/program.adj", "correct_term": "diagnosis(zenker_diverticulum)"},
    ...
  ]
Paths are relative to this crate's manifest dir (the adj52 dir). `correct_term`
is the diagnosis() query term that matches the held-aside ground truth; label it
once per case from the ground_truth prose.
"""

from __future__ import annotations

import json
import math
import re
import subprocess
import sys
from pathlib import Path

# This file lives in <adj52>/calibration/score.py; the crate manifest dir is its parent.
CRATE_DIR = Path(__file__).resolve().parent.parent
MANIFEST = CRATE_DIR / "Cargo.toml"

# The differential = every query EXCEPT next-step/recommendation queries (those are
# decisions, not probability estimates of the answer). The deriver isn't consistent
# about the conclusion predicate (diagnosis / unifying_diagnosis / ...), so we
# exclude by functor blocklist rather than allowlisting one name.
NON_DIFFERENTIAL = ("next_step(", "recommended_next_step(", "next_action(", "recommend(")
def is_differential(term: str) -> bool:
    return not term.startswith(NON_DIFFERENTIAL)
QUERY_LINE = re.compile(r"^Query\s+\d+/\d+:\s+(.*)$")
# RAW posterior — what the differential is RANKED on (preserves correctness).
POST_LINE = re.compile(r"^\s*Posterior:\s+P\s*=\s*([0-9.]+)")
# H2 REPORTED/tempered posterior — what CALIBRATION is scored on. Absent on the
# pre-H2 engine, in which case reported == raw (handled in run_engine).
REPORTED_LINE = re.compile(r"^\s*Reported \(H2[^)]*\):\s+P\s*=\s*([0-9.]+)")


def normalize_term(t: str) -> str:
    """Compare terms ignoring whitespace + case (the engine prints them canonically,
    but hand-written labels drift)."""
    return "".join(t.split()).lower()


def run_engine(rulebook: str, program: str) -> dict[str, dict]:
    """Run the adj52 binary on one (rulebook, program) pair and return
    {query_term: {"raw": p, "reported": p}} for the differential queries.
    `raw` is the ranking posterior; `reported` is the H2-tempered calibration
    posterior (== raw on a pre-H2 engine that prints no Reported line). Deterministic."""
    import os
    # Inherit the FULL environment — mise's cargo shim needs more than PATH/HOME
    # (CARGO_HOME, RUSTUP_HOME, mise vars). Only override the two case selectors.
    env = {**os.environ, "ADJ52_RULEBOOK": rulebook, "ADJ52_PROGRAM": program}
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--manifest-path", str(MANIFEST), "--bin", "adj52", "."],
        capture_output=True, text=True, env=env, cwd=str(CRATE_DIR),
    )
    out = proc.stdout
    if "COMPILE ERROR" in out or proc.returncode != 0:
        raise RuntimeError(f"engine failed on {program}:\n{out}\n{proc.stderr[:500]}")
    # Walk the block per query: "Query k/n: <term>" -> "Posterior: P = raw"
    # -> (optional) "Reported (H2 …): P = tempered".
    posteriors: dict[str, dict] = {}
    pending: str | None = None
    for line in out.splitlines():
        qm = QUERY_LINE.match(line.strip())
        if qm:
            pending = qm.group(1).strip()
            continue
        pm = POST_LINE.match(line)
        if pm and pending is not None and is_differential(pending):
            posteriors[pending] = {"raw": float(pm.group(1)), "reported": float(pm.group(1))}
            continue
        rm = REPORTED_LINE.match(line)
        if rm and pending is not None and pending in posteriors:
            posteriors[pending]["reported"] = float(rm.group(1))
            pending = None
    return posteriors


def score_case(case: dict) -> dict:
    """Score one corpus case. Rank the differential on RAW posterior (preserves
    correctness); score calibration on the H2-tempered REPORTED posterior."""
    posts = run_engine(case["rulebook"], case["program"])
    if not posts:
        return {"id": case["id"], "error": "no diagnosis() queries found", "diag": {}}
    # top-1 is chosen on RAW — H2 reporting must never change who wins.
    top_term = max(posts, key=lambda t: posts[t]["raw"])
    top_raw = posts[top_term]["raw"]
    top_post = posts[top_term]["reported"]  # the calibrated confidence we report/score
    correct = case.get("correct_term", "")
    top1_correct = bool(correct) and normalize_term(top_term) == normalize_term(correct)
    p_correct = next((v["reported"] for t, v in posts.items()
                      if normalize_term(t) == normalize_term(correct)), float("nan"))
    return {
        "id": case["id"],
        "top_term": top_term,
        "top_raw": top_raw,
        "top_post": top_post,
        "correct_term": correct,
        "top1_correct": top1_correct,
        "p_correct": p_correct,
        "confidently_wrong": (not top1_correct) and top_post >= 0.90,
        "diag": {t: v["reported"] for t, v in posts.items()},
    }


def aggregate(records: list[dict]) -> dict:
    """Decompose into correctness vs calibration. These are the numbers a fix must
    move in the right direction WITHOUT regressing any single case."""
    scored = [r for r in records if "error" not in r]
    n = len(scored)
    if n == 0:
        return {"n": 0}
    acc = sum(r["top1_correct"] for r in scored) / n
    # Top-1 reliability: predicted = posterior of the chosen answer; outcome = was it right.
    brier = sum((r["top_post"] - (1.0 if r["top1_correct"] else 0.0)) ** 2 for r in scored) / n
    # log-loss on the top-1 decision (clamped to avoid inf on saturated posteriors —
    # the clamp itself is a quiet signal that saturation is hurting us).
    def clamp(p): return min(max(p, 1e-6), 1 - 1e-6)
    logloss = -sum(
        math.log(clamp(r["top_post"])) if r["top1_correct"] else math.log(1 - clamp(r["top_post"]))
        for r in scored
    ) / n
    saturated = sum(r["top_post"] >= 0.99 for r in scored)
    conf_wrong = sum(r["confidently_wrong"] for r in scored)
    # Expected Calibration Error over 10 bins on the top-1 posterior.
    bins = [[] for _ in range(10)]
    for r in scored:
        bins[min(int(r["top_post"] * 10), 9)].append(r)
    ece = sum(
        (len(b) / n) * abs(
            sum(x["top1_correct"] for x in b) / len(b) - sum(x["top_post"] for x in b) / len(b)
        )
        for b in bins if b
    )
    return {
        "n": n,
        "top1_accuracy": round(acc, 4),
        "brier": round(brier, 4),
        "logloss": round(logloss, 4),
        "ece": round(ece, 4),
        "saturated_ge_0.99": saturated,
        "confidently_wrong": conf_wrong,
        "mean_top_post": round(sum(r["top_post"] for r in scored) / n, 4),
    }


def cmd_score(corpus_path: str, out_path: str) -> None:
    corpus = json.loads(Path(corpus_path).read_text())
    records = []
    for case in corpus:
        try:
            records.append(score_case(case))
        except Exception as e:  # noqa: BLE001 — a broken case shouldn't abort the corpus
            records.append({"id": case.get("id"), "error": str(e)[:300], "diag": {}})
    agg = aggregate(records)
    Path(out_path).parent.mkdir(parents=True, exist_ok=True)
    Path(out_path).write_text(json.dumps({"aggregate": agg, "per_case": records}, indent=2))
    print(f"scored {agg.get('n', 0)} cases -> {out_path}")
    print(json.dumps(agg, indent=2))
    errs = [r["id"] for r in records if "error" in r]
    if errs:
        print(f"\n{len(errs)} case(s) errored: {errs}")


def cmd_diff(before_path: str, after_path: str) -> None:
    """THE GATE. Print per-case signed deltas and flag regressions. A fix that
    regresses ANY case fails, regardless of the aggregate."""
    before = {r["id"]: r for r in json.loads(Path(before_path).read_text())["per_case"]}
    after_doc = json.loads(Path(after_path).read_text())
    after = {r["id"]: r for r in after_doc["per_case"]}
    regressions, improvements = [], []
    for cid, a in after.items():
        b = before.get(cid)
        if not b or "error" in a or "error" in b:
            continue
        # correctness regression: was right, now wrong.
        if b["top1_correct"] and not a["top1_correct"]:
            regressions.append(f"  {cid}: CORRECT -> WRONG  (top now {a['top_term']} @ {a['top_post']:.3f})")
        elif not b["top1_correct"] and a["top1_correct"]:
            improvements.append(f"  {cid}: WRONG -> CORRECT  (top now {a['top_term']} @ {a['top_post']:.3f})")
        # newly confidently-wrong.
        if a["confidently_wrong"] and not b["confidently_wrong"]:
            regressions.append(f"  {cid}: newly CONFIDENTLY-WRONG ({a['top_term']} @ {a['top_post']:.3f})")
        # on wrong cases, did confidence climb? (more entropy even if still wrong)
        if not a["top1_correct"] and not b["top1_correct"] and a["top_post"] > b["top_post"] + 0.02:
            regressions.append(f"  {cid}: wrong-and-MORE-confident ({b['top_post']:.3f} -> {a['top_post']:.3f})")
    print("=== CALIBRATION-REGRESSION GATE ===")
    print(f"before: {before_path}\nafter:  {after_path}\n")
    print(f"AGGREGATE after: {json.dumps(after_doc['aggregate'])}\n")
    if improvements:
        print("IMPROVEMENTS:")
        print("\n".join(improvements))
    print(f"\nREGRESSIONS: {len(regressions)}")
    if regressions:
        print("\n".join(regressions))
        print("\n>>> GATE FAILS — at least one case regressed. Do not ship this fix as-is.")
    else:
        print("  (none)\n>>> GATE PASSES — no case regressed on correctness or confident-wrongness.")


def main() -> None:
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    mode = sys.argv[1]
    if mode == "score" and len(sys.argv) == 4:
        cmd_score(sys.argv[2], sys.argv[3])
    elif mode == "diff" and len(sys.argv) == 4:
        cmd_diff(sys.argv[2], sys.argv[3])
    else:
        print(__doc__)
        sys.exit(2)


if __name__ == "__main__":
    main()
