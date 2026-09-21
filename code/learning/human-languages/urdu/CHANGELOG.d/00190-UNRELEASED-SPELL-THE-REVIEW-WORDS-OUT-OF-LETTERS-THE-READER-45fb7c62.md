## Unreleased — spell the review words out of letters the reader has (HL-C242)

HL-C241 measured that Urdu's remaining closure debt could not be bought with
more letters or a better order: simulating the *entire* remaining alphabet at
the policy's own pace still left 40 violations, because eight glyphs are first
demanded at content lesson 27. The lever it named was prose, and this is that
pass.

- **Closure violations 41 → 4** (corpus-wide 518 → 481). Every lesson from
  chapter 7 to chapter 15 was printing review words in Nastaliq spelled with
  letters no lesson had taught yet. Those words are now printed the way the
  reader can actually use them — romanized — and the Nastaliq that remains in
  a lesson body is either its own headword or spelled entirely from the
  fifteen letters the ladder has handed over by that point.
- **The four survivors are deliberate and are not typos.** `UR-C08-puchhna`,
  `UR-C13-lal`, `UR-C13-kala` and `UR-C15-hawa` cite Persian and Arabic etymons
  in the source language's own orthography — *pursān-i ḥāl*, *lāl*, *mīrā*,
  *hawāʾ* — carrying ه, ء and the vowel marks ◌َ ◌ِ. HL-C241 classified these
  as a gap in the cousin-layer exemption rather than a spelling mistake:
  Arabic and Urdu are one script system as far as `SCRIPT_SYSTEMS` is
  concerned, so the "shows another script, never charged" rule never fires.
  Repairing them would damage the etymology layer, so they stay, and the
  remaining count of 4 is the honest floor of this pass rather than its
  failure. The fix belongs in the measurement.
- **Nothing was laundered into headwords.** `exposureExemptedGlyphs` is
  unchanged at 142 across the whole pass — the number that would have moved if
  script had been pushed into the exempt headword slot to make the count fall.
  `headwordsWithoutRomanization` stays at 0. `exposureOnly` rose 18 → 49,
  which is the honest half: those are lessons now clean whose only untaught
  glyphs sit in a headword the reader was given a romanization for.
- **Letters the reader cannot yet see are now named rather than shown.** The
  aspirate strand that runs through chapters 7–12 — the two-eyed **ھ** taking
  a base letter and turning it into an aspirate — survives intact, because
  **ھ** rides the headword of every lesson that teaches it. What changed is
  its partners: *jīm*, *ṭe*, *ṛe* and *che* are named in words where their
  shapes used to be printed untaught. Naming a letter the reader has been
  taught to say is more honest than showing a shape nobody taught them, and it
  is what the audio carried in either case.
- **Fixed two genuine forward references** found by the pass rather than
  introduced by it. `UR-C06-bolna` told the reader "you have already read
  **پ**" a full chapter before `UR-W07-pe` hands it over; it now points
  forward to the letter instead of backward to a lesson that has not happened.
  `UR-C05-practice` printed a four-line exchange of which the reader could
  decode one line, and now runs the exchange from its romanization and says
  which three pieces of it are inside the ladder today.
- **Etymon citations that were not orthography were transliterated.** Persian
  *guftagū* and *nastaʿlīq*, Arabic *fikr*, *qalam* and *kitāb*, and *namāz*
  were set in Nastaliq in bodies where the reader could not decode them; they
  are now printed the way they are said. The genuine source-language citations
  (the four above) were left alone.
- **One shared number adjusted, and none of its structure touched.**
  `tests/script-closure.test.ts` used to assert corpus violations `> 500` — a
  FLOOR on debt, which fails when the work succeeds. Marathi's runway (PR
  #13976) hit it at 498 and rewrote it properly before this branch landed: the
  floor became a live comparison against `measureScriptRamp`'s own violation
  count, so the module's claim is asserted as a relation rather than a
  magnitude, plus a ceiling on the absolute debt. That restructuring is theirs,
  not this tranche's. All this change does is move their ceiling 498 → 380 to
  the value measured after Urdu's 41 → 4 landed on top of their 44 → 0, with a
  one-line note saying which track moved it. Their comparison is untouched, and
  the relation still holds with room to spare (380 against a pace budget of 37).
  Persian's expectations, and every other track's, are untouched.

