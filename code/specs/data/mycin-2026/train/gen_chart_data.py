#!/usr/bin/env python3
"""gen_chart_data.py — training data for the CHART-FACT decomposer (F3, new IR shape).

REL-14 taught the decomposer the *findings* IR shape (CSF labs / organism-id, with
byte-provenance + discard + inference). This adds the SECOND typed IR shape the system
needs: the **chart-fact IR** — turning a messy free-text patient chart note into the
typed `ChartFact{kind, value, span}` list that the chart-as-constraints COP consumes
(`treatment/antibiotics/chart_to_cop.py`). The structured path already exists
(`fhir/fhir_to_chartfacts.py` maps a FHIR bundle → ChartFacts); this is its PROSE
counterpart, the messy-input front door to the constraint solver (the CC-7 enabler).

Same BACKWARD-GENERATION discipline as gen_data.py (so the label is exact, never a
teacher's fallible extraction):

  1. sample a chart-fact set from the CLOSED chart-fact vocabulary — this IS the gold IR;
  2. a teacher writes a natural chart note stating exactly those facts (+ a non-charting
     distractor or two);
  3. the gold IR is derived from the note with BYTE PROVENANCE — each fact's supporting
     span located verbatim (reusing gen_data.find_span); a distractor that lands in the
     prose is recorded as a justified `discard`.

The closed vocabulary is exactly what `chart_to_cop.compile_cop` maps — so every gold
fact is guaranteed CONSUMABLE by the COP (no unmapped kinds). test_gen_chart_data pins
that F3→F2 contract: feed each sampled (kind, value) through compile_cop and assert it
is NOT discarded.

Output: data/chart_train.jsonl + data/chart_valid.jsonl in MLX chat format.

Usage:  python3 gen_chart_data.py [--n 200] [--teacher llama3.1:8b] [--seed 0]
"""

from __future__ import annotations

import argparse
import json
import random
import sys
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import gen_data  # noqa: E402  (reuse find_span / _norm — one byte-provenance matcher)

OUT = HERE / "data"
OLLAMA = "http://127.0.0.1:11434/api/generate"

# The CLOSED chart-fact vocabulary — kind → list of (value, [surface phrases]). EVERY
# (kind, value) here is mapped by chart_to_cop.compile_cop (verified in the tests), so the
# decomposer can never emit a chart fact the COP would silently drop. Surfaces are the
# natural phrasings a chart note might use; the teacher is steered to one of them.
CHART_PROFILES: dict[str, list[tuple[str, list[str]]]] = {
    "age_band": [
        ("older_adult", ["a 72-year-old", "an elderly patient aged 80", "76-year-old woman"]),
        ("adult", ["a 45-year-old man", "a 38-year-old", "adult patient"]),
    ],
    "immune_status": [
        ("immunocompromised", ["on chemotherapy", "post-transplant on immunosuppression",
                               "HIV with a low CD4 count", "on chronic high-dose steroids"]),
    ],
    "setting": [
        ("post_neurosurgical", ["post-operative day 3 from a craniotomy",
                                "recent neurosurgery", "status post posterior fossa resection"]),
        ("csf_shunt", ["has a ventriculoperitoneal shunt", "a CSF shunt is in place"]),
    ],
    "allergy": [
        ("penicillin", ["anaphylaxis to penicillin", "a severe penicillin allergy"]),
        ("cephalosporin", ["a documented cephalosporin allergy", "hives with cephalosporins"]),
        ("betalactam", ["a severe reaction to all beta-lactams",
                        "an unspecified beta-lactam allergy"]),
    ],
    "renal_status": [
        ("renal_severe", ["an eGFR of 12", "on hemodialysis", "a creatinine of 5.2"]),
        ("renal_moderate", ["an eGFR of 45", "moderate chronic kidney disease"]),
    ],
    "interaction": [
        ("nephrotoxin_interaction", ["also receiving tacrolimus", "concurrent amphotericin B",
                                     "on another nephrotoxic agent"]),
    ],
    "pregnancy": [
        ("present", ["28 weeks pregnant", "G2P1 at 30 weeks gestation", "currently pregnant"]),
    ],
    "weight": [
        ("80", ["weighs 80 kg"]), ("65", ["body weight 65 kg"]), ("90", ["90 kg"]),
    ],
    "culture_resistance": [
        ("ceftriaxone:n_meningitidis", ["CSF isolate resistant to ceftriaxone",
                                        "the meningococcus is ceftriaxone-resistant on sensitivities"]),
    ],
    # --- CC-5 timing inputs (wait-vs-treat-now): culture + clinical status. ---
    "culture_status": [
        ("pending", ["cultures are still pending", "blood cultures sent, results not yet back",
                     "awaiting culture and sensitivities"]),
        ("resulted", ["the culture has resulted", "final culture and sensitivities are back"]),
    ],
    "clinical_status": [
        ("critical", ["in septic shock on vasopressors", "critically ill and intubated in the ICU"]),
        ("unstable", ["hypotensive and tachycardic", "clinically unstable with deteriorating vitals"]),
        ("stable", ["hemodynamically stable", "vital signs are stable and the patient looks well"]),
    ],
    # --- CC-4 objective priority (cost vs side-effect blend). ---
    "objective_priority": [
        ("low_toxicity", ["prioritize minimizing side effects given the patient's frailty",
                          "favor the least toxic regimen"]),
        ("balanced", ["balance cost against side-effect burden"]),
        ("cost", ["choose the most cost-effective regimen"]),
    ],
    # --- CC-6 insurance step-therapy: a payer prerequisite + drugs already tried. ---
    # value is "restricted:prerequisite" (the COP partitions on the colon); the span is the
    # natural phrasing of the policy.
    "step_therapy": [
        ("cefepime:meropenem", ["the payer requires a meropenem trial before approving cefepime",
                                "step therapy: cefepime is restricted until meropenem has been tried"]),
    ],
    "prior_failed": [
        ("ampicillin", ["already failed a course of ampicillin", "ampicillin was tried and failed"]),
        ("meropenem", ["a prior meropenem failure"]),
    ],
}

# Non-charting DISTRACTORS — details a note may carry that map to NO chart-fact kind.
# Recorded in the gold `discard` so the decomposer learns to set them aside with a reason
# rather than coin a spurious ChartFact.
DISTRACTORS = [
    ("drove himself to the emergency department", "logistics, not a chart fact the COP consumes"),
    ("lives at home with his wife", "social history, not a controlled chart-fact kind"),
    ("blood pressure was 132/84 mmHg", "vital sign, no constraint rule maps it"),
    ("has no known sick contacts", "exposure history, not a controlled chart-fact kind"),
    ("prefers to be seen by the morning team", "preference, not a chart fact"),
]


def sample_chart(rng: random.Random) -> list[dict]:
    """Sample a realistic chart-fact set: an age band, plus 1-4 other facts. Returns
    dicts {kind, value, surfaces}. Abstain case (empty) lets the model learn to emit no
    chart facts from a note that states none."""
    if rng.random() < 0.12:
        return []
    facts: list[dict] = []
    # Almost every chart states an age band.
    k = "age_band"
    val, surf = rng.choice(CHART_PROFILES[k])
    facts.append({"kind": k, "value": val, "surfaces": surf})
    others = [kk for kk in CHART_PROFILES if kk != "age_band"]
    for kk in rng.sample(others, rng.randint(1, 4)):
        val, surf = rng.choice(CHART_PROFILES[kk])
        facts.append({"kind": kk, "value": val, "surfaces": surf})
    return facts


def prompt_for_chart(note: str) -> str:
    """The decompose prompt the model is trained on — turn a chart note into the typed
    chart-fact IR over the CLOSED vocabulary, with a span for each fact + a discard list."""
    kinds = ", ".join(sorted(CHART_PROFILES))
    return (
        "You are a clinical chart decomposer. Read the patient chart note and emit ONLY a "
        "JSON object with typed chart facts drawn from this CLOSED vocabulary of kinds: "
        f"{kinds}. For each fact give {{\"kind\", \"value\", \"span\"}} where `span` is the "
        "VERBATIM substring of the note supporting it. Put any detail that does NOT map to a "
        "kind in `discard` as {\"span\", \"reason\"}. Do not invent facts.\n\n"
        f"CHART NOTE:\n{note}\n\nJSON:"
    )


def teacher_chart_note(teacher: str, facts: list[dict], distractors: list[tuple[str, str]]) -> str:
    lines = []
    for f in facts:
        phrase = random.choice(f["surfaces"])
        lines.append(f'- {f["kind"]} = {f["value"]}  (phrase like: "{phrase}")')
    distractor_ask = ""
    if distractors:
        ds = "; ".join(f'"{p}"' for p, _ in distractors)
        distractor_ask = f" Also include, verbatim or nearly so, this non-clinical aside: {ds}."
    if not facts:
        ask = ("Write a realistic 1-2 sentence patient chart note that states NO age, allergy, "
               "renal, pregnancy, immune, setting, weight, or culture-resistance detail (e.g. only "
               "a chief complaint)." + distractor_ask)
    else:
        ask = ("Write a realistic 2-4 sentence patient chart note (ED/admission style) that states "
               "EXACTLY these facts and no other controlled facts. Vary the wording naturally. Do "
               "NOT name a diagnosis or a drug." + distractor_ask + "\n\nFACTS:\n" + "\n".join(lines))
    body = json.dumps({"model": teacher, "prompt": ask, "stream": False,
                       "options": {"temperature": 0.7, "num_predict": 220}}).encode()
    req = urllib.request.Request(OLLAMA, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:  # noqa: S310 (fixed localhost)
        return json.loads(r.read())["response"].strip()


def build_gold_chart_ir(note: str, facts: list[dict],
                        distractors: list[tuple[str, str]]) -> dict:
    """Construct the gold chart-fact IR with BYTE PROVENANCE + DISCARD. Each sampled fact's
    supporting span is located verbatim in the note (reusing gen_data.find_span): a verbatim
    span → type:"stated"; none (teacher paraphrased past recognition) → type:"inferred" with
    an empty span. Each distractor appearing in the note becomes a discard {span, reason}.
    Keeps kind/value (what compile_cop consumes) + adds span/type (the provenance)."""
    gold_facts = []
    for f in facts:
        phrases = list(f.get("surfaces", [])) + [f["value"].replace("_", " ").replace(":", " ")]
        span = gen_data.find_span(note, phrases)
        gold_facts.append({"kind": f["kind"], "value": f["value"],
                           "span": span, "type": "stated" if span else "inferred"})
    discard = []
    for phrase, reason in distractors:
        dspan = gen_data.find_span(note, [phrase])
        if dspan:
            discard.append({"span": dspan, "reason": reason})
    return {"chart_facts": gold_facts, "discard": discard}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=200)
    ap.add_argument("--teacher", default="llama3.1:8b")
    ap.add_argument("--seed", type=int, default=0)
    args = ap.parse_args()
    rng = random.Random(args.seed)

    OUT.mkdir(exist_ok=True)
    records = []
    for i in range(args.n):
        facts = sample_chart(rng)
        distractors = rng.sample(DISTRACTORS, rng.choice([0, 0, 1, 1, 2]))
        try:
            note = teacher_chart_note(args.teacher, facts, distractors)
        except Exception as e:  # noqa: BLE001
            print(f"  [{i}] teacher error: {e}", file=sys.stderr)
            continue
        if not note or len(note) < 20:
            continue
        gold = build_gold_chart_ir(note, facts, distractors)
        records.append({"messages": [{"role": "user", "content": prompt_for_chart(note)},
                                     {"role": "assistant", "content": json.dumps(gold)}]})
        if (i + 1) % 25 == 0:
            print(f"  generated {i + 1}/{args.n}", flush=True)

    rng.shuffle(records)
    n_valid = max(1, len(records) // 10)
    valid, trainset = records[:n_valid], records[n_valid:]
    (OUT / "chart_train.jsonl").write_text("".join(json.dumps(r) + "\n" for r in trainset))
    (OUT / "chart_valid.jsonl").write_text("".join(json.dumps(r) + "\n" for r in valid))
    n_abstain = sum(1 for r in records
                    if json.loads(r["messages"][1]["content"])["chart_facts"] == [])
    print(f"\ngen_chart_data: {len(trainset)} train + {len(valid)} valid chart-decompose "
          f"examples ({n_abstain} abstain) -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
