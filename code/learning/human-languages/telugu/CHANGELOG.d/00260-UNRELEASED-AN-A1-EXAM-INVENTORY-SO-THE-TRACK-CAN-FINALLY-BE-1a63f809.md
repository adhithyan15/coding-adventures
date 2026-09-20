## Unreleased — an A1 exam inventory, so the track can finally be measured

`core/exam-inventory-telugu-a1.json` enumerates **326 things an A1 Telugu
candidate must be able to do**, each probed by the knowledge atoms whose
presence would demonstrate that the track teaches it. Telugu is the third Indic
track to have one, after Hindi and Marathi, and until now it was one of the
eighteen tracks whose exam gap could not be measured at all.

**The number: Telugu covers 169 of 326 points (52%).** Zero points are partly
taught — every probe names atoms that actually exist — so the shortfall is 157
points with nothing in the corpus corresponding to them. By category:

    18/22   Kriya (the verb)
     7/8    Kriya visheshanamu (adverbs)
     3/4    Suchaka padalu (demonstratives and deixis)
     7/10   Kaalam mariyu teedi (time and date)
     6/9    Vakyamu (the sentence)
     6/9    Prashnalu (asking questions)
     2/3    Nishedhamu (negation)
     5/8    Vibhakti (case endings and postpositions)
     5/8    Sarvanamamu (pronouns)
    36/61   Samvyavaharam (communicative functions)
     5/9    Sankhyalu (numerals and quantity)
     3/6    Visheshanamu (the adjective)
    11/23   Telugu lipi (script and orthography)
     4/9    Namavachakamu (the noun)
    47/110  Padasampada (core lexis)
     1/3    Nirdeshakamu (definiteness and the missing article)
     1/4    Sambandham (possession)
     2/13   Samuchchayamu (joining clauses)
     0/7    Uccharana (pronunciation)

### Where the point set comes from, and what it may not be used to claim

No awarding body publishes a content syllabus for Telugu; `core/exam-levels.json`
records this track as exam `no widely-sat ladder`, basis `editorial`, with no
caveat at all — the barest entry of any Indic track in that file. The point set
is therefore derived **structurally** from `core/exam-inventory-es-a1.json`, the
project's restatement of the Instituto Cervantes inventories behind DELE, used as
a proxy for **LEVEL and not for grammar**: each of its 273 points is read as a
demand on a learner, and answered with whatever Telugu exponent carries the same
load. All 273 are accounted for — 268 cited by some point's `derivedFrom`, and 5
named as untransferable Spanish typography, with reasons, in `about`.

**Nothing in that file may be attributed to DELE or the Instituto Cervantes about
Telugu.** They published a Spanish syllabus and have said nothing about this
language. The Telugu exponents are this project's editorial judgement throughout.

The evidence base is **thinner than Hindi's or Marathi's**, and the file says so
in as many words. Hindi could pin exponents against two checked-in timed A1
mocks and Marathi against eleven paper parts of `task-shapes/a1.json`; Telugu has
**neither** — there is no `telugu/assessment-spec.md`, no `telugu/task-shapes/`
and no `telugu/mocks/`. Writing that contract is the single change that would
most improve the inventory. No fresh external search for a Telugu awarding body
was carried out either, and `about` refuses to claim one.

### What the 157 unmapped points say about this track

The gap is not evenly spread, and its shape is the finding. The track is
**vocabulary-rich and sentence-poor**: 47 of 110 lexis points are covered,
including sixteen body-part atoms and thirteen kinship atoms that are well past
A1, while **11 of 13 subordination and coordination points have no atom at all**
— there is no word for *and*, none for *or*, none for *but*, and no quotative
`ani`, so nothing above a single clause can be built.

The largest single clusters behind the number:

* **Subordination and coordination (11 of 13 unmapped).** The whole apparatus for
  joining two ideas is absent.
* **Pronunciation (7 of 7 unmapped), for a structural reason.** The track's sound
  material lives in per-lesson `sounds:` front matter and in
  `pronunciation-reference.md`, and **neither declares an atom**, so no probe can
  reach it. These seven are unmeasurable rather than untaught, and the file says
  which.
* **The script's unfinished letter set (12 of 23 unmapped).** 23 of about 36
  consonants have a recognition lesson, 4 of about 12 independent vowels, and 10
  of 12 everyday vowel signs. Punctuation is untouched entirely.

  **This does not contradict `script-closure`, which reports Telugu at zero
  never-taught glyphs, and the inventory says so where it makes the claim.** The
  301 lesson files use **64 distinct Telugu characters and 49 have a dedicated
  recognition lesson** declaring an atom of their own; the other 15 are taught
  only inside a word, which is exactly what the closure gate counts. The
  inventory asks the narrower question a probe can answer — does the letter have
  an atom — and for 15 characters it does not. Counted over those same files:
  `ma` **636** occurrences, retroflex `ta` **292**, independent `a` **248**, the
  long `oo` sign **211**, `ba` **169**, the `o` sign **144**. `ma` is in the very
  first word the book teaches, `namaskaaram`.
* **Whole lexis domains with nothing in them.** Transport, media, the telephone,
  leisure, documents, clothing and payment are empty; there is no word for a
  *house*, a *city*, a *country* or a *year*.
* **The verb's missing half.** Four of Telugu's six persons are never taught, the
  negative conjugation is absent, and with it `teliyadu` — *I don't know* — which
  is the commonest thing an A1 candidate has to say.

Of the four points behind what a first-day speaking paper opens with — identify
yourself by name, give your name, age and origin, answer with an age in years,
say which place you are from — exactly **one** is covered.

### Also changed

`tests/plan-cli.test.ts` pins the completion-plan counters, which this inventory
moves. Re-measured against the merged tree rather than derived by arithmetic:
`0 complete and 6 partial` becomes `7 partial`, and `529 uncovered point(s)
across 6 written` becomes **`686 uncovered point(s) across 7 written`** — exactly
Telugu's 157 — while the unmeasurable remainder falls from 18 tracks to 17.

The base moved from 530 to 529 under this branch while it was open: French
chapter 6 was generated on main and closed one of that track's points. The
figure above was **re-measured against the merged tree** rather than carried
forward, which is why it is 686 and not the 687 an earlier commit message on
this branch quotes.

