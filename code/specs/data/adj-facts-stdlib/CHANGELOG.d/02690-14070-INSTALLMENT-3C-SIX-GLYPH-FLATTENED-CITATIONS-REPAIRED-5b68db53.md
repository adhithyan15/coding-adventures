- **#14070 installment 3c: six glyph-flattened citations repaired outside `language/*`, with nineteen
  comment lines synced across eleven files.** Five contraction repairs
  (`biology/cell-division-daughter-cells`, `biology/cell-division-genetic-outcome`,
  `biology/heredity-term`, `biology/vitamin-deficiency-symptom`, `physics/newton-laws`) and one
  double-quote repair (`biology/consumer-trophic-level`). Every value verifies verbatim on its page;
  the old forms appear on none. All six mutations redden — five restore-the-flattened-form, one
  flatten-the-double-quotes.

  **The desync screen now covers ALL comment text, not just lines that begin with `%`.** A trailing
  comment on a row line — `row (mitosis, x)   % NHGRI: "…"` — does not start with `%`, so scoping the
  screen to header blocks made it invisible. Two files shipped carrying the repaired sentence in
  their header and the flattened one three lines below on a row, contradicting themselves; review
  caught both. With the wider scope that screen reports **zero across the whole stdlib**, and so does
  the double-quote value screen.

  **That is not the same as saying every shipped value is verbatim, and an earlier draft of this
  entry conflated the two.** Both screens key on a character class chosen in advance. A class-general
  check found four values that neither could see — `earth-science/atmosphere-layers:103`,
  `geometry/quadrilateral-types:116`, `geometry/triangle-alt-name:57`, `geometry/triangle-types:95`.
  Three of them substitute **ASCII single quotes for the page's ASCII doubles**: no curly glyph
  anywhere and no ASCII double in the value, so the apostrophe screen and the double-quote screen are
  both blind by construction. That is a seventh distinct blind spot on a seventh axis, and like the
  six before it, review found it and I did not.

  Those four are **not repaired here**, because none is verbatim even after the quote substitution —
  each differs from its page by more than quote characters, and `atmosphere-layers` also truncates
  the sentence. Curling them would make the visible defect vanish while the citation still failed its
  page: the same reasoning that left the 17 `language/*` composed spans alone. Filed to #14111 for
  re-grounding (#13934), along with the observation that the durable fix is class-general —
  normalize every quote codepoint to one sentinel and compare raw bytes — rather than another
  per-class screen.

  **Two of the header syncs needed ASCII double quotes, not curly ones.** `anatomy/brain-parts` and
  `anatomy/tooth-parts` cite pages that really do use ASCII doubles there; curling them would have
  replaced a correct quote with a wrong one. Each header was therefore synced to *the bytes of the
  value its own file ships*, after confirming that value appears verbatim on its page — never to
  whatever glyph the pattern suggested. This is 3a's NASA lesson arriving a second time.

  **`physics/newton-laws` quoted three law labels and only one was repaired at first.** NASA renders
  all three with U+2019; the value repair touched the First Law, and the header block — which asserts
  the labels are "quoted here character-for-character so any reader can re-check it" — kept ASCII for
  the Second and Third on four lines. Neither screen could see it, because laws 2 and 3 are not
  shipped values. Fixed, and worth stating plainly: a file can assert character-for-character
  fidelity and be right about one third of its own quotes.

  **The `vitamin-deficiency-symptom` pin queries `vitamin_a`, not `vitamin_d`.** That table ships one
  envelope for five rows, and the envelope is the xerophthalmia/night-blindness sentence — which
  grounds `vitamin_a` and not `vitamin_d`. The first version of this pin bound a bone-deformity
  answer beside a citation about vision and would have frozen that pairing in a test. The entry
  claimed each envelope had been checked to ground its row; that check was mechanical — it asked
  whether the pin *contained* the repaired sentence, which is a different question. #14124.

  **`vitamin-deficiency-symptom` is in this batch only because a swallowed error was made loud.** 3a's
  collector could not complete TLS to its host, caught the exception, and moved on, so an
  **unchecked** site was indistinguishable from a checked-and-clean one. Once fetch failures were
  printed, it came back reachable *and* flattened.

  **Two pre-existing pins failed, correctly.** `cell-division-daughter-cells` and
  `cell-division-genetic-outcome` each asserted the whole citation with an ASCII apostrophe — right in
  form, faithfully defending a value absent from its own page. Repointed. Third occurrence this
  effort, after `electoral-college-count` and `mixture-types`. Both libraries quote the *same*
  genome.gov sentence, so one page's glyph governed two libraries' citations.

  **Two pins span the bindings through a corroboration**, because those repairs were on `cites` lines
  and corroborations sit past the envelope's `trust` field where an envelope-shaped pin cannot reach.
  Each was checked to pin the corroboration carrying the repaired sentence rather than a neighbour: a
  corroboration pin bound to the wrong entry is unique, anchored, and tests nothing, which happened
  once already this effort.

  Five files beyond the six-value batch were repaired: the three header-sync files
  (`anatomy/brain-parts`, `anatomy/tooth-parts`, `biology/animal-habitat` — the first two needing
  ASCII doubles, the third U+2019), plus `biology/vitamins` (the parent `vitamin-deficiency-symptom`
  was derived from: the child was fixed and the parent left flattened) and
  `biology/abo-genotype-phenotype`, which quotes a genome.gov sentence this installment repaired
  elsewhere.

  `physics/newton-laws` needed one more pass after that: its header quotes the NASA page **title**,
  which was still ASCII inside a block asserting character-for-character fidelity. Neither screen
  could see it — a page title is not a shipped value and not a same-file desync.

  533 test binaries / 1641 tests green; clippy `-D warnings` clean; `adj_stdlib_report
  --fail-on-unreferenced-tests` and `adj_stdlib_manifest --validate-json-schema` both exit 0; all six
  `.query.adj` companions parse and run clean (`cell-division-genetic-outcome.query.adj` has no
  abstention query, so "abstains correctly" is vacuous for it rather than true).

  *Still open, and not glyph work:* the 17 composed-span `language/*` sites (#14111) differ from their
  pages by more than glyphs and need re-grounding (#13934) — curling them would be a defect wearing
  the appearance of a fix. `geography/map-type.adj:57` ships a paraphrase as a verbatim citation,
  which no glyph screen can ever see because the value has no apostrophe. #14124 remains.

