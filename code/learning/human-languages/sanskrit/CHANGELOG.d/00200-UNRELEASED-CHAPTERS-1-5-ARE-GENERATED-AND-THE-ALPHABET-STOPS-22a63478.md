## Unreleased — chapters 1–5 are generated, and the alphabet stops arriving all at once

The owner opened the book and asked why the opening chapters still teach the
alphabet and several headwords at once, when the gentle-ramp rewrite was
greenlit long ago. The answer was that `ch01`–`ch05` were **hand-written LaTeX
the generator skipped**. No lesson-level gate had ever applied to them, so every
report saying "0 lessons over 300 effective seconds" was true and entirely
irrelevant to those five chapters. They are now generated from lessons.

### It was not a flip

`handwritten_parity.py` reported Sanskrit under NOTHING WOULD BE LOST and
`--check sanskrit` already exited 0, so the plan was to move five owners from
`book-generation.d/handwritten.d/` to `targets.d/` and regenerate. That failed
immediately: **generated books require schema version 2**, and all thirty
chapter 1–5 lessons were schema v1 — the "legacy schema-v1 chapter has no typed
knowledge atoms yet" note in `chapters.d` said so. `chapters.ts` names this
exact case and names the fix: "the fix is the schema-v2 migration, not a looser
gate." So the thirty lessons were migrated, and Sanskrit became the tenth track
that is schema v2 end to end (corpus: 14 mixed / 9 v2 → 13 mixed / 10 v2).

### What the reader actually gets

Every word lesson opened with a block called "The letters in this word" that
decoded the whole headword into Devanagari at once. Chapter 1's first lesson
introduced न, म, ते, त, े and स् — six characters — before a single script
lesson had run; the lesson teaching न came third, and the one teaching the
vowel-killer ् came at the very end of the chapter, after four words had already
used it. That block is gone from all thirty lessons. In its place each word
lesson carries a **romanization-only** "Sounds you'll need" that says what a
reader can act on by ear: which beat is long, that *dh* and *bh* are one sound
with breath behind them, that a final *-ḥ* is a soft breath rather than a stop.
It sits directly after the warm-up, so the word is heard and said before anyone
takes it apart.

The same rule was applied to the prose. Etymology blocks wrote every Sanskrit
word twice — `**नाम** (*nāma*, "name," neuter)` — which asks a pre-A1 reader to
decode a script they have not been taught, in order to reach a romanization
sitting right beside it. Chapters 1–5 now keep Devanagari **only in the
headword**, where the romanization beside it makes it exposure rather than
something to decode. The script ladder teaches the shapes, one per lesson.

Measured with `measureScriptClosure` before and after:

| | before | after |
|---|---|---|
| Sanskrit lessons decoding an untaught character | **31** | **21** |
| of those, in chapters 1–5 | **10** | **0** |
| corpus-wide | 303 | **293** |
| lessons that cannot be measured for atom pace | 30 | **0** |
| glyph-step spikes | 6 | **4** |

### Three lessons that packed two headwords, split

Page and lesson counts are not a constraint, and splitting was the point rather
than a side effect:

- `SA-C01-am-na` → `SA-C01-am` (आम्, yes) and `SA-C01-na` (न, no). The second
  carries the PIE \*ne material — the word that reaches furthest unchanged —
  and now has a lesson to itself. The track realizes canonical `RESPONSE-YES`
  and `RESPONSE-NO` instead of the lumped `RESPONSE-YESNO`, so the
  `SPINE-RESPOND-BASIC` omission ledger was updated to match.
- `SA-C02-bhavan-tvam` → `SA-C02-bhavan` (respectful) and `SA-C02-tvam`
  (familiar, the ancestor of *thou*).
- `SA-C03-kushalam` keeps the word; the reply *ahaṁ kuśalī asmi* becomes
  `SA-C03-kushali-asmi`, which is also where *asmi* and the first-person `-mi`
  ending are now taught.

### Chapter 2 got back the exchange it was about to lose

Reading the generated chapters against the hand-written ones found one genuine
loss: chapter 2's closing "The whole exchange" section was labelled
`SA-C02-practice`, a lesson that **did not exist**. Chapter 2 was the only one
of the five with no recap lesson, so generating it would have dropped the
four-line introduction dialogue and the "every atom is a source" paragraph. The
parity script could not see this — it counts whether a prose block disappears,
and this block belonged to no lesson at all. `SA-C02-practice` was written to
carry that content, and it is now the chapter's payoff.

### Two bugs only the generator could find

Both were latent in the lesson sources and harmless while nobody generated
these chapters:

- `SA-C01-namaste` cited PIE `*nem-` with an unescaped asterisk, which opened an
  italic run that swallowed the next three sentences of the *namaste* etymology.
- `SA-C02-nama` cited `*h₃nómn̥` with a combining ring below (U+0325). No
  precomposed form exists, so NFC cannot compose it and the vendored book font
  cannot render it — `glyph-coverage.test.ts` failed the moment chapter 2 became
  a generated book. It now cites the stem `*h₃nomn-`, which is what the
  surrounding sentence is talking about anyway.

### Verified

`node dist/cli.js validate` 0 errors; `npx vitest run` 1689 passing with only
the two pre-existing `figure*.test.ts` failures (missing `paint-vm` dependency);
every `check:*` gate exit 0; `handwritten_parity.py --check sanskrit` exit 0 and
now reporting "already retired, nothing handwritten remains"; XeLaTeX compiles
the book to 452 pages.

Chapter 1–5 content lessons went 30 → 34, and the track 300 → 304.

