# Changelog

## Unreleased — schema-v2 lesson compatibility

- Keep the browser's lightweight dataset adapter compatible with the canonical
  typed lesson AST.
- Derive the lesson-card minute label from schema-v2 `duration.max_seconds`,
  while retaining `est_minutes` for unmigrated tracks.
- Independently combine loaded lesson fingerprints and show `book synced` for a
  generated chapter only when the app AST matches the committed book manifest;
  Spanish Chapters 1–3 now verify all 24 migrated lessons this way.

## 0.25.0 — the same syllable in its sister scripts (syllabary, PR 9)

- **Telugu కి, Kannada ಕಿ, Malayalam കി — side by side.** The three Dravidian
  cousins write one sound three ways, and once you can read one the others are a
  short hop. The Browse detail panel now shows, under **"Same sound, sister
  scripts,"** the selected syllable as the *other* syllabaries write it — turning
  "learn Telugu" into "learn the family" by making the connection visible (the
  spiral model's whole premise: the links between languages are the memory hooks).
- **Grounded, nothing invented.** A new pure `crossScriptSiblings` matches the
  syllable's romanization *exactly* across scripts. That is safe because Telugu /
  Kannada / Malayalam are all emitted by the one generator from the same
  ISO-15919 scheme, so "ki" is byte-identical everywhere; every sibling glyph is
  a real letter already in another script's data, pulled out by that match.
- **Restricted to the fully-syllabic trio.** Only scripts where every letter is a
  `syllable` (the `isSyllabary` predicate) contribute siblings, so Tamil /
  Devanagari / Gujarati — abugidas that model a consonant and a vowel-sign
  separately — are never mis-matched, and alphabets get no sibling row at all. A
  Malayalam-only row (the alveolar **ṉa**) correctly shows no siblings.
- **Control test.** Telugu "ki" resolves to the real Kannada ಕಿ + Malayalam കി
  (and never Telugu itself); an alphabet and the Malayalam-only ṉa row yield
  none; and — the control — the helper is read-only: `letters`, `isSyllabary` and
  the matrix are untouched.

## 0.24.0 — the script's numerals (syllabary, PR 8)

- **The syllabaries now carry their own digits.** Reading a language means
  reading its numbers, and Telugu / Kannada / Malayalam write them with distinct
  glyphs, not Western 0-9 (Telugu ౦౧౨౩౪౫౬౭౮౯). Browse now shows a **"Numerals
  (0–9)"** strip for the three Dravidian scripts, each digit tile the glyph over
  its value.
- **Grounded, additive, same pattern as the independent vowels.** The generator
  composes each from `<SCRIPT> DIGIT <ZERO..NINE>` and romanizes it as the digit
  value (the Unicode name fixes it unambiguously — these are decimal digits, no
  guessing). They live in a **separate `digits` field**, not mixed into `letters`,
  so the consonant syllabary and the gate/matrix that key on it being
  all-syllables stay untouched (the generated `letters` and `independentVowels`
  are byte-for-byte unchanged).
- **Control test.** The real Telugu digits are the 10 expected glyphs mapped to
  "0"…"9" (role `digit`, no fabricated ductus), all three scripts carry them, and
  — the control — none leak into `letters`, so `isSyllabary` still holds and the
  matrix is unaffected.

## 0.23.0 — flag the special-consonant rows in the matrix (syllabary, PR 7)

- **The matrix now marks its tricky rows.** In the consonant × vowel grid the
  retroflex **ḷa** and alveolar **ṟa / ṉa** rows — the ones a reader confuses
  with the ordinary *la / ra / na* — now carry a **★** and the same teal tint the
  Browse tiles give them, so in the full grid the confusable rows stand out at a
  glance. Connects the special-consonant flag (PR 4) to the matrix (PR 5).
- **No new judgement.** `buildSyllableMatrix` gains a `special` flag per row,
  computed by reusing the already-tested `specialConsonant` classifier on the
  row's base syllable — so the matrix flags exactly the same rows the tiles do
  (Telugu ḷa / ṟa; Malayalam also ṉa; Telugu has no ṉa). Control-tested: a "ḷa"
  row is flagged, "ka"/"la"/"ra" are not, and the real Telugu grid flags exactly
  the ḷa / ṟa rows. Zero new data.

## 0.22.0 — the independent (word-initial) vowels (syllabary, PR 6)

- **The syllabaries now carry their standalone vowels.** Everything so far was
  consonant syllables (a consonant + a vowel *sign*); but a word that *begins*
  with a vowel writes a different letter — the **independent vowel** (అ *a*,
  ఆ *ā*, ఇ *i* … ఔ *au*, ఋ *r̥*). Without them a vowel-initial word can't be read.
  Browse now shows an **"Independent vowels (word-initial)"** strip above the
  grid for the three Dravidian scripts.
- **Still Unicode-grounded, still additive.** The generator composes each from
  `<SCRIPT> LETTER <V>` (the inherent /a/ is `LETTER A`), romanized in ISO-15919
  from the same vetted vowel table as the signs — never re-typed, so the vocalic
  R is r̥ (r + U+0325), not IAST ṛ. They live in a **separate `independentVowels`
  field**, not mixed into `letters`, so the consonant syllabary — and the
  slow-unlock gate and the matrix that key on it being all-syllables — is
  completely untouched (the generated `letters` are byte-for-byte unchanged).
- **Control test.** Asserts the real Telugu independent vowels are the 13 expected
  glyphs + ISO-15919 romans (role `vowel`, no fabricated ductus, r̥ = r + U+0325),
  all three scripts carry them, and — the control — none leak into `letters`, so
  `isSyllabary` still holds and the matrix still builds its full 35 × 13 grid.

## 0.21.0 — Browse a syllabary as its consonant × vowel matrix (syllabary, PR 5)

- **The syllabaries now offer a grid view.** A Dravidian abugida isn't a flat
  list of ~450 signs; it's a table — every consonant marching across the same
  vowel row (ka kā ki … , kha khā khi … , ga gā gi …). Browse gains a **List /
  Matrix** toggle (syllabaries only; alphabets stay a plain list): Matrix lays
  the syllables out as **rows = consonants, columns = vowels**, so the abugida's
  regularity is the first thing you see. Clicking any cell selects that syllable
  and opens the existing "break it apart" detail panel. No new data — the same
  generated syllables, re-arranged.
- **New pure helper `buildSyllableMatrix(letters)` in `matrix.ts`.** It reuses
  the grounded consonant boundary from `syllabary.ts` (a new row at each bare
  consonant) and reads the column vowels off the first consonant's own row (its
  base syllable's sound minus its inherent vowel gives the consonant prefix;
  stripping that off each syllable yields the vowel it carries — kā → "ā",
  kr̥ → "r̥"). Nothing is invented. If the rows don't all span the same vowels it
  returns **null** rather than risk a syllable sitting under the wrong vowel
  header. Unit-tested with a **control** that a ragged input yields no matrix,
  plus a check against the real Telugu data (a full 35 × 13 grid; the vocalic-R
  column header is ISO-15919 r̥ = r + U+0325, not the IAST dot-below ṛ).

## 0.20.0 — flag the special consonants: retroflex ḷ, alveolar ṟ / ṉ (syllabary, PR 4)

- **The three Dravidian "special" consonants now carry a contrast hint.** To an
  outsider ల vs ళ (*la* vs *ḷa*) is the kind of near-miss that stalls reading, so
  the app now flags the **retroflex ḷ** and the **alveolar ṟ / ṉ** the same way
  it flags Latin false friends: a **★ special consonant** badge on the Browse
  detail, a *"Special letter — tell it apart from 'l/r/n'"* section with a
  grounded note on how it differs, and a tinted grid tile. No new data — these
  letters were already generated (LLA / RRA / NNNA); this only surfaces them.
- **New pure helper `specialConsonant(letter)` in `core.ts`** (mirrors
  `isFalseFriend`): it keys on the syllable's ISO-15919 romanization, which is
  script-agnostic — the leading code point ḷ (U+1E37, dot below) / ṟ (U+1E5F) /
  ṉ (U+1E49, line below) is the retroflex/alveolar marker. Those marks appear
  *only* on these consonants in our data — the vocalic-R vowel uses a different
  code point (ring-below r̥, U+0325) — so the test is exact, not heuristic.
  `LetterView` gains a `special` field. Unit-tested with a **control** that keeps
  the ordinary l / r / n and the vocalic r̥ un-flagged, plus a check that exactly
  the 26 LLA+RRA rows of the real Telugu data are marked (Telugu has no ṉ).

## 0.19.0 — the full vowel row: ai, au & vocalic R (syllabary, PR 3)

- **Each consonant now carries three more syllables.** The generator's core
  vowel row was the ten short/long vowels (a ā i ī u ū e ē o ō); it now also
  composes the two **diphthongs** (ai, au) and the **vocalic R** that
  Sanskrit-derived words carry — కృ = *kr̥*, as in కృష్ణ *kr̥ṣṇa* "Krishna". So a
  consonant's row grows from 10 to 13, and the regenerated data goes **Telugu
  350 → 455, Kannada 350 → 455, Malayalam 360 → 468** syllables.
- **Still Unicode-grounded.** The three new syllables are composed from the
  `VOWEL SIGN AI` / `AU` / `VOCALIC R` code points of each block, verified to
  exist by their official Unicode names before use — nothing hand-typed. The
  vocalic-R romanization is **ISO-15919 `r̥`** (a plain *r* with a combining ring
  below), deliberately *not* IAST's dot-below `ṛ` — in ISO-15919 that dot-below
  form is the *retroflex* ṛ, a different sound, so using it would be wrong.
- **Flows through the slow-unlock gate unchanged.** The new syllables are signed
  (two components), so `consonantGroups` keeps them inside their consonant's
  group automatically: Practice on Telugu now reads *"mastered 0 / 13"* and the
  first row is `ka kā ki kī ku kū ke kē ko kō kai kau kr̥`. No app code changed —
  only the generator and the regenerated JSON (plus the one data-dependent test
  assertion, 10 → 13). Still recognition only (`strokeOrder: []`).

## 0.18.0 — introduce syllables slowly, one consonant at a time (syllabary, PR 2)

- **The Dravidian drill no longer dumps 350 syllables at once.** Practice on
  Telugu / Kannada / Malayalam now opens with a *single consonant's vowel row*
  (ka kā ki kī ku kū ke kē ko kō) and unlocks the next consonant only once the
  current row is mastered — the "ka, ki, ku … kha, khi, khu" build-up the app is
  meant to teach. This is recognition pattern-building, done the slow way.
- **New pure module `src/syllabary.ts`** is the gate: `consonantGroups` segments
  the consonant-major syllabary at each bare consonant (a grounded boundary — a
  base syllable has one component, a signed one has two), `unlockedConsonantCount`
  counts how many rows are open given the SRS state (a Leitner box ≥ 3 marks a
  syllable mastered; a gap holds everything after it locked), and
  `unlockedLetterIndices` returns the currently drillable subset. No DOM, no
  globals; unit-tested with a control that keeps the 2nd consonant locked until
  the 1st row is fully mastered, plus a check against the real generated Telugu
  data (35 consonants, a 10-syllable first row).
- **Practice wiring** (`main.ts`): on a syllabary in single-script scope, the
  scheduler picks the next question *only from unlocked syllables*, distractors
  are drawn *only from unlocked syllables* (a not-yet-introduced consonant never
  appears as a decoy), the mastery read-out is scoped to the unlocked rows
  (`mastered 0 / 10`, not `0 / 350`), and a cue reads **"Learning consonant N of
  M — master this vowel row to unlock the next."** The alphabets and Mixed mode
  are untouched (the gate is null for them).

## 0.17.0 — Telugu, Kannada & Malayalam letters (syllabary, PR 1)

- **The three Dravidian scripts now have letters.** Browse and Practice covered
  only Arabic / Devanagari / Tamil; Telugu, Kannada, and Malayalam — the tail of
  the language chain — had none. They're **abugidas**, so each "letter" is a
  syllable: a base consonant carries an inherent *a*, and a vowel sign turns it
  into ka → ki → ku, kha → khi → khu. All three now appear as Browse tabs (350
  Telugu / 350 Kannada / 360 Malayalam syllables) and drill in Practice, reusing
  the existing letter engine unchanged (a syllable is just a `Letter`).
- **Generated from Unicode, not hand-typed.** New
  `data/scripts/generate_syllabary.py` composes every syllable from Unicode code
  points, taking each base consonant / vowel-sign's identity and ISO-15919
  romanization from its official Unicode character name (`TELUGU LETTER KA`, …) —
  a letter it can't name from the standard is skipped, never guessed. The three
  `*.json` files are its regenerable output; the generator is the provenance.
- **Recognition only — no fabricated stroke order.** These carry `strokeOrder:
  []` (their ductus is a separate, source-gated effort, still paused). The Browse
  detail now **hides the "Write it — stroke order" section when there is none**,
  rather than showing an empty one — so we never imply data we don't have. The
  grounded consonant⊕vowel-sign decomposition (`క ka + ి "i" sign`) still shows.
- 10 new tests (238 total) grounding the glyphs to code points, with controls
  that bite: `ka` must equal the block's KA code point, `ki` must be KA + the
  i-sign, and every syllable's `strokeOrder` must be `[]`. Verified in a real
  browser — Telugu and Malayalam grids render real glyphs, no tofu. Slowly
  unlocking the syllables one consonant at a time is the next slice (PR 2).

## 0.16.0 — a spine progress bar (HL03 polish)

- **A slim progress bar under the "Concept N of 186" line**, showing how far
  along the whole spine the walk has reached — a sense of the journey's scale
  that the bare count doesn't convey at a glance (a thin sliver at concept 1, a
  half-full bar at ~93, full at the end).
- New pure `spineProgress(cursor, length)` in `sequence.ts` returns the fraction
  reached in `[0, 1]`, counting the current concept (cursor 0 of 10 → 0.1),
  clamping an out-of-range cursor and returning 0 for an empty spine. 3 new tests
  (228 total) with a control that bites: a naive `cursor/length` would read 0 at
  the start, so the test pins 0.1. Width is set via `style.width` (no innerHTML).
  Verified in a real browser (seeded to concept 93 → the bar sits at ~50%).

## 0.15.0 — jump to any concept (HL03 polish)

- **A "jump to concept" picker in the Learn nav.** Walking 186 concepts one
  Next-click at a time is a long way to get anywhere; the nav row is now
  `← Previous | [jump ▾] | Next →`, where the picker is a native `<select>` of
  the whole book-ordered spine (`1. courtesy · thanks`, `2. farewell`, …) with
  free keyboard type-ahead. Selecting one jumps the cursor straight there.
- All three controls now funnel through one `jumpToConcept(index)` — it clamps,
  resets the review draw (the covered set changed), persists the cursor (so the
  jump is where you resume next visit, via the existing `cursorstore`), and
  re-renders. No new persistence surface; it reuses the tested cursor save/clamp.
- **A slice was abandoned, honestly:** the planned "romanization under the review
  options" turned out un-grounded — only ~54 of ~700 lessons populate a
  `romanization` field, and the Indic vocabulary (where script-shape guessing is
  the real gap) carries its romanization inside the *gloss* text instead. Showing
  it would render inconsistent subtext for <8% of options, so it was dropped
  rather than faked; this jump-picker was built instead. 225 tests still pass;
  verified in a real browser.

## 0.14.0 — start over: a "Reset progress" control (HL03 polish)

- **You can now clear your progress.** The app persists a lot — the review
  quiz's SRS state and answer log, the teaching cursor, and the lesson schedule —
  but had no way to wipe it (handing the tab to someone else, or re-walking from
  the top). A quiet **"Reset progress"** control sits at the foot of the Learn
  view; it's a **two-click confirm** (first click arms *"Clear all progress…?"*
  with Yes / Cancel, second executes) so a stray tap can't erase everything.
- New pure `src/reset.ts`: `OWNED_STORAGE_KEYS` sources the three keys from the
  modules that own them (so the list can't drift from what's actually written),
  and `clearProgress(storage)` removes exactly those — **only keys this app
  owns**, guarded per-key so one locked key can't turn "start over" into a crash.
  Executing also resets the in-memory session to concept 0 with an empty review.
- 6 new tests (225 total) with a control that bites: a `clearProgress` that
  missed any owned key leaves it behind and fails; unowned keys are left
  untouched; a throwing `removeItem` still clears the rest. Verified in a real
  browser — both the link and the armed *"Yes, reset / Cancel"* state render.

## 0.13.0 — the Learn walk resumes where you left off (HL03 phase 7b)

- **The teaching cursor now persists.** Review progress and the lesson schedule
  already survived reloads; the Learn cursor didn't — walk to concept 40, close
  the tab, and you were dumped back at "thanks". New `src/cursorstore.ts` saves
  the concept index to `localStorage` on every Prev/Next and restores it at
  startup, so the app resumes exactly where you were. The restored index is
  **clamped to the current spine** (the curriculum grows and shrinks), and a
  corrupt / wrong-version / out-of-range blob falls back to concept 0 rather than
  throwing or pointing off the end.
- 11 new tests (219 total) with controls that bite: strip the version gate and a
  stale blob resurfaces; drop the `getItem` guard and a throwing storage breaks
  startup; a saved index past a now-shorter spine clamps to the last concept.
  Verified in a real browser — seeding the cursor and reloading opens on
  "Concept 5 · Greeting · Hello", not concept 1.
- **Slice 7b (grammar introduction) was reframed:** the curriculum's grammar
  signal is a single concept tag (`GRAMMAR-THE`, articles) with no dedicated
  explanation field — too thin to ground an honest "new grammar" note the way
  scripts have `signature` data. Rather than fabricate one, this slice does the
  more valuable, fully-grounded resume-cursor work. Grammar introduction can
  return if the curriculum grows richer grammar metadata.

## 0.12.0 — introduces a script the first time you meet it (HL03 phase 7a)

- **New writing systems are now introduced as-needed in the Learn sweep.** When
  the walk first reaches a non-Latin script — Arabic, then Devanagari (Hindi),
  then Tamil — that step's card carries a compact **"New script"** note: the
  script's name, its system (abjad / abugida), and *how to recognise it*, pulled
  straight from the script data's `signature` field (e.g. Devanagari's "a
  horizontal head-line runs across the top; letters hang beneath it like laundry
  on a line"). It appears **once**, at the earliest concept in book order that
  teaches the script, and never again.
- New pure `src/scriptintro.ts`: `LANGUAGE_SCRIPT`/`scriptOf` map each chain
  language to its writing system, `firstIntroductionByScript` computes the intro
  concept per script from the spine + lessons, and `scriptIntroFor` returns the
  note for a step or null. **Grounded, never invented:** a script with no JSON
  data (Kannada / Telugu / Malayalam today) gets no note — the mapping still
  knows the language's script, but the note is gated on having real data.
- 13 new tests (208 total) with controls that bite: the *second* appearance of a
  script must not re-introduce it; a script absent from the available-data set
  yields no note. Verified in a real browser — concept #1 (COURTESY-THANKS)
  shows the note on its Arabic / Devanagari / Tamil stops and nothing on the
  data-less Dravidian stops. Grammar-intro is the next slice (7b).

## 0.11.0 — the review quiz remembers you (HL03 phase 6, persistence)

- **The Learn-mode review quiz now persists between visits.** New
  `src/reviewstore.ts` saves the review `Progress` (per-cell Leitner state + the
  answer log) and the SRS session clock to `localStorage` after every answer,
  and restores them at startup — so promotions, demotions, and logged confusions
  survive a reload. Mirrors `progress.ts` (which does this for the lesson
  schedule): the engine stays pure, all (de)serialization is pure and
  unit-tested, and the untrusted blob is validated field-by-field — a corrupt,
  wrong-version, or wrong-shaped payload restores as **empty rather than
  throwing** (a study app that won't start over one bad key is worse than lost
  progress). States are stored as `[cellKey, QuizState]` entry pairs, sidestepping
  the `__proto__`/key-escaping hazards of an object map.
- 9 new tests (194 total), including controls that bite: strip the version gate
  and a stale blob surfaces; drop the `getItem` guard and a throwing storage
  breaks startup. Verified in a real browser — seeding a saved review and
  reloading restores "1 answered" (fresh is "0").
- **Slice 6d (retire the standalone artifacts) was a no-op:** the HL03 spec's
  "script field-guide, spot-the-script quiz, letter-reading trainer" were only
  ever ephemeral exploratory Artifacts, never committed to the repo (no matching
  files or history), and their capabilities already live in Browse / Practice /
  Concepts. Nothing to remove — so this slice does the persistence work instead.

## 0.10.0 — renamed to **language-ladder** (HL03 phase 6, slice 6c)

- **The app is renamed `script-writing-visualizer` → `language-ladder`.** The
  name no longer described it: what began as the HL02 "break a script apart and
  write it" MVP has become the HL03 unified curriculum app, with Learn (the
  teaching sweep + review quiz) as its default and the old script/lesson/concept
  modes folded in as facets. `language-ladder` names what it now is — the
  language chain, climbed rung by rung.
- Directory `git mv`d (history preserved); `package.json`/`package-lock.json`
  name, `index.html` title, the in-app `<h1>`, and the `BUILD` header updated;
  cross-references in the HL03 spec and the Arabic/Russian curriculum docs
  repointed. No source logic changed — the engine and all five modes are byte-
  for-byte the same; 185 tests still pass and the app builds and renders
  unchanged (verified in a real browser). Earlier changelog entries keep the old
  name: they are the accurate record of what happened under it.

## 0.9.6 — the Learn session, review quiz (HL03 phase 6, slice 6b-2)

- **The review pass, wired into Learn mode** — the second of the app's two
  mechanisms. Below the teaching sweep, a randomised cumulative quiz draws over
  everything covered so far (`plan.reviewGrid`, the concept×language grid up to
  the cursor), SRS-weighted by the engine's `pickNext` so missed/overdue items
  resurface and mastered ones fade.
- A cell is asked as **"‹meaning› — in ‹language›?"** and the options are the
  **same concept in other languages** (plus the answer) — the cross-language
  look-alikes the interleaving exists to expose (Telugu ధన్యవాద vs Hindi
  धन्यवाद, both from `dhanya`). If a concept lives in only one language, the
  remaining option slots are filled from elsewhere in the grid so there is
  always a real choice.
- Answering threads through the tested engine: `applyAnswer` **promotes** a hit
  (comes back later) or **demotes** a miss (resurfaces soon) and logs which wrong
  word was picked; the SRS clock advances. A **"what you keep confusing"** panel
  rolls those up from `confusions(log)`, showing the actual words (e.g. "Picked
  ధన్యవాద (telugu) for धन्यवाद (hindi)").
- Moving the concept cursor redraws the review from the new covered set. Progress
  lives in a module-level `let` for now (persistence is a later slice). DOM-only
  shell over the tested engine; 185 tests still pass. Verified in a real browser:
  the quiz renders below the sweep with four real-script options, no tofu.

## 0.9.5 — the Learn session, teaching view (HL03 phase 6, slice 6b-1)

- **New "Learn" mode, now the default** — the curriculum walked the way the book
  does: one concept at a time, forward along the language chain. It renders the
  engine's *teaching pass* (`planSession(...).teaching`) as a numbered sweep of
  cards, one per active language that teaches the concept, in chain order.
- Each card shows the word in its own script, its gloss/romanization, its
  etymology hook, and — the point of the whole app — the **connections back** to
  earlier languages that share a root (e.g. Telugu *ధన్యవాదములు* → Hindi and
  Kannada via `dhanya, vada`). Connections are grounded and backward-only; the
  first stop, where the concept enters, wears an "introduced here" badge.
- Prev / Next walk the **concept spine** (`sweepableConcepts`) — the concepts in
  book order (earliest chapter first). Consolidation lessons (`practice`,
  `practice-mix`, `review` — placeholder headwords, no roots, `reviews_of`
  links) are filtered OUT of the spine: that kind of revisiting is what the
  review quiz is for, so the learner walks real words, not "(practice)". 205 →
  186 concepts.
- DOM-only shell; all sequencing stays in the tested engine. Verified in a real
  browser: every script renders (no tofu), the ten-language THANKS sweep shows
  its `dhanya`/`nal` threads. The review quiz (`pickNext`/`applyAnswer`) is the
  next slice (6b-2).

## 0.9.4 — the session view-model (HL03 phase 6, slice 6a)

- `src/sessionplan.ts` — the seam that assembles the four engine modules into
  one session, with no DOM (the UI slice renders what this returns):
  `planSession(current, covered, lessons, activeCount)` returns the **teaching
  pass** (the current concept swept across the active chain, with connections)
  and the **review pass** (the covered grid the quiz draws from).
- `applyAnswer(progress, cell, correct, session, chosenKey?)` threads the state
  that makes review adaptive: a hit **promotes** the cell (comes back later), a
  miss **demotes** it (box 0, due now) and logs the confusion — so the next
  `pickNext` leans on what was just missed. Immutable; `initProgress` seeds it.
- Controls bite (fault-injected): a review pass that only covered the current
  concept fails the "spans every covered concept" test; a no-op `applyAnswer`
  fails the "missed cell outweighs a mastered one" test. Verified against the
  real curriculum (COURTESY-THANKS teaches across all ten, reviews alongside
  GREETING-HELLO). Pure, deterministic. Next: slice 6b renders it.

## 0.9.3 — the mistakes store (HL03 phase 5)

- `src/mistakes.ts` — records each quiz answer and, crucially, WHAT THE LEARNER
  CHOSE when wrong (the confusion — e.g. picking the French cognate's meaning
  for the Spanish word). `recordAnswer` appends immutably; `demote` feeds a miss
  back into the SRS (box→0, due now, lapse++) so the item resurfaces sooner in
  `pickNext`; `confusions` rolls the wrong answers into ranked "what you keep
  mixing up" pairs.
- Grounded: a confusion only ever appears if the learner actually made it — no
  pair is inferred. Pair keys use `JSON.stringify`, not delimiter-joining, so an
  id containing a comma can't collapse two distinct confusions into one.
- Controls bite (fault-injected): a no-op demote fails the "missed cell
  resurfaces" test (its draw weight must jump above a mastered cell's); a
  fabricated pair fails the "nothing invented" control. Pure, deterministic, no
  I/O — the caller passes the session index in.
- This completes the pure-logic layers of HL03 (phases 2–5). Next: phase 6, the
  UI that unifies the four modes into one curriculum-driven session.

## 0.9.2 — the SRS-weighted draw (HL03 phase 4, part 2 — quiz complete)

- Extends `src/quiz.ts` with the randomised cumulative quiz's draw:
  `pickNext(grid, states, session, rng)` selects a cell from the covered grid
  weighted by `cellWeight` — never-seen cells rank high, DUE cells rank higher
  the more overdue / lower-box / lapsed they are (the missed material review
  exists for), and not-yet-due cells sink to a floor so review stays
  interleaved. Per-cell Leitner state (`QuizState`, keyed by `cellKey`) reuses
  scheduler.ts's box/interval math. Deterministic via a seeded LCG (`makeRng`);
  the app never depends on `Math.random`.
- Two controls, both verified by injection: over many draws the sample spans
  MULTIPLE concepts AND languages (a collapsed draw fails); and the draw biases
  toward a missed/overdue cell over a mastered one by a wide margin (injecting
  uniform weighting fails it). This is the primary review mechanism from HL03 —
  "what is 5 in Telugu? 12 in Latin?" — now complete and pure.

## 0.9.1 — the covered grid (HL03 phase 4, part 1)

- `src/quiz.ts` — `coveredGrid(covered, lessons, activeCount)` enumerates every
  (concept × language) cell the learner has studied, each tied to the real
  lesson that answers it. This is the pool the randomised cumulative quiz will
  draw from ("what is 5 in Telugu? 12 in Latin?").
- Built by **reusing the teaching sweep**, not re-deriving the concept→language
  join: a cell exists exactly where a covered concept's sweep has a stop in an
  active language — so the review side can only ever ask about a (concept,
  language) the teaching side actually presents. Deterministic (concepts sorted,
  then chain order, then chapter/id). Plus `conceptsIn` / `languagesIn` and a
  stable `cellKey` for the SRS to track state per item.
- Verified against the real curriculum: COURTESY-THANKS covers all ten chain
  languages; two covered concepts interleave across both concepts and many
  languages. Controls bite — mislabelling a cell fails the grounding test, and
  collapsing the grid to one concept fails the interleave control. Pure, no UI.
- Next (part 2): the SRS-weighted `pickNext` draw over this grid.

## 0.9.0 — the session orchestrator: sweep + grounded connections (HL03 phase 3)

- `src/session.ts` — `buildSession(concept, lessons, activeCount)` takes the
  teaching sweep (phase 2) and annotates each stop with the **connections back
  to earlier languages in the sweep**: where two languages' lessons for the
  concept share an etymological root, the link is surfaced. This is the payoff
  of interleaving — meeting "thank you" in Telugu right after Kannada and Hindi,
  and being shown all three carry the Sanskrit root *dhanya*.
- **The grounding rule, enforced in code**: a connection exists *iff* the two
  stops' lessons literally share a root string (from `lesson.roots`). Nothing is
  inferred or invented; the reported `sharedRoots` is the exact set intersection,
  sorted. Connections always point backward in chain order.
- Verified against the real curriculum: Kannada and Telugu "thank you" link back
  to Hindi via `dhanya`. Controls bite — asserting a connection without a shared
  root fails the grounding test; over-reporting (union instead of intersection)
  fails the "never from thin air" control; a concept sharing no roots surfaces
  no link. Pure, deterministic, no UI.

## 0.8.1 — carry etymological roots on each lesson (HL03 phase 3 prerequisite)

- `Lesson` now carries `roots: string[]` — the etymological roots a lesson cites
  (e.g. `["bonus", "dies"]`). `toLesson` maps them from the frontmatter the same
  way it maps `prerequisites`; the human-language-data parser already extracted
  them, they just were not threaded into the app's `Lesson`.
- This is the **join key for cross-language connections** (the next phase): two
  lessons in different languages that share a root are etymologically linked.
- Tests: roots parse through (a lesson citing none gets `[]`), and — against the
  real curriculum — the Sanskrit root `dhanya` is carried by lessons in more
  than one chain language (Hindi/Kannada/Telugu). Both fail if roots aren't
  plumbed, confirmed by injection.

## 0.8.0 — the language chain and the teaching sweep (HL03 phase 2)

- First implementation piece of the unified language-learning app
  ([HL03](../../../specs/HL03-unified-language-learning-app.md)): a pure
  `sequence.ts` module encoding the fixed **language chain**
  (Spanish → Latin → French → German → Arabic → Hindi → Tamil → Kannada →
  Telugu → Malayalam) and the **teaching sweep** — for one concept, the active
  languages that teach it, walked in chain order.
- `teachingSweep(concept, lessons, active)` filters to the concept, restricts to
  the active chain prefix, skips languages that do not teach it, and orders the
  result by the chain (never by input order). `sweepableConcepts` lists concepts
  in book order (earliest chapter first). No UI — sequencing logic only.
- Verified against the real curriculum: `GREETING-HELLO` sweeps all ten
  languages in exact chain order. Every honesty check is paired with a control
  that fails on broken input — and writing this caught a redundant active-filter
  that would have made the "only active languages" test vacuous; removed so the
  test can actually fail.

## 0.7.1 — each script's at-a-glance identification signature

- Every script now carries a **`signature`** — the one visual feature that gives
  it away at a glance (Devanagari's head-line; Gujarati being Devanagari with
  that line erased; Arabic's joined right-to-left ribbon vs Hebrew's blocky
  separate letters; Cyrillic's Я/Ж/Д tells; Chinese's dense square blocks).
- Added to all seven script data files (`data/scripts/*.json`) and to the
  `ScriptData` type; a test asserts every script ships a non-empty signature.
- Each signature was written **against the rendered font**, not from memory —
  the same verify-by-looking discipline the stroke data uses. This is the data
  backbone for a future "spot the script" identification mode.

## 0.7.0 — read the letter's TRUE shape out of the font

- **`src/truetype.ts`** — a zero-dependency TrueType reader: table directory,
  `cmap` (formats 4 and 12), `loca`, `glyf`, simple and composite glyphs, the
  delta-encoded coordinate flags, and the on-curve midpoint TrueType implies
  between consecutive off-curve points. Outlines come back in font units
  (y-up, baseline 0); the renderer applies one `scale(1,-1)`.
- **Why not hand-drawn SVG paths.** A subtly wrong ண looks fine to anyone who
  cannot already read Tamil — the entire audience — so the error would ship as
  the lesson. Extracting from the vendored font makes shapes correct by
  construction and keeps them identical to what the app renders text with.
- **Hostile input is bounded.** Every count and offset in a font file is
  attacker-controlled if this is ever pointed at an untrusted font, and it runs
  in the browser. `cmap` ranges clamp to U+10FFFF; a single decrementing budget
  bounds total mapping ITERATIONS across both cmap readers (capping the map's
  size alone is not enough — re-mapping groups and format 4's BMP-bounded keys
  both cost work without growing it); a component budget bounds composite
  FAN-OUT, which the depth cap does not (N components at each of 6 levels is
  N⁶ visits — minutes of frozen main thread from a 632-byte file);
  non-ascending contour end points and scaled components are refused rather
  than drawn wrong.
- **Tests rasterise the font** — flatten the quadratics, scan-convert with the
  non-zero winding rule — so shape assertions are checked against what the
  glyph actually looks like. **The raster window is derived from the glyphs'
  own bounding boxes and the rasteriser throws if a glyph would be clipped.**
  A second guard checks the metric's INPUT: the final-stroke measure reports
  its sample count, and the assertion requires samples before believing the
  answer. Without it the measure anchored on the top bar — which overhangs the
  final vertical — collected nothing, and `Math.max([]) - Math.min([])` is
  `-Infinity`, which satisfies any upper bound. It measured nothing and
  reported agreement.
  The window guard exists because a hard-coded window (x ≤ 1030, against ண's true
  extent of 1631) silently amputated 37% of the letter and produced a
  confident, wrong description of its final stroke. A clipped raster does not
  look like an error; it looks like a letter.

## 0.6.1 — Tamil and Gujarati join the script list

- **Tamil** is new: `data/scripts/tamil.json` ships with the first handwriting
  lessons for **any Dravidian language** (`TA-W01`–`W04`). 11 letters and 4
  marks, `complete: false`.
- **Gujarati was already there and simply never wired in.** `gujarati.json` has
  existed since the Gujarati track was authored, but `SCRIPTS` in `src/data.ts`
  listed five scripts while `data/scripts/` held six. Both are now included, so
  Browse and Practice cover **seven** scripts.
- No logic changed — two imports and two array entries. `tests/core.test.ts` uses
  `arrayContaining`, so the script list is not pinned by count.

## 0.6.0 — Concepts mode: one idea, every language that has it

The app could drill letters, and drill lessons. It could not yet do the thing
the curriculum's shared `concept_tag`s exist for: **compare**.

- **New "Concepts" mode.** Canonical concept tags are deliberately identical
  across tracks, which makes them a join key — *gracias / merci / danke /
  धन्यवाद / നന്ദി* are one concept realized eighteen ways. The mode lists every
  concept **two or more languages** share (**39** of them, from **701** lessons)
  and expands each into a side-by-side table.
- **It calls the package's own `languagesForConcept`.** That function has
  shipped since HL01, tested and documented as "what the companion app calls,"
  with **no caller**. This is the caller — and `buildDataset` beside it, so the
  join is the package's tested logic rather than a second implementation that
  could drift from it.
- Each row carries **headword**, **romanization** (only when it differs from the
  headword — for Latin-script tracks the package sets them equal, and repeating
  it is noise), and gloss. The **etymology hooks** follow the comparison, which
  is where they earn their keep: *gracias* ← *gratia* "favour", *merci* ←
  *mercēs* "wages, price", *danke* ← *denken* "to think", *спасибо* ← *спаси
  Бог* "God save you". One courtesy, four unrelated ideas.
- **Concepts only one language realizes are dropped.** Not a special case for
  namespaced tags — a language floor removes them naturally, because a card with
  nothing to compare against isn't a card.

### Prerequisite gating (the other half)

`scheduler.ts` is generic over a numeric index and has no idea that "the
preterite of *comer*" presupposes *comer*. New `concepts.ts` supplies that
knowledge before the scheduler ever sees the pool:

- A lesson unlocks when every id in its `prerequisites` has been **seen**.
- **Unknown prerequisite ids count as satisfied.** A curriculum typo, or a
  prerequisite pointing at an unwritten lesson, should degrade to "shown
  slightly early" — never to "silently unreachable forever," which is the
  failure nobody notices.
- **The gate fails open.** `unlockedOrAll` falls back to every lesson if the
  gated pool is empty (a prerequisite cycle would do it), because practice
  stalling completely is worse than practising something early. Tested with an
  actual cycle.
- "Seen" is computed from **review history** (`reps`/`lapses`/`box`), never from
  `dueAtSession` — the 0.5.0 bug that reported the whole curriculum as started
  after one reload.
- `reviewTargets` maps `reviews_of` onto scheduler indices and is tested, but
  **nothing calls it yet** — it is groundwork for having the app follow the
  syllabus's own "answering this should refresh those" instead of waiting on a
  Leitner interval. Said plainly because an earlier draft of this entry claimed
  the app already did that, which was false. (`ConceptCard.namespaced` is
  likewise computed and not yet rendered.)

### The bug this shipped with, and how it was caught

The first cut gated the **pick**: choose from the rotation, and if the chosen
lesson is locked, substitute the first unlocked one. That is wrong in a way that
is invisible by inspection — the same pick is rejected every turn, so the
substitute is served over and over. A review simulation of the real curriculum
measured it serving **one Arabic lesson 34 times in 40**, wiping out both the
0.5.0 rotating cursor and cross-language interleaving.

The fix gates the **pool**: `nextDue` now takes an `accept` predicate and skips
locked indices *during* the scan, so the cursor keeps advancing; the
nothing-due fallback runs `pickNext` over the unlocked states rather than
grabbing `open[0]`.

The regression test took three attempts to make honest, which is worth
recording. Versions 1 and 2 **passed against the broken implementation** — the
fixture's chain order happened to match its pool order, so the rotation landed
on unlocked lessons anyway. Only a fixture whose dependency order runs
*counter* to pool order (as the real curriculum's does) reproduces it. The test
now fails on the broken version and passes on the fixed one, and both were
verified by actually injecting the old code.

### Notes

- Tests: **97**, up from 75 — `tests/concepts.test.ts`, including checks against
  the **real curriculum** (every card genuinely spans ≥2 languages; gating opens
  a non-empty but non-total pool on a fresh profile; everything is reachable
  once everything is seen).
- `Lesson` gained `romanization`, `script` and `etymologyHook`, which the cards
  need. `tests/lessons.test.ts` now builds fixtures through a defaulting helper
  so adding a field doesn't touch every test.
- **Verified in a browser**, not just built: the 0.5.0 `process is not defined`
  bug was a successful build that died on load, and only a real page load
  catches that class of error. Both new deep imports (`parse.ts`, `queries.ts`)
  are pure modules; console is clean.
- **Known cost, unchanged:** the eager `import.meta.glob` still inlines every
  lesson, so the bundle is ~1.72 MB (542 kB gzipped). Concepts mode makes the
  parsed corpus more valuable, not less, but a lazy glob or a build-time index
  remains the right fix. Deferred, not hidden.

## 0.5.0 — Lessons mode: the whole curriculum, and a memory that survives reloads

The app could already schedule you. It could not **remember** you, and it had
never read a single lesson. Both are fixed.

- **New "Lessons" mode** drills the **written curriculum** — all **679 lessons
  across 18 languages** — instead of only script letters. It reads them from
  `@coding-adventures/human-language-data`, the package that has always shipped
  `frontmatter.ts` / `loader.ts` / `parse.ts` / `queries.ts` for exactly this
  purpose and had **zero consumers** until now.
- **Progress now persists** (`src/progress.ts`). Previously there was no storage
  layer at all, so every Leitner box reset on reload and nothing was ever really
  "tracked". State is saved to `localStorage` keyed by **lesson id**, never by
  array index — indices shift every time a lesson is added, and saving by
  position would silently reattribute your progress to the wrong lesson. Adding
  a lesson now simply means one more unseen item.
- **Cross-language interleaving comes free.** `interleave.buildPool` already
  round-robins across groups for scripts; grouping lessons by language and
  feeding it the same way yields Arabic → Bengali → French → German → Gujarati →
  Hindi in consecutive reviews. A **rotating cursor** walks that order, which
  also fixes the obvious failure mode: box 0/1 fall due again after one session,
  so a scan-from-the-front picker would hand you the lesson you just answered,
  forever.
- **`scheduler.ts`, `interleave.ts` and `drill.ts` are unchanged.** They are
  generic over a numeric index and never needed to know what an item is — which
  is precisely why lessons could reuse them. The new *logic* is pure and tested
  (`lessons.ts`, `progress.ts`, including the `nextDue` cursor scan); the only
  impure edges are a tiny `StorageLike` port and `main.ts`'s DOM shell, which
  remains untested as before.
- **Defensive loading.** Saved state is untrusted input (hand-edited,
  half-written by another tab, left over from an older build). Every field is
  validated, unknown ids are treated as fresh, `__proto__`/`constructor` keys are
  skipped, the item map is `Object.create(null)`, and a throwing or absent
  `localStorage` (Safari private mode, quota) degrades instead of crashing.
- Tests: **67**, up from 57 — `tests/lessons.test.ts` and `tests/progress.test.ts`.

### Known cost

`import.meta.glob(…, { eager: true })` inlines the full text of all 679 lesson
files, so the bundle is ~1.56 MB (480 kB gzipped) and every startup parses them
all — even in Browse mode, which doesn't need them. Only the frontmatter
survives parsing; the bodies are discarded. A lazy glob or a build-time JSON
index would fix both; deferred rather than hidden.

### Three bugs worth recording

- **`process is not defined` at startup.** Importing the package's barrel
  (`index.ts`) pulled `cli.ts` and `loader.ts` — `process`, `node:fs` — into the
  browser bundle. The build *succeeded* and the app then died on load with a
  blank page. Fixed by deep-importing the pure module
  (`.../src/parse.ts`). Caught only by opening the app in a browser; no test or
  build step would have found it.
- **`vitest.config.ts` does not inherit `vite.config.ts`'s `server.fs.allow`.**
  The lesson glob reaches outside the package root, so tests failed with "Denied
  ID" until the same allowance was declared in both configs.
- **The "don't persist untouched items" guard failed open after one reload.**
  It tested `dueAtSession <= 0`, but fresh items are seeded with the *current*
  session, so from session 1 onward every unseen lesson looked touched: the
  payload grew from ~100 bytes to ~48 kB and the "started" count reported the
  whole curriculum. Now keyed on review history (`reps`/`lapses`/`box`) only,
  with a reload round-trip test that would have caught it — the original test
  only covered session 0, the one case where the bug was invisible.

## 0.4.0 — Cross-script interleaving ("Mixed") practice

- **New "Mixed (all scripts)" practice scope** alongside "This script": Practice
  can now **interleave letters from every script in one session** — HL02's
  interleaving principle ("mixing forces discrimination and transfers better").
  The scheduler picks the next due item across the whole combined pool, so a
  Cyrillic prompt is followed by a Hebrew one, then Devanagari, and so on;
  distractors always come from the **target's own script**. Mastery reads across
  the full pool (e.g. "mastered N / 128").
- **New pure module `src/interleave.ts`** — `buildPool(counts)` lays every letter
  of every script into one **round-robin-interleaved** pool (letter 0 of each
  script, then letter 1 of each, …) so mixing starts on the first pass; the
  generic scheduler drives it unchanged. **6 new unit tests (42 total)** incl. an
  integration proving the scheduler alternates scripts and resurfaces a missed
  letter amid the others.
- UI: a scope toggle in Practice; the per-script tabs hide during a mixed
  session; the prompt shows a small script tag. Still zero runtime deps.

## 0.3.0 — Spaced-repetition scheduler wired into Practice

- **New pure module `src/scheduler.ts`** — a Leitner / SM-2-lite scheduler
  measured in **sessions** (no wall-clock, no `Date`), the "core module" of HL02.
  Each item tracks a Leitner box, `dueAtSession`, lapses, and reps; a correct
  answer promotes the box and expands the interval (1 → 3 → 7 → 15 → 30 sessions),
  a wrong answer drops it to box 0 (due again immediately). `pickNext` returns the
  most-overdue item deterministically (ties → fewest reps → lowest index), falling
  back to the soonest-due so practice never stalls.
- **Practice mode now uses it**: instead of a random letter each question, the
  **scheduler chooses** which letter to ask, so missed letters resurface soon and
  mastered ones fade back — real spaced repetition. Each answer advances the
  session clock and feeds `review`. A **"mastered N / total"** read-out joins the
  score line. (Randomness stays only in distractor choice + answer position.)
- **14 new unit tests (36 total)** covering promotion/expansion, lapse-reset,
  `pickNext` ordering + tie-breaks + the never-stall fallback, immutability, and
  an integration check (a correct streak masters an item; a miss resurfaces).

## 0.2.0 — Practice mode (recall drill)

- **New "Practice" mode** alongside "Browse": a recall drill that shows a
  letter's **sound** and asks the learner to pick the matching **glyph** from
  four options, then reveals right/wrong, shows the answer's decomposition, and
  tracks a running **score** (correct / total / %). Recognition builds reading;
  recall (sound → glyph) is the harder second half.
- **Confusable distractors**: wrong answers are drawn from the same script and
  ranked by confusability (same role / same false-friend status rank higher), so
  the choices are meaningfully hard rather than random noise.
- **New pure module `src/drill.ts`** — `buildDrillQuestion`, `confusabilityOrder`,
  `checkAnswer`, and immutable scoring (`record`/`accuracy`). All randomness is
  **injected by the UI** (target, distractor pick, answer position), so the core
  stays deterministic; **10 new unit tests** (22 total) incl. edge cases
  (small inventories, sloppy choosers, clamping) and a real-data check.
- UI toggle wired in `main.ts` with vanilla DOM (still **zero runtime deps**);
  `main.ts` holds the only `Math.random`.

## 0.1.0 — HL02 MVP: break a script apart and write it

- **New app** (`script-writing-visualizer`): the companion "how to write it"
  surface for the Human Languages curriculum. Renders each non-Latin letter with
  its **component pieces**, a numbered **stroke order**, and a **false-friend**
  badge, for pen-and-paper practice.
- **Reads the canonical script data directly** from
  `code/learning/human-languages/data/scripts/*.json` (no copy — the app cannot
  drift from the curriculum). Ships Cyrillic, Hebrew, Chinese, Arabic, Devanagari.
- **Pure core** (`src/core.ts`) covered by unit tests, including an integration
  check against the real curriculum data (every letter has a glyph + pieces +
  stroke order; Cyrillic flags в/р/с/н as false friends).
- Framework-free vanilla-DOM UI; zero runtime dependencies.
- **Scope:** v1 is read + decompose only (no handwriting capture, no scheduler
  yet) — the first slice of the `HL02` spec.
