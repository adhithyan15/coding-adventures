# HL03 — The Unified Language-Learning App

## Overview

There should be **one** app, not a scatter of separate tools. Earlier work
produced a `script-writing-visualizer` (now renamed `language-ladder`) with four disconnected modes
(Browse / Practice / Lessons / Concepts) plus a handful of standalone
exploratory artifacts (a script field-guide, a spot-the-script quiz, a
letter-reading trainer). Each was useful in isolation and wrong as an
architecture: a learner does not want ten apps.

This spec defines the single app they *do* want. Its spine is the **curriculum**:
it walks the lessons in book order, and everything else — a script, a grammar
point, a quiz, a review — appears **when a lesson reaches it**, not as a
separate destination. It is built for a learner studying **many languages at
once**, and its distinctive job is to surface the **connections between them**
so those connections become the memory.

It supersedes HL02's framing (a "companion practice app") by absorbing it: HL02's
[interleaving scheduler and SRS](HL02-companion-practice-app.md) remain the
review engine; HL01's [concept taxonomy and data layer](HL01-concept-taxonomy-and-data-layer.md)
remains the substrate. What is new here is the **unification** around the
curriculum, the **language-chain / spiral** sequencing, the **as-needed**
introduction of scripts and grammar, and a **mistakes** layer.

## The learner's model — a spiral with cumulative concept-sweeps

HL02 stated the pattern in miniature (Spanish → French → German). The real model
the app must encode, in the learner's own words, is larger and more structured:

1. **A fixed chain of languages**, each added so that it *connects to what came
   before*:

   | # | language | the connection it opens |
   |---|----------|--------------------------|
   | 1 | Spanish | the starting point |
   | 2 | *Latin* | the **roots** beneath Spanish |
   | 3 | French | Romance cognates with Spanish/Latin |
   | 4 | German | the **English twins** (Germanic cognates) |
   | 5 | Arabic | a Semitic, right-to-left departure |
   | 6 | Hindi | the great connector — **Arabic + Persian + Sanskrit** roots all meet here |
   | 7 | Tamil | the first **Dravidian** language |
   | 8 | Kannada | Tamil's cousin, but also borrows from **Sanskrit** (links back to Hindi) |
   | 9 | Telugu | Dravidian, Sanskrit-influenced |
   | 10 | Malayalam | Dravidian, the most Sanskrit-laden |

   The chain is not arbitrary: each hop is chosen because a bridge already exists
   to a language the learner knows. Latin explains Spanish; German's vocabulary
   rhymes with English; Hindi reaches *back* to Arabic (Perso-Arabic vocabulary)
   and *forward* to Sanskrit (which the Dravidian languages then borrow). The
   chain is a connected graph walked in one order.

2. **Advance concept by concept.** Progress follows the *book's* concept order
   (greetings, then yes/no, then numbers, …). A concept is not "done" in one
   language; it is covered **across every language active so far, in chain
   order**, so the cognates, shared roots, and false friends line up side by side
   at the moment of learning.

3. **The sweep is per-concept, not a full replay.** Learning the *current* (most
   recent) concept means covering **that one concept** across the active chain
   from the start — Spanish → Latin → French → German → … — so its cognates,
   roots, and false friends line up. Adding a new language simply **extends the
   current concept's sweep** to include it; it does *not* re-teach every earlier
   concept. Teaching moves forward, one concept at a time.

4. **Review is a separate, randomised cumulative quiz.** Retention comes not from
   re-teaching but from **interleaved retrieval across the whole grid of
   (concept × language) covered so far**. A review draws items at random from
   everything learned — *"what is 5 in Telugu? 12 in Latin? 3 in Arabic?"* — mixing
   concepts and languages maximally. This is the retrieval-practice engine
   (HL02's SRS + `interleave.ts`), and it — not the sweep — is the primary review
   mechanism. The sweep teaches a concept across the chain; the quiz pulls any
   learned concept in any learned language, unpredictably.

This is progressive cross-language interleaving (HL02) plus an explicit
**chain sequence**, a per-concept **sweep** that teaches forward, and a
**randomised cumulative quiz** that reviews across the entire covered grid.

## The unified session flow

A study session is concept-centred, not mode-centred:

1. The app picks the learner's **current concept** (the next unmet concept in
   book order for the newest active language), or a **due review** concept.
2. It walks that concept **across the active chain in order**. For each language
   that teaches the concept, it presents the lesson content, and —
3. **surfaces the connection** to the languages already seen this pass: the
   shared root, the cognate, the false friend, or (for a new script) the letter
   forms needed to read it.
4. It **quizzes** — recognition first, then recall — mixing the concept across
   languages so the learner discriminates and links them.
5. **Mistakes are logged** (see below) and **fed back** into the SRS queue, so a
   missed item resurfaces sooner in the randomised cumulative quiz.

The four old modes become *facets* of this one flow: "Browse" is how a script is
introduced, "Practice" is the quiz step, "Concepts" is the cross-language link
surfaced at step 3, "Lessons" is the spine itself.

## Connections — the memory hooks, and where they come from

The connective tissue already lives in the curriculum data; the app reads it, it
does not invent it:

| connection | source in the data | example |
|------------|--------------------|---------|
| shared root / etymology | lesson `roots`, `etymology_hook` | *gato · gatto · chat* ← Latin *cattus* |
| same concept across languages | lesson `concept_tag` (HL01) joined by `concepts.ts` | "thank you" in every active language |
| false friend | script/lexical lookalikes (`core.ts` `falseFriends`) | *embarazada* ≠ *embarrassed* |
| shared / contrasting script | `data/scripts/*.json` incl. the new `signature` (PR #8810) | Devanagari's head-line vs Gujarati's absence of one |

Nothing about a connection is authored from memory: roots and etymologies come
from the lessons (which are themselves sourced), and script signatures were
verified by rendering the font.

## Scripts and grammar, introduced as needed

- **Scripts.** When the chain reaches a language in a new script (Arabic at #5,
  Devanagari at #6, Tamil at #7, …), the app introduces exactly the letters that
  the concept's words need, using the letter-recognition and — where sourced —
  stroke-order material (the field-guide, quiz, and letter-trainer artifacts are
  folded in here as steps, then retired as standalone pages). A letter is
  introduced the first time a word requires it, and thereafter drilled by the
  SRS like any other item.
- **Grammar.** Grammar points attach to the lessons that need them (a gender
  system when the first gendered noun appears, a case ending when the first case
  contrast appears) rather than as a separate syllabus. Grammar items enter the
  same review queue.

## Review — the randomised cumulative quiz

The primary review is a quiz that draws at random from the **entire grid of
(concept × language) covered so far**, mixing concepts and languages maximally
so nothing is predictable: *"5 in Telugu? 12 in Latin? 3 in Arabic?"* This is
interleaved retrieval practice — the strongest lever for retention — built on
`interleave.ts` (the cross-language pool) and `scheduler.ts` (which weights the
draw toward items that are due or previously missed). It is distinct from the
teaching sweep: the sweep walks one concept across the chain in order; the quiz
pulls any learned item in any learned language, unordered.

## Mistakes

The one genuinely new subsystem.

- Every quiz answer is recorded: the item, the prompt direction, right/wrong,
  and *what the learner chose* when wrong (the confusion, not just the miss).
- Mistakes **demote** the item in the SRS (HL02 boxes) so it returns sooner, and
  are **collected into a review** the learner can open directly ("the letters I
  keep confusing", "the false friends that catch me").
- A wrong answer that reveals a *cross-language confusion* (choosing the French
  cognate's meaning for the Spanish word) is surfaced as a connection to
  reinforce, not merely an error to repeat.
- "What you learned in the past" is the union of mastered items, browsable by
  concept and by language, and continually re-touched by the randomised
  cumulative quiz.

## Architecture — reuse, don't rebuild

The engine largely exists in `code/programs/typescript/language-ladder/src`:

- `lessons.ts` — reads the written `.md` curriculum (chapter order,
  `prerequisites`). The spine.
- `concepts.ts` — cross-language concept join + prerequisite gating. The
  connections.
- `interleave.ts` — the cross-language practice pool.
- `scheduler.ts` — Leitner SRS (`INTERVALS = [1,1,3,7,15,30]`). The review.
- `truetype.ts` / `strokes.ts` — authentic glyph outlines and the sourced
  stroke ductus. Script introduction.
- `data/scripts/*.json` — script data incl. `signature`.

What must be built: a **language-chain / sequencing** layer (the ordered chain +
the per-concept sweep), a **session orchestrator** that walks a concept across
the chain and surfaces connections, the **randomised cumulative quiz** over the
covered (concept × language) grid, the **mistakes** store, and a UI that
presents one flow instead of four modes. The app has been **renamed** from
`script-writing-visualizer` to **`language-ladder`** to reflect what it now is.

## Scope and phasing

Built incrementally, one bounded piece per PR (this session's lesson: do not
rush large work):

1. **This spec** (HL03), committed first.
2. **Sequencing layer** — encode the language chain and the concept-across-chain
   walk as a pure, tested module (no UI). Deterministic; controls that bite.
3. **Session orchestrator** — assemble a teaching sweep from (current concept ×
   active chain), pulling connections from `concepts.ts` + `roots`. Pure + tested.
4. **Randomised cumulative quiz** — draw items across the whole covered
   (concept × language) grid, weighted by the SRS. Pure + tested; a control
   proves the draw actually spans concepts and languages, not one bucket.
5. **Mistakes store** — record answers and the chosen-wrong confusion, demote in
   SRS, surface a "what I keep confusing" review. Pure + tested.
6. **Unify the shell** — fold the four modes into the one session flow; rename
   the app; retire the standalone artifacts as separate pages.
7. **As-needed script/grammar introduction** wired into the walk.

Each step verifies in the browser (screenshot + read the render) and ships only
with tests that fail on the pre-change behaviour.

### Status — shipped

All of the above shipped as the app **`language-ladder`** (renamed from
`script-writing-visualizer`, v0.16.0), each step its own reviewed PR:

- **2–5** — the pure engine: sequencing (`sequence.ts`), the session orchestrator
  (`session.ts` + `sessionplan.ts`), the SRS-weighted cumulative quiz (`quiz.ts`),
  and the mistakes store (`mistakes.ts`), all deterministic and unit-tested with
  controls that bite.
- **6** — the unified shell: **Learn** mode folds the teaching sweep (with the
  grounded cross-language connections) and the review quiz into one flow; the app
  was renamed; the standalone exploratory artifacts were a no-op to retire (they
  were only ever ephemeral, never committed).
- **7** — **script introduction** is wired in (`scriptintro.ts`): a "new script"
  note the first time the walk reaches a non-Latin script, from the scripts'
  `signature` data. **Grammar introduction was parked**, not built: the curriculum
  carries a single grammar concept tag (`GRAMMAR-THE`) with no explanation field —
  too thin to ground an honest note the way scripts have `signature` data.
- **Polish beyond the plan**: localStorage persistence for the review
  (`reviewstore.ts`), the teaching cursor (`cursorstore.ts`), and a "reset
  progress" clearer (`reset.ts`); resume-where-you-left-off; a jump-to-concept
  picker; and a spine progress bar. One candidate was **abandoned as un-grounded**:
  romanization under the quiz options — the `romanization` field is populated for
  only ~54 of ~700 lessons (and not the Indic vocabulary, whose romanization lives
  in the gloss text), so showing it would be inconsistent rather than helpful.

The two honest non-builds (grammar intro, romanization) can return if the
curriculum grows the metadata they need.

## Verification

- Pure modules (sequencing, orchestrator, mistakes) are deterministic and unit-
  tested, each honesty check paired with a control that fails on broken input.
- Any claim about a connection (a shared root, a false friend) traces to the
  curriculum data, never to memory — the same grounding rule as the rest of the
  project.
- UI changes are screenshotted and read before being called done.
