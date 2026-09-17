- **#14986: `biology/plant-parts.adj` — the stem, leaves and flower stop being warranted by the roots sentence.**
  The envelope was the `roots` span, so asking what a leaf does returned `photosynthesis` evidenced by
  *"Roots anchor a plant in the soil and absorb air, water, and nutrients."* — a sentence about roots.
  The other three sentences were header prose that reached no answer.

  **The file stated its own defect.** Its provenance paragraph read: *"The `source` span below is the
  VERBATIM sentence for the FIRST data row (roots)… the other three rows' atoms are each grounded in
  the same page's verbatim sentence shown in the table above."* That paragraph is rewritten, not left
  standing beside contradicting text.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of the UF/IFAS Extension "Organs" article, with
  inline tags removed without a space and only ASCII whitespace collapsed.
  - **Each row takes the sentence stating its own function.** Each occurs **exactly once**, inside one
    `<p>`, scripts set aside and over the whole file, with no `<head>` copy.
  - **The envelope** is the page's sentence framing the organs as a set — *"Each of the plant's organs
    play a vital role in the plant's survival."* — once, in one `<p>`, naming no part as a whole word.
  - **There is no 404 control for this host.** A nonsense path answers **200**, not 404, with a real
    63 918-byte page against the article's 100 998. So absence is **content-based only**: each span
    scores zero against that page rather than against an error stub. Stated plainly because every
    other conversion in this batch had a 404 to point at.

  ### The envelope's apostrophes are U+2019

  The page writes "plant's" with U+2019. Counted on the fetched page: that spelling occurs **once**,
  and the same sentence with ASCII apostrophes occurs **zero** times — so shipping the ASCII form
  would cite a sentence the source does not contain (#15324's defect for `soil-texture-class`, and the
  one `volcano-type` and `metamorphism-cause` both shipped before their conversions). The envelope is
  extracted from the fetched page rather than retyped.

  ### The stem span names four row keys, which constrains the tests

  Measured, not assumed: the `roots` span names `{roots}`, `leaves` names `{leaves}`, `flower` names
  `{flowers}` — but **`stem` names `{roots, stems, leaves, flowers}`**, because the page writes
  *"…transport water and nutrients between roots, leaves, and flowers."* So a bare part-name needle is
  exclusive to no row, and **every negative arm names a WHOLE SPAN**. Same hazard as `comet-part.adj`'s
  shared "nucleus", reached by a different route.

  ### Pins

  - **Kept:** the forward bind, the backward `photosynthesis` bind, and the `mushroom` abstention.
  - **Inverted:** `contains("blogs.ifas.ufl.edu") && contains("\"trust\":\"authoritative\"")` — satisfied
    by any UF/IFAS citation, constraining no sentence text, and previously satisfied by the roots span
    riding on every answer. Now each answer's own whole citations array, closing on both the
    corroborations `]` and the citations `]` (#14735), as **separate** assertions rather than one `&&`.
  - **Added:** a per-part check; the apostrophe test; a table-shape test (four row sources,
    keyword-anchored `cites` absence, exactly one `locator` and one `trust`, whole-word part-name check
    on the envelope); and a value-supported-by-its-own-span test (#15318).

  ### 10 of 11, with the survivor named — and a claim of mine that was falsified

  **10 of 11 mutants are killed by their named assertion**, both controls green, both files
  byte-identical. The eleventh **SURVIVED**, and it is kept in the set rather than deleted, because a
  surviving mutant is a finding.

  **What I claimed, and why it was wrong.** Three variants each reached the U+2019 arm only by
  tripping something else: swapping the envelope's apostrophes dies on arm *one* and the
  verbatim-block arm; planting the ASCII variant as the flower row's source also trips `flower carries
  its own span`; adding it as a fifth row trips `four row sources`. From those three I wrote that the
  arm was **"not isolable by a `.adj`-only mutation"**.

  A security review produced a fourth route, and **I reproduced it before accepting it**: a `%`
  **comment** inside the table block carrying the ASCII envelope. Comments are lexer-skipped, so the
  CLI answers identically and every shape assertion passes — exactly **one** test failed, on the
  U+2019 arm's own message. **Three failed attempts are not a proof of impossibility.**

  **What the survivor now tells me.** Acting on the same review's corollary, the U+2019 arm is now
  **scoped to `source "` lines** rather than reading the whole table block — because a `%` comment is
  not a shipped citation, and failing on one is a false alarm. Under the scoped arm that mutant
  survives, which is correct behaviour and also an honest gap: **nothing now pins "no ASCII envelope
  anywhere in the table block"**, only "no `source` line carries one". That is the right trade — the
  shipped string is what provenance means — but it is a gap, so it is written down instead of being
  scored away.

  It's a local scratch harness, so the count can't be reproduced from the repo.

  ### A count I asserted and had to correct

  I wrote `count == 3` for the citations arrays in the existing test, reasoning "one per query". The
  case asks three queries but `mushroom` **abstains**, and an abstaining query produces no citations
  array — the answer is **2**. The run said `left: 2, right: 3`. I had counted in my head instead of
  reading the query block four lines above the assertion I was writing.

  ### The query example, reported as it actually runs

  4 answers and 1 abstention; the envelope's wording occurs **zero** times; 4 citations arrays and
  **no** non-empty corroborations.

  **Per-row occurrence counts, with the instrument named** — because an unscoped count is not a
  measurement. Counting the **span-prefix needle** (e.g. `Leaves capture sunlight`) over the **whole
  4 080-byte output**: `leaves` **4**, `roots` **2**, `flower` **2**, `stem` **0**. Counting the
  **bare word** over the same output gives different figures — `leaves` 8, `roots` 5, `flower` 5 —
  because the word also appears in row keys and inside the stem span's text. Both are correct for
  their instrument; the first is the one that counts citation text.

  **`stem` is 0 under every scope and every instrument** — bare word and span needle, whole output,
  `recall` block, `governing` block — because the example never queries that row. That row is
  exercised by the test suite, not by this example.

  The query file said the engine returns the function atom plus "the table's source/locator/trust"; it
  now describes the row's own UF/IFAS sentence with the locator and trust inherited, with the old
  wording quoted and dated in place because it was accurate before this conversion. The README has no
  row for this table — checked, not assumed.
