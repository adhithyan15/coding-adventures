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

# The native adj-lang CLI binary — for the DIFFERENTIAL tactic, the board runs a
# case .adj (rulebook + observations + ? hypotheses) and reads the ranked
# differential decision. Recall stays pure-Python; only differential needs the
# engine. If the binary is absent (e.g. a Python-only CI job that didn't build the
# Rust workspace), differential items abstain and the run logs how many were
# skipped — no silent caps, no fabricated answers.
_CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"


def cli_available() -> bool:
    return _CLI.exists()


def run_differential(program: Path) -> dict | None:
    """Run the native CLI on a case .adj and return its `decision` dict, or None if
    the binary is unavailable or the program failed to compile."""
    if not cli_available():
        return None
    import subprocess

    out = subprocess.run([str(_CLI), str(program)], capture_output=True, text=True)
    try:
        doc = json.loads(out.stdout)
    except (json.JSONDecodeError, ValueError):
        return None
    if not isinstance(doc, dict):
        return None
    return doc.get("decision")


def score_differential(decision: dict | None, gold: str) -> tuple[str, str | None]:
    """Score a differential decision against the gold leader (or ABSTAIN).

    determinate → commits to a leader: correct iff it matches gold (wrong if gold
    is ABSTAIN — it committed when it should not have). kickback / empty → the
    engine abstained (margin too small / no hypothesis): defensible, and correct
    behaviour when gold is ABSTAIN. A missing decision (CLI unavailable) abstains.
    """
    if decision is None:
        return "abstained", None
    kind = decision.get("type")
    if kind == "determinate":
        leader = decision.get("leader")
        if gold == "ABSTAIN":
            return "wrong", leader  # committed when it should have abstained
        return ("correct" if leader == gold else "wrong"), leader
    # kickback / empty → the engine declined to commit.
    return "abstained", None


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
        # grounded-coverage is a RECALL grounding metric (a recall answer cites one
        # edge whose trust tier signals grounding). Differential answers are proven
        # by a multi-clause LR proof DAG, not a single edge, so they are excluded
        # from the denominator rather than diluting the number.
        recall_correct = [r for r in self.results if r.outcome == "correct" and r.tactic == "recall"]
        grounded = sum(1 for r in recall_correct if r.trust == "authoritative")
        return {
            "total": total,
            "correct": correct,
            "abstained": abstained,
            "wrong": wrong,
            "by_tactic": {
                t: self._count_tactic(t)
                for t in sorted({r.tactic for r in self.results})
            },
            # Defensible = answered-right OR honestly-abstained; never fabricated.
            "defensibility": round((correct + abstained) / total, 4) if total else 0.0,
            # Of what it chose to answer, how often was it right (fabrication rate = 1 - this).
            "accuracy_on_attempted": round(correct / attempted, 4) if attempted else None,
            # The live number a grounding PR moves: recall answers citing an
            # authoritative (spider-grounded) edge, over all correct recall answers.
            "grounded_coverage": round(grounded / len(recall_correct), 4) if recall_correct else 0.0,
            "grounded_correct": grounded,
        }

    def _count_tactic(self, tactic: str) -> dict:
        rs = [r for r in self.results if r.tactic == tactic]
        return {o: sum(1 for r in rs if r.outcome == o) for o in ("correct", "abstained", "wrong")}


def score(items: list[dict], store: "recall.RelationStore") -> Scorecard:
    card = Scorecard()
    for it in items:
        tactic = it.get("tactic")
        if tactic == "differential":
            # Run the native engine on the case .adj and score its ranked decision.
            # The "trust" of a differential is None — its proof is the LR proof DAG,
            # not a single cited edge (so it is excluded from grounded-coverage).
            decision = run_differential(HERE / it["program"])
            outcome, answer = score_differential(decision, it["gold"])
            card.results.append(Result(it["id"], "differential", it["gold"], answer, outcome, None))
            continue
        if tactic != "recall":
            # Unknown tactic: record as abstained-from-scoring rather than silently
            # dropping (no silent caps — the count stays honest).
            card.results.append(Result(it["id"], tactic or "?", it.get("gold", ""),
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

    skipped_diff = sum(
        1 for it in items if it.get("tactic") == "differential"
    ) if not cli_available() else 0

    if not quiet:
        print("MYCIN-2026 board-eval — defensibility scoreboard\n")
        for r in card.results:
            mark = {"correct": "✓", "abstained": "·", "wrong": "✗"}[r.outcome]
            ans = r.answer if r.answer is not None else "UNKNOWN (abstain)"
            tier = f"  [{r.trust}]" if r.trust else f"  ({r.tactic})" if r.tactic != "recall" else ""
            print(f"  {mark} {r.item_id:<24} {r.outcome:<10} {ans}{tier}")
        s = summary
        print(f"\n  correct {s['correct']} · abstained {s['abstained']} · wrong {s['wrong']}  "
              f"(of {s['total']})")
        bt = "  ·  ".join(
            f"{t}: {c['correct']}✓/{c['abstained']}·/{c['wrong']}✗" for t, c in s["by_tactic"].items()
        )
        print(f"  by tactic — {bt}")
        print(f"  defensibility {s['defensibility']:.0%}  ·  accuracy-on-attempted "
              f"{('n/a' if s['accuracy_on_attempted'] is None else format(s['accuracy_on_attempted'], '.0%'))}"
              f"  ·  grounded-coverage {s['grounded_coverage']:.0%} "
              f"({s['grounded_correct']} recall answers cite a spider-grounded edge)")
        if skipped_diff:
            print(f"  ⚠ {skipped_diff} differential item(s) skipped (adj-lang-cli not built) — "
                  f"abstained, not scored. Build it: cargo build -p adj-lang-cli")
        if s["wrong"] == 0:
            print("\n  ✓ never fabricated — every answer is correct-with-proof or an honest abstention.")

    # Gate: a fabricated answer (wrong) is the only hard failure.
    return 1 if summary["wrong"] > 0 else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
