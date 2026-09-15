- `anatomy/heart-valves.adj` — all 4 rows converted to per-row provenance (RS-5e, #14986), and
  **two of them ship at `trust inferred` because the page does not state their value outright.**

  Every row was warranted by the TRICUSPID sentence, so `? valve_separates(aortic, $B)` came back
  proved by *"The right atrioventricular valve is the tricuspid valve."* Three of the four were in
  that position.

  **Two kinds of row, and the tier is where the difference shows.**

  *READ* — `pulmonary` and `aortic`. One sentence names the valve **and both chambers**: *"The
  valve between the right ventricle and pulmonary trunk is the pulmonary semilunar valve."*
  Nothing is reasoned, so these inherit the envelope's `authoritative`.

  *REASONED* — `tricuspid` and `mitral`. Their sentences name the valve and call it right/left
  **atrioventricular**; neither names a chamber. The boundary is reached by combining that with a
  second span — *"The valves between the atria and ventricles are called atrioventricular valves
  …"* — so these ship `source` + `cites` at **`trust inferred`**, the shape
  `geography/reference-lines.adj` already uses. Shipping them at `authoritative` would claim the
  page states something it does not.

  **The assertion that makes this real binds the TIER TO THE SPAN in one contiguous run.** Three
  tier mutants die on it: promoting a reasoned row, downgrading a read one, and **swapping both
  pairs**, which leaves the counts at two and two and still fails.

  **The reason I first gave for that pin was wrong, and review measured it.** I wrote that
  checking tier and span separately "would pass on either kind of row, because `authoritative`
  appears in the output either way — it is the envelope's tier". It does not. The envelope is not
  a fact and every row overrides `source`, so on a single-valve query the envelope's tier never
  reaches stdout. Measured directly, per query: `tricuspid` and `mitral` contain `authoritative`
  **0** times; `pulmonary` and `aortic` contain `inferred` **0** times. The claim holds only of
  the pre-existing multi-query test, which runs four queries in one program so both tiers appear —
  and that is where the two-loose-needles weakness actually bit. The contiguous pin is still the
  right assertion; the justification was overstated, and the **stronger** `!contains(wrong_tier)`
  arm that an earlier comment talked itself out of is now asserted outright.

  Measured 2026-09-14 (HTTP 200, 46,225 chars), with the shipped envelope found verbatim as a
  positive control and a fabricated sentence absent as a negative one: all five spans occur
  **exactly once**, each inside a `<p>`. **"Verbatim" is conditional on one stripping rule for
  three of the five** — the pulmonary, aortic and tricuspid sentences are interrupted in the raw
  HTML by inline `<a class='glossaryTerm'>` anchors, so their raw-byte count is zero and they match
  only after tags are stripped with no separator. The anchors wrap plain text, so the strip
  reproduces the page's characters exactly; the other two are contiguous in the raw bytes.

  The old citation assertion — `contains("training.seer.cancer.gov/…") &&
  contains("\"trust\":\"authoritative\"")` — was the #15209 two-loose-needles shape; #15209 measures
  **321 test files, 323 occurrences** carrying it. This is the fourth fixed *of the set that issue
  tracks* (`em-spectrum`, `circuit-parts`, `energy-form-family`); a fifth instance was also fixed
  in `earth-science/water-cycle.adj`, recorded further down this file, so "fourth" is an accounting
  of #15209's list rather than of the repo.

  **Review then found the stale-header defect for the third time in this cascade.** The
  top-of-file provenance block still described the pre-conversion design and still asserted
  `trust authoritative` for a table half of which no longer ships at that tier — the same defect
  found in `em-spectrum` and `circuit-parts`, both recorded in this file. I fixed it twice and did
  not grep for it here. It also carried a **truncated quote under a heading promising
  character-for-character**: the atrioventricular sentence rendered as ending after "(also called
  cuspid valves)." with a period the page does not have, a string occurring **zero** times on it,
  while the rows fifty lines below cite the full sentence. Both corrected. And one structural
  check used `unwrap_or(0)`, which turns a missing closing brace into an empty block — and an
  empty block satisfies `!contains("trust inferred")`, the expected value for the two read rows,
  so the assertion named "the reasoned rows are the right two" could have passed while asserting
  nothing about them.

  **The stale-header defect was then swept for, rather than waiting for a fourth review to find
  it** — and the sweep's one real hit was a leftover I had created an hour earlier in this very
  file. Fixing the paragraph review flagged, I ended my replacement with *"The retained envelope
  holds the single strongest span"* — stitching new prose onto the old sentence and leaving the
  rest of the old paragraph standing. The file still said every other row's span was merely
  "listed above", and still closed with a bare *"so `trust authoritative`"* for a table half of
  which now ships `inferred`.

  Scope of the sweep: **30 converted tables**, needle = header phrases asserting the
  single-envelope design (the wordings from the three known cases plus variants — a lower bound,
  since a header can be stale in wording not listed). Six matched; opening all six, three **quote**
  the old wording while correcting it (`em-spectrum`, `circuit-parts`, `heredity-term`), one is
  **accurate as written** (`energy-form-family` genuinely has one envelope — no row carries a
  `source`), one was a false positive whose correction marker sat outside my context window
  (`body-counts`, which says *"The paragraph that stood here read:"* and then quotes it), and one
  was real. After the fix, every remaining hit is a record or accurate; **none asserts the old
  design**.

  **A second recurring class was swept for too** — a header that QUOTES a span differently from
  what the table ships, found by review in `circuit-parts` (a sentence occurring zero times on the
  page) and in `heart-valves` (a truncation with an invented terminal period).

  The first needle was useless: "any header quote that is not a shipped span" flagged **30 of 30**
  converted tables with 120 hits, because it caught ellipsis-marked abbreviations, atom names,
  column labels and quotes of other documents. Narrowed to the actual defect shape — a quote
  **presented as a complete sentence** (initial capital, terminal punctuation, no ellipsis, 40+
  characters) that the table does not ship — it reports **8 tables, 12 hits**.

  Opening all twelve: most are benign (a rhetorical question in a header; correction-paragraphs
  that quote the old wording; a sentence about a row the table deliberately excludes, in
  `energy-forms`; a prefix flagged as partial). **One was real**, in
  `physics/circuit-parts.adj`: the header rendered the LED span's `'arrow'` with straight quotes
  where the page and the shipped row have curly `“arrow”` — same length, one character class
  different, inside a header promising the spans character-for-character. A typographic
  normalization is the same defect as the other two, one notch quieter. Fixed; the sweep now
  reports 11 hits, none of them a mismatch with the page.

  9 of 9 mutants killed, green baseline before and after, file verified byte-identical afterwards.
  Local scratch harness, so that count is not reproducible from the repo.

