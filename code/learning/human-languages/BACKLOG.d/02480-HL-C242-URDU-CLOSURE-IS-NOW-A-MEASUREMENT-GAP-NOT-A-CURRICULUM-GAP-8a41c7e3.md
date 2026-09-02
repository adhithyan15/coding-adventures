## HL-C242 — Urdu closure bottomed out at 4, and all four are the measurement's fault

HL-C241 proved redistribution had a floor of 41 and named prose as the only
remaining lever. The prose pass is done: **41 → 4**, corpus-wide 518 → 481.

The four that remain are `UR-C08-puchhna`, `UR-C13-lal`, `UR-C13-kala` and
`UR-C15-hawa`, and between them they carry exactly four glyphs — **ه** (U+0647),
**ء** (U+0621), **◌َ** (U+064E) and **◌ِ** (U+0650). None of these occurs in an
Urdu word anywhere in the track. They occur only inside deliberate Persian and
Arabic etymon citations spelled the source language's way: *hawāʾ*, *lāl*,
*mīrā*, *pursān-i ḥāl*.

**This is now the whole of Urdu's closure debt, and it cannot be fixed in the
curriculum.** The report's cousin-layer exemption — "shows another script,
context, never charged to the budget" — does not fire, because Arabic, Persian
and Urdu are one entry in `SCRIPT_SYSTEMS`. The fix is a rule, not a rewrite:
**an etymon citation set in a sibling language's orthography should be exempted
the way cousin script already is.** Whoever picks this up should expect the same
pattern in Persian and Arabic, and should check whether the Punjabi finding
(Sanskrit etymons set in Gurmukhi, where transliteration was the correctness
fix) is genuinely a different case — it is: Sanskrit is not written in Gurmukhi,
whereas Persian *is* written in Perso-Arabic, so Urdu's citations are correct
orthography and Punjabi's were not.

### What the pass did, so the next person does not redo it

Every violation from chapter 7 to chapter 15 was the same shape: a lesson body
printing a *previous lesson's headword* in Nastaliq, as review, using letters
the ladder had not yet handed over. The taught set at chapter 15 is fifteen
glyphs — ش ک ر س ا ل م ن ی ہ ں ے آ پ و — and the review vocabulary of chapters
8–15 (**حافظ**, **خدا**, **بھائی**, **پسند**, **مجھے**, **سفید**, **قمیض**,
**ٹھیک**) is spelled almost entirely outside it. Those are romanized now.

Three things are worth carrying forward:

- **`exposureExemptedGlyphs` is the number that proves the pass was honest.**
  It stayed at 142 through the whole rewrite. It is what would have moved if
  script had been pushed into the exempt headword slot to make violations fall,
  and it is the metric to check first when reviewing any future closure tranche.
- **Naming a letter beats showing it untaught.** The aspirate strand in
  chapters 7–12 survived intact because **ھ** rides the headword of every lesson
  that teaches it; only its partners (*jīm*, *ṭe*, *ṛe*, *che*) had to become
  names. No pedagogy was lost, and no new lessons were needed.
- **A naive substring replace corrupts Urdu words.** The first draft of this
  pass replaced the bare glyphs ج and ھ everywhere and turned **مجھے** into
  `م<jīm><he>ے` across eleven lessons, and wrote markdown emphasis into
  `hl-activity` JSON answers. Any scripted rewrite over this corpus must match
  whole words with Arabic-script lookarounds, must leave `hl-activity` payloads
  to hand editing, and must be read as a diff before it is trusted.

### Still open, and measured rather than guessed

- **Eight glyphs still have pedagogy that earns no closure credit** — the list
  in HL-C241 is unchanged (`UR-C06-bolna`→ب, `UR-C07-sochna`→چ, and so on).
  Making them creditable needs properly declared script segments, which cost
  new lessons in chapters that are *already* over the 12-atom budget (Urdu 1, 3,
  4, 5, 6, 7). The right order is **chapter splits first, letters second** — and
  that is a tranche of its own, not a rider on this one.
- **`neverTaughtGlyphs` fell 22 → 20 and `shownGlyphs` 37 → 35.** The track now
  shows less Nastaliq than it did, which is the correct trade at this point on
  the ladder but is a debt to repay by teaching letters, not by reprinting them.

### A floor on debt is the wrong shape, and this file proved it twice

`tests/script-closure.test.ts` asserted corpus violations `> 500`. That is a
FLOOR on debt in a programme whose goal is zero debt, so **the test fails when
the work succeeds** — and it had already failed that way once, at `> 5`, when
the Chinese, Japanese and Gujarati tranches removed whole tracks. The comment
beneath it recorded that failure, drew the right conclusion ("debt assertions
belong the other way up"), inverted the companion assertion, and left the
violations line as it was. It then failed a second time, for exactly the reason
already written down next to it.

It is now a relation rather than a magnitude: *while any closure debt exists,
some of it must sit in a lesson the pace budget calls gentle.* That is the
module's actual point — two instruments, not one number — and it survives the
corpus going 481 → 0 with nobody editing a shared file. The zero case is an
explicit branch so success reads as success.

The general lesson, for the other ratchets in this suite: **a burn-down number
should be asserted as a ceiling or as a relation, never as a floor**, and when
a comment predicts a failure mode, the fix belongs in every assertion that
shares it rather than only the one that happened to break that day.
