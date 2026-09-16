- **#14986: `astronomy/comet-part.adj` — the coma and the tail stop being warranted by the nucleus sentence.**
  The envelope was the `nucleus` span, so asking what a comet's coma is returned
  `fuzzy_cloud_of_gas_and_dust_around_the_nucleus` evidenced by *"At the heart of every comet is a
  solid, frozen core called the nucleus."* — a sentence about a different part. The coma and tail
  sentences were header prose that reached no answer.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of NASA Space Place's "What Is a Comet?" page,
  with inline tags removed without a space and only ASCII whitespace collapsed.
  - **Each row takes the NASA sentence introducing its own part.** Each occurs **exactly once**, inside
    one `<p>`, scripts set aside and over the whole file, with no `<head>` copy.
  - **The envelope** is the page's sentence framing the anatomy as a whole — *"This diagram is not to
    scale, but it shows the anatomy of a comet."* — which names none of `nucleus`, `coma`, `tail` as a
    whole word. Once, in one `<p>`, no `<head>` copy.
  - **The nonsense-path control is not a stub, and the byte counts are not properties of the page.**
    A nonsense path on the same host answers 404 with a full styled page of about **11 280** bytes
    against the real page's about **21 830** — so the control is that the spans are absent from a page
    of *comparable size*, not that the page is empty. Two fetches minutes apart returned 21 823 and
    21 830 bytes, so those figures are measured-at-a-moment.

  ### All three spans mention the nucleus, which constrains the tests

  The coma is a cloud **around the nucleus**; the tail streams **away from the nucleus**. So the bare
  word `nucleus` is satisfied by all three spans, and **every negative arm asserts absence of a WHOLE
  SPAN** — an arm built on that word would fire for every part and prove nothing about which sentence
  reached an answer. The shipped header records this for whoever edits the test next.

  **What actually carries the weight is `only_citation`**, which pins the whole citations array as one
  contiguous run: given that, no other span can appear *inside* the citation block at all. The
  cross-span negative arms are defense-in-depth against text outside it, not independent evidence.
  Said plainly because the first draft of this entry implied they were doing the work.

  This is the hurricane table's shared-`catastrophic_damage` trap in a different costume: there the
  shared token was a value, here it is a word inside three different sentences.

  ### Pins

  - **Kept:** the forward bind, the reverse bind, and the `short_period_comet` abstention — a real
    comet-related term the same page covers, but one that classifies by orbital period rather than
    physical anatomy.
  - **Inverted:** `contains("nasa.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by any
    NASA citation, constraining no sentence text, and previously satisfied by the nucleus span riding
    on every answer. Now the nucleus row's own whole citations array, closing on both the
    corroborations `]` and the citations `]` (#14735), **as two separate assertions** rather than one
    `&&`: joined, a failure cannot say which arm broke.
  - **Added:** a per-part check; a table-shape test (three row sources, keyword-anchored `cites`
    absence, **exactly one** `locator` and **exactly one** `trust` line, and a whole-word part-name
    check on the envelope); and a value-supported-by-its-own-span test (#15318).

  ### The #15318 mutant took three shapes before it isolated its arm

  **10 of 10 mutants are now killed by their named assertion**, both controls green, both files
  byte-identical afterwards. The first run scored **9 of 10**, and the one that missed is worth
  recording because each attempt failed for a different, instructive reason:

  1. **Rebinding the atom** (in the `.adj` and `scale()`) dies on the **pre-existing** reverse-bind
     test, which hardcodes the query `comet_part($P, fuzzy_cloud_of_gas_and_dust_around_the_nucleus)`.
     Change the atom and that query binds nothing — the mutant never reaches the new arm.
  2. **Replacing the span in the `.adj` alone** dies on arm *one* of the value↔span test ("coma carries
     its own span"), because the test's `COMA` const no longer matches the file.
  3. **What works:** change the span in **both** places so arm one passes, keep the atom, and pick a
     replacement that **names the part but omits the atom's distinctive content**. Then the part-name
     arm passes and only `must state` can fire.

  Each shape was diagnosed by reading which assertion fired, not by reasoning about which one ought to.
  It's a local scratch harness, so that count can't be reproduced from the repo.

  ### The query example, reported as it actually runs

  2 answers and 1 abstention; the envelope's wording occurs **zero** times in the output; 2 citations
  arrays and **no** non-empty corroborations. The example queries the nucleus directly and the coma in
  reverse, so the **tail** span occurs **zero** times in its output — that row is exercised by the test
  suite, not by this example.

  ### What needed no change, checked rather than assumed

  The query file states no fact and makes no claim about what the citation carries. The README row
  describes the axis, the abstention and the picking method. The sibling `comet-tail-type.adj` was
  checked reference by reference: it cites this table as a coarser sibling, cites the same page, and
  shares its tier — but quotes none of its spans and asserts nothing about its provenance model, so
  nothing there goes stale.
