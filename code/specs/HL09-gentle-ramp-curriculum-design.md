# HL09 — The gentle-ramp curriculum, designed from zero

**Status:** specification, 2026-08-07
**Supersedes in scope:** the implicit sizing behind HL04's shared spine and HL07's
spine expansion. It does not replace them; it says how big they have to be.
**Pilot track:** Spanish. Every rule here is language-neutral and meant to be
replicated across all 22 tracks.

---

## 1. Why this exists: the A2 claim was wrong

The gap report says Spanish "reaches A2". The project owner, who has sat A2
examinations, did not believe it. The owner was right, and the margin is not small.

Measured on 2026-08-07 against the committed corpus:

| | Spanish today | A2 actually requires | short by |
|---|---|---|---|
| distinct headwords taught | **178** | ~1,000–1,500 | **≈6.7×** |
| lessons realizing any A2 node | **14** | — | — |
| A2 spine nodes realized | **1 of 5** | 5 | 4 missing |
| verb tenses taught | present only | preterite, imperfect, perfect, near future, imperative, subjunctive exposure | 5+ missing |

The 14 lessons all realize a single node, `SPINE-SAY-WHAT-I-DO`, which declares
**42 concepts**. Three of the other four A2 nodes declare **exactly one concept
each** — `SPINE-TALK-ABOUT-PAST` is one concept, and it stands for the entire
past tense of the language. Spanish realizes none of them.

So "A2" rested on fourteen present-tense verb lessons pointing at a node holding
forty-two concepts. That is the failure mode this spec exists to prevent:

> **A level is not reached by declaring a node. It is reached by teaching the
> language the level requires, one small step at a time.**

The concept-count asymmetry is itself the bug. A node with 42 concepts and a node
with 1 concept cannot both be one rung of a gentle ramp.

---

## 2. The governing rule

**No lesson introduces more than three new atoms, and every atom is revisited.**

The first half already exists as `maxNewAtomsPerLesson` (HL08). The second half
does not exist anywhere, and its absence is measurable: **93 of Spanish's 182
taught atoms (51%) are never practised again** after the lesson that introduces
them. Median revisits per atom: **zero**.

A course that teaches a word once and never returns to it is not a gentle ramp.
It is a list.

### 2.1 There are four ramps, not one

Learning a language is not memorising a set of items. It is acquiring vocabulary,
learning to *build* with it, and being able to *put pieces together you were never
explicitly given*. Each of those is its own curve, and a course can be gentle on
one while being brutal on another.

| ramp | measures | status |
|---|---|---|
| **Vocabulary** | new atoms per lesson | `maxNewAtomsPerLesson` (HL08), now published |
| **Script** | new target-script glyphs per lesson | `maxNewGlyphsPerLesson` (HL-C18C), shipped |
| **Sentence** | how much structure an utterance carries | **missing — §5** |
| **Synthesis** | whether the learner *produces* rather than recalls | **missing — §6** |

The vocabulary ramp alone is what made Spanish look gentle while being unusable:
178 words, every one of them at ≤3 per lesson, and almost no growth in what the
learner can actually *say* with them.

---

## 3. The arithmetic, stated honestly

Length is explicitly not a cost (HL08), and the project owner has confirmed the
books may run to thousands of pages. That permission is what makes the following
numbers acceptable rather than alarming.

Taking a conservative **2 new atoms per lesson average** (the budget is 3; the
average must sit below it so the ramp has slack for revision lessons):

| level | cumulative vocabulary | new at this level | lessons at this level | cumulative |
|---|---|---|---|---|
| pre-A1 | ~300 | 300 | ~150 | ~150 |
| A1 | ~600 | 300 | ~150 | ~300 |
| A2 | ~1,200 | 600 | ~300 | ~600 |
| B1 | ~2,500 | 1,300 | ~650 | ~1,250 |
| B2 | ~4,000 | 1,500 | ~750 | ~2,000 |
| C1 | ~8,000 | 4,000 | ~2,000 | ~4,000 |
| C2 | ~16,000 | 8,000 | ~4,000 | ~8,000 |

**A complete Spanish track is on the order of 8,000 lessons.** It has 146.

Vocabulary counts are the conventional working figures for CEFR receptive
vocabulary size; they are editorial planning targets, not claims about any
awarding body's published syllabus, and are recorded as such (§10).

These numbers are a **budget, not a promise**. What this spec fixes is the shape
of the ramp; filling it is many tranches of work, replicated per track.

### 3.1 What this means for "reaching a level"

A track has **complete structural coverage** of a level only when it has:

1. realized **every** spine node at that level and all levels below,
2. taught at least the cumulative vocabulary for that level,
2b. **and enough of that vocabulary is verbs** — see the composition rule below,
3. zero lessons over the atom budget at or below that level, and
4. every atom at or below that level revisited at least twice (§7).

Anything less is reported as *in progress at level X*, never as *coverage complete
at X*.

#### These five criteria measure the corpus. None measures a learner.

**This is the most important sentence in the section, and until 2026-08-26 the
gate's own output contradicted it.** Read the list again and ask what each
criterion counts: spine nodes realized, headwords taught, verb headwords taught,
lessons over budget, atoms revisited. Every one is a property of the *book*.
Not one of them requires a person to answer a question.

The report nevertheless printed **`levels ATTAINED`**, and Spanish was about to
print *"1 track at A1"* having never had a single exam item scored against it.
"Attained" is what a candidate does; "covered" is what a syllabus does. The
gate was honest about which lessons exist and dishonest about what that proves,
in exactly the shape §1 opens with — a number that means TOUCHES being read as
MEANS, one rung higher up.

So the terms are now fixed and the report says both halves:

| term | what it asserts | measured by |
|---|---|---|
| **touches** *X* | some lesson sits at *X* | `TrackLevelCoverage.reach` |
| **coverage complete** at *X* | the corpus teaches what *X* asks for | criteria 1–4 above |
| **performance verified** at *X* | a learner scored a pass on a real task | **nothing — see §3.2** |

The third row is empty on purpose. It is not a criterion that was weakened; it
is a criterion that does not exist yet, and the report now says so in the line
under the coverage figure rather than letting the reader supply it. Removing or
softening criteria 1–4 to make the humbler wording true would be the same
mistake pointing the other way: the corpus claims are correct, and they are
claims about a corpus.

The wording is also load-bearing outside this repository. These books are meant
to be read by people deciding whether they are ready to sit DELE A1. A reader
who sees "attained A1" and books an exam has been told something the project
never measured.

#### Every count criterion carries a composition criterion

> Where this section asserts *how many*, it must also assert **of what**, for at
> least one partition of the counted set that a reader would care about. A
> criterion that can be satisfied by an arbitrary composition of the counted
> items is not measuring the thing it is named after.

This is not an abstraction looking for a customer. Criterion 2 counted headwords
and asked nothing about part of speech, and `HL23` measured what that permits:
Spanish reached **584 of the 600 headwords A1 asks for, of which seven were
verbs** — five distinct lexemes — while the same levels taught the complete
present paradigm of all three conjugations. The learner had the machinery and
almost nothing to run it on, and criterion 2 reported a track sixteen words from
certification. **A total can always be reached by the wrong parts.**

Criterion 2b is that rule applied to vocabulary. Its floors live in
`LEVEL_VERB_VOCABULARY` beside the totals they partition, and they are
**editorial in the strong sense of §10** — conventional vocabulary sizes are at
least conventional, whereas no awarding body publishes a verb quota and these
numbers are not derived from one. They are a project choice: 1.7% of the total
at pre-A1, 6.7% at A1, and 10% at every level from A2 up. A beginner level is
allowed to be noun-heavy; the share then converges on a tenth.

The measurement under it must **fail safe**, and this one does. A verb is
identified by the `concept_tag` the validator already requires, which `HL23` §6.1
measured at 96.2% recall corpus-wide; the residual 4% are verbs the tag misses,
so the count runs low and the criterion flags a track far more readily than it
certifies one. A composition check that erred the other way would be worse than
having none, because it would launder exactly the case it exists to catch.

**Criterion 4 has the same defect and is not fixed here.** Reinforcement counts
revisits and never asks *what* is revisited, so a track can satisfy it while
every unrevisited atom is the same kind — and Spanish's 83-atom shortfall is
currently un-partitioned. That is its own piece of work, tracked separately;
it is named here so the rule is visibly not a one-off written to justify a
single gate.

### 3.2 Proposed: a `mock-performance` criterion — **NOT ENABLED, BLOCKED**

*Status: **sketch only**. Not implemented, not wired, not gating anything.*

The empty row above is not meant to stay empty. The shape a fifth criterion
would take:

> **5. `mock-performance` — at least one full mock at this level has been sat
> and scored, and the score meets or exceeds the awarding body's own pass rule
> for that level.**

Its parts, and where each already has machinery:

- **Which mock.** `<track>/assessment.json` already names two timed mocks per
  level with a rubric and an answer key (HL16). Those artifacts must exist —
  they mostly do not, which is what the `assessment-artifact-ceiling` gate now
  records rather than tolerates.
- **What "pass" means.** The awarding body's real rule, transcribed by HL18 §4
  into `task-shapes/<level>.json` `passRule`, **not improved**. DELE A1 is not
  60% of a project-invented total; it is whatever Instituto Cervantes publishes,
  including the per-skill structure. Where the body publishes no independent
  per-skill threshold, HL16's stricter project thresholds apply *on top*, and
  the two remain separately visible (HL18 §4).
- **Who sat it.** A score with no scorer is a number, not evidence. The record
  must name the sitting: date, form, conditions (timed? single audio pass?),
  who marked it, and against which rubric revision. HL16's
  `requiresHumanValidation` is the existing hook.
- **What it may not do.** It may not be satisfied by a self-scored, untimed, or
  post-hoc-marked attempt, and it may not be satisfied by the *average* of two
  skills — the whole point of independent thresholds is that a strong reading
  score cannot hide an untaught speaking strand.

**Why this is a sketch and not a change.** The A1 mocks are being written and
sat right now, in a parallel effort. What a criterion should demand is exactly
what that sitting is about to reveal — how long a real mock takes to author,
whether a rubric written from an inventory survives contact with a real answer,
whether a per-skill threshold is measurable from the artifacts HL16 asks for.
Specifying the criterion before that result would be guessing at numbers and
then defending them, which is how §3's editorial figures got their §10 warning
label.

**Blocked on:** the first scored Spanish A1 mock. Until it lands, the honest
state of this project is *coverage complete at pre-A1, performance unmeasured
everywhere*, and that is what the report prints.

---

## 4. The lesson contract

Every lesson is one small step. Concretely:

- **≤3 new atoms**, and the running average per chapter must sit at or below 2.
- **Useful immediately.** A lesson teaches something the learner can say, ask,
  recognise, or answer by the end of it. No lesson exists only to set up a later
  one; scaffolding rides inside a lesson that also pays off.
- **Revisits at least one earlier atom**, explicitly, in `practises.knowledge`.
  This is the mechanism §7 measures. A lesson that revisits nothing may only be
  the first lesson of a track.
- **Declares its order.** `sequence:` is mandatory. Today **56 of Spanish's 146
  lessons have none**, so their reading order exists only inside hand-typed
  LaTeX — and a ramp whose order is unknown cannot be verified at all. French is
  worse: 64 of 73.
- **Names its level implicitly**, never in frontmatter. Level stays derived from
  the spine (HL-C10); 8,000 authored copies of a computed fact are 8,000 places
  for it to go stale.

### 4.1 No forward references — you may only use what you have taught

**A lesson's examples, drills and payoffs may contain only material already taught.**

This is the rule the corpus breaks most often, and it is invisible to every ramp
budget because the offending vocabulary is not *introduced* — it is merely *used*.
A close read of Spanish chapters 1–8 found:

- `ES-C07-beber` rewards the learner with *"Como pan y bebo agua"*. **`pan` and
  `agua` are taught in Chapter 26.**
- `ES-C07-practice` drills *"Como una tapa y bebo un café. ¿Algo más?"* — `un`,
  `una`, `tapa` and `¿Algo más?` are all untaught; `un/una` arrives in Chapter 8,
  the **next** chapter.
- `ES-C08-practice` drills *veintiuno* and *diecinueve* in a chapter that taught
  1–10. **Both are Chapter 31.**
- `ES-C08-tener` generalizes "all forms but *nosotros/vosotros*" — `vosotros`
  appears exactly once in eight chapters, unexplained, never taught.

The mechanism is diagnosed in §7.3: chapters starved of reviewable material reach
sideways for whatever they need. A forward-reference check is therefore both a
defect gate **and** an early warning that reinforcement has failed upstream.

Two functional gaps of the same kind, found by reading rather than counting:

- **The learner cannot say "no".** They are drilled on *¿Hablas español?* in
  Chapter 6 and questioned throughout Chapter 7, but `sí`/`no` are Chapter 19.
  `SPINE-NEGATE-AND-ASK` has `"segments": []` — negation is realized nowhere.
- **The learner cannot say "I am".** `estoy` is used as a given in two Chapter 4
  lessons and taught in none; `SPINE-EXCHANGE-NAMES` explicitly omits `PRONOUN-I`
  and `WORD-IS`, so `yo` is absent from the whole path.

A course where the learner can ask "how are you?" but cannot answer, and can be
asked a question but cannot decline, has a hole no vocabulary count would show.

### 4.2 Etymology is per lesson

**Every lesson carries its own etymological connection.** The owner's rule, and
the corpus already agrees with it: 708 lessons carry an `etymology_hook` and 788
carry a `## The word, taken apart` section. HL00 calls that section "the heart of
the lesson… the signature of this curriculum," and the chapter-1–8 review singled
it out as the strongest thing in the course — `hasta` and eight centuries of
al-Andalus, `de nada` ← *rēs nāta*, `trabajar` ← *tripālium* and its road to
English *travel*, `adiós` traced across four Romance languages.

So this is not new work; it is a rule protecting something that already exists and
must not be diluted as the corpus grows to thousands of lessons.

Two constraints follow:

- **The etymology must land on a word the lesson actually teaches.** A hook
  attached to a word introduced three lessons later is a forward reference (§4.1).
- **Where a word has no honest etymology, say so briefly and move on.** `ES-C01-hola`
  spends its entire "taken apart" section explaining that *hola*'s origin is
  unsettled, that the resemblance to English *hello* is a false friend, and that
  the learner's instinct is wrong. That is true, and it is a poor opening: the
  course's signature move is not demonstrated until lesson 2. A word whose
  etymology is a dead end should not be the first word taught.

Chapters additionally carry a **thread** — the connections between their lessons'
etymologies, so the reader sees a family rather than a list. That is a bonus on
top of the per-lesson rule, never a substitute for it.

This is also why a 42-concept node is wrong twice over: you cannot give 42
concepts their own etymological treatment inside one unit of study, and the owner
named exactly this — *"we cannot do that if you put in 50 concepts in a single
lesson."*

### 4.3 A note on the word "chapter"

The project owner thinks of **one chapter as one lesson** — a single sitting, one
small step. The repository's data model uses `chapter` for a *group* of lessons
(Spanish chapter 4 holds fifteen).

Throughout this spec, **"lesson" means the unit of study** — the ≤5-minute,
≤3-atom step the owner calls a chapter — and **"chapter" means the repository's
grouping**. Where a rule says "every lesson", it means every unit the learner
sits down to, which is the thing the owner is asking about.

---

## 5. The sentence ramp

Vocabulary without structure is a phrasebook. The learner must move from single
words to complex utterances, and that movement is a separate curve that must ramp
just as gently — **one new structural move at a time**.

### 5.1 The rungs

Each rung is a **sentence frame** the learner can fill with any vocabulary they
already hold. A track declares which rung each lesson's target utterances sit at.

| rung | frame | Spanish example |
|---|---|---|
| S0 | bare word / formula | *hola* · *gracias* |
| S1 | two-part formula | *buenos días* · *¿qué tal?* |
| S2 | subject + verb | *yo como* |
| S3 | + object | *como pan* |
| S4 | + modifier | *como pan blanco* |
| S5 | + prepositional phrase | *como pan en casa* |
| S6 | + time or manner | *como pan en casa por la mañana* |
| S7 | two clauses coordinated | *como pan y bebo café* |
| S8 | subordination (cause, time) | *como pan porque tengo hambre* |
| S9 | relative clause | *el pan que como es fresco* |
| S10 | reported speech | *dice que come pan* |
| S11 | irrealis — conditional, subjunctive | *si tuviera pan, comería* |
| S12 | multi-clause with discourse marking | *aunque no tenía hambre, comí, ya que…* |

The rungs are cumulative and language-neutral in *shape*; the realization differs
per language, and a track's ledger says how it realizes each rung. Rung order is
not negotiable — S8 cannot precede S3 — but a track may take several chapters over
one rung, and should.

### 5.2 The budget

- **One new rung per chapter, at most.** A chapter may consolidate a rung it has
  already reached as often as it likes.
- **A lesson's target utterances sit at the current rung or below.** A lesson may
  not silently demonstrate S8 while claiming to teach S3 — this is the single most
  common way a "gentle" course stops being gentle, because the example sentences
  race ahead of the syllabus.
- **A new rung is introduced with known vocabulary only.** Never a new structure
  and new words in the same breath: the learner must have one of the two free.

That last rule is the sentence-ramp analogue of the cousin-layer rule in HL-C18C —
the learner should only ever be decoding one unfamiliar thing at a time.

### 5.3 Why this is measurable

The rung of an utterance is largely recoverable from the utterance itself — clause
count, presence of a subordinator, verb count. The measurement does not need to be
a parser; it needs to catch the case where a lesson at rung S3 shows a sentence
with two finite verbs and a *porque*. Report-only first, per the HL05 precedent.

---

## 6. Synthesis: producing, not recalling

The owner's word is *synthesizing*, and it is the difference between a learner who
recognises a course's sentences and one who can speak.

**Every chapter must contain at least one synthesis activity**: a prompt whose
correct answer is an utterance the course has **never shown**, built only from
atoms and rungs the learner already holds.

- Not "repeat after me" — that is rung practice.
- Not "translate this sentence you saw two lessons ago" — that is recall.
- **Yes**: "You are hungry and it is morning. Say what you are going to eat." The
  learner assembles *por la mañana* + *voy a comer* + a food word, none of which
  were ever combined for them.

The repo already has the mechanism: `guided-production` blocks and the
`hl-activity` contract, which scores a spoken answer against a compiled activity
rather than against a cue. What is missing is the **requirement** that every
chapter carry one, and the measurement of which chapters do.

### 6.1 Synthesis is what makes the ramp feel gentle

A learner who has only ever recalled hits a wall the first time they must produce.
A learner who has produced from chapter one experiences each new rung as a small
extension of something they already do. The gentleness is not only in the step
size; it is in never deferring the hard skill to later.

---

## 7. Reinforcement: the spaced-retrieval schedule

New in this spec, and the answer to the 51% orphan rate.

Every atom must be **revisited at expanding intervals** after the lesson that
introduces it. Writing *n* for the position of the introducing lesson in track
reading order, the atom must reappear in `practises.knowledge` of at least one
lesson in each of these windows:

| window | range | purpose |
|---|---|---|
| R1 | n+1 … n+3 | consolidate before it decays |
| R2 | n+5 … n+15 | first real retrieval |
| R3 | n+20 … n+60 | durable |
| R4 | n+80 … n+250 | recognition at distance |

An atom that misses a window is an **orphan at that depth** and is reported.
Missing R1 is a defect; missing R4 in a track that is only 200 lessons long is
not — the schedule is evaluated only where the track is long enough to contain
the window.

**Chapter payoff lessons and `practice-mix` lessons are the natural carriers.**
This schedule is what those lesson types are *for*, and it gives a generator a
concrete job: given the atom inventory and the introduction positions, emit the
revision lessons that close every open window.

### 7.1 The machinery was specified and never built

This is not a case of a schedule being applied badly. HL00 already defines the
interval (N+1, N+3, N+7, N+15), already defines `type: review` as a lesson that
"resurfaces earlier words in a new combination", and already designates
`session-map.md` as the artifact that verifies the schedule is satisfied.

In the shipped corpus:

- **There are zero `type: review` lessons.** Not few — none, in any of 22 tracks.
- **`session-map.md` covers Spanish chapters 1, 2 and 3.** The schedule is
  unverified for chapters 4 through 33.
- The only re-emphasis vehicle that exists is the chapter-terminal `practice-mix`,
  and every one is scoped to its own chapter.

So the course reads as **eight self-contained mini-courses**. Chapter 2's four
greetings never reappear. Chapter 5's three farewells never reappear — and when
Chapter 7 finally builds a café scene where *hasta luego* belongs, it reaches for
*adiós*, which Chapter 5 taught for a different situation.

### 7.2 Measured: a chapter-end review cannot close R1

*Added 2026-08-07, before any review lesson was authored, because it changes what
building the scheduler means.*

R1 is **n+1 … n+3**. A review lesson placed at the END of a chapter is therefore
out of range for everything the chapter taught more than three lessons earlier.
Measured on Spanish: **11 of 35 chapters hold more than four lessons** (median 4,
max 15), so a terminal review cannot reach their opening material at all. The
`practice-mix` lessons the corpus already has are exactly this shape, which is
part of why they never closed a window.

The second measurement settles what kind of problem this is. Of Spanish's 114
R1 misses, **99 are never revisited at ANY distance** — only 15 are merely late.
So this is not a scheduling problem to be fixed by moving reviews around. It is an
absence.

Two ways to close R1, and they are not equivalent:

- **(a) Interleave dedicated `review` lessons every ~3 lessons.** Honest, but it
  implies roughly **one review lesson per three teaching lessons** — about 50 new
  lessons for Spanish alone, and ~2,600 at the 8,000-lesson target. It also
  segregates retrieval into its own lesson type, which is the weaker form.
- **(b) Make every teaching lesson practise the preceding one to three lessons'
  atoms**, via `practises.knowledge`. This is what §4's lesson contract already
  requires — "revisits at least one earlier atom, explicitly" — and it costs no new
  lessons. Retrieval is **interleaved with new material** rather than quarantined,
  which is the stronger form.

**(b) is the rule; (a) is the exception.** Dedicated `review` lessons remain the
right tool for the wider windows — R3 and R4 pull from far enough back that no
single teaching lesson can carry them — and for atoms whose chapter has ended. But
R1 and R2 must be closed by the teaching lessons themselves, or the corpus doubles
in size to say what a `practises.knowledge` line already says.

This also explains why `reviews_of` looked like reinforcement and was not: it was
authored per-lesson, at exactly the cadence (b) needs, but pointed at lesson ids
instead of atoms, so it closed nothing.

### 7.3 The failure is causal, not cosmetic

The reviewer's summary is worth keeping verbatim as the design principle:

> **A gentle ramp is not made of small steps; it's made of steps you can still
> stand on.**

Every other defect in this spec is downstream of the missing scheduler. With no
mechanism forcing earlier atoms forward, later chapters have nothing to build with
and reach sideways for whatever they need — which is exactly why Chapter 7 drills
Chapter 26's vocabulary (§4.1). Fix the scheduler and the forward references lose
their cause.

### 7.4 Why this is measured, not trusted

`reviews_of` exists on 144 of Spanish's 146 lessons, so the corpus *looks* like it
reinforces. But `reviews_of` names **lesson ids** while atoms live in a different
namespace, so it cannot close a window and never did. The measurement must run on
`practises.knowledge` — atom to atom — or it measures nothing.

---

## 8. The chapter contract

A chapter is the unit the reader experiences. Every chapter carries:

1. **An intro that stands alone in English.** A few sentences saying what the
   reader will be able to do by the end. Not "you learnt this in Hindi" — 288 of
   393 chapters currently have no intro at all, and six of those that do make
   cross-track references that dangle in a single-language PDF.
2. **An etymological thread** tying its lessons' per-lesson etymologies together
   (§4.2) — a bonus on top of the per-lesson rule, never a substitute for it.
3. **A culture or region note.** Not decoration: for Spanish, `vos` appears
   **0 times in 146 lessons**, in a language where voseo is the everyday second
   person for roughly 100 million speakers. A course that never mentions it is
   teaching a Spanish that does not exist anywhere in the Southern Cone.
4. **A payoff lesson** built only from material already taught (HL05).
5. **≤12 new atoms total** (`maxNewAtomsPerChapter`), so splitting a steep lesson
   into two steep lessons cannot game the rule.

### 8.1 Regional variation is a strand, not an appendix

Spanish is the pilot precisely because it forces the question. The rule:

- The **default taught form is marked**, never unmarked. A lesson says which
  Spanish it is teaching.
- Where usage splits (tú / usted / vos; *vosotros* / *ustedes*; *coche* / *carro*;
  seseo), the split is taught **at the point the form is introduced**, not
  deferred to a back-matter note.
- A regional form the course does not teach actively is still taught
  **receptively** — the learner must recognise `¿vos tenés?` even if they produce
  `¿tú tienes?`.

---

## 9. Vocabulary selection

Which 300 words come first is a design decision, and it must be reproducible
rather than intuitive.

**Selection rule, in priority order:**

1. **Function first.** The word is required by a spine node the learner is on.
2. **Frequency second.** Among candidates, prefer higher corpus frequency. Rank
   is taken from a published frequency ordering for the language; the project
   stores **the rank it used and the source**, never a copy of the list.
3. **Cognate leverage third.** Among near-equal candidates, prefer the one whose
   etymology connects to something the reader already has — the project's
   signature move, and the reason `concept_tag` exists.
4. **Concreteness fourth.** Prefer words that can be pictured or acted.

Rule 2 is the one that stops a curriculum drifting into charming but useless
vocabulary. Rule 3 is what makes the ramp feel gentle even when it is not: a
reader meeting *comprender* for the first time is not meeting it cold.

---

## 10. Honesty requirements

Carried over from `core/exam-levels.json`, which already records whether a level
mapping is `published`, `research`, or `editorial`:

- Vocabulary size targets (§3) are **editorial** planning figures.
- CEFR can-do statements written for spine nodes are **this project's own
  wording**, informed by the level definitions. CEFR descriptor text is not
  reproduced.
- A track's claimed level is **derived and gated** (§3.1), never authored.
- Where a language has no CEFR examination, the ladder is editorial and says so —
  as `exam-levels.json` already does for the twelve tracks mapped straight to CEFR.

---

## 11. What ships in what order

This spec is the design. Implementation is a long series of small tranches:

| # | Work | Signal |
|---|---|---|
| 1 | Measure order integrity, reinforcement windows, and forward references; publish all three in the gap report | Every track reports orphan atoms per window, lessons lacking `sequence`, and every use of untaught vocabulary |
| 2 | Migrate Spanish chapters 7–8 to schema v2 and give all 56 sequence-less lessons a declared order | The ramp collapses exactly at the schema boundary: chapters 1–6 carry `sequence`/`spine_node`/atoms, chapters 7–8 carry none, so they are invisible to every tool that reads the knowledge graph. Chapter 7's order needs a **human decision** — `curriculum.json` says comer→beber→qué→vivir→dónde while the lesson prose and `reviews_of` say comer→vivir→beber→qué→dónde |
| 3 | Close R1/R2 by wiring `practises.knowledge` on existing teaching lessons (§7.2), then add `review` lessons only for R3/R4 | 99 of Spanish's 114 R1 misses are never revisited at ANY distance, so this is an absence, not a scheduling error. A chapter-end review cannot reach R1 for 11 of 35 chapters. Extend `session-map.md` past chapter 3 as the schedule becomes real |
| 4 | Close the functional holes: `sí`/`no`, `estoy`, `yo`, `un`/`una` | The learner can be asked a question and cannot decline; can ask "how are you?" and cannot answer |
| 5 | Split `SPINE-SAY-WHAT-I-DO` (42 concepts at diagnosis, **35 today**) into rungs of ≤6 | No spine node holds more concepts than a chapter may introduce. **Two slices shipped.** `HL23` §8.3: three concepts to a new A1 `SPINE-NAME-EVERYDAY-ACTIONS`, 42 → 39. `HL23` §9.2: `VERB-DO-MAKE`, `VERB-BUY`, `VERB-OPEN` and `VERB-CLOSE` to the same node, 39 → **35**, driven by the DELE A1 sitting that found those verbs taught and staged above the exam asking for them. The remainder is a cross-track **lesson migration**, not a ledger edit — `HL23` §8.1 explains why, §8.2 prices it per concept and §9.1 prices the six concepts §8.2 omits |
| 6 | Close Spanish's R1 windows | Zero atoms miss the first revisit window |
| 7 | Author pre-A1 to its real size (~150 lessons) | Vocabulary ≥300; every pre-A1 node realized; culture/region strand present in every chapter |
| 8 | A1, then A2, at their real sizes | §3.1 satisfied per level before the next begins |
| 9 | Spine B1…C2, then climb | — |

Steps 1–3 are the unblockers: step 1 makes every later step checkable, step 2
ends the schema split that causes the collapse, and step 3 builds the scheduler
whose absence causes everything else (§7.2). **Nothing above step 7 may claim a
level.**

---

## 12. Acceptance criteria for this spec

- [ ] Reinforcement windows (§5) are measured and published per track.
- [ ] `sequence` is required on every lesson, enforced.
- [ ] No spine node declares more concepts than `maxNewAtomsPerChapter`.
- [ ] Level claims are gated on §3.1 rather than on node realization alone.
- [ ] Every chapter has an intro, an etymological thread, and a culture/region note.
- [ ] The gap report distinguishes *in progress at level X* from *reached X*.
- [x] The gap report distinguishes **structural coverage** from **learner
      performance**, and states in its own output that §3.1 measures the corpus
      (§3.1). Enforced by `level-gate.test.ts`, which asserts the word
      "ATTAINED" is absent from the rendered report — a positive assertion alone
      would be satisfied by a rename that left the old phrasing elsewhere.
- [ ] `mock-performance` (§3.2) is specified against a real scored mock. Blocked
      on the first Spanish A1 sitting; deliberately not implemented.
