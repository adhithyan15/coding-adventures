- **#13934 installment 4k: three real sentences, welded; and a zero that was the instrument.** **1**
  value line rewritten into a `source` plus **2** `cites`, alongside **38** comment lines added
  against **11** removed, in **1** `.adj` file (41 added / 12 removed lines in total) —
  `earth-science/cloud-types` — plus one e2e test file. **Six mutations redden.**

  `cloud-types:72` carried three sentences in one `source` under one `locator`. Measured against
  the page, **each of the three is real and occurs exactly once** — this is not the mitosis case,
  where the quoted string had been assembled out of a fragment and four list labels. What is not
  real is the WELD: the three sit **1443 and 1446 characters apart**, separated by other prose and
  photo captions, so the string as shipped occurs nowhere.

  The repair is the house answer 4c established and 4i and 4j applied: the `source` keeps the
  high-deck sentence, and the mid-level and low-deck sentences become `cites` carrying the same
  locator, because they come from the same page.

  ### Two things that look like defects and are not

  **The singular is the page's.** The mid-level sentence reads `The two main type of mid-level
  clouds` — singular — and the raw HTML reads exactly that: `The two main type of <em>mid-level
  clouds</em>`. The value faithfully reproduces the NWS's own grammatical slip. Correcting it would
  be the defect, and a mutant that quietly pluralises it now reddens.

  **The zero was the instrument.** A tag-stripping extractor that replaces every tag with a break
  scores all three sentences **0** against this page — in raw HTML, in stripped text, and after
  whitespace collapsing. That is not a property of the value. This page wraps every cloud name in
  `<font color>` / `<span style>` for colour, so `<span>cirrus</span>, ` becomes `cirrus , `, with
  a space before the comma that **no reader ever sees and the document's text content never
  contains**. Extracting text nodes and breaking only on BLOCK elements — what a browser does —
  finds each sentence exactly once, byte-exact.

  This matters past this one file: **the contiguity sweep can report a faithful quotation as
  non-contiguous**, purely because of inline formatting. Filed separately; it bears on how the rest
  of the backlog was measured, and no sweep figure is asserted here.

  ### Why it survived: the pin was a locator and a trust tier

  The suite asserted `contains(locator) && contains(trust)` and never the `source` text — so the
  welded string satisfied it exactly as happily as the three real spans do. Same shape as 4j's
  hostname pin, in a different file.

  The needle is now the whole citation object as the serialiser actually emits it, taken from a
  real run rather than from memory of the format, and it CLOSES on the corroborations `]` — which
  bounds the source text, the locator, the trust tier and the corroboration SET together, so a
  fabricated `cites` cannot be appended without reddening (#14735).

  **The duplicate check was done before trusting that needle, not after.** Its string appears
  **four** times in this test's output: the serialiser echoes each citation in `citations` and in
  `steps` — two sections per answer — and the test's program issues three queries of which two
  bind, so 2 × 2. None of the four is an independent copy; mutating the `.adj` takes the old string
  4 → 0 and the new one 0 → 4. So a `contains` needle here does constrain the artifact — unlike
  #14745, where a rule inlines a copy of a composed library's citation and the needle then
  constrains the copy rather than the library.

  This paragraph first said **twice**, in bold, as something "measured rather than assumed". Two is
  what a ONE-QUERY probe returns; the shipping test has two binding queries. The mechanism was
  right and the total was a probe's leftovers — which is the one kind of number a changelog like
  this cannot afford, since its whole value is that the numbers can be checked.

  ### Mutants

  Six, one per thing the repair asserts: RESTORE (the welded string put back exactly as it
  shipped), TYPE-PLURAL (the page's own singular quietly corrected), DROP-MID (one corroboration
  removed), EXTRA-CITES (a fabricated corroboration appended under an attacker-controlled locator),
  LOCATOR-TAIL (a look-alike suffix on the main locator) and WORD (one word of the source span
  changed). All six redden. **RESTORE is the one that earns the pin** — it proves the new assertion
  catches the defect this installment repaired, which the locator-and-trust probe did not. The
  negative arm, the repaired artifact untouched, stays green.

