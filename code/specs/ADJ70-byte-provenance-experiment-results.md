# ADJ70 — Byte-Provenance Experiment Results

> **Status (2026-06-07):** Three manual cross-domain shakedowns of the
> byte-provenance discipline: a coding benchmark task, a legal/regulatory
> reasoning case, and a closed-book memory-source experiment. The headline
> result is not that the framework always beats a blind frontier model. It is
> sharper: when every stage is forced to account for input bytes, source bytes,
> and discarded bytes, the framework exposes mistakes that a correct-looking
> answer can hide.

## 1. What Was Tested

The experiments tested the current ADJ direction under three increasingly
strict provenance shapes:

1. **Coding task with code-as-input.** A DeepSWE-style Bandit task asked for
   structured `nosec` directives. The first framework attempt was invalid
   because it treated the natural-language task as input but not the codebase.
   The run was restarted from a code-input CAS/IR that accounted for the base
   repository bytes before implementation could count.
2. **Legal/regulatory task with source-byte grounding.** An FCRA employment
   background-check fact pattern was answered using official U.S. Code pages
   converted into selected source-byte spans. The framework answer and blind
   answer agreed, but only the framework answer was auditable.
3. **Training-memory source test.** A constitutional eligibility fact pattern
   prohibited retrieval. The model had to emit the exact source text it believed
   supported its answer. Those emitted strings were hashed as claimed source
   bytes, then audited separately for grounding and authenticity.

These are not automated benchmark numbers. They are shakedown runs designed to
find methodological failures in the proposed universal pipeline.

## 2. Result Summary

| Run | Blind result | Framework result | Judge/audit outcome |
|---|---|---|---|
| Bandit coding task | Passed its own tests, but treated empty selector results as blanket suppression | Passed affected tests and hidden-style probes after code-as-input correction | Framework selected; blind had a likely hidden-test bug |
| FCRA legal case | Broadly correct from model memory | Same conclusion, with byte citations to input and statute spans | Framework selected because it was grounded and surfaced exception uncertainty |
| Memory-source constitutional case | N/A; retrieval was disabled | Answer grounded in model-emitted claimed source bytes | Conditionally admissible only; authenticity of memory bytes not established |

## 3. Coding Task: Bandit Structured `nosec`

### Setup

The task was copied from a public DeepSWE task page, but solution material was
not opened. A neutral prompt was used for agents: the task text and local repo
path were provided, without benchmark framing or the source URL. Agents were
also told not to use external sites.

Both candidate repos were cloned from `PyCQA/bandit` at base commit
`b46fa3a2723635aa29cc012538df4867ac2ac006`.

### Methodological Failure Caught

The first framework path skipped a crucial step: it decomposed the task text but
not the codebase. That was invalid. Code is input.

The corrected framework run built a code-input CAS/IR from the base commit:

- 298 tracked base-repo files stored in CAS by SHA-256.
- 12 files selected for detailed byte partitioning.
- 30 represented code/data spans.
- 33 discarded selected-file spans, each with a reason.
- 286 discarded whole files, each with a reason.
- A no-edit verifier rejected v1 and v2 of the ledger, then passed v3 after
  missing `node_visitor.py`, fixture, and functional-test spans were added.

The key lesson: a code task cannot treat source code as ambient context. The
repo bytes must be part of the provenance graph before a patch can be credited
to the framework path.

### Candidate Comparison

The blind candidate implemented structured `nosec` support and passed its own
tests. A final judge still found a semantic bug: the blind parser collapsed
empty resolved selector sets into blanket suppression. Examples such as
`# nosec-next-line B999` and `# nosec-next-line B101 & B102` suppressed when
they should have had no effect.

The framework candidate preserved the required distinction:

- `None` means no suppression applies.
- `set()` means blanket suppression.
- A non-empty set means specific tests are suppressed.

The framework patch passed:

- Python compile checks on touched files.
- `flake8` on touched files.
- `git diff --check`.
- 30 affected unit/functional tests.
- Hidden-style probes for empty selectors, statement-wide suppression,
  directive-line exclusion, `ignore_nosec`, and grouping/ellipsis lines with
  trailing comments.

### What Was Not Proven

The hidden DeepSWE grader was not run, and no official solution page was opened.
The strongest claim is therefore:

> The blind solution solved a weaker version and likely fails hidden tests; the
> framework candidate is likely correct against the task text after the
> code-as-input correction and additional hidden-style probes.

## 4. Legal Task: FCRA Employment Background Check

### Setup

The fact pattern:

- Acme used a third-party background-check company to obtain a consumer report
  about a store-manager applicant.
- Acme decided not to hire the applicant partly because of the report and partly
  because of interview concerns.
- Acme sent one final-decision email attaching the report and federal summary of
  rights.
- Acme sent no earlier notice or report copy before deciding not to hire.

The framework used official U.S. Code pages as primary sources:

- 15 U.S.C. 1681a: adverse-action definition.
- 15 U.S.C. 1681b: employment pre-adverse-action rule and exception scope.
- 15 U.S.C. 1681g: summary-of-rights source provision.
- 15 U.S.C. 1681m: post-adverse-action notice context.
- 15 U.S.C. 1681n: willful noncompliance remedies.
- 15 U.S.C. 1681o: negligent noncompliance remedies.

Those pages were fetched into local source files, hashed into CAS, and reduced
to seven selected source segments, including the case input itself.

### Outcome

Blind and framework agents reached the same broad legal answer:

- Acme likely did not comply because the report and rights summary had to be
  provided before the adverse action.
- Partial reliance did not change the answer because the trigger is based
  "in whole or in part" on the consumer report.
- Negligent noncompliance exposes actual damages plus costs and reasonable
  attorney's fees.
- Willful noncompliance adds statutory damages, possible punitive damages, plus
  costs and reasonable attorney's fees.

The final judge selected the framework answer, not because the blind answer was
wrong, but because the framework answer tied each rule and fact to input/source
segments and surfaced a narrow exception: the transportation-position and
remote-application exception in the selected 15 U.S.C. 1681b span. The provided
facts did not trigger that exception.

### Lesson

On an easier legal case, the framework did not produce a different binary
answer. Its value was auditability: it made the same answer reviewable, cited,
and sensitive to a source-backed exception.

## 5. Memory-Source Experiment: Claimed Bytes From Model Weights

### Setup

The final test intentionally disabled retrieval. The model received only a
short constitutional eligibility fact pattern:

- Taylor was born in Canada to parents who were not U.S. citizens.
- Taylor naturalized at age 10.
- Taylor is now 45 and has lived in the United States for 30 years.
- Taylor asks whether they are eligible to be President or Vice President.

The model was instructed:

> If you assert a rule from memory, name the source and provide the exact source
> bytes you believe prove it. If you cannot recall exact source text, say the
> claim lacks memory-byte provenance.

### Outcome

The model emitted claimed exact bytes for:

- U.S. Constitution Article II, Section 1, Clause 5.
- The Twelfth Amendment vice-presidential eligibility sentence.
- 8 U.S.C. 1101(a)(23), defining naturalization as conferring nationality after
  birth.

Those emitted strings were placed in `claimed_memory_sources.json` and hashed
as CAS blobs. A separate auditor treated them as the only source-byte ledger
besides the input.

The audit verdict was:

> Mostly byte-grounded but only conditionally admissible.

It classified:

- Input facts as verified input bytes.
- Constitution/statute strings as stable claimed memory bytes.
- Real-world authenticity as not established.

It also caught a missing bridge: the answer relied on an interpretive step that
"natural born Citizen" means citizen at birth and excludes Taylor. The emitted
bytes supported naturalization being after birth, but did not themselves fully
define "natural born Citizen."

### Lesson

Model-memory source bytes can be forced into a ledger, hashed, cited, and
audited. But they are not source-verified bytes. They need a distinct
authenticity label:

```text
source_verified
input_verified
claimed_from_model_memory
```

The memory-source path is useful because it forces latent recall to become an
inspectable artifact. It is not a substitute for retrieval when authenticity
matters.

## 6. Cross-Run Findings

### A. Every Stage Needs Byte Provenance

The coding run failed methodologically until code itself was decomposed into a
CAS/IR. The law run included both input and source bytes. The memory run showed
that even model-recalled source text can be made concrete, but must be labeled
as claimed.

The rule is stronger than "cite your sources":

> Every byte that influences a claim is input, source, or discard. If it is
> discarded, the reason must be explicit.

### B. Provenance Changes The Failure Mode

The blind Bandit agent looked good until hidden-style probes exposed a selector
semantics bug. The FCRA blind answer was substantively fine but could not be
audited. The memory-source answer looked legally plausible but the auditor
found an authenticity gap and an interpretive bridge gap.

The framework did not merely improve accuracy. It made errors locatable.

### C. Judge Agents Are Useful Only After Provenance Gates

The final judges selected the framework candidates in the coding and law runs.
That judgment is meaningful only because earlier stages created auditable
artifacts. A judge without byte ledgers is just another opinion.

### D. "Training Data Provenance" Needs A New Type

The model cannot expose hidden training-corpus byte offsets. But it can emit
the exact bytes it believes it remembers. Those bytes can enter the CAS as
claimed source artifacts, then downstream claims can be tested against them.

This does not solve authenticity. It cleanly separates:

- "The answer is grounded in the emitted bytes."
- "The emitted bytes are authentic to the named source."

Those are different propositions and need different gates.

## 7. Proposed Follow-Ups

1. **Add first-class authenticity classes** to source records:
   `input_verified`, `source_verified`, `claimed_from_model_memory`,
   `user_supplied_unverified`, and `derived`.
2. **Make code-as-input mandatory** for coding tasks. A patch should not count
   as framework-derived unless the relevant base code bytes are represented or
   explicitly discarded.
3. **Add source-claim bridge checks.** A citation can support exact text but
   still not support the legal or technical bridge the answer draws from it.
4. **Persist compact run manifests.** The local runs produced useful artifacts,
   but the repo should eventually store small manifests rather than only prose
   reports.
5. **Automate hidden-style probe generation.** The Bandit run improved only
   after manually probing edge cases. The next harness should generate probes
   from the rule IR and uncertainty list.

## 8. Bottom Line

The framework appears to scale across coding, law, and memory-only settings,
but only if it stays strict:

- No input kind is exempt from decomposition.
- No source byte is accepted without an authenticity class.
- No answer is accepted merely because it is correct-looking.
- No discard is silent.

The most important result is the negative one: a framework run that skips code
bytes or accepts memory bytes as authentic is not a valid byte-provenance run.
The experiments sharpened the contract rather than merely celebrating the
framework.
