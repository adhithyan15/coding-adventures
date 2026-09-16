- **#14986: `astronomy/comet-tail-type.adj` — the ion tail stops being warranted by the dust-tail sentence, and a table-level `cites` becomes a row's own source.**
  The envelope was the `dust_tail` span, so asking what the ion tail does returned
  `always_points_directly_away_from_the_sun` evidenced by *"This dust tail traces a broad, gently
  curving path away from the Sun."* — a sentence about the other tail. The ion-tail sentence was
  present, but as a **table-level `cites`**: riding along as a corroboration rather than as that row's
  warrant.

  ### The first conversion in this batch that relocates a `cites`

  Every previous conversion demoted an envelope. This one also removes a `cites` line (which carried
  its own redundant `locator`), because its sentence becomes the `ion_tail` row's `source`. Closer to
  #15327 (aqi-category-color), where six spans shipped as table-level `cites` and the conversion moved
  them into rows.

  **A consequence worth stating, because it is the figure that proves the relocation:** the
  corroborations array goes from non-empty to **empty**. So `only_citation()`, which closes on
  `"corroborations":[]`, could not have matched this table before the conversion at all. The query
  example now reports **0 non-empty corroborations**.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of NASA Space Place's "What Is a Comet?" page,
  with inline tags removed without a space and only ASCII whitespace collapsed.
  - **Each row takes the NASA sentence defining its own tail.** Each occurs **exactly once**, inside
    one `<p>`, scripts set aside and over the whole file, with no `<head>` copy. Both spans sit in the
    **same `<p>`**, so the truncation mutant cuts mid-clause rather than at a boundary.
  - **The envelope** is the page's sentence framing the two-tail split — *"When astronomers look
    closely, they find that comets actually have two separate tails."* — once, in one `<p>`, pure
    ASCII, naming neither tail as a phrase or as a bare token, and sharing **no content word** with
    either row's value.
  - **The control is a page of comparable size.** The nonsense path answers 404 with a full styled
    page of **11 281** bytes against the real page's **21 830**. Those counts are
    measured-at-a-moment: the sibling `comet-part.adj` measured the same 404 at **11 280** bytes about
    an hour earlier.

  ### The envelope's rejected alternatives, and why the filters could not decide

  Two candidates passed my automated checks and failed on reading:
  - *"Both tails are always directed away from the Sun."* — my restates-row-content check said "no",
    because its needle was the literal `always points directly away` while the sentence writes
    *"always directed away"*. It is a near-paraphrase of the `ion_tail` atom. **The automated "no" was
    not a clearance.**
  - *"The other tail is bluish and is made up of electrically charged gas molecules, or ions."* — it
    DEFINES the ion tail without using the phrase "ion tail", so the `defines()` filter let it through.

  A filter that tests for a phrase cannot see a paraphrase. These narrow the field; they do not make
  the judgement.

  ### Pins

  - **Kept:** the forward bind, the reverse bind, and the `coma` abstention — a real comet part tabled
    in the coarser sibling `comet-part.adj`, not a tail sub-type.
  - **Re-pointed:** the direct test already pinned the whole `"source"` sentence (#13916/#13918), but
    that sentence *was* the envelope, so it rode on every answer including the ion-tail one. It is now
    the dust row's own whole citations array, closing on both the corroborations `]` and the citations
    `]` (#14735).
  - **Inverted:** `contains("nasa.gov") && contains("\"trust\":\"authoritative\"")`, which any NASA
    citation satisfies. The locator is now its own assertion rather than joined by `&&`, because a
    joined failure cannot say which arm broke.
  - **Added:** a per-tail check; a table-shape test whose **no-`cites` arm would have failed on the
    pre-conversion file**, plus exactly-one-`locator` and exactly-one-`trust` (the old file had two
    locator lines — the envelope's and the one inside the `cites`); and a value-supported-by-its-own-
    span test (#15318).

  ### The first mutation run scored 8 of 10, and both misfires were mine

  **10 of 10 mutants are now killed by their named assertion**, both controls green, both files
  byte-identical afterwards.

  - **The #15318 mutant died on a pre-existing test.** Rebinding the atom breaks the reverse-bind test
    that hardcodes `always_points_directly_away_from_the_sun`, so the mutant never reaches the new arm
    — the identical failure `comet-part.adj` hit on the same shape. The form that isolates it: keep
    the atom, replace the **span** in both the `.adj` and the test const, and choose a replacement
    that names the tail but omits the atom's content.
  - **The both-rows-same-span mutant was sound; my expectation named the wrong arm.** It dies on
    `carries its own span`, not on the cross-span negative arm I had written.

  Both were settled by reading which assertion fired, not by reasoning about which one ought to. It's
  a local scratch harness, so that count can't be reproduced from the repo.

  ### The negative arms are NOT redundant here, and I measured that rather than assuming it

  Shard 03540 (`comet-part.adj`) says its cross-span negative arms are "defense-in-depth, not
  independent evidence", because `only_citation()` pins the whole citations array. **That is wrong,
  and this conversion is where I found out.**

  Each row's sentence renders on **two** surfaces. In this table's own query output the dust span
  occurs at byte 334, preceded by `"citations":[{"source":"`, and again at byte 594, preceded by
  `"steps":[{"kind":"fact","step":0,...,"source":"`. The mechanism is in the CLI: the `FromFact` arm
  of `trace_steps_json` calls `prov(&f.provenance)` and inlines it into the step.

  So `only_citation()` constrains the citations surface only. `!out.contains(other)` and
  `!out.contains(ENVELOPE)` are the **sole** guard on the steps surface. They carry independent
  weight, and 03540's narrowing of them should be corrected — filed as **#15334** rather than folded
  into this table's PR, since it is a different table's shard.

  How the wrong claim got in is worth recording: a security review of the comet-part conversion
  reported those arms as dominated, reasoning only about the citations block; I accepted it and
  narrowed the shard. A later review contradicted it with the `trace_steps_json` mechanism. **I then
  measured the byte offsets myself rather than believing whichever reviewer spoke last.**

  ### The #15318 test asserts the WHOLE atom, not fragments of it

  The first draft pinned hand-picked needles — `"always points directly away"` for `ion_tail`. That
  leaves the atom's `_from_the_sun` unchecked, so a span reading *"…directly away from the **comet**"*
  would pass while failing to support `always_points_directly_away_from_the_sun`. Same gap on
  `dust_tail`, where *"…a broad, gently curving path **toward** the Sun"* would pass.

  In this table each atom is a **contiguous phrase of its own span** once underscores become spaces,
  so the test now asserts the whole atom. `comet-part.adj` cannot do this — its atoms reorder their
  sentences — which is why that table needs distinctive phrases and this one does not.

  ### The query example, reported as it actually runs

  2 answers and 1 abstention; the envelope's wording occurs **zero** times in the output; 2 citations
  arrays and **no** non-empty corroborations. Both rows are exercised — dust and ion each appear twice
  (answer plus proof step) — so unlike the `comet-part` and `solar-eclipse-type` examples, no row is
  left to the test suite alone.

  The query file said the engine returns the description plus "the table's own source/locator/trust";
  it now describes the row's own NASA sentence with the locator and trust inherited. The old wording
  is quoted and dated in place rather than silently replaced, because it was accurate before this
  conversion.
