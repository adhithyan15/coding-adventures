- **#13934 installment 4b: four more citations re-grounded, and the "easy band" turns out not to be
  one defect.** 4 value lines, 9 comment lines, 7 files. `geography/map-type`,
  `meteorology/precipitation-freeze-threshold`, `geography/landform-secondary-feature`,
  `geography/landform-extent`. Four mutations redden. Contiguity **79 → 75**.

  4a's five were one editorial act — a fragment punctuated into a sentence, one character each. The
  next band down is **elision**, and it is three different things:

  - `map-type:57` — one character of **case**: `Political maps` for the page's `Political Maps`. The
    last survivor of 4a's shape, and the case this changelog called a *paraphrase* for three
    installments before review fetched the page.
  - `precipitation-freeze-threshold` — a **leading ellipsis and a truncated tail**. Restored to the
    paragraph's full 472-character span, which begins "Freezing rain occurs when snowflakes
    descend…".
  - `landform-secondary-feature:79` and `landform-extent:59` — a **trailing ellipsis** on a
    209-character value that *two* libraries ship. Restored to the page's complete 399-character
    sentence.

  **The restored sentence reproduces a typo in its source** — "anything that that is at or below
  freezing" — doubled word and all. A citation that silently corrects its page is the same defect
  class as one that silently tidies it, and this entire effort exists because of the second kind.

  **Both of those repairs were WORSE on the first attempt, and review caught both.** My first
  precipitation span began "Because they are “supercooled,” **they** instantly refreeze…" — pronoun-led,
  never naming freezing rain, while being the sole evidence for `freezing_rain → 32`. My first
  landform repair simply deleted the trailing " ..." — which **removed a MARKED elision and left an
  UNMARKED truncation**, mid-sentence and with no terminal punctuation. This entry's own standard is
  that a marked ellipsis *declares* its omission while a truncated prefix *hides* one; the repair had
  swapped the honest form for the dishonest one. Both better spans were available on the same pages
  and are now shipped. **The fault in both was the same: taking the first contiguous span that
  worked rather than the best one available.**

  **The landform citations still do not name their subject, and cannot.** The USGS page renders the term
  and its definition as sibling elements — `plateaus`, then `Comparatively flat areas…` — so a span
  naming the subject would be verbatim against the *extractor's concatenation* rather than against
  the page. 4a shipped two subjectless citations by extending forward into a figure reference; this
  is a different reason for the same shortfall, stated rather than papered over.

  **A fourth #14124 instance, again from reading bindings beside source.** `map-type`'s first
  authored query asks `physical`, and the envelope is the *political* sentence — so the obvious pin
  would have bound an answer about landscape features to a citation about boundaries.
  `landform-secondary-feature` needed the same correction for a different reason: its repair is a
  `cites` grounding `(plateau, bounded_by_abrupt_descent)`, and **no authored query asks `plateau`**.
  Twice now the authored `.query.adj` has not covered the row the repaired evidence grounds; that is
  worth expecting rather than rediscovering.

  **Three sibling libraries quoted the repaired sentences without shipping them.** `landforms`,
  `map-type-classification` and `precipitation-types` carry the old wording in prose only — no
  `source`, no `cites`. Left alone they would leave the repo shipping two renderings of one page
  sentence, which is the cross-file propagation problem flagged in 3b. Four of the five comment sites
  wrap across lines — the shape a whole-span per-line matcher cannot see; `map-type.adj:28` is a
  single-line table row and would have been visible to one.

  One elision is deliberately **not** touched: `landform-secondary-feature:19` quotes a fragment with
  explicit ellipses at both ends (`"…commonly limited on at least one side by an abrupt descent…"`).
  Marked elision in prose is a legitimate device, not a verbatim claim.

  *Contiguity numbers are measured over* `source`/`cites` values of at least 20 characters with a
  resolvable locator, compared against their page after stripping tags **with no separator inserted
  at tag boundaries**, unescaping entities, NBSP→space and collapsing whitespace. That rule is
  load-bearing and is stated because the number flips on it.

  **The screen's buckets are not stable across runs, and that is worth knowing before anyone reads a
  delta as progress.** Between the 4a and 4b sweeps of the same tree, three untouched values
  (`anatomy/ear-parts:90`, `anatomy/ear-structure-function:72`, `biology/major-organs:99`) moved from
  *verbatim* into *unjudgeable*, and one (`language/author-purpose:80`) moved out of *fetch-failed* —
  because Content-Type and fetch success depend on what the host returns that minute, not on the
  repository. I verified the 79 → 75 delta is exactly the four values repaired here, by diffing the
  two runs' non-contiguous sets rather than subtracting totals: the four that left are the four
  repaired, none entered, and none of the three newly-unjudgeable values had been non-contiguous.
  Subtracting totals alone would have credited this installment with someone else's server.

  533 test binaries / 1652 tests green; clippy `-D warnings` clean; both `adj_stdlib_*` gates exit 0;
  the header/data desync screen reports 0.

  *Deferred with a reason, not skipped:* `geometry/angle-pairs:80` is a **new class**. Its page states
  the fact as LaTeX — `\(90^{\circ}\)` — which MathJax renders as `90°`, and the shipped value
  carries the rendered form. "Verbatim against the page" is ambiguous when the page's text and the
  page's display differ, exactly as it is for the 33 PDF locators. Filed rather than guessed.
  `geography/landform-secondary-feature:80` remains deferred too: it sits inside a bracketed glossary
  designation rather than a sentence.

