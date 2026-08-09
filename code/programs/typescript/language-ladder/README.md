# Language Ladder

Live app: <https://adhithyan15.github.io/coding-adventures/language-ladder/>

**The HL03 unified curriculum learning app** (it began life as the HL02
`script-writing-visualizer` and has subsumed that app's modes). Five modes.
**Learn** (the default) walks each selected language's validated local path —
one prerequisite-safe micro-lesson at a time — and admits a lesson to mixed
review only after focused retrieval in that language. Shared-spine abilities
and grounded roots still show where independently ready paths connect.
**Browse** and **Practice** work on *script letters*;
**Lessons** drills the *written curriculum* — every lesson in every track — on a
spaced-repetition schedule that persists between visits; and **Concepts** shows
one idea in every language that has it, side by side.

The production app keeps lesson Markdown lazy. Learn fetches only the small
track-local batches containing completed and current-frontier lessons; opening
Lessons or Concepts opts into all 278 batches rather than 1,669 individual
lesson requests. BUILD checks both the request ceiling and chunk-size budgets.

## Learn mode (the curriculum session)

The spine of the app is [HL03](../../../specs/HL03-unified-language-learning-app.md)
plus the stricter [HL04](../../../specs/HL04-shared-spine-and-content-pipeline.md)
progression contract. `curriculum.ts` loads every active `curriculum.json` map and
the pure frontier planner returns exactly one safe next lesson per selected
language. A language advances independently; paths are grouped only when their
current lessons share a spine ability. The picker includes every track,
including Russian, Persian, and Urdu, and reports the exact mapped lesson and
extension totals for the mix.

Each frontier card shows the target form, romanization, etymology hook, complete
authored Markdown micro-lesson, shared can-do, local `N of M` position, and any
typed script/grammar/register/etymology extension attached at that point.
Grounded root connections are shown only among languages simultaneously ready
at the same shared ability. Script notes come from explicit local script
extensions and the canonical script data, so Persian and Urdu keep distinct
identities and no global concept cursor guesses where a script belongs.

Before advancing, the learner starts a **focused check**. When a canonical block
has an `hl-activity` contract, the app uses its authored prompt, normalized answer
variants, corrective feedback, and response budget without scraping prose or
showing an answer-bearing summary. Other lexical lessons ask for one English
meaning. A wrong answer leaves the local frontier unchanged; a correct answer
shows feedback before the learner continues. The first objective non-lexical
pilots covered Spanish grammatical gender and punctuation spans; the first
HL-A01 tranches now reach 25 of 119 mapped non-lexical lessons across 18 tracks,
including script, grammar, etymology, culture, and cumulative practice. The
remaining 94 support lessons retain temporary final-recall confirmation while
HL-A01 fills the measured contract backlog.
One successful check completes exactly the current frontier lesson.
`learnprogress.ts` persists stable lesson IDs independently per language and,
on load, keeps only the longest valid local prefix. A newly inserted prerequisite
therefore becomes the frontier instead of being skipped by stale saved state.

Below the frontiers, the randomised SRS review draws only from independently
focused-successful shared lessons. It waits until at least two visually distinct
answers are eligible, then asks *"‹meaning› — in ‹language›?"* with options from
that safe grid. A cross-language comparison can appear only after both local
realizations have passed their own check. Answers still flow through
`applyAnswer`; misses are demoted and recorded for *"what you keep confusing"*.
`reviewstore.ts` persists that SRS state and answer log separately from local
path completion.

A quiet **"Reset progress"** control at the foot of the Learn view clears it all,
including the saved language mix,
(`reset.ts` — only the keys this app owns), behind a two-click confirm so a stray
tap can't wipe everything.

## Concepts mode

The curriculum tags lessons with a shared `concept_tag`, and canonical tags are
deliberately the same across tracks. That makes them a **join key**: *gracias /
merci / danke / धन्यवाद / നന്ദി* are one concept realized eighteen ways.

This mode is that join, and it is the data package's own
`languagesForConcept` — a function shipped from the start, documented as "what
the companion app calls," which until now had **no caller**.

- **42 concepts are shared by two or more tracks**, from 1,066 lessons. A concept
  only one language tags is filtered out: there is nothing to compare it with,
  which also removes almost every namespaced (`ES-…`) tag without a special case.
- Each row shows the **headword**, a **romanization** where it differs, and the
  gloss — so a non-Latin script is legible next to a Latin one.
- The **etymology hooks** sit underneath the comparison, where they do the most
  work: *gracias* ← *gratia* "favour", *merci* ← *mercēs* "wages", *danke* ←
  *denken* "to think", *спасибо* ← *спаси Бог* "God save you". Four languages'
  words for the same courtesy, from four unrelated ideas.

## Lessons mode

Reads all **1,066 lessons across 20 languages** straight from the curriculum via
`@coding-adventures/human-language-data`, and schedules them with the same
Leitner machinery the letter drills use (`scheduler.ts` is generic over an
index; it never needed to know what an item is).

- **It remembers you.** Progress is saved to `localStorage` keyed by **lesson
  id** — never by array index, because indices shift every time a lesson is
  added and saving by position would reattribute your history to the wrong
  lesson. New lessons simply appear as unseen items.
- **It uses the same authored content as the books.** Reveal opens the complete
  Markdown micro-lesson rather than discarding its explanations and practice.
- **It mixes the languages you selected.** Consecutive reviews round-robin across tracks — Arabic,
  then Bengali, then French — rather than marching through one language. That
  interleaving is deliberate: it forces discrimination instead of coasting.
- **Recall, not recognition.** You see the headword in its own script; the
  meaning stays hidden until you ask for it, then you grade yourself
  *Again* / *Got it*.
- Each card also surfaces what the lesson **revisits** (`reviews_of`), the
  curriculum's own review graph.

To clear your progress, delete the `hl-study:progress:v1` key in localStorage.

## Browse & Practice (script letters)

The Human Languages curriculum teaches a non-Latin script *inline* — a letter is
introduced inside the first word that needs it. These modes are the other half
of that promise: they **break each letter apart** into its pieces and show a
**stroke order**, so you can practise *writing it on paper*.

Pick a script, pick a letter, and the detail panel shows:

- the **glyph**, big, with its sound and role;
- **Break it apart** — the letter's component pieces (the "a vertical + two
  stacked bowls" of Cyrillic *в*);
- **Write it** — a conventional stroke order, numbered; and for letters with an
  authored pen path, the **stroke-order filmstrip** below;
- a **⚠ false friend** badge for letters that look like a Latin letter but
  aren't (Cyrillic *в*=v, *р*=r, *с*=s, *н*=n) — the fastest way into the script.

### The stroke-order filmstrip

A numbered list tells you *what* to draw. It does not tell you where the pen
starts, which way it travels, or — the thing a picture of the finished letter
can never show — where the hand **lifts**. For letters that have an authored
pen path, "Write it" becomes a strip of panels instead, each one showing the
letter a little further written:

```
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│ ▏      │  │ ▏      │  │ ▏    ▕ │  │ ▏ ⌒  ▕ │  │ ▏ ⌒▕ ▕ │
│ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │
└────────┘  └────────┘  └────────┘  └────────┘  └────────┘
 1. down     2. along    3. up the   4. over     5. down
 the left    the bottom  right side  the top     the middle
```

Behind every panel, in pale grey, sits the **finished letter — the outline read
straight out of the shipped font**, never a drawing of one. In front of it, in
ink, sits as much of the pen path as the hand has travelled so far, with a dot
showing where the pen is. Underneath sits the cited source for the stroke
*order*, because unlike the shape, the order is not something a font can vouch
for.

Three modules meet to make one picture:

| module | knows | checked by |
|---|---|---|
| `src/truetype.ts` | what the letter *looks like* — the real outline | the font itself |
| `src/strokes.ts` | how it is *written* — pen path, parts, lifts | `strokes.test.ts`: every point on real ink, every join < 2 font units, whole letter traced |
| `src/ductusview.ts` | how to *draw that* — SVG frames, no DOM | `ductusview.test.ts` |

Font units are **y-up**; SVG is **y-down**. The glyph and the pen path are both
in font units, and `ductusview.ts` flips them together with exactly **one**
`scale(1,-1)` group — so a mistake cannot leave a plausible-looking stroke
sitting upside down on a correct letter.

**Tamil அ, ஆ, and ம have authored pen paths today.** `DUCTUS` admits no letter
without a citation for its stroke order, and hand-drawing a letter is forbidden
outright (a subtly wrong Tamil ண looks perfect to exactly the audience that
cannot yet read Tamil, so the error would ship *as the lesson*). அ and ஆ exercise
real two-stroke paths with one lift; ஆ keeps its upright and long-vowel loop in
the same pen-down run, while ம remains one unbroken stroke. Every other
letter falls back to the numbered prose list, unchanged. Extending the coverage
is HL-C09, and it needs a cited source per letter.

## Where it fits

```
code/learning/human-languages/data/scripts/*.json   ← the source of truth (HL01)
        │  (glyph, components, strokeOrder, notes per letter)
        ▼
language-ladder                                     ← this app renders it (HL03)
```

The app imports those JSON files **directly**, so it can never drift from the
curriculum. Adding a script to the curriculum surfaces it here with a one-line
edit in `src/data.ts`. Ships today with **Cyrillic, Hebrew, Chinese, Arabic,
Devanagari, Gujarati, Tamil**, and the three **Dravidian syllabaries** below.

### Dravidian syllabaries (Telugu / Kannada / Malayalam)

These three are **abugidas** — a base consonant carries an inherent *a*, and a
vowel sign turns it into a syllable (క = *ka*, కి = *ki*, కు = *ku*; ఖ = *kha*).
So each "letter" is a syllable, and `data/scripts/{telugu,kannada,malayalam}.json`
are **generated from Unicode** by `data/scripts/generate_syllabary.py`: every
glyph is composed from code points and its romanization taken from the official
Unicode character name (ISO-15919), never hand-typed. They are **recognition
only** — `strokeOrder` is empty, since the handwriting ductus is a separate,
source-gated effort; the Browse detail hides the stroke-order section when it's
absent rather than showing an empty one.

Each consonant carries its full vowel row: the ten short/long vowels plus the
two diphthongs *ai / au* and the **vocalic R** of Sanskrit-derived words (కృ =
*kr̥*, as in *kr̥ṣṇa* "Krishna") — thirteen syllables per consonant, so **Telugu
455 / Kannada 455 / Malayalam 468**. The vocalic-R romanization is ISO-15919
`r̥` (a plain *r* with a ring below), deliberately not IAST's dot-below `ṛ`
(which in ISO-15919 is the unrelated retroflex ṛ).

**Practice introduces them slowly** (`src/syllabary.ts`). Rather than drill all
~450 syllables at once, the recall drill opens with a *single consonant's vowel
row* — ka kā ki kī ku kū ke kē ko kō kai kau kr̥ — and unlocks the next consonant only once
the current row is mastered (a Leitner box ≥ 3). So recognition is built one
consonant at a time, the "ka, ki, ku … kha, khi, khu" way. On these scripts the
drill's target and its distractors are both confined to the unlocked syllables
(a consonant you haven't met never appears, even as a wrong option), the mastery
read-out is scoped to the open rows, and a cue shows *"Learning consonant N of M
— master this vowel row to unlock the next."* The gate is a pure, unit-tested
helper (with a control that keeps the 2nd consonant locked until row 1 is done);
the alphabets and Mixed mode are unaffected.

**The special consonants are flagged** the way Latin false friends are. The
retroflex **ḷ** and the alveolar **ṟ / ṉ** are exactly the letters an outsider
mistakes for the ordinary *l / r / n* (ల vs ళ), so Browse gives them a **★
special consonant** badge, a *"tell it apart from 'l'"* note grounded in the
retroflex/alveolar distinction, and a tinted tile. The classifier
(`specialConsonant` in `core.ts`, unit-tested with a control) keys on the
script-agnostic ISO-15919 mark — leading ḷ (U+1E37) / ṟ (U+1E5F) / ṉ (U+1E49) —
which appears only on these consonants, so no data changed to add it.

**Browse them as a matrix.** An abugida is really a table, so for the
syllabaries Browse offers a **List / Matrix** toggle (alphabets stay a plain
list). Matrix lays the syllables out as **rows = consonants, columns = vowels**
— the same ka/kā/ki… pattern repeating down every consonant, made visible at a
glance — and clicking a cell opens the usual decomposition panel. The layout is
pure (`buildSyllableMatrix` in `src/matrix.ts`): rows reuse the grounded
consonant boundary from `syllabary.ts`, the vowel column headers are read off
the first consonant's own row, and a ragged script yields **no matrix** rather
than a mislabelled cell (unit-tested with a control). No new data — the same
generated syllables, re-arranged. The **special-consonant rows** (retroflex ḷa,
alveolar ṟa / ṉa) are marked with a ★ in the grid, reusing the same
`specialConsonant` classifier the tiles use so the confusable rows stand out.

**The independent (word-initial) vowels.** Everything above is consonant + vowel
*sign*; a word that *begins* with a vowel writes a different letter — the
independent vowel (అ *a*, ఆ *ā* … ఔ *au*, ఋ *r̥*). Browse shows these as a small
strip above the grid. They are generated the same way (`<SCRIPT> LETTER <V>`,
ISO-15919 roman from the shared vowel table) but kept in a **separate
`independentVowels` field**, not mixed into `letters`, so the syllabary and the
gate/matrix that key on it being all-syllables are untouched. Control-tested
(the 13 grounded glyphs; none leak into `letters`, so `isSyllabary` still holds).

**The script's numerals.** Reading a language means reading its numbers too, and
these scripts write them with distinct glyphs (Telugu ౦౧౨…). Browse shows a
**"Numerals (0–9)"** strip; each digit is generated from `<SCRIPT> DIGIT <N>` and
romanized as its value, kept in a **separate `digits` field** (same non-breaking
pattern as the vowels). Control-tested (the 10 grounded glyphs → 0–9).

**The same syllable in its sister scripts.** The three cousins write one sound
three ways — కి / ಕಿ / കി are all *ki* — and once you can read one, the others
are a short hop. When you select a syllable in Browse, the detail panel shows it
as the *other* syllabaries write it, under **"Same sound, sister scripts"**, so
the family connection (the spiral model's core memory hook) is visible on the
page. The pure `crossScriptSiblings` (`src/siblings.ts`) matches by the shared
ISO-15919 romanization — safe because the trio come from one generator, so "ki"
is byte-identical everywhere — and is **restricted to fully-syllabic scripts**
(`isSyllabary`), so Tamil / Devanagari / Gujarati and the alphabets are never
mis-matched, and a Malayalam-only row (alveolar *ṉa*) correctly shows none.
Control-tested (Telugu *ki* → the real Kannada + Malayalam glyphs, never itself;
read-only — `letters` / `isSyllabary` / the matrix untouched).

## Design

- **`src/core.ts`** — the pure, unit-tested heart: `buildScriptView`,
  `scriptSummary`, `isFalseFriend`, `falseFriends`. No DOM, no globals; this is
  where the pedagogy is tested.
- **`src/data.ts`** — the only place that imports the canonical script JSON.
- **`src/truetype.ts`** — a small zero-dependency TrueType reader, so every
  letter this app *draws* comes from the font rather than from a hand.
- **`src/strokes.ts`** — the pen-path model: strokes as pen-down runs, segments
  as labelled parts that must meet head-to-tail, with cited provenance.
- **`src/ductusview.ts`** — the two above, composed into SVG. Pure: it returns a
  tree of plain objects plus a serialiser, and never touches `document`.
- **`src/main.ts`** — a deliberately framework-free vanilla-DOM shell. It walks
  the `ductusview` tree with `createElementNS`/`setAttribute`/`textContent`;
  there is no `innerHTML` anywhere in the app.

## Develop

```sh
npm install
npm run dev        # local dev server
npm run typecheck  # strict source + test typecheck
npm test           # unit tests (vitest)
npm run build      # production build to dist/
npm run preview    # serve the production build
```

## Practice mode (recall drill)

Toggle **Practice** to drill *recall*: the app shows a **sound** and you pick the
matching **glyph** from four options. Wrong answers are the **confusable** ones
(same role / same false-friend status), reveal shows the answer's decomposition,
and a running **score** tracks correct / total / %. The drill logic lives in the
pure, unit-tested `src/drill.ts` (`buildDrillQuestion`, `confusabilityOrder`,
`checkAnswer`, scoring); all randomness is injected by the UI so the core stays
deterministic.

**Spaced repetition.** Which letter you're asked isn't random — a Leitner /
SM-2-lite scheduler (`src/scheduler.ts`, measured in *sessions*, no `Date`)
picks the most-overdue letter each question. Get it right and it drifts into the
future (1 → 3 → 7 → 15 → 30 sessions); miss it and it comes straight back. A
**"mastered N / total"** read-out shows progress. The scheduler is pure and
unit-tested — per `HL02` it's "the core module … where test coverage matters
most."

**Interleaving (Mixed mode).** Toggle **Practice → Mixed (all scripts)** to drill
every script at once. `src/interleave.ts` lays all letters into one
round-robin pool and the scheduler picks across it, so a Cyrillic prompt is
followed by Hebrew, then Devanagari, … — the mixing that HL02 says "forces
discrimination and transfers better." Distractors still come from the target's
own script; mastery counts across the whole pool.

## Scope and what's next

Today the app does **read + decompose** (Browse) and **recall** (Practice). Still
to come toward the full `HL02` spec: the **interleaving scheduler** (spaced,
cross-language review, measured in sessions) and a **write/produce** mode. See
`code/specs/HL02-companion-practice-app.md`.
