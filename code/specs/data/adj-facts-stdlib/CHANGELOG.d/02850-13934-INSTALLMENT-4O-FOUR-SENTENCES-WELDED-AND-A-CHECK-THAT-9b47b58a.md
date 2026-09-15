- **#13934 installment 4o: four sentences welded, and a check that could not fail.**
  The LAST of the six actionable candidates. **1** value line rewritten into **2** `cites`,
  alongside **86** comment lines added against **13** removed, across **2** `.adj` files (88 added /
  14 removed lines in total) — `science/scientific-method-step` and, repaying 4n's debt,
  `physics/energy-conversion-example` — plus two e2e test files, the second being 4n's, which
  shares an assertion message the review found overclaiming. **Seven mutations redden.**

  ### Part one: 4n's debt

  4n wrote that both its EIA sentences "END their own paragraph", on the evidence that *the first
  tag after either is `</p>`*. That parenthetical is **true of both and proves neither** — prose is
  not a tag, so the check cannot see prose sitting between a sentence and the tag. Measured by
  walking the `<p>` elements whole: S1 ends its paragraph, one space then the tag; **S2 does not**,
  with 143 characters of prose after it inside the same `<p>` and two sentences before it.

  Run over every `<p>` on the page, that check returns `</p>` for **every** sentence it *can* be
  run on — that is, every one the raw HTML contains verbatim. Why the rest are not verbatim is not
  claimed. Inspecting them turned up one blocked by a **doubled space** and two by **HTML
  entities**, alongside inline tags; the remainder were not examined one by one, so no cause is
  attributed to the set and **no proportion is asserted**. Absolute counts are not quoted, and
  could not be: the population itself is what two implementations of one written census disagreed
  about. A check whose verdict is fixed by how it was built is
  the mirror of 4n's other finding, the `<title>` control that could never pass.

  Nothing 4n shipped is wrong — the spans really are non-adjacent, the weld really occurs zero
  times. Two comments carried the false description and both are corrected here.

  **Finding the two took three attempts.** A five-alternative search over `code/` returned one hit
  and under-reported. Per-needle matching then missed `**ends** a different paragraph`, because
  markdown emphasis sits inside the phrase. The third attempt normalises emphasis on both sides
  **and gives the search harness its own positive and negative control** — without which a scan
  that cannot find anything returns zeros that read as evidence.

  ### Part two: `scientific-method-step:109`

  A `cites` holding **four** sentences. Counted under both extractors, comments stripped in both:

  | | block | crude | where |
  |---|---|---|---|
  | pair A "Define the parts of your experiment that will change. These are called variables." | **1** | 0 | contained whole in `<p>` #7 of 35 (counting from 1) |
  | pair B "Define the parts … will not change. These are called controls." | **1** | 0 | contained whole in `<p>` #12, **four `<p>` elements** and **235 raw characters** later |
  | **the shipped 166-character value** | **0** | **0** | — |

  The crude zeros are the `<strong>` case below, not missing quotes; only the weld's zero holds under both extractors.

  The four intervening elements are `For example:`, `Variable 1: …`, `Variable 2: …` and an empty
  one. Each pair is now its own `cites` under the same locator, and the header's truth-table row —
  which had quoted the weld as one verbatim string — now joins the two with an explicit `AND`.

  **The atom is a conjunction, and that is said plainly.** `define_variables_and_controls` names two
  things: A states the variables half, B the controls half, and **neither alone states the whole
  atom**. Together they do, and both sit in the envelope, which is the condition
  [#14758](https://github.com/adhithyan15/coding-adventures/issues/14758) is about.

  Two further header claims were wrong and are corrected: the two definitions do **not** sit "back
  to back" (four `<p>` between them), and "every other step states exactly one instruction under its
  heading" is not supportable as anything countable — counting every `<p>` under each label except
  the label itself, empties included, the counts per step are 4, 9, 1, 9, 1, 2, 2. The argument
  they were supporting survives and is now the measured part: the capture carries exactly seven
  `Step N.` labels, `Step 1.` through `Step 7.`, each a `<p><strong>` paragraph rather than an
  `<h*>` element (its only headings are two `<h2>`s), and **no** `Step 2a`/`2b` sub-label anywhere
  in its 4,302 bytes, so keeping `step_2` as one row still declines to invent a numbering the
  source never gives.

  ### The page is a fragment, and the crude extractor misses the two sentences that matter

  The fetched capture is a content fragment — 4,302 bytes beginning at an `<h2>`, with no
  `<html>`, `<head>`, `<title>`, `<body>` or doctype. That is a property of what the fetch
  returned, not of what the site serves; it does mean a title-based control could not have been
  run against these bytes at all. Census: every `<p>`, each tag replaced by **nothing** rather than by a
  space (that choice inverts the result — with a space it is block 41, crude 43), entities
  decoded, whitespace runs collapsed, split before a capital following a period, question or
  exclamation mark, keeping 25–200 characters containing a space, de-duplicated — **43** body
  sentences. The block extractor found **43** and the crude one
  **41**; both found **0** of 43 negative controls. The two
  the crude extractor missed are `These are called variables.` and `These are called controls.` —
  because the page writes `called <strong>variables</strong>.`, which also makes their **raw** count
  zero. A reading that used only the crude extractor would have called two real sentences missing.
  No rule about *when* it misses is asserted; that population belongs to
  [#14752](https://github.com/adhithyan15/coding-adventures/issues/14752).

  ### Why it survived: the pin was a host and a trust tier

  `contains("nasa.gov") && contains(trust)` — the same shape repaired in 4j, 4k, 4l, 4m and 4n
  before it. (No count of *files* is given: 4j's own entry records repairing two.) The
  needle is now the whole citation object as the serialiser emits it, closing on the corroborations
  `]`, bounding the source text, every locator, the trust tier and the corroboration set. Its string
  appears **eight** times at eight distinct JSON paths (`citations/[0]` and `steps/[0]` for each of
  the four queries), counted before it was trusted. Six of the seven mutants touch a citation field
  and drive all eight to zero together; the seventh, `DROP-ROW`, touches none — it drops the count
  to six and is caught by the `step_2` assertion instead. What it does *not* bound is how many citation objects the output holds, and the comment
  says so.

  ### Mutants

  `RESTORE` (the 166-character weld put back), `DROP-B`, `SWAP`, `WORD`, `LOCATOR-TAIL`,
  `DROP-ROW`, `EXTRA-CITES` (under `https://evil.example/facts`). Each turns the real suite red;
  none is vacuous; none crashes the CLI. `RESTORE` is caught by the no-weld assertion, `DROP-ROW`
  by the new `step_2` assertion, and the other five by the citation object, so **each of the three
  assertions has a live positive control**. The shipped `.adj` was restored byte-identically
  afterwards (sha256 checked).

  ### The review found eight things

  Two `<p>` ordinals were **0-based against a 1-based total**, so "#6 of 35" pointed at the
  `Step 2.` label and "#11" at the empty paragraph; they are #7 and #12. The per-step tally
  counted `<p>` elements by a rule that **excluded empties**, four lines after a figure that
  **included** one — step 2 is 9, not 8, under one consistent rule. "Exactly seven headings" was
  false as an element count: the capture has two `<h2>`s and the seven `Step N.` labels are
  `<p><strong>` paragraphs. "THE PAGE IS AN HTML FRAGMENT" attributed to NASA's page a property of
  what the fetch returned, and scoped the load-bearing sub-label search to that capture without
  saying so. And the occurrences table said "counted under both extractors" while giving one
  number, when the two extractors disagree on both pairs — which the same entry said correctly
  twenty-five lines later.

  None of the counts could be re-derived from the descriptions, either. The censuses are now stated
  with the elements walked, the splitter, the length filter, whether duplicates are dropped — and,
  after round 3 caught the omission, what a stripped tag is replaced by, which on its own inverts
  the block/crude figures.

  The assertion message claimed the test proves "nothing appended after the corroborations `]`".
  A `contains` needle that ENDS at `]` cannot verify that nothing follows it. That property does
  hold — the serialiser emits `corroborations` last — but not because this test checks it, and a
  change to the serialiser adding a trailing field would not redden. The same message shipped in
  4n, so both files are corrected.

  Last: **the row this installment exists to fix was never queried**. The direct test bound
  `step_1`, `step_4` and `step_7`; the two new spans were covered only through the table-wide
  envelope, so a regression dropping the `step_2` row would have left every assertion green. It is
  queried now, and `DROP-ROW` is the mutant that earns the assertion.

  **This exhausts the six actionable candidates.** What remains on #13934 is the seven held entries,
  enumerated by locator in the loop ledger; none is to be decided by shipping.

