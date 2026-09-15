- **#13934 batches 5c+5d: six `cites` across three libraries, one header quote corrected.**
  `physics/light-colors` (+1), `physics/sound-properties` (+1), `biology/animal-homes` (+4 — bird,
  spider, rabbit, beaver).

  *** A PIN OF MINE WAS UNIQUE, ANCHORED, AND BOUND TO THE WRONG EVIDENCE. *** The `spider` test
  pinned bindings + envelope + `corroborations[0]` — but the cites were inserted bird, spider,
  rabbit, beaver, so index 0 is **bird's** sentence. The test asserted "spider's answer carries its
  full unelided sentence" while checking a span that did not contain spider's sentence at all.
  Restoring the old elided spider quote left it **GREEN**.

  Only the directional mutation found it, and it is the same defect as the chamber-branch pin earlier
  in this effort: uniqueness and anchoring say nothing about *ownership*. The pin now extends through
  `corroborations[1]`, spider's own, and the mutation reddens it.

  `light-colors` and `sound-properties` were the two libraries I had used as the example of
  "membership is not defect" — **and the exemplars were wrong**. Both are genuine members; their
  headers explicitly document these spans as corroborations ("A second span of the same primary
  university resource restates the primaries"; "The first two rows are ALSO corroborated by a primary
  U.S. government / academic source"). The general point stands — a header URL absent from every
  locator is a reason to look, not a finding — but not on the strength of these two.

  ON `sound-properties`' TRUST DIRECTION. Its envelope is LibreTexts at `trust consensus`; the
  NIH/Purves corroboration is authoritative. Since `cites` has no trust field, this puts a **stronger**
  source under a **weaker** envelope, which *understates* the corroboration and is harmless. The gap
  only bites the other way — a weaker cite under a stronger envelope would overstate the evidence,
  which is `kidney-parts` and only `kidney-parts`. The library's header had already reasoned this out
  correctly: the NIH sentence grounds only pitch and loudness, which is exactly why the envelope uses
  the LibreTexts span that fixes all three rows and why `trust` is honestly `consensus`. Nothing to
  correct there; the evidence simply never reached the data. The new test binds **timbre** on
  purpose — the one row the NIH sentence does *not* ground — to show the corroboration rides the
  table, not the row.

  `animal-homes`' spider header quote read "A spider web ... is a structure created by a spider…",
  eliding the page's whole alias list. That shortened form names its subject only if you supply the
  elided material yourself; the full 233-character sentence names both the subject and the row's
  value. Fourth elision found (after `tape`, `noble_gas`, `rectus_abdominis`) and the fourth where the
  unelided sentence was strictly better evidence. The `[1]` is a real rendered footnote marker and
  stays.

  Five directional mutations pass, including two that only a whole-list pin can catch: truncating the
  **last** corroboration reddens the whole-list pin while the prefix pin stays green, and a pure
  **reorder** of two middle entries does the same with nothing removed.

  All three `.query.adj` companions parse, run, and abstain correctly. 533 test binaries / 1608 tests
  green, clippy `-D warnings` clean.

