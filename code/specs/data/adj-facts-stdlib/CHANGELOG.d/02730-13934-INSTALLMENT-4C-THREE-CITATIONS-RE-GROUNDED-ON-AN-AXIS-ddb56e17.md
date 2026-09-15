- **#13934 installment 4c: three citations re-grounded, on an axis no screen here could see.**
  **4 value lines** across 3 `.adj` files — `biology/cell-division-genetic-outcome`,
  `music/solfege`, `music/solfege-alt-name` — and 72 comment lines beside them; the rest of the diff
  is prose (this stdlib's README row, the solfège query companion, three test files, this entry). **Five mutations redden**, the
  fifth being the deletion of the round-2 disclosure marker itself, so the disclosure is load-bearing
  rather than decorative. Contiguity **75 → 72** on the shipped tree, verified by diffing the two
  sweeps' non-contiguous sets: the three that left are the three repaired, none entered. The
  unjudgeable (36) and fetch-failed (1) sets are identical member-for-member across the two runs, not
  merely equal in count — equal totals hide swaps, which is how a delta was nearly published by
  subtraction earlier in this effort.

  **A DASH flattening.** `cell-division-genetic-outcome` shipped `--` where its page has an em dash.
  Every screen in this effort keyed on **quote** characters, so none of them could see this. It was
  found by hand-diffing one triage candidate.

  **The punctuation-general screen built to catch it returned a FALSE ZERO on that very defect**, and
  I ran it without positive controls — the rule this package has applied to every other screen and
  wrote into its own notes. It mapped each dash *character* to a marker, so `--` read as two marks
  against the page's one and never matched. With runs collapsed it finds exactly the one case, and
  agrees with the hand diff. A mark is one mark however it is spelt.

  **The solfege pair had dropped a footnote marker.** Both libraries *shipped* the same 170-character
  sentence (186 as repaired), and both had removed the Wikipedia marker `[2]` from the middle of the
  syllable list and truncated before " (see below).". **The marker is restored, not elided.** It sits
  between "tonic sol-fa)," and "re, mi, fa", so no contiguous span covers those syllables without it
  — and including it is correct under *both* readings of the open document-text versus
  rendered-display question, since `[2]` is in the page's text *and* displayed to a reader. Dropping
  it was a silent tidy, the same judgement as reproducing a source typo in 4b.

  **But that sentence never mentions a scale degree, and review caught it.** `solfege_degree`'s rows
  are `(do, 1) … (ti, 7)`, and the syllable sentence only lists seven syllables in order — so "1" was
  an ordinal **inference from list position**, not a claim the citation made. This library's own
  header already conceded it, naming a "movable-do degree table" that was not in the envelope at all.
  I fixed that sentence's two real defects and never asked whether it was the *right* sentence:
  **the first-workable-span fault, for the third time in this effort.**

  The page states the mapping outright, marker-free and unique — "In the movable do system, each
  solfège syllable corresponds not to a pitch, but to a scale degree: The first degree of a major
  scale is always sung as "do", the second as "re", etc." (the backslashes in the shipped value are
  the `.adj` string literal's, not the page's; round 3 caught this paragraph writing them as though
  they were) — so that is now the `source`, and the
  syllable sentence is a `cites` grounding the **set and order** of the syllables, which is what it
  actually says. That is the `source` + `cites` shape `cell-division-genetic-outcome` already uses.
  `solfege-alt-name` keeps the syllable sentence as its own `source`, because its row (`do → doh`)
  *is* stated there — but its header claimed to reproduce "the SAME span already quoted inside
  `solfege.adj`'s own `source` field", which this change falsified, so that header is corrected in
  the same edit. The pin now spans bindings → envelope → corroboration, so mutating **either** repair
  reddens it.

  **Round 2 found that fix incomplete — the same fault, one level down.** The movable-do sentence
  states the mapping for `do` and `re` and then says "etc.", so it grounds **2 of the 7 rows**
  outright and the other five remain an ordinal inference. The defect narrowed from seven rows to
  five; it did not close, and the entry above was written as though it had. The page *does* pair all
  seven — but only in a movable-do degree table whose cells extract as separate blocks, which is the
  **CROSS-BLOCK** class held open on #14111 below. Citing it would silently decide the question this
  effort has been holding for the owner, so it is deliberately not cited.

  The residual gap is **disclosed** instead. The header says which span carries which rows, every row
  is marked `STATED` or `COMPOSED`, and a new test queries `sol` — a *composed* row — and pins the
  answer a consumer actually receives. The other pin queries `do`, the one row the envelope grounds
  outright, so nothing in CI could see the gap.

  **Round 3 corrected the disclosure itself, which misdescribed the artifact.** I had written that
  those citations "never name that syllable" — but the corroboration lists `so(l)` outright, and the
  composition *depends* on it doing so. **The gap is the DEGREE, not the syllable.** Understating
  evidence is the safe direction, but a disclosure that misstates what a consumer receives is the
  same prose-drift fault the disclosure exists to prevent. The header, the row marker, the test's
  name and its control were all restated. That control is now the digit rather than a phrase: strip
  the binding the answer is disclosing, and nothing resembling `5` may remain in the citations — a
  phrase needle ("the fifth") would have passed against a span stating the degree numerically, which
  is the shape a repoint would most likely take. The test also reads the shipped library for its
  `COMPOSED, not stated` marker, so the disclosure cannot quietly outlive the situation it describes.

  Round 3 also found a **fifth** artifact still carrying the over-claim after the entry said four
  were fixed: `music/solfege.query.adj`, the shipped worked example, still told a reader "the engine
  never invents a degree it cannot cite" and handed them `sol → 5` unmarked.

  **And the round-1 header correction had reached one artifact of four.** `solfege-alt-name.adj`'s
  header still said the sentence sat in `solfege.adj`'s **`source`** field; so did this stdlib's
  README row and that library's test module doc; and its pin comment still called the sentence
  "170-character" and a peer `source`. That is the **fourth** time in this effort a claim was
  retracted in the changelog and left standing in the artifacts. All four are fixed here.

  **The meiosis pin queries `meiosis`, not the companion's first query (`mitosis`).** The repaired
  value is the meiosis `cites`; pinning mitosis would have tied an answer about genetic identity to
  evidence about gametes. **Fifth time** the authored `.query.adj` has not covered the row the
  repaired evidence grounds — worth expecting, not rediscovering.

  *Consumer note:* for a meiosis answer, `citations[0]` is still the table's **mitosis** `source`; the
  per-row evidence is in `corroborations`. The pin spans the bindings through that corroboration for
  exactly that reason. Recorded on #14124.

  ### The band changed shape a third time, and the backlog is partly blocked

  4a was sentence-tidying, 4b was elision, 4c is dash-and-marker. Sampling the five highest-overlap
  candidates gave **five different situations with only one actionable**, so the whole backlog was
  classified rather than sampled further:

  | class | count | status |
  |---|---|---|
  | OTHER | 49 | needs individual reading |
  | **CROSS-BLOCK** | **16** | **blocked — is a table row a verbatim span?** |
  | **LATEX-MATH** | **7** | **blocked — document text or rendered display?** |
  | MARKED-ELISION | 2 | one of them is the ungroundable `mitosis-phase-order` column |
  | DASH | 1 | repaired here |

  **23 of 75 are blocked on a decision rather than on work** — roughly four installments that cannot
  be started. And the first version of that classifier found **none of the 16** — its five hits and
  the sixteen are disjoint sets. It detected table rows by looking for `" | "`, which missed all
  **twelve** AQI values (six in `air-quality-index`, six in `aqi-category-color`) whose cells the
  extractor joins with plain spaces, plus four others, and instead flagged five pipe-joined values
  that are each contiguous within a single block. `Red Unhealthy 151 to 200 Some members of the
  general public may…` is a row of EPA's table, not a sentence, and no `|` appears anywhere in it.
  **That is the same character-keying mistake the quote screens made seven times**, and the
  structural replacement — contiguous in the joined page text but present in no single block — needs
  no separator at all.

  Round 2 caught the *arithmetic* too. Round 1's finding was that "found 5, missing six" did not
  reconcile with 16; I closed it by rewriting the sentence until it added up, instead of re-reading
  `cls75.txt` against `cls75b.txt` — which shows the two sets are disjoint. **Making a number
  consistent is not measuring it**, and that is exactly what the finding had said.

  *Contiguity numbers are measured over* `source`/`cites` values of at least 20 characters with a
  resolvable locator (**562 of 568**), compared after stripping tags with **no separator inserted at
  tag boundaries**, unescaping entities, NBSP→space and collapsing whitespace. The sweep quoted here
  was re-run *after* the round-1 and round-2 restructures; the earlier one examined 561 values and
  had never seen `solfege.adj`'s new `cites` at all.

  **A hole in that oracle, recorded rather than fixed here.** Stripping `<[^>]*>` leaves Parsoid
  `data-mw` attribute payloads in what the screen treats as page text — on the Wikipedia Solfège page
  the extract contains raw wikitext (`'''Ut'''</u> queant laxīs&nbsp;…`) that no reader ever sees. A
  future citation could therefore be certified "contiguous" against a machine-readable attribute blob
  rather than displayed content. None of the values in this installment rely on it — all **four** are
  body-text spans confirmed in the raw markup — but it is the same shape as the character-keying
  mistakes above and belongs on #14111.

  **A blind spot in the desync screen, found by using it.** The header I wrote in round 2 quoted the
  new envelope with its INNER quotation marks stripped — the page reads `always sung as "do"`, the
  header read `always sung as do` inside quotes of its own — and the screen reported **0**. It
  normalizes every quote codepoint to one sentinel in order to find *where* to look, so a header with
  no character where the value has `"` never matches even normalized, and the raw byte comparison
  that would prove the defect never runs. Same shape as every other blind spot here: the instrument
  could not see the class it was aimed at. The header now quotes only the clause with no inner quotes
  and reproduces the rest in row comments as unwrapped page text, checked by a fragment control with
  a **negative arm** — the stripped forms must be *absent*, or the control proves nothing. Filed on
  #14111. **361 of the stdlib's 362 library `.adj` files carry a quoted fragment in a comment** and
  none has been swept under a deletion normalization, so the scope is unmeasured — the "81" first
  written here was 82 minus 1, and that 82 counts a different population (#14099's obsolete-rationale
  headers), which `solfege.adj` was never in. Subtracting from someone else's count is the same fault
  as the AQI arithmetic two paragraphs down. There is a second axis on the same screen: the header's
  remaining `cites` quote carries a MARKED elision and wraps across two `%` lines, and a line-scoped
  check cannot see a wrapped quote at all. Both axes are on #14111.

  **`cargo test -p adj-lang-cli -p adj-lang`: 537 test binaries, 1919 tests run, 0 failed.** Clippy
  `-D warnings` clean over them; both `adj_stdlib_*` gates exit 0; the header/data desync screen
  reports 0. (A static `#[test]` grep over the same two packages gives **1920** — one declared test
  does not appear in the run and I have not identified which. Recorded rather than reconciled: making
  the two numbers agree by choosing one is the fault this entry already documents twice.)

  *The scope of that figure is stated because the one earlier installments carried — "533 test
  binaries / 1655 tests" — names no package set and does not reproduce, and forwarding a number whose
  scope I cannot state is the same fault as forwarding one I did not take. The workspace-wide claim
  belongs to CI. Two things blocked measuring anything wider here, both worth recording: this box's
  disk hit **zero free** mid-round (the cargo target directory in this worktree alone had reached
  69 GB) — a rust-lld failure from an exhausted disk is indistinguishable from a missing-symbol one
  until you read the log, two packages written off as native-binding failures turned out to be disk
  casualties, and it **truncated `music/solfege-alt-name.adj` to zero bytes mid-edit**, caught only
  because the tests went red and restored from the index. And ten packages genuinely cannot build on
  this box for reasons unrelated to this change: `tcp-reactor` (Unix file descriptors) and nine
  `*-python`/`*-ruby`/`*-node`/`*-napi` bridges that link against libpython, libruby or Node's
  N-API.*

