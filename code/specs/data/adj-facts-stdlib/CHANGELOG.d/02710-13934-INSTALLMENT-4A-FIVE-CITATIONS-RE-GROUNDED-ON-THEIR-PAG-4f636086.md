- **#13934 installment 4a: five citations re-grounded on their pages' own sentences.** 5 value
  lines, 3 comment lines, 5 files. `biology/symbiosis-type`, `language/capitalization-rule`,
  `earth-science/plate-boundaries`, `metrology/si-base-units`,
  `chemistry/measuring-tool-si-unit`. Five mutations redden; one pre-existing assertion failed
  correctly and was repointed.

  **All five were the same editorial act: a fragment punctuated into a sentence.** Each differed from
  its page by exactly one character — two had a capital added at the front, three a terminal period
  the page does not have. `symbiosis-type` shipped "Where both parties benefit…" where the page reads
  "Finally, **w**here both…"; `plate-boundaries` ended a clause with a period where the page has a
  semicolon. That is the same act as `geography/map-type.adj:57`'s "Political maps" for the page's
  "Political Maps" — which this changelog called a *paraphrase* for three installments before review
  checked the page. It is not paraphrasing. It is tidying, and **every quote-keyed screen passes it,
  because the quotes are all correct.**

  **The five repairs are not one shape, and an earlier draft of this entry counted them as four of a
  kind.** Precisely: **two** extend *backward* to the page's sentence start and name their subjects
  (`symbiosis-type`, `capitalization-rule`). **Two** extend *forward* and end by naming NIST SP 1247,
  a poster — `si-base-units` and its importer `measuring-tool-si-unit`, whose values are
  byte-identical — so neither names a row nor a measuring tool. **One** changed only its terminal
  character, restoring the page's semicolon (`plate-boundaries`); it extends in neither direction and
  does not end as a sentence.

  So this installment introduces **two** citations that name no subject, not one. Winning contiguity
  by starting mid-sentence is how 3d shipped a citation that never named a square; winning it by
  running *forward* into a figure reference is the same trade in the other direction, and it is worth
  naming as a recurring temptation rather than a one-off.

  **`si-base-units`'s pin deliberately binds no row — but grounding *was* available, and an earlier
  draft of this entry said it was not.** That 7-row table's single envelope says "The SI is made up
  of 7 base units that define the 22 derived units with special names and symbols…" — a fact about
  the SI and about none of its rows, with zero per-row `cites`. A row-binding pin would freeze the
  #14124 defect into a test exactly as 3d froze a bone-deformity answer beside a citation about night
  blindness, so the pin binds nothing.

  What I wrote as *"the grounding claim is not mine to make"* overstated it. **The same cited page
  ships the row-level claims verbatim**, further down the same page — `Length - meter (m)`,
  `Mass - kilogram (kg)`, `Time - second (s)`, `Electric current - ampere (A)`,
  `Temperature - kelvin (K)`, `Amount of substance - mole (mol)`, `Luminous intensity - candela (cd)`
  — all seven confirmed present on the page. Per-row `cites` from that list would ground every row
  and make a sound row-binding pin available at negligible cost. Deferred to #14124 **with the exact
  text recorded**, rather than described as unavailable.

  `measuring-tool-si-unit` carries the byte-identical value and is in the same position: it too ends
  on the poster, and its rule head is `measuring_tool_si_unit($Tool, $Unit, $Symbol)`, so the
  sentence names no measuring tool either.

  **A third #14124 instance surfaced from reading bindings beside source.** `capitalization-rule`'s
  first authored query asks `pronoun_i`, and its envelope — about capitalizing the first letter of a
  sentence — does not ground that row. The pin uses the query the envelope *does* ground. This was
  visible only because the pin generator prints the returned bindings next to the returned citation;
  no screen measures whether a citation supports its answer.

  **`measuring-tool-si-unit`'s warrant is pinned through `steps`, and my first attempt was wrong in
  an instructive way.** A citation-only pin came back **green** under the restore-the-fragment
  mutation, and I recorded that as *"the two citations are indistinguishable, so no pin is
  possible"* — because this rule imports `../metrology/si-base-units.adj` whose `source`, `locator`
  and `trust` are byte-identical to its own warrant.

  **The byte-identity is real; the conclusion was not**, and review disproved it against the binary.
  `citations` carries **leaf-fact citations only** — a rule's own warrant never appears there, which
  is why a citation-only pin could not see the mutation. The warrant *is* emitted in `steps`, tagged
  `"kind":"rule"` with the rule's goal, while the imported table's identical sentence appears as
  `"kind":"fact"` with goal `si_base_unit(...)`. Perfectly distinguishable. The library's own header
  says exactly this at lines 39-43; **my excuse contradicted the file it was written in.** The
  step-anchored pin is in place and reddens.

  Worth noting for consumers: a caller reading only `citations` never sees the warrant behind a
  derived answer. Recorded on #14124.

  **The mutation harness itself was hardened.** After that pin was removed, its mutation still
  reported GREEN — because `cargo test` with a filter matching *no* tests prints "test result: ok"
  and the harness read that as a pass. A removed or mistyped test name could therefore have passed
  every mutation silently. The harness now requires the filter to have run at least one test.

  Contiguity screen: **84 → 79** non-contiguous values, exactly the five repaired, from re-running
  the full sweep.

  *The rule that number is measured under*, because 3d wrote its population down "so it can be
  recomputed rather than trusted" and this entry initially did not: every `source`/`cites` value of
  at least 20 characters that has a resolvable locator — **561 examined of 567 total** — is compared
  against its page after stripping tags **with no separator inserted at tag boundaries**, unescaping
  entities, converting NBSP to space and collapsing whitespace. That choice is load-bearing: two of
  these five values span an `<a>` element, and an extractor that inserts a separator at tag
  boundaries would call them non-contiguous. The number flips on it.

  **A metric this entry first credited to 4a belonged to 3d.** I wrote "composed spans fell 17 → 15";
  that drop was installment 3d's, and 4a moved the number by zero. It could not have moved it: none
  of the five values *as shipped before repair* contained a quote character, so none was ever in the
  quote screen's population. The sentence also reused the raw 17 after 3d had corrected it to **14 composed spans
  plus 3 unjudgeable** — re-conflating the exact three cases 3d separated. True composed-span count:
  **12**, plus 3 non-HTML locators. 4a's only effect on that screen is the opposite of the one
  claimed: `capitalization-rule`'s repaired value introduces a curly apostrophe, so the population
  rose 77 → 78 with a clean verbatim match.

  Header/data desync screen: 0. 533 test binaries / 1648 tests green; clippy `-D warnings` clean;
  both `adj_stdlib_*` gates exit 0.

  *Still open:* 79 non-contiguous values (#14111); 36 unjudgeable locators — 33 of them PDFs, where
  "verbatim against a PDF" needs defining before it can be checked; **2 fetch failures, which are
  UNCHECKED rather than clean**; and #14124, whose census now stands at 240 of 350 tables sharing one
  envelope across 1,482 rows.

