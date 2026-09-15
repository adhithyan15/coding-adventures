- **#14986: `biology/muscle-striation.adj` — the smooth and cardiac answers stop being warranted by the skeletal sentence.**
  The envelope was the skeletal row's own sentence, *"Skeletal muscle fibers are cylindrical,
  multinucleated, striated, and under voluntary control."*, so it was the primary source of all three
  answers. The smooth and cardiac sentences were only table-level corroborations.

  ### What each row carries now

  Measured 2026-09-15 on SEER's "Muscle Tissue" page (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the sentences below.
  - **Every row takes its own sentence as a `source`.** Each sentence names its muscle type and whether
    it is striated, and each occurs **exactly once**, inside a `<p>`.
  - **The smooth row takes one sentence, not two.** The shipped `cites` was *"Smooth muscle cells are
    spindle shaped, have a single, centrally located nucleus, and lack striations. They are called
    involuntary muscles."* The second sentence names no striation, so the row's `source` is the first
    alone. Both forms occur once.
  - **The envelope** is now the page's *"Muscle tissue is composed of cells that have the special ability
    to shorten or contract in order to produce movement of the body parts."* It occurs once in a `<p>`
    and names no muscle type and no striation.
  - The table-level `cites` are gone. Every row's page is the envelope's, so no row restates a locator
    line or trust (`ADJ-TABLES.md` §4).

  ### The header

  The provenance paragraph said the spans were reproduced "byte-for-byte" with "no new WebFetch". It is
  replaced by the per-row account and the measurement. The row comments that repeated each sentence are
  dropped, since each row's `source` now carries it.

  ### Pins

  The test keeps every behaviour it shipped with: cardiac recalled as striated, smooth as the only
  non-striated type, and no abstention across the three types. Its `contains("seer.cancer.gov") &&
  contains(trust)` check (#15209's shape) becomes the cardiac row's whole contiguous citation. Added:
  - every muscle type's answer is one answer carrying its own sentence, with no other type's sentence
    and not the envelope;
  - the smooth answer carries the one-sentence form, not the shipped two-sentence span;
  - a table-shape test.

  **8 of 8 mutants killed, two controls:**
  - the smooth row given the old two-sentence span;
  - the cardiac row warranted by the skeletal sentence;
  - the smooth `source` demoted to `cites`;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a striation atom rebound;
  - a muscle type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
