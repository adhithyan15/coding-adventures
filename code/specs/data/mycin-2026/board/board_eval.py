#!/usr/bin/env python3
"""board_eval.py — the board-style defensibility scoreboard (MYCIN-2026 REL-5).

The north star is to pass medical board exams, built organ-by-organ. This harness
is the scoreboard that makes progress measurable: it runs a bank of board-style
fact-recall questions end-to-end over the grounded knowledge graph and scores each
into one of THREE outcomes — not one:

    correct   — answered, matches the key, WITH a proof (the citing edge)
    abstained — returned UNKNOWN because no grounded edge supports an answer
                (the honest failure — a feature, the discriminator vs a
                hallucinating recall model)
    wrong     — answered, but incorrectly (the ONLY real failure)

The headline is the **defensibility curve**: on the covered subset, near-100%
correct-with-proof; on the uncovered subset, abstain rather than fabricate. The
harness GATES on never-fabricate — a single `wrong` is a non-zero exit.

It also reports **grounding coverage**: of the correct answers, how many cite an
`authoritative` (spider-grounded) edge vs a `consensus` (authored-debt) edge. That
is the live number every grounding PR moves — today the IEM edges are consensus
(authored-debt), so grounded-coverage is 0%; after the REL-4 spider runs it climbs.

Recall is scored against the same grounded graph the native engine uses, via the
REL-1 `RelationStore` (pure Python, deterministic, 0 model calls) — so the
scoreboard runs anywhere without a build. (Differential items slot into the same
schema once the harness drives the native CLI for ranked hypotheses.)

Usage:  python3 board_eval.py            # score the item bank, print + emit scorecard
        python3 board_eval.py --quiet    # scorecard JSON only
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
RECALL = HERE.parent / "recall"
ITEMS = HERE / "items.json"
SCORECARD = HERE / "board-scorecard.json"
sys.path.insert(0, str(RECALL))
import recall  # noqa: E402  (the REL-1 RelationStore + parse_edges)


@dataclass
class Result:
    item_id: str
    tactic: str
    gold: str
    answer: str | None          # the bound value, or None on abstention
    outcome: str                # correct | abstained | wrong
    trust: str | None           # the citing edge's trust tier (grounding signal)


@dataclass
class Scorecard:
    results: list[Result] = field(default_factory=list)

    def _count(self, outcome: str) -> int:
        return sum(1 for r in self.results if r.outcome == outcome)

    def summary(self) -> dict:
        total = len(self.results)
        correct = self._count("correct")
        abstained = self._count("abstained")
        wrong = self._count("wrong")
        attempted = correct + wrong
        # Of the correct answers, how many rest on a spider-grounded edge.
        grounded = sum(1 for r in self.results if r.outcome == "correct" and r.trust == "authoritative")
        return {
            "total": total,
            "correct": correct,
            "abstained": abstained,
            "wrong": wrong,
            # Defensible = answered-right OR honestly-abstained; never fabricated.
            "defensibility": round((correct + abstained) / total, 4) if total else 0.0,
            # Of what it chose to answer, how often was it right (fabrication rate = 1 - this).
            "accuracy_on_attempted": round(correct / attempted, 4) if attempted else None,
            # The live number a grounding PR moves: correct answers citing an
            # authoritative (spider-grounded) edge, over all correct answers.
            "grounded_coverage": round(grounded / correct, 4) if correct else 0.0,
            "grounded_correct": grounded,
        }


def score(items: list[dict], store: "recall.RelationStore") -> Scorecard:
    card = Scorecard()
    for it in items:
        if it.get("tactic") != "recall":
            # Differential/management tactics are not scored by this slice; record
            # them as abstained-from-scoring rather than silently dropping (no
            # silent caps — the count stays honest).
            card.results.append(Result(it["id"], it.get("tactic", "?"), it.get("gold", ""),
                                       None, "abstained", None))
            continue
        var = "$" + it["var"]
        hits = store.query(it["relation"], [it["subject"], var])
        gold = it["gold"]
        if gold == "ABSTAIN":
            if not hits:
                card.results.append(Result(it["id"], "recall", gold, None, "abstained", None))
            else:
                # Produced an answer for a disease that should be uncovered: fabrication.
                ans = hits[0].bindings.get(var)
                card.results.append(Result(it["id"], "recall", gold, ans, "wrong", hits[0].proof.trust))
            continue
        if not hits:
            # No grounded edge → honest abstention even though an answer exists in
            # the world; counts as defensible, not as a wrong answer.
            card.results.append(Result(it["id"], "recall", gold, None, "abstained", None))
            continue
        ans = hits[0].bindings.get(var)
        outcome = "correct" if ans == gold else "wrong"
        card.results.append(Result(it["id"], "recall", gold, ans, outcome, hits[0].proof.trust))
    return card


def main(argv: list[str]) -> int:
    quiet = "--quiet" in argv
    items = json.loads(ITEMS.read_text())["items"]
    store = recall.parse_edges(RECALL / "iem-edges.adj")
    card = score(items, store)
    summary = card.summary()

    scorecard = {
        "summary": summary,
        "results": [
            {"id": r.item_id, "tactic": r.tactic, "gold": r.gold,
             "answer": r.answer, "outcome": r.outcome, "trust": r.trust}
            for r in card.results
        ],
    }
    SCORECARD.write_text(json.dumps(scorecard, indent=2) + "\n")

    if not quiet:
        print("MYCIN-2026 board-eval — defensibility scoreboard\n")
        for r in card.results:
            mark = {"correct": "✓", "abstained": "·", "wrong": "✗"}[r.outcome]
            ans = r.answer if r.answer is not None else "UNKNOWN (abstain)"
            tier = f"  [{r.trust}]" if r.trust else ""
            print(f"  {mark} {r.item_id:<22} {r.outcome:<10} {ans}{tier}")
        s = summary
        print(f"\n  correct {s['correct']} · abstained {s['abstained']} · wrong {s['wrong']}  "
              f"(of {s['total']})")
        print(f"  defensibility {s['defensibility']:.0%}  ·  accuracy-on-attempted "
              f"{('n/a' if s['accuracy_on_attempted'] is None else format(s['accuracy_on_attempted'], '.0%'))}"
              f"  ·  grounded-coverage {s['grounded_coverage']:.0%} "
              f"({s['grounded_correct']}/{s['correct']} correct answers cite a spider-grounded edge)")
        if s["wrong"] == 0:
            print("\n  ✓ never fabricated — every answer is correct-with-proof or an honest abstention.")

    # Gate: a fabricated answer (wrong) is the only hard failure.
    return 1 if summary["wrong"] > 0 else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
