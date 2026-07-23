# Language Ladder

**The HL03 unified curriculum learning app** (it began life as the HL02
`script-writing-visualizer` and has subsumed that app's modes). Five modes.
**Learn** (the default) walks the curriculum the way the book does — one concept
at a time, forward along the language chain, each new language showing its
threads back to the ones already learned, then reviewing it all with a
randomised SRS quiz. **Browse** and **Practice** work on *script letters*;
**Lessons** drills the *written curriculum* — every lesson in every track — on a
spaced-repetition schedule that persists between visits; and **Concepts** shows
one idea in every language that has it, side by side.

## Learn mode (the curriculum session)

The spine of the app: [HL03](../../../specs/HL03-unified-language-learning-app.md)
in one screen. For each concept in book order (`sweepableConcepts`), the
engine's *teaching pass* (`sessionplan.ts` → `planSession`) is rendered as a
numbered sweep — one card per language that teaches it, in chain order
(Spanish → Latin → French → … → Malayalam). Each card carries the word in its
own script, its etymology hook, and the **connections back** to earlier
languages that share a root, so the cross-language memory the interleaving is
meant to build is made visible rather than left implicit. Prev / Next walk the
spine; consolidation lessons (`practice`/`review`) are left to the review quiz,
not the teaching sweep.

Below the sweep sits the **review pass** — the second mechanism. A randomised,
SRS-weighted quiz draws over everything covered so far (`plan.reviewGrid`, the
concept×language grid up to the cursor), leaning on what you keep missing
(`pickNext`). Each question is *"‹meaning› — in ‹language›?"* with options drawn
from the **same concept in other languages** — the cross-language look-alikes
the interleaving targets (Telugu ధన్యవాద vs Hindi धन्यवाद). Answers thread
through `applyAnswer` (promote a hit, demote + log a miss), and a *"what you keep
confusing"* panel rolls the mistakes up from `confusions(log)`. The review
**persists** (`reviewstore.ts`): its SRS state and answer log are saved to
`localStorage` after every answer and restored at startup — the same pattern
`progress.ts` uses for the lesson schedule, with the same defensive parse (a
corrupt or wrong-version blob restores as empty, never throws).

## Concepts mode

The curriculum tags lessons with a shared `concept_tag`, and canonical tags are
deliberately the same across tracks. That makes them a **join key**: *gracias /
merci / danke / धन्यवाद / നന്ദി* are one concept realized eighteen ways.

This mode is that join, and it is the data package's own
`languagesForConcept` — a function shipped from the start, documented as "what
the companion app calls," which until now had **no caller**.

- **39 concepts are shared by two or more tracks**, from 701 lessons. A concept
  only one language tags is filtered out: there is nothing to compare it with,
  which also removes almost every namespaced (`ES-…`) tag without a special case.
- Each row shows the **headword**, a **romanization** where it differs, and the
  gloss — so a non-Latin script is legible next to a Latin one.
- The **etymology hooks** sit underneath the comparison, where they do the most
  work: *gracias* ← *gratia* "favour", *merci* ← *mercēs* "wages", *danke* ←
  *denken* "to think", *спасибо* ← *спаси Бог* "God save you". Four languages'
  words for the same courtesy, from four unrelated ideas.

## Lessons mode

Reads all **701 lessons across 18 languages** straight from the curriculum via
`@coding-adventures/human-language-data`, and schedules them with the same
Leitner machinery the letter drills use (`scheduler.ts` is generic over an
index; it never needed to know what an item is).

- **It remembers you.** Progress is saved to `localStorage` keyed by **lesson
  id** — never by array index, because indices shift every time a lesson is
  added and saving by position would reattribute your history to the wrong
  lesson. New lessons simply appear as unseen items.
- **It mixes languages.** Consecutive reviews round-robin across tracks — Arabic,
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
- **Write it** — a conventional stroke order, numbered;
- a **⚠ false friend** badge for letters that look like a Latin letter but
  aren't (Cyrillic *в*=v, *р*=r, *с*=s, *н*=n) — the fastest way into the script.

## Where it fits

```
code/learning/human-languages/data/scripts/*.json   ← the source of truth (HL01)
        │  (glyph, components, strokeOrder, notes per letter)
        ▼
language-ladder                                     ← this app renders it (HL03)
```

The app imports those JSON files **directly**, so it can never drift from the
curriculum. Adding a script to the curriculum surfaces it here with a one-line
edit in `src/data.ts`. Ships today with **Cyrillic, Hebrew, Chinese, Arabic, and
Devanagari**.

## Design

- **`src/core.ts`** — the pure, unit-tested heart: `buildScriptView`,
  `scriptSummary`, `isFalseFriend`, `falseFriends`. No DOM, no globals; this is
  where the pedagogy is tested.
- **`src/data.ts`** — the only place that imports the canonical script JSON.
- **`src/main.ts`** — a deliberately framework-free vanilla-DOM shell.

## Develop

```sh
npm install
npm run dev        # local dev server
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
