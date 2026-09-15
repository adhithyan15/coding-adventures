- **#13934 installment 4e: two citations re-grounded, and seven fabricated header quotes behind
  them.** 3 value lines and 11 header quotes across 4 `.adj` files —
  `language/syllable-segmentation`, `language/syllable-deletion`, `language/syllable-count` and
  `language/phoneme-deletion`. Three mutations redden. Contiguity
  **70 → 68**, verified by diffing the non-contiguous SETS: the two that left are the two
  repaired, none entered.

  **QUOTE FLATTENING PLUS A TRUNCATED BRACKET.** `syllable-segmentation` shipped ASCII quotes where
  Reading Rockets uses curly ones, and stopped its bracket at `[place a card].` where the page reads
  `[place a card so it appears left-to-right for students].` — the glyph class the earliest screens
  were built for, and 4b's silent-elision class, in a single string.

  **AN ASSEMBLED QUOTATION.** `syllable-deletion` shipped

  > The word is: 'pencil'. I take away 'cil'. 'Pen' is left.

  and those three fragments are not adjacent. The page runs them across intervening sentences with
  bracketed stage directions between: `The word is: 'pencil'. 'Pen' 'cil' [place a card …]. 'Pencil'
  [sweep finger …]. I take away 'cil' [remove the card]. 'Pen' is left [touch remaining card].` Three
  fragments stitched, every stage direction deleted, presented as a span. Its **25% triage overlap
  said so** — this is the first installment to take a value from the bottom of the band rather than
  the top, and the bottom is where the assembled ones live.

  The replacement is the page's own contiguous sentence pair, `I take away 'cil' [remove the card].
  'Pen' is left [touch remaining card].`, which states this table's deletion in one breath and is
  **better evidence than the stitched version, not merely more honest**.

  **But it grounds two of the row's three columns, and my first defence of that was circular.** I
  wrote that the dropped lead-in named the word and "the row's own first column already carries it,
  so nothing the table asserts lost support" — the column IS the claim, not evidence for it, and the
  reverse query binds `pencil` off exactly that column. Review caught it. The page's lead-in, `The
  word is: 'pencil'.`, is a contiguous span of its own, so it returns as a `cites` beside the
  envelope; between them the citation grounds all three columns, which one span alone did not. The
  library's rationale paragraph — which still argued for the old two-fragment composition and for
  "deliberately NOT drawing from the section's bracketed stage-direction text", when the shipped
  value is now one span that keeps the brackets — was rewritten in the same edit, along with a
  fabricated comma it had put inside a quotation mark where the page has a full stop.

  ### The value defects were the thread; the header family was the sweater

  `syllable-segmentation`'s header claims its quotes are "**byte-identical to `syllable-count.adj`'s
  existing quotes**". They were not. **All five** of its header quotes truncated the same bracket —
  the four in its quote block and the one in its opening paragraph — and **three of
  `syllable-count`'s** were ASCII where the page is curly, with two more in `syllable-deletion`.
  Three libraries quote the same four sentences and **no two agreed** — not with the page, and not
  with each other.

  Review caught the ledger undercounting its own repair: this entry first said seven quotes across
  three files. It is **eleven across four**. In an installment whose thesis is honest accounting,
  that is the finding worth keeping.

  **And it was not the last one on that page.** Eleven libraries cite this Reading Rockets module,
  not the three named here; review screened all of them and found one more glyph-flattened quote, in
  `phoneme-deletion.adj`, two lines below a quote already curly in the same block. Fixed, which
  closes the class on this page.

  Rather than soften the byte-identity claim, all seven were made identical to the page, which makes
  the sentence true and removes seven "verbatim" strings that were on no page in any style. **The
  spans are read from the page by script, not typed** — after this many hand-reconstruction failures
  the page is the only authority for its own bytes.

  **A third mutation covers them — but only locally, and that is a gap this entry has to name.**
  `syllable-count` ships a correct value and only its header changed, so nothing in CI would notice
  those quotes regressing. That mutation typos one and is caught by the header-quote control rather
  than by cargo, because the defect lives in prose.

  **That control lives in a scratchpad, not in the repository.** Review pointed out that after this
  PR the eleven repaired header quotes are protected by nothing a contributor or CI can run — which
  is precisely the condition the sentence above warns about. `syllable-count`'s *value* is pinned by
  a test; its header is not. Recorded on #14111 rather than papered over, and the honest statement is
  that **header quotes across this stdlib remain uninstrumented in-repo**.

  ### Two mistakes of mine, both caught immediately

  **The header rewriter fabricated a byte while fixing fabricated bytes.** `textwrap` split
  `left-to-right` across the hyphen, so unwrapping a header quote yielded `left-to- right` — a string
  on no page, introduced by the script whose entire purpose was byte-accuracy, and caught only by its
  own verification pass. Fixed with `break_on_hyphens=False`.

  **Reverting to redo that wrapping, I ran `git checkout --` on all three files** — which also undid
  the two VALUE repairs, not just the headers. Caught by re-grepping the `source` lines instead of
  assuming, and re-applied. A revert scoped to "the files I touched" is not scoped to "the change I
  regret".

  The header check now carries a **negative arm**: the pre-repair truncated form `[place a card].`
  must be ABSENT from every header, or a pass proves nothing. It also verifies by unwrapping `%`
  continuation lines the way a reader does, rather than by pairing quote characters — quote pairing
  is what defeated four earlier header screens.

  *Contiguity numbers are measured over* `source`/`cites` values of at least 20 characters with a
  resolvable locator (562 of 568), compared after stripping tags with **no separator inserted at tag
  boundaries**, unescaping entities, NBSP→space and collapsing whitespace.

  *The `fetch failed` bucket went 2 → 1: the NIH rate-limit artifact cleared, leaving only
  `physics/circuit-parts`'s genuinely dead k12maker PDF. Across the last three sweeps that bucket
  held 2, 2 and 1 while its membership changed every time — `biology/vitamins:122`, then
  `biology/vitamin-deficiency-symptom:84`, then `:77`, each re-screening clean when probed alone. The
  unjudgeable set (36) is identical member-for-member.*

  *Backlog, re-derived on the current tree before choosing: **31 agreed-actionable, 23 disputed, 16
  agreed-held** — exactly the 33 − 2 predicted after 4d, the first time this bookkeeping has closed
  without a correction. A higher-scoring candidate (`biology/animal-babies:148`, 62%) was left alone:
  the page reads `colt (male), filly (female), foal` and the value APPENDS `, weanling, yearling`, a
  value claiming more than its span contains, which needs its own reading rather than a quick fix.*

  `cargo test -p adj-lang-cli`: the three touched binaries green (5, 4 and 4 tests); clippy
  `-D warnings` clean; both `adj_stdlib_*` gates exit 0; the header/data desync screen reports 0; the
  scratch-tag screen reports 0 duplicates across 527 binaries. A full-workspace run is still
  impossible on this box — ten packages do not build on Windows — so that claim stays with CI.

