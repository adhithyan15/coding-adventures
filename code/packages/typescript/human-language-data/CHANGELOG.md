# Changelog

## Unreleased

### Fixed - a lesson id is validated at parse (HL-C211)

- `lessonId` is interpolated RAW into `\label{lesson:<id>}` and into the `% canonical-lessons:` header of every generated `.tex` — the sibling of the `chapters.json` label hole closed in HL-C209, and found by the security review of the very next tranche.
- Demonstrated: an id of `X}\write18{id}{` closed the brace and emitted a live control sequence into a file XeLaTeX compiles in CI. Builds run without `--shell-escape` so `\write18` is refused, but `\input` and `\openout` are not — an arbitrary local file read into a published PDF.
- `parseLesson` now rejects any id outside `/^[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*$/`, the same shape `ACTIVITY_ID` has always enforced. All 2,823 corpus ids pass.
- Three existing tests had to move rather than be relaxed: two control-character tests were laundering a hostile id through `parseLesson`, which now refuses it, so they set the field directly and keep the render helper as their subject. `info-dump.ts` reads the RAW frontmatter id rather than the validated one, and that test now poisons the field the helper actually reads — which is the reason that helper still needs its own guard.
- One book-cli fixture was rewriting `id: TEST-C01-hello` into a non-ASCII id by a blanket `replaceAll`; narrowed to the headword, gloss and body it was actually testing.


### Fixed - a chapter's LaTeX label is validated at load (HL-C209)

- `chapters.json` `label` was the one author-controlled field the book generator interpolated RAW into `\label{...}`; titles go through the LaTeX escaper, output paths through `safeOutput`, activity ids and script commands through their own regexes.
- A security review demonstrated a label of `ch:x}\immediate\write18{id}{` emitting a live control sequence into a generated `.tex`. Builds run plain `latexmk -xelatex` with no `-shell-escape`, so `\write18` is refused today — but `\input` and `\openout` are not, and a compiler flag is not a property this module can see or keep.
- `loadTrackChapters` now rejects any label outside `/^[A-Za-z0-9:_-]+$/`, which accepts all 900 committed labels and every convention in use.
- Pinned with a test that also proves the guard is falsifiable, and self-tested against the review's own hostile label before being trusted.


### Added - the completion plan is computed, not typed (HL15, HL-C208)

- Add `completion-plan.ts`: the work queue is now a pure function of the measured deficit, built from the level gate, the script-closure report and the external exam inventories rather than from a hand-ordered list in `BACKLOG.md`.
- Add `plan-cli.ts` and an `npm run plan` target, kept separate from `report` so picking up the next item does not mean reading 100 lines of diagnostics.
- Add `listExamInventories`, which reads each inventory's own declared `language`/`level` rather than parsing the filename — the file for Spanish is `exam-inventory-es-a1.json`, so a filename-keyed queue would have reported Spanish's A1 target as missing and queued it to be written twice.
- Order by three mechanical keys: level rank (the floor is universal), family priority, then furthest-behind-first — and ROTATE across tracks so every language moves once before any moves twice.
- Extend the definition of done with two criteria the four-criterion gate did not carry: external exam-point coverage, and script closure.
- Measured today: 22 tracks, 89 enumerable items, ~10,172 projected to C2; 459 glyphs shown but never taught corpus-wide; 1 of 132 exam inventories written.


### Changed - the continuity walk is no longer quadratic (HL-C205)

- Index forward-reference candidates by their leading word-run, so each lesson is asked only about words its own text can reach instead of about every word its track teaches.
- Replace the per-word `(?<![\p{L}\p{M}-])...` regex — ~330µs to build and first run, ~2,700 of them per report — with one shared character class and an `indexOf` boundary walk.
- `measureContinuity` 2,065ms to 218ms on the 2,771-lesson corpus; `report --format json` 3.94s to 1.79s; the full test suite's CPU time 93.8s to 79.7s.
- Report output is byte-identical in both formats, verified against a differential check of 379,834 real-corpus pairs and 60,000 adversarial Unicode cases.
- Skip the rest of a word-adjacent run when an occurrence is rejected, so a body of near-misses cannot drive the matcher quadratic — 3.9s to 2.3ms on the case a security review constructed, which is faster than the regex it replaced.
- Pin the boundary rule with eight tests covering glued words, hyphens, combining marks, astral-plane neighbours, multi-word headwords and non-Latin script, plus a ninth that fails at 8.2s without the run skip and passes at 0.3s with it.

### Added - source-verified Gujarati આ (HL-C09FH)

- Cite t30apps.com's version-1.0 આ animation and record the full અ sequence before the added trailing ā stem.
- Preserve the source's two-lift evidence while retaining its explicit warning that the demonstrated order is one variant.
- Reduce measured HL-C09 debt to 68 entries and queue Gujarati ઇ next.

### Added - source-verified Gujarati અ (HL-C09FG)

- Cite t30apps.com's version-1.0 અ animation and record its joined body before the separately descended right stem.
- Preserve the source's one-lift evidence while retaining its explicit warning that the demonstrated order is one variant.
- Reduce measured HL-C09 debt to 69 entries and queue Gujarati આ next.

### Added - source-verified Cyrillic я (HL-C09FF)

- Cite RussianIrina's 12:13–12:21 lowercase я school-hand demonstration and record its one-stroke rise-to-loop-to-leg order.
- Explain how the curved entry, narrow loop, and slanted leg fit Noto Sans Cyrillic's straight right upright, broad upper bowl, and angular lower-left leg.
- Reduce measured HL-C09 debt to 70 entries, complete the Cyrillic lowercase inventory, and queue Gujarati next.

### Added - source-verified Cyrillic ю (HL-C09FE)

- Cite RussianIrina's 11:44–11:58 lowercase ю school-hand demonstration and record its one-stroke stem-to-connector-to-oval order.
- Explain how the looped entry, diagonal connector, and cursive oval fit Noto Sans Cyrillic's straight upright, horizontal middle bar, and wide oval.
- Reduce measured HL-C09 debt to 71 entries and queue Cyrillic я next.

### Added - source-verified Cyrillic э (HL-C09FD)

- Cite RussianIrina's 11:25–11:32 lowercase э school-hand demonstration and record its outer-before-tongue order and one lift.
- Explain how the narrow rounded curve and hooked right-to-left tongue fit Noto Sans Cyrillic's broad open-left curve and straight middle bar.
- Reduce measured HL-C09 debt to 72 entries and queue Cyrillic ю next.

### Added - source-verified Cyrillic ь (HL-C09FC)

- Cite RussianIrina's 11:16–11:20 lowercase ь school-hand demonstration and record its one-stroke, zero-lift stem-before-bowl order.
- Explain how the narrow handwritten entry and rounded bowl fit Noto Sans Cyrillic's straight upright and wide closed lower bowl.
- Reduce measured HL-C09 debt to 73 entries and queue Cyrillic э next.

### Added - source-verified Cyrillic ы (HL-C09FB)

- Cite RussianIrina's 10:45–10:56 lowercase ы school-hand demonstration and record its two-stroke, one-lift body-before-right-stem order.
- Explain how the handwritten narrow entry loop and curled exit fit Noto Sans Cyrillic's straight left upright, wide closed lower bowl, and separate right stem.
- Reduce measured HL-C09 debt to 74 entries and queue Cyrillic ь next.

### Added - source-verified Cyrillic ъ (HL-C09FA)

- Cite RussianIrina's 10:34–10:38 lowercase ъ school-hand demonstration and record its one-stroke, zero-lift flag-to-stem-to-bowl order.
- Explain how the handwritten narrow entry loop and rounded shoulder fit Noto Sans Cyrillic's broad top flag, straight stem, and closed lower bowl.
- Reduce measured HL-C09 debt to 75 entries and queue Cyrillic ы next.

### Added - source-verified Cyrillic щ (HL-C09EZ)

- Cite RussianIrina's 10:17–10:25 lowercase щ school-hand demonstration and record its one-stroke, zero-lift left-to-middle-to-right-to-tail order.
- Explain how the handwritten diagonal rounded joins and looped exit fit Noto Sans Cyrillic's three straight stems, baseline bars, and short right descender.
- Reduce measured HL-C09 debt to 76 entries and queue Cyrillic ъ next.

### Added - source-verified Cyrillic ш (HL-C09EY)

- Cite RussianIrina's 09:49–09:57 lowercase ш school-hand demonstration and record its one-stroke, zero-lift left-to-middle-to-right order.
- Explain how the handwritten diagonal rounded joins and rising exit fit Noto Sans Cyrillic's three straight stems and horizontal baseline joins.
- Reduce measured HL-C09 debt to 77 entries and queue Cyrillic щ next.

### Added - source-verified Cyrillic ч (HL-C09EX)

- Cite RussianIrina's 09:24–09:28 lowercase ч school-hand demonstration and record its one-stroke, zero-lift short-stem-to-bowl-to-long-stem order.
- Explain how the narrow rounded handwritten bridge and rising exit fit Noto Sans Cyrillic's shorter left stem, shallow bowl, and full-height right stem.
- Reduce measured HL-C09 debt to 78 entries and queue Cyrillic ш next.

### Added - source-verified Cyrillic ц (HL-C09EW)

- Cite RussianIrina's 09:05–09:10 lowercase ц school-hand demonstration and record its one-stroke, zero-lift left-stem-to-right-stem-to-tail order.
- Explain how the rounded joined handwritten body and looped exit fit Noto Sans Cyrillic's squared U-like body and short right descender.
- Reduce measured HL-C09 debt to 79 entries and queue Cyrillic ч next.

### Added - source-verified Cyrillic х (HL-C09EV)

- Cite RussianIrina's 08:42–08:49 lowercase х school-hand demonstration and record its left-run-first, two-stroke, one-lift crossing order.
- Explain how the two facing handwritten curves and rising exit fit Noto Sans Cyrillic's four straight diagonal arms.
- Reduce measured HL-C09 debt to 80 entries and queue Cyrillic ц next.

### Added - source-verified Cyrillic ф (HL-C09EU)

- Cite RussianIrina's 08:16–08:26 lowercase ф school-hand demonstration and record its stem-first, one-lift, linked-left-loop-to-right-loop order.
- Explain how the narrow joined handwritten loops and rising exit fit Noto Sans Cyrillic's straight ascender-descender and two wider upright bowls.
- Reduce measured HL-C09 debt to 81 entries and queue Cyrillic х next.

### Added - source-verified Cyrillic у (HL-C09ET)

- Cite RussianIrina's 07:50–07:55 lowercase у school-hand demonstration and record its one-stroke, zero-lift upper-body-to-looped-descender order.
- Explain how the narrow rounded handwritten body, lower loop, and rising exit fit Noto Sans Cyrillic's printed upper arms and broad left-curving terminal.
- Reduce measured HL-C09 debt to 82 entries and queue Cyrillic ф next.

### Added - source-verified Cyrillic т (HL-C09ES)

- Cite RussianIrina's 07:29–07:36 lowercase т school-hand demonstration and record its one-stroke, zero-lift two-arch Latin-m-like order.
- Explain how the rounded handwritten arches and rising exit fit Noto Sans Cyrillic's printed central stem and horizontal top bar.
- Reduce measured HL-C09 debt to 83 entries and queue Cyrillic у next.

### Added - source-verified Cyrillic с (HL-C09ER)

- Cite RussianIrina's 07:04–07:08 lowercase с school-hand demonstration and record its one-stroke, zero-lift counterclockwise open-curve order.
- Explain how the tall, slightly slanted handwritten curve and rising exit fit Noto Sans Cyrillic's wider upright C-like outline.
- Reduce measured HL-C09 debt to 84 entries and queue Cyrillic т next.

### Added - source-verified Cyrillic р (HL-C09EQ)

- Cite RussianIrina's 06:46–06:52 lowercase р school-hand demonstration and record its one-stroke, zero-lift stem-before-bowl order.
- Explain how the open cursive shoulder and baseline exit fit Noto Sans Cyrillic's straight descender and closed rounded bowl.
- Reduce measured HL-C09 debt to 85 entries and queue Cyrillic с next.

### Fixed
- A dotted circle carrying a combining mark now joins that mark's script run when
  inline markdown is rendered to LaTeX. U+25CC DOTTED CIRCLE has
  `Script_Extensions=Common`, so it matched no script and was emitted outside the
  run — handing it to the Latin body font, which has no such glyph. The first
  build of HL12's Indic recognition segments logged 184 `Missing character`
  warnings, one per use, and printed nothing where the character being taught
  should have been. The dotted circle exists precisely to be the base a combining
  mark is shown on, so when the next character belongs to a run, it belongs to
  that run too.

### Changed
- `tests/script-closure.ts`'s corpus assertion no longer pins Telugu, Kannada and
  Malayalam at **zero** script lessons. That was true and was the defect; it now
  asserts every Indic track teaches letters, with Tamil still teaching the most
  because Tamil is the only one of them with a cited stroke order.

## Unreleased

### Added — Letter ledgers (HL11 section 4)

- `loadLetterLedgers()` reads `data/scripts/<script>-ledger.json`: the order a
  reader meets each script's letters, ordered by the words they make writable
  rather than by the traditional recitation order.
- `validateLetterLedger()` checks a ledger against the corpus that justifies it —
  contiguous positions, glyphs belonging to the named script, no vowel sign
  before a base letter, families kept together, every claimed unlock naming a
  lesson that exists, and unspent letters. Report-only.
- `summarizeLetterLedger()` publishes `firstWritableWord` and the writable-word
  curve. A word, not a letter count: twenty taught letters is not something a
  reader can feel, and writing *thank you* is.
- Each ledger row carries its **code point** beside its Unicode name. A rendered
  glyph is not an audit surface — it can be a lookalike, and it can carry code
  points that render as nothing — so the validator rejects a multi-code-point
  glyph, a code point disagreeing with the glyph, and a name from the wrong
  script. Without the first of those the two Unicode checks are satisfiable by
  different parts of one string: the script test is unanchored and the combining
  test is anchored.
- A ledger whose `tracks` match no loaded lesson now reports
  `ledger-unlocks-unverified` rather than passing silently. One mistyped track
  name would otherwise make the only check for fictional unlock claims vanish
  while the report still read zero.
- `loadLetterLedgers()` shape-checks each ROW, not just the two top-level
  arrays. The validator reads `glyph`, `codePoint`, `unicodeName` and `unlocks`
  off every row before it checks anything, so guarding only the arrays moved the
  unhandled TypeError down a level instead of removing it.
- Two positions sharing a `unicodeName` is an error. The code point pins a row to
  a character; this pins a name to one row, so a row duplicated and half-edited
  cannot leave two positions claiming to be the same letter.
- `loadScripts()` now skips `*-ledger.json`, case-insensitively. Both files sit in `data/scripts` and
  carry the same `script` key, so reading both into one map would have had one
  silently overwrite the other — decided by filename sort order.

### Added — Script closure (HL11)

- `measureScriptClosure()` asks the question the glyph budget cannot: for each
  glyph the reader is asked to read, had an earlier lesson taught it? Wired into
  the gap report, always present, report-only.
- First measurement: **932** lessons across 16 non-Latin tracks show a glyph
  nobody taught, and **12 of those 16 teach no letters at all**. The pace budget
  flags 61. A track can satisfy a cap on speed while teaching nothing.
- Exposure is drawn mechanically: a headword is exposure when the lesson declares
  a `romanization`. **489** native-script headwords carry none, so they are
  load-bearing — and each becomes exempt the moment somebody writes down how to
  say it, which is a real improvement rather than a way to hide from the number.
- Two numbers watch the exemption. `exposureOnly` counts lessons it flipped to
  clean (49); `exposureExemptedGlyphs` counts what it actually removed, including
  from lessons that violate anyway (**1,997**). The lesson count alone cannot see
  a lesson reporting five untaught glyphs while fifteen more were exempted.
- A track whose declared script is unknown is reported as UNMEASURED by name,
  never skipped. Both "genuinely Latin" and "unrecognised" used to fall out of
  the report identically, which is the silent zero this module exists to prevent.
- `belongsToAny` replaces `systemOf` at the two classification sites.
  `Script_Extensions` is set-valued, so the shared Vedic and Indic combining
  marks belong to Devanagari *and* to Bengali, Kannada, Malayalam, Tamil and
  Telugu at once — and first-match attribution gave every one of them to
  Devanagari, silently dropping them from every other abugida.
- `SCRIPT_SYSTEMS` is exported frozen. The regex matchers are derived from it
  once at module load, so a consumer adding a script afterwards would pass
  membership tests that `belongsToAny` never learned — and the track would report
  zero debt while appearing measured.
- `SCRIPT_SYSTEMS` and `systemOf` are exported from `ramp.ts` so the two script
  measurements share one definition of what belongs to a script.

All notable changes to `@coding-adventures/human-language-data` are documented here.

## [Unreleased]

### Added - A1 grammar coverage reaches 85/85 (HL-C128)

- Add Spanish chapters 262-266, closing the last four points on the A1
  inventory: the ordinals, `uno...otro`, the infinitive as subject, and
  word-order flexibility. **Coverage 81/85 -> 85/85 (100%).**
- 262: `segundo` is Latin `secundus`, which meant FOLLOWING, from the verb to
  follow -- so second place is literally the one that comes after, and English
  `second` is the same word. `primer`/`tercer` drop their `o` before a
  masculine noun, which is the THIRD sighting of that habit after `muy` and
  `mal` -- three makes a rule rather than a list of exceptions.
- 263: `otro` is Latin `alter`, the other of TWO rather than any other, which
  is exactly the job it does in this pattern. It takes no article.
- 264: the infinitive names an activity where English reaches for `-ing`. The
  gerund cannot do this job -- `comiendo es bueno` is wrong -- because the
  gerund is for an action underway, not a name for the activity.
- 265: the pieces that say when and where can move, and the front of the
  sentence is the emphatic position. This is possible only because the verb
  ending already says who acts and the personal `a` marks who receives, so
  nothing depends on position to keep the roles straight.
- 266 closes the level on the thread that has run through it: Latin's endings
  fell away, and Spanish built replacements out of small audible things -- an
  article, a preposition, a comma, and a fixed emphatic position.
- The "worst category first" assertion moved to a fixture. With every category
  at 100% the real report can no longer demonstrate the property -- its
  ordering falls back to the alphabetical tie-break -- so asserting its first
  line would pin the tie-break while claiming to pin the sort.

### Added - the four rules the book never stated (HL-C128)

- Add Spanish chapters 257-261. **A1 coverage 77/85 (91%) -> 81/85 (95%)**.
- These four points were a different problem from every batch before them.
  Each names something the book DEMONSTRATES on nearly every page and had never
  once stated, so the reader obeys the rule perfectly and cannot answer a
  question about it. Every lesson therefore has the reader do the thing first
  and names it afterwards, rather than opening with a rule.
- 257: the plain present has carried TWO readings since the first verb -- the
  habit and this moment -- so `estoy comiendo` was never required for "right
  now". It is for insisting, which is why an English speaker who reaches for it
  every time sounds oddly emphatic.
- 258: the verb ending agrees with the subject. Stated for the first time after
  two hundred chapters of the reader obeying it -- and it explains something
  taught at chapter 16 without a reason: dropping `yo` is not a habit, it is a
  CONSEQUENCE, because the ending already carried the subject.
- 259: `mucho` agrees in front of a noun (`mucha agua`, and `agua` is the test
  case the reader already owns) and freezes after a verb (`trabaja mucho`),
  because a verb has no gender to agree with.
- 260: a proper noun already points at exactly one thing, so the article that
  exists to say WHICH one has nothing to do. The same logic returns the article
  to a title -- until you address the person, at which point you are calling
  them rather than identifying them.
- 261 review closes on why naming a rule is worth a chapter: a rule you can
  state is one you can CHECK, and naming one often explains another.

### Added - every half-taught set finished (HL-C128)

- Add Spanish chapters 251-256. **A1 coverage 71/85 (84%) -> 77/85 (91%)**.
- Each of these was a set the book already had PART of, which is a harder gap
  to notice than a missing one -- nothing feels absent, because the reader uses
  the half they have and routes around the rest. `aqui` without `ahi`/`alli`.
  `manana` without `ahora`/`hoy`. `un` without `unos`. `nuestro` without
  `vuestro`. And a preterite missing only `ver` and `dar`.
- 251 lines the three place words up against the three pointing words, one for
  one: they answer the same question, which is whose side a thing is on.
- 252 `ahora` is `hac hora` -- *this hour* -- with the word for hour still
  visible in the middle of it, and `hoy` is `hodie`, *this day*, worn to three
  letters. Both are Latin phrases that stopped having parts.
- 253 `unos`: a plural of *one* sounds impossible until you hear what it does --
  it stops being a number and becomes a vague few, which the bare plural (the
  KIND of thing) does not mean.
- 254 `vuestro` closes the possessive set at six, of which only two vary for
  gender.
- 255 `vi` and `di` take no written accent, and the reason is not an exception:
  an accent marks a beat that could have fallen elsewhere, and in a
  one-syllable word it could not. `dar` is an `-ar` verb that takes the `-er`
  endings in the past.
- 256 review. It exists because the gate asked for it: without a review this
  batch pushed `atomsNeverRevisited` up by **15**, since nothing revisited the
  new atoms. With it, +1. That is a pedagogical gap the counter found, not a
  number to re-pin.
- `A1-Q-04` closed with no new content: `bastante` was taught at ch227 and its
  probe had never been wired. A point marked uncovered while genuinely taught
  is as wrong as the reverse, and it surfaced only because the report names
  every uncovered point rather than counting them.

### Added - stressed pronouns, exclamations, and the vocative (HL-C128)
- Add Spanish chapters 246-250. **A1 coverage 68/85 (80%) -> 71/85 (84%)**.
- 246 `para mi`: only the first two persons change shape after a preposition,
  and the accent on `mi` is the diacritic kind -- `mi` is *my*, `mi` is *me*.
  `ti` takes none, which looks inconsistent and is not: there is no other `ti`
  to separate it from. The mark is only ever spent where something needs
  telling apart.
- 247 `conmigo`: Latin stuck `cum` on the END of the pronoun (`mecum`), Spanish
  inherited it as a meaningless `-migo` tail, and then put `con` back on the
  front where prepositions belong. The word therefore says *with* **twice**,
  once at each end, fifteen centuries apart -- the same reshaping instinct that
  wore `mucho` down to `muy`, running in reverse for once.
- 248 `!Que grande!`: the accent on `que` has never been about questions. It
  marks the word carrying the force, which is why it survives into an
  exclamation and into a reported question with no marks at all.
- 249 the vocative: Latin had a whole CASE for addressing somebody, and Spanish
  replaced the lot with a comma. The lesson pairs it with the personal `a` --
  `Veo a Maria` puts her on the receiving end, `Maria, ven` has her doing the
  coming, and the comma is what tells them apart.
- 250 review notes that three of the four rules exist for one reason: Latin's
  endings fell away and something audible had to take over -- a stressed
  pronoun, a doubled preposition, and a comma.

### Added - source-verified Cyrillic п (HL-C09EP)

- Verify lowercase п as one continuous left-stem-to-top-shoulder-to-right-stem run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced rounded Latin-n-like school hand while fitting the bundled squared arch, straight uprights, and horizontal top bar and documenting its entry and exit joins.
- Reduce measured HL-C09 debt to 86 entries and queue Cyrillic р next.

### Added - source-verified Cyrillic о (HL-C09EO)

- Verify lowercase о as one continuous upper-right-to-left-side-to-bottom-to-right-side counterclockwise closure with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced tall, slightly slanted school-hand oval while fitting the bundled wider upright printed outline.
- Reduce measured HL-C09 debt to 87 entries and queue Cyrillic п next.

### Added - source-verified Cyrillic н (HL-C09EN)

- Verify lowercase н as one continuous left-stem-to-middle-bridge-to-right-stem run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced rounded school-hand bridge while fitting the bundled straight vertical stems and horizontal middle bar and documenting the printed form's omitted entry and exit joins.
- Reduce measured HL-C09 debt to 88 entries and queue Cyrillic о next.

### Added - source-verified Cyrillic м (HL-C09EM)

- Verify lowercase м as one continuous entry-to-first-apex-to-valley-to-second-apex-to-baseline run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced rounded two-arch order while fitting the bundled straight upright stems and deep central V and documenting the printed form's omitted entry and exit joins.
- Reduce measured HL-C09 debt to 89 entries and queue Cyrillic н next.

### Added - source-verified Cyrillic л (HL-C09EL)

- Verify lowercase л as one continuous hooked-left-leg-to-apex-to-right-leg run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced pointed school-hand order while fitting the bundled curved left leg, horizontal top shoulder, and straight right stem and documenting the printed form's omitted entry and exit joins.
- Reduce measured HL-C09 debt to 90 entries and queue Cyrillic м next.

### Added - source-verified Cyrillic к (HL-C09EK)

- Verify lowercase к as one continuous left-stem-to-upper-arm-to-lower-arm run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced looped order while fitting the bundled printed vertical and two angular diagonals and documenting its omitted entry and exit joins.
- Reduce measured HL-C09 debt to 91 entries and queue Cyrillic л next.

### Added - source-verified Cyrillic й (HL-C09EJ)

- Verify lowercase й as the joined и body followed by one lifted left-to-right breve against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced body-before-breve order while fitting the bundled printed backwards-N body and separate curved mark.
- Reduce measured HL-C09 debt to 92 entries and queue Cyrillic к next.

### Added - source-verified Cyrillic и (HL-C09EI)

- Verify lowercase и as one continuous left-stem-to-rising-diagonal-to-right-stem run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced joined order while fitting the bundled printed backwards-N glyph and documenting its omitted rounded entry and exit joins.
- Reduce measured HL-C09 debt to 93 entries and queue Cyrillic й next.

### Added - source-verified Cyrillic з (HL-C09EH)

- Verify lowercase з as one continuous smaller-upper-lobe-to-larger-lower-lobe run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced joined lobe order while fitting the bundled compact printed glyph and documenting its omitted rising exit join.
- Reduce measured HL-C09 debt to 94 entries and queue Cyrillic и next.

### Added - source-verified Cyrillic ж (HL-C09EG)

- Verify lowercase ж as one continuous lower-left-to-centre-to-right run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced rounded wing order while fitting the bundled symmetric printed glyph through its straight central upright and four diagonal arms.
- Reduce measured HL-C09 debt to 95 entries and queue Cyrillic з next.

### Added - source-verified Cyrillic ё (HL-C09EF)

- Verify lowercase ё as the continuous looped е body followed by separately lifted left and right dots against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced body-before-left-dot-before-right-dot order while fitting the bundled compact printed glyph through its upper bowl, middle bar, lower bowl, and two circular dots.
- Reduce measured HL-C09 debt to 96 entries and queue Cyrillic ж next.

### Added - source-verified Cyrillic е (HL-C09EE)
- Verify lowercase е as one continuous upper-loop-to-middle-to-lower-bowl run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced loop order while fitting the bundled compact printed glyph through its upper bowl, long middle bar, and rounded lower bowl.
- Reduce measured HL-C09 debt to 97 entries and queue Cyrillic ё next.


### Added - both past tenses finish (HL-C127, HL-C128)

- Add Spanish chapters 241-245, paying a debt the book made in print. The
  preterite review told the reader the `vosotros` forms were still owed;
  they are now taught, and so is the imperfect plural, which had had only its
  singular for well over a hundred chapters.
- 241 `hablasteis` is `hablaste` with the plural `-is` stacked on it -- two
  pieces the reader already owned, in a row. 242 puts the same `-isteis` on the
  other two families AND on every strong stem, which closes the preterite
  outright: six forms, every family, regular and strong, nothing outstanding.
- 243 gives the `-aba` imperfect its plural, and explains the one written
  accent in the set: `hablabamos` is stressed a syllable earlier than the
  default would put it, so the mark is the difference between the word being a
  past tense and being nothing at all.
- 244 gives the `-ia` set its plural, where the accent is on EVERY form and for
  a different reason -- the `i` and `a` would otherwise collapse into one beat.
  The two sets are mirror images. It also finishes the three irregular
  imperfects, which is the whole irregular list for the tense.
- 245 reviews both tenses as complete paradigms and keeps the two accent rules
  apart, since they look alike and are not.
- **A1 coverage 66/85 (78%) -> 68/85 (80%).**
- Two gates caught real problems on the way through and both were fixed in the
  corpus rather than re-pinned: seven lessons named literal chapter numbers
  (which shift on every insert, and which a Spanish-track invariant forbids),
  and two paradigm grids pushed the info-dump ceiling -- rewritten as chants,
  which is what every other review in this arc already does.

### Added - the gerund, the progressive, and the personal a (HL-C128)
- Add Spanish chapters 236-240. **A1 coverage 64/85 (75%) -> 66/85 (78%)**.
- 236 and 237 build the gerund: `-ando` for `-ar` verbs, `-iendo` for
  everything else. Two endings for three families, no agreement and no persons
  -- the smallest new machinery this book has asked for. And the `-er`/`-ir`
  merge the reader watched happen in the preterite happens again here, more
  completely: there is not even a difference to notice.
- 238 is the payoff: `estar` + gerund. It had to be `estar` rather than `ser`
  for a reason the reader already holds -- `estar` is the verb for how
  something temporarily stands, and an action caught mid-stride is as temporary
  as things get. The lesson also draws the line English blurs: `estoy comiendo`
  means NOW and cannot reach tomorrow, which still wants `voy a comer`.
- 239 the personal `a`. Latin marked the receiving end with an ending; Spanish
  discarded those endings and grew a preposition back in their place -- but
  only where the ambiguity bites, which is people. A house is unlikely to be
  seeing anybody.
- 240 review pairs the two rules by their shared cause: the gerund is what is
  LEFT of a Latin form after its cases fell off, and the personal `a` is what
  grew back BECAUSE they fell off. One leftover, one repair.
- **A third point was deliberately left uncovered.** `A1-V-03` asks for the
  present indicative's own actual and durative values; chapter 238 teaches the
  progressive and contrasts it with the plain present, which is adjacent but
  not the same claim. Closing it with progressive atoms would be exactly the
  gaming this gate exists to catch, so the probe stays `null` with the reason
  written into the inventory.

### Added - source-verified Cyrillic д (HL-C09ED)

- Verify lowercase д as one continuous counterclockwise body and below-baseline descender loop with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced body-before-descender order while fitting the bundled block glyph through its trapezoidal body, joined base shelf, and two retraced feet.
- Reduce measured HL-C09 debt to 98 entries and queue Cyrillic е next.

### Added - source-verified Cyrillic г (HL-C09EC)
- Verify lowercase г as one continuous two-hump cursive run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the sourced lift count while fitting the bundled block glyph through its straight upright and top bar, explicitly documenting the omitted cursive exit arch.
- Reduce measured HL-C09 debt to 99 entries and queue Cyrillic д next.


### Added - the joining words, and Coordinacion closes (HL-C128)
- Add Spanish chapters 230-235: `al`, `del`, `quien`, `o`, `ni`, review.
  **A1 coverage 60/85 (71%) -> 64/85 (75%)**, and `Coordinacion` finishes
  outright at 5/5.
- 230 `al` and 231 `del`: the two obligatory contractions, taught as one
  instinct the reader has already met -- when a small word leans on a bigger
  one, something gives, exactly as `mucho` gave way to `muy`. 231 closes with
  the rare pleasure of a finished list: Spanish has **two** contractions and no
  others, so `de la` and `a los` stay two words forever. `El Salvador` keeps its
  distance, because the rule is about the article and not the spelling.
- 232 `quien` is Latin `quem` -- the OBJECT form, which survived after Spanish
  discarded the case endings and now does every job. Reserved for people, where
  English blurs the line.
- 233 `o` is a whole word one letter long, from Latin `aut`, and it swaps to `u`
  before a word starting with the o sound -- a rule about sound, so a silent `h`
  does not stop it.
- 234 `ni` carries its `no` inside it (Latin `nec`), which is why a correct
  Spanish sentence can hold two negatives: they AGREE rather than cancel.
- 235 review names the instinct the first three share and separates the fourth,
  which is about agreement rather than sound.
### Added - source-verified Cyrillic в (HL-C09EB)
- Verify lowercase в as one continuous baseline-to-upper-loop-to-lower-bowl run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the source's tall looped school hand while routing the Noto Sans Cyrillic fit through the printed glyph's upper bowl and straight left stem.
- Reduce measured HL-C09 debt to 100 entries and queue Cyrillic г next.


### Added - the degree words, and four inventory points for three lessons (HL-C128)
- Add Spanish chapters 226-229: `muy`, `bastante`, `mal`, and a review.
  **A1 coverage 56/85 (66%) -> 60/85 (71%)**, and `El sintagma adjetival`
  leaves the floor at last (0/1 -> 1/1).
- Chosen for points-per-lesson: `muy` and `bastante` each sit on more than one
  inventory line, so three lessons close four points across three categories.
- 226 `muy` is not a new word: it is `mucho` with its ending bitten off, both
  from Latin `multum`. The long form goes in front of a noun, the short one in
  front of an adjective or adverb -- so the SHAPE of the word tells you what
  kind of word is coming. And `muy` can never stand alone as an answer.
- 227 `bastante` is `bastar` ("to be enough") wearing the `-ante` ending the
  reader learned to decode at chapter 52, so it means literally "sufficing".
  It is the MIDDLE of the scale -- poco, bastante, muy -- and English speakers
  reach for it thinking "quite a lot" and land higher than they meant.
- 228 `mal` gives `bien` the opposite it has waited for since chapter one, and
  is the second long/short pair in three chapters. Two instances make it a
  habit rather than two exceptions: Spanish bites the ending off a word that is
  leaning on the next one.
- 229 review, as three chants and the habit underneath them.
- The coverage report reordered itself: `El sintagma adjetival` was the
  worst-off category and is now closed, so the "worst first" line moved on to
  `Los cuantificadores` at 1/4 without anyone editing the sort.
### Added - source-verified Cyrillic б (HL-C09EA)
- Verify lowercase б as one continuous counterclockwise-body-to-top-flag run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the source's handwritten diagonal transition while routing the Noto Sans Cyrillic fit through the printed glyph's upper-left shoulder.
- Reduce measured HL-C09 debt to 101 entries and queue Cyrillic в next.


### Added - the demonstratives, and the first movement of the exam gate (HL-C128)
- Add Spanish chapters 221-225. `Los demostrativos` was the only A1 category
  reading **0 of 3**, in a book that had already taught the imperfect
  subjunctive: `este`, `ese`, `aquel` and the neuters were absent entirely.
- 221 `este`: it agrees, it goes in FRONT of the noun, and it takes the
  article's place -- never `el este libro`, because both words are answering
  "which one" and Spanish lets only one word do that at a time. Latin `iste`
  meant "that of yours"; Spanish walked it one step closer, off the listener
  and onto the speaker's own hand.
- 222 `ese`: the slot `iste` vacated, filled by Latin `ipse`, "self" -- the
  `ipse` of `ipso facto`. An emphasiser that insists on a particular thing for
  long enough stops emphasising and simply points. And the choice between
  `este` and `ese` is not distance but WHOSE SIDE the thing is on: two people
  at one small table use both, about books a hand's width apart.
- 223 `aquel`: Latin `ille` went two ways at once. Unstressed and leaning on
  the noun it wore down to `el`, the article taught in chapter one; stressed
  and propped up with `accu-` it stayed a pointing word. The same Latin word,
  separated by two thousand years of one being emphasised and the other not.
- 224 the `-o` forms, for pointing at what has no noun yet. Spanish kept one
  leftover of Latin's third gender for exactly this job: `esto` agrees with
  nothing, on purpose, because nothing has been named. Name it and agreement
  returns.
- 225 review, as three chants rather than a grid of twelve forms.
- **A1 coverage: 53/85 (62%) -> 56/85 (66%)**, `Los demostrativos` 0/3 -> 3/3.
  The gate worked as designed on the way through: the number would not move
  until the three probes were wired to real atoms, and the pin had to be
  changed deliberately rather than drifting.
### Added - source-verified Cyrillic а (HL-C09DZ)
- Verify lowercase а as one continuous body-to-stem run with zero lifts against RussianIrina's native-teacher all-letter handwriting lesson.
- Preserve the source's single-storey school-hand motion while fitting the upper entry through Noto Sans Cyrillic's extra double-storey printed shoulder.
- Break the tied Cyrillic/Gujarati queue, establish a source that covers all 33 Russian letters, and reduce measured HL-C09 debt to 102 entries.


### Added - source-verified Devanagari ह (HL-C09DY)

- Verify ह as three ordered strokes with two lifts against Opiaterein's 22-frame animation; the Central Hindi Directorate independently corroborates the same component order.
- Join the descending right stem, leftward shoulder, and clockwise hooked body before the restarted down-left outer curve and down-right tail, then the left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the joined first body and lift evidence animation-backed, complete the source-verified Devanagari starter set, and reduce measured HL-C09 debt to 103 entries.

### Added - source-verified Devanagari स (HL-C09DX)

- Verify स as four ordered strokes with three lifts against JackPotte's 13-frame animation; the Central Hindi Directorate independently corroborates the same component order.
- Keep the descending left stem, central hook, and down-right diagonal tail joined before the restarted middle crossbar, top-to-bottom right stem, and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the hook-to-tail join and lift evidence animation-backed, reduce measured HL-C09 debt to 104 entries, and queue Devanagari ह next.

### Added - source-verified Devanagari श (HL-C09DW)

- Verify श as three ordered strokes with two lifts against Opiaterein's 25-frame animation; JackPotte's animation and the Central Hindi Directorate independently corroborate the same three-part learner buildup.
- Keep the upper loop, descending outer curve, lower loop, and down-right diagonal tail joined before the restarted top-to-bottom right stem and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Preserve the animation-backed directions and lift evidence, reduce measured HL-C09 debt to 105 entries, and queue Devanagari स next.

### Added - source-verified Devanagari व (HL-C09DV)

- Verify व as three ordered strokes with two lifts against JackPotte's 11-frame animation; the Central Hindi Directorate independently corroborates the same three-part learner buildup.
- Circle counterclockwise around the left loop before the restarted top-to-bottom right stem and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep within-run direction and lift evidence animation-backed, reduce measured HL-C09 debt to 106 entries, and queue Devanagari श next.

### Added - source-verified Devanagari ल (HL-C09DU)

- Verify ल as four ordered strokes with three lifts against a 23-frame animated source and the Central Hindi Directorate's matching four-part learner buildup.
- Curve up and clockwise around the open left loop before the restarted up-right diagonal arm, top-to-bottom right stem, and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Preserve JackPotte's alternate 12-frame stem-first order, reduce measured HL-C09 debt to 107 entries, and queue Devanagari व next.

### Added - source-verified Devanagari र (HL-C09DT)

- Verify र as three ordered strokes with two lifts against a 17-frame animated source and the Central Hindi Directorate's matching three-part learner buildup.
- Descend the stem and curl clockwise around the lower loop before the restarted down-right diagonal tail and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Preserve JackPotte's alternate seven-frame joined-body form, reduce measured HL-C09 debt to 108 entries, and queue Devanagari ल next.

### Added - source-verified Devanagari य (HL-C09DS)

- Verify य as four ordered strokes with three lifts against a 22-frame animated source and the Central Hindi Directorate's matching four-part learner buildup.
- Draw the clockwise inner curl before the restarted lower bowl, top-to-bottom right stem, and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Preserve JackPotte's alternate 11-frame joined-body form, reduce measured HL-C09 debt to 109 entries, and queue Devanagari र next.

### Added - source-verified Devanagari म (HL-C09DR)

- Verify म as three ordered strokes with two lifts against a 12-frame animated source; the Central Hindi Directorate independently corroborates component order while staging the joined left-stem-to-loop body as two buildup steps.
- Descend the left stem, curl clockwise around the lower loop, and sweep right through the crossbar before the top-to-bottom right stem and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Reduce measured HL-C09 debt to 110 entries and queue Devanagari य next.

### Added - source-verified Devanagari भ (HL-C09DQ)

- Verify भ as three ordered strokes with two lifts against a 15-frame animated source; the Central Hindi Directorate independently corroborates component order while staging the joined body as two buildup steps.
- Sweep clockwise through the upper loop, descending trunk, lower bowl, and rightward crossbar before the top-to-bottom right stem and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Reduce measured HL-C09 debt to 111 entries and queue Devanagari म next.

### Added - source-verified Devanagari ब (HL-C09DP)

- Verify ब as four ordered strokes with three lifts against a 13-frame animated source, independently corroborated by the Central Hindi Directorate's four-part learner buildup.
- Circle counterclockwise around the oval before the top-to-bottom right stem, down-right inner diagonal, and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Reduce measured HL-C09 debt to 112 entries and queue Devanagari भ next.

### Added - source-verified Devanagari प (HL-C09DO)

- Verify प as three ordered strokes with two lifts against a 19-frame animated source, independently corroborated by the Central Hindi Directorate's three-part learner buildup.
- Descend the left stem and curve right around the lower bowl before the top-to-bottom right stem and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Reduce measured HL-C09 debt to 113 entries and queue Devanagari ब next.

### Added - source-verified Devanagari न (HL-C09DN)

- Verify न as three ordered strokes with two lifts against a 20-frame animated source, independently corroborated by the Central Hindi Directorate's three-part learner buildup.
- Circle clockwise around the left loop and continue right along its shoulder before the top-to-bottom right stem and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Reduce measured HL-C09 debt to 114 entries and queue Devanagari प next.

### Added - source-verified Devanagari ध (HL-C09DM)

- Verify ध as four ordered strokes with three lifts against a 27-frame animated source, independently corroborated by the Central Hindi Directorate's four-part learner buildup.
- Draw the upper spiral and shoulder before the separate lower bowl, top-to-bottom right stem, and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Reduce measured HL-C09 debt to 115 entries and queue Devanagari न next.

### Added - source-verified Devanagari द (HL-C09DL)

- Verify द as three ordered strokes with two lifts against an 18-frame animated source.
- Descend the short stem before one continuous outer-body, inward-curl, and down-right-tail run, then finish with the left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Record that the Central Hindi Directorate deskbook corroborates component order but stages the body and curl-tail separately, reduce HL-C09 debt to 116 entries, and queue Devanagari ध next.

### Added - source-verified Devanagari त (HL-C09DK)

- Verify त as three ordered strokes with two lifts against a 17-frame animated source, independently corroborated by the Central Hindi Directorate's three-part learner buildup.
- Sweep the upper shoulder right-to-left and curve down to the open lower tip before the top-to-bottom right stem and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Reduce measured HL-C09 debt to 117 entries and queue Devanagari द next.

### Added - source-verified Devanagari च (HL-C09DJ)

- Verify च as three ordered strokes with two lifts against a 22-frame animated source.
- Join the short left-to-right upper bar directly to the rounded open body before the top-to-bottom right stem and final shirorekhā in a Noto Sans Devanagari fit.
- Record that the Central Hindi Directorate deskbook corroborates component order but not the animation's first join, reduce HL-C09 debt to 118 entries, and queue Devanagari त next.

### Added - source-verified Devanagari ग (HL-C09DI)

- Verify ग as three ordered strokes with two lifts against an 18-frame animated source, independently corroborated by the Central Hindi Directorate's three-part learner buildup.
- Carry the counterclockwise left loop directly up its joined stem before the top-to-bottom right stem and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Reduce measured HL-C09 debt to 119 entries and queue Devanagari च next.

### Added - source-verified Devanagari क (HL-C09DH)

- Verify क as four ordered strokes with three lifts against a 27-frame animated source, independently corroborated by the Central Hindi Directorate's four-part learner buildup.
- Draw the counterclockwise left bowl before the top-to-bottom central stem, clockwise right-hand arch, and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Record the newly audited GIF collection as coverage for every remaining Devanagari starter consonant and reduce HL-C09 debt to 120 entries.

### Added - source-verified Devanagari औ (HL-C09DG)

- Verify औ as seven ordered strokes with six lifts against a seven-panel modern printed source.
- Reuse आ's joined left body, separate shoulder, and two stems before the two separate upper arcs and final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the sourced teaching form distinct from universal handwriting practice and reduce HL-C09 debt to 121 entries.

### Added - source-verified Devanagari ओ (HL-C09DF)

- Verify ओ as six ordered strokes with five lifts against a six-panel modern printed source.
- Reuse आ's joined left body, separate shoulder, and two stems before the separate upper arc and final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the sourced teaching form distinct from universal handwriting practice and reduce HL-C09 debt to 122 entries.

### Added - source-verified Devanagari ऐ (HL-C09DE)

- Verify ऐ as four ordered strokes with three lifts against a four-panel modern printed source.
- Reuse ए's long stem and tail plus its shorter hooked stem before the separate upper arc and final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the sourced teaching form distinct from universal handwriting practice and reduce HL-C09 debt to 123 entries.

### Added - source-verified Devanagari ए (HL-C09DD)

- Verify ए as three ordered strokes with two lifts against a three-panel modern printed source.
- Join the long left stem to its curved shoulder and descending tail before the separate inward-hooked stem and final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the sourced teaching form distinct from universal handwriting practice and reduce HL-C09 debt to 124 entries.

### Added - source-verified Devanagari ऊ (HL-C09DC)

- Verify ऊ as three ordered strokes with two lifts against a three-panel modern printed source.
- Reuse उ's continuous body before the separate right-hand loop and final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the sourced teaching form distinct from universal handwriting practice and reduce HL-C09 debt to 125 entries.

### Added - source-verified Devanagari उ (HL-C09DB)

- Verify उ as two ordered strokes with one lift against a two-panel modern printed source.
- Preserve its upper bowl and lower loop as one continuous body before the final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the sourced teaching form distinct from universal handwriting practice and reduce HL-C09 debt to 126 entries.

### Added - source-verified Devanagari ई (HL-C09DA)

- Verify ई as three ordered strokes with two lifts against a three-panel modern printed source.
- Reuse इ's continuous double-bowl body before the separate upper curl and final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the sourced teaching form distinct from universal handwriting practice and reduce HL-C09 debt to 127 entries.

### Added - source-verified Devanagari इ (HL-C09CZ)

- Verify इ as two ordered strokes with one lift against a two-panel modern printed source.
- Preserve its continuous upright, upper bowl, lower bowl, and down-right tail before the final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Keep the sourced teaching form distinct from universal handwriting practice and reduce HL-C09 debt to 128 entries.

### Added - source-verified Devanagari आ (HL-C09CY)

- Verify आ as five ordered strokes with four lifts against a five-frame modern printed source.
- Preserve its joined left body, lifted shoulder, two top-to-bottom stems, and final left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Carry the published traditional base-अ variation forward and reduce HL-C09 debt to 129 entries.

### Added - source-verified Devanagari अ (HL-C09CX)

- Verify अ as four ordered strokes with three lifts against a four-frame modern printed source.
- Preserve its joined left body, lifted shoulder, top-to-bottom stem, and left-to-right shirorekhā in a Noto Sans Devanagari fit.
- Record the published six-stroke traditional Sanskrit form as source variation and reduce HL-C09 debt to 130 entries.

### Added - source-verified Chinese 上 (HL-C09CW)

- Verify 上 as three ordered strokes with two lifts against the pinned PRC source.
- Preserve its vertical-first, short-before-long horizontal order in a Noto Sans SC fit.
- Reduce HL-C09 debt to 131 entries and complete the Chinese starter inventory.

### Added - coverage against what the exam tests (HL-C128)
- Add `core/exam-inventory-es-a1.json`, `src/exam-inventory.ts` and
  `tests/exam-inventory.test.ts`: the first measurement in this package that
  can FALL, and the first that does not rise merely because a lesson was added.
- Every other number here walks our own lessons, so all of them improve when
  the corpus grows -- including growth on something no examiner asks about. This
  one resolves the corpus against an external, finite list: the A1 grammar an
  examiner may expect, restated in our own words from the structure of the Plan
  Curricular del Instituto Cervantes.
- The mapping is an **executable probe**, not an annotation. A `coveredBy:`
  field filled in once is a claim about the corpus frozen at a moment in time,
  and it goes stale silently and flatteringly. `probe: ["ES-GRAMMAR-NOUN-GENDER"]`
  is recomputed every run: retire the atom and coverage falls.
- `probe: null` means UNCOVERED, never "skip". Excluding unmapped points from
  the denominator would let the percentage be improved by deleting a mapping --
  the one edit that changes nothing about what a reader knows.
- **Spanish A1: 53 of 85 points, 62%**, after 220 chapters that had climbed to a
  B2 node. Missing entirely: the demonstratives (3 of 3 points), `muy`, the
  `al`/`del` contractions, the gerund, `quien`, the personal `a`.
- The gate was verified adversarially rather than assumed: an empty probe --
  the one malformed shape that scores as covered -- throws at load, and deleting
  a point fails the pin.

### Added - source-verified Chinese 早 (HL-C09CV)

- Verify 早 as six ordered strokes with five lifts against the pinned PRC source.
- Preserve its complete 日-before-十 order and joined top-right turn in a Noto Sans SC fit.
- Reduce HL-C09 debt to 132 entries; Chinese has 1 outstanding.

### Added - source-verified Chinese 么 (HL-C09CU)

- Verify 么 as three ordered strokes with two lifts against the pinned PRC source.
- Preserve the second stroke's joined falling-to-rightward sweep in a Noto Sans SC fit.
- Reduce HL-C09 debt to 133 entries; Chinese has 2 outstanding.

### Added - source-verified Chinese 什 (HL-C09CT)
- Verify 什 as four separately ordered strokes with three lifts against the pinned PRC source.
- Preserve its complete 亻-before-十 component order in a Noto Sans SC fit.
- Reduce HL-C09 debt to 134 entries; Chinese has 3 outstanding.


### Added - the connectives an argument runs on (HL-C113)

- Add Spanish chapters 218-220, opening `SPINE-ARGUE-A-VIEW`. Measurement
  redirected this rung before a word of it was written: the node's concepts are
  CONNECTIVE-HOWEVER and CONNECTIVE-ALTHOUGH, and the corpus taught neither ---
  nor `aunque`, `sin embargo`, `mejor`, `por eso`, which appear **nowhere** in
  217 chapters. **`pero` itself had no lesson**, only seven incidental uses in
  other lessons' prose from chapter 59 on. So the node opens with the
  vocabulary it consumes, not with the argument.
- 218 `pero`: two Latin words, `per hoc` -- "by this", which became "for all
  that", which is already a contrast. The lesson's point is what `pero` does
  NOT do: it denies neither half. `porque` fuses two facts into one reason;
  `pero` keeps two apart and admits both.
- 219 `tambien` = `tan` + `bien` -- "so well". The second half is the word the
  reader has held since **chapter one**, still doing its old job inside a word
  that no longer looks like it has parts. Introduces `tan` on the way past.
- 220 `tampoco` = `tan` + `poco`. One frame, two fillings: the reader is not
  learning a second word so much as turning the first one over. The rule for
  choosing is about the OTHER person's sentence -- listen for the `no`.
- `bene` is re-spent rather than re-minted, so `tambien` pays a root the corpus
  already owns; `tam-latin` is minted by 219 and spent by 220.
### Added - source-verified Chinese 见 (HL-C09CS)

- Verify 见 as four ordered strokes with three lifts against the pinned PRC source.
- Preserve its frame-before-legs order and all three joined turns in a Noto Sans SC fit.
- Reduce HL-C09 debt to 135 entries; Chinese has 4 outstanding.

### Added - reported questions close the B2 node (HL-C113)

- Add Spanish chapters 214-217, closing `SPINE-REPORT-WHAT-OTHERS-SAID` at
  seven segments. The node's can-do says "said **or asked**", and only the
  saying half had been built.
- 214 `pregunto si`: a yes-or-no question goes in on `si`, not `que` -- and the
  question marks are not merely dropped but disallowed, because a report tells
  and does not ask. That `si` is the same word that opens a condition; both
  uses hold something open that has not been settled. English splits the job
  between *whether* and *if*; Spanish never needed to.
- 215 `pregunto donde`: the asking word is its own joint, and the accent stays
  in a sentence that has no question marks at all. That is the answer to a
  question the reader has carried since chapter 6 -- the accent was never a
  partner to the marks. The marks say the SENTENCE is a question; the accent
  says the WORD is doing the asking, and here exactly one of those is true.
- 216 review, again as chants rather than a grid: three joints, two constants.
- 217 synthesis: a four-line conversation carried across to somebody who was
  not there, one joint per line and a different joint every time.
- 214 and 215 classify as `sight`, not `voice`. That is correct rather than a
  gap: a lesson about a written accent cannot be delivered by narration.
- `atomsNeverRevisited` falls by 3 and Spanish's `neverSpent` roots by 1: the
  review and synthesis spend what step 6 minted, including ES-ETYMON-DIJO-DIXIT.

### Added - B2 opens with reported speech (HL-C113)

- Add Spanish chapters 211-213, the first lessons any track has placed on a B2
  node. `SPINE-REPORT-WHAT-OTHERS-SAID` was chosen because the corpus already
  held everything it needs but one form -- measured before authoring: `decir`
  (ch70), the imperfect singular (ch107, 109), the strong preterite (ch105,
  199-203), and relative `que` (ch186-187).
- 211 `dice que`: reporting is one sentence put inside another, and `que` is the
  joint. English may drop *that*; Spanish may not -- the second time the reader
  has seen `que` refuse to vanish, after `el libro que compre`.
- 212 `dijo`: the stem is `dij-`, which obeys the strong singular pattern and
  then breaks the strong plural. Not `dijieron` but `dijeron`: the glide in
  `-ieron` will not stand next to a sound already made in that part of the
  mouth. Stated as a rule for every `j` stem, not an exception for one verb.
  The `j` is a Latin `x` in modern clothes -- dixit, dixo, dijo.
- 213 `dijo que`: the backshift, and the reason for it. The imperfect is chosen
  not by a rule about reporting but because a report describes what WAS GOING ON
  when someone spoke, which is the job the imperfect has always had.
- The level gate now reports Spanish as touching **B2** while `attained` stays
  null. That gap is what the module exists to show, and it widens with the climb.

### Added - the unreal condition (HL-C113)

- Add Spanish chapters 208-210, the rung the whole chain from `si` through the
  preterite plural was built to reach: `si` + imperfect subjunctive, answered by
  `-ria`. *Si tuviera tiempo, hablaria espanol* -- a world that is not so.
- Name the two prohibitions the reader can now hear: `si` never takes the future
  and never takes the present subjunctive. Spanish marks the supposing on the
  verb of the supposition itself, which is why the slot is already occupied.
- Write the review as three chants rather than a 5x3 grid. The grid version of
  the preceding review was an info-dump under HL10 5.3 and could not be narrated;
  the chants carry the same three worlds and leave the chapter `drivable`.
- Close `SPINE-EXPRESS-CONDITION`: eight path segments, B1 at 31 lessons.

### Added - the imperfect subjunctive (HL-C113)

- Add Spanish chapters 206-207: swap `-ron` for `-ra` on the they-past, a rule
  with no exceptions, and the four strong stems collected for free.
- Explain why it has no irregulars: it is carved from the preterite, which had
  already absorbed every irregularity these verbs have.
- `hablara` was Latin `fabulaveram`, "I had spoken" — an indicative pluperfect
  that drifted into supposing, while `hablase` came from a real subjunctive.
- B1 lessons rise 26 to 28.

### Added - the preterite is finished (HL-C113)

- Add Spanish chapters 202-205: `tuvieron` (strong stems, ordinary ending),
  `fueron` (the plural that refuses both endings and serves two verbs), a review
  of the full paradigm, and a synthesis.
- The tense now covers five of the six persons; `vosotros` and the strong
  `nosotros` forms are named as still owed rather than glossed over.
- The synthesis points at the imperfect subjunctive without teaching it: swap
  `-ron` for `-ra` and the raw material is already there.
- The review is written as three per-family chants rather than a paradigm grid,
  which keeps it out of the info-dump count and makes it voice-drivable.

### Added - the preterite plural (HL-C113)

- Add Spanish chapters 199-201: `hablaron`, `hablamos` (present and past alike),
  and `comieron`/`vivieron` sharing one ending.
- Close a gap the corpus had carried since chapter 103: the preterite was taught
  in the singular only, so a reader could say "I spoke" but not "they spoke".
- This is also the hard prerequisite for the imperfect subjunctive, which derives
  from the third-person plural preterite.

### Added - the cross-track cousin join (HL-C88, HL10 §6.7)

- Add `src/cousins.ts`: `buildCousinIndex` and `cousinsFor` find lessons in other
  Romance tracks that teach a reflex of the same etymon, keyed on `roots:`.
- Exclude the lesson's own language and Latin, take one word per language
  (earliest by reading order), and emit a fixed language order.
- Reach: 76 Spanish lessons; 25 under a single-token headword restriction. Both
  numbers are pinned, because the display rule is still an open decision.

### Added - false friends as a formal block (HL-C88, HL10 §6.7)

- Add Spanish chapter 66, immediately after the eight-ending synthesis: a false
  friend is a word that OBEYS the rules and still means something else.
- Give each one its history rather than a warning: `exitus` was a going out, and
  English kept the door while Spanish kept the outcome.
- Add the operational habit: after the ending answers, ask whether the meaning
  fits the sentence.

### Added - the eighth ending, and the review and synthesis that close the set (HL-C88)

- Add Spanish chapters 63-65: `-ario` (English `-ary`), a review of all eight
  endings, and a synthesis reading a paragraph the book never taught.
- Sort the review by what each ending makes rather than by when it was met, and
  name which endings decide gender and how far each one reaches.
- Only the first three endings had ever had a review and synthesis; the five
  added since had none.

### Added - the -ncia friend ending (HL-C88, HL10 §6.7)

- Add Spanish chapter 62: `-ncia` is English `-nce`, from Latin `-entia`.
- Compose rather than accumulate: every `-ncia` word is a feminine noun, so it
  lands on the article and gender rules, and `una diferencia grande` combines it
  with the adjective arc placed just before.
- Teach the near miss: `ciencia` is science, not "cience" — trust the ending and
  check the root.

### Fixed - the adjective arc had no nouns to describe (HL-C112)

- Move `la casa` and `el libro` from chapters 70-71 to 56-57, ahead of the
  adjective arc, so `una casa grande` is taught rather than assumed.
- Forward references fall 446 -> 438; the adjective arc reaches zero.
- Declare the `hacer`, `querer` and `decir` dependencies that 17 downstream
  lessons had been reaching transitively through the moved segments.

### Added - the -oso friend ending (HL-C88, HL10 §6.7)

- Add Spanish chapter 59: `-oso` is English `-ous`, from Latin `-osus`, "full of".
- Build it on the adjective arc: position, agreement, and the `ser`/`estar` choice
  all apply without being restated.
- Name the limit: `hermoso` is beautiful, not "hermous" — the ending is reliable,
  the root still has to be shared.
- Correct the earlier verdict that `-oso` was blocked on vocabulary: a reading
  rule does not need the corpus to teach words ending in it.

### Added - the first describing words (HL-C88, HL10 §5.4b)

- Add Spanish chapters 56-58: `grande`, `cansado`, and a synthesis on choosing
  `ser` or `estar` with an adjective and making the ending agree.
- Close a 129-chapter gap: the corpus's only adjectives were the colours, first
  taught at chapter 130, so `ser` vs `estar` at chapter 48 had nothing to contrast.
- Teach adjective-after-noun order, and that the last letter says whether a word
  shifts: `-o` does, `-e` and `-ista`/`-ante` do not.
- Teach `cansado` as a word the decoder cannot help with, on purpose.

### Added - professions, and the -ante and -ista endings (HL-C88, HL10 §6.7)

- Add Spanish chapters 51-54: `profesor`, `estudiante`, the `-ista` family, and a
  synthesis. The corpus taught `ser` at chapter 48 but had no profession noun in
  322 word forms, so `Soy profesor` could not be said.
- Teach the bare-noun rule: after `ser`, a job takes no `un` or `una`.
- Build `estudiante` from `estudiar` with `-ante`, and read it as English `-ant`.
- Ground the previously blocked `-ista` ending on words that need no teaching.
- Both endings are invariant for gender; only the article moves.

### Added - the es- + consonant friend rule (HL-C88, HL10 §6.7)

- Add Spanish chapter 30: Latin `st-`/`sp-`/`sc-` grew an `e-` in Spanish, so
  taking it off often reveals the English word (`estación` to station).
- Anchor it on `estar` and `estudiar`, taught in chapters 4 and 6, and spend the
  existing `stare-latin` and `studere-latin` roots rather than minting new slugs.
- Name the limit: `español` is not an example, since its `Es-` is what remains of
  `Hispania` rather than an added vowel.
- Grow the chapter-31 synthesis from six rules to seven, and say that they stack.
- Chose this rung by census: it has 11 taught words to decode, while `-ncia`,
  `-oso`, `-ario` and `-ismo`/`-ista` have none anywhere in the book.

### Fixed - over-long section short titles wrapped in the table of contents (HL-C109d)

- Cut `sectionShortTitle` to a budget of 40 display columns, the corpus's 99th
  percentile, so a month or weekday list no longer wraps a one-line TOC entry.
- Count combining marks as zero columns and East Asian wide forms as two; cut at
  a word boundary, and keep a single over-wide word intact rather than mid-word.
- Drop a trailing separator with the item it joined, so a cut list does not read
  as `sal - se - ...` with something missing from the middle.
- 17 section lines in 17 chapters change. All 22 books now build with 0 overfull,
  0 underfull and 0 missing characters.

### Added - hidden friends via the sound laws (HL-C88, HL10 §6.7)

- Add Spanish chapters 28-30: the general `f- -> h-` decoder, `cl-`/`pl-`/`fl- -> ll-`,
  and a synthesis over all six decoding rules the book now holds.
- Introduce `ES-SOUND-F-TO-H-DECODER` and pay the promise `ES-C06-hablar` made in its
  own prose 22 chapters earlier, practising the `ES-SOUND-F-TO-H` atom it planted.
- Review `-CT- -> -ch-` rather than introduce it: `ES-C02-noche` already teaches it.
- Shift 241 lessons at sequence >=548 by +20 sequence and +3 chapter, and rebuild the
  capability ledger, book targets, curriculum path segments and extension nodes with them.
### Added - source-verified Chinese 再 (HL-C09CR)

- Verify 再 as six ordered strokes with five lifts against the pinned PRC source.
- Preserve its joined frame, close-last order, and both turns in a Noto Sans SC fit.
- Reduce HL-C09 debt to 136 entries; Chinese has 5 outstanding.

### Added - source-verified Chinese 请 (HL-C09CQ)

- Verify 请 as ten ordered strokes with nine lifts against the pinned PRC source.
- Preserve 讠-before-青 order and all four joined turns in a Noto Sans SC fit.
- Reduce HL-C09 debt to 137 entries; Chinese has 6 outstanding.
### Added - source-verified Chinese 谢 (HL-C09CP)

- Verify 谢 as twelve ordered strokes with eleven lifts against the pinned PRC source.
- Preserve 讠-before-身-before-寸 order and all five joined turns in a Noto Sans SC fit.
- Reduce HL-C09 debt to 138 entries; Chinese has 7 outstanding.

### Added - source-verified Chinese 字 (HL-C09CO)

- Verify 字 as six ordered strokes with five lifts against the pinned PRC source.
- Preserve 宀-before-子 component order and all three joined turns in a Noto Sans SC fit.
- Reduce HL-C09 debt to 139 entries; Chinese has 8 outstanding.

### Added - source-verified Chinese 名 (HL-C09CN)

- Verify 名 as six ordered strokes with five lifts against the pinned PRC source.
- Preserve 夕-before-口 component order and both joined turns in a Noto Sans SC fit.
- Reduce HL-C09 debt to 140 entries; Chinese has 9 outstanding.

### Added - source-verified Chinese 不 (HL-C09CM)

- Verify 不 as four separately placed strokes with three lifts against the pinned PRC source.
- Record that planned 叫 is outside the measured inventory and needs a separate font-resubsetting change.
- Reduce HL-C09 debt to 141 entries; Chinese has 10 outstanding.
### Added - source-verified Chinese 是 (HL-C09CL)

- Verify 是 as nine ordered strokes with eight lifts against the pinned PRC source.
- Preserve 日-first order and its joined top-right corner in a Noto Sans SC fit.
- Reduce HL-C09 debt to 142 entries; Chinese has 11 outstanding.

### Added - source-verified Chinese 我 (HL-C09CK)

- Verify 我 as seven ordered strokes with six lifts against the pinned PRC source.
- Add a Noto Sans SC fit preserving the hooked vertical and long curved slash.
- Reduce HL-C09 debt to 143 entries; Chinese has 12 outstanding.

### Added - the three highest-reach friend endings (HL-C88, HL10 §6.7)

- Add chapters 23-27, immediately after `español`: `-ción`/`-tion` (~2,000
  words), `-dad`/`-ty` (~1,200), `-mente`/`-ly` (unbounded), a review and a
  synthesis.
- Ground each correspondence rather than asserting it: `-ción` and `-tion` are
  the same Latin `-tiōnem`, inherited by one language and borrowed by the other;
  `-dad` looks less like `-ty` because English wore `-tātem` down further
  through French; `-mente` was a feminine noun meaning *mind*, which explains
  the feminine adjective.
- Separate decoders from machines in the review: two endings let the reader
  read, one lets them build.
- Close by having the reader read a sentence with four untaught words in it.
- Record that `type: pattern` is reserved for slot-filling productions with a
  single `-PATTERN-` atom; these rules are `type: grammar`.
- Spanish reaches 178 chapters and 324 lessons.

### Added - the pronoun as evidence of a hidden gender (HL-C107)

- Add chapter 122, immediately after `el agua`: a third proof that the word is
  feminine, after the adjective and the plural, and the cheapest of the three.
- `La bebo`, not `lo bebo` — nothing sits in front of a pronoun, so no vowels
  collide and it reports the gender straight.
- Close on the general test rather than the single word: when an article looks
  suspicious, listen for `lo` or `la`.
- Spanish reaches 173 chapters and 319 lessons.

### Added - source-verified Chinese 好 (HL-C09CJ)

- Verify 好 from its pinned Hanzi Writer Data record as six ordered runs: all
  three strokes of 女 before all three strokes of 子.
- Preserve three joined turns and five lifts in a Noto Sans SC fit.
- Reduce measured HL-C09 debt to 144 entries; Chinese has 13 outstanding.

### Added - source-verified Chinese 你 (HL-C09CI)

- Verify 你 from its pinned Hanzi Writer Data record as seven ordered runs:
  write 亻 first, then the five strokes of 尔.
- Preserve the joined horizontal hook, joined vertical hook, and six lifts in
  a Noto Sans SC fit.
- Reduce measured HL-C09 debt to 145 entries; Chinese has 14 outstanding.

### Added - source-verified Chinese 宀 (HL-C09CH)

- Verify 宀 from its pinned Hanzi Writer Data record as three ordered runs: a
  top dot, a left-side drop, then a horizontal roof with a joined down-left hook.
- Preserve the roof and hook inside the third stroke and two lifts in a Noto
  Sans SC fit.
- Reduce measured HL-C09 debt to 146 entries; Chinese has 15 outstanding.

### Added - source-verified Chinese 氵 (HL-C09CG)

- Verify 氵 from its pinned Hanzi Writer Data record as three ordered runs: two
  down-right dots, then a bottom stroke that turns slightly left before rising.
- Preserve the bottom turn and rise inside the third stroke and two lifts in a
  Noto Sans SC fit.
- Reduce measured HL-C09 debt to 147 entries; Chinese has 16 outstanding.

### Added - source-verified Chinese 讠 (HL-C09CF)

- Verify 讠 from its pinned Hanzi Writer Data record as two ordered runs: a
  down-right dot, then a horizontal that turns down and rises to finish.
- Preserve both turns inside the second stroke and one lift in a Noto Sans SC
  fit.
- Reduce measured HL-C09 debt to 148 entries; Chinese has 17 outstanding.

### Added - hay, closing rung 1 and the whole HL10 rung audit (HL-C105)

- Add five chapters at 168-172. Placed late on purpose: only after `haber` can
  `hay` be explained rather than listed.
- Record that `hay` is the one common Spanish verb form that **agrees with
  nothing** — visible as a gift only after 167 chapters of agreement.
- Take it apart: `ha` + **`y`**, from Latin *ibi* ("there"), a word Spanish
  otherwise lost completely and which survives only because it was welded to a
  verb. French kept the same three pieces separate as *il y a*.
- Pair `hay que` with the impersonal `se` as Spanish's two ways of leaving the
  person out — one for what happens, one for what must happen.
- Name the pattern: the smallest words need the most explaining, because
  constant use wears them into fossils.
- Close the beginner's arc honestly: the machinery is assembled; what remains
  is words and practice.
- **All eleven absent rungs from the HL10 §5.4a audit are now closed.**
- Spanish reaches 172 chapters and 318 lessons.

### Added - source-verified Chinese 日 (HL-C09CE)

- Verify 日 from its pinned Hanzi Writer Data record as four ordered runs: the
  left side, joined top-and-right corner, middle bar, then closing bottom.
- Preserve the joined corner, inside-before-close order, and three lifts in a
  Noto Sans SC fit.
- Reduce measured HL-C09 debt to 149 entries; Chinese has 18 outstanding.

### Added - relative clauses, closing rung 28 (HL-C105)

- Add five chapters at 163-167. This rung introduces **no new word at all**:
  `que` and `lo` were both already held, and `lo que` is the two together.
- Name what changes: a sentence may now contain another sentence.
- Give a chapter to the error that outlives the others — **English deletes its
  joint and Spanish never does**. The gap is invisible because the learner's own
  language put it there, so the habit is mechanical: a noun, then a new subject
  and verb, needs a `que`.
- Explain why: English word order is rigid, Spanish moves its pieces, so a
  deletable joint would be ambiguous. It is the price of pro-drop.
- Frame `lo que` as a noun-shaped hole with a sentence attached — which is what
  English *what* is.
- Close on the argument that length is not difficulty: a relative clause adds a
  slot, not grammar.
- Spanish reaches 167 chapters and 313 lessons.

### Added - source-verified Chinese 子 (HL-C09CD)

- Verify 子 from its pinned Hanzi Writer Data record as three ordered runs: a
  joined top turn, a separately joined central hook, then the middle héng.
- Preserve both internal turns and two lifts in a Noto Sans SC fit.
- Reduce measured HL-C09 debt to 150 entries; Chinese has 19 outstanding.

### Added - the impersonal se, closing rung 27 (HL-C105)

- Add five chapters at 158-162 for the `se` that names nobody: `se habla
  español`, the agreement that reveals its origin, `¿cómo se dice?`, a review
  and a synthesis.
- Show that the impersonal **is** the reflexive, grown: `se compran libros`
  takes a plural verb because, underneath, books buy themselves.
- Unify material learned a hundred chapters apart: Spanish has **two `se`s and
  a coincidence**, not three. The `se` of `se lo digo` is Latin *illī*, met as
  *gelo* in rung 15.
- Give `¿cómo se dice?` its own chapter as the sentence that lets a
  conversation repair itself.
- Close on reading rather than producing: this construction is written at the
  learner, and the missing person is what gives a sign its authority.
- Spanish reaches 162 chapters and 308 lessons.

### Added - source-verified Chinese 女 (HL-C09CC)

- Verify 女 from its pinned Hanzi Writer Data record as three ordered runs: a
  bent piědiǎn sweep, a separately left-falling piě, then the middle héng.
- Preserve the first stroke's internal turn and two lifts in a Noto Sans SC fit.
- Reduce measured HL-C09 debt to 151 entries; Chinese has 20 outstanding.

### Added - por and para, closing rung 26 (HL-C105)

- Add five chapters at 153-157 replacing the usual twelve-rule list with one
  arrow: `para` points forward at a target, `por` points back at a cause or
  through a middle.
- Ground the test in the words rather than a mnemonic: `para` is *per* + **ad**
  ("toward", as in *advance*), `por` is *pro* + **per** ("through", still alive
  in English *per hour* and *per cent*).
- Explain three phrases the reader has owned since the opening chapters:
  `por favor` is literally *by way of a favour*, `¿por qué?` is *through what?*,
  and `porque` is those two words fused.
- Close on the smallest word carrying the sentence's feeling: *para ti* is a
  gift, *por ti* is a motive — and a wrong choice is a true sentence about a
  different situation, not bad grammar.
- Spanish reaches 157 chapters and 303 lessons.

### Added - source-verified Chinese 口 (HL-C09CB)

- Verify 口 from its pinned Hanzi Writer Data record as three ordered runs:
  left side, joined top-and-right héngzhé, then the closing bottom bar.
- Preserve the joined corner and close-last rule in a two-lift Noto Sans SC fit.
- Reduce measured HL-C09 debt to 152 entries; Chinese has 21 outstanding.

### Added - commands, closing rung 24 (HL-C105)

- Add six chapters at 109-114, placed after the subjunctive because three of
  the four command boxes are the subjunctive.
- Record the arithmetic: a whole mood whose only new material is **eight
  one-syllable words** — `di, haz, ve, pon, ten, sal, sé, ven`.
- The affirmative `tú` command is the he/she present the reader already holds;
  the negative and both `usted` forms are the subjunctive doing its usual job.
- Explain why the `usted` command is the polite one: `usted` is a worn-down
  *vuestra merced*, a title, so the order is aimed past the listener — and that
  indirectness is the politeness.
- Frame the eight irregulars by a checkable fact: every one is a single
  syllable, because commands are shouted and hurried and these are the
  commonest verbs.
- Close with register rather than form: an English speaker's mistake here is
  **person, not tone**. `usted` with an old friend opens a distance.
- Spanish reaches 152 chapters and 298 lessons.

### Added - source-verified Chinese 亻 (HL-C09CA)

- Verify the compressed person radical from its own pinned Hanzi Writer Data
  record: left-falling piě before a separately started vertical shù.
- Fit both ordered medians independently to the narrow Noto Sans SC outline
  instead of mechanically squeezing the full 人 path.
- Reduce measured HL-C09 debt to 153 entries; Chinese has 22 outstanding.

### Added - the present perfect, closing rung 23 (HL-C105)

- Add six chapters at 94-99: the participle, `he` + participle, the full
  `haber` paradigm, the four irregular participles, a review and a synthesis.
- Name the shape: this tense is **built, not conjugated** — six words learned
  once plus one participle per verb, a fixed cost rather than a per-verb one.
- Answer the question learners actually have about `haber`: it means nothing
  here. Spanish gave the meaning to `tener`; English hollowed out `have` the
  same way, which is why *I have eaten* holds nothing.
- Frame the irregular participles as **older than the rule** — `hecho` from
  *factum* (English **fact**), `dicho` from *dictum*, `visto` from *vīsum*,
  `puesto` from *positum* — inherited whole rather than broken.
- Record the third geography split, after `vosotros` and `os`: for something
  that happened today, Madrid says *he hablado* and Mexico City says *hablé*.
- Spanish reaches 146 chapters and 292 lessons.

### Added - source-verified Chinese 人 (HL-C09BZ)

- Open Chinese ductus coverage with the pinned Hanzi Writer Data record for
  **人**, whose ordered medians draw left-falling piě before right-falling nà.
- Fit the two source directions to Noto Sans SC with one verified pen lift while
  recording the Arphic-derived source graphics' different proportions.
- Reduce measured HL-C09 debt to 154 entries; Chinese has 23 outstanding.

### Added - source-verified Hebrew Tav (HL-C09BY)

- Verify printed **ת** from Aural Writing's full-alphabet demonstration as a
  joined top-and-right run followed by one lift and a separate left leg and foot.
- Preserve the source's one-run purple cursive retrace and arch while fitting
  the one-lift printed order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 155 entries and close Hebrew; Chinese is the
  smallest actionable inventory after Arabic's three source-blocked entries.

### Added - the preterite/imperfect contrast, closing rung 20 (HL-C105)

- Add five chapters at 89-93. Neither tense is new; all of the teaching is
  about the choice between them, which is the whole difficulty.
- State the core fact plainly: the two pasts differ in **viewpoint**, not in
  time. The same afternoon can take either.
- Teach `cuando`, which was never taught, because the contrast cannot be shown
  in a single sentence without it.
- Derive `tenía` / `tuve` ("had" / "got") from the aspect rule rather than
  listing it as an exception: a state forced into the preterite gives you the
  moment it began, so the reader can predict any state verb's preterite.
- Hand over a question rather than a table — *am I saying what happened, or
  what things were like?* — because trigger-word lists fail exactly where the
  choice is interesting.
- Close with the first narration in the book: a four-sentence story whose
  imperfects are the background and whose preterites are the foreground.
- Spanish reaches 140 chapters and 286 lessons.

### Added - source-verified Hebrew Shin (HL-C09BX)

- Verify printed **ש** from Aural Writing's full-alphabet demonstration as an
  outer right-base-left run followed by one lift and a descending middle branch.
- Preserve the source's compact one-run purple cursive loop while fitting the
  one-lift printed order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 156 entries; Hebrew has 1 outstanding, with
  the same source's adjacent Tav demonstration queued next.

### Added - two pronouns at once, closing rung 15 (HL-C105)

- Add the double object pronouns at chapters 76-80: the fixed order, the `se`
  substitution, its etymology, a review and a synthesis.
- Introduce **no new pronoun**. The whole arc is one order and one substitution.
- Record that the `se` of `se lo digo` is **not** the reflexive `se` of
  `se llama`: Old Spanish said *gelo* (from *illī* + *illum*), which re-split
  into *ge* + *lo* and then drifted until *ge* sounded like *se*. Two unrelated
  words collided by sound in the sixteenth century.
- Ask the reader to say `le lo digo` aloud before the rule is given, so the
  mouth objects before the grammar does.
- Close on three words carrying a person and a thing with neither named
  (`se la hago`), and on the doubled cost of that compression.
- Spanish reaches 135 chapters and 281 lessons.

### Added - source-verified Hebrew Resh (HL-C09BW)

- Verify printed **ר** from Aural Writing's full-alphabet demonstration as one
  joined top-bar, rounded-corner, and right-downstroke run.
- Preserve the source's rounder one-run purple cursive hook while fitting the
  zero-lift printed order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 157 entries; Hebrew has 2 outstanding, with
  the same source's adjacent Shin demonstration queued next.

### Added - the indirect object pronouns, closing rung 14 (HL-C105)

- Add `le` and `les` at chapters 71-72, a chapter on choosing between the two
  systems, then a review and a synthesis.
- Record that a second complete table costs **two new words**: `me`, `te`, `nos`
  and `os` are identical in both systems, and `les` is derived with the same
  `-s` the reader already uses.
- State the relief plainly: `le` does not mark gender, so one form covers him,
  her and *usted* after four chapters of choosing `lo` against `la`.
- Teach the deepest etymology in the arc: `lo` is Latin *illum* and `le` is
  Latin *illī* — the accusative and the dative. Nouns lost their case endings
  entirely; two cases survived in these pronouns.
- Replace a verb-by-verb list with a test — does the verb act *on* it or aim
  *at* it — and name *leísmo* honestly rather than letting it arrive later as a
  contradiction.
- Spanish reaches 130 chapters and 276 lessons.

### Added - source-verified Hebrew Qof (HL-C09BV)

- Verify printed **ק** from Aural Writing's full-alphabet demonstration as a
  joined top-and-right body followed by one lift and a separate descending stem.
- Preserve the source's one-run purple cursive hook while fitting the printed
  order and below-line stem to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 158 entries; Hebrew has 3 outstanding, with
  the same source's adjacent Resh demonstration queued next.

### Added - the plural object pronouns, closing rung 13 (HL-C105)

- Add `nos`, `los`/`las` and `os` at chapters 66-68, then a review and a
  synthesis, completing the eight-cell direct object set.
- Have the reader **derive** `los` and `las` in a warm-up rather than be taught
  them: the object pronoun pluralises with a plain `-s`, which is the first
  plural in the course that behaves exactly as guessed.
- Pay off the article puzzle from the previous arc: `el` had to become `los`
  because it is a worn-down *elo*, while `lo` kept its vowel and had nothing to
  repair.
- Teach `nosotros` where it belongs, alongside `nos`. The `-mos` ending had been
  in use for thirty chapters and the word itself was never introduced.
- Record that `os` is `vos` worn down, and that the same `vos` survives glued
  into `vosotros` and standing whole as a subject pronoun in Argentina.
- Close on what a pronoun costs: it promises the listener the thing it points
  at is still in the room, and the gender has to point at something real.
- Spanish reaches 125 chapters and 271 lessons.

### Added - source-verified Hebrew Tsadi (HL-C09BU)

- Verify printed **צ** from Aural Writing's full-alphabet demonstration as a
  long diagonal joined to the base, followed by one lift and a short upper arm.
- Preserve the source's compact one-run purple cursive form while fitting the
  printed order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 159 entries; Hebrew has 4 outstanding. Final
  Tsadi is already represented by `forms.final`, so the later Qof is queued next.

### Added - source-verified Hebrew Pe (HL-C09BT)

- Verify printed **פ** from Aural Writing's full-alphabet demonstration as an
  outer top-right-base run followed by one lift and a short inner curl.
- Preserve the source's one-run purple cursive spiral while fitting the printed
  order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 160 entries; Hebrew has 5 outstanding. The
  intervening final-Pe demonstration is already represented by `forms.final`,
  so the same source's later Tsadi demonstration is queued next.

### Added - source-verified Hebrew Ayin (HL-C09BS)

- Verify printed **ע** from Aural Writing's full-alphabet demonstration as one
  joined run: right descent, leftward base, and returning left-branch climb.
- Preserve the source's compact purple cursive loop while fitting the printed
  zero-lift order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 161 entries; Hebrew has 6 outstanding, with
  the same source's Pe demonstration queued next.

### Added - source-verified Hebrew Samekh (HL-C09BR)

- Verify printed **ס** from Aural Writing's full-alphabet demonstration as one
  clockwise loop: flat top, rounded right side, leftward base, and closing left side.
- Preserve the source's rounder purple cursive oval while fitting the printed
  zero-lift order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 162 entries; Hebrew has 7 outstanding, with
  the same source's Ayin demonstration queued next.

### Added - source-verified Hebrew Nun (HL-C09BQ)

- Replace the queued non-writing Nun video with Aural Writing's auditable
  print/cursive demonstration.
- Verify printed **נ** as one joined head, right descent, and leftward base;
  preserve the source's rounder purple cursive hook.
- Reduce measured HL-C09 debt to 163 entries; Hebrew has 8 outstanding, with
  the same source's Samekh demonstration queued next.

### Added - the plural articles, and the plural rule chapter one promised (HL-C108)

- Record a second gap in the noun system: `ES-C01-dia` teaches the vowel plural
  and explicitly defers the consonant one ("later"). It is the only lesson that
  introduces `ES-GRAMMAR-NOUN-NUMBER`, and *later* never came across 115
  chapters. Neither `los` nor `las` was ever taught either.
- Add five chapters at 56-60: `las` (the `-s` lands on article and noun alike),
  `los` (the only article that changes more than its ending), the `-es` rule,
  a review, and a synthesis.
- Explain `los` as the third appearance of one sound change: `el` is a worn-down
  *elo*, while the plural *illos* kept the vowel — the same split as `el`/`lo`.
- Anchor the `-es` rule on `ustedes`, a word the reader has used for fifty
  chapters without being told it is the consonant plural, and state it as one
  rule with a repair rather than two rules.
- Close on the indefinite article **leaving** rather than pluralising: *tengo un
  libro* becomes *tengo libros*, so bare `libros` and `los libros` are two
  different questions.
- Spanish reaches 120 chapters and 266 lessons.

### Added - source-verified Hebrew Mem (HL-C09BP)

- Verify printed **מ** from HebrewPod101's Lamed/Mem lesson: draw the detached
  angled left part, lift once, then join the upper shoulder, right side, and base.
- Preserve the lesson's narrow N-like cursive alternative while fitting the
  demonstrated open printed form to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 164 entries; Hebrew has 9 outstanding, with
  the independently published Nun lesson (`3gYCaDgB-Nk`) queued next.

### Added - three ordinary nouns, and the pronouns that replace them (HL-C106, HL-C105)

- Record the **noun famine**: at chapter 53 the reader held 75 lexical atoms, of
  which the concrete nouns were `café`, `día`, `noche`, `tarde` and `mañana`.
  Every other noun in the course was introduced at chapter 78 or later, while
  gender, number and both article systems were already fully taught.
- Add three general-purpose nouns at chapters 53-55: `la casa` (feminine),
  `el libro` (masculine), and `la comida`, **derived from `comer`** with the
  `-ida` ending, which carries its own gender.
- Add the singular direct object pronouns at chapters 56-60: `lo`, `la`, `me`
  in its plain (non-reflexive) job, and `te`, one new form per chapter, then a
  review and a synthesis.
- Record that `lo` and the article `el` descend from the same Latin `illum`,
  worn down twice because the two jobs stressed it differently, while `la` and
  `la` never split at all.
- Close on the communicative point rather than the forms: the synthesis contrasts
  a grammatically correct dialogue that repeats every noun with the one a speaker
  would actually say.
- Spanish reaches 115 chapters and 261 lessons.

### Added - source-verified Hebrew Lamed (HL-C09BO)

- Verify printed **ל** from HebrewPod101's Lamed/Mem lesson: descend the tall
  left stroke, continue right along the middle bar, and turn diagonally down-left.
- Preserve the lesson's rounded looping handwritten alternative while fitting
  the demonstrated angular print order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 165 entries; Hebrew has 10 outstanding, with
  the same lesson's Mem demonstration queued next.

### Added - deriving the remaining plurals, and closing rung 10 (HL-C105)

- Add a consolidation chapter that introduces almost nothing: the remaining
  irregular plurals are **derived** by the reader from rules already held.
- `queremos`/`quieren` and `podemos`/`pueden` follow from the stress rule alone
  (the stem breaks exactly where it is stressed), so the lessons ask for the
  form **before** giving it — producing it is the evidence the rule stuck.
- Record that the `-go` club has **no plural forms of its own**: the `-go` was
  only ever in *yo*, so `decimos`/`dicen` and `venimos`/`vienen` break only as
  far as the stress rule already predicts.
- Close with a synthesis: three rules — family endings, the stress rule, and
  *yo* being where irregularity hides — generate the whole present tense, and
  the reader conjugates two verbs the course never taught them.
- Rung 10 (present plural) is complete; Spanish reaches 107 chapters.

### Added - source-verified Hebrew Kaf (HL-C09BN)

- Verify printed **כ** from HebrewPod101's dedicated Kaf lesson: draw the top
  left-to-right, continue down the rounded right side, and turn left along the
  base without lifting.
- Preserve the lesson's rounded handwritten half-circle while fitting its sharp
  printed corners to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 166 entries; Hebrew has 11 outstanding, with
  the series' Lamed/Mem lesson (`CBU6aSCcPrE`) queued next.

### Added - tener and ir in the plural (HL-C105)

- Completes the four commonest verbs in Spanish. Before this the reader could
  conjugate any regular verb but only two irregulars.
- **`tener` states the stem-change rule outright**: the stem breaks exactly
  where it is stressed. That is why *nosotros* and *vosotros* never break, in
  every stem-changing verb in the language -- and why the "boot" mnemonic is
  unnecessary. It is the same stress principle that made `-er` and `-ir` look
  identical in the singular and part in the plural.
- **`ir` is a third suppletive.** Its whole present descends from *vadere*, not
  *ire*, so the present is not irregular at all: it is **regular for a verb
  whose infinitive Spanish stopped using**. English does the same with
  *go*/*went*, where *went* belongs to *wend*.
- Three of the four now teach one transferable tool: a verb that looks chaotic
  is usually **more than one word wearing a single name**, and the
  regular-looking parts are where one word survived intact.
- Reuses `ES-ETYMON-IRE-VADERE` rather than minting a near-duplicate
  `ES-ETYMON-VADERE` for the same fact.
- Renames a `concept_tag` that matched `verbs.ts`'s `/(^|-)VERB-/` namespace
  test -- the second time this arc that a plausible tag name would have reported
  a grammar lesson as a Spanish verb.
- Spanish 104 -> **106 chapters**, 249 lessons.

### Added - source-verified Hebrew Yod (HL-C09BM)

- Verify printed **י** from HebrewPod101's Tet/Yod lesson: draw the tiny head
  left-to-right and continue down the short stem without lifting.
- Preserve the lesson's comma-like handwritten alternative while fitting its
  small printed angle to compact Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 167 entries; Hebrew has 12 outstanding, with
  the series' dedicated Kaf lesson (`EcQ0gL-NM-k`) queued next.

### Added - estar in the plural (HL-C105)

- Placed beside `ser`'s plural rather than back at `estar`'s own early chapter,
  so the `ser`/`estar` contrast holds in both numbers instead of only the
  singular.
- The review sets the two paradigms side by side and makes the point they are
  hard for **opposite** reasons. `ser` is two Latin verbs fused (*esse* +
  *sedere*), so its forms share no shape. `estar` is one verb (*stare*), so its
  plural is a plain `-ar` paradigm -- and the only oddity in the whole column is
  where the stress falls, which the written accent records honestly.
- That generalises, and the lesson says so: where a Spanish verb looks chaotic,
  the usual explanation is not that it decayed strangely but that it is **more
  than one word wearing a single name**.
- Re-sequenced 122 lessons by +10 to open room between the `ser` plural and the
  next chapter. Sequences are internal ordering, not stable ids; lesson ids are
  unchanged as always.
- Spanish 103 -> **104 chapters**, 245 lessons.

### Added - source-verified Hebrew Tet (HL-C09BL)

- Verify printed **ט** from HebrewPod101's dedicated Tet/Yod lesson: descend the
  left side and continue right along the base, lift once, then climb from the
  lower-right before turning down-left into the inward hook.
- Preserve the lesson's unusual bottom-up, one-run rounded handwriting while
  fitting its printed order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 168 entries; Hebrew has 13 outstanding, with
  the same lesson's Yod demonstration queued next.

### Added - source-verified Hebrew Heit (HL-C09BK)

- Verify printed **ח** from HebrewPod101's dedicated Zayin/Heit lesson: draw the
  top bar left-to-right and continue down the right side, lift once, then draw
  the joined left leg from top to bottom.
- Preserve the lesson's rounded handwritten alternative while fitting its
  sharp-cornered printed order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 169 entries; Hebrew has 14 outstanding, with
  the series' Tet/Yod lesson (`NBUtBPVKchk`) queued next.

### Added - source-verified Hebrew Zayin (HL-C09BJ)

- Verify handwritten **ז** from HebrewPod101's dedicated Zayin/Heit lesson: rise
  briefly to the right, then curve down and around the base without lifting.
- Preserve the lesson's handwritten-Gimel mirror contrast and Vav warning while
  fitting the one-stroke order to Noto Sans Hebrew's block outline.
- Reduce measured HL-C09 debt to 170 entries; Hebrew has 15 outstanding, with
  the same lesson's Heit demonstration queued next.

### Added - source-verified Hebrew Vav (HL-C09BI)

- Verify printed **ו** from HebrewPod101's dedicated Vav lesson: draw the small
  head left-to-right and continue straight down the stem without lifting.
- Preserve the lesson's simpler handwritten top-to-bottom form while excluding
  its later Hirik and Shuruk vowel signs from base U+05D5's zero-lift count.
- Reduce measured HL-C09 debt to 171 entries; Hebrew has 16 outstanding, with
  the series' Zayin/Heit lesson (`XTqG_1dsFSU`) queued next.

### Added - ser in the plural (HL-C105)

- The highest-value irregular to take first: the commonest verb in Spanish, and
  the reader held only its singular. `somos`, `sois`, `son`, plus a review, one
  cell per lesson.
- The review earns a fact memorisation never gives: **five of ser's six forms
  begin with `s-`, and `eres` does not** -- because it comes from somewhere
  else. `ser` is two Latin verbs fused: *esse* gives the `s-` forms, *sedere*
  gives the infinitive and the future. The irregularity is not chaos; it is a
  seam, and you can see exactly where it runs.
- English does the same thing for the same reason: *am*, *is*, *are*, *was*,
  *be* come from three separate Old English verbs. Languages seem to build "to
  be" out of spare parts.
- Even here the `-mos` survives. A verb that threw away everything else kept the
  most stable ending in the language.
- Spanish 102 -> **103 chapters**, 241 lessons.

### Added - source-verified Hebrew Hei (HL-C09BH)

- Verify printed **ה** from HebrewPod101's dedicated Hei lesson: draw the top
  bar left-to-right and continue down the right side, lift once, then draw the
  detached left leg from top to bottom.
- Preserve the lesson's explicitly contrasted curved handwritten form while
  fitting its sharp-angled printed order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 172 entries; Hebrew has 17 outstanding, with
  the series' dedicated Vav/Hirik/Shuruk lesson (`kJUMyHR0zN4`) queued next.

### Added - the present tense is complete: the -er/-ir plurals (HL-C105)

- Four chapters finishing rung 10's present tense, and they turn on the fact
  §5.2a exists for: **-er and -ir are identical in four of the six slots and
  differ in exactly two** -- *nosotros* and *vosotros*.
- So `comen`/`viven` share one lesson (same form, one fact) while
  `comemos`/`vivimos` do not (different forms, two facts). The rule counts new
  forms, not new slots, and this is the first place it cuts both ways in the
  same chapter run.
- `vivimos` explains **why the singular hid the difference**: there the ending is
  unstressed and its vowel wears toward the other family's; in the plural the
  stress lands on the ending and each family's own vowel survives. The families
  never diverged -- the singular muffled them.
- The closing review prints **eighteen forms**: the present tense of every
  regular Spanish verb, each built separately before appearing in the grid.
- Fixes a rotting sentinel in the consumer: `bookhashes` used chapter 99 as its
  "no such chapter" case, having already moved from 42 for the same reason.
  Now 9999, which cannot become a chapter.
- Spanish 98 -> **102 chapters**, 237 lessons. Fully drivable chapters
  **378 -> 381**.

### Added - source-verified Hebrew Dalet (HL-C09BG)

- Verify cursive **ד** from HebrewPod101's source-adjacent Dalet
  demonstration: sweep the broad arch through its small loop and continue into
  the descending tail without lifting.
- Preserve the instructor's explicit "just one curve" order while fitting that
  single run to the angular Noto Sans Hebrew top bar and right downstroke.
- Reduce measured HL-C09 debt to 173 entries; Hebrew has 18 outstanding, with
  the series' dedicated Hei lesson queued next.

### Added - the plural, at last: the -ar present (HL-C105)

- Opens rung 10, the largest gap the HL-C105 audit found. The book taught five
  tenses and a mood, all singular; a reader could not say *we speak*.
- Five chapters in the shape this arc established: `hablamos`, `hablan`,
  `hablais` -- one cell per lesson -- then a review and a synthesis. The order is
  deliberate: the universally useful forms first, the Spain-only one last.
- The review carries **the first complete paradigm in the book**. Six forms,
  withheld for thirty-two chapters until every box in it was earned. Most
  courses open with that table; HL10 §5.3 says a table you cannot fill is a
  picture of work still to do, and one you can fill is a picture of ground you
  hold.
- The synthesis makes `vosotros`/`ustedes` a genuine split rather than a
  footnote, which is rung 10's stated requirement. It is **the first choice in
  the book decided by geography rather than meaning** -- and neither form
  corrects the other: roughly 40 million speakers use `vosotros` daily, and
  several hundred million never have.
- `ustedes` takes the *they* form for exactly the reason `usted` takes the *he*
  form -- it is *vuestras mercedes*, the plural of the phrase behind `usted`.
  And `vosotros` is *vos + otros*, "you others", a repair Spain needed because
  `vos` had drifted singular there, and the Americas never needed because
  `ustedes` already covered the plural.
- Spanish 93 -> **98 chapters**, 232 lessons. Fully drivable chapters
  **374 -> 378**.

### Added - source-verified Hebrew Gimel (HL-C09BF)

- Verify printed **ג** from HebrewPod101's dedicated Gimel/Dalet lesson: join
  the short left-to-right top bar to the right stem and short lower-right leg,
  lift once, then draw the longer diagonal leg down-left.
- Preserve the lesson's explicitly contrasted rounded cursive form as a
  documented variation while fitting the angular order to Noto Sans Hebrew.
- Reduce measured HL-C09 debt to 174 entries; Hebrew has 19 outstanding, with
  Dalet's one-curve demonstration queued next.

### Added - a synthesis chapter for the vocabulary run (HL-C100)

- Chapters 56-69 are **fourteen consecutive lexical chapters** -- colours,
  family, body, food, seasons, months, time, weather, numbers, animals -- and
  nothing between them ever asked the reader to combine what they held. A
  learner arriving at the end had **27 concrete nouns** and had never been asked
  to say one thing about their own life.
- Chapter 70 is that chapter. It is also the one that surfaced HL-C104: writing
  it was impossible until the indefinite article existed, because describing
  your own life means introducing things your listener does not know about yet,
  which is exactly the job `un` does.
- So the article contrast is the grammar point rather than an aside: *el perro*
  is the dog we were already discussing; *un perro* is one you are mentioning
  for the first time. Almost every sentence in the chapter needs the second.
- Spanish 92 -> **93 chapters**, 227 lessons. Fully drivable chapters
  **373 -> 374**.

### Added - source-verified Hebrew Bet (HL-C09BE)

- Verify block-style handwritten **ב** from HebrewPod101's dedicated Bet
  demonstration: join the left-to-right top bar to the right descent, lift once,
  then draw the baseline left-to-right.
- Keep the optional dagesh separate from base U+05D1's two-stroke, one-lift
  body and preserve that distinction in the source note.
- Reduce measured HL-C09 debt to 175 entries; Hebrew has 20 outstanding, with
  the series' dedicated Gimel/Dalet lesson queued next.

### Added - the indefinite article, which was never taught (HL-C104)

- `un` and `una` did not exist anywhere in the corpus. A learner reaching
  chapter 68 held 27 concrete nouns -- colours, family, body parts, food,
  animals -- and could say *the* dog but not *a* dog.
- HL10 §5.4 rung 3 says "definite article, **then indefinite**", and §12.2 block
  11 says "four lessons, **then indefinite**". The definite articles shipped;
  the indefinite ones never followed.
- Now chapter 3, immediately after the definite articles and the agreement
  payoff, where the learner already holds gender and three nouns: `un`, `una`,
  and a review that lines up all four articles.
- The etymology is a gift. **`un` IS the number one** -- Latin *unus* -- and
  Spanish never separated them, so *un dia* is *a day* and *one day* at once.
  English made the identical move and hid it better: *a*/*an* is the word *one*
  worn down, which is why *an hour* and *one hour* begin with the same sound.
  Spanish left the seam visible.
- Fixes seven book targets whose filenames disagreed with their chapter numbers.
  The early chapters use a zero-padded `ch03-` prefix and the renumber regex
  looked for `/ch3-`, so those targets kept a stale filename through several
  renumbers -- surfacing only now as "book chapter 3 occurs twice".
- Spanish 91 -> **92 chapters**, 226 lessons. Fully drivable chapters
  **372 -> 373**.

### Added - source-verified Hebrew Alef (HL-C09BD)

- Verify handwritten **א** from HebrewPod101's dedicated Alef lesson: draw the
  main descending diagonal, lift once, then draw the opposing diagonal through
  the crossing.
- Preserve the demonstrated two-stroke, one-lift order while explicitly
  adapting its compact X-like handwriting to the vendored Noto Sans Hebrew
  block outline.
- Record the inaccessible Arabic Mim and Nun media and the recovered,
  out-of-inventory Arabic Faa source without changing HL-C09's fixed denominator;
  measured debt is now 176 entries, with 21 Hebrew entries outstanding.

### Changed - the preterite, and the tense cliff is gone (HL-C100)

- Split the preterite into three: the regular forms, the strong preterites, and
  a synthesis. They are two different pasts with two different histories, and
  the chapter had been treating them as one.
- The synthesis is a **recognition rule**, not a paradigm. The two kinds differ
  by where the stress falls, and it is audible: *comi* stresses the ending and
  therefore carries a written accent; *tuve* stresses the stem and therefore
  carries none. The accent is not extra spelling to memorise -- it is the
  stress, written down. **A preterite with no accent is a strong one.**
- The strong preterites are Latin's own perfect, carried through on verbs used
  too heavily ever to be rebuilt -- the same reason the imperfect has only three
  irregulars.
- **The cliff is gone.** Four consecutive chapters once carried 7, 8, 10 and 9
  grammar atoms against a book mean of 1.4. The worst chapter in the book is now
  **4**, and nothing exceeds it.
- Caught by the ledger gate: I referenced `ES-ORTHOGRAPHY-ER-IR-PRETERITE-ACCENT`,
  an atom that does not exist -- I had reconstructed the name from a truncated
  console display of `...-FINAL-STRESS`. Validation caught it in the lesson;
  `payoffsNotClosed` then caught the same wrong name still sitting in the
  chapter ledger after I had fixed only the lesson.
- Spanish 89 -> **91 chapters**, 223 lessons. Fully drivable chapters
  **370 -> 372**.

### Added - source-verified Arabic independent waw (HL-C09AZ)

- Verify independent **و** from the University of Oregon page's directly linked
  `waw.mov`: close the small head loop from its lower-right junction, then
  continue down and left through the tail without lifting.
- Preserve the video's one-stroke, zero-lift order, one-way-connector context,
  w/long-ū roles, and Arabic-scoped Noto Naskh provenance independently of
  Persian **و**.
- Reduce the measured HL-C09 debt to 177 entries; Arabic has 3 outstanding,
  with source recovery for independent **م** queued next.

### Changed - the imperfect, and the spike is flattened (HL-C100)

- Split the imperfect into four: the regular forms, `ver`, the three irregulars,
  and a synthesis. `ver` had been sitting mid-chapter as a new verb, inserted
  only because `veia` needed it; it now has its own chapter and its own
  etymology.
- The synthesis carries the fact no single lesson could: **the imperfect has
  exactly three irregular verbs in the entire language** -- *ser*, *ir*, *ver*.
  Not "the common ones", not "to start with". Three. Every other Spanish verb
  takes `-aba` or `-ia` and behaves.
- And they are irregular *because* they are the most-used verbs in the language,
  worn straight through from Latin *erat* and *ibat* without ever being tidied
  up. Rare verbs get regularised precisely because nobody remembers them well
  enough to keep an odd shape alive. The exceptions are the words you use most,
  which is the reason they are exceptions.
- **The ramp spike is flattened.** Worst chapter is now **7** grammar atoms,
  down from 10 three slices ago, with everything else at 4 or below against a
  book mean of 1.4.
- Spanish 86 -> **89 chapters**, 222 lessons. Fully drivable chapters
  **367 -> 370**.

### Changed - the subjunctive, split by idea (HL-C100)

- The subjunctive was the worst remaining chapter at 9 grammar atoms. Split into
  five, by **idea** rather than by count: the non-assertion concept alone (the
  part learners actually struggle with), the regular forms, the yo-stem
  irregulars, `ojala`, and a synthesis.
- The synthesis makes the mood a **stance**, not a tense. *Hablas espanol* puts
  a fact on the table; *quiero que hables espanol* claims nothing at all. Every
  later use of the mood -- doubt, denial, hope -- is the same move, so the rule
  is one sentence: *indicative: this is so; subjunctive: I am not saying this is
  so.*
- `ojala` closes the chapter because it explains itself: Arabic *wa-sha' allah*,
  "and may God will it", carried through eight centuries of al-Andalus and kept
  long after the religion behind it. A thing God has yet to will is, by
  definition, not a fact anyone can assert -- which is why the word can only
  ever take this mood.
- Caught while writing: the exchange used `mi madre`, and `madre` is taught 37
  chapters later. Replaced with a name.
- Spanish 82 -> **86 chapters**. Worst chapter now **8** grammar atoms, down
  from 9; fully drivable chapters **364 -> 367**.

### Changed - the steepest chapter in the book, split (HL-C100)

- Re-measured grammar load across all 79 chapters. The ramp's remaining spike is
  **not in the opening**: chapters 39-42 carried **7, 8, 10 and 9** grammar
  atoms against a book mean of **1.4** -- four consecutive chapters, each a
  whole tense or mood system, with nothing between them.
- Chapter 41 was the worst and held **two** systems: the future AND the
  conditional, plus the irregular stems they share. It is now four chapters --
  the future, the conditional, the shared stems, and a synthesis.
- The synthesis pays off an etymology the course already taught: both endings
  are *haber* glued onto the infinitive, once in the present and once in the
  past. So future-versus-conditional is **one system seen from two moments** --
  promise from the present, suppose from the past -- and English's *will*/*would*
  is the same present/past pair of one old verb.
- Spanish 79 -> **82 chapters**. Worst chapter now **9** grammar atoms, down from
  10; fully drivable chapters **361 -> 364**; R2 misses **1824 -> 1822**.

### Added - source-verified Arabic independent heh (HL-C09AY)

- Verify independent **ه** from the University of Oregon page's directly linked
  `letter-haa.mov`: close the lower counter, thread through the centre into the
  upper-right counter, then sweep left along the baseline without lifting.
- Preserve the video's one-stroke, zero-lift order, two-way-connector context,
  and Arabic-scoped Noto Naskh provenance independently of Persian **ه**.
- Reduce the measured HL-C09 debt to 178 entries; Arabic has 4 outstanding,
  with the same page's directly linked independent **و** lesson queued next.

### Fixed - prose that names a chapter number, and a gate so it stays fixed (HL-C102)

- Chapter numbers move on every split; lesson ids never do. A sentence like
  "you learned this in chapter 14" is right when written and wrong three
  renumbers later, and **nothing fails** -- the reader just follows a pointer
  into the wrong chapter.
- Three Spanish references were already wrong: `ES-C09-esta-en` sent the reader
  to "Chapter 7" for a question now taught in 24; `ES-C41-explicar` placed
  `contar` in "chapter 38" when it had reached 71; and a comment in `ES-C19-no`
  that was **corrected two PRs ago** had gone stale again, because HL-C101 moved
  the lesson it named.
- All 32 Spanish references now name the thing instead of a number -- "since the
  repair kit", "when you first met them", "the next chapter". Spanish is at zero.
- Adds `tests/chapter-references.test.ts`, which counts **cross-chapter**
  references only: a lesson naming its own chapter (an `# Chapter 2` heading)
  points nowhere else and cannot rot. Spanish is held at zero; the other 19
  tracks are pinned at their current 710 so the debt cannot grow while they are
  stable, and should be cleared before they start splitting chapters.

### Added - source-verified Arabic independent yaa (HL-C09AX)

- Verify independent **ي** from the University of Oregon page's directly linked
  `yaa.mov`: one continuous independent bowl, then the lower-left dot, then the
  lower-right dot.
- Preserve the video's three-stroke, two-lift order, two-way-connector context,
  and Arabic-scoped Noto Naskh provenance independently of Urdu **ی** U+06CC.
- Reduce the measured HL-C09 debt to 179 entries; Arabic has 5 outstanding,
  with the directly linked independent **ه** lesson reprioritized next.

### Added - source-verified Arabic independent lam (HL-C09AW)

- Verify independent **ل** from the University of Oregon page's directly linked
  `lam.mov`: descend the tall upright and turn left through the base bowl in one run.
- Preserve the video's one-stroke, zero-lift order, two-way-connector context,
  and Arabic-scoped Noto Naskh provenance independently of Persian and Urdu **ل**.
- Reduce the measured HL-C09 debt to 180 entries; Arabic has 6 outstanding,
  with the same page's directly linked independent **ي** lesson queued next.

### Changed - the -ar synthesis can finally say "hablo espanol" (HL-C101)

- `hablar` was taught at chapter 15 but `espanol` sat at 22, so the synthesis
  chapter between them had nothing for the verb to take. Its exchange asked a
  bare "**Hablas?**" -- a workaround for a missing noun, not a sentence anyone
  says.
- `espanol` and the first built sentence now precede the synthesis. The exchange
  reads "**Hablas** espanol?" / "Si, **hablo** espanol", which is the most
  useful thing the whole -ar arc affords.
- The first attempt moved `espanol` alone and broke validation:
  `ES-C06-hablo-espanol` uses `trabajar` and `estudiar` as its worked examples,
  so the run had to move together. Final order is `-ar` review -> `trabajar` ->
  `estudiar` -> `espanol` -> *hablo espanol* -> **synthesis**, which is a better
  ramp than the original: the synthesis now exercises everything before it
  rather than only the three cells.
- Spanish 78 -> **79 chapters**. Fully drivable chapters **360 -> 361**;
  R2 reinforcement misses **1825 -> 1824**.

### Changed - one verb per chapter, slice 6: chapter 20 (HL-C99)

- Split chapter 20 -- `trabajar` and `estudiar` -- into three chapters, the
  third keeping `espanol` and the first built sentence together.
- Milder crowding than the earlier slices: both verbs reuse the `-ar` pattern
  and introduce no grammar at all. They are split because each owes the reader
  an origin, and these two earn the room -- `trabajar` from *tripaliare*, from
  *tripalium*, a three-stake torture device, which is why English **travail** is
  the same word; `estudiar` from *studere*, "to be eager".
- No new lessons: this slice is chapter boundaries only.
- Spanish 76 -> **78 chapters**; old 21-76 -> 23-78. Fully drivable chapters
  **358 -> 360**.

### Added - source-verified Arabic independent kaf (HL-C09AV)

- Verify independent **ك** from the University of Oregon page's directly linked
  `kaf.mov`: descend the main upright and turn left along the baseline in one
  run, then lift once and draw the inner arm from upper right down-left.
- Preserve the video's two-stroke, one-lift order, two-way-connector context,
  and Arabic-scoped Noto Naskh provenance independently of Urdu **ک**.
- Reduce the measured HL-C09 debt to 181 entries; Arabic has 7 outstanding,
  with the same page's directly linked independent **ل** lesson queued next.

### Changed - one verb per chapter, slice 5: chapter 30 (HL-C99)

- Split chapter 30 -- `poner`, `salir`, `venir` -- into five: one verb each, a
  review chapter that closes the `-go` club, and a synthesis chapter.
- The review's thesis is historical, not formal: **the six `-go` verbs are not a
  family.** They came from different Latin verbs by different routes and
  converged on the same ending by accident. That has a practical consequence
  worth telling a learner -- a pattern with a cause keeps recruiting, an
  accident stops -- so the list of six is **closed**.
- The synthesis observes that the irregularity lives in exactly one slot, `yo`,
  which is the form a beginner uses most, and pairs each `-go` form with its
  perfectly regular `tu`/`el` counterpart.
- Spanish 72 -> **76 chapters**; old 31-72 -> 35-76. Fully drivable chapters
  **354 -> 358**.

### Fixed - three English homographs reporting as forward references (HL-C103)

- `comes`, `hand` and `regular` are ordinary English words that are also target
  headwords, so sentences like "*comer* **comes** from Latin *comedere*" and
  "**Regular** stress: TAR-de" reported the Spanish word as a forward reference
  from a lesson that was writing English.
- Added by **census, not guesswork**: of 423 forward references, 368 matched via
  emphasis and 55 in plain prose only; 18 of those were pure-ASCII candidates
  and exactly three were English. The other 15 are genuine and must keep
  reporting -- a list built from a plausible wordlist would have suppressed
  them. The census method is now recorded in `continuity.ts`.
- Records that the structural alternative -- guard the plain path only, and
  trust emphasis to mean "target language" -- was tried and is wrong: authors
  emphasise English for stress too.
- Also drops `tres` from an activity's `accepted` list, where it offered credit
  for a word the reader has not met.
- Forward references **423 -> 418**.

### Added - source-verified Arabic independent ayn (HL-C09AU)

- Verify independent **ع** from the University of Oregon page's directly linked
  `ayn.mov`: shape the open head, then continue around the lower bowl without lifting.
- Preserve the video's one-stroke, zero-lift order, two-way-connector context,
  and Arabic-scoped Noto Naskh provenance independently of adjacent Ghayn.
- Reduce the measured HL-C09 debt to 182 entries; Arabic has 8 outstanding,
  with the directly linked independent **ك** lesson queued next.

### Changed - one verb per chapter, slice 4: chapter 21 (HL-C99)

- Split chapter 21, the book's **second and third paradigms**. They were still
  bundled three-cells-to-a-lesson while `-ar` had just been given five chapters,
  so the ramp contradicted itself mid-book.
- `comer` now owns a chapter with one `-er` cell per lesson -- `como`, `comes`,
  `come` -- and a review lesson that earns `ES-GRAMMAR-ER-PRESENT-SINGULAR`.
  The `yo` slot is taught as **free**: `-o` is the ending the learner already
  owns, and only the other two slots carry new information.
- **`-ir` deliberately did not get the same treatment.** In the singular its
  endings ARE the `-er` endings, so three per-cell lessons would have been
  padding. `vivir` declares all three CONJ3 cells in one lesson on the grounds
  that `maxNewGrammarCellsPerLesson` should count **new forms, not new slots**.
  Flagged for HL10 §5.2.
- Add a synthesis chapter where the three families and the two question words
  become a four-line exchange in which every word is one the reader built.
- Fix three forward references introduced while writing, one of them subtle:
  "*comer* **comes** from Latin *comedere*" made the English verb *comes*
  report as the Spanish `tú` form, six sequences before its lesson.
  `continuity.ts` guards this class of collision but does not list `comes`;
  filed as HL-C103.
- Spanish 69 -> **72 chapters**; old 22-69 -> 25-72. Paradigm-shaped tables
  **93 -> 91**; R2 reinforcement misses **1827 -> 1826**; forward references
  unchanged at **423**.

### Changed - one verb per chapter, slice 3: chapter 62 (HL-C99)

- Split chapter 62 -- `traer`, `conseguir`, `jugar`, `conocer` -- into six:
  one verb each, a review chapter, and a synthesis chapter. **No Spanish
  chapter now teaches four verbs.**
- The review chapter closes the stem-change system. e->ie and o->ue were
  already held; `conseguir` adds e->i and `jugar` adds u->ue. There are
  **four patterns in total, and u->ue has exactly one member in the entire
  language** -- which is a single fact wearing a pattern's shape, and easier
  once said out loud.
- The synthesis chapter collects the three pairs where English offers one word
  and Spanish forces a choice: *preguntar*/*pedir*, *traer*/*llevar*,
  *conocer*/*saber*. They were taught chapters apart and had never been placed
  side by side. The decision, not the word, is the work.
- Fix a `concept_tag` that would have corrupted the verb-coverage report:
  `ES-SYNTHESIS-VERB-SPLITS` matches `verbs.ts`'s `/(^|-)VERB-/` namespace
  test, so a synthesis lesson was being counted as a Spanish verb named
  *splits*. Renamed `ES-SYNTHESIS-PAIR-CHOICES`; audited every review and
  synthesis tag added in this arc, and this was the only one.
- Remove three self-references ("this course", "the course") caught by
  `standalone-book`. One curriculum derives N books; no derived book may claim
  to be the course.
- Spanish 64 -> **69 chapters**; old 63-64 -> 68-69. Fully drivable chapters
  **346 -> 351**.

### Added - source-verified Arabic independent daad (HL-C09AT)

- Verify independent **ض** from the University of Oregon page's embedded Daad
  lesson: repeat Saad's oval, shoulder, and restarted bowl, then lift again to
  place the upper dot last.
- Preserve the lesson's three-stroke, two-lift order, two-way-connector context,
  and Arabic-scoped Noto Naskh provenance while recording the direct MOV's 403.
- Reduce the measured HL-C09 debt to 183 entries; Arabic has 9 outstanding,
  with the source-backed independent **ع** lesson queued next.

### Changed - one verb per chapter, slice 2: chapter 53 (HL-C99)

- Split chapter 53 -- `tomar`, `preguntar`, `ayudar`, `gustar` -- into six
  chapters: one verb each, a review chapter, and a synthesis chapter.
- Place `gustar` **after** the review rather than beside the other three. It is
  not a fourth verb; it is the reverse-subject system. The review chapter states
  the shared shape out loud ("the sentence is about the one doing it") so the
  next chapter has something to break. A review chapter is not only
  consolidation -- it is where a contrast gets its setup.
- The synthesis chapter closes a joke 53 chapters in the making: *mucho gusto*,
  taught in chapter 5, is `gustar`'s own noun. It has always meant *much
  pleasure*.
- Fix a stale author comment in `ES-C19-no` that named "chapter 14" for the
  negator sense. That lesson moved to chapter 20 three renumbers ago. The
  comment now names the lesson id, which cannot rot. Audited all 31 prose
  chapter references; this was the only stale one.
- Spanish 59 -> **64 chapters**; old 54-59 -> 59-64. Atoms never revisited
  **459 -> 456**; roots never spent **1803 -> 1801**; fully drivable chapters
  **341 -> 346**.

### Changed - one verb per chapter, starting with the four of the mind (HL-C99)

- Split chapter 47 — which taught `pensar`, `entender`, `leer` and `escribir`
  back to back with **no practice or payoff lesson of any kind** — into six:
  one verb per chapter, then a **review** chapter and a **synthesis** chapter.
- Close a duplicate teaching: `entender` re-introduced `no entiendo` 34 chapters
  after the repair kit taught it, without referencing or practising
  `ES-LEX-NO-ENTIENDO-01`. It now requires and practises that atom, and the
  synthesis chapter turns the frozen formula into a sentence with parts —
  *intendere*, "to stretch toward," so *no entiendo* never said *I failed*.
- The review chapter's thesis is what no single verb chapter could show: all four
  mind-verbs began as physical acts — weighing, stretching, gathering, scratching
  — and *legere*, the root of `leer`, sits inside English **intellect**.
- Add `ES-EXT-031-CONSOLIDATION`; `ES-PATH-031` had no extension node, so the two
  new support lessons belonged nowhere. Staged **A2** from the path segment's own
  spine node — the validator does not gate stage, so a wrong one would have
  silently corrupted level reporting.
- Spanish 54 → **59 chapters**; old 48–54 → 53–59. R1 reinforcement misses
  **865 → 863**; fully drivable chapters **336 → 341**.

### Added - source-verified Arabic independent saad (HL-C09AS)

- Verify independent **ص** from the University of Oregon page's directly linked
  `FullSizeRender-6.mov`: close the oval clockwise, rise into its short shoulder,
  then lift once and restart at the baseline junction for the trailing bowl.
- Preserve the video's two-stroke, one-lift order, two-way-connector context,
  and Arabic-scoped Noto Naskh provenance independently of adjacent Seen and Shiin.
- Reduce the measured HL-C09 debt to 184 entries; Arabic has 10 outstanding,
  with the page's directly linked Daad MOV queued next.

### Changed - the first paradigm, one cell per chapter (HL-C98)

- Split `ES-C06-ar-presente` — which taught `hablo`, `hablas` and `habla` in a
  single lesson, behind a three-row table on first exposure, with pro-drop
  alongside — into five chapters: **15** *hablo* and pro-drop, **16** *hablas*,
  **17** *habla*, **18** a **review** chapter, **19** a **synthesis** chapter.
  `maxNewGrammarCellsPerLesson` is 1; this lesson taught three.
- Keep `ES-C06-ar-presente` as the review chapter so the 14 lessons that require
  `ES-GRAMMAR-AR-PRESENT-SINGULAR` keep resolving; the atom is now *earned* at
  the recap rather than asserted at the introduction. Its table is unchanged and
  finally legitimate.
- Add the corpus's **first `teaches_cells:` declarations** — coverage moves
  **0 → 3 of 231** against `spanish/grammar-cells.json`, whose
  `1SG → 2SG → 3SG` prerequisite chain already prescribed exactly this order.
- Add the book's first chapters that introduce **zero** new atoms. Chapter 19
  makes the register choice itself the communicative act: one conversation held
  twice, warmly then respectfully, where the only thing that changes is one
  letter on one verb.
- Renumber Spanish 50 → **54 chapters** (old 16–50 → 20–54). Lesson ids are
  stable slugs and deliberately do not renumber. Forward references **424 → 423**;
  fully drivable chapters **332 → 336**; chapter 18 is `sight`, because a
  paradigm table cannot be read aloud.

### Added - source-verified Arabic independent shiin (HL-C09AR)

- Verify independent **ش** from the University of Oregon page's directly linked
  `FullSizeRender-7.mov`: draw the complete teeth-and-bowl body continuously,
  then place the lower-left, lower-right, and centered upper dots separately.
- Preserve the video's four-stroke, three-lift order, two-way-connector context,
  and Arabic-scoped Noto Naskh provenance independently of Urdu **ش**.
- Reduce the measured HL-C09 debt to 185 entries; Arabic has 11 outstanding,
  with the page's directly linked Saad MOV queued next.

### Added - source-verified Arabic independent seen (HL-C09AQ)

- Verify independent **س** from the University of Oregon page's directly linked
  `FullSizeRender-8.mov` at 00:01.6–00:02.8 as one continuous pen-down run:
  shape three close teeth right-to-left, then flow directly into the final bowl.
- Preserve the page's two-way-connector context and Arabic-scoped Noto Naskh
  provenance independently of the already-authored Persian and Urdu **س** sources.
- Reduce the measured HL-C09 debt to 186 entries; Arabic has 12 outstanding,
  with the same page's directly linked Shiin MOV queued next.

### Added - source-verified Arabic independent raa (HL-C09AP)

- Verify independent **ر** from the University of Oregon page's directly linked
  `raa.mp4` at 00:08.8–00:09.3 as one continuous pen-down run: descend from the
  upper tip through the short stroke, then sweep left through the lower curve.
- Preserve the page's one-way-connector context and Arabic-scoped Noto Naskh
  provenance independently of the already-authored Urdu **ر** source.
- Reduce the measured HL-C09 debt to 187 entries; Arabic has 13 outstanding,
  with the next measured **س** backed by the dedicated page's direct MOV link.

### Added - source-verified Arabic independent daal (HL-C09AO)

- Verify independent **د** from the University of Oregon page's directly linked
  `letter-daal-2.mp4` demonstration.
- Preserve its zero-lift order: begin at the upper tip, descend down-right
  through the curved shoulder, then turn left along the baseline in the same
  pen-down run.
- Keep one-way-connector context and Arabic-scoped Noto Naskh provenance while
  reducing the measured HL-C09 debt to 188 entries; Arabic has 14 outstanding.

### Added - source-verified Arabic independent khaa (HL-C09AN)

- Resolve the chapter's `kha.mov` through its WordPress attachment ledger and
  verify independent **خ** from its own demonstration.
- Preserve the clip's body-first order: draw the short upper head left-to-right,
  continue around the bowl without lifting, then lift once for the dot above.
- Keep two-way-connector context and Arabic-scoped Noto Naskh provenance while
  reducing the measured HL-C09 debt to 189 entries; Arabic has 15 outstanding.

### Added - source-verified Arabic independent haa (HL-C09AM)

- Resolve the chapter's unlinked `Haa.mov` through its WordPress attachment
  ledger and verify two independent pen-down runs for **ح**.
- Preserve the clip's visible stem-first order: finish the short left descender,
  lift once, then restart near its top and sweep continuously around the dotless
  bowl instead of inheriting adjacent Jeem's motion.
- Keep two-way-connector context and Arabic-scoped Noto Naskh provenance while
  reducing the measured HL-C09 debt to 190 entries; Arabic has 16 outstanding.

### Added - source-verified Arabic independent jeem (HL-C09AL)

- Record HL-C09AK's source defect: the page's link labeled Thaa plays another
  two-dot Taa lesson, so **ث** remains conventional rather than gaining an
  unsupported third-dot order or lift count.
- Reprioritize to the next viable **ج** video and verify its independent body
  first: the short upper head travels left-to-right, continues down and around
  the bowl without lifting, then one lift precedes the dot below.
- Preserve two-way-connector context, Arabic provenance independently of Urdu's
  dot-first ج, and reduce the measured HL-C09 debt to 191 entries across seven
  scripts; Arabic remains the smallest inventory with 17 entries outstanding.

### Added - source-verified Arabic independent taa (HL-C09AJ)

- Verify the independent **ت** body from the University of Oregon page's Baa
  demonstration and its left-then-right upper dots from the dedicated Taa clip.
- Record the evidence split explicitly because Taa opens with the shared bowl
  already complete; preserve two separately lifted dot strokes, the lesson's
  two-way-connector context, and Arabic provenance independently of Persian ت.
- Reduce the measured HL-C09 debt to 192 entries across seven scripts; Arabic
  remains the smallest inventory with 18 entries outstanding.

### Added - source-verified Arabic independent baa (HL-C09AI)

- Verify independent **ب** from the University of Oregon's adjacent video as a
  continuous right-to-left bowl followed by one lift and the dot below.
- Preserve the lesson's two-way-connector context while attaching the exact
  citation and lift count to the canonical Arabic row, independently of the
  Persian record for the same Unicode glyph.
- Reduce the measured HL-C09 debt to 193 entries across seven scripts; Arabic
  remains the smallest inventory with 19 entries outstanding.

### Added - source-verified Arabic independent alif (HL-C09AH)

- Verify independent **ا** from the University of Oregon's *Introduction to
  Arabic* video as one continuous top-to-bottom stroke with zero lifts.
- Preserve the lesson's one-way-connector context while attaching the exact
  citation and lift count to the canonical Arabic row, independently of the
  Persian and Urdu records for the same Unicode glyph.
- Reduce the measured HL-C09 debt to 194 entries across seven scripts; Arabic
  remains the smallest inventory with 20 entries outstanding.

### Added - the irregular and stem-changing overlays (HL-C91)

HL10 section 5.1 sized the Spanish verb system at roughly 630 cells. HL-C82
shipped the 231 regular ones; this adds the **402 overlays**, for **633 total**.
The original estimate holds.

    stem-change  e-ie 36 · o-ue 32 · e-i 20 · u-ue 4
    strong preterite 90 · short stem 144 · irregular subjunctive 36
    irregular imperfect 18 · irregular participle 12 · go-club 10

**They are a separate list, and that is pedagogy rather than tidiness.** A
learner never meets "the irregular verbs" as a category. They meet the regular
row, and then the one verb that breaks it, in frequency order, one cell at a
time. So every overlay's prerequisite is the regular cell it deviates from:
`tengo` hangs off `ES-CELL-IND-PRES-1SG-CONJ2`. The DAG gains depth, nothing
becomes reachable earlier, and an irregular is always taught against a pattern
the learner already holds.

Three shapes are pinned by test because losing them would flatten the model back
into "this verb is irregular":

- **The boot.** A stem change covers the singular and the third plural only; the
  two plural persons keep the regular stem. Four cells per verb, not six.
- **One weld, twice.** A shortened future stem serves the conditional as well, so
  a verb like *tener* owns twelve cells but one thing to learn.
- **Three imperfects.** The entire language has three irregular imperfects, which
  is why HL10 places that tense immediately after the preterite as a rest.

The regular inventory is byte-identical to before -- verified against `HEAD`, not
asserted -- so every HL-C82 pin still measures what it measured.

`BUILD` already gates generator drift with `--check`; the generator now also
refuses any overlay whose `deviatesFrom` is not a real regular cell, and any
duplicate overlay id.

### Added - Urdu baṛī ye completes the starter ductus inventory (HL-C09AG)

- Verify independent **ے** from Northwestern's *Zer o Zabar* calligraphic and
  handwriting animations as one zero-lift folded bowl: upper-right descent,
  leftward sweep, far-left curl, then a rightward lower fold.
- Preserve the source's independent/final sound role and its distinct
  initial/medial be-series tooth while attaching the exact citation and lift
  count to the canonical Urdu script row.
- Reduce the measured HL-C09 debt to 195 entries across seven scripts; Urdu's
  thirteen-entry starter inventory is now fully source-verified.

### Fixed - a report that its own subject could edit (HL-C90)

Every gate in this package interpolates author-written strings into lines
written to stdout: lesson ids, node ids, root slugs, finding messages. A lesson
id carrying an ANSI escape rewrites its own line in a terminal, so a crafted id
could erase the very defect line a reviewer is reading to decide whether the
corpus is sound.

These reports exist to make problems visible. A report that can be edited by its
subject does not.

`stripControlCharacters` now guards **nineteen interpolations** across
`report.ts`, `strands.ts`, `grammar-cells.ts`, `root-ledger.ts`, `info-dump.ts`
and `metalanguage.ts` -- every place a corpus-derived string reaches a report
line. Control characters are removed rather than escaped: the reports
are read by humans, not parsed, so a visible `\u001b` adds noise without adding
information. Tab and newline survive -- they are ordinary layout, and the render
helpers control their own line breaks.

Found by the security review of HL-C80 and filed whole rather than fixed halfway
inside an unrelated PR, because the pattern was package-wide from the start.

The tests build their control characters with `String.fromCharCode` rather than
writing literals, for two reasons learned in this session: a literal ESC in a
source file is invisible to a reviewer, and this repository has already had
non-ASCII source literals silently mangled on write.

### Added - the metalanguage ramp (HL-C89)

The hidden prerequisite of every language textbook: it assumes the reader
already knows grammar *vocabulary*. "The first-person singular present
indicative of a regular -ar verb" spends six technical terms on one form, and a
beginner who never studied grammar understands none of them. The book is gentle
about Spanish and brutal about English, and nobody notices, because the author
has known those words since school.

`core/metalanguage.json` makes it a ramp: **54 terms**, each carrying the thing
the learner must already be able to DO before the term is named. `verb` arrives
once *soy*, *estoy* and one present form are in use. `mood` waits for block D of
the subjunctive arc, twenty-four lessons in.

**`plainAlternative` is the point.** A rule that only forbids is a rule authors
route around, so every term carries what a lesson must say *instead* until the
term is earned -- "a doing word" for verb, "the plain form, the one in a
dictionary" for infinitive, "whether you are asserting or wanting" for mood. The
gate can tell an author what to write, not merely what not to.

The first measurement: **2,289 technical uses across 1,161 lessons**, led by
`verb` (795 lessons), `noun` (398), `regular` (109), `tense` (109), `pronoun`
(102), `article` (91). Nothing anywhere introduces any of them.

**Two numbers, deliberately.** The raw total is 7,738, but `word` alone appears
in 1,555 lessons and needs no introduction at all. A measurement that does not
separate ordinary English from technical vocabulary produces one enormous number
that is identical for every corpus and useless to every author -- the same
cry-wolf failure the info-dump gate avoided by flagging shape rather than size.
So terms carry `technical`, the total says how pervasive the assumption is, and
the technical count says what to fix first. `noun`, `verb` and `adjective` count
as technical on purpose: the premise is a reader who never studied grammar, and
for them "a doing word" lands and "verb" does not.

### Changed - HL-C89's scope, corrected by measuring before building

HL10 section 7.4 also asks for a banned-word lint -- no *simply*, *just*,
*obviously*. Measured first, and it is nearly a no-op: a naive denylist flags
**535 of 1,694** lessons (`just` 359, `simply` 184), narrowing to genuinely
dismissive senses drops it to **23**, and reading those, most are still innocent
-- "*Desde luego* means 'of course'" is teaching the phrase, not talking down to
the reader. The corpus's prose is already kind. Building that half first would
have produced a gate nobody needed.


### Added - the info-dump gate, and what it found (HL-C84)

The owner's rule is one sentence: "will not info dump ever". This makes it a
measurement.

**The prose is not the problem.** Scanning all 1,694 lessons for rule-statement
shapes -- "X is used for...", "X always takes...", "there are four kinds of X" --
turns up **17 lessons**. Seventeen. HL09 called the writing "as well built as
anything commercial" and the corpus agrees.

**The info dump lives in tables**, and in one specific shape:

    70 lessons carry a paradigm-shaped table -- a grid whose first column walks
    a list of grammatical persons -- and 18 of those are FULL grids of five or
    more rows. FR-C05-parler, GE-C05-wohnen and ES-C17-practice each present a
    complete six-person conjugation at once.

That is exactly the artifact HL10 section 5.3 forbids: six new forms, one new
concept, no retrieval, and an implicit claim that the learner absorbs them by
staring. It is also the single most universal convention in language publishing,
which is why it needs a gate rather than a style note -- nobody writing one
thinks they are doing anything unusual.

**Shape, not size.** 470 tables in the corpus have three or more data rows and
most are perfectly good: a vocabulary recap, a regional comparison, a list of
labelled facts. Flagging all 470 would bury the 70 that matter and teach authors
that the gate cries wolf. The signal is a first column that walks a paradigm,
because that is a table presenting N grammar cells where the budget allows one.

`PERSON_LABELS` is a census of what the corpus's own tables put in that column,
per track, covering the six Latin-script tracks that use them today. A track
absent from the map is never flagged -- honest rather than silently clean, the
same rule `continuity.ts` uses for its article map.

Report-only, per the HL05 precedent. Its real value is as a review aid: a lesson
that trips it is not automatically wrong, but it is automatically read by a
human before merge.

### Fixed - two ways a lesson file could attack its own gate

Security review of the above, both verified by execution.

**A quadratic comment strip.** `replace(/<!--[\s\S]*?-->/g, "")` looks like the
safe construct and is not. With `/g` the engine retries at every `<!--`, and
when there is no closing `-->` each start expands one character at a time to
EOF before failing -- O(n squared) in the *count* of `<!--` tokens, with no
`-->` needed anywhere. Measured: 500 KB of repeated `<!--` took **13 seconds**,
and a 4 MB lesson would have pinned a core for roughly fifteen minutes. Now a
monotonic `indexOf` scan: the same input takes **22 ms**, and an unterminated
comment keeps the remainder of the file verbatim rather than swallowing it.

**A directory name resolving through `Object.prototype`.** `PERSON_LABELS` is a
plain object indexed by `lesson.language`, which `loader.ts` takes straight from
`readdirSync` -- so a track directory named `constructor`, `toString` or
`__proto__` resolved to an inherited member, passed the `undefined` check, and
threw on `.includes`. This package already exports `hasOwn` for exactly this and
uses it at five sites; `parse.ts` guards its own language lookup the same way
and `ramp.ts` documents this identical bug being fixed once before. The new
module simply skipped the convention.


### Added - the Root Ledger, and the account it renders (HL-C83)

HL00 calls the etymology "the heart of the lesson... the signature of this
curriculum", and it is genuinely the strongest thing in the corpus. But a root
is only *useful* if it is spent again, which is what HL10 section 6.2's
`rootLedgerMinReuse: 3` says: a root may be taught only if at least three LATER
lessons draw on it.

The first measurement, across both etymology namespaces:

    2,717 roots
    2,624 spent fewer than three times   (97%)
    1,807 never spent at ALL             (taught once, never returned to)

Spanish alone: 303 roots, 290 underspent, 190 never spent. The best-spent root
in the entire corpus is `LA-ETYMON-SALVE-02`, at eight payoffs.

The etymology is real, it is good, and almost none of it is being spent. That is
the difference between a curriculum whose vocabulary compounds and one where
every lesson starts over -- and it is the machinery the friends layer (HL10
section 6.7) needs, since a root with recorded payoffs already knows which later
words it predicts.

**An introduction is not a payoff.** A root named in exactly one lesson scores
zero, not one. Counting the introduction would have started every root at 1 and
flattered the corpus by exactly the number of roots it has.

**Both namespaces, deliberately.** The corpus records etymology twice --
cross-language `roots:` slugs (1,966) that let a Spanish root and an Italian one
be recognised as the same root, and `<LANG>-ETYMON-*` atoms (751) that
participate in prerequisites and reinforcement windows. A ledger over only one
would report a root unspent while the other namespace was quietly spending it.

### Fixed - three bugs found while building it

**The etymon namespace silently contributed zero.** The frontmatter keys are
flat and dotted -- `introduces.knowledge`, never a nested `introduces` object --
and reading them as nested returns `undefined` for every lesson in the corpus.
The ledger reported 1,966 roots instead of 2,717, which reads as "the corpus has
no etymon atoms" rather than "the reader is broken". `ramp.ts` already carried a
warning about this exact mistake, from when it made the chapter gates report all
279 authored chapters as broken; this module now uses that file's shared
`frontmatterList` rather than its own reader.

**A composite key that could merge two roots.** `${language} ${namespace}
${root}` lets `("es", "roots", "a b")` and `("es", "roots a", "b")` collide and
silently sum two roots' payoff counts. Now length-prefixed. No collision existed
in the current corpus -- the counts are unchanged -- but a root slug is
author-written and may contain anything.

**NUL bytes in a source file.** The spaces inside a template literal were
written to disk as U+0000: `${language}\0${namespace}`. The file still compiled,
`grep` silently found nothing in it, and an exact-match edit could not touch the
line. A NUL in source is always a write accident, never intent, so
`tests/root-ledger.test.ts` now asserts that no file in `src/` contains one --
cheaper to assert than to rediscover.


### Added - the grammar cell inventory (HL-C82)

A **cell** is one filled slot in one paradigm: Spanish `hablo` is a cell. The
six-form present-indicative table is not a teachable unit; it is six.

Every language textbook opens a tense with its full grid, and that grid is the
steepest single step in language pedagogy -- six new forms, one new concept, no
retrieval, and an implicit claim that the learner absorbs them by staring. HL10
forbids it twice: `maxNewGrammarCellsPerLesson: 1`, and no paradigm table until
every cell in it has been taught individually, at which point the table is a
recap rather than an introduction. A rule like that is only enforceable if the
cells are enumerated. Now they are.

Two files, because HL10 section 4 makes GRAMMAR a universal slot inventory with
local filling -- which is what lets the other 21 tracks reuse this:

- `core/grammar-slots.json` -- **231 language-neutral slots**. 144 finite, 30
  imperative, 48 compound, 9 non-finite. No id or gloss may name a form from any
  particular language, and a test enforces it. A new track answers "do you have
  this?" instead of designing a syllabus from nothing.
- `spanish/grammar-cells.json` -- Spanish's answers, with the ordering a learner
  climbs.

**`prerequisites` is what makes this a ramp rather than a list**, and each edge
is a pedagogical claim that can be argued with:

- Singular before plural, one person at a time. This is what turns "the present
  tense" from one chapter into fourteen.
- Conjugation 1 before 2 before 3, at the same person and tense.
- Tenses in acquisition order, anchored at the 1SG conjugation-1 cell only --
  anchoring every cell would say "learn all of the present before any of the
  preterite", forbidding the interleaving that makes the ramp gentle.
- The present subjunctive hangs off the present indicative 1SG, because that is
  where its stem comes from (`tengo` to `tenga`). Load-bearing, not decorative:
  it is why HL10 section 5.4 puts the -go verbs before the subjunctive arc.
- A negative command requires the affirmative command **and** the present
  subjunctive -- which is how the subjunctive first reaches a learner, before it
  is ever named.
- Compounds require the participle and the auxiliary's own finite cell. Nobody
  says "I have spoken" before they can say "I have".

The resulting graph has four roots -- the three infinitives and `hablo` -- and a
maximum depth of 15, the future subjunctive third person plural.

The 18 future-subjunctive cells are marked `productive: false` with a recorded
reason, so the gate can tell "not taught yet" from "deliberately recognised and
never produced".

`cellCoverage` measures the corpus and reports **0 of 231**. That zero is the
honest number and is deliberate: the alternative was inferring cells from atom
names, but `ES-GRAMMAR-AR-FUTURE-SINGULAR` is three cells only if the lesson
really taught three at once -- which is the exact thing being forbidden. A fuzzy
mapping would have credited coverage the corpus has not earned and quietly
legitimised the info dump. HL-C84 wires the declarations on.

The data is generated by `data/generate_grammar_cells.py`, which validates the
DAG (no dangling edge, no cycle) at generation time. `BUILD` runs it with
`--check`, so committed drift fails the build -- the same contract
`check:books` and `check:modality` already enforce.

### Fixed - an ordering check that graded file order instead of reading order

Found while testing the above. `sequence` arrives from the frontmatter parser as
a **string**, and the first draft of `sequenceOf` tested `typeof raw ===
"number"`. Every lesson therefore fell through to `Infinity`, the sort became a
no-op, and the out-of-order check silently graded lessons in whatever order the
array happened to hold -- passing on any fixture that was already sorted.

Now coerced the way `continuity.ts`'s `declaredSequence` already did it, with a
regression test asserting both directions: array order wrong but `sequence`
right must not fire, and array order right but `sequence` wrong must.

### Added - the strand dimension, and the three ladders nobody has climbed (HL-C80)

HL09 proved a course can be gentle on one ramp and brutal on another with
nobody noticing, because only the gentle ramp was counted. Spanish measured
178 headwords with every lesson inside the atom budget -- a textbook-perfect
vocabulary ramp -- while the learner still could not say "no", could not say
"I am", and met the entire past tense behind a spine node declaring one
concept.

The fix is not a bigger budget, it is more ladders. `core/spine.json` now
declares eight **strands** -- FUNCTION, GRAMMAR, LEXICON, SOUND, ETYMOLOGY,
CULTURE, IDIOM, TEXT -- and every one of its 33 nodes names exactly one. A new
`strands.ts` measures the distribution, and `report-cli` prints it.

The first snapshot is the reason the model was worth building:

    FUNCTION 14, GRAMMAR 7, LEXICON 2, SOUND 0,
    ETYMOLOGY 0, CULTURE 3, IDIOM 0, TEXT 7

**Three declared ladders have no nodes on them.** ETYMOLOGY is the sharpest:
HL00 calls it "the signature of this curriculum" and 708 lessons carry an
etymology hook, so the content is genuinely there -- as prose an author chose
to write, promised by no node and owed by no chapter. That is the difference
between a commitment and an intention, and it is exactly what a strand model
exists to expose. `summarizeStrands` seeds its counts from the DECLARED strand
list rather than from the nodes present, so an unclimbed ladder reports as a
zero instead of vanishing from the table.

`nodeSizeDefects` makes the HL09 section 1 defect checkable: a node is realized
by one to three chapters, so it may not declare more concepts than a chapter
may introduce. `SPINE-SAY-WHAT-I-DO` declares **42** against a ceiling of 12,
while `SPINE-TALK-ABOUT-PAST` declares one and stands for the entire past tense
of the language. Both cannot be one rung of the same ladder, and that asymmetry
is how a track claimed A2 on fourteen present-tense lessons. HL-C81 splits it;
until then the count is pinned so it cannot grow quietly.

`core/chapter-policy.json` gains the seven HL10 section 2.2 budgets, all
optional so a policy file written before them still loads. The consequential
one is `maxNewGrammarCellsPerLesson: 1` -- a *cell* is one filled slot in one
paradigm (`hablo`), not the six-form table, and Spanish holds roughly 630 verb
cells. `maxRuleStatementsPerLesson: 1` is the info-dump gate, and
`minDownstreamReach: 1` makes "every lesson leads to future lessons"
falsifiable by naming an introduced atom no later lesson ever uses.

Everything here is report-only, per the HL05 precedent: the corpus predates the
model, and a gate that fails on already-recorded debt teaches authors to route
around it rather than pay it down.

### Fixed - a strand gate that a crafted stage name could silence

Security review of the above, verified against the built module rather than
reasoned from the diff. `byStage` was built with `Object.fromEntries`, so it
inherited from `Object.prototype`, and membership was tested with `in`, which
walks the prototype chain. A node declaring `stage: "toString"` therefore passed
the check, read the inherited **function**, and `+= 1` wrote the string
`"function toString() { [native code] }1"` into the counts. That string is not
`=== 0`, so `missingStages` reported the stage as **covered**.

A gate whose whole job is making curriculum defects visible, reporting clean
*because of* a crafted stage name, is worse than no gate. Buckets are now
`Object.create(null)` with an own-property check.

Five malformed-JSON shapes also threw uncaught `TypeError`s out of the CLI --
`strands` as an object or string, `stages` as a string, `nodes` absent, `nodes`
holding `null` -- surfacing as Node stack traces with absolute filesystem paths
where `report-cli` otherwise catches and returns exit 2. `Array.isArray` guards
now match the shape validation `loadChapterPolicy` already performs.

Confirmed not exploitable and deliberately unchanged: `Object.fromEntries` uses
`CreateDataPropertyOrThrow`, so a `__proto__` key becomes an ordinary own
property and never a prototype write.

Specified in `code/specs/HL10-spanish-pre-a1-to-c2-course-architecture.md`.


### Added - Tamil chapter 39, and the first letter of the debt it exists to pay

`TA-W19` measured the strand out of room after itself, and three letters —
**ஏ**, **ஐ**, **ஒ** — were still used inside words and never taught. The track
was extended rather than relaxing the chapter atom cap, the 3:1 cadence or the
rule that a chapter must not open on a pen lesson. This is the first of three
chapters planned to do it.

One correction belongs here, because the decision was taken partly on it. The
position search that informed it reported ZERO admissible slots anywhere in the
track. That was wrong: it assigned a candidate insertion to the chapter of the
lesson it precedes rather than the one it follows, which silently discarded
every end-of-chapter position. Re-run correctly against `origin/main`, exactly
ONE slot exists — chapter 35, after `TA-C35-naarkaali`, gaps of 3 and 3, load
10 + 2 = 12. One slot is still not three, so extending the track was needed
either way for at least two of the three letters; but "no slot at all" was not
true, and chapters 40 and 41 should consider spending that slot before adding a
second new chapter.

- `TA-C39-vendum` (1170) — **வேண்டும்**, another verb Tamil builds with no
  subject, after **தெரியும்** (ch32), **புரிகிறது** (ch33) and **பிடிக்கும்**
  (ch34). The lesson deliberately does not number the family: chapter 19's
  **ஆகிறது** age construction is described in the same terms, so a count would
  have to argue for its own boundary. The lesson does not re-teach the
  dative-subject shape either; it practises chapter 6's
  `TA-GRAMMAR-DATIVE-SUBJECT-02` at a distance of 90 lessons, the first time
  anything has reached it at R4 range.
- `TA-C39-evvalavu` (1180) — **எவ்வளவு**, and the line Tamil draws that English
  does not: **எத்தனை** counts, **எவ்வளவு** measures. Age took the counting one.
- `TA-C39-oru` (1190, payoff) — **ஒரு** in front of a noun where **ஒன்று**
  cannot stand, and the chapter's production task: ask a price, order one tea,
  decline the second with **வேண்டாம்**.
- `TA-W20-read-onru` (1195) — **ஒ**, spelling **ஒன்று** and **ஒரு**. It is
  last in the chapter, so the opening reads "first 3 of 4 lessons".

### Added - the spine node this track had declared and never realized

`curriculum.json` already carried `SPINE-SAY-WHAT-I-WANT` with an empty
`segments` list and `VERB-WANT` sitting in `omits` — an authored admission that
the node was mapped but unmet. Chapter 39 realizes it through `TA-PATH-036`, so
the omission is removed rather than merely annotated. Three of the chapter's
lessons land at A2 as a result, because that node is an A2 node; only
`TA-W20-read-onru` is pre-A1, on `SPINE-MEET-GREET` like every other writing
lesson.

### Changed - what a new chapter costs, measured

Four wiring points are needed, and the first alone is not enough:
`chapters.json` (the capability ledger), `book-generation.json` (the target),
`tamil/book/book.tex` (the `\input`), and `curriculum.json` (path segment,
extension, spine segments). Declaring only the ledger fails the book-cli gate
"puts every ledgered chapter into its book, not merely into a file", which is
exactly the check that exists to catch this.

Pins re-derived by set difference against `origin/main`:

- `atomsTaught` 2652 -> 2660; `pre-A1` 878 -> 879 and `A2` 409 -> 412;
  ramp-to-A1 1187 -> 1188 with `TA-W20-read-onru` the only joiner; manifest
  `totalLessons` 1690 -> 1694, `chapterCount` 513 -> 514, `pen` 68 -> 69,
  `sight` 508 -> 511, `unstartableChapters` 137 -> 138.
- `missedByWindow.R2` 1808 -> 1816, and all eight entrants are one mechanism:
  the track grew 128 -> 132, so a window becomes evaluable for exactly those
  atoms whose `introducedAt + window.from` falls in (127, 131]. They are four
  two-atom pairs — VIDAI at 126, SUGAM at 125, UDAMBU at 124 and
  IVAR-EN-NANBAR at 123 — and not one of their revisit counts changed.
  R4 243 -> 247 is the same arithmetic at 80. R3 does not move at all —
  1309 -> 1309, seven in and seven out — which is the whole argument for
  declaring what a sentence
  actually re-uses. `TA-C39-vendum` names **தெரியும்**, **புரிகிறது** and
  **பிடிக்கும்** in one clause and credits all six of their atoms: the two
  `PIDI` atoms (index 108) land a revisit at exactly distance 20, R3's first
  position, so neither enters, while `TA-LEX-PURI-01`, `TA-GRAMMAR-PURI-02`
  (index 100) and `TA-GRAMMAR-TERI-02` (index 98) leave R3 outright and drop off
  the defect list. The same clause with the verbs merely named would have read
  identically on the page and left R3 five windows worse.
- Against that, ten atoms LEAVE a window, and those are the chapter earning its
  keep: `TA-SCRIPT-EE-SIGN-01` 1 -> 2 revisits, `INDEPENDENT-VOWEL-E-01` 2 -> 3,
  `NGA-LLA-01` 2 -> 3, `TTA-01` 1 -> 2, the two `PURI` atoms and
  `TA-GRAMMAR-TERI-02` out of R3 (seven in all);
  `GRAMMAR-DATIVE-SUBJECT-02` 2 -> 3, `LEX-DATIVE-SUBJECT-01` 3 -> 4 and
  `LEX-NUMBERS-1-5-01` 2 -> 3 out of R4.
- `atomsNeverRevisited` 472 -> 474, five in and three out. IN are
  `TA-GRAMMAR-EVVALAVU-VS-ETHANAI-02`, `TA-LEX-ORU-01`,
  `TA-GRAMMAR-ORU-ATTRIBUTIVE-02` and TA-W20's own `TA-SCRIPT-O-VOWEL-01` and
  `TA-SCRIPT-READ-ONRU-02`; OUT are `TA-SCRIPT-READ-MUUNRU-02`,
  `TA-GRAMMAR-PIDI-02` and `TA-SCRIPT-UU-SIGN-01`, each 0 -> 1 revisits. The
  last is TA-W19's own sign, credited where TA-W20 contrasts **மூன்று** with
  **ஒன்று**. The 422-atom defect
  subset moves separately, 422 -> 424; the two counters are worth keeping apart.
  The `ORU` pair being among the entrants is structural, not
  an oversight: `TA-W20` genuinely re-reads **ஒரு**, but a writing lesson may
  only take other writing lessons as prerequisites — `TA-EXT-003-SCRIPT` is
  inlined at `TA-PATH-003`, so naming a chapter-39 lesson would place the
  prerequisite after its dependent and fail the ordering rule. The tie is
  carried by `reviews_of`, which is not a revisit. Chapters 40 and 41 are
  planned to close it.
- `forwardReferences` 423 -> 424, and the new entry is a measurement
  improvement rather than fresh damage: `TA-C18-mani-homophone-time` has always
  printed **ஒரு**, but no lesson owned the word, so the checker had no teacher
  to measure against. Naming one made a 65-lesson-old early use visible. It is
  also an argument that **ஒரு** belongs earlier than chapter 39, which the
  runway did not allow.

### Added - the ூ sign, and the measured end of the Tamil strand's runway

- Teach `TA-W19-read-muunru` (chapter 38, sequence 1165) around **மூன்று**: the
  long-*ū* sign **ூ**, completing a four-corner *u*-family that took three
  lessons to open — **ு** in `TA-W13` (chapter 31), **உ** in `TA-W17` (36) and
  **ஊ** in `TA-W18` (37) — short/long by independent letter versus consonant
  sign.
- Choose **ூ** by census, not by convenience: it is the highest-usage glyph the
  writing strand never taught, appearing in five lessons.
- Credit `TA-SCRIPT-READ-UUR-02` and `TA-SCRIPT-THREE-NS-01` where the lesson
  genuinely re-reads **ஊர்** and **ன**, rather than only declaring the atoms it
  introduces. Four atoms leave a reinforcement window in total; these two
  credits are what earns two of them. Measured by removing just these two
  credits and re-running: R2 goes back 1808 -> 1809 and R4 243 -> 244, while
  `TA-SCRIPT-U-VOWEL-01` and `TA-SCRIPT-UU-VOWEL-01` leave either way, carried
  by the Warm-up block.
- Say what the word buys. With **ூ** taught, six of the numbers one to ten are
  spellable entirely from glyphs the strand has given: **மூன்று**, **இரண்டு**,
  **நான்கு**, **ஆறு**, **எட்டு**, **பத்து**. The four that are not —
  **ஒன்று**, **ஐந்து**, **ஏழு**, **ஒன்பது** — are blocked on **ஒ**, **ஐ** and
  **ஏ** exactly.

### Measured - the strand cannot finish inside 38 chapters

The queue item that produced this lesson assumed the remaining glyphs were a
matter of authoring more lessons. Measuring the track says otherwise:

- After `TA-W18` (chapter 37) only five speaking lessons remain in the corpus,
  at sequences 1120, 1130, 1140, 1150 and 1160. There is room for one more
  writing lesson and no more, whether it is placed among them or after them.
- Chapter 38's atom load goes 6 -> 8, inside the twelve-atom chapter budget, and
  the lesson introduces 2 atoms, inside the three-atom lesson cap.

So the residue stands at thirteen glyphs in chapter 7's numbers alone — **ஏ**,
**ஐ**, **ஒ** and the ten Tamil digits **௧**-**௰** — with no slot left for any of
them. Closing that debt requires a decision this changelog does not make: extend
the Tamil track past chapter 38, or raise the strand's cadence.

A note on the census, because this entry quotes no total for it and earlier
drafts did. The absolute count of used-but-untaught glyphs is entirely a
function of how "taught" is detected, and small choices swing it by several
glyphs: whether a bold span of four code points such as **ஸ்ரீ** counts as
teaching **ஸ**, how far a negation such as "still wait on letters this book has
not taught" scopes, whether the `TA-C*` lessons may teach as well as use. The
figure of 19 written in `continuity.test.ts` during an earlier change does
reproduce, but only under one particular set of those choices — it needs **ஞ**,
**ஸ**, **ஃ** and **ஷ** to count as NOT taught. Within the writing strand those
four occur only in passing: **ஷ** and **ஸ** in `TA-W03`'s borrowed-ligature
aside (**க்ஷ**, **ஸ்ரீ**) and its Wrap-up answer, **ஃ** in the same lesson's
character-count mention, **ஞ** in `TA-W07`'s sound-table cell **ஞ்ச**. Two of
the four also appear in speaking lessons — **ஞ** in **ஞாயிறு**, **மஞ்சள்** and
**தஞ்சாவூர்**, **ஸ** in **நமஸ்காரம்** — which are uses, not teaching, under
either detector; **ஃ** and **ஷ** appear nowhere in the corpus but that one
`TA-W03` aside.
The detector used here counts all four as taught. Neither reading is wrong; the
number is simply not portable, which is why this entry quotes no total of its
own.

The two facts this entry rests on hold under a detector that does two specific
things, and it is worth naming them rather than claiming detector-independence:
it must scope negation, and it must not treat a `TA-C*` lesson as teaching. Both
matter, and this very lesson is why the first one does — it prints the numbers
it cannot yet spell in bold inside a sentence saying they wait on letters the
book has not taught, so a detector that ignores negation scores those letters
as taught here and puts this lesson's delta at four glyphs instead of one.
(The chapter-39 entry above narrows that sentence, once **ஒ** is taught.)
Chapter 7's own lessons bold the same letters while merely using them, which is
why the second matters. Under a detector that does both: the difference **this**
lesson makes is exactly **ூ**, and the thirteen chapter-7 glyphs named above are
untaught.

This supersedes, rather than continues, the census table in the "last two
glyphs" entry below. That entry's detector scored **ஞ**, **ஸ**, **ஃ** and **ஷ**
as untaught, which is where its 19 came from; the detector described here scores
all four as taught. Both entries then say "thirteen of chapter 7's", and they do
not mean the same thirteen: the earlier list is **ஐ**, **ஒ**, **ூ** plus the ten
digits, this one is **ஏ**, **ஐ**, **ஒ** plus the ten digits. **ூ** moved out of
the untaught set, which is precisely what this lesson did; **ஏ** was in it all
along and the earlier list omitted it. That table is left as written, as a
record of what was measured then.

### Changed - place the writing lesson LAST in its chapter, and measure why

The lesson was first written at sequence 1145, between `TA-C38-udambu` and
`TA-C38-sugam`. Measuring that placement against the alternative changed it, and
both effects are worth recording because neither is visible in a total:

- Hands-free start. `chapter-modalities.tex` for chapter 38 went from "all 3
  lessons" to "first 1 of 4": a `pen` lesson in position 2 truncates the core
  drivable prefix, and two speaking lessons lost hands-free reachability.
  Placed last it reads "first 3 of 4" and nothing is lost. The manifest summary
  cannot see this — its per-chapter `drivablePrefix` for chapter 38 is 0 either
  way — so no pinned test would have caught it.
- Reinforcement distance. At index 127 the lesson sits 6 lessons after `TA-W18`
  and 10 after `TA-W17`, both inside R2's 5-15 span. At index 125 it sat 4 after
  `TA-W18` — past R1's 1-3 and short of R2's 5, in the dead zone between them.
  The same lesson practising the same atoms therefore rescues three atoms from
  R2 rather than one.
- Placing a writing lesson last in its chapter is also the corpus's dominant
  pattern: eleven chapters already do it. The gap from `TA-W18` becomes five
  speaking lessons rather than three, which the strand already varies (existing
  gaps include six and nine).

Being last, the lesson carries no `Next:` line. That is not a special terminal
convention — 84 of the Tamil track's 128 lessons carry no `Next:` line — it is
simply that there is no successor left to name. `TA-C38-vidai`, which the move
displaces from last position, has no `Next:` line either — that much predates
this change — but it now has a successor it could name and does not, and that
much is created by this change. It is left alone: of the 84, all but TA-W19 have
a successor, and only a handful gesture at it at all — `TA-C01-practice` and
`TA-C02-practice` with a "Next chapter:" teaser, `TA-C04-po` in running prose.
So adding a teaser to `TA-C38-vidai` would be an isolated exception rather than
a convention.

### Changed - corpus pins re-derived by measurement

Every moved pin was re-derived as a set difference against `origin/main`, and
the direction of each mover is recorded at the assertion:

- `atomsTaught` 2650 -> 2652; `pre-A1` 877 -> 878; ramp-to-A1 1186 -> 1187;
  manifest `totalLessons` 1689 -> 1690 and `pen` 67 -> 68. The `pen` derivation
  comes from `writing-type` before the script block is considered, matching the
  `["writing-type","script-block"]` pair that 20 other Tamil lessons already
  carry, so the sight seam does not move.
- `atomsNeverRevisited` holds at 472, as does the 422-atom subset of it that
  also misses a window. Both are trades rather than washes, and they trade
  DIFFERENT atoms, which is worth separating because one set is a superset of
  the other. Both lose the same two: `TA-SCRIPT-READ-UUR-02` and
  `TA-SCRIPT-UU-VOWEL-01`, rescued from zero revisits by the re-reading above.
  The 472 set gains `TA-SCRIPT-UU-SIGN-01` and `TA-SCRIPT-READ-MUUNRU-02`, this
  lesson's own two atoms, never revisited because nothing follows them. The 422
  subset gains `TA-ETYMON-VIDAI-02` and `TA-LEX-VIDAI-01` instead — those two
  were already never-revisited at baseline and merely became window-measurable,
  the artifact described next, while TA-W19's own atoms miss the subset because
  at index 127 no window is evaluable for them at all.
- `missedByWindow.R2` 1809 -> 1808, and it goes DOWN. Three atoms leave —
  `TA-SCRIPT-READ-UUR-02` (revisits 0 -> 1), `TA-SCRIPT-UU-VOWEL-01` (0 -> 1)
  and `TA-SCRIPT-U-VOWEL-01` (1 -> 2).
- `missedByWindow.R4` 242 -> 243, with `TA-SCRIPT-THREE-NS-01` leaving
  (revisits 4 -> 5, missing R1/R2/R4 -> R1/R2).
- Every atom that ENTERS a window does so by one mechanism, and the arithmetic
  is exact in all four: the Tamil track was 127 lessons, and for each entrant
  `introducedAt + window.from = 127`, so that window's first position did not
  exist until this lesson made index 127 exist. R1 886, VIDAI pair at 126
  (126 + 1); R2, IVAR pair at 122 (122 + 5); R3 1307 -> 1309, UTAVU pair at 107
  (107 + 20); R4, PLEASE-REGISTER pair at 47 (47 + 80).
- For every one of those entrants the revisit COUNT is identical before and
  after, which is the check that separates an artifact from a regression: no
  existing reinforcement was broken by the insertion. TA-W19's own two atoms
  appear in no window at all, because at index 127 none is evaluable.

### Added - verified Urdu independent ں handwriting

- Record Northwestern's *Zer o Zabar* independent nūn-e ġhunna animations as
  one right-to-left bowl below the baseline with zero pen lifts.
- Preserve the source's final/independent dotless-nūn distinction and its
  ordinary-nūn initial/medial forms; Noto Naskh confirms U+06BA exactly shares
  U+0646's body contour with the dot removed.
- Raise cited handwriting coverage to 32 of 228 prose entries, leaving 196
  explicit unverified part orders and 1 in the Urdu starter inventory.

### Added - verified Urdu independent ی handwriting

- Record Northwestern's *Zer o Zabar* independent chhoṭī ye animations as one
  dotless S-shaped sweep from the upper right through the below-baseline bowl
  to its rising left tip, without lifting.
- Preserve the source's positional distinction: the two dots belong to initial
  and medial ye, not the independent form, while the checked learner path fits
  the canonical Noto Naskh fallback.
- Raise cited handwriting coverage to 31 of 228 prose entries, leaving 197
  explicit unverified part orders and 2 in the Urdu starter inventory.

### Added - verified Urdu independent ہ handwriting

- Record Northwestern's *Zer o Zabar* independent animations as one
  counterclockwise teardrop loop from the upper right, around the base, and
  back up to cross at the top without lifting.
- Preserve the chapter's oval-or-teardrop description and its distinct
  initial, medial, and final forms while fitting the independent zero-lift
  motion to the canonical Noto Naskh fallback.
- Raise cited handwriting coverage to 30 of 228 prose entries, leaving 198
  explicit unverified part orders and 3 in the Urdu starter inventory.

### Added - verified Urdu independent ن handwriting

- Record Northwestern's *Zer o Zabar* independent animations as a
  below-baseline right-to-left bowl followed by one lift and its dot.
- Preserve the chapter's near-baseline dot placement and the distinct
  initial/medial tooth form while fitting the independent order to Noto Naskh.
- Raise cited handwriting coverage to 29 of 228 prose entries, leaving 199
  explicit unverified part orders and 4 in the Urdu starter inventory.

### Added - verified Urdu independent م handwriting

- Record Northwestern's *Zer o Zabar* independent animations as one unbroken
  head-to-tail stroke whose tail drops below the baseline.
- Preserve the chapter's handwritten counterclockwise-loop guidance and its
  calligraphy contrast while fitting the shared zero-lift order to the
  canonical Noto Naskh fallback.
- Raise cited handwriting coverage to 28 of 228 prose entries, leaving 200
  explicit unverified part orders and 5 in the Urdu starter inventory.

### Added - verified Urdu independent ل handwriting

- Record Northwestern's *Zer o Zabar* independent animations as one unbroken
  top-down upright that continues below the baseline through the leftward bowl
  and turns back up without lifting.
- Preserve the chapter's connector and final-bowl distinctions while fitting
  the independent zero-lift order to the canonical Noto Naskh fallback.
- Raise cited handwriting coverage to 27 of 228 prose entries, leaving 201
  explicit unverified part orders and 6 in the Urdu starter inventory.

### Added - verified Urdu independent ک handwriting

- Record Northwestern's *Zer o Zabar* independent animations and prose as a
  continuous main-line stem, flatter bowl, and pronounced final hook followed
  by one lift and the long upper-right downward slash.
- Preserve the source's explicit warning not to write kāf in one penstroke and
  fit its two-stroke order to the canonical Noto Naskh fallback.
- Raise cited handwriting coverage to 26 of 228 prose entries, leaving 202
  explicit unverified part orders and 7 in the Urdu starter inventory.

### Added - verified Urdu independent ش handwriting

- Record Northwestern's *Zer o Zabar* independent animations as the complete
  toothed sīn body followed by lower-left, lower-right, and centered upper dot
  strokes, for four strokes and three verified pen lifts.
- Preserve the chapter's two-below/one-above dot arrangement, centered dots,
  and optional toothless long-curve body while fitting the standard learner
  path to the canonical Noto Naskh fallback.
- Raise cited handwriting coverage to 25 of 228 prose entries, leaving 203
  explicit unverified part orders and 8 in the Urdu starter inventory.

### Added - verified Urdu independent س handwriting

- Record Northwestern's *Zer o Zabar* independent calligraphic and handwriting
  animations as one right-to-left run through three close teeth and the final
  bowl, without a pen lift.
- Preserve the chapter's optional long gentle curve in place of the teeth as an
  especially common handwriting alternative while fitting the standard toothed
  learner path to the canonical Noto Naskh fallback.
- Raise cited handwriting coverage to 24 of 228 prose entries, leaving 204
  explicit unverified part orders and 9 in the Urdu starter inventory.

### Added - verified Urdu independent ر handwriting

- Record Northwestern's *Zer o Zabar* independent-form animation and prose as
  one downward line that continues curving left without a pen lift.
- Keep the chapter's distinct final-form motion and its Naskh/Nastaliq contrast
  explicit while fitting the independent path to the canonical Noto Naskh
  fallback.
- Raise cited handwriting coverage to 23 of 228 prose entries, leaving 205
  explicit unverified part orders and 10 in the Urdu starter inventory.

### Added - verified Urdu independent ج handwriting

- Record Northwestern's *Zer o Zabar* independent-form animation with its dot
  first, one lift, then a continuous pointed hooked head, descent, and bowl.
- Preserve the chapter's flat-head form as a purely aesthetic alternative and
  fit the verified pointed learner path to the canonical Noto Naskh fallback.
- Raise cited handwriting coverage to 22 of 228 prose entries, leaving 206
  explicit unverified part orders and 11 in the Urdu starter inventory.

### Added - verified Urdu independent ا handwriting

- Record Northwestern's *Zer o Zabar* animation for independent ا as one
  continuous top-to-bottom stroke with zero lifts, explicitly distinct from the
  lesson's bottom-to-top final form.
- Preserve Urdu-specific provenance and the canonical Noto Naskh fallback while
  Language Ladder gives shared Persian and Urdu glyphs collision-safe identities.
- Replace the stale pre-HL-U01 font note: Naskh remains this script-data file's
  path-checking fallback, while the book and app already use vendored Nastaliq.

### Added - verified Persian ه handwriting

- Record UT Austin Persian Online's 02:47–02:50 freehand demonstration for
  isolated ه as one simple closed handwritten loop with zero lifts.
- Preserve that one pen-down run while fitting Language Ladder's path to the
  vendored Noto Naskh form's two counters and leftward baseline finish.

### Added - verified Persian و handwriting

- Correct the source-sequence audit: UT Austin Persian Online demonstrates و,
  not ه, immediately after ن at 02:43–02:45.
- Record its isolated small head loop and leftward curving tail as one
  continuous Naskh stroke with zero lifts, matching Language Ladder's
  font-checked two-movement path and the vendored Noto Naskh outline.

### Added - verified Persian ن handwriting

- Record UT Austin Persian Online's 02:37–02:43 freehand demonstration for
  isolated ن: one continuous right-to-left Naskh bowl, then one lift to place
  the dot above.
- Match the two learner movements to Language Ladder's font-checked strokes and
  the vendored Noto Naskh outline.

### Added - verified Persian م handwriting

- Record UT Austin Persian Online's 02:33–02:36 freehand demonstration for
  isolated م: one continuous Naskh movement shapes the round head and flows
  directly into the descending tail with no pen lift.
- Match the two-part learner prose to Language Ladder's font-checked unbroken
  path and the vendored Noto Naskh outline.

### Added - verified Persian ل handwriting

- Record UT Austin Persian Online's 02:29–02:32 freehand demonstration for
  isolated ل: one continuous Naskh movement descends the tall upright and turns
  directly into the leftward base curve with no pen lift.
- Match the two-part learner prose to Language Ladder's font-checked unbroken
  path and the vendored Noto Naskh outline.

### Added - verified Persian س handwriting

- Record UT Austin Persian Online's 01:29–01:35 freehand demonstration for
  isolated س: one continuous right-to-left Naskh movement forms the three teeth
  and flows directly into the final bowl with no pen lift.
- Match the two-part learner prose to Language Ladder's font-checked unbroken
  path and the vendored Noto Naskh outline.

### Added - verified Persian ت handwriting

- Record UT Austin Persian Online's 00:22–00:27 freehand demonstration for
  isolated ت: one right-to-left Naskh bowl, one lift to the left dot above, and
  another lift to the right dot, matching Language Ladder's three-stroke path.
- Preserve the intervening Persian-added پ row as deferred inventory context
  instead of silently expanding HL-C09's fixed prose-entry denominator.

### Added - verified Persian ب handwriting

- Record UT Austin Persian Online's adjacent freehand demonstration for isolated
  ب as a shallow right-to-left Naskh bowl followed by one lift and the separate
  dot below, matching Language Ladder's font-checked two-stroke path.

### Added - verified Persian ا handwriting

- Record UT Austin Persian Online's opening freehand demonstration for isolated
  ا as one top-to-bottom Naskh movement with zero lifts, matching Language
  Ladder's font-checked path and the vendored Noto Naskh outline.

### Added - verified Tamil அ handwriting

- Record the cited five-movement, two-stroke order for Tamil அ with exactly one
  pen lift before its separate right upright, matching Language Ladder's
  font-checked ductus and the UT Austin primer's Frame 4.

### Added - verified Tamil ஆ handwriting

- Record the cited six-movement order for Tamil ஆ: one lift separates the
  shared அ-shaped body from a second run whose upright flows into the long-vowel
  loop without another lift.

### Added - verified Tamil இ handwriting

- Record the cited seven-movement order for Tamil இ: five joined movements form
  its inner curl and lower loops, then one lift precedes a second run joining the
  outer-left climb to the final arch.

### Added - verified Tamil க handwriting

- Record the cited six-movement order for Tamil க: its upper frame and two lower
  bowls form three pen-down runs with exactly two lifts, matching the Frame 3
  source and Language Ladder's font-checked path.

### Added - verified Tamil வ handwriting

- Record the cited five-movement order for Tamil வ: its spiral body, bottom bar,
  and right upright form one unbroken pen-down run with zero lifts, matching the
  Frame 9 source and Language Ladder's font-checked path.

### Added - verified Tamil ல handwriting

- Record the cited four-movement order for Tamil ல: its outward spiral, middle
  descent, deep right-hand turn, and open tip form one unbroken pen-down run
  with zero lifts, matching the next row of Frame 9 and Language Ladder's
  font-checked path.

### Added - verified Tamil ற handwriting

- Record the cited five-movement order for Tamil ற as three pen-down runs: its
  left arch joins the first middle descent, the adjacent descent restarts after
  one lift, and a second lift precedes the right arch's joined below-baseline
  sweep and descender, matching Frame 10 and Language Ladder's font-checked path.

### Added - verified Tamil ன handwriting

- Record Frame 13's cited six-movement order for Tamil ன as two pen-down runs:
  the left spiral, single inner arch, and top bar stay connected through five
  movements, then one lift precedes the separate right upright, matching
  Language Ladder's font-checked path.

### Added - verified Tamil ண handwriting

- Record Frame 13's cited seven-movement order for Tamil ண as two pen-down
  runs: the left spiral, both inner arches, and top bar stay connected through
  six movements, then one lift precedes the separate right upright, matching
  Language Ladder's font-checked path.

### Added - verified Tamil ந handwriting

- Record Frame 12's cited six-movement order for Tamil ந as three pen-down
  runs adapted to the vendored Noto form: three opening movements stay joined,
  one lift precedes the middle rise and top bar, and a second precedes the
  right-hand descent and tail.

### Added - generated lesson figures

- Generate deterministic etymology-route SVGs from the canonical lesson `roots`
  array through `paint-vm-svg`, with a checked manifest that fingerprints both the
  figure-driving source data and committed SVG bytes.
- Render safe relative Markdown images in generated LaTeX chapters, rewrite SVG
  destinations to their preconverted PDF counterparts, and reject traversal,
  remote, absolute, or unsupported image targets.
- Ship the first shared book/app figure for Spanish *café*, tracing Arabic
  *qahwah* through Turkish *kahve* and Italian *caffè*.

### Added - productive pattern lessons

- Parse ordered `slots` into the canonical lesson AST so books and apps consume one
  productive-frame contract, and recognize `pattern` as a non-lexical lesson type.
- Make the three HL05 pattern gates enforce exactly one introduced pattern atom,
  non-empty in-closure filler lists, and at least three distinct guided productions.
- Promote Spanish `ES-C17-comer-futuro` as the first canonical pattern lesson over
  already-known *comer*, *beber*, and *café*.

### Added - chapter modality signs in every book

- Generate and byte-gate one chapter-modality projection for each of the 22 books,
  using font-independent car, eye, and pen signs plus full printed-lesson counts and
  the core-derived hands-free starting prefix.
- Print the projection immediately after all 513 numbered chapter titles, including
  the 93 protected handwritten openings, without deriving a second modality model or
  overwriting authored chapter prose.

### Changed — one authority for chapter titles and labels

- Derive generated and handwritten book chapter titles and labels from each
  track's canonical `chapters.json` capability ledger. The generation manifest
  now owns only chapter coordinates, output paths, and rendering options.
- Reject legacy duplicate metadata and fail closed when a book declaration has
  no capability entry, while retaining the corpus-wide title-drift gate against
  the committed LaTeX chapters.

### Added - generated book subject indexes

- Build an English-first index from canonical word and phrase meanings,
  dedicated grammar/script/etymology/culture/pronunciation lessons, and chapter
  capability titles without mining prose or indexing practice drills.
- Byte-gate and include the generated index in all 22 books, preserving target
  script fonts, explicit lesson-focus facets, and linked chapter page references.

### Added - generated book review questions and answer keys

- Render every executable `hl-activity` contract as a numbered review question
  and answer-key entry, preserving the canonical display answer and all authored
  accepted variants used by Language Ladder.
- Byte-gate and include the generated back matter in all 22 books. French and
  Bengali gain their first typed activity contracts so no volume ships an empty
  key; legacy `[YOU ...]` delivery cues remain deliberately unscored.

### Added — generated book glossaries

- Derive a compact glossary from every track's canonical word and phrase lessons,
  with non-redundant romanization, distinct senses, and the chapter where each
  entry first appears.
- Byte-gate and include the generated back matter in all 22 books, using each
  volume's configured script fonts and page-safe record layout.

### Added — canonical pronunciation back matter

- Render canonical pronunciation-reference Markdown into LaTeX back matter,
  including headings, ordered and unordered lists, tables, citations, and each
  track's configured script font commands.
- Byte-gate the new Chinese, Japanese, Persian, Russian, and Urdu appendices with
  `book-cli --check`, bringing pronunciation back matter to all 22 books.

### Fixed — clean development dependency audit

- Refresh transitive Nano ID from 3.3.16 to 3.3.18 and PostCSS from 8.5.19 to
  8.5.26, clearing GHSA-2v37-7h3g-55p8 and GHSA-fxqj-rqcc-2cmp without changing
  the direct dependency range or runtime curriculum code.

### Added — generated top-level track progress

- Derive every Human Languages index row from `core/languages.json`, canonical
  lessons, realization maps, authored book chapters, and the generated-book hash
  manifest instead of repeating hand-maintained chapter and lesson claims.
- Add `generate:progress` and the byte-for-byte `check:progress` publication gate;
  a new registered language appears even when it has no lessons or book yet.

### Added — உ and ஊ, and two findings that stopped the next tranche

The plan after chapters 4-5 was to keep going: 22 lessons in the generated chapters
33-38 still carry a `## The letters in this word` section, and the obvious next step was
to strip them the way chapters 2-5 were stripped. Measuring first says otherwise, and
the measurement is the substance of this entry.

**Chapters 33-38 are not the same problem.** Running the strict taught-test over all 22
sections: **every glyph in them is already taught by the strand, except two** — **உ**
and **ஊ**. That is the opposite of chapters 2-5, where the sections were the *only*
place their glyphs were explained. Most of these sections review letters the reader has
already been taught, thirty-odd chapters into a track whose script strand starts at
chapter 4.

With one exception this tranche does **not** fix. `TA-C34-utavu` (sequence 1000) says
"**உ** is the standing vowel *u*, used when a word begins with it" — introducing the
glyph 85 sequence units *ahead* of `TA-W17`, and `TA-C36-unavu` and `TA-C37-uur` do the
same a little later. The strand cannot get there earlier without spelling a word the
learner has not been given, which is the rule it exists to keep. Nothing detects this,
either: those sections declare no atom for **உ**, so `forwardReferences` holding at 423
is not evidence against it.

**They also already cost the speaking learner nothing.** All 22 record
`detachableSegments: ["The letters in this word"]` with `coreModality: voice` and
`coreDrivable: true` in the generated manifest. The spoken-only edition those markers
exist for can already drop them. They are 22 of the 414 lessons corpus-wide where
`drivable` and `coreDrivable` disagree — a small share of a seam that exists across many
tracks, and working as designed in all of them.

**What they carry is mostly reinforcement, and one genuinely new thing.** An earlier
draft of this entry claimed the strand does not teach the no-fusion rule or positional
softening. That was wrong, and the fix below proves it wrong: `TA-W03-pulli-vanakkam`
has a whole "What Tamil does *not* do" section with the Devanagari contrast
(`TA-SCRIPT-PULLI-VANAKKAM-02`), and `TA-W01-abugida-va-ka` states the positional rule
outright (`TA-SOUND-ABUGIDA-VA-KA-04`). The chapter 33-38 sections *declare* those very
atoms in their own `assesses` lists — the corpus already says they are reinforcing the
strand, not replacing it.

What they add is thinner than that, and worth stating exactly. **ட**'s softening is
already in the strand too — `TA-W12-read-eppadi` says it outright, and both chapter 33-34
lessons that repeat it come later. What is genuinely new is **த**'s softening (the strand
teaches **த** in `TA-W16` without it), **ற்கா** as a second place two consonants refuse
to fuse, and one framing the strand does not have anywhere: `TA-C36-paal`'s minimal pair,
that a single dot is the whole difference between **பால்** and a non-word.

So the case for keeping them is narrower than the first draft claimed, but it holds:
they cost the spoken edition nothing, they reinforce atoms they correctly declare, and
they extend rules to letters the strand introduces later. Deleting all 22 would move
`sight` down by 22 and `voice` up by 22 — which reads as progress on the headline
modality numbers while losing that. That is the "a net that matches does not mean the
story is right" trap this changelog keeps recording, so the sections stay and this entry
records what was measured instead.

What was genuinely wrong is smaller, and is fixed here.

- **`TA-W17-read-unavu`** (chapter 36) — **உ**, the standing short *u*, spelling
  **உணவு**. The lesson's whole point is that the word holds the same vowel twice in its
  two forms: the letter that opens a word, and the **ு** sign that rides **வ**.
- **`TA-W18-read-uur`** (chapter 37) — **ஊ**, its long partner, spelling **ஊர்**.

**The second finding: those were the last two glyphs *in chapters 33-38*, not in the
corpus.** A census of every Tamil codepoint in every Tamil lesson against the strand's
taught set leaves **19** glyphs still used but never taught, and the cluster is nowhere
near chapter 33:

| glyph | lessons using it | earliest |
|---|---|---|
| **ூ** (long-*ū* sign) | 5 | ch7, **மூன்று** |
| **ஏ** | 4 | ch5 |
| **ஞ** | 4 | ch10, **ஞாயிறு** |
| **ஐ**, **ஒ** | 3 each | ch7, **ஐந்து** / **ஒன்று** |
| **ொ** | 2 | ch8 |
| **ஸ** | 2 | ch1 |
| **ஃ**, **ஷ** | 1 each | ch7 |
| Tamil digits **௧**-**௰** | 1 each | ch7 |

Chapter 7's numbers alone account for thirteen of them. That, not chapters 33-38, is
where the strand's remaining debt actually is.

Neither **உ** nor **ஊ** has an entry in `tamil.json`, so neither gets a stroke order,
and the note that the shorter letter sits inside the longer one is marked as what the
page shows rather than something the data states.

Both new lessons are placed **after** the speaking lesson that teaches their word — `TA-C36-unavu`
at sequence 1080, `TA-C37-uur` at 1100 — because spelling a word the learner has not yet
been given inverts the strand's own rule. A first draft put a single combined lesson at
1055, which would have spelled both words before either was spoken and pointed
`reviews_of` at a lesson 45 sequence units ahead; `forwardReviews` caught the second
half of that. Splitting in two costs one six-lesson gap in the 3:1 cadence, which is
what waiting for the words costs.

### Fixed — three stale chapter references left by the speaking-first restructure

Moving the script strand out of chapter 1 left three lessons pointing at teaching that
is no longer there. All three are in chapter 33 and all three were silently wrong:

| lesson | said | actually |
|---|---|---|
| `TA-C33-ninai` | "Chapter 1 mapped all three — ந, ன, ண" | `TA-W02-three-ns`, chapter **6** |
| `TA-C33-padi` | "That is Chapter 1's rule for க" | `TA-W01-abugida-va-ka`, chapter **5** |
| `TA-C33-ezhutu` | "never a fused shape: Chapter 1's bargain" | `TA-W03-pulli-vanakkam`, chapter **7** |

The word-level "Chapter 1" references elsewhere (வணக்கம், நன்றி) are still correct and
are left alone.

### Fixed — three stale README claims the strand work invalidated

`tamil/README.md` still described the strand as "**Writing W01–W04** … eight gentle
steps"; there are 22 lessons, W01–W18, and they now run to chapter 37. It also said
chapters 2–5 have no `chapters.json` entry, which stopped being true, and stated flatly
that "every payoff is a chapter's last lesson by `sequence`" — interleaving put a
`delivery: script` lesson after the payoff in several chapters, so that is now a
"usually", with a pointer to read `payoff.lesson` instead of assuming.

### Measured

- `totalLessons` 1687 → 1689, `pen` 65 → 67 — the two new lessons.
- `atomsTaught` 2646 → 2650.
- `atomsNeverRevisited` 470 → 472 — both `TA-W18`'s, orphans by construction because
  no later lesson practises script atoms. `TA-W17`'s two are not, because `TA-W18`
  re-reads **உணவு** beside **ஊர்**. (Superseded by the `TA-W19` entry above: a
  later lesson now does practise script atoms, and those same two leave the set.)
- `missedByWindow.R1` 880 → 884 — the four new atoms, nothing out.
- `missedByWindow.R2` 1804 → 1809, and only four of those five are new atoms. The fifth
  is a **regression this tranche causes**, and it belongs in the open rather than filed
  under measurement: `TA-LEX-VEEDU-01` is introduced by `TA-C35-veedu` and revisited far
  away by exactly one lesson, `TA-C38-vidai`. That revisit sat at offset **15** — the
  last position R2's 5-15 window counts. The two new lessons pushed it to **17**, so an
  atom that was passing R2 now misses it.
- `missedByWindow.R3` 1304 → 1307 is **five in, two out**, and `.R4` 240 → 242 is **three
  in, one out**. The arrivals are genuinely newly measurable — a longer track means far
  windows exist for atoms that previously had no room for them. The departures are the
  tranche's only reinforcement wins and were nearly lost to a net: `TA-SCRIPT-U-SIGN-01`
  and `TA-SCRIPT-WRITE-AAM-02` leave R3, and `TA-SCRIPT-PULLI-VANAKKAM-01` leaves R4,
  because `TA-W17` and `TA-W18` re-read the ு sign, **ஆம்** and the puḷḷi at exactly the
  distances those windows are looking for.
- `voice`, `sight`, `drivableLessons`, `drivablePrefixTotal`, `fullyDrivableChapters`,
  `unstartableChapters`, `payoffsNotRepresentative`, `chapterViolations` and
  `forwardReferences` **all hold**. This tranche adds lessons and removes no section, so
  nothing flips.

### Added — the Tamil writing strand reaches chapters 4-5, and stops at chapter 32

Chapters 4 and 5 were the last hand-authored Tamil chapters still teaching script
inline: eight lessons carrying a `## The letters in this word` section, and four
`sounds` boxes in the book. The glyphs they explained — **ே**, **ோ**, **த**, **ழ** —
were taught nowhere in the strand, so the sections could not simply go.

- **`TA-W16-read-tamizh`** (chapter 33) — **த** and **ழ**, spelling **தமிழ்**. **ழ** is
  the third *l*-letter, outstanding since chapter 18, and the letter the language is
  named for. It goes **first** of the three, and deliberately: `TA-C33-ezhutu` at
  sequence 970 already takes **ழு** and **து** apart inline, so the strand has to reach
  those letters before that lesson, not after it.
- **`TA-W14-read-pesu`** (chapter 34) — the **ே** sign, spelling **பேசு**. It is the
  long partner of the **ெ** from **பெயர்**, which makes it the third short/long pair
  after **அ**/**ஆ** and **ி**/**ீ**.
- **`TA-W15-read-po`** (chapter 35) — the **ோ** sign, spelling **போ**. This is the
  first sign that is two marks at once, and it carries the strand's first citation from
  outside `tamil.json`: the **Unicode Character Database** gives **ோ** (U+0BCB) a
  canonical decomposition of **ே** (U+0BC7) + **ா** (U+0BBE), both already taught. That
  is an encoding fact and the lesson says so — it settles what the sign is *composed
  of*, not how either half is drawn; the placement comes from the page.

Neither **த**, **ழ**, **ே** nor **ோ** has an entry in `tamil.json`, so none gets a
stroke order and the mouth-position descriptions are marked as the lesson's own rather
than sourced — except the **ோ** decomposition, which is cited to Unicode.

### Removed — chapters 4-5's inline script sections and book boxes

`TA-C04-po`, `-poy-varugiren`, `-naalai`, `-mindum-sandippom` and `TA-C05-pesu`,
`-velai-sey`, `-vaazh`, `-naan-tamizh-pesugiren` drop their sections. That closes the
five hand-authored chapters; **22** lessons in the generated chapters 33-38 still carry
one, which is the next chunk of this work rather than part of it. Meanwhile
`ch04-farewells.tex` and `ch05-first-verbs.tex` drop their four `sounds` boxes. All
eight flip `reasons: ["script-block"] → ["no-visual-dependency"]` in the generated
manifest with an empty `detachableSegments`.

Four of the eight sections held content that was not letter teaching — five items
between them — and all of it stays. The
*varu* + present + *-ēṉ* build, the *-ppōm* ending and the *-kkalām* ending move into
their lessons' prose in romanization; the retroflex *ṇḍ* of *mīṇḍum* becomes a spoken
note, which is what its `sounds: [retroflex-nd]` id was always pointing at; and the
**ḻ** note — which is about a *sound*, and about why Tamil is sometimes called "the *ḻ*
language" — moves into `TA-C05-naan-tamizh-pesugiren`'s discussion of the name rather
than being deleted with the block it happened to sit in.

### Two constraints that moved the lessons

Two constraints bit here, and both are recorded because both changed the plan.

**Ordering.** `TA-C33-ezhutu` (sequence 970) already makes **ழ** and **த** its own
subjects inline — "**ழு** is **ழ** with the *u* sign" — so a strand lesson introducing
them afterwards would be claiming first contact it does not have. `TA-W16` has no
dependency on the other two, so it goes first, at sequence 965, immediately ahead of
that lesson. The cost is visible in R2 above and is worth naming rather than smoothing.

**The chapter-32 budget.** The strand's 3:1 cadence put two of these lessons inside
chapter 32. Chapter 32 was
already **at** the ramp policy's ceiling — six verb lessons, two atoms each, exactly the
`maxNewAtomsPerChapter: 12` budget — so interleaving there took it to **16** and broke
`chapterViolations`, the number `ramp.test.ts` calls the one that most directly measures
"do not throw many things at the reader at once."

That is the opposite of the point, so the cadence yields and chapter 32 is skipped
entirely. The three lessons sit at chapters 33, 34 and 35, which leaves one nine-lesson
gap after `TA-W13` and then resumes 3:1. Reading distance between consecutive strand
lessons stays at 4 or more everywhere, so the R1 rationale in `continuity.test.ts` still
holds.

Skipping chapter 32 was not only the gentler choice, it was the cheaper one. Measured
both ways: interleaving there would have cost a ramp violation (24 → 25), a payoff
representativeness failure (29 → 30, chapter 32 at 7/16), a fully-drivable chapter
(324 → 323) and three lessons of chapter-32 drivable prefix. Skipping it costs none of
those.

### Measured, not inferred

Set differences against the pre-change corpus, both loaded with the same build.

- `totalLessons` 1684 → 1687, `pen` 62 → 65 — the three new lessons.
- `voice` 1106 → 1114, `sight` 516 → 508, `drivableLessons` 1106 → 1114 — the eight
  chapter 4-5 lessons, no others.
- `drivablePrefixTotal` 918 → 924 and `unstartableChapters` 139 → 137 — chapters 4 and 5
  gain 4 and 2 and both start by ear for the first time. Nothing is lost anywhere.
- `fullyDrivableChapters` **324, unchanged**; `payoffsNotRepresentative` **29,
  unchanged**; `chapterViolations` **24, unchanged**. All three because of the skip.
- `atomsTaught` 2640 → 2646 — 2 + 2 + 2.
- `atomsNeverRevisited` **470, unchanged** — the first time extending the strand has been
  free. Two in (`READ-TAMIZH-02`, `TA-ZHA-01`; nothing follows `TA-W16`), two out
  (`TTA-01` and `U-SIGN-01`, genuinely re-used by `TA-W16`'s ட/த contrast and `TA-W14`'s
  சு).
- `missedByWindow.R1` 874 → 880 — the six new atoms, nothing out.
- `missedByWindow.R2` 1800 → 1804 — all six miss it, and that is an ordering
  consequence rather than an authoring one. Putting `TA-W16` ahead of the other two, so
  it lands before `TA-C33-ezhutu`, spaces consecutive strand lessons 4 apart: outside
  R1's 1-3, but short of R2's 5-15, so nothing one introduces can be reinforced at R2
  distance by the next. Two leave: `TTA-01` and `U-SIGN-01`.
- `forwardReferences` 425 → **423**, and it improves for Tamil reasons rather than
  Spanish ones. Two chapter 4-5 lessons were quoting a verb in *script* that chapter 32
  teaches some twenty-seven chapters later: `TA-C05-vaazh` spelled **வாழ்** out of
  **வா**, and `TA-C04-naalai` glossed **பார்க்கலாம்** from **பார்**. Neither word moved;
  both are now named in romanization, which is what a speaking-first lesson should have
  been doing with them anyway.

### Still open

Chapter 1 keeps five `sounds` boxes, and they are a different problem again: their
content is pure pronunciation prose in romanization, with no Tamil letters in it at all.
The box is what is wrong there — `preamble.tex` hard-codes its title as "The letters in
this word", and `book.ts` routes every generated `Sounds you'll need` block into the same
box, so the mis-titling is corpus-wide across all 22 tracks rather than Tamil-specific.
That wants its own change.

### Added — the Tamil writing strand reaches chapter 3's words

The chapter 2 pass left six lessons still teaching script inline (`TA-C02-nii-niingal`
plus all five of chapter 3), and the same rule applied: the glyphs they explained —
ங, ள, ீ, ா, ட and ு — were taught nowhere in the strand, so deleting the sections
would have deleted the only explanation those letters had. Four new reading lessons
close that:

- **`TA-W10-read-naan`** (chapter 25) — the **ா** sign, spelling **நான்**. This is the
  one sign with a sourced description in `data/scripts/tamil.json` — *"a vertical
  stroke with a small top hook, written after the consonant"* — and it completes the
  picture of where a vowel sign can sit: after (ா), above-right (ி), before (ை, ெ).
- **`TA-W11-read-niingal`** (chapter 27) — **ங**, **ள** and the **ீ** sign, spelling
  **நீங்கள்**. It pays two debts the corpus had been carrying: `TA-W01` explained that
  **க** sounds like *g* after a nasal and printed **ங்க** without ever saying what **ங**
  was, and `TA-W06` promised three *l*-letters while teaching only **ல**.
- **`TA-W12-read-eppadi`** (chapter 29) — **ட**, spelling **எப்படி**, held against the
  **ண** the learner already has: the same retroflex curl, stopped rather than nasal.
- **`TA-W13-read-irukkirirgal`** (chapter 31) — the **ு** sign, spelling
  **இருக்கிறீர்கள்** and then the whole chapter 3 question off the page.

All four are reading-only. Of the six glyphs, only **ா** appears in
`data/scripts/tamil.json` at all (under `marks`, not `letters`), so it is the only one
described from a source. The other five say plainly that the book has no entry for
them, and the hedge covers the **sound** descriptions as well as the missing stroke
orders: **ங**, **ள** and **ட** lean on **ண**'s sourced "tongue curled back" and **க**'s
sourced position-picks-the-sound note, and say so rather than asserting their own
phonetics flat. `TA-W11` marks the same attestation gap for **ீ** that `TA-W09` marked
for **ெ**.

This started as three lessons and became four because the corpus caps a lesson under
300 effective seconds. The first draft put **எப்படி** and **இருக்கிறீர்கள்** in one
lesson and computed at 380s; splitting them is what the cap is for, and the result is
gentler than the draft was.

### Removed — six more lessons stop teaching script

`TA-C02-nii-niingal` and all five chapter 3 lessons drop their `## The letters in this
word` sections, and `ch02-introductions.tex` and `ch03-responding.tex` drop the four
matching `sounds` boxes. Verified against the **generated** manifest rather than the
source: all six flip `reasons: ["script-block"]` → `["no-visual-dependency"]` and their
`detachableSegments` lists empty, so the script teaching genuinely left the lesson.

Two of the six sections were not letter lessons at all. `TA-C03-eppadi-irukkirirgal`
used the heading to carry **verb morphology**, which is speaking content that happens to
be printed in Tamil; it moves into the lesson's own prose as *iru* + *-kkiṟ-* +
*-īrgaḷ* — the segmentation `TA-C32-iru` already pins, and one that concatenates to the
surface word — rather than being deleted. `TA-C03-paravayillai`'s section was a
word-joining note the very next section already makes, so it goes.

`sounds:` frontmatter is **kept** on all six, unlike the chapter 2 pass. Those lessons
list genuine pronunciation ids (`long-aa`, `retroflex-kk`, `final-m`); chapter 2's
listed script ids (`independent-e`, `pulli`), which is why clearing them was right
there and would be wrong here.

Chapter 3 is closed; chapters 4-5 are not. The claim carried over from the last pass —
that their boxes show ப, எ and ய — was wrong, so here is the measured inventory of the
four `sounds` boxes in `ch04-farewells.tex` and `ch05-first-verbs.tex`, by the chapter
that first teaches each glyph:

| | glyphs |
|---|---|
| never taught by any strand lesson | **ோ**, **ே**, **ழ** |
| taught long after book chapter 5 | **ா** (25), **ள** (27), **ு** (31), **ப** (23), **ர** (19), **ச** (19), **ல** (18), **ை** (18), **்** (7), **ந** (6), **ம** (6) |
| taught in time | **வ** (4), **க** (5) |

So the debt there is bigger than "three glyphs," and it is two different debts: three
glyphs the strand never reaches at all, and eleven it reaches only much later. Neither
is addressed here.

### Measured, not inferred

Every figure below is a set difference against the pre-change corpus, computed by
loading both with the same build.

- `totalLessons` 1680 → 1684, `pen` 58 → 62 — the four new lessons. They are `type:
  writing` and therefore `pen`, even though all four are reading-only.
- `voice` 1100 → 1106, `sight` 522 → 516, `drivableLessons` 1100 → 1106 — the same six
  lessons, no others.
- `drivablePrefixTotal` 913 → 918 and `unstartableChapters` 140 → 139. Chapter 3 gains
  6 — its first lesson needed eyes, and the whole chapter is now one ear-only run — and
  chapter 25 loses 1, because holding the 3:1 cadence puts `TA-W10` *between* its two
  speaking lessons rather than after them. `TA-W06` already sits mid-chapter in chapter
  18, so this is the established shape, not a new one. `unstartableChapters` is chapter
  3 alone.
- `fullyDrivableChapters` 327 → **324**, which moves the wrong way. Chapters 25, 27, 29
  and 31 each take a writing lesson and stop being fully drivable (−4); chapter 3
  becomes fully drivable (+1). That is the honest cost of paying the debt where it
  belongs. Corpus `coreDrivable` does not move, but not because these blocks detach —
  rule 1 classifies a `type: writing` lesson as `pen` without reading its body, so all
  four record `coreDrivable: false`. It holds because the six lessons that flipped were
  already core-drivable and the four new ones were never counted.
- `payoffsNotRepresentative` 27 → **29**. Tamil 25 (2/3 → 2/5) and 27 (2/2 → 2/5) fall
  below the 0.5 floor because they gained script atoms their speaking payoffs do not
  assess — the same trade chapter 13 already records, and both are now noted in
  `tamil/chapters.json`. Tamil 29 and 31 took the same hit and did **not** join: each
  landed on exactly 2/4 (0.50). The difference is one atom of arithmetic.
- `atomsTaught` 2631 → 2640 — 2 + 3 + 2 + 2 new atoms.
- `atomsNeverRevisited` rises 469 → 470. Three in — `TTA-01`, `U-SIGN-01` and
  `READ-QUESTION-02`; nothing follows `TA-W13`, so its two are orphans by construction.
  Two out: extending the strand finally re-uses ெ and ப, so `E-SIGN-02` and `PA-YA-01`
  leave the set. `READ-PEYAR-03` stays. Three in, two out. `AA-SIGN-01`, `II-SIGN-01`,
  `NGA-LLA-01` and `READ-NAAN-02` never become orphans, because each lesson declares —
  and genuinely assesses — the earlier letters its own word is built from: `TA-W11`
  credits the ந it builds **நீ** from, `TA-W13` credits the ள it re-reads in
  **ள்** and the **நான்** it holds against **நீங்கள்**.
- `missedByWindow.R1` 864 → 874. Nine are the new atoms. The cadence puts consecutive
  strand lessons **four** apart in reading order, and R1 is a 1-3 window — an early draft
  put `TA-W10` after the fourth speaking lesson instead of the third, which made that gap
  3 and quietly falsified this rationale; the sequence was moved to 785 so the claim and
  the corpus agree. The tenth miss is not new and is worth naming: interleaving at
  chapter 29 pushes `TA-LEX-AFTERNOON-BOUNDARY-01`'s reinforcement past R1.
- `missedByWindow.R2` 1799 → 1800. Four of the nine new atoms miss R2; the other five —
  `AA-SIGN-01`, `II-SIGN-01`, `NGA-LLA-01`, `READ-NAAN-02`, `READ-NIINGAL-03` — do not,
  because a later strand lesson practises them 5-15 lessons on, which is what threading
  them through `practises` was for. Three pre-existing atoms are pulled back inside R2
  against that: `PA-YA-01`, `ETYMON-KAALAI-01` and `ETYMON-MAALAI-01`. `E-SIGN-02` leaves
  the orphan set but not R2 — `TA-W10` re-uses it at a distance R2 does not count.
  Four in, three out.

### Added — the Tamil writing strand reaches chapter 2's words

The speaking-first change left nine chapter 2-3 word lessons still teaching script
inline, and they could not simply be deleted: the glyphs they explained existed nowhere
in the writing strand, so removing them would have removed the only place those letters
were taught. Two new reading lessons close that for chapter 2:

- **`TA-W08-read-en`** (chapter 21) — **எ**, the third word-initial vowel, spelling
  **என்**. The rule was already known from **ஆம்** and **இல்லை**; this is the same rule
  meeting a third vowel, which is the point of having a rule.
- **`TA-W09-read-peyar`** (chapter 23) — **ப**, **ய** and the **ெ** sign, spelling
  **பெயர்**. ெ stands to the **left** of its consonant and is spoken after it, exactly
  as **ை** does, so the left-standing signs become a family rather than a series of
  exceptions.

Both are reading-only. None of எ, ப, ய or ெ has a sourced stroke order in
`data/scripts/tamil.json`, so neither lesson invents one and both say so. `TA-W09` also
marks how well attested each half of its ெ claim is: the left-standing description is
sourced for **ை** and inferred for **ெ** from the pattern the two share.

Both sit at the strand's established cadence — one script lesson after every third
speaking lesson — and both spell words the learner has said since chapter 2. Note the
third removal is not symmetric: **என்ன** loses its section because every glyph in it is
taught (எ by `TA-W08`, ன and the puḷḷi earlier), but no strand lesson ever assembles the
doubled ன்ன itself.

### Removed — three chapter 2 lessons stop teaching script

With the glyphs housed, `TA-C02-en`, `TA-C02-enna` and `TA-C02-peyar` drop their
`## The letters in this word` sections, and `ch02-introductions.tex` drops the three
matching `sounds` boxes. Verified against the **generated** manifest rather than the
source: all three now record `reasons: ["no-visual-dependency"]` and read as `voice`, so
this is script teaching genuinely leaving the lesson, not a heading renamed out from
under the classifier. Tamil chapter 2 becomes startable by ear for the first time
(`unstartableChapters` 129 → 128, its drivable prefix 0 → 2).

**`TA-C02-nii-niingal` keeps its section**, because ங, ள and the ீ sign are still taught
nowhere else. A strict check — does a strand block make the glyph its own subject, in a
heading, its own table row, or an "X is Y" sentence — was needed to see this: all three
*appear* in strand lessons, but only inside examples (ஸ்ரீ, ங்க, புள்ளி). Mere
appearance is not teaching, and the looser check would have licensed deleting the only
explanation those letters have. The same test says ப was never taught either, which is
why `TA-W09` teaches it rather than assuming it.

Six chapter 3 lessons still teach script inline, and chapters 3-5's book `sounds` boxes
still show ப, எ and ய before the strand reaches them. This closes chapter 2 only.

Measured: `atomsTaught` 2502 → 2507, `voice` 1076 → 1079, `sight` 535 → 532, `pen`
56 → 58, `unstartableChapters` 142 → 141, `drivablePrefixTotal` 876 → 878, and
`fullyDrivableChapters` 323 → 321 as chapters 21 and 23 each take a writing lesson.

`atomsNeverRevisited` **rises**, 472 → 474, and it is worth saying why rather than
burying it. Three of the five new atoms are `TA-W09`'s, and nothing follows `TA-W09`, so
they are orphans by construction. Against that, `TA-W09` re-uses ர when it spells
**பெயர்**, and declaring `CA-ONE-LETTER-01` pulls that atom out of the orphan set for
the first time. Three in, one out.

`missedByWindow.R2` 1716 → 1718 is the same shape and the more interesting one: all five
new atoms miss R2 too, offset by **three** pre-existing atoms that `TA-W09` pulls back
into it. `TA-W09` sits 12 lessons after `TA-W06` and 8 after `TA-W07`, both inside R2's
5-15 window, so practising `INDEPENDENT-VOWEL-I-01`, `LA-AI-SIGN-02` and
`CA-ONE-LETTER-01` there reinforces them at a distance R1 can never reach. A strand
spread thin misses the near window and starts hitting the far one.


### Changed — Tamil teaches speaking first, and the script arrives gently

Tamil chapter 1 held **eleven writing lessons against nine speaking lessons**, and the
curriculum path put all eleven at positions 3-13: a learner met வணக்கம், then did eleven
consecutive lessons on letter shapes before reaching the word for *yes*. The chapter's
own declared capability said so out loud — *"I can write வணக்கம் and நன்றி by hand, put
the puḷḷi and the ி sign in the right places"* — a chapter about greeting people whose
stated payoff was handwriting.

You can speak a language without reading a letter of it, so the course now does that:

- **Chapters 1-3 carry no writing lesson.** 23 lessons — greetings, yes/no, thanks,
  names, how-are-you. Chapter 1's capability is now *greet someone, say yes, no and
  thank you, agree with சரி, and hold a short exchange out loud — entirely by ear*,
  and chapter 1 becomes the first Tamil chapter that is fully drivable by ear.
    `TA-C01-practice` had a section headed *"Read them back"* that taught the puḷḷi,
  vowel signs and word-initial vowels inside the chapter-1 recap; it now says the five
  words aloud and states plainly that reading is not expected yet. The **book** said the
  same thing more loudly and had to be fixed too: all five `sounds` boxes in
  `ch01-greetings.tex` taught left-to-right reading, the inherent vowel, the puḷḷi and
  the ி and ை signs, and its recap table was headed *"Read"*. Chapters 1-5 are
  hand-authored, so `book-cli --check` never compares them against the lessons and
  nothing flagged the divergence. Those boxes now teach **pronunciation** — retroflex ṇ,
  the held *kk*, Tamil's three *n* sounds, vowel length, the *ai* diphthong — which is
  what a box called "sounds" should have held all along.
- **The script starts in chapter 4, one lesson at a time.** The eleven script lessons  are spread across chapters 4-19, admitted after every third speaking lesson.
  Measured, they sit at 0-indexed reading positions 27, 31, 36, 40, 44, 48, 52, 56, 60,
  64, 68. Chapter 4's lesson teaches no letter at all — it is the palm-leaf lesson on
  why the script is round; the first actual letters, வ and க, arrive in chapter 5.
- **Every script lesson spells a word already known by ear.** வணக்கம் is learned in
  chapter 1 and written in chapter 8, once all its letters exist; நன்றி, ஆம், இல்லை and
  சரி likewise. The writing strand carries no new vocabulary at all.
- **The script is shown from page one but never taught early.** Tamil words still appear
  in the speaking lessons so the shapes grow familiar; nothing asks the learner to read
  or write them until chapter 4. `tamil/book/chapters/ch01-greetings.tex` said the
  opposite — *"Each lesson introduces the letters its word needs"* — and now says what
  the chapter actually does.

### Known limitations of this change

Two things this does **not** fix, stated plainly rather than implied away:

- **Ten early lessons still teach script inside a speaking lesson.** Nine chapter 2-3
  word lessons carry a `## The letters in this word` section. They are not moved here,
  because the letters they teach (உ, ய, எ, and the ெ sign) are taught nowhere in the
  writing strand — deleting the sections would remove the only place they exist. Moving
  them needs new writing lessons authored first.
- **Chapter 1's payoff is now unmeasured rather than passing.** `chapters.ts` guards the
  representativeness check with `introduced.size > 0`, and chapter 1 introduces no atoms,
  so the gate that used to report `tamil:1` at 5/24 now says nothing about it. The corpus
  total `payoffsNotRepresentative: 25` holds only because `tamil:13` took its place —
  `TA-W04-i-sign-write-nandri` moved to chapter 13, whose payoff now covers 1/4.

### Added — `delivery: script` marks the writing strand

The eleven writing lessons declare `delivery: script` in their frontmatter, so a
spoken-only edition can filter on one field instead of inferring the strand from `type`,
`skills` or a computed modality. A new test pins the marker to exactly the writing
lessons of any track that adopts it, so a writing lesson that forgets it — or a speaking
lesson that gains it by copy-paste — fails rather than shipping in the wrong edition.
Script material *inside* a speaking lesson is already typed at the block level
(`block.type === "script"`), so a spoken edition can drop those blocks with no new
metadata.

### Fixed — Tamil chapters 2-5 had no declared order at all

24 Tamil lessons carried no `sequence`, so four whole chapters fell back to
**alphabetical** order. Chapter 2 therefore taught the assembled phrase *en peyar*
("my name") before *peyar* existed, and asked *"what is your name?"* last,
after the practice lesson. Nothing caught it, because a chapter with no declared order
has no order to contradict. All 116 Tamil lessons now carry a globally unique sequence,
which also clears seven duplicate sequence values main was already carrying undetected
(the duplicate gate only checks schema-v2 lessons, and each pair had a v1 lesson on one
side, so the schema-v2 duplicate gate could not see them — though the continuity walk
was already reporting all seven as `duplicate-sequence` order defects, unsummarised and
unpinned). Two genuine forward reviews are fixed with it: `TA-C02-magizhcci` reviewed
`TA-C02-ungal-peyar-enna` and `TA-C04-mindum-sandippom` reviewed `TA-C04-naalai`, both
before those lessons existed. Tamil forward prerequisites and forward reviews are now
both **zero**.

Measured: `lessonsWithoutSequence` 507 → 483, `tracksWithUnorderedLessons` 19 → 18
(Tamil joins chinese, japanese and latin — the fourth of 22), `forwardPrerequisites` 240 → 230,
`forwardReviews` 285 → 273, `forwardReferences` 469 → 468, and `chapterViolations`
25 → 24 — that last being the gate that measures how much a chapter throws at a reader
at once, which Tamil chapter 1 had been failing at 24 atoms against a budget of 12.

Three numbers moved the wrong way, all deliberate:

- `missedByWindow.R1` 834 → 843. Consecutive script lessons now sit 4-5 apart and R1 is
  a 1-3 window, so no script lesson can reinforce the previous one inside it. 24 atoms
  miss R1 while their real revisit counts run 0 to 7, median 2 — reinforced, just never
  within three lessons. An interleaved strand cannot satisfy R1 as defined.
- `fullyDrivableChapters` 327 → 322, which is −6 +1 rather than a flat loss. Six
  chapters that were entirely ear-only now hold a writing lesson (6, 10, 13, 16, 18, 19;
  chapter 6 holds two), offset by chapter 1 becoming fully drivable for the first time.
- Script-ramp `lessonViolations` 60 → 61, a net of three moves: `TA-W03-pulli-vanakkam`
  (9 glyphs, the steepest in the corpus) stopped counting once it moved to chapter 7,
  while `TA-C01-practice` (5) and `TA-C01-nandri` (4) joined — both **speaking** lessons
  showing glyphs the writing strand has not reached. The gate counts a glyph the first
  time it appears, which is the right rule for a track that teaches script alongside
  speech and the wrong one for a track that shows before it teaches.

### Fixed — the seven script facts the one-word-per-lesson pilot orphaned

Splitting Tamil chapter 1 moved letter-by-letter reading out of the word lessons and
into the writing track, on the grounds that the writing track already taught those
letters. For **seven of them it did not**: ஆ, இ, ச, ர, ல, the ை sign, and the rule
that *a word opening on a vowel opens with a full vowel letter, not a sign*. The
chapter recap still tested all of it.

Three writing lessons close the gap, each teaching its letters and then assembling the
word they spell:

| lesson | teaches | writes |
|---|---|---|
| `TA-W05-write-aam` (230s) | **ஆ**, and the word-initial vowel rule | ஆம் |
| `TA-W06-write-illai` (233s) | **இ**, **ல**, the **ை** sign, three *l*'s | இல்லை |
| `TA-W07-write-sari` (241s) | **ச** as s/ch/j, **ர**, two *r*'s | சரி |

They sit at sequences 182, 192 and 222 — **after** the word each one spells, so the
learner meets a word by ear and then learns to write it. That is the opposite of the
pre-existing `TA-W04` / நன்றி inversion, where the writing lesson taught how to write a
word 30 sequence-steps before the word itself was introduced.

Each computes to 230–241 seconds under `estimateLessonDuration`, which puts them
with the writing lessons they sit among (TA-W01–W04 compute 176–297s) rather than
with the word lessons, which run 88–136s. That split is the point: a word lesson
is short because it holds one word, and a writing lesson is longer because the
hand is slower than the ear. All three declare `max_seconds: 260`, above their
computed cost, so the effective figure is the declared one and nothing is
silently absorbed by `max(declared, computed)`.

Two things the review caught that are worth recording. **ை was described backwards** —
it is written to the *left* of its consonant and pronounced after it, which the
project's own `data/scripts/tamil.json` and `TA-W04` both already said. The wrong
version had reached the narration script. And **ச and ர are taught for reading only**:
this project gives a stroke order only where it has a sourced one, and those two have
none, so the lesson says so rather than inventing strokes.

### Changed — an etymology is a hook, so the gate stops demanding it be drilled

Project owner's directive: *"Etymology should only be mentioned once. I do not want
that re-emphasized again and again. It is mostly a memory hook for me."*

**The gate was manufacturing the repetition.** HL09 §3.1 requires every atom to be
revisited at least twice, and for an etymon the only way to satisfy that was to
re-state it in the Guided Practice and again in the Wrap-up Recall. The prose was
shaped by the measurement, so the measurement had to change first.

Etymology atoms — `*-ETYMON-*`, a naming convention every track already follows — are
now **waived from the reinforcement criterion**. Spanish's pre-A1 reinforcement blocker
goes from 87 atoms to 53, and says so: *"53 atom(s) at or below pre-A1 are revisited
fewer than twice (35 etymology hook(s) waived)."*

This also settles a question open for weeks: the once-cited `ES-ETYMON-*` atoms should
be **waived, not re-cited**.

**The waiver lives in `level-gate.ts`, not in `continuity.ts`, on purpose.**
`measureContinuity` goes on reporting every atom truthfully, so `atomsTaught`,
`atomsNeverRevisited` and the R-window counts keep meaning what they say, the gap
report stays honest, and **no pinned corpus figure moves**. Only the level *claim*
ignores them — the one place the decision actually applies — and the waiver is printed
in the blocker rather than silently absent.

The `-ETYMON-` convention is not enforced by schema, and a census found a few atoms
that arguably qualify and are not matched (`ES-HISTORY-AL-ANDALUS-LOANS`,
`SA-SOUND-PIE-KW-OUTCOMES`). Naming those consistently is the fix; widening the regex
is not.

### Not done here: removing the repetition already written

The prose still re-states etymology in Guided Practice and Wrap-up Recall blocks. An
automated sweep over chapters 1–3 was attempted and **reverted** — review found it
corrupted a URL and an `i.e.` by inserting spaces inside them, deleted 23 load-bearing
`[PAUSE 3s]` cues that `explicitPauseSeconds` and the narration script both consume,
glued four headings, left dangling connectives ("Two things:" above one question), took
eleven questions that were testing real skills, and in about ten files removed the
lesson's single *teaching* of an etymology while keeping the *drill* — the exact
inverse of the directive.

Two lessons from that: in Arabic and the other Semitic tracks **"root" means the
three-consonant root system**, which is core grammar and not an etymology hook, so a
matcher keyed on the word cannot be trusted; and a recall paragraph cannot be re-flowed
freely, because `countPromptLines` counts lines containing a question mark and a
re-wrap therefore changes the computed duration.

That work needs to be done by hand, per track, and is not attempted here.

### Fixed — `taughtWords` stripped any short word, and that masked a second bug

A headword carries its article — *el pan*, *la casa* — and a body saying *bebo agua*
is using the same word, so the bare noun has to match too. The rule that did this
stripped **any leading word of three characters or fewer**.

A census of the corpus's own headwords shows what that meant: the rule fired on
**227 of 1,453 lessons** (247 headword parts), and only **49 lessons** (64 parts)
actually begin with an article. It was registering

- `llamo` as taught by *me llamo*, `favor` by *por favor*, `dia` by *bom dia*,
- `piace` by *mi piace*, `heiße…` by *ich heiße…*, `wiedersehen` by *auf wiedersehen*,
- and the night- and afternoon-word of every ശുഭ / शुभ / శుభ / ಶುಭ greeting across
  Malayalam, Hindi, Telugu and Kannada — because all those openers are three
  characters.

The rule is now an **allowlist of real definite articles, per language**, taken from
that census rather than from a length guess. Two deliberate exclusions: Spanish
**`lo`** (it *is* a neuter article, but the corpus's only `lo `-headword is *lo
siento*, where it is a pronoun) and Italian **`a`** (*a domani*, *a presto* are
prepositions). A track absent from the map never has anything stripped, which is right
for Latin, Arabic, the Indic tracks and every other language whose headwords carry no
article — Arabic's `ال` is prefixed without a space, so neither rule ever reached it.

### …and the second bug, which only became visible once the first was fixed

The bad strip had been seeding each lesson's own word set with the stripped tail, which
incidentally stopped it reporting itself. Removing the strip removed that accident, and
**four lessons began being reported for a word sitting in their own headword** —
`مع السلامة` for `السلامة`, `bom dia` for `dia` — which is exactly what this module's
docstring says must not happen.

`ownHeadwordTokens` now does it deliberately, and completely. The accident only ever
covered the tail after the first word, so 45 self-references it had never caught are
gone too; every one was verified to be a token of the reporting lesson's own headword.

**`forwardReferences`: 524 → 443, and none of the 81 was a real finding.** What remains
is a claim about the corpus rather than about the matcher.

**The workaround this forced is removed.** Spanish chapter 41's connective lesson had
been renamed from `así que` to `así` purely to dodge the bug — `así` is three
characters, so `que` registered as first taught there and flagged ten earlier lessons.
The true headword is restored, and the full result set is byte-identical either way.

Four unit tests pin the behaviour. Three of them fail if the length rule comes back;
the fourth asserts the *positive* case — that `el pan` still registers `pan` — and
fails if `taughtWords` returns nothing. Both directions were checked by stubbing.

### Added — coverage assertions for the book manifest (HL-C50)

The Spanish book was printing chapter 38 twice and dropping chapter 40, and **every
check passed** while it did. That defect is fixed; these are the assertions that would
have caught it, plus an audit confirming no other track carries the same drift.

The gap was structural. `check:books` compares each **declared** target against what
the generator produces, so a manifest declaring the wrong chapter round-trips
perfectly. `titleDrift` stayed 0 because each file took its title from its own
(correct) target. Narration and modality read the corpus directly and never consult
the manifest. Nothing asked the coverage question: *do the declarations line up with
the corpus?*

Five assertions now do, in `book-cli.test.ts`'s existing manifest block:

1. **No chapter number declared twice in a track** — the drift's direct signature.
2. **Every filename agrees with its declared chapter** — `ch39-*.tex` must not be
   declared as chapter 38. The cheapest tripwire, and it would have fired the instant
   the drift was written.
3. **Every declaration stays inside its own track's directory** — a target writing
   into another track's folder passes every other check while silently adding a
   chapter to a book nobody edited.
4. **No two declarations write the same path** — the loser vanishes silently.
5. **Every ledgered chapter is `\input` into its book**, not merely present on disk.
   "Reaches a file" is the weaker claim, and the weaker claim is what let the original
   bug through in spirit.

**They run over `targets` and `handwritten` together**, which matters more than it
sounds: the manifest's `handwritten[]` array holds 105 of the 452 declarations, and the
identical drift there was invisible to every test in the package. Keeping the two
halves apart is what allowed that.

Each assertion was proven to fire before being trusted. Reintroducing the exact
Spanish drift trips three; the same drift in `handwritten[]` trips two; a target
escaping its track trips two; deleting a declaration trips two; and removing an
`\input` while leaving the file on disk trips the fifth alone.

**Audit result: no other track is affected.** Across all 22 — and across both arrays —
there are no duplicate declarations, no filename/chapter mismatches, no path
collisions, and no ledgered chapter missing from its book. Seven chapters have no
generation target (hindi 1–2, latin 1, persian 2, russian 2, tamil 1, urdu 2); all
seven are hand-authored, declared in `handwritten[]`, and `\input` into their books.

### Added — Spanish chapter 41, the second B1 rung (SPINE-GIVE-REASONS)

**"Saying Why"** — four lessons, one concept each, etymology in every one, ending in
a four-move explanation the reader can build in either direction.

| lesson | concept | what is new |
|---|---|---|
| `ES-C41-creer` | OPINION-I-THINK | stating an opinion, against `pensar`'s weighing |
| `ES-C41-asi-que` | CONNECTIVE-SO | the forward arrow, where `porque` points back |
| `ES-C41-deber` | MODAL-SHOULD | obligation, from a word that means *to owe* |
| `ES-C41-explicar` | EXPLANATION-GIVE | the synthesis: opinion, cause, consequence, obligation |

Spanish now realizes **two** of the seventeen B1–C2 nodes. `attained` is still null.

The chapter-38 review's four failure classes were each checked for up front, and three
held: nothing here re-teaches owned material (`creer`, `deber`, `explicar`, `así` and
`entonces` appear nowhere else in the track), every word in the payoff is taught, and
the book wiring — target, `\input`, generated `.tex` — went in with the chapter rather
than after it.

### Note — the fourth class did not hold: five etymological errors

Same shape as chapter 38's: **right family, wrong route or wrong strength.**

- *"`así` ← Latin **`ad sīc`**"* — the weakest of four proposals. The DLE now gives
  plain `sīc`; the `a-` is a Romance accretion, on the pattern of *apenas*, *afuera*.
- *"`*ḱred-` **is** the heart word"* — traditional but **disputed**: the heart root is
  `*ḱḗr`, never `*ḱred-`. The compound is secure; the identification is not. Now hedged
  in the lesson rather than asserted, and the recall question asks *why* to hedge.
- *"`endeavour` **reached English** via French `devoir`"* — no French word `endeavour`
  ever existed. English **calqued** `mettre en devoir` as *put in dever*, which fused.
- *"**explicit**, **application** — straight from Latin"* — both came through French.
- *"`comply` from `plicāre`"* — it is from `complēre`, "to fill", a different PIE root
  that the French verb ending disguises. Exactly the trap the lesson otherwise teaches.

### Fixed — three defects that reached the reader, and one the TTS would have spoken

- **`explicar` was said to "take the same shape as `contar`"** — but chapter 38 teaches
  `contar` as an o→ue stem-changer, three chapters earlier. `explicar` is plain `-ar`.
  It now points at `hablar` and says explicitly that `contar` does the other thing.
- **`No creo que …`** sat bare between two indicative rows. Negating `creer` takes the
  **subjunctive** — *no creo que llueva* — which chapter 18 already taught. Completed
  and tied back rather than dropped.
- **`pr-cluster`** was tagged on the *pl* of `explicar`; that tag is defined elsewhere
  as *pr* with a tapped *r*. Now `l-clear`.
- **A Greek capital beta (U+0392)** had crept into `deber`'s pronunciation hint, and
  propagated into `narration/ch41.txt` — a file fed to text-to-speech.

### Changed — a headword narrowed to stop nine false positives

The connective lesson was first written with the multi-word headword **`así que`**.
`taughtWords` strips a leading word of ≤3 characters — a heuristic meant for articles —
so it registered **`que`** as first taught at chapter 41 and flagged all nine earlier
lessons containing it. `que` is genuinely taught at chapter 7.

The headword is now **`así`**, the only word that is actually new, with the gloss
carrying the `que`. `forwardReferences` holds flat at 524. The loader heuristic is the
real bug and is recorded as such in the pin comment.

### Added — the first content above A2: Spanish chapter 38 (HL09, SPINE-NARRATE-EVENTS)

The B1–C2 spine landed with all 17 nodes declared as gaps in all 22 tracks. This
realizes the first of them, in one track, as a worked example: **"Telling What
Happened"**, four lessons, one concept each.

| lesson | concept | what is new |
|---|---|---|
| `ES-C38-luego` | CONNECTIVE-THEN | the connective use of a word chapter 5 already gave you |
| `ES-C38-porque` | CONNECTIVE-BECAUSE | joining a reason to a claim |
| `ES-C38-imperfecto` | ASPECT-ONGOING-PAST | **not** the forms — scene versus event |
| `ES-C38-contar` | NARRATIVE-SEQUENCE | the synthesis: a four-sentence told story |

Every lesson carries an etymological connection, per HL09. Spanish `touches` B1 now;
`attained` is still null, which is the distinction the level gate exists to keep.

### Note — two lessons were re-teaching material the book already owned

Review caught both, and both were the same mistake: writing a B1 lesson without
checking what the track had already taught.

- **The imperfect is taught in full at chapter 16**, with its three irregulars and
  the same `-ābam` etymology. The draft presented it as new — 22 chapters late.
  Rewritten to teach the only thing that *is* new at B1: **which past tense to reach
  for**, scene versus event. Chapter 16 is now cited, reviewed, and credited.
- **`luego` is taught at chapter 5**, inside *hasta luego*, along with its `locus`
  etymology. The draft re-introduced `ES-LEX-LUEGO` and the etymology as fresh atoms.
  It now *requires* chapter 5's atoms and introduces only
  `ES-GRAMMAR-LUEGO-CONNECTIVE-01` — the connective function, which is genuinely new.

Also caught: the payoff story used five words the course never teaches (*ventana*,
*temprano*, *cada*, *hambre*, *historia*) directly beneath the claim **"every word is
one you already own"**. The story is rebuilt from taught vocabulary and the claim is
now true. `forwardReferences` cannot see this class — its blind spot is words the
course teaches *nowhere*.

### Fixed — four wrong etymologies and one missing book target

- *"`*kʷ-` **hardened** to `hw-`"* — Grimm's Law turns a stop into a fricative. That
  is softening, and it was the one sentence the lesson was built on.
- *"`que` ← Latin `quid` / **`quod`**"* — `que` inherits `quid` and *usurps* `quod`'s
  roles without taking its form. The lesson's own recall answer already said `quid`.
- *"Knock the final consonant off each"* — false for `-ābās` → `-abas`, in the table
  three lines below. Both this and the contested intervocalic-`b`-loss claim were cut
  with the rest of the forms material.
- *"Two languages, **independently**"* — English *recount* and *account* are the same
  word Spanish inherited, via Old French. Only *tell* is a genuine parallel, and the
  paragraph now says so.

**The chapter reached the app but not the book.** `core/book-generation.json` is a
hand-maintained target list; narration and modality are corpus-driven and picked
chapter 38 up automatically, so `check:books` passed while the book had no chapter.
Target added and `book.tex` wired — `bookChapters` 442 → 443.

### Changed — two pin comments that described the wrong cause

- **R1 +3 / R2 +10** was attributed to the final lesson's atoms having nothing after
  them. Wrong: of the +3 R1 exactly one is a chapter-38 atom, and of the +10 R2 none
  is. `continuity.ts` only judges a window that fits, so *lengthening* the track
  un-suppressed windows on chapters 36–37. The debt was already there; appending a
  chapter made it measurable.
- **forwardReferences +6** was called six instances of the fixed phrase from chapter
  4. Five are chapter 5; the sixth (`ES-C10-practice`) is bare adverbial *luego* —
  a real 28-lesson-early leak, reported identically to the five spirals. That is the
  severity split the metric's own comment has been asking for.

### Added — the B1–C2 spine, so the ladder has rungs above A2 (HL09 §3.1)

`spine.json` stopped at A2. The level gate handled that correctly — it refused B1–C2
on the grounds that "no node is unrealized" is not "every node is realized" — but the
effect was that **no amount of content could ever certify above A2**, because there
was nothing to certify against.

**17 nodes**: B1 ×5, B2 ×4, C1 ×4, C2 ×4, each a CEFR can-do statement with
prerequisites that resolve and never point up a level (both asserted).

- **B1** narrate in order, give reasons, cope while travelling, describe experience,
  express a real condition.
- **B2** argue a view, report what others said, read extended prose, discuss the
  abstract.
- **C1** infer what is implied, structure a long text, shift register — and
  **follow regional variation**, which had no home in the spine at all.
- **C2** synthesise several sources, express fine shades, read literary and older
  text, and **read the cultural weight of a phrase**, not only its meaning.

**68 concepts registered canonically**, each owned by exactly one node. Five were
first written as `VERB-EXPLAIN`, `VERB-HOPE` and so on, which silently pushed
`coreVerbCount` from 40 to 45 — joining a baseline that HL-C46/47/49 owns. They are
named by discourse function instead (`EXPLANATION-GIVE`, `AMBITION-EXPRESS`).

**All 22 tracks declare all 17 as gaps** — `segments: []` with every concept in
`omits`. A ledger that stays silent about a node reads the same as one that has
nothing to say; this one says exactly what is missing.

Nothing is realized, and the gate still reports every track at pre-A1 with its real
blockers. Authoring a rung does not climb it — the test that used to assert "B1 is not
authored" now asserts the stronger thing: B1 exists, and no track attains it.

### Fixed — the course named as a set (HL-C50)

The last of the standalone-book classes: the series referred to as a **set**, without
the word "in" that the previous guard keyed on — *"the course keeps finding
connections"*, *"the curriculum treats Latin as a taproot"*, *"the course's first
taste of case"*, *"every other language in this arc"*, *"the fifth language to close
the loop"*. **66 authored files** — 53 lesson sources, 11 handwritten chapters and
book shells, 2 reference docs.

Two treatments, and the split is the substance of the change:

- **In-volume references became "this book"** — *"the first word of the course"* →
  *"the first word of this book"*. A reader is holding the book, so a claim about it
  is answerable. The guard deliberately does not match "in this book".
- **Cross-volume references were reworded** to drop the invisible set — *"unlike
  every other language in this arc's dog-word"* → *"unlike the tangled dog-words of
  Spanish, Hindi, and English"*.

### Note — moving a claim in-volume makes it checkable, so check it

Book-scope is the safer place for a claim, but it is not a free move: it converts
something vague into something a reader can verify. Two failed that verification and
were caught in review:

- *"three genuinely separate calendar traditions across **this book**"* — the Tamil
  book teaches **one** calendar. The proof sat nine lines above, unchanged: *"By now
  you've seen Arabic and Hindi each juggle two calendars."* The reader met the other
  two in other volumes.
- *"closing **this book** on the root that opened it"* — the Arabic book runs 32
  chapters; this is chapter 4. It closes the **greeting arc**, and now says so.

That is three such failures in two changes (the first was *"a third fate for a
consonant in this book"*, which the Punjabi volume never enumerates). The rule is
recorded in `lessons.md`: an ordinal or count moved to book scope must be walked
against that volume's actual table of contents, not assumed.

### Changed — the guard, and the bypass it had already been taught about

Six patterns added: `(the|this) (course|curriculum|series)`, `(course|curriculum)'s`,
`(in|across|throughout) this arc`, `the (first…tenth) language`, `every track`, `this
course covers`. Each proven to fire on a reintroduced defect while *"this book"*,
*"this book's first taste of case"* and an in-volume *"the numbers arc"* keep passing.

The first version shipped a bypass the test file's own comments already warned about:
**one intervening adjective defeats the pattern.** *"this **whole** curriculum"*,
*"this **single** course"*, *"the **whole** course"* — an earlier pattern had learned
exactly this and the lesson was not carried across. Seven more sites were hiding
behind it.

### Fixed — the course-level phrases (HL-C50)

The class the previous change counted and deferred: bare *"in this course"*, *"in
this curriculum"*, *"in this series"*. **31 sites across 29 authored files** — 29 in
27 lesson sources, 2 in the handwritten `sanskrit/book/book.tex` and its ch01.

- A reader holding the Tamil book does not know which languages the series covers,
  so *"every Dravidian language in this course"* names a set they cannot see.
- Two were **build-system notes printed at a learner**: *"**ट** is not yet in this
  curriculum's stroke data — draw it only after its entry is authored"*. The reader
  is not the person who authors stroke data.
- Ordinals moved **in-volume** rather than being deleted: *"the first one in this
  course whose vowel changes"* → *"in this **book**"*. A reader is holding the book,
  so that ordinal is answerable. The guard deliberately does not match "in this book".

### Note — deleting "in this course" widens a claim from the books to the world

This is the whole difficulty of the class, and it cost real defects again.

*"Every European language **in this course** splits the year into four seasons"* is a
safe claim about a curriculum. Drop three words and it is a claim about European
languages — and Sami traditionally counts eight. Every quantifier here was re-scoped
deliberately, and review still found two that had become false:

- *"Across Europe and North India, weekdays are named for planet-gods"* — Portuguese
  counts its weekdays (*segunda-feira*, *terça*…), as do Greek and every Slavic
  language. Worse, it destroyed the lesson's own punchline, which is that Arabic
  *counts* them: Portuguese does the same thing. The original carried **two** hedges,
  "so far" and "in this course"; the rewrite dropped both.
- *"Most languages say 'my name is' with a word for my and a word for name"* — false
  for the whole Romance branch (*je m'appelle*, *me llamo*), for German, and for
  Chinese: six tracks in this series.

Three more rewrites lost their footing without being false: a cousin web described as
"carrying" a language (inverted), kanji said to have "no parallel in any alphabet"
(cuneiform and hieroglyphs are just as polyvalent, and nothing nearby mentioned
alphabets), and *"a third fate for a consonant in this book"* — moving that ordinal
in-volume made it checkable, and the Punjabi volume never labels a first or second.

### Known remaining gap, measured

The same defect wearing different words — **99 sites across 79 files**: *"the
course"* / *"this curriculum"* / *"this series"* not preceded by "in" (65), *"in this
arc"* used for a cross-volume set (27), invisible-set ordinals like *"the fifth
language to close the loop"* (3), and *"every track"* / *"this course covers"* (4).
Counted rather than left to be rediscovered. The guard does not match them yet, for
the same reason it did not match this class until today.

### Fixed — the pointers the last guard could not see (HL-C50)

The previous change closed cross-volume *lesson ids* and left a gap it named out
loud: pointers phrased in prose, which no pattern then in place could match. This
closes it — **72 pointer sites across 59 authored files** (53 lesson sources and 6
handwritten chapters).

- **Second-person memory claims aimed at another language**: *"you learned in
  Hindi"*, *"you met in Tamil"*, *"You may remember from Latin's colour lesson"*,
  *"You've met a genuine dog-word mystery in Spanish"*. A reader holding one volume
  has met none of it.
- **Pointers at another volume's material**: *"the Spanish track"*, *"the Tamil
  book"*, *"German's lesson on you"*, *"the Hindi lesson on ऋतु"*, *"Telugu earlier
  in this arc"*, *"every other language in this arc"*.
- Every one keeps the cross-language **fact** and resolves it to the **language and
  the word** — *"the same worldview inside Tamil's pōy varugiṟēṉ"*, not *"the
  worldview you met in Tamil"*. Nothing was cut.
- **Future-tense pointers count too** — *"You'll meet these twelve names again in
  Spanish"*, *"the words you'll meet in Kannada"* — and so does *"earlier in this
  arc"* with no memory verb in front of it — including with an adjective wedged in
  (*"elsewhere in this **entire** arc"*, verbatim the phrasing fixed in Kannada while
  its Malayalam sibling was left standing). Plural material nouns too: *"the Spanish,
  Italian, French, and Portuguese **tracks**"*. All were missed by earlier sweeps of
  this same change.
- **Five of these live in handwritten chapters**, not generated ones — `bengali`,
  `kannada`, `telugu` ch01, `hindi` ch03, `latin` ch01 carry no `GENERATED FILE`
  header, so the `.tex` *is* the source there. Checked before editing, both ways.

### Changed — the guard now covers the phrasings, and the frontmatter

- Extended from `.tex` alone to **chapters plus lesson sources**, so frontmatter is
  held to the same rule. Six patterns replace one: a memory verb aimed at another
  language, `the <lang> <material>`, `<lang>'s … <material>`, and the original
  "in this course". Each proven to fire on a reintroduced defect.
- Two false-positive classes it must **not** flag, both found by running it:
  *"seen Hindi borrow umr **from Arabic**"* — a borrowing source, not a locator, so
  bare `from <language>` no longer counts; and *"unlike how closely Kannada and
  Telugu **track** Tamil"* — `track` is a verb there, so the material nouns require
  an article or possessive.
- **The lesson surface needed un-escaping first.** `canonicalLessonSource` returns
  JSON, where a line break is the two-character escape `\n` and not a newline. So
  `\s+` could not cross a wrap — a pointer split as `"the Spanish\ntrack"` was
  invisible, and one real defect (`FR-C03-de-rien`) hid there — and `[^.?!\n]`
  bounded nothing, leaving the 60-char window free to jump paragraphs on exactly the
  surface that had just been added. One `.replace(/\\n/g, "\n")` fixes both.

### Known remaining gap, measured

Bare *"in this course"* (40 lessons) and *"in this curriculum"* (8) are a coherent
class of their own and the next item — named here with a count rather than left to be
rediscovered. The guard deliberately does not match them: one muted on 48 files the
day it lands is worth less than one that holds.

### Note — five claims this change invented, and then withdrew

Replacing a pointer means writing a new sentence, and a new sentence can be false in
ways the old one was not. Five were caught in review before landing:

- *"the one European blue that is not a Germanic loan at all"* — Spanish's `azul` is
  Arabic too, and the corpus says so two files away.
- *"Latin's own vocabulary family for age and vigour"* — no such family. The second
  attempt, *"the sense narrowed to force alone"*, was **also** wrong and contradicted
  the same lesson's own body three lines down. The body already said it: the two words
  split toward different senses.
- *"the PIE/`nox` contrast is not independently attested"* for Kannada and Telugu — it
  is the identical Sanskrit word. The gap was in the curriculum, not the scholarship.
- *"the boundary-blur Malayalam shows when its TIME word widens toward AFTERNOON"* —
  backwards. Malayalam is the corpus's explicit counter-example (*"keeps 'noon'
  unwidened"*); the widening is Telugu's.
- *"svāgatam survives unchanged in Sanskrit itself"* — Sanskrit is the ancestor. A
  word cannot survive in its own source.

Every one has the same shape: a statement about **the books** rewritten as a statement
about **the world**. The pointer was the only thing making the original true, so
deleting it silently widened the claim. Three review passes were needed, and each
found defects the one before it had not.

### Fixed — one volume no longer cites another (HL-C50)

- **Nine cross-volume citations across 9 lessons** (8 distinct foreign ids): Malayalam
  cited `TE-C27/28/29`, Hindi cited `AR-C27` and `TE-C25`, Telugu cited `LA-C33` and
  `KA-C28`, and a German frontmatter cited `ES-C14`. A reader
  holding one language's book owns exactly that book; none of those pointers resolve.
  The cross-language **fact** beside each one is the point of the comparison and always
  stayed — only the pointer to another volume went.
- **An unanswerable recall question.** `HI-C28` asked *"What Arabic phrase, already
  taught in this course, shares सुबह's exact root?"* — a Hindi chapter quizzing the
  Arabic volume. Reworded to ask about a phrase stated earlier in the same lesson.
- **Eight "you've already met it, in <other language>" claims** in Hindi, Telugu,
  German, Tamil and Kannada chapters, including a printed section heading. The connection is real and
  is now presented as a connection rather than as recall the reader cannot have.
- **`TA-C25` was never cross-volume**: both components are Tamil, and an over-eager
  edit had replaced the correct pedagogical claim with the word "attested". Reworded to
  "two words you already have" — in-volume, and true.
- **A gloss that this change itself made false.** `HI-C28` said ṣabāḥ "sits inside the
  Arabic *good night* phrase". It sits inside the *good morning* phrase; the good-night
  phrase carries `tuṣbiḥ`, same root ص-ب-ح, different word. The inherited wording was
  ambiguous and wrong, and a first pass had sharpened it into a crisp falsehood. A token
  replacement needs a truth check, not only a grammar check.
- **A forward reference introduced by a fix.** Naming the answer "Telugu's സുപ്രഭാതം" in
  Malayalam script collided with a Malayalam headword taught three lessons later. Written
  in Telugu script — which is what the answer line beneath it already used.

### Added — guards, scoped to the defect class

- `standalone-book.test.ts` grew three: a foreign `XX-CNN` in a chapter, the same in a
  lesson's **frontmatter** (which never reaches the `.tex`, and is where the German
  instance hid), and an "already met" claim carrying an **out-of-volume locator**. The
  last is deliberately context-sensitive: ~86 chapters legitimately say "already met in
  Chapter 24", and a guard that bans the phrase outright would ban the ramp's own
  callbacks. Each was proven to fail on a reintroduced defect before being trusted.

### Fixed — the book no longer sends its reader into a git checkout (HL-C50)

- **105 handwritten chapters printed a repo path at somebody holding a PDF**:
  *"Practice lessons: `lessons/AR-C01-*.md`"* (99) and *"Companion lesson:
  `FA-C02-esm-e-man.md`"* (6). `book-cli.ts` already stated the principle — "a reader
  holding the PDF cannot follow a link into a Git repository" — and drops relative
  links for exactly this reason. The handwritten chapters had never been held to it.
- **The worst instance was on a title page, and a chapter-only check could not see
  it.** Japanese and Chinese printed *"Companion practice lessons live alongside this
  book at `code/learning/human-languages/japanese/lessons/`"* on the **title page** —
  more prominent than any chapter clause. `loadBookCorpus` records `entrypoint` as a
  path and never reads it, so the first version of the guard test passed green while
  the defect sat on page one. The test now reads `book.tex` too.
- Chinese's printed "Sources" backmatter also cited `data/scripts/chinese.json` by
  repo path and sent the reader to "the companion Markdown lessons". The source is
  now named without the path, and the dangling pointer is gone.
- Zero generated files touched: 416 chapters, 311 generated, 105 modified — the
  intersection is empty, and the change set is exactly the complete set of handwritten
  chapters.
- The guard covers **all** chapters including generated ones, so it is a regression
  test on the generator too, not only on handwritten prose.

### Added — Russian chapter 3 gets the capability it never had

- It was the **only generated chapter with no opening**, because `russian/chapters.json`
  had entries for chapters 2, 4 and 5 and skipped 3. The book could not describe a
  chapter the ledger did not know about.
- `canDo` names what the chapter title already promised — *"Six Verbs, and the One You
  Never Say"* — which is `быть`: Russian drops the present-tense copula, so the verb is
  learned in order **not** to say it.
- The payoff is `RU-C03-idti`, and every atom in its `assesses` list is taken from that
  lesson's own block directives. None is invented, and none is borrowed from a lesson
  that does not assess it.
- **It is an honest, declared regression on one metric.** The chapter has no terminal
  practice lesson, so the payoff is the last lesson by sequence and assesses 6 of 18
  atoms — below the 0.5 representativeness floor, taking the corpus 24 → **25**. That is
  recorded in the chapter's own non-printed `payoff.note` and pinned with its reason. A
  chapter with an opening and a thin payoff is better than one with neither; HL-C25
  exists to author real payoff lessons.
- Corpus: declared chapters 317 → **318**, chapters without a capability 99 → **98**.
  Every generated chapter now has an opening, so the chapter-intro test's by-name
  exception list is empty — and it is still a by-name check, not a count, so a future
  chapter without a capability is named rather than silently tolerated.

### Fixed — a chapter's fingerprint now covers the capability the book prints

- `canonicalChapterHash` covered lessons only, so `chapters.json` was **invisible to
  the fingerprint**. CI still caught a stale chapter — `book-cli --check` compares
  full text — but `core/generated-book-hashes.json` came out **byte-identical** after
  a capability edit, so `language-ladder`'s `bookHashStatus` reported a genuinely
  stale `.tex` as *synced*. The README's claim that the fingerprint "detects drift
  between book and app inputs" was false for that input class; it is now true, and
  the sentence says exactly what is covered.
- **Only the two fields the book prints are hashed** (`canDo`, `payoff.summary`).
  Hashing the whole capability would make `payoff.note` — deliberately non-printed
  tooling prose — regenerate every chapter carrying one, churn with no reader-visible
  cause. A fingerprint covers what the artifact SHOWS, no more.
- The capability argument is **optional**, and the narration export passes none: it
  builds a spoken script from lessons alone, so a capability edit must not churn 789
  narration files that cannot have changed. Verified — this change touches 310 book
  chapters and zero narration files.
- Russian chapter 3, the one generated chapter with no capability, hashes exactly as
  before. Adding the opening did not renumber chapters that have no opening.
- **The browser app was updated in the same change, and had to be.** `language-ladder`
  reproduced only the lesson half via `combineLessonHashes` — correct while that WAS
  the whole fingerprint. Folding the capability in without giving it a seam would have
  turned "always synced" into **"always stale"**: the same broken signal, inverted, on
  every lesson in every chapter. Pre-push review caught it with 188 of its 189 tests
  failing. `combineChapterHash` is now exported over already-computed lesson hashes —
  the app has no `ParsedLesson`, since it globs lesson sources rather than using the
  Node-only loader — and the app globs `chapters.json` the same way it globs lessons.
- The printed check is gated on `canDo`, matching `chapterIntro`'s own condition
  exactly: a capability with no `canDo` prints nothing, so it must hash as though
  absent. Otherwise the fingerprint would claim a difference the reader cannot see.

### Added — every generated chapter opens by saying what the reader will be able to do (HL-C49)

- **Russian chapter 3 is the one generated chapter with no opening**, because it has
  no HL05 capability entry at all. A chapter with no `canDo` gets no opening rather
  than an invented one; the gap is capability debt the gap report already counts, and
  the test names it so it shrinks visibly instead of hiding behind a number.
- **288 of 407 chapters opened on a bare title** — `\chapter{}`, `\label{}`, straight
  into the first lesson section. Nothing told the reader why they were there. All
  **302 generated chapters** now carry a short opening, and **all 302 had the data
  already**: every one has a `canDo` in its HL05 capability ledger.
- **Derived, never authored.** `book.ts` composes the opening from `canDo` and
  `payoff.summary`. 302 hand-written intros would be 302 places to drift from the
  lessons they describe, and the generated file says at the top that editing it is
  pointless. `canDo` is quoted verbatim, so the book and the ledger cannot disagree
  about the same sentence.
- **It must stand alone in English**, per HL09 §8 — the book is a standalone artifact
  and English is its only requirement. Naming a *source* language is not a violation
  and is the point of the book ("negro inherited from Latin", "trace *hermano* through
  *germānus*"); naming another **track of this course** is, because it dangles for a
  reader holding one PDF. One real violation was found and fixed at source: Telugu
  ch11's payoff said "the borrowed blue every language in this course now shares".
- The blurb that used to sit here explained how the chapter was *produced*. Removing
  it was right; leaving nothing was not.

### Fixed — the reconstruction asterisk was being deleted, turning reconstructions into attested forms

- `renderInlineMarkdown` reads a bare `*` as an italic opener, so `PIE *ne` printed as
  `PIE ` with the rest of the sentence italicised. In five chapters across German,
  Hindi and Telugu that **silently converted a reconstructed form into an attested
  one** — a false etymological claim, in the part of the book that exists for
  etymology. Lesson authors already wrote `\*`; the ledger authors did not, and
  nothing warned them. Escaped at source, with a test.

### Fixed — four books explained their own build system to the reader

- Four `payoff.summary` fields ended with a note addressed to the gap report:
  *"Chapter 17 has no terminal practice lesson, so the payoff is the last lesson by
  sequence (4 of 12 atoms, below the 0.5 floor)."* Printing that under the chapter
  title broke the very rule that got the old blurb removed. Moved to a
  non-printed `payoff.note`; a test rejects it returning.

### Known follow-up

- `canonicalChapterHash` covers lessons only, not the capability. CI still catches a
  stale chapter — `book-cli --check` compares full text, and the workflow's path
  filter includes `chapters.json` — but `core/generated-book-hashes.json` is
  byte-identical after a capability edit, so `language-ladder`'s `bookHashStatus`
  reports a genuinely stale `.tex` as synced. Folding the capability into the hash is
  the fix, and it regenerates every chapter, so it ships separately.

### Added — a track must EARN a level, not touch one (HL09 §3.1)

- Add `src/level-gate.ts`. The gap report now publishes two numbers per track where
  it published one:

      levels: 650 pre-A1, 297 A1, 186 A2; 148 unmapped (88% placed)
      levels ATTAINED (HL09 §3.1): none; 22 tracks touch a level they have not attained

- **This is the gate that would have caught "Spanish reaches A2".** Nothing lied:
  `TrackLevelCoverage.reach` is documented as *the highest level this track has any
  lesson at*, and that was true. The mistake was letting a number that means
  **touches** be read as **means**. One lesson pointing at one A2 node moves `reach`;
  it is nowhere near enough to sit the exam.
- `touches` keeps the old meaning. `attained` is the highest level where all four
  §3.1 criteria hold at that level **and every level below**: every spine node
  realized, cumulative vocabulary met, no lesson over the atom budget, every atom
  revisited twice. **Zero of 22 tracks have attained even pre-A1.**
- Spanish is *in progress at pre-A1*: **44 distinct headwords at or below pre-A1
  against a 300 target** (shortfall 256), plus 92 atoms revisited fewer than twice.
- **Every criterion is scoped "at or below the level", and getting that wrong was the
  first version of this module committing the exact error it exists to catch.** The
  initial implementation measured whole-track vocabulary (Spanish 138) against a
  per-level cumulative target, and applied the atom-budget and reinforcement criteria
  track-wide — so Hindi's single over-budget lesson, which sits *above* pre-A1, blocked
  pre-A1, making criterion 3 unfalsifiable at the bottom of the ladder. Security review
  caught it; the honest pre-A1 vocabulary is **44**, not 138.
- Criterion 4 counts atoms revisited **fewer than twice**, per §3.1 — not "never
  revisited". The looser reading hid 51 of Spanish's 141 failures.
- Vocabulary counts only `CONTENT_TYPES` lessons. Counting every lesson type credited
  drill titles and grammar labels as vocabulary — `(practice)`, `qu-`, `fact or wish?` —
  25 of Spanish's 138.
- A level with **no authored spine nodes fails** criterion 1 rather than passing it
  vacuously. `spine.json` has zero B1-C2 nodes, and "no node is unrealized" is not
  "every node is realized" — the same touches-vs-means error, one level up.
- **Failures name the criterion and the shortfall**, not a bare `false` — a boolean
  would move the argument rather than settle it. `vocabulary: teaches 138 distinct
  headwords against 300 for pre-A1, shortfall 162`.
- The gate stops at the **first** failing level, because the criteria are cumulative:
  a level above a failing one is unreachable by definition.
- Vocabulary targets live in `LEVEL_VOCABULARY` and are **editorial** per §10 —
  conventional working figures for CEFR receptive vocabulary, not a claim about any
  awarding body's syllabus. They are named so a failure can cite what it was measured
  against.
- Absent, not empty, when the caller supplies no policy: *not measured* and *attained
  nothing* are opposite facts, and a test pins that distinction.

### Fixed — the CLI had never once printed the level figures

- `report-cli` never passed `curricula` or `spine` to `buildCurriculumGapReport`, so
  the `levels` section has been silently absent from every CLI run since HL-C10
  shipped it. Both `levels` and the new gate now render. The section existed, was
  tested, and was invisible to anyone reading the report.

### Changed — 17 R1 reinforcement windows closed in Spanish chapters 3-6 (HL09 step 3)

- Records practice the lessons **already do**. 17 atoms across 11 lessons gain an entry
  in `practises.knowledge` **and** in the `assesses=[...]` directive of the specific
  body block that exercises them.

      corpus R1 misses      766 -> 749   (exactly the 17 wired)
      corpus never revisited 767 -> 755   (12; five already had a distant revisit)
      Spanish never revisited 102 -> 90 of 199

  Those are the figures on the corpus of the day. A verb tranche landed on main in
  parallel, so the committed pins read 1599 atoms / 745 never revisited / R1 778;
  what this change is accountable for is the 17 windows and the 12-atom move.

- **Only 17 of the 58 R1 misses in these chapters could be wired.** The other **41 are
  genuine absence** — no lesson in the window touches the atom at all, so there is no
  practice to record. That is what HL09 §7.2 predicted, and it is the honest result:
  a `practises` entry the prose does not back is worse than an open window.
- **A frontmatter-only edit was tried first and rejected by the schema.** Adding an
  atom to `practises.knowledge` without declaring it in a body block fails validation
  with `schema-v2-block-assessment-missing`. That rule is the schema enforcing HL09
  §7.2's honesty principle directly: **you cannot claim practice without pointing at
  where it happens.** The rejected attempt was reverted, not worked around.
- Placement is evidenced, not guessed. Each atom went to the block containing the
  drill or recall that exercises it — mostly `## Guided Practice` and
  `## The word, taken apart`. A "what you've learned" bullet was **rejected as
  practice** five times during the audit; a recall *task* ("order the three *hasta*
  goodbyes by time") was accepted, because it cannot be done without the words.
- **R2 is unchanged at 1107, and that is correct** — closing a near window does not
  close a far one. R2, R3 and R4 need dedicated `review` lessons, per §7.2.
- **Open question for the project owner: 15 of 18 `ES-ETYMON-*` atoms could not be
  wired.** This is systematic rather than eighteen oversights — an etymon is cited when
  introduced and never re-cited; only `hasta` comes back. Either etymon atoms should be
  exempt from the retrieval schedule, or lessons should re-cite earlier etymons the way
  they re-use vocabulary. Not decided here.

### Changed — Spanish's `sequence` numbers are renumbered on a clean 10-spaced run

- Every one of Spanish's 148 sequenced lessons is renumbered to **10, 20, 30 … 1680**,
  in the same reading order it already had. **Relative order is unchanged**, and so
  is every measurement derived from it: forward prerequisites 5, forward reviews 6,
  forward references 99, atoms 199/102. Byte-identical answers, different integers.
- **Why it needed doing.** HL09 step 2 had to fit chapters 7–18 into the 129 integers
  between 510 (chapter 6's end) and 640 (chapter 19's start), because chapters 19–33
  were already sequenced at 640–845. That forced a spacing of **2**. Gap census before:
  51 gaps of 2, 33 of 5, and a scattering of 3s and 4s. After: **147 gaps, every one of
  them 10.**
- **Chapter 7 now has room.** Its six lessons are still unsequenced pending the owner's
  ruling on their order, but the renumbering reserves **210 numbers — 21 slots — between
  chapter 6 and chapter 8** for six lessons plus the splits they will need. Previously
  the gap was 10, which would have forced a second renumber the moment chapter 7 landed.
- Safe by construction, and verified rather than assumed: the security review of #10047
  confirmed **nothing consumes a sequence's absolute value** — every comparison in
  `ramp.ts`, `book.ts`, `modality.ts`, `hash.ts` is relative, and the only absolute
  predicate is `curriculum.ts`'s `Number.isInteger(sequence) && sequence > 0`. The values
  are persisted verbatim into three generated artifacts, so this is a regeneration event,
  and the byte-exact `--check` CLIs fail loudly rather than silently on a stale one.
- Diff shape confirms the claim: lesson files changed **only** in their `sequence:` line,
  and the 21 regenerated book chapters changed **only** in their `canonical-source-hash`
  comment. No rendered content moved.

### Changed — Spanish has a declared reading order (HL09 step 2)

- 50 Spanish lessons across chapters 8–18 gain a `sequence:`, recovered from
  evidence rather than invented: the `Next: …` sentence ending each lesson's
  Wrap-up Recall, corroborated by `prerequisites:` and `reviews_of`.
- **26 of Spanish's 31 "forward prerequisites" were never real.** With no declared
  order the walk fell back to sorting alphabetically inside a chapter, which put
  `beber` before `comer` and then reported `beber` as depending on a later lesson.
  Declaring the true order removed them:

      Spanish              before   after
      no sequence              56       6
      forward prerequisites    31       5
      forward references      143      99

  Corpus-wide: 565 → **515** unsequenced, 271 → **245** forward prerequisites,
  331 → **300** forward reviews. Spanish's atom figures are unchanged by the
  ordering itself, as they must be — ordering moved no content; the corpus totals
  moved only because a verb tranche landed on main in parallel.
- **Chapter 7's six lessons are deliberately left unsequenced.** `curriculum.json`
  says comer → beber → qué → vivir → dónde; the prose `Next:` chain **and**
  `ES-C07-beber`'s own `reviews_of` say comer → vivir → beber → qué → dónde. Under
  the ledger's order, `beber` reviews a lesson that has not happened. Guessing
  would bake a false ramp into every later measurement, so they wait for a ruling.
  A test pins exactly which six remain, so this cannot be forgotten.
- Chapter 18 is the weakest recovery: none of its ten lessons carries a `Next:`
  line, so its order rests solely on `prerequisites`/`reviews_of`. Those happen to
  form one clean chain, but with no prose corroboration.
- **Known remainder: the numbering is cramped.** Chapters 19–33 were already
  sequenced at 640–845, so chapters 7–18 had to fit between 510 and 640 — 129
  integers for 56 lessons. Spacing is therefore **2**, not the intended 10, leaving
  almost no insertion room in a track meant to grow from 146 lessons to thousands.
  Renumbering the whole track by 10s is mechanical and should follow.

### Added — does the course have a memory of itself? (HL09 step 1)

- Add `src/continuity.ts`: `measureContinuity` measures the three things a
  per-lesson budget cannot see, published in the gap report's new `continuity`
  section. The ramp budgets measure how big each *step* is; this measures whether
  the steps hold together.

      order: 565 lessons with no declared sequence across 19 tracks;
             271 prerequisites and 331 reviews pointing forward
      reinforcement: 746 of 1469 atoms never revisited (51%);
             missed windows R1 745, R2 1068, R3 649, R4 132
      forward references: 509 uses of material a later lesson teaches

- **You cannot review a lesson that has not happened yet**, and **331** do. A
  forward `reviews_of` cannot close a reinforcement window — it names lessons, not
  atoms — but it is still an authored claim about order, and a claim pointing
  forward is wrong on its own terms. `ES-C07-beber` reviews `ES-C07-vivir`, which
  `curriculum.json` places *after* it.
- **Order comes first because everything else depends on it.** 565 lessons carry
  no `sequence`, so their reading order exists only inside hand-typed LaTeX —
  Spanish 56 of 146, French **64 of 73**. A ramp whose order is unknown cannot be
  verified at all, so every other number here is provisional until this is zero.
- **51% of taught atoms are never practised again.** HL00 specified the schedule
  (N+1, N+3, N+7, N+15), defined a `review` lesson type to carry it, and named
  `session-map.md` as the artifact that verifies it. The corpus has **zero**
  `review` lessons and a session map covering 3 chapters of 33. The schedule was
  specified and never built.
- The measurement reads `practises.knowledge`, **never `reviews_of`** — which 144
  of Spanish's 146 lessons set, and which cannot close a window because it names
  *lesson ids* while atoms live in another namespace. Measuring that field would
  report a corpus that reinforces beautifully and teaches nothing twice.
- Windows are judged **only where the track is long enough to contain them**. A
  25-lesson track missing R4 has not failed; it has not got there yet.
- **Forward references are proved, not guessed.** A word is reported only when a
  *later lesson's own headword* teaches it, so the finding carries its own
  evidence and cannot false-positive on ordinary English prose. It reproduces
  what a human reviewer found by reading: `ES-C07-beber` rewards the learner with
  *"Como pan y bebo agua"* while **`pan` and `agua` are chapter 26**, and
  `ES-C08-practice` drills `diecinueve` in a chapter that taught 1–10.
- Three false-positive classes were found by censusing the output rather than
  guessing, and each is excluded on principle: **single-character headwords** from
  `writing` lessons (a Cyrillic `е` or a Devanagari mātrā matched in every lesson
  of its script — five scripts' worth), **pattern notation** like `e→ie`, and
  English collisions like `once` (18 hits) — only lessons whose type is `word` or
  `phrase` create a matcher at all.
- Honest limit, stated because it changes how the number reads: a word the course
  **never** teaches anywhere is invisible here. Chapter 7's `¿Algo más?` and the
  untaught `un`/`una` do not appear, because nothing in the data says they are
  target language. 509 is a floor.
- Report-only, per the HL05 precedent: the debt predates the measurement.
- `readingOrder`, `frontmatterList` and `introducedAtoms` are now exported from
  `ramp.ts` and shared. Two independent orderings that drifted apart would make
  the two reports disagree about which lesson comes first, silently.

### Added — the ramp now includes the script (HL-C18C)

- Add `measureScriptRamp` to `src/ramp.ts`, and two budgets to `core/chapter-policy.json`:
  `maxNewGlyphsPerLesson` (**3**) and `maxNewScriptSystemsPerLesson` (**1**).
- **The atom budget was measuring one of two burdens.** `maxNewAtomsPerLesson` counts
  units of *meaning*. `HI-W01-shirorekha-na-ma` declares **one** atom and puts **twelve**
  new Devanagari glyphs on the page, and passed cleanly for a whole release. It is not an
  outlier: **61 lessons** exceed three new glyphs and **38 of them declare zero atoms**, so
  they read as maximally gentle while teaching up to a dozen new shapes. Decoding is a
  separate skill on a separate curve, and nothing was watching it.
- **3 is the corpus's own p90**, the same rule that justified `maxNewAtomsPerLesson` — not
  the observed max of 12, because a budget placed at the worst case is not a budget. The
  median non-Latin lesson introduces **zero** new glyphs, so this flags genuine spikes
  rather than taxing ordinary lessons.
- **Target script and the cousin layer are counted separately, and only the first is
  charged.** A Kannada Chapter 1 lesson showing the same word in Devanagari, Tamil, Telugu
  and Malayalam looks like a **34-glyph cliff** when the two are conflated; its actual
  Kannada load is **7**. Sister-script material is context for a reader who already knows a
  relative, and English is the only requirement for each book — so it is reported (119
  lessons, up to 26 foreign glyphs in one) and never penalised. What that footprint
  justifies is keeping the layer visually skippable.
- Counting rules, each load-bearing: charged **once**, in reading order, so revision is
  free; **Latin excluded**, or romanization would swamp the signal; **combining marks
  included**, because an abugida is mostly marks; **script digits included**, because ०१२
  is not readable to someone born to ASCII; and **`Script_Extensions`, not `Script`**,
  because ー is formally `Common` and the narrow property undercounts コーヒー by the
  mark that makes it a long vowel.
- `maxNewScriptSystemsPerLesson: 1` states the rule that you cannot introduce more than
  one script at a time. It flags **5** lessons, all Japanese Chapter 1, which opens kanji
  beside hiragana in its first lesson and adds katakana in its fifth.
- Report-only, per the HL05 precedent: the debt predates the measurement.

### Fixed — `measureRamp` was called by nothing but its own test

- The gap report now carries a `ramp` section, so `maxNewAtomsPerLesson` and
  `maxNewAtomsPerChapter` are finally read by something a human sees. They had been
  declared in `core/chapter-policy.json` since HL08 and enforced by nobody — policy in the
  sense that a sign is policy. The first published figures: **40** lessons over the atom
  budget, **25** chapters, with **572 lessons (47%) unmeasurable** because schema-v1
  declares no atoms.

### Fixed — three tracks silently resolved to the Latin script

- `LANGUAGE_SCRIPT` had no entry for **Gujarati** — which was the worked example in its
  own doc comment — so all 39 Gujarati lessons resolved to `latin`. Glyph-coverage
  validation looked Gujarati headwords up in the *Latin* inventory, and `romanization`
  fell back to the Gujarati headword itself, so the narration export published
  `"romanization": "આભાર"` — **Gujarati script in the field a speech engine reads as
  Latin.** Regenerating `lesson-modality.json` and the seven Gujarati narration chapters
  is the whole blast radius; no lesson content changed.
- **Chinese** and **Japanese** were missing from the same map and were saved only by
  shipping a `track.json` the loader prefers. A fallback that is wrong for some tracks
  fails only in the paths that skip the loader — which is exactly where a unit test lives.
  Completing the map removes the trap.

### Added — every lesson declares the level it builds toward (HL-C10)

- Add `src/levels.ts`: `CEFR_LEVELS` (`pre-A1` … `C2`), `deriveLessonLevel`,
  `summarizeLevels`, and `lessonsUpToLevel` — the filter a "gentle ramp to A1" edition
  applies. Published through the gap report's new `levels` section, and
  `core/exam-levels.json` records how the language-specific exams line up.
- **Derived, never authored.** A lesson sits in a realization-path segment, the segment
  names a spine node, the node declares a CEFR stage. HL08 refused to write `modality:`
  into 1,134 frontmatter files because that is 1,134 places for a computed fact to go
  stale; a level is the same kind of fact. Deriving it also means a track cannot claim A1
  by editing frontmatter — it has to actually realize the A1 spine nodes.
- **The measured answer to "how far is each track from Advanced":**

      pre-A1 657 | A1 307 | A2 0 | B1 0 | B2 0 | C1 0 | C2 0
      964 of 1,134 lessons placed (85%); 170 unmapped, all schema-v1

  **No track has reached A2**, and five (`chinese`, `japanese`, `persian`, `russian`,
  `urdu`) have not reached A1. A ramp-to-A1 edition would today contain **964 lessons** —
  as a filter over the one corpus, not a second corpus.
- Unmapped lessons report `null` and are **excluded** from a ramp edition rather than
  included by default. A wrong level is worse than a missing one: it would put material a
  reader is not ready for inside a book that promises a gentle ramp, so the honest failure
  is a shorter book.
- `core/spine.json` `stages` extends to `B2`, `C1`, `C2` so later tranches can declare
  their own stage. The project owner's direction is that the content reaches the most
  advanced level, gently, with page count explicitly not a cost.
- `core/exam-levels.json` maps CEFR onto the exams a learner would actually sit, and
  **every one of the 22 tracks is mapped — no gaps.** An unmapped track silently drops out
  of every level report, and a learner asking "what is A1 in Tamil?" deserves an answer.
- **What is recorded instead of a gap is the KIND of answer.** `basis: published` means the
  awarding body states the alignment (DELE, DELF/DALF, Goethe, CILS, CAPLE, TORFL, HSK);
  `research` means a widely-cited third-party correspondence (JLPT, Arabic ILR/ACTFL);
  `editorial` means this project's judgement — a working default to be corrected, never a
  claim about what a certificate is worth. A test enforces that every registered track has
  a mapping and a valid basis, so registering a track now requires answering the question.
- Judgement calls worth knowing: **Hindi** is anchored to the Dakshina Bharat Hindi Prachar
  Sabha ladder (Prathmic → Praveen), which is real and widely sat but built to spread Hindi
  within India rather than against CEFR descriptors. **Tamil** is mapped straight to CEFR
  because its diglossia makes any mapping unclean — this curriculum teaches the spoken
  register first, so A1 means the CEFR descriptor, not a claim about a Tamil examination.
  **Latin** takes CEFR too, with the honest note that CEFR is communicative and Latin is
  read; a reading-only ladder would fit it better. A second test requires a caveat on any
  mapping that names a specific foreign ladder without the awarding body's backing — it
  caught a bare Persian/AMFA correspondence during this change.


### Fixed — "detachable" and "is a writing segment" are two different things

- `DETACHABLE_BLOCK_TYPES` gains `script`, so a hands-free renderer may set aside the
  inline-letters section. HL00 makes it optional scaffolding by design — "a reader who
  already knows the script skims that section" — and nothing later in the lesson depends
  on having read it.
- **This required separating two ideas the model had merged.** `writingSegments` was
  computed as `blocks.filter((block) => block.detachable)` — named for writing, filtered on
  detachability. That was harmless only while `writing` was the sole detachable type. The
  moment a second type joined, every inline-letters section counted as a writing segment,
  which set `hasWritingBlock` and dragged the lesson to `pen`: **`pen` 53 → 309, and 276
  reported "writing segments" that teach no writing at all.** Detachability is about what a
  renderer may skip; pen-ness is about what the learner's hand must do.
- `writingSegments` now filters on `block.type === "writing"`, and a new
  `detachableSegments` carries what a hands-free view sets aside — a superset.
- **Result: the book stays honest and the driver gets more.** Whole-lesson modality is
  unchanged (`voice` 726, `sight` 355, `pen` 53) because the printed book really does show
  glyphs; the core — what the driving edition reads — is **972 lessons, 86%**, above even
  the 84% that stood before the inline-letters section was classified honestly.
- `drivablePercent` is derived from `coreVoice` and now legitimately differs from
  `voice / totalLessons`. The invariant test was updated to assert the correct relationship
  rather than the coincidence that held while core and whole were always equal, and gained
  two more: the whole-lesson partition still closes, and `coreVoice >= voice` always
  (detaching can only help).
- A chapter whose only obstacle was a script section is no longer blocked; the gap
  report's blocked-chapter fixture was moved to a four-column paradigm, which the
  lineariser genuinely refuses, so the test still proves a real blocker gets named.
- **Next slice:** the manifest still publishes the conservative whole-lesson figure (64%)
  while the gap report publishes the core (86%). `coreModality` is the additive key HL-C44
  reserved for exactly this; emitting it and flipping `features.blockModality` closes the
  gap.

### Changed — the inline-letters section is a `script` block, honestly

- `## The letters in this word` — HL00's inline-letters section, used by **240 lessons
  across 12 tracks** — parsed as `unknown`, which schema v2 rejects. That single gap
  blocked the v2 migration for every Indic track at once. It now parses as `script`,
  which is what it has always been: the place a word lesson teaches the glyphs that word
  needs.
- **This costs 20 points of drivable share (84% → 64%) and that is the point.** A glyph
  shape cannot be read aloud, so the previous number advertised a driving edition that
  would have narrated "ब plus the o-mātrā" at somebody on a motorway. Corpus moves
  `voice` 957 → 726, `sight` 124 → 355, `pen` unchanged at 53, unstartable chapters
  44 → 92.
- **The loss is recoverable and the route is known.** HL-C41 gave `writing` blocks a
  `coreModality` so a hands-free view can set them aside, and the inline-letters section
  is detachable in exactly that sense — HL00 calls it optional scaffolding a fluent reader
  skims. Adding `script` to `DETACHABLE_BLOCK_TYPES` was tried and reverted here: the
  model currently conflates "detachable" with "is a writing segment", so script blocks
  began claiming a lesson needs a **pen** to read letters (`pen` 53 → 309) and reported
  276 writing segments that are nothing of the kind. Separating those two ideas returns
  the core share to ~86% with the honest label intact, and is the natural next slice.

### Added — HL-C10: the shared spine reaches above A1

- Add an **A2 tranche** of five spine nodes — `SPINE-SAY-WHAT-I-DO`,
  `SPINE-NEGATE-AND-ASK`, `SPINE-SAY-WHAT-I-WANT`, `SPINE-TALK-ABOUT-PAST`,
  `SPINE-TALK-ABOUT-FUTURE` — and the seven canonical concepts they own
  (`VERB-INFINITIVE`, `VERB-PRESENT-HABITUAL`, `VERB-NEGATE`, `QUESTION-POLAR`,
  `VERB-WANT`, `VERB-PAST`, `VERB-FUTURE`).
- **This unblocks the entire Easy-to-Advanced grammar arc, and nothing else could.**
  Schema v2 requires a canonical `spine_node`. Every one of the previous eleven nodes was
  an A1 social function — greeting, taking leave, counting to five — with nothing covering
  verbs or tense, so a lesson teaching a present tense had no node it could legally
  declare. The arc was unauthorable in v2 for all 22 tracks. It was found the hard way,
  by trying to migrate a Hindi verb lesson and discovering its chapter belongs to no node.
- All 22 realization ledgers declare where they stand on each new node. An unrealized node
  is recorded as `segments: []` **with `omits` naming every concept it is not yet
  delivering** — the validator requires this, and rightly: "we have not built this yet" is
  a recorded position, so the debt stays countable instead of being an absent key nobody
  can see. Today that is all 22 tracks on all five nodes; those numbers are the burn-down.
- The taxonomy grows 46 → 53 concepts. Each concept is owned by exactly one node, which
  the validator enforces, so a later tranche cannot quietly re-file a concept it wants.

### Added — HL-C03: the nine HL05 chapter gates, as measurement rather than judgement

- Add `src/chapters.ts` with all nine HL05 gates — `chapter-missing-capability`,
  `chapter-unknown-payoff-lesson`, `chapter-payoff-not-closed`,
  `chapter-payoff-not-representative`, `chapter-duplicate`, `chapter-title-drift`,
  `pattern-slot-not-closed`, `pattern-missing-production`, `pattern-multiple-atoms` —
  and publish them through the gap report's new `chapters` section.
- **Report-only, and that is the design, not caution.** 98 of the corpus's 377 book
  chapters carry no capability entry. Wiring these into `validateCurriculum()` as errors
  would have converted a measurement of pre-existing debt into 98 build failures on a
  corpus nobody had regressed. Per-track rollups carry a `clean` flag so a track flips to
  hard errors once its own debt is zero — the HL-V01 precedent, and the same reasoning
  that ships the LaTeX warning baselines unseeded.
- **The first published snapshot: 377 book chapters, 279 declared, 98 without a
  capability, 24 payoffs below the 0.5 representativeness floor, and zero unclosed
  payoffs, zero unknown payoff lessons, zero title drift, zero duplicates.** Three tracks
  — `chinese`, `japanese`, `latin` — are already clean and could flip to errors today.
- **`payoffsNotClosed` read 279 — every authored chapter — on the first run, and that was
  this module, not the corpus.** Introduced atoms live in a FLAT dotted frontmatter key
  (`introduces.knowledge`) plus block-level `hl-knowledge` directives; reading a nested
  `introduces: { knowledge }` object returns `undefined` for every lesson in the corpus,
  which silently empties the "taught so far" set instead of failing. The fix reads the
  union of both sources. A gate reporting total corpus failure is usually reporting on
  itself, and the pinned snapshot exists so that stays visible.
- The three `pattern` rules find nothing, because HL-C05 has not added the `pattern`
  lesson type yet. They are wired now so the first authored pattern is checked the moment
  it exists rather than being remembered later.
- Summary gains `chaptersWithoutCapability`, `chapterPayoffsNotRepresentative` and
  `chapterGateCleanTracks`, each `null` rather than `0` when a caller passes no ledgers —
  "not measured" and "measured, none found" are different facts.

### Changed — HL-C38: the generated books read as books, not as exports

- **`src/book.ts` gains one documented "book voice" section.** Lessons are
  authored as audio scripts (HL00) so a track can be recorded; the book view was
  printing those stage directions. It no longer does. The transformation is
  book-view only — `block.markdown` still holds every cue, and a future narration
  exporter must read it directly rather than reusing `bookVoice`.
  - `[PAUSE Ns]` is deleted. A reader paces themselves.
  - `[REPEAT xN]` becomes prose: *Twice through:* …
  - `[YOU <VERB>: …]` becomes a printed practice prompt. A run of bullets sharing
    one verb gets a single lead-in (*Say these aloud:*); a mixed or lone cue gets
    a per-bullet italic label (*Say it:*, *Write it:*, *Trace it:*). Twenty-eight
    cue verbs are mapped in `CUE_VOICES`, with a sentence-case fallback so an
    unmapped verb still prints as English. Writing and tracing prompts are real
    printed exercises and are never suppressed.
- **Printed block headings.** The internal block-type names are replaced from one
  table, `BOOK_BLOCK_TITLES`: `Guided Practice` → **Your turn**, `Wrap-up recall`
  → **Before you move on**, `You'll want to know first` → **What to know first**.
  The warm-up loses its printed label entirely and stands as the section's
  indented lead-in — several lessons share a chapter, and a bold `Warm-up.` five
  times on one spread reads like a worksheet. Headings the author extended with a
  descriptive tail are left untouched.
- **The chapter blurb is gone.** Every chapter opened with "This chapter is
  generated from the canonical micro-lessons used by Language Ladder. Each
  section stays independently resumable…". Books do not describe their build
  system.
- **Links: the book is a standalone artefact.** `absoluteBookLink` replaces
  `resolveMarkdownLink`. Absolute HTTP(S) citations (UT Austin, MSU, Wiktionary)
  stay live `\href`s; repository-relative destinations print their label with no
  link, because a reader holding the PDF cannot follow them. `sourceBaseUrl` is
  still required and validated in `book-generation.json` — it is that config's
  statement of where the curriculum lives — but it no longer reaches the
  renderer, so `BookGenerationTarget.sourceBaseUrl` and `MarkdownRenderContext`
  are removed.
- `bookVoice` and `bookBlockTitle` are exported for testing and reuse.
- Regenerated all 270 chapters. Source hashes are unchanged: no lesson file was
  edited, and `core/generated-book-hashes.json` is byte-identical.

### Added — HL08 narration export: the drivable course, out loud (HL-C16)

- Add `src/speech.ts`: the shared judgement of **what can be said aloud**. Markdown
  inline → words a voice can pronounce (emphasis, code fences, link destinations and
  the linguist's reconstruction asterisk removed; `→` `←` `·` given spoken readings),
  and Markdown tables → spoken utterances or a *reasoned refusal*. Both `modality.ts`
  and `narration.ts` import it, so "this lesson is drivable" and "the export can
  actually narrate this lesson" are the same question asked once.
- Add `src/narration.ts`: the pure narration builder. From the canonical lesson AST it
  produces typed segments — `speech`, `pause`, `repeat`, `prompt`, `table`,
  `table-skipped`, `activity` — plus the continuous plain-text script rendered from
  them. This is the **audio-script output HL04's one-source pipeline diagram has named
  since it was written and which nothing had ever built**.
- Add `src/narration-cli.ts`: `--write` / `--check`, modelled joint for joint on
  `book-cli.ts`. Writes `<language>/narration/chNN.txt` and `.json` for all 375
  chapters plus a hash manifest at `core/generated-narration-hashes.json`. `--check`
  compares byte for byte and exits 1, so a lesson edited without re-running the
  exporter fails the build instead of leaving a voice assistant confidently teaching a
  lesson that no longer exists.
- **`[PAUSE Ns]`, `[REPEAT xN]` and `[YOU …: …]` are preserved as structured
  directives, not flattened into prose.** Cue parsing is a depth-tracking bracket scan,
  because the corpus nests brackets inside cues for real
  (`[YOU SAY: the pattern — "[nā] [pēru]"]`), and a Markdown link that is not a cue is
  handed back intact rather than mistaken for one.
- **A `[YOU SAY: …]` cue is never treated as an answer key.** Cues become `prompt`
  segments with `scored: false`; only `hl-activity` contracts, compiled through
  `compileLessonActivities`, become `activity` segments carrying `acceptedResponses`.
  This is `activity.ts`'s own rule — runtime consumers use only the typed AST and never
  recover prompts or answers from learner-facing Markdown — and the narration export
  would have been the easiest place in the package to break it.
- **Tables are linearised, never dropped.** A two-column word→gloss table becomes
  *"नमस्ते means hello"*; a three-column table becomes labelled facts. A column with no
  heading is spoken as a bare value rather than refused, because `| Read | | Meaning |`
  — script, romanization, gloss — is the corpus's commonest practice-table shape and
  the blank heading is one a sighted reader does not have either. A run of pipe rows
  with no delimiter row is read as an unlabelled sequence for the same reason.
- **A table that cannot be linearised is spoken, not skipped**: the learner hears its
  size, its column headings, and why it needs eyes, and the lesson is marked `sight` so
  they are told before they start. `sight` and `pen` lessons still export in full,
  opening with a notice naming what they will need and which sections to leave until
  they have stopped.
- Target-script text carries its `romanization` alongside — *"خداحافظ (khodâ hâfez)"* —
  drawn from the **whole chapter's** headwords, so a lesson can pair a word a
  neighbouring lesson introduced. Pairing is whole-word only: the Arabic track teaches
  ا (*alif*) as its own lesson, and a plain substring replace turned سلام into
  `سلا (alif)م`, splicing the pronunciation guide into the middle of the word.
- Report `narration-block-unrenderable` when a lesson carries a table the export cannot
  speak yet claims `voice`, and `narration-activity-invalid` when an authored contract
  will not compile. Both are collected, never thrown — one bad directive must not
  silence a lesson.

### Changed — `maxLinearisableTableColumns` moves from 0 to 3

- The knob shipped at **0** in the modality slice on purpose: no lineariser existed,
  and claiming a table was speakable would have claimed a capability nothing
  implemented. The lineariser now exists, so the default is its measured value, **3**,
  and it is authored in `core/chapter-policy.json` (validated on load: an integer from
  0 through 16) rather than living only as a constant.
- Three, and not four, because that is where a table stops being a list of labelled
  facts a listener can hold — *"Language: Telugu. Hello: namaskāram. Source:
  Sanskrit."* — and starts being a grid whose meaning lives in the comparison *across*
  rows. The corpus's own four-column tables prove the point: `| | numeral | word | said |`
  has an unlabelled first column that means something only because of where it sits on
  the page. Measured over the 340 table-bearing lesson files: 99 are 2 columns wide,
  173 are 3, 60 are 4, and 8 are 5 or more.
- At width 3 the lineariser reads **371 of the corpus's 442 tables (84%)**, covering
  272 of the 340 table-bearing files. The corpus moves from **694 drivable lessons
  (63%) to 925 (84%)**. Of the 120 that still need eyes, 65 carry a wide table, 61
  point at the page in prose, 7 have a `script` block, and **52 need eyes for a wide
  table and nothing else** — HL08's table-remediation burn-down list, now measured.
- `modality.ts`'s `wide-table` rule no longer means "wider than N". It means *"the
  narration lineariser refuses it"*, which is strictly larger: a three-column table
  inside the limit is still unspeakable when its rows are ragged. Asking the exporter's
  own judgement is the only way `voice` can be a promise the export is able to keep.
- `report-cli.ts` reads the same policy width, so the published drivable percentages
  and the committed narration export can never be computed at different settings.
- `tableRowColumns` now delegates its cell splitting to `speech.ts`, so the count a
  lesson is judged on is produced by the same scan the narration is built from.

### Added — HL-C41 block-level modality: one lesson, two answers

- Add the `writing` lesson-body block type (`## Writing: …`), for a section that
  teaches the **hand** to form a letter — as against `script`, which teaches the
  **eye** to recognise one. It is the first and so far only **detachable** block type:
  nothing later in a lesson depends on it, so a renderer that cannot use a hand may
  set it aside and still deliver a coherent lesson.
- Derive modality at two scales. `LessonModality.modality` is unchanged and still
  describes the whole lesson — what the **book** signs. New `coreModality` describes
  the lesson minus its detachable blocks — what a hands-free view can deliver. New
  `coreDerived`, `coreReasons`, `blocks` (per-block `BlockModality`) and
  `writingSegments` expose the derivation. New `deriveBlockModality`,
  `lessonCoreText`, `isDetachableBlock`, `DETACHABLE_BLOCK_TYPES`,
  `strongerModality`, `weakerModality`.
- **This is why it exists, and it is not what an earlier framing assumed.** The
  project owner's ruling is that the book is a standalone artifact and keeps all
  writing content in full; a dictation-friendly edition is a *separate output view*
  over the same canonical source, exactly as the narration export is. `coreModality`
  is the metadata that view reads. It is a strict improvement for that view: today a
  lesson with any pen content is lost to a commuter wholesale, whereas block marking
  lets them take the voice core and defer only the segment.
- Sight cues and tables are now attributed to the block they occur in, so a cue inside
  a writing segment does not follow it out into the core, while a cue in ordinary prose
  still does.
- An authored `modality:` override **caps** the core, giving the invariant a hands-free
  view relies on: `coreModality` is never stronger than `modality`.
- `drivablePrefix` and `drivablePercent` now count the core; `coreVoice` and
  `lessonsWithWritingSegments` are published beside the unchanged `voice`/`sight`/`pen`
  counts so the book's numbers and the hands-free numbers reconcile in the gap report.
- New report-only finding `modality-writing-segment-not-separable`: a lesson that is
  not `type: writing` may carry one writing segment; several means it should be split
  or declared a writing lesson. `type: writing` lessons are exempt.
- **Measured no-op.** No track has authored an interspersed writing segment yet, so
  every lesson's core equals its full modality and no published number moves — the
  regenerated `core/lesson-modality.json` is byte-identical in its summary (1,133
  lessons, 725 `voice`, 64% drivable). Pinned as `coreVoice === voice` alongside
  `lessonsWithWritingSegments === 0`, so the first interspersed lesson has to break the
  equality deliberately. Deliberately *not* pinned as an absolute literal here: the
  corpus totals live in one place, `modality-manifest.test.ts`, against the generated
  manifest.
- `features.blockModality` stays **false**: this change derives block modality but the
  manifest does not yet emit block rows, and the flag exists precisely so a consumer can
  tell those two states apart.
- Amends [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md),
  which had assumed one modality per lesson.

### Changed — corpus pins moved by the Japanese track (HL-C40)

No source change: the Japanese track is content, and the package loaded it without
a code edit because `japanese/track.json` declares the script (the built-in
`LANGUAGE_SCRIPT` map was deliberately left alone, proving that path works). The
pinned corpus measurements moved, and each pin now records why:

- `registeredTracks`, `authoredBooks`, `schemas.tracks`, `books.tracks`: 21 → **22**,
  Japanese following Mandarin Chinese (HL-C39) as the 22nd track.
- `modality-manifest.test.ts`: `totalLessons` 1125 → **1133**, `voice` 724 → **725**,
  `sight` 348 → **355**, `chapterCount` 376 → **377**, `unstartableChapters`
  121 → **122**; `pen` stays 53 and the drivable share stays **64%**.
- `drivablePrefixTotal` does **not** move (558). Japanese ch1 opens on one of its
  seven `script` lessons, so the chapter's drivable prefix is zero — which is also
  why `unstartableChapters` gains one.
- The compiled-activity id list gains the eight `JA-C01-*` activities.

Seven of the eight Japanese lessons carry a `script` block and therefore derive as
`sight`. That is the honest classification — a kana or kanji shape cannot be read
aloud — and it was chosen over routing the same content through `input` blocks,
which would have held the drivable percentage flat by mislabelling it.

Added one integration test, `keeps the Japanese Chapter 1 mixed-script chain closed
and under five minutes`, which asserts the property rather than only the counts:
the same chapter carries a hiragana, a katakana, and a kanji headword; every lesson
is schema-v2 with exactly one compiled activity; nothing exceeds the duration
budget; and the plain and polite thanks keep distinct `register` values.

### Added — tone in the script data model, and a `pronunciation` lesson type (HL-C39)

Driven entirely by the Mandarin Chinese track, which was added as a scale test for
whether the curriculum model generalises outside Indo-European and Dravidian.

- `ScriptData` gains `tones?: Tone[]` and `toneSandhi?: ToneSandhiRule[]`.
  `Letter.tone` already existed and labels the tone a *character* carries, which is
  enough to tag a glyph and nothing more. It cannot say what tone 3 *is* (contour
  214, low and creaky), and it cannot express **sandhi** — a rule that changes a
  syllable's pitch because of the syllable *after* it while the characters and the
  printed pinyin stay identical. Every previously modelled script encodes
  pronunciation segmentally, and a segment always attaches to a glyph; tone is
  suprasegmental, so the existing shape did not stretch. `data/scripts/chinese.json`
  populates both fields.
- `EXEMPT_TYPES` gains `pronunciation`. No earlier track ever needed a lesson
  *about* sound, because segmental facts belong to letters and therefore live inside
  the word lesson that first uses that letter (HL00, "Pronunciation & Script:
  Inline, Never a Gate"). Folding Mandarin's tone system into its first character
  lesson pushed that lesson to 352 effective seconds, past the five-minute contract,
  and HL08's rule is to split rather than waive. `grammar` would have misfiled a
  sound rule as morphology; an unrecognised type would have produced a permanent
  validator warning. Like `grammar` and `etymology`, `pronunciation` is exempt from
  the cross-language concept join because its progression lives in knowledge atoms.

### Changed — corpus pins moved by the new track, never weakened

Adding a 21st track necessarily moves whole-corpus measurements. Every pin below
was updated with a comment naming this change as the cause; none was relaxed.

- `integration.test.ts`: registered tracks, authored books, schema tracks and book
  coverage 20 → 21; compiled activity ids 51 → 57. Duration violations and unknown
  prerequisites remain **0**.
- `cli.test.ts`: reported `registeredTracks` 20 → 21.
- `modality-manifest.test.ts`: total lessons 1,118 → 1,125; `voice` 719 → 724;
  `sight` 346 → 348; `trackCount` 20 → 21; `chapterCount` 375 → 376;
  `drivablePrefixTotal` 557 → 558. The `pen` count (53) and the corpus-wide drivable
  share (64%) are unchanged, because no Chinese lesson needs a pen and none carries a
  table. The two `sight` lessons are `ZH-C01-ni` and `ZH-C01-hao`, which each teach a
  character's components in a `script` block.
- **No `modality.test.ts` edit, and no Language Ladder test edit.** Both used to hold
  hard-coded track and corpus counts and were rewritten upstream to derive them —
  `modality.test.ts` now asserts size-independent invariants, and the Language Ladder
  suites read `LANGUAGE_ORDER.length` / `LANGUAGE_CHAIN.length` instead of the literal
  20. Registering a track no longer requires touching any of them, which is why this
  entry is shorter than the same entry would have been a week ago.

### Fixed — HL-C26: hand-written chapters are described, not generated

- Add a `handwritten[]` list to `core/book-generation.json` recording the **105**
  chapters that have a committed `book/chapters/ch*.tex` but no `targets[]`
  entry, with `title` and `label` transcribed from what each `\chapter{}` and
  `\label{}` actually declares. These are the hand-authored prefixes of nearly
  every book, written before the generator existed and mostly still schema-v1.
- The obvious fix — giving them `targets[]` entries — would have **destroyed
  them**. A target is not a description but an instruction: `generatedBookOutputs`
  renders every target and `--write` writes the result over the file at `output`.
  A separate array is used instead of a `generated: false` flag precisely because
  the two fail in opposite directions; `generatedBookOutputs` only ever walks
  `config.targets`, so nothing in `handwritten[]` can be rendered by a missed
  branch. The worst a mistake there can do is leave a chapter unchecked.
- Add `handwrittenBookChapters()`, which reads the list without rendering
  anything. `check:books` output is unchanged, byte for byte.
- `chapter-title-drift` previously **skipped** any chapter with no target, which
  left those titles verified by nothing. It now checks them against
  `handwritten[]`, and a new test fails if any ledger chapter is covered by
  neither list — so the assertion cannot decay back into a silent `continue`.
- Add tests that re-read every hand-written `.tex` to prove its recorded title and
  label were transcribed rather than invented, that the two lists never claim the
  same chapter, that no hand-written path appears in `generatedBookOutputs()`, and
  that every committed chapter file is accounted for by one list or the other.
- Add a check that every generation target's committed file opens with
  `% GENERATED FILE.` (true of 270/270 generated and 0/105 hand-written chapters).
  This is the only guard that catches a chapter *promoted* into `targets[]`, which
  by leaving `handwritten[]` escapes every membership-based check.
- Labels are recorded as declared, not normalised. Three conventions coexist — a
  bare `ch:greetings` slug, an ISO-code `ch:fa-`/`ch:la-` prefix, and a
  language-name `ch:persian-`/`ch:urdu-`/`ch:russian-` prefix — so Persian ch2 is
  `ch:persian-name` beside a generated `ch:fa-ask-and-answer-names`. Rewriting a
  `\label` breaks existing `\hyperref` cross-references, so the inconsistency is
  recorded in the backlog for a deliberate decision rather than silently fixed.

### Added — stroke-order provenance on `Letter`

- Add `StrokeOrderSource` and two optional `Letter` fields, `penLifts` and
  `strokeOrderSource`. A `strokeOrder` list names a letter's **parts** in writing
  order; it has never counted **pen-down runs**, but a numbered list of three
  reads to a learner as three strokes and two lifts. Tamil ம is the counter-
  example that forced the distinction: its prose listed three parts while the
  authored, font-checked pen path in Language Ladder's `strokes.ts` shows one
  unbroken stroke with zero lifts. `penLifts` records that number only where a
  verified path supports it — absent means *not verified*, never *none* — and
  `strokeOrderSource` carries the citation, URL, and the honest `variation` note
  for scripts (every Indic script, Arabic, Hebrew) that have no national
  standard. Both are optional, so every existing script file still validates.
- Document the parts-vs-strokes rule on `strokeOrder` itself, where the next
  author writing one will actually read it.

### Added — HL-C44 the modality manifest, so two editions build from one source

- Add `src/modality-manifest.ts` and `src/modality-cli.ts`, emitting
  `code/learning/human-languages/core/lesson-modality.json`. HL-C14 already derived
  `voice`/`sight`/`pen` per lesson and a drivable prefix per chapter, but only at
  runtime and only into the human-readable gap report — a paragraph of English is not
  something a book builder can filter on. This slice makes the derivation *data*, so
  the complete book, the app, and the forthcoming dictation-friendly driving edition
  (HL-C43) each filter the same canonical corpus rather than maintaining three copies.
- **Per lesson:** `id`, `language`, `chapter`, `sequence`, `modality`, `derived`,
  `drivable`, `reasons`, and the lesson AST's `sourceHash`. The three override fields
  (`authored`, `authoredReason`, `overridden`) are emitted only on the lessons that
  have them, rather than a thousand copies of the empty string. The monotone closure
  (`pen` implies `sight`) is deliberately *not* emitted: it is a three-entry lookup
  table, and restating it beside every pen lesson would add sixty kilobytes of
  duplicating `requiredChannels()`.
- **Per chapter:** the drivable prefix, `firstNonVoiceLesson`, the modality union,
  whether the whole chapter is drivable, and `drivableLessonIds` — the prefix spelled
  out in order, so a driving-edition renderer never has to re-implement "authored
  order" and quietly disagree with the generator about it.
- **Per corpus:** a `summary` pinned by tests — 1,096 lessons, 708 `voice`, 337
  `sight`, 51 `pen`, 65% drivable, 20 tracks, 375 chapters, 551 lessons reachable in
  the car once prerequisite order is respected, 199 fully drivable chapters, 121 that
  cannot be started by ear at all, zero overrides, zero chapterless lessons.
- **Designed for HL-C41's block-level modality to land additively.** Every lesson row
  is a JSON object, not a positional tuple. `modality` keeps its meaning permanently —
  the strongest channel the lesson needs *anywhere* — so a consumer that never learns
  about block modality keeps producing a correct, merely pessimistic driving edition,
  which is the safe direction to be wrong in. `coreModality` arrives as a new optional
  key beside it (`entry.coreModality ?? entry.modality` is correct before and after),
  and the header's `features.blockModality` flag says at a glance whether a build
  carries block data. The shape of the companion block records is deliberately not
  guessed here: an absent key is additive, a wrong key is a breaking change.
- **Nothing is authored.** The manifest is derived, exactly like
  `core/generated-book-hashes.json`. HL08 refused to add `modality:` to 1,096
  frontmatter files precisely because that is 1,096 places for a computed fact to go
  stale, and this artifact does not reintroduce the problem.
- Add `npm run generate:modality` / `npm run check:modality`, mirroring the
  `generate:books` / `check:books` contract: `generatedModalityOutputs()` returns a
  path → content map so `--write` and `--check` consume identical bytes, `--check`
  compares byte for byte and exits 1 on any drift, and the corpus is fingerprinted with
  `fnv1a64` from `hash.ts`.
- Wire `npm run check:modality` into `human-languages-books.yml` beside
  `check:books`. A stale manifest is not cosmetic: a lesson that gained a paradigm
  table would still read `drivable: true`, and the driving edition would tell somebody
  at 70mph to look at a chart. The `books-gate` job's name expression and pass/fail
  contract are untouched.
- Add `loadModalityManifest()` and `modalityManifestById()` to `loader.ts`, exported
  from `index.ts` with the manifest types. The index returns a `Map`, never a plain
  object: the keys come out of parsed JSON, and `index[lesson.id] = lesson` with an id
  of `__proto__` writes the prototype instead of a property.
- Ordering is total and null-last (track, chapter, `sequence`, id), so the file is
  byte-stable regardless of directory-walk order — otherwise `--check` would fail on a
  colleague's machine for no reason. The corpus fingerprint sorts by id rather than
  reusing `combineLessonHashes`, whose `sequence`-first ordering degenerates on the
  many lessons that carry no sequence (`Number(undefined)` is `NaN`, and every
  comparison against `NaN` is false).
- `safeOutput()` fails closed on path escape, checking containment *after* `resolve`
  rather than scanning the input string for `..`, and requires a `.json` extension so a
  mistake cannot land on an authored `.tex` chapter or `.md` lesson.
- 33 new tests: manifest round-trip, order-independent bytes, drift detection
  (including a byte-level reformat), the missing-manifest case, the full path-escape
  matrix, the `__proto__` index case, the additive-`coreModality` read, and the corpus
  summary pinned field by field. `modality-manifest.ts` reaches 100% statement
  coverage. No existing assertion was weakened.

### Added — HL08 modality and the drivable prefix (report only, no gates)

- Add `src/modality.ts`: a pure module deriving each lesson's required channel
  (`voice` / `sight` / `pen`) and each chapter's **drivable prefix** — how many of
  its lessons, in authored `sequence` order, are learnable by ear before the first
  that is not. Implements the first migration step of
  [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md).
- **Modality is derived from lesson type and block structure, never from `skills:`.**
  `skills` records what a lesson *develops*, not what it *requires*: 501 of the 531
  schema-v2 lessons declare `[listening, speaking, reading]`, yet *hola* is
  perfectly learnable by ear. Deriving from `skills` would have stamped roughly 95%
  of the corpus "needs eyes" and made the drivable course an empty promise. The
  rules are: `type: writing` → `pen`; otherwise a `script` block, a sight cue, or a
  table wider than the configured linearisable width → `sight`; otherwise `voice`.
- Modality is monotonic — `pen` implies `sight` — exposed as `requiredChannels()`
  and `unionModalities()`, and a chapter's modality is the union of its lessons'.
- `maxLinearisableTableColumns` defaulted to **0** in this slice: until HL08's
  narration exporter could linearise a two-column table into speech, no table was
  speakable, and claiming otherwise would let a learner silently miss content they
  were never told they had missed. (Superseded above: HL-C16 built the lineariser and
  the default is now 3.)
- Support an authored `modality:` frontmatter override. An override that
  *contradicts* the derivation requires a `modality_reason:`; unexplained overrides
  (`modality-unexplained-override`) and unrecognised values (`modality-unknown-value`,
  which falls back to the derivation) are collected across the whole corpus and
  reported once. Nothing throws, and nothing gates — the HL-V01 precedent.
- Add a modality section to `buildCurriculumGapReport()` and its text renderer:
  per-track `voice`/`sight`/`pen` counts, each chapter's drivable prefix, the
  chapters that cannot be started by ear at all, and the corpus-wide drivable
  percentage. New summary fields: `drivableLessons`, `drivablePercent`,
  `chaptersWithoutDrivablePrefix`, `unexplainedModalityOverrides`.
- Measured over all 1,096 lessons: **51 `pen`**, **7** carrying a `script` block,
  and among the remaining 1,038, **322 carry a Markdown table** — the single largest
  obstacle to a hands-free course, and far more tractable than the script.
  **694 lessons (63%) are drivable exactly as authored.** Track extremes: Bengali
  and Persian at 90%, Russian at 9%.
- `tests/modality.test.ts` covers every derivation branch, monotonicity, the
  override-plus-reason rule, drivable-prefix computation (including a chapter whose
  prefix is 0), and pins the corpus-wide drivable count as a regression. The pin
  exists because a parser change that renamed a block's `markdown` field would make
  every lesson scan clean and silently report a 100%-drivable curriculum.
- Divergence from HL08's recorded baseline, stated rather than tuned away: the spec
  reports 56 cue-bearing lessons and 695 drivable. The published `SIGHT_CUES` list
  matches 61 lessons and lands on 694. Every structural count reproduces exactly
  (51 / 7 / 1,038 / 322), so the gap is entirely in the cue list, whose exact
  contents the spec never recorded. The detector was left alone.

### Added — HL05 chapter capability layer (data only, no gates)

- Add `ChapterCapability`, `ChapterPayoff`, `TrackChapters`, and `ChapterPolicy`
  types for the chapter capability ledger specified in
  [`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md).
  A chapter was previously nothing but an integer stamped on each lesson, so
  nothing in the data model knew what a chapter was for and nothing could check
  that finishing one left the reader able to do anything.
- Add `loadTrackChapters()` and `loadChapterPolicy()` beside the existing
  `loadLanguageCurricula()`. Tracks without a `chapters.json` are **skipped, not
  defaulted** — an absent ledger means "not yet authored", which the gap report
  must be able to tell apart from "authored and empty". Inventing a placeholder
  would erase exactly the debt the report exists to measure.
- Add `core/chapter-policy.json` carrying the HL05 payoff-representativeness
  threshold and the HL08 gentle-ramp budgets, with the corpus measurements the
  values were drawn from recorded alongside them. Thresholds sit at the existing
  distribution: 3 new atoms per lesson (the current p90, flagging 52 lessons) and
  12 per chapter (just above the chapter p90 of 10, flagging 17).
- Add `spanish/chapters.json` covering Chapters 1–3 as the authored proof of
  shape. Chapters 4 onward are deliberately absent rather than stubbed.
- This slice ships **no validation gates**. Those are the next work item, and
  they land report-only over all 379 chapters before any track fails on them.

### Fixed — live generated curriculum links

- Preserve canonical Markdown links as live LaTeX `\href` targets instead of
  dropping every destination during book generation.
- Resolve relative lesson and pronunciation-reference links against stable
  GitHub source URLs while preserving absolute source citations and rich link
  labels from the same canonical blocks consumed by Language Ladder.
- Reject missing relative-link bases and non-HTTP(S) destinations, escape URL
  metacharacters for LaTeX, and regenerate the nine affected chapters with 55
  working links.

### Fixed — generated quotation typography

- Render paired straight double quotes in canonical lesson prose with explicit
  LaTeX opening and closing quote commands across every generated chapter.
- Preserve code spans, escaped literals, link destinations, existing curly
  quotes, and unmatched marks while handling emphasis and nested quotations.
- Keep indented Markdown blockquote continuations inside the same generated
  quote/callout so multiline learner examples are not split during rendering.
- Regenerate all 270 configured chapter targets without changing the canonical
  Markdown consumed by Language Ladder.

### Added — Persian and Urdu take-leave frontiers

- Extend both RTL tracks through `SPINE-TAKE-LEAVE` with four schema-v2
  Chapter 5 micro-lessons apiece: the two historical word layers, the complete
  local-script farewell, and cumulative start-versus-end practice.
- Compile one objective contract for every new lesson, raising mapped
  non-lexical coverage from the Chapter 4 baseline to 25 of 119 lessons while
  leaving the 94-item debt unchanged.
- Generate both Chapter 5 LaTeX files from the same prerequisite-closed lesson
  AST consumed by Language Ladder, preserving Persian joined **خداحافظ** and
  Urdu spaced **خدا حافظ**.

### Added — Persian and Urdu shared name exchange

- Extend both RTL tracks through `SPINE-EXCHANGE-NAMES` with five schema-v2
  Chapter 3 micro-lessons apiece: address/register, question word, complete
  name question, meeting response, and cumulative practice.
- Compile one objective practice contract per track, raising coverage to 21 of
  115 mapped non-lexical lessons across 18 tracks while leaving the 94-item debt
  unchanged.
- Generate both Chapter 3 LaTeX files from the same prerequisite-closed lesson
  AST consumed by Language Ladder and verify their combined source hashes.

### Added — Russian activity prerequisite closure

- Migrate the six-lesson Russian pronoun and naming chain to schema v2 so its
  two mapped non-lexical frontiers have transitive, block-bound knowledge rather
  than activities attached to unowned legacy prerequisites.
- Compile objective checks for polite *вы* and the cross-language *how/what*
  naming contrast, raising coverage to 19 of 113 mapped non-lexical lessons
  across 16 tracks and leaving 94 explicit gaps, 16 of them legacy.

### Added — cross-language objective activity coverage

- Add one prerequisite-closed final-recall contract to a ready non-lexical
  lesson in each of 15 tracks with schema-v2 coverage debt: Arabic, German,
  Gujarati, Hindi, Italian, Kannada, Latin, Malayalam, Marathi, Portuguese,
  Punjabi, Sanskrit, Spanish, Tamil, and Telugu.
- Keep every new response budget at eight seconds and select a safe Italian
  Chapter 3 frontier rather than pushing its 297-second Chapter 2 practice lesson
  past the strict five-minute ceiling.
- Raise measured objective coverage from 2 to 17 of 113 mapped non-lexical
  lessons while leaving the 18 legacy migration prerequisites explicit.

### Added — compiled activity contracts

- Parse compact JSON `hl-activity` directives beside typed block knowledge and
  keep prompts, canonical answers, accepted variants, corrective feedback, and
  response budgets in the canonical AST while omitting metadata from learner copy.
- Compile normalized answer sets once for browser consumers and validate stable
  activity ids, non-empty assessed-atom subsets, block-bound assessment closure,
  unique variants, complete feedback, and 1–299 second response budgets.
- Count authored activity response time in duration model v2 and add objective
  grammar and script pilots to two Spanish lessons without changing book prose.

### Added — per-track shared-spine realization maps

- Load and validate one ordered `curriculum.json` for every registered track,
  with repeatable spine segments, explicit omission/relocation ledgers, and
  typed language-specific extensions placed before, inline, or after a segment.
- Require canonical and schema-v2 lesson coverage, prerequisite-closed local
  order, and exact support-lesson extension classification across all 20 maps.
- Add pure local-path and independent mixed-frontier queries so downstream apps
  can schedule the next safe lesson without borrowing another language's
  progress.

### Added — non-Latin canonical book chapters

- Let a generated-book target declare a Unicode Script property and its existing
  LaTeX font command, wrapping only target-script runs while keeping surrounding
  prose in the book's main font.
- Use authored romanization for non-Latin section bookmarks and fail closed when
  only half of the script-rendering configuration is present.
- Generate Marathi Chapter 6 from its two strict canonical lessons and expose the
  same ordered source hash to Language Ladder.
- Generate Gujarati Chapter 6 from its two strict canonical lessons, preserving
  Gujarati-script runs and bookmark-safe romanization from the shared AST.
- Generate Punjabi Chapter 6 from its two strict canonical lessons, preserving
  Gurmukhi runs and bookmark-safe romanization from the shared AST.
- Generate Sanskrit Chapter 6 from its three strict canonical lessons,
  preserving Devanagari forms, comparison tables, and romanized bookmarks from
  the shared AST.
- Generate Bengali Chapter 6 from its strict canonical lesson, preserving the
  Bengali numeral forms, *dui* history, and bookmark-safe romanization from the
  shared AST.

### Added — block-boundary knowledge closure

- Parse canonical `hl-knowledge` directives beside every schema-v2 body block
  while excluding the metadata from learner-facing Markdown.
- Validate introductions and assessments in rendered order, reject undeclared or
  unavailable prompt knowledge, and require production and recall blocks to name
  what they assess.
- Migrate all 51 Spanish Chapters 1–6 lessons to the fail-closed block contract
  and refresh their shared app/book source hashes.

### Added — canonical LaTeX chapter generation

- Added deterministic lesson-AST fingerprints and a pure Markdown-block to
  LaTeX renderer, now covering all 24 Spanish Chapter 1–3 schema-v2 lessons.
- Preserved nested inline emphasis, wrapped long practice lists, and emitted
  text-safe short titles for running headers and PDF bookmarks.
- Added write/check CLI modes, a committed chapter-hash manifest, path-safety
  validation, and a unified-book CI drift gate.
- Exposed each parsed lesson's source hash so book and app consumers can verify
  that they loaded the same canonical content.

### Added — schema-v2 lesson AST and strict curriculum contract

- Parse one-level nested lesson frontmatter and losslessly expose level-two
  Markdown sections as stable typed body blocks.
- Enforce schema-v2 spine mapping, local sequence, strict computed duration,
  block shape, coverage metadata, same-language prerequisites, stable knowledge
  atoms, unique introductions, and transitive knowledge closure.
- Prove the contract on all 24 Spanish Chapter 1–3 lessons while preserving
  schema-v1 compatibility for the rest of the corpus.

### Added — curriculum migration gap report

- Added deterministic JSON and text reports for effective lesson duration,
  unknown and omitted prerequisites, book-chapter coverage, and per-track schema
  migration status.
- Added a CLI format switch so CI can publish both report forms with the unified
  human-language book artifact without turning existing migration debt into a
  false regression gate.

## [0.3.0] - 2026-07-18

### Added — `writing` lesson type (orthography / writing nuances)
- **New exempt lesson type `writing`** for lessons that teach a *writing-system*
  nuance — an accent mark, a diacritic, an inverted punctuation mark — rather
  than a vocabulary word. Its `headword` is the mark itself and it carries **no
  `concept_tag`** (a mark does not join across languages), so it is exempt from
  the cross-language concept join, exactly like `practice`/`review`.
- Validator now accepts `writing` without flagging `unknown-type` or requiring a
  concept; added a test covering it. Supports the curriculum's "teach the
  accent marks and other writing nuances" goal (HL00) and gives HL02's
  hand-writing practice a lesson type to draw from.

## [0.2.0] - 2026-07-18

### Changed — general script model (teach any writing system)
- **`Script` is now an open string**, not a closed union — a new script needs no
  type edit.
- **Generalized the script-data schema** to cover all three families with one
  shape: `alphabet`, `abugida`, `abjad`. `ScriptData` gains `name`, `direction`
  (ltr/rtl), `system`, and `complete`; `Glyph`→`Letter` (with `role`, optional
  contextual `forms` for cursive/abjad scripts, `inherentVowel` for abugidas);
  `VowelSign`→`Mark` (vowel signs *or* harakat/niqqud). (Breaking, but nothing
  consumed the old shape yet.)
- **Tracks may self-declare their script** via `<track>/track.json`
  (`{ "script": "hebrew" }`); `parseLesson` takes an optional resolved script and
  the loader passes it in. Adding a new-script language needs no shared-map edit.
- **Coverage hardens with `complete`**: unknown headword characters are warnings
  while a script file has `"complete": false`, and become errors once it's `true`.

### Added
- `data/scripts/devanagari.json` (abugida) and `data/scripts/arabic.json`
  (abjad, rtl, contextual forms) — the two reference inventories proving the
  general schema across LTR-abugida and RTL-abjad.
- `data/scripts/README.md` — the "add any script" checklist (Gujarati, Bengali,
  Hebrew, …): author `<script>.json`, vendor the font, point a track at it.
- `trackScript` loader export; tests for open script ids, contextual-form
  coverage, and complete→error escalation.

## [0.1.0] - 2026-07-17

### Added
- Initial release — the HL01 data layer over the Human Languages curriculum.
- **Types** (`types.ts`): `Concept`, `Realization`, `Dataset`, `Taxonomy`,
  `ScriptData`/`Glyph`/`VowelSign`, `Issue`.
- **Frontmatter reader** (`frontmatter.ts`): a tiny zero-dependency parser for the
  `key: value` / `[list]` frontmatter shape the lesson schema uses (BOM- and
  CRLF-tolerant, quote-stripping, comment-skipping).
- **Parser** (`parse.ts`): `parseLesson` derives a `Realization` from lesson
  frontmatter (romanization defaults to headword for Latin scripts; gender sniffed
  from the gloss when unfielded); `buildDataset` joins content lessons through the
  taxonomy into concepts + per-language indexes.
- **Validator** (`validate.ts`): the round-trip consistency gate — resolves every
  concept tag, forbids duplicate realizations per language, checks required fields
  and field shapes, script-glyph coverage, and core-concept coverage. Errors fail
  CI; warnings/info are tolerated.
- **Queries** (`queries.ts`): `allConcepts`, `conceptsByLanguage`,
  `languagesForConcept`, `coverageByLanguage`.
- **Loader + CLI** (`loader.ts`, `cli.ts`): the filesystem boundary — reads the
  curriculum and runs `validate`. Declared `fs:read`/`fs:list` capabilities.
- Tests for the pure core (frontmatter, parse, validate, queries) plus an
  integration test that validates the **real** curriculum in CI and asserts the
  cross-language joins (e.g. `GREETING-HELLO` across all 16 tracks).

### Notes
- `data/scripts/*.json` character-breakdown data is authored incrementally in
  follow-up work; the package degrades gracefully when it is absent.
