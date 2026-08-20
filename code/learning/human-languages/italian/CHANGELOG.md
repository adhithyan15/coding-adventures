# Changelog

## Writing begins with the first greeting (#12299)

- Migrate `IT-C01-ciao` to the measured lesson schema and add one visible-model
  observe-and-trace microstep for the four letters in **ciao**.
- Add a separate two-minute guided copy of the same greeting. Neither step asks
  for spelling from memory, a second word, or free composition.
- Record the first two HL19 stages as a required pre-A1 extension. This closes
  only Italian's no-writing-at-all cliff; later writing stages remain debt.

## The pre-A1 noun tranche: coffee to sugar, friends to persons, heart to throat (HL09)

Fifteen everyday nouns across four chapters (22–25), the track's first tranche
built entirely of nouns rather than verbs, targeting the level-gate's
`vocabulary` blocker at pre-A1. Wired to three pre-A1 spine nodes through new
`IT-PATH-026`..`029` segments and `IT-EXT-026`..`029` language-specific
extensions:

- **Chapter 22 — Coffee, Tea, Milk, and Sugar** (`IT-C22-caffe`, `IT-C22-te`,
  `IT-C22-latte`, `IT-C22-zucchero`; concepts `IT-FOOD-COFFEE`, `IT-FOOD-TEA`,
  `IT-FOOD-MILK`, `IT-FOOD-SUGAR`) on `SPINE-POLITE-REQUEST-REPAIR`, alongside
  Chapter 11's bread and wine. Three loanwords and one inherited word at the
  same table: *caffè* through Ottoman Turkish from Arabic *qahwa*, *tè*
  through Dutch from Hokkien Chinese *tê* (the sea route, against the *chai*
  languages' overland one), *zucchero* from Arabic *sukkar* by a different
  road than *caffè* — through Old French, the same route English *sugar*
  took — and *latte*, straight from Latin *lac*, never borrowed at all.
  *Zucchero* is also the first noun in the track that actually needs Chapter
  1's second article, *lo*, on a real word rather than an invented example.
  11 new atoms.
- **Chapter 23 — Friends, Family, and a Name** (`IT-C23-amico-amica`,
  `IT-C23-famiglia`, `IT-C23-nome`, `IT-C23-persona`; concepts
  `IT-PEOPLE-FRIEND`, `IT-FAMILY-GROUP`, `IT-WORD-NOME`, `IT-WORD-PERSONA`) on
  `SPINE-EXCHANGE-NAMES`. *L'amico*/*l'amica* ← *amicus*/*amica*, from
  *amāre* ("to love") — and English *enemy* is the same root with a "not" in
  front (*inimicus*). *Il nome* is the word Chapter 3's *mi chiamo* ("I call
  myself") always talked around without ever saying. *La persona* closes the
  chapter: grammatically feminine by fixed category, never by the sex of
  whoever it names — a second, sharper example of the gender-vs-referent gap
  Chapter 17's *la mano* first raised. 11 new atoms.
- **Chapter 24 — Heart, Eyes, Ears, and Mouth** (`IT-C24-cuore`,
  `IT-C24-occhio`, `IT-C24-orecchio`, `IT-C24-bocca`; concepts
  `IT-BODY-HEART`, `IT-BODY-EYE`, `IT-BODY-EAR`, `IT-BODY-MOUTH`) and
  **Chapter 25 — Nose, Stomach, and Throat** (`IT-C25-naso`, `IT-C25-stomaco`,
  `IT-C25-gola`; concepts `IT-BODY-NOSE`, `IT-BODY-STOMACH`, `IT-BODY-THROAT`)
  on `SPINE-CHECK-WELLBEING`, paying off the "next: the rest of the body"
  line Chapter 17's *la mano* lesson ended on. *Occhio* and *orecchio* are
  built by the same Italian sound-law, applied twice: an unstressed vowel
  drops from *oculus*/*auricula* and the leftover *-c(u)l-* hardens to
  *-cchi-*. *Bocca* is Vulgar Latin slang for "cheek" that displaced the real
  classical word for mouth, *os/oris*, across nearly all of Romance. *Stomaco*
  is the one body word that passed through Greek (*stómakhos*) before it ever
  reached Latin. 11 + 8 new atoms.

**Vocabulary, measured with `buildCurriculumGapReport` before and after:**

    headwords at or below pre-A1     30 -> 45
    vocabulary shortfall (of 300)   270 -> 255
    track vocabulary (any level)     67 -> 82
    reinforcement shortfall (pre-A1) 24 -> 13
    attained / inProgressAt        null / pre-A1  (unchanged)

Fifteen new word lessons moved the pre-A1 headword count by exactly fifteen —
`vocabularyOf()` counts distinct `headword:` strings, and even the paired
`l'amico, l'amica` lesson contributes one, the same as every other track's
prior noun tranche found. Closing pre-A1 on vocabulary alone still needs
~255 more lessons of this shape.

The reinforcement drop is real work, not incidental: eleven of the 24 atoms
the continuity ledger reported as revisited fewer than twice before this
tranche are now revisited at least twice, rescued by reaching back into
`practises.knowledge` from these new lessons — *così così* and *prego*'s
sounds from Chapter 2, *età*'s three atoms from Chapter 14, *parlo italiano*
from Chapter 5, and *mano*/*testa* from Chapter 17. The other thirteen (seven
`(practice)`-lesson atoms, the *passato remoto*'s three, and three farewell
sound-atoms from Chapter 4) were left alone deliberately: none of them had an
honest home in a noun tranche about drinks, people and the body, and forcing
one would have been exactly the mismatched-node move the prior wave warned
against.

**A structural finding, not a new one — confirmed, and sharpened.** The prior
wave's three tracks (Hindi, Arabic, Tamil) reported that the seven pre-A1
spine nodes are all social speech acts with no concept for naming a concrete
object in front of you, and dropped household words that had no honest home
rather than force them on. Italian's level-gate report carries a second,
different `spine-nodes` finding worth naming precisely: `SPINE-RESPOND-BASIC`
— the node for *yes*/*no*/*okay* — is not merely under-realized, it has **zero**
lessons (`"segments": []` in `curriculum.json`, all four of its concepts
correctly listed under `omits`). Italian has never taught *sì* or *no*. This
tranche does not touch it: it is a `vocabulary` gap on a `SAY-WHAT-I-DO`-shaped
node, not a `POLITE-REQUEST-REPAIR`/`EXCHANGE-NAMES`/`CHECK-WELLBEING` noun
gap, and belongs to whichever tranche authors Italian's basic responses.

Gates: zero forward references, zero atom-budget violations from these
fifteen lessons (Italian's three pre-existing violations — `IT-C13-rosso-blu`,
`IT-C14-eta`, `IT-C15-passato-remoto` — predate this branch), all four
chapters at or under `maxNewAtomsPerChapter: 12`, and every lesson computes
under 300 effective seconds. `narration/ch22.txt`–`ch25.txt` read correctly
aloud, including the two 3-column tables (the article-elision table in
`IT-C23-amico-amica` and the sound-law table in `IT-C24-orecchio`). The
25-chapter, 160-page book compiles under XeLaTeX with zero `Missing character`
errors.

## The final verb tranche: carrying, buying, waiting, meeting, playing, getting, answering

Italian authors the last seven core verbs no track in the corpus taught, moving
from **14 of 40** to **21 of 40**. With Spanish, Latin and Portuguese landing the
same seven in parallel, **every one of the 40 core verbs is now realized
somewhere** — `verbCoverage` reports `universallyMissing: []` for the first time.

Seven one-verb lessons in **two** chapters again, for the same reason as
Chapters 18–19: seven lessons introduce 21 atoms, more than
`maxNewAtomsPerChapter: 12` allows in one chapter.

- **Chapter 20 — Carrying, Buying, Waiting, Meeting** (`IT-C20-portare`,
  `IT-C20-comprare`, `IT-C20-aspettare`, `IT-C20-incontrare`; concepts
  `VERB-BRING`, `VERB-BUY`, `VERB-WAIT`, `VERB-MEET`). Four regular *-are*
  verbs, so the endings cost nothing and the whole chapter's weight sits on the
  roots. Etymological thread: three of the four are a plain Latin verb with a
  small word welded to the front — *com-*, *ad-*, *in-*. 12 new atoms, at the
  chapter budget.
- **Chapter 21 — Two Ways to Play, and Two Verbs That Bend** (`IT-C21-giocare`,
  `IT-C21-ottenere`, `IT-C21-rispondere`; concepts `VERB-PLAY`, `VERB-GET`,
  `VERB-ANSWER`). 9 new atoms.
- **The signature: *giocare* against *suonare*.** English has one verb for
  playing a game and playing an instrument; Italian has two, from two unrelated
  Latin words — *iocus* "a jest" (→ *joke*, *jocular*, *jeopardy* ← *jeu parti*)
  and *sonus* "a noise" (→ *sound*, *sonata*, *unison*, *dissonant*,
  *counterpoint*). The split is not a rule to memorise; it is the merger English
  performed and Italian never did.
- **A correction the brief did not contain.** The tranche brief asserted
  *comprare* ← *comparāre* → English *compare* as "a non-obvious true link". It
  is false. Latin had **two** verbs spelled *comparāre*: one on *parāre* "to
  make ready, procure" (→ Italian, Spanish and Portuguese *comprare/comprar*)
  and one on *pār* "equal" (→ English *compare*, *par*, *parity*, *peer*,
  *pair*, *umpire*). They are homographs, not relatives. Verified against
  Wiktionary's `comprare` entry, which derives it from `cum + parō`. The refusal
  is authored as the lesson's third atom (`IT-ETYMON-COMPRARE-04`, "The
  look-alike, taken apart"), in the same move the track already makes on
  *domandare* ≠ *demand*, and the real *parāre* family is taught in its place:
  *prepare*, *repair*, *separate*, *apparatus*, *apparel*, *emperor*, and
  Italian's own *parare* behind *parry*, *parasol* and *parachute*.
- **Reinforcement (HL09 §7) at two cadences.** Every lesson practises atoms from
  the immediately preceding one to three lessons, across the chapter seam, and
  each payoff reaches back several chapters. The tranche **rescues eleven
  pre-existing orphans** — atoms taught once and never touched again:
  `IT-LEX-MI-PIACE-02`, `IT-GRAMMAR-MI-PIACE-03` and `IT-NOTICE-MI-PIACE-04`
  (Ch. 19, via *mi piace giocare* and the extended reaching-thread);
  `IT-LEX-ETA-02`, `IT-ETYMON-ETA-03`, `IT-GRAMMAR-ETA-04` and
  `IT-GRAMMAR-ETA-05` (Ch. 14, via the silent *h* of *giochi* against *hanno*,
  and via Spanish *tengo* / Portuguese *tenho* being *tenēre*);
  `IT-LEX-PASSATO-PROSSIMO-ESSERE-02`, `IT-GRAMMAR-PASSATO-PROSSIMO-ESSERE-03`
  and `IT-PRAGMATICS-PASSATO-PROSSIMO-ESSERE-04` (Ch. 16, via *ho incontrato
  Anna* against *Anna è andata*); and `IT-LEX-MANO-02` (Ch. 17, via *a mano*).
  Chapter 21's payoff also retrieves `IT-LEX-PASSATO-REMOTO-02`,
  `IT-PRAGMATICS-PASSATO-REMOTO-04` and `IT-NOTICE-PASSATO-REMOTO-05`, and
  `IT-GRAMMAR-MANO-03` — Chapter 17's principle that an "irregular" form is a
  regular one from a system you can no longer see, applied to *risposto*.
  Measured on the committed corpus, Italian's never-revisited count falls
  **28 → 14** while its taught-atom count rises **135 → 156**. Only **three**
  new orphans appear, and all three belong to the track's final lesson, where no
  later lesson exists to reach them.
- **No forward references.** Every example uses vocabulary the track has already
  taught — *pane*, *acqua*, *vino*, *mano*, *anni*, *Marco*, *Anna* — and no
  lesson teases a later one.
- **Wiring.** `SPINE-SAY-WHAT-I-DO` gains two real path segments, `IT-PATH-024`
  and `IT-PATH-025`, and drops the seven concepts from its `omits`.
  `chapters.json` gains capability entries 20 and 21,
  `core/book-generation.json` two Italian targets, and `book/book.tex` two
  `\input` lines. Book chapters, the modality manifest and the narration export
  are regenerated; all seven lessons classify **voice**, so the whole tranche is
  drivable. Every lesson's computed duration lands between 281 s and 298 s,
  inside the 300 s gate.

## Two verb chapters: the mind, and the verb that runs backwards

Italian joins Spanish, Latin and Portuguese on eight core verbs, taking the
track from **6 of 40** core verbs to **14 of 40**. Eight one-verb lessons in
**two** chapters, not one: eight lessons introduce more atoms than
`maxNewAtomsPerChapter: 12` allows in a single chapter, and splitting is the
resolution rather than raising the budget.

- **Chapter 18 — Four Verbs of the Mind** (`IT-C18-pensare`, `IT-C18-capire`,
  `IT-C18-leggere`, `IT-C18-scrivere`; concepts `VERB-THINK`,
  `VERB-UNDERSTAND`, `VERB-READ`, `VERB-WRITE`). One verb from each of Italian's
  three conjugation families, and the tranche where the **-isc- class** earns
  its keep: *capisco / capisci / capisce* but *capiamo / capite*, a family
  Italian shares with French (*je finis, nous finissons*) and Spanish does not
  have. Etymological thread: all four Latin roots name a **physical act** before
  a mental one — *pēnsāre* to weigh, *capere* to seize, *legere* to gather,
  *scrībere* to scratch. 12 new atoms, at the chapter budget.
- **Chapter 19 — Taking, Asking, Helping — and Mi Piace**
  (`IT-C19-prendere`, `IT-C19-chiedere`, `IT-C19-aiutare`, `IT-C19-mi-piace`;
  concepts `VERB-TAKE`, `VERB-ASK`, `VERB-HELP`, `VERB-LIKE-LOVE`). The payoff
  is *mi piace*, which is backwards exactly as Spanish *gustar* is: *mi piace il
  vino* says the wine pleases me, so the thing liked is the subject and the verb
  goes plural for it. Chapter 3 taught *piacere* as a one-word courtesy; this
  chapter reveals it as the same verb's dictionary form. 11 new atoms.
- **The two chapters are linked by their roots, not by a cross-reference.**
  *Capire* is built on *capere*, "to seize"; *prendere* on *prehendere*, a
  **different** Latin verb with the same meaning. Italian built understanding on
  one grasping-verb and taking on the other, and Chapter 19 says so by looking
  **back** at Chapter 18 rather than Chapter 18 teasing forward.
- **Reinforcement (HL09 §7), measured rather than asserted.** Each payoff
  re-practises atoms from *earlier* chapters, not only its own: Chapter 18's
  reaches back to `IT-GRAMMAR-PARLARE-04` (Ch. 5), `IT-GRAMMAR-PASSATO-PROSSIMO-02`
  (Ch. 15), `IT-ETYMON-NUMERI-6-10-03` (the *-ct-* → *-tt-* law, Ch. 6) and
  `IT-ETYMON-MANO-04` (Ch. 17, via *manoscritto*); Chapter 19's reaches back to
  `IT-SOUND-PIACERE-02`, `IT-ETYMON-PIACERE-03`, `IT-GRAMMAR-PIACERE-04` and
  `IT-GRAMMAR-MI-CHIAMO-04` (Ch. 3), `IT-ETYMON-STAGIONI-02` (Ch. 9),
  `IT-ETYMON-ACQUA-VINO-03` (Ch. 11) and Chapter 18's *leggere* and *capire*.
  On the committed corpus, **none of the 23 atoms these chapters introduce
  misses a reinforcement window**, and only **3 of the 23** are never revisited
  — the three introduced by the track's final lesson, where no later lesson
  exists to revisit them. That is a 13% orphan rate against the corpus's 50%.
  The tranche also rescues a pre-existing orphan: `IT-ETYMON-MANO-04`, taught in
  Chapter 17 and never practised again, is now retrieved twice (via
  *manoscritto* and via *mandāre*). Italian's orphan count therefore moves 26 →
  28 while its taught-atom count moves 112 → 135.
- **No forward references.** The continuity walk reports **zero** forward
  references from any of the eight lessons, and no lesson closes with a "next
  time we'll meet X" tease. Every example uses only vocabulary the track has
  already taught — *pane*, *acqua*, *vino*, *ora*, *stagioni*, *Roma*,
  *italiano* — which is also why *non capisco* is **not** taught here: Italian's
  `SPINE-NEGATE-AND-ASK` is still unrealised, so *non* does not exist yet in
  this track. The lesson teaches *Capisco* and *Capisci?* instead.
- **Wiring.** `SPINE-SAY-WHAT-I-DO` gains two real path segments, `IT-PATH-022`
  and `IT-PATH-023`, and drops the eight concepts from its `omits`; the segment
  ledger is refreshed against the authored path. `chapters.json` gains capability
  entries 18 and 19, `core/book-generation.json` two Italian targets, and
  `book/book.tex` two `\input` lines. Book chapters, the modality manifest and
  the narration export are regenerated; all eight lessons classify **voice**, so
  both chapters are fully drivable.
- Every lesson is under five minutes as computed, not merely as declared: the
  effective durations run 257–298 s against the 300 s gate.

## Joined the cross-language verb corpus

- Retagged six verb lessons from language-local `IT-VERB-*` ids to the canonical
  `VERB-*` concepts, so Italian's verbs now join the cross-language corpus
  instead of being unrelated to every other track's:
  `IT-VERB-BE` → `VERB-BE` (essere), `IT-VERB-HAVE` → `VERB-HAVE` (avere),
  `IT-VERB-ANDARE` → `VERB-GO`, `IT-VERB-PARLARE` → `VERB-SPEAK`,
  `IT-VERB-ABITARE` → `VERB-LIVE`, `IT-VERB-LAVORARE` → `VERB-WORK`.
- `IT-VERB-STARE` stays namespaced on purpose. *stare* is the other half of the
  essere/stare pair and `essere` already takes `VERB-BE`; a second lesson on the
  same canonical concept is a duplicate realisation, and no core concept covers
  the state-of-being verb on its own.
- Rewired `curriculum.json` for the realisation rule that a canonical concept
  owned by `SPINE-SAY-WHAT-I-DO` brings with it, using two different mechanisms
  because the six lessons are not one case:
  - **Chapter 5 moved.** `IT-PATH-011` is split in place into a
    `SPINE-SAY-WHAT-I-DO` segment (parlare, abitare, lavorare) and a
    `SPINE-POLITE-REQUEST-REPAIR` segment (parlo-italiano, practice), and the
    three verbs' authored `spine_node` pins are corrected to match. Chapter 5 is
    titled "The First Verbs" and teaches the regular `-are` present; its pin to
    `SPINE-POLITE-REQUEST-REPAIR` was a placeholder from before the A2 verb
    tranche existed, when a present-tense lesson had no node it could legally
    declare. Nothing moves relative to anything else — the split is where the
    chapter already changed subject.
  - **Chapters 14 and 16 stayed put, recorded as `relocates`.** avere is taught
    in "Having and Telling Your Age" to build *ho vent'anni*, and essere/andare
    in the location-and-past chapter; those pins are right, and forcing the three
    into a "what I do (present)" segment would have split two coherent chapter
    units and filed past-tense material under the present. `relocates` records
    where the concept is realised without falsifying the authored pin.
- `VERB-BE`, `VERB-HAVE`, `VERB-GO`, `VERB-SPEAK`, `VERB-LIVE` and `VERB-WORK`
  leave `SPINE-SAY-WHAT-I-DO`'s `omits`; every node's `segments` ledger is
  refreshed against the authored path. `IT-EXT-011-LANGUAGE-SPECIFIC` keeps only
  parlo-italiano — the three verbs are shared realisations now, not
  language-specific support. Path and extension ids are renumbered to stay
  ascending in path order, the convention every other track follows.
- `chapters.json` chapter 5 now declares both nodes it realises. Chapters 14 and
  16 needed no change, a direct consequence of the `relocates` choice.
- No chapter reordered, no lesson renumbered, no lesson prose touched — only
  frontmatter `concept_tag` and three `spine_node` pins.

## HL05 chapter capabilities — Chapters 2–17

- Added `chapters.json`, the track's HL05 capability ledger, with 16 entries
  covering every Italian chapter that owns a `core/book-generation.json` target.
- Each entry declares a first-person `canDo`, the shared spine nodes the chapter
  realises (derived from `curriculum.json` path segments), and a `payoff` naming
  the lesson that proves the claim, its kind, a one-line summary, and the
  knowledge atoms it exercises. Every `assesses` list is exactly the payoff
  lesson's own `practises.knowledge` — nothing invented.
- Payoff selection: Chapters 2–5 use their terminal `practice-mix`. Chapters
  6–17 have no practice lesson, so the payoff is the chapter's last lesson by
  sequence, which is where its recombination and wrap-up recall live.
- **Skipped, deliberately:** Chapter 1. Its lessons are still schema v1 with no
  declared `practises.knowledge`, so no payoff can be claimed honestly, and it
  owns no book-generation target either. The absence is tracked debt; a stub
  would have destroyed the HL05 gap report's signal.
- **Representativeness** against the 0.5 threshold in
  `core/chapter-policy.json`: fifteen of sixteen chapters score 1.00, and
  Chapter 16 scores 0.67 (6/9). No Italian chapter falls below the threshold.

## Warning-free 104-page book — 2026-08-03

- Taught the shared inline renderer to preserve backslash-escaped Markdown
  punctuation, repairing the reconstructed `*parabolāvit` form in Chapter 15,
  and added regression coverage for the exact bold form.
- Removed paragraph indentation before generated tables, added a bookmark-safe
  Chapter 1 title, and made intentionally short lesson pages ragged-bottom.
- Tightened only the canonical prose and table cells responsible for horizontal
  layout warnings, then recalculated their sub-five-minute duration budgets.
- Forced a clean XeLaTeX build with zero missing glyphs, overfull or underfull
  boxes, duplicate destinations, Hyperref warnings, or LaTeX warnings. All 104
  rendered pages and the complete Preface/pronunciation/Chapter 1–17 outline
  were inspected successfully.

## Canonical book Chapters 2–17 — 2026-08-03

- Migrated all 49 lessons in Chapters 2–17 to the strict schema-v2 shared-spine
  contract with prerequisite-closed knowledge boundaries and sub-five-minute
  duration budgets.
- Added sixteen deterministic chapter targets and source hashes so Language
  Ladder and the downloadable book consume and verify the same canonical lesson
  AST instead of maintaining separate copies.
- Expanded the book from 13 to 104 pages, added width-aware generated tables,
  and taught the renderer portable TeX fallbacks for scholarly Unicode symbols.
- Verified zero missing glyphs and duplicate destinations, inspected all 104
  rendered pages, and retained the remaining layout/bookmark cleanup as HL-B15.

## Sub-five-minute remediation — 2026-08-02

- Corrected seventeen declared five-minute estimates whose lesson bodies
  already compute below 300 seconds.
- Replaced the three computed violations with four prerequisite-ordered
  micro-lessons: informal → formal → register-neutral wellbeing questions, then
  *essere* forms → borrowed *stato* → *andare* → participle agreement.
- Preserved the original etymology and cross-language comparisons while making
  each register, metaphor, suppletive stem, and grammar rule independently
  learnable in under five minutes. The shared report now measures zero Italian
  duration violations.
- `IT-C02-practice` at 297 computed seconds and `IT-C17-mano` at 298 are the
  tightest remaining Italian lessons and should be watched during copy edits.

## Chapter 17 — The body: the pot kept whole, and a noun that breaks the rule

- **Chapter 17 authored** (`IT-C17-testa`, `-mano`) — the **body**, the theme the
  parallel-track roadmaps name next.
- **la testa** (`IT-C17-testa`): the same pot-word as French's *tête*, and the
  point is **how little Italian did to it**. Latin ***testa*** → Italian
  ***testa***, almost unchanged (the spelling is identical; the vowel opened); French wore the identical word down to *tête*. That is
  the **conservative-sister** pattern this track keeps meeting — *acqua* against
  *eau* in Ch. 11, *parlato* keeping the Latin *-t-* that *parlé* wore away in
  Ch. 15.
  - And where French gave *caput* away almost entirely, **Italian kept it as a
    live everyday word**: ***il capo***, boss/chief — which English borrowed
    straight out of Italian — plus *capitale*, *capitolo*, *capitano*. So Italian
    runs both the old head-word and its slang replacement in parallel.
- **la mano** (`IT-C17-mano`): chosen because it **breaks the *-o*/*-a*
  tendency** — feminine, ending in *-o*, plural *le mani*. (A draft credited that
  rule to Chapter 1; `IT-C01-il-la-lo` teaches the opposite — "learn each noun
  **with** its article" — so the lesson now states the tendency itself and cites
  Ch. 1 as the reason *not* to trust endings.) The explanation is the
  lesson: Latin *manus* was **fourth declension**, a class whose nouns ended
  *-us* but could be feminine. Italian flattened five Latin declensions to
  **three** productive classes (*-o*/*-i*, *-a*/*-e*, and *-e*/*-i* as in
  *notte/notti*, which Ch. 1 already gave the learner),
  and left this word holding an old gender with an ending that now looks
  misfiled. **The word didn't change; the system around it did.**
  - Stated as a general principle, because it recurs everywhere in this
    curriculum: **an "irregular" word is usually a regular word from a system
    that no longer exists.**
  - English took **manage** specifically from Italian *maneggiare*, "to handle a
    **horse**" — so every manager is etymologically working in a riding school.

## Chapter 16 — *essere*, and the participle it borrowed from *stare*

- **Chapter 16 authored** (`IT-C16-essere`, `-essere-stato`, `-andare`,
  `-passato-prossimo-essere`).
  Ch. 15 taught only the *avere* half of the passato prossimo; *essere* existed
  in no lesson. This supplies it — and Italian turns out to have the most
  interesting version of the story, because **Ch. 2 already taught *stare***.
- **essere** (`IT-C16-essere`): the six forms, the `è`/`e` accent contrast
  (*Marco **è** italiano* vs *Marco **e** Anna* — one accent, two different
  words), and the *io sono* / *loro sono* collision that is one of the few places
  a pro-drop language has to keep its pronoun.
- **borrowed *stato*** (`IT-C16-essere-stato`) holds the chapter's centrepiece:
  Latin's *esse* and *stāre* both survived into Italian as separate living verbs
  — but *essere*'s own participle did not, so Italian filled the gap with
  ***stare*'s**. Both verbs' participle is **stato**, and ***sono stato***
  therefore means both "I have **been**" and "I have **stayed**", separable only
  by context.
- Set against the sisters in one table: **Spanish** split the pair fully (*ser*
  / *estar*), **French** kept *esse* as *être* and absorbed *stāre*'s whole
  *ét-* limb (*été*, *étant*, *étais*; *stāre* survives elsewhere in French too
  — *rester*, *coûter* — just not as a separate "to be"), **Italian** kept both
  but let them **overlap**. Italian sits exactly between the other two.
- **andare** (`IT-C16-andare`) is introduced explicitly as a new verb
  (`vado/vai/va/andiamo/andate/vanno`) rather than smuggled into the past. It is
  itself suppletive, shown as a
  stem table rather than prose so no form is left unaccounted for: **vad-**
  (*vado, vai, va, **vanno*** ← *vādere* "to stride") against **and-**
  (*andiamo, andate, andare, andato*, origin genuinely disputed, most likely
  *ambitāre*) — **four** present forms from the first stem, **two** from the
  second, plus the infinitive and participle. The lesson flags that ***vanno*
  files with *vado***, so the split is *not* singular-versus-plural — the same two-stem trick as *essere*, and the
  one behind Spanish *voy* vs *andar*.
- **passato prossimo with essere** (`IT-C16-passato-prossimo-essere`): after the
  dedicated atoms, opens with **`sono stato`** so the first *essere*-past costs
  **no new vocabulary**.
  - The **subject agreement** in all four endings (*andato / -a / -i / -e*), with
    the note that a woman says *sono andat**a***; explained via the same
    adjective fossil as French (*Anna è andata* ← "Anna **is** gone-away",
    describing her, like *Anna è stanca*).
  - Closes the three-language table: **French and Italian keep** participle
    agreement, **German drops** it — with the lesson body corrected to say the
    three systems were arrived at **in parallel** rather than "inherited", since
    German's is a native Germanic development that grew up alongside the Romance
    ones through contact. (The same correction was applied to the German track;
    this lesson's table is the other place the claim appears.)

## Chapter 15 — The compound past, and the one geography decides

- **Chapter 15 authored** (`IT-C15-passato-prossimo`, `-passato-remoto`): the
  everyday past, built on Ch.14's *avere* — reviewing Ch.5/14 via `reviews_of`.
- **passato prossimo** (`IT-C15-passato-prossimo`): *avere* + past participle
  (*-are*→*-ato*, *-ere*→*-uto*, *-ire*→*-ito*), with two callbacks. The silent
  **h** from Ch.14 is doing its job again — *ho parlato* can't be misread as *o
  parlato* ("or spoken"). And Italian **keeps the Latin -t-** (*parlato*) that
  French wore away (*parlé*), both from *-ātum*: the conservative sister again,
  matching Ch.11's *acqua* against *eau*. Same buried possessive as French — Latin
  *habeō litterās scriptās*, where the participle was an **adjective** agreeing with
  the object, a construction that **hardened into a tense**; the agreement survives
  when the object precedes (*le ho vist**e***).
- **passato remoto** (`IT-C15-passato-remoto`): *parlò* ← Vulgar Latin **\*parabolāvit**, with
  the final written stress the numbers chapter already introduced. Italian's
  distinctive fact is that this tense's survival is **geographic, not stylistic** —
  everyday speech in **Sicily and much of the south**, both-with-a-distinction in
  Tuscany, and literary in the **north**. So the "correct" past tense changes as you
  travel, and Italian is the language caught **mid-process**: French lost the
  inherited past from speech entirely, Spanish and Portuguese never gave it up, and
  Italian still holds it in half the country.
- Taxonomy: namespaced `IT-PAST-COMPOUND`, `IT-PAST-SIMPLE-REGIONAL`.

## Chapter 14 — avere, and having your years

- **Chapter 14 authored** (`IT-C14-avere`, `-eta`): the workhorse verb and the
  language's only silent letter, reviewing Ch.5/9/10/11/12/13 via `reviews_of`.
- **avere** (`IT-C14-avere`): *ho/hai/ha/abbiamo/avete/hanno* ← *habēre*, the same
  source as French *avoir*, and English's *habit/inhabit/exhibit/prohibit*. The
  chapter's real subject is the **silent h**: Italian discarded the Latin *h*
  almost everywhere (*homō* → *uomo*, *herba* → *erba*), but kept it in exactly
  these four forms because without it they collide with **o** ("or"), **ai** ("to
  the"), **a** ("to") and **anno** ("**year**"). The letter is never pronounced and
  survives **only so the eye can tell the words apart** — spelling doing a job
  sound cannot. Also notes *abbiamo*'s **bb** as the old *habē-* resurfacing while
  *ho* wore down to one vowel.
- **ho venti anni** (`IT-C14-eta`): age via *avere*, never *essere*; *anno* ←
  *annus* → *annual/anniversary*, with a genuinely held **double n**. The silent
  *h* then pays off inside this very chapter — ***hanno*** ("they have") and
  ***anno*** ("year") are **homophones** that co-occur in age sentences, which is
  exactly why the letter was worth keeping. Closes on the five-language table:
  **Romance has its years; Germanic is its years.**
- Sets up the *passato prossimo*, which is built on *avere*.
- Taxonomy: namespaced `IT-VERB-HAVE`, `IT-AGE`.

## Chapter 13 — Colours

- **Chapter 13 authored** (`IT-C13-nero-bianco`, `-rosso-blu`): two colours from two
  different peoples, reviewing Ch.11/12 via `reviews_of`.
- **nero & bianco** (`IT-C13-nero-bianco`): *nero* ← Latin *niger* is Rome's own word,
  barely changed. **Bianco** is not: it comes from Germanic ***blank*** ("shining"),
  most likely carried in by the **Lombards**, the Germanic people who ruled the north
  for two centuries and left their name on **Lombardia**. The loan won so completely
  that Latin *albus* was pushed out of the colour slot, surviving as **alba**
  ("dawn"), **albume** ("egg white"), and in place names.
- **rosso & blu** (`IT-C13-rosso-blu`): *rosso* ← *russus* ← PIE ***h₁rewdʰ-***,
  a **cousin** of *red/rot/rouge* rather than a borrowing (with a note on holding the
  **double s** — *roso* ≠ *rosso*). *Blu* ← Germanic *blāo* confirms the chapter
  pattern: Italian's **white and blue are both Germanic imports**. Then **azzurro** ←
  Arabic ***lāzaward*** ("lapis lazuli") ← Persian *lāžward*, the initial *l-*
  swallowed as if it were an article — the same journey that gave Spanish/Portuguese
  *azul*, French *azur*, English **azure**. Payoff: **gli Azzurri** are named, at the
  end of a long chain, after a blue stone mined in Afghanistan.
- Taxonomy: namespaced `IT-COLOUR-BLACK-WHITE`, `IT-COLOUR-RED-BLUE`.

## Chapter 12 — Numbers 11–20

- **Chapter 12 authored** (`IT-C12-numeri-11-16`, `-17-20`): the teens, atom-first,
  reviewing Ch.6/Ch.11 via `reviews_of`.
- **undici–sedici** — Italian keeps the Latin fusions **most legibly**: the shared
  **-dici** is still visibly *decem* ("ten"), the very word the learner says as
  **dieci**. Set against the sisters, the clarity is the point — Latin *sēdecim* →
  Italian **sedici** (ten audible) vs French **seize** (worn to *-ze*) vs Portuguese
  **dezesseis** (rebuilt entirely).
- **diciassette–venti** — the **reversal**: at 17 Italian turns the count around,
  *se-dici* ("six-ten") becoming *dici-assette* ("**ten**-and-seven"), the ten
  jumping to the front; the linking sounds (*diciAssette*, *diciANnove*) are just
  Italian smoothing the joint. *venti* ← *vīgintī*.
- Includes the three-sister table of **where each breaks** — Portuguese 16, French
  and Italian 17 — one inherited Latin system, three different seams.
- Taxonomy: namespaced `IT-NUM-11-16`, `IT-NUM-17-20`.

## Chapter 11 — Food (bread, water, wine)

- **Chapter 11 authored** (`IT-C11-pane`, `-acqua-vino`): the everyday table trio,
  atom-first, reviewing Ch.10/Ch.1 via `reviews_of`.
- **pane** ("bread") — **closest to Latin** *pānis*; the **companion** payoff
  (*com-* + *pānis*, "one you share bread with"), plus the purely Italian
  **companatico** — "whatever you eat **with** bread."
- **acqua / vino** — **acqua** ("water") **kept** Latin *aqua* almost whole (even
  doubling *-cq-*), a sharp contrast with French *eau* worn to a single vowel;
  **vino** ← *vīnum* → *wine/vine/vinegar*.
- Taxonomy: namespaced `IT-FOOD-BREAD`, `IT-FOOD-DRINKS`.

## Chapter 10 — Family

- **Chapter 10 authored** (`IT-C10-genitori`, `-fratello-sorella`): the immediate
  family, atom-first, reviewing Ch.9/Ch.1 via `reviews_of`.
- **padre / madre** — the sisters' **closest to Latin** *pater / māter* (only the
  *-t-* softened to *-d-*); *padre* is the very word English borrowed for a priest.
  **i genitori** ("parents") ← *genitor* "**begetter**" (*gignere* "beget") →
  genesis/gene/progenitor; with the **false-friend** warning that *parenti* means
  **relatives**, not "parents."
- **fratello / sorella** — Italian rebuilt "brother/sister" with its **diminutive**
  *-ello / -ella* ("little brother/sister"), keeping the *frat- / soror-* roots
  (→ fraternal, sorority).
- Taxonomy: namespaced `IT-FAMILY-PARENTS`, `IT-FAMILY-SIBLINGS`.

## Chapter 9 — Months & seasons

- **Chapter 9 authored** (`IT-C09-mesi`, `-stagioni`): the calendar year, atom-first,
  reviewing Ch.6–8 via `reviews_of`, with Spanish twins supplied.
- **The months, closest to Latin** of the sisters (*gennaio* keeps *Januarius*'s
  *-aio*; *ottobre* echoes *otto*): the god/emperor parade (Janus, Mars, Maia, Juno,
  Julius, Augustus), with the payoffs — *marzo* is the **same Mars** as *martedì*,
  and *settembre–dicembre* are the Latin **7–10** (Roman year began in March).
- **The seasons**: *primavera* = *prima vera*, "**first spring / first green**"
  (from Latin *vēr*); *estate* ← *aestas*; *autunno* ← *autumnus*; *inverno* ←
  *hibernum*. Even *stagione* is Latin *statiō*, "a standing" — a *station* of the
  year.
- Taxonomy: namespaced `IT-MONTHS`, `IT-SEASONS`.

## Chapter 8 — Time & the clock

- **Chapter 8 authored** (`IT-C08-ora`, `-mezzogiorno-mezzanotte`): telling the
  time, atom-first, reviewing Ch.6–7 via `reviews_of`.
- **ora** ← Latin *hōra* ← Greek *hṓrā* — the **closest of the sisters to Latin**
  (French wore it to *heure*; Italian barely touched *hōra → ora*). The Italian
  twist: time is told with the **feminine article**, the word *ore* left implied —
  *è l'una* (one, singular) but *sono le due* ("they are the two [hours]").
- **mezzogiorno / mezzanotte** — noon/midnight = *mezzo/mezza* ("half/middle," ←
  *medius*, cousin of French *mi-*) + *giorno* (← *diurnum*, root of *journal/
  journey*) / *notte* (← *noctem*). *mezza*notte is feminine (for *notte*),
  *mezzo*giorno masculine (for *giorno*) — the gender system again.
- Taxonomy: namespaced `IT-TIME-HOUR`, `IT-TIME-NOON-MIDNIGHT`.

## Chapter 7 — Days of the week

- **Chapter 7 authored** (`IT-C07-giorni-1`, `-giorni-2`): the seven days,
  atom-first, reviewing Ch.6 via `reviews_of`, with Spanish/French twins supplied.
- **giorni-1** (lunedì–venerdì): the **planet-week** with the accented **-dì**
  (← *diēs* "day") kept audible and stressed (*lu-ne-DÌ*). Three-sister lines make
  the shared Latin visible — *lunedì / lunes / lundi* are one word, *lūnae diēs*,
  worn three ways (IT/FR keep the day-word at the end, Spanish dropped it);
  *giovedì* wears *Giove* (Jupiter), the king-god English honours as Thor.
- **giorni-2** (sabato, domenica): the religious weekend — *sabato* ← *Sabbatum*
  (Hebrew *shabbāt*), the Sabbath every Romance language kept; *domenica* ← *(diēs)
  Dominica* "the **Lord's** day" (*Dominus* → dominion/dame), feminine *la domenica*.
- Taxonomy: namespaced `IT-DAYS-WEEKDAYS`, `IT-DAYS-WEEKEND`.

## Chapter 6 — Numbers 1–10

- **Chapter 6 authored** (`IT-C06-numeri-1-5`, `-numeri-6-10`): counting to ten,
  atom-first, each ~4 min, reviewing Ch.5 via `reviews_of`; Spanish and French
  twins supplied for each.
- **Italian kept the numbers closest to Latin** — it stayed next to Rome and wore
  them down least: *cinque* keeps Latin *quīnque*'s *-que* whole (vs Spanish *cinco*
  / French *cinq*), and Latin's *-pt-*/*-ct-* clusters **assimilate** to a doubled
  *-tt-* rather than dropping (*septem → sette*, *octō → otto*), so *otto* and
  *ottobre* still show the 8 side by side.
- **6–10** (*sei/sette/otto/nove/dieci*) carry the **settembre–dicembre = Latin
  7–10** calendar trick (the Roman year began in March; *luglio/agosto* displaced
  the counting months).
- Taxonomy: namespaced `IT-NUM-1-5`, `IT-NUM-6-10`.

## Chapter 5 — The first verbs (sentences start to move)

- **Chapter 5 authored** (`IT-C05-parlare`, `-abitare`, `-lavorare`,
  `-parlo-italiano`, `-practice`): Italian's first **grammar-engine** chapter,
  parallel to Spanish Ch.6 / French Ch.5 / German Ch.5. The learner stops reciting
  phrases and starts **building sentences from a pattern**.
- **The regular -are present tense** — drop *-are*, add *-o/-i/-a/-iamo/-ate/-ano*.
  Taught on **parlare**, cemented on **abitare** and **lavorare**. Italian is
  **pro-drop** (drops *io*, like Spanish).
- **The pronoun-rule circle closed** across five languages: **drop** (Spanish
  *hablo*, Italian *parlo* — distinct endings) vs **keep** (French *je parle* —
  silent endings; German *ich lerne* — grammar needs a subject).
- **Etymology, with cross-language contrasts**: *parlare* ← *parabolāre* "tell
  parables" (= French *parler*; Spanish *hablar* is from *fabulārī* instead);
  *abitare* ← *habitāre* (twin of *habiter*); **the "work" split** — *lavorare* ←
  *labōrāre* "to labour" (→ labor/laboratory/elaborate) where Spanish *trabajar* /
  French *travailler* come from *tripalium*, "torture." First self-assembled
  sentence: **Parlo italiano** (*italiano* ← *Italia*, perhaps "land of calves").
- Taxonomy: namespaced `IT-VERB-PARLARE/ABITARE/LAVORARE`, `IT-WORD-ITALIANO`.

## Chapter 3 — Introducing Yourself

- **Chapter 3 authored** (`IT-C03-io`, `-mi-chiamo`, `-come-ti-chiami`,
  `-piacere`, `-practice`): fills the gap between the greetings/how-are-you
  chapters and the farewells, so Italian now runs greet → introduce →
  how-are-you → goodbye end to end. Each lesson reviews Chapter 2.
- **io** (← *ego*) introduces the **pro-drop** habit — Italian usually omits the
  subject pronoun because the verb ending already carries it (shared with
  Spanish *yo* / Portuguese *eu*).
- **mi chiamo** — "I call myself" (*chiamarsi* ← Latin *clāmāre* "to call out" →
  claim/exclaim/clamor), completing the Romance naming-verb set: Spanish *me
  llamo* (*cl-*→*ll-*), Italian *mi chiamo* (*ch* = hard *k*), Portuguese *me
  chamo* (*ch* = *sh*).
- **Come ti chiami? / Come si chiama?** — the name asked with *come* ("how"), tu
  vs Lei; **piacere** ← *placēre* "to please" (please/pleasure/placid; twin of
  Portuguese *prazer*).
- Uses canonical `PRONOUN-I`, `INTRO-MY-NAME-IS`, `INTRO-WHATS-YOUR-NAME`,
  `INTRO-NICE-TO-MEET-YOU` — no taxonomy change.

## Chapter 4 — Farewells

- **Chapter 4 authored** (`IT-C04-arrivederci`, `-a-domani`, `-a-presto`,
  `-a-piu-tardi`, `-practice`): closing a conversation, reviewing Chapter 2. The
  learner can now run an Italian exchange end to end. Numbered Ch. 4 so the
  introductions chapter can slot in at Ch. 3.
- **The "until re-seeing" family**: *arrivederci* = *a + ri(re-) + vedere 'see'
  + ci 'us'* → "to our seeing-again" — shown beside French *au revoir* and German
  *auf Wiedersehen*, all the same gesture. The heavier *addio* (*a Dio*, "to
  God") is flagged as the twin of Spanish *adiós*.
- **"See you when" set**, each an atom traced: *a domani* (*domani* ← *dē māne*
  "from the morning", kin of Spanish *mañana*); *a presto* (*presto* ← *praestō*
  "at hand" → the English music/magic word); *a più tardi* (*più* ~ *plūs*/plus,
  *tardi* ~ *tardus*/tardy).
- Uses the canonical `FAREWELL`, `FAREWELL-TOMORROW`, `FAREWELL-SOON`,
  `FAREWELL-LATER` concepts (shared with the Spanish/French/German farewells) —
  no taxonomy change.

## Chapter 2 — "Come stai?" (the how-are-you chapter)

- **Chapter 2 authored** (`IT-C02-prego`, `-come`, `-stare`, `-come-stai`,
  `-come-sta`, `-come-va`, `-cosi-cosi`, `-practice`): the "how are you?"
  exchange, atom-first, reviewing
  Chapter 1. Fourth track in the PR's cross-language how-are-you set, reusing the
  canonical concepts `STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`, `WORD-SOSO`.
  Register (tu/Lei) and the question word (come) are introduced inline, since the
  track had no separate introductions chapter yet.
- **Italian sits between the two metaphors**: it asks *Come stai?* on **stare**
  (← Latin *stāre* "to stand" — literally Spanish *estar* with the propping *e-*
  removed) **and** *Come va?* on **andare** ("to go") — so it bridges the
  Spanish "stand" and the French/German "go."
- **Etymology hooks**: *prego* ← *pregare* "to pray" (→ pray/precarious/deprecate),
  behaving like German *bitte*; *come* ← *quōmodo* (sibling of *cómo*/*comment*);
  *così così* ← *(ec)cum sīc* "thus" (the *[sic]* English still writes) — and
  English "so-so" is a loan translation of it.
- Taxonomy: namespaced `IT-VERB-STARE` documented.

## Chapter 1 — Greetings (track bootstrapped)

- New Italian track on the HL00 framework: one word per lesson, slug ids,
  gender-before-nouns, atom-first, derivations shown, LaTeX book (Latin Modern;
  CI auto-discovers `italian/book/`).
- Chapter 1 (`lessons/IT-C01-*`), atom-first, with Italian's closest-to-Latin
  flavour:
  - **ciao** ("hi/bye" ← *s-ciào*, "I am your slave" ← Latin *sclavus*; English
    *slave*, *Slav*) — the showpiece etymology.
  - **buono / buon** ("good" ← *bonus*; adjective agreement).
  - **il / la / lo** ("the"; grammatical gender ← *ille/illa*; the two masculine
    articles).
  - **giorno** ("day" ← *diurnum* ← *dies*; English *journal*, *journey*;
    plural by vowel-change).
  - **buongiorno** (assembled).
  - **sera / buonasera** ("evening" ← *serus*; feminine agreement).
  - **notte / buonanotte** ("night" ← *noctem*; the Latin *-ct-* → Italian
    *-tt-* rule vs. Spanish *-ch-* / French *-it-*).
  - **grazie** ("thanks" ← *gratia*, "grace"; English *grace*, *gratitude*).
  - **practice**.
- Grounds each word against English + Latin, with Spanish/French supplied for
  contrast (beginner-audience, no prior knowledge assumed). Book compiles clean
  with XeLaTeX.
