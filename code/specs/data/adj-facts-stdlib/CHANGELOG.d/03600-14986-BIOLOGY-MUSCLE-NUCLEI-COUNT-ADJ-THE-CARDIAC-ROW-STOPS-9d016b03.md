- **#14986: `biology/muscle-nuclei-count.adj` -- the cardiac row stops being warranted by the skeletal sentence.**
  The envelope was the `skeletal` span and the cardiac sentence rode as a table-level `cites`, so asking
  how many nuclei a cardiac fiber has returned `single_nucleus` evidenced primarily by *"Skeletal muscle
  fibers are cylindrical, multinucleated, striated, and under voluntary control."* — a sentence about a
  different muscle type. Relocating it empties the corroborations array on every answer.

  ### What each row carries now

  Measured 2026-09-16 UTC by this table's own fetch of the NCI SEER Training Modules "Muscle Tissue"
  page, inline tags removed without inserting a space, only ASCII whitespace collapsed.
  - The envelope and both row spans occur **exactly once** on the page and **zero** times against a real
    404 control on the same host.
  - **Zero U+00A0, U+2013 and U+2019 on the page** — checked up front, and worth stating because the two
    previous conversions in this batch both shipped invisible-byte hazards (an NBSP envelope in shard
    03570, an en dash and right quote in 03590). This one has none.
  - Each row span names its own muscle type and states the content its atom compresses
    (`multinucleated`; "one nucleus per cell").

  ### The envelope must name no muscle type — not merely neither row key

  The new envelope is the page's framing sentence, *"Muscle tissue is composed of cells that have the
  special ability to shorten or contract in order to produce movement of the body parts."*

  This page describes a **third** type, `smooth`, which is deliberately no row here. A sentence about
  smooth muscle names neither row key and would still frame the wrong table — the exact mutant that
  survived in shard 03590 before its forbidden-token list was widened. So the shape test forbids the
  whole taxonomy: `skeletal`, `cardiac` **and** `smooth`.

  That mutant is not hypothetical here, and it is stronger than 03590's. The page states a genuine nuclei
  count for smooth muscle — *"Smooth muscle cells are spindle shaped, have a single, centrally located
  nucleus, and lack striations."* — so planted as a **row source** it would read as entirely plausible
  provenance rather than as an obviously wrong frame.

  ### Two kinds of span-in-a-comment, and only one was removed

  This table had both, which no previous conversion in the batch did.
  - **Removed:** the trailing `% SEER: "..."` comment on each row line. Those existed only because a row
    could not carry its own citation before RS-5e — #13934's evidence-left-in-comments shape. The span is
    now the row's `source` two lines below, so the comment was duplicated data that could drift.
  - **Kept:** the header prose that quotes both sentences while *arguing why this table exists* (a second
    leftover fact buried in spans a sibling table already cites). That is explanation, not stranded
    evidence.

  The converter's first guard demanded each span survive **twice** (source + comment) and refused to
  write. The refusal was correct and the requirement was wrong; the guard now asserts **exactly once**.

  ### A true claim that became misleading when a new paragraph landed beside it

  The header said *"Only skeletal and cardiac muscle have a nuclei-count fact stated in this cited span"*
  — true, and scoped to the two sentences `tissue-types.adj` quotes in its header, which is where this
  table's facts were mined. My new paragraph about the **page** landed directly beneath it, and a reader
  moving between them would read "this cited span" as "this page" and find it false.

  Narrowed rather than rewritten, and the narrowing is itself measured: `tissue-types.adj`'s shipped
  muscle `cites` reads *"Muscle tissue can be categorized into skeletal muscle tissue, smooth muscle
  tissue, and cardiac muscle tissue."* — final period included, because a quotation missing one is the
  error class this batch exists to remove — and it **names smooth** while giving no nuclei count at all,
  so the claim is not true of that span either. Smooth muscle is excluded for the reason the header gives,
  not for want of a stated fact.

  The same narrowing was applied to a parenthetical **one line above** the sentence being repaired, which
  still asserted smooth was "not part of `tissue-types.adj`'s own muscle citation at all" — the header had
  been left asserting both halves of a contradiction. Correcting a claim and leaving its twin standing two
  lines away is the failure this batch keeps rediscovering.

  ### Pins

  - **Inverted:** `contains("seer.cancer.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by
    any SEER citation, constraining no sentence text, and before this change satisfied by the skeletal
    span riding on the cardiac answer. Each answer is now pinned to its own whole citations array, closing
    on both the corroborations `]` and the citations `]` (#14735).
  - **Added:** a per-type loop with whole-span negative arms; a shape test forbidding the whole muscle
    taxonomy in the envelope; and a span-supports-its-own-row test anchored on the **row header**.
  - **Kept:** the backward bind and the `smooth` abstention.
  - **Scope stated in the failure text.** The "no `cites` at any indent" and "exactly one `locator`/`trust`"
    arms pin a convention local to *this* table, not a language rule: `lower.rs`'s row path accepts
    `Source`, `Locator`, `Trust`, `Cites` and `Quote`, and 18 shipped tables carry a row-level `cites`.

  ### The query example, reported as it runs

  3 queries, 2 answers, 1 abstention (`smooth`); **2** citations arrays, **0** non-empty corroborations,
  envelope wording in **zero** answers, and **4** empty-corroboration occurrences because provenance
  renders on two surfaces (`citations`, and the steps array's `"kind":"fact"` entries).

  ### A third spelling of the retired boilerplate

  This header said each row lowers to a relation *"carrying **the citation**"* — a **third** #15336
  spelling, matched by neither of the two I had reported. Measured over
  `code/specs/data/adj-facts-stdlib/**/*.adj` with `newline + %` collapsed, **before this conversion**:
  `the table's citation` **125**, `carrying the citation` **93**, `the source citation` **82** — a sum of
  **300** over **299 distinct files**, because exactly one file
  (`earth-science/water-movement-route.adj`) carries two of the spellings. Against the **104** the issue
  records and the **208** I posted for two spellings.

  Sum and union are stated separately on purpose: a review read the union as a failed addition. It is not
  — a file carrying two spellings is counted once in the union and twice in the sum, and the overlap is
  exactly one file, measured rather than inferred.

  All of that is reported on #15336 — including that my own earlier correction there had repeated the
  original error, by extending the needle list with the examples I had tripped over rather than
  rebuilding it from a loose scan of the corpus.

  The same scan *after* this conversion reads **125 / 92 / 82, union 298**, because fixing this file
  removed one occurrence. The pre-conversion figures are the ones quoted here and on the issue, since
  that is what was measured when the claim was made; the one-file delta is this change.
