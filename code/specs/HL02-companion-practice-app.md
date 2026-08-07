# HL02 — Companion Practice App & the Learning-Science Method

## Overview

The Human Languages **book** (`HL00`) teaches; this **app** drives *practice to
mastery*. The division of labour is deliberate and load-bearing:

> **You learn a concept from the book. Then you practise it in the app until it
> sticks — across every language you're learning, on the app's schedule, not
> yours.**

The app is a **companion**, not a replacement: it never teaches a concept cold.
It walks the book's already-learned content in **randomized** order, **tracks
progress per language**, and sequences practice using evidence-based learning
science. Its data comes entirely from the `HL01` data layer, so it stays a true
mirror of the book.

**v1 is reading-focused.** It builds recognition (glyph → sound/meaning) and
recall (prompt → pick the glyph), and — for the scripts the learner can't yet
read — it **decomposes each character into learnable pieces** so the learner can
practise *writing on paper*. There is no in-app handwriting capture in v1.

## The learner's method, which this app is built around

The user described a specific practice pattern that works for them, and the app's
scheduler is built to execute exactly it:

> Learn a word in Spanish. Then learn the same word in French. Then review both
> Spanish and French together. Then add German. Then review Spanish, French, and
> German together. And so on.

This is **progressive cross-language interleaving**: introduce one new
(concept, language) at a time, then immediately fold it into cumulative,
mixed review of everything introduced so far. It is not an idiosyncrasy — it
is, almost exactly, the combination of learning-science principles below applied
to a multilingual learner. The app generalizes it into a scheduler (see
"The interleaving scheduler").

## What the science says actually works (and how the app uses each)

The user asked, specifically, what neuroscience says works for learning a script.
The app commits to the following findings; each maps to a concrete mechanism.

| Principle | Finding | Mechanism in the app |
|---|---|---|
| **Retrieval practice / testing effect** | *Being tested* on material builds durable memory far better than restudying it (Roediger & Karpicke). The most robust result in the field. | Every item is a **test**: recall the sound, choose the glyph — never a passive flashcard flip. "Reveal" comes *after* an attempt. |
| **Spaced repetition** | Memory is strengthened most when review happens as the trace is about to fade; expanding intervals beat massed practice. | An **adaptive scheduler**: each item's next-due gap expands on success, contracts on failure (Leitner/SM-2-lite), measured in **sessions**, matching `HL00`'s N+1/N+3/N+7/N+15 open-loop baseline but now closed-loop. |
| **Interleaving** | Mixing related items in one session beats blocking one item at a time — it forces discrimination and transfers better (Rohrer & Taylor). | The session **mixes concepts and languages**. The learner's cross-language method *is* interleaving; it is the scheduler's default ordering. |
| **Generation effect** | Producing an answer beats recognizing one. | Recall mode makes you **produce/choose the glyph** from distractors, not just rate your recall. |
| **Elaborative encoding** | New material bonds to memory by connecting to what you already know. | Each item surfaces its **etymology hook** (the `HL01` `etymologyHook` / the book's cousin web) on reveal — the same principle the whole curriculum is built on. |
| **Dual coding** | Pairing verbal + visual codes strengthens recall. | Items pair **glyph + sound + meaning + root** simultaneously. |
| **Desirable difficulty** | Effortful (but succeeding) retrieval encodes better than easy retrieval. | Spacing, interleaving, and generation are all tuned to keep retrieval *just* hard enough — the app avoids re-showing an item while it's still easy. |

### Specifically for learning a script

Reading a new writing system is **grapheme→phoneme mapping** plus, for the Indic
abugidas, learning a **combination system**. Two evidence-aligned commitments:

1. **Teach the system, not the syllabary.** A Telugu or Kannada abugida generates
   thousands of syllables from ~40 base consonants × ~12 vowel signs + conjunct
   rules. Drilling syllables as flat flashcards fights the structure. The app
   drills the **bases, the vowel-sign transformations, and the conjunct rule**,
   so the learner internalizes the generator (this is why the `HL01` script data
   is compositional).
2. **Letters in real words.** Consistent with `HL00`'s "letters taught inside the
   word," the app's script items are drawn from actual headwords the learner is
   studying, not an abstract chart — the word is the vehicle for its letters.

## The interleaving scheduler (the core module)

A **pure, deterministic, unit-tested** module — the heart of the app, and where
test coverage matters most. Given the learner's history and a random seed, it
produces the next session's item queue.

### State per item

An **item** is a (concept, language, mode) triple — e.g. (`GREETING-HELLO`,
`telugu`, `recognize`). Its state:

```ts
interface ItemState {
  box: number;         // Leitner box 0..N; higher = longer interval
  dueAtSession: number;// session index when it next becomes due
  introducedAt: number;// session it first appeared
  lapses: number;      // times failed after being learned
}
```

### The progressive-introduction rule (the user's method, generalized)

1. **One new thing per step.** A session introduces a *small, bounded* number of
   new items (default: 1–3, configurable), never a flood. A "new item" is a
   (concept, language) the learner has marked learned-in-the-book but hasn't
   practised yet.
2. **Cross-language before cross-concept.** When the learner is studying a
   concept in multiple languages, the scheduler prefers to introduce *the same
   concept in the next language* over a brand-new concept — reproducing "learn it
   in Spanish, then the same word in French." Order of languages follows the
   learner's configured priority.
3. **Cumulative interleaved review.** After the new item, the session fills with
   **due** items from the whole history, **interleaved** across concepts and
   languages (shuffled by the seed), oldest-most-overdue first. This is the
   "review Spanish + French together, then add German, then review all three."
4. **Expanding spacing.** A correct answer promotes the item a box (next due gap
   ×≈2); a wrong answer demotes to box 0 (due next session) and increments
   `lapses`. Baseline gaps follow `HL00`: 1, 3, 7, 15 sessions, then a long-term
   pool.

### Determinism & testability

The scheduler takes `(history, config, seed)` and is a pure function — no clock,
no `Math.random` inside (seeded PRNG passed in), matching this repo's constraint
that `Date.now`/`Math.random` are avoided in deterministic cores. Invariants the
tests assert:

- a new item is introduced **exactly once**;
- an item answered correctly at session *N* does not reappear before its box
  interval elapses;
- a lapsed item reappears at *N+1*;
- the due-set ordering is **stable for a fixed seed** (reproducible sessions);
- cross-language preference: with two languages queued for the same concept, the
  second language's item is introduced before an unrelated new concept.

## Progress model

Per-language, persisted in `localStorage` (no backend, matching repo app
convention). Shape:

```ts
interface Progress {
  version: 1;
  sessionIndex: number;                    // monotonic session counter
  learnedInBook: Record<string, boolean>;  // "concept|language" → learned flag
  items: Record<string, ItemState>;        // "concept|language|mode" → state
  perLanguage: Record<string, {            // rollups for the dashboard
    learned: number; mastered: number; due: number;
  }>;
}
```

- **`learnedInBook`** is the gate: an item can't enter practice until the learner
  flags they've met it in the book. This keeps the app a companion, never a
  first-teacher.
- **"Mastered"** = reached the top Leitner box with no recent lapse.
- Export/import the JSON (a settings action) so progress survives a browser wipe;
  no accounts.

## Practice modes (v1)

1. **Recognize** — show the glyph/word; learner recalls sound + meaning; reveal
   shows romanization, gloss, and the etymology hook. (Reading, testing effect.)
2. **Recall / generate** — show the romanization + gloss; learner **picks the
   correct glyph/word** from distractors drawn from the same script/language
   (generation effect, reading-friendly — no handwriting needed).
3. **Character breakdown (study, not tested)** — for a non-Latin script, a screen
   that decomposes the selected glyph into its **components** and shows the
   **typical stroke order** and the vowel-sign/conjunct system around it (from
   `HL01` `data/scripts/*.json`). This is the "learn to write, piece by piece"
   surface, for paper practice. Reading-only; no in-app tracing in v1.

Distractor selection for Recall is its own small tested helper: same-script,
same-length-ish, visually-confusable-preferred glyphs, seeded.

## Architecture

`code/programs/typescript/human-languages-companion/`, scaffolded with the repo's
**`web-app-scaffold-generator`** (`visualization` template):

- **Vite + React 19 + TypeScript + Vitest**, styled with the internal **Lattice**
  stack (`vite-plugin-lattice`), deployed to GitHub Pages via a
  `deploy-human-languages-companion.yml` workflow — all standard for this repo.
- Depends on **`@coding-adventures/human-language-data`** (`HL01`) via a `file:`
  dependency for the concept dataset and script data.
- Renders every script with the **vendored Noto fonts already in `_fonts/`**
  (bundled via Vite `?url` imports / `@font-face`), so local and deployed builds
  render identically — the same guarantee the books make.
- Layers: **data** (thin wrapper over the `HL01` package) → **scheduler** (pure)
  → **progress store** (localStorage adapter) → **React UI** (session runner,
  mode screens, per-language dashboard, character-breakdown view). The scheduler
  and store are pure/mockable and carry the coverage weight; the UI is thin.

## Extensibility — adding a script or language (the Gujarati on-ramp)

The whole stack is built so that a new language or script is **data, not code**:

- **A new language track** = a new `lessons/*.md` set tagged with canonical
  concepts (`HL01`) + its script declared. It appears in the app automatically
  once the data layer picks it up. No app code changes.
- **A new script** (e.g. **Gujarati**, explicitly wanted) = (1) author the
  Gujarati HL track per `HL00`; (2) vendor `NotoSansGujarati-Static.ttf` into
  `_fonts/`; (3) author `data/scripts/gujarati.json` per `HL01`. The app's
  character-breakdown view and drills then work for it with no new code.

Gujarati has none of these today (no track, no font, no data), so it is a
**documented future on-ramp**, not part of v1 — but v1 must be built so that
delivering it is exactly the three data steps above and nothing more. A
`docs/adding-a-script.md` in the app records the checklist.

## v1 Scope

**In:** the three practice modes above; the interleaving scheduler; per-language
progress in localStorage; the character-breakdown study view for all vendored
non-Latin scripts; a per-language dashboard; export/import of progress.

**Out (future):** in-app handwriting/tracing capture and scoring; audio/TTS
playback and speech scoring (the `HL00` "voice work"); server-side accounts/sync;
Gujarati and any script without a vendored font + `HL01` data; grammar-drill
modes beyond vocabulary/script recognition.

## Verification

- **Scheduler**: `vitest` unit tests asserting every invariant in "Determinism &
  testability" above; ≥95% coverage on the scheduler and distractor helper.
- **Progress store**: tests for persistence round-trip, migration/versioning,
  export/import.
- **End-to-end**: `preview_start` the dev server; drive a full session with the
  browser tools — pick a language, flag a concept learned-in-book, run a
  Recognize drill and a Recall drill, open a character breakdown, reload the page
  and confirm progress persisted; verify each script's glyphs render with the
  vendored font; screenshot as proof.
- **Data contract**: the app's data wrapper is tested against the `HL01`
  validator's output so a taxonomy/lesson change that breaks the app fails CI.
