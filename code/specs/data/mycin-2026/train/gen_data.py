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


def teacher_vignette(teacher: str, findings: list[dict], surfaces: dict) -> str:
    if not findings:
        ask = ("Write a realistic 1-2 sentence clinical vignette of a patient being "
               "evaluated for headache/meningitis that states NO specific CSF lab "
               "findings (e.g. only chief complaint / demographics, labs pending).")
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
               "complaint). Do NOT name a diagnosis.\n\nFINDINGS:\n" + "\n".join(lines))
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
        try:
            vignette = teacher_vignette(args.teacher, findings, surfaces)
        except Exception as e:  # noqa: BLE001
            print(f"  [{i}] teacher error: {e}", file=sys.stderr)
            continue
        if not vignette or len(vignette) < 20:
            continue
        user = decompose_mod.prompt_for(vignette, d)
        # The gold IR carries only the typed fields — the generation-time `hint` (used to
        # steer the teacher's phrasing) must NOT leak into the label the model learns.
        gold_findings = [{"functor": f["functor"], "value": f["value"], "polarity": f["polarity"]}
                         for f in findings]
        gold = {"findings": gold_findings, "discard": [], "inference_justifications": []}
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
