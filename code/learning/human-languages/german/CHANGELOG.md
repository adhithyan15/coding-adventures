# Changelog

## Writing now starts with the first word (#12282)

- Migrate the opening **Hallo** lesson to measurable schema-v2 knowledge.
- Add one model-visible trace, followed by a two-minute guided copy: no recall,
  no free composition, and no new word beyond the greeting already taught.
- Keep the pen block detachable so the voice-first core remains available.

## German chapters 1-16 regain their reading order (#12248)

- Add one global, spaced sequence to all 66 legacy lessons, recovered from the
  hand-authored book sections and closed against every prerequisite and review.
- Remove 66 missing-sequence findings plus 30 forward prerequisites and 36
  forward reviews that alphabetical filename fallback had fabricated. German's
  order-integrity backlog moves from 132 defects to zero.
- Keep genuine content-placement debt separate: nine apparent forward-language
  uses disappear with the real order, while 55 still require teaching or
  reseating work. No learner content is silently declared taught.

## Pre-A1 vocabulary tranche — fourteen everyday nouns, four chapters (2026-08-07)

The level gate (`src/level-gate.ts`) reports every track blocked on
**vocabulary**: 300 distinct headwords at or below pre-A1. This tranche
authors fourteen concrete nouns across four new chapters, the third such
tranche after Hindi, Arabic and Tamil, and confirms the same mechanism:
`vocabularyOf()` counts distinct `headword:` strings, so fourteen one-headword
lessons move German's pre-A1 vocabulary by exactly fourteen — **31 → 45**
distinct headwords at or below pre-A1 (against the 300 target, shortfall
269 → 255), and 77 → 91 distinct headwords track-wide. No bulk credit; measured,
not assumed.

| Lesson | Concept | Word |
|---|---|---|
| `GE-C28-kaffee` | `GE-FOOD-COFFEE` | der Kaffee |
| `GE-C28-tee` | `GE-FOOD-TEA` | der Tee |
| `GE-C28-milch` | `GE-FOOD-MILK` | die Milch |
| `GE-C29-freund` | `GE-PEOPLE-FRIEND` | der Freund |
| `GE-C29-freundin` | `GE-PEOPLE-FRIEND-FEMININE` | die Freundin |
| `GE-C29-familie` | `GE-FAMILY-WHOLE` | die Familie |
| `GE-C30-auge` | `GE-BODY-EYE` | das Auge |
| `GE-C30-ohr` | `GE-BODY-EAR` | das Ohr |
| `GE-C30-mund` | `GE-BODY-MOUTH` | der Mund |
| `GE-C30-nase` | `GE-BODY-NOSE` | die Nase |
| `GE-C31-arm` | `GE-BODY-ARM` | der Arm |
| `GE-C31-finger` | `GE-BODY-FINGER` | der Finger |
| `GE-C31-fuss` | `GE-BODY-FOOT` | der Fuß |
| `GE-C31-herz` | `GE-BODY-HEART` | das Herz |

**Atom-first, three genders taught with every noun.** Each lesson introduces
2–3 knowledge atoms (`GE-LEX-*`, usually `GE-SOUND-*` or `GE-GRAMMAR-*`, and
`GE-ETYMON-*`), at or under `maxNewAtomsPerLesson: 3`. Chapter 28 introduces 9
atoms, Chapter 29 introduces 9, Chapters 30 and 31 introduce 12 each — all at
or under `maxNewAtomsPerChapter: 12`. Every noun's article is taught with the
noun (der/die/das), and German's capitalize-every-noun rule and three-gender
system — already established in Chapter 11 — are reinforced rather than
re-explained.

**Chapter 28 — Coffee, Tea, and Milk** (`SPINE-POLITE-REQUEST-REPAIR`):
extends Chapter 19's `Wasser, bitte` pattern to two more drinks, then closes
on the native word. **Kaffee** is a loanword three hops deep — Arabic *qahwa*
→ Ottoman Turkish *kahve* → Italian *caffè* → German *Kaffee*. **Tee** is a
different loan by a different route — Hokkien Chinese *tê*, carried by Dutch
sea traders — and the lesson names the well-known *tea*/*chai* isogloss split
by name (Hindi, Russian and Turkish took the overland Chinese syllable
instead) without turning it into an uncited language count. **Milch** closes
the trio as the one native Germanic word, from PIE *\*h₂melg-*, "to milk" —
deliberately mirroring Chapter 11's Wasser (native) beside Wein (loan) shape,
now run a third time with two loans instead of one. The payoff lesson also
rescues Chapter 27's two never-revisited orphan atoms, `GE-LEX-SCHLIESSEN-10`
and `GE-ETYMON-SCHLIESSEN-11`.

**Chapter 29 — Friend and Family** (`SPINE-EXCHANGE-NAMES`): **Freund** and
English *friend* are the same inherited word, a frozen Proto-Germanic present
participle of "to love" (PIE *\*preyH-*) — the same root inside Chapter 2's
*freut mich*. **Freundin** teaches German's native feminine suffix *-in* as a
general, reusable rule (Lehrer/Lehrerin, Student/Studentin) and names its one
surviving English fossil, **vixen**. **Familie** is the chapter's one loan —
Latin *familia*, "household," related to *famulus*, "servant" — and closes by
naming that it is the group Chapter 10's Eltern and Geschwister already
belong to.

**Chapter 30 — Eyes, Ears, Mouth, Nose** (`SPINE-CHECK-WELLBEING`): extends
Chapter 17's *Kopf*/*Hand* body-part material with four more parts of the
face. **Auge**/*eye* and **Ohr**/*ear* both trace to confirmed PIE roots;
**Mund**/*mouth* is inherited but its root beyond Proto-Germanic is not agreed
upon, and the lesson says so rather than inventing an ancestor; **Nase**/nose
is cousin to Latin *nasus* (English *nasal*) by shared descent, not
borrowing — the same *rot*/*rouge* shape Chapter 13 already taught. The
payoff also rescues Chapter 26's disputed "sharp-eared" link between *hören*
and *Ohr*, never revisited since it was flagged.

**Chapter 31 — Arm, Finger, Foot, Heart** (`SPINE-CHECK-WELLBEING`): Chapter
17's *Hand* lesson printed a five-word comparison — Hand, Arm, Finger, Fuß,
Herz — and taught only the first. This chapter teaches the other four.
**Arm** sits entirely outside Grimm's law's reach (its consonants were never
in the law's path), which is *why* it looks nearly identical to English *arm*
where *Vater*/*father* do not. **Finger** is identical to its English cousin,
with a proposed but explicitly unproven link to *fünf* ("five"). **Fuß** is a
second *p → f* Grimm's-law case beside *Vater*/*father*, and its **ß**
follows Chapter 13's own long-vowel rule. **Herz** closes the chapter — and
the whole five-word list — with a third instance of Grimm's law's *k → h*
swap, alongside *hören*/*akoúein* (Chapter 26) and *Hund*/*canis* (Chapter
22).

**Reach-back at two cadences (HL09 §7).** Every lesson names atoms from the
one to three lessons immediately before it. Each chapter's payoff also
reaches back several chapters: Chapter 28 to Chapter 19 and Chapter 27;
Chapter 29 to Chapter 2 and Chapter 10; Chapter 30 to Chapters 17 and 26;
Chapter 31 to Chapters 10, 13, 17, 22 and 26. Chapter 31's payoff closes over
all twelve of its own chapter's atoms (1.00 representativeness).

**No forward references.** Where a new word needed an example sentence, every
lesson uses the case-safe `Das ist der/die/das ___.` construction (predicate
nominative, no accusative article) rather than risk an untaught case form on
the mostly-masculine new nouns — Chapter 27's own note that "this track has
not taught cases" still holds. Drink requests reuse Chapter 19's `___, bitte.`
pattern rather than reach for the untaught verb `trinken`.

**Font check.** One lesson draft used Cyrillic (`Tee`'s *чай*) and one used
the unmapped PIE palatovelar diacritic `ǵ` (`Milch`'s root); both were caught
by a forced XeLaTeX compile ("Missing character: there is no ч in font
Latin Modern Roman…") and fixed before commit — Cyrillic dropped in favor of
the transliteration already given in prose, `ǵ` flattened to `g`. One
lesson (`GE-C31-herz`) originally tripped the sight-cue scanner on the literal
phrase "the table"; reworded to "list" throughout, restoring the chapter to
fully `voice`/drivable.

**Verification.** A forced XeLaTeX build of the 166-page book has zero
missing characters, zero overfull/underfull boxes, and zero duplicate labels
from this tranche (the corpus's one pre-existing underfull box, in Chapter
17, predates it). All four new chapters generate as `voice`, drivable
end-to-end. `npx vitest run tests/integration.test.ts tests/cli.test.ts`
passes (19/19); `check:modality`, `check:books` and `check:narration` all
pass with no diff beyond the new chapters. The six corpus-wide pinned-number
tests (`chapters`, `continuity`, `levels`, `modality-manifest`, `narration`,
`ramp`) shift with any authored content and are left failing, per standing
instruction — their numbers are reported here, not re-pinned.

**Wiring**: `GE-PATH-031`–`GE-PATH-034` are four new path segments (one on
`SPINE-POLITE-REQUEST-REPAIR`, one on `SPINE-EXCHANGE-NAMES`, two on
`SPINE-CHECK-WELLBEING`), each with a matching `GE-EXT-0{31..34}-LANGUAGE-SPECIFIC`
extension — both steps are required, since `lessonSpineNodes` only walks
`curriculum.path[].lessons`.

## Eight more core verbs, in two more chapters (2026-08-07)

Chapters 26 and 27 realize the eight core-verb concepts that **no track in the
corpus had realized anywhere**. German goes **14/40 → 22/40** on the taxonomy's
core verbs, and the count of core verbs unrealized in every track drops from
**15 to 7**.

| Lesson | Concept | Word |
|---|---|---|
| `GE-C26-sitzen` | `VERB-SIT` | sitzen |
| `GE-C26-stehen` | `VERB-STAND` | stehen |
| `GE-C26-schlafen` | `VERB-SLEEP` | schlafen |
| `GE-C26-hoeren` | `VERB-HEAR` | hören |
| `GE-C27-gehen` | `VERB-WALK` | gehen |
| `GE-C27-laufen` | `VERB-RUN` | laufen, rennen |
| `GE-C27-oeffnen` | `VERB-OPEN` | öffnen, aufmachen |
| `GE-C27-schliessen` | `VERB-CLOSE` | schließen, zumachen |

**Two chapters, not one**, on the Chapter 24/25 precedent: ten new atoms each,
against `maxNewAtomsPerChapter: 12`, each with its own capability and its own
payoff closing over all ten of its own atoms.

**This is the set where the blood relationship is the lesson.** Every one of
these eight is a cousin rather than a loan, and each shows a *different* and
*teachable* correspondence, so the chapter can say **why** the words look alike:

- *sitzen* **is** *sit* — Proto-Germanic \**sitjaną*, Old High German *sizzen*.
  The second (High German) shift's **t**-branch, which the track had never
  named: Germanic *t* → German *z*/*ss*. It retro-explains *Wasser* (ch. 11)
  and *zehn* (ch. 6), taught long before there was a name for what had happened
  to them. Chapter 25 had already given the **p**-branch on *helfen*.
- *stehen* / *stand* — Germanic ran two stems, \**stāną* and \**standaną*.
  English generalized the one with the nasal, German the one without; German's
  **past** (*ich stand*, *gestanden*) hands the *n* back. PIE \**steh₂-* is the
  root chapter 24 already named inside *verstehen*, and Latin *stō*/*stāre*
  descends from it independently — as Latin *sedeō* does from *sitzen*'s
  \**sed-*. Two roots, two families, no borrowing in either direction.
- *schlafen* / *sleep* — two German changes stacked: the second shift's
  **p** → **f**, plus *s* → *sch* before *l*, *m*, *n*, *w* (*schwimmen*,
  *Schnee*, *Schmied*), which German also did before *p* and *t* and never
  spelt — which is why *stehen* is said *SHTAY-en*. And the honest twist: the
  inherited Indo-European verb for sleeping was \**swep-* (Latin *somnus*,
  Greek *hýpnos*, Old Norse *sofa*). German and English replaced it
  **together**, on \**sleb-* "be slack", before they were two languages.
- *hören* / *hear* — the one change they **share**: Gothic *hausjan* keeps the
  *s* that both Old English *hīeran* and Old High German *hōren* had already
  turned to *r*. The initial *h* is Grimm's law on an old *k*, still audible in
  Greek *akoúein* → English **acoustic**, and the same swap that gave *Hund*
  against Latin *canis*. The "sharp-eared" analysis of the root is reported as
  widely cited and unsettled, not as fact.
- *laufen* / *leap* — a fourth **p**/**f** pair, after *helfen*, *offen* and
  *schlafen*; *lope*, *elope* and *interloper* are the English scraps. Outside
  Germanic there are no secure cousins, and the lesson says so.
- *offen* / *open* — the pair chapter 25 *listed* when it named the second
  shift, now taught. Neither word is a root: both are built on **up**, which is
  why *offen* and *auf* are relatives, and why *aufmachen* is the same recipe
  spoken out loud.
- *schließen* has **no English cousin at all**. Germanic \**sleutaną* survives
  in German and Dutch and left nothing in English, which closes things with
  Latin's *close* and with *shut* — native, but really *shoot*, a bolt shot
  across a door. That is the same story as chapter 25's *nehmen*, and the two
  are named together.
- *gehen* / *go* is the one verb that **cannot** be followed past Germanic; the
  root is disputed and no agreed cousin list exists. Both languages had to
  borrow a past: English took *went* from *wend*, German took *ging* and
  *gegangen* from \**ganganą* — which English still owns in *gangway*.

**The walk/run boundary, taught honestly.** English cuts between *walk* and
*run*; German does not cut in the same place. *gehen* is unhurried, *rennen* is
flat out, and *laufen* lies across the line, so **Wir laufen** is "we're
walking" or "we're running" and only the situation decides. That is true across
most of Germany and **not** in Austria, where *laufen* means *run* — recorded
in the lesson as a regional split, per HL09 §8.1, rather than dropped.

**Separable verbs, introduced with nothing new.** The track had never taught
them. HL09 §5.2 allows a new structural move only on vocabulary the reader
already holds, and both halves were held: *auf* from chapter 4's *auf
Wiedersehen*, *machen* from chapter 5. So `GE-C27-oeffnen` introduces the split
— **Ich mache die Hand auf** — and `GE-C27-schliessen` immediately re-uses it
for *zumachen*. The object is *die Hand* throughout, because feminine and
neuter accusative articles are identical to the nominative and this track has
not taught cases.

**Reach-back at two cadences (HL09 §7).** Every one of the eight names atoms
from the one to three lessons immediately before it, across the chapter seam,
and both payoffs reach several chapters back. Six atoms that no lesson had ever
revisited are revisited here — `GE-SOUND-HAND-03`, `GE-ETYMON-HUND-03`,
`GE-LEX-REGNET-05`, `GE-LEX-MOEGEN-LIEBEN-09`, `GE-ETYMON-MOEGEN-LIEBEN-10`,
`GE-GRAMMAR-GERN-11`. The track's never-revisited share falls from **31 of 61
atoms (51%) to 27 of 81 (33%)**. The two atoms of the final lesson are orphans
because nothing follows them yet.

All eight lessons are `voice` — drivable, no tables, no sight cues. Book: 140
pages, zero missing characters, zero overfull boxes under XeLaTeX.

## Eight core verbs, in two chapters (2026-08-07)

Chapters 24 and 25 add the eight verbs Spanish, Latin and Portuguese landed
last, so each of them turns a three-way cross-language join into a four-way one.
German goes **6/40 → 14/40** on the taxonomy's core verbs.

| Lesson | Concept | Word |
|---|---|---|
| `GE-C24-denken` | `VERB-THINK` | denken |
| `GE-C24-verstehen` | `VERB-UNDERSTAND` | verstehen |
| `GE-C24-lesen` | `VERB-READ` | lesen |
| `GE-C24-schreiben` | `VERB-WRITE` | schreiben |
| `GE-C25-nehmen` | `VERB-TAKE` | nehmen |
| `GE-C25-fragen` | `VERB-ASK` | fragen |
| `GE-C25-helfen` | `VERB-HELP` | helfen |
| `GE-C25-moegen-lieben` | `VERB-LIKE-LOVE` | mögen, lieben |

**Two chapters, not one.** Eight one-verb lessons introduce twenty atoms
against `maxNewAtomsPerChapter: 12`. Splitting is the resolution, not raising
the budget: chapter 24 introduces 10, chapter 25 introduces 10, and each has
its own capability and its own payoff. Page count is never a cost.

**What only this track can do.** These are English's *blood* relatives, not
loans, so the cousin webs are the point:

- *denken* **is** *think* — Proto-Germanic \**þankijaną*, with Grimm's law
  turning PIE \**t* into the *th* English kept and German softened to *d*. It
  is also the verb chapter 3 promised inside **danke**, finally given its own
  forms.
- *verstehen* is *ver-* + *stehen*, "to stand around" — and English built
  *understand* out of the same inherited verb and its own prefix, separately.
  The "under = among" account is given as **standard but not certain**.
- *lesen* first meant "to gather" (*die Weinlese*) — and the resemblance to
  Latin *legere*, which walked the same road from "gather" to "read", is
  named as **probably not** a shared word: the sounds do not correspond, and
  the standard account ties *lesen* to a separate root. English *read* is a
  third story entirely, Old English *rædan*, "to advise" — German *raten*.
- *schreiben* is the one **borrowing**: Latin *scrībere*, taken in early
  enough to pass through German sound changes (*sc-* → *sch-*) and join the
  native strong verbs. English refused the loan and kept *write*, also "to
  scratch". *Manuskript* then closes chapter 17's circle — the Latin hand-word
  German never inherited, bolted to the Latin writing-verb it did.
- *nehmen* is the verb **English threw away**; *numb* ("taken" by cold) and
  *nimble* are its fossils, and Greek's form of the root gives *nomad*,
  *economy* and *nemesis*.
- *fragen* is PIE \**preḱ-*; English lost the native cousin and gets the root
  back only through Latin *precārī* — *pray*, *precarious*. Latin *rogāre* is
  flagged as **not** related.
- *helfen* **is** *help*, split by the **second** (High German) shift, not by
  Grimm's law — *Schiff*/ship, *offen*/open, *scharf*/sharp — and English's own
  *holp*/*holpen* show *help* was once strong too. Outside Germanic the verb
  has **no secure cousins**, and the lesson says so rather than inventing one.
- *mögen* is *may*, *lieben* is *love*, and **gern** is *yearn*.

**Two grammar payoffs German alone can give this set.** Strong-verb vowel
change is introduced on *lesen* (`GE-GRAMMAR-STRONG-VOWEL-09`) and then
re-practised on *nehmen* (*du nimmst*, with the silent *h* dropping and the
*m* doubling) and *helfen* (*du hilfst*) — with *schreiben* and *fragen* as the
counter-examples that keep the rule exact. And *mögen* / *lieben* / **gern**
is German's three ways of liking, where *ich lese gern* ("I read gladly") has
no English shape at all.

**False friends flagged, not skipped**: *also* means "therefore", never
"also"; *bekommen* means "to receive", never "become".

**Reinforcement at two cadences (HL09 §7).** Every lesson names atoms from the
one to three lessons immediately before it — across the chapter seam — because
a chapter-end payoff cannot close the R1 window. On top of that, each payoff
reaches back much further: chapter 24's to `GE-LEX-HAND-02`,
`GE-ETYMON-HAND-MANUS-05` and `GE-SOUND-GRIMMS-LAW-04` (chapter 17), and
chapter 25's to all four chapter-24 verbs plus `GE-LEX-HUND-02`,
`GE-LEX-KATZE-04` and `GE-LEX-WETTER-02`. The reach-backs are real practice —
*Ich denke, es ist kalt*, *Die Hand schreibt*, *Ich mag die Katze*, *Der Hund
mag Wasser* — not name-checks.

**No forward references.** Nothing is used before it is taught, and no lesson
teases the next one. Where a construction the course has not reached would be
needed — the accusative article after *mögen*, the dative object of *helfen* —
the lesson says so and stays inside what the reader can already produce.

**Wiring**: `GE-PATH-027` and `GE-PATH-028` are two new `SPINE-SAY-WHAT-I-DO`
segments, and the eight concepts leave that node's `omits` ledger (36 → 28).
All eight lessons derive as `voice`, so both chapters are drivable end to end;
effective durations are 282–298 s against the 300 s ceiling.

## German joins the cross-language core verbs (2026-08-07)

- Retagged **six** verb lessons from language-local ids to the canonical
  concepts owned by `SPINE-SAY-WHAT-I-DO`, so German's verbs finally join the
  cross-language corpus instead of being seven private ids no other track can
  see: `GE-C16-sein` → `VERB-BE`, `GE-C14-haben` → `VERB-HAVE`,
  `GE-C03-gehen` → `VERB-GO`, `GE-C05-machen` → `VERB-DO-MAKE`,
  `GE-C05-lernen` → `VERB-LEARN`, `GE-C05-wohnen` → `VERB-LIVE`
  (*wohnen* is taught as "to live / to dwell", which is exactly `VERB-LIVE`).
  German's core-verb coverage goes **0/40 → 6/40**, and `VERB-DO-MAKE` and
  `VERB-LEARN` leave the corpus-wide `universallyMissing` list (29 → 27) —
  German is the first track anywhere to realize either.
- `GE-C02-heissen` keeps its namespaced `DE-VERB-HEISSEN`. No core concept
  means "to be called": *heißen* is not a translation of anything on the
  shared list, and forcing it onto one would be a false join.
- **Rewired `curriculum.json` so the realization path matches the retag.** A
  canonical concept obliges its lesson to sit in the segment of the node that
  owns it, so the four verb tranches moved into their own
  `SPINE-SAY-WHAT-I-DO` segments — `GE-PATH-011` (*gehen*), `GE-PATH-014`
  (*wohnen*, *machen*, *lernen*), `GE-PATH-018` (*haben*), `GE-PATH-021`
  (*sein*) — and left `SPINE-CHECK-WELLBEING` and `SPINE-TIME-OF-DAY`, where
  they had been sitting as language-specific extension material.
- **Teaching order is untouched.** No chapter was reordered and no lesson
  renumbered: each moved lesson holds the exact position it already had, and
  the segments were split around it rather than resequenced. *gehen* still
  lands immediately before *wie geht es*, which needs it.
- Three orphan lessons entered the path because the retag required it.
  `GE-C05-lernen` and `GE-C16-sein` realize canonical concepts and so cannot
  be absent from it; *sein* in turn declares `GE-C15-praeteritum` as a
  prerequisite (its *war/waren* forms are Präteritum), which pulled
  `GE-C15-perfekt` and `GE-C15-praeteritum` in behind it. Those two are a
  German-local past, not `VERB-PAST`, so they are recorded as a
  `SPINE-TALK-ABOUT-PAST` segment (`GE-PATH-020`) whose omission ledger still
  names `VERB-PAST` as undelivered. The node is now realized without the
  debt being quietly written off.
- Path segments were renumbered `GE-PATH-001..026` and extensions renamed to
  match their host segment, keeping the ids monotonic in path order as every
  other track has them. `GE-EXT-011-LANGUAGE-SPECIFIC` was deleted outright:
  it existed only to classify *gehen* as local support, and *gehen* is now
  shared content.
- Derived levels move accordingly: German reaches **A2** for the first time,
  corpus `A2` 91 → 99, `A1` 307 → 304, `pre-A1` 657 → 656, unmapped 170 → 166.

## Chapter capability ledger — Chapters 17–23 (2026-08-06)

- Added [`chapters.json`](./chapters.json), the track's HL05 capability ledger:
  a first-person `canDo`, the shared-spine nodes realised, and a validated
  payoff for each of the **seven** chapters that carry schema-v2 lessons.
- Deliberately authored **only Chapters 17–23**. Chapters 1–16 are still schema
  v1 and declare no `practises.knowledge`, so every payoff written for them
  would have to assess atoms that do not exist. They are omitted rather than
  stubbed: an absent entry is honest debt the gap report can count, a
  placeholder is destroyed signal.
- Every `payoff.assesses` list is a strict subset of the payoff lesson's own
  `practises.knowledge`; no atom is invented, and none is padded to clear a
  threshold.
- **Chapter 17 fails the 0.5 representativeness floor at 4/12 = 0.33.** It runs
  three word lessons deep — *Kopf*, *Kopf/Haupt*, *Hand* — with no terminal
  consolidation lesson, so the payoff can only be the last lesson by
  `sequence` and reaches just its own third of the chapter. The shortfall is
  recorded in the ledger rather than hidden; the fix is a real
  Kopf/Haupt/Hand practice lesson, not a longer `assesses` list.
- Chapter 18 also lacks a terminal practice lesson but still reaches 5/8 = 0.63
  because *nein* reassesses *ja*. Chapters 19–23 are single-lesson chapters and
  assess everything they introduce (1.00).
- Titles and labels are copied verbatim from `core/book-generation.json`, so the
  `chapter-title-drift` gate holds through the HL-C04 inversion.

## Warning-free 104-page book (2026-08-03)

- Made intentionally short micro-lesson pages explicit with `\raggedbottom`,
  removing eleven underfull vertical boxes without padding learner content.
- Added concise running titles and a prose-only Chapter 12 bookmark, made the
  Chapter 10 practice path breakable, and reflowed three dense explanations.
- Replaced rigid legacy comparison tables with bounded paragraph columns while
  preserving every vocabulary, grammar, register, and etymology comparison.
- Shortened only the visible `Entschuldigung` heading and reflowed the canonical
  `Kopf` recall; regenerated hashes keep the book and Language Ladder on the
  same source while the full explanations remain intact.
- A forced XeLaTeX build produces 104 pages with zero missing glyphs, overfull
  or underfull boxes, duplicate destinations, Hyperref warnings, or LaTeX
  warnings. All 104 rendered pages were inspected, and the outline retains the
  Preface, pronunciation reference, and all twenty-three chapters.

## Canonical Chapters 17–23 (2026-08-03)

- Migrated the ten lessons in Chapters 17–23 to schema version 2 with typed
  blocks, explicit shared-spine concepts, prerequisite-closed knowledge atoms,
  and honest sub-five-minute duration contracts.
- Repaired the missing shared-spine step between yes/no and sorry: a new
  164-second `bitte` lesson assembles only previously learned words into
  **Wasser, bitte**, while `Entschuldigung` moves from Chapter 19 to Chapter 20.
- Generated seven LaTeX chapters from those canonical lessons and added
  independent Language Ladder source-hash and lesson-count assertions, so the
  app and downloadable book now consume one source of truth through Chapter 23.
- Expanded the book from 84 to 104 pages. A forced XeLaTeX build has no missing
  glyphs, duplicate destinations, LaTeX warnings, or leaked generator metadata;
  all 104 rendered pages and the complete outline were inspected.
- Recorded eighteen overfull boxes, one underfull horizontal box, eleven
  underfull vertical boxes, and three Hyperref warnings for the focused HL-B21
  cleanup tranche.

## Sub-five-minute lesson remediation (2026-08-02)

- All twenty-seven German duration violations are resolved. Twenty-two lessons
  already computed below five minutes and now declare an honest four-minute
  budget without changing their teaching content.
- Five lessons that genuinely exceeded the limit become prerequisite-ordered
  micro-sequences: informal wellbeing → formal *Ihnen* register → separate
  casual/formal practice; Präteritum forms → its north/south areal map; the
  *sein*-perfect auxiliary family → French/German agreement; *Kopf* as cup →
  inherited *Haupt* and the Grimm's-law/container comparison.
- The five new support lessons bring the German track to 86 lessons. Every new
  or rewritten step computes between 147 and 244 seconds, with zero unknown
  prerequisite ids.
- A forced build still succeeds at 84 pages with no missing glyphs or duplicate
  labels. Its existing seventeen overfull boxes, eleven underfull boxes, and
  three Hyperref warnings are recorded separately in `HL-B21`; publishing the
  canonical Chapters 17–23 is recorded in `HL-B20`.

## The book catches up -- Chapters 3-16 typeset

The lessons had run ahead of the published artifact: 61 authored lessons through
Chapter 16, but the LaTeX book still stopped at Chapter 2 ("Introducing
Yourself"). Because the CI book build only compiles what is wired into
`book.tex`, the missing chapters were invisible to CI and the gap drifted
silently. This closes it -- **fourteen new book chapters**, written from the
existing `GE-C03`-`GE-C16` lessons and wired into `book.tex`:

- **Ch3** How Are You (danke, bitte, gehen, wie geht es, es geht)
- **Ch4** Farewells (auf Wiedersehen, tschuess, bis bald, bis morgen)
- **Ch5** The First Verbs (wohnen, machen, lernen, ich lerne Deutsch)
- **Ch6** Numbers One to Ten * **Ch7** The Days of the Week (and Mittwoch)
- **Ch8** Telling the Time * **Ch9** Months and Seasons (Herbst/harvest)
- **Ch10** Family * **Ch11** Bread, Water, Wine
- **Ch12** Numbers Eleven to Twenty (elf/zwoelf, the "-lif = left over" story)
- **Ch13** Colours * **Ch14** To Have, and How Old You Are (the habere false
  cognate)
- **Ch15** The Two Past Tenses (Perfekt, Praeteritum)
- **Ch16** To Be, and the Past That Takes It (sein -- three ancient verbs in one
  paradigm -- and the Perfekt built on it)

Each chapter follows the established book conventions: one `\section` per lesson
with a slug `\label`, the `cousinweb` / `culture` / `grammarlens` / `sounds` /
`etymology` / `morphologybox` boxes, `booktabs` conjugation tables, and every
atom traced to its root -- the German/English cognate webs are the spine.
Content is faithful to the lessons -- no new etymologies introduced.
Practice-section labels are chapter-qualified (`lesson:chN-practice`).

The book grows to **84 pages**; compiles clean with XeLaTeX (0 errors, 0 missing
characters, 0 undefined references, 0 duplicate labels) and was rasterized and
visually QA'd -- the umlauts, the eszett, `fui` with macron, and the PIE
superscripts all render correctly.

## Chapter 17 — The body: a cup for a head, and a hand with no Latin cousin

- **Chapter 17 authored** (`GE-C17-kopf`, `-kopf-haupt`, `-hand`) — the **body**, the theme the
  parallel-track roadmaps name next.
- **der Kopf** (`GE-C17-kopf`): *Kopf* did not originally mean "head." It meant a
  **cup or bowl** — the same word as English **cup**, both early borrowings of
  Late Latin ***cuppa*** — and it displaced the inherited **das Haupt**, the
  Grimm's-law cognate of Latin *caput* and English *head*. The clean
  demonstration there is **k→h** (*caput* / *Haupt* / *head*); the later
  consonants involve a second shift, so the lesson takes k→h and leaves the rest.
  *Haupt* survives in compounds: *Hauptstadt*, *Hauptbahnhof*, *Hauptsache*.
  - **The chapter's best fact is a coincidence.** French replaced "head" with a
    **pot** (*testa* → *tête*) and German with a **cup** (*cuppa* → *Kopf*), with
    nobody coordinating — and **both** kept the old word for chiefs and capitals.
    It is the **metaphor** that was invented twice, not the vocabulary: both
    vessel-words trace back to Latin. Heads look like bowls in any language.
  - *(Corrected here: #8746 fixed this formula in the lesson, roadmap and
    taxonomy but missed the CHANGELOG, which kept a wrong `p→f/d` and called
    \*kuppaz native Germanic. A claim lives in four places.)*
  - Includes the **-pf** note: one sound, *p* released into *f*, with no English
    equivalent.
- **die Hand** (`GE-C17-hand`): the easy word, kept deliberately for what it
  teaches about **absence of connection**. Germanic \**handuz*, inherited
  straight into English (*Hand, Arm, Finger, Fuß, Herz*), with the **final-devoicing**
  note — *Hand* ends in a *t* sound, and the *d* returns in *die Hände*.
  - **Every Romance track in this course builds "hand" on *manus*** (*main*,
    *mano*, *mão*), and \**handuz* **is not related to it**. The lesson says this
    outright, because a curriculum that keeps finding connections can start to
    imply everything connects. It doesn't — and this is where the two families
    diverged early and completely.
  - *Manus* did reach German, but only as **borrowed** learned vocabulary
    (*Manuskript*, *Maniküre*, *manuell*), sitting beside the native word without
    displacing it.

## Chapter 16 — *sein*: three ancient verbs wearing one infinitive

- **Chapter 16 authored** (`GE-C16-sein`, `-perfekt-sein`,
  `-perfekt-sein-agreement`). Ch. 15 taught only
  the *haben* half of the Perfekt because *sein* had never been taught. Fixed.
- **sein** (`GE-C16-sein`): the present, plus *war/waren*, and then the reason
  they look unrelated — they **are** unrelated. *sein* is assembled from **three
  Proto-Indo-European roots**:
  - *ist, sind, seid, sein* ← \**h₁es-* (Latin *est, sunt*; French *est, sont*)
  - *bin, bist* ← \**bʰuH-* "grow, become" (English **be**; Latin *fuī*)
  - *war, waren* ← \**wes-* "dwell, remain" (English **was, were**)
  - The cross-track payoff: \**bʰuH-* is the root of **Spanish *fui***, taught in
    ES-C14 as a *pretérito fuerte*. The **same** root surfaced in German's
    **present** and Spanish's **past**.
- The lesson also states the general law rather than leaving it as trivia: **the
  most-used words are the most irregular**, because regularity spreads by
  **analogy** and you never have to guess at "to be". Rare words get regularised;
  common ones are protected fossils. This is the answer to "why is *sein* like
  this" in every language at once.
- **Perfekt with sein** (`GE-C16-perfekt-sein`): the **motion / change-of-state**
  split, on *gehen* (Ch. 3), *kommen*, *fahren*, *werden*, plus the set that
  breaks the pattern and must simply be learned — ***sein*** and ***bleiben***,
  which are the *opposite* of change, along with *gelingen*, *geschehen* /
  *passieren* and *begegnen*.
  - **The contrast that matters: no agreement.** French makes the participle
    agree with the subject (*elle est allé**e***); **in the perfect** German
    makes it agree with **nothing** (*sie ist gegangen*, for every person and
    gender). Scoped deliberately to the perfect, because German participles **do**
    still inflect attributively (*der angekommen**e** Zug*, *ein geschrieben**er**
    Brief* — chosen over *der gegangene Weg*, which only licenses attributively
    via the marked transitive *einen Weg gehen* and reads stiff); Old High German
    inflected them in the perfect too, and German lost that.
  - **Corrected direction of influence.** German did **not** inherit this from
    Latin. The *haben*- and *sein*-perfects are native Germanic developments that
    grew up **alongside** the Romance ones through centuries of contact — the
    same areal spread this repo already credits (in `FR-PAST-SIMPLE-LITERARY`)
    for the simple past retreating in French, German and Italian together. Stated
    as *split parallel, agreement not shared*.

## Chapter 15 — The Perfekt, and the tense it pushed aside

- **Chapter 15 authored** (`GE-C15-perfekt`, `-praeteritum`,
  `-praeteritum-map`): the everyday past,
  built on Ch.14's *haben* — reviewing Ch.5/14 via `reviews_of`.
- **Perfekt** (`GE-C15-perfekt`): *haben* + past participle (*ich habe gesagt*),
  with two things German does that English can't. First, the weak participle is
  **wrapped** — a **ge-…-t circumfix**, not a suffix. Second, it goes to the **end
  of the clause** (*Ich habe gestern Deutsch **gelernt*** — "I have yesterday German
  learned"), which is simply ungrammatical in English. Plus the semantic note that
  it means the **plain past** ("I said"), not only "I have said." Etymology: **ge-**
  ← Germanic *\*ga-* "together, completely," a **perfective** marker — exactly what
  a past participle is for. English had it as *y-* and dropped it, leaving two
  fossils: **enough** (Old English *genōg*) and archaic *yclept*. So English once
  wrapped its participles the same way; German never stopped.
- **Präteritum** (`GE-C15-praeteritum`): *ich sagte* — the simple past, same
  meaning as the *Perfekt* but a different register. Its **-te** is the Germanic
  **dental preterite**, the identical machinery behind English **-ed** (*walked*)
  and Dutch *-te* — a **Germanic invention** with no Latin equivalent, since Romance
  builds its past from inherited perfect endings instead (*parla*, *habló*).
  Register and geography: nearly gone from speech in the south, better preserved in
  the north, standard in **narrative writing**, with *war*, *hatte* and the modals
  resisting everywhere. Closes on the three-language table — **German, French and
  Italian** each let a "have" compound displace their simple past — an AREAL change spread by contact, not three separate inventions.
- Taxonomy: namespaced `GE-PAST-COMPOUND`, `GE-PAST-SIMPLE-WRITTEN`.

## Chapter 14 — haben, and being your years

- **Chapter 14 authored** (`GE-C14-haben`, `-alter`): the workhorse verb plus the
  one everyday place German won't use it, reviewing Ch.5/9/10/12/13 via
  `reviews_of`.
- **haben** (`GE-C14-haben`): *habe/hast/hat/haben/habt/haben*, where *du hast*
  and *er hat* **drop the b** — precisely as English *have* → *ha**s*** (and
  archaic *hast*), one shortcut the two languages inherited together. The
  showpiece is a **false cognate**: *haben* ← Germanic *\*habjaną* ← PIE *\*kap-*
  "to **seize**," whose Latin child is ***capere*** (→ *capture, captive, capable,
  accept*) — while Latin ***habēre*** (which gave French *avoir* and Italian
  *avere*) descends from *\*gʰabʰ-*, whose English descendant is **give**. The two
  words that look most alike and mean the same thing come from **opposite**
  ancestries; German *haben* is kin to *capture*, Latin *habēre* to *give*.
- **ich bin zwanzig Jahre alt** (`GE-C14-alter`): the one everyday slot where
  German **refuses** *haben* — age takes **sein**, producing word-for-word the
  English sentence, and shortening the same way (*ich bin zwanzig*). *Jahr* ←
  *\*jēra* = **year**; *alt* ← *\*aldaz* = **old**, with the Latin cousin *alere*
  "to nourish, grow" behind English *adult*. Closes on the five-language table:
  **all four Romance sisters *have* their years; German sides with English and
  *is* its years** — and does so even though it borrowed its month names from
  Latin (Ch.9).
- Sets up the *Perfekt*, which is built on *haben*.
- Taxonomy: namespaced `GE-VERB-HAVE`, `GE-AGE`.

## Chapter 13 — Colours

- **Chapter 13 authored** (`GE-C13-schwarz-weiss`, `-rot-blau`): German as the
  **lender** rather than the borrower, reviewing Ch.11/12 via `reviews_of`.
- **schwarz & weiß** (`GE-C13-schwarz-weiss`): both **native Germanic**, no Latin
  anywhere. *Schwarz* ← *swartaz*, whose English cousin survives as **swarthy**, and
  which is kin to Latin *sordēs* ("dirt") → *sordid* — black and grubby from one
  idea. *Weiß* ← *hwītaz* = **exactly** English *white*; includes the **ß** rule
  (sharp *s* after a long vowel; Swiss spelling *weiss*). The showpiece: German's own
  **blank** ("shiny, polished, bare") is the very word Romance **borrowed** for
  **white** — *blanc/bianco/branco* — while German kept the original meaning. This
  reverses the direction seen in Ch.11 (*Wein* ← *vīnum*, *Fenster* ← *fenestra*).
- **rot & blau** (`GE-C13-rot-blau`): *rot* ← *raudaz* ← PIE ***h₁rewdʰ-***, so *rot*
  and French *rouge* are related **by descent, not borrowing** — they split millennia
  before either language existed. *Blau* ← *blēwaz* is the **second** German colour
  word Romance took (*bleu*, *blu*), and English took **blue from French** rather
  than from its own Germanic stock. Closes with a four-row table of which words
  Romance borrowed and which it already had a cousin for.
- Taxonomy: namespaced `GE-COLOUR-BLACK-WHITE`, `GE-COLOUR-RED-BLUE`.

## Chapter 12 — Numbers 11–20

- **Chapter 12 authored** (`GE-C12-elf-zwoelf`, `-zahlen-13-20`): the teens,
  atom-first, reviewing Ch.6/Ch.11 via `reviews_of`.
- **elf / zwölf** — the showpiece: ← *ainlif / twalif*, where **-lif** means "**to
  leave, remain**," so they literally say "**one left over**" and "**two left
  over**" — left over from your **ten fingers**. English *eleven/twelve* are not
  merely similar but **the same inherited words**, which is why both languages share
  the oddity. Extends the Germanic-twin thread from *Vater/father*, *Wasser/water*.
- **dreizehn–zwanzig** — then the pattern turns perfectly regular: **digit + zehn**,
  no exceptions, exactly mirroring English *-teen* (which **is** *ten*: *thir-teen* =
  "three-ten"). *Sechzehn/siebzehn* clip a sound just as English clipped
  *three→thir-*, *five→fif-*; *zwanzig* ← *twaintig* "two tens" (= English *-ty*).
- **The contrast made explicit**: the Romance sisters all **break** their teens
  pattern partway (PT at 16, FR/IT at 17); **German never breaks** — two leftovers,
  then one clean rule to twenty, with English marching alongside the whole way.
- Taxonomy: namespaced `GE-NUM-11-12`, `GE-NUM-13-20`.

## Chapter 11 — Food (bread, water, wine)

- **Chapter 11 authored** (`GE-C11-brot`, `-wasser-wein`): the everyday table
  trio, atom-first, reviewing Ch.10/Ch.1 via `reviews_of`.
- **Brot** ("bread") — **inherited Germanic**, the direct twin of English *bread*
  (NOT the Latin *pānis* the Romance sisters use); introduces the **neuter das**
  (completing *der/die/das*) and the rule that German **capitalizes all nouns**.
- **Wasser / Wein** — the native-vs-borrowed pair: **Wasser** ("water," *w*=*v*) is
  a native Germanic twin of *water*, but **Wein** ("wine," *ei*="eye") is an
  **ancient Latin loan** ← *vīnum*, taken with the grapevine Rome carried north —
  which is exactly why *Wein*, English *wine*, and *vīnum* all match (one loan, not
  three cousins).
- Taxonomy: namespaced `GE-FOOD-BREAD`, `GE-FOOD-DRINKS`.

## Chapter 10 — Family

- **Chapter 10 authored** (`GE-C10-eltern`, `-geschwister`): the immediate family,
  atom-first, reviewing Ch.9/Ch.1 via `reviews_of` — and the **mirror image of the
  months chapter**.
- **der Vater / die Mutter** — taught as **inherited Germanic** words (NOT Latin
  loans like the months), the Grimm's-law twins of English *father / mother*: the
  *V* of *Vater* is pronounced *f*, and German/English agree (*f-*, *m-*) precisely
  because both are Germanic, while French/Latin sit across Grimm's line. The
  standout thread: **family is native where the calendar was borrowed**.
- **der Bruder / die Schwester** — Germanic twins of *brother / sister*; plus
  **die Geschwister** ("siblings"), built with the **collective ge-** prefix that
  English lacks.
- Taxonomy: namespaced `GE-FAMILY-PARENTS`, `GE-FAMILY-SIBLINGS`.

## Chapter 9 — Months & seasons

- **Chapter 9 authored** (`GE-C09-monate`, `-jahreszeiten`): the calendar year,
  atom-first, reviewing Ch.6–8 via `reviews_of`.
- **The native-vs-Latin split deepens** (numbers native, weekday-gods Germanic,
  clock *Uhr* Latin — now): the **months are Latin loans** (Januar ← Janus, *März*
  ← Mars = *Dienstag*'s Tiw, September–Dezember = Latin 7–10), reaching for Rome
  just as *Uhr* did — while German's own numbers stay *sieben, acht, neun, zehn*.
- **The seasons swing back to native Germanic**: *Frühling* ← *früh* "early" (the
  early-season); *Sommer/Winter* = the plain twins of English *summer/winter*; and
  the surprise, **Herbst = English harvest** — the same Germanic reaping-word, which
  English narrowed to the *act* while taking Latin *autumn* for the season.
- Taxonomy: namespaced `GE-MONTHS`, `GE-SEASONS`.

## Chapter 8 — Time & the clock

- **Chapter 8 authored** (`GE-C08-uhr`, `-mittag-mitternacht`): telling the time,
  atom-first, reviewing Ch.6–7 via `reviews_of`.
- **Uhr** — the **standout Latin loanword**: German's numbers are native (*eins,
  zwei*) and its weekdays are Germanic gods (*Donnerstag*), but its *clock*-word
  came from Latin **hōra** (the same *hōra* behind French *heure*, Italian *ora*,
  English *hour*). Three layers of the day, three origins — native numbers,
  Germanic day-gods, Latin clock. (Native *Stunde* = an hour's span; *Uhr* =
  o'clock, no plural: *es ist zwei Uhr*.)
- **Mittag / Mitternacht** — noon/midnight swing back to **native** compounds:
  *Mitte* ("middle") + *Tag* ("day") / *Nacht* ("night," the *Nacht/night* twin
  from the numbers). Same meaning as French *midi/minuit* (*medius diēs*), but
  built from German's own words rather than borrowed.
- Taxonomy: namespaced `GE-TIME-HOUR`, `GE-TIME-NOON-MIDNIGHT`.

## Chapter 7 — Days of the week

- **Chapter 7 authored** (`GE-C07-wochentage-1`, `-wochentage-2`): the seven days,
  atom-first, reviewing Ch.6 via `reviews_of`.
- **wochentage-1** (Montag–Freitag): like the numbers, German goes **Germanic** —
  its weekdays are the **twins of the English days**, named for Germanic gods, not
  Latin planets. *Donnerstag* (*Donner* "thunder" = Donar/**Thor**) = *Thursday*,
  standing in for the Roman Jupiter; *Freitag* (Frigg) = *Friday*. The odd one,
  **Mittwoch** "mid-week," is a religious edit — the Church replaced "Woden's day"
  (which English kept as *Wednesday*), mirroring how Portuguese numbered its days.
- **wochentage-2** (Samstag, Sonntag): the surprise that **Samstag is the Sabbath,
  not Saturn** — *Sabbat* ← Greek *sábbaton* ← Hebrew *shabbāt*, reaching German
  through the early Church, so *Samstag* and Spanish *sábado* share a root while
  English alone keeps *Saturday* = Saturn; *Sonntag* = "Sun's day" = *Sunday* (a day
  the Church left un-renamed, unlike Romance *domingo/dimanche*).
- Taxonomy: namespaced `GE-DAYS-WEEKDAYS`, `GE-DAYS-WEEKEND`.

## Chapter 6 — Numbers 1–10

- **Chapter 6 authored** (`GE-C06-zahlen-1-5`, `-zahlen-6-10`): counting to ten,
  atom-first, each ~5 min, reviewing Ch.5 via `reviews_of`.
- **The distinctive German story**: unlike the Romance tracks, German numbers are
  **not Latin loans** — they're German's own Germanic words and the **near-twins of
  English one…ten**, with Latin as a *cousin* one sound-shift away. **Grimm's Law**
  is the through-line: old *p* → Germanic *f* (*\*pénkʷe* → *fünf/five*, while Latin
  kept *quīnque*); old *d* → *t* → German *z* (*decem → ten → zehn*); and the
  *acht/eight* ~ *Nacht/night* *-cht-/-ght-* correspondence.
- **The month names *are* Latin loans** (*September, Oktober…*), so the 7–10
  calendar trick still shows even though *sieben/acht/neun/zehn* look nothing like
  them — numbers homegrown, month labels imported.
- Taxonomy: namespaced `GE-NUM-1-5`, `GE-NUM-6-10`.

## Chapter 5 — The first verbs (sentences start to move)

- **Chapter 5 authored** (`GE-C05-wohnen`, `-machen`, `-lernen`,
  `-ich-lerne-deutsch`, `-practice`): German's first **grammar-engine** chapter,
  parallel to French Ch.5 / Spanish Ch.6. Uses **regular (weak) verbs only** —
  *sprechen* is irregular and deferred.
- **The regular weak present tense** — drop *-en*, add *-e/-st/-t/-en/-t/-en*.
  Taught on **wohnen** and cemented on **machen** and **lernen**. Unlike French,
  **German endings are audible** (*wohne/wohnst/wohnt* differ).
- **The pronoun rule completed across three languages**: Spanish **drops** *yo*
  (the ending says who); French **keeps** *je* (endings silent); German **keeps**
  *ich* — for yet another reason: its grammar needs an **overt subject**
  (structure, not sound).
- **Etymology, English-cousins-you-own**: *wohnen* ← *wonēn* (→ *wont*
  "accustomed"); *machen* ← *makōn* (= English **make**; the High German *k*→*ch*
  shift); *lernen* ← *liznōjan* (= **learn**; kin of *lore*); *Deutsch* ←
  *diutisc* "of the people" (→ English **Dutch**, **Teutonic**). First
  self-assembled sentence: **Ich lerne Deutsch**.
- Taxonomy: namespaced `GE-VERB-WOHNEN/MACHEN/LERNEN`, `GE-WORD-DEUTSCH`
  documented.

## Writing nuances — the eszett, the umlauts, capital nouns

- **First German `writing`-type lessons** (`GE-W01-eszett`, `GE-W02-umlauts`,
  `GE-W03-capitalization`): orthography taught etymology-first, once enough
  special-character words have accumulated (*heißen*, *weiß*, *Straße*).
- **ß (eszett)**: a long-*s* + *s/z* ligature (hence "es-zett"), always a sharp
  *s*; the rule **ß after long vowels, ss after short** (*Straße* vs *Fluss*) —
  which doubles as a vowel-length cue; no word-initial/lowercase-only quirks
  (ALL-CAPS → SS; Switzerland drops it).
- **Umlauts ä/ö/ü**: the two dots as a **shrunken migrated *e*** (ASCII fallback
  *ae/oe/ue*: *Müller = Mueller*); "um-laut" = around-sound (vowel fronting); and
  the grammar it marks — plural/comparative/diminutive fronting (*Mann→Männer*,
  *groß→größer*, *Hund→Hündchen*). Contrasted with the French tréma.
- **Großschreibung**: German capitalizes **every noun**, mid-sentence and all —
  a part-of-speech signal that disambiguates (*essen* "to eat" vs *das Essen*
  "the food"), a living fossil of older European printing (English dropped it
  ~1700s).
- Uses the `writing` lesson type (no `concept_tag`) — no taxonomy change.

## Chapter 4 — Farewells (completes the ES/FR/DE farewell trilogy)

- **Chapter 4 authored** (`GE-C04-auf-wiedersehen`, `-tschuss`, `-bis-bald`,
  `-bis-morgen`, `-practice`): closing a conversation, atom-first, reviewing
  Chapter 3. Reuses the shared `FAREWELL` / `FAREWELL-SOON` / `FAREWELL-TOMORROW`
  concepts and adds `FAREWELL-CASUAL`.
- **auf Wiedersehen** = "on the seeing-again" (*sehen* = English *see*) — the
  exact twin of French *au revoir*, both against Spanish *adiós* "to God".
- **tschüss**, the best etymology in the chapter: *tschüss* ← Low German
  *atschüs* ← Walloon *adjûs* ← French *adieu* — so the breeziest German bye is
  secretly **"to God"**, a far-travelled cousin of *adiós* and *adieu*.
- **The "bis …" family** mirrors Spanish *hasta* / French *à*: *bis bald* (soon —
  *bald* ← Old High German "bold/quick", = English *bold*), *bis später* (later),
  *bis morgen* (tomorrow — *Morgen* = English *morning/morrow*, the same
  morning→tomorrow move as *mañana* / *demain*).
- Taxonomy: `FAREWELL-CASUAL` added (canonical, `core:false`).

## Chapter 3 — "Wie geht's?" (completes the how-are-you trilogy)

- **Chapter 3 authored** (`GE-C03-danke`, `-bitte`, `-gehen`, `-wie-geht-es`,
  `-wie-geht-register`, `-es-geht`, `-practice`, `-formal-practice`): the
  "how are you?" exchange, atom-first, reviewing
  Chapter 2. Third of a deliberate cross-language trilogy in this PR (Spanish
  Ch.4 / French Ch.3 / German Ch.3), all sharing the canonical concepts
  `STATE-HOW-ARE-YOU`, `COURTESY-YOUREWELCOME`, `WORD-SOSO`.
- **The etymologies English speakers already own**:
  - *danke* ← *denken* "to think" — and English *thank* IS *think* (both from
    Old English *þancian*/*þencan*), set against *merci* (reward) and *gracias*
    (grace).
  - *bitte* ← *bitten* "to ask/pray" — cognate of English *bid* and *bead* (a
    bead was a prayer); the one word doing please / you're-welcome / here-you-go
    / pardon.
  - *gehen* IS English *go* (straight Germanic cognate); *es geht mir gut* = "it
    goes well *to me*" — gently introduces the **dative** (*mir/dir/Ihnen*).
  - *es geht* ("it goes," nothing added) as the understated shrug for "so-so."
- **The trilogy's payoff**, stated in-lesson: German and French say wellbeing as
  motion ("how does it **go**?"), Spanish as posture ("how are you
  **standing**?" — *estar*).
- Taxonomy: namespaced `DE-VERB-GEHEN` documented.

## Chapter 2 — Introducing Yourself

- New chapter built around the introduction dialogue (*Ich heiße Susanne. / Wie
  heißen Sie? / Ich heiße David. / Freut mich.*), atom-first, one word per
  lesson (`lessons/GE-C02-*`, `book/chapters/ch02-introductions.tex`):
  - **ich** ("I" ← *\*ik* / PIE *\*eǵ*; cousin of Latin *ego*, English *I*).
  - **heißen** ("to be called" ← *\*haitaną*; English archaic *hight*, *behest*)
    — German names with a plain verb, no reflexive "myself."
  - **ich heiße…** — **"my name is…"** ("I am called"), with literal *mein Name
    ist* (*Name* ← *\*namô*, English *name* / Latin *nōmen*) as the alternative.
  - **du / Sie** (familiar / formal "you") — *Sie* is the capitalized 3rd-person
    plural "they" used as polite "you"; the third route to politeness beside
    Spanish *usted* and French *vous*.
  - **wie** ("how" ← *\*hwī* / PIE *\*kʷo-*; English *how/what/who*).
  - **wie heißen Sie?** — **"what's your name?"** ("how are you called?");
    verb-second word order; informal *wie heißt du?*.
  - **freut mich** ("pleased to meet you" = "it gladdens me"; ← *froh*, "glad").
    Its object pronoun **mich** ("me") is traced too — ← *\*mek* / PIE *\*me-*,
    cousin of English *me/my/mine* and French *me* (every atom rooted, not
    glossed).
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

## Beginner-audience + parity pass

Brought the German book fully to the Hindi/Spanish standard. Two things:

**Stop assuming prior Spanish/French (HL00 Audience rule).** The books are for a
true beginner whose only shared language is English; German leaned on the other
tracks as knowledge already owned.
- Preface: dropped "exactly as the Spanish book used the *-ct-→-ch-* rules" and
  "Because the reader also knows Spanish (and is meeting French)"; states the
  true-beginner framing and that every Spanish/French form is supplied in full.
- `ch01-greetings.tex`: "German's version of the Spanish *-ct-→-ch-* rule" →
  self-contained sound-law framing; "the same job *bueno/buena* and *bon/bonne*
  did" → "the same job Romance adjectives do."
- Practice lessons `GE-C01-gut` ("the rules you met in Spanish") and
  `GE-C01-der-die-das` ("You've met gender in Spanish and French") de-assumed.

**Filled the parity gaps the audit flagged.**
- Added per-word **`sounds` boxes** (the book previously gave pronunciation only
  inline): *hallo*, *gut*, *der/die/das*, *Tag*, *Morgen*, *Abend*, *Nacht* ---
  including German final-devoicing (*Tag* → *tahk*, *Abend* → *AH-bent*) and the
  *ach*-laut in *Nacht*.
- Added noun **plurals**: *die Tage*, *die Morgen*, *die Abende*, *die Nächte*.
- Book still compiles clean with XeLaTeX (14 pages).

## Chapter 1 — Greetings (track bootstrapped)

- New German track on the HL00 framework: one word per lesson, slug ids,
  gender-before-nouns, atom-first, derivations shown, LaTeX book (CI
  auto-discovers `german/book/`).
- Chapter 1 (`lessons/GE-C01-*`), atom-first, with German's Germanic-roots
  flavor:
  - **hallo** (a *real* cousin of English "hello," unlike Spanish *hola*)
  - **gut** ("good" *and* "well" ← Germanic *\*gōdaz* = English *good*;
    introduces the **High German Consonant Shift** d→t as a recurring decoder)
  - **der / die / das** ("the"; **three** genders — German kept the neuter;
    ← Germanic *\*sa/\*sō/\*þat*, cousins of English *the/that*)
  - **Tag** (← *\*dagaz* = English *day*; ≠ Latin *dies* behind *día/jour*)
  - **Guten Tag** (assembled; the *-en* accusative ending)
  - **Morgen** (← *\*murganaz* = *morning/tomorrow*) · **Guten Morgen**
  - **Abend** (← *\*ābanþs* = English *eve*; contrast with Romance "late"
    words) · **Guten Abend**
  - **Nacht** (← PIE *\*nókʷts* — the four-way *Nacht/night/noche/nuit*
    reunion; feminine) · **Gute Nacht** (feminine agreement, *-e* not *-en*)
  - **practice**
- Grounds each word against English (direct Germanic cousin), with Spanish and
  French alongside for contrast. Book compiles clean with XeLaTeX (13 pages).
