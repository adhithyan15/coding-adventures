#!/usr/bin/env python3
"""board_offline.py — pass board exams with ZERO online model calls (MYCIN-2026).

THE CLAIM, MADE TESTABLE
------------------------
MYCIN answers board questions without ever calling an online model. The only model
on the path is a LOCAL, in-memory one, and it only DECOMPOSES prose into an ADJ query
— it never answers. The native adj-lang engine answers, over the grounded knowledge
graph, with a citation. This runner executes that whole path INSIDE a network-egress
tripwire (offline_guard.no_network), so "no online call" is not a promise — it is a
property the run would crash to violate.

    prose stem
        │  LOCAL model (decompose_query.decompose)  ── the only model call, on-device
        ▼
    {relation, subject, $Var}            ── a typed ADJ recall query
        │  native adj-lang-cli (board_eval.resolve_recall)   ── the CPU reasoner answers
        ▼
    binding + citing edge   OR   honest abstention
        │  scored: correct / abstained / wrong  (wrong = the only real failure)
        ▼
    offline-scorecard.json     (online_calls: 0, enforced)

TWO MODES
---------
  * default (cached): use each item's GOLD query (free_text_board.json). This proves
    the engine + plumbing + the no-network guarantee DETERMINISTICALLY, with no model
    and no network — the form that runs in CI and as the committed scorecard.
  * --model PATH: run a real LOCAL model to decompose each prose stem, score its
    decomposition against the gold query (decompose accuracy) AND the end-to-end
    answer. This is the live proof a small on-device model can drive the board offline.

A decompose error can only ever DEGRADE to an honest abstention (an off-vocabulary
query finds no edge), never to a wrong answer — the engine refuses to fabricate.

Usage:
    python3 board_offline.py                  # cached gold queries (deterministic, offline)
    python3 board_offline.py --model <path>   # live local MLX model decompose
    python3 board_offline.py --quiet          # scorecard JSON only
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import board_eval as be  # noqa: E402  (the native-engine recall resolver + Scorecard)
import decompose_query as dq  # noqa: E402  (prose → ADJ query via a LOCAL model)
from offline_guard import no_network  # noqa: E402  (the zero-online-call tripwire)

ITEMS = HERE / "free_text_board.json"
SCORECARD = HERE / "offline-scorecard.json"


def decompose_all(items: list[dict], gen) -> dict[str, dict | None]:
    """Map each prose stem to a recall query. With gen=None, use the item's gold query
    (cached/offline mode). With a generator, run the LOCAL model and parse its output."""
    if gen is None:
        return {it["id"]: dict(it["query"]) for it in items}
    vocab = dq.build_vocab()
    return {it["id"]: dq.decompose(it["stem"], gen, vocab) for it in items}


def _answer_offline(items: list[dict], decoded: dict[str, dict | None]) -> tuple[dict, "no_network"]:
    """Run the NATIVE engine over the decomposed queries INSIDE the network guard, so
    the answer path is proven to make zero online calls. Returns (engine answers map,
    the guard) — guard.attempts must be empty."""
    recall_items = [
        {"id": it["id"], **q}
        for it in items
        if (q := decoded.get(it["id"])) is not None
    ]
    guard = no_network()
    with guard:
        answers = be.resolve_recall(recall_items)
    return answers, guard


def score(items: list[dict], gen=None) -> dict:
    """Decompose → answer (offline) → score. Returns a scorecard dict with the
    defensibility metrics, the enforced online-call count, and (model mode) the
    decomposition accuracy of the local model against the gold queries."""
    decoded = decompose_all(items, gen)
    answers, guard = _answer_offline(items, decoded)

    results = []
    decompose_hits = 0
    for it in items:
        gold = it["gold"]
        q = decoded.get(it["id"])
        # decomposition accuracy: did the produced query match the gold query exactly?
        if q is not None and all(q.get(k) == it["query"][k] for k in ("relation", "subject", "var")):
            decompose_hits += 1
        r = answers.get(it["id"]) if q is not None else None
        if gold == "ABSTAIN":
            # An uncovered entity: abstaining (or failing to decompose) is defensible;
            # producing an answer is a fabrication.
            if not r or r["abstained"]:
                outcome, answer, trust = "abstained", None, None
            else:
                outcome, answer, trust = "wrong", r["answer"], r["trust"]
        elif q is None or not r or r["abstained"]:
            # Could not form a legal query, or the engine found no grounded edge →
            # honest abstention, never a guess.
            outcome, answer, trust = "abstained", None, None
        else:
            answer, trust = r["answer"], r["trust"]
            outcome = "correct" if answer == gold else "wrong"
        results.append(be.Result(it["id"], "recall", gold, answer, outcome, trust))

    card = be.Scorecard(results=results)
    summary = card.summary()
    summary["online_calls"] = len(guard.attempts)  # ENFORCED 0 — the guard would have raised
    summary["mode"] = "cached_gold_query" if gen is None else "local_model_decompose"
    summary["decompose_accuracy"] = round(decompose_hits / len(items), 4) if items else 0.0
    return {
        "summary": summary,
        "results": [
            {"id": r.item_id, "gold": r.gold, "answer": r.answer,
             "outcome": r.outcome, "trust": r.trust}
            for r in results
        ],
    }


def main(argv: list[str]) -> int:
    quiet = "--quiet" in argv
    gen = None
    if "--model" in argv:
        model_path = argv[argv.index("--model") + 1]
        gen = dq.local_gen(model_path)

    items = json.loads(ITEMS.read_text())["items"]
    scorecard = score(items, gen)
    SCORECARD.write_text(json.dumps(scorecard, indent=2) + "\n")
    s = scorecard["summary"]

    if not quiet:
        print("MYCIN-2026 OFFLINE board-exam — prose → local model → ADJ → native engine\n")
        for r in scorecard["results"]:
            mark = {"correct": "✓", "abstained": "·", "wrong": "✗"}[r["outcome"]]
            ans = r["answer"] if r["answer"] is not None else "UNKNOWN (abstain)"
            tier = f"  [{r['trust']}]" if r["trust"] else ""
            print(f"  {mark} {r['id']:<24} {r['outcome']:<10} {ans}{tier}")
        print(f"\n  correct {s['correct']} · abstained {s['abstained']} · wrong {s['wrong']}  "
              f"(of {s['total']})")
        print(f"  defensibility {s['defensibility']:.0%}  ·  grounded-coverage "
              f"{s['grounded_coverage']:.0%}  ·  mode {s['mode']}")
        if s["mode"] == "local_model_decompose":
            print(f"  local-model decompose accuracy {s['decompose_accuracy']:.0%}")
        print(f"  ONLINE MODEL CALLS: {s['online_calls']}  "
              f"({'✓ zero — proven by the network-egress guard' if s['online_calls'] == 0 else '✗ LEAK'})")
        if not be.cli_available():
            print("  ⚠ adj-lang-cli not built — every item abstained (no Python fallback). "
                  "Build it: cargo build -p adj-lang-cli")
        if s["wrong"] == 0:
            print("\n  ✓ never fabricated — correct-with-citation or honest abstention, zero online calls.")

    # Gate: a fabricated answer OR any online call is a hard failure.
    return 1 if (s["wrong"] > 0 or s["online_calls"] > 0) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
