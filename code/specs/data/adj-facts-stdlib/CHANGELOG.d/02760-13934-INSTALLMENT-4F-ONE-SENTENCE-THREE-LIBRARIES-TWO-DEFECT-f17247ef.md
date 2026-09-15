- **#13934 installment 4f: one sentence, three libraries, two defects — and a sweep that had to be
  thrown out.** 3 value lines and 35 comment lines across **4** `.adj` files — `language/morse-code`,
  `language/morse-code-origin`, `language/morse-code-standard` and `language/morse-code.query`.
  **Three** of those comment blocks are header quotations of the sentence — round 1's disclosure work
  added a third to `morse-code.adj` itself — and the rest is the disclosure review asked for.
  **Nine mutations redden.**

  All three **shipped** a byte-identical **194**-character value (verified by md5); as repaired it is 196. It carried two defects
  this effort has already named:

  **A dropped footnote marker.** Wikipedia reads `ITU-R<NBSP>M.1677-1,[2] was derived`; the value
  dropped the `[2]`. It sits mid-sentence, so no contiguous span covers the clause without it — the
  same class as 4c's solfège repair.

  **AND A THIRD DEFECT THE FIRST REPAIR DID NOT FIX, BECAUSE NO SCREEN HERE CAN SEE IT.** Wikipedia's
  wikitext is `[[ITU-R]]&nbsp;M.1677-1`, so the page carries **U+00A0 NO-BREAK SPACE** between
  `ITU-R` and `M.1677-1`. All three values, both re-synced header quotes and all three CI pins
  spelled it U+0020. The "repaired" value was contiguous only under whitespace normalisation — not
  byte-for-byte, which is the entire contract of the field.

  **`extract_v4.blocks()` ends with `.replace("\xa0", " ")`.** The contiguity screen therefore cannot
  distinguish an NBSP from a space and never could: **every "verbatim" verdict this effort has
  published is modulo NBSP→space normalisation.** Review caught this, not any instrument here — and
  it is an instance of the NON-ALNUM-ONLY class this very entry introduces four paragraphs down,
  shipped inside the value being repaired, one screen above where the class is named.

  The fix carries the page's character rather than normalising the page. U+00A0 is a Zs space
  separator, not an invisible format character: it renders as a space, so "as text" and "as
  rendered" agree that something space-like is there and differ only on which — a byte-fidelity
  question with an answer, not a fourth thing to hold for the owner. Verified by a harness that swaps
  NBSP for a sentinel in the RAW HTML, runs the real extractor, and swaps back, so exactly one
  normalisation is disabled and nothing else changes.

  *That harness was itself wrong first.* Its first version re-implemented extraction inline and
  emitted a space for every tag, losing the adjacent-tag lookthrough — the page came back as
  `Recommendation , ITU-R   M` and `[ 2 ]`, all three values read NOT CONTIGUOUS, and the obvious
  conclusion would have been that the repair was wrong rather than the check. That is `extract_v4`'s
  own bug 3, re-committed in a throwaway script.

  **A fabricated full stop.** The value ended `…Friedrich Gerke in 1848.` The page does not end
  there: `…in 1848 that became known as the "Hamburg alphabet", its only real defect being…`. A
  period invented to close a sentence the page continues — the same class as 4d's canyon repair.

  **Nine mutations, not six**: three per library, because the NBSP is a third axis in the same value
  and a repair no mutation covers is a repair that will rot.

  **THREE PINS, NOT ONE.** A shared value pinned once is coverage by appearance: two of the three
  libraries could revert with CI still green. And each library gets *every* mutation, because a pin
  can span one repair and miss another — which is how 4c's first solfège pin nearly shipped.

  ### One of the three libraries is not grounded by the sentence at all

  Review's second finding, and framing all three as a single repair had hidden it.
  `morse-code.adj` asserts 26 letter → dot/dash rows. The cited sentence is about the code's
  **derivation history and governing standard**; it supports **none** of them. The library's own
  header had said so without noticing — "Each row below reads a letter's code off that standard chart
  at the `locator`" — i.e. the evidence is a *chart elsewhere on the page*. So `? morse_code(s, $P)`
  returns `dot_dot_dot` with a citation that never mentions the letter s, and 4f's new pin would have
  frozen that mismatch into CI as though it were correct.

  **Disclosed, not repaired.** Citing the chart means citing a TABLE — the held question "is a table
  row a verbatim span?" — and deciding it by shipping is the one move unavailable. The header now
  states the gap outright, and the test says what its pin does *not* assert, because a green test
  beside a binding reads like a claim that the evidence supports it. Same shape as 4c's solfège
  disclosure.

  ### The first post-repair sweep was unusable, and subtracting totals would have claimed nine repairs

  `NOT CONTIGUOUS` read **68 → 59**. That is not a nine-value win. Comparing the buckets
  member-for-member:

  | | |
  |---|---|
  | departures from the non-contiguous set | 9 |
  | …that became **UNJUDGEABLE** | **6** |
  | …that are actual repairs | **3** |
  | newly unjudgeable | **+30** (36 → 66) |
  | newly fetch-failed | +3 (1 → 4) |

  **Three LibreTexts subdomains** — `chem.`, `math.` and `phys.` — were returning **503**. Thirty
  values stopped returning a Content-Type mid-sweep (**chemistry 15, physics 13, geometry 2**), and
  six of them happened to sit in the non-contiguous set — so they left it **by becoming uncheckable,
  not by being fixed**. Review corrected that attribution: the first draft blamed one subdomain and
  named the wrong two libraries.

  The rule this effort has repeated since 4a is "diff the sets, never subtract totals"; 4f is where
  it earns its keep, and it needs a second clause: *a value leaving the set because its host went
  down is not a repair.*

  **The hosts then recovered, so the sweep was re-run rather than re-described.** All three
  subdomains answer 200 again, and the fresh sweep is clean: **36 unjudgeable and 1 fetch-failed,
  both identical member-for-member to the pre-repair sweep.** Over the full population, contiguity
  goes **68 → 65**, the three that left are the three repaired, and none entered. That is the number,
  and it is worth more than the caveated one it replaces.

  *Two scoped attempts preceded it, and the first was confidently meaningless.* It counted how many
  CURRENT non-contiguous values sit on a downed host and returned **0 for both sweeps** — values that
  become unjudgeable leave the non-contiguous list entirely, so partitioning that list by host cannot
  see the ones the outage moved. *Partition the population, not the surviving list.* The second
  attempt was sound (restricted to the 493 values judged in both sweeps: 62 → 59), and is recorded
  here because the technique outlives this outage — but a re-run beats a subtotal whenever the
  hosts come back.

  `cargo test -p adj-lang-cli`: the three touched binaries green (2, 4 and 4 tests); clippy
  `-D warnings` clean; both `adj_stdlib_*` gates exit 0; the header/data desync screen reports 0; the
  scratch-tag screen reports 0 duplicates across 527 binaries. A full-workspace run remains
  impossible on this box — ten packages do not build on Windows — so that claim stays with CI.

  ### Both sibling headers quoted the sentence they were about to contradict

  `morse-code-origin` and `morse-code-standard` reproduced the old form verbatim in their headers.
  Sixth time in this effort a header has had to be re-synced with the value directly beneath it. The
  wrapper uses `break_on_hyphens=False` — `much-improved` is exactly the trap that made 4e's
  rewriter fabricate a byte while fixing fabricated bytes. `morse-code`'s own header needed no
  re-sync at that point, because it paraphrased rather than quoted — but the round-1 disclosure work
  then ADDED a quotation of the sentence to it, with a plain space, and round 2 caught that. **Second
  time in two rounds that repairing this class has shipped a new member of it.**

  The verification carries a **negative arm**: the pre-repair `ITU-R M.1677-1, was derived` must be
  absent from all three files.

  ### The mutation harness's uniqueness guard fired, and was right

  After the header sync, `Friedrich Gerke in 1848"` matched **twice** per sibling file — the header
  quote now ends the same way. Same shape as 4d's landform anchor. Both mutations were re-anchored on
  the whole `source "` line rather than on a fragment that happens to be unique today.

  ### The backlog, re-derived on this tree before choosing

  A **third** reading joined the two reconciled in 4d/4e, because both missed a class:
  **NON-ALNUM-ONLY** — value and page agree on every letter and digit and differ only in characters
  that are not. Six values, **three of which both earlier readings called ACTIONABLE**:
  `biology/animal-babies:148` and `biology/animal-baby-sex:46` (a table row with `", "` **invented**
  between cells the page emits fused) and `metrology/si-derived-units:50` (**the page carries a SOFT
  HYPHEN, U+00AD, inside "mathe<U+00AD>matical"**; the value spells it plainly). Every reading has now erred
  toward *more* work than exists — the direction that gets a held question decided by shipping.

  **26 agreed-actionable, 23 disputed, 19 agreed-held.** The screen's first name for its own class
  was wrong and its results said so: called CELL-FUSE after the pair that prompted it, it hit six
  values of which only two were cell fusions — the rest were LaTeX delimiters, MathJax doubling, and
  that soft hyphen. Renamed to what it measures.

  *Reconciling three readings introduced two bugs of mine, both caught before the number was used.*
  *The non-alnum census file predated the rename, so its held set parsed as **zero** and the count*
  *would have read 29 instead of 26. And I treated an abstaining reading as a vote for "work":*
  *NON-ALNUM-ONLY only ever ADDS held values, so counting its silence inflated DISPUTED from 23 to 39*
  *and emptied AGREED-HELD entirely, without a single value changing. A reading that abstains does*
  *not vote.*

