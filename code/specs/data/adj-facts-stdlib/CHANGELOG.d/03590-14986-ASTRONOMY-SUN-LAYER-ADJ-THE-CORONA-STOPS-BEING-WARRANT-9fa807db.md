- **#14986: `astronomy/sun-layer.adj` -- the corona stops being warranted by the photosphere sentence.**
  The envelope was the `photosphere` span, so asking what the corona is returned
  `the_suns_outer_atmosphere` evidenced by *"The Photosphere – the visible surface of the Sun."* -- a
  sentence about a different layer. The corona's own sentence sat in the header truth-table, reaching no
  answer.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of NASA's "Layers of the Sun" (The Sun Spot), inline
  tags removed without inserting a space, only ASCII whitespace collapsed.
  - Both row spans and the envelope occur **exactly once** on the page and **zero** times against a 404
    control on the same host.
  - The envelope is the page's framing sentence, *"The Sun and its atmosphere consist of several zones or
    layers."*, which names **neither** layer -- measured, not assumed: neither `photosphere`, `corona`
    nor `chromosphere` is a token of it. A test arm pins that.
  - Each row span names its own layer as a whole word and states the content its atom compresses.

  ### Both row spans carry non-ASCII bytes, and the ASCII spellings occur ZERO times

  The photosphere span's dash is **U+2013**; the corona span carries **U+2013** *and* the **U+2019** in
  "the Sun's". Counted on the fetched page, the ASCII-hyphen/ASCII-apostrophe spelling of each occurs
  **zero** times, so shipping one would cite a sentence the source does not contain (#15324's defect for
  `soil-texture-class`, and the U+00A0 case in shard 03570). Both spans are **extracted from page bytes**
  rather than retyped, and the test consts use `\u{2013}` / `\u{2019}` escapes rather than literal bytes.

  That escaping is not decoration: a terminal, a ripgrep result and a diff view **all** render U+2013 as
  `-` and U+2019 as `'`. In shard 03570 a ripgrep result showing an NBSP envelope as ordinary spaces was
  indistinguishable from the defect being repaired; only `repr()` on the file settled it.

  The page also carries **U+00B0** (in the chromosphere sentence the header quotes) -- the same
  degree-sign hazard that keeps `geometry/angle-types` parked. It sits in a comment here, not in shipped
  provenance, so it is left alone.

  ### Negative arms name whole spans

  Measured: neither span is a substring of the other or of the envelope, and the two share only "the" and
  "sun". So no short needle is exclusive to either row, and every negative arm names a WHOLE SPAN.

  ### Pins

  - **Inverted:** `contains("science.nasa.gov") && contains("\"trust\":\"authoritative\"")` -- satisfied
    by any NASA citation, constraining no sentence text, and before this change satisfied by the
    photosphere span riding on the corona answer too. Each answer is now pinned to its own whole
    citations array, closing on both the corroborations `]` and the citations `]` (#14735).
  - **Added:** a per-layer loop with whole-span negative arms; a dash/apostrophe test in two scopes
    (positive arm reads the eight-space row lines, negative arm any indent); a shape test pinning that
    the envelope names no layer; and a span-supports-its-own-row test **anchored on the row header**,
    `row ({layer}, {description}) {{\n        source "{span}"`, because the bare-`source` needle proves
    only that a span sits among the row lines somewhere -- on `mixture-types` a mutant swapping two rows'
    spans satisfied it completely.
  - **Kept:** the direct bind, the reverse bind, and the `chromosphere` abstention, with an added arm
    that an abstaining query emits **no** citations array at all.
  - **Scope stated in the failure text.** The shape test's "no `cites` at any indent" and "exactly one
    `locator`/`trust`" arms pin a convention local to *this* table, not a language rule: `lower.rs`'s row
    path accepts `Source`, `Locator`, `Trust`, `Cites` and `Quote`, and 18 shipped tables carry a
    row-level `cites`.

  ### The query example, reported as it runs

  3 queries, 2 answers, 1 abstention (`chromosphere`); **2** citations arrays, **0** non-empty
  corroborations, and the envelope's wording in **zero** answers. Note **4** empty-corroboration
  occurrences against 2 answers, because provenance renders on **two** surfaces -- `citations` and the
  steps array's `"kind":"fact"` entries. `only_citation()` pins the first; the second is guarded only by
  the cross-span negative arms.

  ### Two things checked rather than assumed

  The README **does** have a row for this table (line 97) -- unlike the previous two conversions, where
  it had none. It describes the rows and the `chromosphere` abstention but says nothing about provenance
  shape, so it stays true and is left alone.

  The query companion has **no prose header at all** -- three `?` lines with inline comments -- so there
  was no "the table's source/locator/trust" claim to re-point here.

  ### A retired boilerplate in a spelling the sweep never matched

  This header said each row lowers to a relation *"carrying **the source citation**"* -- the #15336
  boilerplate, but in a second spelling. Fixed here. Measured across
  `code/specs/data/adj-facts-stdlib/**/*.adj` with `newline + %` collapsed so wrapped phrases are found,
  **before this conversion**: **125** files say "the table's citation", **83** say "the source citation",
  union **208** -- against the **104** #15336 records from one spelling matched without wrap tolerance.
  Reported on that issue; five tables already converted in this batch carry the second spelling.

  The same scan *after* this conversion reads **82 / 207**, because fixing this file removed one
  occurrence. The pre-conversion figures are the ones quoted above and on #15336, since that is what was
  measured when the claim was made; the one-file difference is this change.

  ### 10 of 10 — and the tenth kill was bought by a survivor, then by a review

  Both controls green, both files byte-identical after restoration, and an anchor precheck ran before the
  mutants so none could be refused silently. Two mutants attacked the dash/apostrophe arm directly by
  substituting ASCII twins, and both died by it.

  **The first run scored 9 of 10, and the survivor was real.** Replacing the envelope with the page's own
  *"The Convection Zone – the outermost layer of the solar interior…"* — real page text, naming a layer
  that is **not** a row key — passed every arm. The shape test forbade only `photosphere`, `corona` and
  `chromosphere`, and that sentence names none of them. The frame stopped being about this table and
  nothing noticed — the same shape that bought a new assertion in shard 03580.

  **What the security review added.** It observed that the shipped test already pins the *exact* envelope
  string, so an `.adj`-only swap dies immediately; and it proposed a rule rather than a string. Both
  points were checked by running the two mutants side by side:

  | mutant | verdict | killed by |
  |---|---|---|
  | `.adj` only | dies | the exact-envelope pin |
  | `.adj` + the test's `ENVELOPE` const, edited together | **survived** (before this fix) | — |

  The exact-string pin fails on **any** envelope edit, correct or not, so it does not test whether the
  frame is about this table. The co-edited mutant is the one that asks that question, and it survived.

  **The fix is the review's own suggestion, and it is general.** The forbidden-token list now covers the
  page's whole layer taxonomy — `core`, `radiative`, `convection`, `transition`, plus the three already
  there. This is not a new arbitrary list: the test *already* forbade `chromosphere`, which is not a row
  key, so extending to the rest is the same rule applied consistently, and the names are exactly those
  this file's own header enumerates. A framing sentence names the **set**, not a member, so it fails on
  no correct envelope. The co-edited mutant now dies by it:
  *"the envelope must name no layer of the Sun, but contains `"convection"` as a whole word"*.

  So the earlier claim that this gap could not be closed by a general rule was **wrong**, and it is
  corrected here rather than quietly dropped.

  It's a local scratch harness, so the count can't be reproduced from the repo.
