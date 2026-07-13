# HL00 — Human Language Curriculum Framework

## Overview

**HL** (Human Languages) is a personal, etymology-driven curriculum for learning
spoken languages, designed for delivery during a daily car commute rather than
through a screen. It is content, not software: the deliverable is two things
built from the same underlying material — a structured set of Markdown lesson
files for car-commute practice, and a LaTeX book per language, meant for free
publication, that grows one chapter at a time.

This is the first spec in the **HL** series. It defines the pedagogical
framework, the content schema every lesson file follows, and the divergence
from this repo's standard software-package process. `HL01`+ are reserved for
per-language notes as new tracks are added (French, German, Arabic, Hindi,
Tamil, Kannada, Telugu, Malayalam) after the Spanish pilot proves the format.

```
                    ┌───────────────────────────┐      Markdown lesson files
  Roadmap (year) ───►                           ├──►   (audio-script formatted,
                    │  Part → Chapter → Unit     │       car-commute practice)
  Etymology data ───►   (this spec's schema)     │
                    │                           ├──►   LaTeX book, one volume
                    └───────────────────────────┘       per language (publish)
```

A note on terminology: internally (frontmatter, file paths, `session-map.md`
mechanics) units are organized by `phase`/`week` — mechanical, zero-indexed.
In anything user-facing (the roadmap, the book, READMEs) the same structure
is called **Part**/**Chapter** — this is a presentation-layer rename only,
done because the whole point of the book requirement below is that this reads
like an actual book, not a topic-label spreadsheet.

## Motivation

Off-the-shelf apps (Duolingo et al.) optimize for screen-tap engagement, not
for two things this learner specifically wants:

1. **Etymology as the primary memory hook.** Spanish *gracias* sticks better
   once it's anchored to Latin *gratia* and English *grace/gratitude/gratitude*
   — a root the learner already owns fluently in English. Isolated flashcards
   don't build that web of connections; a curriculum that surfaces roots
   deliberately does.
2. **Hands-free, commute-shaped delivery.** The learner spends 1-2 hours/day
   driving. A lesson has to be consumable by ear, fit inside a single commute
   leg (~5 minutes), and the *next* leg has to open with recall of what came
   before — spaced repetition built around driving frequency, not calendar
   dates or screen taps.
3. **No assumed grammar vocabulary.** The learner learned English through
   immersion, not formal instruction — terms like "reflexive verb" or
   "subject pronoun" were never taught explicitly, even for English. Every
   grammar-introducing unit has to explain the concept itself, not just the
   Spanish (or French, German, ...) form of it.
4. **Word formation as a second, parallel goal.** The learner already
   noticed that many words are compressed phrases or built from recognizable
   pieces (*usted* ← *vuestra merced*) — and wants that pattern taught
   deliberately, partly as its own reward and partly as a low-stakes way to
   pick up lexical Latin along the way.

## Divergence From Standard Package Process

Per repo convention, packages get a BUILD file, a language metadata file
(`pyproject.toml`/`Cargo.toml`/etc.), and >80% test coverage. None of that
applies here — there's no code to build or test. The content lives under
`code/learning/human-languages/` (a curriculum track, not a package) instead
of `code/packages/`. The quality bar is:

- **Structural consistency**: every unit file has the required frontmatter
  fields (schema below) and follows the four-part anatomy.
- **Linguistic accuracy**: etymology claims are traceable to real root forms;
  uncertain or disputed etymologies are flagged as such rather than stated as fact.
- **Schedule consistency**: the spaced-repetition intervals defined below are
  actually honored by how units are sequenced into sessions (checked in each
  track's `session-map.md`).

This is a deliberate, documented divergence, consistent with this repo's own
allowance for diverging from standard process when justified.

## Unit Anatomy

A **unit** is one ~5-minute lesson — the atomic thing that has to finish
inside a single commute leg. Every unit file follows the same four-part
structure, written in an audio-script style with bracketed delivery cues
(`[PAUSE Ns]`, `[REPEAT x2]`, `[YOU SAY: ...]`) so it can be read aloud by the
learner, a recorded voice, or eventually a TTS pipeline — that pipeline is
explicitly future work; this spec only guarantees the scripts are *shaped*
for it.

1. **Warm-up** (0:00–0:30) — a quick recall hook tied to the previous unit
   or a prior review.
2. **New material** (0:30–3:00) — 3-5 new vocabulary/grammar items. Every new
   item carries a compact etymology note: root language → root word →
   meaning shift → English cognate(s) where one exists. Every noun carries a
   grammatical-gender tag (see Grammatical Gender Methodology below).
3. **Grammar Lens** (3:00–3:30, *optional* — grammar-bearing units only) —
   three things, in order: (a) what this grammatical concept *is*, in plain
   terms, assuming no prior grammar vocabulary; (b) how English expresses
   the same function, with a concrete example; (c) what's actually different
   about how the target language does it. Skipped entirely for pure-vocabulary
   units (numbers, days of the week, etc.) where there's no grammatical
   structure to contrast.
4. **Guided practice** (3:30–4:30) — produce-on-your-own prompts (`[YOU SAY]`)
   that make the learner retrieve, not just recognize, the new material.
5. **Wrap-up recall** (4:30–5:00) — one retrieval question that seeds the
   next review of this unit.

## Unit Types

- `new` — introduces exactly **one** new concept (one grammar point, or a
  small cluster of 3-5 related vocabulary items). Never more than one new
  concept per unit — this is a hard constraint, not a guideline, because the
  learner explicitly wants each commute leg to add exactly one incremental
  thing on top of what's already solid.
- `review` — resurfaces prior vocabulary/grammar in a **new combination**,
  never a verbatim repeat of a prior unit's sentences.
- `practice-mix` — recombines several recently-seen concepts into short
  dialogue or connected sentences, closing out a session.
- `morphology` — teaches **one** Latin (occasionally Arabic) root, prefix, or
  suffix: its literal meaning, 2-4 derivative words in the target language,
  and 2-4 English cognates built from the same root. Roughly one per week,
  anchored to a root that's already surfaced incidentally in that week's
  vocabulary rather than an arbitrary pick. Scoped to **lexical** Latin
  (roots and word formation) — not formal Latin grammar (noun cases, verb
  paradigms); that would be a different, much larger undertaking and isn't
  what the learner asked for.

## Unit File Schema

One file per unit, YAML frontmatter + Markdown body:

```yaml
---
id: ES-P0-U03                # <lang-code>-P<phase>-U<sequence>
phase: 0
week: 1
type: new                    # new | review | practice-mix | morphology
concept_tag: GREETINGS-01    # shared across future language tracks (see Interleaving)
est_minutes: 5
prerequisites: [ES-P0-U01, ES-P0-U02]
new_vocab: [hola, buenos días, ¿cómo estás?]
reviews_of: []                # unit ids this unit resurfaces (review/practice-mix only)
root: null                    # {form, meaning, origin_language} — morphology units only
---
```

Body sections: `## Warm-up`, `## New Material`, `## Grammar Lens` (optional),
`## Guided Practice`, `## Wrap-up Recall`, matching the anatomy above.
`## New Material` entries use:

```
**hola** — hello
> Etymology: Spanish *hola* is generally traced to Old Spanish *¡ola!*, a
> greeting/hailing call, possibly influenced by nautical Arabic usage. Weaker
> etymology confidence than the Latin-derived items — flagged accordingly.
```

Nouns additionally carry an inline gender tag right after the word:

```
**la casa** (fem.) — house
> Etymology: ...
```

`morphology` units replace `new_vocab` with a `root` block and structure
`## New Material` as: literal meaning of the root, then two labeled lists —
derivative words in the target language, and English cognates from the same
root.

## Spaced Repetition: Session-Count Intervals

Reviews are scheduled in **session-counts, not calendar days** — the learner
won't drive exactly once a day, so a date-based schedule silently drifts out
of sync with actual usage. A concept introduced in session *N* is resurfaced
at sessions *N+1*, *N+3*, *N+7*, and *N+15*. After its fourth successful
resurfacing it rotates into a long-term pool that appears occasionally in
`practice-mix` units rather than on a fixed schedule.

This is **open-loop**: it assumes exposure ≈ retention, because there's no
app tracking pass/fail yet. If a real tracking mechanism is ever built (the
voice/app work called out below), this becomes adaptive — struggling items
get pulled forward, solid ones pushed back, Leitner/SM-2-style.

## Session Composition

A **session** = one commute leg, of variable length (the learner won't always
get the same drive time). Every session has a fixed **core block** designed
to finish in ~15-25 minutes so it never depends on the drive being long:

1. 2-4 `review` units currently due per the schedule above.
2. Exactly one `new` unit.
3. One `practice-mix` unit recombining the new material with recent reviews.

A **bonus queue** of additional `review`/`practice-mix` units is always
appended after the core block, so a 1-2 hour drive never runs dry — the
learner can stop after the core block on a short leg, or keep going.

`session-map.md` (per language track) lays out which units compose which
session and verifies the interval schedule is actually satisfied.

## Interleaving Across Languages (Framework, Not Yet Populated)

Every unit's `concept_tag` (e.g. `GREETINGS-01`, `NUMBERS-01`) is a shared key
across language tracks — Spanish, French, German, Arabic, Hindi, Tamil,
Kannada, Malayalam, and Telugu all tag their Chapter 1 greetings/pronoun/
numbers units the same way. A session can walk the *same* concept in two
languages back-to-back — contrastive reinforcement, and a natural place to
surface cross-language etymology (e.g. Spanish *gracias* vs. French *merci*
diverge in root, while Spanish/French/Italian generally converge on Latin
*gratia* for the "grace" family). The actual session-by-session
interleaving schedule across tracks is not yet built — each track's
`session-map.md` is still per-language — but the shared tags mean it can be
added without a schema rewrite.

## Etymology Methodology (Spanish Track)

Two root chains get deliberate emphasis:

1. **Latin → Romance cognates → English cognates** — the primary chain for
   most core vocabulary (*gracias* ← *gratia* → English *grace, gratitude,
   gratis, gratuity*). This is the strongest hook since the learner already
   has fluent English root-vocabulary to anchor to.
2. **Arabic → Spanish (Al-Andalus loanwords)** — Spanish carries roughly
   4,000 Arabic-derived words from ~800 years of Islamic rule in Iberia
   (*ojalá*, *azúcar*, *aceite*, *almohada*). This is both a genuinely
   interesting hook and a deliberate bridge to the learner's future Arabic
   track — words they'll meet again from the other direction.

Etymology confidence is marked inline where a root is disputed or uncertain
(see the `hola` example above) rather than stated as settled fact.

## Grammatical Gender Methodology

English marks gender almost nowhere — only in a handful of pronouns (*he,
she, it*), and there it tracks real-world sex/animacy, not an arbitrary
grammatical category. Spanish (like French, German, and Hindi later) marks
gender on *every* noun, and that gender forces agreement across articles and
adjectives — a structural feature English speakers have no built-in instinct
for. This gets treated as a **standing methodology**, not a single chapter:

- Every noun introduced from the very first one onward carries an inline
  gender tag (see the `la casa` example above) — building intuition through
  repeated exposure well before any formal rules unit explains the pattern.
- The formal gender chapter (Part I, per the roadmap) is where the pattern
  becomes explicit: the *-o*/*-a* default, common exceptions (*el día, la
  mano, el problema* — Greek-origin *-ma* nouns are masculine despite ending
  in *-a*), and why *el agua* takes the masculine article despite being a
  feminine noun (stressed initial *a* clashing with *la*). That chapter
  builds on tags the learner has already been seeing for weeks, rather than
  introducing gender cold.
- This generalizes: French and German have their own gender systems (German
  adds a third, neuter, and ties gender to case); Hindi marks gender on verbs
  as well as nouns; Tamil, Kannada, Telugu, and Malayalam have their own
  (different — rational/irrational noun-class systems in Tamil's case)
  classification schemes. Each future track gets the same treatment: tag
  early, formalize later, contrast explicitly with what English does (which
  is usually "nothing").

## Part 0 — Sounds & Letters (familiar scripts only)

A language track gets a standalone **Part 0** only when the learner already
*reads* its alphabet and just needs its **pronunciation** conventions —
Spanish, French, German (Latin alphabet). This is short — a handful of
units covering vowel purity, consonants that differ from English, and
stress/accent rules. It gets **no dedicated spaced-repetition cycle** of
its own: pronunciation and orthography are reinforced implicitly by every
single subsequent unit's target-language text, so a separate review
schedule would be redundant.

For any track where the *script itself* needs teaching or refreshing — a
genuinely new alphabet, or one the learner can read but is rusty on — there
is **no separate Part 0 at all**. See "Just-In-Time Script & Grammar
Introduction" below: letters are taught inline, inside real vocabulary
units, the same session structure as everything else, starting straight at
Chapter 1.

## Just-In-Time Script & Grammar Introduction

Per direct learner feedback, the "exactly one new concept per unit" rule
(Unit Types, above) generalizes to two more things beyond vocabulary/grammar
points, and both replace what would otherwise be a front-loaded review
phase:

- **Scripts.** No dedicated "learn/review the alphabet" chapter for any
  track that needs one. A brand-new script (Kannada, Malayalam, Telugu) or
  a rusty-but-known one (Arabic, Hindi) gets its letters introduced *inside*
  Chapter 1's vocabulary units, right where a word first uses them — each
  unit's New Material gets a short "new letters in this word" note
  alongside the vocabulary itself. This is a deliberate pedagogical choice,
  not just economy: learning a glyph attached to a real, meaningful word
  sticks better than memorizing an abstract alphabet chart before ever
  using it. A track's early chapters are chosen specifically to introduce a
  manageable, high-frequency set of letters this way (see Frequency-Driven
  Content Selection below) — by the time a script's first several chapters
  are done, a meaningful share of its alphabet has been covered through
  actual words, not a standalone drill.
- **Grammar.** Same principle, made explicit for tracks where the learner
  speaks the language but doesn't know its grammar formally (Tamil is the
  motivating case): grammar constructs are introduced one at a time, inline
  within vocabulary units, starting from the simplest construct that
  unlocks real sentences and building complexity from there — not a
  front-loaded grammar-reference dump. This was already true of every other
  track (Unit Anatomy's Grammar Lens section, "exactly one new concept");
  this note exists mainly to confirm it applies to a fluency-first,
  literacy-first track like Tamil just as much as to a from-scratch one.

## Frequency-Driven Content Selection

Chapter-by-chapter content — which words, which grammar construct, which
letters — is chosen by asking **"what does a basic real conversation
actually need first?"**, not arbitrary topic order. This isn't corpus-counted
frequency analysis, but reasoned judgment applied explicitly per chapter:
greetings, subject pronouns, a "to be" verb, and numbers 0-10 lead every
track's Chapter 1 because they're genuinely load-bearing — you can't
assemble almost any basic exchange without them. The same reasoning governs
which letters a new-script track's early chapters introduce (pick words
that are both common *and* efficiently cover new glyphs) and which single
grammar construct a track like Tamil starts with (whichever one unlocks the
most new sentence patterns for the least overhead).

## Cross-Language Comparison Web

Etymology Methodology (below) was written Spanish-specific — English
grounding, Latin root chain, Arabic loanword thread. As more tracks come
online, comparisons generalize into an **accumulating hierarchy**: each new
language, in the order it's added to the curriculum, compares its
vocabulary against *every language already established before it*
(English is the permanent base across all tracks), and contributes its own
"deep root" system, which becomes available to whatever comes after it too.

| Order | Track | Compares against | Contributes |
|---|---|---|---|
| 1 | Spanish | English | Latin |
| 2 | French | English, Spanish | Latin (via Old French — occasionally contrasted with Spanish's own routing of the same root, e.g. *gratia*→*gracias* vs. *merci*, which is actually a different root, *merces*) |
| 3 | German | English, Spanish, French | Latin (German's own loanword layer, e.g. *Fenster*←*fenestra*) — Germanic-**inherited** words are already covered by the English comparison, since English is itself Germanic |
| 4 | Arabic | English, Spanish, French, German | Arabic itself, from here on — Al-Andalus loans into Spanish traced *outward* this time, complementing Spanish's own backward trace |
| 5 | Hindi | English, Spanish, French, German, Arabic | Sanskrit + Persian/Arabic loanwords (using the Arabic thread just established) |
| 6 | Tamil | English, Spanish, French, German, Arabic, Hindi, Sanskrit | Tamil/native-Dravidian, from here on |
| 7 | Kannada, Malayalam, Telugu | all of the above | — |

A given word only shows the comparisons that are **genuinely real** for
it — not every accumulated language forced into every entry. By the time a
track has 7-8 possible comparison languages available, most words connect
meaningfully to 2-3 of them, and that's what gets shown.

## Book Format

Each language track produces a **LaTeX book** — one publishable volume per
language, meant to eventually be released for free (CC BY-SA 4.0) — living
at `<track>/book/`. `\documentclass{book}`, compiled with **XeLaTeX**
(`fontspec` + `polyglossia`, not pdflatex) specifically because later tracks
need proper Unicode and right-to-left/complex-script typesetting that
pdflatex can't do. The book is `\input{}`-assembled from one chapter file per
authored Part/Chapter, and grows incrementally — a new chapter file plus one
`\input` line each time a week of units gets authored, never all at once.

The book and the practice units (`<track>/units/*.md`) are **two views of
the same content**, not independently maintained documents: the book is the
polished, continuous-reading edition; the units are the same material sliced
into 5-minute, spaced-repetition-scheduled, car-consumable pieces. Grammar
Lens sections, gender tags, and morphology spotlights all appear in both.

## Roadmap Shape

Each language track has a `roadmap.md`: a year-long, CEFR-informed
Part/Chapter skeleton (topic list per chapter — not fully scripted)
targeting B1 ("normal day-to-day conversation") by year-end. Parts/Chapters
are authored and scripted incrementally, starting from Part 0, in the order:
units (`units/*.md`) first, then that chapter's LaTeX file folded into the
book. The roadmap file is the standing plan, updated as later chapters get
fully authored.

## Explicitly Out of Scope (This Pass)

- **Voice delivery** (TTS reading units aloud) and **speaking practice**
  (STT scoring the learner's spoken answers) — both require a voice model
  and app-level infrastructure. This spec's unit format is written to be
  voice-ready (explicit pause/repeat cues) so that work can consume these
  files directly when it happens, but building it is not part of this pass.
- **Adaptive scheduling** based on actual recall performance — requires the
  tracking mechanism above; the current schedule is open-loop.
- **Formal Latin grammar** (noun cases, verb conjugation paradigms) — the
  morphology thread is lexical only (roots/prefixes/suffixes and how words
  are built). If real Latin grammar instruction is ever wanted, that's a
  candidate to become its own track, not a widening of this one.
- **Depth beyond Chapter 1** for French, German, Arabic, Hindi, Tamil,
  Kannada, Malayalam, Telugu — all eight now have a real Chapter 1 (or, for
  Tamil, an equivalent first grammar+vocabulary chapter), but only Spanish
  has been carried further (through Part I). Each track deepens the same
  way Spanish does, chapter by chapter.
