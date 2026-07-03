# ADJ59 — Cross-Domain Validation + the Qualitative Verdict

> **Status (2026-06-04):** The byte-provenance framework run head-to-head against
> plain Claude with a blind judge across **six non-medical domains**. A small fix —
> the **qualitative verdict** — flipped the framework from losing 0–3 to winning the
> head-to-head. Two honest weaknesses remain; the second is the missing half of the
> invariant (output grounding). Implementation:
> [`code/specs/data/adj57/pipeline/`](data/adj57/pipeline/). Builds on
> [ADJ58](ADJ58-universal-stage-contract.md).

## 1. Why

The framework's value in medicine was *both* correctness (the Wells-0 PE case) and
auditability. Does it generalize? We tested it on six non-medical domains —
engineering failure analysis, astronomy, cybersecurity, geology, paleontology,
linguistics — running each real case through the full pipeline **and** plain Claude
(no framework), with a blind judge scoring both against held-aside ground truth.

## 2. The first run failed honestly — and showed the fix

On the first three domains the framework lost **0–3**. The cause was sharp: outside
medicine there is **no published likelihood-ratio literature** (fatigue fractography,
supernova spectra, malware indicators are not quantified the way epidemiology is), so
the spider honestly returned `direction_only` and the old `run.py` policy
**abstained** ("no defensible posterior") — *even though the `derive` stage had named
the correct hypothesis every time.* The judge rightly penalized an "auditable
non-answer." The framework was conflating **"I can't ground a *number*"** with
**"I can't reach an *answer*."**

**The qualitative verdict** ([`run.py`](data/adj57/pipeline/run.py)): report the
strongest defensible conclusion available — a quantitative posterior where a grounded
prior + root-grounded LRs exist, *otherwise* commit to the `derive`-stage leading
answer with its byte-provenanced evidential basis (each used fact → its role, each
traceable to case bytes). The byte-provenance invariant holds either way; we stop
requiring a *number* to count as an answer.

## 3. Result (blind judge, ground-truth leak fixed — see §5)

| domain | framework | plain Claude | winner |
|---|---|---|---|
| engineering | correct | correct | tie |
| astronomy | correct | partial (under-committed) | **framework** |
| cybersecurity | correct | correct (overconfident) | **framework** |
| geology | correct | partial | **framework** |
| paleontology | correct | **incorrect** (confident wrong clade) | **framework** |
| linguistics | correct | correct (+ more complete) | plain Claude |

**Framework: 4 wins, 1 tie, 1 loss — correct in all 6.** Plain Claude: 3 correct,
2 partial, 1 wrong. The framework won by *committing to its feature-derived answer
with honest calibration*, beating plain Claude's three failure modes: **confidently
wrong** (paleontology's locality trap), **under-committed** (astronomy IIb/IIP), and
**overconfident** (cybersecurity "High" vs the source's medium confidence).

## 4. Two honest weaknesses (next fixes)

### 4.1 Over-specification beyond the byte evidence — the missing half of the invariant
In geology the framework answered `tremolitized_diopside_anorthosite`, but
"tremolitized" / "diopside" **are not in the case bytes** (the scenario had no
FTIR/Raman) — the `derive` agent claimed more specificity than the evidence supports,
smuggled from training. **The validator did not catch it**, because ADJ57/58 enforce
only *input coverage* (every input byte used-or-discarded). They do **not** enforce
*output grounding* (every output claim traces to an input byte). **The byte-provenance
invariant is bidirectional — nothing dropped AND nothing invented — and we have only
the first half.** This is the next build (§6).

### 4.2 The report format undermines its own auditability
In engineering and linguistics the judge found the *correct* answer "buried in
byte-accounting machinery / truncated evidence strings" — sometimes harder to audit
than plain prose. The audit trail should *back* an answer-first report, not bury it.

## 5. A methodology bug we caught mid-experiment
`run.py` printed a `ground truth (held aside)` line for operator inspection, and
`make_judge.py` swept the whole stdout into the framework's report — **leaking ground
truth to the blind judge.** The geology judge flagged it ("reads as possible GT
leakage"); we stripped the line and re-judged both rounds. §3 is the post-fix result.
(Fitting for a project about accounting for every byte: the contamination was a byte
that should not have been there.)

## 6. The next iteration — the output-grounding gate (the dual invariant)

> If a claim is not grounded in bytes (e.g. recalled from training), the validator
> must flag it, reject it, and kick it back to re-derive.

ADJ57/58 gate **input coverage**. ADJ60 should add the dual gate: **output grounding**
— decompose each stage's *output* into its atomic claims, and require every claim to
cite a supporting input unit (a used fact, or a grounded source byte). A claim with no
supporting bytes is *ungrounded* and is **rejected and kicked back** (the ADJ06
self-correction loop) until the output is fully byte-grounded. That gate is exactly
what should have caught "tremolitized" in geology — and it completes the invariant:
**every input byte accounted for, and every output claim grounded in input bytes.**

## 7. CAS accumulation
The content-addressed store grew and deduplicated across every run: **8 (pheo + KFD)
→ 17 (engineering/astronomy/cybersecurity) → 23 sources** (geology/paleontology/
linguistics). The corpus accretes across domains, as designed.
