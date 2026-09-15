- **#13934 installment 4j: a quotation that quoted nothing.** **1** value line rewritten and **60**
  comment lines added against **13** removed, across **2** `.adj` files (61 added / 14 removed lines
  in total) — `biology/mitosis-phase-order` and `biology/mitosis-phases` — plus two e2e test files,
  one new verification tool, and the README section that lists it. **Ten mutations redden; four
  survive and are named below.**

  `mitosis-phase-order:43` shipped this as a `source`:

  ```
  The four phases of mitosis are Prophase ... Metaphase ... Anaphase ... Telophase.
  ```

  That string occurs **zero times** on the page it cites — not in the raw HTML, not in the
  tag-stripped text, not after whitespace collapsing. It is not a span that drifted or a span
  quoted loosely. **Three separate things were invented**, and each is worth naming because each
  is a different failure:

  | # | invention | evidence |
  |---|---|---|
  | 1 | the `" ... "` separator | occurs **0** times in the raw HTML and **0** times in the stripped text |
  | 2 | the terminal period | the page's paragraph is `<p>The four phases of mitosis are</p>` and ends at `are` |
  | 3 | the bare word `Telophase` | the page's fourth item reads `Telophase (divided into parts I and II)` |

  The first is the one that matters most. An ellipsis is an ordinary editorial mark, but placed
  **inside quotation marks** it stops being editorial and becomes a claim about what the page
  says. Three of them, standing in for four list items, turned a lead-in fragment into an
  apparent sentence.

  ### What the page actually does

  It carries `<p>The four phases of mitosis are</p>` — byte-exact, one occurrence — followed
  immediately by `<ol class="usa-list">` whose four top-level items are, in document order:
  Prophase, Metaphase, Anaphase, and `Telophase (divided into parts I and II)`.

  So the ordering this table encodes **is** asserted by the page. An `<ol>` is an ordered list and
  the order of its items is exactly that claim. But it is asserted as **structure**, and the
  `source` field holds **text**. No contiguous span of the page's text states the four phases in
  sequence, because each phase's label is separated from the next by that phase's own nested
  sub-list of events.

  The repair is the house answer installment 4c established on solfège and 4i applied again on
  `shape-composition`: **quote what the page says in one piece, and disclose the composition
  instead of smuggling it inside the quotation marks.** The `source` is now the paragraph. The
  header names the `<ol>`, its four items, and the fact that the ordering is read from list
  structure rather than from prose.

  ### What this does NOT decide

  Whether an ordered-list item is a verbatim span is a question in the same family as the held
  table-row one, and this installment does not answer it. The repair only removes what was
  invented and quotes the paragraph, which is indisputably one real span. It is recorded as a
  fourth case alongside table rows, for the owner.

  ### A false attribution next door

  `mitosis-phases.adj` said the page **"which states:"** the same composite. It does not. That
  file's own shipped `source` is fine — the chromatin sentence, byte-exact — but its header
  attributed to the page a string the page has never carried. **Two sites, not one**: the
  `which states:` attribution, and forty lines above it a smaller version of the same move —
  `the source's "The four phases of mitosis are ..." list`, an ellipsis inside quotation
  marks. Review caught the second after the first was fixed and this entry already said
  "Corrected."; `mitosis-phase-order`'s own header was quoting that stale sentence forward,
  so it is corrected too. Both are now fixed. Both files' shipped
  values are now byte-exact in the raw HTML; that is measured over these two files only, and no
  package-wide sweep figure is asserted here because the sweep is a working-tree instrument
  (#14444) and was not re-run.

  ### Why it survived: the pin was a hostname

  Nothing pinned this value's TEXT. The SEER assertion covering it was

  ```rust
  out.contains("training.seer.cancer.gov")
  ```

  a **host** — which the fabricated composite satisfied exactly as happily as the real span does.
  A provenance value that no test constrains is a value that can say anything. It is now pinned as
  text **joined to** its locator in one needle, so the two halves cannot drift into a text from one
  citation and a locator from another — installment 4i's round-1 finding, applying unchanged here.

  **Two corrections review made to this very paragraph.** It said *only*, and a second SEER
  assertion existed all along in `facts_mitosisphases_e2e.rs` — pinning that file's citation by
  **locator alone**, the same exposure one file over. It is now pinned as text too. And the joined
  needle was still open at its right end and said nothing about the citation **set**: a fabricated
  `cites` carrying the exact string this installment deleted passed all three assertions. Both
  needles now close on `"trust":"authoritative","corroborations":[]`, taken from the serialiser's
  real output rather than from memory of it. That is 4i's finding recurring, and the general layer
  is #14735.

  ### Mutants

  **Ten redden.** Five are one per byte the repair turned on — RESTORE (the original composite put
  back verbatim), PERIOD (the invented terminal period alone), WORD (one word of the real span
  changed), TRUNCATE (the span cut one word short) and EXTEND (the span run past what the paragraph
  contains). **RESTORE is the one that earns the pin**: it proves the new assertion catches the
  defect this installment repaired, which the hostname probe did not.

  Review added five more, aimed at the citation containers rather than the span: SEER-CITES (a
  fabricated `cites` on the repaired library), SEER-LOC-TAIL (its locator given a look-alike
  suffix, `cycle.html.attacker.example/`), SIBLING-CITES (a fabricated `cites` on
  `mitosis-phases`), EF-CITES (one on the composed `ordinal-numbers` library) and RULE-CITES (one
  on the rule itself). All five redden. The negative arm — the repaired tree, untouched — stays
  green.

  This count was **five** for three consecutive review rounds while mutants were being added, which
  is precisely the staleness the "measure after the last edit" rule exists to stop. Two names are
  deduplicated: round 2's EXTRA-CITES and round 3's SEER-CITES are the same mutation reached by
  different anchors, as are LOCATOR-TAIL and SEER-LOC-TAIL. Counting those twice would inflate the
  tally, which is the same sin pointing the other way.

  ### Re-runnable, not a working-tree instrument

  `tools/verify_seer_ol_span.py` ships with this change and is the second answer to #14444, after
  `verify_ncbi_pre_span.py`. It fetches the page, confirms the four top-level `<li>` items are
  still what the artifact assumes, and checks the shipped `source` against the raw HTML. Its exit
  codes separate the three things a checker must never conflate: **1** the artifact is wrong,
  **2** no verdict (the page could not be fetched, or has changed shape), **3** the instrument's
  own controls failed. `--self-test` drives the routing offline and fails if any of the four codes
  is unreachable. A fetch failure routes to **2**, never to **1** — reporting a network problem as
  a provenance defect is how a checker launders one into the other.

  ### A pin matched by a duplicate is a pin on the duplicate

  The strongest thing review found in this installment is not about the page at all. Round 2 closed
  the SEER needle and left the other composed library pinned by the bare host `ef.edu`; round 3
  showed a fabricated `cites` on `ordinal-numbers.adj` — carrying the exact composite this
  installment deleted, under an attacker-controlled locator — keeping every assertion green. So the
  `ef.edu` half was closed the same way as the SEER half.

  **That fix did not work, and the reason is the finding.**
  `mitosis-phase-ordinal-position.adj:64` carries its OWN INLINE COPY of the ordinal-numbers
  citation. A `contains` needle is satisfied by that copy, so it constrains the rule's duplicate and
  says nothing whatever about the composed library. Both mutants aimed at `ordinal-numbers.adj`
  SURVIVED the closed needle — including one that simply altered that library's own `source` text,
  which a duplicate hides best of all. No amount of tightening a needle fixes this: the needle is
  looking at the wrong copy.

  What closes MOST of it is an assertion that names no citation at all — **no `"corroborations":[{`
  anywhere in the derivation's output**. A duplicate cannot satisfy that, because it is a statement
  about the whole output rather than about a string in it. It is the structural shape #14735 asks
  for, and against fabricated `cites` it works on every library in the derivation: EF-CITES (the
  composed ordinal library), SEER-CITES (the library this installment repaired) and RULE-CITES (the
  rule itself) all redden.

  **FOUR mutants still survive, and the first count of this was wrong.** This entry said *one*.
  Measured with a positive control that reddens (the SEER locator, given the same look-alike
  suffix, is caught), the survivors are:

  | mutant | what it changes | suite |
  |---|---|---|
  | EF-DRIFT | the composed library's own `source` text | green |
  | EF-LOCATOR-TAIL | the composed library's locator, `…numbers-english/.attacker.example/` | green |
  | RULE-DRIFT | the rule's own `source` text | green |
  | RULE-LOC-TAIL | the rule's own locator, same suffix | green |

  The duplicate covers **symmetrically, in both directions**: the rule's inline copy satisfies the
  needle when the library drifts, and the library's citation satisfies it when the rule drifts. So
  the EF needle pins neither copy's text nor either copy's locator — both would have to be mutated
  together to redden it. It constrains nothing on its own.

  Which makes an earlier sentence in this entry false too: the EF half was **not** "closed the same
  way as the SEER one". For SEER that is measurable — LOCATOR-TAIL reddens. For EF it is not, and
  an attacker-controlled provenance locator on a library three other stdlib files import would ship
  green. The structural corroborations bound is what saves the *addition* case; nothing here
  constrains EF drift at all.

  This is a disclosure fix, not a code fix. Closing the gap means de-duplicating the rule's inline
  citation or asserting over parsed steps rather than over a flat string, which is #14745's
  subject and a different change. But a disclosure that undercounts its own gap by four is worse
  than the gap, because it tells the next reader the ground is firmer than it is.

  ### A correction to the triage

  The six remaining multi-block assemblies were characterised as values that assemble *complete
  prose sentences* from non-adjacent blocks. That holds for five of them. It does not hold for
  this one: the blocks here are a sentence **fragment** and four one-word list labels, which is
  why the invented separators were needed to make it read as prose. The classification was right
  about the class being actionable and wrong about what the class contains.

