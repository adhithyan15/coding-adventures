# Changelog — Russian track

## 0.10.0 — 2026-08-08

Pre-A1 vocabulary tranche (part of the cross-track vocabulary program that has
now confirmed the same mechanism eight times running). Fourteen everyday nouns
across five new chapters (6–10) — Russian's first nouns of any kind — moving
pre-A1 vocabulary from **11 to 25** distinct headwords (shortfall 289 → 275)
and track-wide vocabulary from **25 to 39**. `SPINE-CHECK-WELLBEING`, unrealized
until now, is realized for the first time; only `SPINE-TAKE-LEAVE` remains
unrealized among the seven pre-A1 spine nodes.

- **Ch. 6 — Water, Coffee, Tea, and Bread** (`SPINE-POLITE-REQUEST-REPAIR`):
  вода, кофе, чай, хлеб. Opens the noun half of the book: no articles at all,
  and gender predictable from the ending (consonant → masculine, -а/-я →
  feminine, -о/-е → neuter) — stated as a rule, then immediately broken by
  *кофе*, masculine despite its -е ending, a fossil of the vanished form
  *кофий*. *чай* (overland, Mandarin *chá* via Persian/Turkic) and *кофе*
  (by sea, Arabic *qahwa* via Turkish and Dutch) took opposite routes to the
  same drink family; *хлеб* looks completely native and is a prehistoric
  Germanic loan, cousin of English *loaf*. Also introduces Russian's first
  polite-request pattern, "*[word], пожалуйста*."
- **Ch. 7 — Friend and Siblings** (`SPINE-EXCHANGE-NAMES`): друг, подруга,
  брат, сестра — exactly who Chapter 2's *ты* was for. *подруга* is built
  live from *по- + друг + -а*; *брат* (the noun) is one soft sign from
  Chapter 5's verb *брать*, the minimal pair that lesson had already warned
  about; *брат*/*сестра* are secure PIE cognates of *brother*/*sister*.
- **Ch. 8 — Family** (`SPINE-EXCHANGE-NAMES`, one lesson): семья gathers
  Chapter 7's four people and turns out to be a distant cousin of English
  *home*, *hamlet*, and every English place name ending in *-ham* — PIE
  *\*ḱey-*, "to lie down, settle."
- **Ch. 9 — Eyes, Ears, Mouth, and Nose** (`SPINE-CHECK-WELLBEING`, first
  realization of this node for Russian): ухо, нос, рот, глаз. *ухо* and *нос*
  are straight PIE inheritances (English *ear*, *nose*); *рот* has no secure
  English cousin and says so; *глаз* is the chapter's real story — Old
  Russian slang for "round stone" that evicted the true inherited eye-word,
  *око* (the actual cousin of English *eye*), now surviving only inside
  *очки*, "glasses" ("little eyes").
- **Ch. 10 — Heart** (`SPINE-CHECK-WELLBEING`, one lesson): сердце, the surest
  cognate in the book — PIE *\*ḱerd-*, with English *heart*, Latin
  *cor/cordis* (→ *cordial*, *courage*, *record*), and Greek *kardía*
  (→ *cardiac*) — and the new letter **ц**.
- **Three new letters, each taught where first needed**: **ф** (*кофе*),
  **х** (*хлеб*), **ц** (*сердце*) — false friends and new shapes alike
  flagged the same way Chapter 1 flagged в/р/с/н.
- **Reinforcement discipline matched the program's best.** Every lesson's
  `practises.knowledge` names atoms from the 1–3 lessons before it, and every
  chapter payoff reaches back further: Chapter 6 rescues Chapter 5's
  previously-unrevisited *любить* atoms; Chapter 7 rescues three Chapter 2
  atoms (the ты/вы comparison, vy-politeness pragmatics, and ty/thou
  etymology) untouched since they were introduced; Chapter 8 rescues Chapter
  3's *жить* etymon; Chapter 9 rescues Chapter 3's *видеть* and *говорить*
  atoms. Track-wide never-revisited-at-any-distance count: **0** (was 0
  before this branch and stayed there through 14 new lessons and 33 new
  atoms).
- **Two font traps caught before commit**: the PIE syllabic-r diacritic in
  *wódr̥* (the unmapped combining ring below, U+0325) and the CJK character
  茶 in *чай*'s etymology — both silently broke a forced XeLaTeX compile with
  "Missing character" until flattened to plain Latin transcription.

Verification: forced XeLaTeX build of the 71-page book has zero missing
characters and zero duplicate labels. `npx vitest run tests/integration.test.ts
tests/cli.test.ts` passes (19/19) — `integration.test.ts`'s pinned Russian
book-chapter list is updated from `[1..5]` to `[1..10]`, the one deliberate
edit to a shared test file. `check:modality`, `check:books` and
`check:narration` all pass. The six corpus-wide pinned-number tests
(chapters, continuity, levels, modality-manifest, narration, ramp) shift with
any authored content and are left failing per standing instruction.

## 0.9.0 — 2026-08-07

HL-C44. Russian sat at **6 of the 40 core verbs** and was still one of the
smallest tracks in the corpus. This adds the tranche of eight that Spanish,
Latin and Portuguese already teach, in **two chapters of four** rather than one
of eight, and takes Russian to **14 of 40** — second only to Latin.

- **New lessons**, in order: `RU-C04-dumat` (думать, `VERB-THINK`),
  `RU-C04-ponimat` (понимать, `VERB-UNDERSTAND`), `RU-C04-chitat` (читать,
  `VERB-READ`), `RU-C04-pisat` (писать, `VERB-WRITE`), `RU-C05-brat` (брать,
  `VERB-TAKE`), `RU-C05-sprashivat` (спрашивать, `VERB-ASK`), `RU-C05-pomogat`
  (помогать, `VERB-HELP`), `RU-C05-lyubit` (любить, `VERB-LIKE-LOVE`).
  Sequences 200–270; all schema v2; effective durations 279–291 s against the
  300 s ceiling; all eight `voice`-cored, so both chapters are drivable.
- **Each of the eight widens a three-way join to four-way.** Before this branch
  these concepts were realized by Spanish, Latin and Portuguese and nobody else.
- **Aspect is introduced, named, and deliberately not finished.** All eight
  verbs are imperfective, and one atom — `RU-GRAMMAR-ASPECT-PARTNER` — carries
  the fact that every Russian verb travels with a perfective partner. It is
  introduced once, on *думать · подумать*, and then practised in all seven
  lessons that follow, each naming its own partner in a paragraph:
  *понимать · понять* (reshaped stem), *читать · прочитать* (the clean case),
  *писать · написать*, *брать · взять* (**suppletive** — a different word
  entirely, the trick that also gives *идти* its past *шёл*),
  *спрашивать · спросить*, *помогать · помочь* (an infinitive in **-чь**), and
  *любить · полюбить* (which means *to come to love*, not *to finish loving* —
  a pair is not always doing-against-done). What a pair does to a whole sentence
  is left to a chapter of its own.
- **The писать stress trap is flagged plainly**, because learners hit it
  constantly: *pisát'* is **to write**, *písat'* is a child's word for
  urinating, and the two differ only in where the voice lifts.
- **One new letter, taught where it is finally needed**: **ш**, inside *пишу*.
  The track had been reading it since *живёшь*, *знаешь* and *говоришь* without
  ever naming it. It sits in a `## The letters in this word` section, which is a
  detachable `script` block, so `coreModality` stays `voice`.
- **Etymology carried both chapters, and said so when the trail stops.**
  *писать* is \**peyḱ-* "to cut, scratch" → Latin *pingere* → **paint,
  picture, pigment**; *брать* is \**bʰer-* → **bear, birth, burden**, Latin
  *ferre*, Greek *phérein* in **metaphor**; *спрашивать* is \**preḱ-* → Latin
  *precārī* → **pray, precarious**, and German *fragen*; *помогать* is
  \**magʰ-* → **may, might, dismay**; *любить* is \**leubʰ-*, which is English
  **love** itself. Where a cousin is **not** secure the lesson says so: *читать*
  is given **no** English descendant (and *cheat* and *chit* are named as the
  false leads they are), and *думать*'s link to **doom** is marked as an early
  Gothic **borrowing**, not an inheritance, with the minority inherited-root
  account noted. *брать* is flagged against *брат* "brother", a different root
  that merely starts the same way — as *быть* is a third.
- **Reinforcement was the point, and it is measured.** Every lesson's
  `practises.knowledge` names atoms from the one to three lessons immediately
  before it, which is what closes R1; the two payoffs reach back several
  chapters. Russian's never-revisited count goes from **21 of 34 atoms (62%) to
  3 of 55 (5%)** — and all three survivors belong to the last lesson, which has
  nothing after it. Rescued at distance: *я*-is-not-*R* and *я* ↔ *ego* from
  Chapter 2, *ты* ↔ *thou*, the polite-pronoun and naming-question comparisons,
  **быть** and the zero copula, **не**, *жить* ↔ *quick*, *говорить* ↔ not
  *govern*, the **г** of gamma, the **д → ж** swap, *идти → шёл*, and the
  one-way/habitual motion pair.
- **Stress notation changed, track-wide.** The vendored
  `NotoSansCyrillic-Static.ttf` is a Basic-Latin + Cyrillic subset with **no
  combining diacritics**, so `U+0301` printed as a missing character 198 times
  the first time a Russian verb chapter reached XeLaTeX. Stress now rides on the
  romanization (*chitát'*, *pishú*, *lyublyú*), which is what the book's own
  preface already promised the reader; the three Chapter-3 lessons that used
  Cyrillic acutes were adjusted with no loss, since each already carried the
  same stress in its romanization.
- **The book grew from two chapters to five.** Chapters 3, 4 and 5 are the
  track's first **generated** LaTeX chapters — chapter 3 is included because
  chapters 4-5 build directly on *знать*, *быть* and *говорить*, and printing
  them without it would put a forward reference into the standalone PDF. The
  preamble gains the four environments the generator emits, kept separate from
  the four the hand-written chapters 1-2 use. `book.pdf` compiles under XeLaTeX
  at 45 pages with **zero** `Missing character` and **zero** overfull boxes.
- **HL05 ledgers** for chapters 4 and 5, each with a payoff assessing **every
  atom its own chapter introduced** (11/11 and 10/10, against a 0.50 floor).
  Chapter 3 is still without one, and now shows as a book chapter lacking an
  HL05 capability: with no consolidation lesson its only candidate payoff
  reaches 0.26. Recorded, not papered over.

## 0.8.0 — 2026-08-06

HL-C42. Russian taught **zero verbs**. It was the smallest real track in the
corpus — 22 lessons across two chapters — and every one of them was a greeting,
a courtesy word, a pronoun or a naming phrase. A learner could introduce
themselves and then say nothing about what they do. This adds Chapter 3: six
core verbs, one per lesson, in a prerequisite chain.

- **New lessons**, in order: `RU-C03-byt` (быть, `VERB-BE`), `RU-C03-zhit`
  (жить, `VERB-LIVE`), `RU-C03-znat` (знать, `VERB-KNOW`), `RU-C03-govorit`
  (говорить, `VERB-SPEAK`), `RU-C03-videt` (видеть, `VERB-SEE`), `RU-C03-idti`
  (идти, `VERB-GO`). Sequences 140–190; all schema v2.
- **First realization of the shared `VERB-*` concepts by any track.** Before
  this, all 85 verb concept tags in the corpus were namespaced (`FR-VERB-ALLER`,
  `HI-VERB-BOLNA`) and therefore joined nothing across languages; the canonical
  core-verb list existed with no realization at all. Russian now covers 6 of the
  core 40 — `tracksWithNoCoreVerb` 22 → 21, `universallyMissing` 40 → 34.
- **The chapter is built around the thing that surprises English speakers most**:
  the present-tense copula is simply **omitted**. *Я студент* is "I student",
  with no word in between. The lesson makes that precise rather than glib —
  the verb returns in the past (*я был студентом*), and *есть* survives as "there
  is", already met in Chapter 1 inside *нет* = *не* + *есть*. It also puts the
  fact in a frame worth keeping: English fuses three PIE roots into *be / is /
  was*, and Russian keeps two of them apart as **быть** (\**bʰuH-*, English
  **be**) and **есть** (\**h₁es-*, English **is**).
- **One grammatical idea per lesson**, each on a word that needs it: the **-у**
  that alone means "I" (*живу*); **не** doing the entire job of English *don't*
  with no helper verb (*я не знаю*); the **-ешь / -ишь** families, which is why
  you learn the *you* form with the infinitive; the **д → ж** swap that hits the
  *I* form and nothing else (*вижу*, but *видишь*); and verbs of motion at their
  gentlest — *иду* (now, one way) against *хожу* (habitually), one pair, named as
  a pattern the rest will follow.
- **Etymology carried the chapter.** *знать* is English **know** with the silent
  *k* still pronounced, plus *notice / ignore / noble* and *diagnosis /
  agnostic*. *видеть* is \**weid-*, which meant *see* **and** *know* — hence
  *video* and *vision* on one branch and **wit**, **wise**, **witness** and
  *Veda* on the other, with Russian keeping both halves as *видеть* and *ведать*.
  *жить* is \**gʷeih₃-*, the root of *vīvere* and *bíos* — and of English
  **quick** in its older sense of *alive* (*the quick and the dead*,
  *quicksilver*). *идти → шёл* is *go → went*, suppletion in two languages for
  one reason. Where the root is **not** clean it says so: *говорить* is from
  Slavic \**govorъ* "a din", most likely imitative, with no tidy English cousin —
  and English **govern** is flagged as a false friend (Greek *kybernân*, "to
  steer", the ancestor of *cybernetics*).
- **One new letter, taught inline**: **г** (Greek gamma), inside *говорить*. The
  other five words need nothing the track has not already taught, which is
  itself worth saying out loud to a beginner. No separate reading course, per
  HL00.
- **Drivability held, deliberately.** All six lessons derive `voice`: no tables
  at all, no `script` blocks, no sight cues, and paradigm pairs written as
  running text ("*ты знаешь* against *ты говоришь*") rather than as a grid. That
  makes Chapter 3 the **first Russian chapter that is drivable end to end**, and
  it protects the 73% the HL-C32 remediation won. Corpus-wide: `voice` 957 → 963,
  `sight` and `pen` unchanged, `fullyDrivableChapters` 284 → 285.
- **Curriculum wiring**: a new `RU-PATH-007` segment on `SPINE-SAY-WHAT-I-DO`,
  with the six lessons attached as a required `RU-EXT-007-CORE-VERBS` extension.
  That node is stage **A2**, so this is the first content in the whole corpus to
  sit above A1 — `byLevel.A2` 0 → 6. The claim is scoped: the node's own
  concepts, `VERB-INFINITIVE` and `VERB-PRESENT-HABITUAL`, remain listed as
  omitted, because they are.
- **Known debt, recorded not hidden**: Chapter 3 has no LaTeX chapter and
  therefore no HL05 capability ledger entry. Russian's book chapters are
  handwritten and none was authored here. Noted in `README.md` and `roadmap.md`.
- Updated `roadmap.md` (Chapter 3 is authored; the old "Being and having" plan
  moves to Chapter 4), `session-map.md` (sessions S12–S16, carrying Chapter 2's
  *N+7 / N+15* resurfacings as promised) and `README.md`.

## 0.7.0 — 2026-08-06

HL-C32. Russian was the worst-performing track in the corpus on two independent
measurements — 9% drivable (2 `voice`, 15 `sight`, 5 `pen`, and **zero** lessons
reachable by ear in either chapter) and payoff representativeness of 0.20. Both
are now fixed, and the diagnosis matters more than the fix.

**The root cause was formatting, not content.** All fifteen `sight` lessons
tripped the same rule: `wide-table`. Not one carried a `script` block. Twelve
tripped nothing else at all; the three that also matched a sight cue matched
phrases like "the course's first **look at** case", which point at nothing on the
page. And the tables themselves were almost entirely cross-language
word→gloss lists — *"Language | 'yes' | built from"* — the exact material
`RU-C01-privet` and `RU-C01-zdravstvuyte` already carry as prose, which is why
those two were the track's only `voice` lessons. The same section, set two
different ways, produced two different modalities.

- Rewrote fourteen table-driven lessons so their word→gloss and letter→sound
  lists are speakable prose or bullets. No content was dropped, no comparison
  was shortened, and every lesson stayed under the duration budget.
- Left `RU-C02-practice-cases` as `sight`, deliberately. Its table is a
  cover-the-column retrieval drill: the table *is* the exercise, and linearising
  it would delete the lesson. An immovable `sight` lesson is a correct finding,
  not a failure.
- Left `RU-W02-false-friends-s-n`'s four-column table alone. It is a `writing`
  lesson, so it is `pen` regardless, and reformatting would buy nothing.
- Result: 16 `voice`, 1 `sight`, 5 `pen` — **9% → 73% drivable**, and **0 → 15
  lessons reachable in chapter-prefix order**. Chapter 1's first seven lessons
  and Chapter 2's first eight are now doable in the car. Corpus-wide drivable
  lessons rose from 694 (63%) to 708 (65%); Russian alone accounts for all of it.
- Migrated `RU-C02-practice` — one lesson, not the remaining fifteen — to schema
  v2 so the chapter payoff can point at the chapter's actual consolidation
  lesson instead of `RU-C02-kak-cross-language`, a cross-language etymology
  lesson that was standing in only because it was the last schema-v2 lesson by
  sequence. The migrated lesson runs the full introduction formally, switches it
  informally, and assesses ten of Chapter 2's fifteen introduced atoms:
  **representativeness 0.20 → 0.67**, above the 0.5 policy floor. Closure is
  strict — every line uses only already-taught material, and the lesson
  introduces nothing new.
- Added `RU-EXT-006-CONSOLIDATION` to `curriculum.json` and placed
  `RU-C02-practice` in `RU-PATH-006`, because every schema-v2 lesson naming a
  spine node must appear in the local realization map.

**Still open, and honestly so.** Fifteen Russian lessons remain schema v1 — all
twelve of Chapter 1, plus `RU-C02-ochen-priyatno`, `RU-C02-practice-cases` and
`RU-C02-practice-zero-copula`. That is why Chapter 1 still has no `chapters.json`
entry: it introduces no knowledge atoms, so no payoff can assess anything. And
because `sequence` is a schema-v2 field, Chapter 1's modality ordering is still
alphabetical rather than pedagogical, and `RU-C02-practice` now sorts ahead of
its own prerequisite `RU-C02-ochen-priyatno` in that ordering. Neither affects
validation or the drivable prefix; both close only with the full migration.

## 0.6.0 — 2026-08-06

- Added `chapters.json`, the HL05 chapter capability ledger, covering Chapter 2:
  the reader can give a name, ask for one, and choose *ты* or *вы* to match the
  relationship.
- Pointed the Chapter 2 payoff at `RU-C02-kak-cross-language`, the chapter's
  last schema-v2 lesson by sequence. The chapter's own `practice-mix` lessons
  are still schema v1, so they declare no `practises.knowledge` and cannot carry
  an honest `assesses` list yet.
- Took the chapter title and label from `book/chapters/ch02-introducing-yourself.tex`.
  Russian has no `core/book-generation.json` targets, so the LaTeX source is the
  only other place the printed name is written down.
- Omitted Chapter 1 rather than stubbing it: all twelve of its lessons are
  schema v1, so it has no assessable payoff. Its absence is the debt the HL05
  gap report exists to measure.
- Measured payoff representativeness for Chapter 2 at 3/15 introduced atoms
  (0.20), below the 0.5 policy floor. Recorded, not padded — the shortfall is
  the schema-v1 practice lessons, and it closes when they migrate.

## 0.5.0 — 2026-08-03

- Migrated the closed Chapter 2 chain from *я* through the cross-language
  naming comparison to schema v2 with stable sequence, typed body blocks,
  transitive knowledge ownership, and explicit skill/mode/strand metadata.
- Added objective final-recall checks for polite *вы* and the *how/what* naming
  contrast, including accepted variants, feedback, and eight-second response
  budgets consumed directly by Language Ladder.
- Preserved the existing lesson prose and prerequisite order while reducing the
  mapped Russian non-lexical self-check backlog from two lessons to zero.

## 0.4.0 — 2026-08-02

- Corrected four honest four-minute estimates that had been rounded up to five
  even though the shared duration model already placed them below that boundary.
- Split the genuinely long Chapter 2 material into prerequisite-ordered support
  lessons: *why вы is polite* and *why Russian asks how rather than what*.
- Replaced the long cumulative recap with three focused practices for the
  formal/informal exchange, subject/object person shapes, and the precise scope
  of the zero copula.
- Preserved the complete etymological and cross-language content while making
  each individual lesson independently doable in under five minutes. Updated
  the roadmap and session map to distinguish lesson duration from a commute
  session that intentionally combines several lessons.

## 0.3.0 — 2026-08-02

- Added the first downloadable LaTeX edition.
- Published the two authored chapters as dependency-ordered micro-sections with
  inline Cyrillic, grammar, usage, etymology, and retrieval prompts.
- Kept the existing canonical duration debt explicit for the shared gap-report
  and lesson-splitting backlog items.

## Chapter 2 — Introducing yourself

Russian was one of **two tracks still at Chapter 1** (with Latin); every other
track had reached Ch. 4 or beyond. This closes that gap, following the
Ch. 2 plan the roadmap already set out.

- **`RU-C02-ya`** — *я*, one letter and one of the least-changed words in the
  family: PIE \**eǵh₂(om)* → Proto-Slavic \**azъ* → a single vowel. Cousins
  *ego*, *ich*, *I*, *egṓ*. **Pronouns are the least borrowable part of a
  language**, which is why this row survives when everything around it turns over.
  Warns that **я is not a mirrored Latin R**.
- **`RU-C02-ty-vy`** — the split English threw away. *Thou* fell out of standard
  English through the 1600s (hedged — it survives in Quaker usage, northern
  dialect and liturgy), leaving *you* for everyone.
  - *вы* is polite the way French *vous* is, and the lesson is careful about
    **who actually did what**: Russian and French use the **2nd-person plural**,
    German borrows the **3rd-person plural** (*Sie*, **not** *ihr*), and **Spanish
    used no plural at all** — *usted* ← *vuestra merced*, "your grace". A draft
    lumped Spanish in; `ES-C03-tu-usted` and `GE-C02-du-sie` both say otherwise.
    Russian's *вы*-politeness is also flagged as a likely **18th-century import**
    rather than an independent invention.
  - The cognate table is **honest about its third row**: *вы* and *vōs* continue
    PIE \**wos*, but English *you* and German *ihr* come from the paradigm's
    **other** stem \**yūs* — so that row is half a set, and says so.
  - Introduces **ы** properly, since *ты*/*вы* turn on it and it is the hardest
    Russian vowel for an English speaker — and **adds it to
    `pronunciation-reference.md`**, where it was missing from every letter group,
    with a new `yery-vowel` id the lesson cites. The reference previously had no
    entry for it at all.
- **`RU-C02-menya-zovut`** — the naming construction, which is the chapter's real
  content. *Меня зовут Анна* is literally "**[they] call me Anna**": there is
  **no word for "my"** and **no word for "name"** in it. The "they" is nobody — a
  bare plural verb meaning "people in general", which English does too in *they
  say it'll rain*.
  - This is the course's **first look at case**. *Меня* is not *я* but its object
    form, and the lesson is careful to set the expectation honestly: English does
    this with about six **pairs** (*I/me*, *he/him*…), Russian does it with
    **every noun and pronoun**. The learner isn't asked to learn the system — only to notice
    that the *shape* carries the meaning.
- **`RU-C02-kak-vas-zovut`** — asks **how** they call you, not *what*. Russian,
  French (*comment*) and Spanish (*cómo*) all ask about an **action**; English is
  the odd one out, asking about a **possession**. Completes the object-form set
  (*меня / тебя / вас*) so the exchange is a matched pair.
- **`RU-C02-ochen-priyatno`** — "very pleasant", with no *I am* and no *to meet
  you*. The etymology is the payoff: *приятно* ← Slavic *prijati* "to favour" ←
  PIE \**preyH-* "to love, please" — the root behind Russian **приятель**
  "friend" **and** English **friend**, arriving from opposite ends of Europe.
  **Free** most likely belongs to the same family ("belonging to the beloved
  household", i.e. not a slave), hedged as the usual account rather than asserted.
- **`RU-C02-practice`** — drills the full formal exchange, then the informal one
  (**greeting and pronoun change; the verb *зовут* never does**), two of the
  shapes each pronoun takes, and a "what Russian leaves out" table.
  - The zero-copula point is **scoped to the one sentence that shows it**:
    ***очень приятно*** has no verb at all. A draft claimed "no sentence in the
    chapter contains a word for *is*" and that "Russian has none in the present"
    — both wrong. *Меня зовут* has a verb (*зовут*); it simply isn't a copular
    sentence. And Russian **does** have a present-tense **есть**, which Ch. 1
    already met inside **нет** = *не + есть*, and which the roadmap's own Ch. 3
    section reuses for *у меня есть*.

### Conventions checked, not assumed

- Concept tags are **canonical** (`PRONOUN-I`, `PRONOUN-YOU`, `INTRO-MY-NAME-IS`,
  `INTRO-WHATS-YOUR-NAME`, `INTRO-NICE-TO-MEET-YOU`) — verified present in
  `concepts/taxonomy.json`, so no new entries were needed.
- **я and ты/вы are separate lessons**, which is why. A draft covered all three in
  one lesson tagged `PRONOUN-I` — leaving Russian with **no `PRONOUN-YOU` node**
  at all, and dropping the ты/вы split out of a join that **14 other lessons** take part in.
  **Most** other tracks split them the same way (`FR-C02-je` + `FR-C02-tu-vous`,
  `GE-C02-ich` + `GE-C02-du-sie`) — though Italian and Portuguese do not, carrying
  only `PRONOUN-I`.
- The practice lesson matches **this track's own** shape — `type: practice-mix`
  with `CH2-PRACTICE` — not the Arabic track's `practice`/`REVIEW`. `CH2-PRACTICE`
  was already in the taxonomy note's label list.
- `sounds:` ids come from `russian/pronunciation-reference.md`. A first draft used
  `cyrillic-new-shapes`, which **is** used by `RU-W03`/`RU-W04` but is **missing
  from the reference's id list** (as is `cyrillic-honest`) — so the new lessons
  use `cyrillic-false-friends` and `stress-unmarked`, both canonical.
- The read-now-draw-later notes (**я**, **ч**, **ы**, **ь**) were written after listing
  the writing track's actual headwords (в р · с н · б д · п и · е т) — none of
  those four has been taught.

## [Unreleased]

### Added — Chapter 1 (Greetings & courtesy)
- Track scaffold: `README.md`, `roadmap.md`, `session-map.md`,
  `pronunciation-reference.md`, and `track.json` declaring the **Cyrillic**
  script (so the data layer resolves Russian → cyrillic).
- Six word lessons, Cyrillic taught inline:
  - `RU-C01-privet` — привет (informal hi); the *-вет* "speak" root ↔ **Soviet**.
  - `RU-C01-zdravstvuyte` — здравствуйте (formal hello); "be healthy", polite `-те`.
  - `RU-C01-spasibo` — спасибо (thank you); worn-down *спаси Бог*, "God save you".
  - `RU-C01-da` — да (yes).
  - `RU-C01-net` — нет (no); *не + есть* "not-is", the PIE **\*ne** cousin of *no/not*.
  - `RU-C01-pozhaluysta` — пожалуйста (please / you're welcome); the favour root *жал-*.
- `RU-C01-practice` — Chapter 1 recap drilling the four false friends (в=v, р=r,
  с=s, н=n) and the greeting exchange.
- Uses the canonical concept taxonomy; adds `COURTESY-PLEASE` to the taxonomy for
  пожалуйста.

### Added — Writing the letters (the "break it apart and write it" strand)
- Three `writing`-type lessons (the HL02 hand-writing surface, taught inline the
  same etymology-first way; no `concept_tag`, exempt from the cross-language join).
  Each breaks a letter into its component strokes with a stroke order and reviews
  the Chapter 1 word it lives in:
  - `RU-W01-false-friends-v-r` — writing **в** (v, ← Greek beta) and **р** (r, ←
    Greek rho): the two false friends from *привет*, stroke by stroke.
  - `RU-W02-false-friends-s-n` — writing **с** (s, ← Greek sigma) and **н** (n,
    the Latin-*H* look-alike), completing the four false friends в·р·с·н.
  - `RU-W03-new-shapes-b-d` — writing **б** (b) and **д** (d, ← Greek delta), two
    shapes with no Latin disguise; contrasts б vs в (the top flag + one belly vs
    two bellies).
  - `RU-W04-privet-letters-p-i` — writing **п** (p, ← Greek pi Π) and **и** (ee,
    the quiet false friend — a *backwards* Latin N: its diagonal **rises** where
    N's falls); contrasts п (top bar) vs н (middle bar).
  - `RU-W05-privet-letters-e-t` — writing **е** (ye, an *iotated* honest vowel) and
    **т** (t, ← Greek tau); **completes every letter of привет** (п·р·и·в·е·т), so
    the learner can hand-write their first Russian word end to end.
- Stroke data is the canonical `data/scripts/cyrillic.json` the companion
  `language-ladder` app renders, so the lessons and the app agree.

### Notes
- Headwords use the lowercase citation form (Cyrillic case is not yet in the
  script inventory).
- The LaTeX book is authored next (lessons-first workflow), typeset with the
  vendored `NotoSansCyrillic-Static.ttf`.
