- **#14986: `earth-science/plate-boundaries.adj` -- the convergent and transform rows stop being warranted by the divergent sentence.**
  The envelope was the `divergent` span, so asking how plates move at a convergent or a transform boundary
  returned its motion evidenced by *"Plates rip apart at a divergent plate boundary, causing volcanic activity
  and shallow earthquakes;"* — a sentence about a different boundary type — while those two rows' own
  sentences reached no answer at all.

  ### What each row carries now

  Measured 2026-09-16 UTC against the U.S. National Park Service "Types of Plate Boundaries" page at the
  `locator`, inline tags removed **without inserting a space**, only ASCII whitespace collapsed. This is a text
  measurement, not a byte-provenance check — see the ceiling note below.
  - The envelope and all three row spans occur **exactly once** on the page **under that recipe** — the
    qualifier is load-bearing rather than pedantic, and the next paragraph is why.
  - Each row span names its own boundary type and states the motion its atom compresses.

  **The recipe is part of the count, and here it decides three of the four needles.** Two recipes that both
  collapse whitespace and differ in one thing only — what an inline tag becomes — disagree on this page: send a
  tag to a **space** and the divergent and convergent spans score **x0**; send it to the **empty string** and
  all four score **x1**. The transform span and the envelope score x1 under both. So the tag-to-space zeros are
  the **instrument**, not the page — and the span they would have condemned is the divergent one this file has
  shipped, green, since it was written. Reported because shard 03600 found the same split on a different host
  (SEER) and the naive recipe is evidently not naive-looking enough to stay caught once.

  ### The convergent span was CORRECTED, not merely relocated

  This is not a plain relocation, and the defect was in the header rather than in the table. The header quoted
  the convergent sentence with **ASCII quotes and a terminating period**:

  > "At a convergent plate boundary, one plate dives (\"subducts\") beneath the other, resulting in a variety
  > of earthquakes and a line of volcanoes on the overriding plate."

  That form occurs **zero** times on the page, under **either** recipe. The page writes **curly** quotes around
  *“subducts”* and ends the sentence with a **semicolon**, because there it is item 2 of a three-item list.
  Measured on the tag-stripped page, **under the tag→empty recipe**: curly + semicolon **x1**; curly + period
  **x0**; ASCII + period **x0**; ASCII + semicolon **x0**. The recipe label matters for the **x1** and only for
  the x1 — under tag→space that form reads x0 too, for the reason the next section gives. The three **zeros**
  hold under *both* recipes, which is what makes the ASCII/period form's absence a real absence rather than an
  artifact of how the page was stripped.

  A fragment punctuated into a sentence is precisely the defect
  `plate_boundary_citation_keeps_the_pages_semicolon` exists to catch — and it had been sitting in this file's
  own header the whole time that test was green, because the test pinned the **divergent** citation and nothing
  read the header. The row ships the page's form. **`--ascii-shipped` stays off for this file**: shipping the
  ASCII form would put a `source` on a row that appears on no page.

  ### The envelope must frame plate motion and name no boundary type

  The new envelope is the page's framing sentence, *"The landscapes of our national parks, as well as geologic
  hazards such as earthquakes and volcanic eruptions, are due to the movement of the large plates of Earth’s
  outer shell."* — chosen as candidate 3 of 22.

  `the_table_shape_matches_the_measured_rows` forbids the three row keys **and the motion verbs** (`rip apart`,
  `laterally`, `subducts`, `dives`), because the sharpest mutants this page offers name **no** row key and still
  restate a row fact: *"Volcanic eruptions and shallow earthquakes are common where plates rip apart."* and
  *"Shallow earthquakes and little volcanism occur where one plate slides laterally past another."* Either would
  warrant a row if shipped as the envelope, and a key-based arm passes both.

  **A local rule, not a general one.** #15344 (`blood-cell-types`) settled the general principle the other way
  for an ENUMERATING frame: there the envelope may name the keys and its shape test *requires* them. This
  table's envelope is DEFINITIONAL — it frames what produces landscapes and hazards, naming no type — so
  forbidding the taxonomy is the right pin *here*, and would be wrong for a frame such as *"There are three
  types of tectonic plate boundaries:"*.

  ### What that screen actually guards, which is narrower than it first reads

  **This arm did not exist in the first version of this PR.** The paragraph above was written in the present
  tense describing a screen that had only ever run in a scratchpad harness, which ships nowhere. A security
  review caught it. The arm is shipped now, and the rest of this section is what watching it fire actually
  established.

  The same test pins the envelope by **exact equality** before it reaches the token scan, so while that pin
  stands **and the table has a single table-level `source` line**, the scan cannot fail: any envelope
  satisfying equality trivially satisfies it. The second condition is load-bearing and was missing from the
  first draft of this paragraph — the scan's operand is `lines().find(|l| l.starts_with("    source \""))`,
  the **first** table-level source line, not necessarily the pinned one, so a table with two of them whose
  first restated a row fact would satisfy the equality pin and still fire the scan. This table has exactly one
  (`:135`), measured, so the claim holds as shipped.

  The scan's real regression target is a future author changing the `.adj` envelope **and** the test's
  `ENVELOPE` constant *together* to a row-restating sentence — the screen reads the envelope out of the
  **file**, not out of the constant, so in that case it does fire.

  **Observed firing on both code paths, and observed NOT firing where it makes no claim**, by mutating both
  files together: `restates_divergent_fact` is rejected on the **phrase** path — `rip apart` is two words, so
  the token scan cannot see it — reporting *"must not restate the divergent motion"*; `restates_transform_fact`
  is rejected on the **token** path reporting *"must name no boundary type and no motion verb"*; and
  `enumerating_frame` is **accepted**. So the arm rejects **2 of 5** prepared mutants and does **not** catch
  `hotspot_wrong_subject`, `fingernails_off_subject` or `enumerating_frame` — those are wrong on subject, or on
  a trailing colon that frames the list rather than the subject — and this arm does not pretend otherwise.

  *The first attempt to watch it fire proved nothing, and the way it failed is the same defect one layer down:
  swapping only the `.adj` broke the equality pin first, `assert!` panicked there, and all three mutants
  "failed" for a reason unrelated to the screen — including the negative control, which must pass.*

  ### Pins

  - **Inverted:** `contains("nps.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by the host and
    the tier appearing anywhere in the output, in two unrelated places, binding neither to a source. Before
    this change it was also satisfied by the divergent span riding on the convergent answer. Each answer is now
    pinned to its own whole serialised citation object, closing on `"corroborations":[]` (#14735).
  - **Kept, untouched:** the full anchored divergent pin (#13916, #13918) and
    `plate_boundary_citation_keeps_the_pages_semicolon`. Both are scar tissue from real incidents.
  - **Added, and each observed failing:** three arms — convergent answer, transform answer, reverse bind —
    each pinning its own whole citation object.
  - **Added:** `the_table_shape_matches_the_measured_rows`, the arm 45 sibling test files ship and this one did
    not — three row `source` lines, no `cites` at any indent, exactly one `locator` and one `trust`, the
    envelope contiguous with both, a negative arm that the divergent span is not the envelope again, and the
    token/phrase screen described above.
  - **The same reachability shape applies to the row-span needles, and this entry disclosed it only for the
    envelope.** In `every_row_motion_is_supported_by_the_span_that_row_carries` the row-header `contains` pins
    each span to the file first, so the token and content-needle checks that follow run over the `scale()`
    constants and are constant-folded exactly as the envelope scan is. No claim here depends on those two
    checks firing independently — the arm genuinely fails against pre-conversion bytes on the row-header pin
    (`:331`) — but disclosing the shape for one arm and not the other would have been the same over-claim in a
    quieter form.
  - **Added:** `every_row_motion_is_supported_by_the_span_that_row_carries` (#15318), anchored on the **row
    header** rather than the bare `source` line — the weaker needle proves a span sits among the eight-space
    source lines somewhere, not that it belongs to *that* row, and on `mixture-types` a mutant swapping two
    rows' spans satisfied it fully.
  - **Added:** `assert!(!out.contains(ENVELOPE))` on the recall. Without it the measured *"envelope wording in
    zero answers"* below was unpinned, and the envelope could drift back to a row-warranting sentence with every
    other assertion still green.

  **Each new arm lives in its own `#[test]`, and that is the finding, not a style choice.** Written together in
  one function first, they were run against the pre-conversion file to watch them fail: **one** fired and the
  other **two never executed**, because `assert!` panics on the first failure. Unexecuted is not passing and
  not failing — it is unproven. Split into three tests, all **3 of 3** are observed failing against the
  pre-conversion bytes, each panicking at its own line, while the two pre-existing tests still pass.

  Re-measured after the shape and row-support arms were added, the whole suite against pre-conversion bytes
  reads **2 passed / 5 failed** — the three citation arms plus both new structural arms, each panicking at its
  own line (172, 182, 195, 234, 331). Both structural arms fail there for the right reason: the unconverted file
  carries **no row-level `source` at all**. The `.adj` was restored and verified byte-identical by sha256 after
  every swap.

  That the old tests still pass is itself the measurement that justifies this PR: the shipped suite scored
  **2 passed / 0 failed** on the unconverted file **and** on the converted one. It could not tell them apart.

  ### The query example, reported as it runs

  5 queries, 4 answers, 1 abstention (`equator`); **4** citations arrays, **8** empty-corroboration occurrences
  and **0 non-empty**, envelope wording in **zero** answers. Span occurrences move from
  **divergent x8 / convergent x0 / transform x0** to **x2 / x2 / x4**.

  **Transform reads four, not two, and the four is correct.** It is the only row bound **twice** — once
  forward, once by the reverse query on `slide_past` — and each answer renders its citation on two surfaces
  (`citations`, and the steps array's `"kind":"fact"` entry). 4 binding queries × 2 surfaces = 8 occurrences
  either way; the conversion changes **which** span fills them, not how many there are.

  *A prepared note for this file predicted **2 / 2 / 2** and called it "the cleanest before/after this batch has
  produced". That prediction was written ahead of its measurement and is retracted here: it had pattern-matched
  the previous shard's "each span 2×" without accounting for this table's reverse bind. The measured figures are
  the ones above.* There is likewise **no corroboration improvement to claim** — unlike shard 03620, non-empty
  corroborations measured **0 before and 0 after**.

  ### The retired boilerplate

  This header said each row lowers to a relation *"carrying **the table's citation**"* — the #15336 spelling.
  Re-measured today over `code/specs/data/adj-facts-stdlib/**/*.adj` (723 files, `CHANGELOG.d` excluded, with
  `newline + %` collapsed so a needle survives comment wrapping), **after** this conversion:
  `the table's citation` **123**, `carrying the citation` **91**, `the source citation` **82** — sum **296**,
  union **295** distinct files. `plate-boundaries.adj` now matches **none** of the three spellings.

  Sum and union differ by one because exactly one file carries two spellings; both are stated because a review
  once read the union as a failed addition.

  ### What this table claims, and its ceiling

  Nothing in it carries a `quote … at <byte_offset> snapshot "<sha256>"` annotation — the repo's byte-provenance
  mechanism. The spans are labelled with the page they were read from and nothing more, so **this table claims
  no more than `source_labeled`**, and the header says so.

  `gaps.missing_byte_pin` and `gaps.missing_pin_syntax` both list this file. Measured on the pre-conversion and
  post-conversion trees: **584 and 584** in each gap, identical — Wave 1 state, pre-existing, **unchanged by
  this PR**.
