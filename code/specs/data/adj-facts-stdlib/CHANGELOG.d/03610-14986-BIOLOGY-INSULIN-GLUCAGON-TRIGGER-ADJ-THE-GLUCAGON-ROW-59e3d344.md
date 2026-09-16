- **#14986: `biology/insulin-glucagon-trigger.adj` -- the glucagon row stops being warranted by the insulin sentence.**
  The envelope was the `insulin` span and the glucagon sentence rode as a table-level `cites`, so asking
  what triggers glucagon release returned `low` evidenced primarily by *"Beta cells in the pancreatic
  islets secrete the hormone insulin in response to a high concentration of glucose in the blood."* — a
  sentence about the other hormone. Relocating it empties the corroborations array on every answer.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of the NCI SEER Training Modules "Pancreas" page,
  inline tags removed without inserting a space, only ASCII whitespace collapsed.
  - The envelope and both row spans occur **exactly once** on the page and **zero** times against a real
    404 control on the same host.
  - **The page's 41,705 bytes are pure ASCII** (zero above 127), and none of U+00A0, U+2013 or U+2019
    appears in it. It *does* encode **U+2014 three times as `&mdash;`** — in the `<title>`, the `<h1>` and
    the suggested-citation block — so "pure ASCII" is a claim about the **markup**, not the decoded text.
    None of the three shipped spans touches any of them. Stated at the width it was measured: an earlier
    draft of this entry said "zero non-ASCII codepoints on the page", which the entities contradict.
    Worth stating at all because **two of the four conversions already merged in this batch ship non-ASCII
    on an evidence line**, measured across all four rather than recalled: 03590 (`sun-layer`) carries
    U+2013 on two row sources with a U+2019 sharing one of them, and 03570 (`mixture-types`) carries a
    **U+00A0** in its envelope — the worse case, being invisible. 03580 (`blood-cell-types`) and 03600
    (`muscle-nuclei-count`) are pure ASCII on those lines.
  - Each row span names its own hormone, its own cell type, and states the blood-glucose condition its
    atom compresses.

  ### The envelope must name no part of the taxonomy — not merely neither row key

  The new envelope is the page's framing sentence, *"The pancreas is a long, soft organ that lies
  transversely along the posterior abdominal wall, posterior to the stomach, and extends from the region
  of the duodenum to the spleen."*

  Naming neither row key is not enough. This page's **very next sentence** frames the exocrine pancreas —
  *"This gland has an exocrine portion that secretes digestive enzymes that are carried through a duct to
  the duodenum."* — names no hormone either, and would frame the wrong half of the organ, since this table
  is entirely about the endocrine islets. So the shape arm forbids the page's whole hormone/cell
  taxonomy: `insulin`, `glucagon`, `alpha`, `beta`, `islet`, `exocrine`, `endocrine`, `hormone`.

  Forbidding alone is not framing, though, and a review caught that the arm was purely negative: this
  page's own left-navigation strings ("Ovaries Genital Tract External Genitalia") name no forbidden token
  and frame nothing whatever, so they would have passed. The arm now also **requires** the envelope to
  name the organ. That exceeds the written standard — `README.md` asks only that a frame "names no row key
  or value" — and is hardening, not a standard violation.

  ### An arm the previous test sets did not have

  The mutation set is built around one sentence from the cited page: *"The endocrine portion consists of
  the pancreatic islets, which secrete glucagons and insulin."* It names **both** hormones and states
  **no** trigger.

  As an envelope it is the shard-03590 shape. As a **row source** it is the #15318 shape — provenance that
  reads as entirely plausible (right organ, right half, real page text) while supporting neither row's
  fact. Every arm this test previously had would have passed it: the row-header anchor proves only *which*
  span sits under a row, not that the span *states what the row asserts*, and a const co-edit satisfies a
  literal pin.

  So `each_span_states_the_level_its_row_compresses` was added: each row's span, read out of the shipped
  file anchored on its row header, must state its level phrase (`high`/`low concentration of glucose in
  the blood`) and name its own cell type. `M07` is what demonstrates the arm works rather than merely
  exists.

  ### The page spells it "glucagons"

  The glucagon span reads *"the hormone glucagons"*. The page writes the plural in both of its prose
  sentences, and it ships **verbatim** under the #15320 precedent, where the `balance` row quoted its
  page's own typo rather than a repaired form. Correcting it to "glucagon" would cite a sentence the page
  does not contain — the error class this batch exists to remove.

  ### Pins

  - **Inverted:** `contains("seer.cancer.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by
    any SEER citation, constraining no sentence text, and before this change satisfied by the insulin span
    riding on the glucagon answer. It could not fail for the bug it was meant to cover. Each answer is now
    pinned to its own whole serialised citation object, closing on `"corroborations":[]}` (#14735).
  - **Added:** a per-hormone loop with whole-span negative arms; a shape arm forbidding the page's whole
    hormone/cell taxonomy; a span-supports-its-own-row arm; and row-header-anchored span checks.
  - **Every negative arm names a WHOLE span,** because the two sentences differ in only three tokens each
    (`Beta`/`insulin`/`high` against `Alpha`/`glucagons`/`low`) and share nine content words. A short
    needle like "pancreatic islets" matches both and would pass whichever sentence came back.
  - **Kept:** the backward bind and the `thyroxine` abstention.
  - **Scope stated in the failure text.** The "no `cites` at any indent" and "exactly one `locator`/`trust`"
    arms pin a convention local to *this* table, not a language rule: `lower.rs`'s row path accepts
    `Source`, `Locator`, `Trust`, `Cites` and `Quote`.

  ### The query example, reported as it runs

  **3** queries, **3** answers, **0** abstentions — this worked example has no abstention query, unlike
  shard 03600's; the `thyroxine` abstention lives in the e2e test. **3** citations arrays, **0** non-empty
  corroborations, envelope wording in **zero** answers, and **6** empty-corroboration occurrences because
  provenance renders on two surfaces (`citations`, and the steps array's `"kind":"fact"` entries). The
  insulin span appears 2× and the glucagon span 4× (one and two answers respectively, across both
  surfaces), which sums to the same 6.

  ### Mutants

  **11 of 11 killed by their named assertion**, unmutated controls GREEN before and after, both files
  restored byte-identically. These are results of a mutation RUN, reported so the claim is attributable —
  they are not properties the shipped files assert, and cannot be re-derived from this diff alone. And
  11/11 measures the mutants I thought of: a review found a **dead negative arm** this set never probed
  (`!contains("\"corroborations\":[\"")` — corroborations render as objects, so the needle could never
  match), proved it by restoring the pre-conversion `cites` and watching the arm pass. It is now a count
  comparison (every corroborations array empty, and at least one rendered) rather than a hand-picked
  needle. The set covers the pre-conversion defect restored, a bare row re-inheriting
  the envelope, cross-span and swapped spans (the swap dies only by a row-header anchor), the exocrine and
  endocrine sentences as envelopes (each with a test-const co-edit, so the taxonomy arm is what kills them
  rather than a literal pin), the endocrine sentence as a row source, a content-word deletion, a re-added
  `cites`, a trust downgrade, and a duplicated locator.
