#!/usr/bin/env python3
"""gen_data.py - the framework authors its own training data for the decomposer.

To train a small model to be an EXPERT at decomposing clinical prose into typed
IR, we need many (prose -> IR) pairs with PERFECT labels. Rather than distill a
teacher's extraction (and inherit its errors), we run the generator BACKWARD:

  1. sample a finding-set from the dictionary (the gold IR, by construction);
  2. ask a teacher model to write NATURAL clinical prose stating exactly those
     findings (varied phrasing, optional non-diagnostic sentence);
  3. the training pair is (decompose-prompt + that prose) -> (the sampled IR).

Because we *chose* the findings, the label is exact - the teacher only supplies
natural language, never the ground truth. We deliberately include:
  - bacterial / viral / mixed finding profiles (coverage of the rulebook),
  - negations (a finding stated as absent/negative),
  - ABSTAIN cases (vignettes with no dictionary findings -> empty IR), so the
    model learns to decline rather than hallucinate (the calibrated-abstention
    we found constrained decoding destroys).

Output: data/train.jsonl + data/valid.jsonl in MLX chat format
(`{"messages":[{"role":"user",...},{"role":"assistant",...}]}`), the SAME
decompose prompt the warm pipeline uses at inference.

Usage:  python3 gen_data.py [--n 300] [--teacher llama3.1:8b] [--seed 0]
"""

from __future__ import annotations

import argparse
import json
import random
import sys
import urllib.request
from pathlib import Path

MYCIN = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(MYCIN / "warm"))
import decompose as decompose_mod  # noqa: E402

DICT = MYCIN / "warm" / "dictionary.json"
OUT = Path(__file__).resolve().parent / "data"
OLLAMA = "http://127.0.0.1:11434/api/generate"

# Finding profiles (functor -> value) that make a realistic clinical picture.
BACTERIAL = [("csf_gram_stain", "positive"), ("csf_neutrophilic_pleocytosis", "high"),
             ("csf_glucose", "low"), ("csf_protein", "high"), ("csf_lactate", "high"),
             ("csf_culture", "positive"), ("serum_procalcitonin", "high"), ("seizure", "present")]
VIRAL = [("csf_lymphocytic_pleocytosis", "high"), ("csf_glucose", "normal"),
         ("csf_lactate", "normal"), ("enteroviral_pcr", "positive"), ("csf_gram_stain", "negative")]
NONSPECIFIC = [("fever", "present"), ("meningismus", "present"), ("fever", "absent"),
               ("meningismus", "absent")]
# ORGANISM-IDENTIFICATION findings (gram morphology + host factors) that the grounded
# organism-id rulebook (G1/G2) reasons over — teaching the decomposer these lets it feed
# the WHICH-organism differential, not just bacterial-vs-viral. Multi-value findings carry
# a VALUE-MATCHED phrasing hint (a random surface could contradict the value, e.g. say
# "elderly" for value=neonate), so the teacher writes prose consistent with the gold label.
ORGANISM_ID = [
    ("csf_gram_morphology", "gram_positive_diplococci", "lancet-shaped gram-positive diplococci on CSF Gram stain"),
    ("csf_gram_morphology", "gram_negative_diplococci", "gram-negative diplococci on CSF Gram stain"),
    ("csf_gram_morphology", "gram_positive_bacilli", "gram-positive bacilli/rods on CSF Gram stain"),
    ("csf_gram_morphology", "gram_negative_coccobacilli", "pleomorphic gram-negative coccobacilli on Gram stain"),
    ("age_band", "neonate", "a neonate / newborn"),
    ("age_band", "older_adult", "an older adult over 50"),
    ("age_band", "infant_child", "a young child"),
    ("immunocompromised", "present", "immunocompromised (on chemotherapy / transplant / HIV)"),
    ("listeria_exposure", "present", "ate unpasteurized soft cheese / deli meats"),
    ("recent_neurosurgery_or_shunt", "present", "recent neurosurgery or a CSF shunt"),
    ("crowding_exposure", "present", "lives in a college dormitory / military barracks"),
    ("petechial_rash", "present", "a petechial / purpuric rash"),
]


def teacher_vignette(teacher: str, findings: list[dict], surfaces: dict,
                     distractors: list[tuple[str, str]] | None = None) -> str:
    distractors = distractors or []
    distractor_ask = ""
    if distractors:
        # Ask the teacher to weave in the incidental detail(s) verbatim-ish, so the
        # gold `discard` span can be located in the prose (teaches set-aside w/ reason).
        ds = "; ".join(f'"{p}"' for p, _ in distractors)
        distractor_ask = (f" Also include, verbatim or nearly so, this incidental "
                          f"non-diagnostic detail: {ds}.")
    if not findings:
        ask = ("Write a realistic 1-2 sentence clinical vignette of a patient being "
               "evaluated for headache/meningitis that states NO specific CSF lab "
               "findings (e.g. only chief complaint / demographics, labs pending)." + distractor_ask)
    else:
        lines = []
        for f in findings:
            sf = surfaces.get(f["functor"], [f["functor"]])
            # A value-matched hint (multi-value findings) takes priority over a random
            # surface, so the teacher's prose can't contradict the gold value.
            hint = f.get("hint") or (random.choice(sf) if sf else f["functor"])
            neg = " (state this as ABSENT/negative)" if f.get("polarity") == "denied" else ""
            lines.append(f'- {f["functor"]} = {f["value"]}  (phrase like: "{hint}"){neg}')
        ask = ("Write a realistic 2-4 sentence clinical vignette for a meningitis workup "
               "that states EXACTLY these findings and no other CSF lab findings. Vary the "
               "wording naturally; you may add ONE non-diagnostic sentence (age, chief "
               "complaint). Do NOT name a diagnosis." + distractor_ask
               + "\n\nFINDINGS:\n" + "\n".join(lines))
    body = json.dumps({"model": teacher, "prompt": ask, "stream": False,
                       "options": {"temperature": 0.7, "num_predict": 220}}).encode()
    req = urllib.request.Request(OLLAMA, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        text = json.loads(r.read())["response"].strip()
    # Drop a leading meta/preamble line ("Here is a vignette:", "Sure,") so the
    # training prose starts with the actual clinical narrative.
    lines = [ln for ln in text.splitlines() if ln.strip()]
    while lines and (lines[0].lower().startswith(("here is", "here's", "sure")) or
                     ("vignette" in lines[0].lower() and lines[0].rstrip().endswith(":"))):
        lines.pop(0)
    return " ".join(lines).strip()


def sample_findings(rng: random.Random) -> list[dict]:
    profile = rng.choices(["bacterial", "viral", "mixed", "organism_id", "abstain"],
                          weights=[28, 25, 15, 17, 15])[0]
    if profile == "abstain":
        return []
    if profile == "organism_id":
        # A which-organism vignette: gram morphology + host factors (+ maybe a CSF lab).
        k = rng.randint(1, 3)
        chosen3 = rng.sample(ORGANISM_ID, k)
        findings = [{"functor": f, "value": v, "hint": h, "polarity": "stated"} for f, v, h in chosen3]
        if rng.random() < 0.5:
            f, v = rng.choice(BACTERIAL + VIRAL)
            findings.append({"functor": f, "value": v, "polarity": "stated"})
        return findings
    pool = (BACTERIAL if profile == "bacterial" else VIRAL if profile == "viral"
            else BACTERIAL + VIRAL)
    k = rng.randint(2, min(5, len(pool)))
    chosen = rng.sample(pool, k)
    # occasionally add a non-specific finding and/or an organism-id host factor.
    if rng.random() < 0.4:
        chosen.append(rng.choice(NONSPECIFIC))
    findings = []
    for functor, value in chosen:
        pol = "denied" if (rng.random() < 0.12 and value in ("positive", "present")) else "stated"
        findings.append({"functor": functor, "value": value, "polarity": pol})
    if rng.random() < 0.3:  # mix a host factor / morphology into a CSF-lab vignette
        f, v, h = rng.choice(ORGANISM_ID)
        findings.append({"functor": f, "value": v, "hint": h, "polarity": "stated"})
    return findings


# Non-diagnostic DISTRACTORS — incidental details a vignette may carry that look
# informative but map to NO controlled-vocabulary finding. We ask the teacher to
# include one, then record it in the gold `discard` (span + reason) so the model learns
# to set it aside with a justification rather than hallucinate a finding from it.
DISTRACTORS = [
    ("blood pressure 128/82 mmHg", "vital sign, not a meningitis CSF/host finding"),
    ("works as a high-school teacher", "social history, not a controlled-vocabulary finding"),
    ("took acetaminophen for the headache", "symptomatic medication, not a diagnostic finding"),
    ("no known drug allergies", "negative allergy history, not a meningitis finding"),
    ("drove himself to the emergency department", "logistical detail, not a clinical finding"),
    ("last ate breakfast around 8 a.m.", "incidental history, not a diagnostic finding"),
]

# NEAR-MISS distractors — the HARD discards. Each phrase superficially reads like a controlled
# FINDING but maps to none: it is the wrong SUBJECT (a relative's illness), a HEDGE (a clinician's
# suspicion/query, not an affirmed result), a PROCESS not a RESULT (a test ordered/sent/pending,
# with no value), or a REFERENCE (a teaching statement, not THIS patient's datum). Coining a
# finding from these is the #1 over-extraction failure of a fine-tuned decomposer, so the gold
# records each (when it lands in the prose) as a justified `discard`. NOTE: a NEGATED finding
# ("no fever", "afebrile") is NOT here — that is a real finding with polarity:denied (the sampler
# handles it), never a discard. These are phrases that map to NO finding in either polarity.
NEAR_MISS_DISTRACTORS = [
    # wrong SUBJECT — someone else's finding is not the patient's.
    ("his father had bacterial meningitis as a child",
     "family history, NOT the patient's finding — wrong subject"),
    ("a sibling with a history of recurrent seizures",
     "family history of seizures, NOT this patient's seizure finding — wrong subject"),
    # HEDGE / suspicion — a query or concern is not an affirmed result.
    ("there is clinical concern for possible meningitis",
     "a clinician's suspicion/hedge, not an affirmed finding — do not coin a result"),
    ("cannot exclude early CSF pleocytosis on these numbers",
     "an explicit uncertainty (cannot exclude), not a confirmed pleocytosis finding"),
    ("query a CNS infection, to be confirmed",
     "a differential question, not a confirmed finding"),
    # PROCESS not RESULT — an order/pending test carries no value to extract.
    ("CSF was sent for Gram stain and culture",
     "a test ORDERED, not a result — no value to coin a finding from"),
    ("blood cultures were drawn and are pending",
     "a pending test, not a resulted finding"),
    ("a lumbar puncture is planned for this afternoon",
     "a planned procedure, not a finding"),
    # REFERENCE / teaching — a general statement is not this patient's datum.
    ("guidelines note that CSF lactate can aid the diagnosis",
     "a reference/teaching statement, not a measured value for this patient"),
    ("textbooks list neck stiffness as a classic meningismus sign",
     "a general teaching point, not an observation of this patient"),
]


def _norm(s: str) -> str:
    """Lowercase + collapse whitespace — for substring matching prose against a phrase
    (mirrors the framework's verify_citation normalization)."""
    return " ".join(s.lower().split())


def find_span(vignette: str, phrases: list[str]) -> str:
    """Return the VERBATIM substring of `vignette` (original case) that supports a
    finding — the first of `phrases` (surface forms / hint / its `/`-split or word
    fragments) that appears in the vignette, normalized. "" if none does. This is the
    byte-provenance: a finding's span must be an actual slice of the source prose."""
    nv = _norm(vignette)
    cands: list[str] = []
    for p in phrases:
        if not p:
            continue
        cands.append(p)
        cands.extend(part.strip() for part in p.split("/"))           # "a / b" alternatives
    vl = vignette.lower()
    # longest first — prefer the most specific phrase that still matches.
    for cand in sorted({c for c in cands if len(c) >= 4}, key=len, reverse=True):
        nc = _norm(cand)
        if nc not in nv:
            continue
        # Map the normalized hit back to a TIGHT original-case slice: anchor on the
        # first word, then extend to the end of the last word (handles whitespace that
        # normalization collapsed, without swallowing trailing punctuation/text).
        words = nc.split()
        lo = vl.find(words[0])
        if lo == -1:
            continue
        end = vl.find(words[-1], lo) + len(words[-1])
        if end <= lo:
            continue
        return vignette[lo:end]
    return ""


def build_gold_ir(vignette: str, findings: list[dict], distractors: list[tuple[str, str]],
                  surfaces: dict) -> dict:
    """Construct the gold IR with BYTE PROVENANCE + DISCARD + INFERENCE justification.

    For each sampled finding, locate its supporting span in the prose: a verbatim span
    → `type:"stated"` + an ENTAILED inference justification; no verbatim span (teacher
    paraphrased past recognition) → `type:"inferred"` + a LEAP justification (which
    ir_to_adj then drops — the safe behavior). Each distractor that appears in the prose
    becomes a `discard` {span, reason}. The gold finding keeps functor/value/polarity
    (what the rulebook consumes) AND adds term/span/type (the provenance the prompt asks
    for) — additive, so downstream ir_to_adj is unaffected."""
    gold_findings, inferences = [], []
    for f in findings:
        functor, value = f["functor"], f["value"]
        phrases = [f.get("hint")] + list(surfaces.get(functor, [])) + [value.replace("_", " ")]
        span = find_span(vignette, phrases)
        denied = f.get("polarity") == "denied"
        ftype = "stated" if span else "inferred"
        term = f"{functor}({value})"
        gold_findings.append({"functor": functor, "value": value, "term": term,
                              "span": span, "type": ftype,
                              "polarity": "denied" if denied else "affirmed"})
        inferences.append({"term": term, "basis_span": span,
                           "verdict": "ENTAILED" if span else "LEAP"})
    discard = []
    for phrase, reason in distractors:
        dspan = find_span(vignette, [phrase])
        if dspan:
            discard.append({"span": dspan, "reason": reason})
    return {"findings": gold_findings, "discard": discard,
            "inference_justifications": inferences}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=300)
    ap.add_argument("--teacher", default="llama3.1:8b")
    ap.add_argument("--seed", type=int, default=0)
    args = ap.parse_args()
    rng = random.Random(args.seed)
    d = json.loads(DICT.read_text())
    surfaces = {f["functor"]: f["surfaces"] for f in d["findings"]}

    OUT.mkdir(exist_ok=True)
    records = []
    for i in range(args.n):
        findings = sample_findings(rng)
        # Inject 0-2 generic distractors AND 0-2 HARD near-miss look-alikes, so a vignette
        # carries red herrings the gold must DISCARD (with a reason) rather than turn into a
        # finding — teaching both "not a finding" and the sharper boundary (a suspicion / a
        # relative's illness / an ordered test / a teaching point is not THIS patient's result).
        distractors = (rng.sample(DISTRACTORS, rng.choice([0, 0, 1, 1, 2]))
                       + rng.sample(NEAR_MISS_DISTRACTORS, rng.choice([0, 1, 1, 2])))
        try:
            vignette = teacher_vignette(args.teacher, findings, surfaces, distractors)
        except Exception as e:  # noqa: BLE001
            print(f"  [{i}] teacher error: {e}", file=sys.stderr)
            continue
        if not vignette or len(vignette) < 20:
            continue
        user = decompose_mod.prompt_for(vignette, d)
        # The gold IR is derived from the prose with BYTE PROVENANCE: build_gold_ir locates
        # each finding's supporting span (ENTAILED) or marks it inferred/LEAP, and records
        # any distractor that landed in the prose as a justified discard. The generation-time
        # `hint` steers the teacher's phrasing but never leaks into the label.
        gold = build_gold_ir(vignette, findings, distractors, surfaces)
        records.append({"messages": [{"role": "user", "content": user},
                                     {"role": "assistant", "content": json.dumps(gold)}]})
        if (i + 1) % 25 == 0:
            print(f"  generated {i + 1}/{args.n}", flush=True)

    rng.shuffle(records)
    n_valid = max(1, len(records) // 10)
    valid, trainset = records[:n_valid], records[n_valid:]
    (OUT / "train.jsonl").write_text("".join(json.dumps(r) + "\n" for r in trainset))
    (OUT / "valid.jsonl").write_text("".join(json.dumps(r) + "\n" for r in valid))
    n_abstain = sum(1 for r in records
                    if json.loads(r["messages"][1]["content"])["findings"] == [])
    print(f"\ngen_data: {len(trainset)} train + {len(valid)} valid examples "
          f"({n_abstain} abstain) -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
