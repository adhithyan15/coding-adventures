# ADJ71 — CAS Program-Cache Experiment

> **Status (2026-06-08):** Automated narrow-slice experiment showing that a
> byte-provenanced legal source can be compiled once into a CAS-backed executable
> library, then reused by compiling an eleventh input into a small program that
> imports the cached library and runs without answer-time model calls. Domain:
> ordinary five-year naturalization eligibility under 8 U.S.C. 1427(a). This is a
> systems experiment, not legal advice.

## 1. Question

The question behind this run:

> If many users repeatedly reason over the same knowledge or rulebook, can that
> rulebook become a content-addressed executable program so future cases only need
> input-to-program compilation?

ADJ16 specified the deterministic-engine version of that idea. ADJ57 made the
source/input byte-provenance invariant explicit. ADJ70 showed that skipping any
input layer invalidates the run. ADJ71 tests the program-cache shape end to end.

## 2. Automated Harness

Implementation:

- [`code/specs/data/adj71/run.py`](data/adj71/run.py)

Command:

```bash
python3 code/specs/data/adj71/run.py --out-dir code/specs/data/adj71/out
```

The harness performs the whole path:

1. Fetches the official OLRC release-point XML zip for Title 8.
2. Interns the raw source bytes into a SHA-256 CAS.
3. Extracts the exact `<section>` bytes for 8 U.S.C. 1427 from `usc08.xml`.
4. Finds eight exact source-byte spans supporting the ordinary 1427(a) rule
   subset.
5. Partitions the source section into represented spans and discarded spans.
6. Generates a deterministic Python rule library from those byte-cited spans.
7. Interns ten synthetic training case inputs and their IRs into the CAS.
8. Runs those ten cases against the generated library as a corpus validation
   pass.
9. Interns an eleventh held-out input, converts it into IR, emits a case program
   that imports the cached library, and executes that program.
10. Writes a manifest, generated library, held-out program, execution result,
   source IR, training results, CAS index, and CAS blobs under
   [`code/specs/data/adj71/out/`](data/adj71/out/).

No model is called at answer time. This first slice also does not call a model at
source-decomposition time; the extractor is deterministic so the cache shape is
isolated from LLM variance.

## 3. Source Corpus

Source:

- `https://uscode.house.gov/download/releasepoints/us/pl/119/95/xml_usc08@119-95.zip`

Release point:

- `Public Law 119-95 (05/29/2026)`
- Archive hash:
  `a66b51e9deae5606ba6d7cad0c8fa920fa96f16a3a4e15e9548767284bfd63c1`
- Title XML hash:
  `40df2a8b138086b6d84e50be8292813dde70798499cb469f42cf104803594898`
- Section byte range in `usc08.xml`: `[5903622, 5929979)`

Run source object:

- Source hash:
  `ac97e9ef0ceb87cefd2df2975920237534c833d33c7bd1a2a019ad5bfb5a9c41`
- Source section bytes: `26357`
- Represented rule spans: `8`
- Represented source bytes: `939`
- Discarded source bytes: `25418`

The discarded source-section bytes are not ignored silently. The generated
source IR records them as outside the ordinary five-year naturalization subset
used by this experiment. This is coarse, but it is explicit and byte-total.

The eight represented spans support:

- ordinary-path scope (`except as otherwise provided`)
- LPR plus five years continuous residence
- physical presence for at least half of the five-year period
- three months in the filing state or USCIS district
- continuous residence from application to admission
- good moral character
- attachment to constitutional principles
- disposition to the good order and happiness of the United States

## 4. Generated Library

Generated library:

- [`code/specs/data/adj71/out/generated/immigration_316a_rules.py`](data/adj71/out/generated/immigration_316a_rules.py)
- Library CAS hash:
  `0849b243e02158ada47f7905582627b2fc96e025385ea9974962c32299f98989`

The library contains:

- `RULE_PROVENANCE`: source span metadata for every rule requirement.
- `REQUIREMENTS`: executable predicates for the ordinary 1427(a) slice.
- `evaluate_case(case)`: deterministic evaluator returning an eligibility result
  and proof trace.

The bridge from "half of five years" to `30` months is represented as derived
arithmetic in the proof trace:

```text
5 years * 12 months/year / 2 = 30 months
```

This is the desired split: source bytes justify the rule; arithmetic is executed
by the CPU.

## 5. Ten-Case Corpus Pass

The ten synthetic immigration-law fixtures are controlled inputs designed to
cover one passing case and failures for each rule condition:

- clean pass
- short continuous residence
- not LPR
- short physical presence
- short state/district residence
- no continuous residence after filing
- no good moral character
- not attached to constitutional principles
- not well disposed
- exception path claimed / outside ordinary-path scope

Result:

- Training cases: `10`
- Matched expected outcome: `10 / 10`
- Answer-time model calls: `0`

The point is not that synthetic fixtures prove legal completeness. They prove
that the generated rule library is executable, reusable, and carries byte
provenance through its proof traces.

## 6. Eleventh Held-Out Case

Held-out input:

- Case ID: `case_011_heldout_borderline_pass`
- Input hash:
  `75a6c599d131b1d34a7ed81ea75313eab7c7eed1c0b606a91f903fd196af25a7`
- Input IR hash:
  `bdb3fc58e18f7180f02faa2869bd5f790813348c3af1a8cdd129462f2c092267`
- Case program hash:
  `6963d3227dfa02e96566d47291347d7d8c7fbde2ae9faaae92a1792fb0c37689`
- Result hash:
  `f697475c6c26d28d9f59a01e0da4a136fe68cdbdce95e823ed95bfcb2f72b0fa`

Input byte coverage:

- Input bytes: `912`
- Represented bytes: `912`
- Discarded bytes: `0`

Execution result:

```json
{
  "case_id": "case_011_heldout_borderline_pass",
  "eligible": true,
  "failed_requirements": [],
  "answer_time_model_calls": 0,
  "engine": "generated_python_rule_library"
}
```

The generated held-out program imports the cached rule library and contains only
the fresh case IR:

- [`code/specs/data/adj71/out/generated/case_011_heldout_borderline_pass_program.py`](data/adj71/out/generated/case_011_heldout_borderline_pass_program.py)

This is the CPU-bound shape the user hypothesized:

```text
source bytes -> source IR -> reusable executable rule library
fresh input bytes -> input IR -> case program -> CPU execution
```

## 7. What Worked

The experiment demonstrates that the core architecture works in a narrow slice:

- CAS can store the raw source, source IR, generated library, case inputs, case
  IRs, held-out program, and result.
- Source-byte spans survive into the generated rule library.
- Input-byte spans survive into the generated case IR.
- The held-out answer is produced by executing a generated program that imports
  the cached library.
- The proof trace cites both source bytes and input bytes for every requirement.
- The answer path is CPU-bound once the library and case program exist.

## 8. Limitations

This run is deliberately narrow.

- The case language is controlled and regex-parsed. It does not yet prove robust
  natural-language input compilation.
- The legal slice is only ordinary five-year naturalization under 8 U.S.C.
  1427(a); it excludes exception pathways and many real-world eligibility
  complications.
- The generated library is Python, not yet Prolog, ProbLog, Datalog, or ADJ.
- The source-byte discard reason is coarse for large HTML page regions. It is
  byte-total, but not semantically rich.
- The ten cases are synthetic validation fixtures, not legal precedent and not a
  benchmark.
- There is no LLM/source-decomposition agent in this first run. That was
  intentional: the experiment isolates whether CAS-backed program reuse works at
  all.

## 9. Takeaway

Yes, parts of reasoning can become CPU-bound once the shared rulebook is compiled
into a content-addressed executable library. In this run, the reusable source
corpus was paid for once, the eleventh case only needed input compilation, and
the answer came from deterministic program execution with byte provenance.

The next harder experiment should replace the controlled input grammar with an
LLM input compiler guarded by byte coverage, and replace the generated Python
library with a small Prolog/ProbLog/ADJ target while keeping the same CAS object
graph.
