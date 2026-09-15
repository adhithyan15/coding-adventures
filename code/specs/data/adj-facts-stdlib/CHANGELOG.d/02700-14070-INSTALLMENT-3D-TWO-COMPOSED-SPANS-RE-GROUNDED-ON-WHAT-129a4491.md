- **#14070 installment 3d: two composed spans re-grounded on what their pages actually say, and the
  glyph line of work closes.** `earth-science/atmosphere-layers` and `geometry/quadrilateral-types`.
  2 value lines, 43 comment lines, 2 files. Both mutations redden; **four** pre-existing
  assertions failed correctly and were repointed.

  **A class-general quote screen now replaces the two per-class ones.** Seven axes of quote defect
  were found across 3a–3c, each by review, each needing its own rule. Rather than write an eighth,
  the screen normalizes **every** quote-class codepoint to a single sentinel on both value and page,
  locates each normalized match, and compares **raw bytes** there. A normalized match whose bytes
  differ *is* the defect, whatever produced it.

  Positive controls first, because a screen reporting zero is worthless until it reports non-zero on
  a defect it should catch: four axes were each re-introduced into a known-good value and the screen
  required to flag them. All four do, including ASCII-single→ASCII-double, which **both** previous
  screens were blind to by construction.

  **Result: 0 citations anywhere in the stdlib are repairable by requoting.** The glyph work begun in
  3a is finished — 23 citations across four axes.

  *The population that "0" is measured over*, stated so it can be recomputed rather than trusted:
  `source`/`cites` values at least 20 characters long containing at least one of
  `' ‘ ’ ‛ ʼ ´ " “ ” „ ‟ « » ‹ ›`, with a resolvable locator — **77 of the stdlib's 567 values**.
  Review counting under a narrower rule got 64. Neither is wrong; they measure different sets, and
  every count in this effort has moved when the rule moved, so the rule is now written down with the
  number.

  **What the screen could not judge, it said so rather than guessing.** 17 values produced no
  contiguous page match — but three are not composed spans at all: two locators are PubChem **JSON**
  endpoints (`chemistry/element-symbols:55`, `chemistry/elements:65`) and one is a **.pptx**
  (`language/dolch-sight-word-level:469`), which an HTML extractor has no business reading — its
  longest "overlap" with the PowerPoint was the literal string `presentation.xml`. That is the
  swallowed-TLS error inverted: that one made an *unchecked* site look clean, this makes an
  *unjudgeable* site look defective. Corrected to **14 composed spans and 3 unjudgeable**.

  **The 3c entry below calls these "the 17 `language/*` composed spans", and that locus was wrong.**
  Only six of the seventeen are under `language/*`; the rest are chemistry, earth-science, geometry
  and physics. 3d corrected the *composition* of the 17 while inheriting 3c's mis-scoping, and review
  caught that too. The full list is on #14111.

  **The two repaired here are re-groundings, not requotings.** `atmosphere-layers` stopped at "found
  here." where the sentence continues through the cumulonimbus clause; the full sentence still
  grounds `troposphere → weather` and adds a caveat, not a contradiction.

  `quadrilateral-types` is the more interesting one, and **review corrected me twice on it**. The
  value had silently dropped a parenthetical while keeping the opening words, so it read as a whole
  sentence and appeared on no page. My first repair quoted from "a geometric figure…" — which fixed
  contiguity and **dropped the subject**, leaving a citation that never names a square, in a file
  whose own header insists a citation must not omit its subject. Extending through the following
  sentence — "In other words, a square is a regular polygon with four sides" — is both contiguous and
  names what it defines.

  I had also justified the truncation by calling the parenthetical **MathML**. It is not: the page
  has zero `<math>` elements and 47 `<img class="inlineformula">` images, which this same header
  already describes correctly for the rectangle row. And the full sentence **is** available as
  plaintext in the page's `<meta>` descriptions, so quoting from the body is a choice, not a
  necessity. Both corrections are recorded in the file, because the wrong version was written as an
  instruction not to revisit it.

  **My repair also moved a value out of the screen's reach.** The new span contains no quote
  characters, so the quote screen's population fell 78 → 77. It is verified by its e2e pin instead. A
  screen keyed on a character cannot vouch for values carrying none — and the instrument this effort
  still lacks checks *every* value for contiguity regardless of quoting. That is also the only kind
  that could see `geography/map-type.adj:57`, which ships a paraphrase and passes every screen built
  so far.

  **The pin queries `square`, which the authored `.query.adj` does not cover** — its queries are
  rhombus/parallelogram/trapezoid/rectangle, each grounded by a per-row `cites`; only `square` is
  grounded by the `source` that changed. Two pre-existing pins that broke were rectangle and trapezoid
  pins whose span opens on the *square* sentence: **pins reaching across the envelope into text that
  does not ground the row they name.** They pass once repointed, but that shape is the #14124 smell.

  533 test binaries / 1643 tests green; clippy `-D warnings` clean; both `adj_stdlib_*` gates exit 0;
  both `.query.adj` companions parse and run clean.

  *Still open:* 14 composed spans (#14111, re-grounding under #13934), 3 unjudgeable locators needing
  a content-type-aware reader, #14124, and the `map-type.adj:57` paraphrase.

