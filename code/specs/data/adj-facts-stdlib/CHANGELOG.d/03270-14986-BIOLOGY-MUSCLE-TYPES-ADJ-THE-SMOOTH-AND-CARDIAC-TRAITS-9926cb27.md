- **#14986: `biology/muscle-types.adj` — the smooth and cardiac traits stop being warranted by the skeletal sentence, and three header claims the page does not support are corrected.**
  The envelope was the skeletal row's own sentence, *"Skeletal muscle fibers are cylindrical,
  multinucleated, striated, and under voluntary control."*, so it was the primary source of all three
  answers.

  ### What each row carries now

  Measured 2026-09-15 on SEER's "Muscle Tissue" page (HTTP 200; a nonsense path on the same host
  returns 404), with inline tags removed without a space and only ASCII whitespace collapsed. The
  nonsense page holds none of the spans below.
  - **Every row takes its own span as a `source`.** Each span names its muscle type and its trait, and
    each occurs **exactly once**, inside a `<p>`.
  - **The smooth row's span is two sentences:** *"Smooth muscle cells are spindle shaped, have a single,
    centrally located nucleus, and lack striations. They are called involuntary muscles."* Its trait,
    "involuntary", is only in the second sentence, whose "They" needs the first.
  - **The envelope** is now the page's *"Muscle tissue is composed of cells that have the special ability
    to shorten or contract in order to produce movement of the body parts."* It occurs once in a `<p>`
    and names no muscle type and no trait.
  - Every row's page is the envelope's, so no row restates a locator line or trust (`ADJ-TABLES.md`
    §4).

  ### Header claims the page does not support

  - **"The source calls both smooth and cardiac muscle involuntary."** "involuntary" occurs **once** on
    the page, in the smooth sentence. Of cardiac muscle, the page says *"Its contraction is not under
    voluntary control."* The header now says exactly that.
  - **"The presence of intercalated disks distinguishes cardiac muscle as unique among the three
    types"**, quoted as "the source's own summary", occurs **zero** times on the cited page. It is marked
    unverified. The cardiac row does not depend on it: the cardiac sentence names intercalated disks.
  - **"There are three types of muscle tissue: …"**, said to be stated on the same page, occurs **zero**
    times. It is marked unverified.
  - The opening paragraph put a paraphrase in quotation marks; it is now marked as a paraphrase.
  - The "An ADJ `table` carries ONE provenance envelope" paragraph is replaced by the per-row account
    and the measurement.

  ### Pins

  The test keeps every behaviour it shipped with: the forward, reverse and abstention recalls. Its
  `contains("training.seer.cancer.gov") && contains(trust)` check (#15209's shape) becomes the skeletal
  row's whole contiguous citation. Added:
  - every muscle type's answer is one answer carrying its own span, with no other type's span and not
    the envelope;
  - the smooth row carries the two-sentence span;
  - a table-shape test.

  **8 of 8 mutants killed, two controls:**
  - the smooth row losing the sentence that names its trait;
  - the cardiac row warranted by the skeletal sentence;
  - the smooth `source` demoted to `cites`;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row restating a different trust tier;
  - a trait atom rebound;
  - a muscle type named in the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output.
