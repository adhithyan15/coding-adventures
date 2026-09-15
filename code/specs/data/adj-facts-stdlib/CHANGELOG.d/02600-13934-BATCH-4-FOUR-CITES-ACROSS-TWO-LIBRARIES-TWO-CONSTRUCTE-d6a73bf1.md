- **#13934 batch 4: four `cites` across two libraries, two CONSTRUCTED header quotes corrected, and
  one row left uncited.** `language/onset-rime` (+2 — map, tape) and `geometry/circle-parts` (+2 —
  chord, circumference). Each sentence was verified as a **single rendered block of body text**: not
  fused across block tags, and not read out of `<meta>` content.

  *** THE PLAN THIS BATCH STARTED FROM WAS WRONG IN BOTH DIRECTIONS, AND BOTH ERRORS WERE MINE
  RATHER THAN THE PAGES'. *** I was about to record "`blast` is SITE CHROME" — a finding from the
  *In Practice* module, when `onset-rime`'s header cites blast from the *Tuning In to the Sounds in
  Words* article, which is also the table's encoded locator. On **that** page blast is real body
  text. And I was about to leave `tape` uncited as "names the word but not its onset or rime" —
  false; the passage gives both parts, `/t/` and `/Ape/`, in one contiguous block. My earlier verdict
  came from reading only the second of its two occurrences.

  **That `tape` correction is the first this session that GREW the work.** The running tally was six
  verdicts overturned, every one shrinking the claimed defect count — which was quietly hardening
  into an expectation. It is not a law.

  `blast` still ends up uncited, but for a reason that survives checking. Its markup is
  `<p>…the word <i>blast</i>:</p>` followed by `<ul><li>Onset (bl) – Rime (ast)</li>`. The lead-in
  names the word without the split; the list item gives the split without naming the word. Neither
  alone supports the row, and joining them constructs text the page never displays as one run —
  exactly the NOAA `<dt>Hail</dt><dd>Showery…</dd>` case from batch 3, resolved the same way.

  *** WHICH MEANS THE SHIPPED HEADER ALREADY CARRIED A CONSTRUCTED QUOTE *** — it presented that
  join as a verbatim span. `tape`'s header quote elided a bracketed stage direction with "…", and
  both `map`'s and `tape`'s flattened the page's **curly quotes to ASCII**. All are corrected to what
  the page renders. **That is the third and fourth header quote examined this session and the third
  and fourth found defective**, after `brain-parts`' hippocampus and `neuron-parts`' cell-body, which
  both altered glyphs and truncated. Header quotes are damaged as a RULE, not as an anecdote.

  `circle-parts`' **diameter row stays uncited, with the mechanism now identified rather than merely
  observed**: MathWorld renders math as `<img class="inlineformula" alt="pi">`, so dropping the tag
  DELETES THE SYMBOL and leaves "to a point radians away" — fluent, grammatical, and missing its
  term. Recovering the `alt` restores the sentence exactly as the header records it, so the evidence
  is real and reachable; what is unavailable is a span matching what a sighted reader *sees*, which
  is a π glyph. Reading the page in full confirms there is no second definition sentence — the only
  other candidate, "If is the radius of a circle or sphere, then .", is math-stripped to nonsense.
  This is not "the page is blocked": the page is fine and the fact is fine.

  **Two more bugs in my own harness, both found by reading output rather than by verifying against
  it.** `&quot;` was never decoded, so MathWorld's `the term &quot;circumference&quot;` came back NOT
  FOUND — trusting that negative would have been a false blocker, and trusting the extractor's
  *output* would have written `&quot;circumference&quot;` into a citation as verbatim. Then the fix
  that stops `<p>`/`<li>` fusion split blocks on `\n`, and raw HTML is full of newlines: MathWorld
  wraps mid-sentence (`describe a <a …>line\n segment</a> whose ends`), so the chord sentence was
  torn in half and reported NOT FOUND *by my own fix*. **That is the third time this session a fix of
  mine created a new failure mode**, after the adjacent-tag fix and the `HailShowery` fusion.
  Verbatim-verification cannot catch any of them, because it compares against the same broken
  extraction.

  `circumference` is the **first string in the stdlib to use the lexer's `\"` escape** (`STRING` is
  `"([^"\\]|\\.)*"`; zero prior uses). The whole round trip was validated end-to-end before being
  written: escaped in the `.adj`, a real quote in the value, re-escaped in the JSON.

  The four new tests are **anchored joint-binding pins** cut from real CLI output — bindings plus the
  full envelope plus the corroboration as one contiguous span ending on a closing quote. The prior
  assertions were `contains("readingrockets.org")` and a separate `contains("\"trust\":\"consensus\"")`:
  two independent scans over one blob, which cannot tell which answer a citation belongs to. The pins
  deliberately **do not claim row-scoped provenance** — `cites` is table-scoped, so every answer
  carries the same corroboration list; they assert that a given answer carries its evidence intact.

  All four were **directionally mutation-checked**: truncating `map`'s cite reddens map and tape;
  truncating `tape`'s reddens tape but leaves map green; corrupting only `circumference`'s escaped
  quotes reddens circumference and leaves chord green. The first run reported the onset-rime
  mutations GREEN and I nearly read that as weak pins — the pins were fine, but correcting the header
  quotes had made the *comment* byte-identical to the cite, so the harness's replace-first was
  mutating a comment. The harness was wrong while the subject was fine: the same shape as the HTTP
  308 false blocker, and the reason a negative result gets the same scrutiny as a positive one.

  Both `.query.adj` companions parse, run, and abstain correctly. 533 test binaries / 1596 tests
  green, clippy `-D warnings` clean.

