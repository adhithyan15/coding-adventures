# PE case PMC11999957 — three-arm comparison (the moment of truth)

The clean comparison [ADJ55](../../../../ADJ55-provenance-first-corpus.md) §5
flagged as not-yet-run: **plain Claude** vs **the grounded corpus** vs **the
ungrounded invent-LRs framework**, all on the same case, blind to ground truth.

**Case:** 55-year-old man, exertional chest pain / palpitations / dyspnea, HR 80,
no leg edema, ECG ST-elevation mimicking ACS, D-dimer elevated, **Wells score 0**
(low pretest), coronary angiography clean.

**Ground truth:** **PE was present** — CTPA filling defect, DVT confirmed, ACS
excluded. *The pretest score was low but the PE was real.* (Source PMC11999957.)

## Result

| arm | P(PE) | disposition | correct? |
|---|---|---|---|
| **Plain Claude** (no framework) | **3–5%** | "PE is not the answer"; CTPA only *if* RV strain / hypoxia appears | ❌ would likely miss it |
| **Ungrounded framework** (invented LRs, self-disclosed) | **1%** | "PE excluded" | ❌ |
| **Grounded corpus** (every LR → a study) | **0.28 pretest → 0.89 after CTPA** | "can't exclude → image" → CTPA confirms | ✅ |

**Both unconstrained reasoners excluded a real PE. Only the byte-grounded corpus
kept it on the table and caught it.** Plain Claude's reasoning was excellent — it
reframed the case as a cardiac mimic (MINOCA / pericarditis) and explicitly named
the anchoring trap — yet its operative conclusion was still wrong.

## Why the grounded corpus diverged (and was right)

Two grounded numbers, neither a gestalt:

1. **Base rate.** Plain Claude anchored the pretest at ~3–5%. The grounded
   prevalence for a *worked-up suspected-PE* patient is **0.192** (Christopher
   study, 634/3306); even the Wells-"unlikely" subgroup is **12%**. Plain Claude
   under-anchored the floor ~4–5×. The framework cannot, because the prior is a
   published number with a byte-anchored chain.
2. **D-dimer.** All three arms agree the positive D-dimer is a weak rule-in
   (grounded `LR+ = 1.64`, sens 0.97 / spec 0.41). But the two Claude arms used
   that weakness to *dismiss* PE; the grounded math multiplies `0.192 × 1.64 = 0.28`
   and **stays there** — a positive D-dimer can't rule PE *out*. There is no
   narrative in the engine to override that, so the better-fitting pericarditis
   story can't talk it out of imaging.

The framework's edge here is therefore **not only auditability — it produced a
different, correct answer** where frontier reasoning was confidently wrong. The
discipline (published base rate + mechanical LR application + no story to override
the data) is the mechanism.

## Honest caveats

- **n = 1.** One case — possibly one that flatters the framework. The honest
  follow-up is the same three-arm run on a true rule-out (low Wells + *negative*
  D-dimer) and a high-Wells confirm. Until then this is an existence proof, not a
  rate.
- The case prose is terse and contains "Wells 0" + "angiography clean" (mildly
  leading) — but **both Claude arms got identical prose**, so the head-to-head is
  fair; only the grounded arm differs by construction.
- The grounded arm also benefits from controlled-vocabulary ingestion (part of the
  framework, not separable here).
- Plain Claude was not reckless — it said "I can't rule out PE on D-dimer alone" and
  "CTPA promptly if RV strain/hypoxia." Its *headline* (3–5%, comfortable not
  imaging) is what was wrong.

## Artifacts

- Plain-Claude run, verbatim: [`plain-claude.md`](plain-claude.md).
- Grounded corpus eval: `../eval_case.py` over `../../../corpus/pulmonary_embolism/corpus.json`
  (`provenance/pe/eval_case.py case-PMC11999957.json grounded`).
- Ungrounded deriver output + the case record: `../case-results.json`.
