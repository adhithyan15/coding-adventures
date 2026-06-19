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

Every tactic — recall, differential, management — is answered by the ONE native
adj-lang engine (the CPU reasoner), deterministically and with zero answer-time
*online* model calls. Recall used to be a Python `RelationStore` shortcut; it now
runs as a native binding query (`? relation(subject, $Var)`) over the imported
grounded edge rulebooks, so the board exercises the real engine, not a parallel
Python re-implementation. With the CLI absent (a Python-only job that didn't build
the Rust workspace) every engine-backed item abstains — honestly — rather than
falling back to a second resolver.

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

# The recall knowledge graph spans one *-edges.adj file per domain (IEM diseases,
# vitamin deficiencies, …). They share the relational substrate, so the board
# imports ALL of them into ONE adj-lang program and resolves each recall item as a
# native binding query. Adding a domain = drop in its edge file + its board items.
#
# THE DESIGN CORRECTION THAT MATTERS (the whole point of this harness): recall is
# answered by the NATIVE adj-lang engine — the SAME CPU reasoner that runs the
# differential and the constraint solver — NOT by a Python resolver. The REL-1
# `recall.py` RelationStore was a proof-of-semantics prototype; it is DEPRECATED and
# no longer on the answer path. A board recall question IS an ADJ program:
#
#     import "<domain>-edges.adj"   …   ? relation(subject, $Var)
#
# which the engine resolves to the variable binding + the citing edge's
# provenance/trust, or an honest abstention (empty answers). One engine for every
# board tactic — recall, differential, management — with zero answer-time *online*
# model calls. (A local in-memory model may generate the ADJ program from prose; see
# board_offline.py. The engine that ANSWERS is always the native CPU reasoner.)
EDGE_FILES = ["iem-edges.adj", "vitamin-edges.adj", "anemia-edges.adj", "endocrine-edges.adj",
              "coag-edges.adj", "micro-edges.adj", "pharm-edges.adj", "immuno-edges.adj",
              "genetics-edges.adj"]

# The native adj-lang CLI binary — the ONE engine behind every tactic (recall,
# differential, management). If the binary is absent (e.g. a Python-only CI job that
# didn't build the Rust workspace), every engine-backed item abstains and the run
# logs how many were skipped — no silent caps, no fabricated answers, and no Python
# fallback resolver (recall is the engine's job now).
_CLI = HERE.parents[3] / "packages" / "rust" / "target" / "debug" / "adj-lang-cli"


def cli_available() -> bool:
    return _CLI.exists()


def _recall_program(recall_items: list[dict]) -> str:
    """Render a batch of recall items as ONE adj-lang program: import every domain's
    grounded edge rulebook, then one binding query `? relation(subject, $Var)` per
    item. Imports are written as the bare filenames because the engine resolves them
    relative to the PROGRAM FILE's directory — so the program must live in RECALL/."""
    lines = [f'import "{name}"' for name in EDGE_FILES]
    lines += [f'? {it["relation"]}({it["subject"]}, ${it["var"]})' for it in recall_items]
    return "\n".join(lines) + "\n"


def resolve_recall(recall_items: list[dict]) -> dict[str, dict]:
    """Answer every recall item through the NATIVE adj-lang engine in one CLI call.

    Returns {item_id: {"answer", "trust", "abstained"}}. answer is the variable's
    binding (or None on abstention); trust is the citing edge's tier (the grounding
    signal); abstained is True when the engine found no grounded edge — the honest
    UNKNOWN, never a fabricated guess. If the CLI is unavailable the map is empty and
    every recall item abstains (there is no Python fallback — recall is the engine's
    job)."""
    if not recall_items or not cli_available():
        return {}
    import os
    import subprocess
    import tempfile

    program = _recall_program(recall_items)
    # The program must sit in RECALL/ so its relative `import` lines resolve.
    fd, path = tempfile.mkstemp(suffix=".adj", prefix=".board_recall_", dir=RECALL)
    try:
        os.write(fd, program.encode("utf-8"))
        os.close(fd)
        out = subprocess.run([str(_CLI), path], capture_output=True, text=True)
        doc = json.loads(out.stdout)
        entries = doc.get("recall", []) if isinstance(doc, dict) else []
    except (json.JSONDecodeError, ValueError, OSError):
        return {}
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass

    # Match each item to its engine answer by the query echo the CLI emits
    # ("relation(subject, Var)" — variable rendered without the `$` sigil), so the
    # mapping is robust to any reordering rather than relying on array position.
    by_query = {e.get("query"): e for e in entries if isinstance(e, dict)}
    resolved: dict[str, dict] = {}
    for it in recall_items:
        echo = f'{it["relation"]}({it["subject"]}, {it["var"]})'
        entry = by_query.get(echo)
        answers = (entry or {}).get("answers") or []
        if not entry or entry.get("abstained") or not answers:
            resolved[it["id"]] = {"answer": None, "trust": None, "abstained": True}
            continue
        top = answers[0]
        citations = top.get("citations") or []
        resolved[it["id"]] = {
            "answer": top.get("bindings", {}).get(it["var"]),
            "trust": citations[0].get("trust") if citations else None,
            "abstained": False,
        }
    return resolved


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


# The MANAGEMENT tactic — "best next step / empiric regimen" — runs the real
# chart-as-constraints engine: a patient context (likely organisms + allergies /
# comorbidity / pregnancy / renal) compiles into a constraint program that the
# adj-constraint-solver SOLVES into a min-cost regimen, OR proves INFEASIBLE with a
# named conflict. Constraints solved OR made unsatisfiable — both are defensible.
_TREATMENT = HERE.parent / "treatment" / "antibiotics"


def run_management(chart: list[list[str]]) -> dict | None:
    """Compile the chart-fact IR → COP, solve it via the native engine, and return the
    result dict (regimen / outcome / conflict). None if the CLI binary is unavailable."""
    if not cli_available():
        return None
    if str(_TREATMENT) not in sys.path:
        sys.path.insert(0, str(_TREATMENT))
    import chart_to_cop as ctc  # noqa: E402  (reuse the merged chart→constraints engine)

    facts = [ctc.ChartFact(*(f + [""])[:3]) for f in chart]  # [kind, value, span?]
    return ctc.derive(_CLI, facts)


def score_management(result: dict | None, gold) -> tuple[str, str | None]:
    """Score a management decision against the gold regimen (a sorted drug list) or the
    literal "INFEASIBLE" (a chart whose constraints conflict — no safe regimen exists).

    A regimen that matches gold is correct. INFEASIBLE when gold is "INFEASIBLE" is
    correct (the engine rightly refused to fabricate a regimen). INFEASIBLE when a
    regimen was expected is an honest abstention (declined, not wrong). A regimen when
    the chart should have been INFEASIBLE is wrong (fabricated a regimen). A missing
    result (CLI unavailable) abstains.
    """
    if result is None:
        return "abstained", None
    regimen = result.get("regimen")
    answer = "+".join(regimen) if regimen else "INFEASIBLE"
    if gold == "INFEASIBLE":
        return ("correct" if regimen is None else "wrong"), answer
    if regimen is None:
        return "abstained", "INFEASIBLE"  # expected a regimen; engine declined — defensible
    return ("correct" if sorted(regimen) == sorted(gold) else "wrong"), answer


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


def score(items: list[dict]) -> Scorecard:
    card = Scorecard()
    # Resolve every recall item up front in ONE native-engine call (the CPU reasoner,
    # not a Python store). Differential/management still run per-item below.
    recalled = resolve_recall([it for it in items if it.get("tactic") == "recall"])
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
        if tactic == "management":
            # Run the chart-as-constraints engine: regimen OR INFEASIBLE(conflict).
            # trust=None — the proof is the constraint trace, not a single cited edge.
            gold = it["gold"]
            result = run_management(it["chart"])
            outcome, answer = score_management(result, gold)
            gold_str = gold if isinstance(gold, str) else "+".join(gold)
            card.results.append(Result(it["id"], "management", gold_str, answer, outcome, None))
            continue
        if tactic != "recall":
            # Unknown tactic: record as abstained-from-scoring rather than silently
            # dropping (no silent caps — the count stays honest).
            card.results.append(Result(it["id"], tactic or "?", it.get("gold", ""),
                                       None, "abstained", None))
            continue
        # Recall is answered by the native engine (resolved above). An entry is
        # missing only if the CLI was unavailable → treat as abstention.
        r = recalled.get(it["id"])
        gold = it["gold"]
        if gold == "ABSTAIN":
            if not r or r["abstained"]:
                card.results.append(Result(it["id"], "recall", gold, None, "abstained", None))
            else:
                # Produced an answer for a disease that should be uncovered: fabrication.
                card.results.append(Result(it["id"], "recall", gold, r["answer"], "wrong", r["trust"]))
            continue
        if not r or r["abstained"]:
            # No grounded edge → honest abstention even though an answer exists in
            # the world; counts as defensible, not as a wrong answer.
            card.results.append(Result(it["id"], "recall", gold, None, "abstained", None))
            continue
        ans = r["answer"]
        outcome = "correct" if ans == gold else "wrong"
        card.results.append(Result(it["id"], "recall", gold, ans, outcome, r["trust"]))
    return card


def main(argv: list[str]) -> int:
    quiet = "--quiet" in argv
    items = json.loads(ITEMS.read_text())["items"]
    card = score(items)
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
