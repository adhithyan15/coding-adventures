- **#14986: `biology/blood-cell-types.adj` -- each blood cell stops being warranted by the red-cell sentence.**
  The envelope was the `red_blood_cells` span, so asking what platelets do returned `clotting` evidenced by
  *"Red blood cells (RBC) deliver oxygen from your lungs to your tissues and organs."* -- a sentence about
  red cells. The other two spans sat in the header comment block, reaching no answer.

  **The file stated its own defect.** Its provenance paragraph read: *"The `source` span below is the
  VERBATIM sentence for the first data row (red_blood_cells)."* Accurate, and it *was* the defect. That
  paragraph is rewritten, with the old wording quoted in place because it was true when written.

  ### The envelope is an enumerating frame, and that shape is already established here

  The new envelope is the page's sentence naming the three solid components together. It names **all
  three** row keys -- which is not the rule I had been applying ("the envelope must name no row key"), so
  I checked for precedent before shipping it rather than arguing a variant into existence. Three
  already-converted tables ship an envelope naming *every* row key:

  | table | rows | envelope names |
  |---|---|---|
  | `biology/rainforest-layer.adj` | 4 | 4 |
  | `earth-science/atmosphere-layers.adj` | 5 | 5 |
  | `astronomy/planets.adj` | 8 | 8 |

  `rainforest-layer` states the rationale inline, and this table inherits it rather than re-deriving it:
  *"The envelope is the FRAMING span -- what defends the table as a whole ... Until #14986 it was the
  EMERGENT row's own sentence, so a recall of the forest floor was warranted by a sentence about the
  treetops."*

  So the principle is **not** "name no row key". It is: **the envelope may ENUMERATE the keys; it may not
  STATE any of the per-row facts the table asserts.** This table maps cell type → *function*, and the
  envelope states no function at all -- measured, not asserted: none of
  `oxygen/deliver/infection/fight/immune/clot/clotting/wound` is a token of it. A test arm pins that.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of MedlinePlus "Blood", inline tags removed without
  inserting a space, only ASCII whitespace collapsed.
  - The envelope and all three row spans occur **exactly once** on the page and **zero** times against the
    404 control -- and this host gives a real one: a nonsense path returns **404 with a 0-byte body**,
    stronger than the LibreTexts control in shard 03570, which returned a full error page.
  - **Zero U+00A0 anywhere** on the page or in the file. Checked up front rather than discovered mid-way,
    because shard 03570 shipped an NBSP envelope and three display layers rendered those bytes as ordinary
    spaces.
  - Each row span names its own cell type in whole words and states the content its atom compresses.

  ### Negative arms name whole spans

  Measured: no span is a substring of another, but **all three share "blood"**, and the two cell rows also
  share **"cells"**. No single word is exclusive to any row, so every negative arm names a WHOLE SPAN --
  the hazard `plant-parts` has with "roots" and `mixture-types` with "salt"/"water", reached by a fourth
  route.

  ### Pins

  - **Inverted:** `contains("medlineplus.gov/blood.html") && contains("\"trust\":\"authoritative\"")` --
    satisfied by any MedlinePlus citation, constraining no sentence text, and before this change satisfied
    by the red-cell span riding on every answer, including the platelet one the same test binds. Now each
    answer is pinned to its own whole citations array, closing on both the corroborations `]` and the
    citations `]` (#14735), as **separate** assertions rather than one `&&`.
  - **Added:** a per-cell loop with whole-span negative arms; a shape test carrying the enumerating-frame
    rule as **two** assertions -- the envelope must state no per-row function, *and* it must name every
    row key -- and a span-supports-its-own-row test. The second of those two arms exists only because a
    mutant survived the first; see below.
  - **The span test is anchored on the ROW HEADER**, `row ({cell}, {function}) {{\n        source "{span}"`,
    not on the bare `source` line. The weaker needle proves a span sits among the eight-space source lines
    *somewhere*, not that it belongs to that row -- on `mixture-types` a mutant swapping two rows' spans
    satisfied it completely.
  - **Scope stated in the failure text.** The shape test's "no `cites` at any indent" and "exactly one
    `locator`/`trust`" arms pin a convention local to *this* table, not a language rule: `lower.rs`'s row
    path accepts `Source`, `Locator`, `Trust`, `Cites` and `Quote`, and shipped tables use them -- ~~18 have
    a row-level `cites`, 12 a row-level `locator`, 5 a row-level `trust`~~ (**superseded; see below**).

    *Re-measured 2026-09-16 (#15378). Those three figures carried no predicate and no denominator, and
    two had gone stale: **19** tables carry a row-level `cites`, **13** a row-level `locator`, and **5** a
    row-level `trust`. Over 362 fact files matching `adj-facts-stdlib/**/*.adj` with `CHANGELOG.d` and
    `*.query.adj` excluded, holding 350 `table` blocks, classified by ENCLOSING CONSTRUCT rather than by
    indent.*

    ***The `locator` figure needs its predicate spelled out, because the obvious re-derivation gives a
    different number.*** Here a row-level `locator` means **a `locator` annotation of the row itself** —
    the keyword OPENING a row-level line — which is the thing the "a row locator means NOT the envelope's
    page" convention governs. It does **not** mean the `locator` that ADJ-A9 requires on a row's own
    `cites`, which rides at the end of the `cites` line. Count any `locator` token appearing on a
    row-level line instead and the answer is **31 tables / 196 lines**, because all 86 row-level `cites`
    carry one by mandate; 196 − 86 = 110, which is exactly the row-level `locator` line count. Same
    corpus, same day, two defensible readings, eighteen tables apart. A reviewer raised this as the one
    figure in this change still lacking a predicate — in a change whose whole thesis is that a number
    without one rots.*

    *That 196 was re-derived here rather than taken from the review, and the first attempt got **197**:
    the scan counted `%` comment lines inside `row` blocks that merely mention the word `locator`.
    Excluding comments, the identity closes exactly — 196 − 86 = 110 = the A-reading — and no row-level
    line carries a `locator` token that is neither the keyword nor part of a `cites`. A count taken from
    a reviewer is not a measurement either.*

    ***The 5 is unchanged, and is left standing rather than restated as if freshly corrected.*** Rewriting
    a number that was already right, to make a paragraph look uniformly re-measured, is the same defect as
    inheriting a wrong one — it destroys the reader's ability to tell which figures actually moved. Only
    `cites` and `locator` moved, each by one table.

    *This entry says **tables** while the sibling corrections say **files**. Both are correct here and the
    distinction was checked rather than assumed: tables-carrying-the-attribute and files-carrying-it are
    equal for all three (19/19, 13/13, 5/5), because no file that carries row-level provenance holds more
    than one table. The noun is left as it was written.*

    *Counted as LINES rather than tables these are 86, 110 and 10 — nothing near 18/12/5, which is how the
    denominator was identified rather than guessed. The line-level account reconciles as 86 row plus 122
    table plus 1 rule = 209 `cites` corpus-wide; shard 03620 published 87 for the row bucket and now
    carries a dated retraction.*

    *That sentence is spelled out in words rather than symbols because the wrapped form began a line with
    `+ 1 rule`, which CommonMark renders as a new list item — splitting the equation and handing the reader
    "86 + 122" as the total. A number can be destroyed by the markup around it as easily as by a bad count.*

  ### 10 of 10 — and one of the kills was bought by a mutant that survived first

  Both controls green, both files byte-identical after restoration, and an anchor precheck ran *before*
  the mutants so no mutant could be refused silently (a refusal shrinks the denominator without saying
  so). The first run scored **8 of 10, with one survivor and one refusal**; both are worth more than the
  final number.

  **The survivor found a hole in the enumerating-frame rule, and paid for a new assertion.** The rule has
  two halves — the envelope *may* enumerate the keys, and *may not* state a per-row fact — and I had
  pinned only the prohibition. A mutant replacing the envelope with the page's own *"Over half of your
  blood is plasma."* (real page text, x1, zero functions, **zero row keys**) satisfied that arm and
  **survived the whole suite**: the frame stopped framing the table and nothing noticed. The fix is a
  second arm requiring the envelope to name every row key, verified non-vacuous in both directions before
  re-running — it passes on the shipped envelope and fails on the plasma mutant. **The precedent tables do
  not pin this either**, so this arm is an improvement on them rather than a convention copied from them.

  **The security review then tightened that new arm, and the tightening is itself demonstrated.** As first
  written it checked each underscore-separated *part* (`white`, `blood`, `cells`), which scattered tokens
  anywhere in the sentence would satisfy — so an envelope carrying the right words in an unrelated
  arrangement would have passed while naming nothing. It now requires each row key as a **contiguous
  phrase**. Checked against a deliberately constructed decoy — *"Blood is white, cells are red, and blood
  cells vary; platelets too."* — the old arm **passes** it and the new arm **rejects** it.

  **The refusal was a duplicate anchor, caught by the precheck rather than by luck.** The `trust`-downgrade
  mutant anchored on `\"trust\":\"authoritative\"`, which occurs **twice** in the test — once inside
  `only_citation()` and once inside a comment. Re-anchored on the unique surrounding fragment so the
  mutant actually runs.

  It's a local scratch harness, so the count can't be reproduced from the repo.

  ### The query example, reported as it runs

  4 queries, 3 answers, 1 abstention (`neuron`); **3** citations arrays, **0** non-empty corroborations,
  and the envelope's wording in **zero** answers. Note **6** empty-corroboration occurrences against 3
  answers, because provenance renders on **two** surfaces -- `citations` and the steps array's
  `"kind":"fact"` entries. `only_citation()` pins the first; the second is guarded only by the cross-span
  negative arms.

  The cross-span arm was checked for vacuity rather than assumed non-vacuous: the needle it looks for
  (the platelet binding followed by the red-cell source) scores **0** in correct output, while the shape
  it mirrors -- the platelet binding followed by the platelet source -- scores **1**. It can pass now and
  it can fail on a real defect.

  The query companion's "the table's source/locator/trust" line is re-pointed, with the old wording quoted
  in place because it was accurate before this conversion. The README has **no** row for this table --
  checked against the 57 `biology/` rows it does have, not assumed from an empty grep. The header's
  retired "carrying the table's citation" boilerplate (#15336) is fixed here too; it wrapped across a line
  break, so a grep for the phrase on one line finds nothing.
