- **#14986: `biology/seed-dispersal-mechanism.adj` — three mechanisms stop being warranted by the barochory sentence.**
  The envelope was the barochory quote, so a `ballochory`, `anemochory` or `epizoochory` answer's
  primary source was a sentence about gravity dispersal, and the other three subsection quotes were
  header prose that reached no answer.

  ### What each row carries now

  Measured 2026-09-15 on Wikipedia's "Seed dispersal" article (HTTP 200; a nonsense path on the same
  host returns a real 404 holding none of these spans), with inline tags removed without a space and
  only ASCII whitespace collapsed. Each row's span occurs **exactly once**, inside one innermost
  `<p>`, scripts set aside and over the whole file, zero times on the nonsense page, and with no
  `<head>` copy.

  - **Each row takes its own subsection's sentence.**
  - **The `anemochory` row takes TWO sentences.** Its shipped span was *"Wind dispersal can take on
    one of two primary forms: seeds or fruits can float on the breeze or, alternatively, they can
    flutter to the ground."* — which never says "anemochory". The sentence before it in the same
    paragraph does (*"Wind dispersal (anemochory) is one of the more primitive means of dispersal."*),
    and the joined pair is contiguous exactly once, so the row carries both. That is the remedy
    `README.md` prescribes under "A citation must name its own subject": widen the quote until it is
    self-contained.
  - **The envelope** is the article's framing sentence — *"Seeds can be dispersed away from the parent
    plant individually or collectively, as well as dispersed in both space and time."* It names no
    mechanism and no description word.
  - Every row's page is the envelope's, so no row restates a locator line or trust.

  ### A `consensus`-tier conversion

  Wikipedia, so `trust consensus` — the needle pins `"trust":"consensus"`, and a mutant swapping the
  tier to `authoritative` is killed by three tests. (No ordinal here: the only other `consensus`-tier
  conversion recorded in main is shard 03460, and the one in between is an unmerged sibling PR, so
  "the third" would not be checkable from the repo.)

  ### Pins

  - **Kept:** the direct bind, the reverse bind, and the `hydrochory` abstention.
  - **Inverted:** `contains("en.wikipedia.org") && contains("\"trust\":\"consensus\"")` — satisfied by
    any Wikipedia citation and by a truncated span. Each answer is now pinned to its own whole
    citations array, closing on both the corroborations `]` and the citations `]` (#14735).
  - **Added:** a per-mechanism check with negative arms and an assertion that each span *names* its own
    mechanism; a test that the anemochory row carries the naming sentence and that no row is warranted
    by the dangling sentence alone; a table-shape test that also pins the tier.

  **10 of 10 mutants killed, two controls:**
  - the barochory row reverted to a bare row that inherits the envelope;
  - the epizoochory row reverted to the barochory span (the exact defect being fixed);
  - the anemochory row cut to the dangling sentence, which names no mechanism;
  - the anemochory row cut to its naming sentence alone;
  - the old envelope restored;
  - the trust tier swapped to `authoritative`;
  - a table-level `cites` re-added;
  - a description atom rebound;
  - the envelope naming a mechanism;
  - the epizoochory span truncated before it names the mechanism.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer.** The shipped query example returns 2 answers and 1
  abstention, every corroboration array is empty, and the framing sentence occurs zero times in the
  output.

  The query example needed no change — it is bare queries with no prose describing the citation shape
  — and neither did the README row, which describes the axis and the row pairs rather than the
  citation shape.
