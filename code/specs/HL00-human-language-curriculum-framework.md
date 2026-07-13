# HL00 — Human Language Curriculum Framework

## Overview

**HL** (Human Languages) is a personal, etymology-driven curriculum for learning
spoken languages, designed for delivery during a daily car commute rather than
through a screen. It is content, not software: the deliverable is a structured
set of Markdown lesson files, not a running application.

This is the first spec in the **HL** series. It defines the pedagogical
framework, the content schema every lesson file follows, and the divergence
from this repo's standard software-package process. `HL01`+ are reserved for
per-language notes as new tracks are added (French, German, Arabic, Hindi,
Tamil, Kannada, Telugu, Malayalam) after the Spanish pilot proves the format.

```
                    ┌───────────────────────────┐
  Roadmap (year) ───►                           │
                    │   Week → Session → Unit    ├──► Markdown lesson files
  Etymology data ───►   (this spec's schema)     │    (audio-script formatted)
                    └───────────────────────────┘
```

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
2. **New material** (0:30–3:30) — 3-5 new vocabulary/grammar items. Every new
   item carries a compact etymology note: root language → root word →
   meaning shift → English cognate(s) where one exists.
3. **Guided practice** (3:30–4:30) — produce-on-your-own prompts (`[YOU SAY]`)
   that make the learner retrieve, not just recognize, the new material.
4. **Wrap-up recall** (4:30–5:00) — one retrieval question that seeds the
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

## Unit File Schema

One file per unit, YAML frontmatter + Markdown body:

```yaml
---
id: ES-P0-U03                # <lang-code>-P<phase>-U<sequence>
phase: 0
week: 1
type: new                    # new | review | practice-mix
concept_tag: GREETINGS-01    # shared across future language tracks (see Interleaving)
est_minutes: 5
prerequisites: [ES-P0-U01, ES-P0-U02]
new_vocab: [hola, buenos días, ¿cómo estás?]
reviews_of: []                # unit ids this unit resurfaces (review/practice-mix only)
---
```

Body sections: `## Warm-up`, `## New Material`, `## Guided Practice`,
`## Wrap-up Recall`, matching the anatomy above. `## New Material` entries use:

```
**hola** — hello
> Etymology: Spanish *hola* is generally traced to Old Spanish *¡ola!*, a
> greeting/hailing call, possibly influenced by nautical Arabic usage. Weaker
> etymology confidence than the Latin-derived items — flagged accordingly.
```

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
across future language tracks. Once a second language track exists, a session
can walk the *same* concept in two languages back-to-back — contrastive
reinforcement, and a natural place to surface cross-language etymology (e.g.
Spanish *gracias* vs. French *merci* diverge in root, while Spanish/French/
Italian generally converge on Latin *gratia* for the "grace" family). Only
the Spanish track is populated in this pass; the schema is designed for this
now so later tracks slot in without a rewrite.

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

## Roadmap Shape

Each language track has a `roadmap.md`: a year-long, CEFR-informed phase
skeleton (topic list per phase/week — not fully scripted) targeting B1
("normal day-to-day conversation") by year-end. Phases are authored and
scripted incrementally, week by week, starting from Phase 0; the roadmap
file is the standing plan, updated as later weeks get fully authored.

## Explicitly Out of Scope (This Pass)

- **Voice delivery** (TTS reading units aloud) and **speaking practice**
  (STT scoring the learner's spoken answers) — both require a voice model
  and app-level infrastructure. This spec's unit format is written to be
  voice-ready (explicit pause/repeat cues) so that work can consume these
  files directly when it happens, but building it is not part of this pass.
- **Adaptive scheduling** based on actual recall performance — requires the
  tracking mechanism above; the current schedule is open-loop.
- **Non-Spanish tracks** — French, German, Arabic, Hindi, Tamil, Kannada,
  Telugu, Malayalam all reuse this framework once the Spanish pilot proves
  it out.
