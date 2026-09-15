- **#13934 installment 4g: five values that were a real paragraph with a lesson table welded onto
  it.** 5 value lines, 37 row lines and 242 comment lines (added, vs origin/main) across **5** `.adj` files —
  `language/digraph-sound`, `diphthong-sound`, `silent-letter-sound`, `long-vowel-team-sound` and
  `other-vowel-team-sound` — plus five README Source cells and five e2e test files. **Five mutations
  redden.**

  **The defect.** Each value was the cited page's own prose paragraph with a hand-assembled
  rendering of the unit's lesson table stitched onto it:

  ```
  A consonant digraph is … speech sound. Digraphs Unit Resources (Lessons 42-53): 44 ck /k/, 45 sh …
  ^-- block 26 minus its last sentence --^ ^--------- heading and table cells, welded ---------^
  ```

  No page contains that string. Across the five values the weld invented a colon after the unit
  heading, invented a separator between each lesson number and the cell before it, **dropped 5
  closing sentences**, **flattened 5 of the page's U+2019 apostrophes to ASCII** — the
  quote-flattening class installment 4e repaired elsewhere, still shipping here — and in two files
  **inserted three spaces the page does not have**: lessons 85, 86 and 91 each break the line inside
  the Concept cell after its second comma, so the page reads `ea /ē/,ey /ē/` and the value wrote
  `ea /ē/, ey /ē/`. (Security review caught that count too: the first draft named lesson 85 and
  stopped, while the file's own cell listing three lines below had always shown 86 correctly.)

  Each header now states **its own** file's defects rather than the class's union. The first draft
  wrote one paragraph and installed it in all five, so four of them claimed something their own old
  value had not done — `diphthong` and `silent-letter` dropped nothing, `digraph` and
  `long-vowel-team` flattened no apostrophe. Writing the union and letting each file wear it is the same
  move as calling a stitch of separate cells "verbatim": close enough to persuade, wrong in the
  specifics.

  ### Why this is repairable when the rest of the band is not

  Every `source` is now the page's paragraph, byte for byte, **pulled live off the page rather than
  retyped** — 4f's rule, earned when a hand-typed NBSP shipped a fresh instance of the defect being
  repaired. That rule was necessary and, as the section below records, **not sufficient**: pulling
  live off the page through a normalising extractor is not pulling live off the page. Total value
  length goes 1857 → 1819 characters.

  That span grounds the unit's DEFINITION. It grounds **no row**, and the repair does not pretend
  otherwise: the rows read the lesson table's "Concept" column, whose cells extract as separate
  blocks — the cross-block/table-row question held open on #14111. So each row now **names the cell
  it reads**, quoted live from the page, and each Provenance block says which of the two carries
  what. That is installment 4c's solfège pattern: **disclose the composition instead of deciding,
  by shipping, whether a table row is a verbatim span.**

  This is the distinction that makes 4g exist at all. A composition presented as a quotation is a
  defect in the FIELD, not a position on the held question — removing it takes no position either
  way.

  ### Four shipped pins asserted the fabricated string, exactly

  Four of the five e2e tests CARRIED a FULL ANCHORED CITATION PIN (the fifth, `digraph-sound`,
  gained one during security review) — anchored on the `"source":"` key,
  closing on the terminating quote, so head, tail, punctuation and length are pinned at once. That
  design came out of #13916/#13918, where a fragment needle let a citation be truncated while the
  test stayed green.

  It worked exactly as designed: the moment the stitch was replaced, `diphthong_sound_recall_binds_
  the_sound_with_citation` went red and named the byte. Worth being plain about what it was pinning,
  though — the unit heading, the invented colon, the lesson-number stitches and the ASCII
  apostrophe, all asserted "exactly". **A pin cannot tell a faithful value from a fabricated one; it
  can only tell you the value changed.** Being fully anchored is what makes it useful, and it is
  also what made it a faithful record of a defect.

  Each is re-anchored on the repaired value, taken from the `.adj` file rather than retyped. The
  five NEW pins are built differently on purpose: a positive needle that is **discriminating** (a
  sentence the stitch dropped, or the U+2019 the stitch flattened — either fails on the old value)
  and negative needles that **span the seam** (`speech sound. Digraphs` joins the last words the
  stitch CARRIED — mid-paragraph in three of the five, since those three dropped the closing
  sentences — to the heading welded after them). A needle wholly inside either side would still pass a
  half-undone repair.

  ### The extractor launders the byte the rule exists to protect

  4f's rule is *carry the page's character, never retype it*. Pass 1 obeyed the letter of it — every
  value was pulled live off the page — and **two of the five still shipped ASCII spaces where the
  page has U+00A0**, because it pulled them through `extract_v4.blocks()`, whose last line is
  `.replace("\xa0", " ")`. "Pulled live from the page" turned out to mean "pulled live from the
  thing that normalises the page".

  Raw HTML, by curl: `toolbox-aus/42-53` carries **three** U+00A0 inside the paragraph —
  `consonant<NBSP>speech sound.<NBSP> The lessons … with<NBSP>consonant digraphs` — and
  `toolbox/84-88` one. In both, `sound.<NBSP> The` is **two** whitespace characters where the value
  had one. The other three pages carry none inside their paragraphs and were already exact: a
  two-file defect, not a policy.

  **No screen here could see it.** The contiguity sweep compares against the extractor's output, so
  it normalises both sides; the five new pins compare the CLI's output to the `.adj` field — the
  field to itself — and never to the page. Security review caught it, as it caught the same class in
  4f. Both values are now taken from the RAW HTML: the block text becomes a whitespace-insensitive
  pattern, is matched against tag-stripped unescaped HTML, and **the matched slice is what gets
  written**, so every whitespace character is the page's own. All five verified byte-exact against
  raw HTML with a negative-control arm that fails.

  Everything quoting those two fields was re-synced, because leaving one behind is exactly how 4f
  shipped a fresh instance of the defect it was repairing. The header quotes render each U+00A0 as
  the visible notation `<U+00A0>` and say so — a literal no-break space in a wrapped comment is
  invisible to a reader and a break opportunity to a rewrapper, the two ways this byte has already
  gone missing. A new mutation harness flattens **only** the U+00A0 and requires a red test: 2/2
  killed. `digraph-sound` also gained the full-anchored `source` pin its four siblings had and it
  lacked.

  ### Round 2: a needle that is not absent from the page, and a test backwards about why

  Round 2 confirmed all five values byte-exact against raw HTML, and found the audit trail wrong in
  SIX more places. The sharpest: every pin claimed its two non-seam negative needles were
  "artifacts the stitch invented outright — neither of which occurs anywhere on the page". Measured
  on all four pages, the colon needle is indeed absent under any normalisation, but **the
  lesson-number needle is not**: normalise the page's whitespace and `44 ck /k/` appears, because
  the lesson cell and the Concept cell are adjacent. What the old value invented was that string as
  a **contiguous** span with the cell boundary erased — still a real defect, still discriminating,
  but a different claim.

  The stated methodology was backwards too. For an **absence** claim, tag-bearing raw HTML is the
  *weaker* test, since embedded markup guarantees a non-match; raw HTML is the right tool for a
  **presence** claim, which is why round 1 reached for it. Reaching for it here inverted the
  reasoning and called the weaker check the stronger one.

  Also: one test doc quoted the pre-4g field as `84 ai /a/, ay /a/` where it held `84 ai /ā/` —
  **non-ASCII flattened to ASCII inside a quotation, in the documentation of the repair for exactly
  that class.** Two ledgers concluded "it never reached an apostrophe, so it flattened none" while
  omitting the U+00A0 each *had* flattened. And `diphthong-sound`'s header said its paragraph "names
  no spelling and no sound" eight lines below quoting `(e.g., /ow/, /oy/)` — it names two sounds; it
  just pairs neither with a spelling, which is the claim that actually carries the argument.

  ### A screen that reported a clean bill of health having examined nothing

  The post-repair sweep was first launched from the scratchpad rather than the repo root. Its file
  glob is relative, so it matched no libraries, and it printed **`NOT CONTIGUOUS: 0`** and exited
  **0** — a perfect result from a sweep that examined nothing, in the same shape as a cargo filter
  matching zero tests and printing "test result: ok". The sweep script now refuses to report on
  fewer than 200 libraries — the guard the scratch-tag screen has carried since 4d.

  **A caveat that belongs next to every figure in this entry**: these screens are working-tree
  instruments, not repository guards. They live in a scratchpad, so "now refuses" is not something
  a reader can go and watch, and the sweep totals below are reported from them rather than
  reproducible from a commit. Whether they should be committed under the stdlib's tooling is
  filed as #14444 rather than decided inside a content fix.

  **The re-run was then thrown out too**, for a different reason: it ran CONCURRENTLY with edits to
  the very files it was sweeping. It reported `other-vowel-team-sound` non-contiguous; re-screened
  after the edits settled, the same value is verbatim. That is 4f's "the population moved underneath
  the number", re-committed by sweeping and editing at once. The published figures come from a third
  sweep, run only after the files stopped changing.

  **And the set-diff itself was wrong twice.** Keyed on `file:line`, it reported all five repaired
  values as having *left* the set and one as having *entered* it — but every header here grew by
  dozens of lines, so four had merely moved down the file. *Keying a diff on the line number keys it
  on something the repair itself changes.* Re-keyed on file, field and the value's own opening, it
  then held **64** members where the sweep counted **65**: two values sharing an opening collapsed
  into one, a member quietly deleted by the instrument meant to notice members leaving. It now
  carries an occurrence index.

  Third sweep, from the root, on stable files: **563 values examined, 60 non-contiguous**
  (from 65), 36 unjudgeable, 23 fetch-failed. The five that left the set are the five repaired here,
  member for member, and **none entered** — diffed as SETS, never subtracted as totals.

  ### What 4g leaves behind, and why it is the last easy one

  The 23 "agreed-actionable" values were read individually rather than treated as one class, and
  they are not one. **Eight are multi-block cell assemblies.** Of the remaining fifteen, **six** are
  this installment's shape (five shipped here; `geometry/shape-composition:57` is the sixth and
  waits, being two MathWorld definitions concatenated with an authored third sentence — a different
  page and a different weld). **Two** are prose *describing* a table ("MathWorld's Platonic Solid
  table gives, for each solid, its number of vertices: …") sitting in a field that claims to quote.
  **Three** are table dumps — `chemistry/atomic-weights:73` ships
  `6\tC\tcarbon\t 12.011 ± 0.002    8\tO\toxygen\t 15.999 ± 0.001`: two element rows whose FIELDS are joined by
  tab escapes, which the reader turns into real tabs, the two rows themselves separated by four
  spaces. Round 3 corrected that description twice over.
  The first draft rendered it `6tCtcarbont`, silently dropping the backslashes — **a non-verbatim
  quotation inside the entry documenting a repair for non-verbatim quotation** — and then
  explained it as "tab characters flattened into a literal `t`", which is not what the field does
  at all. **Three** are rendered math the extractor
  drops, and **one** is an invented `|` separator.

  Groups two and three are the same defect as this one and repairable the same way. The rest waits
  on the owner's three questions.

  ### The review pattern, which outlives this installment

  Security review ran **four rounds**. Round 1 found a real data defect (the NBSP). Rounds 2, 3 and 4
  found only documentation-accuracy defects — but two of those were *caused by the previous round's
  fix*: round 2's rewriter deleted the subject of a sentence in four files, and round 3's
  singular/plural splice said digraph dropped two closing sentences when it dropped one.

  The through-line is sharper than any single fix. Three separate rounds reported the class of "prose
  crediting `source` with content only the lesson table holds" **swept**, and each time the next round
  found more — because each fix was a LIST of the sites the reviewer named, not a SWEEP of the class.
  It terminated when the examples ran out, and was reported complete. Round 4 ends with an actual grep
  over all eleven touched files. It reported eight survivors: two real, and six where the screen had
  matched "source" inside "Re**source**s" — a screen for a word that could not tell a word from a
  substring of another word.

  *When review names instances of a class, fix the class and prove it with a screen. A fix that stops
  when the named examples run out will be reported as complete and will not be.* And a standing correction to how that list was described:
  **"agreed-actionable" has meant "no screen objects", not "verified as ordinary work"** — the
  reading is what tells them apart, and it took reading all fifteen to find that only six were this.

