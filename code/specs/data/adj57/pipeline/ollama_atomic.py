#!/usr/bin/env python3
"""ADJ67 — atomic feed of a decomposed case to a LOCAL (weak) model via Ollama.

The framework already decomposed the case. We never hand the weak model the raw paragraph
(with its priming/framing). Instead we feed it ONE atomic fact at a time, ask only a tiny
local judgment — does this single observation argue for/against each candidate cause — and
the HARNESS aggregates those local verdicts (sensitivity.py) and makes the call. The weak
model contributes local matching; the framework holds the global structure.

Two knobs let you reproduce the model-ladder findings:
  --grounded   prepend the rulebook's forensic criteria to each atom, so the model MATCHES
               against a cited rule instead of recalling from its (weak) memory. Lifted a
               local ~Gemma-class model from a wrong answer to the correct one.
  --model arg  any Ollama model. Below a capability floor (e.g. a 0.5B model) the per-atom
               verdicts become DEGENERATE (no discrimination); the discrimination gate below
               detects this and refuses to launder garbage into a confident decision.

Usage: python ollama_atomic.py <ollama-model> [--grounded]
Requires a running Ollama with the model pulled.
"""
from __future__ import annotations

import json
import re
import statistics
import sys
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import sensitivity as S  # noqa: E402

HYP = ["white shark bite (predation)", "boat propeller strike", "fishing-gear entanglement"]
# the framework's decomposition of the seal case (neutral phrasing — no framing).
FACTS = [
    "Three separate cuts that are roughly parallel to one another, of similar length, spaced at fairly regular intervals.",
    "The cut edges are clean and relatively smooth, each following a shallow even curve.",
    "Little tissue loss between the cuts; no flaps of skin are torn back from the wound margins.",
    "The wounds are open but not deep - they expose blubber but do not reach the body cavity.",
    "No bite injuries to the hind flippers or other limbs; the injuries are confined to the flank and back.",
    "The surrounding waters are a place where white sharks are known to feed on seals at this time of year.",
    "Recreational and fishing vessels are active close to shore in these waters.",
]
# the rulebook criteria (a stand-in for ADJ66 spider output) used only with --grounded.
CRITERIA = ("Reference (forensic wound criteria): propeller strikes = parallel, evenly-spaced, "
            "CLEAN SMOOTH cuts with LITTLE tissue loss and no skin flaps. Shark bites = a curved arc "
            "of JAGGED tooth punctures with TORN skin flaps and tissue removed. Net entanglement = "
            "ENCIRCLING constriction or abrasion marks, often around neck or flippers.\n")
TO_DB = 5  # rating in [-2,+2] -> decibans


def rate(model: str, fact: str, grounded: bool) -> dict:
    pre = CRITERIA if grounded else ""
    prompt = (f"{pre}Observation about an injured seal's wounds: {fact} "
              "Causes: A=shark bite, B=boat propeller, C=fishing net. "
              "Rate each from -2 (against) to +2 (for) based only on this one observation. "
              "Reply ONLY: A=<n> B=<n> C=<n>")
    body = json.dumps({"model": model, "messages": [{"role": "user", "content": prompt}],
                       "stream": False, "options": {"temperature": 0, "num_predict": 800}}).encode()
    req = urllib.request.Request("http://localhost:11434/api/chat", data=body,
                                 headers={"Content-Type": "application/json"})
    txt = json.loads(urllib.request.urlopen(req, timeout=180).read())["message"]["content"]

    def g(letter: str) -> int:
        m = re.search(rf"{letter}\s*=\s*(-?\+?\d)", txt)
        return max(-2, min(2, int(m.group(1)))) if m else 0
    return {"A": g("A"), "B": g("B"), "C": g("C"), "raw": txt.strip()[:50]}


def discrimination_gate(rows: list[dict]) -> dict:
    """The lesson from the 0.5B run: a too-weak model emits near-constant, non-discriminating
    verdicts, and the engine will launder them into a confident WRONG answer. Detect it: if
    the per-atom ratings have ~zero spread (the model never really distinguishes the causes),
    the model is BELOW FLOOR and its aggregate must not be trusted."""
    allvals = [r[k] for r in rows for k in ("A", "B", "C")]
    spread = statistics.pstdev(allvals) if allvals else 0.0
    n_positive = sum(1 for v in allvals if v > 0)
    degenerate = spread < 0.6 or n_positive == 0
    return {"spread": round(spread, 2), "n_positive": n_positive, "below_floor": degenerate}


def main() -> None:
    model = sys.argv[1] if len(sys.argv) > 1 else "gemma4:latest"
    grounded = "--grounded" in sys.argv

    rows, evidence = [], []
    for i, f in enumerate(FACTS):
        r = rate(model, f, grounded)
        rows.append(r)
        print(f"  atom{i+1}: A(shark)={r['A']:+d} B(prop)={r['B']:+d} C(net)={r['C']:+d}")
        evidence.append({"name": f"atom_{i+1}", "source": "weak-model-local-verdict",
                         "weights": {HYP[0]: r["A"] * TO_DB, HYP[1]: r["B"] * TO_DB, HYP[2]: r["C"] * TO_DB}})

    gate = discrimination_gate(rows)
    res = S.assess(HYP, evidence)
    print(f"\n  model={model}  grounded={grounded}")
    print(f"  discrimination gate: spread={gate['spread']}  below_floor={gate['below_floor']}")
    if gate["below_floor"]:
        print("  >>> BELOW FLOOR — verdicts are non-discriminating; the harness REFUSES to report a "
              "confident decision (this model is too weak for the local-matching task).")
        sys.exit(0)
    print(f"  HARNESS DECISION: {res['decision']}  (margin {res['margin_db']:+.0f} dB)")
    for row in res["ranked"]:
        print(f"     {row['hypothesis'][:30]:30s} {res['posteriors'][row['hypothesis']]*100:5.1f}%")


if __name__ == "__main__":
    main()
