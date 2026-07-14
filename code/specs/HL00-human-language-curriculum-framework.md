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
  Roadmap ─────────►                           ├──►   (audio-script formatted,
                    │   Chapter → Lesson         │       car-commute practice)
  Etymology data ───►   (this spec's schema)     │
                    │  (one word/phrase = 1      ├──►   LaTeX book, one volume
                    │   lesson, gone deep)       │       per language (publish)
                    └───────────────────────────┘
```

The unit of the curriculum is the **lesson**: one word or one fixed phrase,
excavated fully in a few minutes. Lessons are grouped into **chapters**
(themed clusters). That's the whole hierarchy — chapters of single-word
lessons. (Earlier drafts used a coarser "Part → Chapter → Unit" scheme with
`phase`/`week` frontmatter and a front-loaded "Part 0" of pronunciation;
both were removed on learner feedback in favor of this finer, gate-free
model. Some legacy `ES-P0-U##` files from that scheme may still exist mid-
migration.)

## Audience — who the books are for

**The books are written for a motivated *true beginner* — someone with no
prior knowledge of the target language (or of any of the others), who wants
to learn it in depth.** They are *not* written for the curriculum's
originator or any particular person. This is a load-bearing rule, because it
constrains what the text may assume:

- **The only language the reader is assumed to know is English** (the books
  are written in English). Every explanation grounds in English + the
  language's own deep roots (Latin, Sanskrit, Semitic roots, etc.), because
  those are universally available to the reader.
- **No lesson may assume the reader knows another *target* language.** Phrases
  like "you already know this from Spanish," "the Spanish twin you know,"
  "if you're also learning Spanish," or "skim this, it's already yours" are
  forbidden — they address a specific reader, not the beginner the book is
  for.
- **Cross-language comparisons are still welcome — as self-contained
  enrichment.** Showing that Spanish *día* and French *jour* both descend from
  Latin *dies* is wonderful depth, and a beginner learns *both* facts fresh
  from it. State such comparisons as information the text supplies ("Spanish,
  another daughter of Latin, kept *día*…"), never as knowledge the reader is
  presumed to arrive with.
- **For non-Latin scripts, the reading course (below) is written for someone
  who cannot read a single letter** — not "a refresher you can skim." Someone
  who happens to read the script will move fast on their own; the text does
  not tell them to.

The curriculum's *methodology* (etymology-first, atom-first, gender-early,
grammar-from-zero) was shaped by one person's learning preferences — the
"Motivation" below records that origin — but the *output* serves the beginner
defined here. Where this spec below says "the learner" as design rationale,
read it as "why the method is shaped this way"; the reader of the books is
always the beginner above.

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

## Lesson Anatomy

A **lesson** is the atom of the curriculum: **one word or one fixed phrase**,
excavated fully, in a few minutes. Not "the ten greetings" — *hola* is a
lesson; *buenos días* is a lesson; *gracias* is a lesson. The learner
explicitly rejected the Spanish-101 model of drilling a list of ten
greetings at once. One thing, gone deep, is the unit of work — because that
is what fits a commute leg *and* what the brain can actually bind into its
existing web of knowledge in one sitting.

Lessons are written in audio-script style with bracketed delivery cues
(`[PAUSE Ns]`, `[REPEAT x2]`, `[YOU SAY: ...]`) so they can be read aloud by
the learner, a recorded voice, or eventually a TTS pipeline (future work;
this spec only guarantees the scripts are *shaped* for it). Every lesson
file follows this structure — sections are present when they apply, skipped
silently when they don't:

1. **Warm-up** — a one-line recall hook, usually tied to a prior lesson.
2. **You'll want to know first** *(optional)* — a short list of links to
   prior lessons whose concepts this one leans on. This is what turns the
   corpus into **reference material**: any lesson can be opened cold, and
   its prerequisites are one click away. (In the book these become
   `\hyperref` cross-references; in the Markdown units they're relative
   links.)
2. **Sounds you'll need** *(inline, only the sounds this word uses)* — the
   pronunciation facts required to say *this* word, and nothing more (e.g.
   for *hola*: "the *h* is silent"). There is **no gated pronunciation
   chapter** — see "Pronunciation: Inline, Never a Gate" below. Each note
   links to the track's `pronunciation-reference.md` for the learner who
   wants the fuller picture, but the lesson never *requires* leaving it.
3. **The word, taken apart** — the heart of the lesson. Each component of
   the word/phrase traced to its root, and then — critically — **the widest
   honest web of English cousins from that same root**, because the whole
   pedagogical bet is that the brain learns by attaching the new to the
   already-known (see Etymology & the Cousin Web below). Where the root
   builds words by prefix/suffix, that construction is shown explicitly and
   named (see Morphology In Context below). Nouns carry a grammatical-gender
   tag; etymology confidence is flagged where a root is disputed.
4. **Why it's said this way** *(the cultural/idiomatic core)* — the part
   Spanish 101 skips: *why* is it *buenos días* (plural)? *why* is the
   formal form the polite default? Where a phrase is idiomatic — a frozen
   cultural formula rather than something derivable from grammar — that is
   stated plainly, with its history. The learner wants the reason, not just
   the rule.
5. **Grammar Lens** *(optional — only when the lesson introduces a
   grammatical structure)* — the concept in plain terms with no assumed
   terminology, how English handles the same function, and what differs.
6. **Guided practice** — `[YOU SAY]` prompts that force retrieval, not
   recognition.
7. **Wrap-up recall** — one retrieval question seeding the next review.

## Lesson Types

- `word` — the default: one new word or fixed phrase, fully excavated per
  the anatomy above. The overwhelming majority of lessons.
- `review` — resurfaces earlier words in a **new combination**, never a
  verbatim repeat.
- `practice-mix` — recombines several recent lessons into short connected
  speech, closing out a session.

Grammar is not a separate lesson type: it is introduced **in context**, on
the first word that needs it (the Grammar Lens section), exactly like
sounds and morphology — never front-loaded. Lexical morphology (roots,
prefixes, suffixes) is likewise woven into `word` lessons rather than
split out, since the whole point is to meet a root *attached to a real word
you're learning*, not as an abstract table.

## Lesson File Schema

One file per lesson, YAML frontmatter + Markdown body. **IDs are stable
*slugs*, never ordinal numbers** — `<lang>-C<chapter>-<slug>` (e.g.
`ES-C01-dia`), matching the filename. This is deliberate: an earlier draft
numbered lessons (`ES-C01-L03`), and every insertion forced a painful
renumber cascade across filenames, ids, prerequisites, and cross-links.
Slugs are insert-proof — adding a lesson is just a new file plus one line in
the book and the session map; nothing renumbers.

**Order is not stored in the lesson files.** It lives where a tool already
counts it for us:

- the **book** `\input`s its chapter `.tex` files in order and each chapter's
  `\section`s run in order, so **LaTeX auto-numbers** every chapter and
  section (and `\ref`/`\S\ref` cross-references stay correct when a lesson is
  inserted — never hand-number in the book);
- `session-map.md` lists the pedagogical sequence by name.

Prose (in lessons and the book) refers to other lessons **by word/name**
(“the *bien* lesson”), never “Lesson 2”, so nothing goes stale on reorder.

```yaml
---
id: ES-C01-dia               # <lang-code>-C<chapter>-<slug> — a stable slug
chapter: 1                   # coarse grouping only (chapters rarely reorder)
type: word                   # word | review | practice-mix
headword: día                # the one word/phrase this lesson excavates
gloss: day (el día — masculine)
concept_tag: DIA
prerequisites: [ES-C01-el-la]  # slug ids this leans on (become cross-links)
sounds: [accent-i, d-soft]   # ids into pronunciation-reference.md
roots: [dies]                # Latin/other roots excavated, for indexing
est_minutes: 4
reviews_of: []               # lesson ids resurfaced (review/practice-mix only)
---
```

Body sections match the Lesson Anatomy above: `## Warm-up`,
`## You'll want to know first` (optional), `## Sounds you'll need`,
`## The word, taken apart`, `## Why it's said this way` (when idiomatic/
cultural), `## Grammar Lens` (optional), `## Guided Practice`,
`## Wrap-up Recall`.

The cousin web in "The word, taken apart" is the signature of this
curriculum. Format each component like:

```
**bueno** — good.
Root: Latin *bonus* ("good"). Its adverb-sibling *bene* ("well") — same
ancient root — is the piece hiding in English **bene**volent, **bene**fit,
**bene**diction, **bene**factor, and (worn down) *bonus*, *bonanza*,
*bounty*. So *bueno* is the Spanish half of a word whose English half you
already own in every "well-wishing" word you have.
```

Nouns carry an inline gender tag (`**la casa** (fem.)`). Etymology
confidence is flagged inline where a root is disputed (see the `hola`
lesson).

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

1. 2-4 `review` lessons currently due per the schedule above.
2. One or a few new `word` lessons — because each is now a single word/
   phrase, a session may introduce two or three of them (still one *thing*
   each), where the old coarse-grained units introduced one big cluster.
3. One `practice-mix` lesson recombining the new words with recent reviews.

A **bonus queue** of additional `review`/`practice-mix` lessons is always
appended after the core block, so a 1-2 hour drive never runs dry — the
learner can stop after the core block on a short leg, or keep going.

`session-map.md` (per language track) lays out which lessons compose which
session and verifies the interval schedule is actually satisfied.

## Etymology & the Cousin Web

This is the engine of the whole curriculum, and the reason it exists at all.
The learning bet is neuroscientific and plain: **the brain acquires by
attaching the new to the already-known.** The learner already carries an
enormous English vocabulary, much of it Latin-derived. So every Spanish word
is taught not as a fresh item to memorize but as a **cousin of words the
learner already owns** — and the lesson's job is to make that family
resemblance impossible to miss.

Concretely, every `word` lesson digs out **the widest honest web of English
cousins** from the headword's root — not one token cognate, but the whole
family:

- *gracias* ← Latin *gratia* → *grace, gratitude, gratuity, gratis,
  gratify, ingrate, congratulate,* and (through French *à gré*) *agree*.
- *quiero* ← Latin *quaerere* ("to seek") → *query, inquire, inquiry,
  quest, question, request, require, acquire, conquer, exquisite,
  perquisite* (→ *perk*).

The more live connections a lesson can honestly light up, the more places
the new word has to anchor. "Honestly" is load-bearing: a false cognate is
worse than none, so disputed or folk etymologies are flagged as such
(the `hola` lesson models this), and roots that merely *look* related but
aren't (e.g. *querer*'s *quaerere* vs. *quarrel*'s *queri*) are kept
separate.

### Show the derivation, not just the root

Naming the root is not enough. Where it's interesting — and it usually is —
a lesson shows **how the target word was actually formed from that root**:
the erosion, compression, or fusion that produced it. This is often the most
memorable part of a lesson, and the learner asked for it explicitly. Examples
the Spanish track already carries:

- **Erosion**: *quomodo → quomo → como* ("how"); the final *-do* drops, the
  middle collapses, *qu-* flattens to *k*. Show the intermediate steps.
- **Compression of a phrase**: *usted ← vuestra merced* ("your grace");
  *adiós ← a + Dios* ("to God"); *buenos días* as the fossil of *"buenos días
  os dé Dios."*
- **Regular sound-changes**, taught once and then reused as decoding tools:
  Latin *-ct- → -ch-* (*noctem → noche*), *-lt- → -ch-* (*multum → mucho*),
  *cl-/fl-/pl- → ll-* (*clamare → llamar*).

When a word belongs to a systematic family (e.g. the Latin *qu-*
interrogatives → Spanish *qué/quién/cuál/cuándo/cuánto/cómo*), the lesson
surfaces the whole pattern, so one derivation unlocks a set.

Spanish's second root stream, alongside Latin, is **Arabic** — roughly
4,000 words from ~800 years of Al-Andalus (*azul, ojalá, azúcar, aceite,
almohada*), often sharing an English cousin borrowed from the same Arabic
source (*azul* / *azure*). These surface as their own cousin-webs whenever a
lesson's headword happens to be Arabic-derived.

### Morphology In Context

When a root builds its family by **prefix and suffix**, the lesson shows
the construction explicitly and names the pieces — because seeing *how*
Latin assembles words is itself a skill that pays off across the learner's
whole English vocabulary. The *quaerere* family is the model:

- *in-* ("into") + *-quire* → **inquire** ("to seek into")
- *re-* ("again/repeatedly") + *-quire* → **require**
- *ac-* (*ad-*, "toward") + *-quire* → **acquire**
- *con-* ("fully") + *-quer* → **conquer** ("to seek fully → overcome")
- *ex-* ("out") + *-quisite* → **exquisite** ("sought out")

The learner asked for exactly this: not just "these words are related," but
"watch — put *in-* in front of *-quiry* and you get *inquiry*." Prefixes
and suffixes are taught the first time a word makes them visible, then
reused as recurring vocabulary in their own right.

Etymology confidence is marked inline where a root is disputed rather than
stated as settled fact.

## Grammatical Gender Methodology

English marks gender almost nowhere — only in a handful of pronouns (*he,
she, it*), and there it tracks real-world sex/animacy, not an arbitrary
grammatical category. Spanish (like French, German, and Hindi later) marks
gender on *every* noun, and that gender forces agreement across articles and
adjectives — a structural feature English speakers have no built-in instinct
for. This gets treated as a **standing methodology**, not a single chapter:

- **Gender is introduced on the very first noun taught, not in a later
  "gender chapter."** For Spanish that first noun is *día* (Chapter 1): the
  lesson introduces the concept of grammatical gender itself, traces it to
  Latin (see below), tags *día* as masculine (*el día*), and establishes the
  rule that *every* noun from then on arrives with its *el*/*la* — because
  gender can't be reliably guessed, it must be learned *with* the word,
  alongside its plural. Number (plural) and gender are taught together, as
  the two tags every Spanish noun carries.
- **Trace the gender system to Latin.** Grammatical gender isn't a Spanish
  invention: Latin sorted every noun into *three* genders (masculine,
  feminine, neuter); Spanish simplified to *two* (masculine, feminine),
  folding most old neuter nouns into the masculine. A given noun's gender is
  usually inherited straight from its Latin gender — which is exactly why
  *el día* is masculine despite its *-a* ending (Latin *dies* was masculine)
  and *la noche* is feminine (Latin *nox* was feminine). The Latin origin is
  presented as the *explanation* for the gender, not a footnote.
- Later, a consolidation lesson can make the surface pattern explicit — the
  *-o*/*-a* default and its real exceptions (*el día, la mano, el problema*,
  *el agua*) — but the learner has been tagging gender per-noun since the
  first noun, so that lesson formalizes an already-lived pattern rather than
  introducing gender cold.
- This generalizes: French and German have their own gender systems (German
  adds a third, neuter, and ties gender to case); Hindi marks gender on verbs
  as well as nouns; Tamil, Kannada, Telugu, and Malayalam have their own
  (different — rational/irrational noun-class systems in Tamil's case)
  classification schemes. Each future track gets the same treatment: tag
  early, formalize later, contrast explicitly with what English does (which
  is usually "nothing").

## Pronunciation & Script: Inline, Never a Gate

There is **no standalone "Sounds & Letters" chapter** in any track. This is
a deliberate reversal of the earlier design (a short Part 0), made on direct
learner feedback: *"Do not make the readers sit through a chapter worth of
sounds. Many will not even make it to the actual first lesson."* Front-loading
pronunciation (or, for new scripts, the alphabet) is exactly the wall that
makes people quit before Lesson 1.

Instead:

- Each lesson carries a **"Sounds you'll need"** note covering *only* the
  sounds its own headword uses — for *hola*, "the *h* is silent, the vowels
  are pure"; for *quiero*, "*qu* is a hard *k*, the *u* is silent." Just
  enough to say *this* word, delivered at the moment it's needed.
- A per-track **`pronunciation-reference.md`** collects the full sound and
  spelling system as **reference material** — the place to look things up,
  not a gate to pass through. Every inline "Sounds you'll need" note links
  into it (via `sounds:` ids in the frontmatter), so the curious learner
  can go deep on demand and everyone else keeps moving.
The payoff is cumulative: by the time a learner has done the first several
lessons, they've absorbed the sounds they've actually needed, each one welded
to a real word, rather than having memorized an abstract chart they can't yet
attach to anything.

### Non-Latin scripts: Chapter 1 IS a reading course

The rules above assume the learner can already *read* the script (true for
the Latin-alphabet tracks — Spanish, French, German). For any track whose
script the learner does **not** fluently read — **Arabic, Hindi, Tamil,
Kannada, Telugu, Malayalam** (i.e. everything not written in the Latin
alphabet) — **Chapter 1 is an incremental reading course**, and vocabulary
(greetings, etc.) begins in Chapter 2, once the letters exist to decode it.

This was a correction after the first Arabic draft dropped whole words
(*مرحبا*, *صباح الخير*) with only a gloss of the hard sounds — which teaches
a vocabulary list, not *reading*. The fix is the atom-first playbook applied
to the **script**: the atoms are **letters**, and words are built from letters
the learner has just learned, exactly as Spanish built *buenos días* from
*bueno* + *días*.

The reading course is **not** the "gated alphabet chart" this section warns
against. The difference:

- A gated chart teaches all ~28-50 letters in the abstract before any word.
- A reading course introduces **a few letters per lesson** and **immediately
  cashes them out in a real, decodable word** — so the learner *reads
  something true* in lesson 1 (e.g. Arabic ل + ا → **لا**, "no"), and every
  lesson after adds letters that unlock the next word. Letters change shape
  by position (Arabic) or combine into conjuncts (Indic scripts); those facts
  are taught as the words that use them arrive, cumulatively.

Sequence the letters so real words — and soon the greeting set — become
readable as fast as possible (Frequency-Driven Content Selection, below,
applied to letters). The reading course is authored for someone who cannot
read a single letter of the script (per Audience, above) — it never tells the
reader to skim. Once Chapter 1's reading course is done, later chapters
proceed word-first like the Latin tracks, with new letters still introduced
inline as rarer ones appear.

Grammar follows the identical principle — introduced in context, on the
first word that needs it (the Grammar Lens section), never front-loaded.
This matters most for a track like Tamil, where the learner is fluent and
literate but never studied the grammar formally: constructs come one at a
time, simplest-first, each on a real word, building up — not as a reference
grammar dumped up front.

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

## Grounding: English First, Then the Deep Root

Each track grounds every word against **the languages the learner already
knows** — nothing more. It does **not** forward-reference other curriculum
tracks that haven't been learned yet: on direct learner feedback, *"You
don't have to reference other languages that are coming in sequence. They
will arrive as they are."* A Spanish lesson connects to English (the
cousin web) and to Latin (the deep root), plus Arabic where a word is
Arabic-derived — and stops there. It does not say "and in French this
is…", because the learner isn't learning French yet.

Each track therefore has its own grounding set, always anchored on English
plus whatever deep-root system that language actually draws on:

- **Spanish** → English cousins + Latin roots (+ Arabic for Al-Andalus
  loanwords).
- **French, German** (when built) → English + Latin/Germanic roots.
- **Hindi** → English + Sanskrit roots (+ Persian/Arabic loanwords).
- **Tamil, Kannada, Malayalam, Telugu** → English + native Dravidian roots
  + Sanskrit for loanwords.

If a genuinely illuminating connection to another language the learner
already knows exists, it can be used — but as an aid to *this* word, never
as a teaser for a track not yet begun.

## Book Format

Each language track produces a **LaTeX book** — one publishable volume per
language, meant to eventually be released for free (CC BY-SA 4.0) — living
at `<track>/book/`. `\documentclass{book}`, compiled with **XeLaTeX**
(`fontspec` + `polyglossia`, not pdflatex) specifically because later tracks
need proper Unicode and right-to-left/complex-script typesetting that
pdflatex can't do. The book opens with a **pronunciation reference
appendix** (the typeset form of `pronunciation-reference.md`) and is then
`\input{}`-assembled from one file per chapter, growing incrementally — a
new chapter file plus one `\input` line as each chapter's lessons are
authored, never all at once. Prerequisite links between lessons become
`\hyperref` cross-references so the PDF reads as navigable reference
material.

The book and the practice lessons (`<track>/lessons/*.md`) are **two views
of the same content**, not independently maintained: the book is the
polished, continuous-reading edition; the lessons are the same material
sliced into few-minute, spaced-repetition-scheduled, car-consumable pieces.
Cousin webs, cultural notes, Grammar Lens sections, and gender tags all
appear in both.

## Roadmap Shape

Each language track has a `roadmap.md`: a chapter skeleton (the word/phrase
list per chapter — not fully scripted) building toward B1 ("normal
day-to-day conversation"). Because lessons are now one-word-each, a chapter
is a themed cluster of many small lessons rather than a single dense unit.
Chapters are authored incrementally, in the order: lessons (`lessons/*.md`)
first, then that chapter's LaTeX file folded into the book. The roadmap is
the standing plan, updated as chapters get authored. There is no "Part 0"
in the roadmap — pronunciation lives in the reference, not a chapter (see
"Pronunciation & Script: Inline, Never a Gate").

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
- **All non-Spanish tracks.** French, German, Arabic, Hindi, Tamil,
  Kannada, Malayalam, Telugu are planned but not yet built in this
  deep-lesson model. Spanish is the pilot proving the format; the others
  follow once it's solid, each grounded per "Grounding: English First, Then
  the Deep Root."
